//! **THE PARSE TWIN** — `docs/parse-twin-design.md`.
//!
//! The `.t1` parser (`व्याकर`, `parse.t1`) joined binary operators with NO
//! precedence levels, so `सरणम् योगः ३ न्यूनम् सीमा` parsed as
//! `सरणम् योगः (३ न्यूनम् सीमा)` and every mixed-level loop bound in the compiled
//! compiler spun natively. No instrument saw it: the interpreter runs the `.t1`
//! compiler through ITS OWN parser (`nirvahana.rs`), twin AGREE compares two
//! emitters that both consume the `.t1`-parsed tree, and the `.t1` unparser
//! prints operator words back without brackets. A twin is blind to what both
//! twins share.
//!
//! So: ONE fully bracketed printer over TWO trees of the same source, compared
//! per routine body.
//!
//! * **Side A, the reference** — `nirvahana`'s parse of the corpus source,
//!   printed by [`Interpreter::canonical`].
//! * **Side B, the subject** — the `.t1` parser's arenas
//!   (`वास्तुॱअभिव्यञ्जककोश`, `वास्तुॱवाक्यकोश`, reached from each `घोषणा`'s
//!   `शरीरसूचकाङ्क`) after the front-half driver `शृङ्खलाॱपठनम्` has parsed the
//!   SAME source under the interpreter, walked here into the same
//!   [`CanonStmt`]s and printed by the same [`canon_text`].
//!
//! Zero differences is the pass. A difference names the routine and the first
//! differing node (the first differing statement line, both sides, with the
//! column at which they part).
//!
//! **Scope.** One test per corpus source, narrowed by `T1_CORPUS=<stems>`
//! exactly as the census is. With `T1_CORPUS` unset every test REFUSES with
//! one line naming the variable and runs nothing — the whole walk is the deep
//! gate's, never a landing's. `T1_COMPILER_DIR=<dir>` points side B at another
//! set of compiler sources; the positive control in the design points it at a
//! copy whose `parse.t1` predates the precedence fix (`2a59ab7b`), and the
//! twin MUST report differences there or it is blind.
//!
//! **What the printer deliberately does not distinguish.** A nullary call and
//! a name (the `.t1` parser has one node for both); the spacing inside a string
//! literal (the two lexers keep it differently); a `चरः`'s declared type.

use sadhana::t1::nirvahana::{
    CanonExpr, CanonStmt, Interpreter, Octets, Value, canon_string, canon_text, numeral_value,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The front half as `t1_driver.rs` loads it for `शृङ्खलाॱपठनम्`: lexer, AST,
/// parser, the numeral reader the parser asks, the declaration store the
/// driver fills, and the modules `शृङ्खला`'s other routines name.
const FRONT: &[&str] = &[
    "lex.t1",
    "ast.t1",
    "parse.t1",
    "sanskrit_text.t1",
    "sanchaya.t1",
    "artha.t1",
    "vastu.t1",
    "ir.t1",
    "shrinkhala.t1",
];

/// Fuel for one `पठनम्` of a corpus source — `t1_driver.rs`'s figure.
const FUEL: u64 = 40_000_000_000;

// `ast.t1`'s discriminants, by name. Expression kinds:
const E_NAME: i128 = 1;
const E_NUMERAL: i128 = 2;
const E_STRING: i128 = 3;
const E_GROUP: i128 = 4;
const E_INDEX: i128 = 5;
const E_FIELD: i128 = 6;
const E_CALL: i128 = 7;
const E_BINARY: i128 = 8;
const E_BOOL: i128 = 9;
const E_NIL: i128 = 10;
const E_EMBED: i128 = 11;
const E_NEGATE: i128 = 12;
const E_SLICE: i128 = 13;
// Statement kinds:
const S_EXPR: i128 = 1;
const S_BLOCK: i128 = 2;
const S_LET: i128 = 3;
const S_RETURN: i128 = 4;
const S_IMPORT: i128 = 5;
const S_IF: i128 = 6;
const S_WHILE: i128 = 7;
const S_ASSIGN: i128 = 8;
/// `parse.t1`: `वृत्तिघोषणाभेद`.
const D_ROUTINE: i128 = 1;
/// `lex.t1`: `शब्दभेद` — a string token (a layout word or a collapsed embed).
const T_STRING: i128 = 3;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// The corpus — the sources under test, always this crate's own.
fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The compiler side B runs — the corpus dir unless `T1_COMPILER_DIR` says
/// otherwise (the positive control's scratch copy with the old `parse.t1`).
fn compiler_dir() -> PathBuf {
    std::env::var("T1_COMPILER_DIR").map_or_else(|_| corpus_dir(), PathBuf::from)
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Every corpus source's stem, sorted.
fn corpus() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter_map(|n| n.strip_suffix(".t1").map(str::to_string))
        .collect();
    names.sort();
    names
}

/// `T1_CORPUS`, parsed as the census parses it: comma-separated stems, `.t1`
/// optional, every name required to exist. `None` when the variable is unset.
fn selection() -> Option<Vec<String>> {
    let want = std::env::var("T1_CORPUS").ok()?;
    let all = corpus();
    let want: Vec<String> = want
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.strip_suffix(".t1").unwrap_or(s).to_string())
        .collect();
    let missing: Vec<&String> = want.iter().filter(|w| !all.contains(w)).collect();
    assert!(
        missing.is_empty(),
        "T1_CORPUS names sources that are not in the corpus: {missing:?}. Available: {all:?}"
    );
    Some(want)
}

