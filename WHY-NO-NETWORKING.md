# ADR-0040 — a Sassembly program cannot wait, and that is what blocks networking

**Status:** proposed — the two options are stated so one can be chosen; neither
is adopted here.
**Date:** 2026-09-24
**Note for the public release:** this is ADR-0040, copied verbatim from the
project's decision log so the networking answer travels with the compiler. Its
original cross-references — ADR-0039 (`yantra` is the developer runtime, RV64 the
only native target), ADR-0015 (the two kinds of program), ADR-0019 (T1 has no
I/O) — are **not** included in this repository. Nothing below depends on reading
them.

## What this is about

Asked what it would take to build "all kinds of software" — servers, anything
networked — the answer is not a library. It is that **the language has no way to
express waiting**, and nothing in the toolchain has ever needed one.

This is written before any device work, because a socket device built on top of
the current model would have nowhere to put its third answer.

## Measured, 2026-09-24

A program reaches the machine through exactly two SBI calls —
`sbi_console_putchar` (EID 1) and `sbi_shutdown` (EID 8), `yantra/src/lib.rs`'s
dispatch at `:1537` — plus MMIO stores the emulator intercepts (`:752`). Every
one of those completes immediately:

| operation | answers |
|---|---|
| store to the UART (`मुद्रणम्`, `ir.t1:2331`) | always, at once |
| store to `FINISHER` | halts the machine |
| `LoadAt` from RAM | the octet that is there |
| the input channel | bytes already in RAM before `pc` moved (`input.rs`) |

**Nothing in the language can answer "not yet".** A file read has two outcomes,
a byte or the end. A socket read has three, and the third has no representation:
no sentinel a program is expected to loop on, no blocking call, no way to yield.

`yantra` does carry the machinery a waiting program would need — `sbi_set_timer`
(EID 0) at `:1548`, `stvec` trap delivery in `loader.rs`, `Halt::SpinForever`
detection — so the emulator is not the obstacle. The obstacle is that `.t1` has
no construct that reaches any of it.

## Why this is not the file-I/O question

Reading a file works today and needed no new mechanism: the host writes the
octets into RAM before the program starts. Naming and writing files need a
channel from the program to the host — real work, but with a known shape, since
`मुद्रणम्` already proves an MMIO request path.

Networking is different in kind. A request channel is not enough: the program
must be able to **not have an answer yet** and still make progress. Adding a
socket device without deciding that would produce a program that either spins
forever or reads zeros and calls it a closed connection.

## Option A — a polling sentinel

A load from the device answers a distinguished value meaning *nothing yet*, and
the program loops.

* **In favour:** needs nothing new in the compiler. `स्थानाहाररचना` (`LoadAt`,
  `ir.t1:1367`) already exists; the device is one arm in `yantra`'s address
  chain. It is the smallest possible change and it is testable today.
* **Against:** a spinning program burns the step budget. `yantra` bounds
  execution (`YANTRA_STEPS`, and `Halt::StepLimit`), and a bounded machine plus
  a spin loop means a program that waits too long is killed rather than served.
  There is also no sentinel value that cannot collide with data: an octet-wide
  port has 256 values and every one of them is a legal byte. A wider port, or a
  separate status address, is needed — which is a protocol, not a value.

## Option B — traps and a wait

The program arms a timer or a device interrupt and executes a wait; `yantra`
delivers through `stvec` and resumes it.

* **In favour:** it is what real hardware does, it does not burn the budget, and
  the plumbing is present in the emulator.
* **Against:** it needs language constructs that do not exist — a handler `.t1`
  can register, and a notion of a program suspended mid-execution. `ir.t1` has
  no arm for either, and `encode.t1` cannot emit the `ecall`/`csrw` a handler
  installation needs (measured: `ecall` appears only in the encoder's tables,
  never as something a program can produce). This is a change to the execution
  model, not an addition to a library.

## What I would want decided, and it is not the option

The choice between A and B is less urgent than this: **is a Sassembly program
allowed to be suspended?** Every current program runs to a halt, and every
measurement this project has — the fixpoint, the rungs, the corpus census —
assumes a run that ends. A program that waits is a program that can be paused,
and that invalidates the shape of "run it and read the status", which is how
essentially every check in `tools/` reports.

That is the question networking actually turns on. Until it has an answer, a
date for "all kinds of software" would be invented, and the honest statement is
that this is undesigned rather than merely unbuilt.

## What this forecloses today, said plainly

No sockets, no servers, no concurrency, and no program that waits for anything.
A Sassembly program computes, allocates, reads the input it was given, prints,
and halts. That is a real category and it is not this one.
