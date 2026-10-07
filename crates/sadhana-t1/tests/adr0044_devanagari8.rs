//! **ADR-0044 — DEVANAGARI-8: ONE OCTET PER LETTER, MARKED `वर्णाष्टकम्`.**
//! The measure-first prototype's tests, one per decision the prototype has to
//! show working before anything lands:
//!
//! 1. **D2, the twins accept and refuse alike.** `lex.rs`'s `lex_t1` and the
//!    Rust parser on one side, `lex.t1` + `व्याकर` on the other, both take
//!    `वर्णाष्टकम् … इति`, and both refuse a space, an ASCII digit and a Latin
//!    letter inside it with the owner's refusal, naming the letter.
//! 2. **D1/D5, the value.** The interpreter's value of a literal is the D1
//!    octets — the block's edge letters included, and `इति इति` inside — and the
//!    `.t1` chain's emitted pool, assembled, holds the same octets.
//! 3. **D3, the two assemblers pack alike.** `॥ अष्टकाः वर्णाष्टकम् … इति ॥`
//!    through `parse.rs` and through `वाक्यविभाग` gives one datum.
//! 4. **D4, the tables.** `sarani.t1`'s decoder fills the embed store with the
//!    same octets the previous (UTF-8) module filled: `spec/`'s, and — with
//!    `ADR0044_BASE_SARANI` naming the base tree's `sarani.t1` — the base
//!    module's own decode, compared octet for octet.
//!
//! Every Devanagari name here is the crate's constant (`devanagari8::OPEN`,
//! `devanagari8::REFUSAL`) or a code point, never a retyped spelling.

use sadhana::devanagari8::{self, OPEN, REFUSAL};
use sadhana::lex::{self, Kind};
use sadhana::parse::{self, Section};
use sadhana::t1::chain::{CHAIN, Front};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::riscv64;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod spec_fixture;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
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

fn cp(c: u32) -> char {
    char::from_u32(c).expect("a scalar value")
}

/// The block's edge letters the ADR names: U+0900, ऽ, ।, ॰, ॱ, U+097F.
fn edges() -> String {
    [0x0900, 0x093D, 0x0964, 0x0970, 0x0971, 0x097F]
        .into_iter()
        .map(cp)
        .collect()
}

/// D1, written out independently of `devanagari8::pack`: from the UTF-8 triple.
fn d1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| {
            let mut b = [0u8; 4];
            let u = c.encode_utf8(&mut b).as_bytes();
            assert_eq!(u.len(), 3, "`{c}` is not in the block");
            assert_eq!(u[0], 0xE0, "`{c}` is not in the block");
            0x80 + (u[1] - 0xA4) * 64 + (u[2] - 0x80)
        })
        .collect()
}

// ── 1. the twins accept and refuse alike ─────────────────────────────────────

/// The three bodies the owner's refusal must catch, with the offending letter.
fn refused_bodies() -> Vec<(String, char)> {
    let k = cp(0x0915);
    let kh = cp(0x0916);
    vec![
        (format!("{k} {kh}"), ' '),
        (format!("{k}1{kh}"), '1'),
        (format!("{k}a{kh}"), 'a'),
    ]
}

/// One routine returning one literal, in a module of its own.
fn program(literal: &str) -> String {
    format!("मण्डलम् क ॥\nसार्वजनिक वृत्तिः घक ददाति अङ्कः अन्तः अ८ आदि\n    प्रत्यागमनम् {literal} ।\nइति\n")
}

#[test]
fn the_rust_lexer_takes_the_literal_whole_and_marks_it() {
    let body: String = [0x0915, 0x0916, 0x0960, 0x097F]
        .into_iter()
        .map(cp)
        .collect();
    let src = format!("{OPEN} {body} इति ।");
    let toks = lex::lex_t1(&src).unwrap_or_else(|e| panic!("lexes: {e:?}"));
    assert!(
        matches!(&toks[0].kind, Kind::Str { value } if *value == body),
        "{:?}",
        toks[0]
    );
    assert!(devanagari8::is_literal(&toks[0]));
    assert_eq!(devanagari8::literal_octets(&toks[0]), Some(d1(&body)));
    // the doubled close inside, exactly as `उक्तम्`
    let toks = lex::lex_t1(&format!("{OPEN} इति इति इति ।")).expect("the pair lexes");
    assert!(matches!(&toks[0].kind, Kind::Str { value } if value == "इति"));
    assert_eq!(devanagari8::literal_octets(&toks[0]), Some(d1("इति")));
    // the control: `उक्तम्` is untouched, a space and all
    let toks = lex::lex_t1("उक्तम् क ख इति ।").expect("उक्तम् with a space lexes");
    assert!(!devanagari8::is_literal(&toks[0]));
    assert_eq!(
        devanagari8::literal_octets(&toks[0]),
        Some("क ख".as_bytes().to_vec())
    );
}

