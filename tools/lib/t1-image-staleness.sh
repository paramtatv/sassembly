#!/bin/sh
# tools/lib/t1-image-staleness.sh — IS THIS stage1.elf THIS TREE'S? Sourced, not run.
#
# ## Why this is a library and not three copies
#
# `check-sassembly-page.sh` learned, at `e01c05f5`, that a cached `stage1.elf` must say
# WHEN and from WHERE. Two tells, because either alone is blind:
#
#   BY DATE  13 of the 21 `crates/sadhana-t1/src/*.t1` were newer than the cached image.
#            That one directory is all of Stage 1's input (`fixpoint.sh:36`), so the net is
#            that directory and not `crates/*/src/*.t1`.
#   BY TREE  `stage1.log:1` named `/Users/pkpro/sansos/wt-raminject`, A DIFFERENT WORKTREE.
#            No mtime can see that: a foreign image written a second ago beats every source.
#
# `check-t1-image-wasm-budget.sh` and `check-laghava.sh` default to the SAME cache
# (`$TMPDIR/sassembly-fixpoint`) and had NEITHER tell, so both were quoting that foreign
# compiler as this tree's and reporting greens — a wasm budget and a laghava pair measured
# against a Stage 1 this tree does not hold. This file is the one place the two tells live,
# because a second copy of them is a second thing to drift.
#
# ## The asymmetry, and it is deliberate
#
# GIVEN (the operator named the file) WARNS and runs: they chose it, so the figure is theirs
# to label. CHOSEN (the script found it in a cache by itself) is REFUSED BY NAME: the choice
# was the script's and so is the refusal. `t1_image_verdict` is that rule, written once.
#
# ## What a stale image is NOT
#
# It is NOT red. Nine sources shared the mtime `2026-10-04 14:10:11` — the same second —
# while `kosha.t1`'s last commit was a day earlier, so a worktree mtime is when git last
# WROTE the file. Reddening on that would fail the tree for having been checked out, and
# this branch is merged into daily. Staleness proves the image UNVOUCHED, never wrong.
#
# ## Usage
#
#   . "$root/tools/lib/t1-image-staleness.sh"     # after cd to the repository root
#   t1_image_staleness "$IMAGE"                   # sets $t1_stale: a reason, or empty
#   t1_image_verdict "$IMAGE" chosen || skip "..." # 0 = may quote it, 1 = must not
#
# Its own selftest is `./tools/lib/t1-image-staleness.sh --selftest` — `./` and not `sh`,
# W-097 (`gate.sh:572`): `sh` overrides the shebang and that broke eight scripts once, and every caller
# runs it before trusting it — `check-laghava.sh:50`'s discipline: a comparator nobody
# exercised can report any order.

t1_stale=
t1_src=${t1_src:-$PWD/crates/sadhana-t1/src}

# `stat -c` is GNU and `stat -f %m` is BSD. GNU IS TRIED FIRST and the order is not
# cosmetic: BSD `stat` has no `-c` and exits nonzero, but GNU `stat -f` IS an option
# (`--file-system`) and would SUCCEED, printing a mount point where a timestamp belongs.
# `date -d @N` before `date -r N` for the same reason. Measured on this host: `stat -c`
# rc=1, `stat -f %m` rc=0, `date -d @N` rc=1, `date -r N` rc=0.
file_mtime_epoch() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null; }
fmt_epoch() {
  date -d "@$1" '+%Y-%m-%d %H:%M:%S' 2>/dev/null \
    || date -r "$1" '+%Y-%m-%d %H:%M:%S' 2>/dev/null \
    || echo "epoch $1"
}

