//! Pratyāhāras, derived from the Māheśvara sūtras — task `B-046`.
//!
//! A pratyāhāra is Pāṇini's device for naming a set of sounds in two syllables.
//! Take the fourteen sūtras as one ordered sequence, start at a phoneme, run
//! forward, stop at a named boundary marker, and discard the markers passed on
//! the way. `अच्` is everything from `अ` to the marker `च्` — the nine vowels.
//! `हल्` is everything from `ह` to the marker `ल्` — every consonant there is.
//!
//! The sets are not listed anywhere in this module. They are computed from
//! [`spec/shiva-sutras.tsv`] by the rule above, which is the point: the sūtras
//! are ordered precisely so that the classes a grammar needs come out
//! contiguous, and an implementation that enumerated the classes instead would
//! throw that away and have to be corrected by hand for ever.
//!
//! # Why this exists in an operating system
//!
//! Because the assembler kept needing it and kept guessing. The lexer has to
//! decide whether a kāraka sigil will fuse with a virama-final name into one
//! akṣara ([`ADR-0005`]), and the answer is exactly "is the sigil in `हल्`" —
//! a consonant conjoins, a vowel does not. That was worked out by hand instead,
//! stated backwards in the ADR, shipped missing one of the four consonant
//! sigils, and found by 439 conformance failures.
//!
//! Enumerating a class is how you get that wrong. Deriving it is how you stop.

extern crate alloc;

use alloc::vec::Vec;

const SUTRAS: &str = include_str!("../../../spec/shiva-sutras.tsv");

/// One entry of the sūtra sequence: a phoneme, or a boundary marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Which of the fourteen sūtras, 1-based.
    pub line: u8,
    /// Position within that sūtra, 1-based.
    pub pos: u8,
    /// The Devanagari form. Markers carry a virama; phonemes do not.
    pub devanagari: &'static str,
    /// IAST transliteration.
    pub iast: &'static str,
    /// True for an anubandha — a boundary sign rather than a sound.
    pub is_marker: bool,
}

/// The sūtras as one ordered sequence, markers included.
#[must_use]
pub fn sequence() -> Vec<Entry> {
    SUTRAS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("line\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 5 {
                return None;
            }
            Some(Entry {
                line: f[0].parse().ok()?,
                pos: f[1].parse().ok()?,
                devanagari: f[2],
                iast: f[3],
                is_marker: f[4] == "it",
            })
        })
        .collect()
}

/// The phonemes of a pratyāhāra, given its start sound and its marker.
///
/// Both are written in Devanagari, the marker with its virama: `pratyahara("अ",
/// "च्")` is the vowels. Returns `None` if the start is not a phoneme of the
/// sūtras, or if the marker never occurs after it.
///
/// A start sound occurring twice takes its FIRST occurrence, which is what
/// makes `हल्` the consonants: `ह` is in sūtra 5 and again in sūtra 14, and
/// starting at the second would yield a set of one.
#[must_use]
pub fn pratyahara(start: &str, marker: &str) -> Option<Vec<&'static str>> {
    let seq = sequence();
    let from = seq
        .iter()
        .position(|e| !e.is_marker && e.devanagari == start)?;

    let mut out = Vec::new();
    for e in &seq[from..] {
        if e.is_marker {
            if e.devanagari == marker {
                return Some(out);
            }
            // A marker that is not the one asked for is a boundary for some
            // other pratyāhāra and simply not a sound. Skipping rather than
            // stopping is what lets a set span several sūtras.
            continue;
        }
        out.push(e.devanagari);
    }
    None
}

/// Where one anubandha points — task `B-049`.
///
/// See [`cross_sutra_edges`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    /// `(sūtra, position)` of the marker.
    pub from: (u8, u8),
    /// `(sūtra, position)` of the phoneme it names.
    pub to: (u8, u8),
    /// The marker, with its virama.
    pub marker: &'static str,
}

