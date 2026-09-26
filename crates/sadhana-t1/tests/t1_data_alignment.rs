//! **EVERY OBJECT'S `.data` STARTS ALIGNED — THE LINKER IS THE ONLY PLACE IT CAN.**
//!
//! `encode.t1:5916` puts each object's `.data` at `संरेखान्तरम् पाठसीमा ८`:
//! eight-aligned **relative to that object**. `samyojana.t1`'s `दत्तारम्भः` then
//! concatenated the objects with a bare sum of their data lengths and no
//! rounding, so an object's whole aligned interior landed wherever the
//! preceding objects' lengths happened to end.
//!
//! Measured 2026-09-21 on the real self-image (`crates/yantra/src/input.rs`):
//! all three interface tags sat at addresses ≡ ५ (mod ८) — **odd**, failing
//! even halfword alignment. Measured again 2026-09-23 after the repair, same
//! instrument, same image: ≡ ० (mod ८).
//!
//! **`॥ संरेखः १६ ॥` IS NOT WHAT THIS IS ABOUT**, though the margin that
//! reported the defect said it was, and that misreading cost a round: the
//! directive is `.text` only — `संरेखपूरणाष्टकम्` pads with `nop`, because
//! 0x00000000 traps on RISC-V — and it never governed a data word. Eight is
//! what the assembler promises and what `ld`/`sd` require.
//!
//! **SIXTEEN WAS TRIED FIRST AND THE TREE REFUSED IT.** Moving `दत्ताधारः` to
//! sixteen reddened `t1_exec_link` twice, one of them a MUTATION test that
//! neuters the `८` inside `अष्टकसंरेखणम्` and asserts `.data` then begins where
//! the text ended. Eight is a contract twinned across the assembler, the linker
//! and the way `kosha` lays an image out, and a twin moved on one side only is
//! how this corpus makes silent defects. `दत्ताधारः` was never part of this
//! defect and is untouched; what was missing is rounding BETWEEN objects.
//!
//! **WHY THIS TESTS THE ROUTINE AND NOT AN IMAGE.** The property is a statement
//! about `दत्तारम्भः` over a set of object lengths, and a built image exercises
//! exactly one such set — the corpus's own, whose lengths could all be
//! congruent by luck and prove nothing. Three objects with lengths ३, ५ and ७
//! are chosen because every one of them is odd and none is a multiple of ८, so
//! an unrounded sum gives ०, ३, ८ and lands the third object somewhere no
//! doubleword load may touch. A 25-minute self-image cannot say more than this
//! about the rule, and says it about one input.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(Rc::new(RefCell::new(vs)))
}

/// A `वास्तुॱवस्तु` with the given `.data` length and nothing else that matters.
///
/// A `संरचना` is `Value::Record`, a map keyed by FIELD NAME — not a positional
/// arena, which is what a first version of this fixture built and what made
/// the interpreter refuse `ॱ शून्यक्षेत्रम्` outright. `दत्तारम्भः` reads
/// `दत्तम्`'s length and `बीजारम्भः` reads `शून्यक्षेत्रम्`, and NOTHING ELSE, so
/// the remaining fields of `vastu.t1:68` are empty: a fixture that filled them
/// would be asserting about fields the routines never touch.
fn object(data_len: usize, bss: i128) -> Value {
    let mut m: HashMap<String, Value> = HashMap::new();
    m.insert("पाठ्यम्".into(), Value::Octets(Octets::new(b"")));
    m.insert(
        "दत्तम्".into(),
        Value::Octets(Octets::new(&vec![0xAAu8; data_len])),
    );
    m.insert("संज्ञाः".into(), arena(vec![]));
    m.insert("पुनःस्थापनानि".into(), arena(vec![]));
    m.insert("शून्यक्षेत्रम्".into(), Value::Int(bss));
    m.insert("शोधनम्".into(), arena(vec![]));
    m.insert("शोधनपुनःस्थापनानि".into(), arena(vec![]));
    m.insert("दत्तपुनःस्थापनानि".into(), arena(vec![]));
    Value::Record(Rc::new(RefCell::new(m)))
}

fn data_start(it: &mut Interpreter, objects: &Value, index: i128) -> i128 {
    it.call(
        "संयोजनॱदत्तारम्भः",
        vec![objects.clone(), Value::Int(index)],
        1_000_000_000,
    )
    .expect("दत्तारम्भः runs")
    .as_int()
    .expect("it answers a number")
}

fn bss_start(it: &mut Interpreter, objects: &Value, index: i128) -> i128 {
    it.call(
        "संयोजनॱबीजारम्भः",
        vec![objects.clone(), Value::Int(index)],
        1_000_000_000,
    )
    .expect("बीजारम्भः runs")
    .as_int()
    .expect("it answers a number")
}

#[test]
fn every_objects_data_starts_on_an_eight_boundary() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    // Three odd lengths, none a multiple of ८: an unrounded sum gives ०, ३, ८.
    let objects = arena(vec![object(3, 0), object(5, 0), object(7, 0)]);

    let starts: Vec<i128> = (0..4).map(|i| data_start(&mut it, &objects, i)).collect();

    assert_eq!(
        starts,
        vec![0, 8, 16, 23],
        "an object's `.data` start must be eight-aligned. A bare sum of the \
         preceding lengths gives [0, 3, 8, 15] and puts every object after the \
         first at an address no doubleword load may touch — which is what the \
         self-image shipped with, tags at ≡ 5 (mod 8). The last entry is the \
         section LENGTH, not a start, so it is the true end and is NOT padded"
    );

    for (i, s) in starts.iter().take(3).enumerate() {
        assert_eq!(
            s % 8,
            0,
            "object {i}'s `.data` starts at {s}, which is {} past an eight \
             boundary",
            s % 8
        );
    }
}

#[test]
fn the_sections_length_is_the_true_end_and_is_not_padded() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let objects = arena(vec![object(3, 0), object(5, 0), object(7, 0)]);

    // ONE ROUTINE, TWO QUESTIONS. `समयोजना` reads the section's LENGTH as the
    // start one past the last object (`samyojana.t1:1586`) — so `दत्तारम्भः`
    // answers both "where does object k begin" (align it) and "how long is the
    // section" (do not). Rounding both is what over-reserved `.bss`.
    assert_eq!(
        data_start(&mut it, &objects, 3),
        23,
        "the LENGTH is not padded. `समयोजना` reads it as the start one past \
         the last object (`samyojana.t1:1586`), but a length is a count of \
         octets and not an address: rounding it reported `.bss` as 8 where the \
         object asked for 5, which `t1_exec_link` calls `the reservation is \
         carried, in memory and not in the file`"
    );
}

#[test]
fn reserved_bytes_are_aligned_on_the_same_rule() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    // `शून्यक्षेत्रम्` is a COUNT, not a run: `.bss` is space in memory and
    // absent from the file, so this field is a number where `दत्तम्` is octets.
    let objects = arena(vec![object(0, 3), object(0, 5), object(0, 7)]);

    let starts: Vec<i128> = (0..4).map(|i| bss_start(&mut it, &objects, i)).collect();

    assert_eq!(
        starts,
        vec![0, 8, 16, 23],
        "`.bss` was concatenated by the same bare sum as `.data` and needs the \
         same rounding, or reserved storage inherits exactly the drift that \
         was just taken out of `.data`"
    );
}
