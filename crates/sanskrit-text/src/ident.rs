//! Identifier validation — the SANSOS profile of UAX #31.
//!
//! Stricter than the standard on purpose. An identifier is a name in the symbol
//! table (doc 03 §5); two names that a reader cannot tell apart, but the linker
//! can, is a supply-chain attack waiting to be written. Doc 13 §7 catalogues the
//! class; this module is where it is closed.
//!
//! # The profile (doc 01 §2.6)
//!
//! ```text
//! start     ::= Script=Devanagari ∧ XID_Start
//! continue  ::= Script=Devanagari ∧ XID_Continue      (includes virāma and ०–९)
//! joiner    ::= ZWJ | ZWNJ, ONLY between a virāma and a consonant
//! ```
//!
//! Everything else is rejected: no Latin, no other Indic script, no bidi
//! controls, no stray format characters, no joiners in decorative positions.
//! UTS #39 restriction level 2 (Single Script) falls out of the Devanagari
//! requirement rather than needing a separate pass.
//!
//! # Precondition: input is NFC
//!
//! Normalization is checked **once per file** at pass 0 (doc 15 §1), where an
//! allocator is available, not once per identifier. This module therefore
//! assumes NFC and stays allocation-free, so the assembler's tokenizer can call
//! it per token without a heap.
//!
//! # Equality is byte equality
//!
//! Two identifiers are the same identifier iff their NFC bytes are equal. There
//! is deliberately **no sandhi folding** (doc 01 D-01-C): reverse sandhi is
//! ambiguous, and a symbol table cannot rest on an ambiguous relation.

use crate::lookup;
use crate::segment::is_devanagari;
use crate::tables::{Flag, INCB, Incb, XID_CONTINUE, XID_START};

const ZWNJ: char = '\u{200C}';
const ZWJ: char = '\u{200D}';
const VIRAMA: char = '\u{094D}';

/// Why an identifier was rejected. Carries the byte offset so the assembler can
/// point at the exact character (doc 03 §4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdentError {
    /// Empty string.
    Empty,
    /// First character cannot begin an identifier.
    BadStart {
        /// Byte offset of the character.
        at: usize,
        /// The offending character.
        ch: char,
    },
    /// Character cannot continue an identifier.
    BadContinue {
        /// Byte offset of the character.
        at: usize,
        /// The offending character.
        ch: char,
    },
    /// A joiner outside its one legal position (between virāma and consonant).
    MisplacedJoiner {
        /// Byte offset of the character.
        at: usize,
        /// The offending joiner.
        ch: char,
    },
    /// A bidi or other format control — the Trojan Source class.
    FormatControl {
        /// Byte offset of the character.
        at: usize,
        /// The offending control.
        ch: char,
    },
    /// A letter from another script.
    MixedScript {
        /// Byte offset of the character.
        at: usize,
        /// The offending character.
        ch: char,
    },
}

impl IdentError {
    /// Byte offset of the offending character, if any.
    #[must_use]
    pub fn offset(&self) -> Option<usize> {
        match *self {
            IdentError::Empty => None,
            IdentError::BadStart { at, .. }
            | IdentError::BadContinue { at, .. }
            | IdentError::MisplacedJoiner { at, .. }
            | IdentError::FormatControl { at, .. }
            | IdentError::MixedScript { at, .. } => Some(at),
        }
    }
}

fn xid_start(ch: char) -> bool {
    lookup(XID_START, ch as u32, Flag::No) == Flag::Yes
}

fn xid_continue(ch: char) -> bool {
    lookup(XID_CONTINUE, ch as u32, Flag::No) == Flag::Yes
}

fn is_consonant(ch: char) -> bool {
    lookup(INCB, ch as u32, Incb::None) == Incb::Consonant
}

/// Identifiers are restricted to the Devanagari **base block**, not the whole
/// `Script=Devanagari` set.
///
/// Devanagari Extended (U+A8E0–A8FF) and Vedic Extensions are `Script=Devanagari`
/// and several are `XID_Continue`, so a plain UAX #31 profile admits them — but
/// [ADR-0003](../../../docs/adr/0003-orthographic-closure-symbol-table.md)
/// measured that **no installed font renders them**. An identifier containing a
/// character the reader cannot see is precisely the invisible-difference attack
/// this module exists to close, so the profile stops at U+097F.
fn is_devanagari_base(ch: char) -> bool {
    ('\u{0900}'..='\u{097F}').contains(&ch)
}

