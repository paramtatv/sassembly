//! **W-209 — JUNCTION STATISTICS.** The secondary conduit (resolution: a use →
//! its declaration) and the ह junction (the module boundary) MEASURED over the
//! fifteen T1 sources after `अर्थॱकार्यक्रमनिर्णयः` has run. Statistics 7–10, 20
//! and 21 of `paramtatva-stats-plan.md` §2.3/§2.6; the numbers feed §6.4's
//! decision table (research/22-two-conduits.md §3.3, rule S3).
//!
//! # The instrument, and the two things it had to learn before it could count
//!
//! The pointer being measured is W-183's `संज्ञासूचकाङ्क`, written onto every
//! Identifier node the resolver reaches (`SymbolId + 1`, ० = unresolved). Reading
//! it off the arena is not enough, for two reasons found by reading the parser
//! rather than assumed:
//!
//! 1. **A primitive TYPE node is shaped exactly like an Identifier node.**
//!    `व्याकरॱप्रकारपठनम्` stores every type with `अभिव्यञ्जकयोजनम्`, and
//!    `मूलप्रकारभेद` is १ — the same number as `नामाभिव्यञ्जकभेद`. A census that
//!    counted kind-१ nodes would count `अ६४` in `ददाति अ६४` as an unresolved
//!    name. So nodes are classified by REACHABILITY: from a routine body along
//!    the resolver's own walk (a use), from a `प्रकारसूचकाङ्क` (a type), from a
//!    module-level `चरः`'s initializer, or from nothing at all.
//! 2. **Which nodes the resolver reaches is decided by the parser, not the
//!    resolver.** `वाक्यपठनम्`'s `यदि` arm parses an `अन्यथा` body into
//!    `अन्यशरीर` and then pushes `वाक्ययोजनम् यदिवाक्यभेद शर्त शरीर` — the else
//!    body's statements sit in `वाक्यकोश`, inside the parent block's range, and
//!    NOTHING points at them. The resolver's `यदि`/`यावत्` arm DOES descend into
//!    the then-body (`प्रत्यागमनम् वाक्यनिर्णयः निर्णायकः वाक्यम् ॱ दक्षिणसूचकाङ्क`),
//!    so W-202's "never enters conditional bodies" is measured here as: then- and
//!    while-bodies entered, else-bodies orphaned. The per-use rate is therefore a
//!    LOWER BOUND on what a complete resolver would answer, and the orphans are
//!    counted separately and by cause.
//!
//! A THIRD FINDING shaped the qualified split: `अभिव्यञ्जकनिर्णयः` resolves a
//! `मण्डलॱनाम` by looking up the MODULE half as an import and returns `सत्यम्`
//! WITHOUT writing `संज्ञासूचकाङ्क`. So every qualified use reads ० on the node
//! while having been checked. They are counted as uses of the IMPORT symbol, and
//! `paradigm_name_uses_qualified_with_symbol_written` is expected ० — if it moves,
//! the resolver changed and this reading must change with it.
//!
//! # How a use is tied to its declaration
//!
//! The resolver pops every scope it opens, so after `कार्यक्रमनिर्णयः` only the
//! global scope survives and a `SymbolId` names nothing on its own. This file
//! REPLICATES the resolver's declaration order — pass one over `घोषणाकोश`, then
//! per routine: parameters, then the body in written order, `चर` after its value,
//! `यदि`/`यावत्` condition then body, a duplicate in the innermost scope consuming
//! no symbol (`घोषणम्` returns `शून्यम्` and its callers discard it) — and CHECKS
//! ITSELF against the node: every bare use's `संज्ञासूचकाङ्क` must equal the
//! replica's symbol + १. `paradigm_name_replica_disagreements` is that check; the
//! census REFUSES (panics) if it is not ० in a file the resolver accepted, because
//! every distance and in-degree below would then be about the wrong symbols.

use sadhana::t1::nirvahana::{
    Declaration, DeclarationKind, Interpreter, ModuleDeclarations, Octets, Value,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

// ── kinds, read off ast.t1 and parse.t1 ─────────────────────────────────
const EXPR_NAME: i128 = 1; // नामाभिव्यञ्जकभेद — and मूलप्रकारभेद in a type position
const EXPR_NUMERAL: i128 = 2;
const EXPR_STRING: i128 = 3;
const EXPR_BOOL: i128 = 9;
const EXPR_VOID: i128 = 10;
const EXPR_EMBED: i128 = 11;
const EXPR_NEG: i128 = 12;

const STMT_EXPR: i128 = 1;
const STMT_BLOCK: i128 = 2;
const STMT_LET: i128 = 3;
const STMT_RETURN: i128 = 4;
const STMT_IMPORT: i128 = 5;
const STMT_IF: i128 = 6;
const STMT_WHILE: i128 = 7;
const STMT_ASSIGN: i128 = 8;

const DECL_ROUTINE: i128 = 1;
const DECL_TYPE: i128 = 2;
const DECL_MACHINE: i128 = 3;
const DECL_GLOBAL: i128 = 4;
const DECL_IMPORT: i128 = 5;
/// `गणनाघोषणाभेद` — an enum, its own kind since `W-215`; a TYPE to this walk,
/// as it was when it shared the struct's kind.
const DECL_ENUM: i128 = 6;

const FUEL_LEX: u64 = 2_000_000_000;
const FUEL_PARSE: u64 = 4_000_000_000;
const FUEL_RESOLVE: u64 = 4_000_000_000;

// ── loading, copied from t1_execution.rs's convention ───────────────────

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// The chain `measure_corpus_resolve` loads: lexer, AST, parser, resolver, and
/// the numeral reader the parser asks. Not shared with that file on purpose —
/// its loaders carry counts other censuses are keyed to.
fn load_chain() -> Interpreter {
    let names = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "artha.t1",
        "sanchaya.t1",
        "sanskrit_text.t1",
    ];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &repo_root().join("spec"))
        .unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

type Rec = HashMap<String, Value>;

fn int(r: &Rec, k: &str) -> i128 {
    r.get(k).and_then(Value::as_int).unwrap_or(0)
}

fn idx(r: &Rec, k: &str) -> usize {
    usize::try_from(int(r, k)).unwrap_or(0)
}

fn truth(r: &Rec, k: &str) -> bool {
    matches!(r.get(k), Some(Value::Bool(true)))
}

/// Every slot of a `सार्वजनिक चरः … अङ्कः अन्तः <record>` global, `None` where
/// the slot is not a record (slot ०, always).
fn arena_of(it: &Interpreter, global: &str) -> Vec<Option<Rec>> {
    match it.global(global) {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .map(|v| match v {
                Value::Record(r) => Some(r.borrow().clone()),
                _ => None,
            })
            .collect(),
        other => panic!("global `{global}` is {other:?}, not an arena — THIS CENSUS is broken"),
    }
}

/// Every slot of a `सार्वजनिक चरः … अङ्कः अन्तः न६४` global, ० where the slot
/// holds no integer (slot ०, always).
fn int_arena_of(it: &Interpreter, global: &str) -> Vec<i128> {
    match it.global(global) {
        Some(Value::Arena(a)) => a.borrow().iter().map(|v| v.as_int().unwrap_or(0)).collect(),
        other => panic!("global `{global}` is {other:?}, not an arena — THIS CENSUS is broken"),
    }
}

// ── the program as the T1 chain left it ─────────────────────────────────

#[derive(Debug, Clone)]
struct Tok {
    text: String,
    line: i128,
}

#[derive(Debug, Clone, Copy, Default)]
struct Expr {
    kind: i128,
    value: usize,
    left: usize,
    right: usize,
    symbol: i128,
}

#[derive(Debug, Clone, Copy, Default)]
struct Stmt {
    kind: i128,
    left: usize,
    right: usize,
    /// The `अन्यथा` body of a `यदि`, ० when there is none — `W-198` gave
    /// `वास्तुॱवाक्य` the slot and `W-202` made the resolver read it. The
    /// replica reads it because this file's contract is that the walk AGREES
    /// with the resolver: a branch the resolver descends and the replica does
    /// not is a disagreement, not a saving.
    alt: usize,
    start: usize,
    ty: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct Decl {
    kind: i128,
    name: usize,
    ty: usize,
    body: usize,
    p_first: usize,
    p_last: usize,
    public: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Param {
    name: usize,
    ty: usize,
}

struct Program {
    file: String,
    module: Option<String>,
    toks: Vec<Option<Tok>>,
    exprs: Vec<Option<Expr>>,
    stmts: Vec<Option<Stmt>>,
    decls: Vec<Option<Decl>>,
    params: Vec<Option<Param>>,
    parse_errors: i128,
    resolved: bool,
    refusal: Option<String>,
    /// `अर्थॱसंज्ञाभेदकोश`, the kind of every symbol by SymbolId+1 — the
    /// `W-228` amendment: १ routine … ५ import, ६ parameter, ७ local.
    kinds: Vec<i128>,
}

impl Program {
    fn tok(&self, i: usize) -> Option<&Tok> {
        self.toks.get(i).and_then(Option::as_ref)
    }
    fn tok_text(&self, i: usize) -> String {
        self.tok(i).map(|t| t.text.clone()).unwrap_or_default()
    }
    fn tok_line(&self, i: usize) -> i128 {
        self.tok(i).map_or(0, |t| t.line)
    }
    fn expr(&self, i: usize) -> Option<Expr> {
        self.exprs.get(i).copied().flatten()
    }
    fn stmt(&self, i: usize) -> Option<Stmt> {
        self.stmts.get(i).copied().flatten()
    }
    fn param(&self, i: usize) -> Option<Param> {
        self.params.get(i).copied().flatten()
    }
    /// The public declarations that are exports: routines, types, machines,
    /// module-level bindings. An import is a declaration but not an export.
    fn exports(&self) -> Vec<(String, i128)> {
        self.decls
            .iter()
            .flatten()
            .filter(|d| d.public && d.kind != DECL_IMPORT)
            .map(|d| (self.tok_text(d.name), d.kind))
            .collect()
    }
    /// Top-level `आयातः` declarations, by module name.
    fn imports(&self) -> Vec<String> {
        self.decls
            .iter()
            .flatten()
            .filter(|d| d.kind == DECL_IMPORT)
            .map(|d| self.tok_text(d.name))
            .collect()
    }
}

/// Lex, parse and resolve one source through the T1 chain and read back every
/// arena the three passes filled.
fn read_program(file: &str, src: &str) -> Program {
    let mut it = load_chain();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], FUEL_LEX)
        .unwrap_or_else(|e| panic!("{file}: पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], FUEL_PARSE)
        .unwrap_or_else(|e| panic!("{file}: कार्यक्रमपठनम् runs: {e:?}"));
    // Three states, not two: a missing global means THIS census is broken.
    let parse_errors = it
        .global("दोषसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the global दोषसूचकाङ्क exists — else THIS CENSUS is broken");
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .unwrap_or_else(|e| panic!("{file}: निर्णायकारम्भः runs: {e:?}"));
    let verdict = it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], FUEL_RESOLVE);
    let resolved = matches!(verdict, Ok(Value::Bool(true)));
    let refusal = if resolved {
        None
    } else {
        Some(match (&verdict, it.global("अनिर्णीतमस्ति")) {
            (Err(e), _) => format!("RUN ERROR {:.80}", format!("{e:?}")),
            (Ok(_), Some(Value::Bool(true))) => match it.global("अनिर्णीतनाम")
            {
                Some(Value::Octets(o)) => format!(
                    "undeclared `{}` at line {}",
                    String::from_utf8_lossy(o.as_slice()),
                    it.global("अनिर्णीतपङ्क्ति")
                        .and_then(Value::as_int)
                        .unwrap_or(0)
                ),
                other => format!("<अनिर्णीतनाम is {other:?}>"),
            },
            (Ok(v), _) => format!("refused with no name recorded ({v:?})"),
        })
    };

    let toks: Vec<Option<Tok>> = arena_of(&it, "चिह्नककोश")
        .into_iter()
        .map(|r| {
            r.map(|r| {
                let text = match r.get("पाठ") {
                    Some(Value::Octets(o)) => {
                        let (a, b) = (idx(&r, "अष्टक"), idx(&r, "पाठसीमा"));
                        let s = o.as_slice();
                        if a <= b && b <= s.len() {
                            String::from_utf8_lossy(&s[a..b]).into_owned()
                        } else {
                            String::new()
                        }
                    }
                    _ => String::new(),
                };
                Tok {
                    text,
                    line: int(&r, "पङ्क्ति"),
                }
            })
        })
        .collect();
    let exprs = arena_of(&it, "अभिव्यञ्जककोश")
        .into_iter()
        .map(|r| {
            r.map(|r| Expr {
                kind: int(&r, "भेद"),
                value: idx(&r, "मूल्यसूचकाङ्क"),
                left: idx(&r, "वामसूचकाङ्क"),
                right: idx(&r, "दक्षिणसूचकाङ्क"),
                symbol: int(&r, "संज्ञासूचकाङ्क"),
            })
        })
        .collect();
    let stmts = arena_of(&it, "वाक्यकोश")
        .into_iter()
        .map(|r| {
            r.map(|r| Stmt {
                kind: int(&r, "भेद"),
                left: idx(&r, "वामसूचकाङ्क"),
                right: idx(&r, "दक्षिणसूचकाङ्क"),
                alt: idx(&r, "अन्यसूचकाङ्क"),
                start: idx(&r, "आदिसूचकाङ्क"),
                ty: idx(&r, "प्रकारसूचकाङ्क"),
            })
        })
        .collect();
    let decls = arena_of(&it, "घोषणाकोश")
        .into_iter()
        .map(|r| {
            r.map(|r| Decl {
                kind: int(&r, "भेद"),
                name: idx(&r, "नामसूचकाङ्क"),
                ty: idx(&r, "प्रकारसूचकाङ्क"),
                body: idx(&r, "शरीरसूचकाङ्क"),
                p_first: idx(&r, "प्राचलादि"),
                p_last: idx(&r, "प्राचलान्त"),
                public: truth(&r, "सार्वजनिकत्व"),
            })
        })
        .collect();
    let params = arena_of(&it, "प्राचलकोश")
        .into_iter()
        .map(|r| {
            r.map(|r| Param {
                name: idx(&r, "नामसूचकाङ्क"),
                ty: idx(&r, "प्रकारसूचकाङ्क"),
            })
        })
        .collect();
    let module = match (toks.get(1), toks.get(2)) {
        (Some(Some(a)), Some(Some(b))) if a.text == "मण्डलम्" => Some(b.text.clone()),
        _ => None,
    };
    let kinds = int_arena_of(&it, "संज्ञाभेदकोश");
    Program {
        file: file.to_string(),
        module,
        toks,
        exprs,
        stmts,
        decls,
        params,
        parse_errors,
        resolved,
        refusal,
        kinds,
    }
}

// ── W-228 amendment: a zero-argument call is a name, and the name knows ──

/// `व्याकर` pushes one call node per argument, so `आरम्भः ।` — a
/// zero-argument call — is a bare `नामाभिव्यञ्जक`, indistinguishable from a
/// variable read by shape. The resolver now records every symbol's KIND
/// (`अर्थॱसंज्ञाभेदकोश`, by SymbolId+1) beside its declared type, so the node's
/// symbol says what it names: a routine kind in value position IS a call;
/// a local or a parameter is a read. The IR reads the same arena (W-204's
/// lane). REFUSED: a bare name resolving to nothing is still the resolver's
/// refusal, by name and line.
#[test]
fn a_routine_name_in_value_position_carries_the_routine_kind_and_a_local_does_not() {
    let src = "मण्डलम् क ॥\n\
               सार्वजनिक वृत्तिः आरम्भः ददाति अ६४ आदि\n\
               \x20   प्रत्यागमनम् १ ।\n\
               इति\n\
               सार्वजनिक वृत्तिः ख आदाय प ॱॱ अ६४ ददाति अ६४ आदि\n\
               \x20   चरः ग ॱॱ अ६४ भवति आरम्भः ।\n\
               \x20   प्रत्यागमनम् ग योगः प ।\n\
               इति\n";
    let prog = read_program("kinds", src);
    assert_eq!(prog.parse_errors, 0, "{:?}", prog.refusal);
    assert!(prog.resolved, "{:?}", prog.refusal);
    let node = |name: &str| -> &Expr {
        prog.exprs
            .iter()
            .flatten()
            .find(|e| e.kind == EXPR_NAME && prog.tok_text(e.value) == name)
            .unwrap_or_else(|| panic!("a name node for `{name}`"))
    };
    let kind_of = |e: &Expr| -> i128 {
        assert!(e.symbol > 0, "the resolver wrote a symbol onto the node");
        prog.kinds
            .get(usize::try_from(e.symbol).expect("an index"))
            .copied()
            .unwrap_or(-1)
    };
    assert_eq!(
        kind_of(node("आरम्भः")),
        1,
        "`आरम्भः` in value position names a ROUTINE: a zero-argument call"
    );
    assert_eq!(kind_of(node("ग")), 7, "`ग` names a local: a read");
    assert_eq!(kind_of(node("प")), 6, "`प` names a parameter: a read");

    // REFUSED: a bare name resolving to nothing.
    let p = read_program(
        "refused",
        "मण्डलम् क ॥\n\
         सार्वजनिक वृत्तिः ख ददाति अ६४ आदि\n\
         \x20   प्रत्यागमनम् घ ।\n\
         इति\n",
    );
    assert!(!p.resolved);
    assert_eq!(p.refusal.as_deref(), Some("undeclared `घ` at line 3"));
}

// ── the replica walk ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    Routine,
    Type,
    Machine,
    Global,
    Import,
    Param,
    Local,
}

impl Kind {
    fn of_decl(kind: i128) -> Self {
        match kind {
            DECL_ROUTINE => Self::Routine,
            DECL_TYPE => Self::Type,
            DECL_MACHINE => Self::Machine,
            DECL_GLOBAL => Self::Global,
            DECL_IMPORT => Self::Import,
            DECL_ENUM => Self::Type,
            other => panic!("घोषणा kind {other} is none of the six parse.t1 declares"),
        }
    }
    fn is_global(self) -> bool {
        !matches!(self, Self::Param | Self::Local)
    }
}

#[derive(Debug, Clone)]
struct Sym {
    name: String,
    kind: Kind,
    line: i128,
    /// Live scopes when declared: global १, routine २, body block ३, …
    depth: usize,
    /// The `घोषणाकोश` slot for a global kind; ० otherwise.
    decl: usize,
    /// For a local: the block statement declaring it and its position among
    /// that block's direct children.
    block: usize,
    pos: usize,
    public: bool,
    uses: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Site {
    Plain,
    Condition,
    IfBody,
    WhileBody,
}

#[derive(Debug, Clone)]
struct Use {
    name: String,
    node_symbol: i128,
    replica: Option<usize>,
    qualified: bool,
    depth: usize,
    site: Site,
    cond_depth: usize,
    /// The `घोषणाकोश` slot of the enclosing routine.
    routine: usize,
    line: i128,
    /// `(block statement, position among its direct children)` from the body
    /// block inward.
    chain: Vec<(usize, usize)>,
}

#[derive(Clone)]
struct Ctx {
    depth: usize,
    site: Site,
    cond_depth: usize,
    routine: usize,
    chain: Vec<(usize, usize)>,
}

#[derive(Default)]
struct Walk {
    syms: Vec<Sym>,
    uses: Vec<Use>,
    scopes: Vec<Vec<(String, usize)>>,
    reached_exprs: BTreeSet<usize>,
    reached_stmts: BTreeSet<usize>,
    /// Qualified names in TYPE positions (`वास्तुॱवाक्य`), and the bare count.
    type_qualified: Vec<String>,
    /// The line of each entry of `type_qualified` (`W-223`'s census names sites).
    type_qualified_lines: Vec<i128>,
    type_bare: usize,
}

/// The module half of `मण्डलॱनाम`, as `पदविभागॱमण्डलउपसर्गः` cuts it: an
/// INTERIOR mark only; a leading or trailing one is a sign the lexer peels.
fn module_prefix(name: &str) -> Option<(&str, &str)> {
    let (head, tail) = name.split_once('ॱ')?;
    if head.is_empty() || tail.is_empty() {
        return None;
    }
    Some((head, tail))
}

impl Walk {
    /// `घोषणम्`: bind in the innermost scope; a duplicate there binds nothing
    /// and consumes no symbol.
    fn declare(&mut self, sym: Sym) -> Option<usize> {
        let inner = self.scopes.last_mut().expect("a scope is open");
        if inner.iter().any(|(n, _)| *n == sym.name) {
            return None;
        }
        let id = self.syms.len();
        inner.push((sym.name.clone(), id));
        self.syms.push(sym);
        Some(id)
    }

    /// `नामनिर्णयः`: innermost scope outward.
    fn lookup(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rev()
            .find_map(|s| s.iter().find(|(n, _)| n == name).map(|(_, id)| *id))
    }

