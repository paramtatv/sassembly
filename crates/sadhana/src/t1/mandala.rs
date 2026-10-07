//! **मण्डलम्** — T1's module system, the declaring half.
//!
//! ADR-0026, task `D-002l`. `spec/grammar-t1.ebnf` now carries three
//! productions:
//!
//! ```text
//! module_decl    = "मण्डलम्" , identifier , double_danda ;
//! import         = "आयातः" , identifier , danda ;
//! qualified_name = identifier , member_mark , identifier ;
//! ```
//!
//! # What was missing was only the DECLARING half
//!
//! `आयातः` has been a frozen keyword since the freeze and the corpus writes it;
//! what it named had no production, so an import imported nothing this grammar
//! could define. `मण्डलम्` is ADOPTED rather than coined —
//! `spec/lexicon.src.tsv:271` gives it as `module / keyword / attested` and
//! nineteen `.t1` sources already declare with it.
//!
//! # The two spellings, and why a reader that knows one is a seventh of a reader
//!
//! `split_trailing_punct` in `crates/sadhana/src/lex.rs` peels `ॱॱ । ॥ ऽ` off a word and never
//! the single `ॱ`, and `punctuation` maps a standalone `ॱ` to
//! [`Kind::MemberMark`]. So `सङ्केतनॱस्थापनम्` reaches this module as ONE token
//! whose text carries the mark, and `सङ्केतन ॱ स्थापनम्` as THREE. Both are
//! written in the tree, the unspaced one far more often. [`read_unit`] reads
//! both, and [`Spelling`] records which was used so a count can be reported per
//! spelling rather than asserted.
//!
//! # `ॱॱ` IS NOT `ॱ`, AND THE LEXER IS WHAT KEEPS THEM APART
//!
//! `crates/sadhana-t1/src/samyojana.t1:74` is `सारणी ॱॱ अङ्कः अन्तः कोशॱसंज्ञा`
//! — the FIELD `सारणी` and ADR-0003's annotation mark — and `सारणी` is *also* a
//! declared module (`crates/textapp/src/text/tables.t1:1`). A reader that takes
//! the first half of `ॱॱ` for a member mark turns that field into a qualified
//! name reaching a module `samyojana.t1` never imports, and reports a fourth
//! missing import that is a colon.
//!
//! **What defeats that here is working on TOKENS rather than on words.** The
//! lexer already decided: `ॱॱ` is [`Kind::LabelMark`] and `ॱ` is
//! [`Kind::MemberMark`], two kinds, and the spaced branch below requires
//! `MemberMark` EXACTLY. Widening it to accept a label mark fails
//! `a_field_named_like_a_module_before_the_annotation_mark_is_not_an_access`
//! here and `exactly_three_modules_are_reached_without_being_imported` in
//! `crates/sadhana-t1/tests/t1_modules.rs` — measured, by making that edit and
//! watching both go red.
//!
//! The `contains(ANNOTATION_MARK)` guard in [`read_unit`] is therefore BELT AND
//! BRACES and is stated as such rather than credited with the save: against a
//! stream from this lexer it is unreachable, because `ॱॱ` is peeled off a word
//! (`split_trailing_punct`) or stands alone as its own token. It earns its place
//! only because [`read_unit`] takes any `&[Token]` a caller can build, and
//! `a_word_the_lexer_could_not_produce_is_still_not_an_access` is the hand-built
//! case that reaches it.
//!
//! # The shape does not say which it is
//!
//! `पदम् ॱ पाठ` is a field of a local; `सङ्केतन ॱ पङ्क्तिसीमा` is a routine of a
//! module. They are the same three tokens. So [`read_unit`] collects
//! [`Access`]es — *candidates* — and only [`missing_imports`], which knows every
//! module DECLARED in the compilation set, decides which candidate is qualified.
//! That is a resolution question and the grammar says so.

use crate::lex::{Kind, LexError, Token};

/// `मण्डलम्` — the module declaration keyword.
pub const MODULE: &str = "मण्डलम्";
/// `आयातः` — the import keyword.
pub const IMPORT: &str = "आयातः";
/// `ॱ` U+0971, ADR-0003's member mark.
pub const MEMBER_MARK: &str = "ॱ";
/// `ॱॱ` U+0971 ×2, ADR-0003's colon role. Tested before [`MEMBER_MARK`].
pub const ANNOTATION_MARK: &str = "ॱॱ";

