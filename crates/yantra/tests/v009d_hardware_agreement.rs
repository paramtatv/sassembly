//! **`V-009` part (i-d) — yantra against `qemu-system-riscv64`, three gaps closed.**
//!
//! The (i-c) review ran raw programs on `qemu-system-riscv64 -bios none -d int` and found
//! three places where yantra and QEMU disagreed:
//!
//! 1. **The float CSRs** `fflags` (0x001), `frm` (0x002) and `fcsr` (0x003) halted
//!    `Halt::Csr` whatever `sstatus.FS` said. QEMU traps them cause 2 under `FS = Off`
//!    and reads and writes them otherwise.
//! 2. **`vsetivli`** halted by name (`REFUSE_SETVL_FORM`). QEMU executes it.
//! 3. **OP-IMM-32 and OP-32** — `addiw`, `slliw`, `srliw`, `sraiw`, `addw`, `subw`, `sllw`,
//!    `srlw`, `sraw`, and the M extension's `mulw`, `divw`, `divuw`, `remw`, `remuw` — had
//!    no arm at all and halted `Unimplemented`. They are RV64I (and RV64M) base
//!    instructions; `li` of a wide constant expands to `addiw`.
//!
//! **EVERY EXPECTED VALUE BELOW IS QEMU'S, NOT A HAND COMPUTATION.** The probe programs
//! are kept with the builder (`<builder>/probes/` on
//! A Linux x86-64 host): `probe.S`, `probe2.S`, `probe3.S`, each assembled with
//! `riscv64-elf-as -march=rv64gcv`, linked at `0x8000_0000`, and run as
//!
//! ```text
//! qemu-system-riscv64 -machine virt -cpu rv64,v=true,vlen=128,elen=64 \
//!     -nographic -bios none -kernel probeN.elf -d int -D qemu-int.probeN.log
//! ```
//!
//! (QEMU 10.1.0, Debian 1:10.1.0+ds-5ubuntu2.7). The probe runs in M-mode with a trap
//! handler that records `0xdead_0000 | mcause` and resumes at `mepc + 4`; every value is
//! printed one hex dword per line (`qemu-out.*.txt`), and the `-d int` log names each
//! trap's `epc`, `tval` and cause. Each comment below cites the probe case by its label
//! in the `.S` file (`B4`, `C3`, …). `sstatus.FS`/`VS` are the same bits as `mstatus`'s,
//! so an M-mode probe answers the S-mode question this machine asks.
//!
//! **Words are assembled by binutils, never written by hand** — the same rule as
//! `vector_extension.rs`.

use std::path::PathBuf;
use std::process::Command;

