#!/bin/sh
# The editor's grammar agrees with `sadhana` about what a verb is — task `F-020`.
#
# Wrapper around `check-sanskriti-grammar.mjs`; the work is there because the
# grammar is JSON and the oracle is a TSV, and node reads both without a parser
# written for this.
#
# ## Why a highlighter needs a check at all
#
# It fails silently. A rule that never matches leaves text the colour of the text
# around it, and nobody files a bug about a word that is not blue.
# `tools/check-highlights.sh` says the same about the tree-sitter queries; this
# is the VS Code grammar, which is a SECOND highlighter over the same language.
#
# The specific hazard is `ADR-0004`. The destination kāraka sigil is `म्` and most
# mnemonics are neuter nominative singulars ending in `-म्`, so the obvious rule
# — "a token ending in म् is a destination operand" — colours the VERB as an
# operand. Measured on this corpus: it disagrees with `sadhana` on 3683 of 5734
# instructions. The grammar therefore anchors to start-of-statement, because
# ADR-0004's ruling is that POSITION marks the verb.
#
# Usage: tools/check-sanskriti-grammar.sh
set -u

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1

# `CANNOT RUN:` must be the FIRST line and spelled EXACTLY that — gate.sh's run()
# matches `^(SKIPPED|CANNOT RUN):` and surfaces only the first line.
command -v node >/dev/null || {
    echo "CANNOT RUN: node absent — the grammar and manifests cannot be parsed"
    echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
    exit 77
}

[ -f spec/parse-shape-t0.tsv ] || {
    echo "  FAIL  spec/parse-shape-t0.tsv is missing — the oracle this check reads."
    echo "        Regenerate with: UPDATE_GOLDEN=1 cargo test -p sadhana --test parse_shape"
    exit 1
}

node tools/check-sanskriti-grammar.mjs || exit 1
