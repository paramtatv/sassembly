//! **साधनम्** — assemble Devanagari source into a bootable image.
//!
//! ```text
//! sadhana [-g] <source.sas>... <out.elf>
//! ```
//!
//! The whole toolchain in one command, because doc 17 names साधनम् as one
//! instrument rather than an assembler and a linker that happen to ship
//! together. Several sources link into one image (`B-014`), and there is no GNU
//! `ld` step: `kosha` writes the ELF itself (`B-010`).

use std::process::ExitCode;

use sadhana::encode::Target;
use sadhana::nidana::{Language, Script, transliterate};

/// Everything this binary writes goes through here, so the script fallback
/// covers all of it.
///
/// A mechanism wired into fourteen of fifteen sites is the shape `B-078`
/// shipped twice: half a migration reads as a finished one, because the sites
/// that were converted all work. There is exactly one `eprintln!` in this file
/// and a test says so.
macro_rules! say {
    ($script:expr, $($arg:tt)*) => {
        eprintln!("{}", transliterate(&format!($($arg)*), $script))
    };
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // Resolved before the first line is written, including the usage message:
    // a terminal that cannot render Devanagari cannot render it in an error
    // about arguments either. `LC_ALL` outranks `LC_CTYPE` outranks `LANG`,
    // which is the order every other tool on the system reads them in.
    // The compiler speaks Sanskrit unless asked otherwise (`B-015`).
    let lang = Language::from_env(std::env::var("SANSOS_LANG").ok().as_deref());
    let script = Script::from_env(
        std::env::var("SANSOS_SCRIPT").ok().as_deref(),
        ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .find_map(|k| std::env::var(k).ok())
            .as_deref(),
    );
    // `-g` is opt-in because debug info is mass: doc 18 §0.2 says removable
    // mass is removed rather than budgeted, and a build that does not ask for
    // a line table pays nothing for one — not even an empty section header.
    let debug = args.iter().any(|a| a == "-g");
    args.retain(|a| a != "-g");
    // `संक्षिप्त` — "abridged". The target, as GNU spells it `-march=rv64gc`:
    // a compressed form is chosen wherever the assembler would choose one.
    // Off by default, because every corpus but the rvc one expects the wide
    // encoding and turning it on changes every image (`B-058b2b6`).
    let target = if args.iter().any(|a| a == "--संक्षिप्त") {
        Target::Compressed
    } else {
        Target::Uncompressed
    };
    args.retain(|a| a != "--संक्षिप्त");
    // `वस्तु` — "object". Assemble one file to `ET_REL` and leave every name it
    // does not define for a linker, instead of demanding the whole program at
    // once (`B-069b2`).
    let object = args.iter().any(|a| a == "--वस्तु");
    args.retain(|a| a != "--वस्तु");
    // `संयोजय` — "join". Link objects that were assembled separately, with no
    // source in the path. `--वस्तु` writes them; this reads them (`B-069d2c`).
    let joining = args.iter().any(|a| a == "--संयोजय");
    args.retain(|a| a != "--संयोजय");
    // `स्थान` — "address", the lexicon's own word, as GNU spells it `-Ttext`.
    // Where the image is loaded (`C-001a1`). The default is the QEMU `virt`
    // reset vector; booting under OpenSBI needs 0x80200000, because the
    // firmware is already sitting at the default.
    //
    // The value is written in the language — `०षोड्८०२०००००` — rather than in C
    // hexadecimal. `numeral::value` is the same reader the assembler uses for
    // an immediate, so there is one way to write a number here and not two.
    let mut load = sadhana::kosha::LOAD_ADDRESS;
    if let Some(i) = args.iter().position(|a| a == "--स्थान") {
        let Some(v) = args
            .get(i + 1)
            .and_then(|a| sanskrit_text::numeral::value(a).ok())
        else {
            say!(script, "--स्थान wants an address, e.g. ०षोड्८०२०००००");
            return ExitCode::from(2);
        };
        // An unaligned entry point is not a placement choice, it is a machine
        // that traps on its first instruction.
        if v % 4 != 0 {
            say!(script, "--स्थान {v:#x} is not four-byte aligned");
            return ExitCode::from(2);
        }
        load = v;
        args.drain(i..=i + 1);
    }
    if args.len() < 2 {
        say!(
            script,
            "usage: sadhana [-g] [--संक्षिप्त] [--वस्तु] [--स्थान <पता>] <source.sas>... <out.elf>\n\
             \x20      sadhana --संयोजय <object.o>... <out.elf>"
        );
        return ExitCode::from(2);
    }
    let (inputs, output) = args.split_at(args.len() - 1);
    let output = &output[0];

    if joining {
        let mut objects = Vec::with_capacity(inputs.len());
        for input in inputs {
            let bytes = match std::fs::read(input) {
                Ok(b) => b,
                Err(e) => {
                    say!(script, "{input}: {e}");
                    return ExitCode::from(2);
                }
            };
            match sadhana::vastu::read(&bytes) {
                Some(o) => objects.push(o),
                None => {
                    say!(script, "{input}: not a relocatable object");
                    return ExitCode::from(2);
                }
            }
        }
        let linked = match sadhana::samyojana::link_at(&objects, load) {
            Ok(l) => l,
            Err(errors) => {
                for e in &errors {
                    say!(script, "{e}");
                }
                return ExitCode::FAILURE;
            }
        };
        // The reservation travels: an image whose `.bss` was dropped runs with
        // its buffer over whatever follows (`B-099`).
        // The names travel too (`B-103`): `nm` on an image linked from objects
        // reported nothing while a single-file build of the same program showed
        // `मुख्यम्`.
        // The debug sections travel too (`B-104`). `write_debuggable` takes
        // borrowed names, and the linker owns them, so they are borrowed here.
        let debug: Vec<(&str, Vec<u8>)> = linked
            .debug
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.clone()))
            .collect();
        let bytes = sadhana::kosha::write_debuggable_at(
            &linked.text,
            &linked.data,
            &linked.table,
            linked.bss,
            &debug,
            load,
        );
        if let Err(e) = std::fs::write(output, bytes) {
            say!(script, "{output}: {e}");
            return ExitCode::from(2);
        }
        say!(
            script,
            "{output}: {} object(s), {} text bytes, {} data bytes, {} symbols, entry {:#x}",
            inputs.len(),
            linked.text.len(),
            linked.data.len(),
            // The table, not the map: the map holds one entry per NAME and two
            // objects may each define a local `चक्रः`, so it undercounts.
            linked.table.len(),
            load
        );
        return ExitCode::SUCCESS;
    }

    let mut programs = Vec::new();
    let mut sources = Vec::new();
    let mut failed = false;

    // Single source fast-path using the top-level assemble function
    if inputs.len() == 1 && !object && !joining && !debug {
        let input = &inputs[0];
        let source = match std::fs::read_to_string(input) {
            Ok(s) => s,
            Err(e) => {
                say!(script, "{input}: {e}");
                return ExitCode::from(2);
            }
        };

        match sadhana::assemble(&source, target, load, lang) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(output, bytes) {
                    say!(script, "{output}: {e}");
                    return ExitCode::from(2);
                }
                say!(script, "{output}: 1 file(s) assembled, entry {:#x}", load);
                return ExitCode::SUCCESS;
            }
            Err(errors) => {
                for e in &errors {
                    say!(script, "{input}: line {}: {}", e.line, e.reason);
                }
                return ExitCode::FAILURE;
            }
        }
    }

    // Otherwise, fallback to the object/linking logic for multi-file operations
    for input in inputs {
        match std::fs::read_to_string(input) {
            Ok(s) => sources.push((input.clone(), s)),
            Err(e) => {
                say!(script, "{input}: {e}");
                return ExitCode::from(2);
            }
        }
    }

    for (name, source) in &sources {
        match sadhana::parse::assemble_program(source) {
            Ok(p) => programs.push((name.clone(), p)),
            Err(errors) => {
                // Every diagnostic, not just the first: a file pasted from a
                // Latin source has hundreds, and reporting them one build at a
                // time is useless.
                for e in &errors {
                    say!(script, "{name}: {e}");
                }
                failed = true;
            }
        }
    }
    if failed {
        return ExitCode::FAILURE;
    }

    if object {
        if programs.len() != 1 {
            say!(
                script,
                "--वस्तु assembles one file at a time; that is what an object is for"
            );
            return ExitCode::from(2);
        }
        let (name, program) = &programs[0];
        let (text, pending) = match sadhana::encode::encode_object_for(program, target) {
            Ok(v) => v,
            Err(errors) => {
                for e in &errors {
                    say!(script, "{name}: {e}");
                }
                return ExitCode::FAILURE;
            }
        };
        // `-g` reaches an object now (`B-100b`): `.debug_line` and
        // `.debug_info` with addresses relative to `.text`, and the records
        // that fix them once the linker chooses one.
        let bytes = sadhana::kosha::object(
            &text,
            program,
            &pending,
            debug.then_some(name.as_str()),
            &sadhana::encode::layout_addresses(program, target),
        );
        if let Err(e) = std::fs::write(output, bytes) {
            say!(script, "{output}: {e}");
            return ExitCode::from(2);
        }
        say!(
            script,
            "{output}: object, {} text bytes, {} relocation(s)",
            text.len(),
            pending.len()
        );
        return ExitCode::SUCCESS;
    }

    // Every source becomes an object, and the objects are linked — the same
    // two steps `--वस्तु` and `--संयोजय` take, run in memory (`B-096`).
    //
    // `बन्धकः` linked by re-encoding every unit from its parse tree, which is
    // why `B-014`'s promise of separate compilation went unkept for a hundred
    // cycles: a build that needs the source at link time has separated nothing.
    // Routing the ordinary build through the object path means the path a
    // program takes is the path that was tested against GNU's tools, rather
    // than a second implementation of the same job.
    //
    // One difference, and it is real: `बन्धकः` iterated the relaxation fixpoint
    // across every unit together, so a branch in one file to a label in another
    // could compress. Objects are compressed one at a time and nothing relaxes
    // afterwards (`B-069e`), so a cross-file branch stays wide. Larger, never
    // wrong, and the alternative is a linker that relaxes.
    let mut objects = Vec::with_capacity(programs.len());
    let mut failed = false;
    for (name, program) in &programs {
        let (text, pending) = match sadhana::encode::encode_object_for(program, target) {
            Ok(v) => v,
            Err(errors) => {
                for e in &errors {
                    say!(script, "{name}: line {}: {}", e.line, e.message(lang));
                }
                failed = true;
                continue;
            }
        };
        let bytes = sadhana::kosha::object(
            &text,
            program,
            &pending,
            debug.then_some(name.as_str()),
            &sadhana::encode::layout_addresses(program, target),
        );
        match sadhana::vastu::read(&bytes) {
            Some(o) => objects.push(o),
            None => {
                say!(
                    script,
                    "{name}: the object this build wrote does not read back"
                );
                failed = true;
            }
        }
    }
    if failed {
        return ExitCode::FAILURE;
    }

    let image = match sadhana::samyojana::link_at(&objects, load) {
        Ok(i) => i,
        Err(errors) => {
            for e in &errors {
                say!(script, "{e}");
            }
            return ExitCode::FAILURE;
        }
    };

    let debug: Vec<(&str, Vec<u8>)> = image
        .debug
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.clone()))
        .collect();

    let bytes = sadhana::kosha::write_debuggable_at(
        &image.text,
        &image.data,
        &image.table,
        image.bss,
        &debug,
        load,
    );
    if let Err(e) = std::fs::write(output, bytes) {
        say!(script, "{output}: {e}");
        return ExitCode::from(2);
    }
    say!(
        script,
        "{output}: {} file(s), {} text bytes, {} data bytes, {} reserved, {} symbols, entry {:#x}",
        inputs.len(),
        // Bytes, not instructions: at the compressed target they are not the
        // same number, and dividing by four reported twelve for a thirteen
        // instruction program (`B-058b2b6`).
        image.text.len(),
        image.data.len(),
        image.bss,
        image.table.len(),
        load
    );
    ExitCode::SUCCESS
}
