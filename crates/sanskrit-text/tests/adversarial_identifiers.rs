//! Task `A-018` — adversarial identifier vectors.
//!
//! Also security suites **S4.7** (confusable identifiers) and **S4.8** (Trojan
//! Source) from doc 18 §6, and the implementation of corpus rows `TX-03`
//! through `TX-07` in [`tests/safety/CORPUS.tsv`](../../../tests/safety/CORPUS.tsv).
//!
//! Every vector below is an attempt to make two identifiers that a **human
//! reviewer cannot tell apart** but the **linker treats as different symbols**.
//! That gap is how a malicious patch passes review. Rust warns on some of these;
//! doc 15's orthographic closure plus the UAX #31 profile make most of them
//! unrepresentable, which is the difference between a lint and a guarantee.
//!
//! Rule: **every vector must be rejected.** A vector that starts passing is a
//! hole, and the test says which class of attack just opened.

use sanskrit_text::{IdentError, validate_identifier as validate};

/// `(attack class, description, hostile input)`
const ATTACKS: &[(&str, &str, &str)] = &[
    // ---- mixed-script homographs (UTS #39; corpus TX-04, TX-07) -------------
    (
        "homograph",
        "Latin 'a' inside a Devanagari name",
        "\u{0915}a\u{0937}",
    ),
    ("homograph", "pure Latin posing as an identifier", "kosha"),
    (
        "homograph",
        "Bengali ক among Devanagari",
        "\u{0915}\u{0995}",
    ),
    (
        "homograph",
        "Gujarati ક among Devanagari",
        "\u{0915}\u{0A95}",
    ),
    (
        "homograph",
        "Kannada ಕ among Devanagari",
        "\u{0915}\u{0C95}",
    ),
    ("homograph", "Tamil க among Devanagari", "\u{0915}\u{0B95}"),
    ("homograph", "Telugu క among Devanagari", "\u{0915}\u{0C15}"),
    ("homograph", "Cyrillic а appended", "\u{0915}\u{0430}"),
    ("homograph", "Greek ο appended", "\u{0915}\u{03BF}"),
    ("homograph", "fullwidth Latin", "\u{0915}\u{FF41}"),
    ("homograph", "mathematical bold Latin", "\u{0915}\u{1D41A}"),
    (
        "homograph",
        "Devanagari Extended letter (outside base block)",
        "\u{0915}\u{A8F2}",
    ),
    // ---- invisible differences (corpus TX-06) -------------------------------
    (
        "invisible",
        "ZWNJ not after virāma",
        "\u{0915}\u{200C}\u{0937}",
    ),
    (
        "invisible",
        "ZWJ not after virāma",
        "\u{0915}\u{200D}\u{0937}",
    ),
    (
        "invisible",
        "ZWJ after a mātrā",
        "\u{0915}\u{093E}\u{200D}\u{0937}",
    ),
    ("invisible", "trailing ZWJ", "\u{0915}\u{094D}\u{200D}"),
    ("invisible", "trailing ZWNJ", "\u{0915}\u{094D}\u{200C}"),
    (
        "invisible",
        "joiner after virāma but before a vowel sign",
        "\u{0915}\u{094D}\u{200D}\u{093E}",
    ),
    ("invisible", "leading ZWJ", "\u{200D}\u{0915}"),
    (
        "invisible",
        "double joiner",
        "\u{0915}\u{094D}\u{200D}\u{200D}\u{0937}",
    ),
    (
        "invisible",
        "zero-width no-break space (BOM) inside",
        "\u{0915}\u{FEFF}\u{0937}",
    ),
    (
        "invisible",
        "word joiner U+2060",
        "\u{0915}\u{2060}\u{0937}",
    ),
    ("invisible", "soft hyphen", "\u{0915}\u{00AD}\u{0937}"),
    (
        "invisible",
        "invisible times U+2062",
        "\u{0915}\u{2062}\u{0937}",
    ),
    (
        "invisible",
        "invisible separator U+2063",
        "\u{0915}\u{2063}\u{0937}",
    ),
    // ---- Trojan Source: bidi reordering (corpus TX-05) ----------------------
    (
        "bidi",
        "RLO right-to-left override",
        "\u{0915}\u{202E}\u{0937}",
    ),
    (
        "bidi",
        "LRO left-to-right override",
        "\u{0915}\u{202D}\u{0937}",
    ),
    ("bidi", "RLE embedding", "\u{0915}\u{202B}\u{0937}"),
    ("bidi", "LRE embedding", "\u{0915}\u{202A}\u{0937}"),
    (
        "bidi",
        "PDF pop directional formatting",
        "\u{0915}\u{202C}\u{0937}",
    ),
    ("bidi", "RLI isolate", "\u{0915}\u{2067}\u{0937}"),
    ("bidi", "LRI isolate", "\u{0915}\u{2066}\u{0937}"),
    (
        "bidi",
        "FSI first-strong isolate",
        "\u{0915}\u{2068}\u{0937}",
    ),
    ("bidi", "PDI pop isolate", "\u{0915}\u{2069}\u{0937}"),
    ("bidi", "LRM mark", "\u{0915}\u{200E}\u{0937}"),
    ("bidi", "RLM mark", "\u{0915}\u{200F}\u{0937}"),
    ("bidi", "Arabic letter mark", "\u{0915}\u{061C}\u{0937}"),
    ("bidi", "leading RLO", "\u{202E}\u{0915}"),
    // ---- structurally invalid (would corrupt the symbol table) --------------
    ("structure", "empty identifier", ""),
    ("structure", "leading mātrā, no base", "\u{093F}\u{0915}"),
    ("structure", "leading virāma", "\u{094D}\u{0915}"),
    ("structure", "leading anusvāra", "\u{0902}\u{0915}"),
    ("structure", "leading digit", "\u{0966}\u{0915}"),
    ("structure", "leading danda", "\u{0964}\u{0915}"),
    ("structure", "danda inside", "\u{0915}\u{0964}\u{0937}"),
    (
        "structure",
        "avagraha inside (it is a separator, not a letter)",
        "\u{0915}\u{093D}\u{0937}",
    ),
    ("structure", "space inside", "\u{0915} \u{0937}"),
    ("structure", "NUL inside", "\u{0915}\u{0000}\u{0937}"),
    ("structure", "newline inside", "\u{0915}\n\u{0937}"),
    ("structure", "ASCII digit", "\u{0915}7"),
    ("structure", "ASCII underscore", "\u{0915}_\u{0937}"),
    ("structure", "emoji", "\u{0915}\u{1F600}"),
    (
        "structure",
        "combining diacritic from another script",
        "\u{0915}\u{0301}",
    ),
];

