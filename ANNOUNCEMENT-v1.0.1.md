# Sassembly v1.0.1 — the release `yantra-run` can grant a file root

**One change.** `yantra-run --files DIR` gives a program a file root, so a Sassembly
program in the prebuilt release can read and write files. In v1.0.0 the release
`yantra-run` granted no root and refused every file request; this is the fix that
[LIMITS.md](LIMITS.md) and the v1.0.0 README promised.

The compiler image does not change. `stage1.elf` is byte-identical to v1.0.0
(sha256 `71d06d6649a861c1c6205c95a59fd362f48914b377c6ace9e612bef827415480`), and the
fixpoint still holds at 922,146 octets (`tools/fixpoint.sh`).

## Use it

```
yantra-run --files DIR [--events LOG | --record-events LOG ...] prog.elf [args...]
```

* `--files DIR` is recognised only as the **first** argument; everything after the
  image belongs to the program.
* The program's file window reads and writes inside `DIR` and nowhere else.
* Without the flag nothing changes: every file request is refused by name
  (halt status `2^48-4`, 281474976710652), exactly as in v1.0.0.

## Safety rules

* `DIR` is canonicalised at start. A missing or non-directory `DIR` is refused at
  load (exit 1) and nothing runs.
* `..`, absolute paths, and on Windows drive-letter, UNC and rooted paths are
  refused by form, with a status distinct from "not found".
* A path that resolves outside `DIR` through a symlink is refused.
* A write does not follow a symlink at the file itself (unix: `O_NOFOLLOW`).
* File requests are not events and are not logged. A log recorded under `--files`
  carries a header line saying so; replaying it without `--files DIR` is refused at
  load, because the replay would run a different program.
* `--files` with a program archive (`--smp`) is refused (exit 64).

## Threat model

`--files` trusts `DIR`. It protects a host from a program asking for a path outside
`DIR`; it does not protect against another party who can write inside `DIR` while
the program runs. Path swaps and hard links between the check and the open are not
detected. Do not grant a directory that an untrusted party can modify concurrently.
The file operations are still only read-whole and write-whole (no append, delete or
directory listing).

## The fixed entry of `stage1.elf` is still there

The prebuilt compiler still builds only module `शृङ्खला` with routine
`स्वपरीक्षास्वप्रतिबिम्बम्`; any other name halts with status 1601. Your options:

1. Name your program's module and routine that way (every example does).
2. Build from source with `t1_image`, which accepts any module and routine.
3. Lifting it in the prebuilt image needs a new compiler image; that is not part
   of v1.0.1, and no date is promised.

## Assets

Prebuilt binaries for Linux x86-64, macOS (arm64, x86-64) and Windows
(arm64, x86-64), plus `sassembly-v1.0.1-stage1.elf`. See the
[README](README.md#install) for the checksums and [LIMITS.md](LIMITS.md) for what
was verified. A Linux aarch64 build is not in this release yet; v1.0.0's aarch64 binary has no `--files`, so build from source if you need it there. The v1.0.0 release stays available.
