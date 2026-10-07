//! **W-208 — CALL STATISTICS AND THE CALL GRAPH: the ट loop measured.**
//!
//! `paramtatva-stats-plan.md` §2.5 asks four numbers of the ट loop (व site,
//! र arguments, ट entry, त body, व् return — `research/22-two-conduits.md` §7):
//!
//! 16. **Return presence** — of the routines that declare a return type, how
//!     many carry a `प्रत्यागमनम्` on EVERY path; and whether the routines
//!     that declare none ever return a value.
//! 17. **Arguments per call** — the distribution, and the ARITY AGREEMENT rate
//!     between every call site and the declaration it names (W-193's defect
//!     class: the Rust body parser and the runtime disagreeing on a bare name).
//! 18. **The call graph** — out-degree, in-degree, and Tarjan's strongly
//!     connected components. Recursion is a ट loop that returns to its own
//!     entry, so every SCC of size > 1 and every self-edge is listed.
//! 19. **Return-to-site** — at every call site, is the result consumed (bound,
//!     assigned, passed, returned, compared, an operand, indexed) or DISCARDED
//!     (an expression statement whose expression is the call)?
//!
//! # The instrument
//!
//! The T1 parser itself, run by [`sadhana::t1::nirvahana`] over each of the
//! fifteen sources exactly as `measure_corpus_resolve` runs it, and its arenas
//! read back from Rust: `घोषणाकोश` (declarations, with `प्राचलादि..प्राचलान्त`
//! into `प्राचलकोश`), `वाक्यकोश` (statements) and `अभिव्यञ्जककोश`
//! (expressions), every one 1-based with slot ० reserved. Nothing here reads
//! the source text for a construct; a name is a token index and a call is an
//! `आह्वान` node. The interpreter's own reading of every declaration
//! (`Routine::arity`) is compared against the T1 parser's parameter range as
//! a check on the instrument, since the two are independent readers of one
//! text.
//!
//! # What "on every path" counts
//!
//! A statement ALWAYS RETURNS when it is a `प्रत्यागमनम्`, a block one of
//! whose direct statements always returns, or a `यदि` whose then-arm always
//! returns AND whose `अन्यथा` arm exists and always returns. A `यावत्` never
//! counts — its condition may be false on entry — and neither does anything
//! else. A routine returns on every path when its body block does.
//!
//! # The base this was measured on, and the else-arm
//!
//! `c72b3926`, which is BEFORE W-198. On this base `व्याकर`'s `यदि` arm parses
//! the `अन्यथा` block and DROPS its index (`parse.t1`, "अन्यथाशाखा
//! वैकल्पिकी"): the node has two child slots and spends them on the condition
//! and the then-body. The block is not lost, though — `समूहपठनम्` pushed it,
//! and it pushed it LAST, after the then-body and immediately before the
//! `यदि` node itself. So on this base the else-arm of `यदि` at index `i` is
//! the block at `i − 1` whenever `i − 1` is not the then-body. On W-198's base
//! the record carries `अन्यसूचकाङ्क` and that is read instead; the census
//! prints which of the two it used, so the number can never be misread.
//!
//! # Two spellings the census had to learn, both counted
//!
//! `मण्डल ॱ नाम` written WITH SPACES is three tokens — the lexer peels only a
//! leading or trailing `ॱ` — and `व्याकर` builds a `क्षेत्र` on the module
//! name. 127 corpus calls are spelled so (`सङ्केतन ॱ पङ्क्तिसीमा पाठ्यम्
//! आरम्भः`); they resolve here as the qualified call the interpreter also
//! reads them as. And `x अङ्कः a अन्तः b` is the interpreter's SLICE form
//! (`nirvahana.rs`, `Expr::Slice`), which `व्याकर` — having no slice
//! production — reads as an index applied to `b`; the 20 such sites are
//! counted apart and are not calls.
//!
//! # Measured on `c72b3926`, 2026-09-03
//!
//! 463 routines, every one declaring a return and every one returning on
//! every path; 1,326 returns. 1,470 call sites, mean 2.27 arguments, 1,452
//! agreeing with their declaration and 18 not — one class, a text passed as
//! its `(पाठ, अष्टक, पाठसीमा)` triple against a single-slice parameter, 16 in
//! `vakyavibhaga.t1` (seven already in `ARITY_FAULTS`), its inverse at
//! `:3294`, and one unbracketed nested call at `utsarjana.t1:1251`. 916
//! distinct edges; two SCCs of size 2, both in `व्याकर`; nine self-recursive
//! routines; 61 never called. 1,391 results consumed (94%), 79 discarded — 55
//! of them diagnostic appenders whose returned index nobody needs.
//!
//! # Measured again on the tree after W-227, 2026-09-04
//!
//! The eighteen were ONE corpus-side class and are repaired at their call
//! sites: the corpus declares 152 slice parameters (`अङ्कः अन्तः अ८`) and 32
//! `(text, start, end)` triples, so the slice is its own convention and a
//! triple handed to a slice parameter is an error, not an idiom for the
//! grammar to name. Each of the sixteen now passes the token's text as the
//! slice `x ॱ पाठ अङ्कः x ॱ अष्टक अन्तः x ॱ पाठसीमा` its callee declares
//! (its Rust twin's `&str`), rebased by `अष्टक` where the callee answers an
//! offset into it; the inverse passes each field as its own `s ऽ ० ऽ s ॱ
//! दैर्घ्य` triple; the nested call is bracketed. 1,519 sites, 1,519
//! agreeing, `paradigm_call_arity_disagreements` 18 → **0**, pinned below by
//! `arity_disagreements_are_pinned_at_zero_and_a_new_one_fails_by_name` and
//! run by `tools/check-t1-ratchets.sh` on the hourly deep gate. The corpus's
//! slice count moves with it: `paradigm_call_slices` 20 → **36**. W-228
//! (landed the same day) reads each as a `खण्डाभिव्यञ्जक`, so
//! `slices_read_as_calls` stays 0; sixteen of the thirty-six are these sites,
//! and W-228's ratchet below is re-pinned at 36 with that reason.
//!
//! # Red is visible
//!
//! Every count has a synthetic program beside it that makes it go the other
//! way — a call whose arity disagrees is COUNTED as a disagreement (and a
//! triple handed to a slice parameter is named as one, site by site), a routine
//! whose `यदि` has no else-arm is COUNTED as an exception, a call in statement
//! position is COUNTED as discarded, and `क → ख → क` is ONE component. A
//! census that could only report 100% would be an assertion wearing a
//! measurement's clothes.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ─────────────────────────────────────────────────────────────────────────
// The loader — the same five modules `measure_corpus_resolve` loads.
// ─────────────────────────────────────────────────────────────────────────

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn source(name: &str) -> String {
    let p = corpus_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// `व्याकर` needs `वास्तु` (the arenas), `पदविभाग` (the tokens) and
/// `अक्षरकोश` (numerals); `अर्थ` rides along because `parse.t1` imports it.
/// NOT `vakyavibhaga.t1`: it declares its own bare `वाक्यकोश` and
/// `दोषसूचकाङ्क`, and `Interpreter::global` looks names up bare.
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
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

fn arena(v: &Value) -> Vec<Value> {
    match v {
        Value::Arena(a) => a.borrow().clone(),
        other => panic!("{other:?} is not an arena"),
    }
}

/// A record's integer member, or `None` when the slot is `शून्यम्` (the
/// reserved ०) or the member does not exist on this base.
fn field_int(rec: &Value, name: &str) -> Option<i128> {
    match rec {
        Value::Record(r) => r.borrow().get(name).and_then(Value::as_int),
        _ => None,
    }
}

fn field_octets(rec: &Value, name: &str) -> Option<Vec<u8>> {
    match rec {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .and_then(Value::octets)
            .map(|o| o.as_slice().to_vec()),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The tree, as `ast.t1` numbers it.
// ─────────────────────────────────────────────────────────────────────────

const E_NAME: i128 = 1;
const E_GROUP: i128 = 4;
const E_INDEX: i128 = 5;
const E_FIELD: i128 = 6;
const E_CALL: i128 = 7;
const E_BIN: i128 = 8;
const E_NEG: i128 = 12;
/// `slice = index , postfix_expr` (W-228): left is the index node, right the limit.
const E_SLICE: i128 = 13;

const S_EXPR: i128 = 1;
const S_BLOCK: i128 = 2;
const S_LET: i128 = 3;
const S_RETURN: i128 = 4;
const S_IF: i128 = 6;
const S_WHILE: i128 = 7;
const S_ASSIGN: i128 = 8;

const D_ROUTINE: i128 = 1;
const D_STRUCT: i128 = 2;
const D_ENUM: i128 = 3;
const D_VAR: i128 = 4;

/// The five comparison operators of `ast.t1`'s thirteen-plus-two.
const COMPARISONS: [i128; 5] = [5, 6, 7, 8, 15];

/// The member mark inside a qualified token: `पदविभागॱचिह्नक` is ONE token.
const QUALIFIER: char = '\u{971}';

#[derive(Clone, Default)]
struct Tok {
    text: String,
    line: i128,
}

#[derive(Clone, Copy, Default)]
struct ExprN {
    kind: i128,
    value: i128,
    left: i128,
    right: i128,
    op: i128,
}

#[derive(Clone, Copy, Default)]
struct StmtN {
    kind: i128,
    left: i128,
    right: i128,
    start: i128,
    /// W-198's `अन्यसूचकाङ्क`, when the record carries it.
    els: Option<i128>,
}

#[derive(Clone, Copy, Default)]
struct DeclN {
    kind: i128,
    name: i128,
    ty: i128,
    body: i128,
    p_start: i128,
    p_end: i128,
}

/// One source, parsed by the T1 parser, its arenas copied out. Every `Vec`
/// keeps slot ० as a default so the arena's own indices address it directly.
///
/// `W-261`: `Clone` so [`parse_corpus`] can hand both ratchets in this binary
/// the same parse. ARENAS COPIED OUT is what makes that safe — see the margin
/// on `parse_corpus` in `t1_paradigm.rs` for the design and the hazard it
/// forecloses.
#[derive(Clone)]
struct FileModel {
    label: String,
    module: String,
    toks: Vec<Tok>,
    exprs: Vec<ExprN>,
    stmts: Vec<StmtN>,
    decls: Vec<DeclN>,
    /// `प्राचलकोश`: each parameter's name token.
    params: Vec<i128>,
    parse_errors: i128,
    /// Whether `वाक्य` records carry W-198's else slot.
    has_else_field: bool,
}

impl FileModel {
    fn tok(&self, i: i128) -> &Tok {
        static NONE: Tok = Tok {
            text: String::new(),
            line: 0,
        };
        usize::try_from(i)
            .ok()
            .and_then(|i| self.toks.get(i))
            .unwrap_or(&NONE)
    }
    fn expr(&self, i: i128) -> ExprN {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.exprs.get(i).copied())
            .unwrap_or_default()
    }
    /// The member token of a `क्षेत्र` node. `व्याकर` records `पठनस्थान` AFTER
    /// `अग्रिमम्` has read the member, so the token is one back.
    fn member_tok(&self, x: ExprN) -> &Tok {
        self.tok(x.value - 1)
    }

    /// The first token an expression subtree carries — for a diagnostic on a
    /// node that has none of its own.
    fn line_of(&self, e: i128) -> i128 {
        let x = self.expr(e);
        if x.kind == E_FIELD {
            return self.member_tok(x).line;
        }
        if x.value > 0 {
            return self.tok(x.value).line;
        }
        for child in [x.left, x.right] {
            if child > 0 {
                let l = self.line_of(child);
                if l > 0 {
                    return l;
                }
            }
        }
        0
    }

    fn stmt(&self, i: i128) -> StmtN {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.stmts.get(i).copied())
            .unwrap_or_default()
    }

    /// A block's DIRECT children, top to bottom, by the walk `वास्तुॱवाक्य`'s
    /// margin prescribes: from `अन्तिम` back to `प्रथम`, each step skipping
    /// the previous statement's whole subtree via `आदिसूचकाङ्क`.
    fn children(&self, block: i128) -> Vec<i128> {
        let b = self.stmt(block);
        let mut out = Vec::new();
        if b.kind != S_BLOCK || b.right == 0 || b.left == 0 {
            return out;
        }
        let mut cur = b.right;
        while cur >= b.left && cur > 0 {
            out.push(cur);
            if cur == b.left {
                break;
            }
            cur = self.stmt(cur).start - 1;
        }
        out.reverse();
        out
    }

    /// The `अन्यथा` block of a `यदि`, by W-198's field where it exists and by
    /// arena adjacency on the base before it (see the module header).
    fn else_of(&self, i: i128) -> Option<i128> {
        let s = self.stmt(i);
        if s.kind != S_IF {
            return None;
        }
        if let Some(e) = s.els {
            return (e > 0).then_some(e);
        }
        let then = s.right;
        let prev = i - 1;
        (then > 0 && prev > then && self.stmt(prev).kind == S_BLOCK).then_some(prev)
    }

    fn always_returns(&self, i: i128) -> bool {
        let s = self.stmt(i);
        match s.kind {
            S_RETURN => true,
            S_BLOCK => self.children(i).iter().any(|c| self.always_returns(*c)),
            S_IF => {
                s.right > 0
                    && self.always_returns(s.right)
                    && self.else_of(i).is_some_and(|e| self.always_returns(e))
            }
            _ => false,
        }
    }
}

/// Lex and parse one source with the T1 parser and copy its arenas out.
fn parse_t1(label: &str, src: &str) -> Result<FileModel, String> {
    let mut it = load_parser();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .map_err(|e| format!("lex: {e:?}"))?
        .as_int()
        .ok_or("lex answered no count")?;
    let decls = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .map_err(|e| format!("parse: {e:?}"))?
        .as_int()
        .ok_or("parse answered no count")?;
    // THREE STATES, NOT TWO: a missing global means THIS CENSUS is broken.
    let parse_errors = it
        .global("दोषसूचकाङ्क")
        .and_then(Value::as_int)
        .ok_or("the global दोषसूचकाङ्क does not exist — this census is broken")?;
    let global = |name: &str| -> Result<Vec<Value>, String> {
        it.global(name)
            .map(arena)
            .ok_or_else(|| format!("the global {name} does not exist — this census is broken"))
    };

    let mut tokens = vec![Tok::default()];
    for (i, rec) in global("चिह्नककोश")?.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if i > usize::try_from(toks).unwrap_or(0) {
            break;
        }
        let text = field_octets(rec, "पाठ").unwrap_or_default();
        let a = usize::try_from(field_int(rec, "अष्टक").unwrap_or(0)).unwrap_or(0);
        let b = usize::try_from(field_int(rec, "पाठसीमा").unwrap_or(0)).unwrap_or(0);
        let piece = text.get(a..b).unwrap_or(&[]);
        tokens.push(Tok {
            text: String::from_utf8_lossy(piece).into_owned(),
            line: field_int(rec, "पङ्क्ति").unwrap_or(0),
        });
    }
    let module = tokens
        .iter()
        .position(|t| t.text == "मण्डलम्")
        .and_then(|p| tokens.get(p + 1))
        .map(|t| t.text.clone())
        .unwrap_or_default();

    let exprs: Vec<ExprN> = global("अभिव्यञ्जककोश")?
        .iter()
        .map(|r| ExprN {
            kind: field_int(r, "भेद").unwrap_or(0),
            value: field_int(r, "मूल्यसूचकाङ्क").unwrap_or(0),
            left: field_int(r, "वामसूचकाङ्क").unwrap_or(0),
            right: field_int(r, "दक्षिणसूचकाङ्क").unwrap_or(0),
            op: field_int(r, "द्विकर्म").unwrap_or(0),
        })
        .collect();
    let stmt_records = global("वाक्यकोश")?;
    let has_else_field = stmt_records
        .iter()
        .any(|r| matches!(r, Value::Record(rec) if rec.borrow().contains_key("अन्यसूचकाङ्क")));
    let stmts: Vec<StmtN> = stmt_records
        .iter()
        .map(|r| StmtN {
            kind: field_int(r, "भेद").unwrap_or(0),
            left: field_int(r, "वामसूचकाङ्क").unwrap_or(0),
            right: field_int(r, "दक्षिणसूचकाङ्क").unwrap_or(0),
            start: field_int(r, "आदिसूचकाङ्क").unwrap_or(0),
            els: field_int(r, "अन्यसूचकाङ्क"),
        })
        .collect();
    let decl_records = global("घोषणाकोश")?;
    let decls: Vec<DeclN> = decl_records
        .iter()
        .take(usize::try_from(decls).unwrap_or(0) + 1)
        .map(|r| DeclN {
            kind: field_int(r, "भेद").unwrap_or(0),
            name: field_int(r, "नामसूचकाङ्क").unwrap_or(0),
            ty: field_int(r, "प्रकारसूचकाङ्क").unwrap_or(0),
            body: field_int(r, "शरीरसूचकाङ्क").unwrap_or(0),
            p_start: field_int(r, "प्राचलादि").unwrap_or(0),
            p_end: field_int(r, "प्राचलान्त").unwrap_or(0),
        })
        .collect();
    let params: Vec<i128> = global("प्राचलकोश")?
        .iter()
        .map(|r| field_int(r, "नामसूचकाङ्क").unwrap_or(0))
        .collect();

    Ok(FileModel {
        label: label.to_string(),
        module,
        toks: tokens,
        exprs,
        stmts,
        decls,
        params,
        parse_errors,
        has_else_field,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// The census over a set of parsed files.
// ─────────────────────────────────────────────────────────────────────────

/// How a call's result is used at its site — the व of the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Use {
    /// An expression statement: the value goes nowhere.
    Discarded,
    /// `चरः x ॱॱ T भवति <call>`.
    Bound,
    /// `place भवति <call>`.
    Assigned,
    /// An argument of another call.
    Passed,
    /// `प्रत्यागमनम् <call>`.
    Returned,
    /// The condition of a `यदि` or `यावत्`.
    Condition,
    /// An operand of one of the five comparisons.
    Compared,
    /// An operand of an arithmetic or bitwise operator, or of `ऋण`.
    Operand,
    /// Indexed, sliced or member-read.
    Indexed,
    /// The callee position of another call (a call returning a callee — the
    /// corpus has none, and it is counted so that it cannot hide).
    Callee,
    /// The place of an assignment, or an index inside one.
    Place,
}

impl Use {
    fn is_consumed(self) -> bool {
        self != Self::Discarded
    }
    fn label(self) -> &'static str {
        match self {
            Self::Discarded => "discarded",
            Self::Bound => "bound",
            Self::Assigned => "assigned",
            Self::Passed => "passed",
            Self::Returned => "returned",
            Self::Condition => "condition",
            Self::Compared => "compared",
            Self::Operand => "operand",
            Self::Indexed => "indexed",
            Self::Callee => "callee",
            Self::Place => "place",
        }
    }
}

#[derive(Clone, Debug)]
struct RoutineInfo {
    file: usize,
    module: String,
    name: String,
    line: i128,
    params: usize,
    /// The parameter range in `प्राचलकोश`, inclusive; `0..=0` means none.
    p_start: i128,
    p_end: i128,
    declares_return: bool,
    body: i128,
}

#[derive(Clone, Debug)]
struct Site {
    caller: usize,
    callee: usize,
    args: usize,
    file: usize,
    line: i128,
    use_: Use,
}

#[derive(Default)]
struct Census {
    routines: Vec<RoutineInfo>,
    sites: Vec<Site>,
    /// Call heads whose callee is not a routine this census can name.
    unresolved: Vec<String>,
    /// Bare names that are neither local, routine nor module global.
    unknown_names: BTreeMap<String, usize>,
    /// Bare names that resolve only to ANOTHER module's routine.
    cross_module_bare: Vec<String>,
    /// Slice forms the T1 parser read as a call on an index expression.
    slice_read_as_call: Vec<String>,
    /// Slice nodes the T1 parser built — `W-228`'s `खण्डाभिव्यञ्जक`, by site.
    slices: Vec<String>,
    /// Spaced qualifiers the T1 parser still reads as a member on a module
    /// name — the shape `W-228` folds away; by site.
    spaced_as_field: Vec<String>,
    /// `(module, name)` declared as a routine more than once.
    duplicate_routines: Vec<String>,
    /// (16)
    returns_on_every_path: Vec<usize>,
    return_exceptions: Vec<usize>,
    no_return_type: Vec<(usize, usize)>,
    returns_total: usize,
    /// (17)
    arity_agree: usize,
    arity_disagree: Vec<String>,
    args_histogram: BTreeMap<usize, usize>,
    /// (18)
    out_degree: Vec<usize>,
    in_degree: Vec<usize>,
    sccs: Vec<Vec<usize>>,
    self_recursive: Vec<usize>,
    /// (19)
    use_histogram: BTreeMap<Use, usize>,
    discarded: Vec<String>,
}

impl Census {
    fn routine_label(&self, r: usize) -> String {
        let x = &self.routines[r];
        format!("{}ॱ{}", x.module, x.name)
    }
}

/// Where a walk is: which file, which routine, and the names its body binds.
struct Scope<'a> {
    file: usize,
    caller: usize,
    locals: &'a HashSet<String>,
}

