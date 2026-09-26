//! Language-completeness corpus — lexical validation (doc 02 task 2.4.6).
//!
//! The corpus in `tests/corpus/t1/` answers the question *"can every kind of
//! logic, data and operation be expressed in Sassembly-sūtra?"* — 16 programs
//! covering scalars through MMIO device registers, catalogued in
//! `tests/corpus/CATALOG.tsv`.
//!
//! The T1 compiler does not exist yet, so nothing here can be *compiled*. What
//! **can** be checked today is everything below the parser, and checking it now
//! is the point: the corpus is the executable specification, so it must be valid
//! before the implementation it specifies exists.
//!
//! Checked per program:
//!
//! 1. **Repertoire gate** (doc 15 R-15-1) — only Sanskrit characters.
//! 2. **NFC** (doc 01 D-01-B) — one canonical spelling.
//! 3. **Akṣara segmentation** — total and lossless.
//! 4. **Word-level identifier validity** — every Devanagari word is either a
//!    valid identifier or built from valid identifiers joined by the ADR-0003
//!    syntax signs.
//!
//! These four are honest, and they are not nothing: together they prove the
//! corpus is writable under orthographic closure at all.
//!
//! **They are also all that will ever read it.** The 16 were written on
//! 2026-08-13 (`8f20f5d5`), before `spec/grammar-t1.ebnf` was frozen from the
//! `.t1` corpus, in a dialect that grammar does not describe (`ॐ`,
//! `॥ मण्डलम् X ॥`, `… समाप्तम् फलम् T आदि`, 0 `आदाय`, 0 `ददाति`; `ध्रुवः`,
//! `यन्त्रम्`, `अपेक्षा`, `विघ्नवृत्तिः`, `समयकृत`, none of which the `.t1`
//! corpus writes). W-238 (2026-09-04, research/25 §1.4a) decided they are a
//! RETIRED DIALECT: kept, marked on line 2, counted 0 readable by design, and
//! neither rewritten into the grammar nor read by a widened parser.
//! [`the_corpus_is_a_retired_dialect_by_decision`] pins that, and
//! [`an_unmarked_seventeenth_program_is_refused_by_name`] is its refused case.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sanskrit_text::{aksharas, is_nfc, is_numeral, repertoire_check, validate_identifier};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .join("tests/corpus/t1")
}

