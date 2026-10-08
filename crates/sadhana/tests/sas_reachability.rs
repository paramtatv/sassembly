//! **Is a `.sas` in this tree SOURCE, or is it a corpse?** `W-313`'s `Next:`,
//! answered by measurement.
//!
//! `W-313` split `repertoire_violations` into 13,442 the lexer refuses and
//! 1,724 a text directive carries, and its `Next:` named the one part of the
//! 13,442 that is **not** the `textapp` decision: 82 characters in four
//! `.sas` drafts under `crates/sadhana/src/t1/tests/` — `ident`, `lex`,
//! `parse`, `repertoire` — written in Latin routine names (`in_repertoire`,
//! `is_punctuation`, `validate`) and ASCII digits (`32`, `9`, `10`). The
//! question it posed is a measurement and not a decision: **does any `.rs`,
//! any `tools/` script or any `include_str!` name these four paths?**
//!
//! `W-312` built the `.t1` form of this instrument
//! (`crates/sadhana-t1/tests/t1_unreached_modules.rs`, REACHED / ENTERED /
//! DEAD). A `.sas` has no imports and declares no module, so that reading does
//! not transfer: a `.sas` is reached because some **reader** opens it, and the
//! readers are a walk over a directory or a path written out in full.
//!
//! # The instrument has FOUR states because the truth does
//!
//! | state | a reader walks its directory | its path or name is written down |
//! | --- | --- | --- |
//! | `WALKED` | yes — it is fed to the toolchain by the walk | — |
//! | `NAMED` | no | yes |
//! | `DEAD` | no | no |
//! | `AMBIGUOUS` | no | its path is not, and its BASENAME is shared |
//!
//! The fourth state is not bookkeeping. Seven `drafts/*.sas` share a basename
//! with a `spec/` namesake — `pty`, `compositor`, `network`, `powerbox`,
//! `boot-counter`, `virtio-gpu`, `virtio-sound` — and `tools/check-pty.sh`
//! names `pty.sas`. A three-state instrument must either call those seven
//! `NAMED` (crediting the draft with its namesake's readers) or `DEAD`
//! (asserting absence it cannot see). Both are claims this file cannot make,
//! so it makes neither and says how many it could not attribute. The four
//! files `W-313` asks about all have basenames unique in the tree, so the
//! ambiguity never touches the finding.
//!
//! # The answer
//!
//! `ident.sas`, `lex.sas` and `parse.sas` are named by **nothing** in
//! `crates/**/*.rs` or `tools/`. `repertoire.sas` is named three times and
//! every site asserts that the lexer **refuses** it — twice by
//! `repertoire_text_directive_boundary.rs` (the control that must stay red)
//! and once in `repertoire_census.rs`'s prose. Not one of the four is read as
//! SOURCE by anything. They join the `textapp` question rather than being
//! deleted here, which is `W-313`'s own instruction.
//!
//! Of 109 `.sas` in the tree: 82 `WALKED`, 3 `NAMED`, 17 `DEAD`, 7
//! `AMBIGUOUS`. Sixteen of the seventeen `DEAD` are `drafts/` and
//! `tests/benchmarks.sas`, and `drafts/README` says a draft is kept on
//! purpose — DEAD there is the intent, not a defect. The finding is the four.
//!
//! # 82 and 10 are the same measurement in different units
//!
//! [`the_four_sas_under_crates_are_still_refused_by_the_real_lexer`] first
//! reported **ten** where `W-313` reported **82**, and the gap was the unit,
//! not the tree: `lex.rs:622-662` pushes **one `LexError` per WORD**, naming
//! the first offending akṣara and putting the rest in the reason as
//! `(N more in this word)`, while the census counts CHARACTERS. Reading the
//! two side by side would have said the refusals fell 88% on a tree where
//! nothing moved. Summed as `1 + N` the lexer answers **82 akṣaras in 10
//! words** — `W-313`'s figure to the character, from the real lexer, through a
//! third loader. Both numbers are printed, so the subtraction cannot be made
//! again by accident.
//!
//! # Why the reader roots are a pin AND a scan
//!
//! `WALKED` rests on a claim — every walker that filters on the `sas`
//! extension is rooted under `spec/` — and a claim that nothing can falsify is
//! a comment. [`every_sas_extension_filter_is_rooted_under_spec`] re-derives
//! it from the tree on every run: it finds every `"sas"` literal in
//! `crates/**/*.rs` and `tools/`, and demands that each one either sit within
//! thirty lines of a `spec` path or be one of four sites declared NOT to be a
//! directory walk — each of which is asserted to still exist. Add a walker
//! over `crates/` and this file reds before the census silently changes
//! meaning.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use sadhana::lex::{LexError, lex};

