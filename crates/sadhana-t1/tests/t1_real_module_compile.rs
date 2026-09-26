//! **Can the compiled compiler compile a REAL module?**
//!
//! Every rung on the ladder so far compiles a demo source written for the
//! rung: the four-construct batch's constructs total 436 octets across four hand-written
//! modules. The corpus is 38,335 lines. Nothing has ever asked the `.t1`
//! compiler to compile a file that someone actually wrote, and the distance
//! between those two facts is the distance left to self-hosting.
//!
//! This asks. `शृङ्खला ॱ पाठवस्तुरचना` is the one-call entry point — Sassembly
//! text to a linkable object — so the question needs no new rung and no native
//! build: the interpreter running the `.t1` compiler over a real file answers
//! it in seconds, and names WHICH file and WHICH stage refuses.
//!
//! # What a refusal means here, and what it does not
//!
//! This is the INTERPRETED `.t1` compiler. A refusal is a gap in the compiler
//! itself — `ir.t1` still carries 49 stub markers — and is real. A SUCCESS is
//! weaker than it looks: it says the `.t1` compiler built an object, not that
//! the object is correct, and not that the image would build the same one.
//! Rung 100 is the evidence for the second claim and it covers four constructs.
//!
//! # The answer, 2026-09-20
//!
//! **20 of 21 modules; 38,320 of 38,335 lines**, in 4162s. Every file anyone
//! actually wrote compiles, `encode.t1`'s 7,322 lines among them. The 21st is
//! `lib.t1` — 15 codeless lines declaring no `मण्डलम्`, so it is SKIPPED rather
//! than refused.
//!
//! An earlier draft of this file reported `BUILT 0 of 21`. That was this
//! harness handing `.t1` source straight to the SECOND stage; the control
//! caught it. See `compile` below for the two-stage pipeline.
//!
//! So this test's job is to RECORD THE FRONTIER, not to pass. It prints a row
//! per module and asserts only the floor that has already been measured, so
//! that the number can only go up.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.t1` in the corpus, smallest first — the order to walk a frontier in.
fn modules() -> Vec<(String, String, usize)> {
    let mut v: Vec<(String, String, usize)> = std::fs::read_dir(src_dir())
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("a corpus source is readable");
            let lines = text.lines().count();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, text, lines)
        })
        .collect();
    v.sort_by_key(|(name, _, lines)| (*lines, name.clone()));
    v
}

/// The whole corpus in one interpreter — `पाठवस्तुरचना` reaches most of it.
fn load() -> Interpreter {
    let sources: Vec<(String, String)> = modules()
        .into_iter()
        .map(|(name, text, _)| (name, text))
        .collect();
    let refs: Vec<(&str, &str)> = sources
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    Interpreter::load(&refs, &repo_root().join("spec")).expect("the corpus loads")
}

/// The exit code `पाठवस्तुरचना` leaves behind, by name.
///
/// READ FROM THE INTERPRETER, NOT TRANSCRIBED. The routine resets this to a
/// sentinel on entry precisely so that a caller cannot read the previous
/// call's exit, and a table of constants copied into this file would be a
/// second place for those names to live.
fn exit_reason(it: &Interpreter) -> String {
    // THE NAME IS TRIED IN BOTH FORMS AND THEN LOOKED UP, rather than being
    // asserted. A wrong key answers `None`, which renders identically to a
    // routine that recorded nothing — and on the first run of this file every
    // one of 21 rows read "unreadable" for exactly that reason.
    for key in ["शृङ्खलाॱवस्तुरचनाविरामभेद", "वस्तुरचनाविरामभेद"]
    {
        if let Some(n) = it.global(key).and_then(Value::as_int) {
            return format!("exit {n}");
        }
    }
    let near: Vec<String> = it
        .globals_snapshot()
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| k.contains("विराम"))
        .collect();
    format!("exit (no such global; near: {near:?})")
}

/// The module name a `.t1` file declares — `मण्डलम् <name> ॥` on its first line.
///
/// READ FROM THE FILE, not from its filename. Two files declare module
/// `वास्तु` (`vastu.t1` and `ast.t1`), so the filename is not the module and a
/// table mapping one to the other would be wrong for those two.
fn module_name(text: &str) -> Option<String> {
    let head = text.lines().next()?;
    let rest = head.strip_prefix("मण्डलम्")?;
    rest.split_whitespace().next().map(str::to_string)
}

