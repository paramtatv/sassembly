//! Task `A-016` — hand-written Devanagari akṣara vectors.
//!
//! # Why this exists alongside `GraphemeBreakTest.txt`
//!
//! `GraphemeBreakTest.txt` is **generated from the same property tables** that
//! [`sanskrit_text::segment`] reads. It therefore cannot catch a table that is
//! wrong in a way the test file shares, nor a `ucdgen` bug that corrupts both
//! consistently. These vectors are derived from the Devanagari writing system
//! itself (doc 01 §2.2) rather than from the UCD, so they are independent
//! evidence.
//!
//! Every expectation below is what a Sanskrit reader would count as one अक्षर —
//! one terminal cell, one cursor step, one backspace (doc 01 D-01-A, doc 04 §2).

use sanskrit_text::aksharas_count;

/// `(description, text, expected akṣara count)`
const VECTORS: &[(&str, &str, usize)] = &[
    // ---- independent letters -------------------------------------------------
    ("single consonant क", "\u{0915}", 1),
    ("independent vowel अ", "\u{0905}", 1),
    (
        "three independent vowels आ इ ई",
        "\u{0906}\u{0907}\u{0908}",
        3,
    ),
    ("three bare consonants क ख ग", "\u{0915}\u{0916}\u{0917}", 3),
    // ---- mātrās: stored after the base, sometimes drawn before it ------------
    ("का  A-matra", "\u{0915}\u{093E}", 1),
    (
        "कि  I-matra — drawn BEFORE the base, stored after",
        "\u{0915}\u{093F}",
        1,
    ),
    ("की  II-matra", "\u{0915}\u{0940}", 1),
    ("कु  U-matra, below base", "\u{0915}\u{0941}", 1),
    ("कू  UU-matra, below base", "\u{0915}\u{0942}", 1),
    ("कृ  vocalic R", "\u{0915}\u{0943}", 1),
    ("कॄ  vocalic RR", "\u{0915}\u{0944}", 1),
    ("के  E-matra, above base", "\u{0915}\u{0947}", 1),
    ("कै  AI-matra", "\u{0915}\u{0948}", 1),
    ("को  O-matra, spans both sides", "\u{0915}\u{094B}", 1),
    ("कौ  AU-matra", "\u{0915}\u{094C}", 1),
    // ---- the three signs ----------------------------------------------------
    ("कं  anusvāra", "\u{0915}\u{0902}", 1),
    (
        "कः  visarga (SpacingMark, still one cell)",
        "\u{0915}\u{0903}",
        1,
    ),
    ("कँ  candrabindu", "\u{0915}\u{0901}", 1),
    // ---- nukta: both spellings are one akṣara (doc 01 D-01-B) ---------------
    ("क़  precomposed U+0958", "\u{0958}", 1),
    ("क़  decomposed क + nukta", "\u{0915}\u{093C}", 1),
    ("ज़  decomposed ज + nukta", "\u{091C}\u{093C}", 1),
    // ---- conjuncts: GB9c, the rule doc 01 D-01-A depends on ------------------
    ("क्ष  ka + virāma + ṣa", "\u{0915}\u{094D}\u{0937}", 1),
    ("त्र  ta + virāma + ra", "\u{0924}\u{094D}\u{0930}", 1),
    ("ज्ञ  ja + virāma + ña", "\u{091C}\u{094D}\u{091E}", 1),
    ("श्र  śa + virāma + ra", "\u{0936}\u{094D}\u{0930}", 1),
    ("द्व  da + virāma + va", "\u{0926}\u{094D}\u{0935}", 1),
    (
        "ट्ट  ṭa + virāma + ṭa (stacked)",
        "\u{091F}\u{094D}\u{091F}",
        1,
    ),
    ("ङ्क  ṅa + virāma + ka", "\u{0919}\u{094D}\u{0915}", 1),
    ("ह्म  ha + virāma + ma", "\u{0939}\u{094D}\u{092E}", 1),
    ("द्ध  da + virāma + dha", "\u{0926}\u{094D}\u{0927}", 1),
    ("न्न  na + virāma + na", "\u{0928}\u{094D}\u{0928}", 1),
    // ---- multi-level conjunct stacks ----------------------------------------
    (
        "क्ष्ण  three consonants, 5 codepoints, ONE akṣara",
        "\u{0915}\u{094D}\u{0937}\u{094D}\u{0923}",
        1,
    ),
    (
        "त्त्व  three-consonant stack",
        "\u{0924}\u{094D}\u{0924}\u{094D}\u{0935}",
        1,
    ),
    (
        "ङ्क्ष  three-consonant stack",
        "\u{0919}\u{094D}\u{0915}\u{094D}\u{0937}",
        1,
    ),
    // ---- reph: र + virāma renders ABOVE and AFTER the syllable ---------------
    ("र्क  reph over ka", "\u{0930}\u{094D}\u{0915}", 1),
    ("र्म  reph over ma", "\u{0930}\u{094D}\u{092E}", 1),
    // ---- conjunct plus mātrā ------------------------------------------------
    (
        "क्षि  conjunct + pre-base I-matra",
        "\u{0915}\u{094D}\u{0937}\u{093F}",
        1,
    ),
    (
        "क्षा  conjunct + A-matra",
        "\u{0915}\u{094D}\u{0937}\u{093E}",
        1,
    ),
    (
        "क्ष्णे  stacked conjunct + E-matra",
        "\u{0915}\u{094D}\u{0937}\u{094D}\u{0923}\u{0947}",
        1,
    ),
    (
        "र्क्ष  reph over a conjunct",
        "\u{0930}\u{094D}\u{0915}\u{094D}\u{0937}",
        1,
    ),
    // ---- halanta-final (dead consonant at word end) -------------------------
    ("क्  bare virāma, as in सस् ", "\u{0915}\u{094D}", 1),
    ("सस्  ends in halanta", "\u{0938}\u{0938}\u{094D}", 2),
    // ---- ZWJ / ZWNJ: the typographic intent IS the cluster boundary ---------
    // ZWNJ is InCB=None, so it RESETS the conjunct state: the reader sees two
    // separate letters, and gets two akṣaras. ZWJ is InCB=Extend, so the
    // conjunct survives as one. Unicode encodes the scribe's intent directly in
    // the segmentation, which is a property worth pinning down.
    (
        "क् + ZWNJ + ष  — conjunct SUPPRESSED, two units",
        "\u{0915}\u{094D}\u{200C}\u{0937}",
        2,
    ),
    (
        "क् + ZWJ + ष  — conjunct JOINED, one unit",
        "\u{0915}\u{094D}\u{200D}\u{0937}",
        1,
    ),
    // ---- Vedic accents (combining, ccc != 0) --------------------------------
    ("क + udātta U+0951", "\u{0915}\u{0951}", 1),
    (
        "क्ष + anudātta U+0952",
        "\u{0915}\u{094D}\u{0937}\u{0952}",
        1,
    ),
    // ---- doc 15 §3.1 signs: each stands alone -------------------------------
    ("।  danda", "\u{0964}", 1),
    ("॥  double danda", "\u{0965}", 1),
    ("॰  abbreviation sign", "\u{0970}", 1),
    ("ॱ  high spacing dot", "\u{0971}", 1),
    ("ऽ  avagraha", "\u{093D}", 1),
    ("ॐ  om", "\u{0950}", 1),
    (
        "क । ख  — sign does not absorb neighbours",
        "\u{0915}\u{0964}\u{0916}",
        3,
    ),
    // ---- Devanagari digits are separate akṣaras -----------------------------
    ("०१२  three digits", "\u{0966}\u{0967}\u{0968}", 3),
    ("१७२९", "\u{0967}\u{096D}\u{0968}\u{096F}", 4),
    // ---- real words, from the purpose.md invocation (doc 01 §5) -------------
    ("परम  pa-ra-ma", "\u{092A}\u{0930}\u{092E}", 3),
    ("नमः  na + maḥ", "\u{0928}\u{092E}\u{0903}", 2),
    (
        "तत्वयाय  ta-tva-yā-ya",
        "\u{0924}\u{0924}\u{094D}\u{0935}\u{092F}\u{093E}\u{092F}",
        4,
    ),
    (
        "गुरुभ्यो  gu-ru-bhyo",
        "\u{0917}\u{0941}\u{0930}\u{0941}\u{092D}\u{094D}\u{092F}\u{094B}",
        3,
    ),
    (
        "नारायणाय  nā-rā-ya-ṇā-ya",
        "\u{0928}\u{093E}\u{0930}\u{093E}\u{092F}\u{0923}\u{093E}\u{092F}",
        5,
    ),
    (
        "संस्कृतम्  saṃ-skṛ-ta-m",
        "\u{0938}\u{0902}\u{0938}\u{094D}\u{0915}\u{0943}\u{0924}\u{092E}\u{094D}",
        4,
    ),
];

