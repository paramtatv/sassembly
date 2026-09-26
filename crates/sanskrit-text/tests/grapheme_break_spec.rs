//! **`spec/grapheme-break.tsv` and `src/tables.rs` are two readers of ONE
//! pinned UCD, and they must agree row for row.**
//!
//! # Why the file exists at all
//!
//! The UAX #29 grapheme-break classes lived only in `src/tables.rs`, generated
//! into RUST by `ucdgen`. ADR-0019's embed resolves a name against a
//! `spec/*.tsv`, so **no T1 port could read them** — `अक्षरकोश ॱ अक्षराणि` was
//! blocked on the absence of a FILE, not on anything about the language. It is
//! the same shape as the five routines blocked by a missing name in
//! `anita.rs`'s `TABLES`, one step further out: there the file existed and was
//! unnamed; here it did not exist.
//!
//! # Why this test and not a glance at the generator
//!
//! Two generators reading one source is exactly the arrangement that lets them
//! drift, and this repository keeps finding defects in the gap between two
//! readers of one thing — an encoder and a decoder disagreeing, a lexer and a
//! parser disagreeing, `Interpreter::routine` splitting a member mark where
//! `resolve_call` did not. So the agreement is ASSERTED rather than assumed.
//!
//! It caught a real difference the first time it ran. The UCD lists
//! `0483..0487` (general category `Mn`) and `0488..0489` (`Me`) as separate
//! rows because their categories differ, though both are `Extend`; `tables.rs`
//! merges adjacent ranges of one class and the first draft of the generator did
//! not. 1429 rows against 1386. The generator merges now, and this test is what
//! said so.

use sanskrit_text::tables::{GRAPHEME_BREAK, GraphemeBreak};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The UAX class name as the TSV spells it, for each Rust variant.
fn uax_name(c: GraphemeBreak) -> &'static str {
    match c {
        GraphemeBreak::Other => "Other",
        GraphemeBreak::Cr => "CR",
        GraphemeBreak::Lf => "LF",
        GraphemeBreak::Control => "Control",
        GraphemeBreak::Extend => "Extend",
        GraphemeBreak::Zwj => "ZWJ",
        GraphemeBreak::RegionalIndicator => "Regional_Indicator",
        GraphemeBreak::Prepend => "Prepend",
        GraphemeBreak::SpacingMark => "SpacingMark",
        GraphemeBreak::L => "L",
        GraphemeBreak::V => "V",
        GraphemeBreak::T => "T",
        GraphemeBreak::Lv => "LV",
        GraphemeBreak::Lvt => "LVT",
    }
}

/// Every `(start, end, class)` of the spec table, read here with Rust's own
/// `lines`/`split` — never through the generator that wrote it.
fn spec_rows() -> Vec<(u32, u32, String)> {
    let p = repo_root().join("spec/grapheme-break.tsv");
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!(
            "{} must exist — tools/gen-grapheme-break.py writes it: {e}",
            p.display()
        )
    });
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| !l.starts_with("devanagari\t"))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(
                f.len() >= 4,
                "a row of grapheme-break.tsv has too few fields: {l:?}"
            );
            (
                u32::from_str_radix(f[2], 16).expect("start is hex"),
                u32::from_str_radix(f[3], 16).expect("end is hex"),
                f[1].to_string(),
            )
        })
        .collect()
}

#[test]
fn the_grapheme_break_table_agrees_with_the_generated_rust_one() {
    let spec = spec_rows();
    let rust: Vec<(u32, u32, String)> = GRAPHEME_BREAK
        .iter()
        .map(|(a, b, c)| (*a, *b, uax_name(*c).to_string()))
        .collect();

    println!(
        "METRIC sanskrit_text_grapheme_break_spec_rows {}",
        spec.len()
    );
    println!(
        "METRIC sanskrit_text_grapheme_break_rust_rows {}",
        rust.len()
    );

    // The floor first: an empty file would make every comparison below vacuous.
    assert!(
        spec.len() > 1000,
        "only {} rows read from spec/grapheme-break.tsv; the UCD carries over a \
         thousand and this test would otherwise pass on nothing",
        spec.len()
    );
    assert_eq!(
        spec, rust,
        "spec/grapheme-break.tsv and src/tables.rs disagree. Both are generated \
         from the SAME pinned UCD, so a difference means one generator was \
         re-run against a different Unicode version — which tables.rs's own \
         header records as a deliberate act that changes akṣara boundaries, \
         never a refresh. Re-run tools/gen-grapheme-break.py and say which UCD."
    );
}

