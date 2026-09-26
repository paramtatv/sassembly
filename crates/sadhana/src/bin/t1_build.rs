//! ONE SOURCE, ONE COMMAND: compile a `.t1` file and get an artefact.
//!
//! **Why this binary exists.** Every path from a `.t1` source to a running
//! image lived inside a test — `crates/yantra/tests/paradigm_encode.rs` drives
//! the whole chain and asserts about it, and `t1_parse` stops at the parse. So
//! the project could MEASURE that the chain worked and could not USE it: there
//! was no program a person could run on a file they had just written. This is
//! that program, and it is deliberately the smallest one that produces an
//! artefact:
//!
//! ```text
//! t1_build --spec-root spec प्रोग्.t1 -o प्रोग्.elf   # lex … link, an ELF on disk
//! yantra-run प्रोग्.elf                               # the answer, as the exit status
//! ```
//!
//! **The chain it drives.** The front end is the `.t1` one, under the
//! interpreter (`t1::chain`), because the Rust twins of parse/resolve/check/IR
//! are a skeleton — `ast::Statement` has no return statement, so the Rust chain
//! compiles `प्रत्यागमनम् २ योगः ३ ।` to an empty routine (measured). The back
//! end is the RUST emitter, `t1::riscv64`, whose T1 twin agrees with it octet
//! for octet (`W-236`), then this crate's assembler and linker.
//!
//! **What it is not.** It compiles ONE source. It does not read a module's
//! imports, does not collect a compilation set, and does not link another
//! module's object — a call to a routine this file does not declare reaches the
//! linker as an undefined name and is refused there, BY NAME, which is the
//! honest stop and not a silent zero. Those are the inter-module rows
//! (`W-243`'s exported labels made the image the unit; the collection question
//! is open), and a driver that pretended to answer them would hide which of
//! them is unbuilt.
//!
//! **Every stage prints, and a stop prints by name.** lex, parse, resolve,
//! typecheck, IR, emit, assemble, link, write. A stage that refuses ends the run
//! with `STOPPED AT <stage>` and the reason the stage itself gave — never a
//! summary of it. That is what makes this usable as a demonstration: when it
//! stops, the output says which of nine stages, and why.

use sadhana::encode::Target;
use sadhana::kosha::LOAD_ADDRESS;
use sadhana::nidana::Language;
use sadhana::t1::chain::{self, Front};
use sadhana::t1::riscv64;
use sadhana::{assemble_object, link_objects, vastu};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Where a stage stopped, and what the stage said.
struct Stop {
    stage: &'static str,
    why: String,
}

impl Stop {
    fn at(stage: &'static str, why: impl std::fmt::Display) -> Self {
        Self {
            stage,
            why: why.to_string(),
        }
    }
}

/// One line per stage, so a run reads as the chain it is.
fn done(stage: &str, detail: impl std::fmt::Display) {
    println!("  {stage:<9} {detail}");
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: t1_build [--spec-root <dir>] [--entry <routine>] [-o <out.elf>] [--run] <file.t1>

  --spec-root <dir>  the directory holding `spec/`'s tables, which four of the
                     front end's sources embed at load time (default: `spec`;
                     ADR-0024 made this a flag rather than a guess)
  --entry <routine>  the zero-parameter routine whose result is the exit status
                     (default: `मुख्य` if declared, else the first routine that
                     takes no parameters)
  -o <out.elf>       where to write the image (default: the source's stem)
  --run              run the image with `yantra-run` after writing it
  --emit             print the T0 text the emitter wrote, before assembling"
    );
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (mut spec_root, mut entry, mut out, mut run) = (None, None, None, false);
    let mut show_text = false;
    let mut source_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        let value = args.get(i + 1).cloned();
        match args[i].as_str() {
            "--spec-root" => {
                let Some(v) = value else { return usage() };
                spec_root = Some(PathBuf::from(v));
                i += 2;
            }
            "--entry" => {
                let Some(v) = value else { return usage() };
                entry = Some(v);
                i += 2;
            }
            "-o" => {
                let Some(v) = value else { return usage() };
                out = Some(PathBuf::from(v));
                i += 2;
            }
            "--run" => {
                run = true;
                i += 1;
            }
            // The T0 text the emitter wrote, on the way past: what the
            // assembler reads, and the first thing anyone asks for when a
            // program runs and answers wrongly.
            "--emit" => {
                show_text = true;
                i += 1;
            }
            other if source_path.is_none() && !other.starts_with('-') => {
                source_path = Some(other.to_string());
                i += 1;
            }
            _ => return usage(),
        }
    }
    let Some(source_path) = source_path else {
        return usage();
    };
    let source_path = PathBuf::from(source_path);
    let out = out.unwrap_or_else(|| source_path.with_extension("elf"));
    let spec_root = spec_root.unwrap_or_else(|| PathBuf::from("spec"));

    println!("{} -> {}", source_path.display(), out.display());
    match build(&source_path, &spec_root, entry.as_deref(), &out, show_text) {
        Err(stop) => {
            eprintln!("STOPPED AT {}: {}", stop.stage, stop.why);
            ExitCode::FAILURE
        }
        Ok(()) if run => run_image(&out),
        Ok(()) => {
            println!("run it:   yantra-run {}", out.display());
            ExitCode::SUCCESS
        }
    }
}

