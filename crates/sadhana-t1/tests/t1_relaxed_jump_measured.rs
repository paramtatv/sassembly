//! `W-306` — THE LOWERING'S RECORDED FAR JUMP AND THE CHECKER'S MEASUREMENT, OVER ONE
//! INTERPRETER. The two halves were each proved alone and NOTHING proved they MEET.
//!
//! `t1_relaxed_jump_recorded.rs` (`91d1df4d`) drives `यन्त्रशाखावतरणम्`'s relaxed arm and
//! reads `यन्त्रलङ्घनसंख्या`/`…स्थानकोश`/`…लक्ष्यकोश` BACK — the WRITE is pinned and the
//! walk never runs. `t1_jump_range_bound.rs` and `t1_jump_block_target.rs` call
//! `यन्त्रशाखादूरपरीक्षा` over a list they seeded themselves — the MEASUREMENT is pinned
//! and no lowering ran. So the address the relaxation stores and the index the walk reads
//! could disagree by any amount and BOTH halves stay green: each one supplies the other's
//! missing side from its own hand. This file supplies neither. It lowers a relaxed
//! conditional, hand-seeds NO jump list at all, and then walks whatever the lowering left.
//!
//! **THE REACH IS MADE TO FAIL BY EXACTLY ONE WORD, FROM THE BLOCK TABLE AND NOT FROM THE
//! LIST.** The only number this test chooses is `यन्त्रपर्वस्थानकोश`'s row for `तदा` — the
//! line `तदा`'s label went out at, which the real driver fills at `yantrotsarjana.t1:2228`
//! as the labels are written. Everything on the jump's side of the subtraction comes from
//! the lowering: the relaxed arm records its far jump at line `४१` (`आरम्भः ४०`, the
//! inverted conditional makes it `४१`), so a label at `२६२१८५` is `४ × (२६२१८५ − ४१)` =
//! `+१०४८५७६` away, the first distance a J-type `लङ्घनम्` does NOT reach, and `२६२१८४` is
//! `+१०४८५७२`, the last it does. Neither number is written into the arena the writer owns.
//!
//! **THE REFUSAL MUST NAME `तदा`'s BLOCK, WHICH IS THE JOIN ITSELF.** `यन्त्रनिषेधलक्ष्य`
//! carries `लङ्घनलक्ष्यम्` — the row the LOWERING wrote — and the walk indexes
//! `यन्त्रपर्वस्थानकोश` with that same value. So `लक्ष्य` reading `२` is the two halves
//! agreeing on WHICH block the stored target names, and not merely that something was out
//! of range: the near arm at `:1565` writes `अन्यत्` into the same list, and a swap on
//! `:1554` would refuse at a distance measured to the WRONG label.
//!
//! **WHY THE CHECKER IS ALWAYS CALLED `विश्रम्भ सत्यम्`, EVEN WHERE THE LOWERING WAS NEAR.**
//! The J-type walk runs FIRST, before the `विश्रम्भ` return, and reads no IR — but an
//! accepted walk falls THROUGH into the B-type half, which reads `मध्यरूपॱवृत्तिकोश` for a
//! routine this fixture never built. The two modes are independent arguments to two
//! different routines, and [`the_near_arm_records_nothing_so_the_far_label_is_never_measured`]
//! uses that on purpose: a NEAR lowering with the same unreachable label must still be
//! ACCEPTED, because the near arm records no far jump for the walk to find. Without that
//! case a walk that refused on the block table alone — never reading the list — would pass
//! the refusal above.
//!
//! **EACH ROW'S ADDRESS GOES WITH ITS OWN TARGET.**
//! [`the_walk_pairs_each_rows_address_with_its_own_target`] fires BOTH arms, puts `तदा`'s
//! label in reach and `अन्यत्`'s one word out, and asserts the measured distance is
//! `१०४८५७६` and not `१०४८५८०`: the two recorded addresses are consecutive (`४१`, `४२`), so
//! a walk that paired row `२`'s target with row `१`'s address refuses all the same and
//! reports a distance four octets too large. Only the number tells them apart.
//!
//! **WHAT THIS FILE CATCHES THAT NO OTHER `W-306` FILE DOES, MEASURED AND NOT ARGUED.**
//! Five mutations of `yantrotsarjana.t1`, each run against all five `W-306` targets in
//! release (the `.t1` is read at run time, so no rebuild is involved):
//!
//! | mutation                                              | measured | recorded | range | block | branch |
//! |-------------------------------------------------------|----------|----------|-------|-------|--------|
//! | drop `:1554`'s `यन्त्रलङ्घनलेखः तदा`                  | 3 red    | 2 red    | green | green | green  |
//! | writer stores `यन्त्राज्ञागणना योगः १`                | 2 red    | 2 red    | 2 red | 2 red | green  |
//! | that writer `+१` AND the walk's read `वियोगः १`       | **green**| 2 red    | green | green | green  |
//! | `:1554` aimed at `अन्यत्` instead of `तदा`            | 2 red    | 2 red    | green | green | green  |
//! | the walk reads row `१`'s address for EVERY row        | **1 red**| green    | green | green | green  |
//!
//! The last row is this file's reason to exist: a walk that pairs every target with the
//! FIRST recorded address is invisible to both hand-seeded checker files (their fixtures
//! hold ONE row, so row `१`'s address IS every row's address) and invisible to the
//! lowering file (which never walks). Only
//! [`the_walk_pairs_each_rows_address_with_its_own_target`] reds, and it reds on the
//! DISTANCE — `१०४८५८०` where `१०४८५७६` was measured — because the verdict is `refused`
//! either way.
//!
//! **THE COMPENSATING PAIR IS THE OTHER WAY ROUND FROM WHAT WAS EXPECTED, AND IS WORTH
//! WRITING DOWN.** `W-306`'s plan predicted that a writer `+१` paid back by a walk `−१`
//! would leave both halves' own tests green and only the JOINED one red. It is the exact
//! opposite: the pair cancels inside the subtraction the walk performs, so THIS file stays
//! green, and the only target that reds is `t1_relaxed_jump_recorded`, which reads the
//! stored address back and compares it to a number. So the two files do not subsume each
//! other in either direction — a joined test cannot see an agreed-upon lie about WHICH
//! line a jump sits on, and a reading test cannot see a mispairing of rows. Both rows of
//! the table above are needed and neither is redundant.
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

