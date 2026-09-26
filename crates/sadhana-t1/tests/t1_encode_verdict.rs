//! **THE OBJECT ENCODER'S VERDICT — `सङ्केतनसिद्धम्`, graded for the first time.**
//!
//! `82be3d5d` (2026-09-15, *"encode: carry the object encoder's verdict, so 1003
//! stops naming a success"*) replaced `पाठवस्तुरचना`'s refusal test. It had asked
//! the PAYLOAD — `यदि अष्टकाः समम् शून्यम्` — and a refusal answers `शून्यम्` while a
//! SUCCESS on a module with no instructions answers an EMPTY run. Natively those
//! are one word, so every data-only module was reported as "encoding refused" for
//! a compile that refused nothing. Eight lines of code; the fix is to carry a
//! verdict `उत्सर्जनक्रमः` sets only on the path that returns real text.
//!
//! **Nothing read it.** `सङ्केतनसिद्धम्` and `अन्तिमसङ्केतनसाफल्यम्` appear in
//! `encode.t1` and `shrinkhala.t1` and in NO test in the tree — checked by
//! content, 2026-09-17. That commit's own falsifier section says so in as many
//! words: *"NOT COVERED: that run is a twin check and cannot say `१००३` is gone.
//! Only rung 73 answering `२०००` natively can, and no native run is in this
//! commit."* This file is the cheap half of that gap; the native half is below.
//!
//! # Three outcomes, not two, because a payload test reports the middle one wrong
//!
//! ```text
//! a module with a routine      object built, text NON-EMPTY   exit ०   सत्यम्
//! a module of pure data        object built, text EMPTY       exit ०   सत्यम्
//! an immediate out of range    nil                            exit ३   असत्यम्
//! ```
//!
//! The middle row is the one the fix exists for and the one a length check cannot
//! see: its text measures `०` exactly as a refusal's does. A pass/fail instrument
//! would report it as the bad state.
//!
//! # What the interpreted side can and cannot say
//!
//! **It cannot re-find the original defect.** Interpreted, an empty run IS
//! distinguishable from nil, so the superseded `यदि अष्टकाः समम् शून्यम्` also
//! answered exit `०` on the data-only row — which is why rung 73 read `२०००`
//! interpreted and `१००३` natively for a week. An interpreted exit code is
//! therefore not evidence about the fix at all.
//!
//! What it CAN do is grade the verdict itself, which is what these tests read:
//! the flag, through the same routine the product calls, on all three outcomes
//! and across calls. A mutation that stops SETTING it reds every success; one
//! that stops CLEARING it reds the refusal that follows a success — which is why
//! the outcomes below run in sequence on ONE interpreter rather than on a fresh
//! one each.
//!
//! # The native half, measured 2026-09-18 and recorded rather than run here
//!
//! A native image is ~30 minutes and cannot be a landing gate. Taken on
//! `379d48e1` with `tools/t1-image.sh`, run under `yantra-run` with
//! `YANTRA_STEPS=200000000000`:
//!
//! ```text
//! rung 73 (dump off)   status 2000, 0 UART octets      — NOT १००३; the fix took
//! rung 79 (dump on)    status 2000, 232 UART octets    — the dump did not move it
//! ```
//!
//! # Falsified by mutation, and what stayed GREEN is half the reading
//!
//! Each restored and re-run; `git status` empty between rows.
//!
//! ```text
//! mutation of encode.t1 / shrinkhala.t1                     this file
//! the SET at encode.t1:5966 deleted                         3 RED of 3
//! the CLEAR at encode.t1:6076 deleted                       1 RED — the SEQUENCE
//!   (लक्ष्यवस्तुसङ्केतनम्, the entry point the object path uses)     test only
//! the CLEAR at encode.t1:5982 deleted                       3 GREEN
//!   (लक्ष्यसङ्केतनम्, the sibling entry point)
//! शृङ्खला:778 back to the superseded `यदि अष्टकाः समम् शून्यम्`   3 GREEN
//! ```
//!
//! Row two is why the three outcomes run in sequence on one interpreter: the two
//! tests that load a fresh chain per row both stayed green with that clear gone.
//!
//! Row three is honest uncovered ground. `लक्ष्यसङ्केतनम्`'s own margin says why —
//! *"No caller reads it through this path today, which is exactly when the trap
//! gets built"* — and nothing here reaches it, so its clear is unguarded and is
//! recorded as unguarded rather than assumed to be covered by proximity.
//!
//! **Row four is the limit of the interpreted side, measured rather than argued.**
//! Restoring the exact predicate `82be3d5d` replaced leaves all three green, with
//! `t1_verdict_dataonly` still reading exit `०` and `text: Some(0)`. Interpreted,
//! an empty run is NOT the nil word; the collision that produced `१००३` exists
//! only natively. So this file grades the verdict and the native reading above
//! grades the defect, and neither substitutes for the other.
//!
//! `२०००` is `२००० + ०`, the empty `.text` of a data-only module reported as a
//! SUCCESS. The 232 octets decode to `pass 1 [0]`, `pass 2 [0]`,
//! `विन्याससाम्यम् [1]` — **byte-identical to the interpreted sink**, so E22 does
//! not fire on either side and there is no layout pass to repair. That closes the
//! attribution `shrinkhala.t1`'s rung-79 margin carried until this commit.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

