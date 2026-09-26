//! ADR-0017 end to end: a T1 source whose string carries a space and a `॰`
//! lexes and parses.
//!
//! `F-004f2`. What makes that row an ADR rather than an edit is not that a
//! production was missing a name — it is that the two characters a string could
//! not hold were the two that mattered. Doc 07 §4.2's counter spells its space
//! `॰ॱ`; before ADR-0017 `crates/sadhana/src/lex.rs` cut the line at the first
//! `॰` *before* it split words, so the escape was eaten with the rest of the
//! line and the whitespace was gone before any parser could ask for it. Three
//! parsers each joined the words back with one space and guessed.
//!
//! Everything here goes through the public path — [`sadhana::lex::lex_t1`] and
//! [`sadhana::t1::parse::Parser::parse_program`] — because a test that reached
//! inside the lexer would pass on a lexer no T1 front end can call. That is the
//! `encode.t1` failure in miniature: ten files parsed, one did not, and nothing
//! in the tree read any of them.

use sadhana::lex::{Kind, lex, lex_t1};
use sadhana::t1::ast::{Declaration, Expression, Statement};
use sadhana::t1::parse::Parser;

/// The whole row in one line: a string holding a space AND a `॰`, inside a
/// block whose close is spelled with the same word the string closes with.
const SOURCE: &str = "वृत्तिः मुख्यम् आदि उक्तम् गणना॰ॱ शून्यम् इति । इति";

/// Parse a program and return the text of the one string literal in it.
///
/// Panics with the diagnostic rather than unwrapping blind: a lexer error here
/// is the interesting result, not an inconvenience.
fn only_string(src: &str) -> String {
    let tokens = match lex_t1(src) {
        Ok(t) => t,
        Err(es) => panic!(
            "lex_t1 refused the source: {}",
            es.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        ),
    };
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
    s.clone()
}

/// The row's own acceptance, stated as a test.
///
/// A space and a `॰` in one string, neither ending the token nor starting a
/// comment, in a source that lexes *and* parses to the value that was written.
#[test]
fn a_string_carries_a_space_and_a_comment_mark_end_to_end() {
    assert_eq!(only_string(SOURCE), "गणना॰ॱ शून्यम्");
}

/// The `॰` alone, so a failure names which of the two properties broke.
#[test]
fn a_comment_mark_inside_a_string_does_not_start_a_comment() {
    assert_eq!(only_string("वृत्तिः मुख्यम् आदि उक्तम् क॰ख इति । इति"), "क॰ख");
    // Standing on its own as a word, too — the mark is text wherever it sits.
    assert_eq!(only_string("वृत्तिः मुख्यम् आदि उक्तम् क ॰ ख इति । इति"), "क ॰ ख");
}

/// The space alone, and it is the SOURCE's space rather than a re-inserted one.
///
/// A run of three spaces survives as three. That is what distinguishes ADR-0017
/// from the one-space join it replaces: a join produces the same answer for
/// every single-spaced literal in the tree, so only a multi-space literal can
/// tell the two apart.
#[test]
fn interior_whitespace_is_the_sources_own() {
    assert_eq!(only_string("वृत्तिः मुख्यम् आदि उक्तम् क ख इति । इति"), "क ख");
    assert_eq!(only_string("वृत्तिः मुख्यम् आदि उक्तम् क   ख इति । इति"), "क   ख");
}

/// ADR-0011 now holds in T1, which it did not before.
///
/// `उक्तम् सः इति इति अवदत् इति` is ADR-0011's own worked example. The word-loop
/// this replaces stopped at the first `इति` and returned `सः`, leaving
/// `इति अवदत् इति` to be read as a block close, an identifier and another block
/// close — so the program parsed, and meant something else.
#[test]
fn a_doubled_iti_is_the_word_itself_and_does_not_close_the_block() {
    assert_eq!(
        only_string("वृत्तिः मुख्यम् आदि उक्तम् सः इति इति अवदत् इति । इति"),
        "सः इति अवदत्"
    );
}

/// A string is one token, and the token knows both what was written and what it
/// means.
#[test]
fn a_string_is_one_token_carrying_its_source_and_its_value() {
    let tokens = lex_t1("उक्तम् क॰ख ग इति ।").expect("lexes");
    assert_eq!(tokens.len(), 2, "a string and a daṇḍa: {tokens:?}");
    assert_eq!(
        tokens[0].kind,
        Kind::Str {
            value: "क॰ख ग".into()
        }
    );
    assert_eq!(
        tokens[0].text, "उक्तम् क॰ख ग इति",
        "`text` is the source exactly, delimiters included"
    );
    assert_eq!(tokens[1].kind, Kind::Danda);
}

/// The repertoire is not weakened, which is the one way this change could have
/// been worse than the gap it fills.
///
/// A Latin letter inside `उक्तम् … इति` is refused exactly as it is anywhere
/// else. This is the defect that stopped `encode.t1` lexing over a `<`, and a
/// string is not a hole to smuggle one back through.
#[test]
fn a_string_is_not_a_hole_in_the_repertoire() {
    let e = lex_t1("उक्तम् क x ग इति ।").expect_err("a Latin letter is refused");
    assert!(
        e.iter().any(|v| v.aksara == "x"),
        "the diagnostic names the offending akṣara: {e:?}"
    );
}

/// An unclosed string is an error, and the error points at the opener.
///
/// Refusing is the decision. A string that ran to end of file would turn one
/// missing word into a diagnostic about the last line of the program.
#[test]
fn an_unclosed_string_is_refused_at_its_opener() {
    let e = lex_t1("वृत्तिः मुख्यम् आदि उक्तम् क ख\nइति").expect_err("refused");
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].line, 1, "reported on the line that opened it");
    assert_eq!(e[0].aksara, "उक्तम्");
}

/// T0 is untouched, and this is the test that says so.
///
/// The same source through [`lex`] yields no [`Kind::Str`] at all: `उक्तम्` is
/// still a word and `crate::parse::string_body` still finds the close. ADR-0017
/// changes which stage owns the string in T1 and changes nothing in T0, and if
/// that ever stops being true it should fail here rather than in a `.sas`
/// program that assembles to the wrong bytes.
#[test]
fn t0_still_lexes_a_string_as_words() {
    let tokens = lex("॥ अष्टकाः उक्तम् सः इति इति अवदत् इति ॥").expect("lexes");
    assert!(
        !tokens.iter().any(|t| matches!(t.kind, Kind::Str { .. })),
        "T0 must not gain a string token: {tokens:?}"
    );
    assert!(
        tokens.iter().any(|t| t.text == "उक्तम्"),
        "`उक्तम्` is still a word to T0: {tokens:?}"
    );
}
