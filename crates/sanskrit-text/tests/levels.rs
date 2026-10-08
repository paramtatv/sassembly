//! The ten-level conformance ladder.
//!
//! `tests/levels/LEVELS.tsv` defines ten gates of increasing algorithmic and
//! feature complexity; `tests/levels/N/` holds the programs. Levels are
//! cumulative, so a failure at level 7 is not a level-7 bug until 1–6 are green.
//!
//! This is also the compiler's development order. "Build the compiler" is not a
//! task anyone can start; "make level 1 pass" is. The ladder turns an open-ended
//! project into ten dated checkpoints that each either pass or do not.
//!
//! Until the T1 compiler exists, what is checked is what can be: every program
//! is lexically valid, every level is non-empty, the ladder is monotonic, and
//! each level's declared feature needs actually exist in `CATALOG.tsv`. The
//! `compiled`, `runs` and `perf` gates land with stage B.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use sanskrit_text::{aksharas, is_nfc, is_numeral, repertoire_check, validate_identifier};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

struct Level {
    n: usize,
    devanagari_n: String,
    name: String,
    english: String,
    needs: Vec<String>,
    programs: Vec<String>,
    perf: String,
    gate: String,
}

/// `१`..`१०` — the levels are numbered in Devanagari, like everything else.
fn deva_to_usize(s: &str) -> Option<usize> {
    let mut n = 0usize;
    for c in s.chars() {
        let d = (c as u32).checked_sub(0x0966).filter(|d| *d < 10)?;
        n = n * 10 + d as usize;
    }
    Some(n)
}

fn levels() -> Vec<Level> {
    let path = root().join("tests/levels/LEVELS.tsv");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("level\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert!(
            f.len() >= 7,
            "malformed LEVELS.tsv row ({} cols): {line}",
            f.len()
        );
        let n = deva_to_usize(f[0])
            .unwrap_or_else(|| panic!("level {:?} is not a Devanagari numeral", f[0]));
        out.push(Level {
            n,
            devanagari_n: f[0].to_string(),
            name: f[1].into(),
            english: f[2].into(),
            needs: f[3].split(',').map(str::to_string).collect(),
            programs: f[4].split(',').map(str::to_string).collect(),
            perf: f[5].into(),
            gate: f[6].into(),
        });
    }
    out
}

fn level_programs(l: &Level) -> Vec<(String, String)> {
    let dir = root().join("tests/levels").join(&l.devanagari_n);
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("सस"))
        .map(|p| {
            (
                p.file_stem().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn the_ladder_has_exactly_ten_levels() {
    let ls = levels();
    assert_eq!(ls.len(), 10, "the ladder must have ten levels");
    let ns: Vec<usize> = ls.iter().map(|l| l.n).collect();
    assert_eq!(
        ns,
        (1..=10).collect::<Vec<_>>(),
        "levels must be 1..10 in order"
    );
}

/// Every level must name a gate and a performance budget. "All levels should
/// fly" — a level that passes slowly has not passed.
#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn every_level_declares_a_gate_and_a_budget() {
    for l in levels() {
        assert!(
            !l.gate.trim().is_empty() && l.gate != "-",
            "level {} has no gate",
            l.n
        );
        assert!(
            !l.perf.trim().is_empty() && l.perf != "-",
            "level {} has no perf budget",
            l.n
        );
        assert!(!l.name.trim().is_empty(), "level {} has no name", l.n);
        assert!(
            !l.english.trim().is_empty(),
            "level {} has no description",
            l.n
        );
    }
}

#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn every_declared_program_exists_and_every_program_is_declared() {
    for l in levels() {
        let on_disk: BTreeSet<String> = level_programs(&l).into_iter().map(|(n, _)| n).collect();
        let declared: BTreeSet<String> = l.programs.iter().map(|s| s.trim().to_string()).collect();
        assert!(!on_disk.is_empty(), "level {} has no programs", l.n);
        let missing: Vec<_> = declared.difference(&on_disk).collect();
        let extra: Vec<_> = on_disk.difference(&declared).collect();
        assert!(
            missing.is_empty(),
            "level {} declares missing programs: {missing:?}",
            l.n
        );
        assert!(
            extra.is_empty(),
            "level {} has undeclared programs: {extra:?}",
            l.n
        );
    }
}

/// Every feature a level says it needs must be a real capability id, or the
/// ladder is describing a language that does not exist.
#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn level_requirements_resolve_to_real_capabilities() {
    let cat = std::fs::read_to_string(root().join("tests/corpus/CATALOG.tsv")).unwrap();
    let ids: BTreeSet<&str> = cat
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("id\t"))
        .filter_map(|l| l.split('\t').next())
        .filter(|s| !s.is_empty())
        .collect();

    let mut unknown = Vec::new();
    for l in levels() {
        for need in &l.needs {
            let need = need.trim();
            // Level 8 and 10 inherit rather than enumerate.
            if need.is_empty() || need.starts_with("all") || need == "everything" {
                continue;
            }
            if !ids.contains(need) {
                unknown.push(format!("  level {}: {need}", l.n));
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "levels require unknown capabilities:\n{}",
        unknown.join("\n")
    );
}

/// Complexity must actually increase. Measured crudely but objectively: total
/// akṣaras of program text per level, which should trend upward.
#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn the_ladder_is_monotonic_in_complexity() {
    let mut sizes: BTreeMap<usize, usize> = BTreeMap::new();
    for l in levels() {
        let total: usize = level_programs(&l)
            .iter()
            .map(|(_, src)| aksharas(src).count())
            .sum();
        sizes.insert(l.n, total);
    }
    // Not strictly increasing step by step — level 9 is dense but short — but
    // the back half must clearly exceed the front half.
    let front: usize = (1..=5).map(|i| sizes[&i]).sum();
    let back: usize = (6..=10).map(|i| sizes[&i]).sum();
    assert!(
        back > front,
        "levels 6-10 ({back} akṣaras) should exceed levels 1-5 ({front})"
    );
    for (n, s) in &sizes {
        assert!(*s > 100, "level {n} is too thin at {s} akṣaras");
    }
}

