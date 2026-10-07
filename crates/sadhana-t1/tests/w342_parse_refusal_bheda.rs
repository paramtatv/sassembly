//! **W-342 — `पठनम्` ANSWERS ० FOR A THIRD CAUSE, AND IT WAS THE ONE NOBODY
//! ASKED ABOUT.** W-345 told a repertoire refusal (८) apart from a source that
//! declares nothing (१). A source the PARSER refuses comes back from `पठनम्` as
//! ० too, and `मण्डलसङ्कलनम्` asked only the repertoire gate: the parse refusal
//! was recorded as `सङ्कलनाघोषणाभेद` (१, "declared nothing"), the corpus walk
//! counted it into `सङ्कलनरिक्तसंख्या` ("not a failure") and never named it.
//!
//! MEASURED ON `393532c5` BEFORE THIS FILE, through `t1_image` with the exit
//! code read directly: the fixture below, built with no entry, printed
//! `build: 0 source(s) failed to compile, 1 declared nothing, 1 object(s)
//! linked (startup included)`, wrote a 65,688-octet ELF of the startup alone
//! and exited ZERO. The source had been refused at line 3.
//!
//! THE PARSER'S OWN RECORD IS THE DISCRIMINATOR, AND IT EXISTED: after the
//! fixture `दोषसूचकाङ्क` read १ and `दोषकोश[१]` held line ३ and the reason
//! `अन्तः अपेक्षितम्`. What was missing is the exit kind:
//! `सङ्कलनव्याकरणभेद` (९).
//!
//! **AND THE RECORD WENT STALE, WHICH IS WHY THE FIX IS TWO LINES AND NOT
//! ONE.** `पठनम्` returns at zero tokens before `कार्यक्रमपठनम्` runs, and
//! `कार्यक्रमपठनम्` is the only routine that resets `दोषसूचकाङ्क`. Measured: a
//! comment-only source compiled right after the fixture still read
//! `दोषसूचकाङ्क` = १. A caller that asked the record without a reset would
//! have called `lib.t1` a parse refusal in every build where it follows one.
//! So `पठनम्` resets the count before the gate, and the second test below is
//! the case that must keep reading १.
//!
//! WHAT THIS FILE DOES NOT CLAIM: that the fixture SHOULD be refused. The
//! fixed-capacity array is ADR-0026's accepted production and the product's
//! parser not reading it is the row's half (a), untouched here. The fixture is
//! used because it is a parse refusal that exists today; when (a) is fixed
//! this file needs another one, and the first test says so when it fails.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

const FUEL: u64 = 80_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

fn int(it: &Interpreter, g: &str) -> i128 {
    match it.global(g) {
        Some(Value::Int(k)) => *k,
        other => panic!("{g} is {other:?}, not an integer"),
    }
}

fn compile(it: &mut Interpreter, module: &str, src: &str) -> Vec<u8> {
    it.call(
        "शृङ्खलाॱमण्डलसङ्कलनम्",
        vec![octets(src.as_bytes()), octets(module.as_bytes())],
        FUEL,
    )
    .expect("मण्डलसङ्कलनम् runs")
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default()
}

/// The parser's first recorded refusal: its line and its reason.
fn first_parse_error(it: &Interpreter) -> (i128, String) {
    let store = match it.global("दोषकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("`दोषकोश` is an arena, not {other:?}"),
    };
    let Value::Record(r) = &store[1] else {
        panic!("`दोषकोश[१]` is a record, not {:?}", store[1]);
    };
    let r = r.borrow();
    let line = match r.get("पङ्क्ति") {
        Some(Value::Int(n)) => *n,
        other => panic!("`पङ्क्ति` is a number, not {other:?}"),
    };
    let reason = match r.get("कारण") {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("`कारण` is text, not {other:?}"),
    };
    (line, reason)
}

/// ADR-0026's fixed-capacity array on line 3. The product's parser stops at
/// the numeral — "`अन्तः` expected" — and every other line is ordinary.
const FIXED: &str = "मण्डलम् सीमितकोश ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः १०२४ अन्तः न६४ भवति ० ।
    क अङ्कः ३ अन्तः भवति ७ ।
    प्रत्यागमनम् क अङ्कः ३ अन्तः ।
इति
";

/// THE CONTROL: the same source one token apart, the bound left out. It must
/// keep compiling, or the fixture above pins a broken compiler rather than a
/// refused production.
const SLICE: &str = "मण्डलम् सीमितकोश ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ३ अन्तः भवति ७ ।
    प्रत्यागमनम् क अङ्कः ३ अन्तः ।
इति
";

/// `lib.t1`'s shape: margins only — zero tokens, so `पठनम्` returns before the
/// parser is ever called.
const NOTHING: &str = "॰ a file that declares nothing, as `lib.t1` does\n";

/// One Latin `x`: W-345's cause, which must keep its own kind beside this one.
const LATIN: &str = "मण्डलम् परिधिपरीक्षा ॥
सार्वजनिक वृत्तिः मुख्यx ददाति न६४ आदि
    प्रत्यागमनम् २१ ।
इति
";

