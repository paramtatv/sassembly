//! `W-302` (option i, part 3) — **ASSEMBLE, DISASSEMBLE, REASSEMBLE.**
//!
//! A compiler verification of the `.t1` chain's encoding tables. For each
//! program the chain's own assembler (`शृङ्खलाॱपाठवस्तुरचना`, then the link
//! and `कोशॱप्रतिबिम्बलेखनम्`) writes an image; every instruction of its
//! linked `.text` is then taken apart by the chain's existing disassembler
//! (`विश्लेषणॱस्थानविश्लेषणम्`: a word to a mnemonic and operand values, read
//! from `spec/encodings-riscv64.tsv`) and put back together by the same
//! module (`विश्लेषणॱपुनःसंयोजनम्`: the mnemonic and values to a word, through
//! the same row); the image is written again from the reassembled text with
//! the same data and reservation, and must be BYTE-IDENTICAL — every octet,
//! so every loaded segment.
//!
//! What it catches: a row whose mask and pattern do not describe the words the
//! encoder writes from it (the word does not decode, or decodes through
//! another row to other bits), a field map that loses or moves a bit, and a
//! reassembly that disagrees with the encoder on any operand. What it cannot
//! catch, said plainly: a row that is wrong in the same way on both sides —
//! that is what the census against Rust (`w302_loaded_identity.rs`) is for.
//!
//! THE POPULATION: the 48 `spec/*.sas` programs (namaste-main, whose callee is
//! in lib-mudraka, as the pair linked together), and the `.sas` the compiler
//! emits for each of its own corpus sources. The emitted text comes from the
//! Rust twin front end and back end (`Front` and `riscv64::emit_module`, as
//! `w366a_sas_lexes_no_string_token.rs` takes it — the interpreted `.t1`
//! front half spends ~4,000 s on the corpus); the chain assembles it to an
//! OBJECT, which cannot link alone (its calls leave the module), so for these
//! the round trip is graded on the object's `.text`, octet for octet.
//!
//! THE CONTROL THAT IT CAN FAIL: the same walk with ONE ROW of the encoding
//! table changed — `addi`'s mask widened by bit १५, the low bit of `rs1`, so
//! the row no longer describes every word written from it — must go red.
//! Measured: on `context` a word written from that row (`0x00028413`, rs1 =
//! x5) no longer decodes; over the 48, 41 go red, some at the decode and some
//! earlier, where the chain's linker reads the same table and refuses.

use sadhana::t1::chain::{CHAIN, Front, module_name};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};

const FUEL: u64 = 4_000_000_000_000;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn le(b: &[u8], at: usize, n: usize) -> u64 {
    b[at..at + n]
        .iter()
        .rev()
        .fold(0u64, |acc, &x| (acc << 8) | u64::from(x))
}

/// One load address per program — the first the check scripts pass.
fn load_of(program: &str) -> u64 {
    match program {
        "bare-metal" | "jump-table" | "lib-mudraka" | "namaste" | "namaste-main" | "atithi" => {
            0x8000_0000
        }
        _ => 0x8020_0000,
    }
}

/// The chain, its tables read from `spec_root`. The interpreter fills the
/// table store from the spec root at load; `सारणीसिद्धम्` is set so that
/// `समावेशसारणीॱसारणीपूरणम्` (called by `सङ्कलनारम्भः`) does not overwrite it
/// with `sarani.t1`'s compiled copy — so the encoder and the disassembler both
/// read THE TABLE IN `spec_root`, and the control below changes exactly one
/// row of it.
fn chain(spec_root: &Path) -> Result<Interpreter, String> {
    let mut it = Interpreter::load(CHAIN, spec_root)
        .map_err(|e| format!("the chain does not load: {e:?}"))?;
    if !it.set_global("सारणीसिद्धम्", Value::Bool(true)) {
        return Err("`सारणीसिद्धम्` is not a global of the chain".into());
    }
    Ok(it)
}

/// The chain's object for a Sassembly text, or why not.
fn object(it: &mut Interpreter, src: &[u8]) -> Result<Value, String> {
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], FUEL)
        .map_err(|e| format!("FAULT in सङ्कलनारम्भः: {e:?}"))?;
    let o = it
        .call(
            "शृङ्खलाॱपाठवस्तुरचना",
            vec![Value::Octets(Octets::new(src))],
            FUEL,
        )
        .map_err(|e| format!("FAULT in पाठवस्तुरचना: {e:?}"))?;
    if o.is_nil() {
        let exit = it
            .global("वस्तुरचनाविरामभेद")
            .and_then(Value::as_int)
            .unwrap_or(-1);
        return Err(format!("REFUSED by पाठवस्तुरचना, exit {exit}"));
    }
    Ok(o)
}

