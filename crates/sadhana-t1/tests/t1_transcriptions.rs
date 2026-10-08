//! `W-260` — A DEFINITION AND EVERY HAND-WRITTEN COPY OF IT, CHECKED BY
//! READING THE TREE.
//!
//! # The shape, stated once because it is here twice
//!
//! Something is defined in ONE place. Rust then transcribes it BY HAND
//! wherever it is needed, and nothing links any transcription to the
//! definition or to another transcription. They agree on the day they are
//! written, which is what makes them look safe rather than what makes them
//! safe: the copy that goes stale is the one whose reader nobody runs.
//!
//! The guard for that shape is always the same three moves — read the
//! DEFINITION from its source, DISCOVER the copies by reading the tree rather
//! than by listing paths, and compare every copy against the definition — plus
//! a fourth that the rest is worthless without: assert that the scan found
//! something, or a rename empties both sides and the guard passes having
//! compared nothing.
//!
//! Two instances live here, sharing the discovery layer because writing it
//! twice would be the very defect this file is about:
//!
//!   1. THE IR KIND TABLES — `ir.t1`'s numbering against every `match` on it.
//!   2. THE MACHINE'S LIMITS — `yantra-run`'s RAM and step budget against every
//!      `const` whose own doc comment claims to mirror them.
//!
//! They are NOT one mechanism. A numbering is discovered from match arms and a
//! limit from a `const` with a claim in its margin; forcing one reader to do
//! both would obscure both. The shape is what is shared, not the code that
//! recognises a copy.
//!
//! ── 1. THE IR KIND TABLES ─────────────────────────────────────────────────
//!
//! `ir.t1` numbers three enumerations and carries the Rust twin of each row in
//! its own margin:
//!
//! ```text
//! सार्वजनिक चरः तुलनाज्ञाभेद ॱॱ न६४ भवति १४ ।   ॰ Instruction::Cmp(CmpOp, ValueId, ValueId)
//! ```
//!
//! That is THE DEFINITION. Every Rust reader of a T1 IR arena then transcribes
//! it BY HAND as a `match` on the number, and until this file nothing linked
//! any transcription to the definition or to any other transcription.
//!
//! # What that cost
//!
//! When `ir.t1` gained eleven instruction kinds, three copies were updated by
//! whoever's tests went red. The fourth — `crates/frontend/src/pathana.rs`,
//! the demonstration's own reader — stayed at five kinds, because nothing runs
//! it over the demonstration programs. So `tools/demo.sh` refused five of its
//! six programs while the compiler computed all six. The copy that went stale
//! was THE ONE WITH AN AUDIENCE, and no test went red for it.
//!
//! # Why this is not a test that counts the copies
//!
//! Making four copies agree fixes today and nothing else: nothing would notice
//! a fifth appearing or a third drifting. So the copies are DISCOVERED, not
//! listed — every `match` arm in the workspace that maps an integer to one of
//! these three families is a copy, found by reading the tree on every run. A
//! new copy is checked the day it is written, and this file names no path.
//!
//! # What "agree" means, and why completeness is part of it
//!
//! A copy agrees when every number it decodes maps to the variant the
//! definition gives it, AND when it decodes every number the definition has.
//! The second half is not pedantry: `pathana.rs`'s defect was OMISSION, not
//! contradiction — it mapped 1..5 correctly and had never heard of 6..16. A
//! check that only compared shared keys would have passed it every day it was
//! broken.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The three enumerations `ir.t1` numbers and Rust transcribes.
const FAMILIES: &[&str] = &["Instruction", "Terminator", "CmpOp"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

/// A Devanagari numeral as a number: `१४` is 14.
fn devanagari_number(s: &str) -> Option<u32> {
    let mut n: u32 = 0;
    let mut any = false;
    for c in s.chars() {
        let d = u32::from(c).checked_sub(0x0966)?;
        if d > 9 {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(d)?;
        any = true;
    }
    any.then_some(n)
}

/// THE DEFINITION: `ir.t1`'s numbering, read from the source of truth.
///
/// One line per row — `चरः <name> ॱॱ न६४ भवति <numeral> ।` with `॰ Family::Variant`
/// in the margin — so the definition cannot drift from the thing this file
/// compares against without this file seeing it.
fn definition() -> BTreeMap<String, BTreeMap<u32, String>> {
    let path = repo_root().join("crates/sadhana-t1/src/ir.t1");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    let mut out: BTreeMap<String, BTreeMap<u32, String>> = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("सार्वजनिक चरः ") else {
            continue;
        };
        let Some((decl, margin)) = rest.split_once('॰') else {
            continue;
        };
        let Some((_, value)) = decl.split_once(" भवति ") else {
            continue;
        };
        let Some(number) = devanagari_number(value.trim().trim_end_matches('।').trim()) else {
            continue;
        };
        let margin = margin.trim();
        for family in FAMILIES {
            let Some(after) = margin.strip_prefix(&format!("{family}::")) else {
                continue;
            };
            let variant: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !variant.is_empty() {
                out.entry((*family).to_string())
                    .or_default()
                    .insert(number, variant);
            }
            break;
        }
    }
    out
}

/// `text` with every line comment and raw-string body blanked, newlines kept.
///
/// Both matter and each has already bitten this tree. A doc comment showing a
/// `match` arm is prose, not a copy; and a raw string holding Rust source is a
/// FIXTURE — this file's own controls spell wrong arms on purpose, and reading
/// them would make the guard permanently red for no defect (`W-255` met the
/// same thing in the call-site scanner).
fn strip_comments_and_raw_strings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // a raw string: r"…", r#"…"#, r##"…"##
        if bytes[i] == b'r' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'#' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                let hashes = j - i - 1;
                let close = format!("\"{}", "#".repeat(hashes));
                if let Some(rel) = text[j + 1..].find(&close) {
                    for c in text[i..j + 1 + rel + close.len()].chars() {
                        out.push(if c == '\n' { '\n' } else { ' ' });
                    }
                    i = j + 1 + rel + close.len();
                    continue;
                }
            }
        }
        // a line comment, `//` and `///` alike
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        let c = text[i..].chars().next().expect("a char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Every `<integer> =>` match arm of `text`, with its body bounded so that one
/// arm never swallows the next.
///
/// # The bound is not one rule but two, and the first draft had only one
///
/// A value arm ends at the `,` that separates it: `3 => Instruction::Add(a, b),`.
/// A BLOCK arm ends at its closing brace and carries NO comma:
///
/// ```text
/// 5 => {
///     params += 1;
///     Instruction::Param(…)
/// }
/// 15 => { … }
/// ```
///
/// Bounding everything at the comma made arm 5 run on through 15 and 16, and
/// because the reader takes the FIRST `Instruction::` in a body, arm 5 still
/// looked right while 15 and 16 vanished. The guard then reported three copies
/// as incomplete when the copies were complete and the reader was wrong. The
/// tree was fine; the instrument was not.
///
/// String literals are skipped inside the body: a `format!("{at}: …")` in an
/// arm would otherwise put a brace into the depth count.
fn numeric_arms(text: &str) -> Vec<(u32, String)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        // the number must start a token: not `x1`, not `1.5`, not `v10`
        let before = text[..i].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let Ok(number) = text[start..i].parse::<u32>() else {
            continue;
        };
        let after = text[i..].trim_start();
        if !after.starts_with("=>") {
            continue;
        }
        let mut j = i + (text[i..].len() - after.len()) + 2;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        let body_at = j;
        let block = bytes.get(j) == Some(&b'{');
        let mut depth = 0i32;
        while j < bytes.len() {
            match bytes[j] {
                b'"' => {
                    j += 1;
                    while j < bytes.len() && bytes[j] != b'"' {
                        j += if bytes[j] == b'\\' { 2 } else { 1 };
                    }
                }
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    // A block arm ends AT its closing brace and has no comma.
                    if block && depth == 0 {
                        j += 1;
                        break;
                    }
                }
                b',' if depth == 0 => break,
                _ => {}
            }
            j += 1;
        }
        out.push((number, text[body_at..j.min(bytes.len())].to_string()));
        // CONTINUE INSIDE THE BODY, not past it. `14 => Instruction::Cmp(…)`
        // holds the whole `CmpOp` table in a nested match, and skipping to the
        // end of the arm lost every copy of it — the scan found four copies of
        // `CmpOp` before this line and none after.
        i = body_at;
    }
    out
}

/// One transcription found in the tree.
#[derive(Debug)]
struct Copy {
    path: String,
    family: String,
    rows: BTreeMap<u32, String>,
}

/// Every `.rs` file under `crates/`, excluding build output.
fn rust_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            if p.is_dir() {
                walk(&p, root, out);
            } else if p.extension().is_some_and(|x| x == "rs")
                && let Ok(text) = std::fs::read_to_string(&p)
            {
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, text));
            }
        }
    }
    let root = repo_root();
    let mut out = Vec::new();
    walk(&root.join("crates"), &root, &mut out);
    out
}

/// Every transcription in the tree, found by reading it — no path is named here.
fn copies_in(sources: &[(String, String)]) -> Vec<Copy> {
    let mut out = Vec::new();
    for (path, text) in sources {
        let code = strip_comments_and_raw_strings(text);
        let mut tables: BTreeMap<String, BTreeMap<u32, String>> = BTreeMap::new();
        for (number, body) in numeric_arms(&code) {
            for family in FAMILIES {
                let needle = format!("{family}::");
                let Some(at) = body.find(&needle) else {
                    continue;
                };
                let variant: String = body[at + needle.len()..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !variant.is_empty() {
                    tables
                        .entry((*family).to_string())
                        .or_default()
                        .entry(number)
                        .or_insert(variant);
                }
                break;
            }
        }
        for (family, rows) in tables {
            out.push(Copy {
                path: path.clone(),
                family,
                rows,
            });
        }
    }
    out
}

fn copies() -> Vec<Copy> {
    copies_in(&rust_sources())
}

// ─────────────────────────────────────────────────────────────────────────
// The guard.
// ─────────────────────────────────────────────────────────────────────────

/// **THE GUARD.** Every hand-written copy of a kind table decodes exactly what
/// `ir.t1` defines — the same numbers, to the same variants, with none missing.
///
/// A failure names the file, the family, the number, and both readings, because
/// the repair differs: a wrong variant is a typo and a missing number is a copy
/// that stopped being updated.
#[test]
fn every_copy_of_a_kind_table_decodes_what_ir_t1_defines() {
    let definition = definition();
    let copies = copies();
    let mut wrong: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();

    for c in &copies {
        let Some(def) = definition.get(&c.family) else {
            unknown.push(format!(
                "{}: decodes `{}::` and ir.t1 numbers no such family",
                c.path, c.family
            ));
            continue;
        };
        for (n, variant) in &c.rows {
            match def.get(n) {
                Some(d) if d == variant => {}
                Some(d) => wrong.push(format!(
                    "{}: {} {n} decodes to `{variant}`; ir.t1 numbers {n} as `{d}`",
                    c.path, c.family
                )),
                None => unknown.push(format!(
                    "{}: {} {n} decodes to `{variant}` and ir.t1 has no {n}",
                    c.path, c.family
                )),
            }
        }
        for (n, d) in def {
            if !c.rows.contains_key(n) {
                missing.push(format!(
                    "{}: {} has no arm for {n} (`{d}`) — this copy has {} of {} rows",
                    c.path,
                    c.family,
                    c.rows.len(),
                    def.len()
                ));
            }
        }
    }

    let mut report = Vec::new();
    report.extend(wrong);
    report.extend(missing);
    report.extend(unknown);
    assert!(
        report.is_empty(),
        "{} disagreement(s) between ir.t1 and the copies that transcribe it:\n  {}",
        report.len(),
        report.join("\n  ")
    );
}

/// The scan found the definition and found copies to check.
///
/// Without this, a rename in `ir.t1` or a change of `match` style would empty
/// both sides and the guard above would pass having compared nothing — which is
/// the failure this row is about, one level up.
#[test]
fn the_kind_table_scan_is_not_vacuous() {
    let definition = definition();
    for family in FAMILIES {
        let rows = definition.get(*family).unwrap_or_else(|| {
            panic!(
                "ir.t1 numbers no `{family}` rows; the definition reader is \
                 broken or the margins were rewritten"
            )
        });
        assert!(
            rows.len() >= 4,
            "ir.t1 gives `{family}` only {} row(s); the definition reader is matching a \
             fragment of the table",
            rows.len()
        );
        // Numbering starts at 1 and has no holes: `०` means ABSENT, never a kind.
        let want: BTreeSet<u32> = (1..=rows.len() as u32).collect();
        let got: BTreeSet<u32> = rows.keys().copied().collect();
        assert_eq!(
            got,
            want,
            "ir.t1's `{family}` numbering is not 1..={}",
            rows.len()
        );
    }

    let copies = copies();
    for family in FAMILIES {
        let n = copies.iter().filter(|c| c.family == *family).count();
        assert!(
            n >= 2,
            "only {n} copy of `{family}` found in the tree; two readers transcribe every one \
             of these tables, so the arm scanner has stopped seeing them"
        );
    }
    println!("METRIC t1_kind_table_copies {}", copies.len());
    for c in &copies {
        println!("  copy: {} {} ({} rows)", c.path, c.family, c.rows.len());
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The controls: the guard can see each way a copy goes wrong.
// ─────────────────────────────────────────────────────────────────────────

/// A copy that maps a number to the WRONG variant is caught.
#[test]
fn a_drifted_arm_is_caught() {
    let def = definition();
    let inst = def.get("Instruction").expect("the definition");
    // `३` is Add. Say Sub.
    let synthetic = r#"fn f() { match k { 3 => Instruction::Sub(a, b), _ => todo!() } }"#;
    let found = copies_in(&[("synthetic.rs".into(), synthetic.into())]);
    let c = found
        .iter()
        .find(|c| c.family == "Instruction")
        .expect("the arm is read as an Instruction copy");
    assert_eq!(c.rows.get(&3).map(String::as_str), Some("Sub"));
    assert_eq!(
        inst.get(&3).map(String::as_str),
        Some("Add"),
        "ir.t1 still numbers 3 as Add; if it does not, this control is out of date"
    );
}

/// A copy that is CORRECT as far as it goes but has stopped being updated is
/// caught — `pathana.rs`'s actual defect, which comparing shared keys misses.
#[test]
fn a_copy_that_stopped_being_updated_is_caught() {
    let def = definition();
    let inst = def.get("Instruction").expect("the definition");
    assert!(
        inst.len() > 5,
        "ir.t1 has more than five instruction kinds; this control is about the copy that \
         stayed at five"
    );
    let synthetic = r#"fn f() { match k {
        1 => Instruction::ConstInt(x),
        2 => Instruction::Call(s, v),
        3 => Instruction::Add(a, b),
        4 => Instruction::Sub(a, b),
        5 => Instruction::Param(i),
        _ => todo!(),
    } }"#;
    let found = copies_in(&[("synthetic.rs".into(), synthetic.into())]);
    let c = found
        .iter()
        .find(|c| c.family == "Instruction")
        .expect("a copy");
    assert_eq!(c.rows.len(), 5, "the fixture transcribes five kinds");
    // Every row it HAS is right, which is why a shared-key comparison passes it.
    for (n, v) in &c.rows {
        assert_eq!(inst.get(n), Some(v), "row {n} of the fixture is correct");
    }
    // And the guard's own rule sees it anyway.
    let absent: Vec<u32> = inst
        .keys()
        .copied()
        .filter(|n| !c.rows.contains_key(n))
        .collect();
    assert!(
        !absent.is_empty(),
        "the completeness half of the guard must find the rows this copy never gained: {absent:?}"
    );
}

/// The arm reader does not let one arm's body leak into the next, and does not
/// read a number that is not an arm.
#[test]
fn the_arm_reader_bounds_each_arm() {
    let text = r#"match k { 1 => f(2, 3), 4 => Instruction::Sub(a) }"#;
    let arms = numeric_arms(text);
    let map: BTreeMap<u32, String> = arms.into_iter().collect();
    assert!(
        !map.get(&1).is_some_and(|b| b.contains("Instruction")),
        "arm 1 is `f(2, 3)` and must not see arm 4's body: {:?}",
        map.get(&1)
    );
    assert!(
        map.get(&4).is_some_and(|b| b.contains("Instruction::Sub")),
        "arm 4 keeps its own body: {:?}",
        map.get(&4)
    );
    assert!(
        !map.contains_key(&2) && !map.contains_key(&3),
        "`2` and `3` are arguments, not arms: {:?}",
        map.keys().collect::<Vec<_>>()
    );
}

/// Prose and fixtures are not copies.
///
/// A doc comment showing a `match` arm, and a raw string holding Rust source,
/// both look exactly like a transcription to a naive scan. This file's own
/// controls are such fixtures, so the guard would be permanently red without
/// this.
#[test]
fn a_comment_or_a_raw_string_is_not_a_copy() {
    // The outer fixture is a raw string with THREE hashes so it can hold one
    // with two; a plain string literal here would not be stripped and this
    // file's own fixture would count as a copy.
    let text = r###"/// 3 => Instruction::Sub(a, b)
const F: &str = r##"4 => Instruction::Xor(a)"##;
fn g() { match k { 5 => Instruction::Param(i), _ => () } }"###;
    let found = copies_in(&[("synthetic.rs".into(), text.into())]);
    let c = found
        .iter()
        .find(|c| c.family == "Instruction")
        .expect("the one real arm is found");
    assert_eq!(
        c.rows.keys().copied().collect::<Vec<_>>(),
        vec![5],
        "only the code arm counts, not the comment or the fixture: {:?}",
        c.rows
    );
}

// ─────────────────────────────────────────────────────────────────────────
// 2. THE MACHINE'S LIMITS — `yantra-run`'s RAM and step budget.
//
// `crates/yantra/src/bin/yantra-run.rs` decides how much memory a program gets
// and how many steps it may take. Four other files declare `const`s holding
// those numbers, and EACH ONE CARRIES A DOC COMMENT SAYING IT MIRRORS THE
// RUNNER — a claim in prose, checked by nothing. Found by a peer session auditing
// `W-262`, and it is the kind-table defect in a second place: one original,
// four transcriptions, every one asserting it matches.
//
// The claim is what makes them discoverable. A `const` whose margin names
// `yantra-run` has volunteered to be checked, so this needs no list of paths
// either — and a fifth copy that makes the same claim is checked the day it is
// written.
//
// NOT CHASED, deliberately: the other limits in the tree (`1 << 21` in
// `traps.rs`, `1 << 16` in `interpreter.rs`, budgets of 200 and 100_000) claim
// nothing about `yantra-run` and may be testing the limits themselves.
// Guessing at intent is how a correct value gets "fixed".
// ─────────────────────────────────────────────────────────────────────────

/// A small integer expression as a number: `1 << 20`, `1_000_000`, `0x1000`.
///
/// Only the forms the tree actually uses. Anything else answers `None`, and a
/// copy whose value cannot be read is REPORTED rather than skipped — a value
/// this cannot parse is a value it cannot check.
fn eval_int(expr: &str) -> Option<u64> {
    let e = expr.trim().trim_end_matches(';').trim();
    if e.is_empty() {
        return None;
    }

    // Split on the LOWEST-PRECEDENCE operator first and only OUTSIDE brackets,
    // so `20 * (1 << 20)` is a product of 20 and a shift rather than a shift of
    // `20 * (1`. `W-262` is what made this necessary: the RAM limit is written
    // `20 * (1 << 20)` so that a reader sees a decision and not a rounding.
    let split_at = |op: &str| -> Option<(&str, &str)> {
        let bytes = e.as_bytes();
        let mut depth = 0i32;
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                // `is_char_boundary` because this walks BYTES over text that is
                // not all ASCII — the tree holds consts with Devanagari in them,
                // and slicing mid-character panics. The operators are ASCII, so
                // a continuation byte can never begin one.
                _ if depth == 0 && e.is_char_boundary(i) && e[i..].starts_with(op) => {
                    return Some((&e[..i], &e[i + op.len()..]));
                }
                _ => {}
            }
            i += 1;
        }
        None
    };
    if let Some((l, r)) = split_at("<<") {
        return eval_int(l)?.checked_shl(u32::try_from(eval_int(r)?).ok()?);
    }
    if let Some((l, r)) = split_at("*") {
        return eval_int(l)?.checked_mul(eval_int(r)?);
    }
    if let Some(inner) = e.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        return eval_int(inner);
    }

    let t = e.replace('_', "");
    if let Some(hex) = t.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16).ok();
    }
    t.parse().ok()
}

