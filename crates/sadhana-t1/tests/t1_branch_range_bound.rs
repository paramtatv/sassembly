//! `W-306` — THE B-TYPE CONDITIONAL'S ±4 KiB REACH IN THE `.t1` TWIN, FROM BOTH SIDES.
//!
//! `यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा`'s SECOND half — the one past the `विश्रम्भ` return —
//! walks a routine's blocks and raises `यन्त्रदूरशाखानिषेधभेद` when a `शाखावसान` sits
//! further than ±२^१२ octets from its target's label. `t1_jump_range_bound.rs` pins the
//! FIRST half (the J-type `लङ्घनम्`, ±1 MiB) on all four corners; this half had none. Its
//! two comparisons — `दूरम् न्यूनम् ऋण४०९६` (`:1775`) and `दूरम् बृहत्समम् ४०९६` (`:1778`) —
//! were free to be wrong by any amount in either direction with every `.t1` test green,
//! and the twin is where the self-host actually runs, so a bound off by one word HERE is
//! the wrap Rust's corners no longer permit.
//!
//! | `शाखास्थानम्` | `चिह्नस्थानम्` | `दूरम्`    | verdict  |
//! |---------------|----------------|------------|----------|
//! | `०`           | `१०२४`         | `+४०९६`    | REFUSED  |
//! | `०`           | `१०२३`         | `+४०९२`    | ACCEPTED |
//! | `१०२५`        | `०`            | `ऋण४१००`   | REFUSED  |
//! | `१०२४`        | `०`            | `ऋण४०९६`   | ACCEPTED |
//!
//! **THE TWO SIDES STOP AT DIFFERENT MAGNITUDES, AND THAT IS NOT SYMMETRY MISSED.** The
//! pair is HALF-OPEN — `न्यूनम् ऋण४०९६` is strict, `बृहत्समम् ४०९६` is not — so `ऋण४०९६`
//! STANDS while `+४०९६` is already out, and the first refused going back is `ऋण४१००`, one
//! word further than forward's `+४०९६`. That is Rust's `!(-4096..4096).contains(&bytes)`
//! exactly (`riscv64.rs`, `check_branch_ranges`) and RISC-V's 13-bit signed B-type reach
//! exactly. A test written symmetric would assert `ऋण४०९६` REFUSED and be WRONG, and the
//! two accepting corners are what make that impossible to spell: a guard that refused
//! every conditional passes a refusal-only test, and so does one whose bound is short.
//!
//! **BOTH ENDS OF THE MEASUREMENT ARE ASSERTED, NOT JUST THE DISTANCE.** `यन्त्रनिषेधपर्व`
//! carries `पर्वाङ्कः` — the BRANCHING block — and `यन्त्रनिषेधलक्ष्य` carries `तदा`, its
//! target. They are DIFFERENT block ids here (१ branches to २), which is the whole of what
//! `W-306` corrected: `यन्त्रशाखास्थानकोश` was keyed by the TARGET, so two blocks branching
//! to one join both read the first one's address and every later branch to that join was
//! certified near. A fixture whose branch and target shared an id could not tell the two
//! keyings apart.
//!
//! **`विश्रम्भ` असत्यम् IS REQUIRED AND IS THE POINT OF THE SPLIT.** This half returns
//! `सत्यम्` unread the moment the relaxed mode is set — correctly, because a relaxed
//! conditional branches over exactly one instruction by construction. So unlike the J-type
//! corners there is no both-modes loop here; the relaxed mode is asserted to reach this
//! half NOT AT ALL, on both refused distances, which is Rust's
//! `a_relaxed_conditional_is_not_measured` for the `.t1` side.
//!
//! THE FIXTURE. `ऋण१` IS A MISS AND NOT AN ADDRESS in either table — a block with no label
//! line written, or one that wrote no conditional — and the walk tests `अधिकम् ऋण१` on each
//! before measuring. Both misses are set at a distance that WOULD be refused were it
//! measured, so neither can pass by being short.
//!
//! The two address tables are `अङ्कः अन्तः अ६४` keyed by block id, written by
//! `यन्त्रवृत्त्युत्सर्जनम्`'s own pass (`yantrotsarjana.t1:2208`, `:2228`) as the lines go
//! out. This test installs them whole rather than running an emit: the walk reads exactly
//! two entries from them and nothing writes them back, so the fixture is four `.t1` calls
//! and two tables, with no register allocation and no output buffer in the way.

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

