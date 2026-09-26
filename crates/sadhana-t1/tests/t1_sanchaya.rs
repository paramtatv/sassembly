//! `W-223` part 1 — THE SHARED DECLARATION STORE, `sanchaya.t1` (घोषणासञ्चय):
//! every parse's declarations copied, as text, into one store keyed by module,
//! and a qualified member looked up against what the module DECLARES.
//!
//! Before this row a qualified use `मण्डलॱनाम` was checked against the import
//! alone (W-209: 256 taken on trust); there was no store to ask. These tests
//! drive the store under the interpreter on small programs: what it holds
//! after a parse, the two-file module (W-224's `वास्तु`), and the REFUSED
//! cases — a member the module does not declare, and a module the store does
//! not hold — each answered ० with its kind and names recorded.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// The lexer, the AST, the parser (and the text helper it imports), and the store.
fn store() -> Interpreter {
    let names = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
    ];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root())
        .unwrap_or_else(|e| panic!("the parser and the store load: {e:?}"));
    it.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000)
        .expect("आरम्भः runs");
    it
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("a run of octets, not {other:?}"),
    }
}

fn int_of(v: &Value) -> i128 {
    v.as_int()
        .unwrap_or_else(|| panic!("an integer, not {v:?}"))
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("{name} is an integer"))
}

/// Lex and parse `src`, then collect its declarations into the store; the
/// number of entries the collector added.
fn collect(it: &mut Interpreter, src: &str) -> i128 {
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("पदविभाग runs");
    it.call("व्याकरॱकार्यक्रमपठनम्", vec![toks], 4_000_000_000)
        .expect("कार्यक्रमपठनम् runs");
    assert_eq!(global_int(it, "दोषसूचकाङ्क"), 0, "the program parses: {src}");
    int_of(
        &it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 400_000_000)
            .expect("सङ्ग्रहः runs"),
    )
}

/// `(kind, name, type, [(member, type)…], public)` of entry `i`.
fn entry(it: &mut Interpreter, i: i128) -> (i128, String, String, Vec<(String, String)>, bool) {
    let call = |it: &mut Interpreter, f: &str, args: Vec<Value>| {
        it.call(&format!("घोषणासञ्चयॱ{f}"), args, 10_000_000)
            .unwrap_or_else(|e| panic!("{f} runs: {e:?}"))
    };
    let kind = int_of(&call(it, "प्रविष्टिभेदः", vec![Value::Int(i)]));
    let name = text_of(&call(it, "प्रविष्टिनाम", vec![Value::Int(i)]));
    let ty = text_of(&call(it, "प्रविष्टिप्रकारः", vec![Value::Int(i)]));
    let n = int_of(&call(it, "प्रविष्टिप्राचलसंख्या", vec![Value::Int(i)]));
    let public = matches!(
        call(it, "प्रविष्टिसार्वजनिकत्वम्", vec![Value::Int(i)]),
        Value::Bool(true)
    );
    let members = (0..n)
        .map(|k| {
            (
                text_of(&call(
                    it,
                    "प्राचलप्रविष्टिनाम",
                    vec![Value::Int(i), Value::Int(k)],
                )),
                text_of(&call(
                    it,
                    "प्राचलप्रविष्टिप्रकारः",
                    vec![Value::Int(i), Value::Int(k)],
                )),
            )
        })
        .collect();
    (kind, name, ty, members, public)
}

fn lookup(it: &mut Interpreter, module: &str, member: &str) -> i128 {
    int_of(
        &it.call(
            "घोषणासञ्चयॱसदस्यान्वेषणम्",
            vec![octets(module), octets(member)],
            50_000_000,
        )
        .expect("सदस्यान्वेषणम् runs"),
    )
}

fn imported(it: &mut Interpreter, from: &str, module: &str) -> bool {
    matches!(
        it.call(
            "घोषणासञ्चयॱआयातितम्",
            vec![octets(from), octets(module)],
            50_000_000
        )
        .expect("आयातितम् runs"),
        Value::Bool(true)
    )
}

const PROGRAM: &str = "मण्डलम् क ॥
आयातः ख ।
सार्वजनिक संरचना बिन्दु आरभ्य
    अ ॱॱ अ६४ ऽ
    ब ॱॱ अङ्कः अन्तः अ८
समाप्तम् ।
सार्वजनिक गणना रङ्ग आरभ्य रक्त ऽ नील समाप्तम् ।
सार्वजनिक चरः गणकः ॱॱ न६४ भवति ० ।
चरः गुप्तः ॱॱ सम्भाव्य बिन्दु भवति ० ।
सार्वजनिक वृत्तिः योगः आदाय प ॱॱ अ६४ ऽ फ ॱॱ स्थानम् बिन्दु ददाति दोषयुक्त अ६४ आदि
    प्रत्यागमनम् प ।
इति
";