/// Install one `अङ्कः अन्तः अ६४` address table WHOLE, because an out-of-range arena READ
/// is a `RunError` where a WRITE resizes — and the walk READS this one.
fn set_table(it: &mut Interpreter, name: &str, entries: &[i128]) {
    let rows = entries.iter().copied().map(Value::Int).collect::<Vec<_>>();
    assert!(
        it.set_global(name, Value::Arena(Rc::new(RefCell::new(rows)))),
        "`{name}` is an address table the loaded corpus declares"
    );
}

/// `Refusal::JumpOutOfRange`'s ordinal, read from the corpus rather than spelled `११`
/// here: a test that pinned the number would go green against a renumbered enum.
fn jump_out_of_range() -> i128 {
    global_int(&loaded(), "यन्त्रदूरलङ्घननिषेधभेद")
}

/// What one lowering-then-walk answered. The jump list is NOT part of this — the point is
/// that the test never touches it — but the recorded count IS, because a walk over an
/// empty list and a walk that measured and accepted both answer `सत्यम्`.
struct Joined {
    recorded: i128,
    ok: bool,
    refused: bool,
    kind: i128,
    block: i128,
    target: i128,
    distance: i128,
}

/// Lower ONE conditional of block `१` — condition `०`, target block `२` (`तदा`), else
/// block `७` (`अन्यत्`), falling into `अग्रिमम्` — with the routine already `४०`
/// instruction lines long, in mode `relaxed`; then walk the list IT left, with `तदा`'s
/// label at line `then_at` and `अन्यत्`'s at `else_at`.
///
/// `संयोज्यम् ०` takes the `शर्तम्` arm and `यन्त्रपठनम् ०` answers location `०`
/// (`utsarjana.t1:1103`) — कोष्ठ `०`, no load emitted — so the conditional is exactly ONE
/// line and `४१` below is exact. The output buffer is appended to and never read.
fn lower_then_walk(next_block: i128, relaxed: bool, then_at: i128, else_at: i128) -> Joined {
    let mut it = loaded();

    assert_eq!(
        global_int(&it, "यन्त्रलङ्घनसंख्या"),
        0,
        "the jump list starts EMPTY and this test never seeds it — every row the walk \
         reads below was written by the lowering"
    );

    set_int(&mut it, "यन्त्राज्ञागणना", 40);
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
            Value::Bool(relaxed),   // विश्रम्भ
        ],
        5_000_000,
    )
    .unwrap_or_else(|e| panic!("`यन्त्रशाखावतरणम्` runs: {e}"));

    // The block table, as the driver fills it when the labels go out (`:2228`): ऋण१
    // everywhere a label has not been written, and a line number for तदा and अन्यत्.
    // THIS IS THE ONLY SIDE OF THE SUBTRACTION THIS TEST CHOOSES.
    let mut blocks = vec![-1i128; 10];
    blocks[2] = then_at;
    blocks[7] = else_at;
    set_table(&mut it, "यन्त्रपर्वस्थानकोश", &blocks);
    // no epilogue written, and no row here targets it — ० is the only target that would.
    set_int(&mut it, "यन्त्रनिर्गमस्थानम्", -1);

    let recorded = global_int(&it, "यन्त्रलङ्घनसंख्या");

    // विश्रम्भ सत्यम् for the WALK in every case: the J-type half runs before the
    // विश्रम्भ return in both modes, and an accepted walk must not fall into the B-type
    // half, which reads मध्यरूपॱवृत्तिकोश this fixture built no routine in.
    let ans = it
        .call(
            "यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा",
            vec![Value::Int(0), Value::Bool(true)],
            50_000_000,
        )
        .unwrap_or_else(|e| panic!("`यन्त्रशाखादूरपरीक्षा` runs: {e}"));

    Joined {
        recorded,
        ok: matches!(ans, Value::Bool(true)),
        refused: global_bool(&it, "यन्त्रनिषेधमस्ति"),
        kind: global_int(&it, "यन्त्रनिषेधभेद"),
        block: global_int(&it, "यन्त्रनिषेधपर्व"),
        target: global_int(&it, "यन्त्रनिषेधलक्ष्य"),
        distance: global_int(&it, "यन्त्रनिषेधसंख्या"),
    }
}

