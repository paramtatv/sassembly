#!/bin/sh
# One-word rule (TAGSCAN-SPEC section 7, a6 ruling 2026-10-16): in every built image each whole tag word
# (SASARGV, SASARGC, SASINPUT, SASINAME, SASTRACE, SASEVENT) occurs AT MOST ONCE over ALL file bytes (once = its slot; 0 = no such global).
# A loader image cannot tell its slot from a compare literal (RCA-B); GAP-TAGABI is the future ABI fix.
#   tools/check-tag-words.sh IMAGE...   count per image; a count above 1 is FAIL naming image, tag and count (rc 1)
#   tools/check-tag-words.sh --interp IMAGE...   an interpreter image (yi.elf) must hold NO SASARGV/SASARGC word at all: it
#                                       carries guest tags only as runtime-built halves (fixB); any copy is FAIL (rc 1)
#   tools/check-tag-words.sh            self-test (a clean image passes, a doubled tag fails) + every committed *.elf
# tools/t1-build.sh runs it on each image it writes, so every image the gate builds is covered.
# A count of 0 is fine (an image without the argument globals). POSIX sh + od + awk; nothing else is needed.
export LC_ALL=C
R=$(cd "$(dirname "$0")/.." && pwd)
# counts IMAGE -> "argv argc input iname trace event": non-overlapping whole-word hits, found byte-aligned in an od hex stream
counts() {
    od -An -v -tx1 "$1" | tr -d '\n' | tr -s ' ' | awk '{
        p[1] = " 53 41 53 41 52 47 56 00"; p[2] = " 53 41 53 41 52 47 43 00"; p[3] = " 53 41 53 49 4e 50 55 54"
        p[4] = " 53 41 53 49 4e 41 4d 45"; p[5] = " 53 41 53 54 52 41 43 45"; p[6] = " 53 41 53 45 56 45 4e 54"
        for (i = 1; i <= 6; i++) { s = $0; c[i] = gsub(p[i], "", s) }
        print c[1], c[2], c[3], c[4], c[5], c[6] }'
}
scan() {
    bad=0; interp=0
    if [ "${1:-}" = "--interp" ]; then interp=1; shift; fi
    for p in "$@"; do
        if [ ! -r "$p" ]; then echo "FAIL tag-words $p: unreadable"; bad=$((bad + 1)); continue; fi
        set -- $(counts "$p"); a=$1; b=$2; i=$3; n=$4; t=$5; e=$6
        over=0
        if [ $interp = 1 ]; then
            [ "$a" -eq 0 ] || { echo "FAIL tag-words $p: SASARGV occurs $a times (must be absent in an interpreter)"; over=$((over + 1)); }
            [ "$b" -eq 0 ] || { echo "FAIL tag-words $p: SASARGC occurs $b times (must be absent in an interpreter)"; over=$((over + 1)); }
        else
            for kv in SASARGV=$a SASARGC=$b SASINPUT=$i SASINAME=$n SASTRACE=$t SASEVENT=$e; do
                [ "${kv#*=}" -le 1 ] || { echo "FAIL tag-words $p: ${kv%=*} occurs ${kv#*=} times (must be at most once)"; over=$((over + 1)); }
            done
        fi
        bad=$((bad + over))
        [ $over -ne 0 ] || echo "PASS tag-words $p: SASARGV=$a SASARGC=$b SASINPUT=$i SASINAME=$n SASTRACE=$t SASEVENT=$e"
    done
    [ $bad -eq 0 ]
}
if [ "$#" -gt 0 ]; then scan "$@"; exit $?; fi
T=$(mktemp -d "${TMPDIR:-/tmp}/tagwords.XXXXXX") || exit 2
trap 'rm -rf "$T"' EXIT
printf 'xx SASARGV\000\000\000\000\000\000\000\000 yy SASARGC\000\000\000\000\000\000\000\000 SASEVENT\000\000\000\000\000\000\000\000' > "$T/good"
cp "$T/good" "$T/bad"; printf 'zz SASARGV\000' >> "$T/bad"
scan "$T/good" > /dev/null || { echo "FAIL tag-words self-test: a clean image was refused"; exit 1; }
scan "$T/bad" > /dev/null && { echo "FAIL tag-words self-test: a doubled SASARGV was accepted"; exit 1; }
scan --interp "$T/good" > /dev/null && { echo "FAIL tag-words self-test: --interp accepted a held SASARGV"; exit 1; }
echo "PASS tag-words self-test: clean image accepted, doubled tag refused"
n=0; set --
for f in $(cd "$R" && git ls-files '*.elf' 2> /dev/null); do [ -f "$R/$f" ] && set -- "$@" "$R/$f" && n=$((n + 1)); done
[ "$n" -eq 0 ] && { echo "PASS tag-words: no committed .elf image"; exit 0; }
scan "$@" > "$T/out"; rc=$?
grep -a -c '^PASS' "$T/out" | sed 's/^/committed images clean: /'; grep -a '^FAIL' "$T/out"
exit $rc
