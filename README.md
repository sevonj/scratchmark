[![CI](https://github.com/sevonj/scratchmark/actions/workflows/ci.yml/badge.svg)](https://github.com/sevonj/scratchmark/actions/workflows/ci.yml)

# Scratchmark

![app icon](data/icons/hicolor/scalable/apps/org.scratchmark.Scratchmark.svg)

Scratchmark is an organized Markdown editor designed for note keeping and writing. It's intended to become a spiritual successor to [ThiefMD](https://github.com/kmwallio/ThiefMD/).

![screenshot](data/screenshots/screenshot_a_light.png)

![screenshot](data/screenshots/screenshot_b_light.png)

![screenshot](data/screenshots/screenshot_c_dark.png)

![cat](https://github.com/user-attachments/assets/aaa7b417-5e2f-4a87-ad9b-aa29591d6bcd)

## Get Scratchmark

### Linux

<a href='https://flathub.org/apps/org.scratchmark.Scratchmark'>
<img height='48' alt='Get it on Flathub' src='https://flathub.org/api/badge?svg&locale=en'/>
</a>

### Windows

(planned)

## Developers

Scratchmark is written in Rust and uses GTK4 + Libadwaita for UI.

[➜ Project backlog](https://github.com/users/sevonj/projects/20)

### License

Scratchmark is licensed GPL-3.0-or-later. Some parts may *additionally* be available under other licenses, such as MIT.

### Building

The project is transitioning to Meson build system, but it remains possible to build and run it with Cargo.

### Cargo

When running from the repository, there's an additional step to the usual `cargo run` command. You need to set an env var to tell the app where to find it:

```sh
export GSETTINGS_SCHEMA_DIR=$PWD/data
```

The app needs its settings schema, and the lookup path seemingly can't be set in the code itself.

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

### Flatpak

Generating a Flatpak

#### Dependencies

You need Flatpak w/ Flathub and the following packages:

```
org.gnome.Sdk//49
```

#### Building

Build & install:

```sh
cd build-aux
sh generate_flatpak.sh && sudo flatpak install Scratchmark.flatpak
```
