//! **`W-355`: A FRESH NON-OCTET RUN IS EMPTY ON BOTH ENGINES, AND INDEX ० OF IT
//! IS REFUSED ON BOTH.**
//!
//! Owner ruling 2026-10-03: a run declared `भवति ०` and never stored into has
//! LENGTH ० and NO slot at index ०, interpreted and compiled alike. Before it:
//!
//! ```text
//!   probe                      interpreted    native        (51598b3c, a peer session)
//!   सूची ॱ दैर्घ्य                  1              1          the COUNT agreed
//!   सूची अङ्कः ० अन्तः              Nil            0          the HOLE did not
//! ```
//!
//! The interpreter's zero run was `vec![Value::Nil]`; natively a fresh run is the
//! nil word, and `खण्डदैर्घ्यरचना` answered `len + (len == ०)` to reproduce the
//! count while `सुरक्षितसूचीरचना` answered a SILENT ० for the read — the value
//! the GPU-driver project's `pramana.t1` drew a phantom leading element from.
//!
//! The coordinator's ruling (i) on the read: it is OUT OF BOUNDS on both engines.
//! The interpreter refuses it ("entry 0 is outside an arena of 0"); the native
//! lowering refuses it by writing `रिक्तखण्डपठननिषेधः` (`0x355`) to the finisher
//! in the FAIL form `0x3333 | 0x355 << 16`, so `yantra` halts
//! `Finisher { status: Some(0x355) }` and QEMU stops on it too (`W-381`: the raw
//! `0x355` this pinned until 2026-10-06 was no finisher form, and QEMU ran past
//! it — `w381_refusal_finisher_form.rs`).
//!
//! The native side is compiled HERE, by the current `.t1` compiler running in the
//! interpreter (`CHAIN`), so the image carries the lowering this tree says — not
//! whatever the shipped `t1_image` binary was built from.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, RunError, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

const FUEL: u64 = 80_000_000_000;
const MODULE: &str = "रिक्तपरीक्षण";

/// `ir.t1`'s `रिक्तखण्डपठननिषेधः`, the native refusal's CODE, its status.
const REFUSAL_CODE: u64 = 0x355;
/// The finisher word: the code in FAIL form (`W-381`; it was the raw code).
const REFUSAL_WORD: u64 = (REFUSAL_CODE << 16) | 0x3333;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// Two fresh runs, one of integers and one of records: ४० + len(ints) +
/// १० × len(records). ४० on the ruling, ५१ on the old one-nil zero run.
const LENGTH: &str = "मण्डलम् रिक्तपरीक्षण ॥
संरचना बिन्दु आरभ्य
    मान ॱॱ अ६४
समाप्तम् ।
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    चरः बिन्दवः ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
    चरः फलम् ॱॱ न६४ भवति ४० ।
    फलम् भवति फलम् योगः सूची ॱ दैर्घ्य ।
    चरः दशकम् ॱॱ न६४ भवति बिन्दवः ॱ दैर्घ्य गुणनम् १० ।
    फलम् भवति फलम् योगः दशकम् ।
    प्रत्यागमनम् फलम् ।
इति
";

/// a peer session's probe (b), `tests/probes/w355_read.t1probe`: index ० of a fresh run.
const READ: &str = "मण्डलम् रिक्तपरीक्षण ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    प्रत्यागमनम् सूची अङ्कः ० अन्तः ।
इति
";

/// THE CONTROL for the refusal: the same read after a store at index ०. If the
/// refusal fired for every index read this would halt instead of answering ४३.
const STORED: &str = "मण्डलम् रिक्तपरीक्षण ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    सूची अङ्कः ० अन्तः भवति ४२ ।
    प्रत्यागमनम् सूची अङ्कः ० अन्तः योगः सूची ॱ दैर्घ्य ।
इति
";

/// `W-381` STAGE 4: a read PAST THE LENGTH of a run that HAS storage (index ५
/// of a run of one) — not a nil base, so only the bound check can refuse it.
/// The mutant below removes that check.
const PAST: &str = "मण्डलम् रिक्तपरीक्षण ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    सूची अङ्कः ० अन्तः भवति ४२ ।
    प्रत्यागमनम् सूची अङ्कः ५ अन्तः ।
इति
";

fn interpret(src: &str) -> Result<Value, RunError> {
    let mut it = Interpreter::load(&[("test", src)], Path::new("."))
        .unwrap_or_else(|e| panic!("the probe must load: {}", e.reason));
    it.call("मुख्यम्", Vec::new(), 1_000_000)
}

/// Compile `src` with the given compiler sources and run the image.
fn native_with(chain: &[(&str, &str)], src: &str) -> Halt {
    let mut it = Interpreter::load(chain, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(MODULE.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(src.as_bytes())]),
                arena(vec![octets(MODULE.as_bytes())]),
                Value::Int(1),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!image.is_empty(), "the probe built no image");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    m.run(200_000_000, &mut out)
}

