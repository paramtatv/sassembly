//! **`शून्यम्` AND `ऋण` HAVE TYPES — the last two expression kinds that had none.**
//!
//! Of the thirteen expression kinds, `शून्याभिव्यञ्जकभेद` (१०) and
//! `ऋणाभिव्यञ्जकभेद` (१२) had no arm in `अभिव्यञ्जकप्रकारः`, so both typed as the
//! poison.
//!
//! **NO STUB EVER FIRED FOR EITHER**, which is exactly why they survived: `ir.t1`
//! lowers both correctly — `शून्यम्` to a constant ०, `ऋण x` to `० − x` — so the
//! corpus compiled and ran. The hole was in the CHECKER, and a poison is a hole
//! the next reader of a type falls into. This corpus paid for that twice in one
//! week: the index arm (2026-09-14) and the slice arm (2026-09-22) were each a
//! missing arm in this same routine, and each cost a fixpoint round when
//! something downstream read the poison and lowered a word compare.
//!
//! - `शून्यम्` answers `शून्यार्थः` (Ty::Void).
//! - `ऋण x` answers `x`'s own type and REQUIRES a number: it lowers to `० − x`,
//!   so a run or a record operand is meaningless. The corpus writes this
//!   production exactly TWICE — `encode.t1:1258` and `:1277`, both `ऋण सीमा`
//!   over a numeric — so the refusal costs it nothing. Every other `ऋण` in the
//!   corpus is the literal TEXT inside `उक्तम् ऋण इति`, a different token
//!   (`ast.t1:135`: the sign is inside a numeral's token, and `ऋण सीमा` is two).

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

/// `ऋण` over a RUN — meaningless, because the lowering is `० − x`.
const NEGATES_A_RUN: &str = "मण्डलम् ऋणपरीक्षा ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि
    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् कख इति ।
    प्रत्यागमनम् ऋण प ।
इति
";

/// The control: `ऋण` over a number, which is what the corpus writes twice.
const NEGATES_A_NUMBER: &str = "मण्डलम् ऋणनियन्त्रणम् ॥
सार्वजनिक वृत्तिः मुख्यम् आदाय सीमा ॱॱ अ६४ ददाति अ६४ आदि
    प्रत्यागमनम् ऋण सीमा ।
इति
";

/// `शून्यम्` in the position the corpus uses it: compared against an optional.
const USES_VOID: &str = "मण्डलम् शून्यपरीक्षा ॥
सार्वजनिक वृत्तिः मुख्यम् आदाय क ॱॱ सम्भाव्य न६४ ददाति न६४ आदि
    यदि क समम् शून्यम् आदि
        प्रत्यागमनम् ७ ।
    इति
    प्रत्यागमनम् ० ।
इति
";

#[test]
fn negating_something_that_is_not_a_number_is_refused() {
    let (text, it) = compile(NEGATES_A_RUN, "ऋणपरीक्षा");
    let site = chain::refusal_site(&it).unwrap_or_default();
    assert!(
        text.is_empty(),
        "`ऋण` over a run compiled; it lowers to `० − x` and means nothing"
    );
    assert!(
        site.contains("ऋण"),
        "the refusal must name the operator; got {site:?}"
    );
}

#[test]
fn negating_a_number_still_compiles() {
    let (text, it) = compile(NEGATES_A_NUMBER, "ऋणनियन्त्रणम्");
    assert!(
        !text.is_empty(),
        "the corpus's own shape (`ऋण सीमा`) must compile; refusal: {:?}",
        chain::refusal_site(&it)
    );
}

#[test]
fn void_compares_against_an_optional_and_compiles() {
    let (text, it) = compile(USES_VOID, "शून्यपरीक्षा");
    assert!(
        !text.is_empty(),
        "`समम् शून्यम्` is how the corpus tests an optional; refusal: {:?}",
        chain::refusal_site(&it)
    );
}
