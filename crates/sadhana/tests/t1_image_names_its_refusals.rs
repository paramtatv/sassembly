//! **W-343: A BUILD THAT CANNOT DO WHAT IT WAS ASKED MUST SAY SO AND EXIT NONZERO.**
//!
//! Three fixtures about `t1_image`'s CLI contract rather than about the compiler.
//! The row's load-bearing claim is that the ZERO EXIT is the defect: a build
//! script continues on it, so a silent failure becomes a missing or entry-less
//! artefact discovered much later.
//!
//! Written as the row's ASK, so they were the falsifier and not a description of
//! the day they were written: all three FAILED then, by design, and were
//! committed `#[ignore]`d with the measurement as the reason. The fix in
//! `t1_image.rs` un-ignored them, and each margin below keeps what the driver
//! used to do.
//!
//! **(a) IS NO LONGER THE ROW'S ASK, AND THE REASON IS A MEASUREMENT.** The row
//! asked for a nonzero exit on an entry module named only positionally, on the
//! premise that the image had no working entry. It has one — `status: Some(5)`
//! under `yantra-run` — so that build WORKS, and refusing it would have broken a
//! working caller for a clearer line. (a) therefore asserts a NOTICE: exit 0,
//! the image written, the cause and the remedy named, and the old misleading
//! "no routine named" line gone. (b) and (c) are unchanged: nonzero.
//!
//! **(b) AND (c) ASSERT ONLY A NONZERO EXIT, SO A DRIVER THAT REFUSED EVERYTHING
//! WOULD PASS BOTH.** The controls at the foot of this file are the builds that
//! must still SUCCEED — an entry named in both lists, an entry built without a
//! prediction on purpose, and an import the run does declare — and
//! `t1_image_steps_meter.rs` holds another, a clean pair with no entry.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One directory per fixture, keyed on the slot, for the reason
/// `t1_image_steps_meter.rs:49` gives: `cargo test` runs a binary's tests in
/// parallel threads and `fs::write` truncates before it writes, so a shared
/// scratch directory is a flake rather than a wrong answer.
fn scratch(slot: &str) -> PathBuf {
    let d = root().join("target/t1-image-refusals").join(slot);
    std::fs::create_dir_all(&d).expect("the scratch directory is creatable");
    d
}

/// The driver's exit code and its whole output. Unlike `build` in
/// `t1_image_steps_meter.rs`, this asserts NOTHING about the status — the exit
/// code is the subject here, so asserting it in the runner would hide every case.
fn run(dir: &Path, out: &str, extra: &[&str], sources: &[&str]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_t1_image"));
    cmd.arg("--spec-root")
        .arg(root().join("spec"))
        .arg("--compiler")
        .arg(root().join("crates/sadhana-t1/src"))
        .arg("-o")
        .arg(dir.join(out));
    for a in extra {
        cmd.arg(a);
    }
    for s in sources {
        cmd.arg(dir.join(s));
    }
    let r = cmd.output().expect("t1_image runs");
    let mut text = String::from_utf8_lossy(&r.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&r.stderr));
    (r.status.code(), text)
}

