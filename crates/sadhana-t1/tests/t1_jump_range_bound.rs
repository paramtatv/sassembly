//! `W-306` — THE J-TYPE `लङ्घनम्`'s ±1 MiB REACH, MEASURED FROM BOTH SIDES OF THE BOUND.
//!
//! `यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा` gained a first half that walks the jump list and
//! raises `यन्त्रदूरलङ्घननिषेधभेद` when a J-type `लङ्घनम्` reaches further than ±२^२०
//! octets. That half runs BEFORE the `विश्रम्भ` return, so a relaxed routine is measured
//! for it too — which is the whole of what `08dfdcdf` changed.
//!
//! A GUARD TESTED ONLY ON ITS REFUSAL IS NOT TESTED. One that refuses everything passes
//! such a test, and so does one whose bound is off by any amount in the accepting
//! direction. So each of the four corners is asserted here:
//!
//! | distance    | verdict  |
//! |-------------|----------|
//! | `+१०४८५७६`  | REFUSED  |
//! | `+१०४८५७२`  | ACCEPTED |
//! | `ऋण१०४८५८०` | REFUSED  |
//! | `ऋण१०४८५७६` | ACCEPTED |
//!
//! and both refusals are asserted IN BOTH EMITTER MODES — `विश्रम्भ` सत्यम् and असत्यम् —
//! because the defect this replaced returned `Ok` the moment the relaxed mode was set.
//!
//! **THE BOUND IS HALF-OPEN, AND MEASURED SO — NOT ASSUMED.** This test was first written
//! asserting `ऋण१०४८५७६` REFUSED, symmetric with `+१०४८५७६`, and that assertion is WRONG:
//! the walk spells `न्यूनम् ऋण१०४८५७६` and `बृहत्समम् १०४८५७६`, so `ऋण१०४८५७६` STANDS and
//! `+१०४८५७६` does not. That is Rust's `!(-(1 << 20)..(1 << 20)).contains(&bytes)` exactly,
//! and it is RISC-V's J-type reach exactly — a 21-bit signed, 2-octet-aligned immediate
//! covers `ऋण२^२०` but not `+२^२०`. The twins agree on the asymmetry, and the asymmetry is
//! what an assumed-symmetric test would never have caught: Rust's own
//! `a_jump_past_one_mib_is_refused_in_both_modes` measures the POSITIVE side only, so until
//! this test neither half had the negative endpoint pinned at all.
//!
//! THE CASE THAT MUST STILL BE ACCEPTED is `यन्त्रनिर्गमस्थानम्` at `ऋण१` — no epilogue
//! written yet — with the jump recorded at line २६२१४४. `ऋण१` is the MISS and not an
//! address; a reader that took it for one would measure `४ × (ऋण१ − २६२१४४)` = `ऋण१०४८५८०`
//! and refuse a routine that is fine. This is Rust's `line_of_label.get(target)` returning
//! `None`, and it is why the walk tests `लक्ष्यस्थानम् अधिकम् ऋण१` before measuring.
//!
//! THE SEED. The jump list is written by `यन्त्रलङ्घनलेखः`, which is advance-then-write
//! on a 1-based arena: it bumps `यन्त्रलङ्घनसंख्या` first and writes row `n`. So the test
//! never sets the count by hand — it sets `यन्त्राज्ञागणना` (the line the jump goes out
//! at, which the writer copies into `यन्त्रलङ्घनस्थानकोश`) and calls the writer, exactly
//! as `यन्त्रलङ्घनम्` does. Target `०` means the exit label — `यन्त्रनिर्गमचिह्नम्`, which
//! no `पर्व` index spells — so the walk reads `यन्त्रनिर्गमस्थानम्` and no block arena is
//! touched. That keeps the fixture to four globals and no IR at all.

use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// THIS TEST'S OWN LOADER. `lex.t1` is in every set that reaches a spec table
/// (`पदविभागॱसमावेशपाठः` is declared there), and the emitter reads the IR arenas
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

/// `Refusal::JumpOutOfRange`'s ordinal, read from the corpus rather than pinned
/// here: a test that spelled `११` would go green against a renumbered enum.
fn jump_out_of_range(it: &Interpreter) -> i128 {
    global_int(it, "यन्त्रदूरलङ्घननिषेधभेद")
}

/// What one walk answers: the verdict, and the refusal record if one was kept.
struct Verdict {
    ok: bool,
    refused: bool,
    kind: i128,
    target: i128,
    distance: i128,
}

