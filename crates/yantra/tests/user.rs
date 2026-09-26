//! `F-001c2b2` — user mode: the `U` bit both ways, `SUM` and `MXR`, and `sret` into U.
//!
//! # Two columns, not three
//!
//! `F-001b`, `F-001c1`, `F-001c2a` and `F-001c2b1` each had an **encoding table** column,
//! because each added instructions. This row adds none: every instruction it touches —
//! `sret`, `ecall`, `sfence.vma`, the six CSR forms — was decoded before it, and
//! `tests/system.rs` already holds each against `spec/encodings-riscv64.tsv`. What moved
//! is the *mode they execute in*, which no encoding table records. So there are two
//! columns here:
//!
//! 1. **The meaning**, written from the privileged specification by hand: which of the
//!    two modes may touch which leaf, what `SUM` and `MXR` widen, what an `ecall` from
//!    U-mode is, and what happens to a privileged instruction executed by a user program.
//! 2. **A whole program** — `spec/user-mode.sas`, at both link addresses, asserted against
//!    the *same four words* `tools/check-user-mode.sh` demands of QEMU under real OpenSBI.
//!
//! # The one number that carries the claim
//!
//! `scause = 8` is ENVIRONMENT CALL FROM U-MODE. An S-mode `ecall` raises 9, and every
//! `ecall` this tree has ever taken has raised 9 — an 8 cannot be produced from supervisor
//! mode, which is why the check script treats it as the evidence rather than a print
//! statement saying "userspace". `SPP` clear at the same moment is the same fact read out
//! of a second register, and both are demanded because a handler that set a variable could
//! produce either one alone.

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::{Csrs, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;
/// 16 MiB. `spec/user-mode.sas` reserves six page frames in its `.bss` and every physical
/// address either program uses stays inside this.
const RAM: usize = 1 << 24;

/// `satp.MODE = 8`, already shifted.
const SV39: u64 = 8 << 60;
/// The 44-bit page number at the bottom of `satp`.
const PPN: u64 = (1 << 44) - 1;

const PTE_V: u64 = 1;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const PTE_A: u64 = 1 << 6;
const PTE_D: u64 = 1 << 7;
/// The permissive leaf `spec/paging.sas` writes — `V R W X A D`, and no `U`.
const KERNEL: u64 = PTE_V | PTE_R | PTE_W | PTE_X | PTE_A | PTE_D;

/// `sstatus.SPP`, bit 8.
const SPP: u64 = 1 << 8;
/// `sstatus.SUM`, bit 18.
const SUM: u64 = 1 << 18;
/// `sstatus.MXR`, bit 19.
const MXR: u64 = 1 << 19;

/// `scause` 2 — illegal instruction.
const ILLEGAL: u64 = 2;
/// `scause` 8 — environment call from U-mode. The claim.
const U_ECALL: u64 = 8;
/// `scause` 12, 13, 15 — fetch, load and store page faults.
const FETCH_FAULT: u64 = 12;
const LOAD_FAULT: u64 = 13;
const STORE_FAULT: u64 = 15;

/// Where a handler is installed in the hand-built machines below. Any non-zero address
/// would do; the assertions read `stvec` rather than this constant.
const HANDLER: u64 = BASE + 0x800;

const ECALL: u32 = 0x0000_0073;
const SRET: u32 = 0x1020_0073;
const SFENCE_VMA: u32 = 0x1200_0073;

/// `csrrs x5, sstatus, x0` — a pure *read* of a supervisor CSR, and the mildest privileged
/// instruction there is. From U-mode it is still illegal.
const CSRR_SSTATUS: u32 = 0x73 | 5 << 7 | 0x2 << 12 | 0x100 << 20;

/// A machine holding `words` at the entry point and nothing else.
fn machine(words: &[u32]) -> Machine {
    let mut m = Machine {
        x: [0; 32],
        pc: BASE,
        base: BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    for (i, w) in words.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

/// Write one page-table entry at a physical address.
fn put_pte(m: &mut Machine, at: u64, pte: u64) {
    let o = (at - m.base) as usize;
    m.mem[o..o + 8].copy_from_slice(&pte.to_le_bytes());
}

/// A leaf entry mapping to physical page `pa`, with `flags`.
fn leaf(pa: u64, flags: u64) -> u64 {
    (pa >> 12) << 10 | flags
}

/// The second virtual view of RAM the machines below use. It is in the gigabyte ABOVE the
/// one the image is linked into, and it maps back onto the same physical gigabyte — so
/// `DATA_VA` and `BASE + 0x4000` are one page of memory reached two ways, under two sets
/// of permissions. That is what makes a permission test a test: the byte is the same byte,
/// and only the leaf differs.
const DATA_VA: u64 = 0xc000_4000;
/// The physical address `DATA_VA` resolves to.
const DATA_PA: u64 = BASE + 0x4000;

/// A machine with translation on, running `words`, whose address space is two gigapages
/// onto the *same* physical gigabyte: the image's own virtual gigabyte with `code_flags`,
/// and [`DATA_VA`]'s with `data_flags`.
///
/// Two views rather than one is the point. The `U` bit under test sits on the data view,
/// so a fetch does not depend on it and the machine can keep running while a load is
/// refused; one leaf for both would make every case a fetch fault at the first step.
fn two_maps(words: &[u32], code_flags: u64, data_flags: u64) -> Machine {
    let mut m = machine(words);
    let root = BASE + 0x2000;
    let giga = BASE & !((1 << 30) - 1);
    let entry = |va: u64| root + ((va >> 30) & 0x1ff) * 8;
    put_pte(&mut m, entry(BASE), leaf(giga, code_flags));
    put_pte(&mut m, entry(DATA_VA), leaf(giga, data_flags));
    m.csr.satp = SV39 | (root >> 12);
    m
}

/// Put an instruction at a physical address, for a page some other view will execute.
fn put_insn(m: &mut Machine, pa: u64, word: u32) {
    let o = (pa - m.base) as usize;
    m.mem[o..o + 4].copy_from_slice(&word.to_le_bytes());
}

/// `sd x6, 0(x5)` and `ld x6, 0(x5)`, the two data accesses the tables below drive.
fn store_word() -> u32 {
    0x23 | 0x3 << 12 | 5 << 15 | 6 << 20
}
fn load_word() -> u32 {
    0x03 | 0x3 << 12 | 5 << 15 | 6 << 7
}

// ---------------------------------------------------------------------------------------
// Column 1: the meaning — the `U` bit, both ways.

#[test]
fn user_mode_reaches_a_u_page_and_nothing_else() {
    // The whole of the protection is this bit. A user program's own page carries it; the
    // kernel's pages do not, and that is what stops the program reading them.
    for (flags, permitted) in [(KERNEL | PTE_U, true), (KERNEL, false)] {
        // The code view carries `U` too, because a user program has to be able to fetch
        // its own instructions; what varies is the leaf the load goes through.
        let mut m = two_maps(&[load_word()], KERNEL | PTE_U, flags);
        m.mode = Privilege::User;
        m.x[5] = DATA_VA;
        let halt = m.step(&mut Vec::new());
        if permitted {
            assert_eq!(halt, None, "a U leaf is exactly what U-mode may touch");
        } else {
            assert_eq!(
                halt,
                Some(Halt::PageFault {
                    pc: BASE,
                    addr: DATA_VA,
                    cause: LOAD_FAULT
                }),
                "a leaf without U is the kernel's, and U-mode is refused it"
            );
        }
    }
}

#[test]
fn supervisor_needs_sum_to_touch_a_u_page_and_never_for_a_fetch() {
    // The other direction, and the asymmetry is the specification's. `SUM` lets the kernel
    // read and write a user page — a syscall argument has to be readable somehow. It does
    // NOT let the kernel *execute* one, ever: a kernel that could would run a user
    // program's instructions with the kernel's privilege, which is the whole attack `SUM`
    // is carefully written not to open.
    for (sum, permitted) in [(0, false), (SUM, true)] {
        let mut m = two_maps(&[load_word()], KERNEL, KERNEL | PTE_U);
        m.csr.sstatus = sum;
        m.x[5] = DATA_VA;
        let halt = m.step(&mut Vec::new());
        assert_eq!(
            halt.is_none(),
            permitted,
            "a supervisor load of a U page with SUM {}",
            if sum == 0 { "clear" } else { "set" }
        );
    }
    // The fetch. `pc` is put in the U-mapped view, so the instruction the machine reaches
    // for is on a user page — with `SUM` set, which changes nothing.
    let mut m = two_maps(&[], KERNEL, KERNEL | PTE_U);
    m.csr.sstatus = SUM;
    m.pc = DATA_VA;
    put_insn(&mut m, DATA_PA, ECALL);
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::PageFault {
            pc: DATA_VA,
            addr: DATA_VA,
            cause: FETCH_FAULT
        }),
        "SUM permits loads and stores. It does not make a user page executable by the \
         kernel, and the spec says so in as many words"
    );
}

#[test]
fn mxr_widens_a_load_and_only_a_load() {
    // An execute-only page: `X` and no `R`. A load of it faults, unless `MXR` says a page
    // that may be executed may also be read.
    let x_only = PTE_V | PTE_X | PTE_A | PTE_D;
    for (mxr, permitted) in [(0, false), (MXR, true)] {
        let mut m = two_maps(&[load_word()], KERNEL, x_only);
        m.csr.sstatus = mxr;
        m.x[5] = DATA_VA;
        assert_eq!(
            m.step(&mut Vec::new()).is_none(),
            permitted,
            "a load of an execute-only page with MXR {}",
            if mxr == 0 { "clear" } else { "set" }
        );
    }
    // And only a load. A store to the same page is refused with `MXR` set, because `MXR`
    // has nothing to say about `W` — reading it as "ignore permissions" would make an
    // execute-only page writable.
    let mut m = two_maps(&[store_word()], KERNEL, x_only);
    m.csr.sstatus = MXR;
    m.x[5] = DATA_VA;
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::PageFault {
            pc: BASE,
            addr: DATA_VA,
            cause: STORE_FAULT
        }),
        "MXR widens R by X. It does not widen W"
    );
}