/// (a) AN ENTRY MODULE NAMED ONLY POSITIONALLY. `--entry` resolves through the
/// INTERPRETER, whose module set is the compiler directory plus `--load`; the
/// positional sources are compiled by the product and never loaded. The file's
/// own header says so. So this names a routine the interpreter does not have,
/// and the ask is that it be TOLD so rather than left to a refused predict that
/// only prints.
///
/// BEFORE THE FIX: a 65,752-octet ELF, `0 source(s) failed`, exit 0, and
/// `predict: interpreted पॱमुख्य refused: … no routine named … is loaded` as the
/// only trace — a line that looks like every other refused predict and names
/// neither the cause nor the remedy.
///
/// **THE IMAGE WAS NOT ENTRY-LESS, WHICH THIS FILE AND THE ROW BOTH SAID IT
/// WAS.** Measured while fixing: that ELF halts `status: Some(5)` under
/// `yantra-run` — `मुख्य`'s own answer. The entry IS installed; what the build
/// lacks is the interpreted half to pair that status against.
///
/// **SO THIS ASSERTS A NOTICE, NOT A REFUSAL.** It first asserted a nonzero
/// exit, and the driver first refused. That refused a build that works — and a
/// caller outside this tree builds exactly this shape under `set -e` and runs
/// the result. The build must go through, exit 0 and leave its image; the line
/// in the `predict:` place must name the module, say it was named positionally
/// and not with `--load`, say the entry is installed, and name both remedies;
/// and the old "no routine named" text must be GONE, because a notice printed
/// beside the line it replaces has replaced nothing.
#[test]
fn an_entry_module_named_only_positionally_builds_and_is_told_why_it_has_no_prediction() {
    let d = scratch("entry-positional");
    std::fs::write(
        d.join("प्रवेश.t1"),
        "मण्डलम् प ॥\nसार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि\n    प्रत्यागमनम् ५ ।\nइति\n",
    )
    .expect("the fixture is writable");
    let elf = d.join("entry-positional.elf");
    let _ = std::fs::remove_file(&elf);

    let (code, text) = run(
        &d,
        "entry-positional.elf",
        &["--entry", "प", "मुख्य"],
        &["प्रवेश.t1"],
    );

    // `W-381` (owner ruling 2026-10-06): A BUILD WITH NO COMPARISON IS NOT A
    // NOTICE ANY MORE. With no interpreted half there is nothing for the
    // differential gate to compare, so the build is REFUSED (exit 97, the image
    // at `.refused`) unless `--accept-divergence <reason>` records why it may
    // stand. The notice is still the `predict:` line — this test is about that.
    assert_eq!(
        code,
        Some(97),
        "a positional-only entry has no prediction; since W-381 that build is \
         refused without --accept-divergence\n{text}"
    );
    assert!(
        !elf.exists(),
        "a refused build leaves nothing at -o\n{text}"
    );
    let (code, accepted) = run(
        &d,
        "entry-positional.elf",
        &[
            "--entry",
            "प",
            "मुख्य",
            "--accept-divergence",
            "the fixture's module प cannot be --load-ed (it collides with sarani.t1's \
             parameter प, see the control below), so no prediction is possible",
        ],
        &["प्रवेश.t1"],
    );
    assert_eq!(
        code,
        Some(0),
        "accepted, the build goes through\n{accepted}"
    );
    assert!(
        elf.is_file(),
        "the accepted build must leave its image\n{accepted}"
    );
    let notice = text
        .lines()
        .find(|l| l.starts_with("predict:"))
        .unwrap_or_else(|| panic!("the notice stands in the `predict:` line's place\n{text}"));
    assert!(
        notice.contains("entry module प"),
        "the notice must NAME the entry module\n{text}"
    );
    assert!(
        notice.contains("positionally") && notice.contains("not with --load"),
        "the notice must say the CAUSE: named positionally and not with --load\n{text}"
    );
    assert!(
        notice.contains("no interpreted prediction") && notice.contains("entry is installed"),
        "the notice must say what is missing (the prediction) and what is NOT \
         (the image's entry)\n{text}"
    );
    assert!(
        notice.contains("--load as well") && notice.contains("--no-predict"),
        "the notice must name both remedies\n{text}"
    );
    // `W-381`: the GATE and the notice now say "refused" about this build (its image goes to
    // `.refused`), so the absence is asked of the `predict:` lines, which is
    // where the misleading line used to stand.
    assert!(
        !text.contains("no routine named")
            && !text
                .lines()
                .any(|l| l.starts_with("predict:") && l.contains("refused: ")),
        "the misleading refused-predict line must be absent, not printed beside \
         the notice that replaces it\n{text}"
    );
}

