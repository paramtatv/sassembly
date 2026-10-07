//! ADR-0044 — Devanagari-8: one octet per letter for data literals.
//!
//! **D1, the code.** `octet = 0x80 + (code point − 0x0900)` for every code
//! point in U+0900–U+097F, and the inverse. The block is exactly 128 code
//! points, so the code is total over it and needs no table. In UTF-8 the block
//! is exactly the triples `E0 A4 80..BF` and `E0 A5 80..BF`, so the translation
//! is `octet = 0x80 + (second − 0xA4) × 64 + (third − 0x80)`, and back.
//!
//! **D2, the source mark.** `वर्णाष्टकम् … इति` is a literal that mirrors
//! `उक्तम् … इति` exactly — the same close, the same `इति इति` escape, the same
//! one-line rule — and whose VALUE is its letters in Devanagari-8. A code point
//! outside the block (a space, a digit, ASCII, another script) is refused at
//! compile time by `अदेवनागरीवर्णप्रतिषेधः`, naming the letter and its column.
//!
//! **D3, the assembler text.** The letters travel as UTF-8 text, the operand
//! `वर्णाष्टकम् … इति` of `॥ अष्टकाः … ॥` beside SAS-011's `उक्तम् … इति`, and
//! both assemblers pack each triple into one octet. [`payload`] is the
//! emitter's predicate: the spelling is used only where it reads back as
//! exactly the same octets, and every other run stays numerals.
//!
//! Every function here is ONE definition of the code: the lexer, the
//! interpreter, `riscv64.rs` and the Rust assembler all call it, and the `.t1`
//! twins (`parse.t1`, `ir.t1`, `yantrotsarjana.t1`, `vakyavibhaga.t1`) are
//! walks over the same two triples.

use crate::lex::{Kind, Token};

/// `वर्णाष्टकम्` — opens a Devanagari-8 literal (ADR-0044 D2; owner-ruled name).
pub const OPEN: &str = "वर्णाष्टकम्";

/// `अदेवनागरीवर्णप्रतिषेधः` — the owner-named refusal for a code point outside
/// U+0900–U+097F inside a `वर्णाष्टकम् … इति` literal (ADR-0044 D2).
pub const REFUSAL: &str = "अदेवनागरीवर्णप्रतिषेधः";

/// The letter's Devanagari-8 octet, if it is in the block.
#[must_use]
pub fn octet(ch: char) -> Option<u8> {
    let c = u32::from(ch);
    if (0x0900..=0x097F).contains(&c) {
        u8::try_from(0x80 + (c - 0x0900)).ok()
    } else {
        None
    }
}

/// The letter an octet names. Octets below `0x80` are not Devanagari-8.
#[must_use]
pub fn letter(octet: u8) -> Option<char> {
    if octet < 0x80 {
        return None;
    }
    char::from_u32(0x0900 + u32::from(octet - 0x80))
}

/// The first code point outside the block: its byte offset in `text` and the
/// character itself. `None` when every code point packs.
#[must_use]
pub fn first_outside(text: &str) -> Option<(usize, char)> {
    text.char_indices().find(|(_, c)| octet(*c).is_none())
}

/// The letters packed one octet each, or the first code point that is not a
/// Devanagari letter (byte offset, character).
///
/// # Errors
/// The offset and the character of the first code point outside U+0900–097F.
pub fn pack(text: &str) -> Result<Vec<u8>, (usize, char)> {
    if let Some(bad) = first_outside(text) {
        return Err(bad);
    }
    Ok(text.chars().filter_map(octet).collect())
}

/// The letters a run of Devanagari-8 octets spells, if every octet is one.
#[must_use]
pub fn unpack(octets: &[u8]) -> Option<String> {
    octets.iter().map(|o| letter(*o)).collect()
}

/// A literal token's value as the program reads it: for `वर्णाष्टकम् … इति` the
/// Devanagari-8 octets, for every other string token its UTF-8. `None` for a
/// token that is not a string, or a `वर्णाष्टकम्` literal the lexer would have
/// refused (it never hands one on).
#[must_use]
pub fn literal_octets(token: &Token) -> Option<Vec<u8>> {
    let Kind::Str { value } = &token.kind else {
        return None;
    };
    if is_literal(token) {
        pack(value).ok()
    } else {
        Some(value.as_bytes().to_vec())
    }
}

