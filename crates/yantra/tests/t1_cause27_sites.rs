//! **CAUSE २७ — WHICH of its fifteen guards declined.**
//!
//! `64a93973` gave every cause-२७ raise in `अभिव्यञ्जकरचना` a SITE number and a
//! counter, `अपूर्णस्थानगणनाकोश`, because `अपूर्णगणनाकोश अङ्कः २७` says how often
//! the index arm refused and never which check did it. This reads the new arena.
//!
//! # WHY THIS IS THE CHEAPEST WAY TO SHRINK THE REMAINING WORK
//!
//! The corpus's whole stub surface is 45 live raise sites, all in `ir.t1`, and
//! fifteen are cause २७. But reading the guards, they are not fifteen missing
//! features. Eleven are INTERNAL CONSISTENCY CHECKS — arena bounds (३, ८, १२),
//! nil values (४, ९, १३), zero indices (६, ७, ११), malformed expression indices
//! (१, २) — states that cannot occur unless something upstream is already wrong.
//! Only ५ (a qualified name as base, which denotes a MODULE and has no run to
//! index), १० (a base whose type is not a slice), १४ (an element width this arm
//! emits no load for) and १५ are refusals of a SHAPE.
//!
//! **A guard that never fires is not work.** Measuring which sites fire turns a
//! static count of fifteen into a list of the ones that are actually reached, and
//! the difference is the part nobody has to implement.
//!
//! # WHAT THIS IS NOT
//!
//! **This is a SAMPLE, not a census.** It compiles the sources named below, not
//! the corpus, so a site reading zero here means "not reached by these sources",
//! never "unreachable". The whole-corpus figure is the census's to report; this
//! probe exists to be cheap enough to run while working on the arm.
//!
//! The driver is `Interpreter::load`, matching `t1_cause26.rs` and
//! `paradigm_encode.rs` — the census that counts these causes has zero mentions
//! of `Front`. A reading taken through `Front` would be true and would not be
//! about the number anyone counts.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

type Counts = Vec<(usize, i128)>;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// Compile one corpus source through the chain and return every non-zero
/// `(cause, count)` and `(cause-२७ site, count)`.
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
    let causes = read(&mut it, "अपूर्णगणनाकोश");
    let sites = read(&mut it, "अपूर्णस्थानगणनाकोश");
    (causes, sites)
}

/// What each site refuses, so a number in the table below is readable without
/// opening `ir.t1`. Taken from the guard at each raise, in file order.
///
/// **NUMBERED AND NAMED, NOT POSITIONAL, BECAUSE `ir.t1`'s `अपूर्णस्थानसीमा` IS
/// CHECKED AGAINST THIS TABLE'S HIGHEST NUMBER.** `W-264`'s
/// `each_arena_bound_equals_the_highest_entry_of_the_table_it_bounds`, in
/// `sadhana-t1`, reads THIS FILE as source and parses `(n, "name")` rows — the
/// shape `STUB_CAUSES` and `LOWERED_SHAPES` carry in `paradigm_encode.rs`. A
/// positional `[&str; 16]` could not say where its arena ends: an array with a
/// nil slot at ० and fifteen sites, and an array with sixteen sites and no nil
/// slot, are the same sixteen-element array. The number is now written down.
///
/// The table starts at १: ० is the arena's nil slot and names no site.
const CAUSE_27_SITES: &[(i128, &str)] = &[
    (1, "index expression missing"),
    (2, "base index below one"),
    (3, "base index past the expression arena"),
    (4, "base expression is nil"),
    (5, "base is a qualified name (a MODULE)"),
    (6, "base has no symbol"),
    (7, "base type index is zero"),
    (8, "type arena too short for the base"),
    (9, "base type is nil"),
    (10, "base type is not a slice"),
    (11, "element type index is zero"),
    (12, "type arena too short for the element"),
    (13, "element type is nil"),
    (14, "element width has no load"),
    (15, "the index arm's last refusal"),
];

/// The name of site `s`, or `None` when the arena reported a number this table
/// does not name — a site wired in `ir.t1` and never named here, which prints as
/// `unknown site` rather than being silently dropped.
fn site_name(s: usize) -> Option<&'static str> {
    CAUSE_27_SITES
        .iter()
        .find(|(n, _)| *n == s as i128)
        .map(|(_, what)| *what)
}

/// **THE SITES THAT FIRE ARE THE ONLY ONES THAT ARE WORK.** Print every site with
/// a non-zero count, and the cause-२७ total beside it so the two can be compared:
/// they must agree, because every cause-२७ raise goes through exactly one site.
#[test]
fn measure_which_cause_27_sites_fire() {
    // Sources chosen for INDEXING DENSITY, which is what this arm lowers — not a
    // random sample and not the corpus. `artha.t1` is the one `t1_cause26.rs`
    // already probes, so a reading here is comparable with that one.
    let sources = ["artha.t1", "lex.t1", "parse.t1"];
    let mut total_27: i128 = 0;
    let mut site_total: i128 = 0;
    let mut fired: Vec<usize> = Vec::new();

    for name in sources {
        let (causes, sites) = compile(name);
        let c27 = causes
            .iter()
            .find(|(c, _)| *c == 27)
            .map(|(_, k)| *k)
            .unwrap_or(0);
        total_27 += c27;
        println!("  {name}: cause २७ x{c27}");
        for (s, k) in &sites {
            let what = site_name(*s).unwrap_or("unknown site");
            println!("    SITE {s:>3} x{k}  {what}");
            site_total += *k;
            if !fired.contains(s) {
                fired.push(*s);
            }
        }
    }
    fired.sort_unstable();

    println!("METRIC cause27_sites_fired {}", fired.len());
    println!("METRIC cause27_sites_total {}", CAUSE_27_SITES.len());
    println!("METRIC cause27_raises_over_sample {total_27}");
    println!("  sites that fired: {fired:?}");

    // THE CONTROL, and it is the point of the site channel. Every cause-२७ raise
    // passes through exactly one site, so the two counters are the same quantity
    // measured on two paths. If they disagree, a raise reached `अपूर्णध्रुवम्`
    // without a site — which is a site number that was never wired, not a
    // compiler defect, and this is the only thing that would say so.
    assert_eq!(
        site_total, total_27,
        "every cause २७ raise must pass through exactly one site: \
         sites summed to {site_total}, cause २७ counted {total_27}"
    );
}
