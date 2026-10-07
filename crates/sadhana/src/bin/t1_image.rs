//! **`t1_image` — THE `.t1` COMPILER COMPILES THE GIVEN SOURCES INTO ONE IMAGE.**
//!
//! `t1_build` is the Rust twin (`Front`, `riscv64::emit_module`). This binary is
//! the product path: it loads the `.t1` compiler into the interpreter and calls
//! `शृङ्खलाॱमण्डलानिप्रतिबिम्बम्` — the routine that already compiles every
//! source it is handed, links every object behind the startup, and writes an
//! ELF — over the sources named on the command line. Handed the compiler's own
//! twenty sources it produces the compiler's own image; that image is the
//! self-hosting subject, and nothing in the tree had ever asked the routine to
//! build it. **The caller supplies the set** (the routine's own margin: a
//! compiler does not read directories), so the list is explicit here.
//!
//! What it does NOT do: name an entry. With none named the startup stub halts
//! success, which is the image's RUN rung; naming `मण्डलसङ्कलनम्` as the entry
//! and handing it a source in memory is the next capability, one per unit.
//!
//!     t1_image [--spec-root <dir>] [--compiler <dir>] [--load <x.t1>]… [--entry <module> <routine>] [--no-predict] -o <out.elf> <a.t1> <b.t1> …
//!
//! **THE EXIT CODE IS THE VERDICT (`W-343`).** A build script continues on a
//! zero, so a zero is only returned when the build did what it was asked. Two
//! outcomes used to print their trouble and exit zero anyway, and are now
//! nonzero — each stated where it is decided, below:
//!
//!   * a source that FAILED TO COMPILE — every diagnostic line is still printed
//!     and the image of the sources that did compile is still written (the
//!     instruments that read `steps:`, `diag:` and `refused:` are unchanged, and
//!     so is every script that tests for the file), but the exit is nonzero;
//!   * an `आयातः` of a module that NO SOURCE DECLARES — refused before the
//!     build, by name, instead of being planted as a stub at each call.
//!
//! A THIRD IS A NOTICE AND NOT A REFUSAL, because the build it describes WORKS:
//! an `--entry` whose module is among the positionals and not in the
//! interpreter. The image's entry is installed and runs; only the interpreted
//! prediction is missing. That used to read `predict: … refused: … no routine
//! named … is loaded`, which names neither the cause nor the remedy; it now
//! reads `predict:  none — entry module … is named positionally …`, the build
//! goes on and the exit is zero.
//!
//! A REFUSED PREDICT IS STILL NOT FATAL. A routine the interpreter HAS and that
//! refuses while running leaves the most informative stream there is; that is
//! printed and the build goes on, as before.
//!
//! `--entry` names the image's entry routine through the driver's own
//! `प्रवेशन्यासः`; the startup then calls it and its result is the exit status.
//! With `--entry शृङ्खला स्वपरीक्षा` the binary also runs `स्वपरीक्षा` under the
//! interpreter after the build and prints the number the image must answer.
//!
//! `--compiler` is the directory holding the compiler's own sources (default:
//! the directory of the first source); the interpreter loads THOSE as the
//! program and compiles the listed files as its input — for the self-image the
//! two lists are the same twenty files.
//!
//! `--load <file.t1>` (repeatable) puts one more source into the INTERPRETER's
//! program without adding it to the build. For a source outside `--compiler`'s
//! directory the two lists are disjoint, so `--entry` could name a routine the
//! interpreter had never loaded and the prediction refused — which is what a
//! rung outside the corpus met for four cycles. With it, a pair in `spec/rung/`
//! is compiled from the positionals and predicted from the same files:
//!
//!     t1_image --compiler crates/sadhana-t1/src --load spec/rung/सेतुः.t1 \
//!              --load spec/rung/पारम्.t1 --entry सेतुः मुख्य --predict-only \
//!              -o /dev/null spec/rung/सेतुः.t1 spec/rung/पारम्.t1
//!
//! `--no-predict` builds an image with an entry and WITHOUT the interpreted
//! half, deliberately: the predict is not attempted, whichever lists the entry
//! is in, and the log reads `predict:  skipped (--no-predict)` where the figure
//! would be. It is how a caller that wants the native half alone — a program
//! that reads its command line, or one too dear to interpret — says so, rather
//! than leaving the `predict:  none` notice above to say it for them.
//!
//! ## THE DIFFERENTIAL GATE (`W-381`, owner ruling 2026-10-06)
//!
//! **A BUILD WITH AN ENTRY IS REFUSED WHEN ITS TWO ENGINES DISAGREE.** The
//! predict is the entry interpreted; after the build the image is run natively
//! by `yantra-run`, as a CHILD (found beside this binary, or named with
//! `--yantra-run <path>`; refused when missing or when its `--source-stamp`, a
//! content hash of both crates' sources, is not this binary's or is `unknown`)
//! under its OWN step budget ([`NATIVE_STEPS`], `--native-steps`), on the same `--input`. The two are compared by
//! `sadhana::t1::agreement::agree` — the status cut to the finisher's 48 bits
//! and the printed octets exactly, and a refusal only against the SAME native
//! refusal — the one rule the engine-agreement ratchet also reads.
//!
//! FAIL CLOSED. The image is built to `<out>.tmp` and renamed to `<out>` only on
//! agreement; a stale `<out>` is removed before the build starts. On a
//! disagreement, an inconclusive run (fuel, step budget, no verdict) or a
//! missing comparison, the image is left at `<out>.refused` and the exit is
//! [`DIVERGED`] (97), an exit no other tool in the tree uses (`t1_image`'s own
//! failures are 1; `yantra-run` uses 1, 64, 66, 75 and 81..88; the stack canary
//! 59 and the refusal statuses 85/89/90 are QEMU exits).
//!
//! `--accept-divergence <reason>` (non-empty) writes the image anyway and
//! RECORDS that it did. `--no-predict` and a `predict:  none` build have no
//! comparison at all, so they REQUIRE it (recorded UNCHECKED). Every build writes
//! a sidecar `<out>.provenance`: the verdict, the reason, both versions, the
//! image's and the sources' sha256 and both figures.
//!
//! A build with NO `--entry` runs nothing but the startup, so nothing is
//! compared; its provenance says `NO-ENTRY`.
use sadhana::t1::agreement::{self, Run};
use sadhana::t1::chain;
use sadhana::t1::mandala;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

/// Fuel for the whole build: the census spends ~80e9 per source; twenty sources
/// and a link need room, and running out is a report, not a crash.
const FUEL: u64 = 4_000_000_000_000;

/// THE NATIVE HALF'S STEP BUDGET (`W-381`), its OWN and not the caller's
/// `YANTRA_STEPS`: `yantra-run`'s default of a million steps refused every real
/// build (the Naad walker needs ~8.4M, vikodaka's predict ~734M interpreted), so
/// the gate hands the child this budget explicitly — the fixpoint's 4·10¹¹ class.
/// `--native-steps <n>` replaces it for one build.
const NATIVE_STEPS: u64 = 400_000_000_000;

