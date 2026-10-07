//! **`अक्षरकोश ॱ परिधिदोषः` IS EXECUTED HERE** — R-15-1 over a whole source,
//! the walk the gated source entry `अक्षरकोशॱपरिधिपदविभाग` calls.
//!
//! # Provenance (W-304 replay, 2026-10-01)
//!
//! Ported from `agent/tick`'s `w304-pass-zero-scan.rs` (rescue ref
//! `rescue/w304-tick-2026-10-01e-99606658`), whose subject was `सङ्ग्रहदोषः`.
//! The walk itself was ported onto main's `परिधिदोषः`, and so were these tests —
//! with their ARITHMETIC rewritten, not their names: `सङ्ग्रहदोषः` answered a
//! ONE-BASED position and `०` for clean; `परिधिदोषः` answers the ZERO-BASED
//! offset of the first refusing octet and `मूल ॱ दैर्घ्य` for clean (`न६४` is
//! unsigned, ADR-0030), and `परिधिदोषकारणम्` names the cause. Every expected
//! offset below is still Rust's own `find`, never a constant.
//!
//! # What it claims
//!
//! The comment carve-out, the `आस्की`/`जाल` text directive, ADR-0017's
//! string-first cut and ADR-0011's `इति` parity, each graded against the real
//! Rust lexer (`lex_t1`) or `in_repertoire` — and the three-state cause: clean,
//! a foreign octet, a `उक्तम्` no `इति` closes.
//!
//! The one stated residual is NARROWER than Rust and never permissive:
//! `split_trailing_punct` peels ONE suffix, so Rust closes a literal written
//! `इति।` and resets a directive on `॥।` — the walk reads the last code point
//! only and does neither. `every_t1_source_in_this_crate_scans_clean` is the
//! instrument that would catch it if a source ever wrote one, because the
//! consequence there is a FALSE REFUSAL and not a hole.
//!
//! # THE ORACLE IS THE RUST SIDE
//!
//! For the verdict on a code point the oracle is `in_repertoire` itself:
//! [`first_violation_rust`] never lists a range. For lexer STATE (a directive,
//! a literal) the oracle is `lex_t1`, because a directive is a lexer state and a
//! second model of it written here would agree with any mistake the port has.

use sadhana::lex::lex_t1;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sanskrit_text::repertoire::in_repertoire;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn source(name: &str) -> String {
    let p = src_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// **THIS FILE'S OWN LOADER.** `w304-pass-zero.rs` has one of its own and the
/// two are deliberately not shared: a loader is part of the fixture, and the
/// set of modules that must be present is a fact about the routine under test.
///
/// `sanskrit_text.t1` opens with `आयातः सङ्केतन ।`, so `encode.t1` must load
/// beside it or the module does not resolve; `encode.t1` carries its own
/// `समावेशः` of `registers-riscv64.tsv` and embeds resolve for every module at
/// load time, so the real `spec/` root is required even though `परिधिदोषः`
/// reads no table.
fn load() -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", source("lex.t1").as_str()),
            ("encode.t1", source("encode.t1").as_str()),
            ("sanskrit_text.t1", source("sanskrit_text.t1").as_str()),
        ],
        &spec_root(),
    )
    .expect("lex.t1, encode.t1 and sanskrit_text.t1 load together")
}

/// A step budget big enough for the longest source in the crate.
///
/// The walk is one pass per code point with a handful of comparisons each, so
/// this is generous rather than tuned. A budget too small would surface as a
/// refusal naming the step limit, not as a wrong answer.
const STEPS: u64 = 2_000_000_000;

/// The zero-based offset `परिधिदोषः` answers, `None` for a clean source.
///
/// The routine answers the offset of the first refusing octet, or the source's
/// LENGTH when clean — one past the last octet, never a real position, so the
/// two cannot collide without a sentinel `न६४` does not have. This is the
/// arithmetic the port from `सङ्ग्रहदोषः` changed: that routine answered
/// one-based with `०` for clean, and its `n - 1` lived here.
fn first_violation_t1(it: &mut Interpreter, text: &str) -> Option<usize> {
    let answer = it
        .call(
            "अक्षरकोशॱपरिधिदोषः",
            vec![Value::Octets(Octets::new(text.as_bytes()))],
            STEPS,
        )
        .unwrap_or_else(|e| panic!("परिधिदोषः refused a {} octet source: {e:?}", text.len()));
    let n = usize::try_from(answer.as_int().expect("परिधिदोषः answers an अङ्क"))
        .expect("an offset is non-negative");
    assert!(
        n <= text.len(),
        "परिधिदोषः answered {n}, past the {} octet source — not an offset",
        text.len()
    );
    (n < text.len()).then_some(n)
}

