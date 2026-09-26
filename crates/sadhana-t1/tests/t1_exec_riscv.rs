//! `W-236` — EMITTER LANE R2: `यन्त्रोत्सर्जन`, the T1 twin of `crates/sadhana/src/t1/riscv64.rs`,
//! EXECUTED under the interpreter, and its text compared with the Rust emitter's OCTET FOR OCTET.
//!
//! research/25 §3.3 names this pair as the third of the twin tables to carry an agreement test
//! and the first whose agreement is measured over the whole corpus (statistic 30, doc 23 §2.8).
//! The IR is built ONCE per test through `मध्यरूप`'s own builder — `यन्त्रयोगफलदृष्टान्तः` is
//! `fixture_recursive_sum()` written with `नवमूल्यम्`, `आज्ञायोजनम्`, `आह्वानाज्ञायोजनम्`,
//! `पर्वयोजनम्` and `वृत्तियोजनम्` — so the emitter is fed the arenas the compiler lays down and
//! not a fixture typed to match.
//!
//! THE REFUSED CASE is a one-octet divergence: `a_one_octet_divergence_names_the_first_differing_line`
//! loads a `yantrotsarjana.t1` with ONE octet changed in the epilogue's return and asserts that the
//! comparison names that line and no other, with both spellings. A comparison that reported
//! "texts differ" would be satisfied by a wrong emitter and a wrong test alike.
//!
//! Also here, because it is the same lane (twin agreement, `W-233`'s finding): the T1 decoder's
//! `उद्धरणम्` lacked the bias step `decode16` gained — `x8..x15` read back as `0..7` — and the test
//! at the foot asserts `सङ्कुचितविश्लेषणम्` and `decode16` now agree on a `c.ld` naming `x8`, with
//! the pre-fix reading reproduced by mutation so the assertion is known to be able to fail.

use sadhana::t1::ast::SymbolId;
use sadhana::t1::ir::{Block, BlockId, CmpOp, Function, Instruction, Terminator, ValueId};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::riscv64::{self, Module, Names, Refusal};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn text_of(v: &Value) -> String {
    match v.octets() {
        Some(o) => String::from_utf8(o.as_slice().to_vec()).expect("the emitted run is UTF-8"),
        None => panic!("{v:?} is not a run of octets"),
    }
}

/// Load `.t1` sources by name, one of them possibly replaced by a mutated text.
fn load_with(names: &[&str], replaced: Option<(&str, &str)>) -> Interpreter {
    // THE LEXER IS PART OF ANY SET THAT READS A TABLE (2026-09-14).
    // `पदविभागॱसमावेशपाठः` is declared in lex.t1 and is how every reader now
    // reaches a spec table, so a list naming `सङ्केतन`, `निदान`, `अक्षरकोश`,
    // `विश्लेषण` or `संयोजन` without the lexer resolves nothing. Added here
    // rather than at each call site because the dependency is the lookup's,
    // not any one test's.
    let mut names: Vec<&str> = names.to_vec();
    if !names.contains(&"lex.t1") {
        names.insert(0, "lex.t1");
    }
    let names: &[&str] = &names;
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| {
            let text = match replaced {
                Some((name, text)) if name == *n => text.to_string(),
                _ => source(n),
            };
            ((*n).to_string(), text)
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

/// The T1 emitter and what it reads: the IR builder and the allocator's module.
const EMITTER: &[&str] = &["ir.t1", "utsarjana.t1", "yantrotsarjana.t1"];

fn call(it: &mut Interpreter, name: &str, args: Vec<Value>, fuel: u64) -> Value {
    it.call(name, args, fuel)
        .unwrap_or_else(|e| panic!("`{name}` runs: {e}"))
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global of the loaded corpus"))
}

fn global_bool(it: &Interpreter, name: &str) -> bool {
    match it.global(name) {
        Some(Value::Bool(b)) => *b,
        other => panic!("`{name}` is a बूल global, not {other:?}"),
    }
}

fn global_text(it: &Interpreter, name: &str) -> String {
    match it.global(name) {
        Some(v @ Value::Octets(_)) => text_of(v),
        other => panic!("`{name}` is a run of octets, not {other:?}"),
    }
}

/// Build the fixture through `मध्यरूप` and emit it through `यन्त्रोत्सर्जन`.
fn emit_fixture(it: &mut Interpreter) -> String {
    let built = call(it, "यन्त्रोत्सर्जनॱयन्त्रयोगफलदृष्टान्तः", vec![], 50_000_000);
    assert_eq!(built.as_int(), Some(2), "the fixture is two routines");
    text_of(&call(it, "यन्त्रोत्सर्जनॱयन्त्रमण्डलोत्सर्जनम्", vec![], 400_000_000))
}

/// The first line where the two texts differ — THE REFUSED MESSAGE — or `None`
/// when they are the same octets.
fn first_divergence(t1: &str, rust: &str) -> Option<String> {
    let a: Vec<&str> = t1.lines().collect();
    let b: Vec<&str> = rust.lines().collect();
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            return Some(format!(
                "line {}: the T1 twin wrote `{x}` where riscv64.rs wrote `{y}`",
                i + 1
            ));
        }
    }
    if a.len() != b.len() {
        return Some(format!(
            "line {}: the T1 twin wrote {} lines, riscv64.rs {}",
            a.len().min(b.len()) + 1,
            a.len(),
            b.len()
        ));
    }
    if t1 != rust {
        return Some("the lines agree and the octets do not (a line ending)".to_string());
    }
    None
}

/// `Refusal`'s variant as the T1 module numbers them (`यन्त्र…निषेधभेद`, १..१०
/// in declaration order).
fn refusal_kind(r: &Refusal) -> i128 {
    match r {
        Refusal::Unreachable { .. } => 1,
        Refusal::NoTerminator { .. } => 2,
        Refusal::TargetNotInFunction { .. } => 3,
        Refusal::UnnamedSymbol { .. } => 4,
        Refusal::LabelCollision { .. } => 5,
        Refusal::ParamAfterCall { .. } => 6,
        Refusal::ParamOutsideEntry { .. } => 7,
        Refusal::FrameTooLarge { .. } => 8,
        Refusal::EntryTakesParameters { .. } => 9,
        Refusal::BranchOutOfRange { .. } => 10,
    }
}

/// `सङ्केतन` on a text, the line named on refusal — riscv64.rs's own test helper.
fn assembles(text: &str) {
    if let Err(ds) = sadhana::assemble_object(
        text,
        Some("परीक्षा"),
        sadhana::encode::Target::Uncompressed,
        false,
        sadhana::nidana::Language::English,
    ) {
        let named: Vec<String> = ds
            .iter()
            .map(|d| {
                let line = text.lines().nth(d.line.saturating_sub(1)).unwrap_or("");
                format!("line {}: `{line}` — {}", d.line, d.reason)
            })
            .collect();
        panic!("सङ्केतन refused the T1 twin's text:\n{}", named.join("\n"));
    }
}

