//! ADR-0026 — `अङ्कः <numeral> अन्तः T`, a fixed-capacity array.
//!
//! # What this file can and cannot claim
//!
//! It reaches [`sadhana::t1::parse::Parser::parse_type`] DIRECTLY, and that is
//! not a shortcut — it is the only door there is.
//! [`sadhana::t1::parse::Parser::parse_program`] does not call `parse_type` at
//! all (`parse.rs`: *"Parse params (deferred for now, assume empty)"*, `B-080`),
//! so `every_t1_source_lexes_and_parses` in
//! `crates/sadhana-t1/tests/t1_sources.rs` walks all fifteen `.t1` sources
//! without ever reading a type. **No `.t1` source is checked against this
//! production today**, and saying so here is cheaper than a later reader
//! discovering that the port's green tests never touched it.
//!
//! Every input below is lexed by the real `lex_t1` rather than hand-built into
//! tokens, so a spelling this lexer cannot produce cannot pass.

use sadhana::lex::lex_t1;
use sadhana::t1::ast::{Declaration, Expression, Statement, Type};
use sadhana::t1::parse::Parser;
use sadhana::t1::resolve::Resolver;
use sadhana::t1::typecheck::TypeChecker;

fn parse_type(src: &str) -> Result<Type, String> {
    let tokens = lex_t1(src).map_err(|e| format!("lex: {e:?}"))?;
    Parser::new(&tokens)
        .parse_type()
        .map_err(|e| format!("parse: {}", e.reason))
}

/// The numeral between the marks is the ONLY thing that separates the two, and
/// each of the two still lands where it belongs.
///
/// This is the assertion that fails if the array arm is deleted (the bound is
/// consumed by nothing and `अन्तः` is not found, or the type comes back a
/// slice) and the one that fails if the two arms are swapped.
#[test]
fn the_bound_between_the_index_marks_is_what_separates_an_array_from_a_slice() {
    assert_eq!(
        parse_type("अङ्कः ८ अन्तः अ८").expect("an array parses"),
        Type::Array {
            element: Box::new(Type::Primitive("अ८".into())),
            capacity: 8,
        },
        "`अङ्कः ८ अन्तः अ८` is an array of eight octets"
    );

    // NOT CHANGED BY ADR-0026, and asserted in the same test so the two cannot
    // drift apart: a slice is this production with the bound left out.
    assert_eq!(
        parse_type("अङ्कः अन्तः अ८").expect("a slice still parses"),
        Type::Slice(Box::new(Type::Primitive("अ८".into()))),
        "`अङ्कः अन्तः अ८` was a slice before ADR-0026 and still is"
    );
}

/// The capacity is the number WRITTEN, read by the assembler's own numeral
/// reader.
///
/// A hand-rolled decimal reader would pass the first two and fail the third:
/// `०षोड्आ` is hexadecimal and `आ` is its eleventh digit. A reader that
/// returned a constant, or counted digits, fails all three.
#[test]
fn the_capacity_is_the_number_written_in_every_radix_the_language_has() {
    for (src, want) in [
        ("अङ्कः ८ अन्तः अ८", 8u64),
        ("अङ्कः १२८ अन्तः अ८", 128),
        ("अङ्कः ०षोड्आ अन्तः अ८", 11),
        ("अङ्कः ०अष्ट७ अन्तः अ८", 7),
        ("अङ्कः ०द्वि१०१ अन्तः अ८", 5),
    ] {
        let Type::Array { capacity, .. } = parse_type(src).unwrap_or_else(|e| panic!("{src}: {e}"))
        else {
            panic!("{src} did not parse as an array");
        };
        assert_eq!(capacity, want, "{src} declares {want} elements");
    }
}

/// Two bounds over one element are two types.
///
/// This is the assertion a reader that PARSED the numeral and then threw it
/// away would fail — such a reader satisfies the shape of every test above if
/// `capacity` is dropped from the checked type, and this one refuses it.
#[test]
fn two_capacities_over_one_element_are_two_different_types() {
    let four = parse_type("अङ्कः ४ अन्तः अ८").expect("parses");
    let eight = parse_type("अङ्कः ८ अन्तः अ८").expect("parses");
    assert_ne!(four, eight, "an array of four is not an array of eight");

    let resolver = Resolver::new();
    let mut tc = TypeChecker::new(&resolver);
    assert_ne!(
        ty_of(&mut tc, &four),
        ty_of(&mut tc, &eight),
        "the difference must survive into the checked type, not stop at the AST"
    );
}

/// The bound may be left out of an INNER type — an array of slices is the shape
/// `रूपाणि` was refused for, so it must be writable even though that routine
/// does not need it.
#[test]
fn an_array_of_slices_nests_the_two_spellings() {
    assert_eq!(
        parse_type("अङ्कः ४ अन्तः अङ्कः अन्तः अ८").expect("parses"),
        Type::Array {
            element: Box::new(Type::Slice(Box::new(Type::Primitive("अ८".into())))),
            capacity: 4,
        }
    );
}

/// `ऋण४` lexes as one numeral and is not a count.
///
/// Refused with the numeral in the message rather than clamped to zero or to
/// its magnitude: a capacity that is not the number written is a store of the
/// wrong size.
#[test]
fn a_negative_bound_is_refused_and_names_itself() {
    let e = parse_type("अङ्कः ऋण४ अन्तः अ८").expect_err("ऋण४ is not a capacity");
    assert!(
        e.contains("ऋण४"),
        "the refusal must quote what was written, got: {e}"
    );
}

fn ty_of(tc: &mut TypeChecker<'_>, ty: &Type) -> String {
    // `eval_ast_type` is private, so the return-type position is how a test
    // reaches it: `typecheck_program` evaluates it before it looks at a body.
    let program = program_returning(ty.clone());
    format!("{:?}", tc.typecheck_program(&program))
}

