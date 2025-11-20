#!/bin/sh
set -e

wlc download --output po
unzip -oj po/scratchmark-app.zip -d po
meson setup builddir --reconfigure && meson compile -C builddir scratchmark-pot
git stage po
git commit -m 'chore: po'