//! The calling convention — measured over the T0 corpus and pinned against
//! doc 02 §2.4. Task `W-234`, the row W-211's census filed.
//!
//! # What was found, and what this file does about it
//!
//! W-211 (`paradigm_t0.rs`, `measure_corpus_calls_and_labels`) read every
//! `jal`/`jalr`/`ecall` site of the 82 hand-written T0 programs and found the
//! convention in use disagreeing with doc 02 §2.4 in three places: arguments
//! travel in temporaries, not `a0–a7`; temporaries are read first after a call
//! against "momentary"; `a7` carries the ecall number while the doc named no
//! number register. Research/25 §2.3 (0e11ea7e) then decided that the emitter
//! USES the doc's role table with `a7` named, and does not copy the t-register
//! habit — because that habit is LEAF code's: the corpus has almost no frames
//! (one program, `spec/golden/34-stack-frame.sas`, writes `स्तूपसूचकः`, and it
//! is the B-062 exhibit of a prologue/epilogue with nothing between them). So
//! W-234 CORRECTS THE DOC, not the emitter: doc 02 §2.4 now states the LEAF
//! convention (measured) and the FRAMED convention (what the emitter writes)
//! apart, with the rule for which applies.
//!
//! This file is the ratchet on that correction:
//!
//! 1. **The census** (`census_of_the_convention`) walks the same sites with
//!    W-211's walker — accesses read off the decoded encoding's field masks,
//!    never a hand table — and prints the convention's numbers as `METRIC`
//!    lines. They are PINNED: doc 02 §2.4's dated note quotes them, so a corpus
//!    change that moves one fails here, naming the note to update.
//! 2. **The agreement test** reads §2.4's register table out of the document
//!    and checks every register the table gives a convention role — argument,
//!    return, callee-saved, temporary, return address, stack pointer, ecall
//!    number — is one the census observed in the corpus. A register the doc
//!    names and the corpus never touches is reported by name; a convention
//!    role given to such a register FAILS by name (the refused case).
//! 3. **The ecall-number test** reads which register §2.4 names for the ecall
//!    number and requires it to be the one the census sees written last before
//!    the `ecall`. Against the text before W-234 this failed with
//!    `doc 02 §2.4 names no register for the ecall number; the census observes
//!    a7 written last before the ecall at 170 of 175 sites (row 2.4.4 open)` —
//!    the fail-first this row recorded.
//! 4. **The sigil census** (`census_of_the_sigils`, research/23 statistic 26,
//!    folded in from W-214's delivery) counts every operand's kāraka sigil by
//!    sigil and by extension, the ADR-0005 conjunct fusions, and the grammar's
//!    declared set against the written one. Finding: `ए` (locus) is declared
//!    and written by no program — 0 of 22,087 operands — and is LISTED as such,
//!    pinned, not folded into a zero.
//!
//! The helpers below are W-211's (`root`, `sources`, `t0_corpus`, `accesses`,
//! `window`, `abi_names`), copied rather than shared: the crate has no
//! `tests/common`, and W-233 was editing `paradigm_t0.rs` in parallel.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sadhana::encode::{EncodeError, Target, encode_object_for, encodings, layout_addresses};
use sadhana::parse::{Program, assemble_program};
use sadhana::vishlesana::decode_at;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// Every file under `dir` with `ext`, sorted, as (path relative to the root,
/// contents).
fn sources(dir: &str, ext: &str) -> Vec<(String, String)> {
    let base = root().join(dir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("read {}: {e}", base.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("read source");
            (
                format!("{dir}/{}", p.file_name().unwrap().to_string_lossy()),
                text,
            )
        })
        .collect()
}

/// The T0 corpus: `spec/*.sas` and `spec/golden/*.sas` — the 82 hand-written
/// programs. This is the corpus that is canonical for the LEAF convention
/// (ADR-0026's direction applied to the code that writes the construct); the
/// FRAMED convention's corpus is the one the emitter writes, and it is
/// measured by W-237's census when it exists.
fn t0_corpus() -> Vec<(String, String)> {
    let mut all = sources("spec", "sas");
    all.extend(sources("spec/golden", "sas"));
    all
}

// ── W-211's walker ─────────────────────────────────────────────────────────

/// How far back and forward a call site is read, in instructions.
const WINDOW: usize = 8;

const RD: u32 = 0x0000_0f80;
const RS1: u32 = 0x000f_8000;
const RS2: u32 = 0x01f0_0000;
const RS3: u32 = 0xf800_0000;

const SP: u32 = 2;
const A0: u32 = 10;
const A7: u32 = 17;

