//! **`Refusal::UnnamedSymbol` HAS FIVE RAISING SITES IN `यन्त्रोत्सर्जन` AND THE
//! RECORD COULD NOT TELL THEM APART.** `यन्त्रनिषेधः` writes five globals; at all
//! five `यन्त्रानामसंज्ञानिषेधभेद` sites four of them are written with the SAME
//! constants — `वृत्ति` empty, `पर्व` ०, `लक्ष्य` ० — so the only varying field is
//! `संख्या`, the offending symbol. **And the symbol is ० in exactly the case that
//! matters**: a lookup of symbol ० misses, so `संख्या=०` is what four of the five
//! sites report, and the record collapses four different defects into one reading.
//!
//! THE READINGS THAT WERE TAKEN ON IT. `hopladder/mod.rs` prints the record for
//! every source that emits nothing, and its margin said *"the variant alone does
//! not localise; this field is what separates them"* of `संख्या` — true only while
//! the symbol is non-zero. `t1_driver.rs` said `पर्व` is *"० for the label and entry
//! sites, a real id for a call"*; it is ० at the call site too. On 2026-09-16 the
//! ledger recorded four corpus sources at hop 0 "sharing ONE cause, measured: all
//! four report `यन्त्रनिषेधभेद=४` with `संख्या=०`" — a reading this instrument
//! cannot support, because that pair names a FAMILY of four sites.
//!
//! **THE FIX IS THE FIELD THAT WAS ALREADY THERE AND ALREADY PRINTED.** `लक्ष्य`
//! carries ० at every `UnnamedSymbol` site and its meaning is already per-variant
//! (`संख्या` is documented `param` / `bytes` / `params` / `symbol`), so the site code
//! goes there and every existing reader — the ladder, `t1_driver`, `t1_boot` — starts
//! saying which site with no change. Rust's `Refusal::UnnamedSymbol { symbol }` has
//! no second field to disagree with, and no test asserts a value for `यन्त्रनिषेधलक्ष्य`.
//!
//! THE REFUSED CASE IS A MUTATION, NOT A NARRATIVE: the third test loads a
//! `yantrotsarjana.t1` with the site code at the call site put back to `०` and
//! asserts the two sites become INDISTINGUISHABLE again — same भेद, same संख्या,
//! same लक्ष्य. Without it, a passing pair of assertions cannot be told from a pair
//! that would pass against any value at all.

use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// THIS TEST'S OWN LOADER. A `.t1` module is well-formed in the company it keeps —
/// the interpreter's arity table is global — so the set is stated here rather than
/// borrowed: `lex.t1` because every spec-table read goes through
/// `पदविभागॱसमावेशपाठः`, `ir.t1` and `utsarjana.t1` because `यन्त्रोत्सर्जन` reads
/// both. `replaced` swaps one source's text for a mutant, which is how the refused
/// case below is produced from the real file instead of from a fixture.
fn load_emitter(replaced: Option<(&str, &str)>) -> Interpreter {
    const EMITTER: &[&str] = &["lex.t1", "ir.t1", "utsarjana.t1", "yantrotsarjana.t1"];
    let texts: Vec<(String, String)> = EMITTER
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
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{EMITTER:?} load: {e:?}"))
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global of the loaded corpus"))
}

/// The record as a whole, so a disagreement names every field at once.
#[derive(Debug, PartialEq, Eq)]
struct Record {
    मस्ति: bool,
    भेद: i128,
    पर्व: i128,
    लक्ष्य: i128,
    संख्या: i128,
    वृत्ति: String,
}

fn record(it: &Interpreter) -> Record {
    Record {
        मस्ति: it
            .global("यन्त्रनिषेधमस्ति")
            .and_then(|v| match v {
                Value::Bool(b) => Some(*b),
                _ => None,
            })
            .expect("`यन्त्रनिषेधमस्ति` is a boolean global"),
        भेद: global_int(it, "यन्त्रनिषेधभेद"),
        पर्व: global_int(it, "यन्त्रनिषेधपर्व"),
        लक्ष्य: global_int(it, "यन्त्रनिषेधलक्ष्य"),
        संख्या: global_int(it, "यन्त्रनिषेधसंख्या"),
        वृत्ति: it
            .global("यन्त्रनिषेधवृत्ति")
            .and_then(|v| {
                v.octets()
                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            })
            .unwrap_or_default(),
    }
}

