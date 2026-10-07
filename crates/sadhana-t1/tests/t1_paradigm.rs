//! THE PARADIGM CENSUS — research/22 §7's letter table as an instrument (`W-206`).
//!
//! research/23's plan (§1) rules that every part of the two-conduits paradigm
//! becomes a number with a definition, an expected value and a falsifier, and
//! that the unit of analysis is a SITE: one occurrence of a construct, mapped
//! to one letter of research/22 §7's table BY ITS AST NODE KIND — never by
//! grepping text. This file is that map and the census that walks it.
//!
//! # What is here
//!
//! - [`LETTERS`]: the fourteen letters of §7 with their `(run, position)` on
//!   `spec/shiva-sutras.tsv`, in code, so a site can only be placed on a
//!   position the model has.
//! - [`place`]: the map from an AST node kind — statement, expression, operator,
//!   declaration, type — to a [`Place`]. It REFUSES a kind it cannot place.
//!   That refusal is research/22's **Rule L1** as an instrument: "a construct
//!   that cannot be placed on this table is either not needed or not yet
//!   understood; say which before adding it." A census that met such a kind
//!   and counted it anyway would be inventing a letter, which is the failure
//!   the plan's §4 names as the thesis being wrong.
//! - [`measure_corpus_paradigm`]: the `#[ignore]` census over the fifteen T1
//!   sources, loaded and parsed the way `measure_corpus_resolve` does it, and
//!   printing `METRIC paradigm_<part>_<stat> <value>` lines for `crates/metrics`
//!   to harvest and for `tools/paradigm-report.py` to turn into
//!   `research/22-stats.md`. It measures the plan's statistics **5** (statements
//!   per routine), **6** (span pairing rate per pair kind), **24** (operator
//!   frequency by ladder level, and chained comparisons) and **25** (sign
//!   purity), each with its exceptions listed as `file:line`.
//!
//! - [`measure_corpus_loops`] (`W-207`): the म loop's statistics **11–15** —
//!   back-edge containment (on the AST, and it says so), condition shape,
//!   entry, step presence with every no-step loop listed, nesting — as
//!   `METRIC paradigm_loop_*` lines.
//!
//! # What is deliberately not here
//!
//! Call statistics (16–19) and junction statistics (7–10, 20, 21) belong to
//! W-208 and W-209. This file gives them the letter table, the loader, the
//! arena snapshot and the walk; it does not take their numbers.
//!
//! # Two traps the plan names, obeyed here
//!
//! Every arena is 1-based with slot ० reserved (`वास्तुॱवाक्य`'s margin), so a
//! child index of ० means NO CHILD and every walk starts at १. And the parser's
//! type reader pushes TYPE nodes into the EXPRESSION arena with
//! `मूलप्रकारभेद`..`दोषयुक्तप्रकारभेद` = १०१..१०५ (disjoint from the
//! expression kinds since `W-171`; they collided at १..५ before) — still one
//! arena, so expressions are
//! reached from their ROOTS (a statement's operands, a binding's initialiser)
//! and types from THEIR roots (a `चरः`'s annotation, a parameter, a return
//! type), and the census reports how many arena slots neither walk reached.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Unique scratch roots, shared with every other binary that needs one —
/// because five copies of `(pid, counter)` was five copies of one defect.
mod spec_fixture;

// ─────────────────────────────────────────────────────────────────────────
// The letter table — research/22 §7, every row, with its place on the model.
// ─────────────────────────────────────────────────────────────────────────

/// The fourteen letters on the loops, research/22 §7's table in order.
///
/// Two are `ह`, at 5-1 and 14-1 — the junction seen from each of its two
/// places — and they are two letters here because they are two rows there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Letter {
    /// ह 5-1 — program entry.
    Ha5,
    /// य 5-2 — binding of the caller's names before the call.
    Ya,
    /// व 5-3 — the call site, the point control returns to.
    Va,
    /// र 5-4 — argument passing on the way out.
    Ra,
    /// ट् 5-5 → ट 11-7 — the call itself: the callee's entry.
    Tta,
    /// त 11-8 — the body: the work the callee does.
    Ta,
    /// व् 11-9 — return.
    VaIt,
    /// म 7-2 — loop head, the point the back-edge lands on.
    Ma,
    /// ङ 7-3 — the condition.
    Nga,
    /// ण 7-4 — loop entry from outside; a declaration.
    Nna,
    /// न 7-5 — the step: what changes before the back-edge.
    Na,
    /// म् 7-6 — the conditional back-edge.
    MaIt,
    /// ल 6-1 — yield / re-entry: a program's end back into the cycle.
    La,
    /// ह 14-1 — program end handing control back to the loop.
    Ha14,
}

/// One row of research/22 §7's table.
struct LetterRow {
    letter: Letter,
    /// The letter as the table writes it.
    glyph: &'static str,
    /// The ASCII key used in a `METRIC` name.
    key: &'static str,
    /// `(sūtra, position)` on `spec/shiva-sutras.tsv`.
    run: u8,
    pos: u8,
    /// For `ट`: the departing marker `ट्` at 5-5 whose secondary edge lands on
    /// the letter. The table writes the row as `5-5 → 11-7`.
    marker: Option<(u8, u8)>,
    /// The language construct, §7's fourth column.
    name: &'static str,
}

/// research/22 §7, row for row. `run`/`pos` are checked against
/// `spec/shiva-sutras.tsv` by [`every_letter_sits_where_the_model_puts_it`].
const LETTERS: [LetterRow; 14] = [
    LetterRow {
        letter: Letter::Ha5,
        glyph: "ह",
        key: "ha5",
        run: 5,
        pos: 1,
        marker: None,
        name: "program entry",
    },
    LetterRow {
        letter: Letter::Ya,
        glyph: "य",
        key: "ya",
        run: 5,
        pos: 2,
        marker: None,
        name: "binding of the caller's names",
    },
    LetterRow {
        letter: Letter::Va,
        glyph: "व",
        key: "va",
        run: 5,
        pos: 3,
        marker: None,
        name: "the call site",
    },
    LetterRow {
        letter: Letter::Ra,
        glyph: "र",
        key: "ra",
        run: 5,
        pos: 4,
        marker: None,
        name: "argument passing",
    },
    LetterRow {
        letter: Letter::Tta,
        glyph: "ट",
        key: "tta",
        run: 11,
        pos: 7,
        marker: Some((5, 5)),
        name: "the call: the callee's entry",
    },
    LetterRow {
        letter: Letter::Ta,
        glyph: "त",
        key: "ta",
        run: 11,
        pos: 8,
        marker: None,
        name: "the body",
    },
    LetterRow {
        letter: Letter::VaIt,
        glyph: "व्",
        key: "v_it",
        run: 11,
        pos: 9,
        marker: None,
        name: "return",
    },
    LetterRow {
        letter: Letter::Ma,
        glyph: "म",
        key: "ma",
        run: 7,
        pos: 2,
        marker: None,
        name: "loop head",
    },
    LetterRow {
        letter: Letter::Nga,
        glyph: "ङ",
        key: "nga",
        run: 7,
        pos: 3,
        marker: None,
        name: "the condition",
    },
    LetterRow {
        letter: Letter::Nna,
        glyph: "ण",
        key: "nna",
        run: 7,
        pos: 4,
        marker: None,
        name: "loop entry; a declaration",
    },
    LetterRow {
        letter: Letter::Na,
        glyph: "न",
        key: "na",
        run: 7,
        pos: 5,
        marker: None,
        name: "the step",
    },
    LetterRow {
        letter: Letter::MaIt,
        glyph: "म्",
        key: "m_it",
        run: 7,
        pos: 6,
        marker: None,
        name: "the conditional back-edge",
    },
    LetterRow {
        letter: Letter::La,
        glyph: "ल",
        key: "la",
        run: 6,
        pos: 1,
        marker: None,
        name: "yield / re-entry",
    },
    LetterRow {
        letter: Letter::Ha14,
        glyph: "ह",
        key: "ha14",
        run: 14,
        pos: 1,
        marker: None,
        name: "program end",
    },
];

fn row(letter: Letter) -> &'static LetterRow {
    LETTERS
        .iter()
        .find(|r| r.letter == letter)
        .expect("every Letter has a row")
}

// ─────────────────────────────────────────────────────────────────────────
// Where a site can sit — a letter, a marker, or a run off the loops.
// ─────────────────────────────────────────────────────────────────────────

/// Where the map puts a node kind. Rule L1 allows "a letter, a marker, or an
/// edge between two of them"; §7's last paragraph adds the runs OFF the loops
/// (vowels, stops, sibilants) as hypotheses with a falsifier each, and §4.2
/// reads the precedence ladder against the same runs. Nothing else exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Place {
    /// One of the fourteen letters on the loops.
    Letter(Letter),
    /// A silent form whose secondary edge lands on a loop letter. The one this
    /// corpus needs is `ण्` at 1-4 → `ण` at 7-4: a USE of a name is a marker
    /// pointing at its declaration (§2's "secondary continuation", and §7's
    /// "ण is the target of two other markers … where the loop is entered from
    /// outside").
    Marker { run: u8, pos: u8, target: Letter },
    /// A run, or a range of runs, off the loops: `from..=to`.
    Runs { from: u8, to: u8, key: &'static str },
}

impl Place {
    /// ASCII key for a `METRIC` name.
    fn key(self) -> String {
        match self {
            Self::Letter(l) => row(l).key.to_string(),
            Self::Marker { run, pos, target } => format!("{}_it_{run}_{pos}", row(target).key),
            Self::Runs { key, .. } => key.to_string(),
        }
    }

    /// The place as research/22 writes it, for the human table.
    fn describe(self) -> String {
        match self {
            Self::Letter(l) => {
                let r = row(l);
                match r.marker {
                    Some((mr, mp)) => {
                        format!("{} {mr}-{mp} → {}-{} — {}", r.glyph, r.run, r.pos, r.name)
                    }
                    None => format!("{} {}-{} — {}", r.glyph, r.run, r.pos, r.name),
                }
            }
            Self::Marker { run, pos, target } => {
                let r = row(target);
                format!(
                    "marker {run}-{pos} → {} {}-{} — a use, resolved",
                    r.glyph, r.run, r.pos
                )
            }
            Self::Runs { from, to, key } => format!("runs {from}–{to} — {key}"),
        }
    }
}

/// The vowels, runs 1–4: a value (§2 row 1; §7 "the vowels are the values").
const VOWELS: Place = Place::Runs {
    from: 1,
    to: 4,
    key: "vowels",
};
/// The nasals, run 7: bitwise mixing — shift, and, xor, or (§4.2).
const NASALS: Place = Place::Runs {
    from: 7,
    to: 7,
    key: "nasals",
};
/// The stops, runs 8–11: arithmetic — mul, add (§4.2; §7 "the stops are the
/// arithmetic").
const STOPS: Place = Place::Runs {
    from: 8,
    to: 11,
    key: "stops",
};
/// The sibilants, run 13: the comparisons (§4.2; §7).
const SIBILANTS: Place = Place::Runs {
    from: 13,
    to: 13,
    key: "sibilants",
};
/// A name use: `ण्` at 1-4 pointing at `ण` at 7-4.
const USE: Place = Place::Marker {
    run: 1,
    pos: 4,
    target: Letter::Nna,
};

// ─────────────────────────────────────────────────────────────────────────
// The node kinds — `ast.t1` and `parse.t1`'s discriminants, by name.
// ─────────────────────────────────────────────────────────────────────────

// Statement kinds, `वास्तुॱ*वाक्यभेद` (ast.t1:132-139).
const S_EXPRESSION: i128 = 1;
const S_BLOCK: i128 = 2;
const S_BINDING: i128 = 3;
const S_RETURN: i128 = 4;
const S_IMPORT: i128 = 5;
const S_IF: i128 = 6;
const S_WHILE: i128 = 7;
const S_ASSIGN: i128 = 8;

// Expression kinds, `वास्तुॱ*अभिव्यञ्जकभेद` (ast.t1:68-117).
const E_NAME: i128 = 1;
const E_NUMERAL: i128 = 2;
const E_STRING: i128 = 3;
const E_GROUP: i128 = 4;
const E_INDEX: i128 = 5;
const E_FIELD: i128 = 6;
const E_CALL: i128 = 7;
const E_BINARY: i128 = 8;
const E_BOOL: i128 = 9;
const E_NULL: i128 = 10;
const E_EMBED: i128 = 11;
const E_NEGATE: i128 = 12;
/// `slice = index , postfix_expr` — W-228; `वामसूचकाङ्क` is the index node,
/// `दक्षिणसूचकाङ्क` the limit.
const E_SLICE: i128 = 13;

// Operator kinds, `वास्तुॱ*द्विकर्मभेद` (ast.t1:37-66), and their ladder level
// (`spec/grammar-t1.ebnf:673-690`).
const OPERATORS: [(i128, &str, &str); 15] = [
    (1, "yogah", "add"),
    (2, "viyogah", "add"),
    (3, "gunanam", "mul"),
    (4, "vibhajanam", "mul"),
    (5, "samam", "compare"),
    (6, "asamam", "compare"),
    (7, "nyunam", "compare"),
    (8, "adhikam", "compare"),
    (9, "yuk", "and"),
    (10, "vikalpa", "or"),
    (11, "vishama", "xor"),
    (12, "vamasr", "shift"),
    (13, "dakshinasr", "shift"),
    (14, "sheshah", "mul"),
    (15, "brhatsamam", "compare"),
];

