#!/usr/bin/env bash
# W-302 — THE SELF-HOSTED IMAGE ASSEMBLES EACH spec/*.sas, NATIVELY.
#
# tools/check-w302-loaded-identity.sh grades the .t1 chain INTERPRETED. This
# grades the SELF-HOSTED image: t1_image builds the compiler's twenty-one
# sources plus the entry module spec/entry/assemble.t1 into one image whose
# startup calls पाठसेतुःॱमुख्यम्; yantra-run runs it once per program and
# address, the .sas on the input channel (YANTRA_INPUT) and the load address
# as its one argument, and its stdout is the ELF. The committed comparator
# (crates/sadhana-t1/tests/w302_loaded_identity.rs, IDENTITY RULED (a)) grades
# each against Rust `sadhana [--स्थान <addr>]`:
#
#   W302_NATIVE=<image> W302_YANTRA_RUN=<yantra-run> W302_SADHANA=<sadhana> \
#     cargo test --release -p sadhana-t1 --test w302_loaded_identity -- \
#       --ignored --exact the_48_programs_load_identically_from_the_native_image
#
# THE BUILD IS THE DEAR PART (a Stage-1-sized interpreted build, tens of
# minutes). W302_NATIVE=<image> skips it and grades that image instead; the
# script then prints the image's sha256 and says it did not build it, because
# an image older than the tree it is graded against measures the wrong tree.
#
# Usage: tools/check-w302-native.sh
# Exits 0 when every run is identical (or the expected refusal), 1 on any
# failure, 77 when it cannot run.

(eval ': <(:)') 2>/dev/null || exec /usr/bin/env bash "$0" "$@"

set -uo pipefail
cd "$(dirname "$0")/.."

for t in cargo python3; do
  if ! command -v "$t" >/dev/null 2>&1; then
    echo "CANNOT RUN: $t is not on PATH — nothing was built, so nothing is compared"
    exit 77
  fi
done

WORK=$(mktemp -d "${TMPDIR:-/tmp}/w302-native.XXXXXX") || exit 77
trap 'rm -rf "$WORK"' EXIT

sha() { { sha256sum "$1" 2>/dev/null || shasum -a 256 "$1"; } | cut -c1-16; }

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

# Release, always.
built() { cargo build --release -p "$1" --bin "$2" --message-format=json 2>/dev/null | pick "$2"; }
SADHANA=$(built sadhana sadhana)
T1_IMAGE=$(built sadhana t1_image)
YANTRA_RUN=$(built yantra yantra-run)
for b in "$SADHANA" "$T1_IMAGE" "$YANTRA_RUN"; do
  if [ -z "$b" ] || [ ! -x "$b" ]; then
    echo "FAIL: a release binary did not build (sadhana, t1_image, yantra-run); run cargo build --release -p sadhana -p yantra to see why"
    exit 1
  fi
done

if [ -n "${W302_NATIVE:-}" ]; then
  NATIVE=$W302_NATIVE
  if [ ! -s "$NATIVE" ]; then
    echo "FAIL: W302_NATIVE=$NATIVE is not a non-empty file"
    exit 1
  fi
  echo "native:  $NATIVE — GIVEN, NOT BUILT by this run; sha256 $(sha "$NATIVE")"
else
  NATIVE="$WORK/assemble.elf"
  echo "native:  building the wrapper image (the corpus + spec/entry/assemble.t1), tens of minutes"
  if ! "$T1_IMAGE" --spec-root spec --compiler crates/sadhana-t1/src \
       --load spec/entry/assemble.t1 --entry पाठसेतुः मुख्यम् --no-predict \
       --accept-divergence "the wrapper assembles the .sas its input channel is handed, and the census hands each of 48 programs its own input later; this build has no input, so its predict compares nothing the census runs" \
       -o "$NATIVE" crates/sadhana-t1/src/*.t1 spec/entry/assemble.t1 > "$WORK/build.log" 2>&1; then
    echo "FAIL: t1_image did not build the wrapper image:"
    tail -20 "$WORK/build.log"
    exit 1
  fi
  grep -E '^(build|write):' "$WORK/build.log"
  echo "native:  sha256 $(sha "$NATIVE")"
fi

BIN=$(cargo test --release -p sadhana-t1 --test w302_loaded_identity --no-run \
        --message-format=json 2>/dev/null | pick w302_loaded_identity)
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "FAIL: the census test did not build; run the cargo line without 2>/dev/null to see why"
  exit 1
fi

# The image from source (mode 0, section table compared), then the object
# files (mode 1 per source, mode 2 on the files), both natively.
for t in the_48_programs_load_identically_from_the_native_image \
         the_48_programs_load_identically_through_native_object_files; do
  if ! W302_SADHANA="$SADHANA" W302_YANTRA_RUN="$YANTRA_RUN" W302_NATIVE="$NATIVE" \
       W302_OUT="$WORK/out" "$BIN" --ignored --exact \
       "$t" --nocapture --test-threads 1; then
    echo "FAIL: W-302 native loaded-image identity does not hold ($t) — the CENSUS lines above name each run"
    exit 1
  fi
done
echo "PASS: the self-hosted image assembles every census program to Rust's loaded image, directly and through object files"
exit 0