/// The one legal joiner position, plus ordinary names — these must all pass, or
/// the rules above are simply "reject everything" and prove nothing.
const MUST_ACCEPT: &[(&str, &str)] = &[
    ("plain consonant", "\u{0915}"),
    ("conjunct क्ष", "\u{0915}\u{094D}\u{0937}"),
    (
        "ZWNJ between virāma and consonant (suppress conjunct)",
        "\u{0915}\u{094D}\u{200C}\u{0937}",
    ),
    (
        "ZWJ between virāma and consonant (force half-form)",
        "\u{0915}\u{094D}\u{200D}\u{0937}",
    ),
    (
        "name with digit suffix कोष्ठ०",
        "\u{0915}\u{094B}\u{0937}\u{094D}\u{0920}\u{0966}",
    ),
    ("योगः", "\u{092F}\u{094B}\u{0917}\u{0903}"),
    ("स्मृति", "\u{0938}\u{094D}\u{092E}\u{0943}\u{0924}\u{093F}"),
    (
        "प्रत्यागमनम्",
        "\u{092A}\u{094D}\u{0930}\u{0924}\u{094D}\u{092F}\u{093E}\u{0917}\u{092E}\u{0928}\u{092E}\u{094D}",
    ),
];

#[test]
fn every_adversarial_vector_is_rejected() {
    let mut holes = Vec::new();
    for &(class, desc, input) in ATTACKS {
        if validate(input).is_ok() {
            holes.push(format!(
                "  [{class}] {desc}\n      ACCEPTED: {}",
                input
                    .chars()
                    .map(|c| format!("U+{:04X}", c as u32))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
    }
    assert!(
        holes.is_empty(),
        "{} of {} adversarial vectors were ACCEPTED — each is an attack that now works:\n{}",
        holes.len(),
        ATTACKS.len(),
        holes.join("\n")
    );
    assert!(ATTACKS.len() >= 40, "task A-018 requires >= 40 vectors");
    println!("METRIC adversarial_ident_vectors {}", ATTACKS.len());
}

#[test]
fn legitimate_identifiers_still_pass() {
    for &(desc, input) in MUST_ACCEPT {
        assert!(
            validate(input).is_ok(),
            "false positive on {desc}: {:?}",
            validate(input)
        );
    }
}

/// Attacks must be classified correctly, not merely rejected. A bidi override
/// reported as "bad continue" tells a reviewer nothing; reported as
/// `FormatControl` it names the attack.
#[test]
fn attacks_are_diagnosed_by_class() {
    for &(class, desc, input) in ATTACKS {
        let Err(e) = validate(input) else { continue };
        let ok = match class {
            "bidi" => matches!(e, IdentError::FormatControl { .. }),
            "invisible" => matches!(
                e,
                IdentError::MisplacedJoiner { .. } | IdentError::FormatControl { .. }
            ),
            "homograph" => matches!(
                e,
                IdentError::MixedScript { .. } | IdentError::BadContinue { .. }
            ),
            _ => true,
        };
        assert!(
            ok,
            "{desc} rejected as {e:?}, which mislabels a '{class}' attack"
        );
    }
}

/// Byte offsets must point into the input so a diagnostic can slice it.
#[test]
fn every_error_offset_is_a_valid_boundary() {
    for &(_, desc, input) in ATTACKS {
        if let Err(e) = validate(input)
            && let Some(at) = e.offset()
        {
            assert!(at <= input.len(), "{desc}: offset {at} past end");
            assert!(
                input.is_char_boundary(at),
                "{desc}: offset {at} is not a char boundary"
            );
        }
    }
}
