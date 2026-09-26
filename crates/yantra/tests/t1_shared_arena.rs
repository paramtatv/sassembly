//! **DOES A GLOBAL ARENA WRITTEN BY ONE MODULE READ BACK IN ANOTHER?**
//!
//! # THE ANSWER, AND A CORRECTION THAT MUST BE READ FIRST
//!
//! **A cross-module read of a global answers 0 where the value is 77, links, runs
//! and finishes CLEANLY.** Measured, with the control that fixes the cause:
//!
//! ```text
//!   अन्यत् declares  सार्वजनिक चरः मान ॱॱ न६४ भवति ७७
//!   अन्यत् reads मान          -> status 77   the initialiser is fine
//!   परीक्षा reads अन्यत्ॱमान  -> status  0   THE CROSS-MODULE READ IS WRONG
//! ```
//!
//! Both arms `Halt::Finisher` with success.
//!
//! ## AND THE CAUSE IS NOT STORAGE. THE READ IS NEVER EMITTED.
//!
//! **This file first reported this as the failure `ir.t1:579` and
//! `yantrotsarjana.t1:2086` name — a per-module copy answering its initialiser.
//! That was WRONG, and the IR refutes it:**
//!
//! ```text
//!   GLOBALS in अन्यत्: [अन्यत्मान=77(8o)]     ONE definition, right label, right value
//!   GLOBALS in परीक्षा: []                     no duplicate, nothing shadowing it
//!   IR अन्यत्  block: [LoadGlobal(SymbolId(1))]   the reader that answers 77
//!   IR परीक्षा block: [ConstInt(0)]               NO LoadGlobal. A CONSTANT.
//! ```
//!
//! **The storage is laid correctly and the cross-module read is lowered to a
//! constant 0.** Not a per-module copy, not a null base, not a namespace
//! collision — **`ir.t1` never emits a load at all.** Link order was varied too
//! and made no difference, which is what a single definition predicts.
//!
//! ### AND IT IS CAUSE 24 — A DELIBERATE REFUSAL, MEASURED, NOT AN ABSENT ARM
//!
//! **The sentence that stood here — "a cross-module GLOBAL has no arm" — was
//! FALSE.** `ir.t1:1705-1726` at `origin/main` (`f73592ec`) has an arm for
//! `व्याकरॱचरघोषणाभेद`, gated on `अर्थॱसंज्ञाघोषणाकोश` being non-`शून्यम्` —
//! "declared in THIS source" — and returning `अपूर्णध्रुवम् २४` when it is not.
//! `.t1` has no raise, so cause 24 lowers to a constant and the image runs clean.
//!
//! **Measured rather than matched**, because inferring it would have repeated the
//! error that produced the false sentence: `अपूर्णध्रुवम्` counts into
//! `अपूर्णगणनाकोश[हेतुः]`, and after compiling this exact pair **cause 24 is raised
//! once and is the ONLY cause that fired.** `ConstInt(0)` on its own is equally
//! consistent with an absent arm, a folded constant and a literal zero.
//!
//! **THE REFUSAL IS LOAD-BEARING. DO NOT LOWER IT.** `ir.t1:1706-1712` records the
//! experiment: lowering cross-module global reads took the corpus **from 15
//! assembled to 2**, the linker naming labels like `सङ्केतनक्षेत्रारम्भः`
//! unresolved. The arm refuses because *"the storage is laid by the declaring
//! module, and that module's object is only linked in if something else pulled it
//! in"*. **So there is no `ir.t1` repair here** — the work is on the side that makes
//! the declaring module's object PRESENT, which is what the gather begins.
//!
//! `artha.t1:2618-2622` records the sibling case for ROUTINES: *"`ir.t1`'s name arm
//! emits a zero-argument CALL only when the symbol's kind is
//! `व्याकरॱवृत्तिघोषणाभेद`, so with the kind absent every cross-module name fell to
//! the Identifier stub."* Same shape, different arm, and the two must not be merged.
//!
//! ### THE ARENA IS THE SAME CAUSE 24 — ONE DEFECT, TWO PRESENTATIONS
//!
//! **This file twice said scalars and arenas "fail by different mechanisms and must
//! not be reported together". That was WRONG, and the counter plus the IR say so:**
//!
//! ```text
//!   scalar   [arena] STUB CAUSE 24 raised 1 time(s)   -- and 24 is the ONLY cause
//!   IR:  [ConstInt(0)]                                 returned directly -> status 0
//!
//!   arena    [arena] STUB CAUSE 24 raised 1 time(s)   -- and 24 is the ONLY cause
//!   IR:  [ConstInt(0) | ConstInt(1) | ConstInt(8) | Mul(v1,v2) | LoadIndex(v0,v3)]
//!        cause 24's constant-0 as the BASE, index scaled 1x8, load at 0+8 -> addr 8
//! ```
//!
//! **The arena does not fail differently. It uses the same stub as an ADDRESS.** A
//! scalar hands the constant back and the program returns 0; an arena adds a
//! correctly-computed offset to it and dereferences. **The index arithmetic is
//! right** — `1×8 = 8`, which is also what `control_index_applied` shows.
//!
//! **So the population is ONE: 730 cross-module global references, one cause.** I
//! had split them into 465 and 265 on the strength of the differing symptom, and a
//! differing symptom is not a differing cause. *Two symptoms, one cause* is the
//! mirror of the trap this file's other margins warn about.
//!
//! **The struct-type row is NOT cause 24** and stays separate: its `ConstInt(0)` is
//! a local record's own `०` initialiser (`[ConstInt(0) | Store | Load |
//! LoadField(v2, 0)]`), which is the known local-record null base, reached here
//! through a cross-module type rather than caused by one.
//!
//! **A cross-module CALL IS NOW FIXED AND RETURNS 77.** It used to fail at LINK
//! with `` `परीक्षासंज्ञा४` is not defined by any object; `अन्यत्संज्ञा४` `` — the
//! symbol minted under the CALLER's module prefix and defined under the CALLEE's,
//! because `chain.rs`'s resolved-symbol arm attributed a foreign member to the
//! calling module. **Ids only compare inside one namespace, and those were two.**
//! It now consults the callee NODE, which `artha.t1:2610` names as the authority:
//! *"a cross-module label from the callee NODE rather than from a SymbolId that
//! means nothing outside the program that minted it"*.
//!
//! **Attributed by control, not assumed:** with the gather alone and the same
//! two-pass harness the call still failed `परीक्षासंज्ञा४`. The node change moved
//! exactly one row and the other three are untouched by it.
//!
//! **The GLOBALS are still broken, and it is ONE defect** — cause 24's stub,
//! returned directly by a scalar and dereferenced as a base by an arena.
//!
//! ## THE CORRECTION: EVERY PROBE BELOW MARKED "BLOCKED" WAS MISSING ONE LINE
//!
//! **T1 has an import declaration — `आयातः <module> ।` — and my fixtures omitted
//! it.** **15 of the 20 corpus sources carry 50 import declarations**, `kosha.t1:40`
//! among them. (An earlier figure here said 72 across 17. It counted LINES rather
//! than occurrences and included margins, and two of those files — `lex.t1` and
//! `lib.t1` — mention `आयातः` only in prose and import nothing. **A prose-mention
//! count reported as a behaviour count.**)
//! Without it a qualified reference refuses at resolve with "has no declaration",
//! which is every refusal the diagnostic probes below report.
//!
//! **I concluded "T1 has no import keyword" from a grep that guessed the wrong
//! inflection** — `आयातम्` where the corpus writes `आयातः`, anchored at line
//! start — so the pattern failed toward EMPTY and the absence read as a fact
//! about the language. Five single-variable probes were then built on it, each
//! correctly refuting a hypothesis that could not have been the cause. **They are
//! kept rather than deleted**, because the sequence is the lesson: a false premise
//! produces probes that work perfectly and answer the wrong question.
//!
//! The bisection is what broke it — the whole of `kosha.t1` reached IR where one
//! of its own routines, lifted verbatim, refused. **A difference that survives
//! every variation of the fixture is in the thing you are copying FROM.**
//!
//! `riscv64.rs:159` states the rule this file tests: a global's storage is ONE
//! object across the image, laid by the module that declares it. The stake is
//! named at `ir.t1:579` and `yantrotsarjana.t1:2086` — if each module instead
//! lays its own copy, every shared global **answers its initialiser for ever**,
//! which is a wrong answer with no fault, no diagnostic and no red.
//!
//! # `ast.t1`'s TWO ARENAS: ANSWERED, AND THE ANSWER IS STRUCTURAL
//!
//! **Nobody lays them, and nobody can.** `build_ir` refuses a source with no
//! routine — `chain.rs:432`, *"the source declares no routine, so there is nothing
//! to build"* — and **`ast.t1` declares ZERO routines. So does `vastu.t1`.** Both
//! files carry `मण्डलम् वास्तु` and neither can produce an object through the
//! chain, so the module that DECLARES `अभिव्यञ्जककोश` and `वाक्यकोश` is never in
//! the image to lay their storage.
//!
//! **That reconciles a contradiction rather than adding to it.** An earlier probe
//! reported `ast.t1` → `object? true`. It calls the object BUILDER on the text
//! directly and never goes through `build_ir`. **Capability, not occurrence, with
//! both code paths now named** — and it is why a hop ladder scored `ast.t1` at hop
//! 5 while the encode census scored it chain-stop IR. Both instruments were right.
//!
//! # THE SUBJECT IS THE MECHANISM, NOT `ast.t1`
//!
//! It was commissioned to settle whether `ast.t1`'s two shared arenas —
//! `अभिव्यञ्जककोश` and `वाक्यकोश`, both `सार्वजनिक चरः … अङ्कः अन्तः … भवति ०`
//! at `:231` and `:232` — get storage laid. **It deliberately does not use them.**
//! Those two are read by five other modules, so a probe built on them cannot
//! separate "the mechanism is broken" from "वास्तु's object is not in the image",
//! and those have different owners and different fixes. Synthetic modules here;
//! `ast.t1` is a second question asked separately.
//!
//! # WHY EVERY READING IS BRACKETED BY A CONTROL
//!
//! **A `0` is uninterpretable on its own**, and — correcting a sentence that stood
//! here — it is NOT the initialiser. The initialiser is 77. A 0 is equally the
//! signature of storage never laid, of a per-module copy, and of an index computed
//! and discarded. Three defects, one number, so each reading has an arm that
//! differs from it in exactly one thing:
//!
//! ```text
//!   control_same_module        write and read in ONE module   -> the mechanism
//!   control_index_applied      write idx १, read idx २        -> the index
//!   ..._global_scalar_...      same decl read from both sides -> WHOSE read is wrong
//! ```
//!
//! **`control_index_applied` exists because a same-index write-then-read passes
//! whether or not the index is applied** — `base+1×8` and `base+0×8` are both "the
//! write I just made" when one element is ever touched. Each assertion in this file
//! was inverted and seen to go red ALONE, the others staying green.
//!
//! # HOW TO READ THE RESULT
//!
//! `Halt::Finisher` decodes `0x3333 | (n << 16)` to `status: Some(n)`; success is
//! `0x5555`, so a returned 0 and a clean finish are the same `Some(0)`.
//!
//! ```text
//!   status 77   correct
//!   status  0   WRONG, and indistinguishable from a successful run
//!   None        did not reach the finisher — a fault, NOT an answer of zero
//!   link error  loud, and the least bad outcome
//! ```
//!
//! **Read `status`, never a process exit code.** `yantra-run` returns
//! `ExitCode::SUCCESS` unconditionally, so `$?` is 0 for every one of them.
//!
//! # WHAT THIS DOES NOT COVER
//!
//! One element at one index, scalars and arenas of `न६४`, two modules, one writer.
//! Not `उ८`, not capacity limits, not records beyond the one type probe. **And
//! nothing here touches `ast.t1`'s own arenas** — that question is still open, and
//! the cross-module read being wrong for a synthetic pair is a reason to expect it
//! is wrong there too, not a measurement of it.