/// One instruction's effect on the integer register file, read off the
/// decoded encoding's field masks.
#[derive(Debug, Clone, Default)]
struct Access {
    insn: String,
    writes: BTreeSet<u32>,
    reads: BTreeSet<u32>,
    rd: Option<u32>,
    rs1: Option<u32>,
}

/// The accesses of every instruction of `program`, uncompressed.
fn accesses(program: &Program) -> Result<Vec<Access>, Vec<EncodeError>> {
    let (text, _) = encode_object_for(program, Target::Uncompressed)?;
    let addresses = layout_addresses(program, Target::Uncompressed);
    let all = encodings();
    let mut out = Vec::with_capacity(program.instructions.len());
    for off in addresses.into_iter().take(program.instructions.len()) {
        let mut a = Access::default();
        if let Some((d, _)) = decode_at(&text, off as usize)
            && let Some(e) = all.iter().find(|e| e.insn == d.insn && e.bits == 32)
        {
            a.insn = d.insn.clone();
            for (slot, (kind, value)) in e.slots.iter().zip(&d.operands) {
                if kind == "freg" {
                    continue;
                }
                let v = u32::try_from(*value).unwrap_or(0);
                match slot.mask {
                    RD => {
                        a.rd = Some(v);
                        if v != 0 {
                            a.writes.insert(v);
                        }
                    }
                    RS1 => {
                        a.rs1 = Some(v);
                        a.reads.insert(v);
                    }
                    RS2 | RS3 => {
                        a.reads.insert(v);
                    }
                    _ => {}
                }
            }
        }
        out.push(a);
    }
    Ok(out)
}

/// What a window around one site saw.
#[derive(Debug, Default)]
struct Site {
    written_before: BTreeSet<u32>,
    read_first_after: BTreeSet<u32>,
    /// The register written LAST before the site — for `ecall`, the one
    /// carrying the call number.
    last_written_before: Option<u32>,
}

fn window(acc: &[Access], i: usize) -> Site {
    let mut s = Site::default();
    let start = i.saturating_sub(WINDOW);
    for a in &acc[start..i] {
        for w in &a.writes {
            s.written_before.insert(*w);
            s.last_written_before = Some(*w);
        }
    }
    let end = (i + 1 + WINDOW).min(acc.len());
    let mut touched: BTreeSet<u32> = BTreeSet::new();
    for a in &acc[i + 1..end] {
        for r in &a.reads {
            if *r != 0 && touched.insert(*r) {
                s.read_first_after.insert(*r);
            }
        }
        for w in &a.writes {
            touched.insert(*w);
        }
    }
    s
}

/// Per-register tallies over a set of sites.
#[derive(Debug, Default)]
struct Tally {
    sites: usize,
    written_before: BTreeMap<u32, usize>,
    read_first: BTreeMap<u32, usize>,
    last_written: BTreeMap<u32, usize>,
}

impl Tally {
    fn add(&mut self, s: &Site) {
        self.sites += 1;
        for r in &s.written_before {
            *self.written_before.entry(*r).or_default() += 1;
        }
        for r in &s.read_first_after {
            *self.read_first.entry(*r).or_default() += 1;
        }
        if let Some(r) = s.last_written_before {
            *self.last_written.entry(r).or_default() += 1;
        }
    }
}

/// Integer register names from `spec/registers-riscv64.tsv`: both the
/// Devanagari spelling and the ABI name resolve to the number.
fn register_numbers() -> BTreeMap<String, u32> {
    let text = std::fs::read_to_string(root().join("spec/registers-riscv64.tsv"))
        .expect("registers table");
    let mut out = BTreeMap::new();
    for l in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t"))
    {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() >= 4
            && f[3] == "int"
            && let Ok(n) = f[2].parse::<u32>()
        {
            out.insert(f[0].to_string(), n);
            out.insert(f[1].to_string(), n);
            // Doc 02 writes the zero register as `x0`.
            out.insert(format!("x{n}"), n);
        }
    }
    out
}

/// ABI name of an integer register.
fn abi_names() -> BTreeMap<u32, String> {
    register_numbers()
        .into_iter()
        .filter(|(name, _)| name.is_ascii() && !name.starts_with('x'))
        .map(|(name, n)| (n, name))
        .collect()
}

/// Doc 02 §2.4's declared roles, by ABI name prefix.
fn declared_role(abi: &str) -> &'static str {
    if abi.starts_with('a') {
        "args/return"
    } else if abi.starts_with('s') && abi != "sp" {
        "callee-saved"
    } else if abi.starts_with('t') && abi != "tp" {
        "temporary"
    } else {
        "special"
    }
}

// ── the census ─────────────────────────────────────────────────────────────