/// Drive ONE site to its refusal in a FRESH interpreter, and answer the record.
///
/// A fresh load per site is not tidiness: `यन्त्रनिषेधः` keeps the FIRST refusal and
/// returns early on every later one, so two sites driven in one interpreter would
/// both report the first one's record and the test would pass while proving nothing.
fn refuse(routine: &str, args: Vec<Value>, mutant: Option<(&str, &str)>) -> Record {
    let mut it = load_emitter(mutant);
    let answer = it
        .call(routine, args, 200_000_000)
        .unwrap_or_else(|e| panic!("`{routine}` runs: {e}"));
    // EVERY ONE OF THESE SITES ANSWERS ० AFTER RECORDING. A routine that answered
    // something else would have taken a different exit and the record would be
    // about a different event than the one the test names.
    assert_eq!(
        answer.as_int(),
        Some(0),
        "`{routine}` answers ० after recording a refusal"
    );
    let r = record(&it);
    assert!(r.मस्ति, "`{routine}` recorded a refusal: {r:?}");
    r
}

/// `यन्त्रवृत्तिचिह्नम् ०` — the label helper, Rust's `routine_label`. Site १.
const LABEL: (&str, i128) = ("यन्त्रोत्सर्जनॱयन्त्रवृत्तिचिह्नम्", 1);
/// `यन्त्राह्वानोत्सर्जनम्`'s callee lookup, Rust's `emit_call`. Site ३.
const CALLEE: (&str, i128) = ("यन्त्रोत्सर्जनॱयन्त्राह्वानोत्सर्जनम्", 3);

fn label_record(mutant: Option<(&str, &str)>) -> Record {
    refuse(LABEL.0, vec![Value::Int(0)], mutant)
}

fn callee_record(mutant: Option<(&str, &str)>) -> Record {
    refuse(
        CALLEE.0,
        // `V-005` added the fifth: whether the callee answers a `प६४` (० here).
        vec![
            Value::Int(0),
            Value::Int(0),
            Value::Int(0),
            Value::Int(0),
            Value::Int(0),
        ],
        mutant,
    )
}

/// **THE PAIR THE OLD RECORD COULD NOT SEPARATE.** Both are `UnnamedSymbol` on
/// symbol ०; the first is a label asked for a symbol with no name entry, the
/// second a CALL to a callee the resolver never named. One is a defect in the
/// name table, the other a defect in `अभिव्यञ्जकरचना`'s call arm — and which
/// decides what to fix.
#[test]
fn the_unnamed_symbol_record_names_which_of_the_five_sites_raised_it() {
    let label = label_record(None);
    let callee = callee_record(None);
    println!("  LABEL  {label:?}");
    println!("  CALLEE {callee:?}");

    // THE FOUR FIELDS THAT AGREE, ASSERTED AS AGREEING. This is the measurement
    // the ledger's "one cause" reading was taken from, and it is kept here so the
    // reason the fifth field is needed stays in the test rather than in a margin.
    assert_eq!(label.भेद, callee.भेद, "both are UnnamedSymbol");
    assert_eq!(label.भेद, global_int_of("यन्त्रानामसंज्ञानिषेधभेद"));
    assert_eq!(label.संख्या, 0, "the offending symbol is ० at both");
    assert_eq!(callee.संख्या, 0, "the offending symbol is ० at both");
    assert_eq!(label.पर्व, 0, "`पर्व` is ० at the call site too");
    assert_eq!(callee.पर्व, 0, "`पर्व` is ० at the call site too");
    assert_eq!(label.वृत्ति, "", "no routine label at either site");
    assert_eq!(callee.वृत्ति, "", "no routine label at either site");

    // AND THE ONE THAT SEPARATES THEM.
    assert_eq!(label.लक्ष्य, LABEL.1, "`यन्त्रवृत्तिचिह्नम्` names its own site");
    assert_eq!(
        callee.लक्ष्य, CALLEE.1,
        "`यन्त्राह्वानोत्सर्जनम्` names its own site"
    );
    assert_ne!(
        label.लक्ष्य, callee.लक्ष्य,
        "two different defects must not produce the same record"
    );
}

