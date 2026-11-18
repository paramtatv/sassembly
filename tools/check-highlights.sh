#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# Task B-034c — the T0 highlight queries must classify every operand, correctly.
#
# A highlighter fails quietly by construction: a rule that never matches leaves
# text the same colour as the text around it, and nobody files a bug about a
# word that is not blue. So this checks the three things that can go wrong
# without anyone noticing.
#
#   1. The queries are STALE. They are generated from three spec tables; a
#      register added to the table and not regenerated stops being highlighted.
#      Checked by regenerating and comparing bytes.
#
#   2. An operand falls through every rule. Checked by requiring every operand
#      in all 39 .sas files to receive a capture.
#
#   3. An operand is classified WRONG. This is the one that needs an
#      independent witness, because the register rule was generated from the
#      register table and comparing it back to that table proves nothing.
#
# The witness for (3) is `spec/parse-shape-t0.tsv` — `sadhana`'s parse, which
# knows where every label in the tree is DEFINED. An operand this query calls a
# label reference must name a label that exists. That is a fact neither the
# generator nor the grammar had access to, and it is what makes the check real
# rather than a restatement.
#
# Usage: tools/check-highlights.sh
# Re-exec under bash if invoked as `sh tools/check-highlights.sh` — the same
# defect `W-052` fixed in `check-reproducible.sh`, left standing here. macOS
# `/bin/sh` is bash 3.2 in POSIX mode and rejects the process substitution at
# the `diff` below (:45): the script aborts with `syntax error near unexpected
# token ('` HAVING ALREADY PRINTED its operand tallies, so a reader who pipes
# it to `tail` sees numbers and a status that is the pipe's. A failure shaped
# like a pass.
#
# MEASURED 2026-09-24: `sh tools/check-highlights.sh` exits 2 on the syntax
# error; `bash tools/check-highlights.sh` exits 0 having classified 22,087
# operands. Both were run — the exit-2 was read as a tree fault first, and it
# was the invocation.
#
# These two are the only scripts in `tools/` that use process substitution, and
# only one carried the guard. The obvious test — `[ -n "$BASH_VERSION" ]` —
# does not work and `check-reproducible.sh` says why: bash sets that variable
# when invoked as `sh` too, so it reports the interpreter and not the dialect.
# Ask for the capability instead; `eval` defers the parse into a subshell, so
# probing for process substitution cannot itself be the syntax error.
(eval ': <(:)') 2>/dev/null || exec /usr/bin/env bash "$0" "$@"

set -uo pipefail
cd "$(dirname "$0")/.."

GRAMMAR=grammar/tree-sitter-sassembly
QUERIES=$GRAMMAR/queries/highlights.scm

if ! command -v tree-sitter >/dev/null 2>&1; then
    echo "SKIPPED: tree-sitter is not installed (B-102: brew install tree-sitter-cli)"
    echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
    exit 77
fi

# (1) Stale check, before anything else reads the file.
# `mktemp`, not a fixed name under the system temp dir: two concurrent gates
# would compare against each other's copy (W-305).
before=$(mktemp "${TMPDIR:-/tmp}/highlights-before.XXXXXX")
trap 'rm -f "$before"' EXIT
cp "$QUERIES" "$before" 2>/dev/null || true
rc=0; sh tools/gen-highlights.sh >/dev/null || rc=$?
if [ "$rc" -eq 77 ]; then echo "SKIPPED: gen-highlights could not run (no Stage 1 compiler) - CANNOT RUN, exiting 77"; exit 77; fi
[ "$rc" -eq 0 ] || exit 1
if ! cmp -s "$before" "$QUERIES"; then
    echo "  FAIL  $QUERIES was stale — regenerating changed it"
    echo "        run tools/gen-highlights.sh and commit the result"
    diff <(head -40 "$before") <(head -40 "$QUERIES") | head -10 | sed 's/^/        /'
    exit 1
fi

(cd "$GRAMMAR" && tree-sitter generate --js-runtime native >/dev/null 2>&1) || {
    echo "  FAIL  tree-sitter generate failed"; exit 1; }


# No interpreter beyond sh/awk/sort/find: the former embedded script is ported
# stage for stage.  Same checks, same messages, same exit codes.
tab=$(printf '\t')
queries=queries/highlights.scm
root=$PWD

# The five ratified sigils must be the ones sadhana implements. The generator
# has to restate them — a .scm file cannot import an enum — so the copy is
# checked against its source here rather than trusted.
# Frozen from the retired Rust Karaka::sigil() (git show 688e3e63a^:crates/sadhana/src/lex.rs, lines 61-69).
sigils=('म्' 'न' 'त्' 'य्' 'ए')
from_rust=$(printf '%s\n' "${sigils[@]}" | LC_ALL=C sort)
from_gen=$(awk -F'\t' 'index($0, "s\t") == 1 { print $2 }' spec/highlight-constants.tsv | LC_ALL=C sort)
pylist() {  # sorted lines on stdin -> ['a', 'b'] as the old report printed them
    awk 'BEGIN { printf "[" } { printf "%s'"'"'%s'"'"'", (NR > 1 ? ", " : ""), $0 } END { print "]" }'
}
if [ -z "$from_rust" ]; then
    echo "  FAIL  frozen sigil list is empty"
    exit 1
