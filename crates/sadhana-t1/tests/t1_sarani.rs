//! **`समावेशसारणीॱसारणीपूरणम्` IS EXECUTED HERE** — the generated embed store, run
//! against `spec/` rather than against a second copy of the tables.
//!
//! # What this file is the evidence for
//!
//! `lex.t1` declares `समावेशनामकोश` / `समावेशपाठकोश` / `समावेशसीमाकोश` /
//! `समावेशसंख्या`, and until now **the host filled them** — `nirvahana.rs:988`
//! reads `spec/` at load and injects all four. In `lex.t1`'s own margin
//! `समावेशसंख्या` is *"how many the host filled"*. A compiled compiler therefore
//! carried no mnemonic table, no directive table and no akṣara tables, so it
//! emitted correct Sassembly text and then assembled nothing — the empty
//! whole-corpus native image, with every refusal record clean.
//!
//! `crates/sadhana-t1/src/sarani.t1` is that store written in this language,
//! generated from `spec/` by `tools/mkspectables.py`. This file runs it.
//!
//! # Why the generator's own round trip is not enough
//!
//! The generator decodes its letters back in Python before it writes the file,
//! which grades **the emitter**. Nothing there executes the `.t1` decoder, so a
//! `सङ्केतयोजनम्` that dropped every eighth octet would pass it. These tests
//! grade the decoder, by running it and comparing the four globals it fills
//! against the same files `spec/` holds — never against a list written here.
//!
//! # And why the interpreter is pointed at an EMPTY spec root
//!
//! `nirvahana.rs:988` fills those four globals at load from the `spec_root` it
//! is handed. Against the real `spec/` every assertion below would pass over a
//! `सारणीपूरणम्` that did nothing at all — the host would have written what the
//! test read. So [`load`] hands the interpreter a root with NO tables in it, the
//! store is empty before the call, and everything in it afterwards came out of
//! the module. The files the assertions compare against are then read directly,
//! outside the interpreter.
//!
//! # And why the fill is not guarded on `समावेशसंख्या`
//!
//! It guards on its own `पूरितम्` flag, so it still fills a store the host has
//! already populated — which is what makes the agreement in
//! [`the_fill_agrees_with_the_host_it_replaces`] checkable at all.
//! [`a_second_call_appends_nothing`] is the case that must still be REFUSED.

use sadhana::t1::anita;
use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::{Path, PathBuf};

/// Unique scratch roots, shared with every other binary that needs one —
/// because five copies of `(pid, counter)` was five copies of one defect.
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

/// A spec root that exists and holds NOTHING, so the host's load-time fill
/// reads no file and leaves `समावेशसंख्या` at ०.
///
/// Per CALL, with a process id and a counter in the name, for the reason
/// `t1_exec_aksara.rs` gives: two tests that share a temp root can remove it
/// under each other. Nothing is copied into it — that is the point of it.
fn empty_spec_root() -> PathBuf {
    // `unique_root` and not a local (pid, counter): that pair is unique inside
    // ONE PROCESS and not on disk, because pids are reused and these roots are
    // never removed. This family had accumulated its own pile of directories
    // under $TMPDIR (W-301).
    let dir = spec_fixture::unique_root("sarani-nospec");
    std::fs::create_dir_all(&dir).expect("temp spec root");
    dir
}

/// **THE LOADER IS PART OF THE TEST**, so this file has its own rather than
/// borrowing one — `t1_exec_aksara.rs` says the same thing for the same reason.
///
/// Two sources and no more: `सारणी` imports `पदविभाग` and nothing else, and
/// `पदविभाग` imports nothing at all, so this is the whole closure. Loading the
/// chain instead would put six other modules' globals in scope and a name
/// collision would read as a fill that worked. Neither source carries a
/// `समावेशः` of its own, which is why an empty spec root still loads.
fn load_with(spec: &Path) -> Interpreter {
    let lex = source("lex.t1");
    let sarani = source("sarani.t1");
    Interpreter::load(
        &[("lex.t1", lex.as_str()), ("sarani.t1", sarani.as_str())],
        spec,
    )
    .expect("lex.t1 and sarani.t1 load together")
}

/// The loader every assertion about the module's own output uses: no tables
/// reached the interpreter at all.
fn load() -> Interpreter {
    let dir = empty_spec_root();
    let it = load_with(&dir);
    assert_eq!(
        it.global("समावेशसंख्या").and_then(Value::as_int),
        Some(0),
        "an empty spec root must leave the store empty — if this is not ०, the \
         assertions below are about the HOST's fill and not the module's"
    );
    it
}

