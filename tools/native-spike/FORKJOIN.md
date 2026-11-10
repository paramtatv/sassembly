# Native fork-join threads in the oracle (`anuvada.py`) — P3, prototype

Status: **ON BY DEFAULT on Linux** (x86-64 and aarch64), in the oracle AND the .t1 translator
(byte-identical), through `tools/t1-run.sh`. **macOS stays serial for now** (no bsdthread_create yet:
t1-run asks no fork-join translation there; agent-handovers/native-threads-plan.md §1b). Plan row:
`agent-handovers/native-parallel-exploration.md` P3.

**The switches.** At run time `SASSEMBLY_NATIVE_THREADS=N`: unset (or empty) = the CPUs this process may run
on (sched_getaffinity, x86-64 syscall 204 / aarch64 123), at most 64; 0 = serial (t1-run then runs the plain
translation); a non-number is refused, exit 64. `SASSEMBLY_NATIVE_FJ_MIN=K` (default 8192, 0 = off): a region
of n indices uses min(N, n / K) threads and runs serially below two. `tools/t1-run.sh` then asks for the fork-join translation (its own cache entry, `…-fj`): the
oracle with `ANUVADA_FORKJOIN=1`, the .t1 translator with trace 65598 (x86-64) / 65719 (aarch64) —
65536 + the target. Without the switch every translation is octet for octet what it was.

**The .t1 port** (`t1/authoring`): head.t1a's encoder puts the fs override before REX for a data symbol
(`दत्तचिह्नम्`, the list written by mk.sh from `fj_x86_layout()`) and rewrites a data `lea`; main.t1a has the
plan (`समान्तरयोजना`, = `fj_plan`), the XF_ stubs (label family 13) and the image stubs; lower2.t1a / arm.t1a
lower the hint word. The runtimes: rt2t1.py / rta2t1.py `--fj` record the fork-join runtime and mk.sh folds it
into the plain one under `समान्तरविधिः`; the x86-64 F/D runtime is NOT recorded twice — rtfp2t1.py marks its
1,277 fs points and 2 syscalls (`समान्तरनिरपेक्षः`, `समान्तरप्रणाली`, compacted as फ / ब) and head.t1a inserts the
prefix in a fork-join image. The device engines (files, spawn) stay plain: a worker is refused every device
but the UART, WAIT and FINISHER (`rt_st_fjdev`). anuvada.t1: 2,881,443 octets (69% of the 4 MiB limit;
main 2,580,973); check-reach: the startup jal reaches +40,884.

