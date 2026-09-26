//! **`W-279` — DOES ANY OBJECT THE CORPUS EMITS HOLD A POINTER TO STORAGE IT
//! NEVER PLACES?**
//!
//! # The defect class the whole `स्वपरीक्षा` ladder is structurally blind to
//!
//! Every rung of the ladder answers `११००० + <object octets>`. `W-293` gives a
//! run-typed module global a POINTER WORD in `ॱदत्त` and its storage in
//! `ॱरिक्त` — and `ॱरिक्त` is space in memory, ABSENT FROM THE FILE. So an
//! emitter that placed the pointer and forgot the storage emits an object of
//! **exactly the same length**: the pointer is eight octets either way and the
//! run's own kilobyte never appears in an octet count at all. A rung answering
//! ११०७२ answers ११०७२ while every read of that global resolves through a null
//! base.
//!
//! `t1_emitted_labels.rs` gave that state a name — [`Storage::DanglingPointer`]
//! — and exercised it by HAND MUTATION on one fixture. **Nothing asked whether a
//! real chain, on the real corpus, ever emits one.** This file asks, over every
//! object the twenty-one sources emit, and it is the first check on this tree of
//! that class.
//!
//! # What it measured, and the two claims it corrected
//!
//! ```text
//!   21 sources · 20 objects · 694 globals · 490 words · 204 runs · 206,928 octets
//!   dangling 0 · zero-sized 0 · shared storage 0 · duplicate label 0
//!   distinct run sizes: 1024 (×202) and 40 (×2)
//! ```
//!
//! **`riscv64.rs:1427` SAYS "163 CORPUS GLOBALS AT 1 KiB EACH" AND BOTH HALVES
//! OF THAT ARE WRONG.** `t1_emitted_labels.rs`'s `RUN_GLOBAL_SOURCE` margin
//! repeats it. Measured on this tree there are **204**, and they are NOT all a
//! kilobyte: two of them are **40 octets**, and those two are not arenas at all.
//! Both are RECORD-typed globals —
//!
//! * `encode.t1:1514` `अन्तिमसङ्केतनदोषः ॱॱ सङ्केतनदोष` — the record at `:1078`,
//!   five fields.
//! * `yantrotsarjana.t1:665` `यन्त्रचौकटम् ॱॱ यन्त्रचौकट` — the record at `:621`,
//!   five fields.
//!
//! — so 40 is `५ × ८`, a field count, where 1,024 is `खण्डसामर्थ्यम् गुणनम् ८`,
//! an arena capacity (`ir.t1:999`, `:4492`). **The run-storage form carries TWO
//! shapes and the margin names one.** The wrong figure is left standing in
//! `riscv64.rs` for now — it is a comment in `src/`, and correcting it rebuilds
//! every crate downstream of `sadhana`, which the gate could not cover on the
//! cycle that measured this. It is named in the ledger as a one-line correction.
//!
//! # The counts are METRIC and the properties are the assertions
//!
//! Owner ruling 2026-09-13, point 1: a count encodes nothing about correctness
//! and re-taking pins was half the cost of a landing. So `694`, `204` and
//! `206,928` are printed as `METRIC` and asserted by NOTHING. What is asserted
//! are the four properties no length can see, each of which stays true however
//! the corpus grows.
//!
//! # AND THE SECOND INSTRUMENT, BECAUSE ONE READER CANNOT CATCH ITSELF
//!
//! [`crate::emitted::data_globals`] finds runs by walking a three-line window
//! and then searching forward for the storage line. A reader with a broken
//! window finds NO globals and every property above is then vacuously true.
//! So each object is counted a SECOND way — `॥ कोष्ठकम् ॱरिक्त ॥` openings,
//! which `riscv64.rs:1433` emits exactly once per run and nowhere else in
//! `emit_module` — and the two counts must agree. When two instruments
//! disagree the run says so by name and by number rather than by a total.

mod emitted;

