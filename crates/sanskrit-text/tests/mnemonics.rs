//! `spec/mnemonics-riscv64.src.tsv` must hold up — task B-001.
//!
//! The mnemonic registry is the ISA surface: rule R-02-1 says there is no
//! `.insn 0x…` escape hatch, so an instruction absent from that file cannot be
//! written at all. That makes the file load-bearing in a way a comment cannot
//! enforce, and these tests are what enforce it.
//!
//! Four properties, each guarding a different way the registry can rot:
//!
//! 1. **Every name survives orthographic closure.** A mnemonic outside the
//!    doc 15 repertoire could not be typed in Sassembly source, so the name
//!    would be unusable while looking fine in a table.
//! 2. **No two families share a name.** A duplicate makes assembly ambiguous
//!    and would be found by the assembler, in B-012, long after the naming
//!    decision was cheap to change.
//! 3. **Every mnemonic resolves through the lexicon.** Doc 01 §1.2.4 has
//!    diagnostics resolve their terms there, so a mnemonic the lexicon has
//!    never heard of is one the assembler cannot name in an error message.
//! 4. **`encoding` is empty.** B-008 fills it from the pinned PDF. A
//!    half-remembered opcode that looks authoritative is worse than a blank,
//!    and this asserts the blank is still there.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

struct Row {
    family: String,
    devanagari: String,
    gloss: String,
    ext: String,
    covers: Vec<String>,
    encoding: String,
    source: String,
    line: usize,
}

/// Split the `covers` column, which is comma-separated EXCEPT INSIDE
/// PARENTHESES.
///
/// One row reads `(pseudo: xori rd,rs,-1)` — a documented pseudo-instruction,
/// not an encoding, and every consumer skips entries that open with `(`. A
/// plain `split(',')` cuts that cell into three, of which only the first opens
/// with `(`; the other two, `rs` and `-1)`, slip past the skip and are counted
/// as architectural instructions. That is where `rv64gc_instructions_named`
/// read 197 for a registry covering 195: the surplus was punctuation.
///
/// Depth is tracked instead, so a parenthesised entry survives whole and is
/// skipped whole.
fn split_covers(field: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in field.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(ch),
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn rows() -> Vec<Row> {
    let text = std::fs::read_to_string(root().join("spec/mnemonics-riscv64.src.tsv"))
        .expect("read spec/mnemonics-riscv64.src.tsv");
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|(_, l)| !l.starts_with("family\t"))
        .map(|(n, l)| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(
                f.len() >= 7,
                "line {}: expected 7 tab-separated columns, got {} — {l:?}",
                n + 1,
                f.len()
            );
            Row {
                family: f[0].into(),
                devanagari: f[1].into(),
                gloss: f[2].into(),
                ext: f[3].into(),
                covers: split_covers(f[4]),
                encoding: f[5].into(),
                source: f[6].into(),
                line: n + 1,
            }
        })
        .collect()
}

#[test]
fn every_mnemonic_survives_orthographic_closure() {
    // If a name cannot appear in Sassembly source, it is not a usable name —
    // it just looks like one in a table.
    for r in rows() {
        let verdict = sanskrit_text::repertoire::check(&r.devanagari);
        assert!(
            verdict.is_empty(),
            "line {}: mnemonic {} ({}) is outside the doc 15 repertoire: {:?}",
            r.line,
            r.devanagari,
            r.family,
            verdict
        );
    }
}

#[test]
fn mnemonics_are_nfc_normalised() {
    // Two byte sequences that render identically would be two different names
    // to the assembler — exactly the confusable class doc 15 exists to remove.
    for r in rows() {
        assert!(
            sanskrit_text::is_nfc(&r.devanagari),
            "line {}: {} is not NFC",
            r.line,
            r.family
        );
    }
}

#[test]
fn no_two_families_share_a_name() {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for r in rows() {
        if let Some(prev) = seen.insert(r.devanagari.clone(), r.family.clone()) {
            panic!(
                "line {}: {} is used by both `{}` and `{}` — assembly would be ambiguous",
                r.line, r.devanagari, prev, r.family
            );
        }
    }
}