**Byte identity** (x86_64 host, the merged tree with 5f's fj-purity, compiler d6480f2d): 80 selfcheck images + 8
fj programs, the built .t1 translator (run natively) against the oracle: 438/438 plain translations
identical over x86_64-linux, aarch64-linux, x86_64-macos, aarch64-macos, wasm32 (2 wasm32 float images
the oracle itself refuses); 176/176 fork-join translations identical (x86-64 and aarch64 Linux).

## What the compiler emits (main at c761e0a0, 5f)

`समान्तरॱचालनम् आरभ्य lo ऽ hi ऽ body ऽ out समाप्तम्` lowers (`ir.t1` `समान्तररचना`) to:

1. a call of the module's **marker routine**, fixed text `lui x0, 0xF0A11; mv a0, a1; ret`
   (`yantrotsarjana.t1` `यन्त्रसमान्तरचिह्नोत्सर्जनम्`), with a0 = 1 (begin), a1 = lo, a2 = hi,
   a3 = the output run, a4 = the body's address. It answers a1: **the index where the
   serial loop starts**;
2. the serial loop `for i in [answer, hi)` (unsigned compare): `body(a0 = i, a1 = out)`;
3. the end call: (0, 0, hi, out, body).

yantra executes the `lui x0` as a no-op hint, so it answers lo and runs every index serially
and in order. The refusals 0x372 to 0x376 are checks compiled INTO the body: a write to an element
other than its own, a read of another index's element, an allocation or growth, a nested region,
file or device I/O. Each check stores the FAIL word to the FINISHER. So any engine that runs the
body's code refuses identically. 0x377 and 0x37F are build refusals.

## The design

The marker is recognised at translation time: the exact three words, called by some `jal ra`. The
entry's `auipc sp`/`addi sp` gives sp0, and the startup's canary sequence must name
SB = sp0 − 64 KiB. Both translators then lower the hint word to a runtime call `rt_fj`.
Only with **`ANUVADA_FORKJOIN=1` at translation**, on Linux, with FAST blocks: every other
translation is octet for octet what origin/main's oracle makes. Checked: 77 selfcheck images
and the extra programs, 5 targets each (408 identical; with `ANUVADA_FORKJOIN=1`, the 253
translations of images without a marker call are identical too).

At run time, **`SASSEMBLY_NATIVE_THREADS=N`** (unset/0 = serial, clamped to 64). `rt_fj`
returns at once, so the serial loop runs everything, unless all of these hold:

- phase 1;
- N > 0;
- lo < hi;
- the caller is the main thread (a nested marker in a worker runs serially);
- the caller's sp lies in (SB + 2056, sp0];
- T × 80 KiB at the top of RAM lies above the image's extent and is all zero;
- the worker pool can be mapped.

When they hold, [lo, hi) is cut into T = min(N, hi − lo) contiguous chunks (q or q+1 indices).
Each chunk runs in order on its own raw `clone(VM|FS|FILES|SIGHAND|THREAD|SYSVSEM|CHILD_CLEARTID)`
thread, with SETTLS on x86-64.

- **x86-64.** A worker gets a copy of the whole data block. Every data label is addressed
  `fs:`-relative in a fork-join image (fs base 0 in the main thread, the block's delta in a
  worker; `lea` of a data label adds `fj_delta`). That covers the guest registers that are not
  in host registers, the F/D state, the UART buffer, the fault words and the FAST sp window. Host
  registers come from the marker's snapshot. Each worker also has a 64 KiB host stack.
- **aarch64.** The frame already holds all of that at the host sp, so a worker's sp is simply a
  copy of frame + F/D data + fork-join area. x18 (no guest register on Linux) addresses the area.
- **The guest stack.** Each worker has an 80 KiB slice at the top of RAM. The body starts at
  slice top − 4 KiB with ra = a sentinel (0x7FFFFFF0). When the body returns there, the worker
  takes the next index (`rt_bad_target`).
- **Join.** The main thread waits for chunk 0, 1, … **in order**: the CLEARTID word through a
  futex, then `mfence` / `dmb ish`. It replays each chunk's records.
  - If a chunk exited (a halt or refusal), the process exits right then with that code. Later
    chunks are never waited for.
  - When every chunk completed, the F/D sticky flags are ORed into the main thread's fcsr, the
    slices are zeroed again, and a1 := hi. `mv a0, a1` then answers hi, the serial loop runs
    nothing, and the end call is the plain hint.
- **Refusals and output.** A worker's host calls never reach the OS: `rt_fj_sys` (x86) and
  `sysc` → `rt_fj_sw` (A64) divert them.
  - A write is appended to its capture (fd, length, octets), up to 16 KiB.
  - An exit records its code and ends the thread.
  - Any other call (the clock, a sleep, …) marks the chunk "refused".

  The main thread flushes its own UART buffer before the fork. So the octets on fd 1 and fd 2,
  in order, and the exit status are exactly the serial run's.

## The exactness argument

The serial run executes chunk 0's indices, then chunk 1's, …, and stops at the first index that
halts. The body may write only its own element of the output run. That is checked in the body's
code, by the same instructions on every engine.

- Within a chunk, a worker runs the same instructions in the same order as the serial run.
- Its view of guest memory is the same: other chunks touch only their own elements and their own
  stack slices.
- Its registers are the marker's snapshot plus a0, a1, ra and sp. Compiled code reads no
  register or stack slot it did not write, so the different ra, sp and stack contents are not
  observable.

So the first halting chunk halts at the same index with the same message, and the chunks before
it completed exactly as they would have. Nothing after that index is observable: the process
ends. The main thread's state after a completed region is the serial run's, with three
differences:

- the dead stack area below its sp;
- caller-saved temporaries, which compiled code does not read;
- fcsr, which is merged.

**Stack depth.** A worker's FAST window is [start − depth, start + 8], with
depth = caller sp − SB − 2056. While a worker is inside it, the serial run at the same depth
stays above the canary word SB, so it neither trips the canary (0x353B) nor overflows. A worker
that leaves the window is not continued. The SAFE region's fall-through after an sp write now
re-enters the check.

## What falls back

A worker that left its slice, made another host call, overflowed its capture, or changed frm or
FS ends the run with `native: not in milestone 1 (a fork-join worker …; run it under
yantra-run)`. `tools/t1-run.sh` then reruns the image under yantra-run. This happens only when
that chunk is the first one that did not simply complete. Everything else falls back to
**serial**, which is exact by definition:

- the switch is off;
- a nested region;
- the caller's sp is outside the stack;
- RAM is too small or the slices are in use;
- `clone` or `mmap` failed for the first thread (a later `clone` failure faults to yantra);
- an image without the canary pattern, or one whose marker is reached only through `jalr`.

## Measured (2026-10-10)

`tools/native-spike/fj-check.sh` compares every N with the serial translation (stdout, stderr,
exit) and the serial run with yantra-run (stdout, exit, halt line). It was run over all 25
`tools/selfcheck/fj_*`, the 3 `arena_*region*` selfchecks, and `fj/matmul.t1`, `fj/hang.t1`
(chunk 1 never ends after chunk 0 refuses), `fj/deep.t1`, `fj/float.t1`, and `deep.t1` with depth
100 000 (both routes fall back).

| host | N = 1, 2, 8, 20 | stress: 200 runs at N = 20 |
|---|---|---|
| x86_64 host | all exact | 12 000 + 1 200 runs, all exact, no hang |
| ARM64 host, aarch64 | all exact | 12 000 + 1 200 runs, all exact, no hang |

Matmul (`fj/matmul.t1` with n = 600: 216 M multiply-adds, plus a serial fill and checksum), wall
ms, the best of 3:

| host | serial | N=1 | 2 | 4 | 8 | 12/16 | 20 |
|---|---|---|---|---|---|---|---|
| x86_64 host, 20 cores, load 30 to 85 (shared) | 2646 | 2670 | 1390 | 1127 | 652 | 565 (16) | 527 |
| ARM64 host, 12 cores, load 2 to 4 | 2428 | 2463 | 1259 | 652 | 349 | 334 (12) | 273 |

With n = 300, x86_64 host (load about 95): serial 330, N=0 322, N=1 305, N=2 176, N=8 91, N=20 78.
ARM64 host: 307, 303, 308, 154, 46, 47 (N=12). The fork-join image run serially (N=0) is no slower
than the plain one.

Stress: `FJ_NS=20 FJ_REPEAT=200`, each run under `timeout 60`, on both hosts. The 30 images
above each ran 200 times at N=0 and 200 times at N=20; the 3 arena-region images were run the
same way. All 13 200 runs per host were exact, and none timed out, `hang.t1` included.

## 0x380 (5f's fj-purity, a run-time check in the body)

`fj/alias_low380_high372.t1`, `fj/alias_low372_high380.t1`, `fj/alias_high380.t1`: [0, 400), with N=20 the
index 3 is in chunk 0 and 390 in chunk 19. The low chunk's refusal wins (896 and 882 respectively), and a
refusal only in the highest chunk is reported once every lower chunk completed (896). Exact at N = 1, 2, 8,
20 on both hosts, and 200 runs at N=20 of each (with `fj_refuse_380_alias_read`) on both: 1,600 per host.
The whole matrix on the merged tree (33 selfchecks with regions + 8 programs): 205 threaded runs per host,
all exact.

## m1 and a WAIT in a region (8f)

`fj/m1.t1` (8f's mutant-killer: a high chunk halts at its first index before a lower chunk does) answers
882 — a "first to halt wins" runtime answers 896. `fj/expected.tsv` pins it (and the other fj programs'
statuses) and fj-check.sh enforces the pin on every run: N = 1, 2, 8, 20 and 200 runs at N=20 on both hosts,
always 882. The .t1 translator equals the oracle for m1 and the alias_* programs on every target.

`fj/wait.t1` reads the wall clock (a store to WAIT, in spec/clock's library) inside a body: the build is
refused, 0x381 (fj-purity: an impure call graph), so no built image reaches a worker's WAIT. Were one to,
rt_wait's host calls go through rt_fj_sys and the worker is refused (kind 3): the run falls back to yantra.

## Default on: the small-region threshold K (measured 2026-10-10)

The cost of a region on threads is about 45 µs per thread on the x86_64 host (20 cores, load 10–30) and 27 µs on
the ARM64 host (12 cores): clone, the state copy, the slice scan and re-zeroing. The cheapest body there is (one
store) costs about 20 / 11 ns an index, so a thread pays for itself from a few thousand indices: K = 8192.
200 regions of n indices, trivial body, wall ms, best of 7 — before (serial) → after (the default):

| n | x86_64 host | ARM64 host |
|---|---|---|
| 64 | 3 → 5 | 1 → 1 |
| 1,024 | 7 → 8 | 3 → 3 |
| 4,096 | 19 → 20 | 10 → 10 |
| 16,384 | 68 → 60 | 37 → 36 |
| 65,536 | 260 → 122 | 146 → 67 |

(K = 0, every region on all CPUs: 189 / 65 ms at n = 64 — that is what the threshold prevents.) The small
regions' 1–2 ms is per process (the fork-join image), not per region. Large regions: matmul 300 (90,000
indices) 447 → 96 ms on the x86_64 host, 382 → 64 ms on the ARM64 host (K = 4096 would give 86 / 55, but makes a trivial
65,536-index region 184 ms on the x86_64 host; K = 16384, 134 / 81).

**The gate.** tools/selfcheck/run.sh runs every region program (`fj_*`, `*region*`) on SC_FJ_THREADS (8)
threads with the threshold off, and `fj_threads_lowest_chunk` (8f's m1) pins 882; every other program runs
with the default. Native selfcheck on the branch: 81 programs, ok, 3.1 s. Byte identity: 448/448 plain and
180/180 fork-join translations. Matrix (`fj-check.sh`, now N = 1, 2, 8, 20 and unset): 258 runs per host,
exact; stress 200 runs each at N = 20 and unset of m1, the alias_* programs, lowest_index_wins, matmul, hang:
4,200 per host, exact.

## What remains

- **macOS** threads (bsdthread_create + __ulock), then the default there too.
- **The static-check gaps** (tools/LANDING.md) were closed by 5f's fj-purity (f90b2859): 0x380 is checked
  in the body at run time, 0x381 (an impure call graph) and 0x382 (a name collision) refuse the build. The
  switch is still opt-in (`SASSEMBLY_NATIVE_THREADS`); serial stays the default.
- **aarch64-macos and x86_64-macos.** These need `bsdthread_create` (or a pthread-free
  equivalent). On x86-64 they also need another per-thread base, because macOS gives no fs base.
- **Cost per region.** A region pays for a zero-scan and zeroing of T × 80 KiB, a copy of the
  per-thread state for each worker (a few KiB: everything but the 64 KiB UART buffer and the
  capture) and T clones. That is fine for matmul-sized regions; a thread pool would suit many
  small regions.
- **The bump allocator.** It has no upper bound (yantra input.rs). A program whose heap has
  already reached the top-of-RAM slices runs the region serially, because the zero-scan sees
  data there. The exception is heap data there that is all zero octets: a worker's stack would
  overwrite it during the region, and the slices are zeroed again only after. That needs a heap
  past 512 MiB, within T × 80 KiB of RAM's end.
