//! `W-237` — EMITTER LANE R3: THE ENCODE CENSUS, research/25 §4 — the last stage of the
//! owner's definition of done, "Sassembly is done when the T1 chain self-hosts:
//! lex → parse → resolve → typecheck → encode".
//!
//! # What is measured, on what
//!
//! **The corpus is the 18 `.t1` sources under `crates/sadhana-t1/src`** — the self-hosting
//! corpus, the one the T1 chain reads (research/25 §1.4). For each one, under the
//! interpreter: `पदविभाग ॱ पदविभाग` → `व्याकर ॱ कार्यक्रमपठनम्` → `अर्थ ॱ कार्यक्रमनिर्णयः`
//! (resolve) → `अर्थ ॱ कार्यक्रमप्रकारपरीक्षा` (typecheck) → `मध्यरूप ॱ कार्यक्रमरचना` (IR);
//! then the IR's arenas are read into `riscv64::Module` and emitted by BOTH twins —
//! `riscv64::emit_module` and `यन्त्रोत्सर्जन ॱ यन्त्रमण्डलोत्सर्जनम्` — and the two texts
//! compared octet for octet; then `सङ्केतन` assembles the text (`sadhana::assemble_object`),
//! `समयोजन` links it at `0x8000_0000`, `yantra` loads it with `yantra-run`'s 1 MiB and runs
//! it for `yantra-run`'s million steps, and the halt is the answer. Every word of the linked
//! text is decoded back through `W-211`'s `decode_at`/`reassemble` (the round trip).
//!
//! **One line per source says where its chain stopped and why** — `lex`, `parse`, `resolve`,
//! `typecheck`, `IR`, `emit`, `assemble`, `link`, `load`, or `run` with the halt's kind. Two
//! stops are printed, because the checker and the emitter are two verdicts on one source:
//! `chain` is the first stage of the owner's chain that refused (the checker gates it), and
//! `encode` is where the emitter's side stopped on the IR the builder made. A source that
//! assembles but halts on a fault is COUNTED at the run stage — `RUN … run:bad-access` — and
//! never hidden inside "assembled".
//!
//! **The `.सस` are counted and not run**: `paradigm_encode_sas_readable 0` BY DESIGN
//! (research/25 §1.4a, `W-238`: a retired dialect).
//!
//! # What names the labels
//!
//! `ir.t1` writes every routine's `नाम` as ० and lowers a qualified callee the resolver took
//! on trust with `संज्ञा` ० (its own admission; `W-223` is the row that changes it). The
//! census therefore reads `ir.t1`'s two side tables — `वृत्तिनामचिह्नककोश`, the declaration's
//! name token per routine, and `आह्वेयकोश`, the callee node per call — and the resolver's own
//! scope entries, and gives BOTH twins one name table: a routine is labelled
//! `module ⧺ name` (§2.2), a same-module call names the routine's symbol, and a cross-module
//! call names `callee-module ⧺ member` under a symbol of its own. So the label a call jumps
//! to is the label the callee's own module would emit — and the census can say, by name,
//! which labels the standalone link of one module cannot resolve.
//!
//! # The refused cases (§4)
//!
//! Shown refusing at the foot: an IR with an `Unreachable` block (refused by block), a label
//! collision (refused by pair), a callee token naming no routine (an undefined symbol the
//! link lists by name), and one Latin letter injected into an emitted text (`सङ्केतन` refuses
//! with the line).
//!
//! # The pin
//!
//! [`the_assembled_count_is_pinned_and_every_stopped_source_is_named`] is NOT ignored: it
//! runs the chain with the Rust twin and asserts `paradigm/pins.rs` — the assembled count,
//! the runnable count, and every stopped source WITH ITS STAGE — so a regression fails by
//! name. The T1 twin (minutes under the interpreter) and the METRIC lines are the `#[ignore]`d
//! [`measure_corpus_encode`], run with `--ignored --nocapture`.

use sadhana::encode::Target;
use sadhana::kosha::{SymSection, write_debuggable_at};
use sadhana::nidana::Language;
use sadhana::samyojana::link_at;
use sadhana::t1::ast::SymbolId;
use sadhana::t1::chain;
use sadhana::t1::ir::{Block, BlockId, CmpOp, FloatOp, Function, Instruction, Terminator, ValueId};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::riscv64::{self, Module, Names, Refusal};
use sadhana::vishlesana::{decode_at, reassemble};
use sadhana::{assemble_object, vastu};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;
use yantra::{Halt, Machine};

#[path = "paradigm/pins.rs"]
mod pins;

// --- the numbers the machine runs with, taken from the binaries ------------------------

/// The limits `yantra-run` gives a program, IMPORTED rather than restated —
/// `W-262` made `yantra::DEFAULT_RAM` and `DEFAULT_STEPS` the one statement,
/// after five copies of these numbers drifted behind a comment claiming they
/// matched. A test needing different limits should say so and why.
///
/// RAM is no longer a constant here: since the record region grew to 320 MiB
/// (2026-09-13) it is `yantra::ram_for(&image)` at the load site — the same
/// statement `yantra-run` uses — because a constant of `DEFAULT_RAM` stopped
/// every census row at `load` while `yantra-run` ran the same image.
/// The step budget, from the same one statement.
const BUDGET: u64 = yantra::DEFAULT_STEPS;
/// Where every `spec/*.sas` boot proof is linked; `e_entry` is the startup's first word.
const LOAD: u64 = 0x8000_0000;

