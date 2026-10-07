//! **W-347: THE BINARIES NAME THE COMMIT THEY WERE BUILT FROM.**
//!
//! The row's cost was concrete: two `t1_image` binaries differing by 24.7% on the
//! same input, and three successive wrong attributions of that difference, because
//! neither binary could say what it was built from. A figure needs a commit beside
//! it, and `--version` is how a reader gets one in a line instead of an afternoon.
//!
//! WHAT IS ASSERTED HERE AND WHAT IS NOT. The stamp agreeing with `git rev-parse
//! HEAD` is testable and tested. The other two falsifiers are build-ENVIRONMENT
//! properties — a tree with no `.git` must stamp `unknown`, and
//! `check-reproducible.sh` must stay green — and neither can be reached from
//! inside a test that runs in this repository. They are recorded on the row with
//! the reasoning instead of asserted by a test that could not fail.

use std::process::Command;

fn git_head() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// `t1_image --version` answers before anything is read, loaded or built, and the
/// answer is this tree's HEAD — or `unknown`, which is a real answer rather than a
/// failure and is what a build with no git to ask must say.
#[test]
fn t1_image_names_its_build_commit() {
    let out = Command::new(env!("CARGO_BIN_EXE_t1_image"))
        .arg("--version")
        .output()
        .expect("t1_image runs");
    assert!(
        out.status.success(),
        "`--version` must exit zero; it is a question, not a build\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(
        !printed.is_empty(),
        "`--version` must print something — a blank line is the defect this row is about"
    );

    match git_head() {
        Some(head) => assert_eq!(
            printed, head,
            "the stamp must be the commit the build was taken at, so a figure can \
             name its provenance in one line"
        ),
        None => assert_eq!(
            printed, "unknown",
            "with no git to ask, the stamp must SAY SO rather than invent or blank — \
             an instrument whose absence is indistinguishable from a reading is the \
             same defect by another route"
        ),
    }
}

/// AND IT ANSWERS WITHOUT A BUILD. `--version` is matched in the argument loop
/// before `--spec-root`, `--compiler` or any source is touched, so it works in a
/// tree where a build could not even start. The proof is that no `-o` was given:
/// every other path through this binary refuses without one.
#[test]
fn the_version_answer_needs_no_output_path_and_no_sources() {
    let out = Command::new(env!("CARGO_BIN_EXE_t1_image"))
        .arg("--version")
        .output()
        .expect("t1_image runs");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && !text.trim().is_empty(),
        "`--version` alone, with no -o and no sources, must still answer\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("usage:"),
        "`--version` must not fall through to the usage message\n{text}"
    );
}
