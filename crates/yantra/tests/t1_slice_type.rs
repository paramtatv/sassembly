//! **A SLICE HAS ITS BASE'S TYPE — asked of the compiled program, not the source.**
//!
//! `artha.t1`'s `अभिव्यञ्जकप्रकारः` had no arm for `खण्डाभिव्यञ्जकभेद`, so a
//! slice typed as the poison. `ir.t1`'s equality arm asks that routine for each
//! operand and compares OCTETS only when both are runs; otherwise it compares
//! the two WORDS. So `आरभ्य नाम अङ्कः ० अन्तः क समाप्तम् समम् उपसर्गः` compared
//! two pointers.
//!
//! **THE INTERPRETER EXECUTING A SOURCE HIDES IT**: it compares octets whatever
//! the checker thought. The compiled compiler does not, and the one place it
//! mattered was `प्रकारपाठारम्भः` — so, natively, no compound type in the collected
//! registry (`अङ्कः अन्तः …`, `सम्भाव्य …`, `स्थानम् …`) ever parsed, every read
//! of a cross-module global of such a type became a constant stub, and the
//! native self-image build lost ~650 lines of `artha.t1` and then faulted on a
//! wild pointer (2026-09-22).
//!
//! **THE CONTROL IS IN THE PROGRAM.** The same slice bound to a local first takes
//! its type from the declaration, which always worked. The status is
//! `10 × control + subject`, so `11` is the fix, `10` is the defect, and anything
//! else means the control itself broke and this test is not measuring slices.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

const SOURCE: &str = "मण्डलम् खण्डपरीक्षा ॥
सार्वजनिक वृत्तिः बद्धसाम्यम् आदाय नाम ॱॱ अङ्कः अन्तः अ८ ऽ उपसर्गः ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि
    चरः खण्डः ॱॱ अङ्कः अन्तः अ८ भवति नाम अङ्कः ० अन्तः उपसर्गः ॱ दैर्घ्य ।
    प्रत्यागमनम् खण्डः समम् उपसर्गः ।
इति
सार्वजनिक वृत्तिः खण्डसाम्यम् आदाय नाम ॱॱ अङ्कः अन्तः अ८ ऽ उपसर्गः ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि
    प्रत्यागमनम् आरभ्य नाम अङ्कः ० अन्तः उपसर्गः ॱ दैर्घ्य समाप्तम् समम् उपसर्गः ।
इति
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः फलम् ॱॱ न६४ भवति ० ।
    यदि बद्धसाम्यम् आरभ्य उक्तम् अङ्कः अन्तः न६४ इति ऽ उक्तम् अङ्कः अन्तः इति समाप्तम् आदि
        फलम् भवति १० ।
    इति
    यदि खण्डसाम्यम् आरभ्य उक्तम् अङ्कः अन्तः न६४ इति ऽ उक्तम् अङ्कः अन्तः इति समाप्तम् आदि
        फलम् भवति फलम् योगः १ ।
    इति
    प्रत्यागमनम् फलम् ।
इति
";

#[test]
fn a_slice_compared_directly_compares_octets_in_the_compiled_program() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let arena = |vs: Vec<Value>| Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)));
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets("खण्डपरीक्षा".as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(SOURCE.as_bytes())]),
                arena(vec![octets("खण्डपरीक्षा".as_bytes())]),
                Value::Int(1),
            ],
            80_000_000_000,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(
        !image.is_empty(),
        "the chain answered no image for the fixture"
    );

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
        status / 10,
        0,
        "the CONTROL failed — a slice bound to a local no longer compares equal \
         to its prefix, so this test is not measuring the slice arm (status {status})"
    );
    assert_eq!(
        status, 11,
        "a slice compared directly lowered as a WORD compare: `अभिव्यञ्जकप्रकारः` \
         has no type for `खण्डाभिव्यञ्जकभेद` (status {status}; 10 is the defect)"
    );
}
