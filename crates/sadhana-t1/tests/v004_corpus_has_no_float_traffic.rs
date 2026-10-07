//! ॥ V-004: REAL CODE IS UNCHANGED, AS A TEST THAT CAN FAIL ॥
//!
//! V-004 gives every routine a second (float) register map and runs a second
//! allocation in the module loop, with a classifier that answers Int for every
//! value, because the IR has no float kind yet (V-005 adds one). So the float map
//! is EMPTY in real code, the frame is the frame it was, and the Rust back end's
//! output for the compiler corpus is the text it was. That was first shown as an
//! observation (the 20 emitted modules byte-identical before and after
//! 46144cff); the Naad lane's review asked for it to be a test.
//!
//! WHAT THIS ASSERTS: emitting every corpus source through `riscv64::emit_module`
//! produces NO float store or load at all (`प्लवनिधानम्`, `प्लवाहारः`). A non-empty
//! float map would put them there — the prologue saves the float callee-saved
//! registers it uses and every float spill is float traffic, which the unit test
//! `v004_a_spilled_float_value_uses_the_float_load_and_store_at_its_float_offset`
//! in riscv64.rs proves by emitting them — so this goes red the day real code
//! starts classifying a value as float without V-005's producer behind it.
//!
//! FULL ACCOUNTING, so a refused or skipped source cannot leave this green on a
//! subset: every source reaches the emitter, `lib.t1` is the only one declaring no
//! module, and the counts add up.

use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::riscv64;
use std::path::PathBuf;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn spec_root() -> PathBuf {
    crate_dir().join("../../spec")
}

/// The float store and load the emitter spells (riscv64.rs `store_mnemonic` /
/// `load_mnemonic` for `RegClass::Float`).
const FLOAT_TRAFFIC: [&str; 2] = ["प्लवनिधानम्", "प्लवाहारः"];

#[test]
fn the_corpus_emits_no_float_store_or_load() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(crate_dir().join("src"))
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "the corpus is not empty");

    let (mut objects, mut total_lines) = (0usize, 0usize);
    let mut no_module: Vec<String> = Vec::new();
    let mut refused = Vec::new();
    let mut offenders = Vec::new();
    for p in &paths {
        let src = std::fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
        let Some(module) = module_name(&src) else {
            no_module.push(
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            continue;
        };
        let mut front = Front::load(&spec_root()).expect("the front end loads");
        let text = (|| -> Result<String, String> {
            front.lex(&src)?;
            front.parse()?;
            front.resolve()?;
            front.typecheck()?;
            front.build_ir()?;
            let module = front.module(&module, None)?;
            riscv64::emit_module(&module).map_err(|e| format!("{e:?}"))
        })();
        let text = match text {
            Ok(t) => t,
            Err(e) => {
                refused.push(format!("{}: {e}", p.display()));
                continue;
            }
        };
        objects += 1;
        total_lines += text.lines().count();
        for (n, line) in text.lines().enumerate() {
            let code = without_text_pieces(line);
            if FLOAT_TRAFFIC.iter().any(|w| code.contains(w)) {
                offenders.push(format!("{}:{}: {line}", p.display(), n + 1));
            }
        }
    }
    println!(
        "METRIC v004_corpus_objects {objects} lines {total_lines} float_traffic {}",
        offenders.len()
    );
    assert!(
        refused.is_empty(),
        "every source that declares a module must reach the emitter:\n  {}",
        refused.join("\n  ")
    );
    assert_eq!(
        no_module,
        vec!["lib.t1".to_string()],
        "`lib.t1` is the ONE source that declares no module"
    );
    assert_eq!(
        objects + no_module.len(),
        paths.len(),
        "every source accounted for"
    );
    assert!(objects > 0 && total_lines > 0, "the sweep emitted nothing");
    assert!(
        offenders.is_empty(),
        "the corpus emits float stores or loads, so the float map is no longer empty in \
         real code (V-004's Int-only classifier): {} line(s):\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}

/// `line` with every `उक्तम् <word> इति` TEXT PIECE removed (`SAS-011` (c)): a
/// literal's octets are DATA, not an instruction. Since (c), the emitter's own
/// mnemonic-name strings (`प्लवनिधानम्`, `प्लवाहारः`) travel as text, and a word
/// match over the whole line read them as float traffic. A text piece is ONE word
/// (the narrow rule), so the span ends at the first ` इति` after the opener.
fn without_text_pieces(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(i) = rest.find("उक्तम् ") {
        out.push_str(&rest[..i]);
        let after = &rest[i + "उक्तम् ".len()..];
        match after.find(" इति") {
            Some(j) => rest = &after[j + " इति".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

#[test]
fn a_text_piece_is_data_and_an_instruction_is_not() {
    // The scanner's own control: the literal is skipped, a real mnemonic is not.
    for w in FLOAT_TRAFFIC {
        let piece = format!("    ॥ अष्टकाः उक्तम् {w} इति ॥");
        assert!(!without_text_pieces(&piece).contains(w), "{piece}");
        let insn = format!("    {w} क्ष१ ० क२ ।");
        assert!(without_text_pieces(&insn).contains(w), "{insn}");
    }
}
