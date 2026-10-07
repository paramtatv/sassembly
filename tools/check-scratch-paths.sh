#!/bin/sh
# tools/check-scratch-paths.sh — W-301
#
# EVERY SCRATCH PATH UNDER $TMPDIR MUST BE UNIQUE ON DISK, NOT MERELY INSIDE ONE
# PROCESS.
#
# Why this exists, measured 2026-09-26. `spec_root_with` named its temp root
# `aksara-spec-<pid>-<counter>` — unique within a process — and never removed it.
# $TMPDIR accumulated 23,528 such directories from 425 DISTINCT pids, and
# `spec/extended-pictographic.tsv` carried 2,074 hard links into them. Pids are
# reused, so a run eventually opens a directory that already holds a link to the
# file it is about to write, and `fs::write` (open with O_TRUNC) then writes
# THROUGH that link into the repository.
#
# It was not theoretical. The rail's own `spec/shiva-sutras.tsv` gained the row
# `5\t9\t\tzz\tphoneme` — byte for byte a test fixture's third probe table — and
# the rail then refused to run for six consecutive fires, because it will not run
# on a dirty tree and only the rail would have cleaned it.
#
# And this machine runs 43 worktrees with several agents gating at once, so a
# FIXED name is not a small risk: two runs simply share the directory.
#
# WHAT THIS CANNOT SEE, so a reader is not surprised by it: the patterns are
# greps over text, so a COMMENT or a string literal under `crates/` that spells
# `temp_dir().join("` reds check 2 as though it were code. That is a false
# positive and the fix is to reword the comment, not to loosen the check — the
# alternative is parsing Rust, which costs far more than this is worth. Both
# checks were verified in BOTH directions: adding one fixed-name site reds check
# 2 and names its file and line; adding one pid-only site takes check 3 to 14
# against the bound of 13; removing them returns it to PASS at 13.
#
# THIS IS A SOURCE-SHAPE RATCHET, NOT A TEST. It greps; it costs about a second.
# It is deliberately NOT added to `gate.sh`: `deep-gate.sh` discovers every
# `tools/check-*.sh` that gate.sh does not run, and hourly is the right cadence
# for a rule about how names are spelled. It is not a per-landing tax.

set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

[ -d crates ] || { echo "CANNOT RUN: no crates/ directory here."; exit 77; }

# ── 1. NON-VACUITY FIRST, because every count below is a grep ────────────────
#
# If `crates/` moves, or the pattern stops matching, every assertion here passes
# while measuring nothing. This project has been wrong that way in three
# different tools, so the floor is asserted before the findings are.
total=$(grep -rn --include='*.rs' 'temp_dir()' crates/ | wc -l | tr -d ' ')
FLOOR=20
if [ "$total" -lt "$FLOOR" ]; then
    echo "CANNOT RUN: only $total temp_dir() site(s) found, expected at least $FLOOR."
    echo "  Either the tree moved or the pattern stopped matching. A zero here is"
    echo "  NOT a clean bill of health — it is a broken reader."
    exit 77
fi
echo "check-scratch-paths: $total temp_dir() reference(s) to read"

# ── 2. NO FIXED NAMES. This one is absolute, not a ratchet ───────────────────
#
# `temp_dir().join("literal")` is a name two concurrent runs share outright.
# There were 12; there are 0, and there is no reason for the number to rise.
fixed=$(grep -rn --include='*.rs' 'temp_dir()\.join("' crates/ || true)
fixed_n=$(printf '%s' "$fixed" | grep -c . || true)
if [ "$fixed_n" -ne 0 ]; then
    echo "RED: $fixed_n scratch path(s) use a FIXED name that concurrent runs share:"
    printf '%s\n' "$fixed" | sed 's/^/    /'
    echo "  Give each one pid AND a nanosecond stamp. Under crates/sadhana-t1/tests/"
    echo "  use spec_fixture::unique_root(prefix), which already does it."
    exit 1
fi
echo "check-scratch-paths: 0 fixed-name scratch paths (was 12 before W-301)"

