//! **`SAS-013` — A REFUSED MODULE NAMED NOTHING, AND THE DRIVERS ARE WHERE IT
//! WAS READ.** A module `अर्थ` refuses came back out of `t1_boot --object` as
//! `सङ्कलनविरामभेद = Int(2)` and out of `t1_image` as `1 source(s) failed to
//! compile (चिन्ता)` — no routine, no line, no reason. The row was filed after a
//! builder bisected instrument variants for an hour to find a call to a routine
//! that existed only in another generator. Measured again on this tree before
//! the change, on the scratch module below:
//!
//! ```text
//!   सङ्कलनविरामभेद = Int(2)
//!   निर्णयविरामभेद = Int(1)
//!   … twenty-seven more globals, none of them a name or a line …
//! no text for चिन्ता
//! ```
//!
//! **THE ANSWER WAS ALREADY IN THE INTERPRETER AND ONLY `Front` COULD READ IT.**
//! `artha.t1` records the first refusal's name and line (`अनिर्णीतनाम` /
//! `अनिर्णीतपङ्क्ति` and four more records beside them), and `chain.rs`'s
//! `Front::resolve` has rendered them as `` `X` at line N has no declaration``
//! since W-223. Both drivers call `शृङ्खलाॱमण्डलसङ्कलनम्` — the `.t1` chain's own
//! driver — and never construct a `Front`, so neither ever saw that sentence.
//! The rendering additionally existed in `pradarshana/src/lib.rs:634` and
//! `yantra/tests/paradigm_encode.rs:2136`: three copies, and none in a driver.
//!
//! **WHAT THIS TEST PINS IS THE STAGE GATE, WHICH IS THE PART THAT CAN BE WRONG
//! QUIETLY.** `chain::refusal_site` asks `निर्णयविरामभेद` which stage refused
//! and reads ONLY that stage's records. The obvious alternative — ask all five
//! records in order — passes every single-module test and lies on the corpus:
//! one interpreter compiles twenty sources, `कार्यक्रमप्रकारपरीक्षा` is not even
//! entered when resolve refuses, and `प्रकारदोषमस्ति` still stands from an
//! earlier source. `the_stage_gate_refuses_a_stale_typecheck_site` drives
//! exactly that order on ONE interpreter and is the case that must still be
//! refused; without it these assertions would pass against the lying design.

use sadhana::t1::chain;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

const FUEL: u64 = 40_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// THIS TEST'S OWN LOADER, and it is `chain::CHAIN` rather than a list written
/// here. The subject is a whole-chain routine — `मण्डलसङ्कलनम्` lexes, parses,
/// resolves, typechecks, builds IR and emits — so no prefix of the corpus is
/// enough, and a second hand-kept list is the thing that costs a day when it
/// drifts. `CHAIN` is what `t1_boot` and `t1_image` load, which is the point:
/// the company these modules keep here is the company they keep in the driver.
fn load_chain() -> Interpreter {
    let refs: Vec<(&str, &str)> = chain::CHAIN.to_vec();
    Interpreter::load(&refs, &spec_root())
        .unwrap_or_else(|e| panic!("the {} chain sources load: {e:?}", refs.len()))
}

/// One source through the drivers' own entry point. Answers the emitted text,
/// which is EMPTY on every refusal — the state the drivers test.
fn compile(it: &mut Interpreter, module: &str, src: &str) -> String {
    match it.call(
        "शृङ्खलाॱमण्डलसङ्कलनम्",
        vec![
            Value::Octets(Octets::new(src.as_bytes())),
            Value::Octets(Octets::new(module.as_bytes())),
        ],
        FUEL,
    ) {
        Ok(v) => v
            .octets()
            .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            .unwrap_or_default(),
        Err(e) => panic!("`शृङ्खलाॱमण्डलसङ्कलनम्` for {module}: {e:?}"),
    }
}

fn stage(it: &Interpreter) -> i128 {
    it.global("निर्णयविरामभेद")
        .and_then(Value::as_int)
        .expect("`निर्णयविरामभेद` is declared by शृङ्खला")
}

