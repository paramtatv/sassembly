//! Numeric literals (doc 15 §3.3).
//!
//! Devanagari digits `०`–`९` only — ASCII digits are a hard error, not a form to
//! be normalized (doc 15 §3.3 tightened doc 01 D-01-D). Radix is named by a
//! Sanskrit prefix rather than a Latin letter:
//!
//! | Radix | Prefix | From | Example |
//! |---|---|---|---|
//! | 10 | *(none)* | — | `१७२९` |
//! | 2 | `०द्वि` | द्वि, two | `०द्वि१०११०१` |
//! | 8 | `०अष्ट` | अष्ट, eight | `०अष्ट७५५` |
//! | 16 | `०षोड्` | षोडश, sixteen | `०षोड्७अ` |
//!
//! Hex digits 10–15 are `अ आ इ ई उ ऊ`. Those letters are extremely common in
//! identifiers, which is safe only because a literal is lexed as a single token
//! from its `०षोड्` prefix — the digits are never encountered free-standing.

// `sanskrit-text` is `#![no_std]`; `lib.rs` already declares `extern crate
// alloc`. Rendering a number to text allocates, so this module takes the same
// imports the other allocating modules here use (`ime.rs:33-34`). Segmentation
// stays allocation-free regardless (lib.rs:10-12).
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

const BINARY: &str = "\u{0966}\u{0926}\u{094D}\u{0935}\u{093F}"; // ०द्वि
const OCTAL: &str = "\u{0966}\u{0905}\u{0937}\u{094D}\u{091F}"; // ०अष्ट
const HEX: &str = "\u{0966}\u{0937}\u{094B}\u{0921}\u{094D}"; // ०षोड्

/// Radix of a literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Radix {
    /// `०द्वि`
    Binary = 2,
    /// `०अष्ट`
    Octal = 8,
    /// bare digits
    Decimal = 10,
    /// `०षोड्`
    Hexadecimal = 16,
}

/// Why a literal was rejected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumeralError {
    /// Empty, or a prefix with no digits after it.
    NoDigits,
    /// A character that is not a digit in this radix.
    BadDigit {
        /// Byte offset.
        at: usize,
        /// The character.
        ch: char,
        /// The radix in force.
        radix: Radix,
    },
    /// Spelled correctly, and naming a value 64 bits cannot hold — `W-075`.
    ///
    /// Kept apart from [`BadDigit`](Self::BadDigit) because nothing is wrong
    /// with the writing. A reader told "malformed" goes looking for a typo that
    /// is not there; the fault is that the number does not exist here.
    TooLarge,
    /// A `ऋण` reached a reader that has no sign to give it.
    ///
    /// [`value`] answers with a magnitude and has nowhere to put a sign. This
    /// used to be reported as [`NoDigits`](Self::NoDigits), which is false —
    /// `ऋण१` has a digit — and sent the reader to look for one.
    Signed,
}

/// Digit value of `ch` in `radix`, if it is one.
#[must_use]
pub fn digit_value(ch: char, radix: Radix) -> Option<u32> {
    let cp = ch as u32;
    let v = match cp {
        0x0966..=0x096F => cp - 0x0966, // ०–९
        0x0905 => 10,                   // अ
        0x0906 => 11,                   // आ
        0x0907 => 12,                   // इ
        0x0908 => 13,                   // ई
        0x0909 => 14,                   // उ
        0x090A => 15,                   // ऊ
        _ => return None,
    };
    (v < radix as u32).then_some(v)
}

/// Render a signed integer, using this language's own negative marker.
///
/// [`NEGATIVE`] is `ऋण`, and [`split_sign`] already READS that prefix — this is
/// the writer for it, so the two are inverses. A caller that reached for
/// `to_devanagari(n as u64)` on a negative would render a 20-digit number
/// instead, which is why this exists rather than a cast at the call site.
#[must_use]
pub fn to_devanagari_signed(n: i64) -> String {
    let mut s = String::new();
    if n < 0 {
        s.push_str(NEGATIVE);
    }
    s.push_str(&to_devanagari(n.unsigned_abs()));
    s
}