/// The Devanagari column is a NAME per class, not a name per row.
///
/// Without this the table could carry 1386 distinct spellings and still agree
/// with Rust on every range — the check above compares the UAX column.
#[test]
fn each_break_class_has_exactly_one_devanagari_name() {
    let p = repo_root().join("spec/grapheme-break.tsv");
    let text = std::fs::read_to_string(&p).expect("spec/grapheme-break.tsv exists");
    let mut pairs: Vec<(String, String)> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| !l.starts_with("devanagari\t"))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f[1].to_string(), f[0].to_string())
        })
        .collect();
    pairs.sort();
    pairs.dedup();

    let mut names: Vec<&String> = pairs.iter().map(|(_, d)| d).collect();
    names.sort();
    let before = names.len();
    names.dedup();
    assert_eq!(
        before,
        names.len(),
        "two classes share one Devanagari name, so the column cannot identify a \
         class: {pairs:?}"
    );
    assert!(
        pairs
            .iter()
            .all(|(_, d)| d.chars().all(|c| ('\u{900}'..='\u{97F}').contains(&c))),
        "a name in the devanagari column is not Devanagari: {pairs:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The two tables UAX #29 needs beyond the break classes.
// ─────────────────────────────────────────────────────────────────────────

/// `spec/incb.tsv` and `src/tables.rs`'s `INCB` are the same pinned UCD.
///
/// **`GB9c` — the rule that makes क्ष ONE akṣara — is undecidable without this
/// table**, which is why `अक्षरकोश ॱ अक्षराणि` stayed stubbed after
/// `grapheme-break.tsv` landed: that cleared ONE THIRD of its blocker, not
/// half. An agent measured the rest against `GraphemeBreakTest.txt`, with a
/// reference first validated at 0 failures of 766 cases — without `InCB`, 16
/// fail; without `Extended_Pictographic`, 3; without both, 19.
///
/// It also refuted the shortcut anyone would reach for: faking `GB9c` from
/// Devanagari ranges still fails 9 of those 16, because the corpus exercises
/// the conjunct rule in FIVE scripts and only 7 of the cases are Devanagari.
/// So the shortcut is silently wrong in four scripts while claiming UAX #29.
#[test]
fn the_incb_table_agrees_with_the_generated_rust_one() {
    let rust: Vec<(u32, u32, String)> = sanskrit_text::tables::INCB
        .iter()
        .map(|(a, b, c)| (*a, *b, format!("{c:?}")))
        .collect();
    let spec: Vec<(u32, u32, String)> = read_spec("incb.tsv", 1);
    println!("METRIC sanskrit_text_incb_spec_rows {}", spec.len());
    assert!(
        spec.len() > 300,
        "only {} rows read from spec/incb.tsv; the UCD carries hundreds and \
         this test would otherwise pass on nothing",
        spec.len()
    );
    // `Incb::Linker` vs the TSV's `Linker` — compare case-insensitively on the
    // variant name, since the Rust side is a Debug rendering.
    let norm = |v: &[(u32, u32, String)]| -> Vec<(u32, u32, String)> {
        v.iter()
            .map(|(a, b, c)| (*a, *b, c.to_lowercase()))
            .collect()
    };
    assert_eq!(
        norm(&spec),
        norm(&rust),
        "spec/incb.tsv and tables.rs INCB disagree — both come from the SAME \
         pinned UCD, so a difference means one was regenerated against a \
         different Unicode version. Re-run tools/gen-incb.py and say which."
    );
}

/// `spec/extended-pictographic.tsv` against `EXTENDED_PICTOGRAPHIC`.
///
/// A boolean property, so only the RANGES are compared — the Rust side is a
/// `Flag` and carries no value to disagree about.
#[test]
fn the_extended_pictographic_table_agrees_with_the_generated_rust_one() {
    let rust: Vec<(u32, u32)> = sanskrit_text::tables::EXTENDED_PICTOGRAPHIC
        .iter()
        .map(|(a, b, _)| (*a, *b))
        .collect();
    let spec: Vec<(u32, u32)> = read_spec("extended-pictographic.tsv", 1)
        .into_iter()
        .map(|(a, b, _)| (a, b))
        .collect();
    println!("METRIC sanskrit_text_extpict_spec_rows {}", spec.len());
    assert!(
        spec.len() > 100,
        "only {} rows; too few to be the real table",
        spec.len()
    );
    assert_eq!(
        spec, rust,
        "spec/extended-pictographic.tsv and tables.rs disagree"
    );
}

/// Read a generated spec table: `(start, end, <column `col`>)`.
fn read_spec(name: &str, col: usize) -> Vec<(u32, u32, String)> {
    let p = repo_root().join("spec").join(name);
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!(
            "{} must exist — tools/gen-incb.py writes it: {e}",
            p.display()
        )
    });
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| !l.starts_with("devanagari\t"))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 4, "a row of {name} has too few fields: {l:?}");
            (
                u32::from_str_radix(f[2], 16).expect("start is hex"),
                u32::from_str_radix(f[3], 16).expect("end is hex"),
                f[col].to_string(),
            )
        })
        .collect()
}