/// `मण्डलम् NAME ॥` — the module a source declares, off its first declaration;
/// `None` for a source that declares none (`lib.t1` is comment only).
fn module_of(source: &str) -> Option<String> {
    let mut words = source.split_whitespace();
    while let Some(w) = words.next() {
        if w == "मण्डलम्" {
            return Some(words.next().unwrap_or_default().to_string());
        }
    }
    None
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn arena(it: &Interpreter, name: &str) -> Vec<Value> {
    match it.global(name) {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("`{name}` is an arena, not {other:?}"),
    }
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    match it.global(name) {
        Some(Value::Int(n)) => *n,
        other => panic!("`{name}` is a number, not {other:?}"),
    }
}

fn field(rec: &Value, name: &str) -> Value {
    match rec {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("the record has no `{name}`")),
        other => panic!("`{name}` read from a non-record {other:?}"),
    }
}

fn field_int(rec: &Value, name: &str) -> i128 {
    match field(rec, name) {
        Value::Int(n) => n,
        other => panic!("`{name}` is a number, not {other:?}"),
    }
}

/// A field that is an arena index — `०` is "no child" and is answered as 0.
fn field_index(rec: &Value, name: &str) -> usize {
    usize::try_from(field_int(rec, name)).unwrap_or_else(|_| panic!("`{name}` is negative"))
}

/// The `.t1` parser's three arenas after one `पठनम्`, walked into the twin's tree.
struct Arenas {
    tokens: Vec<Value>,
    exprs: Vec<Value>,
    stmts: Vec<Value>,
}

impl Arenas {
    fn take(it: &Interpreter) -> Self {
        Self {
            tokens: arena(it, "चिह्नककोश"),
            exprs: arena(it, "अभिव्यञ्जककोश"),
            stmts: arena(it, "वाक्यकोश"),
        }
    }

    /// A token's text is `पाठ[अष्टक..पाठसीमा]` — what `पदविभागॱचिह्नकपाठः`
    /// copies out, read here without a call. A collapsed embed's `पाठ` is the
    /// table itself, so the token's OWN run is read and never the source.
    fn tok_text(&self, i: usize) -> String {
        let t = &self.tokens[i];
        let from = field_index(t, "अष्टक");
        let to = field_index(t, "पाठसीमा");
        match field(t, "पाठ") {
            Value::Octets(o) => String::from_utf8_lossy(&o.as_slice()[from..to]).into_owned(),
            other => panic!("token {i}'s `पाठ` is a run, not {other:?}"),
        }
    }

    fn tok_kind(&self, i: usize) -> i128 {
        field_int(&self.tokens[i], "भेद")
    }

    fn ekind(&self, i: usize) -> i128 {
        field_int(&self.exprs[i], "भेद")
    }

