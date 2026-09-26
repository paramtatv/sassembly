//! **A FIELD READ WHOSE RECORD CANNOT BE RESOLVED IS COUNTED, AND SAYS WHICH WAY.**
//!
//! `अर्थ`'s field arm has FOUR exits that answer the poison before either walk
//! begins:
//!
//! | exit | condition | whose defect |
//! |---|---|---|
//! | १ | the base does not type as a record | the program's |
//! | २ | the record type carries no symbol | this checker's tables |
//! | ३ | the symbol table is shorter than that symbol | this checker's tables |
//! | ४ | `घोषणासञ्चय` holds no entry for that record | the registry's |
//!
//! All four were SILENT: a field read that took one compiled, and measured
//! 2026-09-23 `क ॱ क्षेत्रम्` where `क` is an `अ६४` emitted an object (text 1440,
//! object 904) with `ir.t1` planting a stub for the read. Nothing counted them,
//! so no instrument could say how often the checker gives up.
//!
//! **THEY COUNT AND THEY DO NOT REFUSE, AND THAT TOOK THREE MEASUREMENTS.**
//!
//! 1. A first version refused all four, on a census through `t1_boot` reading
//!    ZERO firings over the 21 sources. `t1_boot` loads the whole source
//!    directory first; `t1_corpus_conditionals` and `t1_corpus_globals` compile
//!    each source ALONE. There, **13 sources stopped emitting objects.**
//! 2. Restricting to exit १ did not save it: all 13 read base kind १२
//!    (`दोषार्थभेद`), the poison the routine returns when it has already failed
//!    upstream — so the refusal named where the poison was next READ, not the
//!    defect.
//! 3. Skipping a poison base left THREE, now base kind १, and all three are
//!    cross-module global reads (`वास्तुॱअभिव्यञ्जककोश ॱ दैर्घ्य` at artha:3055,
//!    `वाक्यविभागॱवाक्यविभागआज्ञाकोश ॱ दैर्घ्य` at encode:6730, and
//!    `सङ्केतनॱअन्तिमसङ्केतनदोषः ॱ पङ्क्ति` at shrinkhala:2106). **An unresolved
//!    cross-module global types as a plain integer — the same kind a genuine
//!    `७ ॱ क्षेत्रम्` gives.**
//!
//! The checker cannot tell those apart, so no refusal on this signal is sound
//! in every configuration; one that fired only when a collect pass had run
//! would be a guard an ordinary build removes. Counting is sound everywhere,
//! and it is the part that was actually missing.
//!
//! So this file asserts the COUNTER, not a refusal — including that the control
//! counts ZERO, because a counter that fired on everything would measure as
//! little as the silence it replaced.

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

/// The compiled text comes back beside the interpreter so a caller can say
/// "it still compiled AND it was counted" — the two halves of the claim.
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

fn int(it: &Interpreter, name: &str) -> i128 {
    it.global(name).and_then(Value::as_int).unwrap_or(0)
}

fn text_of(it: &Interpreter, name: &str) -> String {
    match it.global(name) {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        _ => String::new(),
    }
}

/// Exit १: the base is an integer, so it has no fields at all.
const FIELD_ON_A_NUMBER: &str = "मण्डलम् धारपरीक्षा ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि
    चरः क ॱॱ अ६४ भवति ७ ।
    प्रत्यागमनम् क ॱ क्षेत्रम् ।
इति
";

/// The control: a real record with that field, which must compile AND count ०.
const FIELD_ON_A_RECORD: &str = "मण्डलम् धारनियन्त्रणम् ॥
सार्वजनिक संरचना पद आरभ्य
  क्षेत्रम् ॱॱ अ६४
समाप्तम् ।
सार्वजनिक वृत्तिः मुख्यम् आदाय एकम् ॱॱ पद ददाति अ६४ आदि
    प्रत्यागमनम् एकम् ॱ क्षेत्रम् ।
इति
";

#[test]
fn a_field_read_on_a_non_record_is_counted_and_names_the_exit() {
    let (_text, it) = compile(FIELD_ON_A_NUMBER, "धारपरीक्षा");
    assert!(
        int(&it, "अनिर्णीतधारसंख्या") > 0,
        "the field arm answered the poison and nothing counted it — the silent \
         exit is back"
    );
    assert!(
        matches!(it.global("अनिर्णीतधारमस्ति"), Some(Value::Bool(true))),
        "the counter moved but no site was recorded, so no reader can say WHERE"
    );
    assert_eq!(
        int(&it, "अनिर्णीतधारकूट"),
        1,
        "the base is an `अ६४`, which is exit १ — the four want different repairs \
         and a reader who cannot tell them apart is sent to the wrong one"
    );
    assert!(
        text_of(&it, "अनिर्णीतधारनाम").contains("क्षेत्रम्"),
        "the record must name the field; got {:?}",
        text_of(&it, "अनिर्णीतधारनाम")
    );
}

#[test]
fn the_same_read_on_a_real_record_compiles_and_counts_nothing() {
    let (text, it) = compile(FIELD_ON_A_RECORD, "धारनियन्त्रणम्");
    assert!(
        !text.is_empty(),
        "a record's own field must still compile — this counter is not allowed \
         to cost the corpus an object"
    );
    assert_eq!(
        int(&it, "अनिर्णीतधारसंख्या"),
        0,
        "a resolvable field read was counted as unresolved; a counter that \
         fires on everything measures nothing"
    );
}
