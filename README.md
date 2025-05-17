# TheftMD

TheftMD (To be renamed) is a spiritual successor to [ThiefMD](https://github.com/kmwallio/ThiefMD/).

![cat](https://github.com/user-attachments/assets/eae2e847-a4c3-4cbf-a829-03480cdb266b)

![Screenshot from 2025-05-15 12-30-29](https://github.com/user-attachments/assets/fdfbb3cb-af6f-4109-b4d9-0d001960f983)

Built with Rust + GTK4 / Adwaita 

Early prototype, lacks fundamental features.

## Development

[➜ Project management](https://github.com/users/sevonj/projects/20)

### Continuous Integration

Pull requests are gatekept by [this workflow.](https://github.com/sevonj/theftmd/blob/master/.github/workflows/rust.yml) It will check if the code

- builds
- ~~passes unit tests (run `cargo test`)~~
- has linter warnings (run `cargo clippy`)
- is formatted (run `cargo fmt`)

### Dependencies

Ubuntu

```
libgtk-4-dev build-essential libglib2.0-dev libadwaita-1-dev libgtksourceview-5-dev
```
