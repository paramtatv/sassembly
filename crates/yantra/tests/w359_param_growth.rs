//! **`W-359`: A RUN PARAMETER GROWS ONLY IN A ROUTINE THAT RETURNS IT.**
//!
//! The defect (row `W-359`, `tests/probes/callee.t1probe`): a callee wrote past
//! the end of a run it received as a parameter — interpreted ९९, natively ०.
//! Natively growth cuts a NEW block and writes its base back into the callee's
//! own slot, so the caller keeps the old run; the interpreter shares the arena.
//!
//! The coordinator's rule (2026-10-04), option (4) of the definition exchange:
//!
//! 1. A routine is GROW-AND-RETURN for a run parameter when it returns that
//!    parameter on every path (`शून्यम्` returns admitted). Its stores into
//!    that parameter keep today's lowering, growth included.
//! 2. In every other routine a store into a run parameter at an index at or
//!    past the parameter's LENGTH is refused by name at run time on both
//!    engines: interpreted, an error whose cause text is pinned below;
//!    natively, the code `0x359` (`ir.t1`'s `प्राचलसीमानिषेधः`) in FAIL form,
//!    `0x3333 | 0x359 << 16`, so `yantra` halts `Finisher { status: Some(0x359) }`
//!    and QEMU stops too (`W-381`; the raw word pinned until 2026-10-06 QEMU ran past).
//! 3. A call statement that DROPS a grow-and-return result is refused
//!    statically by both front ends, across modules through the grows flag of
//!    the collected-declaration store.
//!
//! The native side is compiled HERE by the current `.t1` compiler running in
//! the interpreter (`CHAIN`), so the image carries the lowering this tree says.
//! Probes: `crates/sadhana-t1/tests/probes/w359_*.t1probe`.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, RunError, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

const FUEL: u64 = 80_000_000_000;

/// `ir.t1`'s `प्राचलसीमानिषेधः`, the native refusal's CODE, its status.
const REFUSAL_CODE: u64 = 0x359;
/// The finisher word: the code in FAIL form (`W-381`; it was the raw code).
const REFUSAL_WORD: u64 = (REFUSAL_CODE << 16) | 0x3333;

const PAST_END: &str = include_str!("../../sadhana-t1/tests/probes/w359_past_end.t1probe");
const FRESH: &str = include_str!("../../sadhana-t1/tests/probes/w359_fresh_param.t1probe");
const WITHIN: &str = include_str!("../../sadhana-t1/tests/probes/w359_within.t1probe");
const KOSHA: &str = include_str!("../../sadhana-t1/tests/probes/w359_kosha_shape.t1probe");
const DROPPED: &str = include_str!("../../sadhana-t1/tests/probes/w359_dropped.t1probe");
const BOUND: &str = include_str!("../../sadhana-t1/tests/probes/w359_bound.t1probe");
const REMOTE_GROWER: &str =
    include_str!("../../sadhana-t1/tests/probes/w359_remote_grower.t1probe");
const REMOTE_DROPPER: &str =
    include_str!("../../sadhana-t1/tests/probes/w359_remote_dropper.t1probe");

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

fn load(sources: &[(&str, &str)]) -> Result<Interpreter, RunError> {
    Interpreter::load(sources, Path::new("."))
}

fn interpret(src: &str) -> Result<Value, RunError> {
    let mut it =
        load(&[("probe", src)]).unwrap_or_else(|e| panic!("the probe must load: {}", e.reason));
    it.call("मुख्यम्", Vec::new(), 1_000_000)
}

/// The first line of a run error — the cause, without the `.t1` backtrace.
fn cause(e: &RunError) -> &str {
    e.reason.lines().next().unwrap_or("")
}