/// The convention as the corpus practises it, counted once per test binary.
#[derive(Debug, Default)]
struct Census {
    programs: usize,
    undecoded: usize,
    jal_calls: Tally,
    jalr_calls: Tally,
    ecalls: Tally,
    /// Every register read or written anywhere in the corpus, by number.
    observed: BTreeSet<u32>,
    /// `स्तूपसूचकः` writes, and the programs that make them.
    sp_writes: usize,
    sp_writers: Vec<String>,
    /// `file:line abi` for each temporary read first after a `jal` call.
    temporaries_read_first_after_call: Vec<String>,
    /// Statistic 26: operands by sigil; by (extension, sigil); operands whose
    /// sigil was fused into the base's final akṣara (ADR-0005), by sigil.
    operands: usize,
    sigils: BTreeMap<&'static str, usize>,
    sigils_by_ext: BTreeMap<(String, &'static str), usize>,
    fused: BTreeMap<&'static str, usize>,
    /// `file:line base+sigil` for each fused `न` or `ए` (statistic 26 asks).
    fused_source_or_locus: Vec<String>,
}

fn census() -> &'static Census {
    static CENSUS: OnceLock<Census> = OnceLock::new();
    CENSUS.get_or_init(|| {
        let names = abi_names();
        let mut c = Census::default();
        for (name, text) in t0_corpus() {
            let Ok(program) = assemble_program(&text) else {
                continue;
            };
            c.programs += 1;
            for inst in &program.instructions {
                for o in &inst.operands {
                    let sigil = o.karaka.sigil();
                    c.operands += 1;
                    *c.sigils.entry(sigil).or_default() += 1;
                    *c.sigils_by_ext
                        .entry((inst.family.ext.clone(), sigil))
                        .or_default() += 1;
                    // ADR-0005: a base ending in a virama conjoins with a
                    // CONSONANT sigil into one akṣara — `पुनःस्थानम्` + `म्` is
                    // `म्म्`, + `न` is `म्न`; the independent vowel `ए` stays
                    // its own akṣara. The parser has already split it; this
                    // counts where it had to.
                    if o.base.ends_with('\u{094D}') && sigil != "ए" {
                        *c.fused.entry(sigil).or_default() += 1;
                        if sigil == "न" {
                            c.fused_source_or_locus
                                .push(format!("{name}:{} {}{sigil}", inst.line, o.base));
                        }
                    }
                }
            }
            let Ok(acc) = accesses(&program) else {
                continue;
            };
            let mut wrote_sp = false;
            for (i, a) in acc.iter().enumerate() {
                if a.insn.is_empty() {
                    c.undecoded += 1;
                    continue;
                }
                // `x0` counts as observed when read: `योगः … शून्यःन …` is
                // how a constant is materialised, so it is read everywhere.
                c.observed.extend(&a.reads);
                c.observed.extend(&a.writes);
                if a.writes.contains(&SP) {
                    c.sp_writes += 1;
                    wrote_sp = true;
                }
                let line = program.instructions[i].line;
                match a.insn.as_str() {
                    "jal" if a.rd != Some(0) => {
                        let s = window(&acc, i);
                        for r in &s.read_first_after {
                            let abi = names.get(r).cloned().unwrap_or_default();
                            if declared_role(&abi) == "temporary" {
                                c.temporaries_read_first_after_call
                                    .push(format!("{name}:{line} {abi}"));
                            }
                        }
                        c.jal_calls.add(&s);
                    }
                    "jalr" if a.rd != Some(0) => c.jalr_calls.add(&window(&acc, i)),
                    "ecall" => c.ecalls.add(&window(&acc, i)),
                    _ => {}
                }
            }
            if wrote_sp {
                c.sp_writers.push(name);
            }
        }
        c
    })
}

fn fraction(num: usize, den: usize) -> String {
    if den == 0 {
        "0.0000".to_string()
    } else {
        format!("{:.4}", num as f64 / den as f64)
    }
}

/// The share of a per-register map that falls in one of doc 02's roles.
fn share(m: &BTreeMap<u32, usize>, role: &str, names: &BTreeMap<u32, String>) -> (usize, usize) {
    let total: usize = m.values().sum();
    let in_role: usize = m
        .iter()
        .filter(|(r, _)| names.get(r).is_some_and(|n| declared_role(n) == role))
        .map(|(_, c)| c)
        .sum();
    (in_role, total)
}

