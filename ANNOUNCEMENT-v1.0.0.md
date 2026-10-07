# Sassembly v1.0.0 — Sassembly is complete

**The language and its self-hosting compiler are finished.** Every item on the
completion list is done, and the compiler still compiles itself to the same bytes
it started from.

`v0.4.0` gave the language floating point. `v1.0.0` closes the rest: vectors and
matrices, one integer meaning on every engine, a one-octet-per-letter data
literal, a hashed symbol table in the linker, and the devices a program needs to
talk to the outside world — files, threads, sockets and a 2D GPU.

## The claim, and how to check it yourself

```
tools/fixpoint.sh
  Stage 1  922146 octets    built by the interpreted compiler from its own sources
  Stage 2  922146 octets    Stage 1, running natively on yantra, compiling those same sources
  FIXPOINT HOLDS: 922146 octets, byte-identical
```

21 sources, 21 objects linked, `stubs: 0`. Stage 2 runs to a finisher with status
1200 (built), and takes **55,385,359,354** executed instructions to do it. The
image is `71d06d66…15480` (sha256 of `stage1.elf`), and it is **34.1% smaller than
the v0.4.0 image** (1,399,434 → 922,146 octets) while the language has grown.

```sh
cargo build --release -p sadhana -p yantra
tools/fixpoint.sh
```

## What is new since v0.4.0

**Vectors and matrices.** The RISC-V V extension runs in yantra, with vector
lowering in the compiler and matrix and tensor syntax on top. Vector words follow
ADR-0043: operations are call forms, not new keywords, so no existing program
changes meaning.

**One integer meaning across engines.** The interpreter and the native engines now
agree on integer semantics, and the language has *checked* operators for the cases
where they might not (`अष्टकॱसुरक्षितयोगः` and its siblings). A violated check does
not trap with an opaque fault; it halts with a **named refusal** in the finisher's
FAIL form:

| status | meaning |
|---|---|
| `0x355` | out-of-bounds read |
| `0x35b` | aliasing |
| `0x35c` | overflow |
| `0x35d` | out-of-bounds write |
| `0x35e` | division by zero |

Unsigned runs (`न३२`) now load zero-extended.

**Devanagari-8 (ADR-0044).** A data literal (`वर्णाष्टकम्`) can store one octet per
letter. On the measured data that is 35% smaller.

**Symbol lookup.** The linker now keeps a hashed name table for symbol lookup
(the first two steps of that work).

**The same answer everywhere yantra runs.** One image executes the same number of
instructions on x86-64, on aarch64 and in the browser through `yantra-wasm`.

**Devices a program can reach from yantra.** A file window (`पत्रम्`), threads,
sockets and virtio-gpu 2D. The file window is a device a store completes, not a
new instruction, so the compiler gained no new kind of operation. The fixpoint images are
*checked* never to use them: `tools/fixpoint.sh` fails if Stage 2 touches the
socket, reads the retired-instruction counter, or declares threads, because any of
those would make Stage 2 a statement about its environment instead of its input.
These devices answer the question [WHY-NO-NETWORKING.md](WHY-NO-NETWORKING.md)
(ADR-0040) asked; that document is kept as the record of the question.

## Tests

```
cargo test --workspace --release --no-fail-fast
  260 binaries   2431 passed / 56 failed / 68 ignored
```

The 56 failures are the same kind as in v0.4.0: tests that measure the whole
development repository (its `research/` data, `tests/corpus/`, `docs/adr`, and the
count of every `.t1` file in it) and so cannot run on this one. None tests the
compiler, the machine or the fixpoint. v0.4.0 passed 1,634 and failed 61.

## What this release is not

- **Web apps are not here.** Compiling Sanskrit to HTML, CSS and JavaScript is
  planned for a later release.
- **The GPU is 2D only.** A compute path (WGSL, ADR-0045) is accepted as a design
  and is not built.
- **Floats and vectors under QEMU wait on a startup fix.** They run on yantra
  today.
- **It is still a research artifact.** Complete means the language and compiler
  are finished, not that there is an application toolchain around them. The
  interfaces may still change in later releases.

## Assets

The self-hosted compiler image, `sassembly-v1.0.0-stage1.elf` (RV64, 922,146
octets), with its provenance and `SHA256SUMS`, is attached to the release.
