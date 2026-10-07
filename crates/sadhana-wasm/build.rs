//! Seal the assembler's source stamp INTO the module.
//!
//! `tools/src-rev.sh crates/sadhana/src` prints a 16-hex stamp over the assembler.
//! Taking it here, at compile time, is what makes it provenance rather than a
//! second copy of a number: a `sadhana_wasm.wasm` spliced into a page tomorrow
//! carries the stamp of the assembler it WAS BUILT FROM, so a page whose ELFs
//! were assembled by a different `sadhana` can be made to say so even while
//! every program it ships still re-assembles octet-for-octet.
//!
//! A machine with no sha256 tool, or a tree with no script, gives `unknown` and
//! NOT a stamp that happens to match — the page renders that as a third state.

use std::process::Command;

fn main() {
    // Cargo watches a directory argument recursively, so an edit to any module
    // of the assembler re-runs this and moves the sealed stamp with it.
    println!("cargo::rerun-if-changed=../sadhana/src");
    println!("cargo::rerun-if-changed=../../tools/src-rev.sh");

    // `sh <path>` and not the path alone: the executable bit is a property of a
    // checkout and an archive can lose it, whereas the interpreter cannot be lost.
    let rev = match Command::new("sh")
        .arg("../../tools/src-rev.sh")
        .arg("crates/sadhana/src")
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
    println!("cargo::rustc-env=SADHANA_SRC_REV={rev}");
}