use emitted::{Storage, data_globals};
use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::riscv64;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.t1` the corpus ships, in name order — READ OFF THE DIRECTORY.
///
/// **A HAND-WRITTEN LIST HERE WOULD PASS WHILE A SOURCE WAS MISSING FROM IT**,
/// which is the failure `chain.rs`'s `CHAIN` margin records having happened
/// twice, and a sweep whose whole claim is "every object" cannot be the file
/// that decides which objects there are. `t1_hopcorpus.rs:37` reads the
/// directory for the same reason.
fn corpus_paths() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    assert!(
        !v.is_empty(),
        "the corpus is not empty; this sweep read no sources"
    );
    v
}

/// What one source came to. **THREE STATES, NOT TWO.**
///
/// "Emitted" against "did not" would put `lib.t1` — which declares no
/// `मण्डलम्` at all, because Rust's crate root lists modules and T1 has no crate
/// root (`t1_hopcorpus.rs:50`) — in the same bucket as a source the chain
/// REFUSED. Those are different facts and only one of them is about the
/// compiler, so they are different variants and the test asserts on which.
#[derive(Debug)]
enum Object {
    /// The emitted Sassembly text.
    Emitted(String),
    /// The source declares no module, so there is no name to compile it under.
    DeclaresNoModule,
    /// The chain refused it, with its own reason.
    Refused(String),
}

/// THIS FILE'S OWN LOADER AND ITS OWN DRIVER.
///
/// A fresh [`Front`] per source, which is the SLOW choice and the right one:
/// reusing one across twenty sources saves most of the minute and is exactly
/// the shape `W-279` found once already — a measurement whose interval carries
/// state from its predecessor, which produced a count of entries written of −39
/// and went unnoticed because every intermediate number looked plausible
/// (`t1_hopcorpus.rs:15`). A slow measurement beats a contaminated one.
///
/// The back half is `riscv64::emit_module`, the Rust emitter. That is the
/// LENIENT arm and it is named as such: the `.t1` twin `यन्त्रदत्तोत्सर्जनम्`
/// (`yantrotsarjana.t1:2104`) emits these same lines from the same arena, and
/// running THIS sweep through it is the gap this file leaves open.
fn compile(path: &Path) -> Object {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    let Some(module) = module_name(&src) else {
        return Object::DeclaresNoModule;
    };
    let mut front = match Front::load(&spec_root()) {
        Ok(f) => f,
        Err(e) => return Object::Refused(format!("the front end does not load: {e}")),
    };
    let text = (|| -> Result<String, String> {
        front.lex(&src)?;
        front.parse()?;
        front.resolve()?;
        front.typecheck()?;
        front.build_ir()?;
        let module = front.module(&module, None)?;
        riscv64::emit_module(&module).map_err(|e| format!("{e:?}"))
    })();
    match text {
        Ok(t) => Object::Emitted(t),
        Err(e) => Object::Refused(e),
    }
}

/// `॥ कोष्ठकम् ॱरिक्त ॥` — THE SECOND COUNT. See the header: `riscv64.rs:1433`
/// opens this section once per run global and `emit_module` opens it nowhere
/// else, so this number and [`data_globals`]'s run count are two readings of
/// one fact.
///
/// (`emit_startup_with_records` opens one more for the record region — but that
/// is the STARTUP object, which this sweep never builds. A tree that moved the
/// region into `emit_data` would make these two disagree by exactly one per
/// object, which is the kind of drift this pair exists to name.)
fn rikta_openings(text: &str) -> usize {
    text.lines().filter(|l| *l == "॥ कोष्ठकम् ॱरिक्त ॥").count()
}

/// One object's `.data` globals, read into the four findings and the counts.
#[derive(Default)]
struct Findings {
    words: usize,
    runs: usize,
    octets: u64,
    dangling: Vec<String>,
    zero_sized: Vec<String>,
    shared_storage: Vec<String>,
    duplicate_label: Vec<String>,
    unaligned: Vec<String>,
    sizes: BTreeSet<u64>,
}

/// THE FOUR PROPERTIES, READ OFF ONE OBJECT.
///
/// Each is a thing the emitter can really do wrong and **none of them moves an
/// octet count**, which is the whole reason this file exists:
///
/// * a POINTER WITH NO STORAGE — the eight octets are emitted either way;
/// * a STORAGE OF ZERO OCTETS — `ॱरिक्त` is absent from the file, so a block
///   cut to nothing costs nothing to emit;
/// * TWO GLOBALS SHARING ONE STORAGE — the writes of one land in the other,
///   and the object is the same length as if they had two;
/// * A LABEL EXPORTED TWICE — refused at LINK, long after the length is fixed.
///
/// A fifth reading is kept as a finding rather than an assertion: a size that
/// is not a whole number of words. Every run this tree emits is `n × ८`, and
/// nothing in `emit_data` enforces it — the octet count comes from the arena.
fn read(text: &str) -> Findings {
    let mut f = Findings::default();
    let mut labels: BTreeSet<String> = BTreeSet::new();
    let mut stores: BTreeSet<String> = BTreeSet::new();
    for (label, storage) in data_globals(text) {
        if !labels.insert(label.clone()) {
            f.duplicate_label.push(label.clone());
        }
        match storage {
            Storage::Word(_) => f.words += 1,
            Storage::DanglingPointer(store) => {
                f.dangling.push(format!("{label} -> {store}"));
            }
            Storage::Run { store, octets } => {
                f.runs += 1;
                f.octets += octets;
                f.sizes.insert(octets);
                if octets == 0 {
                    f.zero_sized.push(format!("{label} -> {store}"));
                }
                if octets % 8 != 0 {
                    f.unaligned.push(format!("{label} -> {store} ({octets})"));
                }
                if !stores.insert(store.clone()) {
                    f.shared_storage.push(format!("{label} -> {store}"));
                }
            }
        }
    }
    f
}

/// **THE SWEEP.** Every object the corpus emits, against the four properties no
/// object LENGTH can see.
///
/// ~55 s on an idle volume, one fresh interpreter per source. That is the cost
/// of the answer and it is stated rather than hidden: W-282's ~535 s for ONE
/// source was ruled against, and this is the whole corpus for a tenth of it.
#[test]
fn no_object_the_corpus_emits_points_at_storage_it_never_places() {
    let paths = corpus_paths();
    println!("METRIC t1_corpus_globals_sources {}", paths.len());

    let mut no_module: Vec<String> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    let mut objects = 0usize;
    let mut total = Findings::default();
    // NAMED, NOT COUNTED. A sweep that answered "4 defects" would send the next
    // reader back to the corpus to find out which; every finding carries its
    // source, its global and the storage it names.
    let mut dangling: Vec<String> = Vec::new();
    let mut zero_sized: Vec<String> = Vec::new();
    let mut shared: Vec<String> = Vec::new();
    let mut duplicate: Vec<String> = Vec::new();
    let mut unaligned: Vec<String> = Vec::new();
    let mut disagreed: Vec<String> = Vec::new();

    for p in &paths {
        let file = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match compile(p) {
            Object::DeclaresNoModule => {
                println!("  {file:22} declares no मण्डलम्");
                no_module.push(file);
            }
            Object::Refused(why) => {
                println!("  {file:22} REFUSED {why}");
                refused.push(format!("{file}: {why}"));
            }
            Object::Emitted(text) => {
                objects += 1;
                let f = read(&text);
                // THE TWO INSTRUMENTS, COMPARED PER OBJECT. Per object and not
                // in total, because two errors of opposite sign across twenty
                // objects cancel in a sum and a sum is the one place a reader
                // would not look.
                let rikta = rikta_openings(&text);
                if rikta != f.runs {
                    disagreed.push(format!(
                        "{file}: data_globals says {} run(s), `ॱरिक्त` openings say {rikta}",
                        f.runs
                    ));
                }
                println!(
                    "  {file:22} globals {:<4} words {:<4} runs {:<4} octets {:<7} ॱरिक्त {rikta}",
                    f.words + f.runs + f.dangling.len(),
                    f.words,
                    f.runs,
                    f.octets
                );
                for (bucket, found) in [
                    (&mut dangling, &f.dangling),
                    (&mut zero_sized, &f.zero_sized),
                    (&mut shared, &f.shared_storage),
                    (&mut duplicate, &f.duplicate_label),
                    (&mut unaligned, &f.unaligned),
                ] {
                    bucket.extend(found.iter().map(|d| format!("{file}: {d}")));
                }
                total.words += f.words;
                total.runs += f.runs;
                total.octets += f.octets;
                total.sizes.extend(f.sizes);
            }
        }
    }

    let globals = total.words + total.runs + dangling.len();
    println!("METRIC t1_corpus_globals_objects {objects}");
    println!("METRIC t1_corpus_globals_declared {globals}");
    println!("METRIC t1_corpus_globals_words {}", total.words);
    println!("METRIC t1_corpus_globals_runs {}", total.runs);
    println!("METRIC t1_corpus_globals_run_octets {}", total.octets);
    println!(
        "METRIC t1_corpus_globals_distinct_run_sizes {}",
        total.sizes.len()
    );
    println!("NOTE  run sizes measured: {:?}", total.sizes);
    println!(
        "NOTE  these six figures are REPORT-ONLY (owner ruling 2026-09-13, point 1). \
         `riscv64.rs:1427` says 163 globals at 1 KiB EACH; measured here it is {} runs \
         at {} distinct sizes — 1024 is `खण्डसामर्थ्यम् गुणनम् ८` (ir.t1:999, :4492) and \
         40 is a RECORD's five fields (encode.t1:1078, yantrotsarjana.t1:621).",
        total.runs,
        total.sizes.len()
    );

    // ── THE HARNESS FIRST. Every property below is vacuously true of an empty
    // reading, so the sweep must be shown to have READ something before any of
    // them is believed. This is the `repeats(&[])` lesson from
    // `t1_emitted_labels.rs` at corpus scale.
    assert!(
        refused.is_empty(),
        "every source that declares a module must reach the emitter; these did \
         not, so the properties below are silent about them:\n  {}",
        refused.join("\n  ")
    );
    assert_eq!(
        no_module,
        vec!["lib.t1".to_string()],
        "`lib.t1` is the ONE source that declares no `मण्डलम्` — its own margin \
         says why (Rust's crate root lists modules; T1 has no crate root). A \
         second such source is a source this sweep silently skipped."
    );
    assert_eq!(
        objects + no_module.len(),
        paths.len(),
        "every source is accounted for in exactly one state"
    );
    assert!(
        total.runs > 0 && total.words > 0,
        "the corpus emits both shapes of global; {} run(s) and {} word(s) means \
         the READER broke, not that the compiler stopped declaring storage",
        total.runs,
        total.words
    );
    assert!(
        disagreed.is_empty(),
        "THE TWO INSTRUMENTS DISAGREE, which means one of them is wrong and \
         neither says which:\n  {}",
        disagreed.join("\n  ")
    );

    // ── AND THE FOUR PROPERTIES, EACH NAMED BY SOURCE AND BY GLOBAL.
    assert!(
        dangling.is_empty(),
        "A POINTER WITH NO STORAGE. The object's LENGTH DOES NOT MOVE when this \
         happens — the pointer is eight octets either way and `ॱरिक्त` is absent \
         from the file — so no rung on the `स्वपरीक्षा` ladder can see it and \
         every read of the global resolves through a null base:\n  {}",
        dangling.join("\n  ")
    );
    assert!(
        zero_sized.is_empty(),
        "A STORAGE OF ZERO OCTETS. Present, so a check that asked only \"is it \
         there?\" scores it green while the first element written lands in \
         whatever follows:\n  {}",
        zero_sized.join("\n  ")
    );
    assert!(
        shared.is_empty(),
        "TWO GLOBALS POINTING AT ONE STORAGE. Every write through one is a write \
         through the other, and the object is exactly as long as if they had two \
         blocks:\n  {}",
        shared.join("\n  ")
    );
    assert!(
        duplicate.is_empty(),
        "A LABEL EXPORTED TWICE. The linker refuses it by name, long after the \
         octet count is fixed:\n  {}",
        duplicate.join("\n  ")
    );
    assert!(
        unaligned.is_empty(),
        "A RUN WHOSE SIZE IS NOT A WHOLE NUMBER OF WORDS. Every element the \
         language stores in one of these is eight octets wide; nothing in \
         `emit_data` enforces it, because the size comes from the arena:\n  {}",
        unaligned.join("\n  ")
    );
}

/// **THE CASES THAT MUST STILL BE REFUSED, AND THERE ARE FIVE BECAUSE THE SWEEP
/// MAKES FIVE CLAIMS.**
///
/// All five are made BY HAND on the text a real corpus source really emitted —
/// `ashtaka.t1`, three run globals and three word globals, the smallest object
/// that carries both shapes. A sweep that answered "no defects" whatever the
/// text said passes the test above and fails every assertion here.
///
/// `ashtaka.t1` rather than a fixture: the test above makes a statement about
/// OBJECTS THE CORPUS EMITS, so its control must be one of them. A hand-written
/// object would leave the controls green while the reader was wrong about
/// anything only the real emitter does.
#[test]
fn each_defect_is_named_on_a_real_corpus_object_rather_than_counted() {
    let path = corpus_dir().join("ashtaka.t1");
    let Object::Emitted(text) = compile(&path) else {
        panic!("ashtaka.t1 must emit; the control has to itself be real");
    };

    // THE CONTROL IS CLEAN FIRST, or every mutation below proves nothing.
    let clean = read(&text);
    assert_eq!(
        (clean.words, clean.runs),
        (3, 3),
        "ashtaka.t1 emits three word globals and three run globals — this is the \
         one reading in this file that is a PIN, and it is here because a control \
         whose shape changed silently is not a control"
    );
    assert!(clean.dangling.is_empty() && clean.zero_sized.is_empty());
    assert!(clean.shared_storage.is_empty() && clean.duplicate_label.is_empty());
    assert_eq!(rikta_openings(&text), clean.runs);

    // The first run global's own labels, taken from the text rather than written
    // here, so this control does not depend on which global `ashtaka` declares
    // first.
    let (label, store) = data_globals(&text)
        .into_iter()
        .find_map(|(l, s)| match s {
            Storage::Run { store, .. } => Some((l, store)),
            _ => None,
        })
        .expect("ashtaka.t1 declares a run global");

    // ── ONE: THE STORAGE IS NEVER PLACED.
    let storage_line = format!("{store}ॱॱ");
    let dangling: String = text
        .lines()
        .filter(|l| **l != storage_line)
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(dangling, text, "the mutation must change the text");
    let f = read(&dangling);
    assert_eq!(
        f.dangling,
        vec![format!("{label} -> {store}")],
        "the pointer with no storage is named by its global AND the storage it \
         names, not counted"
    );
    assert_eq!(f.runs, 2, "and it stops being counted as a run");
    // AND THE OBJECT IS THE SAME OBJECT EXCEPT FOR ONE LABEL LINE — which is the
    // claim the whole file rests on, so it is measured rather than repeated:
    // `ॱरिक्त` holds no octets in the file, so nothing an image SIZE reads moved.
    assert_eq!(
        rikta_openings(&dangling),
        3,
        "the section is still opened three times — THE SECOND INSTRUMENT IS WHAT \
         CATCHES THIS, and a sweep with only one reader would report two runs and \
         no defect"
    );

    // ── TWO: THE STORAGE IS PLACED AND EMPTY.
    let zeroed = text.replace(
        &format!("{storage_line}\n॥ स्थानम् १०२४ ॥"),
        &format!("{storage_line}\n॥ स्थानम् ० ॥"),
    );
    assert_ne!(zeroed, text, "the mutation must change the text");
    let f = read(&zeroed);
    assert_eq!(f.zero_sized, vec![format!("{label} -> {store}")]);
    assert_eq!(
        f.runs, 3,
        "it is still a run — reported by its SIZE and not by its absence"
    );

    // ── THREE: A SIZE THAT IS NOT A WHOLE NUMBER OF WORDS.
    let odd = text.replace(
        &format!("{storage_line}\n॥ स्थानम् १०२४ ॥"),
        &format!("{storage_line}\n॥ स्थानम् १०२३ ॥"),
    );
    assert_ne!(odd, text, "the mutation must change the text");
    assert_eq!(
        read(&odd).unaligned,
        vec![format!("{label} -> {store} (1023)")],
        "the size is read as a NUMBER, so one octet short of a word is named"
    );

    // ── FOUR: TWO GLOBALS POINTING AT ONE STORAGE. Take the SECOND run global's
    // data word and aim it at the first's block.
    let second = data_globals(&text)
        .into_iter()
        .filter_map(|(l, s)| match s {
            Storage::Run { store, .. } if l != label => Some((l, store)),
            _ => None,
        })
        .next()
        .expect("ashtaka.t1 declares a second run global");
    let aliased = text.replace(
        &format!("॥ अष्टाष्टकाः {} ॥", second.1),
        &format!("॥ अष्टाष्टकाः {store} ॥"),
    );
    assert_ne!(aliased, text, "the mutation must change the text");
    assert_eq!(
        read(&aliased).shared_storage,
        vec![format!("{} -> {store}", second.0)],
        "the second global to name a storage is the one reported, by the name it \
         declared and the block it reached"
    );

    // ── FIVE: A LABEL EXPORTED TWICE. Rename the second run global to the first.
    let duplicated = text.replace(
        &format!("॥ वैश्विकम् {} ॥\n{}ॱॱ", second.0, second.0),
        &format!("॥ वैश्विकम् {label} ॥\n{label}ॱॱ"),
    );
    assert_ne!(duplicated, text, "the mutation must change the text");
    assert_eq!(
        read(&duplicated).duplicate_label,
        vec![label.clone()],
        "a label the object exports twice is named once, by the label"
    );

    // ── AND THE EMPTY READING IS NOT A PASS. Strip every `॥ वैश्विकम् ॥` line and
    // the sweep finds nothing — all five properties hold VACUOUSLY, which is why
    // the test above asserts the harness before it asserts a property.
    let stripped: String = text
        .lines()
        .filter(|l| !l.starts_with("॥ वैश्विकम् "))
        .collect::<Vec<_>>()
        .join("\n");
    let f = read(&stripped);
    assert_eq!((f.words, f.runs), (0, 0), "nothing is read");
    assert!(
        f.dangling.is_empty() && f.zero_sized.is_empty() && f.shared_storage.is_empty(),
        "AND EVERY DEFECT LIST IS EMPTY. An empty reading satisfies every \
         property this file asserts; the `ॱरिक्त` count is still {} and that \
         disagreement is what names it.",
        rikta_openings(&stripped)
    );
    assert_eq!(
        rikta_openings(&stripped),
        3,
        "the second instrument still sees three runs, so the two disagree and \
         the sweep reds"
    );
}
