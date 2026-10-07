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
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: 0x8000_0000,
        base: 0x8000_0000,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
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
    for (value, status) in [(0x5555u64, Some(0)), (0x0003_3333, Some(3))] {
        let mut out = Vec::new();
        let mut m = machine(&[
            u(0x37, 1, FINISHER as u32),
            u(0x37, 2, (value as u32) & 0xffff_f000),
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

/// `W-341` — THE STATUS FIELD IS WIDER THAN SIXTEEN BITS. Error 70000 is
/// written as `0x3333 | (70000 << 16)` = `0x1_1170_3333`, a 33-bit word; the
/// arm that read it through `as u32` and `>> 16` answered **4464**, exactly
/// the figure a peer session measured. The green path (`0x5555` → `Some(0)`)
/// could never show this, so only failure statuses lost information.
#[test]
fn the_finisher_carries_a_status_wider_than_sixteen_bits() {
    let mut out = Vec::new();
    let mut m = machine(&[
        u(0x37, 1, FINISHER as u32), // LUI x1, finisher
        u(0x37, 2, 0x0001_1000),     // LUI x2, 0x11000
        i(0x13, 2, 0x0, 2, 0x170),   // ADDI x2, x2, 0x170 -> 70000
        i(0x13, 2, 0x1, 2, 16),      // SLLI x2, x2, 16
        u(0x37, 3, 0x0000_3000),     // LUI x3, 0x3000
        i(0x13, 3, 0x0, 3, 0x333),   // ADDI x3, x3, 0x333 -> 0x3333
        r(0x33, 2, 0x6, 2, 3, 0x00), // OR x2, x2, x3
        s(0x23, 0x3, 1, 2, 0),       // SD x2, 0(x1)
    ]);
    let mut halt = None;
    for _ in 0..8 {
        if let Some(h) = m.step(&mut out) {
            halt = Some(h);
        }
    }
    let Some(Halt::Finisher { value, status }) = halt else {
        panic!("expected a finisher halt, got {halt:?}");
    };
    assert_eq!(
        value, 0x1_1170_3333,
        "the raw written word must survive whole"
    );
    assert_eq!(status, Some(70_000), "error 70000 must not be read as 4464");
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
    //
    // 2026-09-28: IT MOVES A FIFTH TIME, on that instruction and for the same reason as the
    // fourth. Row `V-001` implemented F and D, so `0x53` is OP-FP and every arm of it now
    // either executes or halts for a REASON — a rounding mode this machine declines to
    // approximate — which is not the same fact as "no arm at all" and would have made this
    // test assert the wrong thing.
    //
    // The word is now `0x57`, OP-V, the vector family: **no arm at all**, which is the
    // property this test needs and the only major family left that has it. `0x0200_0057` is
    // a well-formed V-encoded word rather than a random unused pattern, so the halt it
    // produces is the one a real vector program would provoke.
    //
    // WHEN ROW `V-007` IMPLEMENTS V, THIS MOVES A SIXTH TIME, and by then there may be no
    // major family left to move to. At that point the honest reading is that this test's
    // subject has run out — retire it as `tests/atomics.rs`'s
    // `the_families_this_machine_does_not_have_still_stop` was retired on 2026-09-28, and
    // for the same stated reason: a test given a word it does not mean is worse than no
    // test. Do not weaken what it asks.
    //
    // 2026-10-04: IT MOVES THE SIXTH TIME, and the subject has NOT run out. `V-007` gave
    // OP-V an arm — `0x0200_0057` is `vadd.vv v0, v0, v0`, unmasked and IN the executed
    // subset, so on a reset machine it now traps illegal (`vtype.vill` is set until a
    // `vsetvli`) and after one it adds — the same "no longer an absent family" fact that
    // moved it off `0x53`. But the
    // claim that OP-V was "the only major family left" with no arm was FALSE when written:
    // OP-IMM-32 (`0x1b`) and OP-32 (`0x3b`), the RV64I `w` forms, have no arm either,
    // although `spec/encodings-riscv64.tsv` lists `addiw` and its family. `V-007`'s oracle
    // found it — its first run halted on the `addiw` inside a `li`. So the word is now
    // `addiw a0, a0, 1`, `0x0015_051b`, assembled by `riscv64-elf-as`. When the `w` forms
    // are implemented this moves again; custom-0 (`0x0b`) is the family the ISA promises
    // will never be standard, if nothing well-formed is left by then.
    //
    // 2026-10-06: IT MOVES THE SEVENTH TIME, to custom-0, as the line above said it would.
    // `V-009` part (i-d) implemented OP-IMM-32 and OP-32 against QEMU, and no standard
    // major family is left without an arm. `0x0015_050b` is the same fields under opcode
    // `0x0b`, the family the ISA reserves for custom extensions and promises never to
    // standardise — so this word is unimplemented by construction, not by a gap, and the
    // test cannot be moved again by an extension landing.
    let mut out = Vec::new();
    let mut m = machine(&[0x0015_050b]);
    let halt = m.step(&mut out).expect("must halt");
    assert_eq!(
        halt,
        Halt::Unimplemented {
            pc: 0x8000_0000,
            word: 0x0015_050b,
            opcode: 0x0b
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
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
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
        vec: Default::default(),
        socket: None,
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

/// **W-363: A STORE STOPS AT THE BUDGET AND A LOAD DOES NOT** — the asymmetry that
/// keeps a program from overwriting its own injected input.
///
/// `input::inject` takes `old_top = mem.len()` and then resizes, so the input slab
/// sits ABOVE the RAM the caller asked for. The record allocator is a bump cursor
/// with no upper bound, growing up from the file-backed extent, so with one bound
/// for both directions the heap reaches the slab and the program overwrites the
/// source it is still reading. A peer session bisected what that costs to the octet: at
/// a 2,557,897-octet budget a ten-frame decode answered `Finisher` status 0 with
/// EIGHT frames and a different digest, while 2,557,896 refused outright. Neither
/// was `BeyondRam`. A wrong answer reported as success is the one outcome this VM's
/// whole design refuses, so the bound is split.
///
/// Three parts, and the third is the one a careless fix would break. The refusal
/// must happen; a store just below must still work, or the bound is simply broken
/// rather than placed; and a LOAD above the bound must still succeed, because
/// reading the injected input is the entire reason the slab is mapped at all.
#[test]
fn a_store_stops_at_the_store_limit_while_a_load_still_reaches_the_input_above_it() {
    const BASE: u64 = 0x8000_0000;
    const RAM: usize = 1 << 16;
    const BUDGET: usize = 1 << 15; // the slab would live in [BUDGET, RAM)

    // `sd x2, 0(x1)` and `ld x5, 0(x1)` over the same address register.
    let build = |word: u32| Machine {
        store_limit: BUDGET,
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: BASE,
        base: BASE,
        mem: {
            let mut v = vec![0u8; RAM];
            v[..4].copy_from_slice(&word.to_le_bytes());
            v
        },
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: yantra::Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    let store = s(0x23, 0x3, 1, 2, 0);
    let load = i(0x03, 5, 0x3, 1, 0);

    // (1) THE REFUSAL. The first octet above the budget, which is where the slab's
    // first octet would be.
    let mut m = build(store);
    m.x[1] = BASE + BUDGET as u64;
    m.x[2] = 0xDEAD_BEEF;
    let mut out = Vec::new();
    match m.step(&mut out) {
        Some(Halt::BeyondRam { ram: reported, .. }) => assert_eq!(
            reported, BUDGET,
            "the halt must name the BUDGET it crossed, not the length of `mem` — a \
             program told it ran out at {RAM} would raise the wrong number"
        ),
        other => panic!(
            "a store at the first octet above the budget must halt `BeyondRam`; got \
             {other:?}"
        ),
    }

    // (2) THE REMOVAL CONTROL. Eight octets lower is inside the budget and must
    // land, or part (1) proves only that stores are broken.
    let mut m = build(store);
    m.x[1] = BASE + BUDGET as u64 - 8;
    m.x[2] = 0x0102_0304_0506_0708;
    let mut out = Vec::new();
    assert_eq!(
        m.step(&mut out),
        None,
        "a store inside the budget must not halt"
    );
    assert_eq!(
        &m.mem[BUDGET - 8..BUDGET],
        &0x0102_0304_0506_0708u64.to_le_bytes(),
        "and it must actually have been written"
    );

    // (3) THE ASYMMETRY. A load from above the budget must SUCCEED — this is the
    // injected input, and a guard that walled it off for reads too would make the
    // slab unreachable and break every program that takes an input.
    let mut m = build(load);
    m.mem[BUDGET..BUDGET + 8].copy_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes());
    m.x[1] = BASE + BUDGET as u64;
    let mut out = Vec::new();
    assert_eq!(
        m.step(&mut out),
        None,
        "a LOAD above the store bound must be allowed — that region is the input"
    );
    assert_eq!(
        m.x[5], 0x1122_3344_5566_7788,
        "and it must have read the octets that are there"
    );
}

/// **W-363's THIRD PIECE: THE STORE BOUND UNDER `Span::Declared` TOO.**
///
/// `load_elf` (the declared span, every caller but the browser walker) left
/// `store_limit` at `usize::MAX`, so once input was injected above the RAM it
/// asked for — `input::inject` RESIZES `mem` past it — a store could walk
/// straight into the slab and overwrite the program's own input, silently: the
/// 2026-09-21 self-image build grew to 559,504,480 octets over a 555,254,376 top,
/// and `yantra-run.rs`'s advisory stderr line was the only witness. Now the same
/// store halts `BeyondRam`, naming the RAM it was given. The resize here is the
/// one `inject` performs; a load above the bound must still reach the slab.
#[test]
fn under_the_declared_span_a_store_stops_at_the_ram_it_was_loaded_with() {
    const BASE: u64 = 0x8000_0000;
    const RAM: usize = 1 << 16;
    let store = s(0x23, 0x3, 1, 2, 0); // sd x2, 0(x1)
    let load = i(0x03, 5, 0x3, 1, 0); //  ld x5, 0(x1)
    let machine = |word: u32| {
        let mut m = Machine::load_elf(&kosha::write(&word.to_le_bytes()), RAM)
            .expect("a four-octet image loads at the declared span");
        m.mem.resize(RAM + 4096, 0x5A); // the slab, as `input::inject` appends it
        m
    };

    // (1) THE REFUSAL: the slab's first octet.
    let mut m = machine(store);
    m.x[1] = BASE + RAM as u64;
    m.x[2] = 0xDEAD_BEEF;
    match m.step(&mut Vec::new()) {
        Some(Halt::BeyondRam { ram, .. }) => assert_eq!(ram, RAM, "names the RAM it was given"),
        other => panic!(
            "a store into the injected slab under `Span::Declared` must halt `BeyondRam`; \
             got {other:?} and the slab now reads {:02x?}",
            &m.mem[RAM..RAM + 8]
        ),
    }
    assert_eq!(
        &m.mem[RAM..RAM + 8],
        &[0x5A; 8],
        "and the input is untouched"
    );

    // (2) THE REMOVAL CONTROL: eight octets lower lands.
    let mut m = machine(store);
    m.x[1] = BASE + RAM as u64 - 8;
    m.x[2] = 7;
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "a store inside RAM must not halt"
    );
    assert_eq!(m.mem[RAM - 8], 7);

    // (3) READING THE INPUT still works.
    let mut m = machine(load);
    m.x[1] = BASE + RAM as u64;
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "a load above the bound reaches the slab"
    );
    assert_eq!(m.x[5], 0x5A5A_5A5A_5A5A_5A5A);
}
