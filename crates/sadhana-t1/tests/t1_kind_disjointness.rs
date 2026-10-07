//! **W-171 — THE TYPE KINDS AND THE EXPRESSION KINDS MUST BE DISJOINT.**
//!
//! `व्याकरॱप्रकारपठनम्` stores every type node in `अभिव्यञ्जककोश`, the
//! EXPRESSION arena (`वास्तुॱप्रकार` is declared and never constructed), so
//! the `भेद` field of one arena carries two families. From 2026-09-01 to the
//! fix, `मूलप्रकारभेद` and `नामाभिव्यञ्जकभेद` both held १ and a kind-१ node
//! was ambiguous. This test pins the remedy: the two families' values may
//! never intersect.
//!
//! The scan is its own loader on purpose — the census loaders in
//! `t1_paradigm*.rs` carry counts other tests are keyed to.
//!
//! SANDHI, measured 2026-10-01: `नामाभिव्यञ्जकभेद` does NOT contain the
//! standalone `अभिव्यञ्जक` — the compound absorbs the initial अ into the
//! preceding आ-मात्रा — so the expression family is matched by the suffix
//! `भिव्यञ्जकभेद`, never by the word with its vowel.

use std::collections::BTreeMap;
use std::path::Path;

/// Devanagari numeral → value. Refuses anything that is not purely ०-९.
fn devanagari(num: &str) -> Option<i64> {
    let mut v: i64 = 0;
    let mut any = false;
    for c in num.chars() {
        let d = ('०'..='९').contains(&c).then(|| c as i64 - '०' as i64)?;
        v = v * 10 + d;
        any = true;
    }
    any.then_some(v)
}

/// Every `सार्वजनिक चरः <name> ॱॱ न६४ भवति <n> ।` in the text, split on
/// spaces (`\b` does not work with Devanagari).
fn constants(text: &str) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.len() >= 8
            && w[0] == "सार्वजनिक"
            && w[1] == "चरः"
            && w[3] == "ॱॱ"
            && w[4] == "न६४"
            && w[5] == "भवति"
            && let Some(v) = devanagari(w[6])
        {
            out.insert(w[2].to_string(), v);
        }
    }
    out
}

fn family<'a>(all: &'a BTreeMap<String, i64>, suffix: &str) -> BTreeMap<&'a str, i64> {
    all.iter()
        .filter(|(n, _)| n.ends_with(suffix))
        .map(|(n, v)| (n.as_str(), *v))
        .collect()
}

fn ast_text() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ast.t1");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// The W-171 pin. Fails on any shared value between the two families that
/// share `अभिव्यञ्जककोश`'s `भेद` field.
#[test]
fn the_type_kinds_and_the_expression_kinds_share_no_value() {
    let all = constants(&ast_text());
    let types = family(&all, "प्रकारभेद");
    let exprs = family(&all, "भिव्यञ्जकभेद");

    // The instrument says more than red/green: a scanner that matched nothing
    // would report disjointness vacuously, so the counts are pinned first.
    assert_eq!(types.len(), 5, "five type kinds, ast.t1 — got {types:?}");
    assert!(
        exprs.len() >= 9,
        "at least the nine expression kinds of 2026-09-01 — got {exprs:?}"
    );

    for (tn, tv) in &types {
        for (en, ev) in &exprs {
            assert_ne!(
                tv, ev,
                "`{tn}` and `{en}` share the value {tv} in one arena — W-171"
            );
        }
    }
}

/// Within each family a value names one kind.
#[test]
fn no_family_reuses_a_value_within_itself() {
    let all = constants(&ast_text());
    for suffix in ["प्रकारभेद", "भिव्यञ्जकभेद", "द्विकर्मभेद"]
    {
        let fam = family(&all, suffix);
        assert!(!fam.is_empty(), "family {suffix} must match — sandhi check");
        let mut seen: BTreeMap<i64, &str> = BTreeMap::new();
        for (n, v) in fam {
            if let Some(prev) = seen.insert(v, n) {
                panic!("{prev} and {n} both hold {v}");
            }
        }
    }
}

// ═══ W-337 — A COUNT WRITTEN IN PROSE MUST HAVE A READER. ═══
//
// Four record margins in `ast.t1` state a family's size in English prose
// ("which of the five kinds above"). Two drifted — each addition of a kind
// was correct and none touched the prose two dozen lines away — and the fix
// is this reader, not the two numbers: a hand correction resets a clock that
// drifts at the next appended kind.

/// English count word → value, for the prose the margins are written in.
/// A word outside the table REFUSES rather than skips: a margin this reader
/// cannot parse is exactly the unread-count defect W-337 names.
fn count_word(word: &str) -> Option<usize> {
    [
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ]
    .iter()
    .position(|w| *w == word)
    .map(|i| i + 1)
}

