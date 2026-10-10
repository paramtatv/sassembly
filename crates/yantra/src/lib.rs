//! `yantra` — यन्त्र, "engine" — an RV64 interpreter small enough to run in a browser.
//!
//! # What this is for
//!
//! `F-001` claims *"apps that run in the browser and outside it."* The demo of that claim
//! is one artefact executing in two places. **This interprets the ELF `sadhana` already
//! emits**, so the thing running in the browser is byte-for-byte the thing QEMU runs —
//! not a re-implementation, not a transpile, and nothing re-assembled for the web.
//!
//! That was a decision, and it is the load-bearing one. A *Sassembly* interpreter would
//! have had to re-parse Devanagari source and could drift from what the assembler
//! actually produces; the claim would then be "a second implementation agrees with the
//! first", which is a weaker sentence and a larger job. Interpreting the ELF makes the
//! demo's honesty structural.
//!
//! # Scope, measured rather than guessed
//!
//! `spec/namaste.sas` assembles to **13 instructions**, `spec/bare-metal.sas` to **19**.
//! Between them they use **eight opcodes**:
//!
//! ```text
//! LUI  AUIPC  OP-IMM  OP  LOAD  STORE  BRANCH  JAL
//! ```
//!
//! `F-001b` adds two more families, chosen because neither needs a host-side decision:
//!
//! ```text
//! MISC-MEM  (fence, fence.i)      AMO  (lr, sc, and the nine amo* operations, .w and .d)
//! ```
//!
//! `F-001c1` adds the half of `SYSTEM` that a machine with no privilege levels can mean:
//!
//! ```text
//! SYSTEM  (ecall as an SBI v0.1 call, ebreak)
//! ```
//!
//! `F-001c2a` adds the machine state behind that half — the six supervisor trap registers,
//! the six CSR instructions, `ebreak` delivered to `stvec`, and `sret`:
//!
//! ```text
//! SYSTEM  (csrrw csrrs csrrc csrrwi csrrsi csrrci, ebreak as a trap, sret)
//! ```
//!
//! `F-001c2b1` adds the last of the ten 32-bit `SYSTEM` rows, and the machinery it needs:
//! `satp`, the Sv39 page-table walker every fetch, load and store now goes through, and
//! page faults delivered to `stvec` like any other exception.
//!
//! ```text
//! SYSTEM  (sfence.vma)
//! ```
//!
//! `F-001c2b2` adds **no instruction at all** — it adds the mode the ones above execute
//! in. `sret` now enters user mode when `sstatus.SPP` says the trap came from there, the
//! walker checks the `U` bit in both directions with `SUM` and `MXR` behind it, an `ecall`
//! from user mode is exception 8 delivered to `stvec` rather than an SBI call, and every
//! privileged instruction a user program executes is exception 2. See [`Privilege`].
//!
//! That is the whole instruction set this file implements. It is not RV64GC: it has no
//! interrupts, and no compressed encodings (floating point arrived later: OP-FP and
//! the float loads and stores, `fp.rs`). It gained the **M extension** on 2026-09-05 — the eight multiply, divide
//! and remainder forms at funct7 `0x01` — so it is RV64IMA rather than RV64IA, and it
//! answers a zero divisor and the one division overflow BY VALUE, as the specification
//! requires and as a host language's own `/` would not. **An unimplemented
//! instruction halts and says which one**, at which address, rather than being skipped —
//! a VM that silently ignores what it does not know produces a plausible wrong answer,
//! which is worse than a stop.
//!
//! The two new families share one justification: **this machine has a single hart and no
//! caches**, so a fence has nothing to order and an atomic has nothing to race. Both are
//! implemented as what they *mean* on such a machine rather than skipped — a fence
//! executes as a no-op, and an `amoadd` really does read, add and write back. The
//! difference matters: skipping an `amoadd` loses the write.
//!
//! # What it can and cannot run, measured across every program in the tree
//!
//! Of the 35 `spec/*.sas` programs that assemble at `2555f4e`, **four ran on the spike's
//! instruction set** — `namaste`, `bare-metal`, `jump-table`, `lib-mudraka`. The other 31
//! needed `SYSTEM` (31 of them), `FENCE` (7) or `AMO` (2). Three families, not forty.
//!
//! `F-001b` closed two of the three. **That figure of four is NOT re-measured here and is
//! not claimed to have moved**: `SYSTEM` appears in all 31 remaining programs, so unless a
//! program needed a fence and nothing else, the count is unchanged — and which it is takes
//! assembling all 35, which this row did not do. The honest statement is about families,
//! not programs: two of three are gone.
//!
//! `SYSTEM` is the third, and it is not an increment. It means `ecall` and CSR access, and
//! an `ecall` in a browser has no OpenSBI behind it, so each one needs a host-side decision
//! about what it *means*. `F-001c1` answers it for `ecall` and `ebreak` and **refuses**
//! CSR access; the state behind a CSR file is `F-001c2`.
//!
//! # What an `ecall` means here, and why a CSR is a refusal
//!
//! On QEMU an `ecall` from an S-mode payload reaches OpenSBI. In a browser there is
//! nothing behind it, so this machine *is* the firmware: an `ecall` is read as an **SBI
//! v0.1 legacy call** — extension id in `a7`, argument in `a0` — and exactly two are
//! implemented, because exactly two are used. `sbi_console_putchar` (EID 1) puts its byte
//! through the same [`Output`] the UART store does, and `sbi_shutdown` (EID 8) stops the
//! machine. **Any other EID halts naming the extension and function id**; it does not
//! return 0, which is the plausible-wrong-answer failure this VM exists to avoid — a
//! program whose `sbi_set_timer` silently succeeded would run on and be wrong.
//!
//! A CSR access halted naming the CSR, and that was a **decision, not a gap**: a sparse
//! map with no trap delivery behind it would accept the write to `stvec` and then run on
//! as if the vector had never been written. `F-001c2a` builds the delivery instead, and
//! **only as far as it can honour** — see [`Csrs`] for the measured list. `satp` was
//! refused until `F-001c2b1`, which lifted the refusal the only way it is allowed to be
//! lifted: by building the walker behind it. See [`Machine::translate`], and
//! `tests/paging.rs` for the nine ways a walk can fail.
//!
//! `F-001c2b3` is the last three — `sie`, `sip` and `time` — and it lifts them the same
//! way: by building the clock and the interrupt they describe, not by storing the bits.
//! [`Machine::time`] counts **retired instructions**, `sbi_set_timer` arms
//! [`Machine::timecmp`] against it, and `sip.STIP` is a *function* of the two rather than
//! stored state — which is exactly why software cannot write it. See
//! [`Machine::interrupt`].
//!
//! **`wfi` is still [`Halt::Unimplemented`], and that is a measurement rather than an
//! omission.** No row of `spec/encodings-riscv64.tsv` names it, so nothing this tree
//! assembles can emit one; `spec/timer.sas` waits in a `jal x0, .` for that reason and
//! [`Halt::SpinForever`] is what a wait becomes here. Decoding an instruction the
//! encoding table does not carry would be the first opcode in this crate with no oracle
//! behind it.
//!
//! **`ecall` is still an SBI call, and that is the spec's own answer rather than a
//! convenience.** An environment call from S-mode is the one exception `medeleg` cannot
//! delegate to S-mode: on hardware it always goes up to the firmware. A *breakpoint* can
//! be delegated and OpenSBI delegates it, so `ebreak` goes to the program's `stvec`. The
//! two instructions differ here in exactly the way they differ on the machine these
//! programs were written for.
//!
//! **36 of the 37 programs in `spec/` are OS proofs**, each referenced by a
//! `tools/check-*.sh`. The 37th is `spec/atithi.sas`, which `F-001e3` wrote against
//! ADR-0015 and which this crate's own loader and supervisor run — so the repository
//! contains exactly one Sassembly *application*. Since `F-001g` the demo page runs it:
//! [`host::host`] is the arrangement, `yantra_host` is the one export that spends it, and
//! the page shows the surface the program was granted in a **different pane** from the
//! machine's own UART, which stays empty because an application cannot address a device
//! (ADR-0015 A4). A page that merged the two would say the opposite of what the ABI
//! guarantees. So the claim is no longer only "one artefact in two places, byte for
//! byte" — it is that one, and an app in a browser.
//!
//! ## How to decode a program, and how not to
//!
//! **Decode only the text section, at the size `sadhana` reports.** The first measurement
//! of the numbers above decoded each whole `PT_LOAD` segment and concluded that only two
//! programs were runnable, with a missing-opcode list containing `0x72`, `0x6d`, `0x6e`
//! and `0x20`.
//!
//! Those are ASCII `r`, `m`, `n` and space. It was disassembling `namaste`'s message
//! string. **The tell was that the opcodes spelled words** — a wrong answer that looked
//! exactly like a finding, and would have been reported as one had the data been less
//! legible. A segment holds code *and* data; only the first `text bytes` of it are
//! instructions.
//!
//! # The bridge, which is the interesting part
//!
//! On bare metal, a store to `0x1000_0000` reaches a UART and a character appears on a
//! serial line. In a browser that same store has to become a character on a page. So the
//! memory-mapped store IS the bridge, and it is three lines: [`Machine::store`] routes
//! writes to that address into [`Output::putc`] instead of RAM.
//!
//! No third-party crate is used, per doc 03 §1 — `wasm-bindgen` included. The browser ABI
//! lives in `crates/yantra-wasm` rather than here, so that THIS crate — the whole
//! interpreter — keeps `forbid(unsafe_code)`. See that crate for why the split is real
//! and not bookkeeping.

#![forbid(unsafe_code)]

pub mod fp;
pub mod gpu;
pub mod host;
pub mod input;
pub mod loader;
pub mod patra;
pub mod process;
pub mod profile;
pub mod smp;
pub mod socket;
pub mod supervisor;
pub mod threads;
pub mod vector;
pub mod virtio_gpu;
pub mod virtio_mmio;
pub mod virtqueue;

/// ── the limits a program runs under ──────────────────────────────────────
///
/// STATED ONCE HERE AND IMPORTED, NOT RESTATED. `W-262`, 2026-09-05: five
/// places carried their own copy of these two numbers and four of them a doc
/// comment ASSERTING they matched `yantra-run` — one original and four
/// transcriptions with a comment instead of a check. They import these now.
/// A caller that wants different limits passes them to `load_elf`/`run`
/// directly; the wasm entry point already does.
/// The RAM a program is given, and it is MEASURED rather than round.
///
/// `1 << 20` — one mebibyte — until 2026-09-05, and one pass of the emitted
/// lexer over the corpus's largest source DOES NOT FIT IN IT:
///
/// ```text
/// encode.t1                        426,178 octets, 5,455 lines
/// lexed under sadhana               12,686 tokens
/// token record चिह्नक                6 fields × 8 = 48 bytes
/// token arena     12,686 × 48    =   608,928 bytes     595 KiB
/// source text                    =   426,178 bytes     416 KiB
/// LEXER FOOTPRINT                =  1,035,106 bytes  1,011 KiB
/// RAM at 1 << 20                 =  1,048,576 bytes  1,024 KiB
/// headroom                              13,470 bytes     1.3 %
/// ```
///
/// A slice is ONE address-sized word in this ABI (`abi.rs:80-86`, ADR-0026) and
/// not a fat pointer, which is why the record is 48 bytes and not 56.
///
/// MEASURED UNDER THE INTERPRETER, AND THAT IS THE SAME NUMBER. Nothing runs the
/// emitted lexer on this machine yet — `W-254`'s first step is what will — so the
/// footprint cannot be taken on this side of the emitter. It does not need to be:
/// the arenas `sadhana` fills lexing that source are the arenas the emitted code
/// would fill, so the quantity is the same one taken in a different place.
///
/// THE FIGURE IS SET FAR ABOVE ANY MEASURED NEED, ON PURPOSE. 1,011 KiB is the
/// only footprint anyone has measured — one lexer pass. The whole chain in one
/// image adds the parse tree, the symbol tables, the IR and the emit buffers over
/// those same 12,686 tokens, and NONE of that has been measured. So this is
/// roughly twenty times the one number we have, chosen so that memory is not the
/// thing that fails while those numbers are still unknown.
///
/// THE COSTS ARE NOT SYMMETRICAL, WHICH IS THE WHOLE ARGUMENT. `ram` becomes
/// `vec![0; ram]` — a zero-initialised host allocation, lazily backed, so memory
/// a program never touches costs essentially nothing resident. The cost of
/// shortfall is a person reading `BadAccess` as a wild pointer in the emitted
/// lexer and losing hours to a defect that is not there.
///
/// NOT A POWER OF TWO, DELIBERATELY. `20 * (1 << 20)` is the owner's figure and
/// is written as one so the next reader sees a DECISION rather than an artefact
/// of rounding. Do not tidy it.
///
/// AND RAM IS NOT THE RUNAWAY GUARD — `DEFAULT_STEPS` BELOW IS. A program that loops
/// forever is caught by `StepLimit` in bounded time however much memory exists.
/// Sizing memory small does not catch bugs; it converts "this pass is larger than
/// we thought" into "bad access at an address", and only one of those failures
/// tells you anything. That is the argument against tightening this later.
pub const DEFAULT_RAM: usize = 20 * (1 << 20);

/// Headroom above an image's last segment: the stack and whatever the startup
/// places past the loaded extent.
pub const RAM_HEADROOM: usize = 16 << 20;

/// Which part of a segment's span the loader insists RAM covers.
///
/// A `PT_LOAD` declares two sizes: `p_filesz`, the octets actually in the file, and
/// `p_memsz`, the span it occupies once loaded. The difference is `.bss`, zeroed
/// rather than copied. The `.t1` compiler declares its heap that way — 512.0 MiB of
/// `.bss` tail (`yantrotsarjana.t1:2070`) which costs no file octets and, through
/// `ram_for`, demanded the RAM anyway.
///
/// **W-363's SIZE HALF: THAT DEMAND IS A DECLARATION, NOT A REQUIREMENT, and this
/// is the knob that says which reading a loader takes.** A walker's high water over
/// a whole 154-second recording is 6.3 MiB against a 512.1 MiB ask — 82x — and a
/// phone browser will not hand out 540 MB of linear memory where it would hand out
/// 32. Nothing else stands between that decoder and a phone: T-102 already holds in
/// the browser to the instruction, with the output digest identical to native.
///
/// Five things could have made the small reading unsound and none does, measured
/// before this existed: RAM is `vec![0; ram]`, so `.bss` zeroing is free at any size
/// and a program never reads un-zeroed heap; the loader copies only `filesz`; the
/// bound is read in exactly one place, below; the stack is 64 KiB file-backed in
/// `ॱदत्त` rather than growing down from `base + memsz`; and `ir.t1:634` answers
/// `region + offset`, so the heap grows UP and nothing addresses downward from the
/// top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    /// `memsz` — the whole declared span, `.bss` tail included.
    ///
    /// THE DEFAULT, AND EVERY EXISTING CALLER KEEPS IT. A program that outgrows its
    /// RAM is refused AT LOAD, naming the segment and the shortfall, which is the
    /// failure the margin on `DEFAULT_RAM` argues for: a load-time refusal says
    /// "this is larger than you thought" where a mid-run fault says "bad access at
    /// an address", and only one of those tells you anything.
    Declared,
    /// `filesz` — only the file-backed octets, so a declared `.bss` tail may run
    /// past the end of RAM.
    ///
    /// FOR A CALLER WITH A MEMORY BUDGET IT CANNOT EXCEED — the browser, where
    /// linear memory is the constraint. The trade is explicit: a program that
    /// actually reaches into the truncated tail is no longer refused at load, it
    /// halts mid-run. That is tolerable here and nowhere else, because the halt it
    /// takes is `Halt::BeyondRam`, which NAMES ITSELF and carries the address and
    /// the RAM size — unlike `BadAccess`, it does not have to be diagnosed.
    FileBacked,
}

/// RAM for an image, SIZED FROM THE IMAGE with `DEFAULT_RAM` as the floor.
///
/// The image's own segments say how far it reaches — its record region is a
/// `PT_LOAD` with a `memsz` and no bytes in the file — and a compiler's image
/// reaches further than twenty megabytes once its region is sized for a full
/// native self-compile (320 MiB since 2026-09-13). This is the ONE statement of
/// that sizing: `yantra-run` uses it, and so does the census's runner, which
/// loaded every image at `DEFAULT_RAM` until the region raise stopped every
/// census row at `load` ("segment needs 335,616,120 bytes and RAM is
/// 20,971,520") while `yantra-run` ran the same image — two loaders, two
/// answers. An unparseable image answers the floor and lets `load_elf` name
/// the defect.
pub fn ram_for(image: &[u8]) -> usize {
    ram_for_span(image, Span::Declared)
}

/// `ram_for`, reading the image's extent under an explicit [`Span`].
///
/// `Span::Declared` is `ram_for` itself and is what every caller in this repository
/// takes. `Span::FileBacked` measures the extent over `filesz` instead, so a
/// declared `.bss` tail does not set the floor — the sizing counterpart to the
/// loader's bound, and the reason a caller can ask for 20 MiB where the declaration
/// asks for 512.
///
/// The `DEFAULT_RAM` floor applies to both: a budget below it is the caller's to
/// pass to `load_elf_spanning` directly, since a floor that an argument silently
/// raised would make the budget a suggestion.
pub fn ram_for_span(image: &[u8], span: Span) -> usize {
    let extent = loader::Program::parse(image)
        .ok()
        .and_then(|p| {
            let base = p.segments.iter().map(|s| s.vaddr).min()?;
            let end = p
                .segments
                .iter()
                .map(|s| {
                    let reach = match span {
                        Span::Declared => s.memsz,
                        Span::FileBacked => s.filesz,
                    };
                    s.vaddr + reach as u64
                })
                .max()?;
            usize::try_from(end - base).ok()
        })
        .unwrap_or(0);
    DEFAULT_RAM.max(extent + RAM_HEADROOM)
}

/// The steps a program is given before the machine stops it.
///
/// UNMEASURED AND DELIBERATELY LEFT AT ITS ORIGINAL VALUE. The two limits are not
/// symmetrical: `Halt::StepLimit` NAMES ITSELF, so a program that outruns this
/// tells you it outran this. `BadAccess` does not, which is why the memory limit
/// was the half worth fixing without a measurement in hand.
///
/// A FLAG FOR WHOEVER MEASURES IT, AND IT IS ARITHMETIC ON A GUESSED CONSTANT:
/// 12,686 tokens at somewhere between a hundred and a thousand machine
/// instructions each puts one lexer pass near 1.3–13 million steps, so a million
/// is likely too small by about an order of magnitude. DO NOT SIZE FROM THAT.
/// Measure the pass and replace this with the figure and its margin.
pub const DEFAULT_STEPS: u64 = 1_000_000;

/// Where the SiFive test finisher lives on QEMU `virt`, and what the programs write to it.
pub const FINISHER: u64 = 0x0010_0000;
/// The NS16550A data register on QEMU `virt`. A store here is a character.
pub const UART: u64 = 0x1000_0000;
/// The WAIT register (`W-370`, ADR-0040 Option C). A store here halts the machine with
/// [`Halt::Wait`], keeping every register and octet, and the next [`Machine::run`]
/// continues at the instruction after the store. The value stored is ignored.
///
/// WHY THIS ADDRESS. It is the first octet past the UART's register window — QEMU `virt`
/// gives the NS16550A `0x100` octets at `UART` — and below `virtio_mmio::VIRTIO_MMIO_BASE`
/// (`0x1000_1000`), so it is inside no window this machine or QEMU decodes: not the
/// UART's, not the eight virtio slots', not `patra`'s (`0x1000_2000..=0x1000_201f`, inside
/// slot 1). Not next to `FINISHER`: QEMU's test device owns the whole page at
/// `0x0010_0000` and its neighbour `0x0010_1000` is the RTC, so a store there on QEMU
/// would be swallowed or would set an alarm. Here it reaches nothing, and a program run
/// on QEMU faults rather than silently doing something else — `patra`'s rule for its own
/// addresses.
pub const WAIT: u64 = 0x1000_0100;
/// The retired-instruction COUNTER (`W-374`, ADR-0040 "The counter"): an integer load
/// here answers [`Machine::time`] at that instruction — the instruction doing the reading
/// already counted, so two reads with `k` instructions between differ by `k + 1`. Eight
/// octets: the low word at `COUNTER`, the high word at `COUNTER + 4`, as two 32-bit reads
/// of a 64-bit register take them; `ld` takes it whole.
///
/// DIAGNOSTIC ONLY. A program that branches on it changes behaviour whenever the compiler
/// changes a count, and if the compiler's own image read it Stage 2 could differ from
/// Stage 1 by self-reference — so `W-372`'s ratchet refuses a fixpoint whose Stage 2 read
/// it (`yantra-run` reports every read; `tools/fixpoint.sh` refuses on the report).
///
/// A LOAD ONLY: a store is `BadAccess` (it sits below RAM like every device address), as
/// are a fetch, an AMO and a floating-point load — none of them is how a diagnostic read
/// is spelled, and refusing them keeps the one path that answers the one the `.t1` seam
/// (`अष्टकॱउपकरणचतुरष्टकाहारः`, `W-350`) lowers to: `lw` at the absolute address.
///
/// WHY THIS ADDRESS: the next aligned doubleword after [`WAIT`], in the same unclaimed
/// stretch between the UART's window and `virtio_mmio::VIRTIO_MMIO_BASE` — see `WAIT`.
pub const COUNTER: u64 = 0x1000_0108;
/// THE SOCKET DEVICE's window (`W-377`, `docs/adr/0040-addendum-w377-sockets.md` §1):
/// sixteen octets, NEXT / RX / TX and a reserved fourth word — see [`socket`].
///
/// WHY THIS ADDRESS: the next aligned stretch after [`COUNTER`]'s eight octets, in the
/// same unclaimed gap between the UART's window and `virtio_mmio::VIRTIO_MMIO_BASE` — see
/// [`WAIT`]. Like every device address it lies below `DEVICE_TOP`, which it does not move.
pub const SOCK: u64 = 0x1000_0110;

