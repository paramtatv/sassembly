//! `F-001e2b` — the supervisor that answers the two calls, and the acceptance for
//! ADR-0015's A4 and A5.
//!
//! # What was already true, and what is new here
//!
//! `F-001e1` proved that no number in `spec/application-abi.tsv` escapes U-mode to the
//! firmware, and said in its own header that *nobody answers call 0 or call 1*.
//! `F-001e2a` entered a program at its `e_entry` in U-mode and stopped at its first
//! `ecall`. This file is the other side of that `ecall`: something reads `a7` and answers.
//!
//! # The two clauses, and the shape of each acceptance
//!
//! - **A5 — the exit ends the program and the environment survives to say so.** The test
//!   is not that a function returned [`Ended::Exited`]. It is that the *same*
//!   [`Machine`] then loads and runs a **second application** to its own exit
//!   ([`call_zero_ends_the_program_and_the_machine_runs_the_next_one`]). ADR-0015's
//!   one-line test read backwards: if the machine had stopped, the program was the
//!   machine.
//! - **A4 — it writes to a surface it was handed, never to a device.** The program in
//!   [`call_one_puts_the_programs_own_bytes_on_the_surface_it_was_handed`] carries its
//!   string in its own text, finds it with `auipc` — so it does not learn its load
//!   address — and the bytes arrive on the surface while the machine's UART stays empty.
//!
//! # The sharp one: a pointer into the supervisor
//!
//! Every application's page table contains the supervisor's gigapage, because a trap has
//! to be able to fetch its own handler. It carries no `U`. A supervisor that read the
//! program's buffer with `sstatus.SUM` set would be able to read that gigapage, and a
//! program that passed a pointer into it would have its own supervisor copy the
//! supervisor onto a surface for it. [`the_four_failures_are_the_ones_the_table_names`]
//! passes exactly that pointer and requires `रिक्तप्रवेशः`.
//!
//! # The fixtures are still not applications
//!
//! `F-001e3` writes the smallest honest application, as a `.sas` file of its own. These
//! are a handful of hand-encoded RV64 words in a `kosha` ELF — enough to prove the calls
//! are answered, deliberately not enough to be mistaken for an app, and not in `spec/`.

use sadhana::kosha::write_debuggable_at;
use std::path::{Path, PathBuf};
use yantra::loader::{Loaded, load_application};
use yantra::supervisor::{
    ADHIKARABHAVAH, ASAMARTHITAM, EXIT, Ended, RIKTAPRAVESHAH, SIDDHAM, SIMATIKRAMAH, Supervisor,
    Surface, WRITE, install,
};
use yantra::{Csrs, Halt, Machine, Privilege};

/// Physical RAM, and the supervisor's own address.
const BASE: u64 = 0x8000_0000;
/// 4 MiB — the loader takes about a dozen frames and the rest is slack.
const RAM: usize = 1 << 22;
/// Where the loader may start taking frames.
const FREE: u64 = BASE + 0x1_0000;
/// Where the applications are linked. Any gigabyte but the supervisor's would do.
const APP: u64 = 0x2000_0000;
/// Where the supervisor's one instruction sits — and it is `stvec` as well as the entry.
const SUPERVISOR_TEXT: u64 = BASE;

const ECALL: u32 = 0x0000_0073;
/// `jal x0, .` — [`Halt::SpinForever`], and how a fixture parks so the test can read the
/// registers the call returned in.
const SPIN: u32 = 0x0000_006f;
/// `sstatus.SPP`, bit 8.
const SPP: u64 = 1 << 8;
/// `scause` 12 — an instruction fetch that could not be translated.
const FETCH_FAULT: u64 = 12;
/// Enough steps for any fixture here; none is longer than a dozen instructions.
const BUDGET: u64 = 200;

/// The handle numbers the fixtures use. Nothing here depends on their values, which is
/// the point: they are the supervisor's to choose and the program's only way to name a
/// surface.
const SCREEN: u64 = 7;
const CARVED_STONE: u64 = 9;

/// What the writing fixture puts on the surface. Multi-byte on purpose: the ABI moves
/// octets and the joining is the host's business (`yantra::Output`).
const STRING: &[u8] = "नमस्ते".as_bytes();

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

