#!/bin/sh
# The in-memory file root, end to end: the native-vs-memory parity test (Rust), then the
# same programs through the real wasm module and the JS glue (node).
# Usage: tools/check-yantra-memfs.sh
set -u
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1
command -v node >/dev/null || { echo "CANNOT RUN: node absent"; exit 77; }
command -v cargo >/dev/null || { echo "CANNOT RUN: cargo absent"; exit 77; }
wasm=crates/yantra-wasm/target/wasm32-unknown-unknown/release/yantra_wasm.wasm
(cd crates/yantra-wasm && cargo build -q --release --target wasm32-unknown-unknown) \
  || { echo "CANNOT RUN: yantra-wasm does not build for wasm32 (no target?)"; exit 77; }
d=$(mktemp -d) || exit 1
trap 'rm -rf "$d"' EXIT
MEMFS_ELF_DIR="$d" cargo test -q --release -p yantra --test memfs_parity || exit 1
node tools/check-yantra-memfs.mjs "$wasm" "$d" || exit 1
