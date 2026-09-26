# Sassembly v0.2.0 — a compiler with no English in it compiles itself

**Sassembly is a systems language whose keywords are Sanskrit words and whose
operand roles are marked by kāraka sigils instead of by position. Its compiler is
written in Sassembly. As of this release, that compiler compiles itself into a
byte-identical copy.**

---

## The result

```
Stage 1   21 Sassembly sources, compiled by the interpreted compiler
Stage 2   Stage 1 running natively on bare-metal RISC-V, compiling those same sources

Stage 2 == Stage 1, byte for byte.  1,393,602 octets.
```

Reproducible with one command, `tools/fixpoint.sh`, which packs the source blob
from the working tree immediately before building so a stale input cannot be
mistaken for agreement.

Separately, and on a different date: the whole corpus links into one image of
**1,373,231 octets that runs** — 21 objects, `status: Some(0)`, 1,763 s.

## Why the fixpoint is the interesting number

A self-hosting compiler is usually announced when it can compile itself *and
produce something that works*. That is a weaker claim than it sounds. Two
compilers can both work and still disagree, and the disagreement can hide for a
long time.

That is not a hypothetical here. Reaching this fixpoint meant finding five
defects in a single day that were **invisible to the interpreter and visible only
when the compiled compiler ran itself** — among them:

* a shift by 64, which is 2⁶⁴ in the host's arithmetic and **1** on RISC-V, so a
  hex writer's mask was zero natively and correct interpreted;
* a word above 2⁶³ read as negative by a lowering that was signed where the
  language is not;
* an optional holding zero, indistinguishable from the nil word natively — which
  was the bootstrap blocker, with 484 of 509 slots holed.

Each was a case where the interpreter executing a source *hides* a defect in that
same compiler's output. Byte-identity is the only form of the claim that cannot
be satisfied by two compilers that merely both seem to work.

## What this is not

A general-purpose toolchain. Said plainly, and before the tutorial rather than in
a footnote:

**No file naming, no file writing, no command-line arguments, no network, no
threads, no clock, no standard library.** A program computes, allocates from a
512 MiB region the compiler already reserves, reads the input it was handed,
prints, and halts.

Networking in particular is **undesigned rather than unbuilt**. Nothing in the
language can express "not yet": a socket read has three outcomes and the third
has no representation. The prior question is whether a Sassembly program may be
*suspended at all* — and every measurement this project owns assumes a run that
ends. That question is recorded, with its two candidate answers, and neither is
adopted.

## What works today, with the guard that proves it

| capability | measured |
|---|---|
| read the input it was given | `status: Some(2289)` — the exact octet sum of a 25-byte file |
| allocate dynamically | a run grown to 5,000 elements, every element surviving |
| print to the console | `HI!\n`, asserted from **both** engines and asserted to agree |
| compile a fresh program | 13.4 s to a 65,832-octet ELF |

Each has a test. The console guard is the strongest of the three, because the
output channel is intercepted on both sides by design, so one program's output
can be read from either engine and compared — and cross-engine disagreement is
the class every defect in this project has belonged to.

## Verification

`cargo test -p sadhana-t1 -p sanskrit-text -p yantra --no-fail-fast` —
**129 targets, 1,151 passed, 0 failed, exit 0** (2026-09-25).

That covers the three crates this release is built from. The wider project runs a
further hourly gate over an operating system that is not in this repository.

## Status

**v0.2.0.** Nothing is stable — not the syntax, not the object format, not the
tool names. The fixpoint is the result; everything around it is scaffolding for
having reached it.

**MIT licensed.**