fi
if [ "$from_rust" != "$from_gen" ]; then
    gen_repr=$([ -n "$from_gen" ] && printf '%s\n' "$from_gen" | pylist || echo '[]')
    rust_repr=$(printf '%s\n' "$from_rust" | pylist)
    echo "  FAIL  sigils disagree: sadhana has $rust_repr, the generator has $gen_repr"
    exit 1
fi

# The labels sadhana says exist, from the B-034a oracle. This is the witness.
nlabels=$(awk -F'\t' 'NF == 4 && $3 == "label" { n++ } END { print n + 0 }' spec/parse-shape-t0.tsv)
if [ "${nlabels:-0}" -eq 0 ]; then
    echo "  FAIL  parsed 0 labels out of spec/parse-shape-t0.tsv — its format moved"
    exit 1
fi

# THE ORACLE DEFINES THE CORPUS — the same correction `check-grammar.sh`
# needed, for the same reason and on the same line. `**/*.sas` swept in the T1
# test programs under `crates/sadhana/src/t1/tests/`, which are written in T1
# and were being held against T0's ratified mnemonic table: `इति`, `तुलना`,
# `वृत्तिः` and six more read as "not ratified names" because they are not T0
# mnemonics at all. `drafts/` came in too, and those by definition do not
# assemble. Together they produced 8970 "highlighting faults" that were a
# category error, not a highlighting defect — and they were the ONLY failing
# check in `deep-gate.sh`.
#
# `labels` above is already read from `spec/parse-shape-t0.tsv`; the file set is
# taken from the same oracle so the witness and the corpus cannot drift apart.
# Narrowing a check is how a check quietly stops checking, so what is skipped is
# COUNTED AND NAMED rather than dropped in silence.
listing=$(find . -name '*.sas' 2>/dev/null | sed 's|^\./||' | LC_ALL=C awk -v shape=spec/parse-shape-t0.tsv '
    BEGIN {
        while ((getline line < shape) > 0) {
            n = split(line, f, "\t")
            if (n == 4 && f[1] != "file" && substr(line, 1, 1) != "#") orc[f[1]] = 1
        }
    }
    index($0, "target/") == 0 { print (($0 in orc) ? "K" : "S") "\t" $0 }' | LC_ALL=C sort)
sources=$(printf '%s\n' "$listing" | awk -F'\t' '$1 == "K" { print $2 }')
skipped=$(printf '%s\n' "$listing" | awk -F'\t' '$1 == "S" { print $2 }')
nsrc=0
[ -n "$sources" ] && nsrc=$(printf '%s\n' "$sources" | awk 'END { print NR }')
if [ -n "$skipped" ]; then
    nskip=$(printf '%s\n' "$skipped" | awk 'END { print NR }')
    echo "  note  $nskip .sas file(s) carry no oracle row and are not T0 corpus:"
    printf '%s\n' "$skipped" | awk 'NR <= 4 { print "          " $0 }'
    if [ "$nskip" -gt 4 ]; then
        echo "          ... and $((nskip - 4)) more"
    fi
fi

# Last capture wins, which is how tree-sitter-highlight resolves overlap, so
# the classification of a span is the last one recorded for it.
# (2) Every operand must be captured. Operands are found from the parse tree,
# independently of the query, so a query that matches nothing cannot hide.
# (3) Every operand called a label reference must name a real label.
# Address modifiers, from sadhana's own splitter rather than from the examples.
# Frozen from the retired Rust encode split (git show 688e3e63a^:crates/sadhana/src/encode.rs, lines 443-446).
# (4) No mnemonic may fall through to plain @function. Every instruction in the
# corpus uses a ratified name, and R-02-1 says every architectural instruction
# has one, so a residue here means either an unregistered mnemonic slipped into
# the tree or the व्याप्ति rule is wrong again — it was twice.
#
# Stage 1 streams tree-sitter's query and parse output per source, stage 2
# classifies, sort puts the report records in (file, row, col) order, stage 3
# prints.  The rightmost failing stage sets the exit code (pipefail).
sigs=$(IFS='|'; echo "${sigils[*]}")
{
    printf '%s\n' "$sources" | while IFS= read -r src; do
        [ -n "$src" ] || continue
        printf '\n@Q %s\n' "$src"
        (cd "$GRAMMAR" && tree-sitter query "$queries" "$root/$src") 2>/dev/null || true
        printf '\n@P %s\n' "$src"
        (cd "$GRAMMAR" && tree-sitter parse "$root/$src") 2>/dev/null || true
    done
} | LC_ALL=C awk -v shape=spec/parse-shape-t0.tsv -v sigs="$sigs" '
    BEGIN {
        ns = split(sigs, SG, "|")
        MD[1] = ""; MD[2] = "ॱउपरि"; MD[3] = "ॱअधः"
        while ((getline line < shape) > 0) {
            n = split(line, f, "\t")
            if (n == 4 && f[3] == "label") labels[f[4]] = 1
        }
        close(shape)
        QRE = "capture: [0-9]+ - [A-Za-z0-9_.]+, start: \\([0-9]+, [0-9]+\\)"
        ORE = "\\(operand \\[[0-9]+, [0-9]+\\]"
    }
    /^@[QP] / { mode = substr($0, 2, 1); src = substr($0, 4); next }
    mode == "Q" {
        if (!match($0, QRE)) next
        seg = substr($0, RSTART, RLENGTH); rest = substr($0, RSTART + RLENGTH)
        i = index(rest, "text: `")
        if (i == 0) next
        t = substr(rest, i + 7)
        if (t !~ /`/) next
        sub(/`[^`]*$/, "", t)
        c = seg; sub(/^capture: [0-9]+ - /, "", c); sub(/,.*/, "", c)
        s = seg; sub(/.*start: \(/, "", s); sub(/\)/, "", s); split(s, rc, ", ")
        k = src SUBSEP (rc[1] + 0) SUBSEP (rc[2] + 0)
        fcap[k] = c; ftext[k] = t; fsrc[k] = src; frow[k] = rc[1] + 0; fcol[k] = rc[2] + 0
        next
    }
    mode == "P" {
        s = $0
        while (match(s, ORE)) {
            m = substr(s, RSTART, RLENGTH); s = substr(s, RSTART + RLENGTH)
            gsub(/[^0-9,]/, "", m); split(m, rc, ",")
            k = src SUBSEP (rc[1] + 0) SUBSEP (rc[2] + 0)
            if (!(k in ops)) { ops[k] = 1; nops++; osrc[k] = src; orow[k] = rc[1] + 0; ocol[k] = rc[2] + 0 }
        }
    }
    END {
        for (k in fcap) { nf++; cnt[fcap[k]]++; if (fcap[k] == "function") res[ftext[k]] = 1 }
        for (k in ops) if (!(k in fcap)) { nu++; U[k] = 1 }
        for (k in fcap) {
            if (!(k in ops) || fcap[k] != "constant") continue
            t = ftext[k]; L = length(t); ok = 0
            for (i = 1; i <= ns && !ok; i++)
                for (j = 1; j <= 3 && !ok; j++) {
                    suf = MD[j] SG[i]; ls = length(suf)
                    if (L >= ls && substr(t, L - ls + 1) == suf && (substr(t, 1, L - ls) in labels)) ok = 1
                }
            if (!ok) { nb++; B[k] = 1 }
        }
        for (t in res) nr++
        printf "0\t%d\t%d\t%d\t%d\t%d\n", nf, nops, nu, nb, nr
        for (c in cnt) printf "1\t%s\t%d\t0\n", c, cnt[c]
        for (k in U) printf "2\t%s\t%d\t%d\n", osrc[k], orow[k], ocol[k]
        for (k in B) printf "3\t%s\t%d\t%d\t%s\n", fsrc[k], frow[k], fcol[k], ftext[k]
        for (t in res) printf "4\t%s\t0\t0\n", t
    }' \
  | LC_ALL=C sort -t "$tab" -k1,1 -k2,2 -k3,3n -k4,4n \
  | LC_ALL=C awk -F'\t' -v nsrc="$nsrc" '
    $1 == 0 {
        nf = $2; nop = $3; nu = $4; nb = $5; nr = $6
        if (nf == 0) {
            printf "  FAIL  0 captures across %d files — the query matched nothing\n", nsrc
            dead = 1; exit 1
        }
        print "  capture              spans"; next
    }
    $1 == 1 { printf "  %-20s %5d\n", $2, $3; next }
    $1 == 2 {
        if (!h2) { printf "\n  FAIL  %d operand(s) received no capture at all:\n", nu; h2 = 1 }
        if (++c2 <= 5) printf "        %s:%d\n", $2, $3 + 1
        next
    }
    $1 == 3 {
        if (!h3) {
            printf "\n  FAIL  %d operand(s) highlighted as a label reference name no label sadhana defines:\n", nb
            h3 = 1
        }
        if (++c3 <= 8) printf "        %s:%d  `%s`\n", $2, $3 + 1, $5
        next
    }
    $1 == 4 { rl = rl (nrl++ ? ", " : "") "\047" $2 "\047"; next }
    END {
        if (dead) exit 1
        if (nb > 0) print "        either the register/numeral rules missed it, or the program is broken"
        if (nr > 0) {
            printf "\n  FAIL  %d mnemonic(s) are not ratified names: [%s]\n", nr, rl
            print "        add them to spec/mnemonics-riscv64.src.tsv, or fix the व्याप्ति rule"
        }
        fail = nu + nb + nr
        if (fail) { printf "\n%d highlighting fault(s).\n", fail; exit 1 }
        printf "\n%d operands, every one classified; every label reference names a label sadhana defines; every mnemonic is a ratified name.\n", nop
    }'
