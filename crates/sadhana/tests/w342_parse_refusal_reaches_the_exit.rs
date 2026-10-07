//! **W-342: A SOURCE THE PARSER REFUSED MUST REACH THE EXIT CODE, AND THE STOP
//! MUST SAY WHY.**
//!
//! `t1_image`'s CLI contract over one fixture: a routine holding ADR-0026's
//! fixed-capacity array, which the product's parser does not read (the row's
//! half (a), not fixed here). Measured on `393532c5` with the exit code read
//! directly, before the fix:
//!
//! - NO ENTRY: `build: 0 source(s) failed to compile, 1 declared nothing, 1
//!   object(s) linked (startup included)`, a 65,688-octet ELF of the startup
//!   alone, **exit 0**. This is the silent half and the reason for the row.
//! - WITH AN ENTRY: exit 1 and no file — already a refusal, and it has been
//!   since `84117497`. What it lacked was the cause: the same `0 failed, 1
//!   declared nothing` line, then `STOPPED AT link: the product returned an
//!   empty image` and a `{:?}` dump of the linker's arena in which the symbol's
//!   name is a list of octets. With `--no-predict` nothing named the cause.
//!
//! THE ROW AS FILED SAID THE ENTRY PATH EXITS ZERO. It does not, and the second
//! trigger recorded for it was a wrapper reading `grep`'s status. The entry
//! tests below are therefore PINS on the exit code — they passed before the
//! fix — and falsifiers only on the diagnostic. The no-entry test is the
//! falsifier on the exit code.
//!
//! THE FIX IS IN `shrinkhala.t1` (`सङ्कलनव्याकरणभेद`, ९) AND REACHES THIS BINARY
//! THROUGH W-343's `failed > 0`; `w342_parse_refusal_bheda.rs` in `sadhana-t1`
//! holds the `.t1` side. The Rust side here is the `refused:` line naming the
//! parser's site (`chain::parse_site`) and the link stop printing text.
//!
//! EVERY REFUSAL TEST HAS ITS CONTROL: the same source with the bound left out,
//! built the same way, must exit 0 and leave its image. A driver that refused
//! everything would pass the first three tests.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One directory per fixture: tests in one binary run in parallel threads.
fn scratch(slot: &str) -> PathBuf {
    let d = root().join("target/w342-parse-refusal").join(slot);
    std::fs::create_dir_all(&d).expect("the scratch directory is creatable");
    d
}

/// The driver's exit code and its whole output, stdout then stderr. Asserts
/// nothing about the status: the status is the subject.
fn run(
    dir: &Path,
    out: &str,
    extra: &[&str],
    loads: &[&str],
    sources: &[&str],
) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_t1_image"));
    cmd.arg("--spec-root")
        .arg(root().join("spec"))
        .arg("--compiler")
        .arg(root().join("crates/sadhana-t1/src"))
        .arg("-o")
        .arg(dir.join(out));
    for l in loads {
        cmd.arg("--load").arg(dir.join(l));
    }
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

/// The array type on line 3; the parser stops at the numeral.
const FIXED: &str = "मण्डलम् सीमितकोश ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः १०२४ अन्तः न६४ भवति ० ।
    क अङ्कः ३ अन्तः भवति ७ ।
    प्रत्यागमनम् क अङ्कः ३ अन्तः ।
इति
";

/// The control: one token apart.
const SLICE: &str = "मण्डलम् सीमितकोश ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ३ अन्तः भवति ७ ।
    प्रत्यागमनम् क अङ्कः ३ अन्तः ।
इति
";

fn fixture(slot: &str, file: &str, src: &str, elf: &str) -> PathBuf {
    let d = scratch(slot);
    std::fs::write(d.join(file), src).expect("the fixture is writable");
    let _ = std::fs::remove_file(d.join(elf));
    d
}

/// The cause, as every refusing build must now print it.
fn assert_names_the_parse_site(text: &str) {
    assert!(
        text.contains("1 source(s) failed to compile (सीमितकोश)"),
        "the refused source is COUNTED AS FAILED and NAMED\n{text}"
    );
    assert!(
        !text.contains("declared nothing"),
        "and it is not also filed as a source that declared nothing\n{text}"
    );
    let refused = text
        .lines()
        .find(|l| l.starts_with("refused:"))
        .unwrap_or_else(|| panic!("a `refused:` line names the site\n{text}"));
    assert!(
        refused.contains("सीमितकोश: parse: line 3,") && refused.contains("अन्तः अपेक्षितम्"),
        "the site is the PARSER's: its line and its reason, not a sentence about \
         the decide stage of a source that never parsed\n{text}"
    );
    assert!(
        !text.contains("LOAD-DEPENDENT") && !text.contains("RESOLVED AND TYPECHECKED"),
        "the third-state line is about a source that parsed; this one did not\n{text}"
    );
}