/// What the census measured on 2026-09-04 (re-pinned 2026-10-06 for F-009's
/// play-out wait in spec/virtio-sound.sas), and what doc 02 §2.4's dated note
/// quotes. Order: written before a `jal` call, per temporary.
const PINNED_T_ARGS: [(&str, usize); 7] = [
    ("t0", 323),
    ("t1", 246),
    ("t2", 374),
    ("t3", 48),
    ("t4", 18),
    ("t5", 15),
    ("t6", 126),
];
const PINNED_PROGRAMS: usize = 82;
const PINNED_JAL_CALLS: usize = 686;
const PINNED_JALR_CALLS: usize = 5;
const PINNED_ECALLS: usize = 175;
const PINNED_ARGS_IN_A_REGS: &str = "0.0590";
const PINNED_ECALL_NUMBER_IN_A7: &str = "0.9714";
const PINNED_ECALL_NUMBER_IN_A7_SITES: usize = 170;
const PINNED_TEMPORARIES_READ_FIRST_AFTER_CALL: usize = 181;
const PINNED_READ_FIRST_IN_S_REGS: &str = "0.8192";
const PINNED_SP_WRITES: usize = 2;
const PINNED_FRAMES_WRITTEN: &[&str] = &["spec/golden/34-stack-frame.sas"];

const NOTE: &str = "doc 02 §2.4, the dated note of 2026-09-04 (W-234)";

/// The convention the 82 programs practise, as numbers, pinned.
#[test]
fn census_of_the_convention() {
    let c = census();
    let names = abi_names();
    println!("METRIC paradigm_convention_programs {}", c.programs);
    println!("METRIC paradigm_convention_window {WINDOW}");
    println!(
        "METRIC paradigm_convention_undecoded_instructions {}",
        c.undecoded
    );
    println!("METRIC paradigm_convention_jal_calls {}", c.jal_calls.sites);
    println!(
        "METRIC paradigm_convention_jalr_calls {}",
        c.jalr_calls.sites
    );
    println!("METRIC paradigm_convention_ecall_sites {}", c.ecalls.sites);

    // (a) arguments: what is written before a `jal` call.
    let (a_args, all_args) = share(&c.jal_calls.written_before, "args/return", &names);
    let args_in_a = fraction(a_args, all_args);
    println!("METRIC paradigm_convention_args_in_a_regs_fraction {args_in_a}");
    let mut t_args: Vec<(String, usize)> = Vec::new();
    for (r, n) in &c.jal_calls.written_before {
        let abi = names.get(r).cloned().unwrap_or_else(|| format!("x{r}"));
        if declared_role(&abi) == "temporary" {
            println!("METRIC paradigm_convention_args_in_t_regs_sites_{abi} {n}");
            t_args.push((abi, *n));
        }
    }

    // (b) what is read first after a `jal` call, by role.
    let (s_ret, all_ret) = share(&c.jal_calls.read_first, "callee-saved", &names);
    let (t_ret, _) = share(&c.jal_calls.read_first, "temporary", &names);
    let (a_ret, _) = share(&c.jal_calls.read_first, "args/return", &names);
    let read_first_in_s = fraction(s_ret, all_ret);
    println!(
        "METRIC paradigm_convention_read_first_after_call_in_s_regs_fraction {read_first_in_s}"
    );
    println!(
        "METRIC paradigm_convention_read_first_after_call_in_t_regs_fraction {}",
        fraction(t_ret, all_ret)
    );
    println!(
        "METRIC paradigm_convention_read_first_after_call_in_a_regs_fraction {}",
        fraction(a_ret, all_ret)
    );
    println!(
        "METRIC paradigm_convention_temporaries_read_first_after_call {}",
        c.temporaries_read_first_after_call.len()
    );

    // (c) the ecall number, and the ecall's return.
    let a7 = c.ecalls.last_written.get(&A7).copied().unwrap_or(0);
    let ecall_in_a7 = fraction(a7, c.ecalls.sites);
    println!("METRIC paradigm_convention_ecall_number_in_a7_sites {a7}");
    println!("METRIC paradigm_convention_ecall_number_in_a7_fraction {ecall_in_a7}");
    println!(
        "METRIC paradigm_convention_ecall_return_read_first_in_a0_sites {}",
        c.ecalls.read_first.get(&A0).copied().unwrap_or(0)
    );

    // Frames: writes to `स्तूपसूचकः`, and the programs that make them.
    println!("METRIC paradigm_convention_sp_writes {}", c.sp_writes);
    println!(
        "METRIC paradigm_convention_frames_written {}",
        c.sp_writers.len()
    );
    for p in &c.sp_writers {
        println!("  FRAME {p}");
    }

    assert_eq!(c.undecoded, 0, "every instruction decodes");
    assert_eq!(c.programs, PINNED_PROGRAMS, "programs — update {NOTE}");
    assert_eq!(
        c.jal_calls.sites, PINNED_JAL_CALLS,
        "jal calls — update {NOTE}"
    );
    assert_eq!(
        c.jalr_calls.sites, PINNED_JALR_CALLS,
        "jalr calls — update {NOTE}"
    );
    assert_eq!(c.ecalls.sites, PINNED_ECALLS, "ecall sites — update {NOTE}");
    assert_eq!(
        args_in_a, PINNED_ARGS_IN_A_REGS,
        "args in a-regs — update {NOTE}"
    );
    let pinned: Vec<(String, usize)> = PINNED_T_ARGS
        .iter()
        .map(|(r, n)| (r.to_string(), *n))
        .collect();
    assert_eq!(
        t_args, pinned,
        "temporaries written before a call — update {NOTE}"
    );
    assert_eq!(
        read_first_in_s, PINNED_READ_FIRST_IN_S_REGS,
        "read first after a call in s-regs — update {NOTE}"
    );
    assert_eq!(
        c.temporaries_read_first_after_call.len(),
        PINNED_TEMPORARIES_READ_FIRST_AFTER_CALL,
        "temporaries read first after a call — update {NOTE}"
    );
    assert_eq!(
        a7, PINNED_ECALL_NUMBER_IN_A7_SITES,
        "a7 as the ecall number — update {NOTE}"
    );
    assert_eq!(ecall_in_a7, PINNED_ECALL_NUMBER_IN_A7);
    assert_eq!(
        c.sp_writes, PINNED_SP_WRITES,
        "स्तूपसूचकः writes — update {NOTE}"
    );
    assert_eq!(
        c.sp_writers, PINNED_FRAMES_WRITTEN,
        "programs writing a frame — update {NOTE}"
    );
}