struct Walker<'a> {
    files: &'a [FileModel],
    /// Every module name the parsed files declare — what a spaced qualifier
    /// (`सङ्केतन ॱ पङ्क्तिसीमा`, three tokens) must name on its left.
    modules: &'a HashSet<String>,
    by_qualified: &'a HashMap<(String, String), Vec<usize>>,
    by_bare: &'a HashMap<String, Vec<usize>>,
    globals: &'a HashSet<(String, String)>,
}

impl Walker<'_> {
    /// Every expression a statement owns directly, with the use each gets.
    fn statement_roots(&self, m: &FileModel, s: i128) -> Vec<(i128, Use)> {
        let st = m.stmt(s);
        match st.kind {
            S_EXPR => vec![(st.left, Use::Discarded)],
            S_LET => vec![(st.right, Use::Bound)],
            S_RETURN => vec![(st.left, Use::Returned)],
            S_IF | S_WHILE => vec![(st.left, Use::Condition)],
            S_ASSIGN => vec![(st.left, Use::Place), (st.right, Use::Assigned)],
            _ => Vec::new(),
        }
    }

    /// `मण्डल ॱ नाम` written with spaces is a `क्षेत्र` on a bare name; when
    /// that name is a module (and not a local of the same spelling) the pair
    /// is one qualified name, as the interpreter also reads it.
    fn qualified_member(
        &self,
        m: &FileModel,
        x: ExprN,
        locals: &HashSet<String>,
    ) -> Option<String> {
        let obj = m.expr(x.left);
        if x.kind != E_FIELD || obj.kind != E_NAME {
            return None;
        }
        let module = &m.tok(obj.value).text;
        if locals.contains(module) || !self.modules.contains(module) {
            return None;
        }
        Some(format!("{module}{QUALIFIER}{}", m.member_tok(x).text))
    }

    fn resolve(&self, module: &str, text: &str, locals: &HashSet<String>) -> Resolved {
        if let Some((m, n)) = text.split_once(QUALIFIER) {
            if let Some(ids) = self.by_qualified.get(&(m.to_string(), n.to_string())) {
                return Resolved::Routine(ids[0]);
            }
            if self.globals.contains(&(m.to_string(), n.to_string())) {
                return Resolved::Global;
            }
            return Resolved::Unknown;
        }
        if locals.contains(text) {
            return Resolved::Local;
        }
        if let Some(ids) = self
            .by_qualified
            .get(&(module.to_string(), text.to_string()))
        {
            return Resolved::Routine(ids[0]);
        }
        if self
            .globals
            .contains(&(module.to_string(), text.to_string()))
        {
            return Resolved::Global;
        }
        if let Some(ids) = self.by_bare.get(text) {
            return Resolved::CrossModuleBare(ids[0]);
        }
        Resolved::Unknown
    }

    fn visit(&self, sc: &Scope<'_>, e: i128, use_: Use, c: &mut Census) {
        if e == 0 {
            return;
        }
        let m = &self.files[sc.file];
        let x = m.expr(e);
        match x.kind {
            E_CALL => {
                // Walk the curried chain down to its callee: (((f a) b) c).
                let mut args = Vec::new();
                let mut cur = e;
                while m.expr(cur).kind == E_CALL {
                    args.push(m.expr(cur).right);
                    cur = m.expr(cur).left;
                }
                args.reverse();
                for a in &args {
                    self.visit(sc, *a, Use::Passed, c);
                }
                let callee = m.expr(cur);
                if callee.kind == E_NAME {
                    // `W-228` (b): a spaced qualifier folds into the name node,
                    // its `दक्षिणसूचकाङ्क` naming the member token.
                    let tok = m.tok(callee.value);
                    let text = if callee.right != 0 {
                        format!("{}{QUALIFIER}{}", tok.text, m.tok(callee.right).text)
                    } else {
                        tok.text.clone()
                    };
                    self.site(sc, &text, tok.line, args.len(), use_, c);
                } else if let Some(q) = self.qualified_member(m, callee, sc.locals) {
                    c.spaced_as_field.push(format!(
                        "{}:{} {q}",
                        m.label,
                        m.member_tok(callee).line
                    ));
                    self.site(sc, &q, m.member_tok(callee).line, args.len(), use_, c);
                } else if callee.kind == E_INDEX {
                    // `x अङ्कः a अन्तः b` is the interpreter's SLICE form
                    // (`nirvahana.rs`, `Expr::Slice`); `व्याकर` has no slice
                    // production and reads it as an index applied to `b`.
                    // Not a call — counted so the two readers' difference
                    // is a number.
                    c.slice_read_as_call
                        .push(format!("{}:{}", m.label, m.line_of(cur)));
                    self.visit(sc, cur, Use::Indexed, c);
                } else {
                    c.unresolved.push(format!(
                        "{}:{} a call whose callee is an expression of kind {} with {} argument(s)",
                        m.label,
                        m.line_of(cur),
                        callee.kind,
                        args.len()
                    ));
                    self.visit(sc, cur, Use::Callee, c);
                }
            }
            E_NAME => {
                let tok = m.tok(x.value);
                let text = if x.right != 0 {
                    format!("{}{QUALIFIER}{}", tok.text, m.tok(x.right).text)
                } else {
                    tok.text.clone()
                };
                self.site(sc, &text, tok.line, 0, use_, c);
            }
            E_SLICE => {
                c.slices.push(format!("{}:{}", m.label, m.line_of(x.left)));
                self.visit(sc, x.left, use_, c);
                self.visit(sc, x.right, Use::Operand, c);
            }
            E_GROUP => self.visit(sc, x.left, use_, c),
            E_NEG => self.visit(sc, x.left, Use::Operand, c),
            E_INDEX => {
                let inner = if use_ == Use::Place {
                    Use::Place
                } else {
                    Use::Indexed
                };
                self.visit(sc, x.left, inner, c);
                self.visit(sc, x.right, Use::Operand, c);
            }
            E_FIELD => {
                if let Some(q) = self.qualified_member(m, x, sc.locals) {
                    c.spaced_as_field
                        .push(format!("{}:{} {q}", m.label, m.member_tok(x).line));
                    self.site(sc, &q, m.member_tok(x).line, 0, use_, c);
                    return;
                }
                let inner = if use_ == Use::Place {
                    Use::Place
                } else {
                    Use::Indexed
                };
                self.visit(sc, x.left, inner, c);
            }
            E_BIN => {
                let inner = if COMPARISONS.contains(&x.op) {
                    Use::Compared
                } else {
                    Use::Operand
                };
                self.visit(sc, x.left, inner, c);
                self.visit(sc, x.right, inner, c);
            }
            _ => {}
        }
    }

    /// A name in value position: a local, a global, or a call with `args`
    /// arguments (a bare routine name is a call with none).
    fn site(&self, sc: &Scope<'_>, text: &str, line: i128, args: usize, use_: Use, c: &mut Census) {
        let m = &self.files[sc.file];
        let resolved = self.resolve(&m.module, text, sc.locals);
        if matches!(resolved, Resolved::CrossModuleBare(_)) {
            c.cross_module_bare
                .push(format!("{}:{} `{text}`", m.label, line));
        }
        match resolved {
            Resolved::Routine(callee) | Resolved::CrossModuleBare(callee) => {
                c.sites.push(Site {
                    caller: sc.caller,
                    callee,
                    args,
                    file: sc.file,
                    line,
                    use_,
                });
            }
            Resolved::Local | Resolved::Global => {
                if args > 0 {
                    c.unresolved.push(format!(
                        "{}:{} `{}` is a local or global applied to {} argument(s)",
                        m.label, line, text, args
                    ));
                }
            }
            Resolved::Unknown => {
                if args > 0 {
                    c.unresolved.push(format!(
                        "{}:{} `{}` names no routine and is applied to {} argument(s)",
                        m.label, line, text, args
                    ));
                }
                *c.unknown_names.entry(text.to_string()).or_insert(0) += 1;
            }
        }
    }
}

