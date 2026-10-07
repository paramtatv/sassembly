//! **W-333 — THE MARK ON KIND १०, READ THROUGH THE RUST TWIN.**
//!
//! `yantra`'s `w333_logical_shift.rs` runs the shift on both engines and
//! asserts values. This file asserts the three things a value cannot show:
//!
//! 1. that `मध्यरूप` WRITES the mark and `chain.rs`'s hand-written decoder
//!    READS it — `Instruction::ShrL` for a name declared unsigned,
//!    `Instruction::Shr` for a signed name and for a literal — and that the
//!    Rust emitter then writes the same verb the `.t1` emitter does. `W-306c`
//!    recorded the hazard this is for: a decoder that reads the kind and drops
//!    the field compiles, stays green and answers differently from its
//!    siblings. `Instruction` matches use `_` arms, so the compiler does NOT
//!    find a consumer that ignores `ShrL`; only a test that feeds one can.
//! 2. that the interpreter's copy of "which type names are unsigned" is the
//!    list `artha.t1` declares, since the two engines choose the shift from
//!    two different readings of one property.
//! 3. that the two verbs are different words. `दक्षिणसरणम्` is a SUFFIX of
//!    `सचिह्नदक्षिणसरणम्`, so `text.contains("दक्षिणसरणम् ")` is true of both
//!    and proves nothing; every check below anchors the verb at a line start.

use sadhana::t1::chain::{CHAIN, Front};
use sadhana::t1::ir::Instruction;
use sadhana::t1::nirvahana::{Interpreter, Octets, UNSIGNED_INTEGER_TYPES, Value};
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn fixture(decl: &str, shift: &str) -> String {
    format!(
        "मण्डलम् सरणपरीक्षा ॥\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
         {decl}    प्रत्यागमनम् {shift} ।\nइति\n"
    )
}

/// How many arithmetic and how many logical right shifts the Rust twin's
/// module holds for `src`, and the text the Rust emitter writes for it.
fn through_the_rust_twin(src: &str) -> (usize, usize, String) {
    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(src).expect("the fixture lexes");
    front.parse().expect("the fixture parses");
    front.resolve().expect("the fixture resolves");
    front.typecheck().expect("the fixture typechecks");
    front.build_ir().expect("the fixture builds IR");
    let module = front
        .module("सरणपरीक्षा", Some("मुख्यम्"))
        .expect("the IR reads into the emitter's module");
    let (mut arithmetic, mut logical) = (0, 0);
    for f in &module.functions {
        for b in f.blocks.values() {
            for (_, inst) in &b.insts {
                match inst {
                    Instruction::Shr(..) => arithmetic += 1,
                    Instruction::ShrL(..) => logical += 1,
                    _ => {}
                }
            }
        }
    }
    let text = riscv64::emit_module(&module).expect("riscv64.rs emits the fixture");
    (arithmetic, logical, text)
}

/// The text the `.t1` emitter writes for `src`, through the product's own
/// entry point.
fn through_the_t1_emitter(src: &str) -> String {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![
                Value::Octets(Octets::new(src.as_bytes())),
                Value::Octets(Octets::new("सरणपरीक्षा".as_bytes())),
            ],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!text.is_empty(), "the `.t1` compiler refused the fixture");
    String::from_utf8(text).expect("the emitted text is UTF-8")
}

/// Lines that BEGIN with the verb — never `contains`, see the file header.
fn lines_beginning(text: &str, verb: &str) -> usize {
    let head = format!("{verb} ");
    text.lines().filter(|l| l.starts_with(&head)).count()
}

const LOGICAL: &str = "दक्षिणसरणम्";
const ARITHMETIC: &str = "सचिह्नदक्षिणसरणम्";

#[test]
fn a_shift_of_an_unsigned_name_is_marked_and_both_emitters_write_the_logical_verb() {
    let src = fixture("    चरः क ॱॱ न६४ भवति ऋण१ ।\n", "क दक्षिणसृ ६३");
    let (arithmetic, logical, rust) = through_the_rust_twin(&src);
    assert_eq!(
        (arithmetic, logical),
        (0, 1),
        "(Shr, ShrL): `chain.rs` must read kind १०'s `ध्रुवमूल्यम्` — a decoder \
         that drops it builds (1, 0) here and compiles"
    );
    assert_eq!(
        (
            lines_beginning(&rust, LOGICAL),
            lines_beginning(&rust, ARITHMETIC)
        ),
        (1, 0),
        "riscv64.rs: one `{LOGICAL}` line and no `{ARITHMETIC}`\n{rust}"
    );
    let t1 = through_the_t1_emitter(&src);
    assert_eq!(
        (
            lines_beginning(&t1, LOGICAL),
            lines_beginning(&t1, ARITHMETIC)
        ),
        (1, 0),
        "yantrotsarjana.t1: the same verb the Rust twin wrote\n{t1}"
    );
}

