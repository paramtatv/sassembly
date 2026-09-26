//! Consonant sandhi — task `B-052`, doc 01 §2.
//!
//! # स्थान is the one thing nothing here can derive
//!
//! `B-051` found vowel length missing from the sūtras and recovered it from the
//! Unicode character names. Place of articulation is the same shape of gap and
//! does **not** yield to the same trick. Four sources were tried and
//! `spec/varga.tsv` records each failure:
//!
//! - the **Śiva sūtras** order the stops four different ways, so यथासंख्यम् —
//!   which gave `इ→य` for free — aligns झष् with जश् and nothing else;
//! - **code points** look like five rows of five until `ऩ` (U+0929) turns up
//!   between `न` and `प` and shifts the whole labial varga by one;
//! - **Unicode names** give aspiration (`KA`→`KHA`) and never voicing;
//! - the **repertoire** admits `ऩ` too.
//!
//! `ParamTatva`'s kernel was checked as well: `ptk.v1.json` carries the same 57
//! nodes as `spec/shiva-sutras.tsv` with a `features` array on each, and every
//! one of those arrays is empty. It models the sūtras' *connectivity*, which
//! this crate already derives, and not articulation.
//!
//! So the table is authored, once, and every column of it is checked against a
//! pratyāhāra that knows nothing about place.
//!
//! # What a varga buys
//!
//! Consonant sandhi is stated as "become the जश् / चर् one", and both classes
//! hold one member per place. So every rule below is the same move — **keep the
//! varga, change the column** — and none of them needs a substitution table.

extern crate alloc;

use alloc::vec::Vec;

use crate::phonology::pratyahara;

/// The authored वर्ग table.
const VARGA: &str = include_str!("../../../spec/varga.tsv");

/// One stop, with its place and its manner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop {
    /// The consonant, without a virama.
    pub aksara: &'static str,
    /// Which वर्ग: 0 कण्ठ्य, 1 तालव्य, 2 मूर्धन्य, 3 दन्त्य, 4 ओष्ठ्य.
    pub varga: u8,
    /// Position within it: 0 and 1 unvoiced, 2 and 3 voiced, 4 nasal.
    pub column: u8,
}

/// Every stop in the table.
#[must_use]
pub fn stops() -> Vec<Stop> {
    VARGA
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("aksara\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Some(Stop {
                aksara: f.first()?,
                varga: f.get(1)?.parse().ok()?,
                column: f.get(2)?.parse().ok()?,
            })
        })
        .collect()
}

/// The stop record for a consonant, if it is one of the 25.
#[must_use]
pub fn stop(aksara: &str) -> Option<Stop> {
    stops().into_iter().find(|s| s.aksara == aksara)
}

/// The member of `aksara`'s own varga sitting in `column`.
///
/// This is the whole mechanism: a sandhi rule names a class, each class is one
/// column, and the substitute is the letter where that column meets the place
/// the sound already had.
#[must_use]
pub fn in_column(aksara: &str, column: u8) -> Option<&'static str> {
    let s = stop(aksara)?;
    stops()
        .into_iter()
        .find(|t| t.varga == s.varga && t.column == column)
        .map(|t| t.aksara)
}

/// जश् — the voiced unaspirated column.
const JHASH: u8 = 2;
/// चर् — the unvoiced unaspirated column.
const CHAR: u8 = 0;

/// **झलां जशोऽन्ते** (8.2.39) — a final झल् becomes जश्.
///
/// `वाक्` + nothing is `वाग्`. Returns `None` for anything not a stop: `श ष स ह`
/// are in झल् and have no varga, and they are governed by other sūtras rather
/// than by this one.
#[must_use]
pub fn jhalam_jasho_ante(final_sound: &str) -> Option<&'static str> {
    governed_by_jhal(final_sound)?;
    in_column(final_sound, JHASH)
}

/// **झलां जश् झशि** (8.4.53) — झल् before झश् becomes जश्.
///
/// `तद्` + `हितम्`: the `द्` is already जश् and stays. `अच्` + `अन्तः` gives
/// `अजन्तः`.
#[must_use]
pub fn jhalam_jash_jhashi(left: &str, right: &str) -> Option<&'static str> {
    governed_by_jhal(left)?;
    let jhash = pratyahara("झ", "श्")?;
    jhash.contains(&right).then(|| in_column(left, JHASH))?
}

/// **खरि च** (8.4.55) — झल् before खर् becomes चर्.
///
/// The unvoicing rule: `तद्` + `कालः` is `तत्कालः`. Same move as 8.4.53 with a
/// different column, which is the point of holding the table this way.
#[must_use]
pub fn khari_ca(left: &str, right: &str) -> Option<&'static str> {
    governed_by_jhal(left)?;
    let khar = pratyahara("ख", "र्")?;
    khar.contains(&right).then(|| in_column(left, CHAR))?
}

