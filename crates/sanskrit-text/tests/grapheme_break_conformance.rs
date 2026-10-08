//! Task `A-015` — conformance against the pinned `GraphemeBreakTest.txt`.
//!
//! The oracle for [`sanskrit_text::aksharas`] (doc 14 §P-1). The gate is
//! **100%**: the akṣara is the terminal cell, the cursor step and the backspace
//! unit (doc 01 D-01-A, doc 04 §2), so a single disagreement is visible text
//! corruption rather than an abstract standards failure.

use std::path::PathBuf;

use sanskrit_text::aksharas;

/// A line is `÷ 0020 × 0308 ÷` — `÷` a boundary, `×` no boundary.
struct Case {
    line_no: usize,
    text: String,
    /// Expected cluster lengths in bytes, in order.
    clusters: Vec<usize>,
    raw: String,
}

fn load() -> Vec<Case> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .join("research/specs/unicode/ucd/auxiliary/GraphemeBreakTest.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "read {}: {e}\nrun research/specs/fetch-specs.sh",
            path.display()
        )
    });

    let mut cases = Vec::new();
    for (i, raw_line) in text.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let mut s = String::new();
        let mut clusters: Vec<usize> = Vec::new();
        let mut current = 0usize;

        for token in line.split_whitespace() {
            match token {
                "\u{00F7}" => {
                    // boundary: close the cluster under construction
                    if current > 0 {
                        clusters.push(current);
                        current = 0;
                    }
                }
                "\u{00D7}" => {} // no boundary: keep accumulating
                hex => {
                    let cp = u32::from_str_radix(hex, 16)
                        .unwrap_or_else(|e| panic!("line {}: bad codepoint {hex}: {e}", i + 1));
                    let ch = char::from_u32(cp)
                        .unwrap_or_else(|| panic!("line {}: unpaired surrogate {hex}", i + 1));
                    s.push(ch);
                    current += ch.len_utf8();
                }
            }
        }
        if current > 0 {
            clusters.push(current);
        }

        cases.push(Case {
            line_no: i + 1,
            text: s,
            clusters,
            raw: line.to_string(),
        });
    }
    assert!(cases.len() > 500, "only {} cases parsed", cases.len());
    cases
}

#[test]
#[ignore = "census: needs research/specs/unicode (the Unicode data files) not in the public repository"]
fn grapheme_break_test_conformance() {
    let cases = load();
    let mut failures: Vec<String> = Vec::new();

    for case in &cases {
        let got: Vec<usize> = aksharas(&case.text).map(str::len).collect();
        if got != case.clusters {
            let show = |lens: &[usize]| {
                let mut at = 0;
                let mut parts = Vec::new();
                for &n in lens {
                    let piece: String = case.text[at..at + n]
                        .chars()
                        .map(|c| format!("{:04X} ", c as u32))
                        .collect();
                    parts.push(piece.trim_end().to_string());
                    at += n;
                }
                parts.join(" | ")
            };
            failures.push(format!(
                "line {}: got [{}], want [{}]\n  {}",
                case.line_no,
                show(&got),
                show(&case.clusters),
                case.raw,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} cases failed:\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );
    println!("{} grapheme-break cases pass", cases.len());
    println!("METRIC grapheme_break_cases_passing {}", cases.len());
    println!("METRIC grapheme_break_conformance 100%");
}

/// Segmentation must be lossless and non-empty for arbitrary input — the
/// property that keeps a terminal from dropping or duplicating text.
#[test]
#[ignore = "census: needs research/specs/unicode (the Unicode data files) not in the public repository"]
fn segmentation_is_total_and_lossless() {
    for case in load() {
        let mut total = 0;
        for a in aksharas(&case.text) {
            assert!(!a.is_empty(), "line {}: empty akṣara", case.line_no);
            total += a.len();
        }
        assert_eq!(total, case.text.len(), "line {}: lossy", case.line_no);
    }
}