enum Resolved {
    Local,
    Global,
    Routine(usize),
    CrossModuleBare(usize),
    Unknown,
}

fn tarjan(n: usize, edges: &[BTreeSet<usize>]) -> Vec<Vec<usize>> {
    struct T<'a> {
        edges: &'a [BTreeSet<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn go(t: &mut T<'_>, v: usize) {
        t.index[v] = Some(t.next);
        t.low[v] = t.next;
        t.next += 1;
        t.stack.push(v);
        t.on[v] = true;
        let succ: Vec<usize> = t.edges[v].iter().copied().collect();
        for w in succ {
            if t.index[w].is_none() {
                go(t, w);
                t.low[v] = t.low[v].min(t.low[w]);
            } else if t.on[w] {
                t.low[v] = t.low[v].min(t.index[w].unwrap_or(usize::MAX));
            }
        }
        if t.index[v] == Some(t.low[v]) {
            let mut comp = Vec::new();
            loop {
                let w = t.stack.pop().expect("a member on the stack");
                t.on[w] = false;
                comp.push(w);
                if w == v {
                    break;
                }
            }
            comp.sort_unstable();
            t.out.push(comp);
        }
    }
    let mut t = T {
        edges,
        index: vec![None; n],
        low: vec![0; n],
        on: vec![false; n],
        stack: Vec::new(),
        next: 0,
        out: Vec::new(),
    };
    for v in 0..n {
        if t.index[v].is_none() {
            go(&mut t, v);
        }
    }
    t.out
}

