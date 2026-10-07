#!/usr/bin/env bash
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

python3 - "$GRAMMAR" "$ORACLE" <<'PY'
import re, subprocess, sys, pathlib, collections

grammar_dir, oracle_path = sys.argv[1], sys.argv[2]
root = pathlib.Path('.')

# The kinds the oracle records. A node kind this grammar produces that is not
# here is invisible to the comparison, so the list is asserted below.
KINDS = {'instruction', 'label', 'directive'}

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
oracle_files = set()
for line in pathlib.Path(oracle_path).read_text(encoding='utf-8').splitlines():
    f = line.split('\t')
    if len(f) == 4 and f[0] != 'file' and not line.startswith('#'):
        oracle_files.add(f[0])

found = sorted(str(p) for p in root.glob('**/*.sas') if 'target/' not in str(p))
sources = [s for s in found if s in oracle_files]
skipped = [s for s in found if s not in oracle_files]
if not sources:
    print("  FAIL  no .sas files found — this check would pass having compared nothing")
    sys.exit(1)
if skipped:
    print(f"  note  {len(skipped)} .sas file(s) carry no oracle row and are not T0 corpus:")
    for s in skipped[:4]:
        print(f"          {s}")
    if len(skipped) > 4:
        print(f"          ... and {len(skipped) - 4} more")

NODE = re.compile(r'\((\w+) \[(\d+), (\d+)\] - \[(\d+), (\d+)\]')

mine = set()
rules_seen = collections.Counter()
errors = []

for src in sources:
    # Absolute: tree-sitter runs with cwd=grammar_dir so it finds the parser,
    # and a path relative to the repo root would silently not exist there. The
    # first version did exactly that, found 0 nodes in all 39 files, and this
    # check reported 179 disagreements rather than a pass — which is the whole
    # point of comparing against an oracle instead of counting errors.
    out = subprocess.run(['tree-sitter', 'parse', str(pathlib.Path(src).resolve())],
                         cwd=grammar_dir, capture_output=True, text=True)
    # `tree-sitter parse` exits non-zero when the tree contains an ERROR, and
    # prints the tree either way. Both are wanted: the tree to compare, the
    # status to report.
    tree = out.stdout
    if 'ERROR' in tree or 'MISSING' in tree:
        errors.append(f"{src}: parse tree contains ERROR/MISSING")

    lines = (root / src).read_bytes().split(b'\n')
    nodes = [(m.group(1), int(m.group(2)), int(m.group(3)), int(m.group(5)), m.end())
             for m in NODE.finditer(tree)]
    for i, (kind, row, col, endcol, pos) in enumerate(nodes):
        rules_seen[kind] += 1
        if kind not in KINDS:
            continue
        # The name is the statement's `name:` field — the next node in the
        # sexp. Read its bytes out of the source rather than re-deriving it,
        # so this compares what the parser saw and not what this script thinks
        # the syntax is.
        name = None
        for (k2, r2, c2, e2, _) in nodes[i + 1:i + 3]:
            if k2 in ('mnemonic', 'label_name', 'word'):
                name = lines[r2][c2:e2].decode('utf-8')
                break
        if name is None:
            errors.append(f"{src}:{row+1}: {kind} with no name node")
            continue
        # A label's name excludes the `ॱॱ` that marks it, as the oracle records
        # the label, not the mark.
        if kind == 'label':
            name = name.removesuffix('ॱॱ')
        mine.add((src, row + 1, kind, name))

oracle = set()
for line in pathlib.Path(oracle_path).read_text(encoding='utf-8').splitlines():
    if line.startswith('#') or not line.strip():
        continue
    f = line.split('\t')
    if len(f) != 4 or f[0] == 'file':
        continue
    oracle.add((f[0], int(f[1]), f[2], f[3]))

if not oracle:
    print(f"  FAIL  parsed 0 rows out of {oracle_path} — its format moved")
    sys.exit(1)

missing = oracle - mine     # sadhana saw it, the grammar did not
extra = mine - oracle       # the grammar invented it

# Counts by kind, which is what catches statements swallowed wholesale.
print("  kind          oracle   grammar")
for k in sorted(KINDS):
    o = sum(1 for r in oracle if r[2] == k)
    g = sum(1 for r in mine if r[2] == k)
    flag = '' if o == g else '   <-- DISAGREES'
    print(f"  {k:<12} {o:>7} {g:>9}{flag}")
print(f"  {'total':<12} {len(oracle):>7} {len(mine):>9}")

# Every declared rule must fire. A rule that never matches is a rule that is
# not doing what it says, and the parse stays clean while it happens.
declared = set(re.findall(r'^    (\w+): \$ =>',
                          pathlib.Path(grammar_dir + '/grammar.js').read_text(), re.M))
# A leading underscore means HIDDEN: tree-sitter inlines those rules and they
# can never appear as a node, so demanding they fire would fail forever and the
# real assertion below would be discarded to silence it.
declared = {d for d in declared if not d.startswith('_')} - {'source_file'}
never = sorted(d for d in declared if rules_seen[d] == 0)
if never:
    print(f"\n  FAIL  declared but never matched anywhere in {len(sources)} files: {never}")
    print("        a rule that cannot fire is not a rule; the corpus still parses clean")

for e in errors[:10]:
    print(f"  FAIL  {e}")
for r in sorted(missing)[:10]:
    print(f"  FAIL  sadhana parsed {r[2]} `{r[3]}` at {r[0]}:{r[1]}; the grammar did not")
for r in sorted(extra)[:10]:
    print(f"  FAIL  the grammar invented {r[2]} `{r[3]}` at {r[0]}:{r[1]}")

if missing or extra or errors or never:
    n = len(missing) + len(extra) + len(errors) + len(never)
    print(f"\n{n} disagreement(s) with sadhana. The grammar is not the language.")
    sys.exit(1)

print(f"\ngrammar agrees with sadhana on all {len(oracle)} statements in {len(sources)} files.")
PY
