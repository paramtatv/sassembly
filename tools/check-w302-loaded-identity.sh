#!/usr/bin/env bash
# W-302 — IDENTITY OF THE LOADED IMAGE, the row's acceptance as a run check.
#
# For each of the 48 spec/*.sas programs, at every load address the
# tools/check-*.sh scripts pass for it, Rust `sadhana [--स्थान <addr>]` writes
# one image and the .t1 chain (its own routines, interpreted) writes another;
# they must be equal in the ELF header EXCEPT the four section-table fields
# (e_flags INCLUDED), every program header, and every PT_LOAD's bytes —
# IDENTITY RULED (a), 2026-10-03. The comparator, the address table, and the
# one expected refusal (namaste-main, which needs lib-mudraka linked, refused
# by BOTH sides naming the same routine) live in
# crates/sadhana-t1/tests/w302_loaded_identity.rs; its comparator mutants run
# on every `cargo test`, and this script runs the `#[ignore]`d census:
#
#   W302_SADHANA=<sadhana> cargo test --release -p sadhana-t1 \
#       --test w302_loaded_identity -- --ignored --exact \
#       the_48_programs_load_identically_from_both_assemblers --nocapture
#
# Earlier outputs are deleted before each run, and a MISSING output on either
# side is a failure, never a match.
#
# Usage: tools/check-w302-loaded-identity.sh
# Exits 0 when every run is identical (or the expected refusal), 1 on any
# failure, 77 when it cannot run.

(eval ': <(:)') 2>/dev/null || exec /usr/bin/env bash "$0" "$@"

set -uo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
  echo "CANNOT RUN: cargo is not on PATH — nothing was assembled, so nothing is compared"
  exit 77
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "CANNOT RUN: python3 is not on PATH — needed to read cargo's artifact list"
  exit 77
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/w302-loaded-identity.XXXXXX") || exit 77
trap 'rm -rf "$WORK"' EXIT

# Release, always: the census drives the .t1 chain in the interpreter.
pick() {
  python3 -c '
import json, sys
want = sys.argv[1]
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("executable") \
       and m.get("target", {}).get("name") == want:
        print(m["executable"])
' "$1"
}

SADHANA=$(cargo build --release -p sadhana --bin sadhana --message-format=json 2>/dev/null | pick sadhana)
if [ -z "$SADHANA" ] || [ ! -x "$SADHANA" ]; then
  echo "FAIL: Rust sadhana did not build or was not found; run cargo build --release -p sadhana to see why"
  exit 1
fi
BIN=$(cargo test --release -p sadhana-t1 --test w302_loaded_identity --no-run \
        --message-format=json 2>/dev/null | pick w302_loaded_identity)
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "FAIL: the census test did not build; run the cargo line without 2>/dev/null to see why"
  exit 1
fi
echo "sadhana: $SADHANA"
echo "census:  $BIN"

# The test binary resolves spec/ from its manifest directory, so cwd is free.
# The chain's routines driven directly; then the wrapper module itself (its
# image, section table included) and the object-file path (mode 1 per
# source, mode 2 on the files) — all interpreted.
for t in the_48_programs_load_identically_from_both_assemblers \
         the_48_programs_load_identically_from_the_wrapper_and_through_object_files; do
  if ! W302_SADHANA="$SADHANA" W302_OUT="$WORK/out" "$BIN" --ignored --exact \
       "$t" --nocapture --test-threads 1; then
    echo "FAIL: W-302 loaded-image identity does not hold ($t) — the CENSUS lines above name each run"
    exit 1
  fi
done
echo "PASS: W-302 loaded-image identity holds on every run of both censuses"
exit 0
