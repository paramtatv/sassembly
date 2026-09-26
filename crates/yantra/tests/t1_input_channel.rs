//! **The input channel's contract, checked where it actually holds: in the
//! EMITTED text.**
//!
//! `yantra::input` finds the program's input slots by VALUE — the images carry
//! no symbol table — and writes each at `tag + 8`. That is only right if, in
//! the object the compiler really emits, every slot's word sits IMMEDIATELY
//! after its tag's word.
//!
//! The first design put one tag before three slots and read them at +8, +16 and
//! +24. The `.t1` SOURCE made that look fine: four declarations, one after
//! another. The EMITTED text refuted it — every run global gets 1024 octets of
//! storage, and after each one `emit_data` returns to `ॱदत्त` with
//! `॥ संरेखः १६ ॥`, so a word FOLLOWING a run is re-aligned. Caught by printing
//! the emitted text before a 2h42m image build rather than after. So this file
//! reads the emitted text and asserts the property there.
//!
//! It also pins the three values twice-stated — `yantra::input`'s constants and
//! the `.t1` literals — to each other.

use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};
use yantra::input::{INPUT_TAG, NAME_TAG, TRACE_TAG};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn shrinkhala() -> String {
    std::fs::read_to_string(repo().join("crates/sadhana-t1/src/shrinkhala.t1"))
        .expect("shrinkhala.t1 is readable")
}

/// The three (tag global, slot global, tag value) triples of the interface.
const PAIRS: [(&str, &str, u64); 3] = [
    ("निवेशसङ्केतः", "निवेशपाठः", INPUT_TAG),
    ("निवेशनामसङ्केतः", "निवेशमण्डलनाम", NAME_TAG),
    ("निवेशानुरेखणसङ्केतः", "निवेशानुरेखणम्", TRACE_TAG),
];

fn devanagari_to_u64(s: &str) -> u64 {
    s.chars().fold(0u64, |acc, c| {
        let d = "०१२३४५६७८९"
            .chars()
            .position(|x| x == c)
            .expect("a Devanagari digit");
        acc * 10 + d as u64
    })
}

/// Each tag's `.t1` literal equals the constant the host scans for.
#[test]
fn the_three_tags_are_one_value_in_two_places() {
    let src = shrinkhala();
    for (tag, _, want) in PAIRS {
        let head = format!("सार्वजनिक चरः {tag} ॱॱ न६४ भवति ");
        let line = src
            .lines()
            .find(|l| l.starts_with(&head))
            .unwrap_or_else(|| panic!("shrinkhala.t1 declares no `{tag}`"));
        let lit = line[head.len()..].split_whitespace().next().unwrap();
        assert_eq!(
            devanagari_to_u64(lit),
            want,
            "`{tag}` is {lit} in shrinkhala.t1 but {want:#x} in yantra::input — the host \
             would scan for a word the image does not hold"
        );
        assert!(
            want < 1 << 63,
            "a tag at or above 2^63 is negative to a signed lowering"
        );
    }
}

/// In the EMITTED `ॱदत्त` text, each slot's word is the next data word after
/// its tag's — no directive that moves the location counter between them.
#[test]
fn each_slot_is_the_next_data_word_after_its_tag_in_the_emitted_text() {
    let src = shrinkhala();
    let module = module_name(&src).expect("shrinkhala.t1 declares a module");
    let mut f = Front::load(&repo().join("spec")).expect("the front end loads");
    f.lex(&src).unwrap();
    f.parse().unwrap();
    f.resolve().unwrap();
    f.typecheck().unwrap();
    f.build_ir().unwrap();
    let text = riscv64::emit_module(&f.module(&module, None).unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().collect();

    for (tag, slot, _) in PAIRS {
        let tag_label = format!("{module}{tag}ॱॱ");
        let slot_label = format!("{module}{slot}ॱॱ");
        let t = lines
            .iter()
            .position(|l| *l == tag_label)
            .unwrap_or_else(|| panic!("`{tag_label}` is not in the emitted text"));
        // The tag's own word, then — before the slot's label — nothing but an
        // export directive. Anything that emits octets or moves the counter
        // (a section switch, an alignment, another word) breaks `tag + 8`.
        assert!(
            lines[t + 1].starts_with("॥ अष्टाष्टकाः "),
            "`{tag}` is not followed by its word: {:?}",
            lines[t + 1]
        );
        let between: Vec<&str> = lines[t + 2..]
            .iter()
            .take_while(|l| **l != slot_label)
            .copied()
            .collect();
        assert!(
            between.iter().all(|l| l.starts_with("॥ वैश्विकम् ")),
            "between `{tag}` and `{slot}` the emitter wrote {between:?} — only an export \
             directive may sit there, or the slot is not at tag + 8"
        );
        let s = t + 2 + between.len();
        assert_eq!(lines[s], slot_label, "`{slot}` does not follow `{tag}`");
        assert!(
            lines[s + 1].starts_with("॥ अष्टाष्टकाः "),
            "`{slot}` begins with {:?}, not a word — the host would write the wrong thing",
            lines[s + 1]
        );
    }
}
