// ॥ THE BUILD COMMIT STAMP — W-347 ॥  Shared verbatim by `crates/sadhana/build.rs`
// and `crates/yantra/build.rs` through `include!`, so there is ONE statement of it
// rather than two copies that drift.
//
// ॥ THIS SCRIPT IS INCAPABLE OF FAILING A BUILD, AND THAT IS ITS FIRST PROPERTY ॥
//
// The row it closes was deferred for exactly one reason: a build script that dies
// where `git` is absent or the checkout is not a repository breaks EVERY build for
// EVERY session, and a remote host's checkout is a plain copy with no `.git` at all.
// So every path here ends in a value: `git` missing, `git` failing, a detached or
// broken checkout, output that is not UTF-8, an empty answer — all of them stamp
// `unknown` and exit 0. There is no `panic!`, no `unwrap` on a fallible call and no
// `Err` return in this file, deliberately.
//
// WHY A STAMP AT ALL. Two `t1_image` binaries differing by 24.7% on the same input
// cost an afternoon of timeline reconstruction and three successive wrong
// attributions, because neither binary could say what it was built from. The naad
// page quotes native counts; a figure needs a commit beside it.
//
// REPRODUCIBILITY IS UNAFFECTED, measured before this existed:
// `tools/check-reproducible.sh:111` builds `--release --workspace` TWICE into two
// `CARGO_TARGET_DIR`s AT THE SAME COMMIT and compares shasums. The stamp is
// identical in both halves, so the binaries stay identical.
//
// KNOWN, DELIBERATE, AND RECORDED ON THE ROW: the stamp names HEAD, NOT THE TREE. A
// binary built from a dirty worktree prints a commit it is not — and because the
// rerun lines below name only HEAD and the ref, an uncommitted edit does not even
// rerun this script, so the stamp can name a commit whose content was not what was
// built. A `-dirty` suffix would go stale by the same mechanism and would make the
// stamp depend on files appearing between the two reproducibility builds. So the
// caveat is printed in `--version`'s usage text in words instead.

use std::process::Command;

/// `git` output for one invocation, or `None` for every kind of failure.
fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// The paths whose change should rerun this script.
///
/// **`git` RESOLVES THIS, NOT HAND-PARSING, AND THE REASON IS THIS VERY TREE.** In a
/// worktree `.git` is a FILE holding `gitdir: …`, so the naive `.git/HEAD` is wrong
/// here on the first try; and a `commondir` split or a nested worktree breaks the
/// hand-written version again. `git rev-parse --git-path` is worktree-aware by
/// construction, so it is asked instead of reimplemented.
///
/// A symbolic `HEAD` (`ref: refs/heads/x`) does not change when a commit lands on
/// the branch — the REF FILE does. Both are named, or a commit on the current branch
/// would leave the stamp stale, which is this row's own defect.
///
/// With no `git` this answers empty and the caller emits NO rerun line at all —
/// cargo then falls back to rerunning on package changes, which is the safe
/// default: the failure to avoid is a script that never reruns, not one that reruns
/// too often.
fn rerun_paths() -> Vec<String> {
    let mut v = Vec::new();
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        v.push(head);
        // One `if`, not two: `-D clippy::collapsible-if` is on, and a let-chain is
        // the collapsed form clippy asks for.
        if let Some(r) = git(&["symbolic-ref", "-q", "HEAD"])
            && let Some(p) = git(&["rev-parse", "--git-path", &r])
        {
            v.push(p);
        }
    }
    v
}

/// The tree's root, from this crate's manifest directory (`crates/<name>`).
fn root() -> Option<std::path::PathBuf> {
    let m = std::env::var_os("CARGO_MANIFEST_DIR")?;
    Some(std::path::Path::new(&m).join("../.."))
}

/// THE SOURCE STAMP (`W-381`): `tools/src-rev.sh` over `crates/sadhana/src` and
/// over `crates/yantra/src`, joined — the SAME pair in both crates, so `t1_image`
/// and `yantra-run` carry equal stamps exactly when both were built from the same
/// assembler AND the same emulator. A content hash, so an archive build with no
/// git still gets a real one. Every failure (no `sh`, no script, no hasher) is
/// `unknown`, which the gate never matches — and, as above, never fails a build.
fn source_stamp() -> String {
    let Some(root) = root() else {
        return "unknown".to_string();
    };
    let one = |dir: &str| -> Option<String> {
        let out = Command::new("sh")
            .arg(root.join("tools/src-rev.sh"))
            .arg(dir)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
        if s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
            Some(s)
        } else {
            None
        }
    };
    match (one("crates/sadhana/src"), one("crates/yantra/src")) {
        (Some(a), Some(b)) => format!("{a}{b}"),
        _ => "unknown".to_string(),
    }
}

fn main() {
    let stamp = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SASSEMBLY_BUILD_COMMIT={stamp}");
    println!("cargo:rustc-env=SASSEMBLY_SOURCE_STAMP={}", source_stamp());
    for p in rerun_paths() {
        println!("cargo:rerun-if-changed={p}");
    }
    // The source stamp is stale the moment either crate's source changes, so
    // both directories (scanned recursively by cargo) and the script rerun it.
    if let Some(root) = root() {
        for p in ["crates/sadhana/src", "crates/yantra/src", "tools/src-rev.sh"] {
            println!("cargo:rerun-if-changed={}", root.join(p).display());
        }
    }
}
