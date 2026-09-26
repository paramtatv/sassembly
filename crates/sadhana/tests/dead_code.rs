//! The dead-code budget is zero — task `B-023`, doc 18 §6a.3.
//!
//! `purpose.md`: software accretes because each layer was reasonable when added
//! and nothing is ever deleted. A byte no control path reaches is mass that
//! shipped for no reason, and the budget for it is zero rather than small.
//!
//! Applied to `spec/bare-metal.sas`, which is the only program SANSOS currently
//! ships and runs.

use std::path::{Path, PathBuf};

use sadhana::census::census;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

fn shipped_program() -> Vec<u32> {
    let src = std::fs::read_to_string(root().join("spec/bare-metal.sas")).expect("read");
    let p = sadhana::parse::assemble_program(&src).expect("parses");
    sadhana::encode::words(&sadhana::encode::encode_program(&p).expect("encodes"))
}

#[test]
fn the_shipped_program_has_no_unreachable_bytes() {
    let words = shipped_program();
    let c = census(&words);
    let dead = c.dead();
    assert!(
        dead.is_empty(),
        "{} unreachable byte(s) at instruction(s) {dead:?} — the budget is zero",
        c.dead_bytes()
    );
    println!("METRIC dead_code_bytes {}", c.dead_bytes());
    println!("METRIC census_instructions {}", words.len());
}

#[test]
fn the_census_saw_the_whole_program() {
    // A zero-byte report is only worth having if the analysis was not blind.
    // An indirect jump or an undecodable word means "unreached" cannot be
    // trusted to mean "unreachable", and the budget would be measuring nothing.
    let c = census(&shipped_program());
    assert!(
        c.is_complete(),
        "the census was incomplete: {} indirect jump(s), {} undecodable word(s)",
        c.indirect.len(),
        c.undecodable.len()
    );
}

#[test]
fn the_budget_would_notice_dead_code_if_there_were_any() {
    // A budget that has never failed is a budget nobody has tested. Appending
    // an instruction after the program's final `जुम्प` to itself makes it
    // unreachable, and the census must say so.
    let mut words = shipped_program();
    words.push(0x0000_0013); // addi zero, zero, 0
    let c = census(&words);
    assert_eq!(
        c.dead_bytes(),
        4,
        "an instruction after an unconditional jump is unreachable"
    );
}

#[test]
fn no_shipped_program_emits_the_same_body_twice() {
    // `B-024`, the other half of doc 18 §6a.3's zero budget. Dead code is mass
    // nothing reaches; a duplicate body is mass paid for twice.
    for name in ["spec/bare-metal.sas", "spec/namaste.sas"] {
        let src = std::fs::read_to_string(root().join(name)).expect("read");
        let p = sadhana::parse::assemble_program(&src).expect("parses");
        let words = sadhana::encode::words(&sadhana::encode::encode_program(&p).expect("encodes"));
        let d = sadhana::duplicates::duplicates(&words);
        assert!(
            d.is_empty(),
            "{name}: {} duplicated byte(s) — {d:?}",
            sadhana::duplicates::wasted_bytes(&d)
        );
    }
    println!("METRIC duplicate_body_bytes 0");
}

#[test]
fn the_duplicate_budget_would_notice_a_copy() {
    // The same discipline as the dead-code budget: a budget that has never
    // failed is a budget nobody has tested. This program calls two functions
    // with identical bodies, and the detector must say so.
    let src = "\
लङ्घनम् पुनःस्थानम्म् एकम्य् ।
लङ्घनम् पुनःस्थानम्म् द्वितीयम्य् ।
लङ्घनम् शून्यःम् चक्रःय् ।
एकम्ॱॱ
योगः अर्थ०म् शून्यःन ७न ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।
द्वितीयम्ॱॱ
योगः अर्थ०म् शून्यःन ७न ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।
चक्रःॱॱ
लङ्घनम् शून्यःम् चक्रःय् ।
";
    let p = sadhana::parse::assemble_program(src).expect("parses");
    let words = sadhana::encode::words(&sadhana::encode::encode_program(&p).expect("encodes"));
    let d = sadhana::duplicates::duplicates(&words);
    assert_eq!(
        sadhana::duplicates::wasted_bytes(&d),
        8,
        "one duplicated body"
    );
}
