//! Visarga sandhi — task `B-053`, doc 01 §2.
//!
//! # Visarga is not a sound the sūtras name
//!
//! `ः` appears nowhere in `spec/shiva-sutras.tsv`, and that is correct rather
//! than an omission: it is not a phoneme but what `स्` and `र्` *become* at the
//! end of a word (8.2.66 ससजुषो रुः, 8.3.15 खरवसानयोर्विसर्जनीयः). Every
//! pratyāhāra therefore misses it, and `is_consonant` says no — so it needs its
//! own treatment, which is what this module is.
//!
//! # The rules explain themselves once place is available
//!
//! 8.3.34 विसर्जनीयस्य सः turns visarga back into `स्` before `खर्`, and that
//! `स्` then takes the place of what follows (श्चुत्व, ष्टुत्व). Both steps
//! collapse into one question: **what sibilant is made where the next sound is
//! made?** `varga::sibilant` answers it, and answers `None` for कण्ठ्य and
//! ओष्ठ्य — which is exactly why `कः + कृतम्` keeps its visarga while
//! `कः + च` does not.
//!
//! That absence doing the work is the same shape as `B-052`'s columns: the
//! table is not consulted for a list of exceptions, it is consulted for a place
//! and has nothing to say for two of the five.
//!
//! # What decides the voiced case is what came *before*
//!
//! 6.1.113–114 make `अः` into `ओ` before a voiced sound, and that is the one
//! rule here needing the vowel on the left. `आः` simply loses the visarga, and
//! any other vowel gets `र्`. A caller that does not know the preceding vowel
//! cannot be answered, and is told so rather than given the commonest case.

extern crate alloc;

use crate::phonology::pratyahara;
use crate::varga::{sibilant, stop};

