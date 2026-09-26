//! Line-length telemetry across the corpus — task `15.3.4`, doc 15 §7.
//!
//! Doc 15 §7 ratifies **median instruction ≤ 40 akṣaras** and §6.2 says C-15-2's
//! verbosity is "measurable against" it. It had never been measured, so the
//! budget was a number in a table rather than a fact about the tree. This
//! measures it and fails when it stops holding — a measurement taken once
//! decays into a claim about the morning it was taken.
//!
//! # The unit is the canonical one, and that is not a detail
//!
//! Lengths come from [`sanskrit_text::aksharas_count`], which `segment.rs`
//! documents as "the **column count**, not `chars().count()`" — a space is a
//! column and therefore counts. A hand-rolled count of non-combining characters
//! is the intuitive thing to write and gives a materially different number:
//! measuring the `A-063` specimens both ways moved the closure-form penalty
//! from +30% to +16–23%, because the two forms share their spaces and those
//! spaces dilute the ratio. Doc 15's budget is stated in the canonical unit, so
//! anything compared against it must be too.
//!
//! # What counts as an instruction
//!
//! A line ending in `।` — an instruction is a sentence (doc 02 §2.2, doc 15
//! §3.1). A directive ends in `॥` and a comment opens with `॰`; neither is an
//! instruction, and counting them would measure something the budget does not
//! name.

use std::path::{Path, PathBuf};

/// Doc 15 §7, task 15.3.4.
const BUDGET: usize = 40;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

/// The corpora this budget governs.
///
/// `T1` has no files yet — `B-079b` is blocked on `A-063`, which is the row this
/// measurement exists to inform. It is listed anyway so that the day T1 source
/// appears it is measured without anyone remembering to add it, and so its
/// absence is REPORTED rather than silently skipped.
const CORPORA: &[(&str, &str, &str)] = &[("T0", "spec", "sas"), ("T1", "spec", "सस")];

fn instructions(src: &str) -> Vec<usize> {
    src.lines()
        .map(str::trim)
        .filter(|l| l.ends_with('।') && !l.starts_with('॰'))
        .map(sanskrit_text::aksharas_count)
        .collect()
}

fn scan(dir: &Path, ext: &str) -> (usize, Vec<usize>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        panic!(
            "cannot read {}: the corpus this budget is about is not there",
            dir.display()
        );
    };
    let mut files = 0;
    let mut lens = Vec::new();
    for e in rd.filter_map(Result::ok) {
        let p = e.path();
        if p.extension().is_some_and(|x| x == ext) {
            files += 1;
            let src =
                std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            lens.extend(instructions(&src));
        }
    }
    (files, lens)
}

#[test]
fn median_instruction_is_within_the_line_budget() {
    let root = root();
    let mut measured_any = false;

    for (name, sub, ext) in CORPORA {
        let dir = root.join(sub);
        let (files, mut lens) = scan(&dir, ext);

        if files == 0 {
            // Absence is announced, never silent (`W-087`). For T1 this is the
            // expected state today and is not a failure; for T0 it is a failure,
            // because 29 files were there when this check was written and a
            // corpus that vanished must not read as a corpus that passed.
            println!("{name}: no *.{ext} in {} — nothing measured", dir.display());
            assert_ne!(
                *name, "T0",
                "the T0 corpus is empty. This check measured 29 files when it was \
                 written; an empty corpus is a broken checkout or a moved tree, \
                 and it must not pass by measuring nothing"
            );
            continue;
        }
        measured_any = true;

        // Impossible-value invariants (`W-081`, `W-090`). A statistic computed
        // over an empty or degenerate set is the same defect as a check that
        // exits 0 without running: it reports success about nothing.
        assert!(
            !lens.is_empty(),
            "{name}: {files} file(s) and ZERO instructions. Either every line \
             stopped ending in `।` or the filter is wrong — a budget measured \
             over no instructions is not a measurement"
        );
        assert!(
            lens.iter().all(|&n| n > 0),
            "{name}: an instruction measured 0 akṣaras, which is impossible — \
             it ends in `।`, so it has at least one"
        );

        lens.sort_unstable();
        let n = lens.len();
        let median = lens[n / 2];
        let p90 = lens[n * 9 / 10];
        let max = lens[n - 1];
        let over = lens.iter().filter(|&&x| x > BUDGET).count();

        assert!(
            median <= max && lens[0] <= median,
            "{name}: statistics are unordered"
        );

        println!(
            "{name}: {n} instructions in {files} files — median {median}, \
             p90 {p90}, max {max}, over {BUDGET}: {over}"
        );

        assert!(
            median <= BUDGET,
            "{name}: median instruction is {median} akṣaras, over the {BUDGET} \
             doc 15 §7 ratifies for task 15.3.4. The budget is what makes the \
             operators-as-words decision affordable (§6.2); if it is exceeded, \
             that decision needs revisiting rather than the number"
        );
    }

    assert!(
        measured_any,
        "no corpus was measured at all — every path in CORPORA was empty, so \
         this check passed by doing nothing"
    );
}
