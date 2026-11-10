#!/bin/sh
# The generated page's OWN SCRIPT runs, and the right pane fills — task `F-004`.
#
# ## The gap this closes
#
# `web/` is compiled by nothing. The glue (`web/yantra.mjs`) gained two checks —
# `check-yantra-glue.sh` against a stand-in module and `check-yantra-glue-live.sh`
# against the real one — but the page's own `<script type="module">` in
# `web/sassembly.template.html` had NEITHER, and the check that looks most like
# it would does not run it: `check-sassembly-identity.sh` regexes `PROGRAMS` out
# of the page and drives `yantra_run` through the RAW exports. The button loop,
# the two pane ids, `show()`'s empty-reason branch and the `kind === 'app'`
# switch between `host()` and `run()` are never entered by any gate.
#
# MEASURED, with the page built and every other check green: a `host` renamed in
# the glue, an element id renamed in the HTML, the two output panes merged, the
# `kind` switch dropped, the auto-clicked proof renamed, the halt class
# inverted, a renamed result field, and an `export default` added to the glue —
# EIGHT mutations, each of which leaves `tools/build-sassembly-web.sh` exiting 0
# over a page that is broken on open. This check reds on all eight; it is the
# only thing in the tree that reds on any of them.
#
# The last of the eight is why the check runs the page's INLINED copy of the
# glue rather than importing `web/yantra.mjs`: the page does not import it.
# `build-sassembly-web.sh` splices it in through `sed 's/^export //'`, and
# `export default class Yantra` survives that as `default class Yantra` — a
# syntax error inside a page whose build reports success.
#
# ## Exit 77 when it cannot run, and it BUILDS its own subject
#
# Unlike `check-yantra-glue-live.sh`, which deliberately uses a module that is
# already there, this needs a whole page and a page is cheap: measured 20s
# end to end on a warm `yantra-wasm`, which `check-sassembly-identity.sh` in the
# same deep-gate run has already warmed. So it builds one rather than exiting
# VACUOUS on a tree that has not. Pass a path to an ALREADY-BUILT page to skip
# the build — that is the fast loop while editing the template.
#
# 77 (and never 1) when node, cargo or `wasm32-unknown-unknown` is absent:
# `W-087`'s convention, which `tools/gate.sh` renders as neither pass nor fail.
# The page build's own 77 is PROPAGATED for the same reason — a missing wasm32
# target is a fault of this machine, not a tree fault, and
# `check-sassembly-identity.sh:112` records what happened the last time that
# distinction was dropped: it declined ten consecutive recovery cycles.
#
# ## The fourth button is OPTIONAL and its absence is reported, not passed
#
# `T1_IMAGE` is read by `build-sassembly-web.sh` out of this script's own environment,
# so `T1_IMAGE=... tools/check-sassembly-page.sh` is what runs the T1 image's arms. With
# it unset the page has three buttons, every arm above is still asserted, and the driver
# prints the t1 arms as UNRUN rather than counting them green — `W-087`'s finding was 29
# skip paths that said "passed".
#
# IT IS NO LONGER ONLY A HAND VARIABLE, and the sentence that stood here — "the deep gate
# runs it unset" — WAS FALSE WHEN WRITTEN: measured 2026-10-04, `git grep` finds this
# script named by NO gate at all, so its t1 arms were not merely unrun, nothing ran. Two
# sibling checks already resolve the same artefact the same way —
# `check-t1-image-wasm-budget.sh:74` defaults `T1_IMAGE` to
# `$TMPDIR/sassembly-fixpoint/stage1.elf` and `check-laghava.sh:38` reads that directory
# for its own pair — because that is where `tools/fixpoint.sh:33` writes Stage 1. So this
# takes the SAME default: a round already run on this host arms the fourth button for
# free, and Stage 1 is still never built here (~1560 s, which is why a gate that BUILT one
# would be a gate that never ran).
#
# WHICH OF THE FOUR HAPPENED IS PRINTED BY NAME, never inferred from a green: given,
# found in the cache, found in the cache AND STALE, or absent. An absent image still
# exits 0 with the t1 arms reported UNRUN by the driver — the page is complete without
# them — and so does a stale one. IT WAS THREE UNTIL 2026-10-05; see the next section for
# why a two-state cache (there / not there) hid its own breakage.
#
# Usage: tools/check-sassembly-page.sh [already-built-page.html]
#   T1_IMAGE=path/to/stage1.elf   adds the fourth button and runs its arms, stale or not
#   FIXPOINT_OUT=dir              where to look for `stage1.elf` when `T1_IMAGE` is unset
#                                 (default: $TMPDIR/sassembly-fixpoint, `fixpoint.sh`'s)
#   SASSEMBLY_PAGE_RESOLVE_ONLY=1 print the image resolution and exit 0, building nothing
#                                 (what the self-test below drives; not for gates)
#   SASSEMBLY_PAGE_NO_SELFTEST=1  skip the resolution self-test
set -u

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1

