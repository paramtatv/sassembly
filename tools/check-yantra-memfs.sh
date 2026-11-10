#!/bin/sh
# The in-memory file root, end to end: the native-vs-memory parity test (Rust), then the
# same programs through the real wasm module and the JS glue (node).
# Usage: tools/check-yantra-memfs.sh
# YANTRA_WASM=<path> uses that module; else the native .t1 interpreter is built (tools/build-yantra-wasm.sh).
set -u
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1
command -v node >/dev/null || { echo "CANNOT RUN: node absent"; exit 77; }
command -v cargo >/dev/null || { echo "CANNOT RUN: cargo absent"; exit 77; }
d=$(mktemp -d) || exit 77
trap 'rm -rf "$d"' EXIT
wasm=${YANTRA_WASM:-}
if [ -z "$wasm" ]; then
  wasm=$d/yantra.wasm
  rc=0; sh tools/build-yantra-wasm.sh "$wasm" >"$d/bw.log" 2>&1 || rc=$?
  [ "$rc" -ne 77 ] || { echo "CANNOT RUN: native yantra wasm cannot build (no node/python3/cached Stage 1)"; exit 77; }
  [ "$rc" -eq 0 ] || { tail -5 "$d/bw.log"; echo "FAIL native yantra wasm build rc=$rc"; exit 1; }
fi
mkdir -p "$d/elf"
# the Rust parity test (native --files vs in-memory MemFs: status AND steps) left the tree with crates/yantra (R3-YAN);
# it runs from the pinned frozen oracle tree (tools/frozen-tree.sh).
w=$(mktemp -d "$d/frozen.XXXXXX") || { echo "CANNOT RUN: no scratch dir"; exit 77; }
sh tools/frozen-tree.sh "$w" || exit 77
( cd "$w" && MEMFS_ELF_DIR="$d/elf" CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/frozen-memfs-target} cargo test -q --release -p yantra --test memfs_parity ) || exit 1
node tools/check-yantra-memfs.mjs "$wasm" "$d/elf" || exit 1