// ── the document ───────────────────────────────────────────────────────────

fn doc_02() -> String {
    std::fs::read_to_string(root().join("research/02-sassembly-isa-and-language.md"))
        .expect("research/02 exists")
}

/// §2.4 of doc 02: from its heading to the next `###`.
fn section_2_4(doc: &str) -> &str {
    let start = doc
        .find("### 2.4 Registers")
        .expect("doc 02 has a §2.4 Registers heading");
    let rest = &doc[start..];
    let end = rest[3..].find("\n### ").map_or(rest.len(), |i| i + 3);
    &rest[..end]
}

/// One row of §2.4's register table that names integer registers.
#[derive(Debug, PartialEq)]
struct DocRow {
    /// ABI names, ranges expanded.
    registers: Vec<String>,
    role: String,
    line: usize,
}

/// Expand `t0–t6` (en dash or hyphen) to `t0 … t6`; a single name is itself.
fn expand(token: &str) -> Vec<String> {
    let token = token.trim_matches('`').trim();
    let Some((lo, hi)) = token.split_once(['–', '-']) else {
        return vec![token.to_string()];
    };
    let prefix: String = lo.chars().take_while(|c| !c.is_ascii_digit()).collect();
    let (Ok(a), Ok(b)) = (
        lo[prefix.len()..].parse::<u32>(),
        hi.trim_start_matches(prefix.as_str()).parse::<u32>(),
    ) else {
        return vec![token.to_string()];
    };
    (a..=b).map(|n| format!("{prefix}{n}")).collect()
}

/// The register-table rows of `section`, in written order, keeping only those
/// whose first column resolves to integer registers.
fn register_rows(section: &str, numbers: &BTreeMap<String, u32>) -> Vec<DocRow> {
    let mut rows = Vec::new();
    for (i, l) in section.lines().enumerate() {
        let Some(body) = l.trim().strip_prefix('|') else {
            continue;
        };
        let cols: Vec<&str> = body.split('|').map(str::trim).collect();
        if cols.len() < 3 {
            continue;
        }
        let names: Vec<String> = cols[0].split(',').flat_map(expand).collect();
        if names.is_empty() || !names.iter().all(|n| numbers.contains_key(n)) {
            continue;
        }
        rows.push(DocRow {
            registers: names,
            role: cols[1].to_string(),
            line: i + 1,
        });
    }
    rows
}

/// A role the calling convention assigns, as opposed to a machine's fixed
/// register (`zero`, `gp`, `tp`, `pc`).
fn is_convention_role(role: &str) -> bool {
    let r = role.to_ascii_lowercase();
    [
        "arg",
        "return",
        "callee-saved",
        "temporar",
        "stack pointer",
        "ecall number",
    ]
    .iter()
    .any(|k| r.contains(k))
}

/// Which register-table rows name a register the census never observed:
/// `(row line, role, unobserved ABI names)`. A row with a convention role is
/// the refused case; any row is reported.
fn unobserved_rows(
    rows: &[DocRow],
    observed: &BTreeSet<String>,
) -> Vec<(usize, String, Vec<String>)> {
    rows.iter()
        .filter_map(|row| {
            let missing: Vec<String> = row
                .registers
                .iter()
                .filter(|r| !observed.contains(*r))
                .cloned()
                .collect();
            (!missing.is_empty()).then(|| (row.line, row.role.clone(), missing))
        })
        .collect()
}

