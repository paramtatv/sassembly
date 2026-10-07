//! `F-004f` — doc 07 §4.2's `दृश्यम्`, read from a file by `sadhana`.
//!
//! The row's acceptance: *"a `.sas` file expressing doc 07 §4.2's `दृश्यम्`
//! ASSEMBLES under `sadhana` and produces a `Rupa` EQUAL to the one `F-004a`'s
//! test builds in Rust."* Two of those three clauses are met here and the third
//! is met in a different sense than the words allow. Both facts are asserted
//! rather than described, so a later reader gets them from the suite and not
//! from a report.
//!
//! # "assembles under `sadhana`" cannot mean what it says, and a guard says so
//!
//! `sadhana` the binary has exactly one driver: `sadhana <source.sas>… <out.elf>`,
//! the T0 assembler, which writes a RISC-V ELF. A T2 view is not machine code,
//! so there is no reading on which it assembles — and the tree already holds
//! that boundary in two places:
//!
//! - `crates/sadhana/tests/parse_shape.rs:83` walks `spec/**/*.sas` and asserts
//!   every one of them parses as T0 against `spec/parse-shape-t0.tsv`. Putting a
//!   T2 file there would break that gate, which is the failure mode this row was
//!   opened to avoid, arrived at from the other side.
//! - `crates/sadhana/tests/t0_cannot_read_t1.rs` walks every `.सस` in the tree
//!   and asserts T0 **refuses** each. The fixture read below is a `.सस`, so it
//!   is already under that guard: its non-assembly is a checked fact.
//!
//! So the honest reading of the acceptance is the one the row's own diagnosis
//! points at — *"what no file under `crates/` reads is the UI sub-language
//! itself"*. The claim this file makes is that a file is READ, by
//! [`sadhana::t1::drishya`], into a view. `it_is_refused_by_t0_and_that_is_the_guarded_answer`
//! records the other half so the gap is visible rather than glossed.
//!
//! # "EQUAL to the one `F-004a`'s test builds" is equality by observation
//!
//! `F-004a`'s `Rupa` lives in `crates/renderer/src/vastu/rupa.rs`, and **there
//! is no dependency edge in either direction**: `cargo tree -e normal` gives
//! `sadhana -> sanskrit-text` and nothing else, `renderer` has no runtime
//! dependencies and no runtime dependents at all, and `gavaksha` does not link
//! `renderer` either. `sadhana` cannot name renderer's type, so `==` between
//! the two values is not a comparison any code in this tree can currently
//! write. What is asserted instead is every observable `F-004a`'s own test
//! asserts of the tree it builds — `node_count`, `depth`, `text`, `events` and
//! the no-cascade property — on the tree parsed from the file. That is the
//! strongest claim available without an edge, and closing the gap is a
//! successor row, not a detail.

use sadhana::lex::lex_t1 as lex;
use sadhana::t1::drishya::{Ghatana, Rupa, Samrekha, Sthiti, Vinyasa, render};

/// Doc 07 §4.2's counter module, whole, in ADR-0003's ratified spelling.
const GANAKA: &str = include_str!("गणकः.सस");

fn view_at(n: i32) -> Rupa {
    let tokens = lex(GANAKA).expect("the fixture lexes");
    render(&tokens, &Sthiti::new().with("सङ्ख्या", n)).expect("the fixture's दृश्यम् reads")
}

/// The row's acceptance, as an equality. This is the tree `F-004a`'s `drishya`
/// fixture builds in Rust, transcribed into this crate's constructors — the
/// same forms, the same order, the same arrangement, the same strings.
#[test]
fn the_file_produces_the_tree_f_004a_builds() {
    assert_eq!(
        view_at(0),
        Rupa::Stambha {
            vinyasa: Vinyasa::new(12, Samrekha::Madhyama).expect("12 is not negative"),
            santati: vec![
                Rupa::Patha("गणना ०".into()),
                // The sketch gives this row no `विन्यासः` at all, so it takes
                // the default one — as `F-004a`'s fixture notes in the same
                // place.
                Rupa::Pankti {
                    vinyasa: Vinyasa::default(),
                    santati: vec![
                        // `F-004f5`: each event is a variant of the fixture's
                        // own `प्रकारः घटना भवति गणना`, at its place in it.
                        Rupa::Kunjika {
                            patha: "वर्धय".into(),
                            ghatana: Ghatana {
                                nama: "वर्धनम्".into(),
                                krama: 0,
                            },
                        },
                        Rupa::Kunjika {
                            patha: "ह्रासय".into(),
                            ghatana: Ghatana {
                                nama: "ह्रासः".into(),
                                krama: 1,
                            },
                        },
                    ],
                },
            ],
        }
    );
}

/// Every assertion `F-004a`'s `the_counter_sketch_is_expressible_in_this_type`
/// makes, made here about a tree that came out of a FILE rather than out of a
/// Rust function.
#[test]
fn it_answers_everything_f_004a_asks_of_its_own_tree() {
    let view = view_at(0);
    assert_eq!(view.node_count(), 5, "column, text, row, two buttons");
    assert_eq!(view.depth(), 3, "column > row > button");
    assert_eq!(view.text(), ["गणना ०", "वर्धय", "ह्रासय"]);
    assert_eq!(view.events(), ["वर्धनम्", "ह्रासः"]);
}