/// One parse, every declaration in the store, as text: kind, name, type and run.
#[test]
fn a_parse_is_collected_with_its_kinds_names_types_and_runs() {
    let mut it = store();
    assert_eq!(
        collect(&mut it, PROGRAM),
        8,
        "eight declarations: the import, the struct, the enum and its two variants, two globals, the routine"
    );
    assert_eq!(global_int(&it, "मण्डलसूचकाङ्क"), 1);
    assert_eq!(global_int(&it, "प्रविष्टिसूचकाङ्क"), 8);
    let s = |v: &[(&str, &str)]| -> Vec<(String, String)> {
        v.iter()
            .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
            .collect()
    };
    assert_eq!(
        entry(&mut it, 1),
        (5, "ख".into(), String::new(), vec![], false),
        "the import, by module name"
    );
    assert_eq!(
        entry(&mut it, 2),
        (
            2,
            "बिन्दु".into(),
            String::new(),
            s(&[("अ", "अ६४"), ("ब", "अङ्कः अन्तः अ८")]),
            true
        ),
        "the struct's fields with their types (W-215's run)"
    );
    assert_eq!(
        entry(&mut it, 3),
        (
            6,
            "रङ्ग".into(),
            String::new(),
            s(&[("रक्त", ""), ("नील", "")]),
            true
        ),
        "the enum's variants, typeless"
    );
    assert_eq!(
        entry(&mut it, 4).0,
        7,
        "a variant is an entry of the store's kind ७"
    );
    assert_eq!(entry(&mut it, 4).1, "रक्त");
    assert_eq!(
        entry(&mut it, 6),
        (4, "गणकः".into(), "न६४".into(), vec![], true)
    );
    assert_eq!(
        entry(&mut it, 7),
        (4, "गुप्तः".into(), "सम्भाव्य बिन्दु".into(), vec![], false),
        "a private global, its type spelled as the source spells it"
    );
    assert_eq!(
        entry(&mut it, 8),
        (
            1,
            "योगः".into(),
            "दोषयुक्त अ६४".into(),
            s(&[("प", "अ६४"), ("फ", "स्थानम् बिन्दु")]),
            true
        ),
        "the routine: its return type and its parameter run"
    );
    // Looked up by module and member: the entry's index.
    assert_eq!(lookup(&mut it, "क", "योगः"), 8);
    assert_eq!(lookup(&mut it, "क", "रक्त"), 4, "a variant is a member");
    assert_eq!(
        lookup(&mut it, "क", "गुप्तः"),
        7,
        "declared, though not public — the census tells the two apart"
    );
    assert!(imported(&mut it, "क", "ख"));
    assert!(!imported(&mut it, "क", "ग"));
    assert!(
        !imported(&mut it, "ख", "क"),
        "a module the store does not hold imports nothing"
    );
}

/// W-224's two-file module: `ast.t1` and `vastu.t1` both say `मण्डलम् वास्तु`;
/// two parses of one module name land in ONE module entry.
#[test]
fn two_files_of_one_module_fall_into_one_module_entry() {
    let mut it = store();
    assert_eq!(
        collect(&mut it, "मण्डलम् वास्तु ॥\nसार्वजनिक चरः एकः ॱॱ न६४ भवति १ ।\n"),
        1
    );
    assert_eq!(
        collect(&mut it, "मण्डलम् वास्तु ॥\nसार्वजनिक चरः द्वौ ॱॱ न६४ भवति २ ।\n"),
        1
    );
    assert_eq!(
        collect(&mut it, "मण्डलम् अन्य ॥\nसार्वजनिक चरः त्रयः ॱॱ न६४ भवति ३ ।\n"),
        1
    );
    assert_eq!(global_int(&it, "मण्डलसूचकाङ्क"), 2, "वास्तु once, अन्य once");
    assert_eq!(
        int_of(
            &it.call("घोषणासञ्चयॱमण्डलप्रविष्टिसंख्या", vec![Value::Int(1)], 1_000_000)
                .unwrap()
        ),
        2,
        "both of वास्तु's globals count under it"
    );
    assert_eq!(lookup(&mut it, "वास्तु", "एकः"), 1);
    assert_eq!(lookup(&mut it, "वास्तु", "द्वौ"), 2);
    assert_eq!(lookup(&mut it, "अन्य", "त्रयः"), 3);
    // A parse with no module name (lib.t1's shape) adds nothing.
    assert_eq!(collect(&mut it, "चरः अनाथः ॱॱ न६४ भवति ० ।\n"), 0);
    assert_eq!(global_int(&it, "मण्डलसूचकाङ्क"), 2);
}

/// REFUSED, by name and kind: a member the module does not declare (kind २),
/// a module the store does not hold (kind १); an import is not a member; and
/// the first refusal is the one recorded.
#[test]
fn a_member_the_module_does_not_declare_and_a_module_the_store_does_not_hold_are_refused_by_name() {
    let mut it = store();
    collect(&mut it, PROGRAM);
    assert_eq!(lookup(&mut it, "क", "नास्ति"), 0);
    assert_eq!(
        global_int(&it, "सञ्चयदोषभेद"),
        2,
        "the module is held; the member is not declared"
    );
    assert_eq!(text_of(it.global("सञ्चयदोषमण्डलम्").as_ref().unwrap()), "क");
    assert_eq!(text_of(it.global("सञ्चयदोषनाम").as_ref().unwrap()), "नास्ति");
    // The record keeps the FIRST refusal.
    assert_eq!(lookup(&mut it, "ग", "योगः"), 0);
    assert_eq!(global_int(&it, "सञ्चयदोषभेद"), 2);
    assert_eq!(text_of(it.global("सञ्चयदोषनाम").as_ref().unwrap()), "नास्ति");
    // After a reset, a module the store does not hold is kind १.
    it.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000).unwrap();
    assert_eq!(lookup(&mut it, "ग", "योगः"), 0);
    assert_eq!(global_int(&it, "सञ्चयदोषभेद"), 1);
    assert_eq!(
        global_int(&it, "प्रविष्टिसूचकाङ्क"),
        0,
        "the reset emptied the store"
    );
    // An import is not a member: `कॱख` names no declaration of क.
    collect(&mut it, PROGRAM);
    it.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000).unwrap();
    collect(&mut it, PROGRAM);
    assert_eq!(lookup(&mut it, "क", "ख"), 0);
    assert_eq!(global_int(&it, "सञ्चयदोषभेद"), 2);
    println!(
        "REFUSED kind {} module क member ख",
        global_int(&it, "सञ्चयदोषभेद")
    );
}