/// The sibilant made at a वर्ग's place, if that place has one.
///
/// `शर्` is `श ष स` — palatal, retroflex, dental — which is वर्ग 1, 2, 3 in
/// order, so the pratyāhāra supplies the mapping once you know it begins at
/// तालव्य. That anchor is the authored part and it is one fact, not three.
///
/// **कण्ठ्य and ओष्ठ्य have no sibilant**, and that absence is not a gap in the
/// table — it is the reason visarga survives before `क ख` and `प फ` while
/// turning into one before every other stop (`B-053`).
#[must_use]
pub fn sibilant(varga: u8) -> Option<&'static str> {
    let shar = pratyahara("श", "र्")?;
    (1..=3)
        .contains(&varga)
        .then(|| shar.get(varga as usize - 1).copied())?
}

/// Whether a sound is in झल् at all, which is what all three rules are stated of.
fn governed_by_jhal(sound: &str) -> Option<()> {
    pratyahara("झ", "ल्")?.contains(&sound).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stops of a column, in table order.
    fn column(n: u8) -> Vec<&'static str> {
        stops()
            .into_iter()
            .filter(|s| s.column == n)
            .map(|s| s.aksara)
            .collect()
    }

    fn sorted(mut v: Vec<&'static str>) -> Vec<&'static str> {
        v.sort_unstable();
        v
    }

    #[test]
    fn every_column_is_a_pratyahara_the_table_never_saw() {
        // The authored table's only defence. `spec/varga.tsv` knows about place
        // and the Śiva sūtras know about ordering, and they share nothing — so
        // a single letter in the wrong varga breaks at least one of these.
        assert_eq!(
            sorted(column(2)),
            sorted(pratyahara("ज", "श्").expect("जश्")),
            "column 2 is जश्"
        );
        assert_eq!(
            sorted(column(3)),
            sorted(pratyahara("झ", "ष्").expect("झष्")),
            "column 3 is झष्"
        );
        // चर् and खय् run past the stops into the sibilants, so the first five
        // of each is what a column can equal.
        let first_five = |mut v: Vec<&'static str>| {
            v.truncate(5);
            sorted(v)
        };
        assert_eq!(
            sorted(column(0)),
            first_five(pratyahara("च", "र्").expect("चर्")),
            "column 0 opens चर्"
        );
        assert_eq!(
            sorted(column(1)),
            first_five(pratyahara("ख", "य्").expect("खय्")),
            "column 1 opens खय्"
        );
        // The nasals are the tail of यम्, not its head.
        let yam = pratyahara("य", "म्").expect("यम्");
        assert_eq!(
            sorted(column(4)),
            sorted(yam[4..].to_vec()),
            "column 4 is the nasals"
        );
    }

    #[test]
    fn the_table_holds_five_places_of_five() {
        assert_eq!(stops().len(), 25);
        for v in 0..5u8 {
            assert_eq!(stops().iter().filter(|s| s.varga == v).count(), 5);
        }
    }

    #[test]
    fn the_sibilants_line_up_with_the_places_that_have_one() {
        // शर् is three long and the three places that make a sibilant are
        // consecutive vargas, so this is index alignment with one anchor.
        assert_eq!(pratyahara("श", "र्").expect("शर्").len(), 3);
        assert_eq!(sibilant(1), Some("श"), "तालव्य");
        assert_eq!(sibilant(2), Some("ष"), "मूर्धन्य");
        assert_eq!(sibilant(3), Some("स"), "दन्त्य");
        // The absence is the interesting half: it is why visarga survives
        // before क ख and प फ.
        assert_eq!(sibilant(0), None, "कण्ठ्य has no sibilant");
        assert_eq!(sibilant(4), None, "ओष्ठ्य has none either");
    }

    #[test]
    fn a_final_stop_is_voiced() {
        // 8.2.39. वाक् → वाग्, and the place never moves.
        assert_eq!(jhalam_jasho_ante("क"), Some("ग"));
        assert_eq!(jhalam_jasho_ante("त"), Some("द"));
        assert_eq!(jhalam_jasho_ante("ध"), Some("द"), "aspiration goes too");
        assert_eq!(jhalam_jasho_ante("द"), Some("द"), "already जश्");
    }

    #[test]
    fn a_stop_takes_the_voicing_of_what_follows() {
        // 8.4.53 voices before a voiced sound, 8.4.55 unvoices before an
        // unvoiced one, and neither consults a substitution table.
        assert_eq!(jhalam_jash_jhashi("त", "ध"), Some("द"));
        assert_eq!(khari_ca("द", "क"), Some("त"));
        assert_eq!(khari_ca("ग", "त"), Some("क"));
    }

    #[test]
    fn the_rules_decline_what_they_do_not_govern() {
        // `श ष स ह` are in झल् and have no varga. Answering for them would
        // invent a place, which is the one thing the table must not do.
        assert_eq!(jhalam_jasho_ante("श"), None);
        assert_eq!(jhalam_jasho_ante("ह"), None);
        // A vowel is not in झल् at all.
        assert_eq!(jhalam_jasho_ante("अ"), None);
        // 8.4.55 governs only what stands before खर्; `ध` is in झश्.
        assert_eq!(khari_ca("द", "ध"), None);
        assert_eq!(jhalam_jash_jhashi("त", "क"), None);
    }
}
