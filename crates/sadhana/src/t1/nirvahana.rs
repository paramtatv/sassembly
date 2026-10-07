//! **निर्वहणम्** — executing a `.t1` body. Task `D-002j`.
//!
//! A PER-ROUTINE INVOCATION COUNTER, switched on by the `T1_CALLS` environment
//! variable and read by [`call_counts`]. Landed 2026-09-13 because it named the
//! assembler's cost in ONE run where a fuel bisect would have taken a day: the
//! top rows were `अष्टकान्वेषणम्` 2.48M and the table walkers, and the arithmetic
//! (rows visited × searches per row) closed against the step count. Off by
//! default it costs one `OnceLock` read per call.
//!
//! # Why this module exists
//!
//! Before it, **not one routine in `crates/sadhana-t1/src/*.t1` had ever been
//! RUN.** `crate::t1::parse::parse_program` descends into a routine's body only
//! when the routine's name is followed directly by `आदि`, and every routine in
//! the corpus writes `आदाय` (its parameters) or `ददाति` (its return type) there
//! instead — so `parse_program` stored `body: None` for all of them and
//! `crate::t1::comptime` had no statement, no loop and no call to evaluate even
//! if it had been handed one.
//!
//! **`D-002j`'s note says 256 routines, 235 `आदाय` and 21 `ददाति`, and those
//! three numbers are STALE.** Counted 2026-08-30 over the same 15 files: **317
//! routines, 292 `आदाय`, 25 `ददाति`, and `आदि` — the one spelling
//! `parse_program` can follow — ZERO.** The structural claim the row rests on is
//! exactly right and the arithmetic under it has moved, which is why this
//! comment states the count and the date it was taken.
//!
//! The consequence was measured on 2026-08-29 and it is the whole reason for
//! this file: inverting a comparison inside `संज्ञाकुलपठनम्` — `असमम् ४०` to
//! `समम् ४०`, the same tokens with the prune logic reversed — left **all 34
//! `sadhana-t1` tests green**. Every ratchet in that crate counts DECLARATIONS
//! and matches TEXT; none of them could see behaviour, because nothing in the
//! tree could produce any.
//!
//! [`Interpreter::call`] is the answer: it loads `.t1` sources, resolves their
//! `समावेशः` embeds through [`super::anita`], parses the routines the corpus
//! actually writes, and executes one by name with arguments.
//! `crates/sadhana-t1/tests/t1_execution.rs` calls `सङ्केतन ॱ कोष्ठाङ्कः` with
//! the octets of each of the 64 register names in `spec/registers-riscv64.tsv`
//! and asserts the number that comes back against that file; the mutation tests
//! beside it break one comparison in each routine of the call chain and require
//! the assertion to fail by name.
//!
//! # This is a reader of the corpus, not a second definition of T1
//!
//! `D-002h`'s owner decision of 2026-08-29 is *"the `.t1` corpus is canonical"*,
//! and this module takes that literally. The operator set below is the set that
//! **occurs in the corpus**, counted rather than chosen: `योगः` 399, `समम्`
//! 357, `न्यूनम्` 205, `वियोगः` 95, `असमम्` 78, `अधिकम्` 76, `वामसृ` 23,
//! `युक्` 17, `दक्षिणसृ` 12, `गुणनम्` 12, `विकल्प` 9, `विभाजनम्` 5, `शेषः` 3,
//! `बृहत्समम्` 2.
//!
//! **THIS PARAGRAPH USED TO SAY THE LEXICON PROPOSED A DIFFERENT SET, AND NAMED
//! FOUR WORDS THAT ARE NOT IN IT.** It read that `spec/lexicon.tsv` proposes
//! `अधि` for plus (line 24), `ऊन` for minus (86), `विषमम्` for not-equal (340)
//! and `अधिकसमम्` for greater-or-equal (26). MEASURED 2026-09-28: **none of those
//! four is an entry in that file at all**, and the cited lines hold unrelated
//! rows. Two of them are one syllable from a word that IS there and IS used —
//! `विषम` is bitwise-xor and `बृहत्समम्` is greater-or-equal (ADR-0037, "frozen
//! from the corpus", read by `parse.t1:468` and `:502`). A margin that names a
//! near-miss spelling is worse than none: `W-307` was opened after this sentence
//! sent a reader looking for a `>=` the language already has.
//!
//! What survives is the count above — the operator set IS the set that occurs —
//! and the direction: **CORPUS PRIMACY**, `D-002h` (owner, 2026-08-29), restated
//! as ruling Q5(b) on 2026-09-28. The lexicon reflects the language and does not
//! constrain it; where the two differ the corpus wins and the row changes.
//! `spec/lexicon.tsv`'s `status` column is now DERIVED from corpus token
//! presence or an ADR source rather than authored, so a word this file has never
//! adopted reads `proposed` and is not a defect. `D-002h`'s remaining half is
//! scoped by ruling Q5(c) to the 185 language-bearing entries — keyword,
//! operator, type and mnemonic — and is not done here.
//!
//! `spec/grammar-t1.ebnf:380-395` still says `expression`, `statement` and
//! `type` are DEFERRED NON-TERMINALS that *"are not defined anywhere yet"*, and
//! this module does not amend it — a grammar is normative and a reader is not.
//! But `Parser::comparison` through `Parser::primary` below IS a written-down
//! precedence table for the operators the corpus uses, derived from it, and
//! `D-002h`'s remaining half is the transcription of exactly that into the
//! grammar, `spec/lexicon.tsv` and `crates/tree-sitter-t1/grammar.js`.
//!
//! Precedence is C's, which is what the corpus's own bracketing implies: every
//! place a shift meets an addition (`आरभ्य पदम् वामसृ ४ समाप्तम् योगः
//! अङ्कमूल्यम्`) the corpus writes the group out, so the only mixed cases it
//! relies on are `विकल्प` under `वामसृ` (`पदम् विकल्प द्वितीयः वामसृ ८`) and a
//! comparison under everything.
//!
//! # What is deliberately NOT here
//!
//! * **Narrow widths.** Values are [`i128`] holding 64-bit WORDS: since `W-381`
//!   stage 3 (owner ruling P1, 2026-10-05: "64-bit two's-complement WRAPPING is
//!   the defined semantics") every operator wraps to 64 bits exactly as the RV64
//!   instruction the native code uses (`binop`), and an ordering comparison
//!   over a name declared unsigned compares unsigned. What is NOT modelled is a
//!   width BELOW 64: a `न३२` or `अ८` local is a whole word here, as it is in a
//!   native register, and a store into an octet run keeps the low eight bits
//!   (owner ruling (a), 2026-10-06), as the native `sb` does.
//! * **The type checker.** Declared types are read only far enough to decide
//!   what a `भवति ०` initialiser MEANS: a fresh record for a `संरचना` type, an
//!   empty arena for a slice of one, an empty octet run for `अङ्कः अन्तः अ८`,
//!   and the number zero otherwise. Nothing is checked.
//! * **`दोषयुक्त` propagation, `स्थानम्` dereference, `गणना`.** No routine
//!   reached from the acceptance target uses them.
//! * **A per-module arity table.** A call is juxtaposition, so a bare name's
//!   ARITY has to be known before its arguments can be parsed, and the table is
//!   built across every loaded module at once: two modules declaring the same
//!   routine name with different arities would leave the later one's count
//!   standing for both. Measured on the pair the tests load — `encode.t1` and
//!   `vakyavibhaga.t1` share NO routine name at all — so it costs nothing
//!   today, and it is written down rather than discovered when a third module
//!   is added. (RESOLUTION at run time is already per-module and correct: see
//!   `Interpreter::resolve_call`.)
//!
//! A routine whose body uses something not implemented is not a load failure:
//! it is kept as [`Routine::body`] `Err(reason)` and refuses only when CALLED.
//! [`Interpreter::report`] counts them, so the gap is a number rather than a
//! surprise.

use crate::lex::{Kind, Token};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

// ── the words, all of them from the corpus ───────────────────────────────

const W_MODULE: &str = "मण्डलम्";
const W_PUBLIC: &str = "सार्वजनिक";
const W_FN: &str = "वृत्तिः";
/// `आयातः` — an import declaration (`W-223`: the store records it).
const W_IMPORT: &str = "आयातः";
const W_STRUCT: &str = "संरचना";
const W_ENUM: &str = "गणना";
const W_LET: &str = "चरः";
const W_TAKES: &str = "आदाय";
const W_GIVES: &str = "ददाति";
const W_BLOCK_OPEN: &str = "आदि";
const W_BLOCK_CLOSE: &str = "इति";
const W_ELSE: &str = "अन्यथा";
const W_IF: &str = "यदि";
const W_WHILE: &str = "यावत्";
const W_RETURN: &str = "प्रत्यागमनम्";
const W_BECOMES: &str = "भवति";
const W_GROUP_OPEN: &str = "आरभ्य";
const W_GROUP_CLOSE: &str = "समाप्तम्";
const W_INDEX_OPEN: &str = "अङ्कः";
const W_INDEX_CLOSE: &str = "अन्तः";
const W_POINTER: &str = "स्थानम्";
const W_OPTIONAL: &str = "सम्भाव्य";
const W_ERRUNION: &str = "दोषयुक्त";
const W_TRUE: &str = "सत्यम्";
const W_FALSE: &str = "असत्यम्";
const W_NIL: &str = "शून्यम्";
const W_LEN: &str = "दैर्घ्य";

/// The one octet type. A slice of it is a run of octets rather than an arena.
const T_U8: &str = "अ८";
/// `V-005`: the 64-bit float type, `grammar-t1.ebnf`'s `float_type` at width ६४.
const T_F64: &str = "प६४";
/// `W-373`: the 64-bit unsigned and signed integer types — the event slot's
/// other two accepted types (`Interpreter::set_events`).
const T_U64: &str = "न६४";
const T_I64: &str = "अ६४";
/// The text type, `grammar-t1.ebnf:226` — and the spelling ADR-0029 retired, which
/// the corpus still writes in type position (`W-267`).
const W_TEXT: &str = "पाठ";
const W_TEXT_RETIRED: &str = "पाठः";

/// A scalar `type_name` of `grammar-t1.ebnf:226`/`:248` other than text: `अ` or `न`
/// with a width, `प३२`/`प६४`, `बूल`, `अक्षरम्`.
fn is_scalar_type_name(n: &str) -> bool {
    const WIDTHS: [&str; 5] = ["८", "१६", "३२", "६४", "१२८"];
    // NOT an early return on the prefix: `अक्षरम्` begins with `अ` too.
    let width = |w: &str| WIDTHS.contains(&w);
    n.strip_prefix('अ').is_some_and(width)
        || n.strip_prefix('न').is_some_and(width)
        || matches!(n, "प३२" | "प६४" | "बूल" | "अक्षरम्")
}

// ── values ───────────────────────────────────────────────────────────────

/// A run of octets: the whole of what `अङ्कः अन्तः अ८` holds.
///
/// Backed by an [`Rc`] with an explicit range so that
/// `आरभ्य पाठ्यम् अङ्कः क आरम्भः अन्तः क सीमा समाप्तम्` — the corpus's slice —
/// is a view and not a copy. `spec/mnemonics-riscv64.src.tsv` is 9842 octets,
/// and `संज्ञाकुलपठनम्` takes a slice of it for four fields of every row it
/// keeps and for every alias piece inside the fifth.
#[derive(Debug, Clone)]
pub struct Octets {
    data: Rc<Vec<u8>>,
    start: usize,
    end: usize,
}

impl Octets {
    /// The octets of `text`, whole.
    #[must_use]
    pub fn new(text: &[u8]) -> Self {
        let data = Rc::new(text.to_vec());
        let end = data.len();
        Self {
            data,
            start: 0,
            end,
        }
    }
    /// How many octets the run holds — `ॱ दैर्घ्य`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    /// Whether the run is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The octets themselves.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.data[self.start..self.end]
    }
}

/// A value a `.t1` body can hold.
#[derive(Debug, Clone)]
pub enum Value {
    /// Every numeric type. See the module header on why this is not sized.
    Int(i128),
    /// `बूल`.
    Bool(bool),
    /// `अङ्कः अन्तः अ८` — a run of octets.
    Octets(Octets),
    /// One `संरचना` record, shared by reference as the corpus's arenas need.
    Record(Rc<RefCell<HashMap<String, Value>>>),
    /// `अङ्कः अन्तः <struct>` — a growable arena. A fresh one (`भवति ०`) is
    /// EMPTY, length ०, with no slot at index ० (`W-355`, owner ruling
    /// 2026-10-03); reading any index of it is refused. Most corpus arenas are
    /// still filled ONE-BASED — every `…योजनम्` in `vakyavibhaga.t1` says
    /// "advance the index, write at it, return it — so the number returned is a
    /// live entry and ० never is" — and a first write at index १ grows the store
    /// to two, leaving a `Nil` at ० that only that write created.
    Arena(Rc<RefCell<Vec<Value>>>),
    /// `शून्यम्` — the empty `सम्भाव्य`.
    Nil,
    /// `प६४` — a 64-bit IEEE-754 double, CARRIED AS ITS BIT PATTERN (`V-005`).
    ///
    /// The bits and not an `f64`, because the bits are what the two engines are
    /// compared on: natively the value lives in an `f` register and is read back
    /// through `प्लवसंचारः` as sixty-four bits, so a NaN's payload and the sign of
    /// a zero are part of the answer and an `f64` field would invite an `==` that
    /// says `-0 == +0` and `NaN != NaN`. A variant of its own and not an `Int`, so
    /// that an integer operator meeting a float REFUSES (as the native builder
    /// does) instead of adding two bit patterns.
    Float(u64),
}

impl Value {
    fn int(&self) -> Result<i128, RunError> {
        match self {
            Self::Int(n) => Ok(*n),
            Self::Bool(b) => Ok(i128::from(*b)),
            other => Err(RunError::new(format!("expected a number, found {other:?}"))),
        }
    }
    fn truth(&self) -> Result<bool, RunError> {
        match self {
            Self::Bool(b) => Ok(*b),
            Self::Int(n) => Ok(*n != 0),
            other => Err(RunError::new(format!(
                "expected a truth value, found {other:?}"
            ))),
        }
    }
    /// The octets this value holds, for a caller reading a result back.
    #[must_use]
    pub fn octets(&self) -> Option<&Octets> {
        match self {
            Self::Octets(o) => Some(o),
            _ => None,
        }
    }
    /// The number this value holds, for a caller reading a result back.
    #[must_use]
    pub fn as_int(&self) -> Option<i128> {
        match self {
            Self::Int(n) => Some(*n),
            _ => None,
        }
    }
    /// Whether this value is `शून्यम्`.
    #[must_use]
    pub fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }
    /// The bit pattern of a `प६४` value, for a caller reading a result back
    /// (`V-005`).
    #[must_use]
    pub fn as_float_bits(&self) -> Option<u64> {
        match self {
            Self::Float(b) => Some(*b),
            _ => None,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // `W-381` stage 3: an integer is a 64-bit WORD, and one word has two
            // `i128` spellings when its top bit is set (`ऋण१` and २^६४−१, see
            // [`wrap`]); equal words are equal values, as `beq` says natively.
            (Self::Int(a), Self::Int(b)) => word(*a) == word(*b),
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Bool(b)) | (Self::Bool(b), Self::Int(a)) => {
                word(*a) == u64::from(*b)
            }
            (Self::Octets(a), Self::Octets(b)) => a.as_slice() == b.as_slice(),
            (Self::Nil, Self::Nil) => true,
            // Bit identity, which is what the engines are compared on — NOT the
            // IEEE comparison, which is `प्लवसमम्`'s and refuses here (`binop`).
            (Self::Float(a), Self::Float(b)) => a == b,
            _ => false,
        }
    }
}

// ── errors ───────────────────────────────────────────────────────────────

/// Why a `.t1` body could not be loaded or could not be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunError {
    /// Plain English until `A-037` routes diagnostics through the lexicon.
    pub reason: String,
}

impl RunError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl core::fmt::Display for RunError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for RunError {}

// ── types, read only as far as an initialiser needs ──────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ty {
    Named(String),
    Slice(Box<Ty>),
    Pointer(Box<Ty>),
    Optional(Box<Ty>),
    ErrorUnion(Box<Ty>),
}

impl Ty {
    /// The type as the source spells it — `अङ्कः अन्तः T`, `स्थानम् T`,
    /// `सम्भाव्य T`, `दोषयुक्त T`, or the name — one space between words. The
    /// declaration store's T1 twin (`sanchaya.t1`, `प्रकारलेखनम्`) writes the
    /// same octets, and `W-223`'s census compares the two.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Ty::Named(n) => n.clone(),
            Ty::Slice(t) => format!("{W_INDEX_OPEN} {W_INDEX_CLOSE} {}", t.text()),
            Ty::Pointer(t) => format!("{W_POINTER} {}", t.text()),
            Ty::Optional(t) => format!("{W_OPTIONAL} {}", t.text()),
            Ty::ErrorUnion(t) => format!("{W_ERRUNION} {}", t.text()),
        }
    }
}

// ── the tree ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Shl,
    Shr,
    /// `दक्षिणसृ` over a NAME declared unsigned — `W-333`. The parser never
    /// builds this; [`mark_unsigned_operators`] rewrites a `Shr` into it at load.
    ShrL,
    And,
    Or,
    Xor,
    Eq,
    Ne,
    Lt,
    Gt,
    Ge,
    /// `W-381` stage 3 (owner ruling P1, 2026-10-05): the three ordering
    /// comparisons when EITHER operand is a NAME declared unsigned, which compare
    /// the two 64-bit words unsigned (`sltu`/`bltu`/`bgeu` natively). The parser
    /// never builds these; [`mark_unsigned_operators`] rewrites `Lt`, `Gt`, `Ge`
    /// into them at load, by `W-333`'s rule.
    LtU,
    GtU,
    GeU,
    /// `W-381` stage 3, owner ruling (b): `विभाजनम्` and `शेषः` with a NAME
    /// declared unsigned on either side read both words unsigned (`divu`/`remu`),
    /// marked at load by the compare's rule.
    DivU,
    RemU,
}

#[derive(Debug, Clone)]
enum Expr {
    Num(i128),
    Str(Rc<Vec<u8>>),
    Bool(bool),
    Nil,
    Var(String),
    Call {
        module: Option<String>,
        name: String,
        args: Vec<Expr>,
        /// `V-008` part 2: this call is the vector BUILT-IN — set ONLY by the
        /// one-token `व्यूहॱ<member>` parse path. A flag and not a string in
        /// `module`, because a module name can be anything a source declares:
        /// a mark in `module` was forged by `मण्डलम् व्यूहॱ ॥`.
        vector: bool,
        /// `W-373`: this call is the WAIT intrinsic — set ONLY by the one-token
        /// `अष्टकॱ<member>` parse path ([`WAIT_BUILTIN_MEMBER`]), for the vector
        /// flag's reason: a flag on the node, never a mark forged into `module`.
        wait: bool,
    },
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Slice(Box<Expr>, Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
}

#[derive(Debug, Clone)]
enum Stmt {
    Let {
        name: String,
        ty: Ty,
        init: Expr,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Eval(Expr),
}

/// One `सार्वजनिक वृत्तिः` of a loaded source.
#[derive(Debug, Clone)]
pub struct Routine {
    /// The name as the corpus spells it.
    pub name: String,
    /// The module it was declared in.
    pub module: String,
    /// 1-based line of its `वृत्तिः`.
    pub line: usize,
    params: Vec<(String, Ty)>,
    /// `ददाति T`, or none (`W-223`: kept for the declaration store; the loader
    /// read it and dropped it before).
    returns: Option<Ty>,
    public: bool,
    /// The parsed body, or why it could not be parsed. A routine that cannot
    /// be parsed still LOADS: it refuses when called, and is counted by
    /// [`Interpreter::report`].
    body: Result<Vec<Stmt>, String>,
    /// `W-359`: the slice parameter this routine GROWS AND RETURNS, if any —
    /// see [`Interpreter::grown_and_returned`]. A call to it whose result is
    /// not assigned back to its own name, or tail-returned, is refused at load.
    grows: Option<String>,
    /// `W-359`: its OTHER slice parameters. A store into one of these at an
    /// index at or past the parameter's length is refused at run time
    /// ([`Interpreter::assign`]), as the native lowering refuses it.
    guarded: Vec<String>,
}

/// `W-359` (coordinator's final ruling, 2026-10-04): the most calls deep a
/// grow-and-return classification follows tail calls. The `.t1` twin is
/// `घोषणासञ्चयॱवर्धकगहनसीमा`; past it a routine is not grow-and-return.
const GROWTH_DEPTH: usize = 8;

/// Every `प्रत्यागमनम्` of a body, at any depth.
fn returns_of<'a>(stmts: &'a [Stmt], out: &mut Vec<Option<&'a Expr>>) {
    for s in stmts {
        match s {
            Stmt::Return(e) => out.push(e.as_ref()),
            Stmt::If { then, els, .. } => {
                returns_of(then, out);
                returns_of(els, out);
            }
            Stmt::While { body, .. } => returns_of(body, out),
            Stmt::Let { .. } | Stmt::Assign { .. } | Stmt::Eval(_) => {}
        }
    }
}

impl Routine {
    /// How many arguments the declaration takes.
    #[must_use]
    pub fn arity(&self) -> usize {
        self.params.len()
    }
    /// `W-359`: the run parameter this routine grows and returns, if it is one.
    #[must_use]
    pub fn grows(&self) -> Option<&str> {
        self.grows.as_deref()
    }
    /// Whether the body parsed.
    #[must_use]
    pub fn is_runnable(&self) -> bool {
        self.body.is_ok()
    }
    /// Why the body did not parse, if it did not.
    #[must_use]
    pub fn why_not(&self) -> Option<&str> {
        self.body.as_ref().err().map(String::as_str)
    }
    /// The routine as the loader read it — its parameters and its parsed
    /// body, or the reason the body did not parse — as one string, so two
    /// loads of the same sources can be compared routine by routine.
    ///
    /// `W-224`: the loader is order-independent exactly when this is the
    /// same for every routine under every order the sources are given in.
    /// A bare call's arity is read from a map the loader fills source by
    /// source, so the shape is where an order dependence would first show.
    #[must_use]
    pub fn shape(&self) -> String {
        format!("{:?} {:?}", self.params, self.body)
    }
}

// ── loading ──────────────────────────────────────────────────────────────

/// A `.t1` source, loaded: its module name, its routines, its structs and the
/// initial value of each of its `सार्वजनिक चरः` globals.
struct Module {
    name: String,
    structs: HashMap<String, Vec<(String, Ty)>>,
    globals: Vec<(String, Ty, Expr)>,
    /// The `गणना` variants this module declares, `name → ordinal`. A variant
    /// is MODULE-SCOPED: a bare use inside the declaring module and a
    /// qualified use from outside both read this table (`lookup_global`),
    /// and only a name every declaring module agrees on is also a bare key
    /// in `Interpreter::globals` (`W-224`).
    variants: HashMap<String, i128>,
    routines: Vec<Rc<Routine>>,
    /// `W-223`: what the declaration store holds beside the above — each
    /// `गणना` with its variants in order, each `आयातः`, and the names declared
    /// `सार्वजनिक` (structs, enums and globals; a routine carries its own).
    enums: Vec<(String, Vec<String>)>,
    imports: Vec<String>,
    public: HashSet<String>,
    struct_decls: Vec<(String, Vec<(String, Ty)>)>,
}

/// Everything a `.t1` program needs to run: its modules, and the state its
/// globals hold.
///
/// One interpreter is one RUN. `संज्ञाकुलपठनम्` appends into
/// `पाठांशकोश` and `संज्ञाकुलकोश` and returns an index into them, so a second
/// call on the same interpreter continues where the first left off — exactly as
/// the corpus's own arenas do — and a test that wants a fresh table builds a
/// fresh [`Interpreter`].
pub struct Interpreter {
    modules: Vec<Module>,
    by_module: HashMap<String, usize>,
    /// `(module, name)` and `name` both resolve here; a bare name is looked up
    /// in the calling module first and then across every module, which is what
    /// the corpus's unqualified calls mean.
    globals: HashMap<String, Value>,
    /// Bounds every loop and every call, so a `यावत्` that does not terminate
    /// is a diagnostic and not a hung test.
    fuel: u64,
    /// What `call` was given, so a running profile can say how far it is.
    budget: u64,
    /// THE OUTPUT SINK — the octets `अष्टकॱमुद्रणम्` has written.
    ///
    /// Until this existed, **a `.t1` program had no way to emit anything**: the
    /// only channel out of a compiled image was the 16-bit halt status, which is
    /// why every rung has had to pack its answer into digits (`१००३`, `२०००`,
    /// `५१११`) and why an outcome the encoding could not express — a fault —
    /// arrived as an absence. Natively the same routine lowers to a store at the
    /// device address; here the interpreter collects the octets instead, so the
    /// two sides can be compared on what they WROTE rather than on a number
    /// squeezed through sixteen bits.
    sink: Vec<u8>,
    /// `W-359`: the routines being executed, innermost last — what an indexed
    /// store asks to learn whether its target is a guarded parameter.
    active: Vec<Rc<Routine>>,
    /// `V-008` — THE RUNS DECLARED `अङ्कः अन्तः प६४`, by the address of their
    /// shared storage. An arena carries no element type, and an EMPTY one
    /// cannot show what it holds, so `zero_at` records each float run it makes
    /// here and an indexed store asks: a float run takes only a float, any
    /// other run never one (`FileMismatch`, as `ir.t1` refuses natively).
    /// The `Weak` is what makes the key sound: it keeps the allocation — not
    /// the elements — alive, so no later arena can be given the same address
    /// while the entry stands.
    ///
    /// PRUNED, OR IT LEAKS (review note F1, 2026-10-05): every float run ever
    /// made would hold its entry and its allocation for the whole run — this
    /// interpreter runs the compiler in Stage 1, so ten million short-lived runs
    /// would be ~700 MB. When the table reaches `float_runs_prune_at` the dead
    /// entries are dropped and the threshold becomes twice what survived (never
    /// below [`FLOAT_RUNS_PRUNE_FLOOR`]), so the table is bounded by twice the
    /// LIVE float runs and the pruning is amortised O(1) per creation. The tag
    /// was not moved onto `Value::Arena` itself: that variant is built at ~150
    /// sites in 65 files.
    float_runs: RefCell<HashMap<usize, std::rc::Weak<RefCell<Vec<Value>>>>>,
    float_runs_prune_at: std::cell::Cell<usize>,
    /// `W-381` stage 3 (owner rulings 2026-10-06) — THE NARROW INTEGER RUNS:
    /// `अङ्कः अन्तः न८`/`न१६`/`अ१६`/`न३२`/`अ३२`, by the same address-and-`Weak`
    /// key as `float_runs`, with the element's (bits, signed). A store into one
    /// keeps the low `bits` (as `sb`/`sh`/`sw` do) and is held already EXTENDED
    /// as the native load reads it back — zero for unsigned (`lbu`-like, `lhu`,
    /// `lwu`), sign for signed (`lh`, `lw`) — so a read needs nothing. Pruned
    /// with the float table's rule and threshold cell.
    narrow_runs: RefCell<HashMap<usize, NarrowRun>>,
    /// `W-373` — THE EVENT LOG, when [`Interpreter::set_events`] was given one.
    /// `None` is a run with no event source: a wait then REFUSES, as
    /// `yantra-run` with no `--events` ends the run at exit 75.
    events: Option<EventReplay>,
    /// `W-376` — THE SERIAL THREAD SCHEDULE, while [`Interpreter::run_threads`] runs.
    /// `None` outside it: a wait then takes the event log, as before.
    threads: Option<ThreadReplay>,
    /// `W-377` — THE SOCKET LOG, when [`Interpreter::set_socket`] was given one (option
    /// (b), owner ruling Q1). `None` keeps `W-350`'s refusal for every device address.
    socket: Option<SocketReplay>,
}

/// `V-008` — the smallest size at which `float_runs` is pruned of dead entries.
const FLOAT_RUNS_PRUNE_FLOOR: usize = 1024;

/// What a load found — the numbers `report` prints.
/// `W-223`: THE DECLARATION STORE'S RUST TWIN — what the loader knows of every
/// declaration of every module it loaded, in the spelling `sanchaya.t1`
/// (घोषणासञ्चय) keeps: the kind as `व्याकर` numbers it, the name, the type as
/// the source spells it, and the run of members (parameters, fields or
/// variants) with theirs. The census `measure_corpus_qualified_uses` compares
/// the two stores module by module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclarationKind {
    /// `वृत्तिः` — १.
    Routine = 1,
    /// `संरचना` — २.
    Struct = 2,
    /// `यन्त्रम्` — ३ (the loader reads none; the corpus declares none).
    Machine = 3,
    /// `चरः` — ४.
    Global = 4,
    /// `आयातः` — ५.
    Import = 5,
    /// `गणना` — ६.
    Enum = 6,
    /// A variant of a `गणना` — ७, the store's own kind.
    Variant = 7,
}

/// One declaration as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Declaration {
    pub kind: DeclarationKind,
    pub name: String,
    /// A routine's return type or a global's type, as the source spells it
    /// ([`Ty::text`]); empty for the other kinds.
    pub ty: String,
    /// Parameters, fields or variants, each with its type (a variant's is empty).
    pub members: Vec<(String, String)>,
    pub public: bool,
}

/// A module's declarations — the two files of `वास्तु` under one name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDeclarations {
    pub name: String,
    pub declarations: Vec<Declaration>,
}

impl Interpreter {
    /// Every module's declarations, modules in the order they were first
    /// loaded, a module's files merged (`W-224`), each module's declarations in
    /// the order the loader keeps them: imports, structs, enums with their
    /// variants, globals, routines.
    #[must_use]
    pub fn declarations(&self) -> Vec<ModuleDeclarations> {
        let mut out: Vec<ModuleDeclarations> = Vec::new();
        for m in &self.modules {
            let slot = match out.iter().position(|d| d.name == m.name) {
                Some(i) => i,
                None => {
                    out.push(ModuleDeclarations {
                        name: m.name.clone(),
                        declarations: Vec::new(),
                    });
                    out.len() - 1
                }
            };
            let decls = &mut out[slot].declarations;
            for i in &m.imports {
                decls.push(Declaration {
                    kind: DeclarationKind::Import,
                    name: i.clone(),
                    ty: String::new(),
                    members: Vec::new(),
                    public: false,
                });
            }
            for (name, fields) in &m.struct_decls {
                decls.push(Declaration {
                    kind: DeclarationKind::Struct,
                    name: name.clone(),
                    ty: String::new(),
                    members: fields.iter().map(|(f, t)| (f.clone(), t.text())).collect(),
                    public: m.public.contains(name),
                });
            }
            for (name, variants) in &m.enums {
                let public = m.public.contains(name);
                decls.push(Declaration {
                    kind: DeclarationKind::Enum,
                    name: name.clone(),
                    ty: String::new(),
                    members: variants
                        .iter()
                        .map(|v| (v.clone(), String::new()))
                        .collect(),
                    public,
                });
                for v in variants {
                    decls.push(Declaration {
                        kind: DeclarationKind::Variant,
                        name: v.clone(),
                        ty: String::new(),
                        members: Vec::new(),
                        public,
                    });
                }
            }
            for (name, ty, _) in &m.globals {
                decls.push(Declaration {
                    kind: DeclarationKind::Global,
                    name: name.clone(),
                    ty: ty.text(),
                    members: Vec::new(),
                    public: m.public.contains(name),
                });
            }
            for r in &m.routines {
                decls.push(Declaration {
                    kind: DeclarationKind::Routine,
                    name: r.name.clone(),
                    ty: r.returns.as_ref().map(Ty::text).unwrap_or_default(),
                    members: r
                        .params
                        .iter()
                        .map(|(p, t)| (p.clone(), t.text()))
                        .collect(),
                    public: r.public,
                });
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Routines declared across every loaded module.
    pub routines: usize,
    /// Of those, the ones whose body parsed into statements.
    pub runnable: usize,
}

// ── the parse twin's canonical tree ──────────────────────────────────────
//
// `docs/parse-twin-design.md`. The `.t1` parser (`parse.t1`) joined binary
// operators with NO precedence levels and no instrument saw it: the
// interpreter runs the `.t1` compiler through THIS parser, twin AGREE compares
// two emitters that both consume the `.t1`-parsed tree, and the `.t1` unparser
// prints operator words without brackets. A twin is blind to what both twins
// share. So this is ONE fully bracketed printer over two trees of the same
// source: side A is this parser's tree ([`Interpreter::canonical`]), side B is
// the `.t1` parser's arenas, walked by `crates/sadhana-t1/tests/parse_twin.rs`.
// Both sides build [`CanonStmt`]s and print them with [`canon_text`], so the
// printer is SHARED rather than twinned.

/// An expression as the parse twin prints it: leaves verbatim, every binary
/// operator bracketed, a call as `f(a, b)`.
///
/// A call with NO arguments prints as its bare name: the `.t1` parser has one
/// node for a name and for a nullary call, and the twin compares parse SHAPE,
/// not resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonExpr {
    /// A name, a number (as its value), a string ([`canon_string`]), a keyword.
    Leaf(String),
    /// `(l op r)` — the operator as the design's map spells it:
    /// `+ - * / % << >> & | ^ == != < > >=`.
    Bin(&'static str, Box<CanonExpr>, Box<CanonExpr>),
    /// `l[i]`.
    Index(Box<CanonExpr>, Box<CanonExpr>),
    /// `l[a..b]`.
    Slice(Box<CanonExpr>, Box<CanonExpr>, Box<CanonExpr>),
    /// `l.f`.
    Field(Box<CanonExpr>, String),
    /// `f(args)`; the callee is a name, `मण्डलॱनाम` when the call is qualified.
    Call(String, Vec<CanonExpr>),
}

/// A statement as the parse twin prints it, one line each; blocks indent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonStmt {
    /// `let n = e` (`चरः`); the declared type is not compared.
    Let(String, Option<CanonExpr>),
    /// `t = v` (`भवति`).
    Assign(CanonExpr, CanonExpr),
    /// `if c { … } else { … }`.
    If(CanonExpr, Vec<CanonStmt>, Vec<CanonStmt>),
    /// `while c { … }`.
    While(CanonExpr, Vec<CanonStmt>),
    /// `return e`, or a bare `return`.
    Return(Option<CanonExpr>),
    /// A bare expression statement.
    Eval(CanonExpr),
}

impl CanonExpr {
    /// The expression, fully bracketed, on one line.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Leaf(s) => s.clone(),
            Self::Bin(op, l, r) => format!("({} {op} {})", l.text(), r.text()),
            Self::Index(l, i) => format!("{}[{}]", l.text(), i.text()),
            Self::Slice(l, a, b) => format!("{}[{}..{}]", l.text(), a.text(), b.text()),
            Self::Field(l, f) => format!("{}.{f}", l.text()),
            Self::Call(f, args) => {
                if args.is_empty() {
                    f.clone()
                } else {
                    let args: Vec<String> = args.iter().map(Self::text).collect();
                    format!("{f}({})", args.join(", "))
                }
            }
        }
    }
}

