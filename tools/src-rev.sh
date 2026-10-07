#!/bin/sh
# Print a 16-hex STAMP over the content of a crate's `src` — the provenance of a build.
#
# Usage: tools/src-rev.sh <dir>        e.g. `crates/sadhana/src`, `crates/yantra/src`
#
# ## Why this exists, and why it is ONE script and not two implementations
#
# `F-004`'s page re-assembles its own Devanagari in the tab and compares the octets
# against the ELF the build shipped, then RUNS that ELF and quotes the halt status, the
# step count and the UART text. Three identical programs prove AGREEMENT TODAY and prove
# nothing about a page served tomorrow from a stale module: the wasm could have been
# built from a different assembler — or a different INTERPRETER — than the one that
# assembled and measured what travels beside it, and every button would still match,
# because both sides would be self-consistent. AGREEMENT IS NOT PROVENANCE.
#
# So both sides carry a stamp and the page compares them. The wasm's is COMPILED IN
# (`crates/sadhana-wasm/build.rs` and `crates/yantra-wasm/build.rs` run this script and
# seal the answer into the module, so a stale module carries its OWN old stamp and cannot
# claim a fresh one); the native side's is taken by `tools/build-sassembly-web.sh` from
# the tree it just ran `cargo run -p sadhana` and `yantra-run` against. A second copy of
# the hashing rule in any of those places would be a second thing to drift, which is the
# whole reason this is a file — AND IS WHY IT TAKES THE DIRECTORY AS AN ARGUMENT. The
# assembler's stamp came first and this script was named after it alone; stamping the
# interpreter the same way is one more caller, not a second rule, and a per-crate copy of
# this file beside it would be exactly the duplicate this paragraph argues against.
# (NAMED NO FILENAME FOR EITHER, deliberately: `metrics` asserts that every script a tool
# comment names EXISTS, and a margin that names a script this tree does not have is read
# as a statement of how things are — the former name is history and belongs in the commit.)
#
# ## WHY NOT `SASSEMBLY_BUILD_COMMIT`, WHICH ALREADY EXISTS
#
# `crates/sadhana/build.rs` already stamps a commit into this crate's binaries
# (`tools/build-stamp.rs`, `W-347`) — so there is an existing instrument and this is
# deliberately not it, for two measured reasons. It is not REACHABLE: the stamp is a
# `rustc-env` of the `sadhana` crate's own compilation, read by `t1_image.rs:156` and
# exported from the library nowhere, so `sadhana-wasm` cannot see it without an
# addition to `crates/sadhana/src`. And it would be WEAKER here even if it were: that
# file records its own caveat, that the stamp names HEAD AND NOT THE TREE and that an
# uncommitted edit does not even rerun it. Two artefacts built from one checkout
# minutes apart across an uncommitted change to the assembler carry the SAME commit
# and different behaviour — which is the page saying SAME ASSEMBLER about two that
# are not. A content hash is what distinguishes them, and this is a content hash.
#
# ## Exit 77 when it cannot hash
#
# No `sha256sum` and no `shasum` is a fault of the MACHINE and not of the tree, and
# the callers turn a 77 into the stamp `unknown` rather than into a stamp that
# happens to match. A provenance instrument with two states where the truth has
# three — agrees, differs, UNKNOWN — hides its own breakage.
#
# ## Exit 2 with NO argument, and why that is not defaulted to the assembler
#
# A default of `crates/sadhana/src` would make a caller that forgot its argument stamp
# THE WRONG CRATE and still exit 0 — so `crates/yantra-wasm/build.rs` would seal the
# assembler's hash and the page would compare the interpreter against it, differ, and
# report STALE INTERPRETER forever. A missing argument is a caller bug and says so.
set -eu

[ $# -eq 1 ] || {
  echo "usage: $0 <dir>   (the crate src directory to stamp, e.g. crates/yantra/src)" >&2
  exit 2; }
dir=$1

# GNU `sha256sum` and `shasum -a 256` print an awkward name (newline, backslash)
# the same way; Apple's own /sbin/sha256sum does NOT, so one tree stamped
# differently on the Mac and on Linux (W-381 review). Only a GNU `sha256sum` is
# taken; otherwise `shasum`. For ordinary names all three agree, so no recorded
# stamp moves.
if command -v sha256sum >/dev/null 2>&1 && sha256sum --version 2>/dev/null | grep -q GNU; then
  H=sha256sum
elif command -v shasum >/dev/null 2>&1; then
  H='shasum -a 256'
else
  echo "SKIPPED: neither GNU sha256sum nor shasum is installed, so no stamp can be taken" >&2
  exit 77
fi

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

[ -d "$dir" ] || {
  echo "  FAIL  $dir does not exist, so there is nothing to stamp" >&2
  exit 1; }

# The PATH travels into the hash as well as the content — `$H` prints both — so a
# renamed or deleted module moves the stamp and not only an edited one. `LC_ALL=C
# sort` because a locale-dependent order would make the same tree hash differently
# on two machines, which is exactly the lie this is here to catch.
#
# THE PATH IS THE ONE PASSED IN, so two crates can never collide: the stamp of
# `crates/yantra/src` carries those names and could not be mistaken for the
# assembler's even if the two trees held byte-identical files.
#
# ## EVERY STAGE IS CHECKED, AND A NAME IS NEVER SPLIT (W-381 review)
#
# The first form was one pipe, `find | sort | xargs $H | $H | cut`, whose status
# under `sh` is `cut`'s: a file named `0'x` made `xargs` abort on the quote, the
# script printed the hash of NOTHING (e3b0c442…) and exited 0, so two different
# trees stamped alike, and a name with a space dropped that file silently. Now the
# names travel NUL-separated (`-print0`, `sort -z`, `xargs -0`), each stage writes
# a file and its status is tested, and any failure is exit 1 — which every caller
# turns into `unknown`, never into a stamp. For ordinary names the text hashed is
# the same as before, so no recorded stamp moves.
t=$(mktemp "${TMPDIR:-/tmp}/src-rev.XXXXXX") || exit 1
trap 'rm -f "$t" "$t.s" "$t.h"' EXIT
fail() { echo "  FAIL  $1, so no stamp is printed" >&2; exit 1; }
find "$dir" -type f -print0 >"$t" || fail "find over $dir failed"
LC_ALL=C sort -z "$t" >"$t.s" || fail "sort failed"
[ -s "$t.s" ] || fail "$dir holds no files"
xargs -0 $H <"$t.s" >"$t.h" || fail "hashing a file under $dir failed"
sum=$($H <"$t.h") || fail "hashing the list failed"
printf '%s\n' "$sum" | cut -c1-16