/// The same rule in Rust, and **the per-character verdict is `in_repertoire`,
/// never a range written here.**
///
/// The only thing this routine transcribes is the COMMENT CARVE-OUT — from `॰`
/// to the next line feed is not checked — which is three lines of `pieces`. The
/// verdict it carves out of is asked of the product.
///
/// **IT DELIBERATELY DOES NOT MODEL ADR-0017's STRING-FIRST CUT**, so it is
/// wrong by construction on `उक्तम् ॰ SAS इति` and
/// [`both_ports_are_in_and_a_comment_mark_inside_a_literal_is_text`] asserts
/// that it is. A second transcription of a rule that is already ported would
/// agree with whatever mistake the port carries. It stays a valid oracle over
/// the corpus on one PREMISE, and the premise is asserted rather than assumed
/// in [`no_corpus_line_writes_a_comment_mark_after_a_code_position_opener`]: no
/// `.t1` source in this crate writes a `॰` after a `उक्तम्` that is not itself
/// already commented out, so the two rules cannot disagree on any of them.
fn first_violation_rust(text: &str) -> Option<usize> {
    let mut in_comment = false;
    for (at, ch) in text.char_indices() {
        if ch == '\n' {
            in_comment = false;
        } else if ch == '॰' {
            in_comment = true;
        } else if !in_comment && !in_repertoire(ch) {
            return Some(at);
        }
    }
    None
}

/// **THE PREMISE THAT KEEPS [`first_violation_rust`] A VALID ORACLE.**
///
/// It models the comment carve-out and not ADR-0017's cut, so it and pass 0
/// differ exactly on a `॰` that falls INSIDE a `उक्तम् … इति`. No source in this
/// crate writes one — every mention of the delimiters in the corpus sits in a
/// margin, where no literal opens — so over the corpus the two rules cannot
/// disagree and [`every_t1_source_in_this_crate_scans_clean`] grades honestly.
///
/// Asserted and not assumed: the day a source does write one, this fails by
/// NAME and says what to do about it, instead of the corpus walk quietly
/// grading pass 0 against a model that no longer describes the front end.
///
/// The scan is the same line-local cut as `code_only` in `w304-pass-zero.rs`,
/// which is deliberately NOT shared: there it is the subject, here it is a
/// premise about the corpus.
#[test]
fn no_corpus_line_writes_a_comment_mark_after_a_code_position_opener() {
    let mut names: Vec<String> = std::fs::read_dir(src_dir())
        .expect("src/ is readable")
        .map(|e| e.expect("a dir entry").file_name().to_string_lossy().into())
        .filter(|n: &String| n.ends_with(".t1"))
        .collect();
    names.sort();
    assert!(
        names.len() >= 20,
        "the corpus is 21 sources; found {}",
        names.len()
    );

    // The control: the scan DOES find the shape when the shape is there, so an
    // empty result below is absence and not a broken search.
    assert!(
        offenders("x.t1", "    प्रत्यागमनम् उक्तम् ॰ क इति ।\n") == 1,
        "the search must see a ॰ after a code-position उक्तम्"
    );
    assert!(
        offenders("x.t1", "    ॰ उक्तम् ॰ क इति is only named here\n") == 0,
        "and must not see one inside a margin, which is every corpus mention"
    );

    let mut found = Vec::new();
    for name in &names {
        if offenders(name, &source(name)) > 0 {
            found.push(name.clone());
        }
    }
    assert!(
        found.is_empty(),
        "{found:?} writes a ॰ inside a `उक्तम् … इति`, so first_violation_rust \
         is no longer a valid oracle over the corpus — model the string-first \
         cut there, or move that file's grading to Interpreter::load alone"
    );
}

/// Lines where a `॰` follows a `उक्तम्` that is not itself commented out.
fn offenders(_name: &str, text: &str) -> usize {
    text.lines()
        .filter(|line| match (line.find("उक्तम्"), line.find('॰')) {
            (Some(u), Some(m)) => u < m,
            _ => false,
        })
        .count()
}