#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn every_level_program_passes_the_repertoire_gate() {
    let mut bad = Vec::new();
    for l in levels() {
        for (name, src) in level_programs(&l) {
            for v in repertoire_check(&src) {
                bad.push(format!(
                    "  level {}/{name}:{}:{} U+{:04X} {:?}",
                    l.n, v.line, v.column, v.ch as u32, v.ch
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} violation(s):\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn every_level_program_is_nfc_and_segments_cleanly() {
    for l in levels() {
        for (name, src) in level_programs(&l) {
            assert!(is_nfc(&src), "level {}/{name} is not NFC", l.n);
            let total: usize = aksharas(&src).map(str::len).sum();
            assert_eq!(total, src.len(), "level {}/{name}: lossy segmentation", l.n);
        }
    }
}

const SYNTAX_SIGNS: &[char] = &[
    '\u{0964}', '\u{0965}', '\u{0970}', '\u{0971}', '\u{093D}', '\u{0950}',
];

#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn every_level_word_is_a_valid_identifier_or_numeral() {
    let mut bad: BTreeSet<String> = BTreeSet::new();
    for l in levels() {
        for (name, src) in level_programs(&l) {
            for line in src.lines() {
                let code = line.split('\u{0970}').next().unwrap_or("");
                for word in code.split(|c: char| c.is_whitespace() || SYNTAX_SIGNS.contains(&c)) {
                    if word.is_empty() || is_numeral(word) {
                        continue;
                    }
                    if let Err(e) = validate_identifier(word) {
                        bad.insert(format!("  level {}/{name}: {word:?} -> {e:?}", l.n));
                    }
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} invalid word(s):\n{}",
        bad.len(),
        bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// The ladder, printed. Run with `--nocapture` to see where the compiler is.
#[test]
#[ignore = "blocked: needs tests/levels/ (the level ladder) not in the public repository"]
fn report_the_ladder() {
    println!("METRIC ladder_levels {}", levels().len());
    println!("\nSANSOS conformance ladder — ten gates\n");
    for l in levels() {
        let progs = level_programs(&l);
        let aks: usize = progs.iter().map(|(_, s)| aksharas(s).count()).sum();
        println!("  {:>2}. {:<12} {:<52}", l.n, l.name, l.english);
        println!(
            "      {} program(s), {aks} akṣaras · perf: {}",
            progs.len(),
            l.perf
        );
        println!("      gate: {}", l.gate);
    }
    println!("\n  status: lexically valid at every level; compiled/runs/perf await stage B.");
}