/// A call to a routine no module declares. The line is FIVE, counted by the
/// resolver and not by hand: `प्रत्यागमनम्` is the fifth line of the text below.
const UNDECLARED: &str = "मण्डलम् चिन्ता ॥\n\
     \n\
     सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि\n\
     \x20   चरः संख्या ॱॱ न६४ भवति २१ ।\n\
     \x20   प्रत्यागमनम् अदृष्टवृत्तिः संख्या ।\n\
     इति\n";

/// `spec/rung/निजम्.t1`'s body: the arithmetic that answers ४२, inline, with
/// nothing undeclared and nothing mistyped. It must compile.
const CLEAN: &str = "मण्डलम् निजम् ॥\n\
     \n\
     सार्वजनिक वृत्तिः मुख्य ददाति न६४ आदि\n\
     \x20   चरः संख्या ॱॱ न६४ भवति २१ ।\n\
     \x20   प्रत्यागमनम् संख्या योगः संख्या ।\n\
     इति\n";

/// A declared routine whose body is `न६४` against a declared `बूल` return —
/// `t1_execution.rs`'s own typecheck fixture, with its module renamed so the
/// three sources here never collide in one registry.
const MISTYPED: &str = "मण्डलम् विपरीतम् ॥\n\
     सार्वजनिक वृत्तिः ग ददाति बूल आदि\n\
     \x20   चरः क ॱॱ न६४ भवति ३ ।\n\
     \x20   क ।\n\
     इति\n";

/// A source with a Latin letter in it — `x` in the routine's name. Every other
/// octet is in R-15-1, so the ONE code point outside it is what refuses this.
const LATIN: &str = "मण्डलम् परिधिपरीक्षा ॥\n\
     सार्वजनिक वृत्तिः मुख्यx ददाति न६४ आदि\n\
     \x20   प्रत्यागमनम् २१ ।\n\
     इति\n";

/// **W-304: A SOURCE OUTSIDE R-15-1 IS REFUSED, AND THE REFUSAL SAYS WHY.**
///
/// `पठनम्` gates on `अक्षरकोशॱपरिधिदोषः` and returns ० BEFORE `निर्णयः` runs,
/// so `निर्णयविरामभेद` never records this refusal — it still holds whatever the
/// PREVIOUS source left there. Without an arm of its own, `refusal_site`
/// answered `""` on exactly this path, which is how the wiring first showed up:
/// two fixtures in `t1_statement_expressions.rs` failed with `refusal: None`.
#[test]
fn a_source_outside_the_repertoire_is_named_as_a_repertoire_refusal() {
    let mut it = load_chain();
    let text = compile(&mut it, "परिधिपरीक्षा", LATIN);
    assert!(
        text.is_empty(),
        "a source with a Latin letter must be REFUSED:\n{}",
        &text[..text.len().min(400)]
    );
    let site = chain::refusal_site(&it).unwrap_or_default();
    assert!(
        site.starts_with("repertoire: octet "),
        "the refusal must name the repertoire, not a later stage; got {site:?}"
    );
}

/// **THE CASE THAT MUST STILL BE REFUSED: THE REPERTOIRE RECORD MUST NOT STAND.**
///
/// Same interpreter, immediately after the refusal above. `पठनम्` resets
/// `परिधिदोषस्थितम्` on entry, and if it stopped doing so this routine would
/// report a repertoire refusal for every later source — the stale-record
/// failure the stage gate exists to prevent, arriving by the new arm instead.
#[test]
fn a_clean_source_after_a_repertoire_refusal_has_no_site() {
    let mut it = load_chain();
    assert!(
        compile(&mut it, "परिधिपरीक्षा", LATIN).is_empty(),
        "the refusing module is compiled FIRST, so the clean one runs over its record"
    );
    assert!(
        chain::refusal_site(&it).is_some(),
        "…and that record exists"
    );
    assert!(
        !compile(&mut it, "निजम्", CLEAN).is_empty(),
        "the clean module must compile"
    );
    assert_eq!(
        chain::refusal_site(&it),
        None,
        "a module that compiled must have NO site — the repertoire record was left standing"
    );
}