fn field(v: &Value, k: &str) -> Result<Value, String> {
    match v {
        Value::Record(r) => r
            .borrow()
            .get(k)
            .cloned()
            .ok_or_else(|| format!("no field `{k}`")),
        other => Err(format!("not a record: {other:?}")),
    }
}

fn octets_of(v: &Value) -> Result<Vec<u8>, String> {
    v.octets()
        .map(|o| o.as_slice().to_vec())
        .ok_or_else(|| format!("not octets: {v:?}"))
}

/// Every instruction of `text` taken apart and put back together by the
/// chain's disassembler. The reassembled text, or the first instruction that
/// did not survive and why.
fn disassemble_reassemble(it: &mut Interpreter, text: &[u8]) -> Result<Vec<u8>, String> {
    let all = Value::Octets(Octets::new(text));
    let mut out = Vec::with_capacity(text.len());
    let mut at = 0usize;
    while at < text.len() {
        if text[at] & 3 != 3 {
            return Err(format!(
                "a 16-bit instruction at {at:#x}; the chain assembles the uncompressed target"
            ));
        }
        if at + 4 > text.len() {
            return Err(format!("a truncated word at {at:#x}"));
        }
        let word = le(text, at, 4);
        let d = it
            .call(
                "विश्लेषणॱस्थानविश्लेषणम्",
                vec![all.clone(), Value::Int(at as i128)],
                FUEL,
            )
            .map_err(|e| format!("FAULT decoding {word:#010x} at {at:#x}: {e:?}"))?;
        if d.is_nil() {
            return Err(format!("{word:#010x} at {at:#x} does not decode"));
        }
        let w = it
            .call("विश्लेषणॱपुनःसंयोजनम्", vec![d.clone()], FUEL)
            .map_err(|e| format!("FAULT reassembling {word:#010x} at {at:#x}: {e:?}"))?;
        let Some(w) = w.as_int() else {
            return Err(format!(
                "{word:#010x} at {at:#x} decodes ({}) but does not reassemble",
                mnemonic(&d)
            ));
        };
        let w = u32::try_from(w & 0xffff_ffff).expect("masked");
        if u64::from(w) != word {
            return Err(format!(
                "{word:#010x} at {at:#x} decodes as {} and reassembles to {w:#010x}",
                mnemonic(&d)
            ));
        }
        out.extend_from_slice(&w.to_le_bytes());
        at += 4;
    }
    Ok(out)
}

fn mnemonic(d: &Value) -> String {
    field(d, "आज्ञा")
        .ok()
        .and_then(|v| {
            v.octets()
                .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
        })
        .unwrap_or_else(|| "?".into())
}

/// A spec program (or several, linked): the image, and the image written
/// again from the reassembled `.text` — `Ok((instructions, identical))`.
fn program_round_trip(spec_root: &Path, srcs: &[&str], load: u64) -> Result<(usize, bool), String> {
    let mut it = chain(spec_root)?;
    let mut objects = Vec::new();
    for p in srcs {
        let src = std::fs::read(repo_root().join("spec").join(format!("{p}.sas")))
            .map_err(|e| format!("{p}.sas: {e}"))?;
        objects.push(object(&mut it, &src)?);
    }
    for g in ["भारणस्थानम्", "भारस्थानम्"] {
        if !it.set_global(g, Value::Int(i128::from(load))) {
            return Err(format!("`{g}` is not a global of the chain"));
        }
    }
    let linked = it
        .call(
            "संयोजनॱसंयोजनम्",
            vec![Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(
                objects,
            )))],
            FUEL,
        )
        .map_err(|e| format!("FAULT in संयोजनम्: {e:?}"))?;
    if linked.is_nil() {
        return Err("REFUSED by the link".into());
    }
    let (text, data, bss) = (
        field(&linked, "पाठ्यम्")?,
        field(&linked, "दत्तम्")?,
        field(&linked, "बीजम्")?,
    );
    let write = |it: &mut Interpreter, t: Value| -> Result<Vec<u8>, String> {
        let img = it
            .call(
                "कोशॱप्रतिबिम्बलेखनम्",
                vec![t, data.clone(), bss.clone(), Value::Int(i128::from(load))],
                FUEL,
            )
            .map_err(|e| format!("FAULT in प्रतिबिम्बलेखनम्: {e:?}"))?;
        octets_of(&img)
    };
    let original = write(&mut it, text.clone())?;
    let t = octets_of(&text)?;
    let again_text = disassemble_reassemble(&mut it, &t)?;
    let again = write(&mut it, Value::Octets(Octets::new(&again_text)))?;
    if original.is_empty() {
        return Err("the image is empty".into());
    }
    Ok((t.len() / 4, original == again))
}