/// The single registers (never a range) named in backticks on the lines of
/// `section` that say "ecall number", resolved to numbers.
fn ecall_number_registers(section: &str, numbers: &BTreeMap<String, u32>) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for l in section.lines().filter(|l| l.contains("ecall number")) {
        let mut parts = l.split('`');
        parts.next();
        while let (Some(inner), Some(_)) = (parts.next(), parts.next()) {
            if !inner.contains(['–', '-'])
                && let Some(n) = numbers.get(inner.trim())
            {
                out.insert(*n);
            }
        }
    }
    out
}

/// The observed registers by every name the doc may use: ABI name and `x<n>`.
fn observed_abi(c: &Census) -> BTreeSet<String> {
    let names = abi_names();
    c.observed
        .iter()
        .flat_map(|r| [names.get(r).cloned(), Some(format!("x{r}"))])
        .flatten()
        .collect()
}

/// Doc/census agreement: every register §2.4's table gives a convention role
/// is one the corpus touches. Registers the doc names outside the convention
/// and the corpus never touches are reported, not refused.
#[test]
fn every_register_doc_02_gives_a_convention_role_is_observed_in_the_corpus() {
    let numbers = register_numbers();
    let doc = doc_02();
    let rows = register_rows(section_2_4(&doc), &numbers);
    assert!(rows.len() >= 8, "§2.4's register table was read: {rows:?}");
    let observed = observed_abi(census());
    println!(
        "METRIC paradigm_convention_observed_int_registers {}",
        census().observed.len()
    );
    println!(
        "METRIC paradigm_convention_doc_register_rows {}",
        rows.len()
    );
    let missing = unobserved_rows(&rows, &observed);
    let mut refused = Vec::new();
    for (line, role, regs) in &missing {
        println!(
            "  UNOBSERVED §2.4 line {line} ({role}): {}",
            regs.join(", ")
        );
        if is_convention_role(role) {
            refused.push(format!(
                "§2.4 row at line {line} gives {} the role \"{role}\"",
                regs.join(", ")
            ));
        }
    }
    println!(
        "METRIC paradigm_convention_doc_registers_unobserved {}",
        missing.len()
    );
    assert!(
        refused.is_empty(),
        "doc 02 §2.4 gives a convention role to a register the census never observes: {refused:?}"
    );
}

/// The register §2.4 names for the ecall number is the one the census sees
/// written last before the `ecall`. Fail-first against the text before W-234:
/// `doc 02 §2.4 names no register for the ecall number; the census observes a7
/// written last before the ecall at 170 of 175 sites (row 2.4.4 open)`.
#[test]
fn the_ecall_number_register_doc_02_names_is_the_one_the_census_observes() {
    let c = census();
    let names = abi_names();
    let (by_practice, count) = c
        .ecalls
        .last_written
        .iter()
        .max_by_key(|(_, n)| **n)
        .map(|(r, n)| (*r, *n))
        .expect("ecall sites");
    let practice = names.get(&by_practice).cloned().unwrap_or_default();
    let doc = doc_02();
    let named = ecall_number_registers(section_2_4(&doc), &register_numbers());
    assert!(
        !named.is_empty(),
        "doc 02 §2.4 names no register for the ecall number; the census observes {practice} written last before the ecall at {count} of {} sites (row 2.4.4 open)",
        c.ecalls.sites
    );
    let named_abi: Vec<String> = named
        .iter()
        .map(|r| names.get(r).cloned().unwrap_or_else(|| format!("x{r}")))
        .collect();
    assert_eq!(
        named,
        BTreeSet::from([by_practice]),
        "doc 02 §2.4 names {named_abi:?} for the ecall number; the census observes {practice} at {count} of {} sites",
        c.ecalls.sites
    );
}

// ── tests of the instruments ───────────────────────────────────────────────

const SYNTHETIC_2_4: &str = "### 2.4 Registers

| RISC-V | Role | Sassembly | Gloss |
|---|---|---|---|
| `x0` | hardwired zero | `शून्यः` | zero |
| `tp` | ecall number | `तन्तुसूचकः` | thread-pointer |
| `t0–t6` | temporaries | `क्षणिक०–क्षणिक६` | momentary |
| `a0–a7` | args/return | `अर्थ०–अर्थ७` | argument/value |
| `f0–f31` | float | `प्लव०–प्लव३१` | fractional |
| `pc` | program counter | `क्रमसूचकः` | sequence-pointer |

### 2.5 Labels
| `a7` | this row is in the next section | x | y |
";

