//! `F-001e2a` — the loader: ADR-0015 clause A1 executed rather than described.
//!
//! # What these tests are for
//!
//! `F-001e1` wrote the entry contract down and asserted the tables against the registries
//! they are made of. Nothing entered anything. This file enters something: an ELF the
//! project's own writer produced, placed by [`yantra::loader::load_application`], reached
//! by a real `sret`, running in user mode at its own `e_entry` on a stack it did not
//! choose.
//!
//! # The three claims, and how each is falsifiable
//!
//! 1. **The register file is `spec/application-entry.tsv`'s, all 32 of it.**
//!    [`every_register_at_entry_is_the_one_the_contract_names`] reads that table and
//!    checks each row against the machine — so a loader that left a leftover in `s3`
//!    fails, and so does one that stops setting a register the table names.
//! 2. **The machine changes mode, not the loader.** The loader leaves `sepc`, `SPP` and
//!    `satp` set and stops. One `sret` — one instruction the interpreter executes —
//!    is what puts the hart in U-mode, and
//!    [`the_sret_is_what_enters_user_mode_and_the_loader_only_sets_it_up`] reads the
//!    privilege back out on both sides of that single step.
//! 3. **The program does not learn its load address.** Its frames are the loader's, they
//!    are nowhere in its registers, and its own virtual addresses are nothing like them.
//!
//! # The fixture is not an application
//!
//! `F-001e3` writes the smallest honest application, and ADR-0015 forbids writing one
//! early to make `F-001` satisfiable. The programs here are three and four instructions of
//! hand-encoded RV64 in a `kosha` ELF — enough to prove the loader entered *something*,
//! deliberately not enough to be mistaken for an app, and not in `spec/`.

use sadhana::kosha::write_debuggable_at;
use std::path::{Path, PathBuf};
use yantra::loader::{
    ABI_VERSION, HANDLE_VECTOR, MAX_HANDLES, STACK_PAGES, STACK_TOP, load_application,
};
use yantra::{Csrs, Halt, Machine, Privilege};

/// Physical RAM, and the supervisor's own address: the machine's, not the loader's.
const BASE: u64 = 0x8000_0000;
/// 4 MiB. The loader takes about a dozen frames; the rest is slack so a refusal is a
/// refusal and not an accident of the fixture.
const RAM: usize = 1 << 22;
/// Where the loader may start taking frames. Everything below is the supervisor's.
const FREE: u64 = BASE + 0x1_0000;
/// Where the application is linked. Any gigabyte but the supervisor's would do.
const APP: u64 = 0x1000_0000;
/// Where the supervisor's `sret` sits, and where it resumes after a trap.
const SRET_AT: u64 = BASE;
const HANDLER: u64 = BASE + 0x800;

const SRET: u32 = 0x1020_0073;
const ECALL: u32 = 0x0000_0073;
/// `jal x0, .` — a self-jump, which this machine reports as [`Halt::SpinForever`]. It is
/// the handler's whole body: these tests want the trap, not what follows it.
const SPIN: u32 = 0x0000_006f;

/// `scause` 8 — environment call from U-mode.
const U_ECALL: u64 = 8;
/// `scause` 13 and 15 — load and store page faults.
const LOAD_FAULT: u64 = 13;
const STORE_FAULT: u64 = 15;
/// `sstatus.SPP`, bit 8.
const SPP: u64 = 1 << 8;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

/// `spec/application-entry.tsv`, comments and header dropped.
fn entry_contract() -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(root().join("spec/application-entry.tsv"))
        .expect("spec/application-entry.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .skip(1)
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

// --- three encoders, because four instructions do not need an assembler -----------------

fn i_type(opcode: u32, rd: u32, funct3: u32, rs1: u32, imm: i32) -> u32 {
    opcode | rd << 7 | funct3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn s_type(opcode: u32, funct3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    opcode | (imm & 0x1f) << 7 | funct3 << 12 | rs1 << 15 | rs2 << 20 | (imm >> 5 & 0x7f) << 25
}
/// `addi rd, rs1, imm`.
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x13, rd, 0x0, rs1, imm)
}
/// `ld rd, imm(rs1)`.
fn ld(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x03, rd, 0x3, rs1, imm)
}
/// `sd rs2, imm(rs1)`.
fn sd(rs1: u32, rs2: u32, imm: i32) -> u32 {
    s_type(0x23, 0x3, rs1, rs2, imm)
}

