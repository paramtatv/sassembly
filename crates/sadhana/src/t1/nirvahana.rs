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
//! `बृहत्समम्` 2. `spec/lexicon.tsv` proposes a DIFFERENT set for the same jobs
//! — `अधि` for plus (line 24), `ऊन` for minus (86), `विषमम्` for not-equal
//! (340), `अधिकसमम्` for greater-or-equal (26) — and **not one of those four is
//! used as an operator in any `.t1` body.** `ऊन`, `विषमम्` and `अधिकसमम्` do not
//! occur at all; `अधि` occurs twice, `crates/sadhana-t1/src/utsarjana.t1:624`
//! and `:625`, where it is a LOCAL VARIABLE — a lexicon operator bound as a
//! name, which is the shape `B-112` fixed for `समावेशः`. Reconciling the two
//! sets is `D-002h`'s remaining half and is not done here.
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
//! * **Fixed-width arithmetic.** Values are [`i128`]; `अ८` and `अ६४` do not
//!   wrap or trap. Every routine this module was written to run keeps its
//!   arithmetic inside the range by its own guards, so nothing in the corpus
//!   distinguishes the two yet — but a program that relies on wrapping would
//!   read differently here, and that is a limitation and not a decision.
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
    /// `अङ्कः अन्तः <struct>` — a ONE-BASED growable arena. Index ० is never a
    /// live entry, which is the convention every `…योजनम्` in `vakyavibhaga.t1`
    /// states in its own margin: "advance the index, write at it, return it —
    /// so the number returned is a live entry and ० never is".
    Arena(Rc<RefCell<Vec<Value>>>),
    /// `शून्यम्` — the empty `सम्भाव्य`.
    Nil,
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
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Bool(b)) | (Self::Bool(b), Self::Int(a)) => *a == i128::from(*b),
            (Self::Octets(a), Self::Octets(b)) => a.as_slice() == b.as_slice(),
            (Self::Nil, Self::Nil) => true,
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
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Gt,
    Ge,
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
}