/// A string literal's leaf: `"…"` around the text with EVERY whitespace
/// character removed. The two lexers keep a literal's interior differently —
/// this crate's keeps it exactly, `lex.t1` splits it into words and peels the
/// signs off them — so the twin compares the characters and not the spacing.
#[must_use]
pub fn canon_string(text: &str) -> CanonExpr {
    let s: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    CanonExpr::Leaf(format!("\"{s}\""))
}

/// The value of a numeral as the corpus writes it (`१०`, `ऋण१`, a radix form),
/// or `None` when the text is not one — the SAME reader this parser's
/// `primary` uses, so side B decides "numeral or name" the way side A does.
#[must_use]
pub fn numeral_value(text: &str) -> Option<i128> {
    let (negative, bare) = sanskrit_text::numeral::split_sign(text);
    let n = i128::from(sanskrit_text::numeral::value(bare).ok()?);
    Some(if negative { -n } else { n })
}

/// A routine body printed one statement per line, two spaces of indent per
/// block level, starting one level in — the lines under a `वृत्तिः name`
/// header in [`Interpreter::canonical`].
#[must_use]
pub fn canon_text(body: &[CanonStmt]) -> String {
    fn write(out: &mut String, stmts: &[CanonStmt], depth: usize) {
        let pad = "  ".repeat(depth);
        for s in stmts {
            let line = match s {
                CanonStmt::Let(n, Some(e)) => format!("let {n} = {}", e.text()),
                CanonStmt::Let(n, None) => format!("let {n}"),
                CanonStmt::Assign(t, v) => format!("{} = {}", t.text(), v.text()),
                CanonStmt::Return(Some(e)) => format!("return {}", e.text()),
                CanonStmt::Return(None) => "return".to_string(),
                CanonStmt::Eval(e) => e.text(),
                CanonStmt::If(c, then, els) => {
                    out.push_str(&format!("{pad}if {} {{\n", c.text()));
                    write(out, then, depth + 1);
                    if !els.is_empty() {
                        out.push_str(&format!("{pad}}} else {{\n"));
                        write(out, els, depth + 1);
                    }
                    out.push_str(&format!("{pad}}}\n"));
                    continue;
                }
                CanonStmt::While(c, body) => {
                    out.push_str(&format!("{pad}while {} {{\n", c.text()));
                    write(out, body, depth + 1);
                    out.push_str(&format!("{pad}}}\n"));
                    continue;
                }
            };
            out.push_str(&format!("{pad}{line}\n"));
        }
    }
    let mut out = String::new();
    write(&mut out, body, 1);
    out
}

impl BinOp {
    /// The operator as the parse twin's map spells it.
    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div | Self::DivU => "/",
            Self::Rem | Self::RemU => "%",
            Self::Shl => "<<",
            // One operator in the source, so one spelling here: the canon text
            // is compared against the `.t1` parser's, which has no second kind.
            Self::Shr | Self::ShrL => ">>",
            Self::And => "&",
            Self::Or => "|",
            Self::Xor => "^",
            Self::Eq => "==",
            Self::Ne => "!=",
            // One operator in the source each, as for `>>` above.
            Self::Lt | Self::LtU => "<",
            Self::Gt | Self::GtU => ">",
            Self::Ge | Self::GeU => ">=",
        }
    }
}

/// Side A's conversion: this parser's tree to the twin's.
fn canon_expr(e: &Expr) -> CanonExpr {
    match e {
        Expr::Num(n) => CanonExpr::Leaf(n.to_string()),
        Expr::Str(bytes) => canon_string(&String::from_utf8_lossy(bytes)),
        Expr::Bool(true) => CanonExpr::Leaf(W_TRUE.to_string()),
        Expr::Bool(false) => CanonExpr::Leaf(W_FALSE.to_string()),
        Expr::Nil => CanonExpr::Leaf(W_NIL.to_string()),
        Expr::Var(n) => CanonExpr::Leaf(n.clone()),
        Expr::Call {
            module, name, args, ..
        } => {
            let callee = match module {
                Some(m) => format!("{m}\u{971}{name}"),
                None => name.clone(),
            };
            CanonExpr::Call(callee, args.iter().map(canon_expr).collect())
        }
        Expr::Bin(op, l, r) => CanonExpr::Bin(
            op.symbol(),
            Box::new(canon_expr(l)),
            Box::new(canon_expr(r)),
        ),
        Expr::Index(l, i) => CanonExpr::Index(Box::new(canon_expr(l)), Box::new(canon_expr(i))),
        Expr::Slice(l, a, b) => CanonExpr::Slice(
            Box::new(canon_expr(l)),
            Box::new(canon_expr(a)),
            Box::new(canon_expr(b)),
        ),
        Expr::Field(l, f) => CanonExpr::Field(Box::new(canon_expr(l)), f.clone()),
    }
}

fn canon_stmts(stmts: &[Stmt]) -> Vec<CanonStmt> {
    stmts
        .iter()
        .map(|s| match s {
            Stmt::Let { name, init, .. } => CanonStmt::Let(name.clone(), Some(canon_expr(init))),
            Stmt::Assign { target, value } => {
                CanonStmt::Assign(canon_expr(target), canon_expr(value))
            }
            Stmt::If { cond, then, els } => {
                CanonStmt::If(canon_expr(cond), canon_stmts(then), canon_stmts(els))
            }
            Stmt::While { cond, body } => CanonStmt::While(canon_expr(cond), canon_stmts(body)),
            Stmt::Return(e) => CanonStmt::Return(e.as_ref().map(canon_expr)),
            Stmt::Eval(e) => CanonStmt::Eval(canon_expr(e)),
        })
        .collect()
}

