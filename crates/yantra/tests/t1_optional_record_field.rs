//! **A FIELD OF AN OPTIONAL RECORD IS A FIELD OF THE RECORD — in the compiled program.**
//!
//! `सम्भाव्य R` is one word natively: R's pointer, ० for absent. Neither
//! `artha.t1`'s typer nor `ir.t1`'s field arm unwrapped it, so a field read
//! through `सम्भाव्य` typed as the poison and lowered as a constant ०. The
//! interpreter reads the field regardless, so only the compiled compiler showed
//! it: the linker's decoded instruction name (`विश्लिष्टः ॱ आज्ञा`) was empty
//! natively, every relocation patch refused "no 32-bit encoding", and the native
//! self-image build answered no image (2026-09-22).
//!
//! **THE CONTROL IS IN THE PROGRAM**: the same record read without `सम्भाव्य`.
//! Status `100 × control + subject`, both `97` (9 octets × 10 + 7) when right:
//! `9797` is the fix, `9700` the defect, anything under `9700` a broken control.

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

const SOURCE: &str = "मण्डलम् वैकल्पिकपरीक्षा ॥
सार्वजनिक संरचना अभि आरभ्य
  आज्ञा ॱॱ पाठ ऽ
  कुल ॱॱ न६४
समाप्तम् ।
सार्वजनिक वृत्तिः रचय ददाति सम्भाव्य अभि आदि
    चरः नव ॱॱ अभि भवति ० ।
    नव ॱ आज्ञा भवति उक्तम् कखग इति ।
    नव ॱ कुल भवति ७ ।
    प्रत्यागमनम् नव ।
इति
सार्वजनिक वृत्तिः रचयप ददाति अभि आदि
    चरः नव ॱॱ अभि भवति ० ।
    नव ॱ आज्ञा भवति उक्तम् कखग इति ।
    नव ॱ कुल भवति ७ ।
    प्रत्यागमनम् नव ।
इति
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः व ॱॱ सम्भाव्य अभि भवति रचय ।
    यदि व समम् शून्यम् आदि
        प्रत्यागमनम् ९९ ।
    इति
    चरः प ॱॱ अभि भवति रचयप ।
    चरः फ ॱॱ न६४ भवति प ॱ आज्ञा ॱ दैर्घ्य गुणनम् १० योगः प ॱ कुल ।
    चरः ग ॱॱ न६४ भवति व ॱ आज्ञा ॱ दैर्घ्य गुणनम् १० योगः व ॱ कुल ।
    प्रत्यागमनम् फ गुणनम् १०० योगः ग ।
इति
";

#[test]
fn a_field_read_through_an_optional_record_reads_the_field_in_the_compiled_program() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let arena = |vs: Vec<Value>| Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)));
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets("वैकल्पिकपरीक्षा".as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(SOURCE.as_bytes())]),
                arena(vec![octets("वैकल्पिकपरीक्षा".as_bytes())]),
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
    assert!(
        status >= 9700,
        "the CONTROL failed — a plain record's fields no longer read 97, so this \
         test is not measuring the optional unwrap (status {status})"
    );
    assert_eq!(
        status, 9797,
        "a field read through `सम्भाव्य` lowered as a constant ०: the field arm does \
         not unwrap an optional record (status {status}; 9700 is the defect)"
    );
}
