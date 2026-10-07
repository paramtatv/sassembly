//! **`t1_image`'s BUILD METER WAS REPORTING THE DIAGNOSTIC RE-COMPILE.**
//!
//! `SAS-013` earned the right to say WHY a source was refused by compiling the
//! first failed source a second time, alone, and reading `अर्थ`'s records out of
//! that run. That is the right design and its margin explains it. What came with
//! it was silent: `Interpreter::call` takes a fuel argument and ASSIGNS it —
//! `nirvahana.rs:1345`, `self.fuel = fuel` — so the re-compile RESTARTS the
//! meter, and the `steps:` line printed below it stopped being the build's.
//!
//! **MEASURED ON THIS TREE BEFORE THE CHANGE**, one trivial module plus one that
//! refuses at resolve, against the same two sources with the refusal removed:
//!
//! ```text
//!   ok.t1 + bad.t1  (1 refused)   steps:    35352
//!   ok.t1 + bad2.t1 (0 refused)   steps:    43464532
//!   ok.t1           (0 refused)   steps:    41605839
//! ```
//!
//! A build that did strictly less work reported 1,177 times more. And on the
//! self-image the failure path is the ONLY path — `lib.t1` is the zero-code-line
//! facade and always refuses — so rung 103's build printed `steps: 17436` where
//! the same build one commit earlier printed `27642890406`. The owner's point 3
//! meter, the one that says lex has 4.4x ashtaka's instructions, read six orders
//! of magnitude low and looked exactly like an answer.
//!
//! **WHY THE DRIVER AND NOT THE LIBRARY IS THE SUBJECT.** The defect is not in
//! `call`, which does what its signature says. It is an ORDER: a figure read
//! after a second `call`. Only the driver has that order, so only the driver's
//! own printed output can pin it, and this test runs the binary.
//!
//! `the_success_path_names_no_diagnostic_cost` IS THE CASE THAT MUST STILL BE
//! REFUSED. A `steps:` line that simply summed both calls, or a `diag:` line
//! printed unconditionally as `0`, would satisfy the first test and report a
//! figure nothing spent. The three states — built, built-and-diagnosed, and the
//! diagnostic alone — are what the single line could not tell apart.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// THIS TEST'S OWN SOURCES, written fresh under `target/` rather than kept in
/// `spec/`. They are three and four lines long and exist only to make one build
/// refuse and one not; a fixture in the corpus would be censused, counted and
/// eventually believed to mean something.
///
/// **ONE DIRECTORY PER BUILD, keyed on the output name.** The first version of
/// this used ONE fixed directory for all of them, and `cargo test` runs a
/// binary's tests in PARALLEL THREADS: three tests, four builds, each rewriting
/// the same `ok.t1`, `bad.t1` and `good2.t1`. `fs::write` truncates before it
/// writes, so there is a window where one test's `t1_image` reads a source
/// another test has just emptied — a flake, not a wrong answer, which is the
/// kind that gets re-run rather than fixed. `out` is already unique per build,
/// so keying on it costs nothing and removes the shared path entirely. Same
/// class as `SAS-017`, written the same day that row was closed.
fn scratch(slot: &str) -> PathBuf {
    let d = root().join("target/t1-image-steps-meter").join(slot);
    std::fs::create_dir_all(&d).expect("the scratch directory is creatable");
    let write = |name: &str, body: &str| {
        std::fs::write(d.join(name), body).unwrap_or_else(|e| panic!("writing {name}: {e}"));
    };
    write(
        "ok.t1",
        "मण्डलम् ठ ॥\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    प्रत्यागमनम् ७ ।\nइति\n",
    );
    // REFUSED AT RESOLVE, and the name is deliberately one nothing declares.
    write(
        "bad.t1",
        "मण्डलम् ड ॥\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    प्रत्यागमनम् नास्तिनाम ।\nइति\n",
    );
    // THE SAME SOURCE WITH THE REFUSAL REMOVED — the control for the pair, so
    // the two runs differ in the refusal and in nothing else.
    write(
        "good2.t1",
        "मण्डलम् ड ॥\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    प्रत्यागमनम् ८ ।\nइति\n",
    );
    d
}

/// What a build's exit status must be. Stated per call, because since `W-343`
/// the two kinds of build this file makes exit DIFFERENTLY and each is asserted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exit {
    /// Every source compiled: exit zero.
    Clean,
    /// A source was refused: exit NONZERO — and the meter lines are still there.
    Refused,
}