/// EVERY ADDRESS A DEVICE ARM DECODES (`W-382`), as `(the name the arm spells, first
/// address, length)`. Every device this machine answers is matched on the access's START
/// address — exactly (`UART`, `FINISHER`, `WAIT`, the four `patra` stores), by a short
/// window (`COUNTER`'s eight octets, read in `Machine::counter`) or by the virtio-mmio
/// slots (`virtio_mmio::slot_of`, in both `load_walk` and `store_walk`).
///
/// HAND-KEPT AGAINST THE ARMS, AND CHECKED AGAINST THEM: the arms compare against the
/// named constants, not against this table, so `tests/w382_decode_cache.rs`
/// (`device_top_covers_every_device`) reads the bodies of `load_walk`, `store_walk` and
/// `counter` out of this file and refuses any address an arm names that is missing here.
/// ADDING A DEVICE MEANS ADDING ITS ROW HERE.
pub const DEVICE_WINDOWS: [(&str, u64, u64); 10] = [
    ("UART", UART, 1),
    ("WAIT", WAIT, 1),
    ("COUNTER", COUNTER, 8),
    ("SOCK", SOCK, socket::SOCK_LEN),
    ("FINISHER", FINISHER, 1),
    ("patra::PATRA_PATH", patra::PATRA_PATH, 1),
    ("patra::PATRA_BUFFER", patra::PATRA_BUFFER, 1),
    ("patra::PATRA_GO", patra::PATRA_GO, 1),
    ("patra::PATRA_PUT", patra::PATRA_PUT, 1),
    (
        "virtio_mmio::slot_of",
        virtio_mmio::VIRTIO_MMIO_BASE,
        virtio_mmio::VIRTIO_MMIO_SLOTS * virtio_mmio::VIRTIO_MMIO_STRIDE,
    ),
];

/// ONE PAST THE HIGHEST ADDRESS IN [`DEVICE_WINDOWS`] (`W-382`), computed from it: an
/// access whose address is at or above this line can reach no device arm and is a plain
/// RAM access or a bound refusal. `load`, `store` and the fetch test it FIRST and go
/// straight to `mem` with one word-wide little-endian access; below it they take the
/// original arm-by-arm walk unchanged.
///
/// RAM starts at `0x8000_0000` in every image this tree links, far above this line, so
/// the fast path is the path every ordinary access takes. A machine built by hand with
/// RAM below the line still answers correctly: those accesses simply take the walk.
pub const DEVICE_TOP: u64 = {
    let mut top = 0;
    let mut i = 0;
    while i < DEVICE_WINDOWS.len() {
        let end = DEVICE_WINDOWS[i].1 + DEVICE_WINDOWS[i].2;
        if end > top {
            top = end;
        }
        i += 1;
    }
    top
};

/// Read `width` octets little-endian at `at`, or `None` when they do not lie wholly
/// inside `mem` — in which case the caller takes its original path, which refuses by name.
#[inline(always)]
fn ram_read(mem: &[u8], at: u64, width: usize) -> Option<u64> {
    let at = usize::try_from(at).ok()?;
    if at >= mem.len() || width > mem.len() - at {
        return None;
    }
    let s = &mem[at..at + width];
    Some(match width {
        1 => u64::from(s[0]),
        2 => u64::from(u16::from_le_bytes([s[0], s[1]])),
        4 => u64::from(u32::from_le_bytes([s[0], s[1], s[2], s[3]])),
        8 => u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]),
        _ => {
            let mut v: u64 = 0;
            for (i, b) in s.iter().enumerate() {
                v |= u64::from(*b) << (8 * i);
            }
            v
        }
    })
}

/// Sign-extend a `width`-octet value read from RAM, as `load` always has.
#[inline(always)]
const fn sign_extend(v: u64, width: usize) -> u64 {
    if width < 8 {
        let sign = 1u64 << (width * 8 - 1);
        if v & sign != 0 {
            return v | !((1u64 << (width * 8)) - 1);
        }
    }
    v
}

/// The instruction forms [`Machine::run`] executes from a cached decoding (`W-382`).
/// Exactly the integer forms `step_inner` itself executes on RAM, one variant per form so
/// the dispatch is one match rather than an opcode match and a `funct3` match nested in
/// it. EVERYTHING ELSE IS [`Op::Slow`] — floating point, vector, atomics, fences, SYSTEM,
/// and every word `step_inner` refuses — and goes to `step_inner` unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Slow,
    Lui,
    Auipc,
    Jal,
    Jalr,
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,
    Lb,
    Lh,
    Lw,
    Ld,
    Lbu,
    Lhu,
    Lwu,
    Sb,
    Sh,
    Sw,
    Sd,
    Addi,
    Slli,
    Srli,
    Srai,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi,
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu,
}

/// One word's decoding: its form, its three register fields and the one immediate its
/// format uses, sign-extended exactly as `step_inner` extends it. Sixteen octets.
#[derive(Debug, Clone, Copy)]
struct Decoded {
    /// The word this was decoded FROM — the cache's whole validity test.
    word: u32,
    op: Op,
    rd: u8,
    rs1: u8,
    rs2: u8,
    imm: u64,
}

impl Decoded {
    /// Decode one word. A pure function of `word`: the same field extraction and the same
    /// immediates as `step_inner`, and [`Op::Slow`] for every word whose form `step_inner`
    /// alone executes or refuses.
    fn of(word: u32) -> Self {
        let opcode = word & 0x7f;
        let funct3 = (word >> 12) & 0x7;
        let funct7 = word >> 25;
        let imm_i = ((word as i32) >> 20) as i64 as u64;
        let imm_s =
            ((((word & 0xfe00_0000) as i32) >> 20) as i64 as u64) | u64::from((word >> 7) & 0x1f);
        let imm_u = ((word & 0xffff_f000) as i32) as i64 as u64;
        let imm_b = {
            let v = (((word >> 31) & 1) << 12)
                | (((word >> 7) & 1) << 11)
                | (((word >> 25) & 0x3f) << 5)
                | (((word >> 8) & 0xf) << 1);
            if v & 0x1000 != 0 {
                v as u64 | !0x1fff
            } else {
                u64::from(v)
            }
        };
        let imm_j = {
            let v = (((word >> 31) & 1) << 20)
                | (((word >> 12) & 0xff) << 12)
                | (((word >> 20) & 1) << 11)
                | (((word >> 21) & 0x3ff) << 1);
            if v & 0x0010_0000 != 0 {
                v as u64 | !0x001f_ffff
            } else {
                u64::from(v)
            }
        };
        let (op, imm) = match opcode {
            0x37 => (Op::Lui, imm_u),
            0x17 => (Op::Auipc, imm_u),
            0x6f => (Op::Jal, imm_j),
            // `step_inner`'s JALR does not read `funct3`, so neither does this.
            0x67 => (Op::Jalr, imm_i),
            0x63 => (
                match funct3 {
                    0x0 => Op::Beq,
                    0x1 => Op::Bne,
                    0x4 => Op::Blt,
                    0x5 => Op::Bge,
                    0x6 => Op::Bltu,
                    0x7 => Op::Bgeu,
                    _ => Op::Slow,
                },
                imm_b,
            ),
            0x03 => (
                match funct3 {
                    0x0 => Op::Lb,
                    0x1 => Op::Lh,
                    0x2 => Op::Lw,
                    0x3 => Op::Ld,
                    0x4 => Op::Lbu,
                    0x5 => Op::Lhu,
                    0x6 => Op::Lwu,
                    _ => Op::Slow,
                },
                imm_i,
            ),
            0x23 => (
                match funct3 {
                    0x0 => Op::Sb,
                    0x1 => Op::Sh,
                    0x2 => Op::Sw,
                    0x3 => Op::Sd,
                    _ => Op::Slow,
                },
                imm_s,
            ),
            // OP-IMM: all eight `funct3` values are defined, and — as in `step_inner` —
            // a shift reads only bit 30 of `funct7` (to tell SRAI from SRLI) and the low
            // six bits of the immediate as its amount.
            0x13 => (
                match funct3 {
                    0x0 => Op::Addi,
                    0x1 => Op::Slli,
                    0x5 if funct7 & 0x20 == 0 => Op::Srli,
                    0x5 => Op::Srai,
                    0x2 => Op::Slti,
                    0x3 => Op::Sltiu,
                    0x4 => Op::Xori,
                    0x6 => Op::Ori,
                    _ => Op::Andi,
                },
                imm_i,
            ),
            0x33 => (
                match (funct3, funct7) {
                    (0x0, 0x00) => Op::Add,
                    (0x0, 0x20) => Op::Sub,
                    (0x1, 0x00) => Op::Sll,
                    (0x2, 0x00) => Op::Slt,
                    (0x3, 0x00) => Op::Sltu,
                    (0x4, 0x00) => Op::Xor,
                    (0x5, 0x00) => Op::Srl,
                    (0x5, 0x20) => Op::Sra,
                    (0x6, 0x00) => Op::Or,
                    (0x7, 0x00) => Op::And,
                    (0x0, 0x01) => Op::Mul,
                    (0x1, 0x01) => Op::Mulh,
                    (0x2, 0x01) => Op::Mulhsu,
                    (0x3, 0x01) => Op::Mulhu,
                    (0x4, 0x01) => Op::Div,
                    (0x5, 0x01) => Op::Divu,
                    (0x6, 0x01) => Op::Rem,
                    (0x7, 0x01) => Op::Remu,
                    _ => Op::Slow,
                },
                0,
            ),
            _ => (Op::Slow, 0),
        };
        Decoded {
            word,
            op,
            rd: ((word >> 7) & 0x1f) as u8,
            rs1: ((word >> 15) & 0x1f) as u8,
            rs2: ((word >> 20) & 0x1f) as u8,
            imm,
        }
    }
}

/// THE DECODED-INSTRUCTION CACHE (`W-382`): a direct-mapped table of [`Decoded`] entries
/// indexed by the fetch's PHYSICAL address, local to one [`Machine::run`].
///
/// AN ENTRY IS VALID EXACTLY WHEN ITS `word` EQUALS THE WORD NOW IN RAM AT THE FETCH.
/// The fetch still reads the word every step (one 32-bit RAM read); what the cache spares
/// is turning it into a form. There is therefore no invalidation to get wrong: a store
/// into code, an AMO, a device writing into RAM, the host rewriting `mem` between two
/// runs — each simply changes the word, and the next fetch re-decodes it. Two addresses
/// sharing a slot evict each other and cost a decode, never a wrong instruction.
///
/// Every slot starts as the decoding of word 0, which is itself correct ([`Op::Slow`]),
/// so an empty slot needs no separate flag.
struct DecodeCache {
    entries: Vec<Decoded>,
}

impl DecodeCache {
    /// 2^12 entries, 64 KiB. MEASURED (fixpoint Stage 1 image, first 5e8 steps,
    /// A Linux x86-64 host): 2^10, 2^12, 2^14 and 2^16 entries ran within 10% of each other,
    /// the smaller tables slightly ahead — the host's 1 MiB L2 is shared with the
    /// program's own data, and a decode is cheap next to a miss. At 2^16 the run decoded
    /// only 10,783 times in 5e8 steps.
    const BITS: u32 = 12;
    const MASK: usize = (1 << Self::BITS) - 1;
    /// A run shorter than this steps without the table — allocating and filling 64 KiB
    /// to run a handful of instructions costs more than it saves.
    const MIN_BUDGET: u64 = 1 << 16;

    fn new() -> Self {
        DecodeCache {
            entries: vec![Decoded::of(0); 1 << Self::BITS],
        }
    }
}

/// Where a store to the UART goes.
///
/// A trait rather than a callback field so the native tests and the wasm build share one
/// interpreter. The browser's implementation appends to a DOM node; the test's collects
/// into a `Vec` and asserts on it.
pub trait Output {
    /// One octet, exactly as the program wrote it. Not decoded — the program is emitting
    /// UTF-8 one byte at a time and the joining is the host's business, not the VM's.
    fn putc(&mut self, byte: u8);
    /// A store landed at RAM offset `at`, `width` octets wide. A no-op by
    /// default; a diagnostic sink keeps the high-water mark from it, so a run
    /// can say how much RAM it actually touched — how the record region's
    /// bound is MEASURED (its growth arm leaks old blocks) rather than guessed.
    fn stored(&mut self, at: usize, width: usize) {
        let _ = (at, width);
    }
    /// The program read [`COUNTER`] (`W-374`). A no-op by default; `yantra-run` counts
    /// these and says so, which is what `W-372`'s ratchet in `tools/fixpoint.sh` reads.
    fn counter_read(&mut self) {}
    /// The program stored `byte` at the socket's TX register (`W-377`). A no-op by
    /// default: the HOST decides where the octet goes — `yantra-run --listen` writes it to
    /// the connection at once, a replay collects it — so this crate never sees a stream.
    fn sent(&mut self, byte: u8) {
        let _ = byte;
    }
}

impl Output for Vec<u8> {
    fn putc(&mut self, byte: u8) {
        self.push(byte);
    }
}

/// Why the machine stopped. Every variant is a *statement*, never a shrug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Halt {
    /// The program wrote the finisher. `code` is the exit status it asked for:
    /// `0x5555` means success, and `0x3333 | (n << 16)` means failure with status `n`.
    /// Both fields are the register's full 64 bits (`W-341`): the fields were `u32`
    /// and the arm read `value as u32` then `>> 16`, so the status was sixteen bits
    /// and error 70000 was reported as 4464 with no warning.
    ///
    /// THE NATIVE REFUSALS ARE FAIL-FORM STATUSES (`W-381`, 2026-10-06): `ir.t1`
    /// writes `0x3333 | code << 16` for an out-of-bounds indexed READ
    /// (`रिक्तखण्डपठननिषेधः`, code `0x355`, 853 — `W-355`'s fresh run first, every
    /// index outside the length since `W-381` stage 4, −1 included), `W-359`'s store
    /// past a guarded parameter (`प्राचलसीमानिषेधः`, `0x359`, 857), `V-008`'s unequal
    /// vector lengths (`व्यूहदैर्घ्यनिषेधः`, `0x35a`, 858), `V-009` (ii)'s aliased
    /// matrix result (`अध्यासप्रतिषेधः`, `0x35b`, 859) and `W-381` stage 4's
    /// out-of-bounds STORE (`सीमातीतलेखननिषेधः`, `0x35d`, 861), so this machine reports `Finisher { status: Some(code) }` — as it does the
    /// stack canary's `0x353B`. They used to be the RAW codes, reported here with
    /// `status: None`; QEMU's sifive-test device (and hardware) ignores a word that is
    /// neither form, and the program RAN ON there. A program RETURNING one of these
    /// numbers now writes the same word: the status space is the program's (the canary's
    /// trade, accepted for these by the coordinator, 2026-10-06).
    Finisher {
        /// The raw value written.
        value: u64,
        /// Decoded exit status, or `None` if the value is not a form the finisher defines.
        status: Option<u64>,
    },
    /// A `jal x0, .` to itself — the idiom both demo programs end with.
    SpinForever {
        /// Where it is spinning.
        pc: u64,
    },
    /// An instruction this interpreter does not implement. Named, so the next person
    /// extends the right thing instead of guessing.
    Unimplemented {
        /// Address of the offending word.
        pc: u64,
        /// The raw instruction word.
        word: u32,
        /// Its major opcode.
        opcode: u8,
    },
    /// A load or store outside mapped memory and outside the two device addresses — or an
    /// atomic whose address is not naturally aligned. The A extension requires alignment
    /// and does not define what a misaligned atomic means, so this machine performs no
    /// access rather than inventing one; the address really is the thing wrong with it.
    BadAccess {
        /// Where the access came from.
        pc: u64,
        /// The address it tried to touch.
        addr: u64,
    },
    /// The address was ABOVE the RAM this machine was given — the program asked
    /// for more memory than it has, rather than touching a wild address.
    ///
    /// SEPARATE FROM `BadAccess` BECAUSE THE TWO SEND A READER TO DIFFERENT
    /// PLACES. `W-262`, 2026-09-05: both used to answer `BadAccess { pc, addr }`,
    /// naming an address and nothing else — so a program that outgrew its RAM
    /// looked exactly like a program with a wild pointer, and the next person
    /// went hunting a defect in the compiler that emitted it. One pass of the
    /// emitted lexer over the corpus's largest source needs 1,011 KiB of arenas,
    /// so this is the halt that pass would have produced against the old 1 MiB.
    /// It now says which limit was crossed and what the limit was.
    BeyondRam {
        /// Where the access came from.
        pc: u64,
        /// The address it tried to touch.
        addr: u64,
        /// The RAM this machine was given, in bytes — the limit that was crossed.
        ram: usize,
    },
    /// A load or store inside the virtio-mmio window (`virtio_mmio`) that the device
    /// does not answer — a register not modelled yet, a read-only register written, a
    /// width the specification forbids, or a slot with no device in it.
    ///
    /// SEPARATE FROM `BadAccess` FOR THE REASON `BeyondRam` IS: the window sits below
    /// RAM, so without this arm a driver touching an unbuilt register halted with
    /// `BadAccess`, the same statement as a wild pointer, and sent its reader hunting
    /// the compiler. `why` says which of the four it was.
    Device {
        /// Where the access came from.
        pc: u64,
        /// The register address it tried to touch.
        addr: u64,
        /// The refusal, by name.
        why: &'static str,
    },
    /// The program asked the firmware to shut the machine down — SBI v0.1 `sbi_shutdown`,
    /// EID 8. There is no exit status: the legacy call does not return one and does not
    /// return at all, so inventing a `0` here would be inventing a claim.
    Shutdown {
        /// The `ecall` that asked.
        pc: u64,
    },
    /// An SBI call this machine does not implement. Named by extension and function id
    /// rather than answered with `0`: an unimplemented `sbi_set_timer` that quietly
    /// succeeds leaves a program running on a promise nothing kept.
    Sbi {
        /// The `ecall` that made it.
        pc: u64,
        /// Extension id — `a7`, which is where both SBI v0.1 and v0.2 put it.
        eid: u64,
        /// Function id — `a6`. The v0.1 legacy calls this machine implements do not use
        /// it; it is reported because an unrecognised EID is usually a v0.2 call, and
        /// the pair is what names one.
        fid: u64,
    },
    /// `ebreak`. On hardware this traps to a debugger; there is no debugger here, so the
    /// honest thing is to stop at it rather than step over it.
    Breakpoint {
        /// Where the `ebreak` is.
        pc: u64,
    },
    /// A CSR this machine does not implement. **A refusal, not a gap** — see the crate
    /// documentation. `F-001c2a` gave the six supervisor trap registers real state; the
    /// rest still stop here, and they stop *by name* so that what is missing stays a
    /// measured list rather than a guess.
    Csr {
        /// The instruction that tried.
        pc: u64,
        /// Which CSR, as the 12-bit number the instruction carries.
        csr: u16,
        /// Whether the instruction would have *modified* it. `csrrw` always does;
        /// `csrrs`/`csrrc` and their immediate forms do not when their source operand is
        /// zero, which is the spec's own rule and how a bare read is written.
        write: bool,
    },
    /// A virtual address that has no translation, or one whose permissions refuse the
    /// access — and **with no handler installed to receive it**. A page fault is an
    /// ordinary exception here: [`Machine::step`] delivers it to `stvec` exactly as it
    /// delivers `ebreak`, and this variant is what is left when `stvec` is zero, which on
    /// metal means trapping to address 0 and dying there.
    PageFault {
        /// The instruction that faulted.
        pc: u64,
        /// The virtual address it could not translate. This is what `stval` would hold.
        addr: u64,
        /// The `scause` a handler would read: `12` fetching, `13` loading, `15` storing.
        /// The number rather than an enum, because it is the number the program sees.
        cause: u64,
    },
    /// A trap raised with `stvec` zero, so there is nowhere to deliver it. The same
    /// choice `ebreak` and a page fault make, and for the same reason: on metal it would
    /// trap to address 0 and execute the rubble there.
    ///
    /// Three causes reach it. `8` is an environment call from user mode — the number
    /// `spec/user-mode.sas` exists to print, and the one an S-mode `ecall` (which is 9,
    /// and is an SBI call here) cannot forge. `2` is an illegal instruction: `sret`,
    /// `sfence.vma` and every CSR access are privileged, and a user program that executes
    /// one is trapped rather than obeyed — as is a *write* to a read-only CSR from either
    /// mode. And with bit 63 set it is an **interrupt** that was enabled and pending with
    /// no vector installed to take it.
    Undelivered {
        /// The instruction that raised it — for an interrupt, the one that would have run.
        pc: u64,
        /// The `scause` a handler would have read: `8` for an `ecall`, `2` for an illegal
        /// instruction, or [`INTERRUPT`] `| 5` for the timer.
        cause: u64,
    },
    /// The instruction budget ran out. A browser tab must not be wedged by a bad program,
    /// so the interpreter is bounded rather than trusted.
    StepLimit {
        /// Where it had got to.
        pc: u64,
    },
    /// The program stored to [`WAIT`]: it needs the world before it can go on
    /// (`W-370`, ADR-0040 Option C). **The only halt that is a pause, not an end.**
    ///
    /// The store has RETIRED — it is counted in [`Machine::time`] like any other store,
    /// and `self.pc` already names the instruction after it — so calling
    /// [`Machine::run`] again resumes the program with every register and octet intact,
    /// and the count accumulates. From the program's side the wait is one instruction;
    /// the host's time is outside the count. Whatever the host delivers in between goes
    /// in through RAM (`input::inject`'s direction), never between two instructions.
    ///
    /// A host with no event source must not resume blindly or call this success:
    /// `yantra-run` exits non-zero naming it, and the supervisor and kernel report it as
    /// [`supervisor::Ended::Stopped`] rather than resuming it.
    Wait {
        /// The store that asked. The machine resumes at `pc + 4`.
        pc: u64,
    },
}