// ---------------------------------------------------------------------------------------
// Column 1, continued: what a user program may execute, and where its traps go.

#[test]
fn an_ecall_from_user_mode_is_cause_8_and_not_an_sbi_call() {
    // THE claim. In S-mode `a7 = 1` is `sbi_console_putchar` and puts a byte on the
    // console; in U-mode the same instruction is exception 8 delivered to the kernel, and
    // nothing is printed. A machine that read `a7` first would print here.
    let mut m = machine(&[ECALL]);
    m.mode = Privilege::User;
    m.csr.stvec = HANDLER;
    m.csr.sstatus = SPP; // the last trap came from S-mode; this one does not
    m.x[17] = 1; // a7 — sbi_console_putchar, were this an SBI call
    m.x[10] = u64::from(b'x');
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(m.step(&mut out), None, "the machine is still running");
    assert!(
        out.is_empty(),
        "an environment call from U-mode is not an SBI call"
    );
    assert_eq!(m.csr.scause, U_ECALL, "cause 8, which S-mode cannot forge");
    assert_eq!(
        m.csr.sepc, BASE,
        "sepc is the ecall itself, not the next word"
    );
    assert_eq!(m.csr.stval, 0, "stval is 0 for an environment call");
    assert_eq!(m.pc, HANDLER, "and the kernel's handler is running");
    assert_eq!(
        m.csr.sstatus & SPP,
        0,
        "SPP records where it came from: U-mode"
    );
    assert_eq!(
        m.mode,
        Privilege::Supervisor,
        "and the handler runs in S-mode"
    );
}

