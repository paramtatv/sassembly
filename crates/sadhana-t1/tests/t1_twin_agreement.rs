//! `W-246` — AGREEMENT TESTS FOR THE HIGHEST-VALUE FILE: `यन्त्रोत्सर्जन`
//! (`yantrotsarjana.t1`) against `crates/sadhana/src/t1/riscv64.rs`, run
//! side by side on the same inputs.
//!
//! # Why this file and not another
//!
//! Statistic 30b (`crates/sadhana/tests/paradigm_t0.rs`, `measure_corpus_halves`)
//! classifies all 359 distinct twin pairs and counts, per T1 source, the
//! ROUTINE pairs a test could run both halves of and none does. `yantrotsarjana.t1`
//! held 20 of them when the census chose it — more than twice the next file,
//! `encode.t1` and `nidana.t1` at 9 — and it is the emitter
//! the chain actually runs: `crates/yantra/tests/paradigm_encode.rs` takes a
//! corpus source from lex to a program on the machine THROUGH this module, and
//! `W-245`'s safety argument is that its two emitters stay byte-identical.
//! Every pair below was named by that census, not chosen by hand.
//!
//! # What an agreement test is here
//!
//! research/27 §2 sets the rule for a ROUTINE pair: run both halves on one
//! input and compare the ANSWERS, over a set of inputs wide enough that a
//! wrong reading of the input cannot pass. `register_name` over one register
//! would agree with an emitter that named every register `स्थिर०`; over all
//! twelve it cannot. This is the defect class `W-236` found — the T1 decoder
//! read `x8..x15` back as `0..7`, and the Rust test agreed with the wrong
//! reading because it asked about one register.
//!
//! # THE SHAPE THIS LANE MET: the answer is not the return value
//!
//! Rust's `register_name` returns an owned `String`. Its T1 twin appends to
//! `उत्सर्जन`'s shared octet arena and returns the CURSOR — the run it wrote
//! is `निर्गमकोश[start..cursor]`, read back with `निर्गमांशः` (utsarjana.t1
//! states why: the routine writes at the cursor and hands back a view, so the
//! text is never owned twice and no allocator is needed). So the comparison
//! resets the arena, calls, and reads the run — the T1 half's answer is a
//! STATE CHANGE plus an index, where the Rust half's is a value. research/27
//! §3 names this the arena-answer shape and it is the reason these tests did
//! not exist: `assert_eq!(rust(x), t1(x))` does not typecheck for any of them.
//!
//! # Fail-first
//!
//! Every test here was run against a MUTATED `yantrotsarjana.t1` before it was
//! run against the real one, and `a_wrong_digit_in_the_t1_numeral_is_caught`
//! keeps that evidence in the tree: it loads the emitter with one digit of the
//! numeral writer changed and asserts the comparison names the disagreement.
//! Without it, a test that passed because both halves were never called would
//! be indistinguishable from one that passed because they agree.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::riscv64::{self, ALLOCATABLE};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// The T1 emitter and what it reads.
const EMITTER: &[&str] = &["ir.t1", "utsarjana.t1", "yantrotsarjana.t1"];

