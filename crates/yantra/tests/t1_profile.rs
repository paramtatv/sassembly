//! **WHERE THE `.t1` COMPILER SPENDS ITS STEPS — a stage profile, by fuel.**
//!
//! # The meter is the fuel budget, and nothing new is built
//!
//! `Interpreter::call` takes a budget and refuses with a `fuel` reason when it runs
//! out. `self.fuel` has no public getter, so the reading is taken by BISECTING the
//! budget: the least budget a stage completes under IS its step count, to whatever
//! precision the search is run to. **Deterministic**, unlike a wall clock — the same
//! source gives the same number on a loaded machine and an idle one, which is the
//! property that matters when four lanes share a box.
//!
//! # Kind-first: this measures STAGES, not routines
//!
//! A stage number says which of lex/parse/resolve/typecheck/ir to look inside. It
//! does not name a routine, and a profile that claimed to would be asserting what it
//! cannot see. Routine-level attribution comes after, by changing one routine and
//! re-reading the same stage.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

/// The FRONT HALF only — seven sources, enough to reach IR.
const CHAIN: &[&str] = &[
    "lex.t1",
    "ast.t1",
    "parse.t1",
    "sanskrit_text.t1",
    "sanchaya.t1",
    "artha.t1",
    "ir.t1",
];

fn load() -> Interpreter {
    let texts: Vec<(String, String)> = CHAIN
        .iter()
        .map(|n| {
            (
                (*n).to_string(),
                std::fs::read_to_string(src_dir().join(n)).unwrap_or_else(|e| panic!("{n}: {e}")),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).expect("the chain loads")
}

/// **ONE COMPILE, TIMED PER STAGE.**
///
/// The fuel remainder is NOT readable — `Interpreter` has no public getter and
/// `RunError` carries only `reason` — so bisecting the budget was the only pure-fuel
/// reading available, and it costs 30 full compiles. That is the wrong instrument for
/// a first ranking. An in-test timer around each `call` costs ONE.
///
/// **Its limit, stated:** a wall clock cannot resolve a small change on a shared
/// machine. It can rank stages that differ by an order of magnitude, which is all a
/// first profile needs. Precision comes later, and if it is needed the honest tool is
/// a three-line `pub fn fuel_remaining()` rather than a cleverer clock.
#[test]
#[ignore = "measurement: one full compile of one source"]
fn measure_where_the_t1_compiler_spends_its_steps() {
    use std::time::Instant;
    let name = std::env::var("T1_PROFILE_SOURCE").unwrap_or_else(|_| "vishlesana.t1".to_string());
    let src = std::fs::read_to_string(src_dir().join(&name)).expect("source");
    println!("  source {name}  ({} octets)", src.len());

    let t0 = Instant::now();
    let mut it = load();
    println!("METRIC profile_chain_load_ms {}", t0.elapsed().as_millis());

    let mut mark = Instant::now();
    let lap = |label: &str, mark: &mut Instant| {
        println!("METRIC profile_{label}_ms {}", mark.elapsed().as_millis());
        *mark = Instant::now();
    };

    let t = it
        .call(
            "पदविभागॱपदविभाग",
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            8_000_000_000,
        )
        .expect("lex");
    lap("lex", &mut mark);
    let d = it
        .call(
            "व्याकरॱकार्यक्रमपठनम्",
            vec![Value::Int(t.as_int().unwrap_or(0))],
            8_000_000_000,
        )
        .expect("parse")
        .as_int()
        .unwrap_or(0);
    lap("parse", &mut mark);
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver init");
    let _ = it.call(
        "अर्थॱकार्यक्रमनिर्णयः",
        vec![r.clone(), Value::Int(d)],
        8_000_000_000,
    );
    lap("resolve", &mut mark);
    let _ = it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000);
    let _ = it.call("अर्थॱकार्यक्रमप्रकारपरीक्षा", vec![Value::Int(d)], 8_000_000_000);
    lap("typecheck", &mut mark);
    let _ = it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000);
    let _ = it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(d)], 8_000_000_000);
    lap("ir", &mut mark);
    println!("METRIC profile_declarations {d}");
}