/// THE GATE'S REFUSAL EXIT (`W-381`): the engines disagreed, the comparison was
/// inconclusive, or a build with an entry had no comparison and no
/// `--accept-divergence`. Chosen from an unused range: not 1 (this binary's
/// other failures), not 64/66/75/81..88 (`yantra-run`), not 59/85/89/90 (the
/// QEMU exits of the canary and the three refusal statuses), and not 77 (the
/// landing gate's own refusal). Pinned by
/// `crates/sadhana/tests/w381_differential_gate.rs`.
const DIVERGED: u8 = 97;

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(Rc::new(RefCell::new(vs)))
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: t1_image [--spec-root <dir>] [--compiler <dir>] [--load <file.t1>]...\n\
         \x20              [--entry <module> <routine>] [--predict-sink <path>] [--predict-only]\n\
         \x20              [--no-predict] [--accept-divergence <reason>] [--yantra-run <path>]\n\
         \x20              [--native-steps <n>] [--version] [--source-stamp]\n\
         \x20              -o <out.elf> <file.t1>...\n\
         \n\
         \x20  THE DIFFERENTIAL GATE (W-381): with --entry, the image is run by\n\
         \x20  yantra-run (beside this binary, or --yantra-run) and compared with the\n\
         \x20  interpreted predict. On a disagreement the image is left at\n\
         \x20  <out>.refused and the exit is 97. --accept-divergence <reason> writes\n\
         \x20  it anyway and records the reason in <out>.provenance; --no-predict and\n\
         \x20  an entry the interpreter cannot call REQUIRE it (recorded UNCHECKED).\n\
         \x20  The native half runs under its own budget, --native-steps (default\n\
         \x20  400000000000), whatever YANTRA_STEPS says; a run that does not finish\n\
         \x20  in it is refused as INCONCLUSIVE, not as a disagreement. The runner\n\
         \x20  must carry this binary's --source-stamp (a content hash of\n\
         \x20  crates/sadhana/src and crates/yantra/src); an `unknown` stamp on\n\
         \x20  either side never matches.\n\
         \n\
         \x20  --version prints the commit this build STARTED FROM. It is HEAD,\n\
         \x20  not the tree: a binary built from a dirty worktree prints a commit\n\
         \x20  it is not, and an uncommitted edit does not even rerun the stamp.\n\
         \x20  A figure taken from a dirty tree must say so in words."
    );
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut spec_root: Option<PathBuf> = None;
    let mut compiler: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut entry: Option<(String, String)> = None;
    let mut sources: Vec<PathBuf> = Vec::new();
    // `--load <file>`: ONE MORE SOURCE INTO THE INTERPRETER, NOT INTO THE BUILD.
    // The two lists below were the same distinction `t1_boot` already draws in
    // its own margin — `T1_BOOT_EXTRA` adds a module the INTERPRETER runs,
    // `T1_BOOT_COLLECT` one the compiled program can NAME — and `t1_image` had
    // only the second: `compiler_paths` (a directory walk) is what the
    // interpreter LOADS, `sources` (the positionals) is what gets COMPILED, and
    // nothing could be in both unless it sat inside `--compiler`'s directory.
    // So `--entry` on a source outside that directory printed `refused: no
    // routine named … is loaded` and a rung had no interpreted half to pair its
    // native status against (W-279, measured four times on `spec/rung/`).
    let mut load: Vec<PathBuf> = Vec::new();
    // THE INTERPRETED HALF OF THE OCTET CHANNEL. `अष्टकॱमुद्रणम्` is intercepted
    // into the interpreter's sink, so the SAME marker stream the image writes to
    // the UART exists here — it was simply never handed to anyone. Writing it out
    // turns the prediction into a STATE comparison: two streams from two engines
    // over one source, diffed record by record, where the first differing record
    // names the divergence. Comparing exit statuses cannot do that, and the
    // compiled side cannot report on itself.
    let mut predict_sink: Option<PathBuf> = None;
    // Stop after the prediction. The predict costs a chain load and the build
    // costs twenty minutes, so an instrument that only needs the interpreted
    // stream should not pay for an image it will not read.
    let mut predict_only = false;
    // BUILD WITH AN ENTRY AND NO PREDICTION, ON PURPOSE (`W-343`). Without it an
    // entry the interpreter cannot call gets a `predict:  none` NOTICE and the
    // build goes on; with it the predict is not attempted at all, and the log
    // says `skipped` in the `predict:` line's place.
    let mut no_predict = false;
    // `W-381`: `--accept-divergence <reason>` and `--yantra-run <path>`.
    let mut accept: Option<String> = None;
    let mut yantra_run: Option<PathBuf> = None;
    let mut native_steps: u64 = NATIVE_STEPS;
    // THE INPUT CHANNEL'S INTERPRETED HALF. `yantra-run` places a file in RAM
    // for the image (`YANTRA_INPUT`); these hand the SAME octets to the predict,
    // so the two sinks are about one input and can be compared with `cmp`.
    let mut input: Option<PathBuf> = None;
    let mut input_name: Option<String> = None;
    let mut input_trace: i128 = 0;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            // ॥ THE BUILD COMMIT — W-347 ॥ Answered and exited before anything is
            // read, loaded or built, so `--version` works in a tree where nothing
            // else would. `unknown` is a real answer here, not a failure: the
            // stamp is `unknown` exactly when the build had no git to ask, and a
            // reader needs to be told that rather than shown a blank.
            "--version" => {
                println!("{}", env!("SASSEMBLY_BUILD_COMMIT"));
                return ExitCode::SUCCESS;
            }
            // The CONTENT stamp the gate compares with its runner's (`W-381`).
            "--source-stamp" => {
                println!("{}", env!("SASSEMBLY_SOURCE_STAMP"));
                return ExitCode::SUCCESS;
            }
            "--spec-root" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                spec_root = Some(PathBuf::from(v));
                i += 2;
            }
            "--compiler" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                compiler = Some(PathBuf::from(v));
                i += 2;
            }
            "--load" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                load.push(PathBuf::from(v));
                i += 2;
            }
            "--entry" => {
                let (Some(m), Some(r)) = (args.get(i + 1), args.get(i + 2)) else {
                    return usage();
                };
                entry = Some((m.clone(), r.clone()));
                i += 3;
            }
            "--predict-sink" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                predict_sink = Some(PathBuf::from(v));
                i += 2;
            }
            "--predict-only" => {
                predict_only = true;
                i += 1;
            }
            "--no-predict" => {
                no_predict = true;
                i += 1;
            }
            // `W-381`: the reason a build may stand without its engines agreeing.
            // Non-empty: an accept with no words in it records nothing a reader
            // could check, and is refused below as a usage error.
            "--accept-divergence" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                accept = Some(v.clone());
                i += 2;
            }
            "--yantra-run" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                yantra_run = Some(PathBuf::from(v));
                i += 2;
            }
            "--native-steps" => {
                let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u64>().ok()) else {
                    return usage();
                };
                native_steps = n;
                i += 2;
            }
            "--input" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                input = Some(PathBuf::from(v));
                i += 2;
            }
            "--input-name" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                input_name = Some(v.clone());
                i += 2;
            }
            "--input-trace" => {
                input_trace = 1;
                i += 1;
            }
            // THE LEVEL, for the text dump: 2 prints each module's generated
            // Sassembly text between 0xFF separators, the same as
            // `YANTRA_INPUT_TRACE=2` does natively.
            "--input-trace-level" => {
                let Some(v) = args.get(i + 1).and_then(|v| v.parse::<i128>().ok()) else {
                    return usage();
                };
                input_trace = v;
                i += 2;
            }
            "-o" => {
                let Some(v) = args.get(i + 1) else {
                    return usage();
                };
                out = Some(PathBuf::from(v));
                i += 2;
            }
            a if a.starts_with('-') => return usage(),
            a => {
                sources.push(PathBuf::from(a));
                i += 1;
            }
        }
    }
    let (Some(out), false) = (out, sources.is_empty()) else {
        return usage();
    };
    // `--no-predict` AGAINST A FLAG THAT ONLY THE PREDICT SERVES IS A
    // CONTRADICTION, NOT A PREFERENCE. Each of the three exists to feed or to
    // read the interpreted run, so honouring both would mean silently dropping
    // one of them — and a flag that is accepted and ignored is how a log comes
    // to describe a run that did not happen.
    if no_predict && (predict_only || predict_sink.is_some() || input.is_some()) {
        eprintln!(
            "STOPPED: --no-predict contradicts --predict-only, --predict-sink and --input, \
             which exist only for the prediction it turns off"
        );
        return usage();
    }
    // `W-381`: AN ACCEPT WITH NO WORDS RECORDS NOTHING, and one beside
    // `--predict-only` accepts an image that is never written.
    if let Some(r) = &accept {
        if r.trim().is_empty() {
            eprintln!(
                "STOPPED: --accept-divergence needs a reason — the words recorded in \
                 <out>.provenance for why this build may stand without its engines agreeing"
            );
            return usage();
        }
        if predict_only {
            eprintln!(
                "STOPPED: --accept-divergence contradicts --predict-only, which writes no image"
            );
            return usage();
        }
    }
    // FAIL CLOSED: a file at `-o` from an EARLIER build must not survive a build
    // that is refused, or a script testing for the file reads it as this one.
    // Regular files only — `-o /dev/null` is how a predict-only caller says "none".
    let tmp_out = with_suffix(&out, ".tmp");
    let refused_out = with_suffix(&out, ".refused");
    let provenance_out = with_suffix(&out, ".provenance");
    if !predict_only {
        for p in [&out, &tmp_out, &refused_out, &provenance_out] {
            if p.is_file() {
                let _ = std::fs::remove_file(p);
            }
        }
    }
    let spec_root = spec_root.unwrap_or_else(|| PathBuf::from("spec"));
    let compiler_dir = compiler.unwrap_or_else(|| {
        sources[0]
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    });

    // ── the compiler: every .t1 in its directory, loaded as the program ──
    let mut compiler_paths: Vec<PathBuf> = match std::fs::read_dir(&compiler_dir) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "t1"))
            .collect(),
        Err(e) => {
            eprintln!("STOPPED AT read: {}: {e}", compiler_dir.display());
            return ExitCode::FAILURE;
        }
    };
    compiler_paths.sort();
    // The extras go AFTER the sort, in the order they were named: they are not
    // part of the compiler's own corpus and must not be shuffled into it, and a
    // path already inside `--compiler`'s directory is not loaded twice.
    let from_dir = compiler_paths.len();
    for p in &load {
        if !p.is_file() {
            eprintln!("STOPPED AT load: {}: not a file", p.display());
            return ExitCode::FAILURE;
        }
        if !compiler_paths.contains(p) {
            compiler_paths.push(p.clone());
        }
    }
    let refs: Vec<&Path> = compiler_paths.iter().map(PathBuf::as_path).collect();
    let mut it = match Interpreter::load_paths(&refs, &spec_root) {
        Ok(it) => it,
        Err(e) => {
            eprintln!("STOPPED AT load: {} compiler sources: {e:?}", refs.len());
            return ExitCode::FAILURE;
        }
    };
    println!(
        "compiler: {} sources loaded from {}",
        from_dir,
        compiler_dir.display()
    );
    for p in &compiler_paths[from_dir..] {
        println!("loaded:   {} (interpreter only; not compiled)", p.display());
    }

    // ── the input: the listed sources, each with the module its header declares ──
    let mut texts: Vec<Value> = Vec::new();
    let mut names: Vec<Value> = Vec::new();
    let mut inputs: Vec<(PathBuf, String)> = Vec::new();
    // Each positional's module facts — what it declares and what it imports —
    // read by the same token reader the module censuses use. `read_lossy` and
    // not `read`: it never refuses a source, so this adds no second way for a
    // file to fail before the compiler has had its say.
    let mut units: Vec<mandala::Unit> = Vec::new();
    for p in &sources {
        let src = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("STOPPED AT read: {}: {e}", p.display());
                return ExitCode::FAILURE;
            }
        };
        let name = chain::module_name(&src).unwrap_or_else(|| {
            p.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "कार्यक्रम".to_string())
        });
        println!("input:    {} (module {name})", p.display());
        // KEPT BESIDE THE ARENAS BECAUSE THE ARENAS ARE MOVED. `texts` and
        // `names` go into `मण्डलानिप्रतिबिम्बम्` and are gone by the time the
        // build reports a refusal; the failure path below needs to find the
        // source a failed MODULE name came from, and this pair is the map.
        inputs.push((p.clone(), name.clone()));
        units.push(mandala::read_lossy(&src).0);
        texts.push(octets(src.as_bytes()));
        names.push(octets(name.as_bytes()));
    }
    let n = i128::try_from(sources.len()).unwrap_or(0);

    // ── AN IMPORT OF A MODULE NO SOURCE DECLARES IS REFUSED BY NAME (`W-343`) ──
    //
    // MEASURED BEFORE THIS CHECK: a source holding `आयातः ब ।` and one call
    // `बॱकृ`, built with no `ब` anywhere, printed `stubs: 1 … (by cause: {1: 1})`
    // and `0 source(s) failed`, wrote a 65,728-octet ELF and exited 0. The
    // absent module was not misreported; it was planted as a stub and linked.
    //
    // **THE SIGNAL IS THE IMPORT, NEVER THE STUB COUNT.** Stubs are an ordinary,
    // tracked figure here — the per-cause tally is a ratchet — so `stubs > 0` is
    // true of builds that are fine, and exiting on it would refuse the corpus.
    //
    // "DECLARED" MEANS BY ANY SOURCE THIS RUN HOLDS: a positional, a `--load`,
    // or a file in `--compiler`'s directory. The third is not generosity. A
    // program built from its own sources alone that writes `आयातः अष्टक ।` — the
    // compiler's output channel, never listed as an input — is how the kernels
    // built outside this tree are written, and comparing imports against the
    // positionals alone would refuse them.
    //
    // WHAT THAT LEAVES UNCAUGHT, MEASURED AND NOT HIDDEN: a module the
    // interpreter has and the build does not. The same caller with its callee
    // named by `--load` ONLY passes this check, prints `stubs: 1 … {1: 1}` and
    // exits 0 — the defect above, one list over. Telling that apart from the
    // `अष्टक` case needs to know which of the compiler's modules a program may
    // lean on without compiling, and nothing in this driver knows that.
    let loaded: Vec<String> = it.declarations().into_iter().map(|d| d.name).collect();
    let mut undeclared = 0usize;
    for ((path, module), unit) in inputs.iter().zip(&units) {
        for import in &unit.imports {
            let wanted = &import.name.text;
            let declared = loaded.iter().any(|m| m == wanted)
                || inputs.iter().any(|(_, m)| m == wanted)
                || units.iter().any(|u| u.declares_module(wanted));
            if !declared {
                eprintln!(
                    "STOPPED AT import: {}:{}: module {module} imports {wanted}, and module \
                     {wanted} is not loaded — no input, no --load source and no source in \
                     {} declares it. Add the source that declares {wanted} to the inputs; \
                     built without it, every call into {wanted} becomes a stub and the \
                     image still links.",
                    path.display(),
                    import.name.line,
                    compiler_dir.display()
                );
                undeclared += 1;
            }
        }
    }
    if undeclared > 0 {
        return ExitCode::FAILURE;
    }

    // ── AN ENTRY THE INTERPRETER CANNOT CALL IS SAID BY NAME (`W-343`) ──
    //
    // `--entry` is resolved TWICE and by two different module sets. The product
    // finds it among the POSITIONALS, which it compiles; the predict below finds
    // it in the INTERPRETER, which holds `--compiler`'s directory plus `--load`
    // and never the positionals. Named only positionally, the entry is in the
    // first set and not the second.
    //
    // WHAT THAT BUILD REALLY PRODUCES, measured, because the row described it
    // wrongly: the image is NOT entry-less. `प` built from one positional with
    // `--entry प मुख्य` halts `status: Some(5)` under `yantra-run`, which is
    // `मुख्य`'s own answer. What is missing is the other half — and the only
    // trace of that was `predict: … refused: RunError { … no routine named … is
    // loaded }`, a line that looks like every other refused predict and names
    // neither the cause nor the remedy.
    //
    // **SO THIS IS A NOTICE AND NOT A REFUSAL.** The first version of this
    // change refused the build, on the row's word that the image had no entry.
    // The image works, a caller outside this tree builds exactly this shape
    // under `set -e` and runs it, and refusing a working build to gain a clearer
    // line is the wrong trade. The clearer line is kept and the build goes on.
    //
    // DECIDED HERE, PRINTED BELOW in the `predict:` line's own place. `Some` is
    // the notice; the predict is then not attempted, because its only possible
    // answer is the "no routine named" this replaces. Only an entry the
    // interpreter does not HAVE takes this path, and only when the build does
    // have its module — a routine that RAN and refused is still reported as
    // before, and an entry in neither set still stops at the link.
    let unpredictable: Option<String> = match (&entry, no_predict) {
        (Some((m, r)), false) => {
            let name = format!("{m}ॱ{r}");
            match inputs.iter().find(|(_, module)| module == m) {
                Some((path, _)) if it.routine(&name).is_none() => {
                    let path = path.display();
                    Some(if loaded.iter().any(|l| l == m) {
                        // The rarer case: a module of that NAME is loaded — the
                        // compiler has one, or a `--load` named another file —
                        // and it has no such routine. Whether the positional
                        // has it is not known here, so the line is conditional
                        // about the image where the other is not.
                        format!(
                            "none — the interpreter loaded a module {m} without a \
                             routine {r}, and {path} declares that module for the \
                             build, so {name} has no interpreted prediction. If {path} \
                             holds {r}, the image's entry is installed and the startup \
                             calls it. Name that source with --load as well for a \
                             prediction, or pass --no-predict with --accept-divergence <reason> to \
                             say none is intended (W-381: without it this build is refused)."
                        )
                    } else {
                        format!(
                            "none — entry module {m} is named positionally ({path}) \
                             and not with --load, so the interpreter never loaded it \
                             and {name} has no interpreted prediction. The image's \
                             entry is installed: the build compiles {m} and the \
                             startup calls {name}. Name the source with --load as well \
                             for a prediction, or pass --no-predict with --accept-divergence \
                             <reason> to say none is intended (W-381: without it this build \
                             is refused)."
                        )
                    })
                }
                _ => None,
            }
        }
        _ => None,
    };

    if let Some((m, r)) = &entry {
        match it.call(
            "शृङ्खलाॱप्रवेशन्यासः",
            vec![octets(m.as_bytes()), octets(r.as_bytes())],
            1_000_000,
        ) {
            Ok(_) => println!("entry:    {m} {r} (its result is the exit status)"),
            Err(e) => {
                eprintln!("STOPPED AT entry: {e:?}");
                return ExitCode::FAILURE;
            }
        }
    }
    // ── THE PREDICTION, BEFORE THE BUILD AND FOLLOWING THE ENTRY THAT WAS NAMED ──
    //
    // It used to sit AFTER the image was written and to test `r == "स्वपरीक्षा"`,
    // so it was a guard on a SPELLING rather than on a condition: every other
    // rung of the ladder — all 72 of them — produced a native status with no
    // interpreted side to pair it against, and pairing is the whole of the rung
    // discipline. Any entry now gets its prediction.
    //
    // AND IT RUNS FIRST because it is the CHEAP half. The interpreted call costs
    // a chain load; the build costs twenty minutes. Printed before the build, a
    // reader has the number to compare against within the first minute instead
    // of at the end — and if the interpreted side refuses, that is known before
    // anything is spent on an image that could not have been compared anyway.
    if let Some(path) = &input {
        let Some(mname) = &input_name else {
            eprintln!(
                "STOPPED: --input needs --input-name; the module name is passed, never guessed"
            );
            return ExitCode::FAILURE;
        };
        let text = match std::fs::read(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("STOPPED AT --input {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        };
        // EACH SET MUST LAND. `set_global` refuses a name no module declared;
        // a refused set would leave the program reading its own `०` and the
        // predict would describe an input that never arrived.
        for (g, v) in [
            ("निवेशपाठः", Value::Octets(Octets::new(&text))),
            ("निवेशमण्डलनाम", Value::Octets(Octets::new(mname.as_bytes()))),
            ("निवेशानुरेखणम्", Value::Int(input_trace)),
        ] {
            if !it.set_global(g, v) {
                eprintln!(
                    "STOPPED: no module declares `{g}` — these sources carry no input interface"
                );
                return ExitCode::FAILURE;
            }
        }
        println!(
            "input:    {} octets of {} as module {mname}{}",
            text.len(),
            path.display(),
            if input_trace > 0 {
                ", trace level ON"
            } else {
                ""
            }
        );
    }
    // `W-381`: the interpreted half, kept for the gate. `None` when there is none.
    let mut predicted: Option<Run> = None;
    if let (Some((m, r)), false) = (&entry, no_predict) {
        let name = format!("{m}ॱ{r}");
        if let Some(notice) = &unpredictable {
            // NOT ATTEMPTED: the interpreter has no such routine, so the call
            // could only answer "no routine named … is loaded". The notice,
            // decided above, says why and what to do instead. The sink and the
            // witnesses below still run, as they did after that refusal — an
            // empty sink is what a predict that never started leaves.
            println!("predict:  {notice}");
        } else {
            // `W-381`: the predict is KEPT, not only printed — the gate compares
            // it with the native run after the build. The sink is copied now,
            // before the build's own calls run through the same interpreter.
            let answer = it.call(&name, vec![], FUEL);
            let out = it.sink().to_vec();
            predicted = Some(match answer {
                Ok(Value::Int(n)) => {
                    println!(
                        "predict:  interpreted {name} -> {n}; the image's exit status must equal it"
                    );
                    Run::Ran { status: n, out }
                }
                Ok(v) => {
                    println!("predict:  interpreted {name} answered {v:?}");
                    Run::Answered(format!("{v:?}"))
                }
                Err(e) => {
                    println!("predict:  interpreted {name} refused: {e:?}");
                    let why = e.reason.lines().next().unwrap_or("").to_string();
                    if e.reason.contains("exceeded its fuel") {
                        Run::Inconclusive(format!("the predict ran out of fuel: {why}"))
                    } else if why.starts_with("WAIT") && why.contains("no event source") {
                        Run::Paused { out }
                    } else {
                        Run::Refused { why, out }
                    }
                }
            });
        }
        // AFTER the call and unconditionally on its outcome: a REFUSED predict
        // still leaves everything printed up to the refusal, and that prefix is
        // the most informative stream there is — it ends exactly where the
        // interpreter stopped.
        if let Some(p) = &predict_sink {
            let s = it.sink();
            match std::fs::write(p, s) {
                Ok(()) => println!(
                    "sink:     {} octet(s) of predict output -> {}",
                    s.len(),
                    p.display()
                ),
                Err(e) => {
                    eprintln!("STOPPED AT sink: {p:?}: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }

        // THE ZERO-OPTIONAL WITNESSES. `ir.t1` lowers `सम्भाव्य` as one word with
        // ० meaning absent, so at every site below the image MUST answer the
        // opposite of what this run just answered. Printed unconditionally and
        // AFTER the sink for the same reason: a refused predict still names the
        // sites it reached. Silence here is the result worth having.
        let mut w = sadhana::t1::nirvahana::zero_optional_witnesses();
        if w.is_empty() {
            println!("zero-opt: no Some(०)-against-absent comparison was reached");
        } else {
            w.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let total: u64 = w.iter().map(|(_, n)| n).sum();
            println!(
                "zero-opt: {} site(s), {total} comparison(s) the image must answer the OTHER WAY:",
                w.len()
            );
            for (name, n) in w.iter().take(40) {
                println!("            {n:>9}  {name}");
            }
            if w.len() > 40 {
                println!("            ... and {} more site(s)", w.len() - 40);
            }
        }
    } else if predict_sink.is_some() || predict_only {
        eprintln!("STOPPED: --predict-sink and --predict-only need --entry to predict from");
        return usage();
    } else if let Some((m, r)) = &entry {
        // Reached only under `--no-predict`. Said in the `predict:` line's own
        // place, so a reader looking for the figure finds the reason there is
        // none instead of an absence.
        println!(
            "predict:  skipped (--no-predict) — {m}ॱ{r} was not interpreted, so the image's \
             exit status has no figure here to equal"
        );
    }
    if predict_only {
        println!("predict-only: stopping before the build; no image was written");
        return ExitCode::SUCCESS;
    }

    // ── the build, by the product ──
    let image = match it.call(
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![arena(texts), arena(names), Value::Int(n)],
        FUEL,
    ) {
        Ok(v) => v
            .octets()
            .map(|o| o.as_slice().to_vec())
            .unwrap_or_default(),
        Err(e) => {
            eprintln!("STOPPED AT मण्डलानिप्रतिबिम्बम्: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    // **THE METER IS READ HERE AND NOWHERE LOWER, AND THE REASON IS FOUR LINES
    // OF `SAS-013`.** `Interpreter::call` takes a fuel argument and ASSIGNS it
    // (`t1/nirvahana.rs:1345`, `self.fuel = fuel`), so every `call` restarts the
    // meter. The diagnostic re-compile below is a `call`, and while the `steps:`
    // line was printed AFTER it the figure was that re-compile's and not the
    // build's.
    //
    // MEASURED TWICE, FIVE DAYS APART, ON DIFFERENT TREES. When the defect was
    // found: `steps: 35352` against `41605839` for a build of the SAME sources
    // with the refusal removed — 1,177x. Re-measured 2026-09-27 on this tree by
    // `t1_image_steps_meter.rs` before this line existed: 35352 against
    // 43466282, a factor of 1,229. A build that did strictly LESS work reporting
    // three orders of magnitude more.
    //
    // ON THE SELF-IMAGE THE FAILURE PATH IS THE ONLY PATH, because `lib.t1` is
    // the zero-code-line facade and always refuses, so every self-image build
    // carried this: rung 103 printed `steps: 17436` where the same build before
    // `SAS-013` printed `27642890406`. That is the owner's point-3 meter — the
    // one that says lex has 4.4x ashtaka's instructions — reading six orders of
    // magnitude low and looking exactly like an answer.
    let build_steps = FUEL - it.fuel_remaining();
    let failed = match it.global("सङ्कलनविफलसंख्या") {
        Some(Value::Int(k)) => *k,
        _ => -1,
    };
    let linked = match it.global("संयोजितवस्तुसंख्या") {
        Some(Value::Int(k)) => *k,
        _ => -1,
    };
    // WHICH SOURCES, NOT HOW MANY. Eighteen self-images read `1` here and the
    // name was never printed, so the floor (`lib.t1`, zero code lines) was
    // indistinguishable from a real refusal without knowing the baseline.
    let names: Vec<String> = match it.global("सङ्कलनविफलनामकोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter_map(|v| v.octets())
            .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            .filter(|n| !n.is_empty())
            .collect(),
        _ => Vec::new(),
    };
    let which = if names.is_empty() {
        String::new()
    } else {
        format!(" ({})", names.join(" "))
    };
    // **A SOURCE THAT DECLARED NOTHING IS COUNTED APART** (2026-09-22). The rung
    // used to call every empty text a failure, so this line read
    // "1 source(s) failed to compile (lib)" in EVERY build this repository has
    // run: `lib.t1` holds no declaration on purpose. `मण्डलसङ्कलनम्` already told
    // the two apart (`सङ्कलनाघोषणाभेद` १ against `सङ्कलनानिर्णयभेद` २) and the rung
    // now reads it, so `failed` can express its zero and this line says so.
    let empty = match it.global("सङ्कलनरिक्तसंख्या") {
        Some(Value::Int(k)) => *k,
        _ => -1,
    };
    let declared_nothing = if empty > 0 {
        format!(", {empty} declared nothing")
    } else {
        String::new()
    };
    println!(
        "build:    {failed} source(s) failed to compile{which}{declared_nothing}, {linked} object(s) linked (startup included)"
    );
    // **THE STUB TALLY OF THE WHOLE-PROGRAM BUILD.** `अपूर्णगणनाकोश` counts every
    // stub the lowering planted, by cause, and resets once per program — so
    // after `मण्डलानिप्रतिबिम्बम्` it holds the LAST source's tally, not the
    // corpus's. Printed anyway, and labelled as what it is: the alternative is
    // reading the slot array `अपूर्णहेतुकोश`, which `आज्ञायोजनम्` overwrites as it
    // reuses instruction slots and which under-reports (measured 2026-09-22 on
    // `vishlesana.t1`: slot array `{39: 1}`, tally `{32: 1, 39: 1}`).
    if let Some(Value::Arena(a)) = it.global("अपूर्णगणनाकोश") {
        let mut tally = std::collections::BTreeMap::new();
        let mut total = 0i128;
        for (cause, v) in a.borrow().iter().enumerate() {
            if let Value::Int(k) = v
                && *k > 0
            {
                tally.insert(cause, *k);
                total += *k;
            }
        }
        println!("stubs:    {total} in the source compiled LAST (by cause: {tally:?})");
    }
    // **WHY IT FAILED, AND IT COSTS ONE RE-COMPILE TO EARN THE RIGHT TO SAY
    // IT** — `SAS-013`. The name above was already an improvement on the
    // count; the stop was still "अर्थ refused" and no more, and `W-279`
    // measured a refusal through this binary FOUR TIMES without ever learning
    // which name it was refused on.
    //
    // THE GLOBALS AFTER THE BUILD BELONG TO THE LAST SOURCE RESOLVED AND NOT
    // TO THE FAILED ONE. `मण्डलानिप्रतिबिम्बम्` walks every source through one
    // interpreter and `निर्णयः` resets its record on ENTRY, so reading them
    // here would name a site out of whichever source came last — an instrument
    // answering confidently about the wrong file, which is the defect this row
    // exists to remove and not one to add. So the first failed source is
    // compiled AGAIN, alone, and the record read after that is its own. One
    // source on the failure path, nothing at all on the success path.
    if failed > 0 {
        match names
            .first()
            .and_then(|n| inputs.iter().find(|(_, m)| m == n))
        {
            Some((path, module)) => match std::fs::read_to_string(path) {
                Ok(src) => {
                    let _ = it.call(
                        "शृङ्खलाॱमण्डलसङ्कलनम्",
                        vec![octets(src.as_bytes()), octets(module.as_bytes())],
                        FUEL,
                    );
                    match chain::refusal_site(&it) {
                        Some(site) => println!("refused:  {module}: {site}"),
                        // THE THIRD STATE, NAMED. A refusal `अर्थ` did not
                        // record is not the same as no refusal, and a silent
                        // line here would be read as the latter.
                        // THE THIRD STATE, AND THE RE-COMPILE ABOVE IS ISOLATED,
                        // so `निर्णयविरामभेद = ०` here means THIS SOURCE COMPILES
                        // ALONE and the refusal is load-dependent. `chain` words
                        // it; the fact was already in this run and was printed as
                        // "no site recorded" for as long as the line existed.
                        None => println!(
                            "refused:  {module}: {}",
                            chain::refusal_third_state(&it, true)
                        ),
                    }
                }
                Err(e) => println!("refused:  {module}: {} unreadable: {e}", path.display()),
            },
            // `सङ्कलनविफलनामकोश` named a module this run did not compile, or
            // named nothing at all. Say which rather than blame a source.
            None => println!(
                "refused:  {failed} source(s), and the product recorded no failed \
                 module name this run compiled ({} input(s))",
                inputs.len()
            ),
        }
    }
    // Steps spent by the build (the deterministic meter — a wall clock on this
    // machine measures the other lanes), and with T1_CALLS set the thirty
    // routines invoked most often: what located the assembler's table walk.
    //
    // TWO LINES AND NOT ONE, BECAUSE THE TRUTH HAS THREE STATES. `steps:` alone
    // could not distinguish a build from a build plus a diagnostic re-compile,
    // and that is how it came to report the re-compile for a whole cycle. The
    // re-compile is real work and is now named as its own figure on the one path
    // that spends it, rather than folded into the build's or hidden.
    println!("steps:    {build_steps}");
    if failed > 0 {
        println!(
            "diag:     {} more step(s) re-compiling the first refused source to name its site",
            FUEL - it.fuel_remaining()
        );
    }
    if std::env::var_os("T1_BUMP").is_some() {
        let (octets, growths, runs) = sadhana::t1::nirvahana::bump_total();
        let (first_blocks, records, record_octets, with_records) =
            sadhana::t1::nirvahana::bump_native_view();
        println!(
            "bump:     with records: first blocks {first_blocks} + growths {} + {records} records ({record_octets} octets) = {with_records} octets",
            octets.saturating_sub(first_blocks)
        );
        println!(
            "bump:     {octets} octets the native record region would hold ({growths} growths over {runs} runs)"
        );
        for (cap, key) in sadhana::t1::nirvahana::bump_largest(8) {
            println!("bump:       run at {key:#x} reached {cap} octets of capacity");
        }
    }
    if std::env::var_os("T1_CALLS").is_some() {
        let mut counts = sadhana::t1::nirvahana::call_counts();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, k) in counts.iter().take(30) {
            println!("calls:    {k:>12} {name}");
        }
        let mut steps = sadhana::t1::nirvahana::step_counts();
        steps.sort_by(|a, b| b.2.cmp(&a.2));
        for (name, incl, own) in steps.iter().take(30) {
            println!("self:     {own:>13}  incl {incl:>13}  {name}");
        }
    }
    if image.is_empty() {
        eprintln!("STOPPED AT link: the product returned an empty image");
        // THE CAUSE, WHEN THIS RUN ALREADY KNOWS IT (`W-342`). An empty image
        // beside a failed source is that source's consequence: the entry or a
        // callee it declared is not there to link. The stop used to name only
        // the stage, and the `refused:` line that names the site is on stdout
        // above it, where a reader of stderr alone never sees it.
        if failed > 0 {
            eprintln!(
                "link cause: {failed} source(s) failed to compile{which}; the `refused:` \
                 line above names the site, and nothing was written to {}",
                out.display()
            );
        }
        // The linker's own refusals (संयोजनॱसंयोजनदोषकोश), so the stop names
        // the symbol rather than the stage. AS TEXT, AND TO THE CURSOR: this
        // was `{:?}` of the arena cut at 1200 characters, which printed the
        // symbol's name as a list of octets and printed entries an earlier
        // link had left behind.
        for (k, r) in chain::link_refusals(&it).iter().enumerate() {
            eprintln!("link refusal {}: {r}", k + 1);
        }
        return ExitCode::FAILURE;
    }
    // ── A FAILED SOURCE REACHES THE EXIT CODE (`W-343`) ──
    //
    // MEASURED BEFORE THIS LINE: `build: 1 source(s) failed to compile (ड)` and
    // `refused: ड: resolve: नास्तिनाम at line 3 has no declaration` — module,
    // stage, symbol and line, a good diagnostic — and then exit 0. The report
    // was right and the verdict contradicted it, and a script reads the verdict.
    //
    // LAST, AND AFTER THE WRITE, DELIBERATELY. Everything above still happens:
    // `steps:`, `diag:` and `refused:` are read by instruments off a FAILED
    // build (`t1_image_steps_meter.rs` is about exactly that pair), and
    // `fixpoint.sh` and `rung-answer.sh` test for the FILE rather than the
    // status. So the one thing that changes is the one thing that was wrong.
    // The image is of the sources that DID compile, and the line says so
    // because a file at `-o` beside a nonzero exit otherwise reads as a build.
    //
    // `failed > 0`, not `!= 0`: `-1` is "this compiler declares no such
    // counter" — an older tree behind `--compiler` — which is a figure this run
    // does not have, not a failure it counted.
    //
    // `W-381`: THIS PATH TAKES NO GATE. The image is of the OTHER sources and
    // the exit is already a refusal; there is no program to compare.
    if failed > 0 {
        if let Err(e) = std::fs::write(&out, &image) {
            eprintln!("STOPPED AT write: {}: {e}", out.display());
            return ExitCode::FAILURE;
        }
        println!(
            "write:    {} octets of ELF -> {}",
            image.len(),
            out.display()
        );
        eprintln!(
            "STOPPED AT compile: {failed} source(s) failed to compile{which}. {} holds an \
             image of the OTHER sources only — it is not the program that was asked for.",
            out.display()
        );
        return ExitCode::FAILURE;
    }
    // ── THE DIFFERENTIAL GATE (`W-381`) — the header says what and why ──
    let unchecked = if no_predict {
        Some("--no-predict: the predict was not attempted")
    } else if unpredictable.is_some() {
        Some("predict:  none — the interpreter cannot call the entry")
    } else {
        None
    };
    let mut sources_read: Vec<PathBuf> = sources.clone();
    sources_read.extend(load.iter().cloned());
    gate(&Gate {
        image: &image,
        out: &out,
        tmp_out: &tmp_out,
        refused_out: &refused_out,
        provenance_out: &provenance_out,
        entry: entry.as_ref(),
        predicted: predicted.as_ref(),
        unchecked,
        accept: accept.as_deref(),
        yantra_run: yantra_run.as_deref(),
        native_steps,
        input: input.as_deref(),
        input_name: input_name.as_deref(),
        input_trace,
        sources: &sources_read,
    })
}

/// `out` with `suffix` appended to its file name (`a.elf` → `a.elf.tmp`).
fn with_suffix(out: &Path, suffix: &str) -> PathBuf {
    let mut s = out.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

/// Everything the gate reads, gathered by `main`.
struct Gate<'a> {
    image: &'a [u8],
    out: &'a Path,
    tmp_out: &'a Path,
    refused_out: &'a Path,
    provenance_out: &'a Path,
    entry: Option<&'a (String, String)>,
    predicted: Option<&'a Run>,
    unchecked: Option<&'static str>,
    accept: Option<&'a str>,
    yantra_run: Option<&'a Path>,
    native_steps: u64,
    input: Option<&'a Path>,
    input_name: Option<&'a str>,
    input_trace: i128,
    sources: &'a [PathBuf],
}

/// The verdict a build carries into `<out>.provenance`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Agreed,
    Accepted,
    Unchecked,
    NoEntry,
    Refused,
}

impl Verdict {
    fn word(self) -> &'static str {
        match self {
            Verdict::Agreed => "AGREED",
            Verdict::Accepted => "ACCEPTED",
            Verdict::Unchecked => "UNCHECKED",
            Verdict::NoEntry => "NO-ENTRY",
            Verdict::Refused => "REFUSED",
        }
    }
}

/// A figure for the log and the sidecar: the outcome without its octets, and
/// the octets by count and digest.
fn figure(r: &Run) -> String {
    let octets = |o: &[u8]| {
        format!(
            "{} octet(s) printed, sha256 {}",
            o.len(),
            agreement::hex(&agreement::sha256(o))
        )
    };
    match r {
        Run::Ran { status, out } => format!("finished, status {status}; {}", octets(out)),
        Run::Refused { why, out } => format!("refused: {why}; {}", octets(out)),
        Run::Paused { out } => format!("paused at a WAIT; {}", octets(out)),
        other => format!("{other:?}"),
    }
}

/// Whether `-o` names something that is not a file to rename onto: a character
/// or block device, a FIFO or a socket — through any symlink, since `metadata`
/// follows it. By FILE TYPE, not by a `/dev` prefix (W-381 review): a FIFO or a
/// symlink to a device elsewhere was replaced by a regular file on agreement.
/// AND any path given under `/dev/`: `-o /dev/stdout > x.elf` resolves to a
/// regular file, and a sibling `.tmp` or `.refused` cannot be made under `/dev`.
fn is_device(p: &Path) -> bool {
    if p.starts_with("/dev") {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        std::fs::metadata(p).is_ok_and(|m| {
            let t = m.file_type();
            t.is_char_device() || t.is_block_device() || t.is_fifo() || t.is_socket()
        })
    }
    #[cfg(not(unix))]
    {
        let _ = p;
        false
    }
}

/// THE GATE: compare, then write `<out>` or `<out>.refused`, and the sidecar.
fn gate(g: &Gate<'_>) -> ExitCode {
    let mut native: Option<Run> = None;
    let mut runner_version = String::from("-");
    let mut cause = String::new();
    let verdict = match (g.entry, g.predicted, g.unchecked) {
        (None, _, _) => Verdict::NoEntry,
        (Some(_), _, Some(why)) => {
            cause = format!("UNCHECKED — {why}, so nothing was compared");
            if g.accept.is_some() {
                Verdict::Unchecked
            } else {
                Verdict::Refused
            }
        }
        (Some(_), None, None) => {
            cause = "no predict was recorded, so nothing was compared".into();
            if g.accept.is_some() {
                Verdict::Unchecked
            } else {
                Verdict::Refused
            }
        }
        (Some(_), Some(predicted), None) => match run_native(g) {
            Err(why) => {
                cause = format!("the native half could not run: {why}");
                Verdict::Refused
            }
            Ok((version, n)) => {
                runner_version = version;
                let agreed = agreement::agree(predicted, &n);
                println!("predict:  {}", figure(predicted));
                println!("native:   {}", figure(&n));
                // An inconclusive half is not a disagreement: say which half
                // and why, so the console and the sidecar name the real cause.
                let differ = match (predicted, &n) {
                    (_, Run::Inconclusive(why)) | (Run::Inconclusive(why), _) => {
                        format!("INCONCLUSIVE — {why}")
                    }
                    _ => "the engines DISAGREE".into(),
                };
                native = Some(n);
                if agreed {
                    println!("gate:     AGREED — the image and the predict answer alike (W-381)");
                    Verdict::Agreed
                } else {
                    cause = differ;
                    if g.accept.is_some() {
                        Verdict::Accepted
                    } else {
                        Verdict::Refused
                    }
                }
            }
        },
    };
    // THE IMAGE GOES WHERE THE VERDICT SAYS. Written to `.tmp` first and renamed,
    // so `<out>` never exists half-written or before the verdict is in.
    // A DEVICE AT `-o` (`/dev/null`: "I want no file") takes the image directly
    // and no sidecar; there is nothing to rename onto and nothing to record.
    let device = is_device(g.out);
    let dest = if verdict == Verdict::Refused && !device {
        g.refused_out
    } else {
        g.out
    };
    // FAIL-CLOSED AT A DEVICE TOO (review of W-381 Part B): a REFUSED image is not
    // written to a device at all. `-o /dev/stdout` piped into a consumer would
    // otherwise hand it the refused ELF with only the exit status to say so.
    let withheld = device && verdict == Verdict::Refused;
    let written = if withheld {
        Ok(())
    } else if device {
        std::fs::write(dest, g.image)
    } else {
        std::fs::write(g.tmp_out, g.image).and_then(|()| std::fs::rename(g.tmp_out, dest))
    };
    if let Err(e) = written {
        eprintln!("STOPPED AT write: {}: {e}", dest.display());
        return ExitCode::FAILURE;
    }
    if withheld {
        println!(
            "write:    nothing — REFUSED, and {} is a device",
            dest.display()
        );
    } else {
        println!(
            "write:    {} octets of ELF -> {}",
            g.image.len(),
            dest.display()
        );
    }
    let mut p = String::new();
    p.push_str(&format!("verdict: {}\n", verdict.word()));
    p.push_str(&format!("reason: {}\n", g.accept.unwrap_or("-")));
    if !cause.is_empty() {
        p.push_str(&format!("cause: {cause}\n"));
    }
    p.push_str(&format!(
        "entry: {}\n",
        g.entry
            .map_or_else(|| "-".to_string(), |(m, r)| format!("{m} {r}"))
    ));
    p.push_str(&format!("t1_image: {}\n", env!("SASSEMBLY_BUILD_COMMIT")));
    p.push_str(&format!(
        "source_stamp: {}\n",
        env!("SASSEMBLY_SOURCE_STAMP")
    ));
    p.push_str(&format!("native_steps: {}\n", g.native_steps));
    p.push_str(&format!("yantra_run: {runner_version}\n"));
    p.push_str(&format!(
        "image_sha256: {}  {}\n",
        agreement::hex(&agreement::sha256(g.image)),
        dest.display()
    ));
    for src in g.sources {
        let digest = std::fs::read(src)
            .map(|b| agreement::hex(&agreement::sha256(&b)))
            .unwrap_or_else(|e| format!("unreadable ({e})"));
        p.push_str(&format!("source_sha256: {digest}  {}\n", src.display()));
    }
    if let Some(i) = g.input {
        let digest = std::fs::read(i)
            .map(|b| agreement::hex(&agreement::sha256(&b)))
            .unwrap_or_else(|e| format!("unreadable ({e})"));
        p.push_str(&format!("input_sha256: {digest}  {}\n", i.display()));
    }
    p.push_str(&format!(
        "predict: {}\n",
        g.predicted.map_or_else(|| "-".to_string(), figure)
    ));
    p.push_str(&format!(
        "native: {}\n",
        native.as_ref().map_or_else(|| "-".to_string(), figure)
    ));
    if device {
        // no sidecar beside a device
    } else if let Err(e) = std::fs::write(g.provenance_out, p) {
        eprintln!("STOPPED AT provenance: {}: {e}", g.provenance_out.display());
        return ExitCode::FAILURE;
    }
    let sidecar = if device {
        "no sidecar beside a device".to_string()
    } else {
        g.provenance_out.display().to_string()
    };
    println!("prov:     {} -> {sidecar}", verdict.word());
    match verdict {
        Verdict::Refused => {
            let held = if withheld {
                "was withheld".to_string()
            } else {
                format!("is at {}", g.refused_out.display())
            };
            eprintln!(
                "STOPPED AT the differential gate (W-381): {cause}. The image {held} and \
                 NOT at {}. Pass --accept-divergence <reason> to write it anyway; the reason \
                 is recorded in {sidecar}.",
                g.out.display(),
            );
            ExitCode::from(DIVERGED)
        }
        Verdict::Accepted | Verdict::Unchecked => {
            println!(
                "gate:     {} — {cause}; written under --accept-divergence: {}",
                verdict.word(),
                g.accept.unwrap_or("")
            );
            println!("run it:   yantra-run {}", g.out.display());
            ExitCode::SUCCESS
        }
        Verdict::Agreed | Verdict::NoEntry => {
            println!("run it:   yantra-run {}", g.out.display());
            ExitCode::SUCCESS
        }
    }
}

/// THE NATIVE HALF: `yantra-run` as a child on `<out>.tmp`, after checking that
/// it is this build's. Answers its version and the outcome, or why it could not.
fn run_native(g: &Gate<'_>) -> Result<(String, Run), String> {
    let runner = match g.yantra_run {
        Some(p) => p.to_path_buf(),
        None => std::env::current_exe()
            .map_err(|e| format!("this binary's own path: {e}"))?
            .with_file_name("yantra-run"),
    };
    if !runner.is_file() {
        return Err(format!(
            "no yantra-run at {} — build it beside this binary (cargo build --release -p \
             yantra --bin yantra-run) or name it with --yantra-run",
            runner.display()
        ));
    }
    let v = std::process::Command::new(&runner)
        .arg("--version")
        .output()
        .map_err(|e| format!("{} --version: {e}", runner.display()))?;
    let said = String::from_utf8_lossy(&v.stderr);
    let version = said
        .lines()
        .find_map(|l| l.strip_prefix("version: "))
        .unwrap_or("")
        .trim()
        .to_string();
    // THE CONTENT STAMP, NOT THE COMMIT (`W-381` review; owner: "un-stamped build
    // pairs must no longer evaluate as match passes"). Both binaries seal a hash
    // of `crates/sadhana/src` and `crates/yantra/src` (`tools/build-stamp.rs` via
    // `tools/src-rev.sh`), so an archive build with no git still gets a real stamp,
    // and `unknown` (no stamp could be computed) never matches, not even itself.
    let s = std::process::Command::new(&runner)
        .arg("--source-stamp")
        .output()
        .map_err(|e| format!("{} --source-stamp: {e}", runner.display()))?;
    let theirs = String::from_utf8_lossy(&s.stderr)
        .lines()
        .find_map(|l| l.strip_prefix("source: "))
        .unwrap_or("")
        .trim()
        .to_string();
    let mine = env!("SASSEMBLY_SOURCE_STAMP");
    if mine == "unknown" || theirs.is_empty() || theirs == "unknown" {
        return Err(format!(
            "this t1_image's source stamp is {mine:?} and {}'s is {theirs:?}: a build \
             with no source stamp cannot be shown to be this tree's",
            runner.display()
        ));
    }
    if theirs != mine {
        return Err(format!(
            "{} has source stamp {theirs:?} and this t1_image {mine:?}: a runner from \
             other sources would judge this image by other rules",
            runner.display()
        ));
    }
    // The image the child runs: `<out>.tmp`, or a scratch file when `-o` is a device.
    let run_path = if is_device(g.out) {
        std::env::temp_dir().join(format!("t1_image-gate-{}.elf", std::process::id()))
    } else {
        g.tmp_out.to_path_buf()
    };
    std::fs::write(&run_path, g.image).map_err(|e| format!("{}: {e}", run_path.display()))?;
    let verdict_file = with_suffix(&run_path, ".verdict");
    let _ = std::fs::remove_file(&verdict_file);
    let mut cmd = std::process::Command::new(&runner);
    cmd.arg(&run_path)
        .env("YANTRA_VERDICT", &verdict_file)
        .env_remove("YANTRA_INPUT")
        .env_remove("YANTRA_INPUT_NAME")
        .env_remove("YANTRA_INPUT_TRACE");
    // THE GATE'S OWN BUDGET ([`NATIVE_STEPS`], or `--native-steps`), set on the
    // child whatever the caller's `YANTRA_STEPS` says: without it every real
    // program stopped at `yantra-run`'s default million and was refused.
    cmd.env("YANTRA_STEPS", g.native_steps.to_string());
    // THE SAME INPUT THE PREDICT HAD, placed the way `yantra-run` places it.
    if let Some(i) = g.input {
        cmd.env("YANTRA_INPUT", i);
        if let Some(n) = g.input_name {
            cmd.env("YANTRA_INPUT_NAME", n);
        }
        if g.input_trace > 0 {
            cmd.env("YANTRA_INPUT_TRACE", g.input_trace.to_string());
        }
    }
    println!(
        "native:   {} {} (the gate's native half, W-381)",
        runner.display(),
        run_path.display()
    );
    let r = cmd
        .output()
        .map_err(|e| format!("{}: {e}", runner.display()))?;
    let text = std::fs::read_to_string(&verdict_file).unwrap_or_default();
    let _ = std::fs::remove_file(&verdict_file);
    if run_path.as_path() != g.tmp_out {
        let _ = std::fs::remove_file(&run_path);
    }
    let mut lines = text.lines();
    let out = r.stdout;
    let outcome = match lines.next().map(|l| l.split(' ').collect::<Vec<_>>()) {
        Some(w) if w.first() == Some(&"finisher") && w.len() == 3 => match w[2].parse::<u64>() {
            Ok(s) => Run::Ran {
                status: i128::from(s),
                out,
            },
            Err(_) => Run::Code(w[1].parse().unwrap_or(0)),
        },
        Some(w) if w.first() == Some(&"wait") => Run::Paused { out },
        Some(w) if w.first() == Some(&"steplimit") => Run::Inconclusive(format!(
            "the native run did not finish (step limit {}; raise it with --native-steps)",
            g.native_steps
        )),
        Some(w) if w.first() == Some(&"spin") => Run::Fault("SpinForever".into()),
        Some(w) if w.first() == Some(&"fault") => Run::Fault(w[1..].join(" ")),
        _ => Run::Inconclusive(format!(
            "yantra-run wrote no verdict (exit {:?}): {}",
            r.status.code(),
            String::from_utf8_lossy(&r.stderr)
                .lines()
                .last()
                .unwrap_or("")
        )),
    };
    Ok((version, outcome))
}
