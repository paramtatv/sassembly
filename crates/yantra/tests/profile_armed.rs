//! **A PROFILER THAT PRINTED NOTHING HAD THREE CAUSES AND THE CENSUS COULD
//! SPELL ONLY ONE.**
//!
//! `T1_STEP_PROFILE=shrinkhala` was set on the census twice and printed no
//! profile line either time, for two different reasons, and the output was
//! byte-for-byte the same shape both times — which is also the shape a source
//! with no hot span would print.
//!
//! * 2026-09-28: the switch took the FILE NAME and `T1_CORPUS` takes the STEM,
//!   so the request matched no row at all.
//! * 2026-09-29: with both spellings accepted, `T1_FULL_CENSUS=1
//!   T1_STEP_PROFILE=shrinkhala measure_corpus_encode` ran 209 s, PASSED, and
//!   still printed nothing — `shrinkhala.t1` stops at LINK in that test (one
//!   unresolved label, `अङ्कःअन्तः`) and the profiler lives inside the run
//!   stage. The request was right; the test could not reach it.
//! * 2026-09-29, again: `@link` on its own sent that cycle looking for a
//!   different TEST. Under `T1_CORPUS=shrinkhala` the stop is **33 unresolved
//!   cross-module labels** — a narrowed census links a row only against the
//!   other rows of the same run. That stop asks for a WIDER CORPUS, which is
//!   not the same next step, so the count is in the line.
//! * 2026-09-29, third: WIDER IS NOT MONOTONICALLY BETTER, and the first draft
//!   of this note said it was. 33 at 1 source, 25 at 11, 9 at 14 — and **75 at
//!   20**, because `ir.t1` stops at `emit` in the twenty-source run where it
//!   reached `link` in the fourteen, so no `मध्यरूप` object is in the image.
//!   A count can move the wrong way, which is exactly why the line prints the
//!   count rather than a verdict.
//!
//! Those two call for opposite next work — a different spelling versus a
//! different test — and the third state, a request that genuinely fired, is the
//! only one anybody was reading. [`yantra::profile::armed`] keeps the counts
//! apart and [`yantra::profile::Armed::report`] gives each its own sentence.
//!
//! # The case that must still be refused
//!
//! [`a_request_that_fired_is_not_never_fired_because_a_sibling_stopped`]: under
//! `all` most of a corpus stops early and one source runs. An implementation
//! that asks "is `stopped` non-empty?" — the obvious one, and the one that
//! makes the never-fired line fire on every healthy corpus run — reports the
//! successful profile as a failure. The verdict is `fired.is_empty()`, and only
//! that.
//!
//! [`a_link_stop_with_nothing_unresolved_does_not_ask_for_a_wider_corpus`] is
//! the refusal that comes with the count: a source can stop at `link` with
//! every label resolved, and an implementation that keyed the advice on the
//! STAGE rather than on the COUNT would tell that reader to widen a corpus that
//! is already wide enough — the same wrong next step, with the sign flipped.
//!
//! [`an_unmatched_request_is_not_a_stopped_one_and_names_no_stage`] is the
//! other collapse: `matched() == 0` must not print the never-fired sentence
//! with an empty list, because "nothing reached the run stage" and "nothing by
//! that name exists" are exactly the two the reader is here to tell apart.
//!
//! # No loader, deliberately
//!
//! Unlike `step_profile.rs`, `span_owner.rs` and `routine_visits.rs`, nothing
//! here runs a machine: [`yantra::profile::armed`] is a reading over census
//! ROWS, and a machine in this file would only be a second thing that could be
//! wrong about it.

use yantra::profile::{Armed, Reached, Stop, armed, names_source};

/// A [`Stop`], as the assertions below spell one.
fn stop(source: &str, stage: &str, unresolved: usize) -> Stop {
    Stop {
        source: source.to_string(),
        stage: stage.to_string(),
        unresolved,
    }
}

/// The census's own shape on 2026-09-29, narrowed to what `armed` reads.
fn census_rows() -> Vec<Reached<'static>> {
    vec![
        Reached {
            source: "ashtaka.t1",
            stage: "run",
            unresolved: 0,
            profiled: false,
        },
        Reached {
            source: "ir.t1",
            stage: "link",
            unresolved: 5,
            profiled: false,
        },
        // The measured shape of `T1_CORPUS=shrinkhala T1_STEP_PROFILE=shrinkhala`
        // on 2026-09-29: one row, stopped at link, 33 labels no object in the
        // run defines.
        Reached {
            source: "shrinkhala.t1",
            stage: "link",
            unresolved: 33,
            profiled: false,
        },
        Reached {
            source: "kosha.t1",
            stage: "run",
            unresolved: 0,
            profiled: false,
        },
    ]
}

#[test]
fn the_census_run_that_printed_nothing_reads_as_never_fired_and_names_the_stage() {
    let a = armed("shrinkhala", &census_rows());
    assert!(a.fired.is_empty(), "nothing was profiled: {a:?}");
    assert_eq!(
        a.stopped,
        vec![stop("shrinkhala.t1", "link", 33)],
        "the one matched row, with the stage it did reach"
    );
    assert_eq!(a.matched(), 1);
    let line = a.report("shrinkhala");
    assert!(
        line.starts_with(
            "never-fired shrinkhala 1 matched, 0 ran: shrinkhala.t1@link (33 unresolved)"
        ),
        "the stage AND the count are IN the line — a reader must not have to go find either: {line}"
    );
    assert!(
        line.contains("widen T1_CORPUS"),
        "33 labels no object in this run defines: the remedy is the corpus, and the \
         line says which remedy: {line}"
    );
    // THE WHOLE POINT: this is not the unmatched sentence.
    assert!(!line.starts_with("unmatched"), "{line}");
}