/// **THIS FILE'S OWN LOADER.** The subject is `शृङ्खलाॱपाठवस्तुरचना`, which reaches
/// the splitter, the layout, the symbol table and the emitter, so it needs the
/// whole chain. A borrowed loader is how a test starts grading a different
/// program from the one its margin describes.
fn load_chain() -> Interpreter {
    Interpreter::load(CHAIN, &spec_root()).expect("the chain loads")
}

/// What one build says about itself — every field read, none derived from another.
#[derive(Debug, PartialEq, Eq)]
struct Verdict {
    /// `वस्तुरचनाविरामभेद`: ० सिद्ध, १ वाक्य, २ रिक्त, ३ अष्टक, ४ नारब्ध.
    exit: i128,
    /// Whether `पाठवस्तुरचना` handed back an object at all.
    built: bool,
    /// `सङ्केतनसिद्धम्` — the carried verdict, through the routine the product calls.
    verdict: bool,
    /// The object's `.text` length, or `None` when nothing was built.
    text: Option<usize>,
}

/// Drive the real caller over Sassembly text, resetting first as the driver does.
///
/// **RESET FIRST.** `मण्डलानिप्रतिबिम्बम्` calls `सङ्कलनारम्भः` before every
/// `पाठवस्तुरचना`; calling the builder cold leaves `वाक्यविभाग` holding the previous
/// stage's state and it faults in a routine nobody edited.
fn drive(it: &mut Interpreter, text: &[u8]) -> Verdict {
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    let obj = it
        .call("शृङ्खलाॱपाठवस्तुरचना", vec![octets(text)], 40_000_000_000)
        .expect("पाठवस्तुरचना runs");
    let exit = match it.global("वस्तुरचनाविरामभेद") {
        Some(Value::Int(n)) => *n,
        other => panic!("`वस्तुरचनाविरामभेद` is an int, not {other:?}"),
    };
    // THE ROUTINE, NOT THE GLOBAL — the product reads it through
    // `सङ्केतनॱसङ्केतनसिद्धम्`, and a routine that stopped consulting its own global
    // would be invisible to a test that read the global directly. Both are taken
    // and compared below, so neither can drift without saying so.
    let verdict = match it.call("सङ्केतनॱसङ्केतनसिद्धम्", vec![], 1_000_000)
    {
        Ok(Value::Bool(b)) => b,
        other => panic!("`सङ्केतनसिद्धम्` answers a bool, not {other:?}"),
    };
    assert_eq!(
        it.global("अन्तिमसङ्केतनसाफल्यम्"),
        Some(&Value::Bool(verdict)),
        "`सङ्केतनसिद्धम्` and `अन्तिमसङ्केतनसाफल्यम्` disagree — then the reader has \
         come apart from the flag and neither is evidence about the other"
    );
    let text = match &obj {
        Value::Nil => None,
        Value::Record(r) => Some(
            r.borrow()
                .get("पाठ्यम्")
                .and_then(|v| v.octets().map(|o| o.as_slice().len()))
                .expect("a वस्तु carries `पाठ्यम्` as a run of octets"),
        ),
        other => panic!("`पाठवस्तुरचना` answers a वस्तु or शून्यम्, not {other:?}"),
    };
    Verdict {
        exit,
        built: !matches!(obj, Value::Nil),
        verdict,
        text,
    }
}

