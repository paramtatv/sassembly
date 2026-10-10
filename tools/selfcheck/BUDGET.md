# Selfcheck budget (registered before any measurement)

Registered 2026-10-09, before the suite was compiled or timed once.

Owner ruling: from Monday the everyday Sassembly sanity check is this .t1
self-check suite, run natively. Rust tests and yantra are nightly only.

BUDGET: the WHOLE suite (every image, run in parallel, compare against
expected.tsv) takes <= 10 s wall, natively, on EACH of these hosts:

- a Linux x86_64 host
- a Linux aarch64 host
- an Apple arm64 Mac
- an Intel Mac (x86_64)

Wall time is the "elapsed" line run.sh prints, cold start of run.sh to its exit.
Compilation of the images is not in the budget (images are built ahead).
yantra is UNBOUNDED: its mode (run.sh --verify-pins) is the nightly path that
checks expected.tsv, and it carries no time limit.

A host that misses 10 s fails the budget; the fix is to shrink a kernel, not to
raise the number.

## Measurements (appended after the budget above was committed)

2026-10-09, the Linux x86_64 host (load average 20-55 from other jobs), 29 programs, run.sh:

- native (tools/t1-run.sh, warm translation cache): 2.07 s, 2.31 s, 3.35 s. Within budget.
  The CACHE IS PART OF "WARM": the first ever run on a host builds the .t1 translator (15 min measured here
  under load) and translates each image once; run.sh once after build.sh, then the timed runs are warm.
  file_* programs use --files, which t1-run.sh routes to yantra-run (no native file window).
- yantra (--verify-pins, nightly, unbounded): 9.6 s, 9.7 s, 12.6 s.
- Linux aarch64, Apple arm64, Intel Mac: NOT yet measured.

Update: run.sh now parses the exact status from the relayed "halt:" line for BOTH runners, so native
compares exact codes (853/860/861/862). Native re-measured: 2.11 s, 2.20 s, 2.11 s (the Linux x86_64 host, loaded).
