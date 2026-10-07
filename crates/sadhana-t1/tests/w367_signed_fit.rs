//! **W-367 — `वाक्यविभागॱअन्तर्भाववाचकः` asked a signed question with an
//! operator whose signedness the two engines answer differently.**
//!
//! # What was wrong
//!
//! `vakyavibhaga.t1`'s fit check (W-082's arm, `vakyavibhaga.t1:3328`) decides
//! whether a datum is writable in `व्याप्तिः` octets. Its UNSIGNED half is
//! `मूल्यम् दक्षिणसृ अंशसंख्या समम् ०`. Its SIGNED half was
//!
//! ```text
//! उच्चांशाः = मूल्यम् दक्षिणसृ (अंशसंख्या वियोगः १)
//! सर्वैकाः  = (१ वामसृ (६५ वियोगः अंशसंख्या)) वियोगः १
//! fits iff उच्चांशाः समम् सर्वैकाः
//! ```
//!
//! — "are all the bits from the sign upward set", asked by shifting the sign
//! bit DOWN and naming the ones it should leave. That is only right where
//! `दक्षिणसृ` is LOGICAL. It is logical in the interpreter, whose `Value::Int`
//! is an `i128` holding the 64-bit pattern as a positive number, and it is
//! ARITHMETIC in a register, where `sra` on a pattern with the top bit set
//! leaves all ones rather than `६५ − अंशसंख्या` of them. So the shift's
//! unsettled signedness was load-bearing: the same source gets two answers.
//!
//! # The repair, and why it cannot diverge again
//!
//! The signed half no longer shifts the datum at all. It builds the mask of
//! the bits at and above the sign — `१८४४६७४४०७३७०९५५१६१५ वियोगः
//! ((१ वामसृ (अंशसंख्या वियोगः १)) वियोगः १)` — and asks `मूल्यम् युक् mask
//! समम् mask`. `वामसृ`, `युक्` and `वियोगः` have no signedness to disagree
//! about on a 64-bit pattern, and the subtrahend is at most `२⁵⁵ − १`, so the
//! subtraction never borrows.
//!
//! The surviving `दक्षिणसृ` — the unsigned half — is safe for a different
//! reason, and [`the_bodys_only_right_shift_is_compared_against_zero`] is that
//! reason made checkable: its result is compared against `०`, and a shift that
//! is zero under a logical shift has its top bit clear, where arithmetic and
//! logical agree; a shift that is non-zero under one is non-zero under the
//! other. A comparison against `०` is the one shape this operator is settled
//! for.
//!
//! # Three instruments, because one engine cannot see this
//!
//! The divergence is INVISIBLE to the interpreter — it is the engine that is
//! right — so a test that only runs the `.t1` would pass on the old body too.
//! Hence:
//!
//! 1. [`the_fit_check_answers_rusts_question_on_every_width`] runs the real
//!    routine on the real interpreter over 7 widths × 12 patterns and demands
//!    Rust's `parse.rs` answer, REFUSALS INCLUDED.
//! 2. [`the_bodys_only_right_shift_is_compared_against_zero`] reads the body
//!    and holds the structural invariant that makes the engines agree.
//! 3. [`the_old_form_diverged_by_shift_signedness_and_the_new_one_cannot`]
//!    evaluates both bodies under BOTH shift semantics and names a pattern
//!    where the old one split. It is the only instrument that can fail for the
//!    reason W-367 was filed.
//!
//! # What the trace settled, and it bounds the row
//!
//! W-367 recorded a SECOND defect: handed `ऋण१२८` as a NEGATIVE the routine
//! refuses, though `−१२८` fits one octet signed. The caller was not traced
//! there. It is traced here: `vakyavibhaga.t1:3514` is the only call, it sits
//! in the `अन्यथा` of `व्याप्तिः समम् ०`, and that branch takes `मूल्यम्`
//! from `अक्षरकोशॱअंशाः` — the BIT PATTERN reader, never the magnitude one.
//! So the routine is only ever handed the pattern, and
//! [`the_pattern_reader_is_what_feeds_the_fit_check`] pins that. Given the
//! PATTERN of `−१२८` both the old and the new body accept it; the refusal
//! W-367 measured came from calling the routine with the negative directly,
//! which no source can do. The row's open severity question is answered, not
//! fixed, and the mask form is right for the negative too.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::{Path, PathBuf};

