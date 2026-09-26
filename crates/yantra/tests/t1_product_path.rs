//! **THE PRODUCT PATH, PINNED — `W-290`.**
//!
//! `t1_build` is the binary that must work when Rust is discarded. Until this
//! file, **nothing in the tree ran it across the corpus**: four test files
//! mention `t1_build` and the only reference to the binary was inside a doc
//! comment. Every gate went through a harness — `chain_source`, `Front`, the
//! census — and each of those carries machinery the shipped path does not.
//!
//! # Why a harness figure is not a product figure
//!
//! The encode census reports `paradigm_encode_t1_assembled 17` and
//! `paradigm_boundary_t1_runnable_on_yantra 11`. Both are true **about the
//! census**, which resolves an image with its OWN closure walk over `others`.
//! `t1_build` links exactly two objects — the startup and the module — with no
//! closure and no synthetic name table. **Twelve sources the census carries to
//! RUN or to link cannot be linked by the binary that ships.**
//!
//! > **AN INSTRUMENT'S VERDICT MUST NAME ITS PATH.** Three instruments gave
//! > three true, non-contradicting verdicts on `ir.t1` on 2026-09-12:
//! > `t1_build` FAILS AT LINK, the encode census reaches RUN and faults at
//! > `addr 8`, and the twin census AGREES on octets. **A source does not have a
//! > state; it has a state per path.**
//!
//! # What this pins and why the MEMBERS and not the count
//!
//! Four sources link: `ashtaka`, `lex`, `sanchaya`, `utsarjana`. The
//! self-hosting ladder tracks 4 of 19 after withdrawing `ast` and `vastu` as
//! degenerate — and **its four are these four**, by a different route and a
//! different failure mode. Pinning the SET rather than the size is what makes a
//! swap visible: a count of four survives one source regressing while another
//! starts working, and that is exactly the movement worth seeing.
//!
//! # What this does NOT cover
//!
//! **It links; it does not run.** A linked ELF is not a working program, and
//! this file asserts nothing about execution — `t1_storage_witness` and the
//! ladder do that. It also says nothing about WHY the twelve fail; that cause
//! is recorded in `.loop/STATE.md` as the `२०००००० ` synthetic band minted by
//! two independent counters, and it is not this file's business to diagnose.
//!
//! # What happens if it cannot run
//!
//! It is `#[ignore]`d, so a whole-crate run reports green without reaching it,
//! and `cargo test -- --ignored <name>` that matches nothing also exits 0 — the
//! mechanism that let a census match no test and report success twice in one
//! evening. **A rung whose absence is indistinguishable from its success is not
//! a rung**, so the assertions below name the corpus size and the member set,
//! and a zero anywhere fails loudly rather than reading as a clean corpus.

use sadhana::encode::Target;
use sadhana::kosha::LOAD_ADDRESS;
use sadhana::nidana::Language;
use sadhana::t1::chain::{self, Front};
use sadhana::t1::riscv64;
use sadhana::{assemble_object, link_objects, vastu};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

