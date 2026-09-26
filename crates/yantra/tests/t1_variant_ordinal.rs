//! **A `गणना` VARIANT IS ITS ORDINAL WITHIN ITS OWN `गणना` — across modules.**
//!
//! `व्याकर`'s declaration kinds run १..६; `७` is the kind `घोषणासञ्चय` gives a
//! VARIANT (`sanchaya.t1`'s `भेदप्रविष्टिभेद`), and `ir.t1` answered
//! `अपूर्णध्रुवम् ७` for it — a stub, whose value is ०. So **every cross-module
//! variant reference compiled to ०**, which makes all of an enum's variants
//! indistinguishable from its first.
//!
//! **THE CORPUS COULD NOT SEE IT.** All eighteen variant references in
//! `crates/sadhana-t1/src` are to `सङ्केतनॱअसङ्कुचितम्`, the `लक्ष्य` enum's FIRST
//! variant, whose ordinal is ० — the same value the stub emitted. Sixteen of
//! them were stubs and the emitted object did not change by one octet when they
//! stopped being stubs (`shrinkhala` text 1333159, object 217272, before and
//! after). A green corpus proves nothing here, which is why this fixture reads
//! the SECOND variant, where ० and १ differ.
//!
//! Measured 2026-09-23, this fixture through `t1_image` and `yantra-run`:
//! before the repair `status 0`, after it `status 1`.
//!
//! **TYPE-DIRECTED, NOT A BARE DECLARATION INDEX.** The lowering finds the
//! `गणना` entry in the variant's OWN module that lists the name, and takes the
//! position in THAT list. An ordinal read from anywhere else would put two
//! different enums' first variants on the same ०, which is the `Some(०)` shape
//! this corpus has paid for five times.

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

/// The enum lives in one module and is read from another, which is the only
/// shape that reaches kind ७: a variant reached through the collected registry.
const DECLARES: &str = "मण्डलम् गणनाकोश ॥
सार्वजनिक गणना लक्ष्यम् आरभ्य
  प्रथमम् ऽ
  द्वितीयम् ऽ
  तृतीयम्
समाप्तम् ।
";

const READS: &str = "मण्डलम् गणनापाठकः ॥
आयातः गणनाकोश ।
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    प्रत्यागमनम् गणनाकोशॱद्वितीयम् ।
इति
";

#[test]
fn a_cross_module_variant_lowers_to_its_ordinal_and_not_to_zero() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets("गणनापाठकः".as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(DECLARES.as_bytes()), octets(READS.as_bytes())]),
                arena(vec![
                    octets("गणनाकोश".as_bytes()),
                    octets("गणनापाठकः".as_bytes()),
                ]),
                Value::Int(2),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!image.is_empty(), "the two-module fixture built no image");

    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(200_000_000, &mut out);
    let status = match halt {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the fixture did not finish: {other:?}"),
    };
    assert_eq!(
        status, 1,
        "`गणनाकोशॱद्वितीयम्` is the SECOND variant, ordinal १. Status ० is the \
         stub `अपूर्णध्रुवम् ७` every cross-module variant used to lower to, and \
         it makes every variant of every enum read as its first"
    );
}
