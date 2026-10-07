//! **W-347: `yantra-run --version`, AND THE PROGRAM'S ARGV IS NEVER SWALLOWED.**
//!
//! `yantra-run` had no flags at all, by design: `:31`'s margin says "EVERYTHING
//! AFTER THE IMAGE IS THE PROGRAM'S … A flag added later would have to be rejected
//! before this line or it would silently stop reaching the program." So the flag is
//! recognised ONLY as the sole argument, and the second test here is the one that
//! matters — it pins that a program can still receive `--version` itself.
//!
//! **AND THE STAMP GOES TO STDERR HERE, UNLIKE `t1_image`.** In this binary stdout
//! carries the guest program's payload bytes and nothing else, enforced as a
//! source-level ratchet by `paradigm_boundary.rs:1620`. That ratchet caught a
//! `println!` on this very path during the gate, correctly: a byte on stdout that
//! is not the payload breaks the property that a program's output can be piped
//! uncontaminated, even on a path where no program runs. `t1_image` owns its stdout
//! and already reports `build:` and `write:` there, so it keeps stdout.

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

/// The stamp, on STDERR, and it must be the same commit `t1_image` names — one
/// shared build script, so a reader comparing a compile figure against a run figure
/// is comparing two binaries built from the same tree.
#[test]
fn yantra_run_names_its_build_commit_as_its_sole_argument() {
    let out = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .arg("--version")
        .output()
        .expect("yantra-run runs");
    assert!(
        out.status.success(),
        "`--version` must exit zero; it is a question, not a run"
    );
    // STDOUT MUST STAY EMPTY. This is the half that would silently regress if
    // someone later "fixed" the stream back: the payload channel carries nothing
    // when no program ran.
    assert!(
        String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "stdout belongs to the guest's payload — the stamp must not appear there"
    );
    // THE PREFIX IS PART OF THE CONTRACT, not decoration: `paradigm_boundary.rs`'s
    // census requires every stderr line of this binary to begin with a registered
    // diagnostic prefix, and `version: ` is enrolled there in the same commit that
    // added it.
    let printed = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stamp = printed
        .strip_prefix("version: ")
        .unwrap_or_else(|| {
            panic!("the stderr line must carry the registered `version: ` prefix, got {printed:?}")
        })
        .trim()
        .to_string();
    match git_head() {
        Some(head) => assert_eq!(stamp, head, "the stamp must be this tree's HEAD"),
        None => assert_eq!(stamp, "unknown", "with no git, the stamp must say so"),
    }
}

/// **THE NON-SWALLOWING PROPERTY, which is the whole reason the flag is gated on
/// `argc == 2`.** With a second argument present the version path must not fire —
/// otherwise `yantra-run prog.elf --version` would answer about the RUNNER while the
/// program it was asked to run never saw its own argument, and `W-343`'s family of
/// silent wrong answers would gain a member.
///
/// BOTH STREAMS are checked. Testing stdout alone would pass vacuously now that the
/// stamp is on stderr, which is exactly the shape of a test that stops measuring
/// what it was written for.
#[test]
fn a_second_argument_means_the_version_path_is_not_taken() {
    let out = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .args(["--version", "--version"])
        .output()
        .expect("yantra-run runs");
    let streams = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stamp = git_head().unwrap_or_else(|| "unknown".to_string());
    let answered = format!("version: {stamp}");
    // The version LINE must not appear on either stream. A substring test would be
    // wrong: the usage text names no commit, but a path echoed in an error could
    // coincidentally contain one.
    assert!(
        !streams.lines().any(|l| l.trim() == answered),
        "with two arguments this must NOT answer the version on any stream — the \
         first argument is the image path and everything after it belongs to the \
         program\n{streams}"
    );
}
