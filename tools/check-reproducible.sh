#!/usr/bin/env bash
# Task A-003 — verify the build is bit-reproducible.
#
# RELEASE ONLY, and the scope matters: this builds `--release` (line 84) and
# claims nothing about `--dev`. A debug rebuild from byte-identical source does
# NOT produce an identical binary here, measured 2026-08-18 — which sits beside
# this check rather than against it. `profile.release` carries no debug info;
# `profile.dev` carries `debug = 2` and full DWARF, and that difference is most
# of why one is reproducible and the other is not. The gate's step is labelled
# bare `reproducible` (`gate.sh:94`), so the green tick reads broader than the
# thing tested; this clause is where a reader finds out. Flagged by a peer session.
#
# This is not hygiene. Three things depend on it and none of them can be
# retrofitted:
#
#   Gate D (doc 03 §6)   the triple-build fixpoint that proves self-hosting is
#                        `B ≡ C` byte-for-byte. Without reproducibility there is
#                        no fixpoint to check.
#   doc 16 §3.3          third parties verify a signed release by rebuilding the
#                        commit and comparing. A non-reproducible build makes the
#                        signature unfalsifiable.
#   doc 18 A5            reproducibility is a claimed axis of superiority over
#                        Rust, which is not reproducible by default.
#
# Checks three layers, because each fails differently:
#   1. generated artifacts   ucdgen and lexgen must be functions of their inputs
#   2. compiled binaries     same source, same bytes
#   3. build determinism     no timestamps, paths or hash-order in any output
#
# Usage: tools/check-reproducible.sh [--verbose]
#
# Re-exec under bash if invoked as `sh tools/check-reproducible.sh` — task
# `W-052`. macOS `/bin/sh` is bash 3.2 in POSIX mode, which rejects the process
# substitution at the diff below: the script aborts partway with `syntax error
# near unexpected token ('` having already printed a screen of green `ok` lines,
# and if the caller piped it to `tail` the status they read is the pipe's. This
# is a PROTOCOL §6 gate command, so a failure shaped like a pass is a red tree
# cleared for push. Same defect as `W-051`, found the same cycle, in the more
# expensive place.
#
# The obvious guard — `[ -n "$BASH_VERSION" ]` — does not work and was tried:
# bash sets that variable when invoked as `sh` too, so it reports the
# interpreter and not the dialect. Ask for the capability that is actually
# missing instead. `eval` defers the parse into a subshell, so probing for
# process substitution cannot itself be the syntax error.
(eval ': <(:)') 2>/dev/null || exec /usr/bin/env bash "$0" "$@"

set -uo pipefail
cd "$(dirname "$0")/.."
# CANNOT RUN (77) IS NOT A FAILURE. Without cargo nothing is built and the `fail`
# count below would report "not reproducible" when no build happened. See
# check-link.sh for the class that stopped every landing for two days.
if ! command -v cargo >/dev/null 2>&1; then
  echo "CANNOT RUN: cargo is not on PATH — nothing was built, so reproducibility is unmeasured"
  exit 77
fi

VERBOSE=${1:-}
fail=0

say()  { printf '%s\n' "$*"; }
ok()   { printf '  \033[32mok\033[0m    %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m  %s\n' "$*"; fail=1; }
note() { printf '        %s\n' "$*"; }

# ---------------------------------------------------------------- 1. generated
say "generated artifacts must be a pure function of their inputs"

