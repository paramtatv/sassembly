//! **W-345 — `पठनम्` ANSWERS ० FOR TWO CAUSES, AND THE RECORD MUST TELL THEM
//! APART.** A source with an octet outside R-15-1 and a source that parses to
//! no declarations both come back from `पठनम्` as ०, and `मण्डलसङ्कलनम्` read
//! only the count: both exits recorded `सङ्कलनाघोषणाभेद` (१, "declared
//! nothing"). The consequences were measured, not hypothetical — the corpus
//! walk counted a repertoire-refused source into `सङ्कलनरिक्तसंख्या` ("not a
//! failure") and never wrote its name into `विफलनामानि`, and `t1_boot
//! --object` printed "declares nothing — no object, and not a refusal" and
//! exited SUCCESS over a source it had refused.
//!
//! The discriminator already existed: W-304's gate records
//! `परिधिदोषस्थितम्`/`परिधिदोषस्थानम्` (sanskrit_text.t1:871), reset on entry
//! to `परिधिपदविभाग`, and `chain::refusal_site` reads them. What was missing
//! is the `.t1` side's own exit code: `सङ्कलनपरिधिभेद` (८), set when the gate's
//! बूल stands, so a reader of module state alone — the self-hosted drivers —
//! can name the cause without the Rust renderer.

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

fn compile(it: &mut Interpreter, module: &str, src: &str) -> Vec<u8> {
    it.call(
        "शृङ्खलाॱमण्डलसङ्कलनम्",
        vec![octets(src.as_bytes()), octets(module.as_bytes())],
        FUEL,
    )
    .expect("मण्डलसङ्कलनम् runs")
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default()
}

/// One Latin `x` in the routine's name; every other octet is in R-15-1, so
/// that single code point is what refuses this source.
const LATIN: &str = "मण्डलम् परिधिपरीक्षा ॥
सार्वजनिक वृत्तिः मुख्यx ददाति न६४ आदि
    प्रत्यागमनम् २१ ।
इति
";

/// `lib.t1`'s shape: margins only, no `मण्डलम्`, no declaration — the cause
/// that must KEEP answering १, or the discriminator broke the case it was
/// built beside.
const NOTHING: &str = "॰ a file that declares nothing, as `lib.t1` does\n";

#[test]
fn a_repertoire_refusal_records_its_own_kind_not_no_declarations() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = compile(&mut it, "परिधिपरीक्षा", LATIN);
    assert!(text.is_empty(), "a source outside R-15-1 emits no text");
    assert_eq!(
        int(&it, "सङ्कलनविरामभेद"),
        8,
        "`सङ्कलनपरिधिभेद` — an octet outside R-15-1, NOT `१` (declared nothing): \
         the two causes behind `पठनम्`'s ० must be told apart from the record"
    );
    // The site, not just the kind: the gate's octet is standing and non-zero
    // (the foreign octet is never at offset ० — `मण्डलम्` precedes it).
    assert!(
        matches!(it.global("परिधिदोषस्थितम्"), Some(Value::Bool(true))),
        "the gate's record stands beside the exit code"
    );
    assert!(int(&it, "परिधिदोषस्थानम्") > 0, "…and names the octet");
}

/// **THE CASE THAT MUST STILL BE REFUSED: `१` MUST STILL MEAN "DECLARED
/// NOTHING".** Same interpreter, immediately after the repertoire refusal —
/// the only order that can catch a stale gate record. `परिधिपदविभाग` resets
/// `परिधिदोषस्थितम्` on entry; if it stopped, every later empty source would
/// read ८ and `lib.t1` would be a "repertoire refusal" in every build.
#[test]
fn a_declaration_less_source_after_a_repertoire_refusal_still_reads_one() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    assert!(
        compile(&mut it, "परिधिपरीक्षा", LATIN).is_empty(),
        "the refusing module compiles FIRST, so the empty one runs over its record"
    );
    assert_eq!(int(&it, "सङ्कलनविरामभेद"), 8, "…and that record exists");
    assert!(compile(&mut it, "रिक्त", NOTHING).is_empty());
    assert_eq!(
        int(&it, "सङ्कलनविरामभेद"),
        1,
        "`सङ्कलनाघोषणाभेद` — the repertoire record was left standing if this reads ८"
    );
}

/// **THE CORPUS WALK: A REPERTOIRE-REFUSED SOURCE IS A FAILURE WITH A NAME,
/// NOT AN EMPTY SOURCE.** Before the discriminator, `मण्डलानिप्रतिबिम्बम्`
/// filed it under `सङ्कलनरिक्तसंख्या` and `विफलनामानि` never carried it — a
/// refused source dropped in silence, the exact shape those counters were
/// built against.
#[test]
fn the_image_driver_counts_a_repertoire_refusal_as_a_failure() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let _ = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(LATIN.as_bytes()), octets(NOTHING.as_bytes())]),
                arena(vec![
                    octets("परिधिपरीक्षा".as_bytes()),
                    octets("रिक्त".as_bytes()),
                ]),
                Value::Int(2),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs");
    assert_eq!(
        int(&it, "सङ्कलनविफलसंख्या"),
        1,
        "the repertoire-refused source IS a failure — before W-345 this read ०"
    );
    assert_eq!(
        int(&it, "सङ्कलनरिक्तसंख्या"),
        1,
        "and the declaration-less source beside it is still counted apart"
    );
}
