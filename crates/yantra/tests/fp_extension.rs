//! **The F and D extensions — row `V-001`'s acceptance.**
//!
//! Before this row `fadd.d` halted [`Halt::Unimplemented`], and the scientific-compute
//! specification's entire encoder section was therefore unvalidatable: an oracle cannot
//! disagree with an instruction the machine refuses to run.
//!
//! Every test here drives ONE instruction word into memory and steps once, so a failure
//! names an encoding rather than a program. The encoders below are written from the
//! specification's own field layout — `funct7` is `(op5 << 2) | fmt` — so a reader can
//! check a constant against the table without doing the shift in their head.
//!
//! **THE TEST THIS FILE EXISTS FOR IS [`every_static_rounding_mode_is_honoured`]** (with
//! [`a_dyn_mode_rounds_by_frm`]). Its rule: a rounding mode is honoured or refused, never
//! ignored — ignoring `rm` would make this machine agree with an encoder emitting `rtz`
//! and disagree with real hardware, a twin agreeing with a stub. Until V-009 (i-f) Rust's
//! nearest-even was the only rounding here, so every other mode HALTED (the test was
//! `a_rounding_mode_this_machine_cannot_honour_halts`); the arithmetic rounds in software
//! since (i-e), and the owner's ruling (b) lifted the halt, so now every mode is honoured
//! and the test asserts QEMU's rounding of a tie under each.

use yantra::{Csrs, Halt, Machine, Privilege, fp};

const BASE: u64 = 0x8000_0000;
/// `sstatus.FS`, bits 14:13, and its Initial and Dirty values.
const FS: u64 = 0b11 << 13;
const FS_INITIAL: u64 = 0b01 << 13;

/// A machine with one instruction word at `BASE` and nothing else.
fn machine(word: u32) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
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
        // FS = Initial, as the startup sets it (V-009 (i-b)): under FS = Off every F/D
        // instruction is illegal (i-c), and this file is about what they compute.
        csr: Csrs {
            sstatus: FS_INITIAL,
            ..Csrs::default()
        },
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
        net: None,
    };
    m.mem[0..4].copy_from_slice(&word.to_le_bytes());
    m
}

/// Step once, asserting the machine did not halt, and hand back the machine.
fn step_ok(mut m: Machine) -> Machine {
    let mut out = Vec::new();
    if let Some(h) = m.step(&mut out) {
        panic!("the instruction halted where it should have run: {h:?}");
    }
    m
}

