//! ॥ A DECLARATION WALK STOPS AT THE CURSOR, NOT AT THE ARENA'S LENGTH ॥
//! `SAS-007`.
//!
//! `parse.t1`'s `कार्यक्रमपठनम्` starts every program at declaration १ by
//! resetting `घोषणासूचकाङ्क` to ० — and keeps the storage. So `घोषणाकोश` holds
//! the CURRENT program in `1..=घोषणासूचकाङ्क` and, past it, whatever a LONGER
//! earlier parse left there. Those stale entries carry token indices into the
//! EARLIER program's token arena, which the lexer has since overwritten with the
//! current program's tokens.
//!
//! `ir.t1`'s `संरचनानामघोषणा` (a type name's token -> the `संरचना` declaring it)
//! walked to `व्याकरॱघोषणाकोश ॱ दैर्घ्य`. A stale `संरचना` whose name token index
//! now lands on the current program's `न६४` answered "this is a record", and
//! `चरः x ॱॱ न६४ भवति ०` was lowered to a RECORD ALLOCATION sized by the stale
//! struct's field count. In the fan-out (`t1_boot --object`, which parses every
//! source to collect them and THEN compiles one) that put seven 48-octet records
//! where `sanchaya.t1` declares integers; the linked image faulted `BadAccess`
//! at address 16 running the corpus. The monolith walks the same arena with a
//! different stale tail, so it hid the defect by the luck of its order.
//!
//! ## THE PAIR
//!
//! `पुरातनम्` declares a routine and then a six-field `संरचना` whose NAME is
//! token १७. `विषयः` declares ONE routine whose integer local's TYPE, `न६४`, is
//! also token १७. Parsing `पुरातनम्` and then `विषयः` leaves the struct at
//! declaration २, past `विषयः`'s cursor of १ — a stale entry whose name index
//! reads `न६४` in `विषयः`'s tokens. Lowering `विषयः` must allocate NOTHING:
//! the local is an integer.
//!
//! The precondition is ASSERTED, not assumed: if a change to the lexer or the
//! parser moves either token, the test says the pair no longer lines up rather
//! than going green for the wrong reason.
//!
//! The loader and the lowering follow `w279_bare_type_lowering.rs` and are
//! copied here, not shared: a loader is part of the test it serves.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

/// `ir.t1:236` — `Instruction::AllocRecord(u64)`, the octet count in `ध्रुवमूल्यम्`.
const ALLOC_RECORD: i128 = 24;

/// The modules `मध्यरूप` needs to reach IR — the same set `w279` loads.
const CHAIN: &[&str] = &[
    "lex.t1",
    "ast.t1",
    "parse.t1",
    "artha.t1",
    "sanchaya.t1",
    "sanskrit_text.t1",
    "ir.t1",
];

/// The EARLIER, LONGER program: two declarations, the second a six-field record
/// whose name is token १७ (the routine is `सार्वजनिक` and returns `० योगः ०`
/// only to put it there).
const STALE: &str = "मण्डलम् पुरातनम् ॥\n\n\
     सार्वजनिक वृत्तिः प्रथमा ददाति न६४ आदि\n    \
     प्रत्यागमनम् ० योगः ० ।\n\
     इति\n\n\
     संरचना पुरातनरचना आरभ्य\n    \
     क ॱॱ न६४ ऽ\n    \
     ख ॱॱ न६४ ऽ\n    \
     ग ॱॱ न६४ ऽ\n    \
     घ ॱॱ न६४ ऽ\n    \
     ङ ॱॱ न६४ ऽ\n    \
     च ॱॱ न६४\n\
     समाप्तम् ।\n";

/// The program being lowered: ONE declaration, an integer local whose type
/// `न६४` is token १७. Its initialiser is the literal `०` ON PURPOSE: `ir.t1`'s
/// `चरः` arm asks `संरचनानामघोषणा` only for a `भवति ०` declaration (the record
/// allocation idiom), which is also the shape of every faulting site in
/// `sanchaya.t1`.
const SUBJECT: &str = "मण्डलम् विषयः ॥\n\n\
     सार्वजनिक वृत्तिः मुख्यम् आदाय आरम्भः ॱॱ न६४ ददाति न६४ आदि\n    \
     चरः गणकः ॱॱ न६४ भवति ० ।\n    \
     गणकः भवति आरम्भः ।\n    \
     प्रत्यागमनम् गणकः ।\n\
     इति\n";

/// The token both programs put at the same index.
const ALIGNED_TOKEN: usize = 17;

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

fn int_global(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("{name} is an integer global"))
}

/// Lex and parse `src`; answer its declaration count. A clean parse is asserted
/// through `दोषसूचकाङ्क`, as `w279` does.
fn parse(it: &mut Interpreter, src: &str) -> i128 {
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("पदविभाग runs");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![toks], 4_000_000_000)
        .expect("कार्यक्रमपठनम् runs")
        .as_int()
        .expect("a declaration count");
    assert_eq!(
        int_global(it, "दोषसूचकाङ्क"),
        0,
        "the program parses clean:\n{src}"
    );
    assert!(parsed > 0, "the program declares something:\n{src}");
    parsed
}

