//! `W-267`, the refusal half: a `भवति ०` declaration whose type names nothing the
//! language or the program declares is REFUSED BY NAME, not silently bound to
//! `Value::Int(0)` — while every type the corpus really writes still zeroes.
//!
//! What zeroes, and to what (`nirvahana.rs` `zero_at`):
//! - a grammar scalar (`grammar-t1.ebnf:226`, `:248`) — `Int(0)`, as before;
//! - `पाठ`, and the retired spelling `पाठः` the corpus still writes 14 times — the
//!   EMPTY TEXT, as `अङ्कः अन्तः अ८` already is, not `Int(0)`;
//! - a declared `संरचना` — a record, as before;
//! - a declared `गणना` — `Int(0)`, its FIRST variant (ordinals are zero-based by
//!   position, `nirvahana.rs` where `enum_variants` are pushed). The row's first
//!   prescription left enums out; three are written in type position in the corpus,
//!   and each is checked below with its declaration copied from the source.
//!
//! RED ON origin/main: the refusal tests (an undeclared name answers `Int(0)` there) and
//! the text tests (`ॱ दैर्घ्य` of `Int(0)` is not a length). GREEN ON main BY DESIGN: the
//! enum and record controls, which guard against an allowlist that forgets a kind.

use sadhana::t1::nirvahana::{Interpreter, RunError, Value};
use std::path::Path;

fn call(src: &str, entry: &str) -> Result<Value, RunError> {
    let mut it = Interpreter::load(&[("test", src)], Path::new("."))
        .unwrap_or_else(|e| panic!("`{entry}` must load: {}", e.reason));
    it.call(entry, Vec::new(), 1_000_000)
}

fn int(src: &str, entry: &str) -> i128 {
    let v = call(src, entry).unwrap_or_else(|e| panic!("`{entry}` must run: {}", e.reason));
    v.as_int()
        .unwrap_or_else(|| panic!("`{entry}` must answer a number, not {v:?}"))
}

fn refused(src: &str, entry: &str, name: &str) {
    match call(src, entry) {
        Err(e) => assert!(
            e.reason.contains(name),
            "the refusal must name `{name}`; it said: {}",
            e.reason
        ),
        Ok(v) => panic!(
            "W-267: a `भवति ०` of the undeclared type `{name}` must be REFUSED, \
             not bound to {v:?}"
        ),
    }
}

#[test]
fn an_undeclared_type_in_a_local_is_refused_by_name() {
    let src = "मण्डलम् परीक्षा ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि
    चरः क ॱॱ अघोषितम् भवति ० ।
    प्रत्यागमनम् ० ।
इति
";
    refused(src, "मुख्यम्", "अघोषितम्");
}

#[test]
fn an_undeclared_type_in_a_record_field_is_refused_by_name() {
    let src = "मण्डलम् परीक्षा ॥

संरचना अभिलेखः आरभ्य
  सङ्ख्या ॱॱ न६४ ऽ
  रहस्यम् ॱॱ अघोषितम्
समाप्तम् ।

सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि
    चरः नव ॱॱ अभिलेखः भवति ० ।
    प्रत्यागमनम् ० ।
इति
";
    refused(src, "मुख्यम्", "अघोषितम्");
}

#[test]
fn text_zeroes_to_the_empty_text_in_both_spellings() {
    for ty in ["पाठ", "पाठः"] {
        let src = format!(
            "मण्डलम् परीक्षा ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि
    चरः क ॱॱ {ty} भवति ० ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
"
        );
        assert_eq!(int(&src, "मुख्यम्"), 0, "`{ty}` must zero to the EMPTY TEXT");
    }
}

/// `vastu.t1:27-34` and `:36-42`, margins removed: the struct the object reader zeroes
/// for every symbol (`सङ्केतनॱवस्तुसंज्ञासारणी`), with a `पाठः` field and a `स्थापन` one.
const VASTU: &str = "मण्डलम् परीक्षा ॥

सार्वजनिक गणना स्थापन आरभ्य
  पाठ्यम् ऽ
  दत्तम् ऽ
  शून्यक्षेत्रम् ऽ
  शोधनपङ्क्तिः ऽ
  अनिर्दिष्टम् ऽ
  अन्यत्
समाप्तम् ।

सार्वजनिक संरचना वस्तुसंज्ञा आरभ्य
  नाम ॱॱ पाठः ऽ
  मूल्यम् ॱॱ न६४ ऽ
  खण्डाङ्कः ॱॱ अ१६ ऽ
  स्थापनम् ॱॱ स्थापन ऽ
  वैश्विकम् ॱॱ बूल