impl Interpreter {
    /// Load `.t1` sources, resolving their `समावेशः` embeds against `spec_root`.
    ///
    /// # Errors
    /// A source that does not lex, whose embed does not resolve, or whose
    /// top-level declarations cannot be read. A routine BODY that cannot be
    /// parsed is not an error here — see [`Routine::why_not`].
    pub fn load(sources: &[(&str, &str)], spec_root: &Path) -> Result<Self, RunError> {
        // Pass one: lex, resolve embeds, and read the top level of every
        // source, so that every routine's arity is known before any body is
        // parsed. It has to be: a call in this language is juxtaposition —
        // `अष्टकान्वेषणम् पाठ्यम् आरम्भः पाठ्यम् ॱ दैर्घ्य १०` — and nothing but
        // the arity says where the arguments stop.
        let mut heads = Vec::new();
        for (label, text) in sources {
            let tokens = crate::lex::lex_t1(text).map_err(|es| {
                RunError::new(format!(
                    "{label}: does not lex: {}",
                    es.first().map(ToString::to_string).unwrap_or_default()
                ))
            })?;
            let tokens = super::anita::resolve(tokens, spec_root).map_err(|es| {
                RunError::new(format!(
                    "{label}: embed: {}",
                    es.first().map(ToString::to_string).unwrap_or_default()
                ))
            })?;
            heads.push((*label, read_head(&tokens, label)?));
        }

        let mut sigs: HashMap<String, usize> = HashMap::new();
        let mut modules: Vec<String> = Vec::new();
        for (_, head) in &heads {
            modules.push(head.module.clone());
            for f in &head.fns {
                sigs.insert(f.name.clone(), f.params.len());
                sigs.insert(format!("{}\u{971}{}", head.module, f.name), f.params.len());
            }
        }
        // `W-358`: every loaded module's GLOBALS and `गणना` variants,
        // qualified — a qualified name the parser reads as a variable is a
        // legitimate data read when it is one of these, and the diagnostic
        // must not call it an unresolved routine.
        let data: HashSet<String> = heads
            .iter()
            .flat_map(|(_, h)| {
                h.globals
                    .iter()
                    .map(|(n, _, _)| n)
                    .chain(h.enum_variants.iter().map(|(n, _)| n))
                    .map(move |n| format!("{}\u{971}{n}", h.module))
            })
            .collect();

        let mut out = Vec::new();
        // Every `गणना` variant in every module, flat. Variants share one
        // namespace with globals, which is what the corpus already assumes:
        // `वस्तुसंज्ञा ॱ स्थापनम्` is compared against a bare `पाठ्यम्`.
        let mut enum_variants: Vec<(String, i128)> = Vec::new();
        for (label, head) in heads {
            let mut routines = Vec::new();
            let global_names: Vec<String> =
                head.globals.iter().map(|(n, _, _)| n.clone()).collect();
            for f in head.fns {
                let ctx = ParseCtx {
                    sigs: &sigs,
                    modules: &modules,
                    module: &head.module,
                    globals: &global_names,
                    imports: &head.imports,
                    data: &data,
                };
                let mut locals: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
                let mut parser = Parser::new(&f.body, ctx);
                let body = parser.block(&mut locals).map_err(|e| {
                    format!(
                        "{label}:{}: `{}`: {}{}",
                        f.line,
                        f.name,
                        e.reason,
                        parser.unresolved_note()
                    )
                });
                // `W-359`: `grows` and `guarded` are filled after every
                // module is loaded (`classify_growth`) — a tail call's callee
                // may be declared further down the source.
                let (grows, guarded) = (None, Vec::new());
                routines.push(Rc::new(Routine {
                    name: f.name,
                    module: head.module.clone(),
                    line: f.line,
                    params: f.params,
                    returns: f.returns,
                    public: f.public,
                    body,
                    grows,
                    guarded,
                }));
            }
            enum_variants.extend(head.enum_variants.iter().cloned());
            // Bound before `head.module` is moved into `Module::name` below —
            // the global initialisers parsed inside this block need to know
            // which module they belong to, for the same reason the bodies do.
            let module_name = head.module.clone();
            // An initialiser that does not parse is REFUSED, by name. It used
            // to be read as `०` — and `०` on an arena is "a fresh empty one",
            // so a global whose initialiser the parser could not read loaded
            // as a healthy empty store and no test could tell (`W-242`).
            let mut globals = Vec::with_capacity(head.globals.len());
            for (n, t, toks) in head.globals {
                let ctx = ParseCtx {
                    sigs: &sigs,
                    modules: &modules,
                    module: &module_name,
                    globals: &global_names,
                    imports: &head.imports,
                    data: &data,
                };
                let mut none = Vec::new();
                let e = Parser::new(&toks, ctx).expression(&mut none).map_err(|e| {
                    RunError::new(format!(
                        "{label}: `{module_name}\u{971}{n}`: its initialiser does not parse: {}",
                        e.reason
                    ))
                })?;
                globals.push((n, t, e));
            }
            out.push(Module {
                name: head.module,
                structs: head.structs,
                variants: head.enum_variants.into_iter().collect(),
                globals,
                routines,
                enums: head.enums,
                imports: head.imports,
                public: head.public,
                struct_decls: head.struct_decls,
            });
        }

        // ONE MODULE MAY BE DECLARED BY TWO SOURCES — `वास्तु`, by `ast.t1` and
        // `vastu.t1` (`W-225`). They are ONE module and are merged here, before
        // anything is keyed by name. Without this the LAST source to declare
        // the name owned `by_module` and the FIRST answered `struct_fields`'
        // qualified arm, so `वास्तुॱवस्तु` read from `संयोजन` was a record
        // under one load order and `Int(0)` under the other (`W-224`, measured
        // by `t1_modules.rs` before this was written).
        let mut merged: Vec<Module> = Vec::new();
        for m in out {
            if let Some(same) = merged.iter_mut().find(|e| e.name == m.name) {
                same.structs.extend(m.structs);
                same.globals.extend(m.globals);
                same.variants.extend(m.variants);
                same.routines.extend(m.routines);
                // `W-223`: the store's tables travel with the merge too.
                same.enums.extend(m.enums);
                same.imports.extend(m.imports);
                same.public.extend(m.public);
                same.struct_decls.extend(m.struct_decls);
            } else {
                merged.push(m);
            }
        }
        let mut out = merged;
        // `V-009` (ii): THE NATIVE RULE ON THE KERNEL'S NAMES, MIRRORED EXACTLY
        // (`W-381`: an engine does not run what the others refuse).
        for m in &out {
            refuse_a_kernel_name(m)?;
        }
        // `W-333`: after the merge, so a module declared by two sources is one
        // table of globals, and before anything clones a routine.
        mark_unsigned_operators(&mut out);

        let by_module = out
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name.clone(), i))
            .collect();
        let mut me = Self {
            modules: out,
            by_module,
            globals: HashMap::new(),
            fuel: 0,
            budget: 0,
            sink: Vec::new(),
            active: Vec::new(),
            float_runs: RefCell::new(HashMap::new()),
            float_runs_prune_at: std::cell::Cell::new(FLOAT_RUNS_PRUNE_FLOOR),
            narrow_runs: RefCell::new(HashMap::new()),
            events: None,
            threads: None,
            socket: None,
        };
        me.classify_growth();
        me.refuse_unbound_growth()?;
        // Every global is initialised by EVALUATING its initialiser — the same
        // `eval` a routine body's `चरः` goes through — or the load is refused
        // by name. `भवति ०` alone is read through the declared type, because
        // the corpus has no empty-collection literal and writes every arena
        // `अङ्कः अन्तः पाठांश भवति ०`. See `initial_value`.
        me.fuel = 1_000_000;
        for i in 0..me.modules.len() {
            let decls: Vec<(String, Ty, Expr)> = me.modules[i].globals.clone();
            for (name, ty, init) in decls {
                let v = me.initial_value(&name, &ty, &init, i)?;
                me.globals.insert(name, v);
            }
        }
        // ═══ THE EMBED STORE: the host's ONE contribution to embed resolution ═══
        //
        // `lex.t1` declares `समावेशनामकोश` / `समावेशपाठकोश` / `समावेशसीमाकोश` /
        // `समावेशसंख्या` and its own `पदविभाग` collapses `समावेशः आरभ्य <name>
        // समाप्तम्` into one string token FROM THESE ARENAS. The host fills them
        // here, at load, because `load` is the one place every driver shares —
        // `Front`, `shrinkhala.t1`'s `शृङ्खलाॱपठनम्`, and the census's own
        // `Interpreter::load`. A resolver that lived in `Front::lex` alone
        // passed four green readings and moved the census by nothing; the
        // product path never saw it. This fill is deliberately dumb: every
        // table the build knows, name, bytes, and byte count index for index,
        // so the `.t1` side reads no length (a `ॱ दैर्घ्य` in `lex.t1` would be
        // cause ३२ when `lex.t1` compiles itself). A table that does not read
        // is simply absent from the store, and the lexer leaves its four tokens
        // alone — the parser's embed node, `ir.t1`'s cause १२ stub, the marker
        // that already exists. Only when `lex.t1` is loaded: a chain without the
        // lexer has no store to fill and no one to read it.
        if me.globals.contains_key("समावेशसंख्या") {
            let mut names: Vec<Value> = Vec::new();
            let mut texts: Vec<Value> = Vec::new();
            let mut lens: Vec<Value> = Vec::new();
            for name in super::anita::table_names() {
                let Some(rel) = super::anita::table_path(name) else {
                    continue;
                };
                let Ok(bytes) = std::fs::read(spec_root.join(rel)) else {
                    continue;
                };
                names.push(Value::Octets(Octets::new(name.as_bytes())));
                lens.push(Value::Int(bytes.len() as i128));
                texts.push(Value::Octets(Octets::new(&bytes)));
            }
            let n = names.len() as i128;
            me.globals.insert(
                "समावेशनामकोश".to_string(),
                Value::Arena(Rc::new(RefCell::new(names))),
            );
            me.globals.insert(
                "समावेशपाठकोश".to_string(),
                Value::Arena(Rc::new(RefCell::new(texts))),
            );
            me.globals.insert(
                "समावेशसीमाकोश".to_string(),
                Value::Arena(Rc::new(RefCell::new(lens))),
            );
            me.globals.insert("समावेशसंख्या".to_string(), Value::Int(n));
        }
        // Variants LAST, and deliberately: a global of the same name is the
        // corpus's own declaration and wins over a variant this loader
        // synthesised. Nothing in the tree collides today, and if one ever
        // does the explicit declaration is the one a reader will find.
        //
        // A VARIANT TWO MODULES DECLARE WITH TWO DIFFERENT ORDINALS IS NOT A
        // BARE NAME. `अनिर्दिष्टम्` is `कोश ॱ संज्ञाखण्ड`'s fourth and `वास्तु ॱ
        // स्थापन`'s fifth; `or_insert` alone made the flat key whichever source
        // loaded first, and `W-224`'s order test saw `Int(3)` under one order
        // and `Int(4)` under another. Now neither is the bare key: a bare use
        // from a third module is refused as "not a name in scope" — the
        // paradigm's rule S3, a name in two places is two names, never a
        // shadow — while the declaring module's own bare use and a qualified
        // use from anywhere read the module's table in `lookup_global`. A
        // name every declaring module agrees on (`पाठ्यम्` is ० in both) stays
        // a bare key, since no order can change what it means.
        let mut agreed: HashMap<&str, Option<i128>> = HashMap::new();
        for (name, ordinal) in &enum_variants {
            agreed
                .entry(name.as_str())
                .and_modify(|o| {
                    if *o != Some(*ordinal) {
                        *o = None;
                    }
                })
                .or_insert(Some(*ordinal));
        }
        for (name, ordinal) in &enum_variants {
            if agreed.get(name.as_str()).copied().flatten().is_some() {
                me.globals
                    .entry(name.clone())
                    .or_insert(Value::Int(*ordinal));
            }
        }
        Ok(me)
    }

    /// Load the `.t1` sources at these paths.
    ///
    /// # Errors
    /// As [`Interpreter::load`], plus any file that cannot be read.
    pub fn load_paths(paths: &[&Path], spec_root: &Path) -> Result<Self, RunError> {
        let mut texts = Vec::new();
        for p in paths {
            let t = std::fs::read_to_string(p)
                .map_err(|e| RunError::new(format!("{}: {e}", p.display())))?;
            texts.push((p.display().to_string(), t));
        }
        let borrowed: Vec<(&str, &str)> = texts
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        Self::load(&borrowed, spec_root)
    }

    /// How many routines loaded and how many of them can be run.
    #[must_use]
    pub fn report(&self) -> Report {
        let routines: usize = self.modules.iter().map(|m| m.routines.len()).sum();
        let runnable = self
            .modules
            .iter()
            .flat_map(|m| &m.routines)
            .filter(|r| r.is_runnable())
            .count();
        Report { routines, runnable }
    }

    /// Every routine loaded, in declaration order.
    pub fn routines(&self) -> impl Iterator<Item = &Rc<Routine>> {
        self.modules.iter().flat_map(|m| m.routines.iter())
    }

    /// The routine of this name — qualified `module\u{971}name` or bare.
    #[must_use]
    pub fn routine(&self, name: &str) -> Option<&Rc<Routine>> {
        if let Some((m, n)) = name.split_once('\u{971}')
            && let Some(i) = self.by_module.get(m)
        {
            return self.modules[*i].routines.iter().find(|r| r.name == n);
        }
        self.routines().find(|r| r.name == name)
    }

    /// The current value of a `सार्वजनिक चरः` global — how a test reads the
    /// arena a reader filled.
    #[must_use]
    pub fn global(&self, name: &str) -> Option<&Value> {
        self.globals.get(name)
    }

    /// **THE HOST'S INPUT, FOR THE INTERPRETED HALF OF A RAM-INJECTED RUN.**
    ///
    /// The image has no input channel — exactly two SBI calls, and any other
    /// halts — so `yantra-run` places a file in RAM before the first
    /// instruction and re-points a declared run global at it. This is the same
    /// act for the interpreter, so a `--predict` and the native run are handed
    /// the SAME octets and their sinks can be compared.
    ///
    /// **REFUSES A NAME NO MODULE DECLARED**, returning `false` and changing
    /// nothing. A setter that inserted any name would let a misspelt global
    /// be "set" while the program read its own untouched `०` — an input that
    /// silently never arrived, which is the failure an input channel most
    /// needs to make loud.
    pub fn set_global(&mut self, name: &str, v: Value) -> bool {
        match self.globals.get_mut(name) {
            Some(slot) => {
                *slot = v;
                true
            }
            None => false,
        }
    }

    /// **THE EVENT LOG, FOR THE INTERPRETED HALF OF A REPLAY (`W-373`).**
    ///
    /// LOG REPLAY ONLY (owner ruling 2026-10-05): at wait `k` the interpreter
    /// writes `log[k]` into the program's event global and continues — the act
    /// `yantra::input::replay` performs natively at each `Halt::Wait`. Hand
    /// this the SAME `Vec<u64>` `yantra::input::parse_event_log` gave the
    /// native replay; live mode stays native-only.
    ///
    /// THE SLOT IS FOUND BY THE TAG'S VALUE, as the host finds it in RAM: each
    /// module's globals are scanned in declaration order for one holding
    /// [`EVENT_TAG`], and the slot is the global declared next. Refused, and
    /// nothing set, on `find_event_slot`'s three refusals for its reasons: no
    /// tag, a tag found twice, a tag with no global after it.
    ///
    /// THE RECORD IS 64 BITS, AND THE SLOT'S DECLARED TYPE READS THEM (review
    /// finding 1, 2026-10-05). Natively the host stores the word and the
    /// program's type reinterprets it: a `न६४` slot reads it unsigned, an
    /// `अ६४` slot as two's complement, a `प६४` slot as a double's bit pattern.
    /// Those three are delivered that way here (`EventSlot`); a slot of any
    /// other type is REFUSED, because this interpreter does not model a
    /// narrower or a composite slot's native reading exactly, and a delivery
    /// that only approximated it would be a two-engine green over a guess.
    ///
    /// # Errors
    /// One of the three refusals above, or a slot of another type, at load —
    /// before any wait runs.
    pub fn set_events(&mut self, log: &[u64]) -> Result<(), RunError> {
        let (slot, kind) = self.event_slot()?;
        self.events = Some(EventReplay {
            log: log.to_vec(),
            slot,
            kind,
            delivered: 0,
        });
        Ok(())
    }

    /// The `SASEVENT` slot and how its type reads the host's word — [`Interpreter::set_events`]'s
    /// checks, shared with [`Interpreter::set_socket`].
    fn event_slot(&self) -> Result<(String, EventSlot), RunError> {
        let hits = self.tagged_globals(EVENT_TAG);
        let slot = match hits.as_slice() {
            [] => {
                return Err(RunError::new(format!(
                    "no event interface in this program: the SASEVENT tag ({EVENT_TAG:#x}) is \
                     held by no global. It was built without the event global — declare it, or \
                     do not pass an event log"
                )));
            }
            [(_, Some(next))] => next.clone(),
            [(at, None)] => {
                return Err(RunError::new(format!(
                    "the SASEVENT tag at `{at}` has no global after it"
                )));
            }
            many => {
                return Err(RunError::new(format!(
                    "the SASEVENT tag appears at {} globals ({:?}); the tag must be unique or \
                     the scan cannot tell which global the program reads",
                    many.len(),
                    many.iter().map(|(at, _)| at).collect::<Vec<_>>()
                )));
            }
        };
        let (slot, ty) = slot;
        let kind = match &ty {
            Ty::Named(n) if n == T_U64 => EventSlot::Unsigned,
            Ty::Named(n) if n == T_I64 => EventSlot::Signed,
            Ty::Named(n) if n == T_F64 => EventSlot::Float,
            other => {
                return Err(RunError::new(format!(
                    "the event global `{slot}` is declared {other:?}: the host writes a 64-bit \
                     word there, and this interpreter delivers it only into a `{T_U64}`, `{T_I64}` \
                     or `{T_F64}` slot, whose native reading of those bits it reproduces exactly"
                )));
            }
        };
        Ok((slot, kind))
    }

    /// **THE SOCKET LOG, FOR THE INTERPRETED HALF OF A SOCKET REPLAY (`W-377`, option
    /// (b), owner ruling Q1).** The log IS the device model here, so nothing is invented:
    /// this keeps the native device's queue, latch and counts (`yantra::socket`), and at
    /// each wait applies ONE record and writes the octet count into the `SASEVENT` slot
    /// (`0` for `s=end`) — what `yantra::socket::replay_socket` does at each `Halt::Wait`.
    /// LOG REPLAY ONLY: live sockets stay native (W-373 Q4).
    ///
    /// With a socket log set — and only then — the device calls `अष्टकॱउपकरणचतुरष्टकाहारः`
    /// and `अष्टकॱउपकरणचतुरष्टकनिधानम्` are answered at the FOUR SOCK addresses
    /// ([`SOCK_NEXT`], [`SOCK_RX`], [`SOCK_TX`], [`SOCK_CLOSE`]), refusing by the native
    /// device's own words; every other address keeps `W-350`'s refusal. TX octets go to
    /// a second sink, [`Interpreter::socket_sent`].
    ///
    /// # Errors
    /// [`Interpreter::set_events`]'s slot refusals, at load.
    pub fn set_socket(&mut self, log: &[SockRecord]) -> Result<(), RunError> {
        let (slot, kind) = self.event_slot()?;
        self.socket = Some(SocketReplay {
            log: log.to_vec(),
            slot,
            kind,
            delivered: 0,
            queue: std::collections::VecDeque::new(),
            ended: false,
            latch: SOCK_UNREAD,
            received: 0,
            sent: Vec::new(),
        });
        Ok(())
    }

    /// The octets the program sent through the socket's TX, in order (`W-377`).
    #[must_use]
    pub fn socket_sent(&self) -> &[u8] {
        self.socket.as_ref().map_or(&[], |s| s.sent.as_slice())
    }

    /// Socket records the waits have taken, and octets received over them (`W-377`).
    #[must_use]
    pub fn socket_delivered(&self) -> (usize, u64) {
        self.socket
            .as_ref()
            .map_or((0, 0), |s| (s.delivered, s.received))
    }

    /// Every global holding `tag`, in declaration order, as `(moduleॱname, the global
    /// declared next)` — how the host's scan of RAM for a tag reads in this engine.
    fn tagged_globals(&self, tag: u64) -> Vec<(String, Option<(String, Ty)>)> {
        let tag = Value::Int(i128::from(tag));
        let mut hits = Vec::new();
        for m in &self.modules {
            for (k, (name, ..)) in m.globals.iter().enumerate() {
                if matches!(self.globals.get(name), Some(v) if *v == tag) {
                    let next = m.globals.get(k + 1).map(|(n, t, _)| (n.clone(), t.clone()));
                    hits.push((format!("{}\u{971}{name}", m.name), next));
                }
            }
        }
        hits
    }

    /// **`W-376`, OPTION (b): THREADS, AS A SERIAL SUBSET** (ADR-0040 addendum §4, owner
    /// ruling "All as recommended", 2026-10-05).
    ///
    /// This interpreter is a recursive tree-walker: pausing one thread inside a call to
    /// run another would mean suspending a Rust call stack. So it runs ONLY the schedules
    /// in which every record after a wait names the waiting thread — each thread from its
    /// start to its end, in `@` order — and REFUSES any other, naming the record index:
    /// such a schedule is the native engine's alone.
    ///
    /// The interface is the native one, found the native way, by value: the global after
    /// the `SASTHRDS` tag ([`THREADS_TAG`]) holds the count, 1 to [`MAX_THREADS`]; the
    /// global after `SASTHRID` ([`THREAD_ID_TAG`]), a `न६४`, is the thread-id slot; the
    /// `SASEVENT` interface must be declared too ([`Interpreter::set_events`]'s checks).
    /// At each decision the next record must be `@N` for a thread that has not run; N is
    /// written into the slot and `entry` is CALLED — its return is that thread's end and
    /// its value the thread's status, as the startup's finisher reports it natively. At
    /// each wait, the slot is written again (the host writes it at EVERY switch-in) and the
    /// value record after `@N` is delivered, read through the event slot's type.
    ///
    /// Answers each thread's `(number, returned value)` in the order the threads ended.
    ///
    /// # Errors
    /// Refused at load, before anything runs, on the native host's load refusals (a tag
    /// without its partner, a duplicated tag, a count of 0 or above 64, no event interface)
    /// or a slot that is not a `न६४`; refused while running on an interleaving schedule, a
    /// record that does not fit (an ended or out-of-range thread, a value where `@N` is due
    /// or the reverse), a short log, or — once every thread has ended — a long one; and on
    /// anything `entry` itself does that [`Interpreter::call`] refuses.
    pub fn run_threads(
        &mut self,
        entry: &str,
        schedule: &[ThreadStep],
        fuel: u64,
    ) -> Result<Vec<(u32, Value)>, RunError> {
        if self.socket.is_some() {
            return Err(RunError::new(
                "a threaded program with a socket log is refused in this row: the readiness \
                 wake waits for a multi-connection row (W-377, addendum §3)"
                    .to_string(),
            ));
        }
        let counts = self.tagged_globals(THREADS_TAG);
        let ids = self.tagged_globals(THREAD_ID_TAG);
        match (counts.is_empty(), ids.is_empty()) {
            (true, true) => {
                return Err(RunError::new(format!(
                    "no thread interface in this program: the SASTHRDS tag ({THREADS_TAG:#x}) is \
                     held by no global — declare it and its count, or run the program unthreaded"
                )));
            }
            (false, true) => {
                return Err(RunError::new(format!(
                    "the program declares SASTHRDS ({THREADS_TAG:#x}) without SASTHRID \
                     ({THREAD_ID_TAG:#x}): a threaded program must declare both"
                )));
            }
            (true, false) => {
                return Err(RunError::new(format!(
                    "the program declares SASTHRID ({THREAD_ID_TAG:#x}) without SASTHRDS \
                     ({THREADS_TAG:#x}): a threaded program must declare both"
                )));
            }
            (false, false) => {}
        }
        let one = |hits: &[(String, Option<(String, Ty)>)], what: &str| match hits {
            [(_, Some(next))] => Ok(next.clone()),
            [(at, None)] => Err(RunError::new(format!(
                "the {what} tag at `{at}` has no global after it"
            ))),
            many => Err(RunError::new(format!(
                "the {what} tag appears at {} globals ({:?}); the tag must be unique or the \
                 scan cannot tell which global the program reads",
                many.len(),
                many.iter().map(|(at, _)| at).collect::<Vec<_>>()
            ))),
        };
        let (count_global, _) = one(&counts, "SASTHRDS")?;
        let (id_slot, id_ty) = one(&ids, "SASTHRID")?;
        let count = match self.globals.get(&count_global) {
            Some(Value::Int(n)) if (1..=i128::from(MAX_THREADS)).contains(n) => *n as usize,
            other => {
                return Err(RunError::new(format!(
                    "the SASTHRDS count `{count_global}` is {other:?}: a program declares 1 to \
                     {MAX_THREADS} threads"
                )));
            }
        };
        if !matches!(&id_ty, Ty::Named(n) if n == T_U64) {
            return Err(RunError::new(format!(
                "the thread-id global `{id_slot}` is declared {id_ty:?}: the host writes the \
                 thread's number there as a `{T_U64}` word"
            )));
        }
        // The event interface, by `set_events`'s own checks; its log is unused here. The
        // state it replaces is RESTORED on return (review fix 7): a wait in a later call
        // reads whatever log was set before, or has no event source — never this empty one.
        let saved = self.events.take();
        if let Err(e) = self.set_events(&[]) {
            self.events = saved;
            return Err(RunError::new(format!(
                "a threaded program needs the event interface — {}",
                e.reason
            )));
        }
        self.threads = Some(ThreadReplay {
            schedule: schedule.to_vec(),
            cursor: 0,
            current: 0,
            id_slot: id_slot.clone(),
        });
        let result = self.run_serial(entry, count, fuel);
        self.threads = None;
        self.events = saved;
        result
    }

    /// [`Interpreter::run_threads`]'s loop, once the interface is checked.
    fn run_serial(
        &mut self,
        entry: &str,
        count: usize,
        fuel: u64,
    ) -> Result<Vec<(u32, Value)>, RunError> {
        let mut ended = vec![false; count];
        let mut ends = Vec::new();
        loop {
            let tr = self.threads.as_mut().expect("set by run_threads");
            let index = tr.cursor;
            if ended.iter().all(|e| *e) {
                if index < tr.schedule.len() {
                    return Err(RunError::new(format!(
                        "the log is LONGER than the run: every thread has ended and {} of {} \
                         records are unconsumed, from record index {index}; this log does not \
                         describe this run",
                        tr.schedule.len() - index,
                        tr.schedule.len()
                    )));
                }
                return Ok(ends);
            }
            let k = match tr.schedule.get(index) {
                None => {
                    return Err(RunError::new(format!(
                        "the log is SHORTER than the run: record index {index} is due at a \
                         decision point and the log holds {}; nothing was run on a decision \
                         nobody recorded",
                        tr.schedule.len()
                    )));
                }
                Some(ThreadStep::Value(v)) => {
                    return Err(RunError::new(format!(
                        "record index {index} is the value {v}, and a decision point is due: \
                         the next record must be `@N`, the thread to run"
                    )));
                }
                Some(ThreadStep::Run(k)) => *k,
            };
            if k as usize >= count {
                return Err(RunError::new(format!(
                    "record index {index} is `@{k}`, and the program declares {count} threads"
                )));
            }
            if ended[k as usize] {
                return Err(RunError::new(format!(
                    "record index {index} is `@{k}`, and thread {k} has ended"
                )));
            }
            tr.cursor += 1;
            tr.current = k;
            let slot = tr.id_slot.clone();
            self.globals.insert(slot, Value::Int(i128::from(k)));
            let v = self.call(entry, Vec::new(), fuel)?;
            ended[k as usize] = true;
            ends.push((k, v));
        }
    }

    /// One wait under [`Interpreter::run_threads`]: the next record must name the waiting
    /// thread, and a value must follow it.
    fn eval_thread_wait(&mut self) -> Result<Value, RunError> {
        let tr = self.threads.as_mut().expect("called under run_threads");
        let index = tr.cursor;
        let k = tr.current;
        match tr.schedule.get(index) {
            None => {
                return Err(RunError::new(format!(
                    "the log is SHORTER than the run: record index {index} is due at thread \
                     {k}'s wait and the log holds {} — the thread was NOT resumed",
                    tr.schedule.len()
                )));
            }
            Some(ThreadStep::Run(j)) if *j == k => {}
            Some(ThreadStep::Run(j)) => {
                return Err(RunError::new(format!(
                    "W-376: an INTERLEAVING schedule — record index {index} runs thread {j} \
                     while thread {k} waits. This interpreter runs threads SERIALLY, each from \
                     its start to its end (option (b), ADR-0040 addendum §4), and refuses a \
                     schedule that switches at a wait; it is the native engine's to run"
                )));
            }
            Some(ThreadStep::Value(v)) => {
                return Err(RunError::new(format!(
                    "record index {index} is the value {v}, and thread {k}'s wait is a \
                     decision point: the next record must be `@{k}`"
                )));
            }
        }
        let record = match tr.schedule.get(index + 1) {
            Some(ThreadStep::Value(v)) => *v,
            None => {
                return Err(RunError::new(format!(
                    "the log is SHORTER than the run: record index {} is thread {k}'s value \
                     and the log holds {} — the thread was NOT resumed",
                    index + 1,
                    tr.schedule.len()
                )));
            }
            Some(ThreadStep::Run(j)) => {
                return Err(RunError::new(format!(
                    "record index {} is `@{j}`, and thread {k} is resuming from a wait: its \
                     value record is due",
                    index + 1
                )));
            }
        };
        tr.cursor += 2;
        let id_slot = tr.id_slot.clone();
        // EVERY SWITCH-IN WRITES THE SLOT, a resume included.
        self.globals.insert(id_slot, Value::Int(i128::from(k)));
        let ev = self.events.as_ref().expect("checked by run_threads");
        let (slot, kind) = (ev.slot.clone(), ev.kind);
        self.globals.insert(slot, event_value(kind, record));
        Ok(Value::Int(0))
    }

    /// How many records the waits have taken — `Replayed::Halted`'s
    /// `delivered`. A caller holding a longer log judges it from this, as
    /// `yantra-run` does: a log not consumed exactly is a log of another run.
    #[must_use]
    pub fn events_delivered(&self) -> usize {
        self.events.as_ref().map_or(0, |e| e.delivered)
    }

    /// One wait: the next record into the event global, or the refusal the
    /// native runner gives in the same place.
    fn eval_wait(&mut self) -> Result<Value, RunError> {
        if self.threads.is_some() {
            return self.eval_thread_wait();
        }
        if self.socket.is_some() {
            return self.eval_socket_wait();
        }
        let Some(ev) = self.events.as_mut() else {
            // `yantra-run`'s exit 75: paused, not finished, and never resumed
            // onto an event nobody delivered.
            return Err(RunError::new(format!(
                "WAIT — `{WAIT_BUILTIN_MODULE}\u{971}{WAIT_BUILTIN_MEMBER}` asked the host to \
                 wait for the world (ADR-0040), and this interpreter has no event source (no \
                 log was set). The run is paused, not finished"
            )));
        };
        let Some(&record) = ev.log.get(ev.delivered) else {
            // `Replayed::Short`: never padded.
            return Err(RunError::new(format!(
                "the log is SHORTER than the run: wait index {} has no record; the log holds {} \
                 — the program was NOT resumed, rather than let it read a word nobody delivered",
                ev.delivered,
                ev.log.len()
            )));
        };
        ev.delivered += 1;
        let slot = ev.slot.clone();
        let v = event_value(ev.kind, record);
        self.globals.insert(slot, v);
        Ok(Value::Int(0))
    }

    /// One SOCKET wait (`W-377`): the next record applied to the queue and its octet count
    /// into the event global — or the native replay's refusal in the same place: a wait
    /// after END, or a short log (never padded).
    fn eval_socket_wait(&mut self) -> Result<Value, RunError> {
        let sock = self.socket.as_mut().expect("checked by eval_wait");
        if sock.ended {
            return Err(RunError::new(format!(
                "a WAIT after END: wait index {} waited after the peer closed (s=end), and there                  is no other source to wait for (W-377)",
                sock.delivered
            )));
        }
        let Some(record) = sock.log.get(sock.delivered).cloned() else {
            return Err(RunError::new(format!(
                "the socket log is SHORTER than the run: wait index {} has no record; the log                  holds {} — the program was NOT resumed, rather than let it read octets nobody                  delivered",
                sock.delivered,
                sock.log.len()
            )));
        };
        // THE SESSION CAP (`yantra::socket::check_cap`'s twin, in its words): the record that
        // would pass it is refused at its wait and never applied.
        if let SockRecord::Octets(o) = &record {
            let total = sock.received + o.len() as u64;
            if total > SOCK_SESSION_CAP {
                let (index, octets) = (sock.delivered, o.len());
                return Err(RunError::new(sock_over_cap(index, octets, total)));
            }
        }
        sock.delivered += 1;
        let n = match record {
            SockRecord::Octets(o) => {
                sock.queue.extend(o.iter().copied());
                sock.received += o.len() as u64;
                o.len() as u64
            }
            SockRecord::End => {
                sock.ended = true;
                0
            }
        };
        let (slot, kind) = (sock.slot.clone(), sock.kind);
        self.globals.insert(slot, event_value(kind, n));
        Ok(Value::Int(0))
    }

    /// A device call at one of the four SOCK addresses, with a socket log set (`W-377`):
    /// `store` is the write half. `None` for any other address — `W-350`'s refusal then
    /// stands. The words of every refusal are the native device's (`yantra::socket`).
    fn socket_call(&mut self, store: bool, vals: &[Value]) -> Option<Result<Value, RunError>> {
        let sock = self.socket.as_mut()?;
        let Some(Value::Int(addr)) = vals.first() else {
            return None;
        };
        let addr = u64::try_from(*addr).ok()?;
        if ![SOCK_NEXT, SOCK_RX, SOCK_TX, SOCK_CLOSE].contains(&addr) {
            return None;
        }
        let refuse = |why: &str| Some(Err(RunError::new(why.to_string())));
        if !store {
            return match addr {
                SOCK_RX => Some(Ok(Value::Int(i128::from(sock.latch)))),
                SOCK_CLOSE => refuse(SOCK_WHY_CLOSE),
                _ => refuse(SOCK_WHY_LOAD_STORE_ONLY),
            };
        }
        // The store writes the register's LOW 32 BITS natively (`sw`), so those are judged.
        let word = match vals.get(1) {
            Some(Value::Int(v)) => (*v as u64) & 0xffff_ffff,
            _ => return None,
        };
        match addr {
            SOCK_NEXT if word != 0 => refuse(SOCK_WHY_NEXT_VALUE),
            SOCK_NEXT => {
                sock.latch = match sock.queue.pop_front() {
                    Some(o) => u32::from(o),
                    None if sock.ended => SOCK_END,
                    None => SOCK_EMPTY,
                };
                Some(Ok(Value::Int(0)))
            }
            SOCK_TX if word > 0xff => refuse(SOCK_WHY_TX_VALUE),
            SOCK_TX => {
                sock.sent.push(word as u8);
                Some(Ok(Value::Int(0)))
            }
            SOCK_CLOSE => refuse(SOCK_WHY_CLOSE),
            _ => refuse(SOCK_WHY_STORE_LOAD_ONLY),
        }
    }

    /// **THE PARSE TWIN'S REFERENCE SIDE** (`docs/parse-twin-design.md`):
    /// every routine of `module` as THIS parser read it, in load order, each
    /// under a `वृत्तिः name` header line with its body printed by
    /// [`canon_text`]. A routine whose body did not parse prints
    /// `  <unparsed: why>` in place of a body. An unknown module prints
    /// nothing. Side B (`crates/sadhana-t1/tests/parse_twin.rs`) walks the
    /// `.t1` parser's arenas into the same [`CanonStmt`]s and prints them
    /// with the same printer, so a difference is a difference of TREES.
    #[must_use]
    pub fn canonical(&self, module: &str) -> String {
        let mut out = String::new();
        let Some(&i) = self.by_module.get(module) else {
            return out;
        };
        for r in &self.modules[i].routines {
            out.push_str(&format!("{W_FN} {}\n", r.name));
            match &r.body {
                Ok(body) => out.push_str(&canon_text(&canon_stmts(body))),
                Err(why) => out.push_str(&format!("  <unparsed: {why}>\n")),
            }
        }
        out
    }

    /// What is left of the budget the last [`Interpreter::call`] was given.
    ///
    /// **A GETTER AND NOTHING ELSE, and it is Rust because the counter is Rust's.**
    /// `call` sets `self.fuel` and every step decrements it, so `budget − this` is a
    /// DETERMINISTIC step count — the same on a loaded machine and an idle one,
    /// which a wall clock is not when four lanes share a box.
    ///
    /// The alternative was bisecting the budget: the least budget a stage completes
    /// under. That is a pure-fuel reading needing no new code and it costs THIRTY
    /// full compiles — written, run, and timed out at 900 s having produced nothing.
    #[must_use]
    pub fn fuel_remaining(&self) -> u64 {
        self.fuel
    }

    /// Every global as loading left it, `(name, value)` sorted by name — the
    /// state a run starts from, comparable across two loads (`W-224`).
    #[must_use]
    pub fn globals_snapshot(&self) -> Vec<(String, String)> {
        /// `Debug` of a record walks a `HashMap`, whose order is the map's
        /// and not the value's; this renders fields sorted so two equal
        /// records render equal.
        fn canonical(v: &Value) -> String {
            match v {
                Value::Record(r) => {
                    let mut fields: Vec<String> = r
                        .borrow()
                        .iter()
                        .map(|(k, v)| format!("{k}: {}", canonical(v)))
                        .collect();
                    fields.sort();
                    format!("{{{}}}", fields.join(", "))
                }
                Value::Arena(a) => format!(
                    "[{}]",
                    a.borrow()
                        .iter()
                        .map(canonical)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                other => format!("{other:?}"),
            }
        }
        let mut v: Vec<(String, String)> = self
            .globals
            .iter()
            .map(|(n, v)| (n.clone(), canonical(v)))
            .collect();
        v.sort();
        v
    }

    /// **Every place where what this interpreter answers depends on the ORDER
    /// the sources were loaded in** — `W-224`'s instrument, read off the
    /// loaded tree rather than assumed from the loader's code.
    ///
    /// The loader keeps flat tables beside the per-module ones, and each has
    /// a rule for a collision that names a load position:
    ///
    /// - a BARE call whose name the calling module does not declare: the parser
    ///   read its arity from `sigs`'s bare key (the LAST source to declare the
    ///   name wrote it) and `resolve_call` will run the FIRST loaded routine of
    ///   that name (`W-193` made the parser and the runtime agree on the
    ///   caller's own routine; a name the caller does not own is what is left);
    /// - a bare TYPE the declaring module does not own, declared as a struct by
    ///   two or more other modules: `struct_fields` takes the first loaded;
    /// - a GLOBAL declared by two modules: one flat key, the last loaded
    ///   initialises it (`W-192`'s flat namespace — the values coincide when
    ///   the declared types do, and the snapshot says whether they do).
    ///
    /// Two collisions the loader used to resolve by position are not on this
    /// list because `load` no longer does: a module declared by two sources is
    /// merged into one, and a `गणना` variant two modules declare with
    /// different ordinals has no bare key at all.
    ///
    /// Each entry is one line naming the module, the routine's line and the
    /// name. Empty means: no answer this interpreter gives can change with the
    /// order of the sources, whatever the sources are called.
    #[must_use]
    pub fn load_order_sensitive_sites(&self) -> Vec<String> {
        let mut out = Vec::new();

        // Bare calls a module does not own, and bare types it does not own.
        for m in &self.modules {
            let owns_routine = |n: &str| m.routines.iter().any(|r| r.name == n);
            let struct_owners = |n: &str| {
                self.modules
                    .iter()
                    .filter(|o| o.structs.contains_key(n))
                    .map(|o| o.name.as_str())
                    .collect::<Vec<_>>()
            };
            for r in &m.routines {
                let Ok(body) = &r.body else { continue };
                let mut calls = Vec::new();
                let mut types = Vec::new();
                walk_stmts(body, &mut calls, &mut types);
                for (module, name) in calls {
                    if module.is_none() && !owns_routine(name) {
                        out.push(format!(
                            "{}:{} bare call `{name}` — `{}` declares no routine of that \
                             name; the parser took the LAST loaded arity, the runtime \
                             takes the FIRST loaded routine",
                            m.name, r.line, m.name
                        ));
                    }
                }
                for ty in types {
                    for n in named_types(ty) {
                        if n.contains('\u{971}') || m.structs.contains_key(n) {
                            continue;
                        }
                        let owners = struct_owners(n);
                        if owners.len() > 1 {
                            out.push(format!(
                                "{}:{} bare type `{n}` — `{}` declares no struct of that \
                                 name and {owners:?} all do; `struct_fields` takes the \
                                 FIRST loaded",
                                m.name, r.line, m.name
                            ));
                        }
                    }
                }
            }
        }

        // Globals declared by two modules.
        let mut global_owners: HashMap<&str, Vec<&str>> = HashMap::new();
        for m in &self.modules {
            for (n, _, _) in &m.globals {
                global_owners.entry(n).or_default().push(&m.name);
            }
        }
        let mut shared: Vec<String> = global_owners
            .iter()
            .filter(|(_, owners)| owners.len() > 1)
            .map(|(n, owners)| {
                let mut owners = owners.clone();
                owners.sort_unstable();
                // Joined by hand: `Debug` of a `&str` escapes the virama and
                // the vowel signs, and a site is read by a person.
                format!(
                    "global `{n}` declared by [{}] — one flat key, the LAST loaded \
                     initialises it",
                    owners.join(", ")
                )
            })
            .collect();
        shared.sort();
        out.extend(shared);
        out
    }

    /// Call one routine of a loaded module by name, with arguments.
    ///
    /// `fuel` bounds the whole run: one unit per statement and per expression
    /// node. `संज्ञाकुलपठनम्` over the real mnemonic table costs about 40
    /// million, so a caller that means to run a table reader passes a large
    /// number and one that means to run `कोष्ठाङ्कः` passes a small one.
    ///
    /// # Errors
    /// No such routine; the wrong number of arguments; a body that did not
    /// parse; anything the body does that this module does not implement; or
    /// exhausted fuel.
    /// The octets `अष्टकॱमुद्रणम्` has written, in order.
    ///
    /// This is what a caller ASSERTS ON. Asserting on the intrinsic's return
    /// value or on an exit status would pass while nothing reached the sink at
    /// all — the one failure the marker body cannot make loud — so the
    /// acceptance for the output channel reads these bytes and nothing else.
    #[must_use]
    pub fn sink(&self) -> &[u8] {
        &self.sink
    }

    pub fn call(&mut self, name: &str, args: Vec<Value>, fuel: u64) -> Result<Value, RunError> {
        self.fuel = fuel;
        self.budget = fuel;
        let routine = self
            .routine(name)
            .ok_or_else(|| RunError::new(format!("no routine named `{name}` is loaded")))?
            .clone();
        self.invoke(&routine, args)
    }

    fn invoke(&mut self, routine: &Rc<Routine>, args: Vec<Value>) -> Result<Value, RunError> {
        if !*CALLS_ON.get_or_init(|| std::env::var_os("T1_CALLS").is_some()) {
            return self.invoke_inner(routine, args);
        }
        // With T1_CALLS: count the call and attribute STEPS — inclusive (the
        // routine and everything it called) and self (inclusive minus its
        // callees') — so a profile names where the steps are, not only which
        // routine is called most. The counter alone found the table walks;
        // it could not see a 350K-steps-per-instruction remainder.
        CALLS.with(|c| {
            *c.borrow_mut().entry(routine.name.clone()).or_insert(0) += 1;
        });
        let entry_fuel = self.fuel;
        STACK.with(|st| st.borrow_mut().push(0));
        NAMES.with(|n| n.borrow_mut().push(routine.name.clone()));
        let out = self.invoke_inner(routine, args);
        NAMES.with(|n| {
            n.borrow_mut().pop();
        });
        let inclusive = entry_fuel.saturating_sub(self.fuel);
        let children = STACK.with(|st| st.borrow_mut().pop().unwrap_or(0));
        STACK.with(|st| {
            if let Some(parent) = st.borrow_mut().last_mut() {
                *parent += inclusive;
            }
        });
        STEPS.with(|m| {
            let mut m = m.borrow_mut();
            let e = m.entry(routine.name.clone()).or_insert((0, 0));
            e.0 += inclusive;
            e.1 += inclusive.saturating_sub(children);
        });
        out
    }

    fn invoke_inner(&mut self, routine: &Rc<Routine>, args: Vec<Value>) -> Result<Value, RunError> {
        if args.len() != routine.params.len() {
            return Err(RunError::new(format!(
                "`{}` takes {} arguments, {} given",
                routine.name,
                routine.params.len(),
                args.len()
            )));
        }
        let body = routine
            .body
            .as_ref()
            .map_err(|e| RunError::new(e.clone()))?;
        let mut scope: Vec<Frame> = vec![Frame::new()];
        // `V-005`: an argument in the other register file from its parameter's
        // DECLARED type is refused, `FileMismatch` — as `ir.t1` refuses it at
        // the native call site; never moved by its bits.
        for ((n, ty), v) in routine.params.iter().zip(args) {
            refuse_file_mismatch(
                ty,
                &v,
                &format!("the argument for `{n}` of `{}`", routine.name),
            )?;
            scope[0].push((n.clone(), v));
        }
        let module = self.module_index(&routine.module);
        self.active.push(Rc::clone(routine));
        let ran = self.block(body, &mut scope, module);
        self.active.pop();
        match ran {
            // `V-005`: and a result in the other file from the declared RETURN
            // type is refused, `FileMismatch`.
            Ok(Flow::Return(v)) => {
                if let Some(ty) = &routine.returns {
                    refuse_file_mismatch(ty, &v, &format!("the result of `{}`", routine.name))?;
                }
                Ok(v)
            }
            Ok(Flow::Fall) => Ok(Value::Nil),
            // A `.t1` backtrace, innermost routine first: every frame the error
            // unwinds through appends its name, so a RunError names its site.
            Err(mut e) => {
                e.reason
                    .push_str(&format!("\n  in {}ॱ{}", routine.module, routine.name));
                Err(e)
            }
        }
    }

    /// `V-008` — how many entries the float-run table holds, live or not yet
    /// pruned: what `v008_float_memory.rs` asserts stays bounded.
    #[doc(hidden)]
    #[must_use]
    pub fn float_run_entries(&self) -> usize {
        self.float_runs.borrow().len()
    }

    /// `W-381` stage 3 — `a`'s element (bits, signed) when `zero_at` made it as a
    /// narrow integer run.
    fn narrow_run(&self, a: &Rc<RefCell<Vec<Value>>>) -> Option<(u32, bool)> {
        let table = self.narrow_runs.borrow();
        let (w, bits, signed) = table.get(&(Rc::as_ptr(a) as usize))?;
        let run = w.upgrade()?;
        Rc::ptr_eq(&run, a).then_some((*bits, *signed))
    }

    /// `V-008` — whether `a` is a run `zero_at` made for an `अङ्कः अन्तः प६४`.
    fn is_float_run(&self, a: &Rc<RefCell<Vec<Value>>>) -> bool {
        self.float_runs
            .borrow()
            .get(&(Rc::as_ptr(a) as usize))
            .and_then(std::rc::Weak::upgrade)
            .is_some_and(|run| Rc::ptr_eq(&run, a))
    }

    /// `V-008` part 2 — `व्यूहॱ<op> ( फल , क , ख )`: `फल[i] = क[i] <op> ख[i]`
    /// for every `i` below the length, answering the element count. The native
    /// expansion's twin as a plain loop, with the same refusals in the same
    /// order:
    ///
    /// 1. every argument must be a run of `प६४` (`is_float_run`), or the one
    ///    named cause `FileMismatch` — `ir.t1` refuses the same call at build;
    /// 2. the three lengths must be equal (a fresh run is length ०), or the
    ///    call is refused with [`VECTOR_LENGTH_REFUSAL`] before any element is
    ///    written — never truncated to the shortest.
    ///
    /// Each element is ONE binary64 operation under RNE with a NaN result
    /// canonicalised — exactly the scalar built-in's (`eval_float_builtin`), so
    /// a strip computes the bits the scalar loop would. The result run may be
    /// an operand run (in place): element `i` is read before it is written, and
    /// no other element is read after it.
    fn eval_vector_builtin(
        &self,
        b: FloatBuiltin,
        name: &str,
        vals: &[Value],
    ) -> Result<Value, RunError> {
        let qualified = format!("{VECTOR_BUILTIN_MODULE}\u{971}{name}");
        let mut runs: Vec<Rc<RefCell<Vec<Value>>>> = Vec::with_capacity(VECTOR_BUILTIN_ARITY);
        for (k, v) in vals.iter().enumerate() {
            match v {
                Value::Arena(a) if self.is_float_run(a) => runs.push(Rc::clone(a)),
                _ => {
                    return Err(file_mismatch_error(&format!(
                        "argument {} of `{qualified}`, a run of `{T_F64}`,",
                        k + 1
                    )));
                }
            }
        }
        if runs.len() != VECTOR_BUILTIN_ARITY {
            return Err(RunError::new(format!(
                "`{qualified}` takes {VECTOR_BUILTIN_ARITY} runs and was handed {}",
                runs.len()
            )));
        }
        let lens = [
            runs[0].borrow().len(),
            runs[1].borrow().len(),
            runs[2].borrow().len(),
        ];
        if lens[0] != lens[1] || lens[0] != lens[2] {
            return Err(vector_length_error(&qualified, lens));
        }
        let n = lens[0];
        for i in 0..n {
            // Both reads end before the write borrows: the result run may BE
            // an operand run.
            let x = runs[1].borrow()[i].clone();
            let y = runs[2].borrow()[i].clone();
            let r = eval_float_builtin(b, name, &[x, y])?;
            runs[0].borrow_mut()[i] = r;
        }
        Ok(Value::Int(i128::try_from(n).unwrap_or(i128::MAX)))
    }

    /// `V-009` part (ii) — `व्यूहॱआव्यूहगुणनम् ( फल , क , ख , M , K , N , L )`,
    /// `व्यूहॱव्युत्क्रमः ( फल , क , M , N , L )` and `व्यूहॱसमासः ( फल , क , ख , B ,
    /// M , K , N , L )`: the native kernel's twin (docs/v009-matrix-design.md),
    /// with the same refusals in the same order:
    ///
    /// 1. the last argument must be a layout constant (read as TEXT, never
    ///    evaluated), the runs runs of `प६४` and the dimensions integers, or the
    ///    one named cause `FileMismatch` — `ir.t1` refuses the same at build;
    /// 2. COLUMN-MAJOR IS THE PERMUTATION `ir.t1` makes: the product swaps its
    ///    operand runs and M with N, the transpose M with N; everything after
    ///    is row-major;
    /// 3. every dimension, and each product of two, must be below 2^32 (so no
    ///    product wraps), and the three lengths must be what the shape says, or
    ///    [`VECTOR_LENGTH_REFUSAL`];
    /// 4. an empty result answers ० and writes nothing;
    /// 5. a result run that IS an operand run is refused, [`REFUSAL_ADHYASA`].
    ///
    /// Each element of a product starts from +0.0 and adds `x · y` for `k` in
    /// order — two roundings, no fused multiply-add — through the scalar
    /// built-ins themselves (`eval_float_builtin`), exactly the kernel's
    /// `vfmul.vv` then `vfadd.vv` per lane. A transpose moves the bits.
    fn eval_matrix_builtin(
        &mut self,
        op: MatrixOp,
        args: &[Expr],
        scope: &mut Vec<Frame>,
        module: usize,
    ) -> Result<Value, RunError> {
        let qualified = format!("{VECTOR_BUILTIN_MODULE}\u{971}{}", op.member());
        let layout = match args.last() {
            Some(Expr::Call {
                module: m,
                name,
                args: none,
                vector: true,
                ..
            }) if none.is_empty() => layout_constant(m.as_deref(), name),
            _ => None,
        };
        // `परिवर्तितम्` reads a product's first operand transposed; a
        // transpose takes no such layout (`ir.t1` refuses it the same way).
        let layout = layout.filter(|l| op != MatrixOp::Transpose || *l != Layout::Transposed);
        let Some(layout) = layout else {
            return Err(file_mismatch_error(&format!(
                "argument {} of `{qualified}`, a layout constant,",
                args.len()
            )));
        };
        let column_major = layout == Layout::ColumnMajor;
        let run_count = if op == MatrixOp::Transpose { 2 } else { 3 };
        let mut runs: Vec<Rc<RefCell<Vec<Value>>>> = Vec::with_capacity(run_count);
        let mut dims: Vec<u64> = Vec::with_capacity(4);
        for (k, a) in args[..args.len().saturating_sub(1)].iter().enumerate() {
            let v = self.eval(a, scope, module)?;
            match v {
                Value::Arena(r) if k < run_count && self.is_float_run(&r) => runs.push(r),
                // The register's sixty-four bits, as the kernel reads them.
                Value::Int(i) if k >= run_count => dims.push(i as u64),
                _ => {
                    let what = if k < run_count {
                        format!("a run of `{T_F64}`")
                    } else {
                        "an integer".to_string()
                    };
                    return Err(file_mismatch_error(&format!(
                        "argument {} of `{qualified}`, {what},",
                        k + 1
                    )));
                }
            }
        }
        let len = |r: &Rc<RefCell<Vec<Value>>>| r.borrow().len() as u64;
        let shape = |lens: &[u64]| {
            RunError::new(format!(
                "`{qualified}` over runs whose lengths {lens:?} disagree with its shape {dims:?}: \
                 refused (V-009, finisher word {VECTOR_LENGTH_REFUSAL:#x})"
            ))
        };
        let adhyasa = || {
            RunError::new(format!(
                "`{qualified}`'s result run is one of its operand runs: refused (V-009, \
                 finisher word {REFUSAL_ADHYASA:#x})"
            ))
        };
        const WIDE: u64 = 1 << 32;
        if op == MatrixOp::Transpose {
            let (c, a) = (&runs[0], &runs[1]);
            let (mut m, mut n) = (dims[0], dims[1]);
            if column_major {
                std::mem::swap(&mut m, &mut n);
            }
            if (m | n) >= WIDE || m * n != len(c) || m * n != len(a) {
                return Err(shape(&[len(c), len(a)]));
            }
            if m * n == 0 {
                return Ok(Value::Int(0));
            }
            if Rc::ptr_eq(c, a) {
                return Err(adhyasa());
            }
            let (m, n) = (m as usize, n as usize);
            for j in 0..n {
                for i in 0..m {
                    let x = a.borrow()[i * n + j].clone();
                    c.borrow_mut()[j * m + i] = x;
                }
            }
            return Ok(Value::Int((m * n) as i128));
        }
        let (c, mut a, mut b) = (&runs[0], &runs[1], &runs[2]);
        let (batch, rest) = if op == MatrixOp::Tensor {
            (dims[0], &dims[1..])
        } else {
            (1, &dims[..])
        };
        let (mut m, k, mut n) = (rest[0], rest[1], rest[2]);
        if column_major {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut m, &mut n);
        }
        let lens = [len(c), len(a), len(b)];
        if (batch | m | k | n) >= WIDE
            || ((m * k) | (k * n) | (m * n)) >= WIDE
            || m * k * batch != lens[1]
            || k * n * batch != lens[2]
            || m * n * batch != lens[0]
        {
            return Err(shape(&lens));
        }
        let count = m * n * batch;
        if count == 0 {
            return Ok(Value::Int(0));
        }
        if Rc::ptr_eq(c, a) || Rc::ptr_eq(c, b) {
            return Err(adhyasa());
        }
        let (batch, m, k, n) = (batch as usize, m as usize, k as usize, n as usize);
        let (mul, add) = (FloatBuiltin::Mul, FloatBuiltin::Add);
        for t in 0..batch {
            for i in 0..m {
                for j in 0..n {
                    let mut s = Value::Float(0);
                    for kk in 0..k {
                        // `परिवर्तितम्`: A is stored K×M, so A[i, k] is at k·M + i.
                        let ai = if layout == Layout::Transposed {
                            t * m * k + kk * m + i
                        } else {
                            (t * m + i) * k + kk
                        };
                        let x = a.borrow()[ai].clone();
                        let y = b.borrow()[(t * k + kk) * n + j].clone();
                        let p = eval_float_builtin(mul, &qualified, &[x, y])?;
                        s = eval_float_builtin(add, &qualified, &[s, p])?;
                    }
                    c.borrow_mut()[(t * m + i) * n + j] = s;
                }
            }
        }
        Ok(Value::Int(i128::from(count)))
    }

    fn module_index(&self, name: &str) -> usize {
        self.by_module.get(name).copied().unwrap_or(0)
    }

    /// The value a `भवति ०` initialiser means for a declared type.
    ///
    /// The corpus has no empty-collection literal and no struct literal, so `०`
    /// is what every arena and every record is written with — `सार्वजनिक चरः
    /// पाठांशकोश ॱॱ अङ्कः अन्तः पाठांश भवति ०` and `चरः नव ॱॱ पाठांश भवति ०`.
    /// The TYPE is what says which of the three it means.
    fn zero_of(&self, ty: &Ty, module: usize) -> Result<Value, RunError> {
        self.zero_at(ty, module, 0)
    }

    /// The value a `सार्वजनिक चरः` global holds when loading is done.
    ///
    /// `भवति ०` is the corpus's "a fresh empty one" and is read through the
    /// TYPE (`zero_of`): an arena, a record, the empty run, or the number ०.
    /// Every other initialiser is EVALUATED, by the same `eval` a routine
    /// body's `चरः` goes through, so a literal means the same thing at the top
    /// of a module as it does one line down inside a routine.
    ///
    /// IT DID NOT, FOR THE WHOLE LIFE OF `vakyavibhaga.t1` (`W-242`). The
    /// loader used to initialise every global whose type was not an integer
    /// from the type ALONE and never read the initialiser, so `सार्वजनिक चरः
    /// इतिशब्दः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् इति इति इति ।` — the closing
    /// word every string literal is matched against — was the EMPTY run, and
    /// every literal the T1 reader met was P22 "unclosed". Nothing reported
    /// it: an empty run is exactly what a healthy, not-yet-filled arena looks
    /// like, and the file's own margin had already explained the declaration
    /// as one that "could not have been written until W-185". It was written,
    /// and it was still empty. `W-239` found it by wiring the reader end to
    /// end and worked around it by filling the word octet by octet; this is
    /// the fix, and that workaround is reverted with it.
    ///
    /// AND WHAT THE LOADER CANNOT EVALUATE IT REFUSES, BY NAME. An initialiser
    /// that calls a routine or reads a name would make the global's value
    /// depend on what else had loaded and in which order — the dependence
    /// `W-224` removed from the loader — and the integer-typed path used to
    /// answer `unwrap_or(०)` for one, which is the same silence as the empty
    /// run in another spelling. The paradigm's rule S3: a marker resolves or
    /// is refused, never the silent empty value.
    fn initial_value(
        &mut self,
        name: &str,
        ty: &Ty,
        init: &Expr,
        module: usize,
    ) -> Result<Value, RunError> {
        if matches!(init, Expr::Num(0)) {
            return self.zero_of(ty, module);
        }
        let label = format!("{}\u{971}{name}", self.modules[module].name);
        if let Some(why) = not_a_load_time_value(init) {
            return Err(RunError::new(format!(
                "`{label}`: its initialiser {why}, which the loader does not evaluate at load — \
                 a global is initialised from a literal (a numeral, a string, `{W_TRUE}` or \
                 `{W_FALSE}`), or from `०` for a fresh record or arena"
            )));
        }
        let mut empty: Vec<Frame> = vec![Frame::new()];
        let v = self.eval(init, &mut empty, module).map_err(|e| {
            RunError::new(format!(
                "`{label}`: its initialiser could not be evaluated at load: {}",
                e.reason
            ))
        })?;
        // `V-008`: AN INITIALISER IS A STORE INTO THE GLOBAL'S DECLARED FILE — a
        // `प६४` global is never laid with an integer's bits (`भवति ०`, above, is
        // its typed zero, +0.0), as `ir.t1` refuses the same declaration.
        refuse_file_mismatch(ty, &v, &format!("the global `{label}`"))?;
        Ok(v)
    }

    fn zero_at(&self, ty: &Ty, module: usize, depth: usize) -> Result<Value, RunError> {
        if depth > 8 {
            // A struct that contains itself is not writable in this language
            // and would be an infinite record here. Stop rather than hang.
            return Ok(Value::Nil);
        }
        Ok(match ty {
            Ty::Slice(inner) => match &**inner {
                Ty::Named(n) if n == T_U8 => Value::Octets(Octets::new(&[])),
                // EMPTY, length ० (`W-355`): until 2026-10-03 this was one
                // `Nil` — a hole at index ० that the compiled side reproduced
                // as a COUNT (`len + (len == ०)`) but not as a slot, so the two
                // engines agreed on the loop bound and disagreed on whether
                // index ० was readable. Native reads a fresh run as the nil
                // word, length ०, and refuses an index into it; so does this.
                // `V-008`: a run of `प६४` is recorded as one, so a store into it
                // can be held to its declared file while it is still empty.
                Ty::Named(n) if n == T_F64 => {
                    let run = Rc::new(RefCell::new(vec![]));
                    let mut table = self.float_runs.borrow_mut();
                    table.insert(Rc::as_ptr(&run) as usize, Rc::downgrade(&run));
                    if table.len() >= self.float_runs_prune_at.get() {
                        table.retain(|_, w| w.strong_count() > 0);
                        self.float_runs_prune_at
                            .set((2 * table.len()).max(FLOAT_RUNS_PRUNE_FLOOR));
                    }
                    drop(table);
                    Value::Arena(run)
                }
                // `W-381` stage 3: a narrow integer run is recorded with its
                // element's width and signedness (`narrow_runs`).
                Ty::Named(n) if narrow_width(n).is_some() => {
                    let run = Rc::new(RefCell::new(vec![]));
                    let (bits, signed) = narrow_width(n).expect("matched above");
                    let mut table = self.narrow_runs.borrow_mut();
                    if table.len() >= FLOAT_RUNS_PRUNE_FLOOR.max(2 * self.float_runs_prune_at.get())
                    {
                        table.retain(|_, (w, ..)| w.strong_count() > 0);
                    }
                    table.insert(
                        Rc::as_ptr(&run) as usize,
                        (Rc::downgrade(&run), bits, signed),
                    );
                    drop(table);
                    Value::Arena(run)
                }
                _ => Value::Arena(Rc::new(RefCell::new(vec![]))),
            },
            Ty::Named(n) => match self.struct_fields(n, module) {
                Some(fields) => {
                    let mut map = HashMap::new();
                    for (f, fty) in fields {
                        map.insert(f.clone(), self.zero_at(fty, module, depth + 1)?);
                    }
                    // Top-level only: natively a record-typed FIELD is nil until
                    // assigned, so the nested records this zero builds eagerly are
                    // allocations the native never makes (measured 2026-09-14:
                    // counting them priced 213 MB where native shows ~79).
                    if depth == 0 {
                        bump_record(map.len());
                    }
                    Value::Record(Rc::new(RefCell::new(map)))
                }
                None => self.zero_of_name(n, module)?,
            },
            Ty::Optional(_) => Value::Nil,
            _ => Value::Int(0),
        })
    }

    /// `W-267`: THE ZERO OF A NAMED TYPE THAT IS NOT A `संरचना` — or a REFUSAL.
    ///
    /// Until 2026-10-03 every such name answered `Value::Int(0)`, so a typo'd type
    /// in a `भवति ०` declaration bound an integer silently (the `वास्तुॱअभिव्यञ्जक`
    /// failure was this shape) while the T1 twin refuses it (`artha.t1` seeds
    /// `दोषार्थः`). Now a name is zeroed only if it is one of:
    /// - a grammar scalar, `grammar-t1.ebnf:226`/`:248` — `Int(0)`, unchanged;
    /// - TEXT, `पाठ` and the retired `पाठः` the corpus still writes 14 times — the
    ///   EMPTY TEXT, exactly as `अङ्कः अन्तः अ८` is above, not `Int(0)`;
    /// - a declared `गणना` — `Int(0)`, its FIRST variant, because ordinals are
    ///   zero-based by position (where `enum_variants` are pushed). The row's first
    ///   prescription left enums out, and three are written in type position in
    ///   the corpus (`लक्ष्य`, `संज्ञाखण्ड`, `स्थापन`), one of them a field of the
    ///   record the object reader zeroes for every symbol.
    ///
    /// Anything else is refused BY NAME.
    fn zero_of_name(&self, n: &str, module: usize) -> Result<Value, RunError> {
        if n == W_TEXT || n == W_TEXT_RETIRED {
            return Ok(Value::Octets(Octets::new(&[])));
        }
        // `V-005`: a fresh `प६४` is +0.0 — all sixty-four bits clear, which is
        // also what the native frame slot holds after `भवति ०` stores the
        // integer ० into it.
        if n == T_F64 {
            return Ok(Value::Float(0));
        }
        if is_scalar_type_name(n) || self.enum_declared(n, module) {
            return Ok(Value::Int(0));
        }
        Err(RunError::new(format!(
            "`{n}` is not a type: no `संरचना` or `गणना` declares it and it is not one of \
             the language's own (`अ`/`न` with a width, `प३२`, `प६४`, `बूल`, `अक्षरम्`, \
             `पाठ`) — so `भवति ०` has no zero to give it, and inventing `०` would bind \
             a number where a typo was meant (W-267)"
        )))
    }

    /// Whether `name` is a `गणना` — QUALIFIED OR NOT, resolved exactly as
    /// [`Self::struct_fields`] resolves a struct.
    fn enum_declared(&self, name: &str, module: usize) -> bool {
        let has = |m: &Module, n: &str| m.enums.iter().any(|(e, _)| e == n);
        if let Some(m) = self.modules.get(module)
            && has(m, name)
        {
            return true;
        }
        if let Some((owner, bare)) = name.split_once('\u{971}')
            && let Some(m) = self.modules.iter().find(|m| m.name == owner)
            && has(m, bare)
        {
            return true;
        }
        self.modules.iter().any(|m| has(m, name))
    }

    /// The fields of a struct, by name — QUALIFIED OR NOT.
    ///
    /// THE QUALIFIED ARM WAS MISSING AND IT COST A SILENT `Int(0)`. A type
    /// written `वास्तुॱअभिव्यञ्जक` was looked up verbatim, matched no struct
    /// registered as `अभिव्यञ्जक` inside module `वास्तु`, and `zero_of` fell
    /// through to its integer case. So `चरः नव ॱॱ वास्तुॱअभिव्यञ्जक भवति ०`
    /// bound an INTEGER, and the next line — `नव ॱ भेद भवति भेद` — failed with
    /// "`ॱ भेद` written on Int(0), which is not a record", pointing at the
    /// write rather than at the declaration that had already gone wrong.
    ///
    /// Every `…योजनम्` that constructs a record of ANOTHER module's type hit
    /// this: `व्याकरॱअभिव्यञ्जकयोजनम्` (parse.t1:99) and `व्याकरॱवाक्ययोजनम्`
    /// among them, which is every way a test can build an AST node. The
    /// same-module case worked, which is why the corpus ran at all and why
    /// this stayed open — `encode.t1`'s appenders name their own types.
    ///
    /// The strip is `global_key`'s, on the member mark ADR-0027 froze, and it
    /// is guarded the same way: a prefix is only stripped when it names a
    /// module that is actually loaded, so a struct whose own name contains the
    /// mark is not silently truncated.
    fn struct_fields(&self, name: &str, module: usize) -> Option<&Vec<(String, Ty)>> {
        if let Some(m) = self.modules.get(module)
            && let Some(f) = m.structs.get(name)
        {
            return Some(f);
        }
        if let Some((owner, bare)) = name.split_once('\u{971}')
            && let Some(m) = self.modules.iter().find(|m| m.name == owner)
            && let Some(f) = m.structs.get(bare)
        {
            return Some(f);
        }
        self.modules.iter().find_map(|m| m.structs.get(name))
    }

    fn burn(&mut self, n: u64) -> Result<(), RunError> {
        if self.fuel < n {
            return Err(RunError::new(
                "execution exceeded its fuel; a `यावत्` did not terminate, or the caller gave too little",
            ));
        }
        self.fuel -= n;
        // A RUNNING PROFILE. With `T1_CALLS` set, every 100M steps the top
        // invocation counts go to stderr with the steps so far — so a routine
        // whose count grows faster than the steps names a superlinear term
        // minutes into a run that would otherwise report only at its end.
        if *CALLS_ON.get_or_init(|| std::env::var_os("T1_CALLS").is_some())
            && (self.fuel + n) / 100_000_000 != self.fuel / 100_000_000
        {
            let mut counts = call_counts();
            counts.sort_by(|a, b| b.1.cmp(&a.1));
            eprintln!("profile: steps {}", self.budget - self.fuel);
            for (name, k) in counts.iter().take(12) {
                eprintln!("profile:   {k:>12} {name}");
            }
        }
        Ok(())
    }
}