/// **THE WHOLE PIPELINE, THROUGH THE `.t1` DRIVER'S OWN ENTRY POINT.**
///
/// `शृङ्खलाॱमण्डलसङ्कलनम्` is what `t1_driver` drives and it runs front half AND
/// emitter. Timed against the front-half profile above, the difference IS the back
/// half — which is where the census's `t1_seconds` lives: that field brackets
/// exactly one call, `यन्त्रोत्सर्जनॱयन्त्रमण्डलोत्सर्जनम्`, the T1 EMITTER.
#[test]
#[ignore = "measurement: the whole pipeline on one source, minutes"]
fn measure_the_whole_pipeline_against_the_front_half() {
    use sadhana::t1::chain::CHAIN as FULL;
    use std::time::Instant;
    let name = std::env::var("T1_PROFILE_SOURCE").unwrap_or_else(|_| "vishlesana.t1".to_string());
    let module = std::env::var("T1_PROFILE_MODULE").unwrap_or_else(|_| "विश्लेषण".to_string());
    let src = std::fs::read_to_string(src_dir().join(&name)).expect("source");

    let t0 = Instant::now();
    let mut it = Interpreter::load(FULL, &spec_root()).expect("the full chain loads");
    println!(
        "METRIC whole_full_chain_load_ms {}",
        t0.elapsed().as_millis()
    );

    let t1 = Instant::now();
    let out = it.call(
        "शृङ्खलाॱमण्डलसङ्कलनम्",
        vec![
            Value::Octets(Octets::new(src.as_bytes())),
            Value::Octets(Octets::new(module.as_bytes())),
        ],
        400_000_000_000,
    );
    let ms = t1.elapsed().as_millis();
    let octets = out
        .as_ref()
        .ok()
        .and_then(|v| v.octets().map(|o| o.as_slice().len()))
        .unwrap_or(0);
    println!("METRIC whole_pipeline_ms {ms}");
    println!("METRIC whole_emitted_octets {octets}");
    if let Err(e) = &out {
        println!("  REFUSED {:.140}", format!("{e:?}"));
    }
}

/// **THE EMITTER, BY DETERMINISTIC STEPS.** `ashtaka.t1` because its emit completes
/// — it is the octet-pinned source — and because a routine profile needs a run that
/// reaches the end rather than one that refuses partway.
///
/// Steps, not milliseconds: `budget − fuel_remaining()`.
#[test]
#[ignore = "measurement: a full emit, minutes"]
fn measure_the_emitter_in_steps() {
    use sadhana::t1::chain::CHAIN as FULL;
    use std::time::Instant;
    let name = std::env::var("T1_PROFILE_SOURCE").unwrap_or_else(|_| "ashtaka.t1".to_string());
    let module = std::env::var("T1_PROFILE_MODULE").unwrap_or_else(|_| "अष्टक".to_string());
    let src = std::fs::read_to_string(src_dir().join(&name)).expect("source");
    let mut it = Interpreter::load(FULL, &spec_root()).expect("the full chain loads");

    const B: u64 = 400_000_000_000;
    let wall = Instant::now();
    let out = it.call(
        "शृङ्खलाॱमण्डलसङ्कलनम्",
        vec![
            Value::Octets(Octets::new(src.as_bytes())),
            Value::Octets(Octets::new(module.as_bytes())),
        ],
        B,
    );
    let steps = B - it.fuel_remaining();
    let octets = out
        .as_ref()
        .ok()
        .and_then(|v| v.octets().map(|o| o.as_slice().len()))
        .unwrap_or(0);
    println!("METRIC emit_source {name}");
    println!("METRIC emit_steps {steps}");
    println!("METRIC emit_wall_ms {}", wall.elapsed().as_millis());
    println!("METRIC emit_octets {octets}");
    println!(
        "METRIC emit_steps_per_octet {}",
        steps / octets.max(1) as u64
    );
    if let Err(e) = &out {
        println!("  REFUSED {:.140}", format!("{e:?}"));
    }
}