/// THE CONTROL: the two shapes that must keep the arithmetic shift. A marker
/// that fired on every right shift would pass the test above.
#[test]
fn a_shift_of_a_signed_name_and_of_a_literal_stay_arithmetic_in_both_emitters() {
    for (what, src) in [
        (
            "a signed name",
            fixture("    चरः स ॱॱ अ६४ भवति ऋण८ ।\n", "स दक्षिणसृ १"),
        ),
        ("a literal", fixture("", "ऋण१ दक्षिणसृ ६३")),
    ] {
        let (arithmetic, logical, rust) = through_the_rust_twin(&src);
        assert_eq!((arithmetic, logical), (1, 0), "{what}: (Shr, ShrL)");
        assert_eq!(
            (
                lines_beginning(&rust, LOGICAL),
                lines_beginning(&rust, ARITHMETIC)
            ),
            (0, 1),
            "{what}, riscv64.rs\n{rust}"
        );
        let t1 = through_the_t1_emitter(&src);
        assert_eq!(
            (
                lines_beginning(&t1, LOGICAL),
                lines_beginning(&t1, ARITHMETIC)
            ),
            (0, 1),
            "{what}, yantrotsarjana.t1\n{t1}"
        );
    }
}

/// WHERE EACH LIST LIVES, so an addition to one is made knowing the other
/// exists: the checker's is `crates/sadhana-t1/src/artha.t1`, routine
/// `मूलप्रकारार्थः`, one `यदि नाम समम् … आदि` arm per type name; the
/// interpreter's is `UNSIGNED_INTEGER_TYPES` in
/// `crates/sadhana/src/t1/nirvahana.rs`. This test READS the first and
/// compares, so adding an unsigned integer to `artha.t1` alone reds it.
///
/// **ONE PROPERTY, TWO READINGS, AND THIS IS WHAT KEEPS THEM ONE.** Natively
/// the shift is chosen from `अर्थप्रकार`'s `चिह्नितम्`, which `मूलप्रकारार्थः`
/// sets per type name; the interpreter chooses it from the type's SPELLING,
/// against `UNSIGNED_INTEGER_TYPES`. A sixth unsigned integer added to
/// `artha.t1` and not to that array would be logical natively and arithmetic
/// interpreted — so the array is compared with what `artha.t1` says.
#[test]
fn the_interpreters_unsigned_type_names_are_the_ones_artha_declares() {
    let artha = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/artha.t1"))
        .expect("artha.t1 is readable");
    let (_, body) = artha
        .split_once("सार्वजनिक वृत्तिः मूलप्रकारार्थः ")
        .expect("artha.t1 declares मूलप्रकारार्थः");
    let body = body.split("\nइति\n").next().expect("the routine closes");
    // One arm per type name: `यदि नाम समम् आरभ्य उक्तम् NAME इति समाप्तम् आदि`
    // and then the fields it sets, up to the arm's `प्रत्यागमनम्`.
    let mut unsigned: Vec<&str> = Vec::new();
    let mut arms = 0;
    for arm in body.split("यदि नाम समम् आरभ्य उक्तम् ").skip(1)
    {
        arms += 1;
        let name = arm.split(' ').next().expect("the arm names a type");
        let fields = arm.split("प्रत्यागमनम्").next().unwrap_or("");
        if fields.contains("भेद भवति पूर्णाङ्कार्थभेद") && fields.contains("चिह्नितम् भवति असत्यम्")
        {
            unsigned.push(name);
        }
    }
    assert!(
        arms >= 10,
        "the scan read only {arms} arm(s) of मूलप्रकारार्थः — it has stopped \
         matching the routine's shape and its answer below means nothing"
    );
    assert_eq!(
        unsigned, UNSIGNED_INTEGER_TYPES,
        "the type names `artha.t1` makes an unsigned integer, in its order, \
         against `nirvahana.rs`'s `UNSIGNED_INTEGER_TYPES`"
    );
}
