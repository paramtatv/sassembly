//! **HOW MANY FIELD ACCESSES THE CORPUS LOWERS, AND AT WHAT OFFSETS.** A plain
//! population count: the tree has no figure for it either way.
//!
//! # THIS FILE WAS WRITTEN FOR A DEFECT THAT DOES NOT EXIST, AND THE RECORD IS KEPT
//!
//! It was commissioned to size "the population riding the field-offset defect" —
//! the claim being that a two-field `संरचना` gets THREE field-table entries, that
//! the first field is named by none of them and refuses at cause ३९, and that the
//! second therefore resolves to ordinal 2 and emits offset १६ where ८ is correct.
//!
//! **None of that is true of the corpus. The probe that produced it was malformed:
//! a `संरचना` opens with `आरभ्य`, and the probe wrote `आदि`.** The corpus has 59
//! structs, 59 `आरभ्य` and zero `आदि`, so `संरचनापठनम्` never consumed an opener,
//! the field loop began on the wrong token, and `आदि` and `ॱॱ` were parsed AS FIELD
//! NAMES. Three entries for two fields, first field missing, second at ordinal 2 —
//! **every number was the parser correctly reporting on wreckage.** Re-measured with
//! the opener corrected: entries name the right tokens, offsets are 0, 8, 16, and
//! ३९ never fires.
//!
//! **The falsifier that killed it was registered in advance and was the right one.**
//! Before the run: *"any offset of 0 means a first field lowered, which contradicts
//! cause ३९ being the truth about `प्रथमम्`"* — predicted zero of them, and said one
//! would overturn the account rather than be noise. **Offset 0 is exactly what a
//! correctly-lowered first field emits.** The prediction was derived faithfully from
//! a model that was wrong at its root, so it failed one layer earlier than either
//! party expected, in the place it had named.
//!
//! **Kept rather than deleted, because a file whose premise was retracted teaches
//! what one that quietly became a plain census does not** — and because the shape
//! recurs: a fixture that varies one axis while mis-forming another measures the
//! mis-forming.
//!
//! # This is a COUNT, not a correctness check
//!
//! It reports how many field instructions the corpus emits and at which offsets. It
//! asserts nothing about whether those offsets are right, because **the right offset
//! is not knowable from the instruction alone** — it depends on the declaration the
//! ordinal came from. A wrong offset and a right one are the same shape here.
//!
//! # What it does NOT cover
//!
//! Nothing downstream of IR: not emission, not assembly, not linking, not execution.
//! A source whose IR is full of wrong offsets appears here and says nothing about
//! whether its image runs — those are different questions and this file answers one.
//!
//! # If it cannot run
//!
//! **It FAILS, deliberately.** `#[ignore]`d measurements that match no filter exit 0
//! and read as green, which is how a census once reported success having matched no
//! test. So this asserts the corpus was found and that IR was built for at least one
//! source: an empty scan is a red, never a quiet zero.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `मध्यरूप`'s two field kinds, from `ir.t1:135` and `ir.t1:246`.
const LOAD_FIELD: i128 = 19;
const ADDR_OF_FIELD: i128 = 25;

/// The seven modules `मध्यरूप` needs to reach IR — the same set `load_ir_chain`
/// uses in `t1_execution.rs`, replicated here rather than shared so this file
/// cannot be blocked by a guard belonging to another test.
const CHAIN: &[&str] = &[
    "lex.t1",
    "ast.t1",
    "parse.t1",
    "artha.t1",
    "sanchaya.t1",
    "sanskrit_text.t1",
    "ir.t1",
];

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn source(name: &str) -> String {
    let p = src_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn load_chain() -> Interpreter {
    let texts: Vec<(String, String)> = CHAIN
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("the IR chain loads: {e:?}"))
}

/// A named member of a `Value::Record`, as an integer; `None` when absent or not
/// a record, so a missing field is distinguishable from a zero one.
fn int_of(v: &Value, name: &str) -> Option<i128> {
    match v {
        Value::Record(r) => r.borrow().get(name).and_then(Value::as_int),
        _ => None,
    }
}

/// Every `.t1` in the corpus directory, sorted.
fn corpus() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(src_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    v.sort();
    v
}