use sadhana::encode::Target;
use sadhana::kosha::LOAD_ADDRESS;
use sadhana::nidana::Language;
use sadhana::t1::chain::Front;
use sadhana::t1::riscv64;
use sadhana::{assemble_object, link_objects, vastu};
use std::path::{Path, PathBuf};
use yantra::Machine;

/// **RAM SIZED FROM THE IMAGE, NOT A CONSTANT.** `W-295`.
///
/// This read `yantra::DEFAULT_RAM` and every fixture that ALLOCATES refused at
/// load, before an instruction ran: `"segment at 0x80000000 needs 536938240
/// bytes and RAM is 20971520 — raise it"`. That is the 512 MiB record region
/// (`e9bb8e77`, sized from a MEASURED whole-corpus high water) plus the image;
/// the region is `॥ स्थानम् ॥` in `ॱरिक्त`, address space with no file bytes, so
/// the only thing that has to move is the loader's RAM argument.
///
/// `yantra::ram_for` is the ONE statement of that sizing — `yantra-run` calls
/// it, `paradigm_encode.rs:1971` calls it — and its margin already records this
/// failure once, against the census's runner: *"two loaders, two answers"*.
/// **Measured 2026-09-17, there were FOUR loaders and three of them were wrong**:
/// this file, `t1_whole_chain.rs` and `t1_storage_witness.rs` each carried the
/// stale constant, which is why a region raise landed green and left reds in
/// three targets that no one attributed to it.
fn ram_for(elf: &[u8]) -> usize {
    yantra::ram_for(elf)
}
const BUDGET: u64 = 200_000_000;

/// The value written. Chosen so that it is not 0, not 1 and not an index, and so
/// a `status` carrying it cannot be confused with a count or an initialiser.
const WRITTEN: i64 = 77;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn assemble(text: &str, name: &str) -> Result<Vec<u8>, String> {
    assemble_object(
        text,
        Some(name),
        Target::Uncompressed,
        false,
        Language::English,
    )
    .map_err(|ds| {
        let named: Vec<String> = ds
            .iter()
            .take(4)
            .map(|d| {
                let line = text.lines().nth(d.line.saturating_sub(1)).unwrap_or("");
                format!("{}: {:?} | {}", d.line, d.reason, line.trim())
            })
            .collect();
        format!("assemble {name}: {}", named.join(" ;; "))
    })
}

/// Compile the given sources and run the linked image.
///
/// # CORRECTED: `Front` CAN COMPILE A MULTI-MODULE PROGRAM — the fixture lacked `आयातः`
///
/// **The heading below is wrong and is kept as the witness.** With `आयातः <module>
/// ।` present, two modules compile, link and run through this function. The gather
/// described below is real and does matter, but not for the reason given: it is
/// what makes the remaining defects VISIBLE.
///
/// **Measured both ways, and this is the whole case for the gather:**
///
/// ```text
///                              no gather     gather      gather + node fix
///   routine call               status 0      LINK ERROR  status 77  CORRECT
///   global scalar read         status 0      status 0     status 0   SILENT, wrong
///   global arena element read  status 0      FAULT @8     FAULT @8   null base
///   struct type use            status 0      FAULT @0     FAULT @0   null base
/// ```
///
/// **Read the columns as two separate landings.** The gather turns three clean
/// zeros into loud failures. The node fix then REPAIRS one of them outright. Only
/// `global scalar read` is silent in all three, and it is the one with a control.
///
/// **THE `routine call` ROW IS NOT ABOUT STORAGE, and an earlier version of this
/// table said it was.** Its link error is `chain.rs:664-694`, where two branches
/// disagree about who owns a foreign symbol:
///
/// ```text
///   666  } else if s > 0 {                              the resolver assigned a symbol
///   671      names.entry(sym).or_insert((module.to_string(), nm));   <- THE CALLER
///   673  } else {                                       s == 0, taken on trust
///   678      self.callee_of_node(node) -> (m, r)                     <- THE CALLEE
/// ```
///
/// `परीक्षासंज्ञा४` is line 671 reproduced exactly — `module.to_string()` plus the
/// `संज्ञा<N>` fallback — while `अन्यत्` defines `अन्यत्संज्ञा४`. **The branch is
/// selected by whether the resolver assigned an id at all**, so the gather routes
/// the reference out of the branch that looks the module up and into the one that
/// assumes it is local. **The gather did not break this; it reached it.**
///
/// **Without it all four are clean successes and all four are wrong.** So the
/// gather is an INSTRUMENT, not a repair: **it does not make the chain more
/// correct, it makes the chain's incorrectness visible.** Expect figures that
/// counted those clean zeros as progress to FALL — a metric can worsen because
/// something started being checked, and this is its cleanest instance.
///
/// # THE SUPERSEDED CLAIM, KEPT AS WRITTEN
///
/// ## `Front` CANNOT COMPILE A MULTI-MODULE PROGRAM, AND THAT IS WHY THE PROBE IS BLOCKED
///
/// A qualified cross-module name resolves through `घोषणासञ्चय`, the shared
/// declaration store. **Filling it takes two calls that `chain.rs` does not
/// contain** — `grep -c 'सङ्ग्रहः\|सञ्चयसिद्धिः'` over `chain.rs` is **0**:
///
/// ```text
///   per source, after its parse:  घोषणासञ्चयॱसङ्ग्रहः   copies its declarations in
///   once, before resolving:       अर्थॱसञ्चयसिद्धिः     lets the resolver consult them
/// ```
///
/// `FRONT_END` **does** list `sanchaya.t1`, so the store is in the image — loaded
/// and inert. Without the gather it stays empty and every qualified reference
/// refuses at resolve with "has no declaration". Re-ordering the phases here (all
/// lexes and parses first, then one resolve) does NOT help and was tried: the
/// store is not filled by parsing, only by the gather.
///
/// `paradigm_encode.rs`'s `load_chain_collected` has both calls, and its margin
/// records this as **W-253** — *"every qualified use was taken on trust"*.
/// **W-253's fix landed in a test-local driver and not in `Front`.**
///
/// # AND THE LANDED CROSS-MODULE PROBE HAS NEVER RUN
///
/// `t1_storage_witness.rs`'s `probe_a_cross_module_record` prints
/// ``STOPPED B resolve: `अन्यत्ॱविषम्` at line 3 has no declaration`` — measured,
/// not inferred. Its margin claims the sources are "compiled through ONE Front so
/// the second sees the first's declarations". It is print-only, so it has reported
/// `ok` on every run since it landed **without once executing a cross-module
/// reference**, and its own margin asserts the mechanism the driver lacks.
///
/// `modules` names them in declaration order; the LAST is the entry module and
/// must define `मुख्यम्`.
fn build_and_run(
    sources: &[(&str, &str)],
    modules: &[&str],
) -> Result<(Option<u32>, String), String> {
    // Entry object first — the order every other probe here uses, kept so their
    // readings stay comparable with each other.
    build_and_run_ordered(sources, modules, true)
}