/// The written value of a `const` defined anywhere in the tree, by name.
///
/// `W-262`, 2026-09-05, CHANGED WHAT THIS GUARD HAS TO READ. The runner and its
/// copies used to write the number; they now name `yantra::DEFAULT_RAM` and
/// `yantra::DEFAULT_STEPS`, which are stated once and imported. The guard's
/// subject is unchanged — every const claiming to mirror `yantra-run` holds the
/// number `yantra-run` runs with — but reaching that number now means following
/// a name to its definition.
///
/// A name defined TWICE WITH DIFFERENT TEXT answers `None`. Two definitions of
/// one name is the very drift this file exists to catch, and choosing between
/// them would be guessing at which one the caller meant.
fn const_named(name: &str, sources: &[(String, String)]) -> Option<String> {
    let bare = name.trim().rsplit("::").next()?.trim();
    if bare.is_empty() {
        return None;
    }
    let mut found: Option<String> = None;
    for (_, text) in sources {
        for line in text.lines() {
            let t = line.trim_start();
            let Some(decl) = t
                .strip_prefix("pub const ")
                .or_else(|| t.strip_prefix("const "))
            else {
                continue;
            };
            let Some((n, rest)) = decl.split_once(':') else {
                continue;
            };
            if n.trim() != bare {
                continue;
            }
            let Some((_, value)) = rest.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_end_matches(';').trim().to_string();
            match &found {
                Some(prev) if *prev != value => return None,
                _ => found = Some(value),
            }
        }
    }
    found
}

/// `eval_int`, and if that fails, a name followed to its definition.
///
/// Kept SEPARATE from `eval_int` on purpose: `eval_int` answers what a piece of
/// text says by itself, and "a name is not a number" is one of its pinned
/// answers. This is the other question — what the name resolves to in this tree
/// — and only a caller holding the tree's sources can ask it.
///
/// `depth` bounds the chase, so a constant defined as itself refuses rather
/// than hanging. An unresolvable name still answers `None`, and every caller
/// REPORTS a `None` rather than skipping it: a value this cannot read is a
/// value it cannot check.
fn eval_resolved(expr: &str, sources: &[(String, String)], depth: usize) -> Option<u64> {
    if let Some(v) = eval_int(expr) {
        return Some(v);
    }
    if depth == 0 {
        return None;
    }
    let e = expr.trim().trim_end_matches(';').trim();
    if e.is_empty()
        || !e
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
    {
        return None;
    }
    eval_resolved(&const_named(e, sources)?, sources, depth - 1)
}

/// The fallback expression of a `let <name> = …;` binding — whatever its
/// `unwrap_or` / `unwrap_or_else` hands back when the environment says nothing.
fn binding_fallback(code: &str, name: &str) -> Option<String> {
    let at = code.find(&format!("let {name} ="))?;
    let stmt = &code[at..at + code[at..].find(';')?];
    for marker in [".unwrap_or_else(||", ".unwrap_or("] {
        if let Some(i) = stmt.find(marker) {
            let inner = &stmt[i + marker.len()..];
            return Some(inner[..inner.rfind(')')?].trim().to_string());
        }
    }
    None
}

/// The two numbers `yantra-run` gives a program BY DEFAULT, read from the binary
/// that owns them: `Machine::load_elf(&image, <ram>)` and `m.run(<budget>, …)`.
///
/// **BOTH ARGUMENTS ARE LOCALS, NOT CONSTANTS**, and each is an environment
/// override over a fallback (`YANTRA_RAM`, `YANTRA_STEPS`). So this reads the
/// NAME out of the call itself — renaming a local cannot silently unhook this —
/// and then that binding's fallback. `W-262` made those fallbacks the one
/// statement; before it, five copies of the numbers had drifted behind comments
/// claiming they matched.
///
/// **RAM IS NO LONGER ONE NUMBER.** It was a literal, then a constant, and it is
/// now `yantra::ram_for(&image)` = `DEFAULT_RAM.max(extent + RAM_HEADROOM)` —
/// sized from the image's own segments, because a compiler's image reaches
/// further than the default once its runs grow. A `const` cannot mirror a
/// function of the image; the most it can mirror is the FLOOR, so the floor is
/// what this returns and what the guard below checks. A test whose image reaches
/// past the floor must CALL `ram_for` rather than copy a number, and this cannot
/// see that it failed to — the guard's reach ends at the floor, and says so
/// rather than implying more.
fn yantra_run_limits(sources: &[(String, String)]) -> (u64, u64) {
    let path = repo_root().join("crates/yantra/src/bin/yantra-run.rs");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    let code = strip_comments_and_raw_strings(&text);
    let arg_of = |needle: &str| -> Option<String> {
        let at = code.find(needle)? + needle.len();
        let rest = &code[at..];
        Some(rest[..rest.find([',', ')'])?].trim().to_string())
    };
    let resolve = |arg: &str, what: &str| -> u64 {
        // A literal or a constant name still answers directly.
        if let Some(v) = eval_resolved(arg, sources, 4) {
            return v;
        }
        let fallback = binding_fallback(&code, arg).unwrap_or_else(|| {
            panic!(
                "yantra-run's {what} argument is `{arg}`, which is neither a value this can \
                 read nor a `let {arg} = …unwrap_or…;` binding whose fallback it can read"
            )
        });
        // `ram_for` is a function of the image, and `DEFAULT_RAM` is its floor.
        if fallback.contains("ram_for") {
            return eval_resolved("yantra::DEFAULT_RAM", sources, 4).expect(
                "yantra-run sizes RAM with `ram_for`, whose floor is `yantra::DEFAULT_RAM`, \
                 and that constant must be readable for the floor to be checkable",
            );
        }
        eval_resolved(&fallback, sources, 4).unwrap_or_else(|| {
            panic!(
                "yantra-run's {what} falls back to `{fallback}` and this cannot read that \
                 value, so it cannot be checked — say the number plainly or name a constant \
                 this tree defines exactly once"
            )
        })
    };
    let ram_arg = arg_of("load_elf(&image,")
        .expect("yantra-run calls Machine::load_elf(&image, <ram>) and this reads that argument");
    let budget_arg =
        arg_of("m.run(").expect("yantra-run calls m.run(<budget>, …) and this reads that argument");
    (
        resolve(&ram_arg, "RAM"),
        resolve(&budget_arg, "step budget"),
    )
}

/// One `const` that claims, in its own margin, to mirror `yantra-run`.
#[derive(Debug)]
struct ClaimedLimit {
    path: String,
    name: String,
    /// The value as written, e.g. `1 << 20`.
    written: String,
    /// Which limit the margin claims: the RAM or the step budget.
    claims_budget: bool,
}

/// Every `const` in the tree whose doc comment names `yantra-run`.
///
/// The margin is read for `run(` to tell a budget from a RAM, and the const's
/// own name is the fallback. A claim this cannot classify is reported, not
/// dropped.
fn claimed_limits(sources: &[(String, String)]) -> Vec<ClaimedLimit> {
    let mut out = Vec::new();
    for (path, text) in sources {
        if path.ends_with("t1_transcriptions.rs") {
            continue; // this file's own prose names yantra-run
        }
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim_start();
            let Some(decl) = t.strip_prefix("const ") else {
                continue;
            };
            // gather the doc comment immediately above
            let mut doc = String::new();
            let mut k = i;
            while k > 0 {
                let prev = lines[k - 1].trim_start();
                if prev.starts_with("///") || prev.starts_with("//!") {
                    doc = format!("{prev}\n{doc}");
                    k -= 1;
                } else {
                    break;
                }
            }
            // A MENTION IS NOT A CLAIM, and the difference is what the margin
            // QUOTES — a claim quotes the runner's own call, `load_elf(` or
            // `run(`.
            //
            // THIS POPULATION IS EMPTY TODAY AND THAT IS `W-262` SUCCEEDING, not
            // the scan going blind. The four copies that existed when this was
            // written each quoted `Machine::load_elf(&image, 1 << 20)` or
            // `m.run(1_000_000, …)`; all four now IMPORT `yantra::DEFAULT_RAM`
            // and `DEFAULT_STEPS` instead, and an import cannot drift from the
            // thing it imports. The check stays as a tripwire for the next copy
            // somebody writes by hand; the population it guards is counted by
            // `the_limit_scan_is_not_vacuous`, which floors claims PLUS imports
            // precisely because the conversion moved every member across.
            //
            // Two other consts name `yantra-run` in prose and mirror nothing:
            // `KERNEL_RAM`, which is 32 MiB *because* it is "too large for
            // yantra-run's 1 MiB", and `DIAGNOSTIC_PREFIXES`, which is about
            // what the runner PRINTS. Reading a mention as a claim reported
            // both as drifted copies; they are neither copies nor drifted.
            let quotes_call = doc.contains("load_elf(") || doc.contains("run(");
            if !doc.contains("yantra-run") || !quotes_call {
                continue;
            }
            let Some((name, rest)) = decl.split_once(':') else {
                continue;
            };
            let Some((_, value)) = rest.split_once('=') else {
                continue;
            };
            out.push(ClaimedLimit {
                path: path.clone(),
                name: name.trim().to_string(),
                written: value.trim().trim_end_matches(';').trim().to_string(),
                // The NAME decides when the margin quotes both calls, which
                // it does where one doc comment covers a pair of consts.
                claims_budget: if name.trim().contains("BUDGET") {
                    true
                } else if name.trim().contains("RAM") {
                    false
                } else {
                    doc.contains("run(")
                },
            });
        }
    }
    out
}

/// Every `const` that TAKES its number from the one statement instead of
/// restating it: `const RAM: usize = yantra::DEFAULT_RAM;`.
///
/// THE POPULATION THIS FILE GUARDS MOVED, AND THIS IS WHERE IT MOVED TO. Before
/// `W-262` there were four transcriptions, each a number with a margin claiming
/// it matched the runner; `claimed_limits` found them and checked them. They are
/// now imports, and an import CANNOT drift — which is the point, and which also
/// means `claimed_limits` correctly finds almost nothing. Counting only claims
/// after that change would leave a green guard over an empty set.
///
/// A const counts here when its value is a NAME (not a number) that resolves to
/// one of the runner's two limits. That is deliberately narrow: a const holding
/// `KERNEL_RAM` or some other definition's name resolves to a different number
/// and is not counted.
fn imported_limits(sources: &[(String, String)], ram: u64, budget: u64) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (path, text) in sources {
        if path.ends_with("t1_transcriptions.rs") {
            continue;
        }
        for line in text.lines() {
            let t = line.trim_start();
            let Some(decl) = t
                .strip_prefix("pub const ")
                .or_else(|| t.strip_prefix("const "))
            else {
                continue;
            };
            let Some((name, rest)) = decl.split_once(':') else {
                continue;
            };
            let Some((_, value)) = rest.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_end_matches(';').trim();
            if eval_int(value).is_some() {
                continue; // a number, not an import
            }
            if matches!(eval_resolved(value, sources, 4), Some(v) if v == ram || v == budget) {
                out.push((path.clone(), name.trim().to_string()));
            }
        }
    }
    out
}

