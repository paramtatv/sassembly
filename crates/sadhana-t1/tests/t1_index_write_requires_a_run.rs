//! **W-306's CONSTRUCTIVE FALSIFIER: can a well-typed program reach
//! `ir.t1:3332`?**
//!
//! `T1_FULL_CENSUS=1` reports `assign_index` (shape ३२) reading a PRESENT,
//! WELL-FORMED ZERO on every corpus module, and the census's own diagnostic
//! leaves two candidates: the arm is written and unreached, or its raise is
//! guarded by a condition the corpus never meets. Reading the source settles
//! which, and this file pins the reading so it cannot rot:
//!
//! ```text
//!   ir.t1:3332  रचितगणनम् ३२   under  यदि स्थानमात्रमिदम् समम् सत्यम्
//!   ir.t1:2092  अभिव्यञ्जकरचना CAPTURES स्थानमात्रम् and :2093 CLEARS it
//!               — a one-shot, so only the arm that sets it can reach :3332
//!   ir.t1:4733  the `अन्यथा` arm sets it, chosen by
//!               यदि वृद्धिविस्तार अधिकम् ०   (:4710)
//!   ir.t1:4627  वृद्धिविस्तार starts ०; :4661-4665 set ८/१/२/४ behind six
//!               guards, the decisive one being
//!               यदि वृद्धिप्रकारः ॱ भेद समम् अर्थॱखण्डार्थभेद
//!               — THE BASE'S TYPE MUST BE A RUN.
//! ```
//!
//! So the growth arm is taken whenever the base is a run, and `रचितगणनम् ३२`
//! needs a base that is NOT one.
//!
//! **I PREDICTED `artha.t1:1200-1213` MADE THAT IMPOSSIBLE, AND IT DOES NOT.**
//! The reasoning was: an index expression types to the element type only for a
//! run, every other path answers `प्रत्यागमनम् दोषार्थः`, that raises
//! `प्रकारदोषमस्ति`, and `मण्डलसङ्कलनम्` refuses before any IR. That chain was
//! recorded in `BACKLOG.tsv` W-306 and landed as `e959aab5`. **THE TEST BELOW
//! REFUTED IT ON ITS FIRST RUN.** An indexed write on a plain `अ६४` COMPILES,
//! and emits a store:
//!
//! ```text
//!   योगः स्थिर०म् शून्यःन ६५न ।       the value
//!   योगः स्थिर१म् शून्यःन ०न ।        the base WORD
//!   निधानम् स्थिर१य् ०न स्थिर०न ।     STORE at [base + ०]
//! ```
//!
//! So the word's VALUE was taken as an address. Whatever else that is, it is
//! not a refusal.
//!
//! **WHY THE ARGUMENT FAILED IS THE REUSABLE PART.** I read the index-EXPRESSION
//! rule at `artha.t1:1200` and assumed it governed the index-ASSIGNMENT TARGET.
//! A rule that types a READ says nothing about what the checker does with a
//! WRITE target, and the two are different code paths. Reading one and
//! concluding about the other is how a chain of six correct citations reaches a
//! false conclusion.
//!
//! ---
//!
//! **2026-09-28, THE SAME DAY: THE DEFECT IS FOUND AND THE REFUSAL CANNOT BE
//! WRITTEN YET. The hole is now MEASURED instead.**
//!
//! The finding was not that the checker lacked a rule — `अभिव्यञ्जकप्रकारः`
//! has had the right rule since W-202. It was that `artha.t1`'s
//! `वास्तुॱसमवाक्यभेद` arm computed its target's type into `लक्ष्यार्थः` AND
//! NEVER READ IT. **The value was measured and thrown away**; the arm returned
//! `शून्यार्थः` unconditionally.
//!
//! **BUT REFUSING ON THAT TYPE REFUSES THE CORPUS, AND IT TOOK TWO TRIES AND A
//! RED GATE TO LEARN IT.** First form: refuse any base that is not a run
//! (`असमम् खण्डार्थभेद`). That reads like the rule and is not — the kinds are
//! १ Int, २ Float, ३ Pointer, ४ Slice … १२ poison, so "not ४" swept in
//! Pointer, Struct, Optional and Void. Second form: refuse only an `Int` base,
//! which is exactly the hole the ruling names. **That refuses the corpus too**,
//! and the reason is a prior defect:
//!
//! ```text
//!   vastu.t1:  सार्वजनिक चरः अभिव्यञ्जककोश ॱॱ अङ्कः अन्तः अभिव्यञ्जक भवति ० ।
//! ```
//!
//! Every arena in this corpus is DECLARED a run and INITIALISED `०`, and a
//! global declared `०` is typed as a WORD whatever its declared type — locals
//! get a real empty run, globals do not. So the checker answers
//! `पूर्णाङ्कार्थभेद` for `parse.t1`'s `अभिव्यञ्जकयोजनम्` writing into the
//! expression arena, which is a CORRECT write into a run. The refusal poisoned
//! that routine and `parse.t1` stopped emitting: `t1_corpus_conditionals.rs`
//! reported 15 modules where its own commit says 16.
//!
//! **AND A 21/21 PER-SOURCE CENSUS SAID 0 STOPPED.** It ran `t1_boot`, whose
//! loader resolves the cross-module arena's type where `Front`'s `FRONT_END`
//! does not. A census answers about the driver that took it; the gate used the
//! driver the product uses, and only the gate saw this.
//!
//! **SO THE HOLE IS A NUMBER UNTIL ITS PRECONDITION IS FIXED.**
//! `असूचीलक्ष्यसंख्या` counts every indexed write whose base types as an
//! integer — today that is the security hole and the arena idiom together,
//! because at this level they are the same shape.
//! `the_word_index_counter_actually_fires` is what keeps that count a mechanism
//! rather than a declaration. The refusal goes in when a global carries its
//! declared type, and `an_indexed_write_on_a_word_is_counted_not_yet_refused`
//! is the test that must flip on that day.
//!
//! **W-306 IS UNCHANGED BY ALL OF THIS.** `assign_index` sits in
//! `DECLARED_AND_UNREACHED_SHAPES`, covered by `assign_index_grown`; nothing
//! here makes it satisfiable or unsatisfiable.
//!
//! **THE CONTROL STILL DISCRIMINATES**: the run case and the word case differ
//! in ONE token, and `a_plain_field_write_still_compiles` closes the other
//! side, so whatever this file asserts is about run-ness and about indexing
//! rather than about assignment or the surrounding shape.