/// Render an unsigned integer in Devanagari digits (U+0966..U+096F).
///
/// The inverse of [`value`] for the decimal case, and the piece this module
/// was missing: it could READ `१२` and could not WRITE it, so every caller
/// that needed a numeral in text fell back to ASCII `format!("{n}")`.
/// `drishya.rs`'s `अङ्कपाठः` did exactly that and said so in its own docs.
///
/// ADR-0023 settles which digits the language writes: Devanagari, everywhere.
///
/// Zero renders as a single `०`, not as the empty string — the loop below runs
/// at least once for that reason, and `zero_is_a_digit_not_an_empty_string`
/// fails if it is ever rewritten as a plain `while n > 0`.
pub fn to_devanagari(mut n: u64) -> String {
    const DIGITS: [char; 10] = ['०', '१', '२', '३', '४', '५', '६', '७', '८', '९'];
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[(n % 10) as usize]);
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.iter().rev().collect()
}

/// Whether `ch` is a Devanagari digit `०`–`९`.
#[must_use]
pub fn is_devanagari_digit(ch: char) -> bool {
    ('\u{0966}'..='\u{096F}').contains(&ch)
}

/// Split a literal into its radix and its digit text.
///
/// The sign prefix — task `B-062`.
///
/// `ऋण` is not coined here. Doc 15 already lists it as the negative operator,
/// and it is Brahmagupta's own word for a negative quantity from the
/// *Brāhmasphuṭasiddhānta* — the first systematic treatment of negative numbers
/// anywhere. Sanskrit had a name for this before it had a symbol, which is
/// convenient for a script that admits no minus sign.
pub const NEGATIVE: &str = "ऋण";

/// Split a leading `ऋण` off a literal.
///
/// The sign sits outside the radix prefix, so `ऋण०षोड्ऊ` is negative fifteen —
/// read in the order it is said.
#[must_use]
pub fn split_sign(token: &str) -> (bool, &str) {
    match token.strip_prefix(NEGATIVE) {
        Some(rest) => (true, rest),
        None => (false, token),
    }
}

/// Returns `None` if the token does not begin like a numeral, so a caller can
/// fall through to identifier lexing.
#[must_use]
pub fn classify(token: &str) -> Option<(Radix, &str)> {
    let (_, token) = split_sign(token);
    for (prefix, radix) in [
        (HEX, Radix::Hexadecimal),
        (BINARY, Radix::Binary),
        (OCTAL, Radix::Octal),
    ] {
        if let Some(rest) = token.strip_prefix(prefix) {
            return Some((radix, rest));
        }
    }
    token
        .chars()
        .next()
        .filter(|c| is_devanagari_digit(*c))
        .map(|_| (Radix::Decimal, token))
}

/// Validate a numeric literal.
///
/// # Errors
/// Returns the first character that is not a digit in the token's radix.
pub fn validate(token: &str) -> Result<Radix, NumeralError> {
    let (_, token) = split_sign(token);
    let Some((radix, digits)) = classify(token) else {
        return Err(NumeralError::NoDigits);
    };
    if digits.is_empty() {
        return Err(NumeralError::NoDigits);
    }
    let offset = token.len() - digits.len();
    for (i, ch) in digits.char_indices() {
        if digit_value(ch, radix).is_none() {
            return Err(NumeralError::BadDigit {
                at: offset + i,
                ch,
                radix,
            });
        }
    }
    Ok(radix)
}

/// Whether `token` is a well-formed numeric literal.
#[must_use]
pub fn is_numeral(token: &str) -> bool {
    validate(token).is_ok()
}