    fn run(prog: &Program) -> Self {
        let mut w = Self::default();
        w.scopes.push(Vec::new());
        // PASS ONE — every top-level name, in arena order.
        for (slot, d) in prog.decls.iter().enumerate() {
            let Some(d) = d else { continue };
            w.declare(Sym {
                name: prog.tok_text(d.name),
                kind: Kind::of_decl(d.kind),
                line: prog.tok_line(d.name),
                depth: 1,
                decl: slot,
                block: 0,
                pos: 0,
                public: d.public,
                uses: 0,
            });
        }
        // PASS TWO — each routine body in its own scope, parameters first.
        for (slot, d) in prog.decls.iter().enumerate() {
            let Some(d) = d else { continue };
            if d.kind != DECL_ROUTINE {
                continue;
            }
            w.scopes.push(Vec::new());
            if d.p_first > 0 {
                for p in d.p_first..=d.p_last {
                    let Some(p) = prog.param(p) else { continue };
                    w.declare(Sym {
                        name: prog.tok_text(p.name),
                        kind: Kind::Param,
                        line: prog.tok_line(p.name),
                        depth: 2,
                        decl: 0,
                        block: 0,
                        pos: 0,
                        public: false,
                        uses: 0,
                    });
                }
            }
            let ctx = Ctx {
                depth: 2,
                site: Site::Plain,
                cond_depth: 0,
                routine: slot,
                chain: Vec::new(),
            };
            w.stmt(prog, d.body, &ctx);
            w.scopes.pop();
        }
        w
    }

    /// A block's direct children in written order — the chain `अन्तिम`,
    /// `आदिसूचकाङ्क वियोगः १`, … down to `प्रथम`, reversed.
    fn children(prog: &Program, s: Stmt) -> Vec<usize> {
        let mut out = Vec::new();
        if s.right == 0 {
            return out;
        }
        let mut c = s.right;
        loop {
            out.push(c);
            let Some(cs) = prog.stmt(c) else { break };
            if cs.start <= s.left || cs.start == 0 {
                break;
            }
            c = cs.start - 1;
        }
        out.reverse();
        out
    }

    fn stmt(&mut self, prog: &Program, i: usize, ctx: &Ctx) {
        let Some(s) = prog.stmt(i) else { return };
        if i == 0 {
            return;
        }
        self.reached_stmts.insert(i);
        match s.kind {
            STMT_EXPR | STMT_RETURN => self.expr(prog, s.left, ctx),
            STMT_BLOCK => {
                self.scopes.push(Vec::new());
                for (pos, c) in Self::children(prog, s).into_iter().enumerate() {
                    let mut inner = ctx.clone();
                    inner.depth += 1;
                    inner.chain.push((i, pos));
                    self.stmt(prog, c, &inner);
                }
                self.scopes.pop();
            }
            STMT_LET => {
                self.expr(prog, s.right, ctx);
                let (block, pos) = ctx.chain.last().copied().unwrap_or((0, 0));
                self.declare(Sym {
                    name: prog.tok_text(s.left),
                    kind: Kind::Local,
                    line: prog.tok_line(s.left),
                    depth: ctx.depth,
                    decl: 0,
                    block,
                    pos,
                    public: false,
                    uses: 0,
                });
            }
            STMT_IMPORT => {}
            STMT_ASSIGN => {
                self.expr(prog, s.left, ctx);
                self.expr(prog, s.right, ctx);
            }
            STMT_IF | STMT_WHILE => {
                let mut cond = ctx.clone();
                cond.site = Site::Condition;
                self.expr(prog, s.left, &cond);
                let mut body = ctx.clone();
                body.site = if s.kind == STMT_IF {
                    Site::IfBody
                } else {
                    Site::WhileBody
                };
                body.cond_depth += 1;
                self.stmt(prog, s.right, &body);
                // THE `अन्यथा` BODY. ० means there is none, and `यावत्` reaches
                // here with ० by construction. Before `W-202` nothing pointed
                // at these statements and this walk counted them as ORPHANS;
                // the resolver descends now, so the replica does too.
                if s.kind == STMT_IF && s.alt != 0 {
                    let mut alt = ctx.clone();
                    alt.site = Site::IfBody;
                    alt.cond_depth += 1;
                    self.stmt(prog, s.alt, &alt);
                }
            }
            other => panic!("{}: statement kind {other} is none of the eight", prog.file),
        }
    }

    fn expr(&mut self, prog: &Program, i: usize, ctx: &Ctx) {
        if i == 0 {
            return;
        }
        let Some(e) = prog.expr(i) else { return };
        self.reached_exprs.insert(i);
        match e.kind {
            EXPR_NAME => {
                // `W-228` (b): the spaced qualifier folds into a नाम node whose
                // `दक्षिणसूचकाङ्क` names the member token — one tree with the
                // unspaced form, and one reading here.
                let name = if e.right != 0 {
                    format!(
                        "{}\u{971}{}",
                        prog.tok_text(e.value),
                        prog.tok_text(e.right)
                    )
                } else {
                    prog.tok_text(e.value)
                };
                let (replica, qualified) = match module_prefix(&name) {
                    Some((module, _)) => (self.lookup(module), true),
                    None => (self.lookup(&name), false),
                };
                if let Some(s) = replica {
                    self.syms[s].uses += 1;
                }
                self.uses.push(Use {
                    name,
                    node_symbol: e.symbol,
                    replica,
                    qualified,
                    depth: ctx.depth,
                    site: ctx.site,
                    cond_depth: ctx.cond_depth,
                    routine: ctx.routine,
                    line: prog.tok_line(e.value),
                    chain: ctx.chain.clone(),
                });
            }
            EXPR_NUMERAL | EXPR_STRING | EXPR_BOOL | EXPR_VOID | EXPR_EMBED => {}
            EXPR_NEG => self.expr(prog, e.left, ctx),
            _ => {
                self.expr(prog, e.left, ctx);
                self.expr(prog, e.right, ctx);
            }
        }
    }
}

/// Every expression node under a type: kinds २–५ wrap `वामसूचकाङ्क`.
fn mark_type(prog: &Program, i: usize, out: &mut BTreeSet<usize>) {
    if i == 0 || !out.insert(i) {
        return;
    }
    if let Some(e) = prog.expr(i)
        && (2..=5).contains(&e.kind)
    {
        mark_type(prog, e.left, out);
    }
}

/// Every expression node under an expression, regardless of kind.
fn mark_expr(prog: &Program, i: usize, out: &mut BTreeSet<usize>) {
    if i == 0 || !out.insert(i) {
        return;
    }
    if let Some(e) = prog.expr(i) {
        mark_expr(prog, e.left, out);
        mark_expr(prog, e.right, out);
    }
}

/// Where every Identifier-shaped node of a program sits.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct NodePlaces {
    walked: usize,
    type_position: usize,
    initializer: usize,
    orphan: usize,
    orphan_resolved: usize,
}

/// Every expression node that sits in a TYPE position: a declaration's type,
/// a parameter's, a `चरः`'s.
fn type_positions(prog: &Program) -> BTreeSet<usize> {
    let mut types = BTreeSet::new();
    for d in prog.decls.iter().flatten() {
        mark_type(prog, d.ty, &mut types);
    }
    for p in prog.params.iter().flatten() {
        mark_type(prog, p.ty, &mut types);
    }
    for s in prog.stmts.iter().flatten() {
        if s.kind == STMT_LET {
            mark_type(prog, s.ty, &mut types);
        }
    }
    types
}

impl Walk {
    /// A type position names a declared TYPE, and the resolver never looks at
    /// it — `प्रकारार्थः` reads the text. Counted here as a reference so that a
    /// `संरचना` used only as a type is not reported dead: bare names against
    /// the global scope, qualified ones returned for the cross-module count.
    fn type_references(&mut self, prog: &Program, types: &BTreeSet<usize>) {
        for &i in types {
            let Some(e) = prog.expr(i) else { continue };
            if e.kind != EXPR_NAME {
                continue;
            }
            let name = prog.tok_text(e.value);
            if module_prefix(&name).is_some() {
                self.type_qualified.push(name);
                self.type_qualified_lines.push(prog.tok_line(e.value));
                continue;
            }
            self.type_bare += 1;
            let global = self.scopes.first().expect("the global scope survives");
            if let Some((_, s)) = global.iter().find(|(n, _)| *n == name)
                && self.syms[*s].kind == Kind::Type
            {
                self.syms[*s].uses += 1;
            }
        }
    }
}

fn place_nodes(prog: &Program, w: &Walk, types: &BTreeSet<usize>) -> NodePlaces {
    let mut inits = BTreeSet::new();
    for d in prog.decls.iter().flatten() {
        if d.kind == DECL_GLOBAL {
            mark_expr(prog, d.body, &mut inits);
        }
    }
    let mut p = NodePlaces::default();
    for (i, e) in prog.exprs.iter().enumerate() {
        let Some(e) = e else { continue };
        if e.kind != EXPR_NAME {
            continue;
        }
        if w.reached_exprs.contains(&i) {
            p.walked += 1;
        } else if types.contains(&i) {
            p.type_position += 1;
        } else if inits.contains(&i) {
            p.initializer += 1;
        } else {
            p.orphan += 1;
            if e.symbol != 0 {
                p.orphan_resolved += 1;
            }
        }
    }
    p
}

// ── the cross-module pieces: collisions and the import graph ────────────

/// Public names declared in more than one unit, `name → units`, keyed however
/// the caller names a unit (file stem, as W-192 did; or module).
fn collisions(units: &[(String, Vec<String>)]) -> Vec<(String, Vec<String>)> {
    let mut owner: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (unit, names) in units {
        for n in names {
            let e = owner.entry(n.clone()).or_default();
            if !e.contains(unit) {
                e.push(unit.clone());
            }
        }
    }
    owner.into_iter().filter(|(_, u)| u.len() > 1).collect()
}

/// W-192's text scan (`t1_sources.rs::public_names_are_not_declared_in_two_modules`),
/// verbatim in method: the first three words of a code line, keyed by file.
fn collisions_by_text_scan(files: &[String]) -> BTreeSet<String> {
    let mut units: Vec<(String, Vec<String>)> = Vec::new();
    for f in files {
        let mut names = Vec::new();
        for line in source(f).lines() {
            let code = line.split('॰').next().unwrap_or("").trim();
            let mut w = code.split(' ');
            if w.next() != Some("सार्वजनिक") {
                continue;
            }
            if !matches!(w.next().unwrap_or(""), "वृत्तिः" | "चरः" | "संरचना" | "गणना")
            {
                continue;
            }
            if let Some(name) = w.next().filter(|n| !n.is_empty() && *n != "ॱॱ") {
                names.push(name.to_string());
            }
        }
        units.push((f.clone(), names));
    }
    collisions(&units).into_iter().map(|(n, _)| n).collect()
}

/// Every cycle of a directed graph, each as the nodes on it in order, found
/// by a depth-first search over `edges` (`from → to`).
fn cycles(nodes: &[String], edges: &[(String, String)]) -> Vec<Vec<String>> {
    fn visit(
        n: &str,
        edges: &[(String, String)],
        state: &mut BTreeMap<String, u8>,
        path: &mut Vec<String>,
        out: &mut Vec<Vec<String>>,
    ) {
        state.insert(n.to_string(), 1);
        path.push(n.to_string());
        for (a, b) in edges {
            if a != n {
                continue;
            }
            match state.get(b).copied().unwrap_or(0) {
                1 => {
                    let at = path.iter().position(|p| p == b).unwrap_or(0);
                    out.push(path[at..].to_vec());
                }
                0 => visit(b, edges, state, path, out),
                _ => {}
            }
        }
        path.pop();
        state.insert(n.to_string(), 2);
    }
    let mut state = BTreeMap::new();
    let mut out = Vec::new();
    for n in nodes {
        if state.get(n).copied().unwrap_or(0) == 0 {
            visit(n, edges, &mut state, &mut Vec::new(), &mut out);
        }
    }
    out
}

fn quantiles(v: &mut [i64]) -> String {
    if v.is_empty() {
        return "n=0".to_string();
    }
    v.sort_unstable();
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    format!(
        "n={} min={} median={} p90={} max={}",
        v.len(),
        v[0],
        at(0.5),
        at(0.9),
        v[v.len() - 1]
    )
}

fn corpus_files() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("corpus is readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    names
}

// ── the census ──────────────────────────────────────────────────────────

