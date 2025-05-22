[![CI](https://github.com/sevonj/theftmd/actions/workflows/ci.yml/badge.svg)](https://github.com/sevonj/theftmd/actions/workflows/ci.yml)

# TheftMD

A minimal-distraction markdown editor for note taking and writing.

It's intended to become a spiritual successor to [ThiefMD](https://github.com/kmwallio/ThiefMD/).

![image](https://github.com/user-attachments/assets/0cbf3ec6-edc6-414c-ae0d-5cf0804e26b5)

> [!IMPORTANT]  
> This is an early prototype. It lacks some fundamental features and may eat your homework.
> 
![cat](https://github.com/user-attachments/assets/eae2e847-a4c3-4cbf-a829-03480cdb266b)

## Development

TheftMD is written in Rust and uses GTK4 + Libadwaita for UI.

[➜ Project management](https://github.com/users/sevonj/projects/20)

### Continuous Integration

Pull requests are gatekept by [this workflow.](https://github.com/sevonj/theftmd/blob/master/.github/workflows/rust.yml) It will check if the code

- builds
- passes unit tests (run `cargo test`)
- has linter warnings (run `cargo clippy`)
- is formatted (run `cargo fmt`)

### Dependencies

Ubuntu

```
libgtk-4-dev build-essential libglib2.0-dev libadwaita-1-dev libgtksourceview-5-dev
```