/// **COUNT THE FIELD INSTRUCTIONS THE CORPUS EMITS, AND THEIR OFFSETS.**
///
/// A FRESH INTERPRETER PER SOURCE, and that is not caution — `t1_execution.rs`'s
/// own probe records that one interpreter carried the first source's arenas into
/// the second and reported the same cause for both. `मध्यरूपॱआरम्भः` zeroes
/// `आज्ञासूचकाङ्क` (`ir.t1:778`), so the scan below reads one source's
/// instructions and not an accumulation.
///
/// The arena is walked TO THE CURSOR and not to its length: a length walk reads
/// whatever a previous program left past the live end.
#[test]
#[ignore = "measurement: the whole corpus to IR, minutes"]
fn measure_field_offsets_over_the_corpus() {
    let names = corpus();
    assert_eq!(
        names.len(),
        20,
        "the corpus is twenty sources; found {names:?}"
    );

    let mut by_kind: BTreeMap<i128, usize> = BTreeMap::new();
    let mut offsets: BTreeMap<i128, usize> = BTreeMap::new();
    let mut offsets_by_kind: BTreeMap<(i128, i128), usize> = BTreeMap::new();
    let mut absent_offset = 0usize;
    let mut built = 0usize;
    let mut per_source: Vec<(String, usize, usize)> = Vec::new();

    for name in &names {
        let src = source(name);
        let mut it = load_chain();
        let Ok(toks) = it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        else {
            println!("  {name:22} LEX DECLINED");
            continue;
        };
        let toks = toks.as_int().unwrap_or(0);
        let Ok(parsed) = it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        else {
            println!("  {name:22} PARSE DECLINED");
            continue;
        };
        let parsed = parsed.as_int().unwrap_or(0);
        if parsed == 0 {
            println!("  {name:22} NO DECLARATIONS — not a program");
            continue;
        }
        let Ok(resolver) = it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        else {
            println!("  {name:22} RESOLVER INIT DECLINED");
            continue;
        };
        let resolved = it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![resolver.clone(), Value::Int(parsed)],
            4_000_000_000,
        );
        if resolved != Ok(Value::Bool(true)) {
            println!("  {name:22} RESOLVE DECLINED");
            continue;
        }
        if it
            .call("अर्थॱप्रकारपरीक्षकारम्भः", vec![resolver], 5_000_000)
            .is_err()
        {
            println!("  {name:22} TYPECHECK INIT DECLINED");
            continue;
        }
        let checked = it.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000,
        );
        if checked != Ok(Value::Bool(true)) {
            println!("  {name:22} TYPECHECK DECLINED");
            continue;
        }
        if it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000).is_err() {
            println!("  {name:22} IR INIT DECLINED");
            continue;
        }
        if it
            .call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
            .is_err()
        {
            println!("  {name:22} IR BUILD DECLINED");
            continue;
        }
        built += 1;

        let cursor = it.global("आज्ञासूचकाङ्क").and_then(Value::as_int).unwrap_or(0);
        let Some(Value::Arena(insts)) = it.global("आज्ञाकोश").cloned() else {
            println!("  {name:22} आज्ञाकोश IS NOT AN ARENA");
            continue;
        };
        let (mut n19, mut n25) = (0usize, 0usize);
        for i in 1..=usize::try_from(cursor).unwrap_or(0) {
            let Some(ins) = insts.borrow().get(i).cloned() else {
                continue;
            };
            let Some(kind) = int_of(&ins, "भेद") else {
                continue;
            };
            if kind != LOAD_FIELD && kind != ADDR_OF_FIELD {
                continue;
            }
            *by_kind.entry(kind).or_default() += 1;
            if kind == LOAD_FIELD {
                n19 += 1;
            } else {
                n25 += 1;
            }
            match int_of(&ins, "ध्रुवमूल्यम्") {
                Some(off) => {
                    *offsets.entry(off).or_default() += 1;
                    *offsets_by_kind.entry((kind, off)).or_default() += 1;
                }
                None => absent_offset += 1,
            }
        }
        println!("  {name:22} IR BUILT  {cursor:>5} insts   kind19 {n19:>4}  kind25 {n25:>4}");
        per_source.push((name.clone(), n19, n25));
    }

    let k19 = by_kind.get(&LOAD_FIELD).copied().unwrap_or(0);
    let k25 = by_kind.get(&ADDR_OF_FIELD).copied().unwrap_or(0);
    println!("METRIC field_offsets_sources_ir_built {built}");
    println!("METRIC field_offsets_kind19_load_field {k19}");
    println!("METRIC field_offsets_kind25_addr_of_field {k25}");
    println!("METRIC field_offsets_total {}", k19 + k25);
    println!("METRIC field_offsets_distinct {}", offsets.len());
    // STATED EVEN WHEN ZERO. "No offset of 0" is a claim, and an omitted row reads
    // as untested rather than as measured.
    println!(
        "METRIC field_offsets_zero {}",
        offsets.get(&0).copied().unwrap_or(0)
    );
    println!(
        "METRIC field_offsets_eight {}",
        offsets.get(&8).copied().unwrap_or(0)
    );
    println!("METRIC field_offsets_absent_member {absent_offset}");
    for (off, n) in &offsets {
        println!("  OFFSET {off:>6}  x{n}");
    }
    for ((kind, off), n) in &offsets_by_kind {
        println!("  KIND {kind} OFFSET {off:>6}  x{n}");
    }

    // AN EMPTY SCAN IS A RED, NEVER A QUIET ZERO — see the module margin.
    assert!(built > 0, "no source reached IR; the scan measured nothing");
    assert!(
        k19 + k25 > 0,
        "IR was built for {built} sources and NOT ONE field instruction was found — \
         either the corpus has no field access or the kind numbers {LOAD_FIELD}/{ADDR_OF_FIELD} \
         are wrong, and both are findings rather than a zero"
    );
}
