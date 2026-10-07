//! W-348 — `अन्यथा` inside a `यदि` block is blamed on scope, and the cause is
//! the keyword's position.
//!
//! `अन्यथा` IS this language's `else`, and it is read in exactly one place:
//! after the `इति` that closes a `यदि` block —
//!
//! ```text
//! यदि क आदि … इति अन्यथा आदि … इति
//! ```
//!
//! The reporter found the word in `grammar-t1.ebnf`, wrote it where other
//! languages put it, INSIDE the block before the `इति`, and was told
//! *"`अन्यथा` is not a name in scope"*. That sends the reader looking for a
//! missing declaration. The lesson they drew: a keyword's PRESENCE in the
//! grammar does not establish its VALIDITY in the construct at hand, and the
//! diagnostic should say which construct it belongs to.
//!
//! WHAT MUST NOT CHANGE, asserted here: the correct form still runs and takes
//! the else branch; a name that really is undeclared is still reported as
//! *"not a name in scope"*; and a program that DECLARES `अन्यथा` as a local
//! (which the front end permits, `a-keyword-used-as-a-name-reads-as-nil`) is
//! not refused by this change, because that hazard is a different row.

use std::path::{Path, PathBuf};

use sadhana::t1::nirvahana::{Interpreter, Value};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn run(body: &str) -> Result<Value, String> {
    let src = format!("मण्डलम् परीक्षा ॥\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n{body}इति\n");
    let mut it = Interpreter::load(&[("pariksha.t1", &src)], &spec_root())
        .map_err(|e| format!("load: {}", e.reason))?;
    it.call("परीक्षाॱक", vec![], 1_000_000).map_err(|e| e.reason)
}

const SCOPE: &str = "is not a name in scope";

#[test]
fn else_inside_the_if_block_is_named_as_a_misplaced_keyword() {
    // The reporter's shape: `else` written before the block's closing `इति`.
    let err = run(
        "    यदि असत्यम् आदि\n        प्रत्यागमनम् १ ।\n    अन्यथा\n        प्रत्यागमनम् २ ।\n    इति\n    प्रत्यागमनम् ३ ।\n",
    )
    .expect_err("a misplaced `अन्यथा` is refused");
    assert!(
        !err.contains(SCOPE),
        "the diagnostic still blames scope, which sends the reader after a \
         missing declaration: {err}"
    );
    assert!(
        err.contains("अन्यथा"),
        "the diagnostic does not name the word: {err}"
    );
    assert!(
        err.contains("यदि") && err.contains("इति"),
        "the diagnostic does not say where `अन्यथा` belongs — after the `इति` \
         that closes a `यदि` block: {err}"
    );
}

#[test]
fn else_inside_the_block_with_its_own_block_open_is_named_too() {
    // The other natural mis-spelling: `अन्यथा आदि … इति` nested inside the block.
    let err = run(
        "    यदि असत्यम् आदि\n        प्रत्यागमनम् १ ।\n    अन्यथा आदि\n        प्रत्यागमनम् २ ।\n    इति\n    इति\n    प्रत्यागमनम् ३ ।\n",
    )
    .expect_err("a misplaced `अन्यथा आदि` is refused");
    assert!(!err.contains(SCOPE), "{err}");
    assert!(err.contains("यदि") && err.contains("इति"), "{err}");
}

#[test]
fn the_correct_form_still_takes_the_else_branch() {
    // CONTROL. The fix must not touch the one place the keyword is valid.
    let v = run(
        "    यदि असत्यम् आदि\n        प्रत्यागमनम् १ ।\n    इति अन्यथा आदि\n        प्रत्यागमनम् २ ।\n    इति\n    प्रत्यागमनम् ३ ।\n",
    )
    .expect("the documented form runs");
    assert!(
        matches!(v, Value::Int(2)),
        "the else branch answers २, got {v:?}"
    );
}

#[test]
fn a_really_undeclared_name_is_still_reported_as_scope() {
    // CONTROL. The scope diagnostic is right for a name; only a keyword in the
    // wrong place gets the new sentence.
    let err = run("    प्रत्यागमनम् अघोषितम् ।\n").expect_err("an undeclared name is refused");
    assert!(err.contains(SCOPE), "{err}");
}

