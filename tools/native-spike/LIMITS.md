# Native runtime LIMITS (step 0 of "runtime completeness, no holes")

What `tools/t1-run.sh` does today with each yantra-run feature, per host. Updated 2026-10-10 on agent/native-halts (NATIVE-GAPS A1, A2, B1, G1 and the threads guard), base origin/main c5488650;
first measured 2026-10-10 at origin/main e90def04
with `tools/native-spike/limits/probe.sh` (53 hand-assembled RV64 probe images from `limits/gen.awk` plus the flag and
env probes: 71 probes per host). All four probed hosts gave **identical classes on every probe**; the translator
(`anuvada.py`) is "milestone 1" on all four targets.

## Classes
- **N** native: runs natively, same output and halt as yantra-run.
- **F** falls back: t1-run reruns the whole program under yantra-run (reason text in the last column). Where a host has no
  yantra-run (the Intel Mac) the fallback is itself a refusal: `t1-run: REFUSED -- no yantra-run at ... to fall back to`.
- **R** refused by name by the native runtime itself (exit 1, no rerun).
- **M** missing: no native support and no fallback that helps (silent divergence, crash, or missing in yantra too).
- **pending**: Windows (sansos-27 is porting it).

## Counts (82 rows; identical for the probed hosts)
N 23, N/F 3 (native for what yantra halts on, a rerun for what it would execute), F 48, R 1, M 7. Windows: all pending.
Row 25 (YANTRA_WATERMARK) changed after these counts: N on the two Linux hosts (D2-WATERMARK), still F on macOS.
Row 22 (YANTRA_VERDICT) likewise: N on the two Linux hosts (L33; agent-handovers/L33-VERDICT.md), still F on macOS.
The x86_64-macos column was not re-probed in the 2026-10-10 update (the Intel Mac builds only); its rows carry the
x86-64 runtime the Linux column ran.

**probe.sh classes (71 probes), 2026-10-10, agent/native-halts:** fallback 36, native 13, native-halt 22 on
x86_64-linux (x86_64 Linux), aarch64-linux and aarch64-macos; the sorted `probe<TAB>class`
digest is **2e0c31601328 on all three** (`cut -f1,2 OUT | LC_ALL=C sort | sha256sum`). On origin/main 8c7df26d it was
native 17, native-halt 4, fallback 50 (with the T1_RUN_REQUIRE_NATIVE classifier fix). Every native and native-halt row
was compared with yantra-run on the same host: the same stdout, the same `halt:` line and the same exit status.
probe.sh now reads t1-run's `native required (T1_RUN_REQUIRE_NATIVE) but: <reason>` refusal as a fallback (the probe on
main read it as native).

## What the native runtime IS today
Output of RV64 I + M, `fld`/`fsd` as moves, `fence`/`fence.i` as no-ops, the `sstatus` set-bits CSR form, UART store,
FINISHER store, `YANTRA_INPUT*`/`YANTRA_RAM`/argv, and BeyondRam halts. Anything else either is decided before the run
(any yantra-run flag, a yantra-only env var: `t1-run.sh` lines 49-60) or is hit at run time ("native: not in milestone 1",
"halt: BadAccess", "native: refused a store into the code span", "native: jump to ...") and the output is discarded and
the program rerun whole. So **no float arithmetic, no A, no V, no CSR beyond sstatus, no SBI ecall, no MMIO load, and no
device other than UART/FINISHER is native yet**.

## The table
Cells are the class on that host. "read" in the probe column means classified from code, not probed.