/// As [`build_and_run`], with the non-startup object order under the caller's
/// control. `entry_first` puts the entry module's object ahead of its dependencies.
///
/// **The order is a parameter because it turned out to be a variable.** A global
/// defined by more than one object is resolved by whichever the linker meets first,
/// so a reading taken at one order is a reading about that order.
fn build_and_run_ordered(
    sources: &[(&str, &str)],
    modules: &[&str],
    entry_first: bool,
) -> Result<(Option<u32>, String), String> {
    let (entry_name, dep_names) = modules
        .split_last()
        .ok_or_else(|| "no modules named".to_string())?;

    let mut front = Front::load(&spec_root()).map_err(|e| format!("front: {e}"))?;

    // **PASS 1 — FILL THE DECLARATION STORE.** Lex and parse every source so each
    // one's declarations are gathered into `घोषणासञ्चय` before anything resolves.
    for (which, src) in sources {
        front.lex(src).map_err(|e| format!("{which} lex: {e}"))?;
        front.parse().map_err(|e| format!("{which} parse: {e}"))?;
    }

    // **PASS 2 — COMPILE EACH SOURCE SEPARATELY AND TAKE ITS MODULE IMMEDIATELY.**
    //
    // Two passes and not one, because `व्याकरॱकार्यक्रमपठनम्` RESETS
    // `घोषणासूचकाङ्क` on entry: every program's declarations occupy `1..n` of the
    // same arena, so only the last-parsed program's are live. A single
    // `build_ir()` after both parses therefore builds IR for the LAST source
    // ALONE, and `module()` for the other one returns a module with no functions
    // whose object exports nothing.
    //
    // **That was measured, not reasoned to**: with one pass the link refused
    // `अन्यत्लेखनम्` — the CORRECT label, the callee's own module and member —
    // because `अन्यत्`'s object contained no routine to define it. A correct label
    // pointing at an empty object looks exactly like a wrong label.
    //
    // So each source is re-lexed and re-parsed here, then resolved, typechecked
    // and lowered on its own, and its `Module` is taken before the next source
    // overwrites the arenas. The store filled in pass 1 persists across this
    // because `सञ्चयसिद्धमस्ति` is one-way and the store is not reset per program.
    // The `Module` type is not publicly nameable, so these rely on inference.
    let mut compiled = Vec::new();
    for ((which, src), name) in sources.iter().zip(modules.iter()) {
        front.lex(src).map_err(|e| format!("{which} re-lex: {e}"))?;
        front
            .parse()
            .map_err(|e| format!("{which} re-parse: {e}"))?;
        front
            .resolve()
            .map_err(|e| format!("{which} resolve: {e}"))?;
        front
            .typecheck()
            .map_err(|e| format!("{which} typecheck: {e}"))?;
        front.build_ir().map_err(|e| format!("{which} ir: {e}"))?;
        let entry = if name == entry_name {
            Some("मुख्यम्")
        } else {
            None
        };
        compiled.push((
            *name,
            front
                .module(name, entry)
                .map_err(|e| format!("module {name}: {e}"))?,
        ));
    }

    let entry_mod = compiled
        .iter()
        .find(|(n, _)| n == entry_name)
        .map(|(_, m)| m)
        .ok_or_else(|| format!("module {entry_name} was not compiled"))?;
    let deps: Vec<_> = dep_names
        .iter()
        .filter_map(|d| compiled.iter().find(|(n, _)| n == d).map(|(_, m)| m))
        .collect();

    let entry_label = entry_mod
        .entry
        .map(|s| riscv64::routine_label(&entry_mod.names, s))
        .transpose()
        .map_err(|r| format!("entry label: {r:?}"))?
        .ok_or_else(|| format!("module {entry_name} names no entry"))?;

    let mut texts = vec![(
        *entry_name,
        riscv64::emit_module(entry_mod).map_err(|r| format!("emit {entry_name}: {r:?}"))?,
    )];
    for (name, m) in dep_names.iter().zip(&deps) {
        texts.push((
            *name,
            riscv64::emit_module(m).map_err(|r| format!("emit {name}: {r:?}"))?,
        ));
    }

    // **WHICH OBJECT DECLARES WHICH GLOBAL, AND WITH WHAT VALUE.** Printed for
    // every build because the whole question is whether two objects declare the
    // same label — and a duplicate is invisible in the halt, the link and the run.
    for (name, m) in std::iter::once((*entry_name, entry_mod))
        .chain(dep_names.iter().zip(&deps).map(|(n, m)| (*n, *m as &_)))
    {
        let listed: Vec<String> = m
            .globals
            .iter()
            .map(|(l, v, o)| format!("{l}={v}({o}o)"))
            .collect();
        println!("    GLOBALS in {name}: [{}]", listed.join(", "));
    }

    // **WHAT THE ENTRY ROUTINE ACTUALLY LOWERED TO.** A cross-module read that
    // answers 0 with a clean finish has two very different explanations — a
    // `LoadGlobal` at a wrong address, or no `LoadGlobal` at all because the name
    // was STUBBED to a constant. The halt cannot tell them apart; the instruction
    // list can, so it is printed rather than inferred.
    for f in &entry_mod.functions {
        for b in f.blocks.values() {
            let kinds: Vec<String> = b.insts.iter().map(|(_, i)| format!("{i:?}")).collect();
            println!("    IR {entry_name} block: [{}]", kinds.join(" | "));
        }
    }

    let records =
        riscv64::module_allocates(entry_mod) || deps.iter().any(|m| riscv64::module_allocates(m));
    let startup = riscv64::emit_startup_object_with_records(Some(&entry_label), records);

    let read = |bytes: &[u8], what: &str| {
        vastu::read(bytes).ok_or_else(|| format!("the {what} object does not read back"))
    };
    // The startup object is always first; only the modules' order varies.
    if !entry_first {
        texts.reverse();
    }
    let mut objects = vec![read(&assemble(&startup, "आरम्भ")?, "startup")?];
    for (name, text) in &texts {
        objects.push(read(&assemble(text, name)?, name)?);
    }

    let elf =
        link_objects(&objects, LOAD_ADDRESS).map_err(|es| format!("link: {}", es.join("; ")))?;
    let mut m = Machine::load_elf(&elf, ram_for(&elf)).map_err(|e| format!("load: {e}"))?;
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(BUDGET, &mut out);
    // The returned value arrives ONLY as the finisher's decoded status. A halt of
    // any other kind yields `None`, which is deliberately distinguishable from
    // `Some(0)`: "it did not finish" and "it answered zero" are different findings
    // and an assertion must not read one as the other.
    let status = match &halt {
        yantra::Halt::Finisher { status, .. } => *status,
        _ => None,
    };
    Ok((
        status,
        format!("{halt:?} (records={records}, objects={})", objects.len()),
    ))
}

