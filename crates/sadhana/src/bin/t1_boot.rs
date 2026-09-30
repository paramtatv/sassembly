//! THE BOOT PROBE: `.t1` emits, Rust assembles and links. Answers whether the
//! compiled compiler runs natively, and is the reference side of every rung.
//!
//! **LANDED 2026-09-14, AND THE REASON IS THE RECORD.** This said "DIAGNOSTIC,
//! NOT LANDED" and lived as an uncommitted file in one worktree — in no commit,
//! on no branch, in no other tree — while `native-compile.py`, every `.s`
//! reference and every rung figure in `.loop/STATE.md` was produced by it. A
//! tool that is load-bearing for numbers in the record and cannot be rebuilt at
//! a commit makes "measured at X" a sentence nobody can say: one `git clean`
//! would have deleted the source, and no second reader could reproduce a
//! figure. sansos-c1 found it while rebuilding tools for a sweep, having caught
//! themselves checking the inputs and not the instrument.
//! The objects are byte-identical to the .t1 assembler's (twin AGREE, every source).
//!   t1_boot --object <out.o> <a.t1>
//!   t1_boot --link [--entry <m> <r>] -o <out.elf> <a.o> ...
use sadhana::encode::Target;
use sadhana::kosha::LOAD_ADDRESS;
use sadhana::nidana::Language;
use sadhana::t1::chain;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::{assemble_object, link_objects, vastu};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const FUEL: u64 = 4_000_000_000_000;
fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn load(spec: &Path) -> Interpreter {
    // DEPENDENCY ORDER, TAKEN FROM `CHAIN` AND NOT RESTATED HERE. This used to
    // carry its own nine-name prefix with the rest swept up from the directory,
    // which is a second loader list — the thing that costs a day when it drifts.
    // `chain::CHAIN` is the one both the interpreted driver and `t1_image` walk;
    // taking the order from it means a module added there is added here, and the
    // sweep below still catches anything the chain does not carry (`lib.t1`).
    let dir = PathBuf::from("crates/sadhana-t1/src");
    let mut paths: Vec<PathBuf> = chain::CHAIN.iter().map(|(n, _)| dir.join(n)).collect();
    for e in std::fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "t1") && !paths.contains(&p) {
            paths.push(p);
        }
    }
    // T1_BOOT_EXTRA=<path>: one more source loaded beside the corpus, so a
    // scratch module's entry can be predicted in the interpreter.
    if let Ok(extra) = std::env::var("T1_BOOT_EXTRA") {
        paths.push(PathBuf::from(extra));
    }
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    Interpreter::load_paths(&refs, spec).expect("compiler loads")
}
fn text(it: &mut Interpreter, name: &str, args: Vec<Value>) -> String {
    match it.call(name, args, FUEL) {
        Ok(v) => v
            .octets()
            .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            .unwrap_or_default(),
        Err(e) => {
            eprintln!("{name}: {e:?}");
            String::new()
        }
    }
}
fn asm(t: &str, name: &str) -> Vec<u8> {
    assemble_object(
        t,
        Some(name),
        Target::Uncompressed,
        false,
        Language::English,
    )
    .unwrap_or_else(|ds| {
        for d in ds.iter().take(3) {
            eprintln!("assemble {name}: line {}: {}", d.line, d.reason);
        }
        std::process::exit(2)
    })
}
fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().collect();
    let spec = PathBuf::from("spec");
    // --assemble-text <text-file> <module-name> -o <object>: the Rust half alone,
    // on a text saved by T1_BOOT_TEXT — the .t1 half costs 28 minutes on the
    // fixpoint module and need not be repaid to re-time the assembler.
    if a.get(1).map(String::as_str) == Some("--assemble-text") {
        let text = std::fs::read_to_string(&a[2]).unwrap();
        let name = a[3].clone();
        let out = PathBuf::from(&a[5]);
        let t = std::time::Instant::now();
        let o = asm(&text, &name);
        std::fs::write(&out, &o).unwrap();
        println!(
            "assembled {} octets of text -> {} octets in {:.1}s",
            text.len(),
            o.len(),
            t.elapsed().as_secs_f64()
        );
        return ExitCode::SUCCESS;
    }
    if a.get(1).map(String::as_str) == Some("--object") {
        let out = PathBuf::from(&a[2]);
        let src_path = PathBuf::from(&a[3]);
        let src = std::fs::read_to_string(&src_path).unwrap();
        let name = chain::module_name(&src)
            .unwrap_or_else(|| src_path.file_stem().unwrap().to_string_lossy().into_owned());
        let mut it = load(&spec);
        // collect every input source into the registry first, as the census does
        let mut all: Vec<PathBuf> = std::fs::read_dir("crates/sadhana-t1/src")
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "t1"))
            .collect();
        all.sort();
        // T1_BOOT_COLLECT=<path>[,<path>…]: MORE SOURCES INTO THE REGISTRY, not
        // into the loader. `T1_BOOT_EXTRA` above adds a module the INTERPRETER
        // runs; this adds one the compiled-under-test program can NAME, which is
        // a different thing and is what a cross-module pair needs. The registry
        // walk above is hard-coded to `crates/sadhana-t1/src`, so `spec/rung/`'s
        // two-module pair could not reach this dump at all — W-279 measured its
        // refusal four times through `t1_image`, which prints no `भेद` global,
        // and so could name the stop only as "अर्थ refused".
        if let Ok(list) = std::env::var("T1_BOOT_COLLECT") {
            for p in list.split(',').filter(|s| !s.is_empty()) {
                all.push(PathBuf::from(p));
            }
        }
        for p in &all {
            let s = std::fs::read_to_string(p).unwrap();
            let _ = it.call("शृङ्खलाॱपठनम्", vec![octets(s.as_bytes())], FUEL);
        }
        let _ = it.call("अर्थॱसञ्चयसिद्धिः", vec![], FUEL);
        let t = text(
            &mut it,
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets(name.as_bytes())],
        );
        // **DECLARED NOTHING IS NOT REFUSED** (2026-09-22). `मण्डलसङ्कलनम्`
        // answers `सङ्कलनविरामभेद = १` (`सङ्कलनाघोषणाभेद`) for a source that
        // parsed to no declarations and `२` for one `अर्थ` refused; both give
        // empty text. `lib.t1` holds no declaration on purpose and this binary
        // called it REFUSED with "no site recorded (निर्णयविरामभेद = Int(3))",
        // which is the third state's honest answer to the WRONG question.
        let declared_nothing = matches!(it.global("सङ्कलनविरामभेद"), Some(Value::Int(1)));
        if t.is_empty() && declared_nothing {
            println!("empty:  {name} declares nothing — no object, and not a refusal");
            return ExitCode::SUCCESS;
        }
        if t.is_empty() {
            // **THE SITE FIRST, THE DUMP AFTER** — `SAS-013`. Everything below
            // this line is thirty globals of state; none of them says WHICH
            // routine or WHICH line, and a builder reading `सङ्कलनविरामभेद =
            // Int(2)` off a refused scratch module spent an hour bisecting
            // variants to find a call to a routine nobody declared. `--object`
            // compiles ONE source, so the resolver's record is unambiguously
            // this module's: no re-compile is needed to earn the right to
            // print it, unlike `t1_image`, which compiles twenty.
            match chain::refusal_site(&it) {
                Some(site) => eprintln!("REFUSED {name}: {site}"),
                // NOT SILENCE. A refusal whose stage recorded no site is a
                // THIRD state — `निर्णयविरामभेद` ० or ३, or a stop before
                // `अर्थ` — and printing nothing would read as the second.
                // `--object` compiled ONE source, so this is NOT the isolated
                // re-compile case: `०` beside empty output means a stage after
                // decide refused, and this file is where to look.
                None => eprintln!("REFUSED {name}: {}", chain::refusal_third_state(&it, false)),
            }
            for g in [
                "सङ्कलनविरामभेद",
                // WHICH HALF OF `अर्थ` REFUSED, when `सङ्कलनविरामभेद` reads २
                // (`सङ्कलनानिर्णयभेद`). That code says only "अर्थ refused" because
                // `निर्णयः` folds resolve and typecheck into one `बूल`; this one
                // separates them — १ resolve (a name was never found), २ typecheck
                // (the name was found and its signature disagreed). Different file,
                // different fix, and until W-279 they were one answer.
                "निर्णयविरामभेद",
                "यन्त्रनिषेधसंख्या",
                "यन्त्रनिषेधपर्व",
                "यन्त्रनिषेधलक्ष्य",
                "यन्त्रनामसूचकाङ्क",
                "सञ्चयसंज्ञासूचकाङ्क",
                "संज्ञासूचकाङ्क",
                "यन्त्रनिषेधभेद",
                "यन्त्रनिषेधवृत्ति",
                "यन्त्रनिषेधचिह्न",
                // WHAT `यन्त्रनिषेधलक्ष्य` ABOVE MEANS WHEN `भेद` IS ४. Five routines
                // raise `UnnamedSymbol` and their records are identical at four of
                // them when the offending symbol is ०, which a missed lookup of
                // symbol ० always is; `लक्ष्य` carries a site code, and these are the
                // codes it can carry, so the dump says which without a table.
                "यन्त्रनामस्थानचिह्नम्",
                "यन्त्रनामस्थानपरीक्षा",
                "यन्त्रनामस्थानाह्वानम्",
                "यन्त्रनामस्थानवृत्तिः",
                "यन्त्रनामस्थानप्रवेशः",
                "असदस्यनाम",
                "असदस्यमण्डल",
                "असदस्यमस्ति",
                "असञ्चितप्रयोगसंख्या",
                "असङ्गृहीतमण्डलसंख्या",
                "रिक्तसञ्चितप्रकारसंख्या",
                "अलिखितप्रकारसंख्या",
                "सञ्चयदोषभेद",
                "सञ्चयसिद्धमस्ति",
                "निर्णायकसिद्धः",
                "प्रविष्टिसूचकाङ्क",
                "यन्त्रनिषेधवृत्ति",
            ] {
                let v = it
                    .global(g)
                    .map(|v| format!("{v:?}"))
                    .unwrap_or_else(|| "-".into());
                eprintln!("  {g} = {}", &v[..v.len().min(140)]);
            }
            // T1_BOOT_BLOCKS: every block record of the module (its terminator
            // included), for a nil-terminator bisect.
            if std::env::var("T1_BOOT_BLOCKS").is_ok()
                && let Some(v) = it.global("पर्वकोश")
            {
                let t = format!("{v:?}");
                for (k, piece) in t.split("Record(").enumerate() {
                    eprintln!("  block[{k}] {}", &piece[..piece.len().min(400)]);
                }
            }
            eprintln!("no text for {name}");
            return ExitCode::FAILURE;
        }
        if let Some(v) = it.global("अपूर्णविवरणम्") {
            eprintln!("  अपूर्णविवरणम् (last cause-32 base kind) = {v:?}");
        }
        for g in [
            "संरचनार्थभेद",
            "सम्भाव्यार्थभेद",
            "खण्डार्थभेद",
            "दोषार्थभेद",
            "पूर्णाङ्कार्थभेद",
            "नामार्थभेद",
            "स्थानार्थभेद",
            "दोषयुक्तार्थभेद",
            "गणनार्थभेद",
        ] {
            if let Some(v) = it.global(g) {
                eprintln!("    {g} = {v:?}");
            }
        }
        if let Some(Value::Arena(a)) = it.global("अपूर्णहेतुकोश") {
            let mut hist = std::collections::BTreeMap::new();
            for v in a.borrow().iter() {
                if let Value::Int(k) = v
                    && *k > 0
                {
                    *hist.entry(*k).or_insert(0usize) += 1;
                }
            }
            eprintln!("  stubs by cause (अपूर्णहेतुकोश): {hist:?}");
        }
        // **THE TALLY, NOT THE SLOT ARRAY** (2026-09-22). `अपूर्णहेतुकोश` records
        // a cause AT AN INSTRUCTION SLOT and `आज्ञायोजनम्` reuses those slots as
        // each routine's IR is built, so the histogram above is only what
        // survives at the end — it read `{}` for seventeen of the twenty
        // sources, which is not "no stubs" but "the last routine had none".
        // `अपूर्णगणनाकोश` is incremented on every stub and reset once per
        // program, and until now NO driver read it.
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
            eprintln!("  stubs TALLIED (अपूर्णगणनाकोश): total {total} {tally:?}");
        }
        let records = matches!(
            it.call("यन्त्रोत्सर्जनॱयन्त्ररचनाप्रश्नः", vec![], 1_000_000),
            Ok(Value::Bool(true))
        );
        // T1_BOOT_TEXT=<path>: the module's emitted assembly text, for reading
        // one routine's code as emitted.
        if let Ok(path) = std::env::var("T1_BOOT_TEXT") {
            std::fs::write(path, &t).unwrap();
        }
        let o = asm(&t, &name);
        std::fs::write(&out, &o).unwrap();
        std::fs::write(
            out.with_extension("records"),
            if records { "1" } else { "0" },
        )
        .unwrap();
        // The checker's first unresolved name, if any (artha's अनिर्णीतनाम): the
        // lowering's name_unresolved stubs are this, per name.
        // AND IT PRINTS THE NAME, NOT THE OCTETS. `{v:?}` on a `अङ्कः अन्तः अ८`
        // global renders `Octets(Octets { data: [224, 164, …] })` — the bytes of
        // the very name the line exists to say. `resolve_site` reads the guard
        // beside it, which is the same test this branch was making by hand.
        if let Some(site) = chain::resolve_site(&it) {
            eprintln!("  unresolved (first): {site}");
        }
        println!(
            "object: {name} text {} object {} records {records}",
            t.len(),
            o.len()
        );
        return ExitCode::SUCCESS;
    }
    if a.get(1).map(String::as_str) == Some("--link") {
        let mut i = 2;
        let mut entry: Option<(String, String)> = None;
        let mut out = PathBuf::new();
        let mut objs = vec![];
        while i < a.len() {
            match a[i].as_str() {
                "--entry" => {
                    entry = Some((a[i + 1].clone(), a[i + 2].clone()));
                    i += 3;
                }
                "-o" => {
                    out = PathBuf::from(&a[i + 1]);
                    i += 2;
                }
                s => {
                    objs.push(PathBuf::from(s));
                    i += 1;
                }
            }
        }
        let mut it = load(&spec);
        if let Some((m, r)) = &entry {
            it.call(
                "शृङ्खलाॱप्रवेशन्यासः",
                vec![octets(m.as_bytes()), octets(r.as_bytes())],
                1_000_000,
            )
            .expect("entry");
        }
        let mut records = false;
        let mut objects = vec![];
        for p in &objs {
            if std::fs::read_to_string(p.with_extension("records"))
                .map(|s| s.trim() == "1")
                .unwrap_or(false)
            {
                records = true;
            }
            objects.push(vastu::read(&std::fs::read(p).unwrap()).expect("object reads back"));
        }
        let st = text(&mut it, "शृङ्खलाॱआरम्भपाठ्यरचना", vec![Value::Bool(records)]);
        if std::env::var("T1_BOOT_SHOW_STARTUP").is_ok() {
            eprintln!("--- startup text ---\n{st}--- end ---");
        }
        let startup = vastu::read(&asm(&st, "यन्त्रारम्भ")).expect("startup reads back");
        let mut all = vec![startup];
        all.extend(objects);
        match link_objects(&all, LOAD_ADDRESS) {
            Ok(elf) => {
                std::fs::write(&out, &elf).unwrap();
                println!(
                    "link: {} objects + startup (records {records}) -> {} octets -> {}",
                    objs.len(),
                    elf.len(),
                    out.display()
                );
            }
            Err(es) => {
                eprintln!("link: {}", es.join("; "));
                return ExitCode::FAILURE;
            }
        }
        if let Some((m, r)) = &entry
            && r.starts_with("स्वपरीक्ष")
        {
            match it.call(&format!("{m}ॱ{r}"), vec![], FUEL) {
                Ok(v) => println!("predict: interpreted {r} -> {v:?}"),
                Err(e) => println!("predict: {r} refused {e:?}"),
            }
        }
        // T1_BOOT_PREDICT=<module>ॱ<routine>: run any entry in the interpreter and
        // print its answer, so native and interpreted compare on the same state.
        if let Ok(name) = std::env::var("T1_BOOT_PREDICT") {
            match it.call(&name, vec![], FUEL) {
                Ok(v) => println!("predict: interpreted {name} -> {v:?}"),
                Err(e) => println!("predict: {name} refused {e:?}"),
            }
            // T1_BOOT_PREDICT_TEXT=<path>: the emitter's text after that run.
            if let Ok(path) = std::env::var("T1_BOOT_PREDICT_TEXT") {
                let t = text(&mut it, "उत्सर्जनॱनिर्गमांशः", vec![Value::Int(0)]);
                std::fs::write(path, &t).unwrap();
            }
        }
        if let Some((_, r)) = &entry
            && r == "never"
        {
            match it.call("शृङ्खलाॱस्वपरीक्षा", vec![], FUEL) {
                Ok(v) => println!("predict: interpreted स्वपरीक्षा -> {v:?}"),
                Err(e) => println!("predict: refused {e:?}"),
            }
        }
        return ExitCode::SUCCESS;
    }
    eprintln!("usage: t1_boot --object <out.o> <a.t1> | --link [--entry m r] -o <out.elf> <objs>");
    ExitCode::FAILURE
}