#[test]
fn an_ecall_from_supervisor_mode_is_still_an_sbi_call() {
    // The control. The same instruction, the same `stvec`, the same `a7` — and it prints,
    // because an environment call from supervisor mode is the one exception that always
    // goes up to the firmware, and here the firmware is this interpreter.
    let mut m = machine(&[ECALL]);
    m.csr.stvec = HANDLER;
    m.x[17] = 1;
    m.x[10] = u64::from(b'x');
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(m.step(&mut out), None);
    assert_eq!(out, b"x", "S-mode ecall reached the firmware");
    assert_eq!(m.csr.scause, 0, "nothing was delivered to stvec");
    assert_eq!(m.pc, BASE + 4, "and it returned to the next instruction");
}

#[test]
fn every_privileged_instruction_is_illegal_in_user_mode() {
    // A user program that could execute one of these would not be in user mode. `sret`
    // would let it leave; `sfence.vma` would let it shoot down the kernel's translations;
    // a CSR *read* alone would tell it where the kernel's page tables are — which is why
    // the CSR row here is `csrrs rd, sstatus, x0`, the pure read, and not a write.
    for word in [SRET, SFENCE_VMA, CSRR_SSTATUS] {
        let mut m = machine(&[word]);
        m.mode = Privilege::User;
        m.csr.stvec = HANDLER;
        assert_eq!(
            m.step(&mut Vec::new()),
            None,
            "{word:#010x} is delivered, not obeyed"
        );
        assert_eq!(m.csr.scause, ILLEGAL, "{word:#010x}: cause 2");
        assert_eq!(
            m.csr.stval,
            u64::from(word),
            "{word:#010x}: stval is the instruction, which is what the handler decodes"
        );
        assert_eq!(m.pc, HANDLER, "{word:#010x}: the handler is running");
        assert_eq!(m.csr.sstatus & SPP, 0, "{word:#010x}: it came from U-mode");
        assert_eq!(m.mode, Privilege::Supervisor);
        assert_eq!(m.x[5], 0, "{word:#010x}: and rd was NOT written on the way");
    }
}

