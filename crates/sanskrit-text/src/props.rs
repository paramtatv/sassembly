//! Invariants the text kernel must hold for **any** input.
//!
//! These are the bodies of the fuzz targets in `fuzz/fuzz_targets/`, kept in the
//! library rather than in the targets so that the same assertions run three
//! ways:
//!
//! 1. under `cargo fuzz` (coverage-guided, nightly),
//! 2. as a corpus replay on stable (`tests/fuzz_corpus.rs`) — so a crash found
//!    once is checked forever, on every machine, without nightly,
//! 3. against generated input on stable (`tests/fuzz_corpus.rs`,
//!    `arbitrary_bytes_are_handled`) — every byte, every pair, and Devanagari
//!    lead bytes with an arbitrary tail.
//!
//! Ways 2 and 3 are both in `fuzz_corpus.rs`, which is how way 3 came to be
//! addressed to `tests/corner_cases.rs` — a file that never mentions this
//! module. The clause was true and its address was not, and a reader who
//! grepped only the named file would have found nothing and deleted a true
//! fact about real coverage (`W-110`).
//!
//! A fuzzer that finds a counterexample here has found a real defect: every
//! function below is a property the rest of the system relies on. The lexer, the
//! terminal line discipline and the kernel's own panic path all sit downstream.
//!
//! Each takes raw bytes, because that is what a fuzzer produces and what a file
//! actually contains. Non-UTF-8 input is not an error to report — it is simply
//! not this layer's problem, so it returns early.

extern crate alloc;

use alloc::string::String;

use crate::ime::{Ime, Key};
use crate::{
    aksharas, aksharas_count, is_nfc, is_numeral, last_akshara_start, nfc, nfd, numeral,
    repertoire, slp1, validate_identifier,
};

fn as_str(data: &[u8]) -> Option<&str> {
    core::str::from_utf8(data).ok()
}

/// Segmentation is **total and lossless**: the akṣaras of a string concatenate
/// back to exactly that string, and none is empty.
///
/// If this ever fails, the terminal drops or duplicates text.
///
/// # Panics
/// On any violation.
pub fn segmentation_is_lossless(data: &[u8]) {
    let Some(s) = as_str(data) else { return };

    let mut total = 0usize;
    let mut rebuilt = String::with_capacity(s.len());
    for a in aksharas(s) {
        assert!(!a.is_empty(), "empty akṣara in {s:?}");
        total += a.len();
        rebuilt.push_str(a);
    }
    assert_eq!(total, s.len(), "lossy segmentation of {s:?}");
    assert_eq!(rebuilt, s, "segmentation did not reconstruct {s:?}");

    // The last-akṣara offset must agree with iteration — the terminal's
    // backspace depends on it.
    match last_akshara_start(s) {
        None => assert!(s.is_empty()),
        Some(at) => {
            assert!(
                s.is_char_boundary(at),
                "backspace offset {at} splits a char"
            );
            // Forward scan: `Aksharas` is not double-ended, because finding a
            // cluster boundary backwards needs its own algorithm rather than a
            // reversed walk. That makes both this and `last_akshara_start` O(n),
            // which the terminal will want to revisit for long-line backspace.
            let last = aksharas(s).last().map(str::len).unwrap_or(0);
            assert_eq!(
                at,
                s.len() - last,
                "backspace offset disagrees with iteration"
            );
        }
    }
}

/// Normalization is **idempotent** and **preserves akṣara count**.
///
/// The second half is the one that matters operationally: NFC runs at pass 0 of
/// the toolchain, so if it moved cluster boundaries it would silently shift
/// every terminal column downstream of it.
///
/// # Panics
/// On any violation.
pub fn normalization_is_stable(data: &[u8]) {
    let Some(s) = as_str(data) else { return };

    let c = nfc(s);
    assert_eq!(nfc(&c), c, "NFC is not idempotent on {s:?}");
    assert!(
        is_nfc(&c),
        "nfc() produced something is_nfc() rejects: {s:?}"
    );

    let d = nfd(s);
    assert_eq!(nfd(&d), d, "NFD is not idempotent on {s:?}");
    assert_eq!(nfc(&d), c, "NFC∘NFD != NFC on {s:?}");

    assert_eq!(
        aksharas_count(&c),
        aksharas_count(s),
        "NFC changed the akṣara count of {s:?}"
    );
}