const FUEL: u64 = 20_000_000;

/// This test's own roots. A loader is part of the test.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn body() -> String {
    let src = std::fs::read_to_string(repo_root().join("crates/sadhana-t1/src/vakyavibhaga.t1"))
        .expect("vakyavibhaga.t1 is readable");
    let head = src
        .find("वृत्तिः अन्तर्भाववाचकः")
        .expect("the fit check is in vakyavibhaga.t1");
    let tail = src[head..]
        .find("\nइति")
        .expect("the fit check closes with इति at column zero");
    src[head..head + tail].to_string()
}

/// `fits` as `crates/sadhana/src/parse.rs` asks it, on the 64-bit pattern:
/// writable in `width` octets unsigned, or as two's complement signed.
fn rust_fits(v: u64, width: u32) -> bool {
    if width >= 8 {
        return true;
    }
    let n = width * 8;
    v >> n == 0 || ((v as i64) >> (n - 1)) == -1
}

/// Twelve patterns per width, chosen so every boundary of both halves has a
/// case on each side of it.
fn patterns(width: u32) -> Vec<u64> {
    let n = width * 8;
    let unsigned_max = (1u64 << n) - 1;
    let signed_max = (1u64 << (n - 1)) - 1;
    vec![
        0,
        1,
        200,
        300,
        signed_max,
        signed_max + 1,
        unsigned_max,
        unsigned_max + 1,
        u64::MAX,
        u64::MAX - signed_max,     // the pattern of −२^(n−१), the signed floor
        u64::MAX - signed_max - 1, // one below it: refused
        1u64 << 63,
    ]
}

/// Instrument 1 — the real routine on the real interpreter.
#[test]
fn the_fit_check_answers_rusts_question_on_every_width() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let mut asked = 0usize;
    let mut refused = 0usize;
    for width in 1..=7u32 {
        for v in patterns(width) {
            let got = it
                .call(
                    "वाक्यविभागॱअन्तर्भाववाचकः",
                    vec![Value::Int(i128::from(v)), Value::Int(i128::from(width))],
                    FUEL,
                )
                .expect("अन्तर्भाववाचकः runs");
            let want = rust_fits(v, width);
            assert_eq!(
                got,
                Value::Bool(want),
                "width {width}, pattern {v:#018x}: T1 said {got:?}, parse.rs says {want}"
            );
            asked += 1;
            if !want {
                refused += 1;
            }
        }
    }
    // Width ८ and up: a 64-bit datum holds every अ६४ by construction.
    for width in [8u32, 9, 64] {
        let got = it
            .call(
                "वाक्यविभागॱअन्तर्भाववाचकः",
                vec![
                    Value::Int(i128::from(u64::MAX)),
                    Value::Int(i128::from(width)),
                ],
                FUEL,
            )
            .expect("अन्तर्भाववाचकः runs");
        assert_eq!(got, Value::Bool(true), "width {width} holds every pattern");
        asked += 1;
    }
    assert_eq!(asked, 87, "7 widths × 12 patterns + 3 wide");
    assert!(refused > 0, "a fit check that refuses nothing is not one");
    println!("METRIC w367_fit_cases {asked}");
    println!("METRIC w367_fit_refusals {refused}");
}

