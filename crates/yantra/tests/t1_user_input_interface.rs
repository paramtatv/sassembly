//! **A PROGRAM THAT IS NOT THE COMPILER CAN READ THE FILE IT WAS GIVEN.**
//!
//! Measured 2026-09-24 and it contradicts what this repository's own narrative
//! implied. `crates/sadhana-t1/src/lib.t1` is fifteen lines of comments, a
//! compiled program reaches the machine through exactly two SBI calls
//! (`console_putchar`, `shutdown`), and `ir.t1:2331` says the ONE output
//! channel "is not a call: it is a STORE to the UART". From that it reads as
//! though input were equally absent and would need a device, an `ecall` the
//! language cannot emit, or a standard library that does not exist.
//!
//! **None of that is required.** `yantra::input::inject` writes the host file's
//! octets INTO RAM before `pc` first moves, and finds where to record them by
//! SCANNING RAM FOR A MAGIC WORD (`input.rs:155`) — not by a symbol lookup. So
//! any module that declares the six `निवेश…` globals gets the run filled in,
//! with no compiler change at all.
//!
//! **THE ROUTE THAT FAILS, AND WHY IT MISLEADS.** The obvious attempt is to
//! import the compiler's own declarations — `आयातः शृङ्खला`, then read
//! `शृङ्खलाॱनिवेशपाठः`. That REFUSES: "no input interface in this image: the
//! SASINPUT tag appears at no word." The linker emits only reachable globals,
//! so the tags never enter the image and the host has nothing to fill. Reading
//! that refusal as "a program cannot read files" is the mistake this file
//! exists to prevent; it says the image lacks the interface, not the language.
//!
//! **WHAT THIS IS AND IS NOT.** The program reads THE INPUT IT WAS GIVEN — the
//! host picks the file. It cannot name or open a path, cannot write, and the
//! whole file is resident before the program starts. Those are real limits and
//! none of them is what the refusal above reports.

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

/// The six declarations, verbatim from `shrinkhala.t1:3441-3446`. THE TAGS ARE
/// THE CONTRACT: `input.rs` scans for these three words and writes the injected
/// run into the data word after each. `t1_input_channel.rs` already pins that
/// the numbers here and the constants in `input.rs` are one value in two
/// places; this file pins that a NON-COMPILER module can use them.
///
/// It SUMS the octets rather than reporting the length, so the answer depends
/// on the file's CONTENT. A program that found an empty run, or a run of the
/// right length full of zeroes, would answer ० and pass a length assertion.
const READS_ITS_INPUT: &str = "मण्डलम् निवेशक ॥

सार्वजनिक चरः निवेशसङ्केतः ॱॱ न६४ भवति ६०७६८५१५६९३७४२१६५३१ ।
सार्वजनिक चरः निवेशपाठः ॱॱ अङ्कः अन्तः अ८ भवति ० ।
सार्वजनिक चरः निवेशनामसङ्केतः ॱॱ न६४ भवति ४९९३७१९३६६३१७१९५६०३ ।
सार्वजनिक चरः निवेशमण्डलनाम ॱॱ अङ्कः अन्तः अ८ भवति ० ।
सार्वजनिक चरः निवेशानुरेखणसङ्केतः ॱॱ न६४ भवति ४९९०९०४६३३९१४५०७६०३ ।
सार्वजनिक चरः निवेशानुरेखणम् ॱॱ न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः योगः ॱॱ न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् निवेशपाठः ॱ दैर्घ्य आदि
        योगः भवति योगः योगः निवेशपाठः अङ्कः क्रमः अन्तः ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् योगः ।
इति
";

/// The same program WITHOUT the six declarations — the control. Its image has
/// no tags, so the host must refuse rather than quietly inject nowhere.
const DECLARES_NO_INTERFACE: &str = "मण्डलम् निवेशरहितः ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    प्रत्यागमनम् ० ।
इति
";

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
fn a_program_declaring_the_interface_reads_the_octets_it_was_given() {
    let image = build(READS_ITS_INPUT, "निवेशक");
    assert!(!image.is_empty(), "the fixture built no image");

    // Bytes chosen so the sum is not a round number and not the length: a
    // reader that answered the length would say 11, and one that answered ०
    // would say ०.
    let text = b"sassembly!!";
    let want: u64 = text.iter().map(|b| u64::from(*b)).sum();

    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let base = m.base;
    yantra::input::inject(&mut m.mem, base, text, "निवेशक".as_bytes(), 0)
        .expect("the host finds all three tags and fills the slots");

    // NO CROSS-ENGINE CHECK IS POSSIBLE HERE, AND THAT IS A PROPERTY OF THE
    // FEATURE. `t1_user_allocation.rs` compares the interpreter's answer with
    // the native one because both engines compute the same sum. This fixture's
    // answer DEPENDS ON `yantra::input::inject`, which writes octets into a
    // machine's RAM — the interpreter has no RAM and no injection, so
    // `निवेशपाठः` is the empty run there and the interpreted answer is ० for
    // every input. Asserting agreement would assert ० == the sum and fail on a
    // working feature.
    //
    // So this test is NATIVE-ONLY BY NECESSITY, not by omission, and the
    // strength it has instead is the octet SUM plus the refusal control below.
    // Worth stating because a native-only number is the weak form of evidence
    // in this corpus — five defects on 2026-09-22 were invisible to the
    // interpreter — and a reader should know which kind this is.
    let mut out: Vec<u8> = Vec::new();
    let status = match m.run(200_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the fixture did not finish: {other:?}"),
    };
    assert_eq!(
        status,
        want,
        "the program summed {status} where the {} injected octets sum to {want}. \
         A length reader answers {}, an empty run answers 0 — neither is this",
        text.len(),
        text.len()
    );
}

#[test]
fn an_image_without_the_declarations_is_refused_rather_than_silently_empty() {
    let image = build(DECLARES_NO_INTERFACE, "निवेशरहितः");
    assert!(!image.is_empty(), "the control built no image");

    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let base = m.base;
    let refusal = yantra::input::inject(&mut m.mem, base, b"anything", "निवेशरहितः".as_bytes(), 0);

    // THE REFUSAL IS THE FEATURE. An injection that silently found nowhere to
    // record the run would hand the program an empty input and no way to tell
    // that apart from an empty file — which is exactly the reading that made
    // "a program cannot read files" look true.
    let e = refusal.expect_err("injecting into an image with no interface must refuse");
    assert!(
        e.contains("no input interface"),
        "the refusal must say the IMAGE lacks the interface, since that is the \
         repairable fact; got {e:?}"
    );
}
