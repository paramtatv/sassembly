#!/bin/sh
# tools/selfcheck/build-refuse.sh [name...] — the BUILD-MUST-REFUSE half of the selfcheck (fork-join's 0x377, 0x37f, 0x381, 0x382).
#
# Every program in refuse/*.t1 must FAIL TO BUILD with the cause its row in refuse/expected.tsv names. The everyday
# build.sh cannot hold such a program (it fails closed on any program that does not build), so they live apart.
# The cause is read natively: t1-build.sh runs Stage 1 with YANTRA_INPUT_TRACE=3, and on a refused build Stage 1
# writes marker 233 then two little-endian 32-bit words (shrinkhala.t1, `स्वपरीक्षास्वप्रतिबिम्बम्`; the status, 1601,
# says the link refused, which a module that did not compile reaches): the first failed source's exit (11 =
# सङ्कलनवर्गविरोधभेद, मध्यरूप refused it) and मध्यरूप's shape (7 = क्षेत्रपलायनवर्गविरोधभेद, फलपलायननिषेधः 0x377;
# 8 = क्षेत्रबाह्याह्वानवर्गविरोधभेद, क्षेत्रबाह्याह्वाननिषेधः 0x37f, a body routine called outside its region;
# 9 = क्षेत्राशुद्धिवर्गविरोधभेद, क्षेत्राशुद्धाह्वाननिषेधः 0x381, a routine reachable from a region body is not region-pure;
# 10 = संश्लिष्टनामवर्गविरोधभेद, संश्लिष्टनामनिषेधः 0x382, a routine declared under a name the build synthesises).
# t1-build.sh relays the sink on stderr when it refuses. A row's code is checked against that pair.
#
# FAILS CLOSED: exit 0 only if at least one program was checked and every one refused with its cause. A program
# that BUILDS, refuses for another cause, has no cause dump, times out, has no row, or a row with no program fails.
#
# Env: SC_DIR   the programs' directory (default: this script's); refuse/ is under it
#      SAS      the compiler checkout (default: this checkout) — passed to t1-build.sh
#      SC_T1BUILD  the build tool (default tools/t1-build.sh beside this script)   SC_BUILD_TIMEOUT (900 s)
# NOT YET RUN BY THE LANDING: land-remote.sh is BASE's and runs only build.sh/run.sh (open item).
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
dir=${SC_DIR:-$here}/refuse
t1b=${SC_T1BUILD:-$root/tools/t1-build.sh}
tmo=${SC_BUILD_TIMEOUT:-900}
wt="perl $here/../with-timeout.pl"
exp=$dir/expected.tsv
[ -f "$exp" ] || { echo "build-refuse: no $exp" >&2; exit 2; }
[ -x "$t1b" ] || [ -f "$t1b" ] || { echo "build-refuse: no t1-build at $t1b" >&2; exit 2; }
tmp=$(mktemp -d "${TMPDIR:-/tmp}/build-refuse.XXXXXX") || exit 2
trap 'rm -rf "$tmp"' EXIT INT TERM
bad=0
fail() { echo "BUILD-REFUSE MISMATCH $1: $2" >&2; bad=$((bad + 1)); }

rows=$(awk -F'\t' '!/^#/ && NF >= 2 {print $1}' "$exp" | sort)
[ "$(awk -F'\t' '!/^#/ && NF >= 2 {print $1}' "$exp" | sort | uniq -d)" = "" ] || fail expected.tsv "a DUPLICATE row"
awk -F'\t' '!/^#/ && (NF < 2 || $1 == "" || $2 !~ /^0x3[78][0-9a-f]$/) { exit 1 }' "$exp" || fail expected.tsv "a blank or malformed row"
progs=$(cd "$dir" && ls *.t1 2>/dev/null | sed 's/\.t1$//' | sort)
for n in $progs; do echo "$rows" | grep -qx "$n" || fail "$n" "has no row in refuse/expected.tsv"; done
for n in $rows; do [ -f "$dir/$n.t1" ] || fail "$n" "has a row but no refuse/$n.t1"; done
names=${*:-$progs}
count=0
for n in $names; do
  [ -f "$dir/$n.t1" ] || { fail "$n" "is not a program (unknown name)"; continue; }
  want=$(awk -F'\t' -v n="$n" '$1 == n {print $2; exit}' "$exp")
  mod=$(sed -n '1s/^मण्डलम् \([^ ]*\) .*/\1/p' "$dir/$n.t1")
  [ -n "$mod" ] || { fail "$n" "has no module line"; continue; }
  count=$((count + 1))
  YANTRA_INPUT_TRACE=3 SAS=${SAS:-$root} $wt "$tmo" bash "$t1b" --entry "$mod" मुख्यम् -o "$tmp/$n.elf" "$dir/$n.t1" > "$tmp/$n.out" 2> "$tmp/$n.err"
  rc=$?
  [ "$rc" = 124 ] && { fail "$n" "TIMEOUT after ${tmo}s"; continue; }
  if [ "$rc" = 0 ] || [ -s "$tmp/$n.elf" ]; then fail "$n" "BUILT (exit $rc): it must be refused with $want"; continue; fi
  got=$(python3 - "$tmp/$n.err" <<'PY'
import struct, sys
b = open(sys.argv[1], "rb").read()
i = b.find(b"\xe7\xe9")          # 231 closes the (empty) image, 233 opens the cause
if i < 0 or len(b) < i + 10:
    print("NONE"); sys.exit(0)
exit_, shape = struct.unpack("<II", b[i + 2:i + 10])
code = {(11, 7): "0x377", (11, 8): "0x37f", (11, 9): "0x381", (11, 10): "0x382"}.get((exit_, shape), "?")
print("%s exit=%d shape=%d" % (code, exit_, shape))
PY
)
  case $got in
    NONE) fail "$n" "refused (exit $rc) but wrote no cause dump (marker 233): is Stage 1 older than trace level 3?" ;;
    "$want "*) echo "build-refuse: $n refused: $got" ;;
    *) fail "$n" "refused for another cause: $got (want $want)" ;;
  esac
done
[ "$count" -gt 0 ] || { echo "build-refuse: FAILED, zero programs" >&2; exit 1; }
if [ "$bad" -gt 0 ]; then echo "build-refuse: FAILED, $bad mismatch(es)" >&2; exit 1; fi
echo "build-refuse: $count programs refused as pinned"
echo "build-refuse: ok"
