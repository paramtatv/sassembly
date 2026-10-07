//! **`D-003`, THE CROSS-ENVIRONMENT HALF: one process's emitted bytes, written
//! out so a DIFFERENT process can compare them.**
//!
//! `d003_t1_chain_is_hash_order_blind.rs` settles hash order by varying seeds
//! WITHIN one process, and its own header names what that leaves untouched:
//! *"Cross-host — endianness, pointer width, locale, directory order — is
//! untouched and `D-003` stays open for it."* The literal cross-host half has
//! since been measured elsewhere (2026-10-02, three hosts, two instruction
//! sets, identical octets); what this probe measures, repeatably and in-repo,
//! is the per-host environment axes — **two processes that share nothing an
//! emitted octet may lawfully depend on**: different
//! working directory, different `TZ`, different locale, different `HOME`,
//! different umask — and the corpus and `spec/` read from a DIFFERENT ABSOLUTE
//! PATH, which upgrades the row's paths reading (a bounded grep over
//! `env!(CARGO_MANIFEST_DIR)` uses) into a measurement: a path that leaked
//! into text or octets now differs between the passes and the comparison says
//! where.
//!
//! The two processes and the comparison live in
//! `tools/check-d003-cross-env.sh`, because one process cannot vary its own
//! environment honestly — anything captured at startup is already captured.
//! This file is the half a process CAN do: compile every corpus source once,
//! `.t1` → Sassembly text → object octets, and write both per source into
//! `D003_DIGEST_DIR` under names that carry no path, no time and no host.
//!
//! `#[ignore]` because the sweep is ~60 s in release and the in-gate probe
//! already covers hash order; the tool runs it with `--ignored`, twice.

use sadhana::encode::Target;
use sadhana::nidana::Language;
use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};

/// THIS FILE'S OWN LOADER, overridable at RUNTIME: `D003_SPEC_ROOT` points the
/// front end at a copy of `spec/` living at a different absolute path, which
/// is the variation the tool's second pass exists to apply. The sibling file's
/// compile-time `env!` default remains for a bare run.
fn spec_root() -> PathBuf {
    std::env::var_os("D003_SPEC_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec"),
        PathBuf::from,
    )
}

/// The corpus directory, overridable the same way via `D003_CORPUS_DIR`.
fn corpus_dir() -> PathBuf {
    std::env::var_os("D003_CORPUS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        PathBuf::from,
    )
}

/// Every `.t1` the corpus ships, in name order, read off the directory — the
/// claim is about the corpus, not about a hand-written list.
fn corpus_paths() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    assert!(
        v.len() >= 21,
        "the corpus is the input to this probe and it has shrunk to {} sources; \
         a digest over an empty corpus agrees with everything",
        v.len()
    );
    v
}

/// **THE DIGEST WRITER.** Each corpus source, compiled once through the whole
/// T1 chain, its Sassembly text written to `<name>.text` and its object octets
/// to `<name>.obj` in `D003_DIGEST_DIR`; a refusal is written to
/// `<name>.refused` so that a source refused in ONE environment and compiled
/// in the other is a difference the comparison sees, not a file that is
/// quietly absent from one side.
#[test]
#[ignore = "cross-environment digest — run twice via tools/check-d003-cross-env.sh and diff"]
fn the_corpus_digest_is_written_for_cross_process_comparison() {
    let out = PathBuf::from(std::env::var_os("D003_DIGEST_DIR").expect(
        "D003_DIGEST_DIR names where this pass writes its digests; run via \
         tools/check-d003-cross-env.sh, which sets it per pass",
    ));
    std::fs::create_dir_all(&out).expect("the digest directory is creatable");

    let mut emitted = 0usize;
    let mut no_module: Vec<String> = Vec::new();
    let mut refused: Vec<String> = Vec::new();

    for path in corpus_paths() {
        let name = path
            .file_name()
            .expect("a corpus path has a file name")
            .to_string_lossy()
            .into_owned();
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
        let Some(module) = module_name(&src) else {
            no_module.push(name);
            continue;
        };
        let mut front = Front::load(&spec_root()).expect("the front end loads");
        let text = (|| -> Result<String, String> {
            front.lex(&src)?;
            front.parse()?;
            front.resolve()?;
            front.typecheck()?;
            front.build_ir()?;
            let built = front.module(&module, None)?;
            riscv64::emit_module(&built).map_err(|e| format!("{e:?}"))
        })();
        let text = match text {
            Ok(t) => t,
            Err(why) => {
                std::fs::write(out.join(format!("{name}.refused")), &why)
                    .expect("the refusal file is writable");
                refused.push(name);
                continue;
            }
        };
        // `Language::English` and `Uncompressed` for the same reason the
        // sibling probe gives: compression is a second variable and this
        // probe holds one.
        let octets = sadhana::assemble_object(
            &text,
            Some("d003t1"),
            Target::Uncompressed,
            false,
            Language::English,
        )
        .unwrap_or_else(|d| {
            panic!(
                "{name}: emitted text does not assemble: {} diagnostics",
                d.len()
            )
        });
        std::fs::write(out.join(format!("{name}.text")), &text)
            .expect("the text digest is writable");
        std::fs::write(out.join(format!("{name}.obj")), &octets)
            .expect("the object digest is writable");
        emitted += 1;
    }

    // NO SILENT CAP: say what was not covered, by name.
    println!(
        "METRIC d003_env_sources_emitted {emitted}\n\
         METRIC d003_env_declares_no_module {}\n\
         METRIC d003_env_refused {}",
        no_module.len(),
        refused.len()
    );
    for n in &no_module {
        println!("d003 ENV NOT EMITTED (declares no module): {n}");
    }
    for n in &refused {
        println!("d003 ENV REFUSED: {n}");
    }

    // 20 IS MEASURED AND NOT CHOSEN — the sibling probe's floor: 21 sources,
    // `lib.t1` declares no module, zero refusals on the current tree.
    assert!(
        emitted >= 20,
        "only {emitted} sources emitted where the floor is 20; a digest that \
         silently covers less makes the cross-pass agreement a smaller claim \
         than its name"
    );
}
