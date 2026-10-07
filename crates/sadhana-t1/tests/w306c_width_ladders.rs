//! `W-306c` — THE TWO NARROW-STORE WIDTH LADDERS, COMPARED BY TEXT.
//!
//! A १/२/४-octet indexed store is lowered by TWO ladders one screen apart in
//! `ir.t1`, and until this file nothing compared them:
//!
//!   * the CALLER's, at `:4856-4858` — `यदि वृद्धिविस्तार समम् N आदि
//!     सङ्कीर्णलेख्यम् भवति सत्यम् । इति` — which DECIDES that the narrow path is
//!     taken at all; and
//!   * the PRODUCER's, inside `सङ्कीर्णनिधानरचना` at `:1845-1847` — `यदि विस्तार
//!     समम् N आदि अवरावरणम् भवति <mask> । इति` — which picks the mask, and whose
//!     next line is `यदि अवरावरणम् समम् ० आदि प्रत्यागमनम् ० । इति`: **a width
//!     this ladder does not name emits NOTHING and answers `०`.**
//!
//! `W-306b` half (a) made that divergence LOUD at run time — the caller now
//! reads the answer and raises stub cause ४६ `assign_index_narrow_dropped`
//! instead of discarding it. But cause ४६ CANNOT FIRE ON THIS CORPUS and that
//! is not a defect: both ladders admit `{१,२,४}` today, so the guard is
//! unreachable by construction and the only witness it could ever produce is a
//! divergence that has already shipped. **This file moves the check to the
//! gate.** A fourth width added to one ladder and not the other is refused here
//! — before an image is assembled quietly missing a write.
//!
//! **NEITHER WIDTH SET IS SPELLED AS A LITERAL ANYWHERE IN THIS FILE, AND THAT
//! IS THE DESIGN AND NOT A FLOURISH.** A test that pinned `{१,२,४}` twice would
//! go green on two ladders that had BOTH drifted the same way, which is the
//! check-that-cannot-fail shape: it would assert the pin, not the agreement.
//! Both sides are derived from `ir.t1`'s own text and compared to EACH OTHER.
//! The cost of that choice is stated plainly: two ladders that drift together
//! are INVISIBLE to this file, and nothing here claims otherwise. What is
//! caught is the asymmetric edit, which is the one that drops a store.
//!
//! **THE ANCHOR IS THE ASSIGNMENT TARGET AND THE GUARD IS WHAT IS READ.** Each
//! arm is located by the name it WRITES — `अवरावरणम्` for the producer,
//! `सङ्कीर्णलेख्यम्` for the caller, each unique to its ladder in `ir.t1` — and
//! the guard variable is then read OFF the line rather than asserted. That
//! ordering matters: the guards are DIFFERENT variables in different routines
//! (`विस्तार` against `वृद्धिविस्तार`), so anchoring on the guard would have
//! made a renamed parameter read as an empty ladder, and an empty ladder
//! trivially agrees with anything.
//!
//! **SO THE EMPTINESS GUARD IS PART OF THE INSTRUMENT.** `set_equality` on two
//! empty sets is `true`, so a scraper that stopped matching would report
//! agreement in the exact voice it reports success. Every ladder is required to
//! have at least two arms with one consistent guard and no repeated width, and
//! `a_ladder_the_scraper_cannot_find_is_refused_rather_than_called_agreement`
//! proves that requirement fires: it runs the scraper over a text with the
//! anchors renamed away and asserts the read is REFUSED.
//!
//! **THE REPORT HAS THREE STATES AND NOT TWO,** because the two directions of
//! divergence are different defects. A width in `caller_only` is the
//! DROPPED-STORE direction — the caller sends it, the producer refuses it,
//! nothing is emitted. A width in `producer_only` is a mask no caller can
//! reach: dead arithmetic, not a miscompile. A boolean would have hidden which
//! one happened.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn ir_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("ir.t1")
}