fn census(files: &[FileModel]) -> Census {
    let mut c = Census::default();

    // The routine table across every file — `(module, name)`, and bare.
    let mut by_qualified: HashMap<(String, String), Vec<usize>> = HashMap::new();
    let mut by_bare: HashMap<String, Vec<usize>> = HashMap::new();
    let mut globals: HashSet<(String, String)> = HashSet::new();
    for (fi, m) in files.iter().enumerate() {
        for d in m.decls.iter().skip(1) {
            let name = m.tok(d.name).text.clone();
            match d.kind {
                D_ROUTINE => {
                    let params = if d.p_start == 0 {
                        0
                    } else {
                        usize::try_from(d.p_end - d.p_start + 1).unwrap_or(0)
                    };
                    let id = c.routines.len();
                    c.routines.push(RoutineInfo {
                        file: fi,
                        module: m.module.clone(),
                        name: name.clone(),
                        line: m.tok(d.name).line,
                        params,
                        p_start: d.p_start,
                        p_end: d.p_end,
                        declares_return: d.ty != 0,
                        body: d.body,
                    });
                    by_qualified
                        .entry((m.module.clone(), name.clone()))
                        .or_default()
                        .push(id);
                    by_bare.entry(name).or_default().push(id);
                }
                D_VAR | D_STRUCT | D_ENUM => {
                    globals.insert((m.module.clone(), name));
                }
                _ => {}
            }
        }
    }
    for ((m, n), ids) in &by_qualified {
        if ids.len() > 1 {
            c.duplicate_routines.push(format!("{m}ॱ{n} ×{}", ids.len()));
        }
    }
    c.duplicate_routines.sort();

    // (16) and the walk for (17)/(19).
    let routines = c.routines.clone();
    let modules: HashSet<String> = files.iter().map(|f| f.module.clone()).collect();
    let w = Walker {
        files,
        modules: &modules,
        by_qualified: &by_qualified,
        by_bare: &by_bare,
        globals: &globals,
    };
    for (rid, r) in routines.iter().enumerate() {
        let m = &files[r.file];
        if r.body == 0 {
            if r.declares_return {
                c.return_exceptions.push(rid);
            } else {
                c.no_return_type.push((rid, 0));
            }
            continue;
        }
        let range = m.stmt(r.body).start..=r.body;
        // Locals are flow-insensitive: a `चरः` anywhere in the body, plus the
        // parameters. A name is looked up here before the routine table.
        let mut locals: HashSet<String> = HashSet::new();
        if r.p_start > 0 {
            for p in r.p_start..=r.p_end {
                let t = usize::try_from(p)
                    .ok()
                    .and_then(|p| m.params.get(p).copied());
                if let Some(t) = t {
                    locals.insert(m.tok(t).text.clone());
                }
            }
        }
        let mut returns = 0;
        for s in range.clone() {
            let st = m.stmt(s);
            if st.kind == S_LET {
                locals.insert(m.tok(st.left).text.clone());
            }
            if st.kind == S_RETURN {
                returns += 1;
            }
        }
        c.returns_total += returns;
        if r.declares_return {
            if m.always_returns(r.body) {
                c.returns_on_every_path.push(rid);
            } else {
                c.return_exceptions.push(rid);
            }
        } else {
            c.no_return_type.push((rid, returns));
        }
        let sc = Scope {
            file: r.file,
            caller: rid,
            locals: &locals,
        };
        for s in range {
            for (e, use_) in w.statement_roots(m, s) {
                w.visit(&sc, e, use_, &mut c);
            }
        }
    }

    // (17) arity agreement, and the distribution of arguments per call.
    for s in &c.sites {
        *c.args_histogram.entry(s.args).or_insert(0) += 1;
        let callee = &c.routines[s.callee];
        if s.args == callee.params {
            c.arity_agree += 1;
        } else {
            c.arity_disagree.push(format!(
                "{}:{} calls {}ॱ{} with {} argument(s); it declares {}",
                files[s.file].label, s.line, callee.module, callee.name, s.args, callee.params
            ));
        }
    }

    // (18) degrees and components, over DISTINCT callees.
    let n = c.routines.len();
    let mut edges: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for s in &c.sites {
        edges[s.caller].insert(s.callee);
    }
    c.out_degree = edges.iter().map(BTreeSet::len).collect();
    c.in_degree = vec![0; n];
    for (v, succ) in edges.iter().enumerate() {
        for w in succ {
            c.in_degree[*w] += 1;
        }
        if succ.contains(&v) {
            c.self_recursive.push(v);
        }
    }
    let mut sccs: Vec<Vec<usize>> = tarjan(n, &edges)
        .into_iter()
        .filter(|s| s.len() > 1)
        .collect();
    sccs.sort_by_key(|s| (std::cmp::Reverse(s.len()), s[0]));
    c.sccs = sccs;

    // (19) return-to-site.
    for s in &c.sites {
        *c.use_histogram.entry(s.use_).or_insert(0) += 1;
        if !s.use_.is_consumed() {
            let callee = &c.routines[s.callee];
            c.discarded.push(format!(
                "{}:{} {}ॱ{} ({} argument(s))",
                files[s.file].label, s.line, callee.module, callee.name, s.args
            ));
        }
    }
    c
}

fn metric(name: &str, value: impl std::fmt::Display) {
    println!("METRIC paradigm_call_{name} {value}");
}

fn print_census(c: &Census, files: &[FileModel]) {
    let else_source = if files.iter().any(|f| f.has_else_field) {
        "field"
    } else {
        "adjacency"
    };
    metric("sources", files.len());
    metric(
        "sources_with_parse_errors",
        files.iter().filter(|f| f.parse_errors > 0).count(),
    );
    metric("else_arm_source", else_source);
    metric("routines", c.routines.len());
    metric("routines_duplicate_names", c.duplicate_routines.len());
    for d in &c.duplicate_routines {
        println!("  duplicate: {d}");
    }

    // (16)
    let declared = c.returns_on_every_path.len() + c.return_exceptions.len();
    metric("routines_declaring_return", declared);
    metric(
        "routines_returning_on_every_path",
        c.returns_on_every_path.len(),
    );
    metric(
        "routines_missing_return_on_some_path",
        c.return_exceptions.len(),
    );
    for r in &c.return_exceptions {
        let x = &c.routines[*r];
        println!(
            "  missing return on some path: {}:{} {}",
            files[x.file].label,
            x.line,
            c.routine_label(*r)
        );
    }
    metric("routines_without_return_type", c.no_return_type.len());
    metric(
        "routines_without_return_type_that_return",
        c.no_return_type.iter().filter(|(_, n)| *n > 0).count(),
    );
    for (r, n) in &c.no_return_type {
        let x = &c.routines[*r];
        println!(
            "  no ददाति: {}:{} {} — {} प्रत्यागमनम्",
            files[x.file].label,
            x.line,
            c.routine_label(*r),
            n
        );
    }
    metric("returns_total", c.returns_total);

    // (17)
    let sites = c.sites.len();
    metric("sites", sites);
    metric("sites_unresolved", c.unresolved.len());
    for u in &c.unresolved {
        println!("  unresolved: {u}");
    }
    metric("slices_read_as_calls", c.slice_read_as_call.len());
    metric("slices", c.slices.len());
    metric(
        "calls_spaced_qualifier_read_as_field",
        c.spaced_as_field.len(),
    );
    for u in &c.spaced_as_field {
        println!("  spaced qualifier read as a field: {u}");
    }
    for u in &c.slice_read_as_call {
        println!("  slice read as a call: {u}");
    }
    metric("sites_bare_cross_module", c.cross_module_bare.len());
    for u in &c.cross_module_bare {
        println!("  bare cross-module: {u}");
    }
    metric("names_unknown_distinct", c.unknown_names.len());
    let mut unknown: Vec<(&String, &usize)> = c.unknown_names.iter().collect();
    unknown.sort_by_key(|(n, k)| (std::cmp::Reverse(**k), (*n).clone()));
    for (n, k) in unknown.iter().take(12) {
        println!("  unknown name: {n} ×{k}");
    }
    for (k, v) in &c.args_histogram {
        metric(&format!("sites_with_{k}_args"), v);
    }
    let total_args: usize = c.sites.iter().map(|s| s.args).sum();
    metric(
        "args_per_site_mean_x100",
        if sites == 0 {
            0
        } else {
            total_args * 100 / sites
        },
    );
    metric("arity_agreements", c.arity_agree);
    metric("arity_disagreements", c.arity_disagree.len());
    metric(
        "arity_agreement_rate_x100",
        if sites == 0 {
            0
        } else {
            c.arity_agree * 100 / sites
        },
    );
    for d in &c.arity_disagree {
        println!("  arity disagreement: {d}");
    }

    // (18)
    let n = c.routines.len();
    let edges: usize = c.out_degree.iter().sum();
    metric("graph_edges_distinct", edges);
    metric(
        "out_degree_max",
        c.out_degree.iter().copied().max().unwrap_or(0),
    );
    metric(
        "in_degree_max",
        c.in_degree.iter().copied().max().unwrap_or(0),
    );
    metric(
        "out_degree_mean_x100",
        if n == 0 { 0 } else { edges * 100 / n },
    );
    metric(
        "routines_calling_nothing",
        c.out_degree.iter().filter(|d| **d == 0).count(),
    );
    let never_called: Vec<usize> = (0..n).filter(|r| c.in_degree[*r] == 0).collect();
    metric("routines_never_called", never_called.len());
    let mut by_in: Vec<usize> = (0..n).collect();
    by_in.sort_by_key(|r| (std::cmp::Reverse(c.in_degree[*r]), *r));
    for r in by_in.iter().take(10) {
        println!(
            "  in-degree {:>3}: {}",
            c.in_degree[*r],
            c.routine_label(*r)
        );
    }
    let mut by_out: Vec<usize> = (0..n).collect();
    by_out.sort_by_key(|r| (std::cmp::Reverse(c.out_degree[*r]), *r));
    for r in by_out.iter().take(5) {
        println!(
            "  out-degree {:>3}: {}",
            c.out_degree[*r],
            c.routine_label(*r)
        );
    }
    metric("sccs_larger_than_one", c.sccs.len());
    metric(
        "routines_in_sccs_larger_than_one",
        c.sccs.iter().map(Vec::len).sum::<usize>(),
    );
    metric("scc_largest", c.sccs.first().map(Vec::len).unwrap_or(0));
    for (i, s) in c.sccs.iter().enumerate() {
        let members: Vec<String> = s.iter().map(|r| c.routine_label(*r)).collect();
        println!("  scc {} (size {}): {}", i + 1, s.len(), members.join(" "));
    }
    metric("routines_self_recursive", c.self_recursive.len());
    for r in &c.self_recursive {
        println!("  self-recursive: {}", c.routine_label(*r));
    }
    let recursive: BTreeSet<usize> = c
        .sccs
        .iter()
        .flatten()
        .chain(c.self_recursive.iter())
        .copied()
        .collect();
    metric("routines_recursive_any", recursive.len());

    // (19)
    for (u, k) in &c.use_histogram {
        metric(&format!("sites_{}", u.label()), k);
    }
    let consumed = sites - c.discarded.len();
    metric("sites_consumed", consumed);
    metric("sites_discarded", c.discarded.len());
    metric(
        "return_to_site_rate_x100",
        if sites == 0 {
            0
        } else {
            consumed * 100 / sites
        },
    );
    for d in &c.discarded {
        println!("  discarded: {d}");
    }
}

