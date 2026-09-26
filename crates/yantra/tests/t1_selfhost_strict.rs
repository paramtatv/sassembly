//! **THE STRICT SELF-HOSTING FIGURE: NO SYMBOL FROM THE RUST DRIVER AT ALL.**
//!
//! The lenient figure allows borrowing the Rust driver's embedded corpus. This
//! one does not, and the difference is larger than it sounds: that constant is
//! not a list of filenames but `(name, include_str!(source))` pairs, so the
//! lenient arm takes THE ENTIRE CORPUS TEXT out of the Rust binary. Here the
//! sources are read off the FILESYSTEM by `Interpreter::load_paths`.
//!
//! **THE ABSENCE IS ASSERTED, NOT INTENDED.** This file and the shared ladder are
//! both checked for any mention of the Rust driver, by the test itself. An import
//! that creeps in later would quietly turn this into a duplicate of the lenient
//! figure — which is exactly how two ladders came to be read as one thing for
//! weeks, and the reason both figures now ship labelled.
//!
//! **THE FILE SET IS NOT FILTERED TO MATCH.** The corpus directory holds 20 `.t1`
//! files; the Rust driver names 18. If the extra two break the load, that is a
//! real difference between the two readings and must surface as `CouldNotRun` —
//! not be hidden by trimming the list until the answers agree.

mod hopladder;

use hopladder::{Reach, no_chain_reference_in, reach_with};
use sadhana::t1::nirvahana::Interpreter;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

