//! **निदानम्** — diagnostics by code rather than by string, task `B-015`.
//!
//! Doc 01 §1.2.4: *wire compiler diagnostics to lexicon IDs, not literal
//! strings — enables bilingual errors free.* This is that, and the "free" is
//! real: `spec/diagnostics.tsv` carries both languages per row, so translating
//! is editing a table rather than finding every `format!` in the tree.
//!
//! # Why a code
//!
//! Doc 12's **D-12-A1** says compiler diagnostics are a Sankriti feature and
//! must be *machine-repairable*: a stable code, a precise span, a suggested
//! fix. Prose is none of those. `L01` survives rewording, translation and
//! rephrasing; "is exported by more than one file" survives nothing, and a
//! model trained to recognise it learns a string rather than a fault.
//!
//! # Language
//!
//! Devanagari by default, because this is a Sanskrit-native system and its
//! compiler speaking English by default would be the tail wagging the dog.
//! `SANSOS_LANG=en` asks for English, which is what a bug report pasted into a
//! foreign issue tracker wants.
//!
//! # Script
//!
//! Doc 03 §3.3.4 asks for diagnostics *in Devanagari with SLP1 fallback*: a
//! terminal that cannot render the script shows a row of boxes, and a message
//! nobody can read is not a diagnostic. [`Script::Slp1`] transliterates through
//! [`sanskrit_text::slp1`], which is a proved bijection rather than a
//! transliteration heuristic, so nothing is invented for the fallback.
//!
//! **Language and script are different questions.** The message stays Sanskrit;
//! only the letters change. Folding the two into one enum would make asking for
//! ASCII mean asking for English, which is the assumption doc 15 exists to
//! refuse.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// The registry, compiled in for the same reason the mnemonics are: a
/// diagnostic that depends on a file being present can fail to explain why a
/// file is missing.
const DIAGNOSTICS: &str = include_str!("../../../spec/diagnostics.tsv");

/// Which language to render in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// संस्कृतम् — the default.
    #[default]
    Sanskrit,
    /// English, for a bug report going somewhere else.
    English,
}

impl Language {
    /// What `SANSOS_LANG` asks for, defaulting to Sanskrit.
    ///
    /// An unrecognised value is Sanskrit rather than an error: failing to run
    /// because someone mistyped a language name would be a worse diagnostic
    /// than any this module renders.
    #[must_use]
    pub fn from_env(value: Option<&str>) -> Self {
        match value {
            Some("en" | "english") => Language::English,
            _ => Language::Sanskrit,
        }
    }
}

/// Which letters to write the message in.
///
/// Not a language. `Script::Slp1` renders the same Sanskrit sentence in ASCII,
/// for a terminal with no Devanagari font — doc 03 §3.3.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Script {
    /// देवनागरी — the default.
    #[default]
    Devanagari,
    /// SLP1, one ASCII character per phoneme.
    Slp1,
}

impl Script {
    /// What the environment asks for: `SANSOS_SCRIPT` first, then the locale.
    ///
    /// `ctype` is `LC_ALL`/`LC_CTYPE`/`LANG`, whichever the caller found first.
    /// A locale that does not name UTF-8 cannot carry Devanagari at all, so
    /// that is a fact rather than a guess and the fallback takes over.
    ///
    /// **An unset locale stays Devanagari.** Not knowing whether a terminal can
    /// render the script is not evidence that it cannot, and defaulting to
    /// ASCII on absence would make ASCII the effective default on every system
    /// that leaves `LANG` unset — the tail wagging the dog again.
    #[must_use]
    pub fn from_env(script: Option<&str>, ctype: Option<&str>) -> Self {
        match script {
            Some("slp1" | "ascii") => return Script::Slp1,
            Some("devanagari" | "deva") => return Script::Devanagari,
            _ => {}
        }
        match ctype {
            Some(c) if !c.to_ascii_uppercase().replace('-', "").contains("UTF8") => Script::Slp1,
            _ => Script::Devanagari,
        }
    }
}