/// Instrument 2 — the structural invariant that keeps the engines together.
#[test]
fn the_bodys_only_right_shift_is_compared_against_zero() {
    let body = body();
    let shifts: Vec<&str> = body.lines().filter(|l| l.contains("दक्षिणसृ")).collect();
    assert_eq!(
        shifts.len(),
        1,
        "the fit check should right-shift the datum exactly once; found:\n{}",
        shifts.join("\n")
    );
    assert!(
        shifts[0].contains("समम् ०"),
        "the one दक्षिणसृ must be compared against ० — that is the only shape \
         whose answer is the same under a logical and an arithmetic shift. Got:\n{}",
        shifts[0]
    );
    // And the signed half must be a mask test, not a shift.
    assert!(
        body.contains("युक्"),
        "the signed half tests the high-bit mask with युक्"
    );
}

/// The two candidate bodies, each evaluated under a right shift that is
/// logical and one that sign-extends. `shift` is `दक्षिणसृ`.
fn repaired(v: u64, width: u32, shift: fn(u64, u32) -> u64) -> bool {
    if width > 7 {
        return true;
    }
    let n = width * 8;
    if shift(v, n) == 0 {
        return true;
    }
    let low = (1u64 << (n - 1)) - 1;
    let high = u64::MAX - low;
    v & high == high
}

fn old(v: u64, width: u32, shift: fn(u64, u32) -> u64) -> bool {
    if width > 7 {
        return true;
    }
    let n = width * 8;
    if shift(v, n) == 0 {
        return true;
    }
    shift(v, n - 1) == (1u64 << (65 - n)) - 1
}

fn logical(v: u64, k: u32) -> u64 {
    v >> k
}

fn arithmetic(v: u64, k: u32) -> u64 {
    ((v as i64) >> k) as u64
}

/// Instrument 3 — the only one that can see the defect W-367 named.
#[test]
fn the_old_form_diverged_by_shift_signedness_and_the_new_one_cannot() {
    let mut old_splits: Vec<(u64, u32)> = Vec::new();
    let mut new_splits: Vec<(u64, u32)> = Vec::new();
    let mut checked = 0usize;
    for width in 1..=7u32 {
        for v in patterns(width) {
            if old(v, width, logical) != old(v, width, arithmetic) {
                old_splits.push((v, width));
            }
            if repaired(v, width, logical) != repaired(v, width, arithmetic) {
                new_splits.push((v, width));
            }
            assert_eq!(
                repaired(v, width, logical),
                rust_fits(v, width),
                "the repaired form must still answer parse.rs's question at \
                 width {width}, pattern {v:#018x}"
            );
            checked += 1;
        }
    }
    assert!(
        new_splits.is_empty(),
        "the repaired form must give one answer per pattern whatever \
         दक्षिणसृ's signedness; it split on {new_splits:?}"
    );
    assert!(
        old_splits.contains(&(u64::MAX, 1)),
        "W-367's own measurement — pattern २⁶⁴−१ at width १ — must be among \
         the old form's divergences, else this instrument is not measuring the \
         defect. Found: {old_splits:?}"
    );
    println!("METRIC w367_model_cases {checked}");
    println!("METRIC w367_old_divergences {}", old_splits.len());
}

/// The trace that bounds W-367's second defect: the only caller reads the
/// PATTERN, so the routine is never handed a negative.
#[test]
fn the_pattern_reader_is_what_feeds_the_fit_check() {
    let src = std::fs::read_to_string(repo_root().join("crates/sadhana-t1/src/vakyavibhaga.t1"))
        .expect("vakyavibhaga.t1 is readable");
    let calls: Vec<usize> = src
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("अन्तर्भाववाचकः") && !l.contains("वृत्तिः अन्तर्भाववाचकः"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(calls.len(), 1, "one call site, at lines {calls:?}");
    // Walk back from the call to the assignment of मूल्यम् that reaches it.
    let head = src.lines().take(calls[0]).collect::<Vec<_>>();
    let assign = head
        .iter()
        .rposition(|l| l.contains("मूल्यम् भवति"))
        .expect("मूल्यम् is assigned before the call");
    assert!(
        head[assign].contains("अक्षरकोशॱअंशाः"),
        "the value the fit check sees must come from the BIT PATTERN reader \
         अंशाः, not the magnitude reader मानम्. Got: {}",
        head[assign].trim()
    );
}
