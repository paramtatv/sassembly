#!/bin/sh
# tools/check-adr-discriminator.sh — W-334
#
# WHEN A COMMENT CITES ONE OF THE SIX AMBIGUOUS ADR NUMBERS, IT MUST APPEND THE
# TASK ID: `ADR-0026 [D-002k]`.
#
# Why this exists. Ruling 1 of the owner's second set, 2026-09-30, has two
# halves. The first — renumber the 605 existing citations — stays DROPPED
# (`W-157`, deferred past v0.5.0). The second is a rule about writing: six ADR
# numbers mean two different documents depending on which branch's history you
# are reading, so a bare citation of one of them is ambiguous, and the citer is
# the only person who knows which was meant. `W-334` is that half.
#
# AND A WRITTEN RULE DOES NOT FIRE ON ITS OWN. The row that filed this carries
# the proof: `a-file-that-keeps-its-history-hands-a-grep-the-past` was already
# written down in this project and still misled two agents independently, on the
# same file, in the same hour, from opposite sides. A discriminator that lives
# only in `STATE.md` is obeyed by whoever read `STATE.md` this week. The ruling
# says ENFORCE, and enforcement is a reader.
#
# A RATCHET, NOT A SWEEP. The existing bare citations are NOT in scope and must
# not be rewritten — that is precisely the bulk `sed` the ruling refused, and
# `W-157` records why it cannot be done by pattern: `D-002k`/`D-002l` are this
# branch's array and module ADRs while `D-002h` is origin's operator/statement
# work, so one string means two things in the same file. So this pins a
# PER-FILE BASELINE and refuses only a RISE. Falling is always allowed; a repair
# is never blocked by the guard that measures it.
#
# WHY PER-FILE AND NOT ONE TOTAL. Every citation in scope is bare today — 367 of
# 367 — so a single total cannot tell a NEW bare citation from an old one, and
# "the count went to 368" names nothing a committer can fix. Per-file counts are
# stable against the line shifts that unrelated edits cause, so a rise is
# attributable to a file, and the check then prints every bare citation in THAT
# file with its line number. It over-names within the one risen file; it never
# sends the reader to a file that did not change.
#
# WHAT THIS CANNOT SEE, said rather than left for a reader to find:
#
#   * IT MEASURES PRESENCE, NOT TRUTH. `ADR-0026 [D-002h]` is well-formed and
#     wrong, and nothing here catches it. Checking the id would need the very
#     branch-to-document mapping the dropped renumber needs, which is the work
#     that is deferred — `a-twin-check-never-compares-against-truth`.
#   * IT IS A GREP. A citation inside a string literal, a fenced code block or a
#     quoted error message looks exactly like a citation in a comment.
#   * IT READS TRACKED FILES ONLY (`git ls-files`, `git grep`). A citation in a
#     file that has never been `git add`ed is invisible to both readers here.
#
# ALSO OUT OF SCOPE: spec/golden/**/fixture/** and spec/golden/**/pool/**. These are frozen
# snapshot inputs of the g0n golden harness: earlier revisions and mutants of tracked .t1
# sources that are already counted above. Every distinct citation line in them occurs
# verbatim in some revision of a counted file (proved by comm, MAINRED-FIX.md), so
# scanning them adds no citation, only a per-snapshot copy of an old one.
#
# OUT OF SCOPE BY DESIGN, per the row's acceptance (e): `BACKLOG*.tsv`,
# `.loop/`, and `docs/adr/` itself. Those three QUOTE these strings as data —
# the ledger rows discuss the renumber, and the ADRs are the documents — so
# scanning them makes the guard noisiest where it is least useful.
#
# THIS FILE'S OWN MARGIN CANNOT RED THIS CHECK, and that is enforced rather than
# hoped: every ambiguous number spelled above and below carries a discriminator,
# so this script scans itself and contributes 0. Verified by running it after
# writing it, not argued. Six earlier tools in this project matched prose ABOUT
# the code rather than the code; this one is in its own population.
#
# NOT ON THE LANDING PATH. `deep-gate.sh:259` discovers every `tools/check-*.sh`
# that `gate.sh` does not run and runs it hourly. A rule about how a citation is
# spelled belongs at that cadence, not as a per-landing tax — the same placement
# `check-scratch-paths.sh` documents.

set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

[ -d crates ] || { echo "CANNOT RUN: no crates/ directory here."; exit 77; }
command -v git >/dev/null 2>&1 || { echo "CANNOT RUN: git is needed to enumerate tracked files."; exit 77; }

# ── 1. NON-VACUITY FIRST, because every count below is a grep ────────────────
#
# If the exclusions widen, or `ADR-` stops being how a citation is spelled, every
# assertion below passes while measuring nothing. This project has been wrong
# that way in three different tools, so the floor is asserted before the findings.
grep_total=$(git grep -hoE 'ADR-00(2[6-9]|3[01])' -- . \
    ':(exclude).loop' ':(exclude)docs/adr' ':(exclude)BACKLOG*.tsv' \
    ':(exclude,glob)spec/golden/**/fixture/**' ':(exclude,glob)spec/golden/**/pool/**' \
    | grep -c . || true)
