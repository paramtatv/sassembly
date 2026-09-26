//! Golden T0 programs — task B-004.
//!
//! Each `spec/golden/*.sas` is a small Devanagari program written by hand, and
//! the matching `.words` holds what `riscv64-elf-as` emits for the equivalent
//! RISC-V. This assembles the Sassembly with our own lexer, parser and encoder
//! and requires the same bytes.
//!
//! HOW THIS DIFFERS FROM THE CONFORMANCE SUITE
//! -------------------------------------------
//! `B-003` enumerates single instructions mechanically — every register group
//! against every other — which is thorough about encodings and silent about
//! everything else. It cannot tell you whether the language can be WRITTEN,
//! because nothing in it was written.
//!
//! These were composed: 30 programs a person can read, each doing something
//! nameable. They cover what a generated corpus structurally cannot — sequences
//! where one instruction's destination is the next one's source, free operand
//! order as a program rather than a permutation, the width suffix choosing
//! among encodings one name covers, and virama-final register names in every
//! consonant role.
//!
//! The expectations are still not ours. GNU `as` knows nothing about this
//! project, so agreement is evidence rather than tautology.

use std::path::{Path, PathBuf};

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .join("spec/golden")
}

struct Program {
    name: String,
    source: String,
    expected: Vec<u32>,
}

fn programs() -> Vec<Program> {
    let mut out = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .expect("read spec/golden")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sas"))
        .collect();
    entries.sort();

    for path in entries {
        let source = std::fs::read_to_string(&path).expect("read .sas");
        let words_path = path.with_extension("words");
        let expected = std::fs::read_to_string(&words_path)
            .unwrap_or_else(|_| {
                panic!(
                    "{} has no .words — run tools/gen-golden.py",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            })
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| u32::from_str_radix(l.trim().trim_start_matches("0x"), 16).expect("hex word"))
            .collect();
        out.push(Program {
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            source,
            expected,
        });
    }
    out
}

#[test]
fn the_corpus_holds_the_thirty_programs_b004_asks_for() {
    let n = programs().len();
    assert!(n >= 30, "only {n} golden programs; B-004 requires 30");
    println!("METRIC golden_programs {n}");
}

#[test]
fn every_golden_program_assembles_to_what_gnu_as_emits() {
    let mut failures = Vec::new();
    let mut instructions = 0usize;

    for p in programs() {
        // `assemble_program`, not `assemble_source`: a program with labels is
        // more than its instructions, and `B-007` made the difference real.
        let program = match sadhana::parse::assemble_program(&p.source) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{}: did not parse: {}", p.name, e.join("; ")));
                continue;
            }
        };
        if program.instructions.len() != p.expected.len() {
            failures.push(format!(
                "{}: parsed {} instruction(s), oracle has {}",
                p.name,
                program.instructions.len(),
                p.expected.len()
            ));
            continue;
        }
        // Two passes, so a branch may name a label defined later in the file.
        match sadhana::encode::encode_program(&program).map(|t| sadhana::encode::words(&t)) {
            Ok(words) => {
                for (i, (got, want)) in words.iter().zip(&p.expected).enumerate() {
                    if got == want {
                        instructions += 1;
                    } else {
                        failures.push(format!(
                            "{} instruction {}: ours {got:#010x}, as {want:#010x}",
                            p.name,
                            i + 1
                        ));
                    }
                }
            }
            Err(es) => {
                for e in es {
                    failures.push(format!("{}: {e}", p.name));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} golden failure(s):\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
    println!("METRIC golden_instructions {instructions}");
}

#[test]
fn a_comment_only_line_contributes_no_instruction() {
    // Every program opens with a title comment and carries an `॰ as:` line
    // above each instruction, so the corpus would silently shrink to nothing
    // if comments were parsed as code. The counts above already depend on it;
    // this states it directly so the failure names the cause.
    let p = &programs()[0];
    let comment_lines = p.source.lines().filter(|l| l.starts_with('॰')).count();
    assert!(comment_lines >= 2, "the corpus should carry comments");
    assert_eq!(
        sadhana::parse::assemble_source(&p.source)
            .expect("parses")
            .len(),
        p.expected.len()
    );
}