/// The site codes are the module's own published constants, not numbers this test
/// chose: a renumbering that moved a constant and left a call site behind would
/// pass every assertion above.
#[test]
fn the_site_codes_are_the_modules_published_constants() {
    let it = load_emitter(None);
    for (name, want) in [
        ("यन्त्रनामस्थानचिह्नम्", LABEL.1),
        ("यन्त्रनामस्थानाह्वानम्", CALLEE.1),
    ] {
        assert_eq!(global_int(&it, name), want, "`{name}`");
    }
    // FIVE SITES, FIVE CODES, ALL DISTINCT AND NONE ०. ० is the value every site
    // passed before this change, so it must not be a legal site code: a site left
    // behind would then read as "no site" rather than as another site's.
    let codes: Vec<i128> = SITES.iter().map(|n| global_int(&it, n)).collect();
    assert_eq!(codes.len(), 5, "five sites raise `UnnamedSymbol`");
    for (n, c) in SITES.iter().zip(&codes) {
        assert!(*c > 0, "`{n}` is not ०, which means `no site recorded`");
    }
    let mut sorted = codes.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        codes.len(),
        "the five codes are distinct: {codes:?}"
    );
}

/// Every `UnnamedSymbol` site in the source passes a published site code — read off
/// the file, because a sixth site added later with a `०` there would be invisible to
/// the two run-time tests above, which reach two sites of five.
#[test]
fn every_unnamed_symbol_site_in_the_source_passes_a_site_code() {
    let src = source("yantrotsarjana.t1");
    let sites: Vec<&str> = src
        .lines()
        .filter(|l| l.contains("यन्त्रनिषेधः आरभ्य यन्त्रानामसंज्ञानिषेधभेद"))
        .collect();
    assert_eq!(
        sites.len(),
        5,
        "five sites raise `UnnamedSymbol`: {sites:?}"
    );
    for line in &sites {
        // SPLIT ON SPACES, NOT ON A WORD BOUNDARY: Python's and Rust's `\b` do not
        // work against Devanagari, and this line is read the same way the emitter's
        // own censuses read it.
        let named = SITES
            .iter()
            .any(|n| line.split_whitespace().any(|w| w == *n));
        assert!(
            named,
            "this site passes no published site code, so its refusal is \
             indistinguishable from the other four: {line}"
        );
    }
}

/// **THE CASE THAT MUST STILL BE REFUSED.** Put `०` back where BOTH site codes go
/// and the two records agree in every field again — the pre-change reading,
/// reproduced from the real source rather than described. Both sites are mutated,
/// not one: a mutant that silenced only the call site would still be told apart
/// from an untouched label site, and the collapse is the thing being reproduced.
#[test]
fn site_codes_put_back_to_zero_make_two_defects_read_alike() {
    let real = source("yantrotsarjana.t1");
    let mut mutated = real.clone();
    for (code, subject) in [(SITE_LABEL, "संज्ञा"), (SITE_CALLEE, "आह्वेयसंज्ञा")]
    {
        let before = mutated.clone();
        mutated = mutated.replace(
            &format!("ऽ ० ऽ {code} ऽ {subject} समाप्तम्"),
            &format!("ऽ ० ऽ ० ऽ {subject} समाप्तम्"),
        );
        assert_ne!(
            mutated, before,
            "the mutation found `{code}`'s site — if it did not, this test proves nothing"
        );
    }
    let mutant = Some(("yantrotsarjana.t1", mutated.as_str()));
    let label = label_record(mutant);
    let callee = callee_record(mutant);
    assert_eq!(label.लक्ष्य, 0, "the mutant passes ० again");
    assert_eq!(callee.लक्ष्य, 0, "the mutant passes ० again");
    assert_eq!(
        label, callee,
        "with ० back at both sites two different defects produce the IDENTICAL \
         record, which is the reading the ledger's `one cause` was taken from"
    );
}

