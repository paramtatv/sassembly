//! **A FIELD READ IN ARGUMENT POSITION IS TYPED LIKE ANY OTHER.**
//!
//! `अर्थ`'s call arm answered the CALLEE's type and never visited an argument, so
//! every type error in argument position was invisible to the checker. The
//! field-membership gate — `असत्क्षेत्रसंख्या` and the pass's
//! `यदि असत्क्षेत्रसंख्या अधिकम् ०` — has existed since `W-240` and **never fired
//! on a real defect**, because the corpus's bad field reads all sat inside a
//! call's arguments:
//!
//! - `vakyavibhaga.t1:2978-2979` read `ॱ मूलारम्भ` and `ॱ मूलसीमा` from a
//!   `कारकपद`, whose declaration has three fields — `मूलपाठ`, `कारक`,
//!   `सङ्ख्यात्व`. Those two never existed in any version: commit `c1d64280`
//!   (2026-08-29) added the struct and the reads together. `ir.t1` planted
//!   constant ० for each, so the "conflicting karaka" diagnostic built there
//!   printed a slice from offset ० to ० for four weeks.
//!
//! A call is a LEFT-NESTED CHAIN — one argument on `दक्षिणसूचकाङ्क`, the next
//! call node on `वामसूचकाङ्क` — so typing this node's argument and recursing
//! into the callee reaches every argument exactly once.

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

/// Compile one source through the chain; answer the emitted text and the
/// interpreter, so a caller can read the refusal record.
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

/// The shape the corpus had: a field that is not in the struct, read INSIDE a
/// call's arguments. `लेखा` takes three, as `पाठांशयोजनम्` does.
const BAD_IN_ARGUMENT: &str = "मण्डलम् क्षेत्रपरीक्षा ॥
सार्वजनिक संरचना पद आरभ्य
  मूलपाठ ॱॱ अङ्कः अन्तः अ८ ऽ
  कारक ॱॱ अ६४
समाप्तम् ।
सार्वजनिक वृत्तिः लेखा आदाय प ॱॱ अङ्कः अन्तः अ८ ऽ आरम्भ ॱॱ अ६४ ऽ सीमा ॱॱ अ६४ ददाति अ६४ आदि
    प्रत्यागमनम् आरम्भ योगः सीमा ।
इति
सार्वजनिक वृत्तिः मुख्यम् आदाय एकम् ॱॱ पद ददाति अ६४ आदि
    चरः पदम् ॱॱ पद भवति एकम् ।
    लेखा आरभ्य पदम् ॱ मूलपाठ ऽ पदम् ॱ मूलारम्भ ऽ ० समाप्तम् ।
    प्रत्यागमनम् ० ।
इति
";

/// The control: every field exists, and the same call shape must compile.
const GOOD: &str = "मण्डलम् क्षेत्रनियन्त्रणम् ॥
सार्वजनिक संरचना पद आरभ्य
  मूलपाठ ॱॱ अङ्कः अन्तः अ८ ऽ
  कारक ॱॱ अ६४
समाप्तम् ।
सार्वजनिक वृत्तिः लेखा आदाय प ॱॱ अङ्कः अन्तः अ८ ऽ आरम्भ ॱॱ अ६४ ऽ सीमा ॱॱ अ६४ ददाति अ६४ आदि
    प्रत्यागमनम् आरम्भ योगः सीमा ।
इति
सार्वजनिक वृत्तिः मुख्यम् आदाय एकम् ॱॱ पद ददाति अ६४ आदि
    प्रत्यागमनम् लेखा आरभ्य एकम् ॱ मूलपाठ ऽ एकम् ॱ कारक ऽ ० समाप्तम् ।
इति
";

#[test]
fn a_field_that_is_not_in_the_struct_is_refused_from_inside_a_call() {
    let (text, it) = compile(BAD_IN_ARGUMENT, "क्षेत्रपरीक्षा");
    assert!(
        text.is_empty(),
        "the module emitted text although it reads a field the struct has not"
    );
    let site = chain::refusal_site(&it).unwrap_or_default();
    assert!(
        site.contains("मूलारम्भ") && site.contains("पद"),
        "the refusal must name the field and the struct; got {site:?}"
    );
}

#[test]
fn the_control_compiles_so_the_refusal_is_about_the_field_and_not_the_shape() {
    let (text, it) = compile(GOOD, "क्षेत्रनियन्त्रणम्");
    assert!(
        !text.is_empty(),
        "the same call shape with real fields must still compile; refusal: {:?}",
        chain::refusal_site(&it)
    );
}