FLOOR=300
if [ "${grep_total:-0}" -lt "$FLOOR" ]; then
    echo "CANNOT RUN: only ${grep_total:-0} ambiguous ADR citation(s) in scope, expected at least $FLOOR."
    echo "  Either the exclusions widened or the pattern stopped matching. A zero"
    echo "  here is NOT a clean bill of health — it is a broken reader."
    exit 77
fi
echo "check-adr-discriminator: $grep_total ambiguous citation(s) in scope to read"

# ── 2. THE BASELINE, MEASURED 2026-10-03 AND RE-MEASURED, NOT COPIED ─────────
#
# The filing row's figure was 605 over the WHOLE repository, taken before `W-332`
# landed and before the three exclusions were decided. It said in so many words
# that the number must be re-measured at implementation time —
# `an-absence-measured-too-early-is-not-an-absence`. In scope, today, it is 367
# across 57 files. The `TOTAL` line is asserted against the sum below so the
# table cannot be edited into disagreement with itself.
BASELINE=$(cat <<'TABLE'
crates/sadhana-t1/src/artha.t1	5
crates/sadhana-t1/src/ast.t1	2
crates/sadhana-t1/src/encode.t1	26
crates/sadhana-t1/src/ir.t1	4
crates/sadhana-t1/src/lex.t1	1
crates/sadhana-t1/src/lib.t1	1
crates/sadhana-t1/src/parse.t1	3
crates/sadhana-t1/src/samyojana.t1	3
crates/sadhana-t1/src/sanskrit_text.t1	3
crates/sadhana-t1/src/utsarjana.t1	12
crates/sadhana-t1/src/vakyavibhaga.t1	1
crates/sadhana-t1/src/vishlesana.t1	3
crates/tree-sitter-t1/grammar.js	8
notebooks/03-paramtatva-base-model.ipynb	2
notebooks/11-operators.ipynb	2
notebooks/17-paramtatva-ops-execution-storage.ipynb	2
notebooks/vastu-data-only-object.ipynb	1
research/15-orthographic-closure.md	2
research/23-paradigm-statistics-plan.md	1
research/24-paramtatva-impacts.md	2
research/27-twin-agreement.md	1
spec/grammar-t1.ebnf	39
spec/lexicon.src.tsv	13
spec/lexicon.tsv	14
spec/registers-amd64.tsv	4
tests/corpus/CATALOG.tsv	1
TOTAL	156
TABLE
)

