# Sassembly v1.0.2 — any entry from the input, files in the browser, AGPL-3.0-only

Three changes. The prebuilt compiler lost its fixed entry, the browser machine got an
in-memory file root, and the licence changed.

## 1. The entry comes from the input

In v1.0.1 the prebuilt `stage1.elf` built only module `शृङ्खला` with routine
`स्वपरीक्षास्वप्रतिबिम्बम्`; any other pair halted with status 1601. From v1.0.2 the
compiler reads its entry from the input name: `module NUL routine`. `yantra-run` builds
that name from one variable (an environment variable cannot hold a NUL):

```
YANTRA_INPUT=p.blob YANTRA_INPUT_ENTRY="<module> <routine>" yantra-run sassembly-v1.0.2-stage1.elf
```

* Use `YANTRA_INPUT_ENTRY` in place of `YANTRA_INPUT_NAME`. Exactly one ASCII space; both
  halves non-empty, no whitespace or NUL.
* Malformed, not UTF-8, over 4096 octets, or set together with `YANTRA_INPUT_NAME`: refused
  by name, exit 1, nothing runs.
* Without a NUL in the name the compiler keeps its own entry, so every v1.0.1 command
  still works unchanged.
* An entry the source does not contain halts with status 1601 and builds no image.

The compiler image changed, so the fixpoint was re-measured: Stage 2 equals Stage 1, byte
for byte, **923,042 octets** (v1.0.1: 922,146). `sassembly-v1.0.2-stage1.elf` sha256
`4e7a9a244e95bdbf759ba4d373e7efc2e6dcd0e383b9123ac63b5a37d20070ca`. Check it with `tools/fixpoint.sh`. The test is
`crates/yantra/tests/t1_entry_from_input.rs`.

## 2. The browser can serve files from memory

A browser tab has no directory for `--files`. `yantra-wasm` now serves the program's file
window from an in-memory root with the same path rules and the same refusals by name:
`memfsEnable(caps)`, `memfsPut(name, data)`, `memfsFiles()`, `memfsDisable()` in
`web/yantra.mjs`. Caps default to 64 MiB in total and 16 MiB per file, at most 4096 files;
names and per-entry overhead count against the total. Without `memfsEnable()` every file
request is refused, as natively without `--files`. Two deliberate divergences from the disk
root are in [docs/adr/0041-addendum-memfs.md](docs/adr/0041-addendum-memfs.md); the parity
tests are `crates/yantra/tests/memfs_parity.rs` and `tools/check-yantra-memfs.sh`.

## 3. The licence

From v1.0.2 Sassembly is released under the **GNU Affero General Public License, version 3
only** (AGPL-3.0-only); see [LICENSE](LICENSE). A **commercial licence** is available from
the copyright holder, परमतत्व: see [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md)
(paramtatv@fastbuilder.ai). Third-party files keep their own licences, listed in
[NOTICE](NOTICE).

**Releases v0.2 to v1.0.1 were published under the MIT licence and remain MIT**, as
published. Their tags and assets are unchanged.

## Tests

`cargo test --workspace --release --no-fail-fast`: see the README, Verification.