#[test]
fn no_two_families_claim_the_same_instruction() {
    // A RISC-V instruction covered twice means the assembler has two candidate
    // encodings and no rule to choose between them.
    let mut owner: BTreeMap<String, String> = BTreeMap::new();
    for r in rows() {
        for insn in &r.covers {
            if insn.starts_with('(') {
                continue; // a documented pseudo-instruction, not an encoding
            }
            if let Some(prev) = owner.insert(insn.clone(), r.family.clone()) {
                panic!(
                    "line {}: `{insn}` is claimed by both `{prev}` and `{}`",
                    r.line, r.family
                );
            }
        }
    }
}

#[test]
fn rows_claiming_the_lexicon_are_telling_the_truth() {
    // Doc 01 §4 rule 5: one word, one sense. A row that says `lexicon` while
    // the lexicon has never heard of the term is how a second vocabulary starts.
    let lex = std::fs::read_to_string(root().join("spec/lexicon.tsv")).expect("read lexicon.tsv");
    let known: Vec<&str> = lex
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next())
        .collect();

    let mut missing = Vec::new();
    for r in rows() {
        if r.source == "lexicon" && !known.contains(&r.devanagari.as_str()) {
            missing.push(format!("line {}: {} ({})", r.line, r.devanagari, r.family));
        }
    }
    assert!(
        missing.is_empty(),
        "rows claim `lexicon` but the term is not in spec/lexicon.tsv:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn every_mnemonic_is_in_the_lexicon() {
    // Before B-032 this asserted the opposite for coined rows: that a term
    // marked `coined` was NOT yet in the lexicon. B-032 put all 27 there, so
    // the invariant inverts and gets stronger — EVERY mnemonic, however it was
    // named, must now resolve through spec/lexicon.tsv. Doc 01 §1.2.4 has
    // diagnostics resolve their terms through that file, so a mnemonic missing
    // from it is one the assembler cannot name in an error message.
    //
    // `source` keeps its meaning as provenance: `coined` records that this
    // registry introduced the term, `lexicon` that it predated the registry.
    // That distinction is worth keeping — it is the difference between a name
    // this project chose and one it inherited.
    let lex = std::fs::read_to_string(root().join("spec/lexicon.tsv")).expect("read lexicon.tsv");
    let known: Vec<&str> = lex
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next())
        .collect();

    let mut missing = Vec::new();
    for r in rows() {
        assert!(
            r.source == "coined" || r.source == "lexicon",
            "line {}: source must be `lexicon` or `coined`, got {:?}",
            r.line,
            r.source
        );
        if !known.contains(&r.devanagari.as_str()) {
            missing.push(format!(
                "line {}: {} ({}, {})",
                r.line, r.devanagari, r.family, r.source
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "mnemonics absent from spec/lexicon.tsv — run `cargo run -p lexgen` after \
         adding them to lexicon.src.tsv:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn every_family_carries_an_english_gloss() {
    // The gloss is what makes the registry reviewable by someone who does not
    // read Devanagari, and what a diagnostic falls back to. A blank one turns a
    // naming decision into an unreviewable assertion.
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for r in rows() {
        assert!(
            !r.gloss.trim().is_empty(),
            "line {}: family `{}` has no gloss",
            r.line,
            r.family
        );
        if let Some(prev) = seen.insert(r.gloss.clone(), r.family.clone()) {
            panic!(
                "line {}: gloss {:?} is shared by `{prev}` and `{}` — two families                  meaning the same thing in English is either a duplicate or a gloss                  that is too vague to review",
                r.line, r.gloss, r.family
            );
        }
    }
}

#[test]
fn the_family_encoding_column_stays_empty() {
    // B-008 ran, and deliberately did NOT fill this column: a family covers
    // many encodings — `योगः` alone covers nine — so one cell cannot hold them.
    // The encodings live per-instruction in spec/encodings-riscv64.tsv, derived
    // from the assembler rather than written down. Filling this column would
    // mean picking one encoding to stand for all of them, which is the kind of
    // half-true datum that reads as authoritative.
    for r in rows() {
        assert!(
            r.encoding.trim().is_empty(),
            "line {}: `{}` has an encoding — encodings belong in \
             spec/encodings-riscv64.tsv, one per instruction, derived by \
             tools/gen-encodings.py rather than written here",
            r.line,
            r.family
        );
    }
}

#[test]
fn the_registry_covers_rv64gc_and_r_02_2_actually_pays() {
    let rows = rows();
    let insns: usize = rows
        .iter()
        .flat_map(|r| r.covers.iter())
        .filter(|i| !i.starts_with('('))
        .count();

    // Every RV64GC extension must be represented, or the ISA surface has a hole
    // that rule R-02-1 says cannot be filled by an escape hatch.
    for ext in ["I", "M", "A", "FD", "Zicsr", "Zifencei"] {
        assert!(
            rows.iter().any(|r| r.ext == ext),
            "no family covers extension {ext}"
        );
    }

    // R-02-2's whole claim is that per-family naming keeps the name count far
    // below the encoding count. If that stopped being true the rule would be
    // costing complexity and buying nothing.
    assert!(
        rows.len() * 2 < insns,
        "R-02-2 is not paying: {} families for {insns} instructions",
        rows.len()
    );
    println!("METRIC rv64gc_families {}", rows.len());
    println!("METRIC rv64gc_instructions_named {insns}");
}

// ---- the DENOMINATOR: what the ISA manual itself lists ---------------------
//
// `rv64gc_instructions_named` counts the registry's own `covers` column. It is
// a numerator and it cannot be anything else: nothing in
// `spec/mnemonics-riscv64.src.tsv` knows how many instructions RV64GC HAS, so
// a coverage figure built out of that file alone divides a number by itself
// and reports 100% by construction — including on the day the ISA surface
// grows a hole.
//
// So `spec/rv64gc-manual-listing.tsv` is read out of a document this project
// did not write: chapter 34 of the pinned unprivileged manual
// (`RV32/64G Instruction Set Listings`) and section 26.8 (`RVC Instruction Set
// Listings`), by `tools/gen-rv64gc-manual.py`. Each row carries the manual's
// own section heading and the opcode bits printed beside the mnemonic —
// columns that exist nowhere in the registry, so a denominator quietly copied
// from the numerator cannot wear this file's shape, and the test below says so
// by name.

/// A row of `spec/rv64gc-manual-listing.tsv`.
struct ManualRow {
    mnemonic: String,
    section: String,
    opcode: String,
    /// The manual's own parenthetical beside the mnemonic, verbatim —
    /// `(RV64/128; RES, rd=0)`, `(RV32)`, `(HINT, rd=0)`, or empty.
    xlen: String,
    line: usize,
}

/// This test's own loader. It is deliberately not `rows()`: that one reads the
/// registry, and the whole point of this file is that the two sides come from
/// different documents.
fn manual_rows(text: &str) -> Vec<ManualRow> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|(_, l)| !l.starts_with("mnemonic\t"))
        .map(|(n, l)| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(
                f.len() >= 3,
                "line {}: expected 4 tab-separated columns, got {} — {l:?}",
                n + 1,
                f.len()
            );
            ManualRow {
                mnemonic: f[0].into(),
                section: f[1].into(),
                opcode: f[2].into(),
                xlen: f.get(3).unwrap_or(&"").trim().into(),
                line: n + 1,
            }
        })
        .collect()
}

