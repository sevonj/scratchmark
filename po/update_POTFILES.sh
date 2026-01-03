#!/bin/sh
set -e

# Run from repo root.
# Auto crawl every ui file into POTFILES.in

rm -f po/POTFILES.in
cp po/POTFILES.in.in po/POTFILES.in
find data/resources/ui -type f -name \*.ui -printf '%h\0%p\n' | sort | awk -F '\0' '{print $2}' >> po/POTFILES.in