#[test]
fn the_rust_lexer_refuses_a_space_a_digit_and_a_latin_letter_by_the_owners_name() {
    for (body, bad) in refused_bodies() {
        let line = format!("चरः क ॱॱ अङ्कः अन्तः अ८ भवति {OPEN} {body} इति ।");
        let errs = lex::lex_t1(&line).expect_err("refused");
        let column = line[..line.find(&format!("{OPEN} ")).unwrap() + OPEN.len() + 1]
            .chars()
            .count()
            + body.chars().position(|c| c == bad).unwrap()
            + 1;
        assert!(
            errs.iter().any(|e| e.reason.contains(REFUSAL)
                && e.aksara == bad.to_string()
                && e.reason.contains(&format!("column {column}"))),
            "`{bad}` in `{body}`: {errs:?}"
        );
        // and NOT the repertoire's general refusal for the same letter
        assert!(
            !errs.iter().any(|e| e.reason.contains("repertoire")),
            "the owner's refusal, not doc 15's: {errs:?}"
        );
    }
}

#[test]
fn the_rust_parser_accepts_the_literal() {
    let body: String = [0x0915, 0x0916].into_iter().map(cp).collect();
    let src = program(&format!("{OPEN} {body} इति"));
    let mut front = Front::load(&spec_root()).expect("the front end loads");
    front.lex(&src).expect("lex");
    front
        .parse()
        .expect("the Rust-driven `.t1` parse accepts it");
    let toks = lex::lex_t1(&src).expect("lex_t1");
    let mut p = sadhana::t1::parse::Parser::new(&toks);
    p.parse_program().expect("the Rust parser accepts it");
}

/// `lex.t1` + `व्याकर`, as `t1_paradigm_names.rs` drives them.
fn t1_parse(src: &str) -> (i128, Vec<HashMap<String, Value>>) {
    let names = ["lex.t1", "ast.t1", "parse.t1", "sanskrit_text.t1"];
    let texts: Vec<(String, String)> = names.iter().map(|n| (n.to_string(), source(n))).collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("the parser loads");
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("पदविभाग runs")
        .as_int()
        .expect("a token count");
    it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 20_000_000_000)
        .expect("कार्यक्रमपठनम् runs");
    let n = it
        .global("दोषसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("दोषसूचकाङ्क");
    let rows = match it.global("दोषकोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .skip(1)
            .take(usize::try_from(n).unwrap_or(0))
            .map(|v| match v {
                Value::Record(r) => r.borrow().clone(),
                other => panic!("दोषकोश holds {other:?}"),
            })
            .collect(),
        other => panic!("दोषकोश is {other:?}"),
    };
    (n, rows)
}

#[test]
fn the_t1_parser_accepts_the_literal_and_the_pair() {
    let body: String = [0x0915, 0x093D, 0x0916, 0x097F]
        .into_iter()
        .map(cp)
        .collect();
    for lit in [
        format!("{OPEN} {body} इति"),
        format!("{OPEN} इति इति इति"),
        format!("{OPEN} इति"),
    ] {
        let (n, rows) = t1_parse(&program(&lit));
        assert_eq!(n, 0, "`{lit}` parses: {rows:?}");
    }
}

#[test]
fn the_t1_parser_refuses_a_space_a_digit_and_a_latin_letter_by_the_owners_name() {
    for (body, bad) in refused_bodies() {
        let (n, rows) = t1_parse(&program(&format!("{OPEN} {body} इति")));
        assert!(n >= 1, "`{body}` is refused");
        let reasons: Vec<String> = rows
            .iter()
            .map(|r| String::from_utf8_lossy(&bytes_of(r.get("कारण").expect("कारण"))).into_owned())
            .collect();
        assert!(
            reasons
                .iter()
                .any(|r| r.contains(REFUSAL) && r.contains(bad)),
            "`{bad}` in `{body}`: {reasons:?}"
        );
    }
}

