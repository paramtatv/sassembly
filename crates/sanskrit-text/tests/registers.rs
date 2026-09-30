//! `spec/registers-riscv64.tsv` must be complete, unambiguous and writable — B-005.
//!
//! A register table is the one place where a plausible-looking error is
//! completely silent: name `t3` as x7 instead of x28 and the assembler emits a
//! valid instruction operating on the wrong register. No diagnostic, wrong
//! program. So the numbers are derived from the assembler and these tests check
//! the properties that would survive a hand edit.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

struct Reg {
    devanagari: String,
    abi: String,
    number: u32,
    class: String,
    source: String,
}

fn regs() -> Vec<Reg> {
    std::fs::read_to_string(root().join("spec/registers-riscv64.tsv"))
        .expect("read spec/registers-riscv64.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t") && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 6, "malformed row: {l:?}");
            Reg {
                devanagari: f[0].into(),
                abi: f[1].into(),
                number: f[2].parse().expect("number"),
                class: f[3].into(),
                source: f[5].into(),
            }
        })
        .collect()
}

#[test]
fn every_register_name_can_actually_be_written() {
    // A register outside the doc 15 repertoire could not appear in Sassembly
    // source, so the name would be unusable while looking fine in a table.
    for r in regs() {
        let v = sanskrit_text::repertoire::check(&r.devanagari);
        assert!(
            v.is_empty(),
            "{} ({}) is outside the repertoire: {v:?}",
            r.devanagari,
            r.abi
        );
        assert!(sanskrit_text::is_nfc(&r.devanagari), "{} is not NFC", r.abi);
    }
}

#[test]
fn both_files_cover_all_thirty_two_of_each_class() {
    // A missing register is one a program cannot name — and RISC-V has exactly
    // 32 of each, so the count is checkable rather than a matter of judgement.
    for class in ["int", "float"] {
        let mut nums: Vec<u32> = regs()
            .into_iter()
            .filter(|r| r.class == class)
            .map(|r| r.number)
            .collect();
        nums.sort_unstable();
        assert_eq!(nums, (0..32).collect::<Vec<u32>>(), "{class} registers");
    }
}

#[test]
fn no_two_registers_share_a_name_or_a_slot() {
    let mut by_name: BTreeMap<String, String> = BTreeMap::new();
    let mut by_slot: BTreeMap<(String, u32), String> = BTreeMap::new();
    for r in regs() {
        if let Some(prev) = by_name.insert(r.devanagari.clone(), r.abi.clone()) {
            panic!("{} names both {prev} and {}", r.devanagari, r.abi);
        }
        if let Some(prev) = by_slot.insert((r.class.clone(), r.number), r.abi.clone()) {
            panic!(
                "{} {} is claimed by both {prev} and {}",
                r.class, r.number, r.abi
            );
        }
    }
}

#[test]
fn the_split_ranges_are_what_the_assembler_says() {
    // t0-t6 are x5-x7 then x28-x31; s0-s11 are x8, x9 then x18-x27. These are
    // the mappings a person gets wrong from memory, which is why they are
    // derived — this pins what was derived so a hand edit cannot undo it.
    let by: BTreeMap<String, u32> = regs().into_iter().map(|r| (r.abi, r.number)).collect();
    for (abi, want) in [
        ("zero", 0),
        ("ra", 1),
        ("sp", 2),
        ("t0", 5),
        ("t2", 7),
        ("t3", 28),
        ("t6", 31),
        ("s0", 8),
        ("s1", 9),
        ("s2", 18),
        ("s11", 27),
        ("a0", 10),
        ("a7", 17),
    ] {
        assert_eq!(by[abi], want, "{abi} should be x{want}, got x{}", by[abi]);
    }
}

#[test]
fn rows_claiming_the_lexicon_are_telling_the_truth() {
    // Doc 01 §4 rule 5, the same check the mnemonics carry: a row saying the
    // lexicon already names a concept, when it does not, is how a second
    // vocabulary starts. Indexed names are checked on their stem, since
    // क्षणिक० is क्षणिक plus a numeral.
    let lex = std::fs::read_to_string(root().join("spec/lexicon.tsv")).expect("read lexicon");
    let known: Vec<&str> = lex
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next())
        .collect();

    let mut missing = Vec::new();
    for r in regs() {
        if r.source != "lexicon" {
            continue;
        }
        let stem: String = r
            .devanagari
            .chars()
            .filter(|c| !('\u{0966}'..='\u{096F}').contains(c))
            .collect();
        if !known.contains(&stem.as_str()) {
            missing.push(format!("{} ({}) stem {stem}", r.devanagari, r.abi));
        }
    }
    assert!(
        missing.is_empty(),
        "claim `lexicon` but are not in it:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn indexed_names_use_devanagari_numerals() {
    // `क्षणिक3` would be a Latin digit inside a Sassembly identifier — the exact
    // breach doc 15 exists to prevent, and easy to introduce when generating
    // names from a loop counter.
    for r in regs() {
        assert!(
            !r.devanagari.chars().any(|c| c.is_ascii_digit()),
            "{} ({}) contains an ASCII digit",
            r.devanagari,
            r.abi
        );
    }
    let by: BTreeMap<String, String> = regs().into_iter().map(|r| (r.abi, r.devanagari)).collect();
    assert_eq!(by["t3"], "क्षणिक३");
    assert_eq!(by["a0"], "अर्थ०");
    assert_eq!(by["f31"], "प्लव३१");
    println!("METRIC sassembly_registers_named {}", regs().len());
}