/// D-07-B's *"no CSS cascade"*, on a parsed tree. The column arranges its
/// children with a gap of 12 and centring; the row inside it arranges its own
/// with the default 0 and start. If anything ever inherits, this says so.
#[test]
fn an_arrangement_does_not_reach_the_children_it_arranges() {
    let view = view_at(0);
    let outer = view.arrangement().expect("a column arranges its children");
    assert_eq!(outer.antara(), 12);
    assert_eq!(outer.samrekha(), Samrekha::Madhyama);

    let inner = view.children()[1]
        .arrangement()
        .expect("a row arranges its children");
    assert_eq!(inner.antara(), 0, "the row did not inherit the column's 12");
    assert_eq!(
        inner.samrekha(),
        Samrekha::Adi,
        "nor its centring: there is no inherit step"
    );
}

/// D-07-B's *"a pure `स्थिति → दृश्यम्` render function"*. The file is fixed;
/// the state is not. Equal state gives an equal view — which is the
/// reconciler's precondition, and is `F-004a`'s
/// `equal_state_yields_an_equal_view_because_a_view_has_no_identity` — and a
/// different state gives a different one, which is the proof that the file's
/// `सॱसङ्ख्या` is READ rather than ignored.
#[test]
fn the_file_is_a_function_of_the_state_and_not_a_constant() {
    assert_eq!(view_at(7), view_at(7));
    assert_ne!(view_at(7), view_at(8));
    assert_eq!(view_at(7).text()[0], "गणना ७");
}

/// `F-004f5` — EVENT TYPING, on the file. The fixture's two events are read
/// against the fixture's own `प्रकारः घटना भवति गणना आदि वर्धनम् ऽ ह्रासः ऽ इति ॥`,
/// so a button that names an event the file does not declare is REFUSED, and
/// the refusal names both the word and the type it is missing from. Mutating
/// the fixture in memory rather than keeping a second file: the refused
/// module differs from the accepted one by exactly one word, and a reader can
/// see which.
#[test]
fn a_button_naming_an_event_the_file_does_not_declare_is_refused() {
    let mutated = GANAKA.replace("ह्रासय इति ऽ ह्रासः", "ह्रासय इति ऽ नाशः");
    assert_ne!(mutated, GANAKA, "the mutation must land");
    let tokens = lex(&mutated).expect("still lexes: one word changed for another");
    let e = render(&tokens, &Sthiti::new().with("सङ्ख्या", 0))
        .expect_err("नाशः is not a variant of the fixture's घटना");
    assert!(e.reason.contains("नाशः"), "names the word: {}", e.reason);
    assert!(e.reason.contains("घटना"), "and the type: {}", e.reason);
}

/// The refusal above is only evidence if the declaration is READ from the
/// file: remove it and the two buttons the sketch itself writes are refused
/// too, because there is no longer a type for their events to be values of.
#[test]
fn the_events_are_typed_by_the_declaration_in_the_file_and_not_by_the_reader() {
    let without = GANAKA.replace(
        "प्रकारः घटना भवति गणना आदि\n    वर्धनम् ऽ\n    ह्रासः ऽ\nइति ॥\n",
        "",
    );
    assert_ne!(without, GANAKA, "the declaration must be removed");
    let tokens = lex(&without).expect("lexes");
    let e = render(&tokens, &Sthiti::new().with("सङ्ख्या", 0))
        .expect_err("with no घटना declared, the sketch's own buttons have no type");
    assert!(e.reason.contains("घटना"), "got {}", e.reason);
}

/// The other half of the acceptance, recorded rather than described: `sadhana`
/// the T0 assembler REFUSES this file, and its refusal names a construct of the
/// language it is not — which is what tells a reader these are two tiers rather
/// than one tier and a typo.
#[test]
fn it_is_refused_by_t0_and_that_is_the_guarded_answer() {
    let errors =
        sadhana::parse::assemble_program(GANAKA).expect_err("a T2 view is not T0 machine code");
    let joined = errors.join("\n");
    assert!(
        ["ॐ", "मण्डलम्", "इति", "न६४"]
            .iter()
            .any(|w| joined.contains(w)),
        "the refusal should name a construct of the other tier, got:\n{joined}"
    );
}

/// The fixture is the evidence every assertion above rests on. A test that
/// survives the disappearance of its own evidence is the defect this whole row
/// is about, so the file's identity is asserted and not assumed: it must be doc
/// 07 §4.2's counter and not some other view that happens to parse.
#[test]
fn the_fixture_is_the_sketch_and_this_test_is_not_vacuous() {
    for word in [
        "मण्डलम्",
        "दृश्यम्",
        "स्तम्भः",
        "पङ्क्तिः",
        "पाठः",
        "कुञ्जिका",
        "सन्ततिः",
        "विन्यासः",
    ] {
        assert!(GANAKA.contains(word), "the fixture does not write {word}");
    }
    // The five signs A-033 measured at 0 of 27 faces must NOT be here: the
    // fixture is the sketch in ADR-0003's spelling, and a stray Vedic sign
    // would mean it had been transcribed rather than translated.
    for sign in ['\u{A8FA}', '\u{A8FB}', '\u{A8FC}', '\u{A8F8}', '\u{A8F9}'] {
        assert!(
            !GANAKA.contains(sign),
            "the fixture carries {sign:?}, which ADR-0003 replaced with a word"
        );
    }
}