/// `(table name, file bytes)` for every table `anita.rs` names, in its order —
/// which is the order `सारणीपूरणम्` indexes the three arenas by, because the
/// generator reads that same `const`.
fn tables_from_disk() -> Vec<(&'static str, Vec<u8>)> {
    anita::table_names()
        .into_iter()
        .map(|n| {
            let rel = anita::table_path(n).unwrap_or_else(|| panic!("{n} has a file in TABLES"));
            let bytes = std::fs::read(spec_root().join(rel))
                .unwrap_or_else(|e| panic!("spec/{rel} is readable: {e}"));
            (n, bytes)
        })
        .collect()
}

/// One entry of an arena global, or a panic that names which one is missing —
/// `None` from a bad index and `None` from a global that is not an arena are
/// different defects and must not read alike.
fn entry(it: &Interpreter, global: &str, index: usize) -> Value {
    let v = it
        .global(global)
        .unwrap_or_else(|| panic!("{global} is a global of पदविभाग"));
    match v {
        Value::Arena(a) => a
            .borrow()
            .get(index)
            .unwrap_or_else(|| panic!("{global} has no entry {index}"))
            .clone(),
        other => panic!("{global} is not an arena, it is {other:?}"),
    }
}

fn entry_octets(it: &Interpreter, global: &str, index: usize) -> Vec<u8> {
    match entry(it, global, index) {
        Value::Octets(o) => o.as_slice().to_vec(),
        other => panic!("{global}[{index}] is not a run of octets, it is {other:?}"),
    }
}

fn entry_int(it: &Interpreter, global: &str, index: usize) -> i128 {
    entry(it, global, index)
        .as_int()
        .unwrap_or_else(|| panic!("{global}[{index}] is not a number"))
}

fn fill(it: &mut Interpreter) -> i128 {
    it.call("सारणीपूरणम्", Vec::new(), 4_000_000_000)
        .expect("सारणीपूरणम् runs")
        .as_int()
        .expect("सारणीपूरणम् answers a number")
}

/// **The store the module fills is the store `spec/` holds.**
///
/// Every one of the three arenas is compared entry for entry against the files
/// on disk. The total is asserted too, but as a CROSS-CHECK and not as the
/// oracle: a decoder that swapped two tables of equal size would answer the
/// right total.
#[test]
fn the_generated_module_fills_the_embed_store_from_spec() {
    let mut it = load();
    let want = tables_from_disk();
    let total = fill(&mut it);

    let count = it
        .global("समावेशसंख्या")
        .and_then(Value::as_int)
        .expect("समावेशसंख्या is a number");
    assert_eq!(
        count,
        want.len() as i128,
        "समावेशसंख्या after the fill; anita.rs names {} tables",
        want.len()
    );
    println!("METRIC sarani_tables {count}");
    println!("METRIC sarani_octets {total}");

    for (i, (name, bytes)) in want.iter().enumerate() {
        assert_eq!(
            String::from_utf8_lossy(&entry_octets(&it, "समावेशनामकोश", i)),
            *name,
            "समावेशनामकोश[{i}]"
        );
        assert_eq!(
            entry_int(&it, "समावेशसीमाकोश", i),
            bytes.len() as i128,
            "समावेशसीमाकोश[{i}] — the byte count of {name}"
        );
        let got = entry_octets(&it, "समावेशपाठकोश", i);
        assert_eq!(
            got.len(),
            bytes.len(),
            "समावेशपाठकोश[{i}] — the decoded length of {name}"
        );
        if got != *bytes {
            let at = got
                .iter()
                .zip(bytes.iter())
                .position(|(a, b)| a != b)
                .expect("the lengths agree, so a difference has a position");
            panic!(
                "समावेशपाठकोश[{i}] ({name}) differs from spec/ at octet {at}: \
                 decoded {:#04x}, file {:#04x}",
                got[at], bytes[at]
            );
        }
    }

    let sum: usize = want.iter().map(|(_, b)| b.len()).sum();
    assert_eq!(total, sum as i128, "the total the routine answers");
}

/// **THE CASE THAT MUST STILL BE REFUSED: the second call appends nothing.**
///
/// The accumulator is a module global and `सारणीखण्ड…` only ever appends, so a
/// fill that ran twice would leave every table doubled and `समावेशसीमाकोश`
/// twice the file's size — which still reads as "the store is populated" to any
/// caller that does not compare against `spec/`. The guard is the module's own
/// `पूरितम्` flag.
#[test]
fn a_second_call_appends_nothing() {
    let mut it = load();
    let first = fill(&mut it);
    let second = fill(&mut it);
    assert_eq!(second, first, "the second call's answer");

    let want = tables_from_disk();
    for (i, (name, bytes)) in want.iter().enumerate() {
        assert_eq!(
            entry_int(&it, "समावेशसीमाकोश", i),
            bytes.len() as i128,
            "समावेशसीमाकोश[{i}] ({name}) after a second fill"
        );
        assert_eq!(
            entry_octets(&it, "समावेशपाठकोश", i).len(),
            bytes.len(),
            "समावेशपाठकोश[{i}] ({name}) after a second fill"
        );
    }
}