/// THIS TEST'S OWN LOADER, and it is the same set `t1_jump_range_bound.rs` needs:
/// `lex.t1` carries the spec table every set reaches, and the emitter reads the IR
/// arenas and the output buffer, so `ir.t1` and `utsarjana.t1` come with it.
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

fn call(it: &mut Interpreter, name: &str, args: Vec<Value>) -> Value {
    it.call(name, args, 5_000_000)
        .unwrap_or_else(|e| panic!("`{name}` runs: {e}"))
}

/// Install one `अङ्कः अन्तः अ६४` address table whole, index `i` holding `entries[i]`.
/// This is what `यन्त्रवृत्त्युत्सर्जनम्`'s pass leaves behind — `ऋण१` for every block
/// it wrote no line for, the line count for every one it did.
fn set_table(it: &mut Interpreter, name: &str, entries: &[i128]) {
    let rows = entries.iter().copied().map(Value::Int).collect::<Vec<_>>();
    assert!(
        it.set_global(name, Value::Arena(Rc::new(RefCell::new(rows)))),
        "`{name}` is an address table the loaded corpus declares"
    );
}

/// What one walk answers: the verdict, and the refusal record if one was kept.
struct Verdict {
    ok: bool,
    refused: bool,
    kind: i128,
    block: i128,
    target: i128,
    distance: i128,
}

/// Build a one-block routine whose block `१` ends in a `शाखावसान` aimed at block `२`,
/// record block २'s label at line `label` and block १'s conditional at line `branch`,
/// then walk it in mode `relaxed`.
///
/// The distance the walk measures is `४ × (label − branch)` octets. `ऋण१` in either
/// position is the MISS and not an address.
fn walk(branch: i128, label: i128, relaxed: bool) -> Verdict {
    let mut it = loaded();

    assert_eq!(
        global_int(&it, "यन्त्रलङ्घनसंख्या"),
        0,
        "no J-type row, so the FIRST half of the walk measures nothing and falls through"
    );

    // Some(Terminator::CondBranch(c, १, ०)) — the condition is read by nothing this
    // walk does, and अन्यलक्ष्यम् ० is `falls through`.
    let cond = call(&mut it, "मध्यरूपॱमूल्याङ्कनम्", vec![Value::Int(0)]);
    let then = call(&mut it, "मध्यरूपॱपर्वाङ्कनम्", vec![Value::Int(2)]);
    let otherwise = call(&mut it, "मध्यरूपॱपर्वाङ्कनम्", vec![Value::Int(0)]);
    let term = call(&mut it, "मध्यरूपॱशाखावसानरचना", vec![cond, then, otherwise]);

    // पर्वयोजनम् is KEYED and not an appender: it writes पर्वकोश at अङ्कन ॱ क्रमाङ्क,
    // so this block is id १ and no other id is live.
    let id = call(&mut it, "मध्यरूपॱपर्वाङ्कनम्", vec![Value::Int(1)]);
    let written = call(
        &mut it,
        "मध्यरूपॱपर्वयोजनम्",
        vec![id, Value::Int(0), Value::Int(0), term],
    );
    assert_eq!(written.as_int(), Some(1), "block १ and no other");

    // वृत्तियोजनम् IS an appender — advance-then-write on a 1-based arena — so the
    // index it returns is the one the walk is asked for.
    let entry = call(&mut it, "मध्यरूपॱपर्वाङ्कनम्", vec![Value::Int(1)]);
    let func = call(
        &mut it,
        "मध्यरूपॱवृत्तियोजनम्",
        vec![Value::Int(0), Value::Int(1), Value::Int(1), entry],
    );
    let func = func.as_int().expect("वृत्तियोजनम् answers an index");
    assert_eq!(func, 1, "the first row of a 1-based arena");

    // index ० of each table is no live block — पर्वयोजनम् hands out ids from १ — and
    // ऋण१ is what the pass leaves for a block it wrote no line of.
    set_table(&mut it, "यन्त्रपर्वस्थानकोश", &[-1, -1, label]);
    set_table(&mut it, "यन्त्रशाखास्थानकोश", &[-1, branch]);

    let ans = call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा",
        vec![Value::Int(func), Value::Bool(relaxed)],
    );

    Verdict {
        ok: matches!(ans, Value::Bool(true)),
        refused: global_bool(&it, "यन्त्रनिषेधमस्ति"),
        kind: global_int(&it, "यन्त्रनिषेधभेद"),
        block: global_int(&it, "यन्त्रनिषेधपर्व"),
        target: global_int(&it, "यन्त्रनिषेधलक्ष्य"),
        distance: global_int(&it, "यन्त्रनिषेधसंख्या"),
    }
}