/// The magnitude of a literal, which is what a numeral on its own denotes.
///
/// # What a numeral means, and why too large is refused — `W-075`
///
/// `ऋण` sits OUTSIDE the radix prefix, so a numeral's own grammar has no sign:
/// a numeral is a MAGNITUDE, and `ऋण` is applied to it. The magnitudes this
/// language can write are therefore `०` to 18446744073709551615, and with `ऋण`
/// down to −9223372036854775808 — the two's-complement range of the 64 bits
/// every destination here is made of.
///
/// A magnitude outside that is **refused**. It used to saturate at `u64::MAX`,
/// under a doc comment saying "the lexer is responsible for range-checking
/// against the destination type" — a responsibility no caller discharged. So a
/// literal too large by any amount assembled as a different, plausible number
/// and said nothing, which is the one thing R-02-1 exists to prevent. Widening
/// to `u128` would only move the threshold; wrapping is the same defect with
/// different arithmetic. Refusing is the only answer that keeps what is written
/// and what is emitted the same number.
///
/// # Errors
/// As [`validate`]; [`NumeralError::Signed`] for a `ऋण` this reader cannot
/// express, and [`NumeralError::TooLarge`] past `u64::MAX`.
pub fn value(token: &str) -> Result<u64, NumeralError> {
    let (negative, bare) = split_sign(token);
    if negative {
        return Err(NumeralError::Signed);
    }
    let radix = validate(bare)?;
    let (_, digits) = classify(bare).unwrap();
    let mut n: u64 = 0;
    for ch in digits.chars() {
        let d = digit_value(ch, radix).unwrap();
        n = n
            .checked_mul(radix as u64)
            .and_then(|n| n.checked_add(u64::from(d)))
            .ok_or(NumeralError::TooLarge)?;
    }
    Ok(n)
}