// Declaration kinds, `व्याकरॱ*घोषणाभेद` (parse.t1:68-82).
const D_ROUTINE: i128 = 1;
const D_TYPE: i128 = 2;
const D_MACHINE: i128 = 3;
const D_BINDING: i128 = 4;
const D_IMPORT: i128 = 5;
/// `गणनाघोषणाभेद` — an enum, its own kind since `W-215` (it shared `D_TYPE`
/// with the struct before); its variants follow it as bindings.
const D_ENUM: i128 = 6;

// Type kinds, `वास्तुॱ*प्रकारभेद` — pushed into the EXPRESSION arena by
// `प्रकारपठनम्`, which is why they need their own family here. At base १०१
// since `W-171` renumbered them out of the expression kinds' range.
const T_FIRST: i128 = 101;
const T_LAST: i128 = 105;

/// An AST node kind, by family. `Program` is the `मण्डलम्` header, which
/// `व्याकर` reads and records nowhere (parse.t1:671), so the census takes it
/// from the first token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Kind {
    Program,
    Declaration(i128),
    Statement(i128),
    Expression(i128),
    Operator(i128),
    Type(i128),
}

/// Why the map refused — the L1 answer, "not needed" or "not yet understood".
#[derive(Debug, PartialEq, Eq)]
enum Unplaced {
    /// No such kind exists in `ast.t1`/`parse.t1`; the census is misreading
    /// an arena.
    Unknown(Kind),
    /// The kind exists and the parser never builds one, so nothing has been
    /// understood about it: `यन्त्रघोषणाभेद` (parse.t1:70).
    NeverBuilt(Kind),
    /// A binary expression is placed by its OPERATOR, not by its kind.
    ByOperator,
}

/// The map. Every arm cites the row of research/22 that places it.
fn place(kind: Kind) -> Result<Place, Unplaced> {
    use Letter as L;
    Ok(match kind {
        // §7: ह 5-1 "program entry — `मण्डलम्` and the entry label".
        Kind::Program => Place::Letter(L::Ha5),

        // §7: ट "the call itself: the callee's entry … the label the jump lands
        // on"; plan §0's baseline puts `वृत्तिः` (475) at ट.
        Kind::Declaration(D_ROUTINE) => Place::Letter(L::Tta),
        // Plan §0: "vowels — types in position". A type declaration declares a
        // shape of value.
        Kind::Declaration(D_TYPE) => VOWELS,
        // `W-215`: an enum declares a closed set of VALUES — its variants are
        // the vowels it admits — so it sits where a type declaration sits.
        Kind::Declaration(D_ENUM) => VOWELS,
        Kind::Declaration(D_MACHINE) => return Err(Unplaced::NeverBuilt(kind)),
        // §7: ण "the induction variable's `चरः`"; plan §0 puts `चरः` at ण.
        Kind::Declaration(D_BINDING) => Place::Letter(L::Nna),
        // §7: य "binding of the caller's names before the call — `आयातः`".
        Kind::Declaration(D_IMPORT) => Place::Letter(L::Ya),

        // §7: व "the call site, the point control returns to". An expression
        // statement is a call whose value is dropped; the CALL inside it is ट.
        Kind::Statement(S_EXPRESSION) => Place::Letter(L::Va),
        // §7: त "the body … the routine's `आदि … इति`".
        Kind::Statement(S_BLOCK) => Place::Letter(L::Ta),
        Kind::Statement(S_BINDING) => Place::Letter(L::Nna),
        // §7: व् "return — `प्रत्यागमनम्`".
        Kind::Statement(S_RETURN) => Place::Letter(L::VaIt),
        Kind::Statement(S_IMPORT) => Place::Letter(L::Ya),
        // §7: ङ "the condition … the `यावत् … आदि` test; the six branch
        // conditions"; plan §0 puts `यदि` at ङ.
        Kind::Statement(S_IF) => Place::Letter(L::Nga),
        // §7: म "loop head, the point the back-edge lands on"; plan §0 `यावत्`.
        Kind::Statement(S_WHILE) => Place::Letter(L::Ma),
        // §7: न "the step: what changes before the back-edge — the increment;
        // the store that makes progress". `भवति` is the store.
        Kind::Statement(S_ASSIGN) => Place::Letter(L::Na),

        // §2: "every use of a name" is a marker; §7: ण is the target of ण्.
        Kind::Expression(E_NAME) | Kind::Expression(E_FIELD) => USE,
        // §2 row 1: "a value: a literal, a numeral" — `numeral`, `string`,
        // `बूल` literals; `शून्यम्` is the value a `सम्भाव्य` is compared to.
        Kind::Expression(E_NUMERAL)
        | Kind::Expression(E_STRING)
        | Kind::Expression(E_BOOL)
        | Kind::Expression(E_NULL) => VOWELS,
        // §7: र "argument passing on the way out — `आरभ्य … ऽ … समाप्तम्`; the
        // kāraka-marked operands". An index `अङ्कः … अन्तः` is an operand
        // reached through a span the same way; §4.2 puts postfix in row 5.
        // A slice is an index with its limit: the same reach, bounded twice.
        Kind::Expression(E_GROUP) | Kind::Expression(E_INDEX) | Kind::Expression(E_SLICE) => {
            Place::Letter(L::Ra)
        }
        // §7: ट "`आह्वान`".
        Kind::Expression(E_CALL) => Place::Letter(L::Tta),
        Kind::Expression(E_BINARY) => return Err(Unplaced::ByOperator),
        // `समावेशः आरभ्य <table> समाप्तम्` binds a generated table's names
        // before use — the same act as `आयातः`, one file instead of one module.
        Kind::Expression(E_EMBED) => Place::Letter(L::Ya),
        // ADR-0009: negation is arithmetic; §4.2 reads `unary` against row 6
        // as a mnemonic and the stops for arithmetic. The arithmetic reading
        // is taken, and said here.
        Kind::Expression(E_NEGATE) => STOPS,

        // §4.2's table, row by row.
        Kind::Operator(op) => match OPERATORS.iter().find(|(k, _, _)| *k == op) {
            Some((_, _, "add" | "mul")) => STOPS,
            Some((_, _, "shift" | "and" | "xor" | "or")) => NASALS,
            Some((_, _, "compare")) => SIBILANTS,
            _ => return Err(Unplaced::Unknown(kind)),
        },

        Kind::Type(t) if (T_FIRST..=T_LAST).contains(&t) => VOWELS,

        _ => return Err(Unplaced::Unknown(kind)),
    })
}

/// The ladder level of an operator kind, from `spec/grammar-t1.ebnf:673-690`.
fn ladder(op: i128) -> Option<&'static str> {
    OPERATORS
        .iter()
        .find(|(k, _, _)| *k == op)
        .map(|(_, _, l)| *l)
}

// ─────────────────────────────────────────────────────────────────────────
// Loading and parsing — the same loader and calls `measure_corpus_resolve`
// uses, so a file this census reads is a file that census reads.
// ─────────────────────────────────────────────────────────────────────────