// ── 2. the value: interpreter, emitted pool ──────────────────────────────────

/// A literal's value, read by the interpreter running the program.
fn interpreted(literal: &str) -> Vec<u8> {
    let src = program(literal);
    let mut it = Interpreter::load(&[("k.t1", src.as_str())], &spec_root()).expect("loads");
    bytes_of(&it.call("कॱघक", vec![], 1_000_000).expect("runs"))
}

#[test]
fn the_interpreter_reads_the_d1_octets_edges_and_pair_included() {
    let e = edges();
    assert_eq!(interpreted(&format!("{OPEN} {e} इति")), d1(&e));
    assert_eq!(
        interpreted(&format!("{OPEN} इति इति इति")),
        vec![0x87, 0xA4, 0xBF]
    );
    assert_eq!(interpreted(&format!("{OPEN} इति")), Vec::<u8>::new());
    let want: Vec<u8> = vec![0x80, 0xBD, 0xE4, 0xF0, 0xF1, 0xFF];
    assert_eq!(
        d1(&e),
        want,
        "the edges are 0x80, ऽ BD, । E4, ॰ F0, ॱ F1, 0xFF"
    );
    // the control: `उक्तम्` is still its UTF-8
    assert_eq!(interpreted("उक्तम् क इति"), "क".as_bytes().to_vec());
}

/// The `.t1` chain's text for one module (`sas011_text_literal_twin.rs`'s).
fn t1_text(src: &str) -> String {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the shipped chain loads");
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src), octets("क")],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs");
    let exit = it.global("सङ्कलनविरामभेद").and_then(Value::as_int);
    assert_eq!(exit, Some(0), "the `.t1` chain must emit this module");
    String::from_utf8(bytes_of(&text)).expect("the emitter writes UTF-8")
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

/// The literals the pool test compiles, and whether each takes the
/// `वर्णाष्टकम्` spelling in the pool. The edges (with `ॱ`, `।` and `ऽ`) and the
/// pair are numerals: `devanagari8::payload` refuses the marks and the word.
/// `॰` is left out of the compiled module: `lex.t1` cuts a line at the comment
/// mark before any literal is seen (ADR-0017 is the Rust lexer's only), for
/// `उक्तम्` exactly as for this opener.
fn pool_literals() -> Vec<(String, Vec<u8>, bool)> {
    let plain: String = [0x0915, 0x0960, 0x097F, 0x0905, 0x0966]
        .into_iter()
        .map(cp)
        .collect();
    let edges: String = [0x0900, 0x093D, 0x0964, 0x0971, 0x097F]
        .into_iter()
        .map(cp)
        .collect();
    vec![
        (plain.clone(), d1(&plain), true),
        (edges.clone(), d1(&edges), false),
        ("इति इति".to_string(), d1("इति"), false),
    ]
}

fn pool_source() -> String {
    let names = ["घक", "घख", "घग"];
    let mut s = String::from("मण्डलम् क ॥\n");
    for (name, (spelled, _, _)) in names.iter().zip(pool_literals()) {
        s.push_str(&format!(
            "सार्वजनिक वृत्तिः {name} ददाति अङ्कः अन्तः अ८ आदि प्रत्यागमनम् {OPEN} {spelled} इति । इति\n"
        ));
    }
    s
}

#[test]
fn the_two_emitters_write_one_pool_and_it_assembles_to_the_d1_octets() {
    let src = pool_source();
    let t1 = t1_text(&src);
    let rust = rust_text(&src);
    let (a, b) = (pool(&t1), pool(&rust));
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "pool line {}: `.t1` `{x}`, riscv64.rs `{y}`", i + 1);
    }
    assert_eq!(a.len(), b.len(), "the two pools differ in length");
    let directives: Vec<&str> = a
        .iter()
        .copied()
        .filter(|l| l.starts_with("॥ अष्टकाः "))
        .collect();
    assert_eq!(directives.len(), pool_literals().len(), "{t1}");
    for ((text, _, as_letters), line) in pool_literals().iter().zip(&directives) {
        if *as_letters {
            assert_eq!(*line, format!("॥ अष्टकाः {OPEN} {text} इति ॥"));
        } else {
            assert!(!line.contains(OPEN), "`{text}` stays numerals: `{line}`");
        }
    }
    // assembled by the Rust assembler: each datum is the literal's D1 octets
    let program = parse::assemble_program(&t1).unwrap_or_else(|e| panic!("assembles: {e:?}"));
    let data: Vec<&[u8]> = program
        .data
        .iter()
        .filter(|d| d.section != Section::Bss)
        .map(|d| d.bytes.as_slice())
        .collect();
    for (text, want, _) in pool_literals() {
        assert!(
            data.contains(&want.as_slice()),
            "no datum holds `{text}`'s Devanagari-8 octets {want:02x?}"
        );
    }
}