# Sets `t1_stale` to the reason, or leaves it EMPTY. BOTH tells are collected, not just
# the first to fire, because a same-tree round that is merely old and a foreign-tree round
# of any age are different faults and the operator fixes them differently.
t1_image_staleness() {
  _img=$1
  _why=
  _log=$(dirname -- "$_img")/stage1.log
  if [ -f "$_log" ]; then
    _built=$(sed -n '1s/^compiler: .* sources loaded from //p' "$_log")
    if [ -n "$_built" ] && [ "$_built" != "$t1_src" ]; then
      _why="it was built from ANOTHER TREE ($_built), not $t1_src"
    fi
  fi
  _newer=0
  _newest=
  for _f in "$t1_src"/*.t1; do
    [ -f "$_f" ] || continue
    if [ "$_f" -nt "$_img" ]; then
      _newer=$((_newer + 1))
      if [ -z "$_newest" ] || [ "$_f" -nt "$_newest" ]; then _newest=$_f; fi
    fi
  done
  if [ "$_newer" -gt 0 ]; then
    _m="$_newer source(s) are NEWER than it, the newest $(basename -- "$_newest") at $(fmt_epoch "$(file_mtime_epoch "$_newest")")"
    if [ -n "$_why" ]; then _why="$_why; and $_m"; else _why=$_m; fi
  fi
  t1_stale=$_why
}

# `t1_image_verdict <image> given|chosen` — prints the dating line always, then applies the
# asymmetry. Returns 0 when the caller may quote the image's figures, 1 when it may not.
# It prints the reason either way: a refusal nobody can read is a skip for an unstated cause.
t1_image_verdict() {
  _vimg=$1
  _vhow=$2
  t1_image_staleness "$_vimg"
  echo "t1 image: $_vimg, dated $(fmt_epoch "$(file_mtime_epoch "$_vimg")")"
  if [ -z "$t1_stale" ]; then
    echo "          FRESH: it names this tree and no source is newer."
    return 0
  fi
  if [ "$_vhow" = given ]; then
    echo "          WARNING it is STALE: $t1_stale."
    echo "          Run anyway because YOU named this file — but the figures below are that"
    echo "          compiler's, not this tree's, so do not quote them as the tree's."
    return 0
  fi
  echo "          STALE, and this script CHOSE it, so it is REFUSED: $t1_stale."
  echo "          This proves the image UNVOUCHED, not wrong. Run tools/fixpoint.sh for one"
  echo "          from this tree, or T1_IMAGE=$_vimg to quote this one anyway."
  return 1
}

# ── selftest ───────────────────────────────────────────────────────────────────
# Runs only when this file is EXECUTED, never when sourced: `$0` ends in this file's name
# only in the first case. Nine probes. The third is the one mtimes cannot catch and the
# sixth is the asymmetry, which is the whole reason the siblings differ from the page.
case ${0##*/} in t1-image-staleness.sh)
  [ "${1:-}" = --selftest ] || { echo "$0: sourced, not run. Pass --selftest to exercise it."; exit 2; }
  fails=0
  st=$(mktemp -d) || exit 1
  trap 'rm -rf "$st"' EXIT
  # The sources this library reads are REAL ones, so the selftest needs a tree of its own:
  # pointing `t1_src` at a scratch directory makes every probe independent of the checkout.
  t1_src=$st/src
  mkdir -p "$t1_src" "$st/fresh" "$st/old" "$st/foreign" "$st/both"
  : >"$t1_src/one.t1"
  for d in fresh old foreign both; do
    : >"$st/$d/stage1.elf"
    printf 'compiler: 1 sources loaded from %s\n' "$t1_src" >"$st/$d/stage1.log"
  done
  printf 'compiler: 1 sources loaded from %s\n' /nowhere/src >"$st/foreign/stage1.log"
  printf 'compiler: 1 sources loaded from %s\n' /nowhere/src >"$st/both/stage1.log"
  touch -t 200001010000 "$st/old/stage1.elf" "$st/both/stage1.elf"
  probe() { # probe <label> <want-in-reason|EMPTY> <dir>
    t1_image_staleness "$st/$3/stage1.elf"
    case ${2:-} in
      EMPTY) [ -z "$t1_stale" ] && return 0 ;;
      *) case $t1_stale in *"$2"*) return 0 ;; esac ;;
    esac
    fails=$((fails + 1))
    echo "  FAIL  selftest $1: wanted '$2' in the reason, got '${t1_stale:-<none>}'"
  }
  probe 1-fresh   EMPTY        fresh
  probe 2-by-date 'are NEWER'  old
  probe 3-by-tree 'ANOTHER TREE' foreign
  # 4 is TWO probes on one image: the "A; and B" join must carry BOTH tells, not the first
  # to fire. A single check for the join would pass with either half silently dropped.
  probe 4-both-tree 'ANOTHER TREE' both
  probe 4-both-date 'are NEWER'    both
  # 5 and 6: one stale image, two origins, two verdicts. A library that returned the same
  # for both would make the siblings warn where they must refuse.
  if t1_image_verdict "$st/foreign/stage1.elf" given >/dev/null; then :; else
    fails=$((fails + 1)); echo "  FAIL  selftest 5: a GIVEN stale image must still be runnable (rc 0)"; fi
  if t1_image_verdict "$st/foreign/stage1.elf" chosen >/dev/null; then
    fails=$((fails + 1)); echo "  FAIL  selftest 6: a CHOSEN stale image must be refused (rc 1)"; fi
  # And a FRESH image must pass under both origins, or the siblings would never run at all.
  for how in given chosen; do
    t1_image_verdict "$st/fresh/stage1.elf" "$how" >/dev/null \
      || { fails=$((fails + 1)); echo "  FAIL  selftest 7-$how: a FRESH image must be accepted (rc 0)"; }
  done
  if [ "$fails" -ne 0 ]; then
    echo "t1-image-staleness: $fails of 9 probes FAILED, so nothing that sources this can be believed."
    exit 1
  fi
  echo "t1-image-staleness: 9 probes ok (fresh, stale-by-date, stale-by-tree, both, and the given/chosen asymmetry)"
  exit 0 ;;
esac