fn manual_text() -> String {
    std::fs::read_to_string(root().join("spec/rv64gc-manual-listing.tsv"))
        .expect("read spec/rv64gc-manual-listing.tsv")
}

/// The manual sections that make up RV64GC — I, M, A, F, D, Zicsr, Zifencei
/// (the manual's own definition of G, chapter 34's opening paragraph) plus the
/// three compressed quadrants.
///
/// The `RV32x` sections are NOT RV32-only: chapter 34 lists each extension's
/// shared instructions once under `RV32x` and only the ADDITIONS under
/// `RV64x` ("in addition to RV32M"). So both halves are in, and the union is
/// taken by mnemonic because `slli`, `srli` and `srai` are listed twice — once
/// with a five-bit shamt and once with six.
///
/// What is left out is left out because the manual puts it outside G: `RV32Q`
/// and `RV64Q` (quad-precision float), `RV32Zfh` and `RV64Zfh` (half), and
/// `Zawrs`. The listing file carries those rows too — the extraction does not
/// get to decide — and this list is where the decision lives, in code, with
/// the test below.
const RV64GC_SECTIONS: &[&str] = &[
    "34/RV32I",
    "34/RV64I",
    "34/Zifencei",
    "34/Zicsr",
    "34/RV32M",
    "34/RV64M",
    "34/RV32A",
    "34/RV64A",
    "34/RV32F",
    "34/RV64F",
    "34/RV32D",
    "34/RV64D",
    "26.8/Q0",
    "26.8/Q1",
    "26.8/Q2",
];