// ── 3. the two assemblers pack alike ─────────────────────────────────────────

const DATA: &str = "॥ कोष्ठकम् ॱदत्त ॥\n";

/// `t1_ashtaka_wired.rs`'s reader: the lexer, the statement splitter, the
/// arena and what they reach.
fn t1_assembler() -> Interpreter {
    let names = [
        "lex.t1",
        "vakyavibhaga.t1",
        "ashtaka.t1",
        "sanskrit_text.t1",
        "encode.t1",
    ];
    let texts: Vec<(String, String)> = names.iter().map(|n| (n.to_string(), source(n))).collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).expect("the reader loads")
}

fn records(it: &Interpreter, arena: &str, count: &str) -> Vec<HashMap<String, Value>> {
    let n = it.global(count).and_then(Value::as_int).unwrap_or(0);
    match it.global(arena) {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .skip(1)
            .take(usize::try_from(n).unwrap_or(0))
            .map(|v| match v {
                Value::Record(r) => r.borrow().clone(),
                other => panic!("{arena} holds {other:?}"),
            })
            .collect(),
        other => panic!("{arena} is {other:?}"),
    }
}

/// `(data, error codes)` from the `.t1` assembler.
fn t1_assemble(src: &str) -> (Vec<Vec<u8>>, Vec<String>) {
    let mut it = t1_assembler();
    it.call("वाक्यविभागॱआरम्भः", vec![], 5_000_000)
        .expect("आरम्भः");
    it.call("वाक्यविभागॱनिर्देशकोशपठनम्", vec![], 200_000_000)
        .expect("निर्देशकोशपठनम्");
    it.call("वाक्यविभागॱसङ्कलनम्", vec![octets(src)], 400_000_000)
        .expect("सङ्कलनम्");
    let data = records(&it, "दत्तकोश", "दत्तसूचकाङ्क")
        .iter()
        .map(|r| r.get("अष्टकाः").map(bytes_of).unwrap_or_default())
        .collect();
    let errs = records(&it, "वाक्यविभागदोषकोश", "वाक्यविभागदोषसूचकाङ्क")
        .iter()
        .map(|r| {
            String::from_utf8_lossy(&r.get("कूट").map(bytes_of).unwrap_or_default()).into_owned()
        })
        .collect();
    (data, errs)
}

/// `(data, error text)` from `parse.rs`.
fn rust_assemble(src: &str) -> (Vec<Vec<u8>>, Vec<String>) {
    match parse::assemble_program(src) {
        Ok(p) => (
            p.data
                .iter()
                .filter(|d| d.section != Section::Bss)
                .map(|d| d.bytes.clone())
                .collect(),
            Vec::new(),
        ),
        Err(e) => (Vec::new(), e),
    }
}