/// This file, excluded from its own scan — `W-312`'s lesson, which cost a
/// pinned row when a doc comment was read as a call site. The exclusion is
/// asserted to be load-bearing in
/// [`the_scan_excludes_itself_and_that_exclusion_is_load_bearing`].
const SELF: &str = "crates/sadhana/tests/sas_reachability.rs";

/// The four `W-313` asks about.
const THE_FOUR: &[&str] = &[
    "crates/sadhana/src/t1/tests/ident.sas",
    "crates/sadhana/src/t1/tests/lex.sas",
    "crates/sadhana/src/t1/tests/parse.sas",
    "crates/sadhana/src/t1/tests/repertoire.sas",
];

/// Every directory a reader walks for `.sas`, as a path PREFIX.
///
/// One entry, because every walker in the tree resolves to it —
/// `parse_shape.rs:84` walks `spec/` recursively, `golden.rs:45` and
/// `coverage.rs:63` walk `spec/golden`, `metrics/src/main.rs:1564`,
/// `application.rs:210`, `traps.rs:130`, `grammar_t1.rs:856`,
/// `trap_vector_alignment.rs:90`, `paradigm_t0.rs:111` and
/// `paradigm_convention.rs:95` walk `spec/`. Kept as a list rather than a
/// constant string so that adding a second root is an edit here and not a
/// rewrite, and checked against the tree by
/// [`every_sas_extension_filter_is_rooted_under_spec`].
const READER_ROOTS: &[&str] = &["spec/"];

