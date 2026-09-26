//! `W-239` — THE OCTET ARENA, `crates/sadhana-t1/src/ashtaka.t1`, driven by the
//! interpreter: every operation, and every refusal by name.
//!
//! The arena is the assembler's `Vec<u8>` (`Datum::bytes`) as ONE one-based
//! store a datum names a range of. What these tests hold that the interpreter
//! would not: a write past the append position is REFUSED and the store left
//! untouched — `nirvahana.rs` grows a byte run on any indexed write and
//! zero-fills the gap, so without the arena's own check a gap would appear
//! and be padded silently.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn arena() -> Interpreter {
    Interpreter::load(&[("ashtaka.t1", &source("ashtaka.t1"))], &spec_root())
        .unwrap_or_else(|e| panic!("ashtaka.t1 loads on its own: {e:?}"))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn call(it: &mut Interpreter, name: &str, args: Vec<Value>) -> Value {
    it.call(&format!("अष्टक\u{971}{name}"), args, 5_000_000)
        .unwrap_or_else(|e| panic!("{name} runs: {e:?}"))
}

fn int(v: &Value) -> i128 {
    v.as_int()
        .unwrap_or_else(|| panic!("an integer, not {v:?}"))
}

fn bytes_of(v: &Value) -> Vec<u8> {
    match v {
        Value::Octets(o) => o.as_slice().to_vec(),
        other => panic!("a run of octets, not {other:?}"),
    }
}

/// The refusal record: `(is there one, which routine, at what position)`.
fn refusal(it: &Interpreter) -> (bool, String, i128) {
    let is = matches!(it.global("अष्टकदोषमस्ति"), Some(Value::Bool(true)));
    let name = match it.global("अष्टकदोषनाम") {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        _ => String::new(),
    };
    let at = it
        .global("अष्टकदोषस्थानम्")
        .and_then(Value::as_int)
        .unwrap_or(-1);
    (is, name, at)
}

fn length(it: &mut Interpreter) -> i128 {
    int(&call(it, "अष्टकदैर्घ्य", vec![]))
}

#[test]
fn push_advances_then_writes_and_the_first_octet_is_at_one() {
    let mut it = arena();
    assert_eq!(int(&call(&mut it, "अष्टकारम्भः", vec![])), 0);
    assert_eq!(length(&mut it), 0);
    assert_eq!(int(&call(&mut it, "अष्टकयोजनम्", vec![Value::Int(80)])), 1);
    assert_eq!(int(&call(&mut it, "अष्टकयोजनम्", vec![Value::Int(48)])), 2);
    assert_eq!(int(&call(&mut it, "अष्टकयोजनम्", vec![Value::Int(51)])), 3);
    assert_eq!(length(&mut it), 3);
    for (i, b) in [(1, 80), (2, 48), (3, 51)] {
        assert_eq!(
            int(&call(&mut it, "अष्टकपाठः", vec![Value::Int(i)])),
            b,
            "octet {i}"
        );
    }
    // Slot ० is reserved and a real zero byte, so the arena's index is the
    // run's offset: the store is `[0, 80, 48, 51]`.
    let store = bytes_of(it.global("अष्टककोश").expect("the store"));
    assert_eq!(store, vec![0, 80, 48, 51]);
    assert_eq!(refusal(&it), (false, String::new(), 0));
}

#[test]
fn zeros_are_pushed_by_count_and_a_count_of_none_writes_none() {
    let mut it = arena();
    call(&mut it, "अष्टकारम्भः", vec![]);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(7)]);
    assert_eq!(int(&call(&mut it, "शून्याष्टकयोजनम्", vec![Value::Int(8)])), 8);
    assert_eq!(length(&mut it), 9);
    assert_eq!(int(&call(&mut it, "शून्याष्टकयोजनम्", vec![Value::Int(0)])), 0);
    assert_eq!(length(&mut it), 9);
    let run = bytes_of(&call(
        &mut it,
        "अष्टकखण्डः",
        vec![Value::Int(2), Value::Int(8)],
    ));
    assert_eq!(run, vec![0; 8]);
}

