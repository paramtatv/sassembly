//! **A GLOBAL INITIALISED `सत्यम्` IS TRUE IN THE IMAGE, NOT ONLY IN THE INTERPRETER.**
//!
//! `ir.t1`'s global-initialiser pass evaluated a numeral and an indexed literal
//! and NOTHING ELSE, so a global declared `भवति सत्यम्` took the ० that every
//! unevaluated initialiser takes: **true interpreted, false natively.** The
//! margin beside that pass has always said a global whose initialiser it cannot
//! evaluate "starts at ० and the reader sees ०, which is honest" — honest for a
//! computed initialiser, wrong for a literal the parser has already classified.
//!
//! Measured 2026-09-22, this fixture: interpreted `142`, natively `42` — the
//! `100` the boolean gates was gone. With the repair, natively `142`.
//!
//! **THE CORPUS HAS EXACTLY ONE SUCH GLOBAL** — `artha.t1`'s `पुच्छे` — and it is
//! ASSIGNED in `प्रकारपरीक्षकारम्भः` before anything reads it, which is why the
//! self-hosting fixpoint never saw this. That is luck, not safety: the second
//! one written would have been a silent false.
//!
//! `बूल` is a width-१ unsigned integer, so the value is १ or ०, and `व्याकर`
//! tells `सत्यम्` from `असत्यम्` by the token's TEXT — they are one expression
//! kind whose `मूल्यसूचकाङ्क` carries the word (`ast.t1:111`).

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

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

/// `सत्य` gates the १००; `असत्य` must NOT gate the ७. Both are read, so a pass
/// that answered ० for every boolean would score ४२ and one that answered १ for
/// every boolean would score २४९.
const SOURCE: &str = "मण्डलम् बूलपरीक्षा ॥
सार्वजनिक चरः सत्य ॱॱ बूल भवति सत्यम् ।
सार्वजनिक चरः असत्य ॱॱ बूल भवति असत्यम् ।
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः फलम् ॱॱ न६४ भवति ४२ ।
    यदि सत्य समम् सत्यम् आदि
        फलम् भवति फलम् योगः १०० ।
    इति
    यदि असत्य समम् सत्यम् आदि
        फलम् भवति फलम् योगः ७ ।
    इति
    प्रत्यागमनम् फलम् ।
इति
";

#[test]
fn a_boolean_global_carries_its_initialiser_into_the_image() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets("बूलपरीक्षा".as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(SOURCE.as_bytes())]),
                arena(vec![octets("बूलपरीक्षा".as_bytes())]),
                Value::Int(1),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!image.is_empty(), "the fixture built no image");

    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(200_000_000, &mut out);
    let status = match halt {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the fixture did not finish: {other:?}"),
    };
    assert_ne!(
        status, 42,
        "the `सत्यम्` global read FALSE in the image — the initialiser was never \
         evaluated, which is what this repair is about"
    );
    assert_ne!(
        status, 249,
        "the `असत्यम्` global read TRUE — a pass that answers १ for every boolean \
         is as wrong as one that answers ०"
    );
    assert_eq!(status, 142, "४२ + १०० from `सत्य`, and nothing from `असत्य`");
}