    fn expr(&self, i: usize) -> CanonExpr {
        if i == 0 {
            return CanonExpr::Leaf("<no expression>".to_string());
        }
        let e = &self.exprs[i];
        let value = field_index(e, "मूल्यसूचकाङ्क");
        let left = field_index(e, "वामसूचकाङ्क");
        let right = field_index(e, "दक्षिणसूचकाङ्क");
        match self.ekind(i) {
            E_NAME => {
                // W-228 (b): a spaced `मण्डल ॱ नाम` folds into the name node,
                // `दक्षिणसूचकाङ्क` naming the member token; side A prints a
                // qualified call the same way, and the unspaced token already
                // carries the mark inside it.
                let mut name = self.tok_text(value);
                if right != 0 {
                    name.push('\u{971}');
                    name.push_str(&self.tok_text(right));
                }
                CanonExpr::Leaf(name)
            }
            E_NUMERAL => {
                let text = self.tok_text(value);
                CanonExpr::Leaf(
                    numeral_value(&text)
                        .map_or_else(|| format!("<not a numeral: {text}>"), |n| n.to_string()),
                )
            }
            E_STRING => self.string(value, right),
            E_GROUP => self.expr(left),
            E_INDEX => CanonExpr::Index(Box::new(self.expr(left)), Box::new(self.expr(right))),
            // `मूल्यसूचकाङ्क` is `पठनस्थान` AFTER the member was read — one past it.
            E_FIELD => CanonExpr::Field(Box::new(self.expr(left)), self.tok_text(value - 1)),
            E_CALL => self.call(i),
            E_BINARY => CanonExpr::Bin(
                operator(field_int(e, "द्विकर्म")),
                Box::new(self.expr(left)),
                Box::new(self.expr(right)),
            ),
            E_BOOL => CanonExpr::Leaf(self.tok_text(value)),
            E_NIL => CanonExpr::Leaf("शून्यम्".to_string()),
            E_EMBED => CanonExpr::Leaf(format!("समावेशः({})", self.tok_text(value))),
            // `ऋण x` — side A reads it as `० वियोगः x`.
            E_NEGATE => CanonExpr::Bin(
                "-",
                Box::new(CanonExpr::Leaf("0".to_string())),
                Box::new(self.expr(left)),
            ),
            // `वाम` is the index node (base and lower bound), `दक्षिण` the limit.
            E_SLICE => {
                let inner = &self.exprs[left];
                assert_eq!(
                    self.ekind(left),
                    E_INDEX,
                    "a slice's वाम is its index node, not kind {}",
                    self.ekind(left)
                );
                CanonExpr::Slice(
                    Box::new(self.expr(field_index(inner, "वामसूचकाङ्क"))),
                    Box::new(self.expr(field_index(inner, "दक्षिणसूचकाङ्क"))),
                    Box::new(self.expr(right)),
                )
            }
            other => CanonExpr::Leaf(format!("<expression kind {other}>")),
        }
    }

    /// A string literal's tokens are `[from, to)`: after `उक्तम्` up to the
    /// closing `इति`, a doubled `इति` inside meaning the literal word
    /// (ADR-0011); or ONE `शब्दभेद` token standing BARE — a layout word, whose
    /// value the Rust lexer hands over as the character it names, or a
    /// collapsed embed.
    ///
    /// FAITHFUL TO THE TOKEN KIND, NOT TO THE RULE (2026-09-14). The first
    /// version looked at the `उक्तम्` before the span and read a quoted layout
    /// word as the word — which is the RIGHT rule, and exactly the rule
    /// `lex.t1` did not have: it marked `उक्तम् विवरम् इति` `शब्दभेद` by text
    /// alone, `ir.t1` then folded the quoted word to one octet, and the
    /// compiled compiler emitted `०` where the interpreter emits `३२` (the
    /// lex rung of the native ladder, 03:10). This printer had normalised the
    /// defect away. Now it prints what the `.t1` lexer marked; the positive
    /// control is the tree before the lex.t1 fix, where lex.t1's own quoted
    /// layout words report as differences.
    fn string(&self, from: usize, to: usize) -> CanonExpr {
        if to == from + 1 && self.tok_kind(from) == T_STRING {
            let text = self.tok_text(from);
            return match text.as_str() {
                "यतिः" => canon_string("\n"),
                "विवरम्" => canon_string(" "),
                _ => canon_string(&text),
            };
        }
        let mut out = String::new();
        let mut k = from;
        while k < to {
            let t = self.tok_text(k);
            if t == "इति" && k + 1 < to && self.tok_text(k + 1) == "इति" {
                k += 1;
            }
            out.push_str(&t);
            k += 1;
        }
        canon_string(&out)
    }

