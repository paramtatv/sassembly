//! **`V-009` part (i-e) — yantra AGREES with `qemu-system-riscv64`: the (i-d) pins, closed.**
//!
//! Owner, scheduling (i-e): *"Pinning inexact/overflow/underflow flags, reserved encodings,
//! and sstatus.UXL with explicit test cases completes hardware-exact VM compliance before
//! finalizing the V-009 series."* Each section below was a PIN — in `v009e_pins.rs`
//! (deleted when its last pin closed) or in `v009d_hardware_agreement.rs` — holding
//! yantra's old answer beside QEMU's; here QEMU's answer is the assertion.
//!
//! **EVERY EXPECTED VALUE IS QEMU'S.** The probes are kept with the (i-e) builder at
//! `<builder>/probes/` on a Linux x86-64 host — the generators
//! (`gen_fp.py`, `gen_ctl.py`, `gen_w.py`, on the shared `harness.py`), the programs, the
//! raw QEMU output (`qemu-out.*.txt`), the `-d int` logs, and the annotated pairings —
//! each assembled with `riscv64-elf-as -march=rv64gcv`, linked at `0x8000_0000`
//! (`--no-relax`), and run as
//!
//! ```text
//! qemu-system-riscv64 -machine virt -cpu rv64,v=true,vlen=128,elen=64 \
//!     -nographic -bios none -kernel probe_X.elf -d int -D qemu-int.probe_X.log
//! ```
//!
//! (QEMU 10.1.0, Debian 1:10.1.0+ds-5ubuntu2.7). The bulk results are committed in compact
//! form under `tests/data/v009e_*.tsv`, each file's header naming the probe it came from.

use std::path::PathBuf;
use std::process::Command;

use yantra::{Csrs, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;
const FS_INITIAL: u64 = 0b01 << 13;
const VS_INITIAL: u64 = 0b01 << 9;

fn scratch() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "v009e-agree-{}-{}",
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
        patra_mem: None,
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
    load(&mut m, words);
    m
}

fn load(m: &mut Machine, words: &[u32]) {
    for (i, w) in words.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m.pc = BASE;
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

fn data(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or_else(|e| panic!("{s:?}: {e}"))
}

// ═════════════════════════════════════════════════════════════════════════════════
// 1. NX / OF / UF — every rounding float op yantra executes: scalar in both widths, and
//    the vector subset (vfadd/vfsub/vfmul/vfdiv .vv/.vf, vfredosum.vs) at e64 only
// ═════════════════════════════════════════════════════════════════════════════════
//
// `probe_fp.S` (`gen_fp.py`, seed 0x5a55e7b1): 2,104 inputs — 104 NAMED edges, then 2,000
// from boundary-seeking generators (cancellation, the tininess boundary of `×` and `÷`,
// the overflow threshold of `×` and `+`, fma residuals landing subnormal, exact small
// integers, subnormal arithmetic, doubles aimed at single precision's edges, integers at
// both mantissa boundaries) and plain random bits — each a double triple, a single triple
// and an integer, in `tests/data/v009e_fp_inputs.tsv`. Every one of 37 operations ran
// over all of them under `frm` = 0..4 (the instruction's rm is DYN): 778,480 recorded
// dwords; `probe_fp_static.S` (V-009 (i-f)) ran the 25 operations with an rm field again
// with the mode in the instruction and a DIFFERENT mode in frm, 526,000 dwords — QEMU's
// static results equal its dyn ones in all 125 (op, mode) pairs.
//
// HASHED SINCE (i-f) (owner ruling (c)): `tests/data/v009e_fp_qemu_hashes.tsv` keeps, per
// (operation, mode, form), an FNV-1a-64 of the results and one of the flags (16 KB; (i-e)
// kept every case's flags, 395 KB). A mismatch names the op and mode; to name the CASE,
// run `tools/v009-fp-probe/regenerate.sh DIR` on a host with QEMU and rerun with
// `V009_FP_RESULTS=DIR` — every hashed test then lists the differing cases.

/// The operations of the sweep, in the probe's order, with the assembly the machine runs.
/// `kind`: `d` double, `s` single, `ds` fcvt.s.d, `sd` fcvt.d.s, `i2d`/`i2s` integer to
/// float, `v` an element-wise vector op, `r` the ordered reduction.
const OPS: [(&str, &str, &str); 37] = [
    ("fadd.d", "d", "fadd.d f4, f1, f2, dyn"),
    ("fsub.d", "d", "fsub.d f4, f1, f2, dyn"),
    ("fmul.d", "d", "fmul.d f4, f1, f2, dyn"),
    ("fdiv.d", "d", "fdiv.d f4, f1, f2, dyn"),
    ("fsqrt.d", "d", "fsqrt.d f4, f1, dyn"),
    ("fmadd.d", "d", "fmadd.d f4, f1, f2, f3, dyn"),
    ("fmsub.d", "d", "fmsub.d f4, f1, f2, f3, dyn"),
    ("fnmsub.d", "d", "fnmsub.d f4, f1, f2, f3, dyn"),
    ("fnmadd.d", "d", "fnmadd.d f4, f1, f2, f3, dyn"),
    ("fcvt.s.d", "ds", "fcvt.s.d f4, f1, dyn"),
    ("fadd.s", "s", "fadd.s f4, f1, f2, dyn"),
    ("fsub.s", "s", "fsub.s f4, f1, f2, dyn"),
    ("fmul.s", "s", "fmul.s f4, f1, f2, dyn"),
    ("fdiv.s", "s", "fdiv.s f4, f1, f2, dyn"),
    ("fsqrt.s", "s", "fsqrt.s f4, f1, dyn"),
    ("fmadd.s", "s", "fmadd.s f4, f1, f2, f3, dyn"),
    ("fmsub.s", "s", "fmsub.s f4, f1, f2, f3, dyn"),
    ("fnmsub.s", "s", "fnmsub.s f4, f1, f2, f3, dyn"),
    ("fnmadd.s", "s", "fnmadd.s f4, f1, f2, f3, dyn"),
    ("fcvt.d.s", "sd", "fcvt.d.s f4, f1"),
    ("fcvt.d.l", "i2d", "fcvt.d.l f4, t1, dyn"),
    ("fcvt.d.lu", "i2d", "fcvt.d.lu f4, t1, dyn"),
    ("fcvt.d.w", "i2d", "fcvt.d.w f4, t1"),
    ("fcvt.d.wu", "i2d", "fcvt.d.wu f4, t1"),
    ("fcvt.s.l", "i2s", "fcvt.s.l f4, t1, dyn"),
    ("fcvt.s.lu", "i2s", "fcvt.s.lu f4, t1, dyn"),
    ("fcvt.s.w", "i2s", "fcvt.s.w f4, t1, dyn"),
    ("fcvt.s.wu", "i2s", "fcvt.s.wu f4, t1, dyn"),
    ("vfadd.vv", "v", "vfadd.vv v3, v2, v1"),
    ("vfsub.vv", "v", "vfsub.vv v3, v2, v1"),
    ("vfmul.vv", "v", "vfmul.vv v3, v2, v1"),
    ("vfdiv.vv", "v", "vfdiv.vv v3, v2, v1"),
    ("vfadd.vf", "v", "vfadd.vf v3, v2, f2"),
    ("vfsub.vf", "v", "vfsub.vf v3, v2, f2"),
    ("vfmul.vf", "v", "vfmul.vf v3, v2, f2"),
    ("vfdiv.vf", "v", "vfdiv.vf v3, v2, f2"),
    ("vfredosum.vs", "r", "vfredosum.vs v3, v2, v1"),
];

/// One input case of the sweep.
struct Case {
    name: String,
    d: [u64; 3],
    s: [u64; 3],
    i: u64,
}

fn cases() -> Vec<Case> {
    data("v009e_fp_inputs.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            Case {
                name: c[0].to_string(),
                d: [hex(c[1]), hex(c[2]), hex(c[3])],
                s: [hex(c[4]), hex(c[5]), hex(c[6])],
                i: hex(c[7]),
            }
        })
        .collect()
}