fn programs() -> Vec<(String, String)> {
    let dir = corpus_dir();
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("सस"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect();
    out.sort();
    assert!(
        out.len() >= 16,
        "expected the full corpus, found {}",
        out.len()
    );
    out
}

/// Signs ADR-0003 gives a syntactic role. Words are split on these.
const SYNTAX_SIGNS: &[char] = &[
    '\u{0964}', '\u{0965}', '\u{0970}', '\u{0971}', '\u{093D}', '\u{0950}',
];

#[test]
fn every_program_passes_the_repertoire_gate() {
    let mut bad = Vec::new();
    for (name, src) in programs() {
        for v in repertoire_check(&src) {
            bad.push(format!(
                "  {name}:{}:{} U+{:04X} {:?}",
                v.line, v.column, v.ch as u32, v.ch
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} character(s) outside the Sanskrit repertoire (doc 15 R-15-1):\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn every_program_is_nfc() {
    for (name, src) in programs() {
        assert!(is_nfc(&src), "{name} is not in NFC (doc 01 D-01-B)");
    }
}

#[test]
fn every_program_segments_cleanly() {
    for (name, src) in programs() {
        let mut total = 0;
        for a in aksharas(&src) {
            assert!(!a.is_empty(), "{name}: empty akṣara");
            total += a.len();
        }
        assert_eq!(total, src.len(), "{name}: lossy segmentation");
    }
}

/// Every Devanagari word must be a legal identifier or a legal numeral. This is the strongest
/// check available without a parser, and it catches the realistic mistake:
/// writing a keyword or name that the lexer will later refuse.
#[test]
fn every_word_is_a_valid_identifier() {
    let mut bad: BTreeSet<String> = BTreeSet::new();

    for (name, src) in programs() {
        // Comments are prose, not code — skip from ॰ to end of line.
        for line in src.lines() {
            let code = line.split('\u{0970}').next().unwrap_or("");
            for word in code.split(|c: char| c.is_whitespace() || SYNTAX_SIGNS.contains(&c)) {
                if word.is_empty() {
                    continue;
                }
                // A word is either a name or a numeric literal; the lexer
                // decides by first character, and so does this.
                if is_numeral(word) {
                    continue;
                }
                if let Err(e) = validate_identifier(word) {
                    bad.insert(format!("  {name}: {word:?} -> {e:?}"));
                }
            }
        }
    }

    assert!(
        bad.is_empty(),
        "{} word(s) are not valid identifiers:\n{}",
        bad.len(),
        bad.iter().take(25).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// The catalogue and the corpus must not drift apart: every program named in
/// `CATALOG.tsv` has to exist, or a claimed capability is covered by nothing.
#[test]
fn catalogue_matches_the_corpus() {
    let catalog = corpus_dir().parent().unwrap().join("CATALOG.tsv");
    let text = std::fs::read_to_string(&catalog).unwrap();

    let on_disk: BTreeSet<String> = programs()
        .into_iter()
        .map(|(n, _)| n.trim_end_matches(".सस").to_string())
        .collect();

    let mut missing = BTreeSet::new();
    let mut claimed = BTreeSet::new();
    let mut rows = 0usize;

    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("id\t") || line.trim().is_empty() {
            continue;
        }
        rows += 1;
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 6 {
            continue;
        }
        let (program, status) = (f[4], f[5]);
        if program == "-" {
            assert_eq!(
                status, "spec",
                "row {} has no program but claims {status}",
                f[0]
            );
            continue;
        }
        claimed.insert(program.to_string());
        if !on_disk.contains(program) {
            missing.insert(format!(
                "  {} claims program {program:?}, which does not exist",
                f[0]
            ));
        }
    }

    assert!(
        rows >= 80,
        "catalogue should enumerate the whole language, got {rows} rows"
    );
    assert!(
        missing.is_empty(),
        "catalogue drift:\n{}",
        missing.iter().cloned().collect::<Vec<_>>().join("\n")
    );

    // And the reverse: a program nothing claims is untested surface.
    let unclaimed: Vec<_> = on_disk.difference(&claimed).collect();
    assert!(
        unclaimed.is_empty(),
        "corpus programs not referenced by any catalogue row: {unclaimed:?}"
    );
}

/// Report coverage, so the completeness question has a number rather than a
/// feeling. Printed with `--nocapture`.
#[test]
fn report_capability_coverage() {
    let catalog = corpus_dir().parent().unwrap().join("CATALOG.tsv");
    let text = std::fs::read_to_string(&catalog).unwrap();
    let mut by_status: std::collections::BTreeMap<&str, usize> = Default::default();
    let mut by_area: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();

    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("id\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 6 {
            continue;
        }
        *by_status.entry(f[5]).or_default() += 1;
        let e = by_area.entry(f[1]).or_default();
        e.1 += 1;
        if f[5] != "spec" {
            e.0 += 1;
        }
    }

    let total: usize = by_status.values().sum();
    let covered: usize = total - by_status.get("spec").copied().unwrap_or(0);
    println!("METRIC t1_capabilities_covered {covered}");
    println!("METRIC t1_capabilities_total {total}");
    println!("\nT1 language completeness — {covered}/{total} capabilities have a corpus program");
    for (area, (has, all)) in &by_area {
        println!("  {area:10} {has:3}/{all:<3}");
    }
    println!("  status: {by_status:?}");
    assert!(covered * 100 / total >= 90, "coverage below 90%");
}

/// Line 2 of every retired program, verbatim. *Retired dialect — a document
/// written before the grammar, not a program; decision in research 25 §1.4,
/// task 238.* Every character is in the repertoire (R-15-1), so the four
/// lexical tests above keep passing over it.
const RETIRED_MARK: &str = "॰ निवृत्ता बोली ॱॱ व्याकरणात् पूर्वं लिखितः लेखः ऽ न कार्यक्रमः । \
                            निर्णयः अनुसन्धाने २५ ऽ अंशे १ ऽ ४ ऽ कार्ये २३८ ।";

/// The 16 programs the decision named. A 17th is refused unless the decision
/// is amended here — by name, so the amendment says which file it admits.
const RETIRED: usize = 16;

/// Everything in `dir` that is `.सस` and not a correctly retired program, one
/// line per refusal, each naming the file. Shared by the pin and its refused
/// case so the two cannot drift apart.
fn retired_dialect_refusals(dir: &Path) -> Vec<String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("सस"))
        .collect();
    files.sort();

    let mut refused = Vec::new();
    if files.len() != RETIRED {
        refused.push(format!(
            "{} .सस files, not {RETIRED}: the retired set is closed by decision",
            files.len()
        ));
    }
    for p in &files {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(p).unwrap();
        let mut lines = src.lines();
        // Still the dialect: a file rewritten into the frozen grammar would
        // make its marker a lie, and it is refused here rather than counted.
        if lines.next() != Some("ॐ") {
            refused.push(format!(
                "{name}: line 1 is not `ॐ` — not the retired dialect"
            ));
        }
        if lines.next() != Some(RETIRED_MARK) {
            refused.push(format!("{name}: line 2 is not the retirement marker"));
        }
        for (i, line) in src.lines().enumerate() {
            let code = line.split('\u{0970}').next().unwrap_or("");
            if code.contains("आदाय") || code.contains("ददाति") {
                refused.push(format!(
                    "{name}:{}: writes `आदाय`/`ददाति` — the frozen grammar's routine \
                     head, which this dialect never wrote",
                    i + 1
                ));
            }
        }
    }
    refused
}

/// The corpus is a retired dialect, by decision (W-238): exactly 16 files,
/// each opening `ॐ` and carrying the marker on line 2, none written in the
/// frozen grammar's routine head. A file that stops satisfying any of the
/// three fails here by name.
#[test]
fn the_corpus_is_a_retired_dialect_by_decision() {
    let refused = retired_dialect_refusals(&corpus_dir());
    assert!(
        refused.is_empty(),
        "the .सस corpus is retired by decision (research/25 §1.4a) and {} refusal(s) \
         say where it is not:\n  {}",
        refused.len(),
        refused.join("\n  ")
    );
}

/// The refused case: a 17th `.सस` without the marker, dropped beside faithful
/// copies of the 16, is refused **by its file name** — twice, once for the
/// count and once for the missing marker.
#[test]
fn an_unmarked_seventeenth_program_is_refused_by_name() {
    let tmp = std::env::temp_dir().join(format!(
        "w238-seventeenth-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    for (name, src) in programs() {
        std::fs::write(tmp.join(&name), src).unwrap();
    }
    assert!(
        retired_dialect_refusals(&tmp).is_empty(),
        "the copies must be accepted, or the refusal below proves nothing"
    );

    // Written in the frozen grammar, unmarked: the file W-238's option (A)
    // would have produced.
    let seventeenth = "सप्तदशम्.सस";
    std::fs::write(
        tmp.join(seventeenth),
        "मण्डलम् सप्तदशम् ॥\n\nवृत्तिः क आदाय ख ॱॱ अ६४ ददाति अ६४ आदि\n    प्रत्यागमनम् ख ।\nइति\n",
    )
    .unwrap();
    let refused = retired_dialect_refusals(&tmp);
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        refused.iter().any(|r| r.contains("17 .सस files")),
        "the count must be refused: {refused:?}"
    );
    let named: Vec<&String> = refused
        .iter()
        .filter(|r| r.starts_with(seventeenth))
        .collect();
    assert!(
        named.iter().any(|r| r.contains("line 1 is not `ॐ`"))
            && named
                .iter()
                .any(|r| r.contains("line 2 is not the retirement marker"))
            && named.iter().any(|r| r.contains("`आदाय`/`ददाति`")),
        "the 17th must be refused BY NAME for each of the three reasons: {refused:?}"
    );
    assert!(
        refused
            .iter()
            .all(|r| r.starts_with(seventeenth) || r.contains("17 .सस files")),
        "no faithful copy may be blamed for the 17th: {refused:?}"
    );
}