/// An ELF from the project's own writer, holding `words`, linked at `at`.
fn image(words: &[u32], at: u64) -> Vec<u8> {
    let text: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    write_debuggable_at(&text, &[], &[], 0, &[], at)
}

/// A supervisor: `sret` at [`SRET_AT`], a spinning handler at [`HANDLER`], and `SPP` set —
/// so a loader that forgets to clear it returns to S-mode and the test says so.
fn supervisor() -> Machine {
    let mut m = Machine {
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: SRET_AT,
        base: BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    let put = |m: &mut Machine, at: u64, w: u32| {
        let o = (at - BASE) as usize;
        m.mem[o..o + 4].copy_from_slice(&w.to_le_bytes());
    };
    put(&mut m, SRET_AT, SRET);
    put(&mut m, HANDLER, SPIN);
    m.csr.stvec = HANDLER;
    m.csr.sstatus = SPP;
    m
}

/// Load `words` as an application with `handles` granted, and return the machine standing
/// on the supervisor's `sret`.
fn loaded(words: &[u32], handles: &[u64]) -> (Machine, yantra::loader::Loaded) {
    let mut m = supervisor();
    let elf = image(words, APP);
    let l = load_application(&mut m, &elf, FREE, handles).expect("the fixture must load");
    (m, l)
}

// ---------------------------------------------------------------------------------------
// Claim 1: the register file is the contract's.

#[test]
fn every_register_at_entry_is_the_one_the_contract_names() {
    // The whole point of ADR-0016's 32-row table: not three registers and "the rest are
    // whatever". This reads the table rather than restating it, so a row that changes
    // changes what the loader is held to.
    let handles = [7u64, 9, 11];
    let (m, l) = loaded(&[SPIN], &handles);
    let contract = entry_contract();
    assert_eq!(contract.len(), 32, "the contract states all 32");
    for row in &contract {
        let (abi, number, at_entry) = (&row[0], row[1].parse::<usize>().unwrap(), &row[2]);
        let want = match at_entry.as_str() {
            "0" => 0,
            "stack-top" => l.sp,
            "handle-count" => handles.len() as u64,
            "handle-vector" => l.handles,
            "abi-version" => ABI_VERSION,
            other => panic!(
                "spec/application-entry.tsv says {abi} holds {other}, which this test does not know how to check — add it here rather than weakening the row"
            ),
        };
        assert_eq!(
            m.x[number], want,
            "{abi} (x{number}) must hold {at_entry} at entry"
        );
    }
    assert_eq!(m.x[1], 0, "ra is zero: there is nowhere to return to (A5)");
    assert_eq!(l.sp, STACK_TOP, "sp is the stack the loader mapped");
    assert_eq!(l.sp % 16, 0, "the stack pointer is 16-byte aligned");
}

// ---------------------------------------------------------------------------------------
// Claim 2: the machine changes mode, not the loader.

#[test]
fn the_sret_is_what_enters_user_mode_and_the_loader_only_sets_it_up() {
    let (mut m, l) = loaded(&[SPIN], &[]);
    assert_eq!(
        m.mode,
        Privilege::Supervisor,
        "the loader leaves the hart in S-mode — it prepares the transition, it does not \
         perform it"
    );
    assert_eq!(m.csr.sepc, l.entry, "sepc is e_entry");
    assert_eq!(
        m.csr.sstatus & SPP,
        0,
        "SPP clear, or the sret goes nowhere"
    );
    assert_eq!(
        m.csr.satp, l.satp,
        "the space is installed before the return"
    );

    assert_eq!(m.step(&mut Vec::new()), None, "one sret, and it lands");
    assert_eq!(m.mode, Privilege::User, "A2: the program runs in U-mode");
    assert_eq!(m.pc, l.entry, "at its own entry point, which is the ELF's");
    assert_eq!(
        m.pc, APP,
        "which is where it was linked, not where it was placed"
    );
}

#[test]
fn the_program_runs_on_the_stack_the_loader_chose_and_falls_off_neither_end() {
    // `sd` below `sp` is the only writable memory an application gets from this toolchain
    // (`kosha` emits one `PF_R | PF_X` segment), so this is also the test that the stack
    // is real. The two faults on either side of it are what make it a *bounded* stack
    // rather than a permission to write wherever.
    let (mut m, _) = loaded(&[addi(5, 0, 42), sd(2, 5, -8), ld(6, 2, -8), SPIN], &[]);
    let halt = m.run(64, &mut Vec::new());
    assert_eq!(
        halt,
        Halt::SpinForever { pc: APP + 12 },
        "four instructions, in user mode, ending on their own self-jump: {halt:?}"
    );
    assert_eq!(m.x[6], 42, "what it stored below sp is what it read back");

    // At `sp` exactly: the page above the stack is not mapped, and this is the assertion
    // that STACK_TOP is exclusive.
    let (mut m, _) = loaded(&[sd(2, 0, 0)], &[]);
    m.step(&mut Vec::new());
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "the store faults to the handler rather than halting"
    );
    assert_eq!(m.csr.scause, STORE_FAULT, "a store fault");
    assert_eq!(m.csr.stval, STACK_TOP, "at the top of the stack, exclusive");

    // Below the lowest page: the guard is the absence of a mapping, not a check. The
    // address is walked down in `addi`-sized steps because a 12-bit immediate cannot
    // reach past the stack in one — which is itself the reason a stack this size needs a
    // guard rather than a bounds check.
    let stack = STACK_PAGES * 4096;
    let steps = (stack / 2048) as usize;
    let mut words: Vec<u32> = vec![addi(5, 5, -2048); steps];
    words.insert(0, addi(5, 2, 0));
    words.push(sd(5, 0, -8));
    let (mut m, _) = loaded(&words, &[]);
    let halt = m.run(64, &mut Vec::new());
    assert_eq!(halt, Halt::SpinForever { pc: HANDLER }, "{halt:?}");
    assert_eq!(m.csr.scause, STORE_FAULT);
    assert_eq!(
        m.csr.stval,
        STACK_TOP - stack - 8,
        "an overflow past the last stack page is a fault, not a neighbour's memory"
    );
}

