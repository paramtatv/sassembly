//! `SAS-011` fix (1), the PREDICATE — `parse::string_payload`, the writer's
//! half of `parse.rs`'s `string_body`.
//!
//! The backlog row measures the assembler at ~3,500 interpreter steps per DATA
//! OCTET when a literal travels as one numeral per octet. The same octets as a
//! `उक्तम् … इति` text piece are read once. This file decides WHICH octets may
//! take that spelling, and the deciding question is not "is it text" but "does
//! the assembler read back exactly these octets".
//!
//! # The round trip is the test, and the assembler performs it
//!
//! Every accepted run here is spelled into a directive, run through the REAL
//! lexer and the REAL parser (`parse::assemble_program`), and the datum's
//! octets are compared with the input. A predicate checked against a
//! hand-written list of allowed characters would agree with itself; this one
//! cannot pass unless the toolchain agrees.
//!
//! # Fail-first
//!
//! Each refusal below was first asserted as an ACCEPTANCE — the predicate
//! widened to admit it — and the round trip was run. Every one came back a
//! different run of octets or a parse error, which is the evidence that these
//! are refusals of substance and not taste:
//!
//! | run                 | what the round trip did, measured |
//! |---------------------|-----------------------------------|
//! | `क  ख` (two spaces) | 8 octets in, 7 out — the reader rejoins with ONE space |
//! | `क ॥ ख`             | `॥` closed the directive, so no `इति` closed the literal |
//! | `क ॰ ख`             | the comment ate the close; the directive never closed |
//! | `कॱॱ ख`             | `कॱॱ` became a LABEL DEFINITION and `ख` an unknown directive |

use sadhana::parse::{self, Section};

/// This file's own loader (the trap: a test loader is part of the test).
///
/// Spells one run of octets the way an emitter would — `पाठ०ॱॱ` and one
/// `अष्टकाः` directive holding the text piece — assembles it, and answers the
/// octets the assembler actually recorded.
///
/// The label matters: a directive with no label before it is still data, but
/// the emitters write the label, so the text under test is the text they emit.
fn round_trip(payload: &str) -> Result<Vec<u8>, Vec<String>> {
    let source = format!("पाठ०ॱॱ\n॥ अष्टकाः उक्तम् {payload} इति ॥\n");
    let program = parse::assemble_program(&source)?;
    let data: Vec<&parse::Datum> = program
        .data
        .iter()
        .filter(|d| d.section == Section::Data || d.section == Section::Text)
        .collect();
    assert_eq!(
        data.len(),
        1,
        "one directive is one datum; got {} from\n{source}",
        data.len()
    );
    Ok(data[0].bytes.clone())
}

/// The runs the predicate must ACCEPT, and the assembler must return unchanged.
///
/// Drawn from the shapes a literal in the corpus actually has: a word, a
/// conjunct, digits, a vowel sign, an interior `इ` that is not the close, a
/// word that BEGINS with the close's spelling, and one long word. ONE WORD
/// EACH: the narrow rule (owner, 2026-10-06) admits no space at all.
const ACCEPTED: &[&str] = &[
    "क",
    "अब",
    "क्ष",
    "नमस्ते",
    "०१२३४५६७८९",
    "इ",
    "इत",
    "गतिति",
    "ॐ",
    "कार्यम्",
    "इतिहासः",
    "यन्त्रोत्सर्जनम्",
];

#[test]
fn an_accepted_payload_reads_back_octet_for_octet() {
    for text in ACCEPTED {
        let bytes = text.as_bytes();
        let spelled = parse::string_payload(bytes)
            .unwrap_or_else(|| panic!("`{text}` is a payload the predicate must accept"));
        assert_eq!(
            spelled, *text,
            "the spelling is the text itself — nothing is escaped on this path"
        );
        let back = round_trip(&spelled).unwrap_or_else(|e| panic!("`{text}` assembles: {e:?}"));
        assert_eq!(
            back, bytes,
            "`{text}` did not survive the assembler: {:?} in, {:?} out",
            bytes, back
        );
    }
}

/// The runs the predicate must REFUSE. Each is a measured round-trip failure
/// (the table in this file's header), not a stylistic rule.
#[test]
fn a_run_that_does_not_round_trip_is_refused() {
    let refused: &[(&str, &str)] = &[
        ("", "an empty literal takes its label and NO directive"),
        ("क  ख", "two spaces come back as one"),
        (" कख", "a leading space belongs to the delimiter"),
        ("कख ", "a trailing space belongs to the delimiter"),
        ("क\tख", "a tab is not the space the reader rejoins with"),
        ("क\nख", "a newline ends the directive"),
        ("क ॥ ख", "`॥` closes the directive"),
        ("क ॰ ख", "`॰` ends the line"),
        ("क । ख", "`।` ends the statement"),
        ("कॱॱ ख", "`ॱॱ` is the label mark"),
        ("क इति ख", "a bare `इति` closes the literal"),
        ("इति", "the close alone is not a payload"),
        (
            "क आस्की ख",
            "`आस्की` switches the lexer's repertoire gate off",
        ),
        ("क जाल ख", "`जाल` switches the lexer into markup mode"),
        ("abc", "Latin is not in the repertoire"),
        ("क x ख", "one Latin letter is still Latin"),
        ("ᬓ", "Balinese is valid UTF-8 and not Devanagari"),
        (
            "꣹",
            "doc 15 §3.1's rejected sigil is outside the base block",
        ),
        (
            "\u{A8E1}",
            "Devanagari EXTENDED is in R-15-1 and not on this path",
        ),
    ];
    for (text, why) in refused {
        assert!(
            parse::string_payload(text.as_bytes()).is_none(),
            "`{}` must be refused: {why}",
            text.escape_debug()
        );
    }
}