fn ir_text() -> String {
    let p = ir_path();
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// THE PRODUCER'S ANCHOR — the mask accumulator `सङ्कीर्णनिधानरचना` assigns,
/// and the name its refusal arm then tests against `०`.
const PRODUCER_TARGET: &str = "अवरावरणम्";

/// THE CALLER'S ANCHOR — the `बूल` the lowering sets before it decides to take
/// the narrow path.
const CALLER_TARGET: &str = "सङ्कीर्णलेख्यम्";

/// `०..९`, decoded by codepoint. The source writes `१`, never `1`, so there is
/// no ASCII shortcut here — and word boundaries are no help either: a Devanagari
/// `\b` matches nothing, which is why every scraper in this file splits on
/// SPACES and compares whole tokens.
fn devanagari_number(s: &str) -> Option<u64> {
    let mut n = 0u64;
    let mut any = false;
    for c in s.chars() {
        let d = ('०'..='९').position(|x| x == c)? as u64;
        n = n.checked_mul(10)?.checked_add(d)?;
        any = true;
    }
    any.then_some(n)
}

/// One arm of a width ladder, carrying the line it was read from so a failure
/// names a SITE and not just a number.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Arm {
    line: usize,
    guard: String,
    width: u64,
}

/// Every `यदि <guard> समम् <width> आदि <target> भवति ...` line of `text`, in
/// text order.
///
/// TOKENS, NOT SUBSTRINGS. The shape is fixed at seven leading tokens and each
/// is compared whole, so neither a longer name that merely CONTAINS an anchor
/// nor a differently-indented copy changes the read. Two neighbours are
/// excluded by exactly this strictness and both are deliberate:
/// `ir.t1:1848`'s refusal arm writes `प्रत्यागमनम्` at the target position even
/// though it TESTS the producer's accumulator, and `:4859` tests the caller's
/// `बूल` against `सत्यम्` where a width would be — so neither is an arm.
fn ladder(text: &str, target: &str) -> Vec<Arm> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t: Vec<&str> = line.split_whitespace().collect();
        if t.len() < 7 {
            continue;
        }
        if t[0] != "यदि" || t[2] != "समम्" || t[4] != "आदि" || t[5] != target || t[6] != "भवति"
        {
            continue;
        }
        let Some(width) = devanagari_number(t[3]) else {
            continue;
        };
        out.push(Arm {
            line: i + 1,
            guard: t[1].to_string(),
            width,
        });
    }
    out
}

/// The widths of one ladder, REFUSING the reads that would make a comparison
/// vacuous or incoherent.
///
/// This is the half of the instrument that has no opinion about the other
/// ladder: a one-arm ladder, a ladder whose arms test two different guards, or
/// a ladder that names a width twice is wrong on its own terms, and reporting
/// such a read as a set would launder it into the agreement check.
fn widths(arms: &[Arm], who: &str) -> BTreeSet<u64> {
    assert!(
        arms.len() >= 2,
        "the {who} width ladder reads as {} arm(s) in {}; \
         a ladder the scraper cannot find agrees with everything, \
         so this is a REFUSAL and not an agreement — arms: {arms:?}",
        arms.len(),
        ir_path().display(),
    );
    let guard = &arms[0].guard;
    for a in arms {
        assert_eq!(
            &a.guard, guard,
            "the {who} width ladder tests `{}` at line {} and `{guard}` at line {}; \
             one ladder tests ONE guard, and a mixed read is two ladders spliced",
            a.guard, a.line, arms[0].line,
        );
    }
    let mut set = BTreeSet::new();
    for a in arms {
        assert!(
            set.insert(a.width),
            "the {who} width ladder names width {} twice, at line {}; \
             a repeated arm is the copy-paste that forgets to change the number",
            a.width,
            a.line,
        );
    }
    set
}

/// What the two ladders do NOT share, in both directions.
#[derive(Debug, Default, PartialEq, Eq)]
struct Divergence {
    /// Widths the PRODUCER masks and no caller can reach: dead arithmetic.
    producer_only: BTreeSet<u64>,
    /// Widths the CALLER admits and the producer refuses: the store VANISHES —
    /// no instruction, no stub, no shape, no diagnostic. This is the direction
    /// `W-306b` half (a) wired stub cause ४६ to, and the reason this file runs.
    caller_only: BTreeSet<u64>,
}

impl Divergence {
    fn agree(&self) -> bool {
        self.producer_only.is_empty() && self.caller_only.is_empty()
    }
}