/// OP-FP, `0x53`.
fn op_fp(funct7: u32, rs2: u32, rs1: u32, rm: u32, rd: u32) -> u32 {
    (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (rm << 12) | (rd << 7) | 0x53
}

/// LOAD-FP, `0x07`.
fn load_fp(imm: u32, rs1: u32, funct3: u32, rd: u32) -> u32 {
    ((imm & 0xfff) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | 0x07
}

/// STORE-FP, `0x27`.
fn store_fp(imm: u32, rs2: u32, rs1: u32, funct3: u32) -> u32 {
    let hi = (imm >> 5) & 0x7f;
    let lo = imm & 0x1f;
    (hi << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (lo << 7) | 0x27
}

/// The R4 format: `fmadd` and friends.
fn r4(rs3: u32, fmt: u32, rs2: u32, rs1: u32, rm: u32, rd: u32, opcode: u32) -> u32 {
    (rs3 << 27) | (fmt << 25) | (rs2 << 20) | (rs1 << 15) | (rm << 12) | (rd << 7) | opcode
}

// ── arithmetic ───────────────────────────────────────────────────────────────

#[test]
fn fadd_d_adds() {
    let mut m = machine(op_fp(0x01, 2, 1, fp::RNE, 3));
    m.f[1] = 1.5f64.to_bits();
    m.f[2] = 2.25f64.to_bits();
    let m = step_ok(m);
    assert_eq!(f64::from_bits(m.f[3]), 3.75);
}

#[test]
fn fsub_fmul_fdiv_d() {
    for (funct7, a, b, want) in [
        (0x05u32, 10.0f64, 2.5f64, 7.5f64), // fsub.d
        (0x09, 1.5, 4.0, 6.0),              // fmul.d
        (0x0d, 9.0, 2.0, 4.5),              // fdiv.d
    ] {
        let mut m = machine(op_fp(funct7, 2, 1, fp::RNE, 3));
        m.f[1] = a.to_bits();
        m.f[2] = b.to_bits();
        let m = step_ok(m);
        assert_eq!(
            f64::from_bits(m.f[3]),
            want,
            "funct7 {funct7:#04x} on {a} and {b}"
        );
    }
}

#[test]
fn fsqrt_d() {
    let mut m = machine(op_fp(0x2d, 0, 1, fp::RNE, 2));
    m.f[1] = 6.25f64.to_bits();
    let m = step_ok(m);
    assert_eq!(f64::from_bits(m.f[2]), 2.5);
}

#[test]
fn fmadd_d_fuses_and_the_four_forms_differ_in_sign() {
    // a*b + c with a=2, b=3, c=1 → the four opcodes give 7, 5, -5, -7.
    for (opcode, want) in [(0x43u32, 7.0f64), (0x47, 5.0), (0x4b, -5.0), (0x4f, -7.0)] {
        let mut m = machine(r4(3, 1, 2, 1, fp::RNE, 4, opcode));
        m.f[1] = 2.0f64.to_bits();
        m.f[2] = 3.0f64.to_bits();
        m.f[3] = 1.0f64.to_bits();
        let m = step_ok(m);
        assert_eq!(
            f64::from_bits(m.f[4]),
            want,
            "opcode {opcode:#04x} — the fused forms' signs"
        );
    }
}

// ── flags ────────────────────────────────────────────────────────────────────

#[test]
fn fdiv_by_zero_sets_dz_and_answers_an_infinity() {
    let mut m = machine(op_fp(0x0d, 2, 1, fp::RNE, 3));
    m.f[1] = 1.0f64.to_bits();
    m.f[2] = 0.0f64.to_bits();
    let m = step_ok(m);
    assert!(
        f64::from_bits(m.f[3]).is_infinite(),
        "1.0/0.0 must be an infinity, not a trap and not a NaN"
    );
    assert_eq!(m.fflags() & fp::DZ, fp::DZ, "DZ must be set");
}

#[test]
fn zero_over_zero_is_invalid_not_divide_by_zero() {
    let mut m = machine(op_fp(0x0d, 2, 1, fp::RNE, 3));
    m.f[1] = 0.0f64.to_bits();
    m.f[2] = 0.0f64.to_bits();
    let m = step_ok(m);
    assert!(f64::from_bits(m.f[3]).is_nan(), "0/0 is a NaN");
    assert_eq!(m.fflags() & fp::NV, fp::NV, "NV, not DZ, is 0/0's flag");
    assert_eq!(m.fflags() & fp::DZ, 0, "0/0 must NOT set DZ");
}

#[test]
fn flags_accumulate_and_are_not_cleared_by_a_later_clean_operation() {
    // The architectural contract: only a write to fflags clears them.
    let mut m = machine(op_fp(0x0d, 2, 1, fp::RNE, 3));
    m.f[1] = 1.0f64.to_bits();
    m.f[2] = 0.0f64.to_bits();
    let mut m = step_ok(m);
    assert_eq!(m.fflags() & fp::DZ, fp::DZ);
    // a clean add at the next word
    m.mem[4..8].copy_from_slice(&op_fp(0x01, 2, 1, fp::RNE, 5).to_le_bytes());
    m.f[1] = 1.0f64.to_bits();
    m.f[2] = 1.0f64.to_bits();
    let m = step_ok(m);
    assert_eq!(
        m.fflags() & fp::DZ,
        fp::DZ,
        "a later clean operation must not clear an accrued flag"
    );
}

// ── the rule that is NOT Rust's ───────────────────────────────────────────────

#[test]
fn fmin_returns_the_non_nan_operand() {
    // RISC-V: one NaN → the OTHER operand. This is the rule a naive `f64::min`
    // port gets right by accident and a naive `if a < b` gets wrong.
    let mut m = machine(op_fp(0x15, 2, 1, 0, 3)); // fmin.d
    m.f[1] = f64::NAN.to_bits();
    m.f[2] = 2.0f64.to_bits();
    let m = step_ok(m);
    assert_eq!(
        f64::from_bits(m.f[3]),
        2.0,
        "fmin(NaN, 2.0) is 2.0, not NaN"
    );
}

#[test]
fn fmin_of_both_nan_is_the_canonical_nan() {
    let mut m = machine(op_fp(0x15, 2, 1, 0, 3));
    m.f[1] = f64::NAN.to_bits();
    m.f[2] = f64::from_bits(0x7ff8_0000_dead_beef).to_bits();
    let m = step_ok(m);
    assert_eq!(
        m.f[3],
        fp::CANONICAL_NAN_D,
        "both NaN → the CANONICAL NaN, payload discarded"
    );
}

#[test]
fn fmin_and_fmax_order_the_signed_zeros() {
    // -0.0 < +0.0, which a bare `<` does not see because they compare equal.
    let mut m = machine(op_fp(0x15, 2, 1, 0, 3)); // fmin.d
    m.f[1] = (-0.0f64).to_bits();
    m.f[2] = 0.0f64.to_bits();
    let m = step_ok(m);
    assert!(
        f64::from_bits(m.f[3]).is_sign_negative(),
        "fmin(-0.0, +0.0) must be -0.0"
    );

    let mut m = machine(op_fp(0x15, 2, 1, 1, 3)); // fmax.d
    m.f[1] = (-0.0f64).to_bits();
    m.f[2] = 0.0f64.to_bits();
    let m = step_ok(m);
    assert!(
        f64::from_bits(m.f[3]).is_sign_positive(),
        "fmax(-0.0, +0.0) must be +0.0"
    );
}

// ── conversions ──────────────────────────────────────────────────────────────

#[test]
fn fcvt_w_d_honours_every_rounding_mode() {
    // 2.5 and -2.5 separate ties-to-even from ties-away, and floor from ceil.
    for (rm, x, want) in [
        (fp::RTZ, 2.7f64, 2i64),
        (fp::RTZ, -2.7, -2),
        (fp::RDN, 2.7, 2),
        (fp::RDN, -2.7, -3),
        (fp::RUP, 2.1, 3),
        (fp::RUP, -2.1, -2),
        (fp::RNE, 2.5, 2), // ties to EVEN
        (fp::RNE, 3.5, 4),
        (fp::RMM, 2.5, 3), // ties AWAY
        (fp::RMM, -2.5, -3),
    ] {
        let mut m = machine(op_fp(0x61, 0, 1, rm, 5)); // fcvt.w.d
        m.f[1] = x.to_bits();
        let m = step_ok(m);
        assert_eq!(
            m.x[5] as i64, want,
            "fcvt.w.d rm={rm} on {x} — rounding mode must be honoured, not ignored"
        );
    }
}

#[test]
fn fcvt_w_d_saturates_and_does_not_wrap() {
    // THE SPEC PINS THESE. A wrapping `as` cast would make a converted NaN
    // indistinguishable from a small negative integer.
    for (x, want) in [
        (f64::NAN, i64::from(i32::MAX)),
        (f64::INFINITY, i64::from(i32::MAX)),
        (f64::NEG_INFINITY, i64::from(i32::MIN)),
        (1e300, i64::from(i32::MAX)),
        (-1e300, i64::from(i32::MIN)),
    ] {
        let mut m = machine(op_fp(0x61, 0, 1, fp::RTZ, 5));
        m.f[1] = x.to_bits();
        let m = step_ok(m);
        assert_eq!(m.x[5] as i64, want, "fcvt.w.d of {x} saturates");
        assert_eq!(m.fflags() & fp::NV, fp::NV, "and sets NV");
    }
}

#[test]
fn fcvt_d_w_and_back() {
    let mut m = machine(op_fp(0x69, 0, 1, fp::RNE, 2)); // fcvt.d.w
    m.x[1] = (-7i64) as u64;
    let m = step_ok(m);
    assert_eq!(f64::from_bits(m.f[2]), -7.0);
}

#[test]
fn fcvt_between_the_two_widths() {
    let mut m = machine(op_fp(0x20, 1, 1, fp::RNE, 2)); // fcvt.s.d
    m.f[1] = 1.5f64.to_bits();
    let m = step_ok(m);
    assert_eq!(
        fp::unbox_s(m.f[2]),
        1.5f32,
        "fcvt.s.d, and the result is BOXED"
    );

    let mut m = machine(op_fp(0x21, 0, 1, fp::RNE, 2)); // fcvt.d.s
    m.f[1] = fp::box_s(2.5f32);
    let m = step_ok(m);
    assert_eq!(f64::from_bits(m.f[2]), 2.5f64);
}

// ── comparison, classification, moves ────────────────────────────────────────

#[test]
fn feq_flt_fle_and_their_nan_behaviour() {
    // feq is the QUIET comparison: a quiet NaN answers false WITHOUT NV.
    let mut m = machine(op_fp(0x51, 2, 1, 2, 5)); // feq.d
    m.f[1] = f64::NAN.to_bits();
    m.f[2] = 1.0f64.to_bits();
    let m = step_ok(m);
    assert_eq!(m.x[5], 0, "feq with a NaN is false");
    assert_eq!(
        m.fflags() & fp::NV,
        0,
        "and feq must NOT set NV for a quiet NaN"
    );

    // flt is the SIGNALLING one.
    let mut m = machine(op_fp(0x51, 2, 1, 1, 5)); // flt.d
    m.f[1] = f64::NAN.to_bits();
    m.f[2] = 1.0f64.to_bits();
    let m = step_ok(m);
    assert_eq!(m.x[5], 0);
    assert_eq!(m.fflags() & fp::NV, fp::NV, "flt with a NaN DOES set NV");

    // and the ordinary answers
    let mut m = machine(op_fp(0x51, 2, 1, 0, 5)); // fle.d
    m.f[1] = 1.0f64.to_bits();
    m.f[2] = 1.0f64.to_bits();
    let m = step_ok(m);
    assert_eq!(m.x[5], 1, "1.0 <= 1.0");
}

#[test]
fn fclass_d_names_each_class() {
    for (x, bit) in [
        (f64::NEG_INFINITY, 0),
        (-1.0f64, 1),
        (-0.0f64, 3),
        (0.0f64, 4),
        (1.0f64, 6),
        (f64::INFINITY, 7),
    ] {
        let mut m = machine(op_fp(0x71, 0, 1, 1, 5)); // fclass.d
        m.f[1] = x.to_bits();
        let m = step_ok(m);
        assert_eq!(m.x[5], 1 << bit, "fclass.d of {x} is bit {bit}");
    }
}

#[test]
fn fmv_moves_raw_bits_in_both_directions() {
    // fmv.d.x — an integer pattern becomes float bits with NO conversion.
    let pattern = 0x4008_0000_0000_0000u64; // 3.0
    let mut m = machine(op_fp(0x79, 0, 1, 0, 2));
    m.x[1] = pattern;
    let m = step_ok(m);
    assert_eq!(m.f[2], pattern, "fmv.d.x moves bits, it does not convert");
    assert_eq!(f64::from_bits(m.f[2]), 3.0);

    // fmv.x.d back again
    let mut m = machine(op_fp(0x71, 0, 1, 0, 5));
    m.f[1] = pattern;
    let m = step_ok(m);
    assert_eq!(m.x[5], pattern);
}

#[test]
fn fsgnj_injects_a_sign_without_arithmetic() {
    // fsgnj.d rd, rs1, rs2 — magnitude of rs1, sign of rs2.
    let mut m = machine(op_fp(0x11, 2, 1, 0, 3));
    m.f[1] = 2.5f64.to_bits();
    m.f[2] = (-1.0f64).to_bits();
    let m = step_ok(m);
    assert_eq!(f64::from_bits(m.f[3]), -2.5);
    assert_eq!(
        m.fflags(),
        0,
        "sign injection sets NO flag — it is not arithmetic"
    );
}

// ── NaN-boxing, which is load-bearing ────────────────────────────────────────

#[test]
fn a_register_holding_a_double_does_not_read_as_a_single() {
    // THE POINT OF BOXING. Without it, `fadd.s` on a register written by `fld`
    // would reinterpret the low half of a double as a float and produce a
    // plausible wrong number instead of a NaN.
    let mut m = machine(op_fp(0x00, 2, 1, fp::RNE, 3)); // fadd.s
    m.f[1] = 1.5f64.to_bits(); // a DOUBLE, not boxed
    m.f[2] = fp::box_s(1.0f32);
    let m = step_ok(m);
    assert!(
        fp::unbox_s(m.f[3]).is_nan(),
        "a double read as a single must be NaN, not its low half reinterpreted"
    );
}

#[test]
fn fadd_s_on_properly_boxed_singles() {
    let mut m = machine(op_fp(0x00, 2, 1, fp::RNE, 3));
    m.f[1] = fp::box_s(1.5f32);
    m.f[2] = fp::box_s(2.25f32);
    let m = step_ok(m);
    assert_eq!(fp::unbox_s(m.f[3]), 3.75f32);
    assert_eq!(m.f[3] >> 32, 0xffff_ffff, "and the result is boxed");
}

// ── memory ───────────────────────────────────────────────────────────────────

#[test]
fn fld_and_fsd_round_trip_through_memory() {
    let at: u64 = 0x100;
    let mut m = machine(load_fp(at as u32, 1, 0x3, 2)); // fld f2, at(x1)
    m.x[1] = BASE;
    let v = 1234.5f64;
    m.mem[at as usize..at as usize + 8].copy_from_slice(&v.to_bits().to_le_bytes());
    let mut m = step_ok(m);
    assert_eq!(f64::from_bits(m.f[2]), v, "fld");

    // store it back somewhere else
    let to: u64 = 0x200;
    m.mem[4..8].copy_from_slice(&store_fp(to as u32, 2, 1, 0x3).to_le_bytes());
    let m = step_ok(m);
    let got = u64::from_le_bytes(m.mem[to as usize..to as usize + 8].try_into().unwrap());
    assert_eq!(f64::from_bits(got), v, "fsd");
}

#[test]
fn flw_boxes_what_it_loads() {
    let at: u64 = 0x100;
    let mut m = machine(load_fp(at as u32, 1, 0x2, 2)); // flw
    m.x[1] = BASE;
    m.mem[at as usize..at as usize + 4].copy_from_slice(&2.5f32.to_bits().to_le_bytes());
    let m = step_ok(m);
    assert_eq!(
        m.f[2] >> 32,
        0xffff_ffff,
        "flw must BOX, or a later fadd.s sees a double"
    );
    assert_eq!(fp::unbox_s(m.f[2]), 2.5f32);
}

// ── the refusals ─────────────────────────────────────────────────────────────

/// `1 + 2^-53` — an exact tie — under each mode, as QEMU 10.1.0 rounds it (V-009 (i-f),
/// `tools/v009-fp-probe`, case `d:1 + ulp/2 tie`): even (1.0) under RNE, RTZ and RDN; away
/// (1 + ulp) under RUP and RMM. NX each time.
const TIE: [(u32, u64); 5] = [
    (fp::RNE, 0x3ff0_0000_0000_0000),
    (fp::RTZ, 0x3ff0_0000_0000_0000),
    (fp::RDN, 0x3ff0_0000_0000_0000),
    (fp::RUP, 0x3ff0_0000_0000_0001),
    (fp::RMM, 0x3ff0_0000_0000_0001),
];

#[test]
fn every_static_rounding_mode_is_honoured() {
    // **THIS WAS THE TEST THIS FILE EXISTED FOR**, and its rule stands: a rounding mode is
    // honoured or refused, never ignored. Until V-009 (i-f) Rust's nearest-even was the
    // only rounding here, so RTZ/RDN/RUP/RMM halted `Unimplemented`; the arithmetic now
    // rounds in software in every mode (`fp::add` and its kin, checked against QEMU), so
    // the owner's ruling (b) lifted the halt. frm holds RUP throughout — a static mode
    // must not read it.
    for (rm, want) in TIE {
        let mut m = machine(op_fp(0x01, 2, 1, rm, 3)); // fadd.d f3, f1, f2, rm
        m.f[1] = 1.0f64.to_bits();
        m.f[2] = 2f64.powi(-53).to_bits();
        m.fcsr = (fp::RUP as u64) << 5;
        let m = step_ok(m);
        assert_eq!((m.f[3], m.fflags()), (want, fp::NX), "fadd.d rm={rm}");
    }
}

#[test]
fn rne_and_dyn_resolving_to_rne_are_accepted() {
    for rm in [fp::RNE, fp::DYN] {
        let mut m = machine(op_fp(0x01, 2, 1, rm, 3));
        m.f[1] = 1.0f64.to_bits();
        m.f[2] = 1.0f64.to_bits();
        m.fcsr = 0; // frm = RNE
        let m = step_ok(m);
        assert_eq!(f64::from_bits(m.f[3]), 2.0, "rm={rm} must be accepted");
    }
}

#[test]
fn a_dyn_mode_rounds_by_frm() {
    // DYN must read `frm` — the same tie, the mode supplied by frm (was: a halt for any
    // frm but RNE).
    for (rm, want) in TIE {
        let mut m = machine(op_fp(0x01, 2, 1, fp::DYN, 3));
        m.f[1] = 1.0f64.to_bits();
        m.f[2] = 2f64.powi(-53).to_bits();
        m.fcsr = u64::from(rm) << 5;
        let m = step_ok(m);
        assert_eq!((m.f[3], m.fflags()), (want, fp::NX), "fadd.d dyn, frm={rm}");
    }
}

// ── the state bit ────────────────────────────────────────────────────────────

#[test]
fn writing_an_f_register_marks_fs_dirty() {
    // `SSTATUS_MASK` kept `FS` for this. A supervisor that saves the register
    // file only when FS says dirty needs something to set it.
    let mut m = machine(op_fp(0x01, 2, 1, fp::RNE, 3));
    m.f[1] = 1.0f64.to_bits();
    m.f[2] = 1.0f64.to_bits();
    assert_eq!(m.csr.sstatus & FS, FS_INITIAL, "FS starts Initial");
    let m = step_ok(m);
    assert_eq!(
        m.csr.sstatus & FS,
        FS,
        "an f-register write must set sstatus.FS to Dirty, or keeping the bit is decorative"
    );
}

#[test]
fn a_comparison_does_not_mark_fs_dirty() {
    // It writes an INTEGER register, so the FPU state did not change.
    let mut m = machine(op_fp(0x51, 2, 1, 0, 5)); // fle.d
    m.f[1] = 1.0f64.to_bits();
    m.f[2] = 1.0f64.to_bits();
    let m = step_ok(m);
    assert_eq!(
        m.csr.sstatus & FS,
        FS_INITIAL,
        "fle writes x, not f — FS must stay Initial"
    );
}

// ── V-009 part (i-c): THE FS GATE ────────────────────────────────────────────
//
// With `sstatus.FS = Off` every F/D instruction — arithmetic, loads, stores and moves —
// is an ILLEGAL INSTRUCTION (cause 2), as on QEMU and real hardware. A compiled image's
// startup sets FS (i-b); a program that does not, faults.

#[test]
fn fadd_d_with_fs_off_is_an_illegal_instruction() {
    let w = op_fp(0x01, 2, 1, fp::RNE, 3); // fadd.d f3, f1, f2
    let mut m = machine(w);
    m.csr.sstatus &= !FS;
    m.f[1] = 1.0f64.to_bits();
    m.f[2] = 2.0f64.to_bits();
    let mut out = Vec::new();
    assert_eq!(
        m.step(&mut out),
        Some(Halt::Undelivered { pc: BASE, cause: 2 })
    );
    assert_eq!(m.f[3], 0, "the faulting instruction wrote nothing");
    assert_eq!(m.csr.sstatus & FS, 0, "and did not mark FS dirty");
}

#[test]
fn a_float_load_store_and_move_with_fs_off_are_illegal_instructions() {
    // fld f1, 0(x1) · fsd f1, 0(x1) · fmv.x.d x5, f1 — the gate is the whole F/D space,
    // not only OP-FP.
    let fld = (1 << 15) | (0x3 << 12) | (1 << 7) | 0x07;
    let fsd = (1 << 20) | (1 << 15) | (0x3 << 12) | 0x27;
    let fmv_x_d = op_fp(0x71, 0, 1, 0, 5);
    for w in [fld, fsd, fmv_x_d] {
        let mut m = machine(w);
        m.csr.sstatus &= !FS;
        m.x[1] = BASE + 0x100;
        let mut out = Vec::new();
        assert_eq!(
            m.step(&mut out),
            Some(Halt::Undelivered { pc: BASE, cause: 2 }),
            "{w:#010x} under FS = Off"
        );
    }
}