/// Does the manual's parenthetical say this compressed instruction exists on
/// RV64?
///
/// Section 26.8's figures annotate the RVC mnemonics in place, and the FIRST
/// clause is the one that scopes the instruction: `C.LD(RV64/128)` exists on
/// RV64 and RV128, `C.JAL(RV32)` does not exist on RV64 at all, `C.SRLI64
/// (RV128; RV32/64 HINT)` is an RV128 instruction that RV64 decodes as a hint.
///
/// A clause that is not a bare XLEN set scopes nothing —
/// `C.SLLI(HINT, rd=0; RV32 Custom, uimm[5]=1)` is an ordinary RV64
/// instruction whose rd=0 encoding is a hint — so the answer is yes, as it is
/// for a row with no parenthetical at all. Reading `HINT` as "not on RV64"
/// would delete eight real instructions from the denominator and flatter the
/// coverage figure by three points.
fn exists_on_rv64(xlen: &str) -> bool {
    let Some(inner) = xlen.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
        return true;
    };
    let first = inner.split(';').next().unwrap_or("").trim();
    let Some(rest) = first.strip_prefix("RV") else {
        return true;
    };
    let mut parts = rest.split('/');
    // `RV32/64` is 32 and 64; `RV64/128` is 64 and 128. Anything that is not
    // digits is not an XLEN set, and then the clause scopes nothing.
    if !rest.chars().all(|c| c.is_ascii_digit() || c == '/') || rest.is_empty() {
        return true;
    }
    parts.any(|p| p == "64")
}

/// The RV64GC instruction set as the manual lists it.
fn rv64gc_from_manual(text: &str) -> BTreeSet<String> {
    manual_rows(text)
        .into_iter()
        .filter(|r| RV64GC_SECTIONS.contains(&r.section.as_str()) && exists_on_rv64(&r.xlen))
        .map(|r| r.mnemonic)
        .collect()
}

/// Every architectural instruction the registry covers, pseudo-instructions
/// excluded.
fn registry_covers() -> BTreeSet<String> {
    rows()
        .iter()
        .flat_map(|r| r.covers.iter())
        .filter(|i| !i.starts_with('('))
        .cloned()
        .collect()
}

struct Coverage {
    total: usize,
    named: usize,
    unnamed: Vec<String>,
}

/// `None` when the listing names no RV64GC instruction AT ALL.
///
/// This is the refusal, and it is the same one `W-309` wrote for `nucleus_loc`
/// one rung down. An empty denominator makes the coverage 0/0, and every
/// natural way to render that — `100%`, or the `rv64gc_mnemonics_named 0` this
/// metric was a hand-typed zero for until today — is a sentence about a
/// perfect or an empty ISA surface emitted at the exact moment the instrument
/// stopped reading the manual. So it emits nothing and the dashboard shows the
/// hole.
fn coverage(manual_text: &str, covered: &BTreeSet<String>) -> Option<Coverage> {
    let manual = rv64gc_from_manual(manual_text);
    if manual.is_empty() {
        return None;
    }
    let unnamed: Vec<String> = manual.difference(covered).cloned().collect();
    Some(Coverage {
        total: manual.len(),
        named: manual.len() - unnamed.len(),
        unnamed,
    })
}