/// The interpreter's own reading of every declaration, keyed as the census
/// keys them, for the instrument check in the corpus census.
fn twin_arities() -> HashMap<(String, String), usize> {
    let mut names: Vec<String> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    let texts: Vec<(String, String)> = names.iter().map(|n| (n.clone(), source(n))).collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let it = Interpreter::load(&refs, &spec_root()).expect("all fifteen sources load");
    it.routines()
        .map(|r| ((r.module.clone(), r.name.clone()), r.arity()))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// The census.
// ─────────────────────────────────────────────────────────────────────────

/// Statistics 16–19 of the plan over the fifteen sources. Reports rather than
/// asserts, as every census in this crate does until a number is worth
/// defending; the tests below are what make its counts trustworthy.
#[test]
#[ignore = "measurement"]
fn measure_corpus_calls() {
    let mut names: Vec<String> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    let mut files = Vec::new();
    for n in &names {
        let src = source(n);
        let t = std::time::Instant::now();
        match parse_t1(n, &src) {
            Ok(m) => {
                eprintln!(
                    "  {n:24} module {:12} {:>4} decls {:>5} stmts {:>6} exprs  parse errors {}  {:>5.1}s",
                    m.module,
                    m.decls.len().saturating_sub(1),
                    m.stmts.len().saturating_sub(1),
                    m.exprs.len().saturating_sub(1),
                    m.parse_errors,
                    t.elapsed().as_secs_f64()
                );
                files.push(m);
            }
            Err(e) => eprintln!("  {n:24} NOT PARSED — {e}"),
        }
    }
    let c = census(&files);
    print_census(&c, &files);

    // THE INSTRUMENT CHECK. Two independent readers of the same declaration
    // — the T1 parser's parameter range and the Rust interpreter's `params`
    // — must count the same. A disagreement here is in a reader, not the
    // corpus, and it would make every arity number above suspect.
    let twin = twin_arities();
    let mut twin_disagree = Vec::new();
    for r in &c.routines {
        match twin.get(&(r.module.clone(), r.name.clone())) {
            Some(k) if *k == r.params => {}
            Some(k) => twin_disagree.push(format!(
                "{}ॱ{}: T1 parser {} vs interpreter {}",
                r.module, r.name, r.params, k
            )),
            None => twin_disagree.push(format!(
                "{}ॱ{}: the interpreter did not load it",
                r.module, r.name
            )),
        }
    }
    metric("declaration_readers_disagree", twin_disagree.len());
    for d in &twin_disagree {
        println!("  readers disagree: {d}");
    }
    metric("routines_by_interpreter", twin.len());
}

// ─────────────────────────────────────────────────────────────────────────
// The tests that let the census see red.
// ─────────────────────────────────────────────────────────────────────────

fn synthetic(src: &str) -> Census {
    let m =
        parse_t1("synthetic", src).unwrap_or_else(|e| panic!("the synthetic program parses: {e}"));
    assert_eq!(
        m.parse_errors, 0,
        "the synthetic program has no parse errors"
    );
    census(&[m])
}

/// A call with MORE arguments than its declaration, in both spellings, is
/// counted as a disagreement — and a correct call beside it is not.
#[test]
fn a_call_whose_arity_disagrees_is_counted_as_a_disagreement() {
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क आदाय ख ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् ख ।\n\
         इति\n\
         सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
         \x20   चरः घ ॱॱ न६४ भवति क आरभ्य १ ऽ २ समाप्तम् ।\n\
         \x20   चरः ङ ॱॱ न६४ भवति क १ २ ।\n\
         \x20   प्रत्यागमनम् क घ ।\n\
         इति\n",
    );
    assert_eq!(c.sites.len(), 3, "three call sites: {:?}", c.sites);
    assert_eq!(
        (c.arity_agree, c.arity_disagree.len()),
        (1, 2),
        "one agreement and two disagreements: {:?}",
        c.arity_disagree
    );
    assert!(
        c.arity_disagree
            .iter()
            .all(|d| d.contains("with 2 argument(s); it declares 1")),
        "each disagreement names the two counts: {:?}",
        c.arity_disagree
    );
    assert_eq!(c.args_histogram.get(&1), Some(&1));
    assert_eq!(c.args_histogram.get(&2), Some(&2));
}

/// A bare routine name with no argument is a call with none — and when the
/// routine declares parameters, that too is a disagreement.
#[test]
fn a_bare_routine_name_is_a_call_with_no_arguments() {
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् १ ।\n\
         इति\n\
         सार्वजनिक वृत्तिः ख आदाय ग ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् ग ।\n\
         इति\n\
         सार्वजनिक वृत्तिः घ ददाति न६४ आदि\n\
         \x20   चरः ङ ॱॱ न६४ भवति क ।\n\
         \x20   प्रत्यागमनम् ङ योगः ख ।\n\
         इति\n",
    );
    assert_eq!(c.sites.len(), 2, "{:?}", c.sites);
    assert_eq!(
        (c.arity_agree, c.arity_disagree.len()),
        (1, 1),
        "{:?}",
        c.arity_disagree
    );
    assert!(c.arity_disagree[0].contains("ख with 0 argument(s); it declares 1"));
}

/// Return presence: both arms returning is a return on every path; a
/// then-arm alone is not; a return inside a `यावत्` is not; a return after
/// the `यदि` rescues it. The else-arm is found on THIS base by arena
/// adjacency, and this is the test that pins that reading.
#[test]
fn a_return_in_both_arms_is_a_return_on_every_path_and_one_arm_is_not() {
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क आदाय ख ॱॱ बूल ददाति न६४ आदि\n\
         \x20   यदि ख समम् सत्यम् आदि\n\
         \x20       प्रत्यागमनम् १ ।\n\
         \x20   इति अन्यथा आदि\n\
         \x20       यदि ख समम् असत्यम् आदि\n\
         \x20           प्रत्यागमनम् २ ।\n\
         \x20       इति अन्यथा आदि\n\
         \x20           प्रत्यागमनम् ३ ।\n\
         \x20       इति\n\
         \x20   इति\n\
         इति\n\
         सार्वजनिक वृत्तिः ग आदाय ख ॱॱ बूल ददाति न६४ आदि\n\
         \x20   यदि ख समम् सत्यम् आदि\n\
         \x20       प्रत्यागमनम् १ ।\n\
         \x20   इति\n\
         इति\n\
         सार्वजनिक वृत्तिः घ आदाय ख ॱॱ बूल ददाति न६४ आदि\n\
         \x20   यावत् ख समम् सत्यम् आदि\n\
         \x20       प्रत्यागमनम् १ ।\n\
         \x20   इति\n\
         इति\n\
         सार्वजनिक वृत्तिः ङ आदाय ख ॱॱ बूल ददाति न६४ आदि\n\
         \x20   यदि ख समम् सत्यम् आदि\n\
         \x20       प्रत्यागमनम् १ ।\n\
         \x20   इति\n\
         \x20   प्रत्यागमनम् २ ।\n\
         इति\n\
         सार्वजनिक वृत्तिः च आदाय ख ॱॱ बूल ददाति न६४ आदि\n\
         \x20   यदि ख समम् सत्यम् आदि\n\
         \x20       चरः छ ॱॱ न६४ भवति १ ।\n\
         \x20   इति अन्यथा आदि\n\
         \x20       प्रत्यागमनम् ३ ।\n\
         \x20   इति\n\
         इति\n",
    );
    let names = |ids: &[usize]| -> Vec<String> {
        ids.iter().map(|r| c.routines[*r].name.clone()).collect()
    };
    assert_eq!(names(&c.returns_on_every_path), ["क", "ङ"]);
    assert_eq!(names(&c.return_exceptions), ["ग", "घ", "च"]);
    assert_eq!(c.returns_total, 8);
}

