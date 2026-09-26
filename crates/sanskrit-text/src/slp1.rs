//! SLP1 ⇄ Devanagari, as a **provable bijection**.
//!
//! SLP1 (Sanskrit Library Phonetic Basic) is one ASCII character per phoneme,
//! which makes the round trip a table walk rather than a transliteration
//! heuristic.
//!
//! # This is not an authoring surface
//!
//! Orthographic closure ([doc 15](../../../research/15-orthographic-closure.md),
//! ADR-0001 D-2) forbids ASCII in source. SLP1 exists for exactly three
//! non-authoring uses:
//!
//! 1. **Input-method keystrokes** — the typist presses SLP1 keys; the file
//!    receives Devanagari (doc 04 §6). Keystrokes are not file content.
//! 2. **Pre-font output** — panics and early boot happen before the shaper
//!    exists (doc 11 §7.1), and a table of 60 ASCII bytes is affordable there
//!    when a font is not.
//! 3. **The Rust bootstrap tree**, which is deleted at Gate D.
//!
//! # The domain
//!
//! Bijectivity is a property of a *domain*, so the domain is stated and anything
//! outside it is **rejected rather than mangled** — the same discipline as
//! [`crate::ident`]. Silently lossy transliteration is how `ka` and `k+a` become
//! the same symbol.
//!
//! The one genuine ambiguity is a virāma followed by an **independent** vowel:
//! `क्` + `अ` and `क` both encode to `ka`. That sequence is not valid Devanagari
//! orthography — a vowel after a consonant is written with a mātrā — so it is
//! rejected by [`Slp1Error::ViramaBeforeVowel`] rather than silently collapsed.
//!
//! # Profile
//!
//! Canonical SLP1 for the phonemic core, plus documented extensions for
//! characters SLP1 never covered. Every extension uses an ASCII byte the
//! canonical set leaves free.

use core::fmt;

const VIRAMA: char = '\u{094D}';
const DEVA_ZERO: u32 = 0x0966;

/// Independent vowels. The inherent /a/ has no mātrā, which is why `MATRA`
/// below is shorter than this table by exactly one entry.
const VOWELS: &[(char, char)] = &[
    ('\u{0905}', 'a'),
    ('\u{0906}', 'A'),
    ('\u{0907}', 'i'),
    ('\u{0908}', 'I'),
    ('\u{0909}', 'u'),
    ('\u{090A}', 'U'),
    ('\u{090B}', 'f'),
    ('\u{0960}', 'F'),
    ('\u{090C}', 'x'),
    ('\u{0961}', 'X'),
    ('\u{090F}', 'e'),
    ('\u{0910}', 'E'),
    ('\u{0913}', 'o'),
    ('\u{0914}', 'O'),
];

/// Dependent vowel signs (mātrās), in the same SLP1 letters as `VOWELS`.
const MATRAS: &[(char, char)] = &[
    ('\u{093E}', 'A'),
    ('\u{093F}', 'i'),
    ('\u{0940}', 'I'),
    ('\u{0941}', 'u'),
    ('\u{0942}', 'U'),
    ('\u{0943}', 'f'),
    ('\u{0944}', 'F'),
    ('\u{0962}', 'x'),
    ('\u{0963}', 'X'),
    ('\u{0947}', 'e'),
    ('\u{0948}', 'E'),
    ('\u{094B}', 'o'),
    ('\u{094C}', 'O'),
];

const CONSONANTS: &[(char, char)] = &[
    ('\u{0915}', 'k'),
    ('\u{0916}', 'K'),
    ('\u{0917}', 'g'),
    ('\u{0918}', 'G'),
    ('\u{0919}', 'N'),
    ('\u{091A}', 'c'),
    ('\u{091B}', 'C'),
    ('\u{091C}', 'j'),
    ('\u{091D}', 'J'),
    ('\u{091E}', 'Y'),
    ('\u{091F}', 'w'),
    ('\u{0920}', 'W'),
    ('\u{0921}', 'q'),
    ('\u{0922}', 'Q'),
    ('\u{0923}', 'R'),
    ('\u{0924}', 't'),
    ('\u{0925}', 'T'),
    ('\u{0926}', 'd'),
    ('\u{0927}', 'D'),
    ('\u{0928}', 'n'),
    ('\u{092A}', 'p'),
    ('\u{092B}', 'P'),
    ('\u{092C}', 'b'),
    ('\u{092D}', 'B'),
    ('\u{092E}', 'm'),
    ('\u{092F}', 'y'),
    ('\u{0930}', 'r'),
    ('\u{0932}', 'l'),
    ('\u{0935}', 'v'),
    ('\u{0936}', 'S'),
    ('\u{0937}', 'z'),
    ('\u{0938}', 's'),
    ('\u{0939}', 'h'),
    ('\u{0933}', 'L'),
];