enum Flow {
    Fall,
    Return(Value),
}

/// One lexical scope's names, in declaration order.
///
/// An association list rather than a map, and measured: a T1 routine's frame
/// holds two to six names, and hashing a thirty-octet Devanagari word cost more
/// than walking six of them. `the_register_reader_answers_the_number_the_table_carries`
/// — 67 calls of `कोष्ठाङ्कः` over the whole register table, `dev` profile —
/// went from 9.09 s to 4.05 s on this change alone.
/// The type names that are unsigned integers — `artha.t1`'s `मूलप्रकारार्थः`,
/// the arms that set `पूर्णाङ्कार्थभेद` with `चिह्नितम्` `असत्यम्`.
///
/// A COPY OF THAT LIST, AND SAID TO BE ONE. The native side reads the bit off
/// the checker's own record; this side has only the spelling. So
/// `w333_shift_mark.rs` reads the names out of `artha.t1` and compares them
/// with this array, and an unsigned type added there reds that test instead
/// of making the two engines disagree about it.
///
/// **SEVEN, AND THE TEST IS WHY IT IS NOT FIVE.** This array was written as the
/// five `न` widths, from the letter. The comparison's first run answered
/// `["न८", "न१६", "न३२", "न६४", "न१२८", "बूल", "अक्षरम्"]`: `मूलप्रकारार्थः`
/// also makes `बूल` and `अक्षरम्` integers with `चिह्नितम्` `असत्यम्`, so a name
/// of either type is shifted LOGICALLY in the image. With five here the
/// interpreter would have shifted those two arithmetically — the engines
/// parting on a declared type, which is the defect `W-333` exists to remove.
pub const UNSIGNED_INTEGER_TYPES: [&str; 7] = ["न८", "न१६", "न३२", "न६४", "न१२८", "बूल", "अक्षरम्"];

fn declared_unsigned(ty: &Ty) -> bool {
    matches!(ty, Ty::Named(n) if UNSIGNED_INTEGER_TYPES.contains(&n.as_str()))
}

/// What a load-time walk knows about the names in scope at a shift.
struct ShiftScope<'a> {
    /// Innermost last, and within a frame in declaration order — the shape of
    /// the run-time `Vec<Frame>`, holding a signedness where that holds a value.
    frames: Vec<Vec<(String, bool)>>,
    /// The enclosing module's `गणना` variants: a variant is read BEFORE any
    /// global (`lookup_global`) and is never an unsigned name.
    variants: &'a HashMap<String, i128>,
    /// This module's globals.
    own: &'a HashMap<String, bool>,
    /// Every module's globals, by module name, for `मॱग`.
    by_module: &'a HashMap<String, HashMap<String, bool>>,
    /// Every module's variants, by module name, for `मॱग`.
    variants_of: &'a HashMap<String, HashMap<String, i128>>,
    /// The flat table: a bare name some OTHER module declares. `None` when
    /// two modules declare it with different signedness — the run-time store
    /// has one slot for both (`global_key`'s KNOWN LIMIT), so neither
    /// declaration can be called the name's.
    flat: &'a HashMap<String, Option<bool>>,
}

impl ShiftScope<'_> {
    /// Whether `name` is declared unsigned, by the rule `eval`'s `Expr::Var`
    /// arm and `slot_mut` read a name with: the innermost frame that binds it,
    /// the LAST declaration in that frame, and only then the module level.
    fn unsigned(&self, name: &str) -> bool {
        for frame in self.frames.iter().rev() {
            if let Some((_, u)) = frame.iter().rev().find(|(n, _)| n == name) {
                return *u;
            }
        }
        if self.variants.contains_key(name) {
            return false;
        }
        if let Some((m, n)) = name.split_once('\u{971}')
            && let Some(globals) = self.by_module.get(m)
        {
            if self.variants_of.get(m).is_some_and(|v| v.contains_key(n)) {
                return false;
            }
            return globals
                .get(n)
                .copied()
                .unwrap_or_else(|| self.flat.get(n).copied().flatten().unwrap_or(false));
        }
        self.own
            .get(name)
            .copied()
            .unwrap_or_else(|| self.flat.get(name).copied().flatten().unwrap_or(false))
    }

    fn expr(&self, e: &mut Expr) {
        match e {
            Expr::Bin(op, l, r) => {
                self.expr(l);
                self.expr(r);
                // A NAME, and nothing else: not a call that returns unsigned,
                // not a member, not an index. `ir.t1`'s `अचिह्नितनामसरणम्`
                // answers the same question from the same fact. A grouped
                // name needs no arm here — this tree has no group node, so
                // `आरभ्य क समाप्तम्` is already `Var(क)`.
                if *op == BinOp::Shr
                    && let Expr::Var(name) = &**l
                    && self.unsigned(name)
                {
                    *op = BinOp::ShrL;
                }
                // `W-381` stage 3: an ORDERING comparison with a NAME declared
                // unsigned on EITHER side compares unsigned — `ir.t1` asks
                // `अचिह्नितनामसरणम्` of both operands. Either side, because a
                // comparison has no "left operand" in the shift's sense: `क
                // न्यूनम् ०` and `० अधिकम् क` are one question.
                let unsigned_name = |e: &Expr| matches!(e, Expr::Var(name) if self.unsigned(name));
                // Ruling (b): division and remainder by the same rule.
                if matches!(
                    op,
                    BinOp::Lt | BinOp::Gt | BinOp::Ge | BinOp::Div | BinOp::Rem
                ) && (unsigned_name(l) || unsigned_name(r))
                {
                    *op = match op {
                        BinOp::Lt => BinOp::LtU,
                        BinOp::Gt => BinOp::GtU,
                        BinOp::Ge => BinOp::GeU,
                        BinOp::Div => BinOp::DivU,
                        _ => BinOp::RemU,
                    };
                }
            }
            Expr::Call { args, .. } => {
                for a in args {
                    self.expr(a);
                }
            }
            Expr::Index(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            Expr::Slice(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            Expr::Field(base, _) => self.expr(base),
            Expr::Num(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Nil | Expr::Var(_) => {}
        }
    }

    /// One block: a frame of its own, as `Interpreter::block` opens one.
    fn block(&mut self, stmts: &mut [Stmt]) {
        self.frames.push(Vec::new());
        for s in stmts {
            match s {
                // The initialiser is read BEFORE the name is bound, as
                // `statement` evaluates it before pushing — `चरः क ॱॱ न६४
                // भवति क दक्षिणसृ १` shifts the OUTER `क`.
                Stmt::Let { name, ty, init } => {
                    self.expr(init);
                    let unsigned = declared_unsigned(ty);
                    self.frames
                        .last_mut()
                        .expect("a frame is open")
                        .push((name.clone(), unsigned));
                }
                Stmt::Assign { target, value } => {
                    self.expr(value);
                    self.expr(target);
                }
                Stmt::If { cond, then, els } => {
                    self.expr(cond);
                    self.block(then);
                    self.block(els);
                }
                Stmt::While { cond, body } => {
                    self.expr(cond);
                    self.block(body);
                }
                Stmt::Return(Some(e)) | Stmt::Eval(e) => self.expr(e),
                Stmt::Return(None) => {}
            }
        }
        self.frames.pop();
    }
}

/// **`W-333` — MARK EVERY `दक्षिणसृ` WHOSE LEFT OPERAND IS A NAME DECLARED
/// UNSIGNED, ONCE, AT LOAD** — and, since `W-381` stage 3, every ordering
/// comparison with such a name on either side (`BinOp::LtU`, `GtU`, `GeU`).
///
/// `eval` has two integers and no types, so the choice between the arithmetic
/// and the logical shift cannot be made there. It CAN be made here: a routine's
/// parameters, every `चरः` in its body and every module's globals carry their
/// declared type in the tree, and which declaration a name denotes at a given
/// expression is fixed by the same scope rule the run-time lookup applies.
///
/// No `Frame` changes and nothing is consulted at run time: the mark is the
/// operator itself, `BinOp::Shr` rewritten to `BinOp::ShrL` in place (and
/// `Lt`/`Gt`/`Ge` to their unsigned three).
///
/// A routine whose body did not parse is left alone — it refuses when called.
fn mark_unsigned_operators(modules: &mut [Module]) {
    let by_module: HashMap<String, HashMap<String, bool>> = modules
        .iter()
        .map(|m| {
            (
                m.name.clone(),
                m.globals
                    .iter()
                    .map(|(n, t, _)| (n.clone(), declared_unsigned(t)))
                    .collect(),
            )
        })
        .collect();
    let variants_of: HashMap<String, HashMap<String, i128>> = modules
        .iter()
        .map(|m| (m.name.clone(), m.variants.clone()))
        .collect();
    let mut flat: HashMap<String, Option<bool>> = HashMap::new();
    for globals in by_module.values() {
        for (n, u) in globals {
            flat.entry(n.clone())
                .and_modify(|seen| {
                    if *seen != Some(*u) {
                        *seen = None;
                    }
                })
                .or_insert(Some(*u));
        }
    }
    for m in modules.iter_mut() {
        let own = &by_module[&m.name];
        let variants = &variants_of[&m.name];
        let scope = |frames| ShiftScope {
            frames,
            variants,
            own,
            by_module: &by_module,
            variants_of: &variants_of,
            flat: &flat,
        };
        for (_, _, init) in &mut m.globals {
            scope(Vec::new()).expr(init);
        }
        for r in &mut m.routines {
            // Just built and not yet shared: `load` calls this before any
            // clone of the `Rc`. A shared one is skipped rather than copied,
            // and `w333` asserts the marks landed, so a skip cannot be silent.
            let Some(r) = Rc::get_mut(r) else { continue };
            let params = r
                .params
                .iter()
                .map(|(n, t)| (n.clone(), declared_unsigned(t)))
                .collect();
            if let Ok(body) = &mut r.body {
                scope(vec![params]).block(body);
            }
        }
    }
}

type Frame = Vec<(String, Value)>;

/// `W-381` stage 3: a narrow run's table entry — the run, its element's bits,
/// and whether the element is signed.
type NarrowRun = (std::rc::Weak<RefCell<Vec<Value>>>, u32, bool);

/// `W-274`: UNUSED TODAY. It gives `Frame` — a type alias for `Vec<(String,
/// Value)>` — a `new()` of its own, which every call site currently spells as
/// `Vec::new()` instead. Kept because it names the frame's construction as a
/// frame operation rather than a vector one, which is the distinction the
/// margin above this alias is making. Named here instead of hidden by the
/// directory-wide allow; delete it if the alias ever becomes a real type.
#[allow(dead_code)]
trait FrameNew {
    fn new() -> Self;
}
impl FrameNew for Frame {
    fn new() -> Self {
        Vec::new()
    }
}

impl Interpreter {
    fn block(
        &mut self,
        stmts: &[Stmt],
        scope: &mut Vec<Frame>,
        module: usize,
    ) -> Result<Flow, RunError> {
        scope.push(Frame::new());
        let mut flow = Flow::Fall;
        for s in stmts {
            match self.statement(s, scope, module)? {
                Flow::Fall => {}
                Flow::Return(v) => {
                    flow = Flow::Return(v);
                    break;
                }
            }
        }
        scope.pop();
        Ok(flow)
    }

    fn statement(
        &mut self,
        stmt: &Stmt,
        scope: &mut Vec<Frame>,
        module: usize,
    ) -> Result<Flow, RunError> {
        self.burn(1)?;
        match stmt {
            Stmt::Let { name, ty, init } => {
                // `चरः नव ॱॱ पाठांश भवति ०` is the corpus's record literal: the
                // ० is not the number, it is "a fresh one". Same for an arena.
                let v = match (ty, init) {
                    (Ty::Named(_) | Ty::Slice(_), Expr::Num(0)) => {
                        let z = self.zero_of(ty, module)?;
                        if matches!(z, Value::Int(_)) {
                            self.eval(init, scope, module)?
                        } else {
                            z
                        }
                    }
                    _ => self.eval(init, scope, module)?,
                };
                // `V-005` — FAIL CLOSED (review ruling 2026-10-05): a value in the
                // other register file from the local's DECLARED type is refused,
                // `FileMismatch`, as `ir.t1` refuses it natively — never moved by
                // its bits. `भवति ०` on a `प६४` is the typed zero above (+0.0),
                // not an integer, on both engines.
                refuse_file_mismatch(ty, &v, &format!("the local `{name}`"))?;
                scope
                    .last_mut()
                    .expect("a scope is always open")
                    .push((name.clone(), v));
                Ok(Flow::Fall)
            }
            Stmt::Assign { target, value } => {
                let v = self.eval(value, scope, module)?;
                self.assign(target, v, scope, module)?;
                Ok(Flow::Fall)
            }
            Stmt::If { cond, then, els } => {
                if self.eval(cond, scope, module)?.truth()? {
                    self.block(then, scope, module)
                } else {
                    self.block(els, scope, module)
                }
            }
            Stmt::While { cond, body } => {
                while self.eval(cond, scope, module)?.truth()? {
                    self.burn(1)?;
                    if let Flow::Return(v) = self.block(body, scope, module)? {
                        return Ok(Flow::Return(v));
                    }
                }
                Ok(Flow::Fall)
            }
            Stmt::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e, scope, module)?,
                    None => Value::Nil,
                };
                Ok(Flow::Return(v))
            }
            Stmt::Eval(e) => {
                self.eval(e, scope, module)?;
                Ok(Flow::Fall)
            }
        }
    }

    /// The key a global is stored under, given a name that may be qualified.
    ///
    /// `पदविभागॱचिह्नकसूचकाङ्क` names the same store as the bare
    /// `चिह्नकसूचकाङ्क`: `globals` is ONE map keyed by the bare name. Without
    /// this, a qualified reference to a loaded global reported "is not a name
    /// in scope", which is the least true thing it could have said.
    ///
    /// The same shape as the unspaced call in `postfix`: the corpus qualifies
    /// and the interpreter did not un-qualify. `वाक्यविभाग` reads the lexer's
    /// token count this way, so `सङ्कलनम्` — the one routine that exists to
    /// cross that boundary — could not.
    ///
    /// KNOWN LIMIT, and the module header already records it for arities: two
    /// modules declaring a global of the same name share one slot. Qualifying
    /// the key would fix that and would also make every BARE reference in the
    /// corpus miss, so it is a corpus-wide change and not this function's.
    fn global_key<'a>(&self, name: &'a str) -> &'a str {
        match name.split_once('\u{971}') {
            Some((m, n)) if self.by_module.contains_key(m) => n,
            _ => name,
        }
    }

    /// A module-level name read from inside `module`: the module's OWN
    /// `गणना` variant first, then — for `कोशॱअनिर्दिष्टम्` — the named
    /// module's variant, then the flat table of globals and agreed variants.
    ///
    /// The order is what makes a variant module-scoped (`W-224`): the same
    /// bare word in two modules' `गणना` means each module's own ordinal
    /// inside that module, and a third module has to say which it means.
    fn lookup_global(&self, name: &str, module: usize) -> Option<Value> {
        if let Some(o) = self.modules.get(module).and_then(|m| m.variants.get(name)) {
            return Some(Value::Int(*o));
        }
        if let Some((m, n)) = name.split_once('\u{971}')
            && let Some(i) = self.by_module.get(m)
        {
            if let Some(o) = self.modules[*i].variants.get(n) {
                return Some(Value::Int(*o));
            }
            return self.globals.get(n).cloned();
        }
        self.globals.get(name).cloned()
    }

    /// The storage slot a plain name denotes — the innermost frame that binds
    /// it, else the module's global — for a write that must not go through a
    /// cloned value.
    fn slot_mut<'a>(&'a mut self, name: &str, scope: &'a mut [Frame]) -> Option<&'a mut Value> {
        if scope
            .iter()
            .rev()
            .any(|frame| frame.iter().rev().any(|(n, _)| n == name))
        {
            return scope
                .iter_mut()
                .rev()
                .find_map(|frame| frame.iter_mut().rev().find(|(n, _)| n == name))
                .map(|(_, slot)| slot);
        }
        let key = self.global_key(name).to_string();
        self.globals.get_mut(&key)
    }

    fn assign(
        &mut self,
        target: &Expr,
        v: Value,
        scope: &mut Vec<Frame>,
        module: usize,
    ) -> Result<(), RunError> {
        match target {
            Expr::Var(name) => {
                for frame in scope.iter_mut().rev() {
                    if let Some((_, slot)) = frame.iter_mut().rev().find(|(n, _)| n == name) {
                        // `V-005`: an assignment obeys the local's DECLARED file
                        // too — a `प६४` local holds a float from its binding on,
                        // and any other local never does (`Stmt::Let`).
                        if matches!(slot, Value::Float(_)) != matches!(v, Value::Float(_)) {
                            return Err(file_mismatch_error(&format!("the local `{name}`")));
                        }
                        *slot = v;
                        return Ok(());
                    }
                }
                let key = self.global_key(name).to_string();
                if let Some(slot) = self.globals.get_mut(&key) {
                    // `V-008`: a global obeys its DECLARED file as a local does —
                    // a `प६४` global holds a float from its load on (`भवति ०` is
                    // +0.0), and any other global never does.
                    if matches!(slot, Value::Float(_)) != matches!(v, Value::Float(_)) {
                        return Err(file_mismatch_error(&format!("the global `{name}`")));
                    }
                    *slot = v;
                    return Ok(());
                }
                Err(RunError::new(format!("`{name}` is not a name in scope")))
            }
            Expr::Field(base, field) => match self.eval(base, scope, module)? {
                Value::Record(r) => {
                    // `V-008`: a field obeys its DECLARED file. Every field of a
                    // record is built by `zero_at` from its declaration — a
                    // `प६४` one as +0.0 — and every write since was held to the
                    // same rule, so the value in the slot says the file.
                    let float_field = matches!(r.borrow().get(field), Some(Value::Float(_)));
                    if float_field != matches!(v, Value::Float(_)) {
                        return Err(file_mismatch_error(&format!("the field `{field}`")));
                    }
                    r.borrow_mut().insert(field.clone(), v);
                    Ok(())
                }
                other => Err(RunError::new(format!(
                    "`\u{971} {field}` written on {other:?}, which is not a record"
                ))),
            },
            Expr::Index(base, idx) => {
                let i = self.eval(idx, scope, module)?.int()?;
                // `W-359` rule (2): A STORE PAST THE END OF A GUARDED PARAMETER
                // IS REFUSED, BY NAME. In a routine that does not return a run
                // parameter, growing it cannot reach the caller natively — the
                // grown block's base is written back only into the callee's own
                // slot — so the store is refused here exactly where the native
                // lowering writes `प्राचलसीमानिषेधः` (`0x359`) to the finisher.
                // Within the length the store is untouched; locals, globals and
                // the parameter a routine grows and returns keep the old path.
                if let Expr::Var(name) = &**base
                    && let Some(r) = self.guarded_parameter(name, scope)
                {
                    let len = match self.slot_mut(name, scope) {
                        Some(Value::Arena(a)) => a.borrow().len(),
                        Some(Value::Octets(o)) => o.len(),
                        _ => 0,
                    };
                    if !usize::try_from(i).is_ok_and(|i| i < len) {
                        return Err(RunError::new(format!(
                            "entry {i} is past the end of the parameter `{name}`, a run of \
                             {len}, and `{}` does not return it (W-359)",
                            r.name
                        )));
                    }
                }
                // `W-381` STAGE 4 (the review's finding 1): A STORE AT OR PAST
                // THE LARGEST RUN IS REFUSED, BY NAME, before anything grows —
                // `ir.t1`'s `खण्डप्रविष्टिसीमा`, the same 2^25 entries for every
                // run, checked natively before the need is multiplied. Without it
                // a store at 10^12 asked the HOST for terabytes and aborted it,
                // while the native image faulted `BeyondRam`.
                if i >= i128::from(MAX_RUN_ENTRIES) {
                    return Err(RunError::new(format!(
                        "entry {i} is past the largest run, {MAX_RUN_ENTRIES} entries \
                         (W-381, finisher word {OUT_OF_BOUNDS_WRITE_REFUSAL:#x})"
                    )));
                }
                // IN PLACE WHEN NOBODY ELSE HOLDS THE BUFFER. The copy-on-write
                // arm below rebuilt the whole byte store on EVERY write, so an
                // appender like `अष्टकयोजनम्` — one byte per call, 194,726 calls
                // for ashtaka.t1's build alone — cost O(n²) memcpy and n
                // allocations of up to n bytes: native time the step meter
                // never sees, 44x ashtaka's CPU for 4.4x its instructions on
                // lex.t1 (2026-09-13). When the target is a plain name whose slot
                // holds the whole buffer and no alias, `Rc::make_mut` writes
                // through without a copy; when it IS shared, `make_mut` copies
                // exactly as before, so the value semantics of `भवति` hold. The
                // slot is reached directly rather than through `eval`, because
                // `eval` clones the value out of storage and that clone would
                // be the second holder.
                if let Expr::Var(name) = &**base
                    && let Value::Int(byte) = &v
                    && let Some(Value::Octets(o)) = self.slot_mut(name, scope)
                    && o.start == 0
                    && o.end == o.data.len()
                {
                    let i = usize::try_from(i).map_err(|_| {
                        RunError::new(format!(
                            "octet index {i} is not an index (W-381, finisher word \
                                 {OUT_OF_BOUNDS_WRITE_REFUSAL:#x})"
                        ))
                    })?;
                    // Owner ruling (a), 2026-10-06: the LOW EIGHT BITS, as the
                    // native `sb` stores them — never a refusal.
                    let byte = octet_of(*byte);
                    if *CALLS_ON.get_or_init(|| std::env::var_os("T1_CALLS").is_some()) {
                        let shared = Rc::strong_count(&o.data) > 1;
                        CALLS.with(|c| {
                            *c.borrow_mut()
                                .entry(if shared {
                                    "<octet write: in place, COPIED (shared)>".into()
                                } else {
                                    "<octet write: in place>".into()
                                })
                                .or_insert(0) += 1;
                        });
                    }
                    let data = Rc::make_mut(&mut o.data);
                    if data.len() <= i {
                        bump_model(data.as_ptr() as usize, data.len(), i + 1, 1);
                        data.resize(i + 1, 0);
                    }
                    data[i] = byte;
                    o.end = data.len();
                    return Ok(());
                }
                match self.eval(base, scope, module)? {
                    Value::Arena(a) => {
                        let i = usize::try_from(i).map_err(|_| {
                            RunError::new(format!(
                                "arena index {i} is not an index (W-381, finisher word \
                                 {OUT_OF_BOUNDS_WRITE_REFUSAL:#x})"
                            ))
                        })?;
                        // `V-008`: an element obeys its run's DECLARED file — a
                        // run of `प६४` takes only a float, any other run never
                        // one — and a float run grows with +0.0, the sixty-four
                        // clear bits the native block holds, not with `Nil`.
                        let float_run = self.is_float_run(&a);
                        // `W-381` stage 3 (owner rulings 2026-10-06): a narrow
                        // run keeps the low bits, held as its load extends them.
                        let v = match (&v, self.narrow_run(&a)) {
                            (Value::Int(x), Some((bits, signed))) => {
                                Value::Int(narrow_value(*x, bits, signed))
                            }
                            _ => v,
                        };
                        if float_run != matches!(v, Value::Float(_)) {
                            return Err(file_mismatch_error(&format!(
                                "an element of `{}`",
                                match &**base {
                                    Expr::Var(n) => n.as_str(),
                                    _ => "<expression>",
                                }
                            )));
                        }
                        // `W-381` STAGE 4, OWNER RULING 2026-10-06 (gap reads,
                        // option A): "Interpreter reads unwritten slots as 0.
                        // Matches native execution behavior and avoids requiring
                        // breaking source modifications across compiler tables."
                        // So a run of INTEGERS grows with `Int(0)` — the clear
                        // word the native block holds — as an octet run grows
                        // with ० and a float run with +0.0. A gap slot is INSIDE
                        // the length, so reading it is NOT refused: it reads ०.
                        // An integer element being stored is what says the run
                        // is one of integers; any other run (records, runs of
                        // runs) still grows with `Nil`.
                        let mut a = a.borrow_mut();
                        if a.len() <= i {
                            bump_model(a.as_ptr() as usize, a.len(), i + 1, 8);
                            a.resize(
                                i + 1,
                                if float_run {
                                    Value::Float(0)
                                } else if matches!(v, Value::Int(_)) {
                                    Value::Int(0)
                                } else {
                                    Value::Nil
                                },
                            );
                        }
                        a[i] = v;
                        Ok(())
                    }
                    // A BYTE SLICE IS ALSO WRITTEN BY INDEX, and the corpus
                    // does it: `दत्तक्षेत्रआज्ञाकूटः` (`वाक्यविभाग`:915) is
                    // `अङ्कः अन्तः अ८`, and `:957`–`:959` fill it with ८०, ४८,
                    // ४९ — the octets of `P01`. `zero_of` gives that type an
                    // empty `Octets`, so before this arm every such write
                    // failed and the whole diagnostic table was unreachable.
                    //
                    // COPY-ON-WRITE, because `Octets` is a shared view — an
                    // `Rc<Vec<u8>>` with a range — and the same buffer is
                    // handed out as slices of the source text. Mutating in
                    // place would write through every alias of it. Assigning a
                    // fresh run and storing it back through `base` keeps a
                    // byte slice a VALUE, which is what `भवति` means everywhere
                    // else in this language.
                    Value::Octets(o) => {
                        let i = usize::try_from(i).map_err(|_| {
                            RunError::new(format!(
                                "octet index {i} is not an index (W-381, finisher word \
                                 {OUT_OF_BOUNDS_WRITE_REFUSAL:#x})"
                            ))
                        })?;
                        // `V-008`: an octet run is an integer run.
                        if matches!(v, Value::Float(_)) {
                            return Err(file_mismatch_error("an element of an octet run"));
                        }
                        let mut bytes = o.as_slice().to_vec();
                        if bytes.len() <= i {
                            bytes.resize(i + 1, 0);
                        }
                        let byte = v.int()?;
                        bytes[i] = octet_of(byte);
                        self.assign(base, Value::Octets(Octets::new(&bytes)), scope, module)
                    }
                    // NAME THE TARGET. Without it this says only that
                    // SOMETHING was neither arena nor octets, and every arena
                    // in the corpus is written `भवति ०` — so the reader is told
                    // the shape is wrong and not which store has it.
                    other => Err(RunError::new(format!(
                        "indexed assignment to `{}`, which holds {other:?} and \
                         is neither an arena nor a byte slice",
                        match &**base {
                            Expr::Var(n) => n.as_str(),
                            _ => "<expression>",
                        }
                    ))),
                }
            }
            other => Err(RunError::new(format!(
                "{other:?} is not something that can be assigned to"
            ))),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn eval(
        &mut self,
        expr: &Expr,
        scope: &mut Vec<Frame>,
        module: usize,
    ) -> Result<Value, RunError> {
        self.burn(1)?;
        match expr {
            Expr::Num(n) => Ok(Value::Int(wrap(*n))),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Nil => Ok(Value::Nil),
            Expr::Str(s) => Ok(Value::Octets(Octets {
                data: Rc::clone(s),
                start: 0,
                end: s.len(),
            })),
            Expr::Var(name) => {
                for frame in scope.iter().rev() {
                    if let Some((_, v)) = frame.iter().rev().find(|(n, _)| n == name) {
                        return Ok(v.clone());
                    }
                }
                self.lookup_global(name, module)
                    .ok_or_else(|| RunError::new(format!("`{name}` is not a name in scope")))
            }
            Expr::Bin(op, l, r) => {
                let a = self.eval(l, scope, module)?;
                let b = self.eval(r, scope, module)?;
                if matches!(op, BinOp::Eq | BinOp::Ne) {
                    note_zero_against_nil(&self.modules[module].name, l, r, &a, &b);
                }
                binop(*op, &a, &b)
            }
            Expr::Field(base, field) => {
                let b = self.eval(base, scope, module)?;
                match (&b, field.as_str()) {
                    (Value::Octets(o), W_LEN) => Ok(Value::Int(o.len() as i128)),
                    (Value::Arena(a), W_LEN) => Ok(Value::Int(a.borrow().len() as i128)),
                    (Value::Record(r), _) => r.borrow().get(field).cloned().ok_or_else(|| {
                        RunError::new(format!("no record carries a member `{field}`"))
                    }),
                    // The base is spelled beside the member: a nil read names
                    // which chain answered nothing, not only what was asked of it.
                    _ => Err(RunError::new(format!(
                        "`\u{971} {field}` read from {b:?}, which has no such member (base {base:?})"
                    ))),
                }
            }
            Expr::Index(base, idx) => {
                let b = self.eval(base, scope, module)?;
                let i = self.eval(idx, scope, module)?.int()?;
                match b {
                    Value::Octets(o) => {
                        let i = usize::try_from(i)
                            .ok()
                            .filter(|i| *i < o.len())
                            .ok_or_else(|| {
                                RunError::new(format!(
                                    "octet {i} is outside a run of {} octets",
                                    o.len()
                                ))
                            })?;
                        Ok(Value::Int(i128::from(o.as_slice()[i])))
                    }
                    Value::Arena(a) => {
                        let a = a.borrow();
                        let i = usize::try_from(i)
                            .ok()
                            .filter(|i| *i < a.len())
                            .ok_or_else(|| {
                                RunError::new(format!(
                                    "entry {i} is outside an arena of {}",
                                    a.len()
                                ))
                            })?;
                        Ok(a[i].clone())
                    }
                    other => Err(RunError::new(format!("{other:?} cannot be indexed"))),
                }
            }
            Expr::Slice(base, from, to) => {
                let b = self.eval(base, scope, module)?;
                let f = self.eval(from, scope, module)?.int()?;
                let t = self.eval(to, scope, module)?.int()?;
                let Value::Octets(o) = b else {
                    return Err(RunError::new(format!("{b:?} cannot be sliced")));
                };
                let f = usize::try_from(f).unwrap_or(usize::MAX);
                let t = usize::try_from(t).unwrap_or(usize::MAX);
                if f > t || t > o.len() {
                    return Err(RunError::new(format!(
                        "the slice {f}..{t} is outside a run of {} octets",
                        o.len()
                    )));
                }
                Ok(Value::Octets(Octets {
                    data: Rc::clone(&o.data),
                    start: o.start + f,
                    end: o.start + t,
                }))
            }
            Expr::Call {
                module: m,
                name,
                args,
                vector,
                wait,
            } => {
                // `V-009` (ii) — A MATRIX BUILT-IN, before its arguments are
                // evaluated: the last is the layout constant, read as text.
                if *vector && let Some(op) = matrix_builtin(m.as_deref(), name) {
                    return self.eval_matrix_builtin(op, args, scope, module);
                }
                if *vector && layout_constant(m.as_deref(), name).is_some() {
                    return Err(file_mismatch_error(&format!(
                        "`{VECTOR_BUILTIN_MODULE}\u{971}{name}`, a layout constant read only as \
                         a matrix built-in's last argument,"
                    )));
                }
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a, scope, module)?);
                }
                // `W-373` — THE WAIT, by the flag only its one-token parse path
                // sets. The argument is evaluated (above) and ignored: it is the
                // word the native lowering stores at WAIT, reserved for `W-377`.
                if *wait {
                    return self.eval_wait();
                }
                // THE OUTPUT INTRINSIC, INTERCEPTED BEFORE `resolve_call`.
                //
                // `अष्टकॱमुद्रणम्` is declared in `ashtaka.t1` with a body that is
                // never meant to run: natively `ir.t1` replaces the call with a
                // store at the device address, and here the interpreter collects
                // the octet instead. The body returns the device address itself
                // (268435456) precisely so that a caller can tell — loudly, and
                // without knowing any of this — that BOTH interceptions are gone.
                // A body returning ० would be indistinguishable from success.
                //
                // THE QUALIFIED FORM IS REQUIRED. An unqualified `मुद्रणम्` from
                // inside `अष्टक` itself must NOT match: the intrinsic is a
                // contract about a cross-module call, and matching a bare name
                // would silently capture any same-named routine the module grew
                // later. `m` is `None` for an unqualified call, so the pattern
                // below simply never sees one.
                // `V-005` — THE FLOAT BUILT-INS, by exact qualified name, before
                // `resolve_call`: `ashtaka.t1` declares none of them, so there is
                // no body to fall through to. `ir.t1` lowers the same calls to
                // the D-extension instructions.
                if let Some((b, _)) = float_builtin(m.as_deref(), name) {
                    return eval_float_builtin(b, name, &vals);
                }
                // `W-381` stage 3 — THE CHECKED INTEGER BUILT-INS, the same way.
                if let Some(b) = checked_builtin(m.as_deref(), name) {
                    return eval_checked_builtin(b, name, &vals);
                }
                // `V-008` part 2 — THE VECTOR BUILT-INS, by exact qualified
                // name, for the float built-ins' reason. Natively the call is
                // one IR node the emitter expands into a strip-mined loop.
                if *vector && let Some(b) = vector_builtin(m.as_deref(), name) {
                    return self.eval_vector_builtin(b, name, &vals);
                }
                if m.as_deref() == Some("अष्टक") && name == "मुद्रणम्" {
                    if let Some(Value::Int(v)) = vals.first() {
                        self.sink.push((*v & 0xff) as u8);
                    }
                    return Ok(Value::Int(0));
                }
                // `W-350`'s DEVICE REGISTER WINDOW — AND THIS SIDE REFUSES
                // RATHER THAN ANSWERING, which is the opposite of the three
                // intrinsics above and is deliberate.
                //
                // `मुद्रणम्` can be interpreted because the interpreter can
                // collect an octet, and the file window because the host can
                // read a file. A DEVICE REGISTER HAS NO ANSWER HERE: there is no
                // device model behind `0x1000_1000`, so any value this returned
                // would be invented. Returning `Int(0)` would be the worst of
                // the options — it is a plausible register reading, so a driver
                // would take it for data and the predict would agree with a
                // native run that read something else entirely. That is a
                // both-engine green over an absence.
                //
                // SO IT RAISES, BY NAME. A source that touches a device cannot
                // earn a predict, and it should not: it is verified natively and
                // under QEMU against C-009's screendump. `W-340`'s rule is the
                // one that applies — a predict whose refusal is invisible is
                // worse than no predict — so the refusal says which intrinsic,
                // and why, in the one place a reader will be looking.
                // `W-377`'s CARVE-OUT, AND ONLY IT: with a socket log set, the four SOCK
                // addresses are answered from the log's model (`socket_call`); every other
                // address, and every call with no socket log, keeps the refusal below.
                if m.as_deref() == Some("अष्टक")
                    && (name == "उपकरणचतुरष्टकाहारः" || name == "उपकरणचतुरष्टकनिधानम्")
                    && let Some(answer) = self.socket_call(name == "उपकरणचतुरष्टकनिधानम्", &vals)
                {
                    return answer;
                }
                if m.as_deref() == Some("अष्टक")
                    && (name == "उपकरणचतुरष्टकाहारः" || name == "उपकरणचतुरष्टकनिधानम्")
                {
                    return Err(RunError::new(format!(
                        "`अष्टकॱ{name}` reaches a DEVICE REGISTER, and this interpreter has no \
                         device behind it — so there is no value to predict. Natively `ir.t1` \
                         replaces the call with a four-octet load or store at the address; here \
                         there is nothing to read and inventing a zero would be a plausible \
                         register reading a driver would mistake for data. Verify a source that \
                         touches a device NATIVELY and under QEMU, not by predict (`W-350`)."
                    )));
                }
                let target = self.resolve_call(m.as_deref(), name, module)?;
                self.invoke(&target, vals)
            }
        }
    }

    fn resolve_call(
        &self,
        module: Option<&str>,
        name: &str,
        from: usize,
    ) -> Result<Rc<Routine>, RunError> {
        if let Some(m) = module {
            let i = self
                .by_module
                .get(m)
                .ok_or_else(|| RunError::new(format!("no module named `{m}` is loaded")))?;
            return self.modules[*i]
                .routines
                .iter()
                .find(|r| r.name == name)
                .cloned()
                .ok_or_else(|| RunError::new(format!("`{m}` has no routine `{name}`")));
        }
        if let Some(r) = self.modules[from].routines.iter().find(|r| r.name == name) {
            return Ok(Rc::clone(r));
        }
        self.routines()
            .find(|r| r.name == name)
            .cloned()
            .ok_or_else(|| RunError::new(format!("no routine named `{name}` is loaded")))
    }

    /// `W-359` (coordinator's final ruling, 2026-10-04): the parameter a routine
    /// GROWS AND RETURNS. A routine is grow-and-return for run parameter `P` when
    ///   ॱ `P` is declared `अङ्कः अन्तः T`,
    ///   ॱ the body's LAST top-level statement is a `प्रत्यागमनम्`, so no path
    ///     falls off the end, and
    ///   ॱ EVERY `प्रत्यागमनम्` in the body, at any depth, returns `P`: the bare
    ///     name, or a TAIL CALL of an unqualified routine of the SAME module
    ///     that is itself grow-and-return, with the bare name `P` in its grown
    ///     position — followed at most [`GROWTH_DEPTH`] calls deep.
    ///
    /// `शून्यम्` IS NOT `P`. It was admitted until the final ruling, and it did not
    /// have to precede any store: a routine could grow `P` and answer nil, and a
    /// caller assigning that back would lose the run on BOTH engines (a loss,
    /// not a divergence). The `सम्भाव्य` writers that needed it now report
    /// failure by `संयोजनॱनिधानविफलम्`. The `.t1` twin is
    /// `घोषणासञ्चयॱवर्धकप्राचलः`, with the same bound and the same locality.
    fn grown_and_returned(&self, module: usize, r: &Routine, depth: usize) -> Option<String> {
        let body = r.body.as_ref().ok()?;
        if !matches!(body.last(), Some(Stmt::Return(_))) {
            return None;
        }
        if !r.params.iter().any(|(_, t)| matches!(t, Ty::Slice(_))) {
            return None;
        }
        let mut found = Vec::new();
        returns_of(body, &mut found);
        let mut named: Option<String> = None;
        for e in found {
            let cand = match e {
                Some(Expr::Var(n)) => n.clone(),
                Some(Expr::Call {
                    module: None,
                    name,
                    args,
                    ..
                }) => {
                    // THE `.t1` TWIN'S BOUNDARY, EXACTLY: `प्रत्यागतवर्धनपाठः` refuses
                    // to follow a tail call from a routine at depth ≥ the bound
                    // (root ०), so a callee is classified down to depth ८ — eight
                    // tail calls, nine routines. This read `depth + 1 >= ..`
                    // until the delta review, cutting the chain one level early.
                    if depth >= GROWTH_DEPTH {
                        return None;
                    }
                    let callee = self.modules[module]
                        .routines
                        .iter()
                        .find(|c| &c.name == name)?;
                    let g = self.grown_and_returned(module, callee, depth + 1)?;
                    let at = callee.params.iter().position(|(p, _)| *p == g)?;
                    match args.get(at) {
                        Some(Expr::Var(n)) => n.clone(),
                        _ => return None,
                    }
                }
                _ => return None,
            };
            match &named {
                None => named = Some(cand),
                Some(m) if *m == cand => {}
                Some(_) => return None,
            }
        }
        let n = named?;
        r.params
            .iter()
            .find(|(p, t)| *p == n && matches!(t, Ty::Slice(_)))
            .map(|(p, _)| p.clone())
    }

    /// Fill every routine's `grows` and `guarded` — once, after every module
    /// is loaded and before any routine is shared.
    fn classify_growth(&mut self) {
        let mut found: Vec<Vec<Option<String>>> = Vec::new();
        for (m, module) in self.modules.iter().enumerate() {
            found.push(
                module
                    .routines
                    .iter()
                    .map(|r| self.grown_and_returned(m, r, 0))
                    .collect(),
            );
        }
        for (module, grows) in self.modules.iter_mut().zip(found) {
            for (r, g) in module.routines.iter_mut().zip(grows) {
                let r = Rc::get_mut(r).expect("no routine is shared before classification");
                r.guarded = r
                    .params
                    .iter()
                    .filter(|(p, t)| matches!(t, Ty::Slice(_)) && g.as_deref() != Some(p))
                    .map(|(p, _)| p.clone())
                    .collect();
                r.grows = g;
            }
        }
    }

    /// `W-359` rules (A) and (C), AT LOAD, and the fail-closed unresolved callee.
    ///
    /// A grow-and-return routine hands the grown run back as its result —
    /// natively growth cuts a NEW block whose base reaches the caller only
    /// through the return. So the result is either ASSIGNED BACK to the very
    /// name passed for the grown parameter (`x भवति f … x …`, x a bare name),
    /// or TAIL-RETURNED from a routine that itself grows and returns that name.
    /// Anything else — dropped, bound to another name or by `चरः`, used inside
    /// a larger expression — refuses the load, by name. So does rule (C): an
    /// indexed store into, or a growth of, a LOCAL ALIAS of a run parameter (a
    /// `चरः` local bound to the parameter or to an alias by a bare name, closed
    /// transitively); aliasing alone is allowed. A callee no loaded module
    /// declares cannot be classified, so the CALLING routine is made to refuse
    /// when called (fail closed) — except the intrinsics this interpreter
    /// answers itself. The `.t1` twin is `अर्थॱवर्धनबन्धपरीक्षा`.
    fn refuse_unbound_growth(&mut self) -> Result<(), RunError> {
        let mut unresolved: Vec<(usize, usize, String)> = Vec::new();
        for (m, module) in self.modules.iter().enumerate() {
            for (ri, r) in module.routines.iter().enumerate() {
                let Ok(body) = &r.body else { continue };
                let mut check = BindCheck {
                    it: self,
                    module: m,
                    routine: r,
                    aliases: aliases_of(r, body),
                    unresolved: None,
                };
                check.stmts(body)?;
                if let Some(name) = check.unresolved {
                    unresolved.push((m, ri, name));
                }
            }
        }
        for (m, ri, name) in unresolved {
            let r = Rc::get_mut(&mut self.modules[m].routines[ri])
                .expect("no routine is shared before classification");
            r.body = Err(format!(
                "`{}\u{971}{}` calls `{name}`, which no loaded module declares, so how its \
                 result is used cannot be checked (W-359)",
                r.module, r.name
            ));
        }
        Ok(())
    }

    /// `W-359` rule (2): is `name`, written as the base of an indexed store, a
    /// GUARDED parameter of the routine executing — a run parameter of a
    /// routine that does not grow-and-return it, and not shadowed by a local?
    /// The parameters are the frame at the bottom of the routine's scope.
    fn guarded_parameter(&self, name: &str, scope: &[Frame]) -> Option<Rc<Routine>> {
        let r = self.active.last()?;
        if !r.guarded.iter().any(|p| p == name) {
            return None;
        }
        let innermost = scope
            .iter()
            .rposition(|frame| frame.iter().any(|(n, _)| n == name))?;
        (innermost == 0).then(|| Rc::clone(r))
    }
}