/// **The module's fill agrees with the host's, octet for octet.**
///
/// The two tests above run against an empty spec root, so they say what the
/// module produces and nothing about whether the product agrees with it. This
/// one loads against the REAL `spec/` — the host fills all four globals at
/// load — and then calls the fill anyway, which overwrites them. Every entry
/// must be unchanged. A disagreement here means the generated module and
/// `nirvahana.rs` are reading `spec/` differently, and the compiled compiler
/// would assemble against a table the interpreted one never saw.
#[test]
fn the_fill_agrees_with_the_host_it_replaces() {
    let mut it = load_with(&spec_root());
    let want = tables_from_disk();
    assert_eq!(
        it.global("समावेशसंख्या").and_then(Value::as_int),
        Some(want.len() as i128),
        "the host fills the store at load against the real spec/"
    );
    let before: Vec<Vec<u8>> = (0..want.len())
        .map(|i| entry_octets(&it, "समावेशपाठकोश", i))
        .collect();

    let total = fill(&mut it);
    assert_eq!(
        it.global("समावेशसंख्या").and_then(Value::as_int),
        Some(want.len() as i128),
        "समावेशसंख्या after the module overwrites the host's fill"
    );
    for (i, (name, _)) in want.iter().enumerate() {
        assert_eq!(
            entry_octets(&it, "समावेशपाठकोश", i),
            before[i],
            "समावेशपाठकोश[{i}] ({name}) — the module disagrees with the host"
        );
    }
    assert_eq!(total, before.iter().map(Vec::len).sum::<usize>() as i128);
}

/// **EVERY ROUTINE OF `सारणी` PARSES WHEN THE WHOLE CORPUS IS LOADED WITH IT.**
///
/// FOUND BY THE PARSE TWIN AND BY NOTHING ELSE IN THIS FILE, 2026-09-14. The
/// module's idempotence flag was first spelled `पूरितम्`, and `encode.t1:3872`
/// is `सार्वजनिक वृत्तिः पूरितम् आदाय …` — a public routine of ARITY THREE. The
/// interpreter's arity table is global, so with `encode.t1` loaded the line
/// `यदि पूरितम् समम् सत्यम् आदि` reads the flag as a three-argument bare call,
/// eats `समम् सत्यम् आदि` as its arguments, and the body refuses to parse two
/// lines later. **Every test above passed** — their loader is two sources and
/// carries no `encode.t1`.
///
/// So this one loads the corpus. A `.t1` module is not well-formed on its own;
/// it is well-formed in the company it keeps.
#[test]
fn every_routine_parses_with_the_whole_corpus_loaded() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    paths.sort();
    assert!(
        paths.iter().any(|p| p.file_name().unwrap() == "sarani.t1"),
        "the corpus walk must reach sarani.t1 itself"
    );
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    let it = Interpreter::load_paths(&refs, &spec_root()).expect("the whole corpus loads");

    // The routine names are read from the FILE, so a generator that stops
    // emitting one cannot make this test pass by having nothing to check.
    let text = source("sarani.t1");
    let names: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let code = l.split('\u{0970}').next().unwrap_or("");
            let mut w = code.split_whitespace();
            loop {
                let word = w.next()?;
                if word == "\u{0935}\u{0943}\u{0924}\u{094D}\u{0924}\u{093F}\u{0903}" {
                    return w.next().map(str::to_string);
                }
            }
        })
        .collect();
    assert!(
        names.len() >= 19,
        "sarani.t1 declares {} routines — the generator emits at least 19",
        names.len()
    );

    let mut refused: Vec<String> = Vec::new();
    for name in &names {
        let r = it
            .routine(name)
            .unwrap_or_else(|| panic!("{name} is declared in sarani.t1 but absent from the load"));
        if let Some(why) = r.why_not() {
            refused.push(format!("{name}: {why}"));
        }
    }
    assert!(
        refused.is_empty(),
        "{} of {} routines of सारणी did not parse with the corpus loaded \
         (a name it declares is a routine somewhere else):\n  {}",
        refused.len(),
        names.len(),
        refused.join("\n  ")
    );
    println!("METRIC sarani_routines {}", names.len());
}