use sadhana::t1::chain::{self, CHAIN};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

const FUEL: u64 = 80_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn compile(src: &str, module: &str) -> (Vec<u8>, Interpreter) {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets(module.as_bytes())],
            FUEL,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    (text, it)
}

/// One record with a RUN field and a WORD field, and one indexed write. The
/// `{body}` line is the only difference between the two cases below.
fn source(module: &str, body: &str) -> String {
    format!(
        "मण्डलम् {module} ॥
सार्वजनिक संरचना पेटिका आरभ्य
  खण्डम् ॱॱ अङ्कः अन्तः अ८ ऽ
  शब्दः ॱॱ अ६४
समाप्तम् ।
सार्वजनिक वृत्तिः मुख्यम् आदाय एका ॱॱ पेटिका ददाति अ६४ आदि
    चरः पिटकम् ॱॱ पेटिका भवति एका ।
{body}
    प्रत्यागमनम् ० ।
इति
"
    )
}

/// **THE CONTROL: an indexed write into a RUN compiles.** `खण्डम्` is
/// `अङ्कः अन्तः अ८`, so `वृद्धिविस्तार` resolves to १ and `ir.t1:4710` takes
/// the GROWTH arm — the one that now raises `रचितगणनम् ३५`.
#[test]
fn an_indexed_write_into_a_run_compiles() {
    let body = "    पिटकम् ॱ खण्डम् अङ्कः ० अन्तः भवति ६५ ।";
    let (text, it) = compile(&source("सूचीनियन्त्रणम्", body), "सूचीनियन्त्रणम्");
    assert!(
        !text.is_empty(),
        "the control must compile, or the refusal below is about the shape and \
         not about run-ness; refusal: {:?}",
        chain::refusal_site(&it)
    );
}

/// **THE CASE — AND IT IS NOW A REFUSAL. This test was written the other way
/// round this morning, and the flip is the fix, not a rot.**
///
/// It read `an_indexed_write_on_a_word_compiles_so_assign_index_is_reachable`
/// and asserted `!text.is_empty()`, because that was what the compiler did.
/// Its own margin named the consequence of ever changing it, and this is that
/// change, made deliberately: `artha.t1`'s assignment arm computed its
/// target's type and discarded it, so a write through a plain `अ६४` lowered to
/// `निधानम्` at `[the word's VALUE + ०]` — the declared integer taken as an
/// ADDRESS, unbounded and untyped. `artha.t1:3897-3923` now tests the target's
/// KIND and refuses when an indexed target's base is a real non-run type.
///
/// **THE NAME CHANGED WITH THE ASSERTION** — a test called `…_compiles` that
/// asserts a refusal is a label lying about its contract, and a label is not a
/// contract.
///
/// The counter is `असूचीलक्ष्यसंख्या` and the reason code is `दुष्टकारण` ४.
/// A base that types to `दोषार्थः` is COUNTED (`अप्रकार्यलक्ष्यसंख्या`) and
/// not refused: poison made no judgement, and refusing it would refuse every
/// indexed write through a shape this pass has no arm for.
#[test]
fn an_indexed_write_on_a_word_is_counted_not_yet_refused() {
    let body = "    पिटकम् ॱ शब्दः अङ्कः ० अन्तः भवति ६५ ।";
    let (text, it) = compile(&source("सूचीपरीक्षा", body), "सूचीपरीक्षा");
    assert!(
        !text.is_empty(),
        "this compiles TODAY and the margin above says why — if it has started \
         refusing, the global-typing precondition was fixed and this test owes \
         the flip to `…_is_refused`. Refusal: {:?}",
        chain::refusal_site(&it)
    );
    let s = String::from_utf8_lossy(&text);
    assert!(
        s.contains("निधानम्"),
        "and it must EMIT A STORE — a compile that emitted no store would mean \
         the write was folded away and this says nothing about the hole:\n{}",
        &s[..s.len().min(600)]
    );
}

