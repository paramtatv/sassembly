//! The text-directive exemption, read off the LEXER instead of off a model.
//!
//! `crates/sanskrit-text/tests/repertoire_census.rs` splits
//! `repertoire_violations` in two — 13,442 characters the lexer refuses and
//! 1,724 a text directive carries — and it draws that line by MODELLING
//! `crates/sadhana/src/lex.rs`: the repertoire check is off between the word
//! `आस्की` and the closing `॥` (`lex.rs:584`, cleared at `:678` and `:724`),
//! and narrowed rather than off for `जाल` (`:598`, a named list of thirteen
//! punctuation marks plus the ASCII letters).
//!
//! **A model of the product that only ever agrees with itself is not evidence.**
//! `sanskrit-text` cannot depend on `sadhana` — `sadhana` depends on IT — so the
//! census cannot call the lexer and its split would stand on a reading of the
//! source. This file is the other side: it takes the REAL lexer as ground,
//! walks the same population with its OWN loader, and asks the product the two
//! questions the census answers from a model.
//!
//! # What each half proves
//!
//! [`every_governed_source_naming_a_text_directive_lexes_clean`] is the 1,724:
//! ten `spec/*.sas` files carry `॥ आस्की BOOT-COUNTER-FRESH ॥` and
//! `॥ जाल VIRTIO-NET-OK ॥` — ASCII a SANSOS program emits to a console or a
//! wire — and the toolchain assembles every one of them. Whatever `O-15-2`
//! decides about doc 15 §4's reach, those characters are not a defect TODAY.
//!
//! [`a_source_outside_a_text_directive_is_still_refused`] is the other 13,442,
//! on real lines and not a fixture: the lexer reds on
//! `crates/sadhana/src/t1/tests/repertoire.sas` and on
//! `crates/textapp/src/text/numeral.t1`. Two populations, one rule, opposite
//! answers — which is exactly why one figure could not describe both.
//!
//! [`the_exemption_ends_at_the_bracket_and_the_web_list_is_partial`] pins the
//! two boundaries the census hard-codes. If `lex.rs`'s allow-list is ever
//! widened or the `॥` reset dropped, this file reds and the model is KNOWN
//! stale instead of quietly wrong.

use std::path::{Path, PathBuf};
use std::process::Command;

use sadhana::lex::{LexError, lex, lex_t1};

/// Doc 15 §4 row 1, by EXTENSION and never by directory — the census's rule,
/// restated here rather than shared, because a shared helper would move both
/// numbers the same way and neither would say so.
const GOVERNED_EXTENSIONS: &[&str] = &["सस", "sas", "t1"];

/// D-002f6 and D-002f7 (`vakyavibhaga.t1:1881`), the two text directives.
const TEXT_DIRECTIVES: &[&str] = &["आस्की", "जाल"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

fn is_governed(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| GOVERNED_EXTENSIONS.contains(&ext))
}

/// Whether the source NAMES a text directive, as a whole space-delimited word.
///
/// Split on ASCII whitespace and compared whole. A Devanagari word boundary is
/// not a `\b` and a substring test would match `आस्कीसंज्ञा`
/// (`vakyavibhaga.t1:1934`), which is a variable and not a directive — the same
/// substring trap `W-312` caught in `calls_into`.
///
/// Deliberately CRUDER than the census's scanner: it does not know about
/// comments, strings or the closing `॥`. It only has to pick out the files
/// worth asking the lexer about, and being cruder is what makes it a second
/// reading rather than a copy.
fn names_a_text_directive(src: &str) -> bool {
    src.split_ascii_whitespace()
        .any(|w| TEXT_DIRECTIVES.contains(&w))
}

/// This file's OWN loader (`W-312`'s rule: a test loader is part of the test).
///
/// Panics rather than returning an empty list: "could not run git" reported as
/// a clean result names a cause that does not exist.
fn governed_sources(root: &Path) -> Vec<(String, String)> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files must run — without it NOTHING was measured");
    assert!(
        out.status.success(),
        "git ls-files failed ({}) — NOTHING was measured",
        out.status
    );
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty() && is_governed(p))
        .map(|p| {
            let text = std::fs::read_to_string(root.join(p))
                .unwrap_or_else(|e| panic!("{p} is tracked Sassembly source and unreadable: {e}"));
            (p.to_string(), text)
        })
        .collect()
}

/// The lexer's repertoire complaints about one source, and ONLY those.
///
/// A `.सस` or `.sas` may fail to lex for reasons that have nothing to do with
/// R-15-1 — an unclosed `उक्तम्`, say — and counting those here would make this
/// file red on an unrelated defect and say "repertoire" while doing it.
fn repertoire_errors(path: &str, src: &str) -> Vec<LexError> {
    let result = if path.ends_with(".t1") {
        lex_t1(src)
    } else {
        lex(src)
    };
    result
        .err()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.reason.contains("outside the doc 15 repertoire"))
        .collect()
}