/// **The floor.** Every routine of `यन्त्रोत्सर्जन` has a body this interpreter can
/// run; without this the agreement below could be a comparison of two empty texts.
#[test]
fn the_t1_emitter_loads_with_every_routine_runnable() {
    let it = load_with(EMITTER, None);
    let mine: Vec<_> = it.routines().filter(|r| r.module == "यन्त्रोत्सर्जन").collect();
    let unrunnable: Vec<String> = mine
        .iter()
        .filter(|r| !r.is_runnable())
        .map(|r| format!("{} ({})", r.name, r.why_not().unwrap_or("?")))
        .collect();
    println!("METRIC sadhana_t1_riscv_routines_loaded {}", mine.len());
    println!(
        "METRIC sadhana_t1_riscv_routines_runnable {}",
        mine.len() - unrunnable.len()
    );
    assert!(mine.len() >= 40, "only {} routines loaded", mine.len());
    assert!(
        unrunnable.is_empty(),
        "routines of यन्त्रोत्सर्जन the interpreter cannot run: {unrunnable:?}"
    );
}

/// **THE AGREEMENT, statistic 30 (research/25 §3.3).** The same IR — R1's recursive
/// fixture, built on the T1 side through `मध्यरूप`'s builder and on the Rust side by
/// `fixture_recursive_sum()` — emitted by both twins, and the two texts are the SAME
/// OCTETS. Then the T1 twin's text assembles under `सङ्केतन`, so what agreed is a program.
#[test]
fn the_two_emitters_write_the_same_octets_for_the_recursive_fixture() {
    let mut it = load_with(EMITTER, None);
    let t1 = emit_fixture(&mut it);
    assert!(
        !global_bool(&it, "यन्त्रनिषेधमस्ति"),
        "the T1 twin refused the fixture: kind {}",
        global_int(&it, "यन्त्रनिषेधभेद")
    );
    let rust = riscv64::emit_module(&riscv64::fixture_recursive_sum()).expect("riscv64.rs emits");
    if let Some(named) = first_divergence(&t1, &rust) {
        panic!("REFUSED: the twins diverge — {named}\n--- T1 ---\n{t1}\n--- Rust ---\n{rust}");
    }
    println!("METRIC riscv64_twin_fixture_octets {}", t1.len());
    println!("METRIC riscv64_twin_fixture_lines {}", t1.lines().count());
    println!("METRIC paradigm_encode_twin_agreement_fixture 1/1");
    // The whole point of agreeing on a TEXT: the assembler reads it.
    assembles(&t1);
    // And the frame decision is in the text the T1 side wrote (§2.4): f saves the
    // two स्थिरs it uses, main's two values share one.
    assert!(t1.contains(
        "परीक्षायोगफलम्ॱॱ\nयोगः स्तूपसूचकःम् स्तूपसूचकःन ऋण३२न ।\nनिधानम् स्तूपसूचकःय् २४न पुनःस्थानम्न ।"
    ));
    assert!(t1.contains("परीक्षामुख्यॱॱ\nयोगः स्तूपसूचकःम् स्तूपसूचकःन ऋण१६न ।"));
    // The block label carries the id as Rust counts it (ON INDEX ZERO, ONCE MORE).
    assert!(t1.contains("परीक्षायोगफलम्पर्व१ॱॱ\n"), "{t1}");
    assert!(t1.contains("परीक्षायोगफलम्पर्व२ॱॱ\n"), "{t1}");
}

/// `W-245`: the twins write the same octets for EVERY FORM the row added — the
/// parameter stored to a slot, the loop whose compare is FUSED with its branch
/// (`न्यूनलङ्घनम् … त्`), the eight operators, the six compares as values — built
/// through `मध्यरूप`'s own builder (`यन्त्ररूपदृष्टान्तः`) against `fixture_forms()`,
/// and the agreed text assembles.
#[test]
fn the_two_emitters_write_the_same_octets_for_every_form_of_w245() {
    let mut it = load_with(EMITTER, None);
    let built = call(&mut it, "यन्त्रोत्सर्जनॱयन्त्ररूपदृष्टान्तः", vec![], 80_000_000);
    assert_eq!(built.as_int(), Some(2), "the fixture is two routines");
    let t1 = text_of(&call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रमण्डलोत्सर्जनम्",
        vec![],
        900_000_000,
    ));
    assert!(
        !global_bool(&it, "यन्त्रनिषेधमस्ति"),
        "the T1 twin refused the forms fixture: kind {}",
        global_int(&it, "यन्त्रनिषेधभेद")
    );
    let rust = riscv64::emit_module(&riscv64::fixture_forms()).expect("riscv64.rs emits");
    if let Some(named) = first_divergence(&t1, &rust) {
        panic!("REFUSED: the twins diverge — {named}\n--- T1 ---\n{t1}\n--- Rust ---\n{rust}");
    }
    println!("METRIC riscv64_twin_forms_octets {}", t1.len());
    println!("METRIC paradigm_encode_twin_agreement_forms 1/1");
    assembles(&t1);
    // The fusion is in the text the T1 side wrote: one न्यूनलङ्घनम् with its
    // standard in the ablative, and the local's slot at ० (no spills).
    assert!(
        t1.contains("न्यूनलङ्घनम् स्थिर०न स्थिर१त् परीक्षारूपाणिपर्व२य् ।"),
        "{t1}"
    );
    assert!(t1.contains("निधानम् स्तूपसूचकःय् ०न स्थिर०न ।"), "{t1}");
    for verb in [
        "गुणनम् ",
        "भागः ",
        "शेषः ",
        "वामसरणम् ",
        "दक्षिणसरणम् ",
        "युक्तम् ",
        "विकल्पः ",
        "वैषम्यम् ",
        "अचिह्नन्यूनम् ",
    ] {
        assert!(t1.contains(verb), "the T1 twin wrote no `{verb}`:\n{t1}");
    }
}

/// `W-243`: THE STARTUP OBJECT is a text of its own and the twins agree on it too —
/// the stub calling the fixture's entry `परीक्षामुख्य`, then the stack reservation —
/// and with no entry set (०), the stub that calls nothing and halts success.
#[test]
fn the_two_emitters_write_the_same_startup_object() {
    let mut it = load_with(EMITTER, None);
    call(&mut it, "यन्त्रोत्सर्जनॱयन्त्रयोगफलदृष्टान्तः", vec![], 50_000_000);
    // The fixture sets its entry to मुख्य (symbol 2), as `fixture_recursive_sum` does.
    let t1 = text_of(&call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रारम्भमण्डलोत्सर्जनम्",
        vec![Value::Bool(false)],
        50_000_000,
    ));
    let rust = riscv64::emit_startup_object(Some("परीक्षामुख्य"));
    if let Some(named) = first_divergence(&t1, &rust) {
        panic!(
            "REFUSED: the startup twins diverge — {named}\n--- T1 ---\n{t1}\n--- Rust ---\n{rust}"
        );
    }
    assert!(t1.starts_with("॥ वैश्विकम् यन्त्रारम्भ ॥\nयन्त्रारम्भॱॱ\n"), "{t1}");
    assert!(t1.contains("लङ्घनम् पुनःस्थानम्म् परीक्षामुख्यय् ।"), "{t1}");
    assert!(t1.ends_with("स्तूपःॱॱ\n॥ स्थानम् ६५५३६ ॥\nस्तूपान्तःॱॱ\n"), "{t1}");
    assembles(&t1);
    // And the module text carries no startup any more: its first line is the
    // first routine's exported label.
    let module = emit_fixture(&mut it);
    assert!(
        module.starts_with("॥ वैश्विकम् परीक्षायोगफलम् ॥\nपरीक्षायोगफलम्ॱॱ\n"),
        "{module}"
    );
    assert!(!module.contains("यन्त्रारम्भ"), "{module}");
    // No entry: the stub calls nothing.
    call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रप्रवेशन्यासः",
        vec![Value::Int(0)],
        1_000_000,
    );
    let none = text_of(&call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रारम्भमण्डलोत्सर्जनम्",
        vec![Value::Bool(false)],
        50_000_000,
    ));
    assert_eq!(none, riscv64::emit_startup_object(None));
    assert!(!none.contains("लङ्घनम् पुनःस्थानम्म्"), "{none}");
    println!("METRIC paradigm_encode_twin_agreement_startup 1/1");
}