/// Whether a character belongs to a run the SLP1 encoder can take.
///
/// The main Devanagari block only. A sign from Devanagari Extended — the seven
/// A-033 measured at 0 of 27 faces — ends the run and passes through as itself,
/// so one unmappable character costs its own transliteration rather than the
/// whole message's.
fn is_devanagari(ch: char) -> bool {
    ('\u{0900}'..='\u{097F}').contains(&ch) || ch == '\u{200C}' || ch == '\u{200D}'
}

/// Rewrite a rendered message into `script`.
///
/// Devanagari is the identity. SLP1 transliterates each Devanagari run and
/// leaves everything else — backticks, em dashes, `ADR-0004`, a file name —
/// exactly as it was, because those are already legible on the terminal this
/// fallback exists for.
///
/// A run the encoder refuses is passed through unchanged. A compiler that
/// failed while explaining a mistake would have replaced the user's problem
/// with its own, which is the same rule [`Diagnostic::render`] follows for an
/// unregistered code.
///
/// This is display, not authoring. The output is not re-readable as source:
/// `ॱ` transliterates to `.`, so `क.sas` and `कॱsas` both come out `ka.sas`.
/// Orthographic closure (ADR-0001 D-2) forbids ASCII in a file; nothing here
/// writes a file.
#[must_use]
pub fn transliterate(message: &str, script: Script) -> String {
    if script == Script::Devanagari {
        return message.to_string();
    }
    fn flush(run: &mut String, out: &mut String) {
        if run.is_empty() {
            return;
        }
        let mut encoded = String::with_capacity(run.len());
        if sanskrit_text::slp1::encode_into(run, &mut encoded).is_ok() {
            out.push_str(&encoded);
        } else {
            out.push_str(run);
        }
        run.clear();
    }

    let mut out = String::with_capacity(message.len());
    let mut run = String::new();
    for ch in message.chars() {
        if is_devanagari(ch) {
            run.push(ch);
        } else {
            flush(&mut run, &mut out);
            out.push(ch);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// One fault, named by code and carrying the pieces that vary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable code, e.g. `L01`. Survives translation and rewording.
    pub code: &'static str,
    /// Substituted for `{0}`, `{1}`, … in written order.
    pub args: Vec<String>,
}

impl Diagnostic {
    /// Build one.
    #[must_use]
    pub fn new(code: &'static str, args: &[&str]) -> Self {
        Diagnostic {
            code,
            args: args.iter().map(|a| (*a).to_string()).collect(),
        }
    }

    /// The message, in the given language.
    ///
    /// A code absent from the registry renders as the code itself rather than
    /// panicking. A compiler that crashes while explaining a mistake has taken
    /// the user's problem and replaced it with its own.
    #[must_use]
    pub fn render(&self, lang: Language) -> String {
        let Some(row) = row(self.code) else {
            return alloc::format!("{}: (no message registered)", self.code);
        };
        let template = match lang {
            Language::Sanskrit => row.devanagari,
            Language::English => row.english,
        };
        let mut out = template.to_string();
        for (n, arg) in self.args.iter().enumerate() {
            out = out.replace(&alloc::format!("{{{n}}}"), arg);
        }
        out
    }
}

/// One registry row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// Stable code.
    pub code: &'static str,
    /// The `spec/lexicon.tsv` word this fault is named by.
    pub term: &'static str,
    /// Sanskrit template.
    pub devanagari: &'static str,
    /// English template.
    pub english: &'static str,
}

/// Every registered diagnostic.
#[must_use]
pub fn rows() -> Vec<Row> {
    DIAGNOSTICS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("code\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 4).then_some(Row {
                code: f[0],
                term: f[1],
                devanagari: f[2],
                english: f[3],
            })
        })
        .collect()
}