/// The text of the current token arena's token `i`.
fn token_text(it: &mut Interpreter, i: usize) -> String {
    let Some(Value::Arena(a)) = it.global("चिह्नककोश").cloned() else {
        panic!("चिह्नककोश is an arena");
    };
    let tok = a.borrow().get(i).cloned().expect("the token exists");
    it.call("पदविभागॱचिह्नकपाठः", vec![tok], 1_000_000)
        .expect("चिह्नकपाठः runs")
        .octets()
        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
        .expect("token text is octets")
}

/// Every `AllocRecord` size `ir.t1` lowers for `SUBJECT`, after `earlier` has
/// been parsed on the same interpreter. With `earlier` empty this is the
/// control: nothing stale exists.
fn subject_alloc_sizes(earlier: Option<&str>) -> Vec<i128> {
    let mut it = load_chain();
    if let Some(src) = earlier {
        parse(&mut it, src);
    }
    let parsed = parse(&mut it, SUBJECT);
    assert_eq!(parsed, 1, "the subject declares one routine");

    if earlier.is_some() {
        // THE PRECONDITION. A stale `संरचना` past the cursor whose name index
        // reads `न६४` in the SUBJECT's tokens — the situation this file is for.
        let cursor = int_global(&it, "घोषणासूचकाङ्क");
        let struct_kind = int_global(&it, "प्रकारघोषणाभेद");
        let Some(Value::Arena(decls)) = it.global("घोषणाकोश").cloned() else {
            panic!("घोषणाकोश is an arena");
        };
        let stale: Vec<Value> = decls
            .borrow()
            .iter()
            .skip(usize::try_from(cursor).expect("a cursor") + 1)
            .cloned()
            .collect();
        let names: Vec<usize> = stale
            .iter()
            .filter(|d| int_field(d, "भेद") == Some(struct_kind))
            .filter_map(|d| int_field(d, "नामसूचकाङ्क"))
            .filter_map(|k| usize::try_from(k).ok())
            .collect();
        assert_eq!(
            names,
            vec![ALIGNED_TOKEN],
            "PRECONDITION LOST: the stale record's name is no longer token \
             {ALIGNED_TOKEN} — the pair must be re-aligned, this is not a pass"
        );
        assert_eq!(
            token_text(&mut it, ALIGNED_TOKEN),
            "न६४",
            "PRECONDITION LOST: the subject's token {ALIGNED_TOKEN} is not its \
             local's type — the pair must be re-aligned, this is not a pass"
        );
    }

    lower(&mut it, parsed)
}

/// Resolve, typecheck and lower the program `parsed` declarations long that the
/// interpreter has just parsed; answer every `AllocRecord` size, in order.
fn lower(it: &mut Interpreter, parsed: i128) -> Vec<i128> {
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
        "the subject resolves"
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
        "the subject typechecks"
    );
    it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
        .expect("मध्यरूप starts");
    it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
        .expect("कार्यक्रमरचना runs");

    let cursor = int_global(it, "आज्ञासूचकाङ्क");
    let Some(Value::Arena(insts)) = it.global("आज्ञाकोश").cloned() else {
        panic!("आज्ञाकोश is an arena");
    };
    let mut sizes = Vec::new();
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

/// THE CONTROL: nothing parsed before the subject, so nothing is stale. An
/// integer local allocates nothing. If this goes red the harness is broken.
#[test]
fn an_integer_local_allocates_nothing_with_no_earlier_parse() {
    assert_eq!(subject_alloc_sizes(None), Vec::<i128>::new());
}

/// THE SUBJECT. The same program, lowered after a longer one left a stale
/// record whose name index lands on the local's `न६४`. Before the fix this
/// answered `[48]`: six stale fields of eight octets, allocated for an integer.
#[test]
fn a_stale_record_past_the_cursor_does_not_type_an_integer_local() {
    assert_eq!(
        subject_alloc_sizes(Some(STALE)),
        Vec::<i128>::new(),
        "an integer local was lowered to a record allocation: a declaration \
         walk read past `घोषणासूचकाङ्क` into a stale entry left by the earlier \
         parse (SAS-007)"
    );
}

// ── THE BOUND'S TWO EDGES, ONE TEST EACH (coordinator review, SAS-007) ───────
//
// The test above catches a walk to the arena's LENGTH. It does not catch the two
// near misses of the right bound `आरभ्य घोषणासूचकाङ्क योगः १ समाप्तम्`: ONE TOO
// MANY (`योगः २`, which still reads the first stale entry) and EXCLUSIVE (no
// `योगः १`, which never reads the LAST live declaration). Each edge gets a case
// that only the exact bound passes, at both walkers.

/// A program whose LAST declaration is the `संरचना` its one routine's local
/// uses, so an exclusive bound in `ir.t1`'s `संरचनानामघोषणा` never reaches it
/// and the local gets no storage.
const RECORD_LAST: &str = "मण्डलम् अन्तिमः ॥\n\n\
     सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    \
     चरः धारकम् ॱॱ अन्तिमरचना भवति ० ।\n    \
     धारकम् ॱ प्रथमम् भवति ७ ।\n    \
     प्रत्यागमनम् धारकम् ॱ प्रथमम् ।\n\
     इति\n\n\
     संरचना अन्तिमरचना आरभ्य\n    \
     प्रथमम् ॱॱ न६४ ऽ\n    \
     द्वितीयम् ॱॱ न६४ ऽ\n    \
     तृतीयम् ॱॱ न६४\n\
     समाप्तम् ।\n";

