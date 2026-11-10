#!/bin/bash
set -u
H=$WT/target/release/t1_image
P=$WORK/w356-probe
R=$WT
: > "$P/axis.tsv"
leg() {
  C=$1; T=$WORK/w356-ax-$C
  [ -d "$T" ] || git -C "$R" worktree add --detach "$T" "$C" >/dev/null 2>&1
  G=$(git -C "$T" rev-parse --short HEAD 2>/dev/null)
  [ "$G" = "$C" ] || { echo "$C GUARD tree=$G" >> "$P/axis.tsv"; return; }
  O=$("$H" --spec-root "$T/spec" --compiler "$T/crates/sadhana-t1/src" \
        -o "$P/ax-$C.elf" "$P/fit240.t1" 2>&1)
  S=$(printf '%s' "$O" | grep -oE 'steps: *[0-9]+' | grep -oE '[0-9]+')
  L=$(printf '%s' "$O" | grep -cE 'compiler: 21 sources loaded')
  echo "$C ${S:-NOCOUNT} loaded=$L" >> "$P/axis.tsv"
}
echo "=== W-356 CORRECT AXIS: 51598b3c (FAST 350M) -> origin/main (SLOW 1.286G) ==="
echo "WHY THE FIRST BISECT WAS WRONG: its 7 legs were on a branch that forked at"
echo "  ab3d0d74 and never received the speedup, so all 7 read ~1.28G for the same"
echo "  structural reason — 046718da is IDENTICAL to its parent ab3d0d74 to the step."
echo "  The axis that carries the change is 51598b3c..origin/main."
echo "ONE probe per commit at 240 literals: 350M vs 1.28G separates cleanly, so the"
echo "  600/1200 probes are not needed to locate the commit."
date "+%H:%M:%S START"
for C in 51598b3c 046718da 08dfdcdf db1ed3c8 3d905c37 c352a1fd ec19dca8 faa5e9f7 4cb96bdb b763f502 68cdd66b 6365e334 f168e91a 599962c4 a6fdd068; do
  leg "$C" &
done
wait
date "+%H:%M:%S DONE"
echo
awk -v order="51598b3c 046718da 08dfdcdf db1ed3c8 3d905c37 c352a1fd ec19dca8 faa5e9f7 4cb96bdb b763f502 68cdd66b 6365e334 f168e91a 599962c4 a6fdd068" '
function grp(s,   o, n) {
  sub(/^0+/, "", s); if (s == "") s = "0"
  o = ""; n = length(s)
  while (n > 3) { o = "," substr(s, n - 2, 3) o; n -= 3 }
  return substr(s, 1, n) o
}
NF >= 2 && $2 ~ /^[0-9]+$/ { d[$1] = $2 }
END {
  printf "  %-10s %14s  %8s  verdict\n", "commit", "steps @240", "vs fast"
  n = split(order, ord, " "); havep = 0
  for (i = 1; i <= n; i++) {
    c = ord[i]
    if (!(c in d)) { printf "  %-10s %14s\n", c, "(missing)"; continue }
    r = d[c] / 350519225
    v = (r < 1.5) ? "FAST" : ((r > 2.5) ? "SLOW" : "mid")
    mark = (havep && prev < 1.5 && r > 2.5) ? "   <<< THE SLOWDOWN LANDS HERE" : ""
    printf "  %-10s %14s %7.2fx  %s%s\n", c, grp(d[c]), r, v, mark
    prev = r; havep = 1
  }
}' "$P/axis.tsv"
