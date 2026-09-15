//! Kitty graphics protocol: Unicode placeholders for image display.
//!
//! When an image is placed with `U=1`
//! (`ESC _ G a=p,U=1,i=<image_id>,c=<columns>,r=<rows> ESC \`)
//! the terminal registers a *virtual placement*: no cells are touched and
//! the cursor does not move. Cells containing the U+10EEEE placeholder
//! character then render fragments of that image:
//!
//! * the foreground color encodes the low 24 bits of the image id
//!   (the palette index in 256-color mode, the 24-bit RGB value in
//!   true color mode)
//! * the underline color optionally encodes the placement id
//! * up to three combining diacritics encode the row number, the column
//!   number and the most significant image id byte, in that order, with
//!   inheritance rules from the placeholder cell to the left when some
//!   diacritics are absent
//!
//! The placeholder text itself stays in the cell, so virtually-placed
//! images move, scroll and delete like ordinary text.
//!
//! <https://sw.kovidgoyal.net/kitty/graphics-protocol/#unicode-placeholders>

use super::*;
use std::sync::Arc;
use wezterm_cell::color::{ColorAttribute, SrgbaTuple};
use wezterm_cell::image::{ImageCell, TextureCoordinate};
use wezterm_escape_parser::apc::KittyImagePlacement;

/// The Unicode placeholder character marking cells that display
/// fragments of a virtually-placed kitty image.
pub const UNICODE_PLACEHOLDER: char = '\u{10EEEE}';

/// A kitty image placement created with `U=1`: registered under
/// (image id, placement id) but not drawn anywhere. Placeholder cells
/// (see [`UNICODE_PLACEHOLDER`]) resolve against these at print time.
#[derive(Debug, Clone)]
pub struct VirtualPlacement {
    pub data: Arc<ImageData>,
    pub cols: u32,
    pub rows: u32,
    pub z_index: i32,
}

/// Look up the row/column/most-significant-byte value encoded by a kitty
/// placeholder diacritic.
fn diacritic_value(c: char) -> Option<u32> {
    DIACRITIC_VALUES
        .binary_search_by(|(cp, _)| cp.cmp(&(c as u32)))
        .ok()
        .map(|idx| DIACRITIC_VALUES[idx].1)
}

/// Decode the diacritics carried by a placeholder grapheme into
/// (row, column, most-significant-byte), each `None` when the
/// corresponding diacritic is absent. Returns `None` when the grapheme
/// does not start with [`UNICODE_PLACEHOLDER`] or carries a diacritic
/// outside the kitty set.
fn decode_diacritics(grapheme: &str) -> Option<(Option<u32>, Option<u32>, Option<u32>)> {
    let mut chars = grapheme.chars();
    if chars.next() != Some(UNICODE_PLACEHOLDER) {
        return None;
    }
    let mut values = [None, None, None];
    for (slot, c) in chars.take(3).enumerate() {
        values[slot] = Some(diacritic_value(c)?);
    }
    let [row, col, msb] = values;
    Some((row, col, msb))
}

/// Map a foreground/underline color to the low 24 bits of an image id,
/// per the placeholder spec: the palette index in 256-color mode, or the
/// 24-bit RGB value in true color mode. Returns `None` when the color is
/// the default (unset), which cannot encode an id.
fn color_to_id_bits(color: &ColorAttribute) -> Option<u32> {
    match color {
        ColorAttribute::PaletteIndex(i) => Some(*i as u32),
        ColorAttribute::TrueColorWithPaletteFallback(SrgbaTuple(r, g, b, _), _)
        | ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(r, g, b, _)) => Some(
            (((*r * 255.0).round() as u32) << 16)
                | (((*g * 255.0).round() as u32) << 8)
                | ((*b * 255.0).round() as u32),
        ),
        ColorAttribute::Default => None,
    }
}

impl TerminalState {
    /// Register a virtual (`U=1`) placement for later resolution by
    /// [`UNICODE_PLACEHOLDER`] cells. Does not touch the grid and does
    /// not move the cursor.
    pub(crate) fn kitty_register_virtual_placement(
        &mut self,
        image_id: u32,
        placement: &KittyImagePlacement,
        data: Arc<ImageData>,
    ) -> anyhow::Result<()> {
        let cols = placement
            .columns
            .ok_or_else(|| anyhow::anyhow!("U=1 virtual placement requires c= (columns)"))?;
        let rows = placement
            .rows
            .ok_or_else(|| anyhow::anyhow!("U=1 virtual placement requires r= (rows)"))?;
        log::debug!(
            "registering virtual placement image_id={} placement_id={:?} {}x{} cells",
            image_id,
            placement.placement_id,
            cols,
            rows
        );
        self.kitty_img.virtual_placements.insert(
            (image_id, placement.placement_id),
            VirtualPlacement {
                data,
                cols,
                rows,
                z_index: placement.z_index.unwrap_or(0),
            },
        );
        Ok(())
    }