| # | area | feature | x86_64-linux | aarch64-linux | aarch64-macos | x86_64-macos | windows | probe | reason / note |
|---|---|---|---|---|---|---|---|---|---|
| 1 | flag | --events LOG (replay) | F | F | F | F | pending | `flag:--events` | the yantra-run flag --events has no native counterpart |
| 2 | flag | --record-events LOG (live WAIT clock) | F | F | F | F | pending | `flag:--record-events` | the yantra-run flag --record-events has no native counterpart |
| 3 | flag | --record-events LOG --listen 127.0.0.1:PORT (TCP server) | N | N | F | F | F | `rtprobe/sockpair.sh` | agent/rtc-sock: native on Linux (YANTRA_LISTEN; SOCKET.md), transcripts identical to yantra-run's on both Linux targets; macOS and Windows route to yantra-run; replay (--events) stays yantra-run's |
| 4 | flag | --net-peer PATH (virtio-net frames) | F | F | F | F | pending | `flag:--record-events` | the yantra-run flag --net-peer has no native counterpart |
| 5 | flag | --net-tap NAME | F | F | F | F | pending | `read` | the yantra-run flag --net-tap has no native counterpart |
| 6 | flag | --net-mac | F | F | F | F | pending | `read` | the yantra-run flag --net-mac has no native counterpart |
| 7 | flag | --net-max-frame | F | F | F | F | pending | `read` | the yantra-run flag --net-max-frame has no native counterpart |
| 8 | flag | --net-max-tx-frames | F | F | F | F | pending | `read` | the yantra-run flag --net-max-tx-frames has no native counterpart |
| 9 | flag | --net-max-rx-frames | F | F | F | F | pending | `read` | the yantra-run flag --net-max-rx-frames has no native counterpart |
| 10 | flag | --net-timeout | F | F | F | F | pending | `read` | the yantra-run flag --net-timeout has no native counterpart |
| 11 | flag | --files DIR (patra file root) | F | F | F | F | pending | `flag:--files` | the yantra-run flag --files has no native counterpart |
| 12 | flag | --smp ARCHIVE / SMPT archive | F | F | F | F | pending | `flag:--smp` | the yantra-run flag --smp has no native counterpart |
| 13 | flag | --version | F | F | F | F | pending | `flag:--version` | ROUTED TO yantra-run (H4): t1-run runs yantra-run for a lone `--version`, whose stamp is the answer byte for byte; without a yantra-run (or under T1_RUN_REQUIRE_NATIVE) it prints the native runner own stamp |
| 14 | flag | --source-stamp | F | F | F | F | pending | `flag:--source-stamp` | ROUTED TO yantra-run (H4): as row 13 for a lone `--source-stamp` |
| 15 | env | YANTRA_INPUT + YANTRA_INPUT_NAME (the module, injected) | N | N | N | N | pending | `read; the native .t1 translator itself runs this way` |  |
| 16 | env | YANTRA_INPUT_ENTRY ("module routine") | N | N | N | N | pending | `env:YANTRA_INPUT_ENTRY malformed` | native prints yantra's own refusal text for malformed / both-set / >4096 / non-UTF-8 |
| 17 | env | YANTRA_INPUT on an image with no input tag (or two) | R | R | R | R | pending | `env:YANTRA_INPUT (image has no input tag)` | native: the image lacks an input tag, or has one twice (exit 1, no rerun) |
| 18 | env | YANTRA_INPUT_TRACE (trace code) | N | N | N | N | pending | `read; t1-native-lib passes 0/183/268/263 per target; +65536 fork-join (65598 x86-64), +131072 watermark (131134 x86-64, 131255 aarch64); t1/authoring/README lists them` |  |
| 19 | env | YANTRA_RAM=<n> (valid, big enough) | N | N | N | N | pending | `env:YANTRA_RAM=64M` |  |
| 20 | env | YANTRA_RAM non-numeric or smaller than the image | N | N | N | N | pending | `env:YANTRA_RAM=bogus` | parity with yantra-run (rtprobe/mklimits.sh, limits.expected): a non-numeric value is the default RAM, a too-small one refuses with yantra's text, exit 1; a value that parses but cannot be allocated is not matched |
| 21 | env | YANTRA_STEPS (exact step limit) | F | F | F | F | pending | `env:YANTRA_STEPS` | Native reads YANTRA_STEPS (decimal u64, invalid = default 1000000) and halts exactly as yantra; the StepLimit stderr names the effective limit; t1-run.sh passes it to the native run (no reroute) and defaults native to 4e12 as t1-build/fixpoint do; fork-join images are counted too. DIVERGENCE (named): yantra-run prints a `steps: N` tally on a finished run, native prints none, except under YANTRA_VERDICT (row 22), whose own per-word count prints it |
| 22 | env | YANTRA_VERDICT (+ YANTRA_STEPS) | N | N | F | F | pending | `env:YANTRA_VERDICT` | Linux: the `--verdict` translation (trace 262206 / 262327) counts yantra's steps (one per instruction begun), honours YANTRA_STEPS, answers a `jal .` with SpinForever (exit 1, as yantra-run) and writes `<first>\nsteps N` after the `halt:` line (agent-handovers/L33-VERDICT.md); macOS: YANTRA_VERDICT is a yantra-only feature |
| 23 | env | YANTRA_SCANOUT (gpu scanout PPM) | N | N | F | F | pending | `env:YANTRA_SCANOUT` | L33: Linux, oracle translation (ANUVADA_SC=1 via t1-run: scanout_dev.py, `-sc` cache entry): slot 0's GPU answers exactly as yantra's (virtio_mmio/virtqueue/virtio_gpu/gpu.rs) and writes the PPM; anything yantra HALTS on (and any mmap/open/write failure) is `native: not in milestone 1 (virtio-gpu: WHY; ...)`, exit 2, t1-run reruns under yantra-run; macOS and Windows: yantra's |
| 24 | env | YANTRA_VIRTIO_DEFER | N | N | N | N | pending | `env:YANTRA_VIRTIO_DEFER` | L33: native == yantra-run. A non-net image has no virtio device: t1-run.sh prints yantra's `virtio: completions deferred by N instruction(s)` stderr line and runs on (a value yantra-run would refuse is left to yantra-run). A net run on x86_64 Linux uses the `ANUVADA_VD=1` translation (FAST blocks off, a deadline check per guest word; design in agent-handovers/L33-VDEFER.md) and the log header carries `defer=N`; the other hosts have no native virtio, so a net run there is yantra-run's as before |
| 25 | env | YANTRA_WATERMARK (RAM high-water report) | N | N | F | F | pending | `env:YANTRA_WATERMARK` | Linux: the `--watermark` translation (trace 131134 / 131255) keeps max(ram offset + width) over every RAM store and prints yantra-run's `ram: high water N of M octets` at exit when the variable is set; macOS: YANTRA_WATERMARK is a yantra-only feature |
| 26 | env | program arguments (ARG0 = image path as given, argv[1..]) | N | N | N | N | pending | `read (anuvada.py joins arg0 + argv by NUL)` |  |
| 27 | run | exit status rule (finisher 0 = 0; any other halt = 1) | N | N | N | N | pending | `finisher_fail, finisher_halt_nostatus` |  |
| 28 | run | default step ceiling DEFAULT_STEPS = 1,000,000 (StepLimit halt) | M | M | M | M | pending | `read` | CLOSED (design B): FAST region charges a block at its leader (`sub [steps],rem; jb S_k`); on a borrow S_k runs the block in the SAFE region, one word per charge (`sub [steps],1; jb T_k`), so the limit halts exactly at its word whatever earlier words did (load, store, ecall); no refusal. `halt: StepLimit { pc: N }`, exit 1, equal to yantra. A jalr into a block middle runs SAFE. Fork-join: counter on; workers charge their own, the join subtracts the consumed steps from main; a borrow or a worker meeting the limit exits 1 (`run under yantra-run`; t1-run reruns). H2: a self-jump halts `halt: SpinForever { pc: N }`, exit 1 (yantra-run maps it to 1 since adea9a50; owner ruling H3). The qemu-only images spec/bootloader and spec/virtio-gpu still end in the self-jump (qemu has no finisher; the framebuffer must stay up for the screendump) and exit 1 under yantra-run: that is the ruling, not a bug |
| 29 | run | 'steps: N executed instructions' report | M | M | M | M | pending | `read` | DELIBERATE: native prints no step count (attestation counting exists in the spike only) |
| 30 | run | UART output streaming (bytes visible before exit) | M | M | M | M | pending | `read` | t1-run holds all output until the native run ends (so a discarded run leaves no partial output); an interactive or long-lived program shows nothing until exit |
| 31 | run | stdin to the program | M | M | M | M | pending | `read` | no stdin device exists in yantra either; t1-run says a rerun does not replay stdin |
| 32 | run | wall-clock exit time bound / timeout | M | M | M | M | pending | `read` | neither runtime has one except the step ceiling |
| 33 | device | UART store (0x1000_0000) | N | N | N | N | pending | `uart_finisher` |  |
| 34 | device | UART load | N | N | N | N | pending | `mmio_uart_load` | yantra's `halt: BadAccess { pc, addr }` natively (rtprobe/mklimits.sh); t1-run keeps yantra's Debug form, rerunning only the runtime's prose `halt: BadAccess (` |
| 35 | device | FINISHER store (0x10_0000): pass 0x5555, fail 0x3333, none 0x7777 | N | N | N | N | pending | `uart_finisher, finisher_fail, finisher_halt_nostatus` |  |
| 36 | device | FINISHER load | N | N | N | N | pending | `mmio_finisher_load` | as row 34 |
| 37 | device | WAIT store (0x1000_0100): halt-and-resume | N | N | N | N | pending | `mmio_wait_store` | no event source (no YANTRA_RECORD_EVENTS): yantra's `halt: Wait { pc }`, exit 75, natively (tagged or not); with YANTRA_RECORD_EVENTS the native clock (81d4c51c); a recorded WAIT with the SASEVENT tag missing or duplicated still reruns |
| 38 | device | COUNTER load (0x1000_0108): retired-instruction clock | F | F | F | F | pending | `mmio_counter_load` | native halts BadAccess (MMIO load is not in milestone 1); t1-run reruns whole |
| 39 | device | SOCK store (0x1000_0110) | N | N | F | F | F | `rtprobe/sockpair.sh` | agent/rtc-sock: NEXT, TX and every refusal by name (halt Device), with or without a listener (no device: yantra's words and its `socket: touched` line); macOS and Windows keep the old halt and rerun |
| 40 | device | SOCK load | N | N | F | F | F | `rtprobe/sockpair.sh` | agent/rtc-sock: RX answers through the load's own B label (the load cold stub now calls rt_load_slow); refusals by name; macOS and Windows keep the old halt and rerun |
| 41 | device | patra file window stores: PATH 0x1000_2000, BUFFER +8, GO +0x10, PUT +0x18 | F | F | F | F | pending | `mmio_patra_path_store, mmio_patra_go_store` | BadAccess store; rerun |
| 42 | device | patra status load | N | N | N | N | pending | `mmio_patra_load` | yantra answers no load in virtio-mmio slot 1 (0x1000_2000..0x1000_3000, the file window's slot): `halt: Device { pc, addr, why: "virtio-mmio slot 1 overlaps the file window ..." }`, exit 1, whatever its state (virtio_mmio.rs check()); native prints the same line (rtprobe/mklimits.sh p.l*, fld too) and t1-run keeps it. Stores in slot 1 other than patra's four and native's window ops answer the same. x86_64-macos translated, not run |
| 43 | device | virtio-mmio load (8 slots from 0x1000_1000; slot 0 gpu, slot 2 net) | F | F | F | F | pending | `mmio_virtio_load` | L33: slot 0 (the GPU) answers natively under YANTRA_SCANOUT on Linux (row 23); other slots and the var unset: BadAccess, rerun |
| 44 | device | virtio-mmio store (queue notify, status, ...) | F | F | F | F | pending | `mmio_virtio_store` | L33: slot 0 stores answered natively under YANTRA_SCANOUT on Linux (row 23); others: BadAccess, rerun |
| 45 | device | virtio-gpu (2D, scanout, RESOURCE_FLUSH) | N | N | F | F | pending | `read (via virtio-mmio rows)` | L33: native on Linux with YANTRA_SCANOUT set (row 23); var unset: the stock translation halts at slot 0 and t1-run reruns under yantra-run |
| 46 | device | virtio-net frame device (slot 2, --net-peer / --net-tap) | F | F | F | F | pending | `read (via virtio-mmio rows)` | no native device model |
| 47 | device | unmapped store below 2^31 (BadAccess) | N | N | N | N | pending | `mmio_unmapped_store` | native prints yantra's `halt: BadAccess { pc, addr }` (exit 1) for a store below base at no device: 0, the COUNTER, UART+1, FINISHER+8, 0x1000_0200, 0x7fff_f7f8 (rtprobe/mklimits.sh p.s*); t1-run keeps it. A store to the socket or virtio-mmio slots 0, 2..7 still prints `halt: BadAccess (a store to the socket or a virtio-mmio slot; not in milestone 1)` and reruns (C3/C4). x86_64-macos translated, not run |
| 48 | memory | load / store past RAM + slab: BeyondRam halt | N | N | N | N | pending | `ram_beyond_load, ram_beyond_store` | yantra's own line `halt: BeyondRam { pc, addr, ram }` (ram: the writable span for a store, the loads' limit for a load) |
| 49 | memory | store into the code span (PF_X segment, self-modifying code) | F | F | F | F | pending | `store_into_code` | KEPT: the store is legal in yantra and a later fetch of the changed word would execute the new instruction; the translation is fixed code (patching it is impossible on aarch64-macos, whose signed text pages cannot be rewritten), so a store into the code span is refused and rerun |
| 50 | memory | jump outside the code span / misaligned pc / to 0 | N/F | N/F | N/F | N/F | pending | `jump_out_of_code, jump_misaligned, jump_to_zero` | N: yantra's fetch halt natively: below RAM `BadAccess { pc, addr }`, past the loads' limit `BeyondRam`, in RAM `Unimplemented` for a word with no 11 low bits or an opcode yantra does not execute. F (rerun): a target in the SOCK or virtio window (a device answers the fetch) or a word yantra would execute |
| 51 | memory | fence / fence.i (exact no-ops) | N | N | N | N | pending | `fence, fence_i` |  |
| 52 | memory | guest RAM mapped at its own address (needs 0x8000_0000.. free in the host process) | N | N | N | N | pending | `uart_finisher` | native: guest RAM could not be mapped at its own address is the refusal if not |
| 53 | thread | cooperative threads (SASTHRDS/SASTHRID tags + WAIT switches, up to 64) | F | F | F | F | pending | `threads_tag_wait` | an image carrying SASTHRDS is refused at startup (`native: not in milestone 1 (a threaded image ...)`), so t1-run reruns it whole; it no longer runs single-threaded |
| 54 | thread | event-log replay / determinism (--events, SASEVENT slot) | F | F | F | F | pending | `flag:--events` | flag has no native counterpart |
| 55 | thread | SMP archive hart scheduling (--smp, SMPT) | F | F | F | F | pending | `flag:--smp` | flag has no native counterpart |
| 56 | isa | RV64I: lui auipc jal jalr branches, ALU, shifts, W forms | N | N | N | N | pending | `uart_finisher, int_mul_div_rem` |  |
| 57 | isa | loads/stores to RAM (lb..ld, sb..sd, misaligned allowed) | N | N | N | N | pending | `fld_fsd, int_mul_div_rem` |  |
| 58 | isa | M: mul mulh mulhsu mulhu div divu rem remu + W forms | N | N | N | N | pending | `int_mul_div_rem` |  |
| 59 | isa | fld / fsd (as 64-bit moves) | N | N | N | N | pending | `fld_fsd` |  |
| 60 | isa | flw / fsw | F | F | F | F | pending | `fp_flw` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 61 | isa | F/D arithmetic: fadd fmul fdiv fsqrt fma fcvt fmv fcmp fclass fmin/fmax, rounding modes | F | F | F | F | pending | `fp_fadd_d, fp_fmadd_d, fp_fcvt_w_d` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 62 | isa | fcsr / frm / fflags CSRs | F | F | F | F | pending | `fp_csr_fcsr` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 63 | isa | A: amo*.w/.d | F | F | F | F | pending | `amoadd_w, amoadd_d` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 64 | isa | A: lr / sc | F | F | F | F | pending | `lr_sc_w` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 65 | isa | V (vector): vsetvli, vle, ... | F | F | F | F | pending | `vector_vsetvli, vector_vle` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 66 | isa | C (compressed) / any undecodable word | N | N | N | N | pending | `rvc_word, illegal_word` | yantra's `halt: Unimplemented { pc, word, opcode }`, decided per word at translation (anuvada.py yantra_static) |
| 67 | isa | CSR sstatus set-bits (the startup's FS/VS enable) | N | N | N | N | pending | `csr_sstatus_set` |  |
| 68 | isa | other CSR access: sstatus write/clear, stvec, sepc, scause, stval, sscratch, sie, sip, satp | F | F | F | F | pending | `csr_stvec_write, csr_satp_write, csr_sie_read` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 69 | isa | time / cycle / instret CSRs | F | F | F | F | pending | `csr_time, csr_cycle, csr_instret` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 70 | isa | ecall as SBI putchar (EID 1) | F | F | F | F | pending | `sbi_putchar` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 71 | isa | ecall as SBI shutdown (EID 8) | F | F | F | F | pending | `sbi_shutdown` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 72 | isa | ecall as SBI set_timer (EID 0) and timer interrupt (stip) | F | F | F | F | pending | `sbi_set_timer` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 73 | isa | ecall, any other EID (yantra halts naming it) | F | F | F | F | pending | `read` | native stops 'not in milestone 1' (translator lowers no such word); t1-run: 'the native run reached what the translator refuses at run time' then reruns |
| 74 | isa | ebreak / trap delivery to stvec | N/F | N/F | N/F | N/F | pending | `ebreak` | N: ebreak halts `Breakpoint { pc }` (stvec is always 0 natively). Trap delivery to stvec needs a stvec write (row 68): F |
| 75 | isa | sret / mret / wfi | N | N | N | N | pending | `sret, mret, wfi` | sret returns to sepc = 0: `BadAccess { pc: 0, addr: 0 }`; mret and wfi `Unimplemented`, as yantra |
| 76 | isa | sfence.vma, Sv39 paging, U-mode (process.rs / supervisor.rs) | N/F | N/F | N/F | N/F | pending | `sfence_vma` | N: sfence.vma is a no-op in S-mode, as yantra. Sv39 paging / U-mode need satp and the trap CSRs (row 68): F |
| 77 | wanted | entropy source (TLS needs it) | M | M | M | M | pending | `read` | NO runtime has one: yantra defines no entropy device, SBI call or CSR; native none either |
| 78 | wanted | wall clock (TLS cert validity, timeouts) | F | F | F | F | pending | `mmio_wait_store` | exists only as yantra's WAIT + --record-events t=<ns> (or the instruction COUNTER); native has neither, so any program that waits reruns under yantra-run |
| 79 | wanted | TCP client (outbound connect) | M | M | M | M | pending | `read` | yantra has only --listen (one loopback client) and --net-peer frames; no outbound connect device anywhere |
| 80 | wanted | TCP server (--listen) | F | F | F | F | pending | `flag:--record-events` | needs yantra-run |
| 81 | wanted | file read / write | F | F | F | F | pending | `mmio_patra_path_store, flag:--files` | patra window is yantra-only; patra root confinement (O_NOFOLLOW, memfs caps 64 MiB total / 16 MiB file / 4096 files) are not in native |
| 82 | device | native-only stores: PARAM 0x1000_2020, file append/delete/rename/list/stat/mkdir/read-at/write-at +0x28..+0x60, stdin +0x68, env +0x70, spawn +0x78 | N | N | N | N | pending | `rtprobe/mkstdenv.sh, mkspawn.sh, mkfiles.sh` | H15 (SAS-LIMITS-FIX L20a): NATIVE-ONLY — yantra-run has no such store and halts `Device { …, why: "virtio-mmio slot 1 overlaps the file window …" }`; on the yantra route (T1_RUN_ROUTE=yantra, or a rerun) t1-run passes that halt through and REFUSES by name (`native-only device <name> (0x…) at pc 0x…`), exit 3, never yantra's 1 |

## Findings that are not just "falls back"
1. (FIXED since: rtprobe/mklimits.sh, row 20.) **`YANTRA_RAM` was not validated natively** (row 20). `YANTRA_RAM=bogus` (yantra: ignored, default RAM, runs) and
   `YANTRA_RAM=100` (yantra: "RAM is 100 -- raise it", exit 1) both SEGFAULT the native image (rc 139) on all four hosts.
   t1-run's exit code for the second case is the shell's 139 only if the segfault is not swallowed; the message is lost.
2. **Output is held until exit** (row 30). Nothing a long-lived or interactive program prints is visible meanwhile.
3. **Step ceiling** (row 28): closed by H1 for the translated images (counter per block; exact halt when the limit falls in an effect-free run, else routed to yantra-run); fork-join images still have none.
4. ~~t1-run does not read `T1_RUN_REQUIRE_NATIVE`~~ it does now; probe.sh reads its refusal as a fallback (G1), and
   keeps `SAS_BIN=<empty dir>` as a second guard. The probe uses a throwaway `SASSEMBLY_NATIVE_CACHE`, so every translation
   here is by `anuvada.py` (the oracle, byte-identical to the .t1 translator: checked 2026-10-10 on 232 images x 4 targets, the translator itself among them).
5. A `BadAccess` for a store to an unmapped address is yantra-correct natively; since C5 (2026-10-10) native prints
   yantra's own `{ pc, addr }` form for it and t1-run keeps it (row 47). The `(...)` form now means only a store to a
   device native does not have yet (virtio-mmio slots 1..7 without the net device; slot 0 only without YANTRA_SCANOUT, L33), and t1-run still reruns on it.

## Deliberate differences already in LOWERING-x86-64.md (not holes)
- Self-modifying code is refused by name (a store into the PF_X segment); `fence.i` is therefore an exact no-op.
- A jump outside the translated code span, or to a pc not 4-aligned, halts as yantra's fetch does when yantra would halt
  there (row 50), and is refused by name only when yantra would execute the target.
- The sandbox is kept: guest RAM at its own address; MMIO goes to the host layer, a store below 2^31 never touches host memory.
- Counting (exact `steps`, StepLimit pc) is an attestation-mode option in the spike, not part of the runtime t1-run runs.
- Open items there still open: V is not lowered (refused until a V map is written); F/D lowering rules (NaN canonicalisation,
  NaN-boxing, saturating fcvt, RMM softfloat, fflags mapping) are specified but only `fld`/`fsd` are lowered in the product
  translator; macOS/Windows host layers need libSystem / PE imports (macOS done for UART+FINISHER; Windows pending).
- Both refusals above exit via t1-run's rerun, so they are invisible when yantra-run exists and fatal when it does not.

## The wasm32 target (`anuvada.py --target wasm32`, `wasm-host.mjs`)
An **integer subset**, probed against yantra-run over `examples/limits` (SAS-LIMITS-FIX L23):
- Runs as yantra does: I + M, `fence`, the sstatus write; `fld`/`fsd` with the sstatus.FS state kept at run time
  (`Undelivered { cause: 2 }`, kind 15, exactly when yantra traps: FS Off; an `fld` makes FS Dirty; csrs/csrc/csrw move
  it); `jal rd, .` as `SpinForever { pc }`, status 1 as yantra-run's (kind 16; refusals.sh compares it to yantra-run's rc); UART, FINISHER,
  the input channel, arguments. Same stdout, halt line and exit status, given the same `YANTRA_STEPS` and
  `YANTRA_RAM` (the defaults differ: the wasm budget is 2^62 and its default RAM smaller).
- Refused by name, exit 1: any other instruction (F/D arithmetic incl. `fnmadd`, `flw`/`fsw`, `fmv`, the Zicsr forms,
  vectors), at run time, at its pc: `native: not in milestone 1 (an instruction the translator does not lower)`.
  The decoder's names are classified in `WASM_REFUSED` (anuvada.py); a name in neither that set nor KNAMES stops the
  translation (no silent fall-through).
- A threaded image (SASTHRDS): yantra-run's own load refusal, byte for byte (`threads: N` and the refusal line on
  stderr, empty stdout, exit 1; the host ports `threads::scan`/`discover_tags`; e_entry rides in the module's `entry`
  custom section).
