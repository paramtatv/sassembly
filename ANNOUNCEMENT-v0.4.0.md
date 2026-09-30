# Sassembly v0.4.0 — it computes with real numbers

**Sassembly computes with real numbers, and a machine this project did not write agrees.**

`v0.3.0` put the assembler in a browser. `v0.4.0` gives the language floating
point — encoded, oracle-checked against GNU binutils, and then **executed on two
independent machines that produce the same answer**.

## The claim, and how to check it yourself

`oracle/float-oracle.sas` is a Sassembly program that is its own oracle. Every
check compares a result's **IEEE bit pattern** through `प्लवसंचारः` into an integer
register and branches on an integer compare, so the verdict never rests on a float
comparison — a wrong `fadd.d` cannot be hidden by an equally wrong `feq.d`.

```
tools/check-float-oracle.sh
  ok  float arithmetic agrees on two machines — yantra and qemu-system-riscv64
      yantra: halt 0x5555 in 34 executed instructions
      qemu:   exit 0 (the program's own finisher write decides)
```

Six checks: `1.0 + 2.0 == 3.0` exactly, `2.0 × 3.0 == 6.0`, `6.0 − 3.0 == 3.0`,
`√4.0 == 2.0`, and `feq.d` answering १ on equal operands **and ० on unequal**, so
the fifth cannot pass vacuously.

It needs `qemu-system-riscv64`. Without it the script exits 77 — `CANNOT RUN` —
rather than passing, because running only `yantra` would be this repository
agreeing with itself.

## The self-hosting fixpoint still closes

```
tools/fixpoint.sh
  Stage 1  1,399,434 octets
  Stage 2  1,399,434 octets
  FIXPOINT HOLDS: 1399434 octets, byte-identical
```

21 sources, 21 objects linked, `stubs: 0`. The compiler compiles itself to the same
image, and that image compiles the same sources to the same image again.

## What is new since v0.3.0

**Floating point in the machine.** `crates/yantra/src/fp.rs` — F and D, NaN-boxing
for single precision, RISC-V's own min/max NaN rule rather than Rust's, the
mandated `fcvt` saturation, and **non-RNE arithmetic HALTS rather than
approximating**. A rounding mode this machine does not implement is a refusal, not
a guess.

**A type letter that follows its root.** `भिन्न` denoted a *fraction* — the
project's own document glossed the float registers "fractional" — which collides
with any future rational type. ADR-0042 moves the root to `प्लव` and, because a
type letter is the initial of a chosen word with a published derivation, moves the
letter with it:

```ebnf
float_type = "प" , ( "३२" | "६४" ) ;        (* प्लव *)
```

`प३२` and `प६४`. 22 of 22 float mnemonics now carry the `प्लव` prefix — including
the four that previously lacked it because no integer instruction shared their
name.

**432 float encodings, checked against an authority outside this tree.**
`spec/conformance-t0.tsv` is generated against `riscv64-elf-as`, and the generator
refuses to write the file if the assembler rejects a case. Every one of the 62
float rows is exercised through the `.t1` encoder: `flw`/`fld`/`fsw`/`fsd` at 28
cases each and the four fused-multiply forms at 6 per width.

```
METRIC t1_float_oracle_cases 432   families 62   refused 0   disagreements 0
```

## Measured, in a fresh clone of this repository

2026-09-30, on an Apple Silicon Mac, from this repository and not the upstream one:

```
tools/fixpoint.sh
  packed 21 source(s), 4375038 octets
  Stage 1  1399434 octets                      23m46s
  Stage 2  1399434 octets                      41m57s
  status:  1200 — BUILT (shrinkhala.t1:3528)
  FIXPOINT HOLDS: 1399434 octets, byte-identical

tools/check-float-oracle.sh
  yantra: halt 0x5555 in 34 executed instructions
  qemu:   exit 0
```

Stage 1's own controls — `build: 0 source(s) failed to compile, 1 declared
nothing, 21 object(s) linked`, `stubs: 0`, and **`steps: 27807029110`** — are
IDENTICAL to the upstream tree's. That last number is the interpreted compiler's
instruction count for the whole build, and it moves if any byte of any source or
spec table differs. Two repositories with different workspaces produce it exactly.

**`cargo test` is not a claim this repository makes, and it was not one in v0.3.0
either.** 17 test targets read `tests/corpus/`, `tests/levels/`, UCD data tables,
`docs/adr` or `spec/programs.tsv` — none of which ship here, because they belong
to the development monorepo. Measured on this release and on v0.3.0 with the same
command: the SAME 17 targets fail on both, and this release passes **70 more
tests over 3 more binaries** than v0.3.0 did.

```
v0.3.0    1564 passed / 61 failed / 59 ignored / 174 binaries
v0.4.0    1634 passed / 61 failed / 59 ignored / 177 binaries
```

The three added binaries are `fp_oracle`, `fp_extension` and
`t1_branch_range_per_branch` — the float and branch-range suites, which a
floating-point release has no business omitting.

## What this release does NOT claim

The `.t1` compiler needs no Rust in its build loop. The **stack** does: `yantra` is
a Rust RV64 machine and is permanent infrastructure, the way a C compiler does not
ship its own silicon. Applications may be written in any language over the ABI.

`V` — the vector extension — is not here. Nothing has begun it.

One optimisation is deliberately absent. An inline capacity test in `ir.t1` makes
the 21-source self-hosted build refuse one module: a conditional lands 4,116 bytes
from its target, past RISC-V's ±4 KiB B-type range, and **neither emitter relaxes
a branch** — both refuse. That is a named, tracked gap with a known remedy, and it
is out of this release rather than papered over.
