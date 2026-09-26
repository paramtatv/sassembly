//! **A SOURCE THAT DECLARES NOTHING IS NOT A FAILED SOURCE.**
//!
//! `मण्डलसङ्कलनम्` has always told the two apart — `सङ्कलनाघोषणाभेद` (१) is "the
//! source parsed to no declarations", `सङ्कलनानिर्णयभेद` (२) is "`अर्थ` refused
//! it" — but both answer EMPTY TEXT, and `मण्डलानिप्रतिबिम्बम्` read only the
//! text's length. So `lib.t1`, which holds no declaration on purpose (its own
//! margin says so), was counted a failure in every build this repository has
//! ever run: `build: 1 source(s) failed to compile (lib)`. The count had a
//! floor and could never express its zero, which is exactly the shape that
//! makes a real regression unreadable — a second failure would have read `2`
//! and nobody would have known which of the two was new.
//!
//! Measured 2026-09-22 on `lib.t1` through `t1_boot --object`:
//! `सङ्कलनविरामभेद = Int(1)`, `निर्णयविरामभेद = Int(3)` (`अर्थ` never entered).

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

const FUEL: u64 = 80_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

fn int(it: &Interpreter, g: &str) -> i128 {
    match it.global(g) {
        Some(Value::Int(k)) => *k,
        other => panic!("{g} is {other:?}, not an integer"),
    }
}

/// A module with one routine — the control that proves the corpus still
/// compiles while the declaration-less source beside it is counted apart.
const REAL: &str = "मण्डलम् रिक्तपरीक्षा ॥
सार्वजनिक वृत्तिः क ददाति न६४ आदि
    प्रत्यागमनम् ७ ।
इति
";

/// `lib.t1`'s shape: margins only, no `मण्डलम्`, no declaration.
const NOTHING: &str = "॰ a file that declares nothing, as `lib.t1` does\n";

#[test]
fn a_source_with_no_declarations_answers_the_no_declarations_kind() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(NOTHING.as_bytes()), octets("रिक्त".as_bytes())],
            FUEL,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(
        text.is_empty(),
        "a source with no declarations emits no text"
    );
    assert_eq!(
        int(&it, "सङ्कलनविरामभेद"),
        1,
        "`सङ्कलनाघोषणाभेद` — parsed to no declarations, NOT `२` (अर्थ refused it)"
    );
}

#[test]
fn the_image_driver_counts_an_empty_source_apart_from_a_failure() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(REAL.as_bytes()), octets(NOTHING.as_bytes())]),
                arena(vec![
                    octets("रिक्तपरीक्षा".as_bytes()),
                    octets("रिक्त".as_bytes()),
                ]),
                Value::Int(2),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();

    assert_eq!(
        int(&it, "सङ्कलनविफलसंख्या"),
        0,
        "the declaration-less source is NOT a failure — this is the count that \
         could never read zero before"
    );
    assert_eq!(
        int(&it, "सङ्कलनरिक्तसंख्या"),
        1,
        "and it is counted, not merely ignored: one source declared nothing"
    );
    // The control: the real module beside it still became an object, so the
    // count above is not zero because nothing was compiled at all.
    assert!(
        !image.is_empty(),
        "the module with a routine still linked to an image"
    );
}