/// **THE ROW'S FALSIFIER**, verbatim: *a scratch module calling an undeclared
/// routine reports its name and site.* Through the drivers' own call, not
/// through `Front`.
#[test]
fn an_undeclared_callee_is_named_with_its_line() {
    let mut it = load_chain();
    let text = compile(&mut it, "चिन्ता", UNDECLARED);
    assert!(
        text.is_empty(),
        "the module must be REFUSED — a site for a module that compiled would \
         be a record of nothing:\n{}",
        &text[..text.len().min(400)]
    );
    assert_eq!(stage(&it), 1, "resolve is stage १ of `निर्णयः`");
    assert_eq!(
        chain::refusal_site(&it).as_deref(),
        Some("resolve: `अदृष्टवृत्तिः` at line 5 has no declaration"),
        "the refusal must name the callee and the line"
    );
}

/// **THE CASE THAT MUST STILL BE REFUSED: A SITE FOR A MODULE THAT COMPILED.**
/// Same interpreter, immediately after the refusal above — which is the only
/// order that can catch a record left standing. `निर्णयः` resets
/// `निर्णयविरामभेद` on entry and `निर्णायकारम्भः` clears `अनिर्णीतमस्ति`, and
/// if either stopped doing so every driver would print the PREVIOUS source's
/// site beside a source that was fine.
#[test]
fn a_module_that_compiles_has_no_site() {
    let mut it = load_chain();
    assert!(
        compile(&mut it, "चिन्ता", UNDECLARED).is_empty(),
        "the refusing module is compiled FIRST, so the clean one runs over its record"
    );
    assert!(
        chain::refusal_site(&it).is_some(),
        "…and that record exists"
    );

    let text = compile(&mut it, "निजम्", CLEAN);
    assert!(
        !text.is_empty(),
        "`निजम्` declares everything it names and must EMIT: {}",
        chain::refusal_site(&it).unwrap_or_else(|| "and it recorded no site".into())
    );
    assert_eq!(stage(&it), 0, "both stages passed is `निर्णयसिद्धभेद` (०)");
    assert_eq!(
        chain::refusal_site(&it),
        None,
        "a compiled module has no refusal site, and the refused module's is gone"
    );
}

/// The OTHER stage, so the dispatch is shown to select rather than to route
/// everything through `resolve_site`.
#[test]
fn a_body_against_the_wrong_return_is_named_at_the_typecheck_stage() {
    let mut it = load_chain();
    assert!(
        compile(&mut it, "विपरीतम्", MISTYPED).is_empty(),
        "a न६४ body against a बूल return must be REFUSED"
    );
    assert_eq!(stage(&it), 2, "typecheck is stage २ of `निर्णयः`");
    let site = chain::refusal_site(&it).expect("the checker recorded its refusal");
    assert!(
        site.starts_with("typecheck: `ग`:"),
        "the typecheck refusal must name the routine: {site}"
    );
}

/// **THE STAGE GATE, AND THIS IS THE ASSERTION THAT KILLS THE OBVIOUS DESIGN.**
/// A typecheck refusal leaves `प्रकारदोषमस्ति` raised; `कार्यक्रमप्रकारपरीक्षा`
/// is the only thing that clears it and a RESOLVE refusal never reaches it. So
/// on one interpreter, mistyped-then-undeclared, a `refusal_site` that read all
/// five records in order would answer `` typecheck: `ग` … `` for a module whose
/// actual defect is an undeclared callee — the right sentence about the wrong
/// stage of the wrong source, which is worse than the silence it replaced.
#[test]
fn the_stage_gate_refuses_a_stale_typecheck_site() {
    let mut it = load_chain();
    assert!(compile(&mut it, "विपरीतम्", MISTYPED).is_empty());
    assert_eq!(stage(&it), 2);
    assert!(
        matches!(it.global("प्रकारदोषमस्ति"), Some(Value::Bool(true))),
        "the typecheck record must be STANDING for this test to mean anything"
    );

    assert!(compile(&mut it, "चिन्ता", UNDECLARED).is_empty());
    assert_eq!(stage(&it), 1, "the second module refuses at RESOLVE");
    assert!(
        matches!(it.global("प्रकारदोषमस्ति"), Some(Value::Bool(true))),
        "and the typecheck record is STILL standing — nothing in a resolve \
         refusal clears it, which is precisely why the stage decides"
    );
    assert_eq!(
        chain::refusal_site(&it).as_deref(),
        Some("resolve: `अदृष्टवृत्तिः` at line 5 has no declaration"),
        "the site must come from the stage that refused THIS module"
    );
}
