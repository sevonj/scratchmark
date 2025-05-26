#!/bin/sh
set -e

curl -o flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/refs/heads/master/cargo/flatpak-cargo-generator.py
python3 flatpak-cargo-generator.py ../Cargo.lock -o generated-sources.json
flatpak-builder --repo=flatpakrepo flatpak fi.sevonj.TheftMD.yml --force-clean
flatpak build-bundle flatpakrepo/ TheftMD.flatpak fi.sevonj.TheftMD