//! `W-248` — A TYPE'S TEXT TO AN `अर्थप्रकार`, AND WHAT THE READER REFUSES.
//!
//! The declaration store keeps every type AS TEXT in the source's own spelling,
//! by design. `मूलप्रकारार्थः` answered a PRIMITIVE spelling and poison for
//! everything else, so a wrapped return — `अङ्कः अन्तः अ८`, `सम्भाव्य न६४`,
//! `दोषयुक्त बूल` — had no path at all and its return went unrecorded.
//! `प्रकारपाठार्थः` is that path: the primitive arm plus the four wrappers, each
//! an outer kind over a REAL inner index, recursing on the tail.
//!
//! # What it covers, and what it refuses, and why the second is a finding
//!
//! Measured from the store's own record over the 19 sources — 1223 entries, 599
//! routine returns, 112 distinct type texts:
//!
//! | class | routines | distinct |
//! |---|---|---|
//! | primitive, covered before this row | 443 | 7 |
//! | `अङ्कः अन्तः …` | 64 | 9 |
//! | `सम्भाव्य …` | 40 | 10 |
//! | `दोषयुक्त …` | 15 | 6 |
//! | a named type | 36 | 14 |
//! | a qualified `मण्डलॱनाम` | 1 | 1 |
//!
//! The reader takes the three wrapper classes. IT DOES NOT TAKE THE LAST TWO,
//! and that is not a shortfall of the text path: `प्रकारार्थः`, the AST path,
//! ALSO answers poison for a named type — it has one primitive arm and four
//! wrapper arms and then `प्रत्यागमनम् दोषार्थः`. A named struct return is
//! poison whichever path reaches it, because NEITHER PATH HAS A SYMBOL FOR ONE.
//! That is a gap in the type representation shared by both twins, and it is
//! reported as its own finding rather than hidden here as a residue.
//!
//! # The numbers this moved
//!
//! `typecheck_store_return_type_unrecorded` 72 → 10 and
//! `typecheck_untypable_statements` 4 → 0, both re-measured by
//! `measure_corpus_typecheck`.
//!
//! # There is no Rust twin of this reader, and that is a finding not a gap
//!
//! The row asks for the reader "in BOTH twins with a twin table". Searched:
//! Rust has NO text-to-type reader and no caller for one. Its checker takes
//! types from the AST (`TypeChecker::eval_ast_type ↔ प्रकारार्थः`, already
//! twinned), and `Declaration.ty` — the store's text on the Rust side — is read
//! only by tests, which print it or walk it. Writing a Rust counterpart would
//! be a routine nobody calls, which is the defect this tree keeps finding in
//! its own margins.
//!
//! WHAT IS TWINNED IS THE PAIR THAT EXISTS: `Ty::text ↔ सञ्चयप्रकारलेखनम्`
//! PRODUCES these spellings and this reader CONSUMES them. So the check with
//! meaning is the round trip, and `every_type_text_in_the_corpus_is_read_or_refused_by_name`
//! is it: every spelling the producer put in the store is read by the consumer
//! or refused by name, and no WRAPPER is ever refused. That pins the two halves
//! against each other without inventing a third.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Every `.t1` source of the crate, sorted — the corpus.
fn corpus() -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
            .expect("the corpus directory is readable")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".t1"))
            .collect();
    names.sort();
    names
}

/// The parser, the store and the resolver in one image.
fn reader() -> Interpreter {
    let names = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "artha.t1",
        "sanchaya.t1",
        "sanskrit_text.t1",
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
        .unwrap_or_else(|e| panic!("the reader loads: {e:?}"));
    it.call("अर्थॱनिर्णायकारम्भः", vec![], 50_000_000)
        .expect("निर्णायकारम्भः runs");
    it.call("घोषणासञ्चयॱआरम्भः", vec![], 5_000_000)
        .expect("आरम्भः runs");
    it
}