/// The compiler's emitted `.sas` for one corpus source, assembled by the
/// chain to an object: its `.text` round-tripped — `Ok((instructions, same))`.
fn emitted_round_trip(spec_root: &Path, path: &Path) -> Result<(usize, bool), String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
    let module = module_name(&src).ok_or("declares no module")?;
    let mut front = Front::load(spec_root).map_err(|e| format!("the front end: {e:?}"))?;
    let sas = (|| -> Result<String, String> {
        front.lex(&src)?;
        front.parse()?;
        front.resolve()?;
        front.typecheck()?;
        front.build_ir()?;
        let m = front.module(&module, None)?;
        riscv64::emit_module(&m).map_err(|e| format!("{e:?}"))
    })()?;
    let mut it = chain(spec_root)?;
    let o = object(&mut it, sas.as_bytes())?;
    let text = octets_of(&field(&o, "पाठ्यम्")?)?;
    let again = disassemble_reassemble(&mut it, &text)?;
    Ok((text.len() / 4, text == again))
}

/// The 48 programs as the census takes them: namaste-main through the pair.
fn programs() -> Vec<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(repo_root().join("spec"))
        .expect("spec/ reads")
        .filter_map(Result::ok)
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|n| n.strip_suffix(".sas"))
                .map(str::to_string)
        })
        .collect();
    names.sort();
    assert_eq!(
        names.len(),
        48,
        "the 48 spec programs; found {}",
        names.len()
    );
    names
        .into_iter()
        .map(|p| {
            if p == "namaste-main" {
                vec![p, "lib-mudraka".to_string()]
            } else {
                vec![p]
            }
        })
        .collect()
}

fn corpus() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(repo_root().join("crates/sadhana-t1/src"))
        .expect("the corpus reads")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    v
}

/// A copy of `spec/` with ONE encoding row changed: `addi`'s mask widened by
/// bit १५, the low bit of `rs1`, which the row's pattern leaves clear — so a
/// word the encoder writes for an odd `rs1` is no longer described by it.
fn mutated_spec() -> PathBuf {
    copied_spec(true)
}

/// The same copy with NOTHING changed: the control that makes the mutant's red
/// mean the row. The copy and the mutant differ in that one row only, so a copy
/// that fails to load (or any other cause) shows up here as well and is refused.
fn unchanged_copy_of_spec() -> PathBuf {
    copied_spec(false)
}

fn copied_spec(mutate: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w302-adr-spec.{}.{}",
        std::process::id(),
        if mutate { "mutant" } else { "control" }
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the copy's directory");
    let root = repo_root().join("spec");
    copy_tree(&root, &dir);
    let table = dir.join("encodings-riscv64.tsv");
    let text = std::fs::read_to_string(&table).expect("the table reads");
    let mut changed = 0;
    let out: Vec<String> = text
        .lines()
        .map(|l| {
            let mut cols: Vec<&str> = l.split('\t').collect();
            if mutate && cols.first() == Some(&"addi") && cols.get(4) == Some(&"0x0000707f") {
                cols[4] = "0x0000f07f";
                changed += 1;
            }
            cols.join("\t")
        })
        .collect();
    assert_eq!(
        changed,
        usize::from(mutate),
        "exactly one row changed in the mutant, none in the control"
    );
    std::fs::write(&table, out.join("\n") + "\n").expect("the changed table writes");
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    for e in std::fs::read_dir(from)
        .expect("reads")
        .filter_map(Result::ok)
    {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            std::fs::create_dir_all(&dst).expect("dir");
            copy_tree(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).expect("copy");
        }
    }
}