#[test]
fn a_local_declared_under_the_keywords_name_is_not_refused_by_this_change() {
    // CONTROL on the neighbouring hazard, which is another row's to close: the
    // front end lets `अन्यथा` be declared. This change must not turn that
    // program into a refusal at the statement that assigns it.
    let r = run("    चरः अन्यथा ॱॱ न६४ भवति ४ ।\n    अन्यथा भवति ५ ।\n    प्रत्यागमनम् अन्यथा ।\n");
    assert!(
        matches!(r, Ok(Value::Int(5))),
        "a declared local named `अन्यथा` no longer assigns and reads back: {r:?}"
    );
}

// ── The follow-up, 2026-10-03 ────────────────────────────────────────────────
//
// The first wording ended "so it is inside a block: close the `यदि` block with
// `इति` first". That is advice, and it was stated as fact in three shapes where
// it is false. The message now says only what the parser knows — the keyword
// begins a statement, on which line — and makes the advice conditional.

const CONDITIONAL: &str = "If it is inside";

/// `run`'s source puts the body on line 3: `मण्डलम्` is line 1, the routine's
/// header line 2.
fn refused(body: &str, line: usize) -> String {
    let err = run(body).expect_err("a statement beginning with `अन्यथा` is refused");
    assert!(!err.contains(SCOPE), "{err}");
    assert!(
        err.contains(CONDITIONAL),
        "the advice is stated as fact again, not as a condition: {err}"
    );
    assert!(
        !err.contains("so it is inside a block"),
        "the old unconditional claim is back: {err}"
    );
    assert!(
        err.contains(&format!("line {line}")),
        "the diagnostic does not name line {line}: {err}"
    );
    err
}

#[test]
fn after_a_while_blocks_close_the_advice_is_conditional_and_the_line_is_named() {
    // `यावत् … इति अन्यथा आदि … इति`: there is no `यदि` here to close.
    refused(
        "    यावत् असत्यम् आदि\n        प्रत्यागमनम् १ ।\n    इति अन्यथा आदि\n        प्रत्यागमनम् २ ।\n    इति\n    प्रत्यागमनम् ३ ।\n",
        5,
    );
}

#[test]
fn a_second_else_is_refused_with_the_conditional_advice() {
    // `यदि … इति अन्यथा आदि … इति अन्यथा आदि … इति`: the first is the else,
    // the second begins a statement, and it is not inside any block.
    refused(
        "    यदि असत्यम् आदि\n        प्रत्यागमनम् १ ।\n    इति अन्यथा आदि\n        प्रत्यागमनम् २ ।\n    इति अन्यथा आदि\n        प्रत्यागमनम् ४ ।\n    इति\n    प्रत्यागमनम् ३ ।\n",
        7,
    );
}

#[test]
fn at_the_routines_top_level_with_no_if_the_advice_is_conditional() {
    refused(
        "    अन्यथा आदि\n        प्रत्यागमनम् २ ।\n    इति\n    प्रत्यागमनम् ३ ।\n",
        3,
    );
}

#[test]
fn the_reporters_shape_names_its_line_too() {
    // The original case: inside the block, on the body's third line.
    refused(
        "    यदि असत्यम् आदि\n        प्रत्यागमनम् १ ।\n    अन्यथा\n        प्रत्यागमनम् २ ।\n    इति\n    प्रत्यागमनम् ३ ।\n",
        5,
    );
}

#[test]
fn a_module_level_name_spelled_like_the_keyword_is_not_exempt() {
    // THE EXEMPTION'S LIMIT, pinned rather than accidental. The exemption is
    // `locals` — parameters and `चरः` locals. A module-level `चरः अन्यथा` is
    // not in it, so a statement beginning with that name is refused as the
    // misplaced keyword. Whether a module-level name may be spelled like a
    // keyword at all is `a-keyword-used-as-a-name-reads-as-nil`'s question.
    let src = "मण्डलम् परीक्षा ॥\nचरः अन्यथा ॱॱ न६४ भवति ० ।\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    अन्यथा भवति ५ ।\n    प्रत्यागमनम् अन्यथा ।\nइति\n";
    let r = Interpreter::load(&[("pariksha.t1", src)], &spec_root())
        .map_err(|e| format!("load: {}", e.reason))
        .and_then(|mut it| it.call("परीक्षाॱक", vec![], 1_000_000).map_err(|e| e.reason));
    let err = r.expect_err("a module-level `अन्यथा` at statement start is not exempt");
    assert!(err.contains(CONDITIONAL) && err.contains("line 4"), "{err}");
}