#[test]
fn devanagari_akshara_vectors() {
    let mut failures = Vec::new();

    for &(desc, text, want) in VECTORS {
        let got = aksharas_count(text);
        if got != want {
            let split: Vec<String> = sanskrit_text::aksharas(text)
                .map(|a| {
                    a.chars()
                        .map(|c| format!("{:04X}", c as u32))
                        .collect::<Vec<_>>()
                        .join("+")
                })
                .collect();
            failures.push(format!(
                "  {desc}\n    want {want} akṣara(s), got {got}: [{}]",
                split.join(" | ")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} Devanagari vectors failed:\n{}",
        failures.len(),
        VECTORS.len(),
        failures.join("\n")
    );
    assert!(VECTORS.len() >= 50, "task A-016 requires >= 50 vectors");
    println!("METRIC devanagari_hand_vectors {}", VECTORS.len());
}

/// Segmentation must never split or duplicate text, whatever the input.
#[test]
fn vectors_are_losslessly_segmented() {
    for &(desc, text, _) in VECTORS {
        let rebuilt: String = sanskrit_text::aksharas(text).collect();
        assert_eq!(rebuilt, text, "lossy segmentation: {desc}");
        assert!(
            sanskrit_text::aksharas(text).all(|a| !a.is_empty()),
            "empty akṣara: {desc}"
        );
    }
}

/// Normalization must not change how many akṣaras a string has. If it did, the
/// NFC gate at the start of the toolchain would silently move terminal columns.
#[test]
fn nfc_preserves_akshara_count() {
    for &(desc, text, want) in VECTORS {
        let normalized = sanskrit_text::nfc(text);
        assert_eq!(
            aksharas_count(&normalized),
            want,
            "NFC changed the akṣara count of {desc}"
        );
    }
}
