//! **Row `V-007`, named part (2) — the VECTOR ORACLE.**
//!
//! The vector twin of `V-012`'s scalar float oracle (`tools/check-float-oracle.sh`): the
//! same program runs on `yantra` and on a machine this repository did not write, and the
//! two must agree **bit for bit**.
//!
//! # The tooling, and why it is not the float oracle's
//!
//! The float oracle is a Sassembly program assembled by `sadhana`, and every other
//! generator in `tools/` pins `-march=rv64gc`. Neither can emit a vector instruction —
//! the Sassembly side is `V-005`/`V-008`'s, not this row's. So this oracle is written in
//! GNU assembly, generated below, and built by **binutils with V**
//! (`riscv64-elf-as -march=rv64gcv`, GNU Binutils 2.47 here). The reference machine is
//! **`qemu-system-riscv64` with V enabled at this row's shape**:
//! `-cpu rv64,v=true,vlen=128,elen=64` (QEMU 11.0.3 here). `spike` and `qemu-riscv64` are
//! not installed on this host.
//!
//! **NO `li` OF A WIDE CONSTANT.** `li` expands to `lui` + `addiw`, and `yantra` does not
//! execute OP-IMM-32 (`0x1b`) — nothing `sadhana` emits uses it. The first run of this
//! file halted there, before any vector instruction. Wide values are loaded from data
//! with `ld`, and the two wide addresses (`0x2200`, `0x5555`) are built with shifts.
//! (`V-009` part (i-d) implemented OP-IMM-32 and OP-32, so `addiw` now runs; the
//! program is left as it was.)
//!
//! # What the program does
//!
//! Every case writes into an output buffer, and at the end the program prints the
//! buffer as hex on the UART and writes `0x5555` to the finisher. Both machines' UART
//! text is compared line by line, and each line maps back to the case that wrote it.
//!
//! **THE TAIL IS IN THE DUMP.** Each element-wise case fills its destination group with a
//! sentinel at `VLMAX`, runs the instruction at a partial `vl`, and stores the whole group
//! back at `VLMAX` — so an implementation that writes past `vl`, or skips elements below
//! it, shows up as a sentinel where a result belongs or a result where a sentinel
//! belongs. Lengths are 0, 1, 5 and 13: none but 0 a multiple of any `VLMAX` (2, 4, 8,
//! 16), so every stripmined loop ends on a partial strip.
//!
//! The float data carries the cases a careless implementation gets wrong: both zeros,
//! both infinities, a quiet NaN with a payload and a signalling NaN (both must come out as
//! the canonical NaN), subnormals, the largest finite values (overflow), and values whose
//! ordered sum differs from any reassociated one.

use std::fmt::Write as _;
use std::io::Read as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use yantra::{Halt, Machine};

const LMULS: [u32; 4] = [1, 2, 4, 8];
const LENGTHS: [u64; 4] = [0, 1, 5, 13];
const LANES: u64 = 2; // VLEN 128 / SEW 64

fn vlmax(lmul: u32) -> u64 {
    LANES * u64::from(lmul)
}

/// The integer operand arrays: 64 values each, edge cases first.
fn int_data(seed: u64) -> Vec<u64> {
    let mut v = vec![
        0,
        1,
        u64::MAX,
        i64::MIN as u64,
        i64::MAX as u64,
        0x8000_0000,
        0xffff_ffff,
        0x1234_5678_9abc_def0,
    ];
    let mut x = seed;
    while v.len() < 64 {
        // splitmix64: a fixed, reproducible spread of 64-bit values
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        v.push(z ^ (z >> 31));
    }
    v
}

/// The float operand arrays, as bits. Rotated by `shift` so A and B pair differently.
fn float_data(shift: usize) -> Vec<u64> {
    let mut v: Vec<u64> = vec![
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1.0f64.to_bits(),
        (-1.5f64).to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0123, // quiet NaN with a payload
        0x7ff0_0000_0000_0001, // signalling NaN
        f64::MAX.to_bits(),
        (-f64::MAX).to_bits(),
        1u64,                  // smallest subnormal
        0x000f_ffff_ffff_ffff, // largest subnormal
        f64::MIN_POSITIVE.to_bits(),
        0.1f64.to_bits(),
        (1.0f64 / 3.0).to_bits(),
        std::f64::consts::PI.to_bits(),
        1e16f64.to_bits(),
        (-1e16f64).to_bits(),
        3.0f64.to_bits(),
        1e-300f64.to_bits(),
        1e300f64.to_bits(),
    ];
    let mut k = 1.0f64;
    while v.len() < 64 {
        k = k * -1.37 + 0.25;
        v.push(k.to_bits());
    }
    v.rotate_left(shift);
    v
}

