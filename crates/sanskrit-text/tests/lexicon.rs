//! Task `A-036` — lexicon validator.
//!
//! `spec/lexicon.tsv` is the vocabulary every diagnostic, every document and
//! every the host application prompt resolves through (doc 01 §1.2.4). A term that means two
//! things here means two things everywhere, so the invariants are enforced
//! rather than reviewed.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sanskrit_text::{is_nfc, is_numeral, slp1, validate_identifier};

struct Entry {
    deva: String,
    slp1: String,
    english: String,
    category: String,
}

fn lexicon() -> Vec<Entry> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("spec/lexicon.tsv");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}\nrun `cargo run -p lexgen`", path.display()));

    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("devanagari\t") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        out.push(Entry {
            deva: f[0].into(),
            slp1: f[1].into(),
            english: f[3].into(),
            category: f[4].into(),
        });
    }
    assert!(
        out.len() >= 400,
        "lexicon v0.1 needs >= 400 terms, found {}",
        out.len()
    );
    out
}

/// Doc 01 §4 rule 5: one word, one sense. Building the lexicon found eleven
/// violations; this stops the twelfth.
#[test]
fn no_term_carries_two_senses() {
    let mut by_word: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let entries = lexicon();
    for e in &entries {
        by_word.entry(&e.deva).or_default().push(&e.english);
    }
    let clashes: Vec<String> = by_word
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, v)| format!("  {k} means {v:?}"))
        .collect();
    assert!(
        clashes.is_empty(),
        "{} term(s) carry more than one sense (doc 01 §4 rule 5):\n{}",
        clashes.len(),
        clashes.join("\n")
    );
}

/// The reverse: two words for one concept is drift, not richness.
#[test]
fn no_concept_has_two_words() {
    let mut by_sense: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let entries = lexicon();
    for e in &entries {
        by_sense.entry(&e.english).or_default().push(&e.deva);
    }
    let dups: Vec<String> = by_sense
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, v)| format!("  {k:?} is spelled {v:?}"))
        .collect();
    assert!(dups.is_empty(), "synonym drift:\n{}", dups.join("\n"));
}

#[test]
fn every_term_is_nfc() {
    for e in lexicon() {
        assert!(is_nfc(&e.deva), "{} is not NFC", e.deva);
    }
}

/// Anything that will become a token must be a legal identifier, or the lexer
/// cannot accept the vocabulary its own diagnostics are written in.
#[test]
fn every_word_like_term_is_a_valid_identifier() {
    let mut bad = Vec::new();
    for e in lexicon() {
        if e.category == "sign" || e.deva.contains(' ') || is_numeral(&e.deva) {
            continue;
        }
        if let Err(err) = validate_identifier(&e.deva) {
            bad.push(format!("  {} ({}): {err:?}", e.deva, e.english));
        }
    }
    assert!(
        bad.is_empty(),
        "invalid identifiers in the lexicon:\n{}",
        bad.join("\n")
    );
}

/// The generated SLP1 column must round-trip. 419 real terms is a far better
/// exercise of the encoder than any generated corpus.
#[test]
fn slp1_column_round_trips_for_every_term() {
    for e in lexicon() {
        if e.slp1 == "-" {
            continue;
        }
        let mut back = String::new();
        slp1::decode_into(&e.slp1, &mut back)
            .unwrap_or_else(|err| panic!("{}: decode {err:?}", e.deva));
        assert_eq!(back, e.deva, "slp1 {:?} does not return {}", e.slp1, e.deva);

        let mut fwd = String::new();
        slp1::encode_into(&e.deva, &mut fwd).unwrap();
        assert_eq!(
            fwd, e.slp1,
            "regenerating {} gives a different slp1",
            e.deva
        );
    }
}

/// Coverage report, so "does the language have the words it needs" has a number.
#[test]
fn report_lexicon_coverage() {
    let entries = lexicon();
    let mut by_cat: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &entries {
        *by_cat.entry(&e.category).or_default() += 1;
    }
    println!("METRIC lexicon_terms_ratified {}", entries.len());
    println!("\nlexicon v0.1 — {} terms", entries.len());
    for (c, n) in &by_cat {
        println!("  {c:14} {n:3}");
    }
    // The categories the toolchain cannot start without.
    for required in [
        "mnemonic",
        "keyword",
        "type",
        "directive",
        "operator",
        "error",
    ] {
        assert!(
            by_cat.get(required).copied().unwrap_or(0) >= 10,
            "category {required} is too thin to build against"
        );
    }
}