# `CANNOT RUN:` must be the FIRST line and spelled EXACTLY that — `gate.sh`'s
# run() and `deep-gate.sh` both match `^(SKIPPED|CANNOT RUN):`.
missing=
command -v node  >/dev/null 2>&1 || missing="$missing node"
command -v cargo >/dev/null 2>&1 || missing="$missing cargo"
# Not under RESOLVE_ONLY: that mode builds no page, so demanding its target would make
# the self-test's four probes unrunnable on a host the real check merely SKIPS.
if [ -z "${1:-}" ] && [ -z "${SASSEMBLY_PAGE_RESOLVE_ONLY:-}" ]; then
  rustc --print target-list 2>/dev/null | grep -qx wasm32-unknown-unknown \
    || missing="$missing wasm32-unknown-unknown"
fi
if [ -n "$missing" ]; then
  echo "CANNOT RUN:$missing absent — the page cannot be built or its script cannot be run"
  echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
  exit 77
fi

# ── the fourth button's image, resolved, NAMED and DATED ──────────────────
# Exported, because `build-sassembly-web.sh` reads it out of the environment and this
# script runs it as a child. `T1_IMAGE` set and naming no file is NOT quietly downgraded
# to the cache: the build refuses that at :149 and so does this, because a typo in a gate
# line must not render as the ordinary three-button page.
#
# AND ONLY WHEN THIS RUN BUILDS THE PAGE. Given an already-built page as $1, the embedded
# programs are whatever THAT build embedded, and announcing an image here would be a claim
# about a file this run never reads — the shape `W-087` names: a line that reads as a pass.
#
# ## A CACHED IMAGE MUST SAY WHEN, AND A STALE ONE IS VACUOUS — NOT RED
#
# `found in the fixpoint cache (1445146 octets)` did not say WHEN, so the fourth button's
# arms could quote a compiler this tree no longer holds and still read as an ordinary
# green: a figure that fails to name its tree. TWO INDEPENDENT tells sit beside the image
# and both are now printed, because they fail differently and either alone is blind:
#
#   mtime  any `crates/sadhana-t1/src/*.t1` NEWER than the image. That ONE directory is
#          the whole of Stage 1's input — `fixpoint.sh:36` sets `SRC` to it — so the net
#          is not `crates/*/src/*.t1`: measured 2026-10-05 that glob is 21 files and ALL
#          21 are under `sadhana-t1`, so the wider net would only add false alarms later.
#   tree   `fixpoint.sh` writes `stage1.log` beside the image and its first line reads
#          `compiler: N sources loaded from DIR`. A DIR that is not THIS checkout's `SRC`
#          means the image was built from a DIFFERENT worktree, which mtime cannot see at
#          all — a foreign image minutes old passes every mtime test. Measured on this
#          host 2026-10-05, the cached image names
#          a sibling worktree, not this worktree.
#
# NEITHER MAY BE A RED, AND THAT IS A MEASUREMENT, NOT A PREFERENCE. Measured on this host
# 2026-10-05: NINE of the 21 sources carry the mtime `2026-10-04 14:10:11` — the same
# second — while `kosha.t1`'s last commit to touch it is `3e2952ed`, `2026-10-03
# 19:14:54`, a DAY EARLIER. A worktree mtime is when `git` last WROTE the file, not when
# anyone changed it, so one checkout or merge makes every file it rewrote newer than any
# image built before it, content identical or not. A check that failed the tree on that
# would fail it for having been checked out, and this branch is merged into daily.
#
# So the third state, and it is the point of the change: the cache is used only while it
# is PROVABLY fresh, and a stale one is named out loud with its arms UNRUN — `W-087`'s
# distinction, the same one the absent-image arm already draws. ASYMMETRIC ON PURPOSE:
# with `T1_IMAGE` GIVEN the operator named that exact file, so staleness is a WARNING over
# arms that still run; when this script picked the file out of a cache by itself, the
# choice was the script's and so is the refusal.
t1_image_origin=
t1_stale=
t1_stale_image=
# THE TWO TELLS LIVE IN ONE PLACE, and this script is no longer it. They were written here
# at `e01c05f5` and then `check-t1-image-wasm-budget.sh` and `check-laghava.sh` turned out
# to default to the SAME cache with NEITHER tell, so a copy each would have been three
# copies to drift. `t1_src` is set before the source so the library's `${t1_src:-...}`
# keeps this script's exact net: Stage 1's whole input and nothing else (`fixpoint.sh:36`).
t1_src=$PWD/crates/sadhana-t1/src
. ./tools/lib/t1-image-staleness.sh
# The PRODUCING half of the gate's one-build handover. Sourced here rather than beside its
# use so that a broken library is a syntax error at the top of this script and not a
# surprise after the 25 s build has already been paid for. Its own selftest runs in the
# CONSUMER (`check-sassembly-identity.sh`), which is the side whose correctness decides
# whether a page is believed; publishing is advisory and cannot fail.
. ./tools/lib/page-handoff.sh