/// Every `.t1` source in this crate scans clean — the carve-out's whole point.
///
/// The oracle is `Interpreter::load`: all of them load today (this file's own
/// [`load`] proves three do), so the Rust front end finds no violation in any
/// of them, so neither may pass 0. Without the comment carve-out every one of
/// these files would be refused, which is the measurement that produced the
/// carve-out in the first place.
///
/// Graded against [`first_violation_rust`] on the same text as well, so a file
/// that is clean for the wrong reason — a walk that terminated early, a budget
/// that ran out and was swallowed — cannot read as a pass.
#[test]
fn every_t1_source_in_this_crate_scans_clean() {
    let mut it = load();
    let mut names: Vec<String> = std::fs::read_dir(src_dir())
        .expect("src/ is readable")
        .map(|e| e.expect("a dir entry").file_name().to_string_lossy().into())
        .filter(|n: &String| n.ends_with(".t1"))
        .collect();
    names.sort();
    assert!(
        names.len() >= 20,
        "the corpus is 21 sources; the walk found {} — {names:?}",
        names.len()
    );

    let mut dirty = Vec::new();
    for name in &names {
        let text = source(name);
        let rust = first_violation_rust(&text);
        let t1 = first_violation_t1(&mut it, &text);
        assert_eq!(
            t1,
            rust,
            "{name}: pass 0 answered {t1:?} and the Rust rule {rust:?} over the \
             same {} octets",
            text.len()
        );
        if let Some(at) = t1 {
            let ch = text[at..].chars().next().expect("a char at a boundary");
            dirty.push(format!("{name} octet {at} U+{:04X} `{ch}`", u32::from(ch)));
        }
    }
    assert!(
        dirty.is_empty(),
        "pass 0 refuses sources the Rust loader accepts — the comment carve-out \
         is not doing its job: {dirty:?}"
    );
}

/// W-304's own reproduction, LOCATED and not merely refused.
///
/// The expected offset is Rust's byte offset of the first `S`, computed with
/// `find`, so this cannot pass by answering a constant.
#[test]
fn the_literal_w304_was_filed_over_is_located_by_the_scan() {
    let mut it = load();
    let text = "मण्डलम् परीक्षा ॥\n\n\
                सार्वजनिक वृत्तिः मुख्यम् ददाति अङ्कः अन्तः अ८ आदि\n    \
                प्रत्यागमनम् उक्तम् SAS इति ।\nइति\n";
    let want = text.find('S').expect("the fixture contains the S");
    assert_eq!(
        first_violation_t1(&mut it, text),
        Some(want),
        "the scan must point at the first `S`, octet {want}"
    );
    assert_eq!(
        first_violation_rust(text),
        Some(want),
        "and the Rust rule agrees on the same octet"
    );

    // The control: the same module without the literal. ० .
    let clean = "मण्डलम् परीक्षा ॥\n\n\
                 सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि\n    \
                 प्रत्यागमनम् ० ।\nइति\n";
    assert_eq!(
        first_violation_t1(&mut it, clean),
        None,
        "a module with no foreign character is clean, so the refusal above is \
         about the ASCII and not about the module"
    );
    assert!(
        Interpreter::load(&[("pariksha.t1", clean)], &spec_root()).is_ok(),
        "and the Rust front end agrees that control loads"
    );
}

/// The mutation BOTH ways: the same four letters, in a comment and in code.
#[test]
fn ascii_in_a_comment_is_carved_out_and_the_same_ascii_in_code_is_not() {
    let mut it = load();
    let commented = "मण्डलम् परीक्षा ॥\n\
                     ॰ SAS and more ASCII: build(), lex.rs:583, O-15-2\n\
                     सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि\n    \
                     प्रत्यागमनम् ० ।\nइति\n";
    assert_eq!(
        first_violation_t1(&mut it, commented),
        None,
        "ASCII after ॰ is carved out, exactly as pieces carves it out"
    );
    assert!(
        Interpreter::load(&[("pariksha.t1", commented)], &spec_root()).is_ok(),
        "and the real loader accepts it, which is why the carve-out exists"
    );

    // Drop the ॰ and nothing else. The SAME letters must now be refused, at the
    // offset the ॰ used to occupy.
    let bare = commented.replacen("॰ SAS", "SAS", 1);
    assert_ne!(bare, commented, "the mutation changed the source");
    let want = bare.find('S').expect("the S survived the mutation");
    assert_eq!(
        first_violation_t1(&mut it, &bare),
        Some(want),
        "without the ॰ the same letters are a violation at octet {want}"
    );
    assert!(
        Interpreter::load(&[("pariksha.t1", bare.as_str())], &spec_root()).is_err(),
        "and the real loader refuses it too"
    );

    // And a comment ends at the LINE FEED, not at the end of the file: the
    // letters on the NEXT line are still checked.
    let next_line = "मण्डलम् परीक्षा ॥\n॰ a comment\nSAS\n";
    let want = next_line.find('S').expect("the S is on line three");
    assert_eq!(
        first_violation_t1(&mut it, next_line),
        Some(want),
        "the carve-out must close at the line feed, or one ॰ anywhere disables \
         pass 0 for the whole file"
    );
}

