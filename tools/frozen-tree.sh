#!/bin/sh
# tools/frozen-tree.sh DEST -> DEST becomes a cargo workspace of the pinned frozen oracle commit (tools/frozen-rust-oracle.sha):
# the retired Rust yantra + sadhana crates, so a check can still run the Rust test that guards an engine against the
# CURRENT tree. Default = the pure pinned tree (the frozen Rust reads its own spec/ and compiler sources: the car compiler uses
# syntax the frozen Rust cannot parse). `DEST --live` overlays this tree spec/ and crates/sadhana-t1/src (for differential
# checks that feed the Rust the current compiler, e.g. check-sadhana-native-identity.sh).
# Never writes into the repo. Exit 0 DEST ready (the cargo target is the caller's: set CARGO_TARGET_DIR to reuse builds);
# 77 could not run (no cargo, no pin, the archive failed), the reason on stderr.
root=$(cd "$(dirname "$0")/.." && pwd)
live=0; [ "${2:-}" = --live ] && live=1
dest=${1:-}; [ -n "$dest" ] && [ -d "$dest" ] || { echo "frozen-tree: DEST directory required" >&2; exit 77; }
s=$(cat "$root/tools/frozen-rust-oracle.sha" 2>/dev/null) || { echo "frozen-tree: no tools/frozen-rust-oracle.sha" >&2; exit 77; }
command -v cargo >/dev/null 2>&1 || { echo "frozen-tree: no cargo" >&2; exit 77; }
# tools/build-stamp.rs below is read from the PINNED old sha (retired from main to archive/rust/build-stamp/); the path is valid there.
git -C "$root" archive "$s" Cargo.toml Cargo.lock rust-toolchain.toml crates/yantra crates/sadhana crates/sadhana-t1 spec tools/build-stamp.rs 2>/dev/null \
  | tar -x -C "$dest" || { echo "frozen-tree: archive of $s failed" >&2; exit 77; }
[ -f "$dest/Cargo.toml" ] || { echo "frozen-tree: archive of $s gave an empty tree" >&2; exit 77; }
[ $live = 1 ] || exit 0   # default: the pure frozen tree (its own spec + compiler sources, which its Rust reads)
rm -rf "$dest/spec" && cp -R "$root/spec" "$dest/spec" || { echo "frozen-tree: cannot overlay spec" >&2; exit 77; }
rm -rf "$dest/crates/sadhana-t1/src" && cp -R "$root/crates/sadhana-t1/src" "$dest/crates/sadhana-t1/src" || { echo "frozen-tree: cannot overlay the compiler" >&2; exit 77; }
exit 0