/// Marks and signs. Entries above the divider are canonical SLP1; below it are
/// SANSOS extensions for characters SLP1 does not define. Each extension takes
/// an ASCII byte the canonical set leaves unused.
const SIGNS: &[(char, char)] = &[
    // canonical
    ('\u{0902}', 'M'),  // anusvāra
    ('\u{0903}', 'H'),  // visarga
    ('\u{0901}', '~'),  // candrabindu
    ('\u{093D}', '\''), // avagraha
    // --- SANSOS extensions ---
    ('\u{093C}', 'Z'), // nukta            ('Z' is free: ष is lowercase 'z')
    ('\u{200C}', '^'), // ZWNJ             (conjunct suppressed)
    ('\u{200D}', '_'), // ZWJ              (conjunct forced)
    ('\u{0964}', '|'), // ।  danda
    ('\u{0965}', '&'), // ॥  double danda  (NOT "||": ।। must not alias ॥)
    ('\u{0970}', '@'), // ॰  abbreviation
    ('\u{0971}', '.'), // ॱ  high dot
    ('\u{0950}', '$'), // ॐ  om
    ('\u{0951}', '='), // udātta
    ('\u{0952}', '!'), // anudātta
];

const WHITESPACE: &[char] = &[' ', '\t', '\n'];

/// Why a string fell outside the bijection's domain.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slp1Error {
    /// A Devanagari character with no SLP1 representation in this profile.
    Unmappable {
        /// Byte offset.
        at: usize,
        /// The character.
        ch: char,
    },
    /// An SLP1 byte that is not in the profile.
    UnknownByte {
        /// Byte offset.
        at: usize,
        /// The byte, as a character.
        ch: char,
    },
    /// Virāma immediately followed by an independent vowel — the one sequence
    /// that would make the mapping non-injective. Not valid orthography.
    ViramaBeforeVowel {
        /// Byte offset of the virāma.
        at: usize,
    },
    /// A mātrā or virāma with no consonant to attach to.
    DanglingMark {
        /// Byte offset.
        at: usize,
        /// The character.
        ch: char,
    },
    /// The output buffer could not take more bytes.
    OutputFull,
}

fn fwd(table: &[(char, char)], ch: char) -> Option<char> {
    table.iter().find(|&&(d, _)| d == ch).map(|&(_, s)| s)
}

fn rev(table: &[(char, char)], s: char) -> Option<char> {
    table.iter().find(|&&(_, x)| x == s).map(|&(d, _)| d)
}

fn deva_digit(ch: char) -> Option<char> {
    let cp = ch as u32;
    (DEVA_ZERO..DEVA_ZERO + 10)
        .contains(&cp)
        .then(|| char::from_u32(cp - DEVA_ZERO + 0x30))?
}