    /// Remove virtual placements matching (image_id, placement_id).
    /// With `placement_id == None`, removes every virtual placement of
    /// the image. Implements the virtual-placement side of the
    /// `d=i/I/n/N` deletion commands.
    pub(crate) fn kitty_remove_virtual_placements(
        &mut self,
        image_id: u32,
        placement_id: Option<u32>,
    ) {
        match placement_id {
            Some(_) => {
                self.kitty_img
                    .virtual_placements
                    .remove(&(image_id, placement_id));
            }
            None => {
                self.kitty_img
                    .virtual_placements
                    .retain(|(id, _), _| *id != image_id);
            }
        }
    }

    /// Find the virtual placement for a 32-bit image id and optional
    /// placement id. Without a placement id, any placement of the image
    /// matches: the one without an explicit placement id wins, otherwise
    /// the smallest placement id (deterministic).
    fn find_virtual_placement(
        &self,
        image_id: u32,
        placement_id: Option<u32>,
    ) -> Option<&VirtualPlacement> {
        let placements = &self.kitty_img.virtual_placements;
        match placement_id {
            Some(_) => placements.get(&(image_id, placement_id)),
            None => placements.get(&(image_id, None)).or_else(|| {
                placements
                    .iter()
                    .filter(|((id, _), _)| *id == image_id)
                    .min_by_key(|((_, pid), _)| *pid)
                    .map(|(_, vp)| vp)
            }),
        }
    }

    /// Resolve a [`UNICODE_PLACEHOLDER`] cell printed at (x, y) against the
    /// registered virtual placements, attaching the matching image
    /// fragment to the cell. The placeholder text stays in the cell so
    /// that it moves, scrolls and deletes like ordinary text. Cells that
    /// cannot be resolved (no diacritic match, no virtual placement, or
    /// out-of-range row/column) are left as plain text.
    pub(crate) fn resolve_unicode_placeholder(&mut self, x: usize, y: VisibleRowIndex) {
        // Snapshot the cell's diacritics and colors; the screen borrow
        // ends here.
        let (own, fg, underline) = {
            let screen = self.screen_mut();
            let cell = match screen.get_cell(x, y) {
                Some(cell) => cell,
                None => return,
            };
            let own = match decode_diacritics(cell.str()) {
                Some(own) => own,
                None => return,
            };
            (
                own,
                cell.attrs().foreground(),
                cell.attrs().underline_color(),
            )
        };
        let (mut row, mut col, mut msb) = own;

        // Inherit absent values from the placeholder cell to the left,
        // which must carry the same foreground and underline colors.
        if row.is_none() || col.is_none() || msb.is_none() {
            let left = if x > 0 {
                let screen = self.screen_mut();
                match screen.get_cell(x - 1, y) {
                    Some(left)
                        if left.attrs().foreground() == fg
                            && left.attrs().underline_color() == underline =>
                    {
                        decode_diacritics(left.str())
                    }
                    _ => None,
                }
            } else {
                None
            };
            if let Some((lrow, lcol, lmsb)) = left {
                if row.is_none() && col.is_none() && msb.is_none() {
                    // No diacritics at all: take all three from the left cell.
                    row = lrow;
                    col = lcol;
                    msb = lmsb;
                } else if row.is_some() && col.is_some() {
                    // Only the most significant byte is missing.
                    msb = lmsb;
                } else if row.is_some() {
                    // Only the row is present: take the column from the
                    // left cell, and the msb only when the rows agree.
                    col = lcol;
                    if row == lrow {
                        msb = lmsb;
                    }
                }
            }
        }
        let (row, col, msb) = (row.unwrap_or(0), col.unwrap_or(0), msb.unwrap_or(0));

        let low24 = match color_to_id_bits(&fg) {
            Some(bits) => bits,
            None => return,
        };
        // An unset underline color matches any placement id.
        let placement_id = color_to_id_bits(&underline);
        let image_id = (msb << 24) | low24;

        // Snapshot the fragment geometry, ending the immutable borrow
        // before attaching the image cell below.
        let (data, cols, rows, z_index) = match self.find_virtual_placement(image_id, placement_id)
        {
            Some(vp) => (Arc::clone(&vp.data), vp.cols, vp.rows, vp.z_index),
            None => return,
        };
        if row >= rows || col >= cols {
            // Outside the virtual placement: leave the placeholder as text.
            return;
        }

        let fragment = ImageCell::with_z_index(
            TextureCoordinate::new_f32(col as f32 / cols as f32, row as f32 / rows as f32),
            TextureCoordinate::new_f32(
                (col + 1) as f32 / cols as f32,
                (row + 1) as f32 / rows as f32,
            ),
            data,
            z_index,
            0,
            0,
            0,
            0,
            Some(image_id),
            placement_id,
        );

        let screen = self.screen_mut();
        let phys = screen.phys_row(y);
        let line = screen.line_mut(phys);
        if let Some(cell) = line.cells_mut_for_attr_changes_only().get_mut(x) {
            cell.attrs_mut().attach_image(Box::new(fragment));
        }
    }
}

