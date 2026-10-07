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
command -v python3 >/dev/null 2>&1 || { echo "CANNOT RUN: python3 is needed to classify each citation."; exit 77; }

# ── 1. NON-VACUITY FIRST, because every count below is a grep ────────────────
#
# If the exclusions widen, or `ADR-` stops being how a citation is spelled, every
# assertion below passes while measuring nothing. This project has been wrong
# that way in three different tools, so the floor is asserted before the findings.
grep_total=$(git grep -hoE 'ADR-00(2[6-9]|3[01])' -- . \
    ':(exclude).loop' ':(exclude)docs/adr' ':(exclude)BACKLOG*.tsv' \
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
crates/lexgen/src/main.rs	2
crates/metrics/tests/paradigm_history.rs	1
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
crates/sadhana-t1/tests/t1_exec_aksara.rs	7
crates/sadhana-t1/tests/t1_execution.rs	7
crates/sadhana-t1/tests/t1_modules.rs	3
crates/sadhana-t1/tests/t1_paradigm_names.rs	2
crates/sadhana-t1/tests/t1_sources.rs	19
crates/sadhana-t1/tests/t1_unreached_modules.rs	2
crates/sadhana-t1/tests/w250-shares.rs	5
crates/sadhana-t1/tests/w268-census.rs	1
crates/sadhana-t1/tests/w304-pass-zero-scan.rs	1
crates/sadhana-t1/tests/w342_parse_refusal_bheda.rs	3
crates/sadhana/src/t1/abi.rs	1
crates/sadhana/src/t1/ast.rs	3
crates/sadhana/src/t1/ir.rs	2
crates/sadhana/src/t1/mandala.rs	1
crates/sadhana/src/t1/nirvahana.rs	2
crates/sadhana/src/t1/parse.rs	4
crates/sadhana/src/t1/resolve.rs	1
crates/sadhana/src/t1/riscv64.rs	1
crates/sadhana/src/t1/typecheck.rs	1
crates/sadhana/src/t1/types.rs	1
crates/sadhana/tests/paradigm_convention.rs	1
crates/sadhana/tests/t1_fixed_capacity_array.rs	5
crates/sadhana/tests/t1_operators.rs	1
crates/sadhana/tests/w342_parse_refusal_reaches_the_exit.rs	1
crates/sanskrit-text/tests/grammar_t1.rs	129
crates/tree-sitter-t1/grammar.js	8
crates/yantra/src/lib.rs	1
crates/yantra/tests/t1_storage_witness.rs	1
docs/storage-modelling-method.md	1
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
tools/gen-grapheme-break.py	1
TOTAL	367
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
reader=$(SANSOS_ADR_BASELINE="$BASELINE" python3 - <<'READER'
import os, re, subprocess, sys

AMB  = re.compile(r'ADR-00(?:2[6-9]|3[01])')
# `ADR-0026 [D-002k]`. One optional run of blanks, then an upper-case track, a
# dash, digits, and the optional lower-case/digit tail that `D-002k`,
# `F-016agy5` and `C-014b` all have.
WELL = re.compile(r'[ \t]*\[[A-Z][A-Z0-9]*-[0-9]+[a-z0-9]*\]')

def bare(line, end):
    return WELL.match(line, end) is None

# ── THE CASES. Two that must be ACCEPTED as carrying a discriminator, and four
# that must still be REFUSED. The last two are the ones a looser suffix pattern
# gets wrong: a bracket that holds something other than a task id, and a
# discriminator that arrives too late to belong to this citation.
#
# EACH CITATION IS CONCATENATED RATHER THAN SPELLED, and that is not decoration.
# Four of these six fixtures must be BARE for the refusing arm to mean anything,
# and this file is itself in scope — so spelling them whole put four bare
# citations into the guard's own population and it red-lined itself the moment it
# was `git add`ed, against its own baseline of 0. Found by adding the file and
# running it, which is why that step is in this row's gate and not assumed.
# Joining the halves at run time keeps the fixture intact for the classifier and
# invisible to the grep, so the guard's population stays the product's text.
A = "ADR-00"
CASES = [
    ("cites " + A + "26 [D-002k] here",           False),
    (A + "31[W-334]",                             False),
    ("see " + A + "29 for the array layout",      True),
    (A + "30, and also elsewhere",                True),
    (A + "28 [see below]",                        True),
    (A + "27 and then later [D-002h]",            True),
]
for text, want in CASES:
    m = AMB.search(text)
    if m is None:
        print("SELFTEST-BROKEN", text, sep="\t"); sys.exit(0)
    got = bare(text, m.end())
    if got != want:
        print("SELFTEST-FAILED", text, want, got, sep="\t"); sys.exit(0)

# ── THE BASELINE TABLE. The shell owns the one copy; it arrives in the
# environment because this script itself is on stdin.
want = {}
declared_total = None
for row in os.environ.get("SANSOS_ADR_BASELINE", "").split("\n"):
    if not row.strip():
        continue
    path, n = row.split("\t")
    if path == "TOTAL":
        declared_total = int(n)
    else:
        want[path] = int(n)
if declared_total is None or declared_total != sum(want.values()):
    print("TABLE-BROKEN", declared_total, sum(want.values()), sep="\t"); sys.exit(0)

# ── THE SECOND READER. The shell's `git grep -ho` above counted OCCURRENCES of
# the same pattern over the same pathspec; this walks the tracked files itself.
# Nothing compared the two in `check-scratch-paths.sh`'s first version, and when
# its walker silently dropped seven sites the check still passed — measuring a
# smaller population and reporting on it confidently.
EXCLUDE_DIRS = (".loop/", "docs/adr/")
def in_scope(p):
    if not p or p.startswith(EXCLUDE_DIRS):
        return False
    b = os.path.basename(p)
    return not (b.startswith("BACKLOG") and b.endswith(".tsv"))

files = subprocess.run(["git", "ls-files", "-z"], capture_output=True, text=True,
                       check=True).stdout.split("\0")
seen = 0
got = {}
lines_of = {}
for p in files:
    if not in_scope(p):
        continue
    try:
        text = open(p, encoding="utf-8", errors="replace").read()
    except OSError:
        continue            # deleted-but-tracked; git grep cannot read it either
    for i, line in enumerate(text.split("\n")):
        for m in AMB.finditer(line):
            seen += 1
            if bare(line, m.end()):
                got[p] = got.get(p, 0) + 1
                lines_of.setdefault(p, []).append(i + 1)

risen = sorted(p for p in got if got[p] > want.get(p, 0))
print("OK", seen, sum(got.values()), len(got), sep="\t")
for p in risen:
    print("RISEN", p, want.get(p, 0), got[p],
          ",".join(str(n) for n in lines_of[p]), sep="\t")
READER
)

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