/// (b) A MODULE MISSING FROM THE INPUTS. Measured: the absent module's call is
/// silently STUBBED (`stubs: 1 ... by cause: {1: 1}`) and the build exits 0.
/// The row's claim that it reports a SYNTAX error is wrong — the diagnostic is
/// not misleading, it is nearly absent. Same family as
/// `a-t1-module-exists-only-if-a-rust-loader-lists-it`.
///
/// DO NOT fix this by keying on the stub count: stubs are a normal, tracked
/// metric here and the per-cause tally is a ratchet, so `stubs > 0` is true of
/// ordinary builds. The unresolved IMPORT is the signal.
///
/// BEFORE THE FIX: the absent module became a STUB (cause 1), the build exited
/// 0 and wrote a 65,728-octet ELF. No syntax error anywhere in the output.
#[test]
fn a_module_missing_from_the_inputs_is_named_as_missing_not_as_syntax() {
    let d = scratch("module-missing");
    std::fs::write(
        d.join("आह्वायक.t1"),
        "मण्डलम् फ ॥\nआयातः ब ।\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    \
         प्रत्यागमनम् बॱकृ ।\nइति\n",
    )
    .expect("the fixture is writable");

    let (code, text) = run(&d, "module-missing.elf", &[], &["आह्वायक.t1"]);

    assert_ne!(
        code,
        Some(0),
        "a build that cannot resolve a module must not exit zero\n{text}"
    );
    assert!(
        !text.to_lowercase().contains("syntax"),
        "an ABSENT module must not be reported as a SYNTAX error — that sends the \
         reader to the wrong file looking for a typo\n{text}"
    );
}

/// (c) THE CONTROL THAT BITES. A source that genuinely fails to compile is
/// already reported WELL — module, stage, symbol and line — and the exit code
/// still does not carry it.
///
/// `t1_image_steps_meter.rs` builds exactly this pair, and its runner asserted
/// `status.success()` on it — a PASSING test that encoded the defect. That
/// assertion was flipped in the commit that made this one pass, with its reason
/// stated there. The empty-image path is NOT a usable control: `t1_image.rs`
/// already exited FAILURE on `STOPPED AT link`, so it passes before and after
/// and discriminates nothing.
///
/// BEFORE THE FIX: a build reporting one failed source exited 0.
#[test]
fn a_source_that_fails_to_compile_carries_that_into_the_exit_code() {
    let d = scratch("source-failed");
    std::fs::write(
        d.join("सम्यक्.t1"),
        "मण्डलम् ठ ॥\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    प्रत्यागमनम् ७ ।\nइति\n",
    )
    .expect("the fixture is writable");
    std::fs::write(
        d.join("दुष्ट.t1"),
        "मण्डलम् ड ॥\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    प्रत्यागमनम् नास्तिनाम ।\nइति\n",
    )
    .expect("the fixture is writable");

    let (code, text) = run(&d, "source-failed.elf", &[], &["सम्यक्.t1", "दुष्ट.t1"]);

    assert!(
        text.contains("1 source(s) failed to compile"),
        "the fixture must actually refuse one source, or this pins nothing\n{text}"
    );
    assert_ne!(
        code,
        Some(0),
        "a build with a failed source must not exit zero\n{text}"
    );
}

// ── THE BUILDS THAT MUST STILL SUCCEED ──────────────────────────────────────
//
// (b) and (c) above assert a nonzero exit and nothing else about the status, so
// a driver that returned FAILURE from its first line would pass them both. These
// are the same shapes one fact apart, and each must exit ZERO and write its
// image.

