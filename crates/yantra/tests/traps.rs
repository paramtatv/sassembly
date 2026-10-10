//! `F-001c2a` — the supervisor trap registers, `ebreak` delivered to `stvec`, and `sret`.
//!
//! # The measurement came first, and it moved the row
//!
//! `F-001c2` was written expecting `mepc`, `mcause` and `mstatus` — machine mode. It also
//! said, in its own text, *"START FROM THE MEASURED LIST rather than the ISA manual"*. The
//! list is below and it is asserted, not quoted: [`csrs_the_programs_actually_name`] reads
//! the three CSR mnemonics out of `spec/mnemonics-riscv64.src.tsv` and every CSR operand
//! out of the 37 `spec/*.sas` programs, and finds **ten registers, all of them supervisor,
//! and not one machine-mode register anywhere in the tree**.
//!
//! That is why this row implements `sstatus`/`stvec`/`sscratch`/`sepc`/`scause`/`stval`
//! and `sret`, and nothing of M-mode. Had it started from the manual it would have built
//! `mtvec` and a trap-delegation model for exceptions that no program in this repository
//! raises.
//!
//! # Three columns, as `F-001b` and `F-001c1` had
//!
//! 1. **The registry and the programs** — the measured list, above. Neither file was
//!    written for this test.
//! 2. **The meaning**, written from the privileged spec by hand: what trap entry puts in
//!    each register, what `sret` puts back, and which of the ten are still refused.
//! 3. **A whole program** — `spec/trap.sas`, at both link addresses, asserted against the
//!    *same two lines* `tools/check-trap.sh` demands of QEMU under real OpenSBI. The first
//!    two columns say the registers are right; only the third says a trap is.
//!
//! # What must still not execute
//!
//! `satp`, `sie`, `sip` and `time` were named by the programs and **refused** when this
//! file was written: a machine that grew a sparse CSR map to get `trap.sas` running would
//! have passed column 3 and failed `the_four_refused_csrs_still_stop`, which was the
//! direction that mattered. `F-001c2b1` and `F-001c2b3` built the walker and the clock
//! behind all four, so that test is now
//! [`every_csr_the_programs_name_is_implemented`] — the same guard, pointing the other
//! way.

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::{Csrs, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;

const SSTATUS: u32 = 0x100;
const SIE: u32 = 0x104;
const STVEC: u32 = 0x105;
const SSCRATCH: u32 = 0x140;
const SEPC: u32 = 0x141;
const SCAUSE: u32 = 0x142;
const STVAL: u32 = 0x143;
const SIP: u32 = 0x144;
const SATP: u32 = 0x180;
const TIME: u32 = 0xc01;

const SSTATUS_SIE: u64 = 1 << 1;
const SSTATUS_SPIE: u64 = 1 << 5;
const SSTATUS_SPP: u64 = 1 << 8;

const EBREAK: u32 = 0x0010_0073;
const SRET: u32 = 0x1020_0073;

/// `csrrw rd, csr, rs1` and its five siblings, assembled by hand from the encoding this
/// file's sibling `system.rs` takes from `spec/encodings-riscv64.tsv`.
fn csr_insn(funct3: u32, rd: u32, csr: u32, source: u32) -> u32 {
    0x73 | rd << 7 | funct3 << 12 | source << 15 | csr << 20
}

/// A machine holding `words` at the entry point and nothing else.
fn machine(words: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_mem: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: BASE,
        base: BASE,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    for (i, w) in words.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// ---------------------------------------------------------------------------------------
// Column 1: what the programs name.

/// Read the Sassembly names of `csrrw`, `csrrs` and `csrrc` out of the mnemonic registry.
///
/// The registry, not this file: the whole point of doc 02 §2.3 is that the name is decided
/// in one place, and a test that transcribed three Devanagari words would be asserting
/// against its own copy of them.
fn csr_mnemonics() -> Vec<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/mnemonics-riscv64.src.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut names = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() > 1 && matches!(f[0], "csrrw" | "csrrs" | "csrrc") {
            names.push(f[1].to_string());
        }
    }
    names.sort();
    assert_eq!(
        names.len(),
        3,
        "the registry names all three CSR instructions"
    );
    names
}