# ── the self-test: the four states must be DISTINGUISHABLE ─────────────────────
# `check-fetch-log.sh:272`'s shape: re-enter this same file under an env var that stops
# the recursion, here `SASSEMBLY_PAGE_RESOLVE_ONLY`, which also stops the 20s page build.
# Four probes against crafted cache directories, and the FOURTH is the one that matters:
# an image whose mtime beats every source but whose `stage1.log` names another tree. A
# check that only looked at mtimes would call that one FRESH.
if [ -z "${SASSEMBLY_PAGE_RESOLVE_ONLY:-}" ] && [ -z "${SASSEMBLY_PAGE_NO_SELFTEST:-}" ]; then
  st=$(mktemp -d) || exit 1
  trap 'rm -rf "$st"' EXIT; trap 'exit 143' INT TERM
  st_fails=0
  # 1 fresh: written now, so newer than every source, and its log names THIS tree.
  mkdir -p "$st/fresh" "$st/old" "$st/foreign" "$st/none"
  : >"$st/fresh/stage1.elf"
  printf 'compiler: 21 sources loaded from %s\n' "$t1_src" >"$st/fresh/stage1.log"
  # 2 old: the same log, an image predating the sources.
  : >"$st/old/stage1.elf"
  printf 'compiler: 21 sources loaded from %s\n' "$t1_src" >"$st/old/stage1.log"
  touch -t 200001010000 "$st/old/stage1.elf"
  # 3 foreign: newest possible image, a log naming a tree that is not this one.
  : >"$st/foreign/stage1.elf"
  printf 'compiler: 21 sources loaded from %s\n' /nowhere/crates/sadhana-t1/src \
    >"$st/foreign/stage1.log"
  # 4 none: an empty directory.
  for probe in 'fresh|will RUN' 'old|NOT USED' 'foreign|ANOTHER TREE' 'none|t1 image: none'; do
    dir=${probe%%|*}
    want=${probe#*|}
    got=$(T1_IMAGE= FIXPOINT_OUT="$st/$dir" SASSEMBLY_PAGE_RESOLVE_ONLY=1 \
            SASSEMBLY_PAGE_NO_SELFTEST=1 sh "$0" 2>&1)
    case $got in
      *"$want"*) ;;
      *) st_fails=$((st_fails + 1))
         echo "  FAIL  self-test: the $dir cache should report '$want'; it reported:"
         echo "$got" | sed 's/^/          /' ;;
    esac
  done
  # `foreign` must not also be excused by its mtime, and `fresh` must not be called stale.
  rm -rf "$st"; trap - EXIT INT TERM
  if [ "$st_fails" -ne 0 ]; then
    echo "        The image resolution cannot tell its four states apart, so nothing it"
    echo "        prints about the fourth button can be believed. Exiting 1."
    exit 1
  fi
  echo "self-test: the four image states are distinguishable (fresh, stale-by-date, stale-by-tree, absent)"
fi

if [ -n "${1:-}" ]; then
  t1_image_origin="PREBUILT"
elif [ -n "${T1_IMAGE:-}" ]; then
  [ -f "$T1_IMAGE" ] || {
    echo "  FAIL  T1_IMAGE=$T1_IMAGE names no file."
    echo "        Unset it for the three-button page; do not point it at a missing image."
    exit 1
  }
  t1_image_origin="GIVEN"
  t1_image_staleness "$T1_IMAGE"
else
  fixpoint_out=${FIXPOINT_OUT:-${TMPDIR:-/tmp}/sassembly-fixpoint}
  if [ -f "$fixpoint_out/stage1.elf" ]; then
    t1_image_staleness "$fixpoint_out/stage1.elf"
    if [ -n "$t1_stale" ]; then
      t1_stale_image=$fixpoint_out/stage1.elf
      t1_image_origin="STALE"
    else
      T1_IMAGE=$fixpoint_out/stage1.elf
      export T1_IMAGE
      t1_image_origin="CACHED"
    fi
  fi
