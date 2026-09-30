//! **THE ALLOCATOR SPILLS, AND THE SPILLED VALUES COME BACK.**
//!
//! # THE CEILING THIS FIXTURE CLEARS IS READ FROM THE CENSUS, NOT REMEMBERED
//!
//! This file was written against a corpus ceiling of **1**: a probe on
//! 2026-09-25 found the only spilling routines were the 17 copies of
//! `खण्डवृद्धिः` that `ir.t1:645` synthesises once per module, one slot each,
//! and `t1_corpus_conditionals.rs` pinned `deepest_spill == 1`. `ARGS = 16`
//! was chosen to sit at `num_spills = 4`, "comfortably past" that ceiling.
//!
//! **THAT CEILING IS FALSIFIED. Measured 2026-09-29, the corpus answers 6** —
//! depths `{0: 848, 1: 17, 2: 1, 5: 6, 6: 2}` over 874 routines reached by an
//! SP access, deepest `encode.t1: सङ्केतनसङ्केतसूचीरचना`. The probe was not
//! wrong about its own population (depth-1 is still exactly 17, the
//! `खण्डवृद्धिः` family, unmoved); it simply never saw nine other routines in
//! three deeper buckets. So for some window this fixture was asserting `>= 2`
//! against a corpus already at 6 and adding **no depth coverage at all**,
//! while its own margin said it did.
//!
//! The repair is not a bigger number, it is a NAMED one: `CORPUS_DEEPEST`
//! below carries the census pin, `ARGS` is sized to clear it by the measured
//! law, and the guard compares against it. When the corpus deepens again, the
//! census reds first (it pins an EQUALITY) and this fixture's guard reds
//! second — instead of silently sliding under the thing it exists to exceed.
//!
//! So `8·(num_spills + k)` — the displacement `riscv64.rs:406` puts every
//! `W-245` local behind — is exercised by the corpus only BY ACCIDENT, at
//! whatever depth the compiler happens to need that week. A defect that first
//! appears past that depth is invisible to every test this repository owns,
//! and the census's populated `spill` band does not say otherwise: a band
//! being non-empty is not a statement about its depth.
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
//! A depth guard alone — `num_spills > CORPUS_DEEPEST` — would pass on an
//! allocator that spilled correctly and a displacement that was wrong; the two
//! are independent, and the second is the one no test reaches. So this runs the
//! image and checks the arithmetic: `सहाय k` answers `k + 1`, twenty of them
//! sum to 230, and a local addressed at the wrong displacement reads a spill
//! slot instead of its own word and answers something else.
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

/// The deepest spill region the CORPUS reaches, carried across from
/// `t1_corpus_conditionals.rs`'s `deepest_spill` pin (measured 2026-09-29 over
/// 20 modules; that pin is an equality, so it reds when this number goes
/// stale). This fixture exists to address a frame DEEPER than anything the
/// corpus reaches, so this is the floor it must clear — not `1`, and not a
/// constant chosen once and left.
const CORPUS_DEEPEST: usize = 6;

/// Arguments to `बहुपर`. The measured law is `num_spills = ARGS -
/// ALLOCATABLE`, and `riscv64::ALLOCATABLE` is 12 — so `ARGS` picks the depth
/// directly.
///
/// **20, giving `num_spills = 8`.** Was 16 (`num_spills = 4`), chosen when the
/// corpus ceiling was believed to be 1; at a real ceiling of
/// `CORPUS_DEEPEST` = 6 that added no depth coverage. 20 clears 6 with two
/// slots to spare — enough that a one-slot change in the allocator's
/// accounting does not put this fixture back under the corpus, and still small
/// enough to read. Sitting ON the ceiling would mean an allocator that changed
/// its accounting by one silently stops testing anything, which is the same
/// mistake the original `14` note warned about at the other threshold.
const ARGS: usize = 20;

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
    source_of(ARGS)
}