/// **WHICH CROSS-MODULE SHAPES RESOLVE AT ALL — a shape census, print-only.**
///
/// Added after the gather went into `chain.rs` and the cross-module probe stayed
/// blocked. **The refusal is the GENERIC "has no declaration", not `artha.t1`'s
/// specific `अमण्डलप्रकारकारण` — "a `मण्डलॱनाम` whose मण्डल the store does not
/// hold"** — so the resolver is not reaching a store lookup and failing it; it is
/// failing before one. That distinction is why this census exists instead of more
/// fuel or another call.
///
/// `artha.t1`'s store-facing machinery is described in terms of `सञ्चयसंज्ञा`,
/// *"THE SYMBOL A CROSS-MODULE MEMBER IS INTERNED UNDER"*, and
/// `प्रकारसंज्ञामण्डलानि`, *"WHOSE type it is"*. **Members and types.** A routine
/// CALL may simply not be a shape it covers — and both the landed probe and my
/// first fixture reached for a call, so neither could tell that apart from the
/// store being empty.
///
/// One `A` declaring a routine, a global scalar, a global arena and a struct;
/// four `B`s each touching exactly one of them. **Each row names the shape and
/// the phase it died in**, so a reader sees which of the four are supported
/// rather than one verdict standing for all.
#[test]
fn measure_which_cross_module_shapes_resolve() {
    let src_a = concat!(
        "मण्डलम् अन्यत् ॥\n",
        "सार्वजनिक संरचना धारकः आरभ्य\n",
        "    प्रथमम् ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "सार्वजनिक चरः मान ॱॱ न६४ भवति ७७ ।\n",
        "सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n",
        "सार्वजनिक वृत्तिः लेखनम् ददाति न६४ आदि\n",
        "    प्रत्यागमनम् ७७ ।\n",
        "इति\n",
    );
    let cases: &[(&str, &str)] = &[
        ("routine call", "    प्रत्यागमनम् अन्यत्ॱलेखनम् ।\n"),
        ("global scalar read", "    प्रत्यागमनम् अन्यत्ॱमान ।\n"),
        (
            "global arena element read",
            "    प्रत्यागमनम् अन्यत्ॱकोशः अङ्कः १ अन्तः ।\n",
        ),
        (
            "struct type use",
            "    चरः ध ॱॱ अन्यत्ॱधारकः भवति ० ।\n    प्रत्यागमनम् ध ॱ प्रथमम् ।\n",
        ),
    ];
    let mut resolved = 0usize;
    for (shape, body) in cases {
        // `आयातः अन्यत् ।` — THE DECLARATION EVERY EARLIER FIXTURE HERE OMITTED.
        // T1 has an import statement; 15 of 20 corpus sources carry 50 of them.
        // Without it a qualified reference refuses at resolve, which is exactly the
        // "has no declaration" every probe above reported.
        let src_b = format!(
            "मण्डलम् परीक्षा ॥\nआयातः अन्यत् ।\n\
             सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n{body}इति\n"
        );
        match build_and_run(&[("A", src_a), ("B", &src_b)], &["अन्यत्", "परीक्षा"])
        {
            Ok((status, how)) => {
                resolved += 1;
                println!("  SHAPE {shape:28} RESOLVED  status={status:?}  {how}");
            }
            Err(e) => println!("  SHAPE {shape:28} REFUSED   {e}"),
        }
    }
    println!(
        "METRIC shared_arena_cross_module_shapes_total {}",
        cases.len()
    );
    println!("METRIC shared_arena_cross_module_shapes_resolved {resolved}");
    // NOT an assertion. A census whose answer is unknown must not carry a number
    // that encodes today's reading as the contract — the whole point is to find
    // out which shapes work, and a guard here would freeze the first answer.
}

/// **THE FALSIFIER FOR THE OBVIOUS READING OF THE CENSUS ABOVE.**
///
/// The shape census says three of four cross-module VALUE shapes refuse at
/// resolve. The tempting inference — that every corpus source making a foreign
/// value reference is blocked — is **refuted by a measurement already in the
/// tree**: `t1_field_offsets.rs` reports `field_offsets_sources_ir_built 19`, so
/// 19 of 20 sources reach IR through a chain with no gather at all. A source
/// blocked at resolve cannot reach IR.
///
/// So a foreign value reference in the corpus does NOT refuse. W-253's margin
/// says what happens instead — *"every qualified use was taken on trust"* — and
/// taken on trust means **accepted and lowered to a stub**, which is the worse
/// failure mode: no refusal, no diagnostic, a stub where a call should be.
///
/// This compiles a real corpus source that makes 18 foreign value references
/// (`kosha.t1` → `अष्टक`, a module `FRONT_END` does not even load) and reports
/// the phase it reaches. **If it resolves, "blocked" is the wrong word for all
/// 1,338 of them and "silently stubbed" is the right one.**
#[test]
fn measure_a_corpus_source_with_foreign_value_references() {
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src/kosha.t1"),
    )
    .expect("kosha.t1 is readable");
    let mut front = Front::load(&spec_root()).expect("the front end loads");
    let phase = (|| -> Result<&'static str, String> {
        front.lex(&src).map_err(|e| format!("lex: {e}"))?;
        front.parse().map_err(|e| format!("parse: {e}"))?;
        front.resolve().map_err(|e| format!("resolve: {e}"))?;
        front.typecheck().map_err(|e| format!("typecheck: {e}"))?;
        front.build_ir().map_err(|e| format!("ir: {e}"))?;
        Ok("IR")
    })();
    match phase {
        Ok(p) => println!(
            "METRIC shared_arena_kosha_reaches {p} — 18 foreign VALUE references to \
             अष्टक, a module FRONT_END does not load, and it did NOT refuse: they \
             were TAKEN ON TRUST, not blocked"
        ),
        Err(e) => println!("METRIC shared_arena_kosha_reaches STOPPED_AT {e}"),
    }
}

/// **THE ASYMMETRY, ISOLATED: ONE SHAPE, ONE VARIABLE — DOES THE MODULE EXIST?**
///
/// `kosha.t1` and my synthetic `B` disagree about the same kind of reference, but
/// they differ in source, shape and module, so the pair cannot say which
/// difference matters. This holds the calling source and the reference shape
/// FIXED and varies only whether the named module was parsed into this `Front`:
///
/// ```text
///   अन्यत्ॱलेखनम्        the module IS parsed here
///   अविद्यमानॱलेखनम्    the module does not exist anywhere
/// ```
///
/// **If the known module refuses and the unknown one reaches IR, the compiler
/// fails HARDER the more it is told** — and the gather I added to `chain.rs`,
/// which registers modules, would convert silent stubs into refusals for every
/// `Front` caller. That is the single most important thing the before-and-after
/// has to measure, so it is isolated here rather than inferred from the pair.
#[test]
fn measure_whether_a_known_module_refuses_where_an_unknown_one_is_trusted() {
    let src_a = concat!(
        "मण्डलम् अन्यत् ॥\n",
        "सार्वजनिक वृत्तिः लेखनम् ददाति न६४ आदि\n",
        "    प्रत्यागमनम् ७७ ।\n",
        "इति\n",
    );
    for (label, module) in [
        ("KNOWN (parsed here)", "अन्यत्"),
        ("UNKNOWN (nowhere)", "अविद्यमान"),
    ] {
        let src_b = format!(
            "मण्डलम् परीक्षा ॥\nसार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    \
             प्रत्यागमनम् {module}ॱलेखनम् ।\nइति\n"
        );
        // Both arms parse `अन्यत्` first, so the ONLY difference between them is
        // which module the reference names. Dropping A from the unknown arm would
        // have varied two things at once.
        let r = build_and_run(&[("A", src_a), ("B", &src_b)], &["अन्यत्", "परीक्षा"]);
        match r {
            Ok((status, how)) => println!("  {label:22} REACHED RUN  status={status:?}  {how}"),
            Err(e) => println!("  {label:22} STOPPED  {e}"),
        }
    }
}

/// **WHAT ACTUALLY SEPARATES `kosha.t1` FROM MY FIXTURE: AN ARGUMENT.**
///
/// Two hypotheses died before this one. It is not the shape of the statement, and
/// it is not whether the module exists — the isolated test above shows a KNOWN and
/// an UNKNOWN module refusing identically.
///
/// What is left is that every foreign call in `kosha.t1` carries an argument:
///
/// ```text
///   kosha.t1:85   भवति अष्टकॱअष्टकयोजनम् नीचम्      one argument  -> reaches IR
///   my fixture    प्रत्यागमनम् अन्यत्ॱलेखनम् ।       none          -> refuses
/// ```
///
/// **A bare qualified name with no argument list has nothing marking it as a
/// call**, so it is plausibly resolved as a NAME — the same path a cross-module
/// global read takes, and the path that refuses. This varies only the argument.
#[test]
fn measure_whether_an_argument_is_what_makes_a_foreign_call_resolve() {
    for (label, decl, call) in [
        (
            "zero-argument call",
            "सार्वजनिक वृत्तिः लेखनम् ददाति न६४ आदि\n    प्रत्यागमनम् ७७ ।\nइति\n",
            "प्रत्यागमनम् अन्यत्ॱलेखनम् ।",
        ),
        (
            "one-argument call",
            "सार्वजनिक वृत्तिः लेखनम् आदाय बीजम् ॱॱ न६४ ददाति न६४ आदि\n    \
             प्रत्यागमनम् बीजम् ।\nइति\n",
            "प्रत्यागमनम् अन्यत्ॱलेखनम् ७७ ।",
        ),
    ] {
        let src_a = format!("मण्डलम् अन्यत् ॥\n{decl}");
        let src_b = format!("मण्डलम् परीक्षा ॥\nसार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    {call}\nइति\n");
        match build_and_run(&[("A", &src_a), ("B", &src_b)], &["अन्यत्", "परीक्षा"])
        {
            Ok((status, how)) => println!("  {label:20} REACHED RUN  status={status:?}  {how}"),
            Err(e) => println!("  {label:20} STOPPED  {e}"),
        }
    }
}

