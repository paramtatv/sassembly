#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# Task B-034b — the tree-sitter grammar must agree with sadhana's parse.
#
# Checked by COUNTING, in both directions, against `spec/parse-shape-t0.tsv`.
#
# ## Why not "no ERROR nodes"
#
# Because that check already passed on a wrong tree. Cycle 128 verified this
# grammar's ancestor by the absence of ERROR nodes, and it passed on files where
# 23 instructions had been swallowed into a single directive — a directive rule
# that opened on `॥` and never closed consumed the rest of the file, and a tree
# that says "one directive" where the truth is "one directive and 23
# instructions" contains no error to find. Absence of an error is not presence
# of a parse.
#
# This cycle produced the same class of defect a second time, in the grammar
# rather than the checker: every kāraka sigil character is also a word
# character, so `word` matched each operand to exactly the same length as
# `operand` and won the tie. The `operand` rule matched NOTHING across all 39
# files — and the corpus still parsed with zero ERROR nodes. Hence the second
# assertion below: every declared rule must fire at least once.
#
# ## What is compared
#
# Row for row: (file, line, kind, name) from this grammar against the oracle
# `sadhana` generates. Both directions, because a grammar that finds a superset
# is as wrong as one that finds a subset, and counting only totals would let a
# label lost here and an instruction invented there cancel out.
#
# Usage: tools/check-grammar.sh
set -uo pipefail
cd "$(dirname "$0")/.."

GRAMMAR=grammar/tree-sitter-sassembly
ORACLE=spec/parse-shape-t0.tsv

if ! command -v tree-sitter >/dev/null 2>&1; then
    echo "SKIPPED: tree-sitter is not installed (B-102 approved it: brew install tree-sitter-cli)"
    echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
    exit 77
fi

# Regenerate, so the check can never pass against a parser built from a grammar
# that no longer exists. `--js-runtime native` is the condition B-102 approved:
# no JavaScript runtime is involved.
if ! (cd "$GRAMMAR" && tree-sitter generate --js-runtime native >/dev/null 2>&1); then
    echo "  FAIL  tree-sitter generate failed"
    (cd "$GRAMMAR" && tree-sitter generate --js-runtime native 2>&1 | tail -20 | sed 's/^/        /')
    exit 1
fi

# The comparison is plain shell (awk/sort/comm); no interpreter is involved.
export LC_ALL=C
TAB=$(printf '\t')
T=$(mktemp -d "${TMPDIR:-/tmp}/check-grammar.XXXXXX") || exit 1
trap 'rm -rf "$T"' EXIT

# The kinds the oracle records. A node kind this grammar produces that is not
# here is invisible to the comparison, so the list is asserted below.
KINDS="directive instruction label"

# THE ORACLE DEFINES THE CORPUS, so it is read before the sources and not after.
#
# `extra = mine - oracle` calls every statement the grammar finds in a file the
# oracle does not cover an INVENTION. So globbing `**/*.sas` did not widen the
# check, it broke it: the T1 test programs under `crates/sadhana/src/t1/tests/`
# are written in T1 and were being held against the T0 grammar, and `drafts/`
# is by definition the programs that do NOT assemble, so `sadhana` cannot state
# a truthful oracle row for them. Together they reported 18165 "disagreements"
# that were a category error, not a grammar defect.
#
# Narrowing a check is how a check quietly stops checking, so the skipped files
# are COUNTED AND NAMED below rather than dropped in silence. This is safe in
# one specific way: `parse_shape`'s `the_oracle_and_the_tree_name_the_same
# _programs` already enforces file<->row correspondence in BOTH directions, so a
# new `spec/` program cannot hide here by simply having no oracle row — that
# test goes red first.
awk -F'\t' 'BEGIN { OFS = "\t" }
    /^#/ { next }
    $0 ~ /^[ \t\r\n\f\v]*$/ { next }
    NF == 4 && $1 != "file" { print $1, $2 + 0, $3, $4 }' "$ORACLE" | sort -u > "$T/oracle"