/// **THE REFUSED CASE: a one-octet divergence names the first differing line.**
/// `०` is `E0 A5 A6` and `१` is `E0 A5 A7`: changing the epilogue's `पुनःस्थानम्त् ०न`
/// to `१न` changes ONE octet of the emitted text, twice (two routines, two returns).
/// The comparison must name the FIRST of the two lines, by number, with both
/// spellings — and nothing before it, because every line before it is the same.
#[test]
fn a_one_octet_divergence_names_the_first_differing_line() {
    let good = source("yantrotsarjana.t1");
    let from = "उक्तम् सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न इति";
    let to = "उक्तम् सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् १न इति";
    assert_eq!(good.matches(from).count(), 1, "the mutation site is unique");
    let wrong = good.replace(from, to);
    let mut it = load_with(EMITTER, Some(("yantrotsarjana.t1", &wrong)));
    let t1 = emit_fixture(&mut it);
    let rust = riscv64::emit_module(&riscv64::fixture_recursive_sum()).expect("riscv64.rs emits");
    assert_eq!(
        t1.len(),
        rust.len(),
        "the mutation changes an octet, not the length"
    );
    let differing: Vec<usize> = t1
        .bytes()
        .zip(rust.bytes())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(differing.len(), 2, "two returns, two octets: {differing:?}");
    let named = first_divergence(&t1, &rust).expect("the texts differ and the test says where");
    let expected_line = rust
        .lines()
        .position(|l| l == "सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।")
        .expect("the first return")
        + 1;
    assert!(
        named.starts_with(&format!("line {expected_line}: ")),
        "the refusal names the wrong line:\n{named}"
    );
    assert!(
        named.contains("पुनःस्थानम्त् १न") && named.contains("पुनःस्थानम्त् ०न"),
        "the refusal shows both spellings:\n{named}"
    );
    println!("REFUSED {named}");
}

