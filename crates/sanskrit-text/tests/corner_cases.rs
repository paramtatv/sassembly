//! Corner cases across the whole text kernel.
//!
//! The conformance suites prove the *specified* behaviour. This file probes the
//! edges the specifications do not talk about: empty input, block boundaries,
//! saturation, exactly-full buffers, idempotence, and the interactions between
//! modules that each module's own tests cannot see.
//!
//! Nothing here may panic. A panic in the text kernel is a panic in the lexer,
//! the terminal line discipline and the kernel's own error path.

use sanskrit_text::{
    aksharas, aksharas_count, ccc, is_devanagari, is_nfc, is_numeral, last_akshara_start, nfc, nfd,
    numeral, repertoire_check, repertoire_clean, slp1, validate_identifier,
};

// ---------------------------------------------------------------- empty input

#[test]
fn every_entry_point_accepts_empty_input() {
    assert_eq!(nfc(""), "");
    assert_eq!(nfd(""), "");
    assert!(is_nfc(""));
    assert_eq!(aksharas_count(""), 0);
    assert_eq!(aksharas("").next(), None);
    assert_eq!(last_akshara_start(""), None);
    assert!(repertoire_clean(""));
    assert!(repertoire_check("").is_empty());
    assert!(validate_identifier("").is_err());
    assert!(!is_numeral(""));

    let mut s = String::new();
    assert!(slp1::encode_into("", &mut s).is_ok());
    assert_eq!(s, "");
    let mut d = String::new();
    assert!(slp1::decode_into("", &mut d).is_ok());
    assert_eq!(d, "");
}

// ------------------------------------------------------- block boundaries

/// The exact first and last codepoint of every block the repertoire admits, and
/// the ones immediately outside. Off-by-one here would silently admit or reject
/// a whole script.
#[test]
fn repertoire_block_edges_are_exact() {
    for (cp, want, what) in [
        (0x08FF, false, "just below Devanagari"),
        (0x0900, true, "Devanagari first"),
        (0x097F, true, "Devanagari last"),
        (0x0980, false, "just above Devanagari (Bengali)"),
        (0x1CCF, false, "just below Vedic Extensions"),
        (0x1CD0, true, "Vedic Extensions first"),
        (0x1CFF, true, "Vedic Extensions last"),
        (0x1D00, false, "just above Vedic Extensions"),
        (0xA8DF, false, "just below Devanagari Extended"),
        (0xA8E0, true, "Devanagari Extended first"),
        (0xA8FF, true, "Devanagari Extended last"),
        (0xA900, false, "just above Devanagari Extended"),
        (0x11AFF, false, "just below Devanagari Extended-A"),
        (0x11B00, true, "Devanagari Extended-A first"),
        (0x11B5F, true, "Devanagari Extended-A last"),
        (0x11B60, false, "just above Devanagari Extended-A"),
    ] {
        let ch = char::from_u32(cp).unwrap();
        assert_eq!(
            sanskrit_text::in_repertoire(ch),
            want,
            "U+{cp:04X} ({what})"
        );
    }
}

/// Identifiers stop at U+097F even though `Script=Devanagari` continues, because
/// no font renders the extended blocks (ADR-0003).
#[test]
fn identifier_block_edge_is_tighter_than_the_repertoire() {
    // U+A8F2 is Script=Devanagari and XID_Continue, and in the repertoire...
    let ext = char::from_u32(0xA8F2).unwrap();
    assert!(is_devanagari(ext));
    assert!(sanskrit_text::in_repertoire(ext));
    // ...but must not reach an identifier.
    let s = format!("\u{0915}{ext}");
    assert!(
        validate_identifier(&s).is_err(),
        "extended block leaked into an identifier"
    );
}

// ------------------------------------------------------------ normalization

#[test]
fn normalization_is_idempotent() {
    for s in [
        "\u{0958}",                 // nukta, precomposed
        "\u{0915}\u{093C}",         // nukta, decomposed
        "\u{0041}\u{0307}\u{0323}", // reorderable marks
        "\u{AC01}",                 // Hangul
        "\u{1E0A}\u{0323}",
        "\u{0915}\u{094D}\u{0937}",
    ] {
        let once = nfc(s);
        assert_eq!(nfc(&once), once, "NFC not idempotent on {s:?}");
        let d = nfd(s);
        assert_eq!(nfd(&d), d, "NFD not idempotent on {s:?}");
        // NFC(NFD(x)) == NFC(x)
        assert_eq!(nfc(&d), once, "NFC∘NFD != NFC on {s:?}");
    }
}