/// `W-359` rule (C): the LOCAL ALIASES of a routine's run parameters — `चरः`
/// locals bound, anywhere in the body, to a parameter or an alias by a bare
/// name (`चरः y … भवति p`, or `y भवति p` for a `चरः` local y), closed.
fn aliases_of(r: &Routine, body: &[Stmt]) -> Vec<String> {
    fn lets<'a>(stmts: &'a [Stmt], out: &mut Vec<&'a str>) {
        for s in stmts {
            match s {
                Stmt::Let { name, .. } => out.push(name),
                Stmt::If { then, els, .. } => {
                    lets(then, out);
                    lets(els, out);
                }
                Stmt::While { body, .. } => lets(body, out),
                _ => {}
            }
        }
    }
    fn bindings<'a>(stmts: &'a [Stmt], locals: &[&str], out: &mut Vec<(&'a str, &'a str)>) {
        for s in stmts {
            match s {
                Stmt::Let {
                    name,
                    init: Expr::Var(z),
                    ..
                } => out.push((name, z)),
                Stmt::Assign {
                    target: Expr::Var(y),
                    value: Expr::Var(z),
                } if locals.contains(&y.as_str()) => out.push((y, z)),
                Stmt::If { then, els, .. } => {
                    bindings(then, locals, out);
                    bindings(els, locals, out);
                }
                Stmt::While { body, .. } => bindings(body, locals, out),
                _ => {}
            }
        }
    }
    let mut locals = Vec::new();
    lets(body, &mut locals);
    let mut pairs = Vec::new();
    bindings(body, &locals, &mut pairs);
    let params: Vec<&str> = r
        .params
        .iter()
        .filter(|(_, t)| matches!(t, Ty::Slice(_)))
        .map(|(p, _)| p.as_str())
        .collect();
    let mut aliases: Vec<String> = Vec::new();
    loop {
        let before = aliases.len();
        for (y, z) in &pairs {
            if (params.contains(z) || aliases.iter().any(|a| a == z))
                && !aliases.iter().any(|a| a == y)
            {
                aliases.push((*y).to_string());
            }
        }
        if aliases.len() == before {
            return aliases;
        }
    }
}

/// A grow-and-return call: its callee, and its grown argument if a bare name.
type GrownCall = (Rc<Routine>, Option<String>);

/// One routine's walk for [`Interpreter::refuse_unbound_growth`].
struct BindCheck<'a> {
    it: &'a Interpreter,
    module: usize,
    routine: &'a Routine,
    aliases: Vec<String>,
    unresolved: Option<String>,
}

impl BindCheck<'_> {
    fn here(&self) -> String {
        format!("{}\u{971}{}", self.routine.module, self.routine.name)
    }

    /// The callee of a call, if it resolves; an unresolvable non-intrinsic is
    /// remembered (fail closed) and answers `None`.
    fn callee(&mut self, module: Option<&str>, name: &str) -> Option<Rc<Routine>> {
        match self.it.resolve_call(module, name, self.module) {
            Ok(c) => Some(c),
            Err(_) => {
                let intrinsic = (module == Some("अष्टक")
                    && matches!(name, "मुद्रणम्" | "उपकरणचतुरष्टकाहारः" | "उपकरणचतुरष्टकनिधानम्"))
                    || float_builtin(module, name).is_some()
                    || checked_builtin(module, name).is_some()
                    || wait_builtin(module, name);
                if !intrinsic && self.unresolved.is_none() {
                    self.unresolved = Some(match module {
                        Some(m) => format!("{m}\u{971}{name}"),
                        None => name.to_string(),
                    });
                }
                None
            }
        }
    }

    fn misuse(&self, callee: &Routine, p: &str) -> RunError {
        RunError::new(format!(
            "`{}` uses the result of `{}\u{971}{}`, which grows its parameter `{p}`, other than \
             by assigning it back to the name passed for `{p}` or tail-returning it (W-359)",
            self.here(),
            callee.module,
            callee.name
        ))
    }

    fn alias(&self, y: &str) -> RunError {
        RunError::new(format!(
            "`{}` stores into or grows `{y}`, a local alias of a run parameter (W-359)",
            self.here()
        ))
    }

    /// A call whose result is used in an approved form: the grown argument,
    /// if it is a bare name. The arguments are walked either way.
    fn grown_arg(&mut self, e: &Expr) -> Result<Option<GrownCall>, RunError> {
        let Expr::Call {
            module,
            name,
            args,
            vector,
            wait,
        } = e
        else {
            return Ok(None);
        };
        // `V-008` part 2: the vector built-in calls no routine; nor does the
        // `W-373` wait.
        if *vector || *wait {
            self.exprs(args)?;
            return Ok(None);
        }
        let Some(callee) = self.callee(module.as_deref(), name) else {
            self.exprs(args)?;
            return Ok(None);
        };
        let Some(g) = callee.grows.clone() else {
            return Ok(None);
        };
        self.exprs(args)?;
        let at = callee.params.iter().position(|(p, _)| *p == g);
        let arg = match at.and_then(|i| args.get(i)) {
            Some(Expr::Var(n)) => Some(n.clone()),
            _ => None,
        };
        Ok(Some((callee, arg)))
    }

    fn exprs(&mut self, es: &[Expr]) -> Result<(), RunError> {
        for e in es {
            self.expr(e)?;
        }
        Ok(())
    }

    /// Any grow-and-return call reached here is a misuse.
    fn expr(&mut self, e: &Expr) -> Result<(), RunError> {
        match e {
            Expr::Call {
                module,
                name,
                args,
                vector,
                wait,
            } => {
                if !*vector
                    && !*wait
                    && let Some(callee) = self.callee(module.as_deref(), name)
                    && let Some(p) = &callee.grows
                {
                    return Err(self.misuse(&callee, p));
                }
                self.exprs(args)
            }
            Expr::Bin(_, l, r) | Expr::Index(l, r) => {
                self.expr(l)?;
                self.expr(r)
            }
            Expr::Slice(a, b, c) => {
                self.expr(a)?;
                self.expr(b)?;
                self.expr(c)
            }
            Expr::Field(b, _) => self.expr(b),
            Expr::Num(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Nil | Expr::Var(_) => Ok(()),
        }
    }

    fn stmts(&mut self, stmts: &[Stmt]) -> Result<(), RunError> {
        for s in stmts {
            match s {
                Stmt::Let { init, .. } => self.expr(init)?,
                Stmt::Assign { target, value } => {
                    if let Some((callee, arg)) = self.grown_arg(value)? {
                        let p = callee.grows.clone().unwrap_or_default();
                        match (target, arg) {
                            (Expr::Var(x), Some(a)) if *x == a => {
                                if self.aliases.iter().any(|y| y == x) {
                                    return Err(self.alias(x));
                                }
                            }
                            _ => return Err(self.misuse(&callee, &p)),
                        }
                    } else {
                        self.expr(value)?;
                    }
                    if let Expr::Index(base, idx) = target {
                        if let Expr::Var(y) = &**base
                            && self.aliases.iter().any(|a| a == y)
                        {
                            return Err(self.alias(y));
                        }
                        self.expr(base)?;
                        self.expr(idx)?;
                    } else {
                        self.expr(target)?;
                    }
                }
                Stmt::If { cond, then, els } => {
                    self.expr(cond)?;
                    self.stmts(then)?;
                    self.stmts(els)?;
                }
                Stmt::While { cond, body } => {
                    self.expr(cond)?;
                    self.stmts(body)?;
                }
                Stmt::Return(Some(e)) => {
                    if let Some((callee, arg)) = self.grown_arg(e)? {
                        let own = self.routine.grows.as_deref();
                        if arg.is_none() || arg.as_deref() != own {
                            return Err(RunError::new(format!(
                                "`{}` tail-returns `{}\u{971}{}`, which grows its parameter \
                                 `{}`, but does not itself grow and return the name it passes \
                                 (W-359)",
                                self.here(),
                                callee.module,
                                callee.name,
                                callee.grows.as_deref().unwrap_or_default()
                            )));
                        }
                    } else {
                        self.expr(e)?;
                    }
                }
                Stmt::Return(None) => {}
                Stmt::Eval(e) => self.expr(e)?,
            }
        }
        Ok(())
    }
}

/// Every call `(module, name)` and every declared local type in a body, in
/// written order — the walk [`Interpreter::load_order_sensitive_sites`] reads.
fn walk_stmts<'a>(
    stmts: &'a [Stmt],
    calls: &mut Vec<(Option<&'a str>, &'a str)>,
    types: &mut Vec<&'a Ty>,
) {
    for s in stmts {
        match s {
            Stmt::Let { ty, init, .. } => {
                types.push(ty);
                walk_expr(init, calls);
            }
            Stmt::Assign { target, value } => {
                walk_expr(target, calls);
                walk_expr(value, calls);
            }
            Stmt::If { cond, then, els } => {
                walk_expr(cond, calls);
                walk_stmts(then, calls, types);
                walk_stmts(els, calls, types);
            }
            Stmt::While { cond, body } => {
                walk_expr(cond, calls);
                walk_stmts(body, calls, types);
            }
            Stmt::Return(Some(e)) | Stmt::Eval(e) => walk_expr(e, calls),
            Stmt::Return(None) => {}
        }
    }
}

fn walk_expr<'a>(e: &'a Expr, calls: &mut Vec<(Option<&'a str>, &'a str)>) {
    match e {
        Expr::Call {
            module, name, args, ..
        } => {
            calls.push((module.as_deref(), name));
            for a in args {
                walk_expr(a, calls);
            }
        }
        Expr::Bin(_, l, r) | Expr::Index(l, r) => {
            walk_expr(l, calls);
            walk_expr(r, calls);
        }
        Expr::Slice(a, b, c) => {
            walk_expr(a, calls);
            walk_expr(b, calls);
            walk_expr(c, calls);
        }
        Expr::Field(inner, _) => walk_expr(inner, calls),
        Expr::Num(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Nil | Expr::Var(_) => {}
    }
}

/// Why a global's initialiser is not a value the loader can settle at load —
/// `None` when it is one: a literal, or literals combined, indexed or sliced.
/// A call and a name are the two things whose value depends on what else has
/// loaded, so each is named (`initial_value`).
///
/// `यतिः अङ्कः ० अन्तः` — `utsarjana.t1`'s line-feed octet, ADR-0018's word
/// indexed — is a literal indexed and passes; `यतिः` is a one-character
/// `Kind::Str`, not a name.
fn not_a_load_time_value(e: &Expr) -> Option<String> {
    match e {
        Expr::Call { module, name, .. } => Some(match module {
            Some(m) => format!("calls `{m}\u{971}{name}`"),
            None => format!("calls `{name}`"),
        }),
        Expr::Var(n) => Some(format!("reads `{n}`")),
        Expr::Bin(_, l, r) | Expr::Index(l, r) => {
            not_a_load_time_value(l).or_else(|| not_a_load_time_value(r))
        }
        Expr::Slice(a, b, c) => not_a_load_time_value(a)
            .or_else(|| not_a_load_time_value(b))
            .or_else(|| not_a_load_time_value(c)),
        Expr::Field(inner, _) => not_a_load_time_value(inner),
        Expr::Num(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Nil => None,
    }
}

/// The names a type mentions — `अङ्कः अन्तः वस्तु` mentions `वस्तु`.
fn named_types(ty: &Ty) -> Vec<&str> {
    match ty {
        Ty::Named(n) => vec![n.as_str()],
        Ty::Slice(t) | Ty::Pointer(t) | Ty::Optional(t) | Ty::ErrorUnion(t) => named_types(t),
    }
}

// ── `V-005`: the float built-ins ─────────────────────────────────────────

/// The module a float built-in is qualified by: `अष्टकॱप्लवयोगः`, as
/// `अष्टकॱमुद्रणम्` is. The owner's shape (2026-10-03) is CALL-FORM BUILT-INS
/// with no grammar change, and the tree's call-form intrinsics are qualified
/// calls into `अष्टक` intercepted on both engines by their EXACT qualified name.
/// Unlike those, a float built-in has NO DECLARATION in `ashtaka.t1`: a missed
/// interception is then an unresolved symbol at link, not a body that answers.
pub const FLOAT_BUILTIN_MODULE: &str = "अष्टक";

/// Which float built-in a name is. `Convert` and `Move` each name BOTH
/// directions — the tree has one word for `fcvt` and one for `fmv`
/// (`spec/mnemonics-riscv64.src.tsv`), and T0 tells the directions apart by the
/// operand's register file — so the direction is the ARGUMENT's type: an
/// integer goes to a float, a float to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatBuiltin {
    /// `fadd.d`
    Add,
    /// `fsub.d`
    Sub,
    /// `fmul.d`
    Mul,
    /// `fmadd.d`: `a × b + c`, rounded once
    MulAdd,
    /// `fdiv.d`
    Div,
    /// `fsqrt.d`
    Sqrt,
    /// `feq.d`
    Eq,
    /// `flt.d`
    Lt,
    /// `fle.d`
    Le,
    /// `fcvt.d.l` from an integer, `fcvt.l.d` from a float
    Convert,
    /// `fmv.d.x` from an integer, `fmv.x.d` from a float
    Move,
}

/// Every float built-in: its name, what it is, how many arguments it takes and
/// the RV64D mnemonic(s) whose `spec/encodings-riscv64.tsv` word it is. The
/// names are THAT FILE'S third column, copied, and
/// `the_float_builtin_names_are_the_encoding_tables_words` reads the file to
/// prove it — a name typed from memory is exactly the defect that test exists
/// for.
pub const FLOAT_BUILTINS: [(&str, FloatBuiltin, usize, &str); 11] = [
    ("प्लवयोगः", FloatBuiltin::Add, 2, "fadd.d"),
    ("प्लववियोगः", FloatBuiltin::Sub, 2, "fsub.d"),
    ("प्लवगुणनम्", FloatBuiltin::Mul, 2, "fmul.d"),
    ("प्लवगुणयोगः", FloatBuiltin::MulAdd, 3, "fmadd.d"),
    ("प्लवभागः", FloatBuiltin::Div, 2, "fdiv.d"),
    ("प्लववर्गमूलम्", FloatBuiltin::Sqrt, 1, "fsqrt.d"),
    ("प्लवसमम्", FloatBuiltin::Eq, 2, "feq.d"),
    ("प्लवन्यूनम्", FloatBuiltin::Lt, 2, "flt.d"),
    ("प्लवानधिकम्", FloatBuiltin::Le, 2, "fle.d"),
    ("प्लवरूपान्तरम्", FloatBuiltin::Convert, 1, "fcvt.d.l fcvt.l.d"),
    ("प्लवसंचारः", FloatBuiltin::Move, 1, "fmv.d.x fmv.x.d"),
];

/// The built-in a `(module, name)` call names, by EXACT qualified name — never
/// by the bare word, so a routine of that name in any module stays a call.
#[must_use]
pub fn float_builtin(module: Option<&str>, name: &str) -> Option<(FloatBuiltin, usize)> {
    if module != Some(FLOAT_BUILTIN_MODULE) {
        return None;
    }
    FLOAT_BUILTINS
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|(_, b, arity, _)| (*b, *arity))
}

// ── `W-381` stage 3: the checked integer built-ins ─────────────────────────

/// `W-381` stage 3 (owner rulings 2026-10-06): the CHECKED integer operations,
/// call-form members of `अष्टक` as the float built-ins are (ADR-0043 D1: a
/// call-form member, never a keyword). Plain `योगः`/`वियोगः`/`गुणनम्`/`वामसृ`
/// WRAP (ruling P1); these refuse instead. The names are the owner's, copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedBuiltin {
    /// `a + b`, refused on SIGNED 64-bit overflow.
    Add,
    /// `a − b`, refused on SIGNED 64-bit overflow.
    Sub,
    /// `a × b`, refused on SIGNED 64-bit overflow.
    Mul,
    /// `a << n`, refused ONLY when `n` is 64 or more (ruling (a)): bits shifted
    /// out are not an overflow here.
    Shl,
}

/// The four checked built-ins: the member name after `अष्टकॱ`, and which one.
/// Each takes two arguments.
pub const CHECKED_BUILTINS: [(&str, CheckedBuiltin); 4] = [
    ("सुरक्षितयोगः", CheckedBuiltin::Add),
    ("सुरक्षितवियोगः", CheckedBuiltin::Sub),
    ("सुरक्षितगुणनम्", CheckedBuiltin::Mul),
    ("सुरक्षितवामसरणम्", CheckedBuiltin::Shl),
];

/// `ir.t1`'s `अतिप्रवाहनिषेधः`: the refusal a checked built-in raises, natively
/// written to the finisher in FAIL form (`0x035c_3333`), so `yantra` halts
/// `Finisher { status: Some(0x35c) }` and QEMU exits `0x5c`.
pub const OVERFLOW_REFUSAL: u64 = 0x35c;
/// Its name (owner ruling, 2026-10-06).
pub const OVERFLOW_REFUSAL_NAME: &str = "अतिप्रवाहनिषेधः";

/// The refusal of a division or remainder BY ZERO (owner ruling (b),
/// 2026-10-06: refuse on both engines, never the RV64 default). Natively a zero
/// test before every `div`/`rem` whose divisor is not a non-zero numeral.
pub const DIVISION_BY_ZERO_REFUSAL: u64 = 0x35e;
/// Its name, APPROVED by the owner 2026-10-06 (the lane's proposal): this
/// constant and `ir.t1`'s one global are the only places it is spelled.
pub const DIVISION_BY_ZERO_REFUSAL_NAME: &str = "शून्यविभाजननिषेधः";

/// The checked built-in a `(module, name)` call names, by EXACT qualified name.
#[must_use]
pub fn checked_builtin(module: Option<&str>, name: &str) -> Option<CheckedBuiltin> {
    if module != Some(FLOAT_BUILTIN_MODULE) {
        return None;
    }
    CHECKED_BUILTINS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
}

fn eval_checked_builtin(b: CheckedBuiltin, name: &str, vals: &[Value]) -> Result<Value, RunError> {
    let int = |i: usize| -> Result<i128, RunError> {
        match vals.get(i) {
            Some(Value::Float(_)) | None => Err(RunError::new(format!(
                "`{FLOAT_BUILTIN_MODULE}\u{971}{name}` takes two integers, and argument {} is not one",
                i + 1
            ))),
            Some(v) => v.int(),
        }
    };
    let (x, y) = (int(0)?, int(1)?);
    let refused = || {
        RunError::new(format!(
            "`{FLOAT_BUILTIN_MODULE}\u{971}{name}` over {} and {} does not fit 64 signed bits: \
             refused as `{OVERFLOW_REFUSAL_NAME}` (W-381, finisher word {OVERFLOW_REFUSAL:#x})",
            signed(x),
            signed(y)
        ))
    };
    let (a, c) = (signed(x), signed(y));
    let r = match b {
        CheckedBuiltin::Add => a.checked_add(c).ok_or_else(refused)?,
        CheckedBuiltin::Sub => a.checked_sub(c).ok_or_else(refused)?,
        CheckedBuiltin::Mul => a.checked_mul(c).ok_or_else(refused)?,
        CheckedBuiltin::Shl => {
            if word(y) >= 64 {
                return Err(RunError::new(format!(
                    "`{FLOAT_BUILTIN_MODULE}\u{971}{name}` by {}, which is 64 or more: refused as \
                     `{OVERFLOW_REFUSAL_NAME}` (W-381, finisher word {OVERFLOW_REFUSAL:#x})",
                    word(y)
                )));
            }
            #[allow(clippy::cast_possible_truncation)]
            let n = word(y) as u32;
            return Ok(Value::Int(wrap(x << n)));
        }
    };
    Ok(Value::Int(i128::from(r)))
}

