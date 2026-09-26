//! ॥ A BARE CROSS-MODULE RECORD TYPE MUST GET STORAGE — AND A TIE MUST NOT ॥
//! `W-279`.
//!
//! `ir.t1`'s `चरः` arm allocates a record for `चरः x ॱॱ T भवति ०` in exactly two
//! cases: `संरचनानामघोषणा` finds T among the declarations of the source being
//! lowered, or T carries a MODULE PREFIX and `घोषणासञ्चयॱसदस्यान्वेषणम्` finds it
//! in the store. A BARE name that resolves to another module matched neither:
//! the remote path is gated on `यदि रचनोपसर्गः ॱ दैर्घ्य अधिकम् ० आदि`, the arm
//! fell through to `चरमूल्यम् भवति ध्रुवरचना ०`, the slot held ०, and the first
//! field access faulted at address ० — natively only, since `nirvahana.rs`
//! allocates for `भवति ०` on a record type and the interpreter never sees it.
//!
//! Eight sites were qualified by hand on 2026-09-18 and
//! `t1_bare_cross_module_type.rs` ratchets the population at ०. THAT IS A
//! GUARD OVER THE SOURCE. This file is the guard over the COMPILER: it asks
//! what `ir.t1` lowers, so that the ninth bare declaration anyone writes gets
//! storage instead of a null.
//!
//! ## THE CASE THAT MUST STILL BE REFUSED, and why it is half the file
//!
//! A bare name is NOT always resolvable. `आज्ञा` is declared public by both
//! `मध्यरूप` and `वाक्यविभाग`; `वाक्य` by both `वास्तु` and `वाक्यविभाग`. Picking
//! the first match would hand back a field count from the wrong record — a
//! SILENT WRONG SIZE, which is worse than the null it replaces, because a null
//! faults at the first access and a wrong size corrupts whatever follows it.
//!
//! So the rule is: **exactly one** candidate, or no allocation at all. The
//! three refusals below are the whole reason the resolution is allowed to
//! exist, and each has its own test:
//!
//! ```text
//!   TWO imported modules declare the name   -> ० (a tie is refused, not guessed)
//!   the declaring module is NOT imported    -> ० (reachability, not name matching)
//!   the declaration is not सार्वजनिक         -> ० (privacy, from another module)
//! ```
//!
//! ## THE LOADER IS PART OF THE TEST
//!
//! `घोषणासञ्चय` must hold BOTH modules before `मध्यरूप` runs, and the collector
//! copies text out of `व्याकर` at the moment it is called — so the order is lex,
//! parse, `सङ्ग्रहः`, and only then the next source. A run that collected only
//! the caller would find no candidate and answer ० for the RIGHT answer's
//! reason, which is exactly the shape of a green that means nothing.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

/// `ir.t1:236` — `Instruction::AllocRecord(u64)`, the octet count in `ध्रुवमूल्यम्`.
const ALLOC_RECORD: i128 = 24;

/// The seven modules `मध्यरूप` needs to reach IR, read FROM DISK and listed
/// here rather than shared with another test file: a loader is part of the test
/// it serves, and this one must not be movable by a guard that belongs to
/// someone else. `ast.t1` carries `वास्तु` for this set, as it does in
/// `t1_field_offsets.rs`.
const CHAIN: &[&str] = &[
    "lex.t1",
    "ast.t1",
    "parse.t1",
    "artha.t1",
    "sanchaya.t1",
    "sanskrit_text.t1",
    "ir.t1",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn source(name: &str) -> String {
    let p = repo_root().join("crates/sadhana-t1/src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn load_chain() -> Interpreter {
    let texts: Vec<(String, String)> = CHAIN
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &repo_root().join("spec"))
        .unwrap_or_else(|e| panic!("the IR chain loads: {e:?}"))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn int_field(v: &Value, name: &str) -> Option<i128> {
    match v {
        Value::Record(r) => r.borrow().get(name).and_then(Value::as_int),
        _ => None,
    }
}

/// Lex and parse `src`, then copy its declarations into `घोषणासञ्चय`.
///
/// The parse is asserted through `दोषसूचकाङ्क` rather than through a return
/// value: `कार्यक्रमपठनम्` answers a declaration count, and a scratch module
/// with a typo answers a SMALLER count rather than failing, which would leave
/// every assertion below testing a program that is not the one written here.
fn collect(it: &mut Interpreter, src: &str) {
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("पदविभाग runs");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![toks], 4_000_000_000)
        .expect("कार्यक्रमपठनम् runs");
    assert_eq!(
        it.global("दोषसूचकाङ्क").and_then(Value::as_int),
        Some(0),
        "the scratch module parses clean:\n{src}"
    );
    assert!(
        parsed.as_int().unwrap_or(0) > 0,
        "the scratch module declares something:\n{src}"
    );
    it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 400_000_000)
        .expect("सङ्ग्रहः runs");
}