/// Whether a string token was opened by `वर्णाष्टकम्`. A layout word's text is
/// the word itself and an `उक्तम्` literal's begins `उक्तम्`, so the opener
/// is the whole test.
#[must_use]
pub fn is_literal(token: &Token) -> bool {
    matches!(token.kind, Kind::Str { .. })
        && token
            .text
            .strip_prefix(OPEN)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

/// The octets a UTF-8 text cannot be: a LOOSE well-formedness walk, every lead
/// `C0..FF` followed by its continuation count of `80..BF`. Every genuine UTF-8
/// run passes it, so a run that fails it never came from source text; it is
/// deliberately looser than `str::from_utf8` so the `.t1` twin
/// (`yantrotsarjana.t1`) is the same eight lines.
fn utf8_shaped(octets: &[u8]) -> bool {
    let mut i = 0;
    while i < octets.len() {
        let lead = octets[i];
        if lead < 0xC0 {
            return false;
        }
        let need = if lead >= 0xF0 {
            3
        } else if lead >= 0xE0 {
            2
        } else {
            1
        };
        for k in 1..=need {
            match octets.get(i + k) {
                Some(c) if (0x80..=0xBF).contains(c) => {}
                _ => return false,
            }
        }
        i += need + 1;
    }
    true
}

/// The `वर्णाष्टकम् … इति` spelling of a run of data octets, if that spelling
/// reads back as **exactly** these octets and could not be confused with a
/// run the source wrote as UTF-8 — the emitter's predicate (ADR-0044 D3).
///
/// * **Non-empty, and every octet `0x80..=0xFF`** — a Devanagari-8 octet.
/// * **None of `ऽ । ॥ ॰ ॱ`** (`BD E4 E5 F0 F1`): the same five marks
///   `parse::string_payload` refuses, for the same reasons — the T0 reader
///   peels or cuts at each of them.
/// * **Not shaped like UTF-8.** Every literal written with `उक्तम्` is UTF-8,
///   and its numeral spelling must not change (D6, byte-identical text as well
///   as image), so a run that could be UTF-8 keeps the form it had. A
///   Devanagari-8 run beginning with a letter below U+0940 fails the walk at
///   its first octet.
/// * **Not the letters of `इति`, `आस्की` or `जाल`**, which close the literal
///   early or switch the T0 lexer's mode.
#[must_use]
pub fn payload(octets: &[u8]) -> Option<String> {
    if octets.is_empty() || octets.iter().any(|o| *o < 0x80) {
        return None;
    }
    if octets
        .iter()
        .any(|o| matches!(o, 0xBD | 0xE4 | 0xE5 | 0xF0 | 0xF1))
    {
        return None;
    }
    if utf8_shaped(octets) {
        return None;
    }
    let text = unpack(octets)?;
    if text == "इति" || text == "आस्की" || text == "जाल" {
        return None;
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_code_is_the_block_one_to_one() {
        for c in 0x0900u32..=0x097F {
            let ch = char::from_u32(c).expect("in the BMP");
            let o = octet(ch).expect("in the block");
            assert_eq!(u32::from(o), 0x80 + (c - 0x0900));
            assert_eq!(letter(o), Some(ch));
            // the UTF-8 form of the ADR: 0x80 + (second − 0xA4)×64 + (third − 0x80)
            let mut b = [0u8; 4];
            let u = ch.encode_utf8(&mut b).as_bytes();
            assert_eq!(u.len(), 3);
            assert_eq!(
                u32::from(o),
                0x80 + (u32::from(u[1]) - 0xA4) * 64 + (u32::from(u[2]) - 0x80)
            );
        }
        assert_eq!(octet(' '), None);
        assert_eq!(octet('a'), None);
        assert_eq!(octet('\u{08FF}'), None);
        assert_eq!(octet('\u{0980}'), None);
        assert_eq!(letter(0x7F), None);
    }

    #[test]
    fn the_payload_refuses_utf8_and_the_marks_and_the_three_words() {
        assert_eq!(payload(&pack("कखग").unwrap()), Some("कखग".into()));
        assert_eq!(payload("इति".as_bytes()), None, "UTF-8 keeps its form");
        assert_eq!(payload("क".as_bytes()), None);
        assert_eq!(payload(&pack("इति").unwrap()), None);
        assert_eq!(payload(&pack("आस्की").unwrap()), None);
        assert_eq!(payload(&pack("जाल").unwrap()), None);
        assert_eq!(payload(&pack("कऽ").unwrap()), None);
        assert_eq!(payload(&pack("क।").unwrap()), None);
        assert_eq!(payload(&[]), None);
        assert_eq!(payload(&[0x41]), None);
    }
}