fn compare(producer: &BTreeSet<u64>, caller: &BTreeSet<u64>) -> Divergence {
    Divergence {
        producer_only: producer.difference(caller).copied().collect(),
        caller_only: caller.difference(producer).copied().collect(),
    }
}

/// Both ladders, read off a text, with every per-ladder refusal applied.
fn both(text: &str) -> (BTreeSet<u64>, BTreeSet<u64>) {
    let p = widths(&ladder(text, PRODUCER_TARGET), "producer");
    let c = widths(&ladder(text, CALLER_TARGET), "caller");
    (p, c)
}

/// THE MEASUREMENT. The two ladders admit the SAME width set, derived on both
/// sides and compared to each other.
#[test]
fn the_two_narrow_store_width_ladders_admit_the_same_widths() {
    let text = ir_text();
    let (producer, caller) = both(&text);
    let d = compare(&producer, &caller);
    let path = ir_path().display().to_string();
    assert!(
        d.agree(),
        "the two narrow-store width ladders in {path} have DIVERGED.\n  \
         producer (`{PRODUCER_TARGET}`, ir.t1:1845): {producer:?}\n  \
         caller   (`{CALLER_TARGET}`, ir.t1:4856): {caller:?}\n  \
         widths the caller admits and the producer REFUSES (the store vanishes; \
         stub cause ४६): {:?}\n  \
         widths the producer masks and no caller reaches (dead arithmetic): {:?}\n  \
         add the width to BOTH ladders or to neither.",
        d.caller_only,
        d.producer_only,
    );
    println!("METRIC ir_narrow_store_admitted_widths {}", producer.len());
}

/// EACH LADDER IS A LADDER BEFORE IT IS A SET, and the two are DISTINCT reads.
/// `widths` carries the arity, guard and duplicate refusals; what is added here
/// is that the two ladders were not the same lines read twice — their guards
/// are different variables in different routines, and equal guards would mean
/// the scraper had collapsed onto one site and was comparing it with itself.
#[test]
fn each_width_ladder_is_read_as_a_ladder_and_the_two_reads_are_distinct() {
    let text = ir_text();
    let p = ladder(&text, PRODUCER_TARGET);
    let c = ladder(&text, CALLER_TARGET);
    let _ = widths(&p, "producer");
    let _ = widths(&c, "caller");
    assert_ne!(
        p[0].guard, c[0].guard,
        "both ladders test `{}`; the producer's guard is its own `विस्तार` \
         parameter and the caller's is the `वृद्धिविस्तार` it computed, so an \
         equal read means the scraper found ONE site twice and the comparison \
         is with itself",
        p[0].guard,
    );
    let overlap: BTreeSet<usize> = p
        .iter()
        .map(|a| a.line)
        .collect::<BTreeSet<_>>()
        .intersection(&c.iter().map(|a| a.line).collect())
        .copied()
        .collect();
    assert!(
        overlap.is_empty(),
        "the two ladder reads share source line(s) {overlap:?}",
    );
}

/// THE REFUSAL ARM IS WHY THE PRODUCER'S SET IS AUTHORITATIVE, AND IT IS BELOW
/// THE LADDER. `सङ्कीर्णनिधानरचना` returns `०` without emitting when the mask is
/// still `०`, so an unnamed width is REFUSED rather than guessed — and that
/// line must sit AFTER every arm, because above them it would refuse width १ on
/// its way past. The accumulator name is taken from the ladder's own read, so
/// renaming it moves both halves together instead of making this vacuous.
#[test]
fn the_producer_ladder_is_followed_by_its_refusal_arm() {
    let text = ir_text();
    let arms = ladder(&text, PRODUCER_TARGET);
    let _ = widths(&arms, "producer");
    let last_arm = arms
        .iter()
        .map(|a| a.line)
        .max()
        .expect("arms are non-empty");
    let refusals: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let t: Vec<&str> = line.split_whitespace().collect();
            t.len() >= 7
                && t[0] == "यदि"
                && t[1] == PRODUCER_TARGET
                && t[2] == "समम्"
                && devanagari_number(t[3]) == Some(0)
                && t[4] == "आदि"
                && t[5] == "प्रत्यागमनम्"
                && devanagari_number(t[6]) == Some(0)
        })
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(
        refusals.len(),
        1,
        "`यदि {PRODUCER_TARGET} समम् ० आदि प्रत्यागमनम् ० ।` reads {} time(s) in {}; \
         without it an unnamed width falls through to a guess and the producer's \
         admitted set stops being a set",
        refusals.len(),
        ir_path().display(),
    );
    assert!(
        refusals[0] > last_arm,
        "the refusal arm is at line {} and the last ladder arm at {last_arm}; \
         a refusal ABOVE the ladder refuses every width on its way past",
        refusals[0],
    );
}

