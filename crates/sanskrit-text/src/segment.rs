//! अक्षर segmentation — extended grapheme clusters (UAX #29).
//!
//! The akṣara is SANSOS's atomic unit of text (doc 01 D-01-A): one terminal
//! cell, one cursor step, one backspace, one unit of column arithmetic. It is
//! defined as the UAX #29 **extended** grapheme cluster computed with the
//! `InCB` rules of Unicode ≥ 15.1, which is what makes `क्ष` a single cluster
//! rather than two.
//!
//! # Allocation-free by requirement
//!
//! This module uses neither `alloc` nor `std`. The terminal line discipline
//! needs akṣara-granular erase (doc 04 §3), and a table plus a five-field
//! cursor is all that costs. [`Aksharas`] borrows from its input and yields
//! `&str` slices; nothing here can fail or allocate.

use crate::lookup;
use crate::tables::{
    DEVANAGARI, EXTENDED_PICTOGRAPHIC, Flag, GRAPHEME_BREAK, GraphemeBreak as G, INCB, Incb,
};

fn gcb(ch: char) -> G {
    lookup(GRAPHEME_BREAK, ch as u32, G::Other)
}

fn incb(ch: char) -> Incb {
    lookup(INCB, ch as u32, Incb::None)
}

fn is_ext_pict(ch: char) -> bool {
    lookup(EXTENDED_PICTOGRAPHIC, ch as u32, Flag::No) == Flag::Yes
}

/// Whether a codepoint belongs to the Devanagari block proper.
#[must_use]
pub fn is_devanagari(ch: char) -> bool {
    lookup(DEVANAGARI, ch as u32, Flag::No) == Flag::Yes
}

/// Everything the break rules need to remember about the text so far.
///
/// Three of the rules are not decidable from the adjacent pair alone, which is
/// the whole reason this is a state machine rather than a lookup:
///
/// - **GB9c** needs to know a `Linker` was seen after a `Consonant` — the rule
///   that keeps a conjunct together.
/// - **GB11** needs to know a `ZWJ` was reached through `ExtPict Extend*`.
/// - **GB12/13** need the *parity* of the regional-indicator run, so flags pair
///   up two at a time.
#[derive(Clone, Copy)]
struct Cursor {
    prev: G,
    /// Length of the run of regional indicators ending at `prev`.
    ri_run: usize,
    /// 0 = no pictographic context, 1 = `ExtPict Extend*`, 2 = that plus `ZWJ`.
    pict: u8,
    /// Seen `InCB=Consonant`, with only Extend/Linker since.
    incb_consonant: bool,
    /// Seen `InCB=Linker` after that consonant.
    incb_linker: bool,
}

impl Cursor {
    fn new(first: char) -> Self {
        let mut c = Self {
            prev: G::Other,
            ri_run: 0,
            pict: 0,
            incb_consonant: false,
            incb_linker: false,
        };
        c.push(first);
        c
    }

    fn push(&mut self, ch: char) {
        let g = gcb(ch);

        self.ri_run = if g == G::RegionalIndicator {
            self.ri_run + 1
        } else {
            0
        };

        // Track `ExtPict Extend*` as one state, then `ZWJ` advances it.
        self.pict = if is_ext_pict(ch) || (self.pict == 1 && g == G::Extend) {
            1
        } else if self.pict == 1 && g == G::Zwj {
            2
        } else {
            0
        };

        match incb(ch) {
            Incb::Consonant => {
                self.incb_consonant = true;
                self.incb_linker = false;
            }
            Incb::Linker if self.incb_consonant => self.incb_linker = true,
            Incb::Extend if self.incb_consonant => {}
            _ => {
                self.incb_consonant = false;
                self.incb_linker = false;
            }
        }

        self.prev = g;
    }

    /// Is there a cluster boundary between the text so far and `next`?
    ///
    /// Rules are applied in the order UAX #29 numbers them; the first that
    /// matches decides, and GB999 breaks by default.
    fn breaks_before(&self, next: char) -> bool {
        let (a, b) = (self.prev, gcb(next));

        if a == G::Cr && b == G::Lf {
            return false; // GB3
        }
        if matches!(a, G::Control | G::Cr | G::Lf) {
            return true; // GB4
        }
        if matches!(b, G::Control | G::Cr | G::Lf) {
            return true; // GB5
        }
        if a == G::L && matches!(b, G::L | G::V | G::Lv | G::Lvt) {
            return false; // GB6
        }
        if matches!(a, G::Lv | G::V) && matches!(b, G::V | G::T) {
            return false; // GB7
        }
        if matches!(a, G::Lvt | G::T) && b == G::T {
            return false; // GB8
        }
        if matches!(b, G::Extend | G::Zwj) {
            return false; // GB9
        }
        if b == G::SpacingMark {
            return false; // GB9a — matras attach to their base
        }
        if a == G::Prepend {
            return false; // GB9b
        }
        if self.incb_consonant && self.incb_linker && incb(next) == Incb::Consonant {
            return false; // GB9c — the conjunct rule: क + ् + ष stays one akṣara
        }
        if self.pict == 2 && is_ext_pict(next) {
            return false; // GB11
        }
        if b == G::RegionalIndicator && !self.ri_run.is_multiple_of(2) {
            return false; // GB12/13 — flags pair up
        }
        true // GB999
    }
}

/// Iterator over the akṣaras of a string, yielding borrowed slices.
#[derive(Clone)]
pub struct Aksharas<'a> {
    s: &'a str,
    start: usize,
}