/// The T1 chain the census runs, in load order: the lexer, the AST, the parser, the
/// resolver and checker, the lexer's helper module, the IR builder, the allocator and
/// output buffer, and the T1 emitter.
const CHAIN: &[&str] = &[
    "lex.t1",
    "ast.t1",
    "parse.t1",
    "artha.t1",
    "sanchaya.t1",
    "sanskrit_text.t1",
    "ir.t1",
    "utsarjana.t1",
    "yantrotsarjana.t1",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

fn t1_dir() -> PathBuf {
    root().join("crates/sadhana-t1/src")
}

fn source(name: &str) -> String {
    let p = t1_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// THE OWNER'S RULING, 2026-09-13: THE FULL CORPUS WALK IS LEGACY. A landing is
/// gated by the narrowed census (`T1_CORPUS=<names>`, seconds); the whole walk
/// runs only when the hourly deep gate asks for it with `T1_FULL_CENSUS=1`. A
/// plain `cargo test` prints why and does nothing — "ok" here means SKIPPED,
/// and the line below says so.
fn walk_allowed(test: &str) -> bool {
    if std::env::var("T1_FULL_CENSUS").is_ok() || std::env::var("T1_CORPUS").is_ok() {
        return true;
    }
    eprintln!(
        "!! {test}: SKIPPED — the full corpus walk is legacy; set T1_CORPUS=<names> (landing gate) or T1_FULL_CENSUS=1 (deep gate) !!"
    );
    false
}

/// Every `.t1` source, sorted by name — the corpus.
///
/// `T1_CORPUS=<a,b,c>` NARROWS IT, AND EVERY FIGURE THE RUN PRINTS IS THEN A
/// FIGURE ABOUT THOSE SOURCES ONLY. This exists because the full census costs
/// ~31 minutes and a lowering change needs it to say whether its cause fell —
/// so four lanes iterating on lowering arms spend the whole session waiting on
/// a corpus walk to re-derive figures for nineteen sources they did not touch.
///
/// ॥ A NARROWED CENSUS MUST NEVER BE QUOTED AS A CENSUS ॥ `paradigm_ir_stubs`
/// under this variable is a SUBSET SUM, not the 1997 that every pin and every
/// report in this project means by that name. The run says so on stderr, loudly
/// and unconditionally, because the danger is not the narrowing — it is a
/// narrowed number travelling into a report that reads like a whole one. That
/// failure has a history here: a gate scoped by a stray untracked file printed
/// a normal-looking scope block and narrowed silently.
///
/// A name that matches nothing is a HARD FAILURE rather than an empty corpus:
/// a typo'd source name would otherwise produce a census of zero sources, every
/// count zero, every assertion about "no source faults" vacuously true. That is
/// the check-that-cannot-fail shape and it must not be reachable from a typo.
/// Say so, loudly, when this is running under the dev profile.
///
/// The `.t1` front half runs under the interpreter here, and `cargo test`'s dev
/// profile builds it at opt-level 0. Measured 2026-09-13 on one tree, same verdict
/// and same twin AGREE either way: debug 95.60 s, release 8.52 s — **11x**. The
/// owner's ruling of that date, item 2, is that the interpreter runs in release
/// ALWAYS, and `tools/t1-census.sh` exists to make that the easy path.
///
/// This warns rather than refusing: the result is correct either way, and failing
/// would break a caller for being slow rather than for being wrong. But it is
/// unmissable, because a banner is the only thing that reaches an operator who
/// reached for `cargo test` out of habit.
fn warn_if_debug() {
    if cfg!(debug_assertions) {
        eprintln!(
            "\n!! THIS CENSUS IS RUNNING UNDER THE DEV PROFILE — about 11x slower !!\n\
             !! (measured: debug 95.60 s, release 8.52 s, same verdict)\n\
             !! Use  tools/t1-census.sh <sources>  which is release and narrowed,\n\
             !! or add --release. Owner ruling 2026-09-13 item 2: the interpreter\n\
             !! runs in release always.\n"
        );
    }
}

fn corpus() -> Vec<String> {
    warn_if_debug();
    let mut names: Vec<String> = std::fs::read_dir(t1_dir())
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();

    // ── GENERATED SOURCES ARE SKIPPED BY THE FULL WALK, AND LOUDLY ──────────
    //
    // `tools/mkspectables.py` emits the embed store as a `.t1` module — ~470 KB,
    // about 29% of the corpus — so the whole corpus's tables live in the language
    // rather than being filled by the host. Walking it in the full census costs
    // that 29% for figures that say nothing about the product: its token counts
    // move when a `spec/` table changes, not when the compiler does, and the
    // generator's own round trip already proves it matches `spec/` byte for byte.
    //
    // SKIPPED ONLY WHEN NOTHING NAMES IT. `T1_CORPUS=sarani` still censuses it,
    // deliberately, because a generated source is still compiled by the product
    // and its stubs would be real. Measured today: it compiles with ZERO stubs.
    //
    // AND NEVER SILENTLY. A metric that improves because something stopped being
    // measured is the failure this project has paid for before, so each skipped
    // file is named on stderr with its size every time the full walk runs.
    if std::env::var("T1_CORPUS").is_err() {
        let mut skipped: Vec<(String, usize)> = Vec::new();
        names.retain(|n| {
            let text = std::fs::read_to_string(t1_dir().join(n)).unwrap_or_default();
            let generated = text
                .lines()
                .take(8)
                .any(|l| l.contains("DO NOT EDIT BY HAND"));
            if generated {
                skipped.push((n.clone(), text.len()));
            }
            !generated
        });
        if !skipped.is_empty() {
            eprintln!(
                "\n!! FULL WALK SKIPS {} GENERATED SOURCE(S), so every count below \
                 EXCLUDES them:\n!!   {}\n\
                 !! They are generated from spec/ and their figures track spec/, not \
                 the compiler.\n\
                 !! Census one deliberately with T1_CORPUS=<name>.\n",
                skipped.len(),
                skipped
                    .iter()
                    .map(|(n, b)| format!("{n} ({b} octets)"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    if let Ok(want) = std::env::var("T1_CORPUS") {
        let want: Vec<String> = want
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.ends_with(".t1") {
                    s.to_string()
                } else {
                    format!("{s}.t1")
                }
            })
            .collect();
        let missing: Vec<&String> = want.iter().filter(|w| !names.contains(w)).collect();
        assert!(
            missing.is_empty(),
            "T1_CORPUS names sources that are not in the corpus: {missing:?}. \
             Available: {names:?}. Failing rather than censusing an empty corpus, \
             because every count would read zero and every `no source faults` \
             assertion would pass vacuously."
        );
        names.retain(|n| want.contains(n));
        eprintln!(
            "\n!! T1_CORPUS IS SET — THIS IS NOT A CENSUS !!\n\
             !! {} of the corpus's sources: {}\n\
             !! Every METRIC below is a SUBSET SUM. `paradigm_ir_stubs` here is \
             NOT the project's 1997.\n\
             !! Do not quote any figure from this run without naming these sources \
             beside it.\n",
            names.len(),
            names.join(", ")
        );
    }
    names
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
    Interpreter::load(&refs, &root().join("spec"))
        .unwrap_or_else(|e| panic!("the chain loads: {e:?}"))
}

/// `load_chain` PLUS the declaration store FILLED from every corpus source, and
/// `अर्थॱसञ्चयसिद्धिः` called so the resolver may consult it.
///
/// W-253. THE CHAIN COMPILED ONE SOURCE AT A TIME AND COULD NOT SEE A NEIGHBOUR.
/// `घोषणासञ्चय` holds what every module declares, and it is filled only by
/// lexing and parsing every source into ONE interpreter — `सङ्ग्रहः` copies each
/// program out of `व्याकर`'s arena after its parse. Until this, `chain()` built a
/// fresh interpreter per source, so the store sat in the image and was never
/// filled: every qualified use was taken on trust, `ir.t1` refused the two calls
/// whose callee therefore carried no symbol, and `T1_ASSEMBLED` read 13 while
/// `sadhana-t1`'s collected IR census read 15. That gap was the whole of W-253.
///
/// PER-SOURCE ATTRIBUTION SURVIVES because `कार्यक्रमपठनम्` resets
/// `घोषणासूचकाङ्क` on entry — every program still begins at declaration १ — and
/// `W-223` part 2a gave `कार्यक्रमनिर्णयः` the EXTENT so the walk stops at this
/// program's last declaration rather than the arena's length. `मध्यरूपॱकार्यक्रमरचना`
/// already took its bound (`घोषणासंख्यान`), which a corpus-wide sweep for
/// `घोषणाकोश ॱ दैर्घ्य` confirmed: the four hits are all bounds guards, none a walk.
fn load_chain_collected() -> Interpreter {
    let mut it = load_chain();
    for n in corpus() {
        let Ok(toks) = it.call("पदविभागॱपदविभाग", vec![octets(&source(&n))], 2_000_000_000)
        else {
            continue;
        };
        let toks = toks.as_int().unwrap_or(0);
        if it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .is_ok()
        {
            let _ = it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 2_000_000_000);
        }
    }
    it.call("अर्थॱसञ्चयसिद्धिः", vec![], 5_000_000)
        .expect("सञ्चयसिद्धिः runs");
    it
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn text_of(v: &Value) -> String {
    match v.octets() {
        Some(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        None => panic!("{v:?} is not a run of octets"),
    }
}

fn member(v: &Value, name: &str) -> Value {
    match v {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no member `{name}`")),
        other => panic!("{other:?} is not a record, so it has no `{name}`"),
    }
}

fn int_of(v: &Value, name: &str) -> i128 {
    member(v, name)
        .as_int()
        .unwrap_or_else(|| panic!("`{name}` is a number"))
}

fn set_int(v: &Value, name: &str, n: i128) {
    if let Value::Record(r) = v {
        r.borrow_mut().insert(name.to_string(), Value::Int(n));
    }
}

/// `x ॱ क्रमाङ्क` of a `मूल्याङ्क`/`पर्वाङ्क`, as Rust counts it: १-based → 0-based;
/// `None` for ० — `मध्यरूप`'s ABSENT.
fn id_of(v: &Value, name: &str) -> Option<usize> {
    let k = int_of(&member(v, name), "क्रमाङ्क");
    usize::try_from(k).ok().filter(|k| *k > 0).map(|k| k - 1)
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global of the loaded chain"))
}

fn global_bool(it: &Interpreter, name: &str) -> bool {
    matches!(it.global(name), Some(Value::Bool(true)))
}

fn global_text(it: &Interpreter, name: &str) -> String {
    match it.global(name) {
        Some(v @ Value::Octets(_)) => text_of(v),
        _ => String::new(),
    }
}

fn arena(it: &Interpreter, name: &str) -> Rc<RefCell<Vec<Value>>> {
    match it.global(name) {
        Some(Value::Arena(a)) => Rc::clone(a),
        other => panic!("`{name}` is an arena, not {other:?}"),
    }
}

fn arena_int(a: &Rc<RefCell<Vec<Value>>>, i: usize) -> i128 {
    a.borrow().get(i).and_then(Value::as_int).unwrap_or(0)
}

/// **WHAT ONE SHAPE'S COUNTER READS, AND IT HAS FOUR ANSWERS WHERE
/// [`arena_int`] HAS ONE.**
///
/// `arena_int` answers `0` for a slot that is PAST THE END of the arena, for a
/// slot holding something that is not an integer, and for a slot holding a
/// genuine zero. `read_module` then wrote the count only `if n > 0`, so an
/// absent MAP key folded those three together with a fourth thing — a shape
/// declared in [`LOWERED_SHAPES`] whose count was never asked for at all.
///
/// **THAT COLLAPSE IS NOT A STYLE COMPLAINT; IT COST SIX READINGS.** `W-306`
/// recorded `assign_index` as "absent or zero" and was then diagnosed, in
/// order, as a lowering regression, a stale pin, an arm dead by achievement, a
/// live-but-unexercised failure path, a read of the wrong table, and a name
/// collision merging two shapes in one map. Every one of those is a different
/// state of this counter, and the instrument printed the same thing for all of
/// them. The assertion's own margin says it out loud — *"an ABSENT key and a
/// MEASURED ZERO are one output here"* — and then leaves the reader to
/// discriminate by hand from the rest of the log.
///
/// So the read is named. The two states that matter are kept APART by
/// construction:
///
/// * [`ShapeRead::OutsideArena`] is the LOUD cause. `ir.t1:755-762` sizes
///   `रचितगणनाकोश` by walking `१..=रचितशेषसीमा`, so a shape above the bound
///   faults on the WRITE and takes the source's IR build down with it. It is
///   the one to fix first, and it must never be reported as a zero.
/// * [`ShapeRead::Zero`] is the QUIET cause: the slot EXISTS, the arm is
///   written, and this corpus never reaches it.
///
/// Measured while naming these, and it is why the distinction is not
/// hypothetical here: `रचितशेषसीमा` is `३५` (`ir.t1:791`) and `assign_index` is
/// shape `३२`, so slot 32 is INSIDE the arena and `assign_index` is
/// [`ShapeRead::Zero`] — not the bound case at all.
///
/// **THE NUMBER AND THE LINE BOTH MOVED, AND THE CLAIM DID NOT.** This
/// margin read `३४` at `:784` until `W-306` added shape
/// `३५ assign_index_grown` and raised the bound in the same edit. Slot 32
/// was inside an arena of 34 and is inside an arena of 35, so the reading
/// this paragraph justifies is unchanged. Restated rather than deleted
/// because a bound quoted BY NUMBER is a dated claim, and this one was
/// already stale on the commit that raised it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ShapeRead {
    /// The shape's code is past the end of `रचितगणनाकोश`. Carries the arena's
    /// length so the bound and the code can be compared in the message rather
    /// than looked up. The LOUD cause; never folded into [`ShapeRead::Zero`].
    OutsideArena { slots: usize },
    /// A slot that exists and does not hold an integer. Cannot happen while
    /// `ir.t1` writes only counts here, which is exactly why it is named: if it
    /// ever does happen, `arena_int`'s `unwrap_or(0)` would have reported it as
    /// a zero and the arm would read unreachable.
    NotAnInteger,
    /// The slot exists and holds zero — or a negative, which a count cannot be
    /// and which `usize::try_from` used to turn into a zero as well. The arm is
    /// written and this corpus does not reach it.
    Zero { raw: i128 },
    /// The arm fired. The only one of the four that is coverage.
    Raised(usize),
}

impl ShapeRead {
    /// The count in the currency [`stub_report`] totals — zero for the three
    /// arms that are not coverage, so a report cannot accidentally add a
    /// diagnostic state to a census figure.
    fn count(&self) -> usize {
        match self {
            ShapeRead::Raised(n) => *n,
            ShapeRead::OutsideArena { .. } | ShapeRead::NotAnInteger | ShapeRead::Zero { .. } => 0,
        }
    }
}

/// One shape's counter in one module, with the three non-answers kept apart.
fn read_shape(a: &Rc<RefCell<Vec<Value>>>, code: usize) -> ShapeRead {
    let a = a.borrow();
    match a.get(code) {
        None => ShapeRead::OutsideArena { slots: a.len() },
        Some(v) => match v.as_int() {
            None => ShapeRead::NotAnInteger,
            Some(raw) => match usize::try_from(raw) {
                Ok(n) if n > 0 => ShapeRead::Raised(n),
                _ => ShapeRead::Zero { raw },
            },
        },
    }
}

/// The module name a `.t1` source declares: `मण्डलम् NAME ॥`.
fn module_name(src: &str) -> String {
    src.lines()
        .next()
        .and_then(|l| l.strip_prefix("मण्डलम् "))
        .and_then(|r| r.split(' ').next())
        .unwrap_or("अज्ञात")
        .to_string()
}

/// The text of token `idx` of `पदविभाग ॱ चिह्नककोश`, through the lexer's own reader.
fn token_text(it: &mut Interpreter, idx: i128) -> String {
    if idx <= 0 {
        return String::new();
    }
    let tokens = arena(it, "चिह्नककोश");
    let Some(tok) = tokens.borrow().get(idx as usize).cloned() else {
        return String::new();
    };
    match it.call("पदविभागॱचिह्नकपाठः", vec![tok], 10_000_000) {
        Ok(v) => text_of(&v),
        Err(_) => String::new(),
    }
}

/// The line of token `idx`, for a diagnostic.
fn token_line(it: &Interpreter, idx: i128) -> i128 {
    if idx <= 0 {
        return 0;
    }
    arena(it, "चिह्नककोश")
        .borrow()
        .get(idx as usize)
        .map(|t| int_of(t, "पङ्क्ति"))
        .unwrap_or(0)
}

// --- the stages ------------------------------------------------------------------------

/// Where a source's chain stopped. In chain order, so the first refusal is the least.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
enum Stage {
    #[default]
    Lex,
    Parse,
    Resolve,
    Typecheck,
    Ir,
    Emit,
    Assemble,
    Link,
    Load,
    Run,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::Lex => "lex",
            Stage::Parse => "parse",
            Stage::Resolve => "resolve",
            Stage::Typecheck => "typecheck",
            Stage::Ir => "IR",
            Stage::Emit => "emit",
            Stage::Assemble => "assemble",
            Stage::Link => "link",
            Stage::Load => "load",
            Stage::Run => "run",
        }
    }
}

/// **THE POPULATION THE TWO EMITTERS WERE COMPARED OVER: every source whose IR
/// BUILT**, which is every source that REACHED the emit stage.
///
/// Reaching `Emit` is not a proxy for being compared, it is the condition:
/// `chain_source` calls `riscv64::emit_module` and then runs the T1 twin
/// unconditionally, and every stop before that returns without either emitter.
/// A row that stops AT `Emit` was still compared — both halves refused it, and
/// the census records their agreement on the refusal KIND as `twin AGREE 0
/// octets`.
///
/// It is deliberately NOT read off `Row::twin`, the field it is compared
/// against; see [`the_population_is_not_read_off_the_twin_field`].
fn built_ir(row: &Row) -> bool {
    row.encode_stop >= Stage::Emit
}

/// The variant of a [`Halt`], as a word: `finisher`, `bad-access`, `step-limit`, …
fn halt_kind(h: &Halt) -> String {
    let dbg = format!("{h:?}");
    let name: String = dbg
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// One source's census row: every count the METRIC lines sum, and the two stops.
#[derive(Debug, Default)]
struct Row {
    name: String,
    module: String,
    decls: i128,
    /// `None`: the checker was never reached; `Some(Err(why))`: it refused.
    typecheck: Option<Result<(), String>>,
    /// The IR builder refused a call (a partial build); the arenas are still read.
    ir_partial: bool,
    no_routines: bool,
    routines: usize,
    insts: usize,
    calls: usize,
    zero_arg_calls: usize,
    cross_module_calls: usize,
    /// See [`Read::growth_routines`].
    growth_routines: usize,
    /// `W-332` — how many of this source's routines the Rust emitter emitted in
    /// their RELAXED form (`riscv64::emit_module_and_relaxations`). It counts
    /// ROUTINES, not far branches: one out-of-range conditional re-emits the
    /// whole routine relaxed, so a routine with four far branches counts once.
    ///
    /// ZERO IS THE READING THIS CORPUS IS EXPECTED TO GIVE, and it is a reading
    /// and not an absence. Every module here is near — `ir.t1` was restored to
    /// its pre-`W-330` form so the trunk keeps a buildable corpus — so a
    /// non-zero sum means something in the corpus started relaxing, which is
    /// the silent change no octet count can attribute.
    relaxed_routines: usize,
    zero_constants: usize,
    stub_constants: usize,
    stubs_by_cause: BTreeMap<&'static str, usize>,
    lowered_by_shape: BTreeMap<&'static str, usize>,
    /// EVERY row of [`LOWERED_SHAPES`], including the ones that read nothing —
    /// which is the point. [`Row::lowered_by_shape`] holds only the positive
    /// counts, so it cannot tell an unreached arm from a short arena; this holds
    /// the named read. See [`ShapeRead`].
    lowered_shape_reads: BTreeMap<&'static str, ShapeRead>,
    args_in_registers: usize,
    args_on_stack_sites: usize,
    /// `Some(Ok(octets))`: the twins agree; `Some(Err(where))`: they diverge.
    twin: Option<Result<usize, String>>,
    t1_seconds: f64,
    text_lines: usize,
    text_instructions: usize,
    constant_pool: usize,
    /// `W-243`: the names this object references and does not define — the
    /// cross-module callees — which the image's link resolves through the other
    /// objects' exports. Informational since the exports; not a stop.
    undefined: Vec<String>,
    /// What the LINK could not resolve in this source's image: a name no object
    /// of the corpus exports. `W-243`: expected empty; the stop when it is not.
    unresolved: Vec<String>,
    /// `W-243`: the assembled module object, kept for the image's link.
    object: Option<Vec<u8>>,
    /// `W-243`: the startup object's text for this source's image — the T1 twin's
    /// when the twins agreed on it, else `riscv64.rs`'s.
    startup: String,
    /// `W-286`: the same startup emitted WITH the record region. The startup is
    /// built per SOURCE and consumed per IMAGE, and only the image knows whether
    /// any of its objects references the region — so both texts are carried and
    /// the link step picks. See `link_and_run`.
    startup_records: String,
    entry_label: Option<String>,
    /// `W-243`: how many module objects the image holds (this one and the closure).
    image_objects: usize,
    words: usize,
    words_back: usize,
    halt: Option<Halt>,
    /// `W-306`: HOW MANY STEPS THE RUN ACTUALLY TOOK, out of [`BUDGET`]. The
    /// machine has counted this all along — `Machine::time` ticks once per
    /// instruction the hart begins — and `Machine::run` threw it away, so
    /// `ran()` was a BOOLEAN over a quantity. That is the two-state instrument
    /// this cycle was sent to fix: a source that halts at the finisher having
    /// spent 999,000 of a million steps and one that spends 9,000 are the same
    /// `true`, and re-taking a pin on the first is re-taking it on a coin toss.
    /// See [`Row::step_margin`].
    steps: u64,
    status: Option<u64>,
    /// Where the emitter's side stopped, and why.
    encode_stop: Stage,
    encode_why: String,
    /// The T0 text the machine ran (the T1 twin's when it agreed), for a reader
    /// of a wrong status (`W-245`).
    text: String,
    /// `W-306`: whether the STEP PROFILER actually ran for this source.
    ///
    /// Recorded rather than re-derived, because the thing being reported is
    /// precisely that a request can name a source the run never profiles, and a
    /// reading that inferred it from `T1_STEP_PROFILE` and `encode_stop` would
    /// be asserting the very fact it is supposed to be checking.
    profiled: bool,
}

impl Row {
    /// The first stage of the OWNER'S chain that refused: the checker's verdict
    /// gates everything after it; otherwise the emitter's stop.
    fn chain_stop(&self) -> (Stage, String) {
        if self.encode_stop < Stage::Typecheck {
            return (self.encode_stop, self.encode_why.clone());
        }
        if let Some(Err(why)) = &self.typecheck {
            return (Stage::Typecheck, why.clone());
        }
        (self.encode_stop, self.encode_why.clone())
    }
    fn assembled(&self) -> bool {
        self.encode_stop > Stage::Assemble
    }
    fn ran(&self) -> bool {
        matches!(self.halt, Some(Halt::Finisher { .. }))
    }
    fn typechecked(&self) -> bool {
        matches!(self.typecheck, Some(Ok(())))
    }
    fn stop_word(&self) -> String {
        match (&self.encode_stop, &self.halt) {
            (Stage::Run, Some(h)) => format!("run:{}", halt_kind(h)),
            (s, _) => s.name().to_string(),
        }
    }
    /// `W-306`: WHERE THIS SOURCE SITS AGAINST THE STEP BUDGET — four answers
    /// where [`Row::ran`] has two.
    ///
    /// The instrument this replaces could not tell a source that finishes with
    /// room to spare from one that finishes on its last thousand steps, and
    /// `T1_ON_YANTRA` is a pin over exactly that boolean. So the cycle that
    /// moves the pin because a source "now runs" cannot know whether it moved
    /// because the compiler got shorter or because the budget happened to be
    /// enough this once — and the next unrelated instruction pushes it back.
    fn step_margin(&self) -> StepMargin {
        if !self.assembled() || self.encode_stop != Stage::Run {
            return StepMargin::NotReached;
        }
        match &self.halt {
            Some(Halt::Finisher { .. }) => {
                // The budget is the denominator on purpose: the question is not
                // "how long did it take" but "how much of what it was given did
                // it need", and only the second one predicts a flip.
                if self.steps * TIGHT_HEADROOM >= BUDGET {
                    StepMargin::Tight
                } else {
                    StepMargin::Spare
                }
            }
            Some(Halt::StepLimit { .. }) => StepMargin::Exhausted,
            _ => StepMargin::NotReached,
        }
    }
}

/// `W-306`: HOW MUCH ROOM TO GROW A RUN MUST HAVE TO COUNT AS `Spare` — it must
/// survive its own work multiplying by this and still halt.
///
/// FOUR, AND THE CORPUS PICKED IT, NOT A PREFERENCE. Measured over the 19-source
/// walk of this tree: eighteen sources finish under 2% of the budget and
/// `shrinkhala.t1` finishes at **487,371 of 1,000,000 — 48%**. There is nothing
/// in between, so the only question a threshold answers here is which side of
/// that gap `shrinkhala` falls on. A doubling test (`*2`) puts it at `Spare` by
/// 2.5% of the budget, which is the instrument calling the one source that can
/// be pushed over the limit by a single change SAFE — the exact blindness this
/// reading exists to remove. Four is the smallest headroom factor that reads the
/// measured corpus honestly, and it is a factor rather than a fitted percentage
/// because `DEFAULT_STEPS`'s own margin says the budget is "likely too small by
/// about an order of magnitude": the constant under it is the thing expected to
/// move.
const TIGHT_HEADROOM: u64 = 4;

/// `W-306`: the four states a source can be in against the step budget. THE
/// POINT OF THE FOURTH: `Tight` is the one the old boolean could not say, and it
/// is the state in which a pin re-take is NOT justified — the source ran, so
/// `ran()` is `true` and `T1_ASSEMBLED_NOT_ON_YANTRA` empties, but it ran on a
/// margin thin enough that the emptying says more about `DEFAULT_STEPS` than
/// about the compiler. An instrument with two states where the truth has four
/// hides its own breakage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepMargin {
    /// Halted at the finisher having used less than half its budget.
    Spare,
    /// Halted at the finisher, but needed half the budget or more.
    Tight,
    /// Did not halt: the budget ran out first.
    Exhausted,
    /// Never got as far as the machine, so the budget never applied.
    NotReached,
}

// --- reading the IR into a named module ------------------------------------------------

/// An IR that names the absent value ० where a value is required — the result of a
/// REFUSED call, consumed by the instruction after it. Neither emitter has an answer
/// for it, so such a module is not measured past the IR stage.
#[derive(Debug)]
struct Hole(String);

/// Everything the census learnt while reading one module's arenas.
struct ReadModule {
    module: Module,
    insts: usize,
    calls: usize,
    zero_arg_calls: usize,
    cross_module: usize,
    /// How many routine slots carried the ZERO name token — the module's
    /// synthesised growth routine, and nothing else on a sound read. This is the
    /// only witness in the object that the index-assignment arm took its GROWTH
    /// branch: `ir.t1:4675` emits the call and sets the flag, `ir.t1:5077` emits
    /// the routine, and the `anyatha` branch at `ir.t1:4730` emits neither.
    /// See [`GrowthBranch`].
    growth_routines: usize,
    zero_constants: usize,
    /// `W-245`: of the `ConstInt(0)`s, how many are the builder's STUB for a shape it
    /// does not lower (the rest are a written `०`), and the stub count per cause.
    stub_constants: usize,
    stubs_by_cause: BTreeMap<&'static str, usize>,
    lowered_by_shape: BTreeMap<&'static str, usize>,
    /// See [`Row::lowered_shape_reads`].
    lowered_shape_reads: BTreeMap<&'static str, ShapeRead>,
    args_in_registers: usize,
    args_on_stack_sites: usize,
}

/// The builder's stub causes, as `ir.t1`'s `अपूर्णगणनाकोश` numbers them (its header
/// lists them), named for the METRIC lines. `W-245`.
const STUB_CAUSES: &[(i128, &str)] = &[
    (1, "name_unresolved"),
    (2, "name_type"),
    (3, "name_device"),
    (4, "name_global"),
    (5, "name_import"),
    (6, "name_enum"),
    (7, "name_local_unheld"),
    (8, "string"),
    (9, "index"),
    (10, "field"),
    (11, "nil"),
    (12, "embed"),
    (13, "slice"),
    (14, "call_on_index"),
    (15, "other"),
    (20, "assign_field"),
    (21, "assign_index"),
    (22, "assign_name"),
    (23, "assign_other"),
    // `W-278`: a global whose value is an ADDRESS, not a word — an arena or a
    // symbol whose type this pass cannot read. Its own cause so the residue
    // left by the scalar lowering is visible rather than folded back into 4.
    (24, "name_global_arena"),
    // `W-field`: a क्षेत्र the lowering declines. TWO POPULATIONS UNDER ONE
    // NUMBER, and a report that does not split them is misleading: sites the
    // CORPUS cannot lower (the base's struct declaration is unreachable — the
    // four guards the checker itself applies) and sites THIS IMPLEMENTATION
    // declines (a base that is not a plain name, which makes `a ॱ b ॱ c`
    // residue by choice). A residue created is not a residue found.
    (25, "field_qualified"),
    // `W-field-split`: cause 25 NARROWED to one population — the base's struct
    // declaration cannot be reached (a cross-module struct interned by
    // `सञ्चयसंज्ञा`, which writes only SymbolId-keyed arenas and is deliberately
    // not `घोषणम्`; or a base that is a MODULE, which has no fields at all).
    // These are FACTS ABOUT THE CORPUS: no edit to `ir.t1` recovers them.
    //
    // Cause 26 is the other half and it is a SCOPE DECISION: the base is not a
    // plain name, so `a ॱ b ॱ c` and a field read on a call's result are
    // declined by THIS ARM and by nothing else. Splitting them is what stops
    // one number crediting the row with the smaller residue while hiding the
    // larger. A residue created is not a residue found.
    (26, "field_base_not_a_name"),
    // The index lowering's residue. TWO POPULATIONS UNDER ONE NUMBER, split
    // when reported and not before: an element width the arm emits no load for
    // is THIS IMPLEMENTATION DECLINING — an `अङ्कः अन्तः अ८` run, refused rather
    // than scaled by १ and read eight octets at a time — where a base whose
    // type the arenas cannot reach is the CORPUS. Only the first can be
    // widened by a decision.
    //
    // AND IT IS NOT WHAT ९ MEANT. Cause ९ said "no index is lowered at all";
    // २७ says the arm ran, walked the base's type, and refused THIS one.
    (27, "index_declined"),
    // `W-279`: CAUSE 25 DECOMPOSED ACROSS ITS SIXTEEN RETURN SITES.
    //
    // 25 was ONE counter over SIXTEEN distinct conditions, and its own margin
    // said so — "TWO POPULATIONS UNDER ONE NUMBER" — while leaving all sixteen
    // sharing it. The claim that justified it, that cause 25 is "a property of
    // the corpus", cannot be checked against a number that answers for a
    // missing type index, an out-of-range arena read and a module-qualified
    // base at once. The sixteen must sum to what 25 read, or the attribution is
    // wrong and THAT is the finding.
    //
    // THE ARM IS A PIPELINE, so each of these SHADOWS every one below it: a
    // field access walks base -> symbol -> type -> struct -> declaration ->
    // field name, and the first refusal returns. A later site can only ever see
    // what survived the earlier ones, which is why the mass concentrates and
    // why one counter hid so much.
    //
    // CLASSIFIED BY THE MARGIN'S OWN TEST — "would relaxing this be a change to
    // `ir.t1` and nothing else?":
    //
    //   SCOPE DECISION   43 — the base is a MODULE-QUALIFIED name. `parse.t1:894`
    //                    folds a right index onto a name node ONLY when the base
    //                    is an imported module, and what it folds in is
    //                    `सदस्यस्थानम्`, the MEMBER. So the node denotes
    //                    `module.member`, not the module — and a member can be a
    //                    record with fields. The margin at `ir.t1:1657` justifies
    //                    this refusal as corpus by saying the base "denotes a
    //                    MODULE, which has no fields", which is about the wrong
    //                    thing. Whether relaxing it is CORRECT is a separate
    //                    question and is the owner's: see `ir.t1:1660`.
    //
    //   DEFENSIVE        41, 30, 34, 37 — arena-bounds guards. Predicted zero,
    //                    and MEASURED zero. A non-zero one would not have been a
    //                    lowering gap; it would be an indexing defect, and it was
    //                    pre-registered as outranking this decomposition.
    //
    //   CORPUS           the rest — a symbol with no recorded type, an
    //                    unreachable declaration, a base that is not a struct.
    //                    No edit to `ir.t1` recovers these.
    //
    // ── MEASURED, 2026-09-10, `measure_corpus_encode` on the merged tree ──
    //
    //     36  field_qualified_decl_index_zero      793
    //     32  field_qualified_type_not_struct      223
    //     39  field_qualified_name_not_in_struct     4
    //                                             ----
    //                                             1020   exactly, and the total
    //     paradigm_ir_stubs                       3818   unmoved
    //
    // THIRTEEN OF THE SIXTEEN ARE MEASURED ZERO, and they are readable as zeros
    // rather than as omissions only because
    // `each_arena_bound_equals_the_highest_entry_of_the_table_it_bounds` passes:
    // a cause above the bound is excluded from the census print AND from its
    // total, so a population up there would reconcile anyway and prove nothing.
    //
    // THE 793 IS THE FINDING. `घोषणासूचकः समम् ०` at `ir.t1:1702` — a struct
    // symbol with no declaration index in `अर्थॱसंज्ञाघोषणाकोश` — is the corpus's
    // largest single stub cause at 21% of all 3818, and it was invisible inside
    // 1020.
    //
    // AND THE ARGUMENT THAT PRODUCED THIS SPLIT WAS ABOUT A SITE THAT IS EMPTY.
    // `43 field_qualified_base_is_a_module` is ZERO. Two lanes spent hours on
    // whether `:1671` is a scope decision or a corpus fact; `parse.t1:894` folds
    // a right index only for an imported module and folds in the MEMBER, so the
    // margin at `ir.t1:1657` justifies that refusal by the wrong object — and
    // the refusal never fires. The margin's CLAIM, that cause 25 is a property
    // of the corpus, is measurably TRUE: all three live sites are corpus facts.
    // A claim built to be independent of a count can still be EMPTIED by one.
    //
    // BOTH FILED PREDICTIONS FAILED THE SAME WAY AND THAT IS THE METHOD
    // FINDING. Two readers, independently, before the numbers: one ranked
    // `:1671` the plurality, the other ranked `:1686 type_absent` largest.
    // NEITHER RANKED `:1702` AT ALL, and `:1686` measured zero. Both reasoned
    // about which guard most expressions would FAIL and neither asked which
    // lookup the corpus actually leaves EMPTY. The second prediction was
    // commissioned as a control on the first and could not catch it, because
    // the blind spot was in the question and not in the answer — INDEPENDENCE
    // DOES NOT DEFEAT A SHARED METHOD.
    //
    // The sum could not have caught it either: 793 + 223 + 4 = 1020 whatever the
    // ranking. A sum tests attribution ARITHMETIC; only a prediction of MEMBERS
    // tests the model that produced it.
    //
    // ── THE FALL, MEASURED 2026-09-11 AT THE LANDING COMMIT ───────────────────
    //
    // THE FIRST STUB FALL RECORDED ON THIS PROJECT. Every prior movement of
    // `paradigm_ir_stubs` was upward.
    //
    //     paradigm_ir_stubs               3818 -> 3028     -790
    //     36  field_qualified_decl_index_zero  793 -> ABSENT  -793
    //         name_global_arena                854 -> 857      +3
    //     32  field_qualified_type_not_struct  223 -> 223  UNMOVED
    //     39  field_qualified_name_not_in_struct 4 ->   4  UNMOVED
    //
    // THE FALL IS 790 AND THE RESOLVED CAUSE WAS 793; THE THREE-SITE GAP IS THE
    // WHOLE RECONCILIATION. Three sites RELOCATED to `name_global_arena` rather
    // than lowering — they were never field accesses. Nothing rounds here: a
    // -790 against a -793 reads like slack in a four-digit number, and it is
    // not. The relocation is visible only because it was looked for; the check
    // was requested explicitly and would not have been run otherwise.
    //
    // THE TWO NEIGHBOURS DID NOT MOVE, and that is the load-bearing negative:
    // a decomposition that leaked into adjacent causes would show at 32 and 39.
    // The fix touched cause 36 and nothing beside it.
    //
    // THREE KINDS OF FACT, AND THEY ARE NOT INTERCHANGEABLE:
    //   MEASURED   3028 and all twelve live causes, this run, the gate's yantra
    //              step, emitted by the non-ignored pin in this file.
    //   RECORDED   3818, 793, 223, 4 — read from the block above.
    //   DERIVED    854, as 857 - 3. NOBODY HAS AN INDEPENDENT 854. It holds only
    //              if `name_global_arena` is the sole other mover; the recorded
    //              baseline gives three causes and a total, not a full table.
    //
    // AND THE BEFORE AND AFTER COME FROM DIFFERENT INSTRUMENTS. 3818 is
    // `measure_corpus_encode`, which is `#[ignore]`d and did NOT run at the
    // landing. 3028 is this file's non-ignored pin. **The bridge is the
    // standalone run at `7c4106ab`, where both produced 3028 with a
    // byte-identical by-cause table** — and that bridge is doing real work, so
    // it belongs wherever the -790 is quoted. It also independently confirms the
    // census is unchanged by the pin re-take, which was predicted before taking.
    //
    // WHICH HALF MOVED: THE `.t1` PRODUCT HALF. The fix is in `ir.t1`; no Rust
    // lowering changed. Under the standing ruling that Sassembly must work
    // without Rust, that is the half that counts.
    (40, "field_qualified_base_index_zero"),
    (41, "field_qualified_base_bounds"),
    (42, "field_qualified_base_null"),
    (43, "field_qualified_base_is_a_module"),
    // `W-cause24-split`: cause 24 was raised at TWO sites with different owners,
    // and its name describes only the second. `ir.t1`'s guard A refuses when the
    // declaration-site slot reads ०; guard B refuses an unreadable type or an
    // arena kind. Same shape as `W-field-split` on cause 25 and the same reason.
    //
    // NAMED FOR WHAT IS OBSERVED, NOT FOR THE CAUSE INFERRED FROM IT. A ० here
    // is a cross-module member (`ir.t1:1713` — `सञ्चयसंज्ञा` interns one with ०,
    // a DESIGN REFUSAL priced at 15 assembled -> 2) OR an in-module name whose
    // declaration index was never recorded, a gap in `अर्थ`: `घोष्यमाणघोषणा`
    // resets to ० at `artha.t1:1794` and only one of three `घोषणम्` callers
    // shares its setter's innermost block. Those want opposite repairs, so
    // calling this `_foreign` would repeat the error the split exists to undo.
    //
    // A THIRD CAUSE WAS SPECIFIED AND THEN REFUTED BEFORE IT WAS WRITTEN. The
    // model had guard A's ० as a sentinel merging three situations; two of them
    // — index past the arena length, and a `शून्यम्` slot — are UNREACHABLE.
    // `ir.t1:1705` has already matched the KIND at this index and `Nil == <int>`
    // answers false rather than faulting, so a hole diverts elsewhere; and the
    // two arenas are written in locked pairs (`artha.t1:1790`/`:1792`,
    // `:2629`/`:2630`) so lengths are equal and holes coincide. A declared row
    // that can never be raised is the silent half of the numbered-table trap.
    //
    // 44, not 28: the literal raises in `crates/sadhana-t1/src/*.t1` run to 43,
    // and 28/29 are taken below by the field-qualified family.
    (44, "name_global_declsite_zero"),
    // `W-283`, 2026-09-27: CAUSE 32 WAS 92% ONE THING AND ITS NAME SAID ANOTHER.
    //
    // `field_qualified_type_not_struct` reads as a CORPUS FACT — the source asks
    // for a field of something that is not a record — and that is how cause 25's
    // margin above justifies the whole family: "no edit to `ir.t1` recovers
    // these". For almost all of 32 that reading is WRONG. The base's recorded type
    // is `अर्थ`'s POISON (`Ty::Error`), which is not a type the corpus wrote: it is
    // the checker declining to type the declaration at all.
    //
    // MEASURED BEFORE IT WAS BELIEVED, `T1_CORPUS=ir,encode` through this census,
    // by a throwaway probe that routed each of the eleven possible base kinds onto
    // a cause measured ZERO on that set — so the split is read off the counter and
    // not off anybody's reading of the corpus:
    //
    //                                        HEAD   probe   this tree
    //     32  field_qualified_type_not_struct   399      41      41
    //     45  field_qualified_type_is_poison      —     369     359
    //         paradigm_ir_stubs                 671     682     673
    //
    // THE PROBE'S 369 IS NOT THE POPULATION — ELEVEN OF THEM WERE THE PROBE.
    // `ir.t1` is itself one of the two sources being compiled, so the probe's own
    // eleven `वस्तुप्रकारः ॱ भेद` reads were compiled and counted, and all eleven
    // landed in the SAME population: 41 + 369 = 399 + 11. The standing population
    // is 358 and the landed guard line makes it 359.
    //
    // 359 AND 41 WERE PREDICTED BEFORE THE RUN AND BOTH ANSWERED EXACTLY. The total
    // did not: predicted 672, measured 673, and the extra one is the same
    // self-measurement in a second place — the guard names `अर्थॱदोषार्थभेद`, a
    // module-qualified read `ir.t1` lowers as cause 1, which went 132 -> 133.
    //
    // THE 41 IS WHAT MUST STILL REFUSE AS 32, and it is a measurement rather than
    // a hope: a base with a real non-record type — an integer, a run asked for a
    // field that is not `दैर्घ्य`, an enum — still answers 32. A split that had
    // emptied 32 would have proved only that the new guard swallowed the arm.
    //
    // WHERE THE NEXT UNIT IS: not here. `अर्थॱनामप्रकारार्थः` already RECORDS why it
    // refused a spelling — `अज्ञातप्रकारपाठकोश` beside `अज्ञातप्रकारकारणकोश`, five
    // named reasons at `artha.t1:219-225` — and `t1_execution.rs` already prints
    // them. No new instrument is needed to ask which reason carries the 369.
    (45, "field_qualified_type_is_poison"),
    (28, "field_qualified_base_symbol_zero"),
    (29, "field_qualified_type_absent"),
    (30, "field_qualified_type_bounds"),
    (31, "field_qualified_type_null"),
    (32, "field_qualified_type_not_struct"),
    (33, "field_qualified_struct_symbol_zero"),
    (34, "field_qualified_decl_index_bounds"),
    (35, "field_qualified_decl_index_null"),
    (36, "field_qualified_decl_index_zero"),
    (37, "field_qualified_decl_bounds"),
    (38, "field_qualified_decl_null"),
    (39, "field_qualified_name_not_in_struct"),
    // `W-306b` half (a): THE SILENT DROP, NOW NAMED. `ir.t1:4883` lowers a
    // १/२/४-octet indexed store through `सङ्कीर्णनिधानरचना`, which answers `०`
    // and emits NOTHING for a width its own ladder does not name. The caller
    // ran the SAME three-way ladder and then DISCARDED that answer — the
    // binding was literally spelled `अवगणन`, disregard — so a width admitted
    // here and refused there produced a store with no instruction, no stub, no
    // shape and no diagnostic.
    //
    // **THIS CAUSE READS ZERO ON THIS CORPUS AND THAT IS CORRECT.** The two
    // ladders admit the same `{१,२,४}` today, so it is unreachable by
    // construction — an instrument for the day a fourth width is added to one
    // ladder and not the other, which is the only way the drop ever happened.
    // It is deliberately NOT in any must-fire list; a cause asserted to rise
    // would red the moment the defect it watches for is absent.
    (46, "assign_index_narrow_dropped"),
];

/// What the builder LOWERED, by shape (`ir.t1`'s `रचितगणनाकोश`): before `W-245`
/// every one of these sites was a stub, so the row's "before" is this count added
/// to the stub count, per shape.
const LOWERED_SHAPES: &[(i128, &str)] = &[
    (1, "param"),
    (2, "local_load"),
    (3, "let_store"),
    (4, "assign_store"),
    (5, "bool"),
    (6, "negate"),
    (11, "op_add"),
    (12, "op_sub"),
    (13, "op_mul"),
    (14, "op_div"),
    (15, "op_eq"),
    (16, "op_ne"),
    (17, "op_lt"),
    (18, "op_gt"),
    (19, "op_and"),
    (20, "op_or"),
    (21, "op_xor"),
    (22, "op_shl"),
    (23, "op_shr"),
    (24, "op_rem"),
    (25, "op_ge"),
    (26, "global_load"),
    // `W-field`: a record field read, LoadField. A NEW number, not a reuse —
    // १३ is `op_mul`, and a reused shape would add every field read to an
    // unrelated count in silence.
    (27, "field_load"),
    // An index read: scale, add, load. A NEW number for the reason २७ gives
    // for itself — a reused shape would add every index read to the FIELD
    // count in silence, and the two could never be separated afterwards.
    (28, "index_load"),
    // `W-283` — a store to a module-level global, lowered as `AddrOfGlobal` २१
    // then `StoreAt` २३: the ruled storage model's first lowering.
    //
    // २९ WAS PICKED BY READING WHAT `रचितगणनम्` RAISES, NOT WHAT THIS TABLE
    // DECLARES, and the two differ by fifteen rows. Rows 11..25 above have NO
    // literal raise anywhere — `ir.t1:2485` computes them as `१० + द्विकर्म` —
    // so a grep for a literal `रचितगणनम् <digits>` returns nine sites and
    // silently misses every `op_*`. Occupied is 1..6 literal, 11..25 computed,
    // 26..28 literal. Nothing about nine plausibly-spaced results looked
    // partial; what caught it was this table DECLARING rows the raise-list did
    // not contain.
    //
    // AND THAT COMPUTED RANGE IS EXACTLY FULL: `ast.t1` declares fifteen
    // operator kinds, `१० + १५ = २५`, and 26 is `global_load` with NO GAP.
    // A sixteenth operator computes 26 and increments `global_load` silently.
    // Filed as a W-283 amendment, unruled, and deliberately not fixed here —
    // mixing it into a lowering census would make every term of that census's
    // conservation identity unattributable.
    (29, "assign_store_global"),
    // `W-284`, THE STORAGE MODEL COMPLETED. Three rows, and they are three
    // because the census must be able to tell them apart afterwards — the same
    // argument २७ and २८ make against sharing one number.
    //
    // ३० is the STORAGE ITSELF and the other two are addresses into it. It is
    // raised by `चरः X ॱॱ <a struct> भवति ० ।`, the corpus's construction
    // idiom, which both halves used to disagree about: the interpreter's
    // `zero_at` answered `Value::Record` with every field zeroed, and `ir.rs`
    // had no record concept at all and lowered the same source to
    // `ConstInt(0)` — a NULL POINTER. That disagreement was invisible for as
    // long as the write was a stub, and became `addr: 0` the moment ३१ made it
    // real. The two rows belong in one landing for that reason and not for
    // tidiness.
    (30, "record_alloc"),
    // ३१/३२ — the write halves of `field_load` २७ and `index_load` २८, each
    // lowered as an address kind then `StoreAt` २३, exactly as २९ lowers a
    // global store. A write stops being a separate design from a read.
    (31, "assign_field"),
    (32, "assign_index"),
    // `W-287`: a RUN's allocation, distinct from `record_alloc` ३० because the
    // two get their size differently — a record counts declared fields, a run
    // takes a constant capacity. A shared number could never be un-mixed.
    (33, "run_alloc"),
    // CONSERVATION HERE IS PAIRWISE AND NOT A TOTAL, AND A READER OF THE TOTAL
    // WILL BOOK THE DIFFERENCE AS RESIDUE THAT IS NOT ONE.
    //
    // A stub refuses ONE site and short-circuits the whole subtree beneath it,
    // so the operands it never lowered are not counted as stubs either.
    // Converting that stub lowers the site AND everything under it, and the
    // extra lowerings have no matching stub fall BY CONSTRUCTION.
    //
    // Measured, `W-len` against its own control:
    //   samyojana  stub fall 24  ->  run_length 20 + index shapes 5 = 25
    //              local_load +32, op_add +3 are the operands
    //   lex        cause 27 fall 17  ->  index_load +23
    //              local_load +72, op_add +15, and nine more shapes move
    //
    // The lex row is the same effect at six times the scale, which is what
    // makes it a mechanism rather than a rounding story. The ONE extra
    // `run_length` on samyojana is `samyojana.t1:731`, `पाठ्यम् अङ्कः पाठ्यम् ॱ
    // दैर्घ्य अन्तः भवति अष्टकम्` — a length read NESTED inside an indexed
    // store's index. The store was refused as cause 27, so the nested read was
    // never lowered and never counted; the store converts, the nested read
    // lowers, and one lowering appears with no fall behind it.
    //
    // SO: pair each cause's fall with the shapes ITS sites became, and read the
    // remainder as subtree exposure. A bare fall-equals-rise check on the totals
    // says -36 on samyojana and -152 on lex, and neither is a defect.
    //
    // `W-len`: a RUN's LENGTH — one load of the header word at `आधार वियोगः ८`.
    //
    // NAMING IT IS NOT COSMETIC AND THE FIRST RUN PROVED IT. `रचितगणनम् ३४` was
    // already incrementing the arena; with no name here the row printed nothing,
    // so 19 lowered length reads were invisible and the CONSERVATION CHECK could
    // not close — a stub fall of 24 against a lowered rise of 40 left a residue
    // of −16 that was entirely this. A shape that lowers and is not named looks
    // exactly like a stub that vanished, which is the one reading this census
    // exists to make impossible.
    (34, "run_length"),
    // `W-306`: the GROWTH branch of the index-assignment arm (`ir.t1:4714`),
    // which built its `सूचीस्थानाज्ञाभेद` inline and raised nothing at all.
    //
    // A NEW NUMBER AND NOT `assign_index` ३२, and the reason is the one this
    // table gives for every split it has made: ३२ names the branch at
    // `ir.t1:3332`, which forms the address from a base the growth routine
    // never touched, and folding the grown store into it would move ३२ from a
    // truthful zero to a count of the OTHER branch — unseparable afterwards.
    //
    // THIS ROW IS WHY `assign_index` READ ZERO. `W-306` spent six readings on
    // that zero before `GrowthBranch` measured it: every module of this corpus
    // takes the growth branch, so ३२'s well-formed zero was CONSISTENT and the
    // lowering it was supposed to witness was happening one branch over,
    // counted by nothing. `assign_index` therefore stays `Quiet`, and THAT IS
    // THE MEASUREMENT AND NOT A DEFECT — so it moved OUT of
    // `REQUIRED_LOWERED_SHAPES`, whose claim is "this corpus lowers it", and
    // INTO [`DECLARED_AND_UNREACHED_SHAPES`], whose claim is "this corpus is
    // WITNESSED not to". Deleting the row is still the failure the five-state
    // read was built to prevent; the move is not a deletion, and this row is
    // the witness it is covered BY.
    (35, "assign_index_grown"),
    // `W-306b`: the masked read-modify-write a NARROW (१/२/४-octet) indexed
    // store lowers to — `ir.t1`'s `सङ्कीर्णनिधानरचना`, raised inside the routine
    // at `ir.t1:1827` and not at either of its two callers.
    //
    // A NEW NUMBER AND NOT ३५, and this time the reason is a WIDTH and not a
    // branch. ३५ `assign_index_grown` is raised before the width ladder runs,
    // on the ADDRESS the growth branch formed, so one count covers an octet
    // store and a word store alike — and that is exactly the blindness that
    // left `अ१६` and `अ३२` lowering to a word store for two weeks after the
    // one-octet case landed (`ir.t1:4805`): the `अ३२` fixture wrote its two
    // elements in ASCENDING order and the second write repaired what the first
    // ate, so no figure in this census could see three clobbered neighbours.
    //
    // ३६ IS THE INSTRUMENT FOR THAT, and it is strictly contained in ३५ on the
    // growth branch: every narrow indexed store raises BOTH, so `३६ <= ३५` is
    // the invariant to read if the two ever disagree in the wrong direction.
    (36, "assign_index_narrow"),
];

/// Shapes this corpus MUST lower — checked present-and-positive by
/// `the_assembled_count_is_pinned_and_every_stopped_source_is_named`.
///
/// **NOT every row of `LOWERED_SHAPES`.** A shape declared there and never
/// raised by this corpus is legitimate — the table describes what `ir.t1` CAN
/// build, and some arms are unreachable on today's sources. This list is the
/// smaller claim: shapes a landed row asserted it was lowering, which must
/// therefore keep lowering. Adding a row above without adding it here leaves
/// the new shape unchecked; that is deliberate, and the check is owed by
/// whoever claims their arm fires.
///
/// `assign_store_global` is here because `W-283` claims it lowers. If it ever
/// reads absent, the two causes are a bound left behind or an arm gone
/// unreachable — the assertion says which to look at first.
const REQUIRED_LOWERED_SHAPES: &[&str] = &[
    "param",
    "local_load",
    "let_store",
    "assign_store",
    "global_load",
    "field_load",
    "index_load",
    "assign_store_global",
    // `W-284` claims all three fire on this corpus, so all three are owed the
    // check. `record_alloc` is the one to watch: if it ever reads absent while
    // `assign_field` is present, every record write in the image is going to
    // address zero and no other figure in this census would say so.
    "record_alloc",
    "assign_field",
    // `assign_index` IS NOT HERE, AND ITS ROW WAS NOT DELETED — it moved to
    // [`DECLARED_AND_UNREACHED_SHAPES`], which is still a check. See that
    // table's own margin for why membership here was the defect and the zero
    // was not.
    // `W-287` claims every run-typed local allocates, so this is owed the check.
    "run_alloc",
    // `W-306` claims this one fires on EVERY module of this corpus — that is
    // exactly what `GrowthOnly` over nine modules measured — so it is owed the
    // check the moment it is declared. Added HERE and in `LOWERED_SHAPES` in
    // one commit: a row declared above and not listed here lowers uncounted
    // and reads exactly like a stub that vanished, which is the `run_length`
    // ३४ lesson recorded in that table's own margin.
    //
    // IF THIS EVER READS `Quiet` WHILE `assign_index` STILL DOES, the corpus
    // has stopped lowering indexed writes on BOTH branches and no other figure
    // in this census would say so.
    "assign_index_grown",
    // `W-306b` — MEASURED BEFORE IT WAS CLAIMED, which is the only reason it is
    // in THIS list and not in `DECLARED_AND_UNREACHED_SHAPES`. `measure_corpus_ir`
    // over 18 built sources read `assign_index_narrow` **319** against
    // `assign_index_grown`'s **667**: narrow indexed writes were a little under
    // half of all indexed writes in this corpus, so the claim "this corpus
    // lowers it" was a reading and not an expectation.
    //
    // ॥ BOTH FIGURES ARE PRE-`f168e91a` AND NEITHER HAS BEEN RE-TAKEN ॥ `W-306c`
    // slice 3 took the narrow INDEXED-STORE path out of ३६ entirely, so ३६ no
    // longer counts what this margin says it counts. 319 and 667 stand here as
    // DATED READINGS and not as the present corpus; re-taking them needs
    // `T1_FULL_CENSUS=1`, which no landing gate sets.
    //
    // WHAT THIS ROW STILL ASSERTS, AND WHAT IT STOPPED ASSERTING. ३६ is raised
    // at `ir.t1:1859`, inside `सङ्कीर्णनिधानरचना`, and `:4883`'s narrow arm no
    // longer calls it — it appends one `स्थाननिधानाज्ञाभेद` carrying
    // `वृद्धिविस्तार` in `ध्रुवमूल्यम्` and raises NO shape, which `ir.t1:4917`
    // argues for in its own margin: ३६ counts a masked read-modify-write and
    // that arm no longer does one. The two callers that remain —
    // `अष्टकनिधानरचना` at `:1904` and `खण्डवृद्धिप्रतिलेखनम्`'s copy loop — keep
    // ३६ positive, so this row is still a live presence check. It is a check on
    // THOSE, not on indexed stores.
    //
    // TWO CLAIMS THIS MARGIN USED TO MAKE ARE NOW FALSE, AND THEY ARE WRITTEN
    // OUT RATHER THAN DELETED because each was the stated reason for a read:
    //
    //   * "३६ can never exceed ३५, because the growth branch reaches
    //     `सङ्कीर्णनिधानरचना` only after raising ३५." UNSOUND — the growth
    //     branch does not reach it at all any more, and ३६'s two remaining
    //     raisers are not on that branch. The containment is gone; ३६ > ३५ is
    //     now an ordinary reading and not a contradiction.
    //   * "If ३६ reads `Quiet` while ३५ stays positive, every narrow indexed
    //     store has silently gone back onto the word store." FALSE — that is
    //     now what a change to `अष्टकनिधानरचना` would say, and it says nothing
    //     about indexed stores either way.
    //
    // SO THE `अ१६`/`अ३२` REGRESSION OF 2026-09-13 IS NO LONGER WATCHED HERE,
    // and the census has no figure that would see it: the arm that would
    // regress raises no shape. What watches it instead is TEXT, in
    // `sadhana-t1/tests/w306c_width_ladders.rs` — the two width ladders
    // compared to each other, and `the_caller_ladder_admits_no_width_zero`,
    // which refuses a `०` in the caller's ladder because `ध्रुवमूल्यम् ०` means
    // "no width stated" and the two emitters answer it differently (the `.t1`
    // one writes eight octets, the Rust twin refuses). That is a gate check and
    // not a census one, which is why nothing in this file reports it.
    "assign_index_narrow",
];

/// **A SHAPE THIS CORPUS IS WITNESSED NOT TO REACH — AND WHY THAT IS A CHECK
/// AND NOT A DELETION.**
///
/// `REQUIRED_LOWERED_SHAPES` says "this corpus MUST lower it". For
/// `assign_index` that claim was simply FALSE, and it had been false since it
/// was written: `ir.t1`'s index-assignment arm has two branches, every module
/// of this corpus takes the GROWTH one, and only the OTHER one raises
/// `रचितगणनम् ३२`. So the red it produced was the table being wrong about the
/// corpus, not the corpus being wrong about the lowering — and six readings
/// went into the zero before [`GrowthBranch`] measured the branch.
///
/// **THE REPAIR IS NOT `- "assign_index",`.** An arm declared in
/// `LOWERED_SHAPES` and named in no assertion anywhere reads exactly like a
/// stub that vanished — that is the `run_length` ३४ lesson recorded in that
/// table's own margin, in reverse. A row that stops being checked must start
/// being checked for something ELSE, and what this table checks is the
/// narrower, TRUE claim: the shape is inside the arena, well-formed, zero —
/// and the work it was supposed to witness is witnessed somewhere.
///
/// **`covered_by` IS WHAT MAKES THE MOVE SAFE.** Silence on its own is not a
/// measurement; silence NEXT TO a named raise is. `assign_index` is covered by
/// `assign_index_grown`, which is itself in `REQUIRED_LOWERED_SHAPES`, so the
/// pair cannot both go quiet without a red: that is the state where the corpus
/// has stopped lowering indexed writes on BOTH branches, and no other figure in
/// this census would say so. [`GrowthBranch`] reads the same fact off a second,
/// independent witness — the growth ROUTINES in the object rather than the
/// shape counts in the arena — and prints it as a `METRIC` above the loop.
const DECLARED_AND_UNREACHED_SHAPES: &[DeclaredUnreached] = &[DeclaredUnreached {
    shape: "assign_index",
    covered_by: "assign_index_grown",
    witness: "`GrowthBranch::GrowthOnly` — every lowered module of this corpus emits a growth \
              routine and none raises `रचितगणनम् ३२`, so the indexed writes are lowered by the \
              GROWTH branch (`ir.t1:4675-4729`) and counted under shape ३५",
}];

/// One row of [`DECLARED_AND_UNREACHED_SHAPES`].
struct DeclaredUnreached {
    /// The shape declared in `LOWERED_SHAPES` and unreached on this corpus.
    shape: &'static str,
    /// The shape whose raise is what makes `shape`'s silence a measurement.
    /// It must be in `REQUIRED_LOWERED_SHAPES` too, or this row's safety rests
    /// on a check nobody runs.
    covered_by: &'static str,
    /// The sentence naming what measured the unreachedness. Quoted into every
    /// complaint, so a reader who hits one is never asked to go find it.
    witness: &'static str,
}

/// **WHETHER A DECLARED-AND-UNREACHED ROW STILL HOLDS — FOUR ANSWERS, AND THE
/// ARM ORDER IS AGAIN THE REFUSAL.**
///
/// 1. `OutsideArena` and `Malformed` on the shape itself are decided FIRST and
///    delegated to [`ShapeCoverage::complaint`] unchanged. Moving a row here
///    does NOT buy it out of the loud causes: a bound left behind still faults
///    the IR build, and a slot holding a non-count is still a broken write.
///    Reading either as "well, it's unreached anyway" is exactly the inference
///    this whole file exists to refuse.
/// 2. [`ShapeCoverage::Raised`] is decided SECOND and IS A RED. The row's claim
///    is "unreached"; a raise falsifies it. The fix is one line — move it back
///    to `REQUIRED_LOWERED_SHAPES` — but it must be MADE, because a list of
///    stale exemptions is how a required check quietly stops being one.
/// 3. `Quiet` with the cover shape NOT raised is a RED, and it is the state
///    this table exists to catch: both branches silent means the corpus lowers
///    no indexed write at all.
/// 4. `Quiet` with the cover shape raised is the declared state. Not a defect.
///
/// `NoModules` on either side is nothing to ask, as everywhere else here.
fn declared_unreached_complaint(
    row: &DeclaredUnreached,
    coverage: &ShapeCoverage,
    cover: &ShapeCoverage,
) -> Option<String> {
    let DeclaredUnreached {
        shape,
        covered_by,
        witness,
    } = row;
    match coverage {
        ShapeCoverage::NoModules => None,
        ShapeCoverage::OutsideArena { .. } | ShapeCoverage::Malformed { .. } => {
            coverage.complaint(shape)
        }
        ShapeCoverage::Raised { total, modules } => Some(format!(
            "`{shape}` is listed DECLARED-AND-UNREACHED and it RAISED {total} time(s) across \
             {modules} module(s). The exemption is STALE, not the corpus: the witness that put \
             it here was {witness}. Move `{shape}` back into `REQUIRED_LOWERED_SHAPES` and \
             delete this row — a list of exemptions nobody re-takes is how a required check \
             stops being one."
        )),
        ShapeCoverage::Quiet { modules } => match cover {
            ShapeCoverage::Raised { .. } | ShapeCoverage::NoModules => None,
            _ => Some(format!(
                "`{shape}` reads a present, well-formed zero in all {modules} module(s) AND its \
                 cover `{covered_by}` reads {cover:?}. THIS IS THE RED THIS TABLE EXISTS FOR. \
                 `{shape}`'s silence is only a measurement while `{covered_by}` is raised — the \
                 witness is {witness}. Both quiet means the corpus lowers the construct on \
                 NEITHER branch, and no other figure in this census would say so. Do not clear \
                 this by deleting a row."
            )),
        },
    }
}

/// **WHETHER ONE REQUIRED SHAPE IS COVERED BY THE CORPUS — ONE DECISION, AND IT
/// HAS FIVE ANSWERS WHERE THE ASSERTION ASKED `n > 0`.**
///
/// A shape lives in each module's own arena, so the question is about a LIST of
/// [`ShapeRead`]s and not about one. The old code summed the positive ones and
/// compared to zero, which is right about coverage and says nothing about cause.
///
/// **THE ARM ORDER IS THE REFUSAL, and it is the whole of the design here:**
///
/// 1. [`ShapeCoverage::Raised`] is decided FIRST. A shape that fires in ONE
///    module is covered, however many others read it zero — a presence claim
///    over the corpus is met by any witness, and a module that simply contains
///    no indexed write must not be evidence against one that does.
/// 2. [`ShapeCoverage::OutsideArena`] is decided SECOND, ahead of the zero. It
///    is the LOUD cause — `रचितशेषसीमा` left behind a shape — and a run that has
///    it has nothing to learn from the quiet reading. ONE module short of the
///    slot is enough to name it: the bound is a property of `ir.t1`, so if any
///    module's arena cannot hold the code, the bound is the defect.
/// 3. [`ShapeCoverage::Malformed`] third, for the same reason: a slot holding a
///    non-count is a broken write, not an unreached arm.
/// 4. [`ShapeCoverage::Quiet`] LAST, and only when every module agreed the slot
///    exists, holds a count, and that count is zero. This is the arm that means
///    what `W-306` spent six readings establishing by hand: the code is there
///    and this corpus does not reach it.
/// 5. [`ShapeCoverage::NoModules`] for an empty list, which is nothing to ask
///    rather than a defect — a census that read no module is not a census, and
///    reporting it as an unreachable arm would blame `ir.t1` for an empty run.
///
/// Extracted so the claim can be handed a LIST rather than computed inside the
/// assertion's loop, where only the corpus could reach it and the corpus has
/// only ever produced two of the five answers. Made to fire on all five in
/// [`the_lowered_shape_coverage_reading_names_five_states_and_keeps_the_loud_cause_first`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum ShapeCoverage {
    /// No module was read at all. Counted, never a defect.
    NoModules,
    /// At least one module raised the shape. Carries the corpus total and how
    /// many modules contributed, so a shape covered by a single witness is
    /// visible as such rather than reading like a corpus-wide property.
    Raised { total: usize, modules: usize },
    /// Some module's `रचितगणनाकोश` is too short to hold the code. The loud one.
    OutsideArena { modules: usize, slots: usize },
    /// Some module's slot holds something that is not a count.
    Malformed { modules: usize },
    /// Every module read a present, well-formed zero. The arm exists and is
    /// unreached.
    Quiet { modules: usize },
}

impl ShapeCoverage {
    /// The decision. See the arm order above — it is load-bearing.
    fn of(reads: &[ShapeRead]) -> ShapeCoverage {
        if reads.is_empty() {
            return ShapeCoverage::NoModules;
        }
        let total: usize = reads.iter().map(ShapeRead::count).sum();
        if total > 0 {
            return ShapeCoverage::Raised {
                total,
                modules: reads.iter().filter(|r| r.count() > 0).count(),
            };
        }
        let short: Vec<usize> = reads
            .iter()
            .filter_map(|r| match r {
                ShapeRead::OutsideArena { slots } => Some(*slots),
                _ => None,
            })
            .collect();
        if let Some(slots) = short.iter().copied().min() {
            return ShapeCoverage::OutsideArena {
                modules: short.len(),
                slots,
            };
        }
        let malformed = reads
            .iter()
            .filter(|r| matches!(r, ShapeRead::NotAnInteger))
            .count();
        if malformed > 0 {
            return ShapeCoverage::Malformed { modules: malformed };
        }
        ShapeCoverage::Quiet {
            modules: reads.len(),
        }
    }

    /// The sentence the assertion reports, or `None` for the two arms that are
    /// not a defect. Each names its OWN cause, so the reader is not asked to
    /// discriminate from the rest of the log — which is what `W-306` had to do
    /// six times.
    fn complaint(&self, shape: &str) -> Option<String> {
        match self {
            ShapeCoverage::NoModules | ShapeCoverage::Raised { .. } => None,
            ShapeCoverage::OutsideArena { modules, slots } => Some(format!(
                "`{shape}` reads OUTSIDE THE ARENA in {modules} module(s) — the shortest                  `रचितगणनाकोश` holds {slots} slot(s). THIS IS THE LOUD CAUSE and it is not a                  missing arm: `ir.t1` sizes that arena by walking `१..=रचितशेषसीमा`, so a                  shape above the bound faults on the WRITE and takes the source's IR build                  down with it. Raise `रचितशेषसीमा` in the same edit as the shape. Do NOT                  read this as an unreachable arm."
            )),
            ShapeCoverage::Malformed { modules } => Some(format!(
                "`{shape}`'s slot holds something that is NOT A COUNT in {modules} module(s).                  The arm may well fire; the WRITE is broken. `arena_int` would have reported                  this as a zero and it would have read as an unreachable arm."
            )),
            ShapeCoverage::Quiet { modules } => Some(format!(
                "`{shape}` reads a PRESENT, WELL-FORMED ZERO in all {modules} module(s). \
                 This is the QUIET cause and the reading is now definite, not inferred \
                 from the rest of the log: the slot exists, so the shape is INSIDE \
                 `रचितशेषसीमा` and the bound is NOT the problem. Either nothing raises \
                 this shape on this corpus — the arm is written and unreached — or the \
                 raise is guarded by a condition the corpus never meets. Read the \
                 `रचितगणनम्` call for this shape in `ir.t1` and the guard directly above \
                 it; do not infer a lowering regression from this number, because a \
                 lowering that is merely counted under a DIFFERENT shape also reads \
                 exactly like this."
            )),
        }
    }
}

/// **WHICH BRANCH OF `ir.t1`'s INDEX-ASSIGNMENT ARM THE CORPUS TAKES — FIVE
/// ANSWERS, AND THE TWO-STATE VERSION OF THIS QUESTION IS WHY `W-306` READ IT
/// WRONG.**
///
/// `ir.t1:4622` is the arm for `x अङ्कः i अन्तः भवति v`. Inside it, `:4675`
/// splits on `वृद्धिविस्तार अधिकम् ०` — the element width, set at `:4661-4665`
/// only when the base's type resolves to a `खण्डार्थभेद` with a known element:
///
/// * the GROWTH branch (`:4675-4729`) calls the module's growth routine, sets
///   `वृद्धिवृत्तिप्रयुक्तम्`, and builds `सूचीस्थानाज्ञाभेद` INLINE at `:4714`.
///   It raises NO `रचितगणनम्` at all.
/// * the `अन्यथा` branch (`:4731-4734`) sets `स्थानमात्रम्` and re-enters the
///   expression builder, where `:3332` raises `रचितगणनम् ३२` — `assign_index`.
///
/// So `assign_index` counts ONE branch and the other leaves no shape behind it.
/// A zero there is NOT "the arm is unreachable"; it is consistent with every
/// site in the corpus taking the branch that never raises. The witness for that
/// branch is the growth ROUTINE: emitted once per module at `:5077` iff some
/// site set the flag, and visible in the object as a routine slot whose name
/// token is `०` ([`Read::growth_routines`]).
///
/// **THE DECISION ORDER IS THE REFUSAL.** [`GrowthBranch::AmbiguousMarker`] is
/// decided FIRST, ahead of every product reading, for the reason
/// [`ShapeCoverage::OutsideArena`] is: a broken instrument must never be folded
/// into an answer about the corpus. [`GrowthBranch::Both`] is decided SECOND,
/// because it is the one state in which a raise added to the growth branch would
/// DOUBLE-COUNT a site that also passes `:3332` — the case the fix must refuse.
/// Only then are the single-branch readings, and [`GrowthBranch::Unreached`]
/// last, since "no sites at all" is nothing to explain.
#[derive(Debug, PartialEq, Eq)]
enum GrowthBranch {
    /// Some module carried MORE THAN ONE zero name token, so the marker is not
    /// unique to the growth routine and NEITHER single-branch reading below is
    /// sound. `most` is the largest count seen in one module.
    AmbiguousMarker { modules: usize, most: usize },
    /// Both witnesses fire: `sites` raises of `assign_index` AND a growth
    /// routine in `modules` module(s). The corpus reaches both branches, so a
    /// `रचितगणनम् ३२` added to the growth branch would count some sites twice.
    Both { modules: usize, sites: usize },
    /// A growth routine in `modules` module(s) and `assign_index` quiet
    /// everywhere: EVERY corpus index-assignment site takes the growth branch,
    /// and `ir.t1:3332` is live code this corpus does not reach.
    GrowthOnly { modules: usize },
    /// `assign_index` raised `sites` times and no module emitted a growth
    /// routine: every site takes the `अन्यथा` branch, and the growth branch is
    /// the unreached one.
    PlaceOnly { sites: usize },
    /// Neither witness fires. The arm is not entered at all on this corpus and
    /// the branch question does not arise.
    Unreached,
}

impl GrowthBranch {
    /// `growth_per_module` is one entry per measured module — NOT the positive
    /// ones only, because a module that emitted no growth routine is what makes
    /// [`GrowthBranch::GrowthOnly`] a claim about the corpus rather than about
    /// one source.
    fn of(growth_per_module: &[usize], sites: usize) -> GrowthBranch {
        let most = growth_per_module.iter().copied().max().unwrap_or(0);
        if most > 1 {
            return GrowthBranch::AmbiguousMarker {
                modules: growth_per_module.iter().filter(|n| **n > 1).count(),
                most,
            };
        }
        let modules = growth_per_module.iter().filter(|n| **n == 1).count();
        match (modules, sites) {
            (0, 0) => GrowthBranch::Unreached,
            (0, sites) => GrowthBranch::PlaceOnly { sites },
            (modules, 0) => GrowthBranch::GrowthOnly { modules },
            (modules, sites) => GrowthBranch::Both { modules, sites },
        }
    }

    /// The one state that is a DEFECT rather than a reading. A broken marker is
    /// this instrument's own breakage; every other state is a fact about the
    /// corpus and belongs on a `METRIC` line under the owner ruling of
    /// 2026-09-13, not in a pin.
    fn complaint(&self) -> Option<String> {
        match self {
            GrowthBranch::AmbiguousMarker { modules, most } => Some(format!(
                "THE GROWTH-ROUTINE MARKER IS NOT UNIQUE: {modules} module(s) carry more than \
                 one routine slot with name token ०, the largest holding {most}. `ir.t1:5077` \
                 emits that routine ONCE per module, so either a second routine is reaching \
                 the object unnamed or `वृत्तिनामचिह्नककोश` is being written twice. Until this \
                 reads 0-or-1 per module, `growth_routines` is NOT a witness for the growth \
                 branch and the census row above it says nothing about `ir.t1:4675`. Note the \
                 driver's own symbol insert collides on the second one: both slots become \
                 `SymbolId(10_000_005)`."
            )),
            GrowthBranch::Both { .. }
            | GrowthBranch::GrowthOnly { .. }
            | GrowthBranch::PlaceOnly { .. }
            | GrowthBranch::Unreached => None,
        }
    }
}

/// The T1 instruction kind's Rust twin, for the eleven binary kinds and `Cmp` — the
/// numbering `ir.t1` declares (`W-245`).
fn binary_kind(kind: i128, l: ValueId, r: ValueId, sub: i128, mark: i128) -> Option<Instruction> {
    Some(match kind {
        3 => Instruction::Add(l, r),
        4 => Instruction::Sub(l, r),
        6 => Instruction::Mul(l, r),
        // `W-381` stage 3 (ruling (b)) — KINDS ७ AND ८ ARE TWO INSTRUCTIONS EACH,
        // told apart by `ध्रुवमूल्यम्` as kind १० is: ० signed, १ unsigned (a name
        // declared unsigned on either side). `if`, and the signed variant first,
        // for the kind-table reader's reason given at kind १०.
        7 => {
            if mark == 0 {
                Instruction::Div(l, r)
            } else if mark == 1 {
                Instruction::DivU(l, r)
            } else {
                return None;
            }
        }
        8 => {
            if mark == 0 {
                Instruction::Rem(l, r)
            } else if mark == 1 {
                Instruction::RemU(l, r)
            } else {
                return None;
            }
        }
        9 => Instruction::Shl(l, r),
        // `W-333` — KIND १० IS TWO INSTRUCTIONS, TOLD APART BY `ध्रुवमूल्यम्`: ० the
        // arithmetic shift, १ the logical one `मध्यरूप` builds for a left operand
        // that is a name declared unsigned. Any other value is refused: a
        // decoder that guessed would build `Shr` and compile.
        //
        // WRITTEN AS `if`, NOT AS A NESTED `match mark { 0 => …, 1 => … }`:
        // `t1_transcriptions.rs` reads every `N => Family::Variant` arm in the
        // tree as a row of a kind table, nested ones included, so a nested
        // match here was scanned as "Instruction 0 is `Shr`, Instruction 1 is
        // `ShrL`" and reddened the guard. `Shr` stays FIRST for the same
        // reader: it takes the first variant in the arm as kind १०'s.
        10 => {
            if mark == 0 {
                Instruction::Shr(l, r)
            } else if mark == 1 {
                Instruction::ShrL(l, r)
            } else {
                return None;
            }
        }
        11 => Instruction::And(l, r),
        12 => Instruction::Or(l, r),
        13 => Instruction::Xor(l, r),
        14 => Instruction::Cmp(
            match sub {
                1 => CmpOp::Eq,
                2 => CmpOp::Ne,
                3 => CmpOp::Lt,
                4 => CmpOp::Ge,
                5 => CmpOp::Ltu,
                6 => CmpOp::Geu,
                other => panic!("compare sub-kind {other} is not one of the six"),
            },
            l,
            r,
        ),
        _ => return None,
    })
}

/// The `(module, member)` a callee NODE names — a one-token qualified name split at
/// `ॱ`, W-228 (b)'s folded spaced qualifier, or a `क्षेत्र` whose base is the module.
fn callee_of_node(it: &mut Interpreter, node: i128) -> Option<(String, String)> {
    if node <= 0 {
        return None;
    }
    let nodes = arena(it, "अभिव्यञ्जककोश");
    let n = nodes.borrow().get(node as usize).cloned()?;
    let kind = int_of(&n, "भेद");
    let name_kind = global_int(it, "नामाभिव्यञ्जकभेद");
    let field_kind = global_int(it, "क्षेत्राभिव्यञ्जकभेद");
    if kind == name_kind {
        let text = token_text(it, int_of(&n, "मूल्यसूचकाङ्क"));
        let folded = int_of(&n, "दक्षिणसूचकाङ्क");
        if folded > 0 {
            return Some((text, token_text(it, folded)));
        }
        if let Some((m, r)) = text.split_once('\u{971}') {
            return Some((m.to_string(), r.to_string()));
        }
        return Some((String::new(), text));
    }
    if kind == field_kind {
        let base = int_of(&n, "वामसूचकाङ्क");
        let base_text = {
            let b = nodes.borrow().get(base as usize).cloned();
            match b {
                Some(b) if int_of(&b, "भेद") == name_kind => {
                    token_text(it, int_of(&b, "मूल्यसूचकाङ्क"))
                }
                _ => String::new(),
            }
        };
        return Some((base_text, token_text(it, int_of(&n, "मूल्यसूचकाङ्क"))));
    }
    None
}

/// Read `मध्यरूप`'s arenas into `riscv64::Module`, NAMED: routines by their declaration's
/// name token, same-module callees by the resolver's symbol, cross-module callees by
/// `(module, member)` under a symbol of their own — and write the same symbols into the
/// T1 arenas, so both twins see one IR and one name table (see the module header).
#[allow(clippy::too_many_lines)]
fn read_module(it: &mut Interpreter, module: &str, resolver: &Value) -> Result<ReadModule, Hole> {
    let need = |v: &Value, name: &str, at: &str| -> Result<usize, Hole> {
        id_of(v, name).ok_or_else(|| Hole(format!("{at} names the absent value ० as its `{name}`")))
    };
    // The resolver's entries: name → symbol+1, over every scope still open (the
    // module's), and the kind of every symbol.
    let mut entry_symbol: HashMap<String, i128> = HashMap::new();
    let mut symbol_name: HashMap<i128, String> = HashMap::new();
    if let Value::Arena(scopes) = member(resolver, "परिसराः") {
        for scope in scopes.borrow().iter() {
            if let Value::Record(_) = scope
                && let Value::Arena(entries) = member(scope, "प्रविष्टयः")
            {
                for e in entries.borrow().iter() {
                    if let Value::Record(_) = e {
                        let name = text_of(&member(e, "नाम"));
                        let sym = int_of(e, "संज्ञा") + 1;
                        entry_symbol.entry(name.clone()).or_insert(sym);
                        symbol_name.entry(sym).or_insert(name);
                    }
                }
            }
        }
    }
    let kinds = arena(it, "संज्ञाभेदकोश");
    let routine_kind = global_int(it, "वृत्तिघोषणाभेद");

    let functions = arena(it, "वृत्तिकोश");
    let blocks = arena(it, "पर्वकोश");
    let insts = arena(it, "आज्ञाकोश");
    let args = arena(it, "आदानकोश");
    // `W-254`: a string literal's octets, one contiguous run per literal.
    //
    // A RUN OF OCTETS, NOT AN ARENA. `अङ्कः अन्तः अ८` is a `Value::Octets` in
    // this interpreter, where `अङ्कः अन्तः मूल्याङ्क` is a `Value::Arena` — so
    // `आदानकोश` above is `Rc`-shared and live, and this is a COPY taken here.
    // That is correct only because `read_module` runs after the IR is built;
    // reading it earlier would snapshot an empty run. Reaching for `arena()`
    // out of symmetry with the line above is what the first version did, and
    // the panic named the type.
    let string_octets: Vec<u8> = it
        .global("पाठाक्षरकोश")
        .and_then(Value::octets)
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    let callee_nodes = arena(it, "आह्वेयकोश");
    // `W-259`: the numeral node behind a constant, ० when none put it there.
    let routine_names = arena(it, "वृत्तिनामचिह्नककोश");
    let stub_reasons = arena(it, "अपूर्णहेतुकोश");
    let stub_counts = arena(it, "अपूर्णगणनाकोश");
    let lowered_counts = arena(it, "रचितगणनाकोश");
    let count = usize::try_from(global_int(it, "वृत्तिसूचकाङ्क")).unwrap_or(0);

    let mut names = Names::new();
    // routine name → its symbol, for a call that names this module's own routine.
    let mut routine_symbol: HashMap<String, SymbolId> = HashMap::new();
    let mut routine_syms: Vec<SymbolId> = Vec::new();
    // `W-306` — HOW MANY GROWTH ROUTINES, not whether one exists. The growth
    // routine is emitted ONCE per module (`ir.t1:5077` guards it on
    // `vriddhivrittiprayuktam`), so this is 0 or 1 on a sound read and anything
    // above 1 says the read is not sound — see [`GrowthBranch::AmbiguousMarker`].
    //
    // `N-004` — THE ZERO TOKEN NO LONGER MEANS THE GROWTH ROUTINE ALONE. Token ०
    // marks a routine the IR BUILT (it has no declaration), and a module may
    // hold more than one; each is named by the symbol the IR wrote into its
    // `नाम`, through the same table `chain.rs` names them by
    // (`chain::built_routine_name`), and only the one named `खण्डवृद्धिः` is
    // counted here.
    let mut growth_routines = 0usize;
    for i in 1..=count {
        if arena_int(&routine_names, i) == 0 {
            let symbol = functions.borrow().get(i).map_or(0, |f| int_of(f, "नाम"));
            let Some(name) = chain::built_routine_name(symbol) else {
                panic!(
                    "routine {i} of `{module}` has no declaration and its symbol {symbol} \
                     names no built routine (chain.rs `BUILT_ROUTINES`)"
                );
            };
            if name == "खण्डवृद्धिः" {
                growth_routines += 1;
            }
            let sym =
                SymbolId(usize::try_from(symbol).expect("a built routine's symbol is positive"));
            names.insert(sym, (module.to_string(), name.to_string()));
            routine_symbol.insert(name.to_string(), sym);
            routine_syms.push(sym);
            continue;
        }
        let name = token_text(it, arena_int(&routine_names, i));
        let name = if name.is_empty() {
            format!("वृत्ति{}", riscv64::devanagari(i as i64))
        } else {
            name
        };
        let sym = match entry_symbol.get(&name) {
            Some(s) if arena_int(&kinds, *s as usize) == routine_kind => SymbolId(*s as usize),
            _ => SymbolId(1_000_000 + i),
        };
        names.insert(sym, (module.to_string(), name.clone()));
        routine_symbol.insert(name, sym);
        routine_syms.push(sym);
    }

    // `W-278` — THE MODULE'S GLOBALS: their storage, and a name for each so a
    // `LoadGlobal` can be labelled. `ir.t1` keeps the two halves of the label
    // apart (module, name) because the `.t1` emitter writes them consecutively
    // and needs no octet-concatenation routine; this side joins them, which is
    // what `routine_label` does anyway — no joiner.
    let g_modules = arena(it, "वैश्विकमण्डलकोश");
    let g_names = arena(it, "वैश्विकनामकोश");
    let g_values = arena(it, "वैश्विकमूल्यकोश");
    let g_octets = arena(it, "वैश्विकसामर्थ्यकोश");
    let g_count = usize::try_from(global_int(it, "वैश्विकसञ्चयसूचकाङ्क")).unwrap_or(0);
    // `W-293` — the third element is the OCTETS, read from `मध्यरूप`'s fourth
    // arena and decided there. The census carries it so its `Module` matches the
    // emitter's; nothing here derives a global's storage.
    let mut globals: Vec<(String, i64, i64)> = Vec::new();
    for i in 1..=g_count {
        // `अङ्कः अन्तः अ८` is a `Value::Octets`, NOT an arena — reading one with
        // `arena()` panics. (a peer session hit that on the string path.)
        let g = g_modules.borrow();
        let Some(mv) = g.get(i) else { continue };
        let m = text_of(mv);
        drop(g);
        let g = g_names.borrow();
        let Some(nv) = g.get(i) else { continue };
        let n = text_of(nv);
        drop(g);
        if n.is_empty() {
            continue;
        }
        let octets = arena_int(&g_octets, i) as i64;
        globals.push((
            format!("{m}{n}"),
            arena_int(&g_values, i) as i64,
            if octets < 8 { 8 } else { octets },
        ));
        // The read side names a SYMBOL, so the global needs an entry or
        // `routine_label` refuses the module with `UnnamedSymbol`.
        if let Some(sym) = entry_symbol.get(&n) {
            names.insert(SymbolId(*sym as usize), (m, n));
        }
    }

    // THE LOWERING HALF OF वास्तु (2026-09-13): a cross-module global read lowers to
    // `LoadGlobal(symbol)`, and the resolver minted that symbol for a registry
    // entry (`अर्थॱसञ्चयसंज्ञाप्रविष्टयः` beside `अर्थॱसञ्चयसंज्ञामूल्यानि`). The .t1
    // driver names it (declaring module, member) in `नामसञ्चयः`; the twin does the
    // same here, through the registry's own accessors, so both emitters resolve
    // one label — the one the data-only object exports.
    {
        let n = usize::try_from(global_int(it, "सञ्चयसंज्ञासूचकाङ्क")).unwrap_or(0);
        let entries = arena(it, "सञ्चयसंज्ञाप्रविष्टयः");
        let symbols = arena(it, "सञ्चयसंज्ञामूल्यानि");
        let global_kind = global_int(it, "चरघोषणाभेद");
        for k in 1..=n {
            let e = arena_int(&entries, k);
            let sym = arena_int(&symbols, k);
            if e <= 0 || sym <= 0 {
                continue;
            }
            let kind = it
                .call("घोषणासञ्चयॱप्रविष्टिभेदः", vec![Value::Int(e)], 1_000_000)
                .ok()
                .and_then(|v| if let Value::Int(k) = v { Some(k) } else { None })
                .unwrap_or(0);
            if kind != global_kind {
                continue;
            }
            let mi = it
                .call("घोषणासञ्चयॱप्रविष्टिमण्डलम्", vec![Value::Int(e)], 1_000_000)
                .ok()
                .and_then(|v| if let Value::Int(k) = v { Some(k) } else { None })
                .unwrap_or(0);
            let m = it
                .call("घोषणासञ्चयॱमण्डलनाम", vec![Value::Int(mi)], 1_000_000)
                .ok()
                .map(|v| text_of(&v))
                .unwrap_or_default();
            let member = it
                .call("घोषणासञ्चयॱप्रविष्टिनाम", vec![Value::Int(e)], 1_000_000)
                .ok()
                .map(|v| text_of(&v))
                .unwrap_or_default();
            if m.is_empty() || member.is_empty() {
                continue;
            }
            names
                .entry(SymbolId(usize::try_from(sym).unwrap_or(0)))
                .or_insert((m, member));
        }
    }
    // The record cursor the emitters own, addressed from the IR for run growth
    // under ir.t1's रचनासूचकसंज्ञा (१००००००३); the label is module-less.
    names
        .entry(SymbolId(10_000_003))
        .or_insert((String::new(), "रचनासूचकः".to_string()));
    names
        .entry(SymbolId(10_000_004))
        .or_insert((String::new(), "रचनाक्षेत्रम्".to_string()));
    let mut next_synth = 2_000_000usize;
    let mut synth: HashMap<(String, String), SymbolId> = HashMap::new();
    let mut out = ReadModule {
        module: Module {
            globals,
            name: module.to_string(),
            functions: Vec::new(),
            names: Names::new(),
            entry: None,
        },
        insts: 0,
        calls: 0,
        zero_arg_calls: 0,
        cross_module: 0,
        growth_routines,
        zero_constants: 0,
        stub_constants: 0,
        stubs_by_cause: BTreeMap::new(),
        lowered_by_shape: BTreeMap::new(),
        lowered_shape_reads: BTreeMap::new(),
        args_in_registers: 0,
        args_on_stack_sites: 0,
    };
    for (code, name) in STUB_CAUSES {
        let n = usize::try_from(arena_int(&stub_counts, *code as usize)).unwrap_or(0);
        if n > 0 {
            out.stubs_by_cause.insert(name, n);
        }
    }
    for (code, name) in LOWERED_SHAPES {
        // NAMED FIRST, THEN REDUCED. `lowered_by_shape` keeps exactly the
        // entries it always held — the positive ones — so every existing report
        // and pin reads the same number; the four-state read is recorded BESIDE
        // it rather than in place of it. `ShapeRead::count` is the only bridge,
        // and it answers zero for all three non-answers, so a diagnostic state
        // cannot leak into a census figure.
        let read = read_shape(&lowered_counts, *code as usize);
        let n = read.count();
        if n > 0 {
            out.lowered_by_shape.insert(name, n);
        }
        out.lowered_shape_reads.insert(name, read);
    }
    for i in 1..=count {
        let f = functions.borrow()[i].clone();
        let sym = routine_syms[i - 1];
        set_int(&f, "नाम", sym.0 as i128);
        let start = usize::try_from(int_of(&f, "पर्वारम्भ")).unwrap_or(0);
        let n = usize::try_from(int_of(&f, "पर्वसंख्यान")).unwrap_or(0);
        let mut out_blocks = HashMap::new();
        for b in start..start + n {
            let blk = blocks.borrow()[b].clone();
            let first = usize::try_from(int_of(&blk, "आज्ञारम्भ")).unwrap_or(0);
            let k = usize::try_from(int_of(&blk, "आज्ञासंख्यान")).unwrap_or(0);
            let mut out_insts = Vec::new();
            for j in first..first + k {
                let ins = insts.borrow()[j].clone();
                let at = format!("instruction {j} of block {b}");
                let v = ValueId(need(&ins, "फलम्", &at)?);
                let inst = match int_of(&ins, "भेद") {
                    1 => {
                        let c = int_of(&ins, "ध्रुवमूल्यम्");
                        if c == 0 {
                            out.zero_constants += 1;
                            if arena_int(&stub_reasons, j) > 0 {
                                out.stub_constants += 1;
                            }
                        }
                        Instruction::ConstInt(c as i64)
                    }
                    2 => {
                        out.calls += 1;
                        let s = int_of(&ins, "संज्ञा");
                        // W-253: THE CALLEE NODE DECIDES, NOT THE SYMBOL'S PRESENCE.
                        // A cross-module callee used to arrive with symbol ० — the
                        // resolver "took the member on trust" — and the ० branch below
                        // read the NODE for its module and member and built a
                        // name-based label the defining object exports. `W-223` part 2
                        // gives those callees a real symbol, which sent them into the
                        // middle branch instead, where a symbol that is not one of THIS
                        // module's routines falls back to `संज्ञा<id>` — a label local to
                        // the program that resolved it and therefore one NOTHING
                        // exports. That is what the middle branch's own comment warned
                        // of: "the link will list it if nothing defines it". It did:
                        // three sources that linked and ran stopped at link, and the
                        // labels were `सङ्केतनसंज्ञा२१९`, `मुद्रणसंज्ञा४०` and their like.
                        //
                        // A SymbolId is meaningful only inside the program that minted
                        // it; a cross-module label must be named from the NAME. So ask
                        // the node first, and use the symbol only when the node does not
                        // name another module.
                        let qualified = callee_of_node(it, arena_int(&callee_nodes, j))
                            .filter(|(m, _)| !m.is_empty() && m != module);
                        let sym = if let Some((m, r)) = qualified {
                            out.cross_module += 1;
                            *synth.entry((m.clone(), r.clone())).or_insert_with(|| {
                                next_synth += 1;
                                let sym = SymbolId(next_synth);
                                names.insert(sym, (m, r));
                                sym
                            })
                        } else if s > 0 && names.contains_key(&SymbolId(s as usize)) {
                            SymbolId(s as usize)
                        } else if s > 0 {
                            // A symbol that is not one of this module's routines: a
                            // variable called, or a routine the resolver saw and the
                            // builder did not. Named as the resolver names it; the
                            // link will list it if nothing defines it.
                            let sym = SymbolId(s as usize);
                            let nm = symbol_name.get(&s).cloned().unwrap_or_else(|| {
                                format!("संज्ञा{}", riscv64::devanagari(s as i64))
                            });
                            names.entry(sym).or_insert((module.to_string(), nm));
                            sym
                        } else {
                            // The resolver's own exception: a qualified callee taken on
                            // trust. The callee NODE says which module and member.
                            let (m, r) = callee_of_node(it, arena_int(&callee_nodes, j))
                                .unwrap_or_else(|| (String::new(), "बाह्यम्".to_string()));
                            let (m, r) = if m.is_empty() {
                                (module.to_string(), r)
                            } else {
                                (m, r)
                            };
                            if m == module && routine_symbol.contains_key(&r) {
                                routine_symbol[&r]
                            } else {
                                out.cross_module += 1;
                                *synth.entry((m.clone(), r.clone())).or_insert_with(|| {
                                    next_synth += 1;
                                    let sym = SymbolId(next_synth);
                                    names.insert(sym, (m, r));
                                    sym
                                })
                            }
                        };
                        // One IR for both twins: the symbol the census chose is written
                        // back where the T1 twin reads it.
                        set_int(&ins, "संज्ञा", sym.0 as i128);
                        let a0 = usize::try_from(int_of(&ins, "आदानारम्भ")).unwrap_or(0);
                        let an = usize::try_from(int_of(&ins, "आदानसंख्यान")).unwrap_or(0);
                        if an == 0 {
                            out.zero_arg_calls += 1;
                        }
                        if an > 8 {
                            out.args_on_stack_sites += 1;
                            out.args_in_registers += 8;
                        } else {
                            out.args_in_registers += an;
                        }
                        let mut list = Vec::new();
                        for q in a0..a0 + an {
                            let arg = args.borrow()[q].clone();
                            let k = int_of(&arg, "क्रमाङ्क");
                            let k =
                                usize::try_from(k).ok().filter(|k| *k > 0).ok_or_else(|| {
                                    Hole(format!("{at} passes the absent value ० as argument {q}"))
                                })?;
                            list.push(ValueId(k - 1));
                        }
                        // `V-005`: `उपभेद` १ is a callee answering `प६४`.
                        if int_of(&ins, "उपभेद") != 1 {
                            Instruction::Call(sym, list)
                        } else {
                            Instruction::CallFloat(sym, list)
                        }
                    }
                    // `W-278`: a module-level global read. The symbol names the
                    // global; the label is its `{module}{name}`, and the word
                    // loaded is the one the declaring module exported.
                    // `W-field`: a record field read — one word at a byte offset from
                    // a RUNTIME base. The offset is carried in `ध्रुवमूल्यम्` and never
                    // in `स्थानक्रम`, which is a FRAME INDEX the emitter scales by ८
                    // and the frame-sizing pass maxes over — a byte offset there is
                    // wrong twice and loud neither time.
                    // `W-381` stage 4: the offset may be NEGATIVE (the bound check reads a run's
                    // length word at base − 8), carried as an i64's bits as `chain.rs` does; the
                    // old `u64::try_from(..).unwrap_or(0)` turned −8 into a silent 0.
                    19 => Instruction::LoadField(
                        ValueId(need(&ins, "वाम", &at)?),
                        i64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).map_or(0, i64::cast_unsigned),
                    ),
                    // BOTH operands are values here, where the kind above takes one
                    // value and a constant. A decoder that read `ध्रुवमूल्यम्`
                    // for the second would silently decode every index read as an
                    // offset of zero.
                    //
                    // **`W-294` — THIS IS ONE OF **FOUR** COPIES OF THIS DECODE, AND THE
                    // COMPILER IS THE ONLY REASON IT WAS FOUND.** `chain.rs:705`
                    // has the same arm, and `cargo check -p sadhana` passed with
                    // this one still two-field because it lives in `yantra`'s
                    // tests. I then reported "there are two decoders" to two
                    // lanes, and the commit hook's ALL-TARGETS clippy found two
                    // more — `crates/frontend/src/pathana.rs:502` and
                    // `crates/sadhana-t1/tests/t1_exec_riscv.rs:589`. **A
                    // per-crate check cannot count copies that live in crates it
                    // was not pointed at**, and I had counted with exactly such a
                    // check. Widening the variant turned a silent divergence into
                    // a build error; had the field been added as a mutable
                    // side-channel instead, this copy would have kept decoding
                    // every width as ० and the census would have disagreed with
                    // the chain about the same instruction, in silence.
                    //
                    // THE ० -> ८ MAPPING IS COPIED DELIBERATELY, not re-derived:
                    // two decoders that default differently are worse than two
                    // that default wrongly together, because only the first kind
                    // of disagreement is invisible to a twin comparison.
                    20 => Instruction::LoadIndex(
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                        match u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0)
                        {
                            0 => 8,
                            w => w,
                        },
                    ),
                    18 => Instruction::LoadGlobal(SymbolId(
                        usize::try_from(int_of(&ins, "संज्ञा")).unwrap_or(0),
                    )),
                    // `W-283`, the ruled storage model. `वाम` is the ADDRESS in 23
                    // and `दक्षिण` the value stored; swapping them writes the address
                    // into the value's storage, and both are live words, so nothing
                    // downstream would notice.
                    21 => Instruction::AddrOfGlobal(SymbolId(
                        usize::try_from(int_of(&ins, "संज्ञा")).unwrap_or(0),
                    )),
                    // `V-008`: `उपभेद` १ is a `प६४` slot's read (a run element, a field
                    // or a global declared `प६४`), into the FLOAT file — `LoadAt` first,
                    // as kind २२'s own (the transcription guard reads the first variant).
                    22 => {
                        let a = ValueId(need(&ins, "वाम", &at)?);
                        if int_of(&ins, "उपभेद") != 1 {
                            Instruction::LoadAt(a)
                        } else {
                            Instruction::LoadAtFloat(a)
                        }
                    }
                    // `W-306c` — AND `ध्रुवमूल्यम्` IS THE WIDTH IN OCTETS, a third
                    // thing beside the two values, exactly as it is on kind 20. A
                    // decoder that read it as an OPERAND would store at the
                    // address held in value ०.
                    //
                    // THE ० -> ८ MAPPING IS COPIED FROM KIND 20 DELIBERATELY, not
                    // re-derived: two decoders that default differently are worse
                    // than two that default wrongly together, because only the
                    // first kind of disagreement is invisible to a twin
                    // comparison. `ir.t1` does not write this field yet, so every
                    // instruction in the corpus arrives here as ० and leaves as ८
                    // — a whole word, today's behaviour, and the bare `निधानम्`.
                    23 => Instruction::StoreAt(
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                        match u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0)
                        {
                            0 => 8,
                            w => w,
                        },
                    ),
                    // `W-284` — THE STORAGE THE OTHER FIVE KINDS ADDRESS. 24
                    // allocates it, 25 and 26 form addresses INTO it, and 22/23
                    // spend them. The size is a build-time constant in
                    // `ध्रुवमूल्यम्` and there are NO value operands.
                    24 => Instruction::AllocRecord(
                        u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0),
                    ),
                    // 25 takes a value and a CONSTANT where 26 takes two values —
                    // the same asymmetry as 19 against 20. A decoder reading
                    // `ध्रुवमूल्यम्` for 26's second operand would make every index
                    // address offset zero, silently.
                    25 => Instruction::AddrOfField(
                        ValueId(need(&ins, "वाम", &at)?),
                        u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0),
                    ),
                    26 => Instruction::AddrOfIndex(
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                    ),
                    // `V-005` — a float op: `उपभेद` is the op (from १), `वाम` and
                    // `दक्षिण` its first two operands, `fmadd`'s third the one
                    // entry of its `आदानकोश` run (`chain.rs` decodes the same).
                    27 => {
                        let code = int_of(&ins, "उपभेद");
                        let op = FloatOp::from_code(code).ok_or_else(|| {
                            Hole(format!("{at}: float op {code} is not one of the thirteen"))
                        })?;
                        let mut list = vec![ValueId(need(&ins, "वाम", &at)?)];
                        if op.arity() >= 2 {
                            list.push(ValueId(need(&ins, "दक्षिण", &at)?));
                        }
                        if op.arity() == 3 {
                            let a0 = usize::try_from(int_of(&ins, "आदानारम्भ")).unwrap_or(0);
                            let arg = args.borrow()[a0].clone();
                            let k = int_of(&arg, "क्रमाङ्क");
                            let k =
                                usize::try_from(k).ok().filter(|k| *k > 0).ok_or_else(|| {
                                    Hole(format!(
                                        "{at}: fmadd's third operand is the absent value ०"
                                    ))
                                })?;
                            list.push(ValueId(k - 1));
                        }
                        Instruction::Float(op, list)
                    }
                    // `V-008` part 2 — a vector op: `उपभेद` the op (१..४, add, sub,
                    // mul, div), `वाम` the result run, `दक्षिण` the first operand run,
                    // the second the one entry of its `आदानकोश` run (`chain.rs`).
                    28 => {
                        let code = int_of(&ins, "उपभेद");
                        let op = FloatOp::from_code(code)
                            .filter(|op| {
                                matches!(
                                    op,
                                    FloatOp::Add | FloatOp::Sub | FloatOp::Mul | FloatOp::Div
                                )
                            })
                            .ok_or_else(|| {
                                Hole(format!(
                                    "{at}: vector op {code} is not add, sub, mul or div"
                                ))
                            })?;
                        let a0 = usize::try_from(int_of(&ins, "आदानारम्भ")).unwrap_or(0);
                        let arg = args.borrow()[a0].clone();
                        let k = int_of(&arg, "क्रमाङ्क");
                        let k = usize::try_from(k).ok().filter(|k| *k > 0).ok_or_else(|| {
                            Hole(format!(
                                "{at}: a vector op's second operand is the absent value ०"
                            ))
                        })?;
                        Instruction::Vector(
                            op,
                            vec![
                                ValueId(need(&ins, "वाम", &at)?),
                                ValueId(need(&ins, "दक्षिण", &at)?),
                                ValueId(k - 1),
                            ],
                        )
                    }
                    // `V-005`: `उपभेद` १ is a `प६४` parameter.
                    5 => {
                        let k = usize::try_from(int_of(&ins, "प्राचलक्रम")).unwrap_or(0);
                        if int_of(&ins, "उपभेद") != 1 {
                            Instruction::Param(k)
                        } else {
                            Instruction::ParamFloat(k)
                        }
                    }
                    // `W-245`: a local's slot, as the builder numbers it (from ०).
                    // `V-005`: `उपभेद` १ is a `प६४` local's read, the FLOAT load.
                    15 => {
                        let k = usize::try_from(int_of(&ins, "स्थानक्रम")).unwrap_or(0);
                        if int_of(&ins, "उपभेद") != 1 {
                            Instruction::Load(k)
                        } else {
                            Instruction::LoadFloat(k)
                        }
                    }
                    16 => Instruction::Store(
                        usize::try_from(int_of(&ins, "स्थानक्रम")).unwrap_or(0),
                        ValueId(need(&ins, "वाम", &at)?),
                    ),
                    // `W-254`: a string literal's octets, copied out of
                    // `पाठाक्षरकोश` by the run the instruction names. The arena
                    // is 1-BASED, as `आदानकोश` is — `पाठाक्षरयोजनम्` advances
                    // the cursor before it writes — so the run starts at
                    // `पाठारम्भ` itself and not one past it.
                    //
                    // An octet outside `0..=255` would mean the builder wrote
                    // something that is not an octet, so it is a hole rather
                    // than a truncation: `as u8` would turn a defect in `ir.t1`
                    // into a silently wrong image.
                    17 => {
                        let start = usize::try_from(int_of(&ins, "पाठारम्भ")).unwrap_or(0);
                        let count = usize::try_from(int_of(&ins, "पाठसंख्यान")).unwrap_or(0);
                        let end = start + count;
                        // A run past the end is a HOLE, not a truncation: it
                        // would mean `ir.t1` recorded a span it never wrote, and
                        // a short string is a wrong image rather than a loud one.
                        let bytes = string_octets.get(start..end).ok_or_else(|| {
                            Hole(format!(
                                "{at} names octets {start}..{end} and पाठाक्षरकोश holds {}",
                                string_octets.len()
                            ))
                        })?;
                        Instruction::ConstStr(bytes.to_vec())
                    }
                    kind => binary_kind(
                        kind,
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                        int_of(&ins, "उपभेद"),
                        int_of(&ins, "ध्रुवमूल्यम्"),
                    )
                    .unwrap_or_else(|| {
                        panic!("instruction kind {kind} is not one of the seventeen")
                    }),
                };
                out_insts.push((v, inst));
                out.insts += 1;
            }
            let term = member(&blk, "अवसानम्");
            let terminator = match int_of(&term, "भेद") {
                0 => None,
                1 => {
                    let k = int_of(&member(&term, "मूल्यम्"), "क्रमाङ्क");
                    Some(Terminator::Return(if k > 0 {
                        Some(ValueId(k as usize - 1))
                    } else {
                        None
                    }))
                }
                2 => Some(Terminator::Branch(BlockId(need(
                    &term,
                    "लक्ष्यम्",
                    &format!("the terminator of block {b}"),
                )?))),
                3 => Some(Terminator::Unreachable),
                4 => {
                    let at = format!("the terminator of block {b}");
                    Some(Terminator::CondBranch(
                        ValueId(need(&term, "मूल्यम्", &at)?),
                        BlockId(need(&term, "लक्ष्यम्", &at)?),
                        BlockId(need(&term, "अन्यलक्ष्यम्", &at)?),
                    ))
                }
                other => panic!("terminator kind {other} is not one of the four"),
            };
            let id = BlockId(b - 1);
            out_blocks.insert(
                id,
                Block {
                    id,
                    insts: out_insts,
                    terminator,
                },
            );
        }
        out.module.functions.push(Function {
            name: sym,
            blocks: out_blocks,
            entry_block: BlockId(need(&f, "प्रवेशपर्व", &format!("routine {i}"))?),
        });
    }
    // The entry: the module's FIRST ROUTINE WITH NO PARAMETER (§2.6). Until `W-245`
    // `कार्यक्रमरचना` emitted no `Param` (`ir.t1` (b)) and the first routine served
    // whatever it declared; now a parameter is a `Param`, the emitter refuses an
    // entry that takes one, and a module with no such routine is ENTRYLESS — its
    // stub calls nothing and halts 0x5555, counted as a run of status 0.
    // Recorded from the other side of the same discovery (`W-259`): this read
    // "the module's first routine", under the margin "`कार्यक्रमरचना` emits no
    // `Param`, so no routine takes parameters as the emitter sees them" — true
    // until a parameter loop existed, and the moment one did, thirteen of
    // fifteen sources stopped at `emit` with "the entry takes N parameters".
    // The refusal was right and the CHOICE was wrong; it had never been
    // exercised. `chain.rs`'s driver picks the same way.
    out.module.entry = out
        .module
        .functions
        .iter()
        .find(|f| {
            !f.blocks
                .values()
                .flat_map(|b| b.insts.iter())
                .any(|(_, i)| matches!(i, Instruction::Param(_)))
        })
        .map(|f| f.name);
    out.module.names = names;
    Ok(out)
}