- Device MMIO below the load base (the WAIT register, the instruction counter, the socket, the virtio-mmio slots, the
  file window): refused by name, exit 1, `native: not in milestone 1 (a load|store to a device window ...) at pc N addr
  0x...`, where yantra answers (Finisher, `Device`, `Wait`). A load from slot 1 prints yantra's own `Device` halt. Every
  other address below base is yantra's `BadAccess`.
- `StepLimit` is exact per instruction in both modes (yantra's pc): the plain module runs the last accounting block's
  remaining words under a stop counter and refunds the unrun tail.
- Evidence: `agent-handovers/SAS-LIMITS-L23.md` (sweep of the probe, csr, refusal and instruction-space corpora).

## Per-host notes
| host | how probed | notes |
|---|---|---|
| x86_64-linux host | an x86_64 Linux host, `~/work/limits`, nice | has stage1/native cache elsewhere; probe uses a fresh cache and anuvada.py |
| aarch64-linux host | an ARM64 Linux host, `~/work/limits`, nice | anuvada.py `--target aarch64-linux` via t1-native-lib |
| aarch64-macos host | an ARM64 macOS host, `~/work/limits`, nice | anuvada.py, Mach-O output |
| Intel Mac (x86_64-macos) | local, scratchpad run, light | no yantra-run, no cargo; fallbacks refuse |
| windows | not probed | pending (sansos-27) |

## Reproduce
`tools/native-spike/limits/probe.sh [OUT.tsv]` on any host with bash and python3 (no cargo, no yantra-run, no cache needed);
it prints/writes `probe<TAB>class<TAB>detail`, class in native | native-halt | fallback | refused. native-halt = ran natively
and ended in a halt that t1-run does not rerun (BeyondRam, an input refusal). `limits/gen.awk` (OUTDIR) writes the probe ELFs alone.