/// QEMU's answer for one (operation, mode, form): FNV-1a-64 of the results and of the
/// flags, over every case in input order.
struct Expect {
    op: String,
    rm: u32,
    /// `dyn` (rm field DYN, frm = rm) or `static` (rm field = rm, frm = (rm + 2) % 5).
    form: String,
    results: u64,
    flags: u64,
}

fn expectations() -> Vec<Expect> {
    data("v009e_fp_qemu_hashes.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            Expect {
                op: c[0].to_string(),
                rm: c[1].parse().unwrap(),
                form: c[2].to_string(),
                results: hex(c[3]),
                flags: hex(c[4]),
            }
        })
        .collect()
}

/// (form, op, rm) -> every case's (result, flags), as QEMU recorded them.
type PerCase = std::collections::HashMap<(String, String, u32), Vec<(u64, u64)>>;

/// QEMU's per-case results, when `V009_FP_RESULTS` names a directory that
/// `tools/v009-fp-probe/regenerate.sh` wrote: (form, op, rm) -> [(result, flags)].
fn qemu_per_case() -> Option<PerCase> {
    let dir = PathBuf::from(std::env::var_os("V009_FP_RESULTS")?);
    let mut m = std::collections::HashMap::new();
    for f in ["fp_results.tsv", "fp_results_static.tsv"] {
        let text = std::fs::read_to_string(dir.join(f))
            .unwrap_or_else(|e| panic!("V009_FP_RESULTS: {}: {e}", dir.join(f).display()));
        for l in text.lines() {
            let c: Vec<&str> = l.split('\t').collect();
            m.entry((c[0].to_string(), c[1].to_string(), c[2].parse().unwrap()))
                .or_insert_with(Vec::new)
                .push((hex(c[5]), hex(c[6])));
        }
    }
    Some(m)
}