/// The same name table, written into the T1 twin, and its entry set.
fn write_names_into_t1(it: &mut Interpreter, module: &Module) {
    it.call("यन्त्रोत्सर्जनॱयन्त्रनामारम्भः", vec![], 1_000_000)
        .expect("यन्त्रनामारम्भः runs");
    let mut sorted: Vec<(&SymbolId, &(String, String))> = module.names.iter().collect();
    sorted.sort_by_key(|(s, _)| s.0);
    for (sym, (m, n)) in sorted {
        it.call(
            "यन्त्रोत्सर्जनॱयन्त्रनामयोजनम्",
            vec![Value::Int(sym.0 as i128), octets(m), octets(n)],
            10_000_000,
        )
        .expect("यन्त्रनामयोजनम् runs");
    }
    if let Some(e) = module.entry {
        it.call(
            "यन्त्रोत्सर्जनॱयन्त्रप्रवेशन्यासः",
            vec![Value::Int(e.0 as i128)],
            1_000_000,
        )
        .expect("यन्त्रप्रवेशन्यासः runs");
    }
}

/// `Refusal`'s variant as the T1 module numbers them (`यन्त्र…निषेधभेद`, १..१२).
fn refusal_kind(r: &Refusal) -> i128 {
    match r {
        Refusal::Unreachable { .. } => 1,
        Refusal::NoTerminator { .. } => 2,
        Refusal::TargetNotInFunction { .. } => 3,
        Refusal::UnnamedSymbol { .. } => 4,
        Refusal::LabelCollision { .. } => 5,
        Refusal::ParamAfterCall { .. } => 6,
        Refusal::ParamOutsideEntry { .. } => 7,
        Refusal::FrameTooLarge { .. } => 8,
        Refusal::EntryTakesParameters { .. } => 9,
        Refusal::BranchOutOfRange { .. } => 10,
        Refusal::JumpOutOfRange { .. } => 11,
        Refusal::StoreWidthUnnamed { .. } => 12,
        Refusal::FileMismatch { .. } => 13,
    }
}

