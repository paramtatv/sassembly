//! `W-306` — THE RELAXATION'S FAR JUMP AT A `पर्व` LABEL THE TEXT NEVER DECLARES, AND A
//! WALK THAT MUST GET PAST IT.
//!
//! `यन्त्रशाखादूरपरीक्षा` (`yantrotsarjana.t1:1718`) does not answer two things about a
//! recorded `लङ्घनम्`; it answers three:
//!
//! | `यन्त्रपर्वस्थानकोश[लक्ष्यम्]` | distance   | the walk           |
//! |--------------------------------|------------|--------------------|
//! | a line number, in reach        | measured   | ACCEPTS            |
//! | a line number, out of reach    | measured   | REFUSES, naming it |
//! | `ऋण१` — no label ever written  | NOT taken  | ACCEPTS, SKIPPING  |
//!
//! The third is deliberate and the `.t1` says so at `:1736`: it is Rust's
//! `line_of_label.get(target)` miss, a target with no label line in THIS text being not
//! this routine's distance to measure. The guard that makes it true is one word — `यदि
//! लक्ष्यस्थानम् अधिकम् ऋण१` at `:1738`.
//!
//! **WHAT WAS ALREADY COVERED, AND WHAT THIS FILE ADDS — MEASURED, NOT ARGUED.** Two
//! mutations of `yantrotsarjana.t1`, each run against all four jump targets in release
//! (the `.t1` is read at run time, so no rebuild is involved):
//!
//! | mutation of the J-type walk                          | undeclared | measured | range | block |
//! |------------------------------------------------------|------------|----------|-------|-------|
//! | `:1738` `अधिकम् ऋण१` → `ऋण२`: the sentinel IS an address | 1 red  | green    | 1 red | 1 red |
//! | the skip RETURNS `सत्यम्` instead of advancing `लङ्घनाङ्कः` | **1 red** | green | green | green |
//!
//! The first row is why this file does NOT exist to pin the skip itself: `:1738` is
//! already held by `t1_jump_block_target::a_block_with_no_address_yet_is_skipped_and_not_refused`
//! and `t1_jump_range_bound::a_target_with_no_address_yet_is_skipped_and_not_refused`.
//! The second row is this file's reason to exist, and it is caught by nothing else in the
//! tree. Two things those files cannot do:
//!
//! **ONE: THE ROW COMES FROM THE LOWERING.** Both files above hand-seed
//! `यन्त्रलङ्घनलक्ष्यकोश` and never run `यन्त्रशाखावतरणम्`, so neither says that the
//! relaxation's far jump — the one `.t1` line at `:1554` — records a target that MAY have
//! no label, nor that the pair accepts when it does not. Here nothing on the jump's side
//! of any subtraction is chosen: `then_at` and `else_at`, two rows of
//! `यन्त्रपर्वस्थानकोश`, are the only numbers this test picks.
//!
//! **TWO: THE WALK MUST GET PAST A SKIPPED ROW TO A LATER ONE.** Both files above seed a
//! list of exactly ONE row, so "the walk goes on" can only mean "to the `विश्रम्भ`
//! return". A walk that answered `सत्यम्` at the sentinel instead of advancing
//! `लङ्घनाङ्कः` is green in both of them and green in every case here but
//! [`the_walk_gets_past_a_skipped_row_and_refuses_the_one_behind_it`], which fires BOTH
//! arms so the list is `(२६२२०१, तदा)` then `(२६२२०२, अन्यत्)`, leaves `तदा` undeclared
//! and puts `अन्यत्` one word out of reach. The refusal has to come from row `२`, which
//! the walk only reaches by continuing through row `१`'s skip.
//!
//! **A TWO-STATE INSTRUMENT OVER A THREE-STATE TRUTH IS THE FAILURE THIS ROW KEEPS
//! FINDING,** so `skipped` and `accepted` are not allowed to give the same answer here.
//! `t1_relaxed_jump_measured.rs` lowers with `यन्त्राज्ञागणना ४०`, where `४ × (ऋण१ − ४१)`
//! is `−१६८` — well inside ±१ MiB, so a sentinel read as an address would accept there
//! too and the two states would be indistinguishable. This fixture puts the routine
//! `२६२२००` lines along before lowering, so the relaxed arm records at `२६२२०१` and the
//! sentinel-as-address distance is `४ × (ऋण१ − २६२२०१)` = `−१०४८८०८`, OUT of reach by
//! fifty-eight words. The correct walk skips and accepts; a walk missing `:1738` refuses
//! and names `लक्ष्य २` at `−१०४८८०८`. The verdict alone separates them.
//!
//! **THE CASE THAT MUST STILL BE REFUSED** is the same lowering with `तदा`'s row WRITTEN
//! at the distance the undeclared row would have sat at: `४ × (५२४३४५ − २६२२०१)` =
//! `+१०४८५७६`, the first a J-type `लङ्घनम्` does not reach. **AND THE CASE THAT MUST
//! STILL BE ACCEPTED** is that row one word nearer, `+१०४८५७२`. One table row apart,
//! three fixtures answer accepted, refused, accepted — the three states spelled out.
//!
//! `recorded` is asserted in every case, so a skip is never confused with an absence:
//! acceptance for want of a ROW is
//! `t1_relaxed_jump_measured::the_near_arm_records_nothing_so_the_far_label_is_never_measured`;
//! this file is acceptance with the row present and its ADDRESS absent.
//!
//! THIS TEST'S OWN LOADER, as every `W-306` file here has its own.

