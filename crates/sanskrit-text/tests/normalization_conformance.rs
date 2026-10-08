//! Task `A-013` — conformance against the pinned `NormalizationTest.txt`.
//!
//! This is the oracle for [`sanskrit_text::nfc`] / [`nfd`] (doc 14 §P-1). The
//! gate is **100%**: doc 01 D-01-B makes NFC the storage form of every source
//! file, so a single disagreement with the UCD is a spelling of an identifier
//! that the assembler and the rest of the world would not agree on.

use std::path::PathBuf;

use sanskrit_text::{nfc, nfd};

fn ucd(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .join("research/specs/unicode/ucd")
        .join(name)
}

/// `0041 0300` -> the corresponding string.
fn codepoints(field: &str) -> String {
    field
        .split_whitespace()
        .map(|h| {
            let cp =
                u32::from_str_radix(h, 16).unwrap_or_else(|e| panic!("bad codepoint {h}: {e}"));
            char::from_u32(cp).unwrap_or_else(|| panic!("unpaired surrogate U+{cp:04X}"))
        })
        .collect()
}

fn escape(s: &str) -> String {
    s.chars()
        .map(|c| format!("{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

struct Case {
    line_no: usize,
    part: String,
    c: [String; 5],
}

fn load_cases() -> Vec<Case> {
    let path = ucd("NormalizationTest.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "read {}: {e}\nrun research/specs/fetch-specs.sh",
            path.display()
        )
    });

    let mut part = String::from("(none)");
    let mut cases = Vec::new();

    for (i, raw) in text.lines().enumerate() {
        if let Some(p) = raw.strip_prefix('@') {
            part = p.trim().to_string();
            continue;
        }
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() < 5 {
            continue;
        }
        cases.push(Case {
            line_no: i + 1,
            part: part.clone(),
            c: std::array::from_fn(|k| codepoints(fields[k])),
        });
    }
    assert!(cases.len() > 15_000, "only {} cases parsed", cases.len());
    cases
}

/// The five invariants of NormalizationTest.txt that involve canonical forms:
///
/// ```text
/// NFC(c1) == NFC(c2) == NFC(c3) == c2
/// NFD(c1) == NFD(c2) == NFD(c3) == c3
/// ```
///
/// The NFKC/NFKD columns are not checked — those forms are deliberately not
/// implemented (see `normalize.rs`) — except that c4 and c5 must already be
/// stable under the canonical forms.
#[test]
#[ignore = "needs research/specs/unicode (the Unicode data files) not in the public repository"]
fn normalization_test_conformance() {
    let cases = load_cases();
    let mut failures: Vec<String> = Vec::new();

    for case in &cases {
        let [c1, c2, c3, c4, c5] = &case.c;
        let checks: [(&str, String, &String); 8] = [
            ("NFC(c1)", nfc(c1), c2),
            ("NFC(c2)", nfc(c2), c2),
            ("NFC(c3)", nfc(c3), c2),
            ("NFD(c1)", nfd(c1), c3),
            ("NFD(c2)", nfd(c2), c3),
            ("NFD(c3)", nfd(c3), c3),
            ("NFC(c4)", nfc(c4), c4),
            ("NFD(c5)", nfd(c5), c5),
        ];
        for (what, got, want) in checks {
            if got != *want {
                failures.push(format!(
                    "line {} [@{}] {what}: got [{}], want [{}]  (source [{}])",
                    case.line_no,
                    case.part,
                    escape(&got),
                    escape(want),
                    escape(c1),
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} cases failed:\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );
    println!("{} cases, all canonical invariants hold", cases.len());
    println!("METRIC normalization_cases_passing {}", cases.len());
    println!("METRIC normalization_conformance 100%");
}

/// The other half of the conformance requirement: every codepoint **not** named
/// in `@Part1` must be unchanged by both canonical forms. Line-by-line testing
/// alone would miss a table that wrongly maps something it should leave alone.
#[test]
#[ignore = "needs research/specs/unicode (the Unicode data files) not in the public repository"]
fn unlisted_codepoints_are_unchanged() {
    let listed: std::collections::HashSet<char> = load_cases()
        .iter()
        .filter(|c| c.part.starts_with("Part1"))
        .filter_map(|c| {
            let mut it = c.c[0].chars();
            match (it.next(), it.next()) {
                (Some(ch), None) => Some(ch),
                _ => None,
            }
        })
        .collect();
    assert!(listed.len() > 1000, "only {} Part1 entries", listed.len());

    let mut failures = 0usize;
    let mut first: Option<String> = None;

    for cp in 0u32..0x11_0000 {
        let Some(ch) = char::from_u32(cp) else {
            continue; // surrogates
        };
        if listed.contains(&ch) {
            continue;
        }
        let s = ch.to_string();
        if nfc(&s) != s || nfd(&s) != s {
            failures += 1;
            first.get_or_insert_with(|| {
                format!(
                    "U+{cp:04X}: NFC=[{}] NFD=[{}]",
                    escape(&nfc(&s)),
                    escape(&nfd(&s))
                )
            });
        }
    }

    assert_eq!(
        failures, 0,
        "{failures} unlisted codepoints changed; first: {first:?}"
    );
}