#[test]
fn every_governed_source_naming_a_text_directive_lexes_clean() {
    let root = root();
    let sources = governed_sources(&root);

    // A floor, not a pin (the owner's 2026-09-13 ruling): a walk that reached
    // nothing would otherwise pass by finding nothing to check.
    assert!(
        sources.len() >= 150,
        "only {} governed sources found — the population walk is broken",
        sources.len()
    );

    let carrying: Vec<_> = sources
        .iter()
        .filter(|(_, src)| names_a_text_directive(src))
        .collect();
    assert!(
        !carrying.is_empty(),
        "no governed source names a text directive — this test checks nothing, \
         and the census's directive split would be measuring a region no file \
         reaches"
    );

    let mut refused = Vec::new();
    for (path, src) in &carrying {
        let errors = repertoire_errors(path, src);
        if let Some(first) = errors.first() {
            refused.push(format!(
                "{path}:{} `{}` ({} more)",
                first.line,
                first.aksara,
                errors.len() - 1
            ));
        }
    }
    assert!(
        refused.is_empty(),
        "the toolchain refuses a source whose only Latin is a text directive's \
         operand, so those characters ARE a defect and the census's split is \
         wrong: {refused:#?}"
    );

    println!(
        "NOTE lexer_sources_carrying_a_text_directive {}",
        carrying.len()
    );
    for (path, _) in &carrying {
        println!("NOTE lexer accepts the text directives in: {path}");
    }
}

/// The case that must still be REFUSED, on real lines rather than a fixture.
///
/// These two files are 26 and 4 of the census's 13,442, and nothing about them
/// is inside a directive: `repertoire.sas` writes `in_repertoire` and `32` in
/// Latin and ASCII digits, `numeral.t1` writes `=`. If the exemption ever leaks
/// past its bracket these stop being errors, the two populations collapse into
/// one, and the split stops meaning anything.
#[test]
#[ignore = "census: needs the development repository's tracked source tree not in the public repository"]
fn a_source_outside_a_text_directive_is_still_refused() {
    let root = root();
    for path in [
        "crates/sadhana/src/t1/tests/repertoire.sas",
        "crates/textapp/src/text/numeral.t1",
    ] {
        let src = std::fs::read_to_string(root.join(path)).expect("tracked source");
        assert!(
            !names_a_text_directive(&src),
            "{path} carries a text directive — it is the wrong control for this"
        );
        let errors = repertoire_errors(path, &src);
        assert!(
            !errors.is_empty(),
            "{path} carries characters outside R-15-1 and the lexer accepted it"
        );
        println!(
            "NOTE lexer refuses {path}: {} repertoire errors, first `{}` at line {}",
            errors.len(),
            errors[0].aksara,
            errors[0].line
        );
    }
}

/// The two boundaries the census MODELS, asked of the product.
///
/// Each of these was run against the real lexer before the census was written,
/// and each is the mutation that would make the split lie in a different way.
#[test]
fn the_exemption_ends_at_the_bracket_and_the_web_list_is_partial() {
    // THE BRACKET ENDS IT. Without the `॥` reset the exemption would run to the
    // end of the file and forgive every Latin character after the first
    // directive — the largest way the split could overcount its own half.
    let errors = repertoire_errors("a.sas", "॥ आस्की ABC ॥ DEF\n");
    assert_eq!(
        errors.len(),
        1,
        "the Latin after the closing ॥ must be refused: {errors:?}"
    );
    assert_eq!(errors[0].aksara, "D");

    // ...and INSIDE the brackets the same letters are accepted, so the refusal
    // above is the bracket and not the letters.
    assert!(repertoire_errors("a.sas", "॥ आस्की ABC ॥\n").is_empty());

    // `जाल` FORGIVES A NAMED LIST AND NOT A CHARACTER OUTSIDE IT. Reading it as
    // a second blanket exemption would report a character the lexer really
    // refuses as a pending ruling.
    let errors = repertoire_errors("a.sas", "॥ जाल A&B ॥\n");
    assert_eq!(errors.len(), 1, "the `&` is not on जाल's list: {errors:?}");
    assert_eq!(errors[0].aksara, "&");
    let errors = repertoire_errors("a.sas", "॥ जाल A1B ॥\n");
    assert_eq!(
        errors[0].aksara, "1",
        "an ASCII digit is not on the list either"
    );
    // And the same operand under `आस्की` is forgiven whole — the two exemptions
    // are counted apart because they ARE apart.
    assert!(repertoire_errors("a.sas", "॥ आस्की A&B ॥\n").is_empty());

    // A WHOLE WORD OPENS IT. `lex.rs:583` compares `word == "आस्की"`, so a
    // compound ending in the name opens nothing.
    let errors = repertoire_errors("a.sas", "कआस्की ABC ॥\n");
    assert_eq!(errors[0].aksara, "A", "कआस्की is not the directive");

    // THE FLAG OUTLIVES THE LINE, and a `॰` inside the operand is still a
    // comment — the line is stripped before the words are looked at.
    assert!(repertoire_errors("a.sas", "॥ आस्की ABC\nDEF ॥\n").is_empty());
    assert!(repertoire_errors("a.sas", "॥ आस्की ABC ॰ GHI\nDEF ॥\n").is_empty());
}
