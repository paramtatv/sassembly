//! `F-001b` — fences and the A extension, held to the project's own encoding table.
//!
//! # Why this file has an oracle and `interpreter.rs` does not
//!
//! The spike's tests hand-encode every word, deliberately, so the interpreter and its
//! test cannot agree with each other while both disagree with the ISA. That works for
//! eight opcodes and twenty-odd instructions. The A extension is **twenty-two**
//! instructions differing only in a five-bit field, and hand-encoding twenty-two words is
//! exactly the transcription this repository keeps being burnt by: one wrong nibble and
//! the test asserts about an instruction nobody meant.
//!
//! So the encodings come from [`spec/encodings-riscv64.tsv`], which is **extracted from
//! `riscv64-elf-as`** rather than transcribed by anyone — its own header says so — and the
//! *semantics* come from the mnemonic, written out here by hand. The two halves have
//! independent origins, which is what makes the pair an oracle instead of a mirror:
//! the table cannot tell you that `amomin` is signed, and this file cannot tell you that
//! `amomin.d` is `0x8000302f`.
//!
//! # What is asserted, and what deliberately is not
//!
//! **Both directions.** Every `AMO` and `MISC-MEM` row in the table must execute; and the
//! families this machine does *not* implement — `mul`, `div`, `csr`, the floating point —
//! must still stop with [`Halt::Unimplemented`]. A decoder that has quietly grown a
//! catch-all arm passes the first half and fails the second.
//!
//! NOT asserted: anything about concurrency. There is one hart. These instructions are
//! being tested for their *values*, which is all a single-hart machine can be wrong about.

use std::collections::BTreeMap;
use yantra::{Halt, Machine, Privilege};

/// Where the test puts the word an atomic operates on.
const TARGET: u64 = 0x8000_1000;
const BASE: u64 = 0x8000_0000;

/// A machine with one instruction at the entry point, `x10` pointing at [`TARGET`] and
/// `x11` holding the second operand.
fn machine(word: u32, target: u64, width: usize, src: u64) -> Machine {
    let mut m = Machine {
        x: [0; 32],
        pc: BASE,
        base: BASE,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    m.mem[0..4].copy_from_slice(&word.to_le_bytes());
    let at = (TARGET - BASE) as usize;
    m.mem[at..at + width].copy_from_slice(&target.to_le_bytes()[..width]);
    m.x[10] = TARGET;
    m.x[11] = src;
    m
}

/// Read the `width` bytes at [`TARGET`] back, zero-extended.
fn target_of(m: &Machine) -> u64 {
    let at = (TARGET - BASE) as usize;
    let mut v = 0u64;
    for i in 0..8 {
        v |= u64::from(m.mem[at + i]) << (8 * i);
    }
    v
}

// ---------------------------------------------------------------------------------------
// The oracle.

/// One row of `spec/encodings-riscv64.tsv`, reduced to what an interpreter cares about.
struct Row {
    insn: String,
    family: String,
    pattern: u32,
    width: usize,
}

fn table() -> Vec<Row> {
    // `spec/` is tracked, unlike `research/specs/`, so this file is always present and a
    // missing-file path would be dead code pretending to be care.
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/encodings-riscv64.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut rows = Vec::new();
    for line in text.lines().skip_while(|l| l.starts_with('#')).skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 8 || f[5] != "32" {
            continue; // the compressed encodings are 16 bits and are not implemented
        }
        let pattern = u32::from_str_radix(f[3].trim_start_matches("0x"), 16).expect("pattern");
        rows.push(Row {
            insn: f[0].to_string(),
            family: f[1].to_string(),
            pattern,
            width: match f[7] {
                "32" => 4,
                "64" => 8,
                _ => 0,
            },
        });
    }
    assert!(rows.len() > 100, "the table did not parse: {}", rows.len());
    rows
}

/// What the mnemonic says must happen: `(value left in memory, value left in rd)`, given
/// the old memory contents and the second operand. Written from the instruction's NAME —
/// the table knows the bits and nothing about the meaning.
fn meaning(family: &str, width: usize, old: u64, src: u64) -> (u64, u64) {
    let mask = if width == 8 { !0u64 } else { 0xffff_ffff };
    let sext = |v: u64| {
        if width == 8 {
            v
        } else {
            u64::from(v as u32) as i32 as i64 as u64
        }
    };
    let (a, b) = (sext(old) as i64, sext(src) as i64);
    let (ua, ub) = (old & mask, src & mask);
    let result = match family {
        "lr" => return (old & mask, sext(old)),
        // With no `lr` before it an `sc` must fail, and failing means leaving memory alone.
        "sc" => return (old & mask, 1),
        "amoadd" => (a.wrapping_add(b)) as u64,
        "amoswap" => src,
        "amoxor" => old ^ src,
        "amoor" => old | src,
        "amoand" => old & src,
        "amomin" => {
            if a < b {
                old
            } else {
                src
            }
        }
        "amomax" => {
            if a > b {
                old
            } else {
                src
            }
        }
        "amominu" => {
            if ua < ub {
                old
            } else {
                src
            }
        }
        "amomaxu" => {
            if ua > ub {
                old
            } else {
                src
            }
        }
        other => panic!("no meaning written for family `{other}` — write one, do not skip it"),
    };
    (result & mask, sext(old))
}