/// `.t1` source to an object, THROUGH BOTH STAGES.
///
/// THIS IS THE CORRECTION THAT MADE THIS FILE MEAN ANYTHING. The first draft
/// handed `.t1` source straight to `पाठवस्तुरचना` and got `exit 2` — statements
/// parsed, nothing built — on all 21 modules AND on the control. The two are
/// different stages:
///
///   .t1 source --मण्डलसङ्कलनम्--> Sassembly text --पाठवस्तुरचना--> object
///
/// Every rung does exactly this and it is visible in each of them; I read the
/// second call and assumed it was the entry point. Twenty identical refusals
/// were one wrong stage, which is why the control existed.
fn compile(it: &mut Interpreter, text: &str, name: &str) -> (String, Option<usize>) {
    let asm = it.call(
        "शृङ्खलाॱमण्डलसङ्कलनम्",
        vec![
            Value::Octets(Octets::new(text.as_bytes())),
            Value::Octets(Octets::new(name.as_bytes())),
        ],
        200_000_000_000,
    );
    let asm = match asm {
        Ok(Value::Octets(o)) if !o.as_slice().is_empty() => o,
        Ok(_) => {
            let why = it
                .global("शृङ्खलाॱसङ्कलनविरामभेद")
                .and_then(Value::as_int)
                .map_or("?".to_string(), |n| n.to_string());
            return (format!("STAGE1 refused (सङ्कलनविरामभेद {why})"), None);
        }
        Err(e) => return (format!("STAGE1 raised: {}", &e.reason), None),
    };
    let asm_len = asm.as_slice().len();
    match it.call(
        "शृङ्खलाॱपाठवस्तुरचना",
        vec![Value::Octets(asm)],
        200_000_000_000,
    ) {
        Ok(Value::Nil) => (
            format!(
                "STAGE2 refused ({}, {asm_len} octets of asm)",
                exit_reason(it)
            ),
            None,
        ),
        Ok(_) => (format!("built  ({asm_len} octets of asm)"), Some(asm_len)),
        Err(e) => (format!("STAGE2 raised: {}", &e.reason), None),
    }
}

#[test]
#[ignore = "SLOW BY NATURE, NOT BROKEN — 4351s: it runs the .t1 compiler over \
all 38,335 lines of its own corpus, twice per module (both stages), in the \
interpreter. Run it deliberately: `cargo test --test t1_real_module_compile \
-- --ignored --nocapture`. It PASSES at the floor recorded in the header \
(20 of 21 modules, 38,320 lines, 2026-09-20). An ignore is invisible to a \
whole-crate run, so this reason is the only thing that distinguishes a slow \
measurement from a red someone hid — if you find it ignored for any OTHER \
reason, that is the defect."]
fn the_frontier_of_what_the_t1_compiler_compiles() {
    let all = modules();
    let mut built: Vec<(String, usize)> = Vec::new();
    let mut refused: Vec<(String, usize, String)> = Vec::new();

    // THE POSITIVE CONTROL, FIRST. the four-construct batch (`स्वपरीक्षाचतुष्टयम्`) compiles this exact source
    // natively to a 40-octet object, so if the pipeline refuses it here the
    // harness is wrong and every row below is a statement about the harness.
    {
        let mut it = load();
        it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 50_000_000_000)
            .expect("the compiler's tables load");
        let demo = "सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ४२ । इति";
        let (verdict, _) = compile(&mut it, demo, "क");
        println!("\n  CONTROL (the four-construct batch's demo source): {verdict}");
        assert!(
            verdict.starts_with("built"),
            "the control refused ({verdict}); the four-construct batch compiles this source \
             natively to 40 octets, so the frontier table below would be \
             measuring this harness and not the compiler"
        );
    }

    println!("\n  module                 lines   result");
    println!("  ----------------------------------------------------------");
    for (name, text, lines) in &all {
        // A FRESH INTERPRETER PER MODULE. `पाठवस्तुरचना` writes global cursors
        // and resets them on entry, but a module that refuses mid-way leaves
        // arenas it half-filled; carrying that into the next module would make
        // every row after the first refusal a claim about the wrong state.
        let mut it = load();
        // THE SETUP CALL THE RUNGS MAKE. Every rung opens with `सङ्कलनारम्भः`,
        // which fills the spec tables from `सारणी` and reads the name-family
        // and directive tables.
        it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 50_000_000_000)
            .expect("the compiler's tables load");
        let Some(mname) = module_name(text) else {
            println!("  {name:<22} {lines:>5}   SKIPPED  declares no मण्डलम्");
            continue;
        };
        let (verdict, _) = compile(&mut it, text, &mname);
        println!("  {name:<22} {lines:>5}   {verdict}");
        if verdict.starts_with("built") {
            built.push((name.clone(), *lines));
        } else {
            refused.push((name.clone(), *lines, verdict));
        }
    }

    let total_lines: usize = all.iter().map(|(_, _, l)| l).sum();
    let built_lines: usize = built.iter().map(|(_, l)| l).sum();
    println!(
        "\n  BUILT {} of {} module(s); {built_lines} of {total_lines} lines\n",
        built.len(),
        all.len()
    );

    // THE FLOOR, MEASURED 2026-09-20. 20 of 21, 38,320 of 38,335 lines, in
    // 4162s. `lib.t1` is the 21st and is SKIPPED, not refused: 15 lines that
    // declare no `मण्डलम्`, so there is no module to name.
    //
    // This asserts the frontier cannot move BACKWARDS. It deliberately does
    // not assert 21 — a test demanding what has never happened is a red with
    // no information in it.
    assert!(
        built.len() >= 20 && built_lines >= 38_320,
        "the frontier RETREATED: {} module(s) and {built_lines} line(s) built, \
         against a measured floor of 20 and 38320. Refusals:\n{}",
        built.len(),
        refused
            .iter()
            .map(|(n, l, w)| format!("    {n} ({l} lines): {w}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
