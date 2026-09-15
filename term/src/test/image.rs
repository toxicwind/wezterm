//! Tests for inline image protocol handling

use super::*;
use wezterm_cell::image::ImageDataType;
use wezterm_surface::change::ImageData;

/// A tiny but valid 11x11 PNG, base64 encoded.
/// Taken from the reproduction in <https://github.com/wezterm/wezterm/issues/6344>.
const TINY_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAsAAAALCAYAAACprHcmAAAACXBIWXMAAAGKAAABigEzlzBYAAAAOUlEQVQYlZXOwQ0AMAzCQEdi7yaT0xWAN7JuDCac2PQKYxflycOoICOKtPIuqFCg4/LzKxiz6xjyAYh9DR1sLUN1AAAAAElFTkSuQmCC";

/// Feeding a Kitty graphics escape that requests a zero-sized placement (here `r=0,h=0`)
/// must not panic the terminal.
/// Prior to the fix for <https://github.com/wezterm/wezterm/issues/6344> this divided by zero
/// while computing the per-cell pixel deltas and took down the whole pane.
#[test]
fn kitty_zero_dimension_image_does_not_panic() {
    let mut term = TestTerm::new(3, 10, 0);

    // a=T: transmit and display, t=d: data is directly embedded,
    // f=100: PNG, r=0/h=0: zero rows / zero source height.
    let seq = format!("\x1b_Gr=0,h=0,a=T,t=d,f=100;{}\x1b\\", TINY_PNG_BASE64);
    term.print(seq.as_bytes());

    // The image is refused, so the cursor never moved;
    // Printing normal text and observing it confirms we recovered rather than crashing.
    term.print(b"ok");
    assert_visible_contents(&term, file!(), line!(), &["ok", "", ""]);
}

/// A well-formed Kitty graphic with non-zero dimensions should continue to be accepted.
/// The test passes as long as processing the image does not panic and the terminal remains usable.
#[test]
fn kitty_valid_image_is_accepted() {
    let mut term = TestTerm::new(3, 10, 0);

    let seq = format!("\x1b_Ga=T,t=d,f=100;{}\x1b\\", TINY_PNG_BASE64);
    term.print(seq.as_bytes());

    // Printing normal text and observing it shifted confirms the terminal is usable.
    term.print(b"ok");
    assert_visible_contents(&term, file!(), line!(), &["  ok", "", ""]);
}

/// When the pty has no pixel size, `cell_pixel_width`/`cell_pixel_height` are zero.
/// Displaying an image sized in cells (ie: without explicit `c=`/`r=`) must not divide by zero.
/// This is a distinct crash from the zero-dimension image above and is not caught by that guard.
/// See <https://github.com/wezterm/wezterm/issues/6344>.
#[test]
fn kitty_image_with_zero_pixel_dimensions_does_not_panic() {
    let mut term = Terminal::new(
        TerminalSize {
            rows: 3,
            cols: 80,
            // No pixel size!
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        },
        Arc::new(TestTermConfig { scrollback: 0 }),
        "WezTerm",
        "O_o",
        Box::new(Vec::new()),
    );

    // No `c=`/`r=`, so the placement is computed from the (zero) cell pixel
    // size, exercising the divide that previously panicked.
    let seq = format!("\x1b_Ga=T,t=d,f=100;{}\x1b\\", TINY_PNG_BASE64);
    term.advance_bytes(seq.as_bytes());

    // The image is refused, so the cursor never moved;
    // Printing normal text and observing it confirms we recovered rather than crashing.
    term.advance_bytes(b"ok");
    assert_visible_contents(&term, file!(), line!(), &["ok", "", ""]);
}

/// A 2x2 RGBA image, base64 encoded.
const TINY_RGBA_BASE64: &str = "AQID/wQFBv8HCAn/CgsM/w==";

/// The image data attached to the first cell that carries one.
fn first_image(term: &TestTerm) -> Arc<ImageData> {
    for line in term.screen().visible_lines().iter() {
        for cell in line.visible_cells() {
            if let Some(im) = cell
                .attrs()
                .images()
                .and_then(|images| images.into_iter().next())
            {
                return Arc::clone(im.image_data());
            }
        }
    }
    panic!("no image was attached to the screen");
}

