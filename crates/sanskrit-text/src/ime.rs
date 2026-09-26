//! The input method, as a state machine — task `A-038c1`, doc 04 §8 (4.1.1).
//!
//! Keystrokes in, Devanagari out. No terminal, no raw mode, no rendering: those
//! are `A-038c2`, and keeping them out is what lets the part that decides *what
//! the text becomes* be tested without a TTY. `A-039` measures a person typing
//! 200 lines through this; a bug here is a bug in the number.
//!
//! # Three pieces, already built
//!
//! - **Letters** are SLP1, which decodes partial input: `k` shows क्, `ka` shows
//!   क, `kozWa` shows कोष्ठ. So the pending buffer can be re-decoded on every
//!   keystroke and the typist sees the word forming.
//! - **Signs** are one keystroke each (`A-038a`), and a sign ends the word being
//!   typed: `ko` then `.` is को then ॱ, not an attempt to decode `ko.`.
//! - **Completion** is [`crate::completion`] (`A-038b`): `ko` offers कोष्ठ, and
//!   taking it commits the ratified spelling rather than whatever the fingers
//!   would have produced.
//!
//! # Why the pending buffer holds SLP1 and not Devanagari
//!
//! Backspace. Keystrokes and codepoints do not correspond: `ka` is two keys for
//! the one codepoint क, and `kozWa` is five for five — the counts coincide there
//! and the *positions* still do not, because `o` rewrites the virama it follows
//! rather than appending. Deleting a codepoint would therefore mean something
//! different after every key. Holding the SLP1 and re-decoding makes backspace
//! exactly "unpress the last key", which is the only model a typist can predict.
//!
//! (An earlier version of this comment said कोष्ठ was four codepoints. It is
//! five; `A-039a` caught it while writing test data against it.)

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::completion::Term;

/// The ASCII that ends a word rather than joining it.
///
/// ADR-0003's ratified signs, plus the whitespace that separates words. Pinned
/// against the ADR by [`tests::the_sign_set_is_the_one_the_adr_ratifies`] so a
/// sign added there cannot silently keep being treated as a letter.
const SIGNS: &[char] = &['|', '&', '@', '.', '\'', '$'];

/// What the typist pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character.
    Char(char),
    /// Take the first completion candidate.
    Complete,
    /// Unpress the last key.
    Backspace,
    /// Commit the pending word and start a line.
    Enter,
}

/// The input method's whole state.
#[derive(Debug, Default, Clone)]
pub struct Ime {
    committed: String,
    pending: String,
}