/// SLP1 is a **bijection on its domain**: whatever encodes must decode back to
/// exactly the input, and whatever decodes must re-encode to exactly the SLP1.
///
/// Anything outside the domain must be *rejected*, never silently mangled —
/// lossy transliteration is how two identifiers become one symbol.
///
/// # Panics
/// On any violation.
pub fn slp1_round_trips(data: &[u8]) {
    let Some(s) = as_str(data) else { return };

    let mut encoded = String::new();
    if slp1::encode_into(s, &mut encoded).is_ok() {
        let mut back = String::new();
        slp1::decode_into(&encoded, &mut back).unwrap_or_else(|e| {
            panic!("encoded {s:?} to {encoded:?} which will not decode: {e:?}")
        });
        assert_eq!(
            back, s,
            "round trip lost information: {s:?} -> {encoded:?} -> {back:?}"
        );
    }

    let mut decoded = String::new();
    if slp1::decode_into(s, &mut decoded).is_ok() {
        let mut re = String::new();
        slp1::encode_into(&decoded, &mut re)
            .unwrap_or_else(|e| panic!("decoded {s:?} to something unencodable: {e:?}"));
        assert_eq!(
            re, s,
            "SLP1 spelling is not stable: {s:?} -> {decoded:?} -> {re:?}"
        );
    }
}

/// The identifier verdict is **deterministic**, and anything accepted is
/// constrained: every character is in the repertoire, and the offset of a
/// rejection is a real char boundary a diagnostic can slice at.
///
/// # Panics
/// On any violation.
pub fn identifier_verdict_is_sound(data: &[u8]) {
    let Some(s) = as_str(data) else { return };

    let first = validate_identifier(s);
    assert_eq!(
        first,
        validate_identifier(s),
        "verdict is not deterministic"
    );

    match first {
        Ok(()) => {
            assert!(!s.is_empty(), "empty string accepted as an identifier");
            for ch in s.chars() {
                assert!(
                    repertoire::in_repertoire(ch),
                    "accepted identifier {s:?} contains U+{:04X}, outside the repertoire",
                    ch as u32
                );
            }
            // An accepted identifier is one script by construction, so it can
            // never also be a numeral.
            assert!(!is_numeral(s), "{s:?} is both an identifier and a numeral");
        }
        Err(e) => {
            if let Some(at) = e.offset() {
                assert!(at <= s.len(), "offset {at} past end of {s:?}");
                assert!(s.is_char_boundary(at), "offset {at} splits a char in {s:?}");
            }
        }
    }
}

/// The repertoire gate agrees with itself, and its reported positions are
/// usable: line and akṣara-column must point at a real place in the file.
///
/// # Panics
/// On any violation.
pub fn repertoire_report_is_consistent(data: &[u8]) {
    let Some(s) = as_str(data) else { return };

    let violations = repertoire::check(s);
    assert_eq!(
        violations.is_empty(),
        repertoire::is_clean(s),
        "check() and is_clean() disagree on {s:?}"
    );

    let lines = s.lines().count().max(1);
    for v in &violations {
        assert!(v.at < s.len(), "violation offset past end");
        assert!(s.is_char_boundary(v.at), "violation offset splits a char");
        assert!(
            !repertoire::in_repertoire(v.ch),
            "clean char reported as a violation"
        );
        assert!(
            v.line >= 1 && v.line <= lines + 1,
            "line {} out of range",
            v.line
        );
        assert!(v.column >= 1, "column must be 1-based");
    }
}

/// Numeral parsing agrees with its own validator, and a parsed value never
/// wraps — overflow saturates, so an attacker-supplied literal cannot produce a
/// small number from a huge one.
///
/// # Panics
/// On any violation.
pub fn numeral_parse_is_sound(data: &[u8]) {
    let Some(s) = as_str(data) else { return };

    let valid = is_numeral(s);
    assert_eq!(
        valid,
        numeral::validate(s).is_ok(),
        "is_numeral disagrees with validate"
    );
    // WELL-FORMED IS NOT THE SAME AS REPRESENTABLE, and this used to say it was
    // (`W-075`). The old assertion was `valid == value(s).is_ok()`, which was
    // already FALSE before that row changed anything: `ऋण१` is a well-formed
    // numeral — `is_numeral` says so, because `validate` strips the sign — and
    // `value` refuses it, because a magnitude has nowhere to put a sign. The
    // property never held. It survived because a fuzzer reaching it has to
    // spell `ऋण` exactly, and nine specific bytes is not something random input
    // finds.
    //
    // So the two are related by WHICH error, not by whether there is one. A
    // reader that answers for a spelling it has no room for is the defect that
    // whole row is about, and this would be an odd place to be loose.
    match numeral::value(s) {
        Ok(_) => assert!(
            valid && !s.starts_with(numeral::NEGATIVE),
            "value accepted {s:?}, which is not an unsigned numeral"
        ),
        Err(numeral::NumeralError::TooLarge | numeral::NumeralError::Signed) => assert!(
            valid,
            "value refused {s:?} for its VALUE, so it must be well-formed"
        ),
        Err(numeral::NumeralError::NoDigits | numeral::NumeralError::BadDigit { .. }) => {
            assert!(!valid, "value refused {s:?} as malformed, but it is not");
        }
    }
    // And the bits reader answers for exactly the well-formed literals whose
    // value 64 bits hold — never for one that is spelled wrong.
    if numeral::bits(s).is_ok() {
        assert!(valid, "bits answered for {s:?}, which is not a numeral");
    }

    if let Ok(radix) = numeral::validate(s) {
        let (_, digits) = numeral::classify(s).expect("validated but unclassifiable");
        assert!(!digits.is_empty(), "validated a numeral with no digits");
        for ch in digits.chars() {
            assert!(
                numeral::digit_value(ch, radix).is_some(),
                "validated {s:?} but {ch:?} is not a digit in {radix:?}"
            );
        }
    }
}

