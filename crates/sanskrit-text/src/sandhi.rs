//! Sandhi, quantified over pratyāhāras — task `B-050`.
//!
//! ADR-0004 and ADR-0005 were both sandhi problems and both were solved ad hoc,
//! the second with its mechanism stated backwards. This module exists so the
//! next one is stated as a rule instead of discovered from failures.
//!
//! # The rule this implements, and why it needs no table
//!
//! **इको यणचि** (Aṣṭādhyāyī 6.1.77): *before a vowel, इक् becomes यण्.*
//!
//! Both names are pratyāhāras, so the whole rule is four words, and the two
//! classes come out of the ordering the same length in the same order:
//!
//! | `इक्` | `इ` | `उ` | `ऋ` | `ऌ` |
//! |---|---|---|---|---|
//! | `यण्` | `य` | `व` | `र` | `ल` |
//!
//! That alignment is Pāṇini's यथासंख्यम् — when two lists meet, they correspond
//! in order — and it means **the substitution map is not written down anywhere**.
//! `इ` becomes `य` because both sit at index 0 of their class. Change the
//! ordering in `spec/shiva-sutras.tsv` and the substitution changes with it.
//!
//! This is the clearest evidence for doc 19's thesis available in the codebase:
//! the classes and the mapping between them are both consequences of one
//! ordering, and neither is data anyone maintains.
//!
//! # What this module refuses to do
//!
//! Sandhi is large, and the parts not implemented are **declined explicitly**
//! rather than approximated. [`Sandhi::Deferred`] names the sūtra that governs
//! a pair this module cannot yet handle, so a caller gets a refusal that says
//! why instead of a plausible wrong answer.
//!
//! The important case was savarṇa, and `B-051` closed it. **अकः सवर्णे दीर्घः**
//! (6.1.101) takes precedence over 6.1.77: `इ + इ` is `ई`, not `य् + इ`.
//! Detecting savarṇa needs vowel *length*, which is not in the sūtras at all —
//! `अ` stands for both `अ` and `आ` by convention, which is why the table has
//! nine vowels and not fifteen. [`crate::length`] derives that from the pinned
//! UCD, and this module now defers to it rather than declining.
//!
//! Consonant sandhi (`B-052`) and visarga sandhi (`B-053`) are untouched.

extern crate alloc;

use alloc::vec::Vec;

use crate::length::akah_savarne_dirghah;
use crate::phonology::pratyahara;

/// What a sandhi rule says about a pair of sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandhi {
    /// The rule applies, and the first sound becomes this.
    Substitute(&'static str),
    /// This rule does not govern the pair at all.
    NotApplicable,
    /// Another rule governs and is not implemented. Carries the sūtra.
    ///
    /// A refusal, never a guess: a wrong join produces a real word that is the
    /// wrong word, which is the failure mode every defect in this project so
    /// far has had.
    Deferred(&'static str),
}

/// The यण् substitute for an इक् vowel, by position in the two classes.
///
/// Returns `None` for anything not in `इक्`.
#[must_use]
pub fn yan_of(vowel: &str) -> Option<&'static str> {
    let ik = pratyahara("इ", "क्")?;
    let yan = pratyahara("य", "ण्")?;
    // If these ever differ in length the correspondence is undefined and
    // guessing an alignment would be inventing a rule.
    if ik.len() != yan.len() {
        return None;
    }
    ik.iter().position(|v| *v == vowel).map(|i| yan[i])
}

/// **इको यणचि** (6.1.77) — before a vowel, `इक्` becomes `यण्`.
///
/// `left` and `right` are single phonemes written in Devanagari, as they appear
/// in `spec/shiva-sutras.tsv`.
///
/// ```ignore
/// // दधि + अत्र → दध्यत्र
/// assert_eq!(iko_yan_aci("इ", "अ"), Sandhi::Substitute("य"));
/// ```
#[must_use]
pub fn iko_yan_aci(left: &str, right: &str) -> Sandhi {
    let Some(ac) = pratyahara("अ", "च्") else {
        return Sandhi::NotApplicable;
    };
    // `अचि` — the rule only fires before a vowel.
    if !ac.contains(&right) {
        return Sandhi::NotApplicable;
    }
    let Some(sub) = yan_of(left) else {
        return Sandhi::NotApplicable; // not an इक् vowel
    };
    // 6.1.101 is the earlier and stronger rule and now answers for itself
    // (`B-051`). It fires on savarṇa vowels of ANY lengths, which is more than
    // the identity test this used to defer on: `इ + ई` is savarṇa and is `ई`,
    // and the old check would have sent it to यण् and produced `य्`.
    if let Some(long) = akah_savarne_dirghah(left, right) {
        return Sandhi::Substitute(long);
    }
    Sandhi::Substitute(sub)
}