/// Compile `(module, source)` pairs with the given compiler sources; answer
/// the image (empty when the product refused) and the compiling interpreter,
/// whose globals carry the front end's refusal records.
fn compile_with(
    chain: &[(&str, &str)],
    sources: &[(&str, &str)],
    entry_module: &str,
) -> (Vec<u8>, Interpreter) {
    let mut it = Interpreter::load(chain, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(entry_module.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let srcs = sources.iter().map(|(_, s)| octets(s.as_bytes())).collect();
    let names = sources.iter().map(|(m, _)| octets(m.as_bytes())).collect();
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![arena(srcs), arena(names), Value::Int(sources.len() as i128)],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    (image, it)
}

/// Run an image; answer how it halted and how many instructions it executed.
fn run(image: &[u8]) -> (Halt, u64) {
    let mut m = Machine::load_elf(image, yantra::ram_for(image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let mut steps: u64 = 0;
    loop {
        steps += 1;
        if let Some(h) = m.step(&mut out) {
            return (h, steps);
        }
        assert!(steps < 400_000_000, "the image did not halt");
    }
}

fn native_with(chain: &[(&str, &str)], module: &str, src: &str) -> Halt {
    let (image, _) = compile_with(chain, &[(module, src)], module);
    assert!(!image.is_empty(), "the probe built no image");
    run(&image).0
}

fn native(module: &str, src: &str) -> Halt {
    native_with(CHAIN, module, src)
}

fn status(h: &Halt) -> u64 {
    match h {
        Halt::Finisher {
            status: Some(s), ..
        } => *s,
        other => panic!("the image did not finish with a status: {other:?}"),
    }
}

fn is_native_refusal(h: &Halt) -> bool {
    matches!(
        h,
        Halt::Finisher {
            value: REFUSAL_WORD,
            status: Some(REFUSAL_CODE)
        }
    )
}

fn truth(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(v) => v.as_int().is_some_and(|i| i != 0),
        None => false,
    }
}

fn text(v: Option<&Value>) -> String {
    v.and_then(Value::octets)
        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
        .unwrap_or_default()
}

// ── rule (2): the run-time refusal ────────────────────────────────────────

#[test]
fn a_store_past_the_end_of_a_guarded_parameter_is_refused_by_both_engines() {
    match interpret(PAST_END) {
        Err(e) => assert_eq!(
            cause(&e),
            "entry 5 is past the end of the parameter `सूची`, a run of 1, and \
             `परलेखकः` does not return it (W-359)",
            "the interpreter's cause text"
        ),
        Ok(v) => panic!("the interpreter answered {v:?}; the store must be refused"),
    }
    let h = native("प्राचलसीमापरीक्षण", PAST_END);
    assert!(
        is_native_refusal(&h),
        "natively the store must halt with the finisher word 0x{REFUSAL_WORD:x}, \
         status 0x{REFUSAL_CODE:x}; it halted {h:?}"
    );
}

#[test]
fn a_store_into_a_fresh_parameter_is_refused_by_both_engines() {
    match interpret(FRESH) {
        Err(e) => assert_eq!(
            cause(&e),
            "entry 0 is past the end of the parameter `सूची`, a run of 0, and \
             `परलेखकः` does not return it (W-359)"
        ),
        Ok(v) => panic!("the interpreter answered {v:?}; the store must be refused"),
    }
    let h = native("प्राचलसीमापरीक्षण", FRESH);
    assert!(is_native_refusal(&h), "natively: {h:?}");
}

#[test]
fn a_store_within_the_length_passes_on_both_engines() {
    let interpreted = interpret(WITHIN)
        .unwrap_or_else(|e| panic!("the control must run: {}", e.reason))
        .as_int()
        .expect("a number");
    assert_eq!(interpreted, 105, "९९ written by the callee, plus length ६");
    assert_eq!(
        status(&native("प्राचलसीमापरीक्षण", WITHIN)),
        105,
        "natively the in-length store must pass and reach the caller"
    );
}

/// The mutant sends the bound test's TRUE edge to the store instead of the
/// refusal; the refusal check must then FAIL — without this, a check passing
/// on any halt would pass.
#[test]
fn the_mutant_that_lets_the_store_through_is_caught() {
    const LIVE: &str = "शाखारचना आरभ्य सीमाशर्तः ऽ निषेधपर्व ऽ लेखपर्व समाप्तम्";
    const MUTANT: &str = "शाखारचना आरभ्य सीमाशर्तः ऽ लेखपर्व ऽ लेखपर्व समाप्तम्";
    let ir = CHAIN
        .iter()
        .find(|(n, _)| *n == "ir.t1")
        .map(|(_, s)| *s)
        .expect("CHAIN carries ir.t1");
    assert_eq!(
        ir.matches(LIVE).count(),
        1,
        "the mutation must match exactly one site, `प्राचलसीमारचना`'s bound branch"
    );
    let mutated = ir.replacen(LIVE, MUTANT, 1);
    let chain: Vec<(&str, &str)> = CHAIN
        .iter()
        .map(|(n, s)| {
            if *n == "ir.t1" {
                (*n, mutated.as_str())
            } else {
                (*n, *s)
            }
        })
        .collect();
    let h = native_with(&chain, "प्राचलसीमापरीक्षण", PAST_END);
    assert!(
        !is_native_refusal(&h),
        "the mutant must NOT refuse; it halted {h:?}"
    );
    assert_eq!(
        status(&h),
        7,
        "the mutant lets the store through and answers index ०"
    );
}

#[test]
fn archive_header_shape_is_refused_by_name() {
    match interpret(KOSHA) {
        Err(e) => assert_eq!(
            cause(&e),
            "entry 19 is past the end of the parameter `फलम्`, a run of 15, and \
             `शीर्षम्` does not return it (W-359)"
        ),
        Ok(v) => panic!("the interpreter answered {v:?}; index १९ of १५ must be refused"),
    }
    let h = native("कोशशीर्षपरीक्षण", KOSHA);
    assert!(is_native_refusal(&h), "natively: {h:?}");
}

// ── rule (1): which routines grow and return ──────────────────────────────

/// The corpus's convention after the final ruling's rewrites, read by BOTH
/// twins: the interpreter's classification over the loaded corpus and the
/// `.t1` store's grows flag (`घोषणासञ्चयॱवर्धकप्राचलः`, written by `सङ्ग्रहः`).
/// The six the first ruling named, the two `सम्भाव्य` writers' caller
/// `संस्कारः` (by transitivity, through its tail return), and the three
/// routines rewritten to grow their parameter instead of a local copy.
#[test]
fn the_corpus_grow_and_return_routines_agree_on_both_twins() {
    let it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let mut interpreted: Vec<(String, String)> = it
        .routines()
        .filter_map(|r| r.grows().map(|p| (r.name.clone(), p.to_string())))
        .collect();
    interpreted.sort();
    let mut expected: Vec<(String, String)> = [
        ("नामस्थानयोजनम्", "नामानि"),
        ("अष्टाष्टकनिधानम्", "दत्तम्"),
        ("चतुरष्टकनिधानम्", "दत्तम्"),
        ("अष्टकवर्धनम्", "पाठ्यम्"),
        ("संस्कारः", "पाठ्यम्"),
        ("अष्टकसंयोगः", "लक्ष्यम्"),
        ("दत्तपुनःस्थापनम्", "दत्तम्"),
        ("पाठपुनःस्थापनम्", "पाठ्यम्"),
        ("शोधनसंस्कारः", "अष्टकाः"),
        ("विन्यासावृत्तिः", "स्थानानि"),
    ]
    .iter()
    .map(|(r, p)| ((*r).to_string(), (*p).to_string()))
    .collect();
    expected.sort();
    assert_eq!(
        interpreted, expected,
        "the interpreter's grow-and-return routines"
    );

    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    for (name, src) in CHAIN {
        it.call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], FUEL)
            .unwrap_or_else(|e| panic!("{name} parses: {}", e.reason));
    }
    let entries = it
        .global("प्रविष्टिसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the store's cursor");
    let mut stored: Vec<(String, String)> = Vec::new();
    for e in 1..=entries {
        let k = it
            .call(
                "घोषणासञ्चयॱप्रविष्टिवर्धकप्राचलः",
                vec![Value::Int(e)],
                1_000_000,
            )
            .expect("the flag reads")
            .as_int()
            .unwrap_or(0);
        if k > 0 {
            let name = it
                .call("घोषणासञ्चयॱप्रविष्टिनाम", vec![Value::Int(e)], 1_000_000)
                .expect("the name reads");
            let param = it
                .call(
                    "घोषणासञ्चयॱप्राचलप्रविष्टिनाम",
                    vec![Value::Int(e), Value::Int(k - 1)],
                    1_000_000,
                )
                .expect("the parameter reads");
            stored.push((text(Some(&name)), text(Some(&param))));
        }
    }
    stored.sort();
    assert_eq!(
        stored, expected,
        "the .t1 store's grows flags must name the same routines and parameters"
    );
}

#[test]
fn a_two_parameter_routine_grows_one_and_bounds_the_other() {
    let ok = include_str!("../../sadhana-t1/tests/probes/w359_two_params.t1probe");
    let past = include_str!("../../sadhana-t1/tests/probes/w359_two_params_past.t1probe");
    assert_eq!(interpret(ok).expect("runs").as_int(), Some(74));
    assert_eq!(status(&native("द्विप्राचलपरीक्षण", ok)), 74);
    match interpret(past) {
        Err(e) => assert_eq!(
            cause(&e),
            "entry 5 is past the end of the parameter `पृष्ठम्`, a run of 2, and `द्वयम्` \
             does not return it (W-359)"
        ),
        Ok(v) => panic!("answered {v:?}"),
    }
    assert!(is_native_refusal(&native("द्विप्राचलपरीक्षण", past)));
}

#[test]
fn an_octet_run_parameter_is_bounded_on_both_engines() {
    let src = include_str!("../../sadhana-t1/tests/probes/w359_octet_param.t1probe");
    match interpret(src) {
        Err(e) => assert_eq!(
            cause(&e),
            "entry 3 is past the end of the parameter `पाठ`, a run of 1, and `परलेखकः` \
             does not return it (W-359)"
        ),
        Ok(v) => panic!("answered {v:?}"),
    }
    assert!(is_native_refusal(&native("अष्टकप्राचलपरीक्षण", src)));
}

/// A nil return no longer counts as returning the parameter, so a routine
/// that grows and may answer `शून्यम्` is not grow-and-return: its growth is a
/// guarded store and is refused at run time on both engines.
#[test]
fn a_routine_that_may_return_nil_does_not_grow_its_parameter() {
    let src = include_str!("../../sadhana-t1/tests/probes/w359_nil_return.t1probe");
    match interpret(src) {
        Err(e) => assert_eq!(
            cause(&e),
            "entry 1 is past the end of the parameter `सूची`, a run of 1, and `योजकः` \
             does not return it (W-359)"
        ),
        Ok(v) => panic!("answered {v:?}"),
    }
    assert!(is_native_refusal(&native("शून्यप्रतिफलपरीक्षण", src)));
}

// ── rule (A): the result goes back to its own name, or is tail-returned ───

/// The interpreter refuses at load with `cause`; the `.t1` checker refuses
/// the program and records kind `kind` naming a token ending `name`.
fn refused_statically(
    sources: &[(&str, &str)],
    entry: &str,
    cause_text: &str,
    kind: i128,
    name: &str,
) {
    let labelled: Vec<(&str, &str)> = sources.iter().map(|(_, s)| ("probe", *s)).collect();
    match load(&labelled) {
        Err(e) => assert_eq!(cause(&e), cause_text, "the interpreter's refusal"),
        Ok(_) => panic!("the interpreter loaded it"),
    }
    let (image, it) = compile_with(CHAIN, sources, entry);
    assert!(
        image.is_empty(),
        "the .t1 front end must refuse the program"
    );
    assert!(truth(it.global("वर्धनबन्धदोषमस्ति")), "the checker's record");
    assert_eq!(
        it.global("वर्धनबन्धदोषभेद").and_then(Value::as_int),
        Some(kind),
        "the checker's refusal kind"
    );
    let named = text(it.global("वर्धनबन्धदोषनाम"));
    assert!(named.ends_with(name), "the checker names `{named}`");
}

const MISUSE: &str = "`त्यक्तवर्धनपरीक्षणॱमुख्यम्` uses the result of `त्यक्तवर्धनपरीक्षणॱयोजकः`, \
     which grows its parameter `सूची`, other than by assigning it back to the name passed \
     for `सूची` or tail-returning it (W-359)";

#[test]
fn a_dropped_grow_and_return_result_is_refused_by_both_front_ends() {
    refused_statically(
        &[("त्यक्तवर्धनपरीक्षण", DROPPED)],
        "त्यक्तवर्धनपरीक्षण",
        MISUSE,
        1,
        "योजकः",
    );
}

#[test]
fn a_grown_run_bound_to_another_name_is_refused_by_both_front_ends() {
    let src = include_str!("../../sadhana-t1/tests/probes/w359_bound_elsewhere.t1probe");
    refused_statically(
        &[("त्यक्तवर्धनपरीक्षण", src)],
        "त्यक्तवर्धनपरीक्षण",
        MISUSE,
        1,
        "योजकः",
    );
}

#[test]
fn a_tail_return_from_a_routine_that_does_not_grow_that_name_is_refused() {
    let src = include_str!("../../sadhana-t1/tests/probes/w359_bad_tail.t1probe");
    refused_statically(
        &[("त्यक्तवर्धनपरीक्षण", src)],
        "त्यक्तवर्धनपरीक्षण",
        "`त्यक्तवर्धनपरीक्षणॱमिश्रः` tail-returns `त्यक्तवर्धनपरीक्षणॱयोजकः`, which grows its \
         parameter `सूची`, but does not itself grow and return the name it passes (W-359)",
        2,
        "योजकः",
    );
}

#[test]
fn assigned_back_and_tail_returned_results_pass_on_both() {
    let tail = include_str!("../../sadhana-t1/tests/probes/w359_tail_return.t1probe");
    assert_eq!(interpret(BOUND).expect("runs").as_int(), Some(2));
    assert_eq!(status(&native("त्यक्तवर्धनपरीक्षण", BOUND)), 2);
    assert_eq!(interpret(tail).expect("runs").as_int(), Some(3));
    assert_eq!(status(&native("त्यक्तवर्धनपरीक्षण", tail)), 3);
}

#[test]
fn a_dropped_result_across_modules_is_refused_by_both_front_ends() {
    // The dropper compiles LAST, so the checker's record is its own.
    refused_statically(
        &[("दूरवर्धक", REMOTE_GROWER), ("दूरत्यागी", REMOTE_DROPPER)],
        "दूरत्यागी",
        "`दूरत्यागीॱमुख्यम्` uses the result of `दूरवर्धकॱयोजकः`, which grows its parameter \
         `सूची`, other than by assigning it back to the name passed for `सूची` or \
         tail-returning it (W-359)",
        1,
        "योजकः",
    );
}

/// THE TWO TWINS CUT A TAIL-CALL CHAIN AT THE SAME DEPTH (delta review): a
/// callee is classified down to depth ८. Eight tail calls: the root is
/// grow-and-return on both engines and the caller's run grows. Nine: the root
/// is not, on either, so its tail return is refused by both front ends.
#[test]
fn both_twins_follow_a_tail_call_chain_to_the_same_depth() {
    let eight = include_str!("../../sadhana-t1/tests/probes/w359_chain8.t1probe");
    let nine = include_str!("../../sadhana-t1/tests/probes/w359_chain9.t1probe");
    let it = load(&[("probe", eight)]).expect("eight deep loads");
    assert_eq!(
        it.routine("स्तरक")
            .and_then(|r| r.grows().map(str::to_string)),
        Some("सूची".to_string()),
        "eight deep, the interpreter's root grows and returns"
    );
    assert_eq!(interpret(eight).expect("runs").as_int(), Some(1));
    assert_eq!(status(&native("स्तरपरीक्षण", eight)), 1, "and natively");
    refused_statically(
        &[("स्तरपरीक्षण", nine)],
        "स्तरपरीक्षण",
        "`स्तरपरीक्षणॱस्तरक` tail-returns `स्तरपरीक्षणॱस्तरख`, which grows its parameter \
         `सूची`, but does not itself grow and return the name it passes (W-359)",
        2,
        "स्तरख",
    );
}

// ── rule (C): no store or growth through a local alias ────────────────────

#[test]
fn a_store_through_a_local_alias_of_a_parameter_is_refused() {
    let src = include_str!("../../sadhana-t1/tests/probes/w359_alias.t1probe");
    refused_statically(
        &[("उपनामपरीक्षण", src)],
        "उपनामपरीक्षण",
        "`उपनामपरीक्षणॱलेखकः` stores into or grows `छाया`, a local alias of a run parameter \
         (W-359)",
        3,
        "छाया",
    );
    let read = include_str!("../../sadhana-t1/tests/probes/w359_alias_read.t1probe");
    assert_eq!(interpret(read).expect("runs").as_int(), Some(7));
    assert_eq!(status(&native("उपनामपरीक्षण", read)), 7);
}

// ── fail closed (B) ───────────────────────────────────────────────────────

// An UNRESOLVED callee cannot reach `refuse_unbound_growth` from source: the
// interpreter's parser builds a call only for a name some loaded module
// declares (`sigs`), so an undeclared name is a variable and is refused at
// run time as "not a name in scope", never executed as a call. The arm that
// makes such a caller refuse is therefore unreachable, and the test below
// asserts the corpus count of it — zero.

/// THE FAIL-CLOSED ARMS ARE UNREACHABLE IN THE CORPUS, asserted rather than
/// assumed: no corpus routine is refused for an unresolved callee, and the
/// `.t1` front half, compiling every corpus source after collecting them all,
/// records no rule (A)/(C) refusal (kinds १–४, kind ४ being a callee with no
/// declaration and no store entry) and no unboundable parameter store (no
/// element width, or a slot past the mask) — `मध्यरूपॱप्राचलरक्षादोषसंख्या`.
#[test]
fn the_fail_closed_arms_are_unreachable_in_the_corpus() {
    let it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let refused: Vec<String> = it
        .routines()
        .filter(|r| r.why_not().is_some_and(|w| w.contains("(W-359)")))
        .map(|r| r.name.clone())
        .collect();
    assert_eq!(
        refused,
        Vec::<String>::new(),
        "corpus routines refused for an unresolved callee"
    );

    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    for (name, src) in CHAIN {
        it.call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], FUEL)
            .unwrap_or_else(|e| panic!("{name} parses: {}", e.reason));
    }
    it.call("अर्थॱसञ्चयसिद्धिः", Vec::new(), 1_000_000)
        .expect("the store is asserted");
    let mut compiled = 0;
    for (name, src) in CHAIN {
        let module = sadhana::t1::chain::module_name(src).unwrap_or_default();
        let text = it
            .call(
                "शृङ्खलाॱमण्डलसङ्कलनम्",
                vec![octets(src.as_bytes()), octets(module.as_bytes())],
                FUEL,
            )
            .unwrap_or_else(|e| panic!("{name} compiles: {}", e.reason));
        let bandha = it.global("वर्धनबन्धदोषसंख्या").and_then(Value::as_int);
        let raksha = it.global("प्राचलरक्षादोषसंख्या").and_then(Value::as_int);
        assert_eq!(bandha, Some(0), "{name}: rule (A)/(C) refusals");
        assert_eq!(raksha, Some(0), "{name}: unboundable parameter stores");
        if text.octets().is_some_and(|o| !o.as_slice().is_empty()) {
            compiled += 1;
        }
    }
    assert!(
        compiled >= 17,
        "the corpus compiled: {compiled} of {}",
        CHAIN.len()
    );
}

