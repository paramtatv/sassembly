//! Which encodings any corpus actually produces — task `B-072`.
//!
//! The corpora say "these bytes are right". This says which instructions they
//! have an opinion about at all.
//!
//! # Why it exists
//!
//! `B-054` found that no operandless instruction had ever been assembled by a
//! test — `ebreak` and `fence.i` were nameable, encodable and completely
//! unexercised. The cause was structural rather than an oversight: every loop
//! in `tools/gen-conformance.py` iterates over registers, so an instruction
//! with none to iterate produces no cases.
//!
//! That is a shape of hole no amount of adding cases finds, because the thing
//! missing is a whole category nobody thought to enumerate. So the question is
//! asked directly: of the encodings we can emit, which does no corpus contain?
//!
//! The first run answered **111 of 156**. It is now **zero**: `B-073` took it
//! to 58, `B-060` to 26, and `B-009` closed the last eight — the float loads
//! and stores and lr/sc, every one of them named, derived, and asked for by no
//! corpus in the repository.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// Every RISC-V mnemonic any corpus asks for.
fn exercised() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut first_word = |s: &str| {
        if let Some(w) = s.split_whitespace().next()
            && !w.ends_with(':')
        {
            out.insert(w.to_string());
        }
    };

    let conformance =
        std::fs::read_to_string(root().join("spec/conformance-t0.tsv")).expect("read");
    for l in conformance.lines().filter(|l| !l.starts_with('#')) {
        if let Some(rv) = l.split('\t').nth(2) {
            first_word(rv);
        }
    }

    let differential =
        std::fs::read_to_string(root().join("spec/differential-t0.tsv")).expect("read");
    for l in differential.lines().filter(|l| !l.starts_with('#')) {
        if let Some(rv) = l.split('\t').nth(2) {
            for part in rv.split("\\n") {
                first_word(part);
            }
        }
    }

    for entry in std::fs::read_dir(root().join("spec/golden")).expect("read golden") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "sas") {
            let text = std::fs::read_to_string(&path).expect("read");
            for l in text.lines() {
                if let Some(rest) = l.strip_prefix("॰ as:") {
                    first_word(rest);
                }
            }
        }
    }
    out
}

/// Every 32-bit encoding, with the family it belongs to.
fn encodings() -> Vec<(String, String)> {
    std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv"))
        .expect("read")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("insn\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 6 && f[5] == "32").then(|| (f[0].to_string(), f[1].to_string()))
        })
        .collect()
}

#[test]
fn the_uncovered_encodings_are_pinned() {
    let seen = exercised();
    let all = encodings();
    let missing: Vec<&(String, String)> = all.iter().filter(|(i, _)| !seen.contains(i)).collect();
    let covered = all.len() - missing.len();

    println!("METRIC encodings_exercised {covered}");
    println!("METRIC encodings_total {}", all.len());

    // **Zero, and it stays zero** (`B-009`). This began at 111 of 156, was 58
    // after `B-073` and 26 after `B-060`; the last eight were the float loads
    // and stores and lr/sc, all four families named and derived and asked for
    // by nothing.
    //
    // A ceiling was right while the gap was closing, because it let the number
    // fall without a code change. An equality is right now: every 32-bit
    // encoding in RV64GC is produced by some corpus, and an encoding that stops
    // being exercised is a regression rather than a smaller improvement.
    assert!(
        missing.is_empty(),
        "{} encoding(s) exercised by nothing: {:?}",
        missing.len(),
        missing.iter().map(|(i, _)| i).collect::<Vec<_>>()
    );
}