/// The first line where two texts differ, or `None` when they are the same octets.
fn first_divergence(t1: &str, rust: &str) -> Option<String> {
    let a: Vec<&str> = t1.lines().collect();
    let b: Vec<&str> = rust.lines().collect();
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            // DIAGNOSTIC CONTEXT, behind an env var so the census's own
            // message stays one line per source: a register number differing
            // says the allocators disagree and not WHERE, and the five lines
            // before it are what name the instruction that caused it.
            let ctx = if let Ok(w) = std::env::var("TWIN_CONTEXT") {
                let lo = i.saturating_sub(w.parse().unwrap_or(6));
                let mut c = String::from("\n");
                for j in lo..=i {
                    c.push_str(&format!(
                        "      {:>4} | {:<48} | {}\n",
                        j + 1,
                        a.get(j).unwrap_or(&""),
                        b.get(j).unwrap_or(&"")
                    ));
                }
                c
            } else {
                String::new()
            };
            return Some(format!(
                "line {}: the T1 twin wrote `{x}` where riscv64.rs wrote `{y}`{ctx}",
                i + 1
            ));
        }
    }
    if a.len() != b.len() {
        return Some(format!(
            "line {}: the T1 twin wrote {} lines, riscv64.rs {}",
            a.len().min(b.len()) + 1,
            a.len(),
            b.len()
        ));
    }
    (t1 != rust).then(|| "the lines agree and the octets do not (a line ending)".to_string())
}