/// The twins refuse ALIKE: the same module with its entry pointed at the routine
/// that takes a parameter is `EntryTakesParameters` on both sides, naming the same
/// routine and the same count, and the T1 twin answers no text.
#[test]
fn both_twins_refuse_an_entry_that_takes_parameters_by_the_same_name() {
    let mut it = load_with(EMITTER, None);
    call(&mut it, "यन्त्रोत्सर्जनॱयन्त्रयोगफलदृष्टान्तः", vec![], 50_000_000);
    call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रप्रवेशन्यासः",
        vec![Value::Int(1)],
        1_000_000,
    );
    let t1 = text_of(&call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रमण्डलोत्सर्जनम्",
        vec![],
        400_000_000,
    ));
    assert_eq!(
        t1, "",
        "no text for a module the emitter cannot lower in full"
    );
    assert!(global_bool(&it, "यन्त्रनिषेधमस्ति"));

    let mut m = riscv64::fixture_recursive_sum();
    m.entry = Some(SymbolId(1));
    let rust = riscv64::emit_module(&m).expect_err("riscv64.rs refuses");
    assert_eq!(
        rust,
        Refusal::EntryTakesParameters {
            function: "परीक्षायोगफलम्".into(),
            params: 1,
        }
    );
    assert_eq!(global_int(&it, "यन्त्रनिषेधभेद"), refusal_kind(&rust));
    assert_eq!(
        global_int(&it, "यन्त्रनिषेधभेद"),
        global_int(&it, "यन्त्रप्रवेशप्राचलनिषेधभेद")
    );
    assert_eq!(global_text(&it, "यन्त्रनिषेधवृत्ति"), "परीक्षायोगफलम्");
    assert_eq!(global_int(&it, "यन्त्रनिषेधसंख्या"), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// THE CENSUS — agreement over every IR-built `.t1` source (research/25 §4,
// `paradigm_encode_t1_twin_agreement`). `measure_corpus_ir` builds 11 today.
// ─────────────────────────────────────────────────────────────────────────

fn member(v: &Value, name: &str) -> Value {
    match v {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no member `{name}`")),
        other => panic!("{other:?} is not a record, so it has no `{name}`"),
    }
}

fn int_of(v: &Value, name: &str) -> i128 {
    member(v, name)
        .as_int()
        .unwrap_or_else(|| panic!("`{name}` is a number"))
}

/// `x ॱ क्रमाङ्क` of a `मूल्याङ्क`/`पर्वाङ्क` field, as Rust counts it: १-based → 0-based.
/// `None` for ० — मध्यरूप's ABSENT, which no Rust id can stand for.
fn id_of(v: &Value, name: &str) -> Option<usize> {
    let k = int_of(&member(v, name), "क्रमाङ्क");
    usize::try_from(k).ok().filter(|k| *k > 0).map(|k| k - 1)
}

/// An IR that names the absent value ० where a value is required — the result
/// of a REFUSED call, consumed by the instruction after it. `riscv64.rs` panics
/// on a value with no location and the T1 twin would read register ० for it:
/// neither is an emitter's answer, so such a module is not measured.
#[derive(Debug)]
struct Hole(String);

fn arena(it: &Interpreter, name: &str) -> Rc<std::cell::RefCell<Vec<Value>>> {
    match it.global(name) {
        Some(Value::Arena(a)) => Rc::clone(a),
        other => panic!("`{name}` is an arena, not {other:?}"),
    }
}

/// The module name a `.t1` source declares: `मण्डलम् NAME ॥`.
fn module_name(src: &str) -> String {
    let first = src.lines().next().unwrap_or("");
    first
        .strip_prefix("मण्डलम् ")
        .and_then(|r| r.split(' ').next())
        .unwrap_or("अज्ञात")
        .to_string()
}

/// Read `मध्यरूप`'s arenas into `riscv64.rs`'s `Module`, and write the SAME names
/// into the T1 twin — one IR, one name table, two emitters. Routine `i` is given
/// the symbol `1_000_000 + i` on BOTH sides (the builder leaves every routine's
/// `नाम` at ०, ir.t1's own admission), and every symbol a Call names gets a name
/// from the same rule. Answers the Rust module and the instruction count.
/// The T1 instruction kind's Rust twin for the ten binary kinds and `Cmp`, as `ir.t1`
/// numbers them (`W-245`).
fn binary_kind(kind: i128, l: ValueId, r: ValueId, sub: i128) -> Option<Instruction> {
    Some(match kind {
        3 => Instruction::Add(l, r),
        4 => Instruction::Sub(l, r),
        6 => Instruction::Mul(l, r),
        7 => Instruction::Div(l, r),
        8 => Instruction::Rem(l, r),
        9 => Instruction::Shl(l, r),
        10 => Instruction::Shr(l, r),
        11 => Instruction::And(l, r),
        12 => Instruction::Or(l, r),
        13 => Instruction::Xor(l, r),
        14 => Instruction::Cmp(
            match sub {
                1 => CmpOp::Eq,
                2 => CmpOp::Ne,
                3 => CmpOp::Lt,
                4 => CmpOp::Ge,
                5 => CmpOp::Ltu,
                6 => CmpOp::Geu,
                other => panic!("compare sub-kind {other} is not one of the six"),
            },
            l,
            r,
        ),
        _ => return None,
    })
}

fn read_module(it: &mut Interpreter, module: &str) -> Result<(Module, usize), Hole> {
    let need = |v: &Value, name: &str, at: &str| -> Result<usize, Hole> {
        id_of(v, name).ok_or_else(|| Hole(format!("{at} names the absent value ० as its `{name}`")))
    };
    let functions = arena(it, "वृत्तिकोश");
    let blocks = arena(it, "पर्वकोश");
    let insts = arena(it, "आज्ञाकोश");
    let args = arena(it, "आदानकोश");
    // `W-254`: a string literal's octets, one contiguous run per literal.
    //
    // A RUN OF OCTETS, NOT AN ARENA. `अङ्कः अन्तः अ८` is a `Value::Octets`
    // where `अङ्कः अन्तः मूल्याङ्क` is a `Value::Arena` — so `आदानकोश` above is
    // `Rc`-shared and live, and this is a COPY taken here. Reaching for
    // `arena()` out of symmetry with the line above panics naming the type.
    let string_octets: Vec<u8> = it
        .global("पाठाक्षरकोश")
        .and_then(Value::octets)
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    let count = usize::try_from(global_int(it, "वृत्तिसूचकाङ्क")).unwrap_or(0);
    let mut names = Names::new();
    let mut functions_out = Vec::new();
    let mut inst_count = 0;
    let mut symbols_seen: Vec<i128> = Vec::new();
    for i in 1..=count {
        let f = functions.borrow()[i].clone();
        let sym = SymbolId(1_000_000 + i);
        if let Value::Record(r) = &f {
            r.borrow_mut()
                .insert("नाम".to_string(), Value::Int(sym.0 as i128));
        }
        names.insert(
            sym,
            (
                module.to_string(),
                format!("वृत्ति{}", riscv64::devanagari(i as i64)),
            ),
        );
        let start = usize::try_from(int_of(&f, "पर्वारम्भ")).unwrap_or(0);
        let n = usize::try_from(int_of(&f, "पर्वसंख्यान")).unwrap_or(0);
        let mut out_blocks = HashMap::new();
        for b in start..start + n {
            let blk = blocks.borrow()[b].clone();
            let first = usize::try_from(int_of(&blk, "आज्ञारम्भ")).unwrap_or(0);
            let k = usize::try_from(int_of(&blk, "आज्ञासंख्यान")).unwrap_or(0);
            let mut out_insts = Vec::new();
            for j in first..first + k {
                let ins = insts.borrow()[j].clone();
                let at = format!("instruction {j} of block {b}");
                let v = ValueId(need(&ins, "फलम्", &at)?);
                let inst = match int_of(&ins, "भेद") {
                    1 => Instruction::ConstInt(int_of(&ins, "ध्रुवमूल्यम्") as i64),
                    2 => {
                        let s = int_of(&ins, "संज्ञा");
                        if !symbols_seen.contains(&s) {
                            symbols_seen.push(s);
                        }
                        let a0 = usize::try_from(int_of(&ins, "आदानारम्भ")).unwrap_or(0);
                        let an = usize::try_from(int_of(&ins, "आदानसंख्यान")).unwrap_or(0);
                        let mut list = Vec::new();
                        for q in a0..a0 + an {
                            let arg = args.borrow()[q].clone();
                            let k = int_of(&arg, "क्रमाङ्क");
                            let k =
                                usize::try_from(k).ok().filter(|k| *k > 0).ok_or_else(|| {
                                    Hole(format!("{at} passes the absent value ० as argument {q}"))
                                })?;
                            list.push(ValueId(k - 1));
                        }
                        Instruction::Call(SymbolId(usize::try_from(s).unwrap_or(0)), list)
                    }
                    5 => Instruction::Param(usize::try_from(int_of(&ins, "प्राचलक्रम")).unwrap_or(0)),
                    15 => Instruction::Load(usize::try_from(int_of(&ins, "स्थानक्रम")).unwrap_or(0)),
                    // `W-278`: a module-level global read — the word at the
                    // global's own exported label, named by its symbol.
                    // `W-field`: a record field read — one word at a byte offset from
                    // a RUNTIME base. The offset is carried in `ध्रुवमूल्यम्` and never
                    // in `स्थानक्रम`, which is a FRAME INDEX the emitter scales by ८
                    // and the frame-sizing pass maxes over — a byte offset there is
                    // wrong twice and loud neither time.
                    19 => Instruction::LoadField(
                        ValueId(need(&ins, "वाम", &at)?),
                        u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0),
                    ),
                    // BOTH operands are values here, where the kind above takes one
                    // value and a constant. A decoder that read `ध्रुवमूल्यम्`
                    // for the second would silently decode every index read as an
                    // offset of zero.
                    20 => Instruction::LoadIndex(
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                        match u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0)
                        {
                            0 => 8,
                            w => w,
                        },
                    ),
                    // `W-283`, PART 1 OF THE SYMMETRY: RECORD THE SYMBOL, AS `Call` DOES.
                    // This arm reads the same `संज्ञा` field the `Call` arm above reads, and
                    // that arm pushes it into `symbols_seen` while this one did not — so a
                    // global's symbol reached `emit_module` with no names entry and BOTH
                    // emitters refused `UnnamedSymbol`.
                    //
                    // IT WAS INVISIBLE UNTIL THE SYNTHESISED ENTRY WENT AWAY, because every
                    // source refused at `EntryTakesParameters` first. Measured, 3 sources,
                    // `entry: None` and this line absent: `agree_emit 0/3`, all three
                    // refusing `symbol SymbolId(N) has no name` — ashtaka 4, lex 6,
                    // sanchaya 14. I had predicted 14 would EMIT and exactly two would
                    // refuse, derived from the two sources that SHOWED that refusal while
                    // fourteen refused upstream. A REFUSAL POPULATION MEASURED BEHIND
                    // ANOTHER REFUSAL IS A LOWER BOUND, NEVER A MEMBERSHIP.
                    18 => {
                        let s = int_of(&ins, "संज्ञा");
                        if !symbols_seen.contains(&s) {
                            symbols_seen.push(s);
                        }
                        Instruction::LoadGlobal(SymbolId(usize::try_from(s).unwrap_or(0)))
                    }
                    // `W-283`, the ruled storage model. `वाम` is the ADDRESS in 23
                    // and `दक्षिण` the value stored; swapping them writes the address
                    // into the value's storage, and both are live words, so nothing
                    // downstream would notice.
                    // `W-284`: THE PREDICTION ABOVE FIRED, AND IT IS KEPT VERBATIM BELOW
                    // BECAUSE IT IS WHY THIS COST ONE MESSAGE INSTEAD OF A BISECT.
                    //
                    // W-284's storage model lowers a global store as `AddrOfGlobal` २१ then
                    // `StoreAt` २३ — the first thing in the tree ever to build kind 21. Seven
                    // sources then refused `symbol SymbolId(N) has no name` in BOTH halves,
                    // and `twin_agreement` read a perfect 16/16 because it counts both-refuse
                    // as agreement. The cause was here, exactly as written.
                    //
                    // THE DEFERRAL WAS CORRECT WHEN MADE AND EXPIRED WITHOUT ANNOUNCING IT.
                    // "A fix no test can reach is a claim, not a change" was true while
                    // nothing lowered to 21. It stopped being true the instant W-284 landed.
                    // **A deferral justified by unreachability must name the event that makes
                    // it reachable** — this one did, which is the only reason the expiry was
                    // findable in a margin rather than in a week of bisecting.
                    //
                    // THE PRIOR NOTE, KEPT: "this arm carries a SYMBOL and does not record it
                    // in `symbols_seen`, which is exactly what made arm 18 refuse
                    // `UnnamedSymbol` for every source. It is harmless only because nothing
                    // lowers to kind 21 yet… The first thing that lowers to `AddrOfGlobal`
                    // will refuse here, and the cause will be in this file rather than in the
                    // lowering. Record the symbol then, as arm 18 does."
                    //
                    // AND THE CLASS IS STILL OPEN, DELIBERATELY NOT CLOSED HERE. This table is
                    // SYNTHESISED from a hand-written per-kind walk; `chain.rs:546` instead
                    // walks the resolver's scopes and names every symbol the resolver bound,
                    // whatever kind references it. That approach cannot fail this way and is
                    // fourteen lines from a file this one already imports. It is not done in
                    // this commit because a census is about to be read off this instrument,
                    // and a harness that names MORE symbols in the same change as the
                    // measurement cannot be told apart from a real fall.
                    21 => {
                        let s = int_of(&ins, "संज्ञा");
                        if !symbols_seen.contains(&s) {
                            symbols_seen.push(s);
                        }
                        Instruction::AddrOfGlobal(SymbolId(usize::try_from(s).unwrap_or(0)))
                    }
                    22 => Instruction::LoadAt(ValueId(need(&ins, "वाम", &at)?)),
                    23 => Instruction::StoreAt(
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                    ),
                    // `W-284` — THE STORAGE THE OTHER FIVE KINDS ADDRESS. 24
                    // allocates it, 25 and 26 form addresses INTO it, and 22/23
                    // spend them. The size is a build-time constant in
                    // `ध्रुवमूल्यम्` and there are NO value operands.
                    24 => Instruction::AllocRecord(
                        u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0),
                    ),
                    // 25 takes a value and a CONSTANT where 26 takes two values —
                    // the same asymmetry as 19 against 20. A decoder reading
                    // `ध्रुवमूल्यम्` for 26's second operand would make every index
                    // address offset zero, silently.
                    25 => Instruction::AddrOfField(
                        ValueId(need(&ins, "वाम", &at)?),
                        u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0),
                    ),
                    26 => Instruction::AddrOfIndex(
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                    ),
                    16 => Instruction::Store(
                        usize::try_from(int_of(&ins, "स्थानक्रम")).unwrap_or(0),
                        ValueId(need(&ins, "वाम", &at)?),
                    ),
                    // `W-254`: a string literal's octets, copied out of
                    // `पाठाक्षरकोश` by the run the instruction names. The arena
                    // is 1-BASED, as `आदानकोश` is, so the run starts at
                    // `पाठारम्भ` itself. A run past the end is a HOLE, not a
                    // truncation: a short string is a wrong image, not a loud one.
                    17 => {
                        let start = usize::try_from(int_of(&ins, "पाठारम्भ")).unwrap_or(0);
                        let count = usize::try_from(int_of(&ins, "पाठसंख्यान")).unwrap_or(0);
                        let end = start + count;
                        let bytes = string_octets.get(start..end).ok_or_else(|| {
                            Hole(format!(
                                "{at} names octets {start}..{end} and पाठाक्षरकोश holds {}",
                                string_octets.len()
                            ))
                        })?;
                        Instruction::ConstStr(bytes.to_vec())
                    }
                    kind => binary_kind(
                        kind,
                        ValueId(need(&ins, "वाम", &at)?),
                        ValueId(need(&ins, "दक्षिण", &at)?),
                        int_of(&ins, "उपभेद"),
                    )
                    .unwrap_or_else(|| panic!("instruction kind {kind} is not one of the sixteen")),
                };
                out_insts.push((v, inst));
                inst_count += 1;
            }
            let term = member(&blk, "अवसानम्");
            let terminator = match int_of(&term, "भेद") {
                0 => None,
                1 => {
                    let k = int_of(&member(&term, "मूल्यम्"), "क्रमाङ्क");
                    Some(Terminator::Return(if k > 0 {
                        Some(ValueId(k as usize - 1))
                    } else {
                        None
                    }))
                }
                2 => Some(Terminator::Branch(BlockId(need(
                    &term,
                    "लक्ष्यम्",
                    &format!("the terminator of block {b}"),
                )?))),
                3 => Some(Terminator::Unreachable),
                4 => {
                    let at = format!("the terminator of block {b}");
                    Some(Terminator::CondBranch(
                        ValueId(need(&term, "मूल्यम्", &at)?),
                        BlockId(need(&term, "लक्ष्यम्", &at)?),
                        BlockId(need(&term, "अन्यलक्ष्यम्", &at)?),
                    ))
                }
                other => panic!("terminator kind {other} is not one of the four"),
            };
            let id = BlockId(b - 1);
            out_blocks.insert(
                id,
                Block {
                    id,
                    insts: out_insts,
                    terminator,
                },
            );
        }
        functions_out.push(Function {
            name: sym,
            blocks: out_blocks,
            entry_block: BlockId(need(&f, "प्रवेशपर्व", &format!("routine {i}"))?),
        });
    }
    for s in &symbols_seen {
        let name = if *s == 0 {
            "बाह्यम्".to_string()
        } else {
            format!("संज्ञा{}", riscv64::devanagari(*s as i64))
        };
        names.insert(
            SymbolId(usize::try_from(*s).unwrap_or(0)),
            (module.to_string(), name),
        );
    }
    // The same table, written into the T1 twin.
    call(it, "यन्त्रोत्सर्जनॱयन्त्रनामारम्भः", vec![], 1_000_000);
    let mut sorted: Vec<(&SymbolId, &(String, String))> = names.iter().collect();
    sorted.sort_by_key(|(s, _)| s.0);
    for (sym, (m, n)) in sorted {
        call(
            it,
            "यन्त्रोत्सर्जनॱयन्त्रनामयोजनम्",
            vec![Value::Int(sym.0 as i128), octets(m), octets(n)],
            10_000_000,
        );
    }
    // ── `W-283`: NO ENTRY, SO THE COMPARISON COMPARES OCTETS ─────────────────────
    //
    // This was `(count >= 1).then_some(SymbolId(1_000_001))` — a SYNTHESISED entry on
    // every module with at least one routine — and it is why `twin_agreement` read
    // 16/16 while comparing nothing. `emit_module` refuses with `EntryTakesParameters`
    // when the named entry takes parameters, because the startup stub calls it with
    // none; measured on the corpus, every source refused there and the metric counted
    // each refusal as an AGREEMENT:
    //
    //     16 of 16 agreements were `both refuse`, 0 were `both emit`
    //     14 on this entry limit, 2 on `UnnamedSymbol`
    //
    // THE ENTRY CONTRIBUTES NOTHING TO THE EMITTED TEXT, so dropping it loses no
    // coverage. `riscv64.rs:1215` is `let _ = entry_label;` under the margin "The
    // entry's label is the startup object's business (`emit_startup_object`); it was
    // computed above for the parameter check." The text is the same octets either way.
    //
    // AND BOTH SIDES AGREE THIS IS THE RIGHT SHAPE. `shrinkhala.t1`, the self-hosted
    // driver, says of itself: "NO ENTRY IS NAMED, DELIBERATELY… the stub's own margin
    // says `०` gives an image whose stub halts success." `yantrotsarjana.t1:1740` puts
    // the entry in `यन्त्रारम्भमण्डलोत्सर्जनम्`, the twin of `emit_startup_object`.
    // The synthesised entry here was the anomaly, not the absence of one.
    //
    // THE CHANGE IS SYMMETRIC ON PURPOSE. `यन्त्रप्रवेशन्यासः` told the T1 side the
    // same entry, so both refused together. Setting this to `None` without dropping
    // that call would leave Rust emitting while T1 refused — a DIVERGENCE MANUFACTURED
    // BY THE HARNESS, which is worse than the vacuous agreement it replaces.
    //
    // `EntryTakesParameters` keeps its own coverage: a purpose-built module asserts it
    // directly further down this crate's Rust twin (`riscv64.rs`, the test naming
    // `परीक्षामुख्य` and `params: 1`). It never needed the corpus to exercise it.
    let entry: Option<SymbolId> = None;
    // ── `W-283`, PART 2 OF THE SYMMETRY: TELL RUST THE GLOBALS T1 ALREADY KNOWS ──
    //
    // This was `globals: Vec::new()`, and it is the asymmetry that survived the other
    // two. `riscv64.rs:1132` emits three lines PER ENTRY of this list — `॥ वैश्विकम्
    // {label} ॥`, `{label}ॱॱ`, `॥ अष्टाष्टकाः {init} ॥` — and then the string pool at
    // `पाठकोशःॱॱ`. With an empty list Rust went straight to the pool while the T1 twin
    // emitted the declarations, which is exactly the divergence measured:
    //
    //   ashtaka.t1  line  446: T1 `॥ वैश्विकम् अष्टकअष्टककोश ॥`   Rust `पाठकोशःॱॱ`
    //
    // THE HARNESS ALREADY READS THE T1 ARENAS FOR `functions` AND `names` AND STOPPED
    // SHORT OF `globals`. `yantrotsarjana.t1:1786-1808` walks
    // `मध्यरूपॱवैश्विकसञ्चयसूचकाङ्क` and reads three arenas per global — module name,
    // name, initial value — so the same four reads give Rust the same list. The label
    // is module and name CONCATENATED, as `:1790-1791` writes them.
    //
    // The arenas are 1-BASED and walked TO THE CURSOR, not to the length: a length walk
    // reads the tail a previous caller left. A short arena is a Hole naming the index
    // rather than a panic, because this is a census and one bad source must not take
    // the other nineteen with it.
    let g_count = usize::try_from(global_int(it, "वैश्विकसञ्चयसूचकाङ्क")).unwrap_or(0);
    let g_modules = arena(it, "वैश्विकमण्डलकोश");
    let g_names = arena(it, "वैश्विकनामकोश");
    let g_values = arena(it, "वैश्विकमूल्यकोश");
    let g_octets = arena(it, "वैश्विकसामर्थ्यकोश");
    // `W-293` — THE THIRD ELEMENT IS THE OCTETS, AND THIS HARNESS ONLY CARRIES IT.
    // `मध्यरूप` decides it (`ir.t1:2793`); the fourth arena is read exactly as the
    // other three are. Widened here because the twin comparison this file performs
    // is what holds the two emitters octet-exact, and it cannot run if the harness
    // constructs a `Module` the emitter no longer accepts.
    let mut globals_out: Vec<(String, i64, i64)> = Vec::new();
    for i in 1..=g_count {
        let at = format!("global {i} of {g_count}");
        let m = g_modules
            .borrow()
            .get(i)
            .map(text_of)
            .ok_or_else(|| Hole(format!("{at}: वैश्विकमण्डलकोश holds no entry")))?;
        let n = g_names
            .borrow()
            .get(i)
            .map(text_of)
            .ok_or_else(|| Hole(format!("{at}: वैश्विकनामकोश holds no entry")))?;
        let v = g_values
            .borrow()
            .get(i)
            .and_then(Value::as_int)
            .ok_or_else(|| Hole(format!("{at}: वैश्विकमूल्यकोश holds no value")))?;
        let octets = g_octets
            .borrow()
            .get(i)
            .and_then(Value::as_int)
            .unwrap_or(8) as i64;
        globals_out.push((
            format!("{m}{n}"),
            v as i64,
            if octets < 8 { 8 } else { octets },
        ));
    }
    Ok((
        Module {
            globals: globals_out,
            // `W-283`: `name` IS INERT ON BOTH SIDES AND IS LEFT THAT WAY DELIBERATELY.
            // `emit_module` never reads `module.name` — only `module.names`, twice. And the
            // registered twin (`t1_sources.rs:2103`, `("Module::name", "यन्त्रमण्डलनाम")`)
            // is declared at `yantrotsarjana.t1:160`, assigned only inside two T1 test
            // routines, and READ NOWHERE. So not transporting it costs nothing today.
            //
            // WIRING IT WOULD BE INVENTING BEHAVIOUR TO SATISFY A PAIRING. A registered
            // pair whose two halves both do nothing is a declaration with no mechanism
            // behind it, and the honest repair is to say so here rather than give the field
            // a job so the table looks complete. If either side ever reads it, THAT is when
            // it must be transported, and this margin is the note to come back to.
            name: module.to_string(),
            functions: functions_out,
            names,
            entry,
        },
        inst_count,
    ))
}