/// Every CSR number named by an actual instruction in `spec/*.sas`, with how many times.
///
/// A CSR operand is written in decimal because the language has one numeral form, and it
/// carries the third-operand marker `त्`. Only lines whose *first* token is a CSR mnemonic
/// are read, so the `४त्` of an ordinary `addi` is not mistaken for register 4.
fn csrs_named_by_the_programs() -> Vec<(u32, usize)> {
    let names = csr_mnemonics();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec");
    let mut counts: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
    let mut programs = 0;
    for entry in std::fs::read_dir(&dir).expect("spec/ is tracked and always present") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sas") {
            continue;
        }
        programs += 1;
        let text = std::fs::read_to_string(&path).expect("a Sassembly source");
        for line in text.lines() {
            let mut tokens = line.split_whitespace();
            let Some(first) = tokens.next() else {
                continue;
            };
            if !names.iter().any(|n| n == first) {
                continue;
            }
            for token in tokens {
                // The marker is `त` plus a virama, and the virama is a separate code
                // point — stripping the visible letter without it finds nothing.
                let Some(digits) = token
                    .strip_suffix('\u{094d}')
                    .and_then(|t| t.strip_suffix('त'))
                else {
                    continue;
                };
                let mut value: u32 = 0;
                let mut any = false;
                for c in digits.chars() {
                    let Some(d) = (c as u32).checked_sub(0x966).filter(|d| *d < 10) else {
                        any = false;
                        break;
                    };
                    value = value * 10 + d;
                    any = true;
                }
                if any {
                    *counts.entry(value).or_default() += 1;
                }
            }
        }
    }
    // 37 since `F-001e3`. The new one is `spec/atithi.sas`, the tree's only application,
    // and it contributes nothing to the counts below — a program that named a CSR would
    // be the supervisor, which is ADR-0015's A2 and `spec/programs.tsv`'s `privileged`
    // column. So the list is unchanged and the number is not: the guard is that a
    // program was ADDED and read, not that the reading came out the same.
    // 37 -> 40 on 2026-08-27: `virtio-blk`, `virtio-console` and `virtio-net`
    // came back to `spec/`. They name no CSR — they are drivers, not the
    // supervisor — so the counts below are unchanged and only this total moves,
    // which is exactly the guard this line describes: a program was ADDED and
    // read, not that the reading came out the same.
    assert_eq!(programs, 48, "every Sassembly program in the tree was read");
    counts.into_iter().collect()
}

#[test]
fn csrs_the_programs_actually_name() {
    let named = csrs_named_by_the_programs();
    let numbers: Vec<u32> = named.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        numbers,
        [
            SSTATUS, SIE, STVEC, SSCRATCH, SEPC, SCAUSE, STVAL, SIP, SATP, TIME
        ],
        "ten registers, and the shape of the list is the finding: sstatus/sie/stvec/\
         sscratch/sepc/scause/stval/sip/satp and time are ALL supervisor (or user) \
         registers. F-001c2 was written expecting mepc, mcause and mstatus; there is not \
         one machine-mode CSR in any program in this repository. If this fails, a program \
         grew a CSR and the arms in lib.rs have not been told."
    );
    // `satp` is the most-used of the ten, which is why it was the refusal that cost the
    // most while it lasted — and why `F-001c2b1`, the row after this one, is the walker
    // that lifted it rather than another register.
    let satp = named.iter().find(|(n, _)| *n == SATP).expect("satp").1;
    assert!(
        named.iter().all(|(_, c)| *c <= satp),
        "satp is the most-named CSR in spec/: {named:?}"
    );
}

// ---------------------------------------------------------------------------------------
// Column 2: the meaning.

