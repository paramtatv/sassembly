<p align="center">
  <img src="assets/banner.png" alt="संस्कृतयन्त्रम् · Sassembly — a grammar-based instruction set. Stage 2 == Stage 1, byte for byte." width="100%">
</p>

<p align="center">
  <a href="#status-and-stability"><img src="https://img.shields.io/badge/version-v0.4.0-A63A21?style=flat-square&labelColor=2B2521" alt="version v0.4.0"></a>
  <a href="#licence"><img src="https://img.shields.io/badge/licence-MIT-A63A21?style=flat-square&labelColor=2B2521" alt="licence MIT"></a>
  <a href="#the-claim-and-how-to-check-it"><img src="https://img.shields.io/badge/target-RISC--V%20RV64-1F6F6B?style=flat-square&labelColor=2B2521" alt="target RISC-V RV64"></a>
  <a href="https://paramtatv.github.io/sassembly/"><img src="https://img.shields.io/badge/docs-paramtatv.github.io%2Fsassembly-1F6F6B?style=flat-square&labelColor=2B2521" alt="documentation"></a>
</p>

<p align="center">
  <a href="https://discord.gg/XvYvXR8HAh"><img src="assets/btn-discord.png" alt="Join the study group on Discord · अध्ययनसङ्घः" width="344"></a>
</p>

<h1 align="center">Sassembly · संस्कृतयन्त्रम्</h1>

<p align="center">
  <strong>A compiler that compiles itself, written in a language with no English in it,<br>
  targeting bare-metal RISC-V.</strong>
</p>

<p align="center">
  <a href="https://paramtatv.github.io/sassembly/">docs and playground</a> ·
  <a href="https://discord.gg/XvYvXR8HAh">study group</a> ·
  <a href="ANNOUNCEMENT-v0.4.0.md">v0.4.0 announcement</a> ·
  <a href="ANNOUNCEMENT-v0.2.0.md">v0.2.0 announcement</a>
</p>

---

Sassembly is an instruction set architecture and a systems language whose
keywords are Sanskrit words and whose operand roles are marked by **kāraka
sigils** rather than by position or punctuation. Its compiler is written in
Sassembly. That compiler, compiled by itself, produces a byte-identical copy of
itself.