impl<'a> Iterator for Aksharas<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.start >= self.s.len() {
            return None;
        }
        let begin = self.start;
        let rest = &self.s[begin..];

        let mut chars = rest.char_indices();
        let (_, first) = chars.next()?;
        let mut cursor = Cursor::new(first);
        let mut end = begin + first.len_utf8();

        for (offset, ch) in chars {
            if cursor.breaks_before(ch) {
                break;
            }
            cursor.push(ch);
            end = begin + offset + ch.len_utf8();
        }

        self.start = end;
        Some(&self.s[begin..end])
    }
}

/// Split `s` into akṣaras. The unit of every cursor motion in SANSOS.
#[must_use]
pub fn aksharas(s: &str) -> Aksharas<'_> {
    Aksharas { s, start: 0 }
}

/// Number of akṣaras in `s` — the **column count**, not `chars().count()`.
#[must_use]
pub fn aksharas_count(s: &str) -> usize {
    aksharas(s).count()
}

/// Byte index where the last akṣara begins, for akṣara-granular erase
/// (doc 04 §3). `None` if `s` is empty.
#[must_use]
pub fn last_akshara_start(s: &str) -> Option<usize> {
    let mut start = None;
    let mut consumed = 0;
    for a in aksharas(s) {
        start = Some(consumed);
        consumed += a.len();
    }
    start
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split<'a>(s: &'a str, out: &mut [&'a str]) -> usize {
        let mut n = 0;
        for a in aksharas(s) {
            out[n] = a;
            n += 1;
        }
        n
    }

    #[test]
    fn conjunct_is_one_akshara() {
        // क्ष = क + virama + ष. GB9c. This is doc 01 D-01-A's headline case:
        // before Unicode 15.1 this was TWO clusters, which would have made the
        // terminal cell model wrong for the most common Sanskrit conjuncts.
        let mut buf = [""; 8];
        assert_eq!(split("\u{0915}\u{094D}\u{0937}", &mut buf), 1);
        // क्ष्ण — 5 codepoints, still one akṣara.
        assert_eq!(
            split("\u{0915}\u{094D}\u{0937}\u{094D}\u{0923}", &mut buf),
            1
        );
    }

    #[test]
    fn matra_attaches_to_its_base() {
        // कि — the I-matra is stored AFTER the consonant but drawn BEFORE it.
        // GB9a keeps them one unit regardless of visual order.
        let mut buf = [""; 8];
        assert_eq!(split("\u{0915}\u{093F}", &mut buf), 1);
        // कं anusvara, कः visarga
        assert_eq!(split("\u{0915}\u{0902}", &mut buf), 1);
        assert_eq!(split("\u{0915}\u{0903}", &mut buf), 1);
    }

    #[test]
    fn separate_consonants_are_separate_aksharas() {
        let mut buf = [""; 8];
        assert_eq!(split("\u{0915}\u{0916}\u{0917}", &mut buf), 3);
    }

    #[test]
    fn purpose_invocation_column_count() {
        // Test vector #1 (doc 01 §5). Counted excluding spaces, this is what the
        // terminal must agree with to place a cursor correctly.
        let invocation = "\u{0950} परम तत्वयाय नारायणाय गुरुभ्यो नमः";
        let n = aksharas(invocation).filter(|a| *a != " ").count();
        assert_eq!(n, 18, "akṣara count of the invocation");
        // and never more clusters than codepoints
        assert!(aksharas_count(invocation) <= invocation.chars().count());
    }

    #[test]
    fn crlf_is_one_cluster() {
        let mut buf = [""; 8];
        assert_eq!(split("\r\n", &mut buf), 1); // GB3
        assert_eq!(split("\r\ra", &mut buf), 3); // GB4/GB5
    }

    #[test]
    fn regional_indicators_pair_up() {
        let mut buf = [""; 8];
        assert_eq!(split("\u{1F1EE}\u{1F1F3}", &mut buf), 1); // one flag
        assert_eq!(split("\u{1F1EE}\u{1F1F3}\u{1F1EE}\u{1F1F3}", &mut buf), 2);
        assert_eq!(split("\u{1F1EE}\u{1F1F3}\u{1F1EE}", &mut buf), 2); // GB12/13 parity
    }

    #[test]
    fn zwj_emoji_sequence_is_one_cluster() {
        // GB11: family sequence
        let mut buf = [""; 8];
        assert_eq!(split("\u{1F468}\u{200D}\u{1F469}", &mut buf), 1);
    }

    #[test]
    fn last_akshara_start_is_cluster_aligned() {
        // Backspace must delete क्ष whole, not just ष.
        let s = "\u{0915}\u{0915}\u{094D}\u{0937}";
        let start = last_akshara_start(s).unwrap();
        assert_eq!(&s[start..], "\u{0915}\u{094D}\u{0937}");
        assert_eq!(last_akshara_start(""), None);
    }

    #[test]
    fn slices_reconstruct_the_input() {
        for s in [
            "",
            "abc",
            "\u{0915}\u{094D}\u{0937}\u{0940}",
            "\r\n\u{1F1EE}\u{1F1F3}",
        ] {
            let mut total = 0;
            for a in aksharas(s) {
                assert!(!a.is_empty(), "empty akṣara in {s:?}");
                total += a.len();
            }
            assert_eq!(total, s.len(), "lossy segmentation of {s:?}");
        }
    }

    #[test]
    fn devanagari_block_membership() {
        assert!(is_devanagari('\u{0915}'));
        assert!(is_devanagari('\u{0966}')); // ०
        assert!(!is_devanagari('A'));
    }
}