impl Routine {
    /// How many arguments the declaration takes.
    #[must_use]
    pub fn arity(&self) -> usize {
        self.params.len()
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
}

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
            Self::Div => "/",
            Self::Rem => "%",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::And => "&",
            Self::Or => "|",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Ge => ">=",
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
        Expr::Call { module, name, args } => {
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

        let mut out = Vec::new();
        // Every `गणना` variant in every module, flat. Variants share one
        // namespace with globals, which is what the corpus already assumes:
        // `वस्तुसंज्ञा ॱ स्थापनम्` is compared against a bare `पाठ्यम्`.
        let mut enum_variants: Vec<(String, i128)> = Vec::new();
        for (label, head) in heads {
            let mut routines = Vec::new();
            for f in head.fns {
                let ctx = ParseCtx {
                    sigs: &sigs,
                    modules: &modules,
                    module: &head.module,
                };
                let mut locals: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
                let body = Parser::new(&f.body, ctx)
                    .block(&mut locals)
                    .map_err(|e| format!("{label}:{}: `{}`: {}", f.line, f.name, e.reason));
                routines.push(Rc::new(Routine {
                    name: f.name,
                    module: head.module.clone(),
                    line: f.line,
                    params: f.params,
                    returns: f.returns,
                    public: f.public,
                    body,
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
        let out = merged;

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
        };
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
        for ((n, _), v) in routine.params.iter().zip(args) {
            scope[0].push((n.clone(), v));
        }
        let module = self.module_index(&routine.module);
        match self.block(body, &mut scope, module) {
            Ok(Flow::Return(v)) => Ok(v),
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

    fn module_index(&self, name: &str) -> usize {
        self.by_module.get(name).copied().unwrap_or(0)
    }

    /// The value a `भवति ०` initialiser means for a declared type.
    ///
    /// The corpus has no empty-collection literal and no struct literal, so `०`
    /// is what every arena and every record is written with — `सार्वजनिक चरः
    /// पाठांशकोश ॱॱ अङ्कः अन्तः पाठांश भवति ०` and `चरः नव ॱॱ पाठांश भवति ०`.
    /// The TYPE is what says which of the three it means.
    fn zero_of(&self, ty: &Ty, module: usize) -> Value {
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
            return Ok(self.zero_of(ty, module));
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
        self.eval(init, &mut empty, module).map_err(|e| {
            RunError::new(format!(
                "`{label}`: its initialiser could not be evaluated at load: {}",
                e.reason
            ))
        })
    }

    fn zero_at(&self, ty: &Ty, module: usize, depth: usize) -> Value {
        if depth > 8 {
            // A struct that contains itself is not writable in this language
            // and would be an infinite record here. Stop rather than hang.
            return Value::Nil;
        }
        match ty {
            Ty::Slice(inner) => match &**inner {
                Ty::Named(n) if n == T_U8 => Value::Octets(Octets::new(&[])),
                // One-based: slot ० is reserved and never a live entry, which
                // is the convention every `…योजनम्` states in its own margin.
                _ => Value::Arena(Rc::new(RefCell::new(vec![Value::Nil]))),
            },
            Ty::Named(n) => match self.struct_fields(n, module) {
                Some(fields) => {
                    let mut map = HashMap::new();
                    for (f, fty) in fields {
                        map.insert(f.clone(), self.zero_at(fty, module, depth + 1));
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
                None => Value::Int(0),
            },
            Ty::Optional(_) => Value::Nil,
            _ => Value::Int(0),
        }
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
type Frame = Vec<(String, Value)>;

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
                        let z = self.zero_of(ty, module);
                        if matches!(z, Value::Int(_)) {
                            self.eval(init, scope, module)?
                        } else {
                            z
                        }
                    }
                    _ => self.eval(init, scope, module)?,
                };
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
                        *slot = v;
                        return Ok(());
                    }
                }
                let key = self.global_key(name).to_string();
                if let Some(slot) = self.globals.get_mut(&key) {
                    *slot = v;
                    return Ok(());
                }
                Err(RunError::new(format!("`{name}` is not a name in scope")))
            }
            Expr::Field(base, field) => match self.eval(base, scope, module)? {
                Value::Record(r) => {
                    r.borrow_mut().insert(field.clone(), v);
                    Ok(())
                }
                other => Err(RunError::new(format!(
                    "`\u{971} {field}` written on {other:?}, which is not a record"
                ))),
            },
            Expr::Index(base, idx) => {
                let i = self.eval(idx, scope, module)?.int()?;
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
                    let i = usize::try_from(i)
                        .map_err(|_| RunError::new(format!("octet index {i} is not an index")))?;
                    let byte = u8::try_from(*byte).map_err(|_| {
                        RunError::new(format!(
                            "{byte} does not fit an octet, so it cannot be \
                             written into a `अङ्कः अन्तः अ८`"
                        ))
                    })?;
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
                            RunError::new(format!("arena index {i} is not an index"))
                        })?;
                        let mut a = a.borrow_mut();
                        if a.len() <= i {
                            bump_model(a.as_ptr() as usize, a.len(), i + 1, 8);
                            a.resize(i + 1, Value::Nil);
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
                            RunError::new(format!("octet index {i} is not an index"))
                        })?;
                        let mut bytes = o.as_slice().to_vec();
                        if bytes.len() <= i {
                            bytes.resize(i + 1, 0);
                        }
                        let byte = v.int()?;
                        bytes[i] = u8::try_from(byte).map_err(|_| {
                            RunError::new(format!(
                                "{byte} does not fit an octet, so it cannot be \
                                 written into a `अङ्कः अन्तः अ८`"
                            ))
                        })?;
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
            Expr::Num(n) => Ok(Value::Int(*n)),
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
            } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a, scope, module)?);
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
                if m.as_deref() == Some("अष्टक") && name == "मुद्रणम्" {
                    if let Some(Value::Int(v)) = vals.first() {
                        self.sink.push((*v & 0xff) as u8);
                    }
                    return Ok(Value::Int(0));
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
        Expr::Call { module, name, args } => {
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

fn binop(op: BinOp, a: &Value, b: &Value) -> Result<Value, RunError> {
    match op {
        BinOp::Eq => return Ok(Value::Bool(a == b)),
        BinOp::Ne => return Ok(Value::Bool(a != b)),
        _ => {}
    }
    let (x, y) = (a.int()?, b.int()?);
    Ok(match op {
        BinOp::Add => Value::Int(x + y),
        BinOp::Sub => Value::Int(x - y),
        BinOp::Mul => Value::Int(x * y),
        BinOp::Div => {
            if y == 0 {
                return Err(RunError::new("division by ०"));
            }
            Value::Int(x / y)
        }
        BinOp::Rem => {
            if y == 0 {
                return Err(RunError::new("remainder by ०"));
            }
            Value::Int(x % y)
        }
        BinOp::Shl => Value::Int(
            x.checked_shl(u32::try_from(y).unwrap_or(u32::MAX))
                .ok_or_else(|| RunError::new(format!("`वामसृ {y}` is not a shift")))?,
        ),
        BinOp::Shr => Value::Int(
            x.checked_shr(u32::try_from(y).unwrap_or(u32::MAX))
                .ok_or_else(|| RunError::new(format!("`दक्षिणसृ {y}` is not a shift")))?,
        ),
        BinOp::And => Value::Int(x & y),
        BinOp::Or => Value::Int(x | y),
        BinOp::Lt => Value::Bool(x < y),
        BinOp::Gt => Value::Bool(x > y),
        BinOp::Ge => Value::Bool(x >= y),
        BinOp::Eq | BinOp::Ne => unreachable!("handled above"),
    })
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
}

struct Parser<'a> {
    t: &'a [Token],
    at: usize,
    ctx: ParseCtx<'a>,
}

impl<'a> Parser<'a> {
    fn new(t: &'a [Token], ctx: ParseCtx<'a>) -> Self {
        Self { t, at: 0, ctx }
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
        while self.text(0) == "विकल्प" {
            self.at += 1;
            let r = self.bitand(locals)?;
            l = Expr::Bin(BinOp::Or, Box::new(l), Box::new(r));
        }
        Ok(l)
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
            let bytes = Rc::new(value.as_bytes().to_vec());
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
            });
        }

        // A name in scope is a variable; otherwise a routine of this name is a
        // call. A local shadows a routine, which is what a reader of the corpus
        // expects: `चरः पाठ्यम्` is the text, never a routine.
        if locals.contains(&name) {
            return Ok(Expr::Var(name));
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
                });
            }
            let args = self.arguments(&name, arity, locals)?;
            return Ok(Expr::Call {
                module: None,
                name,
                args,
            });
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