// ── `V-008` part 2: the elementwise vector built-ins ─────────────────────

/// The qualifier of a vector built-in: `व्यूहॱप्लवयोगः` adds two `प६४` runs
/// elementwise into a third (ADR-0043 R3: `व्यूहॱ` + V-005's float names, so
/// no operation word is coined). NOT a module and declared by none: like the
/// float built-ins, a missed interception is an unresolved call, never a body.
pub const VECTOR_BUILTIN_MODULE: &str = "व्यूह";

/// `ir.t1`'s `व्यूहदैर्घ्यनिषेधः`: the refusal CODE the native expansion
/// writes when the three runs' lengths differ (`V-008` part 2, owner ruling
/// Q2: refuse, never truncate). The next value free after `W-359`'s `0x359`.
/// Since `W-381` the finisher word is its FAIL form, `riscv64::fail_word`
/// (`0x035a_3333`), so `yantra` halts `Finisher { status: Some(0x35a) }` and
/// QEMU stops too. The interpreter refuses at the same call with this code in
/// its cause (the text, which says "finisher word", is unchanged).
pub const VECTOR_LENGTH_REFUSAL: u64 = 0x35a;

/// The vector built-in a `(module, name)` call names, by EXACT qualified name:
/// the four elementwise operations yantra executes as `.vv` — add, subtract,
/// multiply, divide — each taking three runs (`फल`, `क`, `ख`). Their member
/// names are [`FLOAT_BUILTINS`]' own, so the encoding-table proof of that table
/// covers these too.
#[must_use]
pub fn vector_builtin(module: Option<&str>, name: &str) -> Option<FloatBuiltin> {
    if module != Some(VECTOR_BUILTIN_MODULE) {
        return None;
    }
    FLOAT_BUILTINS
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|(_, b, ..)| *b)
        .filter(|b| {
            matches!(
                b,
                FloatBuiltin::Add | FloatBuiltin::Sub | FloatBuiltin::Mul | FloatBuiltin::Div
            )
        })
}

/// How many arguments a vector built-in takes: the result run and the two
/// operand runs.
pub const VECTOR_BUILTIN_ARITY: usize = 3;

/// The cause a length mismatch raises, the same words on every call.
fn vector_length_error(qualified: &str, lens: [usize; 3]) -> RunError {
    RunError::new(format!(
        "`{qualified}` over runs of unequal length — the result {}, the operands {} and {}: \
         refused, never truncated (V-008, finisher word {VECTOR_LENGTH_REFUSAL:#x})",
        lens[0], lens[1], lens[2]
    ))
}

// ── `V-009` part (ii): the matrix built-ins ──────────────────────────────

/// The matrix product's member, `व्यूहॱआव्यूहगुणनम्` (ADR-0043 R1/R3, copied
/// from the owner's ruling by script). Also the name of the module's product
/// kernel routine (`chain.rs`'s `synthesised_routine_name`).
pub const MATRIX_PRODUCT_MEMBER: &str = "आव्यूहगुणनम्";
/// The transpose's member, `व्यूहॱव्युत्क्रमः`; also its kernel routine's name.
pub const MATRIX_TRANSPOSE_MEMBER: &str = "व्युत्क्रमः";
/// The tensor member, `व्यूहॱसमासः`: the rank-3 batched product (owner
/// ruling 2026-10-06, the design's Q4 (a)), the product kernel with a batch.
pub const TENSOR_MEMBER: &str = "समासः";
/// The row-major layout constant, `व्यूहॱपङ्क्तिप्रधानम्` (the owner's names table).
pub const ROW_MAJOR: &str = "पङ्क्तिप्रधानम्";
/// The column-major layout constant, `व्यूहॱस्तम्भप्रधानम्`.
pub const COLUMN_MAJOR: &str = "स्तम्भप्रधानम्";
/// The transposed layout constant, `व्यूहॱपरिवर्तितम्` (owner ruling
/// 2026-10-06: "the Transposed layout (Aᵀ, swapping row and column stride
/// semantics)").
pub const TRANSPOSED: &str = "परिवर्तितम्";

/// `ir.t1`'s `अध्यासप्रतिषेधः`: the refusal CODE of a matrix built-in whose result
/// run is one of its operand runs (owner ruling 2026-10-06, the design's D6).
/// The next value free after `0x35a`; FAIL form, as every refusal (`W-381`).
pub const REFUSAL_ADHYASA: u64 = 0x35b;

/// `ir.t1`'s `सीमातीतलेखननिषेधः`: the refusal CODE of an out-of-bounds indexed
/// STORE — natively a negative index (`W-381` stage 4, owner ruling
/// 2026-10-06: "Assign a dedicated refusal. Split from 0x355"). Reads stay
/// `0x355`. FAIL form, as every refusal; the interpreter names the code in its
/// cause, which `agreement.rs`'s `NATIVE_REFUSALS` keys on.
pub const OUT_OF_BOUNDS_WRITE_REFUSAL: u64 = 0x35d;

/// `ir.t1`'s `खण्डप्रविष्टिसीमा`: the largest run, in ENTRIES, for every width
/// (`W-381` stage 4, the review's finding 1). A store at an index at or past it
/// refuses with [`OUT_OF_BOUNDS_WRITE_REFUSAL`] on both engines; below it the
/// native need, (index + 1) × width, is at most 2^28 octets and cannot wrap.
pub const MAX_RUN_ENTRIES: u64 = 1 << 25;

/// The three matrix built-ins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixOp {
    /// `व्यूहॱआव्यूहगुणनम् ( फल , क , ख , M , K , N , L )`.
    Product,
    /// `व्यूहॱव्युत्क्रमः ( फल , क , M , N , L )`.
    Transpose,
    /// `व्यूहॱसमासः ( फल , क , ख , B , M , K , N , L )`.
    Tensor,
}

impl MatrixOp {
    /// How many arguments the call takes, the layout constant included.
    #[must_use]
    pub fn arity(self) -> usize {
        match self {
            Self::Product => 7,
            Self::Transpose => 5,
            Self::Tensor => 8,
        }
    }

    /// The member name.
    #[must_use]
    pub fn member(self) -> &'static str {
        match self {
            Self::Product => MATRIX_PRODUCT_MEMBER,
            Self::Transpose => MATRIX_TRANSPOSE_MEMBER,
            Self::Tensor => TENSOR_MEMBER,
        }
    }
}

/// The matrix built-in a `(module, name)` call names, by EXACT qualified name
/// behind the vector qualifier.
#[must_use]
pub fn matrix_builtin(module: Option<&str>, name: &str) -> Option<MatrixOp> {
    if module != Some(VECTOR_BUILTIN_MODULE) {
        return None;
    }
    [MatrixOp::Product, MatrixOp::Transpose, MatrixOp::Tensor]
        .into_iter()
        .find(|op| op.member() == name)
}

/// The members that also NAME a module's kernel routines: the product's and
/// the transpose's. `ir.t1` synthesises BOTH routines in a module that makes
/// ANY matrix call, and both emitters label them `<module><member>` — so a user
/// routine of either name in that module is a label collision natively. The
/// tensor member names no routine: the tensor is the product kernel with a
/// batch.
pub const MATRIX_KERNEL_MEMBERS: [&str; 2] = [MATRIX_PRODUCT_MEMBER, MATRIX_TRANSPOSE_MEMBER];

/// The refusal of a routine named `member` (one of [`MATRIX_KERNEL_MEMBERS`])
/// in `module`, a module that makes a matrix call. ONE text for its three
/// sayers: the interpreter's load ([`Interpreter::load`]), `riscv64.rs`'s
/// label check, and `chain.rs`'s report of the `.t1` emitter's
/// `LabelCollision`.
#[must_use]
pub fn kernel_name_refusal(module: &str, member: &str) -> String {
    format!(
        "`{module}\u{971}{member}`: {}; this module makes a `{VECTOR_BUILTIN_MODULE}` matrix \
         call, so its matrix kernel routine is named `{member}` (V-009 (ii))",
        kernel_name_reserved(member)
    )
}

/// The reserved sentence alone, WITHOUT the claim that a module makes a matrix
/// call: what a sayer that cannot see the kernel may say (`chain.rs`'s report
/// of `samyojana.t1`'s duplicate symbol, under a condition).
#[must_use]
pub fn kernel_name_reserved(member: &str) -> String {
    format!("`{member}` is reserved for the matrix built-in in a module that uses it")
}

/// The kernel member a symbol `<module><member>` ends with, and the module.
#[must_use]
pub fn kernel_member_of(symbol: &str) -> Option<(&str, &'static str)> {
    MATRIX_KERNEL_MEMBERS
        .iter()
        .find_map(|member| Some((symbol.strip_suffix(member)?, *member)))
}

/// `V-009` (ii): the reserved wording for a symbol two definitions export, when
/// its name is `<module><member>` for a kernel member and `kernel(module)`
/// says that module's matrix kernel is one of the definitions — the Rust
/// linker's duplicate-symbol refusal (`samyojana.rs`), which sees the objects.
/// A kernel-named global beside a matrix call, or a
/// kernel-named routine in a second source of a module whose first source
/// makes the call, collides there rather than in the emitter's label check.
#[must_use]
pub fn kernel_name_duplicate(symbol: &str, kernel: impl Fn(&str) -> bool) -> Option<String> {
    let (module, member) = kernel_member_of(symbol)?;
    kernel(module).then(|| kernel_name_refusal(module, member))
}

/// THE NATIVE RULE, MIRRORED EXACTLY (`W-381`): refused iff the module
/// declares a routine or a global named by a kernel member AND makes a matrix call — which
/// is exactly when `ir.t1` synthesises the kernel routine that collides. A
/// matrix call is one the parser marked as a vector built-in whose
/// `(module, name)` is a matrix member: the call `eval` hands to
/// [`Interpreter::eval_matrix_builtin`].
fn refuse_a_kernel_name(module: &Module) -> Result<(), RunError> {
    fn in_stmts(stmts: &[Stmt]) -> bool {
        stmts.iter().any(|s| match s {
            Stmt::Let { init, .. } => in_expr(init),
            Stmt::Assign { target, value } => in_expr(target) || in_expr(value),
            Stmt::If { cond, then, els } => in_expr(cond) || in_stmts(then) || in_stmts(els),
            Stmt::While { cond, body } => in_expr(cond) || in_stmts(body),
            Stmt::Return(Some(e)) | Stmt::Eval(e) => in_expr(e),
            Stmt::Return(None) => false,
        })
    }
    fn in_expr(e: &Expr) -> bool {
        match e {
            Expr::Call {
                module,
                name,
                args,
                vector,
                ..
            } => {
                (*vector && matrix_builtin(module.as_deref(), name).is_some())
                    || args.iter().any(in_expr)
            }
            Expr::Bin(_, l, r) | Expr::Index(l, r) => in_expr(l) || in_expr(r),
            Expr::Slice(a, b, c) => in_expr(a) || in_expr(b) || in_expr(c),
            Expr::Field(inner, _) => in_expr(inner),
            Expr::Num(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Nil | Expr::Var(_) => false,
        }
    }
    // EVERY MODULE-LEVEL NAME THAT BECOMES A LABEL `<module><name>`: a routine
    // (refused natively by the emitter's label check) and a global, public or
    // private (refused natively at link, as a symbol two definitions export).
    let Some(named) = module
        .routines
        .iter()
        .map(|r| r.name.as_str())
        .chain(module.globals.iter().map(|(n, _, _)| n.as_str()))
        .find(|n| MATRIX_KERNEL_MEMBERS.contains(n))
    else {
        return Ok(());
    };
    let calls = module
        .routines
        .iter()
        .any(|r| r.body.as_ref().is_ok_and(|b| in_stmts(b)));
    if calls {
        return Err(RunError::new(kernel_name_refusal(&module.name, named)));
    }
    Ok(())
}

/// The three layouts a matrix call names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// `व्यूहॱपङ्क्तिप्रधानम्`.
    RowMajor,
    /// `व्यूहॱस्तम्भप्रधानम्`: every operand column-major (`ir.t1` permutes the call).
    ColumnMajor,
    /// `व्यूहॱपरिवर्तितम्` (owner ruling 2026-10-06): a product's FIRST operand
    /// read transposed — A stored K×M, its two strides swapped — so C = Aᵀ·B;
    /// B and C row-major.
    Transposed,
}

/// The layout `(module, name)` names, if it is a layout constant.
#[must_use]
pub fn layout_constant(module: Option<&str>, name: &str) -> Option<Layout> {
    if module != Some(VECTOR_BUILTIN_MODULE) {
        return None;
    }
    match name {
        ROW_MAJOR => Some(Layout::RowMajor),
        COLUMN_MAJOR => Some(Layout::ColumnMajor),
        TRANSPOSED => Some(Layout::Transposed),
        _ => None,
    }
}

// ── `W-373`: the wait intrinsic ──────────────────────────────────────────

/// The qualifier of the WAIT intrinsic (`W-373`, ADR-0040 Option C R4): the
/// call is `अष्टकॱ` + [`WAIT_BUILTIN_MEMBER`], as `अष्टकॱमुद्रणम्` is a call
/// into `अष्टक`. Like the float built-ins it has NO DECLARATION in
/// `ashtaka.t1`, so a missed interception is an unresolved call, never a body.
pub const WAIT_BUILTIN_MODULE: &str = "अष्टक";

/// The member: copied from the owner's ruling in BACKLOG row `W-373`, never
/// retyped. `ir.t1` matches it with the qualifier SPLIT off, so the qualified
/// token appears in no corpus code (`tests/w373_wait.rs` holds that ratchet).
pub const WAIT_BUILTIN_MEMBER: &str = "घटनाप्रतीक्षा";

/// ONE argument, reserved and ignored today: natively it is the word stored
/// at the WAIT address (`W-377` will give it a meaning). Not zero arguments,
/// because a zero-argument call reaches `ir.t1` as an Identifier and lowers to
/// the constant ० — a "wait" that never waits natively.
pub const WAIT_BUILTIN_ARITY: usize = 1;

/// The tag in front of the EVENT word, `"SASEVENT"` read as a little-endian
/// word: the interpreter's own copy of `yantra::input::EVENT_TAG` (`sadhana`
/// does not depend on `yantra`), and `tests/w373_wait.rs` fails if the two
/// ever differ.
pub const EVENT_TAG: u64 = 0x544e_4556_4553_4153;

/// Whether a `(module, name)` call is the wait, by EXACT qualified name.
#[must_use]
pub fn wait_builtin(module: Option<&str>, name: &str) -> bool {
    module == Some(WAIT_BUILTIN_MODULE) && name == WAIT_BUILTIN_MEMBER
}

/// The interpreter's half of an event-log replay (`W-373`): the log, the
/// global the program declared right after its `SASEVENT` tag, and how many
/// records the waits have taken.
struct EventReplay {
    log: Vec<u64>,
    slot: String,
    kind: EventSlot,
    delivered: usize,
}

/// THE SAME 64 BITS, READ AS THE SLOT'S TYPE READS THEM NATIVELY.
fn event_value(kind: EventSlot, record: u64) -> Value {
    match kind {
        EventSlot::Unsigned => Value::Int(i128::from(record)),
        EventSlot::Signed => Value::Int(i128::from(record as i64)),
        // The bit pattern itself, never canonicalised: the host stores the
        // word, and only an arithmetic instruction canonicalises a NaN.
        EventSlot::Float => Value::Float(record),
    }
}

// ── `W-377`: the socket device, replayed from its log (option (b)) ─────────

/// The socket device's NEXT register: the interpreter's own copy of
/// `yantra::socket::NEXT` (`sadhana` does not depend on `yantra`);
/// `tests/w377_sockets.rs` fails if any of these copies differs from yantra's.
pub const SOCK_NEXT: u64 = 0x1000_0110;
/// RX, the copy of `yantra::socket::RX`.
pub const SOCK_RX: u64 = 0x1000_0114;
/// TX, the copy of `yantra::socket::TX`.
pub const SOCK_TX: u64 = 0x1000_0118;
/// The reserved CLOSE word, the copy of `yantra::socket::CLOSE`.
pub const SOCK_CLOSE: u64 = 0x1000_011c;
/// The session cap, the copy of `yantra::socket::SESSION_CAP` (64 MiB delivered).
pub const SOCK_SESSION_CAP: u64 = 64 << 20;
/// The cap's refusal name, the copy of `yantra::socket::SOCKET_LIMIT_EXCEEDED` (owner
/// ruling 2026-10-06, copied byte for byte by script).
pub const SOCKET_LIMIT_EXCEEDED: &str = "सङ्केतमात्राप्रतिषेधः";
/// RX's EMPTY, the copy of `yantra::socket::EMPTY`.
pub const SOCK_EMPTY: u32 = 0x100;
/// RX's END, the copy of `yantra::socket::END`.
pub const SOCK_END: u32 = 0x101;
/// RX's UNREAD, the copy of `yantra::socket::UNREAD`.
pub const SOCK_UNREAD: u32 = 0x102;
/// The copy of `yantra::socket::WHY_LOAD_STORE_ONLY`.
pub const SOCK_WHY_LOAD_STORE_ONLY: &str =
    "NEXT and TX are store-only registers: read the socket at RX (W-377)";
/// The copy of `yantra::socket::WHY_STORE_LOAD_ONLY`.
pub const SOCK_WHY_STORE_LOAD_ONLY: &str =
    "RX is a load-only register: store 0 to NEXT to pop the queue into it (W-377)";
/// The copy of `yantra::socket::WHY_NEXT_VALUE`.
pub const SOCK_WHY_NEXT_VALUE: &str =
    "NEXT takes the value 0 only: other values are reserved for a connection id (W-377)";
/// The copy of `yantra::socket::WHY_TX_VALUE`.
pub const SOCK_WHY_TX_VALUE: &str =
    "TX sends one octet: a value above 0xff is refused, never truncated (W-377)";
/// The copy of `yantra::socket::WHY_CLOSE`.
pub const SOCK_WHY_CLOSE: &str = "0x1000_011c is reserved for CLOSE, which is deferred: the program cannot close the \
     connection in this row (W-377)";

/// `yantra::socket::over_cap_refusal`'s twin: the same words, for the same record.
fn sock_over_cap(index: usize, octets: usize, total: u64) -> String {
    format!(
        "{SOCKET_LIMIT_EXCEEDED} (SOCKET_LIMIT_EXCEEDED): record {index} ({octets} octets) \
         would take the session's delivered records past the cap of {SOCK_SESSION_CAP} octets \
         (64 MiB), to {total} — refused, not applied and not logged (W-377)"
    )
}

/// One record of a socket log: `yantra::socket::SockRecord`'s twin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SockRecord {
    /// `s=<hex>`: the octets that arrived.
    Octets(Vec<u8>),
    /// `s=end`: the peer closed.
    End,
}

/// [`Interpreter::set_socket`]'s state: the log and its cursor, the event slot, and the
/// device's queue, latch and counts.
struct SocketReplay {
    log: Vec<SockRecord>,
    slot: String,
    kind: EventSlot,
    delivered: usize,
    queue: std::collections::VecDeque<u8>,
    ended: bool,
    latch: u32,
    received: u64,
    sent: Vec<u8>,
}

// ── `W-376`: threads, the serial subset ─────────────────────────────────

/// The tag in front of the thread COUNT, `"SASTHRDS"` read as a little-endian
/// word: the interpreter's own copy of `yantra::threads::THREADS_TAG`
/// (`sadhana` does not depend on `yantra`); `tests/w376_threads.rs` fails if
/// the two ever differ.
pub const THREADS_TAG: u64 = 0x5344_5248_5453_4153;
/// The tag in front of the THREAD-ID slot, `"SASTHRID"`: the copy of
/// `yantra::threads::THREAD_ID_TAG`.
pub const THREAD_ID_TAG: u64 = 0x4449_5248_5453_4153;
/// The most threads a program may declare, as the native host allows.
pub const MAX_THREADS: u64 = 64;

/// One record of a threaded schedule: `yantra::input::ThreadRecord`'s twin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStep {
    /// A value for the resuming thread's wait.
    Value(u64),
    /// `@N`: run thread `N`.
    Run(u32),
}

/// [`Interpreter::run_threads`]'s state: the schedule, the next record, the
/// thread running now, and the thread-id global.
struct ThreadReplay {
    schedule: Vec<ThreadStep>,
    cursor: usize,
    current: u32,
    id_slot: String,
}

/// How the event global's declared type reads the 64 bits the host writes —
/// the three types [`Interpreter::set_events`] accepts.
#[derive(Debug, Clone, Copy)]
enum EventSlot {
    /// `न६४`: the word, unsigned.
    Unsigned,
    /// `अ६४`: the word as two's complement.
    Signed,
    /// `प६४`: the word as a double's bit pattern.
    Float,
}

/// RISC-V's canonical NaN for a double, `0x7ff8000000000000` — what every
/// D-extension arithmetic instruction writes when its result is NaN, whatever
/// the operands' payloads (`crates/yantra/src/fp.rs`, `CANONICAL_NAN_D`).
pub const CANONICAL_NAN_D: u64 = 0x7ff8_0000_0000_0000;

/// A float result as the machine writes it: any NaN becomes the canonical one.
fn canonical(x: f64) -> Value {
    Value::Float(if x.is_nan() {
        CANONICAL_NAN_D
    } else {
        x.to_bits()
    })
}

/// `fcvt.l.d` under the dynamic rounding mode at its reset value, round to
/// nearest with ties to even — the emitted instruction's `rm` field is `dyn`
/// (`spec/encodings-riscv64.tsv`: funct3 ७) and `fcsr.frm` is ० — with RISC-V's
/// saturation: NaN and anything at or above 2^63 give `i64::MAX`, anything below
/// −2^63 gives `i64::MIN`. Not Rust's `as`, which answers ० for NaN.
fn float_to_i64(x: f64) -> i64 {
    if x.is_nan() {
        return i64::MAX;
    }
    let r = x.round_ties_even();
    if r < -(2f64.powi(63)) {
        i64::MIN
    } else if r >= 2f64.powi(63) {
        i64::MAX
    } else {
        #[allow(clippy::cast_possible_truncation)]
        let v = r as i64;
        v
    }
}

/// Evaluate a float built-in over its already-evaluated arguments, with IEEE
/// semantics (Rust's `f64` is IEEE-754 binary64, round-to-nearest-even, and
/// `mul_add` is the fused operation) and every NaN result canonicalised.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn eval_float_builtin(b: FloatBuiltin, name: &str, vals: &[Value]) -> Result<Value, RunError> {
    let float = |i: usize| -> Result<f64, RunError> {
        match vals.get(i) {
            Some(Value::Float(bits)) => Ok(f64::from_bits(*bits)),
            other => Err(RunError::new(format!(
                "`{FLOAT_BUILTIN_MODULE}\u{971}{name}` takes a `{T_F64}` as argument {}, \
                 and was handed {other:?} (V-005)",
                i + 1
            ))),
        }
    };
    Ok(match b {
        FloatBuiltin::Add => canonical(float(0)? + float(1)?),
        FloatBuiltin::Sub => canonical(float(0)? - float(1)?),
        FloatBuiltin::Mul => canonical(float(0)? * float(1)?),
        FloatBuiltin::MulAdd => canonical(float(0)?.mul_add(float(1)?, float(2)?)),
        FloatBuiltin::Div => canonical(float(0)? / float(1)?),
        FloatBuiltin::Sqrt => canonical(float(0)?.sqrt()),
        // Rust's `==`/`<`/`<=` on `f64` ARE `feq`/`flt`/`fle`: false for any NaN,
        // and `-0 == +0`.
        FloatBuiltin::Eq => Value::Bool(float(0)? == float(1)?),
        FloatBuiltin::Lt => Value::Bool(float(0)? < float(1)?),
        FloatBuiltin::Le => Value::Bool(float(0)? <= float(1)?),
        // The low sixty-four bits are what the register holds: `as i64` takes them.
        FloatBuiltin::Convert => match vals.first() {
            Some(Value::Int(i)) => canonical((*i as i64) as f64),
            Some(Value::Float(_)) => Value::Int(i128::from(float_to_i64(float(0)?))),
            other => {
                return Err(RunError::new(format!(
                    "`{FLOAT_BUILTIN_MODULE}\u{971}{name}` converts an integer or a `{T_F64}`, \
                     and was handed {other:?} (V-005)"
                )));
            }
        },
        // A bit move: no rounding, no canonicalisation — a NaN's payload passes.
        FloatBuiltin::Move => match vals.first() {
            Some(Value::Int(i)) => Value::Float(*i as u64),
            Some(Value::Float(bits)) => Value::Int(i128::from(*bits as i64)),
            other => {
                return Err(RunError::new(format!(
                    "`{FLOAT_BUILTIN_MODULE}\u{971}{name}` moves an integer or a `{T_F64}`, \
                     and was handed {other:?} (V-005)"
                )));
            }
        },
    })
}

/// `V-005` — A VALUE CROSSING INTO A PLACE WHOSE FILE ITS DECLARATION FIXES —
/// a local, a parameter, a routine's result — must already be in that file:
/// a `प६४` takes only a float, anything else never a float. Otherwise the ONE
/// named cause, `FileMismatch`, which `ir.t1` raises for the same shapes
/// natively (`chain.rs`'s `file_mismatch_site`). Fail closed (review ruling,
/// 2026-10-05): the explicit bit move `अष्टकॱप्लवसंचारः` is the way across, and
/// no place moves a value by its bits on the writer's behalf.
fn refuse_file_mismatch(ty: &Ty, v: &Value, place: &str) -> Result<(), RunError> {
    let wants_float = matches!(ty, Ty::Named(n) if n == T_F64);
    if wants_float != matches!(v, Value::Float(_)) {
        return Err(file_mismatch_error(place));
    }
    Ok(())
}

fn file_mismatch_error(place: &str) -> RunError {
    RunError::new(format!(
        "FileMismatch: {place} is declared in the other register file from the value \
         handed to it (a `{T_F64}` takes only a float, anything else never one); the \
         explicit bit move `{FLOAT_BUILTIN_MODULE}\u{971}प्लवसंचारः` is the way across (V-005)"
    ))
}

fn binop(op: BinOp, a: &Value, b: &Value) -> Result<Value, RunError> {
    // `V-005`: AN INTEGER OPERATOR NEVER MEETS A FLOAT. The native builder
    // refuses the same shape, so a `योगः` over two `प६४`s cannot add their bit
    // patterns on one engine and fail on the other. The float operations are the
    // `अष्टक` built-ins, by name.
    if matches!(a, Value::Float(_)) || matches!(b, Value::Float(_)) {
        return Err(RunError::new(format!(
            "the integer operator {op:?} was handed a `{T_F64}`; a float is added, compared \
             and converted with the `{FLOAT_BUILTIN_MODULE}` float built-ins (V-005)"
        )));
    }
    match op {
        BinOp::Eq => return Ok(Value::Bool(a == b)),
        BinOp::Ne => return Ok(Value::Bool(a != b)),
        _ => {}
    }
    let (x, y) = (a.int()?, b.int()?);
    // `W-381` STAGE 3 (owner ruling P1, 2026-10-05): "64-bit two's-complement
    // WRAPPING is the defined semantics". Every arm below computes what the RV64
    // instruction the native code uses computes, on the operands' 64-bit WORDS:
    // `add`/`sub`/`mul` keep the low 64 bits; a shift reads its count's low SIX
    // bits (`sll`/`srl`/`sra`), so `१ वामसृ ६४` is १ and `वामसृ ६५` is `वामसृ १`;
    // `div`/`rem` are the SIGNED pair natively, and ऋण२^६३ ÷ ऋण१ is ऋण२^६३ (RV64's
    // defined overflow quotient, `wrapping_div`); an ordering comparison is
    // signed, or unsigned when `mark_unsigned_operators` made it so.
    //
    // DIVISION AND REMAINDER BY ZERO ARE REFUSED (owner ruling (b), 2026-10-06),
    // here and natively — never RV64's all-ones / dividend.
    #[allow(clippy::cast_possible_truncation)]
    let count = (word(y) & 63) as u32;
    Ok(match op {
        BinOp::Add => Value::Int(wrap(x + y)),
        BinOp::Sub => Value::Int(wrap(x - y)),
        BinOp::Mul => Value::Int(wrap(x.wrapping_mul(y))),
        BinOp::Div => {
            if word(y) == 0 {
                return Err(division_by_zero("division"));
            }
            Value::Int(i128::from(signed(x).wrapping_div(signed(y))))
        }
        BinOp::Rem => {
            if word(y) == 0 {
                return Err(division_by_zero("remainder"));
            }
            Value::Int(i128::from(signed(x).wrapping_rem(signed(y))))
        }
        BinOp::DivU => {
            if word(y) == 0 {
                return Err(division_by_zero("division"));
            }
            Value::Int(i128::from(word(x) / word(y)))
        }
        BinOp::RemU => {
            if word(y) == 0 {
                return Err(division_by_zero("remainder"));
            }
            Value::Int(i128::from(word(x) % word(y)))
        }
        BinOp::Shl => Value::Int(wrap(x << count)),
        // `sra`: the word read signed, shifted with its sign.
        BinOp::Shr => Value::Int(i128::from(signed(x) >> count)),
        // `W-333` — THE LOGICAL SHIFT, for a left operand that is a name declared
        // unsigned (`srl`): the word read unsigned, shifted with zeros. A count of
        // ६४ is a count of ० and returns the operand unchanged, as `srl` does —
        // the `STATE.md` shift-by-64 note this arm once deferred, answered by P1.
        BinOp::ShrL => Value::Int(i128::from(word(x) >> count)),
        BinOp::And => Value::Int(wrap(x & y)),
        BinOp::Or => Value::Int(wrap(x | y)),
        BinOp::Xor => Value::Int(wrap(x ^ y)),
        BinOp::Lt => Value::Bool(signed(x) < signed(y)),
        BinOp::Gt => Value::Bool(signed(x) > signed(y)),
        BinOp::Ge => Value::Bool(signed(x) >= signed(y)),
        BinOp::LtU => Value::Bool(word(x) < word(y)),
        BinOp::GtU => Value::Bool(word(x) > word(y)),
        BinOp::GeU => Value::Bool(word(x) >= word(y)),
        BinOp::Eq | BinOp::Ne => unreachable!("handled above"),
    })
}

/// `W-381` stage 3 (owner ruling (b)): a division or remainder by zero is
/// REFUSED on both engines, natively by the zero test `ir.t1` writes before the
/// `div`/`rem`. The text keeps "division by" / "remainder by" and adds the code.
fn division_by_zero(what: &str) -> RunError {
    RunError::new(format!(
        "{what} by ० refused as `{DIVISION_BY_ZERO_REFUSAL_NAME}` (W-381, finisher word \
         {DIVISION_BY_ZERO_REFUSAL:#x})"
    ))
}

/// `W-381` stage 3: the narrow integer element types a run can be declared
/// with, as (bits, signed). `अ८` is not here: its run is an `Octets`, unsigned,
/// and `octet_of` truncates it. `न८` is an arena of whole values natively read
/// with `lb` and masked to eight bits, so it is unsigned eight.
fn narrow_width(ty: &str) -> Option<(u32, bool)> {
    let signed = ty.starts_with(T_I64.chars().next()?);
    let bits = match ty.chars().skip(1).collect::<String>().as_str() {
        "८" => 8,
        "१६" => 16,
        "३२" => 32,
        _ => return None,
    };
    let unsigned = ty.starts_with(T_U64.chars().next()?);
    if signed && bits == 8 {
        return None;
    }
    (signed || unsigned).then_some((bits, signed))
}

/// A value stored into a `bits`-wide element and read back: the low `bits`,
/// zero- or sign-extended (`lbu`/`lhu`/`lwu`, `lh`/`lw`).
fn narrow_value(x: i128, bits: u32, signed: bool) -> i128 {
    let low = word(x) & ((1u64 << bits) - 1);
    if signed && low >> (bits - 1) == 1 {
        i128::from(low) - (1i128 << bits)
    } else {
        i128::from(low)
    }
}

/// Owner ruling (a), 2026-10-06: a value stored into an octet run keeps its LOW
/// EIGHT BITS (३०० is ४४), exactly as the native `sb` does.
#[allow(clippy::cast_possible_truncation)]
fn octet_of(x: i128) -> u8 {
    (word(x) & 0xff) as u8
}

/// `W-381` stage 3 — THE 64-BIT WORD an integer stands for: its low 64 bits.
///
/// `Value::Int` is an `i128`, and a word whose top bit is set has TWO spellings
/// in it: `ऋण१` (from a `ऋण` numeral or a subtraction) and २^६४−१ (from a
/// `न६४` numeral, an unsigned event record). Neither is "the" value: every
/// reader that can tell them apart — a comparison, a division, a right shift,
/// equality — reads the word, signed or unsigned as the native instruction
/// does, so the spelling never decides an answer.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn word(x: i128) -> u64 {
    x as u64
}

/// The same word read as two's complement.
#[allow(clippy::cast_possible_truncation)]
fn signed(x: i128) -> i64 {
    x as i64
}

/// `W-381` stage 3 — AN EXACT RESULT BROUGHT BACK TO 64 BITS. A value already in
/// `[−२^६३, २^६४)`, the span both readings of a word cover, is KEPT as it is, so
/// every result that fitted before this stage is spelled as it was (the corpus's
/// tests compare against those spellings); anything outside keeps its low 64
/// bits, read unsigned.
fn wrap(x: i128) -> i128 {
    if (i128::from(i64::MIN)..=i128::from(u64::MAX)).contains(&x) {
        x
    } else {
        i128::from(word(x))
    }
}

// ── the top level of a source ────────────────────────────────────────────

struct FnHead {
    name: String,
    line: usize,
    params: Vec<(String, Ty)>,
    returns: Option<Ty>,
    public: bool,
    body: Vec<Token>,
}

struct Head {
    module: String,
    structs: HashMap<String, Vec<(String, Ty)>>,
    globals: Vec<(String, Ty, Vec<Token>)>,
    /// `गणना` variants, in declaration order — `(name, ordinal)`.
    ///
    /// Held apart from `globals` because a global carries INITIALISER TOKENS
    /// and a variant has none: its value is its position, and synthesising a
    /// numeral token to say so would put a lie in the token stream.
    enum_variants: Vec<(String, i128)>,
    /// `W-223`: enums by name with their variants in order, the imports, and
    /// the names declared `सार्वजनिक`.
    enums: Vec<(String, Vec<String>)>,
    imports: Vec<String>,
    public: HashSet<String>,
    /// `W-223`: every `संरचना` in declaration order, DUPLICATES INCLUDED —
    /// `structs` is keyed by name and keeps the last; the store keeps both, and
    /// the census names a name declared twice (`encode.t1`'s `प्रतीक्षा`).
    struct_decls: Vec<(String, Vec<(String, Ty)>)>,
    fns: Vec<FnHead>,
}