> [!IMPORTANT]
> This release is a **research artifact**. It is a compiler milestone, not an
> application toolchain — see [What this cannot do](#what-this-cannot-do), which
> is deliberately placed before the tutorial.

### At a glance

| | |
|---|---|
| **Stage 2 == Stage 1** | byte-identical, `1,399,434` octets |
| **the compiler** | 21 `.t1` sources, 39,775 lines, written in Sassembly |
| **target** | bare-metal RISC-V RV64, no LLVM, no external toolchain |
| **in a browser** | [playground](https://paramtatv.github.io/sassembly/playground.html) — 476 KB of wasm, no server |
| **measured** | 2026-09-25, in a fresh clone of this repository |
| **status** | v0.4.0 — nothing is stable |

### Contents

1. [The claim, and how to check it](#the-claim-and-how-to-check-it)
2. [What this cannot do](#what-this-cannot-do)
3. [What a program can do today](#what-a-program-can-do-today)
4. [It runs in a browser](#it-runs-in-a-browser)
5. [A first look at the language](#a-first-look-at-the-language)
6. [The heap already exists](#the-heap-already-exists)
7. [Verification](#verification)
8. [Reading the source](#reading-the-source)
9. [Two things a reader will notice](#two-things-a-reader-will-notice)
10. [Status and stability](#status-and-stability)
11. [The study group](#the-study-group)
12. [Licence](#licence)

---

## The claim, and how to check it

The interesting property of a self-hosting compiler is a fixpoint: if you
compile the compiler with itself, you should get the compiler back. Not an
equivalent binary — *the same bytes*.

```
Stage 1   the compiler's 21 sources, compiled by the interpreted compiler
Stage 2   Stage 1 running natively on RISC-V, compiling those same 21 sources
```

**Measured 2026-09-30, `tools/fixpoint.sh`, in a fresh clone of THIS
repository** — not inherited from the tree it was extracted from:

```console
fixpoint: packing the corpus from crates/sadhana-t1/src
packed 21 source(s), 4375038 octets
fixpoint: Stage 1  1399434 octets
fixpoint: Stage 2  1399434 octets
  status:  1200 — BUILT (shrinkhala.t1:3528)
FIXPOINT HOLDS: 1399434 octets, byte-identical
```

Stage 1 took 23m46s and Stage 2 41m57s on an Apple Silicon Mac, with a high
water of 1,602,481,536 octets of the 2,684,354,560 the run is given. The
figures below are that run.

Stage 1's own controls — `build: 0 source(s) failed to compile, 1 declared
nothing, 21 object(s) linked`, `stubs: 0`, and **`steps: 27807029110`** — are
identical to the development tree's. That last figure is the interpreted
compiler's instruction count for the whole build, and it moves if any byte of any
source or spec table differs.

| quantity | value |
|---|---|
| Stage 2 == Stage 1 | **byte-identical** |
| image size | **1,399,434 octets** |
| sources | **21** `.t1` files, 40,542 lines |

Reproduce it:

```sh
cargo build --release -p sadhana -p yantra   # builds t1_image and yantra-run
tools/fixpoint.sh
```

This needs five things present, and they are the whole of what "the compiler"
means here: the 21 `.t1` sources, the `spec/` tables (which the host fills), the
`t1_image` driver, the `yantra` RISC-V emulator, and
`tools/pack-corpus.py`. Without them the headline number above is a claim you
would have to take on trust rather than check.

> [!NOTE]
> The script packs the source blob from the working tree **immediately before
> the build**, with no reuse flag. That is not incidental: an earlier run of
> this measurement was invalidated by a blob packed four hours before the edit
> it was supposed to test, and reported a divergence that did not exist. The
> tell was that a *different* Stage 1 produced a byte-identical Stage 2. The
> script now makes staleness impossible rather than unlikely.

### A separate result: the whole corpus as one running image

Distinct from the fixpoint, and worth stating separately because the numbers get
confused:

**Measured 2026-09-18** — 21 sources, 21 objects linked into one image of
**1,373,231 octets**, which *runs*: `halt Finisher { value: 21845, status:
Some(0) }`, 1,763 s.

> [!WARNING]
> That is a different artifact from the 1,399,434-octet fixpoint image, built on
> a different date. **Neither number is a typo for the other.**

---

## What this cannot do

Stated first, and in full, because a self-hosting compiler invites the
assumption that a general-purpose toolchain comes with it. It does not.

| capability | available today |
|---|---|
| **name or open a file** | no |
| **write a file** | no |
| **command-line arguments** | no |
| **network, sockets, servers** | no — and *undesigned*, not merely unbuilt |
| **threads, concurrency** | no |
| **clock** | no |
| **standard library** | `lib.t1` is fifteen lines of comments and declares no module, so nothing can import it |

Networking is the one that is not a matter of effort. **Nothing in the language
can express "not yet."** A file read has two outcomes, a byte or the end; a
socket read has three, and the third has no representation — no sentinel, no
blocking call, no way to yield. `ecall` appears **zero** times as something a
program can emit. Before a socket device can exist, someone has to decide
whether a Sassembly program may be *suspended at all* — and every measurement
this project owns is shaped as "run it and read the status", which assumes a run
that ends. [WHY-NO-NETWORKING.md](WHY-NO-NETWORKING.md) (ADR-0040)
records the question and the two candidate answers, and adopts neither.

---

## What a program can do today

All of the following are measured and have guard tests, listed so the claims can
be checked rather than taken:

| capability | measured | guard |
|---|---|---|
| compute, branch, loop, records, arenas | `status: Some(55)` = Σ1..10 | — |
| compile a fresh program | **13.4 s**, 65,832-octet ELF | — |
| **read the input it was given** | `status: Some(2289)`, the exact octet sum of a 25-byte file | [`t1_user_input_interface.rs`](crates/yantra/tests/t1_user_input_interface.rs) |
| **allocate dynamically** | a run grown to 5,000 elements and summed | [`t1_user_allocation.rs`](crates/yantra/tests/t1_user_allocation.rs) |
| **print to the console** | `HI!\n`, asserted from **both** engines | [`t1_user_console.rs`](crates/yantra/tests/t1_user_console.rs) |
| return a result | the finisher status — **sixteen bits** | — |

Two footnotes that will otherwise cost you an afternoon:

> [!CAUTION]
> **The status is sixteen bits.** A correct answer above 65535 looks like
> garbage. Compare modulo 65536 before concluding anything is broken: a fixture
> summing 0..4999 = 12,497,500 reports 45,660, and that is right.

> [!NOTE]
> **"Reading input" is not a file API.** The host writes the file's octets *into
> RAM before the program starts*, locating the slots by scanning memory for a
> magic word. There is no port and no syscall. That is why it costs nothing, and
> also why it does not generalise: naming a file requires the program to ask the
> host something *while running*.

---

## It runs in a browser

New in v0.3.0 and unchanged since, and the shortest way to see the thing work:

**<https://paramtatv.github.io/sassembly/playground.html>** — type Devanagari
assembly, press चालय, and the page assembles it and executes it in your tab.

Two crates compiled to wasm do the whole of it:

| | | |
|---|---|---|
| `crates/sadhana-wasm` | **401 KB** | Devanagari assembly → ELF |
| `crates/yantra-wasm` | **75 KB** | an RV64 machine that runs the ELF |

Both are instantiated with an **empty import object**. That is the claim rather
than an omission: the page grants them no syscalls, no clock and no network,
because there is nothing for them to ask for.

Build the standalone page yourself — one self-contained file, no server:

```sh
rustup target add wasm32-unknown-unknown
tools/build-sassembly-web.sh            # writes to a temp dir; pass a path to choose
```

**Measured 2026-09-27, in a fresh clone of this repository**, 54 s from cold:

```
  namaste      proof 848 bytes  sha256 bf2bca26637bc84d
  bare-metal   proof 856 bytes  sha256 d92d476f3ede6e90
  atithi       app   720 bytes  sha256 68655059ec1ab5ba
  wasm         75314 bytes
wrote sassembly.html (109576 bytes) — open it directly, no server needed
```

Those three hashes are the same bytes the upstream tree produces.

> [!NOTE]
> **The browser assembles `.sas`, not `.t1`.** This is Sassembly *assembly* —
> the layer with kāraka sigils on operands. The `.t1` systems language the
> compiler itself is written in is **not** compiled in the browser; that needs
> `t1_image` and the `spec/` tables. The page says so rather than leaving you to
> discover it from a refusal.

One detail worth knowing before you write your own: a bare-metal **proof** is
linked at the reset vector and a hosted **application** at `0x2000_0000`
(`spec/application-load.tsv`), because the reset vector sits inside the
supervisor's own gigabyte. Assemble an application at the wrong one and the
refusal arrives from the *loader*, a stage later, talking about superpages.
`Sadhana.BARE_METAL` and `Sadhana.APP_LOAD` in `web/sadhana.mjs` are those two
addresses.

---

## It computes with real numbers

`v0.4.0` gives the language floating point: the `प्लव` type, F and D in the
machine, and — the part that is worth a section — **arithmetic that a machine this
project did not write agrees with.**

```ebnf
float_type = "प" , ( "३२" | "६४" ) ;        (* प्लव *)
```

`प६४` is a double, `प३२` a single. The letter is the initial of `प्लव`, the way
`अ` is of `अंश` and `न` of `निर्ऋण` — a type letter here is always the initial of a
chosen word with a published derivation, which is why moving the root moved the
letter (`docs/adr/0042`).

### Two machines, one answer

`oracle/float-oracle.sas` is a Sassembly program that is its own oracle. Each of
its six checks computes a result, moves the **bit pattern** into an integer
register with `प्लवसंचारः`, and branches on an integer compare. So the verdict
never rests on a float comparison — a wrong `fadd.d` cannot be hidden by an
equally wrong `feq.d`, which is exactly the shape of the one real defect the F/D
implementation had.

```sh
tools/check-float-oracle.sh
```

```console
ok  float arithmetic agrees on two machines — yantra and qemu-system-riscv64
    yantra: halt 0x5555 in 34 executed instructions
    qemu:   exit 0 (the program's own finisher write decides)
```

`1.0 + 2.0 == 3.0` exactly · `2.0 × 3.0 == 6.0` · `6.0 − 3.0 == 3.0` ·
`√4.0 == 2.0` · and `प्लवसमम्` answering १ on equal operands **and ० on unequal**,
so the fifth check cannot pass vacuously.

It needs `qemu-system-riscv64`. Without it the script exits 77 — `CANNOT RUN` —
rather than passing, because running only `yantra` would be this repository
agreeing with itself.

### The encodings are checked against GNU binutils

`spec/conformance-t0.tsv` is generated against `riscv64-elf-as`, and the generator
**returns non-zero without writing the file** if the assembler rejects a case. So
every row in it is one GNU binutils agreed to. All 62 float rows are driven
through the compiler's own encoder:

```
METRIC t1_float_oracle_cases 432   families 62   refused 0   disagreements 0
```

`flw`/`fld`/`fsw`/`fsd` at 28 cases each, and the four fused-multiply forms at 6
per width.

### There is no decimal float literal, on purpose

`३ॱ१४१५९` will be refused. Decimal-to-binary conversion in the front end would put
host-dependent rounding between a source file and the bits it denotes, and
bit-exact determinism is the property this compiler exists to have. Floats enter
as their exact IEEE octets and are reinterpreted:

```
॥ अष्टाष्टकाः ०षोड्३ऊऊ००००००००००००० ॥     ॰ 0x3FF0000000000000 — 1.0
प्लवाहारःॱप६४ प्लव०म् क्षणिक६त् ०न ।
```

Which is what a conformance check wants anyway: no rounding sits between the
source and the assertion. `oracle/float-oracle.sas` is written this way and is
worth reading as the worked example.

**Non-RNE arithmetic HALTS rather than approximating.** A rounding mode the machine
does not implement is a refusal, not a guess.


## A first look at the language

A whole routine from the compiler itself — [`ashtaka.t1`](crates/sadhana-t1/src/ashtaka.t1),
the octet arena. It pushes *n* zero octets and answers how many it wrote:

```
॰ push संख्यानम् zero octets — Vec::resize(len + n, 0): the padding of
॰ `स्थानम्` and the eight an address reserves (ADR-0013). Returns how many
॰ it wrote, so a caller that asked for none is told none, not ०-as-absent.
सार्वजनिक वृत्तिः शून्याष्टकयोजनम् आदाय संख्यानम् ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् संख्यानम् आदि
        चरः लिखितम् ॱॱ न६४ भवति अष्टकयोजनम् ० ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् संख्यानम् ।
इति
```

Every word in it:

| word | what it means |
|---|---|
| `सार्वजनिक` | public — visible outside this module |
| `वृत्तिः` | routine |
| `आदाय` | *taking* — the parameters follow |
| `ॱॱ` | the sigil that opens a **type** position |
| `न६४` | a 64-bit unsigned number |
| `ददाति` | *gives* — the return type follows |
| `आदि` … `इति` | begin … end, the only block delimiters |
| `चरः` | a local, declared with its type and an initial value |
| `भवति` | *becomes* — assignment, and the initialiser in a declaration |
| `यावत्` | while |
| `न्यूनम्` | is less than |
| `योगः` | plus |
| `प्रत्यागमनम्` | return |
| `।` | the danda, ending a statement |
| `॰` | opens a margin — a comment to end of line |

`अष्टकयोजनम् ०` is a call: one argument, written by juxtaposition, no brackets.
There is no English in any of it, and no positional convention to memorise — the
sigil says what each operand *is*.

---

## The heap already exists

The image's `.bss` declares ~537 MB, which looks like a defect and is not. Of
that, the 21 objects contribute **211,248 octets — 0.04%**. The remaining
536,870,912 is 2²⁹ exactly: `यन्त्ररचनाष्टकाः`, a **512 MiB record region**
declared at `yantrotsarjana.t1:1834` and sized in 2026-09 from a measurement (the
first whole-corpus native compile reached 353,242,600 octets of high water; 512
MiB is 1.5× that).

**The `.bss` *is* the heap.** Run growth bumps a cursor into that region with the
existing load/store primitives — "no new kind" (`ir.t1:604`). A bump allocator
needs no new linker symbols.

---

## Verification

```sh
cargo test -p sadhana-t1 -p sanskrit-text -p yantra --no-fail-fast
```

**Measured 2026-09-25: 129 targets, 1,151 passed, 0 failed, 56 ignored, exit 0.**

That figure is the three crates this release is built from. It does not include
the wider project's hourly gate, which checks an operating system this repository
is not.

`GATE_STRICT=1` is armed on that wider runner: a check that *cannot run* fails
rather than passing quietly. It is distinguished from a check that has *no
subject* — those two conditions shared an exit code until 2026-09-24, and while
they did, arming strictness would have failed every commit that touched no `.t1`
file.

---

## Reading the source

```
crates/sadhana-t1/src/   the 21 .t1 sources — this is the compiler
crates/sadhana/          the toolchain: t1_image and the drivers
crates/yantra/           the RISC-V emulator, and the guard tests
crates/sanskrit-text/    the text kernel: segmentation, normalisation, identifiers
spec/                    the tables the host fills — encodings, grammar, lexicon
crates/sadhana-wasm/     the assembler as wasm, for the browser
crates/yantra-wasm/      the machine as wasm
web/                     the hand-written glue: no wasm-bindgen, no generated bindings
tools/                   fixpoint.sh, pack-corpus.py, build-sassembly-web.sh
```

Two conventions are worth knowing before opening a file, because both are easy
to misread:

* `ॱॱ` marks a **type** position; `॰` opens a **margin** (a comment). They are
  distinct glyphs and a grep that conflates them finds the wrong lines.
* Devanagari **sandhi** fuses compounds at boundaries, so a routine's emitted
  label is often *not* the spelling in its source. `खण्डवृद्धिः` appears in no
  source file at all — it is synthesised per module by `ir.t1:645`.

---

## Two things a reader will notice

<details>
<summary><strong>Some comments point at files that are not here.</strong></summary>

<br>

Twenty-three of them reference `.loop/STATE.md`, `.loop/ASSUMPTIONS.md` or
`.loop/METRICS.tsv` — the private project's decision log, where a measurement or
a ruling was recorded. They are provenance markers, not broken code, and they are
left exactly as written for a specific reason: one of them is inside
`crates/sadhana-t1/src/encode.t1`, and **editing any `.t1` file changes the
1,399,434-octet image**. The fixpoint number above is the claim of this
repository, so the sources are published byte-for-byte as measured rather than
tidied.

The same goes for margins citing "doc 03 §6" or "doc 18 §0" — internal design
documents. Nothing in the code depends on reading them.

</details>

<details>
<summary><strong>The crate names are Sanskrit too.</strong></summary>

<br>

`sadhana` is the toolchain, `yantra` the RISC-V emulator, `sanskrit-text` the
text kernel (segmentation, normalisation, identifiers), and `sadhana-t1` holds
the 21 Sassembly sources that are the compiler.

</details>

---

## Status and stability

This is version **v0.4.0**. Nothing here is stable: not the surface syntax, not
the object format, not the tool names. The fixpoint is the result; the interfaces
around it are scaffolding for reaching it.

The compiler is two stages —
`src --मण्डलसङ्कलनम्--> asm --पाठवस्तुरचना--> object` — and feeding source to
stage 2 is an error, not a shortcut.

---

## The study group

<p align="center">
  <a href="https://discord.gg/XvYvXR8HAh"><img src="assets/btn-discord.png" alt="Join the study group on Discord · अध्ययनसङ्घः" width="344"></a>
</p>

There is a Discord for reading this compiler together —
**<https://discord.gg/XvYvXR8HAh>**.

It is for people who want to work through the sources rather than watch from
outside: how a `यदि` arm becomes a block, why `अष्टकॱमुद्रणम्` is a store and
not a call, what a kāraka sigil buys over positional operands, and how the
fixpoint is actually measured. The 21 `.t1` files are the whole compiler and
they are readable — but they are readable in Sanskrit, and reading them in
company is faster than reading them alone.

Bring a question about a specific line. That works better here than a general
one.

---

## Licence

**MIT.** See [LICENSE](LICENSE).

The licence file sits in this directory, not at the enclosing repository's root,
and that is deliberate: the wider project this compiler was extracted from is
private and not for distribution. MIT covers **what is published here** — the
Sassembly sources, the driver, the emulator and the tools needed to reproduce the
fixpoint — and nothing else.

<p align="center">
  <br>
  <a href="https://paramtatv.github.io/sassembly/">docs</a> ·
  <a href="https://discord.gg/XvYvXR8HAh">study group</a> ·
  <a href="WHY-NO-NETWORKING.md">ADR-0040</a> ·
  <a href="ANNOUNCEMENT-v0.4.0.md">v0.4.0 announcement</a> ·
  <a href="ANNOUNCEMENT-v0.2.0.md">v0.2.0 announcement</a>
  <br><br>
  <sub>सद्गुरुचरणेषु समर्पणम्</sub>
</p>