/// `W-226`: `ददाति type` IS MANDATORY, and a header without it is REFUSED by
/// the parser AT THE ROUTINE'S NAME. This test stood as "a routine with no
/// `ददाति` that returns anyway is counted" — the class the census's "9
/// without" question needed; the grammar now says the class does not exist
/// (463 of 463 headers carry `ददाति`, and a header without it was a runaway
/// parse, not a procedure), so the same two routines are the refused case.
/// A third, well-formed routine after them still parses: one missing word
/// is one error and not a cascade.
#[test]
fn a_routine_header_without_dadati_is_refused_at_its_name() {
    let src = "मण्डलम् परीक्षा ॥\n\
               सार्वजनिक वृत्तिः क आदि\n\
               \x20   प्रत्यागमनम् १ ।\n\
               इति\n\
               वृत्तिः ग आदाय घ ॱॱ न६४ आदि\n\
               \x20   चरः ङ ॱॱ न६४ भवति घ ।\n\
               इति\n\
               वृत्तिः च ददाति न६४ आदि\n\
               \x20   प्रत्यागमनम् २ ।\n\
               इति\n";
    let mut it = load_parser();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lex runs")
        .as_int()
        .expect("a token count");
    let answered = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parse runs")
        .as_int()
        .expect("a count");
    assert_eq!(answered, 0, "कार्यक्रमपठनम् answers ० for a refused program");
    let errors = it
        .global("दोषसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("दोषसूचकाङ्क exists");
    assert_eq!(errors, 2, "one error per missing ददाति, and no cascade");
    let store = it.global("दोषकोश").expect("दोषकोश exists");
    let why: Vec<(String, i128)> = arena(store)
        .iter()
        .skip(1)
        .take(2)
        .map(|r| {
            (
                String::from_utf8_lossy(&field_octets(r, "कारण").unwrap_or_default()).into_owned(),
                field_int(r, "पङ्क्ति").unwrap_or(0),
            )
        })
        .collect();
    assert_eq!(
        why,
        vec![
            ("वृत्तेः ददाति अपेक्षितम्".to_string(), 2),
            ("वृत्तेः ददाति अपेक्षितम्".to_string(), 5),
        ],
        "the reason names the missing word, and the line is the routine NAME's — \
         `क` on line 2 (no parameters), `ग` on line 5 (a parameter and then `आदि`)"
    );
    let declared = it
        .global("घोषणासूचकाङ्क")
        .and_then(Value::as_int)
        .unwrap_or(0);
    assert_eq!(
        declared, 3,
        "the parse read on: all three declarations recorded"
    );
}

/// Return-to-site: a call in statement position is DISCARDED; every other
/// position consumes it, and each is labelled by what consumes it.
#[test]
fn a_call_in_statement_position_is_discarded_and_every_other_site_consumes() {
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क आदाय ख ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् ख ।\n\
         इति\n\
         सार्वजनिक वृत्तिः ग आदाय ख ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   क ख ।\n\
         \x20   चरः घ ॱॱ न६४ भवति क ख ।\n\
         \x20   घ भवति क घ ।\n\
         \x20   यदि क घ समम् १ आदि\n\
         \x20       घ भवति क आरभ्य क घ ऽ २ समाप्तम् ।\n\
         \x20   इति\n\
         \x20   यावत् क घ आदि\n\
         \x20       घ भवति २ ।\n\
         \x20   इति\n\
         \x20   प्रत्यागमनम् आरभ्य क घ समाप्तम् योगः क घ ।\n\
         इति\n",
    );
    let h = &c.use_histogram;
    assert_eq!(h.get(&Use::Discarded), Some(&1));
    assert_eq!(h.get(&Use::Bound), Some(&1));
    assert_eq!(h.get(&Use::Assigned), Some(&2));
    assert_eq!(h.get(&Use::Compared), Some(&1));
    assert_eq!(h.get(&Use::Passed), Some(&1));
    assert_eq!(h.get(&Use::Condition), Some(&1));
    assert_eq!(h.get(&Use::Operand), Some(&2));
    assert_eq!(c.sites.len(), 9, "{:?}", c.sites);
    assert_eq!(c.discarded.len(), 1);
    assert!(
        c.discarded[0].contains("synthetic:6 परीक्षाॱक (1 argument(s))"),
        "{:?}",
        c.discarded
    );
    assert_eq!(
        (c.arity_agree, c.arity_disagree.len()),
        (8, 1),
        "{:?}",
        c.arity_disagree
    );
}

/// The slice form `x अङ्कः a अन्तः b` is the interpreter's, and since `W-228`
/// the T1 parser's too: it is a `खण्डाभिव्यञ्जक`, not a call on an index, and
/// a real call beside it still is one. REFUSED: a slice with no lower bound —
/// `ख अङ्कः अन्तः २` — is refused where the empty index would be read.
#[test]
fn a_slice_form_is_a_slice_and_one_with_a_missing_bound_is_refused() {
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क आदाय ख ॱॱ अङ्कः अन्तः अ८ ददाति अङ्कः अन्तः अ८ आदि\n\
         \x20   चरः ग ॱॱ अङ्कः अन्तः अ८ भवति ख अङ्कः १ अन्तः २ ।\n\
         \x20   प्रत्यागमनम् क ग ।\n\
         इति\n",
    );
    assert_eq!(c.slice_read_as_call, Vec::<String>::new());
    assert_eq!(c.slices, ["synthetic:3"]);
    assert_eq!(c.sites.len(), 1, "{:?}", c.sites);
    assert!(c.unresolved.is_empty(), "{:?}", c.unresolved);

    // An index followed by an operator or a terminator stays an index.
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क आदाय ख ॱॱ अङ्कः अन्तः अ८ ददाति न६४ आदि\n\
         \x20   चरः ग ॱॱ न६४ भवति ख अङ्कः १ अन्तः योगः २ ।\n\
         \x20   प्रत्यागमनम् ख अङ्कः ग अन्तः ।\n\
         इति\n",
    );
    assert!(c.slices.is_empty() && c.slice_read_as_call.is_empty());

    // REFUSED: no lower bound.
    let m = parse_t1(
        "refused",
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क आदाय ख ॱॱ अङ्कः अन्तः अ८ ददाति अङ्कः अन्तः अ८ आदि\n\
         \x20   चरः ग ॱॱ अङ्कः अन्तः अ८ भवति ख अङ्कः अन्तः २ ।\n\
         \x20   प्रत्यागमनम् ग ।\n\
         इति\n",
    )
    .expect("the refused program still lexes and the parser runs");
    assert!(
        m.parse_errors > 0,
        "a slice with no lower bound must be a parse error"
    );
}

/// `W-228` (b): the spaced qualifier and the unspaced one build ONE tree — a
/// name node carrying both tokens — and the census reads both as the same
/// qualified call. A member on a name that is NOT an imported module stays a
/// member access.
#[test]
fn a_spaced_qualifier_folds_into_the_unspaced_ones_tree() {
    let lib = "मण्डलम् ख ॥\n\
               सार्वजनिक वृत्तिः गणना आदाय अ ॱॱ न६४ ददाति न६४ आदि\n\
               \x20   प्रत्यागमनम् अ ।\n\
               इति\n";
    let user = "मण्डलम् परीक्षा ॥\n\
                आयातः ख ।\n\
                संरचना युग्मम् आरभ्य गणना ॱॱ न६४ समाप्तम् ।\n\
                सार्वजनिक वृत्तिः क आदाय य ॱॱ युग्मम् ददाति न६४ आदि\n\
                \x20   चरः प ॱॱ न६४ भवति ख ॱ गणना १ ।\n\
                \x20   चरः फ ॱॱ न६४ भवति खॱगणना २ ।\n\
                \x20   प्रत्यागमनम् प योगः फ योगः य ॱ गणना ।\n\
                इति\n";
    let a = parse_t1("lib", lib).expect("lib parses");
    let b = parse_t1("user", user).expect("user parses");
    assert_eq!(a.parse_errors + b.parse_errors, 0);
    let c = census(&[a, b]);
    assert!(c.spaced_as_field.is_empty(), "{:?}", c.spaced_as_field);
    let qualified = c
        .sites
        .iter()
        .filter(|s| c.routine_label(s.callee) == "खॱगणना")
        .count();
    assert_eq!(
        qualified, 2,
        "both spellings are the same qualified call to खॱगणना"
    );
    assert!(
        c.unresolved.is_empty() && c.arity_disagree.is_empty(),
        "{:?} {:?}",
        c.unresolved,
        c.arity_disagree
    );
}

/// The fold is decided by the import list alone, so it is sound only while
/// no local or parameter is named like a module its file imports. The corpus
/// has none; this holds it there. (`nidana.t1` names a parameter `निदान`,
/// its OWN module — the fold never applies to the file's own name.)
#[test]
fn no_local_or_parameter_is_named_like_an_imported_module() {
    let mut names: Vec<String> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    let mut collisions = Vec::new();
    for n in &names {
        let text = source(n);
        let code: Vec<&str> = text
            .lines()
            .map(|l| l.split('\u{970}').next().unwrap_or(""))
            .collect();
        let imports: Vec<&str> = code
            .iter()
            .filter_map(|l| l.strip_prefix("आयातः ")?.strip_suffix(" ।"))
            .collect();
        for l in &code {
            // `चरः X ॱॱ` and `आदाय X ॱॱ` / `ऽ X ॱॱ`: every binding and parameter.
            let words: Vec<&str> = l.split_whitespace().collect();
            for w in words.windows(3) {
                if matches!(w[0], "चरः" | "आदाय" | "ऽ") && w[2] == "ॱॱ" && imports.contains(&w[1])
                {
                    collisions.push(format!("{n}: {} is bound and imported", w[1]));
                }
            }
        }
    }
    assert_eq!(
        collisions,
        Vec::<String>::new(),
        "a local or parameter named like an imported module would be folded \
         into a qualified name by व्याकर; rename it, or teach the fold about scopes"
    );
}