/// Exactly what `t1_build`'s `build()` does, minus the reporting and the write:
/// load the `.t1` front end, lower, emit, assemble, link TWO objects. **No
/// closure walk and no synthetic name table** — that is the whole difference
/// from the census, and the reason this file exists.
fn links(name: &str, src: &str) -> Result<usize, String> {
    let mut front = Front::load(&spec_root()).map_err(|e| format!("front end: {e}"))?;
    front.lex(src).map_err(|e| format!("lex: {e}"))?;
    front.parse().map_err(|e| format!("parse: {e}"))?;
    front.resolve().map_err(|e| format!("resolve: {e}"))?;
    front.typecheck().map_err(|e| format!("typecheck: {e}"))?;
    front.build_ir().map_err(|e| format!("IR: {e}"))?;
    // THE NAME COMES FROM THE SOURCE, NOT THE FILENAME, AND THE FIRST VERSION OF
    // THIS FILE GOT IT WRONG. `t1_build:160` reads `chain::module_name(&source)`
    // and falls back to the file stem only when the source declares no module.
    // Using the stem gave every label a Latin prefix — `ashtakaयोगः…` instead of
    // `अष्टकयोगः…` — and the assembler refused all twenty sources with hundreds
    // of diagnostics each. **The gate reported 0 linked where the binary linked
    // 4, and the registered prediction is the only reason that read as a defect
    // in the instrument rather than a finding about the compiler.**
    let module_name =
        chain::module_name(src).unwrap_or_else(|| name.trim_end_matches(".t1").to_string());
    let module = front
        .module(&module_name, None)
        .map_err(|e| format!("IR: {e}"))?;
    let entry_label = module
        .entry
        .map(|sym| riscv64::routine_label(&module.names, sym))
        .transpose()
        .map_err(|r| format!("emit: {r}"))?;
    let text = riscv64::emit_module(&module).map_err(|r| format!("emit: {r}"))?;
    let startup_text = riscv64::emit_startup_object_with_records(
        entry_label.as_deref(),
        riscv64::module_allocates(&module),
    );
    let asm = |t: &str, n: &str| -> Result<Vec<u8>, String> {
        assemble_object(t, Some(n), Target::Uncompressed, false, Language::English)
            .map_err(|ds| format!("assemble {n}: {} diagnostic(s)", ds.len()))
    };
    let module_object = asm(&text, &module_name)?;
    let startup_object = asm(&startup_text, "यन्त्रारम्भ")?;
    let read = |b: &[u8], what: &str| {
        vastu::read(b).ok_or_else(|| format!("link: the {what} object does not read back"))
    };
    let objects = [
        read(&startup_object, "startup")?,
        read(&module_object, "module")?,
    ];
    link_objects(&objects, LOAD_ADDRESS)
        .map(|elf| elf.len())
        .map_err(|es| format!("link: {}", es.join("; ")))
}

#[test]
#[ignore = "measurement: the whole corpus through the product path, minutes"]
fn the_product_path_links_exactly_the_four_the_ladder_tracks() {
    let dir = corpus_dir();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();

    let mut linked: BTreeSet<String> = BTreeSet::new();
    for n in &names {
        let Ok(src) = std::fs::read_to_string(dir.join(n)) else {
            continue;
        };
        match links(n, &src) {
            Ok(bytes) => {
                println!("  {n:24} LINKED {bytes} octets");
                linked.insert(n.clone());
            }
            // BY CHARS, NEVER BY BYTES. `&why[..96]` panicked inside `र` — the
            // refusals are Devanagari and a byte slice lands mid-codepoint. The
            // panic killed the loop before ANY metric printed, so a run that had
            // already produced the answer reported nothing at all.
            Err(why) => {
                let brief: String = why.chars().take(96).collect();
                println!("  {n:24} {brief}");
            }
        }
    }
    println!("METRIC t1_product_path_sources {}", names.len());
    println!("METRIC t1_product_path_linked {}", linked.len());
    for n in &linked {
        println!("  linked: {n}");
    }

    // THE CORPUS SIZE FIRST — without it a directory that reads empty would
    // make the member check below vacuously true in the direction of success.
    assert!(
        names.len() >= 19,
        "only {} corpus sources found; this census is broken, not the compiler",
        names.len()
    );

    // THE MEMBERS, NOT THE COUNT. A size of four survives one source regressing
    // while another starts working — the exact movement worth seeing.
    let want: BTreeSet<String> = ["ashtaka.t1", "lex.t1", "sanchaya.t1", "utsarjana.t1"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        linked, want,
        "the product path's member set moved. This is the figure the goal is \
         about — `t1_build` is what must work when Rust is discarded — so a \
         change here is a real result in either direction and the pin is what \
         makes it visible. Gained sources are progress and belong in the pin; \
         lost ones are a regression in the path that ships."
    );
}