/// THE CASE THAT MUST STILL BE REFUSED, IN THE DIRECTION THAT DROPS A STORE.
/// A fourth width admitted by the CALLER alone is exactly the edit `W-306b`
/// half (a) armed cause ४६ for, and the whole claim of this file is that the
/// gate catches it first. The arm is appended to the REAL text, so what is
/// exercised is the scraper on the shipped shape and not a hand-made fixture
/// that could agree with a scraper no longer matching anything.
#[test]
fn a_width_the_caller_alone_admits_is_refused() {
    let text = ir_text();
    let (producer, _) = both(&text);
    let widened = format!(
        "{text}\n                यदि वृद्धिविस्तार समम् ८ आदि {CALLER_TARGET} भवति सत्यम् । इति\n"
    );
    let (p2, c2) = both(&widened);
    assert_eq!(
        p2, producer,
        "the producer ladder is untouched by this edit"
    );
    let d = compare(&p2, &c2);
    assert!(!d.agree(), "a caller-only width read as agreement");
    assert_eq!(
        d.caller_only,
        BTreeSet::from([8]),
        "the divergence is reported in the DROPPED-STORE direction and names the width",
    );
    assert!(
        d.producer_only.is_empty(),
        "and not in the dead-arithmetic direction: {:?}",
        d.producer_only,
    );
}

/// THE OTHER DIRECTION, WHICH IS A DIFFERENT DEFECT AND MUST READ AS ONE. A
/// width the producer masks and the caller never sends is dead arithmetic, not
/// a miscompile; an instrument with two states would have called it the same
/// thing as a vanished store. The caller's first arm is REMOVED from the real
/// text, which is the same asymmetry seen from the other side.
#[test]
fn a_width_the_producer_alone_masks_reads_as_the_other_direction() {
    let text = ir_text();
    let arms = ladder(&text, CALLER_TARGET);
    let dropped = arms[0].line;
    let dropped_width = arms[0].width;
    let narrowed: String = text
        .lines()
        .enumerate()
        .filter(|(i, _)| i + 1 != dropped)
        .map(|(_, l)| format!("{l}\n"))
        .collect();
    let (p2, c2) = both(&narrowed);
    let d = compare(&p2, &c2);
    assert!(!d.agree(), "a producer-only width read as agreement");
    assert_eq!(
        d.producer_only,
        BTreeSet::from([dropped_width]),
        "the divergence names the width the caller stopped admitting",
    );
    assert!(
        d.caller_only.is_empty(),
        "and nothing is reported as dropped: {:?}",
        d.caller_only,
    );
}

/// THE SCRAPER'S OWN BREAKAGE MUST NOT READ AS AGREEMENT. Two empty sets are
/// equal, so the day an anchor is renamed this file would report success in the
/// voice it reports success — unless the emptiness guard fires. It fires: the
/// anchors are renamed away in a copy of the real text and the read PANICS.
#[test]
#[should_panic(expected = "a ladder the scraper cannot find agrees with everything")]
fn a_ladder_the_scraper_cannot_find_is_refused_rather_than_called_agreement() {
    let blinded = ir_text().replace(CALLER_TARGET, "सङ्कीर्णलेख्यम्२");
    let _ = both(&blinded);
}