/// **IS THE EMITTER LINEAR IN THE OCTETS IT EMITS?**
///
/// The trunk's seconds suggest not — 0.083 → 0.155 sec/Koctet from smallest to
/// largest — but those were taken under varying load, and a wall clock cannot tell a
/// superlinear algorithm from a busy machine. **Steps can.** If steps-per-octet is
/// flat the cost is volume and there is nothing here to fix; if it climbs, the climb
/// IS the unit.
#[test]
#[ignore = "measurement: several full emits, minutes"]
fn measure_whether_the_emitter_is_linear_in_its_output() {
    use sadhana::t1::chain::CHAIN as FULL;
    const B: u64 = 400_000_000_000;
    let want = std::env::var("T1_PROFILE_SOURCES")
        .unwrap_or_else(|_| "ashtaka.t1,kosha.t1,vishlesana.t1,unparse.t1".to_string());
    println!("  source           steps        octets   steps/octet");
    for name in want.split(',') {
        let name = name.trim();
        let Ok(src) = std::fs::read_to_string(src_dir().join(name)) else {
            println!("  {name:16} UNREADABLE");
            continue;
        };
        // The module name is the source's own मण्डलम् head — read it, never guess.
        let module = src
            .lines()
            .find_map(|l| l.strip_prefix("मण्डलम् "))
            .and_then(|r| r.split_whitespace().next())
            .unwrap_or("")
            .to_string();
        let mut it = Interpreter::load(FULL, &spec_root()).expect("chain loads");
        let out = it.call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![
                Value::Octets(Octets::new(src.as_bytes())),
                Value::Octets(Octets::new(module.as_bytes())),
            ],
            B,
        );
        let steps = B - it.fuel_remaining();
        let oct = out
            .as_ref()
            .ok()
            .and_then(|v| v.octets().map(|o| o.as_slice().len()))
            .unwrap_or(0);
        let per = if oct > 0 { steps / oct as u64 } else { 0 };
        println!("  {name:16} {steps:>11} {oct:>9}   {per:>6}   module={module}");
        println!("METRIC linear_{}_steps {steps}", name.replace('.', "_"));
        println!("METRIC linear_{}_octets {oct}", name.replace('.', "_"));
    }
}