#[test]
fn a_range_of_a_run_is_appended_and_handed_back_as_the_same_octets() {
    let mut it = arena();
    call(&mut it, "अष्टकारम्भः", vec![]);
    // `नमस्ते` is 18 octets of UTF-8; append `नमः` (the first 9) from the
    // 0-based range a token carries, then a second word after a space.
    let text = "नमस्ते";
    let first = int(&call(
        &mut it,
        "अष्टकपाठयोजनम्",
        vec![octets(text), Value::Int(0), Value::Int(9)],
    ));
    assert_eq!(first, 1, "the arena index of the first octet appended");
    assert_eq!(length(&mut it), 9);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(32)]);
    let second = int(&call(
        &mut it,
        "अष्टकपाठयोजनम्",
        vec![octets(text), Value::Int(9), Value::Int(18)],
    ));
    assert_eq!(second, 11);
    assert_eq!(length(&mut it), 19);
    let whole = bytes_of(&call(
        &mut it,
        "अष्टकखण्डः",
        vec![Value::Int(1), Value::Int(19)],
    ));
    let mut want = text.as_bytes()[..9].to_vec();
    want.push(32);
    want.extend_from_slice(&text.as_bytes()[9..]);
    // Octets, not characters: the cut at 9 falls inside an akṣara (after `स`,
    // before its virama), and the arena neither knows nor cares — a token's
    // range is octets too.
    assert_eq!(whole, want);
    // An empty range appends nothing and answers ०, which is not a refusal.
    assert_eq!(
        int(&call(
            &mut it,
            "अष्टकपाठयोजनम्",
            vec![octets(text), Value::Int(4), Value::Int(4)]
        )),
        0
    );
    assert_eq!(length(&mut it), 19);
    assert!(!refusal(&it).0);
}

#[test]
fn a_write_at_the_append_position_appends_and_one_inside_overwrites() {
    let mut it = arena();
    call(&mut it, "अष्टकारम्भः", vec![]);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(1)]);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(2)]);
    assert_eq!(
        call(&mut it, "अष्टकस्थापनम्", vec![Value::Int(3), Value::Int(9)]),
        Value::Bool(true),
        "position len+1 is Vec's one liberty: a push"
    );
    assert_eq!(length(&mut it), 3);
    assert_eq!(
        call(&mut it, "अष्टकस्थापनम्", vec![Value::Int(1), Value::Int(5)]),
        Value::Bool(true)
    );
    assert_eq!(length(&mut it), 3);
    let run = bytes_of(&call(
        &mut it,
        "अष्टकखण्डः",
        vec![Value::Int(1), Value::Int(3)],
    ));
    assert_eq!(run, vec![5, 2, 9]);
    assert!(!refusal(&it).0);
}

/// THE REFUSED CASE the row names: an append PAST the arena's end is refused
/// by name, not wrapped and not padded. The interpreter alone would have
/// grown the run to reach the index and zero-filled the gap.
#[test]
fn a_write_past_the_append_position_is_refused_by_name_and_the_store_is_untouched() {
    let mut it = arena();
    call(&mut it, "अष्टकारम्भः", vec![]);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(1)]);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(2)]);
    let before = bytes_of(it.global("अष्टककोश").expect("the store"));

    assert_eq!(
        call(&mut it, "अष्टकस्थापनम्", vec![Value::Int(4), Value::Int(9)]),
        Value::Bool(false),
        "position len+2 leaves a gap: refused"
    );
    assert_eq!(refusal(&it), (true, "अष्टकस्थापनम्".to_string(), 4));
    assert_eq!(length(&mut it), 2, "the cursor did not move");
    let after = bytes_of(it.global("अष्टककोश").expect("the store"));
    assert_eq!(after, before, "the run was not grown, not padded");

    // Position ० is the reserved slot: refused too, and the FIRST refusal
    // keeps the record.
    assert_eq!(
        call(&mut it, "अष्टकस्थापनम्", vec![Value::Int(0), Value::Int(9)]),
        Value::Bool(false)
    );
    assert_eq!(refusal(&it), (true, "अष्टकस्थापनम्".to_string(), 4));

    // A fresh program clears the record.
    call(&mut it, "अष्टकारम्भः", vec![]);
    assert_eq!(refusal(&it), (false, "अष्टकस्थापनम्".to_string(), 0));
    assert_eq!(length(&mut it), 0);
}

#[test]
fn a_read_or_a_range_past_the_end_is_refused_by_name() {
    let mut it = arena();
    call(&mut it, "अष्टकारम्भः", vec![]);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(7)]);
    assert_eq!(int(&call(&mut it, "अष्टकपाठः", vec![Value::Int(2)])), 0);
    assert_eq!(refusal(&it), (true, "अष्टकपाठः".to_string(), 2));

    call(&mut it, "अष्टकारम्भः", vec![]);
    call(&mut it, "अष्टकयोजनम्", vec![Value::Int(7)]);
    let run = bytes_of(&call(
        &mut it,
        "अष्टकखण्डः",
        vec![Value::Int(1), Value::Int(2)],
    ));
    assert!(
        run.is_empty(),
        "a range that leaves the arena answers the empty run"
    );
    assert_eq!(refusal(&it), (true, "अष्टकखण्डः".to_string(), 3));

    call(&mut it, "अष्टकारम्भः", vec![]);
    let n = int(&call(
        &mut it,
        "अष्टकपाठयोजनम्",
        vec![octets("अ"), Value::Int(0), Value::Int(9)],
    ));
    assert_eq!(n, 0, "a source range past its run appends nothing");
    assert_eq!(refusal(&it), (true, "अष्टकपाठयोजनम्".to_string(), 9));
    assert_eq!(length(&mut it), 0);
}