// ── acceptance (a): the cost of a parameter store ─────────────────────────

fn devanagari(n: u64) -> String {
    n.to_string()
        .chars()
        .map(|c| char::from_u32(0x966 + c.to_digit(10).expect("a digit")).expect("a digit"))
        .collect()
}

/// A non-returning callee loops `n` times; with `store` each iteration writes
/// its PARAMETER within the length the caller gave it (५१२).
fn cost_probe(store: bool, n: u64) -> String {
    let line = if store {
        "        सूची अङ्कः क अन्तः भवति क ।\n"
    } else {
        ""
    };
    format!(
        "मण्डलम् प्राचलव्ययमापन ॥\n\
         वृत्तिः लेखकः आदाय सूची ॱॱ अङ्कः अन्तः न६४ ऽ परिमाणम् ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   चरः क ॱॱ न६४ भवति ० ।\n\
         \x20   यावत् क न्यूनम् परिमाणम् आदि\n\
         {line}\
         \x20       क भवति क योगः १ ।\n\
         \x20   इति\n\
         \x20   प्रत्यागमनम् क ।\n\
         इति\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
         \x20   चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n\
         \x20   चरः ज ॱॱ न६४ भवति ० ।\n\
         \x20   यावत् ज न्यूनम् ५१२ आदि\n\
         \x20       सूची अङ्कः ज अन्तः भवति ० ।\n\
         \x20       ज भवति ज योगः १ ।\n\
         \x20   इति\n\
         \x20   चरः उ ॱॱ न६४ भवति लेखकः सूची {} ।\n\
         \x20   प्रत्यागमनम् उ ।\n\
         इति\n",
        devanagari(n)
    )
}

