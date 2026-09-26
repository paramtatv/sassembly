//! ADR-0018 end to end: a T1 program produces multi-line text.
//!
//! `emit.rs`'s `emit_program` wrote eight `push_str` calls; every one ended in
//! `\n` and seven began with four spaces. (`W-237`, 2026-09-04, DELETED that
//! routine and its T1 twin — their text was not T0, research/25 §1.2 — so the
//! eight lines below are kept as the FIXTURE this decision was taken on, and
//! `the_real_emitter_has_that_shape` reads the shape off the emitter that
//! replaced it, `riscv64::emit_module`.) ADR-0017 gave T1 a string literal and
//! left both of those unwritable — *"a string does not cross a newline"*, and a
//! leading space is *"not representable, because the delimiters are words and
//! words are whitespace-separated"* — so `crates/sadhana-t1/src/utsarjana.t1`'s
//! port of that routine is a stub whose note (b)(4) asks for this decision by
//! name. ADR-0018 is the answer the owner ratified: `यतिः` is LINE FEED and
//! `विवरम्` is SPACE, as **values**, not escapes.
//!
//! Everything here goes through the public path — [`sadhana::lex::lex_t1`],
//! [`sadhana::t1::drishya::render`] and [`sadhana::t1::parse::Parser`] — because
//! a test that reached inside the lexer would pass on a lexer no front end can
//! call. The two properties that matter are asserted from both readers, since
//! one implementation serving every reader is the whole reason the words are
//! taken by the lexer and not by each parser in turn.

use sadhana::lex::{Kind, lex, lex_t1};
use sadhana::t1::ast::{Declaration, Expression, Statement};
use sadhana::t1::drishya::{Rupa, Sthiti, render};
use sadhana::t1::parse::Parser;

/// `यतिः` — the caesura, ADR-0018's LINE FEED.
const NEWLINE: &str = "यतिः";
/// `विवरम्` — the interstice, ADR-0018's SPACE.
const SPACE: &str = "विवरम्";

/// A `दृश्यम्` whose whole body is one `पाठः`, so the expression under test is
/// the only thing the assertion can be about.
fn view_of(text_expr: &str) -> String {
    format!(
        "वृत्तिः दृश्यम् आरभ्य सॱॱ स्थितिः समाप्तम् फलम् रूपम् आदि\n\
         प्रत्यागमनम् पाठः आरभ्य {text_expr} समाप्तम् ।\n\
         इति ॥\n"
    )
}

/// The text a `पाठः` of `text_expr` renders to.
///
/// Panics with the diagnostic rather than unwrapping blind: a lexer error here
/// is the interesting result, not an inconvenience.
fn text_of(text_expr: &str) -> String {
    let src = view_of(text_expr);
    let tokens = lex_t1(&src).unwrap_or_else(|es| {
        panic!(
            "lex_t1 refused `{text_expr}`: {}",
            es.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        )
    });
    let view = render(&tokens, &Sthiti::new())
        .unwrap_or_else(|e| panic!("`{text_expr}` did not read as a view: {}", e.reason));
    match view {
        Rupa::Patha(text) => text,
        other => panic!("expected a पाठः, got {other:?}"),
    }
}

/// The row's own acceptance, in one line: a T1 program produces text of two
/// lines, and the newline in it came from a word.
#[test]
fn a_named_newline_makes_one_text_of_two_lines() {
    let text = text_of("उक्तम् क इति अधि यतिः अधि उक्तम् ख इति");
    assert_eq!(text, "क\nख");
    assert_eq!(text.lines().count(), 2);
}

/// The space, alone, and it is exactly one U+0020 — not a join, not a tab, not
/// two.
#[test]
fn a_named_space_is_one_space_and_nothing_else() {
    assert_eq!(text_of("विवरम्"), " ");
    assert_eq!(
        text_of("उक्तम् गणना इति अधि विवरम् अधि उक्तम् शून्यम् इति"),
        "गणना शून्यम्"
    );
}