/// THE RATCHET (`W-228`), over the corpus: the T1 parser reads no slice as a
/// call and no spaced qualifier as a member on a module — 20 and 127 before,
/// 0 and 0 now — and the slices are still there, as slices: the 20 W-208
/// counted plus the 16 W-227 wrote at its repaired call sites. One
/// interpreter pass; `tools/check-t1-ratchets.sh` runs it on the hourly deep gate.
#[test]
#[ignore = "ratchet: one interpreter pass over the corpus; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn the_corpus_reads_no_slice_as_a_call_and_no_spaced_qualifier_as_a_field() {
    // `W-261`: through `parse_corpus`, which is the SAME reading this test used
    // to spell out inline — the corpus directory's `.t1` files, sorted, each
    // through `parse_t1`, refusing any source with parse errors. It was a
    // second copy of that walk, so this ratchet and the arity ratchet each paid
    // for their own pass over the same nineteen sources even in one process.
    // Sharing it is what lets one invocation serve both.
    let files: Vec<FileModel> = parse_corpus();
    let names: Vec<String> = files.iter().map(|m| m.label.clone()).collect();
    let refused: Vec<&String> = names
        .iter()
        .zip(&files)
        .filter(|(_, m)| m.parse_errors > 0)
        .map(|(n, _)| n)
        .collect();
    assert!(refused.is_empty(), "sources with parse errors: {refused:?}");
    let c = census(&files);
    println!("METRIC paradigm_call_slices {}", c.slices.len());
    println!(
        "METRIC paradigm_call_slices_read_as_calls {}",
        c.slice_read_as_call.len()
    );
    println!(
        "METRIC paradigm_call_spaced_qualifier_read_as_field {}",
        c.spaced_as_field.len()
    );
    assert_eq!(c.slice_read_as_call, Vec::<String>::new());
    assert_eq!(c.spaced_as_field, Vec::<String>::new());
    // 20 (W-208) + 16 (W-227, a token's text passed as the slice its callee
    // declares) + 1 (W-239, `ashtaka.t1:175`, `अष्टकखण्डः` returning
    // `अष्टककोश अङ्कः आरम्भः अन्तः सीमा`). Taken from the failing assertion on
    // the merge of agent/w239 into agent/w227, 2026-09-04, never computed.
    // 37 -> 36 the same day, W-239 part 2: ONE of W-227's sixteen is GONE — the
    // datum's slice of the shared stash (`vakyavibhaga.t1`, `अष्टकसञ्चयः अङ्कः
    // अष्टकारम्भः अन्तः …` in `अष्टकनिर्देशकार्यम्`) became the arena's range
    // through the CALL `अष्टकॱअष्टकखण्डः`, and the stash itself is gone. A fall
    // that is a replacement, said out loud. Taken from the failing assertion.
    // 36 -> 39 on 2026-09-04, `W-223` part 1: THREE slices ADDED, all in the new
    // `sanchaya.t1` — `पाठखण्डः` answering `पाठकोश अङ्कः आरम्भः अन्तः सीमा` (a
    // range of the text store), and the two token-text slices
    // `चिह्नकम् ॱ पाठ अङ्कः … अन्तः …` in `चिह्नकपाठयोजनम्` and `सङ्ग्रहः`.
    // THE SITES, NOT ONLY THE COUNT. Every raise below names its new slices —
    // that is this ratchet's own discipline and it is the right one — but the
    // list was never printed, so each raise had to be "taken from the failing
    // assertion" and the NAMES were recovered by hand or not at all. `c.slices`
    // has held `module:line` per site since it was written (`W-228`); only
    // `.len()` was ever read. Printing it costs nothing on a pass and makes the
    // next drift attributable instead of a blind bump.
    for s in &c.slices {
        println!("  SLICE SITE {s}");
    }
    // Taken from the failing assertion.
    assert_eq!(
        c.slices.len(),
        // 39 -> 42 on 2026-09-05, MEASURED from this assertion's own failure
        // and never computed. W-248's text-to-type reader in `artha.t1` writes
        // THREE new slices: it takes `पाठ` spellings apart to find a wrapper's
        // inner type, and a slice of the name is how it does that. All three are
        // read AS SLICES — `paradigm_call_slices_read_as_calls` stayed 0, which
        // is the half of this ratchet that guards the confusion it is named for.
        // 42 -> 44 on 2026-09-07, the object builder. TWO NEW SLICES, NAMED:
        // `वस्तुसंज्ञासङ्ग्रहः` takes a label's name out of its source
        // (`चिह्नम् ॱ नाम` from `नामारम्भ` to `नामसीमा`), and `वैश्विकत्वम्`
        // takes a global's name out of its `पाठांश` the same way. Both are the
        // corpus's own idiom for a name that lives as a span in a larger text
        // rather than as its own run.
        //
        // `slices_read_as_calls` and `spaced_qualifier_read_as_field` BOTH
        // STILL ZERO, which is what this test is actually for: the count moved
        // and neither misreading did.
        //
        // 44 -> 46 on 2026-09-17, the two table builders. TWO NEW SLICES,
        // NAMED, and found by BLAMING every printed site rather than by
        // accepting the number the failure offered:
        //   `encode.t1:3243`, in `सङ्केतसूचीरचना` — `पाठ्यम् अङ्कः कुलादिः
        //   अन्तः कुलान्तः`, a family field taken out of the encodings table.
        //   `encode.t1:5009`, in `सङ्कोचसूचीरचना` — `पाठ्यम् अङ्कः क्षेत्रादिः
        //   अन्तः पदान्तः`, a field taken out of the compression table.
        // Both are 2026-09-13 and both are the idiom the 2026-09-07 pair
        // already established: a name that lives as a SPAN in a larger text
        // rather than as its own run — here the embedded tables themselves.
        // Both bounds are named `…अन्तः`, an END and not a length, which is the
        // convention the `सीमा` ruling of 2026-09-17 settled on.
        //
        // THE METHOD IS THE POINT, because every raise before this one was
        // "taken from the failing assertion" — the number, never the sites.
        // The sites are now PRINTED above, and blaming them reproduced the
        // 2026-09-07 raise exactly (`encode.t1:6505` and `:6531`, the object
        // builder) before it was trusted for this one.
        //
        // 46 -> 49 on 2026-09-20, AND THE NET IS NOT THE MOVEMENT: FOUR NEW
        // SLICES AND ONE GONE. This is the first raise taken by the method the
        // paragraph above prescribes rather than off the failing number — the
        // 49 printed sites were diffed against `29583a7c`'s sources (the commit
        // that set 46 and added the printing), file by file, and every one of
        // the six untouched files matched exactly, which is what localised the
        // movement to `encode.t1` and `vishlesana.t1` before any line was read.
        // THE FOUR NEW, NAMED:
        //   `encode.t1:6284`, in `अन्तरालसहितलेखनपरीक्षा`, and
        //   `encode.t1:6318`, in `खण्डलेखनपरीक्षा`, and
        //   `vishlesana.t1:913`, in `परमण्डलक्षेत्रलेखनपरीक्षा` — all three the
        //   same line, `स्रोतः अङ्कः ० अन्तः ३`, a three-octet head of a literal.
        //   All three are inside `…परीक्षा` routines: they are the minimal
        //   reproductions `8a3c9651` and `cce9256b` wrote on 2026-09-18 while
        //   narrowing the native-lowering fault. A fixture's slice is a slice,
        //   so it counts here — the ratchet reads the corpus, not the shipping
        //   subset of it.
        //   `vishlesana.t1:955`, in `पङ्क्तिसङ्केतः` — `चरः आज्ञापाठः ॱॱ … भवति
        //   पाठ्यम् अङ्कः आज्ञादिः अन्तः आज्ञान्तः`, by `5a8a3b86`.
        // THE ONE GONE IS THAT SAME SLICE, and saying so is the point: the
        // inline `नव ॱ आज्ञा भवति पाठ्यम् अङ्कः आज्ञादिः अन्तः आज्ञान्तः` that
        // stood in the same routine is deleted, and `:955` is it HOISTED into a
        // local. Counted as a rise and a fall it is +4/-1; read as movement it
        // is three genuinely new slices and one moved. That is the W-239 shape
        // — "a fall that is a replacement, said out loud" — seen from the other
        // side, and a later reader who takes the +3 as three new sites will go
        // looking for a third `स्रोतः` fixture that does not exist.
        //
        // `slices_read_as_calls` and `spaced_qualifier_read_as_field` BOTH
        // STILL ZERO on all 49, which is the half of this ratchet that is about
        // correctness rather than about census.
        //
        // 49 -> 48 on 2026-10-06, symbol-lookup step 1 (d0): ONE SLICE GONE,
        // NAMED, by diffing the printed sites against `e4ce6430`: `encode.t1:7139`,
        // in `वैश्विकत्वम्` — the 2026-09-07 global-name slice above. It now
        // compares the candidate IN PLACE through `परिधिसाम्यम्`, because the
        // slice copied every candidate's octets (6.12G of Stage 2). The only
        // other movement is `वस्तुसंज्ञासङ्ग्रहः`'s slice, :7165 -> :7168, moved
        // by the margin above `वैश्विकत्वम्`, not changed.
        48,
        "the corpus's slices, all read as slices: 20 W-208 counted, 15 of W-227's \
         16 standing (one became W-239's arena call), 1 W-239 wrote, 3 W-223's \
         store wrote, 3 the 2026-09-18 native-lowering probes wrote. A rise is a \
         new slice — name it here."
    );
}

