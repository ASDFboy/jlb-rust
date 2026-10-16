#!/bin/sh
# Copies the shared graphics core from jlb-c (its home) into native/gfx.
# The test `gfx_core_matches_jlb_c` checks the copies are the same.
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
from=${1:-$here/../jlb-c/src/gfx}
rm -rf "$here/native/gfx"
mkdir -p "$here/native/gfx"
cp "$from"/* "$here/native/gfx/"
echo "Copied $(ls "$here/native/gfx" | wc -l) files from $from"
