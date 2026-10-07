//! `W-306b` — THE NARROW INDEXED STORE'S WIDTH LADDER, CALLED DIRECTLY, AND THE
//! WIDTH THAT MUST STILL BE REFUSED.
//!
//! `मध्यरूपॱसङ्कीर्णनिधानरचना` (`ir.t1:1823`) is the masked read-modify-write a
//! १/२/४-octet indexed store lowers to. Its first four lines are a three-way
//! ladder over `विस्तार` and its fifth is `यदि अवरावरणम् समम् ० आदि प्रत्यागमनम्
//! ० । इति` — **a width this ladder does not name emits NOTHING and answers
//! `०`.** Until this file there was no test on either side of that line: the
//! routine was reached only through the corpus, where the caller's own ladder
//! admits exactly `{१,२,४}` and so the refusal arm was unreachable by
//! construction.
//!
//! **THE REFUSAL ARM IS THE REASON THIS TEST EXISTS, AND THE HAZARD IS A
//! CONVENTION AND NOT A GUARD.** `ir.t1:4811`'s margin states it: the caller at
//! `:4818` runs the SAME three-way ladder before calling, "so a width admitted
//! here and refused there would DROP the store rather than widen it." The two
//! ladders agree today — that is why this cannot fire on the corpus — but they
//! agree because two people wrote them the same way one screen apart, and
//! nothing compares them. A fourth width added to the caller and not to the
//! routine produces a store that VANISHES: no instruction, no stub, no shape,
//! no diagnostic, and an assembled image that is quietly wrong. So the refusal
//! is asserted on its own terms here — answers `०`, appends nothing, raises no
//! shape — and the three admitted widths are asserted beside it, because a
//! routine that refused EVERY width would pass a refusal-only test.
//!
//! **`रचितगणनम् ३६` IS WHAT MAKES THE DISTINCTION READABLE AT ALL.** Shape ३५
//! `assign_index_grown` is raised at `ir.t1:4780`, BEFORE the caller's width
//! ladder runs, on the address the growth branch formed — so it counts an octet
//! store and a word store under one number. That blindness is not hypothetical:
//! the one-octet case landed 2026-09-13 and `अ१६`/`अ३२` stayed on the word store
//! for another two weeks, clobbering six and three neighbours, because the `अ३२`
//! fixture wrote its two elements in ASCENDING order and the second write
//! repaired what the first ate (`ir.t1:4805`). ३६ is raised INSIDE this routine,
//! past the refusal arm, so it counts LOWERINGS and not CALLS.
//!
//! WHY THE SHAPE IS READ OFF THE ARENA AND NOT OFF A RETURN VALUE: nothing in
//! the chain reports a shape. `रचितगणनम्` writes `रचितगणनाकोश अङ्कः रूपम् अन्तः`
//! and that arena is the only witness, which is exactly why `W-283` found the
//! census able to sum to zero and call it progress.
//!
//! THE FIXTURE IS FOUR CALLS. `मध्यरूपॱआरम्भः` FIRST AND IT IS NOT OPTIONAL: its
//! reset loop walking `१ ..= रचितशेषसीमा` is the ONLY thing that sizes
//! `रचितगणनाकोश`, so on a freshly loaded interpreter the arena holds one slot and
//! a write to index ३६ faults with `entry 36 is outside an arena of 1` rather
//! than reporting a missing shape. A fresh interpreter per case, so no case can
//! read another's counters.

