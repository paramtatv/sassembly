//! `W-239` — THE OCTET ARENA, WIRED: the T1 assembler's directive readers
//! appending into `अष्टक` and naming their ranges as data, driven end to end
//! through `सङ्कलनम्` (lexer, statement splitter, directive executor).
//!
//! Before this row `निर्देशानुष्ठानम्` applied only the four byte-free
//! directives and answered ० for every other, so `आस्की`, `जाल` and the five
//! width directives were recorded as statements and never emitted a byte
//! from T1; the width arm's string case returned ० without reading; and
//! nothing ever called `आज्ञाकूटरचना` or `अङ्कपाठरचना`, so every P-code a
//! diagnostic carried was the EMPTY RUN. The first test holds that last
//! defect's before and after.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// The reader and what it reaches: the lexer, the numeral reader the width
/// arm asks (`अक्षरकोश`), the encoder some sentence paths reach, and the arena.
fn assembler() -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("vakyavibhaga.t1", &source("vakyavibhaga.t1")),
            ("ashtaka.t1", &source("ashtaka.t1")),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
            ("encode.t1", &source("encode.t1")),
        ],
        &spec_root(),
    )
    .unwrap_or_else(|e| panic!("the reader and the arena load: {e:?}"))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn bytes_of(v: &Value) -> Vec<u8> {
    match v {
        Value::Octets(o) => o.as_slice().to_vec(),
        other => panic!("a run of octets, not {other:?}"),
    }
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("the global {name} holds an integer"))
}

/// Every live record of a one-based arena global, slot ० skipped.
fn records(it: &Interpreter, arena: &str, count: i128) -> Vec<HashMap<String, Value>> {
    match it.global(arena) {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .skip(1)
            .take(usize::try_from(count).unwrap_or(0))
            .map(|v| match v {
                Value::Record(r) => r.borrow().clone(),
                other => panic!("{arena} holds {other:?}, not a record"),
            })
            .collect(),
        other => panic!("{arena} is {other:?}, not an arena"),
    }
}

/// `(section, bytes, line)` of every datum the reader emitted.
fn data(it: &Interpreter) -> Vec<(i128, Vec<u8>, i128)> {
    records(it, "दत्तकोश", global_int(it, "दत्तसूचकाङ्क"))
        .iter()
        .map(|r| {
            (
                r.get("कोष्ठक").and_then(Value::as_int).unwrap_or(-1),
                r.get("अष्टकाः").map(bytes_of).unwrap_or_default(),
                r.get("पङ्क्ति").and_then(Value::as_int).unwrap_or(-1),
            )
        })
        .collect()
}

/// `(code, line)` of every diagnostic the reader raised.
fn errors(it: &Interpreter) -> Vec<(String, i128)> {
    records(it, "वाक्यविभागदोषकोश", global_int(it, "वाक्यविभागदोषसूचकाङ्क"))
        .iter()
        .map(|r| {
            (
                r.get("कूट")
                    .map(|v| String::from_utf8_lossy(&bytes_of(v)).into_owned())
                    .unwrap_or_default(),
                r.get("पङ्क्ति").and_then(Value::as_int).unwrap_or(-1),
            )
        })
        .collect()
}

/// Begin a program and assemble `src`; the statement count `सङ्कलनम्` answers.
fn assemble(it: &mut Interpreter, src: &str) -> i128 {
    it.call("वाक्यविभागॱआरम्भः", vec![], 5_000_000)
        .unwrap_or_else(|e| panic!("आरम्भः runs: {e:?}"));
    // The directive registry is read from `spec/directives.tsv` by an explicit
    // call and by nothing else; without it EVERY directive is P13 ("not a
    // directive"), the section directive included — which the real-program
    // test in `t1_execution.rs` does not see, because it asserts a statement
    // count and never reads `वाक्यविभागदोषकोश`.
    it.call("वाक्यविभागॱनिर्देशकोशपठनम्", vec![], 200_000_000)
        .unwrap_or_else(|e| panic!("निर्देशकोशपठनम् runs: {e:?}"));
    it.call("वाक्यविभागॱसङ्कलनम्", vec![octets(src)], 200_000_000)
        .unwrap_or_else(|e| panic!("सङ्कलनम् runs over {src:?}: {e:?}"))
        .as_int()
        .expect("a statement count")
}