/// THE CENSUS. Over every `.t1` source: lex → parse → resolve → IR under the
/// interpreter (as `measure_corpus_ir` does), then BOTH emitters over the one IR,
/// and `METRIC paradigm_encode_twin_agreement <n>/<m>` — `m` the sources whose IR
/// was built, `n` those where the two texts are the same octets (or both twins
/// refused by the same kind). A divergence is printed with its first line.
///
/// # WHAT A GREEN HERE DOES NOT MEAN — READ THIS BEFORE QUOTING THE NUMBER
///
/// **This test proves the two emitters AGREE. It cannot prove either is CORRECT.**
/// Both could be wrong in the same way and this census would be perfectly green, and
/// on this corpus that is not a hypothetical: both are handed IR from the SAME builder
/// (`मध्यरूप`), so a fault in the IR reaches both identically, and until `W-283` both
/// refused the same sixteen sources for the same reason and the metric scored it 16/16.
/// **Agreement between two things fed by one source is a weaker claim than it looks.**
///
/// THE INDEPENDENT WITNESSES ARE ELSEWHERE, and a correctness claim needs one of them:
///
/// * `assembles(&text)` — `सङ्केतन` accepts the emitted text, so what agreed is
///   something the assembler will take, not merely two matching strings.
/// * `words == words_back` through `decode_at`/`reassemble` — a differently-derived
///   witness rather than a second opinion from the same family.
/// * **a source whose right answer is known, asserted BY VALUE.** `riscv64_emitter.rs`
///   asserts the recursive fixture returns `15`; a status of `0` proves nothing, because
///   an image that does nothing at all halts `0`.
///
/// So: `twin_agreement_emit` rising is evidence the emitters track each other over real
/// generated code. It is not evidence the code is right. Do not let a green here stand
/// in for a value assertion — [`the_t1_emitter_loads_with_every_routine_runnable`] above
/// exists for the same reason, and its margin says the same thing about empty texts.
///
/// # AND A DIVERGENCE HERE IS A SUCCESS, NOT A REGRESSION
///
/// **A guard that finally works and reports a divergence has done its job.** Sixteen
/// sources went uncompared for the whole life of this metric; the first run that compares
/// them may well find the two emitters genuinely disagree, and that is the *reason* to
/// build the guard rather than a reason to distrust it. Report the first differing line,
/// not a tally, and do not treat a non-zero divergence as something to be fixed back to
/// zero before it has been read.
///
/// **What `W-283` can promise is narrower than a green, and it is the whole point:** after
/// the module is symmetric, a divergence is attributable to the EMITTERS rather than to
/// the harness. It was not before. Three asymmetries — a synthesised entry, an unrecorded
/// `LoadGlobal` symbol, an empty globals list — each masked the next, so every earlier
/// reading of this metric was about the harness and not about the emitters at all.
#[test]
#[ignore = "measurement: the whole corpus through the interpreter, minutes"]
fn measure_corpus_twin_emit() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    // `W-281`: NINE, and `sanchaya.t1` is the ninth. This list held eight until
    // 2026-09-10 — the same eight `pradarshana`'s `CHAIN` held, element for element,
    // and the same defect. `artha.t1:153` imports the module as `घोषणासञ्चय` and uses
    // it at 13 sites; unloaded, `घोषणासञ्चयॱप्रविष्टिभेदः` parses as a FIELD ACCESS on
    // a local of that name, which is legal, and then resolves wrong. None of the 13
    // uses the `आरभ्य … ऽ … समाप्तम्` call form, whose `ऽ` is the only thing that makes
    // the mis-parse a syntax error, so every site failed silently and this census was
    // green while mis-resolving a module it loads the chain to exercise.
    //
    // ── CORRECTION, 2026-09-11: "every site failed silently" IS FALSE, AND THIS TEST
    // ── IS WHAT MEASURED IT. The sentence above stays as the witness.
    //
    // It does not raise at PARSE. It DOES raise at RESOLVE. This census run at eight
    // modules against nine, same tree, one variable:
    //
    //     8 modules   t1_ir_built  4   twin_agreement  4/4    14 of 20 not reached
    //     9 modules   t1_ir_built 16   twin_agreement 16/16    1 of 20 not reached
    //
    // Fourteen sources refused with `resolve: "घोषणासञ्चयॱमण्डलान्वेषणम् is not a name
    // in scope"` — `artha.t1:861`, the first of the 13, refused BY NAME one stage after
    // parse. Absence of one failure shape is not silence: every later stage is another
    // chance to catch it, and the `ऽ` reasoning stopped at the first.
    //
    // AND THE TWO AGREEMENT FIGURES ARE THE REASON TO DISTRUST THIS METRIC'S SHAPE.
    // `4/4` and `16/16` are both a perfect score, four times apart — because the
    // denominator is sources that BUILT IR, so a source that never resolves leaves the
    // ratio at 100% instead of lowering it. Worse, at nine modules ALL SIXTEEN
    // agreements are `both refuse` and NONE is `both emit`: 14 on the harness's own
    // stub-entry limit ("the entry takes N parameters; the stub passes none") and 2 on
    // `symbol SymbolId(N) has no name`. So the headline `16/16` compares no octets at
    // all — it records that the two emitters refuse identically at the door. Report the
    // split (both-emit / both-refuse / divergent), never the bare ratio.
    //
    // `names` above already holds the real corpus, read off the directory and sorted.
    // It is not used here because this list is an ORDER, not a set — but that means the
    // true names were in hand while a hand-written copy of them drifted, which is the
    // shape `t1_driver.rs:87`'s margin warns about: "a hand-written list here would pass
    // this test while `CHAIN` was missing a module." It did, in two other files.
    let chain = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "artha.t1",
        "sanchaya.t1",
        "sanskrit_text.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
    ];
    let started = std::time::Instant::now();
    let (mut built, mut agree) = (0, 0);
    let (mut partial_built, mut partial_agree) = (0, 0);
    // `W-283`: THE SPLIT, because `agree` alone cannot be read. Its numerator counts
    // "the same octets" and "both refused by the same kind" together, so a corpus that
    // never reaches the emitter scores a perfect ratio. Measured before this change:
    // `16/16` with `agree_emit` 0. These two are printed beside it and must be.
    let (mut agree_emit, mut agree_refuse) = (0, 0);
    let mut unmeasurable = 0;
    let mut divergent: Vec<String> = Vec::new();
    for n in &names {
        let src = std::fs::read_to_string(dir.join(n)).unwrap();
        let mut it = load_with(&chain, None);
        let Ok(toks) = it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        else {
            eprintln!("  {n:24} not reached (lex)");
            continue;
        };
        let parsed = match it.call(
            "व्याकरॱकार्यक्रमपठनम्",
            vec![Value::Int(toks.as_int().unwrap_or(0))],
            4_000_000_000,
        ) {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(e) => {
                eprintln!("  {n:24} not reached (parse): {:.100}", format!("{e:?}"));
                continue;
            }
        };
        if parsed == 0 {
            eprintln!("  {n:24} not reached (parse produced no declarations)");
            continue;
        }
        let resolved = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .and_then(|r| {
                it.call(
                    "अर्थॱकार्यक्रमनिर्णयः",
                    vec![r, Value::Int(parsed)],
                    4_000_000_000,
                )
            });
        if !matches!(resolved, Ok(Value::Bool(true))) {
            eprintln!("  {n:24} not reached (resolve: {resolved:?})");
            continue;
        }
        if it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000).is_err() {
            eprintln!("  {n:24} मध्यरूपॱआरम्भः would not run");
            continue;
        }
        if let Err(e) = it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
        {
            eprintln!("  {n:24} IR RUN ERROR {:.80}", format!("{e:?}"));
            continue;
        }
        // A FOURTH STATE (`W-204`): the builder ran and REFUSED a call whose
        // callee carries no symbol — today a name called inside an `अन्यथा`
        // body, which the resolver does not walk until `W-202`. The build is
        // not usable, and `measure_corpus_ir` counts it as broke. The ARENAS
        // are still well-formed (the refused call appended nothing), so the
        // twins are measured over them too, under their own metric: emitter
        // agreement is a fact about the emitters, whatever the IR's standing.
        let partial = matches!(it.global("अनिर्णीताह्वानमस्ति"), Some(Value::Bool(true)));
        let count = global_int(&it, "वृत्तिसूचकाङ्क");
        if count == 0 {
            eprintln!("  {n:24} IR EMPTY — no routine built");
            continue;
        }
        let module = module_name(&src);
        let (rust_module, inst_count) = match read_module(&mut it, &module) {
            Ok(read) => read,
            Err(Hole(why)) => {
                // Only a PARTIAL build can reach here: a refused call answered the
                // absent value and the next instruction consumed it.
                println!("RUN {n:24} routines {count:>3} PARTIAL, UNMEASURABLE: {why}");
                unmeasurable += 1;
                continue;
            }
        };
        if partial {
            partial_built += 1;
        } else {
            built += 1;
        }
        let rust = riscv64::emit_module(&rust_module);
        let t1_started = std::time::Instant::now();
        let t1 = match it.call("यन्त्रोत्सर्जनॱयन्त्रमण्डलोत्सर्जनम्", vec![], 60_000_000_000)
        {
            Ok(v) => text_of(&v),
            Err(e) => {
                eprintln!("  {n:24} T1 EMITTER RUN ERROR {:.120}", format!("{e:?}"));
                divergent.push(format!("{n}: the T1 twin did not finish: {e:?}"));
                continue;
            }
        };
        let t1_secs = t1_started.elapsed().as_secs_f64();
        let t1_refused = global_bool(&it, "यन्त्रनिषेधमस्ति");
        let mut agreed = || {
            if partial {
                partial_agree += 1;
            } else {
                agree += 1;
            }
        };
        let verdict = match (&rust, t1_refused) {
            (Ok(text), false) => match first_divergence(&t1, text) {
                None => {
                    agreed();
                    agree_emit += 1;
                    format!("AGREE {} octets", text.len())
                }
                Some(d) => {
                    divergent.push(format!("{n}: {d}"));
                    format!("DIVERGE {d}")
                }
            },
            (Err(r), true) => {
                let k = global_int(&it, "यन्त्रनिषेधभेद");
                if k == refusal_kind(r) {
                    agreed();
                    agree_refuse += 1;
                    format!("AGREE both refuse: {r}")
                } else {
                    divergent.push(format!("{n}: Rust refused {r}, T1 kind {k}"));
                    format!("DIVERGE Rust refused {r}, T1 kind {k}")
                }
            }
            (Ok(_), true) => {
                let k = global_int(&it, "यन्त्रनिषेधभेद");
                divergent.push(format!("{n}: only the T1 twin refused, kind {k}"));
                format!("DIVERGE only T1 refused, kind {k}")
            }
            (Err(r), false) => {
                divergent.push(format!("{n}: only riscv64.rs refused: {r}"));
                format!("DIVERGE only Rust refused: {r}")
            }
        };
        let standing = if partial {
            "PARTIAL (a call refused)"
        } else {
            "BUILT"
        };
        println!(
            "RUN {n:24} routines {count:>3} insts {inst_count:>5} T1 {t1_secs:6.1}s  {standing:24} {verdict}"
        );
    }
    println!("METRIC paradigm_encode_t1_sources {}", names.len());
    println!("METRIC paradigm_encode_t1_ir_built {built}");
    println!("METRIC paradigm_encode_t1_ir_partial {partial_built}");
    println!("METRIC paradigm_encode_t1_ir_partial_unmeasurable {unmeasurable}");
    println!("METRIC paradigm_encode_twin_agreement {agree}/{built}");
    // `W-283`: THE SPLIT, AND IT IS NOT OPTIONAL READING. `twin_agreement` sums two
    // unlike things — sources where both emitters wrote the SAME OCTETS, and sources
    // where both REFUSED for the same reason. Only the first compares generated code.
    // Before this unit the corpus read `16/16` with `agree_emit` 0: a perfect score in
    // which nothing was compared. Quote these two, never the ratio alone.
    println!("METRIC paradigm_encode_twin_agreement_emit {agree_emit}/{built}");
    println!("METRIC paradigm_encode_twin_agreement_refuse {agree_refuse}/{built}");
    println!("METRIC paradigm_encode_twin_agreement_partial {partial_agree}/{partial_built}");
    println!(
        "METRIC paradigm_encode_twin_emit_seconds {:.0}",
        started.elapsed().as_secs_f64()
    );
    assert!(
        divergent.is_empty(),
        "the twins diverge on {} of {built} IR-built sources:\n  {}",
        divergent.len(),
        divergent.join("\n  ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `W-233`'s FINDING, FIXED HERE (the same twin-agreement lane): the T1 decoder's
// `उद्धरणम्` lacked the bias step `decode16` has, so `x8..x15` read as `0..7`.
// ─────────────────────────────────────────────────────────────────────────

/// `c.ld s0, 0(s1)` = 0x6080 — `vishlesana.rs`'s own constant: both three-bit
/// register fields are biased by 8, so the bits say 0 and 1 and the registers are
/// `x8` and `x9`.
const C_LD_S0_S1: u32 = 0x6080;

/// The operand VALUES `सङ्कुचितविश्लेषणम्` answers, in slot order.
fn t1_operand_values(it: &mut Interpreter, word: u32) -> Vec<i128> {
    let d = call(
        it,
        "विश्लेषणॱसङ्कुचितविश्लेषणम्",
        vec![Value::Int(i128::from(word))],
        600_000_000,
    );
    assert!(!d.is_nil(), "सङ्कुचितविश्लेषणम् answered शून्यम् for 0x{word:04x}");
    let cells: Vec<Value> = match member(&d, "मूल्यानि") {
        Value::Arena(a) => a.borrow().clone(),
        other => panic!("मूल्यानि is an arena, not {other:?}"),
    };
    cells
        .iter()
        .filter(|c| !c.is_nil())
        .map(|c| int_of(c, "मूल्यम्"))
        .collect()
}

#[test]
fn the_compressed_decoder_twin_names_x8_as_decode16_does() {
    let rust = sadhana::vishlesana::decode16(C_LD_S0_S1 as u16).expect("decode16 decodes c.ld");
    assert_eq!(rust.insn, "c.ld");
    let rust_values: Vec<i128> = rust.operands.iter().map(|(_, v)| i128::from(*v)).collect();
    let rust_regs: Vec<i128> = rust
        .operands
        .iter()
        .filter(|(k, _)| k == "reg")
        .map(|(_, v)| i128::from(*v))
        .collect();
    assert_eq!(rust_regs, vec![8, 9], "decode16 names s0 and s1");

    let mut it = load_with(&["encode.t1", "vishlesana.t1"], None);
    let t1_values = t1_operand_values(&mut it, C_LD_S0_S1);
    assert_eq!(
        t1_values, rust_values,
        "सङ्कुचितविश्लेषणम् and decode16 disagree on the operands of c.ld s0, 0(s1)"
    );
    println!("METRIC sadhana_t1_decoder_twin_c_ld_x8 1");

    // REFUSED — the reading before the fix, reproduced so the assertion above is
    // known to be able to fail: with the bias step disabled (`आधार समम् ०` made
    // always true), the same word names `x0` and `x1`.
    let good = source("vishlesana.t1");
    let from = "यदि अवकाशः ॱ आधार समम् ० आदि";
    let to = "यदि अवकाशः ॱ आधार बृहत्समम् ० आदि";
    assert_eq!(good.matches(from).count(), 1, "the mutation site is unique");
    let wrong = good.replace(from, to);
    let mut broken = load_with(
        &["encode.t1", "vishlesana.t1"],
        Some(("vishlesana.t1", &wrong)),
    );
    let before = t1_operand_values(&mut broken, C_LD_S0_S1);
    assert_ne!(
        before, rust_values,
        "without the bias step the twin must disagree"
    );
    let regs_before: Vec<i128> = before
        .iter()
        .zip(rust.operands.iter())
        .filter(|(_, (k, _))| k == "reg")
        .map(|(v, _)| *v)
        .collect();
    assert_eq!(
        regs_before,
        vec![0, 1],
        "W-233's defect: s0 read as x0, s1 as x1"
    );
    println!(
        "REFUSED pre-fix reading of 0x6080: registers {regs_before:?}, decode16 says {rust_regs:?}"
    );
}