/// Read a source's declarations without parsing any body.
fn read_head(tokens: &[Token], label: &str) -> Result<Head, RunError> {
    let mut head = Head {
        module: label.to_string(),
        structs: HashMap::new(),
        globals: Vec::new(),
        enum_variants: Vec::new(),
        enums: Vec::new(),
        imports: Vec::new(),
        public: HashSet::new(),
        struct_decls: Vec::new(),
        fns: Vec::new(),
    };
    let mut i = 0usize;
    let text = |i: usize| tokens.get(i).map(|t| t.text.as_str()).unwrap_or("");
    // `सार्वजनिक` marks the declaration that follows it (`W-223`).
    let mut public = false;

    while i < tokens.len() {
        let was_public = public;
        public = false;
        match text(i) {
            W_MODULE => {
                head.module = text(i + 1).to_string();
                i += 2;
            }
            W_PUBLIC => {
                public = true;
                i += 1;
            }
            W_IMPORT => {
                head.imports.push(text(i + 1).to_string());
                i += 2;
            }
            W_STRUCT => {
                let name = text(i + 1).to_string();
                if was_public {
                    head.public.insert(name.clone());
                }
                // `संरचना NAME आरभ्य (FIELD ॱॱ TYPE ऽ?)* समाप्तम्`
                let mut j = i + 2;
                if text(j) == W_GROUP_OPEN {
                    j += 1;
                }
                let mut fields = Vec::new();
                while j < tokens.len() && text(j) != W_GROUP_CLOSE {
                    let fname = text(j).to_string();
                    j += 1;
                    if tokens.get(j).map(|t| &t.kind) == Some(&Kind::LabelMark) {
                        j += 1;
                        let (ty, next) = read_type(tokens, j)?;
                        fields.push((fname, ty));
                        j = next;
                    }
                    if tokens.get(j).map(|t| &t.kind) == Some(&Kind::Separator) {
                        j += 1;
                    }
                }
                head.struct_decls.push((name.clone(), fields.clone()));
                head.structs.insert(name, fields);
                i = j + 1;
            }
            W_ENUM => {
                // `गणना NAME आरभ्य A ऽ B ऽ C समाप्तम् ।`
                //
                // THIS ARM USED TO SKIP THE BLOCK AND REGISTER NOTHING, so a
                // variant was not a value: `सङ्कुचितम्` answered "is not a name
                // in scope" for a name the corpus declares. Five stubs in
                // `encode.t1` branch on or pass one, and every `गणना` in the
                // tree — `कोश ॱ संज्ञाखण्ड`, `वास्तु ॱ स्थापन` — was equally
                // unreachable. Found by an agent that tried to USE one.
                //
                // A VARIANT'S VALUE IS ITS POSITION, ZERO-BASED, which is what
                // the Rust originals carry: `Placement::Text` is 0 and
                // `#[default] Undefined` is 4 by position, not by annotation.
                // Deliberately NOT one-based — the one-based convention in this
                // corpus belongs to the ARENAS, whose slot ० is never a live
                // entry, and conflating the two is a defect this tree has
                // already paid for twice.
                //
                // `W-223`: the enum's NAME and its variants in order are kept
                // too, for the declaration store's twin.
                let enum_name = text(i + 1).to_string();
                if was_public {
                    head.public.insert(enum_name.clone());
                }
                let mut j = i + 2;
                if text(j) == W_GROUP_OPEN {
                    j += 1;
                }
                let mut ordinal: i128 = 0;
                let mut names = Vec::new();
                while j < tokens.len() && text(j) != W_GROUP_CLOSE {
                    if tokens.get(j).map(|t| &t.kind) == Some(&Kind::Separator) {
                        j += 1;
                        continue;
                    }
                    head.enum_variants.push((text(j).to_string(), ordinal));
                    names.push(text(j).to_string());
                    ordinal += 1;
                    j += 1;
                }
                head.enums.push((enum_name, names));
                i = j + 1;
            }
            W_LET => {
                // `चरः NAME ॱॱ TYPE भवति <expr> ।`
                let name = text(i + 1).to_string();
                if was_public {
                    head.public.insert(name.clone());
                }
                let mut j = i + 2;
                if tokens.get(j).map(|t| &t.kind) == Some(&Kind::LabelMark) {
                    j += 1;
                }
                let (ty, next) = read_type(tokens, j)?;
                j = next;
                if text(j) == W_BECOMES {
                    j += 1;
                }
                let start = j;
                while j < tokens.len() && tokens[j].kind != Kind::Danda {
                    j += 1;
                }
                head.globals.push((name, ty, tokens[start..j].to_vec()));
                i = j + 1;
            }
            W_FN => {
                let line = tokens[i].line;
                let name = text(i + 1).to_string();
                let mut j = i + 2;
                let mut params = Vec::new();
                if text(j) == W_TAKES {
                    j += 1;
                    while j < tokens.len() && text(j) != W_GIVES && text(j) != W_BLOCK_OPEN {
                        let pname = text(j).to_string();
                        j += 1;
                        if tokens.get(j).map(|t| &t.kind) != Some(&Kind::LabelMark) {
                            return Err(RunError::new(format!(
                                "{label}:{line}: `{name}` parameter `{pname}` has no `\u{971}\u{971}` type mark"
                            )));
                        }
                        j += 1;
                        let (ty, next) = read_type(tokens, j)?;
                        params.push((pname, ty));
                        j = next;
                        if tokens.get(j).map(|t| &t.kind) == Some(&Kind::Separator) {
                            j += 1;
                        }
                    }
                }
                let mut returns = None;
                if text(j) == W_GIVES {
                    j += 1;
                    let (ty, next) = read_type(tokens, j)?;
                    returns = Some(ty);
                    j = next;
                }
                if text(j) != W_BLOCK_OPEN {
                    // A declaration with no body — nothing in the corpus writes
                    // one, but refusing to load the whole file over it would be
                    // the wrong trade.
                    i = j;
                    continue;
                }
                j += 1;
                let start = j;
                let mut depth = 1usize;
                while j < tokens.len() && depth > 0 {
                    match text(j) {
                        W_BLOCK_OPEN => depth += 1,
                        W_BLOCK_CLOSE => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                let end = j.saturating_sub(1);
                head.fns.push(FnHead {
                    name,
                    line,
                    params,
                    returns,
                    public: was_public,
                    body: tokens[start..end].to_vec(),
                });
                i = j;
            }
            _ => i += 1,
        }
    }
    Ok(head)
}

/// Read one type starting at `at`; answer it and the index after it.
fn read_type(tokens: &[Token], at: usize) -> Result<(Ty, usize), RunError> {
    let text = |i: usize| tokens.get(i).map(|t| t.text.as_str()).unwrap_or("");
    match text(at) {
        W_INDEX_OPEN => {
            if text(at + 1) != W_INDEX_CLOSE {
                return Err(RunError::new(format!(
                    "a slice type is `{W_INDEX_OPEN} {W_INDEX_CLOSE} <type>`, found `{}`",
                    text(at + 1)
                )));
            }
            let (inner, next) = read_type(tokens, at + 2)?;
            Ok((Ty::Slice(Box::new(inner)), next))
        }
        W_POINTER => {
            let (inner, next) = read_type(tokens, at + 1)?;
            Ok((Ty::Pointer(Box::new(inner)), next))
        }
        W_OPTIONAL => {
            let (inner, next) = read_type(tokens, at + 1)?;
            Ok((Ty::Optional(Box::new(inner)), next))
        }
        W_ERRUNION => {
            let (inner, next) = read_type(tokens, at + 1)?;
            Ok((Ty::ErrorUnion(Box::new(inner)), next))
        }
        "" => Err(RunError::new("a type was expected and the source ended")),
        name => Ok((Ty::Named(name.to_string()), at + 1)),
    }
}

// ── the body parser ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct ParseCtx<'a> {
    sigs: &'a HashMap<String, usize>,
    modules: &'a [String],
    /// THE MODULE BEING PARSED, so a bare call can mean this module's own
    /// routine — which is what `resolve_call` already does at run time.
    ///
    /// Without it the arity of a bare name came from a FLAT map in which
    /// whichever module loaded last owned the key, so the parser consumed the
    /// wrong number of arguments while dispatch found the right routine. Two
    /// resolutions, two answers, and only the parser's was silent.
    module: &'a str,
    /// `V-008` part 2: the module's own GLOBAL names, so a spaced `व्यूह ॱ x`
    /// can tell a bound `व्यूह` (a field access) from an unbound one (refused).
    /// Locals and parameters are already the parser's `locals`.
    globals: &'a [String],
    /// What this module's `आयातः` lines name — every module it says it uses,
    /// loaded or not. `modules` lists only what the loader was HANDED, so it
    /// cannot tell `सङ्केतन ॱ पङ्क्तिसीमा` (a call into an absent module) from
    /// `रङ्गः ॱ नीलः` (a variant qualified by its type); `आयातः` declares the
    /// name to be a module, so this can. Read by the diagnostic only
    /// ([`Parser::unresolved_note`]); it changes no parse.
    imports: &'a [String],
    /// Every loaded module's globals and `गणना` variants, qualified
    /// (`परीक्षाॱसीमा`). Read by the diagnostic only.
    data: &'a HashSet<String>,
}

struct Parser<'a> {
    t: &'a [Token],
    at: usize,
    ctx: ParseCtx<'a>,
    /// Every MODULE-QUALIFIED name this parse read as a variable because no
    /// routine answered it, in the order met (`W-358`).
    unresolved: Vec<String>,
}

impl<'a> Parser<'a> {
    fn new(t: &'a [Token], ctx: ParseCtx<'a>) -> Self {
        Self {
            t,
            at: 0,
            ctx,
            unresolved: Vec::new(),
        }
    }

    /// Whether `name` is a module as far as this parse can know: one the
    /// load carries or one this module imports.
    fn names_a_module(&self, name: &str) -> bool {
        self.ctx.modules.iter().any(|m| m == name) || self.ctx.imports.iter().any(|m| m == name)
    }

    /// What to append to a FAILED body parse so it lists the qualified names
    /// read as variables because nothing loaded answered them, or `""` when
    /// there were none (`W-358`, replayed from the `W-304` tick's rescue ref).
    ///
    /// `primary`'s last line reads a name nothing declares as a VARIABLE, and
    /// a variable takes no arguments. For a qualified CALL whose module the
    /// load does not carry, every argument the call site wrote is then left in
    /// the token stream and the parse dies at whatever follows them — a
    /// reported line that is not the fault's (`expected आदि, found पाठ्यम्`).
    /// The list is CANDIDATES, not the cause: a name in an unloaded module may
    /// equally be one of its globals, read correctly, and this parser cannot
    /// tell the two apart without that module's head. The fallback itself is
    /// unchanged and only an already-failing parse reads this, so no
    /// routine's `is_runnable` moves.
    fn unresolved_note(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (i, name) in self.unresolved.iter().enumerate() {
            if self.unresolved[..i].contains(name) {
                continue;
            }
            let (m, n) = name.split_once('\u{971}').unwrap_or((name.as_str(), ""));
            // TWO DIFFERENT REPAIRS: an absent module is a loader's list to
            // widen, a present module without the name is a corpus fault. A
            // loaded module's globals and variants were never recorded.
            if self.ctx.modules.iter().any(|x| x == m) {
                parts.push(format!(
                    "`{name}` (module `{m}` is loaded and declares no routine or global `{n}`)"
                ));
            } else {
                parts.push(format!(
                    "`{name}` (a routine or global of module `{m}`, not loaded)"
                ));
            }
        }
        if parts.is_empty() {
            return String::new();
        }
        let one = parts.len() == 1;
        format!(
            " — {} qualified name{} matched nothing loaded and {} read as a variable taking \
             no arguments; a call among them would leave its arguments unparsed: {}",
            parts.len(),
            if one { "" } else { "s" },
            if one { "was" } else { "were" },
            parts.join(", ")
        )
    }

    fn text(&self, off: usize) -> &str {
        self.t
            .get(self.at + off)
            .map(|t| t.text.as_str())
            .unwrap_or("")
    }
    fn kind(&self, off: usize) -> Option<&Kind> {
        self.t.get(self.at + off).map(|t| &t.kind)
    }
    fn done(&self) -> bool {
        self.at >= self.t.len()
    }
    fn eat(&mut self, word: &str) -> bool {
        if self.text(0) == word {
            self.at += 1;
            return true;
        }
        false
    }
    fn expect(&mut self, word: &str) -> Result<(), RunError> {
        if self.eat(word) {
            return Ok(());
        }
        Err(RunError::new(format!(
            "expected `{word}`, found `{}` on line {}",
            self.text(0),
            self.t.get(self.at).map_or(0, |t| t.line)
        )))
    }

    /// Statements up to the end of the token run.
    fn block(&mut self, locals: &mut Vec<String>) -> Result<Vec<Stmt>, RunError> {
        let mut out = Vec::new();
        while !self.done() {
            out.push(self.statement(locals)?);
        }
        Ok(out)
    }

    /// Statements up to a matching `इति`, which is consumed.
    fn inner_block(&mut self, locals: &mut Vec<String>) -> Result<Vec<Stmt>, RunError> {
        let mut out = Vec::new();
        let depth = locals.len();
        while !self.done() && self.text(0) != W_BLOCK_CLOSE {
            out.push(self.statement(locals)?);
        }
        self.expect(W_BLOCK_CLOSE)?;
        locals.truncate(depth);
        Ok(out)
    }

    fn statement(&mut self, locals: &mut Vec<String>) -> Result<Stmt, RunError> {
        // W-348. `अन्यथा` is read in ONE place — after the `इति` that closes a
        // `यदि` block, by the arm just below. A statement that BEGINS with it
        // is therefore the keyword in the wrong place: written inside the
        // block, where other languages put their `else`. It used to fall
        // through to an expression statement, become `Expr::Var("अन्यथा")`,
        // and be reported AT RUN TIME as "not a name in scope" — which sends
        // the reader after a missing declaration, and only when that statement
        // was reached: with the branch not taken the program ran and answered.
        // So it is refused here, when the body is parsed, with the construct
        // it belongs to.
        //
        // NOT WHEN A LOCAL OF THAT NAME IS IN SCOPE. The front end lets
        // `अन्यथा` be declared (`a-keyword-used-as-a-name-reads-as-nil`), and
        // closing that is another row's; this one must not turn such a
        // program's assignment into a refusal.
        //
        // THE ADVICE IS CONDITIONAL, since the follow-up of 2026-10-03. The
        // first wording said "so it is inside a block: close the `यदि` block
        // first", which is false in three shapes this arm also catches: after
        // a `यावत्` block's `इति`, as a SECOND `अन्यथा` after an else block,
        // and at a routine's top level with no `यदि` at all. What is true in
        // every shape is that it begins a statement; where the `यदि` is, the
        // parser does not know here. And it names the line, as `expect` does.
        //
        // THE EXEMPTION IS `locals` AND NOTHING WIDER: parameters and `चरः`
        // locals. A MODULE-LEVEL name spelled `अन्यथा` is not in `locals`, so
        // a statement beginning with it is refused — pinned by
        // `a_module_level_name_spelled_like_the_keyword_is_not_exempt`.
        if self.text(0) == W_ELSE && !locals.iter().any(|l| l == W_ELSE) {
            return Err(RunError::new(format!(
                "`{W_ELSE}` is the else keyword and is read only after the `{W_BLOCK_CLOSE}` \
                 that closes a `{W_IF}` block — `{W_IF} … {W_BLOCK_OPEN} … {W_BLOCK_CLOSE} \
                 {W_ELSE} {W_BLOCK_OPEN} … {W_BLOCK_CLOSE}`. On line {} it begins a statement \
                 instead. If it is inside a `{W_IF}` block, close that block with \
                 `{W_BLOCK_CLOSE}` first",
                self.t.get(self.at).map_or(0, |t| t.line)
            )));
        }
        if self.eat(W_IF) {
            let cond = self.expression(locals)?;
            self.expect(W_BLOCK_OPEN)?;
            let then = self.inner_block(locals)?;
            let els = if self.eat(W_ELSE) {
                self.expect(W_BLOCK_OPEN)?;
                self.inner_block(locals)?
            } else {
                Vec::new()
            };
            return Ok(Stmt::If { cond, then, els });
        }
        if self.eat(W_WHILE) {
            let cond = self.expression(locals)?;
            self.expect(W_BLOCK_OPEN)?;
            let body = self.inner_block(locals)?;
            return Ok(Stmt::While { cond, body });
        }
        if self.eat(W_RETURN) {
            let e = if self.kind(0) == Some(&Kind::Danda) {
                None
            } else {
                Some(self.expression(locals)?)
            };
            if self.kind(0) == Some(&Kind::Danda) {
                self.at += 1;
            }
            return Ok(Stmt::Return(e));
        }
        if self.eat(W_LET) {
            let name = self.text(0).to_string();
            if name.is_empty() {
                return Err(RunError::new("`चरः` names nothing"));
            }
            self.at += 1;
            if self.kind(0) == Some(&Kind::LabelMark) {
                self.at += 1;
            }
            let (ty, next) = read_type(self.t, self.at)?;
            self.at = next;
            self.expect(W_BECOMES)?;
            let init = self.expression(locals)?;
            if self.kind(0) == Some(&Kind::Danda) {
                self.at += 1;
            }
            locals.push(name.clone());
            return Ok(Stmt::Let { name, ty, init });
        }
        let e = self.expression(locals)?;
        if self.eat(W_BECOMES) {
            let value = self.expression(locals)?;
            if self.kind(0) == Some(&Kind::Danda) {
                self.at += 1;
            }
            return Ok(Stmt::Assign { target: e, value });
        }
        if self.kind(0) == Some(&Kind::Danda) {
            self.at += 1;
        }
        Ok(Stmt::Eval(e))
    }

    // Precedence, loosest first. See the module header for why it is C's.
    fn expression(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        self.comparison(locals)
    }