/// Record ONE J-type `लङ्घनम्` going out at line `at`, aimed at the exit label,
/// with the epilogue at line `exit`, then walk it in mode `relaxed`.
///
/// The distance the walk measures is `४ × (exit − at)` octets.
fn walk(at: i128, exit: i128, relaxed: bool) -> Verdict {
    let mut it = loaded();

    // advance-then-write: the writer copies यन्त्राज्ञागणना into row n's address.
    set_int(&mut it, "यन्त्राज्ञागणना", at);
    let row = it
        .call("यन्त्रोत्सर्जनॱयन्त्रलङ्घनलेखः", vec![Value::Int(0)], 1_000_000)
        .unwrap_or_else(|e| panic!("`यन्त्रलङ्घनलेखः` runs: {e}"));
    assert_eq!(row.as_int(), Some(1), "the first row of a 1-based arena");
    assert_eq!(
        global_int(&it, "यन्त्रलङ्घनसंख्या"),
        1,
        "one jump recorded and no more"
    );

    // ऋण१ = no epilogue yet, which is the MISS this walk must skip.
    set_int(&mut it, "यन्त्रनिर्गमस्थानम्", exit);

    let ans = it
        .call(
            "यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा",
            vec![Value::Int(0), Value::Bool(relaxed)],
            50_000_000,
        )
        .unwrap_or_else(|e| panic!("`यन्त्रशाखादूरपरीक्षा` runs: {e}"));

    Verdict {
        ok: matches!(ans, Value::Bool(true)),
        refused: global_bool(&it, "यन्त्रनिषेधमस्ति"),
        kind: global_int(&it, "यन्त्रनिषेधभेद"),
        target: global_int(&it, "यन्त्रनिषेधलक्ष्य"),
        distance: global_int(&it, "यन्त्रनिषेधसंख्या"),
    }
}

/// The two out-of-reach corners, in BOTH modes. `relaxed = true` is the case the
/// old code could not see at all: it returned `Ok` before ever reading the list.
#[test]
fn the_j_type_jump_is_refused_at_one_mib_in_both_modes() {
    let kind = jump_out_of_range(&loaded());
    // ४ × (२६२१४४ − ०) = +१०४८५७६, the first distance that does not stand going
    // FORWARD, and ४ × (० − २६२१४५) = ऋण१०४८५८०, the first going BACK. They are
    // not the same magnitude: the reach is [ऋण२^२०, +२^२०), so the negative end sits
    // one instruction further out than the positive one.
    for (at, exit, distance) in [(0, 262_144, 1_048_576), (262_145, 0, -1_048_580)] {
        for relaxed in [false, true] {
            let v = walk(at, exit, relaxed);
            assert!(
                !v.ok,
                "a jump of {distance} octets is out of the J-type's ±1 MiB reach \
                 (relaxed = {relaxed})"
            );
            assert!(v.refused, "the refusal is RECORDED, not merely returned");
            assert_eq!(
                v.kind, kind,
                "यन्त्रदूरलङ्घननिषेधभेद and not another भेद (relaxed = {relaxed})"
            );
            assert_eq!(v.target, 0, "लक्ष्य ० reports the exit label");
            assert_eq!(
                v.distance, distance,
                "the refusal carries the distance it measured (relaxed = {relaxed})"
            );
        }
    }
}

/// THE OTHER SIDE OF THE BOUND — one instruction short of it, and accepted. Without
/// these two a guard that refused every jump would pass the test above.
#[test]
fn the_j_type_jump_is_accepted_four_octets_inside_the_bound() {
    // ४ × (२६२१४३ − ०) = +१०४८५७२, the last that stands going forward, and
    // ४ × (० − २६२१४४) = ऋण१०४८५७६, the last going back — the INCLUSIVE end of a
    // half-open reach, and the corner a symmetric reading of this guard gets wrong.
    for (at, exit) in [(0, 262_143), (262_144, 0)] {
        // विश्रम्भ सत्यम्: an ACCEPTED J-type walk falls THROUGH into the B-type half,
        // which reads मध्यरूपॱपर्वकोश for a routine this fixture emitted no block of.
        let v = walk(at, exit, true);
        assert!(
            v.ok,
            "४ × ({exit} − {at}) octets is INSIDE the J-type's ±1 MiB reach"
        );
        assert!(!v.refused, "nothing is recorded for a jump that reaches");
    }
}

/// `ऋण१` IS A MISS AND NOT AN ADDRESS. The epilogue is not written yet, so this
/// routine's jump has no distance to measure — and a reader that measured anyway
/// would see `४ × (ऋण१ − २६२१४४)` = `ऋण१०४८५८०` and refuse.
#[test]
fn a_target_with_no_address_yet_is_skipped_and_not_refused() {
    let v = walk(262_144, -1, true);
    assert!(
        !v.refused,
        "a target with no address is not this routine's distance to measure"
    );
    assert!(v.ok, "and the walk goes on to the विश्रम्भ return");
}