/// `sstatus.SIE` — supervisor interrupts enabled. It gates interrupt delivery **while the
/// hart is in S-mode and only then**: an interrupt destined for S-mode is always taken
/// when the hart is running below S, because a user program must not be able to hold the
/// kernel's own interrupts off by never enabling them. See [`Machine::interrupt`].
const SSTATUS_SIE: u64 = 1 << 1;
/// `sstatus.SPIE` — what `SIE` was before the trap.
const SSTATUS_SPIE: u64 = 1 << 5;
/// `sstatus.SPP` — the privilege the trap came from. `1` is supervisor.
const SSTATUS_SPP: u64 = 1 << 8;
/// `sstatus.SUM` — permit supervisor **loads and stores** to a `U = 1` page. Not fetches:
/// the spec says so in as many words, and [`Machine::translate`] honours the distinction.
const SSTATUS_SUM: u64 = 1 << 18;
/// `sstatus.MXR` — make a load that would need `R` accept an `X`-only page instead.
const SSTATUS_MXR: u64 = 1 << 19;
/// `sstatus.FS` — the floating-point unit's state, two bits: 0 off, 1 initial, 2 clean,
/// 3 dirty.
const SSTATUS_FS: u64 = 0b11 << 13;
/// `sstatus.VS` — the vector unit's state, the same four values as `FS`. Kept since
/// `V-007`; see [`SSTATUS_MASK`].
const SSTATUS_VS: u64 = 0b11 << 9;
/// `sstatus.SD`, bit 63 on RV64 — read-only, "some extension state is Dirty": set exactly
/// when `FS`, `VS` or `XS` reads 3. COMPUTED ON READ ([`Machine::csr_read`]), never
/// stored, so no write can set it and no path that dirties a unit can forget it (`V-009`
/// (i-d) review: yantra never set it, QEMU does, and a kernel that saves the float and
/// vector files only when `SD` says so would have skipped live state). `XS` is not kept
/// here and is always 0.
const SSTATUS_SD: u64 = 1 << 63;
/// `sstatus.UXL`, bits 33:32 — the XLEN of U-mode. It reads 2 (64-bit), as on QEMU
/// (`V-009` (i-e); the (i-d) builder's probe4 S0 read `0x0000000200002200`), and like
/// `SD` it is COMPUTED ON READ and never stored: this machine runs U-mode at 64 bits only,
/// so the field is WARL with one legal value and every write leaves it 2.
///
/// QEMU 10.1.0 DIFFERS ON WRITES: it stores a non-zero UXL (1 and 3 read back) and
/// ignores 0 (`probe_ctl.S` section U); both rows are in [`KNOWN_QEMU_DIFFERENCES`]. A
/// hart whose U-mode is 64-bit only holds 2, which is what the specification's WARL rule
/// and the coordinator's ruling for (i-e) ask; QEMU can hold 1 because it can run U-mode
/// at 32 bits, which this machine cannot.
const SSTATUS_UXL: u64 = 2 << 32;

/// One place where this machine DELIBERATELY reads back something other than QEMU 10.1.0
/// after a CSR write, because the specification says otherwise. See
/// [`KNOWN_QEMU_DIFFERENCES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QemuDifference {
    /// What it covers, in a few words.
    pub covers: &'static str,
    /// The CSR written, and the bits the write replaces (`old & !mask | value`).
    pub write_csr: u16,
    /// See `write_csr`.
    pub write_mask: u64,
    /// The value written into those bits.
    pub value: u64,
    /// The CSR read back, and the bits compared.
    pub read_csr: u16,
    /// See `read_csr`.
    pub read_mask: u64,
    /// What this machine reads there.
    pub yantra: u64,
    /// What QEMU 10.1.0 read there (`probe_ctl.S`, recorded in
    /// `tests/data/v009e_csr_writes_qemu.tsv`).
    pub qemu: u64,
    /// Why the specification, not QEMU, decides it.
    pub reason: &'static str,
}

const UXL_WARL: &str = "sstatus.UXL is WARL; this machine runs U-mode at 64 bits only, so 2 is \
     its one legal value and every write leaves it 2 (QEMU, able to run a 32-bit U-mode, \
     stores 1, and stores the reserved 3)";
const VXRM_TWO_BITS: &str = "vxrm is a two-bit field (bits XLEN-1:2 read zero); QEMU stores a \
     vxrm write whole. A vcsr write QEMU masks, as here";

/// **THE CSR WRITES WHERE THIS MACHINE AND QEMU 10.1.0 DISAGREE, ON PURPOSE** (`V-009`
/// (i-e), confirmed by its review). Everywhere else QEMU is the oracle; in these rows the
/// specification is, and QEMU's observed value is kept beside this machine's so the
/// agreement tests can expect this machine's value AND check that QEMU's recorded one
/// still differs (`tests/v009e_hardware_agreement.rs`, which also fails if a row this
/// machine deviates on is missing from here).
pub const KNOWN_QEMU_DIFFERENCES: [QemuDifference; 4] = [
    QemuDifference {
        covers: "sstatus.UXL written 1",
        write_csr: 0x100,
        write_mask: 3 << 32,
        value: 1 << 32,
        read_csr: 0x100,
        read_mask: 3 << 32,
        yantra: 2 << 32,
        qemu: 1 << 32,
        reason: UXL_WARL,
    },
    QemuDifference {
        covers: "sstatus.UXL written 3",
        write_csr: 0x100,
        write_mask: 3 << 32,
        value: 3 << 32,
        read_csr: 0x100,
        read_mask: 3 << 32,
        yantra: 2 << 32,
        qemu: 3 << 32,
        reason: UXL_WARL,
    },
    QemuDifference {
        covers: "vxrm written 0xff, vxrm read back",
        write_csr: 0x00a,
        write_mask: u64::MAX,
        value: 0xff,
        read_csr: 0x00a,
        read_mask: u64::MAX,
        yantra: 0x3,
        qemu: 0xff,
        reason: VXRM_TWO_BITS,
    },
    QemuDifference {
        covers: "vxrm written 0xff, vcsr read back",
        write_csr: 0x00a,
        write_mask: u64::MAX,
        value: 0xff,
        read_csr: 0x00f,
        read_mask: u64::MAX,
        yantra: 0x6,
        qemu: 0x1fe,
        reason: VXRM_TWO_BITS,
    },
];

/// The only bits of `sstatus` this machine keeps. A write drops the rest, and that is
/// safe **because every feature they gate is itself refused**.
///
/// **`FS` WAS DROPPED UNDER THAT RULE AND THE RULE'S PREMISE HAS GONE.** The sentence
/// here read "`FS`/`XS`/`VS` track extension state and `fadd.d` still halts
/// `Unimplemented`" — which was true, and was the whole argument. Row `V-001` implemented
/// F and D ([`crate::fp`]), so `fadd.d` runs, and a dropped `FS` would mean a program
/// could not record that its FPU state is live: it would write `FS = dirty`, read back 0,
/// and a context switch built on that reading would discard a live register file. `FS` is
/// therefore kept now, for exactly the reason `F-001c2b2` kept `SUM` and `MXR` — the
/// feature the bit gates exists.
///
/// **`VS` WAS DROPPED UNDER THE SAME RULE, AND ROW `V-007` REMOVED ITS PREMISE TOO.** The
/// argument here read "there is no vector unit, so `vsetvli` halts `Unimplemented`".
/// [`crate::vector`] now executes `vsetvli` and an e64 subset, so a dropped `VS` would
/// repeat `FS`'s hazard exactly: a supervisor writes `VS = dirty`, reads back 0, and a
/// context switch built on that reading discards a live vector register file. `VS` is
/// kept, and every write of vector state — `vsetvli`, a vector load, an arithmetic
/// result, a `vstart` write — sets it dirty (`crates/yantra/tests/vector_extension.rs`,
/// `vector_state_marks_vs_dirty` and `a_vector_store_does_not_mark_vs_dirty`).
///
/// `XS` alone stays dropped, and the original argument still holds for it: there is no
/// user extension.
///
/// `SUM` and `MXR` were dropped by the same rule until `F-001c2b2`, and they are here now
/// for the reason it lifted the drop: there is a walker, and there is a mode below the
/// one that runs it, so both bits now change what a walk permits. See
/// [`Machine::translate`].
const SSTATUS_MASK: u64 =
    SSTATUS_SIE | SSTATUS_SPIE | SSTATUS_SPP | SSTATUS_SUM | SSTATUS_MXR | SSTATUS_FS | SSTATUS_VS;

/// Tag a rounded `(bits, flags)` with whether the result is a single (to be NaN-boxed).
fn op2((bits, flags): (u64, u64), single: bool) -> (u64, u64, Option<bool>) {
    (bits, flags, Some(single))
}

/// The privilege the hart is executing at.
///
/// **A field on the machine, not a bit in a CSR.** `sstatus.SPP` records the privilege a
/// *trap came from*, which is a different fact and is stale between a trap and its `sret`;
/// reading the current mode out of it would say "supervisor" inside a user program whose
/// last trap came from supervisor mode.
///
/// There is no machine mode. This interpreter *is* the firmware (see the crate docs), so
/// the two modes below it are the two that exist.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Privilege {
    /// U-mode. Reaches only `U = 1` pages, and every privileged instruction it executes
    /// traps as illegal.
    User,
    /// S-mode. The mode a program starts in, because the firmware hands the kernel the
    /// machine, not the other way round.
    #[default]
    Supervisor,
}

/// `scause`'s top bit. Set means the cause number names an **interrupt**; clear means an
/// exception. It is the whole difference between the `5` of a timer and the `5` of a
/// load-address-misaligned, and `tools/check-timer.sh` treats it as the evidence.
pub const INTERRUPT: u64 = 1 << 63;

/// `sie`/`sip` bit 1 — supervisor **software** interrupt. The only bit of `sip` software
/// may write, because it is the only one software raises.
const SI_SSI: u64 = 1 << 1;
/// `sie`/`sip` bit 5 — supervisor **timer** interrupt. In `sip` this is read-only here:
/// it is computed from [`Machine::time`] and [`Machine::timecmp`], and a program clears it
/// by moving the deadline, which is what `sbi_set_timer` is for.
const SI_STI: u64 = 1 << 5;
/// `sie`/`sip` bit 9 — supervisor **external** interrupt. Enable-able and never pending:
/// there is no interrupt controller here, and `spec/irq.sas` is the `C-` row that would
/// build one.
const SI_SEI: u64 = 1 << 9;
/// The interrupt cause number for each of the three, as `scause` reports it under
/// [`INTERRUPT`]. Ordered by the privileged spec's own priority — external, then software,
/// then timer — so a machine with two pending takes the one hardware would take.
const INTERRUPTS: [(u64, u64); 3] = [(SI_SEI, 9), (SI_SSI, 1), (SI_STI, 5)];
/// The three bits `sie` keeps. A write of anything else is dropped, and nothing is lost by
/// it: every other position names an interrupt destined for a mode this machine has not
/// got.
const SIE_MASK: u64 = SI_SSI | SI_STI | SI_SEI;

/// The supervisor trap registers, and only those.
///
/// **The set is measured, not chosen.** The seventeen `spec/` programs that touch a CSR
/// at all name exactly ten between them, and every one is a supervisor register:
/// `sstatus` (0x100), `sie` (0x104), `stvec` (0x105), `sscratch` (0x140), `sepc` (0x141),
/// `scause` (0x142), `stval` (0x143), `sip` (0x144), `satp` (0x180) and `time` (0xc01).
/// **Not one machine-mode register appears** — no `mtvec`, no `mepc`, no `mcause` — which
/// is what the measurement was for: `F-001c2` was written expecting them.
///
/// **All ten are here as of `F-001c2b3`, and none of them is a stored bit pretending to be
/// a mechanism.** `satp` arrived with the walker (`F-001c2b1`); `sie` and `sip` arrive
/// with an interrupt that is actually delivered ([`Machine::interrupt`]) and a clock that
/// actually advances ([`Machine::time`]). `time` is not a field of this struct at all —
/// it is a counter on the machine, read through CSR `0xc01` and **writable by nothing**,
/// because a counter software can set is not a clock.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Csrs {
    /// `sstatus`, masked to [`SSTATUS_MASK`].
    pub sstatus: u64,
    /// `sie`, masked to [`SIE_MASK`]. Which interrupts the kernel is willing to be
    /// interrupted by; `sstatus.SIE` is the separate question of whether it is willing
    /// *now*.
    pub sie: u64,
    /// `sip`, and **only the bit software owns** — `SSIP`. The timer's pending bit is not
    /// here because it is not state: [`Machine::pending`] computes it from the deadline
    /// every time it is read, so there is no moment where a stored `STIP` and the clock
    /// can disagree.
    pub sip: u64,
    /// `stvec`. The low two bits are `MODE`; the base is the rest. Zero means no handler
    /// has been installed — see [`Machine::step`]'s `ebreak`.
    pub stvec: u64,
    /// `sscratch`. The spec gives it no meaning beyond storage, so neither does this.
    pub sscratch: u64,
    /// `sepc`, the address the trap came from. Bit 0 is always zero.
    pub sepc: u64,
    /// `scause`. `3` is a breakpoint, which is the only cause this machine can raise.
    pub scause: u64,
    /// `stval`. For a breakpoint the spec sets it to the address of the `ebreak`; for a
    /// page fault, to the virtual address that could not be translated.
    pub stval: u64,
    /// `satp`. `MODE` is bits 63:60, `ASID` 59:44 and the root table's page number the
    /// low 44. Only `MODE = 0` (Bare) and `MODE = 8` (Sv39) can be *held* — see
    /// [`Machine::csr_write`] for what happens to a write of any other mode, and
    /// [`Machine::translate`] for the walk.
    pub satp: u64,
}

/// `satp.MODE = 8`, the only translation scheme this machine implements, already shifted
/// into place. Sv39 is what every `spec/` program that turns translation on asks for.
const SATP_SV39: u64 = 8 << 60;
/// The 44-bit physical page number at the bottom of `satp`, and of every PTE above bit 10.
const PPN_MASK: u64 = (1 << 44) - 1;

/// PTE bit 0, `V`. A zero here means the entry is not a translation at all.
const PTE_V: u64 = 1 << 0;
/// PTE bit 1, `R` — readable.
const PTE_R: u64 = 1 << 1;
/// PTE bit 2, `W` — writable. `W` without `R` is a reserved encoding, not a permission.
const PTE_W: u64 = 1 << 2;
/// PTE bit 3, `X` — executable.
const PTE_X: u64 = 1 << 3;
/// PTE bit 4, `U` — reachable from user mode. Checked **both ways**: U-mode reaches
/// nothing without it, and S-mode reaches nothing with it unless `sstatus.SUM` says
/// otherwise — and never for a fetch.
const PTE_U: u64 = 1 << 4;
/// PTE bit 6, `A` — accessed. **Not filled by this machine**; see [`Machine::translate`].
const PTE_A: u64 = 1 << 6;
/// PTE bit 7, `D` — dirty. Not filled either, and required of a store.
const PTE_D: u64 = 1 << 7;

/// `scause` 12 — an instruction fetch that could not be translated.
const CAUSE_FETCH_PAGE_FAULT: u64 = 12;
/// `scause` 13 — a load that could not be translated.
const CAUSE_LOAD_PAGE_FAULT: u64 = 13;
/// `scause` 15 — a store or AMO that could not be translated.
const CAUSE_STORE_PAGE_FAULT: u64 = 15;

/// What a translation is *for*. The three access types the privileged spec distinguishes,
/// because they check different permission bits and raise different causes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    /// An instruction fetch: needs `X`, faults with cause 12.
    Fetch,
    /// A load: needs `R`, faults with cause 13.
    Load,
    /// A store or an AMO: needs `W` **and** `D`, faults with cause 15.
    Store,
}

impl Access {
    /// The `scause` a fault on this access raises.
    const fn cause(self) -> u64 {
        match self {
            Access::Fetch => CAUSE_FETCH_PAGE_FAULT,
            Access::Load => CAUSE_LOAD_PAGE_FAULT,
            Access::Store => CAUSE_STORE_PAGE_FAULT,
        }
    }

    /// The PTE permission bit it requires.
    const fn permission(self) -> u64 {
        match self {
            Access::Fetch => PTE_X,
            Access::Load => PTE_R,
            Access::Store => PTE_W,
        }
    }
}

/// A machine: registers, one flat span of RAM, a program counter and the supervisor
/// trap state.
#[derive(Debug)]
pub struct Machine {
    /// `x0`..`x31`. `x0` is written on every step rather than special-cased at each site.
    pub x: [u64; 32],
    /// `f0`..`f31`, the F/D register file, as RAW BITS rather than `f64`.
    ///
    /// Bits and not floats because a register is not always a double: `fmv.w.x`, `flw`
    /// and every single-precision result store a NaN-boxed single (upper 32 bits all
    /// ones, see [`crate::fp::box_s`]), and `fmv.x.d` hands the whole word to an integer
    /// register. Holding `f64` here would make the boxing convention unrepresentable and
    /// would quieten a signalling NaN on every move, which the spec forbids: a move is
    /// not an arithmetic operation and must not touch the payload.
    pub f: [u64; 32],
    /// `fcsr` — `frm` in bits 7..5, `fflags` in bits 4..0.
    ///
    /// One field rather than two CSRs because `frm` and `fflags` are windows onto this
    /// same register, and two fields would let them disagree. See
    /// [`Machine::frm`]/[`Machine::fflags`].
    pub fcsr: u64,
    /// The program counter.
    pub pc: u64,
    /// Physical base address of [`Machine::mem`].
    pub base: u64,
    /// RAM, contiguous from `base`.
    pub mem: Vec<u8>,

