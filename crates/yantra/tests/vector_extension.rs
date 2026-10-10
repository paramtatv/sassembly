//! **The V extension's executed subset — row `V-007`'s acceptance, machine half.**
//!
//! The differential half — every subset instruction against `qemu-system-riscv64` — is
//! `vector_oracle.rs`. This file pins what an oracle run cannot see on its own: that
//! everything OUTSIDE the subset halts by name, that `vill` at reset makes a vector
//! instruction illegal, that the tail is undisturbed, that `vstart` is honoured and
//! reported, and that `sstatus.VS` is kept and set (the refounding `V-001` did for `FS`).
//!
//! **NO BIT PATTERN HERE IS WRITTEN BY HAND.** `spec/encodings-riscv64.tsv` has no vector
//! rows (it is RV64GC), so every word below is produced by GNU binutils
//! (`riscv64-elf-as -march=rv64gcv`) from its mnemonic. The decoder in
//! `crates/yantra/src/vector.rs` reads the ratified field layout, and this file checks it
//! against an assembler this repository did not write.

use std::path::PathBuf;
use std::process::Command;

use yantra::vector::{self, FpOp, IntOp, Op, Src};
use yantra::{Csrs, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;
/// Where test data lives, inside the machine's 64 KiB.
const DATA: u64 = BASE + 0x8000;
const SSTATUS_VS: u64 = 0b11 << 9;
/// `sstatus.VS = Initial` and `sstatus.FS = Initial` — what the startup sets (V-009
/// (i-b)). Both are needed: a vector FLOAT instruction is gated on FS as well as VS (i-c).
const VS_INITIAL: u64 = 0b01 << 9;
const FS: u64 = 0b11 << 13;
const FS_INITIAL: u64 = 0b01 << 13;

/// A fresh directory per call. A COUNTER, not the clock: this Mac's clock moves in
/// microseconds, and two test threads asking in the same one shared a directory and read
/// each other's `t.bin` (this file's first red, 12 octets where 4 were assembled).
fn scratch(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "v007-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Assemble `lines` with binutils and return the instruction words, in order.
fn assemble(lines: &[&str]) -> Vec<u32> {
    let dir = scratch("as");
    let src = dir.join("t.s");
    let mut text = String::from(".option norvc\n");
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    std::fs::write(&src, text).unwrap();
    let obj = dir.join("t.o");
    let bin = dir.join("t.bin");
    let out = Command::new("riscv64-elf-as")
        .args(["-march=rv64gcv", "-o"])
        .arg(&obj)
        .arg(&src)
        .output()
        .expect("riscv64-elf-as — binutils with V is this file's encoding authority");
    assert!(
        out.status.success(),
        "as refused {lines:?}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new("riscv64-elf-objcopy")
        .args(["-O", "binary", "-j", ".text"])
        .arg(&obj)
        .arg(&bin)
        .output()
        .expect("riscv64-elf-objcopy");
    assert!(out.status.success());
    let bytes = std::fs::read(&bin).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(bytes.len(), lines.len() * 4, "one word per line, no RVC");
    bytes
        .chunks(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn word(line: &str) -> u32 {
    assemble(&[line])[0]
}

/// A machine with `words` at `BASE`.
fn machine(words: &[u32]) -> Machine {
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
        // FS and VS Initial: under VS = Off every V instruction is illegal (V-009 (i-c)).
        csr: Csrs {
            sstatus: FS_INITIAL | VS_INITIAL,
            ..Csrs::default()
        },
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

fn step(m: &mut Machine) -> Option<Halt> {
    let mut out = Vec::new();
    m.step(&mut out)
}

/// Run `lines` to completion; every one must retire.
fn run_ok(m: &mut Machine, n: usize) {
    for i in 0..n {
        if let Some(h) = step(m) {
            panic!("instruction {i} halted where it should have run: {h:?}");
        }
    }
}

fn put(m: &mut Machine, addr: u64, values: &[u64]) {
    for (i, v) in values.iter().enumerate() {
        let at = (addr - BASE) as usize + i * 8;
        m.mem[at..at + 8].copy_from_slice(&v.to_le_bytes());
    }
}

fn get(m: &Machine, addr: u64, n: usize) -> Vec<u64> {
    (0..n)
        .map(|i| {
            let at = (addr - BASE) as usize + i * 8;
            u64::from_le_bytes(m.mem[at..at + 8].try_into().unwrap())
        })
        .collect()
}

// ── the decoder, against binutils ────────────────────────────────────────────

#[test]
fn every_subset_mnemonic_decodes_to_the_operation_binutils_assembled() {
    use Src::{F, V, X};
    let table: Vec<(&str, Op)> = vec![
        (
            "vsetvli t0, a0, e64, m1, tu, mu",
            Op::SetVli {
                rd: 5,
                rs1: 10,
                vtypei: 0x18,
            },
        ),
        (
            "vsetvli x0, a1, e64, m8, ta, ma",
            Op::SetVli {
                rd: 0,
                rs1: 11,
                vtypei: 0xdb,
            },
        ),
        (
            "vle64.v v1, (a0)",
            Op::Load {
                vd: 1,
                rs1: 10,
                stride: None,
            },
        ),
        (
            "vlse64.v v2, (a0), a1",
            Op::Load {
                vd: 2,
                rs1: 10,
                stride: Some(11),
            },
        ),
        (
            "vse64.v v3, (a2)",
            Op::Store {
                vs3: 3,
                rs1: 12,
                stride: None,
            },
        ),
        (
            "vsse64.v v4, (a2), a3",
            Op::Store {
                vs3: 4,
                rs1: 12,
                stride: Some(13),
            },
        ),
        (
            "vadd.vv v1, v2, v3",
            Op::Int {
                op: IntOp::Add,
                vd: 1,
                vs2: 2,
                src: V(3),
            },
        ),
        (
            "vadd.vx v1, v2, a0",
            Op::Int {
                op: IntOp::Add,
                vd: 1,
                vs2: 2,
                src: X(10),
            },
        ),
        (
            "vsub.vv v1, v2, v3",
            Op::Int {
                op: IntOp::Sub,
                vd: 1,
                vs2: 2,
                src: V(3),
            },
        ),
        (
            "vsub.vx v1, v2, a0",
            Op::Int {
                op: IntOp::Sub,
                vd: 1,
                vs2: 2,
                src: X(10),
            },
        ),
        (
            "vmul.vv v1, v2, v3",
            Op::Int {
                op: IntOp::Mul,
                vd: 1,
                vs2: 2,
                src: V(3),
            },
        ),
        (
            "vmul.vx v1, v2, a0",
            Op::Int {
                op: IntOp::Mul,
                vd: 1,
                vs2: 2,
                src: X(10),
            },
        ),
        (
            "vfadd.vv v1, v2, v3",
            Op::Fp {
                op: FpOp::Add,
                vd: 1,
                vs2: 2,
                src: V(3),
            },
        ),
        (
            "vfadd.vf v1, v2, fa0",
            Op::Fp {
                op: FpOp::Add,
                vd: 1,
                vs2: 2,
                src: F(10),
            },
        ),
        (
            "vfsub.vv v1, v2, v3",
            Op::Fp {
                op: FpOp::Sub,
                vd: 1,
                vs2: 2,
                src: V(3),
            },
        ),
        (
            "vfsub.vf v1, v2, fa0",
            Op::Fp {
                op: FpOp::Sub,
                vd: 1,
                vs2: 2,
                src: F(10),
            },
        ),
        (
            "vfmul.vv v1, v2, v3",
            Op::Fp {
                op: FpOp::Mul,
                vd: 1,
                vs2: 2,
                src: V(3),
            },
        ),
        (
            "vfmul.vf v1, v2, fa0",
            Op::Fp {
                op: FpOp::Mul,
                vd: 1,
                vs2: 2,
                src: F(10),
            },
        ),
        (
            "vfdiv.vv v1, v2, v3",
            Op::Fp {
                op: FpOp::Div,
                vd: 1,
                vs2: 2,
                src: V(3),
            },
        ),
        (
            "vfdiv.vf v1, v2, fa0",
            Op::Fp {
                op: FpOp::Div,
                vd: 1,
                vs2: 2,
                src: F(10),
            },
        ),
        (
            "vfredosum.vs v1, v2, v3",
            Op::RedOSum {
                vd: 1,
                vs2: 2,
                vs1: 3,
            },
        ),
    ];
    let lines: Vec<&str> = table.iter().map(|(l, _)| *l).collect();
    let words = assemble(&lines);
    let mut wrong = Vec::new();
    for ((line, want), w) in table.iter().zip(&words) {
        let got = vector::decode(*w);
        if got != Ok(*want) {
            wrong.push(format!(
                "{line} = {w:#010x}: decoded {got:?}, want {want:?}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} disagree:\n{}",
        wrong.len(),
        table.len(),
        wrong.join("\n")
    );
}

// ── refusals, by name ────────────────────────────────────────────────────────

/// Each line must halt `Unimplemented` on a configured machine AND decode to `why`.
fn assert_refused(cases: &[(&str, &str)]) {
    let setup = word("vsetvli t0, x0, e64, m1, tu, mu");
    let mut wrong = Vec::new();
    for (line, why) in cases {
        let w = word(line);
        let mut m = machine(&[setup, w]);
        m.x[10] = DATA;
        run_ok(&mut m, 1);
        let h = step(&mut m);
        let named = vector::decode(w).err();
        if !matches!(h, Some(Halt::Unimplemented { word, .. }) if word == w) || named != Some(*why)
        {
            wrong.push(format!(
                "{line} ({w:#010x}): halt {h:?}, named {named:?}, want {why:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_masked_form_halts_naming_the_reserved_mask_register() {
    assert_refused(&[
        ("vadd.vv v1, v2, v3, v0.t", vector::REFUSE_MASKED),
        ("vadd.vx v1, v2, a0, v0.t", vector::REFUSE_MASKED),
        ("vmul.vv v1, v2, v3, v0.t", vector::REFUSE_MASKED),
        ("vfadd.vf v1, v2, fa0, v0.t", vector::REFUSE_MASKED),
        ("vfdiv.vv v1, v2, v3, v0.t", vector::REFUSE_MASKED),
        ("vfredosum.vs v1, v2, v3, v0.t", vector::REFUSE_MASKED),
        ("vle64.v v1, (a0), v0.t", vector::REFUSE_MASKED),
        ("vlse64.v v1, (a0), a1, v0.t", vector::REFUSE_MASKED),
        ("vse64.v v1, (a0), v0.t", vector::REFUSE_MASKED),
        ("vsse64.v v1, (a0), a1, v0.t", vector::REFUSE_MASKED),
        // vm = 0 is also how v0 is named as a merge or carry operand
        ("vmerge.vvm v1, v2, v3, v0", vector::REFUSE_MASKED),
    ]);
}

#[test]
fn every_form_outside_the_subset_halts_by_name() {
    assert_refused(&[
        ("vle32.v v1, (a0)", vector::REFUSE_WIDTH),
        ("vle8.v v1, (a0)", vector::REFUSE_WIDTH),
        ("vse16.v v1, (a0)", vector::REFUSE_WIDTH),
        ("vlse32.v v1, (a0), a1", vector::REFUSE_WIDTH),
        ("vlseg2e64.v v2, (a0)", vector::REFUSE_SEGMENT),
        ("vsseg2e64.v v2, (a0)", vector::REFUSE_SEGMENT),
        ("vluxei64.v v1, (a0), v2", vector::REFUSE_INDEXED),
        ("vloxei64.v v1, (a0), v2", vector::REFUSE_INDEXED),
        ("vsuxei64.v v1, (a0), v2", vector::REFUSE_INDEXED),
        ("vl1re64.v v1, (a0)", vector::REFUSE_UNIT_VARIANT),
        ("vl2re64.v v2, (a0)", vector::REFUSE_UNIT_VARIANT),
        ("vs1r.v v1, (a0)", vector::REFUSE_UNIT_VARIANT),
        ("vle64ff.v v1, (a0)", vector::REFUSE_UNIT_VARIANT),
        ("vadd.vi v1, v2, 3", vector::REFUSE_IMMEDIATE),
        ("vand.vv v1, v2, v3", vector::REFUSE_OP),
        ("vdiv.vv v1, v2, v3", vector::REFUSE_OP),
        ("vrsub.vx v1, v2, a0", vector::REFUSE_OP),
        ("vfrsub.vf v1, v2, fa0", vector::REFUSE_OP),
        ("vfredusum.vs v1, v2, v3", vector::REFUSE_OP),
        ("vfmacc.vv v1, v2, v3", vector::REFUSE_OP),
        // `vsetivli` left this list in V-009 (i-d), and `vsetvl` and every `vtype` in
        // (i-e): all three execute, setting `vill` exactly where QEMU does
        // (`v009e_hardware_agreement.rs`, section 2).
    ]);
}

#[test]
fn a_vector_instruction_before_any_vsetvli_is_illegal_because_vill_is_set_at_reset() {
    // CAUSE 2 HAS TWO SOURCES SINCE V-009 (i-c) — `vill` and `VS = Off` — and this test
    // is about the first. So VS is asserted ON, and the CONTROL is the same instruction
    // after a `vsetvli` clears `vill`: it retires. Without both, a VS-gate cause 2 would
    // pass here for the wrong reason (it did, measured, before this rewrite).
    let w = word("vadd.vv v1, v2, v3");
    let mut m = machine(&[w]);
    assert_eq!(m.vec.vtype, vector::VILL, "reset vtype");
    assert_ne!(
        m.csr.sstatus & SSTATUS_VS,
        0,
        "VS is on: a cause 2 here is vill's"
    );
    assert_eq!(step(&mut m), Some(Halt::Undelivered { pc: BASE, cause: 2 }));
    let mut control = machine(&[word("vsetvli t0, x0, e64, m1, tu, mu"), w]);
    run_ok(&mut control, 2);
}

#[test]
fn a_register_group_not_aligned_to_lmul_is_illegal() {
    let words = assemble(&["vsetvli t0, x0, e64, m2, tu, mu", "vadd.vv v1, v2, v4"]);
    let mut m = machine(&words);
    run_ok(&mut m, 1);
    assert_eq!(
        step(&mut m),
        Some(Halt::Undelivered {
            pc: BASE + 4,
            cause: 2
        })
    );
}

#[test]
fn a_float_vector_op_rounds_by_frm() {
    // Was: a halt under any frm but RNE. Since V-009 (i-f) the subset's vector float arms
    // (vfadd/vfsub/vfmul/vfdiv .vv/.vf, vfredosum.vs; e64 only — vfsqrt, vfmacc and its
    // family, vfcvt/vfncvt and e32 forms still halt Unimplemented) round through the same
    // core in every mode: 1 + 2^-53 (a tie) is 1.0 under RTZ and 1 + ulp
    // under RUP, as QEMU rounds it (`tools/v009-fp-probe`, vfadd.vv case `d:1 + ulp/2 tie`).
    for (frm, want) in [(1u64, 0x3ff0_0000_0000_0000u64), (3, 0x3ff0_0000_0000_0001)] {
        let words = assemble(&["vsetvli t0, x0, e64, m1, tu, mu", "vfadd.vv v1, v2, v3"]);
        let mut m = machine(&words);
        m.fcsr = frm << 5;
        m.vec.v[2] = [1.0f64.to_bits(); 2];
        m.vec.v[3] = [2f64.powi(-53).to_bits(); 2];
        run_ok(&mut m, 2);
        assert_eq!(m.vec.v[1], [want; 2], "frm {frm}");
    }
}

// ── vsetvli ──────────────────────────────────────────────────────────────────

#[test]
fn vsetvli_executes_and_sets_vl_to_the_avl_capped_at_vlmax() {
    // (vtype, avl, expected vl) — VLEN 128 / SEW 64 = 2 per register
    let cases = [
        ("m1", 0, 0),
        ("m1", 1, 1),
        ("m1", 2, 2),
        ("m1", 5, 2),
        ("m2", 3, 3),
        ("m2", 9, 4),
        ("m4", 7, 7),
        ("m8", 100, 16),
    ];
    for (lmul, avl, want) in cases {
        let line = format!("vsetvli t0, a0, e64, {lmul}, tu, mu");
        let mut m = machine(&[word(&line)]);
        m.x[10] = avl;
        run_ok(&mut m, 1);
        assert_eq!(m.x[5], want, "{line} with avl {avl}");
        assert_eq!(m.vec.vl, want);
    }
    // rs1 = x0, rd != x0: AVL is VLMAX
    let mut m = machine(&assemble(&[
        "vsetvli t0, x0, e64, m4, tu, mu",
        "csrr t1, vlenb",
        "csrr t2, vtype",
        "csrr t3, vl",
    ]));
    run_ok(&mut m, 4);
    assert_eq!(m.x[5], 8);
    assert_eq!(m.x[6], 16, "vlenb");
    assert_eq!(m.x[7], 0x1a, "vtype e64 m4 tu mu");
    assert_eq!(m.x[28], 8, "vl");
}

// ── the tail, vstart, and VS ─────────────────────────────────────────────────

#[test]
fn the_tail_past_vl_is_undisturbed() {
    let words = assemble(&[
        "vsetvli t0, a0, e64, m2, tu, mu",
        "vadd.vv v2, v4, v6",
        "vfadd.vv v8, v4, v6",
        "vle64.v v10, (a1)",
        "vfredosum.vs v12, v4, v6",
    ]);
    let mut m = machine(&words);
    m.x[10] = 3; // vl 3 of VLMAX 4: element 3 is tail
    m.x[11] = DATA;
    put(&mut m, DATA, &[7, 7, 7, 7]);
    for r in [2, 3, 8, 9, 10, 11, 12] {
        m.vec.v[r] = [0xdead_0000 + r as u64, 0xbeef_0000 + r as u64];
    }
    m.vec.v[4] = [1, 2];
    m.vec.v[5] = [3, 4];
    m.vec.v[6] = [10, 20];
    m.vec.v[7] = [30, 40];
    run_ok(&mut m, 5);
    assert_eq!(m.vec.v[2], [11, 22]);
    assert_eq!(m.vec.v[3], [33, 0xbeef_0003], "vadd wrote the tail");
    assert_eq!(m.vec.v[9][1], 0xbeef_0009, "vfadd wrote the tail");
    assert_eq!(m.vec.v[11], [7, 0xbeef_000b], "vle64 wrote the tail");
    assert_eq!(
        m.vec.v[12][1], 0xbeef_000c,
        "vfredosum wrote past element 0"
    );
}

#[test]
fn the_ordered_sum_is_ordered() {
    // ((0 + 1e16) + 1) + -1e16 = 0 in order, because 1e16 + 1 rounds back to 1e16;
    // any pairing that adds 1e16 and -1e16 first gives 1.
    let words = assemble(&["vsetvli t0, a0, e64, m2, tu, mu", "vfredosum.vs v1, v2, v1"]);
    let mut m = machine(&words);
    m.x[10] = 3;
    m.vec.v[1] = [0.0f64.to_bits(), 99];
    m.vec.v[2] = [1e16f64.to_bits(), 1.0f64.to_bits()];
    m.vec.v[3] = [(-1e16f64).to_bits(), 5.0f64.to_bits()];
    run_ok(&mut m, 2);
    assert_eq!(f64::from_bits(m.vec.v[1][0]), 0.0);
    assert_eq!(m.vec.v[1][1], 99);
}

#[test]
fn vl_zero_writes_nothing_not_even_the_sums_element_zero() {
    let words = assemble(&[
        "vsetvli t0, a0, e64, m1, tu, mu",
        "vfredosum.vs v1, v2, v3",
        "vadd.vv v4, v2, v3",
    ]);
    let mut m = machine(&words);
    m.vec.v[1] = [123, 456];
    m.vec.v[4] = [789, 1011];
    m.vec.v[3] = [1.0f64.to_bits(), 0];
    run_ok(&mut m, 3);
    assert_eq!(m.vec.v[1], [123, 456]);
    assert_eq!(m.vec.v[4], [789, 1011]);
}

#[test]
fn a_load_that_stops_part_way_reports_the_element_in_vstart() {
    // element 0 is the last octets of RAM; element 1 is past it
    let words = assemble(&["vsetvli t0, x0, e64, m1, tu, mu", "vle64.v v1, (a0)"]);
    let mut m = machine(&words);
    let last = BASE + m.mem.len() as u64 - 8;
    put(&mut m, last, &[0x1234]);
    m.x[10] = last;
    m.vec.v[1] = [0, 0x5a5a];
    run_ok(&mut m, 1);
    assert!(matches!(step(&mut m), Some(Halt::BeyondRam { .. })));
    assert_eq!(m.vec.vstart, 1);
    assert_eq!(m.vec.v[1], [0x1234, 0x5a5a]);
}

#[test]
fn vstart_is_honoured_and_cleared() {
    let words = assemble(&[
        "vsetvli t0, x0, e64, m1, tu, mu",
        "csrwi vstart, 1",
        "vadd.vv v1, v2, v3",
    ]);
    let mut m = machine(&words);
    m.vec.v[1] = [77, 77];
    m.vec.v[2] = [1, 2];
    m.vec.v[3] = [10, 20];
    run_ok(&mut m, 3);
    assert_eq!(m.vec.v[1], [77, 22], "element 0 is below vstart");
    assert_eq!(m.vec.vstart, 0);
}

#[test]
fn sstatus_keeps_vs() {
    let words = assemble(&["csrs sstatus, t0", "csrr t1, sstatus"]);
    let mut m = machine(&words);
    m.x[5] = SSTATUS_VS;
    run_ok(&mut m, 2);
    assert_eq!(
        m.x[6] & SSTATUS_VS,
        SSTATUS_VS,
        "VS was dropped by SSTATUS_MASK"
    );
}

#[test]
fn vector_state_marks_vs_dirty() {
    for line in [
        "vsetvli t0, x0, e64, m1, tu, mu",
        "vle64.v v1, (a0)",
        "vadd.vx v1, v2, a0",
        "vfmul.vf v1, v2, fa0",
        "vfredosum.vs v1, v2, v3",
        "csrwi vstart, 0",
    ] {
        let setup = word("vsetvli t0, x0, e64, m1, tu, mu");
        let mut m = machine(&[setup, word(line)]);
        m.x[10] = DATA;
        run_ok(&mut m, 1);
        m.csr.sstatus = (m.csr.sstatus & !SSTATUS_VS) | VS_INITIAL;
        run_ok(&mut m, 1);
        assert_eq!(
            m.csr.sstatus & SSTATUS_VS,
            SSTATUS_VS,
            "{line} left VS clean"
        );
    }
}

#[test]
fn a_vector_store_does_not_mark_vs_dirty() {
    let words = assemble(&[
        "vsetvli t0, x0, e64, m1, tu, mu",
        "vse64.v v1, (a0)",
        "vsse64.v v1, (a0), a1",
    ]);
    let mut m = machine(&words);
    m.x[10] = DATA;
    m.x[11] = 16;
    m.vec.v[1] = [5, 6];
    run_ok(&mut m, 1);
    m.csr.sstatus = (m.csr.sstatus & !SSTATUS_VS) | VS_INITIAL;
    run_ok(&mut m, 2);
    assert_eq!(m.csr.sstatus & SSTATUS_VS, VS_INITIAL);
    assert_eq!(
        get(&m, DATA, 3),
        vec![5, 6, 6],
        "the unit store, then the strided one over it"
    );
}

// ── V-009 part (i-c): THE VS GATE ────────────────────────────────────────────
//
// With `sstatus.VS = Off` every V instruction — `vsetvli`, the arithmetic, vector loads
// and stores, and the vector CSRs — is an ILLEGAL INSTRUCTION (cause 2), and a vector
// FLOAT instruction additionally needs `sstatus.FS`, as on QEMU.

#[test]
fn vsetvli_with_vs_off_is_an_illegal_instruction() {
    let mut m = machine(&[word("vsetvli t0, x0, e64, m1, tu, mu")]);
    m.csr.sstatus &= !SSTATUS_VS;
    assert_eq!(step(&mut m), Some(Halt::Undelivered { pc: BASE, cause: 2 }));
    assert_eq!(
        m.vec.vtype,
        vector::VILL,
        "vtype unchanged by the faulting vsetvli"
    );
    assert_eq!(m.csr.sstatus & SSTATUS_VS, 0, "and VS not marked dirty");
}

#[test]
fn a_vector_load_and_store_with_vs_off_are_illegal_instructions() {
    for line in ["vle64.v v1, (a0)", "vse64.v v1, (a0)"] {
        let mut m = machine(&[word("vsetvli t0, x0, e64, m1, tu, mu"), word(line)]);
        m.x[10] = DATA;
        run_ok(&mut m, 1);
        m.csr.sstatus &= !SSTATUS_VS;
        assert_eq!(
            step(&mut m),
            Some(Halt::Undelivered {
                pc: BASE + 4,
                cause: 2
            }),
            "{line} under VS = Off"
        );
    }
}

#[test]
fn a_vector_float_op_with_vs_on_and_fs_off_is_an_illegal_instruction() {
    let words = assemble(&["vsetvli t0, x0, e64, m1, tu, mu", "vfadd.vv v1, v2, v3"]);
    let mut m = machine(&words);
    run_ok(&mut m, 1);
    m.csr.sstatus &= !FS;
    assert_ne!(m.csr.sstatus & SSTATUS_VS, 0, "VS is on");
    assert_eq!(
        step(&mut m),
        Some(Halt::Undelivered {
            pc: BASE + 4,
            cause: 2
        })
    );
    // The control: the integer op, which needs VS alone, retires under the same sstatus.
    let words = assemble(&["vsetvli t0, x0, e64, m1, tu, mu", "vadd.vv v1, v2, v3"]);
    let mut c = machine(&words);
    run_ok(&mut c, 1);
    c.csr.sstatus &= !FS;
    run_ok(&mut c, 1);
}

#[test]
fn a_vector_csr_read_with_vs_off_is_an_illegal_instruction() {
    for line in [
        "csrr t0, vl",
        "csrr t0, vtype",
        "csrr t0, vlenb",
        "csrr t0, vstart",
    ] {
        let mut m = machine(&[word(line)]);
        m.csr.sstatus &= !SSTATUS_VS;
        assert_eq!(
            step(&mut m),
            Some(Halt::Undelivered { pc: BASE, cause: 2 }),
            "{line} under VS = Off"
        );
        // The control: the same read with VS on retires.
        let mut c = machine(&[word(line)]);
        run_ok(&mut c, 1);
    }
}
