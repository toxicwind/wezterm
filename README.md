<p align="center">
  <img src="https://raw.githubusercontent.com/wezterm/wezterm/main/assets/icon/wezterm-icon.svg" height="96" alt="wezterm" />
</p>

<h1 align="center">estate / wezterm</h1>

<p align="center">
  <a href="https://github.com/wezterm/wezterm"><img src="https://img.shields.io/badge/upstream-wezterm%2Fwezterm-blue?style=flat-square" alt="upstream" /></a>
  <a href="https://github.com/estate/wezterm/actions"><img src="https://img.shields.io/github/actions/workflow/status/estate/wezterm/ci.yml?style=flat-square" alt="ci" /></a>
  <a href="./LICENSE.md"><img src="https://img.shields.io/badge/license-MIT-yellow?style=flat-square" alt="license" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.7+-orange?style=flat-square&logo=rust" alt="rust" /></a>
</p>

<p align="center">
  <strong>estate fork</strong> of <a href="https://github.com/wezterm/wezterm">wezterm/wezterm</a> — GPU-accelerated terminal emulator.<br/>
  Tracks upstream <code>main</code> (last merge <code>372548295</code>) with local protocol extensions.
</p>

## Fork extensions

| Feature | Commits |
|---------|---------|
| DEC 2031 appearance-change notifications | <code>dcbc1b4</code>, <code>e44c2ec</code> |
| OSC 777 toast notification tests | <code>f23ae77</code> |
| Kitty U=1 unicode placeholders | <code>221a935</code> |
| Weave entity-level merge driver | <code>b8cf8c6</code> |

Diff vs upstream: 19 files, +1182/−9. See <code>MERGE-DECISIONS.md</code>.

---

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