impl Ime {
    /// A fresh, empty input method.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The text as it would be saved.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.committed
    }

    /// The word being typed, as it currently reads.
    ///
    /// Empty when nothing is pending. This is what a front end draws after the
    /// committed text, usually underlined.
    #[must_use]
    pub fn preview(&self) -> String {
        let mut out = String::new();
        if crate::slp1::decode_into(&self.pending, &mut out).is_err() {
            // A prefix SLP1 cannot read yet is shown as typed rather than
            // dropped: the typist needs to see the key they pressed.
            out.clear();
            out.push_str(&self.pending);
        }
        out
    }

    /// What completion offers for the word being typed.
    #[must_use]
    pub fn candidates(&self, limit: usize) -> Vec<Term> {
        crate::completion::candidates(&self.pending, limit)
    }

    /// Commit the pending word, whatever it currently reads as.
    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let word = self.preview();
        self.committed.push_str(&word);
        self.pending.clear();
    }

    /// Commit the word being typed, without a line break.
    ///
    /// What the preview shows is what the typist believes they have written, so
    /// a session that ends — Ctrl-C, end of input — must keep it. Returning only
    /// the committed text would silently drop the word on screen, which is the
    /// last thing anyone typed and the first thing they would look for.
    pub fn commit(&mut self) {
        self.flush();
    }

    /// Apply one keystroke.
    pub fn press(&mut self, key: Key) {
        match key {
            Key::Char(c) if SIGNS.contains(&c) || c.is_whitespace() => {
                // A sign ends the word. Decoding `ko.` as one unit would make
                // the sign's meaning depend on what preceded it.
                self.flush();
                let mut out = String::new();
                let s = alloc::string::ToString::to_string(&c);
                if crate::slp1::decode_into(&s, &mut out).is_ok() {
                    self.committed.push_str(&out);
                } else {
                    self.committed.push(c);
                }
            }
            Key::Char(c) => self.pending.push(c),
            Key::Complete => {
                if let Some(top) = self.candidates(1).first() {
                    self.committed.push_str(top.devanagari);
                    self.pending.clear();
                }
            }
            Key::Backspace => {
                if self.pending.pop().is_none() {
                    self.committed.pop();
                }
            }
            Key::Enter => {
                self.flush();
                self.committed.push('\n');
            }
        }
    }

    /// Apply a whole string of keystrokes, for tests and for replaying a
    /// recorded session.
    pub fn type_str(&mut self, s: &str) {
        for c in s.chars() {
            self.press(Key::Char(c));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    /// The sign set is ADR-0003's, not a list someone remembered.
    ///
    /// Every sign the ADR ratifies must be a key that ends a word. A sign added
    /// to the ADR and not here would be typed *into* the middle of a word and
    /// decoded as if it were a letter.
    #[test]
    fn the_sign_set_is_the_one_the_adr_ratifies() {
        // The signs, via the bijection: each is one ASCII key (`A-038a`).
        for sign in ["।", "॥", "॰", "ॱ", "ऽ", "ॐ"] {
            let mut typed = String::new();
            crate::slp1::encode_into(sign, &mut typed).expect("sign has an SLP1 key");
            let key = typed.chars().next().expect("non-empty");
            assert!(
                SIGNS.contains(&key),
                "`{sign}` is typed `{key}`, which SIGNS does not treat as ending a word"
            );
        }
    }

    #[test]
    fn letters_accumulate_and_the_preview_shows_the_word_forming() {
        let mut ime = Ime::new();
        ime.type_str("k");
        assert_eq!(ime.preview(), "क्");
        ime.type_str("o");
        assert_eq!(ime.preview(), "को");
        ime.type_str("zWa");
        assert_eq!(ime.preview(), "कोष्ठ");
        assert_eq!(ime.text(), "", "nothing is committed until a boundary");
    }

    #[test]
    fn a_sign_ends_the_word_and_is_itself_committed() {
        let mut ime = Ime::new();
        ime.type_str("ko.");
        assert_eq!(ime.text(), "कोॱ", "`ko` commits, then the high dot");
        assert_eq!(ime.preview(), "");
    }

    #[test]
    fn completion_commits_the_ratified_spelling() {
        let mut ime = Ime::new();
        ime.type_str("kozWa");
        let offered = ime.candidates(1);
        assert!(!offered.is_empty(), "`kozWa` should offer something");
        ime.press(Key::Complete);
        assert_eq!(ime.text(), offered[0].devanagari);
        assert_eq!(
            ime.preview(),
            "",
            "taking a candidate clears what was typed"
        );
    }

    /// Backspace unpresses a key, not a codepoint.
    ///
    /// Keystrokes and codepoints do not correspond — `ka` is two keys for one
    /// codepoint — so deleting a codepoint would mean something different after
    /// every key.
    #[test]
    fn backspace_unpresses_the_last_key() {
        let mut ime = Ime::new();
        ime.type_str("kozWa");
        assert_eq!(ime.preview(), "कोष्ठ");
        ime.press(Key::Backspace);
        assert_eq!(ime.preview(), "कोष्ठ्", "one key back is `kozW`");
        ime.press(Key::Backspace);
        assert_eq!(ime.preview(), "कोष्");
    }

    #[test]
    fn backspace_past_the_word_takes_back_committed_text() {
        let mut ime = Ime::new();
        ime.type_str("ka ");
        assert_eq!(ime.text(), "क ");
        ime.press(Key::Backspace);
        assert_eq!(ime.text(), "क");
    }

    #[test]
    fn enter_commits_the_word_before_the_newline() {
        let mut ime = Ime::new();
        ime.type_str("ka");
        ime.press(Key::Enter);
        assert_eq!(ime.text(), "क\n");
        assert_eq!(ime.preview(), "");
    }

    /// A whole line of Sassembly, typed.
    ///
    /// The point of the row: this is the shape `A-039` measures. `yogaH` is five
    /// keys for योगः, the danda is one, and nothing needed a menu.
    #[test]
    fn a_line_of_sassembly_comes_out_right() {
        let mut ime = Ime::new();
        ime.type_str("yogaH kam Kan gan |");
        assert_eq!(ime.text(), "योगः कम् खन् गन् ।");
    }

    #[test]
    fn completion_on_an_empty_word_does_nothing() {
        let mut ime = Ime::new();
        ime.press(Key::Complete);
        assert_eq!(ime.text(), "");
        ime.press(Key::Backspace);
        assert_eq!(ime.text(), "", "backspace on empty is not a panic");
    }

    #[test]
    fn the_preview_never_hides_a_keystroke() {
        // Whatever is pending, the typist can see that they pressed something.
        let mut ime = Ime::new();
        for c in "qQxX".chars() {
            ime.press(Key::Char(c));
            assert!(
                !ime.preview().is_empty(),
                "after `{c}` the preview went blank",
            );
        }
    }

    #[test]
    fn typing_is_deterministic() {
        let mut a = Ime::new();
        let mut b = Ime::new();
        a.type_str("kozWa ka |");
        b.type_str("kozWa ka |");
        assert_eq!(a.text(), b.text());
        assert_eq!(a.text().to_string(), b.text().to_string());
    }
}