/// A lead octet promising octets the source does not have is REFUSED at the
/// lead, not decoded past the end.
///
/// This is the one arm of `परिधिदोषः` that never calls `परिधिस्थम्`, and the
/// reason is both safety — `सङ्केतमूल्यम्` would index past `दैर्घ्य` — and
/// correctness: a truncated sequence denotes no code point at all, so it
/// denotes none in R-15-1.
#[test]
fn a_sequence_running_past_the_end_is_refused_at_its_lead_octet() {
    let mut it = load();
    // `क` is U+0915, three octets. Keep the first two.
    let whole = "मण्डलम् क".as_bytes().to_vec();
    let cut = &whole[..whole.len() - 1];
    let lead = cut.len() - 2;
    assert_eq!(
        cut[lead], 0xE0,
        "the surviving lead octet of a 3-octet rune"
    );

    let answer = it
        .call(
            "अक्षरकोशॱपरिधिदोषः",
            vec![Value::Octets(Octets::new(cut))],
            STEPS,
        )
        .expect("परिधिदोषः accepts a truncated source rather than trapping");
    // ZERO-BASED: the lead octet itself, where `सङ्ग्रहदोषः` answered lead + 1.
    assert_eq!(
        usize::try_from(answer.as_int().expect("an अङ्क")).expect("non-negative"),
        lead,
        "the truncated sequence must be refused at its lead octet {lead}"
    );
}

/// **BOTH PORTS ARE IN: THE TEXT DIRECTIVE AND ADR-0017's STRING-FIRST CUT.**
///
/// This was `the_text_directive_is_ported_and_the_remaining_gap_is_measured`,
/// and before that `the_two_unported_gaps_are_measured_against_the_real_lexer`.
/// Each name pinned a gap that was still PERMISSIVE and each went red when the
/// gap closed, which is what an inversion point is for. There is no third gap
/// left to pin, so the name no longer carries one.
///
/// 1. **`आस्की`/`जाल`.** The directive is accepted, and the oracle is [`lex_t1`]
///    rather than a reading of it — a directive is a lexer state, so the lexer
///    is the only thing that can say what it does.
/// 2. **ADR-0017.** `pieces` recognises `उक्तम् … इति` BEFORE it looks for the
///    comment mark, so a `॰` between the delimiters is TEXT and everything
///    after it on that line is still checked. `उक्तम् ॰ SAS इति` was ACCEPTED by
///    pass 0 and refused by the front end — wrong in the direction that lets
///    something through. Now both refuse it, at the same octet, and the octet
///    is Rust's own `find` rather than a constant written here.
#[test]
fn both_ports_are_in_and_a_comment_mark_inside_a_literal_is_text() {
    let mut it = load();

    // (1) The directive, in the shape the ten spec/*.sas files write it.
    let directive = "मण्डलम् परीक्षा ॥\n\
                     ॥ आस्की BOOT-COUNTER-FRESH ॥\n\
                     सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि\n    \
                     प्रत्यागमनम् ० ।\nइति\n";
    assert!(
        lex_t1(directive).is_ok(),
        "the premise: the real lexer accepts `॥ आस्की … ॥`"
    );
    assert_eq!(
        first_violation_t1(&mut it, directive),
        None,
        "and pass 0 accepts it too"
    );

    // (2) ADR-0017. The `॰` is inside the literal, so the SAS after it is code
    // and the scan must point at its first letter.
    let in_string = "मण्डलम् परीक्षा ॥\n\n\
                     सार्वजनिक वृत्तिः मुख्यम् ददाति अङ्कः अन्तः अ८ आदि\n    \
                     प्रत्यागमनम् उक्तम् ॰ SAS इति ।\nइति\n";
    let want = in_string.find('S').expect("the fixture contains the S");
    assert_eq!(
        first_violation_t1(&mut it, in_string),
        Some(want),
        "a ॰ inside `उक्तम् … इति` is TEXT, so the SAS after it is still \
         checked — and it is refused at octet {want}"
    );
    let err = format!(
        "{:?}",
        Interpreter::load(&[("pariksha.t1", in_string)], &spec_root())
            .err()
            .expect("the real lexer refuses it, because the string is cut first")
    );
    assert!(
        err.contains("outside the doc 15 repertoire"),
        "and it refuses it ON THE REPERTOIRE, which is what made this a pass 0 \
         gap and not a parse difference; it said {err}"
    );
    assert_eq!(
        first_violation_rust(in_string),
        None,
        "and [`first_violation_rust`] still models the COMMENT carve-out only, \
         so it is wrong here on purpose — see its own margin for the premise \
         that keeps it a valid oracle over the corpus"
    );
}