/// How a member access was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spelling {
    /// `सङ्केतनॱस्थापनम्` — one token, the mark inside its text.
    Unspaced,
    /// `सङ्केतन ॱ स्थापनम्` — three tokens.
    Spaced,
}

/// A name and the 1-based line it was written on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The name exactly as written.
    pub text: String,
    /// 1-based line.
    pub line: usize,
}

/// One `आयातः <name>` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The module named.
    pub name: Name,
    /// Whether a `।` closed it, as `import` requires. Recorded rather than
    /// enforced here: `crates/textapp/src/text/*.t1` write the whole statement
    /// wrapped as `॥ आयातः … ॥`, and a reader that silently dropped those would
    /// under-report their imports and invent missing ones.
    pub danda_terminated: bool,
}

/// A `head ॱ member` access, before anything decides what `head` is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Access {
    /// The word left of the mark.
    pub head: String,
    /// The word right of it. For a chained access the rest is kept whole; only
    /// the head decides whether this reaches a module.
    pub member: String,
    /// 1-based line of the head.
    pub line: usize,
    /// Which of the two spellings was used.
    pub spelling: Spelling,
}

/// One `.t1` compilation unit's module facts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Unit {
    /// The module this unit declares, if it declares one.
    pub module: Option<Name>,
    /// Every `मण्डलम्` after the first. A unit declares one module; a second
    /// declaration is reported rather than silently overwriting the first.
    pub extra_declarations: Vec<Name>,
    /// Every import, in source order.
    pub imports: Vec<Import>,
    /// Every member access, module or not.
    pub accesses: Vec<Access>,
}

impl Unit {
    /// Whether this unit imports `name`.
    #[must_use]
    pub fn imports_module(&self, name: &str) -> bool {
        self.imports.iter().any(|i| i.name.text == name)
    }

    /// Whether `name` is this unit's own module.
    #[must_use]
    pub fn declares_module(&self, name: &str) -> bool {
        self.module.as_ref().is_some_and(|m| m.text == name)
    }
}

/// Whether a token can stand as a name: a bare word, or a word the T0 sigil
/// split claimed.
///
/// The second half is not optional. `मण्डलम्` ends in `म्`, the कर्म sigil, so
/// the lexer hands it back as [`Kind::Operand`] and not [`Kind::Word`] — and so
/// do `सङ्केतन` (`न`, करण) and every other name ending in a sigil. Matching on
/// [`Kind::Word`] alone would see neither the keyword nor a third of the module
/// names. `Parser::match_word` in `crates/sadhana/src/t1/parse.rs` takes the same two kinds for
/// the same reason.
fn is_name(t: &Token) -> bool {
    matches!(t.kind, Kind::Word | Kind::Operand { .. } | Kind::Numeral)
}

/// Read one unit's module facts out of a T1 token stream.
///
/// The stream must come from [`crate::lex::lex_t1`] and not [`crate::lex::lex`]:
/// T1 lexing makes a string literal ONE [`Kind::Str`] token (ADR-0017), and this
/// function never looks inside one. `crates/sadhana-t1/src/parse.t1:280` writes
/// `उक्तम् आयातः इति` — the import keyword as string DATA — and a T0 stream hands
/// that over as a bare word, which would be read here as a twenty-first import
/// of a module named `इति`.
#[must_use]
pub fn read_unit(tokens: &[Token]) -> Unit {
    let mut unit = Unit::default();

    for (i, t) in tokens.iter().enumerate() {
        // A string is data. Never a keyword, never a mark.
        if matches!(t.kind, Kind::Str { .. }) {
            continue;
        }

        if t.text == MODULE {
            if let Some(n) = tokens.get(i + 1).filter(|n| is_name(n)) {
                let name = Name {
                    text: n.text.clone(),
                    line: n.line,
                };
                if unit.module.is_none() {
                    unit.module = Some(name);
                } else {
                    unit.extra_declarations.push(name);
                }
            }
            continue;
        }

        if t.text == IMPORT {
            if let Some(n) = tokens.get(i + 1).filter(|n| is_name(n)) {
                unit.imports.push(Import {
                    name: Name {
                        text: n.text.clone(),
                        line: n.line,
                    },
                    danda_terminated: tokens.get(i + 2).is_some_and(|d| d.kind == Kind::Danda),
                });
            }
            continue;
        }

        // BELT AND BRACES, and said so in the module header rather than
        // credited with the save: against a stream from this lexer `ॱॱ` is
        // always its own `Kind::LabelMark` token or already peeled off a word,
        // so this line is unreachable and removing it changes no result in the
        // corpus — measured. It stays because `read_unit` takes any `&[Token]`,
        // and `a_word_the_lexer_could_not_produce_is_still_not_an_access`
        // reaches it with a stream built by hand.
        if t.text.contains(ANNOTATION_MARK) {
            continue;
        }

        // UNSPACED: the mark is inside one word's text.
        if let Some(at) = t.text.find(MEMBER_MARK) {
            let head = &t.text[..at];
            let member = &t.text[at + MEMBER_MARK.len()..];
            if !head.is_empty() && !member.is_empty() {
                unit.accesses.push(Access {
                    head: head.to_string(),
                    member: member.to_string(),
                    line: t.line,
                    spelling: Spelling::Unspaced,
                });
            }
            continue;
        }

        // SPACED: name, mark, name.
        if is_name(t)
            && tokens
                .get(i + 1)
                .is_some_and(|m| m.kind == Kind::MemberMark)
            && let Some(member) = tokens.get(i + 2).filter(|n| is_name(n))
        {
            unit.accesses.push(Access {
                head: t.text.clone(),
                member: member.text.clone(),
                line: t.line,
                spelling: Spelling::Spaced,
            });
        }
    }

    unit
}