/// W-209's census: statistics 7–10, 20, 21 over the fifteen sources.
///
/// Reports rather than asserts, like every census in this crate; the one
/// assertion is the replica's self-check, because a divergence there makes
/// every other number here about the wrong symbol.
#[test]
#[ignore = "measurement"]
fn measure_paradigm_names() {
    let files = corpus_files();
    let mut programs = Vec::new();
    for f in &files {
        let src = source(f);
        let p = read_program(f, &src);
        let mut w = Walk::run(&p);
        let types = type_positions(&p);
        w.type_references(&p, &types);
        let places = place_nodes(&p, &w, &types);
        let bare_walked = w.uses.iter().filter(|u| !u.qualified).count();
        let bare_ok = w
            .uses
            .iter()
            .filter(|u| !u.qualified && u.node_symbol != 0)
            .count();
        println!(
            "  {:<20} {:<12} decls={:>3} parse_errors={} resolved={:<5} walked={:>4} bare={:>4}/{:<4} qualified={:>3} types={:>4} init={:>2} orphan={:>3}{}",
            f,
            p.module.clone().unwrap_or_else(|| "(no मण्डलम्)".into()),
            p.decls.iter().flatten().count(),
            p.parse_errors,
            p.resolved,
            places.walked,
            bare_ok,
            bare_walked,
            w.uses.iter().filter(|u| u.qualified).count(),
            places.type_position,
            places.initializer,
            places.orphan,
            p.refusal
                .as_ref()
                .map(|r| format!("  REFUSED: {r}"))
                .unwrap_or_default()
        );
        programs.push((p, w, places));
    }

    // ── (7) per-use resolution ───────────────────────────────────────────
    let resolved_files = programs.iter().filter(|(p, _, _)| p.resolved).count();
    println!("METRIC paradigm_name_files {}", programs.len());
    println!("METRIC paradigm_name_files_resolved {resolved_files}");
    let sum =
        |f: &dyn Fn(&NodePlaces) -> usize| programs.iter().map(|(_, _, n)| f(n)).sum::<usize>();
    let walked = sum(&|n| n.walked);
    let type_pos = sum(&|n| n.type_position);
    let inits = sum(&|n| n.initializer);
    let orphans = sum(&|n| n.orphan);
    let orphans_resolved = sum(&|n| n.orphan_resolved);
    let all_nodes = walked + type_pos + inits + orphans;
    println!("METRIC paradigm_name_identifier_shaped_nodes {all_nodes}");
    println!("METRIC paradigm_name_nodes_in_type_position {type_pos}");
    println!("METRIC paradigm_name_nodes_in_module_initializers {inits}");
    println!("METRIC paradigm_name_uses_walked {walked}");
    println!("METRIC paradigm_name_uses_orphaned_else_bodies {orphans}");
    println!("METRIC paradigm_name_uses_orphaned_with_symbol {orphans_resolved}");
    let else_tokens: usize = programs
        .iter()
        .map(|(p, _, _)| {
            p.toks
                .iter()
                .flatten()
                .filter(|t| t.text == "अन्यथा")
                .count()
        })
        .sum();
    println!("METRIC paradigm_name_else_bodies {else_tokens}");
    let unreached_stmts: usize = programs
        .iter()
        .map(|(p, w, _)| {
            p.stmts
                .iter()
                .enumerate()
                .filter(|(i, s)| s.is_some() && !w.reached_stmts.contains(i))
                .count()
        })
        .sum();
    println!("METRIC paradigm_name_statements_never_walked {unreached_stmts}");

    let uses: Vec<&Use> = programs
        .iter()
        .flat_map(|(_, w, _)| w.uses.iter())
        .collect();
    let bare: Vec<&Use> = uses.iter().copied().filter(|u| !u.qualified).collect();
    let bare_ok = bare.iter().filter(|u| u.node_symbol != 0).count();
    let qualified: Vec<&Use> = uses.iter().copied().filter(|u| u.qualified).collect();
    let qualified_checked = qualified.iter().filter(|u| u.replica.is_some()).count();
    let qualified_written = qualified.iter().filter(|u| u.node_symbol != 0).count();
    println!("METRIC paradigm_name_uses_bare {}", bare.len());
    println!("METRIC paradigm_name_uses_bare_resolved {bare_ok}");
    println!("METRIC paradigm_name_uses_qualified {}", qualified.len());
    println!("METRIC paradigm_name_uses_qualified_import_checked {qualified_checked}");
    println!("METRIC paradigm_name_uses_qualified_with_symbol_written {qualified_written}");
    let rate = |a: usize, b: usize| if b == 0 { 0.0 } else { a as f64 / b as f64 };
    println!(
        "METRIC paradigm_name_resolution_rate_bare_walked {:.4}",
        rate(bare_ok, bare.len())
    );
    println!(
        "METRIC paradigm_name_resolution_rate_walked_incl_qualified {:.4}",
        rate(bare_ok + qualified_checked, uses.len())
    );
    println!(
        "METRIC paradigm_name_resolution_rate_lower_bound_all_uses {:.4}",
        rate(bare_ok + qualified_checked, uses.len() + orphans)
    );
    // `u.replica` is a POSITION in the replica list, and TWO offsets compose
    // on top of it: since `c107c725` (2026-09-13) the replica's SYMBOL is its
    // position + १ — ids mint from १ because an optional is ONE WORD natively
    // and `Some(०)` was the nil word — and the tree's own `+ १`, which exists
    // so ० can mean "no symbol", applies on top. So the node carries
    // position + २. `the_replica_walk_agrees_with_the_resolver_and_counts_what_it_skips`
    // (:2327) was re-pinned to + २ at the time and this copy WAS NOT, so every
    // bare use disagreed by exactly one: `paradigm_name_uses_bare`,
    // `..._uses_bare_resolved` and `..._replica_disagreements` all read the
    // same number, which is the signature of a stale offset and not of a walk
    // disagreeing structurally. The two sites must move together.
    //
    // MEASURED BOTH WAYS ON THIS TREE (`b4ba8b43`, 2026-09-18), because a
    // recovered fix asserted is a fix re-read and not re-run: with `+ १` the
    // three metrics read 20,522 / 20,522 / 20,522 — every bare use in the corpus
    // called a disagreement — and with `+ २` they read 20,522 / 20,522 / ०. A
    // metric equal to its own population is never a finding about the tree; it
    // is the instrument reporting its own offset, which is why this sat unread.
    //
    // RE-RUN AT `42cbb7aa` (2026-09-19) ON LANDING, AND THE POPULATION HAD MOVED
    // WHILE THE PIN HELD: 20,569 / 20,569 / ० — forty-seven more bare uses than
    // the `b4ba8b43` reading above, because peers landed `.t1` in between. The
    // two numbers here are DATED TO THEIR TREES on purpose; a reader who takes
    // `20,522` as a claim about the tree in front of them will find it off by
    // however much `.t1` has grown since. The assertion is `disagreements == ०`,
    // which is a statement about the OFFSET and survives the corpus growing; the
    // figures are the evidence that produced it and not a second pin.
    //
    // RE-RUN AGAIN AT `b9b9b7de` (2026-09-19) ON THIS LANDING: 20,569 / 20,569 /
    // ० — unchanged from the `42cbb7aa` reading above, so no `.t1` landed in
    // between moved the population. NOTE FOR A LATER READER, because the recovery
    // note does not say it: `disagreements` is PRINTED and never asserted, so
    // this half of the fix moves a METRIC and reds nothing. The assertion that
    // does bite on the offset is
    // `the_replica_walk_agrees_with_the_resolver_and_counts_what_it_skips`
    // (:2327), which already carried `+ २`; this site was the stale twin.
    let disagreements = programs
        .iter()
        .filter(|(p, _, _)| p.resolved)
        .flat_map(|(_, w, _)| w.uses.iter())
        .filter(|u| {
            let expect = if u.qualified {
                0
            } else {
                // The conversion cannot fail for a real position; its fallback
                // is -१ rather than the old `-1 + 1 == ०`, which would have
                // made an unconvertible position AGREE with an unresolved node
                // instead of reporting the disagreement it is.
                u.replica
                    .map_or(0, |s| i128::try_from(s).map_or(-1, |v| v + 2))
            };
            u.node_symbol != expect
        })
        .count();
    println!("METRIC paradigm_name_replica_disagreements {disagreements}");

    for (label, pick) in [
        (
            "outside_conditional_bodies",
            &(|u: &Use| u.cond_depth == 0) as &dyn Fn(&Use) -> bool,
        ),
        ("inside_conditional_bodies", &|u: &Use| u.cond_depth > 0),
        ("inside_if_bodies", &|u: &Use| u.site == Site::IfBody),
        ("inside_while_bodies", &|u: &Use| u.site == Site::WhileBody),
        ("in_conditions", &|u: &Use| u.site == Site::Condition),
    ] {
        let sel: Vec<&Use> = bare.iter().copied().filter(|u| pick(u)).collect();
        let ok = sel.iter().filter(|u| u.node_symbol != 0).count();
        println!("METRIC paradigm_name_uses_bare_{label} {}", sel.len());
        println!("METRIC paradigm_name_uses_bare_{label}_resolved {ok}");
    }
    let mut depth_hist: BTreeMap<usize, usize> = BTreeMap::new();
    for u in &bare {
        *depth_hist.entry(u.cond_depth).or_default() += 1;
    }
    println!("  bare uses by conditional nesting depth: {depth_hist:?}");

    // ── exports and the qualified references into them ───────────────────
    // module → (name → (file, kind))
    let mut exports: BTreeMap<String, BTreeMap<String, (String, i128)>> = BTreeMap::new();
    for (p, _, _) in &programs {
        let Some(m) = &p.module else { continue };
        for (n, k) in p.exports() {
            exports
                .entry(m.clone())
                .or_default()
                .insert(n, (p.file.clone(), k));
        }
    }
    let declared_modules: BTreeSet<String> = exports.keys().cloned().collect();
    // (module, member) → qualified references from OTHER files
    let mut reached: BTreeMap<(String, String), usize> = BTreeMap::new();
    let (mut q_found, mut q_missing, mut q_self, mut q_unknown_module) = (0, 0, 0, 0);
    let mut missing_list: BTreeSet<String> = BTreeSet::new();
    for (p, w, _) in &programs {
        for u in w.uses.iter().filter(|u| u.qualified) {
            let Some((m, member)) = module_prefix(&u.name) else {
                continue;
            };
            if p.module.as_deref() == Some(m) {
                q_self += 1;
                continue;
            }
            match exports.get(m) {
                None => q_unknown_module += 1,
                Some(names) if names.contains_key(member) => {
                    q_found += 1;
                    *reached
                        .entry((m.to_string(), member.to_string()))
                        .or_default() += 1;
                }
                Some(_) => {
                    q_missing += 1;
                    missing_list.insert(format!("{}:{} {}", p.file, u.line, u.name));
                }
            }
        }
    }
    // Qualified names in TYPE positions reach another module's types the same
    // way; the resolver never sees them, `प्रकारार्थः` does.
    let (mut t_found, mut t_missing, mut t_bare) = (0, 0, 0);
    for (p, w, _) in &programs {
        t_bare += w.type_bare;
        for name in &w.type_qualified {
            let Some((m, member)) = module_prefix(name) else {
                continue;
            };
            if p.module.as_deref() == Some(m) {
                continue;
            }
            match exports.get(m) {
                Some(names) if names.contains_key(member) => {
                    t_found += 1;
                    *reached
                        .entry((m.to_string(), member.to_string()))
                        .or_default() += 1;
                }
                _ => {
                    t_missing += 1;
                    missing_list.insert(format!("{} type {name}", p.file));
                }
            }
        }
    }
    println!("METRIC paradigm_name_type_references_bare {t_bare}");
    println!("METRIC paradigm_name_type_references_qualified_found {t_found}");
    println!("METRIC paradigm_name_type_references_qualified_missing {t_missing}");

    // Names the RUST tests call by string — `it.call("अर्थॱकार्यक्रमनिर्णयः", …)`
    // — are reached from outside the T1 corpus, and a dead-name list that
    // ignored them would list every entry point of the chain.
    let rust_tests: String = ["sadhana-t1/tests", "sadhana/tests"]
        .iter()
        .flat_map(|d| {
            std::fs::read_dir(repo_root().join("crates").join(d))
                .into_iter()
                .flatten()
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();

    // ── (8) uses per declaration, dead names ─────────────────────────────
    let mut hist: BTreeMap<&str, usize> = BTreeMap::new();
    let bucket = |n: usize| match n {
        0 => "0",
        1 => "1",
        2..=3 => "2-3",
        4..=10 => "4-10",
        11..=50 => "11-50",
        _ => "51+",
    };
    let mut by_kind: BTreeMap<Kind, (usize, usize, usize)> = BTreeMap::new(); // (decls, uses, dead)
    let mut dead: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut top: Vec<(usize, String)> = Vec::new();
    let mut total_syms = 0;
    for (p, w, _) in &programs {
        for s in &w.syms {
            total_syms += 1;
            let external = if s.kind.is_global() && s.public {
                p.module
                    .as_ref()
                    .and_then(|m| reached.get(&(m.clone(), s.name.clone())))
                    .copied()
                    .unwrap_or(0)
            } else {
                0
            };
            let n = s.uses + external;
            *hist.entry(bucket(n)).or_default() += 1;
            let e = by_kind.entry(s.kind).or_default();
            e.0 += 1;
            e.1 += n;
            top.push((n, format!("{}:{} {}", p.file, s.name, kind_name(s.kind))));
            if n == 0 {
                e.2 += 1;
                let class = match (s.kind, s.public) {
                    (Kind::Routine | Kind::Type | Kind::Machine | Kind::Global, true) => {
                        "public, never referenced anywhere"
                    }
                    (Kind::Routine | Kind::Type | Kind::Machine | Kind::Global, false) => {
                        "private, never used"
                    }
                    // NOT "dead": `uses` counts what the resolver WALKS, and
                    // it never walks a TYPE position, so an import reached
                    // only as `अङ्कः अन्तः वास्तुॱवस्तु` lands here alive.
                    // `W-225` read the nine this class reported and found
                    // seven dead and two live (`samyojana.t1`'s `वास्तु`, 55
                    // type reaches, and its `कोश`). The ratchet that counts
                    // every reach is `t1_modules.rs`'s
                    // `no_import_in_this_crate_is_dead_or_names_an_undeclared_module`.
                    (Kind::Import, _) => "import never used as a prefix IN A WALKED BODY",
                    (Kind::Param, _) => "parameter never read",
                    (Kind::Local, _) => "local never read",
                };
                dead.entry(class)
                    .or_default()
                    .push(format!("{}:{} {}", p.file, s.line, s.name));
            }
        }
    }
    println!("METRIC paradigm_name_declarations {total_syms}");
    for (k, (d, u, z)) in &by_kind {
        println!("METRIC paradigm_name_declarations_{} {d}", kind_name(*k));
        println!("METRIC paradigm_name_uses_of_{} {u}", kind_name(*k));
        println!("METRIC paradigm_name_dead_{} {z}", kind_name(*k));
    }
    for b in ["0", "1", "2-3", "4-10", "11-50", "51+"] {
        println!(
            "METRIC paradigm_name_indegree_{} {}",
            b.replace('-', "_").replace('+', "_plus"),
            hist.get(b).copied().unwrap_or(0)
        );
    }
    top.sort_by(|a, b| b.0.cmp(&a.0));
    println!("  most-used declarations: {:?}", &top[..top.len().min(10)]);
    let dead_total: usize = dead.values().map(Vec::len).sum();
    println!("METRIC paradigm_name_dead_names {dead_total}");
    for (class, names) in &dead {
        if class.starts_with("public") {
            let (from_rust, nowhere): (Vec<&String>, Vec<&String>) = names.iter().partition(|n| {
                n.rsplit(' ')
                    .next()
                    .is_some_and(|bare| rust_tests.contains(bare))
            });
            println!(
                "METRIC paradigm_name_dead_public_called_from_rust_tests {}",
                from_rust.len()
            );
            println!(
                "METRIC paradigm_name_dead_public_referenced_nowhere {}",
                nowhere.len()
            );
            println!(
                "  DEAD ({class}; named in a Rust test) [{}]: {}",
                from_rust.len(),
                from_rust
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!(
                "  DEAD ({class}; named nowhere in T1 or the Rust tests) [{}]: {}",
                nowhere.len(),
                nowhere
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else {
            println!("  DEAD ({class}) [{}]: {}", names.len(), names.join(", "));
        }
    }

    // ── (9) use–declaration distance ─────────────────────────────────────
    let mut local_lines = Vec::new();
    let mut local_stmts = Vec::new();
    let mut param_stmts = Vec::new();
    let mut global_lines = Vec::new();
    let mut global_decls = Vec::new();
    let mut depth_dist: BTreeMap<i64, usize> = BTreeMap::new();
    let (mut forward, mut local_before, mut chain_miss) = (0, 0, 0);
    let mut forward_names: BTreeSet<String> = BTreeSet::new();
    let mut forward_by_file: BTreeMap<String, usize> = BTreeMap::new();
    for (p, w, _) in &programs {
        for u in &w.uses {
            let Some(s) = u.replica else { continue };
            let s = &w.syms[s];
            let dl = i64::try_from(u.line - s.line).unwrap_or(0);
            let dd = i64::try_from(u.depth).unwrap_or(0) - i64::try_from(s.depth).unwrap_or(0);
            *depth_dist.entry(dd).or_default() += 1;
            match s.kind {
                Kind::Local => {
                    local_lines.push(dl);
                    match u.chain.iter().find(|(b, _)| *b == s.block) {
                        Some((_, pos)) => {
                            let d = i64::try_from(*pos).unwrap_or(0)
                                - i64::try_from(s.pos).unwrap_or(0);
                            local_stmts.push(d);
                            if d < 0 {
                                local_before += 1;
                            }
                        }
                        None => chain_miss += 1,
                    }
                }
                Kind::Param => {
                    param_stmts.push(
                        u.chain
                            .first()
                            .map_or(0, |(_, pos)| i64::try_from(*pos).unwrap_or(0)),
                    );
                }
                _ => {
                    global_lines.push(dl);
                    let d =
                        i64::try_from(u.routine).unwrap_or(0) - i64::try_from(s.decl).unwrap_or(0);
                    global_decls.push(d);
                    if d < 0 {
                        forward += 1;
                        forward_names.insert(format!("{}:{} {}", p.file, u.line, u.name));
                        *forward_by_file.entry(p.file.clone()).or_default() += 1;
                    }
                }
            }
        }
    }
    println!(
        "  distance, locals, in lines: {}",
        quantiles(&mut local_lines)
    );
    println!(
        "  distance, locals, in statements of the declaring block: {}",
        quantiles(&mut local_stmts)
    );
    println!(
        "  distance, parameters, in statements from the body's start: {}",
        quantiles(&mut param_stmts)
    );
    println!(
        "  distance, globals, in lines: {}",
        quantiles(&mut global_lines)
    );
    println!(
        "  distance, globals, in declarations (negative = use before declaration): {}",
        quantiles(&mut global_decls)
    );
    println!("  scope-depth distance (use depth − declaration depth): {depth_dist:?}");
    println!("METRIC paradigm_name_local_uses_before_declaration {local_before}");
    println!("METRIC paradigm_name_local_uses_outside_declaring_chain {chain_miss}");
    println!("METRIC paradigm_name_global_uses_before_declaration {forward}");
    println!(
        "METRIC paradigm_name_global_uses_after_declaration {}",
        global_decls.iter().filter(|d| **d > 0).count()
    );
    println!(
        "METRIC paradigm_name_global_uses_within_own_declaration {}",
        global_decls.iter().filter(|d| **d == 0).count()
    );
    let mut fwd: Vec<&String> = forward_names.iter().collect();
    fwd.truncate(12);
    println!("  first forward references: {fwd:?}");
    println!("  forward references by file: {forward_by_file:?}");

    // ── (10) bare-name collisions, bare vs qualified across modules ──────
    let by_file: Vec<(String, Vec<String>)> = programs
        .iter()
        .map(|(p, _, _)| {
            (
                p.file.clone(),
                p.exports().into_iter().map(|(n, _)| n).collect(),
            )
        })
        .collect();
    let by_module: Vec<(String, Vec<String>)> = programs
        .iter()
        .filter_map(|(p, _, _)| {
            p.module
                .clone()
                .map(|m| (m, p.exports().into_iter().map(|(n, _)| n).collect()))
        })
        .collect();
    let coll_file = collisions(&by_file);
    let coll_module = collisions(&by_module);
    // W-192's own instrument, re-run: a TEXT scan of `सार्वजनिक <kind> <name>`
    // lines. The parser sees what a line scan cannot — an enum's variants are
    // public declarations on lines that do not start with `सार्वजनिक` — so
    // the two are printed against each other rather than one trusted.
    let text_scan = collisions_by_text_scan(&files);
    let parser_only: Vec<&String> = coll_file
        .iter()
        .map(|(n, _)| n)
        .filter(|n| !text_scan.contains(*n))
        .collect();
    let text_only: Vec<&String> = text_scan
        .iter()
        .filter(|n| !coll_file.iter().any(|(m, _)| m == *n))
        .collect();
    println!(
        "METRIC paradigm_name_bare_collisions_by_w192_text_scan {}",
        text_scan.len()
    );
    println!("  collisions the parser sees and the text scan does not: {parser_only:?}");
    println!("  collisions the text scan sees and the parser does not: {text_only:?}");
    println!(
        "METRIC paradigm_name_bare_collisions_by_file {}",
        coll_file.len()
    );
    println!(
        "METRIC paradigm_name_bare_collisions_by_module {}",
        coll_module.len()
    );
    for (n, fs) in &coll_file {
        println!("  COLLISION {n} in {}", fs.join(", "));
    }
    let colliding: BTreeSet<&String> = coll_file.iter().map(|(n, _)| n).collect();
    let bare_of_colliding = programs
        .iter()
        .flat_map(|(_, w, _)| w.uses.iter().filter(|u| !u.qualified).map(move |u| (w, u)))
        .filter(|(w, u)| {
            u.replica
                .is_some_and(|s| w.syms[s].kind.is_global() && colliding.contains(&w.syms[s].name))
        })
        .count();
    println!("METRIC paradigm_name_bare_uses_of_colliding_names {bare_of_colliding}");
    // A bare use resolving across a module boundary: the resolver has one
    // file's declarations, so this is ० by construction; measured anyway.
    let bare_cross = bare
        .iter()
        .filter(|u| u.node_symbol != 0 && u.replica.is_none())
        .count();
    println!("METRIC paradigm_name_bare_uses_resolved_across_modules {bare_cross}");
    println!(
        "METRIC paradigm_name_cross_module_refs_qualified {}",
        q_found + q_missing
    );
    println!("METRIC paradigm_name_cross_module_refs_member_found {q_found}");
    println!("METRIC paradigm_name_cross_module_refs_member_missing {q_missing}");
    println!("METRIC paradigm_name_qualified_refs_to_own_module {q_self}");
    println!("METRIC paradigm_name_qualified_refs_to_undeclared_module {q_unknown_module}");
    if !missing_list.is_empty() {
        println!(
            "  MEMBER MISSING: {:?}",
            missing_list.iter().take(20).collect::<Vec<_>>()
        );
    }
    let all_exports: BTreeMap<&String, usize> = {
        let mut m: BTreeMap<&String, usize> = BTreeMap::new();
        for names in exports.values() {
            for n in names.keys() {
                *m.entry(n).or_default() += 1;
            }
        }
        m
    };
    let (mut q_unique, mut q_collides) = (0, 0);
    for u in &qualified {
        let Some((_, member)) = module_prefix(&u.name) else {
            continue;
        };
        match all_exports.get(&member.to_string()).copied().unwrap_or(0) {
            0 | 1 => q_unique += 1,
            _ => q_collides += 1,
        }
    }
    println!("METRIC paradigm_name_qualified_refs_member_unique_corpus_wide {q_unique}");
    println!("METRIC paradigm_name_qualified_refs_member_collides {q_collides}");

    // ── (20) the import graph ────────────────────────────────────────────
    let mut nodes: Vec<String> = Vec::new();
    let mut edges: Vec<(String, String)> = Vec::new();
    let mut in_body = 0;
    let mut unknown: BTreeSet<String> = BTreeSet::new();
    let mut files_per_module: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (p, _, _) in &programs {
        let me = p
            .module
            .clone()
            .unwrap_or_else(|| format!("{}(no मण्डलम्)", p.file));
        if let Some(m) = &p.module {
            files_per_module
                .entry(m.clone())
                .or_default()
                .push(p.file.clone());
        }
        if !nodes.contains(&me) {
            nodes.push(me.clone());
        }
        for m in p.imports() {
            if !declared_modules.contains(&m) {
                unknown.insert(format!("{} → {m}", p.file));
            }
            if !nodes.contains(&m) {
                nodes.push(m.clone());
            }
            edges.push((me.clone(), m));
        }
        in_body += p
            .stmts
            .iter()
            .flatten()
            .filter(|s| s.kind == STMT_IMPORT)
            .count();
    }
    println!("METRIC paradigm_name_import_modules {}", nodes.len());
    println!("METRIC paradigm_name_import_edges {}", edges.len());
    println!("METRIC paradigm_name_import_statements_in_bodies {in_body}");
    println!(
        "METRIC paradigm_name_imports_of_undeclared_modules {}",
        unknown.len()
    );
    println!("  imports of undeclared modules: {unknown:?}");
    let two_files: Vec<_> = files_per_module
        .iter()
        .filter(|(_, f)| f.len() > 1)
        .collect();
    println!(
        "METRIC paradigm_name_modules_declared_by_two_files {}",
        two_files.len()
    );
    println!("  modules declared by more than one file: {two_files:?}");
    let mut degrees: Vec<(String, usize, usize)> = nodes
        .iter()
        .map(|n| {
            (
                n.clone(),
                edges.iter().filter(|(_, b)| b == n).count(),
                edges.iter().filter(|(a, _)| a == n).count(),
            )
        })
        .collect();
    degrees.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("  import graph in/out degree: {degrees:?}");
    println!(
        "METRIC paradigm_name_import_indegree_max {}",
        degrees.iter().map(|d| d.1).max().unwrap_or(0)
    );
    println!(
        "METRIC paradigm_name_import_outdegree_max {}",
        degrees.iter().map(|d| d.2).max().unwrap_or(0)
    );
    println!(
        "METRIC paradigm_name_modules_imported_by_nobody {}",
        degrees.iter().filter(|d| d.1 == 0).count()
    );
    let cyc = cycles(&nodes, &edges);
    println!("METRIC paradigm_name_import_cycles {}", cyc.len());
    println!(
        "METRIC paradigm_name_import_graph_acyclic {}",
        usize::from(cyc.is_empty())
    );
    for c in &cyc {
        println!("  CYCLE {}", c.join(" → "));
    }
    // THE RATCHET (W-224, research/22 §8): cycles are allowed by rule — a
    // junction may be entered from both sides — and every one is between
    // `सङ्केतन` and one of the four modules below. `t1_modules.rs` names
    // every SIMPLE cycle of the same graph (seven: four 2-cycles and the
    // three rings they compose) on every gate; that is the walk-independent
    // pin. THIS number is one per DFS back edge and moves with the walk: it
    // read 4 at `e0374d44` and reads 5 since `parse.t1` imported `अक्षरकोश`,
    // because the walk now enters the hub's cluster through `व्याकर →
    // अक्षरकोश` and `वाक्यविभाग → अक्षरकोश` closes on a node still on the
    // stack. Same seven cycles, one more back edge — an edge on no cycle
    // moved the count, which is why the graph's pin lives in t1_modules.rs.
    let mut partners: Vec<String> = cyc
        .iter()
        .flat_map(|c| c.iter().filter(|n| *n != "सङ्केतन").cloned())
        .collect();
    partners.sort();
    partners.dedup();
    assert_eq!(
        (cyc.len(), partners),
        (
            5,
            ["अक्षरकोश", "निदान", "वाक्यविभाग", "विश्लेषण"]
                .iter()
                .map(ToString::to_string)
                .collect()
        ),
        "the import graph's back-edge cycles moved from W-224's five, every one \
         through `सङ्केतन` with one of its four partners: {cyc:?}. A new cycle \
         is allowed by rule but is NAMED — add it to IMPORT_CYCLES in \
         t1_modules.rs and re-pin here, in the same commit"
    );

    // ── (21) exports per module, share reached through the member mark ───
    let mut export_total = 0;
    let mut reached_total = 0;
    let mut never: Vec<String> = Vec::new();
    for (m, names) in &exports {
        let r = names
            .keys()
            .filter(|n| reached.contains_key(&(m.clone(), (*n).clone())))
            .count();
        export_total += names.len();
        reached_total += r;
        println!(
            "  exports {m:<14} {:>3} public, {r:>3} reached from another file through ॱ ({:.0}%)",
            names.len(),
            100.0 * rate(r, names.len())
        );
        for n in names.keys() {
            if !reached.contains_key(&(m.clone(), n.clone())) {
                never.push(format!("{m}ॱ{n}"));
            }
        }
    }
    println!("METRIC paradigm_name_exports {export_total}");
    println!("METRIC paradigm_name_exports_reached_through_member_mark {reached_total}");
    println!(
        "METRIC paradigm_name_exports_reached_share {:.4}",
        rate(reached_total, export_total)
    );
    println!("METRIC paradigm_name_exports_reached_bare_from_another_file {bare_cross}");
    println!(
        "METRIC paradigm_name_exports_never_reached_externally {}",
        never.len()
    );

    assert_eq!(
        disagreements, 0,
        "the replica walk numbered a symbol differently from the resolver in a \
         file the resolver accepted; every distance and in-degree above is suspect"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `W-223` part 1 — THE SHARED DECLARATION STORE and the qualified-use census.
//
// Every qualified use (`मण्डलॱनाम`) and qualified type position the corpus
// writes, looked up against `sanchaya.t1`'s store of all 18 sources' declarations
// — without touching the resolver, which still takes them on trust (256 and
// 215 at W-209). Three answers per site, and their sum is every site:
//   FOUND         the named module declares the member (public or not — both
//                 counted, the private ones named);
//   NOT DECLARED  the module is in the store and declares no such member —
//                 REFUSED by name, the case the row said to measure first;
//   NOT IMPORTED  the using module has no `आयातः` of the named module (the
//                 resolver's own check; here the store's `आयातितम्`).
// A module the store does not hold at all is a fourth line, expected 0.
// The Rust twin (`Interpreter::declarations`, the loader's tables) is compared
// with the T1 store module by module: same names, kinds, types and runs.

/// The store's chain: the lexer, the AST, the parser and its text helper, and
/// the store — no resolver: the store copies what `व्याकर` parsed.
fn load_store() -> Interpreter {
    let names = [
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
    ];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &repo_root().join("spec"))
        .unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

/// Lex, parse and collect one source into the store; the entries added.
fn collect_into_store(it: &mut Interpreter, file: &str, src: &str) -> i128 {
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], FUEL_LEX)
        .unwrap_or_else(|e| panic!("{file}: पदविभाग runs: {e:?}"));
    it.call("व्याकरॱकार्यक्रमपठनम्", vec![toks], FUEL_PARSE)
        .unwrap_or_else(|e| panic!("{file}: कार्यक्रमपठनम् runs: {e:?}"));
    it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 2_000_000_000)
        .unwrap_or_else(|e| panic!("{file}: सङ्ग्रहः runs: {e:?}"))
        .as_int()
        .expect("a count")
}

fn store_text(it: &mut Interpreter, f: &str, args: Vec<Value>) -> String {
    match it
        .call(&format!("घोषणासञ्चयॱ{f}"), args, 50_000_000)
        .unwrap_or_else(|e| panic!("{f} runs: {e:?}"))
    {
        Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("{f}: octets, not {other:?}"),
    }
}

fn store_int(it: &mut Interpreter, f: &str, args: Vec<Value>) -> i128 {
    it.call(&format!("घोषणासञ्चयॱ{f}"), args, 50_000_000)
        .unwrap_or_else(|e| panic!("{f} runs: {e:?}"))
        .as_int()
        .unwrap_or_else(|| panic!("{f}: an integer"))
}

/// The T1 store's contents, read back as the Rust twin's records.
fn store_declarations(it: &mut Interpreter) -> Vec<ModuleDeclarations> {
    let modules = it
        .global("मण्डलसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the store's module count");
    let entries = it
        .global("प्रविष्टिसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the store's entry count");
    let mut out: Vec<ModuleDeclarations> = (1..=modules)
        .map(|m| ModuleDeclarations {
            name: store_text(it, "मण्डलनाम", vec![Value::Int(m)]),
            declarations: Vec::new(),
        })
        .collect();
    for i in 1..=entries {
        let m = usize::try_from(store_int(it, "प्रविष्टिमण्डलम्", vec![Value::Int(i)])).expect("fits");
        let kind = match store_int(it, "प्रविष्टिभेदः", vec![Value::Int(i)])
        {
            1 => DeclarationKind::Routine,
            2 => DeclarationKind::Struct,
            3 => DeclarationKind::Machine,
            4 => DeclarationKind::Global,
            5 => DeclarationKind::Import,
            6 => DeclarationKind::Enum,
            7 => DeclarationKind::Variant,
            k => panic!("entry {i}: kind {k} is none of the seven"),
        };
        let n = store_int(it, "प्रविष्टिप्राचलसंख्या", vec![Value::Int(i)]);
        let members = (0..n)
            .map(|k| {
                (
                    store_text(it, "प्राचलप्रविष्टिनाम", vec![Value::Int(i), Value::Int(k)]),
                    store_text(it, "प्राचलप्रविष्टिप्रकारः", vec![Value::Int(i), Value::Int(k)]),
                )
            })
            .collect();
        let public = matches!(
            it.call(
                "घोषणासञ्चयॱप्रविष्टिसार्वजनिकत्वम्",
                vec![Value::Int(i)],
                10_000_000
            ),
            Ok(Value::Bool(true))
        );
        out[m - 1].declarations.push(Declaration {
            kind,
            name: store_text(it, "प्रविष्टिनाम", vec![Value::Int(i)]),
            ty: store_text(it, "प्रविष्टिप्रकारः", vec![Value::Int(i)]),
            members,
            public,
        });
    }
    out
}

/// How the store answers one qualified `(module, member)` asked from `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Answer {
    Found,
    FoundPrivate,
    NotDeclared,
    NotImported,
    ModuleUnknown,
}

fn ask(it: &mut Interpreter, from: &str, module: &str, member: &str) -> Answer {
    let imported = matches!(
        it.call(
            "घोषणासञ्चयॱआयातितम्",
            vec![octets(from), octets(module)],
            50_000_000
        ),
        Ok(Value::Bool(true))
    );
    // A module's own qualified name (`वास्तुॱवाक्य` from inside वास्तु) needs no import.
    if !imported && from != module {
        return Answer::NotImported;
    }
    let hit = store_int(it, "सदस्यान्वेषणम्", vec![octets(module), octets(member)]);
    if hit > 0 {
        let public = matches!(
            it.call(
                "घोषणासञ्चयॱप्रविष्टिसार्वजनिकत्वम्",
                vec![Value::Int(hit)],
                10_000_000
            ),
            Ok(Value::Bool(true))
        );
        return if public {
            Answer::Found
        } else {
            Answer::FoundPrivate
        };
    }
    let kind = it.global("सञ्चयदोषभेद").and_then(Value::as_int).unwrap_or(0);
    // The record keeps the FIRST refusal; clear it so the next site records its own.
    it.call("घोषणासञ्चयॱनिषेधशुद्धिः", vec![], 1_000_000)
        .expect("निषेधशुद्धिः runs");
    if kind == 1 {
        Answer::ModuleUnknown
    } else {
        Answer::NotDeclared
    }
}

/// `W-247`: every name a module declares more than once, by kind — an import is
/// not a declaration. The same name in two MODULES is not a duplicate (module
/// scope, `W-224`).
fn duplicate_declarations(store: &[ModuleDeclarations]) -> Vec<String> {
    let mut out = Vec::new();
    for s in store {
        let mut seen: BTreeMap<(DeclarationKind, &str), usize> = BTreeMap::new();
        for d in &s.declarations {
            if d.kind == DeclarationKind::Import {
                continue;
            }
            *seen.entry((d.kind, d.name.as_str())).or_insert(0) += 1;
        }
        for ((k, n), c) in seen {
            if c > 1 {
                out.push(format!("{}: {k:?} `{n}` declared {c} times", s.name));
            }
        }
    }
    out
}

/// THE RATCHET (`W-247`), over the corpus, at ZERO: no module declares a name
/// twice. `W-223`'s store found ONE — `encode.t1`'s `संरचना प्रतीक्षा`, five
/// fields at line 938 and two at 978, the parser accepting both and the
/// loader keeping the last — and W-247 removed the one nothing built or read.
/// REFUSED, in the same pass: a synthetic module declaring one name twice is
/// COUNTED (the store keeps both, and names the module, the kind and the
/// count), and two modules each declaring the same name are NOT (module
/// scope, `W-224`). One interpreter pass over the corpus;
/// `tools/check-t1-ratchets.sh` runs it on the hourly deep gate.
#[test]
#[ignore = "ratchet: one interpreter pass over the corpus; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn no_module_in_the_corpus_declares_a_name_twice_and_a_synthetic_one_is_counted() {
    let files = corpus_files();
    let mut st = load_store();
    st.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000)
        .expect("आरम्भः runs");
    for f in &files {
        collect_into_store(&mut st, f, &source(f));
    }
    let duplicates = duplicate_declarations(&store_declarations(&mut st));
    println!(
        "METRIC paradigm_store_duplicate_declarations {}",
        duplicates.len()
    );
    for d in &duplicates {
        println!("  DUPLICATE {d}");
    }
    assert_eq!(
        duplicates,
        Vec::<String>::new(),
        "a module declares a name twice — make the corpus say which is meant"
    );

    // REFUSED: one module, one name, twice — counted by module, kind and count.
    st.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000)
        .expect("आरम्भः runs");
    collect_into_store(
        &mut st,
        "twice.t1",
        "मण्डलम् परीक्षा ॥\nसार्वजनिक संरचना क आरभ्य अ ॱॱ अ६४ समाप्तम् ।\nसार्वजनिक संरचना क आरभ्य ब ॱॱ न६४ समाप्तम् ।\n",
    );
    let twice = duplicate_declarations(&store_declarations(&mut st));
    assert_eq!(
        twice,
        vec!["परीक्षा: Struct `क` declared 2 times".to_string()]
    );
    println!("REFUSED {}", twice[0]);
    // NOT refused: the same name in two modules is two declarations (W-224).
    st.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000)
        .expect("आरम्भः runs");
    collect_into_store(
        &mut st,
        "one.t1",
        "मण्डलम् एक ॥\nसार्वजनिक संरचना क आरभ्य अ ॱॱ अ६४ समाप्तम् ।\n",
    );
    collect_into_store(
        &mut st,
        "two.t1",
        "मण्डलम् द्वि ॥\nसार्वजनिक संरचना क आरभ्य ब ॱॱ न६४ समाप्तम् ।\n",
    );
    assert_eq!(
        duplicate_declarations(&store_declarations(&mut st)),
        Vec::<String>::new(),
        "two modules may each declare क"
    );
}