#[test]
fn every_atomic_in_the_encoding_table_executes_and_gets_the_right_value() {
    // -16 as a signed operand of either width, against +3: the pair distinguishes signed
    // from unsigned min/max, which a pair of small positives cannot.
    let old: u64 = 0xffff_ffff_ffff_fff0;
    let src: u64 = 3;

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for row in table().into_iter().filter(|r| r.pattern & 0x7f == 0x2f) {
        // rd = x1, rs1 = x10 (the address), rs2 = x11 (the operand). The pattern has all
        // three fields zero, so this is an OR rather than a rewrite.
        // `lr` has no second source and REFUSES a word with the field set — the first
        // run of this loop set it on every row and lr.d duly stopped, which is the
        // harness being wrong about the ISA rather than the machine.
        let rs2 = if row.family == "lr" { 0 } else { 11 };
        let word = row.pattern | 1 << 7 | 10 << 15 | rs2 << 20;
        let mut m = machine(word, old, row.width, src);
        let mut out = Vec::new();
        let halt = m.step(&mut out);
        assert!(
            halt.is_none(),
            "{} ({:#010x}) did not execute: {halt:?}",
            row.insn,
            row.pattern
        );

        let (want_mem, want_rd) = meaning(&row.family, row.width, old, src);
        let mask = if row.width == 8 { !0u64 } else { 0xffff_ffff };
        assert_eq!(
            target_of(&m) & mask,
            want_mem,
            "{} left the wrong value in memory",
            row.insn
        );
        assert_eq!(m.x[1], want_rd, "{} left the wrong value in rd", row.insn);
        assert_eq!(m.pc, BASE + 4, "{} must advance the pc", row.insn);
        assert!(out.is_empty(), "{} wrote to the UART", row.insn);
        *seen.entry(row.family.clone()).or_default() += 1;
    }
    // 11 families × two widths. Named rather than counted only, so a table that lost half
    // its rows cannot pass by having 22 of something else.
    assert_eq!(seen.len(), 11, "families covered: {seen:?}");
    assert_eq!(seen.values().sum::<usize>(), 22, "rows covered: {seen:?}");
}

#[test]
fn every_fence_in_the_encoding_table_executes_as_a_no_op() {
    let mut n = 0;
    for row in table().into_iter().filter(|r| r.pattern & 0x7f == 0x0f) {
        // Fences carry no registers. `fence`'s pred/succ fields are zero in the pattern,
        // which is `fence` with an empty ordering — still a legal encoding, and still a
        // no-op here for the same reason a full one is.
        let mut m = machine(row.pattern, 0, 8, 3);
        let mut out = Vec::new();
        assert!(
            m.step(&mut out).is_none(),
            "{} ({:#010x}) did not execute",
            row.insn,
            row.pattern
        );
        assert_eq!(m.pc, BASE + 4, "{} must advance the pc", row.insn);
        assert_eq!(m.x[11], 3, "{} must not touch a register", row.insn);
        n += 1;
    }
    assert_eq!(n, 2, "fence and fence.i");
}

#[test]
fn the_families_this_machine_does_not_have_still_stop() {
    // The other half of the oracle. Every one of these is a word the table says is a real
    // RV64GC instruction and this interpreter cannot execute; each must halt NAMED. A
    // catch-all arm added while extending the decoder passes the tests above and fails
    // this one.
    //
    // `csrrw`, `csrrs`, `ecall` and `ebreak` stood in this list until `F-001c1`, which
    // gave each of them a halt of its OWN — `Halt::Csr`, `Halt::Sbi`, `Halt::Breakpoint`.
    // They are still named and still stop; they are asserted in `tests/system.rs` now,
    // and removing them from here is the list catching up with the machine rather than
    // being relaxed. `sret` and `sfence.vma` replaced them so the family stayed
    // represented — and `F-001c2b1` executed the last of them, so SYSTEM is no longer
    // represented here AT ALL. That is not the list being weakened: every one of the ten
    // 32-bit SYSTEM rows is now asserted in `tests/system.rs` to do what its name says,
    // which is a stronger claim than "it stops". What remains here are the families
    // this machine genuinely has not got.
    //
    // 2026-09-05: `mul`, `div` and `rem` LEAVE this list, because the machine implements
    // the M extension now — two demonstration programs needed to multiply and to take a
    // remainder, and the frozen grammar promises both. This is the same catching-up the
    // paragraph above describes for SYSTEM, and it is subject to the same requirement:
    // the claim is not weakened by their removal, because all eight M forms are now
    // asserted to compute the right answers in `tests/interpreter.rs` — including the
    // zero divisor and the single overflow case, which RISC-V answers by value and a
    // host language's own `/` would panic on. That is stronger than "it stops".
    //
    // D remains, and is now the only family here. If it ever lands, this list becomes
    // empty and the test should be retired rather than given a word it does not mean.
    let absent = ["fadd.d"];
    let rows = table();
    for name in absent {
        let row = rows
            .iter()
            .find(|r| r.insn == name)
            .unwrap_or_else(|| panic!("`{name}` is not in the encoding table"));
        let word = row.pattern | 1 << 7 | 10 << 15 | 11 << 20;
        let mut m = machine(word, 0, 8, 3);
        let mut out = Vec::new();
        assert!(
            matches!(m.step(&mut out), Some(Halt::Unimplemented { .. })),
            "`{name}` ({:#010x}) must stop and say so",
            row.pattern
        );
    }
}