#[test]
fn a_user_trap_with_no_handler_stops_rather_than_running_the_rubble_at_zero() {
    // The same choice `ebreak` and a page fault make. With `stvec` zero there is nowhere
    // to deliver an exception, and on metal the machine would trap to address 0 and
    // execute whatever is there.
    for (word, cause) in [(ECALL, U_ECALL), (SRET, ILLEGAL)] {
        let mut m = machine(&[word]);
        m.mode = Privilege::User;
        assert_eq!(
            m.step(&mut Vec::new()),
            Some(Halt::Undelivered { pc: BASE, cause }),
            "{word:#010x}"
        );
        assert_eq!(m.pc, BASE, "and it stopped where it was");
    }
}

#[test]
fn spp_records_the_mode_the_trap_came_from() {
    // `sret` returns to whatever `SPP` names, so a trap that recorded the wrong mode sends
    // the handler's `sret` to the wrong privilege — a user program's `ecall` would return
    // into the kernel's own privilege at the kernel's own `sepc`. Both directions, one
    // instruction: `ebreak`, which traps from either mode.
    let ebreak = 0x0010_0073;
    for (mode, spp) in [(Privilege::Supervisor, SPP), (Privilege::User, 0)] {
        let mut m = machine(&[ebreak]);
        m.mode = mode;
        m.csr.stvec = HANDLER;
        m.csr.sstatus = SPP; // set beforehand, so a U-mode trap has to CLEAR it
        assert_eq!(m.step(&mut Vec::new()), None);
        assert_eq!(m.csr.sstatus & SPP, spp, "{mode:?}");
    }
}

#[test]
fn sret_returns_to_the_mode_spp_names_and_the_round_trip_closes() {
    // U-mode entered, trapped out of, and entered again — which is what a kernel does on
    // every syscall. The second `sret` is the one that matters: the handler's own `sret`
    // must go back to U-mode, and it can only know that from `SPP`, which the trap wrote.
    let mut m = two_maps(&[SRET], KERNEL, KERNEL | PTE_U);
    let user = DATA_VA;
    put_insn(&mut m, DATA_PA, ECALL);
    m.csr.sepc = user;
    m.csr.stvec = HANDLER;
    assert_eq!(m.step(&mut Vec::new()), None, "sret");
    assert_eq!(
        (m.pc, m.mode),
        (user, Privilege::User),
        "in user mode, on the U page"
    );
    assert_eq!(m.step(&mut Vec::new()), None, "the user's ecall");
    assert_eq!(m.csr.scause, U_ECALL);
    assert_eq!(
        (m.pc, m.mode),
        (HANDLER, Privilege::Supervisor),
        "back in the kernel"
    );
    // The handler returns. `sepc` is the user's `ecall`; a real kernel would step it past.
    m.mem[(HANDLER - BASE) as usize..(HANDLER - BASE) as usize + 4]
        .copy_from_slice(&SRET.to_le_bytes());
    assert_eq!(m.step(&mut Vec::new()), None, "the handler's sret");
    assert_eq!(
        (m.pc, m.mode),
        (user, Privilege::User),
        "and it went back to U-mode, because SPP said that is where the trap came from"
    );
}

// ---------------------------------------------------------------------------------------
// Column 2: a whole program.

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

/// The program `tools/check-user-mode.sh` boots under real OpenSBI: a three-level Sv39 map
/// with `U` on exactly one 4 KiB page, entered by `sret`, `ecall`ing back out.
const USER_MODE: &str = include_str!("../../../spec/user-mode.sas");