/// Hangul is algorithmic, so its arithmetic must be exact at the ends of the
/// syllable block — one off and 11 172 characters decompose wrongly.
#[test]
fn hangul_range_edges() {
    for (cp, decomposes) in [
        (0xABFF, false), // just below SBase
        (0xAC00, true),  // first syllable
        (0xD7A3, true),  // last syllable
        (0xD7A4, false), // just above
    ] {
        let s = char::from_u32(cp).unwrap().to_string();
        let d = nfd(&s);
        assert_eq!(
            d != s,
            decomposes,
            "U+{cp:04X} decomposition behaviour is wrong"
        );
        if decomposes {
            assert_eq!(nfc(&d), s, "U+{cp:04X} does not recompose");
        }
    }
}

#[test]
fn long_combining_sequences_do_not_break_ordering() {
    // 40 combining marks on one base, in descending ccc order: canonical
    // ordering must sort them all and stay one akṣara.
    let mut s = String::from("\u{0915}");
    for cp in (0x0300u32..0x0328).rev() {
        s.push(char::from_u32(cp).unwrap());
    }
    let d = nfd(&s);
    let classes: Vec<u8> = d.chars().skip(1).map(ccc).collect();
    let mut sorted = classes.clone();
    sorted.sort_unstable();
    assert_eq!(
        classes, sorted,
        "canonical ordering failed on a long sequence"
    );
    assert_eq!(
        aksharas_count(&d),
        1,
        "long mark sequence split into pieces"
    );
}

// -------------------------------------------------------------- segmentation

/// Every ordered pair of grapheme-break classes, exercised through real
/// codepoints. 196 combinations, none of which may panic or lose bytes.
#[test]
fn all_break_class_pairs_are_total_and_lossless() {
    const SAMPLES: &[char] = &[
        'x',         // Other
        '\r',        // CR
        '\n',        // LF
        '\u{0001}',  // Control
        '\u{0300}',  // Extend
        '\u{200D}',  // ZWJ
        '\u{1F1EE}', // Regional_Indicator
        '\u{0600}',  // Prepend
        '\u{0903}',  // SpacingMark
        '\u{1100}',  // L
        '\u{1161}',  // V
        '\u{11A8}',  // T
        '\u{AC00}',  // LV
        '\u{AC01}',  // LVT
        '\u{0915}',  // Devanagari consonant (InCB=Consonant)
        '\u{094D}',  // virama (InCB=Linker)
        '\u{1F600}', // Extended_Pictographic
    ];
    for &a in SAMPLES {
        for &b in SAMPLES {
            let s: String = [a, b].iter().collect();
            let total: usize = aksharas(&s).map(str::len).sum();
            assert_eq!(total, s.len(), "lossy on {:04X}+{:04X}", a as u32, b as u32);
            assert!(aksharas(&s).all(|x| !x.is_empty()));
            let n = aksharas_count(&s);
            assert!((1..=2).contains(&n), "{n} clusters from 2 chars");
        }
    }
}

/// The one-sided form. `expected` below is always `Some`, so this cannot catch a
/// spurious `None` — it constrains the value and not the shape.
///
/// The two-sided form is `src/props.rs`, where `None => assert!(s.is_empty())`
/// makes both outcomes reachable, and it runs on every gate through
/// `tests/fuzz_corpus.rs` over far more input than the five strings here. The
/// `None` case itself is asserted directly at line 25 above and at
/// `src/segment.rs:299`.
///
/// Recorded because three separate proposals to strengthen or widen this were
/// raised and withdrawn: the coverage was never in this file, and nothing here
/// says so (`W-110`). Widening the list below buys nothing — the fold it checks
/// is branchless.
#[test]
fn last_akshara_start_agrees_with_iteration() {
    for s in [
        "\u{0915}",
        "\u{0915}\u{0915}\u{094D}\u{0937}",
        "\r\n",
        "\u{1F1EE}\u{1F1F3}\u{1F1EE}\u{1F1F3}",
        "x",
    ] {
        let via_iter = aksharas(s).map(str::len).collect::<Vec<_>>();
        let expected = s.len() - via_iter.last().copied().unwrap_or(0);
        assert_eq!(last_akshara_start(s), Some(expected), "on {s:?}");
    }
}