#[test]
fn an_unmatched_request_is_not_a_stopped_one_and_names_no_stage() {
    let a = armed("shrnkhala", &census_rows());
    assert_eq!(a.matched(), 0);
    assert!(a.stopped.is_empty(), "a typo stopped at no stage: {a:?}");
    let line = a.report("shrnkhala");
    assert!(
        line.starts_with("unmatched shrnkhala 0 source(s)"),
        "{line}"
    );
    // The two states this instrument exists to separate, kept separate.
    assert!(!line.contains("never-fired"), "{line}");
    assert!(!line.contains('@'), "there is no stage to name: {line}");
}

#[test]
fn a_request_that_fired_is_not_never_fired_because_a_sibling_stopped() {
    // `all` on a corpus where one source runs under the profiler and three do
    // not — the ordinary healthy shape, and the one a `stopped.is_empty()`
    // verdict would report as a failure.
    let mut rows = census_rows();
    rows[2].stage = "run";
    for r in &mut rows {
        // Under `all`, every row that REACHES the run stage is profiled — so
        // the only rows left in `stopped` are the ones that stopped, which is
        // what makes this fixture the healthy shape rather than a contrived
        // one.
        r.profiled = r.stage == "run";
    }
    let a = armed("all", &rows);
    assert_eq!(
        a.fired,
        vec![
            "ashtaka.t1".to_string(),
            "shrinkhala.t1".to_string(),
            "kosha.t1".to_string()
        ],
        "the rows' own order, not sorted"
    );
    assert_eq!(a.matched(), 4, "`all` names every row");
    assert_eq!(a.stopped, vec![stop("ir.t1", "link", 5)]);
    let line = a.report("all");
    assert!(
        line.starts_with("fired all 3 of 4: ashtaka.t1 shrinkhala.t1 kosha.t1"),
        "{line}"
    );
    assert!(
        line.contains("1 stopped earlier: ir.t1@link (5 unresolved)"),
        "the sibling row is still reported: {line}"
    );
    assert!(!line.contains("never-fired"), "IT FIRED: {line}");
}

#[test]
fn the_stem_and_the_file_name_name_the_same_source_and_nothing_else_does() {
    assert!(names_source("shrinkhala", "shrinkhala.t1"), "the stem");
    assert!(
        names_source("shrinkhala.t1", "shrinkhala.t1"),
        "the file name"
    );
    assert!(
        names_source("all", "shrinkhala.t1"),
        "`all` is every source"
    );
    // A PREFIX IS NOT A NAME. `shrink` would make the switch fire on a source
    // nobody asked for, which is the same silence with the sign flipped.
    assert!(!names_source("shrink", "shrinkhala.t1"));
    assert!(!names_source("shrinkhala.t", "shrinkhala.t1"));
    assert!(!names_source("", "shrinkhala.t1"));
    // The stem rule strips exactly one suffix and does not re-read the stem as
    // a file name: `ir` names `ir.t1`, not `ir.t1.t1`.
    assert!(names_source("ir", "ir.t1"));
    assert!(!names_source("ir", "ir.t1.t1"));
}

#[test]
fn a_default_reading_matched_nothing_and_says_so_rather_than_reading_as_fired() {
    let a = Armed::default();
    assert_eq!(a.matched(), 0);
    assert!(a.report("x").starts_with("unmatched x 0 source(s)"));
}

#[test]
fn a_link_stop_with_nothing_unresolved_does_not_ask_for_a_wider_corpus() {
    // The same stage, the opposite finding: every label this row references has
    // an object in the run, and it still did not reach `run`. Widening the
    // corpus would do nothing, so the line must not say to.
    let mut rows = census_rows();
    rows[2].unresolved = 0;
    rows[1].unresolved = 0;
    let a = armed("shrinkhala", &rows);
    assert_eq!(a.stopped, vec![stop("shrinkhala.t1", "link", 0)]);
    let line = a.report("shrinkhala");
    assert!(
        line.starts_with("never-fired shrinkhala 1 matched, 0 ran: shrinkhala.t1@link"),
        "{line}"
    );
    assert!(
        !line.contains("unresolved"),
        "there is nothing unresolved to report, and a `(0 unresolved)` would read \
         as a finding: {line}"
    );
    assert!(
        !line.contains("widen T1_CORPUS"),
        "THE REFUSAL: the advice is keyed on the COUNT, not on the STAGE: {line}"
    );
}

#[test]
fn one_stopped_row_with_a_count_is_enough_to_name_the_remedy() {
    // `all` over a corpus where one row's link is short and another's is not:
    // the advice belongs to the run, and a reader with a mixed list still needs
    // it. Keyed on `any`, and the per-row counts keep the rows apart.
    let a = armed("all", &census_rows());
    assert_eq!(
        a.stopped,
        vec![
            stop("ashtaka.t1", "run", 0),
            stop("ir.t1", "link", 5),
            stop("shrinkhala.t1", "link", 33),
            stop("kosha.t1", "run", 0),
        ],
        "the rows' own order; `ashtaka.t1` and `kosha.t1` REACHED run and were \
         still not profiled, so they are stopped rows with nothing unresolved — \
         `stopped` is about the profiler, not about the stage"
    );
    assert_eq!(
        a.stopped.len(),
        4,
        "nothing was profiled, so every row is stopped"
    );
    let line = a.report("all");
    assert!(
        line.contains("ashtaka.t1@run"),
        "no count, no parenthesis: {line}"
    );
    assert!(line.contains("ir.t1@link (5 unresolved)"), "{line}");
    assert!(line.contains("widen T1_CORPUS"), "{line}");
}