/// Every pair this module can currently resolve, for inspection and testing.
///
/// Useful as a census: a rule that silently governs nothing is worse than one
/// that is absent, and this makes the coverage countable.
#[must_use]
pub fn resolvable_pairs() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut out = Vec::new();
    let (Some(ik), Some(ac)) = (pratyahara("इ", "क्"), pratyahara("अ", "च्")) else {
        return out;
    };
    for l in &ik {
        for r in &ac {
            if let Sandhi::Substitute(s) = iko_yan_aci(l, r) {
                out.push((*l, *r, s));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_substitution_is_index_alignment_and_nothing_else() {
        // यथासंख्यम्: two classes of the same length correspond in order. The
        // map below is not stored anywhere — it is read off the ordering.
        assert_eq!(yan_of("इ"), Some("य"));
        assert_eq!(yan_of("उ"), Some("व"));
        assert_eq!(yan_of("ऋ"), Some("र"));
        assert_eq!(yan_of("ऌ"), Some("ल"));
        assert_eq!(yan_of("अ"), None, "अ is not in इक्");
        assert_eq!(yan_of("ए"), None, "ए is not in इक्");
        assert_eq!(yan_of("क"), None, "not a vowel at all");
    }

    #[test]
    fn the_classical_examples_come_out_right() {
        // Attested joins, each named. These are authored expectations, so they
        // are here to be RUN rather than believed — the same discipline the
        // rejection corpus needed when three of its twelve were wrong.
        for (l, r, want, gloss) in [
            ("इ", "अ", "य", "दधि + अत्र → दध्यत्र"),
            ("उ", "अ", "व", "मधु + अत्र → मध्वत्र"),
            ("ऋ", "अ", "र", "पितृ + अर्थम् → पित्रर्थम्"),
            ("इ", "ए", "य", "दधि + एव → दध्येव"),
            ("उ", "ओ", "व", "मधु + ओदनम् → मध्वोदनम्"),
        ] {
            assert_eq!(
                iko_yan_aci(l, r),
                Sandhi::Substitute(want),
                "{gloss}: {l} + {r}"
            );
        }
    }

    #[test]
    fn a_rule_that_does_not_govern_says_so() {
        // अ is not in इक्; 6.1.87 आद्गुणः governs and gives ए. Answering here
        // would be inventing a rule.
        assert_eq!(iko_yan_aci("अ", "इ"), Sandhi::NotApplicable);
        // ए is in एच्; 6.1.78 governs.
        assert_eq!(iko_yan_aci("ए", "अ"), Sandhi::NotApplicable);
        // A consonant on the right: `अचि` is not satisfied.
        assert_eq!(iko_yan_aci("इ", "क"), Sandhi::NotApplicable);
    }

    #[test]
    fn like_vowels_lengthen_rather_than_becoming_yan() {
        // इ + इ is ई by 6.1.101, not य् + इ. This test used to assert a
        // REFUSAL, which was the honest answer while length was unknown;
        // `B-051` derived length from the pinned UCD and the refusal became an
        // answer. A deferral is a promise to come back, and this is it.
        for v in ["इ", "उ", "ऋ", "ऌ"] {
            let long = crate::length::dirgha(v);
            assert_eq!(
                iko_yan_aci(v, v),
                Sandhi::Substitute(long),
                "{v} + {v} is the long {long}, not a यण् semivowel"
            );
        }
    }

    #[test]
    fn the_coverage_is_countable() {
        // 4 इक् vowels x 9 अच् vowels, and `B-051` closed the last 4: the
        // savarṇa pairs that used to defer now answer, so every pair in the
        // cross product resolves.
        let pairs = resolvable_pairs();
        assert_eq!(pairs.len(), 4 * 9, "coverage changed");
        // Every substitution is a यण् member or a lengthening, never anything
        // else. The two rules answer for disjoint pairs.
        let yan = pratyahara("य", "ण्").expect("derives");
        for (l, _, s) in &pairs {
            let long = crate::length::dirgha(l);
            assert!(yan.contains(s) || *s == long, "{s} is neither यण् nor दीर्घ");
        }
    }
}