    fn comparison(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        let mut l = self.bitor(locals)?;
        loop {
            let op = match self.text(0) {
                "समम्" => BinOp::Eq,
                "असमम्" => BinOp::Ne,
                "न्यूनम्" => BinOp::Lt,
                "अधिकम्" => BinOp::Gt,
                "बृहत्समम्" => BinOp::Ge,
                _ => return Ok(l),
            };
            self.at += 1;
            let r = self.bitor(locals)?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn bitor(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        let mut l = self.bitand(locals)?;
        loop {
            // `विषम` shares this level with `विकल्प`: `द्विकर्मस्तरः`
            // (parse.t1) gives both kinds level २, not C's | < ^ (W-335).
            let op = match self.text(0) {
                "विकल्प" => BinOp::Or,
                "विषम" => BinOp::Xor,
                _ => return Ok(l),
            };
            self.at += 1;
            let r = self.bitand(locals)?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn bitand(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        let mut l = self.shift(locals)?;
        while self.text(0) == "युक्" {
            self.at += 1;
            let r = self.shift(locals)?;
            l = Expr::Bin(BinOp::And, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn shift(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        let mut l = self.additive(locals)?;
        loop {
            let op = match self.text(0) {
                "वामसृ" => BinOp::Shl,
                "दक्षिणसृ" => BinOp::Shr,
                _ => return Ok(l),
            };
            self.at += 1;
            let r = self.additive(locals)?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn additive(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        let mut l = self.multiplicative(locals)?;
        loop {
            let op = match self.text(0) {
                "योगः" => BinOp::Add,
                "वियोगः" => BinOp::Sub,
                _ => return Ok(l),
            };
            self.at += 1;
            let r = self.multiplicative(locals)?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn multiplicative(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        let mut l = self.postfix(locals)?;
        loop {
            let op = match self.text(0) {
                "गुणनम्" => BinOp::Mul,
                "विभाजनम्" => BinOp::Div,
                "शेषः" => BinOp::Rem,
                _ => return Ok(l),
            };
            self.at += 1;
            let r = self.postfix(locals)?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    /// An atom and everything written after it: `ॱ member`, `अङ्कः i अन्तः`,
    /// `अङ्कः a अन्तः b`.
    ///
    /// **This is also the level a call argument is parsed at**, and that is
    /// forced rather than chosen: an argument list has no delimiter, so
    /// `अष्टकान्वेषणम् पाठ्यम् आरम्भः पाठ्यम् ॱ दैर्घ्य १०` can only mean four
    /// arguments if an argument stops before an infix operator would begin.
    fn postfix(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        let mut e = self.primary(locals)?;
        loop {
            if self.kind(0) == Some(&Kind::MemberMark) {
                let name = self.text(1).to_string();
                if name.is_empty() {
                    return Err(RunError::new("`\u{971}` names no member"));
                }
                self.at += 2;
                e = Expr::Field(Box::new(e), name);
                continue;
            }
            if self.text(0) == W_INDEX_OPEN {
                self.at += 1;
                let i = self.expression(locals)?;
                self.expect(W_INDEX_CLOSE)?;
                // An index CLOSES at `अन्तः`; a slice writes its limit after
                // it. The corpus never writes a bare index followed by another
                // argument, so "the next token can open an atom" separates the
                // two — checked against every `अङ्कः … अन्तः` in `encode.t1`
                // and `vakyavibhaga.t1`.
                if self.opens_atom() {
                    let to = self.postfix(locals)?;
                    e = Expr::Slice(Box::new(e), Box::new(i), Box::new(to));
                } else {
                    e = Expr::Index(Box::new(e), Box::new(i));
                }
                continue;
            }
            return Ok(e);
        }
    }

    /// Whether the `आरभ्य` at the cursor holds a `ऽ` at its own nesting level.
    fn group_has_comma(&self) -> bool {
        let mut depth = 0usize;
        let mut i = self.at;
        while i < self.t.len() {
            match self.t[i].text.as_str() {
                W_GROUP_OPEN => depth += 1,
                W_GROUP_CLOSE => {
                    depth -= 1;
                    if depth == 0 {
                        return false;
                    }
                }
                _ => {
                    if depth == 1 && self.t[i].kind == Kind::Separator {
                        return true;
                    }
                }
            }
            i += 1;
        }
        false
    }

    /// Whether the token at the cursor could begin an atom.
    fn opens_atom(&self) -> bool {
        match self.kind(0) {
            Some(Kind::Numeral | Kind::Str { .. }) => true,
            Some(Kind::Word | Kind::Operand { .. }) => !matches!(
                self.text(0),
                "समम्"
                    | "असमम्"
                    | "न्यूनम्"
                    | "अधिकम्"
                    | "बृहत्समम्"
                    | "विकल्प"
                    | "विषम"
                    | "युक्"
                    | "वामसृ"
                    | "दक्षिणसृ"
                    | "योगः"
                    | "वियोगः"
                    | "गुणनम्"
                    | "विभाजनम्"
                    | "शेषः"
                    | W_BECOMES
                    | W_BLOCK_OPEN
                    | W_BLOCK_CLOSE
                    | W_ELSE
                    | W_GROUP_CLOSE
                    | W_INDEX_CLOSE
                    | W_RETURN
                    | W_LET
                    | W_IF
                    | W_WHILE
            ),
            _ => false,
        }
    }

    fn primary(&mut self, locals: &mut Vec<String>) -> Result<Expr, RunError> {
        if let Some(Kind::Str { value }) = self.kind(0) {
            // ADR-0044 D5: a `वर्णाष्टकम्` literal is the octets the image
            // holds — one per letter — so W-381's two engines read one value.
            // Every other string is its UTF-8, as it always was.
            let bytes = match self.t.get(self.at) {
                Some(t) if crate::devanagari8::is_literal(t) => {
                    crate::devanagari8::literal_octets(t).unwrap_or_default()
                }
                _ => value.as_bytes().to_vec(),
            };
            let bytes = Rc::new(bytes);
            self.at += 1;
            return Ok(Expr::Str(bytes));
        }
        if self.kind(0) == Some(&Kind::Numeral) {
            let text = self.text(0).to_string();
            self.at += 1;
            // `ऋण` sits OUTSIDE the radix prefix (`spec/lexicon.tsv:87`, and
            // `numeral::value`'s own doc), so a numeral is a MAGNITUDE and the
            // sign is applied to it. `numeral::value` refuses a signed literal
            // outright; splitting the sign first is what lets `ऋण१` be read.
            let (negative, bare) = sanskrit_text::numeral::split_sign(&text);
            let n = sanskrit_text::numeral::value(bare).map_err(|e| {
                RunError::new(format!("`{text}` is not a number this run can hold: {e:?}"))
            })?;
            let n = i128::from(n);
            return Ok(Expr::Num(if negative { -n } else { n }));
        }
        // `ऋण <expr>` — the same word applied to something that is not a
        // literal, which `चिह्नितान्तर्भावः` writes as `ऋण सीमा`.
        if self.text(0) == "ऋण" {
            self.at += 1;
            let e = self.postfix(locals)?;
            return Ok(Expr::Bin(BinOp::Sub, Box::new(Expr::Num(0)), Box::new(e)));
        }
        if self.eat(W_GROUP_OPEN) {
            let e = self.expression(locals)?;
            self.expect(W_GROUP_CLOSE)?;
            return Ok(e);
        }
        if self.eat(W_TRUE) {
            return Ok(Expr::Bool(true));
        }
        if self.eat(W_FALSE) {
            return Ok(Expr::Bool(false));
        }
        if self.eat(W_NIL) {
            return Ok(Expr::Nil);
        }

        let name = self.text(0).to_string();
        if name.is_empty() {
            return Err(RunError::new(
                "an expression was expected and the body ended",
            ));
        }
        if !matches!(self.kind(0), Some(Kind::Word | Kind::Operand { .. })) {
            return Err(RunError::new(format!(
                "`{name}` is not the start of an expression (line {})",
                self.t.get(self.at).map_or(0, |t| t.line)
            )));
        }
        self.at += 1;

        // `सङ्केतन ॱ पङ्क्तिसीमा …` — a module-qualified call. Distinguished
        // from `नव ॱ पाठ` (a record member) by the name being a MODULE.
        // `V-005`: a FLOAT BUILT-IN has no declaration, so its arity is the
        // table's and not `sigs`'s — in both spellings, `अष्टकॱप्लवयोगः` (one
        // token) and `अष्टक ॱ प्लवयोगः`, and whether or not `अष्टक` is loaded.
        {
            let spaced = self.kind(0) == Some(&Kind::MemberMark);
            let (m, n) = if spaced {
                (Some(name.as_str()), self.text(1).to_string())
            } else {
                match name.split_once('\u{971}') {
                    Some((m, n)) => (Some(m), n.to_string()),
                    None => (None, String::new()),
                }
            };
            // `W-373`: THE WAIT INTRINSIC — THE ONE-TOKEN SPELLING ONLY, ONE
            // argument, no declaration in `ashtaka.t1`. A spaced `अष्टक ॱ <member>`
            // is not it: it takes the ordinary module path below and is refused
            // there by name, as both compilers refuse an undeclared member.
            if !spaced && wait_builtin(m, &n) {
                let key = format!("{WAIT_BUILTIN_MODULE}\u{971}{n}");
                let args = self.arguments(&key, WAIT_BUILTIN_ARITY, locals)?;
                return Ok(Expr::Call {
                    module: m.map(str::to_string),
                    name: n,
                    args,
                    vector: false,
                    wait: true,
                });
            }
            if let Some(arity) = float_builtin(m, &n)
                .map(|(_, a)| a)
                .or_else(|| checked_builtin(m, &n).map(|_| 2))
            {
                let module = m.map(str::to_string);
                if spaced {
                    self.at += 2;
                }
                let key = format!("{FLOAT_BUILTIN_MODULE}\u{971}{n}");
                let args = self.arguments(&key, arity, locals)?;
                return Ok(Expr::Call {
                    module,
                    name: n,
                    args,
                    vector: false,
                    wait: false,
                });
            }
            // `V-008` part 2: a VECTOR BUILT-IN — THE ONE-TOKEN SPELLING ONLY,
            // three arguments, no `व्यूह` module to load. Owner ruling (B): a
            // spaced `व्यूह ॱ x` is always an ordinary field access (below).
            if !spaced && vector_builtin(m, &n).is_some() {
                let module = m.map(str::to_string);
                let key = format!("{VECTOR_BUILTIN_MODULE}\u{971}{n}");
                let args = self.arguments(&key, VECTOR_BUILTIN_ARITY, locals)?;
                return Ok(Expr::Call {
                    module,
                    name: n,
                    args,
                    vector: true,
                    wait: false,
                });
            }
            // `V-009` (ii): A MATRIX BUILT-IN, the one-token spelling only, its
            // member's arity — the last argument is a layout constant, read by
            // its text when the call is evaluated. A LAYOUT CONSTANT itself is
            // parsed as a vector-flagged call with no arguments: a value nowhere
            // (`eval` refuses it), read only in a matrix call's last position.
            if !spaced && let Some(op) = matrix_builtin(m, &n) {
                let module = m.map(str::to_string);
                let key = format!("{VECTOR_BUILTIN_MODULE}\u{971}{n}");
                let args = self.arguments(&key, op.arity(), locals)?;
                return Ok(Expr::Call {
                    module,
                    name: n,
                    args,
                    vector: true,
                    wait: false,
                });
            }
            if !spaced && layout_constant(m, &n).is_some() {
                return Ok(Expr::Call {
                    module: m.map(str::to_string),
                    name: n,
                    args: Vec::new(),
                    vector: true,
                    wait: false,
                });
            }
            // ANY OTHER `व्यूहॱ` MEMBER IS AN UNDECLARED NAME, refused here with
            // the compilers' own words (`artha.t1` refuses it at resolve:
            // "`…` at line N has no declaration"): `व्यूह` is no module, so no
            // routine can answer it. Without this the call fell through to the
            // argument reader and failed as a PARSE error about the bracket.
            // Narrow on purpose: only the `व्यूह` qualifier is decided here; an
            // unloaded MODULE keeps the old path, because the interpreter loads
            // a partial corpus in many tests and resolves those calls late.
            //
            // SPACED, `व्यूह ॱ x` IS A FIELD ACCESS (owner ruling (B)): a `व्यूह`
            // the routine binds — a local, a parameter, or this module's global
            // — reads its field below like any other name; an UNBOUND one is
            // refused here with the compilers' cause ("`व्यूह` at line N has no
            // declaration"), so the three engines refuse it identically.
            //
            // A REAL MODULE NAMED `व्यूह` IS NOT REFUSED HERE (ADR-0043 reserves
            // nothing): when one is loaded, its calls take the ordinary module
            // path below, in both spellings, as they do on both compilers.
            if m == Some(VECTOR_BUILTIN_MODULE)
                && !locals.contains(&name)
                && !self.ctx.modules.iter().any(|x| x == VECTOR_BUILTIN_MODULE)
            {
                let bound = spaced && self.ctx.globals.contains(&name);
                if !bound {
                    let shown = if spaced {
                        name.clone()
                    } else {
                        format!("{VECTOR_BUILTIN_MODULE}\u{971}{n}")
                    };
                    return Err(RunError::new(format!(
                        "`{shown}` at line {} has no declaration",
                        self.t.get(self.at - 1).map_or(0, |t| t.line)
                    )));
                }
            }
        }
        if self.kind(0) == Some(&Kind::MemberMark) && self.ctx.modules.contains(&name) {
            let target = self.text(1).to_string();
            self.at += 2;
            let key = format!("{name}\u{971}{target}");
            let arity = *self
                .ctx
                .sigs
                .get(&key)
                .ok_or_else(|| RunError::new(format!("`{name}` has no routine `{target}`")))?;
            let args = self.arguments(&key, arity, locals)?;
            return Ok(Expr::Call {
                module: Some(name),
                name: target,
                args,
                vector: false,
                wait: false,
            });
        }

        // A name in scope is a variable; otherwise a routine of this name is a
        // call. A local shadows a routine, which is what a reader of the corpus
        // expects: `चरः पाठ्यम्` is the text, never a routine.
        if locals.contains(&name) {
            return Ok(Expr::Var(name));
        }
        // `W-358`: the SPACED qualified call whose module is not loaded (the
        // branch above needs it loaded). Recorded for the diagnostic only —
        // `self.at` does not move, so the parse proceeds exactly as before.
        if self.kind(0) == Some(&Kind::MemberMark) && self.names_a_module(&name) {
            self.unresolved
                .push(format!("{name}\u{971}{}", self.text(1)));
        }
        // THIS MODULE'S OWN ROUTINE FIRST, then anyone's. `resolve_call`
        // (`:1096`) looks in `self.modules[from]` before falling back, so a
        // bare name at RUN time already means the caller's own routine; this
        // makes the PARSER agree. While it did not, two modules exporting one
        // name with different arities made the parser consume the wrong number
        // of arguments and every token after it misalign — twelve of fifteen
        // corpus files failed to parse the moment `मध्यरूप` was loaded beside
        // `अक्षरकोश`, both of which export `अङ्कमूल्यम्`.
        let own = format!("{}\u{971}{name}", self.ctx.module);
        if let Some(arity) = self
            .ctx
            .sigs
            .get(&own)
            .or_else(|| self.ctx.sigs.get(&name))
            .copied()
        {
            // `पदविभागॱपदविभाग` — the UNSPACED module-qualified call, and the
            // one the corpus actually prefers: 228 of its 260 cross-module
            // call sites are written this way. The branch above handles only
            // `सङ्केतन ॱ पङ्क्तिसीमा`, where the member mark arrives as its
            // own token; here the lexer has produced ONE token with the
            // qualification inside it.
            //
            // WITHOUT THIS SPLIT THE CALL PARSES AND THEN CANNOT DISPATCH.
            // `sigs` is keyed by the qualified name, so the arity above is
            // found and the body reports `runnable`; at run time
            // `resolve_call` is handed `module: None` and looks for a routine
            // whose BARE name is the whole `पदविभागॱपदविभाग`. No routine has
            // that name, so it fails with "no routine named … is loaded" —
            // a routine that is loaded, and that `Interpreter::routine`
            // finds, because THAT function splits the mark and this did not.
            //
            // Found by `वाक्यविभाग ॱ सङ्कलनम्`, which is `D-002f9`'s whole
            // point: it is the one entry point that reaches across to the
            // lexer, and it was the first call to try.
            if let Some((m, n)) = name.split_once('\u{971}')
                && self.ctx.modules.iter().any(|x| x == m)
            {
                let (module, target) = (m.to_string(), n.to_string());
                let args = self.arguments(&name, arity, locals)?;
                return Ok(Expr::Call {
                    module: Some(module),
                    name: target,
                    args,
                    vector: false,
                    wait: false,
                });
            }
            let args = self.arguments(&name, arity, locals)?;
            return Ok(Expr::Call {
                module: None,
                name,
                args,
                vector: false,
                wait: false,
            });
        }
        // `W-358`: the silent fallback. Right for a local, a field or a
        // `गणना` variant; wrong for a qualified call no routine answered, so
        // that kind is recorded for `unresolved_note`.
        if let Some((m, _)) = name.split_once('\u{971}')
            && self.names_a_module(m)
            && !self.ctx.data.contains(&name)
        {
            self.unresolved.push(name.clone());
        }
        Ok(Expr::Var(name))
    }

    /// `n` arguments, juxtaposed, with the corpus's optional `ऽ` between them.
    fn arguments(
        &mut self,
        callee: &str,
        n: usize,
        locals: &mut Vec<String>,
    ) -> Result<Vec<Expr>, RunError> {
        let mut out = Vec::with_capacity(n);
        // `f आरभ्य a ऽ b समाप्तम्` — `vakyavibhaga.t1` writes calls this way and
        // `encode.t1` juxtaposes. Both are the corpus, so both are read, and
        // the `ऽ` is what tells them apart: a juxtaposed call whose FIRST
        // argument happens to be a group (`f आरभ्य a योगः b समाप्तम् c`) has no
        // separator at the group's own level. For one argument the two readings
        // are the same expression, so the ambiguity there costs nothing.
        let grouped = n > 0 && self.text(0) == W_GROUP_OPEN && (n == 1 || self.group_has_comma());
        if grouped {
            self.at += 1;
        }
        for k in 0..n {
            if k > 0 && self.kind(0) == Some(&Kind::Separator) {
                self.at += 1;
            }
            out.push(if grouped {
                self.expression(locals)?
            } else {
                self.postfix(locals)?
            });
        }
        if grouped {
            // A separator still standing here is not a parse fault, it is an
            // ARITY fault at the call site, and saying so is the whole value of
            // reading the corpus rather than matching its text. Three of these
            // are live in `vakyavibhaga.t1` today — see the row report.
            if self.kind(0) == Some(&Kind::Separator) {
                let mut extra = 0usize;
                let mut depth = 1usize;
                let mut i = self.at;
                while i < self.t.len() && depth > 0 {
                    match self.t[i].text.as_str() {
                        W_GROUP_OPEN => depth += 1,
                        W_GROUP_CLOSE => depth -= 1,
                        _ => {
                            if depth == 1 && self.t[i].kind == Kind::Separator {
                                extra += 1;
                            }
                        }
                    }
                    i += 1;
                }
                return Err(RunError::new(format!(
                    "`{callee}` declares {n} parameters and is called with {} on line {}",
                    n + extra,
                    self.t.get(self.at).map_or(0, |t| t.line)
                )));
            }
            self.expect(W_GROUP_CLOSE)?;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A T1 program written here, with no embed, so these tests are about the
    /// LANGUAGE and do not depend on `spec/`. The corpus's own routines are
    /// exercised by `crates/sadhana-t1/tests/t1_execution.rs`, which is where
    /// `D-002j`'s acceptance lives.
    fn run(src: &str, entry: &str, args: Vec<Value>) -> Result<Value, RunError> {
        let mut it = Interpreter::load(&[("test", src)], Path::new("."))?;
        it.call(entry, args, 1_000_000)
    }

    #[test]
    fn a_routine_with_parameters_returns_a_value() {
        // The whole of what `t1::parse` could not do: a name followed by
        // `आदाय` used to make the body invisible.
        let v = run(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः योजनम् आदाय क ॱॱ अ६४ ख ॱॱ अ६४ ददाति अ६४ आदि
                 प्रत्यागमनम् क योगः ख ।
             इति",
            "योजनम्",
            vec![Value::Int(40), Value::Int(2)],
        )
        .expect("runs");
        assert_eq!(v.as_int(), Some(42));
    }

    #[test]
    fn a_loop_runs_and_a_condition_chooses() {
        let v = run(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः सङ्कलनम् आदाय सीमा ॱॱ अ६४ ददाति अ६४ आदि
                 चरः क्रमः ॱॱ अ६४ भवति ० ।
                 चरः योगफलम् ॱॱ अ६४ भवति ० ।
                 यावत् क्रमः न्यूनम् सीमा आदि
                     यदि क्रमः समम् ३ आदि
                         क्रमः भवति क्रमः योगः १ ।
                     इति अन्यथा आदि
                         योगफलम् भवति योगफलम् योगः क्रमः ।
                         क्रमः भवति क्रमः योगः १ ।
                     इति
                 इति
                 प्रत्यागमनम् योगफलम् ।
             इति",
            "सङ्कलनम्",
            vec![Value::Int(6)],
        )
        .expect("runs");
        // ० + १ + २ + ४ + ५ — ३ is the arm that skips.
        assert_eq!(v.as_int(), Some(12));
    }

    #[test]
    fn precedence_is_the_corpus_bracketing() {
        // `पदम् विकल्प द्वितीयः वामसृ ८` is `चतुरष्टकम्`'s own line, and it
        // means `पदम् | (द्वितीयः << ८)`. If `विकल्प` bound tighter it would
        // be `(पदम् | द्वितीयः) << ८` and answer २५६ instead of २५७.
        let v = run(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः प ददाति अ६४ आदि
                 प्रत्यागमनम् १ विकल्प १ वामसृ ८ ।
             इति",
            "प",
            Vec::new(),
        )
        .expect("runs");
        assert_eq!(v.as_int(), Some(257));
    }

    #[test]
    fn vishama_is_xor_on_both_engines_figures() {
        // W-335: the native chain already answered these two; the predict
        // refused the operator outright. The figures are the row's own.
        let src = "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः व आदाय क ॱॱ अ६४ ख ॱॱ अ६४ ददाति अ६४ आदि
                 प्रत्यागमनम् क विषम ख ।
             इति";
        let v = run(src, "व", vec![Value::Int(12), Value::Int(10)]).expect("runs");
        assert_eq!(v.as_int(), Some(6));
        let v = run(
            src,
            "व",
            vec![Value::Int(0xffff_ffff_ffff_ffff), Value::Int(255)],
        )
        .expect("runs");
        assert_eq!(v.as_int(), Some(18446744073709551360));
    }

    #[test]
    fn vishama_shares_vikalpa_s_level_and_folds_left() {
        // `द्विकर्मस्तरः` (parse.t1) gives `विषम` level २, the SAME as
        // `विकल्प` — not C's ^-binds-tighter-than-|. Left-folded at one level,
        // `३ विकल्प १ विषम १` is `(३ | १) ^ १` = २; C's bracketing would
        // answer `३ | (१ ^ १)` = ३.
        let v = run(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः प ददाति अ६४ आदि
                 प्रत्यागमनम् ३ विकल्प १ विषम १ ।
             इति",
            "प",
            Vec::new(),
        )
        .expect("runs");
        assert_eq!(v.as_int(), Some(2));
    }

    #[test]
    fn vishama_without_a_left_operand_is_still_not_a_name() {
        // The refusal that must survive the fix: `विषम` written where an
        // ATOM belongs is a bare name, and no such name is in scope.
        let e = run(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः प ददाति अ६४ आदि
                 प्रत्यागमनम् विषम ।
             इति",
            "प",
            Vec::new(),
        )
        .expect_err("refuses");
        assert!(e.reason.contains("विषम"), "{}", e.reason);
        assert!(e.reason.contains("not a name in scope"), "{}", e.reason);
    }

    #[test]
    fn an_index_closes_at_antah_and_a_slice_carries_a_limit() {
        // The one genuine ambiguity in the corpus's expression syntax:
        // `पाठ अङ्कः क अन्तः` is one octet and
        // `आरभ्य पाठ अङ्कः क अन्तः ख समाप्तम्` is a run. What separates them is
        // whether an atom follows.
        let src = "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः अष्टकम् आदाय पाठ ॱॱ अङ्कः अन्तः अ८ ददाति अ८ आदि
                 प्रत्यागमनम् पाठ अङ्कः १ अन्तः ।
             इति
             सार्वजनिक वृत्तिः खण्डदैर्घ्यम् आदाय पाठ ॱॱ अङ्कः अन्तः अ८ ददाति अ६४ आदि
                 चरः खण्डः ॱॱ अङ्कः अन्तः अ८ भवति आरभ्य पाठ अङ्कः १ अन्तः ३ समाप्तम् ।
                 प्रत्यागमनम् खण्डः ॱ दैर्घ्य ।
             इति";
        let bytes = Value::Octets(Octets::new(&[10, 20, 30, 40]));
        assert_eq!(
            run(src, "अष्टकम्", vec![bytes.clone()])
                .expect("runs")
                .as_int(),
            Some(20)
        );
        assert_eq!(
            run(src, "खण्डदैर्घ्यम्", vec![bytes]).expect("runs").as_int(),
            Some(2)
        );
    }

    #[test]
    fn a_call_site_with_the_wrong_arity_is_named_and_refused() {
        // This is not a hypothetical: SEVEN such call sites are live in
        // `crates/sadhana-t1/src/vakyavibhaga.t1` today, and no test in the
        // tree could see one before this module. They are listed by line in
        // `crates/sadhana-t1/tests/t1_execution.rs`'s `ARITY_FAULTS`.
        let it = Interpreter::load(
            &[(
                "test",
                "मण्डलम् परीक्षा ॥
                 सार्वजनिक वृत्तिः द्वयम् आदाय क ॱॱ अ६४ ऽ ख ॱॱ अ६४ ददाति अ६४ आदि
                     प्रत्यागमनम् क योगः ख ।
                 इति
                 सार्वजनिक वृत्तिः आह्वानम् ददाति अ६४ आदि
                     प्रत्यागमनम् द्वयम् आरभ्य १ ऽ २ ऽ ३ समाप्तम् ।
                 इति",
            )],
            Path::new("."),
        )
        .expect("loads");
        let bad = it.routine("आह्वानम्").expect("declared");
        let why = bad.why_not().expect("does not parse");
        assert!(
            why.contains("declares 2 parameters and is called with 3"),
            "the diagnostic must name the arity: {why}"
        );
        // And the sibling still runs — one bad call site is not a bad file.
        assert!(it.routine("द्वयम्").expect("declared").is_runnable());
    }

    /// The `why_not` of `routine` in a one-source load of `text`.
    fn why_not_of(text: &str, routine: &str) -> String {
        let it = Interpreter::load(&[("test", text)], Path::new(".")).expect("loads");
        it.routine(routine)
            .expect("declared")
            .why_not()
            .expect("does not parse")
            .to_string()
    }

    #[test]
    fn a_body_that_calls_an_unloaded_module_names_it_rather_than_the_next_token() {
        // W-358 (replayed from the rescue ref's 1b7f0d5d). A qualified call
        // whose module is not in the load is read by `primary`'s last line as
        // a VARIABLE, which takes no arguments, so the call's arguments stay in
        // the token stream and the parse dies at whatever follows them —
        // `सङ्केतनॱसङ्केतसूचीरचना` (encode.t1:3284) under any loader without
        // `विश्लेषण` says `expected आदि, found पाठ्यम्` and names neither the
        // module nor the routine it wanted. This is that shape, minimised.
        let why = why_not_of(
            "मण्डलम् परीक्षा ॥
             आयातः सङ्केतन ।
             सार्वजनिक वृत्तिः चयनम् आदाय पाठ्यम् ॱॱ अ६४ ददाति अ६४ आदि
                 यदि सङ्केतनॱक्षेत्रसंख्या पाठ्यम् अधिकम् ४ आदि
                     प्रत्यागमनम् १ ।
                 इति
                 प्रत्यागमनम् ० ।
             इति",
            "चयनम्",
        );
        // The original reason survives: the note is additive.
        assert!(
            why.contains("expected `आदि`"),
            "the parse's own reason must survive: {why}"
        );
        assert!(
            why.contains("`सङ्केतनॱक्षेत्रसंख्या` (a routine or global of module `सङ्केतन`, not loaded)"),
            "the diagnostic must name the call and its absent module: {why}"
        );
        assert!(
            why.contains("taking no arguments"),
            "the diagnostic must say how the name was read: {why}"
        );
    }

    #[test]
    fn an_unloaded_module_is_named_when_the_call_is_written_spaced_too() {
        // 9cbdc3ae's half: `सङ्केतन ॱ क्षेत्रसंख्या`, the member mark its own
        // token. `अक्षरकोशॱभेदपूरणम्` (sanskrit_text.t1:1032) and its two
        // siblings write their call this way, so a rule keyed on the mark
        // inside the token named one of the four `अक्षरकोश` rows and not the
        // other three.
        let why = why_not_of(
            "मण्डलम् परीक्षा ॥
             आयातः सङ्केतन ।
             सार्वजनिक वृत्तिः चयनम् आदाय पाठ्यम् ॱॱ अ६४ ददाति अ६४ आदि
                 यदि सङ्केतन ॱ क्षेत्रसंख्या पाठ्यम् अधिकम् ४ आदि
                     प्रत्यागमनम् १ ।
                 इति
                 प्रत्यागमनम् ० ।
             इति",
            "चयनम्",
        );
        assert!(
            why.contains("`सङ्केतनॱक्षेत्रसंख्या` (a routine or global of module `सङ्केतन`, not loaded)"),
            "the spaced call must be named, joined, with its absent module: {why}"
        );
    }

    #[test]
    fn a_global_of_an_unloaded_module_is_listed_as_a_candidate_not_a_cause() {
        // Review finding on W-358: an unloaded module's GLOBAL is read as a
        // variable correctly — `सङ्केतनॱसीमा` takes no arguments either way —
        // and this parser cannot tell it from a routine without the module's
        // head. Seen on `सङ्केतनॱस्थानसङ्केतनम्` (encode.t1:4516) under any
        // loader without `वाक्यविभाग`: two of its four listed names are that
        // module's globals. So the note lists CANDIDATES in neutral words and
        // claims no cause; here the real fault is the stray `४`.
        let why = why_not_of(
            "मण्डलम् परीक्षा ॥
             आयातः सङ्केतन ।
             सार्वजनिक वृत्तिः दोषः ददाति अ६४ आदि
                 यदि सङ्केतनॱसीमा ४ आदि
                     प्रत्यागमनम् १ ।
                 इति
             इति",
            "दोषः",
        );
        assert!(
            why.contains("`सङ्केतनॱसीमा` (a routine or global of module `सङ्केतन`, not loaded)"),
            "an unloaded module's name is a routine OR a global: {why}"
        );
        assert!(
            !why.contains("misaligned"),
            "the note lists candidates and must not claim the cause: {why}"
        );
    }

    #[test]
    fn a_loaded_module_without_the_routine_is_named_as_a_corpus_fault() {
        // The second of the two repairs the note distinguishes: the module IS
        // in the load and declares no such routine, so widening a loader's
        // list would not help.
        let why = why_not_of(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः चयनम् आदाय पाठ्यम् ॱॱ अ६४ ददाति अ६४ आदि
                 यदि परीक्षाॱक्षेत्रसंख्या पाठ्यम् अधिकम् ४ आदि
                     प्रत्यागमनम् १ ।
                 इति
                 प्रत्यागमनम् ० ।
             इति",
            "चयनम्",
        );
        assert!(
            why.contains(
                "`परीक्षाॱक्षेत्रसंख्या` (module `परीक्षा` is loaded and declares no routine or \
                 global `क्षेत्रसंख्या`)"
            ),
            "a present module without the routine is a different repair: {why}"
        );
    }

    #[test]
    fn a_qualified_read_of_a_loaded_modules_global_carries_no_such_note() {
        // THE CASE THAT MUST STILL BE REFUSED for the loaded-module state:
        // `परीक्षाॱसीमा` is this module's GLOBAL, read correctly as a variable.
        // Measured on main before this rule: `अर्थॱअभिव्यञ्जकप्रकारः`
        // (artha.t1:1089) under `t1_paradigm`'s loader listed fourteen
        // qualified names and most were `वास्तुॱ…भेद` globals of a module the
        // load carries — a note blaming them would point at the wrong thing.
        let why = why_not_of(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक चरः सीमा ॱॱ अ६४ भवति ४ ।
             सार्वजनिक वृत्तिः दोषः ददाति अ६४ आदि
                 यदि परीक्षाॱसीमा ४ आदि
                     प्रत्यागमनम् १ ।
                 इति
             इति",
            "दोषः",
        );
        assert!(
            why.contains("expected `आदि`"),
            "the real fault is reported: {why}"
        );
        assert!(
            !why.contains("qualified name"),
            "a loaded module's global is not an unresolved call: {why}"
        );
    }

    #[test]
    fn a_bare_unknown_name_is_still_a_variable_and_carries_no_such_note() {
        // THE CASE THAT MUST STILL BE REFUSED. `primary`'s fallback is
        // load-bearing — an unqualified undeclared name is a legitimate local,
        // field or `गणना` variant — so `चयनम्` parses, and `दोषः`'s real
        // parse fault (a stray `४` where `आदि` belongs) carries no note,
        // because no qualified name went unresolved.
        let text = "मण्डलम् परीक्षा ॥
             गणना रङ्गः आरभ्य रक्तः ऽ नीलः समाप्तम् ।
             सार्वजनिक वृत्तिः चयनम् ददाति अ६४ आदि
                 प्रत्यागमनम् नीलः ।
             इति
             सार्वजनिक वृत्तिः दोषः ददाति अ६४ आदि
                 यदि नीलः ४ आदि
                     प्रत्यागमनम् १ ।
                 इति
             इति";
        let it = Interpreter::load(&[("test", text)], Path::new(".")).expect("loads");
        assert!(it.routine("चयनम्").expect("declared").is_runnable());
        let why = why_not_of(text, "दोषः");
        assert!(
            why.contains("expected `आदि`"),
            "the real fault is reported: {why}"
        );
        assert!(
            !why.contains("qualified name"),
            "no qualified name went unresolved, so no note: {why}"
        );
    }

    #[test]
    fn a_member_mark_on_a_name_that_is_not_a_module_carries_no_such_note() {
        // THE CASE THAT MUST STILL BE REFUSED, in both spellings. `रङ्गः`
        // is a `गणना` TYPE qualifying its variant, not a module: nothing
        // imports it and nothing loads it, so a note saying "module `रङ्गः`
        // is not loaded" would be false. The rescue ref's rule recorded the
        // UNSPACED spelling on the member mark alone; here both spellings are
        // recorded only for a name the module imports or the load carries.
        for cond in ["रङ्गः ॱ नीलः", "रङ्गःॱनीलः"] {
            let why = why_not_of(
                &format!(
                    "मण्डलम् परीक्षा ॥
                     आयातः सङ्केतन ।
                     गणना रङ्गः आरभ्य रक्तः ऽ नीलः समाप्तम् ।
                     सार्वजनिक वृत्तिः दोषः ददाति अ६४ आदि
                         यदि {cond} ४ आदि
                             प्रत्यागमनम् १ ।
                         इति
                     इति"
                ),
                "दोषः",
            );
            assert!(
                !why.contains("qualified name"),
                "`{cond}` names no module, so no note: {why}"
            );
        }
    }

    #[test]
    fn a_loop_that_does_not_terminate_is_refused_rather_than_hung() {
        let e = run(
            "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः सदा ददाति अ६४ आदि
                 चरः क ॱॱ अ६४ भवति ० ।
                 यावत् क समम् ० आदि
                     क भवति ० ।
                 इति
                 प्रत्यागमनम् क ।
             इति",
            "सदा",
            Vec::new(),
        )
        .expect_err("must not hang");
        assert!(e.reason.contains("fuel"), "{}", e.reason);
    }

    #[test]
    fn an_embed_is_a_run_of_octets_and_this_is_where_that_is_asserted() {
        // `समावेशः` is resolved by `super::anita` before this module sees a
        // token, so the interpreter's whole part in it is that a `Kind::Str`
        // becomes octets. Checked against a table this test names itself.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec");
        let mut it = Interpreter::load(
            &[(
                "test",
                "मण्डलम् परीक्षा ॥
                 सार्वजनिक वृत्तिः कोशदैर्घ्यम् ददाति अ६४ आदि
                     चरः पाठ्यम् ॱॱ अङ्कः अन्तः अ८ भवति समावेशः आरभ्य कोष्ठकोशः समाप्तम् ।
                     प्रत्यागमनम् पाठ्यम् ॱ दैर्घ्य ।
                 इति",
            )],
            &root,
        )
        .expect("loads with the embed resolved");
        let want = std::fs::read(root.join("registers-riscv64.tsv"))
            .expect("spec/registers-riscv64.tsv exists")
            .len();
        assert_eq!(
            it.call("कोशदैर्घ्यम्", Vec::new(), 1000)
                .expect("runs")
                .as_int(),
            Some(i128::try_from(want).expect("a table is not that large")),
            "the embed must be the file's octets, exactly"
        );
    }

    // ── W-242: a module-level initialiser is evaluated at load ───────────

    fn bytes(v: Option<&Value>) -> Vec<u8> {
        v.and_then(Value::octets)
            .map(|o| o.as_slice().to_vec())
            .unwrap_or_else(|| panic!("a run of octets, not {v:?}"))
    }

    /// THE PROBE `W-239` FILED, AS IT WAS FOUND: a two-line module whose one
    /// global is a string literal. Before this row the loader initialised every
    /// global that was not an integer from its TYPE alone and never read the
    /// initialiser, so `क` was the EMPTY run — this test failed on 2026-09-04
    /// with `[]` where `अब` was written — while the same literal one line down,
    /// inside a routine, was the text. `इतिशब्दः` in `vakyavibhaga.t1` is that
    /// global, the closing word every string literal is matched against, and
    /// every literal read P22 "unclosed" for the whole life of the file.
    #[test]
    fn a_string_literal_on_a_slice_typed_global_is_the_text_and_not_the_empty_run() {
        let it = Interpreter::load(
            &[(
                "test",
                "मण्डलम् परीक्षा ॥\nसार्वजनिक चरः क ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् अब इति ।",
            )],
            Path::new("."),
        )
        .expect("loads");
        assert_eq!(bytes(it.global("क")), "अब".as_bytes().to_vec());
    }

    /// A numeral, a bool and a string each read back as the literal, and as
    /// the value a routine body gives the same literal — the two sites are one
    /// evaluator. `भवति ०` on an arena or a record stays the corpus's "a fresh
    /// empty one", which is what every arena in `vakyavibhaga.t1` is written with.
    #[test]
    fn a_global_initialised_with_a_numeral_a_bool_and_a_string_reads_back_as_the_literal() {
        let mut it = Interpreter::load(
            &[(
                "test",
                "मण्डलम् परीक्षा ॥
                 सार्वजनिक चरः संख्या ॱॱ अ६४ भवति ४२ ।
                 सार्वजनिक चरः ऋणसंख्या ॱॱ अ६४ भवति ऋण७ ।
                 सार्वजनिक चरः ध्वजः ॱॱ बूल भवति सत्यम् ।
                 सार्वजनिक चरः पाठः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् क ख इति ।
                 सार्वजनिक चरः रिक्तम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।
                 सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः अ६४ भवति ० ।
                 सार्वजनिक वृत्तिः स्थानीयपाठः ददाति अङ्कः अन्तः अ८ आदि
                     चरः स्थानीयः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् क ख इति ।
                     प्रत्यागमनम् स्थानीयः ।
                 इति
                 सार्वजनिक वृत्तिः स्थानीयध्वजः ददाति बूल आदि
                     चरः स्थानीयः ॱॱ बूल भवति सत्यम् ।
                     प्रत्यागमनम् स्थानीयः ।
                 इति",
            )],
            Path::new("."),
        )
        .expect("loads");
        assert_eq!(it.global("संख्या").and_then(Value::as_int), Some(42));
        assert_eq!(it.global("ऋणसंख्या").and_then(Value::as_int), Some(-7));
        assert_eq!(it.global("ध्वजः"), Some(&Value::Bool(true)));
        assert_eq!(bytes(it.global("पाठः")), "क ख".as_bytes().to_vec());
        assert_eq!(
            bytes(it.global("रिक्तम्")),
            Vec::<u8>::new(),
            "`भवति ०` is the empty run"
        );
        assert!(
            matches!(it.global("कोशः"), Some(Value::Arena(_))),
            "`भवति ०` on a slice of records is a fresh arena: {:?}",
            it.global("कोशः")
        );
        // The routine body's value for the same literal IS the global's.
        let in_body = it.call("स्थानीयपाठः", vec![], 1000).expect("runs");
        assert_eq!(Some(&in_body), it.global("पाठः"));
        let in_body = it.call("स्थानीयध्वजः", vec![], 1000).expect("runs");
        assert_eq!(Some(&in_body), it.global("ध्वजः"));
    }

    /// REFUSED, BY NAME: an initialiser that calls a routine, or reads a name,
    /// is not evaluated at load and is not silently ० either — the paradigm's
    /// S3, a marker resolves or is refused. The same call inside a routine body
    /// runs, so the refusal is the loader's and not the language's.
    #[test]
    fn an_initialiser_that_calls_a_routine_is_refused_by_name_and_not_zero() {
        let routine = "मण्डलम् परीक्षा ॥
             सार्वजनिक वृत्तिः गणना ददाति अ६४ आदि
                 प्रत्यागमनम् ७ ।
             इति
             सार्वजनिक वृत्तिः शरीरे ददाति अ६४ आदि
                 चरः क ॱॱ अ६४ भवति गणना ।
                 प्रत्यागमनम् क ।
             इति\n";
        let e = Interpreter::load(
            &[(
                "test",
                &format!("{routine}सार्वजनिक चरः क ॱॱ अ६४ भवति गणना ।"),
            )],
            Path::new("."),
        )
        .err()
        .expect("a call in a global's initialiser is refused at load");
        assert!(
            e.reason.contains("परीक्षाॱक") && e.reason.contains("calls `गणना`"),
            "names the global and the callee: {}",
            e.reason
        );
        let e = Interpreter::load(
            &[(
                "test",
                &format!("{routine}सार्वजनिक चरः ख ॱॱ अ६४ भवति अज्ञातम् ।"),
            )],
            Path::new("."),
        )
        .err()
        .expect("a name in a global's initialiser is refused at load");
        assert!(
            e.reason.contains("परीक्षाॱख") && e.reason.contains("reads `अज्ञातम्`"),
            "names the global and the name: {}",
            e.reason
        );
        // The language allows the call; the body proves it.
        let v = run(routine, "शरीरे", vec![]).expect("runs");
        assert_eq!(v.as_int(), Some(7));
    }
}

// THE ZERO-OPTIONAL WITNESS — every site where this engine and the compiled one
// must DISAGREE. `ir.t1` lowers `सम्भाव्य` as ONE WORD with ० meaning absent, so
// natively `Some(०)` and absence are the same bits; here they are `Value::Int(0)`
// and `Value::Nil`, which are distinct. So a comparison of Int(0) against Nil
// answers "different" (present) in this engine and "same" (absent) in the image:
// the divergence is not a risk at such a site, it is a certainty.
//
// This records the site rather than arguing about it. The census can say which
// of the 60 numeric sites COULD receive a ०; only a run says which DO. Note the
// arena convention that makes many of them safe without a repair: arenas are
// ONE-BASED and index ० is never live, so an arena index of ० genuinely means
// absent. Byte OFFSETS carry no such convention and ० is their first live value.
//
// Always on: it costs one `matches!` on the two equality operators.
thread_local! {
    static ZERO_OPT: RefCell<std::collections::HashMap<String, u64>> = RefCell::new(std::collections::HashMap::new());
}

/// The name an expression reads, for attributing a witness to a census line.
fn zero_opt_name(e: &Expr) -> Option<String> {
    match e {
        Expr::Var(n) => Some(n.clone()),
        Expr::Field(_, f) => Some(format!("ॱ {f}")),
        _ => None,
    }
}

fn note_zero_against_nil(module: &str, l: &Expr, r: &Expr, a: &Value, b: &Value) {
    let hit = match (a, b) {
        (Value::Int(0), Value::Nil) => zero_opt_name(l),
        (Value::Nil, Value::Int(0)) => zero_opt_name(r),
        _ => None,
    };
    if let Some(n) = hit {
        ZERO_OPT.with(|m| *m.borrow_mut().entry(format!("{module} {n}")).or_insert(0) += 1);
    }
}

/// Every `Some(०)`-against-absent comparison this thread has made, with its
/// count. Each entry is a site where the image MUST answer the other way.
pub fn zero_optional_witnesses() -> Vec<(String, u64)> {
    ZERO_OPT.with(|m| m.borrow().iter().map(|(k, v)| (k.clone(), *v)).collect())
}

// The per-routine invocation counter (see the module header).
static CALLS_ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
thread_local! {
    static CALLS: RefCell<std::collections::HashMap<String, u64>> = RefCell::new(std::collections::HashMap::new());
}
/// Every routine invoked so far on this thread with its count; empty unless `T1_CALLS` is set.
pub fn call_counts() -> Vec<(String, u64)> {
    CALLS.with(|c| c.borrow().iter().map(|(k, v)| (k.clone(), *v)).collect())
}
thread_local! {
    static NAMES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static STACK: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static STEPS: RefCell<std::collections::HashMap<String, (u64, u64)>> = RefCell::new(std::collections::HashMap::new());
}
/// Every routine's steps so far on this thread as `(inclusive, self)`; empty unless `T1_CALLS` is set.
pub fn step_counts() -> Vec<(String, u64, u64)> {
    STEPS.with(|m| {
        m.borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.0, v.1))
            .collect()
    })
}

// THE BUMP MODEL — what the NATIVE record region would have allocated for the
// growth the interpreter just did silently. ir.t1's growth arm: a nil-declared
// run gets a 128-element block; a store past capacity allocates need + 2×old
// and leaks the old block. Keyed by the buffer's address (stable across
// in-place growth, recycled across drops — an approximation stated as one).
// Switched on by T1_BUMP; read by `bump_total` and `bump_largest`.
//
// WHAT IT MEASURED, 2026-09-13. ashtaka.t1 alone: 56.9 MB, two runs at 12.7 MB —
// the assembler pushing the record region's own `॥ स्थानम् ४१९४३०४ ॥` as zero
// octets, natively a run inside the region it reserves (fixed, e9c3e8ed: 5.07
// MB after). The 20-source self-compile after that fix: 254,132,032 octets —
// 4,429 growths over 531,818 runs, the largest 8.47 MB of emitted text and the
// bulk one-text-per-token runs each paying its first block. So a full native
// self-compile needs a region of 256 MiB or more and a smaller first block for
// octet runs; the 234-octet स्वपरीक्षा rung needs no growth at all.
static BUMP_ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
thread_local! {
    static BUMP: RefCell<(u64, u64, std::collections::HashMap<usize, u64>)> =
        RefCell::new((0, 0, std::collections::HashMap::new()));
    // THE TWO THINGS THE FIRST MODEL LEFT OUT (2026-09-14, the fixpoint's
    // BeyondRam at 2 GB against a 254 MB prediction): `(first-block octets as
    // the model books them, records allocated, record octets)`. The model books
    // a first block as 128 ELEMENTS — 128 octets for an octet run — while the
    // native region takes खण्डसामर्थ्यम् = १२८ WORDS for every run; and it never
    // counted RECORDS, which the native AllocRecord arm bumps from the same
    // region, one word per field, never freed. `bump_native_view` re-prices
    // both so the model predicts the native high water again.
    static BUMP2: RefCell<(u64, u64, u64)> = const { RefCell::new((0, 0, 0)) };
}
fn bump_on() -> bool {
    *BUMP_ON.get_or_init(|| std::env::var_os("T1_BUMP").is_some())
}
/// A record allocated: natively `AllocRecord(size)` with one word per field.
fn bump_record(fields: usize) {
    if !bump_on() {
        return;
    }
    BUMP2.with(|b| {
        let mut b = b.borrow_mut();
        b.1 += 1;
        b.2 += 8 * fields as u64;
    });
}
fn bump_model(key: usize, old_len: usize, need: usize, elem: usize) {
    if !bump_on() {
        return;
    }
    let need = (need * elem) as u64;
    BUMP.with(|b| {
        let mut b = b.borrow_mut();
        let _ = old_len;
        let first = 128 * elem as u64;
        let cap = match b.2.get(&key) {
            Some(c) => *c,
            None => {
                b.0 += first; // the first block
                BUMP2.with(|b2| b2.borrow_mut().0 += first);
                b.2.insert(key, first);
                first
            }
        };
        if need > cap {
            let new_cap = need + 2 * cap;
            b.0 += new_cap;
            b.1 += 1;
            b.2.insert(key, new_cap);
            if new_cap >= 1 << 20 {
                let names =
                    NAMES.with(|n| n.borrow().iter().rev().take(4).cloned().collect::<Vec<_>>());
                eprintln!(
                    "bump:       growth to {new_cap} octets (need {need}) in {}",
                    names.join(" <- ")
                );
            }
        }
    });
}
/// The model with RECORDS priced in: `(first-block octets, records allocated,
/// record octets, total = runs + records)` — the total is what the native high
/// water should read once the eager declaration allocation is gone (2026-09-14:
/// natively a `चरः x ॱॱ अङ्कः अन्तः T भवति ०` bumped 1,040 octets on EVERY
/// execution, stored into or not — the builder's lowering makes the declaration
/// the nil word and the first growth 128 × width, which is this model's rule).
/// Empty unless `T1_BUMP` is set.
pub fn bump_native_view() -> (u64, u64, u64, u64) {
    let (total, _growths, _runs) = bump_total();
    let (first_model, records, record_octets) = BUMP2.with(|b| *b.borrow());
    (first_model, records, record_octets, total + record_octets)
}
/// `(octets the native region would hold, growth events, runs seen)` — empty unless `T1_BUMP` is set.
pub fn bump_total() -> (u64, u64, usize) {
    BUMP.with(|b| {
        let b = b.borrow();
        (b.0, b.1, b.2.len())
    })
}
/// The largest runs by final capacity, `(capacity octets, buffer key)`, largest first.
pub fn bump_largest(n: usize) -> Vec<(u64, usize)> {
    BUMP.with(|b| {
        let b = b.borrow();
        let mut v: Vec<(u64, usize)> = b.2.iter().map(|(k, c)| (*c, *k)).collect();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v.truncate(n);
        v
    })
}