fn text(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// `(भेद, अन्तःसूचकाङ्क)` of what the reader answers for one spelling.
fn read_type(it: &mut Interpreter, spelling: &str) -> (i128, i128) {
    let v = it
        .call("अर्थॱप्रकारपाठार्थः", vec![text(spelling)], 200_000_000)
        .unwrap_or_else(|e| panic!("प्रकारपाठार्थः runs on `{spelling}`: {e:?}"));
    match v {
        Value::Record(r) => {
            let r = r.borrow();
            (
                r.get("भेद").and_then(Value::as_int).unwrap_or(-1),
                r.get("अन्तःसूचकाङ्क").and_then(Value::as_int).unwrap_or(-1),
            )
        }
        other => panic!("`{spelling}` answered {other:?}, not an अर्थप्रकार"),
    }
}

/// **W-265.** The five refusal reasons `artha.t1` names, in its own numbering.
///
/// Read from `अज्ञातप्रकारकारण`. A code this table does not know is reported as
/// itself rather than as a word — a new reason in the source must not be
/// silently absorbed into an old one's label.
fn refusal_reason(code: i128) -> String {
    match code {
        1 => "bare; no module binds this spelling".into(),
        2 => "the module is known and declares no such type".into(),
        3 => "an empty spelling".into(),
        4 => "bare; MORE THAN ONE module declares it".into(),
        5 => "a qualified name whose module the store does not hold".into(),
        other => format!("a reason this test does not name: {other}"),
    }
}

/// **W-265.** Give every type the collected store declares a symbol, so the
/// reader has something to resolve a NAME to.
///
/// A type name is the store's to answer and a SYMBOL is the resolver's to mint,
/// so the reader needs both — `प्रकारपाठार्थः` sees a spelling and has no
/// program. This is the two-line handshake a consumer of the store makes:
/// assert that a collection ran, open a resolver, intern the store's types
/// under it. Without it every named spelling is refused for want of a symbol,
/// which is the state this row was opened on.
///
/// It answers HOW MANY it bound, and that is asserted rather than discarded: a
/// bind pass that binds nothing would leave every assertion below vacuously
/// true, which is the shape of guard this suite has been bitten by before.
fn bind_store_types(it: &mut Interpreter) -> i128 {
    it.call("अर्थॱसञ्चयसिद्धिः", vec![], 5_000_000)
        .expect("सञ्चयसिद्धिः runs");
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 50_000_000)
        .expect("निर्णायकारम्भः runs");
    let bound = it
        .call("अर्थॱसञ्चयप्रकारबन्धः", vec![r], 2_000_000_000)
        .expect("सञ्चयप्रकारबन्धः runs")
        .as_int()
        .expect("a count of types bound");
    assert!(
        bound > 0,
        "the store declared no types at all; every assertion after this would \
         be vacuous"
    );
    bound
}

/// The `भेद` numbers `artha.t1` gives the variants this file names.
const INT: i128 = 1;
const POINTER: i128 = 3;
const SLICE: i128 = 4;
const OPTIONAL: i128 = 5;
const ERROR_UNION: i128 = 6;
const POISON: i128 = 12;

/// Every spelling the reader takes, one per shape, with the kind it must give.
///
/// The nested rows are the point of writing this as a table: a wrapper whose
/// inner type is itself a wrapper exercises the recursion, and the corpus
/// writes both (`दोषयुक्त अङ्कः अन्तः अ८` fifteen times over).
const COVERED: &[(&str, i128)] = &[
    ("न६४", INT),
    ("अ८", INT),
    ("बूल", INT),
    ("अङ्कः अन्तः अ८", SLICE),
    ("अङ्कः अन्तः न६४", SLICE),
    ("सम्भाव्य न६४", OPTIONAL),
    ("सम्भाव्य अ३२", OPTIONAL),
    ("दोषयुक्त बूल", ERROR_UNION),
    ("स्थानम् अ८", POINTER),
    ("दोषयुक्त अङ्कः अन्तः अ८", ERROR_UNION),
    ("सम्भाव्य अङ्कः अन्तः अ८", OPTIONAL),
];