/// Compare one (op, mode, form)'s per-case answers with QEMU's hashes; on a mismatch,
/// say so — and NAME the differing cases when [`qemu_per_case`] has them.
fn judge(
    e: &Expect,
    got: &[(u64, u64)],
    cases: &[Case],
    per_case: Option<&PerCase>,
    who: &str,
) -> Vec<String> {
    let (mut hr, mut hf) = (Fnv::new(), 0xcbf2_9ce4_8422_2325u64);
    for &(r, f) in got {
        hr.add(r);
        hf = (hf ^ f).wrapping_mul(0x0000_0100_0000_01b3);
    }
    if (hr.0, hf) == (e.results, e.flags) {
        return Vec::new();
    }
    let what = match (hr.0 == e.results, hf == e.flags) {
        (false, false) => "results and flags",
        (false, true) => "results",
        _ => "flags",
    };
    let mut out = vec![format!(
        "{} rm {} ({}): {who}'s {what} hash differs from QEMU's",
        e.op, e.rm, e.form
    )];
    let key = (e.form.clone(), e.op.clone(), e.rm);
    match per_case.and_then(|m| m.get(&key)) {
        Some(q) => {
            for (i, (&(r, f), &(qr, qf))) in got.iter().zip(q).enumerate() {
                if (r, f) != (qr, qf) && out.len() < 11 {
                    let c = &cases[i];
                    out.push(format!(
                        "  case {i} ({}) d {:x?} s {:x?} i {:x}: {who} {r:#x} {}, QEMU {qr:#x} {}",
                        c.name,
                        c.d,
                        c.s,
                        c.i,
                        flag_names(f),
                        flag_names(qf)
                    ));
                }
            }
        }
        None => out.push(
            "  to name the case: tools/v009-fp-probe/regenerate.sh DIR (needs QEMU), then \
             rerun with V009_FP_RESULTS=DIR"
                .into(),
        ),
    }
    out
}

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn add(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

fn narrow(kind: &str) -> bool {
    matches!(kind, "s" | "ds" | "i2s")
}

/// Run one sweep case through the MACHINE: registers set as the probe set them, the one
/// instruction stepped, the result and the flags read back.
fn machine_case(
    m: &mut Machine,
    word: u32,
    frm: u32,
    kind: &str,
    c: &Case,
) -> Result<(u64, u64), Halt> {
    load(m, &[word]);
    m.fcsr = u64::from(frm) << 5; // fflags clear
    match kind {
        "s" | "sd" => {
            for (r, v) in c.s.iter().enumerate() {
                m.f[r + 1] = 0xffff_ffff_0000_0000 | v;
            }
        }
        "i2d" | "i2s" => m.x[6] = c.i,
        _ => {
            for (r, v) in c.d.iter().enumerate() {
                m.f[r + 1] = *v;
            }
        }
    }
    match kind {
        "v" => {
            m.vec.vtype = 0x18;
            m.vec.vl = 1;
            m.vec.v[2][0] = c.d[0];
            m.vec.v[1][0] = c.d[1];
        }
        "r" => {
            m.vec.vtype = 0x18;
            m.vec.vl = 2;
            m.vec.v[2] = [c.d[0], c.d[1]];
            m.vec.v[1][0] = c.d[2];
        }
        _ => {}
    }
    m.vec.vstart = 0;
    if let Some(h) = m.step(&mut Vec::new()) {
        return Err(h);
    }
    let r = match kind {
        "v" | "r" => m.vec.v[3][0],
        k if narrow(k) => m.f[4] & 0xffff_ffff,
        _ => m.f[4],
    };
    Ok((r, m.fflags()))
}

fn flag_names(f: u64) -> String {
    let n: Vec<&str> = [(16, "NV"), (8, "DZ"), (4, "OF"), (2, "UF"), (1, "NX")]
        .iter()
        .filter(|(b, _)| f & b != 0)
        .map(|&(_, n)| n)
        .collect();
    if n.is_empty() {
        "-".into()
    } else {
        n.join("|")
    }
}

/// The rounding modes the MACHINE runs the sweep under: all five since V-009 (i-f)
/// (owner ruling (b): "Allow rounding modes beyond round-to-nearest-even (RNE)").
const MACHINE_MODES: &[u32] = &[0, 1, 2, 3, 4];

const RM_NAMES: [&str; 5] = ["rne", "rtz", "rdn", "rup", "rmm"];

#[test]
fn the_float_sweep_agrees_with_qemu_through_the_machine() {
    // The machine path: every case of every operation under [`MACHINE_MODES`], in both
    // forms — rm DYN with frm = the mode, and (for ops with an rm field) the mode in the
    // instruction with frm = (mode + 2) % 5. The words are binutils'.
    let cases = cases();
    let per_case = qemu_per_case();
    let mut m = machine(&[]);
    let mut wrong = Vec::new();
    let mut checked = 0;
    for e in expectations()
        .iter()
        .filter(|e| MACHINE_MODES.contains(&e.rm))
    {
        let (_, kind, asm) = OPS.iter().find(|(o, _, _)| *o == e.op).unwrap();
        let (line, frm) = if e.form == "static" {
            let line = format!("{}{}", &asm[..asm.len() - 3], RM_NAMES[e.rm as usize]);
            (line, (e.rm + 2) % 5)
        } else {
            (asm.to_string(), e.rm)
        };
        let word = assemble(&[&line])[0];
        let mut got = Vec::with_capacity(cases.len());
        for (i, c) in cases.iter().enumerate() {
            match machine_case(&mut m, word, frm, kind, c) {
                Ok(v) => got.push(v),
                Err(h) => {
                    wrong.push(format!(
                        "{line} (frm {frm}) case {i} ({}): halted {h:?}",
                        c.name
                    ));
                    break;
                }
            }
        }
        checked += got.len();
        if got.len() == cases.len() {
            wrong.extend(judge(e, &got, &cases, per_case.as_ref(), "yantra"));
        }
    }
    let rows = expectations()
        .iter()
        .filter(|e| MACHINE_MODES.contains(&e.rm))
        .count();
    assert_eq!(
        rows,
        62 * MACHINE_MODES.len(),
        "37 dyn + 25 static ops per mode"
    );
    assert!(
        wrong.is_empty(),
        "{} disagreements with QEMU ({checked} cases run):\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

fn box_s(v: f32) -> u64 {
    0xffff_ffff_0000_0000 | u64::from(v.to_bits())
}

/// (what, op, f1..f3, x6, QEMU's result bits, QEMU's fflags).
type Edge = (&'static str, &'static str, [u64; 3], u64, u64, u64);

#[test]
fn the_named_edges_read_qemus_flags() {
    // Hand-picked rows of `fp_results.tsv` (case name as in `v009e_fp_inputs.tsv`), RNE,
    // stated here so the claims read without the sweep: each is (what, op, f1, f2, f3, x6,
    // QEMU's result bits, QEMU's fflags).
    let d = |v: f64| v.to_bits();
    let maxsub_d = 0x000f_ffff_ffff_ffffu64;
    let rows: [Edge; 16] = [
        // AFTER-ROUNDING TININESS: 2^-1022 (1 - 2^-104) rounds to 2^-1022 at 53 bits — NX
        // alone. Before-rounding detection would add UF.
        (
            "tininess witness",
            "fmul.d f4, f1, f2, dyn",
            [d(1.0) + 1, maxsub_d, 0],
            0,
            0x0010_0000_0000_0000,
            0x01,
        ),
        (
            "tininess witness, single",
            "fmul.s f4, f1, f2, dyn",
            [box_s(1.0) + 1, 0xffff_ffff_007f_ffff, 0],
            0,
            0x0080_0000,
            0x01,
        ),
        (
            "fcvt.s.d 2^-126 (1 - 2^-25): rounds up to minnorm, not tiny",
            "fcvt.s.d f4, f1, dyn",
            [0x380f_ffff_f000_0000, 0, 0],
            0,
            0x0080_0000,
            0x01,
        ),
        (
            "fcvt.s.d 2^-126 (1 - 2^-24): tiny after rounding",
            "fcvt.s.d f4, f1, dyn",
            [0x380f_ffff_e000_0000, 0, 0],
            0,
            0x0080_0000,
            0x03,
        ),
        (
            "DBL_MAX * 2 overflows to inf",
            "fmul.d f4, f1, f2, dyn",
            [0x7fef_ffff_ffff_ffff, d(2.0), 0],
            0,
            0x7ff0_0000_0000_0000,
            0x05,
        ),
        (
            "DBL_MAX + half an ulp: the tie goes to inf",
            "fadd.d f4, f1, f2, dyn",
            [0x7fef_ffff_ffff_ffff, 0x7c90_0000_0000_0000, 0],
            0,
            0x7ff0_0000_0000_0000,
            0x05,
        ),
        (
            "DBL_MAX + a quarter ulp stays finite",
            "fadd.d f4, f1, f2, dyn",
            [0x7fef_ffff_ffff_ffff, 0x7c80_0000_0000_0000, 0],
            0,
            0x7fef_ffff_ffff_ffff,
            0x01,
        ),
        (
            "minsub * 1.5: a tie between subnormals",
            "fmul.d f4, f1, f2, dyn",
            [1, d(1.5), 0],
            0,
            2,
            0x03,
        ),
        (
            "minsub / 2 underflows to zero",
            "fdiv.d f4, f1, f2, dyn",
            [1, d(2.0), 0],
            0,
            0,
            0x03,
        ),
        (
            "3 * 0.5 is exact: no NX",
            "fmul.d f4, f1, f2, dyn",
            [d(3.0), d(0.5), 0],
            0,
            d(1.5),
            0,
        ),
        (
            "inf / 0: an exact infinity, no DZ",
            "fdiv.d f4, f1, f2, dyn",
            [0x7ff0_0000_0000_0000, 0, 0],
            0,
            0x7ff0_0000_0000_0000,
            0,
        ),
        (
            "1 / 0: DZ",
            "fdiv.d f4, f1, f2, dyn",
            [d(1.0), 0, 0],
            0,
            0x7ff0_0000_0000_0000,
            0x08,
        ),
        (
            "inf * 0 + qNaN: NV even with a quiet NaN addend",
            "fmadd.d f4, f1, f2, f3, dyn",
            [0x7ff0_0000_0000_0000, 0, 0x7ff8_0000_0000_0000],
            0,
            0x7ff8_0000_0000_0000,
            0x10,
        ),
        (
            "1 * 1 + sNaN: NV from the addend",
            "fmadd.d f4, f1, f2, f3, dyn",
            [d(1.0), d(1.0), 0x7ff0_0000_0000_0001],
            0,
            0x7ff8_0000_0000_0000,
            0x10,
        ),
        (
            "fma residual lands subnormal",
            "fmadd.d f4, f1, f2, f3, dyn",
            [
                0x3ff0_0000_0000_0001,
                0x0020_0000_0000_0001,
                0x8020_0000_0000_0002,
            ],
            0,
            0,
            0x03,
        ),
        (
            "fcvt.s.w 2^24 + 1 rounds",
            "fcvt.s.w f4, t1, dyn",
            [0; 3],
            (1 << 24) + 1,
            0x4b80_0000,
            0x01,
        ),
    ];
    let mut wrong = Vec::new();
    for (what, op, f, x6, res, flags) in rows {
        let m = run_ok(&[op], |m| {
            m.f[1..4].copy_from_slice(&f);
            m.x[6] = x6;
        });
        let r = if op.contains(".s ") || op.starts_with("fcvt.s") || op.starts_with("fmul.s") {
            m.f[4] & 0xffff_ffff
        } else {
            m.f[4]
        };
        if (r, m.fflags()) != (res, flags) {
            wrong.push(format!(
                "{what}: yantra {r:#x} {}, QEMU {res:#x} {}",
                flag_names(m.fflags()),
                flag_names(flags)
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_former_pins_read_qemus_flags() {
    // `v009e_pins.rs`'s and `v009d_hardware_agreement.rs`'s NX/OF/UF pins, now asserting
    // QEMU's column (probe4 section P, probe5, and probe.S C3..C12 of the (i-d) builder).
    let d = |v: f64| v.to_bits();
    let cases: [(&str, &str, u64, u64, u64); 15] = [
        (
            "C3 fadd.d 1 + 2^-60",
            "fadd.d f3, f1, f2",
            d(1.0),
            0x3c30_0000_0000_0000,
            0x01,
        ),
        (
            "C4 fmul.d DBL_MAX * 2",
            "fmul.d f3, f1, f2",
            0x7fef_ffff_ffff_ffff,
            d(2.0),
            0x05,
        ),
        (
            "C5 fmul.d 2^-1000 * 2^-100",
            "fmul.d f3, f1, f2",
            0x0170_0000_0000_0000,
            0x39b0_0000_0000_0000,
            0x03,
        ),
        ("C10 fdiv.d 1/3", "fdiv.d f3, f1, f2", d(1.0), d(3.0), 0x01),
        (
            "C12 fcvt.s.d 0.1",
            "fcvt.s.d f3, f1",
            0x3fb9_9999_9999_999a,
            0,
            0x01,
        ),
        (
            "fsub.d 1 - 2^-60",
            "fsub.d f3, f1, f2",
            d(1.0),
            d(2f64.powi(-60)),
            0x01,
        ),
        ("fsqrt.d 2", "fsqrt.d f3, f1", d(2.0), 0, 0x01),
        (
            "fmadd.d 1*1 + 2^-60",
            "fmadd.d f3, f1, f1, f2",
            d(1.0),
            d(2f64.powi(-60)),
            0x01,
        ),
        (
            "fadd.s 1 + 2^-30",
            "fadd.s f3, f1, f2",
            box_s(1.0),
            box_s(2f32.powi(-30)),
            0x01,
        ),
        (
            "fsub.s 1 - 2^-30",
            "fsub.s f3, f1, f2",
            box_s(1.0),
            box_s(2f32.powi(-30)),
            0x01,
        ),
        (
            "fmul.s FLT_MAX * 2",
            "fmul.s f3, f1, f2",
            box_s(f32::MAX),
            box_s(2.0),
            0x05,
        ),
        (
            "fmul.s (1+2^-23)2^-100 * 2^-40",
            "fmul.s f3, f1, f2",
            box_s((1.0 + 2f32.powi(-23)) * 2f32.powi(-100)),
            box_s(2f32.powi(-40)),
            0x03,
        ),
        (
            "fdiv.s 1/3",
            "fdiv.s f3, f1, f2",
            box_s(1.0),
            box_s(3.0),
            0x01,
        ),
        ("fsqrt.s 2", "fsqrt.s f3, f1", box_s(2.0), 0, 0x01),
        (
            "fmadd.s 1*1 + 2^-30",
            "fmadd.s f3, f1, f1, f2",
            box_s(1.0),
            box_s(2f32.powi(-30)),
            0x01,
        ),
    ];
    let mut wrong = Vec::new();
    for (name, op, a, b, qemu) in cases {
        let m = run_ok(&[op], |m| {
            m.f[1] = a;
            m.f[2] = b;
        });
        if m.fflags() != qemu {
            wrong.push(format!("{name}: yantra {:#x}, QEMU {qemu:#x}", m.fflags()));
        }
    }
    // The vector arm: vfdiv.vv 1/3 — QEMU 0x01 (probe4 section P).
    let m = run_ok(
        &["vsetivli t1, 2, e64, m1, tu, mu", "vfdiv.vv v1, v2, v3"],
        |m| {
            m.vec.v[2] = [d(1.0); 2];
            m.vec.v[3] = [d(3.0); 2];
        },
    );
    if m.fflags() != 0x01 {
        wrong.push(format!("vfdiv.vv 1/3: yantra {:#x}, QEMU 0x1", m.fflags()));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The core's answer for one sweep case under `rm`, as the probe computed it.
fn core_case(op: &str, kind: &str, c: &Case, rm: u32) -> (u64, u64) {
    use yantra::fp::{self, F32, F64};
    let [a, b, cc] = c.d;
    let [sa, sb, sc] = c.s;
    let fused = |f, x, y, z| {
        let (np, na) = match &op[..op.len() - 2] {
            "fmadd" => (false, false),
            "fmsub" => (false, true),
            "fnmsub" => (true, false),
            _ => (true, true),
        };
        fp::fma(f, x, y, z, np, na, rm)
    };
    let base = op.split('.').next().unwrap();
    match (kind, base) {
        ("d" | "v", "fadd" | "vfadd") => fp::add(F64, a, b, rm),
        ("d" | "v", "fsub" | "vfsub") => fp::sub(F64, a, b, rm),
        ("d" | "v", "fmul" | "vfmul") => fp::mul(F64, a, b, rm),
        ("d" | "v", "fdiv" | "vfdiv") => fp::div(F64, a, b, rm),
        ("d", "fsqrt") => fp::sqrt(F64, a, rm),
        ("d", _) => fused(F64, a, b, cc),
        ("s", "fadd") => fp::add(F32, sa, sb, rm),
        ("s", "fsub") => fp::sub(F32, sa, sb, rm),
        ("s", "fmul") => fp::mul(F32, sa, sb, rm),
        ("s", "fdiv") => fp::div(F32, sa, sb, rm),
        ("s", "fsqrt") => fp::sqrt(F32, sa, rm),
        ("s", _) => fused(F32, sa, sb, sc),
        ("ds", _) => fp::convert(F64, F32, a, rm),
        ("sd", _) => fp::convert(F32, F64, sa, rm),
        ("r", _) => {
            let (x, f1) = fp::add(F64, cc, a, rm);
            let (y, f2) = fp::add(F64, x, b, rm);
            (y, f1 | f2)
        }
        _ => {
            let f = if kind == "i2s" { F32 } else { F64 };
            let v = match op.rsplit('.').next().unwrap() {
                "l" => i128::from(c.i as i64),
                "lu" => i128::from(c.i),
                "w" => i128::from(c.i as i32),
                _ => i128::from(c.i as u32),
            };
            fp::from_int(f, v, rm)
        }
    }
}

#[test]
fn the_rounding_core_agrees_with_qemu_in_all_five_modes() {
    // THE CORE, NOT THE MACHINE: the rounding core ([`yantra::fp::add`] and its kin) over
    // the whole dyn sweep — 37 operations x 2,104 inputs x RNE, RTZ, RDN, RUP, RMM =
    // 389,240 cases — against QEMU's hashes: overflow to inf or to the largest finite by
    // mode and sign, ties to max magnitude, directed rounding into and out of the
    // subnormal range.
    let cases = cases();
    let per_case = qemu_per_case();
    let mut wrong = Vec::new();
    let mut n = 0;
    for e in expectations().iter().filter(|e| e.form == "dyn") {
        let (_, kind, _) = OPS.iter().find(|(o, _, _)| *o == e.op).unwrap();
        let got: Vec<(u64, u64)> = cases
            .iter()
            .map(|c| core_case(&e.op, kind, c, e.rm))
            .collect();
        n += got.len();
        wrong.extend(judge(e, &got, &cases, per_case.as_ref(), "core"));
    }
    assert_eq!(n, 37 * 5 * 2104);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// (op, f1, f2, QEMU's (result, fflags) under RNE, RTZ, RDN, RUP, RMM).
type ModeRow = (&'static str, u64, u64, [(u64, u64); 5]);

#[test]
fn arithmetic_runs_under_every_rounding_mode_static_or_dynamic() {
    // V-001's refusal is LIFTED (V-009 (i-f), owner ruling (b)). QEMU's readings (rows of
    // the regenerated `fp_results.tsv` / `fp_results_static.tsv`, identical in both
    // forms): (op, f1, f2, [result, fflags] under RNE, RTZ, RDN, RUP, RMM).
    let one = 0x3ff0_0000_0000_0000u64;
    let rows: [ModeRow; 3] = [
        // 1 + 2^-53, an exact tie: to even (down) under RNE, away under RMM, up under RUP.
        (
            "fadd.d",
            one,
            0x3ca0_0000_0000_0000,
            [(one, 1), (one, 1), (one, 1), (one + 1, 1), (one + 1, 1)],
        ),
        // -DBL_MAX * 2: -inf or -DBL_MAX by mode and sign.
        (
            "fmul.d",
            0xffef_ffff_ffff_ffff,
            0x4000_0000_0000_0000,
            [
                (0xfff0_0000_0000_0000, 5),
                (0xffef_ffff_ffff_ffff, 5),
                (0xfff0_0000_0000_0000, 5),
                (0xffef_ffff_ffff_ffff, 5),
                (0xfff0_0000_0000_0000, 5),
            ],
        ),
        // The tininess witness: rounding DOWN makes it tiny after rounding (UF).
        (
            "fmul.d",
            one + 1,
            0x000f_ffff_ffff_ffff,
            [
                (0x0010_0000_0000_0000, 1),
                (0x000f_ffff_ffff_ffff, 3),
                (0x000f_ffff_ffff_ffff, 3),
                (0x0010_0000_0000_0000, 1),
                (0x0010_0000_0000_0000, 1),
            ],
        ),
    ];
    for (op, a, b, want) in rows {
        for (rm, name) in RM_NAMES.iter().enumerate() {
            for (line, frm) in [
                (format!("{op} f4, f1, f2, {name}"), (rm as u64 + 2) % 5),
                (format!("{op} f4, f1, f2, dyn"), rm as u64),
            ] {
                let m = run_ok(&[&line], |m| {
                    m.f[1] = a;
                    m.f[2] = b;
                    m.fcsr = frm << 5;
                });
                assert_eq!((m.f[4], m.fflags()), want[rm], "{line} (frm {frm})");
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════════
// 2. Reserved encodings: the word forms, vtype and vill
// ═════════════════════════════════════════════════════════════════════════════════

fn undelivered(pc: u64) -> Option<Halt> {
    Some(Halt::Undelivered { pc, cause: 2 })
}

/// The Zba / Zbb word forms QEMU's `rv64` CPU executes and this machine does not
/// implement: (opcode, funct3, bits 31:25, the rs2 values). They halt by name.
fn zb_word_form(w: u32) -> bool {
    let (op, f3, f7, rs2) = (w & 0x7f, (w >> 12) & 7, w >> 25, (w >> 20) & 0x1f);
    match op {
        0x1b => {
            matches!((f3, f7), (1, 0x04 | 0x05) | (5, 0x30)) || ((f3, f7) == (1, 0x30) && rs2 <= 2)
        }
        _ => {
            matches!(
                (f3, f7),
                (0, 0x04) | (1, 0x30) | (2 | 4 | 6, 0x10) | (5, 0x30)
            ) || (f3, f7, rs2) == (4, 0x04, 0)
        }
    }
}

#[test]
fn every_word_form_encoding_does_what_qemu_does() {
    // `probe_w.S`: all 65,536 OP-32 / OP-IMM-32 words (funct3 x bits 31:25 x rs2; rd a0,
    // rs1 t1). `tests/data/v009e_word_qemu.tsv` lists the ones QEMU executes; every other
    // one trapped cause 2 there. Here: a QEMU trap is a cause-2 trap (no handler, so
    // `Undelivered`), and a QEMU execution is an execution — or, for the Zba/Zbb word
    // forms this machine does not implement, a halt BY NAME, never a trap.
    let mut runs = std::collections::HashMap::new();
    for l in data("v009e_word_qemu.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let c: Vec<&str> = l.split('\t').collect();
        let p = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
        runs.insert((p(c[0]), c[1].parse::<u32>().unwrap(), p(c[2])), p(c[3]));
    }
    let mut m = machine(&[]);
    let mut wrong = Vec::new();
    let (mut traps, mut execs, mut named) = (0, 0, 0);
    for opcode in [0x3bu32, 0x1b] {
        for f3 in 0..8 {
            for f7 in 0..128 {
                for rs2 in 0..32 {
                    let w = (f7 << 25) | (rs2 << 20) | (6 << 15) | (f3 << 12) | (10 << 7) | opcode;
                    load(&mut m, &[w]);
                    m.x[6] = 0x1234_5678_9abc_def0;
                    m.x[10] = 0x5a5a;
                    let h = m.step(&mut Vec::new());
                    let qemu_runs = runs
                        .get(&(opcode, f3, f7))
                        .is_some_and(|mask| mask >> rs2 & 1 == 1);
                    let ok = match (qemu_runs, &h) {
                        (false, h) => {
                            traps += 1;
                            *h == undelivered(BASE) && m.x[10] == 0x5a5a
                        }
                        (true, None) => {
                            execs += 1;
                            !zb_word_form(w)
                        }
                        (true, Some(Halt::Unimplemented { word, .. })) => {
                            named += 1;
                            *word == w && zb_word_form(w)
                        }
                        (true, Some(_)) => false,
                    };
                    if !ok && wrong.len() < 30 {
                        wrong.push(format!(
                            "{w:#010x}: QEMU {}, yantra {h:?}",
                            if qemu_runs { "runs" } else { "cause 2" }
                        ));
                    }
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(
        (traps, execs + named),
        (65_536 - 4_804, 4_804),
        "the probe's own split"
    );
}

#[test]
fn the_former_word_form_pins_trap_cause_2() {
    // `v009e_pins.rs` (probe4 section W) and `v009d_hardware_agreement.rs`
    // (`a_reserved_word_form_encoding_still_halts_by_name`, probe.S W13/W14/W34): QEMU
    // traps every one cause 2 and leaves a0 unwritten. So does this machine, with a
    // handler installed: the trap is DELIVERED, `scause` 2, `stval` the word.
    let lines = [
        ".insn i 0x1b, 1, a0, t1, 32",    // slliw, imm[5] = 1
        ".insn i 0x1b, 5, a0, t1, 33",    // srliw, imm[5] = 1
        ".insn i 0x1b, 5, a0, t1, 0x421", // sraiw, imm[5] = 1
        ".insn i 0x1b, 1, a0, t1, 0x401", // slliw, funct7 0x20
        ".insn i 0x1b, 2, a0, t1, 0",
        ".insn i 0x1b, 3, a0, t1, 0",
        ".insn i 0x1b, 4, a0, t1, 0",
        ".insn i 0x1b, 6, a0, t1, 0",
        ".insn i 0x1b, 7, a0, t1, 0",
        ".insn r 0x3b, 0, 0x02, a0, t1, t2",
        ".insn r 0x3b, 1, 0x20, a0, t1, t2",
        ".insn r 0x3b, 2, 0x00, a0, t1, t2",
        ".insn r 0x3b, 3, 0x00, a0, t1, t2",
        ".insn r 0x3b, 4, 0x00, a0, t1, t2",
        ".insn r 0x3b, 6, 0x00, a0, t1, t2",
        ".insn r 0x3b, 7, 0x00, a0, t1, t2",
        ".insn r 0x3b, 2, 0x01, a0, t1, t2",
        ".insn r 0x3b, 3, 0x01, a0, t1, t2",
    ];
    for (line, w) in lines.iter().zip(assemble(&lines)) {
        let (m, h) = run(&[line], |m| {
            m.csr.stvec = BASE + 0x100;
            m.x[10] = 0x5a5a;
        });
        assert_eq!(h, None, "{line}: delivered, not halted");
        assert_eq!(
            (m.pc, m.csr.scause, m.csr.stval, m.csr.sepc, m.x[10]),
            (BASE + 0x100, 2, u64::from(w), BASE, 0x5a5a),
            "{line}"
        );
    }
}

/// Step `w` after an e64 m1 setup (vl 2), with t0 = 0x7777, t1 = 31, t2 = `t2`.
fn vset(w: u32, t2: u64) -> (Machine, Option<Halt>) {
    let mut m = machine(&[w]);
    m.vec.vtype = 0x18;
    m.vec.vl = 2;
    m.x[5] = 0x7777;
    m.x[6] = 31;
    m.x[7] = t2;
    let h = m.step(&mut Vec::new());
    (m, h)
}

/// `tests/data/v009e_vtype_qemu.tsv` expanded: its ranges back into one (form, key,
/// result) row per key, keys spelled as the probe spelled them (`0x01f`, `0x41`, `7`), each
/// form's expansion checked against the file's HASH line.
fn vtype_rows() -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    let mut hashes = 0;
    let text = data("v009e_vtype_qemu.tsv");
    let mut start = 0;
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = l.split('\t').collect();
        if c[0] == "HASH" {
            let mut h = 0xcbf2_9ce4_8422_2325u64;
            for (_, k, r) in &rows[start..] {
                for b in format!("{k} {r}\n").bytes() {
                    h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            assert_eq!(
                h,
                hex(c[2]),
                "{}: the ranges expand to other rows than QEMU's",
                c[1]
            );
            hashes += 1;
            start = rows.len();
            continue;
        }
        let (form, first, last, res) = (c[0], c[1], c[2], c[3]);
        if first == "None" {
            rows.push((form.into(), first.into(), res.into()));
            continue;
        }
        let hexkey = first.starts_with("0x");
        let width = first.len();
        let num = |s: &str| {
            if hexkey {
                u32::from_str_radix(&s[2..], 16).unwrap()
            } else {
                s.parse().unwrap()
            }
        };
        for v in num(first)..=num(last) {
            let key = if hexkey {
                format!("{v:#0width$x}")
            } else {
                v.to_string()
            };
            rows.push((form.into(), key, res.into()));
        }
    }
    assert_eq!((hashes, start, rows.len()), (4, rows.len(), 3201));
    rows
}

#[test]
fn every_vtype_sets_vill_or_vl_exactly_as_qemu_does() {
    // `probe_ctl.S` section V: every 11-bit `vsetvli` vtypei (AVL = VLMAX), every 10-bit
    // `vsetivli` vtypei (AVL 31), `vsetvl` with each bit of rs2 set over e64 m1, and the
    // whole `bits 31:30 = 1x` space — 3,201 rows in `tests/data/v009e_vtype_qemu.tsv`.
    // QEMU never traps a vtype: an illegal one gives vill, vl = 0, rd = 0; a legal one,
    // e32 m1 and e64 mf2 among them, gives vl = rd = min(AVL, VLMAX).
    let mut wrong = Vec::new();
    let mut n = 0;
    for (form, key, res) in vtype_rows() {
        let (form, key, res) = (form.as_str(), key.as_str(), res.as_str());
        let k = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
        let (w, t2, vtype) = match form {
            "vsetvli-max" => (
                (k(key) << 20) | (7 << 12) | (5 << 7) | 0x57,
                0,
                u64::from(k(key)),
            ),
            "vsetivli-31" => (
                (0b11 << 30) | (k(key) << 20) | (31 << 15) | (7 << 12) | (5 << 7) | 0x57,
                0,
                u64::from(k(key)),
            ),
            "vsetvl-e64m1|bit" => {
                let t2 = 0x18 | key.parse::<u32>().map_or(0, |b| 1u64 << b);
                (
                    (0x40 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (5 << 7) | 0x57,
                    t2,
                    t2,
                )
            }
            _ => {
                let f7 = k(key);
                let w = (f7 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (5 << 7) | 0x57;
                (
                    w,
                    0x18,
                    if f7 >= 0x60 {
                        u64::from(((f7 & 0x1f) << 5) | 7)
                    } else {
                        0x18
                    },
                )
            }
        };
        let (m, h) = vset(w, t2);
        n += 1;
        let got = match h {
            Some(Halt::Undelivered { cause: 2, .. }) => "trap".to_string(),
            Some(h) => format!("{h:?}"),
            None if m.vec.vtype == yantra::vector::VILL && m.vec.vl == 0 && m.x[5] == 0 => {
                "vill".into()
            }
            None if m.vec.vtype == vtype && m.x[5] == m.vec.vl => m.vec.vl.to_string(),
            None => format!("vtype {:#x} vl {} rd {:#x}", m.vec.vtype, m.vec.vl, m.x[5]),
        };
        if got != res && wrong.len() < 30 {
            wrong.push(format!(
                "{form} {key} ({w:#010x}): QEMU {res}, yantra {got}"
            ));
        }
    }
    assert_eq!(n, 3201);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn vsetvli_x0_x0_keeps_vl_clamped_as_qemu_does() {
    // `probe_keep.S` K1..K12: keeping vl CLAMPS it to the new VLMAX and never sets vill
    // for a VLMAX change; from vill (vl 0) it keeps 0. (prelude, instruction, t0, vl, vtype)
    let cases: [(&[&str], &str, u64, u64, u64); 12] = [
        (
            &["vsetivli x0, 2, e64, m1, tu, mu"],
            "vsetvli x0, x0, e64, m2, tu, mu",
            0x7777,
            2,
            0x19,
        ),
        (
            &["vsetivli x0, 4, e64, m2, tu, mu"],
            "vsetvli x0, x0, e64, m1, tu, mu",
            0x7777,
            2,
            0x18,
        ),
        (
            &["vsetivli x0, 1, e64, m2, tu, mu"],
            "vsetvli x0, x0, e64, m1, tu, mu",
            0x7777,
            1,
            0x18,
        ),
        (
            &["vsetivli x0, 2, e64, m1, tu, mu"],
            "vsetvli x0, x0, e32, mf2, tu, mu",
            0x7777,
            2,
            0x17,
        ),
        (
            &["vsetivli x0, 2, e64, m1, tu, mu"],
            "vsetvli x0, x0, e64, m1, ta, ma",
            0x7777,
            2,
            0xd8,
        ),
        (
            &["vsetivli x0, 2, e64, m1, tu, mu", "vsetvli x0, x0, 0x1c"],
            "vsetvli x0, x0, e64, m1, tu, mu",
            0x7777,
            0,
            0x18,
        ),
        (
            &["vsetivli x0, 2, e64, m1, tu, mu"],
            "vsetvli x0, x0, 0x1c",
            0x7777,
            0,
            1 << 63,
        ),
        (
            &["vsetivli x0, 2, e64, m1, tu, mu", "li t2, 0x1a"],
            "vsetvl x0, x0, t2",
            0x7777,
            2,
            0x1a,
        ),
        (
            &["vsetivli x0, 1, e64, m1, tu, mu", "li t2, 0x1a"],
            "vsetvl t0, x0, t2",
            8,
            8,
            0x1a,
        ),
        (
            &[
                "vsetivli x0, 1, e64, m1, tu, mu",
                "li t1, 5",
                "li t2, 0x18",
                "li t3, 1",
                "slli t3, t3, 63",
                "or t2, t2, t3",
            ],
            "vsetvl t0, t1, t2",
            0,
            0,
            1 << 63,
        ),
        (
            &["vsetivli x0, 2, e64, m1, tu, mu", "li t1, 0"],
            "vsetvli t0, t1, e64, m1, tu, mu",
            0,
            0,
            0x18,
        ),
        (
            &["li t1, -1"],
            "vsetvli t0, t1, e64, m8, tu, mu",
            16,
            16,
            0x1b,
        ),
    ];
    for (pre, ins, t0, vl, vtype) in cases {
        // t0 is set before the prelude rather than by `li t0, 0x7777` in the program, which
        // assembles to two words; no prelude line writes t0.
        let mut lines = pre.to_vec();
        lines.push(ins);
        let m = run_ok(&lines, |m| m.x[5] = 0x7777);
        assert_eq!(
            (m.x[5], m.vec.vl, m.vec.vtype),
            (t0, vl, vtype),
            "{pre:?} then {ins}"
        );
    }
}

#[test]
fn under_vill_every_vtype_dependent_vector_instruction_traps_cause_2() {
    // `probe_ctl.S` section L, after `vsetvli t0, x0, 0x1c` (reserved LMUL -> vill): QEMU
    // traps cause 2 on all of these, masked and outside-the-subset forms included. Until
    // (i-e) a form this machine refuses halted `Unimplemented` here instead.
    let illegal = [
        "vadd.vv v1, v2, v3",
        "vadd.vx v1, v2, t1",
        "vadd.vi v1, v2, 3",
        "vadd.vv v1, v2, v3, v0.t",
        "vmul.vv v1, v2, v3",
        "vfadd.vv v1, v2, v3",
        "vfadd.vf v1, v2, f1",
        "vfdiv.vv v1, v2, v3",
        "vfredosum.vs v1, v2, v3",
        "vle64.v v1, (a1)",
        "vse64.v v1, (a1)",
        "vlse64.v v1, (a1), t2",
        "vle32.v v1, (a1)",
        "vmv1r.v v1, v2",
        "vmv.x.s a2, v2",
        "vfmv.f.s f2, v2",
        "vmv.s.x v1, t1",
        "vid.v v1",
        "vsadd.vv v1, v2, v3",
        ".word 0x062180d7", // OPIVV funct6 000001: reserved
    ];
    for line in illegal {
        let (m, h) = run(&["vsetvli t0, x0, 0x1c", line], |m| m.x[11] = BASE + 0x8000);
        assert_eq!(
            m.vec.vtype,
            yantra::vector::VILL,
            "{line}: the setup set vill"
        );
        assert_eq!(
            h,
            undelivered(BASE + 4),
            "{line}: QEMU traps cause 2 under vill"
        );
    }
    // The whole-register forms do not read vtype and RUN under vill on QEMU. They are
    // outside this machine's subset, so they stay refused by name — never a trap.
    for line in ["vl1re64.v v1, (a1)", "vs1r.v v1, (a1)"] {
        let (_, h) = run(&["vsetvli t0, x0, 0x1c", line], |m| m.x[11] = BASE + 0x8000);
        assert!(
            matches!(h, Some(Halt::Unimplemented { pc, .. }) if pc == BASE + 4),
            "{line}: {h:?}"
        );
    }
}

#[test]
fn a_legal_vtype_outside_the_engine_is_held_and_its_arithmetic_refused_by_name() {
    // e32 m1 and e64 mf2 are LEGAL (QEMU runs vadd.vv, vfadd.vv, vfredosum under both,
    // `probe_ctl.S` section L): vsetvli holds them and sets vl. The engine computes e64 at
    // integer LMUL only, so an instruction under them halts by name (REFUSE_VTYPE) — the
    // refusal moved from vsetvli to the first instruction that needs the width.
    for (setup, vl) in [
        ("vsetvli t0, x0, e32, m1, tu, mu", 4),
        ("vsetvli t0, x0, e64, mf2, tu, mu", 1),
    ] {
        for line in [
            "vadd.vv v1, v2, v3",
            "vfadd.vv v1, v2, v3",
            "vfredosum.vs v1, v2, v3",
        ] {
            let (m, h) = run(&[setup, line], |_| {});
            assert_eq!((m.x[5], m.vec.vl), (vl, vl), "{setup}");
            assert!(
                matches!(h, Some(Halt::Unimplemented { pc, .. }) if pc == BASE + 4),
                "{setup}; {line}: {h:?}"
            );
        }
    }
}

#[test]
fn the_reserved_vsetvl_space_is_illegal() {
    // `probe_ctl.S` V, `vsetvl-f7` 0x41..0x5f: bits 31:30 = 10 with bits 30:25 non-zero
    // trap cause 2 on QEMU (until (i-e): refused by name, REFUSE_SETVL_FORM).
    for f7 in 0x41u32..0x60 {
        let w = (f7 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (5 << 7) | 0x57;
        let (m, h) = vset(w, 0x18);
        assert_eq!(h, undelivered(BASE), "{f7:#x}");
        assert_eq!(
            (m.vec.vtype, m.vec.vl, m.x[5]),
            (0x18, 2, 0x7777),
            "{f7:#x}: nothing written"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════════
// 3. sstatus.UXL
// ═════════════════════════════════════════════════════════════════════════════════

const UXL_MASK: u64 = 3 << 32;

#[test]
fn sstatus_uxl_reads_2() {
    // QEMU: sstatus from S-mode reads 0x8000000200006600 with FS and VS dirty (`probe_ctl.S`
    // U, "sstatus as entered"); the (i-d) builder's probe4 S0 read 0x0000000200002200
    // from reset. UXL (bits 33:32) = 2: U-mode is 64-bit. This machine read 0 there.
    let m = run_ok(&["csrr a0, sstatus"], |_| {});
    assert_eq!(m.x[10] & UXL_MASK, 2 << 32);
    // SD and UXL together, with FS dirty — QEMU's own word for that state.
    let m = run_ok(&["csrr a0, sstatus"], |m| {
        m.csr.sstatus |= 0b11 << 13 | 0b11 << 9
    });
    assert_eq!(m.x[10], 0x8000_0002_0000_6600);
}

#[test]
fn sstatus_uxl_ignores_writes() {
    // WARL with ONE legal value: this machine runs U-mode at 64 bits only. QEMU's readings
    // (`probe_ctl.S` U) are the rows; where QEMU stores the write (1, 3) the row is in
    // `KNOWN_QEMU_DIFFERENCES` and [`expected`] answers this machine's 2 — after checking
    // that QEMU's recorded value still differs.
    for (uxl, qemu) in [(0u64, 2u64), (1, 1), (3, 3)] {
        let want = expected(0x100, UXL_MASK, uxl << 32, 0x100, UXL_MASK, qemu << 32);
        let got = write_then_read(0x100, UXL_MASK, uxl << 32, 0x100) & UXL_MASK;
        assert_eq!(got, want, "after writing UXL = {uxl}");
    }
    let m = run_ok(&["csrc sstatus, t2", "csrr a0, sstatus"], |m| {
        m.x[7] = UXL_MASK
    });
    assert_eq!(
        m.x[10] & UXL_MASK,
        2 << 32,
        "csrc of the field (QEMU: unchanged)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════════
// 3b. KNOWN_QEMU_DIFFERENCES — the deliberate spec-over-QEMU rows
// ═════════════════════════════════════════════════════════════════════════════════

/// Write `value` into the `mask` bits of CSR `w` (read-modify-write: `old & !mask |
/// value`), then read CSR `r`, on a fresh machine in S-mode with FS and VS Initial.
fn write_then_read(w: u16, mask: u64, value: u64, r: u16) -> u64 {
    let (rd_old, wr, rd) = (
        format!("csrr t1, {w:#x}"),
        format!("csrw {w:#x}, t1"),
        format!("csrr a0, {r:#x}"),
    );
    let m = run_ok(
        &[&rd_old, "and t1, t1, t3", "or t1, t1, t2", &wr, &rd],
        |m| {
            m.x[7] = value;
            m.x[28] = !mask;
        },
    );
    m.x[10]
}

/// The listed difference covering this write/read, if any.
fn known(w: u16, wmask: u64, value: u64, r: u16, rmask: u64) -> Option<yantra::QemuDifference> {
    yantra::KNOWN_QEMU_DIFFERENCES.iter().copied().find(|d| {
        (d.write_csr, d.write_mask, d.value, d.read_csr, d.read_mask) == (w, wmask, value, r, rmask)
    })
}

/// What this machine must read for a row where QEMU read `qemu`: QEMU's value, unless
/// `KNOWN_QEMU_DIFFERENCES` lists the row — then this machine's listed value, and only
/// after asserting that the list still records QEMU's reading and that it still differs.
fn expected(w: u16, wmask: u64, value: u64, r: u16, rmask: u64, qemu: u64) -> u64 {
    match known(w, wmask, value, r, rmask) {
        Some(d) => {
            assert_eq!(
                d.qemu, qemu,
                "{}: the list no longer records QEMU's reading",
                d.covers
            );
            assert_ne!(d.yantra, qemu, "{}: listed, but QEMU agrees now", d.covers);
            d.yantra
        }
        None => qemu,
    }
}

/// `tests/data/v009e_csr_writes_qemu.tsv`: (write CSR, write mask, value, read CSR, read
/// mask, QEMU's reading, probe row).
fn csr_write_rows() -> Vec<(u16, u64, u64, u16, u64, u64, String)> {
    let h = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
    data("v009e_csr_writes_qemu.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (
                h(c[0]) as u16,
                h(c[1]),
                h(c[2]),
                h(c[3]) as u16,
                h(c[4]),
                h(c[5]),
                c[6].to_string(),
            )
        })
        .collect()
}

#[test]
fn every_recorded_csr_write_reads_back_qemus_value_or_a_listed_difference() {
    // THE GUARD BOTH WAYS: every probe row must agree with QEMU unless it is listed — so a
    // deviation that is not in `KNOWN_QEMU_DIFFERENCES` (an entry removed while the code
    // still deviates, or a new one) FAILS here; and a listed row must still deviate, read
    // this machine's listed value, and carry QEMU's recorded reading.
    let rows = csr_write_rows();
    assert_eq!(rows.len(), 24);
    let mut wrong = Vec::new();
    for (w, wm, v, r, rm, qemu, label) in &rows {
        let got = write_then_read(*w, *wm, *v, *r) & rm;
        match known(*w, *wm, *v, *r, *rm) {
            None if got != *qemu => wrong.push(format!(
                "{label}: yantra {got:#x}, QEMU {qemu:#x} — a deviation KNOWN_QEMU_DIFFERENCES \
                 does not list"
            )),
            Some(d) if (got, d.qemu) != (d.yantra, *qemu) || d.yantra == d.qemu => wrong.push(
                format!("{label}: yantra {got:#x}, listed {d:?}, QEMU recorded {qemu:#x}"),
            ),
            _ => {}
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_known_qemu_difference_is_recorded_and_still_deviates() {
    // No stale entry: each one matches a QEMU-recorded probe row, with QEMU's value as
    // recorded, and this machine still reads its listed (different) value there.
    let rows = csr_write_rows();
    for d in yantra::KNOWN_QEMU_DIFFERENCES {
        let row = rows.iter().find(|(w, wm, v, r, rm, _, _)| {
            (*w, *wm, *v, *r, *rm) == (d.write_csr, d.write_mask, d.value, d.read_csr, d.read_mask)
        });
        let (.., qemu, label) =
            row.unwrap_or_else(|| panic!("{}: no probe row records it", d.covers));
        assert_eq!(d.qemu, *qemu, "{} ({label})", d.covers);
        assert_ne!(d.yantra, d.qemu, "{}", d.covers);
        assert!(!d.reason.is_empty(), "{}", d.covers);
        let got = write_then_read(d.write_csr, d.write_mask, d.value, d.read_csr) & d.read_mask;
        assert_eq!(
            got, d.yantra,
            "{}: the code no longer deviates — remove the entry",
            d.covers
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════════
// 4. vxsat / vxrm / vcsr
// ═════════════════════════════════════════════════════════════════════════════════
//
// `probe_ctl.S` section C (and the (i-d) builder's probe.S X1..X3, probe5 U/M-vsoff). No
// vector fixed-point instruction is in this machine's subset, so the three are STORAGE.

const VS: u64 = 0b11 << 9;

#[test]
fn the_fixed_point_csrs_read_0_with_vs_on_and_trap_with_vs_off_in_s_and_u_mode() {
    // QEMU, M, S and U alike: VS on -> reads 0; VS off -> cause 2, a0 unwritten.
    for line in ["csrr a0, vxsat", "csrr a0, vxrm", "csrr a0, vcsr"] {
        for mode in [Privilege::Supervisor, Privilege::User] {
            for vs_on in [true, false] {
                let (m, h) = run(&[line], |m| {
                    m.mode = mode;
                    m.x[10] = 0x5a5a;
                    if !vs_on {
                        m.csr.sstatus &= !VS;
                    }
                });
                let want = if vs_on {
                    (None, 0)
                } else {
                    (undelivered(BASE), 0x5a5a)
                };
                assert_eq!((h, m.x[10]), want, "{line} {mode:?} VS on {vs_on}");
            }
        }
    }
}

#[test]
fn the_fixed_point_csrs_keep_their_legal_bits() {
    // `probe_ctl.S` C, "write X=V": (CSR, value) -> QEMU's (vxsat, vxrm, vcsr) read back,
    // VS 3. QEMU's row `vxrm = 0xff` is a KNOWN difference (QEMU stores vxrm whole; it is
    // a two-bit field): [`expected`] answers this machine's value there, after checking
    // QEMU's recorded one still differs.
    let rows: [(&str, u16, u64, [u64; 3]); 7] = [
        ("vxrm", 0x00a, 0xff, [0, 0xff, 0x1fe]),
        ("vxsat", 0x009, 0xff, [1, 0, 1]),
        ("vcsr", 0x00f, 0xff, [1, 3, 7]),
        ("vcsr", 0x00f, 0x5, [1, 2, 5]),
        ("vcsr", 0x00f, 0x2, [0, 1, 2]),
        ("vxrm", 0x00a, 0x2, [0, 2, 4]),
        ("vxsat", 0x009, 0x3, [1, 0, 1]),
    ];
    for (csr, num, v, qemu) in rows {
        let w = format!("csrw {csr}, t0");
        let m = run_ok(
            &[
                &w,
                "csrr a1, vxsat",
                "csrr a2, vxrm",
                "csrr a3, vcsr",
                "csrr a4, fcsr",
            ],
            |m| m.x[5] = v,
        );
        for (i, r) in [0x009u16, 0x00a, 0x00f].into_iter().enumerate() {
            let want = expected(num, u64::MAX, v, r, u64::MAX, qemu[i]);
            assert_eq!(m.x[11 + i], want, "{csr} = {v:#x}, read {r:#x}");
        }
        assert_eq!(m.csr.sstatus & VS, VS, "{csr} = {v:#x}: VS Dirty");
        assert_eq!(
            m.x[14], 0,
            "fcsr does not mirror them (QEMU: fcsr unchanged)"
        );
    }
    // fcsr bits 10:8 do not reach vcsr (QEMU: fcsr = 0x7ff reads 0xff, vcsr 0).
    let m = run_ok(&["csrw fcsr, t0", "csrr a1, vcsr"], |m| m.x[5] = 0x7ff);
    assert_eq!(m.x[11], 0);
}

#[test]
fn a_fixed_point_csr_access_dirties_vs_exactly_when_qemu_does() {
    // `probe_ctl.S` C, "VS after": VS Initial, vxrm and vxsat holding `start`, t3 = start,
    // t4 = 2. A write that HAPPENS dirties VS (csrrw always; csrrs/csrrc when the mask is
    // non-zero, even if nothing changes); a read does not. (form, VS after for start 0, 1)
    let rows: [(&str, u64, u64); 10] = [
        ("csrr a0, vxrm", 1, 1),
        ("csrrs a0, vxrm, x0", 1, 1),
        ("csrrsi a0, vxrm, 0", 1, 1),
        ("csrrc a0, vxrm, x0", 1, 1),
        ("csrrw a0, vxrm, t3", 3, 3),
        ("csrrs a0, vxrm, t3", 1, 3),
        ("csrrc a0, vxrm, t4", 3, 3),
        ("csrrsi a0, vxsat, 1", 3, 3),
        ("csrrci a0, vxsat, 1", 3, 3),
        ("csrrwi a0, vcsr, 0", 3, 3),
    ];
    for (form, vs0, vs1) in rows {
        for (start, want) in [(0u64, vs0), (1, vs1)] {
            let m = run_ok(&[form], |m| {
                m.vec.vxrm = start;
                m.vec.vxsat = start;
                m.x[28] = start;
                m.x[29] = 2;
            });
            assert_eq!((m.csr.sstatus & VS) >> 9, want, "{form}, start {start}");
        }
    }
}

#[test]
fn u_mode_writes_vcsr_as_qemu_does() {
    // `probe_ctl.S` C: U-mode `csrw vcsr, 6` -> vcsr 6 (vxrm 3, vxsat 0), VS Dirty.
    let m = run_ok(&["csrw vcsr, t0"], |m| {
        m.mode = Privilege::User;
        m.x[5] = 6;
    });
    assert_eq!((m.vec.vxrm, m.vec.vxsat), (3, 0));
    assert_eq!(m.csr.sstatus & VS, VS);
}
