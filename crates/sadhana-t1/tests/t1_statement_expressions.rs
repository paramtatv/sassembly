//! **EVERY STATEMENT'S EXPRESSION IS TYPED — return, binding, and assignment.**
//!
//! `वाक्यप्रकारः` typed an expression-statement and the `यदि`/`यावत्` conditions,
//! and answered a CONSTANT for `प्रत्यागमनम्`, `चरः` and assignment without ever
//! visiting their expressions. Three of the eight statement kinds, so a type
//! error in a return value, an initialiser, or either side of an assignment was
//! invisible to the checker — the same shape as the call arm that typed the
//! callee and not the arguments (`8814a4cd`).
//!
//! **IT CAUGHT THE CORPUS'S FIFTH BAD FIELD READ ON ITS FIRST RUN.**
//! `vishlesana.t1:998` wrote `नव ॱ पङ्क्ति भवति ० ।` — an assignment TARGET, which
//! nothing had ever typed — where `सङ्केतनॱसङ्केत` has ten fields and none is
//! `पङ्क्ति`. It was a probe written to split two hypotheses, on the stated
//! premise that `पङ्क्ति` is "a scalar field of the same record"; it is not, so
//! `ir.t1` stubbed the write (cause ३९) and the probe tested nothing it claimed.
//! `vishlesana` fell from 2 stubs to 0 when it was removed.
//!
//! The four fixtures below are the four positions. Each is refused for the same
//! reason — a field the struct has not — and the control shows the same shapes
//! compile when the field exists, so the refusal is about the field.

use sadhana::t1::chain::{self, CHAIN};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

const FUEL: u64 = 80_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn compile(src: &str, module: &str) -> (Vec<u8>, Interpreter) {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets(module.as_bytes())],
            FUEL,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    (text, it)
}

/// `{BODY}` is the one line under test; everything around it is identical.
fn source(module: &str, body: &str) -> String {
    format!(
        "मण्डलम् {module} ॥
सार्वजनिक संरचना पद आरभ्य
  मूलपाठ ॱॱ अङ्कः अन्तः अ८ ऽ
  कारक ॱॱ अ६४
समाप्तम् ।
सार्वजनिक वृत्तिः मुख्यम् आदाय एकम् ॱॱ पद ददाति अ६४ आदि
    चरः पदम् ॱॱ पद भवति एकम् ।
{body}
    प्रत्यागमनम् ० ।
इति
"
    )
}

/// The four positions that were never typed, and the field none of them has.
const POSITIONS: &[(&str, &str)] = &[
    ("प्रत्यागमनम्", "    प्रत्यागमनम् पदम् ॱ मूलारम्भ ।"),
    ("चरः", "    चरः क ॱॱ अ६४ भवति पदम् ॱ मूलारम्भ ।"),
    ("assignment value", "    पदम् ॱ कारक भवति पदम् ॱ मूलारम्भ ।"),
    ("assignment target", "    पदम् ॱ मूलारम्भ भवति ० ।"),
];

#[test]
fn a_field_the_struct_has_not_is_refused_in_every_statement_position() {
    for (n, (what, body)) in POSITIONS.iter().enumerate() {
        let module = format!("वाक्यपरीक्षा{n}");
        let (text, it) = compile(&source(&module, body), &module);
        let site = chain::refusal_site(&it).unwrap_or_default();
        assert!(
            text.is_empty(),
            "{what}: the module compiled although it names a field `पद` has not"
        );
        assert!(
            site.contains("मूलारम्भ"),
            "{what}: the refusal must name the field; got {site:?}"
        );
    }
}

/// The control: the same four shapes, every field real, must still compile —
/// otherwise the refusals above are about the shape and not the field.
#[test]
fn the_same_shapes_compile_when_the_field_exists() {
    let bodies = [
        "    प्रत्यागमनम् पदम् ॱ कारक ।",
        "    चरः क ॱॱ अ६४ भवति पदम् ॱ कारक ।",
        "    पदम् ॱ कारक भवति पदम् ॱ कारक ।",
        "    पदम् ॱ कारक भवति ० ।",
    ];
    for (n, body) in bodies.iter().enumerate() {
        let module = format!("वाक्यनियन्त्रणम्{n}");
        let (text, it) = compile(&source(&module, body), &module);
        assert!(
            !text.is_empty(),
            "the control at {n} must compile; refusal: {:?}",
            chain::refusal_site(&it)
        );
    }
}