/// **THE INSTRUMENT THAT REPLACED THE REFUSAL: `असूचीलक्ष्यसंख्या` COUNTS IT.**
///
/// A hole that cannot be closed yet must at least be *measured*, or the next
/// reader has only prose. The checker now counts every indexed write whose base
/// types as `पूर्णाङ्कार्थभेद`, which today is the security hole and the
/// corpus's arena idiom together — at this level they are the same shape.
///
/// **This assertion is what makes the count a mechanism rather than a
/// declaration.** If it ever reads 0 on a source that plainly holds one, the
/// counter has stopped counting and the follow-up row is measuring nothing.
#[test]
fn the_word_index_counter_actually_fires() {
    let body = "    पिटकम् ॱ शब्दः अङ्कः ० अन्तः भवति ६५ ।";
    let (_, it) = compile(&source("सूचीगणना", body), "सूचीगणना");
    let n = it
        .global("अर्थॱअसूचीलक्ष्यसंख्या")
        .or_else(|| it.global("असूचीलक्ष्यसंख्या"));
    let n = n.unwrap_or_else(|| {
        panic!("`असूचीलक्ष्यसंख्या` is not a readable global — the counter is not wired")
    });
    let got = match n {
        Value::Int(i) => i,
        other => panic!("`असूचीलक्ष्यसंख्या` is not an integer: {other:?}"),
    };
    assert!(
        *got >= 1,
        "the source holds exactly one indexed write on a word and the counter \
         reads {got} — the instrument behind the follow-up row does not fire"
    );
}

/// **THE DISCRIMINATOR: the refusal is about RUN-NESS, not about writing to a
/// field.** Without this, the test above passes if `artha.t1` started refusing
/// every field assignment — a much larger regression that would read as
/// success here.
///
/// `खण्डम्` and `शब्दः` differ in one token of the record declaration; the two
/// bodies differ in one token. The control compiles, this refuses.
#[test]
fn a_plain_field_write_still_compiles() {
    let body = "    पिटकम् ॱ शब्दः भवति ६५ ।";
    let (text, it) = compile(&source("क्षेत्रनियन्त्रणम्", body), "क्षेत्रनियन्त्रणम्");
    assert!(
        !text.is_empty(),
        "a NON-INDEXED write to the same `अ६४` field must still compile, or the \
         refusal above is about assignment and not about indexing; refusal: {:?}",
        chain::refusal_site(&it)
    );
}

/// **RETRACTION — THE TWIN QUESTION WAS NOT ASKABLE, AND THE TEST THAT ASKED
/// IT PASSED VACUOUSLY.**
///
/// This test read `both_engines_agree_about_indexing_a_word` and landed as
/// `ac0676a2`, concluding: both engines admit the write, so it is a property
/// of the LANGUAGE and not a port divergence. **That conclusion was worthless,
/// because the Rust half had nothing to check.**
///
/// ```text
///   typecheck.rs      Expression::Index(_) => { /* Needs to know base type.
///                     Stub. */ Ok(Ty::Error) }
///   ast.rs:67-73      enum Statement { Expression(..), Block(..) }
///                     — "Variable, return, if, etc. deferred for now"
///   parse.rs          emits only those two variants, ever
///   chain.rs:383      Front::typecheck calls `अर्थॱकार्यक्रमप्रकारपरीक्षा`
///   mod.rs:12         `TypeChecker` survives only in a `dead_code` note
/// ```
///
/// So `rust_verdict.is_ok()` was `true` for the reason every input is `true`:
/// there is no assignment statement in the Rust AST for a typechecker to have
/// an opinion about. An agreement between a checker and a stub is an agreement
/// about the stub. This is the same error as `e959aab5` one level up — a
/// verified fact answering a question nobody asked.
///
/// **W-304's twin gate was right to exist; the subject here just lacks the
/// property.** So the check becomes the narrower TRUE one, and it is a
/// compile-time pin rather than an assertion: the match below is exhaustive,
/// so adding any variant to `Statement` STOPS THIS FILE COMPILING and hands
/// whoever added it this margin. At that moment the twin question becomes
/// askable for the first time and is owed a real test.
#[test]
fn the_rust_half_cannot_express_an_assignment_so_no_twin_is_owed() {
    use sadhana::t1::ast::Statement;

    fn variants(s: &Statement) -> &'static str {
        match s {
            Statement::Expression(_) => "expression",
            Statement::Block(_) => "block",
            // NO WILDCARD ARM, DELIBERATELY. A `_` here would silently absorb
            // an assignment variant and restore the vacuous pass this test
            // exists to retract.
        }
    }

    let block = Statement::Block(vec![]);
    assert_eq!(variants(&block), "block");
}