/// THE JOINED REFUSAL. The relaxed arm records its far jump at line `४१`; `तदा`'s label
/// is at `२६२१८५`, one word beyond the J-type's reach; and the walk — given no list but
/// the one the lowering wrote — refuses, naming `तदा`'s block and the distance it
/// measured. This is the first assertion in `W-306` that the writer's address and the
/// walk's read are the SAME number.
#[test]
fn the_relaxations_own_far_jump_is_refused_one_word_out_of_reach() {
    let j = lower_then_walk(7, true, 262_185, -1);

    assert_eq!(
        j.recorded, 1,
        "the relaxed arm recorded exactly one jump — its far one; `अग्रिमम् समम् अन्यत्` \
         silences the near arm"
    );
    assert!(
        !j.ok,
        "४ × (२६२१८५ − ४१) = +१०४८५७६ octets, the first distance a J-type `लङ्घनम्` \
         does not reach"
    );
    assert!(j.refused, "the refusal is RECORDED, not merely returned");
    assert_eq!(
        j.kind,
        jump_out_of_range(),
        "यन्त्रदूरलङ्घननिषेधभेद and not another भेद"
    );
    assert_eq!(
        j.target, 2,
        "लक्ष्य names तदा's block — the target the LOWERING stored, and the index the \
         walk read यन्त्रपर्वस्थानकोश with; `अन्यत्` here would be the `:1554` swap"
    );
    assert_eq!(
        j.block, 0,
        "पर्व ० — a J-type refusal carries no block, as the Rust variant carries none"
    );
    assert_eq!(
        j.distance, 1_048_576,
        "the distance measured FROM the lowering's own recorded address (४१): ४२ would \
         read १०४८५७२ and accept"
    );
}