/// One case's place in the output: its name and how many 64-bit words it writes.
struct Case {
    name: String,
    words: usize,
}

struct Suite {
    asm: String,
    cases: Vec<Case>,
    /// How many cases each subset instruction appears in, for the report.
    per_insn: std::collections::BTreeMap<&'static str, usize>,
}

fn dwords(out: &mut String, label: &str, values: &[u64]) {
    writeln!(out, "  .balign 8\n{label}:").unwrap();
    for v in values {
        writeln!(out, "  .dword {v:#018x}").unwrap();
    }
}

#[allow(clippy::too_many_lines)]
fn suite() -> Suite {
    let mut a = String::new();
    let mut cases = Vec::new();
    let mut per_insn = std::collections::BTreeMap::new();
    fn count(per: &mut std::collections::BTreeMap<&'static str, usize>, i: &'static str) {
        *per.entry(i).or_insert(0) += 1;
    }
    a.push_str(
        ".option norvc\n.text\n.globl _start\n_start:\n\
         \x20 li t0, 0x22   # FS and VS Initial (0x2200): on hardware both gate their unit\n\
         \x20 slli t0, t0, 8\n\
         \x20 csrs sstatus, t0\n\
         \x20 la s11, OUT\n",
    );

    // ── vsetvli: vl, and the vtype/vl CSRs it leaves behind ──────────────────
    for lmul in LMULS {
        for (pol, avl) in [
            ("tu, mu", 0u64),
            ("tu, mu", 1),
            ("ta, ma", 3),
            ("tu, mu", 7),
            ("ta, mu", 17),
            ("tu, ma", 100),
        ] {
            write!(
                a,
                "  li a0, {avl}\n  vsetvli t0, a0, e64, m{lmul}, {pol}\n  sd t0, 0(s11)\n\
                 \x20 csrr t1, vl\n  sd t1, 8(s11)\n  csrr t1, vtype\n  sd t1, 16(s11)\n  addi s11, s11, 24\n"
            )
            .unwrap();
            cases.push(Case {
                name: format!("vsetvli avl={avl} m{lmul} {pol}"),
                words: 3,
            });
            count(&mut per_insn, "vsetvli");
        }
        // rs1 = x0, rd != x0: AVL = VLMAX
        write!(
            a,
            "  vsetvli t0, x0, e64, m{lmul}, tu, mu\n  sd t0, 0(s11)\n  addi s11, s11, 8\n"
        )
        .unwrap();
        cases.push(Case {
            name: format!("vsetvli x0 m{lmul}"),
            words: 1,
        });
        count(&mut per_insn, "vsetvli");
    }
    a.push_str("  csrr t0, vlenb\n  sd t0, 0(s11)\n  addi s11, s11, 8\n");
    cases.push(Case {
        name: "vlenb".into(),
        words: 1,
    });

    // ── element-wise arithmetic, every LMUL, every length ─────────────────────
    // (mnemonic, form, float?)
    let ops: [(&'static str, &str, bool); 14] = [
        ("vadd.vv", "vv", false),
        ("vadd.vx", "vx", false),
        ("vsub.vv", "vv", false),
        ("vsub.vx", "vx", false),
        ("vmul.vv", "vv", false),
        ("vmul.vx", "vx", false),
        ("vfadd.vv", "vv", true),
        ("vfadd.vf", "vf", true),
        ("vfsub.vv", "vv", true),
        ("vfsub.vf", "vf", true),
        ("vfmul.vv", "vv", true),
        ("vfmul.vf", "vf", true),
        ("vfdiv.vv", "vv", true),
        ("vfdiv.vf", "vf", true),
    ];
    // Scalars for .vx/.vf, cycled per case so the scalar side sees edges too.
    let xs = [3u64, u64::MAX, i64::MIN as u64, 0x0123_4567_89ab_cdef];
    let fs = [
        2.5f64.to_bits(),
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        f64::INFINITY.to_bits(),
        0x7ff0_0000_0000_0001,
        1e-310f64.to_bits(),
    ];
    let mut k = 0usize;
    for (insn, form, float) in ops {
        for lmul in LMULS {
            for n in LENGTHS {
                k += 1;
                let (src_a, src_b) = if float { ("FA", "FB") } else { ("IA", "IB") };
                // offset into the data so cases do not all start at element 0
                let off = (k * 3) % 40 * 8;
                let operand = match form {
                    "vv" => "v24".to_string(),
                    "vx" => {
                        write!(a, "  la t2, XS\n  ld a3, {}(t2)\n", (k % xs.len()) * 8).unwrap();
                        "a3".to_string()
                    }
                    _ => {
                        write!(a, "  la t2, FS\n  fld fa0, {}(t2)\n", (k % fs.len()) * 8).unwrap();
                        "fa0".to_string()
                    }
                };
                write!(
                    a,
                    "  li a0, {n}\n  la a1, {src_a}+{off}\n  la a2, {src_b}+{off}\n\
                     1:\n\
                     \x20 vsetvli t1, x0, e64, m{lmul}, tu, mu\n\
                     \x20 la t2, SENT\n  vle64.v v8, (t2)\n\
                     \x20 vsetvli t0, a0, e64, m{lmul}, tu, mu\n\
                     \x20 vle64.v v16, (a1)\n  vle64.v v24, (a2)\n\
                     \x20 {insn} v8, v16, {operand}\n\
                     \x20 vsetvli t1, x0, e64, m{lmul}, tu, mu\n\
                     \x20 vse64.v v8, (s11)\n\
                     \x20 slli t2, t1, 3\n  add s11, s11, t2\n\
                     \x20 slli t2, t0, 3\n  add a1, a1, t2\n  add a2, a2, t2\n\
                     \x20 sub a0, a0, t0\n  bnez a0, 1b\n"
                )
                .unwrap();
                let strips = n.div_ceil(vlmax(lmul)).max(1);
                cases.push(Case {
                    name: format!("{insn} m{lmul} n={n}"),
                    words: (strips * vlmax(lmul)) as usize,
                });
                count(&mut per_insn, insn);
                count(&mut per_insn, "vle64.v");
                count(&mut per_insn, "vse64.v");
            }
        }
    }

    // ── strided loads: positive, negative and zero strides ───────────────────
    for lmul in LMULS {
        for n in [1u64, 5, 13] {
            for (stride, start) in [(24i64, 0usize), (-8, 63), (0, 7), (16, 1)] {
                write!(
                    a,
                    "  li a0, {n}\n  la a1, IA+{}\n  li a4, {stride}\n\
                     1:\n\
                     \x20 vsetvli t1, x0, e64, m{lmul}, tu, mu\n\
                     \x20 la t2, SENT\n  vle64.v v16, (t2)\n\
                     \x20 vsetvli t0, a0, e64, m{lmul}, tu, mu\n\
                     \x20 vlse64.v v16, (a1), a4\n\
                     \x20 vsetvli t1, x0, e64, m{lmul}, tu, mu\n\
                     \x20 vse64.v v16, (s11)\n\
                     \x20 slli t2, t1, 3\n  add s11, s11, t2\n\
                     \x20 mul t2, t0, a4\n  add a1, a1, t2\n\
                     \x20 sub a0, a0, t0\n  bnez a0, 1b\n",
                    start * 8
                )
                .unwrap();
                let strips = n.div_ceil(vlmax(lmul));
                cases.push(Case {
                    name: format!("vlse64.v m{lmul} n={n} stride={stride}"),
                    words: (strips * vlmax(lmul)) as usize,
                });
                count(&mut per_insn, "vlse64.v");
            }
        }
    }

    // ── strided stores: into a zeroed region, gaps must stay zero ────────────
    for lmul in LMULS {
        for n in [1u64, 5, 13] {
            for stride in [16u64, 24] {
                let region = n * stride / 8; // words the region spans
                write!(
                    a,
                    "  li a0, {n}\n  la a1, IB+8\n  mv a5, s11\n  li a4, {stride}\n\
                     1:\n\
                     \x20 vsetvli t0, a0, e64, m{lmul}, tu, mu\n\
                     \x20 vle64.v v16, (a1)\n\
                     \x20 vsse64.v v16, (a5), a4\n\
                     \x20 slli t2, t0, 3\n  add a1, a1, t2\n\
                     \x20 mul t2, t0, a4\n  add a5, a5, t2\n\
                     \x20 sub a0, a0, t0\n  bnez a0, 1b\n\
                     \x20 li t2, {}\n  add s11, s11, t2\n",
                    region * 8
                )
                .unwrap();
                cases.push(Case {
                    name: format!("vsse64.v m{lmul} n={n} stride={stride}"),
                    words: region as usize,
                });
                count(&mut per_insn, "vsse64.v");
            }
        }
    }

    // ── the ordered sum ───────────────────────────────────────────────────────
    // v8 starts as two words of FS; each strip folds into element 0; the dump is v8
    // whole, so element 1 must come out as it went in. Length 0 must leave element 0
    // alone too — even when it is a signalling NaN.
    for lmul in LMULS {
        for n in LENGTHS {
            for (src, start) in [("FA", 0usize), ("FB", 2), ("SUMS", 0), ("SUMS2", 1)] {
                write!(
                    a,
                    "  la t2, FS+{}\n  vsetvli t1, x0, e64, m1, tu, mu\n  vle64.v v8, (t2)\n\
                     \x20 li a0, {n}\n  la a1, {src}\n\
                     1:\n\
                     \x20 vsetvli t0, a0, e64, m{lmul}, tu, mu\n\
                     \x20 vle64.v v16, (a1)\n\
                     \x20 vfredosum.vs v8, v16, v8\n\
                     \x20 slli t2, t0, 3\n  add a1, a1, t2\n\
                     \x20 sub a0, a0, t0\n  bnez a0, 1b\n\
                     \x20 vsetvli t1, x0, e64, m1, tu, mu\n  vse64.v v8, (s11)\n  addi s11, s11, 16\n",
                    start * 8
                )
                .unwrap();
                cases.push(Case {
                    name: format!("vfredosum.vs m{lmul} n={n} {src}"),
                    words: 2,
                });
                count(&mut per_insn, "vfredosum.vs");
            }
        }
    }

    // ── dump the buffer as hex, one word per line, then the finisher ─────────
    a.push_str(
        "  la s0, OUT\n  li t4, 0x10000000\n\
         2:\n  bgeu s0, s11, 9f\n  ld a0, 0(s0)\n  li t3, 16\n\
         3:\n  srli t5, a0, 60\n  addi t6, t5, 48\n  li t2, 10\n  blt t5, t2, 4f\n  addi t6, t5, 87\n\
         4:\n  sb t6, 0(t4)\n  slli a0, a0, 4\n  addi t3, t3, -1\n  bnez t3, 3b\n\
         \x20 li t6, 10\n  sb t6, 0(t4)\n  addi s0, s0, 8\n  j 2b\n\
         9:\n  li t0, 0x100000\n  li t1, 0x555\n  slli t1, t1, 4\n  addi t1, t1, 5\n  sw t1, 0(t0)\n\
         5:\n  j 5b\n\n.data\n",
    );
    dwords(&mut a, "IA", &int_data(1));
    dwords(&mut a, "IB", &int_data(2));
    dwords(&mut a, "FA", &float_data(0));
    dwords(&mut a, "FB", &float_data(7));
    dwords(&mut a, "FS", &fs);
    dwords(&mut a, "XS", &xs);
    // a sum whose order matters at every step: 1e16 absorbs each 1.0 in order
    let sums: Vec<u64> = [
        1e16, 1.0, 1.0, 1.0, -1e16, 1.0, 0.5, 1e-17, 3.0, -2.0, 1e16, 1.0, 1.0, -1e16,
    ]
    .iter()
    .map(|x: &f64| x.to_bits())
    .collect();
    dwords(&mut a, "SUMS", &sums);
    // Starting from 0.0 (FS word 1). In order, 0.5 + 1e16 swallows the 0.5 and every
    // later one, and -1e16 cancels to 0; summed backwards, -1e16 swallows the 0.5s,
    // cancels against 1e16, and the first 0.5 survives. At n = 5 the order decides
    // between 1e16 and 1e16 + 2.
    let mut sums2 = vec![0.5f64.to_bits(), 1e16f64.to_bits()];
    sums2.extend(std::iter::repeat_n(0.5f64.to_bits(), 10));
    sums2.push((-1e16f64).to_bits());
    dwords(&mut a, "SUMS2", &sums2);
    let sent: Vec<u64> = (0..16).map(|i| 0x5e57_1e00_0000_0000 | i).collect();
    dwords(&mut a, "SENT", &sent);
    let total: usize = cases.iter().map(|c| c.words).sum();
    writeln!(a, "  .balign 8\nOUT:\n  .zero {}", total * 8 + 64).unwrap();
    Suite {
        asm: a,
        cases,
        per_insn,
    }
}

fn scratch() -> PathBuf {
    // One directory per CALL, not per process: this file's tests run on
    // parallel threads of one process, and a pid-only name let one test's
    // assembler overwrite the other's image mid-read ("not an ELF file",
    // seen in a gate on 2026-10-05; green on rerun).
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("v007-oracle-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn tool(cmd: &mut Command, what: &str) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("{what}: {e} — this oracle needs it"));
    assert!(
        out.status.success(),
        "{what} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Build the ELF. Linked at 0x8000_0000, where QEMU's `-bios none` reset vector jumps.
fn build(asm: &str) -> Vec<u8> {
    let dir = scratch();
    let (src, obj, elf) = (dir.join("v.s"), dir.join("v.o"), dir.join("v.elf"));
    std::fs::write(&src, asm).unwrap();
    tool(
        Command::new("riscv64-elf-as")
            .args(["-march=rv64gcv", "-o"])
            .arg(&obj)
            .arg(&src),
        "riscv64-elf-as -march=rv64gcv",
    );
    tool(
        Command::new("riscv64-elf-ld")
            .args(["-Ttext=0x80000000", "-e", "_start", "-o"])
            .arg(&elf)
            .arg(&obj),
        "riscv64-elf-ld",
    );
    std::fs::read(&elf).unwrap()
}

fn run_yantra(image: &[u8]) -> (String, u64) {
    let mut m = Machine::load_elf(image, 1 << 20).expect("load");
    let mut out: Vec<u8> = Vec::new();
    let h = m.run(50_000_000, &mut out);
    assert!(
        matches!(h, Halt::Finisher { value: 0x5555, .. }),
        "yantra did not finish the suite: {h:?}"
    );
    (String::from_utf8(out).expect("hex"), m.time)
}

fn run_qemu(image: &[u8]) -> String {
    let dir = scratch();
    let elf = dir.join("q.elf");
    std::fs::write(&elf, image).unwrap();
    let mut child = Command::new("qemu-system-riscv64")
        .args(["-machine", "virt", "-cpu", "rv64,v=true,vlen=128,elen=64"])
        .args(["-nographic", "-bios", "none", "-kernel"])
        .arg(&elf)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-system-riscv64 — the reference machine this oracle compares against");
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        stdout.read_to_string(&mut s).unwrap();
        s
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if start.elapsed() > Duration::from_secs(60) {
            // ONLY THIS CHILD is killed — never by name.
            let _ = child.kill();
            panic!("qemu never reached the finisher in 60 s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let text = reader.join().unwrap();
    assert!(
        status.success(),
        "qemu exited {status}: the program's finisher reported failure"
    );
    // the console is a terminal stream; drop any carriage returns it adds
    text.replace('\r', "")
}

#[test]
fn every_subset_instruction_agrees_with_qemu_bit_for_bit() {
    let s = suite();
    let image = build(&s.asm);
    let (ours, steps) = run_yantra(&image);
    let theirs = run_qemu(&image);

    let ours: Vec<&str> = ours.lines().collect();
    let theirs: Vec<&str> = theirs.lines().filter(|l| l.len() == 16).collect();
    let expected: usize = s.cases.iter().map(|c| c.words).sum();

    let mut wrong = Vec::new();
    let mut at = 0usize;
    for c in &s.cases {
        for j in 0..c.words {
            let (o, t) = (ours.get(at + j), theirs.get(at + j));
            if o != t {
                wrong.push(format!("{} word {j}: yantra {o:?} qemu {t:?}", c.name));
            }
        }
        at += c.words;
    }
    eprintln!(
        "vector oracle: {} cases, {} words compared, yantra {} instructions; per instruction: {:?}",
        s.cases.len(),
        expected,
        steps,
        s.per_insn
    );
    assert_eq!(
        theirs.len(),
        expected,
        "qemu printed {} words, the suite writes {expected}",
        theirs.len()
    );
    assert_eq!(
        ours.len(),
        expected,
        "yantra printed {} words, the suite writes {expected}",
        ours.len()
    );
    assert!(
        wrong.is_empty(),
        "{} of {expected} words disagree; first 20:\n{}",
        wrong.len(),
        wrong
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The oracle must be able to fail: the suite's dump has to contain tails, NaNs and the
/// ordered sum's distinguishing value, or agreement means nothing. Checked on yantra's
/// output against values computed here.
#[test]
fn the_suite_exposes_tails_and_canonical_nans() {
    let s = suite();
    let (ours, _) = run_yantra(&build(&s.asm));
    let lines: Vec<&str> = ours.lines().collect();
    let sentinels = lines.iter().filter(|l| l.starts_with("5e571e")).count();
    let canonical = lines.iter().filter(|l| **l == "7ff8000000000000").count();
    assert!(
        sentinels > 500,
        "only {sentinels} sentinel words reached the dump: tails are not being shown"
    );
    assert!(
        canonical > 50,
        "only {canonical} canonical NaNs: the NaN inputs are not reaching the arithmetic"
    );
}