/// **THE CASES ADR-0017's CUT MUST STILL REFUSE — written WITH the port.**
///
/// A string rule that only ever OPENED would pass every arm of
/// [`both_ports_are_in_and_a_comment_mark_inside_a_literal_is_text`] and fail
/// here, because the three things that can go wrong with it are all about
/// where it STOPS:
///
/// * **It is LINE-LOCAL, and that carries `pieces`'s own refusal.** A `उक्तम्`
///   that no `इति` closes on its line is an ERROR there — reported at the
///   `उक्तम्` itself, with no word of that line checked — not a literal that
///   runs to the next line or to the end of the file. So pass 0 owes that
///   refusal and owes it AT THE SAME OCTET, which is why an interior violation
///   is held rather than returned.
/// * **ADR-0011, BOTH WAYS.** A pair of `इति` is a literal `इति` and the
///   literal continues; the first unpaired one closes. An EVEN run must not
///   close (so the `॰` after it is still text) and an ODD run must (so the `॰`
///   after it is a comment mark again). One arm each, same four letters.
/// * **The opener is a WHOLE WORD and never a commented-out one.** `उक्तम्कम्`
///   opens nothing, and neither does a `उक्तम्` that a `॰` already commented
///   out — otherwise every margin in the corpus naming the delimiter would
///   open a literal and the carve-out would collapse.
#[test]
fn the_literal_is_line_local_and_adr_0011_closes_it_on_an_odd_run() {
    let mut it = load();

    // Both engines, one grading. A `उक्तम्` with no close is not a repertoire
    // error, so this accepts either reason and grades on the BYTE.
    let mut graded = |label: &str, text: &str| {
        let rust = match lex_t1(text) {
            Ok(_) => None,
            Err(errs) => Some(
                errs.iter()
                    .filter(|e| {
                        e.reason.contains("repertoire") || e.reason.contains("closes on this line")
                    })
                    .map(|e| e.byte)
                    .min()
                    .unwrap_or_else(|| panic!("{label}: refused for neither reason: {errs:?}")),
            ),
        };
        let t1 = first_violation_t1(&mut it, text);
        assert_eq!(
            t1, rust,
            "{label}: pass 0 said {t1:?}, the real lexer {rust:?}"
        );
        t1
    };

    // LINE-LOCAL, arm one: no `इति` at all. Refused AT THE `उक्तम्` — and the
    // `SAS` further along the line is NOT the answer, which is the whole
    // reason an interior violation is held.
    let open = "मण्डलम् परीक्षा ॥\nप्रत्यागमनम् उक्तम् ॰ SAS ।\nइति\n";
    let at = open.find("उक्तम्").expect("the opener");
    assert_eq!(
        graded("a literal no इति closes", open),
        Some(at),
        "refused at the उक्तम् at octet {at}, not at the SAS at {:?}",
        open.find('S')
    );

    // LINE-LOCAL, arm two: the `इति` is on the NEXT line, which does not count.
    // `pieces` is called per line, so the literal cannot reach it.
    let across = "मण्डलम् परीक्षा ॥\nप्रत्यागमनम् उक्तम् क\nइति ।\nइति\n";
    let at = across.find("उक्तम्").expect("the opener");
    assert_eq!(
        graded("a literal closed on the next line", across),
        Some(at),
        "a literal does not run past its own line end"
    );

    // ADR-0011, the EVEN run: `इति इति` is a literal `इति` and the literal
    // continues, so the `॰` after it is still text and the SAS is checked.
    let doubled = "मण्डलम् परीक्षा ॥\nप्रत्यागमनम् उक्तम् इति इति ॰ SAS इति ।\nइति\n";
    let want = doubled.find('S').expect("the SAS");
    assert_eq!(
        graded("an even run of इति does not close", doubled),
        Some(want),
        "the pair is text, so the ॰ after it is text too"
    );

    // ADR-0011, the ODD run: the third `इति` is unpaired and CLOSES, so the
    // same `॰ SAS` is a comment again. The same four letters, the one
    // character of difference being an `इति` — a port that simply never closed
    // would refuse this.
    let odd = "मण्डलम् परीक्षा ॥\nप्रत्यागमनम् उक्तम् इति इति इति ॰ SAS\nइति\n";
    assert_eq!(
        graded("an odd run closes and the mark is a comment again", odd),
        None,
        "the third इति closed the literal, so the ॰ is a comment mark"
    );

    // A WHOLE WORD: `उक्तम्कम्` is an ordinary identifier and opens nothing, so
    // the `॰` after it still starts a comment.
    assert_eq!(
        graded(
            "a word beginning with the opener",
            "मण्डलम् परीक्षा ॥\nप्रत्यागमनम् उक्तम्कम् ॰ SAS\nइति\n"
        ),
        None,
        "उक्तम्कम् is not the opener"
    );

    // AND NOT A COMMENTED-OUT ONE. Every margin in the corpus that names the
    // delimiter is this shape; if one opened a literal, the comment carve-out
    // would collapse on the corpus and `every_t1_source_in_this_crate_scans_clean`
    // would be the one to say so.
    assert_eq!(
        graded(
            "a commented-out opener",
            "मण्डलम् परीक्षा ॥\n॰ उक्तम् ॰ SAS इति\nयोगः कम् खन ।\n"
        ),
        None,
        "a उक्तम् inside a comment opens nothing"
    );
}