cut -f1 "$T/oracle" | sort -u > "$T/oracle_files"

find . -name '*.sas' | sed 's|^\./||' | grep -v 'target/' | sort -u > "$T/found"
comm -12 "$T/found" "$T/oracle_files" > "$T/sources"
comm -23 "$T/found" "$T/oracle_files" > "$T/skipped"
nsources=$(wc -l < "$T/sources" | tr -d ' ')
nskipped=$(wc -l < "$T/skipped" | tr -d ' ')
if [ "$nsources" -eq 0 ]; then
    echo "  FAIL  no .sas files found — this check would pass having compared nothing"
    exit 1
fi
if [ "$nskipped" -gt 0 ]; then
    echo "  note  $nskipped .sas file(s) carry no oracle row and are not T0 corpus:"
    head -4 "$T/skipped" | while IFS= read -r s; do echo "          $s"; done
    if [ "$nskipped" -gt 4 ]; then
        echo "          ... and $((nskipped - 4)) more"
    fi
fi

# Row for row: tree-sitter prints `(kind [r, c] - [r, c]` for each node. The
# name is the statement's `name:` field — the next node in the sexp (within two
# nodes). Its bytes are read out of the source (byte columns, hence LC_ALL=C)
# rather than re-derived, so this compares what the parser saw and not what
# this script thinks the syntax is. A label's name excludes the `ॱॱ` mark, as
# the oracle records the label, not the mark.
: > "$T/mine.raw"; : > "$T/rules"; : > "$T/errors"
here=$(pwd -P)
while IFS= read -r src; do
    # Absolute: tree-sitter runs with cwd=grammar dir so it finds the parser,
    # and a path relative to the repo root would silently not exist there. The
    # first version did exactly that, found 0 nodes in all 39 files, and this
    # check reported 179 disagreements rather than a pass — which is the whole
    # point of comparing against an oracle instead of counting errors.
    # `tree-sitter parse` exits non-zero when the tree contains an ERROR, and
    # prints the tree either way. Both are wanted: the tree to compare, the
    # status to report.
    (cd "$GRAMMAR" && tree-sitter parse "$here/$src" 2>/dev/null) > "$T/tree"
    awk -v src="$src" -v mine="$T/mine.raw" -v rules="$T/rules" -v errs="$T/errors" '
        BEGIN {
            re = "\\([A-Za-z0-9_]+ \\[[0-9]+, [0-9]+\\] - \\[[0-9]+, [0-9]+\\]"
            mark = "\340\245\261\340\245\261"
        }
        FNR == NR { L[FNR - 1] = $0; next }
        {
            if (index($0, "ERROR") || index($0, "MISSING")) bad = 1
            s = $0
            while (match(s, re)) {
                piece = substr(s, RSTART, RLENGTH)
                s = substr(s, RSTART + RLENGTH)
                n++
                K[n] = substr(piece, 2, index(piece, " ") - 2)
                rest = substr(piece, index(piece, "["))
                gsub(/[^0-9]+/, " ", rest)
                split(rest, a, " ")
                R[n] = a[1] + 0; C[n] = a[2] + 0; E[n] = a[4] + 0
            }
        }
        END {
            if (bad) print src ": parse tree contains ERROR/MISSING" >> errs
            for (i = 1; i <= n; i++) {
                kind = K[i]
                print kind >> rules
                if (kind != "instruction" && kind != "label" && kind != "directive") continue
                found = 0
                for (j = i + 1; j <= i + 2 && j <= n; j++) {
                    if (K[j] == "mnemonic" || K[j] == "label_name" || K[j] == "word") {
                        name = substr(L[R[j]], C[j] + 1, E[j] - C[j])
                        found = 1
                        break
                    }
                }
                if (!found) {
                    print src ":" (R[i] + 1) ": " kind " with no name node" >> errs
                    continue
                }
                if (kind == "label" && length(name) >= 6 && substr(name, length(name) - 5) == mark)
                    name = substr(name, 1, length(name) - 6)
                print src "\t" (R[i] + 1) "\t" kind "\t" name >> mine
            }
        }' "$src" "$T/tree"