/// Devanagari → SLP1, writing into `out`.
///
/// Allocation-free: `out` may be a fixed buffer (see [`Buf`]) or, with `alloc`,
/// a `String`.
///
/// # Errors
/// Returns the first character outside the domain, with its byte offset.
pub fn encode_into<W: fmt::Write>(deva: &str, out: &mut W) -> Result<(), Slp1Error> {
    let mut it = deva.char_indices().peekable();

    while let Some((at, ch)) = it.next() {
        if let Some(c) = fwd(CONSONANTS, ch) {
            put(out, c)?;
            match it.peek().copied() {
                Some((v_at, VIRAMA)) => {
                    it.next();
                    // What follows the virāma decides whether this is a conjunct,
                    // a dead consonant, or the one ambiguous sequence.
                    match it.peek().copied() {
                        Some((_, n)) if is_vowel(n) => {
                            return Err(Slp1Error::ViramaBeforeVowel { at: v_at });
                        }
                        // consonant, joiner, whitespace, punctuation, or EOF:
                        // the ABSENCE of a vowel letter is what encodes the virāma
                        _ => {}
                    }
                }
                Some((_, m)) if fwd(MATRAS, m).is_some() => {
                    it.next();
                    put(out, fwd(MATRAS, m).unwrap())?;
                }
                // no virāma, no mātrā: the inherent /a/ is explicit in SLP1
                _ => put(out, 'a')?,
            }
        } else if let Some(v) = fwd(VOWELS, ch) {
            put(out, v)?;
        } else if let Some(s) = fwd(SIGNS, ch) {
            put(out, s)?;
        } else if let Some(d) = deva_digit(ch) {
            put(out, d)?;
        } else if WHITESPACE.contains(&ch) {
            put(out, ch)?;
        } else if ch == VIRAMA || fwd(MATRAS, ch).is_some() {
            return Err(Slp1Error::DanglingMark { at, ch });
        } else {
            return Err(Slp1Error::Unmappable { at, ch });
        }
    }
    Ok(())
}

fn is_vowel(ch: char) -> bool {
    fwd(VOWELS, ch).is_some()
}

/// SLP1 → Devanagari, writing into `out`.
///
/// # Errors
/// Returns the first byte outside the profile, with its offset.
pub fn decode_into<W: fmt::Write>(slp1: &str, out: &mut W) -> Result<(), Slp1Error> {
    let mut it = slp1.char_indices().peekable();

    while let Some((at, ch)) = it.next() {
        if let Some(c) = rev(CONSONANTS, ch) {
            put(out, c)?;
            // A consonant carries an inherent /a/; anything else must be spelled.
            match it.peek().copied() {
                Some((_, 'a')) => {
                    it.next();
                }
                Some((_, v)) if rev(MATRAS, v).is_some() => {
                    it.next();
                    put(out, rev(MATRAS, v).unwrap())?;
                }
                // no vowel letter follows: the consonant is dead
                _ => put(out, VIRAMA)?,
            }
        } else if let Some(v) = rev(VOWELS, ch) {
            put(out, v)?;
        } else if let Some(s) = rev(SIGNS, ch) {
            put(out, s)?;
        } else if ch.is_ascii_digit() {
            put(out, char::from_u32(DEVA_ZERO + (ch as u32 - 0x30)).unwrap())?;
        } else if WHITESPACE.contains(&ch) {
            put(out, ch)?;
        } else {
            return Err(Slp1Error::UnknownByte { at, ch });
        }
    }
    Ok(())
}

fn put<W: fmt::Write>(out: &mut W, ch: char) -> Result<(), Slp1Error> {
    out.write_char(ch).map_err(|_| Slp1Error::OutputFull)
}

