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
//!     t1_image [--spec-root <dir>] [--compiler <dir>] [--load <x.t1>]… [--entry <module> <routine>] -o <out.elf> <a.t1> <b.t1> …
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
use sadhana::t1::chain;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

/// Fuel for the whole build: the census spends ~80e9 per source; twenty sources
/// and a link need room, and running out is a report, not a crash.
const FUEL: u64 = 4_000_000_000_000;

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
         \x20              -o <out.elf> <file.t1>..."
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
    // THE INPUT CHANNEL'S INTERPRETED HALF. `yantra-run` places a file in RAM
    // for the image (`YANTRA_INPUT`); these hand the SAME octets to the predict,
    // so the two sinks are about one input and can be compared with `cmp`.
    let mut input: Option<PathBuf> = None;
    let mut input_name: Option<String> = None;
    let mut input_trace: i128 = 0;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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
        texts.push(octets(src.as_bytes()));
        names.push(octets(name.as_bytes()));
    }
    let n = i128::try_from(sources.len()).unwrap_or(0);

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
    if let Some((m, r)) = &entry {
        let name = format!("{m}ॱ{r}");
        match it.call(&name, vec![], FUEL) {
            Ok(Value::Int(n)) => println!(
                "predict:  interpreted {name} -> {n}; the image's exit status must equal it"
            ),
            Ok(v) => println!("predict:  interpreted {name} answered {v:?}"),
            Err(e) => println!("predict:  interpreted {name} refused: {e:?}"),
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
                        None => println!(
                            "refused:  {module}: no site recorded (निर्णयविरामभेद = {})",
                            it.global("निर्णयविरामभेद")
                                .map_or_else(|| "-".to_string(), |v| format!("{v:?}"))
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
    println!("steps:    {}", FUEL - it.fuel_remaining());
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
        // The linker's own refusals (संयोजनॱसंयोजनदोषकोश), so the stop names
        // the symbol rather than the stage.
        if let Some(v) = it.global("संयोजनदोषकोश") {
            let t = format!("{v:?}");
            eprintln!("link refusals: {}", &t[..t.len().min(1200)]);
        }
        return ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::write(&out, &image) {
        eprintln!("STOPPED AT write: {}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    println!(
        "write:    {} octets of ELF -> {}",
        image.len(),
        out.display()
    );
    println!("run it:   yantra-run {}", out.display());
    ExitCode::SUCCESS
}