/// The row for a code.
#[must_use]
pub fn row(code: &str) -> Option<Row> {
    rows().into_iter().find(|r| r.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEXICON: &str = include_str!("../../../spec/lexicon.tsv");

    /// Every `term` is a lexicon word **filed under `error`**.
    ///
    /// The whole point of doc 01 §1.2.4. Without the membership half the `term`
    /// column is decoration and a new diagnostic can coin a word for something
    /// the lexicon named years ago — which is what a one-word-one-sense registry
    /// exists to stop.
    ///
    /// The category half was added by `A-037`, because membership alone lets a
    /// diagnostic name `योगः` — a real lexicon word, and a mnemonic. A fault is
    /// not an instruction, and the registry already says which is which; asking
    /// only "is it in the table" throws that away. All 47 diagnostics were
    /// already filed correctly, so this pins a property that held rather than
    /// fixing one that did not.
    #[test]
    fn every_diagnostic_names_a_word_the_lexicon_files_as_an_error() {
        let mut entries: alloc::vec::Vec<(&str, &str)> = alloc::vec::Vec::new();
        for l in LEXICON.lines().filter(|l| !l.starts_with('#')) {
            let mut f = l.split('\t');
            if let (Some(word), Some(category)) = (f.next(), f.nth(3)) {
                entries.push((word, category));
            }
        }
        for r in rows() {
            let found = entries.iter().find(|(w, _)| *w == r.term);
            match found {
                None => panic!(
                    "`{}` names `{}`, which is not in spec/lexicon.tsv",
                    r.code, r.term
                ),
                Some((_, category)) => assert_eq!(
                    *category, "error",
                    "`{}` names `{}`, which the lexicon files as `{}` rather than \
                     an error — a fault is not an instruction",
                    r.code, r.term, category
                ),
            }
        }
    }

    #[test]
    fn both_languages_use_the_same_placeholders() {
        // A translation that drops `{1}` produces a message naming no symbol at
        // all, and the reader has to reproduce the fault in the other language
        // to find out which one it meant.
        for r in rows() {
            for n in 0..4 {
                let p = alloc::format!("{{{n}}}");
                assert_eq!(
                    r.devanagari.contains(&p),
                    r.english.contains(&p),
                    "`{}` uses {p} in one language and not the other",
                    r.code
                );
            }
        }
    }

    #[test]
    fn a_code_is_never_reused() {
        let mut codes: Vec<&str> = rows().iter().map(|r| r.code).collect();
        let before = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(before, codes.len(), "two diagnostics share a code");
    }

    #[test]
    fn rendering_substitutes_every_argument_in_both_languages() {
        let d = Diagnostic::new("L01", &["क.sas", "चक्रः"]);
        for lang in [Language::Sanskrit, Language::English] {
            let m = d.render(lang);
            assert!(m.contains("क.sas"), "the file: {m}");
            assert!(m.contains("चक्रः"), "and the name: {m}");
            assert!(!m.contains('{'), "no placeholder survives: {m}");
        }
        // And they are different messages, or the table is not bilingual.
        assert_ne!(d.render(Language::Sanskrit), d.render(Language::English));
    }

    #[test]
    fn an_unregistered_code_does_not_panic() {
        // A compiler that crashes while explaining a mistake has replaced the
        // user's problem with its own.
        let m = Diagnostic::new("ZZ99", &[]).render(Language::Sanskrit);
        assert!(m.contains("ZZ99"), "{m}");
    }

    #[test]
    fn every_diagnostic_has_an_slp1_form_that_round_trips() {
        // The fallback is only a fallback if it covers the whole registry. A
        // message with one character outside the bijection would pass through
        // as itself and be a box on exactly the terminal this exists for.
        //
        // Round-tripped rather than merely encoded: SLP1 is a proved bijection
        // over its domain, so decoding back is available and is what proves the
        // transliteration lost nothing. Encoding alone would accept a mapping
        // that silently collapsed two words into one.
        let mut runs = 0;
        for r in rows() {
            for template in [r.devanagari, r.english] {
                let mut run = String::new();
                for ch in template.chars().chain(core::iter::once(' ')) {
                    if is_devanagari(ch) {
                        run.push(ch);
                        continue;
                    }
                    if run.is_empty() {
                        continue;
                    }
                    runs += 1;
                    let mut slp = String::new();
                    sanskrit_text::slp1::encode_into(&run, &mut slp).unwrap_or_else(|e| {
                        panic!("`{}`: {run:?} has no SLP1 form: {e:?}", r.code)
                    });
                    let mut back = String::new();
                    sanskrit_text::slp1::decode_into(&slp, &mut back)
                        .unwrap_or_else(|e| panic!("`{}`: {slp:?} does not decode: {e:?}", r.code));
                    assert_eq!(back, run, "`{}` does not round trip via {slp:?}", r.code);
                    run.clear();
                }
            }
        }
        // The English column carries Devanagari too — it names `ॱअ३२` and
        // `व्याप्ति` — so a count anywhere near the row count would mean the
        // scan found almost nothing and passed by doing nothing.
        assert!(runs > 2 * rows().len(), "only {runs} runs scanned");
    }

    #[test]
    fn the_fallback_transliterates_the_sanskrit_and_leaves_the_rest() {
        let d = Diagnostic::new("P09", &["कम्"]);
        let deva = d.render(Language::Sanskrit);
        let ascii = transliterate(&deva, Script::Slp1);

        assert!(
            !ascii.chars().any(is_devanagari),
            "still Devanagari: {ascii}"
        );
        // The argument travelled too. A fallback that transliterates the
        // template and not what it names leaves the reader the one word they
        // needed in the script they cannot read.
        assert!(ascii.contains("kam"), "the argument: {ascii}");
        // And the parts that were already legible are untouched, including the
        // rule reference a reader is being sent to.
        assert!(ascii.contains("ADR-0004"), "{ascii}");
        assert!(
            ascii.contains('—'),
            "the em dash is not Devanagari: {ascii}"
        );

        // Devanagari is the identity, or the default path is doing work.
        assert_eq!(transliterate(&deva, Script::Devanagari), deva);
        assert_ne!(ascii, deva, "the fallback changed nothing");
    }

    #[test]
    fn an_unmappable_sign_costs_its_own_run_and_not_the_message() {
        // `꣼` is Devanagari Extended, which A-033 measured in 0 of 27 faces and
        // ADR-0003 refused. Nothing writes one, but a diagnostic quoting source
        // could carry one in — and losing the whole sentence to it would be the
        // compiler failing while explaining a mistake.
        let m = transliterate("योगः\u{A8FC}खम्", Script::Slp1);
        assert!(m.starts_with("yogaH"), "{m}");
        assert!(m.ends_with("Kam"), "{m}");
        assert!(m.contains('\u{A8FC}'), "the sign itself survives: {m}");
    }

    #[test]
    fn a_message_with_no_devanagari_is_untouched() {
        // The English column is mostly ASCII, and rewriting it would be the
        // fallback inventing work.
        let plain = "E06: does not fit the field";
        assert_eq!(transliterate(plain, Script::Slp1), plain);
    }

    #[test]
    fn the_locale_decides_only_when_it_is_certain() {
        // An explicit ask wins over any locale.
        assert_eq!(
            Script::from_env(Some("slp1"), Some("en_US.UTF-8")),
            Script::Slp1
        );
        assert_eq!(
            Script::from_env(Some("deva"), Some("C")),
            Script::Devanagari
        );

        // A locale that cannot carry Devanagari is a fact, not a guess.
        assert_eq!(Script::from_env(None, Some("C")), Script::Slp1);
        assert_eq!(Script::from_env(None, Some("POSIX")), Script::Slp1);
        assert_eq!(
            Script::from_env(None, Some("en_US.ISO8859-1")),
            Script::Slp1
        );

        // UTF-8 might render it, spelled either way, so Devanagari stands.
        assert_eq!(
            Script::from_env(None, Some("en_US.UTF-8")),
            Script::Devanagari
        );
        assert_eq!(Script::from_env(None, Some("C.utf8")), Script::Devanagari);

        // And an unset locale is not evidence of anything.
        assert_eq!(Script::from_env(None, None), Script::Devanagari);
        assert_eq!(Script::default(), Script::Devanagari);
    }

    #[test]
    fn the_default_language_is_sanskrit() {
        assert_eq!(Language::from_env(None), Language::Sanskrit);
        assert_eq!(Language::from_env(Some("en")), Language::English);
        // A mistyped language must not stop the build.
        assert_eq!(Language::from_env(Some("klingon")), Language::Sanskrit);
    }
}
