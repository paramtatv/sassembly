//! Round-trip fixpoint — task `B-013`.
//!
//! Every word in every corpus is taken apart by `विश्लेषणम्` and put back
//! together, and must come out identical.
//!
//! # Why this is a different question from the differential gate
//!
//! The differential corpora ask *does our encoder produce the same bytes as GNU
//! `as`*. That is a check on one direction, and the encoder only ever uses the
//! table in one direction: it starts from a pattern whose operand bits are zero
//! and ORs values in. It never asks whether a mask is correct — only whether
//! the bits it is setting are free.
//!
//! Which is exactly how four mask defects shipped. `slli` carried the probe's
//! own registers in its opcode; 174 masks claimed operand bits as fixed
//! encoding; `beq` and `jal` swallowed their own displacements; every signed
//! field was missing its sign bit. All four encoded correctly the whole time.
//!
//! `(word & mask) == pattern` is a claim about every bit at once, so decoding
//! is the question the encoder cannot ask itself. The words used are the ones
//! `riscv64-elf-as` emitted, so nothing here is checking our beliefs against
//! our beliefs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sadhana::vishlesana::{candidates, decode, reassemble};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// Every distinct word the corpora contain, with an example of its source.
fn corpus_words() -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();

    let conformance =
        std::fs::read_to_string(root().join("spec/conformance-t0.tsv")).expect("read conformance");
    for l in conformance.lines() {
        if l.starts_with('#') || l.starts_with("sassembly\t") || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() >= 3
            && let Ok(w) = u32::from_str_radix(f[1].trim_start_matches("0x"), 16)
        {
            out.entry(w).or_insert_with(|| f[2].to_string());
        }
    }

    let differential = std::fs::read_to_string(root().join("spec/differential-t0.tsv"))
        .expect("read differential");
    for l in differential.lines() {
        if l.starts_with('#') || l.starts_with("sassembly\t") || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() >= 3 {
            for w in f[1].split_whitespace() {
                if let Ok(w) = u32::from_str_radix(w.trim_start_matches("0x"), 16) {
                    out.entry(w).or_insert_with(|| f[2].replace("\\n", " ; "));
                }
            }
        }
    }
    out
}

#[test]
fn the_corpus_is_wide_enough_to_mean_something() {
    let n = corpus_words().len();
    assert!(n >= 2000, "only {n} distinct words");
    println!("METRIC roundtrip_distinct_words {n}");
}

#[test]
fn every_word_survives_being_taken_apart_and_put_back() {
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for (word, source) in corpus_words() {
        let Some(d) = decode(word) else {
            failures.push(format!("{word:#010x} did not decode  ({source})"));
            continue;
        };
        match reassemble(&d) {
            Some(back) if back == word => checked += 1,
            Some(back) => failures.push(format!(
                "{word:#010x} came back as {back:#010x} via {}  ({source})",
                d.insn
            )),
            None => failures.push(format!(
                "{word:#010x} decoded to {} and would not reassemble  ({source})",
                d.insn
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} words failed the round trip:\n  {}",
        failures.len(),
        checked + failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    println!("METRIC roundtrip_words {checked}");
}

#[test]
fn no_word_matches_two_encodings() {
    // Two rows constraining the same bits to the same values are the same
    // instruction, so a second match is a fault in the table rather than an
    // ambiguity in the ISA. `ecall` is the one legitimate near-miss: it fixes
    // every bit and shares an opcode with the CSR instructions, which is why
    // `decode` prefers the most constrained match.
    let mut ambiguous = Vec::new();
    for (word, source) in corpus_words() {
        let c = candidates(word);
        if c.len() > 1 {
            let names: Vec<&str> = c.iter().map(|e| e.insn.as_str()).collect();
            ambiguous.push(format!(
                "{word:#010x} matches {}  ({source})",
                names.join(", ")
            ));
        }
    }
    assert!(
        ambiguous.is_empty(),
        "{} word(s) match more than one encoding:\n  {}",
        ambiguous.len(),
        ambiguous
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn the_decoder_reads_the_program_that_runs_on_the_metal() {
    // spec/bare-metal.sas is the program QEMU executes. Decoding what our own
    // assembler produced for it is the narrowest possible statement that the
    // two halves of the toolchain agree.
    let source = std::fs::read_to_string(root().join("spec/bare-metal.sas")).expect("read");
    let program = sadhana::parse::assemble_program(&source).expect("parses");
    let words =
        sadhana::encode::words(&sadhana::encode::encode_program(&program).expect("encodes"));

    let names: Vec<String> = words
        .iter()
        .map(|w| decode(*w).expect("every word decodes").insn)
        .collect();
    assert!(names.contains(&"blt".to_string()), "the loop branch");
    assert!(names.contains(&"bne".to_string()), "the equality check");
    assert!(names.contains(&"sw".to_string()), "the finisher write");
    assert!(names.contains(&"lui".to_string()), "the address constant");
    for (w, d) in words.iter().zip(&names) {
        let back = reassemble(&decode(*w).expect("decodes")).expect("reassembles");
        assert_eq!(back, *w, "{d} did not survive the round trip");
    }
}