    /// The arguments curry — `f a b` is `(f a) b`, one argument per node in
    /// `दक्षिण` and the callee-so-far in `वाम` — so the chain is flattened to
    /// the `f(a, b)` side A builds from the arity.
    fn call(&self, i: usize) -> CanonExpr {
        let mut args = Vec::new();
        let mut k = i;
        while self.ekind(k) == E_CALL {
            args.push(self.expr(field_index(&self.exprs[k], "दक्षिणसूचकाङ्क")));
            k = field_index(&self.exprs[k], "वामसूचकाङ्क");
        }
        args.reverse();
        match self.expr(k) {
            CanonExpr::Leaf(name) => CanonExpr::Call(name, args),
            other => CanonExpr::Call(format!("<{}>", other.text()), args),
        }
    }

    /// A block's direct children, from `ast.t1`'s own walk: start at the last
    /// statement and step to `आदिसूचकाङ्क − 1`, skipping each child's whole
    /// subtree, until the first. `०` is an empty block.
    fn block(&self, i: usize) -> Vec<CanonStmt> {
        if i == 0 {
            return Vec::new();
        }
        let s = &self.stmts[i];
        if field_int(s, "भेद") != S_BLOCK {
            return vec![self.stmt(i)];
        }
        let first = field_index(s, "वामसूचकाङ्क");
        let last = field_index(s, "दक्षिणसूचकाङ्क");
        let mut out = Vec::new();
        if first == 0 {
            return out;
        }
        let mut k = last;
        while k >= first && k != 0 {
            out.push(self.stmt(k));
            let start = field_index(&self.stmts[k], "आदिसूचकाङ्क");
            if start == 0 {
                break;
            }
            k = start - 1;
        }
        out.reverse();
        out
    }

    fn stmt(&self, i: usize) -> CanonStmt {
        let s = &self.stmts[i];
        let left = field_index(s, "वामसूचकाङ्क");
        let right = field_index(s, "दक्षिणसूचकाङ्क");
        match field_int(s, "भेद") {
            S_EXPR => CanonStmt::Eval(self.expr(left)),
            S_BLOCK => CanonStmt::Eval(CanonExpr::Leaf("<bare block>".to_string())),
            S_LET => CanonStmt::Let(self.tok_text(left), (right != 0).then(|| self.expr(right))),
            S_RETURN => CanonStmt::Return((left != 0).then(|| self.expr(left))),
            S_IMPORT => CanonStmt::Eval(CanonExpr::Leaf("<import>".to_string())),
            S_IF => CanonStmt::If(
                self.expr(left),
                self.block(right),
                self.block(field_index(s, "अन्यसूचकाङ्क")),
            ),
            S_WHILE => CanonStmt::While(self.expr(left), self.block(right)),
            S_ASSIGN => CanonStmt::Assign(self.expr(left), self.expr(right)),
            other => CanonStmt::Eval(CanonExpr::Leaf(format!("<statement kind {other}>"))),
        }
    }
}

/// `ast.t1`'s fifteen operator kinds, in the design's spelling.
fn operator(code: i128) -> &'static str {
    match code {
        1 => "+",
        2 => "-",
        3 => "*",
        4 => "/",
        5 => "==",
        6 => "!=",
        7 => "<",
        8 => ">",
        9 => "&",
        10 => "|",
        11 => "^",
        12 => "<<",
        13 => ">>",
        14 => "%",
        15 => ">=",
        other => panic!("`द्विकर्म` {other} is not one of the fifteen operator kinds"),
    }
}

/// One side's routines: `(name, body as canon_text prints it)`, in order.
type Routines = Vec<(String, String)>;

