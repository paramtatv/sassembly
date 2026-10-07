#!/usr/bin/env python3
"""W-346: fit w346.tsv (written by w346-fit.sh). Rows: shape N kind steps octets.
A row is a point only if its build linked (octets above the 0-site image);
split 19,200 was REFUSED (load-dependent size ceiling) and is excluded by that test."""
import math, sys, numpy as np
path = sys.argv[1] if len(sys.argv) > 1 else __file__.replace("w346-fit.py", "w346.tsv")
R = [l.rstrip("\n").split("\t") for l in open(path)]
num = lambda x: int(x) if x.isdigit() else -1   # octets NA = refused build
one = {int(r[1]): (int(r[3]), num(r[4])) for r in R if r[0] == "one" and r[2] == "plain"}
B, O0 = one[0]
ok = lambda v: v[1] > O0
one = {k: v for k, v in one.items() if k == 0 or ok(v)}
spl = {int(r[1]): (int(r[3]), num(r[4])) for r in R if r[0] == "split" and r[2] == "plain"}
spl = {k: v for k, v in spl.items() if ok(v)}
FIT = {}

def table(d, label):
    ns = sorted(n for n in d if n > 0)
    print(f"{label}: N steps per-site(base-sub) pairwise(raw, base-sub)")
    for i, n in enumerate(ns):
        s = d[n][0]; pw = ""
        if i:
            m = ns[i - 1]
            pw = "n^%.3f  n^%.3f" % (math.log(s / d[m][0]) / math.log(n / m),
                                     math.log((s - B) / (d[m][0] - B)) / math.log(n / m))
        print(f"  {n:>6} {s:>15,} {(s - B) / n / 1e6:.4f}M  {pw}")

def models(d, label):
    n = np.array(sorted(d), float); y = np.array([d[int(k)][0] for k in n], float)
    for name, X in (("linear", np.c_[np.ones_like(n), n]),
                    ("n log n", np.c_[np.ones_like(n), n, n * np.log(np.maximum(n, 1))]),
                    ("a n+b n^2", np.c_[np.ones_like(n), n, n * n])):
        c, *_ = np.linalg.lstsq(X / y[:, None], np.ones_like(y), rcond=None); r = y - X @ c  # relative-error weighting
        print(f"  {label:6s} {name:9s} coef={['%.4g' % v for v in c]}  res%={' '.join('%+.2f' % v for v in r / y * 100)}")
        FIT[label, name] = (c, max(abs(r / y)))
    m = n > 0
    e = np.polyfit(np.log(n[m]), np.log(y[m] - B), 1)[0]
    print(f"  {label:6s} power law over N>0, base-subtracted: n^{e:.3f}")

table(one, "one"); models(one, "one")
table(spl, "split"); models({0: one[0], **spl}, "split")
ns = sorted(n for n in one if n > 0)
o = np.array([one[k][1] - O0 for k in ns], float)
print("control, image octets (one): n^%.3f" % np.polyfit(np.log(ns), np.log(o), 1)[0])

# THE RECORDED VERDICT, AS A CHECK THAT REFUSES (exit 1), not a report.
# W346_BMAX overrides the ceiling on the quadratic coefficient (to show this
# check red: W346_BMAX=3 reads RED on the linear data).
# PINNED TO THE FULL FIX (agent ae-w346-walk3, 2026-10-05): all three block
# walks are linear — the resolver's and typechecker's (artha.t1, 6a92dd5e) and
# the lowering's `वाक्यरचना` (ir.t1). b fell 70.2 -> 26.2 -> 4.2. The residual
# is NOT a block walk: per-routine self-steps 2,400 -> 9,600 put it on lexing
# and literal routines at n^1.02-1.05 (परिधिसाम्यम्, कोष्ठपङ्क्तिः,
# प्रकारसंख्या), consistent with the generator's index numerals growing a digit
# (1..N); the split series, whose indices stay 1..600, reads b 1.5. So a
# RELATIVE band around a pin near 0 cannot work; this is an ABSOLUTE ceiling,
# one tenth of the pre-fix 70.2. w346-before.tsv (b 70.2) and the partial-fix
# data (b 26.2) both read RED under it.
bmax = float(__import__("os").environ.get("W346_BMAX", "7"))
(c1, r1), (c2, r2) = FIT["one", "a n+b n^2"], FIT["split", "linear"]
fails = []
if not (r1 < 0.01 and c1[2] < bmax):
    fails.append(f"one: b={c1[2]:.1f} (want below {bmax:g}), max res {r1:.2%} (want <1%)")
if not (r2 < 0.01 and FIT["split", "a n+b n^2"][0][2] < bmax):
    fails.append(f"split: not linear (max res {r2:.2%}, b={FIT['split', 'a n+b n^2'][0][2]:.1f})")
print("VERDICT:", "RED " + "; ".join(fails) if fails else
      f"GREEN one block is linear (b={c1[2]:.1f} below {bmax:g}); the same sites split 600 per function are linear")
sys.exit(1 if fails else 0)