/// A fixed-capacity `fmt::Write` sink, so panics and early boot can transliterate
/// with no allocator (doc 11 §7.1).
pub struct Buf<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> Buf<'a> {
    /// Wrap a byte buffer.
    #[must_use]
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, len: 0 }
    }
    /// The bytes written so far.
    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
    /// Number of bytes written.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether nothing has been written.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl fmt::Write for Buf<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > self.buf.len() {
            return Err(fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(s: &str) -> Result<alloc::string::String, Slp1Error> {
        let mut o = alloc::string::String::new();
        encode_into(s, &mut o)?;
        Ok(o)
    }
    fn dec(s: &str) -> Result<alloc::string::String, Slp1Error> {
        let mut o = alloc::string::String::new();
        decode_into(s, &mut o)?;
        Ok(o)
    }
    extern crate alloc;

    #[test]
    fn inherent_vowel_versus_virama() {
        // The distinction the whole scheme turns on.
        assert_eq!(enc("\u{0915}").unwrap(), "ka"); // क
        assert_eq!(enc("\u{0915}\u{094D}").unwrap(), "k"); // क्
        assert_eq!(dec("ka").unwrap(), "\u{0915}");
        assert_eq!(dec("k").unwrap(), "\u{0915}\u{094D}");
    }

    #[test]
    fn conjuncts_need_no_separator() {
        assert_eq!(enc("\u{0915}\u{094D}\u{0937}").unwrap(), "kza"); // क्ष
        assert_eq!(dec("kza").unwrap(), "\u{0915}\u{094D}\u{0937}");
        // क्ष्ण
        assert_eq!(
            enc("\u{0915}\u{094D}\u{0937}\u{094D}\u{0923}").unwrap(),
            "kzRa"
        );
    }

    #[test]
    fn matras_and_marks() {
        assert_eq!(enc("\u{0915}\u{093F}").unwrap(), "ki"); // कि
        assert_eq!(enc("\u{0915}\u{0902}").unwrap(), "kaM"); // कं
        assert_eq!(enc("\u{0915}\u{0903}").unwrap(), "kaH"); // कः
        assert_eq!(enc("\u{0928}\u{092E}\u{0903}").unwrap(), "namaH"); // नमः
    }

    #[test]
    fn word_final_halanta_round_trips() {
        // सस् — the case that made the "absence of a vowel encodes virāma" rule
        let s = "\u{0938}\u{0938}\u{094D}";
        assert_eq!(enc(s).unwrap(), "sas");
        assert_eq!(dec("sas").unwrap(), s);
    }

    #[test]
    fn joiners_round_trip() {
        let zwnj = "\u{0915}\u{094D}\u{200C}\u{0937}";
        let zwj = "\u{0915}\u{094D}\u{200D}\u{0937}";
        assert_eq!(enc(zwnj).unwrap(), "k^za");
        assert_eq!(enc(zwj).unwrap(), "k_za");
        assert_eq!(dec("k^za").unwrap(), zwnj);
        assert_eq!(dec("k_za").unwrap(), zwj);
    }

    #[test]
    fn digits_and_signs() {
        assert_eq!(enc("\u{0967}\u{096D}\u{0968}\u{096F}").unwrap(), "1729");
        assert_eq!(dec("1729").unwrap(), "\u{0967}\u{096D}\u{0968}\u{096F}");
        assert_eq!(enc("\u{0964}").unwrap(), "|");
        assert_eq!(enc("\u{0965}").unwrap(), "&");
        // ।। must NOT alias ॥
        assert_ne!(enc("\u{0964}\u{0964}").unwrap(), enc("\u{0965}").unwrap());
    }

    #[test]
    fn the_invocation_round_trips() {
        let s = "\u{0950} \u{092A}\u{0930}\u{092E} \u{0928}\u{092E}\u{0903}";
        let e = enc(s).unwrap();
        assert_eq!(e, "$ parama namaH");
        assert_eq!(dec(&e).unwrap(), s);
    }

    #[test]
    fn ambiguous_sequence_is_rejected_not_mangled() {
        // क् + अ would encode to "ka", colliding with क. Rejected instead.
        assert!(matches!(
            enc("\u{0915}\u{094D}\u{0905}"),
            Err(Slp1Error::ViramaBeforeVowel { .. })
        ));
    }

    #[test]
    fn dangling_marks_are_rejected() {
        assert!(matches!(
            enc("\u{093F}"),
            Err(Slp1Error::DanglingMark { .. })
        ));
        assert!(matches!(
            enc("\u{094D}"),
            Err(Slp1Error::DanglingMark { .. })
        ));
    }

    #[test]
    fn out_of_domain_is_rejected() {
        assert!(matches!(enc("A"), Err(Slp1Error::Unmappable { .. })));
        assert!(matches!(
            dec("\u{0915}"),
            Err(Slp1Error::UnknownByte { .. })
        ));
    }

    #[test]
    fn fixed_buffer_needs_no_allocator() {
        let mut raw = [0u8; 64];
        let mut b = Buf::new(&mut raw);
        encode_into("\u{0915}\u{094D}\u{0937}", &mut b).unwrap();
        assert_eq!(b.as_str(), "kza");
    }

    #[test]
    fn buffer_overflow_is_an_error_not_a_truncation() {
        let mut raw = [0u8; 2];
        let mut b = Buf::new(&mut raw);
        assert_eq!(
            encode_into("\u{0915}\u{0916}\u{0917}", &mut b),
            Err(Slp1Error::OutputFull)
        );
    }
}