// ------------------------------------------------------------------ numerals

/// `W-075` reversed this test's verdict, and the reversal is the row.
///
/// It used to be `numeral_saturates_rather_than_wrapping`, and saturating was
/// held to be the safe choice against wrapping. It is not safe: both emit a
/// number other than the one written, and saturation is the more convincing of
/// the two because the result looks deliberate. The FNV-1a basis came out as
/// 0x7fffffffffffffff and nothing said so.
#[test]
fn numeral_too_large_is_refused_rather_than_saturated() {
    // 30 Devanagari nines: far past u64.
    let big: String = core::iter::repeat_n('\u{096F}', 30).collect();
    assert_eq!(numeral::value(&big), Err(numeral::NumeralError::TooLarge));
    assert_eq!(numeral::bits(&big), Err(numeral::NumeralError::TooLarge));
}

#[test]
fn numeral_boundary_values() {
    let devanagari = |n: u128| -> String {
        format!("{n}")
            .chars()
            .map(|c| char::from_u32(0x0966 + c.to_digit(10).unwrap()).unwrap())
            .collect()
    };
    // u64::MAX itself is representable and one more is not. That boundary is
    // where an off-by-one in the overflow check would live, so both sides of it
    // are named rather than just the safe one.
    assert_eq!(
        numeral::value(&devanagari(u128::from(u64::MAX))),
        Ok(u64::MAX)
    );
    assert_eq!(
        numeral::value(&devanagari(u128::from(u64::MAX) + 1)),
        Err(numeral::NumeralError::TooLarge)
    );
    assert_eq!(numeral::value("\u{0966}"), Ok(0));

    // And the negative side reaches ONE FURTHER, which is why `bits` negates in
    // u64 rather than through i64. i64::MIN was unwritable before `W-075`: it
    // came back as i64::MIN + 1, silently.
    let min = format!("{}{}", numeral::NEGATIVE, devanagari(1u128 << 63));
    assert_eq!(
        numeral::bits(&min),
        Ok(i64::MIN as u64),
        "ऋण followed by 2^63 is i64::MIN exactly"
    );
    let past = format!("{}{}", numeral::NEGATIVE, devanagari((1u128 << 63) + 1));
    assert_eq!(
        numeral::bits(&past),
        Err(numeral::NumeralError::TooLarge),
        "and one further does not fit"
    );
}

#[test]
fn radix_prefix_alone_and_almost_alone() {
    // ०षोड् with nothing after it, and with one digit.
    let hex_prefix = "\u{0966}\u{0937}\u{094B}\u{0921}\u{094D}";
    assert!(!is_numeral(hex_prefix));
    assert!(is_numeral(&format!("{hex_prefix}\u{0966}")));
    // A bare ० is decimal zero, not a truncated prefix.
    assert!(is_numeral("\u{0966}"));
}

#[test]
fn hex_digit_letters_are_inert_outside_hex() {
    for (cp, hex_val) in [(0x0905u32, 10u32), (0x090A, 15)] {
        let ch = char::from_u32(cp).unwrap();
        assert_eq!(
            numeral::digit_value(ch, numeral::Radix::Hexadecimal),
            Some(hex_val)
        );
        assert_eq!(numeral::digit_value(ch, numeral::Radix::Decimal), None);
        assert_eq!(numeral::digit_value(ch, numeral::Radix::Octal), None);
        // and they remain perfectly ordinary identifier characters
        assert!(validate_identifier(&ch.to_string()).is_ok());
    }
}

// ---------------------------------------------------------------------- SLP1

#[test]
fn fixed_buffer_boundary_is_exact() {
    // "kza" is 3 bytes. A 3-byte buffer must succeed; 2 must fail cleanly.
    let deva = "\u{0915}\u{094D}\u{0937}";
    let mut exact = [0u8; 3];
    let mut b = slp1::Buf::new(&mut exact);
    assert!(slp1::encode_into(deva, &mut b).is_ok());
    assert_eq!(b.as_str(), "kza");
    assert_eq!(b.len(), 3);

    let mut short = [0u8; 2];
    let mut b2 = slp1::Buf::new(&mut short);
    assert_eq!(
        slp1::encode_into(deva, &mut b2),
        Err(slp1::Slp1Error::OutputFull)
    );

    let mut zero = [0u8; 0];
    let mut b3 = slp1::Buf::new(&mut zero);
    assert!(b3.is_empty());
    assert!(slp1::encode_into(deva, &mut b3).is_err());
}