fn cost_steps(store: bool, n: u64) -> u64 {
    let src = cost_probe(store, n);
    let (image, _) = compile_with(CHAIN, &[("प्राचलव्ययमापन", &src)], "प्राचलव्ययमापन");
    assert!(!image.is_empty(), "the cost probe built no image");
    let (h, steps) = run(&image);
    assert_eq!(status(&h), n, "the cost probe answers its iteration count");
    steps
}

/// TODAY'S FIGURE, measured with this same probe pair on the tree before this
/// change (`origin/main` at the merge base): a parameter store in a loop took
/// the growth arm's inline fast path — capacity test and length raise.
const TODAY_PER_STORE: u64 = 28;

/// THE RATCHET: this change measured 13 per store (21 per iteration against
/// 8 for the loop alone). A store may get cheaper, never dearer.
const RATCHET_PER_STORE: u64 = 13;

#[test]
fn a_parameter_store_costs_no_more_than_the_ratchet() {
    let (lo, hi) = (256, 512);
    let store = cost_steps(true, hi) - cost_steps(true, lo);
    let null = cost_steps(false, hi) - cost_steps(false, lo);
    let per_iteration = store / (hi - lo);
    let per_store = (store - null) / (hi - lo);
    let rem = (store - null) % (hi - lo);
    eprintln!(
        "W-359 cost: {per_iteration} instructions per iteration with the store, \
         {} without; {per_store} per parameter store (remainder {rem})",
        null / (hi - lo)
    );
    assert_eq!(rem, 0, "a store's cost is a whole number of instructions");
    assert!(
        per_store <= RATCHET_PER_STORE,
        "a bounded parameter store costs {per_store}, more than the ratchet's \
         {RATCHET_PER_STORE} (the growth arm's fast path, before W-359, was {TODAY_PER_STORE})"
    );
}