/// The fourteen edges that close the sūtras into one connected structure.
///
/// An anubandha is written as a consonant with a virama — `ण्`, `क्`, `ट्` —
/// and that consonant occurs elsewhere in the sūtras as an actual sound. The
/// edge is simply that correspondence: **a marker points at the phoneme
/// spelling the same consonant.**
///
/// # Why derive it rather than store it
///
/// `ParamTatva.md` lists this map by hand, fourteen lines of
/// `1-4 (ṇ) → 7-4 (ṇa)`. Doc 19 R1 reproduced all fourteen from the ordering
/// alone, which means the map is a *consequence* of the table and storing it
/// would be storing a derivable fact — the failure this project has hit three
/// times in the assembler (`B-042`, `B-044`).
///
/// It also makes the table self-checking, which is the point of `B-049`.
/// `spec/shiva-sutras.tsv` is authored and cannot be derived from anything, so
/// nothing else can catch a transcription slip in it. But every marker must
/// resolve to **exactly one** phoneme, and a table with a sound dropped,
/// duplicated or misspelled fails that immediately.
#[must_use]
pub fn cross_sutra_edges() -> Vec<Edge> {
    let seq = sequence();
    let mut out = Vec::new();
    for m in seq.iter().filter(|e| e.is_marker) {
        let base = m.devanagari.trim_end_matches('\u{094d}'); // strip the virama
        let mut hits = seq
            .iter()
            .filter(|e| !e.is_marker && e.devanagari == base)
            .map(|e| (e.line, e.pos));
        if let (Some(to), None) = (hits.next(), hits.next()) {
            out.push(Edge {
                from: (m.line, m.pos),
                to,
                marker: m.devanagari,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The same sets, resolved at compile time — task `B-047`.
//
// [`pratyahara`] above is the readable statement of the rule and allocates a
// Vec per call. The lexer asks "is this a consonant" for every operand it
// splits, so it needs the answer without walking a table, and every phoneme
// class is a CONTIGUOUS RANGE of the ordering (doc 19 R2) — which is exactly
// what makes a bitset the natural representation rather than a clever one.
//
// The Devanagari block is 128 code points, so a class of characters is two
// `u64`s and membership is a shift and an AND. Both are computed by `const fn`
// from the same TSV, so there is still no list anywhere: change the ordering
// and the constants change with it.
// ---------------------------------------------------------------------------

/// First code point of the Devanagari block.
const BLOCK: u32 = 0x0900;

/// A set of Devanagari characters, as a 128-bit mask over the block.
pub type CharSet = [u64; 2];

/// Compare `hay[a..b]` with `needle` without slicing, which is not const.
const fn seg_eq(hay: &[u8], a: usize, b: usize, needle: &[u8]) -> bool {
    if b - a != needle.len() {
        return false;
    }
    let mut i = 0;
    while i < needle.len() {
        if hay[a + i] != needle[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Bounds of the `n`-th tab-separated field of `hay[ls..le]`.
const fn field(hay: &[u8], ls: usize, le: usize, n: usize) -> (usize, usize) {
    let mut idx = 0;
    let mut a = ls;
    let mut i = ls;
    while i < le {
        if hay[i] == b'\t' {
            if idx == n {
                return (a, i);
            }
            idx += 1;
            a = i + 1;
        }
        i += 1;
    }
    if idx == n { (a, le) } else { (le, le) }
}

/// Decode one three-byte UTF-8 sequence. Every Devanagari character is three.
const fn scalar_at(hay: &[u8], i: usize) -> u32 {
    ((hay[i] as u32 & 0x0f) << 12) | ((hay[i + 1] as u32 & 0x3f) << 6) | (hay[i + 2] as u32 & 0x3f)
}

/// The characters of a pratyāhāra, as a block mask, computed at compile time.
///
/// Same rule as [`pratyahara`]: start at a phoneme, run forward, stop at the
/// named marker. Returns an empty set if the range does not exist, which is a
/// compile-time value and so cannot pass unnoticed — the tests below assert
/// each constant is populated.
#[must_use]
pub const fn char_set(start: &str, marker: &str) -> CharSet {
    let s = SUTRAS.as_bytes();
    let m = marker.as_bytes();
    let st = start.as_bytes();
    let mut out = [0u64; 2];
    let mut started = false;
    let mut i = 0;

    while i < s.len() {
        let ls = i;
        while i < s.len() && s[i] != b'\n' {
            i += 1;
        }
        let le = i;
        if i < s.len() {
            i += 1;
        }
        if le <= ls || s[ls] == b'#' {
            continue;
        }

        let (ds, de) = field(s, ls, le, 2);
        let (ks, ke) = field(s, ls, le, 4);

        if seg_eq(s, ks, ke, b"it") {
            // A marker is a boundary, never a sound. Only the named one ends
            // the range; the others are passed over.
            if started && seg_eq(s, ds, de, m) {
                return out;
            }
            continue;
        }
        if !seg_eq(s, ks, ke, b"phoneme") {
            continue; // the header row
        }
        if !started && seg_eq(s, ds, de, st) {
            started = true;
        }
        if started {
            let c = scalar_at(s, ds);
            if c >= BLOCK && c < BLOCK + 128 {
                let bit = c - BLOCK;
                out[(bit / 64) as usize] |= 1u64 << (bit % 64);
            }
        }
    }
    [0, 0]
}

/// Whether a character is in a [`CharSet`].
#[must_use]
pub const fn holds(set: CharSet, c: char) -> bool {
    let v = c as u32;
    if v < BLOCK || v >= BLOCK + 128 {
        return false;
    }
    let bit = v - BLOCK;
    set[(bit / 64) as usize] >> (bit % 64) & 1 == 1
}

/// `हल्` — the consonants.
pub const HAL: CharSet = char_set("ह", "ल्");
/// `अच्` — the vowels.
pub const AC: CharSet = char_set("अ", "च्");
/// `यण्` — the semivowels.
pub const YAN: CharSet = char_set("य", "ण्");
/// `शर्` — the sibilants.
pub const SHAR: CharSet = char_set("श", "र्");

/// Whether an akṣara begins with a consonant — membership of `हल्`.
///
/// This is the question the lexer asks of a kāraka sigil. `म्`, `न`, `त्` and
/// `य्` are consonants and will fuse with a preceding virama into one cluster;
/// `ए` is an independent vowel and will not.
///
/// One shift and one AND against a constant derived from the sūtras. Nothing is
/// listed, so a sigil added later is classified without anyone remembering to
/// update a table.
#[must_use]
pub fn is_consonant(aksara: &str) -> bool {
    aksara.chars().next().is_some_and(|c| holds(HAL, c))
}

/// Whether an akṣara begins with a vowel — membership of `अच्`.
#[must_use]
pub fn is_vowel(aksara: &str) -> bool {
    aksara.chars().next().is_some_and(|c| holds(AC, c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    fn joined(start: &str, marker: &str) -> String {
        pratyahara(start, marker).expect("derives").join(" ")
    }

    fn distinct(start: &str, marker: &str) -> usize {
        let mut v = pratyahara(start, marker).expect("derives");
        v.sort_unstable();
        v.dedup();
        v.len()
    }

    #[test]
    fn the_table_is_the_fourteen_sutras() {
        let seq = sequence();
        assert_eq!(seq.len(), 57, "57 entries: 43 phonemes and 14 markers");
        assert_eq!(seq.iter().filter(|e| e.is_marker).count(), 14);
        assert_eq!(seq.iter().map(|e| e.line).max(), Some(14));
        // Every sūtra ends with exactly one marker and nothing follows it.
        for line in 1..=14u8 {
            let of_line: Vec<&Entry> = seq.iter().filter(|e| e.line == line).collect();
            assert!(!of_line.is_empty(), "sūtra {line} is empty");
            assert!(
                of_line.last().expect("non-empty").is_marker,
                "sūtra {line} does not end in an anubandha"
            );
            assert_eq!(
                of_line.iter().filter(|e| e.is_marker).count(),
                1,
                "sūtra {line} has more than one anubandha"
            );
        }
    }

    #[test]
    fn the_classical_pratyaharas_come_out_right() {
        // Values fixed by Pāṇini, not by this implementation. If the ordering
        // in the TSV is ever disturbed these are what notice.
        assert_eq!(joined("अ", "च्"), "अ इ उ ऋ ऌ ए ओ ऐ औ", "अच् — the vowels");
        assert_eq!(joined("अ", "ण्"), "अ इ उ", "अण्");
        assert_eq!(joined("इ", "क्"), "इ उ ऋ ऌ", "इक्");
        assert_eq!(joined("अ", "क्"), "अ इ उ ऋ ऌ", "अक् — the simple vowels");
        assert_eq!(joined("ए", "च्"), "ए ओ ऐ औ", "एच्");
        assert_eq!(joined("ऐ", "च्"), "ऐ औ", "ऐच् — the diphthongs");
        assert_eq!(joined("य", "ण्"), "य व र ल", "यण् — the semivowels");
        assert_eq!(joined("श", "र्"), "श ष स", "शर् — the sibilants");
        assert_eq!(joined("झ", "ष्"), "झ भ घ ढ ध", "झष् — voiced aspirated stops");
        assert_eq!(distinct("ह", "ल्"), 33, "हल् — the 33 consonants");
        assert_eq!(distinct("अ", "ल्"), 42, "अल् — every sound, 9 vowels + 33");
    }

    #[test]
    fn ha_occurs_twice_and_hal_starts_at_the_first() {
        let seq = sequence();
        let ha: Vec<&Entry> = seq
            .iter()
            .filter(|e| !e.is_marker && e.devanagari == "ह")
            .collect();
        assert_eq!(ha.len(), 2, "ह is in sūtra 5 and sūtra 14");
        assert_eq!((ha[0].line, ha[1].line), (5, 14));
        // Taking the second would give a set of one, and हल् would name
        // nothing useful.
        assert_eq!(pratyahara("ह", "ल्").expect("derives").len(), 34);
    }

    #[test]
    fn a_marker_is_not_a_sound() {
        // म् closes sūtra 7 and म is a phoneme inside it. Confusing the two
        // would put a boundary sign into a phoneme set.
        let yam = pratyahara("य", "म्").expect("derives");
        assert_eq!(yam.join(" "), "य व र ल ञ म ङ ण न");
        assert!(!yam.contains(&"म्"), "the marker itself is never a member");
    }

    #[test]
    fn every_anubandha_names_exactly_one_sound() {
        // The self-check. `spec/shiva-sutras.tsv` is authored and cannot be
        // derived, so nothing else can catch a slip in it — but each of the
        // fourteen markers must resolve to exactly one phoneme, and a table
        // with a sound dropped, duplicated or misspelled fails here.
        let edges = cross_sutra_edges();
        let markers = sequence().iter().filter(|e| e.is_marker).count();
        assert_eq!(
            edges.len(),
            markers,
            "an anubandha resolved to zero or several sounds"
        );
        assert_eq!(edges.len(), 14);
    }

    #[test]
    fn the_derived_edges_match_the_recorded_map() {
        // Pinned from doc 19 R1, which reproduced ParamTatva's hand-authored
        // secondary-continuation map from the ordering alone, 14 of 14. This is
        // an external cross-check: the values were arrived at independently, so
        // agreement is evidence about the transcription rather than about this
        // function.
        let want: [((u8, u8), (u8, u8)); 14] = [
            ((1, 4), (7, 4)),
            ((2, 3), (12, 1)),
            ((3, 3), (7, 3)),
            ((4, 3), (11, 6)),
            ((5, 5), (11, 7)),
            ((6, 2), (7, 4)),
            ((7, 6), (7, 2)),
            ((8, 3), (7, 1)),
            ((9, 4), (13, 2)),
            ((10, 6), (13, 1)),
            ((11, 9), (5, 3)),
            ((12, 3), (5, 2)),
            ((13, 4), (5, 4)),
            ((14, 2), (6, 1)),
        ];
        let got: Vec<((u8, u8), (u8, u8))> =
            cross_sutra_edges().iter().map(|e| (e.from, e.to)).collect();
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(want.iter()) {
            assert_eq!(g, w, "edge from sūtra {}-{} moved", w.0.0, w.0.1);
        }
    }

    #[test]
    fn the_markers_reach_only_five_sutras() {
        // Not decoration: the fourteen edges land in sūtras 5, 6, 7, 11, 12 and
        // 13 only, because those are where the consonants that serve as
        // anubandhas actually occur. If a transcription error moved a sound
        // into another sūtra this is what would notice.
        let mut lines: Vec<u8> = cross_sutra_edges().iter().map(|e| e.to.0).collect();
        lines.sort_unstable();
        lines.dedup();
        assert_eq!(lines.as_slice(), [5u8, 6, 7, 11, 12, 13].as_slice());
    }

    #[test]
    fn the_compiled_sets_agree_with_the_table_walk() {
        // Two implementations of one rule: the readable walk and the const
        // masks. They must not drift, and only the walk is obviously correct.
        for (name, set, start, marker) in [
            ("हल्", HAL, "ह", "ल्"),
            ("अच्", AC, "अ", "च्"),
            ("यण्", YAN, "य", "ण्"),
            ("शर्", SHAR, "श", "र्"),
        ] {
            assert_ne!(set, [0, 0], "{name} compiled to an empty set");
            let walked = pratyahara(start, marker).expect("derives");
            for p in &walked {
                let c = p.chars().next().expect("non-empty");
                assert!(holds(set, c), "{name}: mask is missing {p}");
            }
            // And nothing the walk did not produce.
            let bits = set[0].count_ones() + set[1].count_ones();
            let mut uniq: Vec<char> = walked
                .iter()
                .map(|p| p.chars().next().expect("non-empty"))
                .collect();
            uniq.sort_unstable();
            uniq.dedup();
            assert_eq!(bits as usize, uniq.len(), "{name}: mask has extra bits");
        }
    }

    #[test]
    fn a_character_outside_the_block_is_in_no_class() {
        assert!(!holds(HAL, 'x'));
        assert!(!holds(AC, '7'));
        assert!(!is_consonant("।"), "the daṇḍa is punctuation, not a sound");
    }

    #[test]
    fn the_karaka_sigils_split_exactly_along_hal() {
        // This is the property ADR-0005 needed and got wrong by hand. Four of
        // the five sigils are consonants and fuse with a preceding virama; the
        // fifth is a vowel and does not.
        for sigil in ["म्", "न", "त्", "य्"] {
            assert!(is_consonant(sigil), "{sigil} is in हल् and conjoins");
        }
        assert!(!is_consonant("ए"), "ए is in अच् and cannot conjoin");
    }

    #[test]
    fn an_unknown_start_is_reported_rather_than_guessed() {
        assert!(pratyahara("क्ष", "ल्").is_none(), "क्ष is not a sūtra entry");
        assert!(pratyahara("अ", "ह्").is_none(), "ह् is not an anubandha");
        assert!(!is_consonant(""));
    }
}