/// (codepoint, row/column/most-significant-byte value) for the kitty
/// Unicode placeholder diacritics, sorted by codepoint for binary search.
/// Source: kitty's rowcolumn-diacritics.txt (Unicode 6.0.0 Mn/230/NSM
/// combining marks); index 0 -> U+0305, index 1 -> U+030D, ...
const DIACRITIC_VALUES: &[(u32, u32)] = &[
    (0x0305, 0),
    (0x030d, 1),
    (0x030e, 2),
    (0x0310, 3),
    (0x0312, 4),
    (0x033d, 5),
    (0x033e, 6),
    (0x033f, 7),
    (0x0346, 8),
    (0x034a, 9),
    (0x034b, 10),
    (0x034c, 11),
    (0x0350, 12),
    (0x0351, 13),
    (0x0352, 14),
    (0x0357, 15),
    (0x035b, 16),
    (0x0363, 17),
    (0x0364, 18),
    (0x0365, 19),
    (0x0366, 20),
    (0x0367, 21),
    (0x0368, 22),
    (0x0369, 23),
    (0x036a, 24),
    (0x036b, 25),
    (0x036c, 26),
    (0x036d, 27),
    (0x036e, 28),
    (0x036f, 29),
    (0x0483, 30),
    (0x0484, 31),
    (0x0485, 32),
    (0x0486, 33),
    (0x0487, 34),
    (0x0592, 35),
    (0x0593, 36),
    (0x0594, 37),
    (0x0595, 38),
    (0x0597, 39),
    (0x0598, 40),
    (0x0599, 41),
    (0x059c, 42),
    (0x059d, 43),
    (0x059e, 44),
    (0x059f, 45),
    (0x05a0, 46),
    (0x05a1, 47),
    (0x05a8, 48),
    (0x05a9, 49),
    (0x05ab, 50),
    (0x05ac, 51),
    (0x05af, 52),
    (0x05c4, 53),
    (0x0610, 54),
    (0x0611, 55),
    (0x0612, 56),
    (0x0613, 57),
    (0x0614, 58),
    (0x0615, 59),
    (0x0616, 60),
    (0x0617, 61),
    (0x0657, 62),
    (0x0658, 63),
    (0x0659, 64),
    (0x065a, 65),
    (0x065b, 66),
    (0x065d, 67),
    (0x065e, 68),
    (0x06d6, 69),
    (0x06d7, 70),
    (0x06d8, 71),
    (0x06d9, 72),
    (0x06da, 73),
    (0x06db, 74),
    (0x06dc, 75),
    (0x06df, 76),
    (0x06e0, 77),
    (0x06e1, 78),
    (0x06e2, 79),
    (0x06e4, 80),
    (0x06e7, 81),
    (0x06e8, 82),
    (0x06eb, 83),
    (0x06ec, 84),
    (0x0730, 85),
    (0x0732, 86),
    (0x0733, 87),
    (0x0735, 88),
    (0x0736, 89),
    (0x073a, 90),
    (0x073d, 91),
    (0x073f, 92),
    (0x0740, 93),
    (0x0741, 94),
    (0x0743, 95),
    (0x0745, 96),
    (0x0747, 97),
    (0x0749, 98),
    (0x074a, 99),
    (0x07eb, 100),
    (0x07ec, 101),
    (0x07ed, 102),
    (0x07ee, 103),
    (0x07ef, 104),
    (0x07f0, 105),
    (0x07f1, 106),
    (0x07f3, 107),
    (0x0816, 108),
    (0x0817, 109),
    (0x0818, 110),
    (0x0819, 111),
    (0x081b, 112),
    (0x081c, 113),
    (0x081d, 114),
    (0x081e, 115),
    (0x081f, 116),
    (0x0820, 117),
    (0x0821, 118),
    (0x0822, 119),
    (0x0823, 120),
    (0x0825, 121),
    (0x0826, 122),
    (0x0827, 123),
    (0x0829, 124),
    (0x082a, 125),
    (0x082b, 126),
    (0x082c, 127),
    (0x082d, 128),
    (0x0951, 129),
    (0x0953, 130),
    (0x0954, 131),
    (0x0f82, 132),
    (0x0f83, 133),
    (0x0f86, 134),
    (0x0f87, 135),
    (0x135d, 136),
    (0x135e, 137),
    (0x135f, 138),
    (0x17dd, 139),
    (0x193a, 140),
    (0x1a17, 141),
    (0x1a75, 142),
    (0x1a76, 143),
    (0x1a77, 144),
    (0x1a78, 145),
    (0x1a79, 146),
    (0x1a7a, 147),
    (0x1a7b, 148),
    (0x1a7c, 149),
    (0x1b6b, 150),
    (0x1b6d, 151),
    (0x1b6e, 152),
    (0x1b6f, 153),
    (0x1b70, 154),
    (0x1b71, 155),
    (0x1b72, 156),
    (0x1b73, 157),
    (0x1cd0, 158),
    (0x1cd1, 159),
    (0x1cd2, 160),
    (0x1cda, 161),
    (0x1cdb, 162),
    (0x1ce0, 163),
    (0x1dc0, 164),
    (0x1dc1, 165),
    (0x1dc3, 166),
    (0x1dc4, 167),
    (0x1dc5, 168),
    (0x1dc6, 169),
    (0x1dc7, 170),
    (0x1dc8, 171),
    (0x1dc9, 172),
    (0x1dcb, 173),
    (0x1dcc, 174),
    (0x1dd1, 175),
    (0x1dd2, 176),
    (0x1dd3, 177),
    (0x1dd4, 178),
    (0x1dd5, 179),
    (0x1dd6, 180),
    (0x1dd7, 181),
    (0x1dd8, 182),
    (0x1dd9, 183),
    (0x1dda, 184),
    (0x1ddb, 185),
    (0x1ddc, 186),
    (0x1ddd, 187),
    (0x1dde, 188),
    (0x1ddf, 189),
    (0x1de0, 190),
    (0x1de1, 191),
    (0x1de2, 192),
    (0x1de3, 193),
    (0x1de4, 194),
    (0x1de5, 195),
    (0x1de6, 196),
    (0x1dfe, 197),
    (0x20d0, 198),
    (0x20d1, 199),
    (0x20d4, 200),
    (0x20d5, 201),
    (0x20d6, 202),
    (0x20d7, 203),
    (0x20db, 204),
    (0x20dc, 205),
    (0x20e1, 206),
    (0x20e7, 207),
    (0x20e9, 208),
    (0x20f0, 209),
    (0x2cef, 210),
    (0x2cf0, 211),
    (0x2cf1, 212),
    (0x2de0, 213),
    (0x2de1, 214),
    (0x2de2, 215),
    (0x2de3, 216),
    (0x2de4, 217),
    (0x2de5, 218),
    (0x2de6, 219),
    (0x2de7, 220),
    (0x2de8, 221),
    (0x2de9, 222),
    (0x2dea, 223),
    (0x2deb, 224),
    (0x2dec, 225),
    (0x2ded, 226),
    (0x2dee, 227),
    (0x2def, 228),
    (0x2df0, 229),
    (0x2df1, 230),
    (0x2df2, 231),
    (0x2df3, 232),
    (0x2df4, 233),
    (0x2df5, 234),
    (0x2df6, 235),
    (0x2df7, 236),
    (0x2df8, 237),
    (0x2df9, 238),
    (0x2dfa, 239),
    (0x2dfb, 240),
    (0x2dfc, 241),
    (0x2dfd, 242),
    (0x2dfe, 243),
    (0x2dff, 244),
    (0xa66f, 245),
    (0xa67c, 246),
    (0xa67d, 247),
    (0xa6f0, 248),
    (0xa6f1, 249),
    (0xa8e0, 250),
    (0xa8e1, 251),
    (0xa8e2, 252),
    (0xa8e3, 253),
    (0xa8e4, 254),
    (0xa8e5, 255),
    (0xa8e6, 256),
    (0xa8e7, 257),
    (0xa8e8, 258),
    (0xa8e9, 259),
    (0xa8ea, 260),
    (0xa8eb, 261),
    (0xa8ec, 262),
    (0xa8ed, 263),
    (0xa8ee, 264),
    (0xa8ef, 265),
    (0xa8f0, 266),
    (0xa8f1, 267),
    (0xaab0, 268),
    (0xaab2, 269),
    (0xaab3, 270),
    (0xaab7, 271),
    (0xaab8, 272),
    (0xaabe, 273),
    (0xaabf, 274),
    (0xaac1, 275),
    (0xfe20, 276),
    (0xfe21, 277),
    (0xfe22, 278),
    (0xfe23, 279),
    (0xfe24, 280),
    (0xfe25, 281),
    (0xfe26, 282),
    (0x10a0f, 283),
    (0x10a38, 284),
    (0x1d185, 285),
    (0x1d186, 286),
    (0x1d187, 287),
    (0x1d188, 288),
    (0x1d189, 289),
    (0x1d1aa, 290),
    (0x1d1ab, 291),
    (0x1d1ac, 292),
    (0x1d1ad, 293),
    (0x1d242, 294),
    (0x1d243, 295),
    (0x1d244, 296),
];