/// Every `AllocRecord` size `ir.t1` emitted for `caller`, in instruction order.
///
/// `callees` are collected into the store FIRST, in the order given; `caller` is
/// collected and then lowered. The whole chain runs on one interpreter because
/// `घोषणासञ्चय` is the point of the exercise.
fn alloc_sizes(callees: &[&str], caller: &str) -> Vec<i128> {
    let mut it = load_chain();
    it.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000)
        .expect("आरम्भः runs");

    for m in callees {
        collect(&mut it, m);
    }

    // The caller is parsed LAST so that `व्याकर` holds it when `मध्यरूप` reads
    // the declarations — `संरचनानामघोषणा` scans that arena, not the store.
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(caller)], 2_000_000_000)
        .expect("पदविभाग runs");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![toks], 4_000_000_000)
        .expect("कार्यक्रमपठनम् runs")
        .as_int()
        .expect("a declaration count");
    assert_eq!(
        it.global("दोषसूचकाङ्क").and_then(Value::as_int),
        Some(0),
        "the caller parses clean:\n{caller}"
    );
    it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 400_000_000)
        .expect("सङ्ग्रहः runs");

    let resolver = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("the resolver starts");
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![resolver.clone(), Value::Int(parsed)],
            4_000_000_000
        ),
        Ok(Value::Bool(true)),
        "the caller resolves:\n{caller}"
    );
    it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![resolver], 5_000_000)
        .expect("the checker starts");
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000
        ),
        Ok(Value::Bool(true)),
        "the caller typechecks:\n{caller}"
    );

    it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
        .expect("मध्यरूप starts");
    it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
        .expect("कार्यक्रमरचना runs");

    let cursor = it
        .global("आज्ञासूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the instruction cursor");
    let Some(Value::Arena(insts)) = it.global("आज्ञाकोश").cloned() else {
        panic!("आज्ञाकोश is an arena");
    };
    let mut sizes = Vec::new();
    // ONE-BASED, as every कोश of the corpus is: slot ० is reserved.
    for i in 1..=usize::try_from(cursor).unwrap_or(0) {
        let Some(ins) = insts.borrow().get(i).cloned() else {
            continue;
        };
        if int_field(&ins, "भेद") == Some(ALLOC_RECORD) {
            sizes.push(int_field(&ins, "ध्रुवमूल्यम्").unwrap_or(-1));
        }
    }
    sizes
}

/// A module declaring one public three-field record, under a name of the
/// caller's choosing. Three fields so the expected size — २४ — cannot be
/// confused with a header, a word, or a wrongly-scaled count.
fn provider(module: &str, ty: &str, public: bool) -> String {
    let vis = if public {
        "सार्वजनिक "
    } else {
        ""
    };
    format!(
        "मण्डलम् {module} ॥\n\n\
         {vis}संरचना {ty} आरभ्य\n    \
         प्रथमम् ॱॱ न६४ ऽ\n    \
         द्वितीयम् ॱॱ न६४ ऽ\n    \
         तृतीयम् ॱॱ न६४\n\
         समाप्तम् ।\n"
    )
}