/// The same fixture at an arbitrary arity, so the guard below can be shown to
/// REFUSE a shallow one rather than only to accept the deep one.
fn source_of(n: usize) -> String {
    let params = (1..=n)
        .map(|i| format!("प{} ॱॱ न६४", devanagari(i)))
        .collect::<Vec<_>>()
        .join(" ऽ ");
    let sum = (1..=n)
        .map(|i| format!("प{}", devanagari(i)))
        .collect::<Vec<_>>()
        .join(" योगः ");
    let args = (1..=n)
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

    // THE DEPTH ITSELF, PRINTED. The guard below is an INEQUALITY, so passing
    // it proves only `> CORPUS_DEEPEST` and hides whether the measured law
    // (`num_spills = ARGS - ALLOCATABLE`) still holds or the fixture drifted
    // to one slot above the floor. An instrument with two states where the
    // truth has many hides its own breakage, so the number is reported beside
    // the verdict and a later cycle can read the margin without re-deriving it.
    println!("METRIC t1_spill_fixture_num_spills {spills}");
    println!("METRIC t1_spill_fixture_corpus_deepest {CORPUS_DEEPEST}");
    println!(
        "fixture ARGS {ARGS}, ALLOCATABLE {}, num_spills {spills}, corpus \
         deepest {CORPUS_DEEPEST}, margin {}",
        riscv64::ALLOCATABLE,
        spills as isize - CORPUS_DEEPEST as isize
    );

    assert!(
        spills > CORPUS_DEEPEST,
        "the fixture's deepest frame uses {spills} spill slots and the CORPUS \
         already reaches {CORPUS_DEEPEST} (`deepest_spill` in \
         t1_corpus_conditionals.rs, measured 2026-09-29). At {spills} this \
         test adds NO DEPTH COVERAGE — which is exactly the state it was in \
         while it asserted `>= 2` against a corpus at 6. `ALLOCATABLE` is {}, \
         and the measured law is `num_spills = ARGS - ALLOCATABLE`: raise ARGS \
         (currently {ARGS}) past `ALLOCATABLE + CORPUS_DEEPEST` = {}. If the \
         census pin itself moved, move CORPUS_DEEPEST with it FIRST — this \
         guard is only as honest as that number",
        riscv64::ALLOCATABLE,
        usize::from(riscv64::ALLOCATABLE) + CORPUS_DEEPEST
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
         `8·(num_spills + k)`, and no routine the corpus exercises pushes that \
         term past {CORPUS_DEEPEST} slots",
        want(),
        want() % 65536
    );
}

/// **THE CASE THE GUARD MUST STILL REFUSE.**
///
/// `spills > CORPUS_DEEPEST` is an inequality, and an inequality that only
/// ever sees passing inputs is indistinguishable from `assert!(true)`. That is
/// not hypothetical here: this fixture spent some window asserting `>= 2`
/// against a corpus at 6 and reported itself green the whole time, because
/// nothing ever fed it a depth it was supposed to reject.
///
/// So the law is pinned at THREE points on the same front end the guard uses —
/// below the ceiling, exactly ON it, and above it — and the middle one is the
/// one that matters: at `ALLOCATABLE + CORPUS_DEEPEST` arguments the fixture
/// merely TIES the corpus, adds no depth coverage, and the guard must say no.
/// A guard written `>=` instead of `>` passes the deep case identically and
/// reds only here.
#[test]
fn the_depth_guard_refuses_a_fixture_that_only_ties_the_corpus() {
    let allocatable = usize::from(riscv64::ALLOCATABLE);

    for (n, want) in [
        (allocatable, 0),                               // below: does not spill at all
        (allocatable + CORPUS_DEEPEST, CORPUS_DEEPEST), // ON the ceiling
        (ARGS, ARGS - allocatable),                     // above: the fixture in use
    ] {
        let got = deepest_spill(&source_of(n));
        assert_eq!(
            got, want,
            "at {n} arguments the deepest frame uses {got} spill slots, not \
             {want}. The measured law is `num_spills = n - ALLOCATABLE` \
             (ALLOCATABLE = {allocatable}), clamped at 0; if the allocator's \
             accounting changed, ARGS and this table move together — and \
             CORPUS_DEEPEST does NOT, because it is the census's number"
        );

        // THE GUARD ITSELF, applied to the depth just measured.
        let accepted = got > CORPUS_DEEPEST;
        assert_eq!(
            accepted,
            n > allocatable + CORPUS_DEEPEST,
            "the guard {} a fixture of {n} arguments ({got} spill slots) \
             against a corpus ceiling of {CORPUS_DEEPEST}. Tying the ceiling \
             is NOT clearing it: at {got} slots the fixture exercises \
             `8·(num_spills + k)` only where the corpus already does",
            if accepted { "ACCEPTED" } else { "refused" }
        );
    }
}