/// **THE GUARD.** Every `const` that says it mirrors `yantra-run` holds the
/// number `yantra-run` actually uses.
#[test]
fn every_constant_claiming_to_mirror_yantra_run_does() {
    let sources = rust_sources();
    let (ram, budget) = yantra_run_limits(&sources);
    let claims = claimed_limits(&sources);
    let mut wrong = Vec::new();
    for c in &claims {
        let want = if c.claims_budget { budget } else { ram };
        let quantity = if c.claims_budget {
            "step budget"
        } else {
            "RAM"
        };
        match eval_resolved(&c.written, &sources, 4) {
            Some(v) if v == want => {}
            Some(v) => wrong.push(format!(
                "{}: `{}` = {} ({v}) and claims to mirror yantra-run's {quantity}, which is {want}",
                c.path, c.name, c.written
            )),
            None => wrong.push(format!(
                "{}: `{}` = `{}` claims to mirror yantra-run's {quantity} and this cannot read \
                 that value, so it cannot be checked — say the number plainly, name a constant \
                 this tree defines exactly once, or drop the claim",
                c.path, c.name, c.written
            )),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} constant(s) claim to mirror yantra-run and do not:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// The limit scan found the runner's numbers and found claims to check.
#[test]
fn the_limit_scan_is_not_vacuous() {
    let sources = rust_sources();
    let (ram, budget) = yantra_run_limits(&sources);
    assert!(
        ram > 0 && budget > 0,
        "yantra-run's limits read as {ram}/{budget}"
    );
    let claims = claimed_limits(&sources);
    let imports = imported_limits(&sources, ram, budget);
    println!("METRIC yantra_run_ram {ram}");
    println!("METRIC yantra_run_budget {budget}");
    println!("METRIC yantra_run_claimed_copies {}", claims.len());
    println!("METRIC yantra_run_imported_copies {}", imports.len());
    for (path, name) in &imports {
        println!("  import: {path} {name}");
    }

    // THE COUNT IS OF BOTH FORMS, BECAUSE `W-262` CONVERTED ONE INTO THE OTHER.
    // Four files transcribed these numbers when this test was written and the
    // threshold was four claims; those four are now imports, and a threshold on
    // claims alone would have been satisfied by deleting every copy. What must
    // stay true is that this scan still SEES the places that use the runner's
    // limits — by either route — so that the guard above is checking a
    // population rather than an empty set.
    //
    // WHAT THIS FLOOR DOES NOT CATCH, SAID PLAINLY: eight imports stand here and
    // the floor is four, so four of them could be repointed at another
    // definition and this would stay green — the count would fall to four and
    // the metric above would show it, but nothing would fail. The floor is set
    // at the population that existed when the rule was written, not at today's
    // count, so that adding or retiring a test is not a red build. The METRIC
    // line is what a reader watches for that drift.
    assert!(
        claims.len() + imports.len() >= 4,
        "only {} const(s) take yantra-run's limits, by claim ({}) or import ({}); four files did \
         when this was written, so the scan has stopped seeing them",
        claims.len() + imports.len(),
        claims.len(),
        imports.len()
    );

    // THE BLIND SPOT, COUNTED RATHER THAN LEFT SILENT. A const takes its claim
    // from the doc comment directly above it, so where ONE doc covers a PAIR the
    // second const carries no claim and is not checked. `riscv64_emitter.rs` is
    // still that shape — one margin, then `const RAM` and `const BUDGET` — though
    // since `W-262` its margin quotes no call at all: it says the limits are
    // "IMPORTED rather than restated", which is a mention and not a claim, so
    // neither const is a claim today and the pair is reached by the import scan
    // instead. Inheriting the neighbour's doc would be guessing at which const
    // the margin meant; saying how many are unchecked is not.
    let claimed: BTreeSet<(&str, &str)> = claims
        .iter()
        .map(|c| (c.path.as_str(), c.name.as_str()))
        .collect();
    let mut unchecked = Vec::new();
    for (path, text) in &rust_sources() {
        if path.ends_with("t1_transcriptions.rs") || !claims.iter().any(|c| &c.path == path) {
            continue;
        }
        for line in text.lines() {
            let Some(decl) = line.trim_start().strip_prefix("const ") else {
                continue;
            };
            let Some((name, _)) = decl.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if (name.contains("RAM") || name.contains("BUDGET"))
                && !claimed.contains(&(path.as_str(), name))
            {
                unchecked.push(format!("{path}: {name}"));
            }
        }
    }
    println!(
        "METRIC yantra_run_unchecked_limit_consts {}",
        unchecked.len()
    );
    // Most of these are not blind spots at all — `HOST_RAM`, `HOST_BUDGET`,
    // `KERNEL_RAM`, `KERNEL_BUDGET` mirror `yantra-host` and the kernel
    // configuration, which are OTHER definitions with other numbers. The line
    // says "not checked here" rather than "unchecked copy" for that reason:
    // one of them (`riscv64_emitter.rs`'s `BUDGET`) is a real copy whose margin
    // is shared with its neighbour, and the rest are someone else's constants.
    for u in &unchecked {
        println!("  not checked here (no margin of its own, or another definition's): {u}");
    }
    for c in &claims {
        println!(
            "  claim: {} {} = {} ({})",
            c.path,
            c.name,
            c.written,
            if c.claims_budget { "budget" } else { "RAM" }
        );
    }
}

/// A copy that drifts from the runner is caught, and a `const` that makes no
/// claim is not a copy.
#[test]
fn a_drifted_limit_is_caught_and_an_unclaimed_constant_is_not_a_copy() {
    let fixture = r#"
/// `yantra-run`'s machine: `Machine::load_elf(&image, 1 << 20)`.
const RAM: usize = 1 << 21;
/// Nothing to do with the runner.
const OTHER: usize = 1 << 16;
"#;
    let found = claimed_limits(&[("synthetic.rs".to_string(), fixture.to_string())]);
    assert_eq!(
        found.len(),
        1,
        "only the claiming const is a copy: {found:?}"
    );
    assert_eq!(found[0].name, "RAM");
    assert!(
        !found[0].claims_budget,
        "its margin names load_elf, not run"
    );
    let (ram, _) = yantra_run_limits(&rust_sources());
    assert_ne!(
        eval_int(&found[0].written),
        Some(ram),
        "the fixture drifts from the runner on purpose; if `1 << 21` is now yantra-run's RAM, \
         this control is out of date"
    );
}

/// The value reader handles the forms the tree writes, and refuses what it
/// cannot evaluate rather than guessing.
#[test]
fn the_value_reader_reads_the_forms_the_tree_uses() {
    assert_eq!(eval_int("1 << 20"), Some(1_048_576));
    assert_eq!(eval_int("1_000_000"), Some(1_000_000));
    assert_eq!(eval_int("0x8000_0000"), Some(0x8000_0000));
    assert_eq!(eval_int(" 1 << 20 ;"), Some(1_048_576));
    assert_eq!(eval_int("RAM * 2"), None, "a name is not a number here");

    // `W-262` added this form to the tree: the RAM limit is deliberately not a
    // power of two, so the reader has to hold a product and a bracket.
    assert_eq!(eval_int("20 * (1 << 20)"), Some(20_971_520));
    assert_eq!(
        eval_int("(1 << 20) * 20"),
        Some(20_971_520),
        "the bracket may come first"
    );
    assert_eq!(
        eval_int("2 * 1 << 3"),
        Some(16),
        "`<<` binds looser than `*`, as it does in Rust: (2*1) << 3"
    );
    assert_eq!(eval_int("(1 << 20"), None, "an unclosed bracket is refused");
}

/// The name resolver follows a constant to its definition, and refuses a name
/// it cannot resolve to exactly one value.
///
/// `W-262` turned the runner's literals into imports, so this reader is what
/// stands between the guard and a value it cannot see. The refusals matter as
/// much as the successes: each `None` here is a case the guard REPORTS instead
/// of passing silently, which is the difference between a check and a comment.
#[test]
fn the_name_resolver_follows_a_constant_to_its_definition() {
    let tree = |s: &str| vec![("synthetic.rs".to_string(), s.to_string())];

    let one = tree("pub const DEFAULT_RAM: usize = 20 * (1 << 20);\n");
    assert_eq!(
        const_named("yantra::DEFAULT_RAM", &one).as_deref(),
        Some("20 * (1 << 20)"),
        "a qualified path resolves by its last segment"
    );

    let plain = tree("pub const DEFAULT_STEPS: u64 = 1_000_000;\n");
    assert_eq!(
        eval_resolved("DEFAULT_STEPS", &plain, 4),
        Some(1_000_000),
        "a name resolves to the number its definition writes"
    );
    assert_eq!(
        eval_resolved("1 << 20", &plain, 4),
        Some(1_048_576),
        "a literal still reads as itself"
    );

    // TWO DEFINITIONS OF ONE NAME IS THE DRIFT THIS FILE EXISTS TO CATCH, so it
    // refuses rather than picking one.
    let twice = tree("const R: usize = 1 << 20;\nconst R: usize = 1 << 21;\n");
    assert_eq!(const_named("R", &twice), None);
    assert_eq!(eval_resolved("R", &twice, 4), None);

    // Agreeing duplicates are not drift.
    let agree = tree("const R: usize = 1 << 20;\nconst R: usize = 1 << 20;\n");
    assert_eq!(eval_resolved("R", &agree, 4), Some(1_048_576));

    assert_eq!(
        eval_resolved("NOWHERE", &plain, 4),
        None,
        "an undefined name is reported, not skipped"
    );
    assert_eq!(
        eval_resolved("RAM * 2", &plain, 4),
        None,
        "an expression over names is still not a number"
    );

    // A constant defined as itself refuses instead of hanging.
    let cyclic = tree("const A: usize = B;\nconst B: usize = A;\n");
    assert_eq!(eval_resolved("A", &cyclic, 4), None);
}

/// The binding reader finds a fallback behind either `unwrap_or` form, and
/// REFUSES rather than inventing one when the shape is not there.
///
/// Its own positive control. `yantra_run_limits` stopped working the day both of
/// the runner's arguments became locals over an environment override, and the
/// failure was a bare `expect` on a name this could not resolve. A reader with no
/// test of its own fails that way again the next time the shape moves; a reader
/// with one says which shape it stopped recognising.
#[test]
fn the_binding_reader_finds_a_fallback_and_refuses_when_there_is_none() {
    let code = "\
    let ram = std::env::var(\"YANTRA_RAM\").ok().unwrap_or_else(|| yantra::ram_for(&image));\n\
    let steps = std::env::var(\"YANTRA_STEPS\").ok().unwrap_or(DEFAULT_STEPS);\n\
    let plain = 7;\n";
    assert_eq!(
        binding_fallback(code, "ram").as_deref(),
        Some("yantra::ram_for(&image)"),
        "the closure form"
    );
    assert_eq!(
        binding_fallback(code, "steps").as_deref(),
        Some("DEFAULT_STEPS"),
        "the value form"
    );
    assert_eq!(
        binding_fallback(code, "plain"),
        None,
        "a binding with no fallback answers None rather than guessing"
    );
    assert_eq!(
        binding_fallback(code, "absent"),
        None,
        "a name that is not bound at all answers None"
    );
}

/// A const that MENTIONS `yantra-run` without claiming to mirror it is not a
/// copy — the two near-misses in the tree, kept as the fixture.
///
/// This is the rule's whole difficulty. Both of these name the runner and
/// neither transcribes it: one is sized *because* the runner's memory is too
/// small, and the other is about what the runner prints.
#[test]
fn a_mention_of_the_runner_is_not_a_claim_to_mirror_it() {
    let fixture = r#"
/// The kernel configuration's RAM: 32 MiB, so that `compositor.sas`'s address space —
/// too large for `yantra-run`'s 1 MiB — fits with room.
const KERNEL_RAM: usize = 1 << 25;
/// The prefixes `yantra-run` and `yantra-host` write at the head of a diagnostic line.
const DIAGNOSTIC_PREFIXES: &[&str] = &["yantra-run:"];
/// `yantra-run`'s machine: `Machine::load_elf(&image, 1 << 20)`.
const RAM: usize = 1 << 20;
"#;
    let found = claimed_limits(&[("synthetic.rs".to_string(), fixture.to_string())]);
    let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["RAM"],
        "only the const quoting the runner's own call is a copy: {found:?}"
    );
}

// ── an arena bound and the table it bounds ────────────────────────────────

/// How a bound in `ir.t1` relates to the highest entry of the table it bounds.
///
/// **THE BOUNDS DO NOT SHARE ONE CONVENTION, AND UNTIL `1cc531bd` THIS TEST
/// ASSERTED ONE RULE FOR ALL OF THEM.** `रचितशेषसीमा` is the highest shape number and its
/// loop restates the bound (`... योगः १`); `अपूर्णहेतुसीमा` was re-founded as one
/// PAST the highest cause so its loops need not. Both are correct; a single rule
/// cannot hold for both, and forcing one by changing a number would move an arena
/// under live loops.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Convention {
    /// `bound == highest`. The walk is `यावत् क न्यूनम् आरभ्य सीमा योगः १ समाप्तम्` —
    /// it restates the bound, so the last number walked IS the bound.
    Inclusive,
    /// `bound == highest + 1`. The walk is `यावत् क न्यूनम् सीमा` — the last number
    /// walked is one BELOW the bound.
    Exclusive,
}

impl Convention {
    /// The bound this convention demands for a table whose highest entry is
    /// `highest`.
    fn bound_for(self, highest: u32) -> u32 {
        match self {
            Convention::Inclusive => highest,
            Convention::Exclusive => highest + 1,
        }
    }

    /// The lowest number that is OUTSIDE the arena a bound of `bound` sizes.
    fn first_outside(self, bound: u32) -> u32 {
        match self {
            Convention::Inclusive => bound + 1,
            Convention::Exclusive => bound,
        }
    }
}

/// The hand-maintained bounds in `ir.t1`, the Rust table each one bounds, the
/// file that table lives in, and the convention each is DECLARED to follow. Both
/// halves of the comparison are literals, in different files and different
/// languages, and nothing compared them until `W-264`.
///
/// **THE FILE IS A COLUMN AND NOT A CONSTANT** since `अपूर्णस्थानसीमा` joined:
/// `STUB_CAUSES` and `LOWERED_SHAPES` live in `paradigm_encode.rs` because THAT
/// file turns their arena numbers into METRIC names, and `CAUSE_27_SITES` lives
/// in `t1_cause27_sites.rs` for exactly the same reason — it is the file that
/// reads `अपूर्णस्थानगणनाकोश`. A copy of the site names in `paradigm_encode.rs`
/// would have been a FOURTH hand-maintained list with no consumer, and a
/// sixteenth site added to the real one would still have been silent.
///
/// The convention column is a PIN, not the source of truth: the test derives the
/// convention from `ir.t1`'s own loops and reds when the two disagree. Deriving
/// it alone would be worse than useless — a loop silently changed from `न्यूनम् सीमा`
/// to `न्यूनम् आरभ्य सीमा योगः १ समाप्तम्` would move the arena by one and the bound
/// check would re-derive itself into agreement, reporting green. The column makes
/// that edit red and demands both sides move together.
struct Bound {
    /// The `सार्वजनिक चरः` in `ir.t1` that sizes the arena.
    name: &'static str,
    /// The ASCII key this bound's numbers carry in `.loop/METRICS.tsv`.
    ///
    /// **WRITTEN DOWN RATHER THAN TRANSLITERATED**, because a metric name is an
    /// identifier: deriving `apurnahetusima` from `अपूर्णहेतुसीमा` in code would
    /// rename every row of the dashboard the day the derivation changed, and the
    /// old rows would read as DELETED.
    metric: &'static str,
    /// The file the table lives in — beside the code that reads its arena.
    file: &'static str,
    /// The `&[(i128, &str)]` table this bound bounds.
    table: &'static str,
    /// The convention this bound is DECLARED to follow. A PIN, cross-checked
    /// against `ir.t1`'s own loops.
    declared: Convention,
    /// What one entry IS, so a refusal reads without opening `ir.t1`.
    what: &'static str,
}

const BOUNDS: &[Bound] = &[
    Bound {
        name: "अपूर्णहेतुसीमा",
        metric: "apurnahetusima",
        file: "crates/yantra/tests/paradigm_encode.rs",
        table: "STUB_CAUSES",
        declared: Convention::Exclusive,
        what: "a stub CAUSE — `अपूर्णध्रुवम्` writes one per refused shape",
    },
    Bound {
        name: "रचितशेषसीमा",
        metric: "rachitashesasima",
        file: "crates/yantra/tests/paradigm_encode.rs",
        table: "LOWERED_SHAPES",
        declared: Convention::Inclusive,
        what: "a lowered SHAPE — `रचितगणनम्` writes one per lowered form",
    },
    Bound {
        name: "अपूर्णस्थानसीमा",
        metric: "apurnasthanasima",
        file: "crates/yantra/tests/t1_cause27_sites.rs",
        table: "CAUSE_27_SITES",
        declared: Convention::Exclusive,
        what: "a cause-२७ SITE — `अपूर्णध्रुवस्थाने` writes one per index-arm refusal",
    },
];

/// `(number, name)` for every row of a `&[(i128, &str)]` table in `file`, read
/// from the source rather than linked, because this crate cannot depend on
/// another crate's test binary — and because the three tables live in two files,
/// each beside the code that reads its arena.
fn census_table(file: &str, name: &str) -> BTreeMap<u32, String> {
    let path = repo_root().join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    let head = format!("const {name}: &[(i128, &str)] = &[");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("{name} must exist in {file}"))
        + head.len();
    let body = &text[start..start + text[start..].find("];").expect("the table ends")];
    let mut out = BTreeMap::new();
    for line in body.lines() {
        let line = line.trim();
        // `(24, "name_global_arena"),` — and NOT a commented-out row, which is
        // how a retired entry is kept visible in these tables.
        if line.starts_with("//") {
            continue;
        }
        let Some(rest) = line.strip_prefix('(') else {
            continue;
        };
        let Some((num, tail)) = rest.split_once(',') else {
            continue;
        };
        let Ok(n) = num.trim().parse::<u32>() else {
            continue;
        };
        let name = tail.trim().trim_start_matches('"');
        let Some(end) = name.find('"') else { continue };
        out.insert(n, name[..end].to_string());
    }
    out
}

/// One `सार्वजनिक चरः <name> ॱॱ न६४ भवति <numeral> ।` from `ir.t1`.
fn ir_bound(name: &str) -> u32 {
    let path = repo_root().join("crates/sadhana-t1/src/ir.t1");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(&format!("सार्वजनिक चरः {name} "))
        else {
            continue;
        };
        let Some((_, value)) = rest.split_once(" भवति ") else {
            continue;
        };
        let value = value.split('॰').next().unwrap_or(value);
        if let Some(n) = devanagari_number(value.trim().trim_end_matches('।').trim()) {
            return n;
        }
    }
    panic!("ir.t1 declares no `{name}`");
}

/// The convention `ir.t1`'s OWN LOOPS use for `name`, read off the walks that
/// `ir_walked_arena_bounds` already parsed.
///
/// **THIS ONCE PARSED THE WALK A SECOND TIME, AND THE TWO READERS COULD
/// DISAGREE.** It searched for two hand-written spellings — `आरभ्य {name} योगः १
/// समाप्तम्` and `न्यूनम् {name} ` — while `ir_walked_arena_bounds` read the same
/// grammar POSITIONALLY. A walk the positional reader finds and the string
/// reader misses panicked `no loop in ir.t1 walks an arena with {name}`, which
/// names the wrong defect outright: the bound IS walked, and it is this file's
/// reader that cannot spell it. Two parsers of one grammar in one file is the
/// defect this file's own header is about, so there is now one.
///
///   ॱ `यावत् <var> न्यूनम् आरभ्य <name> योगः १ समाप्तम् आदि` — restates the bound,
///     INCLUSIVE.
///   ॱ `यावत् <var> न्यूनम् <name> आदि` — does not, EXCLUSIVE.
///
/// **A BOUND WALKED BOTH WAYS IS ITS OWN DEFECT AND IS REFUSED HERE**, not
/// resolved by majority. `ir.t1:919` records that a second convention inside one
/// reset produced two silent defects in 24 hours on 2026-09-16/17; two loops
/// disagreeing means one of them is already off by one, and picking a winner
/// would hide it.
///
/// **A THIRD READING IS ALSO A REFUSAL AND NOT A GUESS.** `आरभ्य <name> योगः २
/// समाप्तम्` is a real offset this file has no name for. The string reader passed
/// over such a line in SILENCE — neither spelling matched — and a positional
/// reader that only asked whether `आरभ्य` was present would have called it
/// INCLUSIVE and been off by one. `Walk::convention` is `None` there, and the
/// refusal quotes the words it actually read.
///
/// Returns `Err` rather than panicking so that one unreadable bound does not
/// hide the state of the other two: every `BOUNDS` row is compared and the
/// findings are reported together.
fn ir_bound_convention(
    name: &str,
    walked: &BTreeMap<String, Vec<Walk>>,
) -> Result<Convention, String> {
    let Some(walks) = walked.get(name) else {
        return Err(format!(
            "no ir.t1 loop walks an arena with `{name}`, so its convention cannot \
             be derived — a bound with no walk bounds nothing, and this test would \
             otherwise pin a number against a table by a hand-written column \
             alone. Either the name in `BOUNDS` is stale, or ir.t1 now ASSIGNS \
             `{name}` and it is a live count no fixed table can equal."
        ));
    };
    let unreadable: Vec<String> = walks
        .iter()
        .filter(|w| w.convention.is_none())
        // An EMPTY tail quoted as `... ` reads as if nothing were wrong. The
        // walk that ends at the bound is a real form — the `आदि` dropped — and
        // it is the one the old string reader missed too.
        .map(|w| {
            if w.tail.is_empty() {
                format!("ir.t1:{} ENDS at the bound, nothing follows it", w.line)
            } else {
                format!("ir.t1:{} reads `... {}`", w.line, w.tail)
            }
        })
        .collect();
    if !unreadable.is_empty() {
        return Err(format!(
            "`{name}` is walked in a form this file has no convention for: {}. \
             A walk is INCLUSIVE only as `न्यूनम् आरभ्य {name} योगः १ समाप्तम् आदि` and \
             EXCLUSIVE only as `न्यूनम् {name} आदि`; anything else is a different \
             offset, and reading it as either would move the arena by one under a \
             live table. Teach `ir_walked_arena_bounds` the form, or put the loop \
             back.",
            unreadable.join(", ")
        ));
    }
    let first = walks[0].convention.expect("no walk is unreadable here");
    let split: Vec<String> = walks
        .iter()
        .filter(|w| w.convention != Some(first))
        .map(|w| format!("ir.t1:{} is {:?}", w.line, w.convention.unwrap()))
        .collect();
    if !split.is_empty() {
        return Err(format!(
            "`{name}` is walked BOTH WAYS in ir.t1: ir.t1:{} is {first:?} but {}. \
             One of those loops is already off by one — a second convention inside \
             one bound is the shape that produced two silent defects in 24 hours \
             on 2026-09-16/17. Make every walk of `{name}` agree before this test \
             can say anything about its number.",
            walks[0].line,
            split.join(", ")
        ));
    }
    Ok(first)
}

