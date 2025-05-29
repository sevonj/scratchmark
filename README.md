[![CI](https://github.com/sevonj/scratchmark/actions/workflows/ci.yml/badge.svg)](https://github.com/sevonj/scratchmark/actions/workflows/ci.yml)

# Scratchmark

![app icon](resources/org.scratchmark.Scratchmark.svg)

Scratchmark is a distraction-free markdown editor, designed both for keeping notes and writing. It's intended to become a spiritual successor to [ThiefMD](https://github.com/kmwallio/ThiefMD/).

![screenshot](data/screenshots/screenshot_a.png)

![screenshot](data/screenshots/screenshot_b.png)

![cat](https://github.com/user-attachments/assets/aaa7b417-5e2f-4a87-ad9b-aa29591d6bcd)

## Development

Scratchmark is written in Rust and uses GTK4 + Libadwaita for UI.

[➜ Project management](https://github.com/users/sevonj/projects/20)

### Continuous Integration

Pull requests are gatekept by [this workflow.](https://github.com/sevonj/scratchmark/blob/master/.github/workflows/rust.yml) It will check if the code

- builds
- passes unit tests (run `cargo test`)
- has linter warnings (run `cargo clippy`)
- is formatted (run `cargo fmt`)

### Dependencies

Ubuntu

```
libgtk-4-dev build-essential libglib2.0-dev libadwaita-1-dev libgtksourceview-5-dev
```