#[test]
fn a_parse_refusal_records_its_own_kind_not_no_declarations() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = compile(&mut it, "सीमितकोश", FIXED);
    assert!(text.is_empty(), "a source the parser refused emits no text");
    assert!(
        int(&it, "दोषसूचकाङ्क") > 0,
        "THE FIXTURE MUST ACTUALLY BE A PARSE REFUSAL, or this file pins nothing. If \
         this reads ०, half (a) of W-342 has landed and the product's parser now reads \
         ADR-0026's array type: give this file another source the parser refuses"
    );
    assert_eq!(
        int(&it, "सङ्कलनविरामभेद"),
        9,
        "`सङ्कलनव्याकरणभेद` — the parser refused it, NOT `१` (declared nothing): the \
         third cause behind `पठनम्`'s ० must be told apart from the record"
    );
    // The site, not just the kind.
    let (line, reason) = first_parse_error(&it);
    assert_eq!(
        line, 3,
        "the parser's record names the line of the array type"
    );
    assert_eq!(reason, "अन्तः अपेक्षितम्", "…and why it stopped there");
}

/// **THE CASE THAT MUST STILL READ १.** Same interpreter, immediately after the
/// parse refusal — the only order that can catch a stale parser record.
/// `कार्यक्रमपठनम्` resets `दोषसूचकाङ्क`, and a zero-token source never reaches
/// it; before the reset in `पठनम्` the count read १ here, left over from the
/// source before.
#[test]
fn a_declaration_less_source_after_a_parse_refusal_still_reads_one() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    assert!(
        compile(&mut it, "सीमितकोश", FIXED).is_empty(),
        "the refused module compiles FIRST, so the empty one runs over its record"
    );
    assert!(int(&it, "दोषसूचकाङ्क") > 0, "…and that record exists");
    assert!(compile(&mut it, "रिक्त", NOTHING).is_empty());
    assert_eq!(
        int(&it, "दोषसूचकाङ्क"),
        0,
        "the parser's count belongs to THIS source, which has no token to refuse — a \
         १ here is the previous source's refusal left standing"
    );
    assert_eq!(
        int(&it, "सङ्कलनविरामभेद"),
        1,
        "`सङ्कलनाघोषणाभेद` — a declaration-less source is not a parse refusal, \
         whatever was compiled before it"
    );
}

/// The two refusals keep their own kinds in either order: the repertoire gate
/// is asked first, and its record and the parser's are both reset per source.
#[test]
fn a_repertoire_refusal_and_a_parse_refusal_do_not_read_as_each_other() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    assert!(compile(&mut it, "सीमितकोश", FIXED).is_empty());
    assert_eq!(int(&it, "सङ्कलनविरामभेद"), 9, "the parse refusal, first");
    assert!(compile(&mut it, "परिधिपरीक्षा", LATIN).is_empty());
    assert_eq!(
        int(&it, "सङ्कलनविरामभेद"),
        8,
        "a source that never lexed is `सङ्कलनपरिधिभेद` even right after a parse refusal"
    );
    assert!(compile(&mut it, "सीमितकोश", FIXED).is_empty());
    assert_eq!(
        int(&it, "सङ्कलनविरामभेद"),
        9,
        "and a parse refusal right after a repertoire refusal is still ९"
    );
}

/// THE CONTROL, AND IT RUNS AFTER THE REFUSAL ON PURPOSE: a source that parses
/// must compile, with the record clean, on an interpreter that has just
/// refused one. A fix that failed everything would pass the three tests above.
#[test]
fn the_same_source_without_the_bound_still_compiles() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    assert!(compile(&mut it, "सीमितकोश", FIXED).is_empty());
    let text = compile(&mut it, "सीमितकोश", SLICE);
    assert!(!text.is_empty(), "the plain slice emits its text");
    assert_eq!(int(&it, "सङ्कलनविरामभेद"), 0, "`सङ्कलनसिद्धभेद` — emitted");
    assert_eq!(int(&it, "दोषसूचकाङ्क"), 0, "and the parser recorded nothing");
}

/// **THE CORPUS WALK: A PARSE-REFUSED SOURCE IS A FAILURE WITH A NAME, NOT AN
/// EMPTY SOURCE.** This is the half that reached the exit code: before the
/// kind existed `मण्डलानिप्रतिबिम्बम्` filed the source under
/// `सङ्कलनरिक्तसंख्या`, `सङ्कलनविफलसंख्या` stayed ०, and `t1_image`'s
/// `failed > 0` exit (W-343) never fired.
#[test]
fn the_image_driver_counts_a_parse_refusal_as_a_failure_and_names_it() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let _ = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(FIXED.as_bytes()), octets(NOTHING.as_bytes())]),
                arena(vec![
                    octets("सीमितकोश".as_bytes()),
                    octets("रिक्त".as_bytes()),
                ]),
                Value::Int(2),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs");
    assert_eq!(
        int(&it, "सङ्कलनविफलसंख्या"),
        1,
        "the parse-refused source IS a failure — before W-342 this read ०"
    );
    assert_eq!(
        int(&it, "सङ्कलनरिक्तसंख्या"),
        1,
        "and the declaration-less source AFTER it is still counted apart, not as a \
         second parse refusal"
    );
    let names: Vec<String> = match it.global("सङ्कलनविफलनामकोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter_map(|v| v.octets())
            .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            .filter(|n| !n.is_empty())
            .collect(),
        other => panic!("`सङ्कलनविफलनामकोश` is an arena, not {other:?}"),
    };
    assert_eq!(names, ["सीमितकोश"], "the failed source is NAMED");
}