#[test]
fn both_assemblers_pack_the_directive_identically() {
    // every letter the emitter may write: the block less the five marks,
    // in runs that cross both halves (२२४ १६४ x and २२४ १६५ x)
    let letters: Vec<char> = (0x0900u32..=0x097F)
        .filter(|c| !matches!(c, 0x093D | 0x0964 | 0x0965 | 0x0970 | 0x0971))
        .map(cp)
        .collect();
    for chunk in letters.chunks(17) {
        let word: String = chunk.iter().collect();
        let src = format!("{DATA}॥ अष्टकाः {OPEN} {word} इति ॥\n");
        let (t1, t1e) = t1_assemble(&src);
        let (rs, rse) = rust_assemble(&src);
        assert!(
            t1e.is_empty() && rse.is_empty(),
            "`{word}`: {t1e:?} {rse:?}"
        );
        assert_eq!(t1, vec![d1(&word)], "`.t1` assembler on `{word}`");
        assert_eq!(rs, vec![d1(&word)], "parse.rs on `{word}`");
    }
    // the doubled close, and the empty literal
    for (body, want) in [("इति इति", d1("इति")), ("", Vec::new())] {
        let src = format!("{DATA}॥ अष्टकाः {OPEN} {body} इति ॥\n");
        let (t1, _) = t1_assemble(&src);
        let (rs, _) = rust_assemble(&src);
        assert_eq!(t1, rs, "`{body}`: the two assemblers agree");
        assert_eq!(rs.concat(), want, "`{body}`");
    }
    // a body that does not pack: a space between two words is P19 on both
    let src = format!("{DATA}॥ अष्टकाः {OPEN} क ख इति ॥\n");
    let (t1, t1e) = t1_assemble(&src);
    let (rs, rse) = rust_assemble(&src);
    assert!(
        t1.is_empty() && rs.is_empty(),
        "nothing is written: {t1:?} {rs:?}"
    );
    assert_eq!(t1e.first().map(String::as_str), Some("P19"), "{t1e:?}");
    // parse.rs renders its refusal: P19's own sentence (spec/diagnostics.tsv,
    // the Sanskrit column), naming the first word as `.t1` names its piece.
    let table = std::fs::read_to_string(spec_root().join("diagnostics.tsv")).expect("diagnostics");
    let p19 = table
        .lines()
        .find_map(|l| l.strip_prefix("P19\t"))
        .and_then(|l| l.split('\t').nth(1))
        .expect("P19's row");
    let k = cp(0x0915).to_string();
    let want = p19.replace("{0}", &k);
    assert!(rse.iter().any(|e| e.contains(&want)), "{rse:?} ∌ {want}");
}

// ── 4. the sarani decoder: the same tables ───────────────────────────────────

/// The embed store a `sarani.t1` text fills, decoded by RUNNING it, against an
/// empty spec root so nothing the host filled is read back.
fn decoded_store(sarani: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
    let dir = spec_fixture::unique_root("adr0044-sarani");
    std::fs::create_dir_all(&dir).expect("an empty spec root");
    let lex = source("lex.t1");
    let mut it = Interpreter::load(&[("lex.t1", lex.as_str()), ("sarani.t1", sarani)], &dir)
        .expect("lex.t1 and sarani.t1 load");
    assert_eq!(it.global("समावेशसंख्या").and_then(Value::as_int), Some(0));
    it.call("सारणीपूरणम्", Vec::new(), 8_000_000_000)
        .expect("सारणीपूरणम् runs");
    let n = it.global("समावेशसंख्या").and_then(Value::as_int).unwrap_or(0);
    let entry = |g: &str, i: usize| match it.global(g) {
        Some(Value::Arena(a)) => bytes_of(&a.borrow()[i]),
        other => panic!("{g} is {other:?}"),
    };
    (0..usize::try_from(n).unwrap())
        .map(|i| (entry("समावेशनामकोश", i), entry("समावेशपाठकोश", i)))
        .collect()
}

#[test]
fn the_sarani_decoder_yields_the_same_tables() {
    let branch = source("sarani.t1");
    assert!(
        branch.contains(&format!("योजनम् {OPEN} ")),
        "sarani.t1 carries its tables as `{OPEN}` literals"
    );
    let got = decoded_store(&branch);
    assert_eq!(got.len(), sadhana::t1::anita::table_names().len());
    for (name, bytes) in &got {
        let name = String::from_utf8_lossy(name).into_owned();
        let rel = sadhana::t1::anita::table_path(&name).expect("a table file");
        let want = std::fs::read(spec_root().join(rel)).expect("the spec file");
        assert_eq!(bytes, &want, "{name}: decoded octets equal spec/{rel}");
    }
    let total: usize = got.iter().map(|(_, b)| b.len()).sum();
    println!("METRIC adr0044_sarani_decoded_octets {total}");
    // base vs branch, octet for octet, when the base module is named
    if let Some(p) = std::env::var_os("ADR0044_BASE_SARANI") {
        let base = std::fs::read_to_string(&p).expect("ADR0044_BASE_SARANI is readable");
        assert!(!base.contains(OPEN), "the base module predates the opener");
        let was = decoded_store(&base);
        assert_eq!(was.len(), got.len(), "the same number of tables");
        for ((wn, wb), (gn, gb)) in was.iter().zip(&got) {
            assert_eq!(wn, gn, "the same table, in the same slot");
            assert_eq!(
                wb,
                gb,
                "{}: base and branch decode alike",
                String::from_utf8_lossy(gn)
            );
        }
        println!(
            "METRIC adr0044_sarani_base_vs_branch identical {} tables",
            got.len()
        );
    }
}

