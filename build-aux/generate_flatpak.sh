#!/bin/sh
set -e

# For flathub offline build
# curl -o flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/refs/heads/master/cargo/flatpak-cargo-generator.py
# python3 flatpak-cargo-generator.py ../Cargo.lock -o generated-sources.json

flatpak-builder --repo=flatpakrepo flatpak org.scratchmark.Scratchmark.yml --force-clean
flatpak build-bundle flatpakrepo/ Scratchmark.flatpak org.scratchmark.Scratchmark