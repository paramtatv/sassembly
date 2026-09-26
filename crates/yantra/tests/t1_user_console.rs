//! **A PROGRAM THAT IS NOT THE COMPILER REACHES THE CONSOLE — AND BOTH ENGINES AGREE.**
//!
//! This is the third capability measured on 2026-09-24 that the repository's own
//! narrative reads as absent. `lib.t1` is fifteen lines of comments, a compiled
//! program has two SBI calls, and `ir.t1:2331` says the one output channel "is
//! not a call: it is a STORE to the UART at 0x10000000" — from which it is easy
//! to conclude that printing is a compiler privilege. It is not: a user module
//! writes `आयातः अष्टक ।` and calls `अष्टकॱमुद्रणम्` per octet.
//!
//! **THIS ONE CAN BE CROSS-CHECKED, AND THAT IS THE POINT.** Its two siblings
//! could not both do it. `t1_user_allocation.rs` compares an arithmetic answer
//! across the engines; `t1_user_input_interface.rs` cannot, because the
//! interpreter has no RAM for the host to inject into. The output channel is
//! intercepted on BOTH sides by design — `nirvahana.rs:2035` matches the
//! qualified call and pushes the octet into `Interpreter::sink()`, `ir.t1:2580`
//! lowers the same call to an MMIO store, and `yantra/src/lib.rs:752` catches
//! that store — so the same program's output can be read from either and
//! compared.
//!
//! That matters because this corpus's whole defect history is the two engines
//! disagreeing: five in one day on 2026-09-22, every one invisible to the
//! interpreter and visible only when the compiled compiler ran itself. An
//! output test that only ran natively would be the weak half of the evidence
//! for a channel that has two halves on purpose.
//!
//! **WHAT IS ASSERTED IS THE BYTES.** `nirvahana.rs:1333` says why: asserting on
//! the intrinsic's return value or on an exit status "would pass while nothing
//! reached the sink at all — the one failure the marker body cannot make loud".
//! `अष्टकॱमुद्रणम्`'s body answers the device address (268435456) when NEITHER
//! interception fires, precisely so a caller can tell.

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

/// Writes `HI!\n`. The octets are ASCII so a reader can see what is expected,
/// and the trailing newline is included because a channel that dropped the last
/// octet would otherwise pass.
const PRINTS: &str = "मण्डलम् मुद्रक ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ७२ ।
    अवगणना भवति अष्टकॱमुद्रणम् ७३ ।
    अवगणना भवति अष्टकॱमुद्रणम् ३३ ।
    अवगणना भवति अष्टकॱमुद्रणम् १० ।
    प्रत्यागमनम् ० ।
इति
";

const WANT: &[u8] = b"HI!\n";

fn build(src: &str, module: &str) -> Vec<u8> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(module.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    it.call(
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![
            arena(vec![octets(src.as_bytes())]),
            arena(vec![octets(module.as_bytes())]),
            Value::Int(1),
        ],
        FUEL,
    )
    .expect("मण्डलानिप्रतिबिम्बम् runs")
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default()
}

#[test]
fn a_user_module_prints_and_both_engines_emit_the_same_octets() {
    // ── the INTERPRETER's half: the fixture joins the chain, as `t1_image
    //    --load` does, and its own routine runs. Omitting that flag is why the
    //    predict answered "no routine named …" on every user program built on
    //    2026-09-24 — a correct refusal that reads as noise.
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push(("मुद्रक.t1", PRINTS));
    let mut it = Interpreter::load(&srcs, &spec_root()).expect("the chain plus the fixture loads");
    it.call("मुद्रकॱमुख्यम्", vec![], FUEL)
        .expect("the fixture runs under the interpreter");
    let interpreted = it.sink().to_vec();
    assert_eq!(
        interpreted,
        WANT,
        "the INTERPRETER's sink holds {:?}, not {:?}. `अष्टकॱमुद्रणम्`'s body \
         answers 268435456 when no interception fires, so an empty sink here \
         means the call reached the body instead of the channel",
        String::from_utf8_lossy(&interpreted),
        String::from_utf8_lossy(WANT)
    );

    // ── the NATIVE half: the same source, compiled and run on the machine.
    let image = build(PRINTS, "मुद्रक");
    assert!(!image.is_empty(), "the fixture built no image");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut native: Vec<u8> = Vec::new();
    match m.run(200_000_000, &mut native) {
        yantra::Halt::Finisher {
            status: Some(0), ..
        } => {}
        other => panic!("the fixture did not finish cleanly: {other:?}"),
    }

    assert_eq!(
        native,
        WANT,
        "the MACHINE wrote {:?}, not {:?}",
        String::from_utf8_lossy(&native),
        String::from_utf8_lossy(WANT)
    );
    // The claim this file exists for, asserted separately so a divergence is
    // named as a divergence and not as a wrong string.
    assert_eq!(
        interpreted,
        native,
        "the two engines emitted DIFFERENT octets for one source — interpreted \
         {:?}, native {:?}. That is the class every defect in this corpus has \
         been, and the output channel is intercepted on both sides precisely so \
         it can be caught here",
        String::from_utf8_lossy(&interpreted),
        String::from_utf8_lossy(&native)
    );
}