fn native(src: &str) -> Halt {
    native_with(CHAIN, src)
}

fn status(h: &Halt) -> u64 {
    match h {
        Halt::Finisher {
            status: Some(s), ..
        } => *s,
        other => panic!("the image did not finish with a status: {other:?}"),
    }
}

fn is_native_refusal(h: &Halt) -> bool {
    matches!(
        h,
        Halt::Finisher {
            value: REFUSAL_WORD,
            status: Some(REFUSAL_CODE)
        }
    )
}

#[test]
fn a_fresh_run_has_length_zero_on_both_engines() {
    let interpreted = interpret(LENGTH)
        .unwrap_or_else(|e| panic!("the length probe must run: {}", e.reason))
        .as_int()
        .expect("a number");
    let compiled = status(&native(LENGTH));
    assert_eq!(
        interpreted, 40,
        "interpreted: a fresh `अङ्कः अन्तः न६४` and a fresh `अङ्कः अन्तः बिन्दु` must \
         both have length ० (४०); ५१ is the old zero run of one nil each"
    );
    assert_eq!(
        compiled, 40,
        "natively: the same two lengths must be ०; ५१ is `खण्डदैर्घ्यरचना`'s old \
         `len + (len == ०)` reproducing the interpreter's one-nil count"
    );
}

#[test]
fn index_zero_of_a_fresh_run_is_refused_by_both_engines() {
    match interpret(READ) {
        Err(e) => assert!(
            e.reason.contains("outside an arena of 0"),
            "the interpreter must refuse index ० of a fresh run as out of bounds; \
             it said: {}",
            e.reason
        ),
        Ok(v) => panic!(
            "the interpreter answered {v:?} for index ० of a fresh run; it must \
             refuse (Nil is the old zero run's hole)"
        ),
    }
    let h = native(READ);
    assert!(
        is_native_refusal(&h),
        "natively index ० of a fresh run must halt with the finisher word \
         0x{REFUSAL_WORD:x}, status 0x{REFUSAL_CODE:x}, the refusal `सुरक्षितसूचीरचना` writes; \
         it halted {h:?} (status 0 is the old silent ०)"
    );
}

#[test]
fn a_stored_slot_reads_back_on_both_engines() {
    let interpreted = interpret(STORED)
        .unwrap_or_else(|e| panic!("the stored probe must run: {}", e.reason))
        .as_int()
        .expect("a number");
    assert_eq!(interpreted, 43, "४२ stored at ० plus length १, interpreted");
    assert_eq!(
        status(&native(STORED)),
        43,
        "natively the refusal must not fire on a run that has storage"
    );
}

/// THE MUTANT REMOVES THE BOUND CHECK (`W-381` stage 4): the bound branch's
/// refusal target becomes the load's own block, so a read past the length of a
/// run with storage reads whatever word lies there. Unmutated, both engines
/// refuse `PAST`; mutated, the native image must NOT refuse — so this goes red
/// if the bound check is ever removed or stops reaching the refusal. (Until
/// stage 4 the mutant sent the nil branch to a join that answered a pre-stored
/// ०; there is no join now: every refusal spins, so the read has no slot.)
#[test]
fn the_mutant_without_the_bound_check_is_caught() {
    const LIVE: &str = "शाखारचना आरभ्य सीमाशर्तः ऽ निषेधपर्व ऽ अग्रिमपर्व समाप्तम्";
    const MUTANT: &str = "शाखारचना आरभ्य सीमाशर्तः ऽ अग्रिमपर्व ऽ अग्रिमपर्व समाप्तम्";
    match interpret(PAST) {
        Err(e) => assert!(
            e.reason.contains("is outside an arena of 1"),
            "the interpreter must refuse index ५ of a run of one; it said: {}",
            e.reason
        ),
        Ok(v) => panic!("the interpreter answered {v:?} for a read past the length"),
    }
    let live = native(PAST);
    assert!(
        is_native_refusal(&live),
        "natively a read past the length must refuse 0x{REFUSAL_CODE:x}; it halted {live:?}"
    );
    let ir = CHAIN
        .iter()
        .find(|(n, _)| *n == "ir.t1")
        .map(|(_, s)| *s)
        .expect("CHAIN carries ir.t1");
    assert_eq!(
        ir.matches(LIVE).count(),
        1,
        "the mutation must match exactly one site, `सुरक्षितसूचीरचना`'s bound branch"
    );
    let mutated = ir.replacen(LIVE, MUTANT, 1);
    let chain: Vec<(&str, &str)> = CHAIN
        .iter()
        .map(|(n, s)| {
            if *n == "ir.t1" {
                (*n, mutated.as_str())
            } else {
                (*n, *s)
            }
        })
        .collect();
    let h = native_with(&chain, PAST);
    assert!(
        !is_native_refusal(&h),
        "the mutant without the bound check must NOT refuse; it halted {h:?}"
    );
}
