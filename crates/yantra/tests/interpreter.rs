//! What the interpreter must do, instruction by instruction — task `F-001` spike.
//!
//! # Why these are here at all
//!
//! The spike was demonstrated through `yantra-run` and a node harness, both of which are
//! things a person runs and looks at. Neither is in the gate, so neither would notice a
//! regression. **An interpreter nobody tests is the one thing in this demo that could
//! quietly start producing a plausible wrong answer** — the exact failure the whole VM is
//! written to avoid, arriving through the back door.
//!
//! # The two levels, and why the second one matters more than it looks
//!
//! 1. **Semantics.** Hand-encoded instruction words, run one step, assert the effect.
//!    Every one of the eight opcodes the demo programs use, plus the arms that halt.
//! 2. **Against the project's own ELF writer.** `sadhana::kosha` is what produces the
//!    artefact QEMU runs, so the loader is tested against *that* rather than against an
//!    ELF this file invents. If `kosha`'s output moves, these fail — which is the whole
//!    point, because the demo's claim is that one artefact runs in both places. A loader
//!    validated against a hand-rolled fixture would keep passing while the claim broke.

use sadhana::kosha;
use yantra::{FINISHER, Halt, Machine, Privilege, UART};

/// A machine with `text` at `0x8000_0000` and nothing else. Registers zeroed.
fn machine(text: &[u32]) -> Machine {
    let mut m = Machine {
        x: [0; 32],
        pc: 0x8000_0000,
        base: 0x8000_0000,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    for (i, w) in text.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// Encoders. Written out rather than pulled from `sadhana::encode` ON PURPOSE: if both the
// interpreter and its test derived the encoding from one source, the pair would agree with
// each other while disagreeing with the ISA, and the test would be a mirror.
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
fn b(f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    0x63 | (imm >> 11 & 1) << 7
        | (imm >> 1 & 0xf) << 8
        | f3 << 12
        | rs1 << 15
        | rs2 << 20
        | (imm >> 5 & 0x3f) << 25
        | (imm >> 12 & 1) << 31
}
fn u(op: u32, rd: u32, imm: u32) -> u32 {
    op | rd << 7 | (imm & 0xffff_f000)
}
fn j(rd: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    0x6f | rd << 7
        | (imm >> 12 & 0xff) << 12
        | (imm >> 11 & 1) << 20
        | (imm >> 1 & 0x3ff) << 21
        | (imm >> 20 & 1) << 31
}

#[test]
fn lui_and_auipc_place_the_upper_twenty_bits() {
    let mut m = machine(&[u(0x37, 5, 0x1000_0000), u(0x17, 6, 0x0000_1000)]);
    let mut out = Vec::new();
    m.step(&mut out);
    assert_eq!(m.x[5], 0x1000_0000, "LUI");
    m.step(&mut out);
    // AUIPC is pc-relative, and the pc it adds to is its OWN address, not the next one.
    assert_eq!(m.x[6], 0x8000_0004 + 0x1000, "AUIPC");
}

#[test]
fn arithmetic_covers_the_immediate_and_register_forms() {
    let mut out = Vec::new();
    let mut m = machine(&[
        i(0x13, 1, 0x0, 0, 5),       // ADDI x1, x0, 5
        i(0x13, 2, 0x0, 0, -3),      // ADDI x2, x0, -3   (sign extension)
        r(0x33, 3, 0x0, 1, 2, 0x00), // ADD  x3, x1, x2
        r(0x33, 4, 0x0, 1, 2, 0x20), // SUB  x4, x1, x2
    ]);
    for _ in 0..4 {
        m.step(&mut out);
    }
    assert_eq!(m.x[1], 5);
    assert_eq!(m.x[2] as i64, -3, "the I-immediate must be sign-extended");
    assert_eq!(m.x[3], 2, "5 + (-3)");
    assert_eq!(m.x[4], 8, "5 - (-3)");
}

#[test]
fn x0_stays_zero_however_it_is_written() {
    // The register that is a hole. A VM that lets `x0` hold a value computes plausible
    // wrong answers everywhere, because `x0` is how every program says "nothing".
    let mut out = Vec::new();
    let mut m = machine(&[i(0x13, 0, 0x0, 0, 99), r(0x33, 0, 0x0, 0, 0, 0x00)]);
    m.step(&mut out);
    assert_eq!(m.x[0], 0, "ADDI into x0");
    m.step(&mut out);
    assert_eq!(m.x[0], 0, "ADD into x0");
}

#[test]
fn lui_sign_extends_on_rv64_which_is_why_the_test_below_sets_the_address_directly() {
    // Pinning a real ISA property, found by getting it wrong here first. On RV64, LUI
    // produces a SIGN-EXTENDED 32-bit value — so `LUI x1, 0x80001` yields
    // 0xFFFF_FFFF_8000_1000 and not 0x8000_1000. Loading a RAM address at 0x8000_0000
    // with LUI alone therefore does NOT reach RAM, which is why the demo programs reach
    // their data through AUIPC and why `namaste.sas` can use bare LUI for the UART: the
    // UART is at 0x1000_0000, whose top bit is clear.
    let mut out = Vec::new();
    let mut m = machine(&[u(0x37, 1, 0x8000_1000), u(0x37, 2, 0x1000_0000)]);
    m.step(&mut out);
    m.step(&mut out);
    assert_eq!(
        m.x[1], 0xFFFF_FFFF_8000_1000,
        "the top bit set must sign-extend"
    );
    assert_eq!(m.x[2], 0x1000_0000, "the top bit clear must not");
}

#[test]
fn loads_sign_extend_and_stores_round_trip() {
    let mut out = Vec::new();
    // The address is set directly rather than built with LUI — see the test above; LUI
    // cannot express 0x8000_1000 on RV64 in one instruction.
    let mut m = machine(&[
        i(0x13, 2, 0x0, 0, 0xff), // ADDI x2, x0, 255
        s(0x23, 0x0, 1, 2, 0),    // SB  x2, 0(x1)
        i(0x03, 3, 0x0, 1, 0),    // LB  x3, 0(x1)   signed
        i(0x03, 4, 0x4, 1, 0),    // LBU x4, 0(x1)   unsigned
    ]);
    m.x[1] = 0x8000_1000;
    for _ in 0..4 {
        assert_eq!(m.step(&mut out), None, "no step here may halt");
    }
    assert_eq!(m.x[3] as i64, -1, "LB must sign-extend 0xFF to -1");
    assert_eq!(m.x[4], 255, "LBU must not");
}

#[test]
fn branches_take_and_fall_through_on_the_right_condition() {
    let cases: [(u32, u64, u64, bool); 4] = [
        (0x0, 7, 7, true),                     // BEQ equal
        (0x0, 7, 8, false),                    // BEQ unequal
        (0x1, 7, 8, true),                     // BNE unequal
        (0x4, 0xffff_ffff_ffff_ffff, 1, true), // BLT is SIGNED: -1 < 1
    ];
    for (f3, a, bb, taken) in cases {
        let mut out = Vec::new();
        let mut m = machine(&[b(f3, 1, 2, 16)]);
        m.x[1] = a;
        m.x[2] = bb;
        m.step(&mut out);
        let want = if taken { 0x8000_0010 } else { 0x8000_0004 };
        assert_eq!(m.pc, want, "f3={f3:#x} a={a:#x} b={bb:#x}");
    }
}

#[test]
fn jal_links_the_return_address_and_a_self_jump_is_reported() {
    let mut out = Vec::new();
    let mut m = machine(&[j(1, 8)]);
    assert!(m.step(&mut out).is_none());
    assert_eq!(m.x[1], 0x8000_0004, "JAL must link pc+4");
    assert_eq!(m.pc, 0x8000_0008);

    // `jal x0, .` is how both demo programs park. Reporting it beats burning the budget
    // to reach the same conclusion, and it must be a HALT rather than a hang.
    let mut m = machine(&[j(0, 0)]);
    assert_eq!(
        m.step(&mut out),
        Some(Halt::SpinForever { pc: 0x8000_0000 })
    );
}

#[test]
fn a_store_to_the_uart_becomes_output_and_never_reaches_ram() {
    // THE BRIDGE. This one assertion is the whole demo: the same instruction that drives
    // a UART on bare metal must put a character on the page instead.
    let mut out: Vec<u8> = Vec::new();
    let mut m = machine(&[
        u(0x37, 1, UART as u32), // LUI x1, 0x10000
        i(0x13, 2, 0x0, 0, b'A' as i32),
        s(0x23, 0x0, 1, 2, 0), // SB x2, 0(x1)
    ]);
    for _ in 0..3 {
        m.step(&mut out);
    }
    assert_eq!(out, b"A", "the store must be routed to the output");
    // And it must NOT have been written into RAM as well — a device address that also
    // lands in memory is a bug that only shows up when something later reads it back.
    assert!(
        m.mem.iter().all(|&b| b != b'A'),
        "UART write leaked into RAM"
    );
}

#[test]
fn the_finisher_decodes_success_and_failure() {
    for (value, status) in [(0x5555u32, Some(0)), (0x0003_3333, Some(3))] {
        let mut out = Vec::new();
        let mut m = machine(&[
            u(0x37, 1, FINISHER as u32),
            u(0x37, 2, value & 0xffff_f000),
            i(0x13, 2, 0x0, 2, (value & 0xfff) as i32),
            s(0x23, 0x2, 1, 2, 0), // SW
        ]);
        let mut halt = None;
        for _ in 0..4 {
            if let Some(h) = m.step(&mut out) {
                halt = Some(h);
            }
        }
        assert_eq!(
            halt,
            Some(Halt::Finisher { value, status }),
            "value {value:#x}"
        );
    }
}

#[test]
fn an_unknown_instruction_stops_and_names_itself() {
    // The property that separates this from a VM that quietly produces a wrong answer.
    // `fadd.d` — 0x0200_0053 — is the D extension, which this machine does not have;
    // running it as anything would put a number in `rd` that no addition produced.
    //
    // It was `0x0000_0073` until `F-001c1` made that word an `ecall`, `0x1020_0073` until
    // `F-001c2a` made that one an `sret`, and `0x1200_0073` until `F-001c2b1` gave
    // `sfence.vma` a meaning. Each time the test kept its point by moving to a word that
    // is still genuinely unimplemented rather than by weakening what it asks — and once it
    // had to leave SYSTEM entirely, because after `F-001c2b1` every 32-bit row of that
    // family executes.
    //
    // 2026-09-05: it moves a fourth time, and for the first time because the machine gained
    // an ARITHMETIC extension rather than a system instruction. It was `0x0200_0033` — `mul`
    // — until the M extension was implemented so that two demonstration programs could
    // multiply and take a remainder. `0x33` is now a poor choice for this test at any
    // funct7, since the opcode is densely populated and the next extension to land there
    // would move it again; `0x53` has no arm AT ALL, so the whole floating-point family
    // halts and this word stays genuinely unimplemented for as long as that is true.
    // If it ever moves again, move it — do not weaken what it asks.
    let mut out = Vec::new();
    let mut m = machine(&[0x0200_0053]);
    let halt = m.step(&mut out).expect("must halt");
    assert_eq!(
        halt,
        Halt::Unimplemented {
            pc: 0x8000_0000,
            word: 0x0200_0053,
            opcode: 0x53
        }
    );
}

#[test]
fn a_runaway_program_stops_instead_of_wedging_the_tab() {
    let mut out = Vec::new();
    // ADDI x1, x1, 1 forever — no self-jump, so SpinForever cannot catch it.
    let mut m = machine(&[i(0x13, 1, 0x0, 1, 1), j(0, -4)]);
    assert!(matches!(m.run(100, &mut out), Halt::StepLimit { .. }));
}

#[test]
fn the_loader_refuses_what_it_cannot_honestly_run() {
    // Each of these is a wrong ANSWER if accepted, not merely an error, so each is checked.
    let good = kosha::write_with_data(&[0x13, 0x00, 0x00, 0x00], &[]);
    assert!(
        Machine::load_elf(&good, 1 << 16).is_ok(),
        "a real kosha ELF must load"
    );

    assert!(Machine::load_elf(b"not an elf at all", 1 << 16).is_err());

    let mut wrong_machine = good.clone();
    wrong_machine[18] = 62; // x86-64
    let e = Machine::load_elf(&wrong_machine, 1 << 16).unwrap_err();
    assert!(e.contains("243"), "must name the machine it wanted: {e}");

    // 2 bytes cannot hold the 4-byte segment. This must be a stated refusal rather than
    // a truncated load — a program half in memory runs and produces nonsense.
    let e = Machine::load_elf(&good, 2).unwrap_err();
    assert!(
        e.contains("RAM"),
        "too little RAM must say so, not truncate: {e}"
    );
}
/// A PROGRAM THAT OUTGROWS ITS RAM SAYS SO, rather than naming an address.
///
/// `W-262`, 2026-09-05. Both "the address is below this machine's base" and "the
/// address is past the end of the RAM it was given" used to answer
/// `BadAccess { pc, addr }` — a wild pointer and an exhausted machine, reported
/// identically. The second is not a defect in the program: it is the machine
/// being too small, and reporting it as an address sends the next person into
/// the compiler that emitted the program to look for a bug that is not there.
///
/// THIS IS NOT HYPOTHETICAL. One pass of the emitted lexer over the corpus's
/// largest source needs 1,011 KiB of arenas against the 1,024 KiB `yantra-run`
/// used to give, so `W-254`'s first step would have produced exactly this halt
/// and exactly that misdirection.
#[test]
fn a_program_that_outgrows_its_ram_halts_naming_the_limit_not_the_address() {
    const BASE: u64 = 0x8000_0000;
    let ram = 1 << 16;
    // `ld x5, 0(x6)` — a load whose address this machine does not have.
    let word: u32 = 0x0003_3283;
    let mut m = Machine {
        x: [0; 32],
        pc: BASE,
        base: BASE,
        mem: {
            let mut v = vec![0u8; ram];
            v[..4].copy_from_slice(&word.to_le_bytes());
            v
        },
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: yantra::Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    m.x[6] = BASE + ram as u64 + 8; // eight bytes past the end of this RAM
    let mut out = Vec::new();
    match m.step(&mut out) {
        Some(Halt::BeyondRam { ram: reported, .. }) => assert_eq!(
            reported, ram,
            "the halt must name the RAM that was crossed, or it cannot be told \
             from a wild pointer"
        ),
        other => panic!(
            "a load past the end of RAM must halt `BeyondRam`, naming the limit; \
             got {other:?}"
        ),
    }
}

#[test]
fn an_elf_from_the_projects_own_writer_runs_end_to_end() {
    // The coupling test. `kosha` is what writes the artefact QEMU runs, so the loader is
    // held to THAT format rather than to one this file invented. A hand-rolled fixture
    // would go on passing while the demo's one-artefact claim quietly broke.
    //
    // The program: write "ॐ" (U+0950, three UTF-8 bytes) to the UART, then the finisher.
    let om = "ॐ".as_bytes();
    let mut text: Vec<u32> = vec![u(0x37, 1, UART as u32)];
    for &byte in om {
        text.push(i(0x13, 2, 0x0, 0, i32::from(byte)));
        text.push(s(0x23, 0x0, 1, 2, 0));
    }
    text.push(u(0x37, 3, FINISHER as u32));
    text.push(u(0x37, 4, 0x5000));
    text.push(i(0x13, 4, 0x0, 4, 0x555));
    text.push(s(0x23, 0x2, 3, 4, 0));

    let bytes: Vec<u8> = text.iter().flat_map(|w| w.to_le_bytes()).collect();
    let elf = kosha::write_with_data(&bytes, &[]);

    let mut m = Machine::load_elf(&elf, 1 << 20).expect("kosha's ELF must load");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(1000, &mut out);

    assert_eq!(String::from_utf8_lossy(&out), "ॐ");
    assert_eq!(
        halt,
        Halt::Finisher {
            value: 0x5555,
            status: Some(0)
        }
    );
}

// ---------------------------------------------------------------------------
// The M extension. Added 2026-09-05, after two demonstration programs — one
// that multiplies and one that takes a remainder — halted with
// `Unimplemented { opcode: 51 }`. Opcode 51 is `OP`, and this machine matched
// on `(funct3, funct7)` with arms for funct7 0x00 and 0x20 only; funct7 0x01
// is the whole extension and had no arm at all, so all eight fell through
// together. Everything upstream of those two programs was correct.
// ---------------------------------------------------------------------------

/// `i64::MIN` in `rd`, built from instructions rather than loaded, because the
/// only interesting overflow case in the extension needs it.
fn min_into(rd: u32) -> [u32; 2] {
    [
        i(0x13, rd, 0x0, 0, 1),   // ADDI rd, x0, 1
        i(0x13, rd, 0x1, rd, 63), // SLLI rd, rd, 63  → 0x8000_0000_0000_0000
    ]
}

#[test]
fn multiply_covers_the_low_word_and_all_three_high_words() {
    let mut out = Vec::new();
    let mut m = machine(&[
        i(0x13, 1, 0x0, 0, 6),       // x1 = 6
        i(0x13, 2, 0x0, 0, 7),       // x2 = 7
        i(0x13, 3, 0x0, 0, -1),      // x3 = -1 (all ones)
        r(0x33, 4, 0x0, 1, 2, 0x01), // MUL    x4 = 6 * 7
        r(0x33, 5, 0x1, 3, 3, 0x01), // MULH   x5 = high(-1 * -1)  signed×signed
        r(0x33, 6, 0x3, 3, 3, 0x01), // MULHU  x6 = high(all ones × all ones)
        r(0x33, 7, 0x2, 3, 3, 0x01), // MULHSU x7 = high(-1 × all ones) signed×unsigned
    ]);
    for _ in 0..7 {
        m.step(&mut out);
    }
    assert_eq!(m.x[4], 42, "MUL keeps the low 64 bits");
    assert_eq!(m.x[5], 0, "MULH: (-1) * (-1) is 1, so the high word is 0");
    assert_eq!(
        m.x[6], 0xFFFF_FFFF_FFFF_FFFE,
        "MULHU treats both operands as unsigned, so this is (2^64-1)^2 >> 64"
    );
    assert_eq!(
        m.x[7] as i64, -1,
        "MULHSU is signed times UNSIGNED: -1 * (2^64-1) >> 64 is -1, which is \
         the arm that distinguishes it from both of its neighbours"
    );
}

#[test]
fn divide_and_remainder_agree_with_the_signed_and_unsigned_forms() {
    let mut out = Vec::new();
    let mut m = machine(&[
        i(0x13, 1, 0x0, 0, 20),      // x1 = 20
        i(0x13, 2, 0x0, 0, 6),       // x2 = 6
        i(0x13, 3, 0x0, 0, -20),     // x3 = -20
        r(0x33, 4, 0x4, 1, 2, 0x01), // DIV  20 / 6
        r(0x33, 5, 0x6, 1, 2, 0x01), // REM  20 % 6
        r(0x33, 6, 0x4, 3, 2, 0x01), // DIV  -20 / 6   — truncates toward zero
        r(0x33, 7, 0x6, 3, 2, 0x01), // REM  -20 % 6   — sign follows the dividend
        r(0x33, 8, 0x5, 1, 2, 0x01), // DIVU 20 / 6
        r(0x33, 9, 0x7, 1, 2, 0x01), // REMU 20 % 6
    ]);
    for _ in 0..9 {
        m.step(&mut out);
    }
    assert_eq!(m.x[4], 3, "DIV");
    assert_eq!(m.x[5], 2, "REM");
    assert_eq!(
        m.x[6] as i64, -3,
        "DIV truncates toward zero, not toward -inf"
    );
    assert_eq!(m.x[7] as i64, -2, "REM takes the sign of the DIVIDEND");
    assert_eq!(m.x[8], 3, "DIVU");
    assert_eq!(m.x[9], 2, "REMU");
}

#[test]
fn division_by_zero_returns_a_value_because_riscv_does_not_trap() {
    // The case that makes a host language dangerous here: Rust's `/` and `%`
    // PANIC on a zero divisor, and this machine must instead answer by value.
    // All ones for both quotients, and the dividend unchanged for both
    // remainders — a program that divides by zero keeps running.
    let mut out = Vec::new();
    let mut m = machine(&[
        i(0x13, 1, 0x0, 0, 20),      // x1 = 20, x2 stays 0
        r(0x33, 3, 0x4, 1, 2, 0x01), // DIV  20 / 0
        r(0x33, 4, 0x5, 1, 2, 0x01), // DIVU 20 / 0
        r(0x33, 5, 0x6, 1, 2, 0x01), // REM  20 % 0
        r(0x33, 6, 0x7, 1, 2, 0x01), // REMU 20 % 0
    ]);
    for _ in 0..5 {
        m.step(&mut out);
    }
    assert_eq!(m.x[3] as i64, -1, "DIV by zero is all ones");
    assert_eq!(m.x[4], u64::MAX, "DIVU by zero is all ones");
    assert_eq!(m.x[5], 20, "REM by zero is the dividend, unchanged");
    assert_eq!(m.x[6], 20, "REMU by zero is the dividend, unchanged");
}

#[test]
fn the_one_overflow_case_is_answered_by_value_too() {
    // i64::MIN / -1 has no representable quotient. RISC-V does not trap: DIV
    // yields the dividend and REM yields zero. Rust's `/` panics here in debug
    // and this is the only input pair for which it does apart from a zero
    // divisor, so it is the second place a naive host silently differs.
    let mut out = Vec::new();
    let mut prog = min_into(1).to_vec(); // x1 = i64::MIN
    prog.push(i(0x13, 2, 0x0, 0, -1)); // x2 = -1
    prog.push(r(0x33, 3, 0x4, 1, 2, 0x01)); // DIV
    prog.push(r(0x33, 4, 0x6, 1, 2, 0x01)); // REM
    let steps = prog.len();
    let mut m = machine(&prog);
    for _ in 0..steps {
        m.step(&mut out);
    }
    assert_eq!(m.x[1], i64::MIN as u64, "the fixture itself");
    assert_eq!(m.x[3], i64::MIN as u64, "DIV overflow yields the dividend");
    assert_eq!(m.x[4], 0, "REM overflow yields zero");
}

#[test]
fn an_unclaimed_funct7_still_halts_so_the_new_arms_did_not_widen_the_door() {
    // The extension is funct7 0x01 exactly. Adding eight arms must not make the
    // machine accept encodings it has no meaning for — this pins that the
    // catch-all is still reachable.
    let mut out = Vec::new();
    let mut m = machine(&[r(0x33, 1, 0x0, 0, 0, 0x02)]);
    let halt = m.step(&mut out);
    assert!(
        matches!(halt, Some(yantra::Halt::Unimplemented { opcode: 0x33, .. })),
        "funct7 0x02 is not the M extension and must still halt, got {halt:?}"
    );
}
