//! Unicode Normalization Forms C and D (UAX #15).
//!
//! SANSOS stores, transmits, and hashes every source file as UTF-8 in **NFC**
//! (doc 01 D-01-B). Two visually identical identifiers must be one identifier,
//! and byte-identical builds (doc 03 §6, doc 16 §3.3) require one canonical
//! spelling — so this runs before the lexer sees anything.
//!
//! Only the canonical forms are implemented. NFKC/NFKD fold distinctions that
//! matter in source code (superscripts, ligatures, width variants) and have no
//! use here.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::lookup;
use crate::tables::{CANONICAL_COMPOSE, CANONICAL_DECOMP, CCC};

// Hangul syllables decompose and compose arithmetically rather than by table
// (UAX #15 §16), which is why they are absent from CANONICAL_DECOMP.
const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT; // 588
const S_COUNT: u32 = L_COUNT * N_COUNT; // 11172

/// Canonical combining class of a codepoint. 0 for starters.
#[must_use]
pub fn ccc(ch: char) -> u8 {
    lookup(CCC, ch as u32, 0)
}

fn canonical_decomp(cp: u32) -> Option<&'static [u32]> {
    CANONICAL_DECOMP
        .binary_search_by_key(&cp, |&(c, _)| c)
        .ok()
        .map(|i| CANONICAL_DECOMP[i].1)
}

fn compose_pair(a: char, b: char) -> Option<char> {
    let (a, b) = (a as u32, b as u32);

    // Hangul: L + V, then LV + T.
    if (L_BASE..L_BASE + L_COUNT).contains(&a) && (V_BASE..V_BASE + V_COUNT).contains(&b) {
        let s = S_BASE + ((a - L_BASE) * V_COUNT + (b - V_BASE)) * T_COUNT;
        return char::from_u32(s);
    }
    if (S_BASE..S_BASE + S_COUNT).contains(&a)
        && (a - S_BASE).is_multiple_of(T_COUNT)
        && (T_BASE + 1..T_BASE + T_COUNT).contains(&b)
    {
        return char::from_u32(a + (b - T_BASE));
    }

    CANONICAL_COMPOSE
        .binary_search_by_key(&(a, b), |&(x, y, _)| (x, y))
        .ok()
        .and_then(|i| char::from_u32(CANONICAL_COMPOSE[i].2))
}

fn decompose_into(ch: char, out: &mut Vec<char>) {
    let cp = ch as u32;

    if (S_BASE..S_BASE + S_COUNT).contains(&cp) {
        let i = cp - S_BASE;
        let (l, v, t) = (
            L_BASE + i / N_COUNT,
            V_BASE + (i % N_COUNT) / T_COUNT,
            T_BASE + i % T_COUNT,
        );
        out.extend(char::from_u32(l));
        out.extend(char::from_u32(v));
        if !i.is_multiple_of(T_COUNT) {
            out.extend(char::from_u32(t));
        }
        return;
    }

    // Mappings are stored unexpanded, so recurse. Unicode guarantees this
    // terminates well within a handful of levels.
    if let Some(seq) = canonical_decomp(cp) {
        for &c in seq {
            if let Some(c) = char::from_u32(c) {
                decompose_into(c, out);
            }
        }
        return;
    }

    out.push(ch);
}

/// Canonical ordering (UAX #15 D108): stably sort each run of non-starters by
/// combining class. Insertion sort — runs are short, and stability is required.
fn canonical_order(buf: &mut [char]) {
    for i in 1..buf.len() {
        let cc = ccc(buf[i]);
        if cc == 0 {
            continue;
        }
        let mut j = i;
        while j > 0 && ccc(buf[j - 1]) > cc {
            buf.swap(j - 1, j);
            j -= 1;
        }
    }
}

/// Normalization Form D — canonical decomposition.
#[must_use]
pub fn nfd(s: &str) -> String {
    let mut buf = Vec::with_capacity(s.len());
    for ch in s.chars() {
        decompose_into(ch, &mut buf);
    }
    canonical_order(&mut buf);
    buf.into_iter().collect()
}