# ── 3. EVERY NAME CARRIES A CLOCK. The bound reached 0, so it is now absolute ─
#
# `<prefix>-<pid>` and `<prefix>-<pid>-<counter>` are unique inside a process and
# NOT on disk: pids are reused and these roots are never removed. Lower severity
# than a fixed name — the collision is probabilistic rather than certain — so
# these were converted in a second pass. That pass is DONE: the count went
# 13 -> 0 and the bound came down with it.
#
# The bound stays here as a number rather than becoming a bare `-ne 0` so the
# metric below keeps meaning the same thing, and so a deliberate exception would
# have to be written down rather than slipped in. It only ever falls.
#
# The window is 14 lines because the fixed sites carry a four-line margin
# between `temp_dir()` and the clock. A narrower window reported 25 unstamped
# sites including ones that were already fixed: the first version of this
# measurement was wrong, not the code.
command -v python3 >/dev/null 2>&1 || {
    echo "CANNOT RUN: python3 is needed to read the window around each site."
    exit 77
}
CEILING=0
# ── THE READER WALKS FILES. IT USED TO PARSE `grep -A` OUTPUT AND THAT WAS WRONG
#
# The first version keyed on a regex over `grep -rn -A14` output:
#     ([^:\-]+)[:\-](\d+)[:\-](.*)
# The first group stops at a hyphen, so for any path containing one —
# `crates/sadhana-t1/...` — it captured `crates/sadhana`, then failed to find
# digits, and DROPPED THE LINE ENTIRELY. Every site in `sadhana-t1` was invisible,
# which is the crate with the most of them: the check reported 0 unstamped while
# FIVE sites carried no clock (t1_sarani, t1_chain_tables, t1_bare_cross_module_type
# twice, t1_paradigm), and those two families had 894 and 732 directories piled up
# under $TMPDIR.
#
# It was also FORWARD-ONLY, so `spec_fixture::unique_root` — which computes the
# stamp into a variable on the lines BEFORE the join — read as unstamped. One bug
# in each direction.
#
# So: walk the files, and look both ways. No separator ambiguity to get wrong.
reader=$(python3 - <<'READER'
import os
total = 0
bad = []
for root, dirs, files in os.walk("crates"):
    dirs[:] = [d for d in dirs if d != "target"]
    for fn in sorted(files):
        if not fn.endswith(".rs"):
            continue
        path = os.path.join(root, fn)
        lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
        for i, line in enumerate(lines):
            if "temp_dir()" not in line:
                continue
            total += 1
            # BOTH DIRECTIONS: the clock may be computed into a variable above the
            # join, or spelled inline below it.
            window = " ".join(lines[max(0, i - 6):i + 15])
            if "as_nanos" not in window:
                bad.append(f"{path}:{i + 1}")
print(total)
print(len(bad))
for b in bad:
    print(b)
READER
)
seen=$(printf '%s\n' "$reader" | sed -n 1p)
unstamped=$(printf '%s\n' "$reader" | sed -n 2p)
offenders=$(printf '%s\n' "$reader" | sed -n '3,$p')

# ── THE CROSS-CHECK WHOSE ABSENCE LET THE FALSE NEGATIVE THROUGH ─────────────
#
# Two readers count the same thing here: the grep in check 1 and the walker
# above. Nothing compared them, so when the walker silently dropped 7 sites the
# check still passed — it was measuring a smaller population and reporting on it
# confidently. A guard falsified only against the sites its reader can SEE cannot
# catch its reader going blind.
if [ "$seen" -ne "$total" ]; then
    echo "RED: the two readers disagree — grep found $total temp_dir() reference(s),"
    echo "  the walker found $seen. One of them is dropping lines, and until they"
    echo "  agree every count below is about an unknown population."
    exit 1
fi
echo "check-scratch-paths: both readers agree on $seen site(s)"

echo "METRIC sansos_scratch_paths_without_a_clock $unstamped"
if [ "$unstamped" -gt "$CEILING" ]; then
    echo "RED: $unstamped scratch path(s) carry no nanosecond stamp; the bound is $CEILING."
    echo "  A new one was added, or one was converted back. Every scratch name must"
    echo "  carry pid AND a nanosecond stamp; under crates/sadhana-t1/tests/ use"
    echo "  spec_fixture::unique_root(prefix). This bound only falls, never rises."
    exit 1
fi
echo "check-scratch-paths: $unstamped without a clock, bound $CEILING (was 13 before W-301 part two)"

