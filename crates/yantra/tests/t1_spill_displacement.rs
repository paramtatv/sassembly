//! **THE ALLOCATOR SPILLS, AND THE SPILLED VALUES COME BACK.**
//!
//! `SAS-016`'s corpus census (`t1_corpus_conditionals.rs`) measures 17,249
//! offsets off `स्तूपसूचकः` across 20 modules and finds the spill region used
//! **34 times, never deeper than a single slot** — `deepest_spill == 1`, pinned
//! there. A probe on 2026-09-25 named the reason: the only spilling routines in
//! the corpus are the 17 copies of `खण्डवृद्धिः`, the run-growth helper
//! `ir.t1:645` synthesises once per module, and one slot is all it needs.
//!
//! So `8·(num_spills + k)` — the displacement `riscv64.rs:406` puts every
//! `W-245` local behind — **is exercised by the whole corpus only at
//! `num_spills` ∈ {0, 1}**. A defect that first appears at two spill slots is
//! invisible to every test this repository owns, and the census's populated
//! `spill` band does not say otherwise: a band being non-empty is not a
//! statement about its depth.
//!
//! # What shape spills, measured rather than assumed
//!
//! Three hypotheses were refuted before the fourth worked, and each refutation
//! says something about the IR:
//!
//! | shape | `num_spills` | why |
//! |---|---|---|
//! | 30 declared `चरः` | 0 | every named value is a FRAME LOCAL (`num_locals=30`) |
//! | 20 parameters | 0 | a parameter is in the local table too (`num_locals=20`) |
//! | a 24-deep nest of literals | 0 | constant-folded; `bytes` stayed 32 at every depth |
//! | **a call whose ARGUMENTS are calls** | **n − 12** | every argument must be live at once |
//!
//! The threshold is `riscv64::ALLOCATABLE`, which is **12** — not the ~32 a
//! register-count guess suggests. Measured: 12 arguments spill 0, 14 spill 2,
//! 16 spill 4, 20 spill 8. Linear in `n − ALLOCATABLE`, which is why this
//! fixture can pick its depth.
//!
//! # And the assertion is the ANSWER, not the spill count
//!
//! `num_spills >= 2` alone would pass on an allocator that spilled correctly
//! and a displacement that was wrong — the two are independent, and the second
//! is the one no test reaches. So this runs the image and checks the arithmetic:
//! `सहाय k` answers `k + 1`, sixteen of them sum to 152, and a local addressed
//! at the wrong displacement reads a spill slot instead of its own word and
//! answers something else.
//!
//! **THE FIXTURE IS AN INLINE SOURCE AND NOT A FILE IN
//! `crates/sadhana-t1/src/`.** A `.t1` there joins the CORPUS: it would move the
//! 17,249-access census and the band counts, redding the pins that motivate this
//! test, change the `t1_boot` stub tally, and oblige a fixpoint round. Nothing
//! about spill displacement needs the corpus to grow.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::chain::Front;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::regalloc::allocate_registers;
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};
use yantra::Machine;

const FUEL: u64 = 80_000_000_000;

/// Arguments to `बहुपर`. **16, not 14**: 14 is the first depth that spills at
/// all (`14 - 12 = 2`) and sitting on a threshold means an allocator that
/// changed its accounting by one would silently stop testing anything. 16 gives
/// `num_spills = 4` — comfortably past the corpus's ceiling of 1, and still
/// small enough to read.
const ARGS: usize = 16;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn devanagari(n: usize) -> String {
    n.to_string()
        .chars()
        .map(|c| match c {
            '0' => '०',
            '1' => '१',
            '2' => '२',
            '3' => '३',
            '4' => '४',
            '5' => '५',
            '6' => '६',
            '7' => '७',
            '8' => '८',
            '9' => '९',
            c => c,
        })
        .collect()
}