/// REFUSED: a table row that gives a convention role to a register the corpus
/// never touches (`tp`, 0 reads and 0 writes in 82 programs) is named — the
/// row, the role and the register.
#[test]
fn a_doc_row_naming_a_register_the_census_never_observes_is_refused_by_name() {
    let numbers = register_numbers();
    let rows = register_rows(section_2_4(SYNTHETIC_2_4), &numbers);
    assert_eq!(
        rows.len(),
        4,
        "x0, tp, t0–t6, a0–a7 — not f, pc, or §2.5: {rows:?}"
    );
    assert_eq!(
        rows[2].registers,
        ["t0", "t1", "t2", "t3", "t4", "t5", "t6"]
    );
    let observed = observed_abi(census());
    assert!(
        !observed.contains("tp"),
        "the refused case needs tp unobserved"
    );
    let missing = unobserved_rows(&rows, &observed);
    assert_eq!(missing.len(), 1, "{missing:?}");
    let (line, role, regs) = &missing[0];
    assert_eq!(
        (*line, role.as_str(), regs.as_slice()),
        (6, "ecall number", &["tp".to_string()][..])
    );
    assert!(is_convention_role(role));
    assert!(!is_convention_role("thread pointer") && !is_convention_role("hardwired zero"));
    // And the number test would refuse the same row: tp is named, a7 is practised.
    let named = ecall_number_registers(section_2_4(SYNTHETIC_2_4), &numbers);
    assert_eq!(named, BTreeSet::from([4]), "tp is x4");
    assert_ne!(named, BTreeSet::from([A7]));
}

/// The range reader expands both dashes and leaves a single name alone.
#[test]
fn register_ranges_in_the_doc_are_expanded() {
    assert_eq!(expand("`s0–s11`").len(), 12);
    assert_eq!(
        expand("a0-a7"),
        ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"]
    );
    assert_eq!(expand("`ra`"), ["ra"]);
    assert_eq!(expand("pc"), ["pc"]);
}

// ── statistic 26: the sigils ───────────────────────────────────────────────
//
// research/23 §2 statistic 26 (S2, T0 corpus): per-sigil counts; the
// conjunct-fusion rate ADR-0005 measured on the conformance suite (439 of
// 2,862 = 15.3%) re-measured on the programs; occurrences of `न`/`ए` fusing
// with a preceding virama. Expected: the lexer's split handles 100% — which
// is what `paradigm_convention_programs 82` already says, since a fused sigil
// the lexer missed would be an operand that did not parse.

/// The sigils `spec/grammar-t0.ebnf` declares: `(production, sigil)`, in the
/// order the `karaka` production lists its alternatives.
fn declared_sigils(grammar: &str) -> Vec<(String, String)> {
    let start = grammar
        .find("\nkaraka ")
        .expect("grammar has a karaka production");
    let production = &grammar[start..];
    let production = &production[..production.find(';').expect("karaka production ends")];
    let mut names: Vec<&str> = Vec::new();
    for l in production.lines() {
        let l = l.split("(*").next().unwrap_or("");
        let l = l.split_once('=').map_or(l, |(_, r)| r);
        names.extend(
            l.split('|')
                .map(str::trim)
                .filter(|w| !w.is_empty() && *w != "karaka"),
        );
    }
    names
        .into_iter()
        .map(|n| {
            let line = grammar
                .lines()
                .find(|l| l.starts_with(n) && l[n.len()..].trim_start().starts_with('='))
                .unwrap_or_else(|| panic!("grammar defines {n}"));
            let sigil = line.split('"').nth(1).expect("a quoted sigil");
            (n.to_string(), sigil.to_string())
        })
        .collect()
}

/// Declared sigils no program writes — LISTED, never hidden.
fn declared_unwritten(declared: &[(String, String)], written: &BTreeSet<&str>) -> Vec<String> {
    declared
        .iter()
        .filter(|(_, s)| !written.contains(s.as_str()))
        .map(|(n, s)| format!("{n} = \"{s}\""))
        .collect()
}

/// What the census measured on 2026-09-04 (re-pinned 2026-10-06, F-009); doc 02 §2.2's sigils, by count.
const PINNED_OPERANDS: usize = 22_101;
const PINNED_SIGILS: [(&str, usize); 5] = [
    ("म्", 6_470),
    ("न", 12_152),
    ("त्", 1_047),
    ("य्", 2_432),
    ("ए", 0),
];
const PINNED_FUSED: usize = 1_902;
const PINNED_FUSED_RATE: &str = "0.0861";
/// The finding: the grammar declares five sigils and the programs write four.
/// `ए` (अधिकरण, locus) is written by no program — doc 02's open question
/// O-02-2 ("what does अधिकरण mark?") has, in the corpus, the answer "nothing
/// yet". Listed, not hidden; pinned so that the first program to write one
/// moves this line and the doc's note together.
const PINNED_DECLARED_UNWRITTEN: &[&str] = &["locus = \"ए\""];