// --- the chain, per source -------------------------------------------------------------

/// Assemble → link → round trip → load → run, on one module's text. Fills the row from
/// the assemble stage on and answers where it stopped.
/// Assemble one module's text into an object: the text's counts, and the names
/// it references without defining (`W-243`: cross-module callees, the link's
/// business through the exports).
fn assemble_stage(text: &str, row: &mut Row) -> Result<Vec<u8>, (Stage, String)> {
    row.text_lines = text.lines().count();
    row.text_instructions = text.lines().filter(|l| l.ends_with(" ।")).count();
    row.constant_pool = text
        .lines()
        .filter(|l| l.starts_with("ध्रुव") && l.ends_with("ॱॱ") && l != &"ध्रुवकोशःॱॱ")
        .count();
    let bytes = match assemble_object(
        text,
        Some(&row.module),
        Target::Uncompressed,
        false,
        Language::English,
    ) {
        Ok(b) => b,
        Err(ds) => {
            let named: Vec<String> = ds
                .iter()
                .take(3)
                .map(|d| {
                    let line = text.lines().nth(d.line.saturating_sub(1)).unwrap_or("");
                    format!("line {}: `{line}` — {}", d.line, d.reason)
                })
                .collect();
            return Err((Stage::Assemble, named.join("; ")));
        }
    };
    let Some(obj) = vastu::read(&bytes) else {
        return Err((Stage::Assemble, "the object does not read back".to_string()));
    };
    row.undefined = obj
        .symbols
        .iter()
        .filter(|s| s.is_undefined() && !s.name.is_empty())
        .map(|s| s.name.clone())
        .collect();
    Ok(bytes)
}

/// The name a link error of the form `` `X` is not defined by any object `` carries.
fn unresolved_name(error: &str) -> Option<String> {
    let rest = error.strip_prefix('`')?;
    let (name, tail) = rest.split_once('`')?;
    tail.contains("is not defined").then(|| name.to_string())
}

/// THE IMAGE (`W-243`, research/25 §5 R6): one startup object — the stub and the
/// stack, `e_entry` first — then this source's object, then every module object
/// its undefined labels REACH, transitively, through the exported routine labels
/// (`samyojana.rs`: the object's own names, then any export) — what a driver
/// links, and no more: an object with a label no module exports is that image's
/// stop, not every image's. Link, W-211's round trip over every word, load, run.
/// The symbols the records startup defines and the plain one does not — the
/// record region and its cursor, DERIVED rather than named. `W-286`.
///
/// Returns empty when either text does not assemble; the caller then falls
/// through to its ordinary unresolved report, which is the honest outcome —
/// a startup that will not assemble is a louder failure than a missing region.
fn region_symbols(plain_text: &str, records_text: &str) -> BTreeSet<String> {
    let exports = |text: &str| -> BTreeSet<String> {
        assemble_object(
            text,
            Some("यन्त्रारम्भ"),
            Target::Uncompressed,
            false,
            Language::English,
        )
        .ok()
        .and_then(|b| vastu::read(&b))
        .map(|o| {
            o.symbols
                .iter()
                .filter(|s| s.global && !s.is_undefined())
                .map(|s| s.name.clone())
                .collect()
        })
        .unwrap_or_default()
    };
    let plain = exports(plain_text);
    exports(records_text).difference(&plain).cloned().collect()
}

fn link_and_run(
    startup_text: &str,
    records_startup_text: &str,
    own_bytes: &[u8],
    others: &[vastu::Object],
    row: &mut Row,
) -> (Stage, String) {
    let own = own_bytes;
    let startup = match assemble_object(
        startup_text,
        Some("यन्त्रारम्भ"),
        Target::Uncompressed,
        false,
        Language::English,
    ) {
        Ok(b) => b,
        Err(ds) => {
            return (
                Stage::Link,
                format!(
                    "the startup object does not assemble: {}",
                    ds.iter()
                        .take(2)
                        .map(|d| format!("line {}: {}", d.line, d.reason))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            );
        }
    };
    let Some(startup) = vastu::read(&startup) else {
        return (
            Stage::Link,
            "the startup object does not read back".to_string(),
        );
    };
    let Some(own) = vastu::read(own) else {
        return (
            Stage::Link,
            "the module object does not read back".to_string(),
        );
    };
    // The closure: every export any other object offers, then the objects this
    // one's undefined names reach, and theirs, until nothing new is named.
    let undefined = |o: &vastu::Object| -> Vec<String> {
        o.symbols
            .iter()
            .filter(|s| s.is_undefined() && !s.name.is_empty())
            .map(|s| s.name.clone())
            .collect()
    };
    let mut exports: HashMap<String, usize> = HashMap::new();
    for (i, o) in others.iter().enumerate() {
        for s in &o.symbols {
            if s.global && !s.is_undefined() {
                exports.entry(s.name.clone()).or_insert(i);
            }
        }
    }
    // `W-245`: THE ROOT'S OWN EXPORTS ARE IN THE IMAGE TOO. The walk pulls in an
    // object and then extends `pending` with ITS undefined names — and one of
    // those may be a routine of the root module, which is in the image and in no
    // `other`. Before this row the corpus's IR held too few cross-module calls
    // for any pulled object to refer back; with the calls inside `चरः`
    // initialisers lowered, four sources reported their OWN routines unresolved
    // (`अक्षरकोशमानम्` in `sanskrit_text.t1`, whose object defines and exports
    // it). A name the root defines is resolved by the root.
    let own_exports: BTreeSet<String> = own
        .symbols
        .iter()
        .filter(|s| s.global && !s.is_undefined())
        .map(|s| s.name.clone())
        .collect();
    // AND SO ARE THE STARTUP'S — THE SAME OMISSION AS `W-245` ABOVE, ONE OBJECT
    // OVER. That row noticed the walk consulted neither `own` nor `others` for a
    // name the ROOT defines and fixed it for the root. The startup is the other
    // object in every image and it was left out, so this walk resolved names
    // against two of the image's three sources and refused the third's.
    //
    // WHAT IT COSTS: `:1265` below pushes `startup` into the image — AFTER this
    // loop has already returned `N unresolved`. So a symbol the image genuinely
    // defines is refused before `link_at` is ever called, and the refusal names
    // the symbol, which reads exactly like a real link error.
    //
    // MEASURED (`agent/assignfield`, seven censuses): with the record region
    // defined per module the walk SAW it — `13` sources refused it as "defined by
    // more than one object", which is a true defect. Moving the definition to the
    // startup (`9d1d1580`) fixed that (multi-def 13 → 0) and raised unresolved
    // 15 → 17, because it moved the definition into the one object this walk
    // cannot see. Three further commits chased the symbol's placement and moved
    // the outcome by exactly zero: 2 running and 17 unresolved at `9d1d1580`,
    // `5c972822` and `9eb36b5a` alike. The only wrong place was the only right
    // place.
    //
    // IT IS NOT ABOUT RECORDS AND IT IS NOT NEW. Main's startup already exports
    // `यन्त्रारम्भ` (`riscv64.rs`), so the blindness is latent here too; it stays
    // quiet only because no module REFERENCES that name — the ELF header reaches
    // the entry through `e_entry`, not through a module's code. The record region
    // is simply the first startup-defined symbol a module ever named.
    //
    // The real linker has no such walk: `t1_build` calls `link_objects` directly,
    // and `probe_store.t1` — a struct, `भवति ०`, a field write — links and writes
    // an ELF at `9d1d1580` with the module referencing the region at four sites
    // and the startup defining it. This walk is an approximation of that linker,
    // and an approximation that refuses what the real one accepts is a false red.
    let startup_exports: BTreeSet<String> = startup
        .symbols
        .iter()
        .filter(|s| s.global && !s.is_undefined())
        .map(|s| s.name.clone())
        .collect();
    let defined_in_image =
        |name: &String| own_exports.contains(name) || startup_exports.contains(name);
    let mut needed: BTreeSet<usize> = BTreeSet::new();
    let mut unresolved: BTreeSet<String> = BTreeSet::new();
    let mut pending = undefined(&own);
    while let Some(name) = pending.pop() {
        if defined_in_image(&name) {
            continue;
        }
        match exports.get(&name) {
            Some(&i) => {
                if needed.insert(i) {
                    pending.extend(undefined(&others[i]));
                }
            }
            None => {
                if !defined_in_image(&name) {
                    unresolved.insert(name);
                }
            }
        }
    }
    row.image_objects = 1 + needed.len();
    // ══════ `W-286` — THE IMAGE'S ANSWER, TAKEN WHERE THE IMAGE IS KNOWN ══════
    //
    // The startup is emitted per SOURCE and consumed per IMAGE. `W-285` made the
    // emitters TAKE the flag instead of reading a module-scoped global, which was
    // the right shape and left this caller still computing it over its own module
    // alone. Three sources then failed to link — `sanskrit_text`, `shrinkhala`,
    // `unparse` — each naming `रचनाक्षेत्रम्` and `रचनासूचकः`, and the
    // discriminator is exact: **all three declare ZERO record locals.** They do
    // not allocate; they are linked beside a module that does.
    //
    // SO THE QUESTION IS ASKED HERE, WHERE THE CLOSURE IS KNOWN, AND NOWHERE
    // EARLIER. Nothing above this walk can answer it: `chain_source` has one
    // module and no idea what will be pulled in beside it.
    //
    // THE REGION'S SYMBOLS ARE DERIVED, NOT NAMED. The set is exactly what the
    // records startup exports and the plain one does not, computed from the two
    // objects — so a rename of the region, a third symbol, or its removal all
    // flow through without touching this file. **A literal here would be a fourth
    // copy of a name that already exists in two halves, and the twin tests cannot
    // see a test-side literal at all.**
    //
    // AND THE RETRY IS DRIVEN BY `unresolved`, WHICH IS THE EVIDENCE ITSELF: an
    // object in this image referenced a symbol only the records startup defines.
    // That is not a proxy for "does the image allocate" — it IS the question,
    // read off the objects rather than predicted from them.
    if !unresolved.is_empty() && !records_startup_text.is_empty() {
        let only_with_records = region_symbols(startup_text, records_startup_text);
        if unresolved.iter().any(|n| only_with_records.contains(n)) {
            return link_and_run(records_startup_text, "", own_bytes, others, row);
        }
    }
    if !unresolved.is_empty() {
        row.unresolved = unresolved.into_iter().collect();
        return (
            Stage::Link,
            format!(
                "{} unresolved: {}",
                row.unresolved.len(),
                row.unresolved
                    .iter()
                    .take(6)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    let mut objects = Vec::with_capacity(needed.len() + 2);
    objects.push(startup);
    objects.push(own);
    objects.extend(needed.iter().map(|&i| others[i].clone()));
    let image = match link_at(&objects, LOAD) {
        Ok(i) => i,
        Err(es) => {
            row.unresolved = es.iter().filter_map(|e| unresolved_name(e)).collect();
            row.unresolved.sort();
            row.unresolved.dedup();
            let mut why = format!(
                "{} unresolved: {}",
                row.unresolved.len(),
                row.unresolved
                    .iter()
                    .take(6)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let other: Vec<&String> = es.iter().filter(|e| unresolved_name(e).is_none()).collect();
            if !other.is_empty() {
                let _ = write!(
                    why,
                    "; {}",
                    other
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                );
            }
            return (Stage::Link, why);
        }
    };
    // W-211's round trip over every word the machine will fetch.
    let mut at = 0;
    while at + 4 <= image.text.len() {
        row.words += 1;
        let word = u32::from_le_bytes(image.text[at..at + 4].try_into().unwrap());
        match decode_at(&image.text, at) {
            Some((decoded, width)) => {
                if reassemble(&decoded) == Some(word) {
                    row.words_back += 1;
                }
                at += width.max(2);
            }
            None => at += 4,
        }
    }
    let elf = write_debuggable_at(&image.text, &image.data, &image.table, image.bss, &[], LOAD);
    let mut m = match Machine::load_elf(&elf, yantra::ram_for(&elf)) {
        Ok(m) => m,
        Err(e) => return (Stage::Load, e),
    };
    let mut out = Vec::new();
    // `W-306`: WHERE THE STEPS WENT, not just how many — off by default.
    //
    // `T1_STEP_PROFILE=<name|all>` swaps `Machine::run` for
    // `yantra::profile::run_profiled`, which keeps the same halts and the same
    // budget and records the program counter each step began at. It is opt-in
    // because a `BTreeMap` write per instruction is real, and because the
    // question it answers is asked of ONE source at a time: the census's
    // `METRIC t1_run_steps` says `shrinkhala.t1` finishes on 48% of the budget
    // and cannot say whether that is a long program or one hot loop, and
    // `DEFAULT_STEPS`'s margin forbids sizing the constant off the total.
    //
    // `ashtaka` NAMES THE SAME SOURCE AS `ashtaka.t1`, because `T1_CORPUS` takes
    // the bare stem and this took the file name — measured 2026-09-28:
    // `T1_CORPUS=ashtaka T1_STEP_PROFILE=ashtaka` printed the census and not one
    // profile line, and a switch that silently does nothing reads exactly like a
    // source with no hot span.
    // ONE MATCHING RULE, AND IT LIVES IN `profile`. `armed` reports what this
    // switch reached, so a second spelling of "matched" here would let the
    // report and the switch disagree — which is the same silence by another
    // route.
    let profiling = std::env::var("T1_STEP_PROFILE")
        .is_ok_and(|v| yantra::profile::names_source(&v, &row.name));
    row.profiled = profiling;
    let halt = if profiling {
        let (halt, p) = yantra::profile::run_profiled(&mut m, BUDGET, &mut out);
        // The window is reported against the TEXT, because "half the steps in
        // 0.2% of the text" and "half the steps in half the text" are the two
        // answers the total hides and the ratio is what separates them.
        let text = image.text.len() as u64;
        // `W-306`: THE SPAN'S NAME. An offset is not somewhere anyone can go
        // read. The image's symbol table is right here, so lay its TEXT names
        // out as extents and ask which routine each window falls in — and let
        // it answer with two when the window genuinely straddles a boundary,
        // because a reading that always named one routine would name one
        // whether or not that were true.
        let text_syms: Vec<(String, u64)> = image
            .table
            .iter()
            .filter(|s| s.section == SymSection::Text)
            .map(|s| (s.name.clone(), s.value))
            .collect();
        let routines = yantra::profile::routines(&text_syms, LOAD, LOAD + text.saturating_sub(1));
        // TWO SHARES, NOT ONE. Half the steps in a narrow window says "there is a
        // loop"; NINE TENTHS in a window still narrow says the loop is the whole
        // run and there is nothing else to look at. One share cannot tell those
        // apart, and they call for different next work.
        for (numer, denom, what) in [(1u64, 2u64, "half"), (9, 10, "9/10")] {
            if let Some(span) = p.hot_span(numer, denom) {
                println!(
                    "METRIC t1_step_hotspan {} {} steps; {what} in {} octets ({} per mille \
                     of {} text) at +{:#x}, {} sites of {}",
                    row.name,
                    p.steps(),
                    span.width(),
                    span.width() * 1000 / text.max(1),
                    text,
                    span.lo - LOAD,
                    span.sites,
                    p.sites()
                );
                // Each owner with its own share, so "one hot loop" names the
                // routine or admits it spans more than one. `?` for a stretch
                // the symbol table does not cover: not the nearest name.
                let owners: Vec<String> = p
                    .owners(&span, &routines)
                    .iter()
                    .map(|o| {
                        format!(
                            "{}@+{:#x}:{}",
                            o.name.as_deref().unwrap_or("?"),
                            o.lo - LOAD,
                            o.steps
                        )
                    })
                    .collect();
                println!(
                    "METRIC t1_step_hotspan_owners {} {what} {} routine(s) of {}: {}",
                    row.name,
                    owners.len(),
                    routines.len(),
                    owners.join(" ")
                );
            }
        }
        // `W-306`: HOW MANY TIMES, beside how long. A routine holding half the
        // run reads the same whether it span once or was called ten thousand
        // times, and those call for opposite next work — `visit`'s entry count
        // is what separates them. `?` for a ratio there is no entry count for:
        // a routine whose `lo` the run never reached, which is how an extent
        // inferred up to the next NAME shows it annexed an unnamed neighbour.
        let busiest: Vec<String> = p
            .busiest(&routines, 6)
            .iter()
            .map(|v| {
                // `entries x per-entry = steps`, so the row states its own
                // arithmetic and a reader can see which factor is the large one.
                format!(
                    "{}:{}x{}={}/{}sites",
                    v.name,
                    v.entries,
                    v.steps_per_entry()
                        .map_or_else(|| "?".to_string(), |r| r.to_string()),
                    v.steps,
                    v.sites
                )
            })
            .collect();
        println!(
            "METRIC t1_step_busiest_routines {} {} of {}: {}",
            row.name,
            busiest.len(),
            routines.len(),
            busiest.join(" ")
        );
        let hottest: Vec<String> = p
            .hottest(5)
            .iter()
            .map(|(pc, n)| format!("+{:#x}={n}", pc - LOAD))
            .collect();
        println!("METRIC t1_step_hottest {} {}", row.name, hottest.join(" "));
        halt
    } else {
        m.run(BUDGET, &mut out)
    };
    // `W-306`: the steps the run took. `Machine::time` is one tick per
    // instruction the hart BEGINS and starts at zero, so after `run` returns it
    // is the step count — read here rather than threaded out of `run`, whose
    // signature eight other callers share. It is the clock either way: the
    // profiler counts the same event, and `step_profile.rs` pins them equal.
    row.steps = m.time;
    if let Halt::Finisher { status, .. } = &halt {
        row.status = *status;
    }
    let why = match &halt {
        Halt::Finisher { status, value } => {
            format!(
                "finisher status {status:?} (value {value:#x}), {} module object(s) in the image",
                row.image_objects
            )
        }
        other => format!(
            "{other:?}, {} module object(s) in the image",
            row.image_objects
        ),
    };
    row.halt = Some(halt);
    (Stage::Run, why)
}

/// `W-306`: WHAT `T1_STEP_PROFILE` ACTUALLY REACHED, printed whenever the switch
/// is set.
///
/// A profile that prints nothing has three causes and the output spelled only
/// one: it fired, it named a source this test never RUNS, or it named nothing at
/// all. Measured 2026-09-29 — `T1_FULL_CENSUS=1 T1_STEP_PROFILE=shrinkhala` on
/// `measure_corpus_encode` ran 209 s, PASSED and printed no profile line,
/// because `shrinkhala.t1` stops at LINK there. Measured again 2026-09-29 under
/// `T1_CORPUS=shrinkhala`: the stop is 33 UNRESOLVED cross-module labels, since
/// a narrowed run links a row only against the other rows of the same run. The
/// count travels in the line now. See [`yantra::profile::armed`].
///
/// Printed and not asserted: a census reports, and the request is the reader's.
fn armed_report(rows: &[Row]) -> Option<String> {
    let request = std::env::var("T1_STEP_PROFILE").ok()?;
    let reached: Vec<yantra::profile::Reached<'_>> = rows
        .iter()
        .map(|r| yantra::profile::Reached {
            source: &r.name,
            stage: r.encode_stop.name(),
            // `W-306`: the labels THIS run's corpus could not supply. `@link`
            // with a count is "widen T1_CORPUS"; `@link` with zero is "the stop
            // is not about the corpus", and the last cycle read the first as
            // the second.
            unresolved: r.unresolved.len(),
            profiled: r.profiled,
        })
        .collect();
    Some(format!(
        "METRIC t1_step_profile_armed {}",
        yantra::profile::armed(&request, &reached).report(&request)
    ))
}

/// `W-245`: every stub the builder still writes, by cause and by name, and every
/// shape it lowers — summed over the rows, as METRIC lines.
fn stub_report(rows: &[Row]) -> String {
    let mut report = String::new();
    let mut by_cause: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_shape: BTreeMap<&str, usize> = BTreeMap::new();
    for r in rows {
        for (k, n) in &r.stubs_by_cause {
            *by_cause.entry(k).or_insert(0) += n;
        }
        for (k, n) in &r.lowered_by_shape {
            *by_shape.entry(k).or_insert(0) += n;
        }
    }
    let _ = writeln!(
        report,
        "METRIC paradigm_ir_stubs {}",
        by_cause.values().sum::<usize>()
    );
    for (k, n) in &by_cause {
        let _ = writeln!(report, "METRIC paradigm_ir_stub_{k} {n}");
    }
    let _ = writeln!(
        report,
        "METRIC paradigm_ir_lowered {} # sites W-245's arms lower that were stubs before it",
        by_shape.values().sum::<usize>()
    );
    for (k, n) in &by_shape {
        let _ = writeln!(report, "METRIC paradigm_ir_lowered_{k} {n}");
    }
    report
}

/// One text through every machine stage as an image of its own: assemble, then
/// link behind `startup_text` with `others`, load, run.
fn machine_stages(
    text: &str,
    startup_text: &str,
    records_startup_text: &str,
    others: &[vastu::Object],
    row: &mut Row,
) -> (Stage, String) {
    match assemble_stage(text, row) {
        Ok(bytes) => link_and_run(startup_text, records_startup_text, &bytes, others, row),
        Err(stop) => stop,
    }
}

/// `W-243`: every assembled source's image, linked and run — after the whole
/// corpus is assembled, because each image holds every other module's object.
// The index is the point: row `i` is written while every other row's object is
// read, and `i` is what tells them apart.
#[allow(clippy::needless_range_loop)]
fn link_images(rows: &mut [Row]) {
    let objects: Vec<Option<vastu::Object>> = rows
        .iter()
        .map(|r| r.object.as_deref().and_then(vastu::read))
        .collect();
    for i in 0..rows.len() {
        let Some(own) = rows[i].object.clone() else {
            continue;
        };
        let others: Vec<vastu::Object> = objects
            .iter()
            .enumerate()
            .filter(|(j, o)| *j != i && o.is_some())
            .map(|(_, o)| o.clone().expect("filtered"))
            .collect();
        let startup = rows[i].startup.clone();
        let startup_records = rows[i].startup_records.clone();
        let (stop, why) = link_and_run(&startup, &startup_records, &own, &others, &mut rows[i]);
        rows[i].encode_stop = stop;
        rows[i].encode_why = why;
    }
}

/// The whole chain over one source. `with_t1_twin`: also run `यन्त्रोत्सर्जन` under the
/// interpreter and compare (the census); without it the Rust twin's text is what the
/// machine runs (the pin) — the same octets, by the census's assertion.
/// `W-253` threads ONE interpreter through the corpus and `W-245` runs a
/// fixture from TEXT; both are kept — the interpreter is a parameter, and the
/// source is a parameter of the inner form.
fn chain(it: &mut Interpreter, name: &str, with_t1_twin: bool) -> Row {
    let src = source(name);
    chain_source(it, name, &src, with_t1_twin)
}

/// [`chain`] over a source TEXT — the corpus file, or a fixture built from one
/// (`W-245`'s corpus routine).
#[allow(clippy::too_many_lines)]
fn chain_source(it: &mut Interpreter, name: &str, src: &str, with_t1_twin: bool) -> Row {
    let src = src.to_string();
    let mut row = Row {
        name: name.to_string(),
        module: module_name(&src),
        ..Row::default()
    };
    let toks = match it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
    {
        Ok(v) => v.as_int().unwrap_or(0),
        Err(e) => {
            row.encode_stop = Stage::Lex;
            row.encode_why = format!("{e:?}");
            return row;
        }
    };
    let parsed = match it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
    {
        Ok(v) => v.as_int().unwrap_or(0),
        Err(e) => {
            row.encode_stop = Stage::Parse;
            row.encode_why = format!("{:.120}", format!("{e:?}"));
            return row;
        }
    };
    row.decls = parsed;
    if parsed == 0 {
        row.encode_stop = Stage::Parse;
        row.encode_why = "parse produced no declarations".to_string();
        return row;
    }
    let resolver = match it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
    {
        Ok(r) => r,
        Err(e) => {
            row.encode_stop = Stage::Resolve;
            row.encode_why = format!("{e:?}");
            return row;
        }
    };
    match it.call(
        "अर्थॱकार्यक्रमनिर्णयः",
        vec![resolver.clone(), Value::Int(parsed)],
        4_000_000_000,
    ) {
        Ok(Value::Bool(true)) => {}
        other => {
            row.encode_stop = Stage::Resolve;
            // `SAS-013` PART TWO: the census reads the site through
            // `chain::resolve_site` rather than re-rendering half of it. The copy that
            // stood here knew only `अनिर्णीतमस्ति`, so a corpus source that
            // declared a name twice would have been censused with `encode_why` =
            // `Bool(false)` — a stop with no site, in the column that exists to hold one.
            row.encode_why = chain::resolve_site(it).unwrap_or_else(|| format!("{other:?}"));
            return row;
        }
    }
    // TYPECHECK — the owner's fourth stage. Its verdict is recorded and does not stop
    // the emitter's side: the checker does not change the IR, and a source it refuses
    // still has an emitted text whose assembly and run are facts worth measuring. The
    // `chain` stop is what the checker gates; the `encode` stop is the emitter's.
    // ZERO THE CHECKER'S STATE FOR THIS SOURCE. W-253: this census resets the
    // resolver (`निर्णायकारम्भः`) and the IR builder (`मध्यरूपॱआरम्भः`) per source and
    // never reset the checker — harmless only while a FRESH INTERPRETER per source
    // did the zeroing, which is the same unstated mechanism that made
    // `t1_execution.rs`'s counters read a triangular over-count when its census
    // began sharing one. DEFENSIVE HERE AND NOT A FIX: this census reads no summed
    // counter, and a verdict provably does not move without it
    // (`a_checker_state_that_persists_does_not_move_a_later_programs_verdict`).
    // What it does prevent is a later poison's recorded SITE being attributed to
    // an earlier program.
    //
    // THE REASON WAS RE-FOUNDED 2026-09-05 (`W-275`); THE CONCLUSION DID NOT MOVE.
    // SUPERSEDED, and kept because W-253's margin asked to be contradicted and
    // earned it: "the fold sets `पुच्छे` true before its last child, so a
    // completed typecheck ends with the value the reset would write". MEASURED
    // FALSE. W-253 drove only sources the checker ACCEPTS; this census drives a
    // corpus that CONTAINS REFUSED ONES, which is exactly the case the sentence
    // does not cover. Three sources through the checker, reading `पुच्छे` after:
    //     accepted (several statements)      verdict true    पुच्छे TRUE
    //     refused, body/return disagree      verdict false   पुच्छे TRUE
    //     refused, non-बूल `यदि` condition   verdict false   पुच्छे FALSE
    // So it is false for SOME refusals and not all — the refusal has to happen
    // inside the block fold, after `पुच्छे भवति असत्यम् ।` and before the
    // `भवति सत्यम् ।` that precedes the last child.
    //
    // WHAT ACTUALLY HOLDS: NO RULE READS THE INCOMING VALUE. `पुच्छे` has one
    // reader, `वाक्यप्रकारः`'s `यदि` arm, and every block arm writes the flag
    // immediately before typing a child — so the value a program inherits is
    // overwritten before anything consults it. Measured the way the superseded
    // sentence should have been: run the non-बूल refusal FIRST so the flag comes
    // in FALSE, then an accepting source with NO reset, against a fresh-and-reset
    // reference. Verdict unchanged, for a plain body and for one containing
    // `यदि`/`अन्यथा` — which is the reader's own shape.
    //
    // The reset therefore stays, and stays defensive. It is worth having the
    // right reason on it: someone who checked the old one against a refused
    // source would find it false and might conclude the reset is load-bearing —
    // or, reading the other way, that a false claim guards nothing and the
    // reset can go.
    let _ = it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![resolver.clone()], 5_000_000);
    row.typecheck = Some(
        match it.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000,
        ) {
            Ok(Value::Bool(true)) => Ok(()),
            Ok(_) if global_bool(it, "प्रकारदोषमस्ति") => Err(format!(
                "`{}`: body kind {} vs return kind {}",
                global_text(it, "प्रकारदोषनाम"),
                global_int(it, "प्रकारदोषशरीरभेद"),
                global_int(it, "प्रकारदोषप्रत्यागमनभेद")
            )),
            Ok(v) => Err(format!("answered {v:?} and named no cause")),
            Err(e) => Err(format!("{:.120}", format!("{e:?}"))),
        },
    );
    if it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000).is_err() {
        row.encode_stop = Stage::Ir;
        row.encode_why = "मध्यरूपॱआरम्भः would not run".to_string();
        return row;
    }
    if let Err(e) = it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
    {
        row.encode_stop = Stage::Ir;
        row.encode_why = format!("{:.120}", format!("{e:?}"));
        return row;
    }
    row.ir_partial = global_bool(it, "अनिर्णीताह्वानमस्ति");
    if row.ir_partial {
        row.encode_stop = Stage::Ir;
        row.encode_why = format!(
            "refused: a call's callee has no symbol, line {}",
            token_line(it, global_int(it, "अनिर्णीताह्वानचिह्नकाङ्क"))
        );
        return row;
    }
    row.routines = usize::try_from(global_int(it, "वृत्तिसूचकाङ्क")).unwrap_or(0);
    if row.routines == 0 {
        // **`no_routines` IS DECIDED BY THE PARSER'S DECLARATION KINDS, NOT BY A
        // SEARCH FOR A SPELLING.** It read `!src.contains("वृत्तिः ")` — a guard
        // naming a SPELLING rather than a condition, so a source with that word
        // in a MARGIN took the other branch and reported "ran, appended nothing"
        // for a source that declares nothing at all. `ast.t1` happens to contain
        // the string ZERO times, so the row was right today by luck of spelling.
        //
        // **AND IT IS NOT `row.routines == 0` EITHER, THOUGH THAT WAS THE
        // OBVIOUS ONE-LINE FIX.** We are already inside `if row.routines == 0`,
        // so that would make this always true and the `else` below DEAD —
        // discarding a real distinction: a source that declares NO routine is a
        // different fact from one that declares routines the builder then
        // appended none of. Neither source hits the second today (zero in the
        // clean-base census), which is exactly why deleting it would be silent.
        //
        // The mechanism is the parser's own: walk `घोषणाकोश` and count
        // declarations whose `भेद` is `वृत्तिघोषणाभेद`. Same arena, same kind
        // constant the parser writes at `parse.t1:1302`.
        let decls = arena(it, "घोषणाकोश");
        let routine_kind = global_int(it, "वृत्तिघोषणाभेद");
        let declared = usize::try_from(global_int(it, "घोषणासूचकाङ्क")).unwrap_or(0);
        let declared_routines = (1..=declared)
            .filter(|i| {
                decls
                    .borrow()
                    .get(*i)
                    .map(|d| int_of(d, "भेद") == routine_kind)
                    .unwrap_or(false)
            })
            .count();
        row.no_routines = declared_routines == 0;
        // **NO ROUTINES IS NOT NOTHING TO BUILD. A MODULE'S DATA IS SOMETHING.**
        // `वास्तु` declares 45 globals and not one routine, and six modules read
        // them — reads `ir.t1`'s cause-44 arm refuses ON PURPOSE, because the
        // declaring module yields no object for the linker to resolve against.
        //
        // The `.t1` emitter has always been able to emit that object:
        // `यन्त्रमण्डलोत्सर्जनम्` calls `यन्त्रदत्तोत्सर्जनम्` AFTER its routine loop
        // and unconditionally, so a zero-routine module skips the loop and still
        // emits its data. **It was never asked** — this early return and
        // `chain.rs:430` were the two scaffolding refusals, and nothing in the
        // product half needed changing.
        let globals = usize::try_from(global_int(it, "वैश्विकसञ्चयसूचकाङ्क")).unwrap_or(0);
        if globals == 0 {
            row.encode_stop = Stage::Ir;
            row.encode_why = if row.no_routines {
                // AND THE SENTENCE CHANGED WITH THE CONDITION. It said "no
                // routines — nothing to build", which is now only half the
                // reason: `vastu.t1` has no routines AND no data, and `ast.t1`
                // has no routines and 45 globals. A row that keeps an old
                // sentence for a new reason is lying about why.
                "no routines and no data — nothing to build".to_string()
            } else {
                "ran, appended nothing".to_string()
            };
            return row;
        }
    }
    let read = match read_module(it, &row.module.clone(), &resolver) {
        Ok(r) => r,
        Err(Hole(why)) => {
            row.encode_stop = Stage::Ir;
            row.encode_why = format!("unmeasurable: {why}");
            return row;
        }
    };
    row.insts = read.insts;
    row.calls = read.calls;
    row.zero_arg_calls = read.zero_arg_calls;
    row.cross_module_calls = read.cross_module;
    row.growth_routines = read.growth_routines;
    row.zero_constants = read.zero_constants;
    row.stub_constants = read.stub_constants;
    row.stubs_by_cause = read.stubs_by_cause;
    row.lowered_by_shape = read.lowered_by_shape;
    row.lowered_shape_reads = read.lowered_shape_reads;
    row.args_in_registers = read.args_in_registers;
    row.args_on_stack_sites = read.args_on_stack_sites;

    let rust = riscv64::emit_module_and_relaxations(&read.module);
    let mut text = match &rust {
        Ok((t, relaxed)) => {
            row.relaxed_routines = relaxed.len();
            t.clone()
        }
        Err(r) => {
            row.encode_stop = Stage::Emit;
            row.encode_why = format!("{r}");
            String::new()
        }
    };
    if with_t1_twin {
        write_names_into_t1(it, &read.module);
        let started = Instant::now();
        let t1 = match it.call("यन्त्रोत्सर्जनॱयन्त्रमण्डलोत्सर्जनम्", vec![], 60_000_000_000)
        {
            Ok(v) => text_of(&v),
            Err(e) => {
                row.twin = Some(Err(format!(
                    "the T1 twin did not finish: {:.120}",
                    format!("{e:?}")
                )));
                if rust.is_err() {
                    return row;
                }
                String::new()
            }
        };
        row.t1_seconds = started.elapsed().as_secs_f64();
        if row.twin.is_none() {
            let t1_refused = global_bool(it, "यन्त्रनिषेधमस्ति");
            row.twin = Some(match (&rust, t1_refused) {
                (Ok((r, _)), false) => match first_divergence(&t1, r) {
                    None => Ok(r.len()),
                    Some(d) => Err(d),
                },
                (Err(r), true) => {
                    let k = global_int(it, "यन्त्रनिषेधभेद");
                    if k == refusal_kind(r) {
                        Ok(0)
                    } else {
                        Err(format!("Rust refused {r}, T1 kind {k}"))
                    }
                }
                (Ok(_), true) => Err(format!(
                    "only the T1 twin refused, kind {}",
                    global_int(it, "यन्त्रनिषेधभेद")
                )),
                (Err(r), false) => Err(format!("only riscv64.rs refused: {r}")),
            });
            // What the machine runs is the T1 twin's text when it agreed (§4 step 3).
            if matches!(row.twin, Some(Ok(n)) if n > 0) {
                text = t1;
            }
        }
    }
    if rust.is_err() {
        return row;
    }
    // `W-243`: the startup object, compared between the twins like the module text.
    let entry_label = read
        .module
        .entry
        .and_then(|e| riscv64::routine_label(&read.module.names, e).ok());
    let rust_startup = riscv64::emit_startup_object_with_records(
        entry_label.as_deref(),
        riscv64::module_allocates(&read.module),
    );
    let mut startup = rust_startup.clone();
    if with_t1_twin && matches!(row.twin, Some(Ok(n)) if n > 0) {
        match it.call(
            "यन्त्रोत्सर्जनॱयन्त्रारम्भमण्डलोत्सर्जनम्",
            vec![Value::Bool(riscv64::module_allocates(&read.module))],
            100_000_000,
        ) {
            Ok(v) => {
                let t1 = text_of(&v);
                match first_divergence(&t1, &rust_startup) {
                    None => startup = t1,
                    Some(d) => row.twin = Some(Err(format!("startup object: {d}"))),
                }
            }
            Err(e) => {
                row.twin = Some(Err(format!(
                    "the T1 startup object did not finish: {:.120}",
                    format!("{e:?}")
                )));
            }
        }
    }
    // `W-286`: the same startup WITH the region, carried so the link step can
    // pick when the IMAGE turns out to need one. Emitted from the Rust half
    // only — the twin comparison above has already agreed the two emitters write
    // the same octets for this source's own answer, and re-running the T1
    // emitter for a flag this module does not need would cost a second
    // interpreter call per source for a text most images discard.
    let startup_records = riscv64::emit_startup_object_with_records(entry_label.as_deref(), true);
    row.entry_label = entry_label;
    row.startup = startup;
    row.startup_records = startup_records;
    match assemble_stage(&text, &mut row) {
        Ok(bytes) => {
            row.object = Some(bytes);
            // Assembled; the image is linked once the corpus is (`link_images`).
            row.encode_stop = Stage::Link;
            row.encode_why = "assembled, awaiting the image".to_string();
        }
        Err((stop, why)) => {
            row.encode_stop = stop;
            row.encode_why = why;
        }
    }
    row.text = text;
    row
}

fn run_line(row: &Row) -> String {
    let (chain_stop, chain_why) = row.chain_stop();
    let typecheck = match &row.typecheck {
        None => "-".to_string(),
        Some(Ok(())) => "ok".to_string(),
        Some(Err(w)) => format!("REFUSED {w}"),
    };
    let twin = match &row.twin {
        None => String::new(),
        Some(Ok(n)) => format!("  twin AGREE {n} octets ({:.1}s)", row.t1_seconds),
        Some(Err(d)) => format!("  twin DIVERGE {d}"),
    };
    format!(
        "RUN {:<20} module {:<14} decls {:>3} routines {:>3} insts {:>5} calls {:>4} (zero-arg {:>3}, cross-module {:>3})  typecheck {}  chain-stop {}  encode-stop {} — {}{}",
        row.name,
        row.module,
        row.decls,
        row.routines,
        row.insts,
        row.calls,
        row.zero_arg_calls,
        row.cross_module_calls,
        typecheck,
        if chain_stop == Stage::Typecheck {
            format!("typecheck ({chain_why})")
        } else {
            chain_stop.name().to_string()
        },
        row.stop_word(),
        row.encode_why,
        twin
    )
}

fn sas_programs() -> usize {
    std::fs::read_dir(root().join("tests/corpus/t1"))
        .expect("tests/corpus/t1")
        .filter(|e| {
            e.as_ref()
                .expect("entry")
                .path()
                .extension()
                .is_some_and(|x| x == "सस")
        })
        .count()
}

// --- the census ------------------------------------------------------------------------

/// **THE CENSUS.** `cargo test -p yantra --test paradigm_encode -- --ignored --nocapture`
/// prints one `RUN` line per `.t1` source, one `METRIC paradigm_encode_<stat> <value>` line
/// per statistic, and asserts what §4 says is asserted: the twins agree on every emitted
/// module, and the `.सस` are 0 readable by design.
#[test]
#[ignore = "a census, not a check: the whole corpus and both emitters under the interpreter, minutes"]
#[allow(clippy::too_many_lines)]
fn measure_corpus_encode() {
    if !walk_allowed("measure_corpus_encode") {
        return;
    }
    let clock = Instant::now();
    let names = corpus();
    let mut rows: Vec<Row> = Vec::new();
    // ONE interpreter with the store filled (W-253), shared across every source.
    let mut it = load_chain_collected();
    for n in &names {
        rows.push(chain(&mut it, n, true));
    }
    // `W-243`: the images — every assembled source's, each holding every module's
    // object behind one startup object — linked and run once the corpus is assembled.
    link_images(&mut rows);
    for row in &rows {
        println!("{}", run_line(row));
    }
    if let Some(line) = armed_report(&rows) {
        println!("{line}");
    }
    let mut report = String::new();
    let count = |f: &dyn Fn(&Row) -> bool| rows.iter().filter(|r| f(r)).count();
    let sum = |f: &dyn Fn(&Row) -> usize| rows.iter().map(f).sum::<usize>();
    let _ = writeln!(report, "METRIC paradigm_encode_t1_sources {}", rows.len());
    // `W-332` — THE TWO READINGS ARE BOTH NEEDED AND THE SECOND IS THE GUARD.
    // The sum alone cannot tell "nothing relaxed" from "nothing was emitted":
    // a corpus that refused at every source also sums to 0, and so does a
    // corpus of sources that never reach the emitter at all (`ast.t1`,
    // `vastu.t1` and `lib.t1` build no IR). The second line is the DENOMINATOR
    // — the sources the Rust emitter actually lowered — and it is what makes
    // the first a measurement rather than a silence.
    let _ = writeln!(
        report,
        "METRIC t1_relaxed_routines {}",
        sum(&|r| r.relaxed_routines)
    );
    let _ = writeln!(
        report,
        "METRIC t1_relaxation_measured_sources {}",
        count(&|r| r.encode_stop > Stage::Emit)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_lexed {}",
        count(&|r| r.encode_stop > Stage::Lex)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_parsed {}",
        count(&|r| r.encode_stop > Stage::Parse)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_resolved {}",
        count(&|r| r.encode_stop > Stage::Resolve)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_typechecked {}",
        count(&|r| r.typechecked())
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_ir_built {}",
        count(&|r| r.routines > 0 && !r.ir_partial)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_ir_partial {}",
        count(&|r| r.ir_partial)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_nothing_to_build {}",
        count(&|r| r.no_routines)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_emitted {}",
        count(&|r| r.encode_stop > Stage::Emit)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_emit_refused {}",
        count(&|r| r.encode_stop == Stage::Emit)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_assembled {}",
        count(&|r| r.assembled())
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_twin_agreement {}/{}",
        count(&|r| matches!(r.twin, Some(Ok(_)))),
        count(&|r| r.twin.is_some())
    );
    // `W-243`: the labels a module references and does not define — cross-module
    // callees — are resolved by the image's link through the other modules'
    // exported routine labels; what the link still cannot resolve is the stop.
    let cross: BTreeSet<String> = rows
        .iter()
        .flat_map(|r| r.undefined.iter().cloned())
        .collect();
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_cross_module_labels {}",
        cross.len()
    );
    let unresolved: BTreeSet<String> = rows
        .iter()
        .flat_map(|r| r.unresolved.iter().cloned())
        .collect();
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_unresolved_labels {}{}",
        unresolved.len(),
        if unresolved.is_empty() {
            String::new()
        } else {
            format!(
                " # {}",
                unresolved.iter().cloned().collect::<Vec<_>>().join(", ")
            )
        }
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_unresolved_label_sites {}",
        sum(&|r| r.unresolved.len())
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_label_collisions {}",
        count(&|r| r.encode_stop == Stage::Emit && r.encode_why.contains("is both"))
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_linked_images {}",
        count(&|r| r.encode_stop > Stage::Link)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_loaded {}",
        count(&|r| r.encode_stop > Stage::Load)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_runs {}",
        count(&|r| r.encode_stop == Stage::Run)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_halted_finisher {}",
        count(&|r| r.ran())
    );
    let mut ends: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rows {
        if let Some(h) = &r.halt {
            *ends.entry(halt_kind(h)).or_insert(0) += 1;
        }
    }
    for (k, n) in &ends {
        let _ = writeln!(report, "METRIC paradigm_encode_ends_{k} {n}");
    }
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_status_zero {} # printed, not asserted: what remains stubbed is counted below by cause",
        count(&|r| r.status == Some(0))
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_status_nonzero {}",
        count(&|r| matches!(r.status, Some(s) if s != 0))
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_instructions {}",
        sum(&|r| r.text_instructions)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_ir_instructions {}",
        sum(&|r| r.insts)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_zero_constants {}",
        sum(&|r| r.zero_constants)
    );
    // `W-245`: the zero constants split — the builder's stubs against a written ० —
    // and every stub the builder still writes, by cause and by name.
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_stub_constants {}",
        sum(&|r| r.stub_constants)
    );
    // `W-259`'s two metrics, kept BY NAME because they are the numbers this
    // project has been quoted, and read from the CAUSE rather than from a second
    // arena: a `ConstInt(0)` the builder stubbed carries a cause, and one the
    // source wrote carries none.
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_zero_constants_unlowered {}",
        sum(&|r| r.stub_constants)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_zero_constants_from_numeral {}",
        sum(&|r| r.zero_constants - r.stub_constants)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_literal_zeros {}",
        sum(&|r| r.zero_constants - r.stub_constants)
    );
    report.push_str(&stub_report(&rows));
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_functions {}",
        sum(&|r| if r.encode_stop > Stage::Emit {
            r.routines
        } else {
            0
        })
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_frames {}",
        sum(&|r| if r.encode_stop > Stage::Emit {
            r.routines
        } else {
            0
        })
    );
    let _ = writeln!(report, "METRIC paradigm_encode_calls {}", sum(&|r| r.calls));
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_zero_argument_calls {}",
        sum(&|r| r.zero_arg_calls)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_cross_module_calls {}",
        sum(&|r| r.cross_module_calls)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_args_in_a_registers {}",
        sum(&|r| r.args_in_registers)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_args_on_stack {}",
        sum(&|r| r.args_on_stack_sites)
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_constant_pool {}",
        sum(&|r| r.constant_pool)
    );
    let words = sum(&|r| r.words);
    let back = sum(&|r| r.words_back);
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_roundtrip {:.4} # {back} of {words} words decode back",
        if words == 0 {
            1.0
        } else {
            back as f64 / words as f64
        }
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_sas_programs {}",
        sas_programs()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_sas_readable 0 # BY DESIGN: research/25 §1.4a, W-238 — a retired dialect (ॐ, ॥ मण्डलम् ॥, फलम्; 0 आदाय), read by no chain"
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_t1_runnable_on_yantra {}",
        count(&|r| r.ran())
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_chain_on_yantra {} # ran AND the checker accepted the source",
        count(&|r| r.ran() && r.typechecked())
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_t1_emitter_seconds {:.0}",
        rows.iter().map(|r| r.t1_seconds).sum::<f64>()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_encode_census_seconds {:.0}",
        clock.elapsed().as_secs_f64()
    );
    for n in &unresolved {
        let _ = writeln!(report, "UNRESOLVED {n}");
    }
    print!("{report}");

    // Asserted by design (§4): the twins agree wherever both emitted, and the `.सस`
    // are counted, not read.
    // ══ `W-288` — THE TABLE PRINTS BEFORE EVERY GUARD, AND THAT IS THE WHOLE
    // CHANGE. It used to print at the END of this test, after five assertions.
    //
    // **A DIAGNOSTIC'S SURVIVAL OF A RED IS DECIDED BY STATEMENT ORDER, AND
    // NOTHING DECLARED IT.** Two tests in this file print this same table:
    // `measure_corpus_encode` accumulates into a string and flushes BEFORE its
    // guards, so a red there still yields the by-cause figure; this one printed
    // AFTER, so a red here yielded nothing. Same file, same table, opposite
    // outcomes, and nobody chose it.
    //
    // MEASURED, TWICE, IN ONE EVENING: a census that stopped at the twin guard
    // reported NO metric lines at all and `assign_field` was unanswerable; a
    // census that stopped later reported the full table. Both cost ~2,000 s.
    // **"Not measured because the run went red" was a statement about ordering,
    // not about the run** — and it was said to the owner twice.
    //
    // SO THE RULE, AND IT IS CHEAPER THAN ANY NEW INSTRUMENT: emit a diagnostic
    // as soon as it is computable, never after the assertions that might not
    // survive to reach it. The stub counters are complete once the IR is built
    // — emit, assemble, link and run contribute nothing to them — so there is
    // no correctness reason for this to have been last.
    print!("{}", stub_report(&rows));

    let divergent: Vec<String> = rows
        .iter()
        .filter_map(|r| match &r.twin {
            Some(Err(d)) => Some(format!("{}: {d}", r.name)),
            _ => None,
        })
        .collect();
    assert!(
        divergent.is_empty(),
        "the twins diverge on {} sources:\n  {}",
        divergent.len(),
        divergent.join("\n  ")
    );
    assert_eq!(
        sas_programs(),
        pins::SAS_PROGRAMS,
        "the retired dialect's 16 programs"
    );
    assert_eq!(
        words, back,
        "every word of every linked image decodes back (W-211's round trip)"
    );
}

