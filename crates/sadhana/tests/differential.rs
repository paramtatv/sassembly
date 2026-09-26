//! Program-level differential gate — task `B-012`, gate B8.
//!
//! `spec/conformance-t0.tsv` compares 3322 single instructions against GNU `as`.
//! This compares whole PROGRAMS, which is the only way to reach a displacement:
//! how far a branch jumps is a property of the program around it, so no
//! single-instruction corpus can test one.
//!
//! That mattered more than it sounds. Six branch families, a two-pass symbol
//! table and a derived displacement field landed across three cycles, and until
//! this file the mechanical coverage of all three was **zero** — one hand-
//! written golden program touched a branch at all. Every serious defect this
//! assembler has shipped was found by a corpus rather than by reasoning.

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
    expected: Vec<u32>,
    riscv: String,
    line: usize,
}

fn cases() -> Vec<Case> {
    std::fs::read_to_string(root().join("spec/differential-t0.tsv"))
        .expect("read spec/differential-t0.tsv")
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            !l.starts_with('#') && !l.starts_with("sassembly\t") && !l.trim().is_empty()
        })
        .map(|(n, l)| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 3, "malformed row at line {}", n + 1);
            Case {
                sassembly: f[0].replace("\\n", "\n"),
                expected: f[1]
                    .split_whitespace()
                    .map(|w| u32::from_str_radix(w.trim_start_matches("0x"), 16).expect("hex"))
                    .collect(),
                riscv: f[2].replace("\\n", " ; "),
                line: n + 1,
            }
        })
        .collect()
}

#[test]
fn the_corpus_reaches_every_branch_family_in_both_directions() {
    // A harness that only ever jumped forward would miss the sign bit
    // entirely, and the sign bit is the one `B-055`'s first probe could not
    // reach. This checks the corpus is wide enough for the gate below to mean
    // what it claims.
    let all = cases();
    assert!(all.len() >= 200, "only {} programs", all.len());
    for family in [
        "समलङ्घनम्",
        "विषमलङ्घनम्",
        "न्यूनलङ्घनम्",
        "अन्यूनलङ्घनम्",
        "अचिह्नन्यूनलङ्घनम्",
        "अचिह्नान्यूनलङ्घनम्",
        "लङ्घनम्",
    ] {
        assert!(
            all.iter().any(|c| c.sassembly.contains(family)),
            "no program uses {family}"
        );
    }
    // Forward means the label is the last line; backward means it is the first.
    let forward = all
        .iter()
        .filter(|c| c.sassembly.trim_end().ends_with("ॱॱ"))
        .count();
    let backward = all
        .iter()
        .filter(|c| c.sassembly.starts_with("लक्ष्यॱॱ"))
        .count();
    assert!(
        forward > 50 && backward > 50,
        "{forward} forward, {backward} backward"
    );
    println!("METRIC differential_programs {}", all.len());
}

#[test]
fn every_program_assembles_byte_identically_to_gnu_as() {
    let mut failures = Vec::new();
    let mut instructions = 0usize;

    for c in cases() {
        let program = match sadhana::parse::assemble_program(&c.sassembly) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("line {}: did not parse: {}", c.line, e.join("; ")));
                continue;
            }
        };
        match sadhana::encode::encode_program(&program).map(|t| sadhana::encode::words(&t)) {
            Ok(words) if words == c.expected => instructions += words.len(),
            Ok(words) => {
                let at = words
                    .iter()
                    .zip(&c.expected)
                    .position(|(a, b)| a != b)
                    .unwrap_or(0);
                failures.push(format!(
                    "line {}: instruction {} ours {:#010x}, as {:#010x}  ({})",
                    c.line,
                    at + 1,
                    words.get(at).copied().unwrap_or(0),
                    c.expected.get(at).copied().unwrap_or(0),
                    c.riscv
                ));
            }
            Err(es) => failures.push(format!(
                "line {}: did not encode: {}  ({})",
                c.line,
                es.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
                c.riscv
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} programs disagree with GNU as:\n  {}",
        failures.len(),
        cases().len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    println!("METRIC differential_instructions {instructions}");
}

#[test]
fn a_target_out_of_range_is_refused_rather_than_truncated() {
    // The corpus above only holds distances that fit, because a generator that
    // emitted an out-of-range branch would be measuring GNU `as`'s long-branch
    // REWRITE rather than the instruction asked for — the same trap `B-055`'s
    // probe fell into.
    //
    // So the boundary is checked directly. A branch reaches ±4 KiB and a jump
    // ±1 MiB; past that the displacement's high bits have nowhere to go, and
    // dropping them silently would produce a valid branch to the wrong place.
    use std::collections::BTreeMap;

    let program = sadhana::parse::assemble_program("समलङ्घनम् शून्यःन शून्यःत् लक्ष्यय् ।").expect("parses");
    let inst = &program.instructions[0];

    for (distance, fits) in [(4094i64, true), (4096, false), (65536, false)] {
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "लक्ष्य".to_string(),
            u32::try_from(distance).expect("positive"),
        );
        let got = sadhana::encode::encode_at(inst, 0, &symbols);
        assert_eq!(
            got.is_ok(),
            fits,
            "a branch {distance} bytes away: expected fits={fits}, got {got:?}"
        );
        if let Err(e) = got {
            // The DISTANCE, not the English word for it. `B-078` moved this
            // message into `spec/diagnostics.tsv` and the compiler now speaks
            // Sanskrit by default, so "away" is a fact about which language
            // won rather than about the diagnostic.
            assert!(
                e.reason.contains(&distance.to_string()),
                "the message should name the distance, got: {}",
                e.reason
            );
        }
    }
}