# A generator absent from this list is a generator nothing verifies, which is
# the check-that-passes-by-doing-nothing shape this script exists to refuse. So
# adding one here is part of writing one, not a follow-up (`F-002b2`).
gen_files="spec/aksara-widths.tsv"
for gen in "ucdgen:crates/sanskrit-text/src/tables.rs" "lexgen:spec/lexicon.tsv" \
           "encgen:crates/anayana/src/encoding_tables.rs" \
           "mimegen:crates/adhvan/src/mimesniff_table.rs" \
           "entgen:crates/renderer/src/vigraha/entities.rs" \
           "idnagen:crates/adhvan/src/idna_tables.rs"; do
  tool=${gen%%:*}; out=${gen##*:}
  gen_files="$gen_files $out"
  before=$(shasum -a 256 "$out" | cut -d' ' -f1)
  # Assert the generator RAN. Without this the comparison below passes most
  # convincingly when the tool never executed: `ucdgen` exits 101 on a tree with
  # no pinned UCD, writes nothing, and `before` therefore equals `after`. That
  # is a green tick for a generator that crashed, and it is what CI reported on
  # every fresh checkout (`W-076`). Output stays discarded — it is noisy — but
  # the status is now load-bearing.
  if ! cmd_out=$(cargo run -q -p "$tool" 2>&1); then
    bad "$tool exited non-zero — nothing was regenerated, so this proves nothing"
    note "run research/specs/fetch-specs.sh unicode if the pinned UCD is absent"
    echo "$cmd_out"
    continue
  fi
  after=$(shasum -a 256 "$out" | cut -d' ' -f1)
  if [ "$before" = "$after" ]; then
    ok "$tool -> $out"
  else
    bad "$tool -> $out changed on regeneration"
    [ -n "$VERBOSE" ] && say "      $before -> $after"
  fi
done

# ---------------------------------------------------------------- 2. binaries
say
say "release binaries must be byte-identical across a clean rebuild"

# --remap-path-prefix removes the absolute build directory, which is the single
# most common source of Rust irreproducibility. If this is needed to pass, it is
# needed in the real build too — so it lives in .cargo/config.toml, not here.
build_and_hash() {
  local dir="$1"
  if ! CARGO_TARGET_DIR="$dir" cargo build -q --release --workspace; then
    return 1
  fi
  find "$dir/release" -maxdepth 1 -type f -perm -u+x ! -name '*.d' 2>/dev/null \
    | sort | while read -r f; do
        printf '%s  %s\n' "$(shasum -a 256 "$f" | cut -d' ' -f1)" "$(basename "$f")"
      done
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
A=$(build_and_hash "$tmp/a")
status_a=$?
B=$(build_and_hash "$tmp/b")
status_b=$?

if [ $status_a -ne 0 ] || [ $status_b -ne 0 ]; then
  bad "release build failed"
elif [ -z "$A" ]; then
  bad "no binaries produced — nothing to compare"
elif [ "$A" = "$B" ]; then
  n=$(printf '%s\n' "$A" | wc -l | tr -d ' ')
  ok "$n binary/binaries identical across independent target dirs"
else
  bad "binaries differ between builds"
  diff <(printf '%s\n' "$A") <(printf '%s\n' "$B") | sed 's/^/      /'
fi

# ------------------------------------------------------------- 3. determinism
say
say "no output may embed a timestamp, an absolute path, or hash-map order"

# Generated files are checked in, so a leaked build path would be visible in the
# diff forever.
leaks=$(grep -rlE "$(pwd | sed 's/[][\.*^$/]/\\&/g')" \
          $gen_files 2>/dev/null || true)
if [ -z "$leaks" ]; then
  ok "no absolute build paths in generated artifacts"
else
  bad "absolute build path leaked into: $leaks"
fi

# SOURCE_DATE_EPOCH is the standard lever; if anything honours it, changing it
# must not change the output.
# Same trap as section 1, and here it is inside a command substitution where the
# generator's status was discarded twice over: if ucdgen cannot run, both halves
# leave tables.rs untouched, h1 equals h2, and the clock-independence claim is
# reported as proven by two crashes (`W-076`).
#
# The comparison lives INSIDE the branch that produces its inputs. It was once
# outside, fed by `h1=skipped; h2=untested` — two sentinels that differ, so one
# absent generator printed BOTH "ucdgen exited non-zero" and "output depends on
# the clock", the second of them a verdict on a comparison whose inputs never
# existed. Two messages for one cause send the reader to the wrong fix
# (`C-002c`), and the wrong one here accuses the clock.
if ! out=$(cargo run -q -p ucdgen 2>&1); then
  bad "ucdgen exited non-zero — SOURCE_DATE_EPOCH independence is untested"
  echo "$out"
else
  out1=$(SOURCE_DATE_EPOCH=1 cargo run -q -p ucdgen 2>&1)
  if [ $? -ne 0 ]; then
    bad "ucdgen failed on run 1: $out1"
  else
    h1=$(shasum -a 256 crates/sanskrit-text/src/tables.rs | cut -d' ' -f1)
    out2=$(SOURCE_DATE_EPOCH=1700000000 cargo run -q -p ucdgen 2>&1)
    if [ $? -ne 0 ]; then
      bad "ucdgen failed on run 2: $out2"
    else
      h2=$(shasum -a 256 crates/sanskrit-text/src/tables.rs | cut -d' ' -f1)
      if [ "$h1" = "$h2" ]; then
        ok "output is independent of SOURCE_DATE_EPOCH"
      else
        bad "output depends on the clock"
      fi
    fi
  fi
fi

say
say "generated identity assets match their generator"
# assets/marks/marks.svg and sheet-marks.html are written by tools/gen-marks.py.
# A hand-edit to either would survive review and then be silently reverted by the
# next regeneration, so the claim "generated" is checked rather than trusted.
_m=$(mktemp -d); trap 'rm -rf "$tmp" "$_m"' EXIT
cp assets/marks/marks.svg "$_m/marks.svg"
cp assets/marks/sheet-marks.html "$_m/sheet.html"
if ! out=$(python3 tools/gen-marks.py 2>&1); then
  bad "tools/gen-marks.py failed"
  echo "$out"
else
  if cmp -s "$_m/marks.svg" assets/marks/marks.svg && cmp -s "$_m/sheet.html" assets/marks/sheet-marks.html; then
    ok "marks.svg and sheet-marks.html regenerate byte-identically"
  else
    bad "a generated identity asset was edited by hand — rerun tools/gen-marks.py"
    cp "$_m/marks.svg" assets/marks/marks.svg
    cp "$_m/sheet.html" assets/marks/sheet-marks.html
  fi
fi

say
if [ "$fail" -eq 0 ]; then
  say "reproducible."
else
  say "NOT reproducible — see failures above."
fi
exit "$fail"
