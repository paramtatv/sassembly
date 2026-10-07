//! **W-333 — `दक्षिणसृ` OVER A NAME DECLARED UNSIGNED IS A LOGICAL SHIFT, ON
//! BOTH ENGINES, AND OVER ANYTHING ELSE IT IS THE ARITHMETIC SHIFT IT WAS.**
//!
//! EVERY ASSERTION HERE IS AN ABSOLUTE VALUE, NEVER "THE TWO ENGINES AGREE".
//! That is the row's own finding: before the fix both engines answered
//! all-ones for `क दक्षिणसृ ६३` over `क ॱॱ न६४ भवति ऋण१` and AGREED, so every
//! twin comparison the project owns was blind to it by construction. Two
//! kernels in two other repositories found it with absolute checks (the archive project
//! `ea74ec28`, sravan `a2bd6664`); those two witnesses are the first test.
//!
//! MEASURED ON `b7e3e8c6` BEFORE ANY EDIT, same fixtures, both engines
//! (१ = the wanted value, ३ = all-ones, written `interpreted/native`):
//!
//! | fixture                                   | before | now |
//! |-------------------------------------------|--------|-----|
//! | `न६४ = ऋण१`, shifted by ६३                 | ३/३    | १/१ |
//! | `न६४ = ऋण३`, shifted by ३२                 | ३/३    | १/१ |
//! | `न६४ = 2^63`, shifted by ६३                | १/३    | १/१ |
//! | `अ६४ = ऋण८`, shifted by १ (wants `ऋण४`)    | १/१    | १/१ |
//! | the literal `ऋण१`, shifted by ६३           | ३/३    | ३/३ |
//! | parameter, own global, qualified, grouped | ३/३    | १/१ |
//! | inner `न६४` shadowing an outer `अ६४`       | ३/३    | १/१ |
//! | inner `अ६४` shadowing an outer `न६४`       | १/१    | १/१ |
//!
//! THE THIRD ROW IS THE ONE PLACE THE ENGINES DISAGREED BEFORE: the
//! interpreter keeps a `न६४` with bit ६३ set as a POSITIVE i128, so its
//! arithmetic shift was already right there, and the native `sra` was not.
//!
//! THE RULE IS ABOUT A NAME, AND THE CONTROLS SAY SO. A literal left operand
//! stays arithmetic on both engines; a signed name sign-extends. A fix that
//! made every right shift logical would pass the first three tests and fail
//! the fourth — and would break the corpus's five unmasked shifts, all of
//! which have a signed left operand (the hi/lo splits of the relocation
//! writer among them).
//!
//! ONE LIMIT, STATED SO IT IS NOT MISTAKEN FOR COVERAGE: the shadowing probes
//! read the shadowed name ONLY INSIDE the block that shadows it. After that
//! block closes the native engine returns the inner value (`W-364`, a slot
//! defect unrelated to shifts), so a read placed there would fail for that
//! row's reason and say nothing about this one.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

const FUEL: u64 = 80_000_000_000;

/// The wanted value was computed.
const WANTED: u64 = 1;
/// All-ones: the arithmetic shift of a value with bit ६३ set.
const ALL_ONES: u64 = 3;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// `मुख्यम्` of `सरणपरीक्षा` as the INTERPRETER answers it: the fixture's
/// sources loaded as a program of their own.
fn interpreted(sources: &[(&str, &str)]) -> u64 {
    let mut it = Interpreter::load(sources, &spec_root()).expect("the fixture loads");
    match it.call("सरणपरीक्षाॱमुख्यम्", vec![], FUEL) {
        Ok(Value::Int(n)) => u64::try_from(n).expect("the fixture answers a small code"),
        other => panic!("the interpreter did not answer a number: {other:?}"),
    }
}