/// THE CASE THAT MUST STILL BE ACCEPTED — the same lowering, the label one word nearer.
/// Without it a walk that refused every J-type jump, or one whose bound is off in the
/// accepting direction, passes the refusal above.
#[test]
fn the_same_lowering_with_the_label_one_word_nearer_is_accepted() {
    let j = lower_then_walk(7, true, 262_184, -1);

    assert_eq!(j.recorded, 1, "the same one jump is recorded and measured");
    assert!(
        j.ok,
        "४ × (२६२१८४ − ४१) = +१०४८५७२ octets, the last distance that stands"
    );
    assert!(!j.refused, "nothing is recorded for a jump that reaches");
}

/// THE NEAR ARM RECORDS NOTHING, SO THE SAME UNREACHABLE LABEL IS NEVER MEASURED. Near
/// mode with `अग्रिमम् समम् अन्यत्`: the conditional carries its own target, neither
/// `यन्त्रलङ्घनलेखः` runs, and the walk has no row to measure — so `तदा`'s label at
/// `२६२१८५` is accepted. Without this case a walk that read the block table alone, never
/// touching the list, would refuse here too and still pass the refusal above.
#[test]
fn the_near_arm_records_nothing_so_the_far_label_is_never_measured() {
    let j = lower_then_walk(7, false, 262_185, -1);

    assert_eq!(
        j.recorded, 0,
        "no `लङ्घनम्` line was written, so no row belongs in the list"
    );
    assert!(
        j.ok,
        "the same label that refuses a RELAXED lowering is out of nobody's reach here"
    );
    assert!(
        !j.refused,
        "and nothing was recorded — the refusal above came from the relaxation's own \
         far jump and from no other source"
    );
}

/// EACH ROW'S ADDRESS GOES WITH ITS OWN TARGET. Both arms fire (`अग्रिमम् असमम् अन्यत्`),
/// so the list is `(४१, तदा)` then `(४२, अन्यत्)` — CONSECUTIVE, because the skip label
/// between the two `लङ्घनम्` lines advances no address. `तदा`'s label is put in reach and
/// `अन्यत्`'s one word out, so the walk must get PAST row `१` and then measure row `२`
/// against row `२`'s address: `४ × (२६२१८६ − ४२)` = `१०४८५७६`. A walk that paired row
/// `२`'s target with row `१`'s address refuses all the same and reports `१०४८५८०`, so
/// only the distance tells the two apart.
#[test]
fn the_walk_pairs_each_rows_address_with_its_own_target() {
    let j = lower_then_walk(9, true, 141, 262_186);

    assert_eq!(j.recorded, 2, "the far jump and the near one, two rows");
    assert!(!j.ok, "row २ is out of reach even though row १ is not");
    assert!(j.refused, "and the refusal is kept");
    assert_eq!(j.kind, jump_out_of_range(), "यन्त्रदूरलङ्घननिषेधभेद");
    assert_eq!(
        j.target, 7,
        "अन्यत्'s block — the walk did not stop at row १, which reaches (४ × (१४१ − ४१) \
         = ४००)"
    );
    assert_eq!(
        j.distance, 1_048_576,
        "measured from row २'s OWN address (४२), not row १'s (४१), which would read \
         १०४८५८०"
    );
}