/// Read one T1 source's module facts.
///
/// # Errors
/// Whatever [`crate::lex::lex_t1`] refuses.
pub fn read(source: &str) -> Result<Unit, Vec<LexError>> {
    Ok(read_unit(&crate::lex::lex_t1(source)?))
}

/// Read a source's module facts **line by line**, keeping what lexes and
/// reporting what does not.
///
/// # Why this exists, and why it is not a weakening
///
/// Five `.t1` sources in the tree do not lex as T1 at all — four of
/// `crates/textapp/src/text/` are machine transliterations that still hold
/// `( ) = ::` and Latin digits, and `crates/sadhana-t1/src/parse.t1:198` writes
/// ADR-0011's escape one `इति` short. [`read`] gives up on all five, and giving
/// up is corrosive HERE in a way it is not elsewhere: a module the set no longer
/// knows about turns every import of it into an unknown import and every reach
/// into it into a field access. One bad line at `parse.t1:198` silently deletes
/// module `व्याकर` from the language.
///
/// A line is the right unit to fall back to because the grammar already says so:
/// **a string does not cross a newline** (`spec/grammar-t1.ebnf`, the note under
/// `string`), so lexing one line alone gives the same tokens lexing the file
/// does. `line_by_line_reading_agrees_with_whole_file_reading` asserts exactly
/// that over every source that lexes whole, so this path cannot quietly become
/// the weaker one.
///
/// What it does not see is a construct SPLIT across a newline. None of the three
/// module productions is written that way in this tree, and the test above is
/// what says so.
///
/// Each returned [`LexError`] carries its real `line` and `byte`; `index`, the
/// akṣara count, is line-local, because a refused line was never counted into
/// the file's running total.
#[must_use]
pub fn read_lossy(source: &str) -> (Unit, Vec<LexError>) {
    let mut tokens: Vec<Token> = Vec::new();
    let mut errors: Vec<LexError> = Vec::new();

    for (n, line) in source.lines().enumerate() {
        let line_no = n + 1;
        let start = line.as_ptr() as usize - source.as_ptr() as usize;
        match crate::lex::lex_t1(line) {
            Ok(ts) => tokens.extend(ts.into_iter().map(|mut t| {
                t.line = line_no;
                t.byte += start;
                t
            })),
            Err(errs) => errors.extend(errs.into_iter().map(|mut e| {
                e.line = line_no;
                e.byte += start;
                e
            })),
        }
    }

    (read_unit(&tokens), errors)
}

/// A module a unit reaches into without importing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingImport {
    /// The unit's key, as the caller named it — a path, usually.
    pub unit: String,
    /// The module reached.
    pub module: String,
    /// Every line of that unit that reaches it, ascending, deduplicated.
    pub sites: Vec<usize>,
}

/// An `आयातः` that names no module the compilation set declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownImport {
    /// The unit's key.
    pub unit: String,
    /// The name imported.
    pub module: String,
    /// 1-based line of the import.
    pub line: usize,
}