// ---------------------------------------------------------------------------
// `॥ संरेखः n ॥` against GNU `as` — task `W-079`, test 1.
//
// KILLS: every mutation of the padding itself. Gate B8 is byte-identical
// against gas, and until this test **neither oracle contained a directive of
// any kind** — `spec/differential-t0.tsv` and `spec/conformance-t0.tsv` are 0
// for `॥` and 0 for gas directives alike. So B8 was not comparing `संरेखः`
// badly; it was not comparing it at all, and the justification `W-071` wrote
// for emitting `nop` cited an oracle that never assembled an aligned program.
//
// This is not an improvement to B8. It is the first directive B8 ever checks.
//
// It compares the PADDING RUN, not the whole section, and that is deliberate.
// gas also pads the SECTION END to the boundary, where this assembler does not
// — `.text` 0x14 against gas's 0x20. Comparing whole sections would fail on
// that and bury the claim this test can settle: that the bytes BETWEEN the two
// instructions are the ones gas writes.
//
// `sh_addralign` NO LONGER diverges. It did when this test was drafted, and
// `W-080` (`d87e4bd`) found that hardcoding it to 4 was a CORRECTNESS defect
// rather than a cosmetic one: a 16-boundary inside an object that claims 4 is
// undone by the first link placing `.text` on a 4-byte multiple, so the padding
// proved nothing once linked. `kosha::write_relocatable` now raises it to the
// largest boundary `संरेखः` asked for.
// ---------------------------------------------------------------------------

/// `riscv64-elf-as`, if this machine has one. The corpus above exists precisely
/// so the suite does not need an assembler; this test does, because there is no
/// generated row to compare against yet.
fn gnu_as() -> Option<String> {
    for name in [
        "riscv64-elf-as",
        "riscv64-unknown-elf-as",
        "riscv64-linux-gnu-as",
    ] {
        if std::process::Command::new(name)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            return Some(name.to_string());
        }
    }
    None
}

#[test]
fn the_padding_संरेखः_emits_is_the_padding_gnu_as_emits() {
    let Some(as_bin) = gnu_as() else {
        // Loud rather than silent. A test whose oracle is absent and which
        // reports success is the exact failure `W-079` exists to correct, so
        // this says so in the output instead of passing quietly.
        println!(
            "METRIC align_differential_skipped 1  \
             (no riscv64-elf-as on PATH; this test did NOT run)"
        );
        return;
    };

    let dir = std::env::temp_dir().join("sansos-w079-align");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let src = dir.join("a.s");
    let obj = dir.join("a.o");
    // `.balign 16`, not `.align 16`: on RISC-V gas `.align n` means 2^n, so
    // `.align 16` would ask for a 65536-byte boundary. That mismatch is the
    // whole of `W-077` — `spec/directives.tsv` named `.align` for a directive
    // whose description is `.balign` semantics.
    std::fs::write(
        &src,
        ".section .text\nmain:\n    addi a0, zero, 10\n    .balign 16\ntarget:\n    addi a1, zero, 20\n",
    )
    .expect("write asm");

    let out = std::process::Command::new(&as_bin)
        .args(["-march=rv64gc", "-o"])
        .arg(&obj)
        .arg(&src)
        .output()
        .expect("run gnu as");
    assert!(
        out.status.success(),
        "{as_bin} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let theirs = std::process::Command::new(as_bin.replace("-as", "-objcopy"))
        .args(["-O", "binary"])
        .arg(&obj)
        .arg(dir.join("a.bin"))
        .output();
    let theirs = match theirs {
        Ok(o) if o.status.success() => std::fs::read(dir.join("a.bin")).expect("read bin"),
        _ => {
            println!("METRIC align_differential_skipped 1  (no objcopy; did NOT run)");
            return;
        }
    };

    let ours = sadhana::encode::encode_program(
        &sadhana::parse::assemble_program(
            "॥ कोष्ठकम् ॱपाठ ॥\nमुख्यम्ॱॱ\n    योगः  अर्थ०म्  शून्यःन  ०१०न ।\n\
             ॥ संरेखः १६ ॥\nलक्ष्यम्ॱॱ\n    योगः  अर्थ१म्  शून्यःन  ०२०न ।\n",
        )
        .expect("assembles"),
    )
    .expect("encodes");

    // The run between the first instruction and the boundary. gas's section is
    // longer because it pads the END too (`W-080`), so compare this span only.
    let (from, to) = (4usize, 16usize);
    assert!(
        theirs.len() >= to && ours.len() >= to,
        "both must reach the boundary: ours {} bytes, gas {} bytes",
        ours.len(),
        theirs.len()
    );
    assert_eq!(
        &ours[from..to],
        &theirs[from..to],
        "the padding run disagrees with GNU as\n  ours {:02x?}\n  gas  {:02x?}",
        &ours[from..to],
        &theirs[from..to]
    );
    println!("METRIC align_differential_bytes {}", to - from);
}