/// The RV64GC instructions with no Sassembly name, dated 2026-09-18.
///
/// EXACT, not an allowlist, in the shape `W-308` gave the unproduced targets:
/// a third unnamed instruction reds, and so does naming one of these two
/// without striking its row. Both are memory-ordering forms the registry's
/// `fence` family did not pick up — `fence.tso` is a distinct funct4 encoding
/// and `pause` is the `fence w,0` HINT — and chapter 34 lists both in the
/// RV32I base table, so R-02-1's "every architectural instruction has a
/// Sassembly name" is two short and has been for as long as the registry has
/// existed. Naming them is a lexicon decision, not a measurement, so this
/// cycle records the hole rather than closing it.
const RV64GC_WITHOUT_A_NAME: &[&str] = &["fence.tso", "pause"];

#[test]
fn the_covers_column_keeps_a_parenthesised_entry_whole() {
    // The bug this replaced: `split(',')` cut `(pseudo: xori rd,rs,-1)` into
    // three, and only the first opened with `(`, so `rs` and `-1)` were
    // counted as architectural instructions by every consumer that skips
    // pseudo-entries by that test.
    assert_eq!(
        split_covers("add,addi,(pseudo: xori rd,rs,-1),addw"),
        vec!["add", "addi", "(pseudo: xori rd,rs,-1)", "addw"]
    );
    // And the ordinary case is unchanged, blanks and spacing included.
    assert_eq!(split_covers("sub, subw ,,"), vec!["sub", "subw"]);
    assert_eq!(split_covers(""), Vec::<String>::new());

    // On the real file: exactly one entry is parenthesised, and it is whole.
    let pseudo: Vec<String> = rows()
        .iter()
        .flat_map(|r| r.covers.iter())
        .filter(|i| i.starts_with('('))
        .cloned()
        .collect();
    assert_eq!(pseudo, vec!["(pseudo: xori rd,rs,-1)"]);
    // Nothing that is not an instruction survives the skip. Every counted
    // entry is a mnemonic: lowercase letters, digits and dots.
    for insn in registry_covers() {
        assert!(
            insn.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.'),
            "`{insn}` is counted as an architectural instruction and is not a mnemonic"
        );
    }
}

#[test]
fn the_listing_is_the_manuals_and_could_not_be_the_registrys() {
    let text = manual_text();
    let rows = manual_rows(&text);
    assert!(
        rows.len() > 200,
        "only {} rows — the extraction shrank",
        rows.len()
    );

    // Every row carries the manual's own provenance. The registry has neither
    // column, so a denominator copied from it cannot arrive in this shape.
    for r in &rows {
        assert!(
            r.section.starts_with("34/") || r.section.starts_with("26.8/"),
            "line {}: `{}` cites section {:?}, which is not chapter 34 or section 26.8",
            r.line,
            r.mnemonic,
            r.section
        );
        assert!(
            !r.opcode.is_empty() && r.opcode.chars().all(|c| c == '0' || c == '1'),
            "line {}: `{}` has opcode {:?}, which is not the manual's bits",
            r.line,
            r.mnemonic,
            r.opcode
        );
    }

    // Every section this file's verdict names must actually be present, or the
    // denominator is short and nobody would know which part is missing.
    let present: BTreeSet<&str> = rows.iter().map(|r| r.section.as_str()).collect();
    for s in RV64GC_SECTIONS {
        assert!(present.contains(s), "the listing carries no {s} rows");
    }

    // And the sections the verdict EXCLUDES must be present too. They are the
    // proof the extraction read the chapter rather than the registry: nothing
    // in Sassembly names a quad-precision float or a Zawrs instruction, so a
    // file built from `covers` would contain none of these rows.
    for s in [
        "34/RV32Q",
        "34/RV64Q",
        "34/RV32Zfh",
        "34/RV64Zfh",
        "34/Zawrs",
    ] {
        assert!(present.contains(s), "the listing carries no {s} rows");
    }
    let covered = registry_covers();
    let outside: Vec<&ManualRow> = rows
        .iter()
        .filter(|r| !RV64GC_SECTIONS.contains(&r.section.as_str()))
        .collect();
    assert!(
        outside.iter().all(|r| !covered.contains(&r.mnemonic)),
        "an out-of-G instruction is in the registry — the two files may share a source"
    );
    assert!(outside.len() > 50, "only {} out-of-G rows", outside.len());
}