/// `ir.t1:974`'s LOWER EDGE: the record declared last is found, and its three
/// fields are allocated. An exclusive bound answers `[]`.
#[test]
fn a_record_declared_last_is_found() {
    let mut it = load_chain();
    let parsed = parse(&mut it, RECORD_LAST);
    assert_eq!(parsed, 2, "the program declares a routine, then the record");
    assert_eq!(
        lower(&mut it, parsed),
        vec![24],
        "the record is the program's LAST declaration and the walk did not \
         reach it: the bound stops one short of `घोषणासूचकाङ्क` (SAS-007)"
    );
}

/// The earlier, LONGER program for `sanchaya.t1`'s `वृत्तिघोषणान्वेषणम्`: its
/// SECOND routine `सहायकः` is declaration २ and its name is token
/// `ROUTINE_TOKEN`.
const STALE_ROUTINE: &str = "मण्डलम् पुरातनम् ॥\n\n\
     सार्वजनिक वृत्तिः प्रथमा ददाति न६४ आदि\n    \
     प्रत्यागमनम् ० योगः ० ।\n\
     इति\n\n\
     सार्वजनिक वृत्तिः सहायकः ददाति न६४ आदि\n    \
     प्रत्यागमनम् ० ।\n\
     इति\n";

/// The subject: ONE routine, whose token `ROUTINE_TOKEN` is ALSO `सहायकः` (a
/// local's name, read back) — so the stale declaration २'s name reads as the
/// name the lookup asks for.
const ROUTINE_SUBJECT: &str = "मण्डलम् विषयः ॥\n\n\
     सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    \
     चरः सहायकः ॱॱ न६४ भवति ० ।\n    \
     प्रत्यागमनम् सहायकः ।\n\
     इति\n";

const ROUTINE_TOKEN: usize = 18;

/// The routine of THIS parse named `name`, or ० — `घोषणासञ्चयॱवृत्तिघोषणान्वेषणम्`.
fn routine_lookup(it: &mut Interpreter, name: &str) -> i128 {
    it.call("घोषणासञ्चयॱवृत्तिघोषणान्वेषणम्", vec![octets(name)], 400_000_000)
        .expect("वृत्तिघोषणान्वेषणम् runs")
        .as_int()
        .expect("a declaration index")
}

/// `sanchaya.t1:388`'s UPPER EDGE: a stale routine declaration just past the
/// cursor, whose name index reads `सहायकः` in the subject's tokens, is NOT this
/// parse's routine. A walk to the length, or one too many, answers २.
#[test]
fn a_stale_routine_past_the_cursor_is_not_found() {
    let mut it = load_chain();
    assert_eq!(parse(&mut it, STALE_ROUTINE), 2);
    assert_eq!(parse(&mut it, ROUTINE_SUBJECT), 1);
    // THE PRECONDITION: declaration २ is stale, a routine, named at
    // ROUTINE_TOKEN, and that token now reads `सहायकः`.
    let routine_kind = int_global(&it, "वृत्तिघोषणाभेद");
    let Some(Value::Arena(decls)) = it.global("घोषणाकोश").cloned() else {
        panic!("घोषणाकोश is an arena");
    };
    let stale = decls.borrow().get(2).cloned().expect("a stale entry at २");
    assert_eq!(
        (int_field(&stale, "भेद"), int_field(&stale, "नामसूचकाङ्क")),
        (Some(routine_kind), i128::try_from(ROUTINE_TOKEN).ok()),
        "PRECONDITION LOST: the stale routine is not at token {ROUTINE_TOKEN} — re-align"
    );
    assert_eq!(
        token_text(&mut it, ROUTINE_TOKEN),
        "सहायकः",
        "PRECONDITION LOST: the subject's token {ROUTINE_TOKEN} is not `सहायकः` — re-align"
    );
    assert_eq!(
        routine_lookup(&mut it, "सहायकः"),
        0,
        "a routine lookup answered a STALE declaration past `घोषणासूचकाङ्क` \
         (SAS-007): the walk reads past the cursor"
    );
}

/// `sanchaya.t1:388`'s LOWER EDGE: this parse's LAST declaration is found. An
/// exclusive bound answers ०.
#[test]
fn the_last_routine_of_the_parse_is_found() {
    let mut it = load_chain();
    assert_eq!(parse(&mut it, STALE_ROUTINE), 2);
    assert_eq!(
        routine_lookup(&mut it, "सहायकः"),
        2,
        "the program's LAST routine was not found: the bound stops one short \
         of `घोषणासूचकाङ्क` (SAS-007)"
    );
    assert_eq!(
        routine_lookup(&mut it, "प्रथमा"),
        1,
        "the control: the first routine"
    );
}