#[test]
fn every_csr_the_programs_name_is_implemented() {
    // This test was `the_three_refused_csrs_still_stop`, and before that
    // `the_four_refused_csrs_still_stop`. It asserted that `satp`, `sie`, `sip` and `time`
    // halted by name, because nothing stood behind them.
    //
    // Something stands behind all four now — the Sv39 walker (`F-001c2b1`, `tests/
    // paging.rs`) and the clock and its interrupt (`F-001c2b3`, `tests/interrupts.rs`) —
    // so the list it guarded is empty and the assertion is inverted rather than deleted:
    // **not one of the ten measured CSRs may refuse.** A row that regressed one to a halt
    // would fail here, which is what the refusal list was protecting in the first place.
    for csr in [
        SSTATUS, SIE, STVEC, SSCRATCH, SEPC, SCAUSE, STVAL, SIP, SATP, TIME,
    ] {
        // `csrrs x5, csr, x0` — a pure read, which is the access every one of the ten
        // must answer. `time` is read-only and a write to it is an illegal instruction,
        // so a write is not the universal question here.
        let halt = machine(&[csr_insn(0x2, 5, csr, 0)]).step(&mut Vec::new());
        assert_eq!(halt, None, "csr {csr:#x} must not refuse a read");
    }
}

#[test]
fn a_write_then_a_read_gives_back_what_the_machine_holds() {
    // csrrw x0, sscratch, x6 ; csrrs x5, sscratch, x0
    let write = csr_insn(0x1, 0, SSCRATCH, 6);
    let read = csr_insn(0x2, 5, SSCRATCH, 0);
    let mut m = machine(&[write, read]);
    m.x[6] = 0xdead_beef_0bad_f00d;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.csr.sscratch, 0xdead_beef_0bad_f00d);
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.x[5], 0xdead_beef_0bad_f00d,
        "sscratch is storage and nothing else, which is all the spec asks of it"
    );
}

#[test]
fn set_and_clear_read_the_old_value_first() {
    // The defining property of csrrs/csrrc: rd gets the value from BEFORE the update.
    // csrrs x5, sscratch, x6 ; csrrc x7, sscratch, x8
    let mut m = machine(&[csr_insn(0x2, 5, SSCRATCH, 6), csr_insn(0x3, 7, SSCRATCH, 8)]);
    m.csr.sscratch = 0b1010;
    m.x[6] = 0b0101;
    m.x[8] = 0b1000;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.x[5], 0b1010, "csrrs returned the value before the set");
    assert_eq!(m.csr.sscratch, 0b1111, "and set the bits its operand named");
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.x[7], 0b1111, "csrrc returned the value before the clear");
    assert_eq!(
        m.csr.sscratch, 0b0111,
        "and cleared the bit its operand named"
    );
}

#[test]
fn the_immediate_forms_read_the_same_five_bits_as_a_number() {
    // csrrsi x5, sscratch, 6 — the operand is the uimm, NOT x6. The distinction is the
    // whole difference between the two halves of funct3, and getting it backwards is
    // silent: x6 is usually zero, so the wrong answer looks like a working no-op.
    let mut m = machine(&[csr_insn(0x6, 5, SSCRATCH, 6)]);
    m.x[6] = 0xffff_ffff; // if this were read as a register the answer would be all ones
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.csr.sscratch, 6, "the uimm is 6, and 6 is what got set");
}

#[test]
fn sstatus_keeps_only_the_bits_this_machine_stands_behind() {
    // SUM (bit 18) and MXR (bit 19) were dropped until `F-001c2b2` — they change what a
    // page-table walk permits, and a kernel must not read back a permission model nothing
    // enforces. They are STORED now, and the rule did not change: there is a walker, it is
    // in user mode's way, and `tests/user.rs` holds both bits to what they permit.
    //
    // **FS (bits 14:13) JOINED THEM ON 2026-09-28, AND THIS TEST ASSERTED THE OPPOSITE.**
    // It read "FS, XS and VS are still dropped, because `fadd.d` still halts" — which was
    // true, and was the whole argument. Row `V-001` implemented F and D, so `fadd.d` runs;
    // a dropped FS would mean a program writes `FS = dirty`, reads back 0, and a context
    // switch built on that reading discards a live register file. The rule has not changed
    // once: a bit is kept exactly when the feature it gates exists.
    //
    // **XS STAYS DROPPED AND IS WHAT MAKES THIS TEST STILL DISCRIMINATE.** Asserting only
    // that FS survives would pass against a machine that had stopped narrowing at all, so
    // the write below sets XS (bits 16:15) too and the expectation excludes it. There is no
    // user extension, so nothing would honour it. VS is the same case until row `V-007`
    // lands the vector unit — at which point this test and `SSTATUS_MASK`'s own margin are
    // BOTH owed the refounding FS just had, and both say so.
    let mut m = machine(&[csr_insn(0x1, 0, SSTATUS, 6)]);
    let (sum, mxr, fs, xs) = (1 << 18, 1 << 19, 0x3 << 13, 0x3 << 15);
    m.x[6] = SSTATUS_SPP | sum | mxr | fs | xs;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.csr.sstatus,
        SSTATUS_SPP | sum | mxr | fs,
        "the WARL narrowing happens in the machine, so a later read cannot report a \
         permission bit as set when nothing would honour it — and cannot report one as \
         clear when the walker, or now the FPU, is honouring it"
    );
}