/// Every record-field margin of the shape `… ॰ … which of the <word> … kinds …`,
/// as `(record, field, stated count)`. Record tracking is by syntax
/// (`सार्वजनिक संरचना <name> आरभ्य` … `समाप्तम्`), fields by first token,
/// split on spaces as `constants` is.
fn margin_counts(text: &str) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    let mut record = String::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.len() >= 4 && w[0] == "सार्वजनिक" && w[1] == "संरचना" && w[3] == "आरभ्य"
        {
            record = w[2].to_string();
            continue;
        }
        if w.first() == Some(&"समाप्तम्") {
            record.clear();
            continue;
        }
        if record.is_empty() {
            continue;
        }
        let Some((field_part, comment)) = line.split_once('॰') else {
            continue;
        };
        let Some(field) = field_part.split_whitespace().next() else {
            continue;
        };
        let lc = comment.to_lowercase();
        let Some(rest) = lc.split("which of the").nth(1) else {
            continue;
        };
        if !rest.contains("kinds") {
            continue;
        }
        let word = rest.split_whitespace().next().unwrap_or("");
        let n = count_word(word).unwrap_or_else(|| {
            panic!("{record}.{field}: margin states a count word this reader does not know: {word:?} — extend the table, do not skip")
        });
        out.push((record.clone(), field.to_string(), n));
    }
    out
}

/// Which constant family a margin's count refers to, by (record, field).
/// The suffixes are the post-sandhi forms — see the module header.
fn margin_family(record: &str, field: &str) -> Option<&'static str> {
    match (record, field) {
        (_, "द्विकर्म") => Some("द्विकर्मभेद"),
        ("प्रकार", "भेद") => Some("प्रकारभेद"),
        ("अभिव्यञ्जक", "भेद") => Some("भिव्यञ्जकभेद"),
        ("वाक्य", "भेद") => Some("वाक्यभेद"),
        _ => None,
    }
}

/// Every margin whose stated count disagrees with the measured family size,
/// naming both numbers and the family.
fn margin_disagreements(text: &str) -> Vec<String> {
    let all = constants(text);
    margin_counts(text)
        .into_iter()
        .filter_map(|(record, field, stated)| {
            let suffix = margin_family(&record, &field).unwrap_or_else(|| {
                panic!("{record}.{field}: margin states a count but this reader knows no family for it — W-337's defect class, add the mapping")
            });
            let measured = family(&all, suffix).len();
            (stated != measured).then(|| {
                format!("{record}.{field}: the margin says {stated} but the {suffix} family measures {measured}")
            })
        })
        .collect()
}

/// The W-337 pin: every count a margin states is the one the constants
/// measure. Non-vacuous first — all four known margins must be seen, so a
/// scanner regression cannot pass by matching nothing.
#[test]
fn every_margin_count_is_the_measured_count() {
    let text = ast_text();
    let margins = margin_counts(&text);
    for want in [
        ("प्रकार", "भेद"),
        ("अभिव्यञ्जक", "भेद"),
        ("अभिव्यञ्जक", "द्विकर्म"),
        ("वाक्य", "भेद"),
    ] {
        assert!(
            margins
                .iter()
                .any(|(r, f, _)| (r.as_str(), f.as_str()) == want),
            "margin {want:?} not seen — the scanner lost a reader; got {margins:?}"
        );
    }
    let bad = margin_disagreements(&text);
    assert!(
        bad.is_empty(),
        "stale margin counts in ast.t1 — W-337:\n{}",
        bad.join("\n")
    );
}

/// The case that must still be REFUSED: the instrument, fed W-337's actual
/// defect in miniature — a margin saying thirteen over a family of two —
/// must name both numbers and the family.
#[test]
fn the_instrument_sees_a_stale_margin() {
    let old = "सार्वजनिक चरः योगद्विकर्मभेद ॱॱ न६४ भवति १ ।\n\
               सार्वजनिक चरः शेषद्विकर्मभेद ॱॱ न६४ भवति २ ।\n\
               सार्वजनिक संरचना अभिव्यञ्जक आरभ्य\n\
                   द्विकर्म ॱॱ न६४ ऽ ॰ which of the thirteen operator kinds\n\
               समाप्तम् ।\n";
    let got = margin_disagreements(old);
    assert_eq!(got.len(), 1, "exactly the one stale margin: {got:?}");
    assert!(
        got[0].contains("13") && got[0].contains('2') && got[0].contains("द्विकर्मभेद"),
        "the refusal names both numbers and the family: {}",
        got[0]
    );
}

/// The case that must still be REFUSED: the instrument, fed the 2026-09-01
/// collision verbatim, must see it. A checker that cannot turn red proves
/// nothing by staying green.
#[test]
fn the_instrument_sees_the_original_collision() {
    let old = "सार्वजनिक चरः मूलप्रकारभेद ॱॱ न६४ भवति १ ।\n\
               सार्वजनिक चरः नामाभिव्यञ्जकभेद ॱॱ न६४ भवति १ ।\n";
    let all = constants(old);
    let types = family(&all, "प्रकारभेद");
    let exprs = family(&all, "भिव्यञ्जकभेद");
    assert_eq!(
        (types.len(), exprs.len()),
        (1, 1),
        "sandhi: both lines match"
    );
    assert_eq!(
        types["मूलप्रकारभेद"], exprs["नामाभिव्यञ्जकभेद"],
        "the historical collision is what this instrument exists to see"
    );
}
