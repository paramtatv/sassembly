//! **Row `V-012` — the differential oracle over F and D.**
//!
//! # Why this exists
//!
//! `V-001` implemented F and D in [`yantra`], and its own commit said the honest thing:
//! *"correctness here rests on this tree's own reading of the specification."* That is the
//! twin-versus-truth problem this project already has a name for, and `v0.3.0`'s own
//! release notes name the remedy — an off-the-shelf oracle rather than a second
//! implementation by the same author.
//!
//! `spike` and `qemu-riscv64` are not installed here. `riscv64-elf-objdump` and
//! `qemu-system-riscv64` are, and the first is enough for the DECODE half.
//!
//! # Two oracles, because one of them cannot see the bug V-001 actually had
//!
//! [`binutils_agrees_with_our_table_on_every_float_encoding`] asks GNU binutils to
//! disassemble every float pattern in `spec/encodings-riscv64.tsv` and checks the mnemonic
//! it reports against the name the table gives that row. That is a genuinely independent
//! authority on the ENCODING: a wrong `funct7` in the table, or a wrong `fmt` bit, shows up
//! as a mnemonic mismatch.
//!
//! **IT WOULD NOT HAVE CAUGHT `V-001`'s ONE REAL DEFECT, AND THAT IS THE POINT OF SAYING SO
//! HERE.** `fnmsub` and `fnmadd` were implemented with their signs swapped. The encodings
//! were right; objdump would have agreed with the table and with the machine, and the
//! arithmetic would still have been wrong. Decode is necessary and is not sufficient, so
//! this file does not let `V-012` close on it.
//!
//! [`yantra_implements_every_float_row_the_table_names`] is the other half available
//! without a second simulator: it drives every float pattern through the machine and
//! asserts none of them halts `Unimplemented`. That is a COVERAGE claim rather than a
//! semantic one — it catches a family the table names and the machine forgot, which is the
//! `run_length` ३४ shape in reverse.
//!
//! **WHAT IS STILL OWED, so this file cannot be mistaken for the whole row:** a SEMANTIC
//! oracle. `qemu-system-riscv64` is present and can execute a bare-metal ELF, but building
//! one that exercises floats needs an assembler that emits them, which is row `V-003`. Until
//! that lands, no independent authority has checked what these instructions COMPUTE — only
//! what they decode to and that they run. `V-012` stays open on that.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use yantra::{Csrs, Machine, Privilege};

const BASE: u64 = 0x8000_0000;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

/// One float row of the encoding table: its name and its 32-bit pattern.
struct Row {
    insn: String,
    pattern: u32,
    /// Bits the encoding FIXES. A clear bit is an operand field, and that is what lets
    /// [`Row::probe`] fill `rs1`/`rs2` only where they are operands — `fsqrt` and the
    /// `fcvt` forms encode part of their identity in `rs2`, and writing a register number
    /// there would ask objdump about a different instruction.
    mask: u32,
}

impl Row {
    /// The pattern with DISTINCT register numbers wherever the mask leaves them free.
    ///
    /// **THE BARE PATTERN IS THE WRONG PROBE, AND THIS WAS THIS TEST'S FIRST FINDING
    /// ABOUT ITSELF.** With `rs1 == rs2 == 0` three encodings are canonical
    /// pseudo-instructions and binutils prints them as such — correctly:
    ///
    /// ```text
    ///   fsgnj.d  rd, rs, rs   IS   fmv.d  rd, rs
    ///   fsgnjn.d rd, rs, rs   IS   fneg.d rd, rs
    ///   fsgnjx.d rd, rs, rs   IS   fabs.d rd, rs
    /// ```
    ///
    /// So the oracle reported six disagreements where the table and binutils and the
    /// machine all agreed. Giving `rs1` and `rs2` different numbers removes the alias
    /// without a special case, and using the MASK to decide where they may be written
    /// keeps `fsqrt`/`fcvt`/`fmv.x`/`fclass` intact.
    fn probe(&self) -> u32 {
        const RS1: u32 = 0x000f_8000; // bits 19:15
        const RS2: u32 = 0x01f0_0000; // bits 24:20
        let mut w = self.pattern;
        if self.mask & RS1 == 0 {
            w = (w & !RS1) | (1 << 15);
        }
        if self.mask & RS2 == 0 {
            w = (w & !RS2) | (2 << 20);
        }
        w
    }
}

/// Every float row of `spec/encodings-riscv64.tsv`.
///
/// `fence` and `fence.i` begin with `f` and are MISC-MEM, not floating point; excluding
/// them by name rather than by opcode keeps this readable against the table.
fn float_rows() -> Vec<Row> {
    let path = root().join("spec/encodings-riscv64.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 4 {
            continue;
        }
        let insn = f[0];
        if !insn.starts_with('f') || insn.starts_with("fence") {
            continue;
        }
        let pattern = f[3]
            .strip_prefix("0x")
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .unwrap_or_else(|| panic!("{insn}: pattern `{}` is not hex", f[3]));
        let mask = f[4]
            .strip_prefix("0x")
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .unwrap_or_else(|| panic!("{insn}: mask `{}` is not hex", f[4]));
        out.push(Row {
            insn: insn.to_string(),
            pattern,
            mask,
        });
    }
    assert!(
        out.len() >= 60,
        "expected the whole F and D set in the table and found {} rows — if the table \
         shrank, that is the finding",
        out.len()
    );
    out
}