/// A byte, read as a keystroke.
///
/// The mapping `pravesha::keys` makes, minus the escape-sequence state that
/// belongs to the terminal rather than to the engine. Bytes above ASCII are
/// handed over as characters too: the engine does not get to assume its front
/// end filtered anything, and a key it cannot spell must still be survivable.
fn key_for(byte: u8) -> Key {
    match byte {
        0x09 => Key::Complete,
        0x08 | 0x7f => Key::Backspace,
        b'\r' | b'\n' => Key::Enter,
        _ => Key::Char(byte as char),
    }
}

/// Typing is **deterministic**, **never silently loses a keystroke**, and is
/// **always reversible back to nothing**.
///
/// The middle one is the property a typist depends on and cannot check: every
/// press must land somewhere they can see — in the preview if it joined the word
/// being typed, in the committed text if it ended one. A press that produced
/// neither has vanished, and because the preview is the only feedback there is,
/// they would not find out until the line was finished.
///
/// The third is backspace. Keystrokes and codepoints do not correspond — `ka` is
/// two keys for the one codepoint क — so "delete one thing" is only well defined
/// if repeating it always terminates at empty. A sequence that cannot be deleted
/// back out strands whatever is already on the line.
///
/// # Panics
/// On any violation.
pub fn ime_is_total(data: &[u8]) {
    // Re-decoding the pending word on every press is quadratic in the length of
    // one unbroken word, so a fuzzer given a long run of letters would spend its
    // budget inside a single execution. A word longer than this is not a word.
    const KEYS: usize = 512;

    let mut ime = Ime::new();
    let mut replay = Ime::new();

    for &byte in data.iter().take(KEYS) {
        let key = key_for(byte);
        let before = ime.text().len();

        ime.press(key);
        replay.press(key);
        assert_eq!(ime.text(), replay.text(), "typing is not deterministic");

        if matches!(key, Key::Char(_)) {
            assert!(
                !ime.preview().is_empty() || ime.text().len() > before,
                "{key:?} left no trace: the preview is blank and nothing was committed"
            );
        }
        if key == Key::Enter {
            assert!(ime.text().ends_with('\n'), "Enter did not start a line");
            assert!(ime.preview().is_empty(), "Enter left a word pending");
        }
        if key == Key::Backspace {
            assert!(ime.text().len() <= before, "backspace lengthened the text");
        }
    }

    // Completion answers within the limit it was given, and answers the same
    // way twice. The front end draws this list under the cursor on every
    // keystroke, so a limit it does not respect is a menu over the typist's line.
    for limit in [0usize, 1, 8] {
        let offered = ime.candidates(limit);
        assert!(
            offered.len() <= limit,
            "candidates({limit}) offered {}",
            offered.len()
        );
        assert_eq!(
            offered.len(),
            ime.candidates(limit).len(),
            "candidates is not deterministic"
        );
    }

    // Committing keeps what the preview showed, and doing it twice is doing it
    // once. A session ends on Ctrl-C or on end of input, and the word on screen
    // is the last thing anyone typed — dropping it there is the most visible
    // way this could fail and the easiest to not notice in a test.
    let shown = ime.preview();
    ime.commit();
    assert!(
        ime.text().ends_with(shown.as_str()),
        "commit dropped the previewed word {shown:?}"
    );
    assert!(ime.preview().is_empty(), "commit left a word pending");
    let once = String::from(ime.text());
    ime.commit();
    assert_eq!(ime.text(), once, "commit is not idempotent");

    // Backspace gets back to empty. Each press unpresses one key or removes one
    // committed character, so the text's own length bounds how many it can take.
    let budget = ime.text().chars().count() + 1;
    for _ in 0..budget {
        ime.press(Key::Backspace);
    }
    assert!(
        ime.text().is_empty(),
        "{budget} backspaces did not empty {:?}",
        ime.text()
    );
    assert!(ime.preview().is_empty(), "a word survived being deleted");
}

/// Every property at once — the entry point for a single combined fuzz target
/// and for corpus replay.
///
/// # Panics
/// On any violation.
pub fn all(data: &[u8]) {
    segmentation_is_lossless(data);
    normalization_is_stable(data);
    slp1_round_trips(data);
    identifier_verdict_is_sound(data);
    repertoire_report_is_consistent(data);
    numeral_parse_is_sound(data);
    ime_is_total(data);
}