/// Every module name the set declares.
fn declared(units: &[(String, Unit)]) -> Vec<String> {
    let mut v: Vec<String> = units
        .iter()
        .filter_map(|(_, u)| u.module.as_ref().map(|m| m.text.clone()))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every module a unit reaches into without importing it.
///
/// An [`Access`] counts only when its head names a module the set DECLARES —
/// that is the whole disambiguation between `सङ्केतन ॱ पङ्क्तिसीमा` and
/// `पदम् ॱ पाठ`, which are the same shape. A unit's own module is not a missing
/// import: `crates/sadhana-t1/src/nidana.t1` writes `निदानॱ…` three times and is
/// reaching itself.
///
/// Sorted by unit then module, so the report is stable to compare against.
#[must_use]
pub fn missing_imports(units: &[(String, Unit)]) -> Vec<MissingImport> {
    let modules = declared(units);
    let mut out: Vec<MissingImport> = Vec::new();

    for (key, unit) in units {
        let mut by_module: Vec<(String, Vec<usize>)> = Vec::new();
        for a in &unit.accesses {
            if !modules.contains(&a.head)
                || unit.declares_module(&a.head)
                || unit.imports_module(&a.head)
            {
                continue;
            }
            match by_module.iter_mut().find(|(m, _)| *m == a.head) {
                Some((_, lines)) => lines.push(a.line),
                None => by_module.push((a.head.clone(), vec![a.line])),
            }
        }
        by_module.sort_by(|a, b| a.0.cmp(&b.0));
        for (module, mut sites) in by_module {
            sites.sort_unstable();
            sites.dedup();
            out.push(MissingImport {
                unit: key.clone(),
                module,
                sites,
            });
        }
    }

    out.sort_by(|a, b| (&a.unit, &a.module).cmp(&(&b.unit, &b.module)));
    out
}

/// Every `आयातः` naming a module no unit in the set declares.
///
/// The other half of the same question, and it is not hypothetical:
/// `crates/sadhana-t1/src/kosha.t1` declares `कओश` while two sources import
/// `कोश`.
#[must_use]
pub fn unknown_imports(units: &[(String, Unit)]) -> Vec<UnknownImport> {
    let modules = declared(units);
    let mut out: Vec<UnknownImport> = Vec::new();
    for (key, unit) in units {
        for i in &unit.imports {
            if !modules.contains(&i.name.text) {
                out.push(UnknownImport {
                    unit: key.clone(),
                    module: i.name.text.clone(),
                    line: i.name.line,
                });
            }
        }
    }
    out.sort_by(|a, b| (&a.unit, a.line).cmp(&(&b.unit, b.line)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(src: &str) -> Unit {
        read(src).expect("hand-written T1 source lexes")
    }

    #[test]
    fn a_module_declaration_is_read() {
        let u = unit("मण्डलम् वाक्यविभाग ॥\n");
        assert_eq!(
            u.module,
            Some(Name {
                text: "वाक्यविभाग".into(),
                line: 1
            })
        );
        assert!(u.extra_declarations.is_empty());
    }

    #[test]
    fn the_module_keyword_survives_the_karaka_split() {
        // `मण्डलम्` ends in `म्`, the कर्म sigil, so the lexer classifies the
        // keyword itself as an OPERAND. A reader that matched `Kind::Word`
        // would find no module declaration in the whole tree.
        let tokens = crate::lex::lex_t1("मण्डलम् वास्तु ॥\n").expect("lexes");
        assert!(
            matches!(tokens[0].kind, Kind::Operand { .. }),
            "the premise of `is_name` changed: {:?}",
            tokens[0].kind
        );
        assert_eq!(tokens[0].text, MODULE);
        assert_eq!(unit("मण्डलम् वास्तु ॥\n").module.unwrap().text, "वास्तु");
    }

    #[test]
    fn an_import_is_read_with_its_danda() {
        let u = unit("मण्डलम् अर्थ ॥\nआयातः वास्तु ।\n");
        assert_eq!(u.imports.len(), 1);
        assert_eq!(u.imports[0].name.text, "वास्तु");
        assert_eq!(u.imports[0].name.line, 2);
        assert!(u.imports[0].danda_terminated);
    }

    #[test]
    fn an_import_closed_with_the_double_danda_is_read_and_marked() {
        // `crates/textapp/src/text/ident.t1:2` is `॥ आयातः सअरणई ॥`.
        let u = unit("॥ मण्डलम् इडएनट ॥\n॥ आयातः सारणी ॥\n");
        assert_eq!(u.module.unwrap().text, "इडएनट");
        assert_eq!(u.imports.len(), 1);
        assert!(!u.imports[0].danda_terminated);
    }

    #[test]
    fn both_spellings_of_a_qualified_name_are_read() {
        let u = unit("मण्डलम् संयोजन ॥\nआयातः सङ्केतन ।\nसङ्केतनॱस्थापनम् ।\nसङ्केतन ॱ पङ्क्तिसीमा ।\n");
        assert_eq!(
            u.accesses,
            vec![
                Access {
                    head: "सङ्केतन".into(),
                    member: "स्थापनम्".into(),
                    line: 3,
                    spelling: Spelling::Unspaced,
                },
                Access {
                    head: "सङ्केतन".into(),
                    member: "पङ्क्तिसीमा".into(),
                    line: 4,
                    spelling: Spelling::Spaced,
                },
            ]
        );
    }

    #[test]
    fn a_word_the_lexer_could_not_produce_is_still_not_an_access() {
        // The `contains(ANNOTATION_MARK)` guard's only reachable case. `lex_t1`
        // peels `ॱॱ` off a word, so it never hands one over like this; a caller
        // building a stream by hand can, and this is what it gets.
        let toks = vec![Token {
            kind: Kind::Word,
            text: "सारणीॱॱअङ्कः".into(),
            byte: 0,
            aksara: 0,
            line: 1,
        }];
        assert_eq!(read_unit(&toks).accesses, vec![]);
        // And the same word WITHOUT the doubling is an access, so the guard is
        // discriminating rather than refusing everything.
        let toks = vec![Token {
            kind: Kind::Word,
            text: "सारणीॱअङ्कः".into(),
            byte: 0,
            aksara: 0,
            line: 1,
        }];
        assert_eq!(read_unit(&toks).accesses.len(), 1);
    }

    #[test]
    fn a_field_named_like_a_module_before_the_annotation_mark_is_not_an_access() {
        // `crates/sadhana-t1/src/samyojana.t1:74` — the field `सारणी` and the
        // colon role, NOT a reach into module `सारणी`. Let the spaced branch
        // accept `Kind::LabelMark` beside `Kind::MemberMark` and this line
        // becomes a fourth missing import; that edit fails this test by name.
        let tables = ("सारणी.t1".to_string(), unit("मण्डलम् सारणी ॥\n"));
        let user = (
            "समयोजन.t1".to_string(),
            unit("मण्डलम् संयोजन ॥\nसारणी ॱॱ अङ्कः अन्तः ।\n"),
        );
        assert_eq!(
            user.1.accesses,
            vec![],
            "the annotation mark was read as a member mark"
        );
        assert_eq!(missing_imports(&[tables, user]), vec![]);
    }

    #[test]
    fn a_glued_annotation_mark_is_not_an_access_either() {
        // `प्रारम्भःॱॱ` — the lexer peels `ॱॱ` and hands back the head, so the
        // word this module sees holds no mark at all.
        let u = unit("मण्डलम् व्याकर ॥\nप्रारम्भःॱॱ ।\n");
        assert_eq!(u.accesses, vec![]);
    }

    #[test]
    fn the_import_keyword_inside_a_string_is_data() {
        // `crates/sadhana-t1/src/parse.t1:280` writes exactly this.
        let u = unit("मण्डलम् व्याकर ॥\nउक्तम् आयातः इति ।\n");
        assert_eq!(u.imports, vec![], "a string literal was read as an import");
    }

    #[test]
    fn a_field_of_a_local_is_not_a_qualified_name() {
        // `पदम् ॱ पाठ` is the same three tokens as a module reach. It is an
        // access, and it is NOT a missing import, because no unit declares
        // `पदम्`.
        let u = unit("मण्डलम् वाक्यविभाग ॥\nपदम् ॱ पाठ ।\n");
        assert_eq!(u.accesses.len(), 1);
        assert_eq!(u.accesses[0].head, "पदम्");
        assert_eq!(missing_imports(&[("अ.t1".into(), u)]), vec![]);
    }

    #[test]
    fn reaching_an_unimported_module_is_reported_by_line() {
        let a = ("क.t1".to_string(), unit("मण्डलम् अक्षरकोश ॥\n"));
        let b = (
            "ख.t1".to_string(),
            unit("मण्डलम् वाक्यविभाग ॥\nआयातः पदविभाग ।\nअक्षरकोशॱमानम् ।\nअक्षरकोश ॱ मानदोषः ।\n"),
        );
        assert_eq!(
            missing_imports(&[a, b]),
            vec![MissingImport {
                unit: "ख.t1".into(),
                module: "अक्षरकोश".into(),
                sites: vec![3, 4],
            }]
        );
    }

    #[test]
    fn reaching_an_imported_module_is_not_reported() {
        let a = ("क.t1".to_string(), unit("मण्डलम् अक्षरकोश ॥\n"));
        let b = (
            "ख.t1".to_string(),
            unit("मण्डलम् वाक्यविभाग ॥\nआयातः अक्षरकोश ।\nअक्षरकोशॱमानम् ।\n"),
        );
        assert_eq!(missing_imports(&[a, b]), vec![]);
    }

    #[test]
    fn reaching_your_own_module_is_not_a_missing_import() {
        // `crates/sadhana-t1/src/nidana.t1` writes `निदानॱ…`.
        let u = unit("मण्डलम् निदान ॥\nनिदानॱस्थानम् ।\n");
        assert_eq!(missing_imports(&[("क.t1".into(), u)]), vec![]);
    }

    #[test]
    fn an_import_of_an_undeclared_module_is_reported() {
        let a = ("क.t1".to_string(), unit("मण्डलम् कओश ॥\n"));
        let b = ("ख.t1".to_string(), unit("मण्डलम् संयोजन ॥\nआयातः कोश ।\n"));
        assert_eq!(
            unknown_imports(&[a, b]),
            vec![UnknownImport {
                unit: "ख.t1".into(),
                module: "कोश".into(),
                line: 2,
            }]
        );
    }

    #[test]
    fn the_lossy_reader_keeps_the_lines_that_lex_and_names_the_one_that_does_not() {
        // Line 3 is `crates/textapp/src/text/nfc.t1:5`'s shape: a machine
        // transliteration that still holds `( ) ऽ` and Latin digits, outside the
        // doc 15 repertoire. `read` gives up on the whole file; this keeps the
        // module declaration, which is what the rest of the tree needs from it.
        let (unit, errs) = read_lossy(
            "मण्डलम् सारणी ॥\nआयातः वास्तु ।\nप्रत्यागमनम् सअरणईॱलओओकउप(सअरणईॱससस ऽ कह ऽ 0) ।\n",
        );
        assert_eq!(unit.module.unwrap().text, "सारणी");
        assert_eq!(unit.imports.len(), 1);
        // One diagnostic per offending WORD, so a line can raise several.
        assert!(!errs.is_empty());
        assert!(
            errs.iter().all(|e| e.line == 3),
            "an error was not renumbered to the file: {errs:?}"
        );
        assert!(
            read("मण्डलम् सारणी ॥\nआयातः वास्तु ।\nप्रत्यागमनम् सअरणईॱलओओकउप(सअरणईॱससस ऽ कह ऽ 0) ।\n")
                .is_err()
        );
    }

    #[test]
    fn line_by_line_reading_agrees_with_whole_file_reading() {
        // The property `read_lossy` rests on: a string does not cross a newline,
        // so a line lexed alone gives the tokens the file gives. Held here on a
        // source that exercises every construct, and over the whole corpus by
        // the test of the same name in `crates/sadhana-t1/tests/t1_modules.rs`.
        let src = "मण्डलम् संयोजन ॥\nआयातः सङ्केतन ।\n\
                   ॰ a comment holding उक्तम् and ॱ\n\
                   चरः पदम् ॱॱ अङ्कः अन्तः अ८ भवति सङ्केतनॱस्थापनम् ।\n\
                   उक्तम् आयातः इति ।\nसङ्केतन ॱ पङ्क्तिसीमा ।\n";
        let (lossy, errs) = read_lossy(src);
        assert_eq!(errs, vec![]);
        assert_eq!(read(src).unwrap(), lossy);
    }

    #[test]
    fn a_second_module_declaration_is_kept_rather_than_overwriting_the_first() {
        let u = unit("मण्डलम् अर्थ ॥\nमण्डलम् वास्तु ॥\n");
        assert_eq!(u.module.unwrap().text, "अर्थ");
        assert_eq!(u.extra_declarations.len(), 1);
        assert_eq!(u.extra_declarations[0].text, "वास्तु");
    }
}