/// Base-block characters that ADR-0003 gives a **syntactic** role.
///
/// These must not also be identifier characters, or the lexer is ambiguous:
/// `कऽख` would be either one name or two names separated by `ऽ`, with no way to
/// decide. Several are `XID_Start`/`XID_Continue` by Unicode (`ऽ` and `ॐ` are
/// `Lo`, `ॱ` is `Lm`), so excluding them has to be explicit.
fn is_syntax_reserved(ch: char) -> bool {
    matches!(
        ch,
        '\u{0964}'   // ।  statement terminator
        | '\u{0965}' // ॥  directive / section terminator
        | '\u{0970}' // ॰  comment and string-escape introducer
        | '\u{0971}' // ॱ  member access; doubled, the label/type mark
        | '\u{093D}' // ऽ  argument separator
        | '\u{0950}' // ॐ  file invocation
    )
}

/// Bidi controls and the invisible-formatting characters that make one string
/// display as another. Called out separately from `BadContinue` because the
/// diagnostic matters: this is an attack, not a typo.
fn is_format_control(ch: char) -> bool {
    matches!(ch,
        '\u{200E}' | '\u{200F}'              // LRM, RLM
        | '\u{202A}'..='\u{202E}'            // LRE, RLE, PDF, LRO, RLO
        | '\u{2066}'..='\u{2069}'            // LRI, RLI, FSI, PDI
        | '\u{061C}'                         // ALM
        | '\u{FEFF}'                         // BOM / ZWNBSP
        | '\u{00AD}'                         // soft hyphen
        | '\u{2060}'..='\u{2064}'            // word joiner, invisible operators
    )
}

/// Validate an identifier against the SANSOS profile.
///
/// Allocation-free. Assumes NFC input (see module docs).
///
/// # Errors
/// Returns the first violation found, with its byte offset.
pub fn validate(s: &str) -> Result<(), IdentError> {
    let mut chars = s.char_indices();

    let Some((_, first)) = chars.next() else {
        return Err(IdentError::Empty);
    };

    if first == ZWJ || first == ZWNJ {
        // A leading joiner has nothing to join. Diagnosed as a misplaced joiner
        // rather than a bad start, because it is an attack, not a typo.
        return Err(IdentError::MisplacedJoiner { at: 0, ch: first });
    }
    if is_format_control(first) {
        return Err(IdentError::FormatControl { at: 0, ch: first });
    }
    if !is_devanagari_base(first) {
        // A letter from elsewhere gets the sharper diagnostic; anything else is
        // simply not a valid start. `is_devanagari` (whole script) would let
        // Devanagari Extended through — see `is_devanagari_base`.
        return Err(if xid_start(first) || is_devanagari(first) {
            IdentError::MixedScript { at: 0, ch: first }
        } else {
            IdentError::BadStart { at: 0, ch: first }
        });
    }
    if is_syntax_reserved(first) || !xid_start(first) {
        // Devanagari, but a combining mark, a digit, or a sign ADR-0003 reserves.
        return Err(IdentError::BadStart { at: 0, ch: first });
    }

    let mut prev = first;
    for (at, ch) in chars {
        if ch == ZWJ || ch == ZWNJ {
            // The single legal position: virāma before, consonant after. This is
            // what lets a scribe request or suppress a conjunct (see the A-016
            // vectors) without opening the invisible-character hole.
            let next_is_consonant = s[at + ch.len_utf8()..]
                .chars()
                .next()
                .is_some_and(is_consonant);
            if prev != VIRAMA || !next_is_consonant {
                return Err(IdentError::MisplacedJoiner { at, ch });
            }
            prev = ch;
            continue;
        }
        if is_format_control(ch) {
            return Err(IdentError::FormatControl { at, ch });
        }
        if !is_devanagari_base(ch) {
            return Err(if xid_continue(ch) || is_devanagari(ch) {
                IdentError::MixedScript { at, ch }
            } else {
                IdentError::BadContinue { at, ch }
            });
        }
        if is_syntax_reserved(ch) || !xid_continue(ch) {
            return Err(IdentError::BadContinue { at, ch });
        }
        prev = ch;
    }

    Ok(())
}

/// Convenience predicate over [`validate`].
#[must_use]
pub fn is_valid(s: &str) -> bool {
    validate(s).is_ok()
}

