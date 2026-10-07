//! **`D-003`, the single-machine half: does the emitted octet depend on hash order?**
//!
//! `D-003` asks for three absences — no time, no paths, no hash order — and its
//! own row says the first readings were BOUNDED SEARCHES AND NOT PROOFS OF
//! ABSENCE. The weakest of the three is hash order: the row records *"103
//! HashMap/HashSet mentions in crates/sadhana/src; a grep for iteration over
//! them under src/t1/ returned nothing, WHICH IS A BOUNDED SEARCH AND NOT AN
//! ABSENCE"*. A grep cannot settle it, because iteration over a map wears a
//! dozen spellings (`.iter()`, `.keys()`, `.values()`, `for x in &map`,
//! `.into_iter()`, `.drain()`, `.retain()`) and none of them says `HashMap` on
//! the line that does the damage.
//!
//! So this does not grep. It makes the hash order MOVE and asks whether the
//! octets moved with it.
//!
//! # Why repeating the emit inside one process is a hash-order probe
//!
//! `std::collections::hash_map::RandomState::new` draws its keys from a
//! thread-local pair and INCREMENTS one of them per instance. Two `HashMap`s
//! built in the same process from the same insertions therefore have different
//! hashers and generally iterate in different orders. Assembling the same
//! source twice in one process is thus not a tautology: every map on the path is
//! freshly seeded the second time round.
//!
//! That argument is the whole load-bearing claim of this file, and an argument
//! is exactly what this project does not accept. So `the_hasher_reseeds_per_instance`
//! below MEASURES it, and is the reason the assertions underneath are not
//! vacuous. If a future toolchain or a workspace-wide fixed-seed hasher makes
//! per-instance reseeding stop, that control goes RED and says the probe has
//! lost its teeth — rather than letting the determinism assertions keep passing
//! for the wrong reason. **An instrument with two states where the truth has
//! three hides its own breakage**; the third state here is *probe disarmed*.
//!
//! # The case that must still be refused
//!
//! `the_comparator_sees_a_single_flipped_octet` asserts that the comparison
//! these tests do would actually FAIL on a difference. Without it, a comparator
//! that compared lengths, or compared nothing, would report agreement over
//! every corpus file and read as proof.
//!
//! # BOTH PROBES WERE PROVEN RED BY INJECTION, SEPARATELY
//!
//! A probe whose subject contains no hash iteration at all is green for the
//! wrong reason, and the reseeding control above cannot tell that apart from a
//! real absence. So each assertion was driven RED by a hash-order leak injected
//! into the function IT calls, and reverted:
//!
//! - `assemble_object`'s last octet set from `HashMap::keys().next()` →
//!   `the_emit_is_byte_identical_under_a_moved_hash_order` RED at octet 623 on
//!   `01-sum-of-two.sas`; the image probe stayed green, correctly, because
//!   `assemble` does not route through it.
//! - `assemble`'s last octet, likewise → `the_linked_image_is_byte_identical_too`
//!   RED (`34` against `52`); the object probe stayed green.
//!
//! Each probe is therefore sensitive to its own path and to nothing else.
//!
//! **A THIRD INJECTION MEASURED SOMETHING ELSE AND IS WORTH THE LINE.** The
//! first attempt PUSHED a hash-ordered octet onto `kosha::object`'s output
//! inside `assemble`, before `vastu::read`. Every test stayed green — because
//! `vastu::read` parses the object by its header and DISCARDS a trailing octet
//! it has no section for, so the leak never reached the image. That is a real
//! property of the path and not a probe failure: octets appended past the last
//! section are not part of the object, and a determinism claim about
//! `kosha::object`'s raw return is therefore a weaker claim than one about what
//! `vastu::read` admits. These tests make the stronger claim, about the bytes
//! that are actually read back.
//!
//! # WHAT THE AUDIT FOUND BY CONTENT, NOT BY GREP WIDTH
//!
//! The row counted 103 `HashMap`/`HashSet` mentions in `crates/sadhana/src` and
//! could not say which iterate. Narrowed to the modules `assemble` and
//! `assemble_object` actually call — `lex`, `parse`, `encode`, `kosha`,
//! `vastu`, `samyojana`, `dwarf`, `nidana` — there is **exactly ONE** hash
//! container, `parse.rs:829`'s `dict` for the `जाल` directive, and it is
//! `insert` then `get`: never iterated, so its order cannot reach an octet.
//! The other 102 mentions are under `src/t1/`, which is the self-hosting
//! chain's Rust side and not this path. That is why these probes are green,
//! and the number to watch is ONE.
//!
//! # What this does NOT settle
//!
//! The row's title says *two machines* and this session has one. This covers
//! the single-machine half only: same binary, same host, varying hash seeds.
//! Cross-host (endianness, pointer width, locale, filesystem order) is untouched
//! and the row stays open for it.

use sadhana::encode::Target;
use sadhana::nidana::Language;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// This test's own loader. A new test gets its own; it does not borrow one.
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/golden")
}

/// Every hand-written golden program, name and text, in sorted order so the
/// report reads the same twice.
fn sources() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = std::fs::read_dir(golden_dir())
        .expect("spec/golden is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sas"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("a golden source is readable");
            (p.file_name().unwrap().to_string_lossy().into_owned(), text)
        })
        .collect();
    v.sort();
    assert!(
        v.len() >= 30,
        "the golden corpus is the input to this probe and it has shrunk to {} \
         files; a probe over an empty corpus agrees with everything",
        v.len()
    );
    v
}