/// `Refusal::BranchOutOfRange`'s ordinal, read from the corpus rather than pinned here:
/// a test that spelled `१०` would go green against a renumbered enum.
fn branch_out_of_range() -> i128 {
    global_int(&loaded(), "यन्त्रदूरशाखानिषेधभेद")
}

/// THE TWO OUT-OF-REACH CORNERS, AND THEY ARE NOT THE SAME MAGNITUDE. Forward the first
/// refused is `+४०९६`; backward it is `ऋण४१००`, one word further out, because the pair is
/// half-open. A guard spelled `ऋण४०९२..४०९६` or `ऋण४०९६..=४०९६` refuses or accepts one of
/// these two and is caught here.
#[test]
fn the_b_type_conditional_is_refused_past_four_kib_on_both_sides() {
    let kind = branch_out_of_range();
    for (branch, label, distance) in [(0, 1_024, 4_096), (1_025, 0, -4_100)] {
        let v = walk(branch, label, false);
        assert!(
            !v.ok,
            "a conditional of {distance} octets is outside the B-type's ±4 KiB reach"
        );
        assert!(v.refused, "the refusal is RECORDED, not merely returned");
        assert_eq!(
            v.kind, kind,
            "यन्त्रदूरशाखानिषेधभेद and not another भेद ({distance})"
        );
        assert_eq!(v.block, 1, "पर्व names the BRANCHING block (W-306)");
        assert_eq!(v.target, 2, "लक्ष्य names its TARGET, a different id");
        assert_eq!(
            v.distance, distance,
            "the refusal carries the distance it measured, sign and all"
        );
    }
}

/// THE OTHER SIDE OF THE BOUND — one word inside it, and accepted. Without these two a
/// guard that refused every conditional would pass the test above, and `ऋण४०९६` is the
/// corner a symmetric reading of the pair gets wrong: it is the INCLUSIVE end of a
/// half-open reach, where `+४०९६` is already out.
#[test]
fn the_b_type_conditional_is_accepted_at_the_half_open_endpoints() {
    for (branch, label, distance) in [(0, 1_023, 4_092), (1_024, 0, -4_096)] {
        let v = walk(branch, label, false);
        assert!(
            v.ok,
            "{distance} octets is INSIDE the B-type's ±4 KiB reach and must stand"
        );
        assert!(
            !v.refused,
            "nothing is recorded for a conditional that reaches"
        );
    }
}

/// **THE RELAXED FORM REACHES THIS HALF NOT AT ALL**, and it is asserted on the two
/// distances that ARE refused unrelaxed, so the test cannot pass by the distance being
/// short. `विश्रम्भ` सत्यम् means the conditional was already inverted over a `लङ्घनम्` and
/// branches over exactly one instruction — eight octets — by construction; the far target
/// moved to the J-type, which the FIRST half of this routine measures in both modes.
#[test]
fn a_relaxed_routine_s_conditional_is_not_measured() {
    for (branch, label, distance) in [(0, 1_024, 4_096), (1_025, 0, -4_100)] {
        let v = walk(branch, label, true);
        assert!(
            v.ok,
            "{distance} octets is not measured once the routine is relaxed"
        );
        assert!(
            !v.refused,
            "and nothing is recorded: the walk returns before reading a block"
        );
    }
}

/// `ऋण१` IS A MISS AND NOT AN ADDRESS, in EITHER table and for different reasons. A target
/// whose label line is not written yet is not this routine's distance to measure
/// (`यन्त्रपर्वस्थानकोश`); neither is a block that wrote no conditional at all
/// (`यन्त्रशाखास्थानकोश`) — the `W-245` fused form writes one line where the unfused writes
/// two, and the second pass rewinds both tables to `ऋण१` before re-walking.
///
/// Both misses sit at a distance that WOULD be refused were `ऋण१` taken for an address:
/// `४ × (ऋण१ − १०२५)` is `ऋण४१०४` and `४ × (१०२४ − ऋण१)` is `+४१००`.
#[test]
fn a_block_with_no_address_yet_is_skipped_and_not_refused() {
    for (branch, label) in [(1_025, -1), (-1, 1_024)] {
        let v = walk(branch, label, false);
        assert!(
            !v.refused,
            "ऋण१ is a miss, not an address (branch = {branch}, label = {label})"
        );
        assert!(v.ok, "and the walk goes on to the end of the block run");
    }
}
