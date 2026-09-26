//! The frozen T0 grammar must agree with the code that will read it.
//!
//! `spec/grammar-t0.ebnf` is frozen: the assembler in `B-012` may depend on its
//! shape. A frozen document that quietly disagrees with the implementation is
//! worse than an unfrozen one, because everyone stops checking it.
//!
//! These tests hold it to the three sources it claims to be derived from:
//! ADR-0003 for punctuation, `numeral.rs` for literals, and the doc 15
//! repertoire for every terminal. Each has already been the source of a real
//! error in this project — a sign no font renders, a value written from memory,
//! a term coined twice — so none is checked on trust.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

fn grammar() -> String {
    std::fs::read_to_string(root().join("spec/grammar-t0.ebnf")).expect("read spec/grammar-t0.ebnf")
}

/// Every quoted terminal in the grammar.
fn terminals() -> Vec<String> {
    let text = grammar();
    let mut out = Vec::new();
    // Comments are (* … *) and may contain quotes in prose; skip them so a
    // worked example does not masquerade as a terminal.
    let mut rest = text.as_str();
    let mut code = String::new();
    while let Some(open) = rest.find("(*") {
        code.push_str(&rest[..open]);
        match rest[open..].find("*)") {
            Some(close) => rest = &rest[open + close + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    code.push_str(rest);

    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut lit = String::new();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                lit.push(c);
            }
            if !lit.is_empty() {
                out.push(lit);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn every_terminal_survives_orthographic_closure() {
    // A grammar terminal outside the repertoire could not appear in Sassembly
    // source, so the production naming it would be unreachable — the grammar
    // would describe a language nobody can type.
    for t in terminals() {
        let violations = sanskrit_text::repertoire::check(&t);
        assert!(
            violations.is_empty(),
            "terminal {t:?} is outside the doc 15 repertoire: {violations:?}"
        );
    }
}

#[test]
fn no_latin_anywhere_in_the_terminals() {
    // The whole point of doc 15. If a Latin colon or bracket crept into a
    // production, orthographic closure would be breached by the grammar itself.
    for t in terminals() {
        assert!(
            !t.chars()
                .any(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation()),
            "terminal {t:?} contains ASCII — Sassembly source admits none"
        );
    }
}

#[test]
fn punctuation_matches_adr_0003() {
    // ADR-0003 chose these after measuring 27 installed Devanagari faces: the
    // Vedic-extension signs originally proposed are rendered by none of them.
    // A grammar that reintroduced one would be unreadable on every real system.
    let g = grammar();
    for (role, sign) in [
        ("statement end", "।"),
        ("directive end", "॥"),
        ("comment", "॰"),
        ("member access", "ॱ"),
        ("argument separator", "ऽ"),
    ] {
        assert!(
            g.contains(&format!("\"{sign}\"")),
            "{role} sign {sign} is missing from the grammar"
        );
    }
    for banned in [
        '\u{A8FC}', '\u{A8F8}', '\u{A8FA}', '\u{A8FB}', '\u{A8F9}', '\u{1CF5}', '\u{1CF6}',
    ] {
        assert!(
            !g.contains(banned),
            "grammar uses {banned:?}, which ADR-0003 removed because no installed face renders it"
        );
    }
}

#[test]
fn numeral_prefixes_match_the_implementation() {
    // The grammar claims to describe what numeral.rs already accepts. If the
    // two drift, the assembler will reject literals the grammar permits.
    let g = grammar();
    for (prefix, sample, radix) in [
        ("०द्वि", "०द्वि१०१", 2u32),
        ("०अष्ट", "०अष्ट७७", 8),
        ("०षोड्", "०षोड्ऊ", 16),
    ] {
        assert!(
            g.contains(&format!("\"{prefix}\"")),
            "grammar is missing the {radix}-radix prefix {prefix}"
        );
        assert!(
            sanskrit_text::is_numeral(sample),
            "numeral.rs rejects {sample}, which the grammar accepts"
        );
    }
    // Bare digits are decimal, and a prefix with no digits is not a numeral.
    assert!(sanskrit_text::is_numeral("१२३"));
    assert!(!sanskrit_text::is_numeral("०षोड्"));

    // The sign prefix — ADR-0009. It sits OUTSIDE the radix prefix, in the
    // order the number is said: negative, base, digits.
    assert!(
        g.contains("\"ऋण\""),
        "grammar is missing the sign prefix ऋण"
    );
    for sample in ["ऋण१", "ऋण३२", "ऋण०षोड्ऊ", "ऋण०द्वि१०१"]
    {
        assert!(
            sanskrit_text::is_numeral(sample),
            "numeral.rs rejects {sample}, which the grammar accepts"
        );
    }
    assert!(!sanskrit_text::is_numeral("ऋण"), "a sign with no digits");
    assert_eq!(
        sanskrit_text::numeral::bits("ऋण०षोड्ऊ").expect("parses") as i64,
        -15,
        "the sign applies to the whole literal, not to the last digit"
    );
    // `value` refuses a negative rather than wrapping it, so a caller that has
    // not thought about sign cannot get 18446744073709551615 by accident.
    //
    // The VARIANT is pinned, not merely that it errs (`W-075`). It used to
    // answer `NoDigits` for `ऋण१`, which has a digit — a refusal naming a rule
    // the writer did not break sends them looking for a typo instead of for the
    // reader that has no sign to give them.
    assert_eq!(
        sanskrit_text::numeral::value("ऋण१"),
        Err(sanskrit_text::numeral::NumeralError::Signed)
    );
}

#[test]
fn hex_digits_are_the_six_implemented_vowels() {
    // numeral.rs maps 10–15 to अ आ इ ई उ ऊ. These letters are common in
    // ordinary words, which is exactly why they are only ever legal behind the
    // ०षोड् prefix — the grammar must not admit them free-standing as digits.
    let g = grammar();
    for v in ["अ", "आ", "इ", "ई", "उ", "ऊ"] {
        assert!(g.contains(&format!("\"{v}\"")), "hex digit {v} missing");
    }
    assert!(
        sanskrit_text::is_numeral("०षोड्अआइईउऊ"),
        "the six hex letters must parse behind the prefix"
    );
    assert!(
        !sanskrit_text::is_numeral("अआइईउऊ"),
        "hex letters must NOT be a numeral without the prefix — they are words"
    );
}

#[test]
fn every_karaka_role_from_d_02_c_is_present() {
    // Five roles, five sigils. A missing one would silently make an operand
    // direction inexpressible, and the assembler would need a positional
    // convention — which is precisely what D-02-C exists to remove.
    let g = grammar();
    for (role, sigil) in [
        ("destination", "म्"),
        ("source", "न"),
        ("source_address", "त्"),
        ("dest_address", "य्"),
        ("locus", "ए"),
    ] {
        assert!(g.contains(role), "kāraka role {role} is missing");
        assert!(g.contains(&format!("\"{sigil}\"")), "sigil {sigil} missing");
    }
}

#[test]
fn the_grammar_is_frozen_and_says_so() {
    let g = grammar();
    assert!(
        g.contains("FROZEN"),
        "a grammar the assembler depends on must state that it is frozen"
    );
}

#[test]
fn every_production_referenced_is_defined() {
    // An EBNF that references a production it never defines is not a grammar,
    // it is a sketch — and B-012 would discover that instead of this test.
    let text = grammar();
    let mut code = String::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find("(*") {
        code.push_str(&rest[..open]);
        match rest[open..].find("*)") {
            Some(close) => rest = &rest[open + close + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    code.push_str(rest);

    let mut defined: Vec<String> = Vec::new();
    for line in code.lines() {
        if let Some((lhs, _)) = line.split_once('=')
            && !lhs.trim().is_empty()
            && lhs
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_whitespace())
        {
            defined.push(lhs.trim().to_string());
        }
    }

    // Terminals the lexer supplies rather than the grammar defining them.
    let primitive = ["aksara", "non_newline", "newline"];

    let mut referenced: Vec<String> = Vec::new();
    for line in code.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let mut word = String::new();
        for c in body.chars().chain(std::iter::once(' ')) {
            if c.is_ascii_lowercase() || c == '_' {
                word.push(c);
            } else {
                if word.len() > 2 {
                    referenced.push(word.clone());
                }
                word.clear();
            }
        }
    }
    referenced.sort();
    referenced.dedup();

    let missing: Vec<&String> = referenced
        .iter()
        .filter(|r| !defined.contains(r) && !primitive.contains(&r.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "productions referenced but never defined: {missing:?}"
    );
    println!("METRIC t0_grammar_productions {}", defined.len());
}
