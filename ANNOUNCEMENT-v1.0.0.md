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

**Devices a program can reach from yantra.** A file window (`पत्रम्`) that reads
and writes files under a root directory the host grants, command-line arguments,
a clock, one host socket, cooperative threads, and virtio-gpu 2D. The clock, the
socket and the threads follow one rule: outside events are delivered only at an
explicit wait and are recorded in an event log, so a run can be replayed exactly,
with no network or clock needed. Threads are switched by the host only at waits,
never preempted. The fixpoint images are *checked* never to use these:
`tools/fixpoint.sh` fails if Stage 2 touches the socket, reads the
retired-instruction counter, or declares threads, because any of those would make
Stage 2 a statement about its environment instead of its input. There is no TLS,
DNS or HTTP. [WHY-NO-NETWORKING.md](WHY-NO-NETWORKING.md) (ADR-0040) is kept as
the record of the question these answer.

## Measured, in a fresh clone of this repository

2026-10-07, on a Linux x86-64 host with 20 cores, from this repository's tree
and not the upstream one (a release build, with the test suite running alongside):

```
tools/fixpoint.sh
  packed 21 source(s), 5040596 octets
  Stage 1  922146 octets                       about 26 minutes
  Stage 2  922146 octets                       about 16 minutes
  status:  1200 - BUILT (shrinkhala.t1:3551)
  FIXPOINT HOLDS: 922146 octets, byte-identical
```

Stage 2 reached a high water of 1,049,907,192 octets of the 2,684,354,560 the run
is given. Stage 1's own controls - `build: 0 source(s) failed to compile, 1
declared nothing, 21 object(s) linked`, `stubs: 0`, and **`steps: 22192201700`** -
are the interpreted compiler's instruction count for the whole build, and it moves
if any byte of any source or spec table differs. The compiler is 21 `.t1` sources,
46,794 lines.

In a browser (`tools/build-sassembly-web.sh`): `sadhana-wasm` 430,275 bytes and
`yantra-wasm` 196,737 bytes, both instantiated with an empty import object. The
playground is at <https://paramtatv.github.io/sassembly/playground.html>.

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

## How to get it

The release page is
<https://github.com/paramtatv/sassembly/releases/tag/v1.0.0>. It carries
`sassembly-v1.0.0-stage1.elf` (RV64, 922,146 octets), `stage1.provenance.txt` and
`SHA256SUMS`. Download them and check:

```sh
sha256sum -c SHA256SUMS
```

`SHA256SUMS` lists two files:

```
71d06d6649a861c1c6205c95a59fd362f48914b377c6ace9e612bef827415480  sassembly-v1.0.0-stage1.elf
35bc530183591b96dd9a9eb84620dc23ceb37e285941d50f2545cfee47ce98ff  stage1.provenance.txt
```

To rebuild the image yourself from source instead of trusting it:

```sh
git clone https://github.com/paramtatv/sassembly
cd sassembly && git checkout v1.0.0
cargo build --release -p sadhana -p yantra
tools/fixpoint.sh
```