/// The `"sas"` literals in the tree that are NOT a directory walk, with why.
///
/// Every one is asserted to still contain the literal, so an exemption that
/// outlives its site reds instead of quietly forgiving a real walker.
const SAS_LITERAL_IS_NOT_A_WALK: &[(&str, &str)] = &[
    (
        "crates/sanskrit-text/src/slp1.rs",
        "SLP1 transliteration output — `enc(शस्) == \"sas\"`, not a file extension",
    ),
    (
        "crates/sanskrit-text/tests/repertoire_census.rs",
        "GOVERNED_EXTENSIONS — doc 15 §4's population, walked over the WHOLE tree by design",
    ),
    (
        "crates/sadhana/tests/repertoire_text_directive_boundary.rs",
        "GOVERNED_EXTENSIONS again, the second reading of the same population",
    ),
    (SELF, "this file's own population and prose"),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// This file's OWN loader. `W-312`'s rule: a test loader is part of the test,
/// so it does not borrow one.
///
/// `--others --exclude-standard` as well as the index, and that is not
/// convenience. The first cut read `git ls-files` alone, so THIS FILE — added
/// and not yet committed — was outside its own population, and
/// [`the_scan_excludes_itself_and_that_exclusion_is_load_bearing`] passed by
/// asserting that an absent file was absent. A census that cannot see an
/// untracked `.sas` also reports a draft dropped into `drafts/` as DEAD
/// because it cannot see it at all, which is the same lie in the other
/// direction. `--exclude-standard` keeps `target/` out.
///
/// Panics rather than returning an empty list — "no `.sas` is reachable"
/// reported because `git` did not run is the worst possible answer here.
fn tracked(root: &Path) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .expect("git ls-files must run — without it NOTHING was measured");
    assert!(
        out.status.success(),
        "git ls-files failed ({}) — NOTHING was measured",
        out.status
    );
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Whether `needle` occurs in `text` at a position no identifier runs into.
///
/// The preceding character must not be alphanumeric, `_` or `-`. Without it
/// `ident.sas` matches `xident.sas` and `parse.sas` matches `reparse.sas` —
/// the substring trap `W-312` caught in `calls_into` and `W-313` caught in
/// `कआस्की`. Devanagari has no `\b` either, but these needles are ASCII file
/// names, so a character class is the honest form of the same guard.
fn names(text: &str, needle: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut from = 0usize;
    while let Some(at) = text[from..].find(needle) {
        let at = from + at;
        let prev_ok = at == 0 || {
            let c = bytes[at - 1] as char;
            !(c.is_ascii_alphanumeric() || c == '_' || c == '-')
        };
        if prev_ok {
            out.push(text[..at].matches('\n').count() + 1);
        }
        from = at + 1;
    }
    out
}

/// Everything that could name a `.sas`: `crates/**/*.rs` and all of `tools/`.
///
/// NOT the ledger, NOT `BACKLOG*.tsv`, NOT `docs/`. Those describe the tree;
/// they do not read it, and `.loop/STATE.md` alone names all four of the files
/// under measurement. A census whose population includes its own minutes
/// cannot find a corpse.
fn readers(root: &Path, tracked: &[String]) -> Vec<(String, String)> {
    tracked
        .iter()
        .filter(|p| (p.starts_with("crates/") && p.ends_with(".rs")) || p.starts_with("tools/"))
        .filter(|p| p.as_str() != SELF)
        .filter_map(|p| {
            std::fs::read_to_string(root.join(p))
                .ok()
                .map(|t| (p.clone(), t))
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum State {
    Walked,
    Named,
    Dead,
    Ambiguous,
}

impl State {
    fn label(self) -> &'static str {
        match self {
            State::Walked => "WALKED",
            State::Named => "NAMED",
            State::Dead => "DEAD",
            State::Ambiguous => "AMBIGUOUS",
        }
    }
}

/// The census: every tracked `.sas`, its state, and where it is named.
fn census(root: &Path) -> BTreeMap<String, (State, Vec<String>)> {
    let tracked = tracked(root);
    let sources: Vec<&String> = tracked.iter().filter(|p| p.ends_with(".sas")).collect();
    assert!(
        sources.len() >= 100,
        "only {} `.sas` tracked — the walk is broken, not the tree",
        sources.len()
    );
    let readers = readers(root, &tracked);
    assert!(
        readers.len() >= 300,
        "only {} reader files — the walk is broken",
        readers.len()
    );

    let mut shared: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &sources {
        *shared.entry(basename(p)).or_default() += 1;
    }

    let mut out = BTreeMap::new();
    for p in sources {
        if READER_ROOTS.iter().any(|r| p.starts_with(r)) {
            out.insert(p.clone(), (State::Walked, Vec::new()));
            continue;
        }
        let mut sites: Vec<String> = Vec::new();
        for (rp, text) in &readers {
            for line in names(text, p) {
                sites.push(format!("{rp}:{line}"));
            }
        }
        if !sites.is_empty() {
            out.insert(p.clone(), (State::Named, sites));
            continue;
        }
        if shared[basename(p)] > 1 {
            out.insert(p.clone(), (State::Ambiguous, Vec::new()));
            continue;
        }
        for (rp, text) in &readers {
            for line in names(text, basename(p)) {
                sites.push(format!("{rp}:{line}"));
            }
        }
        let state = if sites.is_empty() {
            State::Dead
        } else {
            State::Named
        };
        out.insert(p.clone(), (state, sites));
    }
    out
}

#[test]
#[ignore = "blocked: needs the full development repository's .sas tree not in the public repository"]
fn the_sas_census_reports_four_states_and_the_four_drafts_are_not_source() {
    let root = root();
    let c = census(&root);

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (state, _) in c.values() {
        *counts.entry(state.label()).or_default() += 1;
    }
    println!("METRIC sas_files_total {}", c.len());
    for state in [State::Walked, State::Named, State::Dead, State::Ambiguous] {
        println!(
            "NOTE  sas_{:<10} {}",
            state.label().to_lowercase(),
            counts.get(state.label()).copied().unwrap_or(0)
        );
    }
    for (p, (state, sites)) in &c {
        if *state != State::Walked {
            println!("      {:<10} {p}  {}", state.label(), sites.join(" "));
        }
    }

    // THE ANSWER. Three named by nothing; one named only where it is refused.
    for p in THE_FOUR {
        let (state, sites) = c.get(*p).unwrap_or_else(|| panic!("{p} is tracked"));
        assert_ne!(
            *state,
            State::Walked,
            "{p} is under a reader root — the toolchain would be assembling a file the lexer refuses"
        );
        assert_ne!(
            *state,
            State::Ambiguous,
            "{p}'s basename stopped being unique; the finding can no longer be attributed"
        );
        if *p == "crates/sadhana/src/t1/tests/repertoire.sas" {
            assert_eq!(*state, State::Named, "{p}: {sites:?}");
            // Every site asserts a REFUSAL. Two hold the control in
            // `repertoire_text_directive_boundary.rs`; the third is
            // `repertoire_census.rs`'s prose about the same control.
            let files: BTreeSet<&str> = sites
                .iter()
                .map(|s| s.rsplit_once(':').expect("file:line").0)
                .collect();
            assert_eq!(
                files,
                BTreeSet::from([
                    "crates/sadhana/tests/repertoire_text_directive_boundary.rs",
                    "crates/sanskrit-text/tests/repertoire_census.rs",
                ]),
                "a new reader names repertoire.sas — check whether it ASSEMBLES it: {sites:?}"
            );
        } else {
            assert_eq!(
                *state,
                State::Dead,
                "{p} gained a reader: {sites:?} — it is still a file the lexer refuses"
            );
        }
    }

    // THE CASE THAT MUST STILL COME BACK REACHED. A detector that answered
    // DEAD for everything would satisfy every assertion above.
    let walked = counts.get("WALKED").copied().unwrap_or(0);
    assert!(
        walked >= 80,
        "only {walked} `.sas` WALKED — `spec/` stopped being a reader root and the census means nothing"
    );
    assert!(
        c.values().any(|(s, _)| *s == State::Named),
        "no `.sas` is NAMED — the naming scan found nothing and every DEAD above is an artefact"
    );
    // `drafts/segmenter.sas` is NAMED by `grammar_t1.rs:851`, which counts its
    // `०षोड्इ६४१न` as the reason the type-suffix census excludes `drafts/`.
    // `tests/defects.sas` is NAMED by `tools/run-defects.sh:5` — a comment, in
    // a script whose own body is a stub that prints three verdicts and reads
    // nothing. NAMED is therefore not a claim that anything ASSEMBLES it.
    assert_eq!(
        c.get("drafts/segmenter.sas").map(|(s, _)| *s),
        Some(State::Named),
        "the naming scan no longer reaches a draft it did reach"
    );
    assert_eq!(
        c.get("tests/defects.sas").map(|(s, _)| *s),
        Some(State::Named),
        "the naming scan no longer reaches tools/"
    );
    assert_eq!(
        c.get("tests/benchmarks.sas").map(|(s, _)| *s),
        Some(State::Dead),
        "tests/benchmarks.sas gained a reader"
    );
}

#[test]
fn the_word_boundary_guard_is_load_bearing() {
    // `ident.sas` inside `xident.sas` is a different file, and a bare
    // `text.contains` would credit the first with the second's reader.
    assert!(names("read(\"ident.sas\")", "ident.sas") == vec![1]);
    assert!(names("read(\"xident.sas\")", "ident.sas").is_empty());
    assert!(names("read(\"re-parse.sas\")", "parse.sas").is_empty());
    assert!(names("read(\"t1_parse.sas\")", "parse.sas").is_empty());
    // A path prefix is NOT an identifier character, so the full path still
    // matches when only the basename is sought.
    assert_eq!(names("x/ident.sas", "ident.sas"), vec![1]);
    // Two occurrences, two lines.
    assert_eq!(names("a lex.sas\nb lex.sas", "lex.sas"), vec![1, 2]);
}

#[test]
fn every_sas_extension_filter_is_rooted_under_spec() {
    let root = root();
    let tracked = tracked(&root);
    let mut checked = 0usize;
    let mut exempt_seen: BTreeSet<&str> = BTreeSet::new();

    for p in &tracked {
        if !((p.starts_with("crates/") && p.ends_with(".rs")) || p.starts_with("tools/")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(p)) else {
            continue;
        };
        if !text.contains("\"sas\"") {
            continue;
        }
        if let Some((f, _)) = SAS_LITERAL_IS_NOT_A_WALK.iter().find(|(f, _)| f == p) {
            exempt_seen.insert(f);
            continue;
        }
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !line.contains("\"sas\"") {
                continue;
            }
            checked += 1;
            // Thirty lines back, two forward. Deliberately a WINDOW and not a
            // parse: this file does not need to know Rust, only whether the
            // walk that produced the filter names `spec`. It is cruder than
            // reading the expression, which is what keeps it a second reading.
            let lo = i.saturating_sub(30);
            let hi = (i + 3).min(lines.len());
            let window = lines[lo..hi].join("\n");
            assert!(
                ["spec/", "spec\"", "spec)"]
                    .iter()
                    .any(|n| window.contains(n)),
                "{p}:{} filters on the `sas` extension and no `spec` path is within thirty \
                 lines — a walker over a new root would silently change what WALKED means",
                i + 1
            );
        }
    }

    println!("NOTE  sas_extension_filter_sites {checked}");
    assert!(
        checked >= 10,
        "only {checked} `sas` filter sites found — the scan is broken, not the tree"
    );
    // A stale exemption forgives a real walker. Each must still be a site.
    let declared: BTreeSet<&str> = SAS_LITERAL_IS_NOT_A_WALK.iter().map(|(f, _)| *f).collect();
    assert_eq!(
        exempt_seen, declared,
        "an exemption in SAS_LITERAL_IS_NOT_A_WALK no longer names a file carrying `\"sas\"`"
    );
}

#[test]
fn the_scan_excludes_itself_and_that_exclusion_is_load_bearing() {
    let root = root();
    let text = std::fs::read_to_string(root.join(SELF)).expect("this file is readable");
    // This file writes all four names in its prose and its constants. Were it
    // in its own population every one of them would come back NAMED by itself.
    for p in THE_FOUR {
        assert!(
            !names(&text, p).is_empty(),
            "{SELF} no longer names {p}; the self-exclusion has become a no-op"
        );
    }
    let tracked = tracked(&root);
    // NON-VACUOUS, and it was not: until the loader took `--others` this file
    // was untracked, so the exclusion below held because the file was not in
    // the list at all rather than because it was removed from it.
    assert!(
        tracked.iter().any(|p| p == SELF),
        "the loader cannot see {SELF} — every exclusion in this file is then vacuous"
    );
    assert!(
        !readers(&root, &tracked).iter().any(|(p, _)| p == SELF),
        "{SELF} is inside its own reader population"
    );
}

/// The lexer's repertoire complaints about one source, and ONLY those.
///
/// A `.sas` may fail to lex for reasons with nothing to do with R-15-1, and
/// counting those would let this file red on an unrelated defect while saying
/// "repertoire" — `W-313`'s rule, restated rather than shared.
fn repertoire_errors(src: &str) -> Vec<LexError> {
    lex(src)
        .err()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.reason.contains("outside the doc 15 repertoire"))
        .collect()
}

/// How many akṣaras one `LexError` stands for.
///
/// `lex.rs:622-662` pushes **one error per WORD**, names the first offending
/// akṣara in `aksara`, and puts the rest in the reason as
/// `(N more in this word)`. The count is therefore `1 + N`, and the only
/// channel the lexer offers for `N` is that sentence. Parsing a diagnostic is
/// brittle, so [`the_four_sas_under_crates_are_still_refused_by_the_real_lexer`]
/// asserts the suffix is REACHED — if `lex.rs` ever rewords it, this reds
/// instead of silently reporting one akṣara per word.
fn aksaras_in(e: &LexError) -> usize {
    let Some(rest) = e.reason.split_once(" (") else {
        return 1;
    };
    let Some(n) = rest.1.strip_suffix(" more in this word)") else {
        return 1;
    };
    1 + n.parse::<usize>().unwrap_or(0)
}

#[test]
fn the_four_sas_under_crates_are_still_refused_by_the_real_lexer() {
    let root = root();
    let (mut words, mut aksaras, mut with_suffix) = (0usize, 0usize, 0usize);
    for p in THE_FOUR {
        let src = std::fs::read_to_string(root.join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));
        let errors = repertoire_errors(&src);
        assert!(
            !errors.is_empty(),
            "{p} now lexes clean — `W-313`'s split must be re-read, and this file is no \
             longer a source the lexer refuses"
        );
        let a: usize = errors.iter().map(aksaras_in).sum();
        with_suffix += errors
            .iter()
            .filter(|e| e.reason.contains(" more in this word)"))
            .count();
        println!(
            "NOTE  sas_dead_refusal {p} words {} aksaras {a} first {} line {}",
            errors.len(),
            errors[0].aksara,
            errors[0].line
        );
        words += errors.len();
        aksaras += a;
    }
    println!("METRIC sas_unreached_refusal_words {words}");
    println!("METRIC sas_unreached_refusal_aksaras {aksaras}");

    // THE UNIT IS THE FINDING. `W-313` reported **82** over these four files
    // and this file reports ten — and neither is wrong, because `W-313`'s
    // census counts CHARACTERS while the lexer refuses WORDS. Reading the two
    // as one number would say the refusals had fallen by 88% on a tree where
    // nothing moved. Both are printed here so the next reader cannot make that
    // subtraction, and `aksaras` is the one comparable to the census.
    assert!(
        with_suffix > 0,
        "no error carried `(N more in this word)` — `aksaras_in` is returning 1 for \
         everything and the character count above is really a word count"
    );
    // Floors, not pins (the owner's 2026-09-13 ruling): what matters is that
    // both numbers come from a lexer run on this tree rather than a memory.
    assert!(
        words >= 4,
        "only {words} refusal words over four files — one stopped being refused"
    );
    assert!(
        aksaras > words,
        "aksaras ({aksaras}) is not above words ({words}) — the multi-akṣara arm is dead"
    );
    assert!(
        aksaras >= 50,
        "only {aksaras} refused akṣaras over the four — well below `W-313`'s 82 characters, \
         so either the files were edited or the repertoire rule moved"
    );
}