/// Run `user-mode.sas` at `load` and return its four printed words.
fn run_user_mode(load: u64) -> Vec<u64> {
    let elf = build(USER_MODE, load);
    let mut m = Machine::load_elf(&elf, RAM).expect("the toolchain's own ELF must load");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(5_000_000, &mut out);
    assert!(
        matches!(halt, Halt::Shutdown { .. }),
        "the program ends by asking the firmware to stop; anything else means it died on \
         the way up or on the way back: {halt:?}"
    );
    String::from_utf8(out)
        .expect("the program prints ASCII hex")
        .lines()
        .map(|l| u64::from_str_radix(l, 16).unwrap_or_else(|e| panic!("{l:?}: {e}")))
        .collect()
}

#[test]
fn user_mode_sas_says_the_same_four_words_qemu_hears() {
    // Exactly what `tools/check-user-mode.sh` demands of QEMU, at the two link addresses
    // it uses, derived the way it derives them: word 3's expected value comes out of word
    // 1 and out of nothing this test was told separately. The layout — root table,
    // megapage table, page table, window table, the user's page, the data page — is the
    // program's own declaration of it.
    const USER_FRAME: u64 = 4 * 4096;
    for load in [0x8020_0000u64, 0x8040_0000] {
        let w = run_user_mode(load);
        assert_eq!(
            w.len(),
            4,
            "fewer than four means the machine stopped where the line stops: a bad root \
             table kills the fetch after the satp write, and a page without U kills the \
             sret: {w:?}"
        );
        let (satp, shared, cause, sstatus) = (w[0], w[1], w[2], w[3]);
        assert_eq!(satp >> 60, 8, "word 1: Sv39 is the mode it installs");
        let tables = (satp & PPN) << 12;
        assert_eq!(tables % 4096, 0, "word 1: satp keeps a page NUMBER");
        assert_eq!(
            shared,
            tables + USER_FRAME,
            "word 3: the program stores the user page's address in the data frame and \
             reads it back at a virtual address that is not that frame's own, so a word \
             that is not that address did not come through the window"
        );
        assert_eq!(
            cause, U_ECALL,
            "word 5: cause 8 is ENVIRONMENT CALL FROM U-MODE. A 9 here is the S-mode \
             ecall this tree has taken every other time, and says the process never left \
             supervisor mode"
        );
        assert_eq!(
            sstatus & SPP,
            0,
            "word 6: SPP clear is the same fact from a second register, and both are \
             demanded because a handler that set a variable could produce either alone"
        );
    }
    // What moved and what had better not. Words 1 and 3 are derived from where the image
    // was loaded; a value that did not move is a constant in the source and the derivation
    // it stands for never happened. A cause number is architectural and must not move.
    let (lo, hi) = (run_user_mode(0x8020_0000), run_user_mode(0x8040_0000));
    assert_ne!(lo[0], hi[0], "satp is the same at both link addresses");
    assert_ne!(
        lo[1], hi[1],
        "the windowed word is the same at both link addresses"
    );
    assert_eq!(lo[2], hi[2], "the cause number followed the link address");
}

#[test]
fn the_program_needed_user_mode_to_get_there() {
    // The control, and it is the one `tools/check-user-mode.sh` cannot run: hold the
    // machine in supervisor mode across the `sret` — the same image, the same map, the
    // same handler — and see what the four words become.
    //
    // They become 12 and SPP set. The kernel cannot even FETCH from the user's page: `U`
    // is on it, and no `SUM` makes a user page executable by supervisor mode. So the
    // program does not merely print a different cause here, it never reaches its own
    // `ecall` — which is the sharpest form of the control, because it says the sret's
    // change of privilege is load-bearing three instructions before the number that
    // reports it.
    let elf = build(USER_MODE, 0x8020_0000);
    let mut m = Machine::load_elf(&elf, RAM).expect("load");
    let mut out: Vec<u8> = Vec::new();
    for _ in 0..5_000_000 {
        m.mode = Privilege::Supervisor; // undo the sret's promotion, every step
        if m.step(&mut out).is_some() {
            break;
        }
    }
    let words: Vec<u64> = String::from_utf8(out)
        .expect("ASCII hex")
        .lines()
        .filter_map(|l| u64::from_str_radix(l, 16).ok())
        .collect();
    assert_eq!(
        words.get(2).copied(),
        Some(FETCH_FAULT),
        "with the machine pinned in S-mode the user's page is unreachable to fetch from, \
         so the cause is 12 and never the 8 the real run prints: {words:?}"
    );
    assert_eq!(
        words.get(3).map(|s| s & SPP),
        Some(SPP),
        "and SPP says supervisor, which is the whole of what was pinned: {words:?}"
    );
}

