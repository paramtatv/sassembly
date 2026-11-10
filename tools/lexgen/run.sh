#!/bin/sh
# Native generator for spec/lexicon.tsv (no Rust). POSIX sh + awk. Run from repo root.
# Usage: sh tools/lexgen/run.sh [--stdout]
export LC_ALL=C
D=$(dirname "$0")
M=spec/mnemonics-riscv64.src.tsv
S=spec/lexicon.src.tsv
for f in "$M" "$S" "$D/lexgen.awk" "$D/lexicon-header.txt"; do
  [ -f "$f" ] || { echo "lexgen: missing input $f" >&2; exit 2; }
done
files=
for g in crates/sadhana-t1/src/*.t1 spec/*.sas spec/golden/*.sas; do
  [ -f "$g" ] && files="$files $g"
done
[ -n "$files" ] || { echo "lexgen: no corpus files" >&2; exit 2; }
T=$(mktemp "${TMPDIR:-/tmp}/lexgen.XXXXXX") || exit 2
trap 'rm -f "$T"' EXIT
# shellcheck disable=SC2086
awk -v HEADER="$D/lexicon-header.txt" -v MNEM="$M" -v SRC="$S" -f "$D/lexgen.awk" "$M" $files "$S" >"$T" 2>"$T.err" || { cat "$T.err" >&2; rm -f "$T.err"; echo "lexgen: FAILED" >&2; exit 1; }
rm -f "$T.err"
[ -s "$T" ] || { echo "lexgen: empty output" >&2; exit 1; }
if [ "$1" = "--stdout" ]; then cat "$T"; exit 0; fi
if cmp -s "$T" spec/lexicon.tsv; then echo "spec/lexicon.tsv unchanged"
else cat "$T" >spec/lexicon.tsv && echo "wrote spec/lexicon.tsv"; fi