fi
case $t1_image_origin in
  PREBUILT) echo "t1 image: not resolved — the page was built elsewhere; its t1 arms run only if THAT build embedded one" ;;
  GIVEN)  echo "t1 image: $T1_IMAGE — GIVEN, dated $(fmt_epoch "$(file_mtime_epoch "$T1_IMAGE")"); the fourth button's arms will RUN"
          if [ -n "$t1_stale" ]; then
            echo "          WARNING it is STALE: $t1_stale."
            echo "          Run anyway because YOU named this file — but the arms' figures are that"
            echo "          compiler's, not this tree's, so do not quote them as the tree's."
          fi ;;
  CACHED) echo "t1 image: $T1_IMAGE — found in the fixpoint cache ($(wc -c < "$T1_IMAGE" | tr -d ' ') octets), dated $(fmt_epoch "$(file_mtime_epoch "$T1_IMAGE")")"
          echo "          FRESH: it names this tree and no source is newer, so the fourth button's arms will RUN" ;;
  STALE)  echo "t1 image: $t1_stale_image — found in the fixpoint cache ($(wc -c < "$t1_stale_image" | tr -d ' ') octets), dated $(fmt_epoch "$(file_mtime_epoch "$t1_stale_image")")"
          echo "          STALE, so NOT USED: $t1_stale."
          echo "          The three-button page is still fully asserted; the t1 arms are UNRUN — not"
          echo "          passed, and NOT RED either: a worktree mtime is when git last wrote the file,"
          echo "          so this proves the image UNVOUCHED, never proves it wrong."
          echo "          Run tools/fixpoint.sh for one from this tree, or T1_IMAGE=$t1_stale_image to quote this one anyway." ;;
  *)      echo "t1 image: none — T1_IMAGE unset and no stage1.elf in ${FIXPOINT_OUT:-${TMPDIR:-/tmp}/sassembly-fixpoint}."
          echo "          The three-button page is still fully asserted; the t1 arms are UNRUN, not passed."
          echo "          Run tools/fixpoint.sh, or set T1_IMAGE, to arm them." ;;
esac

if [ -n "${SASSEMBLY_PAGE_RESOLVE_ONLY:-}" ]; then
  echo "SASSEMBLY_PAGE_RESOLVE_ONLY: stopping here — no page was built and no arm was run."
  exit 0
fi

if [ -n "${1:-}" ]; then
  page=$1
  [ -f "$page" ] || { echo "  FAIL  $page does not exist."; exit 1; }
  echo "the page's own script, against the already-built $page:"
else
  work=$(mktemp -d) || exit 1
  trap 'rm -rf "$work"' EXIT
  page=$work/page.html
  # Stderr kept in the log and shown only on trouble: whatever the build says is
  # the only thing that distinguishes an absent target from a tree that does not
  # compile, and `build-sassembly-web.sh` is written to say which.
  sh tools/build-sassembly-web.sh "$page" >"$work/build.log" 2>&1
  rc=$?
  if [ "$rc" -eq 77 ]; then
    # `not a tree fault` IS THE PHRASE `salvage` GREPS FOR.
    echo "SKIPPED: the page build cannot run here (wasm32 target absent) — a toolchain fault of this machine, not a tree fault; the page's script is unrun."
    echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
    exit 77
  elif [ "$rc" -ne 0 ]; then
    echo "  FAIL  the page build failed (rc=$rc)."
    sed 's/^/        /' "$work/build.log"
    exit 1
  fi
  echo "the page's own script, against a page built in this run:"
  # ONE GATE RUN, ONE PAGE BUILD. `check-sassembly-identity.sh` builds this same page for
  # its own assertions and neither step read the other's, so `gate.sh` paid the ~25 s
  # build twice over one identical file. This hands THIS build on, vouched by a content
  # digest of the tree it came out of, and `page_handoff_publish` cannot fail: a lost
  # handover costs the later step a build and nothing else.
  #
  # ONLY HERE, and not in the `$1` arm above. A page the operator handed in was built
  # somewhere this script cannot see, and passing it on would make this step a LAUNDERER
  # of provenance — which is `e01c05f5`'s fault one layer up, a `stage1.elf` from
  # `wt-raminject` quoted as this tree's by three siblings that had not fetched it.
  # shellcheck disable=SC2086
  page_handoff_publish "$page" $PAGE_HANDOFF_SIDE
fi

node tools/check-sassembly-page.mjs "$page" || exit 1