/// `W-247`: lex, parse and resolve one program; the resolver's verdict and its
/// redeclaration record `(flag, name, first line, second line)`.
fn resolve_program(file: &str, src: &str) -> (bool, bool, String, i128, i128) {
    let mut it = load_chain();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], FUEL_LEX)
        .unwrap_or_else(|e| panic!("{file}: पदविभाग runs: {e:?}"));
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![toks], FUEL_PARSE)
        .unwrap_or_else(|e| panic!("{file}: कार्यक्रमपठनम् runs: {e:?}"));
    assert_eq!(
        it.global("दोषसूचकाङ्क").and_then(Value::as_int),
        Some(0),
        "{file} parses"
    );
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    let verdict = it
        .call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], FUEL_RESOLVE)
        .unwrap_or_else(|e| panic!("{file}: कार्यक्रमनिर्णयः runs: {e:?}"));
    let flag = matches!(it.global("पुनर्घोषणामस्ति"), Some(Value::Bool(true)));
    let name = match it.global("पुनर्घोषणानाम") {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        _ => String::new(),
    };
    let first = it
        .global("पूर्वघोषणापङ्क्ति")
        .and_then(Value::as_int)
        .unwrap_or(-1);
    let second = it
        .global("पुनर्घोषणापङ्क्ति")
        .and_then(Value::as_int)
        .unwrap_or(-1);
    (
        matches!(verdict, Value::Bool(true)),
        flag,
        name,
        first,
        second,
    )
}

/// `W-247`: A NAME DECLARED TWICE IN ONE MODULE IS REFUSED BY NAME, WITH BOTH
/// LINES. The CONTROL first: two modules each declaring `क` — one importing the
/// other — resolve, because a module's scope is its own (`W-224`) and an import
/// binds no bare name. Then the refusal: `संरचना क` twice names lines 2 and 4;
/// a `संरचना` and a `वृत्तिः` of one name are a redeclaration too (the scope
/// holds NAMES, not kinds — Rust's `resolve.rs` refuses the same); only the
/// FIRST duplicate is recorded; and the reset clears the record.
#[test]
fn a_name_declared_twice_in_one_module_is_refused_by_name_with_both_lines() {
    // CONTROL: the same name in two modules is two declarations.
    let (ok, flag, _, _, _) = resolve_program(
        "one.t1",
        "मण्डलम् एक ॥\nआयातः द्वि ।\nसार्वजनिक संरचना क आरभ्य अ ॱॱ अ६४ समाप्तम् ।\n",
    );
    assert!(ok && !flag, "एक declares क once; द्वि's क is द्वि's business");
    let (ok, flag, _, _, _) = resolve_program(
        "two.t1",
        "मण्डलम् द्वि ॥\nसार्वजनिक संरचना क आरभ्य ब ॱॱ न६४ समाप्तम् ।\n",
    );
    assert!(ok && !flag);

    // REFUSED: one module, `संरचना क` at line 2 and again at line 4.
    let (ok, flag, name, first, second) = resolve_program(
        "twice.t1",
        "मण्डलम् परीक्षा ॥\nसार्वजनिक संरचना क आरभ्य अ ॱॱ अ६४ समाप्तम् ।\nसार्वजनिक चरः ग ॱॱ न६४ भवति ० ।\nसार्वजनिक संरचना क आरभ्य ब ॱॱ न६४ समाप्तम् ।\n",
    );
    assert!(!ok, "the resolver answers असत्यम् on a redeclaration");
    assert!(flag);
    assert_eq!(name, "क");
    assert_eq!(
        (first, second),
        (2, 4),
        "both lines, the first declaration's first"
    );
    println!("REFUSED redeclaration of `{name}`: first at line {first}, again at line {second}");

    // REFUSED: a struct and a routine of one name — the scope holds names.
    let (ok, flag, name, first, second) = resolve_program(
        "kinds.t1",
        "मण्डलम् परीक्षा ॥\nसार्वजनिक संरचना क आरभ्य अ ॱॱ अ६४ समाप्तम् ।\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n",
    );
    assert!(!ok && flag);
    assert_eq!((name.as_str(), first, second), ("क", 2, 3));

    // The FIRST duplicate only, and the reset clears it.
    let (ok, flag, name, first, second) = resolve_program(
        "three.t1",
        "मण्डलम् परीक्षा ॥\nसार्वजनिक चरः क ॱॱ न६४ भवति ० ।\nसार्वजनिक चरः ख ॱॱ न६४ भवति ० ।\nसार्वजनिक चरः क ॱॱ न६४ भवति १ ।\nसार्वजनिक चरः ख ॱॱ न६४ भवति १ ।\n",
    );
    assert!(!ok && flag);
    assert_eq!(
        (name.as_str(), first, second),
        ("क", 2, 4),
        "क, not ख: the first"
    );
    let (ok, flag, _, first, second) = resolve_program(
        "clean.t1",
        "मण्डलम् परीक्षा ॥\nसार्वजनिक चरः क ॱॱ न६४ भवति ० ।\n",
    );
    assert!(
        ok && !flag && first == 0 && second == 0,
        "a fresh resolver carries nothing over"
    );
}

/// THE CENSUS (`W-223` part 1): the store over all 18 sources, its Rust twin,
/// and every qualified use and type position looked up — three answers whose
/// sum is every site, the refused ones named; and what the store says of the
/// two calls the IR builder refuses (W-237: samyojana.t1:870, vishlesana.t1:615).
#[test]
#[ignore = "measurement"]
fn measure_corpus_qualified_uses() {
    let files = corpus_files();
    // THE STORE, from every source's parse.
    let mut st = load_store();
    st.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000)
        .expect("आरम्भः runs");
    let mut added = 0;
    for f in &files {
        added += collect_into_store(&mut st, f, &source(f));
    }
    let modules = st
        .global("मण्डलसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the store's module count");
    println!("METRIC paradigm_store_modules {modules}");
    println!("METRIC paradigm_store_entries {added}");
    // THE TWIN: the loader's tables over the same sources.
    let texts: Vec<(String, String)> = files.iter().map(|f| (f.clone(), source(f))).collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let rust = Interpreter::load(&refs, &repo_root().join("spec"))
        .unwrap_or_else(|e| panic!("the loader takes the corpus: {e:?}"))
        .declarations();
    let t1 = store_declarations(&mut st);
    let mut disagree: Vec<String> = Vec::new();
    let mut agreeing = 0;
    for m in &rust {
        if m.declarations.is_empty() {
            continue; // lib.t1: no module name, nothing declared, nothing the store keys
        }
        let Some(s) = t1.iter().find(|s| s.name == m.name) else {
            disagree.push(format!("{}: in the loader, not in the store", m.name));
            continue;
        };
        let a: BTreeSet<&Declaration> = m.declarations.iter().collect();
        let b: BTreeSet<&Declaration> = s.declarations.iter().collect();
        if a == b {
            agreeing += 1;
        } else {
            for d in a.difference(&b) {
                disagree.push(format!(
                    "{}: loader only: {:?} {} `{}` {:?} public={}",
                    m.name, d.kind, d.name, d.ty, d.members, d.public
                ));
            }
            for d in b.difference(&a) {
                disagree.push(format!(
                    "{}: store only: {:?} {} `{}` {:?} public={}",
                    m.name, d.kind, d.name, d.ty, d.members, d.public
                ));
            }
        }
    }
    for s in &t1 {
        if !rust.iter().any(|m| m.name == s.name) {
            disagree.push(format!("{}: in the store, not in the loader", s.name));
        }
    }
    let compared = rust.iter().filter(|m| !m.declarations.is_empty()).count();
    println!("METRIC paradigm_store_twin_agreement {agreeing}/{compared}");
    // A name declared twice in one module, by kind: the parser accepts it and
    // the store keeps both; the resolver binds one of them silently (`W-247`).
    let duplicates = duplicate_declarations(&t1);
    println!(
        "METRIC paradigm_store_duplicate_declarations {}",
        duplicates.len()
    );
    for d in &duplicates {
        println!("  DUPLICATE {d}");
    }
    for d in disagree.iter().take(60) {
        println!("  DISAGREE {d}");
    }
    // EVERY QUALIFIED SITE, asked.
    let mut counts: BTreeMap<Answer, usize> = BTreeMap::new();
    let mut named: Vec<String> = Vec::new();
    let (mut uses, mut types) = (0usize, 0usize);
    for f in &files {
        let p = read_program(f, &source(f));
        let mut w = Walk::run(&p);
        let tps = type_positions(&p);
        w.type_references(&p, &tps);
        let from = p.module.clone().unwrap_or_default();
        let mut sites: Vec<(String, i128, &str)> = w
            .uses
            .iter()
            .filter(|u| u.qualified)
            .map(|u| (u.name.clone(), u.line, "use"))
            .collect();
        uses += sites.len();
        for (n, l) in w.type_qualified.iter().zip(&w.type_qualified_lines) {
            sites.push((n.clone(), *l, "type"));
        }
        types += w.type_qualified.len();
        for (name, line, what) in sites {
            let Some((module, member)) = module_prefix(&name) else {
                continue;
            };
            let a = ask(&mut st, &from, module, member);
            *counts.entry(a).or_insert(0) += 1;
            if a != Answer::Found {
                named.push(format!("{f}:{line} {what} `{name}` from {from}: {a:?}"));
            }
        }
    }
    println!("METRIC paradigm_name_uses_qualified {uses}");
    println!("METRIC paradigm_name_types_qualified {types}");
    let n = |a: Answer| counts.get(&a).copied().unwrap_or(0);
    println!(
        "METRIC paradigm_name_qualified_found {}",
        n(Answer::Found) + n(Answer::FoundPrivate)
    );
    println!(
        "METRIC paradigm_name_qualified_found_private {}",
        n(Answer::FoundPrivate)
    );
    println!(
        "METRIC paradigm_name_qualified_not_declared {}",
        n(Answer::NotDeclared)
    );
    println!(
        "METRIC paradigm_name_qualified_not_imported {}",
        n(Answer::NotImported)
    );
    println!(
        "METRIC paradigm_name_qualified_module_unknown {}",
        n(Answer::ModuleUnknown)
    );
    for s in &named {
        println!("  SITE {s}");
    }
    // The two calls the IR builder refuses (W-237): what the store answers.
    for (from, module, member, site) in [
        ("संयोजन", "सङ्केतन", "दशाङ्कमूल्यम्", "samyojana.t1:870"),
        ("विश्लेषण", "सङ्केतन", "अष्टकान्वेषणम्", "vishlesana.t1:615"),
    ] {
        let a = ask(&mut st, from, module, member);
        println!("  IR-REFUSED {site} `{module}ॱ{member}` from {from}: {a:?}");
    }
    // ASSERTED: every site answered, the twins agree, and the store holds every module.
    let answered: usize = counts.values().sum();
    assert_eq!(
        answered,
        uses + types,
        "every qualified site has one of the answers"
    );
    // 18 -> 19 over 20 -> 21 files on 2026-09-17, ONE FILE AND ONE NAME, BOTH
    // MEASURED. `014794dc` added `sarani.t1` — the embed store generated by
    // `tools/mkspectables.py` under `SAS-014` — and `29b1169a` settled its module
    // name as `समावेशसारणी`. It is the only file in `crates/sadhana-t1/src/*.t1`
    // whose declared name appears nowhere else, so it is the whole of both rises:
    // 21 files, 19 distinct names, `वास्तु` still the one name over two files and
    // `lib.t1` still naming none. A generated source is still a source, which is
    // the same reading that put `sarani.t1` in this file's forward-reference
    // ceiling table. RE-MEASURED on this tree at `b4ba8b43` before re-taking, not
    // carried over from the recovery: 21 files, 19 names, `lib.t1` the only one
    // declaring none. RE-MEASURED ONCE MORE AT `b9b9b7de` (2026-09-19) BEFORE
    // LANDING, from the sources and not from this file: 21 `.t1` files, 20
    // `मण्डलम्` declarations, `वास्तु` twice — 19 distinct. So the stash's `18`
    // was RED on this tree and `19` is the taking, not a carry-over.
    assert_eq!(
        modules, 19,
        "19 module names over 21 files (वास्तु is two files, lib.t1 names none)"
    );
    assert!(
        disagree.is_empty(),
        "the T1 store and the loader disagree on {} declaration(s):\n  {}",
        disagree.len(),
        disagree.join("\n  ")
    );
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Routine => "routines",
        Kind::Type => "types",
        Kind::Machine => "machines",
        Kind::Global => "globals",
        Kind::Import => "imports",
        Kind::Param => "params",
        Kind::Local => "locals",
    }
}

// ── the tests, with the routine ─────────────────────────────────────────