/// The chain, one source, stage by stage.
fn build(
    source_path: &Path,
    spec_root: &Path,
    entry: Option<&str>,
    out: &Path,
    show_text: bool,
) -> Result<(), Stop> {
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| Stop::at("read", format!("{}: {e}", source_path.display())))?;
    // The name is half of every label; a source that declares no module is
    // given its file's stem, and the line below is the only place that is
    // decided.
    let module_name = chain::module_name(&source).unwrap_or_else(|| {
        source_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "कार्यक्रम".to_string())
    });

    let mut front = Front::load(spec_root).map_err(|e| Stop::at("front end", e))?;
    let tokens = front.lex(&source).map_err(|e| Stop::at("lex", e))?;
    done("lex", format!("{tokens} tokens"));

    let declarations = front.parse().map_err(|e| Stop::at("parse", e))?;
    done(
        "parse",
        format!("{declarations} declarations, module `{module_name}`"),
    );

    front.resolve().map_err(|e| Stop::at("resolve", e))?;
    done("resolve", "every name bound to a symbol");

    front.typecheck().map_err(|e| Stop::at("typecheck", e))?;
    done("typecheck", "every statement typed");

    let routines = front.build_ir().map_err(|e| Stop::at("IR", e))?;
    done("IR", format!("{routines} routine(s)"));

    let module = front
        .module(&module_name, entry)
        .map_err(|e| Stop::at("IR", e))?;
    let entry_label = module
        .entry
        .map(|sym| riscv64::routine_label(&module.names, sym))
        .transpose()
        .map_err(|r| Stop::at("emit", r))?;
    match &entry_label {
        Some(label) => done("entry", format!("`{label}`, its result is the exit status")),
        None => done(
            "entry",
            "none — the image halts 0x5555 without calling anything",
        ),
    }

    // ── emit ─────────────────────────────────────────────────────────────
    let text = riscv64::emit_module(&module).map_err(|r| Stop::at("emit", r))?;
    // `W-243`: the startup stub and the stack it addresses are an object of
    // their own, linked first so `e_entry` is the stub's first instruction.
    let startup_text = riscv64::emit_startup_object_with_records(
        entry_label.as_deref(),
        riscv64::module_allocates(&module),
    );
    done(
        "emit",
        format!(
            "{} instruction line(s) + a startup object",
            text.lines().filter(|l| l.ends_with(" ।")).count()
        ),
    );
    if show_text {
        println!("--- {module_name} ---\n{text}--- startup ---\n{startup_text}");
    }

    // ── assemble ─────────────────────────────────────────────────────────
    let module_object = assemble(&text, &module_name)?;
    let startup_object = assemble(&startup_text, "यन्त्रारम्भ")?;
    done(
        "assemble",
        format!("{} + {} octets", startup_object.len(), module_object.len()),
    );

    // ── link ─────────────────────────────────────────────────────────────
    let read = |bytes: &[u8], what: &'static str| {
        vastu::read(bytes)
            .ok_or_else(|| Stop::at("link", format!("the {what} object does not read back")))
    };
    let objects = [
        read(&startup_object, "startup")?,
        read(&module_object, "module")?,
    ];
    let elf = link_objects(&objects, LOAD_ADDRESS).map_err(|es| Stop::at("link", es.join("; ")))?;
    done("link", format!("{} octets of ELF", elf.len()));

    // ── write ────────────────────────────────────────────────────────────
    std::fs::write(out, &elf).map_err(|e| Stop::at("write", format!("{}: {e}", out.display())))?;
    done("write", out.display());
    Ok(())
}

/// `assemble_object`, with every diagnostic carrying the line of the EMITTED
/// text it refused — what a reader needs to see when the emitter and the
/// assembler disagree, which is not the source's line.
fn assemble(text: &str, name: &str) -> Result<Vec<u8>, Stop> {
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
            .take(3)
            .map(|d| {
                let line = text.lines().nth(d.line.saturating_sub(1)).unwrap_or("");
                format!("emitted line {}: `{line}` — {}", d.line, d.reason)
            })
            .collect();
        Stop::at("assemble", named.join("; "))
    })
}

/// Defer to `yantra-run`, which already runs an ELF and prints what it wrote
/// and how it halted. It lives in a crate that depends on this one, so this
/// cannot call it as a library without a cycle — it is a separate program and
/// is run as one, from beside this binary or from the PATH.
fn run_image(out: &Path) -> ExitCode {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("yantra-run")))
        .filter(|p| p.exists());
    let program = beside.unwrap_or_else(|| PathBuf::from("yantra-run"));
    println!("running:  {} {}", program.display(), out.display());
    match Command::new(&program).arg(out).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!(
                "could not run `{}`: {e}\nthe image is written; run it with `yantra-run {}`",
                program.display(),
                out.display()
            );
            ExitCode::FAILURE
        }
    }
}