done < "$T/sources"
sort -u "$T/mine.raw" > "$T/mine"

if [ ! -s "$T/oracle" ]; then
    echo "  FAIL  parsed 0 rows out of $ORACLE — its format moved"
    exit 1
fi

comm -23 "$T/oracle" "$T/mine" > "$T/missing.u"     # sadhana saw it, the grammar did not
comm -13 "$T/oracle" "$T/mine" > "$T/extra.u"       # the grammar invented it
srt() { sort -t "$TAB" -k1,1 -k2,2n -k3,3 -k4,4 "$1"; }
srt "$T/missing.u" > "$T/missing"
srt "$T/extra.u" > "$T/extra"
cnt() { wc -l < "$1" | tr -d ' '; }
nor=$(cnt "$T/oracle"); nmi=$(cnt "$T/mine")

# Counts by kind, which is what catches statements swallowed wholesale.
echo "  kind          oracle   grammar"
for k in $KINDS; do
    o=$(awk -F'\t' -v k="$k" '$3 == k { n++ } END { print n + 0 }' "$T/oracle")
    g=$(awk -F'\t' -v k="$k" '$3 == k { n++ } END { print n + 0 }' "$T/mine")
    flag=''
    [ "$o" -eq "$g" ] || flag='   <-- DISAGREES'
    printf '  %-12s %7d %9d%s\n' "$k" "$o" "$g" "$flag"
done
printf '  %-12s %7d %9d\n' total "$nor" "$nmi"

# Every declared rule must fire. A rule that never matches is a rule that is
# not doing what it says, and the parse stays clean while it happens.
# A leading underscore means HIDDEN: tree-sitter inlines those rules and they
# can never appear as a node, so demanding they fire would fail forever and the
# real assertion below would be discarded to silence it.
sort -u "$T/rules" > "$T/rules.u"
awk '/^    [A-Za-z0-9_]+: \$ =>/ { s = $1; sub(/:$/, "", s); print s }' "$GRAMMAR/grammar.js" \
    | grep -v '^_' | grep -vx 'source_file' | sort -u > "$T/declared" || true
comm -23 "$T/declared" "$T/rules.u" > "$T/never"
nnever=$(cnt "$T/never")
if [ "$nnever" -gt 0 ]; then
    list=$(awk 'BEGIN { printf "[" } { printf "%s'"'"'%s'"'"'", (NR > 1 ? ", " : ""), $0 } END { printf "]" }' "$T/never")
    printf '\n  FAIL  declared but never matched anywhere in %s files: %s\n' "$nsources" "$list"
    echo "        a rule that cannot fire is not a rule; the corpus still parses clean"
fi

head -10 "$T/errors" | while IFS= read -r e; do echo "  FAIL  $e"; done
head -10 "$T/missing" | awk -F'\t' '{ printf "  FAIL  sadhana parsed %s `%s` at %s:%s; the grammar did not\n", $3, $4, $1, $2 }'
head -10 "$T/extra" | awk -F'\t' '{ printf "  FAIL  the grammar invented %s `%s` at %s:%s\n", $3, $4, $1, $2 }'

nmis=$(cnt "$T/missing"); nex=$(cnt "$T/extra"); nerr=$(cnt "$T/errors")
if [ "$nmis" -gt 0 ] || [ "$nex" -gt 0 ] || [ "$nerr" -gt 0 ] || [ "$nnever" -gt 0 ]; then
    n=$((nmis + nex + nerr + nnever))
    printf '\n%s disagreement(s) with sadhana. The grammar is not the language.\n' "$n"
    exit 1
fi

printf '\ngrammar agrees with sadhana on all %s statements in %s files.\n' "$nor" "$nsources"
