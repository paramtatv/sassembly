//! The repertoire gate — **pass 0** of the toolchain (doc 15 §1).
//!
//! Rule R-15-1: every character in an authored SANSOS source file must belong to
//! the Sanskrit repertoire, plus exactly three layout characters. No Latin, no
//! ASCII digits, no ASCII punctuation, no other script.
//!
//! This runs *before* normalization and *before* the lexer, so no later stage
//! ever has to consider a foreign character. It is also, incidentally, the
//! strongest identifier-confusability defence available: a mixed-script
//! identifier is not rejected here, it is **unrepresentable** (doc 13 §7,
//! corpus row `TX-04`).
//!
//! # What counts as the repertoire
//!
//! Per R-15-1: Devanagari (U+0900–097F), Devanagari Extended (U+A8E0–A8FF),
//! Devanagari Extended-A (U+11B00–11B5F), Vedic Extensions (U+1CD0–1CFF), plus
//! SPACE, TAB and LINE FEED.
//!
//! Note the asymmetry with [`crate::ident`]: identifiers are restricted further,
//! to the base block only, because ADR-0003 found no font renders the extended
//! blocks. The wider set is still allowed **in strings and comments**, which are
//! data rather than names.

use crate::segment::aksharas;

/// A character that does not belong in SANSOS source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Violation {
    /// Byte offset in the file.
    pub at: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column, counted in **akṣaras** — not bytes and not codepoints,
    /// so the reported column matches what the terminal shows (doc 04 §2).
    pub column: usize,
    /// The offending character.
    pub ch: char,
}

/// Whether a character belongs to the Sanskrit repertoire (R-15-1).
#[must_use]
pub fn in_repertoire(ch: char) -> bool {
    matches!(ch,
        ' ' | '\t' | '\n'
        | '\u{0900}'..='\u{097F}'      // Devanagari
        | '\u{A8E0}'..='\u{A8FF}'      // Devanagari Extended
        | '\u{1CD0}'..='\u{1CFF}'      // Vedic Extensions
        | '\u{11B00}'..='\u{11B5F}'    // Devanagari Extended-A
        | '\u{200C}' | '\u{200D}'      // ZWNJ/ZWJ — position-checked in `ident`
    )
}

/// Check a source file against R-15-1.
///
/// Returns every violation, not just the first: a file pasted from a Latin
/// source has hundreds, and reporting them one build at a time is useless.
#[must_use]
pub fn check(source: &str) -> alloc::vec::Vec<Violation> {
    extern crate alloc;
    let mut out = alloc::vec::Vec::new();
    let mut line = 1usize;
    let mut line_start = 0usize;

    for (at, ch) in source.char_indices() {
        if ch == '\n' {
            line += 1;
            line_start = at + 1;
            continue;
        }
        if !in_repertoire(ch) {
            // Column in akṣaras, so the caret lands where the eye expects.
            let column = aksharas(&source[line_start..at]).count() + 1;
            out.push(Violation {
                at,
                line,
                column,
                ch,
            });
        }
    }
    out
}

/// Whether a source file passes the gate.
#[must_use]
pub fn is_clean(source: &str) -> bool {
    source.chars().all(in_repertoire)
}

extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devanagari_source_passes() {
        assert!(is_clean(
            "\u{0950}\n\u{0915}\u{094D}\u{0937} \u{0964}\n\t\u{0967}\u{096D}\n"
        ));
    }

    #[test]
    fn latin_is_rejected() {
        let v = check("\u{0915} = 1");
        // '=', ' ' is fine, '1' is an ASCII digit
        assert!(v.iter().any(|x| x.ch == '='));
        assert!(v.iter().any(|x| x.ch == '1'));
    }

    #[test]
    fn ascii_digits_are_rejected_not_normalised() {
        // doc 15 §3.3 tightened doc 01 D-01-D: ASCII digits are a hard error.
        let v = check("\u{0967}7");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].ch, '7');
    }

    #[test]
    fn every_violation_is_reported_not_just_the_first() {
        let v = check("fn main() {}");
        assert!(v.len() > 8, "got {} violations", v.len());
    }

    #[test]
    fn column_is_counted_in_aksharas() {
        // क्ष is 3 codepoints, 9 bytes, but ONE column.
        let v = check("\u{0915}\u{094D}\u{0937}X");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].column, 2, "column must be akṣara-based");
        assert_eq!(v[0].line, 1);
    }

    #[test]
    fn line_numbers_are_tracked() {
        let v = check("\u{0915}\n\u{0916}\nZ");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].line, 3);
    }

    #[test]
    fn joiners_are_allowed_here_and_position_checked_elsewhere() {
        // The gate admits them; `ident` decides whether the position is legal.
        assert!(is_clean("\u{0915}\u{094D}\u{200C}\u{0937}"));
    }

    #[test]
    fn carriage_return_is_not_layout() {
        // Only SPACE, TAB, LF. CR would make line endings ambiguous and break
        // byte-identical reproducibility across platforms.
        assert!(!is_clean("\u{0915}\r\n"));
    }
}