/// Normalization Form C — canonical decomposition followed by canonical
/// composition. **The form every SANSOS source file must be stored in.**
#[must_use]
pub fn nfc(s: &str) -> String {
    let decomposed = nfd(s);
    let mut out: Vec<char> = Vec::with_capacity(decomposed.len());

    // Index of the starter a following character may compose onto, and the
    // combining class of the most recent character appended after it. `None`
    // means nothing intervenes, so the next character is not blocked.
    let mut starter: Option<usize> = None;
    let mut prev_ccc: Option<u8> = None;

    for ch in decomposed.chars() {
        let cc = ccc(ch);

        // Blocked (UAX #15 D115) unless nothing intervenes or the intervening
        // class is strictly lower.
        if let Some(sp) = starter
            && prev_ccc.is_none_or(|p| p < cc)
            && let Some(composed) = compose_pair(out[sp], ch)
        {
            out[sp] = composed;
            continue;
        }

        if cc == 0 {
            starter = Some(out.len());
            prev_ccc = None;
        } else {
            prev_ccc = Some(cc);
        }
        out.push(ch);
    }

    out.into_iter().collect()
}

/// Whether `s` is already NFC. Cheaper to call than comparing to [`nfc`] only
/// in intent — the assembler rejects non-NFC source rather than fixing it
/// silently (doc 01 D-01-B), so this is the gate, not a repair.
#[must_use]
pub fn is_nfc(s: &str) -> bool {
    nfc(s) == s
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn latin_precomposed_round_trips() {
        assert_eq!(nfd("\u{00C0}"), "\u{0041}\u{0300}"); // À -> A + grave
        assert_eq!(nfc("\u{0041}\u{0300}"), "\u{00C0}");
    }

    #[test]
    fn devanagari_nukta_is_a_composition_exclusion() {
        // The case doc 01 D-01-B exists for: क़ has a precomposed form U+0958,
        // but NFC must leave it DECOMPOSED. Getting this backwards would make
        // two byte-different spellings of one identifier both look canonical.
        let precomposed = "\u{0958}";
        let decomposed = "\u{0915}\u{093C}";
        assert_eq!(nfd(precomposed), decomposed);
        assert_eq!(nfc(precomposed), decomposed, "U+0958 must NOT recompose");
        assert_eq!(nfc(decomposed), decomposed);
        assert!(!is_nfc(precomposed));
        assert!(is_nfc(decomposed));
    }

    #[test]
    fn devanagari_conjunct_is_unchanged_by_nfc() {
        // क्ष = क + virama + ष. No canonical mapping applies; the conjunct is a
        // rendering matter, not a normalization one.
        let ksha = "\u{0915}\u{094D}\u{0937}";
        assert_eq!(nfc(ksha), ksha);
        assert_eq!(nfd(ksha), ksha);
    }

    #[test]
    fn purpose_invocation_is_nfc() {
        // Test vector #1 from doc 01 §5.
        let invocation = "\u{0950} परम तत्वयाय नारायणाय गुरुभ्यो नमः";
        assert!(is_nfc(invocation));
        assert_eq!(nfc(invocation), invocation);
    }

    #[test]
    fn combining_marks_are_canonically_ordered() {
        // ccc(0323)=220 below, ccc(0307)=230 above: below sorts first.
        assert_eq!(ccc('\u{0323}'), 220);
        assert_eq!(ccc('\u{0307}'), 230);
        assert_eq!(nfd("\u{0041}\u{0307}\u{0323}"), "\u{0041}\u{0323}\u{0307}");
    }

    #[test]
    fn hangul_is_algorithmic() {
        assert_eq!(nfd("\u{AC01}"), "\u{1100}\u{1161}\u{11A8}");
        assert_eq!(nfc("\u{1100}\u{1161}\u{11A8}"), "\u{AC01}");
        assert_eq!(nfd("\u{AC00}"), "\u{1100}\u{1161}");
        assert_eq!(nfc("\u{1100}\u{1161}"), "\u{AC00}");
    }

    #[test]
    fn singleton_decomposition_does_not_recompose() {
        // U+2126 OHM SIGN -> U+03A9 GREEK CAPITAL OMEGA, and stays there.
        assert_eq!(nfc("\u{2126}"), "\u{03A9}".to_string());
    }

    #[test]
    fn empty_and_ascii_are_identity() {
        assert_eq!(nfc(""), "");
        assert_eq!(nfd(""), "");
        assert_eq!(nfc("abc"), "abc");
    }
}