#[test]
fn a_breakpoint_with_no_handler_installed_still_stops() {
    // stvec is zero: nothing has claimed the trap. On metal this delivers to address 0
    // and executes whatever is there; saying so is better than running the rubble.
    assert_eq!(
        machine(&[EBREAK]).step(&mut Vec::new()),
        Some(Halt::Breakpoint { pc: BASE }),
        "F-001c1's halt survives exactly where it is still the honest answer"
    );
}

#[test]
fn a_breakpoint_enters_the_handler_with_the_four_registers_set() {
    let handler = BASE + 0x400;
    let mut m = machine(&[EBREAK]);
    // MODE = 1 is vectored, and vectored spreads INTERRUPTS across a table; an exception
    // enters at BASE either way. A machine that jumped to base + 4 * cause would land
    // twelve bytes into the handler and run the wrong instruction first.
    m.csr.stvec = handler | 1;
    m.csr.sstatus = SSTATUS_SIE;

    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "a delivered trap is not a halt"
    );
    assert_eq!(m.pc, handler, "control is at the vector base");
    assert_eq!(
        m.csr.scause, 3,
        "cause 3 is a breakpoint — privileged spec §4.1.9"
    );
    assert_eq!(
        m.csr.sepc, BASE,
        "sepc is the ebreak ITSELF, not the instruction after it: the handler is the one \
         that decides whether to retry or step over, and spec/trap.sas adds the four"
    );
    assert_eq!(m.csr.stval, BASE, "stval is the address of the breakpoint");
    assert_eq!(
        m.csr.sstatus & SSTATUS_SPP,
        SSTATUS_SPP,
        "the trap came from S-mode"
    );
    assert_eq!(
        m.csr.sstatus & SSTATUS_SPIE,
        SSTATUS_SPIE,
        "SIE was set, so SPIE is"
    );
    assert_eq!(
        m.csr.sstatus & SSTATUS_SIE,
        0,
        "and interrupts are off in the handler"
    );
}

#[test]
fn sret_returns_to_sepc_and_puts_sstatus_back() {
    let handler = BASE + 0x400;
    let mut m = machine(&[EBREAK]);
    m.mem[0x400..0x404].copy_from_slice(&SRET.to_le_bytes());
    m.csr.stvec = handler;
    m.csr.sstatus = SSTATUS_SIE;

    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.pc, handler);
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "sret executes; it does not halt"
    );
    assert_eq!(
        m.pc, BASE,
        "back at the breakpoint — the handler did not advance sepc, so this is where it \
         asked to go, and a second ebreak here would trap again. That is the loop \
         check-trap.sh watches for, and it is the program's job to break it, not the \
         machine's"
    );
    assert_eq!(
        m.csr.sstatus & SSTATUS_SIE,
        SSTATUS_SIE,
        "SIE came back from SPIE"
    );
    assert_eq!(
        m.csr.sstatus & SSTATUS_SPIE,
        SSTATUS_SPIE,
        "and SPIE is left set"
    );
    assert_eq!(
        m.csr.sstatus & SSTATUS_SPP,
        0,
        "SPP is cleared, as the spec says"
    );
}

#[test]
fn sret_with_spp_clear_enters_user_mode() {
    // SPP = 0 says the trap came from U-mode, so returning enters it. `F-001c2b1` refused
    // this — the walker followed no `U` leaf and `SUM` was dropped, so U-mode would have
    // seen every byte S-mode does — and `F-001c2b2` built what was missing rather than
    // keeping the refusal. `tests/user.rs` holds the walk; here the claim is narrower and
    // is the door itself: the jump is taken AND the mode moved with it.
    let mut m = machine(&[SRET]);
    m.csr.sepc = BASE + 0x40;
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "the machine is still running"
    );
    assert_eq!(m.pc, BASE + 0x40, "it went to sepc");
    assert_eq!(
        m.mode,
        Privilege::User,
        "and it is IN user mode — a jump to sepc that left the privilege alone would          satisfy every assertion above and be S-mode wearing another name"
    );
}