/// ॥ EACH ARENA BOUND EQUALS THE HIGHEST ENTRY OF THE TABLE IT BOUNDS ॥ `W-264`,
/// per its OWN CONVENTION since `1cc531bd`.
///
/// `ir.t1` bounds three counter arenas with hand-written literals, and the
/// censuses that read them carry the matching tables in OTHER FILES, in ANOTHER
/// LANGUAGE. Nothing compared the two sides, and both directions of the mismatch
/// have happened, days apart, with different symptoms:
///
///   ॱ **BOUND TOO LOW** — a shape or cause numbered at or above the arena's end
///     indexes past it and the pass DIES. Its fault says `entry 29 is outside an
///     arena of 29`, which names neither the bound nor what overflowed it; twice
///     that was the first notification, and both times it was recognised only by
///     an operator who had met it before. **A diagnostic that requires having
///     already met it is a mnemonic, not a diagnostic.**
///   ॱ **BOUND TOO HIGH** — a number the arena clears and the census has no NAME
///     for. That one is SILENT, and it happened on 2026-09-07: `अपूर्णहेतुसीमा`
///     read २६ while `STUB_CAUSES` ended at 25, so an entire residue population
///     was written, cleared, counted by `ir.t1` — and excluded from the census's
///     print AND from its total, because that total is a sum over NAMED causes.
///     "No line for cause 26" and "cause 26 is zero" are the same output.
///     **This assertion would have caught it before the run rather than after.**
///
/// EQUALITY, NOT `>=`, for that second reason. A bound above its table is not
/// slack, it is an unnamed population.
///
/// **AND EQUALITY TO WHAT IS PER-BOUND.** `1cc531bd` re-founded `अपूर्णहेतुसीमा` as
/// EXCLUSIVE — `ir.t1:914` and `:1638` walk `यावत् हेतुः न्यूनम् अपूर्णहेतुसीमा`, so 45
/// bounds causes १..४४ and `STUB_CAUSES`'s highest of 44 AGREES. `रचितशेषसीमा` did
/// not move: `ir.t1:928` walks `यावत् रूपम् न्यूनम् आरभ्य रचितशेषसीमा योगः १ समाप्तम्`,
/// so 34 bounds shapes १..३४ and `LOWERED_SHAPES`'s highest of 34 agrees too.
/// `अपूर्णस्थानसीमा` joined third and is EXCLUSIVE by the same reading — `ir.t1:923`
/// walks `यावत् स्थलाङ्कः न्यूनम् अपूर्णस्थानसीमा`, so १६ bounds sites १..१५ and
/// `CAUSE_27_SITES`'s highest of 15 agrees. **It arrived GREEN, so the landing is
/// not its falsifier and a MUTATION is**: a sixteenth site reds as OUTSIDE the
/// arena, and the bound raised to १७ reds as bounded-and-UNNAMED. Before this row
/// it was the one bound of this kind with no table at all — `W-264` recorded it
/// as `METRIC w264_bounds_unguarded` rather than assume proximity covered it.
/// Asserting `bound == highest` for both reported the exclusive one red at HEAD
/// from `64a93973` — **and NEITHER NUMBER WAS WRONG.** Changing either to satisfy
/// one rule would have moved a live arena by one.
///
/// The convention is DERIVED from `ir.t1`'s loops and CROSS-CHECKED against the
/// column in `BOUNDS`, so three different things now red separately: a number
/// that drifted from its table, a loop whose convention drifted from the column,
/// and a bound walked both ways at once.
///
/// DERIVED FROM BOTH SIDES AND MATCHED BY NUMBER, never by cardinality — the
/// shape `every_copy_of_a_kind_table_decodes_what_ir_t1_defines` above uses, and
/// for the same reason: two tables of equal length can disagree on every row.
#[test]
fn each_arena_bound_equals_the_highest_entry_of_the_table_it_bounds() {
    let mut wrong: Vec<String> = Vec::new();
    println!("METRIC w264_arena_bounds_compared {}", BOUNDS.len());
    // ONE reader of the walk grammar, shared with the census below since
    // `W-303`. The guard is this test's too: a rename of `यावत्` or `न्यूनम्`
    // empties the map, and every convention would then be un-derivable for the
    // same reason at once — which reads as three stale `BOUNDS` names unless
    // the scan says it read nothing.
    let (walked, scanned) = ir_walked_arena_bounds();
    assert!(
        scanned > 0,
        "no line of ir.t1 both starts `यावत् ` and contains `न्यूनम्`, so the walk \
         reader read NO walks at all — the loop keyword or the comparison was \
         renamed, and no bound's convention can be derived. Teach \
         `ir_walked_arena_bounds` the new spelling."
    );
    for Bound {
        name: bound_name,
        metric,
        file,
        table: table_name,
        declared,
        what,
    } in BOUNDS
    {
        let derived = match ir_bound_convention(bound_name, &walked) {
            Ok(derived) => derived,
            Err(why) => {
                wrong.push(why);
                continue;
            }
        };
        println!(
            "METRIC w264_bound_{metric}_convention {}",
            format!("{derived:?}").to_lowercase()
        );
        if derived != *declared {
            wrong.push(format!(
                "`{bound_name}` is declared {declared:?} in BOUNDS but ir.t1's own \
                 loops walk it {derived:?}. The arena moved by one under the \
                 table: move the number AND this column in the same edit, or put \
                 the loop back. Each entry is {what}."
            ));
            continue;
        }
        let bound = ir_bound(bound_name);
        let table = census_table(file, table_name);
        let highest = *table.keys().max().expect("the table has rows");
        println!("METRIC w264_bound_{metric} {bound}");
        println!(
            "METRIC w264_{}_highest {highest}",
            table_name.to_lowercase()
        );
        let expected = declared.bound_for(highest);
        let first_outside = declared.first_outside(bound);
        if bound < expected {
            let over: Vec<String> = table
                .range(first_outside..)
                .map(|(n, name)| format!("{n} `{name}`"))
                .collect();
            wrong.push(format!(
                "{table_name} numbers {} OUTSIDE the arena: {}. \
                 `{bound_name}` is {bound} in ir.t1 and must be {expected} — it is \
                 {declared:?}, so it bounds १..{} — raise it in ir.t1, in the same \
                 edit as the entry. Each is {what}, and one numbered past the arena \
                 indexes outside it, which faults as `entry N is outside an arena \
                 of N` and names neither side.",
                over.len(),
                over.join(", "),
                first_outside - 1
            ));
        } else if bound > expected {
            wrong.push(format!(
                "`{bound_name}` is {bound} in ir.t1 and is {declared:?}, so it \
                 bounds १..{}, but {table_name}'s highest entry is {highest} — {} \
                 number(s) are bounded and UNNAMED. An unnamed entry is excluded \
                 from the census's print AND from its total, so it reads exactly \
                 like a zero — name {} in {table_name}, or lower the bound to \
                 {expected}. Each is {what}.",
                first_outside - 1,
                first_outside - 1 - highest,
                (highest + 1..first_outside)
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "an arena bound and the table it bounds disagree:\n  {}",
        wrong.join("\n  ")
    );
}

// ── every bound ir.t1 WALKS, and not only the ones someone wrote down ──────

/// A bound `ir.t1` walks that legitimately has NO `&[(i128, &str)]` table, with
/// the reason written down WHERE THE CHECK READS IT.
///
/// **THIS LIST IS WHY THE GUARD BELOW CANNOT BE SILENCED BY DELETION.** A census
/// that reds on a bound with no table and offers no way to say "this one has
/// none, and here is why" becomes a nuisance the first time a legitimate one
/// arrives, and a nuisance gets worked around — which is exactly how the first
/// `the_state_file_describes_this_tree` was defeated. The escape hatch is a row
/// that must NAME the bound and state the reason, so silencing one costs a
/// sentence a reviewer can disagree with, rather than a deleted line nobody
/// sees again.
///
/// It is NOT a free list: the guard also reds on a row here whose bound no loop
/// walks, so a retired bound cannot linger as a permanent exemption.
///
/// **AND SINCE `W-305` THE REASON IS NOT ONLY PROSE.** The half of it that does
/// the work — "a POOL of identical slots, not N kinds" — is derivable from the
/// loop that cuts the pool, and
/// `an_untabled_exemption_writes_the_same_thing_into_every_slot` derives it: the
/// body must write THE SAME LITERAL into every slot. A row whose loop writes a
/// NAME is an arena of distinguishable entries wearing an exemption, and reds.
/// A new row must earn the same thing; the sentence a reviewer disagrees with is
/// now backed by a sentence a machine does.
struct Untabled {
    /// The `सार्वजनिक चरः` in `ir.t1`.
    name: &'static str,
    /// The ASCII key this bound's numbers carry in `.loop/METRICS.tsv` —
    /// written down, not transliterated, for the reason `Bound::metric` gives.
    metric: &'static str,
    /// Why a name table would say nothing here. Printed in the refusal when the
    /// row outlives its bound.
    why: &'static str,
    /// The names by which a PRODUCER could reach one slot of this arena: the
    /// arena itself, its accessors, and the pool's own bound and base. A
    /// census that prints a line per slot must name one of them, so this list
    /// is what `no_producer_prints_a_line_per_slot_of_an_untabled_arena`
    /// searches the producer roots for.
    ///
    /// **A NAME THAT MEANS TWO THINGS IN TWO FILES IS NOT A KEY.**
    /// `स्थानीयसंख्यान` is the local COUNT that indexes this arena in `ir.t1`
    /// and the `Frame::num_locals` FIELD in `yantrotsarjana.t1` — two subjects
    /// under one spelling, so a hit on it would say nothing about which one
    /// was printed. It is left out, and the count is not a slot anyway.
    slots: &'static [&'static str],
    /// The routine that answers ONE slot of this pool by index: `वृत्तिः
    /// <accessor> आदाय <index>`. It is the only way a slot is reached, so its
    /// CALL SITES are the whole population of indices into the pool — which is
    /// what `every_index_into_an_untabled_pool_is_inside_it` bounds-checks.
    accessor: &'static str,
    /// Phrases that mark a margin as RECORDING this pool's slot layout.
    ///
    /// The exemption's third sentence says the layout margin "is a frame
    /// layout, not an arena census". A frame layout names indices, and an index
    /// it names must be one the pool HAS — so the margins are found by their
    /// own words rather than by a line number, which moves on every edit, and
    /// every numeral on a marked line is bounds-checked.
    layout_markers: &'static [&'static str],
}

const UNTABLED: &[Untabled] = &[Untabled {
    name: "अनामस्थलसंख्या",
    metric: "anamasthalasankhya",
    why: "a per-routine POOL of eight scratch slots, not eight kinds. \
          `अनामस्थानम्` cuts all eight at once and writes ० into each \
          (ir.t1:1186), and NO census anywhere prints a line per slot — so the \
          defect a name table exists to prevent, an unnamed population that \
          reads exactly like a zero, has no way to happen here. The margin at \
          ir.t1:1176 records which helper uses which index; that is a frame \
          layout, not an arena census",
    slots: &[
        "स्थानीयचिह्नककोश",
        "स्थानीयस्थानम्",
        "स्थानीयघोषणम्",
        "अनामस्थानम्",
        "अनामस्थलसंख्या",
        "अनामस्थलारम्भः",
    ],
    accessor: "अनामस्थानम्",
    layout_markers: &["fixed indices", "free scratch"],
}];

/// One `ir.t1` loop that walks an arena bound: where it is, and which convention
/// the walk ITSELF uses.
///
/// The convention is read here, in the ONE reader of this grammar, rather than
/// by a second string search in `ir_bound_convention` — see that function for
/// the disagreement the second reader could produce.
struct Walk {
    /// The 1-based line of `ir.t1` that walks the bound.
    line: usize,
    /// `None` when the words after the bound are NEITHER form — an offset this
    /// file has no name for, refused rather than guessed at.
    convention: Option<Convention>,
    /// The words after the bound, so a refusal quotes what it read instead of
    /// asserting what it did not find.
    tail: String,
}

/// Every ARENA BOUND an `ir.t1` loop walks, mapped to the walks themselves, and
/// the number of walks the scan read to find them.
///
/// **AN ARENA BOUND IS DERIVED, NOT SPELLED.** Two properties, and both are
/// load-bearing because `ir.t1` has a counter-example to each within twenty
/// lines of the other:
///
///   1. **A module-level constant** — `सार्वजनिक चरः <name> ॱॱ न६४ भवति <numeral> ।`.
///      `ir.t1:3478` walks `यावत् सीमा न्यूनम् आरभ्य क्रमः योगः १ समाप्तम्`, where the
///      word `सीमा` is the LOOP VARIABLE and the bound is a parameter. A scan
///      that recognised a bound by its name ending in `सीमा` would report that
///      line's `सीमा` as a fourth arena and demand a table for a local counter.
///   2. **Never assigned.** `बाह्यसंज्ञासूचकाङ्क` and `स्थानीयसंख्यान` are declared
///      `भवति ०` at the top and counted up at `ir.t1:1128` and `:1161`; loops
///      at `:1117` and `:1141` walk them. They are LIVE COUNTS, and the number
///      a loop stops at is whatever the pass just built — there is nothing for
///      a hand-written table to equal. An arena bound is a literal that sizes
///      storage, and `ir.t1` never writes to one.
///
/// Read off the walks and not off a list, because the whole point is to notice
/// the bound NOBODY wrote down: `W-264` closed `अपूर्णस्थानसीमा` by hand and
/// recorded `METRIC w264_bounds_unguarded` — a human had read `ir.t1` once and
/// typed what they saw, and the fourth bound of this kind would have been just
/// as silent.
///
/// **AND THE WALK'S CONVENTION IS READ HERE TOO**, since `W-303`: the tail after
/// the bound decides it, and a tail that is neither form leaves `convention`
/// `None` rather than defaulting to the one whose keyword happened to appear.
fn ir_walked_arena_bounds() -> (BTreeMap<String, Vec<Walk>>, usize) {
    walked_arena_bounds(&ir_t1_text())
}

/// `crates/sadhana-t1/src/ir.t1`, read once per caller.
fn ir_t1_text() -> String {
    let path = repo_root().join("crates/sadhana-t1/src/ir.t1");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
}

/// The reader itself, over TEXT rather than over the checkout, so a refusal can
/// be demonstrated on a three-line program instead of by editing `ir.t1` — the
/// mutation that proves this reader works must not be a mutation of the tree it
/// is the only reader of.
fn walked_arena_bounds(text: &str) -> (BTreeMap<String, Vec<Walk>>, usize) {
    let mut constant: BTreeSet<&str> = BTreeSet::new();
    let mut assigned: BTreeSet<&str> = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("सार्वजनिक चरः ") {
            // `<name> ॱॱ न६४ भवति <numeral> ।` — a numeral, so `भवति <expr>`
            // and `भवति <other name>` are not constants.
            let t: Vec<&str> = rest.split(' ').filter(|t| !t.is_empty()).collect();
            if t.len() >= 5
                && t[1] == "ॱॱ"
                && t[3] == "भवति"
                && devanagari_number(t[4].trim_end_matches('।').trim()).is_some()
            {
                constant.insert(t[0]);
            }
            continue;
        }
        // `<name> भवति <...>` — the whole variable is written. A slot write,
        // `<कोश> अङ्कः <i> अन्तः भवति <...>`, is NOT: it assigns into the arena
        // rather than to the bound, and its second word is `अङ्कः`.
        let t: Vec<&str> = line.split(' ').filter(|t| !t.is_empty()).collect();
        if t.len() >= 2 && t[1] == "भवति" {
            assigned.insert(t[0]);
        }
    }

    let mut walked: BTreeMap<String, Vec<Walk>> = BTreeMap::new();
    let mut scanned = 0usize;
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        // Only a line that IS the walk: a margin quoting a loop is prose, and
        // prose about these bounds has been wrong twice.
        if !line.starts_with("यावत् ") {
            continue;
        }
        let t: Vec<&str> = line.split(' ').filter(|t| !t.is_empty()).collect();
        let Some(i) = t.iter().position(|w| *w == "न्यूनम्") else {
            continue;
        };
        scanned += 1;
        // `... न्यूनम् <bound> आदि` and `... न्यूनम् आरभ्य <bound> योगः १ समाप्तम् आदि` —
        // the two conventions `Convention` names. The bound sits in a different
        // place in each, and so does the tail that confirms which one this is.
        let restated = t.get(i + 1) == Some(&"आरभ्य");
        let k = if restated { i + 2 } else { i + 1 };
        let Some(bound) = t.get(k) else { continue };
        if !constant.contains(bound) || assigned.contains(bound) {
            continue;
        }
        let tail: Vec<&str> = t[k + 1..].to_vec();
        // `आदि` opens the loop body, so it is where the comparison ends. The
        // tail is matched WHOLE: `योगः २ समाप्तम्` is a third offset, not a
        // sloppy spelling of the first, and it is left unreadable on purpose.
        let convention = match (restated, tail.as_slice()) {
            (true, ["योगः", "१", "समाप्तम्", "आदि"]) => {
                Some(Convention::Inclusive)
            }
            (false, ["आदि"]) => Some(Convention::Exclusive),
            _ => None,
        };
        walked.entry((*bound).to_string()).or_default().push(Walk {
            line: n + 1,
            convention,
            tail: tail.join(" "),
        });
    }
    (walked, scanned)
}

