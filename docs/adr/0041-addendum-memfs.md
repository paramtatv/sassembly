# ADR-0041 addendum: an in-memory file root for hosts with no filesystem

Status: accepted (owner-approved task), 2026-10-09.

The file window (ADR-0041) resolves a program's path under a directory root. A browser
has no directory, so `yantra::patra::MemFs` serves the same window from memory and
`yantra-wasm` exposes it (`yantra_memfs_*`, `web/yantra.mjs` `memfs*`). The compiler is
unchanged and `yantra-run --files` is unchanged.

Same refusals, by name, as the disk root: absolute path and climb above the root are
`Refused`; NUL, an over-long component (255) or path (4095), a missing directory on the
path and a file used as a directory are `NotFound` (read) or `NotWritten` (write); the
root itself is `NotFound` to read and `Refused` to write; a write over a cap is
`NotWritten`. Caps: 64 MiB total (data, names and a 64-octet entry overhead), 16 MiB per
file, 4096 files; a per-file cap above the total is refused. These are checked against
the disk root, case by case, by `patra::memfs_tests` and `tests/memfs_parity.rs`.

## Two divergences, deliberate

1. **An escape whose target does not exist is `Refused` in memory and `NotFound` on
   disk**, because on disk `canonicalize` fails before the prefix check can run. Refusing
   is the answer the `Refused` status documents (a policy must not read as a missing
   file), and it reveals nothing.
2. **Host seeding creates parent directories implicitly, and memory has no symlinks, hard
   links or permissions.** A program cannot create a directory (the window has no mkdir),
   so on disk the host makes them; `memfsPut("a/b.png", ...)` is the host doing so.
   `O_NOFOLLOW` has nothing to guard in memory.

The root is carried across runs until the host enables a fresh one; `yantra_host`
(applications in U-mode) never sees it, since an application cannot address the window.