/// Compile `.t1` source to Sassembly text through the front half.
fn assemble_source(it: &mut Interpreter, src: &str, module: &str) -> Vec<u8> {
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets(module.as_bytes())],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(
        !text.is_empty(),
        "the front half emitted nothing for module `{module}` — the exemplar is \
         then wrong and nothing below is about the encoder"
    );
    text
}

/// Rung 73's exemplar: one global, no routine, so the object's `.text` is EMPTY.
///
/// **THE `इति` IS THE LITERAL'S TERMINATOR, NOT PART OF THE SOURCE** — the rung
/// writes `उक्तम् … । इति ।` and copying the tail in makes text that splits into
/// zero statements, which is exit १ and a different row of the table entirely.
const DATA_ONLY: &str = "मण्डलम् क ॥ सार्वजनिक चरः ख ॱॱ न६४ भवति ७ ।";

/// A module with a routine, so the object's `.text` carries real octets.
const WITH_CODE: &str = "मण्डलम् ग ॥\nसार्वजनिक वृत्तिः घ ददाति न६४ आदि\n    प्रत्यागमनम् ७ ।\nइति\n";

/// Four lines of Sassembly whose one `योगः` carries the immediate under test.
///
/// `addi`'s immediate is twelve bits signed, so `७` encodes and `५०००` cannot:
/// `स्थानसङ्केतनम्` answers `शून्यम्`, `उत्सर्जनक्रमः` sets `दोषवत्` and returns
/// `शून्यम्` WITHOUT setting the verdict, and the caller reports exit ३.
fn one_instruction(immediate: &str) -> String {
    format!(
        "॥ वैश्विकम् गघ ॥\nगघॱॱ\nयोगः स्थिर०म् शून्यःन {immediate}न ।\n\
         सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n"
    )
}

/// **THE ACCEPTANCE: three outcomes, in sequence, on one interpreter.**
///
/// The sequence is the point. `अन्तिमसङ्केतनसाफल्यम्` is MODULE STATE, cleared at
/// both of `encode.t1`'s entry points and set at exactly one place; a test that
/// loaded a fresh chain per row would pass with the clears deleted, because a
/// fresh module's flag is already `असत्यम्`. Running success → refusal → success
/// is what makes the clear observable.
#[test]
fn the_encoder_carries_its_verdict_and_an_empty_object_is_not_a_refusal() {
    let mut it = load_chain();

    // ── ROW ONE: a module with a routine. Text, and a verdict. ──────────────
    let code = assemble_source(&mut it, WITH_CODE, "ग");
    let with_code = drive(&mut it, &code);
    println!("METRIC t1_verdict_withcode {with_code:?}");
    assert_eq!(
        (with_code.exit, with_code.built, with_code.verdict),
        (0, true, true),
        "a module with a routine must build, exit ० and read सत्यम्"
    );
    assert!(
        with_code.text.unwrap_or(0) > 0,
        "the control's `.text` is empty — then the two success rows below are \
         the same row and this file separates nothing"
    );

    // ── ROW TWO: THE ROW `82be3d5d` EXISTS FOR. Built, EMPTY, and a success. ─
    let data = assemble_source(&mut it, DATA_ONLY, "क");
    let data_only = drive(&mut it, &data);
    println!("METRIC t1_verdict_dataonly {data_only:?}");
    assert_eq!(
        data_only.text,
        Some(0),
        "the data-only exemplar's `.text` must be EMPTY — that is the whole \
         difficulty: it measures ० exactly as a refusal's nil does, which is why \
         the verdict has to be carried rather than derived from the payload"
    );
    assert_eq!(
        (data_only.exit, data_only.built, data_only.verdict),
        (0, true, true),
        "a module of pure data ASSEMBLED — zero failing instructions of zero \
         total. Reporting it as exit ३ is `१००३`, the defect this file grades"
    );

    // ── ROW THREE: THE CASE THAT MUST STILL BE REFUSED. ────────────────────
    // Deliberately AFTER a success, so the flag it reads is one the previous
    // call set: this row fails if the clear at either entry point is removed.
    let refused = drive(&mut it, one_instruction("५०००").as_bytes());
    println!("METRIC t1_verdict_refused {refused:?}");
    assert_eq!(
        (refused.exit, refused.built, refused.verdict, refused.text),
        (3, false, false, None),
        "an immediate twelve bits cannot hold must still be REFUSED — and this \
         row runs after a success, so a सत्यम् here is the entry-point clear \
         missing, not the emitter"
    );

    // ── AND BACK: the flag is re-set, not merely left alone. ────────────────
    let again = drive(&mut it, one_instruction("७").as_bytes());
    println!("METRIC t1_verdict_after_refusal {again:?}");
    assert_eq!(
        (again.exit, again.built, again.verdict),
        (0, true, true),
        "a success AFTER a refusal must read सत्यम् — a flag that is only ever \
         cleared would leave this असत्यम् and report a sound compile as refused"
    );
}

