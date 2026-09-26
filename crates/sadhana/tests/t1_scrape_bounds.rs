//! The rig rule that had no enforcement: **bound every scrape to the
//! declaration it means.**
//!
//! # What this exists because of
//!
//! `parse_t1_operator_table` in `t1_operators.rs` walked EVERY string literal
//! in `parse.t1` and paired each with the next word ending in `द्विकर्मभेद`.
//! Its only bound was "a kind word must arrive before the next string", which
//! bounds nothing for the last string of a routine that names no kind
//! constants. When `72117493` added exactly such a routine — the
//! operator-spelling predicate at `parse.t1:779-830`, which returns `सत्यम्`
//! and names no kinds — that last string scanned on to
//! `वास्तुॱगुणनद्विकर्मभेद` at `:1086` and minted a spurious pair. The test
//! went red reporting 16 operators against `ast.t1`'s 15, and **the corpus was
//! correct the whole time**: the instrument's premise ("strings and kind
//! constants alternate") was never a property of the corpus.
//!
//! The lesson was already written down — `tools/T1-RIG.md` rule 1, and
//! `t1-rig.py`'s `enclosing_routine`, built after two runs were lost to markers
//! landing in a duplicated body in `encode.t1`. It was written for `.t1` marker
//! placement and never applied to the Rust helpers that scrape `.t1`. **A rule
//! with no enforcement point is a rule that gets relearned.** This is the
//! enforcement point: it runs in `cargo test`, so it cannot be skipped.
//!
//! # The escape hatch
//!
//! A whole-file scan is sometimes exactly right — a census over a file's
//! constants has no routine to bound to. Mark those `RIG-ALLOW: whole-file`
//! with a reason on the same line. The marker is greppable, so the set of
//! deliberate whole-file scans is always one command away.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<pkg> has a grandparent")
        .to_path_buf()
}

/// Split Rust source into `fn` blocks: a header line to the next one.
fn fn_blocks(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut name: Option<String> = None;
    let mut body = String::new();
    for line in src.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("fn ").or_else(|| t.strip_prefix("pub fn ")) {
            if let Some(n) = name.take() {
                out.push((n, std::mem::take(&mut body)));
            }
            name = Some(
                rest.split(['(', '<'])
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .to_string(),
            );
        }
        body.push_str(line);
        body.push('\n');
    }
    if let Some(n) = name {
        out.push((n, body));
    }
    out
}

/// Does this function lex a `.t1` and then walk every token it got?
fn scrapes_whole_file(body: &str) -> bool {
    body.contains("lex_t1(")
        && (body.contains("tokens.iter()")
            || body.contains("in &tokens")
            || body.contains("in tokens"))
}

/// Is the walk bound to a declaration, sliced, or deliberately whole-file?
fn is_bounded(body: &str) -> bool {
    body.contains("वृत्तिः")            // bound to a routine declaration
        || body.contains("RIG-ALLOW: whole-file") // deliberate, with a reason
        || body.contains(".position(")  // located a span before walking
        || body.contains("tokens[") // walks a slice, not the whole arena
}

fn offenders_in(src: &str) -> Vec<String> {
    fn_blocks(src)
        .into_iter()
        .filter(|(_, b)| scrapes_whole_file(b) && !is_bounded(b))
        .map(|(n, _)| n)
        .collect()
}

/// THE POSITIVE CONTROL. A check that cannot fail reads exactly like one that
/// passed, and this file exists because an unbounded scan went unnoticed for a
/// commit. So the detector is shown catching one, and shown clearing the same
/// function once bounded — every run, in the same binary as the real check.
#[test]
fn the_detector_catches_an_unbounded_scrape_and_clears_a_bounded_one() {
    let unbounded = r#"
fn scrape() -> Vec<String> {
    let tokens = lex_t1(&text).expect("lexes");
    for t in &tokens { out.push(t.text.clone()); }
    out
}
"#;
    assert_eq!(
        offenders_in(unbounded),
        vec!["scrape".to_string()],
        "the detector must flag a lex-and-walk-everything helper"
    );

    let bounded = r#"
fn scrape() -> Vec<String> {
    let tokens = lex_t1(&text).expect("lexes");
    let start = tokens.windows(2).position(|w| w[0].text == "वृत्तिः").unwrap();
    for t in &tokens[start..end] { out.push(t.text.clone()); }
    out
}
"#;
    assert!(
        offenders_in(bounded).is_empty(),
        "the detector must clear a helper bounded to its routine"
    );

    let allowed = r#"
fn scrape() -> Vec<String> {
    // RIG-ALLOW: whole-file — a census of one file's constants has no routine.
    let tokens = lex_t1(&text).expect("lexes");
    for t in &tokens { out.push(t.text.clone()); }
    out
}
"#;
    assert!(
        offenders_in(allowed).is_empty(),
        "the marker must be honoured, so a deliberate census is not a refusal"
    );
}

#[test]
fn no_test_helper_scrapes_a_t1_file_without_bounding_the_walk() {
    let tests_dir = repo_root().join("crates");
    let mut found: Vec<String> = Vec::new();
    let mut files = 0usize;

    for pkg in std::fs::read_dir(&tests_dir).expect("read crates/") {
        let dir = pkg.expect("entry").path().join("tests");
        if !dir.is_dir() {
            continue;
        }
        for e in std::fs::read_dir(&dir).expect("read tests/") {
            let p = e.expect("entry").path();
            if p.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            // THIS FILE IS THE DETECTOR. Its pattern strings and its positive
            // control's fixtures look exactly like what it hunts — and on the
            // first run it duly reported itself, which is what a new scanner
            // always finds first. Skipping it is not weakening the check: the
            // control above already proves the detector still bites.
            if p.file_name().and_then(|s| s.to_str()) == Some("t1_scrape_bounds.rs") {
                continue;
            }
            files += 1;
            let src = std::fs::read_to_string(&p).expect("read test source");
            for name in offenders_in(&src) {
                found.push(format!(
                    "{}::{name}",
                    p.file_name().unwrap().to_string_lossy()
                ));
            }
        }
    }

    // The scan must have had something to scan — an empty sweep would pass
    // silently and is the failure mode this whole file is about.
    assert!(
        files >= 20,
        "expected to scan the test suite, saw {files} files"
    );

    assert!(
        found.is_empty(),
        "these helpers lex a `.t1` and walk every token with no bound.\n\
         Bound the walk to the declaration it means (see `parse_t1_operator_table`),\n\
         or mark it `RIG-ALLOW: whole-file` with a reason if a census is intended:\n  {}",
        found.join("\n  ")
    );
}