fn source(name: &str) -> String {
    let p = corpus_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Every `.t1` in the corpus directory, read off DISK — the list comes from the
/// filesystem, which is the whole of what makes this arm strict.
fn corpus_paths() -> Vec<PathBuf> {
    let dir = corpus_dir();
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    v
}

#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus); t1_image is the product path and its smoke is this ladder's hop 5"]
fn the_strict_ladder_names_its_hop_and_reaches_no_rust_driver() {
    // THE CHECK THAT MAKES THE FIGURE MEAN WHAT IT SAYS, FIRST — before any
    // measurement, so a file that has drifted cannot report a number at all.
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t1_selfhost_strict.rs");
    let ladder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/hopladder/mod.rs");
    no_chain_reference_in(&here);
    no_chain_reference_in(&ladder);

    let paths = corpus_paths();
    println!("METRIC t1_selfhost_strict_disk_sources {}", paths.len());

    let build = || {
        let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        Interpreter::load_paths(&refs, &spec_root())
            .map_err(|e| format!("strict load of {} disk sources: {e:?}", refs.len()))
    };

    let ashtaka = reach_with(build, "अष्टक", &source("ashtaka.t1"));
    println!(
        "METRIC t1_selfhost_hops_strict_ashtaka {}",
        ashtaka.number()
    );
    println!("  strict ashtaka.t1 → {ashtaka:?}");

    // THE POSITIVE CONTROL, AND IT MUST LAND **LOWER** — a ladder answering the
    // same for a real module and a refused source measures nothing. `अज्ञातनाम`
    // is declared nowhere, so `अर्थ` refuses at resolve and no text is emitted.
    let control_src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् अज्ञातनाम ।\nइति\n";
    let build2 = || {
        let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        Interpreter::load_paths(&refs, &spec_root())
            .map_err(|e| format!("strict load of {} disk sources: {e:?}", refs.len()))
    };
    let control = reach_with(build2, "क", control_src);
    println!(
        "METRIC t1_selfhost_hops_strict_control {}",
        control.number()
    );
    println!("  strict control → {control:?}");

    assert_eq!(
        ashtaka,
        Reach::Hops(5),
        "ashtaka.t1 reached {ashtaka:?} on the STRICT ladder, not RUN. Two \
         independent witnesses stand behind hop 5 and a red means one moved: the \
         halt must not fault (measured Finisher 21845 = 0x5555, status 0) and the \
         loadable segment must equal Rust's octet for octet (measured 67210 = \
         67210, in `the_t1_image_is_octet_for_octet_the_rust_one`). Check which."
    );
    assert!(
        control.number() < ashtaka.number(),
        "control {control:?} did not land below ashtaka {ashtaka:?}"
    );
    assert_eq!(
        control,
        Reach::Hops(0),
        "the control must be refused at EMIT, not fail in the harness: {control:?}"
    );
}

/// The module a source declares, from its `मण्डलम् <name> ॥` header — or `None`
/// when it declares none.
///
/// `lib.t1` is the one that answers `None` today and it is deliberate: its own
/// margin says "Rust's crate root lists its modules, and T1 has no crate root".
/// **A source with no module is not a ladder rung that failed, it is a source
/// that was never on the ladder** — and the census below must report those two
/// as different observations or it manufactures a denominator.
fn declared_module(src: &str) -> Option<String> {
    src.lines()
        .map(str::trim_start)
        .find(|l| l.starts_with("मण्डलम्"))
        .and_then(|l| l.split_whitespace().nth(1))
        .map(ToString::to_string)
}

/// **THE TRACKED NUMBER, OVER THE WHOLE CORPUS** — `9394b9b3`, the owner's
/// ruling: *"the tracked number is sources compiling with `chain.rs` unreached"*.
///
/// The ruling recorded **0 of 20** on 2026-09-08 and the instrument to measure it
/// already existed — [`hopladder`], whose `no_chain_reference_in` enforces the
/// "unreached" half by reading this file and its own. **What was missing was a
/// census: the ladder had only ever been driven for `ashtaka.t1` and a control.**
/// This walks every source.
///
/// # The denominator is the finding, not the numerator
///
/// **`20` may not be reachable, and asserting it would repeat the error that made
/// `paradigm_encode_t1_assembled` read as stalled when it was SATURATED at 17 of
/// 17 compilable units.** Three sources are the usual reason: `lib.t1` declares no
/// module by design, and `ast.t1` and `vastu.t1` build no routines. **So this test
/// reports three populations and never merges them:**
///
/// * `NO MODULE`   — never on the ladder; excluded from the denominator
/// * `CouldNotRun` — the harness could not start; **not** a product reaching zero
/// * `Hops(n)`     — a real reading, `0` included
///
/// `Reach::number()` already encodes the second distinction as `-1` rather than
/// `0`, with the margin "a harness that cannot start must never be readable as a
/// product that reached nothing". This census keeps that separation in its output.
///
/// Set `HOP_LIMIT=<n>` to walk only the first `n` sources — the cost probe, because
/// the ladder reloads all twenty sources per source measured, so the run is
/// quadratic in corpus size and had never been paid more than once.
#[test]
#[ignore = "measurement: the strict ladder over every corpus source, minutes"]
fn measure_the_strict_ladder_over_every_source() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t1_selfhost_strict.rs");
    let ladder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/hopladder/mod.rs");
    no_chain_reference_in(&here);
    no_chain_reference_in(&ladder);

    let paths = corpus_paths();

    // ── MEASURE THE INFORMATIVE SOURCES FIRST, NOT THE ALPHABETICAL ONES ──────
    //
    // A census whose cost depends on its own answer is a census that may be cut
    // short, and alphabetical order puts the cheap, already-known members first:
    // `artha.t1` (hop 0) and `ashtaka.t1` (hop 5, pinned) are both measured and
    // neither can move the prediction. **The sources that can falsify it go first.**
    //
    // `lex.t1` and `kosha.t1` are the two the record shows reaching emit on the
    // `.t1`-only path besides `ashtaka`, so they are where "most sources sit at 0"
    // breaks if it is going to. `shrinkhala.t1` has ten imports and compiles itself
    // last, so it is the strongest test of monotonicity-by-import-depth.
    //
    // A PARTIAL CENSUS OF THE INFORMATIVE MEMBERS BEATS A COMPLETE CENSUS OF THE
    // CHEAP ONES — and this ordering is why a truncated run still answers the
    // question rather than merely reporting the easy half of it.
    const INFORMATIVE: &[&str] = &["lex.t1", "kosha.t1", "shrinkhala.t1"];
    let mut paths = paths;
    paths.sort_by_key(|p| {
        let n = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        (
            INFORMATIVE
                .iter()
                .position(|x| *x == n)
                .unwrap_or(usize::MAX),
            n,
        )
    });
    let limit = std::env::var("HOP_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    println!("METRIC t1_selfhost_census_sources {}", paths.len());
    // THE ROSTER, PRINTED BEFORE ANY MEASUREMENT — so an interrupted run stays
    // readable. A source listed here with NO `hop` line below it was NEVER REACHED
    // by the run; a source with `hop -1` WAS reached and could not start. Without
    // this, a truncated log merges those two, and a truncated log is the likeliest
    // outcome of a census whose cost depends on its own answer.
    for p in paths.iter().take(limit) {
        println!(
            "  ROSTER {}",
            p.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    let (mut on_ladder, mut reached_run, mut could_not_run, mut no_module) = (0, 0, 0, 0);
    for p in paths.iter().take(limit) {
        let file = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let src = std::fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
        let Some(module) = declared_module(&src) else {
            no_module += 1;
            println!("  {file:22} NO MODULE — not on the ladder");
            continue;
        };
        let started = std::time::Instant::now();
        let build = || {
            let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
            Interpreter::load_paths(&refs, &spec_root())
                .map_err(|e| format!("strict load of {} disk sources: {e:?}", refs.len()))
        };
        let reach = reach_with(build, &module, &src);
        let secs = started.elapsed().as_secs_f64();
        on_ladder += 1;
        match &reach {
            Reach::CouldNotRun(_) => could_not_run += 1,
            Reach::Hops(5) => reached_run += 1,
            Reach::Hops(_) => {}
        }
        // EMITTED PER SOURCE, NOT BATCHED AT THE END: a summary written only on
        // completion is lost entirely when the run is stopped — exactly when a
        // partial reading is most wanted.
        println!(
            "METRIC t1_selfhost_strict_hop_{} {}",
            file.replace(".t1", ""),
            reach.number()
        );
        println!(
            "  {file:22} {module:16} hop {:>2}  {secs:6.1}s  {reach:?}",
            reach.number()
        );
    }
    println!("METRIC t1_selfhost_census_on_ladder {on_ladder}");
    println!("METRIC t1_selfhost_census_no_module {no_module}");
    println!("METRIC t1_selfhost_census_could_not_run {could_not_run}");
    println!("METRIC t1_selfhost_census_reached_run {reached_run}/{on_ladder}");
    // **WHAT THIS NUMBER CANNOT MOVE FOR, PRINTED BESIDE IT.** The ladder links ONE
    // object plus the startup, so a module whose cross-module references are CALLS
    // cannot link here however correct it is — `kosha`'s fifteen errors are its
    // fifteen call sites to `अष्टकॱअष्टकयोजनम्`, undefined because `ashtaka`'s object
    // is not in the link. Modules whose cross-module references are TYPES and GLOBALS
    // need no symbol and do reach RUN: `sarani` at 40 references, `utsarjana` 33,
    // `sanchaya` 28. So this is a per-module ISOLATION test, and coupling counts
    // never predicted its split because coupling was never the variable.
    //
    // WITHOUT THIS LINE THE NUMBER READS AS PROGRESS AND IS NOT. On 2026-09-16 a
    // one-line fix moved six sources from emitting nothing to emitting, building an
    // object and correctly reporting they cannot link alone — real movement that this
    // metric CANNOT show, and a day of reporting called it "stuck on capability".
    // The whole-corpus measure is `the_driver_compiles_every_corpus_source_into_one_image_that_runs`
    // (21 sources, 21 objects linked, image runs) and the self-image answering २३४.
    println!(
        "NOTE  t1_selfhost_census_reached_run is an ISOLATION test: capped at modules \
         with no cross-module CALLS (types and globals need no symbol). It cannot rise \
         for a module that calls across modules, however correct that module is. The \
         whole-corpus measures are the driver's one-image test and the self-image's २३४."
    );
}