/// **THE CONTROL: exit ३ is the IMMEDIATE, not the exemplar's shape.**
///
/// Three lines of Sassembly and a label could refuse for a dozen reasons. The
/// same four lines with an in-range immediate must build, or "out of range" is an
/// attribution and not a measurement.
#[test]
fn the_same_four_lines_build_when_the_immediate_fits() {
    let mut it = load_chain();
    let fits = drive(&mut it, one_instruction("७").as_bytes());
    println!("METRIC t1_verdict_immediate_fits {fits:?}");
    assert_eq!(
        (fits.exit, fits.built, fits.verdict),
        (0, true, true),
        "the in-range control must build; if it does not, the refusal below is \
         about this text's shape and says nothing about the immediate"
    );
    assert!(
        fits.text.unwrap_or(0) > 0,
        "the control emitted no octets — then nothing was encoded either way"
    );

    // Three widths past the boundary, all refused, none of them building.
    for immediate in ["५०००", "१०००००", "ऋण५०००"] {
        let mut it = load_chain();
        let out = drive(&mut it, one_instruction(immediate).as_bytes());
        println!("METRIC t1_verdict_immediate_{immediate} {out:?}");
        assert_eq!(
            (out.exit, out.built, out.verdict),
            (3, false, false),
            "`{immediate}` does not fit twelve signed bits and must be refused"
        );
    }
}

/// **WHAT THE EARLY EXITS LEAVE BEHIND — recorded, not asserted.**
///
/// `पाठवस्तुरचना` returns at exit १ (`वाक्यभेद`) and exit २ (`रिक्तभेद`) BEFORE the
/// encoder runs, so the verdict it leaves is the PREVIOUS call's. Measured
/// 2026-09-17: empty text immediately after a success reads exit १ with the
/// verdict still `सत्यम्`.
///
/// That is stale and it is not a defect, because the caller reads the verdict
/// only on the path where the encoder ran — it is below both early returns. It is
/// written down so the next reader who finds a `सत्यम्` beside a nil object knows
/// it was measured and why, rather than filing it. **If a future caller starts
/// reading the verdict above those returns, this is the trap**, and the assertion
/// here pins the fact rather than the design so that it cannot change silently.
#[test]
fn an_early_exit_leaves_the_previous_calls_verdict_and_that_is_known() {
    let mut it = load_chain();
    let ok = drive(&mut it, one_instruction("७").as_bytes());
    assert!(ok.verdict, "the success this row is measured against");

    let early = drive(&mut it, b"");
    println!("METRIC t1_verdict_early_exit {early:?}");
    assert_eq!(
        (early.exit, early.built, early.text),
        (1, false, None),
        "empty text takes exit १, `वस्तुरचनावाक्यभेद` — no statements"
    );
    assert!(
        early.verdict,
        "the early exit must still read the PREVIOUS call's सत्यम् — if this ever \
         reads असत्यम् the flag has gained a clear on a path that does not run the \
         encoder, which is an improvement and wants this margin rewritten rather \
         than the assertion flipped"
    );
}