#[test]
fn the_handle_vector_is_readable_by_number_and_writable_by_nobody() {
    // A4: every surface arrives here and there is no other way to obtain one. A program
    // that could write this page would be granting itself, which is what doc 07 §3's
    // "nothing is ambient" forbids.
    let (mut m, l) = loaded(&[ld(5, 11, 0), ld(6, 11, 8), SPIN], &[0xbeef, 0xcafe]);
    assert_eq!(l.handles, HANDLE_VECTOR);
    let halt = m.run(64, &mut Vec::new());
    assert_eq!(halt, Halt::SpinForever { pc: APP + 8 }, "{halt:?}");
    assert_eq!(
        (m.x[5], m.x[6]),
        (0xbeef, 0xcafe),
        "the grant, read by number"
    );

    let (mut m, _) = loaded(&[sd(11, 0, 0)], &[0xbeef]);
    m.step(&mut Vec::new());
    m.step(&mut Vec::new());
    assert_eq!(m.csr.scause, STORE_FAULT, "the vector is read-only");
    assert_eq!(m.csr.stval, HANDLE_VECTOR);
}

#[test]
fn an_ecall_from_the_loaded_program_reaches_the_supervisor_as_exception_eight() {
    // The seam `F-001e2b` picks up. Nothing here answers the call: what is asserted is
    // that a call made by a loaded application arrives at the supervisor's `stvec` as
    // exception 8 with the hart back in S-mode, which is the precondition for answering it.
    let (mut m, _) = loaded(&[addi(17, 0, 0), ECALL], &[]);
    let halt = m.run(64, &mut Vec::new());
    assert_eq!(
        halt,
        Halt::SpinForever { pc: HANDLER },
        "the call goes to the handler the supervisor installed: {halt:?}"
    );
    assert_eq!(
        m.csr.scause, U_ECALL,
        "8, and it cannot be produced from S-mode"
    );
    assert_eq!(m.csr.sepc, APP + 4, "sepc is the ecall itself");
    assert_eq!(
        m.csr.sstatus & SPP,
        0,
        "SPP records that it came from U-mode"
    );
    assert_eq!(
        m.mode,
        Privilege::Supervisor,
        "and the supervisor is running"
    );
}