/// **THE VARIABLE I SHOULD HAVE ISOLATED FIRST: HOW MANY SOURCES WENT IN.**
///
/// Three hypotheses about the reference died — its statement shape, whether the
/// named module exists, whether the call carries an argument. **Every one of those
/// tests held "two sources in one `Front`" fixed**, because that was the thing my
/// fixture was for, so it was the one condition never varied.
///
/// `kosha.t1` is compiled ALONE and reaches IR with 18 foreign references. Every
/// fixture of mine parses `A` and then `B`. This compiles a `B` that names a
/// module which does not exist, **once alone and once after an unrelated `A`**,
/// and nothing else differs.
///
/// If the lone arm reaches IR and the paired arm refuses, the defect is not about
/// cross-module references at all: **a second source parsed into one `Front`
/// breaks resolution of that source**, which would explain the landed probe, all
/// four shapes of the census, and both arms of every test above.
#[test]
fn measure_whether_a_second_source_in_one_front_breaks_resolution() {
    // Names a module that exists nowhere, so BOTH arms make an identically
    // hopeless reference and the only difference is how many sources were parsed.
    let src_b = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    प्रत्यागमनम् अविद्यमानॱलेखनम् ।\n",
        "इति\n",
    );
    let src_a = concat!(
        "मण्डलम् अन्यत् ॥\n",
        "सार्वजनिक वृत्तिः लेखनम् ददाति न६४ आदि\n",
        "    प्रत्यागमनम् ७७ ।\n",
        "इति\n",
    );
    let alone = build_and_run(&[("B", src_b)], &["परीक्षा"]);
    let paired = build_and_run(&[("A", src_a), ("B", src_b)], &["अन्यत्", "परीक्षा"]);
    for (label, r) in [("B ALONE", alone), ("A then B", paired)] {
        match r {
            Ok((status, how)) => println!("  {label:10} REACHED RUN  status={status:?}  {how}"),
            Err(e) => println!("  {label:10} STOPPED  {e}"),
        }
    }
}

/// **IS IT THE REFERENCE, OR IS IT `kosha.t1`?**
///
/// Four hypotheses about my fixture died. `kosha.t1` resolves
/// `अष्टकॱअष्टकयोजनम्` — a module `FRONT_END` does NOT load (the list carrying
/// `ashtaka.t1` is a different one) and which no fixture here parses. My minimal
/// source cannot resolve any foreign name at all.
///
/// So the remaining difference is not the statement, the module's existence, the
/// argument, or the source count. **It is either the NAME or the calling file.**
/// This puts kosha's exact reference into my minimal source. If `अष्टक` resolves
/// where `अन्यत्` does not, something privileges that name and every "foreign
/// reference" count in my earlier census is contaminated by it.
#[test]
fn measure_whether_koshas_own_reference_resolves_from_a_minimal_source() {
    for (label, reference) in [
        ("kosha's own name", "अष्टकॱअष्टकयोजनम् ७७"),
        ("my synthetic name", "अन्यत्ॱलेखनम् ७७"),
    ] {
        let src = format!(
            "मण्डलम् परीक्षा ॥\nसार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    \
             प्रत्यागमनम् {reference} ।\nइति\n"
        );
        match build_and_run(&[("B", &src)], &["परीक्षा"]) {
            Ok((status, how)) => println!("  {label:20} REACHED RUN  status={status:?}  {how}"),
            Err(e) => println!("  {label:20} STOPPED  {e}"),
        }
    }
}

/// **THE BISECTION: THE WHOLE FILE, ONE ROUTINE OF IT VERBATIM, AND MY FIXTURE.**
///
/// Five single-variable tests failed to separate `kosha.t1` from my minimal
/// source, and the last one showed kosha's OWN reference refusing when my source
/// makes it. So the difference is somewhere in `kosha.t1` itself, and the way to
/// find it is to cut the file down rather than to keep varying the fixture.
///
/// Three arms, most to least of kosha:
///
/// ```text
///   whole file          20 declarations, 5 foreign references
///   one routine         kosha.t1:78-87 verbatim, 2 foreign references
///   my fixture          one routine, 1 foreign reference
/// ```
///
/// **A middle arm that resolves puts the cause in the routine; one that refuses
/// puts it in what the rest of the file declares.** Either way the next cut is
/// determined rather than guessed.
#[test]
fn measure_the_bisection_between_kosha_and_a_minimal_source() {
    let kosha = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src/kosha.t1"),
    )
    .expect("kosha.t1 is readable");
    // kosha.t1:77-87 verbatim — its header plus the one routine, nothing else.
    let one_routine = concat!(
        "मण्डलम् कोश ॥\n",
        "सार्वजनिक वृत्तिः प्रतिबिम्बद्व्यष्टकम् आदाय मूल्यम् ॱॱ अ६४ ददाति न६४ आदि\n",
        "    चरः नीचम् ॱॱ अ६४ भवति मूल्यम् शेषः २५६ ।\n",
        "    चरः उच्चम् ॱॱ अ६४ भवति मूल्यम् विभाजनम् २५६ ।\n",
        "    उच्चम् भवति उच्चम् शेषः २५६ ।\n",
        "    चरः अवगणना ॱॱ अ६४ भवति अष्टकॱअष्टकयोजनम् नीचम् ।\n",
        "    प्रत्यागमनम् अष्टकॱअष्टकयोजनम् उच्चम् ।\n",
        "इति\n",
    );
    let mine = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    प्रत्यागमनम् अष्टकॱअष्टकयोजनम् ७७ ।\n",
        "इति\n",
    );
    for (label, src) in [
        ("whole kosha.t1", kosha.as_str()),
        ("one routine only", one_routine),
        ("my fixture", mine),
    ] {
        let mut front = Front::load(&spec_root()).expect("the front end loads");
        let r = (|| -> Result<&'static str, String> {
            front.lex(src).map_err(|e| format!("lex: {e}"))?;
            front.parse().map_err(|e| format!("parse: {e}"))?;
            front.resolve().map_err(|e| format!("resolve: {e}"))?;
            front.typecheck().map_err(|e| format!("typecheck: {e}"))?;
            front.build_ir().map_err(|e| format!("ir: {e}"))?;
            Ok("IR")
        })();
        match r {
            Ok(p) => println!("  {label:18} REACHES {p}"),
            Err(e) => println!("  {label:18} STOPPED  {e}"),
        }
    }
}

/// **THE CROSS-MODULE GLOBAL READ ANSWERS 77 — AND THE CONTROL THAT MAKES IT
/// MEAN THAT.** `अन्यत्` declares `सार्वजनिक चरः मान ॱॱ न६४ भवति ७७` and `परीक्षा`
/// reads `अन्यत्ॱमान`. Both arms link, run and finish with the declared value.
///
/// **On its own that reading cannot say WHY**, because "the cross-module read
/// works" and "both arms are reading the same module's storage" produce the same
/// 77. So the same declaration is read from INSIDE its own module too, and the
/// two arms differ in nothing but which module does the reading.
///
/// ```text
///   same module   अन्यत् reads its own मान      -> 77 means the initialiser is fine
///   cross module  परीक्षा reads अन्यत्ॱमान      -> 77 through अन्यत्'s exported label
/// ```
///
/// # THE HISTORY, KEPT BECAUSE THE SEQUENCE IS THE LESSON
///
/// This witness stood at **0** — a silent wrong answer, a stub's constant
/// returned by a clean `Halt::Finisher`. `6e3e03cc` (2026-09-13) lowered the read
/// to a load through the resolver's symbol, and this test then went **red at a
/// third value**: `emit परीक्षा: UnnamedSymbol { symbol: SymbolId(4) }`, because
/// `chain.rs` never named the symbol the new load carries. **The pin caught it
/// exactly as its margin promised** — the `other =>` arm below is why the failure
/// had a sentence rather than a number. `W-296` supplied the missing loop and the
/// value the declaration states now arrives.
///
/// **A silent 0 and a loud refusal are not the same distance from correct**, and
/// this file's own reading of the middle state took two cycles to date, so the
/// three values are named in the assertion below and not just in prose.
#[test]
fn measure_a_cross_module_global_scalar_against_its_own_module() {
    const DECL: &str = "सार्वजनिक चरः मान ॱॱ न६४ भवति ७७ ।\n";
    // Arm 1: the declaring module is also the entry, and reads its own global.
    let same = format!(
        "मण्डलम् अन्यत् ॥\n{DECL}\
         सार्वजनिक वृत्तिः अकर्म ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    प्रत्यागमनम् मान ।\nइति\n"
    );
    // Arm 2: the SAME declaration, read from a second module that imports it.
    //
    // **`अकर्म` IS NOT A FIXTURE QUIRK — IT IS THE ONLY WAY A GLOBALS-ONLY MODULE
    // CAN PRODUCE AN OBJECT.** `build_ir` refuses a source with no routine
    // (`chain.rs:432`, "the source declares no routine, so there is nothing to
    // build"), and a module declaring only globals is exactly that. So a shared
    // global cannot get storage unless its declaring module ALSO declares at least
    // one routine, called or not.
    //
    // **This is documented nowhere else, and it is the mechanism behind `ast.t1`'s
    // arenas having no storage**: that file declares zero routines, so it never
    // produces an object, so nothing lays what it declares. A never-called routine
    // is the difference between a module that exists in the image and one that does
    // not.
    //
    // **Both arms carry it**, so the arms still differ in one thing only.
    let a =
        format!("मण्डलम् अन्यत् ॥\n{DECL}सार्वजनिक वृत्तिः अकर्म ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n");
    let b = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "आयातः अन्यत् ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    प्रत्यागमनम् अन्यत्ॱमान ।\n",
        "इति\n",
    );
    let own = build_and_run(&[("A", &same)], &["अन्यत्"]);
    let foreign = build_and_run(&[("A", &a), ("B", b)], &["अन्यत्", "परीक्षा"]);
    let mut own_status = None;
    let mut foreign_status = None;
    for (label, r, slot) in [
        ("read by अन्यत्", own, &mut own_status),
        ("read by परीक्षा", foreign, &mut foreign_status),
    ] {
        match r {
            Ok((status, how)) => {
                *slot = status;
                println!("  {label:16} status={status:?}  {how}");
            }
            Err(e) => println!("  {label:16} STOPPED  {e}"),
        }
    }
    println!(
        "  77 from BOTH = the declaring module's storage is what the reader reaches; \
         0 from arm 2 = a stub's constant returned silently; anything else = neither"
    );

    // **THE CONTROL IS THE CONTRACT AND IS ASSERTED.** A module reading its own
    // global scalar must answer 77; if that breaks, arm 2's reading means nothing.
    assert_eq!(
        own_status,
        Some(u32::try_from(WRITTEN).unwrap()),
        "a module read its OWN global scalar, initialised {WRITTEN}, as {own_status:?} \
         — the initialiser does not reach storage even within one module, and the \
         cross-module arm below is measuring that instead"
    );

    // **NOW ASSERTED AS CORRECT, WHICH IT WAS NOT ALLOWED TO BE UNTIL `W-296`.**
    // The `Some(0)` arm is kept as a NAMED regression rather than folded into the
    // catch-all, because 0 and a refusal are different failures with different
    // owners: 0 means the read was stubbed again (`ir.t1`'s cause arm), and a
    // `None` means it never got that far (`chain.rs`'s name table, or the link).
    // A single `assert_eq!` would report both as "expected 77".
    match foreign_status {
        Some(w) if w == u32::try_from(WRITTEN).unwrap() => {
            println!("  cross-module read answers {WRITTEN}, the value अन्यत् declares");
        }
        Some(0) => panic!(
            "the cross-module read answered 0 where {WRITTEN} is declared. That is \
             the SILENT wrong answer this file was opened for, retired by \
             `6e3e03cc` (2026-09-13): the read is being stubbed to a constant \
             again rather than lowered to a load through its symbol. Start at \
             `measure_that_a_cross_module_global_read_lowers_and_is_not_stubbed`, \
             which reads the two counters that say which"
        ),
        other => panic!(
            "the cross-module read answered {other:?}, neither the correct \
             {WRITTEN} nor the retired 0. `None` means it never ran — the arm \
             above prints the stage. The one measured this way was \
             `emit परीक्षा: UnnamedSymbol {{ symbol: SymbolId(4) }}`: the load \
             carries the resolver's symbol and `chain.rs` did not name it, which \
             is `W-296`"
        ),
    }
}