// --- encoders, because a dozen instructions do not need an assembler --------------------

fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    0x13 | rd << 7 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
/// `auipc rd, 0` — the program asking where it is without being told.
fn auipc(rd: u32) -> u32 {
    0x17 | rd << 7
}

/// An ELF from the project's own writer holding `words` and then `tail` bytes, linked at
/// [`APP`].
fn image(words: &[u32], tail: &[u8]) -> Vec<u8> {
    let mut text: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    text.extend_from_slice(tail);
    write_debuggable_at(&text, &[], &[], 0, &[], APP)
}

/// A machine with the supervisor's one instruction installed, `SPP` set — so a loader
/// that forgets to clear it returns to S-mode and the test says so — and nothing else.
fn machine() -> Machine {
    let mut m = Machine {
        x: [0; 32],
        pc: 0,
        base: BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    m.csr.sstatus = SPP;
    install(&mut m, SUPERVISOR_TEXT).expect("the supervisor's word is inside RAM");
    m
}

/// Load `words` (plus `tail` bytes of data) as an application with `sup`'s handles
/// granted, and leave the machine standing on the supervisor's `sret`.
fn load(m: &mut Machine, sup: &Supervisor, words: &[u32], tail: &[u8]) -> Loaded {
    let elf = image(words, tail);
    load_application(m, &elf, FREE, &sup.handles()).expect("the fixture must load")
}

/// The instructions of `exit(status)`.
fn exiting(status: i32) -> Vec<u32> {
    vec![
        addi(17, 0, EXIT as i32),
        addi(10, 0, status),
        ECALL,
        SPIN, // never reached: call 0 does not return
    ]
}

/// A program that writes the string it carries to `handle` twice and then exits, with the
/// string placed immediately after its last instruction.
///
/// **It reloads `a0` and `a1` before the second call and nothing else**, which is the
/// ABI's one guarantee spent rather than asserted: `a2` still holds the length and `a7`
/// still holds the call number because the environment may not touch them. If that were
/// false this program would write some other length to some other handle, and the surface
/// would say so.
///
/// It finds the string with `auipc`: A1 says an application does not know its load
/// address, and a fixture that reached its own data through an absolute constant would be
/// quietly assuming it did.
fn writing_twice(handle: i32, length: i32) -> Vec<u32> {
    let mut words = vec![
        auipc(11),       // 0: a1 = the address of this instruction
        addi(11, 11, 0), // 1: patched below to reach the string
        addi(10, 0, handle),
        addi(12, 0, length),
        addi(17, 0, WRITE as i32),
        ECALL,
        auipc(11),       // 6: a1 came back holding the count, so the pointer is recomputed
        addi(11, 11, 0), // 7: patched below
        addi(10, 0, handle), // a0 came back holding सिद्धम्
        ECALL,
    ];
    words.extend_from_slice(&exiting(0));
    // The string follows the last word; each `addi` is measured from its own `auipc`.
    let string = 4 * words.len() as i32;
    words[1] = addi(11, 11, string);
    words[7] = addi(11, 11, string - 4 * 6);
    words
}

// ---------------------------------------------------------------------------------------
// A5 — the exit ends the program, not the machine.

#[test]
fn call_zero_ends_the_program_and_the_machine_runs_the_next_one() {
    // ADR-0015 A5, and the acceptance the backlog row names: `yantra` keeps running and
    // says so. Nothing below asserts on a message — the machine says it by running a
    // second application in the same RAM, from the same supervisor instruction, after the
    // first one has ended.
    let mut m = machine();
    let mut sup = Supervisor::default();
    let mut uart: Vec<u8> = Vec::new();

    load(&mut m, &sup, &exiting(3), &[]);
    assert_eq!(
        sup.run(&mut m, BUDGET, &mut uart),
        Ended::Exited { status: 3 },
        "call 0 ends the program with the status it asked for"
    );
    assert_eq!(
        m.mode,
        Privilege::Supervisor,
        "and leaves the hart in S-mode, standing on the supervisor's own word"
    );
    assert_eq!(
        m.pc, SUPERVISOR_TEXT,
        "which is `stvec`, where the trap put it"
    );

    // The second program. Same machine, same frame pool — the first one's pages are the
    // supervisor's to hand out again, which is only true because nothing stopped.
    load(&mut m, &sup, &exiting(11), &[]);
    assert_eq!(
        sup.run(&mut m, BUDGET, &mut uart),
        Ended::Exited { status: 11 },
        "the environment survived its guest, and proves it by hosting another"
    );
    assert!(uart.is_empty(), "no application reached a device (A4)");
}

#[test]
fn a_fault_from_user_mode_ends_the_program_and_the_machine_survives_that_too() {
    // `spec/application-entry.tsv` leaves `ra` zero so that a program which returns from
    // `e_entry` faults instead of leaving by a path the loader happened to open
    // (`tests/application_abi.rs`). Here is who is told: the supervisor has no policy for
    // a fetch at 0, so it ends the program and names the cause rather than resuming into
    // the same fault forever.
    let mut m = machine();
    let mut sup = Supervisor::default();
    let mut uart: Vec<u8> = Vec::new();

    load(&mut m, &sup, &[0x0000_8067], &[]); // jalr x0, 0(ra), and ra is zero
    let ended = sup.run(&mut m, BUDGET, &mut uart);
    assert_eq!(
        ended,
        Ended::Faulted {
            cause: FETCH_FAULT,
            tval: 0,
            epc: 0,
        },
        "the fetch at 0 is reported to the supervisor, never a silent success"
    );

    load(&mut m, &sup, &exiting(0), &[]);
    assert_eq!(
        sup.run(&mut m, BUDGET, &mut uart),
        Ended::Exited { status: 0 },
        "a program that faulted did not take the machine with it"
    );
}

// ---------------------------------------------------------------------------------------
// A4 — it writes to a surface it was handed.

#[test]
fn call_one_puts_the_programs_own_bytes_on_the_surface_it_was_handed() {
    let mut m = machine();
    let mut sup = Supervisor::new(vec![Surface::writable(SCREEN)]);
    let mut uart: Vec<u8> = Vec::new();

    // Write the string, then write it again, then exit. The second write is what proves
    // the supervisor resumed the program at the instruction *after* its `ecall`: a `sepc`
    // left where the trap put it would run the first `ecall` forever.
    let words = writing_twice(SCREEN as i32, STRING.len() as i32);
    load(&mut m, &sup, &words, STRING);
    assert_eq!(
        sup.run(&mut m, BUDGET, &mut uart),
        Ended::Exited { status: 0 },
        "it wrote twice and then asked to be ended"
    );
    let screen = sup.surface(SCREEN).expect("the surface it was handed");
    assert_eq!(
        screen.bytes,
        [STRING, STRING].concat(),
        "both writes landed, in order, on the surface"
    );
    assert!(
        uart.is_empty(),
        "and none of it reached the UART: an application has no device (A4)"
    );
    assert_eq!(
        m.x[11],
        STRING.len() as u64,
        "a1 is the count of bytes written"
    );
}

#[test]
fn a_write_the_supervisor_refused_puts_nothing_on_the_surface() {
    // All of the bytes or none: the buffer is translated in full before the first byte is
    // copied. A half-written surface with an error code the program cannot act on is the
    // failure mode this is here to exclude.
    let mut sup = Supervisor::new(vec![Surface::writable(SCREEN)]);
    let (m, _) = call(&mut sup, &[SCREEN, 0, 8]);
    assert_eq!(m.x[10], RIKTAPRAVESHAH as u64, "the null buffer is refused");
    assert!(
        sup.surface(SCREEN).expect("granted").bytes.is_empty(),
        "and the surface never saw a byte of it"
    );
}

// ---------------------------------------------------------------------------------------
// The failures the table names.

/// Run `ecall` with `a0`, `a1`, `a2` set from `args` and `a7` from [`WRITE`], and park.
///
/// The registers are placed directly rather than assembled, because the interesting
/// arguments are addresses no application could name — the supervisor's own gigapage,
/// among them — and hand-encoding a `lui` for each would test the encoder.
fn call(sup: &mut Supervisor, args: &[u64; 3]) -> (Machine, Ended) {
    call_with(sup, WRITE, args)
}

fn call_with(sup: &mut Supervisor, a7: u64, args: &[u64; 3]) -> (Machine, Ended) {
    let mut m = machine();
    load(&mut m, sup, &[ECALL, SPIN], &[]);
    m.x[10] = args[0];
    m.x[11] = args[1];
    m.x[12] = args[2];
    m.x[17] = a7;
    let ended = sup.run(&mut m, BUDGET, &mut Vec::new());
    (m, ended)
}

#[test]
fn the_four_failures_are_the_ones_the_table_names() {
    // The four codes of `spec/application-abi.tsv`, each produced by the one thing that is
    // wrong with the call — so a supervisor that collapsed them into a single "no" fails
    // here rather than in a program that cannot tell why it was refused.
    let handles = vec![Surface::writable(SCREEN), Surface::read_only(CARVED_STONE)];

    let mut sup = Supervisor::new(handles.clone());
    assert_eq!(
        call(&mut sup, &[SCREEN + CARVED_STONE, 0, 0]).0.x[10],
        SIMATIKRAMAH as u64,
        "a handle number nobody granted is सीमातिक्रमः"
    );

    let mut sup = Supervisor::new(handles.clone());
    assert_eq!(
        call(&mut sup, &[CARVED_STONE, 0, 0]).0.x[10],
        ADHIKARABHAVAH as u64,
        "a handle granted without आलेखनम् is अधिकाराभावः, not 'no such handle'"
    );

    let mut sup = Supervisor::new(handles.clone());
    assert_eq!(
        call(&mut sup, &[SCREEN, 0x4000_0000, 16]).0.x[10],
        RIKTAPRAVESHAH as u64,
        "a buffer in a page the program has not got is रिक्तप्रवेशः"
    );

    // THE SHARP ONE. `BASE` is the supervisor's own RAM, identity-mapped into this
    // program's address space without `U` because the trap must be able to fetch its
    // handler. With `sstatus.SUM` set the supervisor could read it, and this call would
    // copy the supervisor onto a surface, 4 KiB at a time.
    let mut sup = Supervisor::new(handles.clone());
    let (m, _) = call(&mut sup, &[SCREEN, BASE, 16]);
    assert_eq!(
        m.x[10], RIKTAPRAVESHAH as u64,
        "a pointer into the supervisor's own gigapage is refused: the buffer is read as \
         the program, and the program has no `U` there"
    );
    assert!(
        sup.surface(SCREEN).expect("granted").bytes.is_empty(),
        "not one byte of the supervisor reached the surface"
    );

    // The positive control that keeps the three refusals above from being "everything
    // fails": a zero-length write to a granted surface touches no memory, so its null
    // pointer is not an error.
    let mut sup = Supervisor::new(handles);
    let (m, _) = call(&mut sup, &[SCREEN, 0, 0]);
    assert_eq!(m.x[10], SIDDHAM as u64, "nothing to write is not a failure");
    assert_eq!(m.x[11], 0, "and nothing is what it wrote");
}

#[test]
fn every_number_outside_the_table_is_asamarthitam_and_the_program_runs_on() {
    // Including 8 — `sbi_shutdown`, which from S-mode stops this machine dead
    // (`tests/application_abi.rs`). From U-mode it is a call number the table does not
    // assign, and the answer is a sentence rather than a shutdown.
    for a7 in [2u64, 8, 63, 64, u64::MAX] {
        let mut sup = Supervisor::new(vec![Surface::writable(SCREEN)]);
        let (m, ended) = call_with(&mut sup, a7, &[SCREEN, 0, 0]);
        assert_eq!(
            m.x[10], ASAMARTHITAM as u64,
            "call {a7} is not in the table and is told so"
        );
        assert!(
            matches!(ended, Ended::Stopped(Halt::SpinForever { .. })),
            "call {a7} returned to the program, which parked — the machine never stopped \
             on the call itself"
        );
    }
}

#[test]
fn a0_and_a1_are_the_only_registers_the_environment_clobbers() {
    // `spec/application-abi.tsv`: "`a0` and `a1` are the ONLY registers the environment
    // may clobber; everything else is preserved across the call, which is SBI's own rule
    // and the one a caller can actually rely on." A supervisor that spilled into `s2`
    // would be one no program could call twice.
    let mut sup = Supervisor::new(vec![Surface::writable(SCREEN)]);
    let mut m = machine();
    load(&mut m, &sup, &[ECALL, SPIN], &[]);
    // x0 is hardwired and is not the machine's to keep; every other register gets a value
    // nothing in the call has a reason to produce.
    for (i, x) in m.x.iter_mut().enumerate().skip(1) {
        *x = 0xA5A5_0000 | i as u64;
    }
    // The four the call itself uses. They are compared against what they were *set* to
    // rather than skipped: `a2` and `a7` are arguments and are still guaranteed.
    m.x[10] = SCREEN;
    m.x[11] = 0;
    m.x[12] = 0;
    m.x[17] = WRITE;
    let before = m.x;
    let ended = sup.run(&mut m, BUDGET, &mut Vec::new());
    assert!(matches!(ended, Ended::Stopped(Halt::SpinForever { .. })));
    for (i, x) in m.x.iter().enumerate() {
        // x0 is hardwired: it is not the environment's to preserve and not the
        // program's to have set. Every other register is compared to what it held.
        if i == 0 || i == 10 || i == 11 {
            continue;
        }
        assert_eq!(
            *x, before[i],
            "x{i} survived the call, or the ABI's one guarantee is not one"
        );
    }
    assert_eq!(
        m.x[17], WRITE,
        "a7 too: the call number the caller wrote is still there afterwards"
    );
}

// ---------------------------------------------------------------------------------------
// The codes are the spec file's.

#[test]
fn the_error_codes_are_the_numbers_the_spec_file_states() {
    // ADR-0016 states the five codes in `spec/application-abi.tsv`'s header, beside the
    // words they are. They are read from there rather than restated here, so a code that
    // changes in the file changes what this supervisor is held to — and a supervisor that
    // answered `-1` where the file says `-4` fails here rather than in a program that
    // cannot tell "no such handle" from "no such call".
    let text = std::fs::read_to_string(root().join("spec/application-abi.tsv"))
        .expect("spec/application-abi.tsv");
    let stated: Vec<(i64, String)> = text
        .lines()
        .filter_map(|l| l.strip_prefix('#'))
        .filter_map(|l| {
            let mut fields = l.split_whitespace();
            let code: i64 = fields.next()?.parse().ok()?;
            let name = fields.next()?;
            // A code line is a number and then the word it is. Prose in this header also
            // begins with a number — "8 in `a7` does not stop the machine" — and it is
            // ASCII, which the five words are not.
            (!name.is_ascii()).then_some(())?;
            Some((code, name.to_string()))
        })
        .collect();
    assert_eq!(
        stated.len(),
        5,
        "five codes are stated in the file: {stated:?}"
    );
    let ours = [
        (SIDDHAM, "सिद्धम्"),
        (ASAMARTHITAM, "असमर्थितम्"),
        (ADHIKARABHAVAH, "अधिकाराभावः"),
        (RIKTAPRAVESHAH, "रिक्तप्रवेशः"),
        (SIMATIKRAMAH, "सीमातिक्रमः"),
    ];
    for (code, name) in ours {
        assert!(
            stated.contains(&(code, name.to_string())),
            "the file does not state {code} as {name}; it states {stated:?}"
        );
    }
}

#[test]
fn the_call_numbers_are_the_rows_of_the_table() {
    // Two calls, and this supervisor answers exactly those two. A row added to the table
    // fails here until somebody answers it, which is the point: the table is the ABI.
    let text = std::fs::read_to_string(root().join("spec/application-abi.tsv"))
        .expect("spec/application-abi.tsv");
    let numbers: Vec<u64> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .skip(1)
        .filter_map(|l| l.split('\t').next()?.parse().ok())
        .collect();
    assert_eq!(
        numbers,
        vec![EXIT, WRITE],
        "the supervisor answers the table's numbers and no others"
    );
}