/// **THE PIN — not ignored.** The chain over every source with the Rust twin, compared
/// with `paradigm/pins.rs`: the assembled count, the runnable count, and EVERY stopped
/// source by name with its stage. A regression fails by name; a source that starts
/// reaching further also fails, so the pin is re-measured and the commit says so.
///
/// A source that assembles but halts on a fault is COUNTED at the run stage, in
/// `T1_ASSEMBLED_NOT_ON_YANTRA` with its halt — never folded into "assembled".
#[test]
fn the_assembled_count_is_pinned_and_every_stopped_source_is_named() {
    if !walk_allowed("the_assembled_count_is_pinned_and_every_stopped_source_is_named") {
        return;
    }
    let names = corpus();
    // A COMPLETENESS PIN IS MEANINGLESS OVER A DELIBERATE SUBSET, and asserting
    // it there would make `T1_CORPUS` useless — this is the FIRST statement in
    // the test, so it fires before a single figure is computed and a narrowed
    // run prints nothing at all. Skipping it is right; skipping it QUIETLY is
    // not, because "the corpus is complete" is exactly the kind of guarantee a
    // later reader assumes held. The narrowing already announces itself on
    // stderr from `corpus()`; this names the specific pin that stood down.
    if std::env::var("T1_CORPUS").is_ok() {
        eprintln!(
            "!! T1_CENSUSED completeness pin ({} sources) SKIPPED — narrowed to {}",
            pins::T1_CENSUSED,
            names.len()
        );
    } else {
        // `T1_CENSUSED`, NOT `T1_SOURCES`: `corpus()` above drops the generated
        // sources from the full walk and names each on stderr, so the set this
        // pin is complete OVER is the authored one. Comparing against the
        // directory count would make this test red for a file the walk itself
        // decided not to measure.
        assert_eq!(
            names.len(),
            pins::T1_CENSUSED,
            "the self-hosting corpus, authored sources only: {names:?}"
        );
    }
    // The SAME collected interpreter the census uses, or the two would measure
    // different builds — the pins header forbids them disagreeing.
    // ═══ THE CENSUS COMPILES ITS SOURCES IN PARALLEL, EACH THREAD ON ITS OWN
    // COLLECTED INTERPRETER ═══
    //
    // Serially this walk took ~32 minutes: twenty sources on one core, the
    // four big ones (`encode`, `ir`, `vakyavibhaga`, `yantrotsarjana`) three to
    // five minutes each, on a machine with sixteen. The interpreter is
    // `Rc`/`RefCell` throughout and cannot cross a thread, so every thread
    // builds its OWN `load_chain_collected()` — which is the same pre-state
    // for all of them: every source lexed, parsed and gathered into the
    // declaration store, then `सञ्चयसिद्धिः`, BEFORE any source is compiled.
    // Sources are then dealt round-robin so the big ones spread, and the rows
    // are sorted back into corpus order, so everything printed below is
    // byte-for-byte what the serial walk printed.
    //
    // ॥ THE FALSIFIER, and it was run before this landed ॥ `T1_CENSUS_SERIAL=1`
    // keeps the old single-interpreter walk. The parallel walk's by-cause table
    // and every source's `twin AGREE` octet count must be IDENTICAL to the
    // serial one on the same tree — a difference would mean a source's compile
    // depends on state an EARLIER source left in the shared interpreter, which
    // the between-program resets (`रचितारम्भः` and its kin) exist to forbid.
    // That equality is a property of the compiler, and this loop is now a
    // standing test of it.
    //
    // Thread count is capped at eight: each interpreter holds the corpus's
    // arenas, and memory is the bind on this machine, not cores.
    //
    // ॥ AND THE FALSIFIER FIRED, 2026-09-13, ON 8130f208 ॥ Parallel read 1213
    // stubs against the serial 1245, and the 32 missing were `shrinkhala.t1`'s
    // exactly: compiled on a fresh interpreter it stops in IR — `instruction 309
    // of block 74 names octets 1..1 and पाठाक्षरकोश holds 0` — and serially it
    // "passed" ONLY on literals twelve earlier sources had left in the shared
    // store. `vishlesana.t1`'s octets moved 318786 → 329622 the other way. So
    // the walk is now a FRESH INTERPRETER PER SOURCE in both arms: every figure
    // below is what compiling that source alone produces, which is what a user
    // running the compiler on one file gets. Serial and parallel must still
    // agree byte-for-byte — that equality is now a property of the THREADING
    // only. `shrinkhala.t1`'s stop is a product defect and is named in the table.
    let serial = names.len() <= 1 || std::env::var("T1_CENSUS_SERIAL").is_ok();
    let mut rows: Vec<Row> = if serial {
        names
            .iter()
            .map(|n| {
                let mut it = load_chain_collected();
                chain(&mut it, n, true)
            })
            .collect()
    } else {
        // `T1_CENSUS_THREADS=n` overrides the cap: at eight, every compile ran
        // ~2.5× slower than serial (per-source sum 2536 s against 1037 s; the
        // slowest source 621 s against 191 s) — the allocator, not the cores.
        let threads = std::env::var("T1_CENSUS_THREADS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(std::num::NonZeroUsize::get)
                    .unwrap_or(4)
                    .min(8)
            })
            .min(names.len());
        eprintln!("== census: {} sources over {threads} threads", names.len());
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let mine: Vec<(usize, String)> = names
                        .iter()
                        .cloned()
                        .enumerate()
                        .filter(|(i, _)| i % threads == t)
                        .collect();
                    s.spawn(move || {
                        mine.into_iter()
                            .map(|(i, n)| {
                                let mut it = load_chain_collected();
                                (i, chain(&mut it, &n, true))
                            })
                            .collect::<Vec<(usize, Row)>>()
                    })
                })
                .collect();
            let mut all: Vec<(usize, Row)> = handles
                .into_iter()
                .flat_map(|h| h.join().expect("a census thread panicked"))
                .collect();
            all.sort_by_key(|(i, _)| *i);
            all.into_iter().map(|(_, r)| r).collect()
        })
    };
    link_images(&mut rows);
    if let Some(line) = armed_report(&rows) {
        println!("{line}");
    }
    // ══ `W-288` — THE TABLE PRINTS HERE, BEFORE EVERY GUARD IN THIS TEST ══
    //
    // It used to print at the END, after five assertions. **A DIAGNOSTIC'S
    // SURVIVAL OF A RED WAS DECIDED BY STATEMENT ORDER AND NOTHING DECLARED
    // IT** — `measure_corpus_encode` flushes before its guards and this one
    // printed after, so the same failure yielded a usable by-cause figure from
    // one and nothing from the other. Measured twice in one evening at ~2,000 s
    // each; "not measured because the run went red" was a statement about
    // ordering, not about the run.
    //
    // AND THE FIRST ATTEMPT AT THIS FIX PUT THE LINE IN THE WRONG FUNCTION —
    // into `measure_corpus_encode`, which already printed early, while DELETING
    // it from here. The next census produced ZERO `paradigm_ir_stub_*` lines and
    // the by-cause table was lost for a whole 2,074 s run. **Two functions in
    // one file print the same table; an edit aimed at one landed in the other,
    // and nothing about the change looked wrong.** Check which function a moved
    // statement is inside, not merely that it moved.
    print!("{}", stub_report(&rows));
    for r in &rows {
        println!("{}", run_line(r));
    }

    // ── `W-255` (2026-09-05): THE TWO EMITTERS ARE COMPARED HERE, IN A TEST A
    // GATE RUNS. This flag was `false` and the comparison lived only in
    // `measure_corpus_encode`, which is `#[ignore]`d, so the one check that the
    // T1 emitter and `riscv64.rs` write the same octets was reached by NOTHING:
    // a crate run skips ignored tests and `tools/light-gate.sh` does not scope
    // to this crate. The chain above already costs 585s; the T1 emitter's own
    // share is 261s (measured), so the comparison was 261 seconds away from
    // being guarded while everything it depends on was already being paid for.
    //
    // The census keeps its copy. Two tests asserting one thing is not
    // duplication here — one is gated and one is the census's own consistency.
    let divergent: Vec<String> = rows
        .iter()
        .filter_map(|r| match &r.twin {
            Some(Err(d)) => Some(format!("{}: {d}", r.name)),
            _ => None,
        })
        .collect();
    assert!(
        divergent.is_empty(),
        "the two emitters diverge on {} source(s):\n  {}",
        divergent.len(),
        divergent.join("\n  ")
    );
    // AND THE NON-VACUITY HALF, which is the lesson of the row this came from:
    // a comparison that ran on nothing passes. Every source that built IR must
    // have been emitted by BOTH halves and compared.
    let compared = rows.iter().filter(|r| r.twin.is_some()).count();
    // **`built` MEANT `routines > 0` AND THAT DEFINITION IS NOW WRONG.** A
    // data-only module — globals and no routines — produces an object and is
    // twin-compared, so it counts as compared while counting as unbuilt, and
    // this guard fired with "the twin was run on 3 of the 1 sources that built
    // IR". The guard was right to fire; its definition of the population was
    // what had gone stale.
    //
    // BUILT NOW MEANS PRODUCED AN OBJECT, which is what the comparison is over.
    // `routines > 0` was a proxy that held only while every object had code in
    // it, and this row is what ends that.
    //
    // **AND IT IS NOW WRONG THE OTHER WAY: A MODULE BOTH EMITTERS REFUSE IS
    // COMPARED AND HAS NO OBJECT.** Measured 2026-09-29 on the twenty-source
    // narrowed census (`T1_CORPUS` = every `.t1` but `lib`): `ir.t1` builds
    // 8,836 IR instructions, BOTH emitters refuse it, and they agree on the
    // refusal — `twin AGREE 0 octets`, which is the agreement-on-a-REFUSAL-KIND
    // this guard exists to witness and the strongest comparison it can see —
    // and `check_branch_ranges` then leaves the row with no object at all. So
    // `compared` read 20, `built` read 19, and the guard fired on a healthy
    // state. Its definition of the population had gone stale a second time, in
    // the opposite direction from the first.
    //
    // BUILT NOW MEANS REACHED THE EMIT STAGE, and that is the condition itself
    // rather than another proxy for it: `chain_source` calls
    // `riscv64::emit_module` and then runs the T1 twin unconditionally, and
    // every earlier stop returns before either emitter. `object.is_some()` held
    // only while every compared module also ASSEMBLED, and a module the
    // emitters refuse never reaches the assembler.
    //
    // `!ir_partial` is SUBSUMED, not dropped: a partial IR sets
    // `encode_stop = Stage::Ir` and returns, which is below `Emit`.
    let built = rows.iter().filter(|r| built_ir(r)).count();
    // AND THE THIRD STATE IS NAMED RATHER THAN COUNTED AWAY. A reader of
    // `compared == built` cannot see the difference between "every compared
    // module assembled" and "one of them was refused by both emitters"; that
    // difference is the whole of this row, so the rows in the second state are
    // printed with the refusal that put them there.
    let refused_by_both: Vec<String> = rows
        .iter()
        .filter(|r| built_ir(r) && r.object.is_none())
        .map(|r| format!("{} ({}): {}", r.name, r.stop_word(), r.encode_why))
        .collect();
    if !refused_by_both.is_empty() {
        eprintln!(
            "NOTE {} of {built} source(s) that built IR were compared and produced NO \
             object — both emitters refused them:\n  {}",
            refused_by_both.len(),
            refused_by_both.join("\n  ")
        );
    }
    assert_eq!(
        compared, built,
        "the T1 twin was run on {compared} of the {built} sources that built IR; \
         a comparison that ran on nothing would pass"
    );
    assert!(
        built > 0,
        "no source built IR, so the emitter comparison above compared nothing"
    );
    // REFUSED (`W-243`): a call to a routine NO source declares still fails at the
    // link, by name, with every module of the corpus in the image.
    {
        let (main, missing) = (SymbolId(1), SymbolId(2));
        let f = leaf(
            main,
            vec![(ValueId(0), Instruction::Call(missing, vec![]))],
            Terminator::Return(Some(ValueId(0))),
        );
        let mut module = hand_module(vec![(main, "परीक्षा", "मुख्य", f)], main);
        module
            .names
            .insert(missing, ("अन्यमण्डल".into(), "अनुपस्थितम्".into()));
        let text = riscv64::emit_module(&module).expect("emits");
        let corpus_objects: Vec<vastu::Object> = rows
            .iter()
            .filter_map(|r| r.object.as_deref().and_then(vastu::read))
            .collect();
        let mut row = Row {
            name: "hand".into(),
            module: "परीक्षा".into(),
            ..Row::default()
        };
        let (stage, why) = machine_stages(
            &text,
            &riscv64::emit_startup_object(Some("परीक्षामुख्य")),
            // `W-286`: a hand-written fixture whose image may pull corpus
            // objects, so it is handed the records variant too and the link step
            // picks — the same rule as the corpus pass.
            &riscv64::emit_startup_object_with_records(Some("परीक्षामुख्य"), true),
            &corpus_objects,
            &mut row,
        );
        assert_eq!(stage, Stage::Link, "{why}");
        assert_eq!(row.unresolved, vec!["अन्यमण्डलअनुपस्थितम्".to_string()]);
        println!(
            "REFUSED link with {} corpus objects: {why}",
            corpus_objects.len()
        );
    }
    let assembled = rows.iter().filter(|r| r.assembled()).count();
    let on_yantra = rows.iter().filter(|r| r.ran()).count();
    let chain_on_yantra = rows.iter().filter(|r| r.ran() && r.typechecked()).count();
    let unassembled: Vec<(String, String)> = rows
        .iter()
        .filter(|r| !r.assembled())
        .map(|r| (r.name.clone(), r.encode_stop.name().to_string()))
        .collect();
    let not_on_yantra: Vec<(String, String)> = rows
        .iter()
        .filter(|r| r.assembled() && !r.ran())
        .map(|r| (r.name.clone(), r.stop_word()))
        .collect();
    let pinned = |list: &[(&str, &str)]| -> Vec<(String, String)> {
        list.iter()
            .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
            .collect()
    };
    println!("METRIC paradigm_encode_t1_assembled {assembled}");
    println!("METRIC paradigm_boundary_t1_runnable_on_yantra {on_yantra}");
    println!("METRIC paradigm_encode_t1_chain_on_yantra {chain_on_yantra}");
    // `W-306` — THE STEP MARGIN BEHIND THE RUNNABLE COUNT. Printed in a NARROWED
    // run too (it is above the early return), because the whole reason this
    // exists is to answer "did this source get under the limit, or did it get
    // under it by a hair" in seconds rather than in a corpus walk.
    let mut tight: Vec<(&str, u64)> = Vec::new();
    for r in rows.iter() {
        let margin = r.step_margin();
        if margin == StepMargin::NotReached {
            continue;
        }
        println!(
            "METRIC t1_run_steps {} {} {} of {BUDGET} ({}%)",
            r.name,
            match margin {
                StepMargin::Spare => "spare",
                StepMargin::Tight => "tight",
                StepMargin::Exhausted => "exhausted",
                StepMargin::NotReached => unreachable!(),
            },
            r.steps,
            r.steps * 100 / BUDGET
        );
        if margin == StepMargin::Tight {
            tight.push((r.name.as_str(), r.steps));
        }
    }
    // NOT A RED, AND DELIBERATELY. A tight source is not a fault in the source;
    // it is a fact about `DEFAULT_STEPS`, whose own margin says it is "likely too
    // small by about an order of magnitude" and asks to be measured. What this
    // line buys is that a cycle which empties `T1_ASSEMBLED_NOT_ON_YANTRA`
    // cannot claim the source "runs" without this sentence appearing beside it.
    if !tight.is_empty() {
        println!(
            "!! {} source(s) reach the finisher on HALF THE BUDGET OR MORE: {tight:?} — a pin              re-take that counts these as running is pinned to DEFAULT_STEPS, not to the              compiler. Name the margin in the commit.",
            tight.len()
        );
    }
    println!(
        "METRIC paradigm_encode_zero_constants {} # of {} IR instructions; {} are the builder's stubs",
        rows.iter().map(|r| r.zero_constants).sum::<usize>(),
        rows.iter().map(|r| r.insts).sum::<usize>(),
        rows.iter().map(|r| r.stub_constants).sum::<usize>()
    );
    println!(
        "METRIC paradigm_encode_status_nonzero {}",
        rows.iter()
            .filter(|r| matches!(r.status, Some(s) if s != 0))
            .count()
    );
    // `W-283` — **EVERY SHAPE THIS CORPUS LOWERS MUST BE A PRESENT KEY WITH A
    // POSITIVE VALUE, AND "THE BOUND ROSE" IS NOT EVIDENCE OF IT.**
    //
    // `read_module` above does
    // `usize::try_from(arena_int(&lowered_counts, code)).unwrap_or(0)` and then
    // `if n > 0`, so an absent key and a measured zero are one output here.
    //
    // **MEASURED, AND IT CORRECTS THE OBVIOUS STORY.** The first version of this
    // margin said a shape above `ir.t1`'s `रचितशेषसीमा` reads back as a SILENT
    // absent row. It does not. `ir.t1:625-629` resets `रचितगणनाकोश` by walking
    // `१..=रचितशेषसीमा`, and **that loop is what sizes the arena** — so a shape
    // above the bound faults on the WRITE, long before any read:
    //
    //     bound २८ + `रचितगणनम् २९`  ->  "outside an arena of 29", ×15,
    //     paradigm_ir_stubs 113 (not 3038), assembled 2 (not 15)
    //
    // That is LOUD: it takes the source's IR build down with it. The bound case
    // is the one you cannot miss.
    //
    // **THE SILENT CASE IS THE OTHER ONE: a row declared in `LOWERED_SHAPES`
    // that nothing in the corpus ever raises.** No fault, no diagnostic, every
    // other metric plausible, and the row simply never appears — which is what
    // an arm that was written but is unreachable looks like from out here. That
    // is what this assertion is really for.
    //
    // Until it existed the shape table was REPORTED and never CHECKED.
    //
    // Named, never numbered: `STUB_CAUSES` and `LOWERED_SHAPES` share small
    // integers — 20 of 39 keys collide, and 29 is a row in both tables.
    //
    // The control that proves this can fail is `a_shape_above_the_bound_is_an_
    // absent_row`: the bound left at २८ with the lowering in place, the row
    // required to vanish and this assertion required to fire.
    // A PRESENCE ASSERTION OVER THE CORPUS CANNOT BE MET BY A SUBSET, and the
    // failure says nothing about the tree: `ashtaka.t1` alone simply contains no
    // field load. Under `T1_CORPUS` these become warnings, NAMED ONE BY ONE so a
    // narrowed run still tells you which coverage you gave up — the whole point
    // of this assertion is that an absent row and a measured zero are one
    // output, and narrowing adds a THIRD cause for the same output. A count of
    // how many stood down would hide which.
    let narrowed = std::env::var("T1_CORPUS").is_ok();
    // `W-306` — WHICH BRANCH OF THE INDEX-ASSIGNMENT ARM THIS CORPUS TAKES.
    // `assign_index` reads a present, well-formed zero in the loop below; that is
    // the QUIET cause and it does NOT say the arm is unreachable, because only
    // one of the arm's two branches raises a shape at all. This reads the OTHER
    // branch off its own witness — the growth routine in the object — so the pair
    // is a measurement and not an inference from a name to a kind. See
    // [`GrowthBranch`]. METRIC, not a pin: the ratio is a corpus property and
    // will move with the sources. The ONE red is a broken marker.
    //
    // PRINTED BEFORE THAT LOOP, AND THE ORDER IS THE POINT: the loop PANICS on
    // the first complaining shape, and `assign_index` is a complaining shape
    // today. A reading placed after it would be invisible on exactly the run
    // whose red it explains.
    let growth_per_module: Vec<usize> = rows
        .iter()
        .filter(|r| r.insts > 0)
        .map(|r| r.growth_routines)
        .collect();
    let index_writes: usize = rows
        .iter()
        .filter_map(|r| r.lowered_shape_reads.get("assign_index"))
        .map(ShapeRead::count)
        .sum();
    let branch = GrowthBranch::of(&growth_per_module, index_writes);
    eprintln!(
        "METRIC index_assign_branch {branch:?} over {} lowered module(s)",
        growth_per_module.len()
    );
    if let Some(complaint) = branch.complaint() {
        panic!("{complaint}");
    }

    for shape in REQUIRED_LOWERED_SHAPES {
        if narrowed {
            let n: usize = rows
                .iter()
                .filter_map(|r| r.lowered_by_shape.get(shape))
                .sum();
            if n == 0 {
                eprintln!("!! presence of `{shape}` UNCHECKED — narrowed corpus");
            }
            continue;
        }
        // Decided across rows exactly as `stub_report` sums them — a shape lives
        // in each module's own map, so asking one row would answer about one
        // source and read as absence for every other.
        //
        // **THIS USED TO BE `n > 0` OVER `lowered_by_shape` AND ITS MESSAGE
        // ASKED THE READER TO DISCRIMINATE.** It said "an ABSENT key and a
        // MEASURED ZERO are one output here", named the loud signature to grep
        // for, and left the rest to inference. `W-306` did that inference six
        // times and was wrong six times. The read is now NAMED at the arena
        // (see [`ShapeRead`]) and the corpus-wide decision is
        // [`ShapeCoverage`], so the failure states which of the five it is
        // instead of describing how to tell.
        let reads: Vec<ShapeRead> = rows
            .iter()
            .filter_map(|r| r.lowered_shape_reads.get(*shape).cloned())
            .collect();
        let coverage = ShapeCoverage::of(&reads);
        if let Some(complaint) = coverage.complaint(shape) {
            panic!("{complaint}");
        }
    }

    // `W-306` — THE ROWS THAT ARE DECLARED AND UNREACHED. Same five-state read,
    // a DIFFERENT claim, and a red on three of the four answers. Placed AFTER
    // the required loop on purpose: `covered_by` names a shape the loop above
    // has already required, so if the cover itself is broken the reader gets
    // the required-shape complaint about the cover rather than a derived one
    // about its dependent.
    let shape_coverage = |shape: &str| {
        ShapeCoverage::of(
            &rows
                .iter()
                .filter_map(|r| r.lowered_shape_reads.get(shape).cloned())
                .collect::<Vec<ShapeRead>>(),
        )
    };
    for row in DECLARED_AND_UNREACHED_SHAPES {
        let coverage = shape_coverage(row.shape);
        let cover = shape_coverage(row.covered_by);
        eprintln!(
            "METRIC declared_unreached {} {coverage:?} covered_by {} {cover:?}",
            row.shape, row.covered_by
        );
        if narrowed {
            eprintln!(
                "!! `{}` DECLARED-AND-UNREACHED unchecked — narrowed corpus cannot witness \
                 a cover",
                row.shape
            );
            continue;
        }
        if let Some(complaint) = declared_unreached_complaint(row, &coverage, &cover) {
            panic!("{complaint}");
        }
    }

    // THE LAST CORPUS-WIDE PIN, AND THE ONE MOST WORTH NOT LOSING QUIETLY: it
    // holds BOTH the assembled count and the exact list of stopped sources. A
    // subset can satisfy neither, so it stands down under `T1_CORPUS` — but it
    // prints what it measured, because a narrowed run is still the fastest way
    // to see that a source you touched stopped assembling.
    if narrowed {
        eprintln!(
            "!! T1_ASSEMBLED pin ({} assembled, stopped {:?}) SKIPPED — narrowed run \
             measured {assembled} assembled, stopped {unassembled:?}",
            pins::T1_ASSEMBLED,
            pinned(pins::T1_UNASSEMBLED)
        );
        eprintln!(
            "!! Nothing above is a census. Re-run WITHOUT T1_CORPUS before quoting any figure."
        );
        return;
    }
    assert_eq!(
        (assembled, &unassembled),
        (pins::T1_ASSEMBLED, &pinned(pins::T1_UNASSEMBLED)),
        "the assembled count or the stopped sources moved: measured {assembled} assembled, \
         stopped {unassembled:?}; the pin says {} and {:?}. Re-measure, and say in the commit \
         which source moved and why.",
        pins::T1_ASSEMBLED,
        pins::T1_UNASSEMBLED
    );
    assert_eq!(
        (on_yantra, &not_on_yantra),
        (
            pins::T1_ON_YANTRA,
            &pinned(pins::T1_ASSEMBLED_NOT_ON_YANTRA)
        ),
        "the runnable count or the assembled-but-not-running sources moved: measured \
         {on_yantra} on yantra, not running {not_on_yantra:?}; the pin says {} and {:?}",
        pins::T1_ON_YANTRA,
        pins::T1_ASSEMBLED_NOT_ON_YANTRA
    );
    assert_eq!(
        chain_on_yantra,
        pins::T1_CHAIN_ON_YANTRA,
        "the number of sources the whole chain carries to the machine moved"
    );
}

