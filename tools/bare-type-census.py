#!/usr/bin/env python3
"""Bare cross-module record type declarations — the population of W-279's gap.

UNTIL 2026-09-19 `ir.t1` allocated storage for `चरः x ॱॱ T भवति ०` only when T
was declared in the SAME module, or when T carried a module prefix (`मॉड्यूलॱT`).
A bare name that resolved to another module got NO allocation, the slot held ०,
and the first field access faulted at address ० — as a STORE when a field is
written and as a LOAD when one is read. Both shapes were seen natively.

`आयातितसंरचनासंख्या` (ir.t1) CLOSED THE UNAMBIGUOUS HALF OF THAT: a bare name
EXACTLY ONE IMPORTED module declares public now resolves through the store and
allocates. WHAT REMAINS IS THE TIE. `आज्ञा` is declared public by both `मध्यरूप`
and `वाक्यविभाग`, `वाक्य` by both `वास्तु` and `वाक्यविभाग`; two candidates give
two field counts, and guessing would hand back a wrong SIZE where the null at
least faults. So a tie is refused, lowers to ०, and faults exactly as before.

THIS SCRIPT THEREFORE REPORTS TWO POPULATIONS AND THEY ARE NOT THE SAME CLAIM:
the unambiguous count is a legibility report, and the AMBIGUOUS count is the
one that still crashes.

TWO THINGS THIS SCRIPT LEARNED THE HARD WAY, both of which produced wrong counts:

  ॱ A type name can be declared in MORE THAN ONE module. Keying `type -> file`
    silently overwrites, and six `चरः नव ॱॱ आज्ञा` sites inside ir.t1 — which
    declare ir.t1's OWN आज्ञा and are correct — were reported as cross-module
    because वाक्यविभाग also declares आज्ञा and sorted later. Key `type -> {modules}`.

  ॱ THE INITIALISER IS PART OF THE DEFECT, not incidental. Only `भवति ०` means
    ALLOCATE; `चरः अवकाशः ॱॱ अवकाश भवति पङ्क्त्यवकाशः ...` is a record obtained
    from a call and lowers correctly as the expression it is. Matching the type
    position alone reported four sites of which two were this shape.

  ॱ Two FILES share one module: ast.t1 and vastu.t1 are both वास्तु. Comparing
    filenames instead of module names reports every वास्तु type used across that
    pair as remote. Compare MODULES.

Margins (`॰`) are stripped before matching: this corpus quotes its own idioms in
prose, and an unstripped sweep reports a comment as code.
"""
import re, glob, collections, sys

# The source directory is an ARGUMENT so the guard can run this over a
# DELIBERATELY BROKEN COPY and check it still finds the defect. A census that
# reports ० because it is broken looks exactly like a census that reports ०
# because the corpus is clean, and only the positive control separates them.
srcdir = sys.argv[1] if len(sys.argv) > 1 else 'crates/sadhana-t1/src'
files = sorted(glob.glob(srcdir.rstrip('/') + '/*.t1'))
if not files:
    print(f"no .t1 sources under {srcdir}", file=sys.stderr)
    raise SystemExit(2)
def code_lines(f):
    for i, line in enumerate(open(f, encoding='utf-8'), 1):
        yield i, line.split('॰')[0]

module = {}
for f in files:
    module[f] = '(none)'
    for _, c in code_lines(f):
        m = re.match(r'\s*मण्डलम्\s+(\S+)', c)
        if m:
            module[f] = m.group(1); break

# type name -> set of modules declaring it
decl = collections.defaultdict(set)
for f in files:
    for _, c in code_lines(f):
        m = re.match(r'\s*(?:सार्वजनिक\s+)?संरचना\s+(\S+)\s+आरभ्य', c)
        if m:
            decl[m.group(1)].add(module[f])

bare, ambiguous = [], []
for f in files:
    here = module[f]
    for i, c in code_lines(f):
        # ॱॱ is the TYPE POSITION marker. A type containing ॱ is module-qualified
        # and is exactly the case ir.t1 already handles.
        # `भवति ०` — THE NUMERAL, per ir.t1's own margin. Anything else is an
        # expression initialiser and already lowers correctly as that expression.
        for m in re.finditer(r'चरः\s+(\S+)\s+ॱॱ\s*(\S+)\s+भवति\s+०\s*।', c):
            var, ty = m.group(1), m.group(2)
            if 'ॱ' in ty:
                continue
            owners = decl.get(ty)
            if not owners or here in owners:
                continue      # not a record type, or this module's own — allocated today
            row = (f.split('/')[-1], i, var, ty, sorted(owners))
            (ambiguous if len(owners) > 1 else bare).append(row)

for n, i, var, ty, o in bare:
    print(f"  {n}:{i}  चरः {var} ॱॱ {ty}   <- module {o[0]}")
for n, i, var, ty, o in ambiguous:
    print(f"  {n}:{i}  चरः {var} ॱॱ {ty}   <- AMBIGUOUS across {o}")

# TWO POPULATIONS, TWO LINES, AND THEY ARE DISJOINT — the summary said
# "of which ambiguous" for weeks and that was FALSE: an ambiguous row goes to
# the other list, so it was never counted in the first number. A corpus with
# five ambiguous bare declarations printed `bare ...: 0`, and a ratchet reading
# only that line would have passed over five sites that get no storage.
#
# SINCE 2026-09-19 THE TWO MEAN DIFFERENT THINGS, which is the other reason
# they may not share a line. `ir.t1`'s `आयातितसंरचनासंख्या` resolves a bare type
# through the current module's imports and allocates for it — so a bare name
# EXACTLY ONE imported module declares is now correct, and the first count is a
# legibility report. A TIE is refused rather than guessed (two candidates, two
# field counts, and a wrong size corrupts where a null at least faults), so the
# second count is the population that still lowers to ० and still faults
# natively at its first field access. That one is the correctness ratchet.
print(f"\n  bare cross-module declarations: {len(bare)}")
print(f"  ambiguous bare cross-module declarations: {len(ambiguous)}")
print(f"  record type names declared in >1 module: "
      f"{sorted(k for k, v in decl.items() if len(v) > 1)}")