# ── 3. THE READER, WHICH SELF-TESTS ITS CLASSIFIER BEFORE IT TRUSTS IT ───────
#
# The classifier decides one thing: does this citation carry a `[<task-id>]`
# suffix. Today NOTHING in the tree carries one, so the accepting half of that
# decision is exercised by NO real input — a guard falsified only against inputs
# it can see cannot catch its own accepting arm being broken, and an arm that is
# wrong in the accepting direction would make the whole ratchet unsatisfiable:
# a committer appends the discriminator the message asks for and the count does
# not move. So the cases are pinned here, INCLUDING the ones that must still be
# REFUSED, and a disagreement is exit 77 and not a quiet pass.
# THE TABLE TRAVELS IN THE ENVIRONMENT, NOT ON STDIN. The first version piped it
# into `python3 - <<READER`, and the heredoc IS stdin — so the pipe was shadowed,
# the reader saw an empty table and reported TABLE-BROKEN. It exited 77 rather
# than passing, which is the only reason that was visible at all.
reader=$(git ls-files -z | tr '\0' '\n' | SANSOS_ADR_BASELINE="$BASELINE" LC_ALL=C awk '
function bare(line, endpos,   rest) {
    rest = substr(line, endpos)
    return !match(rest, /^[ \t]*\[[A-Z][A-Z0-9]*-[0-9]+[a-z0-9]*\]/)
}
function in_scope(p,   b) {
    if (p == "" || index(p, ".loop/") == 1 || index(p, "docs/adr/") == 1) return 0
    if (p ~ /^spec\/golden\// && (p ~ /\/fixture\// || p ~ /\/pool\//)) return 0
    b = p; sub(/^.*\//, "", b)
    return !(index(b, "BACKLOG") == 1 && b ~ /\.tsv$/)
}
BEGIN {
    # THE CASES. Citations are concatenated, not spelled whole: this file is itself in scope and
    # four of the six must be BARE for the refusing arm to mean anything.
    A = "ADR-00"
    c[1] = "cites " A "26 [D-002k] here";           w[1] = 0
    c[2] = A "31[W-334]";                           w[2] = 0
    c[3] = "see " A "29 for the array layout";      w[3] = 1
    c[4] = A "30, and also elsewhere";              w[4] = 1
    c[5] = A "28 [see below]";                      w[5] = 1
    c[6] = A "27 and then later [D-002h]";          w[6] = 1
    for (k = 1; k <= 6; k++) {
        if (!match(c[k], /ADR-00(2[6-9]|3[01])/)) { printf "SELFTEST-BROKEN\t%s\n", c[k]; exit }
        g = (bare(c[k], RSTART + RLENGTH) ? 1 : 0)
        if (g != w[k]) { printf "SELFTEST-FAILED\t%s\t%s\t%s\n", c[k], (w[k] ? "True" : "False"), (g ? "True" : "False"); exit }
    }
    # THE BASELINE TABLE (arrives in the environment: stdin is the file list).
    nrows = split(ENVIRON["SANSOS_ADR_BASELINE"], rows, "\n"); dt = ""; sum = 0
    for (k = 1; k <= nrows; k++) {
        if (rows[k] ~ /^[ \t\r\f\v]*$/) continue
        split(rows[k], kv, "\t")
        if (kv[1] == "TOTAL") dt = kv[2] + 0
        else want[kv[1]] = kv[2] + 0
    }
    for (p in want) sum += want[p]
    if (dt == "" || dt != sum) { printf "TABLE-BROKEN\t%s\t%d\n", (dt == "" ? "None" : dt), sum; exit }
    # THE SECOND READER: walk the tracked files itself.
    seen = 0; tot = 0; nf = 0
    while ((getline p) > 0) {
        if (!in_scope(p)) continue
        ln = 0
        while ((r = (getline line < p)) > 0) {
            ln++; sub(/\r$/, "", line); off = 0; rest = line
            while (match(rest, /ADR-00(2[6-9]|3[01])/)) {
                seen++; endpos = off + RSTART + RLENGTH
                if (bare(line, endpos)) {
                    if (!(p in got)) nf++
                    got[p]++; tot++
                    lines_of[p] = ((p in lines_of) ? lines_of[p] "," : "") ln
                }
                off = endpos - 1; rest = substr(line, endpos)
            }
        }
        close(p)
    }
    printf "OK\t%d\t%d\t%d\n", seen, tot, nf
    for (p in got) if (got[p] > ((p in want) ? want[p] : 0)) printf "RISEN\t%s\t%d\t%d\t%s\n", p, ((p in want) ? want[p] : 0), got[p], lines_of[p]
}' | { IFS= read -r _first || true; printf '%s\n' "$_first"; LC_ALL=C sort; })

verdict=$(printf '%s\n' "$reader" | sed -n 1p | cut -f1)
case "$verdict" in
    SELFTEST-BROKEN|SELFTEST-FAILED)
        echo "CANNOT RUN: the citation classifier failed its own cases:"
        printf '%s\n' "$reader" | sed 's/^/    /'
        echo "  Until it agrees with them, every count below is about an unknown"
        echo "  population. Fix the classifier, not the cases."
        exit 77 ;;
    TABLE-BROKEN)
        echo "CANNOT RUN: the pinned baseline table disagrees with its own TOTAL:"
        printf '%s\n' "$reader" | sed 's/^/    /'
        echo "  A per-file line was edited without the total, or the reverse."
        exit 77 ;;
    OK) ;;
    *)
        echo "CANNOT RUN: the reader produced no verdict. Raw output:"
        printf '%s\n' "$reader" | sed 's/^/    /'
        exit 77 ;;
esac
echo "check-adr-discriminator: classifier passed 6 pinned cases (2 accept, 4 refuse)"

seen=$(printf '%s\n' "$reader" | sed -n 1p | cut -f2)
bare_n=$(printf '%s\n' "$reader" | sed -n 1p | cut -f3)
file_n=$(printf '%s\n' "$reader" | sed -n 1p | cut -f4)

# ── 4. THE CROSS-CHECK WHOSE ABSENCE LET AN EARLIER TOOL GO BLIND ────────────
if [ "$seen" -ne "$grep_total" ]; then
    echo "RED: the two readers disagree — git grep found $grep_total citation(s),"
    echo "  the walker found $seen. One of them is dropping lines, and until they"
    echo "  agree every count below is about an unknown population."
    exit 1
fi
echo "check-adr-discriminator: both readers agree on $seen citation(s)"

# ── 5. THE RATCHET ───────────────────────────────────────────────────────────
echo "METRIC sansos_adr_citations_without_a_discriminator $bare_n"
risen=$(printf '%s\n' "$reader" | sed -n '2,$p' | grep -c . || true)
if [ "${risen:-0}" -ne 0 ]; then
    echo "RED: $risen file(s) gained a bare citation of an ambiguous ADR number."
    printf '%s\n' "$reader" | sed -n '2,$p' | while IFS="$(printf '\t')" read -r _ path was now lines; do
        echo "    $path — baseline $was, now $now"
        echo "      bare citation(s) at line(s): $lines"
    done
    echo "  Six ADR numbers mean two different documents depending on which"
    echo "  branch's history you are reading, so append the task id you meant:"
    echo "      ADR-0026 [D-002k]"
    echo "  The existing citations are NOT to be swept — that renumber is W-157"
    echo "  and it is deferred. This bound only falls."
    exit 1
fi
echo "check-adr-discriminator: $bare_n bare citation(s) across $file_n file(s), at or below every pinned baseline"
echo "check-adr-discriminator: PASS"
exit 0