/// A program with every shape the census distinguishes: a parameter used and
/// one not, a local, a forward call, a use inside a `यदि` body, a use inside a
/// `यावत्` body, a qualified use through an import, and a name inside an
/// `अन्यथा` body that is DECLARED NOWHERE.
///
/// THE ELSE BODY IS THE REFUSED CASE, in the census's own terms: the resolver
/// must accept the program — proving it never looked at `अघोषितम्` — and the
/// census must count that node as an orphan rather than as a resolved use or
/// as nothing.
const SHAPES: &str = "मण्डलम् क ॥\n\
आयातः ख ।\n\
सार्वजनिक वृत्तिः साधारणम् आदाय अ ॱॱ अ६४ ऽ निष्फलम् ॱॱ अ६४ ददाति अ६४ आदि\n\
    चरः फलम् ॱॱ अ६४ भवति द्वितीयम् अ ।\n\
    यदि फलम् अधिकम् ० आदि\n\
        फलम् भवति फलम् योगः अ ।\n\
    इति अन्यथा आदि\n\
        फलम् भवति अघोषितम् ।\n\
    इति\n\
    यावत् फलम् न्यूनम् ९ आदि\n\
        फलम् भवति फलम् योगः १ ।\n\
    इति\n\
    प्रत्यागमनम् खॱगणना फलम् ।\n\
इति\n\
वृत्तिः द्वितीयम् आदाय ग ॱॱ अ६४ ददाति अ६४ आदि\n\
    प्रत्यागमनम् ग ।\n\
इति\n";

#[test]
fn the_replica_walk_agrees_with_the_resolver_and_counts_what_it_skips() {
    let p = read_program("shapes", SHAPES);
    assert_eq!(p.parse_errors, 0, "the fixture must parse: {:?}", p.refusal);
    assert_eq!(
        p.decls.iter().flatten().count(),
        3,
        "an import and two routines"
    );
    // THIS READ `p.resolved`, and its message said "that acceptance is the
    // proof the else body is never walked". It was true, and it was a defect
    // MEASURED rather than fixed: `अघोषितम्` is declared nowhere and sits in the
    // `अन्यथा` body, and the resolver said yes. `W-202` made the resolver read
    // `अन्यसूचकाङ्क`, so it says no.
    //
    // THE NAME **AND** THE LINE: `अघोषितम्` alone would also be satisfied by a
    // resolver that reached it some other way; line 8 is the `अन्यथा` body's own
    // line in `SHAPES`, so this pins the refusal to the branch the walk now
    // enters rather than to the name appearing anywhere.
    assert!(
        !p.resolved,
        "an undeclared name in an `अन्यथा` body must now be REFUSED: {:?}",
        p.refusal
    );
    let refusal = p.refusal.clone().unwrap_or_default();
    assert!(
        refusal.contains("अघोषितम्") && refusal.contains("line 8"),
        "the refusal must name the else body's undeclared name AND its line: {:?}",
        p.refusal
    );
    let w = Walk::run(&p);
    let places = place_nodes(&p, &w, &type_positions(&p));

    // Every bare use the walk reached carries the replica's symbol + १ — and
    // since 2026-09-13 the replica's SYMBOL is its position + १ as well, so the
    // node carries position + २. `c107c725` states the convention it created:
    // "The tree and every table index by id + १ as before; slot १ is now
    // unused." Ids mint from १ because an optional is ONE WORD natively and
    // `Some(०)` was the nil word, so zero had to stop being a valid id; the
    // tree's own `+ १`, which exists so that ० can mean "no symbol", still
    // applies on top. Two offsets that compose, both deliberate, neither
    // re-pinned here at the time — this is the re-take, and `u.replica` is a
    // POSITION in the replica list, which is what makes the second `+ 1`
    // visible at all.
    for u in &w.uses {
        let expect = if u.qualified {
            0
        } else {
            u.replica.map_or(0, |s| i128::try_from(s).unwrap() + 2)
        };
        assert_eq!(
            u.node_symbol, expect,
            "use `{}` at line {}: node carries {} and the replica says {:?}",
            u.name, u.line, u.node_symbol, u.replica
        );
    }
    // What the walk reached: `द्वितीयम् अ` (2), `यदि फलम्` (1), then-body (3),
    // ELSE-BODY (2 — `फलम्` and `अघोषितम्`, W-202), `यावत् फलम्` (1),
    // while-body (2), `खॱगणना फलम्` (2), `ग` (1) = 14.
    assert_eq!(places.walked, 14, "walked uses");
    // WAS 2 — "the else body's `फलम्` and `अघोषितम्` sit in the arena and
    // nothing points at them". Something points at them now.
    assert_eq!(places.orphan, 0, "the else body is no longer orphaned");
    assert_eq!(places.orphan_resolved, 0, "an orphan is never written to");
    // `अ६४` six times as a type: three parameters (`अ`, `निष्फलम्`, `ग`), two
    // returns, one `चरः`. None is a use, and each is a kind-१ node.
    assert_eq!(places.type_position, 6, "type-position nodes are not uses");

    let q: Vec<&Use> = w.uses.iter().filter(|u| u.qualified).collect();
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].node_symbol, 0, "a qualified use leaves the node at ०");
    let import = q[0].replica.expect("the prefix resolves to the import");
    assert_eq!(w.syms[import].kind, Kind::Import);
    assert_eq!(
        w.syms[import].uses, 1,
        "the qualified use counts for the import"
    );

    let sym = |name: &str| w.syms.iter().find(|s| s.name == name).expect(name);
    assert_eq!(
        sym("फलम्").uses,
        8,
        "in-degree of the local — was 7 before the else body was walked"
    );
    assert_eq!(sym("अ").uses, 2, "in-degree of the read parameter");
    assert_eq!(sym("निष्फलम्").uses, 0, "the dead parameter");
    assert_eq!(sym("द्वितीयम्").uses, 1);
    assert_eq!(sym("साधारणम्").uses, 0, "public and unused in its own file");

    let in_cond: Vec<&Use> = w.uses.iter().filter(|u| u.cond_depth > 0).collect();
    assert_eq!(
        in_cond.len(),
        7,
        "three in the यदि body, TWO IN THE अन्यथा BODY (W-202), two in the यावत् body"
    );
    // ONE DELIBERATE EXCEPTION: `अघोषितम्` is declared nowhere — the point of
    // the fixture — so its node keeps ०. A blanket `all(!= 0)` would now fail
    // for the right reason while hiding the wrong one.
    assert!(
        in_cond
            .iter()
            .all(|u| u.node_symbol != 0 || u.name == "अघोषितम्"),
        "then-, else- and while-bodies ARE entered and their uses resolved"
    );
    assert_eq!(
        in_cond
            .iter()
            .filter(|u| u.node_symbol == 0)
            .map(|u| u.name.as_str())
            .collect::<Vec<_>>(),
        vec!["अघोषितम्"],
        "exactly one conditional-body use is unresolved, and it is the one the \
         fixture declares nowhere"
    );
    assert_eq!(
        w.uses.iter().filter(|u| u.site == Site::Condition).count(),
        2
    );

    // The forward reference: `साधारणम्` (slot 2) calls `द्वितीयम्` (slot 3).
    let call = w.uses.iter().find(|u| u.name == "द्वितीयम्").unwrap();
    let target = &w.syms[call.replica.unwrap()];
    assert!(
        call.routine < target.decl,
        "a use before its declaration in the global scope"
    );
    // The local's distance: declared at position 0 of the body, used from
    // positions 1, 2 and 3 of the same block.
    let local = sym("फलम्");
    let positions: Vec<usize> = w
        .uses
        .iter()
        .filter(|u| u.replica == Some(w.syms.iter().position(|s| s.name == "फलम्").unwrap()))
        .filter_map(|u| {
            u.chain
                .iter()
                .find(|(b, _)| *b == local.block)
                .map(|(_, p)| *p)
        })
        .collect();
    // ONE MORE `1` THAN BEFORE `W-202`: the `अन्यथा` body's own `फलम्` use, at
    // the same chain position as the then-body's — both halves of a `यदि` are
    // one step inside the block that declares the local.
    assert_eq!(positions, vec![1, 1, 1, 1, 2, 2, 2, 3]);
}

// ── W-222: hoisting, stated and ratcheted ───────────────────────────────

/// A bare use of a name that a `चरः` LATER in one of the use's enclosing
/// blocks declares — the use is before that local's declaration, or inside
/// its own initializer. Two kinds, told apart by what the use resolved to:
/// `outward` when an enclosing scope bound the name (a parameter, a module
/// name — a legal shadow-to-come), `unresolved` when nothing did, which is
/// what the resolver REFUSES.
///
/// `W-209`'s `paradigm_name_local_uses_before_declaration` counted a use
/// whose replica symbol is a local declared later in the same block. That is
/// ZERO BY CONSTRUCTION: the replica declares a local in written order, after
/// its initializer, exactly as the resolver does, so no use can resolve to a
/// local not yet declared. This detector looks at the block instead of the
/// symbol, and the refused case below shows it seeing what the resolver
/// refuses.
#[derive(Debug, Default, PartialEq, Eq)]
struct BeforeLet {
    unresolved: Vec<String>,
    outward: Vec<String>,
}

fn uses_before_their_let(prog: &Program, w: &Walk) -> BeforeLet {
    let mut out = BeforeLet::default();
    for u in w.uses.iter().filter(|u| !u.qualified) {
        let later_let = u.chain.iter().any(|&(block, pos)| {
            let Some(b) = prog.stmt(block) else {
                return false;
            };
            Walk::children(prog, b)
                .into_iter()
                .enumerate()
                .any(|(q, c)| {
                    q >= pos
                        && prog
                            .stmt(c)
                            .is_some_and(|s| s.kind == STMT_LET && prog.tok_text(s.left) == u.name)
                })
        });
        if !later_let {
            continue;
        }
        let site = format!("{}:{} {}", prog.file, u.line, u.name);
        if u.replica.is_some() {
            out.outward.push(site);
        } else {
            out.unresolved.push(site);
        }
    }
    out
}

/// Every bare use of a module-level name whose declaration slot comes AFTER
/// the slot of the routine using it: a forward reference the two-pass
/// resolver permits.
fn forward_references(prog: &Program, w: &Walk) -> Vec<String> {
    w.uses
        .iter()
        .filter(|u| !u.qualified)
        .filter_map(|u| {
            let s = &w.syms[u.replica?];
            (!matches!(s.kind, Kind::Local | Kind::Param) && u.routine < s.decl)
                .then(|| format!("{}:{} {}", prog.file, u.line, u.name))
        })
        .collect()
}

/// Forward references per file, as `W-209` measured them on 2026-09-03 and
/// `spec/grammar-t1.ebnf`'s scope note states them. A CEILING: the count may
/// fall — a source may be reordered — and a rise means a rule this file
/// pins has changed, or the instrument has.
const FORWARD_REFERENCES_CEILING: [(&str, usize); 12] = [
    // 25 -> 37 on 2026-09-04: `W-202`'s else-walk (ee0251d7) added twelve forward
    // references in `artha.t1` and landed without raising this pin (the ratchet is
    // `#[ignore]`d and the peer's gate did not run the light gate); found RED on
    // main by `W-239`'s gate and raised here by the trunk at W-239's merge, with
    // the sites printed by the test. Permitted: top-level names are order-free.
    // 37 -> 75 on 2026-09-04, and the two causes are measured apart because they
    // are not one row's: `W-223` part 2 (314eb51b, 343 new lines in `artha.t1` —
    // the store-facing state, its five kind constants and the qualified arm)
    // took it 37 -> 66 AND LANDED WITHOUT RAISING THIS PIN, the same way W-202
    // did before it and for the same reason (this ratchet is `#[ignore]`d, so a
    // plain `cargo test -p sadhana-t1` does not run it and a crate gate reports
    // green); measured here by checking out main's own `artha.t1` and running
    // this test on it. `W-247` takes it 66 -> 75: the redeclaration record's
    // four globals are declared beside `अनिर्णीतनाम` near the file's foot and
    // written in `निर्णायकारम्भः` and in pass one, both above it. Permitted, as
    // the note above says — top-level names are order-free — and said out loud.
    // AND THE CONTROL, FROM THE OTHER LANE that measured this independently:
    // every other file in this table returned its frozen value EXACTLY on the
    // same run — encode 9, parse 9, unparse 1, vakyavibhaga 10, sanskrit_text 3,
    // utsarjana 2, samyojana 1, and `ir.t1` 1, a file that lane rewrote heavily
    // and which added no forward reference at all. A pin that only ever moves
    // for the file nobody is editing is the signature of an `#[ignore]`d ratchet
    // rather than of a regression.
    // 75 -> 80 on 2026-09-05, MEASURED from this ratchet's own failing
    // assertion, never computed. W-248 added a text-to-type reader whose
    // routines reference module constants declared below them, and the trunk
    // hoisted `विवराष्टकम्` out of a routine body to module scope — `विवरम्` is
    // ADR-0018's word for U+0020 and lexes as a layout WORD, so it resolves at
    // module scope and is refused inside a body. Both are uses before their
    // declaration, which this file permits and asks to be said aloud.
    // 80 -> 84 on 2026-09-05, TAKEN FROM THIS RATCHET'S OWN FAILING ASSERTION
    // ("artha.t1: 84 forward references, ceiling 80") and never computed.
    // W-265 gave a named type a symbol, and the four new forward references are
    // that repair's shape rather than an accident of layout: the type-name
    // table, its five refusal-reason constants and its two counters are
    // declared beside `अज्ञातप्रकारपाठकोश` near the head of the file, while
    // `नामप्रकारार्थः` and `प्रकारसंज्ञायोजनम्` — the routines that read and
    // write them — sit with the readers they serve; and `सञ्चयप्रकारबन्धः`,
    // called at the head of `कार्यक्रमनिर्णयः`, is written beside
    // `सञ्चयसंज्ञा`, the minter it must not duplicate. Each is a use before its
    // declaration, which top-level order-freedom permits and this pin asks to
    // be said aloud.
    // 84 -> 91 on 2026-09-07. A DEBT FROM `123da7be`, NOT ROOM FOR NEW WORK — the
    // bad-field refusal landed 109 lines into `artha.t1` and never raised this pin,
    // because THE GATE THAT PASSED IT NEVER RAN THIS RATCHET. That run scoped to a
    // dirty tree whose real changes were already committed, so its `== changed:`
    // list held three test and spec files and no `.t1` at all; the `.t1` ratchet
    // block did not execute and the run ended GREEN in 682s without this test
    // appearing in it. THE VERDICT WAS TRUE OF THE SCOPE IT TOOK AND FALSE OF THE
    // CHANGE IT WAS ASKED ABOUT, and the trunk quoted it without reading the scope.
    //
    // The seven are that row's own shape: the `असत्क्षेत्र…` family's globals are
    // declared with the other refusal records at the head of the file, the arm that
    // writes them is far below, and the routine that reads them is below that.
    // Permitted — top-level names are order-free — and bisected rather than assumed:
    // the file PASSES at `5afe68b1` and reads 91 at `123da7be`, and reverting a
    // later row's three `.t1` files still reads 91, so none of it is inherited.
    //
    // THE RULE THIS COST: a gate run whose `== changed:` list does not contain the
    // files the row is about HAS NOT GATED THE ROW, whatever its verdict says. Read
    // the scope line before quoting the verdict. The gate has since been repaired to
    // scope by a branch's commits when the worktree is clean, which closes this
    // hole for a clean tree but not for the dirty-tree case that caused it.
    //
    // 91 -> 96, MEASURED FROM THIS RATCHET'S OWN FAILING ASSERTION ON A CLEAN
    // TREE AT `4ae9ea8c` and ATTRIBUTED BY `git blame` OVER THE 94 LINES THE RUN
    // NAMES: exactly five of them postdate `123da7be`, and those five are the
    // whole of the rise. `fee3dccd` (2026-09-13, the parser's native zero) owns
    // four — `:1279 संज्ञाघोषणाकोश`, `:1282 सञ्चयसंज्ञासूचकाङ्क`,
    // `:1283 सञ्चयसंज्ञामूल्यानि`, `:1285 सञ्चयसंज्ञाप्रविष्टयः` — the
    // remote-struct-through-the-registry arm reading the declaration and
    // accumulation stores that are declared near the file's foot. `1ec816297`
    // (2026-09-13, a run's दैर्घ्य types as a length) owns one,
    // `:1254 पूर्णाङ्कार्थः`. Permitted — top-level names are order-free — and
    // said aloud, which is the whole of what this pin asks for.
    //
    // IT WAS RED FOR FIVE DAYS AND THE HOURLY GATE SAID SO EVERY HOUR. The rise
    // landed 2026-09-13; `.loop/deep-gate.stdout.log` carries the same
    // assertion message 57 times and `deep-gate: FAILED` 49 times. This is the
    // SAME failure mode as the `84 -> 91` row below and not a new one: a ratchet
    // nobody runs on the landing path reds somewhere nobody looks.
    //
    // "Nothing read the log" was true when this margin was written and is no
    // longer, which is why the count above moved 48 -> 49 the moment anything
    // counted: `crates/metrics` now reads that file on every `metrics` run and
    // reports the NEWEST run's verdict. It reports this pin's own recovery —
    // runs 318-337 red, run 338 PASS — and a lifetime total is exactly what
    // could not say that. And note what BOTH numbers miss: run 317 carries no
    // verdict at all, so the 20-run red streak the reader names is bounded
    // above by a MUTE run, not by a green one. The not-green span really
    // reaches back to run 309 — 29 runs, 28 red and one the file cannot speak
    // for. Run 308 is the last true PASS before it.
    // 96 -> 119 on 2026-09-24. PERMITTED AND SAID SO, per this pin's own
    // message. TWO landings of 2026-09-23, both after the ceiling was taken at
    // `af7388a4` (2026-09-18), and named per site by `--nocapture`:
    //   19 from `bdd95b86` — the `शून्यम्`/`ऋण` typing arms and the variant
    //      ordinal. `शून्यार्थः` alone is 11 of them: the arm answers it from
    //      `अभिव्यञ्जकप्रकारः` (:1100s) and the `अर्थ` constructors sit in the
    //      trailing block, which is this file's shape and why the cap was 96.
    //    4 from `ee35b35c` — `अनिर्णीतधार*`, the field arm's four silent exits
    //      turned into counters. The recorder is declared with the other
    //      recorders at :2659 and called from the arm at :1353-:1383.
    // NOT a new convention: every one is a global declared in the trailing
    // block and used by a routine above it, which is what the existing 96
    // already counted.
    ("artha.t1", 119),
    // 9 -> 10 on 2026-09-07, the object builder. PERMITTED AND SAID SO, as this
    // pin's own message asks. `वस्तुसंज्ञासारणी` (:5676) calls
    // `संज्ञासूचकाङ्कः` (:5732) — the table builder asking whether a name is
    // already in the table it is filling, 56 lines forward.
    //
    // The order is the file's argument: the two routines that BUILD an object's
    // parts come before the small predicate they share, so a reader meets the
    // stages in the order the chain runs them. Moving the predicate up to save
    // the forward reference would put a helper before the thing it serves.
    //
    // 10 -> 15 on 2026-09-13, and it was HIDDEN BEHIND `artha.t1`: the old loop
    // asserted inside itself, so it died on the alphabetically first offender
    // and every file after it was reported as absence. Two commits own the five,
    // both 2026-09-13: `6dfb3026` (records cached per index row) owns
    // `:3449 सङ्केतसूचीक्रमः`, `:4999 सङ्कोचपङ्क्तिवत्`, `:5018 रूपयुग्मसाम्यम्`;
    // `d920a18b` (families grouped once) owns `:3333 कुलस्मरणम्` and
    // `:3339 सूचितांशसाम्यम्`. Each is a caching or grouping helper placed with
    // the stage it serves, above the predicate it shares. Permitted, and said.
    ("encode.t1", 15),
    // 7 -> 9 on 2026-09-04, MEASURED by this ratchet on the trunk's merged tree at W-239's
    // merge (W-215's printer arms, W-236's twin and W-239's wiring all landed on `parse.t1`'s
    // callers/callees the same night, each lane gating on its own base); the sites are printed
    // above. Permitted: top-level names are order-free.
    //
    // 9 -> 13, and four of the rise are older than the pin's own note rather
    // than newer: `44e21441`, `5ecbb244`, `8dc21efe`, `a2536895`, `a2e3765c`,
    // `84d20a04` and `b77df9dd` all predate 2026-09-05 and were already in the
    // file when 9 was written, so the 9 was a measurement of a subset. The one
    // unambiguously new site is `72117493` (2026-09-13, binary operators bracket
    // by level) at `:1100 द्विकर्मश्रेणिपठनम्`. `अभिव्यञ्जकपठनम्` — the recursive
    // expression reader — accounts for eight of the thirteen and is a genuine
    // self-recursion through a forward name, not layout drift.
    ("parse.t1", 13),
    // `W-204` (2026-09-04): `अभिव्यञ्जकरचना`'s call arm reaches `आह्वानादानरचना`,
    // written after it because it is the arm's helper; a mutual recursion, so
    // one direction is forward whichever comes first. W-204 landed between
    // this ratchet's landing and its first run over the merged tree — found
    // by `W-215`'s light gate, pinned here with its cause.
    //
    // 1 -> 27 on 2026-09-13/14, THE LARGEST RISE IN THIS TABLE AND THE ONE MOST
    // WORTH NAMING: `ee0d9996` (runs grow, compare by content, slice, and store
    // octets natively) owns 24 and `4461732d` (an index read of a nil run
    // answers ०) owns 2, on top of `17ad11d4`'s single `:2408 आह्वानादानरचना`.
    // Eighteen of the 24 are calls to ONE name, `पर्वस्थापनम्` — the step
    // appender — declared below the arms that append through it. That is the
    // shape of a helper written once and used by every arm above it, which is
    // exactly what top-level order-freedom is for; it is not 24 separate debts.
    ("ir.t1", 27),
    // `W-215`: the printer's expression arm calls `आह्वानादानलेखनम्`, which
    // calls it back for each argument — a mutual recursion, so one direction
    // is forward whichever is written first. Every other helper is written
    // above its caller (measured 3 before `प्राचलपङ्क्तिलेखनम्` was moved
    // above `घोषणालेखनम्`, 1 after).
    ("unparse.t1", 1),
    // 4 -> 10 on 2026-09-04, MEASURED by this ratchet on the trunk's merged tree at W-239's
    // merge (W-215's printer arms, W-236's twin and W-239's wiring all landed on `vakyavibhaga.t1`'s
    // callers/callees the same night, each lane gating on its own base); the sites are printed
    // above. Permitted: top-level names are order-free.
    // 10 -> 11 on 2026-09-07, MEASURED by this ratchet on the trunk's integration
    // and INDEPENDENTLY by the lane on its own tree. The new site is
    // `वाक्यविभाजनम्` (2047) calling `वाक्यपठनम्` (2958) — the statement splitter
    // calling the statement reader, 900 lines forward. THAT CALL IS d92229c2's
    // WHOLE POINT: before it, the splitter recorded statements into `वाक्यकोश`
    // and nothing turned them into instructions, so the assembler answered a
    // plausible number — four, for four statements — and built nothing.
    // Permitted: top-level names are order-free, and the rise is the join.
    //
    // d92229c2 SHOULD HAVE RAISED THIS AND DID NOT. Its message lists five
    // re-taken figures; this was a sixth, and the gate that would have shown it
    // was read through `| tail -25`, which discarded the evidence for every red
    // step and let the pipe answer for the exit code. The ratchet was never
    // silent — its reader was.
    //
    // TWO TREES FOUND IT, and that is the part worth keeping: the lane's tree
    // carries the driver, the trunk's integration does not, so the two runs
    // differ in the ONE variable that could have explained the rise. The same
    // verdict from both rules out the driver. Neither operator built that
    // control; it fell out of the merge order.
    ("vakyavibhaga.t1", 11),
    ("sanskrit_text.t1", 3),
    ("utsarjana.t1", 2),
    // 1 -> 3 on 2026-09-24. PERMITTED AND SAID SO. The pinned 1 is
    // `स्थानसंयोजनम्` (:1637). The two new ones are `वस्तुसंख्या` at :200 and
    // :222, from `64d2da01`: `दत्तारम्भः` and `बीजारम्भः` each ask the object
    // count to tell a START query from a LENGTH query, and `वस्तुसंख्या` is
    // declared at :1015 with the other readers. Two sites, one name, one
    // commit — the whole rise.
    ("samyojana.t1", 3),
    // ── THE THREE THAT HAD NO ROW AT ALL ─────────────────────────────────────
    //
    // These joined the corpus after this table was written and `ceiling.get(f)
    // .unwrap_or(0)` reported each as "over a ceiling of 0" — a number nobody
    // chose, rendered as a decision. They are pinned here at their measured
    // value, and `Standing::Unpinned` now refuses the NEXT such file by that
    // word instead of inventing a zero for it.
    //
    // 16, all of them from `014794dc` (2026-09-14, the embed store as a `.t1`
    // module). `सारणीखण्ड०` … `सारणीखण्ड१५` are the sixteen table segments,
    // declared at the file's foot and named by the assembler above them — one
    // reference each, which is the whole of the count.
    ("sarani.t1", 16),
    // 13. TWELVE from `239eac74` and `4e8020fe` (both 2026-09-13): `रुङ्गचयनम्`
    // — the colour/segment picker — takes five of them and the three सङ्केत
    // names the rest; all sit in the layout-word arm added that day, at
    // `shrinkhala.t1:1857-1871`.
    //
    // THE THIRTEENTH, RAISED 2026-09-22 AND SAID OUT LOUD BECAUSE THE RATCHET'S
    // OWN MESSAGE DEMANDS IT: `shrinkhala.t1:3495` calls `मण्डलानिप्रतिबिम्बम्`,
    // which is declared thirty-two lines below it at `:3527`. It arrived with
    // the fixpoint rung `स्वपरीक्षास्वप्रतिबिम्बम्` in `33b1b511` (2026-09-21) and
    // is a permitted use-before-declaration, not a defect — the corpus-walk top
    // level sits at the foot of the file by construction and the rung that
    // drives it sits above.
    //
    // **THE PIN WAS NOT RAISED IN THAT COMMIT BECAUSE THIS TEST IS `#[ignore]`
    // AND ONLY THE HOURLY DEEP GATE RUNS IT.** `33b1b511`'s gate line reads
    // "t1_paradigm_names 8", which is this file's count WITHOUT
    // `--include-ignored`, so the landing was green and the deep gate went red
    // eighty minutes later and stayed red — 383 hourly runs' newest, measured
    // 2026-09-22. A landing gate that cannot see a ratchet cannot be the thing
    // that keeps it.
    // 13 -> 16 on 2026-09-24. PERMITTED AND SAID SO, and note WHAT the margin
    // above predicted: a landing gate that cannot see a ratchet did not keep
    // it, and four units landed across 2026-09-22..24 without this being run.
    // The three, named per site:
    //   2 `प्रतिबिम्बविरामः` from `e034b803` — the stage instrument (status
    //     1201 + 100 x stage) that named which stage an image build reached
    //     while the native self-image was being brought up.
    //   1 `पाठमुद्रणार्हम्` from `e28c6642`.
    // Both are 2026-09-22, after the ceiling was taken at `af7388a4`
    // (2026-09-18). Neither is a new shape: a driver global declared below the
    // routine that reads it.
    ("shrinkhala.t1", 16),
    // 4, all `यन्त्रवैश्विकचिह्नम्` from `cd8e1877` (2026-09-17, the gate was one
    // missing label). FOUR SITES, ONE NAME: the global-symbol emitter is
    // declared below the four arms that emit through it.
    ("yantrotsarjana.t1", 4),
    // `yantrotsarjana.t1` (W-236, 2026-09-04) read ० here for thirteen days:
    // its three would-be forward references were reordered above their first
    // use, so the new file needed no pin. THAT SENTENCE IS NOW HISTORY AND IS
    // KEPT AS HISTORY — `cd8e1877` (2026-09-17) added four, the row is above,
    // and a margin that still said "needs no pin" would be the dated claim this
    // ratchet exists to catch.
];