समाप्तम् ।

सार्वजनिक वृत्तिः नामदैर्घ्यम् ददाति अ६४ आदि
    चरः नव ॱॱ वस्तुसंज्ञा भवति ० ।
    प्रत्यागमनम् नव ॱ नाम ॱ दैर्घ्य ।
इति

सार्वजनिक वृत्तिः स्थापनप्रथमम् ददाति अ६४ आदि
    चरः नव ॱॱ वस्तुसंज्ञा भवति ० ।
    यदि नव ॱ स्थापनम् समम् पाठ्यम् आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः मूल्यशून्यम् ददाति अ६४ आदि
    चरः नव ॱॱ वस्तुसंज्ञा भवति ० ।
    प्रत्यागमनम् नव ॱ मूल्यम् ।
इति
";

#[test]
fn vastus_symbol_record_zeroes_its_text_field_to_the_empty_text() {
    assert_eq!(int(VASTU, "नामदैर्घ्यम्"), 0);
}

#[test]
fn vastus_symbol_record_zeroes_its_sthapana_field_to_the_first_variant() {
    assert_eq!(int(VASTU, "स्थापनप्रथमम्"), 1, "स्थापनम् must zero to पाठ्यम्");
}

#[test]
fn a_declared_record_still_zeroes_its_scalar_fields_as_before() {
    assert_eq!(int(VASTU, "मूल्यशून्यम्"), 0);
}

/// One enum per program, because `स्थापन` and `संज्ञाखण्ड` share variant names.
fn enum_zero_is_first_variant(decl: &str, ty: &str, first: &str) {
    let src = format!(
        "मण्डलम् परीक्षा ॥

{decl}
सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि
    चरः क ॱॱ {ty} भवति ० ।
    यदि क समम् {first} आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
"
    );
    assert_eq!(
        int(&src, "मुख्यम्"),
        1,
        "`{ty}` must zero to its first variant `{first}`"
    );
}

#[test]
fn the_enum_sthapana_zeroes_to_its_first_variant() {
    // vastu.t1:27-34
    enum_zero_is_first_variant(
        "सार्वजनिक गणना स्थापन आरभ्य\n  पाठ्यम् ऽ\n  दत्तम् ऽ\n  शून्यक्षेत्रम् ऽ\n  शोधनपङ्क्तिः ऽ\n  अनिर्दिष्टम् ऽ\n  अन्यत्\nसमाप्तम् ।\n",
        "स्थापन",
        "पाठ्यम्",
    );
}

#[test]
fn the_enum_sanjnakhanda_zeroes_to_its_first_variant() {
    // kosha.t1:20-26
    enum_zero_is_first_variant(
        "सार्वजनिक गणना संज्ञाखण्ड आरभ्य\n  पाठ्यम् ऽ\n  दत्तम् ऽ\n  शून्यक्षेत्रम् ऽ\n  अनिर्दिष्टम् ऽ\n  शोधनपङ्क्तिखण्डः ऽ\n  पाठ्यखण्डः\nसमाप्तम् ।\n",
        "संज्ञाखण्ड",
        "पाठ्यम्",
    );
}

#[test]
fn the_enum_lakshya_zeroes_to_its_first_variant() {
    // encode.t1:919-922
    enum_zero_is_first_variant(
        "सार्वजनिक गणना लक्ष्य आरभ्य\n  असङ्कुचितम् ऽ\n  सङ्कुचितम्\nसमाप्तम् ।\n",
        "लक्ष्य",
        "असङ्कुचितम्",
    );
}

#[test]
fn every_grammar_scalar_still_zeroes_to_zero() {
    // grammar-t1.ebnf:226 and :248 — including `अक्षरम्`, which begins with `अ` and so
    // must not be read as a malformed integer width.
    for ty in [
        "अ८",
        "अ१६",
        "अ३२",
        "अ६४",
        "अ१२८",
        "न८",
        "न६४",
        "प३२",
        "प६४",
        "बूल",
        "अक्षरम्",
    ] {
        let src = format!(
            "मण्डलम् परीक्षा ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि
    चरः क ॱॱ {ty} भवति ० ।
    प्रत्यागमनम् ० ।
इति
"
        );
        assert!(
            call(&src, "मुख्यम्").is_ok(),
            "`{ty}` is a grammar scalar and must zero"
        );
    }
}