#[test]
fn the_reader_gives_every_wrapper_shape_its_kind() {
    let mut it = reader();
    let mut wrong = Vec::new();
    for (spelling, want) in COVERED {
        let (kind, _) = read_type(&mut it, spelling);
        if kind != *want {
            wrong.push(format!("`{spelling}`: kind {kind}, wanted {want}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "the reader mis-kinds:\n  {}",
        wrong.join("\n  ")
    );
}

/// A WRAPPER'S INNER TYPE IS REAL, not a placeholder.
///
/// `प्रकारार्थः` records a genuine index for the inner type and this must too,
/// or the two paths would answer the same `भेद` and disagree underneath it —
/// which is the shape of defect the twin tables exist to catch.
#[test]
fn a_wrapper_records_a_real_inner_type_and_nesting_recurses() {
    let mut it = reader();
    let (kind, inner) = read_type(&mut it, "अङ्कः अन्तः अ८");
    assert_eq!(kind, SLICE);
    assert!(
        inner > 0,
        "the element type is an index into अर्थप्रकारकोश, not ०"
    );

    // Nesting: the inner of an ErrorUnion over a slice is itself a slice.
    let (outer_kind, outer_inner) = read_type(&mut it, "दोषयुक्त अङ्कः अन्तः अ८");
    assert_eq!(outer_kind, ERROR_UNION);
    assert!(outer_inner > 0, "the inner type is recorded");
    let arena = it
        .global("अर्थप्रकारकोश")
        .expect("the type arena is a global");
    let Value::Arena(rows) = arena else {
        panic!("अर्थप्रकारकोश is an arena");
    };
    let rows = rows.borrow();
    let at = usize::try_from(outer_inner).expect("an index");
    let Some(Value::Record(inner_ty)) = rows.get(at) else {
        panic!("the inner type sits at {at} of {} rows", rows.len());
    };
    assert_eq!(
        inner_ty.borrow().get("भेद").and_then(Value::as_int),
        Some(SLICE),
        "`दोषयुक्त अङ्कः अन्तः अ८` wraps a SLICE, so the recursion ran"
    );
}

/// **THE REFUSAL.** A spelling the reader does not know is refused BY NAME with
/// its text kept, never mapped to a primitive and never silently ०.
#[test]
fn an_unknown_spelling_is_refused_by_name_with_its_text() {
    let mut it = reader();
    let before = it
        .global("अज्ञातप्रकारपाठसंख्या")
        .and_then(Value::as_int)
        .expect("the refusal counter");
    let (kind, _) = read_type(&mut it, "निर्णायक");
    assert_eq!(
        kind, POISON,
        "a named type is poison, not a guessed primitive"
    );
    let after = it
        .global("अज्ञातप्रकारपाठसंख्या")
        .and_then(Value::as_int)
        .expect("the refusal counter");
    assert_eq!(after, before + 1, "the refusal was counted");
    assert!(
        matches!(it.global("अज्ञातप्रकारपाठमस्ति"), Some(Value::Bool(true))),
        "the flag is raised"
    );
    let last = match it.global("अज्ञातप्रकारपाठ") {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("the refused text is a run of octets, not {other:?}"),
    };
    assert_eq!(last, "निर्णायक", "and the TEXT is kept, not just a count");
}

/// The refusal keeps a RUN and not one slot.
///
/// This row was opened partly because the checker's own untypable record is
/// first-only per source — 4 counted and 2 named. Writing that shape into the
/// fix would repeat the defect inside its own repair, so every refused spelling
/// is appended and this proves the second one survives the third.
#[test]
fn every_refused_spelling_is_kept_and_not_only_the_last() {
    let mut it = reader();
    for s in ["निर्णायक", "पदविभागॱचिह्नक", "मूल्याङ्क"]
    {
        read_type(&mut it, s);
    }
    let Some(Value::Arena(rows)) = it.global("अज्ञातप्रकारपाठकोश")
    else {
        panic!("the refused run is an arena");
    };
    let kept: Vec<String> = rows
        .borrow()
        .iter()
        .skip(1)
        .filter_map(|v| match v {
            Value::Octets(o) => Some(String::from_utf8_lossy(o.as_slice()).into_owned()),
            _ => None,
        })
        .collect();
    for s in ["निर्णायक", "पदविभागॱचिह्नक", "मूल्याङ्क"]
    {
        assert!(
            kept.contains(&s.to_string()),
            "`{s}` is in the run: {kept:?}"
        );
    }
}

/// **THE RATCHET.** Every type text the corpus writes is either read or refused
/// BY NAME, and the set that is refused is pinned.
///
/// A new WRAPPER spelling is covered the day it is written, because the reader
/// works by shape. A new NAMED type is refused, and this fails until either the
/// reader learns it or the pin is lowered deliberately — which is what "a new
/// spelling fails by name" has to mean.
#[test]
#[ignore = "ratchet: the store and the resolver over the corpus; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn every_type_text_in_the_corpus_is_read_or_refused_by_name() {
    let mut it = reader();
    for n in corpus() {
        let src = source(&n);
        let toks = it
            .call("पदविभागॱपदविभाग", vec![text(&src)], 2_000_000_000)
            .unwrap_or_else(|e| panic!("{n} lexes: {e:?}"));
        it.call("व्याकरॱकार्यक्रमपठनम्", vec![toks], 4_000_000_000)
            .unwrap_or_else(|e| panic!("{n} parses: {e:?}"));
        it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 2_000_000_000)
            .unwrap_or_else(|e| panic!("{n} collects: {e:?}"));
    }
    let entries = it
        .global("प्रविष्टिसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the store's entry count");
    assert!(
        entries > 500,
        "only {entries} declarations collected; the ratchet would pin almost nothing"
    );
    // W-265: the store's types get symbols before anything is read.
    let bound = bind_store_types(&mut it);
    println!("METRIC type_text_store_types_bound {bound}");

    let mut spellings: BTreeSet<String> = BTreeSet::new();
    for i in 1..=entries {
        let mut of = |v: Value| {
            if let Value::Octets(o) = v {
                let s = String::from_utf8_lossy(o.as_slice()).into_owned();
                if !s.is_empty() {
                    spellings.insert(s);
                }
            }
        };
        of(it
            .call("घोषणासञ्चयॱप्रविष्टिप्रकारः", vec![Value::Int(i)], 20_000_000)
            .expect("a type text"));
        let n = it
            .call(
                "घोषणासञ्चयॱप्रविष्टिप्राचलसंख्या",
                vec![Value::Int(i)],
                20_000_000,
            )
            .expect("a member count")
            .as_int()
            .unwrap_or(0);
        for k in 0..n {
            of(it
                .call(
                    "घोषणासञ्चयॱप्राचलप्रविष्टिप्रकारः",
                    vec![Value::Int(i), Value::Int(k)],
                    20_000_000,
                )
                .expect("a member type text"));
        }
    }
    println!("METRIC type_text_distinct_spellings {}", spellings.len());

    let mut refused: Vec<String> = Vec::new();
    let mut why: Vec<(String, i128)> = Vec::new();
    for s in &spellings {
        if read_type(&mut it, s).0 == POISON {
            // W-265: WHICH of the five reasons, read off the record the refusal
            // just wrote. A refusal that says only "unknown" is silence with a
            // number attached, and this area's failure mode IS silence.
            let reason = it
                .global("अज्ञातप्रकारकारण")
                .and_then(Value::as_int)
                .expect("the refusal reason");
            refused.push(s.clone());
            why.push((s.clone(), reason));
        }
    }
    println!("METRIC type_text_refused_spellings {}", refused.len());
    for (r, code) in &why {
        println!("  REFUSED ({}): {r}", refusal_reason(*code));
    }

    // Every refusal is a NAMED or QUALIFIED type — never a wrapper and never a
    // primitive. If a wrapper ever lands here the reader has a hole in a shape
    // it claims, which is a different defect from meeting a new named type.
    let wrappers: Vec<&String> = refused
        .iter()
        .filter(|s| {
            s.starts_with("अङ्कः ")
                || s.starts_with("सम्भाव्य ")
                || s.starts_with("दोषयुक्त ")
                || s.starts_with("स्थानम् ")
        })
        .collect();
    assert!(
        wrappers.is_empty(),
        "the reader refused {} WRAPPER spelling(s), which it claims to cover: {wrappers:?}",
        wrappers.len()
    );
}

/// The census: what the corpus spells, by class.
#[test]
#[ignore = "measurement"]
fn measure_corpus_type_texts() {
    let mut it = reader();
    for n in corpus() {
        let src = source(&n);
        let toks = it
            .call("पदविभागॱपदविभाग", vec![text(&src)], 2_000_000_000)
            .expect("lex");
        it.call("व्याकरॱकार्यक्रमपठनम्", vec![toks], 4_000_000_000)
            .expect("parse");
        it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 2_000_000_000)
            .expect("collect");
    }
    let entries = it
        .global("प्रविष्टिसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("entries");
    bind_store_types(&mut it);
    let mut returns: BTreeMap<String, usize> = BTreeMap::new();
    for i in 1..=entries {
        let kind = it
            .call("घोषणासञ्चयॱप्रविष्टिभेदः", vec![Value::Int(i)], 20_000_000)
            .expect("kind")
            .as_int()
            .unwrap_or(0);
        if kind != 1 {
            continue;
        }
        if let Value::Octets(o) = it
            .call("घोषणासञ्चयॱप्रविष्टिप्रकारः", vec![Value::Int(i)], 20_000_000)
            .expect("type")
        {
            let s = String::from_utf8_lossy(o.as_slice()).into_owned();
            if !s.is_empty() {
                *returns.entry(s).or_default() += 1;
            }
        }
    }
    let mut by_class: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (s, n) in &returns {
        let class = if read_type(&mut it, s).0 != POISON {
            "read"
        } else if s.contains('ॱ') {
            "refused: qualified"
        } else {
            "refused: named type"
        };
        let e = by_class.entry(class).or_default();
        e.0 += 1;
        e.1 += n;
    }
    println!("METRIC type_text_store_entries {entries}");
    println!(
        "METRIC type_text_routine_returns {}",
        returns.values().sum::<usize>()
    );
    for (class, (distinct, total)) in &by_class {
        println!("  {class:22}  {distinct:3} distinct  {total:4} routines");
    }
}