/// Where one measured file stands against the pinned table.
///
/// **THREE STATES, BECAUSE THE READING IT REPLACES HAD TWO AND ONE OF THEM WAS
/// AN INVENTION.** `ceiling.get(f).copied().unwrap_or(0)` gave a file with no
/// row a cap of `0` and then reported it as `ceiling 0` — a number nobody
/// chose, printed in the same words as a number somebody did. Three `.t1`
/// modules joined the corpus after this table was written — `shrinkhala.t1`
/// (2026-09-13), `sarani.t1` (2026-09-14), `yantrotsarjana.t1` (2026-09-17) —
/// and every one of them read as "over a ceiling of 0". `Unpinned` is a
/// REFUSAL and says so by that word: *this table has no row for this file* is
/// a different fact from *this file grew*, and only the second is a ratchet
/// finding.
#[derive(Debug, PartialEq, Eq)]
enum Standing {
    Within,
    Over { n: usize, cap: usize },
    Unpinned { n: usize },
}

fn standing(n: usize, cap: Option<usize>) -> Standing {
    match cap {
        None => Standing::Unpinned { n },
        Some(cap) if n > cap => Standing::Over { n, cap },
        Some(_) => Standing::Within,
    }
}

/// EVERY failing file, not the alphabetically first one.
///
/// **THE LOOP THIS REPLACES ASSERTED INSIDE ITSELF**, so it died on the first
/// offender in `BTreeMap` order and every file behind it was reported as
/// absence — the same shape as `cargo test` without `--no-fail-fast`, inside a
/// test. Measured on a clean tree at `4ae9ea8c`: the old loop named `artha.t1`
/// and **seven** files were failing (`artha`, `encode`, `ir`, `parse` over
/// their pins; `sarani`, `shrinkhala`, `yantrotsarjana` with no pin at all).
/// One name for seven faults is how a five-day-old red stayed a one-line red.
fn ceiling_report(
    per_file: &BTreeMap<String, usize>,
    ceiling: &BTreeMap<&str, usize>,
) -> Vec<String> {
    let mut faults = Vec::new();
    for (f, n) in per_file {
        match standing(*n, ceiling.get(f.as_str()).copied()) {
            Standing::Within => {}
            Standing::Over { n, cap } => faults.push(format!(
                "{f}: {n} forward references, ceiling {cap} (+{}). A rise means the corpus \
                 gained a use before its declaration — permitted, but say so and raise the \
                 pin — or the instrument changed",
                n - cap
            )),
            Standing::Unpinned { n } => faults.push(format!(
                "{f}: {n} forward references and NO ROW in FORWARD_REFERENCES_CEILING — \
                 UNPINNED, which is not the same as over a ceiling of zero. A .t1 module \
                 joined the corpus and nobody pinned it: measure it, attribute the sites, \
                 and add the row"
            )),
        }
    }
    faults
}

/// THE CASES THAT MUST STILL BE REFUSED, AND THE ONE THAT MUST NOT — written
/// with the routine above rather than after it.
///
/// It touches no corpus and takes no interpreter pass, so it is deliberately
/// **not** a `ratchet:` and runs in the ordinary `cargo test -p sadhana-t1`.
/// The census it guards runs only on the hourly deep gate, which is precisely
/// how the 91 stayed wrong for five days; the classifier it guards can at least
/// be wrong for no longer than one landing.
#[test]
fn the_ceiling_reading_refuses_an_unpinned_file_rather_than_pinning_it_at_zero() {
    let one: BTreeMap<&str, usize> = [("pinned.t1", 5)].into_iter().collect();

    // AT the pin: silent. A ratchet that reds at its own pinned value pins nothing.
    assert_eq!(standing(5, one.get("pinned.t1").copied()), Standing::Within);
    // ONE OVER: refused, and the pin is carried in the verdict.
    assert_eq!(
        standing(6, one.get("pinned.t1").copied()),
        Standing::Over { n: 6, cap: 5 }
    );
    // UNDER: silent — the ceiling may fall, which is the const's own note.
    assert_eq!(standing(0, one.get("pinned.t1").copied()), Standing::Within);
    // NO ROW: refused as UNPINNED, and NOT as `Over { cap: 0 }`. This is the
    // case that read wrong for three real corpus modules.
    assert_eq!(
        standing(4, one.get("new.t1").copied()),
        Standing::Unpinned { n: 4 }
    );

    // AND THE REPORT NAMES EVERY FAILING FILE, NOT THE FIRST — the whole reason
    // the loop was replaced.
    let per_file: BTreeMap<String, usize> = [
        ("aaa.t1".to_string(), 9),
        ("pinned.t1".to_string(), 5),
        ("zzz.t1".to_string(), 1),
    ]
    .into_iter()
    .collect();
    let ceiling: BTreeMap<&str, usize> = [("pinned.t1", 5), ("aaa.t1", 2)].into_iter().collect();
    let faults = ceiling_report(&per_file, &ceiling);
    assert_eq!(faults.len(), 2, "{faults:?}");
    assert!(
        faults[0].starts_with("aaa.t1: 9 forward references, ceiling 2 (+7)"),
        "{faults:?}"
    );
    assert!(faults[1].starts_with("zzz.t1"), "{faults:?}");
    assert!(faults[1].contains("UNPINNED"), "{faults:?}");
    assert!(
        !faults[1].contains("ceiling 0"),
        "an unpinned file must never be reported as a ceiling of zero: {faults:?}"
    );
    // A CLEAN TABLE IS SILENT, or the report is not a report.
    assert!(ceiling_report(&BTreeMap::new(), &ceiling).is_empty());
}

/// THE RULE, HELD FROM BOTH SIDES. `spec/grammar-t1.ebnf`'s scope note
/// (W-222) says a module's top-level names are order-free and a local is
/// visible only from its `चरः` to the end of its block. This is the corpus
/// measured against that note, in one pass — the resolver's own verdict on
/// every file, the walk's count of forward references per file, and the
/// walk's count of uses before their `चरः` — with the REFUSED case first.
#[test]
#[ignore = "ratchet: one interpreter pass over the corpus; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn hoisting_is_stated_module_names_are_order_free_and_a_local_is_not() {
    // ── the refused case: a local used before its `चरः` ──────────────────
    let before = "वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ग ।\n    चरः ग ॱॱ अ६४ भवति ० ।\nइति\n";
    let p = read_program("before-let", before);
    assert_eq!(p.parse_errors, 0, "the fixture parses: {:?}", p.refusal);
    assert!(
        !p.resolved,
        "the resolver ACCEPTED a use of `ग` before its `चरः`; the language would \
         have hoisting and the grammar's scope note would be false"
    );
    assert_eq!(
        p.refusal.as_deref(),
        Some("undeclared `ग` at line 2"),
        "refused by name and line, as the first-refusal record reports it"
    );
    let w = Walk::run(&p);
    assert_eq!(
        uses_before_their_let(&p, &w),
        BeforeLet {
            unresolved: vec!["before-let:2 ग".into()],
            outward: Vec::new(),
        },
        "the detector sees the use the resolver refused"
    );
    assert!(forward_references(&p, &w).is_empty());

    // The initializer cannot name the local it initialises, either.
    let self_init = "वृत्तिः क ददाति अ६४ आदि\n    चरः ग ॱॱ अ६४ भवति ग ।\n    प्रत्यागमनम् ग ।\nइति\n";
    let p = read_program("self-init", self_init);
    assert_eq!(p.parse_errors, 0, "{:?}", p.refusal);
    assert!(
        !p.resolved,
        "a `चरः` whose initializer names itself is refused"
    );
    assert_eq!(p.refusal.as_deref(), Some("undeclared `ग` at line 2"));
    let w = Walk::run(&p);
    assert_eq!(
        uses_before_their_let(&p, &w).unresolved,
        vec!["self-init:2 ग".to_string()]
    );

    // The accepted twins: declared before use; and the same name bound
    // OUTSIDE first, so the early use resolves outward and the later `चरः`
    // is a shadow — legal, and counted as `outward`, not refused.
    let after = "वृत्तिः क ददाति अ६४ आदि\n    चरः ग ॱॱ अ६४ भवति ० ।\n    प्रत्यागमनम् ग ।\nइति\n";
    let p = read_program("after-let", after);
    assert!(p.resolved, "{:?}", p.refusal);
    let w = Walk::run(&p);
    assert_eq!(uses_before_their_let(&p, &w), BeforeLet::default());

    let shadow = "वृत्तिः क आदाय ग ॱॱ अ६४ ददाति अ६४ आदि\n    चरः घ ॱॱ अ६४ भवति ग ।\n    चरः ग ॱॱ अ६४ भवति १ ।\n    प्रत्यागमनम् घ ।\nइति\n";
    let p = read_program("shadow", shadow);
    assert!(
        p.resolved,
        "a shadow-to-come resolves outward: {:?}",
        p.refusal
    );
    let w = Walk::run(&p);
    let b = uses_before_their_let(&p, &w);
    assert_eq!(b.unresolved, Vec::<String>::new());
    assert_eq!(b.outward, vec!["shadow:2 ग".to_string()]);

    // ── the other half: a module name used before its declaration ────────
    let forward = "वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ख ० ।\nइति\nवृत्तिः ख आदाय ग ॱॱ अ६४ ददाति अ६४ आदि\n    प्रत्यागमनम् ग ।\nइति\n";
    let p = read_program("forward", forward);
    assert!(
        p.resolved,
        "a routine may call one declared after it: {:?}",
        p.refusal
    );
    let w = Walk::run(&p);
    assert_eq!(forward_references(&p, &w), vec!["forward:2 ख".to_string()]);
    assert_eq!(uses_before_their_let(&p, &w), BeforeLet::default());

    // ── the corpus, one pass ─────────────────────────────────────────────
    let mut per_file: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_forward = 0;
    let mut before_let = BeforeLet::default();
    let mut unresolved_files = Vec::new();
    for f in corpus_files() {
        let p = read_program(&f, &source(&f));
        if !p.resolved {
            unresolved_files.push(format!("{f}: {:?}", p.refusal));
        }
        let w = Walk::run(&p);
        let fwd = forward_references(&p, &w);
        total_forward += fwd.len();
        if !fwd.is_empty() {
            per_file.insert(f.clone(), fwd.len());
        }
        // Named per site, so a file over its ceiling says WHICH use.
        for s in &fwd {
            println!("  forward reference: {s}");
        }
        let b = uses_before_their_let(&p, &w);
        before_let.unresolved.extend(b.unresolved);
        before_let.outward.extend(b.outward);
    }
    println!("METRIC paradigm_name_global_uses_before_declaration {total_forward}");
    for (f, n) in &per_file {
        println!(
            "METRIC paradigm_name_global_uses_before_declaration_{} {n}",
            f.trim_end_matches(".t1")
        );
    }
    println!(
        "METRIC paradigm_name_local_uses_before_own_let_unresolved {}",
        before_let.unresolved.len()
    );
    println!(
        "METRIC paradigm_name_local_uses_before_own_let_resolving_outward {}",
        before_let.outward.len()
    );
    println!(
        "  resolving outward (shadow-to-come): {:?}",
        before_let.outward
    );

    assert!(
        unresolved_files.is_empty(),
        "every source resolves, or this ratchet is measuring refusals: {unresolved_files:?}"
    );
    // THE LOCAL RATCHET, at zero: the corpus writes no use of a local before
    // its `चरः` that nothing else binds. It cannot, and stay resolved — the
    // resolver refuses it — so this is the resolver's rule checked by a second
    // instrument, not a stylistic count.
    assert_eq!(
        before_let.unresolved,
        Vec::<String>::new(),
        "a use before its `चरः` that resolved to nothing survived resolution"
    );
    // THE GLOBAL RATCHET, per file, as a ceiling — REPORTED WHOLE.
    let ceiling: BTreeMap<&str, usize> = FORWARD_REFERENCES_CEILING.into_iter().collect();
    let faults = ceiling_report(&per_file, &ceiling);
    assert!(
        faults.is_empty(),
        "{} file(s) fail this ratchet, and the loop this replaced would have named \
         only the first of them:\n  {}",
        faults.len(),
        faults.join("\n  ")
    );
    let cap_total: usize = FORWARD_REFERENCES_CEILING.iter().map(|(_, n)| n).sum();
    assert!(
        total_forward <= cap_total,
        "{total_forward} forward references, ceiling {cap_total}"
    );
    assert!(
        total_forward > 0,
        "no forward reference in the corpus at all: the instrument is blind, or \
         the corpus was reordered and the note under module_decl is now history"
    );
}

// ── W-225: public names referenced nowhere, adjudicated and ratcheted ────

/// A public top-level name (`सार्वजनिक वृत्तिः|चरः|संरचना|गणना|यन्त्रम् NAME`)
/// that no line of T1 CODE in the corpus names outside its own declaration —
/// bare or as `मण्डलॱNAME` — and that no Rust test under `sadhana-t1/tests`
/// or `sadhana/tests` names either (the census's rule: a name a Rust test
/// calls by string is an entry point, not a dead name).
///
/// TEXT, NOT THE WALK, ON PURPOSE. The census above counts the uses the
/// resolver walks, and `W-209` found the resolver never walks an `अन्यथा`
/// body; so its "referenced nowhere" list of 52 held 29 names whose only
/// callers sit in else-bodies (`संरचनापठनम्` at `parse.t1:1203`,
/// `अन्तरमूल्यम्` at `samyojana.t1:1280`, `गणनाचिह्नकूटः` at
/// `vakyavibhaga.t1:3189`, …). A text scan sees an else-body. It is also
/// blind in its own way — a same-named local elsewhere counts as a reference
/// — so what it flags is dead for certain and what it clears may not be; a
/// ratchet wants exactly that side.
///
/// Enum VARIANTS are not in its universe: `vastu.t1:31 शोधनपङ्क्तिः` and
/// `:33 अन्यत्` are members of the closed set `स्थापन` mirrors from Rust's
/// `Placement`, and a closed set keeps its members.
fn public_names_referenced_nowhere(sources: &[(String, String)], rust_tests: &str) -> Vec<String> {
    let is_deva = |c: char| ('\u{0900}'..='\u{097F}').contains(&c) && c != '\u{0971}';
    let code: Vec<(String, String)> = sources
        .iter()
        .map(|(f, t)| {
            let stripped: Vec<&str> = t
                .lines()
                .map(|l| l.split('॰').next().unwrap_or(""))
                .collect();
            (f.clone(), stripped.join("\n"))
        })
        .collect();
    let mentions = |hay: &str, name: &str| -> usize {
        hay.match_indices(name)
            .filter(|&(i, _)| {
                let before = hay[..i].chars().next_back();
                let after = hay[i + name.len()..].chars().next();
                !before.is_some_and(is_deva) && !after.is_some_and(is_deva)
            })
            .count()
    };
    let mut out = Vec::new();
    for (f, t) in sources {
        for line in t.lines() {
            let Some(rest) = line.strip_prefix("सार्वजनिक ") else {
                continue;
            };
            let mut w = rest.split_whitespace();
            let (Some(kw), Some(name)) = (w.next(), w.next()) else {
                continue;
            };
            if !matches!(kw, "वृत्तिः" | "चरः" | "संरचना" | "गणना" | "यन्त्रम्")
            {
                continue;
            }
            let refs: usize = code
                .iter()
                .map(|(g, c)| mentions(c, name) - usize::from(g == f))
                .sum();
            if refs == 0 && !rust_tests.contains(name) {
                out.push(format!("{f} {kw} {name}"));
            }
        }
    }
    out.sort();
    out
}

