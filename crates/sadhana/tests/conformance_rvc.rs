//! Conformance at the compressed target — task `B-058b2b4`, doc 03 §4.4.
//!
//! `tests/conformance.rs` checks the assembler against `riscv64-elf-as` with
//! `.option norvc`. This checks **the same programs** against the same assembler
//! with compression **on**, which is what `-march=rv64gc` does by default and
//! therefore what the march this project targets actually means.
//!
//! "The same programs" is asserted rather than asserted-about: [`base_corpus_len`]
//! reads the other table's row count, because the two files drifted 49 apart
//! while a literal `5949` here agreed with the smaller one.
//!
//! 75 of them differ between the two files — the corpus uses registers from
//! across the whole file and immediates near the ends of their ranges, and most
//! of those cannot be named by a compressed form. Those 75 are the whole of
//! what compression buys here, and the whole of what selecting it can get
//! wrong. We match 37 of them and stay wide on 38, and never choose a
//! different instruction.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// `(sassembly, expected encoding, riscv)` with compression enabled.
fn corpus() -> Vec<(String, u32, bool, String)> {
    let text = std::fs::read_to_string(root().join("spec/conformance-rvc-t0.tsv"))
        .expect("read spec/conformance-rvc-t0.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("sassembly\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let hex = f.get(1)?.trim().trim_start_matches("0x");
            let word = u32::from_str_radix(hex, 16).ok()?;
            Some((
                f.first()?.to_string(),
                word,
                hex.len() == 4,
                f.get(2)?.to_string(),
            ))
        })
        .collect()
}

/// How many rows `spec/conformance-t0.tsv` has, counted with the same filter.
///
/// **THIS FILE'S HEADER CLAIMS THE TWO TABLES HOLD "THE SAME PROGRAMS", AND
/// NOTHING CHECKED IT.** The claim was pinned instead as a literal `5949`, which
/// a stale table satisfies perfectly: `conformance-t0.tsv` grew by 49 rows
/// (`ecall`, `sret`, the `sfence.vma` family) and `gen-conformance-rvc.py` was
/// never re-run, so the compressed corpus sat 49 programs short and the number
/// agreed with itself the whole time. Rediscovered while regenerating both
/// tables for ADR-0042. A count taken from the other file cannot express that
/// drift and a literal cannot avoid it.
fn base_corpus_len() -> usize {
    let text = std::fs::read_to_string(root().join("spec/conformance-t0.tsv"))
        .expect("read spec/conformance-t0.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("sassembly\t") && !l.trim().is_empty())
        .filter(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            f.get(1)
                .and_then(|h| u32::from_str_radix(h.trim().trim_start_matches("0x"), 16).ok())
                .is_some()
        })
        .count()
}

/// What this assembler would emit for one statement at the compressed target.
fn ours(src: &str) -> Option<(u32, bool)> {
    let p = sadhana::parse::assemble_program(&format!("{src}\n")).ok()?;
    let inst = p.instructions.first()?;
    let symbols = BTreeMap::new();
    if let Some(half) = sadhana::encode::compressed_at(inst, 0, &symbols) {
        return Some((u32::from(half), true));
    }
    sadhana::encode::encode_at(inst, 0, &symbols)
        .ok()
        .map(|w| (w, false))
}