// ---------------------------------------------------------------------------------------
// Column 3: a whole program.

/// Assemble a Sassembly source at `load` and write the ELF `sadhana` would have written —
/// the same four calls `sadhana`'s own `main` makes, in the same order.
fn build(source: &str, load: u64) -> Vec<u8> {
    let program = assemble_program(source).unwrap_or_else(|e| panic!("{e:?}"));
    let (text, pending) = encode_object(&program).unwrap_or_else(|e| panic!("{e:?}"));
    let bytes = object(
        &text,
        &program,
        &pending,
        None,
        &layout_addresses(&program, Target::Uncompressed),
    );
    let objects = [read(&bytes).expect("the object reads back")];
    let image = link_at(&objects, load).unwrap_or_else(|e| panic!("{e:?}"));
    write_debuggable_at(&image.text, &image.data, &image.table, image.bss, &[], load)
}

/// The program `tools/check-trap.sh` boots under real OpenSBI: it installs a handler in
/// `stvec`, executes `ebreak`, prints `scause`, advances `sepc` past the breakpoint and
/// returns.
const TRAP: &str = include_str!("../../../spec/trap.sas");

/// Run `trap.sas` at `load` and return its two printed lines.
fn run_trap(load: u64) -> (String, String) {
    let elf = build(TRAP, load);
    let mut m = Machine::load_elf(&elf, 1 << 21).expect("the toolchain's own ELF must load");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(100_000, &mut out);
    assert!(
        matches!(halt, Halt::Shutdown { .. }),
        "the program ends by asking the firmware to stop; anything else means it never \
         got there: {halt:?}"
    );
    let text = String::from_utf8(out).expect("the program prints ASCII hex");
    let mut lines = text.lines();
    let cause = lines.next().unwrap_or_default().to_string();
    let pc = lines.next().unwrap_or_default().to_string();
    (cause, pc)
}

#[test]
fn trap_sas_says_the_same_two_lines_qemu_hears() {
    // `tools/check-trap.sh` demands EXACTLY this of QEMU under real OpenSBI, at these two
    // link addresses, and for the same reasons: two runs, because one cannot tell a
    // computed handler address from a lucky constant.
    let (cause_a, pc_a) = run_trap(0x8020_0000);
    let (cause_b, pc_b) = run_trap(0x8040_0000);

    assert_eq!(
        cause_a, "0000000000000003",
        "scause is 3 — the handler was reached, and it read the cause out of the machine \
         rather than being told it"
    );
    assert_eq!(cause_b, "0000000000000003");
    assert!(
        pc_a.starts_with("00000000802") && pc_b.starts_with("00000000804"),
        "the handler returned into the loaded image, and into the RIGHT one: {pc_a} {pc_b}"
    );
    assert_ne!(
        pc_a, pc_b,
        "the resume address tracks the link address, so it is sepc + 4 and not a constant \
         — the check the shell script makes for exactly this reason"
    );
}

#[test]
fn the_program_needed_the_trap_to_get_there() {
    // The control. If `trap.sas` were passing above for some other reason — an output
    // buffer that was already right, a constant that happened to match — then breaking
    // the delivery would not change the answer. Here it does: with the handler's address
    // never reaching stvec, the ebreak has nowhere to go and the machine says so.
    let elf = build(TRAP, 0x8020_0000);
    let mut m = Machine::load_elf(&elf, 1 << 21).expect("load");
    let mut out: Vec<u8> = Vec::new();
    for _ in 0..100_000 {
        m.csr.stvec = 0; // undo the install, every step
        if let Some(h) = m.step(&mut out) {
            assert_eq!(
                h,
                Halt::Breakpoint { pc: m.pc },
                "it stopped at the ebreak, which is the instruction the whole program is \
                 about"
            );
            assert!(out.is_empty(), "and nothing was printed");
            return;
        }
    }
    panic!("the program never reached its ebreak — then the test above proves nothing");
}