/// The five published site codes, in the order the file declares them.
const SITES: &[&str] = &[
    "यन्त्रनामस्थानचिह्नम्",
    "यन्त्रनामस्थानपरीक्षा",
    "यन्त्रनामस्थानाह्वानम्",
    "यन्त्रनामस्थानवृत्तिः",
    "यन्त्रनामस्थानप्रवेशः",
];

const SITE_LABEL: &str = "यन्त्रनामस्थानचिह्नम्";
const SITE_CALLEE: &str = "यन्त्रनामस्थानाह्वानम्";

fn global_int_of(name: &str) -> i128 {
    let it = load_emitter(None);
    global_int(&it, name)
}

/// **`W-331` — `refusal_site` READS THE EMITTER'S RECORD NOW, AND THE RECORD WAS
/// ALWAYS THERE.**
///
/// The five globals every test above asserts about were filled by `यन्त्रनिषेधः`
/// and read by NOTHING outside this file. `chain::refusal_site` asked only
/// `निर्णयविरामभेद`, the DECIDE stage's verdict, so a source that resolved and
/// typechecked and was then refused by the EMITTER answered `०` —
/// `निर्णयसिद्धभेद`, "resolved AND typechecked" — and `t1_image` printed
/// `no site recorded` after spending 347,659,457 extra steps looking for one.
/// Measured 2026-09-29 while chasing `W-330`.
///
/// This drives a REAL refusal through the emitter rather than setting the globals
/// by hand: an arm tested against values a test wrote is a test of the test.
#[test]
fn the_refusal_site_names_the_emitters_own_record() {
    let mut it = load_emitter(None);
    // Site १, the same one `the_unnamed_symbol_record_names_which_of_the_five_sites_raised_it`
    // drives: a label asked for symbol ०, which no name entry holds.
    let answer = it
        .call(LABEL.0, vec![Value::Int(0)], 200_000_000)
        .unwrap_or_else(|e| panic!("`{}` runs: {e}", LABEL.0));
    assert_eq!(
        answer.as_int(),
        Some(0),
        "the site answers ० after recording"
    );

    let site = sadhana::t1::chain::refusal_site(&it)
        .expect("the emitter refused, so a site must be named");

    // THE VARIANT BY NAME, NOT BY NUMBER. `UnnamedSymbol` is kind ४
    // (`yantrotsarjana.t1:108`), and the pairing with Rust's `Refusal::UnnamedSymbol`
    // is asserted by `t1_sources.rs`, so this reads the name the reader prints.
    assert!(
        site.starts_with("emit: UnnamedSymbol (4) in "),
        "the site must name the STAGE and the VARIANT, and got: {site}"
    );
    // AND THE SITE CODE, WHICH IS THIS VARIANT'S DISCRIMINATOR — NOT THE ROUTINE.
    //
    // I ASSERTED THE ROUTINE HERE FIRST AND IT WAS THE WRONG FIELD. All five
    // `UnnamedSymbol` sites pass `रिक्तम्` for the routine on purpose
    // (`yantrotsarjana.t1:489`, `:585`, `:1282`, `:1940`, `:2597`) because no
    // routine is known at any of them; what separates them is the SITE CODE, which
    // is exactly what the first test in this file is about. So the reader must
    // print `site 1` and not `target 1`, and this asserts the field that carries
    // the information rather than the one that happens to be adjacent.
    assert!(
        site.contains(&format!("site {}", LABEL.1)),
        "the site code is this variant's discriminator and must be named: {site}"
    );
    println!("METRIC t1_emit_refusal_site_example {site}");
}

/// **AND IT MUST NOT INVENT ONE.** A fresh interpreter has refused nothing:
/// `यन्त्रनिषेधमस्ति` is `असत्यम्` and `निर्णयविरामभेद` is ३ (never entered), so
/// the honest answer is `None`. An arm keyed on the KIND rather than on the guard
/// would answer here, because `:168` initialises the variant to `०` while the
/// numbered kinds start at `१` — `०` is not "no refusal", it is "no variant".
#[test]
fn a_tree_that_refused_nothing_names_no_site() {
    let it = load_emitter(None);
    assert_eq!(
        sadhana::t1::chain::refusal_site(&it),
        None,
        "nothing has been compiled, so there is no site to name"
    );
}