#[test]
fn the_supervisor_is_mapped_beside_the_program_and_unreachable_from_it() {
    // Both halves matter. The gigapage has to be there — a trap that cannot fetch its own
    // handler is a machine that stops, and the `ecall` test above is what proves it is.
    // Here is the other half: it carries no `U`, so the program cannot read a byte of it.
    // `x0` is the only register holding an address the program can use without building
    // one, and 0 is not mapped either — so the address is built from a1, which is the
    // one thing it was given.
    let (mut m, _) = loaded(&[ld(5, 11, 0), ld(6, 5, 0), SPIN], &[BASE]);
    m.run(64, &mut Vec::new());
    assert_eq!(
        (m.csr.scause, m.csr.stval),
        (LOAD_FAULT, BASE),
        "a handle whose number happens to be the supervisor's address is still just a \
         number: U-mode reaches nothing without the U bit"
    );
}

// ---------------------------------------------------------------------------------------
// Claim 3: the program does not learn its load address.

#[test]
fn no_register_at_entry_holds_a_frame_the_loader_chose() {
    let (m, l) = loaded(&[SPIN], &[1, 2, 3]);
    let pool = FREE..l.free;
    for (n, v) in m.x.iter().enumerate() {
        assert!(
            !pool.contains(v),
            "x{n} = {v:#x} is inside the frames the loader took ({:#x}..{:#x}) — A1 says \
             the program does not know its own load address",
            pool.start,
            pool.end
        );
    }
    assert!(
        l.free > FREE,
        "the loader took frames from the pool it was given"
    );
    assert!(
        APP < FREE || APP >= l.free,
        "and the fixture's virtual address is not one of them"
    );
}

// ---------------------------------------------------------------------------------------
// The refusals. Each is a load that cannot be honestly completed.

#[test]
fn the_loader_refuses_what_it_cannot_honestly_place() {
    // Linked into the supervisor's own gigabyte. Mapping it would either overwrite the
    // supervisor's leaf or hand the program the supervisor's pages; both are wrong
    // answers, so it is refused by address.
    let mut m = supervisor();
    let e = load_application(&mut m, &image(&[SPIN], BASE), FREE, &[]).unwrap_err();
    assert!(e.contains("gigabyte"), "must name the collision: {e}");

    // More handles than the one page holds.
    let too_many = vec![0u64; MAX_HANDLES + 1];
    let mut m = supervisor();
    let e = load_application(&mut m, &image(&[SPIN], APP), FREE, &too_many).unwrap_err();
    assert!(
        e.contains(&MAX_HANDLES.to_string()),
        "must say how many fit: {e}"
    );

    // A pool with no room. A loader that allocated past the end would place the program
    // on top of whatever was there.
    let mut m = supervisor();
    let e =
        load_application(&mut m, &image(&[SPIN], APP), BASE + RAM as u64 - 4096, &[]).unwrap_err();
    assert!(
        e.contains("out of frames"),
        "must refuse rather than run past: {e}"
    );

    // Not an ELF at all — `Program::parse` is `load_elf`'s own header check, and this is
    // the assertion that the application loader makes the same ones rather than fewer.
    let mut m = supervisor();
    assert!(load_application(&mut m, b"not an elf", FREE, &[]).is_err());
}