// ══════════════ `V-006` — THE SECOND COUNTER HAD NO READER ══════════════

/// ॥ `अप्रकार्यलक्ष्यसंख्या` IS A READABLE GLOBAL AND IT FIRES ON A FLOAT BASE ॥
///
/// **THE SIBLING COUNTER HAS HAD A WIRING GUARD SINCE IT WAS FILED; THIS ONE
/// NEVER DID.** `the_word_index_counter_actually_fires` above reads
/// `असूचीलक्ष्यसंख्या` and PANICS if the global is unreadable, precisely so a
/// follow-up row cannot be left measuring nothing. `अप्रकार्यलक्ष्यसंख्या` was
/// declared at `artha.त१:3365`, incremented at two sites, and read NOWHERE —
/// it appeared only in a doc comment at `:182` of this very file, naming a
/// counter no test asked for.
///
/// **AND THE GAP WAS MINE TO CLOSE BECAUSE I WIDENED IT.** `V-006` added a
/// third arm to that counter — `प्लवार्थभेद`, so an indexed write through a
/// float base is censused like one through `दोषार्थभेद` — and shipped it with
/// no reader. A census entry that cannot be observed is indistinguishable from
/// one that counts nothing, which is the whole failure
/// `the_word_index_counter_actually_fires` exists to prevent one counter away.
///
/// A FLOAT BASE IS THE SUBJECT, not an error type, because the float arm is the
/// new one. `ध्वनिः ॱॱ प६४` is a legal field — ADR-0042 froze
/// `float_type = "प" , ( "३२" | "६४" )` — and indexing through it is nonsense a
/// run-typed base would never produce, so reaching the arm proves the
/// classification rather than merely the increment.
///
/// COUNTED AND NOT REFUSED, which this test asserts by COMPILING rather than by
/// expecting a diagnostic: nothing at that site can refuse until a global
/// carries its declared type, and the existing margin says so.
#[test]
fn the_unworkable_target_counter_is_readable_and_fires_on_a_float_base() {
    let src = "मण्डलम् प्लवसूची ॥
सार्वजनिक संरचना पेटिका आरभ्य
  ध्वनिः ॱॱ प६४
समाप्तम् ।
सार्वजनिक वृत्तिः मुख्यम् आदाय एका ॱॱ पेटिका ददाति अ६४ आदि
    चरः पिटकम् ॱॱ पेटिका भवति एका ।
    पिटकम् ॱ ध्वनिः अङ्कः ० अन्तः भवति ६५ ।
    प्रत्यागमनम् ० ।
इति
";
    let (_, it) = compile(src, "प्लवसूची");
    let n = it
        .global("अर्थॱअप्रकार्यलक्ष्यसंख्या")
        .or_else(|| it.global("अप्रकार्यलक्ष्यसंख्या"));
    let n = n.unwrap_or_else(|| {
        panic!(
            "`अप्रकार्यलक्ष्यसंख्या` is not a readable global — the counter is \
             not wired, so `V-006`'s float arm censuses nothing"
        )
    });
    let got = match n {
        Value::Int(i) => *i,
        other => panic!("`अप्रकार्यलक्ष्यसंख्या` is not an integer: {other:?}"),
    };
    println!("METRIC artha_unworkable_index_targets {got}");
    assert!(
        got >= 1,
        "the source holds exactly one indexed write through a `प६४` field and \
         the counter reads {got} — either the float arm added by `V-006` is not \
         reached, or the base does not type to `प्लवार्थभेद` at that site. Read \
         the arm at `artha.त१`'s `अप्रकार्यलक्ष्यसंख्या` increments before \
         changing this bound"
    );
}