/// `SAS-011` (c), THE NARROW RULE (owner ruling, 2026-10-06): a literal is
/// written as text only when it is ONE word — no space anywhere — and that word
/// is not `इति`, `आस्की` or `जाल`. These runs round-trip through the
/// assembler (the first four were in `ACCEPTED` until this ruling, and the
/// control below re-proves it) and are refused anyway: the rule buys the T1
/// twin a predicate with no word splitting, which is what the ~1.5 KB of
/// compiler image the owner accepted pays for.
#[test]
fn the_narrow_rule_refuses_every_space_and_the_three_whole_words() {
    let spaced = ["क ख", "क ख ग घ", "पदविभागः चिह्नकपाठः", "कार्यम् भवति"];
    for text in spaced {
        assert_eq!(
            round_trip(text).as_deref().ok(),
            Some(text.as_bytes()),
            "CONTROL: `{text}` must round-trip, or this is not a refusal of taste"
        );
        assert!(
            parse::string_payload(text.as_bytes()).is_none(),
            "`{text}` holds a space, which the narrow rule refuses"
        );
    }
    for word in ["इति", "आस्की", "जाल"] {
        assert!(
            parse::string_payload(word.as_bytes()).is_none(),
            "`{word}` alone is one of the three words the rule names"
        );
    }
}

/// EVERY character of U+0900–U+097F as a one-character literal: the predicate
/// accepts it only if the assembler gives the same octets back. This is the
/// claim the T1 twin's octet walk rests on (`E0 A4|A5 80–BF` less five
/// triples), checked over the whole block rather than over examples.
#[test]
fn every_accepted_base_block_character_round_trips() {
    let mut accepted = 0;
    for cp in 0x0900u32..=0x097F {
        let ch = char::from_u32(cp).expect("the base block is all scalar values");
        let text = ch.to_string();
        if parse::string_payload(text.as_bytes()).is_some() {
            accepted += 1;
            assert_eq!(
                round_trip(&text).as_deref().ok(),
                Some(text.as_bytes()),
                "U+{cp:04X} is accepted and does not survive the assembler"
            );
        }
    }
    assert_eq!(accepted, 128 - 5, "the block less ऽ । ॥ ॰ ॱ");
}

/// `SAS-011` (b): the AVAGRAHA `ऽ` (U+093D) sits inside U+0900–U+097F and the
/// predicate's four excluded marks did not name it, so it was ACCEPTED — but
/// `lex.rs`'s `split_trailing_punct` peels a trailing `ऽ` off a word as the
/// separator token, and the reader then rejoins the two pieces with a space.
///
/// The control runs the assembler on the spelling directly, without the
/// predicate, so it holds whatever the predicate says: if it ever goes green
/// for the trailing case the lexer has changed and this refusal may be
/// re-examined. The refusal covers every position, not only the trailing one,
/// because the T1 twin has to agree octet for octet and a per-position rule is
/// a second place for the two to drift.
#[test]
fn the_avagraha_is_refused_because_the_lexer_peels_it() {
    let text = "कऽ";
    let back = round_trip(text);
    assert_ne!(
        back.as_deref().ok(),
        Some(text.as_bytes()),
        "CONTROL: `{text}` read back unchanged, so the lexer no longer peels `ऽ`"
    );
    for text in ["कऽ", "कऽख", "ऽ", "क ऽ ख", "सोऽहम्"] {
        assert!(
            parse::string_payload(text.as_bytes()).is_none(),
            "`{text}` holds the avagraha, which the lexer splits off a word"
        );
    }
}

/// Octets that are not UTF-8 at all — the common case, since a literal is an
/// arbitrary run and most runs are not text.
#[test]
fn octets_that_are_not_utf8_are_refused() {
    let runs: &[&[u8]] = &[
        &[0xff],
        &[0x00],
        &[0xe0, 0xa4],             // a truncated Devanagari sequence
        &[0xe0, 0xa4, 0x95, 0x80], // a lone combining octet after `क`
        &[0xc0, 0x80],             // an overlong NUL
        &[0xed, 0xa0, 0x80],       // a surrogate
    ];
    for run in runs {
        assert!(
            parse::string_payload(run).is_none(),
            "{run:?} is not text and must be refused"
        );
    }
}

/// THE REASON THE FIX EXISTS, stated as a measurement rather than a claim: the
/// text form of a run is several times shorter than the numeral form it
/// replaces, and the ratio grows with the octet count because every octet
/// carries its own numeral and its own space.
#[test]
fn the_text_form_is_shorter_than_the_numerals_it_replaces() {
    let text = "यन्त्रोत्सर्जनम्";
    let octets = text.as_bytes();
    let payload = parse::string_payload(octets).expect("one word is a payload");
    let spelled = format!("॥ अष्टकाः उक्तम् {payload} इति ॥");
    // What the emitters write today: one Devanagari numeral per octet.
    let numerals: Vec<String> = octets
        .iter()
        .map(|o| sadhana::t1::riscv64::devanagari(i64::from(*o)))
        .collect();
    let today = format!("॥ अष्टकाः {} ॥", numerals.join(" "));
    assert!(
        today.chars().count() > 3 * spelled.chars().count(),
        "the numeral form is {} characters and the text form {} — the row claims 4x, \
         so anything under 3x means the shapes changed and the row needs re-measuring",
        today.chars().count(),
        spelled.chars().count()
    );
}