/// **IS THE CROSS-MODULE GLOBAL MULTIPLY DEFINED? VARY ONLY THE LINK ORDER.**
///
/// `chain.rs`'s globals loop pushes a definition for **every** global in the IR's
/// arena, foreign ones included, labelled `{declaring module}{name}`:
///
/// ```text
///   globals.push((format!("{m}{n}"), arena_int(&g_values, i) as i64, …))
/// ```
///
/// So if `परीक्षा`'s compilation also carries `अन्यत्`'s global, **two objects
/// define `अन्यत्मान`** — `अन्यत्`'s with 77, `परीक्षा`'s with whatever its own
/// arena holds. The linker then takes one, and nothing says which.
///
/// **This varies the link order and NOTHING else.** If the answer changes with the
/// order, the symbol is multiply defined and the value a program reads is decided
/// by object sequence — *a global symbol needs an owner; defined by every object,
/// then by none.* If the answer is 0 both ways, the duplicate is not the cause and
/// this rules it out rather than leaving it as a story.
#[test]
fn measure_whether_the_link_order_decides_a_cross_module_globals_value() {
    let a = concat!(
        "मण्डलम् अन्यत् ॥\n",
        "सार्वजनिक चरः मान ॱॱ न६४ भवति ७७ ।\n",
        "सार्वजनिक वृत्तिः अकर्म ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n",
    );
    let b = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "आयातः अन्यत् ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    प्रत्यागमनम् अन्यत्ॱमान ।\n",
        "इति\n",
    );
    for (label, entry_first) in [("entry object FIRST", true), ("declarer FIRST", false)] {
        match build_and_run_ordered(&[("A", a), ("B", b)], &["अन्यत्", "परीक्षा"], entry_first)
        {
            Ok((status, how)) => println!("  {label:20} status={status:?}  {how}"),
            Err(e) => println!("  {label:20} STOPPED  {e}"),
        }
    }
    println!("  DIFFERENT answers = `अन्यत्मान` is DEFINED TWICE and the linker picks by order");
}

/// **THE CROSS-MODULE GLOBAL READ IS NO LONGER REFUSED — DATED, NOT DISCOVERED.**
///
/// This test used to require cause 24 (later 44) to fire for a cross-module global
/// read, on the strength of `ir.t1`'s arm for `व्याकरॱचरघोषणाभेद` gated on the
/// declaration being *"declared in THIS source"*. **That contract was retired on
/// purpose on 2026-09-13 by `6e3e03cc`, "the lowering half of वास्तु"**, and its
/// own commit message says why and by how much:
///
/// ```text
///   name_global_declsite_zero 77 -> 0, lowered global_load 128 -> 205 (+77, exact)
/// ```
///
/// `crates/yantra/tests/paradigm/pins.rs:613` carries the corpus figure —
/// **cause 44 went 667 -> 0** — so the `[]` this test reported was not drift
/// nobody chose. `ir.t1:2494-2502` states the surviving guard: the type is looked
/// up FIRST and 44 is raised only when the checker has no type for the symbol
/// either. The read itself lowers to `वैश्विकाज्ञाभेद` carrying the resolver's
/// symbol, counted as shape 26 at `ir.t1:2559`.
///
/// **SO THE SUBJECT IS REPLACED RATHER THAN RE-PINNED.** Re-pinning the number
/// that fires today would enshrine the same kind of claim that broke here: a
/// cause count read on its own. **The assertion is now the JUNCTION — refused or
/// lowered — and it reads BOTH counters**, because `[]` from the cause arena is
/// equally consistent with "lowered", "deleted" and "the counter moved", and only
/// the shape arena separates those three. See `stub_causes_for`'s own margin.
///
/// A raw `Interpreter` rather than `Front`, because `Front` exposes no global
/// accessor and both counters live in the interpreter's own state.
#[test]
fn measure_that_a_cross_module_global_read_lowers_and_is_not_stubbed() {
    let (fired, shapes) = stub_causes_for(
        "मण्डलम् अन्यत् ॥\nसार्वजनिक चरः मान ॱॱ न६४ भवति ७७ ।\n\
         सार्वजनिक वृत्तिः अकर्म ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n",
        "मण्डलम् परीक्षा ॥\nआयातः अन्यत् ।\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    प्रत्यागमनम् अन्यत्ॱमान ।\nइति\n",
        "scalar",
    );
    // The reading module's whole body is the one cross-module read, and
    // `मध्यरूपॱआरम्भः` zeroes both counters immediately before it is lowered
    // (`ir.t1:869-878`), so every number below is about that ONE expression.
    const GLOBAL_LOAD_SHAPE: usize = 26;
    const PRE_SPLIT: usize = 24;
    const POST_SPLIT: usize = 44;
    let refused: Counts = fired
        .iter()
        .copied()
        .filter(|(c, k)| (*c == PRE_SPLIT || *c == POST_SPLIT) && *k > 0)
        .collect();
    let lowered = shapes
        .iter()
        .find(|(s, _)| *s == GLOBAL_LOAD_SHAPE)
        .map(|(_, k)| *k);
    println!("METRIC shared_arena_guard_a_cause {refused:?}");
    println!("METRIC shared_arena_global_load_shape {lowered:?}");

    // **THE ACCEPTED CASE.** The read reaches the emitter as a load through its
    // symbol, exactly once, and neither retired cause fires with it.
    assert_eq!(
        lowered,
        Some(1),
        "a cross-module global read must lower to shape {GLOBAL_LOAD_SHAPE} \
         (`ir.t1:2559`, `वैश्विकाज्ञाभेद` carrying the resolver's symbol) exactly \
         once, and the shape ledger says {shapes:?}. If the number MOVED, this \
         constant is stale and the fix is here; if the shape is ABSENT while \
         causes {refused:?} fired, `6e3e03cc` has been reverted; if BOTH are \
         empty the expression lowered to nothing at all, which is the finding \
         this pair exists to separate"
    );
    assert!(
        refused.is_empty(),
        "cause {refused:?} fired for a cross-module global read that also lowered \
         ({lowered:?} of shape {GLOBAL_LOAD_SHAPE}). `6e3e03cc` retired that \
         refusal for a read the checker can type — corpus-wide 667 -> 0, \
         `paradigm/pins.rs:613` — so a raise beside a lowering means the arm now \
         does both and one of the two is unreachable code. All causes: {fired:?}"
    );

    // **AND THE CASE THAT MUST STILL BE REFUSED, IN THE SAME INSTRUMENT.**
    // `अन्यत्` declares `अन्यमान` and the reader asks for `अन्यत्ॱमान`, which no
    // module declares. `6e3e03cc` lowered the read of a member the store KNOWS;
    // a member it does not know must still reach no load, because there is no
    // storage for the linker to resolve against and the label would be invented
    // — which is `W-279`'s `परीक्षासंज्ञा४` against `अन्यत्संज्ञा४` exactly.
    //
    // **THE CAUSE IS 1 AND THAT WAS MEASURED, NOT PREDICTED.** I expected 44 —
    // `ir.t1:2494-2502`'s surviving guard, "declaration site ० and the checker
    // has no type either" — and the instrument answered `[(1, 1)]` with
    // `resolve verdict: Ok(Bool(false))`. The resolver refuses the name first,
    // so `ir.t1`'s `नामभेदः समम् ०` arm (`:2610`, the Identifier stub) takes it
    // and cause 44 is never reached. **So this arm does not assert a NUMBER**,
    // which would be a second guess dressed as a pin: it asserts that the read
    // left a trace in ONE of the two ledgers and not the other.
    //
    // **This arm is what makes the arm above mean something.** Without it
    // "0 causes" would pass for a build in which the guard had been deleted
    // outright, which is precisely the reading this test was rewritten from.
    let (untyped_fired, untyped_shapes) = stub_causes_for(
        "मण्डलम् अन्यत् ॥\nसार्वजनिक चरः अन्यमान ॱॱ न६४ भवति ७७ ।\n\
         सार्वजनिक वृत्तिः अकर्म ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n",
        "मण्डलम् परीक्षा ॥\nआयातः अन्यत् ।\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    प्रत्यागमनम् अन्यत्ॱमान ।\nइति\n",
        "undeclared-member",
    );
    let untyped_lowered = untyped_shapes
        .iter()
        .find(|(s, _)| *s == GLOBAL_LOAD_SHAPE)
        .map(|(_, k)| *k);
    println!("METRIC shared_arena_untyped_member_causes {untyped_fired:?}");
    println!("METRIC shared_arena_untyped_member_global_load {untyped_lowered:?}");
    assert_eq!(
        untyped_lowered, None,
        "a qualified read of a member `अन्यत्` does not declare must NOT reach \
         shape {GLOBAL_LOAD_SHAPE}: there is no storage for the linker to resolve \
         against and the label would be invented. Shapes: {untyped_shapes:?}"
    );
    assert!(
        !untyped_fired.is_empty(),
        "a qualified read of a member `अन्यत्` does not declare lowered to \
         NOTHING and raised NO cause — it neither emitted a load nor was counted \
         as a stub, so it left no trace in either ledger. That is the silent \
         third state both arenas are read to rule out"
    );
}