/// What becomes of a visarga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visarga {
    /// It stays as it is.
    Kept,
    /// It becomes this sound.
    Becomes(&'static str),
    /// It disappears, leaving the vowel before it.
    Dropped,
    /// The vowel before it and the visarga together become this.
    ///
    /// `अ` + `ः` before a voiced sound is `ओ` — one sound replacing two, so a
    /// caller splicing a substitute in would produce `अओ`.
    Merges(&'static str),
    /// No rule here governs the pair.
    NotApplicable,
}

/// **विसर्जनीयस्य सः** (8.3.34) with श्चुत्व/ष्टुत्व — visarga before `खर्`.
///
/// Before a stop it becomes the sibilant made at that stop's place, and before
/// `क ख प फ` there is no such sibilant, so it stays. Before a sibilant it
/// assimilates to that one (8.3.36 वा शरि allows either; the assimilated form
/// is what is produced here).
#[must_use]
pub fn before_khar(next: &str) -> Visarga {
    let Some(khar) = pratyahara("ख", "र्") else {
        return Visarga::NotApplicable;
    };
    if !khar.contains(&next) {
        return Visarga::NotApplicable;
    }
    // A sibilant on the right: visarga takes it. `शर्` is inside `खर्`, so this
    // has to be asked before the stop lookup, which would find nothing.
    if let Some(shar) = pratyahara("श", "र्")
        && let Some(same) = shar.into_iter().find(|s| *s == next)
    {
        // The `'static` member, not the caller's slice: a substitute has to
        // outlive the call that asked for it.
        return Visarga::Becomes(same);
    }
    match stop(next).and_then(|s| sibilant(s.varga)) {
        Some(s) => Visarga::Becomes(s),
        // कण्ठ्य and ओष्ठ्य make no sibilant, so there is nothing to become.
        None => Visarga::Kept,
    }
}

/// **अतो रोरप्लुतादप्लुते** and **हशि च** (6.1.113–114) — visarga before a
/// voiced sound, decided by the vowel in front of it.
///
/// - `अ` + `ः` becomes `ओ`: `रामः + गच्छति` is `रामो गच्छति`.
/// - `आ` + `ः` loses the visarga: `रामाः + गच्छन्ति` is `रामा गच्छन्ति`.
/// - any other vowel gets `र्`: `कविः + गच्छति` is `कविर् गच्छति`.
#[must_use]
pub fn before_voiced(previous_vowel: &str, next: &str) -> Visarga {
    let Some(hash) = pratyahara("ह", "श्") else {
        return Visarga::NotApplicable;
    };
    if !hash.contains(&next) {
        return Visarga::NotApplicable;
    }
    let Some(ac) = pratyahara("अ", "च्") else {
        return Visarga::NotApplicable;
    };
    match previous_vowel {
        // गुण: अ and the visarga's उ become ओ, so two sounds become one.
        "अ" => Visarga::Merges("ओ"),
        "आ" => Visarga::Dropped,
        v if ac.contains(&v) || crate::length::dirgha(v) != v => Visarga::Becomes("र्"),
        // Not a vowel at all: nothing here governs it.
        _ => Visarga::NotApplicable,
    }
}

/// Visarga at the end of an utterance — **अवसान**.
///
/// 8.3.15 puts visarga at a pause, and there is nothing after it to react to,
/// so it stays. Stated so a caller need not treat end-of-input as a special
/// case it invents itself.
#[must_use]
pub fn at_pause() -> Visarga {
    Visarga::Kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visarga_is_not_in_the_sutras_at_all() {
        // The premise of this module. If visarga ever appears in the table,
        // every pratyāhāra changes and these rules are the wrong shape.
        let ac = pratyahara("अ", "च्").expect("अच्");
        let hal = pratyahara("ह", "ल्").expect("हल्");
        assert!(!ac.contains(&"ः"), "not a vowel");
        assert!(!hal.contains(&"ः"), "and not a consonant");
        assert!(!crate::phonology::is_consonant("ः"));
    }

    #[test]
    fn before_a_stop_it_becomes_that_place_s_sibilant() {
        // 8.3.34 plus assimilation, in one question: what sibilant is made
        // where the next sound is made?
        assert_eq!(before_khar("च"), Visarga::Becomes("श"), "तालव्य");
        assert_eq!(before_khar("छ"), Visarga::Becomes("श"));
        assert_eq!(before_khar("ट"), Visarga::Becomes("ष"), "मूर्धन्य");
        assert_eq!(before_khar("त"), Visarga::Becomes("स"), "दन्त्य");
        assert_eq!(before_khar("थ"), Visarga::Becomes("स"));
    }

    #[test]
    fn before_a_velar_or_labial_it_survives() {
        // Not an exception list: कण्ठ्य and ओष्ठ्य make no sibilant, so there is
        // nothing for the visarga to become. `कः कृतम्`, `कः प्रियः`.
        assert_eq!(before_khar("क"), Visarga::Kept);
        assert_eq!(before_khar("ख"), Visarga::Kept);
        assert_eq!(before_khar("प"), Visarga::Kept);
        assert_eq!(before_khar("फ"), Visarga::Kept);
    }

    #[test]
    fn the_voiced_case_is_decided_by_the_vowel_in_front() {
        // रामः + गच्छति → रामो गच्छति; रामाः + गच्छन्ति → रामा गच्छन्ति;
        // कविः + गच्छति → कविर् गच्छति.
        assert_eq!(before_voiced("अ", "ग"), Visarga::Merges("ओ"));
        assert_eq!(before_voiced("आ", "ग"), Visarga::Dropped);
        assert_eq!(before_voiced("इ", "ग"), Visarga::Becomes("र्"));
        assert_eq!(before_voiced("उ", "म"), Visarga::Becomes("र्"));
    }

    #[test]
    fn the_two_rules_govern_disjoint_sounds() {
        // खर् is unvoiced and हश् is voiced, so no sound is answered twice —
        // which is what lets a caller try one and then the other.
        let khar = pratyahara("ख", "र्").expect("खर्");
        let hash = pratyahara("ह", "श्").expect("हश्");
        for k in &khar {
            assert!(!hash.contains(k), "{k} is in both classes");
        }
        assert_eq!(before_voiced("अ", "क"), Visarga::NotApplicable);
        assert_eq!(before_khar("ग"), Visarga::NotApplicable);
    }

    #[test]
    fn a_caller_who_cannot_say_what_preceded_is_refused() {
        // Answering `अ` by default would be right most of the time, which is
        // the failure mode every defect in this project has had.
        assert_eq!(before_voiced("क", "ग"), Visarga::NotApplicable);
    }
}