fn program_returning(ty: Type) -> sadhana::t1::ast::Program {
    sadhana::t1::ast::Program {
        declarations: vec![Declaration::Function {
            name: "क".into(),
            params: vec![],
            return_type: Some(ty),
            body: Some(Statement::Block(vec![Statement::Expression(
                Expression::Numeral("१".into()),
            )])),
        }],
    }
}

/// A capacity of `०` is refused, and by the TYPE CHECKER rather than the parser.
///
/// `०` is a well-formed numeral in a well-formed production, so the parser has
/// nothing to complain about — this asserts it parses AND is then refused, so a
/// fix that moved the refusal into the parser would be caught rather than
/// silently accepted.
///
/// The second half is what makes this test survive an inverted comparison: a
/// non-zero capacity must reach the ordinary type rule and fail there for a
/// DIFFERENT reason. Flipping `== 0` to `!= 0` swaps the two messages and both
/// assertions fail.
#[test]
fn a_capacity_of_zero_holds_nothing_and_is_refused_after_it_parses() {
    let zero = parse_type("अङ्कः ० अन्तः अ८").expect("`०` is a well-formed numeral");
    assert_eq!(
        zero,
        Type::Array {
            element: Box::new(Type::Primitive("अ८".into())),
            capacity: 0,
        },
        "the parser accepts it; the refusal is the checker's"
    );

    let resolver = Resolver::new();
    let mut tc = TypeChecker::new(&resolver);

    let refused = tc
        .typecheck_program(&program_returning(zero))
        .expect_err("an array of ० is refused");
    assert!(
        refused.reason.contains("holds nothing"),
        "expected the ० refusal, got: {}",
        refused.reason
    );

    let eight = parse_type("अङ्कः ८ अन्तः अ८").expect("parses");
    let other = tc
        .typecheck_program(&program_returning(eight))
        .expect_err("the body is a numeral, so the ordinary rule still refuses");
    assert!(
        !other.reason.contains("holds nothing"),
        "an array of eight must not be refused as empty, got: {}",
        other.reason
    );
}

/// The arenas this production exists for, read from the tree rather than typed
/// out here.
///
/// `ast.t1:65`, `:66` and `parse.t1:36`, `:38` declare four module-level stores
/// that the parser already writes past a rising index into (`parse.t1:107`,
/// `:117`, `:376`, `:92`) — the arena idiom `artha.t1:444`, `samyojana.t1:457`
/// and `encode.t1` all name as what T1 does instead of `push`. **Every one of
/// them is spelled as an UNBOUNDED slice**, and that is the gap ADR-0026 fills.
///
/// The element names are taken from the files' own bytes and never typed here,
/// because a hand-typed Devanagari name agrees with the source only by luck —
/// the first draft of this test typed one and did not match.
///
/// If a later row gives one of them a bound, this test fails and points at
/// it. That failure is the notification wanted: the capability would then have
/// its first real user, and this row's honest claim (that it has none yet)
/// would have stopped being true.
/// FIVE SINCE 2026-09-01, AND THE FIFTH IS DEBT THIS ROW NOW CARRIES.
/// `प्राचलकोश` was added to `parse.t1` so a routine's parameters survive
/// parsing — before it, `वृत्तिपठनम्` read each parameter into a local and
/// dropped it, and `अर्थॱकार्यक्रमनिर्णयः` therefore opened every body scope
/// EMPTY and refused every program that used a parameter. The arena was the
/// fix; it is unbounded like the other four, so it belongs in this census and
/// raises the count rather than being exempted from it.
///
/// THE NUMBER GOING UP IS NOT THIS GUARD FAILING. It counts arenas that ADR-
/// 0026's bound would apply to and none yet has, so a larger number is more
/// owed, and the honest move is to record it.
#[test]
fn the_unbounded_arenas_are_still_unbounded_and_each_would_take_a_bound() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .join("sadhana-t1")
        .join("src");

    let mut found: Vec<(String, String)> = Vec::new();
    for file in ["ast.t1", "parse.t1"] {
        let text = std::fs::read_to_string(root.join(file))
            .unwrap_or_else(|e| panic!("{file} is in the tree: {e}"));
        for line in text.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            // सार्वजनिक चरः <name> ॱॱ अङ्कः अन्तः <element> भवति ० ।
            if w.len() < 8 || w[0] != "सार्वजनिक" || w[1] != "चरः" {
                continue;
            }
            if w[3] != "ॱॱ" || w[4] != "अङ्कः" || w[5] != "अन्तः" {
                continue;
            }
            assert_eq!(
                w[7], "भवति",
                "{file}: `{}` now carries something between `अन्तः` and its \
                 initialiser — if that is ADR-0026's bound, this capability has \
                 its first user and this test should say so instead of guarding \
                 the gap",
                w[2]
            );
            found.push((w[2].to_string(), w[6].to_string()));
        }
    }

    println!("METRIC t1_unbounded_arenas {}", found.len());
    // Six since df66d098 (parse.t1's one-entry match cache added `मेलनपाठः`,
    // an octet run kept across tokens — unbounded like the other five).
    assert_eq!(
        found.len(),
        6,
        "expected the six arenas of ast.t1 and parse.t1, found {found:?}"
    );

    // Each of the four, with a bound, is what this production accepts.
    for (name, element) in &found {
        let bounded = format!("अङ्कः ४०९६ अन्तः {element}");
        assert_eq!(
            parse_type(&bounded).unwrap_or_else(|e| panic!("{name}: {bounded}: {e}")),
            Type::Array {
                element: Box::new(Type::Primitive(element.clone())),
                capacity: 4096,
            },
            "a bounded {name} must parse"
        );
    }
}