#[test]
fn binutils_agrees_with_our_table_on_every_float_encoding() {
    let rows = float_rows();

    // ONE objdump invocation over all the patterns concatenated, not one per row: 62
    // process spawns is a slow test, and a slow test gets `#[ignore]`d and then nothing
    // runs it. Every float pattern has `11` in its low two bits, so none decodes as a
    // compressed instruction and the 4-byte stride holds.
    let dir = std::env::temp_dir().join(format!(
        "fp-oracle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let bin = dir.join("patterns.bin");
    {
        let mut f = std::fs::File::create(&bin).expect("create");
        for r in &rows {
            f.write_all(&r.probe().to_le_bytes()).expect("write");
        }
    }

    let out = Command::new("riscv64-elf-objdump")
        .args(["-D", "-b", "binary", "-m", "riscv:rv64"])
        .arg(&bin)
        .output()
        .expect("riscv64-elf-objdump — binutils is required for this oracle, as it is for tests/relocatable.rs");
    assert!(
        out.status.success(),
        "objdump refused the patterns:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dump = String::from_utf8_lossy(&out.stdout);

    // Disassembly lines look like `   0:\t02007053          \tfadd.d\tf0,f0,f0`.
    // Keyed by ADDRESS rather than by line order, so an extra blank or header line in a
    // future binutils cannot silently shift every comparison by one.
    let mut seen: std::collections::BTreeMap<u64, String> = std::collections::BTreeMap::new();
    for line in dump.lines() {
        let Some((addr, rest)) = line.split_once(":\t") else {
            continue;
        };
        let Ok(addr) = u64::from_str_radix(addr.trim(), 16) else {
            continue;
        };
        // after the hex word comes a tab, then the mnemonic
        let mut parts = rest.split('\t').skip(1);
        if let Some(mnemonic) = parts.next() {
            seen.insert(addr, mnemonic.trim().to_string());
        }
    }

    let mut disagreed = Vec::new();
    let mut missing = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let addr = (i * 4) as u64;
        match seen.get(&addr) {
            None => missing.push(format!(
                "{} ({:#010x}) — objdump emitted no line",
                r.insn,
                r.probe()
            )),
            Some(m) if m != &r.insn => disagreed.push(format!(
                "{} ({:#010x}) — our table says `{}`, binutils says `{}`",
                r.insn,
                r.probe(),
                r.insn,
                m
            )),
            Some(_) => {}
        }
    }

    println!("METRIC fp_oracle_decode_rows {}", rows.len());
    println!("METRIC fp_oracle_decode_disagreements {}", disagreed.len());

    assert!(
        missing.is_empty(),
        "binutils produced no disassembly for {} pattern(s), so those rows are UNCHECKED \
         rather than agreeing:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
    assert!(
        disagreed.is_empty(),
        "GNU binutils disagrees with `spec/encodings-riscv64.tsv` on {} row(s). binutils is \
         the independent authority here, so the table is what moves:\n  {}",
        disagreed.len(),
        disagreed.join("\n  ")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn yantra_implements_every_float_row_the_table_names() {
    // A COVERAGE claim, not a semantic one: every float encoding the table names must
    // RETIRE — one step, no halt of any kind. This catches a family the table has and the
    // machine forgot — which is exactly how F and D looked before `V-001`.
    //
    // **ANY HALT, NOT ONLY `Unimplemented` (V-009 part (i-c)).** The first version failed
    // on `Unimplemented` alone, and so had two false greens. Under the FS gate every row
    // halted `Undelivered { cause: 2 }` and this still passed; and on main before (i-c)
    // the four loads and stores halted `BadAccess` at address 0 or 2 — `x1`, the `rs1`
    // the probe names, was 0 — so they never ran at all. Now `x1` points into RAM, FS is
    // Initial as the startup sets it, and every halt is a failure, named.
    let rows = float_rows();
    let mut halted = Vec::new();

    for r in &rows {
        let mut m = Machine {
            store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
            patra_root: None,
            patra_mem: None,
            patra_path: None,
            patra_buffer: None,
            virtio: Default::default(),
            x: [0; 32],
            f: [0; 32],
            fcsr: 0, // frm = RNE, so a `dyn` pattern resolves to a mode we honour
            pc: BASE,
            base: BASE,
            mem: vec![0; 1 << 16],
            reservation: None,
            // FS = Initial: a float instruction under FS = Off is illegal (V-009 (i-c)).
            csr: Csrs {
                sstatus: 0b01 << 13,
                ..Csrs::default()
            },
            mode: Privilege::Supervisor,
            time: 0,
            timecmp: None,
            vec: Default::default(),
            socket: None,
            net: None,
        };
        m.mem[0..4].copy_from_slice(&r.probe().to_le_bytes());
        // Give the loads and stores an address inside RAM, so a genuine implementation is
        // not mistaken for a fault: [`Row::probe`] names `x1` as `rs1` wherever the mask
        // leaves it free, and the loads' and stores' offset is the probe's `rs2` field
        // (2) at most, so `x1 = BASE + 0x1000` keeps every access on one RAM page. The
        // comment this replaces said the target page was "at offset 0, which BASE already
        // is" — it was not: the access went to address 0 and halted `BadAccess`.
        m.x[1] = BASE + 0x1000;
        if let Some(h) = m.step(&mut Vec::new()) {
            halted.push(format!("{} ({:#010x}): {h:?}", r.insn, r.probe()));
        }
    }

    println!("METRIC fp_oracle_coverage_rows {}", rows.len());
    println!("METRIC fp_oracle_coverage_halted {}", halted.len());

    assert!(
        halted.is_empty(),
        "{} float encoding(s) the table names halt on this machine instead of retiring. \
         The table and the machine must agree about what exists:\n  {}",
        halted.len(),
        halted.join("\n  ")
    );
}