/// The call graph: `क → ख → क` is one component of two, `ग → ग` is
/// self-recursion, and `घ → क` is neither. Degrees count DISTINCT callees.
#[test]
fn mutual_recursion_is_one_component_and_self_recursion_is_its_own() {
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः क आदाय न ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् ख न ।\n\
         इति\n\
         सार्वजनिक वृत्तिः ख आदाय न ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   यदि न समम् ० आदि\n\
         \x20       प्रत्यागमनम् ० ।\n\
         \x20   इति\n\
         \x20   प्रत्यागमनम् क आरभ्य न वियोगः १ समाप्तम् ।\n\
         इति\n\
         सार्वजनिक वृत्तिः ग आदाय न ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् ग न ।\n\
         इति\n\
         सार्वजनिक वृत्तिः घ ददाति न६४ आदि\n\
         \x20   चरः च ॱॱ न६४ भवति क १ ।\n\
         \x20   प्रत्यागमनम् क च ।\n\
         इति\n",
    );
    let names = |ids: &[usize]| -> Vec<String> {
        ids.iter().map(|r| c.routines[*r].name.clone()).collect()
    };
    assert_eq!(c.sccs.len(), 1, "{:?}", c.sccs);
    assert_eq!(names(&c.sccs[0]), ["क", "ख"]);
    assert_eq!(names(&c.self_recursive), ["ग"]);
    assert_eq!(c.out_degree, [1, 1, 1, 1], "distinct callees per routine");
    assert_eq!(c.in_degree, [2, 1, 1, 0]);
    assert_eq!(c.sites.len(), 5);
}

/// A qualified call names another module's routine, and a bare name that is
/// a LOCAL is never mistaken for the routine of the same name.
#[test]
fn a_qualified_call_crosses_modules_and_a_local_shadows_nothing_into_a_call() {
    let a = parse_t1(
        "a",
        "मण्डलम् एक ॥\n\
         सार्वजनिक चरः ध ॱॱ न६४ भवति ७ ।\n\
         सार्वजनिक वृत्तिः क आदाय ख ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् ख ।\n\
         इति\n\
         सार्वजनिक वृत्तिः छ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् ८ ।\n\
         इति\n",
    )
    .expect("module एक parses");
    let b = parse_t1(
        "b",
        "मण्डलम् द्वि ॥\n\
         आयातः एक ।\n\
         सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
         \x20   चरः क ॱॱ न६४ भवति एकॱक ५ ।\n\
         \x20   चरः ख ॱॱ न६४ भवति एक ॱ क ६ ।\n\
         \x20   चरः ङ ॱॱ न६४ भवति एक ॱ ध ।\n\
         \x20   चरः च ॱॱ न६४ भवति एक ॱ छ ।\n\
         \x20   प्रत्यागमनम् क योगः ख ।\n\
         इति\n",
    )
    .expect("module द्वि parses");
    let c = census(&[a, b]);
    // `एकॱक ५` and `एक ॱ क ६` are the same call in two spellings; `एक ॱ ध`
    // is a global of module एक, not a call; `एक ॱ छ` is the routine with no
    // arguments, a call with none.
    assert_eq!(c.sites.len(), 3, "{:?}", c.sites);
    for s in &c.sites {
        assert_eq!(c.routine_label(s.caller), "द्विॱग");
    }
    let callees: Vec<(String, usize)> = c
        .sites
        .iter()
        .map(|s| (c.routine_label(s.callee), s.args))
        .collect();
    assert_eq!(
        callees,
        [
            ("एकॱक".to_string(), 1),
            ("एकॱक".to_string(), 1),
            ("एकॱछ".to_string(), 0)
        ]
    );
    assert_eq!(
        (c.arity_agree, c.arity_disagree.len()),
        (3, 0),
        "{:?}",
        c.arity_disagree
    );
    assert!(c.unresolved.is_empty(), "{:?}", c.unresolved);
    assert!(c.unknown_names.is_empty(), "{:?}", c.unknown_names);
}

// ─────────────────────────────────────────────────────────────────────────
// The arity ratchet — W-227. The pin W-193's `ARITY_FAULTS` table used to be.
// ─────────────────────────────────────────────────────────────────────────

/// Every `.t1` source parsed by the T1 parser, in name order — ONCE PER
/// PROCESS, however many tests in this binary ask for it (`W-261`). A source
/// the parser refuses is a failure here, not a line in a report: a ratchet
/// that skipped a file would be pinning a smaller corpus than it names.
///
/// The shared value is immutable plain data, not a live interpreter, which is
/// what makes it safe; the reasoning is on `parse_corpus` in `t1_paradigm.rs`
/// and is the same here.
fn parse_corpus() -> Vec<FileModel> {
    static CORPUS: OnceLock<Vec<FileModel>> = OnceLock::new();
    CORPUS.get_or_init(parse_corpus_uncached).clone()
}

/// The pass itself. Called at most once per process, through [`parse_corpus`].
/// Reports its own cost for the same reason as `t1_paradigm.rs`'s: the saving
/// is read from an in-test timer, never from wall clock.
fn parse_corpus_uncached() -> Vec<FileModel> {
    let started = std::time::Instant::now();
    let out = parse_corpus_pass();
    println!(
        "METRIC paradigm_calls_corpus_pass_ms {}",
        started.elapsed().as_millis()
    );
    out
}

fn parse_corpus_pass() -> Vec<FileModel> {
    let mut names: Vec<String> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    names
        .iter()
        .map(|n| {
            let m = parse_t1(n, &source(n)).unwrap_or_else(|e| panic!("{n} did not parse: {e}"));
            assert_eq!(m.parse_errors, 0, "{n} has parse errors");
            m
        })
        .collect()
}

/// The pin: every call site names a routine and passes what it declares.
/// Returns the disagreements exactly as the census prints them — `file:line
/// calls मण्डलॱनाम with N argument(s); it declares M` — so a new one fails by
/// its site and both counts, never as a bare number.
fn arity_disagreements(c: &Census) -> Vec<String> {
    c.arity_disagree.clone()
}

/// **`paradigm_call_arity_disagreements` is pinned at 0 over the whole
/// corpus.** Measured 2026-09-04 after W-227 repaired the eighteen; the
/// sixteen triple-for-slice sites, the inverse and the unbracketed nested call
/// are each named in `t1_execution.rs`'s retired-table note. A new
/// disagreement anywhere in the fifteen sources fails this test with the
/// site, the callee and the two counts.
///
/// `#[ignore]`d for the same reason W-222's hoisting ratchet is: it is one T1
/// parser pass over all fifteen sources (about two minutes), and what it
/// measures can move only when a `.t1` source moves.
/// `tools/check-t1-ratchets.sh` runs it on the hourly deep gate.
#[test]
#[ignore = "ratchet over the whole corpus (~2 min) — run by tools/check-t1-ratchets.sh on the hourly deep gate"]
fn arity_disagreements_are_pinned_at_zero_and_a_new_one_fails_by_name() {
    let files = parse_corpus();
    let c = census(&files);
    let found = arity_disagreements(&c);
    println!("METRIC paradigm_call_sites {}", c.sites.len());
    println!("METRIC paradigm_call_arity_disagreements {}", found.len());
    assert!(
        c.sites.len() >= 1_500,
        "only {} call sites were seen across {} sources; the pin below would be \
         vacuous on a corpus this small",
        c.sites.len(),
        files.len()
    );
    assert!(
        found.is_empty(),
        "{} call site(s) pass a number of arguments the callee does not declare \
         (was 0 on 2026-09-04, W-227). Each is a corpus error until the grammar \
         says otherwise — fix the site to pass what the callee declares, as its \
         Rust twin does:\n  {}",
        found.len(),
        found.join("\n  ")
    );
}

/// THE REFUSED CASE, without waiting for the corpus: the exact shape W-227
/// repaired — a token's text passed as its `(पाठ ऽ अष्टक ऽ पाठसीमा)` triple to a
/// routine declaring one slice — is a disagreement the pin above names by
/// site and by both counts, and the repaired spelling beside it (the slice)
/// is not.
#[test]
fn a_triple_passed_to_a_slice_parameter_is_a_disagreement_the_pin_names() {
    let c = synthetic(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक संरचना चिह्नक आरभ्य\n\
         \x20   पाठ ॱॱ अङ्कः अन्तः अ८ ऽ\n\
         \x20   अष्टक ॱॱ न६४ ऽ\n\
         \x20   पाठसीमा ॱॱ न६४\n\
         समाप्तम् ।\n\
         सार्वजनिक वृत्तिः संख्या आदाय पाठ ॱॱ अङ्कः अन्तः अ८ ददाति न६४ आदि\n\
         \x20   प्रत्यागमनम् पाठ ॱ दैर्घ्य ।\n\
         इति\n\
         सार्वजनिक वृत्तिः पठनम् आदाय चिह्नकम् ॱॱ चिह्नक ददाति न६४ आदि\n\
         \x20   चरः क ॱॱ न६४ भवति संख्या आरभ्य चिह्नकम् ॱ पाठ ऽ चिह्नकम् ॱ अष्टक ऽ चिह्नकम् ॱ पाठसीमा समाप्तम् ।\n\
         \x20   चरः ख ॱॱ न६४ भवति संख्या आरभ्य चिह्नकम् ॱ पाठ अङ्कः चिह्नकम् ॱ अष्टक अन्तः चिह्नकम् ॱ पाठसीमा समाप्तम् ।\n\
         \x20   प्रत्यागमनम् क योगः ख ।\n\
         इति\n",
    );
    let found = arity_disagreements(&c);
    assert_eq!(
        found,
        ["synthetic:11 calls परीक्षाॱसंख्या with 3 argument(s); it declares 1"],
        "the triple is the disagreement and the slice beside it is not: {found:?}"
    );
    assert_eq!(c.sites.len(), 2, "two call sites, one each: {:?}", c.sites);
    assert_eq!(c.arity_agree, 1, "the slice spelling agrees");
    assert_eq!(
        c.slices,
        ["synthetic:12"],
        "the slice spelling is a slice (W-228's `खण्डाभिव्यञ्जक`), not an arity fault"
    );
    assert!(
        c.slice_read_as_call.is_empty(),
        "{:?}",
        c.slice_read_as_call
    );
}