/// Why each name is kept — read at its site, 2026-09-04, and the rule
/// applied is the row's: delete with a dated note where the Rust twin has no
/// such routine, else name the row that will use it. Every twin exists, so
/// nothing is deleted; and every deletion of a routine would also move the
/// ADR-0030 ledger (its parameters are typed bindings), which is the other
/// reason this is a list and not a diff.
const KEPT_PUBLIC_NAMES_REFERENCED_NOWHERE: &[(&str, &str)] = &[
    // encode.t1 — its Rust twin is `crates/sadhana/src/encode.rs`; every one
    // is a port of a routine or constant that file has, and five are pinned
    // by their declaration text in the ADR-0030 ledger tests
    // (`sanskrit-text/tests/grammar_t1.rs`). Consumer: `D-002a2`, port the
    // encoder's logic — the callers are what that open row still owes.
    //
    // THREE ROWS STRUCK 2026-09-04 BY `D-002a2`, THE CONSUMER THEY NAMED:
    // `असम्बन्धः`, `अपाकर्तव्यम्` and `सङ्कोचनिषेधः` gained their callers when
    // `सङ्कोचः` (`compressed_at`) got its body — `सम्बन्धाङ्कः` answers the
    // first, `रूपसङ्कोचः` calls the second, `सङ्कोचः` the third. Seven
    // encode.t1 rows became four. The two that still said "consumer D-002a2"
    // and were NOT reached by it now say who is.
    // THREE ROWS ADDED 2026-09-14, all from the table-index landing and none
    // of them dead code in the "delete it" sense — each is a routine whose
    // caller the index took over, recorded here rather than removed, because
    // deciding to delete a live-looking routine is a separate judgement from
    // noticing it has no caller.
    (
        "encode.t1 वृत्तिः रूपपूर्वदृष्टम्",
        "the dedupe walk over compression rows; `सङ्कोचसूचीरचना` now does that walk \
         over the index's own arenas and nothing calls this one",
    ),
    (
        "encode.t1 वृत्तिः सूचितकुलपङ्क्तिवत्",
        "written FOR the index — its margin says so — but `सङ्केतसूचीक्रमः` inlines \
         the two tests it wraps (`परिधिसाम्यम्` then `सूचितांशसाम्यम्`), so it \
         arrived without a caller",
    ),
    (
        "ir.t1 चरः खण्डशीर्षविस्तार",
        "the run header's total width; the sites that need it spell the two \
         offsets (base − १६, base − ८) directly, so the constant states a layout \
         nothing reads",
    ),
    (
        "encode.t1 चरः शोधनकोष्ठकम्",
        "closed set: RelSection::{Text,Data,Debug}; the other two are referenced",
    ),
    (
        "encode.t1 वृत्तिः चतुरष्टकसंख्या",
        "twin: the Vec<u32>::len of chunks_exact; ledger-pinned; D-002a2 closed without it — `words` is read by encode.rs's analysers, not by the encoder; consumer: whichever row ports the census/duplicate walk",
    ),
    (
        "encode.t1 वृत्तिः प्रत्ययवत्",
        "its own margin: 'NO caller in the corpus; the general form kept beside the specialised one'; ledger-pinned",
    ),
    (
        "encode.t1 वृत्तिः विन्यासविचलनम्",
        "twin: the drift B-058b2b7 traced; ledger-pinned; D-002a2 closed without it — उत्सर्जनक्रमः's E23 is `सङ्केतनदोषरचना ० २३ ० ०`, no arguments; consumer: the row that fills E23's four arguments",
    ),
    // parse.t1 — T1's own parser, `व्याकर`. `यन्त्रघोषणाभेद` (closed set: the
    // five declaration kinds, ३) STOOD HERE on the W-225 base; on main the
    // W-207 census (`t1_paradigm.rs:423`) names it in a margin, and a name a
    // Rust test file carries is "named in a Rust test" by the census's rule —
    // so it left the list when W-226 rebased, 2026-09-04. 22, not 23.
    // (W-224's gate found the same red on origin/main 707ca206 the same hour:)
    // parse.t1 — T1's own parser, `व्याकर`.
    // `parse.t1 चरः यन्त्रघोषणाभेद` ("closed set: the five declaration kinds
    // (३); the census's DECL_MACHINE") WAS HERE and is dropped 2026-09-04
    // (W-224's gate): `t1_paradigm.rs:420` has named it in a margin since
    // W-206 (`901f387c`), so by this scan's own rule — "named only by a Rust
    // test is not flagged", the refused case above — it was never on the
    // flagged list, and this row made the ratchet RED on `origin/main` at
    // `707ca206` (reproduced on a pristine checkout). The corpus still
    // references it nowhere; the census's DECL_MACHINE constant is the twin
    // that keeps it.
    // …and `W-215`'s printer references it besides (a device declaration is
    // refused by that kind), so it has a caller in the corpus too.
    (
        "parse.t1 वृत्तिः दर्शनम्",
        "twin: parse.rs's peek; व्याकर reads by index and has not needed it; W-216's decision table",
    ),
    // kosha.t1 — twin `crates/sadhana/src/kosha.rs`. ADDED 2026-09-06 by the ELF
    // image writer's phase 2, and added WITH THE ROW THAT WILL USE THEM rather
    // than referenced from nowhere: `प्रतिबिम्बलेखनम्` is called by phase 3,
    // which feeds it `samyojana.t1`'s layout, and `भारस्थानम्` is the load
    // address that phase passes as its argument.
    //
    // THEY ARE PROVEN, NOT MERELY DECLARED. `crates/yantra/tests/kosha_image.rs`
    // calls the writer directly and asserts `Machine::load_elf` accepts what it
    // returns — so a name with no caller IN THE CORPUS still has a test that
    // fails if it stops working. That is the distinction this list is for: an
    // uncalled name that nothing exercises is dead, and this one is not.
    // TWO kosha.t1 ROWS STRUCK 2026-09-07 BY THE DRIVER'S BACK HALF, THE
    // CONSUMER THEY WERE WAITING FOR. `प्रतिबिम्बलेखनम्` and `भारस्थानम्` said
    // "phase 3 calls it" and phase 3 was Rust; `शृङ्खलाॱवस्तुप्रतिबिम्बम्` now
    // calls both from `.t1`, so they are referenced and their rows are gone.
    //
    // That is the direction this list is supposed to move: an entry here is a
    // routine whose caller has not been written yet, and the row that writes
    // the caller removes the entry. Three names left this list in one change —
    // these two, and `मण्डलसङ्कलनम्`, which the back half now calls.
    // samyojana.t1 — twin `samyojana.rs`.
    (
        "samyojana.t1 चरः अनिर्दिष्टकोष्ठकम्",
        "closed set: SymSection::{Text,Data,Bss,Undefined} (४)",
    ),
    // shrinkhala.t1 — THE DRIVER'S TOP LEVEL, and the one entry here that is
    // referenced nowhere BY CONSTRUCTION rather than pending a consumer.
    // `मण्डलप्रतिबिम्बम्` takes a `.t1` source and answers an ELF image; every
    // stage below it is called by the stage above, and nothing in the corpus
    // calls the top because the top is what an outside caller invokes.
    //
    // A CONSUMER WOULD BE THE WRONG THING TO NAME. The other entries wait for
    // a row to reach them; this one is reached by `yantra`'s
    // `the_driver_takes_a_source_to_an_image_that_loads`, which is a Rust
    // caller and so invisible to a scan over `.t1` text. **Its being unreferenced
    // in the corpus is the property that makes it an entry point.**
    //
    // Note this row REMOVED an entry as well as adding one: `मण्डलसङ्कलनम्`
    // was the top before the back half existed, and it now has a `.t1` caller
    // — `मण्डलप्रतिबिम्बम्` — so it left this list on its own.
    (
        "shrinkhala.t1 वृत्तिः मण्डलप्रतिबिम्बम्",
        "the driver's top level: a .t1 source to an ELF image, invoked from outside the corpus; unreferenced here is what an entry point looks like",
    ),
    // ADDED 2026-09-10 BY W-279, AND IT IS THE OPPOSITE KIND OF ENTRY TO THE ONE
    // ABOVE. `मण्डलप्रतिबिम्बम्` is unreferenced because it is an ENTRY POINT.
    // This one is unreferenced because its case CEASED TO EXIST: it was the exit
    // meaning "no routines — data only", assigned by a guard that refused any
    // module without routines. W-279 removed that guard, because it deferred a
    // capability rather than preventing a defect — `ast.t1` and `vastu.t1` now
    // compile to real objects. Its in-degree went 1 → 0 in that commit.
    //
    // KEPT RATHER THAN DELETED, deliberately: its absence is the record that a
    // routine-less module used to be refused here, and a reader who finds the
    // constant will find this row. **But a declared exit at in-degree 0 is a
    // liability and this row says so out loud** — the same unit found that a
    // DIFFERENT exit had silently inherited this one's case, which is exactly
    // how a premise-less constant misleads. If a future row gives it a meaning,
    // give it a caller in the same commit or strike it.
    (
        "shrinkhala.t1 चरः सङ्कलनावृत्तिभेद",
        "W-279: the routine-less exit, in-degree 1 -> 0 when the guard that refused data-only modules was removed; kept as the record that they used to be refused, not because anything reaches it",
    ),
    // THE INPUT CHANNEL'S THREE TAGS — read by the HOST, by VALUE (2026-09-21).
    //
    // `yantra-run` places a file in RAM before the first instruction, and the
    // image carries no symbol table, so it finds each input slot by scanning for
    // the tag word declared IMMEDIATELY before it ("SASINPUT", "SASINAME",
    // "SASTRACE"). No `.t1` code reads a tag, and none should: a `.t1` reader
    // would be a second consumer of a value whose only job is to be FOUND. Their
    // being unreferenced in the corpus is the property, exactly as for the
    // driver's top level above — the consumer is outside the corpus.
    //
    // The slots they mark (`निवेशपाठः`, `निवेशमण्डलनाम`, `निवेशानुरेखणम्`) are
    // read by `स्वपरीक्षानिवेशः` and so are not listed. The tags are pinned to
    // `yantra::input`'s constants and checked adjacent to their slots in the
    // EMITTED text by `crates/yantra/tests/t1_input_channel.rs`; delete a tag and
    // that file, not this list, is what should fail first.
    (
        "shrinkhala.t1 चरः निवेशसङ्केतः",
        "input channel: the SASINPUT tag, found by the host by value in front of निवेशपाठः; unreferenced in the corpus by design",
    ),
    (
        "shrinkhala.t1 चरः निवेशनामसङ्केतः",
        "input channel: the SASINAME tag, found by the host by value in front of निवेशमण्डलनाम; unreferenced in the corpus by design",
    ),
    (
        "shrinkhala.t1 चरः निवेशानुरेखणसङ्केतः",
        "input channel: the SASTRACE tag, found by the host by value in front of निवेशानुरेखणम्; unreferenced in the corpus by design",
    ),
    // THE CORPUS-WALK TOP LEVEL IS NOT LISTED HERE, AND THE REASON IS A TRAP
    // WORTH THE PARAGRAPH.
    //
    // `मण्डलानिप्रतिबिम्बम्` has the identical property to the entry above: a
    // second top level, invoked from `yantra`'s Rust tests, called by nothing in
    // the corpus. It WAS adjudicated here for exactly one gate, and then left
    // the flagged set on its own — **without gaining a caller.**
    //
    // THE SCAN'S LAST CLAUSE IS `!rust_tests.contains(name)`, a substring test
    // over the RAW TEXT of every `.rs` file under `sadhana-t1/tests` and
    // `sadhana/tests`, COMMENTS INCLUDED. This file is excluded — `:3061` says
    // why, "a list that counted as a caller would clear every name it holds" —
    // **but no other test file is.** So when the `w250-shares` share census was
    // re-taken and its margin named the two new share sites
    // `[मण्डलानिप्रतिबिम्बम्]`, that margin became a "reference" and unflagged
    // the name here.
    //
    // A MARGIN IN ONE TEST FILE CHANGED THE RESULT OF A CENSUS IN ANOTHER. The
    // self-exclusion at `:3061` anticipated the circularity for THIS file and
    // the hazard is not specific to it: any test margin that names a public
    // routine silently adjudicates it. `मण्डलप्रतिबिम्बम्` above survives only
    // because `w250-shares.rs` happens not to name it — zero occurrences,
    // checked, not assumed.
    //
    // LEFT UNLISTED RATHER THAN WORKED AROUND. Renaming the share margin to
    // dodge the substring would be gaming the check, and deleting the margin
    // would lose the composition finding it carries. The scan's rule is
    // "named in a Rust test"; it is named in one; so it is not flagged and must
    // not be adjudicated. **If that margin is ever reworded, this name returns
    // to the flagged set and the entry must come back** — which is the one
    // thing a reader of a shrinking list needs to know.
    // vakyavibhaga.t1 — the sentence reader, twin `crates/sadhana/src/parse.rs`.
    // EIGHT ROWS STOOD HERE on 2026-09-04 morning — the byte- and
    // text-directive halves of D-002f6/f7/f8 (अङ्कपाठरचना, अष्टकनिर्देशवाचकः,
    // आज्ञाकूटरचना, आस्कीपाठः, उक्तिपदसंख्या, उक्तिपरीक्षा, उक्तिपाठः, जालपाठः)
    // "awaiting an octet arena". W-239 built the arena (`ashtaka.t1`) and wired
    // all eight the same day: the executor `निर्देशानुष्ठानम्` dispatches the
    // text and width arms, the width arm's string case reads through the
    // three string readers, and `आरम्भः` builds the two code tables. Gone
    // from the list because they are called, as the assertion instructs.
    (
        "vakyavibhaga.t1 वृत्तिः आज्ञासङ्कलनम्",
        "twin: assemble_source (D-002f9), THE ENTRY; Rust tests drive वाक्यपठनम्/आरम्भः instead; consumer D-002 (self-hosting)",
    ),
    // vishlesana.t1 — the disassembler, twin `vishlesana.rs`. Their caller
    // is named in their own margins: `संयोजन ॱ पदपुनर्निर्माणम्`, the
    // linker's patch path, which does not call them yet.
    (
        "vishlesana.t1 वृत्तिः अवकाशभेदाः",
        "twin: operand kinds; ledger-pinned; consumer: संयोजन's पदपुनर्निर्माणम् (D-002)",
    ),
    (
        "vishlesana.t1 वृत्तिः अवकाशमूल्यानि",
        "twin: operand values; ledger-pinned; same consumer",
    ),
    (
        "vishlesana.t1 वृत्तिः नामसङ्केतः",
        "twin: name → encoding lookup; same consumer",
    ),
    (
        "vishlesana.t1 वृत्तिः मेलनगणना",
        "twin: candidates.len, which tests/roundtrip.rs asserts is never two; same consumer",
    ),
    // sanchaya.t1 — the shared declaration store (W-223 part 1). Its lookups
    // and accessors are called from Rust today (t1_sanchaya.rs, the census
    // measure_corpus_qualified_uses); the one below nothing in the corpus calls
    // yet.
    //
    // `निषेधशुद्धिः` LEFT THIS LIST on 2026-09-04 (W-223 part 2), which is what
    // part 1's note here predicted: "their consumer is part 2, the resolver
    // writing संज्ञासूचकाङ्क from the store". `सदस्यनिर्णयः` calls it before every
    // lookup, because `सञ्चयदोषभेद` is STICKY — `निषेधः` sets it and only this
    // clears it — so a lookup that did not clear would read the kind left by an
    // earlier failure and take the wrong branch, turning a module nobody
    // collected into a module missing a member. A routine written for a hazard
    // and never called is a hazard nobody has met yet.
    //
    // `प्रविष्टिमण्डलम्` LEFT THIS LIST on 2026-09-05 (`W-265`), and its row's
    // own prediction is what came true a second time. It read "consumer: the
    // census reading the store back (Rust), part 2"; the consumer turned out to
    // be T1 and not Rust. The store keeps a type as TEXT in the SOURCE's own
    // spelling, so a bare `चिह्नक` in `पदविभाग`'s entry names `पदविभाग`'s type
    // and nobody else's — which means reading a store entry's return type
    // needs the entry's MODULE, and this is the accessor that answers it.
    // `सञ्चयसंज्ञा` and `सञ्चयप्रकारबन्धः` both call it now. Removed here in the
    // same commit that gave it callers, as this ratchet's message asks.
];