/// The sigils the 82 programs write, per sigil and per extension; the fusion
/// rate; and the grammar's declared set against the observed one.
#[test]
fn census_of_the_sigils() {
    let c = census();
    let grammar =
        std::fs::read_to_string(root().join("spec/grammar-t0.ebnf")).expect("grammar-t0.ebnf");
    let declared = declared_sigils(&grammar);
    println!("METRIC paradigm_sigils_operands {}", c.operands);
    println!("METRIC paradigm_sigils_declared {}", declared.len());
    println!("METRIC paradigm_sigils_observed {}", c.sigils.len());
    let mut measured: Vec<(&str, usize)> = Vec::new();
    for (name, sigil) in &declared {
        let n = c.sigils.get(sigil.as_str()).copied().unwrap_or(0);
        println!("METRIC paradigm_sigils_{name} {n}");
        measured.push((sigil.as_str(), n));
    }
    let exts: BTreeSet<&str> = c.sigils_by_ext.keys().map(|(e, _)| e.as_str()).collect();
    for e in exts {
        for (name, sigil) in &declared {
            let n = c
                .sigils_by_ext
                .get(&(e.to_string(), sigil.as_str()))
                .copied()
                .unwrap_or(0);
            println!("METRIC paradigm_sigils_by_ext_{e}_{name} {n}");
        }
    }
    let fused: usize = c.fused.values().sum();
    println!("METRIC paradigm_sigils_fused_into_base {fused}");
    println!(
        "METRIC paradigm_sigils_fused_rate {}",
        fraction(fused, c.operands)
    );
    for (name, sigil) in &declared {
        println!(
            "METRIC paradigm_sigils_fused_{name} {}",
            c.fused.get(sigil.as_str()).copied().unwrap_or(0)
        );
    }
    println!(
        "METRIC paradigm_sigils_fused_source_or_locus {}",
        c.fused_source_or_locus.len()
    );
    for f in c.fused_source_or_locus.iter().take(5) {
        println!("  FUSED {f}");
    }
    let written: BTreeSet<&str> = c.sigils.keys().copied().collect();
    let unwritten = declared_unwritten(&declared, &written);
    println!(
        "METRIC paradigm_sigils_declared_unwritten {}",
        unwritten.len()
    );
    for u in &unwritten {
        println!("  DECLARED, UNWRITTEN {u}");
    }
    let undeclared: Vec<&&str> = written
        .iter()
        .filter(|s| !declared.iter().any(|(_, d)| d == **s))
        .collect();
    println!(
        "METRIC paradigm_sigils_written_undeclared {}",
        undeclared.len()
    );

    assert_eq!(
        declared.len(),
        5,
        "the grammar declares five kāraka sigils: {declared:?}"
    );
    assert!(
        undeclared.is_empty(),
        "a sigil the parser accepts and the grammar does not declare: {undeclared:?}"
    );
    assert_eq!(c.operands, PINNED_OPERANDS, "operands — update {NOTE}");
    assert_eq!(measured, PINNED_SIGILS, "per-sigil counts — update {NOTE}");
    assert_eq!(fused, PINNED_FUSED, "fused sigils — update {NOTE}");
    assert_eq!(fraction(fused, c.operands), PINNED_FUSED_RATE);
    assert_eq!(
        unwritten, PINNED_DECLARED_UNWRITTEN,
        "declared and written by no program — listed above; update {NOTE} and doc 02 O-02-2"
    );
}

/// REFUSED: a sigil the grammar declares and no program writes is listed by
/// production and sigil, not folded into a count of zero.
#[test]
fn a_declared_sigil_no_program_writes_is_listed_by_name() {
    let grammar = "\nkaraka         = destination        (* कर्म *)\n               | source\n               | vocative ;\n\ndestination    = \"म्\" ;\nsource         = \"न\" ;\nvocative       = \"ओ\" ;\n";
    let declared = declared_sigils(grammar);
    assert_eq!(
        declared,
        [
            ("destination".to_string(), "म्".to_string()),
            ("source".to_string(), "न".to_string()),
            ("vocative".to_string(), "ओ".to_string()),
        ]
    );
    let written: BTreeSet<&str> = census().sigils.keys().copied().collect();
    assert_eq!(
        declared_unwritten(&declared, &written),
        ["vocative = \"ओ\""]
    );
    // The real grammar declares one no program writes, and it is listed too.
    let real = std::fs::read_to_string(root().join("spec/grammar-t0.ebnf")).expect("grammar");
    assert_eq!(
        declared_unwritten(&declared_sigils(&real), &written),
        PINNED_DECLARED_UNWRITTEN
    );
}