/// (a)'s first remedy, as its notice names it: the SAME source in BOTH lists.
/// The interpreter then has the entry, predicts `5`, and the build goes through.
///
/// **THE MODULE IS `द्वारपाल` AND NOT (a)'s `प`, AND THE REASON IS A MEASUREMENT.**
/// `--load` puts the module into the interpreter THAT IS RUNNING THE COMPILER,
/// and there a module name competes with the compiler's own locals. `sarani.t1`'s
/// `योजनम्` takes a parameter named `प` and reads `प ॱ दैर्घ्य`; with a module `प`
/// loaded that is read as a call into the module, and the BUILD stops at
/// `STOPPED AT मण्डलानिप्रतिबिम्बम्`, the interpreter saying of `sarani.t1:30`
/// that module `प` has no routine `दैर्घ्य`. So (a)'s own fixture cannot be
/// remedied with `--load` at all, only with `--no-predict` or a longer name.
/// That hazard is the interpreter's and older than this file; it is recorded
/// here because it decides this name.
#[test]
fn an_entry_named_in_both_lists_predicts_and_builds() {
    let d = scratch("entry-both-lists");
    let src = d.join("प्रवेश.t1");
    std::fs::write(
        &src,
        "मण्डलम् द्वारपाल ॥\nसार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि\n    प्रत्यागमनम् ५ ।\nइति\n",
    )
    .expect("the fixture is writable");
    let elf = d.join("entry-both-lists.elf");
    let _ = std::fs::remove_file(&elf);

    let load = src.to_string_lossy().into_owned();
    let (code, text) = run(
        &d,
        "entry-both-lists.elf",
        &["--load", &load, "--entry", "द्वारपाल", "मुख्य"],
        &["प्रवेश.t1"],
    );

    assert_eq!(
        code,
        Some(0),
        "an entry named with --load AND positionally is the remedy the notice \
         itself names; if this fails the notice is giving advice that does not \
         work\n{text}"
    );
    assert!(
        text.contains("predict:  interpreted द्वारपालॱमुख्य -> 5"),
        "with the source loaded the interpreter must answer the entry's own 5\n{text}"
    );
    assert!(elf.is_file(), "a zero exit must leave the image\n{text}");
}

/// (a)'s second remedy: the one-sided build, ASKED FOR. The positional-only
/// entry compiles, links and is what the startup calls; `--no-predict` says the
/// missing interpreted half is intended, and the log must say `skipped` in the
/// `predict:` place — NOT the `none` notice, which is for a caller who did not
/// say, and not nothing at all.
#[test]
fn an_entry_built_without_a_prediction_on_purpose_succeeds_and_says_so() {
    let d = scratch("entry-no-predict");
    std::fs::write(
        d.join("प्रवेश.t1"),
        "मण्डलम् प ॥\nसार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि\n    प्रत्यागमनम् ५ ।\nइति\n",
    )
    .expect("the fixture is writable");
    let elf = d.join("entry-no-predict.elf");
    let _ = std::fs::remove_file(&elf);

    let (code, text) = run(
        &d,
        "entry-no-predict.elf",
        &[
            "--entry",
            "प",
            "मुख्य",
            "--no-predict",
            // `W-381`: a build with no comparison must say why it may stand.
            "--accept-divergence",
            "module प cannot be --load-ed beside the compiler (sarani.t1's parameter \
             प), so this fixture has no interpreted half by construction",
        ],
        &["प्रवेश.t1"],
    );

    assert_eq!(
        code,
        Some(0),
        "--no-predict asks for the native half alone; a caller that wants it \
         must be able to have it\n{text}"
    );
    assert!(
        text.contains("predict:  skipped (--no-predict)"),
        "a prediction that was not attempted must be SAID to be skipped, in the \
         `predict:` line's own place\n{text}"
    );
    assert!(
        !text.contains("predict:  none"),
        "with --no-predict the notice is REPLACED by the skipped line; a caller \
         who said so on purpose is not told again that they might\n{text}"
    );
    assert!(
        !text.contains("refused"),
        "nothing was attempted, so nothing may read as a refusal\n{text}"
    );
    assert!(elf.is_file(), "a zero exit must leave the image\n{text}");
}

