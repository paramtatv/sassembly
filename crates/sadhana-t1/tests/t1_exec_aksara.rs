//! **`अक्षरकोश ॱ व्यञ्जन` IS EXECUTED HERE** — the हल् reader, run against
//! `spec/shiva-sutras.tsv` rather than against a second copy of the list.
//!
//! # Why this file exists at all, and why it is its own file
//!
//! `व्यञ्जन` was a stub with an exact, single reason: `phonology.rs:281` is
//! `pub const HAL: CharSet = char_set("ह", "ल्")`, derived at compile time from
//! `spec/shiva-sutras.tsv` (`phonology.rs:30`, `include_str!`) — and that file
//! had **no name** in `crates/sadhana/src/t1/anita.rs`'s `TABLES`, so `समावेशः`
//! could not reach it. `("शिवसूत्रकोशः", "shiva-sutras.tsv")` is now
//! `anita.rs:197`, and this file is the evidence that the name was the whole
//! blocker.
//!
//! It is a NEW file because `tests/t1_execution.rs` is 3000 lines that five
//! agents edited in one round, and the merge cost more than the code. Every
//! helper it needs — `mutate`, `octets`, `spec_root` — is copied here rather
//! than shared, deliberately.
//!
//! # The one rule every assertion below obeys
//!
//! **The routine is checked against the FILE, never against a list written
//! here.** [`hal_from_the_file`] re-derives हल् in Rust with its own
//! `lines`/`split('\t')` walk — start at the first non-marker `ह`, run forward,
//! drop markers, stop at `ल्` — and *that* is what `व्यञ्जन` is graded on. A
//! constant `&["क", "ख", …]` in this file would be the second transcription of
//! an authored table that the whole design exists to prevent, and it would
//! agree with a `व्यञ्जन` that had the same 33 baked into it.
//!
//! The count is asserted (33) but the count is a *cross-check*, not the
//! oracle — `the_sutras_yield_exactly_the_thirty_three_consonants` says why a
//! range test would answer 37.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

/// Building a substitute `spec/` root — shared with the other binary that does
/// it, because two copies of this routine carried the same defect.
mod spec_fixture;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// Replace `from` with `to` in `text`, and **fail if `from` is not there
/// exactly once.**
///
/// Copied from `t1_execution.rs` for the reason the header gives. This is the
/// whole guard on the mutation tests: a mutation applied to text that does not
/// contain it changes nothing, and the test it guards then passes for the wrong
/// reason — the same defect, one level up, as a ratchet that counts
/// declarations.
fn mutate(text: &str, from: &str, to: &str) -> String {
    let n = text.matches(from).count();
    assert_eq!(
        n, 1,
        "the mutation `{from}` -> `{to}` matches {n} places in the source; \
         a mutation test is only evidence when it changes exactly one"
    );
    text.replace(from, to)
}

/// `sanskrit_text.t1` **with `encode.t1`**, because `व्यञ्जन` reaches the five
/// TSV helpers through `आयातः सङ्केतन ।`.
///
/// `समयोजन ॱ भेदाङ्कः` reads `spec/relocations-riscv64.tsv` through the same
/// five (`samyojana.t1:501-531`), so this is the crate's one table reader and
/// not a second one. The two modules share no routine name, which matters
/// because the interpreter's arity table is global.
fn load(sanskrit_text: &str) -> Interpreter {
    let encode = source("encode.t1");
    let lex = source("lex.t1");
    Interpreter::load(
        &[
            ("lex.t1", lex.as_str()),
            ("encode.t1", encode.as_str()),
            ("sanskrit_text.t1", sanskrit_text),
        ],
        &spec_root(),
    )
    .expect("encode.t1 and sanskrit_text.t1 load together")
}

fn unmutated() -> Interpreter {
    load(&source("sanskrit_text.t1"))
}

/// A spec root that is `spec/` with one table replaced.
///
/// The whole directory is populated because `encode.t1` — which `व्यञ्जन`
/// reaches its five TSV helpers through — carries its own `समावेशः` of
/// `registers-riscv64.tsv`, and the embed is resolved for EVERY module at load
/// time, not lazily when a routine runs. A root holding only the sūtras fails
/// to load with `कोष्ठकोशः … could not be read`.
///
/// # THE DIRECTORY NAME IS PER-CALL AND MUST NOT BECOME CONTENT-DERIVED
///
/// It used to be `aksara-spec-{FNV hash of (content, file)}`, which looks like
/// a harmless cache — two callers wanting the same substitution share one
/// directory and the copy is paid once. **It is a data race, and it failed the
/// gate.** The body is `remove_dir_all` then `create_dir_all` then a copy loop,
/// and that sequence is not atomic while cargo runs tests on parallel threads.
/// Two callers computing the SAME name — the unmutated control and a mutation
/// that happens to reproduce the original content are the natural pair — delete
/// each other's root mid-read, and the victim is whichever test is loading at
/// that instant:
///
/// ```text
/// encode.t1: embed: `कोष्ठकोशः` names `registers-riscv64.tsv`,
/// which could not be read: No such file or directory
/// ```
///
/// `registers-riscv64.tsv` is copied by the loop below, so its ABSENCE means
/// the directory was removed underneath a reader rather than never populated.
/// It does not reproduce under `--test t1_exec_aksara` alone, at any
/// `--test-threads`; it needs the whole crate running, which is exactly what
/// makes it dangerous — it fails the gate intermittently and looks like a real
/// defect in whatever test drew the short straw.
///
/// So the name carries a process id and an atomic counter and is unique to the
/// CALL. Nothing is shared, so nothing can be removed underneath anything.
/// **Do not reintroduce the hash as a caching optimisation**: the cost it saves
/// is already gone, because the files are HARD-LINKED rather than copied and
/// the substituted one is written fresh — the link is why a per-call root is
/// affordable at ~50 calls against a 4.9 MB `spec/`.
fn spec_root_with(file: &str, content: &str) -> PathBuf {
    let dir = spec_fixture::unique_root("aksara-spec");
    std::fs::create_dir_all(&dir).expect("temp spec root");
    for entry in std::fs::read_dir(spec_root()).expect("spec/ is readable") {
        let p = entry.expect("entry").path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().expect("file name").to_owned();
        // The substituted table is WRITTEN, never linked — a hard link would
        // make `fs::write` below edit the repository's own `spec/` file
        // through the shared inode.
        if name == std::ffi::OsStr::new(file) {
            continue;
        }
        let to = dir.join(name);
        // Hard link when the temp dir and the repo share a filesystem, which is
        // the usual case and makes this loop nearly free; copy when they do not.
        if std::fs::hard_link(&p, &to).is_err() {
            std::fs::copy(&p, &to).expect("copy spec file");
        }
    }
    spec_fixture::write_substituted(&dir, file, content);
    dir
}

/// **The substituted table must not be the repository's table.** SAS-017.
///
/// `spec_root_with` fills its root with HARD LINKS and then writes one file. If
/// that one file is ever reached as a link, `fs::write` truncates
/// `spec/` itself, and every reader in the workspace — this binary and the dozen
/// running beside it — sees an empty table until something restores it. The
/// failure that cost a day looked nothing like this: `no_pict` = 0 in a
/// whole-workspace run, green everywhere else.
///
/// **WHAT THIS TEST PROVES AND WHAT IT DOES NOT, said because the first version
/// of this margin overclaimed.** It checks the HAPPY PATH: the substitution
/// takes, the result is a fresh file rather than a link, and two calls get two
/// roots — so `spec_fixture::unique_root` is doing its job.
///
/// It does NOT prove the unlink works. It starts from a FRESH directory, where
/// the substituted path was never a link, so the bug's PRECONDITION is absent
/// and this would pass with the defect present — in every run where a collision
/// did not happen to occur, which is about 249 in 250.
///
/// The proof is `spec_fixture::\
/// writing_a_substituted_table_never_edits_the_file_it_was_linked_from`, which
/// CONSTRUCTS the link first. Verified by deleting the unlink: that test reds and
/// names the cause, and this one stays green.
#[test]
fn a_substituted_table_is_never_the_repositorys_own_file() {
    use std::os::unix::fs::MetadataExt;

    let real = spec_root().join("extended-pictographic.tsv");
    let before = std::fs::metadata(&real).expect("the real table").len();
    assert!(
        before > 0,
        "spec/extended-pictographic.tsv is already empty"
    );

    // Twice, because the bug needed a REUSED root: two calls must not land on
    // one directory either.
    let mut seen = Vec::new();
    for _ in 0..2 {
        let root = spec_root_with("extended-pictographic.tsv", "");
        let sub = root.join("extended-pictographic.tsv");
        let m = std::fs::metadata(&sub).expect("the substituted table");
        assert_eq!(m.len(), 0, "the substitution did not take");
        assert_ne!(
            m.ino(),
            std::fs::metadata(&real).expect("the real table").ino(),
            "the substituted table SHARES AN INODE with spec/\
             extended-pictographic.tsv — writing it edits the repository"
        );
        assert_eq!(
            m.nlink(),
            1,
            "the substituted table has {} links; it should be a fresh file",
            m.nlink()
        );
        seen.push(root);
    }
    assert_ne!(seen[0], seen[1], "two calls returned the SAME root");

    assert_eq!(
        std::fs::metadata(&real).expect("the real table").len(),
        before,
        "spec/extended-pictographic.tsv changed size while a substituted copy \
         was written — the write went through a link"
    );
}