// --- W-245's acceptance: a CORPUS routine computes on the machine --------------------------

/// The routine, verbatim from `vakyavibhaga.t1`: `कारकमूलसीमा(सीमा, कारक)` — where a
/// kāraka-bearing operand's STEM ends, given the word's octet length and the sigil's
/// code (१ म्, २ न, ३ त्, ४ य्, ५ ए; ० none): `न` and `ए` are three octets, `म् त् य्`
/// six. Its Rust twin is `lex.rs`'s `split_sigil`, reached through `sadhana::lex::lex`,
/// whose `Kind::Operand { base, .. }` is the stem.
const CORPUS_ROUTINE: &str = "कारकमूलसीमा";

fn corpus_routine_text() -> String {
    let src = source("vakyavibhaga.t1");
    let head = format!("सार्वजनिक वृत्तिः {CORPUS_ROUTINE} आदाय ");
    let start = src
        .find(&head)
        .unwrap_or_else(|| panic!("`{CORPUS_ROUTINE}` is a routine of vakyavibhaga.t1"));
    let rest = &src[start..];
    let end = rest.find("\nइति\n").expect("the routine closes") + "\nइति\n".len();
    rest[..end].to_string()
}

/// **THE ROW'S ACCEPTANCE.** A corpus routine, copied verbatim into a module with a
/// zero-parameter driver, driven through the REAL chain — lex → parse → resolve →
/// typecheck → IR (`ir.t1`) → emit (both twins, agreeing) → assemble → link → load →
/// run — halts with the finisher status its Rust twin computes for the same input:
/// `कारकमूलसीमा(6, 2)` is the stem of `कन` (`क`, 3 octets) and `कारकमूलसीमा(15, 1)` the
/// stem of `क्षम्` (`क्ष`, 9 octets). Until `W-245` every parameter read was the
/// constant-० stub and every finisher status was 0.
#[test]
fn a_corpus_routine_computes_its_rust_twins_answer_on_yantra() {
    if !walk_allowed("a_corpus_routine_computes_its_rust_twins_answer_on_yantra") {
        return;
    }
    let routine = corpus_routine_text();
    assert!(routine.starts_with(&format!(
        "सार्वजनिक वृत्तिः {CORPUS_ROUTINE} आदाय सीमा ॱॱ अ६४ ऽ कारक ॱॱ अ६४ ददाति अ६४ आदि"
    )));
    // The Rust twin's answers: the stem's length in octets, by the T0 lexer.
    let twin = |word: &str| -> u64 {
        let toks = sadhana::lex::lex(&format!("योगः {word} ।")).expect("lexes");
        let base = toks
            .iter()
            .find_map(|t| match &t.kind {
                sadhana::lex::Kind::Operand { base, .. } => Some(base.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("`{word}` is an operand"));
        u64::try_from(base.len()).expect("fits")
    };
    let cases: [(&str, &str, usize, i64); 2] = [
        ("कन", "६ ऽ २", "कन".len(), 2),
        ("क्षम्", "१५ ऽ १", "क्षम्".len(), 1),
    ];
    for (word, args, len, karaka) in cases {
        assert_eq!(
            args,
            format!(
                "{} ऽ {}",
                riscv64::devanagari(len as i64),
                riscv64::devanagari(karaka)
            ),
            "the driver passes the word's octet length and the sigil's code"
        );
        let src = format!(
            "मण्डलम् परीक्षा ॥\n\n{routine}\nसार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि\n    प्रत्यागमनम् {CORPUS_ROUTINE} आरभ्य {args} समाप्तम् ।\nइति\n"
        );
        // `W-253`: the chain takes an interpreter; a fixture gets its own, since
        // it is not part of the corpus pass that shares one.
        let mut it = load_chain();
        let mut row = chain_source(&mut it, "परीक्षा.t1", &src, true);
        // `W-243`: a source is assembled by `chain_source` and its image linked
        // afterwards (`link_images`, over the whole corpus). This fixture is one
        // self-contained module — the routine and its zero-parameter driver —
        // so its image is the startup object and its own, and nothing else.
        if let Some(own) = row.object.clone() {
            let startup = row.startup.clone();
            let startup_records = row.startup_records.clone();
            let (stop, why) = link_and_run(&startup, &startup_records, &own, &[], &mut row);
            row.encode_stop = stop;
            row.encode_why = why;
        }
        println!("{}", run_line(&row));
        assert_eq!(
            row.typecheck,
            Some(Ok(())),
            "the fixture passes the checker: {:?}",
            row.typecheck
        );
        assert!(
            matches!(row.twin, Some(Ok(n)) if n > 0),
            "the twins agree on the fixture: {:?}",
            row.twin
        );
        assert!(
            row.ran(),
            "the fixture runs to the finisher: {}",
            row.encode_why
        );
        let expected = twin(word);
        assert_eq!(
            row.status,
            Some(expected),
            "`{CORPUS_ROUTINE}({args})` on yantra answers the stem length of `{word}` that lex.rs answers; the text that ran:\n{}",
            row.text
        );
        assert_ne!(expected, 0, "the answer is a number, not the stub's ०");
        println!(
            "METRIC paradigm_encode_corpus_routine_status {expected} # {CORPUS_ROUTINE}({args}) == lex.rs's stem of `{word}`"
        );
    }
}

// --- the refused cases (§4) --------------------------------------------------------------

/// **`W-368` — THIS FILE'S DECODER IS FED A LOGICAL SHIFT, AND THE IMAGE IS
/// RUN.** `binary_kind` above reads `W-333`'s mark off kind १० and no fixture
/// had ever handed it a १: the one non-ignored caller of `chain_source`
/// compiles a corpus routine with no shift in it.
///
/// `chain_source` is the right path because it does three things at once. It
/// DECODES the chain's IR through this file's `read_module`; it emits the
/// decoded module with `riscv64.rs` and compares that text with the T1
/// emitter's (`row.twin`), so a decoder that dropped the mark would write
/// `सचिह्नदक्षिणसरणम्` where the T1 twin writes `दक्षिणसरणम्` and the twins
/// would part; and it links and RUNS the image, so the VALUE is checked and
/// not only the decode.
///
/// Two fixtures, because one proves nothing: a `न६४` name holding `ऋण१`
/// shifted by ६३ answers १ under the logical shift and all-ones under the
/// arithmetic one; an `अ६४` name holding `ऋण८` shifted by १ and raised by ९
/// answers ५ under the arithmetic shift and something near 2^63 under the
/// logical one. A decoder, or an emitter, that made every right shift
/// logical passes the first and fails the second.
#[test]
fn a_shift_of_an_unsigned_name_is_decoded_as_logical_here_and_answers_one_on_yantra() {
    for (what, decl, answer, expected, verb) in [
        (
            "a name declared `न६४`",
            "    चरः क ॱॱ न६४ भवति ऋण१ ।\n",
            "क दक्षिणसृ ६३",
            1u64,
            "दक्षिणसरणम् ",
        ),
        (
            "a name declared `अ६४`",
            "    चरः स ॱॱ अ६४ भवति ऋण८ ।\n",
            "आरभ्य स दक्षिणसृ १ समाप्तम् योगः ९",
            5u64,
            "सचिह्नदक्षिणसरणम् ",
        ),
    ] {
        let src = format!(
            "मण्डलम् परीक्षा ॥\n\nसार्वजनिक वृत्तिः मुख्यम् ददाति अ६४ आदि\n{decl}    प्रत्यागमनम् {answer} ।\nइति\n"
        );
        let mut it = load_chain();
        let mut row = chain_source(&mut it, "परीक्षा.t1", &src, true);
        if let Some(own) = row.object.clone() {
            let startup = row.startup.clone();
            let startup_records = row.startup_records.clone();
            let (stop, why) = link_and_run(&startup, &startup_records, &own, &[], &mut row);
            row.encode_stop = stop;
            row.encode_why = why;
        }
        println!("{}", run_line(&row));
        assert_eq!(
            row.typecheck,
            Some(Ok(())),
            "{what}: the fixture passes the checker: {:?}",
            row.typecheck
        );
        assert!(
            matches!(row.twin, Some(Ok(n)) if n > 0),
            "{what}: the Rust emitter over THIS FILE'S DECODED module and the T1 \
             emitter write the same text — a decoder that drops kind १०'s \
             `ध्रुवमूल्यम्` parts them here: {:?}",
            row.twin
        );
        // The verb at a LINE START: `दक्षिणसरणम्` is a suffix of
        // `सचिह्नदक्षिणसरणम्`, so `contains` would be true of both.
        assert_eq!(
            row.text.lines().filter(|l| l.starts_with(verb)).count(),
            1,
            "{what}: exactly one line beginning `{verb}` in the text that ran:\n{}",
            row.text
        );
        assert!(
            row.ran(),
            "{what}: the fixture runs to the finisher: {}",
            row.encode_why
        );
        assert_eq!(
            row.status,
            Some(expected),
            "{what}: `{answer}` on yantra; the text that ran:\n{}",
            row.text
        );
    }
}

fn leaf(name: SymbolId, insts: Vec<(ValueId, Instruction)>, term: Terminator) -> Function {
    let mut blocks = HashMap::new();
    blocks.insert(
        BlockId(0),
        Block {
            id: BlockId(0),
            insts,
            terminator: Some(term),
        },
    );
    Function {
        name,
        blocks,
        entry_block: BlockId(0),
    }
}

fn hand_module(functions: Vec<(SymbolId, &str, &str, Function)>, entry: SymbolId) -> Module {
    let mut names = Names::new();
    for (sym, module, name, _) in &functions {
        names.insert(*sym, ((*module).to_string(), (*name).to_string()));
    }
    Module {
        globals: Vec::new(),
        name: "परीक्षा".into(),
        functions: functions.into_iter().map(|(_, _, _, f)| f).collect(),
        names,
        entry: Some(entry),
    }
}

/// An IR with an `Unreachable` block is refused BY BLOCK, by name — not emitted as a
/// program that falls off its end.
#[test]
fn an_ir_with_an_unreachable_block_is_refused_by_block() {
    let main = SymbolId(1);
    let f = leaf(
        main,
        vec![(ValueId(0), Instruction::ConstInt(1))],
        Terminator::Unreachable,
    );
    let refused = riscv64::emit_module(&hand_module(vec![(main, "परीक्षा", "मुख्य", f)], main))
        .expect_err("refused");
    assert_eq!(
        refused,
        Refusal::Unreachable {
            function: "परीक्षामुख्य".into(),
            block: BlockId(0)
        }
    );
    println!("REFUSED {refused}");
}

/// Two `(module, name)` pairs that concatenate to one word are refused BY PAIR.
#[test]
fn a_label_collision_is_refused_by_pair() {
    let (a, b) = (SymbolId(1), SymbolId(2));
    let fa = leaf(
        a,
        vec![(ValueId(0), Instruction::ConstInt(0))],
        Terminator::Return(Some(ValueId(0))),
    );
    let fb = leaf(
        b,
        vec![(ValueId(0), Instruction::ConstInt(0))],
        Terminator::Return(Some(ValueId(0))),
    );
    let refused = riscv64::emit_module(&hand_module(
        vec![(a, "परीक्षाक", "ख", fa), (b, "परीक्षा", "कख", fb)],
        a,
    ))
    .expect_err("refused");
    assert!(
        matches!(&refused, Refusal::LabelCollision { label, .. } if label == "परीक्षाकख"),
        "{refused}"
    );
    println!("REFUSED {refused}");
}

/// A callee token that names no routine assembles (an undefined symbol is a link's
/// business) and the standalone link lists it BY NAME — the same stop the census
/// reports for a cross-module call.
#[test]
fn a_callee_that_names_no_routine_is_an_unresolved_label_the_link_names() {
    let (main, missing) = (SymbolId(1), SymbolId(2));
    let f = leaf(
        main,
        vec![(ValueId(0), Instruction::Call(missing, vec![]))],
        Terminator::Return(Some(ValueId(0))),
    );
    let mut module = hand_module(vec![(main, "परीक्षा", "मुख्य", f)], main);
    module
        .names
        .insert(missing, ("अन्यमण्डल".into(), "अनुपस्थितम्".into()));
    let text = riscv64::emit_module(&module).expect("emits");
    let mut row = Row {
        name: "hand".into(),
        module: "परीक्षा".into(),
        ..Row::default()
    };
    let (stage, why) = machine_stages(
        &text,
        &riscv64::emit_startup_object(Some("परीक्षामुख्य")),
        // `W-286`: this image is the startup and one hand-written module, so it
        // can need no region — the empty text says so rather than leaving the
        // reader to infer it from the `&[]` two lines down.
        "",
        &[],
        &mut row,
    );
    assert_eq!(stage, Stage::Link, "{why}");
    assert_eq!(row.undefined, vec!["अन्यमण्डलअनुपस्थितम्".to_string()]);
    assert_eq!(row.unresolved, vec!["अन्यमण्डलअनुपस्थितम्".to_string()]);
    assert!(why.contains("अन्यमण्डलअनुपस्थितम्"), "{why}");
    println!("REFUSED link: {why}");
}

/// One Latin letter injected into an emitted text: `सङ्केतन` refuses WITH THE LINE.
#[test]
fn one_latin_letter_in_the_emitted_text_is_refused_by_the_assembler_with_its_line() {
    let good = riscv64::emit_module(&riscv64::fixture_recursive_sum()).expect("emits");
    let mut row = Row {
        name: "fixture".into(),
        module: "परीक्षा".into(),
        ..Row::default()
    };
    let startup = riscv64::emit_startup_object(Some("परीक्षामुख्य"));
    let (stage, _) = machine_stages(&good, &startup, "", &[], &mut row);
    assert_eq!(stage, Stage::Run, "the fixture runs: {:?}", row.halt);
    assert_eq!(row.status, Some(15));
    assert_eq!(row.words, row.words_back, "every word decodes back");

    let lines: Vec<&str> = good.lines().collect();
    let victim = lines
        .iter()
        .position(|l| l.starts_with("योगः "))
        .expect("an add line");
    let mut bad: Vec<String> = lines.iter().map(|l| (*l).to_string()).collect();
    bad[victim] = bad[victim].replacen("योगः", "योगःx", 1);
    let bad = bad.join("\n") + "\n";
    let (stage, why) = machine_stages(&bad, &startup, "", &[], &mut Row::default());
    assert_eq!(stage, Stage::Assemble);
    assert!(
        why.starts_with(&format!("line {}: ", victim + 1)),
        "the refusal names line {}: {why}",
        victim + 1
    );
    println!("REFUSED {why}");
}

/// The `.सस` are counted and not run: 16, 0 readable by design (`W-238`).
#[test]
#[ignore = "needs tests/corpus/t1 (the development corpus) not in the public repository"]
fn the_sas_programs_are_counted_and_none_is_readable_by_design() {
    assert_eq!(sas_programs(), pins::SAS_PROGRAMS);
    println!("METRIC paradigm_encode_sas_programs {}", sas_programs());
    println!("METRIC paradigm_encode_sas_readable 0");
}

/// ONE SOURCE THROUGH BOTH TWINS, named by `TWIN_SOURCE`. A scaffold for
/// finding WHICH instruction makes the emitters disagree: the corpus census
/// takes forty minutes and reports the divergence as one line per source,
/// which says a divergence exists and not what caused it.
#[test]
#[ignore = "diagnostic"]
fn twin_one_source() {
    let n = std::env::var("TWIN_SOURCE").expect("TWIN_SOURCE names a corpus file");
    let mut it = load_chain_collected();
    // TWIN_PATH takes a file OUTSIDE the corpus, so a divergence can be cut
    // down to a few lines instead of being read out of a 130-line source.
    let row = match std::env::var("TWIN_PATH") {
        Ok(p) => {
            let src = std::fs::read_to_string(&p).expect("TWIN_PATH is readable");
            chain_source(&mut it, &n, &src, true)
        }
        Err(_) => chain(&mut it, &n, true),
    };
    match &row.twin {
        Some(Ok(k)) => println!("TWIN AGREE {n}: {k} octets"),
        Some(Err(d)) => println!("TWIN DIVERGE {n}: {d}"),
        None => println!("TWIN ABSENT {n} — the chain did not reach the emitter"),
    }
}

/// **THE FIVE STATES, FIRED ON AN OBJECT — AND THE FOUR THINGS THAT MUST STILL
/// BE REFUSED.**
///
/// The corpus has only ever produced two of [`ShapeCoverage`]'s five answers
/// (`Raised` for every shape but one, `Quiet` for `assign_index`), so a control
/// that ran only over the corpus would leave three arms unexercised — which is
/// the same defect this cycle is fixing, one altitude up. [`read_shape`] and
/// [`ShapeCoverage::of`] were extracted to take an arena and a LIST, so both are
/// reachable from here without a census.
///
/// **THE REFUSALS, each of which is a way this could be wrong and green:**
///
/// 1. **A single witness covers the corpus.** A shape raised in ONE module and a
///    present zero in every other is `Raised`, never `Quiet`. Getting this wrong
///    reds `ir.t1` for a module that simply contains no indexed write — and 4 of
///    the 21 sources contain none.
/// 2. **The loud cause outranks the quiet one.** With no raise anywhere, a short
///    arena must report `OutsideArena` and NOT `Quiet`. Getting this wrong sends
///    the reader to hunt an unreachable arm when the bound is what moved.
/// 3. **A raise outranks a short arena too.** Coverage is the question; the
///    faulting module's collapse is loud on its own and reported elsewhere.
/// 4. **A non-count is not a zero.** `arena_int` answered `0` for it, which is
///    exactly how this whole family of misreadings became possible — so the test
///    asserts BOTH that `read_shape` separates it AND that `arena_int` does not,
///    because a claim that the new reading is better is empty without the old
///    one's answer beside it.
#[test]
fn the_lowered_shape_coverage_reading_names_five_states_and_keeps_the_loud_cause_first() {
    // `0` is never a shape code — `LOWERED_SHAPES` starts at 1 and `ir.t1`
    // clears `१..=रचितशेषसीमा` — so slot 0 stands for the unused head the way
    // every other arena in this file treats it.
    let arena = |slots: Vec<Value>| Rc::new(RefCell::new(slots));
    let head = Value::Int(0);

    // ── read_shape: the four answers at ONE slot ────────────────────────────
    let a = arena(vec![head.clone(), Value::Int(7)]);
    assert_eq!(read_shape(&a, 1), ShapeRead::Raised(7));
    // PAST THE END. Two slots exist, so code 2 has none.
    assert_eq!(read_shape(&a, 2), ShapeRead::OutsideArena { slots: 2 });
    assert_eq!(read_shape(&a, 99), ShapeRead::OutsideArena { slots: 2 });

    let z = arena(vec![head.clone(), Value::Int(0)]);
    assert_eq!(read_shape(&z, 1), ShapeRead::Zero { raw: 0 });

    // REFUSAL 4 — a slot that is not a count. The new read separates it; the
    // old one answered zero. Both halves asserted, because the second is the
    // evidence that the first was worth building.
    let m = arena(vec![head.clone(), Value::Bool(true)]);
    assert_eq!(read_shape(&m, 1), ShapeRead::NotAnInteger);
    assert_eq!(
        arena_int(&m, 1),
        0,
        "`arena_int` must still answer 0 here — that is the collapse this reading \
         replaces, and if it ever stops doing so this control's premise is stale"
    );
    // And a NEGATIVE count, which `usize::try_from` used to turn into a zero
    // with the sign thrown away. It is a zero for coverage and the raw value is
    // kept, so a negative can be SEEN rather than inferred.
    let n = arena(vec![head.clone(), Value::Int(-4)]);
    assert_eq!(read_shape(&n, 1), ShapeRead::Zero { raw: -4 });
    assert_eq!(read_shape(&n, 1).count(), 0);

    // REFUSAL 4's other half: a diagnostic state cannot inflate a census.
    for r in [
        ShapeRead::OutsideArena { slots: 2 },
        ShapeRead::NotAnInteger,
        ShapeRead::Zero { raw: 0 },
    ] {
        assert_eq!(r.count(), 0, "{r:?} must contribute nothing to a total");
    }
    assert_eq!(ShapeRead::Raised(7).count(), 7);

    // ── ShapeCoverage: the five answers over a LIST ─────────────────────────
    assert_eq!(ShapeCoverage::of(&[]), ShapeCoverage::NoModules);
    assert_eq!(
        ShapeCoverage::of(&[ShapeRead::Raised(3), ShapeRead::Raised(4)]),
        ShapeCoverage::Raised {
            total: 7,
            modules: 2
        }
    );
    assert_eq!(
        ShapeCoverage::of(&[
            ShapeRead::Zero { raw: 0 },
            ShapeRead::Zero { raw: 0 },
            ShapeRead::Zero { raw: 0 },
        ]),
        ShapeCoverage::Quiet { modules: 3 }
    );
    assert_eq!(
        ShapeCoverage::of(&[ShapeRead::NotAnInteger, ShapeRead::Zero { raw: 0 }]),
        ShapeCoverage::Malformed { modules: 1 }
    );
    // The SHORTEST arena is the one named, since that is the bound to clear.
    assert_eq!(
        ShapeCoverage::of(&[
            ShapeRead::OutsideArena { slots: 29 },
            ShapeRead::OutsideArena { slots: 22 },
        ]),
        ShapeCoverage::OutsideArena {
            modules: 2,
            slots: 22
        }
    );

    // REFUSAL 1 — ONE witness covers the corpus, however many modules read a
    // present zero. This is the arm that keeps `ir.t1` out of the dock for a
    // source that contains none of the shape.
    assert_eq!(
        ShapeCoverage::of(&[
            ShapeRead::Zero { raw: 0 },
            ShapeRead::Raised(1),
            ShapeRead::Zero { raw: 0 },
        ]),
        ShapeCoverage::Raised {
            total: 1,
            modules: 1
        }
    );

    // REFUSAL 2 — with NO raise anywhere, the short arena outranks the zero.
    let loud = ShapeCoverage::of(&[
        ShapeRead::Zero { raw: 0 },
        ShapeRead::OutsideArena { slots: 28 },
    ]);
    assert_eq!(
        loud,
        ShapeCoverage::OutsideArena {
            modules: 1,
            slots: 28
        },
        "a module short of the slot must not be reported as an unreached arm"
    );
    let said = loud.complaint("x").expect("the loud cause is a defect");
    assert!(
        said.contains("OUTSIDE THE ARENA") && said.contains("रचितशेषसीमा"),
        "the loud complaint must name the bound: {said}"
    );
    assert!(
        !said.contains("unreached arm"),
        "the loud complaint must not offer the quiet reading: {said}"
    );

    // REFUSAL 3 — a raise outranks a short arena as well.
    assert_eq!(
        ShapeCoverage::of(&[ShapeRead::OutsideArena { slots: 28 }, ShapeRead::Raised(5),]),
        ShapeCoverage::Raised {
            total: 5,
            modules: 1
        }
    );

    // ── complaint(): a defect exactly on the three defect arms ──────────────
    assert!(ShapeCoverage::NoModules.complaint("x").is_none());
    assert!(
        ShapeCoverage::Raised {
            total: 1,
            modules: 1
        }
        .complaint("x")
        .is_none()
    );
    let quiet = ShapeCoverage::Quiet { modules: 19 }
        .complaint("assign_index")
        .expect("the quiet cause is a defect");
    assert!(
        quiet.contains("PRESENT, WELL-FORMED ZERO") && quiet.contains("assign_index"),
        "the quiet complaint must name the shape and the state: {quiet}"
    );
    assert!(
        quiet.contains("bound is NOT the problem"),
        "the quiet complaint must RULE OUT the bound rather than describe how to \
         check for it — that inference is the one `W-306` got wrong: {quiet}"
    );
    assert!(
        ShapeCoverage::Malformed { modules: 1 }
            .complaint("x")
            .is_some_and(|c| c.contains("NOT A COUNT"))
    );
}

/// `W-306` — THE DECLARED-AND-UNREACHED READING, AND THE THREE IT MUST REFUSE.
///
/// The corpus can only ever produce the one answer this row was written for, so
/// this is the only place the other three are reachable. Each refusal is a way
/// the move of `assign_index` out of `REQUIRED_LOWERED_SHAPES` could turn into
/// the deletion it is not allowed to be:
///
/// 1. **A short arena must not be bought out by the exemption.** `OutsideArena`
///    faults the IR build; "it's unreached anyway" is the inference that would
///    hide it, and the complaint must still be the LOUD one, word for word.
/// 2. **A raise must be a red.** The row claims "unreached". If the shape ever
///    fires, the exemption is stale and the table is now lying about the
///    corpus — which is the exact defect the move was made to remove.
/// 3. **Quiet with a quiet cover must be a red.** This is the state where the
///    corpus lowers indexed writes on NEITHER branch. It is the whole reason
///    the row carries a `covered_by` instead of just a sentence.
#[test]
fn the_declared_and_unreached_reading_refuses_a_stale_exemption_and_a_silent_pair() {
    let row = &DECLARED_AND_UNREACHED_SHAPES[0];
    assert_eq!(
        row.shape, "assign_index",
        "this test reads row 0 by name so a reordering cannot silently retarget it"
    );
    assert!(
        REQUIRED_LOWERED_SHAPES.contains(&row.covered_by),
        "`{}`'s cover `{}` must itself be a REQUIRED shape, or this row's safety rests on a \
         check nobody runs",
        row.shape,
        row.covered_by
    );
    assert!(
        !REQUIRED_LOWERED_SHAPES.contains(&row.shape),
        "`{}` must not be in BOTH tables — the two claims contradict and the required loop \
         runs first, so the exemption would be unreachable",
        row.shape
    );
    assert!(
        LOWERED_SHAPES.iter().any(|(_, name)| *name == row.shape),
        "a declared-and-unreached row must still be DECLARED in `LOWERED_SHAPES`; otherwise \
         this table is the deletion it exists to prevent"
    );

    let raised = ShapeCoverage::Raised {
        total: 4,
        modules: 2,
    };
    let quiet = ShapeCoverage::Quiet { modules: 9 };

    // THE DECLARED STATE — quiet, and the cover raised. The only answer today.
    assert_eq!(
        declared_unreached_complaint(row, &quiet, &raised),
        None,
        "silence next to a named raise is the measurement this row records"
    );

    // REFUSAL 1 — the loud cause outranks the exemption, and keeps its words.
    let loud = declared_unreached_complaint(
        row,
        &ShapeCoverage::OutsideArena {
            modules: 1,
            slots: 28,
        },
        &raised,
    )
    .expect("a short arena is a defect even for an unreached row");
    assert!(
        loud.contains("OUTSIDE THE ARENA") && loud.contains("रचितशेषसीमा"),
        "the loud complaint must survive the move verbatim: {loud}"
    );
    assert!(
        declared_unreached_complaint(row, &ShapeCoverage::Malformed { modules: 1 }, &raised)
            .is_some_and(|c| c.contains("NOT A COUNT")),
        "a slot holding a non-count is a broken write, not an unreached arm"
    );

    // REFUSAL 2 — a raise means the exemption went stale.
    let stale = declared_unreached_complaint(row, &raised, &raised)
        .expect("a raise falsifies the row's own claim");
    assert!(
        stale.contains("STALE") && stale.contains("REQUIRED_LOWERED_SHAPES"),
        "the stale complaint must name the one-line repair: {stale}"
    );
    assert!(
        stale.contains("GrowthOnly"),
        "every complaint quotes the witness, so nobody has to go find it: {stale}"
    );

    // REFUSAL 3 — both branches silent. The red this table exists for.
    let both = declared_unreached_complaint(row, &quiet, &quiet)
        .expect("a quiet shape with a quiet cover is unwitnessed silence");
    assert!(
        both.contains("NEITHER branch") && both.contains("assign_index_grown"),
        "the silent-pair complaint must name the cover and the reading: {both}"
    );
    assert!(
        !both.contains("STALE"),
        "the silent pair must not be reported as a stale exemption: {both}"
    );
    // ...and a cover that is short or malformed is silence just the same.
    assert!(
        declared_unreached_complaint(
            row,
            &quiet,
            &ShapeCoverage::OutsideArena {
                modules: 1,
                slots: 28
            }
        )
        .is_some(),
        "a cover that never got written is not a witness"
    );

    // NOT DEFECTS — an empty census blames nobody, on either side.
    assert_eq!(
        declared_unreached_complaint(row, &ShapeCoverage::NoModules, &quiet),
        None
    );
    assert_eq!(
        declared_unreached_complaint(row, &quiet, &ShapeCoverage::NoModules),
        None,
        "a census that read no module for the cover is nothing to ask, not a red"
    );
}

/// `W-306` — THE INDEX-ASSIGNMENT BRANCH READING, ALL FIVE STATES AND THE THREE
/// IT MUST REFUSE.
///
/// The corpus reading it feeds is a `METRIC` line, so this is the only place the
/// decision itself is falsified. What must be refused, and why each one is the
/// mistake this reading exists to prevent:
///
/// 1. **A broken marker must not answer a product question.** Two zero tokens in
///    one module and the answer is [`GrowthBranch::AmbiguousMarker`], NOT
///    `GrowthOnly` — even though `most > 1` also means "at least one growth
///    routine". The old driver would have called that a growth routine twice and
///    inserted `SymbolId(10_000_005)` twice.
/// 2. **`Both` must not be reported as `GrowthOnly`.** `Both` is the state in
///    which adding `रचितगणनम्` to the growth branch double-counts, so a reading
///    that lets one raised `assign_index` hide behind a growth routine would
///    green-light exactly the edit the `Next:` line forbids.
/// 3. **`Unreached` must not be reported as `GrowthOnly`.** Zero modules with a
///    growth routine and zero raises is "no index assignment anywhere", which is
///    a different cause from "all of them grew" and points at a different file.
#[test]
fn the_step_margin_reading_names_four_states_and_refuses_a_tight_run() {
    // The row a margin is read off: everything else defaulted, because
    // `step_margin` must depend on the stop, the halt and the count and on
    // nothing else.
    let row = |encode_stop: Stage, halt: Option<Halt>, steps: u64| Row {
        encode_stop,
        halt,
        steps,
        ..Default::default()
    };
    let finisher = || {
        Some(Halt::Finisher {
            value: 0x5555,
            status: Some(0),
        })
    };

    // 1. SPARE — halted with room to multiply its work by `TIGHT_HEADROOM`.
    assert_eq!(
        row(Stage::Run, finisher(), 80).step_margin(),
        StepMargin::Spare,
        "`sanskrit_text.t1`'s measured 80 steps is the clearest Spare in the corpus"
    );

    // 2. TIGHT — halted, but cannot absorb that multiplication. THIS IS THE CASE
    //    THAT MUST STILL BE REFUSED, and it is the whole reason this reading
    //    replaced a boolean: `ran()` answers `true` here, identically to the
    //    line above, so a pin re-take that consults only `ran()` counts a source
    //    one change from the limit as settled.
    let tight = row(Stage::Run, finisher(), 487_371);
    assert!(
        tight.ran(),
        "the old instrument cannot tell this from Spare"
    );
    assert_eq!(
        tight.step_margin(),
        StepMargin::Tight,
        "`shrinkhala.t1`'s measured 487,371 steps is 48% of the budget: it halts, \
         and a fourfold growth would not. An instrument that calls this Spare is \
         the two-state instrument this reading replaced."
    );

    // 3. The boundary, from both sides, so the comparison cannot silently invert.
    assert_eq!(
        row(Stage::Run, finisher(), BUDGET / TIGHT_HEADROOM).step_margin(),
        StepMargin::Tight,
        "exactly one quarter of the budget has no headroom left, so it is Tight"
    );
    assert_eq!(
        row(Stage::Run, finisher(), BUDGET / TIGHT_HEADROOM - 1).step_margin(),
        StepMargin::Spare
    );

    // 4. EXHAUSTED — the budget ran out. Distinct from Tight: this one did not
    //    halt at all, and it is what a Tight source becomes when it grows.
    assert_eq!(
        row(Stage::Run, Some(Halt::StepLimit { pc: LOAD }), BUDGET).step_margin(),
        StepMargin::Exhausted,
        "the state `sanskrit_text.t1` was pinned in before this cycle measured it"
    );

    // 5. NOT REACHED — the budget never applied, and this must NOT be folded
    //    into Exhausted. A source that stops at `link` has no step count to
    //    report, and reporting it as though it ran out of steps would send the
    //    next reader to `DEFAULT_STEPS` for a fault that is in the linker.
    assert_eq!(
        row(Stage::Link, None, 0).step_margin(),
        StepMargin::NotReached
    );
    assert_eq!(
        row(Stage::Assemble, None, 0).step_margin(),
        StepMargin::NotReached,
        "not assembled at all: `assembled()` is false and the machine never saw it"
    );
    // A halt that is neither the finisher nor the step limit is a fault of its
    // own kind — `bad-access`, `unimplemented` — and `stop_word` already names
    // it. It is NotReached here because the step budget is not what stopped it.
    assert_eq!(
        row(
            Stage::Run,
            Some(Halt::BadAccess {
                pc: LOAD,
                addr: 0xdead_beef
            }),
            12
        )
        .step_margin(),
        StepMargin::NotReached,
        "a bad access is not a verdict about the step budget"
    );
}

#[test]
fn the_index_assignment_branch_reading_names_five_states_and_refuses_a_broken_marker() {
    // ── the two single-branch readings ──────────────────────────────────────
    assert_eq!(
        GrowthBranch::of(&[1, 0, 1, 0, 0], 0),
        GrowthBranch::GrowthOnly { modules: 2 },
        "growth routines and a quiet `assign_index` is the growth branch, corpus-wide"
    );
    assert_eq!(
        GrowthBranch::of(&[0, 0, 0], 19),
        GrowthBranch::PlaceOnly { sites: 19 },
        "raises with no growth routine anywhere is the `anyatha` branch"
    );

    // ── REFUSAL 1: a broken marker is decided FIRST, ahead of every product
    // reading, even though it also carries a growth routine. ───────────────
    assert_eq!(
        GrowthBranch::of(&[1, 2, 0], 0),
        GrowthBranch::AmbiguousMarker {
            modules: 1,
            most: 2
        },
        "two zero tokens in one module is the instrument's breakage, not `GrowthOnly`"
    );
    // And it outranks `Both` too — a raise beside a broken marker changes nothing
    // about the marker.
    assert_eq!(
        GrowthBranch::of(&[3, 1], 5),
        GrowthBranch::AmbiguousMarker {
            modules: 1,
            most: 3
        }
    );
    assert!(
        GrowthBranch::of(&[1, 2, 0], 0).complaint().is_some(),
        "the broken marker is the ONE red; every other state is a METRIC"
    );

    // ── REFUSAL 2: `Both` is its own state and never collapses into either
    // single-branch reading. ───────────────────────────────────────────────
    assert_eq!(
        GrowthBranch::of(&[1, 0, 1], 4),
        GrowthBranch::Both {
            modules: 2,
            sites: 4
        },
        "one raise beside a growth routine must NOT read as `GrowthOnly` — that is \
         the state in which a raise on the growth branch double-counts"
    );

    // ── REFUSAL 3: no witness at all is `Unreached`, not `GrowthOnly`. ──────
    assert_eq!(GrowthBranch::of(&[0, 0, 0], 0), GrowthBranch::Unreached);
    assert_eq!(
        GrowthBranch::of(&[], 0),
        GrowthBranch::Unreached,
        "no modules measured is nothing to explain, not a claim about a branch"
    );

    // ── every non-broken state is report-only, per the owner ruling of
    // 2026-09-13: the ratio is a corpus property and moves with the sources. ─
    for b in [
        GrowthBranch::of(&[1, 0], 0),
        GrowthBranch::of(&[0, 0], 7),
        GrowthBranch::of(&[1, 0], 7),
        GrowthBranch::of(&[0], 0),
    ] {
        assert!(b.complaint().is_none(), "{b:?} must not red a landing");
    }
}

// --- the population the emitter comparison is over (`built_ir`) --------------------------
//
// THREE CHEAP TESTS, NO LOADER AND NO INTERPRETER. [`built_ir`] is a reading over
// one field of a [`Row`], and a machine in these would only be a second thing
// that could be wrong about it. They are NOT `#[ignore]`d: the guard they pin
// fired on the twenty-source census (2026-09-29) against a tree nobody had
// touched, and the two definitions it has already worn out were each discovered
// by a census run costing minutes. This costs microseconds.

#[test]
fn a_module_both_emitters_refused_is_still_one_the_comparison_ran_on() {
    // `ir.t1`'s MEASURED shape, 2026-09-29, twenty-source narrowed census: the
    // IR built (8,836 instructions), both emitters refused it the same way, and
    // `check_branch_ranges` left no object. This is the row that retired
    // `object.is_some()` as the population rule.
    let row = Row {
        name: "ir.t1".to_string(),
        module: "मध्यरूप".to_string(),
        encode_stop: Stage::Emit,
        encode_why: "मध्यरूपकार्यक्रमरचना: the conditional in BlockId(1401) is 4436 \
                     bytes from BlockId(1572), past ±4 KiB"
            .to_string(),
        twin: Some(Ok(0)),
        object: None,
        ..Row::default()
    };
    assert!(
        built_ir(&row),
        "both emitters ran on it and agreed on the refusal; that IS the comparison"
    );
    // The RETIRED rule, spelled out here rather than only in a commit message,
    // because the difference between the two is the whole of this row: it
    // excluded exactly this shape and made the guard read `20 of the 19`.
    let by_object = |r: &Row| r.object.is_some() && !r.ir_partial;
    assert!(
        !by_object(&row),
        "the object rule excluded exactly this row, which is why it was retired"
    );
}

#[test]
fn every_stage_at_or_past_emit_was_compared_and_every_earlier_one_was_not() {
    for stage in [
        Stage::Lex,
        Stage::Parse,
        Stage::Resolve,
        Stage::Typecheck,
        Stage::Ir,
    ] {
        let row = Row {
            encode_stop: stage,
            ..Row::default()
        };
        assert!(
            !built_ir(&row),
            "`{}` returns before `riscv64::emit_module`, so NEITHER emitter ran",
            stage.name()
        );
    }
    for stage in [
        Stage::Emit,
        Stage::Assemble,
        Stage::Link,
        Stage::Load,
        Stage::Run,
    ] {
        let row = Row {
            encode_stop: stage,
            ..Row::default()
        };
        assert!(
            built_ir(&row),
            "`{}` is at or past emit, so BOTH emitters ran",
            stage.name()
        );
    }
    // `Stage::default()` is `Lex`, so a row nothing touched is not in the
    // population — which is what keeps `built > 0` a real assertion.
    assert!(!built_ir(&Row::default()));
}

#[test]
fn the_population_is_not_read_off_the_twin_field() {
    // **THE CASE THAT MUST STILL BE REFUSED.** `compared == built` has to remain
    // able to FAIL. Define the population as `twin.is_some()` — the shortest fix
    // for the `20 of the 19` red, and the one that makes it never come back —
    // and the assertion becomes `n == n`: the vacuity it was written to catch
    // becomes the one thing it can no longer see. A row that reached the run
    // stage with NO twin recorded is exactly that fault, and it must count as
    // built so the guard reds on it.
    let row = Row {
        name: "untwinned.t1".to_string(),
        encode_stop: Stage::Run,
        twin: None,
        object: Some(vec![0u8; 4]),
        ..Row::default()
    };
    assert!(
        built_ir(&row),
        "it reached emit, so it belongs to the population whether or not a twin \
         verdict was recorded — that gap is the fault, not an exemption from it"
    );
    let compared = usize::from(row.twin.is_some());
    let built = usize::from(built_ir(&row));
    assert_ne!(
        compared, built,
        "and the guard must therefore RED on it: 0 compared of 1 built"
    );
}