# ── 4. THE SHELL SCRIPTS, WHICH THIS CHECK DID NOT LOOK AT UNTIL W-305 ───────
#
# Checks 1-3 grep `--include=*.rs crates/`. `tools/*.sh` was entirely outside
# them, so this script reported "0 without a clock" while NINETEEN LINES in the
# scripts it lives beside were unguarded — 7 fixed names with no suffix at all, 6
# pid-only, across 13 distinct paths. That is a blind spot in SCOPE, not a
# false negative, and it was found by reading a deep-gate log rather than by the
# check — which is the same way the hyphen bug was found.
#
# 70 of the scripts use `mktemp` and are unique by construction. These are the
# ones that are not. The worst is `check-measurement-host.sh`, which writes
# `/tmp/j.c`, compiles it to `/tmp/j` and EXECUTES it: two concurrent runs and one
# runs the other's binary.
#
# `CARGO_TARGET_DIR` defaults are EXCLUDED on purpose: they key on
# `$(basename "$root")`, the worktree name, so they are already per-worktree.
# THE BOUND IS 8, AND ALL EIGHT ARE KNOWN-BENIGN RATHER THAN OUTSTANDING.
#
# W-305 converted the 11 GENUINE sites to `mktemp`. The other 8 are not host
# scratch paths at all, and THIS CHECK CANNOT TELL — it greps text, so a path in a
# heredoc, a container command or a comment looks exactly like a path the script
# uses. That is the same limitation stated for checks 2 and 3, and I walked into it
# twice while fixing this row:
#
#   demo-package.sh :219 :220 :225 :226  — INSIDE `cat > "$dest/README.md" <<EOF`.
#       They are DOCUMENTATION: example commands printed into a fenced `sh` block
#       for a human to type. "Fixing" them injected shell into the README and, the
#       heredoc being unquoted, expanded `$(mktemp -d)` at write time and left
#       `demo_d: unbound variable`. The script still exited 0 — a failure shaped
#       like a pass.
#   check-measurement-host.sh :58 :78 :79  — INSIDE
#       `docker run --rm "$IMAGE" sh -c '…'`. That is the CONTAINER's /tmp. The
#       attempted fix broke the script too: an apostrophe in a comment closed the
#       outer single-quoted string.
#   rung-answer.sh :56  — `${RUNG_DIR:-/tmp/rung$N}` is a NAMED ARTIFACT directory,
#       keyed by rung number and overridable. Rung artefacts persist for
#       inspection; mktemp would destroy the ability to find them.
#
# So this bound falls only if one of those three judgements is overturned, and it
# must never rise. A NEW line here is a real finding; these eight are not.
#
# AND THIS FILE'S OWN MARGIN CANNOT RED THIS CHECK — verified, not assumed. The
# text above names `/tmp/j.c`, `/tmp/rung$N` and the other paths it discusses, so
# check 4 scanning `tools/*.sh` scans its own documentation. Measured 2026-09-27:
# 2 raw hits in this file, 0 after the `^[^:]+:[0-9]+: *#` comment filter. Checks
# 1-3 are safe by scope — they read `crates/**/*.rs` and this is `tools/*.sh`.
#
# That is worth stating because the opposite has happened SIX times in one session:
# a search matching prose ABOUT the code rather than the code — a row naming the
# symbol it deleted, a comment naming the pattern it replaced, and once a failure
# message quoting the very signature it told the reader to look for, which sent the
# reader down the wrong branch of its own decision procedure. Anchor on syntax
# prose cannot hold, scope to code paths, and read `grep -n` rather than `grep -c`
# whenever the answer decides something.
TOOLS_CEILING=8
tools_unsafe=$(grep -nE '(/tmp/|TMPDIR[:}]*/)[a-zA-Z0-9_.-]+' tools/*.sh 2>/dev/null \
    | grep -v mktemp \
    | grep -v CARGO_TARGET_DIR \
    | grep -vE '^[^:]+:[0-9]+: *#' \
    | grep -c . || true)
echo "METRIC sansos_tools_scratch_paths_unsafe ${tools_unsafe:-0}"
if [ "${tools_unsafe:-0}" -gt "$TOOLS_CEILING" ]; then
    echo "RED: $tools_unsafe fixed or pid-only scratch path(s) in tools/*.sh; the bound is $TOOLS_CEILING."
    echo "  70 of these scripts use mktemp and are unique by construction. Use it, or"
    echo "  add pid AND a nanosecond stamp. This bound only falls."
    grep -nE '(/tmp/|TMPDIR[:}]*/)[a-zA-Z0-9_.-]+' tools/*.sh 2>/dev/null \
        | grep -v mktemp | grep -v CARGO_TARGET_DIR | grep -vE '^[^:]+:[0-9]+: *#' \
        | sed 's/^/    /' | head -20
    exit 1
fi
echo "check-scratch-paths: $tools_unsafe unsafe path(s) in tools/*.sh, bound $TOOLS_CEILING (W-305)"

echo "check-scratch-paths: PASS"
exit 0
