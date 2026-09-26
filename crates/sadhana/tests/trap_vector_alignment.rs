//! A trap handler must be four-octet aligned — task `B-058b2b7`.
//!
//! # The architecture, and what the assembler cannot see
//!
//! `stvec` holds the handler's address in its upper bits and **MODE in its low
//! two**: 0 Direct, 1 Vectored, anything above reserved. A handler that does not
//! begin on a four-octet boundary therefore does not merely sit oddly — its low
//! bits are read as a mode the machine does not implement, and traps stop
//! dispatching. The assembler cannot catch this: it writes whatever register the
//! program hands `csrrw`, and every part of that is individually legal.
//!
//! Under the wide encoding every instruction is four octets, so every label is
//! aligned by accident and the requirement is invisible. `--संक्षिप्त` shrinks
//! the code ahead of the handler and the accident stops holding:
//!
//! ```text
//! timer         plain 0x2c  & 3 = 0     compressed 0x26  & 3 = 2  RESERVED
//! milestone-k2  plain 0x364 & 3 = 0     compressed 0x25a & 3 = 2  RESERVED
//! ```
//!
//! Both images assembled clean, exited 0, and did not run. `spec/milestone-k2.sas`
//! states the requirement in its own comment — "हस्तक stvec में; निचले दो बिट ० =
//! सीधा रूप" — and nothing enforced it. **A comment is not an assertion.**
//!
//! # Why this is a test and not a `tools/check-*.sh`
//!
//! It was written as a `tools/check-*.sh` first and the gate measured it at
//! **365 seconds**: it spawned `cargo run` twice for each of fourteen programs,
//! which nearly doubled the gate on its own. `W-097` keeps expensive checks out
//! of a gate that has to run every cycle.
//!
//! As a test it rides the `cargo test` step already in the gate and calls the
//! encoder in-process, which removes 28 process spawns. **Measured at 220s, not
//! free.** An earlier draft of this comment said "milliseconds"; that was
//! written before it was timed and it was wrong by three orders of magnitude.
//! The cost is the relaxation fixpoint run twice per target on fourteen
//! programs in a debug build — `pager-handoff` alone is over 1200 lines.
//!
//! It is kept at that price because the class it catches produces images that
//! assemble clean, exit 0, and do not run. If the gate cannot afford 220s, the
//! honest options are to run it in release or to drop it from the gate and say
//! so — **not** to trim which files it looks at, which would make it report a
//! clean sweep over a partial one.

use std::collections::BTreeMap;

use sadhana::encode::{Target, encode_object_for, layout_addresses};
use sadhana::parse::{Section, assemble_program};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("root")
        .to_path_buf()
}

/// The handler label, read from the source rather than assumed to be `हस्तकः`.
///
/// Thirteen of the fourteen programs that write `stvec` call it `हस्तकः`.
/// **`spec/irq.sas` calls it `विघ्नग्राहकः`.** A check keyed on the common name
/// would pass that file while inspecting nothing — the very defect this file
/// exists to close, reproduced inside its own instrument. So the label is
/// derived: the `ॱअधःन` reference that feeds the `२६१त्` write is the handler,
/// whatever it is called.
fn handler_label(src: &str) -> Option<String> {
    let mut last: Option<String> = None;
    for line in src.lines() {
        if line.starts_with('॰') {
            continue;
        }
        if let Some(w) = line.split_whitespace().find(|w| w.contains("ॱअधःन")) {
            last = Some(w.split("ॱअधःन").next().unwrap_or_default().to_string());
        }
        if line.contains("२६१त्") {
            return last;
        }
    }
    None
}

#[test]
fn every_trap_handler_is_four_octet_aligned() {
    let spec = root().join("spec");
    let mut assessed = 0usize;
    let mut refused: Vec<String> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    let mut derived: BTreeMap<String, String> = BTreeMap::new();

    let mut files: Vec<_> = std::fs::read_dir(&spec)
        .expect("spec/")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "sas"))
        .collect();
    files.sort();

    for path in files {
        let src = std::fs::read_to_string(&path).expect("read");
        if !src.contains("२६१त्") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let Some(label) = handler_label(&src) else {
            bad.push(format!(
                "{name} writes stvec and the handler label cannot be identified — \
                 unidentifiable is not aligned"
            ));
            continue;
        };
        let Ok(program) = assemble_program(&src) else {
            bad.push(format!("{name} does not parse"));
            continue;
        };
        derived.insert(name.clone(), label.clone());

        for target in [Target::Uncompressed, Target::Compressed] {
            // A refusal is not a pass and not a failure of THIS property: the
            // assembler declined to build the image, so there is no handler
            // address to look at. Counted and named, never silent.
            if encode_object_for(&program, target).is_err() {
                refused.push(format!("{name} ({target:?})"));
                continue;
            }
            // Safe to read the address from `layout_addresses`: `B-058b2b7`'s
            // guard refuses any program whose layout and emit disagree, so for
            // a program that assembled at all, these are the emitted addresses.
            let at = layout_addresses(&program, target);
            let Some(l) = program.labels.iter().find(|l| l.name == label) else {
                bad.push(format!("{name} ({target:?}): `{label}` is not a label"));
                continue;
            };
            assert_eq!(l.section, Section::Text, "{name}: handler is not in text");
            let addr = at.get(l.at).copied().unwrap_or(u32::MAX);
            assessed += 1;
            // Addresses here are relative to the image base, and every link
            // address this project uses is itself four-octet aligned, so the low
            // two bits are the same relative or absolute. Asserting at a second
            // link address would be armour that cannot be wrong; what varies
            // here is the TARGET, and both are run.
            if addr % 4 != 0 {
                bad.push(format!(
                    "{name} ({target:?}): the handler `{label}` is at {addr:#x} — low \
                     bits {}, so stvec reads a reserved MODE and traps stop dispatching",
                    addr % 4
                ));
            }
        }
    }

    // Coverage is stated, so `ok` cannot mean less than it appears to.
    println!("  {assessed} handler address(es) assessed");
    for (n, l) in &derived {
        println!("    {n} -> {l}");
    }
    if !refused.is_empty() {
        println!(
            "  {} refused by the assembler, alignment not assessed: {}",
            refused.len(),
            refused.join(", ")
        );
    }

    assert!(bad.is_empty(), "\n  {}", bad.join("\n  "));
    assert!(
        assessed > 0,
        "no handler address could be read at all — this test asserted nothing"
    );
}
