#!/bin/bash
set -u
H=/path/to/sansos/wt-raminject/target/release/t1_image
P=/path/to/w356-probe
DEV() { python3 -c "
import sys
D='०१२३४५६७८९'
print(''.join(D[int(c)] for c in sys.argv[1]))" "$1"; }
gen() { python3 - "$1" <<'PY'
import sys
DEV="०१२३४५६७८९"
def dev(n): return "".join(DEV[int(c)] for c in str(n)) if n else "०"
N=int(sys.argv[1])
L=[f"मण्डलम् बहुमूल्य{dev(N)} ॥","","सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि",
   "    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।"]
for i in range(1,N+1): L.append(f"    सूची अङ्कः {dev(i)} अन्तः भवति {dev(i*7%997)} ।")
L+=["","    प्रत्यागमनम् ० ।","इति",""]
open(f"/path/to/w356-probe/fit{N}.t1","w").write("\n".join(L))
PY
}
echo "=== W-356 PRE-REGRESSION EXPONENT, fitted directly on 51598b3c ==="
echo "WHY: the exponent delta of 0.32 came from a two-point fit on the RATIO, and"
echo "  the arithmetic 'check' I ran on it was circular. This measures the base"
echo "  tree's own exponent so it can be compared with W-346's post-regression"
echo "  n^1.63 without deriving one from the other."
echo "PREDICTION: if the delta is real the base tree fits near n^1.31."
echo
for T in /path/to/w356-base /path/to/w347-before; do
  C=$(git -C "$T" rev-parse --short HEAD)
  echo "---------- tree $C ----------"
  for N in 240 600 1200; do
    gen "$N"
    S=$( { "$H" --spec-root "$T/spec" --compiler "$T/crates/sadhana-t1/src" \
             -o "$P/fit$N-$C.elf" "$P/fit$N.t1" 2>&1; } | grep -oE 'steps: *[0-9]+' | grep -oE '[0-9]+' )
    [ -n "$S" ] || { echo "  GUARD: no step count for N=$N on $C"; exit 9; }
    echo "  $C N=$N steps=$S"
    echo "$C $N $S" >> "$P/fit.tsv"
  done
done
echo
python3 - <<'PY'
import math, collections
rows = collections.defaultdict(dict)
for line in open("/path/to/w356-probe/fit.tsv"):
    c,n,s = line.split(); rows[c][int(n)] = int(s)
for c, d in rows.items():
    ns = sorted(d)
    print(f"=== {c} ===")
    for n in ns: print(f"  N={n:>5} steps={d[n]:>14,}")
    if len(ns) >= 2:
        for a,b in zip(ns, ns[1:]):
            print(f"  {a}->{b}: n^{math.log(d[b]/d[a])/math.log(b/a):.3f}")
        print(f"  overall: n^{math.log(d[ns[-1]]/d[ns[0]])/math.log(ns[-1]/ns[0]):.3f}")
PY
date "+%H:%M:%S %Z END"
