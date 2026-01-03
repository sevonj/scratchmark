#!/bin/sh
set -e

# Run from repo root.
# CI check to make sure POTFILES.in is correct

cp po/POTFILES.in.in po/POTFILES.in.temp
find data/resources/ui -type f -name \*.ui -printf '%h\0%p\n' | sort | awk -F '\0' '{print $2}' >> po/POTFILES.in.temp
diff ./po/POTFILES.in ./po/POTFILES.in.temp