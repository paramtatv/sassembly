//! **CAUSE २६ — a field base that is not a plain name.**
//!
//! The arm's own margin called it a SCOPE refusal: *"MY RESTRICTION, not the
//! corpus's… The base could be lowered — it is a record expression like any
//! other."* Overturned; the else-arm types the base through
//! `अर्थॱअभिव्यञ्जकप्रकारः`.
//!
//! # THE DRIVER HERE IS THE ONE THAT MEASURES
//!
//! `paradigm_encode.rs` — the census, which counts this cause — has **zero**
//! mentions of `Front` and loads through `Interpreter::load`. So this probe loads
//! the same way. A reading taken through `Front` would be true and would not be
//! about the number anyone counts; that mistake cost a whole unit one landing ago.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

type Counts = Vec<(usize, i128)>;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// Compile one corpus source through the chain and return every non-zero
/// `(stub cause, count)` and `(lowered shape, count)`.
fn compile(name: &str) -> (Counts, Counts) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src");
    let chain = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "ir.t1",
    ];
    let texts: Vec<(String, String)> = chain
        .iter()
        .map(|n| {
            (
                (*n).to_string(),
                std::fs::read_to_string(dir.join(n)).unwrap_or_else(|e| panic!("{n}: {e}")),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("the chain loads");
    let src = std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));

    let toks = it
        .call(
            "पदविभागॱपदविभाग",
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            2_000_000_000,
        )
        .expect("lex");
    let decls = it
        .call(
            "व्याकरॱकार्यक्रमपठनम्",
            vec![Value::Int(toks.as_int().unwrap_or(0))],
            4_000_000_000,
        )
        .expect("parse")
        .as_int()
        .unwrap_or(0);
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver init");
    let v = it.call(
        "अर्थॱकार्यक्रमनिर्णयः",
        vec![r.clone(), Value::Int(decls)],
        8_000_000_000,
    );
    assert_eq!(v, Ok(Value::Bool(true)), "{name} must resolve; got {v:?}");
    let _ = it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000);
    let _ = it.call(
        "अर्थॱकार्यक्रमप्रकारपरीक्षा",
        vec![Value::Int(decls)],
        8_000_000_000,
    );
    it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000).expect("ir init");
    let built = it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(decls)], 8_000_000_000);
    assert!(built.is_ok(), "{name} must reach IR; got {built:?}");

    let read = |it: &mut Interpreter, g: &str| -> Counts {
        let Some(Value::Arena(a)) = it.global(g).cloned() else {
            panic!("{g} is not an arena");
        };
        let n = a.borrow().len();
        let mut out = Vec::new();
        for i in 0..n {
            if let Some(v) = a.borrow().get(i).and_then(Value::as_int)
                && v > 0
            {
                out.push((i, v));
            }
        }
        out
    };
    let c = read(&mut it, "अपूर्णगणनाकोश");
    let s = read(&mut it, "रचितगणनाकोश");
    (c, s)
}

/// **`artha.t1` CARRIES CHAINED FIELD BASES** — `परिसरः ॱ प्रविष्टयः ॱ दैर्घ्य` and
/// `निर्णायकः ॱ परिसराः ॱ दैर्घ्य` among them — so it is the source where cause २६
/// lives. Print every cause and shape; a table showing only the one expected
/// cannot show a different one moving.
#[test]
fn measure_cause_26_on_artha() {
    let (causes, shapes) = compile("artha.t1");
    for (c, k) in &causes {
        println!("  CAUSE {c:>3} x{k}");
    }
    println!(
        "METRIC cause26_artha_cause_26 {:?}",
        causes.iter().find(|(c, _)| *c == 26).map(|(_, k)| *k)
    );
    println!(
        "METRIC cause26_artha_shape_27_field_load {:?}",
        shapes.iter().find(|(s, _)| *s == 27).map(|(_, k)| *k)
    );
    println!(
        "METRIC cause26_artha_cause_44 {:?}",
        causes.iter().find(|(c, _)| *c == 44).map(|(_, k)| *k)
    );
    println!(
        "METRIC cause26_artha_total_stubs {}",
        causes.iter().map(|(_, k)| k).sum::<i128>()
    );
}