use sadhana::t1::nirvahana::{Interpreter, Value};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// `lex.t1` carries the spec table every set reaches, and the emitter reads the IR enums
/// and the output buffer, so `ir.t1` and `utsarjana.t1` come with it.
fn loaded() -> Interpreter {
    let names = ["lex.t1", "ir.t1", "utsarjana.t1", "yantrotsarjana.t1"];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

fn set_int(it: &mut Interpreter, name: &str, v: i128) {
    assert!(
        it.set_global(name, Value::Int(v)),
        "`{name}` is a global the loaded corpus declares"
    );
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global of the loaded corpus"))
}

fn global_bool(it: &Interpreter, name: &str) -> bool {
    match it.global(name) {
        Some(Value::Bool(b)) => *b,
        other => panic!("`{name}` is a बूल global, not {other:?}"),
    }
}

/// Install one `अङ्कः अन्तः अ६४` address table WHOLE: an out-of-range arena READ is a
/// `RunError` where a WRITE resizes, and the walk READS this one.
fn set_table(it: &mut Interpreter, name: &str, entries: &[i128]) {
    let rows = entries.iter().copied().map(Value::Int).collect::<Vec<_>>();
    assert!(
        it.set_global(name, Value::Arena(Rc::new(RefCell::new(rows)))),
        "`{name}` is an address table the loaded corpus declares"
    );
}

/// `Refusal::JumpOutOfRange`'s ordinal, read from the corpus rather than spelled here: a
/// test that pinned the number would go green against a renumbered enum.
fn jump_out_of_range() -> i128 {
    global_int(&loaded(), "यन्त्रदूरलङ्घननिषेधभेद")
}

/// The line the routine is already at when the conditional is lowered. Chosen so that
/// `ऋण१` READ AS AN ADDRESS is out of reach — see the header.
const ALREADY: i128 = 262_200;
/// …so the relaxed arm's far `लङ्घनम्` is recorded here: the inverted conditional is
/// exactly one line (`संयोज्यम् ०` takes the `शर्तम्` arm, and `यन्त्रपठनम् ०` answers
/// location `०` — कोष्ठ `०`, no load emitted).
const JUMP_AT: i128 = ALREADY + 1;
/// The near arm's `लङ्घनम्`, when it fires: CONSECUTIVE with the far one, because the
/// skip label between the two lines advances no address.
const ELSE_JUMP_AT: i128 = JUMP_AT + 1;
/// `४ × (FAR − JUMP_AT)` = `+१०४८५७६`, the first distance a J-type does not reach.
const FAR: i128 = JUMP_AT + 262_144;
/// `+१०४८५७२`, the last it does.
const NEAR: i128 = FAR - 1;
/// The same first-unreachable distance measured from row `२`'s OWN address.
const ELSE_FAR: i128 = ELSE_JUMP_AT + 262_144;
/// What `ऋण१` would measure as if `:1738`'s guard were dropped: `−१०४८८०८`, out of reach
/// by fifty-eight words. CHECKED AT COMPILE TIME, because it is a property of the
/// fixture's own numbers and not of a run — if [`ALREADY`] is ever made small enough that
/// the sentinel read as an address would be IN reach, the three-state separation below
/// collapses silently and every test here still passes.
const SENTINEL_AS_ADDRESS: i128 = 4 * (-1 - JUMP_AT);
const _: () = assert!(
    SENTINEL_AS_ADDRESS < -1_048_576,
    "the fixture is only three-state if ऋण१-as-an-address is OUT of a J-type's reach"
);

struct Walked {
    recorded: i128,
    ok: bool,
    refused: bool,
    kind: i128,
    block: i128,
    target: i128,
    distance: i128,
}

/// Lower ONE conditional of block `१` — condition `०`, target block `२` (`तदा`), else
/// block `७` (`अन्यत्`) — in mode `relaxed`, with the routine already [`ALREADY`] lines
/// long; then walk the list IT left, with `तदा`'s row of `यन्त्रपर्वस्थानकोश` set to
/// `then_at`, `अन्यत्`'s to `else_at`, and every other row `ऋण१`.
///
/// `next_block` is `अग्रिमम्`: equal to `अन्यत्` the near arm stays silent and the list
/// holds ONE row; different and it holds two. THOSE THREE NUMBERS ARE THE ONLY ONES THIS
/// TEST CHOOSES — every address on the jump side comes from the lowering.
fn lower_then_walk(next_block: i128, then_at: i128, else_at: i128) -> Walked {
    let mut it = loaded();

    assert_eq!(
        global_int(&it, "यन्त्रलङ्घनसंख्या"),
        0,
        "the jump list starts EMPTY and this test never seeds it — every row the walk \
         reads below was written by the lowering"
    );

    set_int(&mut it, "यन्त्राज्ञागणना", ALREADY);
    // index १ is block १'s slot; ऋण१ is `no conditional line recorded yet`.
    set_table(&mut it, "यन्त्रशाखास्थानकोश", &[-1, -1]);

    it.call(
        "यन्त्रोत्सर्जनॱयन्त्रशाखावतरणम्",
        vec![
            Value::Int(1),          // शाखापर्वम् — the BRANCHING block
            Value::Int(0),          // शर्तम् — location ०, no load
            Value::Int(2),          // तदा
            Value::Int(7),          // अन्यत्
            Value::Int(next_block), // अग्रिमम्
            Value::Int(0),          // संयोज्यम् — no folded comparison
            Value::Bool(true),      // विश्रम्भ — the relaxed pass
        ],
        5_000_000,
    )
    .unwrap_or_else(|e| panic!("`यन्त्रशाखावतरणम्` runs: {e}"));

    // The block table as the driver fills it when the labels go out (`:2228`): ऋण१
    // everywhere a label has NOT been written.
    let mut blocks = vec![-1i128; 10];
    blocks[2] = then_at;
    blocks[7] = else_at;
    set_table(&mut it, "यन्त्रपर्वस्थानकोश", &blocks);
    // no epilogue written, and no row here targets it — ० is the only target that would.
    set_int(&mut it, "यन्त्रनिर्गमस्थानम्", -1);

    let recorded = global_int(&it, "यन्त्रलङ्घनसंख्या");

    // विश्रम्भ सत्यम् for the WALK in every case: the J-type half runs before the
    // विश्रम्भ return, and an accepted walk must not fall into the B-type half, which
    // reads मध्यरूपॱवृत्तिकोश this fixture built no routine in.
    let ans = it
        .call(
            "यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा",
            vec![Value::Int(0), Value::Bool(true)],
            50_000_000,
        )
        .unwrap_or_else(|e| panic!("`यन्त्रशाखादूरपरीक्षा` runs: {e}"));

    Walked {
        recorded,
        ok: matches!(ans, Value::Bool(true)),
        refused: global_bool(&it, "यन्त्रनिषेधमस्ति"),
        kind: global_int(&it, "यन्त्रनिषेधभेद"),
        block: global_int(&it, "यन्त्रनिषेधपर्व"),
        target: global_int(&it, "यन्त्रनिषेधलक्ष्य"),
        distance: global_int(&it, "यन्त्रनिषेधसंख्या"),
    }
}

/// THE THIRD STATE, FROM THE LOWERING'S OWN ROW. `तदा`'s label was never written, so its
/// row is `ऋण१`, and the walk does not measure the row it nonetheless holds: it SKIPS it
/// and accepts. The fixture says WHICH of the two acceptances this is, because reading
/// `ऋण१` as an address here is `−१०४८८०८` — out of reach — and would refuse.
#[test]
fn a_far_jump_to_an_undeclared_label_is_skipped_and_not_refused() {
    let w = lower_then_walk(7, -1, -1);

    assert_eq!(
        w.recorded, 1,
        "the relaxed arm DID record its far jump — this is a skip, not an absence"
    );
    assert!(
        w.ok,
        "`यदि लक्ष्यस्थानम् अधिकम् ऋण१` (`:1738`): a target with no label line in this \
         text is not this routine's distance to measure — Rust's `line_of_label` miss"
    );
    assert!(
        !w.refused,
        "and NOTHING is recorded. A walk that read ऋण१ as an address would refuse here \
         at {SENTINEL_AS_ADDRESS}, which is how this case is told from an accepted \
         MEASUREMENT and not merely from a `सत्यम्`"
    );
}

/// THE CASE THAT MUST STILL BE REFUSED — the same list, the same recorded address, the
/// same target block, and `तदा`'s row WRITTEN at the distance the undeclared row would
/// have sat at. Without it the acceptance above would hold of a walk that refuses
/// nothing.
#[test]
fn the_same_row_with_the_label_declared_out_of_reach_is_refused() {
    let w = lower_then_walk(7, FAR, -1);

    assert_eq!(w.recorded, 1, "the same one row the undeclared case held");
    assert!(
        !w.ok,
        "४ × (५२४३४५ − २६२२०१) = +१०४८५७६, the first distance a J-type `लङ्घनम्` does \
         not reach"
    );
    assert!(w.refused, "the refusal is RECORDED, not merely returned");
    assert_eq!(
        w.kind,
        jump_out_of_range(),
        "यन्त्रदूरलङ्घननिषेधभेद and not another भेद"
    );
    assert_eq!(
        w.target, 2,
        "लक्ष्य names तदा's block — the target the LOWERING stored into the row the \
         undeclared case left unmeasured"
    );
    assert_eq!(
        w.block, 0,
        "पर्व ० — a J-type refusal carries no block, as the Rust variant carries none"
    );
    assert_eq!(
        w.distance, 1_048_576,
        "measured from the lowering's own recorded address ({JUMP_AT})"
    );
}

/// THE CASE THAT MUST STILL BE ACCEPTED, AND BY MEASUREMENT RATHER THAN BY SKIP — the row
/// written one word nearer. Three fixtures differing in ONE table row answer `accepted`,
/// `refused`, `accepted`, which is the three states spelled out.
#[test]
fn the_same_row_with_the_label_declared_in_reach_is_accepted() {
    let w = lower_then_walk(7, NEAR, -1);

    assert_eq!(w.recorded, 1, "the same one row, measured this time");
    assert!(
        w.ok,
        "४ × (५२४३४४ − २६२२०१) = +१०४८५७२, the last distance that stands"
    );
    assert!(!w.refused, "nothing is recorded for a jump that reaches");
}

/// THE WALK MUST GET PAST A SKIPPED ROW. Both arms fire (`अग्रिमम् असमम् अन्यत्`), so the
/// list is `(२६२२०१, तदा)` then `(२६२२०२, अन्यत्)`. `तदा` is UNDECLARED — row `१` is
/// skipped — and `अन्यत्` is one word out of reach, so the refusal can only come from row
/// `२`. A walk that answered `सत्यम्` at the sentinel instead of advancing `लङ्घनाङ्कः`
/// accepts here, and is green in every one-row fixture: `t1_jump_block_target` and
/// `t1_jump_range_bound` each seed exactly ONE row, so "the walk goes on" means only
/// "to the `विश्रम्भ` return" in both.
#[test]
fn the_walk_gets_past_a_skipped_row_and_refuses_the_one_behind_it() {
    let w = lower_then_walk(9, -1, ELSE_FAR);

    assert_eq!(w.recorded, 2, "the far jump and the near one, two rows");
    assert!(!w.ok, "row २ is out of reach even though row १ was skipped");
    assert!(w.refused, "and the refusal is kept");
    assert_eq!(w.kind, jump_out_of_range(), "यन्त्रदूरलङ्घननिषेधभेद");
    assert_eq!(
        w.target, 7,
        "अन्यत्'s block — the walk did not STOP at row १, it skipped it and went on"
    );
    assert_eq!(
        w.distance, 1_048_576,
        "measured from row २'s OWN address ({ELSE_JUMP_AT}), not row १'s ({JUMP_AT}), \
         which would read १०४८५८०"
    );
}