/// **THE CASES THE DIRECTIVE MUST STILL REFUSE.** Written with the port, not
/// after it: an exemption that cannot be closed is not an exemption, it is a
/// hole, and five of these six would pass against a routine that simply stopped
/// checking at the first `आस्की` it ever saw.
///
/// Every arm is graded against [`lex_t1`] on the same text. That is the only
/// oracle that can settle a question about lexer STATE — `first_violation_rust`
/// models the comment carve-out and nothing else, and a second model of
/// `in_ascii` written here would agree with any mistake this port carries.
#[test]
fn the_directive_closes_and_a_word_that_merely_begins_with_it_is_not_one() {
    let mut it = load();

    // Both engines, one grading. `want` is Rust's own byte offset or None.
    let mut graded = |label: &str, text: &str| {
        let rust = match lex_t1(text) {
            Ok(_) => None,
            Err(errs) => Some(
                errs.iter()
                    .filter(|e| e.reason.contains("repertoire"))
                    .map(|e| e.byte)
                    .min()
                    .unwrap_or_else(|| {
                        panic!("{label}: refused, but not on the repertoire: {errs:?}")
                    }),
            ),
        };
        let t1 = first_violation_t1(&mut it, text);
        assert_eq!(
            t1, rust,
            "{label}: pass 0 said {t1:?}, the real lexer {rust:?}"
        );
        t1
    };

    // The closing ॥ restores the check: the directive does not run to the end
    // of the file. A port that only ever SET the state passes everything above
    // and fails here.
    assert!(
        graded(
            "reset at the closing danda",
            "॥ आस्की FRESH ॥ योगः कम् खन ।\nप्रत्यागमनम् X ।\n"
        )
        .is_some(),
        "ASCII after the closing ॥ is refused"
    );

    // ...and the ASCII inside it is not. Same line, the directive's own word.
    assert_eq!(
        graded(
            "the directive's own payload",
            "॥ आस्की FRESH ॥ योगः कम् खन ।\n"
        ),
        None
    );

    // `word == "आस्की"`, not `starts_with`: a word that merely begins with the
    // directive is an ordinary word and turns nothing on.
    assert!(
        graded("a word beginning with the directive", "॥ आस्कीकम् FRESH ॥\n").is_some(),
        "आस्कीकम् is not the directive"
    );

    // A commented-out directive turns nothing on — `pieces` strips the comment
    // before the word is ever compared.
    assert!(
        graded("a commented-out directive", "॰ ॥ आस्की FRESH ॥\nयोगः X ।\n").is_some(),
        "a directive inside a comment is not a directive"
    );

    // जाल NARROWS the gate rather than opening it: the twelve marks and the
    // ASCII letters pass, and the first mark outside that list does not.
    assert_eq!(
        graded("jal's exempted set", "॥ जाल VIRTIO-NET-OK ॥\n"),
        None
    );
    assert!(
        graded("jal does not exempt everything", "॥ जाल a#b ॥\n").is_some(),
        "`#` is not one of the twelve, so जाल still refuses it"
    );
}