/// THE RATCHET: exactly the kept list, so that a NEW public name nothing
/// references fails here by name, and a kept one that gains a caller is
/// removed from the list in the same commit. The refused case first.
#[test]
fn every_public_name_referenced_nowhere_is_adjudicated_and_the_list_is_exact() {
    // REFUSED: nothing references `क`, so it is flagged; `ख` is called only
    // from an else-body — the census's blind spot — and is NOT flagged; `ग`
    // is reached only as `मॱग` from another file and is not flagged; `घ` is
    // named only by a Rust test and is not flagged.
    let a = "मण्डलम् म ॥\n\
             सार्वजनिक वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
             सार्वजनिक वृत्तिः ख ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
             सार्वजनिक वृत्तिः ग ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
             सार्वजनिक वृत्तिः घ ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
             वृत्तिः च आदाय य ॱॱ बूल ददाति अ६४ आदि\n\
             \x20   यदि य आदि\n        प्रत्यागमनम् ० ।\n    इति अन्यथा आदि\n        प्रत्यागमनम् ख ।\n    इति\n\
             इति\n";
    let b = "मण्डलम् न ॥\nआयातः म ।\n\
             वृत्तिः छ ददाति अ६४ आदि\n    प्रत्यागमनम् मॱग ।\nइति\n\
             ॰ a margin naming क does not count: क क क\n";
    let flagged = public_names_referenced_nowhere(
        &[("a.t1".into(), a.into()), ("b.t1".into(), b.into())],
        "it.call(\"मॱघ\", vec![], 1)",
    );
    assert_eq!(flagged, vec!["a.t1 वृत्तिः क".to_string()]);

    // The corpus.
    let sources: Vec<(String, String)> = corpus_files()
        .into_iter()
        .map(|f| {
            let text = source(&f);
            (f, text)
        })
        .collect();
    let rust_tests: String = ["sadhana-t1/tests", "sadhana/tests"]
        .iter()
        .flat_map(|d| {
            std::fs::read_dir(repo_root().join("crates").join(d))
                .into_iter()
                .flatten()
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        // NOT THIS FILE: the kept list above names all 23, and a list that
        // counted as a caller would clear every name it holds. (The census's
        // own "named in a Rust test" partition does read this file, so its
        // 52 reads 29 from here on; the number that matters is this one.)
        .filter(|p| p.file_name().is_none_or(|n| n != "t1_paradigm_names.rs"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();
    let mut flagged = public_names_referenced_nowhere(&sources, &rust_tests);
    // **THE RUNG LADDER IS A CLASS, NOT SEVENTY-FOUR ROWS (2026-09-14).**
    // `shrinkhala.t1`'s `स्वपरीक्षा…` routines are ENTRY POINTS: each is one rung
    // of the self-hosting ladder, run by name from outside as
    // `t1_image --entry शृङ्खला स्वपरीक्षाNN`, and referenced by nothing inside the
    // corpus BY DESIGN — including by `स्वपरीक्षा` itself, which is its own rung
    // and compiles a one-declaration module. Nothing here is dead: the thing
    // that calls them is a command line.
    //
    // Held as a rule with its population ASSERTED rather than as rows, because
    // seventy-four hand-copied names is a list nobody reads and every future
    // rung would arrive as a red demanding another. The count below is what
    // keeps the rule from being a blanket: a rung that vanishes fails here just
    // as loudly as one that appears, and anything NOT named `स्वपरीक्षा` still
    // has to be adjudicated by name in the list above.
    //
    // THE STEM `स्वपरीक्ष`, NOT THE WORD `स्वपरीक्षा`, AND THE DIFFERENCE IS
    // THIRTEEN RUNGS. Sandhi joins the ordinal to the name: `स्वपरीक्षा` + `एक`
    // is `स्वपरीक्षैक…`, so `स्वपरीक्षैकविंशी` and twelve of its neighbours do not
    // begin with the word this rule was first written against. Measured, not
    // reasoned: the first version of this assertion said 58 and the population
    // is 71.
    //
    // 71 -> 72 on 2026-09-14: `स्वपरीक्षात्रिसप्ततिः`, the first rung that
    // ASSEMBLES rather than stopping at emitted text. The pin firing on a
    // deliberate addition is the rule working — it cost one line to re-take and
    // it is the same line that would fire if a rung silently vanished.
    //
    // 72 -> 74 the same day: `चतुःसप्ततिः` (which line the compiled encoder
    // refused) and `पञ्चसप्ततिः` (whether `भवति ०` gives one slot). Three rungs
    // in one afternoon is what a ladder looks like when it is bisecting rather
    // than reporting — each one is a question the previous answer made precise.
    // 74 -> 76 with `षट्सप्ततिः` (the layout's length) and `सप्तसप्ततिः` (the
    // comparison as E22 builds it). A rung's own weight is negligible against
    // what it measures: rung 76 cost +12,911,498 steps over rung 73e, 0.05%,
    // beside the 6.6% the index-read guard cost.
    // **COUNTED FROM THE DECLARATIONS, NOT FROM THE UNREFERENCED SET
    // (2026-09-14, after sansos-c1 measured it falling the wrong way).** This
    // used to count rungs inside `flagged`, which is the set NOTHING NAMES — so
    // when `t1_output_channel.rs` named rung 73, the rung left the set and the
    // number FELL from 77 to 76 while the ladder GREW from 79 to 80. A fall then
    // had two causes wanting opposite responses — a rung gained a caller inside
    // the corpus, or a Rust test started naming one — and the assertion could
    // not tell them apart, which is the whole job of a pin.
    //
    // The population I meant is "how many rungs the ladder HAS". That is a
    // property of `shrinkhala.t1` and of nothing else, so it is read from the
    // source. Naming a rung in a test now moves NOTHING here, and a rung added
    // or deleted moves it by exactly one.
    let rungs = source("shrinkhala.t1")
        .lines()
        .filter(|l| l.starts_with("सार्वजनिक वृत्तिः स्वपरीक्ष"))
        .count();
    println!("METRIC paradigm_name_shrinkhala_rung_entry_points {rungs}");
    // **THE COUNT ABOVE READS `shrinkhala.t1` DIRECTLY SINCE 2026-09-14
    // (`1c2c6afb`). EVERYTHING BELOW IS HISTORY — it WAS a count of rungs
    // nothing names, and that is the defect being recorded, not the behaviour.**
    //
    // 77 -> 76 while a rung was ADDED, and the direction was the whole finding.
    // Rung 79 (`स्वपरीक्षैकोनाशीतिः`) landed with the clause-nine layout dump, so
    // the declared ladder went 79 -> 80 and this count went DOWN.
    //
    // It was not a count of rungs. It was a count of rungs NOTHING NAMES — and
    // `t1_output_channel.rs` names two of them: rung 79, so it never enters the
    // set, and rung 73 (`स्वपरीक्षात्रिसप्ततिः`), which was in it and left. Rung 79
    // is the dump switched ON and rung 73 is the same run with it OFF; the test
    // asserts their statuses agree, because a status that moves means the
    // instrument perturbed what it measures. **Naming rung 73 is the point of
    // that test**, so this is the pin recording a real change of state rather
    // than an accounting wobble.
    //
    // The arithmetic is checkable rather than asserted: at `1d57e645` — the
    // consumer landed, its test naming `वस्तुपाठ्यमुद्रणम्` but no rung — the whole
    // package was green at 77. One commit later, two rungs named, 77 - 1 = 76.
    //
    // **WHY IT WAS REPAIRED RATHER THAN RE-TAKEN.** A fall had two causes wanting
    // OPPOSITE responses — a rung gained a caller inside the corpus, or a Rust
    // test started naming one — and the assertion could not separate them. A pin
    // that cannot separate its own causes gets re-taken by whoever meets it, in
    // whichever direction is convenient at the time. It was in fact re-taken to
    // 76 first: correct arithmetic, wrong repair. The rule the two instances of
    // this gave up (here and `t1_driver`'s census the same morning) is
    // **assert what the thing itself determines, not an artefact of how other
    // files happen to refer to it.**
    //
    // `measure_paradigm_names` still prints the two DEAD buckets separately —
    // "named in a Rust test" against "named nowhere" — which is where to look
    // when the ADJUDICATION list below moves, since that list still depends on
    // who names what.
    // **REPORT-ONLY SINCE 2026-09-19, AND THE RULING NAMES THE SHAPE.** Owner
    // ruling of 2026-09-13, item 1: a pin that "encodes nothing about
    // correctness" prints a `METRIC` and a `NOTE` when it moves, and reds
    // nothing. This was `assert_eq!(rungs, 80)` and it had been RED since
    // `7614a6fb`.
    //
    // **THE CAUSE WAS MEASURED PER COMMIT, NOT GUESSED.** Counting the
    // declarations in every tree from `1c2c6afb` (where 80 was taken) to HEAD,
    // the number moves by exactly one, eight times, and every one is a rung the
    // native bisect deliberately added:
    //
    //     80 -> 81  7614a6fb  rung 80: the compiler's own output through the channel
    //     81 -> 82  6713c8ae  bisect: the front half compiles a routine NATIVELY
    //     82 -> 83  23272e63  rung 82: a progress marker, because a fault cannot report
    //     83 -> 84  37cf662a  rung 82 native: the assembler dies inside
    //     84 -> 85  ff3d479f  rung 83 native: the instruction ENCODER is the site
    //     85 -> 86  92f5b393  rung 84 native: BYTE-IDENTICAL for 232 records, then faults
    //     86 -> 87  9e745382  rung 85 did not refute anything
    //     87 -> 88  4fee0d04  rung 95 native: the FIELD WRITE faults
    //
    // Eight deliberate additions, eight re-takes owed on a number that says
    // nothing about whether any of the eight works. That is the tax the ruling
    // exists to stop paying, and a ladder mid-bisect grows a rung per step.
    //
    // **WHAT STILL REDS, BECAUSE A REPORT-ONLY LINE INVITES ITS OWN BREAKAGE.**
    // The predicate is a literal Devanagari prefix. Rename `सार्वजनिक वृत्तिः`
    // or the `स्वपरीक्ष` stem and this instrument answers 0 for a ladder that
    // is still entirely there — and a `METRIC … 0` reads like a measurement.
    // So the count being ZERO is still a refusal: that is the instrument
    // breaking, which is a different fact from the count moving, and the two
    // must not share one silent line.
    const RUNGS_WHEN_LAST_TAKEN: usize = 88; // 2026-09-19, clean tree, measured twice
    assert!(
        rungs > 0,
        "the rung scan found NO `सार्वजनिक वृत्तिः स्वपरीक्ष…` declaration in \
         `shrinkhala.t1`. The ladder did not vanish — the predicate stopped \
         matching. This is the INSTRUMENT breaking, not the count moving."
    );
    if rungs != RUNGS_WHEN_LAST_TAKEN {
        println!(
            "NOTE  paradigm_name_shrinkhala_rung_entry_points moved \
             {RUNGS_WHEN_LAST_TAKEN} -> {rungs} since 2026-09-19. This counts \
             DECLARATIONS IN THE SOURCE, so naming a rung in a Rust test does \
             not move it; a rung added or deleted moves it by exactly one."
        );
    }
    flagged.retain(|f| !f.starts_with("shrinkhala.t1 वृत्तिः स्वपरीक्ष"));
    println!(
        "METRIC paradigm_name_public_referenced_nowhere_by_text {}",
        flagged.len()
    );
    for f in &flagged {
        println!("  {f}");
    }
    let kept: Vec<String> = KEPT_PUBLIC_NAMES_REFERENCED_NOWHERE
        .iter()
        .map(|(n, _)| (*n).to_string())
        .collect();
    let mut kept_sorted = kept.clone();
    kept_sorted.sort();
    assert_eq!(
        flagged, kept_sorted,
        "the set of public names nothing references changed. A NEW one: reference \
         it, or delete it with a dated note if its Rust twin has no such routine, \
         or add it above WITH the row that will use it. One GONE: it gained a \
         caller — remove its row above in the same commit."
    );
    assert_eq!(
        kept.len(),
        // 13 since `W-kosha` phase 2 added the ELF image writer's two —
        // `प्रतिबिम्बलेखनम्` and `भारस्थानम्`. Both are adjudicated above WITH
        // the row that will call them (phase 3), and both are exercised today by
        // `crates/yantra/tests/kosha_image.rs`, which asserts `Machine::load_elf`
        // accepts what the writer returns. THE COUNT RISING IS THE EXPECTED
        // DIRECTION HERE and not a regression: this list shrinks when a name
        // gains a caller and grows when a module is written before its caller,
        // which is the order a port has to happen in.
        //
        // 13 -> 12 on 2026-09-07, THE DRIVER'S BACK HALF, AND THIS IS THE
        // OPPOSITE DIRECTION — the one the margin above says to expect when a
        // name gains a caller. THREE NAMES LEFT AND ONE JOINED:
        //
        //   struck  kosha.t1 प्रतिबिम्बलेखनम् · kosha.t1 भारस्थानम्
        //           adjudicated WITH "phase 3 calls it", and phase 3 was Rust;
        //           `शृङ्खलाॱवस्तुप्रतिबिम्बम्` now calls both from `.t1`.
        //   struck  shrinkhala.t1 मण्डलसङ्कलनम् — the driver's top level until
        //           the back half existed; `मण्डलप्रतिबिम्बम्` now calls it.
        //   joined  shrinkhala.t1 मण्डलप्रतिबिम्बम् — the new top, and the one
        //           entry here that is unreferenced BY CONSTRUCTION rather than
        //           pending a consumer.
        //
        // The prediction those two kosha rows carried was correct and is now
        // discharged: they named phase 3 as their consumer and phase 3 reached
        // them. AN ADJUDICATION THAT NAMES ITS CONSUMER CAN BE CHECKED WHEN THE
        // CONSUMER ARRIVES; one that says "no caller yet" cannot.
        //
        // UNMOVED AT 12 ACROSS THE CORPUS WALK, AND THE ROUTE THERE IS THE
        // RECORD. `मण्डलानिप्रतिबिम्बम्` joined the flagged set when it was
        // written — 12 -> 13, adjudicated — and left it again one gate later
        // WITHOUT GAINING A CALLER, because the `w250-shares` re-take named it
        // in a margin and the scan's `rust_tests.contains(name)` reads every
        // other test file's comments as references. The long note beside
        // `मण्डलप्रतिबिम्बम्` above says how, and what to do if that margin is
        // reworded.
        //
        // THE COUNT IS A SECOND ASSERTION BEHIND THE LIST and both had to move
        // together in each direction: a test stops at its FIRST failing
        // assertion, so re-taking the list alone left this red, and "the pin is
        // re-taken" after one green was wrong. It went 12 -> 13 -> 12 over
        // three gates, and only the last of those is a number anyone would
        // guess from the diff.
        // 12 -> 13 on 2026-09-10, W-279: `सङ्कलनावृत्तिभेद`. The margin above
        // was written from the last three gates and it held again — this second
        // assertion sat invisible behind the list until the list went green,
        // exactly as it says. A RED IS A LOWER BOUND, and the file predicted its
        // own next failure.
        //
        // AND THE DIRECTION IS THE UNUSUAL ONE. This list normally grows when a
        // module is written before its caller. This entry joined because its
        // CASE was deleted: the guard that refused routine-less modules is gone,
        // so the exit meaning "no routines" has nothing left to assign it. A name
        // arriving here for that reason is a liability, not a pending consumer,
        // and its row says so.
        // 13 -> 16 on 2026-09-14, THE GROWING DIRECTION AGAIN, and all three
        // from one landing: the table index took over the callers of
        // `रूपपूर्वदृष्टम्` and arrived with `सूचितकुलपङ्क्तिवत्` already unused,
        // and `खण्डशीर्षविस्तार` states a header width the sites spell as two
        // offsets. None is a port waiting for its caller, which is what the
        // margin above expects of a rise — they are routines a rewrite went
        // around, so the row to watch is whether they are still here in a week.
        //
        // The self-test rung ladder is NOT in this count: 71 `स्वपरीक्ष…` entry
        // points in shrinkhala.t1 are held as a class with their population
        // asserted, above.
        //
        // 16 -> 19 on 2026-09-21, THE GROWING DIRECTION, and deliberately: the
        // input channel's three TAGS (`निवेशसङ्केतः`, `निवेशनामसङ्केतः`,
        // `निवेशानुरेखणसङ्केतः`). Unlike the rise above, these are not routines
        // a rewrite went around and not ports awaiting a caller — their
        // consumer is `yantra-run`, which finds them by VALUE because the image
        // has no symbol table. A `.t1` caller would be wrong. So the row to watch
        // is the opposite of the one above: they should STILL be here in a week.
        19,
        "W-225 left 23, one of which (`यन्त्रघोषणाभेद`) a Rust margin had named all \
         along — 22 since W-226's rebase and W-224's gate (W-215's printer references the \
         same name); D-002a2 19 — असम्बन्धः, अपाकर्तव्यम् and सङ्कोचनिषेधः gained callers with \
         सङ्कोचः; 11 since W-239 wired the eight directive halves (called now, so gone); \
         13 since W-223's store added two the corpus does not call yet (part 2 will); \
         12 since W-223 PART 2 called one of them — `निषेधशुद्धिः`, exactly as the note \
         beside it predicted, because `सदस्यनिर्णयः` must clear the sticky refusal kind \
         before every lookup; the census's 52 held 29 else-body callers; 11 since \
         `W-265` called the OTHER of W-223's two — `प्रविष्टिमण्डलम्`, and its note \
         predicted a Rust consumer where the consumer turned out to be T1: reading a \
         store entry's return type needs the entry's MODULE, because the store keeps a \
         type in the SOURCE's own spelling and a bare name means that source's type"
    );
}

#[test]
fn two_modules_exporting_one_name_are_one_collision_and_a_private_one_is_none() {
    let a = read_program(
        "a",
        "मण्डलम् क ॥\nसार्वजनिक वृत्तिः साधारणम् ददाति अ६४ आदि\n    प्रत्यागमनम् १ ।\nइति\n",
    );
    let b = read_program(
        "b",
        "मण्डलम् ख ॥\nसार्वजनिक वृत्तिः साधारणम् ददाति अ६४ आदि\n    प्रत्यागमनम् २ ।\nइति\nसार्वजनिक वृत्तिः अन्यम् ददाति अ६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n",
    );
    let c = read_program(
        "c",
        "मण्डलम् ग ॥\nवृत्तिः साधारणम् ददाति अ६४ आदि\n    प्रत्यागमनम् ४ ।\nइति\n",
    );
    for p in [&a, &b, &c] {
        assert_eq!(p.parse_errors, 0, "{} parses", p.file);
    }
    let units: Vec<(String, Vec<String>)> = [&a, &b, &c]
        .iter()
        .map(|p| {
            (
                p.module.clone().unwrap(),
                p.exports().into_iter().map(|(n, _)| n).collect(),
            )
        })
        .collect();
    let found = collisions(&units);
    // THE REFUSED CASE: the same public name in two modules IS a collision,
    // and the private `साधारणम्` in `ग` does not join it.
    assert_eq!(
        found,
        vec![(
            "साधारणम्".to_string(),
            vec!["क".to_string(), "ख".to_string()]
        )]
    );
    // `अन्यम्` in one module only is none.
    assert!(collisions(&units[1..]).is_empty());
}

#[test]
fn the_import_graph_reports_a_cycle_and_none_for_a_chain() {
    let s = |x: &str| x.to_string();
    let nodes = vec![s("क"), s("ख"), s("ग")];
    let chain = vec![(s("क"), s("ख")), (s("ख"), s("ग"))];
    assert!(cycles(&nodes, &chain).is_empty());
    let ring = vec![(s("क"), s("ख")), (s("ख"), s("ग")), (s("ग"), s("क"))];
    assert_eq!(cycles(&nodes, &ring), vec![vec![s("क"), s("ख"), s("ग")]]);
    let me = vec![(s("क"), s("क"))];
    assert_eq!(
        cycles(&nodes, &me),
        vec![vec![s("क")]],
        "a self-import is a cycle"
    );
}

/// ॥ THE STORE AND THE LOADER HOLD THE SAME DECLARATIONS ॥ rowless, `W-279`.
///
/// A GUARD FOR A DEFECT THAT WAS NEVER DIAGNOSED, and that is why it exists
/// rather than a fix. On 2026-09-07 the T1 store silently lost FIVE of
/// `artha.t1`'s globals — `असत्क्षेत्रसंख्या मस्ति नाम वस्तु पङ्क्ति`, consecutive,
/// mid-file, mixed types — while the Rust loader kept them. It appeared when
/// `ir.t1` grew by 371 lines, and it stopped reproducing when main grew by
/// other means. Four suspects were eliminated and none explained it: the
/// declaration cursor (reset at `parse.t1:1433`), a capacity bound
/// (`घोषणाकोश` grows), tail truncation (40 declarations survived after the
/// five) and the collector's enum window (no `गणना` within two thousand lines).
///
/// IT IS NOT KNOWN TO BE FIXED. IT IS KNOWN NOT TO REPRODUCE. The failure
/// tracked how much `.t1` corpus existed, so a tree that shifted the quantity
/// the other way hides it exactly as completely as a repair would — and
/// `encode.t1` is the next large addition due.
///
/// NOT `#[ignore]`d, DELIBERATELY. The check that held this property already
/// existed inside `measure_corpus_qualified_uses`, which IS ignored, so only
/// `tools/light-gate.sh` reached it — and the gate did not run it for the
/// landing that first broke it, because that landing was gated on a dirty tree
/// whose real changes were already committed. A correctness assertion nobody
/// invokes by name is a correctness assertion nobody runs.
///
/// TWO THINGS THE HUNT EARNED, both load-bearing:
///
///   ॱ THE COUNT IS ASSERTED, NOT ONLY THE AGREEMENT. Two sides that both lost
///     the same declarations agree perfectly. `paradigm_store_entries` moving
///     without a `.t1` change is itself a finding, and agreement alone is blind
///     to it. The pin is a floor rather than an equality: corpus growth raises
///     it legitimately and often, and a ratchet that must be re-taken on every
///     `.t1` edit gets re-taken without being read.
///   ॱ THE DISAGREEING DECLARATIONS ARE NAMED, not counted. The five names are
///     what let four suspects be eliminated in an hour; "5 disagree" would have
///     told nobody anything.
#[test]
fn the_store_and_the_loader_hold_the_same_declarations() {
    let files = corpus_files();
    let mut st = load_store();
    st.call("घोषणासञ्चयॱआरम्भः", vec![], 1_000_000)
        .expect("आरम्भः runs");
    let mut added = 0;
    for f in &files {
        added += collect_into_store(&mut st, f, &source(f));
    }

    let texts: Vec<(String, String)> = files.iter().map(|f| (f.clone(), source(f))).collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let rust = Interpreter::load(&refs, &repo_root().join("spec"))
        .unwrap_or_else(|e| panic!("the loader takes the corpus: {e:?}"))
        .declarations();
    let t1 = store_declarations(&mut st);

    let mut disagree: Vec<String> = Vec::new();
    for m in &rust {
        // `lib.t1` names no module and declares nothing the store keys.
        if m.declarations.is_empty() {
            continue;
        }
        let Some(s) = t1.iter().find(|s| s.name == m.name) else {
            disagree.push(format!("{}: in the loader, not in the store", m.name));
            continue;
        };
        let a: BTreeSet<&Declaration> = m.declarations.iter().collect();
        let b: BTreeSet<&Declaration> = s.declarations.iter().collect();
        for d in a.difference(&b) {
            disagree.push(format!(
                "{}: LOADER ONLY: {:?} {} `{}` public={}",
                m.name, d.kind, d.name, d.ty, d.public
            ));
        }
        for d in b.difference(&a) {
            disagree.push(format!(
                "{}: STORE ONLY: {:?} {} `{}` public={}",
                m.name, d.kind, d.name, d.ty, d.public
            ));
        }
    }
    assert!(
        disagree.is_empty(),
        "the T1 store and the loader disagree on {} declaration(s) — \
         a declaration reaching one and not the other is the 2026-09-07 defect \
         returning, and the names say which:\n  {}",
        disagree.len(),
        disagree.join("\n  ")
    );

    // THE POPULATION'S SIZE, BESIDE THE CLAIM ABOUT IT — PRINTED, NOT ASSERTED.
    //
    // A pinned figure here would be wrong in both available forms. An EQUALITY
    // is re-taken on nearly every `.t1` edit, and a ratchet re-taken that often
    // gets re-taken WITHOUT BEING READ — which is how the check this replaces
    // became something nobody looked at. A FLOOR has the defect's own shape:
    // lose five declarations while the corpus gains ten and the store reads
    // 1,311, above the floor, green, with the defect live. The original loss
    // was five out of ~1,300, so a floor would have caught it only if nothing
    // else moved, and something else is always moving.
    //
    // The assertion above has neither hole: a loader-only declaration is the
    // failure stated directly, it is scale-free, and the corpus can double
    // without touching it. This number is here so a reader sees the
    // population's size next to the claim — a population metric without it is
    // how a 436-site fall once read as progress when a module had stopped
    // assembling.
    println!("METRIC paradigm_store_entries {added}");
}