#[test]
fn rv64_membership_reads_the_manuals_own_scope_note() {
    // The clause that scopes, and the clauses that do not.
    assert!(exists_on_rv64(""));
    assert!(exists_on_rv64("(RV64/128; RES, rd=0)"));
    assert!(exists_on_rv64("(RV32/64)"));
    assert!(exists_on_rv64("(HINT, rd=0; RV32 Custom, uimm[5]=1)"));
    assert!(exists_on_rv64("(RES, imm=0)"));
    assert!(exists_on_rv64("RES, uimm=0"));
    assert!(!exists_on_rv64("(RV32)"));
    assert!(!exists_on_rv64("(RV128)"));
    assert!(!exists_on_rv64("(RV128; RV32/64 HINT)"));
    // `RV32 Custom` is prose, not an XLEN set — reading its `RV32` as a scope
    // would drop `c.srli` and `c.srai`, which RV64 certainly has.
    assert!(exists_on_rv64("(RV32 Custom, uimm[5]=1)"));

    // On the real file, by name, both directions.
    let set = rv64gc_from_manual(&manual_text());
    for insn in [
        "c.addiw",
        "c.ldsp",
        "c.sdsp",
        "c.fld",
        "c.srli",
        "c.slli",
        "ld",
        "amomaxu.d",
    ] {
        assert!(
            set.contains(insn),
            "`{insn}` is RV64GC and the verdict dropped it"
        );
    }
    for insn in [
        "c.jal", "c.flw", "c.fswsp", "c.lq", "c.sqsp", "c.slli64", "fmadd.q", "fadd.h",
    ] {
        assert!(
            !set.contains(insn),
            "`{insn}` is not RV64GC and the verdict kept it"
        );
    }
}

#[test]
fn an_empty_listing_names_no_coverage_rather_than_a_perfect_one() {
    let covered = registry_covers();

    // A listing whose every row is outside G: the denominator is zero, and
    // 0 of 0 is not 100%.
    let handed = "mnemonic\tsection\topcode\txlen\n\
                  fadd.q\t34/RV32Q\t1010011\t\n\
                  wrs.nto\t34/Zawrs\t1110011\t\n";
    assert!(coverage(handed, &covered).is_none());
    assert!(coverage("mnemonic\tsection\topcode\txlen\n", &covered).is_none());

    // And a listing that does reach G is measured, holes and all.
    let handed = "mnemonic\tsection\topcode\txlen\n\
                  add\t34/RV32I\t0110011\t\n\
                  fence.tso\t34/RV32I\t0001111\t\n\
                  c.jal\t26.8/Q1\t01\t(RV32)\n";
    let c = coverage(handed, &covered).expect("two in-G rows are a denominator");
    assert_eq!(
        c.total, 2,
        "c.jal is RV32-only and is not in the denominator"
    );
    assert_eq!(c.named, 1);
    assert_eq!(c.unnamed, vec!["fence.tso"]);
}

#[test]
fn every_rv64gc_instruction_the_manual_lists_has_a_name_or_is_named_as_a_hole() {
    let c = coverage(&manual_text(), &registry_covers())
        .expect("the pinned manual lists RV64GC instructions");
    assert_eq!(
        c.unnamed, RV64GC_WITHOUT_A_NAME,
        "the RV64GC instructions with no Sassembly name have changed — \
         R-02-1 says every architectural instruction has one, so a new entry \
         here is a hole in the ISA surface and a departed one should be struck \
         from RV64GC_WITHOUT_A_NAME"
    );

    // Truncated, never rounded: 194 of 195 must not be allowed to print the
    // one figure that means "done". Only 195 of 195 reads 100%.
    let percent = c.named * 100 / c.total;
    println!("METRIC rv64gc_mnemonics_named {percent}%");
    println!("METRIC rv64gc_manual_instructions {}", c.total);
    println!("METRIC rv64gc_manual_unnamed {}", c.unnamed.len());
}