// ── review F1/F2: Unicode whitespace is not a separator on either side ───────

/// The reviewer's nine (`zz_review_adr0044.rs`, r2b): Unicode whitespace outside
/// R-15-1's three layout characters.
const UNICODE_SPACES: [u32; 9] = [
    0x00A0, 0x0085, 0x1680, 0x2003, 0x2028, 0x202F, 0x205F, 0x3000, 0x000B,
];

/// F1: such a character at a literal body's EDGE — where `split_whitespace`
/// used to swallow it as the delimiter's whitespace — is refused by the owner's
/// name on all three readers: `lex.rs`, `व्याकर`, and the interpreter (which
/// lexes with `lex.rs`). It used to be doc 15's repertoire refusal on the Rust
/// side and अदेवनागरीवर्णप्रतिषेधः on the `.t1` side.
#[test]
fn unicode_whitespace_at_a_body_edge_is_the_owners_refusal_on_every_reader() {
    let k = cp(0x0915);
    let mut bad = Vec::new();
    for c in UNICODE_SPACES {
        let ch = cp(c);
        for body in [format!("{ch}{k}"), format!("{k}{ch}"), ch.to_string()] {
            let lit = format!("{OPEN} {body} इति");
            let line = format!("चरः क ॱॱ अङ्कः अन्तः अ८ भवति {lit} ।");
            let lx = lex::lex_t1(&line);
            if !lx
                .as_ref()
                .is_err_and(|e| e.iter().any(|e| e.reason.contains(REFUSAL)))
            {
                bad.push(format!("lex.rs U+{c:04X} `{body:?}`: {lx:?}"));
            }
            let (_, rows) = t1_parse(&program(&lit));
            if !rows.iter().any(|r| {
                String::from_utf8_lossy(&bytes_of(r.get("कारण").expect("कारण"))).contains(REFUSAL)
            }) {
                bad.push(format!(
                    "parse.t1 U+{c:04X} `{body:?}`: {} row(s)",
                    rows.len()
                ));
            }
            let src = program(&lit);
            match Interpreter::load(&[("k.t1", src.as_str())], &spec_root()) {
                Err(e) if e.reason.contains(REFUSAL) => {}
                Err(e) => bad.push(format!("interp U+{c:04X} `{body:?}`: {}", e.reason)),
                Ok(_) => bad.push(format!("interp U+{c:04X} `{body:?}`: loaded")),
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} reader(s) disagree:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// F2: in T0 text the same character was a SILENT separator for `parse.rs`
/// (`॥ अष्टकाः वर्णाष्टकम् <NBSP>क इति ॥` assembled as [0x95]) while `वाक्यविभाग`
/// refused it with P19. Both now refuse: the `.t1` assembler with P19, `lex.rs`
/// at the word (the character is inside it, as the `.t1` lexer keeps it, and
/// R-15-1 refuses it). `उक्तम्` had the same split on the Rust side and is
/// refused the same way; the `.t1` T0 reader has no repertoire gate and still
/// writes the octets for `उक्तम्` — recorded, not fixed here.
#[test]
fn unicode_whitespace_in_a_t0_string_operand_is_never_a_silent_separator() {
    let k = cp(0x0915);
    for c in UNICODE_SPACES {
        let ch = cp(c);
        for opener in [OPEN, "उक्तम्"] {
            for body in [
                format!("{ch}{k}"),
                format!("{k}{ch}"),
                format!("{k}{ch}{k}"),
            ] {
                let src = format!("{DATA}॥ अष्टकाः {opener} {body} इति ॥\n");
                let (rs, rse) = rust_assemble(&src);
                assert!(
                    rs.is_empty() && !rse.is_empty(),
                    "parse.rs accepted `{opener} {body:?}` as {rs:02x?}"
                );
                if opener == OPEN {
                    let (t1, t1e) = t1_assemble(&src);
                    assert!(
                        t1.is_empty() && t1e.first().map(String::as_str) == Some("P19"),
                        "vakyavibhaga on `{body:?}`: {t1:02x?} {t1e:?}"
                    );
                }
            }
        }
    }
}