    /// The highest RAM offset a STORE may touch. Loads are bounded by `mem.len()`.
    ///
    /// **W-363: THE INJECTED INPUT LIVES ABOVE THIS LINE, AND THIS IS WHAT KEEPS THE
    /// PROGRAM FROM OVERWRITING IT.** `input::inject` takes `old_top = mem.len()` and
    /// then RESIZES, so the slab sits past the budget the caller asked for — and the
    /// record allocator is a bump cursor with no upper bound, growing up from the
    /// file-backed extent. With one bound for both directions the heap reaches the
    /// slab, the program overwrites its own input, and the result is the one failure
    /// nobody can see: at a 2,557,897-octet budget a ten-frame decode answered
    /// `Finisher` status 0 with EIGHT frames and a different digest. Measured by
    /// A peer session, bisected to the octet — 2,557,896 refused, 2,557,897 "succeeded"
    /// wrongly, and neither was `BeyondRam`.
    ///
    /// Splitting the two directions makes that collision a REFUSAL instead: the
    /// program may still READ its input, which is the slab's whole purpose, and a
    /// store past the budget halts with `Halt::BeyondRam`, which names itself and
    /// carries the address and the bound. Since W-363's third piece this holds under
    /// `Span::Declared` too: a declared heap that outgrows its RAM reaches the slab
    /// just the same, and halts rather than overwriting the input.
    ///
    /// `usize::MAX` means "no bound beyond `mem` itself", which is what a machine
    /// built by hand for a test wants; `load_elf` sets it to the RAM it was given.
    pub store_limit: usize,
    /// The PATH run a [`patra`] request named, waiting for its third store.
    pub patra_path: Option<u64>,
    /// The BUFFER run likewise.
    pub patra_buffer: Option<u64>,
    /// The virtio-mmio GPU in slot 0 (`W-351`): the registers a driver can observe.
    pub virtio: virtio_mmio::VirtioMmio,
    /// The directory a [`patra`] request may read under, or `None`.
    ///
    /// **`None` REFUSES EVERY FILE REQUEST, and that is the default.** `yantra`
    /// is a developer runtime and not an isolation boundary (ADR-0039), but a
    /// program that can name any path can read `~/.ssh`, and no existing caller
    /// asked for that capability. Opting in is one field; opting out must not
    /// be something a caller forgets.
    pub patra_root: Option<std::path::PathBuf>,
    /// An in-memory file root ([`patra::MemFs`]). When `Some` it is used INSTEAD of
    /// `patra_root`; the browser host sets it, the native runner never does.
    pub patra_mem: Option<patra::MemFs>,
    /// The address `lr.w`/`lr.d` reserved, if one is live. `sc` succeeds only against a
    /// reservation for its own address; on one hart nothing else can steal it, so this is
    /// a single word rather than a set. It is deliberately **not** cleared by an ordinary
    /// store: the A extension permits that, and permitting it here would make a
    /// `lr`/`sc` pair fail for a reason no multi-hart machine would produce.
    pub reservation: Option<u64>,
    /// The supervisor trap registers. There is no M-mode above this machine: it *is* the
    /// firmware (see the crate docs), so `sret` is the only return and there is nothing
    /// for an `mret` to mean.
    pub csr: Csrs,
    /// The privilege the next instruction executes at. Starts [`Privilege::Supervisor`]
    /// and moves only two ways: `sret` into what `sstatus.SPP` names, and a trap back up.
    pub mode: Privilege,
    /// The `time` CSR: **one tick per instruction the hart begins**, and nothing else.
    ///
    /// A browser tab's wall clock is not this hart's, and reading it would make a run
    /// depend on how busy the machine underneath was — the same program would take a
    /// different number of ticks each time, and `spec/timer.sas` would fire its handler at
    /// a different instruction on every load. Retired instructions are the only quantity
    /// this interpreter has that advances monotonically, is a function of the program
    /// alone, and cannot be run backwards. See `.loop/ASSUMPTIONS.md`.
    ///
    /// The consequence to be honest about: a **second here is not a second**, so a program
    /// that computes a deadline as "now plus a million" is asking for a million
    /// instructions, not for a millisecond. Programs that arm an absolute past or future
    /// deadline — which is every one in `spec/` — are unaffected.
    pub time: u64,
    /// The deadline `sbi_set_timer` armed, compared against [`Machine::time`]. `None` is a
    /// machine whose timer was never set, which is **not** the same as a deadline of zero:
    /// zero is already past and fires at once.
    pub timecmp: Option<u64>,
    /// The vector register file, `vl`, `vtype` and `vstart` — row `V-007`, see
    /// [`crate::vector`].
    pub vec: vector::VectorUnit,
    /// THE SOCKET DEVICE (`W-377`): the queue the program drains through [`SOCK`]'s
    /// registers. `None` is a machine with no socket — every access to the window halts
    /// `Device` naming `W-377` — and that is the default; `yantra-run` makes one for
    /// `--listen` or a socket log.
    pub socket: Option<socket::Socket>,
}

impl Machine {
    /// Load an ELF and place its segments. Returns the machine ready at the entry point.
    ///
    /// # Errors
    /// A string naming what is wrong with the file. This is a demo loader and it checks
    /// what it relies on — 64-bit, little-endian, RISC-V — rather than assuming a
    /// well-formed input, because the failure of an unchecked assumption here is a wrong
    /// answer rather than a crash.
    pub fn load_elf(image: &[u8], ram: usize) -> Result<Self, String> {
        Self::load_elf_spanning(image, ram, Span::Declared)
    }

    /// `load_elf`, with the RAM bound read under an explicit [`Span`].
    ///
    /// `Span::Declared` is `load_elf` and insists RAM cover `memsz` — the whole
    /// declared span. `Span::FileBacked` insists only that the file-backed octets
    /// fit, which is what the loader actually copies (the line below), letting a
    /// caller run an image whose declared `.bss` tail it cannot afford.
    pub fn load_elf_spanning(image: &[u8], ram: usize, span: Span) -> Result<Self, String> {
        // The header checks and the program headers are `loader::Program`'s, because the
        // application loader has to make exactly the same ones and a second copy of them
        // is a second set of answers to "is this file runnable".
        let program = loader::Program::parse(image)?;

        // Base is taken from the lowest loadable segment rather than hard-coded, so a
        // program linked at the second address (`0x8040_0000`) loads without a change here.
        let base = program
            .segments
            .iter()
            .map(|s| s.vaddr)
            .min()
            .expect("Program::parse refuses a file with no PT_LOAD");
        let mut m = Machine {
            patra_root: None,
            patra_mem: None,
            patra_path: None,
            patra_buffer: None,
            virtio: Default::default(),
            x: [0; 32],
            f: [0; 32],
            fcsr: 0,
            pc: program.entry,
            base,
            mem: vec![0; ram],
            // ॥ STORES STOP AT THE RAM THE CALLER GAVE, UNDER EITHER SPAN ॥ (W-363)
            //
            // `Span::Declared` kept `usize::MAX` until W-363's third piece, and the
            // heap reaching an injected slab is not hypothetical there: on
            // 2026-09-21 the first native self-image build grew to 559,504,480
            // octets over a 555,254,376-octet top, and `yantra-run.rs`'s advisory
            // stderr line was the only witness. Bounding it converts that silent
            // corruption into `Halt::BeyondRam`. The corpus-wide witness the old
            // margin asked for is the gate on 962c6905, where the bound applied to
            // both spans: 188 targets, 1587 tests, 0 failures. Loads are still
            // bounded by `mem` alone, so a program reads its input above the line.
            // (The fixpoint's Stage 2 is a script no `cargo test` reaches; it is
            // owed a run under this bound before landing.)
            store_limit: ram,
            reservation: None,
            csr: Csrs::default(),
            // The firmware hands the kernel the machine in supervisor mode; every
            // `spec/` program is entered that way by OpenSBI and is entered that way here.
            mode: Privilege::Supervisor,
            // Zero, and no deadline: the hart has retired nothing and nobody has asked
            // for the clock. QEMU's `mtimecmp` starts at zero instead, which is why
            // `tools/check-timer.sh` had to grow a negative control — a machine where the
            // timer is already pending at boot passes a test of `set_timer` without ever
            // being told anything.
            time: 0,
            timecmp: None,
            vec: vector::VectorUnit::default(),
            socket: None,
        };
        for s in &program.segments {
            let at = (s.vaddr - base) as usize;
            // THE ONE PLACE THE RAM BOUND IS READ (W-363's size half). `filesz` is
            // in the `max` under both spans and not only for tidiness: it is the
            // slice the copy below takes, so a span that did not cover it would
            // turn a refusal into a panic. `memsz < filesz` is not a shape this
            // writer emits, and this does not rely on that.
            let need = match span {
                Span::Declared => s.memsz.max(s.filesz),
                Span::FileBacked => s.filesz,
            };
            if at + need > m.mem.len() {
                return Err(format!(
                    "segment at {:#x} needs {} bytes and RAM is {} — raise it",
                    s.vaddr,
                    need,
                    m.mem.len()
                ));
            }
            m.mem[at..at + s.filesz].copy_from_slice(&image[s.offset..s.offset + s.filesz]);
        }
        Ok(m)
    }

    /// An integer load at [`COUNTER`]'s eight octets, or `None` for any other address.
    /// `time` already counts the instruction doing the read (`step` advances it first).
    /// A read must lie wholly inside the window and be aligned to its width; anything
    /// else is refused by name rather than answered with a shifted piece of the count.
    fn counter(&self, addr: u64, width: usize, signed: bool, pc: u64) -> Option<Result<u64, Halt>> {
        let off = addr.checked_sub(COUNTER).filter(|o| *o < 8)?;
        let width = width as u64;
        if off % width != 0 || off + width > 8 {
            return Some(Err(Halt::Device {
                pc,
                addr,
                why: "the retired-instruction counter is read aligned and inside its eight \
                      octets (W-374)",
            }));
        }
        let mut v = self.time >> (8 * off);
        if width < 8 {
            v &= (1u64 << (8 * width)) - 1;
            let sign = 1u64 << (8 * width - 1);
            if signed && v & sign != 0 {
                v |= !((1u64 << (8 * width)) - 1);
            }
        }
        Some(Ok(v))
    }

    #[inline]
    fn load(&self, addr: u64, width: usize, signed: bool, pc: u64) -> Result<u64, Halt> {
        // RAM FIRST (`W-382`): above `DEVICE_TOP` no device can answer, so an access
        // wholly inside `mem` is read in one piece. Anything else — a device address, an
        // address below `base`, a read past the end — takes the original walk below.
        if addr >= DEVICE_TOP
            && let Some(off) = addr.checked_sub(self.base)
            && let Some(v) = ram_read(&self.mem, off, width)
        {
            return Ok(if signed { sign_extend(v, width) } else { v });
        }
        self.load_walk(addr, width, signed, pc)
    }

    /// The original `load`: every device window, then RAM a byte at a time. Reached only
    /// for what the fast path above declines, so its answers are unchanged.
    #[cold]
    #[inline(never)]
    fn load_walk(&self, addr: u64, width: usize, signed: bool, pc: u64) -> Result<u64, Halt> {
        // THE SOCKET'S RX (`W-377`): a load reads the latch and changes nothing — the pop
        // is NEXT's store, because this takes `&self` (`socket`'s margin). Every other
        // offset, width or a machine with no socket refuses by name. The answers are
        // below `0x200`, so `lw` and `lwu` read the same word.
        if let Some(off) = addr.checked_sub(SOCK).filter(|o| *o < socket::SOCK_LEN) {
            return socket::load(self.socket.as_ref(), off, width)
                .map(u64::from)
                .map_err(|why| Halt::Device { pc, addr, why });
        }
        // THE VIRTIO-MMIO WINDOW (`W-351`), the machine's first device that answers a
        // read. A range, not an exact address like the arms in `store`, because a
        // transport is a block of registers; everything in it that the device does not
        // answer halts `Device` by name rather than falling through to `BadAccess`.
        if let Some((slot, off)) = virtio_mmio::slot_of(addr) {
            let v = u64::from(
                self.virtio
                    .read(slot, off, width)
                    .map_err(|why| Halt::Device { pc, addr, why })?,
            );
            // Registers are 32 bits (§4.2.2), so `lw` sign-extends and `lwu` does not,
            // exactly as for a word read from RAM.
            return Ok(if signed && v & 0x8000_0000 != 0 {
                v | !0xffff_ffff
            } else {
                v
            });
        }
        let at = addr
            .checked_sub(self.base)
            .and_then(|o| usize::try_from(o).ok())
            .ok_or(Halt::BadAccess { pc, addr })?;
        if at + width > self.mem.len() {
            return Err(Halt::BeyondRam {
                pc,
                addr,
                ram: self.mem.len(),
            });
        }
        let mut v: u64 = 0;
        for i in 0..width {
            v |= u64::from(self.mem[at + i]) << (8 * i);
        }
        if signed && width < 8 {
            let sign = 1u64 << (width * 8 - 1);
            if v & sign != 0 {
                v |= !((1u64 << (width * 8)) - 1);
            }
        }
        Ok(v)
    }

    #[inline]
    fn store(
        &mut self,
        addr: u64,
        width: usize,
        value: u64,
        pc: u64,
        out: &mut impl Output,
    ) -> Result<Option<Halt>, Halt> {
        // RAM FIRST (`W-382`): above `DEVICE_TOP` none of the arms in `store_walk` can
        // match, so a store wholly inside the writable span (the same `store_limit`
        // bound, W-363) is written in one piece and reported to `out.stored` exactly as
        // the walk reports it. Anything else takes the walk, unchanged.
        if addr >= DEVICE_TOP
            && let Some(off) = addr.checked_sub(self.base)
            && let Ok(at) = usize::try_from(off)
        {
            let writable = self.mem.len().min(self.store_limit);
            if width <= 8 && at < writable && width <= writable - at {
                out.stored(at, width);
                let bytes = value.to_le_bytes();
                self.mem[at..at + width].copy_from_slice(&bytes[..width]);
                return Ok(None);
            }
        }
        self.store_walk(addr, width, value, pc, out)
    }

    /// The original `store`: the device arms in order, then RAM a byte at a time. Reached
    /// only for what the fast path in `store` declines.
    #[cold]
    #[inline(never)]
    fn store_walk(
        &mut self,
        addr: u64,
        width: usize,
        value: u64,
        pc: u64,
        out: &mut impl Output,
    ) -> Result<Option<Halt>, Halt> {
        // THE BRIDGE. On metal this store reaches a device; here it reaches the page.
        if addr == UART {
            out.putc(value as u8);
            return Ok(None);
        }
        // THE FILE WINDOW. The store carries the ADDRESS of a request block in
        // the program's own RAM, and the whole operation finishes before it
        // returns — see `patra`'s module margin for why there is no status
        // register to poll and no "not yet" to encode.
        // THE FILE WINDOW, three stores. The first two are REMEMBERED and the
        // third does the work, so a half-built request never reaches the
        // filesystem — see `patra`'s margin for why it is three runs and not
        // one block.
        if addr == patra::PATRA_PATH {
            self.patra_path = Some(value);
            return Ok(None);
        }
        if addr == patra::PATRA_BUFFER {
            self.patra_buffer = Some(value);
            return Ok(None);
        }
        // THE WRITE, at its own address. A separate arm rather than a command
        // word, so the emitted text says which direction the octets move and a
        // program cannot turn a read into a write by computing a wrong number.
        if addr == patra::PATRA_PUT {
            let root = self.patra_root.clone();
            let (Some(p), Some(b)) = (self.patra_path, self.patra_buffer) else {
                patra::put(&mut self.mem, self.base, 0, 0, value, None);
                return Ok(None);
            };
            if let Some(fs) = self.patra_mem.as_mut() {
                patra::put_mem(&mut self.mem, self.base, p, b, value, fs);
            } else {
                patra::put(&mut self.mem, self.base, p, b, value, root.as_deref());
            }
            self.patra_path = None;
            self.patra_buffer = None;
            return Ok(None);
        }
        if addr == patra::PATRA_GO {
            let root = self.patra_root.clone();
            let (Some(p), Some(b)) = (self.patra_path, self.patra_buffer) else {
                // A go with no path or buffer named: refuse into the status run
                // rather than reading something arbitrary.
                patra::serve(&mut self.mem, self.base, 0, 0, value, None);
                return Ok(None);
            };
            if let Some(fs) = self.patra_mem.as_ref() {
                patra::serve_mem(&mut self.mem, self.base, p, b, value, fs);
            } else {
                patra::serve(&mut self.mem, self.base, p, b, value, root.as_deref());
            }
            self.patra_path = None;
            self.patra_buffer = None;
            return Ok(None);
        }
        // THE VIRTIO-MMIO WINDOW (`W-351`). AFTER the four exact patra arms on purpose:
        // `patra::PATRA_PATH` is slot 1's base, so those stores must be matched first —
        // `virtio_mmio`'s margin has the address map and why the overlap is safe.
        if let Some((slot, off)) = virtio_mmio::slot_of(addr) {
            let ram = self.base..self.base + self.mem.len() as u64;
            let notified = self
                .virtio
                .write(slot, off, width, value, ram)
                .map_err(|why| Halt::Device { pc, addr, why })?;
            // QUEUENOTIFY: walk every chain the driver has offered, then raise the
            // used-buffer interrupt bit (§4.2.2 InterruptStatus bit 0). A refusal
            // halts by name and, by `virtqueue::process`'s contract, writes nothing for
            // that chain; chains already served before it stay served.
            //
            // THE ANSWER IS THE DEVICE'S (`gpu.rs`, `W-378`): each chain's request is
            // served against the GPU's resources, with a read-only view of RAM for a
            // transfer's backing. Before W-378 every chain got ERR_UNSPEC.
            //
            // DEFERRED (`YANTRA_VIRTIO_DEFER`, [`virtio_mmio::VirtioMmio::defer`]): with a
            // nonzero delay the chains are NOT served inside this store; the queue waits
            // and [`Machine::complete_deferred`] serves it that many instructions later,
            // as QEMU's device does. Zero, the default, serves them here, as always.
            if let Some(q) = notified {
                if self.virtio.defer == 0 {
                    self.serve_notified(q, pc, addr)?;
                } else if self.virtio.pending[q].is_none() {
                    self.virtio.pending[q] = Some(virtio_mmio::Pending {
                        left: self.virtio.defer,
                        pc,
                        addr,
                    });
                    self.virtio.waiting += 1;
                }
            }
            return Ok(None);
        }
        // THE SOCKET'S NEXT AND TX (`W-377`): NEXT pops the queue into the RX latch, TX
        // hands its low octet to `out.sent`. Refused by name otherwise — see `socket::store`.
        if let Some(off) = addr.checked_sub(SOCK).filter(|o| *o < socket::SOCK_LEN) {
            socket::store(self.socket.as_mut(), off, width, value, out)
                .map_err(|why| Halt::Device { pc, addr, why })?;
            return Ok(None);
        }
        if addr == FINISHER {
            // `W-341` — the whole register, not `as u32` then `>> 16`. Those two
            // truncations compounded to a SIXTEEN-bit status: error 70000 was
            // written as `0x3333 | (70000 << 16)` and read back as 4464. Success
            // (`0x5555`) maps to 0 and could never show it; only failures lost
            // information, which is what made it dangerous.
            let status = match value & 0xffff {
                0x5555 => Some(0),
                0x3333 => Some(value >> 16),
                _ => None,
            };
            return Ok(Some(Halt::Finisher { value, status }));
        }
        // THE WAIT (`W-370`, ADR-0040 Option C). Halts like the finisher, but it is a
        // pause: `step` moves `pc` past the store before returning it, so the next
        // `run` resumes there — see `Halt::Wait`. The value is ignored.
        if addr == WAIT {
            return Ok(Some(Halt::Wait { pc }));
        }
        let at = addr
            .checked_sub(self.base)
            .and_then(|o| usize::try_from(o).ok())
            .ok_or(Halt::BadAccess { pc, addr })?;
        // W-363: STORES STOP AT THE BUDGET, loads at `mem` itself. The difference is
        // where the injected input lives — see `store_limit`'s margin for the silent
        // wrong answer this refuses.
        let writable = self.mem.len().min(self.store_limit);
        if at + width > writable {
            return Err(Halt::BeyondRam {
                pc,
                addr,
                ram: writable,
            });
        }
        out.stored(at, width);
        for i in 0..width {
            self.mem[at + i] = (value >> (8 * i)) as u8;
        }
        Ok(None)
    }