const DATA: &str = "॥ कोष्ठकम् ॱदत्त ॥\n";

/// THE DEFECT THE WIRING CLOSES, before and after. `आज्ञाकूटरचना` and
/// `अङ्कपाठरचना` build the P-code and digit tables a diagnostic carries; no
/// routine called either, so the tables were empty runs and every diagnostic
/// said `` where it meant `P03`. `आरम्भः` builds them now.
#[test]
fn the_p_codes_are_empty_before_the_reader_starts_and_built_after() {
    let mut it = assembler();
    let code = |it: &Interpreter, name: &str| bytes_of(it.global(name).expect(name));
    // BEFORE: the state every earlier run of this reader was in.
    assert_eq!(
        code(&it, "अज्ञातसंज्ञाकूटः"),
        Vec::<u8>::new(),
        "P03 was the empty run"
    );
    assert_eq!(
        code(&it, "उक्तिव्याप्तिकूटः"),
        Vec::<u8>::new(),
        "P17 was the empty run"
    );
    assert_eq!(
        code(&it, "एकाङ्कपाठः"),
        Vec::<u8>::new(),
        "the digit `१` was the empty run"
    );

    it.call("वाक्यविभागॱआरम्भः", vec![], 5_000_000)
        .expect("आरम्भः runs");

    // AFTER.
    assert_eq!(code(&it, "अज्ञातसंज्ञाकूटः"), b"P03".to_vec());
    assert_eq!(code(&it, "अप्रकारकूटः"), b"P23".to_vec());
    assert_eq!(code(&it, "उक्तिव्याप्तिकूटः"), b"P17".to_vec());
    assert_eq!(code(&it, "एकाङ्कपाठः"), "१".as_bytes().to_vec());
    // And the arena begins with the program.
    assert_eq!(global_int(&it, "अष्टकसूचकाङ्क"), 0);
}

/// `W-242` — THE CLOSING WORD IS THE LITERAL AGAIN, NOW BEHIND A ROUTINE.
/// W-242 made `इतिशब्दः` a GLOBAL initialised `भवति उक्तम् इति इति इति` once the
/// interpreter's loader evaluated such initializers. That was right for the
/// interpreter and wrong for the image: the NATIVE lowering drops a global's
/// literal initializer, so in the compiled compiler the word was the empty run
/// (measured 2026-09-21, alongside the allocator's two names, which the same
/// defect emptied). A zero-argument routine returning the literal is honoured
/// by both engines, and a bare name of it is still a call, so every use is
/// unchanged. The P-codes the reader raises are the same ones: a literal that
/// closes raises nothing and is written; a literal nothing closes is P22, the
/// code every literal read while the word was empty.
#[test]
fn the_closing_word_is_the_literal_at_load_and_a_literal_closes_against_it() {
    let mut it = assembler();
    let word = it
        .call("वाक्यविभागॱइतिशब्दः", vec![], 1_000_000)
        .expect("the closing word is a routine returning its literal");
    assert_eq!(
        bytes_of(&word),
        "इति".as_bytes().to_vec(),
        "the closing word, returned from its literal by a routine both engines honour"
    );

    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ अष्टकाः उक्तम् क इति ॥\n"));
    assert_eq!(
        errors(&it),
        Vec::<(String, i128)>::new(),
        "a closed literal"
    );
    assert_eq!(data(&it)[0].1, "क".as_bytes().to_vec());

    // REFUSED as before: nothing closes it — P22.
    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ अष्टकाः उक्तम् क ॥\n"));
    assert_eq!(
        errors(&it).first().map(|e| e.0.as_str()),
        Some("P22"),
        "an unclosed literal: {:?}",
        errors(&it)
    );
    assert!(
        data(&it).is_empty(),
        "nothing emitted for a refused literal"
    );
}