/// The driver's whole stdout for one build, with its exit status checked
/// against what the caller says this build is.
///
/// **THIS ASSERTED `status.success()` FOR EVERY BUILD, INCLUDING THE PAIR WITH A
/// REFUSED SOURCE — A PASSING TEST THAT ENCODED A DEFECT.** `t1_image` printed
/// `1 source(s) failed to compile` and exited 0, this helper required that 0,
/// and so the false green was pinned in place. `W-343` made a failed source
/// reach the exit code; the assertion is flipped for that pair rather than
/// dropped, so the status is now checked in BOTH directions.
///
/// What did NOT change is what this file is about: a refused build still
/// prints `steps:`, `diag:` and `refused:` before it exits, because the driver
/// returns its failure LAST. A build that stopped EARLY prints no `steps:` line
/// at all, and `None` from a parser is the least informative way to learn that
/// — so a status that is not the expected one panics with the whole output.
fn build(out: &str, sources: &[&str], expect: Exit) -> String {
    let d = scratch(out);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_t1_image"));
    cmd.arg("--spec-root")
        .arg(root().join("spec"))
        .arg("--compiler")
        .arg(root().join("crates/sadhana-t1/src"))
        .arg("-o")
        .arg(d.join(out));
    for s in sources {
        cmd.arg(d.join(s));
    }
    let r = cmd.output().expect("t1_image runs");
    let text = String::from_utf8_lossy(&r.stdout).into_owned();
    let got = if r.status.success() {
        Exit::Clean
    } else {
        Exit::Refused
    };
    assert_eq!(
        got,
        expect,
        "t1_image {sources:?} exited {:?}; a build with a refused source must exit \
         nonzero and a clean one zero (W-343)\n{text}\n{}",
        r.status.code(),
        String::from_utf8_lossy(&r.stderr)
    );
    text
}

/// The leading figure on a `name:    <digits> …` line, or `None` when the line
/// is absent — which is itself an assertion elsewhere in this file. The rest of
/// the line is prose on `diag:` and empty on `steps:`, so the FIRST token is
/// taken rather than the whole tail parsed.
fn figure(text: &str, name: &str) -> Option<u64> {
    text.lines().find_map(|l| {
        l.strip_prefix(name)?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
    })
}

#[test]
fn the_steps_line_reports_the_build_and_not_the_diagnostic_recompile() {
    let refused = build("refused.elf", &["ok.t1", "bad.t1"], Exit::Refused);
    let clean = build("clean.elf", &["ok.t1", "good2.t1"], Exit::Clean);

    assert!(
        refused.contains("1 source(s) failed to compile (ड)"),
        "the refused pair must actually refuse, or this test measures nothing\n{refused}"
    );
    assert!(
        clean.contains("0 source(s) failed to compile"),
        "the control pair must NOT refuse\n{clean}"
    );

    let a = figure(&refused, "steps:").expect("the refused build prints a `steps:` figure");
    let b = figure(&clean, "steps:").expect("the clean build prints a `steps:` figure");

    // THE BAND, NOT A PIN. The two builds do almost the same work — one source
    // stops at resolve instead of reaching emit — so their meters belong within
    // a factor of two of each other. The defect put them a factor of 1,177
    // apart, so any band this side of that catches it while leaving the figures
    // free to move with the compiler.
    assert!(
        a * 2 > b && b * 2 > a,
        "the refused build's meter is {a} and the clean build's is {b}; these are \
         the same work to within a stage, so one of them is not the build's \
         figure\n--- refused ---\n{refused}\n--- clean ---\n{clean}"
    );
}

#[test]
fn the_diagnostic_recompile_is_named_and_is_the_smaller_figure() {
    let refused = build("refused2.elf", &["ok.t1", "bad.t1"], Exit::Refused);
    let steps = figure(&refused, "steps:").expect("a `steps:` figure");
    let diag = figure(&refused, "diag:").expect(
        "a refused build re-compiles one source to name its site and must say what that cost",
    );
    assert!(
        diag > 0,
        "the re-compile ran — it is what printed the `refused:` line below — so a \
         zero here means the figure is not being read\n{refused}"
    );
    assert!(
        diag < steps,
        "re-compiling ONE source cost {diag} against {steps} for a build of TWO \
         plus the link; the two figures are the wrong way round\n{refused}"
    );
}

/// THE CASE THAT MUST STILL BE REFUSED.
#[test]
fn the_success_path_names_no_diagnostic_cost() {
    let clean = build("clean2.elf", &["ok.t1", "good2.t1"], Exit::Clean);
    assert!(
        clean.contains("0 source(s) failed to compile"),
        "this test needs a build that does NOT refuse\n{clean}"
    );
    assert!(
        figure(&clean, "steps:").is_some_and(|n| n > 0),
        "the success path still reports the build\n{clean}"
    );
    assert!(
        figure(&clean, "diag:").is_none(),
        "nothing was re-compiled, so there is no diagnostic cost to report; a \
         figure printed here would be the build's own steps wearing another \
         name\n{clean}"
    );
}