/// Side A: every corpus source loaded (a call is juxtaposition, and only the
/// arity table says where its arguments stop — so the reference needs every
/// module's signatures, exactly as the product's interpreter has them), then
/// `canonical` of the module the source under test declares.
fn side_a(module: &str) -> Routines {
    let all = corpus();
    let texts: Vec<(String, String)> = all
        .iter()
        .map(|n| {
            (
                format!("{n}.t1"),
                read(&corpus_dir().join(format!("{n}.t1"))),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let it = Interpreter::load(&refs, &spec_root())
        .unwrap_or_else(|e| panic!("the corpus loads into nirvahana: {e}"));
    let mut out: Routines = Vec::new();
    for line in it.canonical(module).lines() {
        if let Some(name) = line.strip_prefix("वृत्तिः ") {
            out.push((name.to_string(), String::new()));
        } else if let Some((_, body)) = out.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out
}

/// What side B answered: its routines, and the `.t1` parser's own refusals.
struct SideB {
    routines: Routines,
    errors: Vec<String>,
}

/// Side B: the front half from `compiler_dir()`, `शृङ्खलाॱपठनम्` on the source,
/// then the arenas — every `घोषणा` of kind `वृत्तिः`, its name token's text
/// and the block at its `शरीरसूचकाङ्क`.
fn side_b(source: &str) -> SideB {
    let dir = compiler_dir();
    let texts: Vec<(String, String)> = FRONT
        .iter()
        .map(|n| ((*n).to_string(), read(&dir.join(n))))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root())
        .unwrap_or_else(|e| panic!("the front half loads from {}: {e}", dir.display()));
    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(source)], FUEL)
        .unwrap_or_else(|e| panic!("शृङ्खलाॱपठनम् refused to run: {e}"));
    println!("  पठनम् answered {decls:?}");

    let errors: Vec<String> = {
        let n = usize::try_from(global_int(&it, "दोषसूचकाङ्क")).unwrap_or(0);
        let store = arena(&it, "दोषकोश");
        (1..=n)
            .map(|k| {
                let reason = match field(&store[k], "कारण") {
                    Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
                    other => format!("{other:?}"),
                };
                format!("line {}: {reason}", field_int(&store[k], "पङ्क्ति"))
            })
            .collect()
    };

    let arenas = Arenas::take(&it);
    let decls_n = usize::try_from(global_int(&it, "घोषणासूचकाङ्क")).unwrap_or(0);
    let store = arena(&it, "घोषणाकोश");
    let mut routines: Routines = Vec::new();
    // Slot ० is the arena's reserved hole; live declarations are १..=cursor.
    for g in store.iter().take(decls_n + 1).skip(1) {
        if field_int(g, "भेद") != D_ROUTINE {
            continue;
        }
        let name = arenas.tok_text(field_index(g, "नामसूचकाङ्क"));
        let body = field_index(g, "शरीरसूचकाङ्क");
        let text = if body == 0 {
            "  <no body>\n".to_string()
        } else {
            canon_text(&arenas.block(body))
        };
        routines.push((name, text));
    }
    SideB { routines, errors }
}

/// The first line on which two bodies part, with the column where they do.
fn first_difference(a: &str, b: &str) -> String {
    let (al, bl): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    for k in 0..al.len().max(bl.len()) {
        let (x, y) = (
            al.get(k).copied().unwrap_or("<end>"),
            bl.get(k).copied().unwrap_or("<end>"),
        );
        if x != y {
            let col = x.chars().zip(y.chars()).take_while(|(p, q)| p == q).count();
            return format!(
                "statement {} (column {col})\n    A: {}\n    B: {}",
                k + 1,
                clip(x),
                clip(y)
            );
        }
    }
    "<no differing line>".to_string()
}

fn clip(s: &str) -> String {
    if s.chars().count() > 400 {
        let head: String = s.chars().take(400).collect();
        format!("{head}…")
    } else {
        s.to_string()
    }
}

/// How many corpus sources declare `module` — `वास्तु` is declared by two
/// (`ast.t1` and `vastu.t1`), and side A merges them into one module.
fn declarers(module: &str) -> usize {
    corpus()
        .iter()
        .filter(|n| {
            module_of(&read(&corpus_dir().join(format!("{n}.t1")))).as_deref() == Some(module)
        })
        .count()
}

/// The twin on one corpus source: refuses without `T1_CORPUS`, stands down
/// when the variable names other sources, and otherwise asserts ZERO
/// differences between the two parses of every routine body.
fn twin(stem: &str) {
    let Some(selected) = selection() else {
        eprintln!(
            "!! parse_twin::{stem}: REFUSED — T1_CORPUS is not set; name the sources \
             (T1_CORPUS=lex,parse,…) — the twin never walks the whole corpus on its own !!"
        );
        return;
    };
    if !selected.iter().any(|s| s == stem) {
        return;
    }
    let source = read(&corpus_dir().join(format!("{stem}.t1")));
    let Some(module) = module_of(&source) else {
        // A source with no `मण्डलम्` has no routines for either side; the one
        // thing to check is that the `.t1` parser agrees it holds none.
        let b = side_b(&source);
        println!(
            "parse-twin {stem}.t1: declares no module; .t1 parsed {} routine(s), {} error(s)",
            b.routines.len(),
            b.errors.len()
        );
        assert!(
            b.routines.is_empty() && b.errors.is_empty(),
            "{stem}.t1 declares no module yet the .t1 parser answered {:?} / {:?}",
            b.routines,
            b.errors
        );
        return;
    };
    println!(
        "parse-twin {stem}.t1 (module {module}; compiler {})",
        compiler_dir().display()
    );

    let t = Instant::now();
    let a = side_a(&module);
    let a_secs = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let b = side_b(&source);
    let b_secs = t.elapsed().as_secs_f64();

    let mut differences: Vec<String> = Vec::new();
    let mut ref_unparsed = 0usize;
    let mut compared = 0usize;
    for (name, body_b) in &b.routines {
        match a.iter().find(|(n, _)| n == name) {
            None => differences.push(format!("{name}: parsed by .t1, absent from the reference")),
            Some((_, body_a)) if body_a.starts_with("  <unparsed:") => {
                ref_unparsed += 1;
                println!("  REF-UNPARSED {stem} {name}: {}", body_a.trim());
            }
            Some((_, body_a)) => {
                compared += 1;
                if body_a != body_b {
                    differences.push(format!("{name}: {}", first_difference(body_a, body_b)));
                }
            }
        }
    }
    let shared = declarers(&module);
    for (name, _) in &a {
        if !b.routines.iter().any(|(n, _)| n == name) {
            if shared > 1 {
                println!("  (reference routine {name} belongs to another file of module {module})");
            } else {
                differences.push(format!(
                    "{name}: in the reference, absent from the .t1 parse"
                ));
            }
        }
    }
    for e in &b.errors {
        differences.push(format!("<.t1 parser refused> {e}"));
    }

    for d in &differences {
        println!("  DIFF {stem} {d}");
    }
    println!(
        "parse-twin {stem}.t1: ref={} t1={} compared={compared} differences={} ref_unparsed={ref_unparsed} t1_errors={} (side A {a_secs:.1}s, side B {b_secs:.1}s)",
        a.len(),
        b.routines.len(),
        differences.len(),
        b.errors.len()
    );
    assert!(
        differences.is_empty(),
        "{stem}.t1: {} difference(s) between the two parses:\n  {}",
        differences.len(),
        differences.join("\n  ")
    );
    // A pass over NOTHING is the check-that-cannot-fail shape: a source whose
    // parser answered no routines would agree with the reference vacuously.
    assert!(
        compared > 0 || a.is_empty(),
        "{stem}.t1: the reference holds {} routines and none was compared",
        a.len()
    );
}

macro_rules! twins {
    ($($f:ident = $s:literal;)*) => {
        /// The sources this file twins — pinned against the directory below.
        const SOURCES: &[&str] = &[$($s),*];
        $(
            #[test]
            fn $f() {
                twin($s);
            }
        )*
    };
}

twins! {
    artha = "artha";
    ashtaka = "ashtaka";
    ast = "ast";
    encode = "encode";
    ir = "ir";
    kosha = "kosha";
    lex = "lex";
    lib = "lib";
    nidana = "nidana";
    parse = "parse";
    samyojana = "samyojana";
    sanchaya = "sanchaya";
    sanskrit_text = "sanskrit_text";
    sarani = "sarani";
    shrinkhala = "shrinkhala";
    unparse = "unparse";
    utsarjana = "utsarjana";
    vakyavibhaga = "vakyavibhaga";
    vastu = "vastu";
    vishlesana = "vishlesana";
    yantrotsarjana = "yantrotsarjana";
}

/// A source added to the corpus without a twin here would be walked by no
/// test, silently. This runs without `T1_CORPUS`: it is a listing, not a walk.
#[test]
fn every_corpus_source_has_a_twin() {
    let mut listed: Vec<&str> = SOURCES.to_vec();
    listed.sort_unstable();
    assert_eq!(
        listed,
        corpus(),
        "the twins above must name every corpus source"
    );
}
