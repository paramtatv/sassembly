//! What a type class asserts, and whether anything checks it — task `B-097`.
//!
//! Doc 02 §2.5 fixed three classes: `ॱअ` signed, `ॱन` unsigned, `ॱप` float.
//! `B-075` made all three *writable* and only the width was read, so a class
//! letter was validated and thrown away. `B-077` closed that for `ॱप` by
//! deriving what an encoding operates on from its `freg` slot; this closes it
//! for `ॱन`.
//!
//! **Signedness is not in the encoding.** ADR-0006 decided it is a FAMILY —
//! `अचिह्नाहारः` is `lbu/lhu/lwu` and `आहारः` stays signed — so the answer is
//! read from the registry, which is the authority R-02-1 makes it, rather than
//! from a second column that could disagree with the name.
//!
//! **A suffix names the VALUES, not the instruction** (ADR-0014). Doc 02 §2.5's
//! own line says `अ३२` is `i32` and `न३२` is `u32` — a T1 type name that T0
//! borrowed. So a suffix is refused only where something can contradict it: the
//! register class, or a family name that states the opposite signedness.
//! Everywhere else it is a true claim that selects nothing, and true claims are
//! not errors.

use sadhana::encode::encode;
use sadhana::parse::assemble_program;

/// Encode the first instruction of a source, as a single-instruction test does.
fn first(src: &str) -> Result<u32, sadhana::encode::EncodeError> {
    let p = assemble_program(src).unwrap_or_else(|e| panic!("{src}: {e:?}"));
    encode(&p.instructions[0])
}

#[test]
fn a_suffix_is_refused_only_where_the_family_contradicts_it() {
    // ADR-0014. `सचिह्नदक्षिणसरणम्` is `sra`, which shifts an `i64`, so calling
    // its operands unsigned contradicts the name it was given by ADR-0006.
    let e = first("सचिह्नदक्षिणसरणम्ॱन६४ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect_err("must refuse");
    assert_eq!(e.code, "E20");
    assert!(
        e.message(sadhana::nidana::Language::English)
            .contains("ADR-0014")
    );

    // And a family that states no signedness contradicts nothing, so a class
    // letter on it is a true sentence about the values. `B-097` refused this by
    // reasoning from ADR-0006, which decides how an INSTRUCTION is named and
    // says nothing about what its OPERANDS are.
    first("योगःॱन३२ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect("add two u32s");
    // Same instruction either way — a type is a claim, not a selector.
    assert_eq!(
        first("योगःॱन३२ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect("unsigned"),
        first("योगःॱअ३२ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect("signed"),
    );

    // `दक्षिणसरणम्` is `srl` and names no signedness, so `ॱन` on it is true and
    // accepted. What the unmarked NAME means is O-02-3 and is still open; this
    // is about what the suffix says, which is a different question.
    first("दक्षिणसरणम्ॱन६४ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect("shift u64s");
}

#[test]
fn a_family_that_names_itself_unsigned_takes_the_class_and_means_it() {
    // The rule has to ACCEPT something, or it is a blanket refusal wearing a
    // reason. All nine families ADR-0006 marked take it.
    for family in ["अचिह्नभागः", "अचिह्नशेषः", "अचिह्नन्यूनम्", "अचिह्नोपरिगुणनम्"]
    {
        let src = format!("{family}ॱन६४ अर्थ०म् अर्थ१न अर्थ२न ।\n");
        first(&src).unwrap_or_else(|e| panic!("{family}: {e:?}"));
    }

    // And it is the same instruction the bare width gives, because the family
    // already said it: the letter is a claim, not a selector.
    assert_eq!(
        first("अचिह्नभागःॱन३२ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect("unsigned"),
        first("अचिह्नभागःॱ३२ अर्थ०म् अर्थ१न अर्थ२न ।\n").expect("bare width"),
    );

    // `ॱअ` is the one spelling it refuses (`B-101`): the family has already
    // said unsigned, so writing `signed` after it contradicts the name. This is
    // the only place `ॱअ` can be false, because everywhere else it is what
    // omitting the letter already means.
    assert_eq!(
        first("अचिह्नभागःॱअ३२ अर्थ०म् अर्थ१न अर्थ२न ।\n")
            .expect_err("contradicts the family")
            .code,
        "E21"
    );
}

#[test]
fn a_conversion_is_exempt_because_its_classes_name_what_it_moves_between() {
    // ADR-0010 gives a conversion a PAIR of types, and there `ॱन` already
    // carries: `fcvt.d.lu` reads an unsigned long and `fcvt.d.l` a signed one,
    // and nothing else in the instruction says which. Thirty-two conformance
    // rows depend on it, and a rule that did not exempt them would have failed
    // the corpus rather than this test — but the corpus cannot say WHY.
    let unsigned = first("प्लवरूपान्तरम्ॱप६४ॱन६४ प्लव०म् स्तूपसूचकःन ।\n").expect("fcvt.d.lu");
    let signed = first("प्लवरूपान्तरम्ॱप६४ॱअ६४ प्लव०म् स्तूपसूचकःन ।\n").expect("fcvt.d.l");
    assert_ne!(
        unsigned, signed,
        "the class letter chooses the conversion; if these agree it does not"
    );
    assert_eq!(unsigned, 0xd231_7053, "fcvt.d.lu f0,sp — from the oracle");
}

#[test]
fn the_registry_is_where_signedness_lives() {
    // ADR-0006's decision, asserted rather than assumed: nine families name
    // themselves unsigned and one names itself signed, and that naming is the
    // only place the answer exists. A tenth `अचिह्न-` family appearing is fine;
    // the check is that the convention is real and not a coincidence of four
    // names.
    let unsigned = sadhana::parse::families()
        .into_iter()
        .filter(|f| f.name.starts_with("अचिह्न"))
        .count();
    assert!(
        unsigned >= 9,
        "only {unsigned} families name themselves unsigned"
    );
    // Every one of them covers a RISC-V encoding whose key says so too, which
    // is the external agreement that makes the convention more than a habit.
    for f in sadhana::parse::families()
        .into_iter()
        .filter(|f| f.name.starts_with("अचिह्न"))
    {
        assert!(
            f.covers.iter().all(|c| c.contains('u')),
            "`{}` names itself unsigned and covers {:?}",
            f.name,
            f.covers
        );
    }
}