/// **THE CALLER'S LADDER MUST BE POSITIVE-ONLY, AND THE AGREEMENT CHECK ABOVE
/// CANNOT SAY SO.** Read on its own terms, by text, with the scraper's own
/// breakage still a REFUSAL.
///
/// `f168e91a` CHANGED WHAT `०` MEANS AT THIS SITE. While the narrow arm called
/// `सङ्कीर्णनिधानरचना`, a `०` admitted by the caller was refused by the
/// producer's `यदि अवरावरणम् समम् ० आदि प्रत्यागमनम् ० ।` and the store was
/// DROPPED — bad, and in `caller_only`'s direction, which
/// [`a_width_the_caller_alone_admits_is_refused`] catches. The arm no longer
/// calls the producer: it appends one `स्थाननिधानाज्ञाभेद` carrying
/// `वृद्धिविस्तार` in `ध्रुवमूल्यम्`, and `ir.t1:4866-4872` states what `०`
/// does there — "the `.t1` emitter maps it to `८` and the Rust twin REFUSES
/// it. So these three lines are the only thing between a non-slice indexed
/// write and a twin divergence, and they are positive-only on purpose."
/// `वृद्धिविस्तार` is `०` for every indexed write whose target is not a slice
/// of sized integers (`:4660`), and all of them walk through these lines.
///
/// SO THE PROPERTY IS NOW ABSOLUTE AND NOT RELATIVE. Agreement with the
/// producer no longer implies anything about this site, because the producer is
/// not on this path. This file's header states its own blind spot — "two
/// ladders that drift together are INVISIBLE to this file" — and a `०` added
/// to BOTH ladders is exactly that drift. It used to be benign on this arm and
/// since `f168e91a` it is a twin divergence, so the blind spot acquired a
/// defect and this is the check that sees it.
fn zero_arms(text: &str) -> Vec<Arm> {
    let arms = ladder(text, CALLER_TARGET);
    // THE EMPTINESS GUARD, FOR THE SAME REASON EVERY OTHER READ IN THIS FILE
    // TAKES IT. "No zero arm" and "no arms at all" are one output otherwise,
    // and the second reads as success in the voice of the first.
    let _ = widths(&arms, "caller");
    arms.into_iter().filter(|a| a.width == 0).collect()
}

#[test]
fn the_caller_ladder_admits_no_width_zero() {
    let zeros = zero_arms(&ir_text());
    assert!(
        zeros.is_empty(),
        "the caller width ladder in {} admits width ० at {:?}; \
         `वृद्धिविस्तार` is ० for every indexed write whose target is not a \
         slice of sized integers, so this arm sends all of them down the \
         narrow path, where ० means `no width stated` — the `.t1` emitter \
         writes eight octets and the Rust twin REFUSES, which is a twin \
         divergence and not a dropped store",
        ir_path().display(),
        zeros,
    );
}

/// THE CASE THAT MUST STILL BE REFUSED — AND IT IS THE ONE THE AGREEMENT CHECK
/// CALLS AGREEMENT. `०` is appended to BOTH ladders of the REAL text, which is
/// the symmetric drift this file's header declares itself blind to. The two
/// instruments are then made to SAY MORE: `compare` is asserted to report
/// AGREE on that text, and `zero_arms` is asserted to refuse it. If the first
/// assertion ever fails, the agreement check grew teeth here and this test's
/// reason for existing should be re-read rather than the test deleted.
#[test]
fn a_zero_width_drifting_into_both_ladders_is_refused_where_agreement_is_blind() {
    let text = ir_text();
    assert!(
        zero_arms(&text).is_empty(),
        "the shipped text is the baseline for this test"
    );
    let drifted = format!(
        "{text}\n    यदि विस्तार समम् ० आदि {PRODUCER_TARGET} भवति २५५ । इति\
         \n                यदि वृद्धिविस्तार समम् ० आदि {CALLER_TARGET} भवति सत्यम् । इति\n"
    );
    let (p, c) = both(&drifted);
    assert!(
        p.contains(&0) && c.contains(&0),
        "the edit lands in both ladders: producer {p:?}, caller {c:?}"
    );
    assert!(
        compare(&p, &c).agree(),
        "THE PREMISE OF THIS TEST IS THAT THE AGREEMENT CHECK IS BLIND HERE, \
         and it just was not: producer {p:?}, caller {c:?}"
    );
    let zeros = zero_arms(&drifted);
    assert_eq!(
        zeros.len(),
        1,
        "the positive-only read refuses the drift the agreement check passed: {zeros:?}"
    );
    assert_eq!(zeros[0].width, 0, "and it names the width it refused");
}