/// **The words are values, not escapes.** Inside `उक्तम् … इति` they are the
/// letters they are made of, because ADR-0017's string is taken whole before
/// anything else is recognised. This is the property that distinguishes this
/// decision from the escape ADR-0011 refused: there is no position at which one
/// spelling means two things.
#[test]
fn a_layout_word_inside_a_string_is_its_own_letters() {
    assert_eq!(text_of("उक्तम् यतिः इति"), NEWLINE);
    assert_eq!(text_of("उक्तम् विवरम् इति"), SPACE);
    assert_eq!(text_of("उक्तम् यतिः विवरम् इति"), "यतिः विवरम्");
}

/// One implementation, every reader. `super::drishya` is not the only thing
/// that reads a string: `t1::parse` reads one too, and it gets the layout word
/// without a second rule — which is the reason the lexer takes the word and each
/// parser does not.
#[test]
fn the_other_t1_reader_takes_the_same_value_with_no_second_rule() {
    let tokens = lex_t1("वृत्तिः मुख्यम् आदि यतिः । इति").expect("lexes");
    let program = Parser::new(&tokens)
        .parse_program()
        .unwrap_or_else(|e| panic!("parse failed: {}", e.reason));
    let [Declaration::Function { body, .. }] = &program.declarations[..] else {
        panic!("expected one function, got {:?}", program.declarations);
    };
    let Some(Statement::Block(stmts)) = body else {
        panic!("expected a block body, got {body:?}");
    };
    let [Statement::Expression(Expression::StringLiteral(s))] = &stmts[..] else {
        panic!("expected one string statement, got {stmts:?}");
    };
    assert_eq!(s, "\n");
}

/// A layout word is one token and it carries both what was written and what it
/// means, exactly as a `उक्तम् … इति` literal does — including when a daṇḍa is
/// written against it, which is how every statement in the language ends.
#[test]
fn a_layout_word_is_one_token_that_knows_its_word_and_its_value() {
    let tokens = lex_t1("यतिः।").expect("lexes");
    assert_eq!(tokens.len(), 2, "a value and a daṇḍa: {tokens:?}");
    assert_eq!(tokens[0].kind, Kind::Str { value: "\n".into() });
    assert_eq!(tokens[0].text, NEWLINE, "`text` is the word as written");
    assert_eq!(tokens[1].kind, Kind::Danda);
}

/// T0 is untouched, and this is the test that says so. ADR-0018 is a T1
/// decision for ADR-0017's reason: `spec/grammar-t0.ebnf` is frozen and T0's
/// parser reassembles a string from words, so a word that became a value there
/// would change what every `ॱदत्त` holding text means.
#[test]
fn t0_does_not_take_a_layout_word() {
    let tokens = lex("॥ अष्टकाः उक्तम् यतिः विवरम् इति ॥").expect("lexes");
    assert!(
        !tokens.iter().any(|t| matches!(t.kind, Kind::Str { .. })),
        "T0 must not gain a value token: {tokens:?}"
    );
    assert!(
        tokens.iter().any(|t| t.text == NEWLINE) && tokens.iter().any(|t| t.text == SPACE),
        "both words are still ordinary words to T0: {tokens:?}"
    );
}

/// The repertoire is not weakened, which is the one way this change could have
/// been worse than the gap it fills. No character was added: R-15-1 (doc 15 §1)
/// already admits *"exactly three layout characters"*, and the two values named
/// here are two of those three.
#[test]
fn the_layout_words_add_no_character_to_the_repertoire() {
    let src = view_of("उक्तम् क इति अधि यतिः अधि विवरम् अधि उक्तम् ख इति");
    assert!(
        sanskrit_text::repertoire::check(&src).is_empty(),
        "the source that writes them is inside doc 15's repertoire"
    );
    for v in [text_of(NEWLINE), text_of(SPACE)] {
        let c = v.chars().next().expect("one character");
        assert_eq!(v.chars().count(), 1);
        assert!(
            c == '\u{0020}' || c == '\u{0009}' || c == '\u{000A}',
            "{c:?} is not one of R-15-1's three layout characters"
        );
    }
}

/// ADR-0017's rules are not weakened either, and this is the measurement that
/// says the blocker was real rather than an inconvenience.
///
/// A literal still cannot begin with a space — the leading whitespace belongs to
/// the delimiter — and still cannot cross a newline. Both are what the values
/// are *for*: the indent and the line end are written beside the literal, not
/// inside it.
#[test]
fn a_string_still_cannot_hold_the_indent_or_the_line_end() {
    assert_eq!(
        text_of("उक्तम्     निधेहि कोष्ठ५ इति"),
        "निधेहि कोष्ठ५",
        "four spaces after the opener still belong to the delimiter"
    );
    let e = lex_t1("उक्तम् क\nख इति").expect_err("a string still does not cross a newline");
    assert_eq!(e[0].aksara, "उक्तम्");
}

