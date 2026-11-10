#!/bin/sh
# Generates assets/marks/marks.svg, assets/marks/sheet-marks.html and spec/marks.tsv with the T1 port (tools/gen/marks.t1).
# usage: gen-marks.sh [outdir]   (outdir gets marks.svg sheet-marks.html marks.tsv; default: the committed paths)   exit 1 = refused
set -eu
here=$(cd "$(dirname "$0")" && pwd); root=$(cd "$here/.." && pwd)
d=${GEN_MARKS_DIR:-$here/gen/marks}
w=$(mktemp -d "${TMPDIR:-/tmp}/gen-mk.XXXXXX")
elf=$w/mk.elf
(cd "$root" && tools/t1-build.sh --entry चिह्नसाधन मुख्यम् -o "$elf" crates/sadhana-t1/src/ashtaka.t1 tools/gen/sanchika.t1 tools/gen/marks.t1) >"$w/blog" 2>&1 || true
[ -f "$elf" ] || { echo "gen-marks: build refused" >&2; rm -rf "$w"; exit 1; }
mkdir "$w/f"
for t in "$d"/[0-9]*.tmpl "$d/marks.tsv" "$d/housings.tsv"; do cat "$t"; printf '\001'; done >"$w/f/निवेश"
rc=0; (cd "$root" && tools/t1-run.sh --files "$w/f" "$elf") >"$w/log" 2>&1 || rc=$?
if [ "$rc" -ne 0 ] || [ ! -f "$w/f/चित्र" ] || [ ! -f "$w/f/पत्र" ] || [ ! -f "$w/f/सारणी" ]; then echo "gen-marks: refused (exit $rc)" >&2; rm -rf "$w"; exit 1; fi
if [ $# -ge 1 ]; then mkdir -p "$1"; o1=$1/marks.svg; o2=$1/sheet-marks.html; o3=$1/marks.tsv
else o1=$root/assets/marks/marks.svg; o2=$root/assets/marks/sheet-marks.html; o3=$root/spec/marks.tsv; fi
cp "$w/f/चित्र" "$o1"; cp "$w/f/पत्र" "$o2"; cp "$w/f/सारणी" "$o3"; rm -rf "$w"
