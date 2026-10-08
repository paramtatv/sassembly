//! Every ratified sign is reachable in ≤3 keystrokes — BUILD.md A11, task
//! `A-038a`.
//!
//! A11's gate condition has two halves: **≥30 WPM** and **every sign in ≤3
//! keystrokes**. The first needs a person at a keyboard and is `A-039`. The
//! second is a property of the input mapping and can be measured today, so it
//! is measured today rather than asserted in a design document.
//!
//! # There is no new input method here, and that is the finding
//!
//! The SLP1 bijection (`A-019`, `A-020`) already maps every one of ADR-0003's
//! ratified signs to a single ASCII character, and the Devanagari digits too:
//! `|` → `।`, `&` → `॥`, `@` → `॰`, `.` → `ॱ`, `'` → `ऽ`, `$` → `ॐ`, `0`–`9`
//! → `०`–`९`. The label sign `ॱॱ` is two, being the high dot twice. Nothing
//! exceeds two.
//!
//! So the keystroke half of A11 was satisfied by work done for a different
//! reason, and nobody had checked. This test is the check.
//!
//! # Derived from the ADR, not restated from it
//!
//! The sign inventory is parsed out of `docs/adr/0003-…` rather than copied
//! here. A sign added to the ADR is a sign this test demands an input path for;
//! a copy would go quietly stale, which is the failure `c_params_agree.rs` was
//! written to stop in the S1 harness.

use std::path::{Path, PathBuf};

/// How many keystrokes A11 allows.
const BUDGET: usize = 3;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

/// The signs ADR-0003 ratifies, read from its own role/sign/CP table.
fn ratified_signs() -> Vec<(String, String)> {
    let adr =
        std::fs::read_to_string(root().join("docs/adr/0003-orthographic-closure-symbol-table.md"))
            .expect("read ADR-0003");

    let mut found = Vec::new();
    for line in adr.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // `| Role | `sign` | U+XXXX |` — five cells with the empty edges.
        if cells.len() != 5 || !cells[3].starts_with("U+") {
            continue;
        }
        let sign = cells[2].trim_matches('`');
        if sign.is_empty() || sign.is_ascii() {
            continue;
        }
        found.push((cells[1].to_string(), sign.to_string()));
    }
    found
}

/// The ADR's table must still be a table.
///
/// A parser that matches nothing reports no violations, which reads exactly
/// like a pass. Seven roles are ratified in ADR-0003 §"Assignments"; fewer means
/// the format moved and this test stopped looking.
#[test]
#[ignore = "blocked: needs docs/adr (the design records) not in the public repository"]
fn the_adr_still_lists_its_signs() {
    let signs = ratified_signs();
    assert!(
        signs.len() >= 7,
        "parsed only {} signs out of ADR-0003 — the table format probably \
         moved: {signs:?}",
        signs.len()
    );
}

#[test]
#[ignore = "blocked: needs docs/adr (the design records) not in the public repository"]
fn every_ratified_sign_is_within_the_keystroke_budget() {
    for (role, sign) in ratified_signs() {
        let mut typed = String::new();
        sanskrit_text::slp1::encode_into(&sign, &mut typed)
            .unwrap_or_else(|e| panic!("`{sign}` ({role}) has no SLP1 input at all: {e:?}"));
        let strokes = typed.chars().count();
        assert!(
            strokes <= BUDGET,
            "`{sign}` ({role}) needs {strokes} keystrokes as {typed:?}; A11 allows {BUDGET}"
        );
    }
}

/// Numerals are written in Devanagari digits, so every program contains them.
///
/// Checked separately because they are not in the ADR's sign table — they are
/// in its font-coverage table, and a reader could reasonably conclude the input
/// path does not carry them. It does: one keystroke each.
#[test]
fn every_devanagari_digit_is_within_the_keystroke_budget() {
    for (i, digit) in ('\u{0966}'..='\u{096F}').enumerate() {
        let s = digit.to_string();
        let mut typed = String::new();
        sanskrit_text::slp1::encode_into(&s, &mut typed)
            .unwrap_or_else(|e| panic!("digit {i} (`{digit}`) has no SLP1 input: {e:?}"));
        let strokes = typed.chars().count();
        assert!(
            strokes <= BUDGET,
            "digit {i} (`{digit}`) needs {strokes} keystrokes as {typed:?}"
        );
    }
}