    /// Turn a virtual address into a physical one by walking the Sv39 tables `satp` names.
    ///
    /// With `satp.MODE = 0` there are no tables and the address is already physical, which
    /// is the state every program in this repository starts in.
    ///
    /// # The walk
    ///
    /// Three levels, nine bits of index each, and a leaf is any entry with `R`, `W` or `X`
    /// set — so a level-2 leaf maps a gigabyte, a level-1 leaf two megabytes and a level-0
    /// leaf a page. `spec/paging.sas` uses the first of those: 512 gigapage entries, each
    /// mapping a virtual gigabyte onto the identical physical one.
    ///
    /// # `A` and `D` are faults here, not fills
    ///
    /// The privileged spec allows a hart either to set `A` (and `D` on a store) itself or
    /// to raise a page fault and let the handler do it. **This machine faults**, and that
    /// is a decision rather than an omission: `spec/paging.sas` sets both by hand and its
    /// comment says why it must — *"A और D हाथ से भरे जाते हैं क्योंकि यन्त्र उन्हें स्वयं
    /// भरने को बाध्य नहीं है"*. A walker that quietly filled them would run that program
    /// and every other one identically while diverging from the hart they target, and the
    /// divergence would only ever appear on a program that forgot the bits — which is the
    /// one program the fault exists to catch.
    ///
    /// # Errors
    ///
    /// [`Halt::PageFault`] naming the address and the cause the access type raises;
    /// [`Machine::step`] delivers it to `stvec` rather than stopping, when one is
    /// installed. A PTE that lies outside RAM is a [`Halt::BadAccess`] instead — the walk
    /// reads it *physically*, so an unreachable table is a bad physical address and not a
    /// missing translation.
    #[inline]
    fn translate(&self, va: u64, access: Access, pc: u64) -> Result<u64, Halt> {
        // BARE MODE (`W-382`): one test, inlined into every caller, and no walk.
        if self.csr.satp & SATP_SV39 != SATP_SV39 {
            return Ok(va);
        }
        self.translate_walk(va, access, pc)
    }

    /// The Sv39 walk itself, kept out of line so the bare-mode test above inlines.
    #[inline(never)]
    fn translate_walk(&self, va: u64, access: Access, pc: u64) -> Result<u64, Halt> {
        // A closure rather than a value: it is returned from four places inside a loop,
        // and a value would have to be cloned at each of them.
        let fault = || Halt::PageFault {
            pc,
            addr: va,
            cause: access.cause(),
        };
        // Sv39 addresses are 39 bits sign-extended to 64, and the spec requires the walk
        // to fault on anything else rather than to truncate. Truncating would map the
        // hole in the middle of the address space onto real memory.
        if ((va << 25) as i64 >> 25) as u64 != va {
            return Err(fault());
        }
        let mut table = (self.csr.satp & PPN_MASK) << 12;
        for level in (0..3).rev() {
            let index = (va >> (12 + 9 * level)) & 0x1ff;
            let pte = self.load(table + index * 8, 8, false, pc)?;
            // `W` without `R` is reserved, not write-only: the spec names it as a fault
            // so that a typo'd permission cannot read as a narrower one.
            if pte & PTE_V == 0 || (pte & PTE_R == 0 && pte & PTE_W != 0) {
                return Err(fault());
            }
            let ppn = (pte >> 10) & PPN_MASK;
            if pte & (PTE_R | PTE_X) == 0 {
                // A pointer to the next level. There is no level below 0.
                if level == 0 {
                    return Err(fault());
                }
                table = ppn << 12;
                continue;
            }
            // `MXR` widens exactly one thing: a *load* may be satisfied by an execute-only
            // page. It does not make a fetch readable, and it does not touch a store.
            let permitted = if access == Access::Load && self.csr.sstatus & SSTATUS_MXR != 0 {
                PTE_R | PTE_X
            } else {
                access.permission()
            };
            if pte & permitted == 0
                || !self.privilege_permits(pte, access)
                || pte & PTE_A == 0
                || (access == Access::Store && pte & PTE_D == 0)
            {
                return Err(fault());
            }
            // A superpage whose low PPN bits are not zero is misaligned. The spec faults
            // rather than ignoring them, because ignoring them silently maps a gigabyte
            // somewhere its entry did not say.
            let within = (1u64 << (9 * level)) - 1;
            if ppn & within != 0 {
                return Err(fault());
            }
            return Ok(((ppn & !within) | ((va >> 12) & within)) << 12 | (va & 0xfff));
        }
        unreachable!("the level-0 arm returns or faults, so the loop always leaves early")
    }

    /// Whether the mode this hart is in may touch a leaf with these flags, by the `U` bit
    /// alone — the permission bits and `A`/`D` are checked beside it.
    ///
    /// **The bit is checked in both directions, and the asymmetry is the spec's.** U-mode
    /// reaches nothing without `U`: that is the whole of the protection, and it is why
    /// `spec/user-mode.sas` puts `U` on exactly one page rather than on the picture.
    /// S-mode reaches nothing *with* it unless `sstatus.SUM` is set — and **never** for a
    /// fetch, `SUM` or not, because a kernel that could execute a user page could be made
    /// to execute a user program's instructions with the kernel's own privilege.
    const fn privilege_permits(&self, pte: u64, access: Access) -> bool {
        match self.mode {
            Privilege::User => pte & PTE_U != 0,
            Privilege::Supervisor => {
                pte & PTE_U == 0
                    || (!matches!(access, Access::Fetch) && self.csr.sstatus & SSTATUS_SUM != 0)
            }
        }
    }

    /// Translate an access of `width` bytes, refusing one that would cross a page.
    ///
    /// A four-byte fetch never crosses: `IALIGN` is 32 here, so every instruction address
    /// is aligned and 4096 divides by 4. A *misaligned* load or store can, and this
    /// machine permits misaligned data accesses. Two pages need two walks and the halves
    /// need not be physically adjacent, so rather than translate the first page and read
    /// on past its end — which is a wrong answer, in another program's memory — the access
    /// is refused by address. That is a limitation and it is named as one.
    #[inline]
    fn translate_span(&self, va: u64, width: usize, access: Access, pc: u64) -> Result<u64, Halt> {
        // BARE MODE (`W-382`): no tables, so both halves are the identity and the
        // page-crossing comparison below would always hold.
        if self.csr.satp & SATP_SV39 != SATP_SV39 {
            return Ok(va);
        }
        let pa = self.translate(va, access, pc)?;
        if (va & 0xfff) + width as u64 > 0x1000 {
            let last = self.translate(va + width as u64 - 1, access, pc)?;
            if last != pa + width as u64 - 1 {
                return Err(Halt::BadAccess { pc, addr: va });
            }
        }
        Ok(pa)
    }

    /// Read a CSR, or `None` if this machine does not implement it.
    ///
    /// `None` is the refusal, and it is the whole difference between this and a sparse
    /// map that returns zero: a zero is an *answer*, and an answer about `satp` that no
    /// page-table walker stands behind is the plausible wrong one.
    fn csr_read(&self, csr: u16) -> Option<u64> {
        Some(match csr {
            // `SD` and `UXL` are computed on read: neither is stored, so no write can
            // reach them (see [`SSTATUS_UXL`]).
            0x100 => {
                let s = self.csr.sstatus | SSTATUS_UXL;
                let dirty = s & SSTATUS_FS == SSTATUS_FS || s & SSTATUS_VS == SSTATUS_VS;
                if dirty { s | SSTATUS_SD } else { s }
            }
            0x104 => self.csr.sie,
            // The timer's pending bit is folded in on the way out rather than stored, so
            // a kernel that polls `sip` without enabling anything still sees the deadline
            // pass — which is what polling it is for.
            0x144 => self.csr.sip | self.pending_timer(),
            0x105 => self.csr.stvec,
            0x140 => self.csr.sscratch,
            0x141 => self.csr.sepc,
            0x142 => self.csr.scause,
            0x143 => self.csr.stval,
            0x180 => self.csr.satp,
            // `time` is the S-mode view of the machine's counter. Read-only — see
            // [`Machine::step_inner`] for what a write to it is.
            0xc01 => self.time,
            // The vector CSRs. `vl`, `vtype` and `vlenb` are read-only by number (0xc__).
            // `vxsat`, `vxrm` and `vcsr` (`V-009` (i-e)) belong to fixed-point
            // arithmetic, which the subset does not execute, so they are STORAGE: they
            // read 0 at reset, hold what is written to their legal bits, and nothing else
            // reads or sets them. Gated on `sstatus.VS` like the rest (the gate in
            // `step_inner`) and served to U-mode, as on QEMU (`probe_ctl.S` section C).
            // Until (i-e) they halted `Halt::Csr` from S-mode and trapped from U-mode,
            // with VS on or off.
            0x009 => self.vec.vxsat,
            0x00a => self.vec.vxrm,
            0x00f => (self.vec.vxrm << 1) | self.vec.vxsat,
            0x008 => self.vec.vstart,
            0xc20 => self.vec.vl,
            0xc21 => self.vec.vtype,
            0xc22 => vector::VLENB,
            // The float CSRs (`V-009` part (i-d)): three windows on one register,
            // `fcsr` = `frm` << 5 | `fflags`. Until (i-d) they halted `Halt::Csr`, so a
            // program could neither read the flags `V-001` accrues nor set `frm`; QEMU
            // reads and writes them, and traps them under `FS = Off` (the gate in
            // `step_inner`).
            0x001 => self.fflags(),
            0x002 => u64::from(self.frm()),
            0x003 => self.fcsr & 0xff,
            _ => return None,
        })
    }

    /// Is this CSR an UNPRIVILEGED one this machine implements — readable from U-mode?
    /// Bits 9:8 of a CSR number are the lowest privilege that may access it, and `00` is
    /// user: the float CSRs and the vector unit's. QEMU reads every one of them from
    /// U-mode (`V-009` (i-d) probe, section U). `time` (0xc01) is a user-level number too,
    /// but whether U-mode may read it is `scounteren`'s to say, and this machine has no
    /// `scounteren`; QEMU with `mcounteren = 0` traps it, which is what it already does
    /// here.
    fn user_csr(csr: u16) -> bool {
        matches!(csr, 0x001..=0x003 | 0x008..=0x00a | 0x00f | 0xc20..=0xc22)
    }

    /// Is `rm` — an instruction's rounding-mode field — a mode no hart may execute?
    /// `5` and `6` are reserved, and `DYN` (7) defers to `frm`, where 5, 6 and 7 are all
    /// invalid. Each is an ILLEGAL INSTRUCTION on QEMU (`V-009` (i-d) probe, D3..D15), and
    /// became reachable when `frm` became writable.
    fn rm_reserved(&self, rm: u32) -> bool {
        let effective = if rm == fp::DYN { self.frm() } else { rm };
        effective > fp::RMM
    }

    /// Write a CSR that [`Machine::csr_read`] accepted. WARL fields are narrowed here
    /// rather than at the call site, so a read after a write returns what the machine
    /// actually holds and not what the program asked for.
    fn csr_write(&mut self, csr: u16, value: u64) {
        match csr {
            0x100 => self.csr.sstatus = value & SSTATUS_MASK,
            0x104 => self.csr.sie = value & SIE_MASK,
            // Only `SSIP` survives a write to `sip`. `STIP` is the firmware's — this
            // machine IS the firmware, and it sets that bit from the deadline — and
            // `SEIP` belongs to an interrupt controller that does not exist. Dropping
            // them is the spec's own rule for a read-only field in a writable register,
            // and it is why `spec/timer.sas` stops its clock with `set_timer` rather than
            // by clearing the bit: clearing the bit would not have worked on hardware
            // either.
            0x144 => self.csr.sip = value & SI_SSI,
            0x105 => self.csr.stvec = value,
            0x140 => self.csr.sscratch = value,
            // IALIGN is 32 here — compressed encodings are not decoded — so the low bit
            // of an address is never meaningful and the spec makes it read as zero.
            0x141 => self.csr.sepc = value & !1,
            0x142 => self.csr.scause = value,
            0x143 => self.csr.stval = value,
            // `satp.MODE` is WARL, and the spec's own rule for a mode a hart does not
            // implement is that the whole write **has no effect**. That is taken here
            // rather than narrowing the field to Sv39, because the two differ in exactly
            // the way this VM cares about: a kernel that asks for Sv48 and reads back
            // Sv39 has been told its four-level tables are live while three levels are
            // walked. Leaving the register alone means the read-back disagrees with the
            // write, which is a kernel's own way of finding out it was refused — and
            // `spec/paging.sas` reads `satp` back for precisely that reason.
            0x180 => {
                let mode = value >> 60;
                if mode == 0 || mode == SATP_SV39 >> 60 {
                    self.csr.satp = value;
                }
            }
            // `vstart` holds an element index, and with VLEN 128 the largest group has
            // sixteen elements; seven bits is log2(VLEN), the width the spec gives it.
            0x008 => {
                self.vec.vstart = value & (vector::VLEN - 1);
                self.csr.sstatus |= SSTATUS_VS;
            }
            // `vxsat` keeps one bit, `vxrm` two, `vcsr` is both (`vxrm` << 1 | `vxsat`).
            // A write sets VS Dirty, as on QEMU (`probe_ctl.S` C, "VS after").
            //
            // QEMU 10.1.0 DIFFERS ON ONE: a write to `vxrm` itself stores the whole value
            // (0xff reads back 0xff, and `vcsr` then 0x1fe), where the specification makes
            // `vxrm` a two-bit field. This machine keeps two bits, as the coordinator's
            // ruling for (i-e) specifies; `vcsr` and `vxsat` writes QEMU masks as here.
            // Listed in [`KNOWN_QEMU_DIFFERENCES`].
            0x009 => {
                self.vec.vxsat = value & 1;
                self.csr.sstatus |= SSTATUS_VS;
            }
            0x00a => {
                self.vec.vxrm = value & 3;
                self.csr.sstatus |= SSTATUS_VS;
            }
            0x00f => {
                self.vec.vxrm = (value >> 1) & 3;
                self.vec.vxsat = value & 1;
                self.csr.sstatus |= SSTATUS_VS;
            }
            // Each keeps only its legal bits: five flags, three mode bits, eight in all.
            // `fcsr` bits 10:8 — draft V's `vxrm`/`vxsat` mirror — do not stick, as on
            // QEMU with V on or off. `frm` takes any 3-bit value, the invalid 5..7
            // included: they are refused when an instruction USES them, not here. Any
            // write sets `FS` Dirty, an unchanged value too (QEMU, probe B3).
            0x001 => {
                self.fcsr = (self.fcsr & !0x1f) | (value & 0x1f);
                self.mark_fp_dirty();
            }
            0x002 => {
                self.fcsr = (self.fcsr & 0x1f) | ((value & 0x7) << 5);
                self.mark_fp_dirty();
            }
            0x003 => {
                self.fcsr = value & 0xff;
                self.mark_fp_dirty();
            }
            _ => unreachable!("csr_write is only reached for a CSR csr_read accepted"),
        }
    }

    /// Take a trap into supervisor mode: the four registers the handler reads, the two
    /// `sstatus` bits `sret` puts back, and the jump to `stvec`.
    ///
    /// `stvec`'s `MODE` selects direct or vectored, and vectored spreads **interrupts
    /// only** across the table: an interrupt of cause `n` enters at `BASE + 4n`, an
    /// exception at `BASE` either way. Until `F-001c2b3` this machine raised no interrupt
    /// and the distinction could not be observed, so it was not made.
    fn trap(&mut self, cause: u64, tval: u64, epc: u64) -> u64 {
        let sie = self.csr.sstatus & SSTATUS_SIE != 0;
        self.csr.sstatus &= !(SSTATUS_SIE | SSTATUS_SPIE);
        if sie {
            self.csr.sstatus |= SSTATUS_SPIE;
        }
        // `SPP` is the privilege the trap came *from*, and it is what `sret` will return
        // to. Written from the mode field rather than left set: a trap from U-mode that
        // recorded supervisor would send the handler's `sret` back into the kernel.
        self.csr.sstatus &= !SSTATUS_SPP;
        if self.mode == Privilege::Supervisor {
            self.csr.sstatus |= SSTATUS_SPP;
        }
        // A trap is always *into* supervisor mode here; there is no mode above it.
        self.mode = Privilege::Supervisor;
        self.csr.sepc = epc;
        self.csr.scause = cause;
        self.csr.stval = tval;
        let base = self.csr.stvec & !3;
        if cause & INTERRUPT != 0 && self.csr.stvec & 3 == 1 {
            base + 4 * (cause & !INTERRUPT)
        } else {
            base
        }
    }

    /// Take a trap, or say there is nowhere to take it.
    ///
    /// # Errors
    /// [`Halt::Undelivered`] when `stvec` is zero — the same answer `ebreak` and a page
    /// fault give, and for the same reason: delivering to address 0 executes the rubble
    /// there.
    fn deliver(&mut self, cause: u64, tval: u64, pc: u64) -> Result<u64, Halt> {
        if self.csr.stvec == 0 {
            return Err(Halt::Undelivered { pc, cause });
        }
        Ok(self.trap(cause, tval, pc))
    }

    /// An illegal instruction: cause 2, `stval` the instruction itself, and the program
    /// counter left at the handler. `None` means the machine is still running, exactly as
    /// it means everywhere else in [`Machine::step_inner`] — which is why this commits
    /// `pc` rather than reporting where to put it.
    ///
    /// Three things reach it. A **privileged instruction executed in user mode** is the
    /// original, and the larger set. A **write to a read-only CSR** is the second, and it
    /// is illegal from S-mode too: `time` is a counter, and a kernel that could set it
    /// could run its own deadlines backwards. A **float or vector instruction while its
    /// unit is Off** (`sstatus.FS`/`VS`) is the third, since `V-009` part (i-c).
    fn illegal(&mut self, word: u32, pc: u64) -> Option<Halt> {
        match self.deliver(2, u64::from(word), pc) {
            Ok(handler) => {
                self.pc = handler;
                None
            }
            Err(h) => Some(h),
        }
    }

    /// An OP-IMM-32 / OP-32 word with no arm in [`Machine::step_inner`]. `V-009` (i-e)
    /// probed all 65,536 of them (funct3 x bits 31:25 x the rs2 field) on QEMU
    /// (`probe_w.S`, `tests/data/v009e_word_qemu.tsv`): every one QEMU does not execute
    /// traps cause 2, and here it is now an ILLEGAL INSTRUCTION too — `slliw` with
    /// `shamt[5]` set, `funct7` 0x20 on `slliw`, OP-IMM-32 funct3 2/3/4/6/7, every
    /// undefined OP-32 pair. Until (i-e) each halted `Unimplemented`.
    ///
    /// The words QEMU EXECUTES and this machine does not are Zba and Zbb's word forms —
    /// `add.uw`, `sh1add.uw`..`sh3add.uw`, `slli.uw`, `rolw`, `rorw`, `roriw`, `clzw`,
    /// `ctzw`, `cpopw`, `zext.h` — enabled by default on QEMU's `rv64` CPU. Trapping one
    /// would hand a handler an instruction real hardware runs; they halt `Unimplemented`,
    /// BY NAME, as every unimplemented extension does here.
    fn reserved_word_form(&mut self, word: u32, pc: u64) -> Option<Halt> {
        let (f3, f7, rs2) = ((word >> 12) & 7, word >> 25, (word >> 20) & 0x1f);
        let zb = match word & 0x7f {
            0x1b => {
                matches!((f3, f7), (1, 0x04 | 0x05) | (5, 0x30))
                    || (f3, f7) == (1, 0x30) && rs2 <= 2
            }
            _ => {
                matches!(
                    (f3, f7),
                    (0, 0x04) | (1, 0x30) | (2 | 4 | 6, 0x10) | (5, 0x30)
                ) || (f3, f7, rs2) == (4, 0x04, 0)
            }
        };
        if zb {
            Some(Halt::Unimplemented {
                pc,
                word,
                opcode: (word & 0x7f) as u8,
            })
        } else {
            self.illegal(word, pc)
        }
    }

    /// `SI_STI` if the deadline has passed, nothing otherwise.
    ///
    /// **Computed, never stored.** `sip.STIP` on hardware is written by the firmware when
    /// `mtime` reaches `mtimecmp`; this interpreter *is* the firmware, and the cheapest
    /// honest way to be a firmware that never forgets is to hold no copy of the answer.
    fn pending_timer(&self) -> u64 {
        match self.timecmp {
            Some(deadline) if self.time >= deadline => SI_STI,
            _ => 0,
        }
    }

    /// The interrupt to take before the next instruction, if any: the highest-priority bit
    /// that is both pending and enabled, at a moment the hart may be interrupted.
    ///
    /// `sstatus.SIE` gates this **only in S-mode**. In U-mode an S-mode interrupt is
    /// always taken, which is not a special case but the point of the bit: `SIE` is the
    /// kernel saying "not while I am in here", and a user program is never in there.
    #[inline]
    fn interrupt(&self) -> Option<u64> {
        // NOTHING ENABLED, NOTHING TAKEN (`W-382`): `pending` below is masked by `sie`,
        // so with `sie` zero it is zero whatever the clock says. The common case, and one
        // test; the clock still advanced in `step` before this was asked.
        if self.csr.sie == 0 {
            return None;
        }
        if self.mode == Privilege::Supervisor && self.csr.sstatus & SSTATUS_SIE == 0 {
            return None;
        }
        let pending = (self.csr.sip | self.pending_timer()) & self.csr.sie;
        INTERRUPTS
            .iter()
            .find(|(bit, _)| pending & bit != 0)
            .map(|(_, cause)| *cause)
    }