/// **THE ARENA ROW, WHICH IS A THIRD DEFECT AND NOT CAUSE 24.**
///
/// A cross-module ARENA element read faults at `addr 8`, so it DOES emit an
/// address computation — `base + 1×8` against a base of zero. **A stub lowers to a
/// constant and cannot fault**, so whatever this is, it is not the scalar's
/// deliberate refusal.
///
/// Two possibilities and they have different owners: the arena path stubs under a
/// DIFFERENT cause, or it lowers a real address against storage that is absent.
/// **The counter separates them.** No assertion — the answer is not known, and a
/// guard here would freeze whichever one today gives.
#[test]
fn measure_which_cause_a_cross_module_arena_read_raises() {
    let (fired, shapes) = stub_causes_for(
        "मण्डलम् अन्यत् ॥\nसार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n\
         सार्वजनिक वृत्तिः अकर्म ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n",
        "मण्डलम् परीक्षा ॥\nआयातः अन्यत् ।\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    \
         प्रत्यागमनम् अन्यत्ॱकोशः अङ्कः १ अन्तः ।\nइति\n",
        "arena",
    );
    println!("METRIC shared_arena_arena_causes_fired {}", fired.len());
    println!("METRIC shared_arena_arena_shapes_lowered {}", shapes.len());
    println!(
        "  NO cause fired = a real address computation against absent storage; \
         a cause = a stub, and its number says which arm refused. The SHAPES say \
         which of those two an empty cause list means, which the count alone \
         cannot: see `stub_causes_for`"
    );
}

/// **THE LOWERING WITNESS: `ashtaka.t1`'s ONE `assign_name` SITE.**
///
/// `ashtaka.t1:56` is `अष्टकदोषनाम भवति नाम ।` — a bare-name assignment to a
/// RUN-typed global. Before the fix, `ir.t1`'s global-store arm tested
/// `पूर्णाङ्क` ALONE while the read arm accepted `पूर्णाङ्क` OR `खण्ड`, so this
/// could be read and not written and fell to `अपूर्णवाक्यम् २२`.
///
/// **One source, compiled in ~1s, so the arm can be iterated without the 31-minute
/// census.** `ashtaka.t1` was chosen because it has exactly ONE such site — found
/// by scanning all twenty sources for bare-name assignments to run-typed globals,
/// which gives 113 syntactic sites and exactly 1 in `ashtaka.t1`, agreeing with the
/// trunk's by-cause count of `assign_name 1` for that source.
///
/// **This is NOT the census and must not be quoted as one.** One source of twenty.
/// It shows the arm fires; it says nothing about the corpus-wide 44.
#[test]
fn measure_the_assign_name_lowering_on_ashtaka() {
    let (causes, shapes) = compile_one_source("ashtaka.t1");
    let c22 = causes.iter().find(|(c, _)| *c == 22).map(|(_, k)| *k);
    let s29 = shapes.iter().find(|(s, _)| *s == 29).map(|(_, k)| *k);
    for (c, k) in &causes {
        println!("  ashtaka STUB CAUSE {c} x{k}");
    }
    println!("METRIC ashtaka_cause_22_assign_name {c22:?}");
    println!("METRIC ashtaka_shape_29_assign_store_global {s29:?}");
    println!(
        "  BEFORE the fix: cause 22 = Some(1), shape 29 absent. \
         AFTER: cause 22 absent, shape 29 = Some(1). One site moves, not two."
    );
}

/// `(number, count)` for every non-zero entry of a counter arena.
type Counts = Vec<(usize, i128)>;

/// Compile ONE corpus source and return `(stub causes, lowered shapes)`.
fn compile_one_source(name: &str) -> (Counts, Counts) {
    use sadhana::t1::nirvahana::{Interpreter, Octets, Value};

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src");
    let chain = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "ir.t1",
    ];
    let texts: Vec<(String, String)> = chain
        .iter()
        .map(|n| {
            (
                (*n).to_string(),
                std::fs::read_to_string(dir.join(n)).unwrap_or_else(|e| panic!("{n}: {e}")),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("the front end loads");
    let src = std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));

    let toks = it
        .call(
            "पदविभागॱपदविभाग",
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            2_000_000_000,
        )
        .expect("lex");
    let decls = it
        .call(
            "व्याकरॱकार्यक्रमपठनम्",
            vec![Value::Int(toks.as_int().unwrap_or(0))],
            4_000_000_000,
        )
        .expect("parse")
        .as_int()
        .unwrap_or(0);
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver init");
    let v = it.call(
        "अर्थॱकार्यक्रमनिर्णयः",
        vec![r.clone(), Value::Int(decls)],
        4_000_000_000,
    );
    assert_eq!(
        v,
        Ok(Value::Bool(true)),
        "{name} must RESOLVE for its stub counts to mean anything; got {v:?}"
    );
    let _ = it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000);
    let _ = it.call(
        "अर्थॱकार्यक्रमप्रकारपरीक्षा",
        vec![Value::Int(decls)],
        4_000_000_000,
    );
    it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000).expect("ir init");
    let built = it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(decls)], 4_000_000_000);
    assert!(
        built.is_ok(),
        "{name} must reach IR for its counts to mean anything; got {built:?}"
    );

    let read = |it: &mut Interpreter, g: &str| -> Counts {
        let Some(Value::Arena(a)) = it.global(g).cloned() else {
            panic!("{g} is not an arena — the counter this rests on is absent");
        };
        let n = a.borrow().len();
        let mut out = Vec::new();
        for i in 0..n {
            if let Some(v) = a.borrow().get(i).and_then(Value::as_int)
                && v > 0
            {
                out.push((i, v));
            }
        }
        out
    };
    let causes = read(&mut it, "अपूर्णगणनाकोश");
    let shapes = read(&mut it, "रचितगणनाकोश");
    (causes, shapes)
}