use yantra::{Csrs, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;
const FS: u64 = 0b11 << 13;
const FS_INITIAL: u64 = 0b01 << 13;
const VS: u64 = 0b11 << 9;
const VS_INITIAL: u64 = 0b01 << 9;

fn scratch() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "v009d-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Assemble `lines` with binutils and return the words, in order.
fn assemble(lines: &[&str]) -> Vec<u32> {
    let dir = scratch();
    let (src, obj, bin) = (dir.join("t.s"), dir.join("t.o"), dir.join("t.bin"));
    let mut text = String::from(".option norvc\n");
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    std::fs::write(&src, text).unwrap();
    let out = Command::new("riscv64-elf-as")
        .args(["-march=rv64gcv", "-o"])
        .arg(&obj)
        .arg(&src)
        .output()
        .expect("riscv64-elf-as — binutils is this file's encoding authority");
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

/// A machine with `words` at `BASE`, FS and VS Initial — what the startup sets (i-b).
fn machine(words: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX,
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        socket: None,
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: BASE,
        base: BASE,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: Csrs {
            sstatus: FS_INITIAL | VS_INITIAL,
            ..Csrs::default()
        },
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
    };
    for (i, w) in words.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

/// Assemble `lines`, let `prep` set registers, and step until every line has retired or
/// one halts. The halt, if any, is returned beside the machine.
fn run(lines: &[&str], prep: impl FnOnce(&mut Machine)) -> (Machine, Option<Halt>) {
    let mut m = machine(&assemble(lines));
    prep(&mut m);
    let mut out = Vec::new();
    for _ in 0..lines.len() {
        if let Some(h) = m.step(&mut out) {
            return (m, Some(h));
        }
    }
    (m, None)
}

/// [`run`], and every line must retire.
fn run_ok(lines: &[&str], prep: impl FnOnce(&mut Machine)) -> Machine {
    let (m, h) = run(lines, prep);
    assert!(h.is_none(), "{lines:?} halted where QEMU ran: {h:?}");
    m
}

fn illegal_at(pc: u64) -> Option<Halt> {
    Some(Halt::Undelivered { pc, cause: 2 })
}

const A0: usize = 10;
const T0: usize = 5;

// ── 1. THE FLOAT CSRs ────────────────────────────────────────────────────────

#[test]
fn the_float_csrs_under_fs_off_are_illegal_instructions() {
    // QEMU probe.S A1..A6: each of the six traps cause 2 (`-d int`: tval 0x00102573,
    // 0x00202573, 0x00302573, 0x0010d073, 0x0020d073, then csrw fcsr), and mstatus.FS
    // reads 0 after them (A, the RECFS line).
    for line in [
        "csrr a0, fflags",
        "csrr a0, frm",
        "csrr a0, fcsr",
        "csrwi fflags, 1",
        "csrwi frm, 1",
        "csrw fcsr, t0",
    ] {
        let (m, h) = run(&[line], |m| {
            m.csr.sstatus &= !FS;
            m.x[T0] = 0xff;
            m.x[A0] = 0x1234;
        });
        assert_eq!(h, illegal_at(BASE), "{line} under FS = Off (QEMU: cause 2)");
        assert_eq!(m.csr.sstatus & FS, 0, "{line}: FS stays Off");
        assert_eq!(m.x[A0], 0x1234, "{line}: rd not written");
        assert_eq!(m.fcsr, 0, "{line}: fcsr not written");
    }
}

#[test]
fn a_float_csr_read_under_fs_initial_reads_and_does_not_dirty_fs() {
    // QEMU probe.S B1: fcsr reads 0. B2: FS after the pure read is 1 (Initial).
    let m = run_ok(&["csrr a0, fcsr"], |m| m.x[A0] = 0x1234);
    assert_eq!(m.x[A0], 0);
    assert_eq!(
        m.csr.sstatus & FS,
        FS_INITIAL,
        "a pure read leaves FS Initial"
    );
    // And the three are windows on one register: fcsr 0x45 (frm 2, fflags 5).
    let m = run_ok(&["csrr a0, fcsr", "csrr a1, fflags", "csrr a2, frm"], |m| {
        m.fcsr = 0x45
    });
    assert_eq!([m.x[10], m.x[11], m.x[12]], [0x45, 0x05, 0x02]);
}

#[test]
fn a_float_csr_write_dirties_fs_even_when_the_value_is_unchanged() {
    // QEMU probe.S B3: `csrwi fflags, 0` on fflags = 0 leaves FS = 3 (Dirty).
    for line in [
        "csrwi fflags, 0",
        "csrwi frm, 0",
        "csrw fcsr, x0",
        "csrsi fflags, 1",
    ] {
        let m = run_ok(&[line], |_| {});
        assert_eq!(m.csr.sstatus & FS, FS, "{line} must set FS Dirty");
    }
}

#[test]
fn float_csr_writes_keep_only_the_legal_bits() {
    // QEMU probe.S, FS Initial, t0 = -1 throughout:
    //   B4  csrw fflags, t0      -> fflags 0x1f, fcsr 0x1f
    //   B5  csrw fcsr, x0; csrw frm, t0 -> frm 7, fcsr 0xe0
    //   B6  csrw fcsr, t0        -> fcsr 0xff, fflags 0x1f, frm 7
    //   B7  csrrw a0, fcsr, x0   -> old 0xff, then fcsr 0
    //   B8  csrsi fflags,5; csrci fflags,1 -> fflags 4
    //   B9  csrwi frm, 3         -> fcsr 0x64 (fflags kept)
    //   B10 csrw fcsr, 0x7ff     -> fcsr 0xff: bits 10:8 (the draft-V vxrm/vxsat
    //       mirror) do not stick on QEMU with V on or off.
    let m = run_ok(
        &["csrw fflags, t0", "csrr a0, fflags", "csrr a1, fcsr"],
        |m| {
            m.x[T0] = u64::MAX;
        },
    );
    assert_eq!([m.x[10], m.x[11]], [0x1f, 0x1f], "B4");

    let m = run_ok(&["csrw frm, t0", "csrr a0, frm", "csrr a1, fcsr"], |m| {
        m.x[T0] = u64::MAX;
    });
    assert_eq!([m.x[10], m.x[11]], [7, 0xe0], "B5");

    let m = run_ok(
        &[
            "csrw fcsr, t0",
            "csrr a0, fcsr",
            "csrr a1, fflags",
            "csrr a2, frm",
            "csrrw a3, fcsr, x0",
            "csrr a4, fcsr",
        ],
        |m| m.x[T0] = u64::MAX,
    );
    assert_eq!(
        [m.x[10], m.x[11], m.x[12], m.x[13], m.x[14]],
        [0xff, 0x1f, 7, 0xff, 0],
        "B6, B7"
    );

    let m = run_ok(
        &[
            "csrsi fflags, 5",
            "csrci fflags, 1",
            "csrr a0, fflags",
            "csrwi frm, 3",
            "csrr a1, fcsr",
        ],
        |_| {},
    );
    assert_eq!([m.x[10], m.x[11]], [4, 0x64], "B8, B9");

    let m = run_ok(&["csrw fcsr, t0", "csrr a0, fcsr"], |m| m.x[T0] = 0x7ff);
    assert_eq!(m.x[A0], 0xff, "B10");
}

#[test]
fn the_float_csrs_are_user_level_and_readable_from_u_mode() {
    // QEMU probe.S U: mstatus.MPP = U, mret, then from U-mode `csrr a0, fcsr` reads 0x45
    // (U1), `csrwi fflags, 2; csrr a1, fflags` reads 2 (U2), `csrr a2, vl` reads 2 (U3),
    // and `csrr a3, sstatus` traps cause 2 (`-d int`: epc of that word). probe2.S U1..U3:
    // vstart 0, vtype 0x18, vlenb 0x10 from U-mode. CSR numbers 0x0__ and 0xc2_ are
    // unprivileged by their bits 9:8; `sstatus` (0x100) is not.
    let lines = [
        "csrr a0, fcsr",
        "csrwi fflags, 2",
        "csrr a1, fflags",
        "csrr a2, vl",
        "csrr a3, vstart",
        "csrr a4, vtype",
        "csrr a5, vlenb",
        "csrr a6, sstatus",
    ];
    let (m, h) = run(&lines, |m| {
        m.mode = Privilege::User;
        m.fcsr = 0x45;
        m.vec.vl = 2;
        m.vec.vtype = 0x18;
        m.x[16] = 0x1234;
    });
    assert_eq!(h, illegal_at(BASE + 28), "sstatus from U-mode is cause 2");
    assert_eq!(
        [m.x[10], m.x[11], m.x[12], m.x[13], m.x[14], m.x[15]],
        [0x45, 2, 2, 0, 0x18, 0x10]
    );
    assert_eq!(m.x[16], 0x1234);
}

// ── fflags ACCRUAL — what matches QEMU, and the gap that does not ────────────

/// Run `op` on f1 = `a`, f2 = `b` from fcsr = 0 and FS Initial; answer (fflags, FS) as
/// read after the op, the order probe.S's FLAGS_AFTER reads them.
fn flags_after(op: &str, a: u64, b: u64, x5: u64) -> (u64, u64) {
    let m = run_ok(&[op, "csrr a0, fflags"], |m| {
        m.f[1] = a;
        m.f[2] = b;
        m.x[T0] = x5;
    });
    (m.x[A0], (m.csr.sstatus & FS) >> 13)
}

const ONE: u64 = 0x3ff0_0000_0000_0000;
const SNAN: u64 = 0x7ff4_0000_0000_0000;
const QNAN: u64 = 0x7ff8_0000_0000_0000;

/// A case name, the op, f1, f2, t0, and QEMU's (fflags, FS).
type FlagCase = (&'static str, &'static str, u64, u64, u64, (u64, u64));

#[test]
fn fflags_read_back_what_qemu_accrues_for_nv_dz_and_the_conversions() {
    // probe.S section C, each case from fcsr 0 with FS reset to Initial just before the
    // op; QEMU's (fflags, FS) per case:
    let cases: [FlagCase; 9] = [
        ("C1 1/0: DZ", "fdiv.d f3, f1, f2", ONE, 0, 0, (0x08, 3)),
        (
            "C2 sqrt(-1): NV",
            "fsqrt.d f3, f1",
            0xbff0_0000_0000_0000,
            0,
            0,
            (0x10, 3),
        ),
        // FS STAYS INITIAL on QEMU when an op only accrues flags and writes x — C6, C7,
        // C8. The flags are still read back.
        (
            "C6 feq sNaN: NV",
            "feq.d a1, f1, f2",
            SNAN,
            ONE,
            0,
            (0x10, 1),
        ),
        (
            "C7 flt qNaN: NV",
            "flt.d a1, f1, f2",
            QNAN,
            ONE,
            0,
            (0x10, 1),
        ),
        (
            "C8 fcvt.w.d 1.5: NX",
            "fcvt.w.d a1, f1, rne",
            0x3ff8_0000_0000_0000,
            0,
            0,
            (0x01, 1),
        ),
        (
            "C9 1+1 exact: none",
            "fadd.d f3, f1, f2",
            ONE,
            ONE,
            0,
            (0x00, 3),
        ),
        (
            "C11 fcvt.d.l 2^53+1: NX",
            "fcvt.d.l f3, t0",
            0,
            0,
            0x20_0000_0000_0001,
            (0x01, 3),
        ),
        (
            "C13 fmin sNaN: NV",
            "fmin.d f3, f1, f2",
            SNAN,
            ONE,
            0,
            (0x10, 3),
        ),
        (
            "C14 fsgnj: none",
            "fsgnj.d f3, f1, f2",
            SNAN,
            ONE,
            0,
            (0x00, 3),
        ),
    ];
    let mut wrong = Vec::new();
    for (name, op, a, b, x5, qemu) in cases {
        let got = flags_after(op, a, b, x5);
        if got != qemu {
            wrong.push(format!("{name}: yantra {got:x?}, QEMU {qemu:x?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

// The NX/OF/UF pin that stood here asserts QEMU's column now, with the (i-e) sweep:
// `v009e_hardware_agreement.rs`, `the_former_pins_read_qemus_flags`.

// ── frm and the rounding-mode field ──────────────────────────────────────────

#[test]
fn a_dynamic_to_int_conversion_rounds_by_frm() {
    // QEMU probe3.S Q1..Q5: fcvt.w.d dyn under frm = RTZ 1.5 -> 1, RDN -1.5 -> -2,
    // RUP 1.5 -> 2, RMM 2.5 -> 3, RNE 2.5 -> 2.
    for (frm, x, want) in [
        (1u64, 0x3ff8_0000_0000_0000u64, 1u64),
        (2, 0xbff8_0000_0000_0000, (-2i64) as u64),
        (3, 0x3ff8_0000_0000_0000, 2),
        (4, 0x4004_0000_0000_0000, 3),
        (0, 0x4004_0000_0000_0000, 2),
    ] {
        let m = run_ok(&["csrw frm, t0", "fcvt.w.d a0, f1, dyn"], |m| {
            m.x[T0] = frm;
            m.f[1] = x;
        });
        assert_eq!(m.x[A0], want, "frm {frm}");
    }
}

#[test]
fn an_invalid_rounding_mode_is_an_illegal_instruction() {
    // QEMU probe.S D3, D5..D8, D15 (frm = 5 or 7 under `dyn`) and D11..D14 (a static
    // rm of 5 or 6): every one traps cause 2. probe2.S P1, P2: vfadd.vv and
    // vfredosum.vs under frm = 5 trap cause 2 too.
    let one = ONE;
    for (frm, op) in [
        (5u64, "fadd.d f3, f1, f2, dyn"),
        (5, "fcvt.w.d a0, f1, dyn"),
        (5, ".insn r 0x53, 7, 0x69, f3, x0, x0"), // fcvt.d.w f3, x0, dyn
        (5, ".insn r 0x53, 7, 0x21, f3, f1, f0"), // fcvt.d.s f3, f1, dyn
        (5, "fmadd.d f3, f1, f1, f2, dyn"),
        (7, "fadd.d f3, f1, f2, dyn"),
        (0, ".insn r 0x53, 5, 0x01, f3, f1, f2"), // fadd.d rm = 5
        (0, ".insn r 0x53, 6, 0x01, f3, f1, f2"), // fadd.d rm = 6
        (0, ".insn r 0x53, 5, 0x61, a0, f1, f0"), // fcvt.w.d rm = 5
        (0, ".insn r 0x53, 5, 0x2d, f3, f1, f0"), // fsqrt.d rm = 5
    ] {
        let (m, h) = run(&["csrw frm, t0", op], |m| {
            m.x[T0] = frm;
            m.f[1] = one;
            m.f[2] = one;
        });
        assert_eq!(
            h,
            illegal_at(BASE + 4),
            "{op} under frm {frm} (QEMU: cause 2)"
        );
        assert_eq!(m.f[3], 0, "{op}: nothing written");
    }
    for op in ["vfadd.vv v1, v2, v3", "vfredosum.vs v1, v2, v3"] {
        let (_, h) = run(
            &["vsetivli t1, 2, e64, m1, tu, mu", "csrwi frm, 5", op],
            |_| {},
        );
        assert_eq!(h, illegal_at(BASE + 8), "{op} under frm 5 (QEMU: cause 2)");
    }
}

#[test]
fn an_invalid_frm_does_not_touch_an_instruction_without_a_rounding_mode() {
    // QEMU probe.S D4: fadd.d with a STATIC rne under frm = 5 runs, 0x3ff0000000000002.
    // D9 fsgnj.d and D10 feq.d (whose funct3 is not an rm) run; feq answers 0.
    // probe2.S P3: vadd.vv runs.
    let m = run_ok(
        &[
            "csrwi frm, 5",
            "fadd.d f3, f1, f2, rne",
            "fsgnj.d f4, f1, f2",
            "feq.d a0, f1, f2",
            "vsetivli t1, 2, e64, m1, tu, mu",
            "vadd.vv v1, v2, v3",
        ],
        |m| {
            m.f[1] = ONE;
            m.f[2] = 0x3cb8_0000_0000_0000; // 1.5 * 2^-52, one and a half ulps of 1.0
            m.x[A0] = 7;
        },
    );
    assert_eq!(m.f[3], 0x3ff0_0000_0000_0002);
    assert_eq!(m.x[A0], 0);
}

// ── 2. vsetivli ──────────────────────────────────────────────────────────────

#[test]
fn vsetivli_under_vs_off_is_an_illegal_instruction() {
    // QEMU probe.S V1: cause 2.
    let (m, h) = run(&["vsetivli t0, 5, e64, m1, ta, ma"], |m| {
        m.csr.sstatus &= !VS;
    });
    assert_eq!(h, illegal_at(BASE));
    assert_eq!(m.vec.vtype, yantra::vector::VILL);
}

#[test]
fn vsetivli_takes_its_avl_from_the_immediate() {
    // QEMU probe.S V2..V9 (VLEN 128, so VLMAX = 2 * LMUL at e64):
    //   V2 vsetivli t0, 5, e64, m1, ta, ma -> t0 2, vl 2, vtype 0xd8; V3 VS -> 3
    //   V4 1, m1 -> 1        V5 0, m1 -> t0 0, vl 0      V6 31, m8 -> 16
    //   V7 vsetivli x0, 3, e64, m2 -> vl 3, vtype 0x19   V8 7, m4 -> 7
    //   V9 vstart 1, then vsetivli -> vstart 0
    let m = run_ok(
        &[
            "vsetivli t0, 5, e64, m1, ta, ma",
            "csrr a0, vl",
            "csrr a1, vtype",
        ],
        |m| m.x[T0] = 0x7777,
    );
    assert_eq!([m.x[T0], m.x[10], m.x[11]], [2, 2, 0xd8], "V2");
    assert_eq!(m.csr.sstatus & VS, VS, "V3: VS Dirty");

    for (line, want) in [
        ("vsetivli t0, 1, e64, m1, tu, mu", 1u64),
        ("vsetivli t0, 0, e64, m1, tu, mu", 0),
        ("vsetivli t0, 31, e64, m8, tu, mu", 16),
        ("vsetivli t0, 7, e64, m4, tu, mu", 7),
    ] {
        let m = run_ok(&[line, "csrr a0, vl"], |m| m.x[T0] = 0x7777);
        assert_eq!([m.x[T0], m.x[A0]], [want, want], "{line}");
    }

    let m = run_ok(
        &[
            "vsetivli x0, 3, e64, m2, tu, mu",
            "csrr a0, vl",
            "csrr a1, vtype",
        ],
        |_| {},
    );
    assert_eq!(
        [m.x[10], m.x[11]],
        [3, 0x19],
        "V7: rd = x0 still takes the immediate"
    );

    let m = run_ok(
        &[
            "csrwi vstart, 1",
            "vsetivli t0, 2, e64, m1, tu, mu",
            "csrr a0, vstart",
        ],
        |_| {},
    );
    assert_eq!(m.x[A0], 0, "V9");
}

// ── 3. OP-IMM-32 and OP-32 ───────────────────────────────────────────────────

#[test]
fn the_word_forms_compute_what_qemu_computes() {
    // QEMU probe.S W1..W33. Each row: the instruction, t1, t2, and QEMU's t0.
    #[rustfmt::skip]
    let cases: [(&str, u64, u64, u64); 31] = [
        ("addiw t0, t1, 1",   0x7fff_ffff,            0, 0xffff_ffff_8000_0000), // W1
        ("addiw t0, t1, -1",  0,                      0, u64::MAX),              // W2
        ("addiw t0, t1, 0",   0x1_2345_6789,          0, 0x2345_6789),           // W3 sext.w
        ("addiw t0, t1, 1",   0xffff_ffff,            0, 0),                     // W4
        ("addiw t0, t1, 0",   0x1_8000_0000,          0, 0xffff_ffff_8000_0000), // W5
        ("slliw t0, t1, 31",  1,                      0, 0xffff_ffff_8000_0000), // W6
        ("slliw t0, t1, 2",   0x1_0000_0003,          0, 0xc),                   // W7
        ("srliw t0, t1, 4",   0xffff_ffff_8000_0000,  0, 0x0800_0000),           // W8
        ("srliw t0, t1, 0",   u64::MAX,               0, u64::MAX),              // W9
        ("sraiw t0, t1, 4",   0x8000_0000,            0, 0xffff_ffff_f800_0000), // W10
        ("sraiw t0, t1, 31",  0x1_7fff_ffff,          0, 0),                     // W11
        ("srliw t0, t1, 31",  0x8000_0000,            0, 1),                     // W12
        ("addw t0, t1, t2",   0x7fff_ffff,            1, 0xffff_ffff_8000_0000), // W15
        ("subw t0, t1, t2",   0,                      1, u64::MAX),              // W16
        ("subw t0, t1, t2",   0x8000_0000,            1, 0x7fff_ffff),           // W17
        ("sllw t0, t1, t2",   1,                     33, 2),                     // W18
        ("srlw t0, t1, t2",   u64::MAX,              36, 0x0fff_ffff),           // W19
        ("sraw t0, t1, t2",   0x8000_0000,            1, 0xffff_ffff_c000_0000), // W20
        ("mulw t0, t1, t2",   0x10000,          0x10000, 0),                     // W21
        ("mulw t0, t1, t2",   0xffff_ffff,            2, (-2i64) as u64),        // W22
        ("divw t0, t1, t2",   (-7i64) as u64,         2, (-3i64) as u64),        // W23
        ("remw t0, t1, t2",   (-7i64) as u64,         2, u64::MAX),              // W24
        ("divw t0, t1, t2",   (-7i64) as u64,         0, u64::MAX),              // W25
        ("remw t0, t1, t2",   (-7i64) as u64,         0, (-7i64) as u64),        // W26
        ("divw t0, t1, t2",   0x8000_0000,     u64::MAX, 0xffff_ffff_8000_0000), // W27
        ("remw t0, t1, t2",   0x8000_0000,     u64::MAX, 0),                     // W28
        ("divuw t0, t1, t2",  0xffff_ffff,            2, 0x7fff_ffff),           // W29
        ("divuw t0, t1, t2",  0xffff_ffff,            0, u64::MAX),              // W30
        ("remuw t0, t1, t2",  0xffff_ffff,            0, u64::MAX),              // W31
        ("remuw t0, t1, t2",  7,                      3, 1),                     // W32
        ("divw t0, t1, t2",   0x1_0000_0007, 0x5_0000_0003, 2),                  // W33
    ];
    let mut wrong = Vec::new();
    for (line, t1, t2, qemu) in cases {
        let (m, h) = run(&[line], |m| {
            m.x[6] = t1;
            m.x[7] = t2;
        });
        if h.is_some() || m.x[T0] != qemu {
            wrong.push(format!(
                "{line} t1={t1:#x} t2={t2:#x}: yantra {:#x} ({h:?}), QEMU {qemu:#x}",
                m.x[T0]
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

// The reserved word-form pin that stood here is an agreement test now (`V-009` (i-e)):
// `v009e_hardware_agreement.rs`, `the_former_word_form_pins_trap_cause_2`.

// ── 4. THE FIXED-POINT CSRs ──────────────────────────────────────────────────
//
// They halted by name here until `V-009` (i-e); they are QEMU's now — storage, gated on
// VS, served to U-mode: `v009e_hardware_agreement.rs`, section 4.

// ── THE (i-d) REVIEW: three more disagreements, fixed ────────────────────────
//
// The review's own probes (2,190 values) found 85 disagreements; three are fixed here,
// the rest pinned in `v009e_pins.rs`. Expected values are from `probe4.S` and `probe5.S`
// (generated by `gen4.py`/`gen5.py`, same harness and same QEMU as above); each row's
// label is the probe's, and `probe4.annotated.tsv`/`probe5.annotated.tsv` pair every
// recorded dword with its label.

const SD: u64 = 1 << 63;

#[test]
fn sstatus_sd_reads_set_exactly_when_fs_or_vs_is_dirty() {
    // probe4 S0..S5, the bit-63 column of `csrr sstatus` on QEMU:
    //   S0 FS, VS Initial                     -> 0
    //   S1 after `csrwi fflags, 0`            -> 1   (0x8000000200006200)
    //   S2 after FS written Dirty directly    -> 1
    //   S3 after `vsetivli`                   -> 1   (0x8000000200002600)
    //   S4 after VS written Dirty directly    -> 1
    //   S5 after `csrs sstatus` of bit 63     -> 0   (SD is read-only, computed)
    // SD is how a kernel learns, in one bit, that there is live float or vector state to
    // save; a yantra that never set it told such a kernel to skip the save.
    let cases: [(&str, &[&str], u64); 6] = [
        ("S0", &[], 0),
        ("S1", &["csrwi fflags, 0"], SD),
        ("S2", &["csrs sstatus, t0"], SD),
        ("S3", &["vsetivli t1, 2, e64, m1, tu, mu"], SD),
        ("S4", &["csrs sstatus, t1"], SD),
        ("S5", &["csrs sstatus, t2"], 0),
    ];
    for (name, lines, want) in cases {
        let mut all: Vec<&str> = lines.to_vec();
        all.push("csrr a0, sstatus");
        let m = run_ok(&all, |m| {
            m.x[5] = 0x6000;
            m.x[6] = 0x600;
            m.x[7] = SD;
        });
        assert_eq!(m.x[A0] & SD, want, "{name}: sstatus = {:#x}", m.x[A0]);
    }
}

#[test]
fn csrrs_and_csrrc_write_only_when_their_mask_is_non_zero() {
    // QEMU's rule, measured (probe4 R1..R15, probe5 T1..T3): csrrw/csrrwi always write;
    // csrrs/csrrc and their immediates write — and so dirty FS or VS — only when the
    // MASK VALUE is non-zero. `rs1 = t0` holding 0 is not a write (R5, R6, R9, R10, R13,
    // R14); a non-zero mask that changes nothing is (T1..T3).
    //   form                        t0  FS/VS after (1 Initial, 3 Dirty)
    let cases: [(&str, &str, u64, u64); 18] = [
        ("R1", "csrrs a0, fflags, x0", 0, 1),
        ("R2", "csrrc a0, fflags, x0", 0, 1),
        ("R3", "csrrsi a0, fflags, 0", 0, 1),
        ("R4", "csrrci a0, fflags, 0", 0, 1),
        ("R5", "csrrs a0, fflags, t0", 0, 1),
        ("R6", "csrrc a0, fflags, t0", 0, 1),
        ("R7", "csrrw a0, fflags, x0", 0, 3),
        ("R8", "csrrwi a0, fflags, 0", 0, 3),
        ("R9", "csrrs a0, fcsr, t0", 0, 1),
        ("R10", "csrrs a0, frm, t0", 0, 1),
        ("T1", "csrrs a0, fflags, t0", 1, 3),
        ("T2", "csrrsi a0, fflags, 1", 0, 3),
        ("T3", "csrrc a0, fflags, t0", 2, 3),
        ("R11", "csrrs a0, vstart, x0", 0, 1),
        ("R12", "csrrsi a0, vstart, 0", 0, 1),
        ("R13", "csrrs a0, vstart, t0", 0, 1),
        ("R14", "csrrc a0, vstart, t0", 0, 1),
        ("R15", "csrrwi a0, vstart, 0", 0, 3),
    ];
    let mut wrong = Vec::new();
    for (name, line, t0, qemu) in cases {
        let m = run_ok(&[line], |m| {
            m.x[T0] = t0;
            m.fcsr = 1; // T1, T2: fflags already holds bit 0
        });
        let shift = if line.contains("vstart") { 9 } else { 13 };
        let got = (m.csr.sstatus >> shift) & 3;
        if got != qemu {
            wrong.push(format!(
                "{name} `{line}` t0={t0}: yantra {got}, QEMU {qemu}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_read_only_csr_is_illegal_under_any_rs1_but_x0() {
    // The OTHER half of the rule is the register NUMBER, not its value (probe5 T4..T10):
    // `csrrs time, x0` and `csrrsi time, 0` read; `csrrs time, t0` with t0 = 0,
    // `csrrc time, t0`, `csrrs vl, t0`, `csrrc vlenb, t0`, `csrrsi vl, 1` are cause 2.
    for (line, illegal) in [
        ("csrrs a0, time, x0", false),
        ("csrrsi a0, time, 0", false),
        ("csrrs a0, time, t0", true),
        ("csrrc a0, time, t0", true),
        ("csrrs a0, vl, t0", true),
        ("csrrc a0, vlenb, t0", true),
        ("csrrsi a0, vl, 1", true),
    ] {
        let (_, h) = run(&[line], |m| m.x[T0] = 0);
        if illegal {
            assert_eq!(h, illegal_at(BASE), "{line}");
        } else {
            assert_eq!(h, None, "{line}");
        }
    }
}

#[test]
fn every_float_to_int_conversion_agrees_with_qemu() {
    // 660 conversions — fcvt.{w,wu,l,lu}.{s,d} x rm 0..4 x 33 inputs, chosen for the
    // saturating, rounding-out-of-range and NaN/inf cases — on QEMU (probe4 section N,
    // `tests/data/v009d_cvt_qemu.tsv`). The review's three named failures are rows here:
    // `fcvt.w.d rne` of 2147483647.5 (NV alone, 0x10 — yantra said 0x11), `fcvt.wu.d rtz`
    // of -1.5, `fcvt.lu.s rdn` of -0.5. A SATURATED result raises NV ALONE.
    let table = include_str!("data/v009d_cvt_qemu.tsv");
    let ops = ["w", "wu", "l", "lu"];
    // Every word binutils' way: one assembly of the 40 forms.
    let mut lines = Vec::new();
    for fmt in ["d", "s"] {
        for op in ops {
            for rm in ["rne", "rtz", "rdn", "rup", "rmm"] {
                lines.push(format!("fcvt.{op}.{fmt} a0, f1, {rm}"));
            }
        }
    }
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let words = assemble(&refs);
    let mut wrong = Vec::new();
    let mut n = 0;
    for line in table.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        let hex = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
        let (fmt, op, rm) = (f[0], f[1], f[2].parse::<usize>().unwrap());
        let (input, qemu, qflags) = (hex(f[3]), hex(f[4]), hex(f[5]));
        let k = ops.iter().position(|o| *o == op).unwrap();
        let w = words[usize::from(fmt == "s") * 20 + k * 5 + rm];
        let mut m = machine(&[w]);
        m.f[1] = if fmt == "s" {
            0xffff_ffff_0000_0000 | input
        } else {
            input
        };
        let mut out = Vec::new();
        let h = m.step(&mut out);
        if h.is_some() || m.x[A0] != qemu || m.fflags() != qflags {
            wrong.push(format!(
                "fcvt.{op}.{fmt} rm={rm} of {input:#x}: yantra {:#x} flags {:#x} ({h:?}); \
                 QEMU {qemu:#x} flags {qflags:#x}",
                m.x[A0],
                m.fflags()
            ));
        }
        n += 1;
    }
    assert_eq!(n, 660, "the table is the whole probe");
    assert!(
        wrong.is_empty(),
        "{} of {n} disagree:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