    /// Run until the machine halts for one of the reasons in [`Halt`].
    ///
    /// `budget` bounds the run. A browser tab must not be wedged by a program that never
    /// finishes, and "it hung" is a worse demo than "it stopped and said why".
    ///
    /// # Errors
    /// Never — it returns the reason it stopped, which is always one of [`Halt`]'s
    /// variants. The `Result` is the reason, not an exception.
    pub fn run(&mut self, budget: u64, out: &mut impl Output) -> Halt {
        // A run long enough to repay the table's allocation goes through the decoded
        // cache (`W-382`, see [`DecodeCache`]); a short one steps exactly as `step` does.
        // The two paths retire the same instructions with the same effects — the cache
        // only spares re-decoding a word it has already decoded.
        if budget >= DecodeCache::MIN_BUDGET {
            let mut cache = DecodeCache::new();
            for _ in 0..budget {
                if let Some(h) = self.step_cached(&mut cache, out) {
                    return h;
                }
            }
        } else {
            for _ in 0..budget {
                if let Some(h) = self.step(out) {
                    return h;
                }
            }
        }
        Halt::StepLimit { pc: self.pc }
    }

    /// Serve every chain the driver has offered on queue `q`, then raise the used-buffer
    /// interrupt bit — what a QueueNotify store does, at once or, under a deferred
    /// completion, later. `pc` and `addr` are the notifying store's, so a refusal names it.
    ///
    /// # Errors
    /// [`Halt::Device`] naming the chain's refusal; chains served before it stay served.
    fn serve_notified(&mut self, q: usize, pc: u64, addr: u64) -> Result<(), Halt> {
        let queue = self.virtio.queue(q);
        let mut cursor = self.virtio.queues[q].last_avail;
        let mut served = false;
        loop {
            let base = self.base;
            let gpu = &mut self.virtio.gpu;
            match virtqueue::process(&mut self.mem, base, &queue, &mut cursor, |mem, req| {
                gpu.serve(mem, base, req)
            }) {
                Ok(Some(_)) => served = true,
                Ok(None) => break,
                Err(r) => {
                    self.virtio.queues[q].last_avail = cursor;
                    if served {
                        self.virtio.interrupt_status |= 1;
                    }
                    return Err(Halt::Device {
                        pc,
                        addr,
                        why: r.name(),
                    });
                }
            }
        }
        self.virtio.queues[q].last_avail = cursor;
        if served {
            self.virtio.interrupt_status |= 1;
        }
        Ok(())
    }

    /// THE DEFERRED COMPLETION'S CLOCK: one instruction has begun, so every waiting queue
    /// is one nearer, and a queue that reaches zero is served now, before that instruction.
    /// `Some` is a refusal found while serving, which ends the step.
    #[cold]
    fn complete_deferred(&mut self) -> Option<Halt> {
        for q in 0..virtio_mmio::QUEUES {
            let Some(p) = self.virtio.pending[q].as_mut() else {
                continue;
            };
            p.left -= 1;
            if p.left == 0 {
                let p = *p;
                self.virtio.pending[q] = None;
                self.virtio.waiting -= 1;
                if let Err(h) = self.serve_notified(q, p.pc, p.addr) {
                    return Some(h);
                }
            }
        }
        None
    }

    /// [`Machine::step`] through a [`DecodeCache`]: the same clock, the same interrupt
    /// check, the same handling of a page fault and of a WAIT — only the instruction in
    /// the middle is executed from its cached decoding when it is one of the common
    /// integer forms ([`Op`]), and by [`Machine::step_inner`] otherwise.
    #[inline]
    fn step_cached(&mut self, cache: &mut DecodeCache, out: &mut impl Output) -> Option<Halt> {
        if let Some(taken) = self.step_prelude() {
            return taken;
        }
        let r = self.exec_cached(cache, out);
        self.step_epilogue(r)
    }

    /// The part of a step BEFORE the instruction: the clock and the interrupt check.
    /// `Some` is the step's whole answer (an interrupt taken or undeliverable).
    #[inline(always)]
    fn step_prelude(&mut self) -> Option<Option<Halt>> {
        // The clock advances first, and it advances even on the step that takes an
        // interrupt rather than an instruction: a hart whose time stopped while it was
        // being interrupted could not reach a deadline from inside a handler.
        self.time = self.time.wrapping_add(1);
        // A deferred virtio completion lands here, before the instruction (one word tested
        // per step; never true unless `YANTRA_VIRTIO_DEFER` is set).
        if self.virtio.waiting != 0
            && let Some(h) = self.complete_deferred()
        {
            return Some(Some(h));
        }
        // An interrupt is taken BEFORE the instruction at `pc`, not after it — that
        // instruction has not run, and `sepc` naming it is what makes `sret` a resume
        // rather than a skip. It is also why `spec/timer.sas` can escape a `jal x0, .`:
        // the jump never executes, so [`Halt::SpinForever`] is never reached.
        if let Some(cause) = self.interrupt() {
            let pc = self.pc;
            // `stval` is zero for an interrupt: there is no faulting address, and a
            // handler that read one would be reading the last exception's.
            return Some(match self.deliver(INTERRUPT | cause, 0, pc) {
                Ok(handler) => {
                    self.pc = handler;
                    None
                }
                Err(h) => Some(h),
            });
        }
        None
    }

    /// The part of a step AFTER the instruction: a page fault delivered when a handler is
    /// installed, and a WAIT's `pc` moved past its store.
    #[inline(always)]
    fn step_epilogue(&mut self, r: Option<Halt>) -> Option<Halt> {
        match r {
            None => None,
            Some(Halt::PageFault { pc, addr, cause }) if self.csr.stvec != 0 => {
                self.pc = self.trap(cause, addr, pc);
                None
            }
            // A WAIT HAS RETIRED, unlike every other halt a store returns: the store
            // arms return before `pc = next`, so it is moved here, once, for all of them.
            // `pc + 4` is right because only a plain store or `fsw`/`fsd` reaches the
            // address — an AMO or an `lr` loads it first and `load` refuses it, so an
            // `sc` can never hold a reservation on it — and none of those writes `rd`
            // or branches. The clock already counted it, at the top of `step`.
            // AND IT RESTS ON IALIGN 32 (see `step_inner`'s fetch, "compressed encodings
            // are not decoded"): if RVC is ever decoded, this must take the store's own
            // length, 2 or 4, not a constant.
            Some(Halt::Wait { pc }) => {
                self.pc = pc.wrapping_add(4);
                Some(Halt::Wait { pc })
            }
            other => other,
        }
    }

    /// One instruction through the decoded cache, with a page fault reported rather than
    /// delivered — [`Machine::step_inner`]'s contract exactly.
    ///
    /// EVERY PATH THAT IS NOT PLAIN RAM AND A COMMON INTEGER FORM GOES TO `step_inner`
    /// BEFORE ANYTHING IS CHANGED: a fetch that needs the walk or a device, a load or
    /// store whose address is not RAM above [`DEVICE_TOP`] (a device, the counter, a
    /// fault, a bound refusal), and every opcode [`Op::Slow`] stands for. `step_inner`
    /// then does the whole instruction from the beginning, as it always did, so every
    /// refusal and every device is answered by the original code.
    #[inline(always)]
    fn exec_cached(&mut self, cache: &mut DecodeCache, out: &mut impl Output) -> Option<Halt> {
        let pc = self.pc;
        let Ok(fetch) = self.translate(pc, Access::Fetch, pc) else {
            return self.step_inner(out);
        };
        let word = if fetch >= DEVICE_TOP
            && let Some(off) = fetch.checked_sub(self.base)
            && let Some(w) = ram_read(&self.mem, off, 4)
        {
            w as u32
        } else {
            return self.step_inner(out);
        };
        // THE VALIDATION IS THE WORD ITSELF. A decoding is a pure function of the 32-bit
        // word, so an entry is right for this fetch exactly when it was decoded from the
        // word now in RAM. That makes the cache correct under EVERY writer of `mem` — a
        // store, an AMO, a `patra` read landing in RAM, a virtqueue response, the host
        // injecting input, a test poking `mem` between runs — with no invalidation for
        // any of them to forget. Self-modifying code is the case it exists for
        // (`tests/w382_decode_cache.rs`).
        let slot = &mut cache.entries[(fetch >> 2) as usize & DecodeCache::MASK];
        if slot.word != word {
            *slot = Decoded::of(word);
        }
        let d = *slot;
        let (rd, rs1, rs2) = (usize::from(d.rd), usize::from(d.rs1), usize::from(d.rs2));
        let imm = d.imm;
        let mut next = pc.wrapping_add(4);
        match d.op {
            Op::Slow => return self.step_inner(out),
            Op::Lui => self.x[rd] = imm,
            Op::Auipc => self.x[rd] = pc.wrapping_add(imm),
            Op::Jal => {
                self.x[rd] = pc.wrapping_add(4);
                next = pc.wrapping_add(imm);
                if next == pc {
                    self.x[0] = 0;
                    return Some(Halt::SpinForever { pc });
                }
            }
            Op::Jalr => {
                let t = self.x[rs1].wrapping_add(imm) & !1;
                self.x[rd] = pc.wrapping_add(4);
                next = t;
            }
            Op::Beq | Op::Bne | Op::Blt | Op::Bge | Op::Bltu | Op::Bgeu => {
                let (a, b) = (self.x[rs1], self.x[rs2]);
                let take = match d.op {
                    Op::Beq => a == b,
                    Op::Bne => a != b,
                    Op::Blt => (a as i64) < (b as i64),
                    Op::Bge => (a as i64) >= (b as i64),
                    Op::Bltu => a < b,
                    _ => a >= b,
                };
                if take {
                    next = pc.wrapping_add(imm);
                }
            }
            Op::Lb | Op::Lh | Op::Lw | Op::Ld | Op::Lbu | Op::Lhu | Op::Lwu => {
                let (width, signed) = match d.op {
                    Op::Lb => (1, true),
                    Op::Lh => (2, true),
                    Op::Lw => (4, true),
                    Op::Ld => (8, false),
                    Op::Lbu => (1, false),
                    Op::Lhu => (2, false),
                    _ => (4, false),
                };
                let addr = self.x[rs1].wrapping_add(imm);
                let Ok(pa) = self.translate_span(addr, width, Access::Load, pc) else {
                    return self.step_inner(out);
                };
                // Above `DEVICE_TOP` the counter (`COUNTER`) cannot answer either.
                if pa >= DEVICE_TOP
                    && let Some(off) = pa.checked_sub(self.base)
                    && let Some(v) = ram_read(&self.mem, off, width)
                {
                    self.x[rd] = if signed { sign_extend(v, width) } else { v };
                } else {
                    return self.step_inner(out);
                }
            }
            Op::Sb | Op::Sh | Op::Sw | Op::Sd => {
                let width = match d.op {
                    Op::Sb => 1,
                    Op::Sh => 2,
                    Op::Sw => 4,
                    _ => 8,
                };
                let addr = self.x[rs1].wrapping_add(imm);
                let Ok(pa) = self.translate_span(addr, width, Access::Store, pc) else {
                    return self.step_inner(out);
                };
                let writable = self.mem.len().min(self.store_limit);
                if pa >= DEVICE_TOP
                    && let Some(off) = pa.checked_sub(self.base)
                    && let Ok(at) = usize::try_from(off)
                    && at < writable
                    && width <= writable - at
                {
                    out.stored(at, width);
                    let bytes = self.x[rs2].to_le_bytes();
                    self.mem[at..at + width].copy_from_slice(&bytes[..width]);
                } else {
                    return self.step_inner(out);
                }
            }
            Op::Addi => self.x[rd] = self.x[rs1].wrapping_add(imm),
            Op::Slli => self.x[rd] = self.x[rs1] << (imm & 0x3f),
            Op::Srli => self.x[rd] = self.x[rs1] >> (imm & 0x3f),
            Op::Srai => self.x[rd] = ((self.x[rs1] as i64) >> (imm & 0x3f)) as u64,
            Op::Slti => self.x[rd] = u64::from((self.x[rs1] as i64) < (imm as i64)),
            Op::Sltiu => self.x[rd] = u64::from(self.x[rs1] < imm),
            Op::Xori => self.x[rd] = self.x[rs1] ^ imm,
            Op::Ori => self.x[rd] = self.x[rs1] | imm,
            Op::Andi => self.x[rd] = self.x[rs1] & imm,
            _ => {
                // OP and the M extension: the same arithmetic as `step_inner`'s OP arm.
                let (a, b) = (self.x[rs1], self.x[rs2]);
                self.x[rd] = match d.op {
                    Op::Add => a.wrapping_add(b),
                    Op::Sub => a.wrapping_sub(b),
                    Op::Sll => a << (b & 0x3f),
                    Op::Slt => u64::from((a as i64) < (b as i64)),
                    Op::Sltu => u64::from(a < b),
                    Op::Xor => a ^ b,
                    Op::Srl => a >> (b & 0x3f),
                    Op::Sra => ((a as i64) >> (b & 0x3f)) as u64,
                    Op::Or => a | b,
                    Op::And => a & b,
                    Op::Mul => a.wrapping_mul(b),
                    Op::Mulh => (((a as i64 as i128) * (b as i64 as i128)) >> 64) as u64,
                    Op::Mulhsu => (((a as i64 as i128) * (b as u128 as i128)) >> 64) as u64,
                    Op::Mulhu => (((a as u128) * (b as u128)) >> 64) as u64,
                    Op::Div => match (a as i64, b as i64) {
                        (_, 0) => u64::MAX,
                        (i64::MIN, -1) => i64::MIN as u64,
                        (x, y) => x.wrapping_div(y) as u64,
                    },
                    Op::Divu => {
                        if b == 0 {
                            u64::MAX
                        } else {
                            a / b
                        }
                    }
                    Op::Rem => match (a as i64, b as i64) {
                        (x, 0) => x as u64,
                        (i64::MIN, -1) => 0,
                        (x, y) => x.wrapping_rem(y) as u64,
                    },
                    _ => {
                        // Op::Remu — the last form `Decoded::of` produces.
                        if b == 0 { a } else { a % b }
                    }
                };
            }
        }
        self.x[0] = 0;
        self.pc = next;
        None
    }

    /// `fcsr.frm`, the dynamic rounding mode.
    #[must_use]
    pub fn frm(&self) -> u32 {
        ((self.fcsr >> 5) & 0x7) as u32
    }

    /// `fcsr.fflags`, the five accrued exception flags.
    #[must_use]
    pub fn fflags(&self) -> u64 {
        self.fcsr & 0x1f
    }

    /// Accrue exception flags. **They accumulate and are never cleared here** — that is
    /// the architectural contract: only a write to `fflags`/`fcsr` clears them, so a
    /// program can run a whole kernel and ask once at the end whether anything was
    /// invalid.
    fn set_fflags(&mut self, flags: u64) {
        self.fcsr |= flags & 0x1f;
    }

    /// Record that the FPU state is live, by setting `sstatus.FS` to `dirty`.
    ///
    /// Every instruction that writes an `f` register calls this. It is what makes keeping
    /// `FS` in `SSTATUS_MASK` meaningful rather than decorative: a supervisor that saves
    /// the register file only when `FS` says dirty needs something to have set it.
    fn mark_fp_dirty(&mut self) {
        self.csr.sstatus |= SSTATUS_FS;
    }

    /// OP-FP (`0x53`) — the whole scalar floating-point operation space.
    ///
    /// The arithmetic and the two format conversions answer `(bits, flags, Some(single))`
    /// through [`op2`] and are written back in one place; every other arm is the `match`
    /// below it.
    ///
    /// Split out of [`Machine::step_inner`] because that function's `match` is already
    /// long and this is a second, independently-indexed encoding table. `funct7` is
    /// `(op5 << 2) | fmt`, and the arms are written as the funct7 VALUE so a reader can
    /// check them against the specification's table without doing the shift by hand.
    #[allow(clippy::too_many_arguments, clippy::cast_possible_truncation)]
    fn op_fp(
        &mut self,
        word: u32,
        opcode: u8,
        pc: u64,
        rd: usize,
        rs1: usize,
        rs2: usize,
        funct3: u32,
        funct7: u32,
    ) -> Option<Halt> {
        let rm = funct3;
        let unimpl = || Some(Halt::Unimplemented { pc, word, opcode });

        // THE MODE THE ARITHMETIC ROUNDS UNDER — any of the five, static or DYN through
        // `frm` (`V-009` (i-f), owner ruling (b)). Until (i-f) every mode but RNE halted
        // here by name (V-001's refusal: Rust rounds to nearest only, and rounding to
        // nearest under `rtz` would have agreed with nobody). The arithmetic rounds in
        // software since (i-e) ([`fp::add`] and the rest), checked against QEMU in all five
        // modes and both forms, so the refusal's premise is gone. A RESERVED mode (5, 6, or
        // DYN with frm 5..7) never reaches here: it is an illegal instruction
        // (`rm_reserved`, in `step_inner`).
        let erm = if rm == fp::DYN { self.frm() } else { rm };

        // Operands in their encodings: a double's 64 bits, or a single's 32 read through
        // the NaN-boxing rule (an unboxed register is the canonical NaN).
        let s_in = |r: usize| u64::from(fp::unbox_s(self.f[r]).to_bits());
        let d_in = |r: usize| self.f[r];

        // THE ARITHMETIC ROUNDS IN SOFTWARE since `V-009` part (i-e), so `NX`, `OF` and
        // `UF` accrue exactly as on QEMU, `NV` and `DZ` with them; see `fp`'s margin.
        let (bits, flags, single) = match funct7 {
            0x00 => op2(fp::add(fp::F32, s_in(rs1), s_in(rs2), erm), true),
            0x01 => op2(fp::add(fp::F64, d_in(rs1), d_in(rs2), erm), false),
            0x04 => op2(fp::sub(fp::F32, s_in(rs1), s_in(rs2), erm), true),
            0x05 => op2(fp::sub(fp::F64, d_in(rs1), d_in(rs2), erm), false),
            0x08 => op2(fp::mul(fp::F32, s_in(rs1), s_in(rs2), erm), true),
            0x09 => op2(fp::mul(fp::F64, d_in(rs1), d_in(rs2), erm), false),
            // DIVISION SETS `DZ` AND THE RESULT IS STILL AN INFINITY, not a trap.
            0x0c => op2(fp::div(fp::F32, s_in(rs1), s_in(rs2), erm), true),
            0x0d => op2(fp::div(fp::F64, d_in(rs1), d_in(rs2), erm), false),
            // fsqrt — one operand; `rs2` must be 0 and a non-zero one is not this
            // instruction.
            0x2c | 0x2d if rs2 != 0 => return unimpl(),
            0x2c => op2(fp::sqrt(fp::F32, s_in(rs1), erm), true),
            0x2d => op2(fp::sqrt(fp::F64, d_in(rs1), erm), false),
            // fcvt.s.d (0x20, rs2 = 1) and fcvt.d.s (0x21, rs2 = 0)
            0x20 if rs2 != 1 => return unimpl(),
            0x20 => op2(fp::convert(fp::F64, fp::F32, d_in(rs1), erm), true),
            0x21 if rs2 != 0 => return unimpl(),
            0x21 => op2(fp::convert(fp::F32, fp::F64, s_in(rs1), erm), false),
            _ => (0, 0, None),
        };
        if let Some(single) = single {
            self.set_fflags(flags);
            self.f[rd] = if single {
                0xffff_ffff_0000_0000 | bits
            } else {
                bits
            };
            self.mark_fp_dirty();
            return None;
        }

        match funct7 {
            // fsgnj / fsgnjn / fsgnjx — SIGN INJECTION IS NOT ARITHMETIC. It moves bits,
            // sets no flag, and must not quieten a signalling NaN.
            0x10 | 0x11 => {
                let wide = funct7 == 0x11;
                let (sign_bit, mask) = if wide {
                    (1u64 << 63, u64::MAX)
                } else {
                    (1u64 << 31, 0xffff_ffff)
                };
                let a = self.f[rs1] & mask;
                let b = self.f[rs2] & mask;
                let s = match funct3 {
                    0x0 => b & sign_bit,
                    0x1 => !b & sign_bit,
                    0x2 => (a ^ b) & sign_bit,
                    _ => return unimpl(),
                };
                let v = (a & !sign_bit) | s;
                self.f[rd] = if wide { v } else { 0xffff_ffff_0000_0000 | v };
                self.mark_fp_dirty();
            }
            // fmin / fmax — the NaN rule is RISC-V's, not Rust's. See `fp::minmax_d`.
            0x14 | 0x15 => {
                let max = match funct3 {
                    0x0 => false,
                    0x1 => true,
                    _ => return unimpl(),
                };
                if funct7 == 0x14 {
                    let (v, fl) =
                        fp::minmax_s(fp::unbox_s(self.f[rs1]), fp::unbox_s(self.f[rs2]), max);
                    self.set_fflags(fl);
                    self.f[rd] = fp::box_s(v);
                } else {
                    let (v, fl) = fp::minmax_d(
                        f64::from_bits(self.f[rs1]),
                        f64::from_bits(self.f[rs2]),
                        max,
                    );
                    self.set_fflags(fl);
                    self.f[rd] = v.to_bits();
                }
                self.mark_fp_dirty();
            }
            // feq / flt / fle — the result is an INTEGER register.
            0x50 | 0x51 => {
                if funct3 > 2 {
                    return unimpl();
                }
                let (v, fl) = if funct7 == 0x50 {
                    fp::compare_s(fp::unbox_s(self.f[rs1]), fp::unbox_s(self.f[rs2]), rm)
                } else {
                    fp::compare_d(f64::from_bits(self.f[rs1]), f64::from_bits(self.f[rs2]), rm)
                };
                self.set_fflags(fl);
                self.x[rd] = u64::from(v);
            }
            // fcvt.{w,wu,l,lu}.{s,d} — float to integer, ALL FIVE ROUNDING MODES.
            0x60 | 0x61 => {
                let x = if funct7 == 0x60 {
                    f64::from(fp::unbox_s(self.f[rs1]))
                } else {
                    f64::from_bits(self.f[rs1])
                };
                let (signed, bits) = match rs2 {
                    0 => (true, 32),
                    1 => (false, 32),
                    2 => (true, 64),
                    3 => (false, 64),
                    _ => return unimpl(),
                };
                let effective = if rm == fp::DYN { self.frm() } else { rm };
                if effective > fp::RMM {
                    return unimpl();
                }
                let (v, fl) = fp::cvt_to_int(x, effective, signed, bits);
                self.set_fflags(fl);
                // `fcvt.w`/`wu` SIGN-EXTEND their 32-bit result into the 64-bit register,
                // both of them — `wu` too, which reads wrong and is what the spec says.
                self.x[rd] = if bits == 32 {
                    i64::from(v as i32) as u64
                } else {
                    v
                };
            }
            // fcvt.{s,d}.{w,wu,l,lu} — integer to float.
            0x68 | 0x69 => {
                let src = self.x[rs1];
                let v: i128 = match rs2 {
                    0 => i128::from(src as i32),
                    1 => i128::from(src as u32),
                    2 => i128::from(src as i64),
                    3 => i128::from(src),
                    _ => return unimpl(),
                };
                let f = if funct7 == 0x68 { fp::F32 } else { fp::F64 };
                // ONE ROUNDING, INTO THE TARGET, UNDER THE INSTRUCTION'S MODE. Until (i-e)
                // "exact" was asked of the DOUBLE conversion, and the single went through
                // it — `fcvt.s.w` of 2^24 + 1 read exact and flagged nothing, and a wide
                // integer was rounded twice; until (i-f) an inexact one under a mode other
                // than RNE halted.
                let (bits, flags) = fp::from_int(f, v, erm);
                self.set_fflags(flags);
                self.f[rd] = if funct7 == 0x68 {
                    0xffff_ffff_0000_0000 | bits
                } else {
                    bits
                };
                self.mark_fp_dirty();
            }
            // fmv.x.w / fmv.x.d (funct3 0) and fclass (funct3 1) — f register to x.
            0x70 | 0x71 => match funct3 {
                0x0 => {
                    // `fmv.x.w` sign-extends the low 32 bits; `fmv.x.d` moves all 64.
                    self.x[rd] = if funct7 == 0x70 {
                        i64::from(self.f[rs1] as i32) as u64
                    } else {
                        self.f[rs1]
                    };
                }
                0x1 => {
                    self.x[rd] = if funct7 == 0x70 {
                        fp::fclass_s(fp::unbox_s(self.f[rs1]))
                    } else {
                        fp::fclass_d(f64::from_bits(self.f[rs1]))
                    };
                }
                _ => return unimpl(),
            },
            // fmv.w.x / fmv.d.x — x register to f, RAW BITS, no conversion.
            0x78 | 0x79 => {
                if funct3 != 0 || rs2 != 0 {
                    return unimpl();
                }
                self.f[rd] = if funct7 == 0x78 {
                    0xffff_ffff_0000_0000 | (self.x[rs1] & 0xffff_ffff)
                } else {
                    self.x[rs1]
                };
                self.mark_fp_dirty();
            }
            _ => return unimpl(),
        }
        None
    }

