#!/bin/bash
# W-346: fit the build cost of the literal-site probe ABOVE 1,200 sites, and
# locate the stage that carries any excess. Measurement only; changes nothing.
#
# PREDICTION, WRITTEN BEFORE THESE RUNS (2026-10-04, agent ae-w346), in the
# row's own terms (steps = interpreter steps printed by t1_image, deterministic):
#   The row's last figures (W-356 compiler, 3309cb42 lineage) are 1,200 / 6,000 /
#   12,000 sites = 1.541 G / 9.629 G / 24.095 G steps, i.e. per-site cost
#   1.284 M / 1.605 M / 2.008 M. Three candidate models and what separates them:
#     linear     per-site cost flat; every pairwise exponent ~1.00.
#     n log n    per-site cost rises with log n: equal increments per DOUBLING,
#                pairwise exponent ~1 + 1/ln n ~ 1.11 and FALLING as n grows.
#     a n + b n^2  per-site cost rises with n: equal increments per ADDED SITE,
#                pairwise exponent climbing toward 2.
#   The row's three points already give per-site slopes of 6.7e-5 M/site over
#   1,200->6,000 AND over 6,000->12,000 (equal per added site), against
#   0.20 vs 0.58 M per ln-unit (unequal per doubling). So I PREDICT a n + b n^2
#   with b ~ 67 steps per site per prior site, a ~ 1.2 M, and per-site cost at
#   2,400 / 4,800 / 9,600 / 19,200 of ~1.37 / 1.53 / 1.85 / 2.49 M.
#   A per-FUNCTION walk predicts the SPLIT shape (the same N sites spread over
#   functions of 600) is linear; a per-MODULE walk predicts SPLIT keeps b.
#
# Usage: w346-fit.sh <repo root> <out dir>    (t1_image from <root>/target/release)
set -u
R=${1:?repo root}; O=${2:?out dir}; mkdir -p "$O"
H="$R/target/release/t1_image"
gen() { # gen <shape one|split> <N> <file>
python3 - "$1" "$2" "$3" <<'PY'
import sys
DEV="०१२३४५६७८९"
def dev(n): return "".join(DEV[int(c)] for c in str(n))
shape, N, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
def sites(lo, hi):
    return [f"    सूची अङ्कः {dev(i)} अन्तः भवति {dev(i*7%997)} ।" for i in range(lo, hi+1)]
if shape == "one":   # byte-identical to w356-fit.sh's generator (fit240.t1probe)
    L=[f"मण्डलम् बहुमूल्य{dev(N)} ॥","","सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि",
       "    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।"]
    L+=sites(1,N)
    L+=["","    प्रत्यागमनम् ० ।","इति",""]
else:                # split: the same N sites, 600 per function, each called once
    K=N//600
    L=[f"मण्डलम् बहुमूल्यखण्ड{dev(N)} ॥",""]
    for k in range(1,K+1):
        L+=[f"वृत्तिः खण्ड{dev(k)} आदाय क न६४ ददाति न६४ आदि",
            "    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति क ।"]
        L+=sites(1,600)
        L+=["    प्रत्यागमनम् ० ।","इति",""]
    L+=["सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि"]
    for k in range(1,K+1):
        L+=[f"    चरः र{dev(k)} न६४ भवति खण्ड{dev(k)} ० ।"]
    L+=["    प्रत्यागमनम् ० ।","इति",""]
open(out,"w").write("\n".join(L))
PY
}
run() { # run <shape> <N> [profile]
  local f="$O/$1$2.t1"; gen "$1" "$2" "$f"
  local env=(); [ "${3:-}" = profile ] && env=(T1_CALLS=1)
  env "${env[@]}" nice -n 10 "$H" --spec-root "$R/spec" --compiler "$R/crates/sadhana-t1/src" \
      -o "$O/$1$2${3:+-$3}.elf" "$f" > "$O/$1$2${3:+-$3}.log" 2>/dev/null
  local s; s=$(grep -oE '^steps: *[0-9]+' "$O/$1$2${3:+-$3}.log" | grep -oE '[0-9]+')
  local o; o=$(grep -oE '^write: *[0-9]+' "$O/$1$2${3:+-$3}.log" | grep -oE '[0-9]+')
  [ -n "$s" ] || { echo "GUARD: no step count for $1 $2 $3" >&2; return 9; }
  # A REFUSED build still prints steps (split 19,200 did: 7.2 G steps, a 65,704-octet
  # image). Its row is kept for the record with octets NA; w346-fit.py drops it.
  grep -q '^build: *0 source(s) failed' "$O/$1$2${3:+-$3}.log" || o=NA
  echo -e "$1\t$2\t${3:-plain}\t$s\t${o:-NA}" >> "$O/w346.tsv"
}
export -f gen run; export R O H
# The 0-site baseline is the one-shape generator at N=0 (an empty site list).
{ for n in 0 600 1200 2400 4800 9600 19200; do echo "one $n"; done
  for n in 1200 4800 9600 19200; do echo "split $n"; done
  for n in 2400 9600; do echo "one $n profile"; done
} | xargs -P 13 -L 1 bash -c 'run "$@"' _
sort -k1,1 -k3,3 -k2,2n "$O/w346.tsv"
