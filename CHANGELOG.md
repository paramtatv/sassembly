# Changelog

Earlier releases are described in their `ANNOUNCEMENT-*.md` files. This file starts at v1.1.0.

## v1.1.0

### Native packages: no Rust, no emulator
- **Four native packages:** `x86_64-linux`, `aarch64-linux`, `aarch64-macos` and `x86_64-macos`.
  - Each holds the release's Stage 1 compiler and the RV64-to-native translator, both translated ahead of time for that target.
  - It also holds `bin/sassembly-build`, `bin/sassembly-run`, an example, `LICENSE.md`, `NOTICE`, `COMMERCIAL-LICENSE.md`, and a `SHA256SUMS` over every file.
  - A package needs only POSIX `sh`, `awk`, `od`, `grep` and `sha256sum` or `shasum`.
- **Smoke test.** Each package was smoke-tested on its own target, from the unpacked tarball, in an empty environment with no checkout:
  - it builds `example/namaste.t1` with the default entry and with a non-default one, and runs both;
  - it checks that a yantra-only feature is refused by name.
- **The packages are `.t1` only.** Assembling T0 `.sas` text still needs the Rust `sadhana` from the v1.1.0-alpha or v1.0.2 release archives.
- **No Windows package.** The translator does not emit a Windows target yet.
- **New tools in `tools/`:**
  - `t1-build.sh` compiles `.t1` with the native Stage 1;
  - `t1-run.sh` translates an RV64 image for the host and runs it;
  - `t1-native-lib.sh` holds the shared routes and the content-keyed cache;
  - `release-native.sh` (with `release-native/`) builds and smoke-tests a package. For a published release, set `RELEASE_STAGE1_NAME=sassembly-<tag>-stage1.elf`;
  - `native-spike/t1/anuvada.t1` is the translator, written in `.t1`, and `native-spike/anuvada.py` is its byte-identical oracle;
  - `compiler-key-spec.txt`, `compiler-image.sha256` and `with-timeout.pl` support them.
- **Self-check suite.** `tools/selfcheck/` is a native `.t1` suite: 80 programs and 23 build refusals, covering integers, floats, strings, arrays, arenas and fork-join. It has a registered budget of 10 s or less per host (`BUDGET.md`).

### Compiler
- **The compiler image is new:** `sassembly-v1.1.0-stage1.elf`, 951,146 octets, sha256 `e0adbb0ff066d6bc2c024fd647c5e3b6c888c94068c58d5921e8ce8a89cd46c6` (v1.0.2: 923,042 octets). Compiled natively by itself from this repository's sources, it reproduces itself byte for byte.
- **Register promotion** (ADR-0049). Frame slots are promoted to registers, and call-free temporaries use `a0`–`a7`. It is on by default.
- **Fork-join.** The call-form member `समान्तरॱचालनम्` is a deterministic parallel-for over an output run. Today it runs serially.
  - **Refused when the region body runs:**
    - a write beyond the body's own element (0x372);
    - a read of another index's element (0x373);
    - an allocation or run growth (0x374);
    - a nested region (0x375);
    - a file or device access (0x376);
    - a read of the output run under another name (0x380).
  - **Refused at build:**
    - the output run used other than as an index base (0x377);
    - a body routine called outside its region (0x37F);
    - an impure routine reachable from a body (0x381);
    - a routine declared under one of the names the build synthesises (0x382).
- **Arenas** (memory freeing, stage 1). Six call-form members of `अष्टक` are lowered inline:
  - the members are `वाटनिर्माणम्`, `वाटवितरणम्`, `वाटपठनम्`, `वाटलेखनम्`, `वाटप्रत्यावर्तनम्` and `वाटविसर्जनम्`;
  - every access is checked for generation and object bounds;
  - refusals 0x378–0x37E: use after reset, use after destroy, a stale or foreign handle, arena full, object bounds, slots exhausted, and generation exhausted.
- **Fix:** a nested call that the `.t1` parser reads by juxtaposition is now bound as the parser reads it.
- **Lexicon:** `spec/lexicon.tsv` and `spec/lexicon.src.tsv` gain the fork-join and arena rows above. `lexicon.tsv` also gains the 0x371 row, which v1.1.0-alpha's `.src` already had.

### Libraries
- **`spec/entropy`:** host entropy for a program, as a pool file `.entropy` in the granted file root. The pool is read through the file window and drawn with a cursor, so no draw repeats.
  - A missing pool is refused as 0x3d2, and a short one as 0x3d3; the library never returns zeros.
  - `tools/yantra-entropy.sh` writes a fresh pool for each run, and can record or replay one.
  - `tools/check-native-entropy.sh` checks the library.

### Known limits
- **Threads run under `yantra-run` only.** The native runtime refuses a threaded image by name when it starts it, exit 1. In a checkout, `tools/t1-run.sh` reruns the image under `yantra-run` instead.
- **Native runs refuse, by name:**
  - vector instructions;
  - CSR calls and MMIO loads, including the instruction counter;
  - replaying an event log, and a wait with no event source;
  - `YANTRA_STEPS`-style variables;
  - every `yantra-run` flag except `--files`. `tools/t1-run.sh --files DIR` and `YANTRA_FILES=DIR` grant a file root natively.
- **What runs natively:** scalar floats (F/D), file read and write, a recorded clock (`YANTRA_RECORD_EVENTS`) and the entropy library.
- **`YANTRA_RAM`:** a value too small for the image, or one that cannot be mapped, is refused by name and never crashes. macOS maps very large sizes that Linux refuses.
- **Package archives are not bit-reproducible.** `README-BIN.txt` records the build time. The Stage 1 and the translator inside each package are covered by its `SHA256SUMS`.
- [LIMITS.md](LIMITS.md) is rewritten for v1.1.0 with verified rows only. Each row is checked on all four native targets and under `yantra-run`.

### Not in this release
- **Networking is unchanged since v1.1.0-alpha.** `spec/net` ships exactly as in the alpha. Later network and crypto work is not included:
  - TCP, HTTP/1.1, IPv6 and DHCP wait for interoperability testing against real stacks;
  - TLS is record-layer only;
  - X.509 refuses every chain until signature verification lands;
  - the crypto primitives are also left out.
- **The Rust route is not maintained from v1.1.0.** The Rust is frozen and archived.
  - The Rust crates are as in v1.1.0-alpha, and no Rust packages are built for v1.1.0.
  - `tools/fixpoint.sh`, `t1_image` and `cargo test` were not re-run, and may not reproduce the v1.1.0 image.
- **No Windows native package** (see above).