fn source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// The corpus, sorted by name.
fn corpus_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(source_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    names
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// `load_sema_with_parser`'s module list (t1_execution.rs), copied rather than
/// shared: that helper's count is keyed by another census.
fn load_parser() -> Interpreter {
    let names = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "artha.t1",
        "sanskrit_text.t1",
    ];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| {
            let p = source_dir().join(n);
            let t = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
            ((*n).to_string(), t)
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

/// One token of `पदविभागॱचिह्नक`: kind, the byte range in the source, line.
#[derive(Clone, Debug, Default)]
struct Tok {
    kind: i128,
    start: usize,
    end: usize,
    line: i128,
}

/// One `वास्तुॱवाक्य`.
#[derive(Clone, Copy, Debug, Default)]
struct Stmt {
    kind: i128,
    left: i128,
    right: i128,
    /// `आदिसूचकाङ्क` — the first index of this statement's own subtree.
    first: i128,
    /// `प्रकारसूचकाङ्क` — a `चरः`'s declared type, ० when none.
    ty: i128,
}

/// One `वास्तुॱअभिव्यञ्जक`.
#[derive(Clone, Copy, Debug, Default)]
struct Expr {
    kind: i128,
    value: i128,
    left: i128,
    right: i128,
    op: i128,
}

/// One `व्याकरॱघोषणा`.
#[derive(Clone, Copy, Debug, Default)]
struct Decl {
    kind: i128,
    name: i128,
    ty: i128,
    body: i128,
    p0: i128,
    p1: i128,
}

/// A parsed source, with every arena copied out of the interpreter. Index ०
/// of each vector is the reserved slot and holds a default.
///
/// `W-261`: `Clone` so [`parse_corpus`] can hand every ratchet in this binary
/// the same parse. COPIED OUT is what makes that safe — see the margin there.
#[derive(Clone)]
struct Program {
    name: String,
    src: String,
    toks: Vec<Tok>,
    stmts: Vec<Stmt>,
    exprs: Vec<Expr>,
    decls: Vec<Decl>,
    /// `(name token, type expression)` per parameter.
    params: Vec<(i128, i128)>,
    parse_errors: i128,
}

impl Program {
    fn text(&self, t: &Tok) -> &str {
        &self.src[t.start..t.end]
    }

    fn tok(&self, i: i128) -> Option<&Tok> {
        usize::try_from(i)
            .ok()
            .filter(|i| *i > 0)
            .and_then(|i| self.toks.get(i))
    }

    fn stmt(&self, i: i128) -> Option<Stmt> {
        usize::try_from(i)
            .ok()
            .filter(|i| *i > 0)
            .and_then(|i| self.stmts.get(i).copied())
    }

    fn expr(&self, i: i128) -> Option<Expr> {
        usize::try_from(i)
            .ok()
            .filter(|i| *i > 0)
            .and_then(|i| self.exprs.get(i).copied())
    }

    /// The line of an expression: the line of its leftmost token.
    fn expr_line(&self, i: i128) -> i128 {
        let mut i = i;
        for _ in 0..64 {
            let Some(e) = self.expr(i) else { return 0 };
            if let Some(t) = self.tok(e.value) {
                return t.line;
            }
            if e.left == 0 {
                return 0;
            }
            i = e.left;
        }
        0
    }
}

fn field(v: &Value, name: &str) -> i128 {
    match v {
        Value::Record(r) => r.borrow().get(name).and_then(Value::as_int).unwrap_or(0),
        _ => 0,
    }
}

fn arena_records(it: &Interpreter, name: &str) -> Vec<Value> {
    match it.global(name) {
        Some(Value::Arena(a)) => a.borrow().clone(),
        Some(other) => panic!("`{name}` is {other:?}, not an arena"),
        None => panic!("the global `{name}` does not exist — THIS CENSUS is broken, not व्याकर"),
    }
}

fn count_global(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("the global `{name}` does not exist — THIS CENSUS is broken"))
}

/// Lex and parse one source through `व्याकर`, then copy the arenas out.
///
/// A source the parser refuses is still returned, with `parse_errors` > 0 and
/// whatever it built — the census prints the refusal rather than skipping the
/// file, because a skipped file looks exactly like a file with no sites.
fn parse_source(name: &str, src: &str) -> Program {
    let mut it = load_parser();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .unwrap_or_else(|e| panic!("{name}: पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .unwrap_or_else(|e| panic!("{name}: कार्यक्रमपठनम् runs: {e:?}"));
    let parse_errors = count_global(&it, "दोषसूचकाङ्क");

    let ntoks = usize::try_from(toks).expect("fits");
    let toks: Vec<Tok> = arena_records(&it, "चिह्नककोश")
        .iter()
        .take(ntoks + 1)
        .map(|v| Tok {
            kind: field(v, "भेद"),
            start: usize::try_from(field(v, "अष्टक")).unwrap_or(0),
            end: usize::try_from(field(v, "पाठसीमा")).unwrap_or(0),
            line: field(v, "पङ्क्ति"),
        })
        .collect();
    let nstmts = usize::try_from(count_global(&it, "वाक्यसूचकाङ्क")).expect("fits");
    let stmts: Vec<Stmt> = arena_records(&it, "वाक्यकोश")
        .iter()
        .take(nstmts + 1)
        .map(|v| Stmt {
            kind: field(v, "भेद"),
            left: field(v, "वामसूचकाङ्क"),
            right: field(v, "दक्षिणसूचकाङ्क"),
            first: field(v, "आदिसूचकाङ्क"),
            ty: field(v, "प्रकारसूचकाङ्क"),
        })
        .collect();
    let nexprs = usize::try_from(count_global(&it, "अभिव्यञ्जकसूचकाङ्क")).expect("fits");
    let exprs: Vec<Expr> = arena_records(&it, "अभिव्यञ्जककोश")
        .iter()
        .take(nexprs + 1)
        .map(|v| Expr {
            kind: field(v, "भेद"),
            value: field(v, "मूल्यसूचकाङ्क"),
            left: field(v, "वामसूचकाङ्क"),
            right: field(v, "दक्षिणसूचकाङ्क"),
            op: field(v, "द्विकर्म"),
        })
        .collect();
    let ndecls = usize::try_from(count_global(&it, "घोषणासूचकाङ्क")).expect("fits");
    let decls: Vec<Decl> = arena_records(&it, "घोषणाकोश")
        .iter()
        .take(ndecls + 1)
        .map(|v| Decl {
            kind: field(v, "भेद"),
            name: field(v, "नामसूचकाङ्क"),
            ty: field(v, "प्रकारसूचकाङ्क"),
            body: field(v, "शरीरसूचकाङ्क"),
            p0: field(v, "प्राचलादि"),
            p1: field(v, "प्राचलान्त"),
        })
        .collect();
    let nparams = usize::try_from(count_global(&it, "प्राचलसूचकाङ्क")).expect("fits");
    let params: Vec<(i128, i128)> = arena_records(&it, "प्राचलकोश")
        .iter()
        .take(nparams + 1)
        .map(|v| (field(v, "नामसूचकाङ्क"), field(v, "प्रकारसूचकाङ्क")))
        .collect();

    Program {
        name: name.to_string(),
        src: src.to_string(),
        toks,
        stmts,
        exprs,
        decls,
        params,
        parse_errors,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The walk — every site placed, or the census refuses.
// ─────────────────────────────────────────────────────────────────────────

/// What one walk of one program found.
#[derive(Default)]
struct Sites {
    /// Sites per place.
    by_place: BTreeMap<Place, usize>,
    /// Sites per node kind, for the human table and the coverage check.
    by_kind: BTreeMap<Kind, usize>,
    /// Expression-arena slots reached as an expression, and as a type.
    reached_expr: Vec<bool>,
    reached_type: Vec<bool>,
}

impl Sites {
    fn site(&mut self, kind: Kind) {
        let p = match place(kind) {
            Ok(p) => p,
            Err(why) => panic!(
                "RULE L1: the census met a node kind it cannot place on research/22 §7's \
                 table — {why:?}. Say whether it is not needed or not yet understood \
                 before adding a letter."
            ),
        };
        *self.by_place.entry(p).or_default() += 1;
        *self.by_kind.entry(kind).or_default() += 1;
    }
}

/// Walk an expression subtree from its root, placing every node.
fn walk_expr(p: &Program, s: &mut Sites, i: i128) {
    let Some(e) = p.expr(i) else { return };
    if let Some(slot) = s.reached_expr.get_mut(usize::try_from(i).expect("fits")) {
        *slot = true;
    }
    match e.kind {
        E_BINARY => {
            s.site(Kind::Operator(e.op));
            walk_expr(p, s, e.left);
            walk_expr(p, s, e.right);
        }
        E_GROUP | E_NEGATE => {
            s.site(Kind::Expression(e.kind));
            walk_expr(p, s, e.left);
        }
        E_INDEX | E_CALL | E_SLICE => {
            s.site(Kind::Expression(e.kind));
            walk_expr(p, s, e.left);
            walk_expr(p, s, e.right);
        }
        E_FIELD => {
            s.site(Kind::Expression(e.kind));
            walk_expr(p, s, e.left);
        }
        k => s.site(Kind::Expression(k)),
    }
}

/// Walk a type from its root — the same arena, a different family.
fn walk_type(p: &Program, s: &mut Sites, i: i128) {
    let Some(t) = p.expr(i) else { return };
    if let Some(slot) = s.reached_type.get_mut(usize::try_from(i).expect("fits")) {
        *slot = true;
    }
    s.site(Kind::Type(t.kind));
    if t.kind != T_FIRST {
        walk_type(p, s, t.left);
    }
}

/// The expression and type roots of one statement, by its kind — read off
/// `वाक्ययोजनम्`'s callers (parse.t1:893-993), which is also the table
/// `वाक्ययोजनम्` itself branches on.
fn walk_stmt_roots(p: &Program, s: &mut Sites, st: Stmt) {
    match st.kind {
        S_EXPRESSION | S_RETURN => walk_expr(p, s, st.left),
        S_BINDING => {
            walk_type(p, s, st.ty);
            walk_expr(p, s, st.right);
        }
        S_IF | S_WHILE => walk_expr(p, s, st.left),
        S_ASSIGN => {
            walk_expr(p, s, st.left);
            walk_expr(p, s, st.right);
        }
        _ => {}
    }
}

/// Place every site of one program.
fn walk(p: &Program) -> Sites {
    let mut s = Sites {
        reached_expr: vec![false; p.exprs.len()],
        reached_type: vec![false; p.exprs.len()],
        ..Sites::default()
    };
    if p.toks.get(1).is_some_and(|t| p.text(t) == "मण्डलम्") {
        s.site(Kind::Program);
    }
    for d in p.decls.iter().skip(1) {
        s.site(Kind::Declaration(d.kind));
        match d.kind {
            D_ROUTINE => {
                walk_type(p, &mut s, d.ty);
                if d.p0 > 0 {
                    for i in d.p0..=d.p1 {
                        if let Some((_, ty)) = p.params.get(usize::try_from(i).expect("fits")) {
                            walk_type(p, &mut s, *ty);
                        }
                    }
                }
            }
            D_BINDING => {
                walk_type(p, &mut s, d.ty);
                walk_expr(p, &mut s, d.body);
            }
            // `W-244`: a struct's FIELDS live in the प्राचल run since `W-215`
            // (e6b05a62) — a field is a name and a type, exactly a parameter —
            // and their type nodes sit in the expression arena. This walk
            // visited a routine's run and not a struct's, so every field type
            // in the corpus was an "unreached slot" (289 at 15347eb6, kosha.t1
            // alone 4 with 0 statements) and rule L1 read UNLISTED. Decided in
            // writing (research/22 §7): a field is a TYPE POSITION, placed on
            // the vowels as a parameter's type is; the letter table is not
            // extended. An enum's `p0..p1` is its VARIANT DECLARATIONS' run —
            // declaration indices, walked as declarations by this loop already —
            // and not a प्राचल run, so it is not read here.
            D_TYPE => {
                if d.p0 > 0 {
                    for i in d.p0..=d.p1 {
                        if let Some((_, ty)) = p.params.get(usize::try_from(i).expect("fits")) {
                            walk_type(p, &mut s, *ty);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for st in p.stmts.iter().skip(1) {
        s.site(Kind::Statement(st.kind));
        walk_stmt_roots(p, &mut s, *st);
    }
    s
}

// ─────────────────────────────────────────────────────────────────────────
// Statistic 5 — statements per routine (plan §2.2). Shape.
// ─────────────────────────────────────────────────────────────────────────

/// A statement is a `वाक्य` that is not a `समूह` block: the block is the span
/// (`आदि … इति`, त), its statements are the sequence (।). A routine's
/// statements are the contiguous arena range of its body — `आदिसूचकाङ्क` to
/// the body's own index — which includes the `अन्यथा` bodies the parser
/// pushes and then drops from the `यदि` node (parse.t1:966).
#[derive(Default)]
struct Sequence {
    /// Statements per routine, in declaration order.
    per_routine: Vec<usize>,
    /// Routines whose body is ० — declared with no `आदि … इति`.
    without_body: usize,
    /// Non-block statements, all of them.
    statements: usize,
    /// Block statements, all of them.
    blocks: usize,
    /// Non-block statements no routine's range covers — expected ०, since
    /// only a `वृत्ति` has statements.
    outside_routines: usize,
    /// The routine with the most statements, as `(count, "file:line name")`.
    largest: Option<(usize, String)>,
}

fn sequence(p: &Program) -> Sequence {
    let mut s = Sequence::default();
    let mut covered = vec![false; p.stmts.len()];
    for d in p.decls.iter().skip(1).filter(|d| d.kind == D_ROUTINE) {
        let Some(body) = p.stmt(d.body) else {
            s.without_body += 1;
            s.per_routine.push(0);
            continue;
        };
        let lo = usize::try_from(body.first).expect("fits");
        let hi = usize::try_from(d.body).expect("fits");
        let mut n = 0;
        for (seen, st) in covered[lo..=hi].iter_mut().zip(&p.stmts[lo..=hi]) {
            *seen = true;
            if st.kind != S_BLOCK {
                n += 1;
            }
        }
        if s.largest.as_ref().is_none_or(|(m, _)| n > *m) {
            let (line, name) = p.tok(d.name).map_or((0, "?"), |t| (t.line, p.text(t)));
            s.largest = Some((n, format!("{}:{line} {name}", p.name)));
        }
        s.per_routine.push(n);
    }
    for (i, st) in p.stmts.iter().enumerate().skip(1) {
        if st.kind == S_BLOCK {
            s.blocks += 1;
        } else {
            s.statements += 1;
            if !covered[i] {
                s.outside_routines += 1;
            }
        }
    }
    s
}

/// Nearest-rank quantile of a sorted sample: the smallest value at or above
/// the `p`-th fraction. `p = 0.5` on an even sample is the lower middle.
fn quantile(sorted: &[usize], p: f64) -> usize {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

// ─────────────────────────────────────────────────────────────────────────
// Statistic 6 — span pairing rate per pair kind (plan §2.2). Rule: 100%.
// ─────────────────────────────────────────────────────────────────────────

/// Openers seen and openers whose closer was found, for one pair kind.
#[derive(Default)]
struct Pairing {
    openers: usize,
    paired: usize,
    /// `file:line what` for every opener left unpaired or closer that did not
    /// match — each one is a row candidate.
    exceptions: Vec<String>,
}

/// The four pairs of research/22 §4.1, walked over the TOKEN stream — an
/// instrument independent of the parser, which refuses an unpaired span
/// (the statistic exists "to be seen, once, and to catch a parser regression").
///
/// `उक्तम् … इति` is walked by ADR-0011's rule as `मूलाभिव्यञ्जकपठनम्` applies
/// it (parse.t1:512-546): inside a string a doubled `इति इति` is the literal
/// word and a lone `इति` closes. A string's body may contain `आदि`, `अन्तः` or
/// `समाप्तम्` — the parser's own error messages do — so it is skipped whole.
#[derive(Default)]
struct Spans {
    adi_iti: Pairing,
    arabhya_samaptam: Pairing,
    ankah_antah: Pairing,
    uktam_iti: Pairing,
}

const TOKEN_WORD: i128 = 1;

fn span_pairing(p: &Program) -> Spans {
    let mut s = Spans::default();
    let word = |i: usize| -> Option<&str> {
        p.toks
            .get(i)
            .filter(|t| t.kind == TOKEN_WORD)
            .map(|t| p.text(t))
    };
    fn bucket<'a>(s: &'a mut Spans, opener: &str) -> &'a mut Pairing {
        match opener {
            "आदि" => &mut s.adi_iti,
            "आरभ्य" => &mut s.arabhya_samaptam,
            _ => &mut s.ankah_antah,
        }
    }
    // (opener, line) — the stack of open spans.
    let mut stack: Vec<(&str, i128)> = Vec::new();
    let mut i = 1;
    while i < p.toks.len() {
        let line = p.toks[i].line;
        match word(i) {
            Some("उक्तम्") => {
                s.uktam_iti.openers += 1;
                let mut j = i + 1;
                let mut closed = false;
                while j < p.toks.len() {
                    if word(j) == Some("इति") {
                        if word(j + 1) == Some("इति") {
                            j += 2;
                            continue;
                        }
                        closed = true;
                        break;
                    }
                    j += 1;
                }
                if closed {
                    s.uktam_iti.paired += 1;
                } else {
                    s.uktam_iti
                        .exceptions
                        .push(format!("{}:{line} उक्तम् never closed", p.name));
                }
                i = j + 1;
                continue;
            }
            Some(w @ ("आदि" | "आरभ्य" | "अङ्कः")) => {
                bucket(&mut s, w).openers += 1;
                stack.push((w, line));
            }
            Some(c @ ("इति" | "समाप्तम्" | "अन्तः")) => {
                let wants = match c {
                    "इति" => "आदि",
                    "समाप्तम्" => "आरभ्य",
                    _ => "अङ्कः",
                };
                // The closer looks DOWN the stack for the opener it wants.
                // Every span above that opener was left open and is closed
                // over — each is its own exception at its own line — and the
                // opener found is paired, so one unclosed inner span does not
                // unpair every span around it and hide as a hundred.
                match stack.iter().rposition(|(o, _)| *o == wants) {
                    Some(at) => {
                        for (o, oline) in stack.drain(at + 1..) {
                            bucket(&mut s, o).exceptions.push(format!(
                                "{}:{oline} {o} never closed — closed over by {c} at line {line}",
                                p.name
                            ));
                        }
                        stack.pop();
                        bucket(&mut s, wants).paired += 1;
                    }
                    None => bucket(&mut s, wants)
                        .exceptions
                        .push(format!("{}:{line} {c} with nothing open", p.name)),
                }
            }
            _ => {}
        }
        i += 1;
    }
    for (o, oline) in stack {
        bucket(&mut s, o)
            .exceptions
            .push(format!("{}:{oline} {o} never closed", p.name));
    }
    s
}

// ─────────────────────────────────────────────────────────────────────────
// Statistic 24 — operators by ladder level; chained comparisons (plan §2.7).
// ─────────────────────────────────────────────────────────────────────────

/// Comparison operators on one ungrouped operator SPINE.
///
/// `अभिव्यञ्जकपठनम्` (parse.t1:748-881) has no ladder: it reads a postfix
/// expression, then an operator, then the REST recursively, so a flat run
/// `a op1 b op2 c` is `op1(a, op2(b, c))` — every operator of one ungrouped
/// run sits on the right spine of its head. `compare_expr` in the frozen
/// grammar (`spec/grammar-t1.ebnf:675`) is `or_expr , [ compare_op , or_expr ]`
/// — at most ONE comparison per ungrouped run — so a spine with two is a
/// chained comparison the grammar refuses and the corpus writes anyway.
fn chained_comparisons(p: &Program, roots: &[i128]) -> Vec<String> {
    fn spine_compares(p: &Program, i: i128, acc: &mut usize, heads: &mut Vec<i128>) {
        let Some(e) = p.expr(i) else { return };
        match e.kind {
            E_BINARY => {
                if ladder(e.op) == Some("compare") {
                    *acc += 1;
                }
                spine_compares(p, e.left, acc, heads);
                spine_compares(p, e.right, acc, heads);
            }
            // Below a grouping boundary a new spine begins.
            E_GROUP | E_NEGATE | E_INDEX | E_CALL | E_FIELD | E_SLICE => {
                heads.push(e.left);
                if matches!(e.kind, E_INDEX | E_CALL | E_SLICE) {
                    heads.push(e.right);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    let mut pending: Vec<i128> = roots.to_vec();
    while let Some(head) = pending.pop() {
        if head == 0 {
            continue;
        }
        let mut n = 0;
        spine_compares(p, head, &mut n, &mut pending);
        if n >= 2 {
            out.push(format!(
                "{}:{} {n} comparisons on one ungrouped run",
                p.name,
                p.expr_line(head)
            ));
        }
    }
    out
}

/// Every expression root of a program: statement operands and binding
/// initialisers — the same roots [`walk`] takes.
fn expression_roots(p: &Program) -> Vec<i128> {
    let mut roots = Vec::new();
    for d in p.decls.iter().skip(1).filter(|d| d.kind == D_BINDING) {
        roots.push(d.body);
    }
    for st in p.stmts.iter().skip(1) {
        match st.kind {
            S_EXPRESSION | S_RETURN | S_IF | S_WHILE => roots.push(st.left),
            S_BINDING => roots.push(st.right),
            S_ASSIGN => {
                roots.push(st.left);
                roots.push(st.right);
            }
            _ => {}
        }
    }
    roots
}

// ─────────────────────────────────────────────────────────────────────────
// Statistic 25 — sign purity (plan §2.7, rule S1). Rule: 0.
// ─────────────────────────────────────────────────────────────────────────

/// Signs in CODE — each line up to its first `॰`, which is where `lex.t1`'s
/// `टिप्पणीसीमा` cuts it (lex.t1:267), strings included, since the lexer cuts
/// them the same way.
#[derive(Default)]
struct Signs {
    danda: usize,
    double_danda: usize,
    /// A lone `ॱ` — member access.
    member_mark: usize,
    /// `ॱॱ` — the label / type-annotation role.
    label_mark: usize,
    avagraha: usize,
    om: usize,
    comment_lines: usize,
    /// Characters outside the repertoire, as `file:line U+XXXX`.
    nonrepertoire: Vec<String>,
}

fn sign_purity(p: &Program) -> Signs {
    let mut s = Signs::default();
    for (n, line) in p.src.lines().enumerate() {
        let code = match line.find('॰') {
            Some(at) => {
                s.comment_lines += 1;
                &line[..at]
            }
            None => line,
        };
        let chars: Vec<char> = code.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            match c {
                '।' => s.danda += 1,
                '॥' => s.double_danda += 1,
                'ऽ' => s.avagraha += 1,
                'ॐ' => s.om += 1,
                'ॱ' => {
                    if chars.get(i + 1) == Some(&'ॱ') {
                        s.label_mark += 1;
                        i += 1;
                    } else {
                        s.member_mark += 1;
                    }
                }
                c if c.is_whitespace() => {}
                // Letters, vowel signs, virāma, nuktā, anusvāra, visarga,
                // candrabindu and the digits ०–९: the Devanagari block less
                // the signs matched above and the Vedic accents.
                '\u{0900}'..='\u{097F}' => {
                    if matches!(c, '\u{0951}'..='\u{0954}') {
                        s.nonrepertoire
                            .push(format!("{}:{} U+{:04X}", p.name, n + 1, c as u32));
                    }
                }
                other => {
                    s.nonrepertoire
                        .push(format!("{}:{} U+{:04X}", p.name, n + 1, other as u32))
                }
            }
            i += 1;
        }
    }
    s
}

/// A rate as the plan prints it: `paired/openers` in percent, one decimal.
/// ० of ० is reported as 100.0% — nothing was asked, nothing failed.
fn rate(num: usize, den: usize) -> String {
    if den == 0 {
        return "100.0%".to_string();
    }
    format!("{:.1}%", 100.0 * num as f64 / den as f64)
}

// ─────────────────────────────────────────────────────────────────────────
// The census.
// ─────────────────────────────────────────────────────────────────────────

/// Everything the census prints, gathered so a test can read it too.
#[derive(Default)]
struct Census {
    metrics: Vec<(String, String)>,
    notes: Vec<String>,
}

impl Census {
    fn metric(&mut self, name: &str, value: impl std::fmt::Display) {
        self.metrics
            .push((format!("paradigm_{name}"), value.to_string()));
    }

    fn note(&mut self, line: impl Into<String>) {
        self.notes.push(line.into());
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.metrics
            .iter()
            .find(|(k, _)| k == &format!("paradigm_{name}"))
            .map(|(_, v)| v.as_str())
    }
}

/// The census over a set of parsed programs.
fn census(programs: &[Program]) -> Census {
    let mut c = Census::default();
    let mut total = Sites::default();
    let mut refused = 0;
    let mut orphans = 0usize;
    let mut arena_slots = 0usize;

    for p in programs {
        if p.parse_errors > 0 {
            c.note(format!("  {:24} PARSE ERRORS: {}", p.name, p.parse_errors));
            refused += 1;
        }
        let s = walk(p);
        for (k, v) in &s.by_place {
            *total.by_place.entry(*k).or_default() += v;
        }
        for (k, v) in &s.by_kind {
            *total.by_kind.entry(*k).or_default() += v;
        }
        let unreached = s
            .reached_expr
            .iter()
            .zip(&s.reached_type)
            .skip(1)
            .filter(|(e, t)| !**e && !**t)
            .count();
        orphans += unreached;
        arena_slots += p.exprs.len().saturating_sub(1);
        c.note(format!(
            "  {:24} {:>4} decls {:>5} stmts {:>6} exprs ({unreached} unreached)",
            p.name,
            p.decls.len().saturating_sub(1),
            p.stmts.len().saturating_sub(1),
            p.exprs.len().saturating_sub(1),
        ));
    }

    c.metric("corpus_files", programs.len());
    c.metric("corpus_files_refused", refused);
    // Every letter of the table gets a line, INCLUDING the zeros: ल, ह-14 and
    // म् have no AST construct in T1 (§7 says so — म् is the IR's back-edge,
    // ल is the scheduler, ह-14 the halt), and a line that says ० is the
    // honest gap made visible rather than a missing row.
    for r in &LETTERS {
        let n = total
            .by_place
            .get(&Place::Letter(r.letter))
            .copied()
            .unwrap_or(0);
        c.metric(&format!("letters_{}_sites", r.key), n);
    }
    for p in [USE, VOWELS, NASALS, STOPS, SIBILANTS] {
        let n = total.by_place.get(&p).copied().unwrap_or(0);
        c.metric(&format!("letters_{}_sites", p.key()), n);
    }
    c.metric("letters_kinds_placed", total.by_kind.len());
    c.metric("letters_expression_slots_unreached", orphans);
    c.note(format!(
        "  expression arena: {arena_slots} slots, {orphans} reached by neither the \
         expression walk nor the type walk"
    ));

    // ── 5. The primary conduit: statements per routine ──────────────────
    let mut per_routine: Vec<usize> = Vec::new();
    let (mut statements, mut blocks, mut without_body, mut outside) = (0, 0, 0, 0);
    let mut largest: Option<(usize, String)> = None;
    for p in programs {
        let s = sequence(p);
        per_routine.extend(&s.per_routine);
        statements += s.statements;
        blocks += s.blocks;
        without_body += s.without_body;
        outside += s.outside_routines;
        if let Some((n, who)) = s.largest
            && largest.as_ref().is_none_or(|(m, _)| n > *m)
        {
            largest = Some((n, who));
        }
    }
    if let Some((n, who)) = &largest {
        c.note(format!("  largest routine: {who} — {n} statements"));
    }
    per_routine.sort_unstable();
    c.metric("sequence_routines", per_routine.len());
    c.metric("sequence_routines_without_body", without_body);
    c.metric("sequence_statements", statements);
    c.metric("sequence_blocks", blocks);
    c.metric("sequence_statements_outside_routines", outside);
    c.metric(
        "sequence_statements_per_routine_min",
        per_routine.first().copied().unwrap_or(0),
    );
    c.metric(
        "sequence_statements_per_routine_median",
        quantile(&per_routine, 0.5),
    );
    c.metric(
        "sequence_statements_per_routine_p90",
        quantile(&per_routine, 0.9),
    );
    c.metric(
        "sequence_statements_per_routine_max",
        per_routine.last().copied().unwrap_or(0),
    );
    if !per_routine.is_empty() {
        let mean = per_routine.iter().sum::<usize>() as f64 / per_routine.len() as f64;
        c.metric("sequence_statements_per_routine_mean", format!("{mean:.1}"));
    }

    // ── 6. Span pairing rate per pair kind ──────────────────────────────
    let mut spans = Spans::default();
    for p in programs {
        let s = span_pairing(p);
        for (into, from) in [
            (&mut spans.adi_iti, s.adi_iti),
            (&mut spans.arabhya_samaptam, s.arabhya_samaptam),
            (&mut spans.ankah_antah, s.ankah_antah),
            (&mut spans.uktam_iti, s.uktam_iti),
        ] {
            into.openers += from.openers;
            into.paired += from.paired;
            into.exceptions.extend(from.exceptions);
        }
    }
    for (key, pr) in [
        ("adi_iti", &spans.adi_iti),
        ("arabhya_samaptam", &spans.arabhya_samaptam),
        ("ankah_antah", &spans.ankah_antah),
        ("uktam_iti", &spans.uktam_iti),
    ] {
        c.metric(&format!("spans_{key}_openers"), pr.openers);
        c.metric(&format!("spans_{key}_paired"), pr.paired);
        c.metric(
            &format!("spans_{key}_unpaired"),
            pr.openers.saturating_sub(pr.paired),
        );
        c.metric(&format!("spans_{key}_rate"), rate(pr.paired, pr.openers));
        for e in &pr.exceptions {
            c.note(format!("  SPAN {key}: {e}"));
        }
    }

    // ── 24. Operators by ladder level; chained comparisons ──────────────
    let mut by_level: BTreeMap<&str, usize> = BTreeMap::new();
    for (op, key, level) in &OPERATORS {
        let n = total
            .by_kind
            .get(&Kind::Operator(*op))
            .copied()
            .unwrap_or(0);
        c.metric(&format!("operators_{key}_sites"), n);
        *by_level.entry(level).or_default() += n;
    }
    let negations = total
        .by_kind
        .get(&Kind::Expression(E_NEGATE))
        .copied()
        .unwrap_or(0);
    for level in ["compare", "or", "xor", "and", "shift", "add", "mul"] {
        c.metric(
            &format!("operators_level_{level}_sites"),
            by_level.get(level).copied().unwrap_or(0),
        );
    }
    c.metric("operators_level_unary_sites", negations);
    c.metric("operators_binary_sites", by_level.values().sum::<usize>());
    let mut chained: Vec<String> = Vec::new();
    for p in programs {
        chained.extend(chained_comparisons(p, &expression_roots(p)));
    }
    c.metric("operators_chained_comparisons", chained.len());
    for e in &chained {
        c.note(format!("  CHAINED COMPARISON: {e}"));
    }

    // ── 25. Sign purity ─────────────────────────────────────────────────
    let mut signs = Signs::default();
    for p in programs {
        let s = sign_purity(p);
        signs.danda += s.danda;
        signs.double_danda += s.double_danda;
        signs.member_mark += s.member_mark;
        signs.label_mark += s.label_mark;
        signs.avagraha += s.avagraha;
        signs.om += s.om;
        signs.comment_lines += s.comment_lines;
        signs.nonrepertoire.extend(s.nonrepertoire);
    }
    c.metric("signs_danda_sites", signs.danda);
    c.metric("signs_double_danda_sites", signs.double_danda);
    c.metric("signs_member_mark_sites", signs.member_mark);
    c.metric("signs_label_mark_sites", signs.label_mark);
    c.metric("signs_avagraha_sites", signs.avagraha);
    c.metric("signs_om_sites", signs.om);
    c.metric("signs_comment_lines", signs.comment_lines);
    c.metric("signs_nonrepertoire", signs.nonrepertoire.len());
    for e in &signs.nonrepertoire {
        c.note(format!("  NON-REPERTOIRE SIGN: {e}"));
    }
    c
}

/// Render the human table of a walk: kind → place → count.
fn kind_table(sites: &BTreeMap<Kind, usize>) -> String {
    let mut out = String::new();
    for (k, n) in sites {
        let p = place(*k).map_or_else(|e| format!("{e:?}"), Place::describe);
        let _ = writeln!(out, "  {n:>6}  {k:?}  →  {p}");
    }
    out
}

/// Every source of the corpus, parsed — ONCE PER PROCESS, however many tests
/// in this binary ask for it (`W-261`).
///
/// THE COST IS THE INTERPRETED LEX AND PARSE, NOT THE LOAD, and the row this
/// came from assumed the opposite. Measured with an in-test timer over four
/// sources: `Interpreter::load` of the five front-end modules is 395-444ms,
/// while `artha.t1` lexes in 3821ms and parses in 5700ms and `encode.t1` in
/// 6300ms and 10565ms. **Load is 5% of a pass.** So sharing the interpreter
/// IMAGE saves ~8%; what is worth sharing is the RESULT, and the six gated
/// ratchets re-derive it from the same nineteen sources.
///
/// WHY SHARING THIS IS SAFE WHERE SHARING AN INTERPRETER IS NOT, which is the
/// whole design and not an optimisation detail: a [`Program`] is arenas COPIED
/// OUT — `Vec<Tok>`, `Vec<Stmt>`, ints and `String`s, no `Rc` into the
/// interpreter's arenas and no live arena at all. The recorded hazard for a
/// shared image is that `व्याकर` does not truncate its arenas between programs,
/// so a `दैर्घ्य`-bounded walk reads the previous program's tail; that cannot
/// arise here because there is nothing left to walk. The design was chosen so
/// the hazard is impossible rather than survivable, and
/// `a_shared_corpus_cannot_carry_one_source_into_another` guards it against a
/// later "simplification" back to a shared mutable interpreter.
///
/// The clone is a memcpy of plain data against a ~90s pass, so it is free at
/// this scale and keeps every caller's signature and ownership unchanged.
fn parse_corpus() -> Vec<Program> {
    static CORPUS: OnceLock<Vec<Program>> = OnceLock::new();
    CORPUS.get_or_init(parse_corpus_uncached).clone()
}

/// The pass itself. Called at most once per process, through [`parse_corpus`].
///
/// It reports its own cost, because `W-261`'s saving must be read from an
/// in-test timer and never from wall clock — this machine cannot resolve a
/// difference under ~200s by the clock. One `METRIC paradigm_corpus_pass_ms`
/// line per process is the measurement: two ratchets in one invocation print
/// ONE, and the same two run separately print one each.
fn parse_corpus_uncached() -> Vec<Program> {
    let started = std::time::Instant::now();
    let out = parse_corpus_pass();
    println!(
        "METRIC paradigm_corpus_pass_ms {}",
        started.elapsed().as_millis()
    );
    out
}

fn parse_corpus_pass() -> Vec<Program> {
    corpus_names()
        .iter()
        .map(|n| {
            let src = std::fs::read_to_string(source_dir().join(n)).expect("a readable source");
            parse_source(n, &src)
        })
        .collect()
}

/// THE CENSUS. Parses the fifteen sources through `व्याकर`, places every site,
/// and prints `METRIC paradigm_*` lines.
#[test]
#[ignore = "measurement"]
fn measure_corpus_paradigm() {
    let programs = parse_corpus();
    let c = census(&programs);
    for line in &c.notes {
        eprintln!("{line}");
    }
    let mut kinds: BTreeMap<Kind, usize> = BTreeMap::new();
    for p in &programs {
        for (k, v) in walk(p).by_kind {
            *kinds.entry(k).or_default() += v;
        }
    }
    eprint!("{}", kind_table(&kinds));
    for (k, v) in &c.metrics {
        println!("METRIC {k} {v}");
    }
}

// ─────────────────────────────────────────────────────────────────────────
// W-207 — the म loop: statistics 11–15 of plan §2.4, over every यावत् and यदि.
//
// MEASURED ON THE AST. Plan 11 asks for the IR back-edge; at this base
// `ir.t1` lowers neither `यावत्` nor `यदि` (no arm names either kind), and
// W-198's `CondBranch` lowering sits on `origin/agent/w198`, unmerged. So 11
// is taken where it can be: a loop's condition and body are INSIDE its own
// arena span by construction (`आदिसूचकाङ्क` … the node), which is Rule X3 at
// the level the parser guarantees it, and `paradigm_loop_backedge_instrument`
// says `ast` so the day the IR measure lands the two are not confused.
// ─────────────────────────────────────────────────────────────────────────

/// One `यावत्` or `यदि`: where it is and what it holds.
#[derive(Clone, Debug)]
struct LoopSite {
    /// Arena index of the statement.
    index: usize,
    kind: i128,
    line: i128,
    cond: i128,
    /// The body block's index, ० when the parser recorded none.
    body: i128,
    /// The statement's own subtree start.
    first: usize,
    /// The routine range `(first, body index)` this site sits in, and the
    /// routine's declaration.
    routine: Option<(usize, usize, Decl)>,
}

/// Every यावत् and यदि of a program, with its routine.
fn loop_sites(p: &Program) -> Vec<LoopSite> {
    let routines: Vec<(usize, usize, Decl)> = p
        .decls
        .iter()
        .skip(1)
        .filter(|d| d.kind == D_ROUTINE)
        .filter_map(|d| {
            let body = p.stmt(d.body)?;
            Some((
                usize::try_from(body.first).ok()?,
                usize::try_from(d.body).ok()?,
                *d,
            ))
        })
        .collect();
    p.stmts
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, st)| st.kind == S_WHILE || st.kind == S_IF)
        .map(|(i, st)| LoopSite {
            index: i,
            kind: st.kind,
            line: p.expr_line(st.left),
            cond: st.left,
            body: st.right,
            first: usize::try_from(st.first).unwrap_or(i),
            routine: routines
                .iter()
                .find(|(lo, hi, _)| (*lo..=*hi).contains(&i))
                .cloned(),
        })
        .collect()
}

/// The names an expression reads, as token text, in tree order.
fn names_read(p: &Program, i: i128, out: &mut Vec<String>) {
    let Some(e) = p.expr(i) else { return };
    match e.kind {
        E_NAME => {
            if let Some(t) = p.tok(e.value) {
                out.push(p.text(t).to_string());
            }
        }
        E_FIELD => names_read(p, e.left, out),
        E_BINARY | E_INDEX | E_CALL => {
            names_read(p, e.left, out);
            names_read(p, e.right, out);
        }
        E_GROUP | E_NEGATE => names_read(p, e.left, out),
        E_SLICE => {
            names_read(p, e.left, out);
            names_read(p, e.right, out);
        }
        _ => {}
    }
}

/// The operators an expression holds, by kind, in tree order.
fn operators_in(p: &Program, i: i128, out: &mut Vec<i128>) {
    let Some(e) = p.expr(i) else { return };
    match e.kind {
        E_BINARY => {
            out.push(e.op);
            operators_in(p, e.left, out);
            operators_in(p, e.right, out);
        }
        E_INDEX | E_CALL => {
            operators_in(p, e.left, out);
            operators_in(p, e.right, out);
        }
        E_GROUP | E_NEGATE | E_FIELD => operators_in(p, e.left, out),
        E_SLICE => {
            operators_in(p, e.left, out);
            operators_in(p, e.right, out);
        }
        _ => {}
    }
}

/// The name an assignment target writes: `x`, `x ॱ f`, `x अङ्कः i अन्तः` all
/// write `x` — the arena or record IS the place, so a field or slot store is
/// a store to the name that holds it.
fn target_name(p: &Program, i: i128) -> Option<String> {
    let mut i = i;
    for _ in 0..64 {
        let e = p.expr(i)?;
        match e.kind {
            E_NAME => return p.tok(e.value).map(|t| p.text(t).to_string()),
            E_FIELD | E_INDEX | E_GROUP => i = e.left,
            _ => return None,
        }
    }
    None
}

/// Whether statement `c` is in scope at statement `at` inside one routine:
/// `c` comes earlier, and no block that holds `c` fails to hold `at`. A
/// `चरः` inside an earlier sibling `यदि` is NOT in scope; one in an enclosing
/// block is.
fn in_scope_before(p: &Program, c: usize, at: usize, routine: (usize, usize)) -> bool {
    if c >= at || c < routine.0 {
        return false;
    }
    !p.stmts
        .iter()
        .enumerate()
        .skip(routine.0)
        .take(routine.1 - routine.0 + 1)
        .filter(|(_, st)| st.kind == S_BLOCK)
        .any(|(k, st)| {
            let lo = usize::try_from(st.first).unwrap_or(k);
            (lo..=k).contains(&c) && !(lo..=k).contains(&at)
        })
}

/// How one यावत् is entered (13) and stepped (14).
#[derive(Debug, PartialEq, Eq)]
enum Entry {
    /// A name the condition reads is a `चरः` in scope, declared before the loop.
    LocalBinding,
    /// Only a parameter of the routine.
    Parameter,
    /// Only a module-level `चरः` of this file.
    ModuleBinding,
    /// Nothing the condition reads is declared in this file before the loop —
    /// a qualified name, a field, a call, or a literal-only condition.
    Other,
}

#[derive(Debug, PartialEq, Eq)]
enum Step {
    /// The body assigns, directly or through a field or slot, a name the
    /// condition reads.
    Present,
    /// No such store, and the body returns — an exit by `प्रत्यागमनम्`.
    AbsentExitByReturn,
    /// No such store in the body, but the body calls a routine of the same
    /// file whose own body assigns a name the condition reads — the step is
    /// ONE CALL LEVEL down. `W-230`: `parse.t1`'s declaration loop reads
    /// `पठनस्थान`, which `मेलनम्` and `अग्रिमम्` advance.
    PresentThroughCall,
    /// No such store, no return, and no call that stores: the census cannot
    /// tell this from an infinite loop.
    Absent,
}

/// The callee names of every call under expression `i`, bare or qualified.
fn callee_names(p: &Program, i: i128, out: &mut Vec<String>) {
    let Some(e) = p.expr(i) else { return };
    match e.kind {
        E_CALL => {
            let mut cur = i;
            while let Some(x) = p.expr(cur).filter(|x| x.kind == E_CALL) {
                callee_names(p, x.right, out);
                cur = x.left;
            }
            match p.expr(cur) {
                Some(c) if c.kind == E_NAME => {
                    if let Some(t) = p.tok(c.value) {
                        out.push(p.text(t).to_string());
                    }
                }
                _ => callee_names(p, cur, out),
            }
        }
        // A bare routine name in value position IS a zero-argument call —
        // `व्याकर` pushes one call node per argument, so `अग्रे ।` has none.
        // `routine_stores` keeps only the names that are routines of the file.
        E_NAME => {
            if let Some(t) = p.tok(e.value) {
                out.push(p.text(t).to_string());
            }
        }
        E_BINARY | E_INDEX | E_SLICE => {
            callee_names(p, e.left, out);
            callee_names(p, e.right, out);
        }
        E_GROUP | E_NEGATE | E_FIELD => callee_names(p, e.left, out),
        _ => {}
    }
}

/// Whether a routine of this program named `callee` assigns, anywhere in its
/// own body, one of `names`.
fn routine_stores(p: &Program, callee: &str, names: &[String]) -> bool {
    p.decls
        .iter()
        .skip(1)
        .filter(|d| d.kind == D_ROUTINE)
        .filter(|d| p.tok(d.name).is_some_and(|t| p.text(t) == callee))
        .any(|d| {
            let Some(body) = p.stmt(d.body) else {
                return false;
            };
            let hi = usize::try_from(d.body).unwrap_or(0);
            let lo = usize::try_from(body.first).unwrap_or(hi);
            p.stmts[lo..=hi].iter().any(|st| {
                st.kind == S_ASSIGN && target_name(p, st.left).is_some_and(|t| names.contains(&t))
            })
        })
}

/// What the loop walk found for one program.
#[derive(Default)]
struct Loops {
    yavat: usize,
    yadi: usize,
    yadi_with_else: usize,
    /// Operators in यावत् conditions, then in यदि conditions, by kind.
    yavat_ops: BTreeMap<i128, usize>,
    yadi_ops: BTreeMap<i128, usize>,
    yavat_bare: usize,
    yadi_bare: usize,
    entries: BTreeMap<&'static str, usize>,
    steps: BTreeMap<&'static str, usize>,
    /// `file:line names…` for every loop with no step.
    no_step: Vec<String>,
    /// `file:line callee` for every loop whose step is one call level down.
    step_through_call: Vec<String>,
    /// `file:line name` for every loop entered other than through a local binding.
    other_entry: Vec<String>,
    /// Nesting depth (enclosing यावत् count) per यावत्.
    depths: Vec<usize>,
    /// Routines that hold at least one यावत्, and the most in one routine.
    routines_with_loops: usize,
    max_loops_in_a_routine: usize,
    /// Plan 11 on the AST: यावत् whose condition and body lie in its own span.
    contained: usize,
}

fn loops(p: &Program) -> Loops {
    let mut l = Loops::default();
    let sites = loop_sites(p);
    let yavats: Vec<&LoopSite> = sites.iter().filter(|s| s.kind == S_WHILE).collect();
    let mut per_routine: BTreeMap<usize, usize> = BTreeMap::new();

    for s in &sites {
        let mut ops = Vec::new();
        operators_in(p, s.cond, &mut ops);
        let (count, bucket, bare) = if s.kind == S_WHILE {
            (&mut l.yavat, &mut l.yavat_ops, &mut l.yavat_bare)
        } else {
            (&mut l.yadi, &mut l.yadi_ops, &mut l.yadi_bare)
        };
        *count += 1;
        if ops.is_empty() {
            *bare += 1;
        }
        for op in ops {
            *bucket.entry(op).or_default() += 1;
        }
        if s.kind == S_IF {
            // The else block, when the parser read one, is pushed right
            // before the यदि node (parse.t1:966-969), so the node's
            // predecessor being a block OTHER than the then-block means an
            // `अन्यथा` was there — even though the node itself dropped it.
            let then = usize::try_from(s.body).unwrap_or(0);
            if s.index >= 2
                && then + 1 != s.index
                && p.stmts[s.index - 1].kind == S_BLOCK
                && then != 0
            {
                l.yadi_with_else += 1;
            }
        }
    }

    for s in &yavats {
        // 11. Containment on the AST.
        let body = usize::try_from(s.body).unwrap_or(0);
        if body != 0 && (s.first..s.index).contains(&body) && s.cond != 0 {
            l.contained += 1;
        }
        // 15. Nesting.
        let depth = yavats
            .iter()
            .filter(|o| o.index != s.index && (o.first..o.index).contains(&s.index))
            .count();
        l.depths.push(depth);
        if let Some((lo, _, _)) = s.routine {
            *per_routine.entry(lo).or_default() += 1;
        }

        // 13. Entry.
        let mut names = Vec::new();
        names_read(p, s.cond, &mut names);
        names.dedup();
        let routine = s.routine.as_ref().map(|(lo, hi, d)| (*lo, *hi, *d));
        let is_local = |name: &str| -> bool {
            let Some((lo, hi, _)) = routine else {
                return false;
            };
            p.stmts
                .iter()
                .enumerate()
                .skip(lo)
                .take(hi - lo + 1)
                .filter(|(_, st)| st.kind == S_BINDING)
                .any(|(c, st)| {
                    p.tok(st.left).is_some_and(|t| p.text(t) == name)
                        && in_scope_before(p, c, s.index, (lo, hi))
                })
        };
        let is_param = |name: &str| -> bool {
            let Some((_, _, d)) = routine else {
                return false;
            };
            d.p0 > 0
                && (d.p0..=d.p1).any(|i| {
                    p.params
                        .get(usize::try_from(i).unwrap_or(0))
                        .and_then(|(n, _)| p.tok(*n))
                        .is_some_and(|t| p.text(t) == name)
                })
        };
        let is_module = |name: &str| -> bool {
            p.decls
                .iter()
                .skip(1)
                .filter(|d| d.kind == D_BINDING)
                .any(|d| p.tok(d.name).is_some_and(|t| p.text(t) == name))
        };
        let entry = if names.iter().any(|n| is_local(n)) {
            Entry::LocalBinding
        } else if names.iter().any(|n| is_param(n)) {
            Entry::Parameter
        } else if names.iter().any(|n| is_module(n)) {
            Entry::ModuleBinding
        } else {
            Entry::Other
        };
        let key = match entry {
            Entry::LocalBinding => "local_binding",
            Entry::Parameter => "parameter",
            Entry::ModuleBinding => "module_binding",
            Entry::Other => "other",
        };
        *l.entries.entry(key).or_default() += 1;
        if entry != Entry::LocalBinding {
            l.other_entry.push(format!(
                "{}:{} {key} — reads {}",
                p.name,
                s.line,
                names.join(" ")
            ));
        }

        // 14. Step.
        let (mut stored, mut returns) = (false, false);
        if body != 0 {
            let lo = usize::try_from(p.stmts[body].first).unwrap_or(body);
            for st in &p.stmts[lo..=body] {
                match st.kind {
                    S_ASSIGN => {
                        if target_name(p, st.left).is_some_and(|t| names.contains(&t)) {
                            stored = true;
                        }
                    }
                    S_RETURN => returns = true,
                    _ => {}
                }
            }
        }
        // 14b. ONE CALL LEVEL (`W-230`). Only asked when the body itself
        // neither stores nor returns: the callees of every call in the body,
        // and whether one of them — a routine of this file — assigns a name
        // the condition reads.
        let mut through: Option<String> = None;
        if !stored && !returns && body != 0 {
            let lo = usize::try_from(p.stmts[body].first).unwrap_or(body);
            let mut callees = Vec::new();
            for st in &p.stmts[lo..=body] {
                match st.kind {
                    S_EXPRESSION | S_RETURN | S_IF | S_WHILE => {
                        callee_names(p, st.left, &mut callees)
                    }
                    S_BINDING => callee_names(p, st.right, &mut callees),
                    S_ASSIGN => {
                        callee_names(p, st.left, &mut callees);
                        callee_names(p, st.right, &mut callees);
                    }
                    _ => {}
                }
            }
            callees.dedup();
            through = callees.into_iter().find(|c| routine_stores(p, c, &names));
        }
        let step = if stored {
            Step::Present
        } else if returns {
            Step::AbsentExitByReturn
        } else if through.is_some() {
            Step::PresentThroughCall
        } else {
            Step::Absent
        };
        let key = match step {
            Step::Present => "present",
            Step::AbsentExitByReturn => "absent_exit_by_return",
            Step::PresentThroughCall => "present_through_call",
            Step::Absent => "absent",
        };
        *l.steps.entry(key).or_default() += 1;
        if let Some(callee) = &through {
            l.step_through_call
                .push(format!("{}:{} {callee}", p.name, s.line));
        }
        if step != Step::Present && step != Step::PresentThroughCall {
            l.no_step.push(format!(
                "{}:{} {key} — condition reads {}",
                p.name,
                s.line,
                names.join(" ")
            ));
        }
    }
    l.routines_with_loops = per_routine.len();
    l.max_loops_in_a_routine = per_routine.values().copied().max().unwrap_or(0);
    l
}

/// `W-230`: a loop that steps through a CALL, and the refused case beside it.
const LOOPS_THROUGH_CALLS: &str = "मण्डलम् परीक्षा ॥
चरः स्थानम् ॱॱ न६४ भवति ० ।
चरः सीमा ॱॱ न६४ भवति १० ।
वृत्तिः अग्रे ददाति न६४ आदि
    स्थानम् भवति स्थानम् योगः १ ।
    प्रत्यागमनम् स्थानम् ।
इति
वृत्तिः पश्य ददाति न६४ आदि
    प्रत्यागमनम् स्थानम् ।
इति
वृत्तिः क ददाति न६४ आदि
    यावत् स्थानम् न्यूनम् सीमा आदि
        चरः अ ॱॱ न६४ भवति अग्रे ।
    इति
    यावत् स्थानम् न्यूनम् सीमा आदि
        चरः आ ॱॱ न६४ भवति पश्य ।
    इति
    यावत् स्थानम् न्यूनम् सीमा आदि
        चरः इ ॱॱ न६४ भवति १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";

/// The one-call-level step, on a program read by eye: the first loop calls
/// `अग्रे`, which assigns `स्थानम्` — a step through a call. REFUSED: the
/// second calls `पश्य`, which stores nothing, and the third calls nothing,
/// stores nothing and returns nothing — both are reported, and the census
/// still cannot tell either from an infinite loop.
#[test]
fn a_loop_that_steps_one_call_down_is_seen_and_one_that_does_not_is_reported() {
    let p = parse_source("through.t1", LOOPS_THROUGH_CALLS);
    assert_eq!(p.parse_errors, 0, "the program parses");
    let l = loops(&p);
    assert_eq!(l.yavat, 3);
    assert_eq!(l.steps.get("present_through_call"), Some(&1));
    assert_eq!(l.steps.get("absent"), Some(&2));
    assert_eq!(l.steps.get("present"), None);
    assert_eq!(l.step_through_call, vec!["through.t1:12 अग्रे".to_string()]);
    assert_eq!(l.no_step.len(), 2, "{:?}", l.no_step);
    assert!(
        l.no_step[0].starts_with("through.t1:15 absent"),
        "{:?}",
        l.no_step
    );
    assert!(
        l.no_step[1].starts_with("through.t1:18 absent"),
        "{:?}",
        l.no_step
    );
}

/// `W-261`'s ACCEPTANCE, and it guards a DESIGN rather than a defect: the
/// shared corpus carries nothing from one source into another.
///
/// Not `#[ignore]`d — it parses three small fixtures of its own and never
/// touches the corpus, so it costs a couple of seconds and runs in every
/// `cargo test`. The corpus-side half of the acceptance is
/// [`the_shared_corpus_is_one_parse`], which IS `#[ignore]`d because it needs
/// the pass — and costs nothing when the gate runs it beside the ratchets that
/// have already warmed the cache.
///
/// WHY IT EXISTS WHEN THE HAZARD IS ALREADY IMPOSSIBLE. `व्याकर` does not
/// truncate its arenas between programs, so any reader that walks a shared
/// interpreter to `दैर्घ्य` rather than to the CURSOR reads the previous
/// program's tail. [`parse_corpus`] cannot suffer that, because what it shares
/// is arenas copied out — plain `Vec`s with no `Rc` into the interpreter and no
/// live arena to over-walk. This test pins that property so a later change
/// cannot quietly trade it away: the obvious "simplification" of the cache is
/// to share one loaded [`Interpreter`] and re-run the two calls per source,
/// which saves 5% (the load) and reintroduces the leak at full strength.
///
/// It plants the leak the only way a copied-out design can suffer one — a long
/// source followed by a short one, where a stale tail would show as the second
/// program carrying tokens or statements it does not have.
#[test]
fn a_shared_corpus_cannot_carry_one_source_into_another() {
    // Long first, then short. If anything of the first survived into the
    // second — a `दैर्घ्य`-bounded walk over an untruncated arena — the short
    // program would answer with the long one's tail.
    let long = "मण्डलम् दीर्घः ॥\nवृत्तिः एकः ददाति न६४ आदि\n    चरः क ॱॱ न६४ भवति १ ।\n    चरः ख ॱॱ न६४ भवति २ ।\n    चरः ग ॱॱ न६४ भवति ३ ।\n    प्रत्यागमनम् क योगः ख योगः ग ।\nइति\n";
    let short = "मण्डलम् ह्रस्वः ॥\nवृत्तिः द्वौ ददाति न६४ आदि\n    प्रत्यागमनम् ७ ।\nइति\n";

    // The short source read cold, by a reader that has seen nothing else.
    let alone = parse_source("short-alone", short);
    // The same source read after a longer one, in the order that would expose a
    // stale tail: the long program's arenas are the ones that would still be
    // there to over-walk.
    let _ = parse_source("long", long);
    let after_long = parse_source("short-after-long", short);

    assert_eq!(
        after_long.stmts.len(),
        alone.stmts.len(),
        "the short source read a different number of statements depending on \
         what was parsed before it — a source is leaking into the next"
    );
    assert_eq!(
        after_long.toks.len(),
        alone.toks.len(),
        "the short source read a different number of tokens depending on what \
         was parsed before it — a source is leaking into the next"
    );
    assert_eq!(after_long.parse_errors, 0, "the short fixture parses");
    // Taken from the instrument, not computed: `३` is what `व्याकर` reads for
    // this fixture — slot ० and its own statements. It is pinned so that a
    // wrong answer above is read as the fixture drifting rather than as the
    // property failing, and the two are not the same finding.
    assert_eq!(
        after_long.stmts.len(),
        3,
        "the short fixture's own statement count changed; re-read the fixture \
         before reading this as a leak"
    );
}

/// `W-261`'s acceptance, corpus side: every ratchet in this binary is handed
/// THE SAME parse, not one parse each.
///
/// A cache that re-derived the corpus per caller would pass every ratchet and
/// silently mean the ratchets no longer agree with each other about what they
/// are pinning — the failure would surface as two ratchets disagreeing months
/// later, which is the shape of defect this row exists to avoid.
///
/// `#[ignore]`d because it needs the pass. It is FREE when run in the same
/// invocation as the ratchets, which is how `tools/check-t1-ratchets.sh` runs
/// it, and costs one pass if run alone.
#[test]
#[ignore = "ratchet: shares the corpus pass with this binary's ratchets; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn the_shared_corpus_is_one_parse() {
    let a = parse_corpus();
    let b = parse_corpus();
    assert_eq!(a.len(), b.len(), "the shared corpus is one corpus");
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.name, y.name, "the shared corpus is in one order");
        assert_eq!(
            (x.toks.len(), x.stmts.len(), x.exprs.len(), x.decls.len()),
            (y.toks.len(), y.stmts.len(), y.exprs.len(), y.decls.len()),
            "{}: the two handings of the shared corpus disagree",
            x.name
        );
    }
}

/// THE RATCHET (`W-230`), over the corpus, at ZERO: no `यावत्` is without a
/// step in its own body, one call down, or an exit by return. `W-207` found
/// one (`parse.t1`'s declaration loop) and the one-call-level reading above
/// sees its step in `मेलनम्`/`अग्रिमम्`. One interpreter pass over the corpus;
/// `tools/check-t1-ratchets.sh` runs it on the hourly deep gate.
#[test]
#[ignore = "ratchet: one interpreter pass over the corpus; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn no_yavat_in_the_corpus_is_without_a_step_or_an_exit() {
    let programs = parse_corpus();
    let mut absent = Vec::new();
    let mut through = Vec::new();
    let mut by_return = 0;
    for p in &programs {
        let l = loops(p);
        through.extend(l.step_through_call);
        by_return += l.steps.get("absent_exit_by_return").copied().unwrap_or(0);
        absent.extend(l.no_step.into_iter().filter(|s| s.contains(" absent —")));
    }
    println!("METRIC paradigm_loops_step_through_call {}", through.len());
    for t in &through {
        println!("  step through call: {t}");
    }
    println!("METRIC paradigm_loops_exit_by_return {by_return}");
    println!("METRIC paradigm_loops_no_step {}", absent.len());
    // `W-240`: parse.t1's declaration loop no longer reads as a step through
    // `मेलनम्`. Its last arm — the skip-a-token else W-230 measured this loop
    // by — is a REFUSAL now (`प्रत्यागमनम् दोषयोजनम् …`), so the instrument,
    // which reads a return before it looks one call down, files the loop
    // under `absent_exit_by_return`. The loop still advances through the
    // readers' `मेलनम्`; what changed is that the way OUT of it on a token
    // nobody reads is a refusal, not a step. Pinned as the instrument reads
    // it, with the reason.
    let parse_by_return = programs
        .iter()
        .flat_map(|p| loops(p).no_step)
        .filter(|s| s.starts_with("parse.t1:") && s.contains("absent_exit_by_return"))
        .count();
    assert!(
        parse_by_return >= 1,
        "parse.t1's declaration loop exits by return since W-240 (its else arm refuses); \
         the instrument lists none: through={through:?}"
    );
    assert_eq!(
        absent,
        Vec::<String>::new(),
        "a यावत् with no step in its body, none one call down, and no return"
    );
}

/// THE RATCHET (`W-229`), over the corpus, at ZERO: no comparison is chained.
/// `compare_expr` in the frozen grammar is non-associative and REFUSES
/// `a op b op c`; `व्याकर`'s flat expression reader accepts it and computes
/// `a < (b == false)`, right only because `बूल` is one octet wide. `W-206`
/// found nine such spellings of `>=` from before ADR-0037 gave the corpus
/// `बृहत्समम्`; fourteen stood on 2026-09-04 (W-202 and W-215 had copied the
/// idiom into artha.t1's else-walk and unparse.t1) and W-229 rewrote every
/// one as `x बृहत्समम् y`. REFUSED, in the same pass: a synthetic chain is
/// SEEN — counted at its line by the instrument while `व्याकर` is flat, and
/// refused as a parse error once it has a ladder; the ratchet holds through
/// that transition, and a chain that is neither counted nor refused is the
/// instrument going blind. One interpreter pass over the corpus;
/// `tools/check-t1-ratchets.sh` runs it on the hourly deep gate.
#[test]
#[ignore = "ratchet: one interpreter pass over the corpus; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn no_comparison_in_the_corpus_is_chained_and_a_synthetic_chain_is_seen() {
    let programs = parse_corpus();
    let c = census(&programs);
    let chained: Vec<&String> = c
        .notes
        .iter()
        .filter(|n| n.contains("CHAINED COMPARISON"))
        .collect();
    println!(
        "METRIC paradigm_operators_chained_comparisons {}",
        c.get("operators_chained_comparisons").unwrap_or("?")
    );
    for n in &chained {
        println!("  {n}");
    }
    assert_eq!(
        c.get("operators_chained_comparisons"),
        Some("0"),
        "a chained comparison in the corpus — spell `>=` as `बृहत्समम्`: {chained:?}"
    );

    // THE REFUSED CASE. `a न्यूनम् b समम् असत्यम्` on one ungrouped spine.
    let chain = "मण्डलम् परीक्षा ॥
वृत्तिः क आदाय अ ॱॱ न६४ ददाति बूल आदि
    प्रत्यागमनम् अ न्यूनम् १ समम् असत्यम् ।
इति
";
    let p = parse_source("chain.t1", chain);
    let c = census(std::slice::from_ref(&p));
    let counted = c.get("operators_chained_comparisons") == Some("1");
    let refused = p.parse_errors > 0;
    assert!(
        counted || refused,
        "a chain is counted while व्याकर is flat, or refused once it has a ladder; \
         this one was neither (counted {counted}, parse errors {})",
        p.parse_errors
    );
    if counted {
        assert!(
            c.notes.iter().any(|n| n.contains("chain.t1:3")),
            "the chain is reported at its line: {:?}",
            c.notes
        );
    }
}

/// THE LOOP CENSUS (`W-207`). The same fifteen parses as
/// [`measure_corpus_paradigm`], walked for the म loop.
#[test]
#[ignore = "measurement"]
fn measure_corpus_loops() {
    let programs = parse_corpus();
    let c = loop_census(&programs);
    for line in &c.notes {
        eprintln!("{line}");
    }
    for (k, v) in &c.metrics {
        println!("METRIC {k} {v}");
    }
}

fn loop_census(programs: &[Program]) -> Census {
    let mut c = Census::default();
    let mut t = Loops::default();
    for p in programs {
        let l = loops(p);
        t.yavat += l.yavat;
        t.yadi += l.yadi;
        t.yadi_with_else += l.yadi_with_else;
        t.yavat_bare += l.yavat_bare;
        t.yadi_bare += l.yadi_bare;
        t.contained += l.contained;
        t.routines_with_loops += l.routines_with_loops;
        t.max_loops_in_a_routine = t.max_loops_in_a_routine.max(l.max_loops_in_a_routine);
        for (k, v) in l.yavat_ops {
            *t.yavat_ops.entry(k).or_default() += v;
        }
        for (k, v) in l.yadi_ops {
            *t.yadi_ops.entry(k).or_default() += v;
        }
        for (k, v) in l.entries {
            *t.entries.entry(k).or_default() += v;
        }
        for (k, v) in l.steps {
            *t.steps.entry(k).or_default() += v;
        }
        t.no_step.extend(l.no_step);
        t.other_entry.extend(l.other_entry);
        t.depths.extend(l.depths);
        c.note(format!(
            "  {:24} {:>4} यावत् {:>5} यदि ({} with अन्यथा)",
            p.name, l.yavat, l.yadi, l.yadi_with_else
        ));
    }
    c.metric("loop_yavat_sites", t.yavat);
    c.metric("loop_yadi_sites", t.yadi);
    c.metric("loop_yadi_with_else", t.yadi_with_else);

    // 11.
    c.metric("loop_backedge_instrument", "ast");
    c.metric("loop_backedge_contained", t.contained);
    c.metric("loop_backedge_rate", rate(t.contained, t.yavat));

    // 12.
    for (which, ops, bare) in [
        ("yavat", &t.yavat_ops, t.yavat_bare),
        ("yadi", &t.yadi_ops, t.yadi_bare),
    ] {
        let mut by_level: BTreeMap<&str, usize> = BTreeMap::new();
        for (op, n) in ops {
            if let Some(level) = ladder(*op) {
                *by_level.entry(level).or_default() += n;
            }
        }
        for level in ["compare", "or", "xor", "and", "shift", "add", "mul"] {
            c.metric(
                &format!("loop_condition_{which}_level_{level}_sites"),
                by_level.get(level).copied().unwrap_or(0),
            );
        }
        c.metric(&format!("loop_condition_{which}_bare"), bare);
    }
    for (op, key, _) in &OPERATORS {
        c.metric(
            &format!("loop_condition_yavat_{key}_sites"),
            t.yavat_ops.get(op).copied().unwrap_or(0),
        );
    }

    // 13.
    for key in ["local_binding", "parameter", "module_binding", "other"] {
        c.metric(
            &format!("loop_entry_{key}"),
            t.entries.get(key).copied().unwrap_or(0),
        );
    }
    c.metric(
        "loop_entry_local_binding_rate",
        rate(
            t.entries.get("local_binding").copied().unwrap_or(0),
            t.yavat,
        ),
    );
    for e in &t.other_entry {
        c.note(format!("  ENTRY: {e}"));
    }

    // 14.
    for key in ["present", "absent_exit_by_return", "absent"] {
        c.metric(
            &format!("loop_step_{key}"),
            t.steps.get(key).copied().unwrap_or(0),
        );
    }
    c.metric(
        "loop_step_present_rate",
        rate(t.steps.get("present").copied().unwrap_or(0), t.yavat),
    );
    for e in &t.no_step {
        c.note(format!("  NO STEP: {e}"));
    }

    // 15.
    let max_depth = t.depths.iter().copied().max().unwrap_or(0);
    c.metric("loop_nesting_max_depth", max_depth);
    for d in 0..=max_depth {
        c.metric(
            &format!("loop_nesting_depth_{d}_sites"),
            t.depths.iter().filter(|x| **x == d).count(),
        );
    }
    c.metric("loop_routines_with_loops", t.routines_with_loops);
    c.metric("loop_max_loops_in_a_routine", t.max_loops_in_a_routine);
    c
}

/// A program with every loop shape the walk classifies.
const LOOPS: &str = "मण्डलम् परीक्षा ॥
चरः सीमा ॱॱ न६४ भवति १० ।
वृत्तिः क आदाय अ ॱॱ न६४ ददाति न६४ आदि
    चरः ख ॱॱ न६४ भवति ० ।
    यावत् ख न्यूनम् अ आदि
        यावत् ख न्यूनम् सीमा आदि
            ख भवति आरभ्य ख योगः १ समाप्तम् ।
        इति
        यदि ख समम् ५ आदि
            प्रत्यागमनम् ख ।
        इति अन्यथा आदि
            ख भवति ख योगः २ ।
        इति
    इति
    यावत् अ अधिकम् ० आदि
        प्रत्यागमनम् अ ।
    इति
    यावत् सीमा अधिकम् ० आदि
        ग आरभ्य ० समाप्तम् ।
    इति
    प्रत्यागमनम् ० ।
इति
";

/// The loop walk on a program where every answer can be read by eye.
#[test]
fn the_loop_walk_classifies_every_shape_it_names() {
    let p = parse_source("loops.t1", LOOPS);
    assert_eq!(p.parse_errors, 0, "the loop program parses");
    let l = loops(&p);
    assert_eq!((l.yavat, l.yadi, l.yadi_with_else), (4, 1, 1));
    // 11. Every यावत् holds its condition and body.
    assert_eq!(l.contained, 4);
    // 12. Four comparisons in यावत् conditions, one in the यदि.
    assert_eq!(l.yavat_ops.values().sum::<usize>(), 4);
    assert_eq!(l.yadi_ops.get(&5), Some(&1), "one समम्");
    assert_eq!(l.yavat_bare, 0);
    // 13. Two loops read `ख` (a local चरः); one reads `अ` (a parameter) —
    // and `ख न्यूनम् अ` counts as local because ख is; one reads `सीमा`.
    assert_eq!(l.entries.get("local_binding"), Some(&2));
    assert_eq!(l.entries.get("parameter"), Some(&1));
    assert_eq!(l.entries.get("module_binding"), Some(&1));
    // 14. The outer loop steps ख through its else branch; the inner steps ख
    // through a group; the `अ` loop exits by return; the `सीमा` loop calls.
    assert_eq!(l.steps.get("present"), Some(&2));
    assert_eq!(l.steps.get("absent_exit_by_return"), Some(&1));
    assert_eq!(l.steps.get("absent"), Some(&1));
    assert_eq!(l.no_step.len(), 2);
    assert!(
        l.no_step[0].starts_with("loops.t1:15 absent_exit_by_return"),
        "{:?}",
        l.no_step
    );
    assert!(
        l.no_step[1].starts_with("loops.t1:18 absent"),
        "{:?}",
        l.no_step
    );
    // 15. One loop at depth 1, three at depth 0; all four in one routine.
    let mut d = l.depths.clone();
    d.sort_unstable();
    assert_eq!(d, vec![0, 0, 0, 1]);
    assert_eq!((l.routines_with_loops, l.max_loops_in_a_routine), (1, 4));

    // A `चरः` inside an earlier sibling block is NOT the loop's entry.
    let sibling = "मण्डलम् परीक्षा ॥
वृत्तिः क आदाय अ ॱॱ न६४ ददाति न६४ आदि
    यदि अ समम् ० आदि
        चरः ख ॱॱ न६४ भवति ० ।
    इति
    यावत् ख न्यूनम् अ आदि
        ख भवति ख योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let p = parse_source("sibling.t1", sibling);
    let l = loops(&p);
    assert_eq!(
        l.entries.get("local_binding"),
        None,
        "ख is out of scope: {:?}",
        l.entries
    );
    assert_eq!(
        l.entries.get("parameter"),
        Some(&1),
        "so the parameter अ is the entry"
    );

    let c = loop_census(std::slice::from_ref(&p));
    assert_eq!(c.get("loop_backedge_instrument"), Some("ast"));
    assert_eq!(c.get("loop_step_present"), Some("1"));
}

// ─────────────────────────────────────────────────────────────────────────
// Tests that run with the suite.
// ─────────────────────────────────────────────────────────────────────────

/// Every `(run, pos)` in [`LETTERS`] is a row of `spec/shiva-sutras.tsv` with
/// that glyph — the table in code cannot drift from the model's file.
#[test]
fn every_letter_sits_where_the_model_puts_it() {
    let tsv = std::fs::read_to_string(spec_root().join("shiva-sutras.tsv"))
        .expect("spec/shiva-sutras.tsv is readable");
    let rows: Vec<(u8, u8, String)> = tsv
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("line\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Some((
                f.first()?.parse().ok()?,
                f.get(1)?.parse().ok()?,
                (*f.get(2)?).to_string(),
            ))
        })
        .collect();
    assert_eq!(rows.len(), 57, "the model has 57 positions");
    for r in &LETTERS {
        let at = rows
            .iter()
            .find(|(run, pos, _)| (*run, *pos) == (r.run, r.pos))
            .unwrap_or_else(|| panic!("{}: {}-{} is not on the model", r.glyph, r.run, r.pos));
        assert_eq!(
            at.2, r.glyph,
            "{}-{} holds `{}`, the table says `{}`",
            r.run, r.pos, at.2, r.glyph
        );
        if let Some((mr, mp)) = r.marker {
            let m = rows
                .iter()
                .find(|(run, pos, _)| (*run, *pos) == (mr, mp))
                .unwrap_or_else(|| panic!("{}: marker {mr}-{mp} is not on the model", r.glyph));
            assert_eq!(
                m.2,
                format!("{}्", r.glyph),
                "the marker is the letter with virāma"
            );
        }
    }
    assert_eq!(
        LETTERS.iter().filter(|r| r.glyph == "ह").count(),
        2,
        "ह stands in two places"
    );
}

/// (a) THE REFUSED CASE. A kind the table cannot place is refused, by name,
/// and every kind the corpus contains is placed.
#[test]
fn the_map_refuses_what_it_cannot_place_and_places_every_corpus_kind() {
    // Synthetic unknowns, one per family, both sides of the range.
    for k in [
        Kind::Statement(0),
        Kind::Statement(9),
        Kind::Statement(99),
        Kind::Expression(0),
        // 13 is the slice since W-228; the first unplaced expression kind is 14.
        Kind::Expression(14),
        Kind::Operator(0),
        Kind::Operator(16),
        Kind::Declaration(0),
        // ६ is `गणनाघोषणाभेद` since `W-215`; the first unknown is ७.
        Kind::Declaration(7),
        Kind::Type(0),
        // १ was `मूलप्रकारभेद` until `W-171` moved the family to १०१..१०५;
        // the old range must now be refused, both ends of the new one too.
        Kind::Type(1),
        Kind::Type(5),
        Kind::Type(100),
        Kind::Type(106),
    ] {
        assert_eq!(
            place(k),
            Err(Unplaced::Unknown(k)),
            "{k:?} must be refused as unknown"
        );
    }
    // Declared and never built: refused as not yet understood, not as unknown.
    assert_eq!(
        place(Kind::Declaration(D_MACHINE)),
        Err(Unplaced::NeverBuilt(Kind::Declaration(D_MACHINE)))
    );
    // A binary node is placed by its operator; asking by kind is a census bug.
    assert_eq!(place(Kind::Expression(E_BINARY)), Err(Unplaced::ByOperator));

    // Every kind `ast.t1`/`parse.t1` declare and `व्याकर` builds.
    let mut placed = Vec::new();
    placed.push(Kind::Program);
    placed.extend((1..=8).map(Kind::Statement));
    placed.extend((1..=12).filter(|k| *k != E_BINARY).map(Kind::Expression));
    placed.extend((1..=15).map(Kind::Operator));
    placed.extend([D_ROUTINE, D_TYPE, D_BINDING, D_IMPORT, D_ENUM].map(Kind::Declaration));
    placed.extend((T_FIRST..=T_LAST).map(Kind::Type));
    for k in &placed {
        let p = place(*k).unwrap_or_else(|e| panic!("{k:?} must be placed, got {e:?}"));
        // A place is on the model: a letter row, or a run 1..=14.
        match p {
            Place::Letter(_) => {}
            Place::Marker { run, pos, .. } => assert_eq!((run, pos), (1, 4)),
            Place::Runs { from, to, .. } => assert!(from >= 1 && to <= 14 && from <= to),
        }
    }
    // five declaration kinds placed since `W-215` (the enum); the machine is
    // declared and never built, and is refused above by name.
    assert_eq!(placed.len(), 1 + 8 + 11 + 15 + 5 + 5);
    // Every ladder level is one the grammar has.
    for (op, _, level) in &OPERATORS {
        assert!(["compare", "or", "xor", "and", "shift", "add", "mul"].contains(level));
        assert_eq!(ladder(*op), Some(*level));
    }
}

/// A tiny program — one import, one binding, one loop, one call as a
/// statement, one call in a return — for the walk to count.
const TINY: &str = "मण्डलम् परीक्षा ॥
आयातः वास्तु ।
सार्वजनिक वृत्तिः क आदाय अ ॱॱ न६४ ददाति न६४ आदि
    चरः ख ॱॱ न६४ भवति ० ।
    यावत् ख न्यूनम् अ आदि
        ख भवति ख योगः १ ।
    इति
    ग आरभ्य ख समाप्तम् ।
    प्रत्यागमनम् आरभ्य क ख समाप्तम् ।
इति
";

/// (c) THE TINY PROGRAM yields the letters its constructs name.
#[test]
fn a_tiny_program_lands_on_the_letters_its_constructs_name() {
    let p = parse_source("tiny.t1", TINY);
    assert_eq!(p.parse_errors, 0, "the tiny program parses");
    assert_eq!(
        p.text(&p.toks[1]),
        "मण्डलम्",
        "the first token is the module header"
    );
    let s = walk(&p);
    let at = |pl: Place| s.by_place.get(&pl).copied().unwrap_or(0);
    use Letter as L;
    assert_eq!(at(Place::Letter(L::Ha5)), 1, "one मण्डलम्");
    assert_eq!(at(Place::Letter(L::Ya)), 1, "one आयातः");
    assert_eq!(at(Place::Letter(L::Tta)), 3, "one वृत्तिः and two calls");
    assert_eq!(
        at(Place::Letter(L::Ta)),
        2,
        "the routine body and the loop body"
    );
    assert_eq!(at(Place::Letter(L::Va)), 1, "one call as a statement");
    assert_eq!(at(Place::Letter(L::Ra)), 1, "one आरभ्य … समाप्तम् group");
    assert_eq!(at(Place::Letter(L::VaIt)), 1, "one प्रत्यागमनम्");
    assert_eq!(at(Place::Letter(L::Ma)), 1, "one यावत्");
    assert_eq!(at(Place::Letter(L::Nga)), 0, "no यदि");
    assert_eq!(at(Place::Letter(L::Nna)), 1, "one चरः");
    assert_eq!(at(Place::Letter(L::Na)), 1, "one भवति");
    assert_eq!(
        at(Place::Letter(L::MaIt)) + at(Place::Letter(L::La)) + at(Place::Letter(L::Ha14)),
        0
    );
    assert_eq!(at(SIBILANTS), 1, "one न्यूनम्");
    assert_eq!(at(STOPS), 1, "one योगः");
    assert_eq!(
        at(VOWELS),
        2 + 3,
        "two numerals (०, १) and three न६४ in position"
    );
    // Uses: ख (loop condition), अ, ख (assignment target), ख (assignment
    // source), ग, ख (call), क, ख (return) — eight names.
    assert_eq!(at(USE), 8, "eight name uses");
    let c = census(std::slice::from_ref(&p));
    assert_eq!(c.get("letters_ma_sites"), Some("1"));
    assert_eq!(
        c.get("letters_expression_slots_unreached"),
        Some("0"),
        "every expression slot is reached"
    );
}

/// The four statistics on the tiny program, where every number can be
/// counted by eye.
#[test]
fn the_tiny_program_measures_as_it_reads() {
    let p = parse_source("tiny.t1", TINY);
    let c = census(std::slice::from_ref(&p));
    // 5. One routine: चरः, यावत्, the assignment in its body, the call
    // statement, the return — five statements; two blocks (routine, loop).
    assert_eq!(c.get("sequence_routines"), Some("1"));
    assert_eq!(c.get("sequence_statements"), Some("5"));
    assert_eq!(c.get("sequence_blocks"), Some("2"));
    assert_eq!(c.get("sequence_statements_per_routine_max"), Some("5"));
    assert_eq!(c.get("sequence_statements_outside_routines"), Some("0"));
    // 6. Two आदि … इति, two आरभ्य … समाप्तम्, no index, no string — all paired.
    assert_eq!(c.get("spans_adi_iti_openers"), Some("2"));
    assert_eq!(c.get("spans_adi_iti_rate"), Some("100.0%"));
    assert_eq!(c.get("spans_arabhya_samaptam_paired"), Some("2"));
    assert_eq!(c.get("spans_ankah_antah_openers"), Some("0"));
    assert_eq!(c.get("spans_uktam_iti_openers"), Some("0"));
    // 24. One न्यूनम् (compare), one योगः (add), no chain.
    assert_eq!(c.get("operators_nyunam_sites"), Some("1"));
    assert_eq!(c.get("operators_level_compare_sites"), Some("1"));
    assert_eq!(c.get("operators_level_add_sites"), Some("1"));
    assert_eq!(c.get("operators_chained_comparisons"), Some("0"));
    // 25. Five dandas, one double danda, two ॱॱ, no member mark, no comment.
    assert_eq!(c.get("signs_danda_sites"), Some("5"));
    assert_eq!(c.get("signs_double_danda_sites"), Some("1"));
    assert_eq!(c.get("signs_label_mark_sites"), Some("2"));
    assert_eq!(c.get("signs_member_mark_sites"), Some("0"));
    assert_eq!(c.get("signs_nonrepertoire"), Some("0"));
}

/// The instruments SEE what they measure: a chained comparison, an unpaired
/// span and a stray ASCII sign are each counted with a file and a line, not
/// silently zero.
#[test]
fn the_instruments_see_a_chain_an_unpaired_span_and_a_stray_sign() {
    // parse.t1:781 writes exactly this shape: `a न्यूनम् b समम् असत्यम्`.
    let chained = "मण्डलम् परीक्षा ॥
वृत्तिः क आदाय अ ॱॱ न६४ ददाति बूल आदि
    प्रत्यागमनम् अ न्यूनम् १ समम् असत्यम् ।
इति
";
    let p = parse_source("chained.t1", chained);
    assert_eq!(p.parse_errors, 0, "the flat parser accepts the chain");
    let c = census(std::slice::from_ref(&p));
    assert_eq!(c.get("operators_chained_comparisons"), Some("1"));
    assert!(
        c.notes.iter().any(|n| n.contains("chained.t1:3")),
        "the chain is reported at its line: {:?}",
        c.notes
    );
    // A grouped comparison is NOT a chain: `आरभ्य a न्यूनम् b समाप्तम् समम् x`.
    let grouped = "मण्डलम् परीक्षा ॥
वृत्तिः क आदाय अ ॱॱ न६४ ददाति बूल आदि
    प्रत्यागमनम् आरभ्य अ न्यूनम् १ समाप्तम् समम् असत्यम् ।
इति
";
    let p = parse_source("grouped.t1", grouped);
    assert_eq!(p.parse_errors, 0);
    let c = census(std::slice::from_ref(&p));
    assert_eq!(c.get("operators_chained_comparisons"), Some("0"));

    // The span walk is independent of the parser: a program the parser
    // refuses still has its openers counted and its gap named.
    let unpaired = "मण्डलम् परीक्षा ॥
वृत्तिः क ददाति न६४ आदि
    प्रत्यागमनम् आरभ्य १ ।
इति
";
    let p = parse_source("unpaired.t1", unpaired);
    let s = span_pairing(&p);
    assert_eq!(
        (s.arabhya_samaptam.openers, s.arabhya_samaptam.paired),
        (1, 0)
    );
    assert_eq!(s.arabhya_samaptam.exceptions.len(), 1);
    assert!(s.arabhya_samaptam.exceptions[0].starts_with("unpaired.t1:3"));
    assert_eq!((s.adi_iti.openers, s.adi_iti.paired), (1, 1));

    // ADR-0011: a doubled `इति इति` inside a string is the word, and the
    // string's body may hold an opener without opening anything.
    let strings = "मण्डलम् परीक्षा ॥
वृत्तिः क ददाति न६४ आदि
    चरः अ ॱॱ पाठ भवति उक्तम् आदि इति इति समाप्तम् इति ।
    प्रत्यागमनम् ० ।
इति
";
    let p = parse_source("strings.t1", strings);
    assert_eq!(
        p.parse_errors, 0,
        "the parser reads the same string the walk does"
    );
    let s = span_pairing(&p);
    assert_eq!((s.uktam_iti.openers, s.uktam_iti.paired), (1, 1));
    assert_eq!(
        (s.adi_iti.openers, s.adi_iti.paired),
        (1, 1),
        "the आदि inside the string is text"
    );
    assert_eq!(
        s.arabhya_samaptam.openers, 0,
        "the समाप्तम् inside the string closes nothing"
    );

    // A stray ASCII sign in code is a non-repertoire finding at its line; the
    // same sign after `॰` is a comment and is not.
    let stray = "मण्डलम् परीक्षा ॥\nचरः क ॱॱ न६४ भवति १ ; ।\n॰ a comment, with ; and (parentheses)\n";
    let p = Program {
        name: "stray.t1".into(),
        src: stray.into(),
        toks: vec![Tok::default()],
        stmts: vec![Stmt::default()],
        exprs: vec![Expr::default()],
        decls: vec![Decl::default()],
        params: vec![(0, 0)],
        parse_errors: 0,
    };
    let s = sign_purity(&p);
    assert_eq!(s.nonrepertoire, vec!["stray.t1:2 U+003B".to_string()]);
    assert_eq!(
        (s.danda, s.double_danda, s.label_mark, s.comment_lines),
        (1, 1, 1, 1)
    );
}

/// (b) THE REPORT GENERATOR IS BYTE-STABLE: two runs over the same METRIC
/// lines write the same bytes, and the bytes carry the numbers.
#[test]
fn the_report_generator_is_byte_stable() {
    let dir = spec_fixture::unique_root("w206-report");
    std::fs::create_dir_all(&dir).expect("a temp dir");
    let input = dir.join("census.txt");
    std::fs::write(
        &input,
        "running 1 test\nMETRIC paradigm_letters_ma_sites 239\n\
         METRIC paradigm_spans_adi_iti_rate 100.0%\nMETRIC paradigm_operators_chained_comparisons 3\n\
         METRIC not_paradigm 1\ntest result: ok.\n",
    )
    .expect("the fixture is written");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/paradigm-report.py");
    let run = |out: &Path| {
        let status = std::process::Command::new("python3")
            .arg(&script)
            .arg(&input)
            .arg("--out")
            .arg(out)
            .status()
            .expect("python3 runs — the project's own generators need it (tools/gen-*.py)");
        assert!(status.success(), "the generator exits 0");
        std::fs::read(out).expect("the report is written")
    };
    let a = run(&dir.join("a.md"));
    let b = run(&dir.join("b.md"));
    assert_eq!(a, b, "two runs over the same input write the same bytes");
    let text = String::from_utf8(a).expect("UTF-8");
    assert!(
        text.contains("| `paradigm_letters_ma_sites` | 239 | – |"),
        "{text}"
    );
    assert!(
        text.contains("| `paradigm_operators_chained_comparisons` | 3 | 0 |"),
        "{text}"
    );
    assert!(
        !text.contains("not_paradigm"),
        "only paradigm_ lines are taken"
    );
    assert!(text.contains("Do not edit"));
    let _ = std::fs::remove_dir_all(&dir);
}
