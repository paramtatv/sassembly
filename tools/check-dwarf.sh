#!/usr/bin/env bash
# Does a debugger agree with our line table? — task `B-011a`, doc 03 §3.
#
# The unit tests check the bytes we meant to write. This checks that an
# INDEPENDENT implementation reads them back as the lines we meant — readelf
# has no idea what our assembler intended, so agreement is evidence rather than
# tautology. It is the same argument the conformance corpus rests on.
#
# The expectation is not written down here either: it is computed from the
# source, by finding which lines of the `.sas` actually carry an instruction.
set -euo pipefail

cd "$(dirname "$0")/.."

READELF=${READELF:-riscv64-elf-readelf}
# An absent reader is CANNOT RUN (77), not a DWARF defect. See check-link.sh.
command -v "$READELF" >/dev/null || {
  echo "CANNOT RUN: $READELF is not installed — the debug info was not read"
  exit 77
}

SRC=spec/namaste.sas
OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT

cargo run --quiet -p sadhana -- -g "$SRC" "$OUT/n.elf" >/dev/null

# Which lines of the source carry an instruction: a statement ends in the daṇḍa
# `।`, which is what the grammar says and what the parser counts.
#
# The comment mark `॰` has to come off first. Lines 3 and 4 of namaste.sas are
# Devanagari PROSE, and Devanagari prose ends its sentences with a daṇḍa — so
# the naive search claimed two comments were instructions and blamed the line
# table for the difference.
expected=$(sed 's/॰.*//' "$SRC" | grep -n '।' | cut -d: -f1 | tr '\n' ' ')

# Which lines the debug info claims, in address order.
actual=$("$READELF" --debug-dump=decodedline "$OUT/n.elf" \
  | awk '$2 ~ /^[0-9]+$/ && $3 ~ /^0x/ { print $2 }' \
  | tr '\n' ' ')

if [ "$expected" != "$actual" ]; then
  echo "the line table disagrees with the source"
  echo "  instructions are on lines: $expected"
  echo "  the table says:            $actual"
  exit 1
fi

# And every row must name the file rather than <corrupt>: DWARF 5 starts the
# `file` register at 1 while the table is indexed from 0, so a table with one
# entry decodes the right LINES while naming no file at all.
if "$READELF" --debug-dump=decodedline "$OUT/n.elf" 2>&1 | grep -q 'corrupt\|Warning'; then
  echo "readelf reported the line table as malformed:"
  "$READELF" --debug-dump=decodedline "$OUT/n.elf" 2>&1 | grep 'corrupt\|Warning' | head -3
  exit 1
fi

count=$(echo "$actual" | wc -w | tr -d ' ')

# The compilation unit — `B-011b`. Without low_pc/high_pc a debugger has no
# reason to believe an address belongs to this unit and never reads the line
# table it already has.
info=$("$READELF" --debug-dump=info "$OUT/n.elf" 2>&1)
for want in DW_TAG_compile_unit DW_AT_low_pc DW_AT_high_pc DW_AT_stmt_list DW_AT_name; do
  echo "$info" | grep -q "$want" || { echo "the CU has no $want:"; echo "$info"; exit 1; }
done
echo "$info" | grep -q 'Version:  *5' || { echo "the CU is not DWARF 5"; exit 1; }

# A LINKED image is several sources laid end to end, and a table without a file
# index per row blames every line on the first — a debugger stepping into a
# library then shows the caller's source with the callee's lines. Plausible,
# and wrong in the way that costs an hour.
cargo run --quiet -p sadhana -- -g spec/namaste-main.sas spec/lib-mudraka.sas \
  "$OUT/two.elf" >/dev/null
named=$("$READELF" --debug-dump=decodedline "$OUT/two.elf" 2>&1 \
  | awk '$2 ~ /^[0-9]+$/ && $3 ~ /^0x/ { print $1 }' | sort -u | tr '\n' ' ')
expected_files="spec/lib-mudraka.sas spec/namaste-main.sas "
if [ "$named" != "$expected_files" ]; then
  echo "a two-file image does not name both sources"
  echo "  expected: $expected_files"
  echo "  got:      $named"
  exit 1
fi

echo "dwarf: readelf decodes $count line rows matching $SRC, a DWARF 5 CU, and both files of a linked image"
