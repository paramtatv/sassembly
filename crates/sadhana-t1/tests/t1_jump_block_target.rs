//! `W-306` — THE J-TYPE AIMED AT A **BLOCK**, WHICH NEITHER TWIN'S TESTS TOOK.
//!
//! `यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा`'s J-type walk reads its target TWO ways
//! (`yantrotsarjana.t1:1734`–`:1737`):
//!
//! ```text
//! चरः लक्ष्यस्थानम् ॱॱ अ६४ भवति यन्त्रनिर्गमस्थानम् ।
//! यदि लङ्घनलक्ष्यम् अधिकम् ० आदि
//!     लक्ष्यस्थानम् भवति यन्त्रपर्वस्थानकोश अङ्कः लङ्घनलक्ष्यम् अन्तः ।
//! इति
//! ```
//!
//! `लङ्घनलक्ष्यम् ०` means the exit label and falls through to `यन्त्रनिर्गमस्थानम्`;
//! anything above `०` is a BLOCK ID and takes the second line. `t1_jump_range_bound.rs`
//! seeds target `०` for every one of its seven cases, so that second line had NEVER
//! been evaluated by a test — and it is the exact shape of the keying defect `W-306`
//! found in `यन्त्रशाखास्थानकोश`: a table read with the wrong index answers an address
//! that belongs to another row and certifies a far jump as near. Rust's side was blind
//! the same way: `jumping_function` spells `परीक्षानिर्गम` and every J-type corner in
//! `riscv64.rs` targets the exit label, so a block-keyed rewrite of that read would
//! have gone green on both sides.
//!
//! ALL FOUR CORNERS GO THROUGH THE BLOCK READ HERE, and the bound is the same
//! half-open `[ऋण२^२०, +२^२०)` the exit-label corners pinned — the point of this file is
//! not a new bound but a new PATH to the one that exists:
//!
//! | distance    | verdict  |
//! |-------------|----------|
//! | `+१०४८५७६`  | REFUSED  |
//! | `+१०४८५७२`  | ACCEPTED |
//! | `ऋण१०४८५८०` | REFUSED  |
//! | `ऋण१०४८५७६` | ACCEPTED |
//!
//! **THE FIXTURE IS A FALSIFIER FOR TWO MIS-READS AT ONCE, and that is why the ids
//! disagree.** The jump is ordinal `१` and its target is block `२`, so
//! `यन्त्रपर्वस्थानकोश` is installed WHOLE with a DECOY at index `१` — the jump's own
//! address, distance `०`, in range — and the real label at index `२`. And
//! `यन्त्रनिर्गमस्थानम्` is seeded to that same in-range address. So:
//!
//! * reading the table by `लङ्घनाङ्कः` (the jump ordinal) instead of `लङ्घनलक्ष्यम्`
//!   answers the decoy, and the two REFUSED corners go green — caught.
//! * dropping the `यदि लङ्घनलक्ष्यम् अधिकम् ०` override, so every jump measures against
//!   the exit label, answers the same in-range `०` — caught the same way.
//!
//! A test that seeded one address for everything could not tell either mutation from
//! the correct read.
//!
//! **THE CASE THAT MUST STILL BE SKIPPED, AND IT IS PINNED FROM FAR DOWN THE TEXT.**
//! `ऋण१` in `यन्त्रपर्वस्थानकोश` is a MISS and not an address: the block carries no line
//! yet. [`a_block_with_no_address_yet_is_skipped_and_not_refused`] records its jump at
//! line `२६२१४४` rather than `०` precisely so that a walk which measured the miss anyway
//! would see `ऋण१०४८५८०` and REFUSE. Recorded at line `०` that same mis-measure is
//! `ऋण४` — in range, green, and no test at all. The four corners cannot pin this guard,
//! because each of them has a real address to measure.
//!
//! THIS TEST'S OWN LOADER, as every test here has its own.

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

/// Install one `अङ्कः अन्तः अ६४` address table WHOLE, index `i` holding `entries[i]`.
/// Installed whole and not appended to, because an out-of-range arena READ is a
/// `RunError` where a WRITE resizes — `08dfdcdf` reverted a test red on exactly that.
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

/// Record ONE J-type `लङ्घनम्` going out at line `at`, aimed at BLOCK `२`, with block
/// `२`'s label at line `label`, then walk it in mode `relaxed`.
///
/// The distance the walk measures is `४ × (label − at)` octets, and it must reach that
/// through `यन्त्रपर्वस्थानकोश अङ्कः लङ्घनलक्ष्यम्` — index `२` — and through nothing else.
/// Index `१` of the table and `यन्त्रनिर्गमस्थानम्` both hold `at`, the DECOY: distance
/// `०` and in range, so either mis-read shows up as an acceptance.
fn walk(at: i128, label: i128, relaxed: bool) -> Verdict {
    let mut it = loaded();

    // advance-then-write: the writer bumps यन्त्रलङ्घनसंख्या first and writes row n,
    // copying यन्त्राज्ञागणना into row n's address. So the count is never set by hand.
    set_int(&mut it, "यन्त्राज्ञागणना", at);
    let row = it
        .call("यन्त्रोत्सर्जनॱयन्त्रलङ्घनलेखः", vec![Value::Int(2)], 1_000_000)
        .unwrap_or_else(|e| panic!("`यन्त्रलङ्घनलेखः` runs: {e}"));
    assert_eq!(row.as_int(), Some(1), "the first row of a 1-based arena");
    assert_eq!(
        global_int(&it, "यन्त्रलङ्घनसंख्या"),
        1,
        "one jump recorded and no more"
    );
    assert_eq!(
        global_int(&it, "यन्त्रनिर्गमस्थानम्"),
        -1,
        "no epilogue written yet, so the DECOY below is this test's and not the corpus's"
    );

    // The two mis-reads this fixture is a falsifier for, both pointing at `at`.
    set_int(&mut it, "यन्त्रनिर्गमस्थानम्", at);
    set_table(&mut it, "यन्त्रपर्वस्थानकोश", &[-1, at, label]);

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
        block: global_int(&it, "यन्त्रनिषेधपर्व"),
        target: global_int(&it, "यन्त्रनिषेधलक्ष्य"),
        distance: global_int(&it, "यन्त्रनिषेधसंख्या"),
    }
}