/// The control the two assertions below rest on: are two fresh `HashMap`s in
/// this process actually seeded differently?
///
/// Built from identical insertions, so any difference in iteration order is the
/// hasher and nothing else. 64 keys makes an accidental agreement of two
/// independent orders vanishingly unlikely while staying instant.
#[test]
fn the_hasher_reseeds_per_instance() {
    fn order() -> Vec<u32> {
        let mut m: HashMap<u32, u32> = HashMap::new();
        for k in 0..64u32 {
            m.insert(k, k);
        }
        m.keys().copied().collect()
    }

    // Several draws, because two draws agreeing by chance is possible and five
    // all agreeing is not.
    let first = order();
    let differs = (0..5).any(|_| order() != first);

    assert!(
        differs,
        "PROBE DISARMED: five freshly built HashMaps with identical insertions \
         all iterated in the SAME order, so this process does not reseed per \
         instance. Repeating an emit in one process therefore does NOT vary \
         hash order, and `the_emit_is_byte_identical_under_a_moved_hash_order` \
         below proves nothing about D-003's third absence. Fix the probe (fork \
         a child process per emit) before trusting the green."
    );
}

/// The refused case: the comparator has to be able to see a difference.
#[test]
fn the_comparator_sees_a_single_flipped_octet() {
    let (name, text) = sources().into_iter().next().expect("a golden source");
    let good = sadhana::assemble_object(
        &text,
        Some("d003"),
        Target::Uncompressed,
        false,
        Language::Sanskrit,
    )
    .unwrap_or_else(|d| panic!("{name} assembles: {d:?}"));

    assert!(!good.is_empty(), "an object is not empty");

    let mut bad = good.clone();
    let last = bad.len() - 1;
    bad[last] ^= 0x01;

    assert_ne!(
        good, bad,
        "the comparator cannot distinguish two objects differing in one bit, so \
         every agreement it reports is worthless"
    );
    assert_eq!(
        good.len(),
        bad.len(),
        "the flip changed a value, not a length"
    );
}

/// The assertion. Each source assembled three times in this one process —
/// three independently seeded sets of maps — must give the same octets.
#[test]
fn the_emit_is_byte_identical_under_a_moved_hash_order() {
    let mut compared = 0usize;
    let mut skipped: Vec<String> = Vec::new();

    for (name, text) in sources() {
        let first = match sadhana::assemble_object(
            &text,
            Some("d003"),
            Target::Uncompressed,
            false,
            Language::Sanskrit,
        ) {
            Ok(b) => b,
            // A source this chain refuses is not evidence either way about hash
            // order; it is counted and named, not silently dropped.
            Err(_) => {
                skipped.push(name);
                continue;
            }
        };

        for round in 1..3 {
            let again = sadhana::assemble_object(
                &text,
                Some("d003"),
                Target::Uncompressed,
                false,
                Language::Sanskrit,
            )
            .unwrap_or_else(|d| {
                panic!("{name} assembled once and refused on round {round}: {d:?}")
            });

            assert_eq!(
                first.len(),
                again.len(),
                "{name}: round {round} produced {} octets against {} on round 0",
                again.len(),
                first.len()
            );
            let at = first.iter().zip(&again).position(|(a, b)| a != b);
            assert!(
                at.is_none(),
                "{name}: round {round} differs from round 0 at octet {} — the \
                 emitted object depends on hash iteration order",
                at.unwrap()
            );
        }
        compared += 1;
    }

    // NO SILENT CAP: say what was not covered.
    assert!(
        compared >= 30,
        "only {compared} of the golden corpus assembled, {} refused ({:?}); the \
         floor is 30 and a probe over fewer is a smaller claim than the one this \
         test's name makes",
        skipped.len(),
        skipped
    );
    println!(
        "D-003 single-machine: {compared} sources, 3 emits each, byte-identical; {} refused",
        skipped.len()
    );
}

/// The same question one level up, for the whole linked image rather than the
/// object: link-time layout is where an iteration order would most plausibly
/// leak into an address.
#[test]
fn the_linked_image_is_byte_identical_too() {
    let mut compared = 0usize;
    for (name, text) in sources() {
        let Ok(first) =
            sadhana::assemble(&text, Target::Uncompressed, 0x8000_0000, Language::Sanskrit)
        else {
            continue;
        };
        let again = sadhana::assemble(&text, Target::Uncompressed, 0x8000_0000, Language::Sanskrit)
            .unwrap_or_else(|d| panic!("{name} linked once and refused on the second: {d:?}"));
        assert_eq!(
            first.len(),
            again.len(),
            "{name}: the image is {} octets on the second link against {} on the first",
            again.len(),
            first.len()
        );
        // The offset, not the two vectors: a 300-octet dump on both sides buries
        // the one octet that moved.
        let at = first.iter().zip(&again).position(|(a, b)| a != b);
        assert!(
            at.is_none(),
            "{name}: the linked image differs at octet {} within one process",
            at.unwrap()
        );
        compared += 1;
    }
    assert!(
        compared >= 30,
        "only {compared} images were compared; the floor is 30"
    );
    println!("D-003 single-machine: {compared} images, byte-identical");
}