/// An Rgba8 stores the hash of its pixels and `compute_hash` reports it without
/// recomputing, so a kitty frame transmission that edits those pixels in place
/// must leave the stored hash describing what is now there.
#[test]
fn kitty_frame_edit_keeps_the_stored_hash_current() {
    let mut term = TestTerm::new(3, 10, 0);

    // a=T: transmit and display, f=32: RGBA, s/v: 2x2 pixels.
    let seq = format!("\x1b_Ga=T,t=d,f=32,s=2,v=2,i=1;{}\x1b\\", TINY_RGBA_BASE64);
    term.advance_bytes(seq.as_bytes());
    let transmitted_hash = first_image(&term).data().compute_hash();

    // a=f: transmit a frame, r=1: edit frame 1 in place, painting the top left
    // pixel opaque red over the 0x01,0x02,0x03 it arrived with.
    term.advance_bytes(b"\x1b_Ga=f,t=d,f=32,s=1,v=1,i=1,r=1,x=0,y=0;/wAA/w==\x1b\\");

    let image = first_image(&term);
    let image = image.data();
    let edited_hash = image.compute_hash();

    match &*image {
        ImageDataType::Rgba8 { data, .. } => {
            // Asserted on the pixels rather than on the hash, so a blit that
            // did nothing cannot be mistaken for a hash that went stale.
            k9::assert_equal!(&data[0..4], &[0xff, 0x00, 0x00, 0xff][..]);
            k9::assert_equal!(edited_hash, ImageDataType::hash_bytes(data));
        }
        other => panic!("expected Rgba8, got {:?}", other),
    }

    assert_ne!(edited_hash, transmitted_hash);
}

/// Kitty Unicode placeholders: a `U=1` virtual placement registers the
/// image without touching the grid or moving the cursor, and a U+10EEEE
/// cell whose colors and diacritics match resolves to the image fragment.
use k9::assert_equal;

/// Transmit the tiny PNG as image `id` without displaying it, so that a
/// later `a=p,U=1,i=<id>` virtual placement has data to reference.
fn transmit_virtual_image(term: &mut TestTerm, id: u32) {
    let seq = format!("\x1b_Ga=t,t=d,f=100,i={id};{TINY_PNG_BASE64}\x1b\\");
    term.print(seq.as_bytes());
}

/// Kitty Unicode placeholders: a `U=1` virtual placement registers the
/// image without touching the grid; a U+10EEEE cell whose foreground
/// color encodes the image id then renders that image's fragment.
#[test]
fn kitty_unicode_placeholder_resolves_virtual_placement() {
    let mut term = TestTerm::new(3, 10, 0);
    transmit_virtual_image(&mut term, 42);

    // Virtual placement: no cells drawn, cursor unmoved.
    term.print("\x1b_Ga=p,U=1,i=42,c=4,r=2\x1b\\");
    assert_visible_contents(&term, file!(), line!(), &["", "", ""]);

    // Foreground palette index 42 -> image id 42; diacritics U+0305 (row 0)
    // and U+030D (column 1) address the fragment.
    term.print("\x1b[38;5;42m\u{10EEEE}\u{0305}\u{030D}");
    assert_visible_contents(
        &term,
        file!(),
        line!(),
        &["\u{10EEEE}\u{0305}\u{030D}", "", ""],
    );

    let line = &term.screen().visible_lines()[0];
    let cell = line.visible_cells().next().unwrap();
    let images = cell
        .attrs()
        .images()
        .unwrap_or_else(|| panic!("no image attached to placeholder cell"));
    assert_equal!(images.len(), 1);
    assert_equal!(images[0].image_id(), Some(42));
    assert_equal!(images[0].placement_id(), None);
    // Column 1 of 4 -> x in [0.25, 0.5); row 0 of 2 -> y in [0.0, 0.5).
    assert_equal!(images[0].top_left().x.into_inner(), 0.25);
    assert_equal!(images[0].bottom_right().x.into_inner(), 0.5);
    assert_equal!(images[0].top_left().y.into_inner(), 0.0);
    assert_equal!(images[0].bottom_right().y.into_inner(), 0.5);
    // The placeholder text itself stays in the cell.
    assert!(cell.str().starts_with('\u{10EEEE}'));
}

