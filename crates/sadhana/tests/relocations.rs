//! The relocation table says what the assembler does — task `B-069a`.
//!
//! `बन्धकः` linked by re-encoding every unit, which satisfied gate B10 and did
//! not let a file be assembled once and linked many times. It is gone
//! (`B-096b`); every build takes this path now. `ET_REL` needs
//! relocation records, and a record needs a type NUMBER. Those numbers are
//! derived from `riscv64-elf-as` rather than transcribed, exactly as the
//! encoding fields were (`B-037`).

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

fn table() -> Vec<(String, u32)> {
    let text = std::fs::read_to_string(root().join("spec/relocations-riscv64.tsv"))
        .expect("read spec/relocations-riscv64.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("name\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let (name, number) = l.split_once('\t')?;
            Some((name.to_string(), number.trim().parse().ok()?))
        })
        .collect()
}

#[test]
fn every_relocation_a_linked_program_needs_is_derived() {
    // The four a multi-file program actually uses: a call to another file, the
    // two halves of a pc-relative address, and an absolute pointer in data.
    // Their numbers came out of the `info` field of a real object, not a table
    // anyone typed.
    let t = table();
    let by = |n: &str| t.iter().find(|(k, _)| k == n).map(|(_, v)| *v);
    assert_eq!(by("R_RISCV_JAL"), Some(17), "a call to another file");
    assert_eq!(
        by("R_RISCV_PCREL_HI20"),
        Some(23),
        "auipc half of an address"
    );
    assert_eq!(by("R_RISCV_PCREL_LO12_I"), Some(24), "and its addi half");
    assert_eq!(by("R_RISCV_64"), Some(2), "a pointer in ॱदत्त");
    assert!(t.len() >= 6, "only {} types derived", t.len());
}

#[test]
fn there_is_no_branch_relocation_and_that_is_the_finding() {
    // The assembler never emits one. A branch it cannot resolve is REWRITTEN
    // into an inverted branch over a jump — for a symbol in another section,
    // for an undefined symbol, and with `.option norelax`, every form tried.
    //
    // So `R_RISCV_BRANCH` is not a gap in this table, it is a fact about the
    // oracle: if GNU never leaves a branch for the linker to patch, `बन्धकः`
    // does not need to either. Asserting the absence keeps a later cycle from
    // "fixing" the table by transcribing a number the probe never saw.
    let t = table();
    assert!(
        !t.iter().any(|(n, _)| n == "R_RISCV_BRANCH"),
        "a branch relocation appeared; the probe cannot produce one, so it was typed"
    );
    let text = std::fs::read_to_string(root().join("spec/relocations-riscv64.tsv")).expect("read");
    assert!(
        text.contains("R_RISCV_BRANCH is absent"),
        "the absence must be explained in the file, not just be true"
    );
}

#[test]
fn the_numbers_are_distinct() {
    // Two names sharing a number would make a relocation ambiguous at exactly
    // the moment it is applied, which is after the program is otherwise built.
    let mut seen: Vec<u32> = table().iter().map(|(_, v)| *v).collect();
    let before = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(before, seen.len(), "two relocation types share a number");
}

#[test]
fn an_error_names_the_fault_and_not_a_wording() {
    // `B-078b`. The point of a code is that a test can say WHICH diagnostic
    // fired without pinning any language's prose — `B-015` moved messages into
    // a table so the wording could change, and a test matching on wording is
    // precisely what that would break.
    use std::collections::BTreeMap;
    let p = sadhana::parse::assemble_program("योगःॱभ३२ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect("parses");
    let e = sadhana::encode::encode_at(&p.instructions[0], 0, &BTreeMap::new())
        .expect_err("an integer family with a float type must be refused");

    assert_eq!(e.code, "E03", "`योगः` operates on integers");
    assert!(
        e.args.iter().any(|a| a == "योगः"),
        "the family at fault travels with the error: {:?}",
        e.args
    );

    // And the same fault reads in either language, from the same record.
    let sa = e.message(sadhana::nidana::Language::Sanskrit);
    let en = e.message(sadhana::nidana::Language::English);
    assert_ne!(sa, en, "two languages, one code");
    assert!(en.contains("operates on integers"), "{en}");
    assert!(sa.contains("पूर्णाङ्केषु"), "{sa}");
    for m in [&sa, &en] {
        assert!(m.contains("योगः"), "the name survives translation: {m}");
    }
}

#[test]
fn the_diagnostics_still_written_by_hand_are_counted() {
    // Every one of them now carries a code — the encoder's seventeen
    // (`B-078c`) and the parser's twenty-three (`B-078e`). This was a ceiling
    // that could only fall while the migration ran; it is an equality now,
    // because a diagnostic added without a code cannot be translated or
    // asserted on and would be a step backwards rather than a smaller step
    // forwards.
    //
    // Both files, because the parser was the half left behind: a count that
    // watched only the encoder read as zero for four cycles while ten parse
    // messages were still English formatted at the site that raised them.
    let mut raw = 0;
    for file in [
        "crates/sadhana/src/encode.rs",
        "crates/sadhana/src/parse.rs",
    ] {
        let src = std::fs::read_to_string(root().join(file)).expect("read");
        let n = src.matches("code: \"\"").count();
        assert_eq!(
            n, 0,
            "every diagnostic in {file} carries a code; {n} have gone back to prose"
        );
        raw += n;
    }
    println!("METRIC uncoded_diagnostics {raw}");
}