/// **THE DIRECTIVE DOES NOT RESET AT A LINE END.**
///
/// `in_ascii` lives outside `lex_with`'s line loop, so a directive left open
/// runs to the next `॥` however many lines later — and the octet walk inherits
/// that only because its state lives outside the line too. Its own test because
/// the failure mode is invisible in the one-line shape every `spec/*.sas` writes.
#[test]
fn an_open_directive_runs_past_the_line_end() {
    let mut it = load();
    let across = "॥ आस्की FIRST\nSECOND\nTHIRD ॥\nयोगः कम् खन ।\n";
    assert!(
        lex_t1(across).is_ok(),
        "the premise: the real lexer accepts it"
    );
    assert_eq!(
        first_violation_t1(&mut it, across),
        None,
        "three lines of ASCII under one unclosed आस्की"
    );

    // And the close still lands: ASCII after the third line's ॥ is refused.
    let closed = "॥ आस्की FIRST\nSECOND ॥\nX\n";
    let want = closed.rfind('X').expect("the trailing ASCII");
    assert_eq!(
        first_violation_t1(&mut it, closed),
        Some(want),
        "the ॥ on the second line closes the directive opened on the first"
    );
}

/// The cause `परिधिदोषः` recorded for its last answer, read through `global`.
///
/// `global` answers `None` for a name no module declared, so a misspelt name
/// here cannot read as `०` — "clean" — the way a raw offset would.
fn cause(it: &Interpreter) -> i128 {
    it.global("परिधिदोषकारणम्")
        .and_then(Value::as_int)
        .expect("sanskrit_text.t1 declares परिधिदोषकारणम्")
}

/// A declared `न६४` constant of the module under test, read the same way.
fn konstant(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("sanskrit_text.t1 declares {name}"))
}

/// `W-304` — **ONE POSITION, TWO CAUSES, AND THEY ARE NOW TOLD APART.**
///
/// `परिधिदोषः` answers a single octet offset for two unrelated refusals: an
/// octet doc 15 does not admit, and a `उक्तम्` that no `इति` closes. That is a
/// two-state instrument — refused or not — over a truth with THREE states, and
/// it hid its own breakage: `पदविभाग`'s gate read the position alone, so an
/// unclosed literal was refused BEFORE the walk and `ashtaka`'s `P22` — the
/// pass that owns that refusal and reports it with the word — never ran. This
/// row holds the distinction itself: same shape of answer, different cause.
#[test]
fn the_scan_says_which_of_its_two_causes_refused_the_source() {
    let mut it = load();

    let repertoire = konstant(&it, "परिधिअक्षरदोषः");
    let unclosed = konstant(&it, "परिधिउक्तदोषः");
    assert_ne!(
        repertoire, unclosed,
        "two causes that share a code would tell nothing apart"
    );

    // (a) CLEAN — no position, and the cause is ० and not a stale one.
    let clean = "मण्डलम् परीक्षा ॥\n\n\
                 सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि\n    \
                 प्रत्यागमनम् ० ।\nइति\n";
    assert_eq!(first_violation_t1(&mut it, clean), None, "a clean source");
    assert_eq!(cause(&it), 0, "and no cause is recorded for it");

    // (b) A FOREIGN OCTET — the position is the first `S`, the cause is the
    // repertoire. Rust's `find` computes the offset, so a constant cannot pass.
    let foreign = "मण्डलम् परीक्षा ॥\n\n\
                   सार्वजनिक वृत्तिः मुख्यम् ददाति अङ्कः अन्तः अ८ आदि\n    \
                   प्रत्यागमनम् उक्तम् SAS इति ।\nइति\n";
    let want = foreign.find('S').expect("the fixture contains the S");
    assert_eq!(
        first_violation_t1(&mut it, foreign),
        Some(want),
        "the foreign octet is located"
    );
    assert_eq!(
        cause(&it),
        repertoire,
        "and doc 15's repertoire is why it was refused"
    );

    // (c) A `उक्तम्` NO `इति` CLOSES — refused at the `उक्तम्`, and the cause is
    // the STRUCTURE one. Every octet of this source is in the repertoire, so a
    // scan that answered the repertoire cause here would be answering about a
    // violation that is not present.
    let open = "॥ अष्टकाः उक्तम् क ॥\n";
    let at = open.find("उक्तम्").expect("the fixture contains the उक्तम्");
    assert_eq!(
        first_violation_t1(&mut it, open),
        Some(at),
        "reported AT the उक्तम् that was left open"
    );
    assert_eq!(
        cause(&it),
        unclosed,
        "and the unclosed literal is why — not the repertoire"
    );
    // AND THE REPERTOIRE RULE ITSELF CALLS THIS SOURCE CLEAN. `first_violation_rust`
    // is `in_repertoire` per code point plus the comment carve-out and NOTHING else,
    // so its `None` here is the independent statement that no octet of this source is
    // foreign. The two instruments differ because they are measuring different
    // things, which is the whole reason the cause has to be carried separately: the
    // T1 routine refuses this source and the repertoire does not.
    assert_eq!(
        first_violation_rust(open),
        None,
        "the repertoire rule finds nothing foreign — the refusal above is structural"
    );

    // (c2) THE PROBE THAT SETTLED THE THIRD STATE (W-304 replay, probe B3):
    // an unclosed literal with ASCII INSIDE it. Before the port, main's
    // `परिधिदोषः` answered the `S` — a plausible number with the WRONG cause.
    // `lex_t1` refuses the line at the `उक्तम्` and never checks a word of it,
    // so the answer must be the `उक्तम्`, and the cause the structure one.
    let open_ascii = "मण्डलम् परीक्षणम् ॥\n\nसार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    \
                      चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् SAS\n    प्रत्यागमनम् २१ ।\nइति\n";
    let at = open_ascii.find("उक्तम्").expect("the opener");
    assert_eq!(
        first_violation_t1(&mut it, open_ascii),
        Some(at),
        "an unclosed literal is answered AT the उक्तम् (octet {at}), not at the \
         S inside it ({:?})",
        open_ascii.find('S')
    );
    assert_eq!(
        cause(&it),
        unclosed,
        "and its cause is the unclosed literal, not the repertoire"
    );
    let rust = format!("{:?}", lex_t1(open_ascii).expect_err("lex_t1 refuses it"));
    assert!(
        rust.contains("closes on this line") && !rust.contains("repertoire"),
        "the premise: the real lexer refuses this on the STRUCTURE alone: {rust}"
    );

    // (d) THE CAUSE IS RESET, NOT LATCHED: the clean source again, after three
    // refusals, must answer clean — `परिधिदोषः` resets its cause on entry.
    assert_eq!(first_violation_t1(&mut it, clean), None, "clean again");
    assert_eq!(cause(&it), 0, "the previous source's cause is not carried");
}