/// Load [`EMITTER`], one source optionally replaced by a mutated text.
fn load_with(replaced: Option<(&str, &str)>) -> Interpreter {
    let texts: Vec<(String, String)> = EMITTER
        .iter()
        .map(|n| {
            let text = match replaced {
                Some((name, text)) if name == *n => text.to_string(),
                _ => source(n),
            };
            ((*n).to_string(), text)
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("the emitter loads: {e:?}"))
}

fn emitter() -> Interpreter {
    load_with(None)
}

fn call(it: &mut Interpreter, name: &str, args: Vec<Value>, fuel: u64) -> Value {
    it.call(name, args, fuel)
        .unwrap_or_else(|e| panic!("`{name}` runs: {e}"))
}

fn text_of(v: &Value) -> String {
    match v.octets() {
        Some(o) => String::from_utf8(o.as_slice().to_vec()).expect("the run is UTF-8"),
        None => panic!("{v:?} is not a run of octets"),
    }
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global"))
}

/// Run one T1 routine that APPENDS to `उत्सर्जन`'s arena and hand back the run
/// it wrote: reset the cursor, call, then read `निर्गमकोश[0..cursor]`.
///
/// This is the whole of the arena-answer shape, in five lines, and every
/// agreement test below is written on it.
fn emitted(it: &mut Interpreter, name: &str, args: Vec<Value>) -> String {
    call(it, "उत्सर्जनॱनिर्गमारम्भः", vec![], 1_000_000);
    call(it, name, args, 200_000_000);
    text_of(&call(it, "उत्सर्जनॱनिर्गमांशः", vec![Value::Int(0)], 1_000_000))
}

/// Every disagreement over `inputs`, as `input: T1 wrote … where riscv64.rs wrote …`.
fn disagreements<T: std::fmt::Display + Copy>(
    it: &mut Interpreter,
    routine: &str,
    inputs: &[T],
    arg: impl Fn(T) -> Value,
    rust: impl Fn(T) -> String,
) -> Vec<String> {
    let mut out = Vec::new();
    for &x in inputs {
        let ours = emitted(it, routine, vec![arg(x)]);
        let theirs = rust(x);
        if ours != theirs {
            out.push(format!(
                "{x}: the T1 twin wrote `{ours}` where riscv64.rs wrote `{theirs}`"
            ));
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// The pairs, one test each. Every one is named by statistic 30b as
// `open_and_untouched` — both halves callable, no test running both.
// ─────────────────────────────────────────────────────────────────────────

/// `register_name ↔ यन्त्रकोष्ठनाम`, over EVERY allocatable register.
///
/// The whole range and not a sample: `W-236`'s defect was an off-by-eight in
/// a register number, and one register cannot see it.
#[test]
fn every_allocatable_register_is_named_the_same_by_both_halves() {
    let mut it = emitter();
    let numbers: Vec<u8> = (0..ALLOCATABLE).collect();
    let bad = disagreements(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रकोष्ठनाम",
        &numbers,
        |n| Value::Int(i128::from(n)),
        riscv64::register_name,
    );
    assert!(
        bad.is_empty(),
        "the two halves name {} of {ALLOCATABLE} registers differently:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
    // The instrument must be able to tell the registers apart at all.
    let all: Vec<String> = numbers
        .iter()
        .map(|n| {
            emitted(
                &mut it,
                "यन्त्रोत्सर्जनॱयन्त्रकोष्ठनाम",
                vec![Value::Int(i128::from(*n))],
            )
        })
        .collect();
    let mut distinct = all.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        all.len(),
        "twelve registers must have twelve names, not {}: {all:?}",
        distinct.len()
    );
}

/// The integers the numeral writer is asked for: both signs, both sides of
/// every carry, and the ends of the range.
const NUMERALS: &[i64] = &[
    0,
    1,
    9,
    10,
    11,
    99,
    100,
    101,
    999,
    1000,
    1024,
    2047,
    2048,
    4095,
    4096,
    65535,
    65536,
    -1,
    -9,
    -10,
    -99,
    -100,
    -2048,
    -2049,
    -65536,
    2_147_483_647,
    -2_147_483_648,
    i64::MAX,
    i64::MIN + 1,
];

/// `riscv64::devanagari ↔ यन्त्राङ्कः` — the emitter's numeral writer.
#[test]
fn the_two_halves_write_the_same_devanagari_numeral() {
    let mut it = emitter();
    let bad = disagreements(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्राङ्कः",
        NUMERALS,
        |n| Value::Int(i128::from(n)),
        riscv64::devanagari,
    );
    assert!(
        bad.is_empty(),
        "the two halves write {} of {} numerals differently:\n  {}",
        bad.len(),
        NUMERALS.len(),
        bad.join("\n  ")
    );
}

/// `riscv64::devanagari ↔ देवनागराङ्कः` — THE SAME Rust function, named a
/// second time in `EMITTERS` against `उत्सर्जन`'s own writer. Both pairs are
/// in the tables and both are measured; that one Rust routine has two T1
/// twins is a fact about the port, and each twin has to be checked.
///
/// This one returns the run itself rather than a cursor, so it needs no arena
/// read — the one routine of the family whose answer IS its return value.
#[test]
fn the_lower_numeral_writer_agrees_with_the_same_rust_routine() {
    let mut it = emitter();
    let mut bad = Vec::new();
    for &n in NUMERALS {
        let ours = text_of(&call(
            &mut it,
            "उत्सर्जनॱदेवनागराङ्कः",
            vec![Value::Int(i128::from(n))],
            200_000_000,
        ));
        let theirs = riscv64::devanagari(n);
        if ours != theirs {
            bad.push(format!("{n}: `{ours}` against `{theirs}`"));
        }
    }
    assert!(
        bad.is_empty(),
        "देवनागराङ्कः disagrees:\n  {}",
        bad.join("\n  ")
    );
}

/// The bit patterns the hexadecimal writer is asked for: every digit, every
/// leading-zero boundary, and the ends.
const PATTERNS: &[u64] = &[
    0,
    1,
    9,
    0xa,
    0xf,
    0x10,
    0xff,
    0x100,
    0x0123_4567,
    0x89ab_cdef,
    0xdead_beef,
    0x8000_0000,
    0xffff_ffff,
    0x1_0000_0000,
    0x7fff_ffff_ffff_ffff,
    0x8000_0000_0000_0000,
    u64::MAX,
];

/// `hex64 ↔ यन्त्रषोडशाङ्कः` — the constant pool's writer.
///
/// The T1 half masks the interpreter's wider number down to 64 bits by hand
/// (`यन्त्रषोडशाङ्कः`'s own margin: "the one place this file needs the cast
/// Rust gets for free"), so the top of the range is where the two could part.
#[test]
fn the_two_halves_write_the_same_hexadecimal_numeral() {
    let mut it = emitter();
    let bad = disagreements(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रषोडशाङ्कः",
        PATTERNS,
        |b| Value::Int(i128::from(b)),
        riscv64::hex64,
    );
    assert!(
        bad.is_empty(),
        "the two halves write {} of {} patterns differently:\n  {}",
        bad.len(),
        PATTERNS.len(),
        bad.join("\n  ")
    );
}

/// `split_hi_lo ↔ यन्त्रोच्चनीचविभागः` — the `lui`/`addi` split.
///
/// The halves DISAGREE IN SHAPE and this is the second thing research/27 §3
/// names: Rust returns `Option<(hi, lo)>`, one value carrying both the verdict
/// and the parts; T1 returns `बूल` and leaves the parts in two globals. The
/// agreement is over all three — refused together, and when accepted, the same
/// two numbers.
#[test]
fn the_two_halves_split_a_constant_into_the_same_high_and_low_parts() {
    let mut it = emitter();
    let constants: Vec<i64> = NUMERALS
        .iter()
        .copied()
        .chain([
            0x7fff_f7ff,
            0x7fff_f800,
            0x7fff_ffff,
            -0x8000_0000,
            0x1000,
            0x1001,
            -0x1000,
            -0x1001,
        ])
        .collect();
    let mut bad = Vec::new();
    for c in constants {
        let ours = call(
            &mut it,
            "यन्त्रोत्सर्जनॱयन्त्रोच्चनीचविभागः",
            vec![Value::Int(i128::from(c))],
            50_000_000,
        );
        let split = matches!(ours, Value::Bool(true));
        match (split, riscv64::split_hi_lo(c)) {
            (false, None) => {}
            (true, Some((hi, lo))) => {
                let ours_hi = global_int(&it, "यन्त्रोच्चांशः");
                let ours_lo = global_int(&it, "यन्त्रनीचांशः");
                if ours_hi != i128::from(hi) || ours_lo != i128::from(lo) {
                    bad.push(format!(
                        "{c}: the T1 twin split it ({ours_hi}, {ours_lo}) where riscv64.rs split it ({hi}, {lo})"
                    ));
                }
            }
            (t1, rust) => bad.push(format!(
                "{c}: the T1 twin {} where riscv64.rs {}",
                if t1 { "split it" } else { "refused it" },
                if rust.is_some() {
                    "split it"
                } else {
                    "refused it"
                }
            )),
        }
    }
    assert!(
        bad.is_empty(),
        "the two halves disagree on {} constants:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The control: the comparison can see red.
// ─────────────────────────────────────────────────────────────────────────

/// One arm of the T1 digit map changed, and the agreement test above must
/// NAME the disagreement rather than pass.
///
/// `अङ्कचिह्नम्` writes the ten digits as ten arms — the frozen grammar has no
/// production that writes a character down, so the digit arrives as a number
/// and leaves as a one-akṣara `पाठः`. The mutation makes the `४` arm answer
/// `५`, which is exactly the defect a character map can carry.
///
/// The test asserts three things, and the third is the one that matters: the
/// comparison sees red, it names both spellings, and IT NAMES ONLY THE INPUTS
/// THAT HOLD A FOUR. A comparison that reported "the texts differ" would
/// satisfy the first two and tell a reader nothing about where to look.
#[test]
fn a_wrong_digit_in_the_t1_numeral_is_caught() {
    let real = source("utsarjana.t1");
    let arm = "यदि अङ्कम् समम् ४ आदि प्रत्यागमनम् उक्तम् ४ इति । इति";
    assert_eq!(
        real.matches(arm).count(),
        1,
        "the `४` arm of अङ्कचिह्नम् is written once"
    );
    let mutated = real.replace(arm, "यदि अङ्कम् समम् ४ आदि प्रत्यागमनम् उक्तम् ५ इति । इति");
    let mut it = load_with(Some(("utsarjana.t1", &mutated)));
    let bad = disagreements(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्राङ्कः",
        NUMERALS,
        |n| Value::Int(i128::from(n)),
        riscv64::devanagari,
    );
    assert!(
        !bad.is_empty(),
        "a mutated digit map must be caught by the numeral agreement test"
    );
    assert!(
        bad.iter().all(|b| b.contains("the T1 twin wrote")),
        "each disagreement names both spellings: {bad:?}"
    );
    let with_a_four: Vec<&i64> = NUMERALS
        .iter()
        .filter(|n| n.unsigned_abs().to_string().contains('4'))
        .collect();
    assert_eq!(
        bad.len(),
        with_a_four.len(),
        "only the numerals holding a four may differ; {} differed and {} hold one:\n  {}",
        bad.len(),
        with_a_four.len(),
        bad.join("\n  ")
    );
}

/// The image the tests above run in is the one the chain runs, and the
/// routines they name are the ones the twin tables name.
///
/// Without this a rename in `yantrotsarjana.t1` would make every test above
/// pass vacuously — `it.call` would refuse, `call` would panic, and that IS
/// caught; but a rename of the RUST half would silently leave the T1 half
/// agreeing with a function nobody uses.
#[test]
fn the_pairs_tested_here_are_the_pairs_the_tables_name() {
    let tables =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t1_sources.rs"))
            .expect("the twin tables are readable");
    for (rust, t1) in [
        ("register_name", "यन्त्रकोष्ठनाम"),
        ("devanagari", "यन्त्राङ्कः"),
        ("riscv64::devanagari", "देवनागराङ्कः"),
        ("hex64", "यन्त्रषोडशाङ्कः"),
        ("split_hi_lo", "यन्त्रोच्चनीचविभागः"),
    ] {
        let row = format!("(\"{rust}\", \"{t1}\")");
        assert!(
            tables.contains(&row),
            "t1_sources.rs still names the pair {row}"
        );
    }
    let _ = Octets::new(b"");
}
