#!/usr/bin/env bash
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
python3 tools/gen-highlights.py >/dev/null || exit 1
if ! cmp -s "$before" "$QUERIES"; then
    echo "  FAIL  $QUERIES was stale — regenerating changed it"
    echo "        run tools/gen-highlights.py and commit the result"
    diff <(head -40 "$before") <(head -40 "$QUERIES") | head -10 | sed 's/^/        /'
    exit 1
fi

(cd "$GRAMMAR" && tree-sitter generate --js-runtime native >/dev/null 2>&1) || {
    echo "  FAIL  tree-sitter generate failed"; exit 1; }

python3 - "$GRAMMAR" <<'PY'
import re, subprocess, sys, pathlib, collections

grammar_dir = sys.argv[1]
root = pathlib.Path('.')
queries = 'queries/highlights.scm'

# The five ratified sigils must be the ones sadhana implements. The generator
# has to restate them — a .scm file cannot import an enum — so the copy is
# checked against its source here rather than trusted.
lex = (root / 'crates/sadhana/src/lex.rs').read_text(encoding='utf-8')
# Only `sigil()`'s arms. `Karaka` also has `name()`, whose arms are the kāraka
# names — कर्म, करण, … — and a looser pattern picks up both, which is exactly
# what the first version of this check did.
sigil_fn = re.search(r'pub fn sigil\(self\).*?\n    \}', lex, re.S)
from_rust = re.findall(r'Karaka::\w+ => "([^"]+)"', sigil_fn.group(0)) if sigil_fn else []
gen = (root / 'tools/gen-highlights.py').read_text(encoding='utf-8')
from_gen = re.findall(r'SIGILS = \[([^\]]+)\]', gen)
from_gen = re.findall(r'"([^"]+)"', from_gen[0]) if from_gen else []
if not from_rust:
    print("  FAIL  parsed 0 sigils out of lex.rs — Karaka::sigil() moved")
    sys.exit(1)
if sorted(from_rust) != sorted(from_gen):
    print(f"  FAIL  sigils disagree: sadhana has {sorted(from_rust)}, "
          f"the generator has {sorted(from_gen)}")
    sys.exit(1)

# The labels sadhana says exist, from the B-034a oracle. This is the witness.
labels = set()
for line in (root / 'spec/parse-shape-t0.tsv').read_text(encoding='utf-8').splitlines():
    f = line.split('\t')
    if len(f) == 4 and f[2] == 'label':
        labels.add(f[3])
if not labels:
    print("  FAIL  parsed 0 labels out of spec/parse-shape-t0.tsv — its format moved")
    sys.exit(1)

CAP = re.compile(r'capture: \d+ - ([\w.]+), start: \((\d+), (\d+)\).*?text: `(.*)`')

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
oracle_files = set()
for line in (root / 'spec/parse-shape-t0.tsv').read_text(encoding='utf-8').splitlines():
    f = line.split('\t')
    if len(f) == 4 and f[0] != 'file' and not line.startswith('#'):
        oracle_files.add(f[0])
found = sorted(str(p) for p in root.glob('**/*.sas') if 'target/' not in str(p))
sources = [s for s in found if s in oracle_files]
skipped = [s for s in found if s not in oracle_files]
if skipped:
    print(f"  note  {len(skipped)} .sas file(s) carry no oracle row and are not T0 corpus:")
    for s in skipped[:4]:
        print(f"          {s}")
    if len(skipped) > 4:
        print(f"          ... and {len(skipped) - 4} more")
# Last capture wins, which is how tree-sitter-highlight resolves overlap, so
# the classification of a span is the last one recorded for it.
final = {}
for src in sources:
    out = subprocess.run(['tree-sitter', 'query', queries, str(pathlib.Path(src).resolve())],
                         cwd=grammar_dir, capture_output=True, text=True)
    for m in CAP.finditer(out.stdout):
        cap, row, col, text = m.group(1), int(m.group(2)), int(m.group(3)), m.group(4)
        final[(src, row, col)] = (cap, text)

if not final:
    print(f"  FAIL  0 captures across {len(sources)} files — the query matched nothing")
    sys.exit(1)

counts = collections.Counter(c for c, _ in final.values())
print("  capture              spans")
for cap, n in sorted(counts.items()):
    print(f"  {cap:<20} {n:>5}")

# (2) Every operand must be captured. Operands are found from the parse tree,
# independently of the query, so a query that matches nothing cannot hide.
operands = set()
NODE = re.compile(r'\(operand \[(\d+), (\d+)\]')
for src in sources:
    out = subprocess.run(['tree-sitter', 'parse', str(pathlib.Path(src).resolve())],
                         cwd=grammar_dir, capture_output=True, text=True)
    for m in NODE.finditer(out.stdout):
        operands.add((src, int(m.group(1)), int(m.group(2))))

uncaptured = sorted(o for o in operands if o not in final)
fail = 0
if uncaptured:
    print(f"\n  FAIL  {len(uncaptured)} operand(s) received no capture at all:")
    for o in uncaptured[:5]:
        print(f"        {o[0]}:{o[1]+1}")
    fail += len(uncaptured)

# (3) Every operand called a label reference must name a real label.
SIGILS = from_rust
# Address modifiers, from sadhana's own splitter rather than from the examples.
MODIFIERS = [''] + re.findall(r'strip_suffix\("(ॱ[^"]+)"\)',
                              (root / 'crates/sadhana/src/encode.rs').read_text(encoding='utf-8'))
bad = []
for (src, row, col), (cap, text) in sorted(final.items()):
    if (src, row, col) not in operands or cap != 'constant':
        continue
    # Strip the sigil by construction rather than by segmentation: the base is
    # whichever known label, plus an optional address modifier, plus a sigil,
    # that reproduces the text exactly. `सारणीॱउपरि` is the high 20 bits of
    # `सारणी` (B-064), and it is a reference to that label, not to another one.
    if not any(text == lbl + m + s
               for lbl in labels for m in MODIFIERS for s in SIGILS):
        bad.append((src, row, text))
if bad:
    print(f"\n  FAIL  {len(bad)} operand(s) highlighted as a label reference name no label "
          f"sadhana defines:")
    for src, row, text in bad[:8]:
        print(f"        {src}:{row+1}  `{text}`")
    print("        either the register/numeral rules missed it, or the program is broken")
    fail += len(bad)

# (4) No mnemonic may fall through to plain @function. Every instruction in the
# corpus uses a ratified name, and R-02-1 says every architectural instruction
# has one, so a residue here means either an unregistered mnemonic slipped into
# the tree or the व्याप्ति rule is wrong again — it was twice.
residue = sorted({t for c, t in final.values() if c == 'function'})
if residue:
    print(f"\n  FAIL  {len(residue)} mnemonic(s) are not ratified names: {residue}")
    print("        add them to spec/mnemonics-riscv64.src.tsv, or fix the व्याप्ति rule")
    fail += len(residue)

if fail:
    print(f"\n{fail} highlighting fault(s).")
    sys.exit(1)

print(f"\n{len(operands)} operands, every one classified; "
      f"every label reference names a label sadhana defines; "
      f"every mnemonic is a ratified name.")
PY