    /// One instruction. `None` means keep going.
    ///
    /// A **page fault is an exception, not a stop**: if the instruction faults and the
    /// program has installed a handler, the fault is delivered to it here — `sepc`,
    /// `scause` and `stval` filled by [`Machine::trap`] and the program counter moved to
    /// `stvec` — and this returns `None`, because the machine is still running. That is
    /// why the delivery lives out here rather than at the six places a translation can
    /// fail: [`Machine::step_inner`] leaves `pc` and the registers untouched when it
    /// returns a fault, so the instruction has taken no effect and the trap can be taken
    /// against the state that entered it.
    ///
    /// With `stvec` zero there is nowhere to deliver it, and [`Halt::PageFault`] comes out
    /// instead — the same choice `ebreak` makes, and for the same reason: on metal it
    /// would trap to address 0 and execute the rubble there.
    pub fn step(&mut self, out: &mut impl Output) -> Option<Halt> {
        // The clock, then any interrupt, then the instruction, then the delivery of a
        // fault or the retirement of a WAIT — `step_prelude` and `step_epilogue` hold
        // the first and last, shared with `step_cached` (`W-382`) so the cached path
        // cannot drift from this one.
        if let Some(taken) = self.step_prelude() {
            return taken;
        }
        let r = self.step_inner(out);
        self.step_epilogue(r)
    }

