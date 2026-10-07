#!/bin/bash
set -u
H=/path/to/sansos/wt-raminject/target/release/t1_image
P=/path/to/w356-probe
R=/path/to/sansos/wt-raminject
: > "$P/axis.tsv"
leg() {
  C=$1; T=/path/to/w356-ax-$C
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
python3 - <<'PY'
order = "51598b3c 046718da 08dfdcdf db1ed3c8 3d905c37 c352a1fd ec19dca8 faa5e9f7 4cb96bdb b763f502 68cdd66b 6365e334 f168e91a 599962c4 a6fdd068".split()
d = {}
for ln in open("/path/to/w356-probe/axis.tsv"):
    p = ln.split()
    if len(p) >= 2 and p[1].isdigit(): d[p[0]] = int(p[1])
print(f"  {'commit':<10} {'steps @240':>14}  {'vs fast':>8}  verdict")
prev = None
for c in order:
    if c not in d: print(f"  {c:<10} {'(missing)':>14}"); continue
    s = d[c]; r = s/350519225
    v = "FAST" if r < 1.5 else ("SLOW" if r > 2.5 else "mid")
    mark = "   <<< THE SLOWDOWN LANDS HERE" if (prev is not None and prev < 1.5 and r > 2.5) else ""
    print(f"  {c:<10} {s:>14,} {r:>7.2f}x  {v}{mark}")
    prev = r
PY
