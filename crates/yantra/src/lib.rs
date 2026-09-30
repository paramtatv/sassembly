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
//! interrupts, and no floating point or compressed
//! encodings. It gained the **M extension** on 2026-09-05 — the eight multiply, divide
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
pub mod host;
pub mod input;
pub mod loader;
pub mod patra;
pub mod process;
pub mod profile;
pub mod supervisor;
pub mod virtio_gpu;

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
    let extent = loader::Program::parse(image)
        .ok()
        .and_then(|p| {
            let base = p.segments.iter().map(|s| s.vaddr).min()?;
            let end = p.segments.iter().map(|s| s.vaddr + s.memsz as u64).max()?;
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
    Finisher {
        /// The raw value written.
        value: u32,
        /// Decoded exit status, or `None` if the value is not a form the finisher defines.
        status: Option<u32>,
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
/// `XS` and `VS` stay dropped and the original argument still holds for them: there is no
/// user extension and no vector unit, so `vsetvli` halts `Unimplemented`. When row `V-007`
/// implements V, **this margin is owed the same refounding for `VS`** — the pattern is
/// now twice-established, so it is written down rather than rediscovered.
///
/// `SUM` and `MXR` were dropped by the same rule until `F-001c2b2`, and they are here now
/// for the reason it lifted the drop: there is a walker, and there is a mode below the
/// one that runs it, so both bits now change what a walk permits. See
/// [`Machine::translate`].
const SSTATUS_MASK: u64 =
    SSTATUS_SIE | SSTATUS_SPIE | SSTATUS_SPP | SSTATUS_SUM | SSTATUS_MXR | SSTATUS_FS;

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
    /// The PATH run a [`patra`] request named, waiting for its third store.
    pub patra_path: Option<u64>,
    /// The BUFFER run likewise.
    pub patra_buffer: Option<u64>,
    /// The directory a [`patra`] request may read under, or `None`.
    ///
    /// **`None` REFUSES EVERY FILE REQUEST, and that is the default.** `yantra`
    /// is a developer runtime and not an isolation boundary (ADR-0039), but a
    /// program that can name any path can read `~/.ssh`, and no existing caller
    /// asked for that capability. Opting in is one field; opting out must not
    /// be something a caller forgets.
    pub patra_root: Option<std::path::PathBuf>,
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
            patra_path: None,
            patra_buffer: None,
            x: [0; 32],
            f: [0; 32],
            fcsr: 0,
            pc: program.entry,
            base,
            mem: vec![0; ram],
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
        };
        for s in &program.segments {
            let at = (s.vaddr - base) as usize;
            if at + s.memsz > m.mem.len() {
                return Err(format!(
                    "segment at {:#x} needs {} bytes and RAM is {} — raise it",
                    s.vaddr,
                    s.memsz,
                    m.mem.len()
                ));
            }
            m.mem[at..at + s.filesz].copy_from_slice(&image[s.offset..s.offset + s.filesz]);
        }
        Ok(m)
    }

    fn load(&self, addr: u64, width: usize, signed: bool, pc: u64) -> Result<u64, Halt> {
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

    fn store(
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
            patra::put(&mut self.mem, self.base, p, b, value, root.as_deref());
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
            patra::serve(&mut self.mem, self.base, p, b, value, root.as_deref());
            self.patra_path = None;
            self.patra_buffer = None;
            return Ok(None);
        }
        if addr == FINISHER {
            let value = value as u32;
            let status = match value & 0xffff {
                0x5555 => Some(0),
                0x3333 => Some(value >> 16),
                _ => None,
            };
            return Ok(Some(Halt::Finisher { value, status }));
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
    fn translate(&self, va: u64, access: Access, pc: u64) -> Result<u64, Halt> {
        if self.csr.satp & SATP_SV39 != SATP_SV39 {
            return Ok(va);
        }
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
    fn translate_span(&self, va: u64, width: usize, access: Access, pc: u64) -> Result<u64, Halt> {
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
            0x100 => self.csr.sstatus,
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
            _ => return None,
        })
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
    /// Two things reach it. A **privileged instruction executed in user mode** is the
    /// original, and the larger set. A **write to a read-only CSR** is the other, and it
    /// is illegal from S-mode too: `time` is a counter, and a kernel that could set it
    /// could run its own deadlines backwards.
    fn illegal(&mut self, word: u32, pc: u64) -> Option<Halt> {
        match self.deliver(2, u64::from(word), pc) {
            Ok(handler) => {
                self.pc = handler;
                None
            }
            Err(h) => Some(h),
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
    fn interrupt(&self) -> Option<u64> {
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
        for _ in 0..budget {
            if let Some(h) = self.step(out) {
                return h;
            }
        }
        Halt::StepLimit { pc: self.pc }
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

        // The arithmetic arms share one refusal: a rounding mode this machine cannot
        // honour must STOP rather than round to nearest and look right. Checked once,
        // here, for every arm that rounds.
        let needs_rne = matches!(
            funct7,
            0x00 | 0x01 | 0x04 | 0x05 | 0x08 | 0x09 | 0x0c | 0x0d | 0x2c | 0x2d | 0x20 | 0x21
        );
        if needs_rne && !fp::arith_rm_supported(rm, self.frm()) {
            return unimpl();
        }

        macro_rules! bin_s {
            ($op:expr) => {{
                let (a, b) = (fp::unbox_s(self.f[rs1]), fp::unbox_s(self.f[rs2]));
                let out: f32 = $op(a, b);
                self.set_fflags(fp::arith_nv_s(a, b, out));
                self.f[rd] = fp::box_s(fp::canonicalise_s(out));
                self.mark_fp_dirty();
            }};
        }
        macro_rules! bin_d {
            ($op:expr) => {{
                let (a, b) = (f64::from_bits(self.f[rs1]), f64::from_bits(self.f[rs2]));
                let out: f64 = $op(a, b);
                self.set_fflags(fp::arith_nv(a, b, out));
                self.f[rd] = fp::canonicalise_d(out).to_bits();
                self.mark_fp_dirty();
            }};
        }

        match funct7 {
            0x00 => bin_s!(|a: f32, b: f32| a + b),
            0x01 => bin_d!(|a: f64, b: f64| a + b),
            0x04 => bin_s!(|a: f32, b: f32| a - b),
            0x05 => bin_d!(|a: f64, b: f64| a - b),
            0x08 => bin_s!(|a: f32, b: f32| a * b),
            0x09 => bin_d!(|a: f64, b: f64| a * b),
            // DIVISION SETS `DZ` AND THE RESULT IS STILL AN INFINITY, not a trap.
            0x0c => {
                let (a, b) = (fp::unbox_s(self.f[rs1]), fp::unbox_s(self.f[rs2]));
                if b == 0.0 && !a.is_nan() && a != 0.0 {
                    self.set_fflags(fp::DZ);
                }
                let out = a / b;
                self.set_fflags(fp::arith_nv_s(a, b, out));
                self.f[rd] = fp::box_s(fp::canonicalise_s(out));
                self.mark_fp_dirty();
            }
            0x0d => {
                let (a, b) = (f64::from_bits(self.f[rs1]), f64::from_bits(self.f[rs2]));
                if b == 0.0 && !a.is_nan() && a != 0.0 {
                    self.set_fflags(fp::DZ);
                }
                let out = a / b;
                self.set_fflags(fp::arith_nv(a, b, out));
                self.f[rd] = fp::canonicalise_d(out).to_bits();
                self.mark_fp_dirty();
            }
            // fsqrt — one operand; `rs2` must be 0 and a non-zero one is not this
            // instruction.
            0x2c | 0x2d => {
                if rs2 != 0 {
                    return unimpl();
                }
                if funct7 == 0x2c {
                    let a = fp::unbox_s(self.f[rs1]);
                    let out = a.sqrt();
                    self.set_fflags(fp::arith_nv_s(a, a, out));
                    self.f[rd] = fp::box_s(fp::canonicalise_s(out));
                } else {
                    let a = f64::from_bits(self.f[rs1]);
                    let out = a.sqrt();
                    self.set_fflags(fp::arith_nv(a, a, out));
                    self.f[rd] = fp::canonicalise_d(out).to_bits();
                }
                self.mark_fp_dirty();
            }
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
            // fcvt.s.d (0x20, rs2 = 1) and fcvt.d.s (0x21, rs2 = 0)
            0x20 => {
                if rs2 != 1 {
                    return unimpl();
                }
                let a = f64::from_bits(self.f[rs1]);
                let out = a as f32;
                self.set_fflags(fp::arith_nv(a, a, f64::from(out)));
                self.f[rd] = fp::box_s(fp::canonicalise_s(out));
                self.mark_fp_dirty();
            }
            0x21 => {
                if rs2 != 0 {
                    return unimpl();
                }
                let a = fp::unbox_s(self.f[rs1]);
                let out = f64::from(a);
                self.set_fflags(fp::arith_nv_s(a, a, out as f32));
                self.f[rd] = fp::canonicalise_d(out).to_bits();
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
                let as_f64: f64 = match rs2 {
                    0 => f64::from(src as i32),
                    1 => f64::from(src as u32),
                    2 => src as i64 as f64,
                    3 => src as f64,
                    _ => return unimpl(),
                };
                // AN INEXACT INT-TO-FLOAT UNDER A MODE THIS MACHINE CANNOT HONOUR MUST
                // STOP. i32 -> f64 is always exact, so `rm` cannot matter there and
                // refusing it would refuse the common case for no reason; a wide integer
                // that does not fit the mantissa is the case that actually rounds.
                let exact = match rs2 {
                    0 | 1 => true,
                    2 => (as_f64 as i64) == (src as i64),
                    _ => (as_f64 as u64) == src,
                };
                if !exact && !fp::arith_rm_supported(rm, self.frm()) {
                    return unimpl();
                }
                if !exact {
                    self.set_fflags(fp::NX);
                }
                if funct7 == 0x68 {
                    self.f[rd] = fp::box_s(as_f64 as f32);
                } else {
                    self.f[rd] = as_f64.to_bits();
                }
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
        // The clock advances first, and it advances even on the step that takes an
        // interrupt rather than an instruction: a hart whose time stopped while it was
        // being interrupted could not reach a deadline from inside a handler.
        self.time = self.time.wrapping_add(1);
        // An interrupt is taken BEFORE the instruction at `pc`, not after it — that
        // instruction has not run, and `sepc` naming it is what makes `sret` a resume
        // rather than a skip. It is also why `spec/timer.sas` can escape a `jal x0, .`:
        // the jump never executes, so [`Halt::SpinForever`] is never reached.
        if let Some(cause) = self.interrupt() {
            let pc = self.pc;
            // `stval` is zero for an interrupt: there is no faulting address, and a
            // handler that read one would be reading the last exception's.
            return match self.deliver(INTERRUPT | cause, 0, pc) {
                Ok(handler) => {
                    self.pc = handler;
                    None
                }
                Err(h) => Some(h),
            };
        }
        match self.step_inner(out) {
            Some(Halt::PageFault { pc, addr, cause }) if self.csr.stvec != 0 => {
                self.pc = self.trap(cause, addr, pc);
                None
            }
            other => other,
        }
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
        let word = match self.load(fetch, 4, false, pc) {
            Ok(w) => w as u32,
            Err(h) => return Some(h),
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
            // ── the F and D extensions, row `V-001` ─────────────────────────
            // Until this row every one of these fell to the `_` below and halted
            // `Unimplemented`, and `SSTATUS_MASK`'s margin used that as its safety
            // argument for dropping `FS`. Both facts changed together; see
            // [`crate::fp`] for what is honoured and what is refused, in
            // particular THE ROUNDING MODE, which is never silently ignored.
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
                if !fp::arith_rm_supported(rm, self.frm()) {
                    return Some(Halt::Unimplemented { pc, word, opcode });
                }
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
                match fmt {
                    0x0 => {
                        let (a, b, c) = (
                            fp::unbox_s(self.f[rs1]),
                            fp::unbox_s(self.f[rs2]),
                            fp::unbox_s(self.f[rs3]),
                        );
                        let a = if neg_prod { -a } else { a };
                        let c = if neg_add { -c } else { c };
                        let out = a.mul_add(b, c);
                        self.set_fflags(fp::arith_nv_s(a, b, out));
                        self.f[rd] = fp::box_s(fp::canonicalise_s(out));
                    }
                    0x1 => {
                        let (a, b, c) = (
                            f64::from_bits(self.f[rs1]),
                            f64::from_bits(self.f[rs2]),
                            f64::from_bits(self.f[rs3]),
                        );
                        let a = if neg_prod { -a } else { a };
                        let c = if neg_add { -c } else { c };
                        let out = a.mul_add(b, c);
                        self.set_fflags(fp::arith_nv(a, b, out));
                        self.f[rd] = fp::canonicalise_d(out).to_bits();
                    }
                    _ => return Some(Halt::Unimplemented { pc, word, opcode }),
                }
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
                match self.load(pa, width, signed, pc) {
                    Ok(v) => self.x[rd] = v,
                    Err(h) => return Some(h),
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
                    // csrrw/csrrwi always write. csrrs/csrrc set or clear the bits their
                    // operand names, so a zero operand names none and the instruction is
                    // a pure read.
                    let write = funct3 & 0x3 == 0x1 || rs1 != 0;
                    let csr = (word >> 20) as u16;
                    // Every CSR this machine implements is a *supervisor* register, so
                    // from U-mode every one of them is an illegal instruction — including
                    // a pure read. A user program that could read `satp` would be told
                    // where the kernel's tables are; one that could write it would leave.
                    if self.mode == Privilege::User {
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
                    if write {
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