/// THE SILENT HALF. BEFORE: exit 0, `0 failed, 1 declared nothing`, a
/// startup-only ELF. NOW: the source is a named failure and W-343's
/// `failed > 0` carries it to the exit code.
///
/// THE FILE IS STILL WRITTEN, AND THAT IS W-343's CONTRACT, NOT AN OVERSIGHT
/// (`t1_image.rs`, "LAST, AND AFTER THE WRITE, DELIBERATELY"): instruments read
/// `steps:` and `refused:` off a failed build and `fixpoint.sh` tests for the
/// file. So this asserts the line that says what the file is, beside the exit.
#[test]
fn a_parse_refused_source_built_with_no_entry_exits_nonzero_and_is_named() {
    let d = fixture("no-entry", "सीमित.t1", FIXED, "no-entry.elf");
    let (code, text) = run(&d, "no-entry.elf", &[], &[], &["सीमित.t1"]);
    assert_ne!(
        code,
        Some(0),
        "a build whose only source was refused by the parser must not exit zero\n{text}"
    );
    assert_names_the_parse_site(&text);
    assert!(
        text.contains("STOPPED AT compile")
            && text.contains("it is not the program that was asked for"),
        "the stop is W-343's, and it says what the file at -o is\n{text}"
    );
}

/// THE ENTRY PATH, WITHOUT A PREDICTION — the case where NOTHING named the
/// cause before: no predict to refuse, `0 failed`, and a link stop.
/// The exit code and the absent file are PINS (true before the fix).
#[test]
fn a_parse_refused_entry_stops_at_link_with_no_file_and_the_cause_named() {
    let d = fixture("entry-no-predict", "सीमित.t1", FIXED, "entry.elf");
    let (code, text) = run(
        &d,
        "entry.elf",
        &[
            "--no-predict",
            // `W-381`: --no-predict requires a recorded reason. This build stops
            // at the link (an empty image) before the gate is reached, so the
            // reason is never written anywhere; it is passed so the command line
            // is one the gate would accept, and the stop stays the link's.
            "--accept-divergence",
            "the subject is the link stop of a parse-refused entry; no image reaches the gate",
            "--entry",
            "सीमितकोश",
            "मुख्यम्",
        ],
        &[],
        &["सीमित.t1"],
    );
    assert_ne!(code, Some(0), "PIN: the entry path refuses\n{text}");
    assert!(
        !d.join("entry.elf").exists(),
        "PIN: and it writes NO file — an empty image is not an image\n{text}"
    );
    assert!(
        text.contains("STOPPED AT link: the product returned an empty image"),
        "the stop is the link's\n{text}"
    );
    assert_names_the_parse_site(&text);
    assert!(
        text.contains("link cause: 1 source(s) failed to compile (सीमितकोश)"),
        "the link stop itself says the empty image is the failed source's \
         consequence, on the stream the stop is on\n{text}"
    );
    assert!(
        !text.contains("RefCell") && !text.contains("Octets {"),
        "the linker's refusals are printed as text, not as a `{{:?}}` of the arena\n{text}"
    );
}

/// The same with the prediction left on and the source in both lists. The
/// interpreter's own parser refuses the array type too, so the predict line
/// reads `refused` — and before the fix that line was the ONLY trace of the
/// cause in the whole output.
#[test]
fn a_parse_refused_entry_in_both_lists_refuses_the_same_way() {
    let d = fixture("entry-both", "सीमित.t1", FIXED, "entry-both.elf");
    let (code, text) = run(
        &d,
        "entry-both.elf",
        &["--entry", "सीमितकोश", "मुख्यम्"],
        &["सीमित.t1"],
        &["सीमित.t1"],
    );
    assert_ne!(code, Some(0), "PIN: the entry path refuses\n{text}");
    assert!(
        !d.join("entry-both.elf").exists(),
        "PIN: and writes no file\n{text}"
    );
    assert_names_the_parse_site(&text);
}

// ── THE BUILDS THAT MUST STILL SUCCEED ──────────────────────────────────────

/// The no-entry test's control: one token apart, exit 0, the image written,
/// nothing failed.
#[test]
fn the_same_source_without_the_bound_builds_with_no_entry() {
    let d = fixture("control-no-entry", "सीमित.t1", SLICE, "control.elf");
    let (code, text) = run(&d, "control.elf", &[], &[], &["सीमित.t1"]);
    assert_eq!(code, Some(0), "a source that parses builds\n{text}");
    assert!(
        d.join("control.elf").is_file(),
        "and leaves its image\n{text}"
    );
    assert!(
        text.contains("0 source(s) failed to compile") && !text.contains("refused:"),
        "with nothing failed and nothing refused\n{text}"
    );
}

/// The entry tests' control: predicted, built, and the prediction is the
/// routine's own answer.
#[test]
fn the_same_source_without_the_bound_builds_with_its_entry_and_predicts_seven() {
    let d = fixture("control-entry", "सीमित.t1", SLICE, "control-entry.elf");
    let (code, text) = run(
        &d,
        "control-entry.elf",
        &["--entry", "सीमितकोश", "मुख्यम्"],
        &["सीमित.t1"],
        &["सीमित.t1"],
    );
    assert_eq!(code, Some(0), "the control builds\n{text}");
    assert!(
        d.join("control-entry.elf").is_file(),
        "and leaves its image\n{text}"
    );
    assert!(
        text.contains("-> 7; the image's exit status must equal it"),
        "the prediction is the routine's answer, ७\n{text}"
    );
    assert!(
        !text.contains("STOPPED") && !text.contains("link refusal"),
        "and no stop of any kind is printed\n{text}"
    );
}
