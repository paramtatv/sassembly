//! `SAS-011` (c): THE TWO EMITTERS WRITE THE SAME STRING POOL, AND THE POOL
//! TAKES THE TEXT FORM EXACTLY WHERE THE NARROW RULE ADMITS IT.
//!
//! One module of seven routines, each returning one literal, is compiled twice
//! through the `.t1` front end: once to the end by `शृङ्खला` (the `.t1`
//! emitter, `yantrotsarjana.t1`), and once through `Front` into `riscv64.rs`.
//! The pool — from `पाठकोशःॱॱ` to the end of the text — must be the same
//! octets, and each literal's directive must be the form the rule names:
//!
//! | literal       | why                                   | form     |
//! |---------------|---------------------------------------|----------|
//! | `नमस्ते`       | one word                              | text     |
//! | `इतिहासः`      | begins with the close's spelling      | text     |
//! | `क ख`          | a space                               | numerals |
//! | `इति`, `आस्की`, `जाल` | the three words the rule names | numerals |
//! | `सोऽहम्`        | the avagraha, which the lexer peels   | numerals |
//!
//! Then the agreed text is assembled and each literal's datum is compared with
//! the literal's own octets, so what agreed is also what the assembler reads.

use sadhana::parse::{self, Section};
use sadhana::t1::chain::{CHAIN, Front};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// The literals in source order, as the `.t1` source spells them and as the
/// octets they denote. `इति` is spelled doubled inside a literal (ADR-0011).
const LITERALS: &[(&str, &str, bool)] = &[
    ("नमस्ते", "नमस्ते", true),
    ("इतिहासः", "इतिहासः", true),
    ("क ख", "क ख", false),
    ("इति इति", "इति", false),
    ("आस्की", "आस्की", false),
    ("जाल", "जाल", false),
    ("सोऽहम्", "सोऽहम्", false),
];

/// Routine names, one per literal; none is a keyword of either grammar.
const NAMES: &[&str] = &["घक", "घख", "घग", "घघ", "घङ", "घच", "घछ"];

fn source() -> String {
    let mut s = String::from("मण्डलम् क ॥\n");
    for (name, (spelled, _, _)) in NAMES.iter().zip(LITERALS) {
        s.push_str(&format!(
            "सार्वजनिक वृत्तिः {name} ददाति पाठः आदि प्रत्यागमनम् उक्तम् {spelled} इति । इति\n"
        ));
    }
    s
}

/// The `.t1` emitter's text, through the shipped chain.
fn t1_text(src: &str) -> String {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the shipped chain loads");
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![
                Value::Octets(Octets::new(src.as_bytes())),
                Value::Octets(Octets::new("क".as_bytes())),
            ],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs");
    let exit = it.global("सङ्कलनविरामभेद").and_then(Value::as_int);
    assert_eq!(exit, Some(0), "the `.t1` chain must emit this module");
    String::from_utf8(text.octets().expect("a run of octets").as_slice().to_vec())
        .expect("the emitter writes UTF-8")
}

/// `riscv64.rs`'s text for the same module, through the same `.t1` front end.
fn rust_text(src: &str) -> String {
    let mut front = Front::load(&spec_root()).expect("the front end loads");
    front.lex(src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("IR");
    let module = front.module("क", None).expect("module");
    riscv64::emit_module(&module).expect("riscv64.rs emits")
}

fn pool(text: &str) -> Vec<&str> {
    let at = text
        .find("पाठकोशःॱॱ")
        .unwrap_or_else(|| panic!("the module has a string pool:\n{text}"));
    text[at..].lines().collect()
}

#[test]
fn the_two_emitters_write_the_same_pool_and_the_text_form_where_the_rule_admits_it() {
    let src = source();
    let t1 = t1_text(&src);
    let rust = rust_text(&src);
    let (a, b) = (pool(&t1), pool(&rust));
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x,
            y,
            "pool line {}: the `.t1` emitter wrote `{x}`, riscv64.rs `{y}`",
            i + 1
        );
    }
    assert_eq!(a.len(), b.len(), "the two pools differ in length");

    let directives: Vec<&str> = a
        .iter()
        .copied()
        .filter(|l| l.starts_with("॥ अष्टकाः "))
        .collect();
    assert_eq!(
        directives.len(),
        LITERALS.len(),
        "one directive per literal:\n{t1}"
    );
    for ((_, text, as_text), line) in LITERALS.iter().zip(&directives) {
        let spelled = format!("॥ अष्टकाः उक्तम् {text} इति ॥");
        if *as_text {
            assert_eq!(
                *line, spelled,
                "`{text}` is one word and takes the text form"
            );
        } else {
            assert!(
                !line.contains("उक्तम्"),
                "`{text}` is outside the narrow rule and stays numerals: `{line}`"
            );
        }
    }
    println!(
        "METRIC sas011_text_literals {}/{}",
        directives.iter().filter(|l| l.contains("उक्तम्")).count(),
        directives.len()
    );

    // The agreed text, assembled: every literal's datum is its own octets.
    let program = parse::assemble_program(&t1).unwrap_or_else(|e| panic!("assembles: {e:?}"));
    let data: Vec<&[u8]> = program
        .data
        .iter()
        .filter(|d| d.section != Section::Bss)
        .map(|d| d.bytes.as_slice())
        .collect();
    for (_, text, _) in LITERALS {
        assert!(
            data.contains(&text.as_bytes()),
            "no datum holds `{text}`'s octets after assembly"
        );
    }
}
