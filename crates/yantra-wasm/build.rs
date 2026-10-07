//! Seal the interpreter's source stamp INTO the module.
//!
//! `tools/src-rev.sh crates/yantra/src` prints a 16-hex stamp over the interpreter.
//! Taking it here, at compile time, is what makes it provenance rather than a
//! second copy of a number: a `yantra_wasm.wasm` spliced into a page tomorrow
//! carries the stamp of the interpreter it WAS BUILT FROM.
//!
//! ॥ WHY THE INTERPRETER NEEDS THIS AND NOT ONLY THE ASSEMBLER ॥
//!
//! `crates/sadhana-wasm/build.rs` does the same for the assembler, and the argument
//! there is that three programs re-assembling octet-for-octet proves AGREEMENT and
//! not sameness. The interpreter's case is STRICTLY WORSE, because its figures are
//! not even a comparison: the page prints a halt status, a step count and the UART
//! text that THIS module produced, and prints a sizing line — `ram` asked for against
//! octets ever `touched` — that native `yantra-run` measured. Those are two different
//! binaries and nothing on the page said so. A stale `yantra-wasm` that happens to
//! agree on these three small programs would show a correct halt, a correct count and
//! correct text while the quoted sizing figures belong to an interpreter it is not.
//!
//! `crates/yantra/src` is a fixpoint input and nothing here touches it: the stamp is
//! taken over that directory from outside it.
//!
//! A machine with no sha256 tool, or a tree with no script, gives `unknown` and
//! NOT a stamp that happens to match — the page renders that as a third state.

use std::process::Command;

fn main() {
    // Cargo watches a directory argument recursively, so an edit to any module
    // of the interpreter re-runs this and moves the sealed stamp with it.
    println!("cargo::rerun-if-changed=../yantra/src");
    println!("cargo::rerun-if-changed=../../tools/src-rev.sh");

    // `sh <path>` and not the path alone: the executable bit is a property of a
    // checkout and an archive can lose it, whereas the interpreter cannot be lost.
    let rev = match Command::new("sh")
        .arg("../../tools/src-rev.sh")
        .arg("crates/yantra/src")
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
        _ => String::new(),
    };
    let rev = if rev.is_empty() {
        "unknown".to_owned()
    } else {
        rev
    };
    println!("cargo::rustc-env=YANTRA_SRC_REV={rev}");
}
