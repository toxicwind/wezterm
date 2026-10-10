> **🍴 estate fork** of [wezterm/wezterm](https://github.com/wezterm/wezterm) —
> Wez's GPU-accelerated terminal emulator and multiplexer. Everything below is
> upstream's README, kept verbatim; this fork tracks upstream `main` (last merge
> `4ad65b0`, 2026-09-18) and carries Chris's own patches on top:
>
> - **DEC 2031 appearance-change notifications** — parser, term, mux and gui
>   support for live color-palette update notifications (`dcbc1b4`, `e44c2ec`).
> - **OSC 777 toast notifications** — regression tests (`f23ae77`).
> - **Kitty graphics** — `U=1` unicode placeholders for image placements
>   (`221a935`).
> - **Secret hygiene** — gitignore block so credentials are never committed
>   (`869faf8`).
>
> License: MIT (Wez Furlong) — see [LICENSE.md](./LICENSE.md).
>
> ---
# Wez's Terminal

<img height="128" alt="WezTerm Icon" src="https://raw.githubusercontent.com/wezterm/wezterm/main/assets/icon/wezterm-icon.svg" align="left"> *A GPU-accelerated cross-platform terminal emulator and multiplexer written by <a href="https://github.com/wez">@wez</a> and implemented in <a href="https://www.rust-lang.org/">Rust</a>*

User facing docs and guide at: https://wezterm.org/

![Screenshot](docs/screenshots/two.png)

*Screenshot of wezterm on macOS, running vim*

## Installation

https://wezterm.org/installation

## Getting help

This is a spare time project, so please bear with me.  There are a couple of channels for support:

* You can use the [GitHub issue tracker](https://github.com/wezterm/wezterm/issues) to see if someone else has a similar issue, or to file a new one.
* Start or join a thread in our [GitHub Discussions](https://github.com/wezterm/wezterm/discussions); if you have general
  questions or want to chat with other wezterm users, you're welcome here!
* There is a [Matrix room via Element.io](https://matrix.to/#/#wezterm:matrix.org)
  for (potentially!) real time discussions.

The GitHub Discussions and Element/Gitter rooms are better suited for questions
than bug reports, but don't be afraid to use whichever you are most comfortable
using and we'll work it out.

## Supporting the Project

If you use and like WezTerm, please consider sponsoring it: your support helps
to cover the fees required to maintain the project and to validate the time
spent working on it!

[Read more about sponsoring](https://wezterm.org/sponsor.html).

* [![Sponsor WezTerm](https://img.shields.io/github/sponsors/wez?label=Sponsor%20WezTerm&logo=github&style=for-the-badge)](https://github.com/sponsors/wez)
* [Patreon](https://patreon.com/WezFurlong)
* [Ko-Fi](https://ko-fi.com/wezfurlong)
* [Liberapay](https://liberapay.com/wez)