use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// THIS TEST'S OWN LOADER. `ir.t1` is the whole subject and `lex.t1` comes with
/// it because it carries the spec table every load reaches; nothing here parses,
/// resolves or emits, so no other source is owed.
fn loaded() -> Interpreter {
    let names = ["lex.t1", "ir.t1"];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

fn call(it: &mut Interpreter, name: &str, args: Vec<Value>) -> Value {
    it.call(name, args, 5_000_000)
        .unwrap_or_else(|e| panic!("`{name}` runs: {e}"))
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global of the loaded corpus"))
}

/// One entry of a `अङ्कः अन्तः न६४` census arena, by its `ir.t1` index.
///
/// **THE ARENA IS 1-BASED AND SLOT `०` IS THE UNUSED HEAD** — `आरम्भः` clears
/// `१ ..= रचितशेषसीमा` — so the shape number IS the Rust index and no `−1` is
/// owed. A read that is SHORT is a hard failure and not a zero: an arena the
/// reset never grew would otherwise report every shape absent, which is the
/// check-that-cannot-fail shape `W-283` split `StubArenaState` to prevent.
fn census(it: &Interpreter, arena: &str, index: usize) -> i128 {
    match it.global(arena) {
        Some(Value::Arena(a)) => {
            let rows = a.borrow();
            assert!(
                index < rows.len(),
                "`{arena}` holds {} slot(s) and {index} is outside it — `मध्यरूपॱआरम्भः` \
                 did not size it, so this is NOT a shape reading zero",
                rows.len()
            );
            rows[index]
                .as_int()
                .unwrap_or_else(|| panic!("`{arena}` slot {index} is numeric"))
        }
        other => panic!("`{arena}` is a census arena, not {other:?}"),
    }
}

/// `रचितशेषसीमा`, read from the corpus. A test that spelled `३६` would go green
/// against a bound left behind by the next shape.
fn shape_bound() -> i128 {
    global_int(&loaded(), "रचितशेषसीमा")
}

/// What one direct call leaves behind.
struct Lowering {
    /// The routine's answer: `स्थाननिधानरचना`'s instruction index, or `०`.
    answer: i128,
    /// Instructions appended by the call — `आज्ञासूचकाङ्क` after minus before.
    appended: i128,
    /// `रचितगणनाकोश अङ्कः ३६ अन्तः` after minus before.
    shape: i128,
}

/// Call `सङ्कीर्णनिधानरचना` once at `width`, on a fresh initialised chain.
///
/// The two `मूल्याङ्क` arguments are bare ids and not lowered expressions: this
/// routine reads only their `क्रमाङ्क` to build operands, so a real base and a
/// real value would add instructions this measurement would then have to
/// subtract back out.
fn lower(width: i128) -> Lowering {
    let mut it = loaded();

    // NOT OPTIONAL — the reset loop is the only thing that sizes
    // `रचितगणनाकोश`, and `census` above refuses a short read rather than
    // reporting zero.
    call(&mut it, "मध्यरूपॱआरम्भः", vec![]);
    assert_eq!(
        global_int(&it, "आज्ञासूचकाङ्क"),
        0,
        "a fresh chain has appended no instruction"
    );

    let before_instructions = global_int(&it, "आज्ञासूचकाङ्क");
    let before_shape = census(&it, "रचितगणनाकोश", 36);

    let address = call(&mut it, "मध्यरूपॱमूल्याङ्कनम्", vec![Value::Int(1)]);
    let value = call(&mut it, "मध्यरूपॱमूल्याङ्कनम्", vec![Value::Int(2)]);
    let answer = call(
        &mut it,
        "मध्यरूपॱसङ्कीर्णनिधानरचना",
        vec![address, value, Value::Int(width)],
    );

    Lowering {
        answer: answer
            .as_int()
            .expect("सङ्कीर्णनिधानरचना answers a न६४ instruction index"),
        appended: global_int(&it, "आज्ञासूचकाङ्क") - before_instructions,
        shape: census(&it, "रचितगणनाकोश", 36) - before_shape,
    }
}

/// **THE THREE ADMITTED WIDTHS LOWER, AND EACH RAISES SHAPE ३६ EXACTLY ONCE.**
///
/// The instruction count is NOT pinned to a literal — the owner's ruling of
/// 2026-09-13 took count pins off the landing path — but it IS asserted EQUAL
/// across the three widths, which is the real claim: the ladder differs only in
/// the mask constant it loads, so `अ८`, `अ१६` and `अ३२` must lower to the same
/// shape of instruction sequence. A width that fell through to a cheaper or
/// longer path would be caught by the disagreement and not by a number.
///
/// THE ANSWER IS THE LAST INSTRUCTION APPENDED, and that is a structural claim
/// rather than a count: `आज्ञायोजनम्` advances then writes on a 1-based arena, so
/// a store that is the final append answers exactly the new `आज्ञासूचकाङ्क`. An
/// answer short of it would mean the store is not last — an instruction emitted
/// AFTER the write it depends on.
#[test]
fn each_narrow_width_lowers_a_masked_store_and_raises_shape_thirty_six_once() {
    assert_eq!(
        shape_bound(),
        36,
        "`रचितशेषसीमा` bounds shape numbers INCLUSIVELY and ३६ is the highest; a \
         lower bound means this shape was added without its arena"
    );

    let mut widths = Vec::new();
    for width in [1, 2, 4] {
        let l = lower(width);
        assert!(
            l.answer > 0,
            "a {width}-octet indexed store LOWERS — `०` is the refusal and would be \
             a silently dropped store"
        );
        assert_eq!(
            l.shape, 1,
            "exactly one `रचितगणनम् ३६` per lowering ({width} octets): a shape that \
             counted calls instead of lowerings would also rise on the refused widths"
        );
        assert_eq!(
            l.answer, l.appended,
            "the masked store is the LAST instruction of the sequence ({width} octets)"
        );
        widths.push((width, l.appended));
    }

    let (_, first) = widths[0];
    assert!(
        first > 1,
        "a masked read-modify-write is a SEQUENCE and not one instruction, or this \
         routine has stopped doing the thing it exists for ({first} appended)"
    );
    for (width, appended) in &widths {
        assert_eq!(
            *appended, first,
            "{width} octets lowers to the same sequence as १ — the ladder differs only \
             in its mask constant, so a different length means a different path"
        );
    }
    eprintln!("METRIC ir_narrow_store_instructions {first}");
}

/// **THE CASE THAT MUST STILL BE REFUSED, AND IT IS REFUSED SILENTLY BY DESIGN
/// AT THIS LAYER.** `३` is a width no `.t1` type has and the one the caller's
/// ladder would have to grow to admit; `५`, `६` and `७` are the same shape of
/// mistake. **`८` IS HERE ON PURPOSE AND IS NOT A DEFECT** — `ir.t1:1806` says
/// so: a whole word needs no read-modify-write and the caller sends it to
/// `स्थाननिधानरचना` unchanged, and `(१ << ६४) − १` does not fit the `न६४` this
/// ladder holds. `०` cannot reach the routine from the corpus — the caller's
/// block sits inside `वृद्धिविस्तार अधिकम् ०` — and is asserted anyway rather
/// than resting on that.
///
/// ALL THREE FIGURES ARE ASSERTED AND NOT JUST THE ANSWER. An arm that answered
/// `०` after appending its constants would leave dead instructions in the
/// stream; one that answered `०` after raising ३६ would make the census count
/// refusals as lowerings. Both read identical from the return value alone.
#[test]
fn a_width_the_ladder_does_not_name_answers_zero_and_appends_nothing() {
    for width in [0, 3, 5, 6, 7, 8] {
        let l = lower(width);
        assert_eq!(
            l.answer, 0,
            "width {width} is not in the three-way ladder and must answer `०`"
        );
        assert_eq!(
            l.appended, 0,
            "and must append NOTHING — a refusal that emitted its constants first \
             leaves dead instructions behind it (width {width})"
        );
        assert_eq!(
            l.shape, 0,
            "and must raise NO shape — `रचितगणनम् ३६` sits PAST the refusal arm, so a \
             rise here would make the census a count of calls (width {width})"
        );
    }
}

/// One raise of `अपूर्णवाक्यम् हेतुः`, and what it left behind.
#[derive(Debug)]
struct Raise {
    /// `अपूर्णगणनाकोश अङ्कः हेतुः अन्तः` after minus before.
    slot: i128,
    /// `अपूर्णयोगः` after minus before — the SUM over `१ ..< अपूर्णहेतुसीमा`.
    sum: i128,
}

/// Raise `हेतुः` once on a fresh initialised chain. `Err` is the arena refusing
/// the write, which is the whole claim of the second test below.
fn raise(cause: i128) -> Result<Raise, String> {
    let mut it = loaded();

    // NOT OPTIONAL, AND IT IS THE ONLY THING THAT SIZES `अपूर्णगणनाकोश`:
    // `आरम्भः` clears `१ ..< अपूर्णहेतुसीमा` (`ir.t1:1125`), so the arena's
    // length IS the bound. On a freshly loaded interpreter it holds one slot.
    call(&mut it, "मध्यरूपॱआरम्भः", vec![]);

    let before_sum = call(&mut it, "मध्यरूपॱअपूर्णयोगः", vec![])
        .as_int()
        .expect("`अपूर्णयोगः` answers a न६४");
    let before_slot = census(&it, "अपूर्णगणनाकोश", usize::try_from(cause).unwrap());

    it.call("मध्यरूपॱअपूर्णवाक्यम्", vec![Value::Int(cause)], 5_000_000)
        .map_err(|e| format!("{e}"))?;

    Ok(Raise {
        slot: census(&it, "अपूर्णगणनाकोश", usize::try_from(cause).unwrap()) - before_slot,
        sum: call(&mut it, "मध्यरूपॱअपूर्णयोगः", vec![])
            .as_int()
            .expect("`अपूर्णयोगः` answers a न६४")
            - before_sum,
    })
}

/// **CAUSE ४६ IS INSIDE THE ARENA AND THE SUM COUNTS IT.** `W-306b` half (a)
/// raises `अपूर्णवाक्यम् ४६` at `ir.t1:4885` when `सङ्कीर्णनिधानरचना` answers `०`
/// for a width the caller's ladder admitted — the silent drop, which until this
/// cycle discarded that answer under a binding spelled `अवगणन`.
///
/// **THE CAUSE READS ZERO ON THIS CORPUS AND THAT IS CORRECT**, so this test
/// does NOT walk the corpus looking for it: the two width ladders admit the same
/// `{१,२,४}` today and the guard is unreachable by construction. What CAN go
/// wrong — and what `W-283` caught reporting progress while summing to zero — is
/// a counter whose READER is dead. A bound left at ४६ sizes the arena to slots
/// `०..=४५`, so the raise would fault rather than count, and `अपूर्णयोगः`'s loop
/// would never reach ४६ even if it did. This asserts the slot AND the sum,
/// because either alone reads identical to a cause nobody can observe.
#[test]
fn stub_cause_forty_six_lands_in_the_arena_and_the_sum_reaches_it() {
    assert_eq!(
        global_int(&loaded(), "अपूर्णहेतुसीमा"),
        47,
        "`अपूर्णहेतुसीमा` bounds stub causes EXCLUSIVELY and ४६ is the highest live \
         cause, so the bound is ४७; a bound of ४६ means cause ४६ was added without \
         its arena slot"
    );

    let r = raise(46).expect("`अपूर्णवाक्यम् ४६` writes inside the arena `आरम्भः` sized");
    assert_eq!(
        r.slot, 1,
        "one raise writes exactly one entry into `अपूर्णगणनाकोश अङ्कः ४६ अन्तः`"
    );
    assert_eq!(
        r.sum, 1,
        "and `अपूर्णयोगः` REACHES it — a sum that stopped short of ४६ would report \
         the corpus's stub total unchanged while the cause rose, which is a counter \
         with a dead reader and not a cause reading zero"
    );
}

/// The number of slots an `अङ्कः अन्तः` census arena holds, which for a census
/// IS its bound: `आरम्भः` clears `१ ..< सीमा` and that write is the only thing
/// that grows it. Read raw, with no range check, because the claim under test is
/// the LENGTH itself and `census` above refuses a short read by design.
fn arena_len(it: &Interpreter, arena: &str) -> usize {
    match it.global(arena) {
        Some(Value::Arena(a)) => a.borrow().len(),
        other => panic!("`{arena}` is a census arena, not {other:?}"),
    }
}

/// **THE VALUE THAT MUST STILL BE REFUSED: ४७.** The bound is declared
/// EXCLUSIVE (`t1_transcriptions.rs`'s `BOUNDS` pins that convention), so ४७ is
/// the first value that is NOT a cause and the arena `आरम्भः` sized must not
/// hold it. Without this arm the test above passes against a bound moved to ५०,
/// or to ४७ with a reset loop quietly rewritten to `न्यूनम् आरभ्य सीमा योगः १
/// समाप्तम्` — either of which makes the convention column a lie while every
/// positive assertion stays green.
///
/// TWO FACTS AND NOT ONE, because the length alone does not prove the write is
/// refused and the error alone does not prove WHERE the arena ends:
///
///  1. the arena's length EQUALS the bound read from the corpus, which on a
///     1-based arena with an unused slot `०` means entries `१..=४६` — the
///     EXCLUSIVE convention, derived rather than spelled;
///  2. `अपूर्णवाक्यम् ४७` is REFUSED by the interpreter. The refusal is the
///     arena's and not a guard's — `अपूर्णवाक्यम्` does not range check, so the
///     write lands outside and the interpreter says so.
#[test]
fn stub_cause_forty_seven_is_outside_the_arena_the_reset_sized() {
    let mut it = loaded();
    call(&mut it, "मध्यरूपॱआरम्भः", vec![]);

    let bound = usize::try_from(global_int(&it, "अपूर्णहेतुसीमा")).expect("a positive bound");
    assert_eq!(
        arena_len(&it, "अपूर्णगणनाकोश"),
        bound,
        "`आरम्भः` clears `१ ..< अपूर्णहेतुसीमा`, so the arena holds slot `०` unused \
         plus causes `१..={}` and NOTHING for the bound itself — a length of {} \
         would mean the reset's convention changed under the pin",
        bound - 1,
        bound + 1
    );

    let e = it
        .call(
            "मध्यरूपॱअपूर्णवाक्यम्",
            vec![Value::Int(i128::try_from(bound).unwrap())],
            5_000_000,
        )
        .map(|v| format!("{v:?}"))
        .expect_err(
            "the EXCLUSIVE bound is the first value that is not a cause, so writing \
             it must land outside the arena; if it succeeded, either the bound moved \
             without `STUB_CAUSES` or the reset loop's convention changed",
        );
    let text = format!("{e}");
    assert!(
        text.contains(&bound.to_string()) && text.contains("arena"),
        "the refusal must be the ARENA rejecting an out-of-range entry, not some \
         other failure that happens to error: {text}"
    );
}
