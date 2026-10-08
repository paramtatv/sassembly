//! Corpus replay — the stable half of the fuzzing setup.
//!
//! `cargo fuzz` needs nightly and, on this macOS host, `-s none` (the ASan
//! runtime SIGILLs before libFuzzer's banner). None of that may stand between a
//! contributor and the regressions: a crash found once must be checked forever,
//! on every machine, in the ordinary `cargo test` run.
//!
//! So every file in `fuzz/corpus/` and `fuzz/artifacts/` is replayed here
//! through the same [`sanskrit_text::props`] assertions the fuzz targets use.
//! Adding a crash reproducer to `fuzz/artifacts/` is all it takes to turn a
//! fuzzer finding into a permanent regression test.

use std::path::PathBuf;

use sanskrit_text::props;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

/// Every corpus and artifact file, as `(label, bytes)`.
fn corpus() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for base in ["fuzz/corpus", "fuzz/artifacts"] {
        let root = repo_root().join(base);
        let Ok(targets) = std::fs::read_dir(&root) else {
            continue;
        };
        for target in targets.filter_map(Result::ok) {
            let Ok(files) = std::fs::read_dir(target.path()) else {
                continue;
            };
            for f in files.filter_map(Result::ok).filter(|f| f.path().is_file()) {
                if let Ok(bytes) = std::fs::read(f.path()) {
                    out.push((
                        format!(
                            "{}/{}",
                            target.file_name().to_string_lossy(),
                            f.file_name().to_string_lossy()
                        ),
                        bytes,
                    ));
                }
            }
        }
    }
    out.sort();
    out
}

#[test]
#[ignore = "blocked: needs fuzz/corpus (the fuzz seed corpus) not in the public repository"]
fn every_corpus_input_holds_every_invariant() {
    let files = corpus();
    assert!(
        files.len() >= 60,
        "expected at least the seed corpus, found {} files — is fuzz/corpus/ present?",
        files.len()
    );

    // Everything committed (seeds and artifacts) is always replayed. A machine
    // that has been fuzzing locally may hold thousands of discovered inputs; those
    // are sampled deterministically so the ordinary test run stays inside the
    // 10-minute CI budget (doc 14 §3.1) without ever skipping a committed case.
    const SAMPLE_ABOVE: usize = 400;
    let mut replayed = 0usize;
    for (i, (name, bytes)) in files.iter().enumerate() {
        let committed = name.contains("seed-") || name.starts_with("artifacts");
        let sampled = files.len() <= SAMPLE_ABOVE || i % (files.len() / SAMPLE_ABOVE + 1) == 0;
        if !committed && !sampled {
            continue;
        }
        // A panic here names the file, so the failure reproduces directly with
        // `cargo fuzz run <target> fuzz/corpus/<name>`.
        props::all(bytes);
        replayed += 1;
    }
    println!("METRIC fuzz_corpus_inputs {}", files.len());
    println!("replayed {replayed} of {} corpus inputs", files.len());
}

/// Any file in `fuzz/artifacts/` is a crash the fuzzer *did* find. If one
/// exists and still reproduces, the build must fail — that is the whole point of
/// keeping it.
#[test]
fn no_known_crash_reproduces() {
    let root = repo_root().join("fuzz/artifacts");
    let mut checked = 0usize;
    if let Ok(targets) = std::fs::read_dir(&root) {
        for target in targets.filter_map(Result::ok) {
            let Ok(files) = std::fs::read_dir(target.path()) else {
                continue;
            };
            for f in files.filter_map(Result::ok).filter(|f| f.path().is_file()) {
                let bytes = std::fs::read(f.path()).unwrap();
                props::all(&bytes);
                checked += 1;
            }
        }
    }
    println!("METRIC fuzz_outstanding_crashes {checked}");
    println!("{checked} known crash reproducer(s) no longer crash");
}

/// The properties must be total: byte sequences that are not UTF-8 at all are
/// something a file on disk can contain, and must produce a verdict rather than
/// a panic.
#[test]
fn arbitrary_bytes_are_handled() {
    // Every single byte, every pair, and a run of each — cheap and exhaustive
    // at the boundary where UTF-8 validity flips.
    for a in 0u8..=255 {
        props::all(&[a]);
        props::all(&[a, a]);
        props::all(&[0xE0, 0xA4, a]); // Devanagari lead bytes + arbitrary tail
        props::all(&[a, 0xE0, 0xA4, 0x95]);
    }
    for len in [0usize, 1, 2, 3, 4, 7, 64, 1024] {
        props::all(&vec![0xFFu8; len]);
        props::all(&vec![0x00u8; len]);
    }
}