/// `emit_program`'s eight `push_str` calls, as one T1 expression — the fixture
/// ADR-0018 was decided on, kept after `W-237` deleted the routine.
///
/// The shape was the emitter's own, not one invented here; the register
/// spelling is `कोष्ठ` + a numeral, which is what `utsarjana.t1`'s
/// `अधिकरणवचनम्` returned — emit.rs's `x5`, `a0` and `main` were that file's
/// admitted Latin mocks. Neither routine exists now; the property under test is
/// the LANGUAGE's — a T1 program writes multi-line, indented text — and it is
/// the same property whatever the emitter.
const EMITTER_LINES: [&str; 8] = [
    "॥ जाल मुख्यम् ॥",
    "निधेहि कोष्ठ५ १०",
    "योगः कोष्ठ७ कोष्ठ५ कोष्ठ६",
    "वियोगः कोष्ठ८ कोष्ठ७ कोष्ठ५",
    "॰ अपूर्णा आज्ञा कोष्ठ९",
    "निधेहि अर्थ० कोष्ठ७",
    "प्रत्यावर्तनम्",
    "प्रत्यावर्तनम्",
];

/// Four spaces, written as four values, because a string cannot begin with one.
const INDENT: &str = "विवरम् अधि विवरम् अधि विवरम् अधि विवरम्";

/// **The end-to-end proof: a T1 program emits the eight lines `emit_program`
/// emits.** Line 1 carries no indent, the other seven do; every line ends in a
/// newline; and the fifth is the `_ =>` arm's `॰` comment, which is inside a
/// string and therefore text (ADR-0017 rule 3) rather than the start of one.
#[test]
fn the_emitters_eight_lines_are_writable_as_one_t1_expression() {
    let mut pieces = Vec::new();
    for (i, body) in EMITTER_LINES.iter().enumerate() {
        if i > 0 {
            pieces.push(INDENT.to_owned());
        }
        pieces.push(format!("उक्तम् {body} इति"));
        pieces.push(NEWLINE.to_owned());
    }
    let text = text_of(&pieces.join(" अधि "));

    let mut expected = String::new();
    for (i, body) in EMITTER_LINES.iter().enumerate() {
        if i > 0 {
            expected.push_str("    ");
        }
        expected.push_str(body);
        expected.push('\n');
    }
    assert_eq!(text, expected);

    assert_eq!(text.lines().count(), 8);
    assert_eq!(
        text.lines().filter(|l| l.starts_with("    ")).count(),
        7,
        "seven of the eight are indented by four spaces"
    );
    assert!(text.ends_with('\n'), "every line ends in a newline");
}

/// The shape asserted above is the REAL emitter's, read off the Rust function
/// rather than transcribed from its source: line oriented, every line ending in
/// a newline. `W-237` (2026-09-04): the function is `riscv64::emit_module` now,
/// not `emit_program` — T0 has no indent, so the four-space claim was the
/// retired routine's and goes with it; what survives is that the text the
/// machine assembles is lines, and that `सङ्केतन` reads it (`riscv64.rs`'s own
/// tests). If the emitter ever stops being line oriented, this fails and the
/// proof above stops claiming anything about it.
#[test]
fn the_real_emitter_has_that_shape() {
    use sadhana::t1::riscv64::{emit_module, fixture_recursive_sum};

    let out = emit_module(&fixture_recursive_sum()).expect("the fixture emits");
    assert!(out.ends_with('\n'), "every emitted line ends in a newline");
    assert!(
        out.lines().count() >= 8,
        "the emitter writes more than the fixture's eight lines:\n{out}"
    );
    assert!(
        out.lines().all(|l| !l.is_empty() && !l.starts_with(' ')),
        "a T0 line begins with its word, never with a space:\n{out}"
    );
    assert!(
        out.lines().filter(|l| l.ends_with(" ।")).count() >= 8,
        "the instruction lines end in the danda:\n{out}"
    );
}