    /// One instruction, with a page fault reported rather than delivered. See
    /// [`Machine::step`], which is the entry point.
    #[allow(clippy::too_many_lines)]
    fn step_inner(&mut self, out: &mut impl Output) -> Option<Halt> {
        let pc = self.pc;
        // A four-byte fetch never crosses a page: IALIGN is 32 here, so `pc` is always
        // four-byte aligned and 4096 divides by four.
        let fetch = match self.translate(pc, Access::Fetch, pc) {
            Ok(pa) => pa,
            Err(h) => return Some(h),
        };
        // THE FETCH, RAM FIRST (`W-382`): one 32-bit read when `fetch` is RAM above every
        // device; otherwise the general `load`, which is what a fetch always was — a fetch
        // from a device window still reads that window, and one past RAM still refuses.
        let word = if fetch >= DEVICE_TOP
            && let Some(off) = fetch.checked_sub(self.base)
            && let Some(w) = ram_read(&self.mem, off, 4)
        {
            w as u32
        } else {
            match self.load(fetch, 4, false, pc) {
                Ok(w) => w as u32,
                Err(h) => return Some(h),
            }
        };
        let opcode = (word & 0x7f) as u8;
        let rd = ((word >> 7) & 0x1f) as usize;
        let rs1 = ((word >> 15) & 0x1f) as usize;
        let rs2 = ((word >> 20) & 0x1f) as usize;
        let funct3 = (word >> 12) & 0x7;
        let funct7 = word >> 25;

        // Sign-extended immediates, one per format.
        let imm_i = ((word as i32) >> 20) as i64 as u64;
        let imm_s =
            ((((word & 0xfe00_0000) as i32) >> 20) as i64 as u64) | u64::from((word >> 7) & 0x1f);
        let imm_u = ((word & 0xffff_f000) as i32) as i64 as u64;
        let imm_b = {
            let v = (((word >> 31) & 1) << 12)
                | (((word >> 7) & 1) << 11)
                | (((word >> 25) & 0x3f) << 5)
                | (((word >> 8) & 0xf) << 1);
            // 13-bit signed
            if v & 0x1000 != 0 {
                v as u64 | !0x1fff
            } else {
                u64::from(v)
            }
        };
        let imm_j = {
            let v = (((word >> 31) & 1) << 20)
                | (((word >> 12) & 0xff) << 12)
                | (((word >> 20) & 1) << 11)
                | (((word >> 21) & 0x3ff) << 1);
            if v & 0x0010_0000 != 0 {
                v as u64 | !0x001f_ffff
            } else {
                u64::from(v)
            }
        };

        // THE FS/VS GATE (`V-009` part (i-c), owner ruling 2026-10-05: "yantra must
        // enforce exact hardware semantics"). Under `sstatus.FS = Off` every F/D
        // instruction — OP-FP, the fused forms, and the float loads and stores, which is
        // where the moves and conversions live too — is an ILLEGAL INSTRUCTION; under
        // `sstatus.VS = Off` so is every V instruction: OP-V (`vsetvli` included), the
        // vector loads and stores, and the vector CSRs. A vector FLOAT instruction
        // (OPFVV, OPFVF) needs FS as well, as QEMU requires. Until this, yantra ran a
        // float image whose startup never set FS, and a real hart refused it: from
        // `V-005` to (i-b) no float or vector image ran on QEMU, and this machine never
        // said so. Integer forms never reach here through the decode cache's fast path
        // ([`Op::Slow`] covers every opcode below), so this is the one place.
        let fs_off = self.csr.sstatus & SSTATUS_FS == 0;
        let vs_off = self.csr.sstatus & SSTATUS_VS == 0;
        let gated = match opcode {
            0x07 | 0x27 if vector::is_vector_width(funct3) => vs_off,
            0x57 => vs_off || ((funct3 == 0x1 || funct3 == 0x5) && fs_off),
            0x07 | 0x27 | 0x43 | 0x47 | 0x4b | 0x4f | 0x53 => fs_off,
            // csrrw/csrrs/csrrc and their immediates on vstart, vxsat, vxrm, vcsr, vl,
            // vtype, vlenb.
            0x73 if matches!(funct3 & 0x3, 1..=3)
                && matches!(word >> 20, 0x008..=0x00a | 0x00f | 0xc20 | 0xc21 | 0xc22) =>
            {
                vs_off
            }
            // ... and on fflags, frm, fcsr, which need FS (`V-009` part (i-d)).
            0x73 if matches!(funct3 & 0x3, 1..=3) && matches!(word >> 20, 0x001..=0x003) => fs_off,
            _ => false,
        };
        if gated {
            return self.illegal(word, pc);
        }
        // A RESERVED ROUNDING MODE is illegal — a static `rm` of 5 or 6, or `DYN` while
        // `frm` holds 5, 6 or 7 — on every F/D instruction that has an `rm` field: the
        // fused four, and in OP-FP the arithmetic and every conversion (`V-009` (i-d),
        // QEMU probe D3..D15). Sign injection, min/max, compares, moves and `fclass` use
        // the same bits as a `funct3` and are untouched (D9, D10).
        let has_rm = match opcode {
            0x43 | 0x47 | 0x4b | 0x4f => true,
            0x53 => matches!(funct7 >> 2, 0x00..=0x03 | 0x08 | 0x0b | 0x18 | 0x1a),
            _ => false,
        };
        if has_rm && self.rm_reserved(funct3) {
            return self.illegal(word, pc);
        }

        let mut next = pc.wrapping_add(4);
        match opcode {
            0x37 => self.x[rd] = imm_u,                  // LUI
            0x17 => self.x[rd] = pc.wrapping_add(imm_u), // AUIPC
            0x6f => {
                // JAL
                self.x[rd] = pc.wrapping_add(4);
                next = pc.wrapping_add(imm_j);
                // `jal x0, .` is how both demo programs park. Reporting it beats
                // burning the whole budget to reach the same conclusion. The return
                // skips the `x[0] = 0` at the bottom, so it is done here: a hart parked
                // under a kernel (`process::Kernel::run`) keeps running, and must not
                // find `pc + 4` in the zero register.
                if next == pc {
                    self.x[0] = 0;
                    return Some(Halt::SpinForever { pc });
                }
            }
            0x13 => {
                // OP-IMM
                let a = self.x[rs1];
                self.x[rd] = match funct3 {
                    0x0 => a.wrapping_add(imm_i),                     // ADDI
                    0x1 => a << (imm_i & 0x3f),                       // SLLI
                    0x5 if funct7 & 0x20 == 0 => a >> (imm_i & 0x3f), // SRLI
                    0x5 => ((a as i64) >> (imm_i & 0x3f)) as u64,     // SRAI
                    0x2 => u64::from((a as i64) < (imm_i as i64)),    // SLTI
                    0x3 => u64::from(a < imm_i),                      // SLTIU
                    0x4 => a ^ imm_i,                                 // XORI
                    0x6 => a | imm_i,                                 // ORI
                    0x7 => a & imm_i,                                 // ANDI
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
            }
            0x33 => {
                // OP
                let (a, b) = (self.x[rs1], self.x[rs2]);
                self.x[rd] = match (funct3, funct7) {
                    (0x0, 0x00) => a.wrapping_add(b),                  // ADD
                    (0x0, 0x20) => a.wrapping_sub(b),                  // SUB
                    (0x1, 0x00) => a << (b & 0x3f),                    // SLL
                    (0x2, 0x00) => u64::from((a as i64) < (b as i64)), // SLT
                    (0x3, 0x00) => u64::from(a < b),                   // SLTU
                    (0x4, 0x00) => a ^ b,                              // XOR
                    (0x5, 0x00) => a >> (b & 0x3f),                    // SRL
                    (0x5, 0x20) => ((a as i64) >> (b & 0x3f)) as u64,  // SRA
                    (0x6, 0x00) => a | b,                              // OR
                    (0x7, 0x00) => a & b,                              // AND

                    // The M extension, funct7 = 0x01. Until 2026-09-05 there was no arm
                    // for it at all, so every one of these eight fell to the `_` below and
                    // halted `Unimplemented`. That was a real boundary and not an oversight
                    // — `ir.t1`'s margin named it when the operators were lowered here —
                    // but the frozen grammar promises `गुणनम्` and `शेषः`, so the machine
                    // owed the language these. Found by two demonstration programs that
                    // multiply and take a remainder halting on opcode 51 with everything
                    // upstream of them correct.
                    //
                    // The division cases are the ones worth writing down, because RISC-V
                    // does NOT trap on them and a host that uses Rust's `/` and `%` here
                    // panics where the machine must not. §7.2 fixes all six by value:
                    // divide by zero yields all ones (DIV, DIVU); remainder by zero yields
                    // the DIVIDEND unchanged (REM, REMU); and the one overflow case,
                    // i64::MIN / -1, yields i64::MIN for DIV and 0 for REM. Each is
                    // spelled as its own guard rather than folded, so a reader can check
                    // it against the table in the specification line by line.
                    (0x0, 0x01) => a.wrapping_mul(b), // MUL — low 64 bits
                    (0x1, 0x01) => (((a as i64 as i128) * (b as i64 as i128)) >> 64) as u64, // MULH
                    (0x2, 0x01) => (((a as i64 as i128) * (b as u128 as i128)) >> 64) as u64, // MULHSU
                    (0x3, 0x01) => (((a as u128) * (b as u128)) >> 64) as u64, // MULHU
                    (0x4, 0x01) => match (a as i64, b as i64) {
                        (_, 0) => u64::MAX,                 // DIV by zero — all ones
                        (i64::MIN, -1) => i64::MIN as u64,  // the single overflow case
                        (x, y) => x.wrapping_div(y) as u64, // DIV
                    },
                    (0x5, 0x01) => {
                        if b == 0 { u64::MAX } else { a / b } // DIVU, zero → all ones
                    }
                    (0x6, 0x01) => match (a as i64, b as i64) {
                        (x, 0) => x as u64,                 // REM by zero — the dividend
                        (i64::MIN, -1) => 0,                // the overflow case
                        (x, y) => x.wrapping_rem(y) as u64, // REM
                    },
                    (0x7, 0x01) => {
                        if b == 0 { a } else { a % b } // REMU, zero → the dividend
                    }

                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
            }
            // ── the RV64 word forms, `V-009` part (i-d) ─────────────────────
            // OP-IMM-32 and OP-32 compute on the low 32 bits and SIGN-EXTEND the 32-bit
            // result — every one of them, `srliw`/`srlw`/`divuw`/`remuw` included, which
            // reads wrong and is what the spec says (QEMU probe W8, W9, W30, W31). Until
            // (i-d) neither opcode had an arm, though both are RV64I base (and the
            // `w` M forms RV64M), and `li` of a wide constant expands to `addiw`.
            // Every other encoding is [`Machine::reserved_word_form`]'s: an ILLEGAL
            // INSTRUCTION where QEMU traps, a halt by name where QEMU runs an extension
            // this machine does not implement (`V-009` (i-e)).
            0x1b => {
                // OP-IMM-32
                let a = self.x[rs1] as u32;
                let shamt = (word >> 20) & 0x1f;
                let r: u32 = match (funct3, funct7) {
                    (0x0, _) => a.wrapping_add(imm_i as u32),    // ADDIW
                    (0x1, 0x00) => a << shamt,                   // SLLIW
                    (0x5, 0x00) => a >> shamt,                   // SRLIW
                    (0x5, 0x20) => ((a as i32) >> shamt) as u32, // SRAIW
                    _ => return self.reserved_word_form(word, pc),
                };
                self.x[rd] = i64::from(r as i32) as u64;
            }
            0x3b => {
                // OP-32
                let (a, b) = (self.x[rs1] as u32, self.x[rs2] as u32);
                let (sa, sb) = (a as i32, b as i32);
                let r: u32 = match (funct3, funct7) {
                    (0x0, 0x00) => a.wrapping_add(b),         // ADDW
                    (0x0, 0x20) => a.wrapping_sub(b),         // SUBW
                    (0x1, 0x00) => a << (b & 0x1f),           // SLLW
                    (0x5, 0x00) => a >> (b & 0x1f),           // SRLW
                    (0x5, 0x20) => (sa >> (b & 0x1f)) as u32, // SRAW
                    (0x0, 0x01) => a.wrapping_mul(b),         // MULW
                    // §7.2's table at 32 bits, with the same three fixed answers as
                    // the 64-bit forms above: by zero all ones / the dividend, and
                    // i32::MIN / -1 = i32::MIN with remainder 0.
                    (0x4, 0x01) => match (sa, sb) {
                        (_, 0) => u32::MAX, // DIVW by zero
                        (i32::MIN, -1) => i32::MIN as u32,
                        (x, y) => x.wrapping_div(y) as u32, // DIVW
                    },
                    (0x5, 0x01) => a.checked_div(b).unwrap_or(u32::MAX), // DIVUW
                    (0x6, 0x01) => match (sa, sb) {
                        (x, 0) => x as u32, // REMW by zero
                        (i32::MIN, -1) => 0,
                        (x, y) => x.wrapping_rem(y) as u32, // REMW
                    },
                    (0x7, 0x01) => a.checked_rem(b).unwrap_or(a), // REMUW
                    _ => return self.reserved_word_form(word, pc),
                };
                self.x[rd] = i64::from(r as i32) as u64;
            }
            // ── the F and D extensions, row `V-001` ─────────────────────────
            // Until this row every one of these fell to the `_` below and halted
            // `Unimplemented`, and `SSTATUS_MASK`'s margin used that as its safety
            // argument for dropping `FS`. Both facts changed together; see
            // [`crate::fp`] for what is honoured and what is refused, in
            // particular THE ROUNDING MODE, which is never silently ignored.
            // ── the V extension's subset, row `V-007` ──────────────────────
            // A vector load or store shares LOAD-FP/STORE-FP and is told apart by its
            // `width`; OP-V is its own opcode. See [`crate::vector`].
            0x07 | 0x27 if vector::is_vector_width(funct3) => {
                if let Err(h) = self.step_vector(word, pc, out) {
                    return h;
                }
            }
            0x57 => {
                if let Err(h) = self.step_vector(word, pc, out) {
                    return h;
                }
            }
            0x07 => {
                // LOAD-FP — flw (funct3 2), fld (funct3 3)
                let addr = self.x[rs1].wrapping_add(imm_i);
                let width = match funct3 {
                    0x2 => 4,
                    0x3 => 8,
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
                let pa = match self.translate_span(addr, width, Access::Load, pc) {
                    Ok(pa) => pa,
                    Err(h) => return Some(h),
                };
                match self.load(pa, width, false, pc) {
                    // A loaded single is BOXED. Without this a later `fadd.s` on the
                    // register would be indistinguishable from one holding a double.
                    Ok(v) => {
                        self.f[rd] = if width == 4 {
                            0xffff_ffff_0000_0000 | v
                        } else {
                            v
                        };
                        self.mark_fp_dirty();
                    }
                    Err(h) => return Some(h),
                }
            }
            0x27 => {
                // STORE-FP — fsw (2), fsd (3)
                let addr = self.x[rs1].wrapping_add(imm_s);
                let width = match funct3 {
                    0x2 => 4,
                    0x3 => 8,
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
                let pa = match self.translate_span(addr, width, Access::Store, pc) {
                    Ok(pa) => pa,
                    Err(h) => return Some(h),
                };
                // `fsw` stores the LOW 32 bits, box or no box.
                match self.store(pa, width, self.f[rs2], pc, out) {
                    Ok(Some(h)) => return Some(h),
                    Ok(None) => {}
                    Err(h) => return Some(h),
                }
            }
            0x43 | 0x47 | 0x4b | 0x4f => {
                // The four fused multiply-adds. `rs3` is bits 31..27; `fmt` is 26..25.
                let rs3 = ((word >> 27) & 0x1f) as usize;
                let fmt = (word >> 25) & 0x3;
                let rm = funct3;
                // Negate the product / negate the addend, per opcode.
                //
                // **THE TWO `n` FORMS ARE NAMED FOR THE OPPOSITE OF WHAT THEY DO, and
                // this table had them swapped until the test caught it.** Read the spec's
                // definitions, not the mnemonics:
                //
                //     fmadd   =    a*b + c
                //     fmsub   =    a*b - c
                //     fnmsub  = -(a*b) + c     <-- an ADD, despite "sub"
                //     fnmadd  = -(a*b) - c     <-- a SUB, despite "add"
                //
                // The `n` negates the PRODUCT, and the `add`/`sub` suffix then describes
                // the operation as it reads BEFORE that negation is distributed. So
                // `fnmsub` is "negate the multiply-sub", i.e. -(a*b - c) = -(a*b) + c.
                // With a=2 b=3 c=1 the four answers are 7, 5, -5, -7; getting the last
                // two the wrong way round is arithmetically silent — both are plausible
                // numbers of the right magnitude.
                let (neg_prod, neg_add) = match opcode {
                    0x43 => (false, false), // fmadd   =    a*b + c
                    0x47 => (false, true),  // fmsub   =    a*b - c
                    0x4b => (true, false),  // fnmsub  = -(a*b) + c
                    _ => (true, true),      // fnmadd  = -(a*b) - c
                };
                // One rounding, in software (`V-009` (i-e)): `NX`/`OF`/`UF` exact, and an
                // addend that signals now raises `NV` — `a.mul_add(b, c)` with the flags
                // read off `a` and `b` alone never saw `c`.
                let erm = if rm == fp::DYN { self.frm() } else { rm };
                let s_in = |r: usize| u64::from(fp::unbox_s(self.f[r]).to_bits());
                let (bits, flags) = match fmt {
                    0x0 => {
                        let (v, fl) = fp::fma(
                            fp::F32,
                            s_in(rs1),
                            s_in(rs2),
                            s_in(rs3),
                            neg_prod,
                            neg_add,
                            erm,
                        );
                        (0xffff_ffff_0000_0000 | v, fl)
                    }
                    0x1 => fp::fma(
                        fp::F64,
                        self.f[rs1],
                        self.f[rs2],
                        self.f[rs3],
                        neg_prod,
                        neg_add,
                        erm,
                    ),
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
                self.set_fflags(flags);
                self.f[rd] = bits;
                self.mark_fp_dirty();
            }
            0x53 => {
                // OP-FP. `funct7` is (op5 << 2) | fmt, so the arms below are written as
                // the funct7 VALUE rather than as a pair — the encoding table in the
                // specification is indexed that way and a reader checking one against the
                // other should not have to do the shift in their head.
                if let Some(h) = self.op_fp(word, opcode, pc, rd, rs1, rs2, funct3, funct7) {
                    return Some(h);
                }
            }
            0x03 => {
                // LOAD
                let addr = self.x[rs1].wrapping_add(imm_i);
                let (width, signed) = match funct3 {
                    0x0 => (1, true),  // LB
                    0x1 => (2, true),  // LH
                    0x2 => (4, true),  // LW
                    0x3 => (8, false), // LD
                    0x4 => (1, false), // LBU
                    0x5 => (2, false), // LHU
                    0x6 => (4, false), // LWU
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
                let pa = match self.translate_span(addr, width, Access::Load, pc) {
                    Ok(pa) => pa,
                    Err(h) => return Some(h),
                };
                // `W-374`: the counter answers HERE, on the integer load, and nowhere
                // else — see `COUNTER` for why every other access to it is refused.
                if let Some(read) = self.counter(pa, width, signed, pc) {
                    match read {
                        Ok(v) => {
                            out.counter_read();
                            self.x[rd] = v;
                        }
                        Err(h) => return Some(h),
                    }
                } else {
                    match self.load(pa, width, signed, pc) {
                        Ok(v) => self.x[rd] = v,
                        Err(h) => return Some(h),
                    }
                }
            }
            0x23 => {
                // STORE
                let addr = self.x[rs1].wrapping_add(imm_s);
                let width = match funct3 {
                    0x0 => 1, // SB
                    0x1 => 2, // SH
                    0x2 => 4, // SW
                    0x3 => 8, // SD
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
                let pa = match self.translate_span(addr, width, Access::Store, pc) {
                    Ok(pa) => pa,
                    Err(h) => return Some(h),
                };
                match self.store(pa, width, self.x[rs2], pc, out) {
                    Ok(Some(h)) => return Some(h),
                    Ok(None) => {}
                    Err(h) => return Some(h),
                }
            }
            0x63 => {
                // BRANCH
                let (a, b) = (self.x[rs1], self.x[rs2]);
                let take = match funct3 {
                    0x0 => a == b,                   // BEQ
                    0x1 => a != b,                   // BNE
                    0x4 => (a as i64) < (b as i64),  // BLT
                    0x5 => (a as i64) >= (b as i64), // BGE
                    0x6 => a < b,                    // BLTU
                    0x7 => a >= b,                   // BGEU
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
                if take {
                    next = pc.wrapping_add(imm_b);
                }
            }
            0x67 => {
                // JALR
                let t = self.x[rs1].wrapping_add(imm_i) & !1;
                self.x[rd] = pc.wrapping_add(4);
                next = t;
            }
            0x0f => {
                // MISC-MEM: FENCE and FENCE.I, both no-ops HERE — and that is a property
                // of this machine, not a shrug. One hart, one instruction at a time,
                // every store landing in `mem` before the next step can read it, and
                // every instruction fetched out of that same `mem` on every step. There
                // is no store buffer to drain and no instruction cache to invalidate, so
                // the ordering a fence asks for already holds and `fence.i` has nothing
                // stale to discard. On the hardware these programs were written for
                // neither is true, which is why they carry the instructions.
                //
                // The predecessor/successor sets are therefore not read. Executing a
                // fence as a no-op is correct; *decoding* it as one would not be, so the
                // funct3 values that are not FENCE or FENCE.I still stop.
                match funct3 {
                    0x0 | 0x1 => {}
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                }
            }
            0x2f => {
                // AMO — the A extension. Genuinely atomic here for the reason above: a
                // step is indivisible because nothing else runs, so the read-modify-write
                // needs no mechanism, only the right VALUE.
                let width = match funct3 {
                    0x2 => 4, // .w
                    0x3 => 8, // .d
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                };
                // `aq` and `rl` are the low two bits of funct7 and are ignored, by the
                // same argument the fences are: acquire and release order this hart
                // against others, and there are none.
                let funct5 = funct7 >> 2;
                let addr = self.x[rs1];
                if !addr.is_multiple_of(width as u64) {
                    return Some(Halt::BadAccess { pc, addr });
                }
                // An atomic is naturally aligned and no wider than eight bytes, so it
                // cannot cross a page, and ONE walk serves both halves of the
                // read-modify-write — which is also the truthful model: an AMO takes a
                // single translation on hardware, not one per access. `lr` faults as a
                // load and `sc` and the nine `amo*` as stores, which is where the
                // privileged spec puts each of them.
                let access = if funct5 == 0x02 {
                    Access::Load
                } else {
                    Access::Store
                };
                let pa = match self.translate(addr, access, pc) {
                    Ok(pa) => pa,
                    Err(h) => return Some(h),
                };

                if funct5 == 0x03 {
                    // SC. The store happens only against a live reservation for THIS
                    // address, and the reservation is spent either way. `rd` is 0 for
                    // success — the inverse of every other flag in this file, and the
                    // spec's choice, so the failure path can carry a reason.
                    let reserved = self.reservation == Some(addr);
                    self.reservation = None;
                    if reserved {
                        match self.store(pa, width, self.x[rs2], pc, out) {
                            Ok(Some(h)) => return Some(h),
                            Ok(None) => {}
                            Err(h) => return Some(h),
                        }
                    }
                    self.x[rd] = u64::from(!reserved);
                } else {
                    if funct5 == 0x02 && rs2 != 0 {
                        // `lr` has no second source; the bits are reserved. A word with
                        // them set is not an `lr` this machine can honestly execute.
                        return Some(Halt::Unimplemented { pc, word, opcode });
                    }
                    // Sign-extended for `.w`, which is what `rd` must receive.
                    let old = match self.load(pa, width, true, pc) {
                        Ok(v) => v,
                        Err(h) => return Some(h),
                    };
                    let src = self.x[rs2];
                    // The comparisons are at the OPERAND width, not at 64 bits: for `.w`
                    // an unsigned compare of two sign-extended halves gets the answer
                    // backwards as soon as one of them has the high bit set.
                    let (a, b, ua, ub) = if width == 4 {
                        (
                            old as i32 as i64,
                            src as i32 as i64,
                            u64::from(old as u32),
                            u64::from(src as u32),
                        )
                    } else {
                        (old as i64, src as i64, old, src)
                    };
                    let value = match funct5 {
                        0x02 => {
                            // LR: no store, and the reservation is the whole effect.
                            self.reservation = Some(addr);
                            self.x[rd] = old;
                            self.x[0] = 0;
                            self.pc = next;
                            return None;
                        }
                        0x00 => a.wrapping_add(b) as u64, // AMOADD
                        0x01 => src,                      // AMOSWAP
                        0x04 => old ^ src,                // AMOXOR
                        0x08 => old | src,                // AMOOR
                        0x0c => old & src,                // AMOAND
                        0x10 => {
                            if a < b {
                                old
                            } else {
                                src
                            }
                        } // AMOMIN
                        0x14 => {
                            if a > b {
                                old
                            } else {
                                src
                            }
                        } // AMOMAX
                        0x18 => {
                            if ua < ub {
                                old
                            } else {
                                src
                            }
                        } // AMOMINU
                        0x1c => {
                            if ua > ub {
                                old
                            } else {
                                src
                            }
                        } // AMOMAXU
                        _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                    };
                    match self.store(pa, width, value, pc, out) {
                        Ok(Some(h)) => return Some(h),
                        Ok(None) => {}
                        Err(h) => return Some(h),
                    }
                    self.x[rd] = old;
                }
            }
            0x73 => {
                // SYSTEM. `funct3 == 0` is the no-operand half of the family. `ecall`,
                // `ebreak` and `sret` are masked `0xffffffff` in the encoding table, so
                // every other bit of those must be zero and they are matched on the
                // whole word. `sfence.vma` is not: its mask is `0xfe007fff`, because it
                // carries two register operands, so it is matched on the bits the table
                // leaves fixed. `wfi` and `mret` still fall through to Unimplemented,
                // which is where they belong. `mret` returns from a mode above this one
                // and there is none. `wfi` is not in `spec/encodings-riscv64.tsv` at all
                // — no program this tree assembles can emit one — and `spec/timer.sas`
                // waits in a `jal x0, .` instead, which an interrupt now escapes. See the
                // crate docs: decoding it would be this crate's first opcode with no row
                // behind it.
                if funct3 == 0 {
                    if word & 0xfe00_7fff == 0x1200_0073 {
                        if self.mode == Privilege::User {
                            // Privileged. A no-op is still an instruction U-mode may not
                            // execute, and letting it through would say the machine has no
                            // opinion about who may shoot down a translation.
                            return self.illegal(word, pc);
                        }
                        // SFENCE.VMA — a no-op HERE, by the same argument `fence` is
                        // (see MISC-MEM above) and not by the same argument as `wfi`.
                        // It orders a page-table write against the translations a hart
                        // has cached, and [`Machine::translate`] caches none: every
                        // fetch, load and store walks the tables in `mem` as they stand
                        // at that instant, so a store to a PTE is visible to the very
                        // next access with nothing asked for. There is no TLB to flush
                        // because there is no TLB.
                        //
                        // `rs1` and `rs2` narrow the flush to one address or one ASID,
                        // and are not read for the same reason the fence's predecessor
                        // and successor sets are not: a flush of everything satisfies
                        // every narrower request. `spec/paging.sas` issues one
                        // immediately after writing `satp` and it must not stop there.
                    } else {
                        match word {
                            0x0000_0073 => {
                                // ECALL from U-mode is cause 8, and it goes to the
                                // program's own `stvec` — a user program has a kernel
                                // above it, and the kernel is what an environment call
                                // from user mode is addressed to. This is the number
                                // `spec/user-mode.sas` prints and the one thing in it that
                                // S-mode cannot forge: an S-mode `ecall` raises 9.
                                // `stval` is 0 for an environment call, per the spec.
                                if self.mode == Privilege::User {
                                    match self.deliver(8, 0, pc) {
                                        Ok(handler) => next = handler,
                                        Err(h) => return Some(h),
                                    }
                                } else {
                                    // From S-mode it is an SBI call: this machine is the
                                    // firmware, and an environment call from supervisor mode
                                    // is the one exception that always goes up. See the crate
                                    // docs.
                                    let eid = self.x[17]; // a7
                                    match eid {
                                        // sbi_set_timer. `a0` is an ABSOLUTE deadline on
                                        // the `time` counter, not a delay — which is why
                                        // `spec/timer.sas` can ask for 0 and mean "now"
                                        // and for -1 and mean "never", without reading
                                        // the clock first. Arming it replaces the old
                                        // deadline and nothing else: the pending bit is
                                        // recomputed from it, so this is also how a
                                        // handler stops a timer that would otherwise
                                        // re-fire the instant it returns.
                                        0x00 => {
                                            self.timecmp = Some(self.x[10]);
                                            self.x[10] = 0;
                                        }
                                        // sbi_console_putchar. `a0` is the byte, and it goes
                                        // through the same Output the UART store does: one
                                        // bridge, whether the program talks to the device or
                                        // asks the firmware to.
                                        0x01 => {
                                            out.putc(self.x[10] as u8);
                                            // The legacy call returns in `a0`, and 0 is success.
                                            // SBI promises only `a0` and `a1` are clobbered, so
                                            // nothing else is touched.
                                            self.x[10] = 0;
                                        }
                                        0x08 => return Some(Halt::Shutdown { pc }),
                                        _ => {
                                            return Some(Halt::Sbi {
                                                pc,
                                                eid,
                                                fid: self.x[16], // a6
                                            });
                                        }
                                    }
                                }
                            }
                            0x0010_0073 => {
                                // EBREAK. A breakpoint is a *delegatable* exception and
                                // OpenSBI delegates it, so on the hardware these programs
                                // target the handler that runs is the program's own — which
                                // is exactly what `spec/trap.sas` proves by printing
                                // `scause = 3`. With no handler installed there is nowhere
                                // to deliver it: on metal it would trap to address 0 and
                                // die, and saying so beats executing the rubble there.
                                if self.csr.stvec == 0 {
                                    return Some(Halt::Breakpoint { pc });
                                }
                                next = self.trap(3, pc, pc);
                            }
                            0x1020_0073 => {
                                // SRET. `SPP` says which mode the trap came from, and that
                                // is the mode this returns to — U-mode included, since
                                // `F-001c2b2` built the `U` bit and `SUM` that make U-mode
                                // something other than S-mode wearing another name.
                                if self.mode == Privilege::User {
                                    // Privileged, so a user program executing it is trapped
                                    // rather than obeyed — otherwise U-mode could leave
                                    // itself by executing two instructions.
                                    return self.illegal(word, pc);
                                }
                                self.mode = if self.csr.sstatus & SSTATUS_SPP == 0 {
                                    Privilege::User
                                } else {
                                    Privilege::Supervisor
                                };
                                let spie = self.csr.sstatus & SSTATUS_SPIE != 0;
                                self.csr.sstatus &= !(SSTATUS_SIE | SSTATUS_SPP);
                                if spie {
                                    self.csr.sstatus |= SSTATUS_SIE;
                                }
                                // SPIE is left set, which is the spec's rule and not an
                                // omission: it has no earlier value to be restored to.
                                self.csr.sstatus |= SSTATUS_SPIE;
                                next = self.csr.sepc;
                            }
                            _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                        }
                    }
                } else if funct3 == 0x4 {
                    // The one funct3 SYSTEM does not define. Not a CSR instruction, so
                    // it must not be reported as one.
                    return Some(Halt::Unimplemented { pc, word, opcode });
                } else {
                    // CSR access. `funct3`'s high bit selects the immediate forms, where
                    // the source operand is those same five bits read as a number rather
                    // than as a register — which is why one field serves both and
                    // `rs1 == 0` is "no bits" either way.
                    let source = if funct3 & 0x4 == 0 {
                        self.x[rs1]
                    } else {
                        rs1 as u64
                    };
                    // TWO RULES, and QEMU keeps them apart (`V-009` (i-d) review, probe4
                    // R1..R15, probe5 T1..T10):
                    //
                    // `write` — the instruction ATTEMPTS a write: csrrw/csrrwi always, and
                    // csrrs/csrrc/their immediates when the `rs1` FIELD is not 0. This is
                    // what makes a read-only CSR illegal (`csrrs a0, time, t0` with t0 = 0
                    // traps; `csrrs a0, time, x0` reads).
                    //
                    // `stores` — the write HAPPENS, with its side effects (FS or VS made
                    // Dirty): csrrw/csrrwi always, csrrs/csrrc only when the MASK VALUE is
                    // non-zero. `csrrs a0, fflags, t0` with t0 = 0 leaves FS Initial on
                    // QEMU; with t0 = 1 it dirties FS even if the bit was already set.
                    let write = funct3 & 0x3 == 0x1 || rs1 != 0;
                    let stores = funct3 & 0x3 == 0x1 || source != 0;
                    let csr = (word >> 20) as u16;
                    // Every CSR this machine implements is a *supervisor* register, so
                    // from U-mode every one of them is an illegal instruction — including
                    // a pure read. A user program that could read `satp` would be told
                    // where the kernel's tables are; one that could write it would leave.
                    //
                    // EXCEPT THE UNPRIVILEGED ONES (`V-009` (i-d)): the float and vector
                    // CSRs belong to the program, not the kernel, and QEMU serves them to
                    // U-mode — see [`Machine::user_csr`].
                    if self.mode == Privilege::User && !Self::user_csr(csr) {
                        return self.illegal(word, pc);
                    }
                    let Some(old) = self.csr_read(csr) else {
                        return Some(Halt::Csr { pc, csr, write });
                    };
                    // Bits 11:10 of a CSR number are its accessibility, and `11` means
                    // read-only. A write to one is an ILLEGAL INSTRUCTION — the whole
                    // instruction, so `rd` is not written either — and the check is on
                    // the number rather than on a list, because that is where the ISA
                    // puts it: `time` (0xc01) is read-only for the same reason every
                    // other 0xc__ counter is.
                    if write && csr >> 10 == 0b11 {
                        return self.illegal(word, pc);
                    }
                    if stores {
                        let value = match funct3 & 0x3 {
                            0x1 => source,       // csrrw, csrrwi
                            0x2 => old | source, // csrrs, csrrsi
                            _ => old & !source,  // csrrc, csrrci
                        };
                        self.csr_write(csr, value);
                    }
                    // `csrrw rd=x0` is defined not to read at all. There is no CSR here
                    // whose read has a side effect, so the difference is unobservable —
                    // and `x0` is cleared below in any case.
                    self.x[rd] = old;
                }
            }
            _ => return Some(Halt::Unimplemented { pc, word, opcode }),
        }

        self.x[0] = 0; // written once, rather than guarded at every assignment above
        self.pc = next;
        None
    }
}