/// ॥ EVERY ARENA BOUND `ir.t1` WALKS IS TABLED OR DECLARED UNTABLED ॥ `W-302`.
///
/// `each_arena_bound_equals_the_highest_entry_of_the_table_it_bounds` checks the
/// bounds SOMEBODY LISTED. This one checks the list.
///
/// **THE ROW IT REPLACES WAS A HUMAN'S SINGLE READING OF `ir.t1`.** `W-264`
/// printed `METRIC w264_bounds_unguarded अपूर्णस्थानसीमा` because an operator had
/// opened the file, seen one bound with no table, and typed its name; when that
/// bound was guarded the row was dropped as false and NOTHING was left watching.
/// A fourth bound of the same kind would have been exactly as silent as the
/// third was — and the third was silent from `64a93973` until someone happened
/// to look.
///
/// **THE SCAN FOUND A FOURTH ON THE DAY IT WAS WRITTEN.** `अनामस्थलसंख्या` = ८ is
/// walked at `ir.t1:1186` and appears in no `BOUNDS` row, which is not a defect:
/// it is a pool of identical slots, so it is declared in `UNTABLED` with its
/// reason rather than guarded with a table of eight names for one thing.
///
/// **THE CASE THAT MUST STILL BE REFUSED IS THE EXEMPTION ITSELF.** An escape
/// hatch that only ever lets things through is a deleted check with extra steps,
/// so it reds three ways: a walked bound in NEITHER list, a bound in BOTH, and a
/// row in EITHER list whose bound no loop walks — the last so a retired bound
/// cannot leave a permanent exemption behind, and so a renamed one is loud
/// rather than quietly unguarded.
///
/// And the scan asserts it read something. A rename of `यावत्` or `न्यूनम्` would
/// otherwise empty both sides and this test would pass having compared nothing,
/// which is the fourth move this file's header says the other three are
/// worthless without.
#[test]
fn every_arena_bound_ir_t1_walks_is_tabled_or_declared_untabled() {
    let (walked, scanned) = ir_walked_arena_bounds();
    assert!(
        scanned > 0,
        "no line of ir.t1 both starts `यावत् ` and contains `न्यूनम्`, so this scan \
         read NO walks at all — the loop keyword or the comparison was renamed \
         and this census would pass having compared nothing. Teach \
         `ir_walked_arena_bounds` the new spelling."
    );
    assert!(
        walked.len() >= BOUNDS.len(),
        "ir.t1 walks {} arena bound(s) but BOUNDS lists {} — the scan is reading \
         FEWER bounds than are already known to exist, so its derivation is \
         broken rather than the tree. Found: {:?}",
        walked.len(),
        BOUNDS.len(),
        walked.keys().collect::<Vec<_>>()
    );
    println!("METRIC w302_ir_walks_scanned {scanned}");
    println!("METRIC w302_arena_bounds_walked {}", walked.len());

    let mut wrong: Vec<String> = Vec::new();
    let (mut tabled, mut untabled) = (0usize, 0usize);
    for (name, walks) in &walked {
        let at: Vec<String> = walks.iter().map(|w| format!("ir.t1:{}", w.line)).collect();
        let at = at.join(", ");
        let in_bounds = BOUNDS.iter().find(|b| b.name == name.as_str());
        let in_untabled = UNTABLED.iter().find(|u| u.name == name.as_str());
        match (in_bounds, in_untabled) {
            (Some(b), None) => {
                tabled += 1;
                println!("METRIC w302_walks_{} {}", b.metric, walks.len());
            }
            (None, Some(u)) => {
                untabled += 1;
                println!("METRIC w302_walks_{} {}", u.metric, walks.len());
            }
            (Some(b), Some(_)) => wrong.push(format!(
                "`{name}` is in BOTH lists — `BOUNDS` pairs it with {} and \
                 `UNTABLED` says it has no table. One of those is wrong and the \
                 pair cannot say which, so neither is trusted: delete the \
                 `UNTABLED` row if {} is real, or the `BOUNDS` row if it is not.",
                b.table, b.table
            )),
            (None, None) => wrong.push(format!(
                "`{name}` is an arena bound ir.t1 WALKS ({at}) and NOTHING guards \
                 it — it is in neither `BOUNDS` nor `UNTABLED`. A bound with no \
                 table is silent in the direction that matters: raise it past its \
                 census and the extra numbers are cleared, counted, and excluded \
                 from both the print and the total, so `no line for N` and `N is \
                 zero` are the same output (2026-09-07, `अपूर्णहेतुसीमा`). Add a \
                 `BOUNDS` row naming its `&[(i128, &str)]` table, or — if its \
                 entries are not distinct named kinds — an `UNTABLED` row saying \
                 so and why."
            )),
        }
    }
    println!("METRIC w302_arena_bounds_tabled {tabled}");
    println!("METRIC w302_arena_bounds_untabled {untabled}");

    for Bound { name, table, .. } in BOUNDS {
        if !walked.contains_key(*name) {
            wrong.push(format!(
                "`BOUNDS` pins `{name}` against {table}, but NO ir.t1 loop walks \
                 it as a constant arena bound. Either it was renamed — and the \
                 arena it really sizes is now unguarded under the new name — or \
                 ir.t1 now ASSIGNS it, which makes it a live count that no \
                 hand-written table can equal. Fix the name, or move the row."
            ));
        }
    }
    for Untabled { name, why, .. } in UNTABLED {
        if !walked.contains_key(*name) {
            wrong.push(format!(
                "`UNTABLED` exempts `{name}` — {why} — but NO ir.t1 loop walks it \
                 as a constant arena bound. An exemption that outlives its bound \
                 is a permanent hole: it would go on excusing whatever later \
                 takes that name. Drop the row, or fix it to the name ir.t1 now \
                 uses."
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "the list of guarded arena bounds does not match the ones ir.t1 walks:\n  {}",
        wrong.join("\n  ")
    );
}

// ── an UNTABLED exemption is EARNED by what its loop writes ────────────────

/// What the body of a walk puts into the arena it walks.
///
/// **FOUR STATES AND NOT A BOOLEAN.** "the writes vary", "the body writes into
/// no arena at all" and "the body never closes" are three different defects
/// with three different fixes, and a `bool` would have to report two of them
/// under the third one's name — which is how `ir_bound_convention`'s string
/// reader used to call an unreadable walk an absent one (`W-303`).
#[derive(Debug, PartialEq, Eq)]
enum PoolWrite {
    /// Every slot write in the body writes THE SAME LITERAL. Nothing in the
    /// loop can tell one slot from another, which is exactly what "a pool of
    /// identical slots, not N kinds" asserts.
    Uniform(String),
    /// The body writes into the arena, but not one literal into every slot.
    /// Carries `(line, value expression)` per write so a refusal QUOTES what it
    /// read rather than asserting what it did not find.
    Varying(Vec<(usize, String)>),
    /// The body writes into no arena at all.
    NoWrite,
    /// The walk's `आदि` has no matching `इति`.
    Unterminated,
}

/// A T1 LITERAL, which cannot vary by index; a name can.
///
/// Numerals are what this tree's one pool cut uses. `शून्यम्` is the grammar's
/// other literal — `ir.t1:1237` records that it "is the nil literal: neither
/// can name a value" — and it is admitted here because a nil-filled pool is
/// the same claim as a zero-filled one. Anything else is a NAME, and a name in
/// the value of a slot write is the thing this check exists to notice.
fn is_t1_literal(token: &str) -> bool {
    token == "शून्यम्" || devanagari_number(token).is_some()
}

/// The value expression of a slot write, `<कोश> अङ्कः <i> अन्तः भवति <value> ।`,
/// or `None` when the line is not one.
///
/// **THE INDEX ENDS AT THE FIRST `अन्तः` THAT `भवति` FOLLOWS.** The `भवति` half
/// of that is a guard against a nested index — `<कोश> अङ्कः <कोश> अङ्कः <i> अन्तः
/// अन्तः भवति <value>` — where the FIRST `अन्तः` closes the inner read and
/// splitting there would take the rest of the index for the value.
///
/// **IT IS NOT A GUARD AGAINST A READ-MODIFY-WRITE, AND MEASURING SAID SO.**
/// `ir.t1:856` is `रचितगणनाकोश अङ्कः रूपम् अन्तः भवति रचितगणनाकोश अङ्कः रूपम् अन्तः
/// योगः १ ।`, which carries a second `अन्तः` — but on the VALUE side, after the
/// split point, so a first-match search reaches the right one with or without
/// the guard. Dropping `&& t[i + 1] == "भवति"` left every test in this file
/// GREEN (2026-09-18), so the guard is held by a synthetic nested index in
/// `the_body_reader_stops_at_the_right_iti_and_reads_the_right_value` and by
/// nothing in the tree. `ir.t1` has no nested index today; this reads one
/// correctly if it gains one, rather than silently reporting half of it.
fn slot_write_value(line: &str) -> Option<String> {
    let code = line.split('॰').next().unwrap_or(line).trim();
    let t: Vec<&str> = code.split(' ').filter(|w| !w.is_empty()).collect();
    if t.len() < 6 || t[1] != "अङ्कः" {
        return None;
    }
    let i = (2..t.len() - 1).find(|&i| t[i] == "अन्तः" && t[i + 1] == "भवति")?;
    let value: Vec<&str> = t[i + 2..].iter().copied().filter(|w| *w != "।").collect();
    let value = value.join(" ");
    let value = value.trim().trim_end_matches('।').trim().to_string();
    (!value.is_empty()).then_some(value)
}

/// What the loop opened at `line` (1-based, into `text`) writes into the arenas
/// its body touches.
///
/// **THE BLOCK IS DELIMITED BY DEPTH, AND THAT PART IS LOAD-BEARING.** A reader
/// that ended the body at the first `इति` reported the branched body below as
/// `Uniform("०")` where the truth is `Varying` (measured 2026-09-18) — it read
/// the `यदि` arm, stopped at that arm's own `इति`, and never saw the `अन्यथा`
/// arm write something else. That is the check passing by looking at HALF its
/// subject, which is the failure this whole file is about.
///
/// **THE TOKENS ARE COUNTED IN ORDER, AND THAT PART IS NOT — YET.** `इति अन्यथा
/// आदि` (`ir.t1:1942`, `:2177`, `:2181`) closes and reopens on ONE line, and a
/// per-line net would differ only if such a line could take the depth to zero
/// and back; inside a `यावत्` walk it always sits at depth two or more, so a
/// netting reader measured IDENTICAL on every shape `ir.t1` has (2026-09-18).
/// In-order is kept because it is the reading that stays right if a walk ever
/// gains an else, not because it is guarding anything today.
///
/// Margins are stripped before counting: prose quoting a loop is not a loop,
/// and prose about these bounds has been wrong twice.
fn pool_write(text: &str, line: usize) -> PoolWrite {
    let mut depth = 0i32;
    let mut writes: Vec<(usize, String)> = Vec::new();
    let mut closed = false;
    for (n, raw) in text.lines().enumerate().skip(line - 1) {
        let code = raw.split('॰').next().unwrap_or(raw);
        if n + 1 > line
            && let Some(v) = slot_write_value(code)
        {
            writes.push((n + 1, v));
        }
        for w in code.split(' ').filter(|w| !w.is_empty()) {
            match w {
                "आदि" => depth += 1,
                "इति" => depth -= 1,
                _ => continue,
            }
            if depth == 0 {
                closed = true;
                break;
            }
        }
        if closed {
            break;
        }
    }
    if !closed {
        return PoolWrite::Unterminated;
    }
    if writes.is_empty() {
        return PoolWrite::NoWrite;
    }
    let first = writes[0].1.clone();
    let uniform = writes
        .iter()
        .all(|(_, v)| *v == first && v.split(' ').all(is_t1_literal));
    if uniform {
        PoolWrite::Uniform(first)
    } else {
        PoolWrite::Varying(writes)
    }
}

/// ॥ AN `UNTABLED` EXEMPTION IS EARNED BY WHAT ITS LOOP WRITES ॥ `W-305`.
///
/// `every_arena_bound_ir_t1_walks_is_tabled_or_declared_untabled` checks that
/// every walked bound is in one list or the other, and that a row's bound is
/// still walked. It does NOT check that an `UNTABLED` row's REASON is true —
/// and the reason is the entire difference between a legitimate exemption and
/// a silenced check.
///
/// **THE PART OF THE REASON THAT DOES THE WORK WAS PROSE NOTHING READ.**
/// `अनामस्थलसंख्या`'s row says the arena is "a per-routine POOL of eight scratch
/// slots, not eight kinds", and the evidence it offers is that `अनामस्थानम्`
/// "cuts all eight at once and writes ० into each". Until this test the only
/// part of that a machine checked was that SOME loop walks the name — which is
/// equally true of an arena of eight distinct kinds wearing an exemption.
///
/// **THE CLAIM IS DERIVABLE, so it is derived.** A pool is identical slots; the
/// loop that cuts it therefore writes the SAME LITERAL into every one. A body
/// whose written value is a NAME writes something that can differ per slot, and
/// those slots are distinguishable — an arena of kinds, owed a table.
///
/// **THE CASE THAT MUST STILL BE REFUSED IS THE EXEMPTION WHOSE BODY VARIES BY
/// INDEX**, and it is refused in `an_exemption_whose_body_varies_by_index_is_refused`
/// on synthetic text, so the falsifier does not require editing the tree this
/// file is the only reader of. Two more refusals sit beside it, because a body
/// reader fails three ways and a two-state instrument would report two of them
/// as the third: a body with no write at all, and a body that never closes.
#[test]
fn an_untabled_exemption_writes_the_same_thing_into_every_slot() {
    let text = ir_t1_text();
    let (walked, _) = walked_arena_bounds(&text);
    let mut wrong: Vec<String> = Vec::new();
    let (mut checked, mut uniform) = (0usize, 0usize);

    for Untabled {
        name, metric, why, ..
    } in UNTABLED
    {
        // A row whose bound no loop walks is ALREADY the census test's finding.
        // Restating it here would red two tests for one defect and make the
        // second look like corroboration.
        let Some(walks) = walked.get(*name) else {
            continue;
        };
        for w in walks {
            checked += 1;
            match pool_write(&text, w.line) {
                PoolWrite::Uniform(v) => {
                    uniform += 1;
                    println!("METRIC w305_pool_{metric}_slots {}", ir_bound(name));
                    assert!(!v.is_empty(), "a uniform write has a value");
                }
                PoolWrite::Varying(writes) => wrong.push(format!(
                    "`{name}` is exempted from a name table because {why} — but \
                     the loop at ir.t1:{} does NOT write one literal into every \
                     slot: {}. A value that is a NAME can differ per slot, and \
                     slots that differ are distinct KINDS, which is precisely \
                     what a name table guards. Either the exemption is wrong and \
                     this bound needs a `BOUNDS` row, or the loop changed and the \
                     reason must be rewritten to say what is true now.",
                    w.line,
                    writes
                        .iter()
                        .map(|(n, v)| format!("ir.t1:{n} writes `{v}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                PoolWrite::NoWrite => wrong.push(format!(
                    "`{name}` is exempted because {why} — but the loop at \
                     ir.t1:{} writes into NO arena at all. The reason's evidence \
                     has no subject: whatever that loop does now, it is not the \
                     cut this exemption is built on, and nothing here can say \
                     whether the slots are alike.",
                    w.line
                )),
                PoolWrite::Unterminated => wrong.push(format!(
                    "the loop at ir.t1:{} is never closed — its `आदि` has no \
                     matching `इति` within the rest of the file. This is the \
                     READER failing, not the exemption: `{name}`'s body could \
                     not be delimited, so no claim about what it writes was \
                     checked. Teach `pool_write` the block spelling ir.t1 now \
                     uses.",
                    w.line
                )),
            }
        }
    }

    assert!(
        checked > 0,
        "no `UNTABLED` row's bound is walked, so this test compared NOTHING and \
         would pass on an empty list. `UNTABLED` has {} row(s); either they are \
         all stale — which the census test reds on — or `walked_arena_bounds` \
         stopped finding walks.",
        UNTABLED.len()
    );
    println!("METRIC w305_untabled_walks_checked {checked}");
    println!("METRIC w305_untabled_walks_uniform {uniform}");
    assert!(
        wrong.is_empty(),
        "an `UNTABLED` exemption claims a pool of identical slots and its loop \
         says otherwise:\n  {}",
        wrong.join("\n  ")
    );
}

/// A minimal `.t1` with ONE arena bound, one walk of it, and `body` inside.
///
/// The walk is at line 3 of every program this builds, which the refusals below
/// depend on — so they find it through `walked_arena_bounds` rather than
/// assuming it, and assert the line they got.
fn pool_program(body: &str) -> String {
    format!(
        "सार्वजनिक चरः पूगसंख्या ॱॱ न६४ भवति ८ ।\n\
         वृत्तिः पूगच्छेदः ददाति न६४ आदि\n\
         \x20   यावत् गणकः न्यूनम् पूगसंख्या आदि\n\
         {body}\n\
         \x20   इति\n\
         इति\n"
    )
}

/// The named refusal, plus the two other ways a body reader fails.
#[test]
fn an_exemption_whose_body_varies_by_index_is_refused() {
    let uniform = pool_program(
        "        चिह्नककोश अङ्कः अनुक्रमः अन्तः भवति ० ।\n\
         \x20       अनुक्रमः भवति अनुक्रमः योगः १ ।",
    );
    let (walked, _) = walked_arena_bounds(&uniform);
    let walks = walked
        .get("पूगसंख्या")
        .expect("the synthetic program walks its own bound");
    assert_eq!(walks.len(), 1, "one walk");
    let line = walks[0].line;
    assert_eq!(line, 3, "the walk is on line 3 of pool_program");
    assert_eq!(
        pool_write(&uniform, line),
        PoolWrite::Uniform("०".to_string()),
        "eight slots cut to the same literal is the exemption's own claim"
    );

    // THE NAMED REFUSAL: one edit, the value, from a literal to the index.
    let varying = pool_program(
        "        चिह्नककोश अङ्कः अनुक्रमः अन्तः भवति अनुक्रमः ।\n\
         \x20       अनुक्रमः भवति अनुक्रमः योगः १ ।",
    );
    assert_eq!(
        pool_write(&varying, 3),
        PoolWrite::Varying(vec![(4, "अनुक्रमः".to_string())]),
        "a value that is a NAME differs per slot — those are kinds, not a pool"
    );

    // Two literals that DISAGREE is the same defect without a name in sight.
    let split = pool_program(
        "        चिह्नककोश अङ्कः अनुक्रमः अन्तः भवति ० ।\n\
         \x20       चिह्नककोश अङ्कः अपरः अन्तः भवति १ ।",
    );
    assert_eq!(
        pool_write(&split, 3),
        PoolWrite::Varying(vec![(4, "०".to_string()), (5, "१".to_string())]),
        "two literals is two kinds of slot, however literal each one is"
    );

    // A body with no slot write is NOT `Varying` — the exemption's evidence has
    // no subject, which is a different sentence to a reviewer.
    let silent = pool_program("        गणकः भवति गणकः योगः १ ।");
    assert_eq!(pool_write(&silent, 3), PoolWrite::NoWrite);

    // And an unclosed block is the READER failing, reported as itself.
    let open = "सार्वजनिक चरः पूगसंख्या ॱॱ न६४ भवति ८ ।\n\
                यावत् गणकः न्यूनम् पूगसंख्या आदि\n\
                    चिह्नककोश अङ्कः अनुक्रमः अन्तः भवति ० ।\n";
    assert_eq!(pool_write(open, 2), PoolWrite::Unterminated);
}

/// The delimiter and the value reader, on the two forms `ir.t1` actually has
/// that a naive reading gets wrong.
#[test]
fn the_body_reader_stops_at_the_right_iti_and_reads_the_right_value() {
    // `इति अन्यथा आदि` closes and reopens on ONE line. A closes-first reading
    // ends the body here and never sees the second write; a per-line net leaves
    // the depth right by luck. Counting in order is what makes both writes
    // visible — and this body is Varying BECAUSE the second one is seen.
    let branched = pool_program(
        "        यदि गणकः समम् ० आदि\n\
         \x20           चिह्नककोश अङ्कः अनुक्रमः अन्तः भवति ० ।\n\
         \x20       इति अन्यथा आदि\n\
         \x20           चिह्नककोश अङ्कः अनुक्रमः अन्तः भवति १ ।\n\
         \x20       इति",
    );
    assert_eq!(
        pool_write(&branched, 3),
        PoolWrite::Varying(vec![(5, "०".to_string()), (7, "१".to_string())]),
        "the `अन्यथा` arm is inside the loop and both of its writes are read"
    );

    // `ir.t1:856`'s shape: the value carries a SECOND `अन्तः`. Splitting at the
    // first one reads the value as `योगः १` — two literals that happen to be
    // equal to each other, and an accumulator called a pool.
    assert_eq!(
        slot_write_value("    रचितगणनाकोश अङ्कः रूपम् अन्तः भवति रचितगणनाकोश अङ्कः रूपम् अन्तः योगः १ ।"),
        Some("रचितगणनाकोश अङ्कः रूपम् अन्तः योगः १".to_string())
    );
    assert_eq!(
        slot_write_value("    स्थानीयचिह्नककोश अङ्कः स्थानीयसंख्यान अन्तः भवति ० ।"),
        Some("०".to_string())
    );
    // A NESTED INDEX is what the `भवति` half of the split test is for, and it
    // is the only thing holding it: `ir.t1` has no such write today, and
    // dropping the guard reds nothing else. Splitting at the first `अन्तः`
    // yields `अङ्कः क अन्तः भवति ०` — a value that is mostly index.
    assert_eq!(
        slot_write_value("    कोशः अङ्कः अन्यकोशः अङ्कः क अन्तः अन्तः भवति ० ।"),
        Some("०".to_string())
    );
    // A margin is not a write, and neither is a plain assignment.
    assert_eq!(slot_write_value("॰ कोशः अङ्कः क अन्तः भवति ० ।"), None);
    assert_eq!(slot_write_value("    गणकः भवति गणकः योगः १ ।"), None);
    assert!(is_t1_literal("०") && is_t1_literal("शून्यम्"));
    assert!(!is_t1_literal("अनुक्रमः"));
}

// ── an UNTABLED exemption is SAFE only while no census prints its slots ────

/// The PRODUCER ROOTS: `crates/*/src` and `tools/`, which is where the ledger
/// scopes the claim and where everything that writes a census line lives.
///
/// `crates/*/tests` is deliberately outside it. A test is not a producer of
/// dashboard output, and including it would make this file its own subject —
/// the `UNTABLED` row's slot names are written down a few hundred lines above.
fn producer_roots() -> Vec<PathBuf> {
    let root = repo_root();
    let mut roots = vec![root.join("tools")];
    let mut srcs: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))
        .expect("crates/ must be readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("src"))
        .filter(|p| p.is_dir())
        .collect();
    srcs.sort();
    roots.extend(srcs);
    roots
}

/// Every file under a producer root, plus the number that could NOT be read as
/// text.
///
/// The unreadable count is returned rather than swallowed: a producer hidden
/// inside a blob this scan cannot decode is a hole in the claim, and a scan
/// that silently skips what it cannot read reports the same number as a scan
/// that read everything. It stands at ONE, and that one is
/// `tools/fonts/NotoSansDevanagari-Regular.ttf` — a TrueType font, checked by
/// decoding every file under the roots strictly (2026-09-18), not inferred
/// from the count.
fn producer_files() -> (Vec<(PathBuf, String)>, usize) {
    let mut out: Vec<(PathBuf, String)> = Vec::new();
    let mut unreadable = 0usize;
    let mut stack = producer_roots();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.filter_map(|e| e.ok()) {
            let p = e.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Build output and caches are not sources; a dotfile is not a
            // producer this tree writes census lines from.
            if name == "__pycache__" || name == "target" || name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            match std::fs::read_to_string(&p) {
                Ok(t) => out.push((p, t)),
                Err(_) => unreadable += 1,
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    (out, unreadable)
}

/// How Rust, Python and shell write a line of output.
///
/// **THESE ARE SUBSTRINGS OF A LINE, NOT PARSED CALLS**, and that
/// over-detects: a comment containing the word `echo ` counts as an emission
/// site. The over-detection is the safe direction for three of the four states
/// — it can only move a file toward "this file emits", never toward "this file
/// is silent". The one place it can over-REFUSE is a comment that names the
/// arena on the same line as one of these words, and a comment claiming a
/// per-slot census is worth the human look that costs.
const EMITTERS: &[&str] = &[
    "println!",
    "eprintln!",
    "print!",
    "eprint!",
    "writeln!",
    "write!",
    "wr!(",
    "print(",
    "echo ",
    "printf ",
];

/// How `.t1` writes a line: a CALL to a routine whose name ends in `मुद्रणम्`
/// — `मुद्रणम्` itself (`ashtaka.t1:204`), `अष्टकॱमुद्रणम्`,
/// `वस्तुपाठ्यमुद्रणम्`, `विन्यासमुद्रणम्`.
///
/// **A SUBSTRING WOULD NOT DO, AND MEASURING SAID SO.** This was written as
/// the bare stem `मुद्रण` alongside the list above, and `ir.t1` then read as a
/// file that emits: `ir.t1:2170`, `:2233` and `:2417` carry `मुद्रणमिदम्`, a
/// BOOLEAN FLAG meaning "the call being compiled is a print". Three stores to
/// a `बूल` were reported as three emissions (2026-09-18), which would have put
/// the one file that declares this arena into the state reserved for files
/// that print. Matched on the WHOLE TOKEN's suffix, because `मुद्रणमिदम्` does
/// not end in `मुद्रणम्` — and split on SPACES, because a word boundary does
/// not exist in this script.
const T1_EMITTER_SUFFIX: &str = "मुद्रणम्";

/// The part of a `.t1` line that is CODE: margin stripped, then quoted names
/// stripped.
///
/// **A QUOTED NAME IS NOT A CALL, AND THE TREE HOLDS THAT.** `ir.t1:2232` is
/// `समम् उक्तम् अष्टकॱमुद्रणम् इति` — it COMPARES a token's text to the
/// printer's name while compiling a call to it. Drop this strip and `ir.t1`,
/// the one file under the producer roots that declares this arena, moves from
/// `NamesButNeverEmits` to `EmitsElsewhere` (measured 2026-09-18). That is not
/// the refusal, but it is the wrong sentence about the wrong file, and it is
/// the state the exemption's safety is read out of.
///
/// **THE QUOTE END IS THE FIRST `इति`, AND THAT IS NOT ALWAYS THE RIGHT ONE.**
/// `ir.t1:459` records that in `उक्तम् इति इति इति` the value is the single
/// word `इति`; a first-match search ends the quote one word early and leaves
/// `इति इति` behind as code. That direction is safe here — leftover code can
/// only ADD an emission site, never hide one — and the shapes this reader must
/// get right, a quoted routine name, have no `इति` inside them.
fn t1_code(line: &str) -> String {
    let code = line.split('॰').next().unwrap_or(line);
    let mut out = String::new();
    let mut rest = code;
    while let Some(i) = rest.find("उक्तम्") {
        out.push_str(&rest[..i]);
        let after = &rest[i + "उक्तम्".len()..];
        match after.find("इति") {
            Some(j) => rest = &after[j + "इति".len()..],
            // An unterminated quote swallows the rest of the line, which is
            // what the grammar does with it.
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The scannable code of one line of one file. Only `.t1` is stripped: a `//`
/// or a `#` inside a string literal would truncate a line that names the arena
/// and print it, which is the one defect this scan exists to catch.
fn scan_code(path: &Path, line: &str) -> String {
    if path.extension().is_some_and(|e| e == "t1") {
        t1_code(line)
    } else {
        line.to_string()
    }
}

/// What one producer file does with an arena's slot names.
///
/// **FOUR STATES AND NOT A BOOLEAN**, which is `W-303`'s lesson and `W-305`'s:
/// "this file cannot name a slot", "it names one and prints nothing at all",
/// "it names one and prints, but not on the same line" and "it prints one" are
/// four different sentences to a reviewer, and a `bool` would have to report
/// the middle two under one of the outer ones' names. The middle two are the
/// interesting ones: they are where the tree actually sits, and collapsing
/// them is how a scan that reaches nothing reads exactly like a scan that
/// reached everything and found it clean.
#[derive(Debug, PartialEq, Eq)]
enum SlotCensus {
    /// The file never names the arena or any accessor of it, so nothing it
    /// prints can be keyed by a slot. This is the state of almost every file.
    Silent,
    /// It names the arena and emits NOTHING — no print, no write, no echo.
    /// Carries the lines that name it.
    NamesButNeverEmits(Vec<usize>),
    /// It names the arena AND emits, but no emission LINE names the arena.
    /// This is one variable away from the defect, and it is the state that
    /// proves the scan reaches a real producer rather than stopping at a
    /// directory listing.
    EmitsElsewhere { named: Vec<usize>, emissions: usize },
    /// An emission line names the arena. THE REFUSAL. Carries `(line, text)`
    /// so the message QUOTES what it read instead of asserting what it did not
    /// find.
    EmitsKeyed(Vec<(usize, String)>),
}

/// Classify one file against one arena's slot names.
fn slot_census(path: &Path, text: &str, keys: &[&str]) -> SlotCensus {
    let mut named: Vec<usize> = Vec::new();
    let mut emissions = 0usize;
    let mut keyed: Vec<(usize, String)> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let code = scan_code(path, raw);
        let names = keys.iter().any(|k| code.contains(k));
        let emits = line_emits(path, &code);
        if names {
            named.push(n + 1);
        }
        if emits {
            emissions += 1;
        }
        if names && emits {
            keyed.push((n + 1, raw.trim().to_string()));
        }
    }
    if !keyed.is_empty() {
        SlotCensus::EmitsKeyed(keyed)
    } else if named.is_empty() {
        SlotCensus::Silent
    } else if emissions == 0 {
        SlotCensus::NamesButNeverEmits(named)
    } else {
        SlotCensus::EmitsElsewhere { named, emissions }
    }
}

/// Does this line of CODE write output?
fn line_emits(path: &Path, code: &str) -> bool {
    if path.extension().is_some_and(|e| e == "t1") {
        code.split(' ')
            .any(|w| w.trim_end_matches('\u{964}').ends_with(T1_EMITTER_SUFFIX))
    } else {
        EMITTERS.iter().any(|e| code.contains(e))
    }
}

/// How many lines of a file are emission sites, used to pick the producer the
/// reach proof mutates.
fn emission_lines(path: &Path, text: &str) -> usize {
    text.lines()
        .filter(|raw| line_emits(path, &scan_code(path, raw)))
        .count()
}

/// ॥ NO PRODUCER PRINTS A LINE PER SLOT OF AN `UNTABLED` ARENA ॥ `W-306`.
///
/// `an_untabled_exemption_writes_the_same_thing_into_every_slot` derives the
/// FIRST clause of `अनामस्थलसंख्या`'s exemption — that the arena is a pool of
/// identical slots, because the loop that cuts it writes one literal into every
/// one. That clause says the slots are ALIKE. It does not say the exemption is
/// SAFE.
///
/// **THE SECOND CLAUSE IS THE ONE THAT SAYS SAFE, AND IT WAS PROSE NOTHING
/// READ.** The row claims "NO census anywhere prints a line per slot — so the
/// defect a name table exists to prevent, an unnamed population that reads
/// exactly like a zero, has no way to happen here". A name table exists so that
/// a reader of a census can tell entry from entry; eight anonymous slots in a
/// census that prints one line each are eight blanks a reader cannot tell from
/// each other OR from an absent entry. The first clause is a claim about ONE
/// LOOP. This one is a claim about THE REST OF THE TREE, which is why it is the
/// clause a new `UNTABLED` row is most likely to get wrong.
///
/// **IT IS DERIVABLE.** A census that prints a line per slot must NAME the
/// arena or one of its accessors, because there is no other way to reach a
/// slot. So: read every file under the producer roots, and refuse any whose
/// emission line names one.
///
/// **THAT IS STRICTLY STRONGER THAN THE CLAUSE, AND THE DIFFERENCE IS STATED
/// RATHER THAN HIDDEN.** The clause forbids a line PER SLOT; this refuses any
/// emission keyed by the arena at all, including one that prints only the
/// count. Telling those apart needs loop analysis in four languages, and that
/// reader would be the thing here most likely to be wrong — `W-305` measured a
/// depth-insensitive body reader calling a branched body uniform by reading
/// half of it. When a legitimate aggregate emission is added, this reds, names
/// the site, and the reason gets rewritten to say what is true then. An
/// over-refusal that quotes its evidence is a conversation; a silent
/// under-refusal is the defect.
///
/// **THE VACUITY GUARD IS THE POINT OF THE OTHER THREE STATES.** A scan whose
/// roots moved, whose keys were renamed, or which read nothing reports exactly
/// what a clean tree reports. So the counts are asserted: files were read, some
/// file NAMES the arena, and some file in the tree emits at all.
#[test]
fn no_producer_prints_a_line_per_slot_of_an_untabled_arena() {
    let (files, unreadable) = producer_files();
    assert!(
        files.len() > 100,
        "the producer roots hold {} readable file(s), which is too few to be \
         `crates/*/src` plus `tools/`. Either `producer_roots` is pointing \
         somewhere that no longer exists or the walk stopped early — and a scan \
         that reads nothing reports the same clean result as a scan that read \
         everything.",
        files.len()
    );
    let emitting = files
        .iter()
        .filter(|(p, t)| emission_lines(p, t) > 0)
        .count();
    assert!(
        emitting > 0,
        "NO file under the producer roots contains an emission site, which \
         cannot be true of a tree that writes a dashboard. `EMITTERS` has lost \
         the spellings this tree prints with, so every file below reads as \
         silent and this test would pass on a producer that prints a line per \
         slot in every language at once."
    );
    println!("METRIC w306_producer_files {}", files.len());
    println!("METRIC w306_producer_files_emitting {emitting}");
    println!("METRIC w306_producer_files_unreadable {unreadable}");

    let mut wrong: Vec<String> = Vec::new();
    for Untabled {
        name,
        metric,
        why,
        slots,
        ..
    } in UNTABLED
    {
        assert!(
            !slots.is_empty(),
            "`{name}`'s `slots` list is empty, so the scan below searches for \
             nothing and passes having compared nothing. A row must name the \
             ways a producer could reach one of its slots."
        );
        // AND THE ROOTS MUST STILL REACH THE FILE THAT DECLARES IT. `named > 0`
        // below catches a scan that read NOTHING; it does not catch a scan that
        // read HALF — drop `crates/*/src` from the roots and `tools/` alone
        // still names the arena once, so the count stays non-zero while the
        // declaring source has gone unread. Found by CONTENT, by the `सार्वजनिक
        // चरः` this bound is declared with, rather than by naming a path.
        let declaration = format!("सार्वजनिक चरः {name} ");
        assert!(
            files.iter().any(|(_, t)| t.contains(&declaration)),
            "no file the scan read declares `{name}`, so the producer roots no \
             longer reach the source that defines this arena. Whatever the \
             counts below say, they were taken over a tree that does not \
             contain the subject."
        );
        let (mut named, mut silent, mut never, mut elsewhere) = (0usize, 0usize, 0usize, 0usize);
        for (path, text) in &files {
            match slot_census(path, text, slots) {
                SlotCensus::Silent => silent += 1,
                SlotCensus::NamesButNeverEmits(_) => {
                    named += 1;
                    never += 1;
                }
                SlotCensus::EmitsElsewhere { .. } => {
                    named += 1;
                    elsewhere += 1;
                }
                SlotCensus::EmitsKeyed(sites) => {
                    named += 1;
                    wrong.push(format!(
                        "`{name}` is exempted from a name table partly because NO \
                         census prints a line per slot — {why} — but {} emits with \
                         one of its slot names on the emission line: {}. Either \
                         that census prints the pool's slots, in which case eight \
                         anonymous entries are about to read as eight blanks and \
                         this arena is owed a `BOUNDS` row, or it prints something \
                         aggregate about the arena and the reason must be rewritten \
                         to say so.",
                        path.strip_prefix(repo_root()).unwrap_or(path).display(),
                        sites
                            .iter()
                            .map(|(n, t)| format!("line {n}: `{t}`"))
                            .collect::<Vec<_>>()
                            .join("; ")
                    ));
                }
            }
        }
        assert!(
            named > 0,
            "NO file under the producer roots names `{name}` or any accessor of \
             it, not even the `.t1` that declares it. The scan has lost its \
             subject: either the arena was renamed and `slots` is stale, or the \
             roots no longer reach `crates/sadhana-t1/src`. Nothing below \
             compared anything.",
        );
        assert_eq!(
            silent + never + elsewhere + wrong.len(),
            files.len(),
            "every file lands in exactly one state"
        );
        println!("METRIC w306_{metric}_files_naming_the_arena {named}");
        println!("METRIC w306_{metric}_files_naming_and_never_emitting {never}");
        println!("METRIC w306_{metric}_files_naming_and_emitting_elsewhere {elsewhere}");
    }
    assert!(
        wrong.is_empty(),
        "an `UNTABLED` exemption rests on nothing printing its slots, and a \
         producer prints them:\n  {}",
        wrong.join("\n  ")
    );
}

/// ONE line of a per-slot census, written the way `ext` writes one.
///
/// This is the mutation the ledger names — "a single `println!` of a per-slot
/// line added to any producer the scan reaches" — and it is spelled four ways
/// because the scan reads four languages and a Rust macro is not an emission
/// in a `.t1`.
fn per_slot_line(ext: &str, key: &str) -> String {
    match ext {
        "t1" => format!("    अष्टकॱ{T1_EMITTER_SUFFIX} {key} अङ्कः क अन्तः ।"),
        "py" => format!("    print(f'slot {{i}}: {{{key}[i]}}')"),
        "sh" => format!("echo \"slot $i: ${{{key}[$i]}}\""),
        _ => format!("    println!(\"slot {{i}}: {{}}\", {key}[i]);"),
    }
}

/// ॥ THE SCAN REACHES A REAL PRODUCER, PROVED BY ADDING ONE LINE TO ONE ॥
///
/// The refusal above is worth exactly as much as the scan's reach. A scan that
/// walks an empty directory refuses nothing and says so in the same words as a
/// scan that walked the tree and found it clean — so the falsifier is not a
/// synthetic file, it is **one `println!` appended to the real text of a real
/// producer the scan actually read**, checked to flip that file's state.
///
/// Three more cases sit beside it, because a four-state reader fails four ways:
/// `ir.t1`'s own text must stay `NamesButNeverEmits` (a quoted printer name is
/// not a call), a margin naming both must not red, and the three synthetic
/// shapes must land in the three remaining states.
#[test]
fn a_per_slot_census_added_to_a_real_producer_is_refused() {
    let (files, _) = producer_files();
    let keys = UNTABLED[0].slots;
    let root = repo_root();

    // `ir.t1` DECLARES the pool and cuts it, and prints nothing. Its four
    // `मुद्रण` tokens are three margins and one QUOTED name at :2232, which is
    // the comparison that compiles a call to the printer rather than a call.
    // Asserted by VARIANT and not by line numbers: the lines move on every
    // edit, and pinning them would be the tax the 2026-09-13 ruling removed.
    let (ir_path, ir_text) = files
        .iter()
        .find(|(p, _)| p.ends_with("crates/sadhana-t1/src/ir.t1"))
        .expect("the scan reads ir.t1, which is under crates/sadhana-t1/src");
    assert!(
        matches!(
            slot_census(ir_path, ir_text, keys),
            SlotCensus::NamesButNeverEmits(_)
        ),
        "ir.t1 declares this arena and emits nothing; it read as {:?}",
        slot_census(ir_path, ir_text, keys)
    );

    // THE REACH PROOF, ONCE PER LANGUAGE. The producer with the MOST emission
    // sites in each — picked by READING, not by naming a path, so a file that
    // stops printing cannot quietly take the proof with it — gains ONE
    // per-slot line, written the way that language writes one.
    //
    // **PER LANGUAGE AND NOT ONCE, BECAUSE MEASURING SAID SO.** This was one
    // proof on the busiest producer overall, appending a Rust `println!`
    // whatever the file was. Emptying `EMITTERS` moved the busiest file from
    // `tools/check-wpt-rate.sh` to `crates/sadhana-t1/src/encode.t1`, where a
    // Rust `println!` is not an emission at all — the proof would have red for
    // the wrong reason on a tree whose shell scripts happened to shrink, and
    // gone GREEN on a blinded `EMITTERS` (2026-09-18). Four proofs, four
    // languages, and each one asserts its group is non-empty.
    for ext in ["rs", "py", "sh", "t1"] {
        let mut ranked: Vec<(usize, &PathBuf, &String)> = files
            .iter()
            .filter(|(p, _)| p.extension().is_some_and(|e| e == ext))
            .map(|(p, t)| (emission_lines(p, t), p, t))
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
        let (sites, path, text) =
            ranked
                .first()
                .map(|(n, p, t)| (*n, *p, *t))
                .unwrap_or_else(|| {
                    panic!(
                        "the producer roots hold no `.{ext}` file at all. This tree \
                     has producers in four languages; if one is gone the scan \
                     can no longer be shown to reach it, and a per-slot census \
                     written in it would pass unread."
                    )
                });
        assert!(
            sites > 0,
            "the busiest `.{ext}` producer under the roots, {}, has no emission \
             site at all — so `EMITTERS` no longer knows how this language \
             prints, and every `.{ext}` file reads as silent.",
            path.strip_prefix(&root).unwrap_or(path).display()
        );
        println!(
            "NOTE w306 reach .{ext} proved on {} ({sites} emission sites)",
            path.strip_prefix(&root).unwrap_or(path).display()
        );
        assert!(
            !matches!(slot_census(path, text, keys), SlotCensus::EmitsKeyed(_)),
            "the producer this mutation is applied to must be clean BEFORE it, \
             or the flip below proves nothing"
        );
        let added = per_slot_line(ext, keys[0]);
        let mutated = format!("{text}\n{added}\n");
        match slot_census(path, &mutated, keys) {
            SlotCensus::EmitsKeyed(sites) => {
                assert_eq!(sites.len(), 1, "one line was added, so one site is found");
                assert_eq!(
                    sites[0].1,
                    added.trim(),
                    "the refusal quotes the line that was added"
                );
            }
            other => panic!(
                "a per-slot census line was added to {} — a `.{ext}` file this \
                 scan READ — and the scan reported {other:?}. The scan does not \
                 reach the producers it claims to, so the exemption's second \
                 clause is unguarded in {ext}.",
                path.strip_prefix(&root).unwrap_or(path).display()
            ),
        }
    }

    // The other three states, on text small enough to read whole.
    let arena = keys[0];
    assert_eq!(
        slot_census(Path::new("a.rs"), "println!(\"hello\");\n", keys),
        SlotCensus::Silent,
        "a producer that never names the arena cannot print a slot of it"
    );
    assert_eq!(
        slot_census(
            Path::new("a.rs"),
            &format!("let n = {arena}.len();\n"),
            keys
        ),
        SlotCensus::NamesButNeverEmits(vec![1])
    );
    assert_eq!(
        slot_census(
            Path::new("a.rs"),
            &format!("let n = {arena}.len();\nprintln!(\"{{n}}\");\n"),
            keys
        ),
        SlotCensus::EmitsElsewhere {
            named: vec![1],
            emissions: 1
        },
        "naming the arena and printing are not the same line, so this is not a \
         per-slot census — and it is the state the tree's one tools/ hit sits in"
    );

    // AND THE `.t1` CASES, which is where the quote strip earns its place.
    // A CALL to the printer with the arena on the line reds ...
    assert!(matches!(
        slot_census(
            Path::new("a.t1"),
            &format!("    अष्टकॱमुद्रणम् {arena} अङ्कः क अन्तः ।\n"),
            keys
        ),
        SlotCensus::EmitsKeyed(_)
    ));
    // ... the same words QUOTED do not, because a quoted name is a string ...
    assert_eq!(
        slot_census(
            Path::new("a.t1"),
            &format!("    यदि {arena} समम् उक्तम् अष्टकॱमुद्रणम् इति आदि\n"),
            keys
        ),
        SlotCensus::NamesButNeverEmits(vec![1]),
        "ir.t1:2232's shape: the printer's NAME is compared, not called"
    );
    // ... and neither does a margin, which is prose about a census.
    assert_eq!(
        slot_census(
            Path::new("a.t1"),
            &format!("॰ अष्टकॱमुद्रणम् prints {arena} one line per slot\n"),
            keys
        ),
        SlotCensus::Silent,
        "a margin is not a census, however exactly it describes one"
    );
    // The same text in a `.rs` file is NOT stripped, on purpose: a `//` inside
    // a string would truncate a line that names the arena and prints it.
    assert!(matches!(
        slot_census(
            Path::new("a.rs"),
            &format!("// println!(\"{arena}\")\n"),
            keys
        ),
        SlotCensus::EmitsKeyed(_)
    ));
}

// ── an UNTABLED pool's layout margin is checked AGAINST THE POOL ───────────

/// One index into an anonymous-slot pool, read against the pool's own bound.
///
/// **FOUR STATES AND NOT A BOOLEAN**, which is `W-303`'s lesson, `W-305`'s and
/// `W-306`'s — and here the fourth state is the one that matters most. "inside
/// the pool", "one past its end", "past its end by more" and "not a literal at
/// all" are four sentences with four different fixes, and the last is the one a
/// `bool` cannot say: an index this reader could not evaluate is not an index
/// it checked, and answering `true` for it is a scan that reached nothing
/// reading exactly like a scan that reached everything and found it clean.
#[derive(Debug, PartialEq, Eq)]
enum SlotIndex {
    /// `index < bound` — a slot the cut actually made.
    Within(u32),
    /// `index == bound`. **THE NAMED REFUSAL.** The pool is 0-based —
    /// `ir.t1:1180`'s margin says `अनामस्थलारम्भः` is "१ + the pool's first
    /// slot", so `अनामस्थानम् ०` is the first slot and the bound itself is ONE
    /// PAST THE END. It is the value a reader who thinks in COUNTS rather than
    /// in indices writes without noticing, which is why it gets its own state
    /// and its own sentence instead of sharing `Above`'s.
    AtBound(u32),
    /// `index > bound` — a helper that outgrew the pool, which is the defect
    /// the ledger names: "a helper that gains a sixth slot walks off the pool
    /// silently".
    Above(u32),
    /// The index is not a numeral. Carries what was written there, because
    /// nothing in a text reader can bound a name or an expression, and saying
    /// so IS the finding.
    Computed(String),
}

/// `token` against `bound`, with no state standing in for another.
fn classify_slot_index(token: &str, bound: u32) -> SlotIndex {
    match devanagari_number(token) {
        Some(i) if i < bound => SlotIndex::Within(i),
        Some(i) if i == bound => SlotIndex::AtBound(i),
        Some(i) => SlotIndex::Above(i),
        None => SlotIndex::Computed(token.to_string()),
    }
}

/// Every CALL of `accessor` in `text`'s CODE, as `(1-based line, index token)`.
///
/// **THE CODE IS THE SUBJECT AND THE MARGIN IS THE CLAIM ABOUT IT.** The ledger
/// asks for the margin's indices to be bounds-checked; the defect it describes
/// — "a helper that gains a sixth slot walks off the pool silently" — happens
/// in the code, and a margin check alone would catch only someone EDITING the
/// margin to say `८`. Both are read, and they are the same sentence from two
/// sources.
///
/// **THE DECLARATION IS NOT A CALL**, and it is skipped by the token BEFORE the
/// name rather than by a line number: `वृत्तिः <accessor> आदाय` is followed by
/// `आदाय`, which would classify as `Computed` and red this test on the one line
/// that defines the thing being checked.
///
/// **MARGINS ARE STRIPPED, AND NOTHING IN THE TREE HOLDS THAT — MEASURED, AND
/// I HAD WRITTEN THE OPPOSITE.** This comment first said `ir.t1` names the
/// accessor in two margins, `:1143` and `:1175`, so the strip keeps them out.
/// **Wrong.** Both write it as `(अनामस्थानम्)`, parenthesised, and a token
/// comparison never matches that with or without the strip — dropping
/// `raw.split('॰')` here left every test in this file GREEN (2026-09-18). It is
/// the same shape `W-305` found in `slot_write_value`'s `भवति` guard, and it is
/// kept for the same reason: the day a margin writes the name bare beside a
/// numeral — `॰ पूगस्थानम् ८ would be one past the end` is a sentence someone
/// writes while FIXING this very bound — an unstripped reader reports the
/// margin as a call. It is held by that synthetic margin in
/// `an_index_at_or_past_a_pools_bound_is_refused` and by nothing else.
fn accessor_call_sites(text: &str, accessor: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let code = raw.split('॰').next().unwrap_or(raw);
        let t: Vec<&str> = code.split(' ').filter(|w| !w.is_empty()).collect();
        for (i, w) in t.iter().enumerate() {
            if *w != accessor || (i > 0 && t[i - 1] == "वृत्तिः") {
                continue;
            }
            out.push((n + 1, t.get(i + 1).copied().unwrap_or("").to_string()));
        }
    }
    out
}

/// A margin line that RECORDS a slot layout, and every index written on it.
#[derive(Debug, PartialEq, Eq)]
struct MarginLayout {
    /// The 1-based line of the margin.
    line: usize,
    /// The margin as written, so a refusal QUOTES it rather than asserting
    /// what it did not find.
    text: String,
    /// Every Devanagari numeral on the line, in order, as written.
    indices: Vec<String>,
}

/// The Devanagari numerals in `s`, as maximal runs of `०`–`९`.
///
/// Runs, so `०–५` is TWO numerals and `१०` is one. ASCII digits are not
/// Devanagari digits, so a line number like `1176` inside a margin is not an
/// index — which is exactly the false claim this would otherwise invent.
fn devanagari_numerals(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if ('०'..='९').contains(&c) {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Every MARGIN line of `text` carrying one of `markers`, with its numerals.
///
/// **EVERY NUMERAL ON THE LINE, AND THAT IS AN OVER-APPROXIMATION STATED RATHER
/// THAN HIDDEN** — `W-306`'s rule, applied again. Telling "the index a helper
/// uses" from "a number that happens to share the sentence" needs a reader for
/// two dialects of English prose, and that reader would be the thing here most
/// likely to be wrong. So every numeral in a marked margin is bounds-checked;
/// an unrelated numeral at or past the bound reds, and the refusal quotes the
/// margin, so the answer is a narrowed marker a reviewer can weigh rather than
/// an argument.
///
/// **A RANGE IS ITS ENDPOINTS.** `growth ०–५` names six indices and this reads
/// two, because a range is contained by its ends: if both ends are below the
/// bound then so is everything between them. Nothing is lost, and no dash
/// spelling has to be agreed on — `ir.t1` writes an EN DASH at `:1179` and a
/// HYPHEN at `:1316`, which a range parser would have had to know about and
/// this does not.
///
/// **A LINE WITH NO `॰` IS NOT A MARGIN**, so a marker phrase that appears in
/// code is not a layout claim. The markers are English and `.t1` code is not,
/// but the rule is the reader's, not the alphabet's.
fn margin_layouts(text: &str, markers: &[&str]) -> Vec<MarginLayout> {
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let Some((_, margin)) = raw.split_once('॰') else {
            continue;
        };
        if !markers.iter().any(|m| margin.contains(m)) {
            continue;
        }
        out.push(MarginLayout {
            line: n + 1,
            text: margin.trim().to_string(),
            indices: devanagari_numerals(margin),
        });
    }
    out
}

/// ॥ EVERY INDEX INTO AN `UNTABLED` POOL IS INSIDE IT ॥ `W-307`.
///
/// The third and last un-derived sentence of `अनामस्थलसंख्या`'s exemption, and
/// unlike the first two it is a CORRECTNESS claim rather than a prose one. The
/// row ends: "the margin at ir.t1:1176 records which helper uses which index;
/// that is a frame layout, not an arena census". That margin says the helpers
/// take FIXED indices — `growth ०–५, slice ०–४, equality ०–४, length ७` — into
/// a pool of `अनामस्थलसंख्या` = `८`, and **nothing checked that every index it
/// names is BELOW the bound**. A helper that gains a sixth slot walks off the
/// pool, and the margin still reads as a frame layout.
///
/// **TWO SOURCES FOR ONE SENTENCE, BECAUSE THE MARGIN IS NOT THE SUBJECT.** The
/// margin is a CLAIM about the code; the walk-off happens in the code. So the
/// call sites of `अनामस्थानम्` are bounds-checked too, and the margin's numerals
/// beside them. Checking only the margin would catch someone editing the
/// margin to say `८` and nothing else — which is the smaller half of the
/// defect, and the half nobody does by accident.
///
/// **THE CASE THAT MUST STILL BE REFUSED IS THE INDEX THAT EQUALS THE BOUND**,
/// from either source, and it is refused in
/// `an_index_at_or_past_a_pools_bound_is_refused` on synthetic text — so the
/// falsifier does not edit the tree this file is the only reader of. The other
/// three states sit beside it there.
///
/// **WHAT THIS DOES NOT CLAIM, SAID OUT LOUD.** The same margin also says "the
/// sequences never nest but one". That is not derivable from the margin, and
/// claiming it here would put an unchecked sentence under a checked test's
/// name. It is reported nowhere and asserted nowhere.
#[test]
fn every_index_into_an_untabled_pool_is_inside_it() {
    let text = ir_t1_text();
    let mut wrong: Vec<String> = Vec::new();
    let (mut code_checked, mut margin_checked) = (0usize, 0usize);

    for Untabled {
        name,
        metric,
        why,
        accessor,
        layout_markers,
        ..
    } in UNTABLED
    {
        let bound = ir_bound(name);
        println!("METRIC w307_pool_{metric}_bound {bound}");

        // ── THE CODE ───────────────────────────────────────────────────────
        let sites = accessor_call_sites(&text, accessor);
        assert!(
            !sites.is_empty(),
            "`{accessor}` is declared in `UNTABLED` as the routine every index \
             into `{name}` goes through, and ir.t1 CALLS it nowhere. Either the \
             pool is reached some other way now — in which case this test \
             bounds-checks an empty set and passes having compared nothing — or \
             the accessor was renamed and the row must say the new name."
        );
        let mut code_indices: BTreeSet<u32> = BTreeSet::new();
        for (line, token) in &sites {
            code_checked += 1;
            match classify_slot_index(token, bound) {
                SlotIndex::Within(i) => {
                    code_indices.insert(i);
                }
                SlotIndex::AtBound(i) => wrong.push(format!(
                    "ir.t1:{line} asks `{accessor}` for slot {i}, and `{name}` is \
                     {i}. The pool is 0-BASED — ir.t1:1180's margin calls \
                     `अनामस्थलारम्भः` \"१ + the pool's first slot\" — so slot {i} is \
                     ONE PAST THE END of the eight the cut made, and this helper \
                     writes into whatever the frame put after them. Either the \
                     bound grows or the helper uses a slot it has."
                )),
                SlotIndex::Above(i) => wrong.push(format!(
                    "ir.t1:{line} asks `{accessor}` for slot {i}, past the end of \
                     a pool of {bound}. This is the walk-off the exemption's \
                     third sentence is about: the layout margin would still read \
                     as a frame layout with this line in the file."
                )),
                SlotIndex::Computed(expr) => wrong.push(format!(
                    "ir.t1:{line} asks `{accessor}` for slot `{expr}`, which is \
                     not a numeral. NOTHING HERE CAN BOUND IT, so this call site \
                     is unchecked rather than checked-and-fine, and the pool's \
                     one safety property now rests on a value chosen at run \
                     time. Either the index becomes a literal or `{name}` needs a \
                     guard in ir.t1 rather than a reader in a test."
                )),
            }
        }
        println!("METRIC w307_pool_{metric}_code_sites {}", sites.len());
        println!(
            "METRIC w307_pool_{metric}_code_distinct_indices {}",
            code_indices.len()
        );

        // ── THE MARGIN ─────────────────────────────────────────────────────
        let layouts = margin_layouts(&text, layout_markers);
        assert!(
            !layouts.is_empty(),
            "NO margin of ir.t1 carries any of {layout_markers:?}, so the \
             layout claim this test exists to check was not read at all. \
             `{name}`'s exemption rests on that margin being \"a frame layout, \
             not an arena census\" — if it was reworded, `layout_markers` must \
             be reworded with it; if it was DELETED, the exemption's third \
             sentence has no subject and the row must lose it."
        );
        let mut margin_indices: BTreeSet<u32> = BTreeSet::new();
        for MarginLayout {
            line,
            text: margin,
            indices,
        } in &layouts
        {
            if indices.is_empty() {
                wrong.push(format!(
                    "the margin at ir.t1:{line} reads as a slot layout — it \
                     carries one of {layout_markers:?} — and names NO index at \
                     all: `{margin}`. A layout that names no slot is prose this \
                     test cannot check, and it counts toward nothing; either it \
                     is not a layout and the marker is too wide, or its indices \
                     are written in a spelling `devanagari_numerals` does not \
                     read."
                ));
                continue;
            }
            for token in indices {
                margin_checked += 1;
                match classify_slot_index(token, bound) {
                    SlotIndex::Within(i) => {
                        margin_indices.insert(i);
                    }
                    SlotIndex::AtBound(i) => wrong.push(format!(
                        "the layout margin at ir.t1:{line} names slot {i} and \
                         `{name}` is {i}: `{margin}`. The pool is 0-based, so \
                         the bound itself is one past the end — a range whose \
                         TOP is the bound names a slot that does not exist. \
                         This is the exemption's own sentence being false: \
                         {why}."
                    )),
                    SlotIndex::Above(i) => wrong.push(format!(
                        "the layout margin at ir.t1:{line} names slot {i}, past \
                         the end of a pool of {bound}: `{margin}`."
                    )),
                    SlotIndex::Computed(expr) => wrong.push(format!(
                        "`devanagari_numerals` returned `{expr}` from the margin \
                         at ir.t1:{line}, which is not a numeral — the reader \
                         is broken, not the margin."
                    )),
                }
            }
        }
        println!("METRIC w307_pool_{metric}_margin_lines {}", layouts.len());
        println!(
            "METRIC w307_pool_{metric}_margin_indices {}",
            margin_indices.len()
        );

        // THE TWO SOURCES SIDE BY SIDE — reported, and NOT asserted. The margin
        // writes RANGES and only their endpoints are read here, so the margin's
        // set is legitimately a SUBSET of the code's (`०–५` contributes ० and ५
        // and never १, २, ३). An equality here would red on correct prose, which
        // is how a guard becomes a nuisance and then a deleted line.
        println!(
            "NOTE w307 {metric} margin names {margin_indices:?}; the code uses \
             {code_indices:?}; subset by construction, so this is a METRIC and \
             not a claim"
        );
    }

    assert!(
        code_checked > 0 && margin_checked > 0,
        "this test compared {code_checked} call site(s) and {margin_checked} \
         margin index/indices; with either at zero it passes having checked \
         half its subject or none of it."
    );
    println!("METRIC w307_code_indices_checked {code_checked}");
    println!("METRIC w307_margin_indices_checked {margin_checked}");
    assert!(
        wrong.is_empty(),
        "an `UNTABLED` pool is exempted from a name table partly because its \
         layout margin is a frame layout rather than a census — and an index \
         that pool does not have is neither:\n  {}",
        wrong.join("\n  ")
    );
}

/// A minimal `.t1` with a pool bound of `८`, a layout `margin`, the accessor's
/// DECLARATION, and `body` as the call sites.
///
/// The declaration is line 3 and the body starts at line 6 in every program
/// this builds; the refusals below assert the lines they got rather than
/// assuming them.
fn layout_program(margin: &str, body: &str) -> String {
    format!(
        "सार्वजनिक चरः पूगसंख्या ॱॱ न६४ भवति ८ ।\n\
         ॰ {margin}\n\
         वृत्तिः पूगस्थानम् आदाय क्रमः ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् क्रमः ।\n\
         इति\n\
         {body}\n"
    )
}

/// ॥ AN INDEX AT OR PAST A POOL'S BOUND IS REFUSED ॥ `W-307`'s falsifier.
///
/// The named case is the index that EQUALS the bound, from the code and from
/// the margin, because the pool is 0-based and the bound is one past its end.
/// The other three states sit beside it: a four-state reader fails four ways,
/// and a test that exercised one would let the other three report as the first.
#[test]
fn an_index_at_or_past_a_pools_bound_is_refused() {
    // The baseline, and the declaration is NOT a call site.
    let ok = layout_program(
        "helper uses fixed indices: growth ०–५, length ७.",
        "    चरः स्थलम् ॱॱ न६४ भवति पूगस्थानम् ७ ।",
    );
    let sites = accessor_call_sites(&ok, "पूगस्थानम्");
    assert_eq!(
        sites,
        vec![(6, "७".to_string())],
        "the `वृत्तिः पूगस्थानम् आदाय` on line 3 declares the accessor; reading \
         it as a call would classify `आदाय` as `Computed` and red the file on \
         the one line that defines the subject"
    );
    assert_eq!(classify_slot_index(&sites[0].1, 8), SlotIndex::Within(7));

    // THE NAMED REFUSAL, in the code: one edit, `७` to the bound itself.
    let at_bound = layout_program(
        "helper uses fixed indices: growth ०–५.",
        "    चरः स्थलम् ॱॱ न६४ भवति पूगस्थानम् ८ ।",
    );
    let at = accessor_call_sites(&at_bound, "पूगस्थानम्");
    assert_eq!(
        classify_slot_index(&at[0].1, 8),
        SlotIndex::AtBound(8),
        "a pool of ८ cut 0-based holds ०–७; ८ is one past the end, and it is \
         the value a reader thinking in counts writes"
    );

    // THE SAME REFUSAL FROM THE MARGIN: a range whose TOP is the bound.
    let wide = layout_program(
        "helper uses fixed indices: growth ०–८.",
        "    चरः स्थलम् ॱॱ न६४ भवति पूगस्थानम् ० ।",
    );
    let layouts = margin_layouts(&wide, &["fixed indices", "free scratch"]);
    assert_eq!(layouts.len(), 1, "one marked margin");
    assert_eq!(layouts[0].line, 2);
    assert_eq!(layouts[0].indices, vec!["०".to_string(), "८".to_string()]);
    assert_eq!(
        classify_slot_index(&layouts[0].indices[1], 8),
        SlotIndex::AtBound(8)
    );

    // PAST THE END BY MORE is its own sentence: the helper outgrew the pool
    // rather than miscounting its last slot.
    let past = layout_program("x", "    चरः स्थलम् ॱॱ न६४ भवति पूगस्थानम् ९ ।");
    assert_eq!(
        classify_slot_index(&accessor_call_sites(&past, "पूगस्थानम्")[0].1, 8),
        SlotIndex::Above(9)
    );

    // AND THE INDEX THAT IS NOT A LITERAL, which is the state a `bool` cannot
    // report: unchecked, not checked-and-fine.
    let computed = layout_program("x", "    चरः स्थलम् ॱॱ न६४ भवति पूगस्थानम् क्रमः ।");
    assert_eq!(
        classify_slot_index(&accessor_call_sites(&computed, "पूगस्थानम्")[0].1, 8),
        SlotIndex::Computed("क्रमः".to_string())
    );
    // Nothing after the name at all is the same finding with an empty quote —
    // and not a silently dropped call site.
    let dangling = layout_program("x", "    चरः स्थलम् ॱॱ न६४ भवति पूगस्थानम्");
    let d = accessor_call_sites(&dangling, "पूगस्थानम्");
    assert_eq!(d.len(), 1, "a call with no argument is still a call site");
    assert_eq!(
        classify_slot_index(&d[0].1, 8),
        SlotIndex::Computed(String::new())
    );

    // A MARGIN IS NOT A CALL — and this case is written the way it is because
    // the obvious way passes for the wrong reason. ir.t1:1143 and :1175 name
    // the accessor as `(अनामस्थानम्)`, PARENTHESISED, which a token comparison
    // misses with or without the margin strip; a synthetic copy of that shape
    // is green on an unstripped reader and proves nothing (measured
    // 2026-09-18). The margin here writes the name BARE beside a numeral —
    // the sentence someone writes while fixing this very bound — so an
    // unstripped reader reads it as a call asking for the slot one past the
    // end, and this is the only thing holding the strip.
    let quoted = layout_program(
        "helper uses fixed indices: growth ०–५.",
        "    चरः स्थलम् ॱॱ न६४ भवति पूगस्थानम् ० ।   ॰ पूगस्थानम् ८ would be one past the end",
    );
    assert_eq!(
        accessor_call_sites(&quoted, "पूगस्थानम्"),
        vec![(6, "०".to_string())],
        "prose naming the accessor is not a call to it, however exactly it \
         spells one — without the margin strip this reads TWO sites and the \
         second asks for the bound itself"
    );
    // ... and CODE carrying a marker phrase is not a layout margin.
    assert!(
        margin_layouts("चरः fixed indices ॱॱ न६४ भवति ८ ।\n", &["fixed indices"]).is_empty(),
        "a marked LINE is not a marked margin; the marker must sit after `॰`"
    );

    // A marked margin naming no index is reported, not skipped: a layout that
    // names no slot is prose, and counting it as checked would be the defect.
    let mute = margin_layouts(
        "॰ helper uses fixed indices, see above.\n",
        &["fixed indices"],
    );
    assert_eq!(mute.len(), 1);
    assert!(mute[0].indices.is_empty());

    // THE NUMERAL READER. Runs split on BOTH dashes ir.t1 uses — an EN DASH at
    // :1179 and a HYPHEN at :1316 — so a range is its two endpoints either way,
    // and no dash spelling has to be agreed on. ASCII digits are not indices:
    // `ir.t1:1176` inside a margin is a citation, and reading `1176` as a slot
    // would red every layout margin that cites a line.
    assert_eq!(
        devanagari_numerals("growth ०–५, slice ०-४, length ७."),
        ["०", "५", "०", "४", "७"]
    );
    assert_eq!(devanagari_numerals("१० slots"), ["१०"]);
    assert!(devanagari_numerals("the margin at ir.t1:1176 records").is_empty());
}

/// **`W-333` — KIND १० IS TWO INSTRUCTIONS, AND THE GUARD ABOVE CANNOT SAY SO.**
///
/// `ir.t1` numbers १० as `Instruction::Shr`, and since `W-333` the SAME kind
/// with `ध्रुवमूल्यम्` १ is `Instruction::ShrL`. The table guard reads one
/// variant per number — the first in the arm — so a copy whose arm for १०
/// builds `Shr` and never mentions `ShrL` passes it. That is `W-306c`'s hazard
/// exactly: a decoder that reads the kind and drops the field compiles, stays
/// green, and answers a different shift from its siblings.
///
/// So every copy of the `Instruction` table that has an arm for १० must name
/// BOTH variants inside that arm. This is a check on the TEXT, as the guard
/// above is: it shows the arm knows there are two, not that it picks the
/// right one — `w333_shift_mark.rs` executes that for `chain.rs`, and `W-368`
/// is the row for feeding the other three a marked instruction.
///
/// THE COUNT IS ASSERTED TOO. Four copies exist; a scan that found fewer has
/// stopped matching the arms' shape, and its silence would read as agreement.
///
/// AND THE SHAPE OF THE ARM IS CONSTRAINED BY THE GUARD ABOVE, WHICH IS HOW
/// THIS TEST CAME TO EXIST: the first version of those four arms was a nested
/// `match mark { 0 => Instruction::Shr(..), 1 => Instruction::ShrL(..), .. }`,
/// and `numeric_arms` read the nested arms as rows ० and १ of the kind table —
/// "Instruction 1 decodes to `ShrL`; ir.t1 numbers 1 as `ConstInt`", six
/// disagreements in four files, found by the gate and not by me. They are
/// written with `if` now, `Shr` first.
#[test]
#[ignore = "blocked: needs the full development repository's source copies not in the public repository"]
fn every_copy_of_the_instruction_table_names_both_shifts_under_kind_ten() {
    let mut seen = Vec::new();
    let mut silent = Vec::new();
    for (path, text) in rust_sources() {
        // This file quotes the arms it checks.
        if path.ends_with("t1_transcriptions.rs") {
            continue;
        }
        let code = strip_comments_and_raw_strings(&text);
        for (number, body) in numeric_arms(&code) {
            if number != 10 || !body.contains("Instruction::Shr") {
                continue;
            }
            seen.push(path.clone());
            if !body.contains("Instruction::ShrL") {
                silent.push(path.clone());
            }
        }
    }
    println!("METRIC sadhana_t1_kind_ten_decoders {}", seen.len());
    assert_eq!(
        seen.len(),
        4,
        "the four hand-written decoders of kind १० (chain.rs, pathana.rs, \
         t1_exec_riscv.rs, paradigm_encode.rs); found {seen:?}"
    );
    assert!(
        silent.is_empty(),
        "kind १० is `Shr` or `ShrL` by `ध्रुवमूल्यम्`, and these copies build `Shr` \
         without naming `ShrL`: {silent:?}"
    );
}