#[test]
fn a_program_reassembles_identically_and_a_changed_encoding_row_does_not() {
    // context: 23 of its `addi`s read an odd-numbered rs1 (riscv64-elf-objdump
    // on Rust's image), so the changed row is exercised.
    let spec = repo_root().join("spec");
    let (n, same) =
        program_round_trip(&spec, &["context"], 0x8020_0000).expect("context round-trips");
    println!("ROUND TRIP context: {n} instructions, image identical: {same}");
    assert!(same && n > 0, "context must reassemble to the same image");

    let control = unchanged_copy_of_spec();
    let c = program_round_trip(&control, &["context"], 0x8020_0000);
    let _ = std::fs::remove_dir_all(&control);
    assert!(
        matches!(c, Ok((n, true)) if n > 0),
        "the UNCHANGED copy of spec/ must round-trip, or the mutant's red says nothing about the row: {c:?}"
    );

    let bad = mutated_spec();
    let v = program_round_trip(&bad, &["context"], 0x8020_0000);
    println!("MUTANT addi mask |= bit 15, context: {v:?}");
    assert!(
        !matches!(v, Ok((_, true))),
        "the changed row must go red, got {v:?}"
    );
    let _ = std::fs::remove_dir_all(&bad);
}

#[test]
#[ignore = "census: 48 programs and the compiler's emitted .sas for every corpus source, interpreted (minutes in release)"]
fn every_program_and_the_compilers_emitted_sas_reassemble_identically() {
    let spec = repo_root().join("spec");
    let (mut ok, mut instructions, mut failed) = (0usize, 0usize, Vec::new());
    for set in programs() {
        let names: Vec<&str> = set.iter().map(String::as_str).collect();
        let load = load_of(names[0]);
        match program_round_trip(&spec, &names, load) {
            Ok((n, true)) => {
                ok += 1;
                instructions += n;
                println!(
                    "ROUND TRIP {}: {n} instructions, image identical",
                    names.join("+")
                );
            }
            Ok((n, false)) => failed.push(format!(
                "{}: {n} instructions, image DIFFERS",
                names.join("+")
            )),
            Err(e) => failed.push(format!("{}: {e}", names.join("+"))),
        }
    }
    let (mut emitted_ok, mut emitted_n, mut skipped) = (0usize, 0usize, Vec::new());
    for p in corpus() {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let src = std::fs::read_to_string(&p).expect("reads");
        if module_name(&src).is_none() {
            skipped.push(name);
            continue;
        }
        match emitted_round_trip(&spec, &p) {
            Ok((n, true)) => {
                emitted_ok += 1;
                emitted_n += n;
                println!("ROUND TRIP emitted {name}: {n} instructions, .text identical");
            }
            Ok((n, false)) => {
                failed.push(format!("emitted {name}: {n} instructions, .text DIFFERS"))
            }
            Err(e) => failed.push(format!("emitted {name}: {e}")),
        }
    }
    println!(
        "METRIC w302_assemble_disassemble_reassemble programs={ok}/48 instructions={instructions} emitted={emitted_ok} emitted_instructions={emitted_n} skipped_no_module={skipped:?} failed={}",
        failed.len()
    );

    // the control copy first: the same 48 through an UNCHANGED copy of spec/
    // must all round-trip, so a red below is the changed row and nothing else
    let control = unchanged_copy_of_spec();
    let mut control_bad = Vec::new();
    for set in programs() {
        let names: Vec<&str> = set.iter().map(String::as_str).collect();
        match program_round_trip(&control, &names, load_of(names[0])) {
            Ok((_, true)) => {}
            other => control_bad.push(format!("{}: {other:?}", names.join("+"))),
        }
    }
    let _ = std::fs::remove_dir_all(&control);
    assert!(
        control_bad.is_empty(),
        "the UNCHANGED copy of spec/ does not round-trip, so the mutant says nothing:\n{}",
        control_bad.join("\n")
    );

    // the mutant, over the same 48: the changed row must turn some red
    let bad = mutated_spec();
    let mut red = Vec::new();
    for set in programs() {
        let names: Vec<&str> = set.iter().map(String::as_str).collect();
        match program_round_trip(&bad, &names, load_of(names[0])) {
            Ok((_, true)) => {}
            Ok((_, false)) => red.push(format!("{}: image differs", names.join("+"))),
            Err(e) => red.push(format!("{}: {e}", names.join("+"))),
        }
    }
    let _ = std::fs::remove_dir_all(&bad);
    println!(
        "METRIC w302_adr_mutant addi_mask_bit15 red={}/48",
        red.len()
    );
    for r in red.iter().take(5) {
        println!("MUTANT {r}");
    }
    assert!(
        failed.is_empty(),
        "round trip fails:\n{}",
        failed.join("\n")
    );
    assert!(
        !red.is_empty(),
        "the changed encoding row turned nothing red"
    );
}
