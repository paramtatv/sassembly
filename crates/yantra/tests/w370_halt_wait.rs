//! **`W-370`: A STORE TO `WAIT` HALTS THE MACHINE, AND `run` AGAIN RESUMES IT.**
//!
//! ADR-0040 Option C. A program that needs the world stores to one MMIO address; the
//! machine halts with [`Halt::Wait`] keeping all of its state, and the next
//! [`Machine::run`] continues at the instruction after the store. From the program's side
//! the wait is ONE retired instruction, which is the whole determinism argument: the count
//! is a function of the program and what it was handed, never of how long the host took.
//!
//! The four falsifiers, each written to fail against a machine without the arm:
//!
//! - (a) wait, wait, finish across three `run` calls, the state intact between them;
//! - (b) the total count over those three runs EQUALS the count of the same program with
//!   each wait store replaced by an ordinary store to RAM;
//! - (c) lives in `crates/yantra-wasm` (`every_halt_has_its_own_yantra_run_code`), because
//!   the code mapping is private to that crate;
//! - (d) `yantra-run` on an image that waits exits non-zero, with its own code, and says
//!   WAIT — run as the real binary, not a function standing in for it.

use std::process::Command;

use sadhana::kosha;
use yantra::supervisor::{Ended, Supervisor};
use yantra::{FINISHER, Halt, Machine, Privilege, WAIT};

const BASE: u64 = 0x8000_0000;