#[test]
fn the_compressed_choice_is_never_the_wrong_instruction() {
    // The invariant that must hold before selection can be turned on. Where we
    // choose a compressed form it has to be the assembler's; where we stay wide
    // the wide form is correct by construction, and costs only bytes.
    //
    // A wrong compressed choice is not a smaller program — it is a different
    // one, and `B-058b2b3` found four ways to make one before this passed.
    let corpus = corpus();
    assert_eq!(
        corpus.len(),
        base_corpus_len(),
        "the compressed corpus and spec/conformance-t0.tsv are not the same programs — \
         re-run tools/gen-conformance.py and tools/gen-conformance-rvc.py"
    );
    assert!(
        corpus.len() >= 5949,
        "the corpus shrank to {} from 5949; a suite that stops measuring is not a suite that \
         passes",
        corpus.len()
    );

    let (mut agreed, mut conservative, mut unencodable) = (0, 0, 0);
    let mut wrong = Vec::new();
    for (src, want, want_compressed, riscv) in &corpus {
        let Some((got, got_compressed)) = ours(src) else {
            unencodable += 1;
            continue;
        };
        if got_compressed && got != *want {
            wrong.push(format!("{riscv}: ours {got:#06x}, as {want:#06x}"));
        } else if got_compressed == *want_compressed && got == *want {
            agreed += 1;
        } else if !got_compressed && *want_compressed {
            conservative += 1;
        } else if got != *want {
            // Both wide and disagreeing is a plain conformance failure and
            // belongs to `tests/conformance.rs`, not here.
            wrong.push(format!("{riscv}: wide, ours {got:#010x}, as {want:#010x}"));
        }
    }

    assert!(
        wrong.is_empty(),
        "{} of {} are the wrong instruction:\n  {}",
        wrong.len(),
        corpus.len(),
        wrong
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    // Pinned so the gap can only close. Every one of these is an instruction
    // the assembler compresses and we do not — correct, and larger.
    assert!(
        conservative <= 38,
        "{conservative} left wide, up from 38 — selection got worse"
    );
    println!(
        "METRIC rvc_agreement {agreed} agreed, {conservative} wide, {unencodable} unencodable"
    );
}

// ---------------------------------------------------------------------------
// `॥ संरेखः n ॥` at the compressed target — task `W-079`, test 2.
//
// KILLS: alignment computed as if every instruction were four bytes.
//
// `spec/conformance-rvc-t0.tsv` contains no directive, so nothing here ever
// aligned anything. That matters more at this target than at the other one:
// a compressed instruction is TWO bytes, so how far the section sits from a
// boundary is not a function of the instruction COUNT, and an implementation
// that assumed it was would still satisfy every uncompressed test.
//
// This is also the reason alignment had to join the relaxation fixpoint rather
// than be resolved while parsing — the distance is not known until the widths
// have settled.
// ---------------------------------------------------------------------------

#[test]
fn alignment_at_the_compressed_target_counts_bytes_not_instructions() {
    let src = "॥ कोष्ठकम् ॱपाठ ॥\nमुख्यम्ॱॱ\n    योगः  अर्थ०म्  शून्यःन  ०१०न ।\n\
               ॥ संरेखः १६ ॥\nलक्ष्यम्ॱॱ\n    योगः  अर्थ१म्  शून्यःन  ०२०न ।\n";
    let p = sadhana::parse::assemble_program(src).expect("assembles");

    let wide = sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Uncompressed);
    let tight = sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Compressed);

    let idx = p
        .labels
        .iter()
        .find(|l| l.name == "लक्ष्यम्")
        .expect("लक्ष्यम्")
        .at;

    // The boundary holds at BOTH targets — that is what alignment means.
    assert_eq!(wide[idx] % 16, 0, "uncompressed: {:#x}", wide[idx]);
    assert_eq!(tight[idx] % 16, 0, "compressed: {:#x}", tight[idx]);

    // And the emitted bytes reach it at both, which the address alone does not
    // prove: a layout that agreed with itself and emitted nothing would pass
    // the two assertions above.
    for (target, name) in [
        (sadhana::encode::Target::Uncompressed, "uncompressed"),
        (sadhana::encode::Target::Compressed, "compressed"),
    ] {
        let text = sadhana::encode::encode_program_for(&p, target).expect("encodes");
        assert!(
            text.len() >= 16,
            "{name}: only {} bytes emitted, the boundary at 16 was never reached",
            text.len()
        );
        // Nothing in the padding may be an all-zero word: that traps.
        let words = sadhana::encode::words(&text);
        assert!(!words.contains(&0), "{name}: an all-zero word reached ॱपाठ");
    }
}