/// The 64 bits a literal denotes, honouring `ऋण` as two's complement.
///
/// # Why this returns bits and not an integer — `W-075`
///
/// This replaced `signed_value`, which returned `i64`, and the return type was
/// the bug. Every caller wrote `as u64` on the result, because what a directive
/// datum and an instruction's immediate field each want is a BIT PATTERN and
/// not an integer. So `०षोड्इआऊ२९इउ४८४२२२३२५` is a perfectly good
/// `॥ अष्टाष्टकाः ॥` operand that is not an `i64` at all, and a reader typed
/// `i64` had to either refuse it — losing half the 64-bit space — or return
/// something that was not what was asked for. It returned `i64::MAX`.
///
/// Naming the bits makes both questions answerable: this one, and "does it fit
/// a signed field", which is the caller's own range check against its own width
/// and was always the caller's to make.
///
/// # Errors
/// As [`value`], and [`NumeralError::TooLarge`] for a `ऋण` magnitude past
/// `2^63` — −9223372036854775808 is the last one 64 bits hold.
pub fn bits(token: &str) -> Result<u64, NumeralError> {
    let (negative, bare) = split_sign(token);
    let magnitude = value(bare)?;
    if !negative {
        return Ok(magnitude);
    }
    // 2^63 is representable and 2^63 + 1 is not: the negative range reaches one
    // further than the positive one. Writing the bound as `1 << 63` rather than
    // `i64::MIN.unsigned_abs()` keeps it in the units this function speaks.
    if magnitude > 1u64 << 63 {
        return Err(NumeralError::TooLarge);
    }
    // `wrapping_neg` on the magnitude IS two's complement, and it is the only
    // form that reaches i64::MIN: negating through `i64` would overflow there,
    // which is how `ऋण` followed by 2^63 used to come out one short.
    Ok(magnitude.wrapping_neg())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal() {
        assert_eq!(
            validate("\u{0967}\u{096D}\u{0968}\u{096F}"),
            Ok(Radix::Decimal)
        );
        assert_eq!(value("\u{0967}\u{096D}\u{0968}\u{096F}"), Ok(1729));
    }

    #[test]
    fn binary_octal_hex() {
        let b = "\u{0966}\u{0926}\u{094D}\u{0935}\u{093F}\u{0967}\u{0966}\u{0967}\u{0967}";
        assert_eq!(validate(b), Ok(Radix::Binary));
        assert_eq!(value(b), Ok(0b1011));

        let o = "\u{0966}\u{0905}\u{0937}\u{094D}\u{091F}\u{096D}\u{096B}\u{096B}";
        assert_eq!(validate(o), Ok(Radix::Octal));
        assert_eq!(value(o), Ok(0o755));

        // ०षोड्७अ = 0x7A
        let h = "\u{0966}\u{0937}\u{094B}\u{0921}\u{094D}\u{096D}\u{0905}";
        assert_eq!(validate(h), Ok(Radix::Hexadecimal));
        assert_eq!(value(h), Ok(0x7A));
    }

    #[test]
    fn hex_letters_are_only_digits_inside_a_hex_literal() {
        // अ is 10 in hex...
        assert_eq!(digit_value('\u{0905}', Radix::Hexadecimal), Some(10));
        // ...and not a digit at all in decimal, so it cannot be mistaken for one
        // when it appears in an identifier.
        assert_eq!(digit_value('\u{0905}', Radix::Decimal), None);
    }

    #[test]
    fn negatives_use_the_language_marker_and_round_trip() {
        // `ऋण`, not `-`. And it must survive `split_sign`, which is the reader
        // for exactly this prefix — writer and reader graded against each other.
        for n in [-1i64, -7, -42, -1234, -(u32::MAX as i64)] {
            let s = to_devanagari_signed(n);
            assert!(s.starts_with(NEGATIVE), "{n} rendered {s:?} without ऋण");
            let (neg, rest) = split_sign(&s);
            assert!(neg, "split_sign did not see the sign in {s:?}");
            assert_eq!(
                value(rest).unwrap(),
                n.unsigned_abs(),
                "magnitude lost for {n}"
            );
        }
        // A cast would render -1 as 18446744073709551615; this is the control.
        assert_eq!(to_devanagari_signed(-1), "ऋण१");
        assert_eq!(to_devanagari_signed(0), "०");
    }

    #[test]
    fn zero_is_a_digit_not_an_empty_string() {
        // A `while n > 0` loop renders 0 as "" and every other value correctly,
        // so nothing else in this suite would catch it.
        assert_eq!(to_devanagari(0), "०");
    }

    #[test]
    fn to_devanagari_round_trips_through_value() {
        // GRADED AGAINST THIS MODULE'S OWN READER, not against a second table:
        // if the writer and `value` ever disagree the round trip breaks, which
        // is the whole point of putting them in one file.
        for n in [
            0u64,
            1,
            7,
            9,
            10,
            42,
            100,
            1234,
            65535,
            1_000_000,
            u32::MAX as u64,
        ] {
            let s = to_devanagari(n);
            assert!(
                s.chars().all(is_devanagari_digit),
                "{n} rendered as {s:?} which is not all Devanagari digits"
            );
            assert_eq!(value(&s).unwrap(), n, "round trip failed for {n} via {s:?}");
        }
    }

    #[test]
    fn to_devanagari_is_not_ascii() {
        // The falsification control. `n.to_string()` passes every assertion
        // above except this one, and `n.to_string()` is exactly what this
        // function replaces.
        let s = to_devanagari(2026);
        assert_eq!(s, "२०२६");
        assert!(!s.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn ascii_digits_are_not_numerals() {
        // doc 15 R-15-1: rejected upstream by the repertoire gate, and here too.
        assert!(!is_numeral("1729"));
    }

    #[test]
    fn digit_out_of_radix_is_rejected() {
        // ८ is not a binary digit
        let bad = "\u{0966}\u{0926}\u{094D}\u{0935}\u{093F}\u{096E}";
        assert!(matches!(
            validate(bad),
            Err(NumeralError::BadDigit {
                radix: Radix::Binary,
                ..
            })
        ));
    }

    #[test]
    fn prefix_without_digits_is_rejected() {
        assert_eq!(validate(HEX), Err(NumeralError::NoDigits));
    }

    #[test]
    fn identifiers_are_not_numerals() {
        assert!(!is_numeral("\u{0915}"));
        assert_eq!(classify("\u{0915}"), None);
    }
}