/// A machine with `text` at `BASE` and nothing else — `interpreter.rs`'s helper.
fn machine(text: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX,
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
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    for (n, w) in text.iter().enumerate() {
        m.mem[n * 4..n * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// Encoders, written out rather than taken from `sadhana::encode` for the reason
// `interpreter.rs` gives: a test that derived its words from the encoder it checks
// would agree with it while disagreeing with the ISA.
fn r(op: u32, rd: u32, f3: u32, rs1: u32, rs2: u32, f7: u32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | f7 << 25
}
fn i(op: u32, rd: u32, f3: u32, rs1: u32, imm: i32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn s(op: u32, f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    op | (imm & 0x1f) << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | (imm >> 5 & 0x7f) << 25
}
fn u(op: u32, rd: u32, imm: u32) -> u32 {
    op | rd << 7 | (imm & 0xffff_f000)
}
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i(0x13, rd, 0, rs1, imm)
}
fn sd(rs1: u32, rs2: u32, imm: i32) -> u32 {
    s(0x23, 3, rs1, rs2, imm)
}

/// `x5 = WAIT`, two instructions. Asserted rather than assumed, so a move of the
/// constant reds here instead of silently aiming the program somewhere else.
fn load_wait_address() -> [u32; 2] {
    assert_eq!(WAIT, 0x1000_0100, "the encoding below spells this address");
    [u(0x37, 5, 0x1000_0000), addi(5, 5, 0x100)]
}

/// Word index of the two wait stores in [`program`].
const FIRST_WAIT: u64 = 7;
const SECOND_WAIT: u64 = 9;
/// The counter `x6` ends at: three increments plus the 42 read back from RAM.
const ANSWER: u64 = 3 + 42;
/// Instructions the program retires, start to finisher.
const RETIRED: u64 = 17;

/// Counts in `x6` between waits, parks a marker in RAM before the first, reads it back
/// after the second, and writes the success finisher. With `waits` false the two wait
/// stores become stores to RAM scratch words — the same instruction count, which is
/// what falsifier (b) compares against.
fn program(waits: bool) -> Vec<u32> {
    let [lui, add] = load_wait_address();
    let (first, second) = if waits {
        (sd(5, 6, 0), sd(5, 6, 0))
    } else {
        (sd(7, 6, 8), sd(7, 6, 16))
    };
    let text = vec![
        lui,                          // 0  x5 = 0x1000_0000
        add,                          // 1  x5 = WAIT
        u(0x17, 7, 0),                // 2  x7 = pc = BASE + 8
        addi(7, 7, 0x7f8),            // 3  x7 = BASE + 0x800, a RAM scratch block
        addi(6, 6, 1),                // 4  x6 = 1
        addi(9, 0, 42),               // 5  x9 = 42
        sd(7, 9, 0),                  // 6  RAM[x7] = 42
        first,                        // 7  WAIT (or RAM[x7 + 8] = x6)
        addi(6, 6, 1),                // 8  x6 = 2
        second,                       // 9  WAIT (or RAM[x7 + 16] = x6)
        addi(6, 6, 1),                // 10 x6 = 3
        i(0x03, 8, 3, 7, 0),          // 11 x8 = RAM[x7], the marker
        r(0x33, 6, 0, 6, 8, 0),       // 12 x6 = x6 + x8
        u(0x37, 10, FINISHER as u32), // 13 x10 = FINISHER
        u(0x37, 11, 0x5000),          // 14 x11 = 0x5000
        addi(11, 11, 0x555),          // 15 x11 = 0x5555, success
        s(0x23, 2, 10, 11, 0),        // 16 the finisher
    ];
    assert_eq!(text.len() as u64, RETIRED);
    text
}

/// (a) Wait, Wait, Finisher across three `run` calls, each wait naming its own store,
/// the machine standing on the instruction after it, and registers and RAM intact.
#[test]
fn two_waits_then_the_finisher_across_three_runs_with_the_state_kept() {
    let mut m = machine(&program(true));
    let mut out = Vec::new();

    let first = m.run(1_000, &mut out);
    assert_eq!(
        first,
        Halt::Wait {
            pc: BASE + FIRST_WAIT * 4
        },
        "the first run"
    );
    assert_eq!(
        m.pc,
        BASE + (FIRST_WAIT + 1) * 4,
        "standing on the next instruction"
    );
    assert_eq!(m.x[6], 1, "the counter before the first wait");

    let second = m.run(1_000, &mut out);
    assert_eq!(
        second,
        Halt::Wait {
            pc: BASE + SECOND_WAIT * 4
        },
        "the second run"
    );
    assert_eq!(m.pc, BASE + (SECOND_WAIT + 1) * 4);
    assert_eq!(
        m.x[6], 2,
        "one increment between the waits, not zero and not two"
    );

    let third = m.run(1_000, &mut out);
    assert_eq!(
        third,
        Halt::Finisher {
            value: 0x5555,
            status: Some(0)
        },
        "the third run"
    );
    assert_eq!(
        m.x[6], ANSWER,
        "the counter, and the marker read back from RAM"
    );
    assert_eq!(
        m.x[5], WAIT,
        "a register untouched since before the first wait"
    );
    assert_eq!(
        u64::from_le_bytes(m.mem[0x800..0x808].try_into().unwrap()),
        42,
        "the octets written before the first wait"
    );
    assert!(out.is_empty(), "a wait is not a character");
}

/// (b) The three runs together retire exactly what the same program retires with each
/// wait store replaced by a store to RAM: the wait is one instruction, and the host's
/// time between runs is outside the count.
#[test]
fn the_waits_cost_one_instruction_each_and_the_count_accumulates() {
    let mut reference = machine(&program(false));
    let mut out = Vec::new();
    let halt = reference.run(1_000, &mut out);
    assert!(
        matches!(
            halt,
            Halt::Finisher {
                status: Some(0),
                ..
            }
        ),
        "the control program must finish without waiting: {halt:?}"
    );
    assert_eq!(
        reference.time, RETIRED,
        "the control retires every instruction once"
    );
    assert_eq!(reference.x[6], ANSWER);

    let mut m = machine(&program(true));
    let mut runs = 0;
    loop {
        runs += 1;
        match m.run(1_000, &mut out) {
            Halt::Wait { .. } => continue,
            Halt::Finisher {
                status: Some(0), ..
            } => break,
            other => panic!("run {runs} ended {other:?}"),
        }
    }
    assert_eq!(runs, 3);
    assert_eq!(
        m.time, reference.time,
        "total over three runs against the one-run control"
    );
}

/// A wait is a STORE. A load from the address is not an event the host can answer, and it
/// is refused as the unmapped address it is rather than answered with a zero.
#[test]
fn a_load_from_the_wait_address_is_refused_not_answered() {
    let [lui, add] = load_wait_address();
    let mut m = machine(&[lui, add, i(0x03, 8, 3, 5, 0)]);
    let halt = m.run(10, &mut Vec::new());
    assert_eq!(
        halt,
        Halt::BadAccess {
            pc: BASE + 8,
            addr: WAIT
        }
    );
}

/// The supervisor has no event source, so it must not resume a wait: it reports it, by
/// name, as the machine stopping. Reached here from S-mode, the only mode that can
/// address the device — an application's page tables do not map it.
#[test]
fn the_supervisor_reports_a_wait_as_the_machine_stopping() {
    let mut m = machine(&program(true));
    let ended = Supervisor::new(Vec::new()).run(&mut m, 1_000, &mut Vec::new());
    assert_eq!(
        ended,
        Ended::Stopped(Halt::Wait {
            pc: BASE + FIRST_WAIT * 4
        })
    );
}

/// (d) The real `yantra-run`, on an image whose first act is to wait and whose second is
/// the SUCCESS finisher. A runner that ignored the wait, or resumed it with nothing to
/// deliver, would reach that finisher and exit 0 — so the zero is the wrong answer this
/// refuses, and the code is WAIT's own, not the 1 every other failure uses.
#[test]
fn yantra_run_exits_with_the_wait_code_and_says_wait() {
    let [lui, add] = load_wait_address();
    let words = [
        lui,
        add,
        sd(5, 0, 0),
        u(0x37, 10, FINISHER as u32),
        u(0x37, 11, 0x5000),
        addi(11, 11, 0x555),
        s(0x23, 2, 10, 11, 0),
    ];
    let text: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let dir = std::env::temp_dir().join(format!(
        "w370-wait-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let image = dir.join("waits.elf");
    std::fs::write(&image, kosha::write(&text)).expect("write the image");

    let out = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .arg(&image)
        .current_dir(&dir)
        .output()
        .expect("yantra-run runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(!out.status.success(), "a wait is not a success:\n{stderr}");
    assert_eq!(
        out.status.code(),
        Some(75),
        "WAIT's own exit code:\n{stderr}"
    );
    assert!(
        stderr.lines().any(|l| l.starts_with("yantra-run: WAIT")),
        "a line naming WAIT:\n{stderr}"
    );
    assert!(out.stdout.is_empty(), "nothing reached the payload");
}
