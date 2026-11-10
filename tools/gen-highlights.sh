#!/bin/sh
# Generates grammar/tree-sitter-sassembly/queries/highlights.scm with the T1 port (tools/gen/highlights.t1).
# usage: gen-highlights.sh [out.scm [spec-dir]]   exit 1 = refused. Inputs: the mnemonics, registers, directives,
# fence-domains and highlight-constants TSVs of spec-dir (default spec/) plus tools/gen/highlights.template.
set -eu
here=$(cd "$(dirname "$0")" && pwd); root=$(cd "$here/.." && pwd)
ab() { printf %s/%s "$(cd "$(dirname "$1")" && pwd)" "$(basename "$1")"; }
out=${1:-$root/grammar/tree-sitter-sassembly/queries/highlights.scm}; out=$(ab "$out")
spec=${2:-$root/spec}
for f in mnemonics-riscv64.src.tsv registers-riscv64.tsv directives.tsv fence-domains-riscv64.tsv highlight-constants.tsv; do
    [ -f "$spec/$f" ] || { echo "gen-highlights: $spec/$f is missing" >&2; exit 1; }
done
w=$(mktemp -d "${TMPDIR:-/tmp}/gen-hl.XXXXXX")
elf=$w/hl.elf
(cd "$root" && tools/t1-build.sh --entry उद्दीपन मुख्यम् -o "$elf" crates/sadhana-t1/src/ashtaka.t1 tools/gen/sanchika.t1 tools/gen/highlights.t1) >"$w/blog" 2>&1 || true
[ -f "$elf" ] || { echo "gen-highlights: build refused" >&2; cat "$w/blog" >&2; rm -rf "$w"; exit 1; }
mkdir "$w/f"
cp "$spec/mnemonics-riscv64.src.tsv" "$w/f/आज्ञा"; cp "$spec/registers-riscv64.tsv" "$w/f/पञ्जी"
cp "$spec/directives.tsv" "$w/f/निर्देश"; cp "$spec/fence-domains-riscv64.tsv" "$w/f/आवरण"
cp "$spec/highlight-constants.tsv" "$w/f/स्थिरांक"; cp "$here/gen/highlights.template" "$w/f/साचा"
rc=0; (cd "$root" && tools/t1-run.sh --files "$w/f" "$elf") >"$w/log" 2>&1 || rc=$?
if [ "$rc" -ne 0 ] || [ ! -f "$w/f/निर्गम" ]; then echo "gen-highlights: refused (exit $rc)" >&2; cat "$w/log" >&2; rm -rf "$w"; exit 1; fi
cp "$w/f/निर्गम" "$out"; rm -rf "$w"