/// Compile a two-module pair through a raw `Interpreter` and return every stub
/// cause that fired, as `(cause, count)`.
///
/// A raw `Interpreter` rather than `Front` because `Front` exposes no global
/// accessor and `अपूर्णगणनाकोश` lives in the interpreter's own state.
fn stub_causes_for(a: &str, b: &str, label: &str) -> (Counts, Counts) {
    use sadhana::t1::nirvahana::{Interpreter, Octets, Value};

    // The same seven sources `Front` loads, read from disk so this file states its
    // own chain rather than depending on a private const.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src");
    let names = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "ir.t1",
    ];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| {
            (
                (*n).to_string(),
                std::fs::read_to_string(dir.join(n)).unwrap_or_else(|e| panic!("{n}: {e}")),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("the front end loads");

    let parse = |it: &mut Interpreter, src: &str| -> i128 {
        let t = it
            .call(
                "पदविभागॱपदविभाग",
                vec![Value::Octets(Octets::new(src.as_bytes()))],
                2_000_000_000,
            )
            .expect("lex");
        let d = it
            .call(
                "व्याकरॱकार्यक्रमपठनम्",
                vec![Value::Int(t.as_int().unwrap_or(0))],
                4_000_000_000,
            )
            .expect("parse");
        let _ = it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 4_000_000_000);
        d.as_int().unwrap_or(0)
    };
    parse(&mut it, a);
    parse(&mut it, b);
    it.call("अर्थॱसञ्चयसिद्धिः", vec![], 5_000_000)
        .expect("सञ्चयसिद्धिः");
    let decls = parse(&mut it, b);

    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver init");
    let verdict = it.call(
        "अर्थॱकार्यक्रमनिर्णयः",
        vec![r.clone(), Value::Int(decls)],
        4_000_000_000,
    );
    println!("  resolve verdict: {verdict:?}");
    let _ = it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000);
    let _ = it.call(
        "अर्थॱकार्यक्रमप्रकारपरीक्षा",
        vec![Value::Int(decls)],
        4_000_000_000,
    );
    it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000).expect("ir init");
    let built = it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(decls)], 4_000_000_000);
    println!("  ir build: {built:?}");

    // EVERY non-zero cause, not just 24 — a table showing only the cause I expect
    // cannot show me that a different one fired instead.
    let Some(Value::Arena(counts)) = it.global("अपूर्णगणनाकोश").cloned()
    else {
        panic!("अपूर्णगणनाकोश is not an arena — the counter this claim rests on is absent");
    };
    let n = counts.borrow().len();
    let mut fired: Vec<(usize, i128)> = Vec::new();
    for i in 0..n {
        if let Some(v) = counts.borrow().get(i).and_then(Value::as_int)
            && v > 0
        {
            fired.push((i, v));
        }
    }
    for (cause, k) in &fired {
        println!("  [{label}] STUB CAUSE {cause} raised {k} time(s)");
    }
    println!(
        "METRIC shared_arena_stub_causes_fired_{label} {}",
        fired.len()
    );

    // **W-296 — AND THE SHAPES, BECAUSE A COUNTER WITH ONE HALF OF THE LEDGER
    // REPORTS A RETIREMENT AS A DISAPPEARANCE.** This helper used to answer the
    // stub causes ALONE, so when `6e3e03cc` stopped stubbing the cross-module
    // read it returned `[]` — and `[]` is what it would also answer if the arm
    // had been deleted, if the build had failed, or if the counter itself had
    // been renamed. *An instrument with two states where the truth has three
    // hides its own breakage.* `रचितगणनम्` counts the OTHER side of the same
    // junction — every lowered shape — and the pair separates all three:
    //
    // ```text
    //   causes=[24|44]  shapes without 26   the read is REFUSED   (pre-6e3e03cc)
    //   causes=[]       shapes with 26      the read is LOWERED   (today)
    //   causes=[]       shapes without 26   the read VANISHED     (a finding)
    // ```
    let shapes: Counts = match it.global("रचितगणनाकोश").cloned() {
        Some(Value::Arena(counts)) => {
            let n = counts.borrow().len();
            (0..n)
                .filter_map(|i| {
                    counts
                        .borrow()
                        .get(i)
                        .and_then(Value::as_int)
                        .filter(|v| *v > 0)
                        .map(|v| (i, v))
                })
                .collect()
        }
        _ => panic!("रचितगणनाकोश is not an arena — the lowering half of this reading is absent"),
    };
    for (shape, k) in &shapes {
        println!("  [{label}] LOWERED SHAPE {shape} x{k}");
    }
    println!(
        "METRIC shared_arena_lowered_shapes_{label} {}",
        shapes.len()
    );
    (fired, shapes)
}

/// **CONTROL 1 — ONE MODULE WRITES AND READS ITS OWN GLOBAL ARENA.**
///
/// No cross-module anything: one object, one declarer, one reader. If this does
/// not answer 77 then global arena element access is broken *within* a module and
/// the cross-module probe below is measuring that instead of the one-object rule.
/// **Read this one first; it conditions the other two.**
#[test]
fn control_one_module_writes_then_reads_its_own_arena() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    कोशः अङ्कः १ अन्तः भवति ७७ ।\n",
        "    प्रत्यागमनम् कोशः अङ्कः १ अन्तः ।\n",
        "इति\n",
    );
    let (status, how) = build_and_run(&[("P", src)], &["परीक्षा"])
        .unwrap_or_else(|e| panic!("the single-module control must compile and run: {e}"));
    println!("METRIC shared_arena_control_same_module {how}");
    assert_eq!(
        status,
        Some(u32::try_from(WRITTEN).unwrap()),
        "a module wrote {WRITTEN} into its OWN global arena at index १ and read \
         index १ back as {status:?}. `None` means it never reached the finisher; \
         `Some(0)` means the arena answered its initialiser, so global arena \
         element storage is broken WITHIN one module and the cross-module probe \
         measures that instead of the one-object rule"
    );
}

/// **THE PROBE — MODULE A WRITES, MODULE B READS.**
///
/// `अन्यत्` declares the arena and writes `WRITTEN` at index १. `परीक्षा` calls
/// that writer, then reads index १ **through a qualified name**, and returns what
/// it read. The two modules are separate objects and the symbol must resolve
/// across them.
///
/// **The three outcomes were registered before this ran, each with its own cause:**
///
/// ```text
///   status 77   the read sees the write     -> one object, rule holds
///   status  0   the read sees ०             -> PER-MODULE COPIES
///   link error  symbol undefined            -> nobody laid the storage
/// ```
///
/// **Predicted: 77**, on the ground that `fb038aee` is an ancestor of
/// `origin/main` and derives a global run's capacity from
/// `मध्यरूपॱवैश्विकसामर्थ्यकोश`, so the storage exists to be shared. That is a
/// prediction about THIS shape only, and control 3 below is what keeps a pass
/// from being read as more than it is.
#[test]
fn probe_one_module_writes_and_another_reads() {
    let src_a = concat!(
        "मण्डलम् अन्यत् ॥\n",
        "सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n",
        "सार्वजनिक वृत्तिः लेखनम् ददाति न६४ आदि\n",
        "    कोशः अङ्कः १ अन्तः भवति ७७ ।\n",
        "    प्रत्यागमनम् ० ।\n",
        "इति\n",
    );
    let src_b = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "आयातः अन्यत् ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः उपेक्ष्यम् ॱॱ न६४ भवति अन्यत्ॱलेखनम् ।\n",
        "    प्रत्यागमनम् अन्यत्ॱकोशः अङ्कः १ अन्तः ।\n",
        "इति\n",
    );
    match build_and_run(&[("A", src_a), ("B", src_b)], &["अन्यत्", "परीक्षा"])
    {
        Ok((status, how)) => {
            println!("METRIC shared_arena_probe_cross_module {how}");
            println!("METRIC shared_arena_probe_cross_module_status {status:?}");
            println!("  UNBLOCKED — status {WRITTEN} = one object · 0 = PER-MODULE COPIES");
        }
        // **NOT AN ASSERTION, AND NOT A PASS EITHER.** The measurement is blocked
        // upstream of its subject: `Front` never fills `घोषणासञ्चय`, so this stops
        // at RESOLVE and never reaches storage at all. Asserting anything here
        // would be asserting about the driver while claiming to be about globals.
        //
        // It prints BLOCKED rather than STOPPED so that a reader scanning output
        // cannot mistake it for a measured negative — an absent answer and an
        // answer of zero are the two readings this whole file exists to separate.
        Err(e) => {
            println!("METRIC shared_arena_probe_cross_module BLOCKED {e}");
            println!(
                "  BLOCKED UPSTREAM OF THE SUBJECT: the driver cannot compile two \
                 modules, so the one-object rule is UNTESTED — not refuted, not \
                 confirmed. See this file's `build_and_run` margin for the two \
                 missing calls."
            );
        }
    }
}

/// **CONTROL 3 — THE INDEX'S POSITIVE CONTROL.**
///
/// Writes `WRITTEN` at index १ and reads index २. **A read of 77 here means the
/// index is computed and discarded**, which is defect 2's signature and would
/// make control 1's pass vacuous: if every index lands on the same address, a
/// write-then-read at one index cannot fail.
///
/// Expects 0 — and 0 is the INTERESTING answer for once, because it is the only
/// one under which control 1 measured what it claims to.
#[test]
fn control_a_write_at_one_index_is_not_read_at_another() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    कोशः अङ्कः १ अन्तः भवति ७७ ।\n",
        "    प्रत्यागमनम् कोशः अङ्कः २ अन्तः ।\n",
        "इति\n",
    );
    let (status, how) = build_and_run(&[("P", src)], &["परीक्षा"])
        .unwrap_or_else(|e| panic!("the index control must compile and run: {e}"));
    println!("METRIC shared_arena_control_index_applied {how}");
    assert_eq!(
        status,
        Some(0),
        "a write at index १ was read back at index २ as {status:?}. \
         `Some({WRITTEN})` means THE INDEX IS COMPUTED AND DISCARDED — every \
         element aliasing one address — which would make the same-index control \
         above pass vacuously, since a write-then-read at one index cannot fail \
         when there is only ever one address"
    );
}
