//! `W-364`: a name re-declared in a NESTED block must SHADOW natively, as it does in the
//! interpreter (grammar-t1.ebnf's scope paragraph; `t1_execution.rs`'s stack test) — while a
//! name re-declared in a SIBLING block must keep SHARING one slot.
//!
//! Each case is ONE NUMBER answering three reads of `क` (before, inside, after the inner
//! block), so the engines compare by status alone. From a peer session's probe margin:
//!   २२ = १ + २० + १  CORRECT — innermost-first inside, the shadow left with its scope;
//!   ४१ = १ + २० + २० THE SHADOW LEAKED — the inner `चरः` wrote the outer slot;
//!    ३ = १ +  १ +  १ THE SHADOW WAS NOT SEEN;  २ = १ + ० + १ THE INNER BLOCK NEVER RAN.
//!
//! The probe pair is copied VERBATIM (code lines) from the local checkout's w333-probe/
//! chaya.t1 (md5 d472a035…) and chaya-control.t1 (md5 555760bf…), and kept here as fixture
//! STRINGS, not as `.t1` files, so `t1_population.rs`'s pins do not move. The `अन्यथा` and
//! `यावत्` cases are the same probe with the re-declaration moved into the other two body
//! forms a peer session named (ir.t1's यदि-then, अन्यथा and यावत् arms all lower through the
//! `समूहवाक्यभेद` arm).

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

/// Build `src` (module `छाया`, entry `मुख्य`) with the interpreted chain, run it natively,
/// and answer (native status, the interpreter's own answer, the builder's slot count for
/// the routine just built — `स्थानीयसंख्यान`, ir.t1's locals count).
fn both(src: &str) -> (u64, i128, i128) {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets("छाया".as_bytes()), octets("मुख्य".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(src.as_bytes())]),
                arena(vec![octets("छाया".as_bytes())]),
                Value::Int(1),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!image.is_empty(), "the fixture built no image");
    let slots = match it.global("स्थानीयसंख्यान") {
        Some(Value::Int(n)) => *n,
        other => panic!("स्थानीयसंख्यान should be a number after the build, got {other:?}"),
    };
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let native = match m.run(200_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the fixture did not finish natively: {other:?}"),
    };
    let mut ii = Interpreter::load(&[("chaya.t1", src)], &spec_root()).expect("the fixture loads");
    let interp = match ii
        .call("मुख्य", Vec::new(), 10_000_000)
        .expect("the fixture runs")
    {
        Value::Int(n) => n,
        other => panic!("the interpreter answered {other:?}"),
    };
    (native, interp, slots)
}

/// chaya.t1's code, verbatim (d472a035…): the re-declaration is in the यदि-then body.
const CHAYA: &str = "मण्डलम् छाया ॥

सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ ।
    चरः ख ॱॱ न६४ भवति क ।
    चरः ग ॱॱ न६४ भवति ० ।
    यदि ख समम् १ आदि
        चरः क ॱॱ न६४ भवति २० ।
        ग भवति क ।
    इति
    चरः च ॱॱ न६४ भवति क ।
    चरः फलम् ॱॱ न६४ भवति ख योगः ग ।
    फलम् भवति फलम् योगः च ।
    प्रत्यागमनम् फलम् ।
इति
";

/// chaya-control.t1's code, verbatim (555760bf…): a FRESH inner name, no shadow.
const CONTROL: &str = "मण्डलम् छाया ॥

सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ ।
    चरः ख ॱॱ न६४ भवति क ।
    चरः ग ॱॱ न६४ भवति ० ।
    यदि ख समम् १ आदि
        चरः घ ॱॱ न६४ भवति २० ।
        ग भवति घ ।
    इति
    चरः च ॱॱ न६४ भवति क ।
    चरः फलम् ॱॱ न६४ भवति ख योगः ग ।
    फलम् भवति फलम् योगः च ।
    प्रत्यागमनम् फलम् ।
इति
";

/// The same probe with the re-declaration in an अन्यथा body (the condition is false).
const ANYATHA: &str = "मण्डलम् छाया ॥

सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ ।
    चरः ख ॱॱ न६४ भवति क ।
    चरः ग ॱॱ न६४ भवति ० ।
    यदि ख समम् ० आदि
        ग भवति ० ।
    इति अन्यथा आदि
        चरः क ॱॱ न६४ भवति २० ।
        ग भवति क ।
    इति
    चरः च ॱॱ न६४ भवति क ।
    चरः फलम् ॱॱ न६४ भवति ख योगः ग ।
    फलम् भवति फलम् योगः च ।
    प्रत्यागमनम् फलम् ।
इति
";

/// The same probe with the re-declaration in a यावत् body that runs exactly once.
const YAVAT: &str = "मण्डलम् छाया ॥

सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ ।
    चरः ख ॱॱ न६४ भवति क ।
    चरः ग ॱॱ न६४ भवति ० ।
    चरः घ ॱॱ न६४ भवति ० ।
    यावत् घ न्यूनम् १ आदि
        चरः क ॱॱ न६४ भवति २० ।
        ग भवति क ।
        घ भवति घ योगः १ ।
    इति
    चरः च ॱॱ न६४ भवति क ।
    चरः फलम् ॱॱ न६४ भवति ख योगः ग ।
    फलम् भवति फलम् योगः च ।
    प्रत्यागमनम् फलम् ।
इति
";

fn sibling(second: &str) -> String {
    format!(
        "मण्डलम् छाया ॥

सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि
    चरः ख ॱॱ न६४ भवति १ ।
    चरः ग ॱॱ न६४ भवति ० ।
    यदि ख समम् १ आदि
        चरः ट ॱॱ न६४ भवति ३ ।
        ग भवति ग योगः ट ।
    इति
    यदि ख समम् १ आदि
        चरः {second} ॱॱ न६४ भवति ४ ।
        ग भवति ग योगः {second} ।
    इति
    प्रत्यागमनम् ग ।
इति
"
    )
}

fn shadows(name: &str, src: &str) {
    let (native, interp, _) = both(src);
    assert_eq!(
        interp, 22,
        "{name}: the interpreter must answer २२ (१+२०+१)"
    );
    assert_eq!(
        native, 22,
        "{name}: native answered {native}; ४१ = THE SHADOW LEAKED (the inner चरः wrote the outer slot)"
    );
}

#[test]
fn a_shadow_in_a_yadi_then_body_leaves_with_its_scope_natively() {
    shadows("chaya (यदि-then)", CHAYA);
}

#[test]
fn the_control_with_a_fresh_inner_name_answers_22_on_both() {
    let (native, interp, _) = both(CONTROL);
    assert_eq!(
        (native, interp),
        (22, 22),
        "control: no shadow, so both engines must answer २२"
    );
}

#[test]
fn a_shadow_in_an_anyatha_body_leaves_with_its_scope_natively() {
    shadows("अन्यथा", ANYATHA);
}

#[test]
fn a_shadow_in_a_yavat_body_leaves_with_its_scope_natively() {
    shadows("यावत्", YAVAT);
}

#[test]
fn siblings_declaring_one_name_still_share_one_slot() {
    // Two sibling blocks: the same name in both (must SHARE), against two different names
    // (must take two). Asserted on the builder's own slot count, not only the answer.
    let (n_same, i_same, slots_same) = both(&sibling("ट"));
    let (n_diff, i_diff, slots_diff) = both(&sibling("ठ"));
    assert_eq!(
        (n_same, i_same, n_diff, i_diff),
        (7, 7, 7, 7),
        "both pairs answer ३+४ on both engines"
    );
    assert_eq!(
        slots_diff - slots_same,
        1,
        "the same-name siblings must share ONE slot (same {slots_same}, distinct {slots_diff}); \
         a fix that gives siblings separate slots loses frame space the corpus relies on (63 sites)"
    );
}

/// One routine, FIVE distinct locals and nothing that needs the anonymous pool
/// (`अनामस्थानम्`, which would add eight): the builder's count for it is known in advance.
const FIVE: &str = "मण्डलम् छाया ॥

सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ ।
    चरः ख ॱॱ न६४ भवति २ ।
    चरः ग ॱॱ न६४ भवति ३ ।
    चरः घ ॱॱ न६४ भवति ४ ।
    चरः ङ ॱॱ न६४ भवति ५ ।
    चरः फलम् ॱॱ न६४ भवति क योगः ख ।
    फलम् भवति फलम् योगः ग ।
    फलम् भवति फलम् योगः घ ।
    फलम् भवति फलम् योगः ङ ।
    प्रत्यागमनम् फलम् ।
इति
";

#[test]
fn the_slot_measure_reads_the_routine_it_means() {
    // A POSITIVE CONTROL ON THE INSTRUMENT (a peer session's review): an ABSOLUTE count, not a
    // difference. `स्थानीयसंख्यान` is reset per routine (ir.t1 :5239, before the parameters
    // and the body), so after the build it holds the LAST routine built. If the chain ever
    // lowers another routine after the user's, THIS test fails first and names the measure,
    // rather than leaving the sibling test to wonder whether the subject or the instrument
    // moved. Six locals: क ख ग घ ङ and फलम्, no parameters, no pool.
    let (native, interp, slots) = both(FIVE);
    assert_eq!((native, interp), (15, 15), "१+२+३+४+५ on both engines");
    assert_eq!(
        slots, 6,
        "the builder's count for six distinct locals and no parameters"
    );
}