/// `॥ आस्की क ख ॥` — Rust's arm: the words joined by a space, as one datum in
/// the open section. The octets live in the arena and the datum names them.
#[test]
fn an_ascii_directive_puts_its_words_in_the_arena_and_the_datum_names_the_range() {
    let mut it = assembler();
    let n = assemble(&mut it, &format!("{DATA}॥ आस्की क ख ॥\n"));
    assert!(n >= 2, "two statements: {n}");
    assert_eq!(errors(&it), Vec::<(String, i128)>::new());
    let d = data(&it);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].0, 2, "Section::Data");
    assert_eq!(d[0].1, "क ख".as_bytes().to_vec());
    assert_eq!(d[0].2, 2, "the directive's line");
    assert_eq!(
        global_int(&it, "अष्टकसूचकाङ्क"),
        7,
        "क (3) + space (1) + ख (3) octets in the arena"
    );

    // REFUSED as before: no operand at all is P16, and no datum.
    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ आस्की ॥\n"));
    assert_eq!(errors(&it).first().map(|e| e.0.as_str()), Some("P16"));
    assert!(data(&it).is_empty());
}

/// `॥ जाल शब्दः ॥` — the web arm maps a word through the dictionary; a word it
/// does not know is written as it is, and either way the datum is a range of
/// the arena.
#[test]
fn a_web_directive_writes_through_the_dictionary_or_as_written() {
    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ जाल क्ष ॥\n"));
    assert_eq!(errors(&it), Vec::<(String, i128)>::new());
    let d = data(&it);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(!d[0].1.is_empty(), "something was written for `क्ष`");
    assert_eq!(
        global_int(&it, "अष्टकसूचकाङ्क"),
        i128::try_from(d[0].1.len()).expect("fits"),
        "the datum is exactly what the arena holds"
    );
}

/// The width arm's string case, which answered ० and wrote nothing: `॥ अष्टकाः
/// उक्तम् क ख इति ॥` is the text's UTF-8 with nothing appended; a doubled इति
/// is the one word इति. REFUSED: a string under any width but one is P17 —
/// the code that now exists because this arm now runs.
#[test]
fn a_string_operand_of_a_width_directive_is_written_and_a_wide_one_is_refused() {
    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ अष्टकाः उक्तम् क ख इति ॥\n"));
    assert_eq!(errors(&it), Vec::<(String, i128)>::new());
    let d = data(&it);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].1, "क ख".as_bytes().to_vec());

    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ अष्टकाः उक्तम् क इति इति ख इति ॥\n"));
    assert_eq!(errors(&it), Vec::<(String, i128)>::new());
    assert_eq!(
        data(&it)[0].1,
        "क इति ख".as_bytes().to_vec(),
        "a pair is the word"
    );

    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ द्वयष्टकाः उक्तम् क इति ॥\n"));
    assert_eq!(
        errors(&it).first().map(|e| e.0.as_str()),
        Some("P17"),
        "a string under a width other than one is refused: {:?}",
        errors(&it)
    );
    assert!(
        data(&it).is_empty(),
        "nothing emitted for a refused directive"
    );
}

/// The width arm as it always was, now reached from T1 and writing through
/// the arena: little-endian, the low octets first.
#[test]
fn a_width_directive_writes_little_endian_octets_through_the_arena() {
    let mut it = assembler();
    assemble(&mut it, &format!("{DATA}॥ द्वयष्टकाः २५८ ॥\n"));
    assert_eq!(errors(&it), Vec::<(String, i128)>::new());
    let d = data(&it);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].1, vec![2, 1], "258 = 0x0102, low octet first");
    assert_eq!(global_int(&it, "अष्टकसूचकाङ्क"), 2);
}