fn spec_root_with_sutras(sutras: &str) -> PathBuf {
    spec_root_with("shiva-sutras.tsv", sutras)
}

/// `sanskrit_text.t1` and `encode.t1` loaded against a spec root whose
/// `shiva-sutras.tsv` is `sutras`.
fn load_with_sutras(sanskrit_text: &str, sutras: &str) -> Interpreter {
    let encode = source("encode.t1");
    let lex = source("lex.t1");
    Interpreter::load(
        &[
            ("lex.t1", lex.as_str()),
            ("encode.t1", encode.as_str()),
            ("sanskrit_text.t1", sanskrit_text),
        ],
        &spec_root_with_sutras(sutras),
    )
    .expect("the modules load against the substituted spec root")
}

/// `व्यञ्जन aksara` — `true`, `false`, or the reason it would not run.
///
/// **Matched on `Value::Bool` and not on `as_int`.** `व्यञ्जन` is declared
/// `ददाति बूल` and the interpreter answers `Value::Bool`, for which `as_int()`
/// is `None` — so `v.as_int() == Some(1)` is `false` for TRUE as well as for
/// false, and every assertion in this file would have passed the moment the
/// routine returned anything at all. A `बूल` that arrives as anything else is
/// an error here rather than a silent `false`.
fn consonant(it: &mut Interpreter, aksara: &str) -> Result<bool, String> {
    match it.call("अक्षरकोशॱव्यञ्जन", vec![octets(aksara)], 20_000_000)
    {
        Err(e) => Err(e.reason),
        Ok(Value::Bool(b)) => Ok(b),
        Ok(other) => Err(format!("व्यञ्जन answered {other:?}, which is not a बूल")),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// THE ORACLE — `spec/shiva-sutras.tsv`, read here by Rust, independently.
// ─────────────────────────────────────────────────────────────────────────

/// One row of the sūtras: the Devanagari form, and whether it is an anubandha.
///
/// Deliberately NOT `phonology::sequence()`. Calling the Rust original would
/// make this a comparison of two ports of one function; reading the file with
/// `lines`/`split` makes it a comparison of the T1 routine with the AUTHORITY.
fn sutra_rows() -> Vec<(String, bool)> {
    let text = std::fs::read_to_string(spec_root().join("shiva-sutras.tsv"))
        .expect("spec/shiva-sutras.tsv exists");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("line\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 5).then(|| (f[2].to_string(), f[4] == "it"))
        })
        .collect()
}

/// हल् — every phoneme from the first `ह` to the marker `ल्`, markers dropped.
///
/// Pāṇini's rule, applied here rather than quoted: start at a phoneme, run
/// forward, skip any anubandha that is not the one asked for, stop at the one
/// that is. `ह` occurs in sūtra 5 AND sūtra 14 and the FIRST is taken — the
/// second would yield a set of one.
///
/// **The run is DEDUPLICATED, and `ह` is why.** Walking the rows yields 34
/// entries, not 33, because sūtra 14 spells `ह` a second time just before the
/// closing `ल्` — the file's own header says that repetition is deliberate and
/// is what lets हल् cover the consonants while अच् stops short of them.
/// `phonology.rs` never notices because `char_set` builds a 128-bit SET and a
/// bit set twice is set once; a `Vec` has to be told.
fn hal_from_the_file() -> Vec<String> {
    let rows = sutra_rows();
    let from = rows
        .iter()
        .position(|(d, marker)| !marker && d == "ह")
        .expect("ह is a phoneme of the sūtras");

    let mut out: Vec<String> = Vec::new();
    for (d, marker) in &rows[from..] {
        if *marker {
            if d == "ल्" {
                return out;
            }
            continue;
        }
        if !out.contains(d) {
            out.push(d.clone());
        }
    }
    panic!("the marker ल् never arrived — the file is not the one हल् is defined over");
}

/// अच् — the vowels, by the same rule: `ह` becomes `अ` and `ल्` becomes `च्`.
///
/// These are what must answer FALSE. Derived rather than listed for exactly the
/// reason हल् is.
fn ac_from_the_file() -> Vec<String> {
    let rows = sutra_rows();
    let from = rows
        .iter()
        .position(|(d, marker)| !marker && d == "अ")
        .expect("अ is a phoneme of the sūtras");

    let mut out = Vec::new();
    for (d, marker) in &rows[from..] {
        if *marker {
            if d == "च्" {
                return out;
            }
            continue;
        }
        out.push(d.clone());
    }
    panic!("the marker च् never arrived");
}

// ─────────────────────────────────────────────────────────────────────────
// What the file says, before anything is asked of the T1.
// ─────────────────────────────────────────────────────────────────────────

/// **The oracle is not vacuous, and a range test would be wrong by four.**
///
/// Asserted first because every test below is only as good as
/// [`hal_from_the_file`]. If the walk returned an empty set, or the whole file,
/// every membership assertion would still "pass".
#[test]
fn the_sutras_yield_exactly_the_thirty_three_consonants() {
    let hal = hal_from_the_file();
    assert_eq!(
        hal.len(),
        33,
        "हल् came out as {} sounds, not 33: {hal:?}",
        hal.len()
    );

    // ह is in sūtra 5 and sūtra 14 and the FIRST is taken, so it appears once.
    assert_eq!(hal.iter().filter(|d| *d == "ह").count(), 1);

    // Every one is a single code point — which is what makes व्यञ्जन's prefix
    // test equivalent to `aksara.chars().next()`.
    for d in &hal {
        assert_eq!(d.chars().count(), 1, "{d} is not one code point");
    }

    // **THE FOUR THAT PROVE THE SET IS NOT A RANGE.** क is U+0915 and ह is
    // U+0939; these four are inside that span and are NOT consonants of the
    // sūtras. An `аधिकम् U+0914 / न्यूनम् U+093A` test would answer 37.
    for absent in ["ऩ", "ऱ", "ळ", "ऴ"] {
        assert!(
            !hal.contains(&absent.to_string()),
            "{absent} is inside U+0915..U+0939 but is not in हल् — if the \
             oracle thinks it is, the oracle is a range test"
        );
        let c = absent.chars().next().unwrap() as u32;
        assert!(
            (0x0915..=0x0939).contains(&c),
            "{absent} is not in the span"
        );
    }

    // And the vowels are a disjoint set, so "a vowel answers false" is a real
    // question and not a tautology.
    let ac = ac_from_the_file();
    assert_eq!(ac.len(), 9, "अच् came out as {} sounds: {ac:?}", ac.len());
    for v in &ac {
        assert!(!hal.contains(v), "{v} is in both अच् and हल्");
    }
}

// ─────────────────────────────────────────────────────────────────────────
// THE ROUTINE, AGAINST THE FILE.
// ─────────────────────────────────────────────────────────────────────────

/// **Every consonant `spec/shiva-sutras.tsv` lists answers `सत्यम्`, and every
/// vowel answers `असत्यम्`.**
///
/// This is `D-002i2`'s acceptance. The set on the left comes from the file; the
/// answers on the right come from a T1 body that read the same file through
/// `समावेशः आरभ्य शिवसूत्रकोशः समाप्तम्`. Neither side carries a list.
#[test]
fn every_consonant_of_the_sutras_answers_true_and_every_vowel_false() {
    let mut it = unmutated();
    let mut wrong = Vec::new();

    for d in hal_from_the_file() {
        match consonant(&mut it, &d) {
            Ok(true) => {}
            Ok(false) => wrong.push(format!("व्यञ्जन({d}) said false; हल् lists it")),
            Err(e) => wrong.push(format!("व्यञ्जन({d}) did not run: {e}")),
        }
    }
    for d in ac_from_the_file() {
        match consonant(&mut it, &d) {
            Ok(false) => {}
            Ok(true) => wrong.push(format!("व्यञ्जन({d}) said true; it is a vowel of अच्")),
            Err(e) => wrong.push(format!("व्यञ्जन({d}) did not run: {e}")),
        }
    }

    assert!(
        wrong.is_empty(),
        "{} disagreement(s) between अक्षरकोश ॱ व्यञ्जन and spec/shiva-sutras.tsv:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// The four code points that make the set not a range, asked of the T1 itself.
///
/// `the_sutras_yield_exactly_the_thirty_three_consonants` proves the FILE
/// excludes them. This proves the ROUTINE does — which a body that tested
/// `अधिकम् २३२४ / न्यूनम् २३६२` would fail, and which is the specific defect
/// the stub's note said nothing arithmetic could avoid.
#[test]
fn the_four_letters_inside_the_span_that_are_not_consonants_answer_false() {
    let mut it = unmutated();
    for absent in ["ऩ", "ऱ", "ळ", "ऴ"] {
        assert_eq!(
            consonant(&mut it, absent),
            Ok(false),
            "व्यञ्जन({absent}) said true — U+{:04X} is inside क..ह but हल् does \
             not list it, so this body is a range test",
            absent.chars().next().unwrap() as u32
        );
    }
}

/// `is_consonant` asks after the FIRST character, not the whole akṣara.
///
/// `phonology.rs:301` is `aksara.chars().next().is_some_and(|c| holds(HAL, c))`,
/// so a multi-code-point cluster is classified by its head: क्ष is क + virāma +
/// ष and IS a consonant, मे is म + a mātrā and IS one, and ऐ followed by
/// anything is not.
#[test]
fn a_cluster_is_classified_by_the_sound_it_begins_with() {
    let mut it = unmutated();
    for yes in ["क्ष", "मे", "त्र", "ज्ञ", "ह्य", "स्"] {
        assert_eq!(consonant(&mut it, yes), Ok(true), "व्यञ्जन({yes}) said false");
    }
    for no in ["अ", "आ", "ऐ", "ॐ", "औत्"] {
        assert_eq!(consonant(&mut it, no), Ok(false), "व्यञ्जन({no}) said true");
    }
}

/// Nothing that is not a sound of the sūtras answers true — including the
/// empty akṣara, ASCII, and the anubandhas' own virāma-final spellings.
///
/// **The markers are the interesting half.** `ट्`, `ण्`, `म्` and the rest are
/// written in the file as `it` rows, and a walk that forgot to drop them would
/// answer true for the six-octet `ल्` — while `ल` alone, the SOUND, is a
/// consonant and must still answer true.
#[test]
fn what_the_sutras_do_not_list_answers_false() {
    let mut it = unmutated();

    assert_eq!(consonant(&mut it, ""), Ok(false), "the empty akṣara");
    for other in ["1", "x", " ", "।", "०", "\u{094D}"] {
        assert_eq!(consonant(&mut it, other), Ok(false), "व्यञ्जन({other:?})");
    }

    // ल is a sound of हल् ; ल् is the anubandha that CLOSES it.
    assert_eq!(consonant(&mut it, "ल"), Ok(true), "ल is sūtra 6's phoneme");
}

// ─────────────────────────────────────────────────────────────────────────
// MUTATION. Each breaks one comparison and must change an answer.
// ─────────────────────────────────────────────────────────────────────────

/// `(what the mutation does, from, to)`.
///
/// Every one is a single comparison in `sanskrit_text.t1`, applied by
/// [`mutate`], which fails unless the pattern is in the source **exactly
/// once**. A survivor would be a mutation the tests above cannot see, which
/// means they do not check that behaviour.
const HAL_MUTANTS: &[(&str, &str, &str)] = &[
    (
        "the walk starts at the wrong octet, so ह never opens the range and \
         हल् is empty",
        "यदि पाठ्यम् अङ्कः आरभ्य आदिः योगः २ समाप्तम् अन्तः असमम् १८५ आदि",
        "यदि पाठ्यम् अङ्कः आरभ्य आदिः योगः २ समाप्तम् अन्तः असमम् १८६ आदि",
    ),
    (
        "the closing anubandha ल् is never recognised, so the range runs off \
         the end of the table",
        "यदि पाठ्यम् अङ्कः आरभ्य आदिः योगः ५ समाप्तम् अन्तः असमम् १४१ आदि",
        "यदि पाठ्यम् अङ्कः आरभ्य आदिः योगः ५ समाप्तम् अन्तः असमम् १४२ आदि",
    ),
    (
        "the `it` column stops being read, so every anubandha counts as a \
         sound and ल् is classified a consonant",
        "यदि पाठ्यम् अङ्कः आदिः अन्तः असमम् १०५ आदि",
        "यदि पाठ्यम् अङ्कः आदिः अन्तः असमम् १०६ आदि",
    ),
    (
        "the phoneme test is inverted — markers become sounds and sounds \
         become markers",
        "चरः चिह्नम् ॱॱ बूल भवति इत्संज्ञा पाठ्यम् भेदादिः भेदसीमा ।",
        "चरः चिह्नम् ॱॱ बूल भवति इत्संज्ञा पाठ्यम् वर्णादिः वर्णसीमा ।",
    ),
    (
        "the devanagari column is read as the iast column, so no row spells a \
         Devanagari sound at all",
        "चरः वर्णादिः ॱॱ अ६४ भवति सङ्केतनॱक्षेत्रारम्भः पाठ्यम् आरम्भः सीमा २ ।",
        "चरः वर्णादिः ॱॱ अ६४ भवति सङ्केतनॱक्षेत्रारम्भः पाठ्यम् आरम्भः सीमा ३ ।",
    ),
    (
        "the prefix walk compares nothing, so every row matches every akṣara \
         and the first sound of हल् claims them all",
        "यदि अक्षर अङ्कः सरणम् अन्तः असमम् पाठ्यम् अङ्कः आरभ्य आदिः योगः सरणम् समाप्तम् अन्तः आदि",
        "यदि अक्षर अङ्कः सरणम् अन्तः समम् पाठ्यम् अङ्कः आरभ्य आदिः योगः सरणम् समाप्तम् अन्तः आदि",
    ),
    (
        "an empty phoneme field stops being refused",
        "यदि विस्तारः समम् ० आदि",
        "यदि विस्तारः समम् ९९९ आदि",
    ),
    (
        "the range opens at the SECOND ह — sūtra 14 — which yields a set of one",
        "यदि प्रवृत्तम् समम् असत्यम् आदि",
        "यदि प्रवृत्तम् समम् सत्यम् आदि",
    ),
];

// ─────────────────────────────────────────────────────────────────────────
// THE PROBES — four tables, because the real one cannot exercise everything.
// ─────────────────────────────────────────────────────────────────────────

/// The real sūtras, plus three variants that make an unexercised rule visible.
///
/// **Four mutants survived a differential run against `spec/shiva-sutras.tsv`
/// alone, and none of them was a bad mutation — each broke a rule the real
/// table never puts a foot on.** `ल्` is the LAST row of the file, so a walk
/// that never recognises the closing anubandha stops at end-of-input with the
/// same answers; no row is marked `it` while spelling a sound that is not
/// otherwise in हल्, so ignoring the `kind` column changes nothing; and no
/// phoneme field is empty, so the guard against one is never reached.
///
/// A mutation that survives is a rule the tests do not check. The answer is not
/// to drop the mutation — it is to hand the reader a table that asks the
/// question, which is what these three do.
fn probe_tables() -> Vec<(&'static str, String)> {
    let real = std::fs::read_to_string(spec_root().join("shiva-sutras.tsv"))
        .expect("spec/shiva-sutras.tsv exists");

    // (1) A PHONEME AFTER THE CLOSING MARKER. हल् must stop at ल् , not run to
    // the end of the table. ॹ (U+0979) is outside the Devanagari letters the
    // sūtras use and is in no pratyāhāra.
    let mut past_the_stop = real.clone();
    if !past_the_stop.ends_with('\n') {
        past_the_stop.push('\n');
    }
    past_the_stop.push_str("15\t1\tॹ\tzz\tphoneme\n");

    // (2) A ROW MARKED `it` WHOSE SPELLING IS NOT A SOUND OF हल् . The file's
    // header: "the last entry of each line is the ANUBANDHA — a boundary sign,
    // not a sound." A reader that ignores the `kind` column would make ॺ a
    // consonant. Placed INSIDE the range, between ह and ल् .
    let marked_not_a_sound = real.replace(
        "5\t2\tय\tya\tphoneme",
        "5\t2\tय\tya\tphoneme\n5\t9\tॺ\tzz\tit",
    );
    assert_ne!(marked_not_a_sound, real, "sūtra 5's य row must be there");

    // (3) A PHONEME ROW WITH AN EMPTY SPELLING. An empty field is not a sound,
    // and a prefix walk over zero octets succeeds for EVERY akṣara — so
    // without the guard this row claims अ , the digits and everything else.
    let empty_spelling = real.replace(
        "5\t2\tय\tya\tphoneme",
        "5\t2\tय\tya\tphoneme\n5\t9\t\tzz\tphoneme",
    );
    assert_ne!(empty_spelling, real, "sūtra 5's य row must be there");

    vec![
        ("spec/shiva-sutras.tsv", real),
        ("a phoneme written after the closing ल्", past_the_stop),
        ("a row marked `it` spelling ॺ", marked_not_a_sound),
        ("a phoneme row with an empty spelling", empty_spelling),
    ]
}

/// Every akṣara the differential asks about, over every table.
fn probe_aksharas() -> Vec<String> {
    let mut v: Vec<String> = hal_from_the_file();
    v.extend(ac_from_the_file());
    // The four inside क..ह that हल् does not list, the two crafted spellings,
    // the closing anubandha and its sound, a cluster, and the empty akṣara.
    for extra in ["ऩ", "ऱ", "ळ", "ऴ", "ॹ", "ॺ", "ल", "ल्", "क्ष", "", "x"] {
        v.push(extra.to_string());
    }
    v
}

/// What `व्यञ्जन` answers for every probe, over every table — the signature a
/// mutation has to change.
fn signature(sanskrit_text: &str) -> Vec<(String, String, String)> {
    let probes = probe_aksharas();
    let mut out = Vec::new();
    for (why, sutras) in probe_tables() {
        let mut it = load_with_sutras(sanskrit_text, &sutras);
        for a in &probes {
            let answer = match consonant(&mut it, a) {
                Ok(b) => b.to_string(),
                Err(e) => format!("refused: {e}"),
            };
            out.push((why.to_string(), a.clone(), answer));
        }
    }
    out
}

/// **Every mutation of the हल् reader is killed.**
///
/// Each mutant must change [`signature`] — the routine's answers over four
/// tables. Reported as a list rather than asserted one at a time so that a
/// survivor is named rather than merely counted.
#[test]
fn every_mutation_of_the_hal_reader_is_killed() {
    let base = source("sanskrit_text.t1");
    let clean = signature(&base);
    let mut survivors = Vec::new();

    for (what, from, to) in HAL_MUTANTS {
        if signature(&mutate(&base, from, to)) == clean {
            survivors.push(format!("`{from}` -> `{to}` ({what})"));
        }
    }

    assert!(
        survivors.is_empty(),
        "{} mutation(s) of व्यञ्जन survived — the tests above do not check what \
         they change:\n  {}",
        survivors.len(),
        survivors.join("\n  ")
    );
}

/// The unmutated source survives the path the mutants take.
///
/// Without this, `every_mutation_of_the_hal_reader_is_killed` would pass if
/// `load_with_sutras` were broken for every input — every signature would
/// differ because nothing ran at all.
#[test]
fn the_unmutated_hal_source_survives_the_path_the_mutants_take() {
    let mut it = load(&source("sanskrit_text.t1"));
    for d in hal_from_the_file() {
        assert_eq!(consonant(&mut it, &d), Ok(true), "unmutated व्यञ्जन({d})");
    }
    for d in ac_from_the_file() {
        assert_eq!(consonant(&mut it, &d), Ok(false), "unmutated व्यञ्जन({d})");
    }
}

/// **The three rules the real table cannot ask about, asserted directly.**
///
/// `every_mutation_of_the_hal_reader_is_killed` proves a broken reader answers
/// these differently; this says what the RIGHT answers are, so that the
/// mutation test cannot be satisfied by two wrong readers disagreeing.
#[test]
fn the_range_stops_where_panini_says_it_stops() {
    let text = source("sanskrit_text.t1");
    let tables = probe_tables();
    let get = |why: &str| {
        tables
            .iter()
            .find(|(w, _)| *w == why)
            .map(|(_, t)| t.clone())
            .expect("probe table")
    };

    // हल् ends at ल् . A phoneme written after it is outside the pratyāhāra,
    // however much table follows.
    let mut it = load_with_sutras(&text, &get("a phoneme written after the closing ल्"));
    assert_eq!(
        consonant(&mut it, "ॹ"),
        Ok(false),
        "ॹ is written after the closing ल् and is not in हल् — this reader \
         runs to the end of the table instead of stopping at the anubandha"
    );
    assert_eq!(consonant(&mut it, "क"), Ok(true), "क is still a consonant");

    // An anubandha is a boundary sign, not a sound — even inside the range.
    let mut it = load_with_sutras(&text, &get("a row marked `it` spelling ॺ"));
    assert_eq!(
        consonant(&mut it, "ॺ"),
        Ok(false),
        "ॺ is marked `it` and is a boundary sign, not a sound — this reader \
         is not reading the kind column"
    );
    assert_eq!(consonant(&mut it, "य"), Ok(true), "य is still a consonant");

    // An empty spelling is not a sound, and must not claim every akṣara.
    let mut it = load_with_sutras(&text, &get("a phoneme row with an empty spelling"));
    assert_eq!(
        consonant(&mut it, "अ"),
        Ok(false),
        "अ is a vowel — an empty phoneme field is matching every akṣara"
    );
    assert_eq!(consonant(&mut it, "य"), Ok(true), "य is still a consonant");
}

/// **`व्यञ्जन` reads the file, and this is the proof it is not a baked list.**
///
/// The embed is resolved at LOAD time from the `spec_root` handed to
/// `Interpreter::load` (`anita.rs:306`), so pointing the interpreter at a spec
/// tree whose `shiva-sutras.tsv` says something else must change the answer. A
/// body with the 33 consonants written into it would be unmoved by this.
#[test]
fn pointing_the_embed_at_a_different_table_changes_the_answer() {
    let real = std::fs::read_to_string(spec_root().join("shiva-sutras.tsv"))
        .expect("spec/shiva-sutras.tsv exists");

    // Sūtra 5 cut down: य and व stop being sounds of हल् . Nothing else moves.
    let trimmed: String = real
        .lines()
        .filter(|l| !matches!(*l, "5\t2\tय\tya\tphoneme" | "5\t3\tव\tva\tphoneme"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(trimmed, real, "the two rows must actually have been there");

    let mut it = load_with_sutras(&source("sanskrit_text.t1"), &trimmed);
    for gone in ["य", "व"] {
        assert_eq!(
            consonant(&mut it, gone),
            Ok(false),
            "{gone} still answered true after its row was removed from the \
             table — व्यञ्जन is not reading the file"
        );
    }
    for kept in ["ह", "र", "क", "ल"] {
        assert_eq!(
            consonant(&mut it, kept),
            Ok(true),
            "{kept} must still be a consonant; only य and व were removed"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// `निदान ॱ निदानपङ्क्तयः` — the diagnostic registry, `spec/diagnostics.tsv`.
// ─────────────────────────────────────────────────────────────────────────
//
// The second stub the same `anita.rs` commit cleared, and the expensive one.
// `निदानपङ्क्तिः` walks `निदानपङ्क्तिकोश` and answers ० for every code while
// that arena is empty, so `विवरणम्` has no template to render and every
// diagnostic in the tree renders as its bare code. It is one loop, and eight
// routines downstream were waiting on it.

/// `nidana.t1` with `encode.t1`, which it now reaches its row walk through.
fn load_nidana(nidana: &str) -> Interpreter {
    let encode = source("encode.t1");
    let lex = source("lex.t1");
    Interpreter::load(
        &[
            ("lex.t1", lex.as_str()),
            ("encode.t1", encode.as_str()),
            ("nidana.t1", nidana),
        ],
        &spec_root(),
    )
    .expect("encode.t1 and nidana.t1 load together")
}

/// Every `(code, term, devanagari, english)` of `spec/diagnostics.tsv`, read by
/// THIS test with Rust's own `lines`/`split`.
///
/// The filter is `nidana.rs:225` exactly — `#`, the `code\t` header, and blank
/// lines — so the T1 reader is checked against the FILE and not against a
/// second copy of itself.
fn diagnostics_table() -> Vec<(String, String, String, String)> {
    let text = std::fs::read_to_string(spec_root().join("diagnostics.tsv"))
        .expect("spec/diagnostics.tsv exists");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("code\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 4).then(|| {
                (
                    f[0].to_string(),
                    f[1].to_string(),
                    f[2].to_string(),
                    f[3].to_string(),
                )
            })
        })
        .collect()
}

fn int_call(it: &mut Interpreter, name: &str, args: Vec<Value>) -> Option<i128> {
    it.call(name, args, 200_000_000)
        .ok()
        .and_then(|v| v.as_int())
}

/// How many rows the registry arena holds.
///
/// Read through `Interpreter::global`, because `निदानपङ्क्तिसूचकाङ्क` is a
/// module GLOBAL and not a routine — `call` on it answers `None` for a missing
/// name, which is indistinguishable from a reader that filed nothing.
fn registry_len(it: &Interpreter) -> i128 {
    it.global("निदानपङ्क्तिसूचकाङ्क")
        .unwrap_or_else(|| panic!("निदानपङ्क्तिसूचकाङ्क is a global of निदान"))
        .as_int()
        .expect("निदानपङ्क्तिसूचकाङ्क is a number")
}

/// `nidana.t1` loaded against a spec root whose `diagnostics.tsv` is `table`.
fn load_nidana_with(nidana: &str, table: &str) -> Interpreter {
    let encode = source("encode.t1");
    let lex = source("lex.t1");
    Interpreter::load(
        &[
            ("lex.t1", lex.as_str()),
            ("encode.t1", encode.as_str()),
            ("nidana.t1", nidana),
        ],
        &spec_root_with("diagnostics.tsv", table),
    )
    .expect("the modules load against the substituted spec root")
}

/// **The registry the T1 fills is the registry the file carries.**
///
/// Walked with `निदानपङ्क्तिः` — the routine that was already written and could
/// never find anything, because nothing had ever filled the arena it searches.
#[test]
fn the_diagnostic_reader_fills_the_arena_the_registry_carries() {
    let mut it = load_nidana(&source("nidana.t1"));
    let table = diagnostics_table();
    assert!(table.len() > 40, "the oracle read {} rows", table.len());

    let filled = it
        .call("निदानॱनिदानपङ्क्तयः", vec![], 200_000_000)
        .expect("निदानपङ्क्तयः runs");
    assert!(
        filled.octets().is_none(),
        "निदानपङ्क्तयः must answer the arena, not {filled:?}"
    );

    // THE COUNT. The header must NOT be among them — it is not a comment, not
    // blank, and carries the same four fields every row does, so the only thing
    // refusing it is `शीर्षपङ्क्तिः`.
    let n = registry_len(&it);
    assert_eq!(
        n,
        table.len() as i128,
        "the arena holds {n} rows and spec/diagnostics.tsv carries {}; one \
         too many is the `code term devanagari english` header",
        table.len()
    );

    // AND EACH CODE AT ITS OWN ROW, one-based, in file order.
    let mut wrong = Vec::new();
    for (i, (code, ..)) in table.iter().enumerate() {
        let at = int_call(&mut it, "निदानॱनिदानपङ्क्तिः", vec![octets(code)]);
        if at != Some(i as i128 + 1) {
            wrong.push(format!(
                "{code}: निदानपङ्क्तिः said {at:?}, file row {}",
                i + 1
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} row(s) of spec/diagnostics.tsv are not where the T1 filed them:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );

    // A code no row carries is still ० — the arena did not become a sponge.
    assert_eq!(
        int_call(&mut it, "निदानॱनिदानपङ्क्तिः", vec![octets("Z99")]),
        Some(0),
        "an unknown code must answer ०"
    );
}

/// **Filling the arena twice does not fill it twice over.**
///
/// `निदानपङ्क्तिकोश` is a module global and `निदानपङ्क्तयः` is callable more
/// than once. Without the `निदानपङ्क्तिसूचकाङ्क भवति ०` reset the second call
/// files every row again, the arena grows without bound, and `निदानपङ्क्तिः`
/// keeps answering the FIRST copy — so nothing downstream would ever notice.
#[test]
fn filling_the_registry_twice_leaves_it_the_same_size() {
    let mut it = load_nidana(&source("nidana.t1"));
    it.call("निदानॱनिदानपङ्क्तयः", vec![], 200_000_000)
        .expect("runs");
    let once = registry_len(&it);
    it.call("निदानॱनिदानपङ्क्तयः", vec![], 200_000_000)
        .expect("runs");
    let twice = registry_len(&it);
    assert_eq!(
        once, twice,
        "the arena went from {once} to {twice} rows on a second call"
    );
}

/// **A row with too few fields is refused**, which is Rust's `f.len() >= 4`.
///
/// `spec/diagnostics.tsv` has no such row — every one of its 53 carries four
/// fields — so the guard is unexercised by the real table and a mutation of it
/// survives a differential run against the file alone. This hands the reader a
/// table that asks the question. A short row must be skipped, not filed with
/// fields it does not have.
#[test]
fn a_registry_row_with_too_few_fields_is_refused() {
    let real = std::fs::read_to_string(spec_root().join("diagnostics.tsv"))
        .expect("spec/diagnostics.tsv exists");
    let short = format!("{}\nX01\tकेवलम्\n", real.trim_end());

    let mut it = load_nidana_with(&source("nidana.t1"), &short);
    it.call("निदानॱनिदानपङ्क्तयः", vec![], 200_000_000)
        .expect("runs");

    assert_eq!(
        registry_len(&it),
        diagnostics_table().len() as i128,
        "a two-field row was filed; f.len() >= 4 is not being checked"
    );
    assert_eq!(
        int_call(&mut it, "निदानॱनिदानपङ्क्तिः", vec![octets("X01")]),
        Some(0),
        "the short row must not be findable by its code"
    );
}

/// `(what the mutation does, from, to)` — the diagnostic reader.
const NIDANA_MUTANTS: &[(&str, &str, &str)] = &[
    (
        "the header row stops being refused and becomes registry row १",
        "यदि शीर्षपङ्क्तिः पाठ्यम् कूटादिः कूटसीमा समम् असत्यम् आदि",
        "यदि शीर्षपङ्क्तिः पाठ्यम् कूटादिः कूटसीमा समम् सत्यम् आदि",
    ),
    (
        "the header test reads one octet wrong, so `code` no longer matches it",
        "यदि पाठ्यम् अङ्कः आरभ्य आरम्भ योगः ३ समाप्तम् अन्तः असमम् १०१ आदि",
        "यदि पाठ्यम् अङ्कः आरभ्य आरम्भ योगः ३ समाप्तम् अन्तः असमम् १०२ आदि",
    ),
    (
        "the arena is not reset, so a second fill doubles it",
        "    निदानपङ्क्तिसूचकाङ्क भवति ० ।",
        "    निदानपङ्क्तिसूचकाङ्क भवति निदानपङ्क्तिसूचकाङ्क ।",
    ),
    (
        "the code column is read as the term column, so no row is found by code",
        "चरः कूटादिः ॱॱ न६४ भवति सङ्केतनॱक्षेत्रारम्भः पाठ्यम् आरम्भः सीमा ० ।",
        "चरः कूटादिः ॱॱ न६४ भवति सङ्केतनॱक्षेत्रारम्भः पाठ्यम् आरम्भः सीमा १ ।",
    ),
    (
        "a row with too few fields stops being refused",
        "यदि सङ्केतनॱक्षेत्रसंख्या पाठ्यम् आरम्भः सीमा अधिकम् ३ आदि",
        "यदि सङ्केतनॱक्षेत्रसंख्या पाठ्यम् आरम्भः सीमा अधिकम् ० आदि",
    ),
];

/// **Every mutation of the diagnostic reader is killed.**
///
/// The signature is the arena size after TWO fills plus the row every code
/// lands at, so the reset is part of what a mutation has to preserve.
#[test]
fn every_mutation_of_the_diagnostic_reader_is_killed() {
    let base = source("nidana.t1");
    let table = diagnostics_table();

    // Two tables, for the reason `probe_tables` gives for the sūtras: the real
    // registry has no short row, so the `f.len() >= 4` guard is unexercised by
    // it and a mutation of that guard survives a differential run against the
    // file alone.
    let real = std::fs::read_to_string(spec_root().join("diagnostics.tsv"))
        .expect("spec/diagnostics.tsv exists");
    let short = format!("{}\nX01\tकेवलम्\n", real.trim_end());

    let probe = |text: &str| -> Vec<Option<i128>> {
        let mut v = Vec::new();
        for registry in [&real, &short] {
            let mut it = load_nidana_with(text, registry);
            // Filled TWICE, so that the reset is part of what must be preserved.
            it.call("निदानॱनिदानपङ्क्तयः", vec![], 200_000_000).ok();
            let once = it.global("निदानपङ्क्तिसूचकाङ्क").and_then(Value::as_int);
            it.call("निदानॱनिदानपङ्क्तयः", vec![], 200_000_000).ok();
            v.push(once);
            v.push(it.global("निदानपङ्क्तिसूचकाङ्क").and_then(Value::as_int));
            for (code, ..) in &table {
                v.push(int_call(&mut it, "निदानॱनिदानपङ्क्तिः", vec![octets(code)]));
            }
            v.push(int_call(&mut it, "निदानॱनिदानपङ्क्तिः", vec![octets("X01")]));
        }
        v
    };

    let clean = probe(&base);
    let mut survivors = Vec::new();
    for (what, from, to) in NIDANA_MUTANTS {
        if probe(&mutate(&base, from, to)) == clean {
            survivors.push(format!("`{from}` -> `{to}` ({what})"));
        }
    }

    assert!(
        survivors.is_empty(),
        "{} mutation(s) of निदानपङ्क्तयः survived:\n  {}",
        survivors.len(),
        survivors.join("\n  ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `अक्षरकोश ॱ अक्षराणि` — UAX #29 EXTENDED grapheme clusters.
// ─────────────────────────────────────────────────────────────────────────
//
// **THE STUB TEST THAT USED TO LIVE HERE IS STRUCK.**
// `aksharani_is_still_a_stub_and_both_halves_of_its_blocker_hold` asserted two
// things: that the UAX #29 tables had no `spec/*.tsv` (half 1), and that
// `अक्षराणि`'s declared return `अङ्कः अन्तः अङ्कः अन्तः अ८` was a slice of
// slices no idiom in this corpus can build (half 2). Both are answered and the
// test could not survive either answer, which is what it was for.
//
//   * HALF (1) IS DATA AND IS GONE. `spec/grapheme-break.tsv` (1386 ranges),
//     `spec/incb.tsv` (473) and `spec/extended-pictographic.tsv` (156) are
//     generated from the pinned UCD and named in `anita.rs`'s `TABLES` as
//     `अक्षरभेदकोशः`, `संयोगभेदकोशः` and `चित्राक्षरकोशः`.
//     `the_segmenter_reads_the_three_tables_and_not_a_baked_list` below
//     re-asserts, in the positive direction, that all three are reachable —
//     so REMOVING one fails here rather than quietly restoring a blocker.
//   * HALF (2) WAS A DECISION AND THE ROUTINE TOOK IT. The return is now a
//     COUNT over `अक्षरारम्भकोश`/`अक्षरदैर्घ्यकोश`, with `अक्षरादिः`,
//     `अक्षरमानम्` and `अक्षरसाम्यम्` serving the two consumers that keep a
//     substring. The margin of `sanskrit_text.t1` carries the ADR-0031
//     enumeration; `the_return_is_boundaries_and_the_margin_says_why` below
//     is the ratchet that keeps the reasoning attached to the code.
//
// THE ORACLE IS `GraphemeBreakTest.txt` AND IT IS NOT OPTIONAL. `research/specs/`
// is gitignored (`.gitignore:15`), so a fresh worktree cannot see it; this file
// FAILS HARD with the fetch command rather than skipping, which is the
// convention `adhvan/tests/idna_conformance.rs:136` states and gives the reason
// for. A conformance test that silently passes when its corpus is absent is
// worse than no conformance test.

/// The UCD conformance corpus. A hard failure when absent, never a skip.
fn grapheme_break_test() -> String {
    let p = repo_root().join("research/specs/unicode/ucd/auxiliary/GraphemeBreakTest.txt");
    std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!(
            "{}: {e}\nrun research/specs/fetch-specs.sh unicode\n\n\
             This is a hard failure and not a skip on purpose: research/specs/ is \
             gitignored, so a skip would make this conformance test pass in every \
             worktree that has not fetched the corpus — which is every fresh one.",
            p.display()
        )
    })
}

/// One case: the text, and the expected clusters as `(byte offset, byte len)`.
///
/// The file writes a case as `÷ 0020 × 0308 ÷ 0020 ÷` — `÷` a boundary, `×` no
/// boundary, and the hex between them the code points. Parsed here with `split`
/// rather than through `sanskrit_text::segment`, so the T1 is graded against
/// THE UCD and not against the Rust port of the same algorithm.
fn conformance_cases() -> Vec<(String, Vec<(usize, usize)>)> {
    let text = grapheme_break_test();
    let mut out = Vec::new();
    for line in text.lines() {
        let body = line.split('#').next().unwrap_or("").trim();
        if body.is_empty() {
            continue;
        }
        let mut s = String::new();
        let mut clusters: Vec<(usize, usize)> = Vec::new();
        let mut start = 0usize;
        let mut ok = true;
        for tok in body.split_whitespace() {
            match tok {
                "\u{f7}" => {
                    // A boundary closes the cluster that was open, if any.
                    if s.len() > start {
                        clusters.push((start, s.len() - start));
                        start = s.len();
                    }
                }
                "\u{d7}" => {}
                hex => match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
                    Some(c) => s.push(c),
                    None => {
                        ok = false;
                        break;
                    }
                },
            }
        }
        if ok && !s.is_empty() {
            out.push((s, clusters));
        }
    }
    out
}

/// `sanskrit_text.t1` and `encode.t1` against an arbitrary spec root.
fn load_at(sanskrit_text: &str, root: &Path) -> Interpreter {
    let encode = source("encode.t1");
    let lex = source("lex.t1");
    Interpreter::load(
        &[
            ("lex.t1", lex.as_str()),
            ("encode.t1", encode.as_str()),
            ("sanskrit_text.t1", sanskrit_text),
        ],
        root,
    )
    .expect("the modules load against the spec root")
}

/// Fill the three range arenas ONCE.
///
/// The fills are idempotent and lazy — each `पूरणम्` returns early once it has
/// run — so a lookup may call them unconditionally. Doing it here means the
/// ~2000-row walk is paid once per interpreter instead of inside the first
/// `अक्षराणि` call, where it would dominate that one call's fuel and make a
/// per-case budget meaningless.
fn prepare(it: &mut Interpreter) {
    for fill in ["भेदपूरणम्", "संयोगपूरणम्", "चित्रपूरणम्"]
    {
        it.call(&format!("अक्षरकोशॱ{fill}"), vec![], 4_000_000_000)
            .unwrap_or_else(|e| panic!("{fill} runs: {}", e.reason));
    }
}

fn prepared() -> Interpreter {
    let mut it = unmutated();
    prepare(&mut it);
    it
}

/// `अक्षराणि text` as `(offset, len)` pairs, read back through the accessors.
fn aksharas(it: &mut Interpreter, text: &str) -> Result<Vec<(usize, usize)>, String> {
    let n = it
        .call("अक्षरकोशॱअक्षराणि", vec![octets(text)], 400_000_000)
        .map_err(|e| e.reason)?
        .as_int()
        .ok_or_else(|| "अक्षराणि did not answer a number".to_string())?;
    let mut out = Vec::new();
    for i in 1..=n {
        let at = |name: &str, it: &mut Interpreter| -> Result<usize, String> {
            it.call(name, vec![Value::Int(i)], 1_000_000)
                .map_err(|e| e.reason)?
                .as_int()
                .map(|v| v as usize)
                .ok_or_else(|| format!("{name} did not answer a number"))
        };
        let start = at("अक्षरकोशॱअक्षरादिः", it)?;
        let len = at("अक्षरकोशॱअक्षरमानम्", it)?;
        out.push((start, len));
    }
    Ok(out)
}

/// The clusters as STRINGS, for readable failures.
fn cluster_text(s: &str, cs: &[(usize, usize)]) -> Vec<String> {
    cs.iter()
        .map(|(o, l)| {
            s.get(*o..*o + *l)
                .unwrap_or("<not a char boundary>")
                .to_string()
        })
        .collect()
}

/// **THE CONFORMANCE RUN — every case of `GraphemeBreakTest.txt`.**
///
/// This is the acceptance for the row. The left side is the UCD's own corpus,
/// parsed above with `split`; the right side is a T1 body that read
/// `spec/grapheme-break.tsv`, `spec/incb.tsv` and
/// `spec/extended-pictographic.tsv` through `समावेशः`. Neither side carries a
/// range.
#[test]
#[ignore = "blocked: needs research/specs/unicode (the Unicode data files) not in the public repository"]
fn every_case_of_the_ucd_conformance_corpus_segments_identically() {
    let cases = conformance_cases();
    assert!(
        cases.len() > 700,
        "the corpus parsed to {} cases, which is not GraphemeBreakTest.txt",
        cases.len()
    );

    let mut it = prepared();
    let mut wrong = Vec::new();
    for (s, want) in &cases {
        match aksharas(&mut it, s) {
            Ok(got) if got == *want => {}
            Ok(got) => wrong.push(format!(
                "{:?}: want {:?}, got {:?}",
                s,
                cluster_text(s, want),
                cluster_text(s, &got)
            )),
            Err(e) => wrong.push(format!("{s:?}: did not run: {e}")),
        }
    }

    println!(
        "METRIC sadhana_t1_aksharani_conformance_cases {}",
        cases.len()
    );
    assert!(
        wrong.is_empty(),
        "{} of {} GraphemeBreakTest.txt cases disagree with अक्षरकोश ॱ अक्षराणि:\n  {}",
        wrong.len(),
        cases.len(),
        wrong
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// A `spec/` root with one of the three UAX #29 tables emptied of rows.
///
/// The comment head and the column header stay, so the file is still a
/// well-formed table — it simply claims no code point. Every lookup then falls
/// through to the default, which is what the reader must do for a code point in
/// no range anyway.
/// What a spec root ACTUALLY HOLDS for one table — the path, whether it is
/// there, and how many non-comment rows it carries.
///
/// **BOTH FAILURES IN THIS FILE ARE OF THE FORM "the reader is not reading this
/// file", AND NOTHING SAID WHICH FILE IT READ.** On 2026-09-26 the pair red in
/// a whole-workspace run and passed every other way; four hypotheses died —
/// the landing change, a corrupted `spec/`, the two tests interfering, and
/// cross-process mutation (refuted: these use temp copies) — without localising
/// anything, because no message carried the one fact that would.
///
/// So a red now names the root and the row count. A substituted table that is
/// NOT empty, or a root that is not the temp dir, says in one reproduction what
/// a bisect would take a day to say.
fn table_state(root: &Path, table: &str) -> String {
    let p = root.join(table);
    match std::fs::read_to_string(&p) {
        Ok(t) => {
            let rows = t
                .lines()
                .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                .count();
            // The header line counts as a row here and is subtracted, so an
            // emptied table reads 0 and not 1.
            format!("{} ({} data rows)", p.display(), rows.saturating_sub(1))
        }
        Err(e) => format!("{} (UNREADABLE: {e})", p.display()),
    }
}

fn spec_root_without(table: &str) -> PathBuf {
    let real = std::fs::read_to_string(spec_root().join(table))
        .unwrap_or_else(|e| panic!("spec/{table}: {e}"));
    let head: String = real
        .lines()
        .take_while(|l| l.starts_with('#'))
        .map(|l| format!("{l}\n"))
        .collect();
    spec_root_with(
        table,
        &format!("{head}devanagari\tproperty\tstart\tend\tsource\n"),
    )
}

/// **What each table is WORTH, measured rather than asserted.**
///
/// A port can read three tables and use one. This empties each in turn and
/// counts how many conformance cases break — so a body that never consulted
/// `spec/incb.tsv` would show `0` here and be caught, which no amount of
/// "the embed is present" checking can do.
///
/// The numbers are asserted as a floor rather than exactly: a UCD refresh may
/// add cases, and the point is that the contribution is LARGE and non-zero.
#[test]
#[ignore = "blocked: needs research/specs/unicode (the Unicode data files) not in the public repository"]
fn each_of_the_three_tables_is_load_bearing() {
    let cases = conformance_cases();
    let text = source("sanskrit_text.t1");

    let failures = |root: &Path| -> usize {
        let mut it = load_at(&text, root);
        prepare(&mut it);
        cases
            .iter()
            .filter(|(s, want)| aksharas(&mut it, s).ok().as_ref() != Some(want))
            .count()
    };

    let with_all = failures(&spec_root());
    assert_eq!(with_all, 0, "the unmodified tables must segment every case");

    // The roots are KEPT rather than passed inline: a failure must be able to
    // say what the reader was pointed at, and a temporary that died at the end
    // of the call cannot.
    let root_no_incb = spec_root_without("incb.tsv");
    let root_no_pict = spec_root_without("extended-pictographic.tsv");
    let no_incb = failures(&root_no_incb);
    let no_pict = failures(&root_no_pict);

    println!("METRIC sadhana_t1_aksharani_fail_without_incb {no_incb}");
    println!("METRIC sadhana_t1_aksharani_fail_without_extpict {no_pict}");

    assert!(
        no_incb >= 16,
        "emptying spec/incb.tsv broke only {no_incb} cases; GB9c is either not \
         being applied or not being read from that file.\n  the root it was \
         given holds: {}\n  and the real one holds: {}",
        table_state(&root_no_incb, "incb.tsv"),
        table_state(&spec_root(), "incb.tsv")
    );
    assert!(
        no_pict >= 3,
        "emptying spec/extended-pictographic.tsv broke only {no_pict} cases; \
         GB11 is not reading that file.\n  the root it was given holds: {}\n  \
         and the real one holds: {}",
        table_state(&root_no_pict, "extended-pictographic.tsv"),
        table_state(&spec_root(), "extended-pictographic.tsv")
    );
}

/// **The segmenter reads the three FILES, and this is the proof.**
///
/// The embed is resolved at LOAD time from the `spec_root` handed to
/// `Interpreter::load`, so pointing the interpreter at a tree whose
/// `grapheme-break.tsv` says something else must change the answer. A body with
/// the ranges written into it would be unmoved by this — which is precisely how
/// `व्यञ्जन`'s equivalent test is built, and for the same reason.
#[test]
fn the_segmenter_reads_the_three_tables_and_not_a_baked_list() {
    // All three are reachable — the positive form of the struck stub test's
    // half (1), so that REMOVING a table fails here.
    let anita = std::fs::read_to_string(repo_root().join("crates/sadhana/src/t1/anita.rs"))
        .expect("anita.rs exists");
    for (name, file) in [
        ("अक्षरभेदकोशः", "grapheme-break.tsv"),
        ("संयोगभेदकोशः", "incb.tsv"),
        ("चित्राक्षरकोशः", "extended-pictographic.tsv"),
    ] {
        assert!(
            spec_root().join(file).exists(),
            "spec/{file} has gone; अक्षराणि's data blocker is back"
        );
        assert!(
            anita.contains(name),
            "{file} has no name in anita.rs's TABLES, so समावेशः cannot reach it"
        );
    }

    // U+094D, the virāma, is `Linker` — the ONE row that makes क्ष one akṣara.
    // Strike it from spec/incb.tsv and the conjunct must fall apart into three.
    let mut it = prepared();
    assert_eq!(
        aksharas(&mut it, "क्ष"),
        Ok(vec![(0, 9)]),
        "क्ष must be ONE akṣara of nine octets — GB9c is not firing"
    );

    let incb = std::fs::read_to_string(spec_root().join("incb.tsv")).expect("incb.tsv");
    let without_virama: String = incb
        .lines()
        .filter(|l| !l.contains("\t094D\t094D\t"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(without_virama, incb, "the U+094D Linker row must be there");

    // The root is BOUND, not passed inline: the assertion below concludes "this
    // reader is not reading spec/incb.tsv" and until 2026-09-26 it could not say
    // WHICH file it read. See `table_state`.
    let struck = spec_root_with("incb.tsv", &without_virama);
    let mut it = load_at(&source("sanskrit_text.t1"), &struck);
    prepare(&mut it);
    assert_eq!(
        aksharas(&mut it, "क्ष"),
        Ok(vec![(0, 6), (6, 3)]),
        "with U+094D no longer a Linker, GB9c cannot fire and क + ् must part \
         from ष — this reader is not reading spec/incb.tsv.\n  the root it was \
         given holds: {}\n  and the real one holds: {}",
        table_state(&struck, "incb.tsv"),
        table_state(&spec_root(), "incb.tsv")
    );
}

/// The Devanagari behaviour the rest of this tree depends on, said directly.
///
/// The conformance corpus is enormous and mostly not Sanskrit;
/// `every_case_of_the_ucd_conformance_corpus_segments_identically` could in
/// principle pass while the akṣaras this assembler actually lexes came out
/// wrong, because the UCD's Devanagari cases are a handful. These are the ones
/// `निदान ॱ अक्षरविभागः` was written for, asked of the GENERAL reader.
#[test]
fn the_devanagari_clusters_this_tree_depends_on_are_one_akshara_each() {
    let mut it = prepared();
    // (text, how many akṣaras)
    for (s, n) in [
        ("क", 1),
        ("क्ष", 1),   // GB9c — consonant + virāma + consonant
        ("त्र", 1),   // the same, another conjunct
        ("कि", 1),   // GB9a — the i-matra is a SpacingMark
        ("कु", 1),    // GB9 — the u-matra is an Extend
        ("कं", 1),    // anusvāra, Extend
        ("कः", 1),   // visarga — SpacingMark, NOT a second akṣara
        ("अक्षर", 3), // अ + क्ष + र
        ("योगः", 2), // यो + गः
        ("नमस्ते", 3),
    ] {
        let got = aksharas(&mut it, s).unwrap_or_else(|e| panic!("{s}: {e}"));
        assert_eq!(
            got.len(),
            n,
            "{s} came out as {} akṣaras {:?}, not {n}",
            got.len(),
            cluster_text(s, &got)
        );
        // And the pieces must tile the input exactly — no gap, no overlap.
        let mut at = 0usize;
        for (o, l) in &got {
            assert_eq!(*o, at, "{s}: akṣara at {o} does not continue from {at}");
            at += l;
        }
        assert_eq!(at, s.len(), "{s}: the akṣaras do not cover the whole text");
    }

    // The empty text has no akṣaras and must not answer one.
    assert_eq!(aksharas(&mut it, ""), Ok(vec![]), "the empty text");
}

/// **Segmenting twice does not fill the arena twice over.**
///
/// `अक्षरारम्भकोश` is a module global and `अक्षराणि` is callable more than
/// once. Without the `अक्षरसंख्यानम् भवति ०` reset the second call files every
/// akṣara again and every accessor keeps answering the FIRST text — the same
/// defect `filling_the_registry_twice_leaves_it_the_same_size` pins for the
/// diagnostic registry, and it would make every test above pass on its first
/// case and lie on the rest.
#[test]
fn segmenting_a_second_text_forgets_the_first() {
    let mut it = prepared();
    let long = aksharas(&mut it, "नमस्ते").expect("runs");
    assert_eq!(long.len(), 3);
    let short = aksharas(&mut it, "क").expect("runs");
    assert_eq!(
        short,
        vec![(0, 3)],
        "the second segmentation still reports the first text's akṣaras"
    );
    // And back again, so the reset is not a one-way shrink.
    assert_eq!(aksharas(&mut it, "नमस्ते").expect("runs"), long);
}

/// **`अक्षरसाम्यम्` is the value comparison `parse.rs:376` wants.**
///
/// `a[i-1] != b[j-1]` over two `Vec<&str>`. The boundaries answer is only
/// sufficient if two akṣaras of two different texts can be compared BY VALUE
/// without either being materialised, which is exactly ADR-0031's question and
/// exactly what this routine is for.
#[test]
fn two_aksharas_of_two_texts_compare_by_value_without_a_slice() {
    let mut it = prepared();

    // Segment both, keeping the pairs — the idiom the ADR-0031 note describes.
    let a = aksharas(&mut it, "अक्षर").expect("runs");
    let b = aksharas(&mut it, "क्षर").expect("runs");
    assert_eq!(a.len(), 3);
    assert_eq!(b.len(), 2);

    let same = |it: &mut Interpreter, i: usize, j: usize| -> bool {
        match it.call(
            "अक्षरकोशॱअक्षरसाम्यम्",
            vec![
                octets("अक्षर"),
                Value::Int(a[i].0 as i128),
                Value::Int(a[i].1 as i128),
                octets("क्षर"),
                Value::Int(b[j].0 as i128),
                Value::Int(b[j].1 as i128),
            ],
            2_000_000,
        ) {
            Ok(Value::Bool(v)) => v,
            other => panic!("अक्षरसाम्यम् answered {other:?}, which is not a बूल"),
        }
    };

    // अक्षर is अ क्ष र ; क्षर is क्ष र .
    assert!(same(&mut it, 1, 0), "क्ष and क्ष must compare equal");
    assert!(same(&mut it, 2, 1), "र and र must compare equal");
    assert!(!same(&mut it, 0, 0), "अ and क्ष must not");
    assert!(
        !same(&mut it, 1, 1),
        "क्ष and र differ in LENGTH and must not compare equal"
    );
}

/// **The return type changed, and the margin must carry ADR-0031's reason.**
///
/// The struck stub test asserted the OLD signature. This asserts the new one
/// and, more importantly, that the enumeration ADR-0031 requires before a port
/// may diverge is written where the divergence is — naming both consumers that
/// keep a substring, by file and line. A ratchet on the code alone would let
/// the reasoning rot off while the body stayed.
#[test]
fn the_return_is_boundaries_and_the_margin_says_why() {
    let src = std::fs::read_to_string(repo_root().join("crates/sadhana-t1/src/sanskrit_text.t1"))
        .expect("sanskrit_text.t1 exists");

    assert!(
        src.contains("सार्वजनिक वृत्तिः अक्षराणि आदाय पाठ ॱॱ अङ्कः अन्तः अ८ ददाति अ६४ आदि"),
        "अक्षराणि no longer answers a count over the arenas"
    );
    assert!(
        !src.contains("ददाति अङ्कः अन्तः अङ्कः अन्तः अ८"),
        "the slice-of-slices return is back and nothing can build it"
    );

    // The ADR, and the two consumers the divergence had to account for.
    for needed in ["ADR-0031", "parse.rs:376", "lex.rs:438"] {
        assert!(
            src.contains(needed),
            "the margin of अक्षराणि does not name {needed}; ADR-0031 requires the \
             enumeration to be recorded where the port diverges"
        );
    }

    // And the ranges must come from the files, never from the body. `अक्षरभेदः`
    // would be trivial to write as 1386 comparisons and would pass every
    // behavioural test above on the day it was written.
    let code: String = src
        .lines()
        .map(|l| l.split('॰').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    // THE SPELLING MOVED, THE CONDITION DID NOT (2026-09-14). The tables are
    // still read rather than carried; they are read through
    // `पदविभागॱसमावेशपाठः`, a RUN-TIME lookup in the same store, because a
    // lex-time include cannot serve a compiler compiling itself — it would
    // resolve against whatever store the COMPILING compiler held, which for a
    // compiled one is empty. What this assertion refuses is a table written out
    // as source, and that is refused exactly as before.
    for embed in [
        "पदविभागॱसमावेशपाठः उक्तम् अक्षरभेदकोशः इति",
        "पदविभागॱसमावेशपाठः उक्तम् संयोगभेदकोशः इति",
        "पदविभागॱसमावेशपाठः उक्तम् चित्राक्षरकोशः इति",
    ] {
        assert!(
            code.contains(embed),
            "`{embed}` is not in the code; a table is being carried rather than read"
        );
    }
}