/// A caller that declares ONE local of type `ty` and writes a field, so that a
/// missing allocation is a store through ० natively.
fn caller(imports: &[&str], ty: &str) -> String {
    let mut s = String::from("मण्डलम् परीक्षाकर्ता ॥\n");
    for i in imports {
        s.push_str(&format!("आयातः {i} ।\n"));
    }
    s.push_str(&format!(
        "\nसार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    \
         चरः धारकम् ॱॱ {ty} भवति ० ।\n    \
         धारकम् ॱ प्रथमम् भवति ७ ।\n    \
         प्रत्यागमनम् धारकम् ॱ प्रथमम् ।\n\
         इति\n"
    ));
    s
}

/// THE POSITIVE CONTROL, and it runs on the SAME machinery as the subject.
///
/// A QUALIFIED cross-module type already lowers to an allocation — that path
/// landed on 2026-09-13. If this goes red the harness is broken, not the
/// feature, and the ० below would mean nothing.
#[test]
fn a_qualified_cross_module_record_is_allocated() {
    let sizes = alloc_sizes(
        &[&provider("दाता", "अंशकम्", true)],
        &caller(&["दाता"], "दाताॱअंशकम्"),
    );
    assert_eq!(
        sizes,
        vec![24],
        "a prefixed remote record allocates three fields"
    );
}

/// THE SUBJECT. One imported module declares the name, publicly, and nothing
/// else does.
#[test]
fn a_bare_cross_module_record_is_allocated() {
    let sizes = alloc_sizes(
        &[&provider("दाता", "अंशकम्", true)],
        &caller(&["दाता"], "अंशकम्"),
    );
    assert_eq!(
        sizes,
        vec![24],
        "a bare name that resolves to exactly one imported module's public \
         record must get the SAME storage the qualified spelling gets — \
         otherwise the slot holds ० and the first field access faults at \
         address ० in the native image only"
    );
}

/// REFUSAL ONE — A TIE. Two imported modules declare the same public record
/// name, with DIFFERENT field counts, so that a wrong guess would be visible as
/// a wrong size rather than as a coincidence.
#[test]
fn a_name_two_imported_modules_declare_is_refused() {
    let wider = "मण्डलम् द्वितीयदाता ॥\n\n\
                 सार्वजनिक संरचना अंशकम् आरभ्य\n    \
                 एकम् ॱॱ न६४ ऽ\n    \
                 द्वे ॱॱ न६४ ऽ\n    \
                 त्रीणि ॱॱ न६४ ऽ\n    \
                 चत्वारि ॱॱ न६४ ऽ\n    \
                 पञ्च ॱॱ न६४\n\
                 समाप्तम् ।\n";
    let sizes = alloc_sizes(
        &[&provider("दाता", "अंशकम्", true), wider],
        &caller(&["दाता", "द्वितीयदाता"], "अंशकम्"),
    );
    assert!(
        sizes.is_empty(),
        "a bare name TWO imported modules declare has no one answer; picking \
         either hands back a field count from the wrong record, which corrupts \
         silently where the null at least faults. It allocated {sizes:?}"
    );
}

/// REFUSAL TWO — NOT IMPORTED. The store holds every module the run has
/// parsed, not only the ones this source may see. Matching on the NAME alone
/// would resolve through a module the caller never imported.
#[test]
fn a_name_only_an_unimported_module_declares_is_refused() {
    let sizes = alloc_sizes(
        &[
            &provider("दाता", "अंशकम्", true),
            &provider("अन्यदाता", "गुप्ताङ्गम्", true),
        ],
        &caller(&["दाता"], "गुप्ताङ्गम्"),
    );
    assert!(
        sizes.is_empty(),
        "`गुप्ताङ्गम्` is declared only by `अन्यदाता`, which the caller does not \
         import. It allocated {sizes:?}"
    );
}

/// REFUSAL THREE — NOT PUBLIC. `सार्वजनिकत्व` is carried on every entry of the
/// store and a cross-module use must honour it.
#[test]
fn a_private_record_in_an_imported_module_is_refused() {
    let sizes = alloc_sizes(
        &[&provider("दाता", "अंशकम्", false)],
        &caller(&["दाता"], "अंशकम्"),
    );
    assert!(
        sizes.is_empty(),
        "`अंशकम्` is declared without `सार्वजनिक`, so another module cannot name \
         it. It allocated {sizes:?}"
    );
}