/// `W-304` — **THE GATED SOURCE ENTRY STOPS A FOREIGN OCTET AND LETS A
/// STRUCTURE REFUSAL THROUGH, which is the whole point of telling the causes
/// apart.**
///
/// `अक्षरकोशॱपरिधिपदविभाग` answers ० both for "refused" and for "this source
/// lexed to nothing", and `परिधिदोषस्थितम्` is what tells those apart. The gate
/// must fire for (b) and must NOT fire for (c): the lexer and the parser — and
/// `ashtaka`'s `P22` for `.sas` — refuse an unclosed literal with the word and
/// the column, and a gate that swallowed the source first replaced that named
/// diagnostic with an empty answer. `agent/tick` measured exactly that, as
/// `errors()` answering `[]` in `t1_ashtaka_wired.rs`.
#[test]
fn the_gated_entry_refuses_the_repertoire_and_not_the_unclosed_literal() {
    let mut it = load();

    let lex_of = |it: &mut Interpreter, text: &str| -> (i128, bool, i128) {
        let n = it
            .call(
                "अक्षरकोशॱपरिधिपदविभाग",
                vec![Value::Octets(Octets::new(text.as_bytes()))],
                STEPS,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "परिधिपदविभाग runs over a {} octet source: {e:?}",
                    text.len()
                )
            })
            .as_int()
            .expect("परिधिपदविभाग answers how many tokens were written");
        let stopped = matches!(it.global("परिधिदोषस्थितम्"), Some(Value::Bool(true)));
        let at = it
            .global("परिधिदोषस्थानम्")
            .and_then(Value::as_int)
            .expect("sanskrit_text.t1 declares परिधिदोषस्थानम्");
        (n, stopped, at)
    };

    // A FOREIGN OCTET STOPS THE WALK — no tokens, and the record says why ०.
    let foreign = "॥ अष्टकाः उक्तम् SAS इति ॥\n";
    let want =
        i128::try_from(foreign.find('S').expect("the fixture has the S")).expect("an offset fits");
    assert_eq!(
        lex_of(&mut it, foreign),
        (0, true, want),
        "a foreign octet is refused before the walk, at the zero-based offset of the S"
    );

    // AN UNCLOSED LITERAL DOES NOT — the walk runs, tokens exist, and the
    // record stays clear because no octet was outside the repertoire. Run right
    // after the refusal, so it also proves the record is RESET and not latched.
    let open = "॥ अष्टकाः उक्तम् क ॥\n";
    let (tokens, stopped, at) = lex_of(&mut it, open);
    assert!(
        tokens > 0,
        "the walk must run so the pass that owns the refusal can name it, got {tokens} tokens"
    );
    assert_eq!(
        (stopped, at),
        (false, 0),
        "परिधिदोषस्थितम् is a REPERTOIRE verdict and this source has no foreign octet"
    );
}
