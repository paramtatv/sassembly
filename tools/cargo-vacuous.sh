#!/bin/sh
# tools/cargo-vacuous.sh WHAT -- the cargo step WHAT has no subject: no crates/*/Cargo.toml is left in the tree (WSRET; the last Rust
# crates are retired to archive/rust/). Exit 78 (VACUOUS: ran, had no subject; never a GATE_STRICT failure, never a silent pass).
# While any crates/*/Cargo.toml exists this script exits 0 only to say "run the real step": callers test `cargo-vacuous.sh --has`.
case ${1:-} in
  --has) for m in crates/*/Cargo.toml; do [ -f "$m" ] && exit 0; done; exit 1 ;;
  '') echo "usage: $0 --has | WHAT" >&2; exit 2 ;;
esac
echo "VACUOUS: no cargo workspace ($1): no crates/*/Cargo.toml in the tree; native replacement = SANSOS_GATE_ENGINE=native tools/gate-native-tests.sh"
exit 78