/// The program `tools/check-root-task.sh` boots under real OpenSBI: a root task that walks
/// a description, builds two address spaces from it, runs the *same* user text under both,
/// and restarts only the one whose contract says restartable.
const ROOT_TASK: &str = include_str!("../../../spec/root-task.sas");

/// Run `root-task.sas` at `load` and return its nine printed words.
fn run_root_task(load: u64) -> Vec<u64> {
    let elf = build(ROOT_TASK, load);
    let mut m = Machine::load_elf(&elf, RAM).expect("the toolchain's own ELF must load");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(20_000_000, &mut out);
    assert!(
        matches!(halt, Halt::Shutdown { .. }),
        "the program ends by asking the firmware to stop: {halt:?}"
    );
    String::from_utf8(out)
        .expect("the program prints ASCII hex")
        .lines()
        .map(|l| u64::from_str_radix(l, 16).unwrap_or_else(|e| panic!("{l:?}: {e}")))
        .collect()
}

#[test]
fn root_task_sas_says_the_same_nine_words_qemu_hears() {
    // The second half of this row's gate, and a much larger program than `user-mode.sas`:
    // two address spaces, one user text run under both, a fault taken in each, and a
    // supervisor that brings one back and not the other. It needs everything above at
    // once — `sret` into U with two different `satp`s, `U` leaves, and a store page fault
    // taken from user mode and delivered to the kernel.
    //
    // Every assertion here is `tools/check-root-task.sh`'s, in its order.
    for load in [0x8020_0000u64, 0x8040_0000] {
        let w = run_root_task(load);
        assert_eq!(w.len(), 9, "nine words, in the program's own order: {w:?}");
        let (satp0, satp1, count) = (w[0], w[1], w[2]);
        let (cause0, life1, restarts0) = (w[3], w[4], w[5]);
        let (life2, cause1, restarts1) = (w[6], w[7], w[8]);

        assert_eq!(satp0 >> 60, 8, "word 1: Sv39");
        assert_eq!(satp1 >> 60, 8, "word 2: Sv39");
        assert_ne!(
            satp0, satp1,
            "words 1 and 2: one satp for both services is ONE address space, and then \
             there is no second service to hold a different contract"
        );
        assert_eq!(
            count, 2,
            "word 3: the service count read back out of the description the root task \
             walked. Two spaces under a count that does not say two means the spaces were \
             written by hand and the description is decoration"
        );
        assert_eq!(
            cause0, STORE_FAULT,
            "word 4: the driver is killed by storing where its own space does not grant \
             it, so cause 15 — and a store fault taken IN user mode is the thing this \
             row built"
        );
        assert_ne!(
            life1, 0,
            "word 5: zero is what the frame reads before anything writes to it, and \
             nothing then distinguishes 'the service wrote 0' from 'it never ran'"
        );
        assert_eq!(restarts0, 1, "word 6: restarted exactly once");
        assert_eq!(
            life2,
            life1.wrapping_add(1),
            "word 7: the kernel writes the restart count into the frame before every \
             entry and the user text adds one, so a second life MUST write one more than \
             the first. Equal words are a thread resumed, not a service restarted"
        );
        assert_eq!(
            cause1, cause0,
            "word 8: the two services run the SAME user text and must die the same death, \
             or word 9 is reporting on a different situation"
        );
        assert_eq!(
            restarts1, 0,
            "word 9: service 1's restartable bit is clear. A supervisor that brought it \
             back anyway is restarting unconditionally rather than honouring a contract"
        );
    }
    // Both `satp`s are page numbers of tables the program placed in its own `.bss`, so
    // both move with the link address. Ones that did not are constants in the source.
    let (lo, hi) = (run_root_task(0x8020_0000), run_root_task(0x8040_0000));
    assert_ne!(lo[0], hi[0], "word 1 is the same at both link addresses");
    assert_ne!(lo[1], hi[1], "word 2 is the same at both link addresses");
    assert_eq!(
        (lo[3], lo[8]),
        (hi[3], hi[8]),
        "the cause and the withheld restart are architectural and a contract; neither \
         follows the link address"
    );
}
