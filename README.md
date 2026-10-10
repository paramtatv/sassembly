<p align="center">
  <img src="assets/banner.png" alt="संस्कृतयन्त्रम् · Sassembly — a grammar-based instruction set. Stage 2 == Stage 1, byte for byte." width="100%">
</p>

<p align="center">
  <a href="#status-and-stability"><img src="https://img.shields.io/badge/version-v1.0.2-A63A21?style=flat-square&labelColor=2B2521" alt="version v1.0.2"></a>
  <a href="#licence"><img src="https://img.shields.io/badge/licence-AGPL--3.0-A63A21?style=flat-square&labelColor=2B2521" alt="licence AGPL-3.0-only"></a>
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
  <a href="ANNOUNCEMENT-v1.0.2.md">v1.0.2 announcement</a> ·
  <a href="ANNOUNCEMENT-v1.0.1.md">v1.0.1 announcement</a> ·
  <a href="ANNOUNCEMENT-v1.0.0.md">v1.0.0 announcement</a> ·
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
> This release completes the language and its self-hosting compiler. It is still
> a **research artifact**, not an application toolchain — see
> [What this cannot do](#what-this-cannot-do), which is deliberately placed
> before the tutorial.

## Install

Prebuilt binaries, **no Rust needed**. Each tarball holds `sadhana` (the assembler) and `yantra-run` (the RV64 machine). They are attached to the
[v1.0.2 release](https://github.com/paramtatv/sassembly/releases/tag/v1.0.2).

| OS | archive | sha256 |
|---|---|---|
| Linux x86-64 | [`sassembly-v1.0.2-linux-x86_64.tar.gz`](https://github.com/paramtatv/sassembly/releases/download/v1.0.2/sassembly-v1.0.2-linux-x86_64.tar.gz) | `00730e0894e82dff9b249c5cc8c14f365375da3cf11061cbcfa7a88eb1bc2251` |
| Linux aarch64 | [`sassembly-v1.0.2-linux-aarch64.tar.gz`](https://github.com/paramtatv/sassembly/releases/download/v1.0.2/sassembly-v1.0.2-linux-aarch64.tar.gz) | `746e8a35bbdde7e3bd450c2fe8ce90118addc9acb4a12f899e81ccdd0342a8c9` |
| macOS arm64 (Apple silicon) | [`sassembly-v1.0.2-macos-arm64.tar.gz`](https://github.com/paramtatv/sassembly/releases/download/v1.0.2/sassembly-v1.0.2-macos-arm64.tar.gz) | `7cba1635d53a9424b595954b41c0ad913dc3b18f0dca039e52858c968df533c2` |
| macOS x86-64 (Intel) | [`sassembly-v1.0.2-macos-x86_64.tar.gz`](https://github.com/paramtatv/sassembly/releases/download/v1.0.2/sassembly-v1.0.2-macos-x86_64.tar.gz) | `0adb68435e52897fe78bd221fa74ca529dbac3292938ec80c0bcd543adc92476` |
| Windows arm64 | [`sassembly-v1.0.2-windows-arm64.zip`](https://github.com/paramtatv/sassembly/releases/download/v1.0.2/sassembly-v1.0.2-windows-arm64.zip) | `a286984305059e8f3e0eb5de4917c1a270a86d5e6a94e1c1f19963063d3875d4` |
| Windows x86-64 | [`sassembly-v1.0.2-windows-x86_64.zip`](https://github.com/paramtatv/sassembly/releases/download/v1.0.2/sassembly-v1.0.2-windows-x86_64.zip) | `d974cd3b6212b3017146cda4876e55cef3d5c2af6780c29d3a09c1f77a8ee5ac` |

The sha256s are those in the release's `SHA256SUMS-v1.0.2`, which also covers `sassembly-v1.0.2-stage1.elf` (the v1.0.2 compiler image, sha256 `4e7a9a24…070ca`). The `linux-aarch64` build was built and smoke-tested on aarch64 hardware. Replace `OS_ARCH` below with `linux-x86_64`, `linux-aarch64`, `macos-arm64` or `macos-x86_64`.

```sh
V=v1.0.2; T=sassembly-$V-OS_ARCH.tar.gz
gh release download $V -R paramtatv/sassembly -p "$T" -p SHA256SUMS-$V -p sassembly-$V-stage1.elf
# verify (Linux: sha256sum; macOS: shasum -a 256)
grep " $T\$" SHA256SUMS-$V | sha256sum -c -        # macOS: ... | shasum -a 256 -c -
grep stage1 SHA256SUMS-$V | sha256sum -c -
tar xzf $T && mkdir -p ~/.local/bin && cp sassembly-$V-OS_ARCH/{sadhana,yantra-run} ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"      # add to ~/.profile or ~/.zshrc
# macOS only, if Gatekeeper blocks the binaries (downloaded via a browser):
xattr -d com.apple.quarantine ~/.local/bin/sadhana ~/.local/bin/yantra-run
```

**Windows (PowerShell).** Use `arm64` or `x86_64` (the x86-64 build also runs on Windows on ARM under x64 emulation).

```powershell
$V='v1.0.2'; $A='x86_64'          # or arm64
$T="sassembly-$V-windows-$A.zip"; $R="https://github.com/paramtatv/sassembly/releases/download/$V"
curl.exe -sSLO "$R/$T"; curl.exe -sSLO "$R/SHA256SUMS-v1.0.2"
(Get-FileHash $T).Hash.ToLower()  # compare with the line for $T in SHA256SUMS-v1.0.2
Expand-Archive $T .; $B="$PWD\sassembly-$V-windows-$A"; $env:Path="$B;$env:Path"
chcp 65001                        # UTF-8 console, for Devanagari output
sadhana.exe namaste.sas n.elf; yantra-run.exe n.elf
```

In PowerShell, redirect binary output with `cmd /c "... > file"`: PowerShell's own `>` re-encodes and corrupts it.
To build from source on Windows, clone with `git clone -c core.autocrlf=false`; CRLF line endings break the `.sas` sources.

Then run a `.sas` program with `sadhana prog.sas prog.elf && yantra-run prog.elf`. A `.t1` program is compiled by `sassembly-v1.0.2-stage1.elf`: the next section shows both, step by step.

`yantra-run` exits non-zero whenever the program's halt status is non-zero; that is a report, not a failure of the tool.

**Files (v1.0.1).** `yantra-run --files DIR prog.elf [args...]` grants the program a file root: its file window reads and writes inside `DIR` and nowhere else (`..`, absolute paths and symlinks leaving `DIR` are refused). `--files` must be the first argument. Without it every file request is refused by name. `DIR` must not be writable by an untrusted party while the program runs. See [ANNOUNCEMENT-v1.0.1.md](ANNOUNCEMENT-v1.0.1.md) and [LIMITS.md](LIMITS.md).

**New in v1.0.2.** (1) The prebuilt compiler takes its entry from the input: `YANTRA_INPUT_ENTRY="<module> <routine>"` replaces `YANTRA_INPUT_NAME`, so any module and routine builds. (2) `yantra-wasm` can serve a program's file window from an in-memory root (`memfsEnable`, `memfsPut`, `memfsFiles` in `web/yantra.mjs`). (3) The licence is AGPL-3.0-only, with a commercial licence available ([COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md)); v0.2 to v1.0.1 stay MIT. See [ANNOUNCEMENT-v1.0.2.md](ANNOUNCEMENT-v1.0.2.md).

---

## Write your first program in Sanskrit

Sassembly has two layers. **`.sas`** is the assembler form (T0): one instruction per line, operands marked by kāraka sigils. **`.t1`** is the systems language (T1): routines, loops, arrays, modules. Everything below was run on **v1.0.0**; the outputs are real.

Two facts first.

* **The prebuilt `sadhana` assembles `.sas` only.** A `.t1` is compiled by `sassembly-v1.0.2-stage1.elf`, the self-hosted compiler, run on `yantra-run`. That image takes its entry from the input (v1.0.2): give `YANTRA_INPUT_ENTRY="<module> <routine>"` and any module and routine name builds; without it the compiler keeps its own entry, module `शृङ्खला` with routine `स्वपरीक्षास्वप्रतिबिम्बम्`, so every example below still works unchanged.
* **`yantra-run` exits 1 whenever the program's halt status is not 0.** That is a report, not a tool failure: compiling with `stage1.elf` ends with status `1200` (BUILT), so it always exits 1; your own program returns the status you give it.

### 0. Get the tools

**Release assets (no Rust).** From <https://github.com/paramtatv/sassembly/releases/tag/v1.0.2> download `sassembly-v1.0.2-stage1.elf`, `SHA256SUMS-v1.0.2`, and the tarball for your machine (`sassembly-v1.0.2-linux-x86_64.tar.gz`, `-linux-aarch64`, or `-macos-arm64`). Check and unpack:

```console
$ grep -F sassembly-v1.0.2-stage1.elf SHA256SUMS-v1.0.2 | sha256sum -c -
sassembly-v1.0.2-stage1.elf: OK
$ grep -F sassembly-v1.0.2-linux-x86_64.tar.gz SHA256SUMS-v1.0.2 | sha256sum -c -
sassembly-v1.0.2-linux-x86_64.tar.gz: OK
```

(macOS: `shasum -a 256 -c -` in place of `sha256sum -c -`.) The tarball holds `sadhana` and `yantra-run`.

**From source.** `git clone https://github.com/paramtatv/sassembly && cd sassembly && git checkout v1.0.2 && cargo build --release -p sadhana -p yantra`. The binaries are `target/release/sadhana`, `target/release/yantra-run` and `target/release/t1_image`.

### 1. The smallest program, in `.t1`

Save as `शृङ्खला.t1`. It prints a message one octet at a time and returns 0.

```
मण्डलम् शृङ्खला ॥
आयातः अष्टक ।

॰ Prints a message one octet at a time, then returns 0.  The routine's name is fixed by stage1.elf.
सार्वजनिक वृत्तिः स्वपरीक्षास्वप्रतिबिम्बम् ददाति न६४ आदि
    चरः सन्देश ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् नमस्ते संसार इति ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् सन्देश ॱ दैर्घ्य आदि
        चरः ग ॱॱ न६४ भवति अष्टकॱमुद्रणम् सन्देश अङ्कः क्रमः अन्तः ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    चरः घ ॱॱ न६४ भवति अष्टकॱमुद्रणम् १० ।
    प्रत्यागमनम् ० ।
इति
```

Compile and run with the release assets (from the unpacked tarball directory, with `sassembly-v1.0.2-stage1.elf` copied beside the binaries):

```console
$ { printf "शृङ्खला\0"; cat शृङ्खला.t1; printf "\0"; } > p.blob
$ YANTRA_INPUT=p.blob YANTRA_INPUT_NAME=x YANTRA_RAM=2684354560 YANTRA_STEPS=4000000000000 ./yantra-run sassembly-v1.0.2-stage1.elf > sink
[exit status 1]
halt: Finisher { status: Some(1200) }
$ n=$(wc -c < sink); tail -c +2 sink | dd bs=1 count=$((n-2)) of=prog.elf
$ ./yantra-run prog.elf
halt: Finisher { status: Some(0) }
नमस्ते संसार
[exit status 0]
```

`1200` means built. The ELF is the `sink` file minus one octet at each end; that is what the `dd` line cuts out.

### 2. A loop and an array, in `.t1`

Save as `शृङ्खला.t1` (a new directory). It puts the squares of 1 to 8 into a growing array, then walks the array, printing and summing.

```
मण्डलम् शृङ्खला ॥
आयातः अष्टक ।

॰ Prints a number in decimal: the leading digits first (recursion), then the last one.
वृत्तिः अङ्कमुद्रणम् आदाय मान ॱॱ न६४ ददाति न६४ आदि
    यदि मान अधिकम् ९ आदि
        चरः अग्रिम ॱॱ न६४ भवति मान विभाजनम् १० ।
        चरः क ॱॱ न६४ भवति अङ्कमुद्रणम् अग्रिम ।
    इति
    चरः अन्तिम ॱॱ न६४ भवति मान शेषः १० ।
    चरः कोड ॱॱ न६४ भवति ४८ योगः अन्तिम ।
    चरः ख ॱॱ न६४ भवति अष्टकॱमुद्रणम् कोड ।
    प्रत्यागमनम् ० ।
इति

॰ Squares of 1..8 go into a growing array; then the array is walked and summed.
सार्वजनिक वृत्तिः स्वपरीक्षास्वप्रतिबिम्बम् ददाति न६४ आदि
    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति १ ।
    यावत् क्रमः न्यूनम् ९ आदि
        चरः वर्ग ॱॱ न६४ भवति क्रमः गुणनम् क्रमः ।
        सूची अङ्कः सूची ॱ दैर्घ्य अन्तः भवति वर्ग ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    चरः सञ्चयः ॱॱ न६४ भवति ० ।
    चरः सूचक ॱॱ न६४ भवति ० ।
    यावत् सूचक न्यूनम् सूची ॱ दैर्घ्य आदि
        चरः मान ॱॱ न६४ भवति सूची अङ्कः सूचक अन्तः ।
        चरः ख ॱॱ न६४ भवति अङ्कमुद्रणम् मान ।
        चरः विराम ॱॱ न६४ भवति अष्टकॱमुद्रणम् ३२ ।
        सञ्चयः भवति सञ्चयः योगः मान ।
        सूचक भवति सूचक योगः १ ।
    इति
    चरः ग ॱॱ न६४ भवति अष्टकॱमुद्रणम् ६१ ।
    चरः घ ॱॱ न६४ भवति अष्टकॱमुद्रणम् ३२ ।
    चरः ङ ॱॱ न६४ भवति अङ्कमुद्रणम् सञ्चयः ।
    चरः च ॱॱ न६४ भवति अष्टकॱमुद्रणम् १० ।
    प्रत्यागमनम् ० ।
इति
```

```console
$ { printf "शृङ्खला\0"; cat शृङ्खला.t1; printf "\0"; } > p.blob
$ YANTRA_INPUT=p.blob YANTRA_INPUT_NAME=x YANTRA_RAM=2684354560 YANTRA_STEPS=4000000000000 ./yantra-run sassembly-v1.0.2-stage1.elf > sink
[exit status 1]
halt: Finisher { status: Some(1200) }
$ n=$(wc -c < sink); tail -c +2 sink | dd bs=1 count=$((n-2)) of=prog.elf
$ ./yantra-run prog.elf
halt: Finisher { status: Some(0) }
1 4 9 16 25 36 49 64 = 204
[exit status 0]
```

### 3. The same idea in `.sas`

`.sas` is assembled by `sadhana`, no compiler image needed. Save as `namaste.sas` (it is `spec/namaste.sas` in the repository). It loops over a string of octets and writes each to the console.

```
॰ नमस्ते — पहला देवनागरी कार्यक्रम जो धातु पर बोलता है
॰
॰ सन्देश दत्त-कोष्ठक में है; यह उसे एक-एक अष्टक करके UART को लिखता है।
॰ QEMU virt का UART ०x10000000 पर, समापक ०x100000 पर।

॰ UART का पता
उपरिभारः क्षणिक०म् ०षोड्१००००न ।

॰ सन्देश का पता, स्थान-सापेक्ष
स्थानसापेक्षयोगः क्षणिक१म् सन्देशःॱउपरिन ।
योगः क्षणिक१म् क्षणिक१न सन्देशःॱअधःन ।

मुद्रणम्ॱॱ
आहारःॱअ८ क्षणिक२म् क्षणिक१त् ०न ।
समलङ्घनम् क्षणिक२न शून्यःत् समाप्तिःय् ।
निधानम्ॱअ८ क्षणिक०य् ०न क्षणिक२न ।
योगः क्षणिक१म् क्षणिक१न १न ।
लङ्घनम् शून्यःम् मुद्रणम्य् ।

समाप्तिःॱॱ
॰ समापक को ०x5555 — निकास ०
उपरिभारः क्षणिक४म् ०षोड्१००न ।
उपरिभारः क्षणिक५म् ०षोड्५न ।
योगः क्षणिक५म् क्षणिक५न ०षोड्५५५न ।
निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।

चक्रःॱॱ
लङ्घनम् शून्यःम् चक्रःय् ।

॥ कोष्ठकम् ॱदत्त ॥
सन्देशःॱॱ
॥ अष्टकाः उक्तम् नमस्ते संसार इति ॥
॥ अष्टकाः १० ० ॥
```

```console
$ ./sadhana namaste.sas namaste.elf
namaste.elf: 1 file(s) assembled, entry 0x80000000
$ ./yantra-run namaste.elf
नमस्ते संसार
halt: Finisher { value: 21845, status: Some(0) }
steps: 184 executed instructions
```

### 4. From source

With the cargo-built binaries the commands above work unchanged (`target/release/yantra-run` in place of `./yantra-run`). `t1_image` is the faster route when you build from source, because the entry is yours to name. Save this as `नमस्कारः.t1`:

```
मण्डलम् नमस्कारः ॥
आयातः अष्टक ।

॰ Prints a message one octet at a time, then returns 0.  Here the entry is the one you name to t1_image.
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः सन्देश ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् नमस्ते संसार इति ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् सन्देश ॱ दैर्घ्य आदि
        चरः ग ॱॱ न६४ भवति अष्टकॱमुद्रणम् सन्देश अङ्कः क्रमः अन्तः ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    चरः घ ॱॱ न६४ भवति अष्टकॱमुद्रणम् १० ।
    प्रत्यागमनम् ० ।
इति
```

```console
$ target/release/sadhana spec/namaste.sas namaste.elf
namaste.elf: 1 file(s) assembled, entry 0x80000000
$ target/release/yantra-run namaste.elf
नमस्ते संसार
halt: Finisher { value: 21845, status: Some(0) }
steps: 184 executed instructions
$ target/release/t1_image --compiler crates/sadhana-t1/src --load नमस्कारः.t1 --entry नमस्कारः मुख्यम् -o prog.elf नमस्कारः.t1
[exit status 0]
build:    0 source(s) failed to compile, 2 object(s) linked (startup included)
steps:    36157967
predict:  finished, status 0; 35 octet(s) printed
write:    634 octets of ELF -> prog.elf
$ target/release/yantra-run prog.elf
halt: Finisher { status: Some(0) }
नमस्ते संसार
[exit status 0]
```

The build also interprets your program and compares it with the native run (`predict: finished, status 0`), and refuses the image if they disagree.

### Where next

The larger worked examples (audio, image, protein, video) are at <https://paramtatv.github.io/sassembly/learn.html>.

---

### At a glance

| | |
|---|---|
| **Stage 2 == Stage 1** | byte-identical, `923,042` octets |
| **the compiler** | 21 `.t1` sources, 46,794 lines, written in Sassembly |
| **target** | bare-metal RISC-V RV64, no LLVM, no external toolchain |
| **in a browser** | [playground](https://paramtatv.github.io/sassembly/playground.html) — about 612 KB of wasm, no server |
| **measured** | 2026-10-10, from this repository |
| **status** | v1.0.2 — the language and its compiler are complete; v1.0.2 lifts the fixed entry of the prebuilt compiler, adds an in-memory file root for the browser, and is AGPL-3.0-only |

### Contents

0. [Install](#install) · [Write your first program in Sanskrit](#write-your-first-program-in-sanskrit)
1. [The claim, and how to check it](#the-claim-and-how-to-check-it)
2. [What is new since v0.4.0](#what-is-new-since-v040)
3. [What this cannot do](#what-this-cannot-do) · [LIMITS.md](LIMITS.md), the verified limits with a command for each
4. [What a program can do today](#what-a-program-can-do-today)
5. [It runs in a browser](#it-runs-in-a-browser)
6. [A first look at the language](#a-first-look-at-the-language)
7. [The heap already exists](#the-heap-already-exists)
8. [Verification](#verification)
9. [Reading the source](#reading-the-source)
10. [Two things a reader will notice](#two-things-a-reader-will-notice)
11. [Status and stability](#status-and-stability)
12. [The study group](#the-study-group)
13. [Licence](#licence)
14. [Build from source (Rust)](#build-from-source-rust) · [Networking](#networking)

---

## The claim, and how to check it

The interesting property of a self-hosting compiler is a fixpoint: if you
compile the compiler with itself, you should get the compiler back. Not an
equivalent binary — *the same bytes*.

```
Stage 1   the compiler's 21 sources, compiled by the interpreted compiler
Stage 2   Stage 1 running natively on RISC-V, compiling those same 21 sources
```

**Measured 2026-10-10, `tools/fixpoint.sh`, in a copy of THIS repository's
tree** — not inherited from the tree it was extracted from:

```console
fixpoint: packing the corpus from crates/sadhana-t1/src
packed 21 source(s), 5043687 octets
fixpoint: Stage 1  923042 octets
fixpoint: Stage 2  923042 octets
  status:  1200 — BUILT (shrinkhala.t1:3551)
FIXPOINT HOLDS: 923042 octets, byte-identical
```

Stage 2 ran
**55,485,618,435** executed instructions to a finisher with status 1200, with a
high water of 1,051,408,576 octets of the 2,684,354,560 the run is given. The
sha256 of `stage1.elf` is
`4e7a9a244e95bdbf759ba4d373e7efc2e6dcd0e383b9123ac63b5a37d20070ca`.

Stage 1's own controls — `build: 0 source(s) failed to compile, 1 declared
nothing, 21 object(s) linked`, `stubs: 0`, and **`steps: 22231632704`** — are the
interpreted compiler's instruction count for the whole build. It moves if any
byte of any source or spec table differs.

| quantity | value |
|---|---|
| Stage 2 == Stage 1 | **byte-identical** |
| image size | **923,042 octets** (v0.4.0: 1,399,434) |
| sources | **21** `.t1` files, 46,794 lines |

Reproduce it (needs the Rust build in [Build from source](#build-from-source-rust)):

```sh
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
> That is a different artifact from the fixpoint image (1,399,434 octets in v0.4.0, 923,042 now), built on
> a different date. **Neither number is a typo for the other.**

---

## What is new since v0.4.0

`v1.0.0` completes the language. For a reader new to the project, the short
version is that a Sassembly program can now do arithmetic safely, use vectors,
read and write files (from a host that grants a root directory; see the note below), and talk over a socket, and the compiler that makes all of
that possible still compiles itself to the same bytes.

* **Vectors and matrices.** The RISC-V V extension runs in `yantra`, the compiler
  lowers vector operations to it, and there is matrix and tensor syntax. The
  vector words are call forms, not keywords (ADR-0043), so no existing program
  changed meaning.
* **One integer meaning on every engine.** The interpreter and the native engines
  agree on integer semantics. *Checked* operators (`अष्टकॱसुरक्षितयोगः` and its
  siblings) halt with a named refusal instead of an opaque fault: `0x355`
  out-of-bounds read, `0x35b` aliasing, `0x35c` overflow, `0x35d` out-of-bounds
  write, `0x35e` division by zero. Unsigned runs (`न३२`) load zero-extended.
* **Devanagari-8 (ADR-0044).** A data literal (`वर्णाष्टकम्`) stores one octet per
  letter: 35% smaller on the measured data.
* **Symbol lookup.** The linker keeps a hashed name table.
* **The same answer everywhere.** One image executes the same instruction count on
  x86-64, on aarch64 and in the browser through `yantra-wasm`.
* **Devices.** On `yantra`: a file window (`पत्रम्`) that reads and writes **only when the host grants a root directory** —
  `yantra-run --files DIR` grants one (v1.0.1; v1.0.0's release `yantra-run` granted none and refused every file request),
  command-line arguments, a clock and a socket delivered at waits, cooperative
  threads, and virtio-gpu 2D. The fixpoint script refuses any compiler image that touches the
  socket, reads the retired-instruction counter or declares threads, so none of
  them can make the fixpoint a statement about its environment.
* **A smaller image.** The fixpoint image is **923,042 octets**, 34.1% smaller than
  v0.4.0's 1,399,434.

Not in this release: compiling Sanskrit to web pages, and a GPU compute path
(ADR-0045 is a design only). See the [v1.0.0 announcement](ANNOUNCEMENT-v1.0.0.md).

---

## What this cannot do

Stated first, and in full, because a self-hosting compiler invites the
assumption that a general-purpose toolchain comes with it. It does not.

Every row of the longer list in [LIMITS.md](LIMITS.md) was checked against the v1.0.0 release binaries or a named test (the file rows have been re-checked on v1.0.1), and carries the command to repeat the check.

| capability | available today | shown by |
|---|---|---|
| **open and read a named file** | **works with `--files DIR` (v1.0.1)**: `yantra-run --files DIR prog.elf`. Without the flag every file request is refused by name. A path that escapes DIR (`..`, absolute, drive-letter, symlink out) is refused. Trust model: DIR must not be writable by an untrusted party while the program runs | `t1_files_flag.rs`: `files_grants_the_root_and_the_program_reads_inside_it`, `without_the_flag_the_same_program_is_refused_by_name`; `t1_file_window.rs`: `the_window_refuses_a_path_that_escapes_its_root_and_says_so` |
| **write a file** | **works with `--files DIR` (v1.0.1)**, same window (`PATRA_PUT`); it does not follow a symlink at the file itself (unix) | `t1_files_flag.rs`: `files_writes_inside_the_root_and_reads_it_back`, `a_write_through_a_symlink_in_the_root_leaves_the_outside_file_alone`, `without_the_flag_a_write_creates_nothing` |
| **command-line arguments** | yes, on `yantra-run`; an image that does not declare the argument globals is refused | `t1_arguments.rs`: `a_program_reads_the_arguments_it_was_given`, `the_binary_hands_a_program_its_command_line` |
| **clock** | yes, but only as a value delivered at a wait and recorded in an event log, so a run can be replayed exactly | `w375_clock.rs` |
| **threads** | cooperative only: the host switches threads at waits and never preempts; the schedule is in the event log | `w376_threads.rs` |
| **sockets** | one host socket, served at waits; live it serves one client, and the log replays without a network | `w377_sockets.rs` |
| **TLS, DNS, HTTP** | no | — |
| **standard library** | `lib.t1` is fifteen lines of comments and declares no module, so nothing can import it | — |

These are `yantra` features: the device windows are part of the emulator, and
the same program on bare metal has none of them. What the clock, thread and socket windows do, and do not do, is set out in
[NETWORKING.md](NETWORKING.md). The earlier question of how a program that
cannot wait could use a network is kept as history in
[WHY-NO-NETWORKING.md](WHY-NO-NETWORKING.md) (ADR-0040).

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
| return a result | the finisher status — **48 bits** (the finisher word above its low 16 bits) | a run: status `281474976710652` = 2^48 − 4 |

Two footnotes that will otherwise cost you an afternoon:

> [!CAUTION]
> **The status is 48 bits.** It is the finisher word shifted right by 16, so an answer up to
> 2^48 − 1 comes back exactly; the low 16 bits of the word are the pass/fail code (`0x5555`, `0x3333`).

> [!NOTE]
> **"Reading input" is not a file API.** The host writes the file's octets *into
> RAM before the program starts*, locating the slots by scanning memory for a
> magic word. There is no port and no syscall. That is why it costs nothing, and
> also why it does not generalise: naming a file requires the program to ask the
> host something *while running*.

---

## It runs in a browser

New in v0.3.0, and the shortest way to see the thing work:

**<https://paramtatv.github.io/sassembly/playground.html>** — type Devanagari
assembly, press चालय, and the page assembles it and executes it in your tab.

Two crates compiled to wasm do the whole of it:

| | | |
|---|---|---|
| `crates/sadhana-wasm` | **420 KB** (430,275 bytes) | Devanagari assembly → ELF |
| `crates/yantra-wasm` | **192 KB** (196,737 bytes) | an RV64 machine that runs the ELF |

Both are instantiated with an **empty import object**. That is the claim rather
than an omission: the page grants them no syscalls, no clock and no network,
because there is nothing for them to ask for.

Build the standalone page yourself, as one self-contained file with no server: see [Build from source](#build-from-source-rust).

**Measured 2026-10-07 with `tools/build-sassembly-web.sh` on a Linux x86-64
host, from this repository's tree:**

```
  namaste      proof 904 bytes  sha256 4032492520bbed48
  bare-metal   proof 912 bytes  sha256 7b0e6802889d7b01
  atithi       app   776 bytes  sha256 445524aa772c833b
  wasm         196737 bytes
  sadhana-wasm 430275 bytes
wrote sassembly.html (1422025 bytes) — open it directly, no server needed
```

The sizes depend on the Rust toolchain that built them (this one was a nightly),
so expect them to differ by a few percent elsewhere.

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

`v0.4.0` gave the language floating point: the `प्लव` type, F and D in the
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

## Networking

A program can serve one TCP client on the loopback interface under `yantra-run`, with every outside event recorded in a log that replays exactly. There is no TCP/IP stack, no TLS, DNS or HTTP. [NETWORKING.md](NETWORKING.md) states what exists, what does not, and a verified echo example under [`examples/networking/`](examples/networking/).

---

## Verification

```sh
cargo test --workspace --release --no-fail-fast
```

**Measured 2026-10-10, on a Linux x86-64 host: 2,475 passed,
0 failed, 127 ignored.**

54 of the ignored tests are marked `#[ignore = "census: needs ... not in the public repository"]`.
Each measures the *whole development repository* and so cannot run on this one:
it reads `research/` (the Unicode data files and design notes), `docs/adr`,
`tests/corpus/`, `tests/levels/`, `fuzz/corpus/`, `BACKLOG.tsv`, a tree-sitter
grammar crate, or counts every `.t1`/`.sas` file in a repository that has more of
them than this one. None of those ship here. None of them tests the compiler, the
machine or the fixpoint. The other 73 ignored tests are slow, host-specific or probes (5 are new in v1.0.2: `t1_entry_from_input` needs a Stage 1 image built from the tree, `SAS_STAGE1_ELF=<path>`). `cargo test -- --ignored` runs them and shows each reason; in this
repository the 54 will fail because the files are absent. Earlier releases left
failures here: v0.3.0 and v0.4.0 left 61, v1.0.0 left 56 (v1.0.0: 2,431 passed).

Two test groups from earlier releases are not in this repository at all because
they depend on code that does not ship here: a test comparing two renderer types,
two text-kernel timing tests, and the tests of an archive format owned by another
project.

The wider project this was extracted from also runs a stricter gate, with
`GATE_STRICT=1` armed: a check that *cannot run* fails
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

A few comments reference `.loop/STATE.md`, `.loop/ASSUMPTIONS.md` or
`.loop/METRICS.tsv` — the private project's decision log, where a measurement or
a ruling was recorded. They are provenance markers, not broken code.

The same goes for margins citing "doc 03 §6" or "doc 18 §0" — internal design
documents — and for row ids such as `W-381`. Nothing in the code depends on
reading them.

Some margins in the `.t1` sources named hosts, sessions and sibling projects;
those names were reworded for this release. A margin is not code, and
the fixpoint figures above were measured on the sources exactly as published
here.

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

This is version **v1.0.2** (v1.0.1 plus the entry-from-input compiler, the browser in-memory file root, and the AGPL-3.0-only licence): the language and its compiler are complete. Interfaces around them can still change. Outside the compiler, nothing here is stable: not the tool names, not
the object format. The fixpoint is the result; the interfaces
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

## Build from source (Rust)

The prebuilt binaries above are enough to assemble, compile and run programs. To build the toolchain yourself, or to re-check the fixpoint, you need a Rust toolchain:

```sh
cargo build --release -p sadhana -p yantra   # builds sadhana, t1_image and yantra-run
tools/fixpoint.sh                            # Stage 1, Stage 2, and the byte comparison
```

For the browser build of the assembler and the machine:

```sh
rustup target add wasm32-unknown-unknown
tools/build-sassembly-web.sh            # writes to a temp dir; pass a path to choose
```

Once built, the Sanskrit programs in the quickstart run unchanged with `target/release/sadhana`, `target/release/yantra-run` and `target/release/t1_image`; `t1_image` lets you name your own module and entry routine. The test suite is `cargo test --workspace --release --no-fail-fast`; see [Verification](#verification) for what it measured.

---

## Licence

**AGPL-3.0-only from v1.0.2.** See [LICENSE](LICENSE). Copyright (c) 2026 परमतत्व.
A commercial licence is available from the copyright holder: see
[COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md) (paramtatv@fastbuilder.ai).
Third-party files keep their own licences, listed in [NOTICE](NOTICE).

**Releases v0.2 to v1.0.1 remain MIT**, as published; their tags and release
assets are unchanged.

The licence file sits in this directory, not at the enclosing repository's root,
and that is deliberate: the wider project this compiler was extracted from is
private and not for distribution. The licence covers **what is published here** —
the Sassembly sources, the driver, the emulator and the tools needed to reproduce
the fixpoint — and nothing else.

<p align="center">
  <br>
  <a href="https://paramtatv.github.io/sassembly/">docs</a> ·
  <a href="https://discord.gg/XvYvXR8HAh">study group</a> ·
  <a href="NETWORKING.md">networking</a> ·
  <a href="WHY-NO-NETWORKING.md">ADR-0040</a> ·
  <a href="ANNOUNCEMENT-v1.0.0.md">v1.0.0 announcement</a> ·
  <a href="ANNOUNCEMENT-v0.4.0.md">v0.4.0 announcement</a> ·
  <a href="ANNOUNCEMENT-v0.2.0.md">v0.2.0 announcement</a>
  <br><br>
  <sub>सद्गुरुचरणेषु समर्पणम्</sub>
</p>