#[test]
fn slp1_rejects_rather_than_truncating_unknown_input() {
    // Latin into the decoder is fine (it is SLP1); Devanagari is not.
    let mut o = String::new();
    assert!(slp1::decode_into("\u{0915}", &mut o).is_err());
    // and an unmapped Devanagari character in the encoder
    let mut p = String::new();
    assert!(slp1::encode_into("\u{A8FC}", &mut p).is_err());
}

// ------------------------------------------------------- cross-module invariants

/// NFC must never change how many akṣaras a string has, or the NFC gate at the
/// front of the toolchain would silently move terminal columns.
#[test]
fn normalization_preserves_akshara_count_on_hard_cases() {
    for s in [
        "\u{0958}\u{0959}\u{095A}",         // three precomposed nuktas
        "\u{0915}\u{093C}\u{094D}\u{0937}", // nukta inside a conjunct
        "\u{0915}\u{0300}\u{0301}\u{0302}", // foreign marks on a consonant
        "\u{AC01}\u{0300}",
    ] {
        assert_eq!(
            aksharas_count(&nfc(s)),
            aksharas_count(s),
            "NFC changed the akṣara count of {s:?}"
        );
    }
}

/// Anything the repertoire gate accepts must survive segmentation and, if it is
/// word-shaped, produce a definite identifier verdict rather than a panic.
#[test]
fn repertoire_clean_input_never_panics_downstream() {
    let mut s = String::new();
    for cp in 0x0900u32..0x0980 {
        if let Some(c) = char::from_u32(cp) {
            s.push(c);
        }
    }
    let total: usize = aksharas(&s).map(str::len).sum();
    assert_eq!(total, s.len());
    let _ = nfc(&s);
    let _ = validate_identifier(&s);
    let mut o = String::new();
    let _ = slp1::encode_into(&s, &mut o);
}

/// Adversarial and malformed input from anywhere in Unicode must produce a
/// verdict, never a panic. This is the property a fuzzer would check; doing it
/// deterministically keeps it in the normal test run.
#[test]
fn no_entry_point_panics_on_arbitrary_unicode() {
    let mut rng: u64 = 0x005A_4E53_4F53;
    let mut next = move || {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (rng >> 33) as u32
    };

    for _ in 0..20_000 {
        let mut s = String::new();
        for _ in 0..(next() % 12) {
            // Bias toward Devanagari, but admit anything.
            let cp = match next() % 4 {
                0 => 0x0900 + next() % 0x80,
                1 => 0x0966 + next() % 10,
                2 => next() % 0x11_0000,
                _ => next() % 0x300,
            };
            if let Some(c) = char::from_u32(cp) {
                s.push(c);
            }
        }
        let _ = nfc(&s);
        let _ = nfd(&s);
        let _ = is_nfc(&s);
        let _ = aksharas_count(&s);
        let _ = last_akshara_start(&s);
        let _ = validate_identifier(&s);
        let _ = is_numeral(&s);
        let _ = repertoire_check(&s);
        let mut o = String::new();
        let _ = slp1::encode_into(&s, &mut o);
        let mut d = String::new();
        let _ = slp1::decode_into(&s, &mut d);

        // and segmentation stays lossless whatever the input
        let total: usize = aksharas(&s).map(str::len).sum();
        assert_eq!(total, s.len(), "lossy segmentation on {s:?}");
    }
}

/// Large inputs must not be quadratic. 1 MB of Devanagari through every pass
/// should be well under a second; a blow-up here would show as a timeout.
#[test]
fn large_input_stays_linear() {
    let unit = "\u{0915}\u{094D}\u{0937}\u{0940} \u{0967}\u{096D} ";
    const REPS: usize = 50_000;
    let big: String = unit.repeat(REPS); // ~1 MB
    assert!(big.len() > 900_000, "{} bytes", big.len());

    // क्षी | ␠ | १ | ७ | ␠  — five akṣaras per unit
    assert_eq!(aksharas_count(unit), 5);
    assert_eq!(aksharas_count(&big), 5 * REPS);
    assert!(repertoire_clean(&big));
    assert!(is_nfc(&big));
    let mut o = String::new();
    slp1::encode_into(&big, &mut o).unwrap();
    let mut back = String::new();
    slp1::decode_into(&o, &mut back).unwrap();
    assert_eq!(back, big);
}