/// `बहुपर` takes `ARGS` parameters and sums them; `मुख्यम्` calls it with
/// `ARGS` CALLS as its arguments, so every one of those results is live at the
/// same moment and the allocator must spill `ARGS - ALLOCATABLE` of them.
fn source() -> String {
    let params = (1..=ARGS)
        .map(|i| format!("प{} ॱॱ न६४", devanagari(i)))
        .collect::<Vec<_>>()
        .join(" ऽ ");
    let sum = (1..=ARGS)
        .map(|i| format!("प{}", devanagari(i)))
        .collect::<Vec<_>>()
        .join(" योगः ");
    let args = (1..=ARGS)
        .map(|i| format!("सहाय {}", devanagari(i)))
        .collect::<Vec<_>>()
        .join(" ऽ ");
    format!(
        "मण्डलम् दाब ॥\n\n\
         सार्वजनिक वृत्तिः सहाय आदाय क ॱॱ न६४ ददाति न६४ आदि प्रत्यागमनम् क योगः १ । इति\n\n\
         सार्वजनिक वृत्तिः बहुपर आदाय {params} ददाति न६४ आदि\n    प्रत्यागमनम् {sum} ।\nइति\n\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    प्रत्यागमनम् बहुपर आरभ्य {args} समाप्तम् ।\nइति\n"
    )
}

/// `सहाय k` answers `k + 1`, so the sum over `1..=ARGS` is `Σ(k+1)`.
fn want() -> u64 {
    (1..=ARGS as u64).map(|k| k + 1).sum()
}

/// The deepest spill region any routine in this source needs, laid out by the
/// product's own two functions and not by a second arithmetic here.
fn deepest_spill(src: &str) -> usize {
    let mut front = Front::load(&spec_root()).expect("the front end loads");
    front.lex(src).expect("lexes");
    front.parse().expect("parses");
    front.resolve().expect("resolves");
    front.typecheck().expect("typechecks");
    front.build_ir().expect("builds IR");
    let module = front.module("दाब", None).expect("the module builds");
    module
        .functions
        .iter()
        .map(|f| {
            let alloc = allocate_registers(f, riscv64::ALLOCATABLE);
            riscv64::frame_layout(&alloc, riscv64::count_locals(f)).num_spills
        })
        .max()
        .unwrap_or(0)
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

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
fn a_frame_with_several_spill_slots_still_addresses_its_locals_correctly() {
    let src = source();

    // ── FIRST, that the fixture reaches a depth the corpus never does. If this
    //    fails the test below proves nothing about displacement, so it is
    //    asserted separately and BEFORE the answer.
    let spills = deepest_spill(&src);
    assert!(
        spills >= 2,
        "the fixture's deepest frame uses {spills} spill slots, and the corpus \
         already reaches 1 (`deepest_spill == 1` in t1_corpus_conditionals.rs). \
         At {spills} this test adds no coverage. `ALLOCATABLE` is {}, and the \
         measured law is `num_spills = ARGS - ALLOCATABLE` — if that changed, \
         raise ARGS (currently {ARGS}) until this passes",
        riscv64::ALLOCATABLE
    );

    // ── AND THE INTERPRETER'S ANSWER, which knows nothing about frames. It is
    //    the control for the arithmetic itself: if the fixture is wrong, this
    //    reds before any claim about spilling is in question.
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push(("दाब.t1", &src));
    let mut it = Interpreter::load(&srcs, &spec_root()).expect("the chain plus the fixture loads");
    let interpreted = it
        .call("दाबॱमुख्यम्", vec![], FUEL)
        .expect("the fixture runs under the interpreter")
        .as_int()
        .expect("it answers a number") as u64;
    assert_eq!(
        interpreted,
        want(),
        "the INTERPRETER summed {interpreted} where Σ(k+1) for k in 1..={ARGS} \
         is {} — the fixture is wrong before either engine is in question",
        want()
    );

    // ── THEN the compiled code, which does have a frame and does spill.
    let image = build(&src, "दाब");
    assert!(!image.is_empty(), "the fixture built no image");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let status = match m.run(200_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => u64::from(s),
        other => panic!("the fixture did not finish: {other:?}"),
    };

    assert_eq!(
        status,
        want() % 65536,
        "with {spills} spill slots in the frame, the machine answered {status} \
         where the sum is {} ({} in the sixteen bits a finisher carries). The \
         interpreter agrees with the arithmetic, so this is the COMPILED side: a \
         local addressed at the wrong displacement reads a spill slot instead of \
         its own word. See `riscv64.rs:406` — local `k` lives at \
         `8·(num_spills + k)`, and that term is 0 for every routine the corpus \
         exercises past depth 1",
        want(),
        want() % 65536
    );
}