/// **THE 25K DECOMPOSED: FRONT HALF vs EMITTER, PER INSTRUCTION.**
///
/// Two runs per source on the FULL chain — the front stages alone, then
/// `मण्डलसङ्कलनम्` whole — so the emitter is the difference. Both pay the same front
/// cost, which is the assumption this rests on and it is stated rather than hidden.
///
/// **Registered before the run:** ~23.5K emitter, ~1.5K front, per instruction, from
/// the earlier finding that the emitter is ~94% of wall time. If the split reads
/// otherwise, that earlier profile was measuring something else.
#[test]
#[ignore = "measurement: ten full compiles, minutes"]
fn measure_the_front_emitter_split_per_instruction() {
    use sadhana::t1::chain::CHAIN as FULL;
    const B: u64 = 400_000_000_000;
    let want = std::env::var("T1_PROFILE_SOURCES")
        .unwrap_or_else(|_| "ashtaka.t1,kosha.t1,unparse.t1,parse.t1,ir.t1".to_string());
    println!(
        "  source          front_steps   emit_steps   insts   front/i   emit/i   total/i   octets"
    );
    for name in want.split(',') {
        let name = name.trim();
        let Ok(src) = std::fs::read_to_string(src_dir().join(name)) else {
            continue;
        };
        let module = src
            .lines()
            .find_map(|l| l.strip_prefix("मण्डलम् "))
            .and_then(|r| r.split_whitespace().next())
            .unwrap_or("")
            .to_string();
        let octs = Value::Octets(Octets::new(src.as_bytes()));

        // (1) FRONT HALF ALONE, on the full chain so the load is comparable.
        let mut it = Interpreter::load(FULL, &spec_root()).expect("chain loads");
        let mut front = 0u64;
        let mut per: Vec<(&str, u64)> = Vec::new();
        let mut mark = 0u64;
        let step = |it: &mut Interpreter, f: &mut u64, n: &str, a: Vec<Value>| -> Value {
            let r = it.call(n, a, B);
            *f += B - it.fuel_remaining();
            r.unwrap_or(Value::Int(0))
        };
        let tk = step(&mut it, &mut front, "पदविभागॱपदविभाग", vec![octs.clone()]);
        per.push(("lex", front - mark));
        mark = front;
        // THE LEXER'S OWN OUTPUT. Identical octets downstream could survive a lexer
        // change that merely re-tokenises to the same text, so the token count is
        // asserted separately from the emitted image.
        println!("    TOKENS {name:14} {}", tk.as_int().unwrap_or(-1));
        let d = step(
            &mut it,
            &mut front,
            "व्याकरॱकार्यक्रमपठनम्",
            vec![Value::Int(tk.as_int().unwrap_or(0))],
        )
        .as_int()
        .unwrap_or(0);
        per.push(("parse", front - mark));
        mark = front;
        let r = step(&mut it, &mut front, "अर्थॱनिर्णायकारम्भः", vec![]);
        let _ = step(
            &mut it,
            &mut front,
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r.clone(), Value::Int(d)],
        );
        per.push(("resolve", front - mark));
        mark = front;
        let _ = step(&mut it, &mut front, "अर्थॱप्रकारपरीक्षकारम्भः", vec![r]);
        let _ = step(
            &mut it,
            &mut front,
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(d)],
        );
        per.push(("typecheck", front - mark));
        mark = front;
        let _ = step(&mut it, &mut front, "मध्यरूपॱआरम्भः", vec![]);
        let _ = step(
            &mut it,
            &mut front,
            "मध्यरूपॱकार्यक्रमरचना",
            vec![Value::Int(d)],
        );
        per.push(("ir", front - mark));
        // The instruction count is the IR cursor, never the arena's length.
        let insts = it
            .global("आज्ञासूचकाङ्क")
            .and_then(Value::as_int)
            .unwrap_or(0)
            .max(1) as u64;

        // (2) THE WHOLE PIPELINE, fresh.
        let mut it2 = Interpreter::load(FULL, &spec_root()).expect("chain loads");
        let out = it2.call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octs, Value::Octets(Octets::new(module.as_bytes()))],
            B,
        );
        let whole = B - it2.fuel_remaining();
        let oct = out
            .as_ref()
            .ok()
            .and_then(|v| v.octets().map(|o| o.as_slice().len()))
            .unwrap_or(0);
        let emit = whole.saturating_sub(front);

        println!(
            "  {name:14}{front:>12}{emit:>13}{insts:>8}{:>10}{:>9}{:>10}{oct:>9}",
            front / insts,
            emit / insts,
            whole / insts
        );
        println!("METRIC split_{}_front {front}", name.replace('.', "_"));
        println!("METRIC split_{}_emit {emit}", name.replace('.', "_"));
        println!("METRIC split_{}_insts {insts}", name.replace('.', "_"));
        // PER STAGE, and the sum is checked against the total: if they disagree the
        // per-call fuel deltas are being mis-attributed, which is the second falsifier.
        let sum: u64 = per.iter().map(|(_, s)| *s).sum();
        for (s, v) in &per {
            println!(
                "    STAGE {name:14} {s:10} {:>12} steps  {:>8}/inst",
                v,
                v / insts
            );
        }
        println!(
            "    STAGE {name:14} {:10} {sum:>12} steps  (front total {front}, {})",
            "SUM",
            if sum == front { "AGREES" } else { "DISAGREES" }
        );
    }
}
