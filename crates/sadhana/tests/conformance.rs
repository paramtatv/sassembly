//! T0 conformance — task B-003.
//!
//! Every line of `spec/conformance-t0.tsv` carries Sassembly source and the
//! bytes `riscv64-elf-as` emits for the equivalent RISC-V. This runs our own
//! lexer, parser and encoder over the Sassembly and requires the same bytes.
//!
//! **The expectations are not ours.** GNU `as` knows nothing about this project,
//! so agreement is evidence rather than tautology — the same argument the C and
//! Zig baselines rest on. A suite that compared `sadhana` against `sadhana`
//! would pass no matter how wrong the encoder was.
//!
//! B-003 asked for this before the encoder and it arrived after. What that costs
//! is coverage, not correctness: the case list is mechanical — every register
//! group against every other, immediates at their boundaries — rather than
//! chosen by someone who has seen the implementation.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

struct Case {
    sassembly: String,
    expected: u32,
    riscv: String,
    line: usize,
}

fn cases() -> Vec<Case> {
    std::fs::read_to_string(root().join("spec/conformance-t0.tsv"))
        .expect("read spec/conformance-t0.tsv")
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            !l.starts_with('#') && !l.starts_with("sassembly\t") && !l.trim().is_empty()
        })
        .map(|(n, l)| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 3, "malformed case at line {}: {l:?}", n + 1);
            Case {
                sassembly: f[0].into(),
                expected: u32::from_str_radix(f[1].trim_start_matches("0x"), 16).expect("hex"),
                riscv: f[2].into(),
                line: n + 1,
            }
        })
        .collect()
}

#[test]
fn the_suite_is_large_enough_to_mean_something() {
    // B-003 asks for at least 800 assertions. A handful of cases can pass by
    // accident; a few thousand covering every register field cannot.
    let n = cases().len();
    assert!(n >= 800, "only {n} assertions; B-003 requires 800");
    println!("METRIC t0_conformance_assertions {n}");
}

#[test]
fn every_case_encodes_to_what_gnu_as_emits() {
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for c in cases() {
        let parsed = match sadhana::parse::assemble_source(&c.sassembly) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!(
                    "line {}: {} — did not parse: {}",
                    c.line,
                    c.sassembly,
                    e.join("; ")
                ));
                continue;
            }
        };
        assert_eq!(parsed.len(), 1, "line {}: expected one instruction", c.line);

        match sadhana::encode::encode(&parsed[0]) {
            Ok(got) if got == c.expected => checked += 1,
            Ok(got) => failures.push(format!(
                "line {}: {}\n    ours {got:#010x}, as {:#010x}  ({})",
                c.line, c.sassembly, c.expected, c.riscv
            )),
            Err(e) => failures.push(format!(
                "line {}: {} — did not encode: {e}  ({})",
                c.line, c.sassembly, c.riscv
            )),
        }
    }

    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(12).collect();
        panic!(
            "{} of {} conformance cases disagree with GNU as:\n{}\n{}",
            failures.len(),
            failures.len() + checked,
            shown
                .iter()
                .map(|s| format!("  {s}"))
                .collect::<Vec<_>>()
                .join("\n"),
            if failures.len() > 12 {
                format!("  … and {} more", failures.len() - 12)
            } else {
                String::new()
            }
        );
    }
    println!("METRIC t0_conformance_passing {checked}");
}

#[test]
fn the_suite_exercises_every_bit_of_the_register_fields() {
    // A suite that only ever used x1 and x2 would pass while the encoder placed
    // register numbers one bit off. This checks the corpus itself is wide
    // enough for the previous test to mean what it claims.
    let mut seen = 0u32;
    for c in cases() {
        for word in c.riscv.split(|ch: char| !ch.is_alphanumeric()) {
            if let Some(n) = word.strip_prefix('x').and_then(|d| d.parse::<u32>().ok()) {
                seen |= n;
            }
        }
        // ABI names too — the oracle column uses them.
        for (name, n) in [
            ("zero", 0u32),
            ("ra", 1),
            ("sp", 2),
            ("t0", 5),
            ("t3", 28),
            ("s11", 27),
            ("a7", 17),
        ] {
            if c.riscv.contains(name) {
                seen |= n;
            }
        }
    }
    assert_eq!(
        seen & 0b11111,
        0b11111,
        "the corpus never sets some register bit: {seen:#07b}"
    );
}