/// The same entry as the IMAGE answers it: the `.t1` compiler builds the
/// sources and the machine runs the result.
fn native(sources: &[(&str, &str)]) -> u64 {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets("सरणपरीक्षा".as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let texts = sources.iter().map(|(_, s)| octets(s.as_bytes())).collect();
    let names = sources.iter().map(|(m, _)| octets(m.as_bytes())).collect();
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(texts),
                arena(names),
                Value::Int(i128::try_from(sources.len()).expect("a few sources")),
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
    match m.run(200_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the fixture did not finish: {other:?}"),
    }
}

/// Both engines, each against the SAME absolute expectation.
fn both(what: &str, sources: &[(&str, &str)], want: u64) {
    let i = interpreted(sources);
    let n = native(sources);
    assert_eq!(
        (i, n),
        (want, want),
        "{what}: (interpreted, native) — १ is the wanted value, ३ is all-ones, \
         २ is neither"
    );
}

/// A routine that binds `फलम्` to `shift` after `decls`, and answers १ when it
/// equals `want`, ३ when it is all-ones, २ otherwise.
fn fixture(decls: &str, shift: &str, want: &str) -> String {
    format!(
        "मण्डलम् सरणपरीक्षा ॥\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
         {decls}    चरः फलम् ॱॱ न६४ भवति {shift} ।\n    \
         यदि फलम् समम् {want} आदि\n        प्रत्यागमनम् १ ।\n    इति\n    \
         यदि फलम् समम् ऋण१ आदि\n        प्रत्यागमनम् ३ ।\n    इति\n    \
         प्रत्यागमनम् २ ।\nइति\n"
    )
}

fn one(what: &str, decls: &str, shift: &str, want_value: &str, want: u64) {
    let src = fixture(decls, shift, want_value);
    both(what, &[("सरणपरीक्षा", &src)], want);
}

/// THE TWO WITNESSES, red before and green after. `ऋण१` in a `न६४` has ONE
/// bit left after a shift by ६३; `ऋण३` shifted by ३२ leaves the high thirty-two
/// bits, all set. Both answered all-ones on both engines.
#[test]
fn the_two_foreign_witnesses_answer_the_logical_value_on_both_engines() {
    one(
        "`न६४ क भवति ऋण१`, `क दक्षिणसृ ६३`",
        "    चरः क ॱॱ न६४ भवति ऋण१ ।\n",
        "क दक्षिणसृ ६३",
        "१",
        WANTED,
    );
    one(
        "`न६४ क भवति ऋण३`, `क दक्षिणसृ ३२`",
        "    चरः क ॱॱ न६४ भवति ऋण३ ।\n",
        "क दक्षिणसृ ३२",
        "४२९४९६७२९५",
        WANTED,
    );
}

/// The case the engines DISAGREED on before: 2^63 written as a positive
/// numeral. The interpreter already answered १; the image answered all-ones.
#[test]
fn an_unsigned_name_holding_two_to_the_sixty_three_is_one_on_both_engines() {
    one(
        "`न६४ क भवति ९२२३३७२०३६८५४७७५८०८`, `क दक्षिणसृ ६३`",
        "    चरः क ॱॱ न६४ भवति ९२२३३७२०३६८५४७७५८०८ ।\n",
        "क दक्षिणसृ ६३",
        "१",
        WANTED,
    );
}

/// GREEN ON THE BASE TOO, AND MEANT TO BE: this is a control and not coverage
/// of the fix. It passed on `a4617c80` before any edit; what it guards is the
/// fix going too far.
///
/// **THE CONTROL: WHAT MUST NOT MOVE.** A signed name sign-extends — `ऋण८`
/// shifted by १ is `ऋण४`, which a logical shift would turn into 2^63 − 4 —
/// and a LITERAL left operand is not a name, so it keeps the arithmetic shift
/// and answers all-ones. Both were measured before the fix and read the same.
#[test]
fn a_signed_name_still_sign_extends_and_a_literal_stays_arithmetic() {
    one(
        "`अ६४ स भवति ऋण८`, `स दक्षिणसृ १` is `ऋण४`",
        "    चरः स ॱॱ अ६४ भवति ऋण८ ।\n",
        "स दक्षिणसृ १",
        "ऋण४",
        WANTED,
    );
    one(
        "the literal `ऋण१ दक्षिणसृ ६३` is NOT a name and stays all-ones",
        "",
        "ऋण१ दक्षिणसृ ६३",
        "१",
        ALL_ONES,
    );
}

/// GREEN ON THE BASE TOO: a shift by ० returns the operand unchanged under
/// either lowering, so this passed before the fix and says nothing about it.
/// It is pinned because the interpreter's NEW logical arm treats ० apart from
/// १..६३, and an arm that masked at ० would turn `ऋण१` into 2^64 − 1 there.
#[test]
fn a_shift_by_zero_returns_the_unsigned_name_unchanged() {
    one(
        "`न६४ क भवति ऋण१`, `क दक्षिणसृ ०` is still `ऋण१`",
        "    चरः क ॱॱ न६४ भवति ऋण१ ।\n",
        "क दक्षिणसृ ०",
        "ऋण१",
        WANTED,
    );
}

const TAIL: &str = "    यदि फलम् समम् १ आदि\n        प्रत्यागमनम् १ ।\n    इति\n    \
                    यदि फलम् समम् ऋण१ आदि\n        प्रत्यागमनम् ३ ।\n    इति\n    \
                    प्रत्यागमनम् २ ।\nइति\n";

/// **EVERY WAY A SOURCE NAMES AN UNSIGNED VALUE IS A NAME, IN BOTH ENGINES.**
/// The mark is made from a declaration, and a declaration can be a parameter,
/// a module's own global, another module's global named with its module, or
/// any of those inside a group. The grouped case is the one that decides
/// whether the engines can part: the interpreter's tree has no group node, so
/// `आरभ्य क समाप्तम्` IS `क` there, and the `.t1` builder walks the group off
/// to say the same.
#[test]
fn a_parameter_a_global_a_qualified_global_and_a_grouped_name_are_all_names() {
    let param = format!(
        "मण्डलम् सरणपरीक्षा ॥\n\
         वृत्तिः सरति आदाय क ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् क दक्षिणसृ ६३ ।\nइति\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    चरः फलम् ॱॱ न६४ भवति सरति ऋण१ ।\n{TAIL}"
    );
    both("a parameter", &[("सरणपरीक्षा", &param)], WANTED);

    let global = format!(
        "मण्डलम् सरणपरीक्षा ॥\n\
         सार्वजनिक चरः वैश्विकमानम् ॱॱ न६४ भवति ० ।\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    वैश्विकमानम् भवति ऋण१ ।\n    \
         चरः फलम् ॱॱ न६४ भवति वैश्विकमानम् दक्षिणसृ ६३ ।\n{TAIL}"
    );
    both("the module's own global", &[("सरणपरीक्षा", &global)], WANTED);

    let library = "मण्डलम् सरणकोश ॥\nसार्वजनिक चरः दूरमानम् ॱॱ न६४ भवति ० ।\n";
    let qualified = format!(
        "मण्डलम् सरणपरीक्षा ॥\nआयातः सरणकोश ।\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    सरणकोशॱदूरमानम् भवति ऋण१ ।\n    \
         चरः फलम् ॱॱ न६४ भवति सरणकोशॱदूरमानम् दक्षिणसृ ६३ ।\n{TAIL}"
    );
    both(
        "another module's global, named with its module",
        &[("सरणकोश", library), ("सरणपरीक्षा", &qualified)],
        WANTED,
    );

    one(
        "a grouped name, `आरभ्य क समाप्तम् दक्षिणसृ ६३`",
        "    चरः क ॱॱ न६४ भवति ऋण१ ।\n",
        "आरभ्य क समाप्तम् दक्षिणसृ ६३",
        "१",
        WANTED,
    );
}

/// **THE INNERMOST DECLARATION DECIDES, READ INSIDE ITS BLOCK.** One name,
/// two declarations of opposite signedness; the shift takes the one in scope
/// where it is written. See the file header for why neither probe reads the
/// name after the inner block closes.
#[test]
fn a_shadowed_name_takes_the_signedness_of_its_innermost_declaration() {
    let inner_unsigned = "मण्डलम् सरणपरीक्षा ॥\n\
        सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    चरः क ॱॱ अ६४ भवति ऋण१ ।\n    \
        यदि क समम् ऋण१ आदि\n        चरः क ॱॱ न६४ भवति ऋण१ ।\n        \
        चरः फलम् ॱॱ न६४ भवति क दक्षिणसृ ६३ ।\n        \
        यदि फलम् समम् १ आदि\n            प्रत्यागमनम् १ ।\n        इति\n        \
        यदि फलम् समम् ऋण१ आदि\n            प्रत्यागमनम् ३ ।\n        इति\n        \
        प्रत्यागमनम् २ ।\n    इति\n    प्रत्यागमनम् ९ ।\nइति\n";
    both(
        "an inner `न६४` over an outer `अ६४` is LOGICAL inside the block",
        &[("सरणपरीक्षा", inner_unsigned)],
        WANTED,
    );

    let inner_signed = "मण्डलम् सरणपरीक्षा ॥\n\
        सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    चरः क ॱॱ न६४ भवति ऋण१ ।\n    \
        यदि क समम् ऋण१ आदि\n        चरः क ॱॱ अ६४ भवति ऋण८ ।\n        \
        चरः फलम् ॱॱ अ६४ भवति क दक्षिणसृ १ ।\n        \
        यदि फलम् समम् ऋण४ आदि\n            प्रत्यागमनम् १ ।\n        इति\n        \
        प्रत्यागमनम् २ ।\n    इति\n    प्रत्यागमनम् ९ ।\nइति\n";
    both(
        "an inner `अ६४` over an outer `न६४` is ARITHMETIC inside the block",
        &[("सरणपरीक्षा", inner_signed)],
        WANTED,
    );
}