/// `Refusal::JumpOutOfRange`'s ordinal, read from the corpus rather than pinned here:
/// a test that spelled `११` would go green against a renumbered enum.
fn jump_out_of_range() -> i128 {
    global_int(&loaded(), "यन्त्रदूरलङ्घननिषेधभेद")
}

/// THE TWO OUT-OF-REACH CORNERS, THROUGH THE BLOCK READ, IN BOTH MODES. The refusal
/// carries `लक्ष्य २` here and not the `०` every exit-label corner reports, so the
/// record names the BLOCK the jump could not reach.
#[test]
fn a_jump_to_a_block_past_one_mib_is_refused_in_both_modes() {
    let kind = jump_out_of_range();
    // ४ × (२६२१४४ − ०) = +१०४८५७६ forward, ४ × (० − २६२१४५) = ऋण१०४८५८० back: not the
    // same magnitude, because the reach is half-open at the positive end.
    for (at, label, distance) in [(0, 262_144, 1_048_576), (262_145, 0, -1_048_580)] {
        for relaxed in [false, true] {
            let v = walk(at, label, relaxed);
            assert!(
                !v.ok,
                "a jump of {distance} octets to block २ is out of the J-type's ±1 MiB \
                 reach (relaxed = {relaxed})"
            );
            assert!(v.refused, "the refusal is RECORDED, not merely returned");
            assert_eq!(
                v.kind, kind,
                "यन्त्रदूरलङ्घननिषेधभेद and not another भेद (relaxed = {relaxed})"
            );
            assert_eq!(
                v.block, 0,
                "पर्व ० — a J-type refusal carries no BRANCHING block, as Rust's variant carries none"
            );
            assert_eq!(
                v.target, 2,
                "लक्ष्य २ names the TARGET BLOCK, which no exit-label corner can report"
            );
            assert_eq!(
                v.distance, distance,
                "measured through यन्त्रपर्वस्थानकोश अङ्कः २ and not through the decoy at १ \
                 or यन्त्रनिर्गमस्थानम् (relaxed = {relaxed})"
            );
        }
    }
}

/// THE OTHER SIDE OF THE BOUND, FOUR OCTETS IN. Without these a guard that refused
/// every block-targeted jump would pass the test above.
///
/// `विश्रम्भ सत्यम्`: an ACCEPTED J-type walk falls THROUGH into the B-type half, which
/// reads `मध्यरूपॱवृत्तिकोश` for a routine this fixture emitted no block of — so the
/// accepted corners are taken in the relaxed mode, where that half returns first.
#[test]
fn a_jump_to_a_block_four_octets_inside_the_bound_is_accepted() {
    // ४ × (२६२१४३ − ०) = +१०४८५७२ forward, ४ × (० − २६२१४४) = ऋण१०४८५७६ back — the
    // INCLUSIVE end of a half-open reach, and the corner a symmetric guard gets wrong.
    for (at, label) in [(0, 262_143), (262_144, 0)] {
        let v = walk(at, label, true);
        assert!(
            v.ok,
            "४ × ({label} − {at}) octets to block २ is INSIDE the J-type's ±1 MiB reach"
        );
        assert!(!v.refused, "nothing is recorded for a jump that reaches");
    }
}

/// `ऋण१` IN THE BLOCK TABLE IS A MISS AND NOT AN ADDRESS — block `२` carries no line
/// yet — so this jump has no distance to measure, and the `यदि लक्ष्यस्थानम् अधिकम् ऋण१`
/// guard is what keeps it from being measured anyway.
///
/// THE JUMP IS PUT FAR DOWN THE TEXT ON PURPOSE. A miss recorded at line `०` would
/// measure `४ × (ऋण१ − ०)` = `ऋण४` and go green against a walk with no guard at all; at
/// line `२६२१४४` the same mis-measure is `४ × (ऋण१ − २६२१४४)` = `ऋण१०४८५८०` and REFUSES.
/// So relaxing `अधिकम् ऋण१` to anything below `ऋण१` reds this case — which the four
/// corners cannot do, because each of them has a real address to measure.
#[test]
fn a_block_with_no_address_yet_is_skipped_and_not_refused() {
    let v = walk(262_144, -1, true);
    assert!(
        !v.refused,
        "a block with no address is not this routine's distance to measure"
    );
    assert!(v.ok, "and the walk goes on to the विश्रम्भ return");
}
