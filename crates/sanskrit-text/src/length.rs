//! Vowel length and savarṇa — task `B-051`, doc 01 §2.
//!
//! # The one thing the sūtras do not say
//!
//! Pāṇini's table has **nine** vowels, not fifteen, because a sūtra's `अ`
//! stands for `अ` and `आ` alike (1.1.69 अणुदित्सवर्णस्य चाप्रत्ययः). Every rule
//! quantified over pratyāhāras is the better for it — `इको यणचि` is four words
//! precisely because it does not have to say "and also the long ones".
//!
//! **अकः सवर्णे दीर्घः** (6.1.101) is where that ends. It says the result *is
//! the long one*, so something must know which vowel that is, and the sūtras
//! are silent by design.
//!
//! # Where the answer comes from
//!
//! Not from a table someone typed. `spec/vowel-length.tsv` is derived from the
//! pinned UCD by `tools/gen-vowel-length.py`: the long vowel's Unicode **name**
//! is the short one's with its final letter doubled — A→AA, I→II, U→UU,
//! VOCALIC R→VOCALIC RR, VOCALIC L→VOCALIC LL.
//!
//! Deriving it arithmetically from code points is the trap. `अ आ`, `इ ई` and
//! `उ ऊ` are adjacent, so the rule looks like "+1" — and `ऋ ॠ` are U+090B and
//! U+0960, `ऌ ॡ` are U+090C and U+0961. A rule right for three of five pairs
//! is worse than no rule, because it works on everything anyone tests first.
//!
//! # The two sources check each other
//!
//! The short vowels derived from Unicode must be **exactly pratyāhāra अक्**,
//! which comes from `spec/shiva-sutras.tsv` and knows nothing about Unicode.
//! Those five are the vowels that have a long counterpart, and they are the
//! class 6.1.101 governs — that is not a coincidence to be admired, it is an
//! assertion in [`tests`].

extern crate alloc;

use alloc::vec::Vec;

use crate::phonology::pratyahara;

/// The derived length table.
const LENGTHS: &str = include_str!("../../../spec/vowel-length.tsv");

/// Every `(hrasva, dīrgha)` pair, short first.
#[must_use]
pub fn pairs() -> Vec<(&'static str, &'static str)> {
    LENGTHS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("hrasva\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?, f.next()?))
        })
        .collect()
}

/// The दीर्घ (long) counterpart of a vowel.
///
/// A vowel that is already long is its own answer, which is what makes
/// [`akah_savarne_dirghah`] work for `आ + अ` as well as `अ + अ`. `ए ऐ ओ औ` have
/// no short form and are returned unchanged.
#[must_use]
pub fn dirgha(vowel: &str) -> &str {
    for (short, long) in pairs() {
        if vowel == short || vowel == long {
            return long;
        }
    }
    vowel
}

/// Whether a vowel is short.
#[must_use]
pub fn is_hrasva(vowel: &str) -> bool {
    pairs().iter().any(|(short, _)| *short == vowel)
}

/// **सवर्ण** — homogeneous: same place and same effort (1.1.9).
///
/// For vowels that is the same quality regardless of length, which is exactly
/// what the length table says. `इ` and `ई` are savarṇa; `इ` and `उ` are not.
///
/// This is the vowel case only. Consonant savarṇa needs स्थान and प्रयत्न, which
/// are `B-052`'s business, and claiming otherwise here would answer a question
/// this module cannot see.
#[must_use]
pub fn savarna(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    pairs()
        .iter()
        .any(|(short, long)| (a == *short && b == *long) || (a == *long && b == *short))
}

/// **अकः सवर्णे दीर्घः** (6.1.101) — `अक्` before a savarṇa vowel becomes long.
///
/// Returns `None` when the rule does not govern the pair, so a caller can fall
/// through to the next rule rather than receive a guess. `इ + इ` is `ई`;
/// `इ + अ` is not this rule's business and is `इको यणचि`'s.
#[must_use]
pub fn akah_savarne_dirghah(left: &str, right: &str) -> Option<&'static str> {
    // `अकः` — the rule is stated of the अक् class, and by 1.1.69 that class
    // covers both lengths of each of its five vowels.
    let ak = pratyahara("अ", "क्")?;
    let governed = ak
        .iter()
        .any(|v| *v == left || dirgha(v) == left || *v == dirgha(left));
    if !governed || !savarna(left, right) {
        return None;
    }
    // `दीर्घः` — the result is the long one, whichever lengths came in.
    pairs()
        .iter()
        .find(|(short, long)| left == *short || left == *long)
        .map(|(_, long)| *long)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_derived_short_vowels_are_exactly_pratyahara_ak() {
        // Two sources that share nothing: `spec/vowel-length.tsv` comes from
        // Unicode character names, `अक्` from the Śiva sūtras. They must name
        // the same five vowels, because the class 6.1.101 governs is precisely
        // the class whose members have a long form.
        //
        // This is the check that replaces a repertoire filter which silently
        // passed everything — see the note in tools/gen-vowel-length.py.
        let ak = pratyahara("अ", "क्").expect("अक् is a pratyāhāra");
        let mut derived: Vec<&str> = pairs().iter().map(|(short, _)| *short).collect();
        let mut expected: Vec<&str> = ak.to_vec();
        derived.sort_unstable();
        expected.sort_unstable();
        assert_eq!(derived, expected, "Unicode and the sūtras disagree");
        assert_eq!(derived.len(), 5, "nine vowels, five with a long form");
    }

    #[test]
    fn length_is_not_arithmetic_on_code_points() {
        // The trap this module exists to avoid. Three pairs are adjacent code
        // points and two are not, so "+1" is right often enough to ship.
        let adjacent = |a: &str, b: &str| {
            let (a, b) = (
                a.chars().next().expect("char") as u32,
                b.chars().next().expect("char") as u32,
            );
            b == a + 1
        };
        assert!(adjacent("अ", dirgha("अ")));
        assert!(adjacent("इ", dirgha("इ")));
        assert!(adjacent("उ", dirgha("उ")));
        assert!(!adjacent("ऋ", dirgha("ऋ")), "U+090B and U+0960");
        assert!(!adjacent("ऌ", dirgha("ऌ")), "U+090C and U+0961");
    }

    #[test]
    fn savarna_is_quality_not_identity() {
        assert!(savarna("इ", "ई"), "same vowel, different length");
        assert!(savarna("ई", "इ"), "and the other way round");
        assert!(savarna("इ", "इ"));
        assert!(!savarna("इ", "उ"), "different quality");
        assert!(!savarna("अ", "ए"), "ए is not a lengthened अ");
    }

    #[test]
    fn like_vowels_join_long_at_every_combination_of_lengths() {
        // 6.1.101's whole content: whichever lengths came in, the result is
        // long. `विद्या + आलयः` is `विद्यालयः` and `दैत्य + अरिः` is `दैत्यारिः`.
        for (short, long) in pairs() {
            for left in [short, long] {
                for right in [short, long] {
                    assert_eq!(
                        akah_savarne_dirghah(left, right),
                        Some(long),
                        "{left} + {right}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_rule_declines_what_it_does_not_govern() {
        // Not savarṇa: this is `इको यणचि`'s pair, and answering it here would
        // produce a real word that is the wrong word.
        assert_eq!(akah_savarne_dirghah("इ", "अ"), None);
        // `ए` is in अच् but not अक् and has no long form to become.
        assert_eq!(akah_savarne_dirghah("ए", "ए"), None);
    }
}