/// `--no-predict`'s CORE PROPERTY: IT CHANGES THE LOG AND NOT THE IMAGE. The
/// same source, named in BOTH lists both times, built once with the prediction
/// and once with `--no-predict`. The predict runs in the interpreter that then
/// runs the compiler, so "the flag only skips a report" is a claim about that
/// interpreter's state and not a tautology — a predict that left something
/// behind for the build to read would show here as two different images.
///
/// Both lists in BOTH builds, so the two command lines differ in the flag and
/// in nothing else; and module `द्वारपाल` for the reason the control above gives.
#[test]
fn the_image_built_without_a_prediction_is_the_image_built_with_one() {
    let d = scratch("no-predict-identity");
    let src = d.join("प्रवेश.t1");
    std::fs::write(
        &src,
        "मण्डलम् द्वारपाल ॥\nसार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि\n    प्रत्यागमनम् ५ ।\nइति\n",
    )
    .expect("the fixture is writable");
    let with = d.join("predicted.elf");
    let without = d.join("unpredicted.elf");
    let _ = std::fs::remove_file(&with);
    let _ = std::fs::remove_file(&without);
    let load = src.to_string_lossy().into_owned();

    let (code, text) = run(
        &d,
        "predicted.elf",
        &["--load", &load, "--entry", "द्वारपाल", "मुख्य"],
        &["प्रवेश.t1"],
    );
    assert_eq!(code, Some(0), "the predicted build must succeed\n{text}");
    assert!(
        text.contains("predict:  interpreted द्वारपालॱमुख्य -> 5"),
        "this half must actually PREDICT, or the pair compares one build with \
         itself\n{text}"
    );

    let (code, text) = run(
        &d,
        "unpredicted.elf",
        &[
            "--load",
            &load,
            "--entry",
            "द्वारपाल",
            "मुख्य",
            "--no-predict",
            // `W-381`: required beside --no-predict, recorded UNCHECKED.
            "--accept-divergence",
            "this build exists to be compared octet for octet with the predicted \
             one above, which the gate DID check",
        ],
        &["प्रवेश.t1"],
    );
    assert_eq!(code, Some(0), "the unpredicted build must succeed\n{text}");
    assert!(
        text.contains("predict:  skipped (--no-predict)"),
        "this half must actually SKIP the predict\n{text}"
    );

    let a = std::fs::read(&with).expect("the predicted image was written");
    let b = std::fs::read(&without).expect("the unpredicted image was written");
    assert!(!a.is_empty(), "an empty image equals another empty image");
    let first = a.iter().zip(&b).position(|(x, y)| x != y);
    assert!(
        a == b,
        "--no-predict changed the IMAGE: {} octets with the prediction, {} \
         without, first difference at offset {:?}. The flag is meant to skip a \
         report; if it moves the build, the predict leaves state behind that the \
         compiler reads.",
        a.len(),
        b.len(),
        first
    );
}

/// (b)'s control, and the shape every kernel outside this tree has: an import
/// of a module that IS an input (`ब`), and one of a module only `--compiler`'s
/// directory declares (`अष्टक`, the compiler's own output channel, which a
/// program built alone imports without ever listing `ashtaka.t1`). Neither is
/// an absent module. A check that compared imports against the positionals
/// alone would refuse the second, and with it every such kernel.
#[test]
fn an_import_the_run_does_declare_is_not_refused() {
    let d = scratch("module-present");
    std::fs::write(
        d.join("आह्वायक.t1"),
        "मण्डलम् फ ॥\nआयातः ब ।\nआयातः अष्टक ।\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    \
         प्रत्यागमनम् बॱकृ ।\nइति\n",
    )
    .expect("the fixture is writable");
    std::fs::write(
        d.join("आहूत.t1"),
        "मण्डलम् ब ॥\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    प्रत्यागमनम् ७ ।\nइति\n",
    )
    .expect("the fixture is writable");
    let elf = d.join("module-present.elf");
    let _ = std::fs::remove_file(&elf);

    let (code, text) = run(&d, "module-present.elf", &[], &["आह्वायक.t1", "आहूत.t1"]);

    assert_eq!(
        code,
        Some(0),
        "both imports name a module this run holds a source for — one positional, \
         one in --compiler's directory — so neither is missing\n{text}"
    );
    assert!(
        text.contains("0 source(s) failed to compile"),
        "the control must be a build that compiles, or its zero says nothing\n{text}"
    );
    assert!(elf.is_file(), "a zero exit must leave the image\n{text}");
}
