#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# Task A-003 — verify the build is bit-reproducible.
#
# RELEASE ONLY, and the scope matters: this builds `--release` (line 84) and
# claims nothing about `--dev`. A debug rebuild from byte-identical source does
# NOT produce an identical binary here, measured 2026-08-18 — which sits beside
# this check rather than against it. `profile.release` carries no debug info;
# `profile.dev` carries `debug = 2` and full DWARF, and that difference is most
# of why one is reproducible and the other is not. The gate's step is labelled
# bare `reproducible` (`gate.sh:94`), so the green tick reads broader than the
# thing tested; this clause is where a reader finds out. Flagged by `sansos-f0`.
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
# Usage: tools/check-reproducible.sh [--verbose] [--generated]
#   --generated  section 1 only (checked-in generated files == their generator's output), no
#                release builds: cheap enough for every landing and the nightly. Exit 77 only when
#                NO generator runner exists (no tools/lexgen/run.sh); the caller must
#                treat that as UNMEASURED, never as a pass. Added after b7f9b6f5 (2026-10-12)
#                landed with a stale spec/lexicon.tsv: this check ran only inside the internal gate,
#                which neither land.sh/land-remote.sh nor nightly.sh run.
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
# REPRO_ROOT: the tree to grade. land-remote.sh runs BASE's copy of this script (a tools-only
# extract) over the candidate TREE, so the script's own location is not the tree (RR-wasm-STALE-RCA-19).
cd "${REPRO_ROOT:-$(dirname "$0")/..}" || exit 2
# CANNOT RUN (77) IS NOT A FAILURE. Without cargo nothing is built and the `fail`
# count below would report "not reproducible" when no build happened. See
# check-link.sh for the class that stopped every landing for two days.
GENONLY=0; VERBOSE=
for a in "$@"; do case $a in --generated) GENONLY=1 ;; *) VERBOSE=$a ;; esac; done
# Generator runner: the native port (tools/lexgen/run.sh) always; it writes the output file in
# place. No cargo fallback (P4: crates/lexgen retired).
if [ ! -f tools/lexgen/run.sh ]; then
  echo "CANNOT RUN: tools/lexgen/run.sh missing - generated files UNMEASURED"
  exit 77
fi
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
# ucdgen, encgen, mimegen, entgen and idnagen moved to purged-rust/ on
# 2026-10-10 (owner-approved Rust retirement); their checked-in outputs are
# frozen and no longer regenerated, so only lexgen is verified here.
for gen in "lexgen:spec/lexicon.tsv"; do
  tool=${gen%%:*}; out=${gen##*:}
  gen_files="$gen_files $out"
  before=$(shasum -a 256 "$out" | cut -d' ' -f1)
  # Assert the generator RAN. Without this the comparison below passes most
  # convincingly when the tool never executed: `ucdgen` exits 101 on a tree with
  # no pinned UCD, writes nothing, and `before` therefore equals `after`. That
  # is a green tick for a generator that crashed, and it is what CI reported on
  # every fresh checkout (`W-076`). Output stays discarded — it is noisy — but
  # the status is now load-bearing.
  cmd_out=$(sh tools/lexgen/run.sh 2>&1)
  if [ $? -ne 0 ]; then
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

if [ "$GENONLY" = 1 ]; then
  [ "$fail" -eq 0 ] && say "generated files reproducible." || say "NOT reproducible — regenerate and commit the generated files."
  exit "$fail"
fi

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
if ! sh tools/cargo-vacuous.sh --has; then
  A=; B=; status_a=0; status_b=0
  say "VACUOUS: no cargo workspace (no crates/*/Cargo.toml): no binaries exist to compare; native byte-identity = tools/t1-build.sh fixpoint"
else
A=$(build_and_hash "$tmp/a")
status_a=$?
B=$(build_and_hash "$tmp/b")
status_b=$?
fi

if ! sh tools/cargo-vacuous.sh --has; then
  :
elif [ $status_a -ne 0 ] || [ $status_b -ne 0 ]; then
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

# The SOURCE_DATE_EPOCH check ran ucdgen, which moved to purged-rust/ on
# 2026-10-10 (owner-approved Rust retirement); there is no live generator left
# for it to exercise.

say
say "generated identity assets match their generator"
# assets/marks/marks.svg and sheet-marks.html are written by tools/gen-marks.sh (T1 port).
# A hand-edit to either would survive review and then be silently reverted by the
# next regeneration, so the claim "generated" is checked rather than trusted.
_m=$(mktemp -d); trap 'rm -rf "$tmp" "$_m"' EXIT
cp assets/marks/marks.svg "$_m/marks.svg"
cp assets/marks/sheet-marks.html "$_m/sheet.html"
if ! out=$(tools/gen-marks.sh 2>&1); then
  bad "tools/gen-marks.sh failed"
  echo "$out"
else
  if cmp -s "$_m/marks.svg" assets/marks/marks.svg && cmp -s "$_m/sheet.html" assets/marks/sheet-marks.html; then
    ok "marks.svg and sheet-marks.html regenerate byte-identically"
  else
    bad "a generated identity asset was edited by hand — rerun tools/gen-marks.sh"
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
