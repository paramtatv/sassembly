//! **THE HOP LADDER ACROSS THE WHOLE CORPUS — A MEASUREMENT, NOT A GATE.**
//!
//! `#[ignore]`d deliberately and permanently: twenty sources times two arms is
//! roughly eighty minutes, and adding that to every landing is a decision nobody
//! has made. W-282 put ~535s on the gate for ONE source and that was ruled on
//! explicitly. This is run by hand and reported.
//!
//! **THE PREDICTION IS REGISTERED IN `PREDICTION-hopcorpus.md`, WRITTEN BEFORE
//! THIS RAN.** Its falsifiable half: the brief says hop reach is monotone by
//! IMPORT DEPTH, and the counterexample is already in the record — `kosha.t1`
//! (1 import, 11 declarations) emits while `sanskrit_text.t1`, `utsarjana.t1`
//! and `vishlesana.t1` (also 1 import, 44-94 declarations) do not. Import depth
//! cannot separate four files that share it. Size and feature-use can.
//!
//! **A FRESH INTERPRETER PER SOURCE, WHICH IS THE SLOW CHOICE AND THE RIGHT
//! ONE.** Reusing one across twenty sources would save most of the eighty
//! minutes and would be exactly the shape W-279 found: a measurement whose
//! interval carries state from its predecessor. That defect produced a count of
//! entries written of −39 and went unnoticed because every intermediate number
//! looked plausible. A slow measurement beats a contaminated one.

mod hopladder;

use hopladder::{Reach, reach_with};
use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::Interpreter;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

fn corpus_paths() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("corpus dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    v
}

/// The module a file declares, or `None` when it declares none.
///
/// **`lib.t1` DECLARES NO `मण्डलम्` AND THAT IS ITS CONTENT** — its own margin
/// says so: Rust's crate root lists modules and T1 has no crate root. There is no
/// module name to pass, so it is `CouldNotRun` and NOT hop 0. Those are different
/// facts and only one is about the compiler.
fn module_of(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let first = text.lines().find(|l| l.starts_with("मण्डलम्"))?;
    first.split_whitespace().nth(1).map(str::to_string)
}

#[test]
#[ignore = "~80 minutes, twenty sources times two arms. A MEASUREMENT, run by \
            hand and reported; adding it to the gate is a decision nobody has \
            made. Run with --ignored --nocapture."]
fn the_hop_ladder_across_the_whole_corpus() {
    let paths = corpus_paths();
    println!("METRIC t1_hopcorpus_sources {}", paths.len());

    let strict_refs: Vec<PathBuf> = paths.clone();
    let mut rows: Vec<(String, Reach, Reach)> = Vec::new();

    for p in &paths {
        let file = p.file_name().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(p).expect("readable");
        let Some(module) = module_of(p) else {
            let why = Reach::CouldNotRun("declares no मण्डलम्".into());
            println!("  {file:22} lenient {why:?} · strict {why:?}");
            rows.push((file, why.clone(), why));
            continue;
        };

        let lenient = reach_with(
            || Interpreter::load(CHAIN, &spec_root()).map_err(|e| format!("lenient load: {e:?}")),
            &module,
            &src,
        );
        let strict = reach_with(
            || {
                let refs: Vec<&Path> = strict_refs.iter().map(PathBuf::as_path).collect();
                Interpreter::load_paths(&refs, &spec_root())
                    .map_err(|e| format!("strict load: {e:?}"))
            },
            &module,
            &src,
        );
        println!("  {file:22} {module:16} lenient {lenient:?} · strict {strict:?}");
        rows.push((file, lenient, strict));
    }

    // THE DISTRIBUTION, BOTH ARMS. Printed whole: a filter sized for one source
    // silently truncates twenty, which is how a `head -8` hid a gate verdict.
    for (arm, idx) in [("lenient", 1usize), ("strict", 2usize)] {
        let mut hist = [0usize; 7]; // 0..5 plus CouldNotRun
        for r in &rows {
            let v = if idx == 1 { &r.1 } else { &r.2 };
            match v {
                Reach::CouldNotRun(_) => hist[6] += 1,
                Reach::Hops(n) => hist[*n as usize] += 1,
            }
        }
        for (h, n) in hist.iter().enumerate().take(6) {
            println!("METRIC t1_hopcorpus_{arm}_at_hop_{h} {n}");
        }
        println!("METRIC t1_hopcorpus_{arm}_could_not_run {}", hist[6]);
        println!("METRIC t1_hopcorpus_{arm}_reached_run {}", hist[5]);
        // Same ladder, same cap — see `t1_selfhost_strict.rs`. Printed on BOTH arms
        // because the lenient/strict distinction is about which loader builds the
        // interpreter, not about how many objects reach the link: either way it is
        // one object plus the startup, so a module that CALLS across modules cannot
        // link here. Annotating only the strict arm would leave the lenient number
        // reading as progress.
        println!(
            "NOTE  t1_hopcorpus_{arm}_reached_run is an ISOLATION test: capped at \
             modules with no cross-module CALLS. It cannot rise for a module that \
             calls across modules, however correct that module is."
        );
    }

    for (file, l, s) in &rows {
        println!(
            "METRIC t1_hopcorpus_row_{} {} {}",
            file.replace(['.', '-'], "_"),
            l.number(),
            s.number()
        );
    }

    // THE ONLY ASSERTION: the control from W-282 is not repeated here, but a run
    // in which NOTHING reaches hop 5 would mean the harness broke rather than the
    // corpus regressed — `ashtaka.t1` is measured at 5 by two gated tests.
    let any_five = rows.iter().any(|(_, l, _)| matches!(l, Reach::Hops(5)));
    assert!(
        any_five,
        "no source reached RUN on the lenient arm. `ashtaka.t1` is measured at \
         hop 5 by two tests the gate runs, so this is the harness, not the corpus."
    );
}
