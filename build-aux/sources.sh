#!/bin/sh
set -e

# For flathub offline build
curl -o flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/refs/heads/master/cargo/flatpak-cargo-generator.py
python3 flatpak-cargo-generator.py ../Cargo.lock -o generated-sources.json