// ---------------------------------------------------------------------------------------
// The parts no single-instruction table row can express.

#[test]
fn a_reserved_word_survives_the_pair_and_a_bare_sc_fails() {
    let rows = table();
    let word = |name: &str, rd: u32, rs2: u32| {
        rows.iter()
            .find(|r| r.insn == name)
            .unwrap_or_else(|| panic!("{name}"))
            .pattern
            | rd << 7
            | 10 << 15
            | rs2 << 20
    };

    // lr.d x1, (x10) ; sc.d x2, x11, (x10)
    let mut m = machine(word("lr.d", 1, 0), 7, 8, 42);
    m.mem[4..8].copy_from_slice(&word("sc.d", 2, 11).to_le_bytes());
    let mut out = Vec::new();
    assert!(m.step(&mut out).is_none());
    assert_eq!(m.x[1], 7, "lr must read the old value");
    assert_eq!(m.reservation, Some(TARGET));
    assert!(m.step(&mut out).is_none());
    assert_eq!(
        m.x[2], 0,
        "sc after its own lr must SUCCEED, and 0 means success"
    );
    assert_eq!(target_of(&m), 42, "a successful sc must store");
    assert_eq!(m.reservation, None, "the reservation is spent either way");

    // The same `sc` with no `lr` in front of it must fail AND leave memory alone. A
    // machine that stores anyway passes every value test above — one hart never notices
    // the difference until a second one exists, which is the wrong time to find out.
    let mut m = machine(word("sc.d", 2, 11), 7, 8, 42);
    assert!(m.step(&mut out).is_none());
    assert_eq!(m.x[2], 1, "a bare sc must report failure");
    assert_eq!(target_of(&m), 7, "a failed sc must not store");

    // And a reservation for a DIFFERENT address does not licence this one.
    let mut m = machine(word("sc.d", 2, 11), 7, 8, 42);
    m.reservation = Some(TARGET + 8);
    assert!(m.step(&mut out).is_none());
    assert_eq!(m.x[2], 1);
    assert_eq!(target_of(&m), 7);
}

#[test]
fn a_misaligned_atomic_performs_no_access() {
    let rows = table();
    let amoadd_d = rows.iter().find(|r| r.insn == "amoadd.d").unwrap().pattern;
    let mut m = machine(amoadd_d | 1 << 7 | 10 << 15 | 11 << 20, 7, 8, 3);
    m.x[10] = TARGET + 4; // aligned for `.w`, not for `.d`
    let mut out = Vec::new();
    assert_eq!(
        m.step(&mut out),
        Some(Halt::BadAccess {
            pc: BASE,
            addr: TARGET + 4
        })
    );
    assert_eq!(target_of(&m), 7, "the refused atomic must not have written");
}

#[test]
fn an_lr_with_a_second_source_is_not_executed() {
    // `lr`'s rs2 field is reserved and its mask in the table (0xfff0707f) covers it. A
    // decoder that ignores the field would run a word the ISA does not define.
    let lr_d = table().iter().find(|r| r.insn == "lr.d").unwrap().pattern;
    let mut m = machine(lr_d | 1 << 7 | 10 << 15 | 11 << 20, 7, 8, 3);
    let mut out = Vec::new();
    assert!(matches!(m.step(&mut out), Some(Halt::Unimplemented { .. })));
}

#[test]
fn the_widths_are_not_interchangeable() {
    // amoswap.w on a doubleword must replace the LOW HALF ONLY and sign-extend what it
    // read. This is the one property a `.d`-only implementation passes every other test
    // with — and getting it wrong corrupts the neighbouring half of a 64-bit variable.
    let rows = table();
    let amoswap_w = rows.iter().find(|r| r.insn == "amoswap.w").unwrap().pattern;
    let mut m = machine(
        amoswap_w | 1 << 7 | 10 << 15 | 11 << 20,
        0xdead_beef_8000_0001,
        8,
        5,
    );
    let mut out = Vec::new();
    assert!(m.step(&mut out).is_none());
    assert_eq!(
        target_of(&m),
        0xdead_beef_0000_0005,
        "the upper word must be untouched"
    );
    assert_eq!(
        m.x[1], 0xffff_ffff_8000_0001,
        "rd takes the SIGN-EXTENDED old word"
    );
}