/// The underline color selects the placement id of the virtual placement.
#[test]
fn kitty_unicode_placeholder_underline_selects_placement() {
    let mut term = TestTerm::new(3, 10, 0);
    transmit_virtual_image(&mut term, 42);

    term.print("\x1b_Ga=p,U=1,i=42,p=8,c=4,r=2\x1b\\");
    // Foreground 42 -> image id 42; underline palette index 8 ->
    // placement id 8; diacritics address row 0, column 0.
    term.print("\x1b[38;5;42m\x1b[58:5:8m\u{10EEEE}\u{0305}\u{0305}");

    let line = &term.screen().visible_lines()[0];
    let cell = line.visible_cells().next().unwrap();
    let images = cell
        .attrs()
        .images()
        .unwrap_or_else(|| panic!("no image attached to placeholder cell"));
    assert_equal!(images.len(), 1);
    assert_equal!(images[0].image_id(), Some(42));
    assert_equal!(images[0].placement_id(), Some(8));
}

/// A placeholder cell with no diacritics inherits row, column and the
/// most significant image id byte from the placeholder cell to its left.
#[test]
fn kitty_unicode_placeholder_inherits_from_left_cell() {
    let mut term = TestTerm::new(3, 10, 0);
    transmit_virtual_image(&mut term, 42);

    term.print("\x1b_Ga=p,U=1,i=42,c=4,r=2\x1b\\");
    term.print("\x1b[38;5;42m");
    // First cell carries row 0 / column 0 explicitly; the second has no
    // diacritics and inherits both from the left.
    let full = "\u{10EEEE}\u{0305}\u{0305}";
    let row_only = "\u{10EEEE}";
    term.print(&format!("{full}{row_only}"));
    assert_visible_contents(
        &term,
        file!(),
        line!(),
        &[&format!("{full}{row_only}"), "", ""],
    );

    let line = &term.screen().visible_lines()[0];
    let cells = line.visible_cells();
    for (idx, cell) in cells.take(2).enumerate() {
        let images = cell
            .attrs()
            .images()
            .unwrap_or_else(|| panic!("no image attached to cell {}", idx));
        assert_equal!(images.len(), 1);
        assert_equal!(images[0].image_id(), Some(42));
        // Both cells resolve to column 0 of the virtual area.
        assert_equal!(images[0].top_left().x.into_inner(), 0.0);
        assert_equal!(images[0].bottom_right().x.into_inner(), 0.25);
    }
}

/// In true color mode the foreground RGB encodes the low 24 bits of the
/// image id and the third diacritic the most significant byte.
#[test]
fn kitty_unicode_placeholder_truecolor_image_id() {
    let mut term = TestTerm::new(3, 10, 0);
    // Low 24 bits from the foreground rgb(0, 42, 0); the most
    // significant byte (1) from the third diacritic U+030D (index 1).
    let image_id: u32 = (1 << 24) | 0x2A00;
    transmit_virtual_image(&mut term, image_id);

    term.print(format!("\x1b_Ga=p,U=1,i={image_id},c=4,r=2\x1b\\").as_bytes());
    // Foreground rgb(0, 42, 0) -> low 24 bits = 0x00002A00;
    // U+0305 row 0, U+0305 column 0, U+030D msb 1.
    term.print("\x1b[38;2;0;42;0m\u{10EEEE}\u{0305}\u{0305}\u{030D}");

    let line = &term.screen().visible_lines()[0];
    let cell = line.visible_cells().next().unwrap();
    let images = cell
        .attrs()
        .images()
        .unwrap_or_else(|| panic!("no image attached to placeholder cell"));
    assert_equal!(images.len(), 1);
    assert_equal!(images[0].image_id(), Some(image_id));
}

/// A placeholder cell with no matching virtual placement is left as
/// plain text: no image is attached.
#[test]
fn kitty_unicode_placeholder_without_placement_is_plain_text() {
    let mut term = TestTerm::new(3, 10, 0);

    term.print("\x1b[38;5;99m\u{10EEEE}\u{0305}\u{030D}");
    assert_visible_contents(
        &term,
        file!(),
        line!(),
        &["\u{10EEEE}\u{0305}\u{030D}", "", ""],
    );

    let line = &term.screen().visible_lines()[0];
    let cell = line.visible_cells().next().unwrap();
    assert!(cell.attrs().images().is_none());
}