/// Whether two identifiers denote the same symbol.
///
/// Byte equality, nothing more. Both sides are assumed NFC. See the module docs
/// for why sandhi is deliberately not folded here.
#[must_use]
pub fn same_symbol(a: &str, b: &str) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_devanagari_names() {
        for s in [
            "\u{0915}",                                         // क
            "\u{0915}\u{094D}\u{0937}",                         // क्ष
            "\u{092F}\u{094B}\u{0917}\u{0903}",                 // योगः
            "\u{0915}\u{094B}\u{0937}\u{094D}\u{0920}",         // कोष्ठ
            "\u{0938}\u{094D}\u{092E}\u{0943}\u{0924}\u{093F}", // स्मृति
        ] {
            assert!(is_valid(s), "should accept {s}");
        }
    }

    #[test]
    fn digits_continue_but_do_not_start() {
        // कोष्ठ० — a register name with an index
        assert!(is_valid("\u{0915}\u{0966}"));
        assert!(matches!(
            validate("\u{0966}\u{0915}"),
            Err(IdentError::BadStart { .. })
        ));
    }

    #[test]
    fn matra_cannot_start() {
        // A dependent vowel sign has no base to attach to.
        assert!(matches!(
            validate("\u{093F}\u{0915}"),
            Err(IdentError::BadStart { .. })
        ));
    }

    #[test]
    fn rejects_latin_and_mixed_script() {
        assert!(matches!(
            validate("abc"),
            Err(IdentError::MixedScript { at: 0, .. })
        ));
        // क + Latin a — the classic homograph vector (doc 13 §7)
        assert!(matches!(
            validate("\u{0915}a"),
            Err(IdentError::MixedScript { .. })
        ));
        // क + Bengali ক (visually similar, different script)
        assert!(matches!(
            validate("\u{0915}\u{0995}"),
            Err(IdentError::MixedScript { .. })
        ));
    }

    #[test]
    fn joiner_only_between_virama_and_consonant() {
        // Legal: क + virāma + ZWNJ + ष (suppress the conjunct)
        assert!(is_valid("\u{0915}\u{094D}\u{200C}\u{0937}"));
        assert!(is_valid("\u{0915}\u{094D}\u{200D}\u{0937}"));

        // Illegal: joiner not after a virāma — this is the invisible-difference
        // attack, where क‌ष and कष look identical but are different symbols.
        assert!(matches!(
            validate("\u{0915}\u{200C}\u{0937}"),
            Err(IdentError::MisplacedJoiner { .. })
        ));
        // Illegal: trailing joiner, nothing to join to.
        assert!(matches!(
            validate("\u{0915}\u{094D}\u{200D}"),
            Err(IdentError::MisplacedJoiner { .. })
        ));
        // Illegal: after virāma but followed by a vowel sign, not a consonant.
        assert!(matches!(
            validate("\u{0915}\u{094D}\u{200D}\u{093E}"),
            Err(IdentError::MisplacedJoiner { .. })
        ));
    }

    #[test]
    fn rejects_bidi_and_invisible_controls() {
        // Trojan Source: rustc denies this too (measured, corpus TX-05), so here
        // we match rather than exceed — but we reject it as an identifier error
        // rather than a literal lint.
        for c in [
            '\u{202E}', '\u{2066}', '\u{200E}', '\u{FEFF}', '\u{00AD}', '\u{2060}',
        ] {
            let mut s = alloc_free_pair('\u{0915}', c);
            assert!(
                matches!(validate(s.as_str()), Err(IdentError::FormatControl { .. })),
                "should reject U+{:04X}",
                c as u32
            );
            s.clear();
        }
    }

    #[test]
    fn empty_is_rejected() {
        assert_eq!(validate(""), Err(IdentError::Empty));
    }

    #[test]
    fn error_carries_a_usable_offset() {
        // Offset must be a byte index into the original string, so a diagnostic
        // can slice it. क is 3 bytes.
        let e = validate("\u{0915}a").unwrap_err();
        assert_eq!(e.offset(), Some(3));
    }

    #[test]
    fn equality_is_byte_equality_not_sandhi() {
        let tat = "\u{0924}\u{0924}\u{094D}"; // तत्
        let asti = "\u{0905}\u{0938}\u{094D}\u{0924}\u{093F}"; // अस्ति
        // तत् and अस्ति do not become one symbol just because sandhi could join
        // them in prose. Reverse sandhi is ambiguous; symbol tables cannot be.
        assert!(!same_symbol(tat, asti));
        assert!(same_symbol(tat, tat));
    }

    /// Build a 2-char string without pulling in `alloc` for the test.
    struct Pair([u8; 8], usize);
    impl Pair {
        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.0[..self.1]).unwrap()
        }
        fn clear(&mut self) {
            self.1 = 0;
        }
    }
    fn alloc_free_pair(a: char, b: char) -> Pair {
        let mut buf = [0u8; 8];
        let n = a.encode_utf8(&mut buf).len();
        let m = b.encode_utf8(&mut buf[n..]).len();
        Pair(buf, n + m)
    }
}
