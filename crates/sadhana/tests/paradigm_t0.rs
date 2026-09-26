//! The two halves, the call, and the label — measured over the T0 corpus.
//! Task `W-211`, research/23 §2.8 (statistics 28–30) and the call/label
//! statistics the trunk added to the same row (calling convention, labels).
//!
//! # What "the T0 corpus" is, found by trying
//!
//! The row named two corpora: the 48 `spec/*.sas` programs and the 16
//! `tests/corpus/t1/*.सस` files. The second set is written in the T1
//! *language* (`मण्डलम्`, `ध्रुवः`, `वृत्तिः` …), not in Sassembly, and the T0
//! assembler refuses every one of them — `tests/t0_cannot_read_t1.rs` is the
//! standing proof. No Rust chain in the tree turns a T1 source into T0 text
//! (`t1::emit::emit_program` takes an IR `Function`, and nothing builds one
//! from a corpus file), so those 16 contribute no instruction to a T0
//! round trip. The census SAYS SO with a number (`paradigm_halves_t1_corpus_
//! t0_readable`) rather than silently measuring 48 and calling it both.
//!
//! What the T0 assembler does read is `spec/*.sas` and `spec/golden/*.sas`,
//! and both are measured here as the T0 corpus.
//!
//! # 28 — encode / decode round trip
//!
//! For every instruction `x` of every program: assemble the program
//! (`encode_object_for`, the path `sadhana::assemble` takes), find `x`'s bytes
//! by `layout_addresses`, take them apart with `विश्लेषणम्` (`decode_at`) and
//! put them back with `reassemble`. Three things must hold, and each failure
//! is listed with `file:line`:
//!
//! 1. the bytes come back byte-exact;
//! 2. the decoded row belongs to the family `x` was written in;
//! 3. every register `x` names is among the registers the decoder read out.
//!
//! The third is what makes this "reproduce `x`" and not merely "reproduce the
//! word": a decoder that reads the right bits into the wrong number would pass
//! the first two.
//!
//! When this census was first run, `reassemble` had no 16-bit half —
//! `विश्लेषणम्` decoded a compressed form (`decode16`) and could not put one
//! back — and the compressed target was measured with a test-local inverse.
//! `W-233` gave the assembler its own `reassemble16`, and the census reads it
//! from there, so no assembler code lives here.
//!
//! # 29 — parse / print
//!
//! T1 has no unparser. The census searches the T1 sources and the Rust T1
//! compiler for a routine whose name says it prints an AST back to source and
//! finds none; the statistic is reported UNMEASURABLE and a row is filed for
//! the inverse, not a proxy.
//!
//! # 30 — twin agreement
//!
//! The twin tables in `crates/sadhana-t1/tests/t1_sources.rs` name the pairs
//! (Rust routine ↔ T1 routine). This census reads those tables out of the
//! file's text — so the pairs cannot go stale against it — and, for each pair
//! that is a runnable routine, asks whether any `#[test]` in the tree runs BOTH
//! twins and asserts on the result. It does not write those tests; it counts
//! their absence.
//!
//! # The call and the label
//!
//! At every `jal`/`jalr`/`ecall` site the census looks `WINDOW` instructions
//! back and forward in written order and records which registers were written
//! before and which were read first after. Writes and reads come from the
//! decoded encoding's field masks (`rd` written; `rs1`/`rs2`/`rs3` read), never
//! from a hand table. Labels are counted by in-degree and by direction: a text
//! reference to a label at or before the referring instruction is backward —
//! the machine's `म`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sadhana::encode::{
    EncodeError, Target, compressed_at, encode_object_for, encode_program_for, encodings,
    layout_addresses, register,
};
use sadhana::parse::{Program, Section, assemble_program};
use sadhana::vishlesana::{decode_at, reassemble, reassemble16};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// Every file under `dir` with `ext`, sorted, as (path relative to the root,
/// contents).
fn sources(dir: &str, ext: &str) -> Vec<(String, String)> {
    let base = root().join(dir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("read {}: {e}", base.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("read source");
            (
                format!("{dir}/{}", p.file_name().unwrap().to_string_lossy()),
                text,
            )
        })
        .collect()
}

/// The T0 corpus: `spec/*.sas` and `spec/golden/*.sas`.
fn t0_corpus() -> Vec<(String, String)> {
    let mut all = sources("spec", "sas");
    all.extend(sources("spec/golden", "sas"));
    all
}

// ── 28: the round trip ─────────────────────────────────────────────────────

/// What one program's round trip found.
#[derive(Debug, Default)]
struct RoundTrip {
    checked: usize,
    failures: Vec<String>,
}

/// Round-trip every instruction of `program` against the `text` bytes it was
/// encoded into, `addresses` giving each instruction's offset.
///
/// Takes the bytes rather than making them so a test can hand it a corrupted
/// image and see the census go red.
fn round_trip_bytes(name: &str, program: &Program, text: &[u8], addresses: &[u32]) -> RoundTrip {
    let mut out = RoundTrip::default();
    for (i, inst) in program.instructions.iter().enumerate() {
        let where_ = format!("{name}:{} ({})", inst.line, inst.family.name);
        let Some(off) = addresses.get(i).map(|a| *a as usize) else {
            out.failures.push(format!("{where_} has no address"));
            continue;
        };
        let Some((d, width)) = decode_at(text, off) else {
            let shown: Vec<String> = text
                .get(off..(off + 4).min(text.len()))
                .unwrap_or(&[])
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            out.failures.push(format!(
                "{where_} did not decode: bytes {}",
                shown.join(" ")
            ));
            continue;
        };
        let original = &text[off..off + width];
        let back: Option<Vec<u8>> = if width == 4 {
            reassemble(&d).map(|w| w.to_le_bytes().to_vec())
        } else {
            reassemble16(&d).map(|h| h.to_le_bytes().to_vec())
        };
        match back {
            None => {
                out.failures.push(format!(
                    "{where_} decoded to {} and would not reassemble",
                    d.insn
                ));
                continue;
            }
            Some(b) if b != original => {
                out.failures.push(format!(
                    "{where_} came back different via {}: {:02x?} -> {:02x?}",
                    d.insn, original, b
                ));
                continue;
            }
            Some(_) => {}
        }
        if d.family != inst.family.key {
            out.failures.push(format!(
                "{where_} decoded as {} of family {}, written as family {}",
                d.insn, d.family, inst.family.key
            ));
            continue;
        }
        // Every register the author wrote must be among the registers the
        // decoder read out. A set, not a multiset: a compressed form drops a
        // redundant operand (`c.add rd, rs2` is `add rd, rd, rs2`) — and the
        // operand it drops may be the hardwired zero (`c.li` is `addi rd, x0,
        // imm`), which is a fact of the form and not a register lost.
        let decoded_regs: BTreeSet<(bool, i64)> = d
            .operands
            .iter()
            .filter(|(k, _)| k == "reg" || k == "freg")
            .map(|(k, v)| (k == "freg", *v))
            .collect();
        let missing: Vec<String> = inst
            .operands
            .iter()
            .filter(|o| !o.is_numeral)
            .filter_map(|o| register(&o.base).map(|(n, f)| (o.base.clone(), (f, i64::from(n)))))
            .filter(|(_, r)| !(width == 2 && *r == (false, 0)))
            .filter(|(_, r)| !decoded_regs.contains(r))
            .map(|(b, _)| b)
            .collect();
        if !missing.is_empty() {
            out.failures.push(format!(
                "{where_} via {}: written register(s) {} not among decoded operands {:?}",
                d.insn,
                missing.join(", "),
                d.operands
            ));
            continue;
        }
        out.checked += 1;
    }
    out
}

/// Assemble `program` for `target` and round-trip it.
fn round_trip(
    name: &str,
    program: &Program,
    target: Target,
) -> Result<RoundTrip, Vec<EncodeError>> {
    let (text, _pending) = encode_object_for(program, target)?;
    let addresses = layout_addresses(program, target);
    Ok(round_trip_bytes(name, program, &text, &addresses))
}

// ── 30: the twin tables ────────────────────────────────────────────────────

/// One row of a twin table: (table name, Rust name, T1 name).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Twin {
    table: String,
    rust: String,
    t1: String,
}

/// Read every `const NAME: &[(&str, &str)] = &[ ("rust", "t1"), … ];` table
/// out of a file's text.
///
/// Field tables (`*_FIELDS`) name struct members, not routines, and are left
/// out; so are the three-column and nested tables, which the type text
/// excludes by itself.
///
/// # Errors
/// When the text holds no such table at all: a census reporting 0 of 0 would
/// be a lie with a green tick, so this refuses instead.
fn twin_tables_in(text: &str) -> Result<Vec<Twin>, String> {
    let mut out = Vec::new();
    let mut tables = 0usize;
    let mut rest = text;
    while let Some(at) = rest.find("const ") {
        rest = &rest[at + "const ".len()..];
        let Some(colon) = rest.find(':') else { break };
        let name = rest[..colon].trim();
        if !rest[colon..].starts_with(": &[(&str, &str)] = &[") {
            continue;
        }
        let Some(end) = rest.find("];") else { break };
        let body = &rest[colon..end];
        tables += 1;
        if name.ends_with("_FIELDS") {
            continue;
        }
        let mut b = body;
        while let Some(open) = b.find("(\"") {
            b = &b[open + 2..];
            let Some(q1) = b.find('"') else { break };
            let rust = &b[..q1];
            b = &b[q1 + 1..];
            let Some(open2) = b.find('"') else { break };
            b = &b[open2 + 1..];
            let Some(q2) = b.find('"') else { break };
            let t1 = &b[..q2];
            b = &b[q2 + 1..];
            out.push(Twin {
                table: name.to_string(),
                rust: rust.to_string(),
                t1: t1.to_string(),
            });
        }
    }
    if tables == 0 {
        return Err("no `&[(&str, &str)]` twin table in the text".into());
    }
    Ok(out)
}

/// Whether a Rust-side name is a routine that can be run — as opposed to a
/// type, a constant, a variant, a field access or a described loop.
fn is_routine(rust: &str) -> bool {
    if rust
        .chars()
        .any(|c| c == '(' || c == '[' || c == '.' || c == ' ' || c == '<')
    {
        return false;
    }
    let last = rust.rsplit("::").next().unwrap_or(rust);
    let Some(first) = last.chars().next() else {
        return false;
    };
    first.is_ascii_lowercase()
}

// ── 30b: what shape is the pair, and could a test run it? (`W-246`) ──────
//
// Statistic 30 counts the agreement tests over the pairs `is_routine` calls
// routines. That denominator answers one question — how many pairs does a
// test run both halves of — and hides the two the row asks: WHAT KIND of
// thing does each pair name, and could a test run it AT ALL. A pair whose
// Rust half is a private `fn` in another crate is not an untested routine; it
// is a routine no integration test can reach, and counting it beside one that
// is merely untested makes the rate mean less than it appears to.
//
// The five shapes, each decided from BOTH halves and never from the Rust name
// alone (`is_routine` reads the Rust name alone, and that is how fourteen
// FIELDS — `Program::instructions ↔ आज्ञाकोश` — entered the routine count):
//
//  * ROUTINE — the T1 half is a `वृत्तिः` and the Rust half a `fn`. Agreement:
//    run both on one input and compare the answers.
//  * TYPE — the T1 half is a `संरचना`, a `गणना` or a `यन्त्रम्`. Agreement:
//    the two carry the same members, in the same order, with the same types.
//  * DISCRIMINANT — the T1 half is a scalar `चरः` holding a number and the
//    Rust half is a variant or a sign (`("।", "दण्डभेद")`). Agreement: both
//    halves put the same input under the same code.
//  * ARENA — the T1 half is an `अङ्कः अन्तः` arena and the Rust half a field.
//    Agreement: after one run over one input the two hold the same rows.
//  * DESCRIBED — the Rust half is PROSE naming a fragment of a function
//    ("encode_collecting's placement loop"). Nothing can call it, so no
//    agreement test can exist and its absence is not a debt.
//
// research/27 states the rule for each shape and names the ones that cannot
// carry a test; this census is the measurement that document is written from.

/// What kind of thing a twin pair names — the question that decides what an
/// agreement test for it could be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    Routine,
    Inlined,
    Type,
    Discriminant,
    Arena,
    Described,
}

impl Shape {
    fn label(self) -> &'static str {
        match self {
            Shape::Routine => "routine",
            Shape::Inlined => "inlined",
            Shape::Type => "type",
            Shape::Discriminant => "discriminant",
            Shape::Arena => "arena",
            Shape::Described => "described",
        }
    }
}

/// One T1 declaration as the corpus spells it.
#[derive(Debug, Clone)]
struct T1Decl {
    file: String,
    /// `वृत्तिः`, `चरः`, `संरचना`, `गणना` or `यन्त्रम्`.
    keyword: &'static str,
    public: bool,
    /// Parameters, counted by the `ॱॱ` type marks between `आदाय` and `ददाति`.
    params: usize,
    /// A `चरः` whose type is `अङ्कः अन्तः …`, which is an arena and not a scalar.
    arena: bool,
    /// What a `वृत्तिः` answers: the text between `ददाति` and `आदि`.
    returns: String,
}

const T1_KEYWORDS: &[&str] = &["वृत्तिः", "चरः", "संरचना", "गणना", "यन्त्रम्"];

/// Every declaration of every `.t1` source, by name.
///
/// A name declared twice keeps its FIRST reading (`W-247` found one such pair
/// in `encode.t1`); both readings have the same shape, so the classification
/// below does not turn on which is kept.
fn t1_declarations() -> BTreeMap<String, T1Decl> {
    let mut out: BTreeMap<String, T1Decl> = BTreeMap::new();
    for (path, text) in sources("crates/sadhana-t1/src", "t1") {
        let file = path.rsplit('/').next().unwrap_or(&path).to_string();
        for line in text.lines() {
            // TOP LEVEL ONLY. A `चरः` indented inside a routine is a LOCAL,
            // and `चरः आरम्भः` appears as a local in nine files; reading those
            // as declarations made `IrBuilder::new ↔ आरम्भः` a scalar and the
            // whole pair a discriminant. A module declaration starts its line.
            if line.starts_with(char::is_whitespace) {
                continue;
            }
            let t = line;
            let (public, rest) = match t.strip_prefix("सार्वजनिक ") {
                Some(r) => (true, r.trim_start()),
                None => (false, t),
            };
            let Some((keyword, tail)) = T1_KEYWORDS
                .iter()
                .find_map(|k| rest.strip_prefix(k)?.strip_prefix(' ').map(|r| (*k, r)))
            else {
                continue;
            };
            let name = tail
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('।');
            if name.is_empty() {
                continue;
            }
            let params = match (tail.find(" आदाय "), tail.find(" ददाति ")) {
                (Some(a), Some(d)) if a < d => tail[a..d].matches("ॱॱ").count(),
                _ => 0,
            };
            let returns = match (tail.find(" ददाति "), tail.find(" आदि")) {
                (Some(d), Some(e)) if d < e => tail[d + " ददाति ".len()..e].trim().to_string(),
                _ => String::new(),
            };
            out.entry(name.to_string()).or_insert(T1Decl {
                file: file.clone(),
                keyword,
                public,
                params,
                arena: tail.contains("ॱॱ अङ्कः अन्तः"),
                returns,
            });
        }
    }
    out
}

/// The Rust half of a pair, as every crate's `src` spells it.
///
/// `public_fns` is what an INTEGRATION test can call: `pub fn`. A `pub(crate)`
/// or a bare `fn` goes to `private_fns`, because `crates/*/tests` is another
/// crate and cannot see it — the distinction the rate turns on.
#[derive(Debug, Default)]
struct RustNames {
    public_fns: BTreeSet<String>,
    private_fns: BTreeSet<String>,
    fields: BTreeSet<String>,
    types: BTreeSet<String>,
    /// What each `fn` answers: the text between `->` and the body.
    returns: BTreeMap<String, String>,
}

/// Every `.rs` file under any `crates/*/src`, recursively.
fn rust_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs")
                && let Ok(text) = std::fs::read_to_string(&p)
            {
                out.push((p.display().to_string(), text));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root().join("crates"), &mut out);
    out.retain(|(path, _)| path.contains("/src/"));
    out
}

fn rust_names() -> RustNames {
    let mut n = RustNames::default();
    for (_, text) in rust_sources() {
        for line in text.lines() {
            let t = line.trim_start();
            if let Some(at) = t.find("fn ") {
                let head = t[..at].trim_end();
                let word = |after: &str| -> String {
                    after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect()
                };
                let name = word(&t[at + "fn ".len()..]);
                let is_signature = head.is_empty()
                    || head.split_whitespace().all(|w| {
                        matches!(
                            w,
                            "pub" | "const" | "unsafe" | "async" | "extern" | "default"
                        ) || w.starts_with("pub(")
                            || w.starts_with('"')
                    });
                if !name.is_empty() && is_signature {
                    if let Some(arrow) = t.find("->") {
                        let answer = t[arrow + 2..]
                            .trim()
                            .trim_end_matches('{')
                            .trim()
                            .trim_end_matches(';')
                            .to_string();
                        n.returns.entry(name.clone()).or_insert(answer);
                    }
                    // `pub fn` and nothing else is reachable from another crate.
                    if head.split_whitespace().next() == Some("pub") {
                        n.public_fns.insert(name);
                    } else {
                        n.private_fns.insert(name);
                    }
                }
            }
            if let Some(rest) = t.strip_prefix("pub ") {
                if let Some(colon) = rest.find(':') {
                    let name = rest[..colon].trim();
                    if !name.is_empty()
                        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                        && !name.starts_with("fn")
                    {
                        n.fields.insert(name.to_string());
                    }
                }
                for kw in ["struct ", "enum ", "type ", "trait "] {
                    if let Some(after) = rest.strip_prefix(kw) {
                        let name: String = after
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .collect();
                        if !name.is_empty() {
                            n.types.insert(name);
                        }
                    }
                }
            }
        }
    }
    n
}

/// The shape of one pair, read from both halves.
fn shape_of(twin: &Twin, decls: &BTreeMap<String, T1Decl>, rust: &RustNames) -> Shape {
    // A space, a call, an index or a generic in the Rust half means the row
    // names a FRAGMENT of a function and not an item that could be named.
    if twin
        .rust
        .chars()
        .any(|c| c == '(' || c == ' ' || c == '[' || c == '<')
    {
        return Shape::Described;
    }
    let last = twin.rust.rsplit("::").next().unwrap_or(&twin.rust);
    let is_fn = rust.public_fns.contains(last) || rust.private_fns.contains(last);
    match decls.get(&twin.t1) {
        // A T1 routine whose Rust half is NOT a function of its own: a `.push`
        // written at the site, a `const` table, a field's `.len`. Rust does
        // INLINE what T1 had to name, and an agreement test is still possible
        // — the test writes the Rust expression itself.
        Some(d) if d.keyword == "वृत्तिः" && !is_fn => Shape::Inlined,
        Some(d) if d.keyword == "वृत्तिः" => Shape::Routine,
        Some(d) if d.keyword == "चरः" && d.arena => Shape::Arena,
        Some(d) if d.keyword == "चरः" => Shape::Discriminant,
        Some(_) => Shape::Type,
        // Not declared at the top level of any source: the Rust half decides.
        None if rust.fields.contains(last) => Shape::Arena,
        None if rust.types.contains(last) => Shape::Type,
        None if is_fn => Shape::Routine,
        None => Shape::Discriminant,
    }
}

/// Whether a test could run both halves of a ROUTINE pair, and whether one
/// already runs the T1 half.
///
/// Named for the ACCESS a test has to the pair. Two names in this file were
/// taken already — `Reach` is one test's reach through its helpers and
/// `Access` is a register access of the call census — so this one carries the
/// subject it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TwinAccess {
    /// The Rust twin is a `fn` no other crate can call. An agreement test is
    /// possible only INSIDE the crate (`crates/sadhana/src`, where the
    /// interpreter also lives) or after the item is made `pub`.
    RustHalfPrivate,
    /// No `fn` of that name anywhere: the twin table names something the Rust
    /// side no longer has, or never spelled as a function.
    RustHalfAbsent,
    /// The T1 twin is not `सार्वजनिक`, so `Interpreter::call` cannot name it.
    T1HalfPrivate,
    /// No top-level T1 declaration of that name: the pair names a routine the
    /// T1 side does not have, or has under another spelling.
    T1HalfAbsent,
    /// Both halves are open AND some test already calls the T1 half. One
    /// assertion away from agreement — the cheapest tests in the tree.
    OpenAndExercised,
    /// Both halves are open and no test calls the T1 half at all.
    OpenAndUntouched,
}

impl TwinAccess {
    fn label(self) -> &'static str {
        match self {
            TwinAccess::RustHalfPrivate => "rust_half_private",
            TwinAccess::RustHalfAbsent => "rust_half_absent",
            TwinAccess::T1HalfPrivate => "t1_half_private",
            TwinAccess::T1HalfAbsent => "t1_half_absent",
            TwinAccess::OpenAndExercised => "open_and_exercised",
            TwinAccess::OpenAndUntouched => "open_and_untouched",
        }
    }
}

fn access_of(
    twin: &Twin,
    decls: &BTreeMap<String, T1Decl>,
    rust: &RustNames,
    called: bool,
) -> TwinAccess {
    let last = twin.rust.rsplit("::").next().unwrap_or(&twin.rust);
    if !rust.public_fns.contains(last) {
        return if rust.private_fns.contains(last) {
            TwinAccess::RustHalfPrivate
        } else {
            TwinAccess::RustHalfAbsent
        };
    }
    match decls.get(&twin.t1) {
        None => TwinAccess::T1HalfAbsent,
        Some(d) if !d.public => TwinAccess::T1HalfPrivate,
        _ if called => TwinAccess::OpenAndExercised,
        _ => TwinAccess::OpenAndUntouched,
    }
}

/// A function in a test file: its name, its body, and the last `ॱ`-segment of
/// every string literal it hands to `call(`.
#[derive(Debug, Clone)]
struct Routine {
    name: String,
    is_test: bool,
    body: String,
    t1_called: BTreeSet<String>,
}

/// Every `fn` in a file's text, with its body found by brace matching.
fn routines_in(text: &str) -> Vec<Routine> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("fn ") {
        let at = from + rel;
        from = at + 3;
        // `fn ` must start a token: not `lifn `, and not inside a word.
        if at > 0 && bytes[at - 1].is_ascii_alphanumeric() {
            continue;
        }
        let sig = &text[at + 3..];
        let name: String = sig
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        let Some(open_rel) = sig.find('{') else { break };
        // A `;` before the `{` means a declaration with no body (a trait).
        if sig[..open_rel].contains(';') {
            continue;
        }
        let open = at + 3 + open_rel;
        let mut depth = 0i32;
        let mut close = None;
        for (i, c) in text[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close) = close else { break };
        let body = text[open..=close].to_string();
        // `#[test]` within the attributes just before this `fn`.
        let before = &text[..at];
        let attrs_start = before.rfind("\n\n").map_or(0, |p| p + 2);
        let attrs = &before[attrs_start..];
        let is_test = attrs.contains("#[test]");
        let t1_called = call_literals(&body);
        out.push(Routine {
            name,
            is_test,
            body,
            t1_called,
        });
        from = close;
    }
    out
}

/// The last `ॱ`-segment of every QUALIFIED T1 NAME a body holds —
/// `"सङ्केतनॱसङ्केतनम्"` names the routine `सङ्केतनम्`.
///
/// This read every literal written DIRECTLY AFTER `call(` until `W-246`, and
/// that missed a calling style the tree now uses: `t1_twin_agreement.rs` hands
/// the name to a comparison helper (`disagreements(it, "…ॱ…", inputs, …)`)
/// which calls it, so the literal is in the test and the `call(` is in the
/// helper. Five agreement tests written for this row were invisible to the
/// census that asked for them.
///
/// A name mentioned in a MESSAGE and never called is a false positive of this
/// reader. It is bounded: statistic 30 counts a pair only when the test also
/// runs the Rust twin and asserts, and the reader is a measurement, not a
/// ratchet. The narrower rule was wrong in the direction that matters — it
/// reported work as undone.
fn call_literals(body: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = body;
    while let Some(at) = rest.find('"') {
        rest = &rest[at + 1..];
        let Some(end) = rest.find('"') else { break };
        let literal = &rest[..end];
        rest = &rest[end + 1..];
        if !literal.contains('ॱ') || literal.contains('{') {
            continue;
        }
        let name = literal.rsplit('ॱ').next().unwrap_or("").trim();
        if !name.is_empty() {
            out.insert(name.to_string());
        }
    }
    out
}

/// Whether `body` calls the Rust twin: by its `sadhana::` path, by the bare
/// name when the file imports it from `sadhana`, or THROUGH A MODULE the file
/// imported from `sadhana`.
///
/// The third way was added by `W-246` and it was not a corner case. A file
/// that writes `use sadhana::t1::riscv64;` and then `riscv64::register_name(n)`
/// calls the twin as plainly as either other form, and the reader saw neither:
/// the path in the body is not the table's spelling (`register_name`, not
/// `riscv64::register_name`) and the import line names the MODULE, not the
/// function. Five agreement tests already in the tree were counted as absent
/// for this reason, `W-236`'s own `decode16` test among them.
fn calls_rust(body: &str, imports: &str, rust: &str) -> bool {
    let last = rust.rsplit("::").next().unwrap_or(rust);
    let qualified = format!("{rust}(");
    if body.contains(&qualified) && body.contains("sadhana::") {
        return true;
    }
    let bare = format!("{last}(");
    if imports.contains(last) && body.contains(&bare) {
        return true;
    }
    // `use sadhana::t1::riscv64::{self, …};` names `riscv64`; the call is
    // `riscv64::register_name(…)`.
    imports
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|seg| !seg.is_empty())
        .any(|seg| body.contains(&format!("{seg}::{bare}")))
}

/// One asserting test, with what it reaches through the helpers of its own
/// file folded in: every T1 routine it calls, and the text of every body it
/// runs.
struct Reach {
    name: String,
    t1_called: BTreeSet<String>,
    bodies: String,
}

/// A test file, scanned once: its `use sadhana` lines and its asserting tests.
///
/// Scanned ONCE and reused across every twin pair: the first draft rescanned
/// each file per pair and spent the better part of an hour in `contains`.
struct Scanned {
    imports: String,
    tests: Vec<Reach>,
}

fn scan(text: &str) -> Scanned {
    let routines = routines_in(text);
    let helpers: Vec<&Routine> = routines.iter().filter(|r| !r.is_test).collect();
    let tests = routines
        .iter()
        .filter(|r| r.is_test && r.body.contains("assert"))
        .map(|r| {
            let mut t1_called = r.t1_called.clone();
            let mut bodies = r.body.clone();
            for h in helpers
                .iter()
                .filter(|h| r.body.contains(&format!("{}(", h.name)))
            {
                t1_called.extend(h.t1_called.iter().cloned());
                bodies.push('\n');
                bodies.push_str(&h.body);
            }
            Reach {
                name: r.name.clone(),
                t1_called,
                bodies,
            }
        })
        .collect();
    Scanned {
        imports: text
            .lines()
            .filter(|l| l.trim_start().starts_with("use sadhana"))
            .collect::<Vec<_>>()
            .join("\n"),
        tests,
    }
}

/// The test functions of a scanned file that run both halves of `twin` — the
/// T1 name through `call(` (directly or through one helper of the same file)
/// and the Rust name by path or import — and assert.
fn tests_running_both_in(s: &Scanned, twin: &Twin) -> Vec<String> {
    s.tests
        .iter()
        .filter(|t| t.t1_called.contains(&twin.t1) && calls_rust(&t.bodies, &s.imports, &twin.rust))
        .map(|t| t.name.clone())
        .collect()
}

/// As [`tests_running_both_in`], from the file's text.
fn tests_running_both(text: &str, twin: &Twin) -> Vec<String> {
    tests_running_both_in(&scan(text), twin)
}

/// Every test file the twins could be exercised in.
///
/// `crates/yantra/tests` was NOT in this list until `W-246`. It holds the
/// longest chain in the tree — `paradigm_encode.rs` takes a source from lex to
/// a program on the machine — so a pair agreed on there was invisible to a
/// census asking who agrees. MEASURED: adding it changed no count, because
/// that file drives the T1 side through helpers and compares whole outputs
/// rather than running one pair's two halves. The directory belongs in the
/// list on its merits and the widening is reported as having found nothing,
/// which is what it found.
fn test_files() -> Vec<(String, String)> {
    let mut all = sources("crates/sadhana-t1/tests", "rs");
    all.extend(sources("crates/sadhana/tests", "rs"));
    all.extend(sources("crates/yantra/tests", "rs"));
    all
}

// ── 29: the unparser, searched for ─────────────────────────────────────────

/// Names an unparser would carry, in either language. Any routine so named in
/// the T1 compiler (Rust or T1) makes statistic 29 measurable and this list
/// out of date.
const UNPARSER_NAMES: &[&str] = &[
    "unparse",
    "pretty_print",
    "to_source",
    "print_program",
    "print_ast",
    "format_program",
    "मुद्रणम्",
    "पुनर्लेखनम्",
    "स्रोतलेखनम्",
    "वृक्षमुद्रणम्",
];

/// Routine definitions in the T1 compiler whose name is an unparser's.
fn unparser_candidates() -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in sources("crates/sadhana/src/t1", "rs") {
        for n in UNPARSER_NAMES {
            if text.contains(&format!("fn {n}(")) {
                found.push(format!("{name}: fn {n}"));
            }
        }
    }
    for (name, text) in sources("crates/sadhana-t1/src", "t1") {
        for n in UNPARSER_NAMES {
            if text.contains(&format!("वृत्तिः {n}")) {
                found.push(format!("{name}: वृत्तिः {n}"));
            }
        }
    }
    found
}

// ── the census ─────────────────────────────────────────────────────────────

fn rate(num: usize, den: usize) -> String {
    if den == 0 {
        "0.00".into()
    } else {
        format!("{:.2}", num as f64 * 100.0 / den as f64)
    }
}

/// Statistics 28, 29 and 30. Measurement, not a ratchet: it prints and lists.
#[test]
#[ignore = "measurement"]
fn measure_corpus_halves() {
    let started = std::time::Instant::now();
    // ── 28 ──
    let corpus = t0_corpus();
    let mut programs: Vec<(String, Program)> = Vec::new();
    let mut unparsed = Vec::new();
    for (name, text) in &corpus {
        match assemble_program(text) {
            Ok(p) => programs.push((name.clone(), p)),
            Err(es) => unparsed.push(format!(
                "{name}: {}",
                es.first().cloned().unwrap_or_default()
            )),
        }
    }
    println!("METRIC paradigm_halves_t0_programs {}", corpus.len());
    println!(
        "METRIC paradigm_halves_t0_programs_parsed {}",
        programs.len()
    );
    for u in &unparsed {
        println!("  refused by the parser: {u}");
    }

    for (label, target) in [
        ("", Target::Uncompressed),
        ("compressed_", Target::Compressed),
    ] {
        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();
        let mut refused: Vec<String> = Vec::new();
        let mut instructions = 0usize;
        let mut sixteen = 0usize;
        for (name, p) in &programs {
            match round_trip(name, p, target) {
                Ok(r) => {
                    instructions += p.instructions.len();
                    checked += r.checked;
                    failures.extend(r.failures);
                    if target == Target::Compressed {
                        let addresses = layout_addresses(p, target);
                        sixteen += addresses.windows(2).filter(|w| w[1] - w[0] == 2).count();
                    }
                }
                Err(es) => refused.push(format!(
                    "{name}:{} {}",
                    es[0].line,
                    es[0].message(sadhana::nidana::Language::English)
                )),
            }
        }
        println!(
            "METRIC paradigm_halves_{label}programs_encoded {}",
            programs.len() - refused.len()
        );
        println!(
            "METRIC paradigm_halves_{label}programs_refused_by_encoder {}",
            refused.len()
        );
        for r in &refused {
            println!("  refused by the encoder: {r}");
        }
        println!("METRIC paradigm_halves_{label}instructions {instructions}");
        if target == Target::Compressed {
            println!("METRIC paradigm_halves_compressed_sixteen_bit_forms {sixteen}");
        }
        println!("METRIC paradigm_halves_{label}roundtrip_ok {checked}");
        println!(
            "METRIC paradigm_halves_{label}roundtrip_failures {}",
            failures.len()
        );
        println!(
            "METRIC paradigm_halves_{label}roundtrip_rate {}",
            rate(checked, checked + failures.len())
        );
        for f in &failures {
            println!("  EXCEPTION {label}{f}");
        }
        println!(
            "  elapsed after {label}round trip: {:.0} s",
            started.elapsed().as_secs_f64()
        );
    }

    // The other corpus the row named, and why it contributes nothing here.
    let t1 = sources("tests/corpus/t1", "सस");
    let readable = t1
        .iter()
        .filter(|(_, text)| assemble_program(text).is_ok())
        .count();
    println!("METRIC paradigm_halves_t1_corpus_files {}", t1.len());
    println!("METRIC paradigm_halves_t1_corpus_t0_readable {readable}");
    // `sadhana::vishlesana::reassemble16` is what the round trip above called;
    // the compressed target was measured with the assembler's own inverse,
    // not a test-local one (`W-233`).
    println!("METRIC paradigm_halves_reassemble_sixteen_bit_in_tree 1");

    // ── 29 ──
    let candidates = unparser_candidates();
    println!(
        "METRIC paradigm_halves_unparser_candidates {}",
        candidates.len()
    );
    for c in &candidates {
        println!("  unparser candidate: {c}");
    }
    if candidates.is_empty() {
        println!("  paradigm_halves_parse_print_rate UNMEASURABLE: T1 has no unparser (rule X4)");
    } else {
        println!(
            "  paradigm_halves_parse_print_rate: measured by `measure_corpus_parse_print` \
             (crates/sadhana-t1/tests/t1_execution.rs), W-215"
        );
    }

    // ── 30 ──
    let sources_text =
        std::fs::read_to_string(root().join("crates/sadhana-t1/tests/t1_sources.rs"))
            .expect("the twin tables exist");
    let twins = twin_tables_in(&sources_text).expect("the twin tables are readable");
    // Distinct (Rust, T1) pairs: `encode ↔ सङ्केतनम्` sits in two tables and
    // is one pair. The per-table lines below still count it in each.
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    let routines: Vec<&Twin> = twins
        .iter()
        .filter(|t| is_routine(&t.rust))
        .filter(|t| seen.insert((t.rust.as_str(), t.t1.as_str())))
        .collect();
    let files: Vec<(String, Scanned)> = test_files()
        .into_iter()
        .map(|(name, text)| (name, scan(&text)))
        .collect();
    let mut with = 0usize;
    let mut without: Vec<String> = Vec::new();
    let mut by_table: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for t in &routines {
        let mut where_: Vec<String> = Vec::new();
        for (name, s) in &files {
            for test in tests_running_both_in(s, t) {
                where_.push(format!("{name}::{test}"));
            }
        }
        let has = !where_.is_empty();
        for table in twins
            .iter()
            .filter(|u| u.rust == t.rust && u.t1 == t.t1)
            .map(|u| u.table.as_str())
        {
            let e = by_table.entry(table).or_default();
            if has {
                e.0 += 1;
            } else {
                e.1 += 1;
            }
        }
        if has {
            with += 1;
            println!(
                "  agreement test: {} ↔ {} in {}",
                t.rust,
                t.t1,
                where_.join(", ")
            );
        } else {
            without.push(format!("{} — {} ↔ {}", t.table, t.rust, t.t1));
        }
    }
    println!("  elapsed: {:.0} s", started.elapsed().as_secs_f64());
    println!("METRIC paradigm_halves_twin_pairs {}", twins.len());
    println!(
        "METRIC paradigm_halves_twin_routine_pairs {}",
        routines.len()
    );
    println!("METRIC paradigm_halves_twin_pairs_with_agreement_test {with}");
    println!(
        "METRIC paradigm_halves_twin_pairs_without_agreement_test {}",
        without.len()
    );
    println!(
        "METRIC paradigm_halves_twin_agreement_test_rate {}",
        rate(with, routines.len())
    );
    for (table, (w, wo)) in &by_table {
        println!("  {table}: {w} with, {wo} without");
    }
    for w in &without {
        println!("  NO AGREEMENT TEST {w}");
    }

    // ── 30b: the shape of every pair, and what a test could reach (`W-246`) ──
    let decls = t1_declarations();
    let rust = rust_names();
    let called: BTreeSet<&str> = files
        .iter()
        .flat_map(|(_, s)| s.tests.iter())
        .flat_map(|t| t.t1_called.iter())
        .map(String::as_str)
        .collect();
    let mut distinct: BTreeSet<(&str, &str)> = BTreeSet::new();
    let pairs: Vec<&Twin> = twins
        .iter()
        .filter(|t| distinct.insert((t.rust.as_str(), t.t1.as_str())))
        .collect();
    let mut by_shape: BTreeMap<&str, usize> = BTreeMap::new();
    let mut tested_by_shape: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_reach: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_file: BTreeMap<String, usize> = BTreeMap::new();
    let mut buildable: Vec<String> = Vec::new();
    let mut out_of_reach: Vec<String> = Vec::new();
    let mut miscounted: Vec<String> = Vec::new();
    let mut open_pairs = 0usize;
    let mut open_with_test = 0usize;
    for t in &pairs {
        let sh = shape_of(t, &decls, &rust);
        *by_shape.entry(sh.label()).or_default() += 1;
        let has_test = files
            .iter()
            .any(|(_, s)| !tests_running_both_in(s, t).is_empty());
        if has_test {
            *tested_by_shape.entry(sh.label()).or_default() += 1;
        }
        // The old reader decides from the Rust name alone. Where the two
        // disagree, `is_routine` is counting something that is not a routine
        // (a field, a variant) in the denominator of the published rate.
        if is_routine(&t.rust) != (sh == Shape::Routine) {
            miscounted.push(format!(
                "{} ↔ {} — is_routine says {}, both halves say {}",
                t.rust,
                t.t1,
                is_routine(&t.rust),
                sh.label()
            ));
        }
        if sh != Shape::Routine {
            continue;
        }
        let r = access_of(t, &decls, &rust, called.contains(t.t1.as_str()));
        *by_reach.entry(r.label()).or_default() += 1;
        let file = decls
            .get(&t.t1)
            .map_or_else(|| "-".to_string(), |d| d.file.clone());
        match r {
            TwinAccess::OpenAndExercised | TwinAccess::OpenAndUntouched => {
                open_pairs += 1;
                if has_test {
                    open_with_test += 1;
                } else {
                    *by_file.entry(file.clone()).or_default() += 1;
                    buildable.push(format!(
                        "{file}: {} ↔ {} ({}, {} parameters)",
                        t.rust,
                        t.t1,
                        r.label(),
                        decls.get(&t.t1).map_or(0, |d| d.params)
                    ));
                }
            }
            _ => out_of_reach.push(format!("{file}: {} ↔ {} — {}", t.rust, t.t1, r.label())),
        }
    }
    println!("METRIC paradigm_halves_twin_pairs_distinct {}", pairs.len());
    for (shape, n) in &by_shape {
        println!("METRIC paradigm_halves_twin_shape_{shape} {n}");
        println!(
            "METRIC paradigm_halves_twin_shape_{shape}_with_agreement_test {}",
            tested_by_shape.get(shape).copied().unwrap_or(0)
        );
    }
    for (reach, n) in &by_reach {
        println!("METRIC paradigm_halves_twin_routine_{reach} {n}");
    }
    println!("METRIC paradigm_halves_twin_routine_pairs_open {open_pairs}");
    println!("METRIC paradigm_halves_twin_open_with_agreement_test {open_with_test}");
    println!(
        "METRIC paradigm_halves_twin_agreement_rate_over_open {}",
        rate(open_with_test, open_pairs)
    );
    println!(
        "METRIC paradigm_halves_twin_pairs_miscounted_by_is_routine {}",
        miscounted.len()
    );

    // ── THE OWNERSHIP DIVERGENCE, for `W-250` ──
    //
    // The row after this one asks whether the halves disagree on a semantic no
    // test compares: Rust MOVES or CLONES where T1 SHARES a run. It is visible
    // in the two signatures without reading either body. Where the Rust half
    // answers an OWNED value (a `String`, a `Vec`, an owned container) and the
    // T1 half answers a NUMBER, the T1 answer is an index into shared state and
    // the run itself was written somewhere else. Those are the pairs where
    // `assert_eq!(rust(x), t1(x))` cannot be written at all, and they are the
    // pairs whose agreement test has to read the state as well as the answer.
    let owned = |t: &str| {
        let t = t.trim_start_matches('&');
        t.starts_with("String")
            || t.starts_with("Vec<")
            || t.starts_with("Octets")
            || t.contains("String>")
            || t.contains("Vec<")
    };
    let scalar = |t: &str| matches!(t, "न६४" | "अ६४" | "इ६४" | "अ८" | "बूल");
    let mut divergent: Vec<String> = Vec::new();
    let mut both_owned = 0usize;
    let mut both_scalar = 0usize;
    for t in &pairs {
        if shape_of(t, &decls, &rust) != Shape::Routine {
            continue;
        }
        let last = t.rust.rsplit("::").next().unwrap_or(&t.rust);
        let (Some(r), Some(d)) = (rust.returns.get(last), decls.get(&t.t1)) else {
            continue;
        };
        match (owned(r), scalar(&d.returns)) {
            (true, true) => divergent.push(format!(
                "{}: {} ↔ {} — Rust answers `{r}`, T1 answers `{}`",
                d.file, t.rust, t.t1, d.returns
            )),
            (true, false) => both_owned += 1,
            (false, true) => both_scalar += 1,
            (false, false) => {}
        }
    }
    println!(
        "METRIC paradigm_halves_twin_rust_owns_where_t1_indexes {}",
        divergent.len()
    );
    println!("METRIC paradigm_halves_twin_both_answer_a_run {both_owned}");
    println!("METRIC paradigm_halves_twin_both_answer_a_number {both_scalar}");
    for d in &divergent {
        println!("  OWNERSHIP DIVERGES {d}");
    }
    for m in &miscounted {
        println!("  MISCOUNTED {m}");
    }
    for (file, n) in by_file.iter().collect::<BTreeMap<_, _>>() {
        println!("  buildable in {file}: {n}");
    }
    for b in &buildable {
        println!("  BUILDABLE {b}");
    }
    for u in &out_of_reach {
        println!("  OUT OF REACH {u}");
    }
}

// ── the call and the label ─────────────────────────────────────────────────

/// How far back and forward a call site is read, in instructions.
const WINDOW: usize = 8;

const RD: u32 = 0x0000_0f80;
const RS1: u32 = 0x000f_8000;
const RS2: u32 = 0x01f0_0000;
const RS3: u32 = 0xf800_0000;

/// One instruction's effect on the integer register file, read off the
/// decoded encoding's field masks.
#[derive(Debug, Clone, Default)]
struct Access {
    insn: String,
    writes: BTreeSet<u32>,
    reads: BTreeSet<u32>,
    /// Raw field values, for the call-shape questions (`rd`, `rs1`).
    rd: Option<u32>,
    rs1: Option<u32>,
}

/// The accesses of every instruction of `program`, uncompressed.
fn accesses(program: &Program) -> Result<Vec<Access>, Vec<EncodeError>> {
    let (text, _) = encode_object_for(program, Target::Uncompressed)?;
    let addresses = layout_addresses(program, Target::Uncompressed);
    let all = encodings();
    let mut out = Vec::with_capacity(program.instructions.len());
    // `layout_addresses` ends with the address AFTER the last instruction, so
    // a label standing at the end of the program has somewhere to point. That
    // sentinel is not an instruction and is not walked.
    for off in addresses.into_iter().take(program.instructions.len()) {
        let mut a = Access::default();
        if let Some((d, _)) = decode_at(&text, off as usize)
            && let Some(e) = all.iter().find(|e| e.insn == d.insn && e.bits == 32)
        {
            a.insn = d.insn.clone();
            for (slot, (kind, value)) in e.slots.iter().zip(&d.operands) {
                if kind == "freg" {
                    continue;
                }
                let v = u32::try_from(*value).unwrap_or(0);
                match slot.mask {
                    RD => {
                        a.rd = Some(v);
                        if v != 0 {
                            a.writes.insert(v);
                        }
                    }
                    RS1 => {
                        a.rs1 = Some(v);
                        a.reads.insert(v);
                    }
                    RS2 | RS3 => {
                        a.reads.insert(v);
                    }
                    _ => {}
                }
            }
        }
        out.push(a);
    }
    Ok(out)
}

/// What a window around one site saw.
#[derive(Debug, Default)]
struct Site {
    written_before: BTreeSet<u32>,
    read_first_after: BTreeSet<u32>,
    written_first_after: BTreeSet<u32>,
    /// The register written LAST before the site — for `ecall`, the one
    /// most plausibly carrying the call number.
    last_written_before: Option<u32>,
}

fn window(acc: &[Access], i: usize) -> Site {
    let mut s = Site::default();
    let start = i.saturating_sub(WINDOW);
    for a in &acc[start..i] {
        for w in &a.writes {
            s.written_before.insert(*w);
            s.last_written_before = Some(*w);
        }
    }
    let end = (i + 1 + WINDOW).min(acc.len());
    let mut touched: BTreeSet<u32> = BTreeSet::new();
    for a in &acc[i + 1..end] {
        for r in &a.reads {
            if *r != 0 && touched.insert(*r) {
                s.read_first_after.insert(*r);
            }
        }
        for w in &a.writes {
            if touched.insert(*w) {
                s.written_first_after.insert(*w);
            }
        }
    }
    s
}

/// ABI name of an integer register, from `spec/registers-riscv64.tsv`.
fn abi_names() -> BTreeMap<u32, String> {
    std::fs::read_to_string(root().join("spec/registers-riscv64.tsv"))
        .expect("registers table")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 4 && f[3] == "int").then(|| Some((f[2].parse().ok()?, f[1].to_string())))?
        })
        .collect()
}

/// Per-register tallies over a set of sites.
#[derive(Debug, Default)]
struct Tally {
    sites: usize,
    arg: BTreeMap<u32, usize>,
    read_first: BTreeMap<u32, usize>,
    written_first: BTreeMap<u32, usize>,
    last_written: BTreeMap<u32, usize>,
}

impl Tally {
    fn add(&mut self, s: &Site) {
        self.sites += 1;
        for r in &s.written_before {
            *self.arg.entry(*r).or_default() += 1;
        }
        for r in &s.read_first_after {
            *self.read_first.entry(*r).or_default() += 1;
        }
        for r in &s.written_first_after {
            *self.written_first.entry(*r).or_default() += 1;
        }
        if let Some(r) = s.last_written_before {
            *self.last_written.entry(r).or_default() += 1;
        }
    }

    fn print(&self, what: &str, names: &BTreeMap<u32, String>) {
        println!("  {what}: {} sites, window {WINDOW}", self.sites);
        println!(
            "    reg   written-before  read-first-after  written-first-after  last-written-before"
        );
        let mut regs: BTreeSet<u32> = BTreeSet::new();
        regs.extend(self.arg.keys());
        regs.extend(self.read_first.keys());
        regs.extend(self.written_first.keys());
        for r in regs {
            let n = names.get(&r).cloned().unwrap_or_else(|| format!("x{r}"));
            println!(
                "    {n:5} {:>15} {:>17} {:>20} {:>20}",
                self.arg.get(&r).copied().unwrap_or(0),
                self.read_first.get(&r).copied().unwrap_or(0),
                self.written_first.get(&r).copied().unwrap_or(0),
                self.last_written.get(&r).copied().unwrap_or(0),
            );
        }
    }
}

/// Doc 02 §2.4's declared roles, by ABI name prefix.
fn declared_role(abi: &str) -> &'static str {
    if abi.starts_with('a') {
        "args/return"
    } else if abi.starts_with('s') && abi != "sp" {
        "callee-saved"
    } else if abi.starts_with('t') && abi != "tp" {
        "temporary"
    } else {
        "special"
    }
}

/// The calling convention as the corpus practises it, and the labels as the
/// corpus uses them. Measurement: prints, lists, asserts only that it saw
/// something.
#[test]
#[ignore = "measurement"]
fn measure_corpus_calls_and_labels() {
    let names = abi_names();
    let mut jal_calls = Tally::default();
    let mut jal_jumps = 0usize;
    let mut jalr_calls = Tally::default();
    let mut jalr_returns = 0usize;
    let mut jalr_jumps = 0usize;
    let mut ecalls = Tally::default();
    let mut undecoded: Vec<String> = Vec::new();
    let mut programs = 0usize;
    // Convention findings: a temporary read first after a call is a value
    // assumed to survive a call in a register doc 02 calls momentary.
    let mut temporaries_read_after_call: Vec<String> = Vec::new();

    // Labels.
    let mut labels = 0usize;
    let mut refs_text_backward = 0usize;
    let mut refs_text_forward = 0usize;
    let mut refs_to_data = 0usize;
    let mut refs_from_data = 0usize;
    let mut refs_external = 0usize;
    let mut indegree: BTreeMap<usize, usize> = BTreeMap::new();
    let mut unreferenced: Vec<String> = Vec::new();
    let mut unreferenced_nonglobal = 0usize;
    let mut max_in = (0usize, String::new());

    for (name, text) in t0_corpus() {
        let Ok(program) = assemble_program(&text) else {
            continue;
        };
        programs += 1;

        // ── calls ──
        if let Ok(acc) = accesses(&program) {
            for (i, a) in acc.iter().enumerate() {
                let line = program.instructions[i].line;
                if a.insn.is_empty() {
                    undecoded.push(format!(
                        "{name}:{line} {}",
                        program.instructions[i].family.name
                    ));
                    continue;
                }
                match a.insn.as_str() {
                    "jal" => {
                        if a.rd == Some(0) {
                            jal_jumps += 1;
                        } else {
                            let s = window(&acc, i);
                            for r in &s.read_first_after {
                                let abi = names.get(r).cloned().unwrap_or_default();
                                if declared_role(&abi) == "temporary" {
                                    temporaries_read_after_call
                                        .push(format!("{name}:{line} {abi}"));
                                }
                            }
                            jal_calls.add(&s);
                        }
                    }
                    "jalr" => {
                        if a.rd == Some(0) {
                            if a.rs1 == Some(1) {
                                jalr_returns += 1;
                            } else {
                                jalr_jumps += 1;
                            }
                        } else {
                            jalr_calls.add(&window(&acc, i));
                        }
                    }
                    "ecall" => ecalls.add(&window(&acc, i)),
                    _ => {}
                }
            }
        }

        // ── labels ──
        let by_name: BTreeMap<&str, &sadhana::parse::Label> = program
            .labels
            .iter()
            .map(|l| (l.name.as_str(), l))
            .collect();
        let mut in_deg: BTreeMap<&str, usize> = program
            .labels
            .iter()
            .map(|l| (l.name.as_str(), 0))
            .collect();
        for (i, inst) in program.instructions.iter().enumerate() {
            for o in &inst.operands {
                if o.is_numeral || register(&o.base).is_some() {
                    continue;
                }
                let base = o
                    .base
                    .strip_suffix("ॱउपरि")
                    .or_else(|| o.base.strip_suffix("ॱअधः"))
                    .unwrap_or(&o.base);
                match by_name.get(base) {
                    Some(l) => {
                        *in_deg.entry(base).or_default() += 1;
                        match l.section {
                            Section::Text => {
                                if l.at <= i {
                                    refs_text_backward += 1;
                                } else {
                                    refs_text_forward += 1;
                                }
                            }
                            _ => refs_to_data += 1,
                        }
                    }
                    None => {
                        if sadhana::encode::register(base).is_none() && !base.contains('ऽ') {
                            refs_external += 1;
                        }
                    }
                }
            }
        }
        for d in &program.data {
            for (_, l) in &d.addresses {
                match by_name.get(l.as_str()) {
                    Some(_) => {
                        *in_deg.entry(l.as_str()).or_default() += 1;
                        refs_from_data += 1;
                    }
                    None => refs_external += 1,
                }
            }
        }
        for l in &program.labels {
            labels += 1;
            let n = in_deg.get(l.name.as_str()).copied().unwrap_or(0);
            *indegree.entry(n).or_default() += 1;
            if n > max_in.0 {
                max_in = (n, format!("{name}:{} {}", l.line, l.name));
            }
            if n == 0 {
                let global = program.globals.contains(&l.name);
                if !global {
                    unreferenced_nonglobal += 1;
                }
                unreferenced.push(format!(
                    "{name}:{} {}{}",
                    l.line,
                    l.name,
                    if global { " (global)" } else { "" }
                ));
            }
        }
    }

    println!("METRIC paradigm_call_programs {programs}");
    println!("METRIC paradigm_call_window {WINDOW}");
    println!(
        "METRIC paradigm_call_undecoded_instructions {}",
        undecoded.len()
    );
    for u in &undecoded {
        println!("  UNDECODED {u}");
    }
    println!(
        "METRIC paradigm_call_jal_sites {}",
        jal_calls.sites + jal_jumps
    );
    println!("METRIC paradigm_call_jal_calls {}", jal_calls.sites);
    println!("METRIC paradigm_call_jal_jumps {jal_jumps}");
    println!(
        "METRIC paradigm_call_jalr_sites {}",
        jalr_calls.sites + jalr_returns + jalr_jumps
    );
    println!("METRIC paradigm_call_jalr_calls {}", jalr_calls.sites);
    println!("METRIC paradigm_call_jalr_returns {jalr_returns}");
    println!("METRIC paradigm_call_jalr_jumps {jalr_jumps}");
    println!("METRIC paradigm_call_ecall_sites {}", ecalls.sites);
    jal_calls.print("jal calls (rd != x0)", &names);
    jalr_calls.print("jalr calls (rd != x0)", &names);
    ecalls.print("ecall", &names);

    // Shares against doc 02 §2.4's declaration.
    let share = |t: &Tally, m: &BTreeMap<u32, usize>, role: &str| -> (usize, usize) {
        let total: usize = m.values().sum();
        let in_role: usize = m
            .iter()
            .filter(|(r, _)| names.get(r).is_some_and(|n| declared_role(n) == role))
            .map(|(_, c)| c)
            .sum();
        let _ = t;
        (in_role, total)
    };
    let (a_args, all_args) = share(&jal_calls, &jal_calls.arg, "args/return");
    let (a_ret, all_ret) = share(&jal_calls, &jal_calls.read_first, "args/return");
    let (s_ret, _) = share(&jal_calls, &jal_calls.read_first, "callee-saved");
    let (t_ret, _) = share(&jal_calls, &jal_calls.read_first, "temporary");
    println!(
        "METRIC paradigm_call_written_before_call_in_a_regs_pct {}",
        rate(a_args, all_args)
    );
    println!(
        "METRIC paradigm_call_read_first_after_call_in_a_regs_pct {}",
        rate(a_ret, all_ret)
    );
    println!(
        "METRIC paradigm_call_read_first_after_call_in_s_regs_pct {}",
        rate(s_ret, all_ret)
    );
    println!(
        "METRIC paradigm_call_read_first_after_call_in_t_regs_pct {}",
        rate(t_ret, all_ret)
    );
    println!(
        "METRIC paradigm_call_temporaries_read_first_after_call {}",
        temporaries_read_after_call.len()
    );
    for f in &temporaries_read_after_call {
        println!("  FINDING temporary read first after a call: {f}");
    }
    if let Some((r, c)) = ecalls.last_written.iter().max_by_key(|(_, c)| **c) {
        let abi = names.get(r).cloned().unwrap_or_default();
        println!(
            "  ecall number register by practice: {abi} ({c} of {} sites)",
            ecalls.sites
        );
        println!(
            "METRIC paradigm_call_ecall_number_in_a7_pct {}",
            rate(if abi == "a7" { *c } else { 0 }, ecalls.sites)
        );
    }

    println!("METRIC paradigm_label_count {labels}");
    println!("METRIC paradigm_label_refs_text_backward {refs_text_backward}");
    println!("METRIC paradigm_label_refs_text_forward {refs_text_forward}");
    println!("METRIC paradigm_label_refs_to_data {refs_to_data}");
    println!("METRIC paradigm_label_refs_from_data {refs_from_data}");
    println!("METRIC paradigm_label_refs_external {refs_external}");
    println!(
        "METRIC paradigm_label_backward_share_pct {}",
        rate(refs_text_backward, refs_text_backward + refs_text_forward)
    );
    println!("METRIC paradigm_label_unreferenced {}", unreferenced.len());
    println!("METRIC paradigm_label_unreferenced_nonglobal {unreferenced_nonglobal}");
    println!("METRIC paradigm_label_indegree_max {}", max_in.0);
    println!("  in-degree distribution (in-degree: labels):");
    for (d, n) in &indegree {
        println!("    {d:>4}: {n}");
    }
    println!("  most referenced: {} ({})", max_in.1, max_in.0);
    for u in &unreferenced {
        println!("  UNREFERENCED {u}");
    }
    assert!(programs > 0 && labels > 0, "the corpus was not read");
}

// ── tests of the instruments ───────────────────────────────────────────────

/// The program that runs on the metal round-trips whole, both targets.
#[test]
fn the_round_trip_passes_the_program_that_runs_on_the_metal() {
    let source = std::fs::read_to_string(root().join("spec/bare-metal.sas")).expect("read");
    let program = assemble_program(&source).expect("parses");
    for target in [Target::Uncompressed, Target::Compressed] {
        let r = round_trip("spec/bare-metal.sas", &program, target).expect("encodes");
        assert!(
            r.failures.is_empty(),
            "{target:?}: {} failure(s):\n  {}",
            r.failures.len(),
            r.failures.join("\n  ")
        );
        assert_eq!(
            r.checked,
            program.instructions.len(),
            "{target:?}: every instruction"
        );
    }
}

// ── the layout and the encoder agree about every address ───────────────────
//
// `W-241`. `encode.rs` holds THREE copies of the relaxation loop —
// `encode_program_for`, `encode_object_for` and `layout_addresses` — and
// `kosha` is handed the text from the second next to the addresses from the
// third. Two of the three cleared their symbol table at the TOP of each round,
// before the width pass, so no round ever saw a label: under `Compressed` every
// branch was laid out wide and the forward branch in `compressed.rs`'s
// six-instruction program was placed at `0 2 4 6 10 12 14` by the layout and
// emitted at `0 2 4 6 8 10` by the encoder. Found from the other language:
// `encode.t1`'s `स्थानविन्यासः` rebuilds the table each round and iterates to
// the fixpoint, and its pin in `t1_exec_encode.rs` named the Rust answer as the
// one that was wrong.
//
// The oracle here is the BYTES. `emitted_addresses` reads where each
// instruction landed out of the emitted text — the alignment requests are
// public on `Program` and `decode_at` gives each instruction's width — so it
// shares nothing with `layout_addresses`. The pre-`W-241` loop is kept below,
// in the test, as `layout_clearing_first`: the census prints what it answered
// so the number of programs the defect reached stays a measurement and not a
// memory.

/// The address each instruction was EMITTED at, read back from `text`; one more
/// entry for the end, as `layout_addresses` answers it.
///
/// # Errors
/// When nothing decodes where an instruction should be, or the walk does not
/// end where the text does — either means the walk and the text disagree, and
/// there is no address to compare.
fn emitted_addresses(program: &Program, text: &[u8]) -> Result<Vec<u32>, String> {
    let mut out = Vec::with_capacity(program.instructions.len() + 1);
    let mut pc = 0usize;
    for (i, inst) in program.instructions.iter().enumerate() {
        for (idx, n) in &program.text_aligns {
            if *idx == i {
                pc = pc.next_multiple_of(*n);
            }
        }
        out.push(u32::try_from(pc).expect("text is shorter than 4 GiB"));
        let Some((_, width)) = decode_at(text, pc) else {
            return Err(format!(
                "instruction {i} (line {}) should be at {pc:#x} and nothing decodes there",
                inst.line
            ));
        };
        pc += width;
    }
    if pc != text.len() {
        return Err(format!(
            "the walk ends at {pc:#x} and the text at {:#x}",
            text.len()
        ));
    }
    out.push(u32::try_from(pc).expect("fits"));
    Ok(out)
}

/// `layout_addresses` as it stood before `W-241`: the table cleared at the top
/// of every round, so the width pass never sees a label. Kept so the census can
/// say how many programs that reached.
fn layout_clearing_first(program: &Program, target: Target) -> Vec<u32> {
    let mut symbols: BTreeMap<String, u32> = BTreeMap::new();
    let mut addresses: Vec<u32> = Vec::new();
    let mut previous: Vec<u32> = Vec::new();
    let mut pc = 0u32;
    for _ in 0..8 {
        addresses.clear();
        symbols.clear();
        pc = 0;
        for (i, inst) in program.instructions.iter().enumerate() {
            for (idx, n) in &program.text_aligns {
                if *idx == i {
                    pc = pc.next_multiple_of(u32::try_from(*n).expect("fits"));
                }
            }
            addresses.push(pc);
            pc += match target {
                Target::Compressed if compressed_at(inst, pc, &symbols).is_some() => 2,
                _ => 4,
            };
        }
        for l in &program.labels {
            if l.section == Section::Text {
                symbols.insert(l.name.clone(), addresses.get(l.at).copied().unwrap_or(pc));
            }
        }
        if addresses == previous {
            break;
        }
        previous.clone_from(&addresses);
    }
    addresses.push(pc);
    addresses
}

/// One program's layout held against the bytes: where the two first part, and
/// how far apart they end up.
#[derive(Debug)]
struct Drift {
    /// First instruction whose address differs, and its line.
    index: usize,
    line: usize,
    laid_out: u32,
    emitted: u32,
    /// Instructions whose address differs.
    instructions: usize,
    /// Largest |laid out − emitted| over the program.
    max_shift: u32,
}

fn drift(laid_out: &[u32], emitted: &[u32], program: &Program) -> Option<Drift> {
    let mut first: Option<(usize, u32, u32)> = None;
    let mut instructions = 0usize;
    let mut max_shift = 0u32;
    for (i, (l, e)) in laid_out.iter().zip(emitted).enumerate() {
        if l != e {
            instructions += 1;
            max_shift = max_shift.max(l.abs_diff(*e));
            if first.is_none() {
                first = Some((i, *l, *e));
            }
        }
    }
    if laid_out.len() != emitted.len() {
        // A missing address is a disagreement about the end, not a match.
        instructions += 1;
        first.get_or_insert((laid_out.len().min(emitted.len()), 0, 0));
    }
    first.map(|(index, laid_out, emitted)| Drift {
        index,
        line: program.instructions.get(index).map_or(0, |i| i.line),
        laid_out,
        emitted,
        instructions,
        max_shift,
    })
}

/// Does `inst` carry an address half (`ॱउपरि`/`ॱअधः`) of a label that stands
/// outside `ॱपाठ`?
///
/// The one place the two encoders are ALLOWED to differ in width, and it is
/// stated here rather than assumed: `encode_object_for` leaves such a half to
/// the linker as a relocation and keeps the instruction wide, as `as` does;
/// `encode_program_for` resolves it and may choose a compressed form when the
/// low half happens to fit six bits. `layout_addresses` sees text labels only,
/// so it answers the object path — which is the path `kosha` pairs it with.
fn names_a_label_outside_text(inst: &sadhana::parse::Instruction, program: &Program) -> bool {
    inst.operands.iter().any(|o| {
        let label = o
            .base
            .strip_suffix("ॱउपरि")
            .or_else(|| o.base.strip_suffix("ॱअधः"));
        label.is_some_and(|l| {
            program
                .labels
                .iter()
                .any(|d| d.name == l && d.section != Section::Text)
        })
    })
}

/// REFUSED: a `layout_addresses` answer that differs from where the encoder put
/// any instruction of any spec program, at either target, fails by program name
/// and instruction index.
///
/// Two encoders are asked, because the corpus needs both: `encode_program_for`
/// resolves every label and refuses an undefined name, `encode_object_for`
/// leaves a foreign name to the linker and refuses nothing for it. A program
/// neither can encode at a target is counted and named — a refusal is not an
/// agreement, and the count is asserted so it cannot grow in silence.
///
/// Against the object path — the text `kosha` is handed next to this layout —
/// the agreement is total. Against the whole-program path a disagreement is
/// tolerated ONLY where the first instruction to differ names a label outside
/// `ॱपाठ` ([`names_a_label_outside_text`]); those are printed under their own
/// METRIC so the number stays visible, and anything else fails.
#[test]
fn the_layout_agrees_with_the_encoder_for_every_spec_program() {
    let corpus = t0_corpus();
    let mut programs: Vec<(String, Program)> = Vec::new();
    for (name, text) in &corpus {
        if let Ok(p) = assemble_program(text) {
            programs.push((name.clone(), p));
        }
    }
    assert_eq!(programs.len(), corpus.len(), "every spec program parses");
    println!("METRIC paradigm_layout_programs {}", programs.len());

    let mut checked = 0usize;
    let mut encoded: BTreeMap<&str, usize> = BTreeMap::new();
    let mut refused: Vec<String> = Vec::new();
    let mut before: Vec<String> = Vec::new();
    let mut before_max_shift = 0u32;
    let mut after: Vec<String> = Vec::new();
    let mut after_max_shift = 0u32;
    let mut scope: Vec<String> = Vec::new();
    for (name, p) in &programs {
        for target in [Target::Uncompressed, Target::Compressed] {
            let mut oracles: Vec<(&str, Vec<u8>)> = Vec::new();
            if let Ok(text) = encode_program_for(p, target) {
                oracles.push(("encode_program_for", text));
            }
            if let Ok((text, _)) = encode_object_for(p, target) {
                oracles.push(("encode_object_for", text));
            }
            if oracles.is_empty() {
                refused.push(format!("{name} ({target:?})"));
                continue;
            }
            let laid_out = layout_addresses(p, target);
            let old = layout_clearing_first(p, target);
            for (which, text) in &oracles {
                let emitted = emitted_addresses(p, text)
                    .unwrap_or_else(|e| panic!("{name} ({target:?}, {which}): {e}"));
                checked += 1;
                *encoded.entry(which).or_default() += 1;
                if let Some(d) = drift(&old, &emitted, p) {
                    before_max_shift = before_max_shift.max(d.max_shift);
                    before.push(format!(
                        "{name} ({target:?}, {which}): instruction {} (line {}) laid out at \
                         {:#x}, emitted at {:#x}; {} instruction(s) differ, max shift {}",
                        d.index, d.line, d.laid_out, d.emitted, d.instructions, d.max_shift
                    ));
                }
                if let Some(d) = drift(&laid_out, &emitted, p) {
                    // The first address to differ belongs to the instruction
                    // AFTER the one whose width the two disagreed about.
                    let culprit = d.index.checked_sub(1).and_then(|i| p.instructions.get(i));
                    let shown = format!(
                        "{name} ({target:?}, {which}): instruction {} (line {}) laid out at \
                         {:#x}, emitted at {:#x}; {} instruction(s) differ, max shift {}; \
                         the width that differs is {}",
                        d.index,
                        d.line,
                        d.laid_out,
                        d.emitted,
                        d.instructions,
                        d.max_shift,
                        culprit.map_or("at the start".to_string(), |c| format!(
                            "`{}` at line {}",
                            c.family.name, c.line
                        ))
                    );
                    let outside_text = *which == "encode_program_for"
                        && culprit.is_some_and(|inst| names_a_label_outside_text(inst, p));
                    if outside_text {
                        scope.push(shown);
                    } else {
                        after_max_shift = after_max_shift.max(d.max_shift);
                        after.push(shown);
                    }
                }
            }
        }
    }
    println!("METRIC paradigm_layout_comparisons {checked}");
    for (which, n) in &encoded {
        println!("METRIC paradigm_layout_encoded_by_{which} {n}");
    }
    println!("METRIC paradigm_layout_refused {}", refused.len());
    for r in &refused {
        println!("  refused by both encoders: {r}");
    }
    println!(
        "METRIC paradigm_layout_disagreements_before {}",
        before.len()
    );
    println!("METRIC paradigm_layout_max_shift_before {before_max_shift}");
    for b in &before {
        println!("  BEFORE {b}");
    }
    println!("METRIC paradigm_layout_disagreements_after {}", after.len());
    println!("METRIC paradigm_layout_max_shift_after {after_max_shift}");
    for a in &after {
        println!("  DISAGREES {a}");
    }
    println!("METRIC paradigm_layout_scope_differences {}", scope.len());
    for s in &scope {
        println!("  OUTSIDE-TEXT {s}");
    }

    assert!(checked > 0, "the corpus was not read");
    // The instrument must be able to see the class it was built for: the loop
    // that cleared first is held to the same bytes and must be caught.
    assert!(
        !before.is_empty(),
        "the pre-W-241 loop agrees with the encoder everywhere — either the \
         corpus lost every compressible forward branch or this instrument is blind"
    );
    assert!(
        after.is_empty(),
        "{} layout/encoder disagreement(s):\n  {}",
        after.len(),
        after.join("\n  ")
    );
    // Refusals are named above and there are none: every spec program encodes
    // at both targets under at least one encoder, so a program that stops
    // encoding cannot leave this census quietly greener.
    assert!(
        refused.is_empty(),
        "{} program/target pair(s) refused by both encoders:\n  {}",
        refused.len(),
        refused.join("\n  ")
    );
}

/// The three lines W-211 listed as compressed exceptions — `spec/atithi.sas:38`
/// (`c.ld`), `spec/capability.sas:32` (`c.sd`) and `spec/crossing.sas:67`
/// (`c.srli`) — round-trip whole at the compressed target. Every register they
/// name is one of `x8`..`x15`, the range a three-bit compressed field holds
/// biased by 8; before `W-233` the decoder read the field without the bias and
/// the register check refused all three (`s0` decoded as `x0`).
#[test]
fn the_compressed_round_trip_sees_the_biased_registers() {
    let source = "आहारःॱअ६४ अर्थ०म् अर्थ१त् ०न ।\n\
                  निधानम्ॱअ६४ स्थिर०य् ०न स्थिर०न ।\n\
                  दक्षिणसरणम् स्थिर०म् स्थिर०न १२न ।\n";
    let program = assemble_program(source).expect("parses");
    // Three instructions and the end offset after them.
    let addresses = layout_addresses(&program, Target::Compressed);
    assert_eq!(
        addresses,
        vec![0, 2, 4, 6],
        "all three lines take their compressed form"
    );
    let r = round_trip("three", &program, Target::Compressed).expect("encodes");
    assert!(
        r.failures.is_empty(),
        "{} failure(s):\n  {}",
        r.failures.len(),
        r.failures.join("\n  ")
    );
    assert_eq!(r.checked, 3);
}

/// REFUSED: an image with one word corrupted to all ones is reported at that
/// instruction's line, and nowhere else.
#[test]
fn a_corrupted_encoding_is_reported_as_a_round_trip_failure() {
    let source = std::fs::read_to_string(root().join("spec/bare-metal.sas")).expect("read");
    let program = assemble_program(&source).expect("parses");
    let (mut text, _) = encode_object_for(&program, Target::Uncompressed).expect("encodes");
    let addresses = layout_addresses(&program, Target::Uncompressed);
    let victim = 3usize;
    let off = addresses[victim] as usize;
    text[off..off + 4].copy_from_slice(&[0xff; 4]);

    let r = round_trip_bytes("bare-metal", &program, &text, &addresses);
    assert_eq!(
        r.failures.len(),
        1,
        "exactly the corrupted word: {:?}",
        r.failures
    );
    assert!(
        r.failures[0].starts_with(&format!("bare-metal:{}", program.instructions[victim].line)),
        "named by file:line: {}",
        r.failures[0]
    );
    assert!(
        r.failures[0].contains("did not decode"),
        "{}",
        r.failures[0]
    );
    assert_eq!(r.checked, program.instructions.len() - 1);
}

/// REFUSED: a corruption that still decodes — the opcode of an `add` turned
/// into a `sub` by setting bit 30 — is reported as a family disagreement.
#[test]
fn a_corruption_that_still_decodes_but_changes_the_instruction_is_reported() {
    let program = assemble_program("योगः क्षणिक०म् क्षणिक१न क्षणिक२न ।\n").expect("parses");
    let (mut text, _) = encode_object_for(&program, Target::Uncompressed).expect("encodes");
    let addresses = layout_addresses(&program, Target::Uncompressed);
    let word = u32::from_le_bytes([text[0], text[1], text[2], text[3]]) | 1 << 30;
    text[..4].copy_from_slice(&word.to_le_bytes());
    let r = round_trip_bytes("one", &program, &text, &addresses);
    assert_eq!(r.checked, 0);
    assert_eq!(r.failures.len(), 1);
    assert!(
        r.failures[0].contains("decoded as sub") && r.failures[0].contains("written as family add"),
        "{}",
        r.failures[0]
    );
}

/// REFUSED: a register corrupted in place — bytes that decode and reassemble
/// perfectly — is caught by the operand check, which is why that check exists.
#[test]
fn a_register_swapped_in_the_bytes_is_reported() {
    let program = assemble_program("योगः क्षणिक०म् क्षणिक१न क्षणिक२न ।\n").expect("parses");
    let (mut text, _) = encode_object_for(&program, Target::Uncompressed).expect("encodes");
    let addresses = layout_addresses(&program, Target::Uncompressed);
    // rd: t0 = x5 -> x9, still a valid `add`.
    let word = u32::from_le_bytes([text[0], text[1], text[2], text[3]]);
    let word = (word & !RD) | (9 << 7);
    text[..4].copy_from_slice(&word.to_le_bytes());
    let r = round_trip_bytes("one", &program, &text, &addresses);
    assert_eq!(r.failures.len(), 1, "{:?}", r.failures);
    assert!(
        r.failures[0].contains("written register(s) क्षणिक०"),
        "{}",
        r.failures[0]
    );
}

/// The twin tables are found in the file that holds them, and they name the
/// pairs the row expects.
#[test]
fn the_twin_tables_are_read_from_t1_sources() {
    let text = std::fs::read_to_string(root().join("crates/sadhana-t1/tests/t1_sources.rs"))
        .expect("t1_sources.rs exists");
    let twins = twin_tables_in(&text).expect("tables");
    let encode = twins
        .iter()
        .find(|t| t.rust == "encode" && t.table == "ENCODER_SYMBOLS")
        .expect("ENCODER_SYMBOLS pairs encode");
    assert_eq!(encode.t1, "सङ्केतनम्");
    assert!(
        twins
            .iter()
            .any(|t| t.rust == "decode" && t.t1 == "विश्लेषणम्")
    );
    assert!(
        !twins.iter().any(|t| t.table.ends_with("_FIELDS")),
        "field tables are not routines"
    );
    assert!(twins.len() >= 100, "only {} pairs", twins.len());
    assert!(is_routine("encode") && is_routine("Slot::place") && is_routine("lines"));
    assert!(!is_routine("struct Slot") && !is_routine("RD") && !is_routine("Ty::Int"));
    assert!(!is_routine("ValueId(n)") && !is_routine("Block::insts.push"));
}

/// REFUSED: text with no twin table is refused, not reported as 0 of 0.
#[test]
fn a_file_without_twin_tables_is_refused() {
    let err = twin_tables_in("const NOT_A_TABLE: &[&str] = &[\"a\"];\n").expect_err("refused");
    assert!(err.contains("no"), "{err}");
}

const SYNTHETIC: &str = r#"
use sadhana::t1::nirvahana::{Interpreter, Value};

fn helper(it: &mut Interpreter) -> i128 {
    it.call("सङ्केतनॱस्थानसङ्केतनम्", vec![], 1).unwrap().as_int().unwrap()
}

#[test]
fn runs_both() {
    let mut it = load();
    let got = it
        .call(
            "सङ्केतनॱसङ्केतनम्",
            vec![],
            1,
        )
        .unwrap();
    assert_eq!(got, sadhana::encode::encode(&inst).unwrap());
}

#[test]
fn runs_only_the_prefix_cousin() {
    let mut it = load();
    let got = helper(&mut it);
    assert_eq!(got, sadhana::encode::encode(&inst).unwrap());
}

#[test]
fn runs_only_t1() {
    let got = it.call("सङ्केतनॱसङ्केतनम्", vec![], 1).unwrap();
    assert!(got.is_nil());
}
"#;

/// The scanner recognises a test that runs both twins and asserts, and does
/// not count one that runs only the T1 side, nor one whose T1 call names a
/// LONGER routine that merely ends in the twin's name.
#[test]
fn a_test_that_runs_both_twins_is_recognised_and_its_cousins_are_not() {
    let twin = Twin {
        table: "ENCODER_SYMBOLS".into(),
        rust: "encode".into(),
        t1: "सङ्केतनम्".into(),
    };
    assert_eq!(
        tests_running_both(SYNTHETIC, &twin),
        vec!["runs_both".to_string()]
    );
    let at = Twin {
        table: "ENCODER_SYMBOLS".into(),
        rust: "encode_at".into(),
        t1: "स्थानसङ्केतनम्".into(),
    };
    // The helper calls स्थानसङ्केतनम्, the test calls the helper — but the
    // Rust side is `encode`, not `encode_at`, so it is not an agreement test.
    assert!(tests_running_both(SYNTHETIC, &at).is_empty());
}

/// The two agreement tests the tree has today are found where they are.
#[test]
fn the_encoder_agreement_tests_in_the_tree_are_found() {
    let text = std::fs::read_to_string(root().join("crates/sadhana-t1/tests/t1_exec_encode.rs"))
        .expect("t1_exec_encode.rs exists");
    let twin = Twin {
        table: "ENCODER_SYMBOLS".into(),
        rust: "encode".into(),
        t1: "सङ्केतनम्".into(),
    };
    let found = tests_running_both(&text, &twin);
    assert!(
        found
            .contains(&"the_bare_encoder_is_the_addressed_one_at_zero_with_no_symbols".to_string()),
        "{found:?}"
    );
}

/// The window reads writes and reads off the decoded fields: a call preceded
/// by a write to a0 and followed by a read of a0 sees a0 on both sides.
#[test]
fn the_call_window_sees_what_is_written_before_and_read_after() {
    let src = "योगः अर्थ०म् शून्यःन ७न ।\nलङ्घनम् पुनःस्थानम्म् लक्ष्यम्य् ।\nयोगः क्षणिक०म् अर्थ०न ०न ।\nलक्ष्यम्ॱॱ\nसापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n";
    let program = assemble_program(src).expect("parses");
    let acc = accesses(&program).expect("encodes");
    assert_eq!(acc[1].insn, "jal");
    assert_eq!(acc[1].rd, Some(1), "ra is written by the call");
    let s = window(&acc, 1);
    assert!(s.written_before.contains(&10), "a0 written before: {s:?}");
    assert!(
        s.read_first_after.contains(&10),
        "a0 read first after: {s:?}"
    );
    assert!(
        s.written_first_after.contains(&5),
        "t0 written first after: {s:?}"
    );
    assert_eq!(acc[3].insn, "jalr");
    assert_eq!(
        (acc[3].rd, acc[3].rs1),
        (Some(0), Some(1)),
        "ret is jalr x0, ra"
    );
}

/// Every instruction of the program that runs on the metal yields an access
/// record — the call census cannot be silently skipping instructions.
#[test]
fn every_instruction_of_bare_metal_has_an_access() {
    let source = std::fs::read_to_string(root().join("spec/bare-metal.sas")).expect("read");
    let program = assemble_program(&source).expect("parses");
    let acc = accesses(&program).expect("encodes");
    let missing: Vec<String> = acc
        .iter()
        .zip(&program.instructions)
        .filter(|(a, _)| a.insn.is_empty())
        .map(|(_, i)| format!("line {} {}", i.line, i.family.name))
        .collect();
    assert!(missing.is_empty(), "no access record for: {missing:?}");
}

/// The unparser search found nothing until `W-215`; it finds two now, the
/// Rust `unparse` and the T1 `स्रोतलेखनम्`, and statistic 29 is measured by
/// `measure_corpus_parse_print` in `crates/sadhana-t1/tests/t1_execution.rs`
/// (`METRIC paradigm_halves_parse_print_rate`). This test is the notice's
/// inverse: the day the search finds nothing again, the unparser has been
/// renamed out of the census's sight or deleted, and either is worth a
/// failure.
#[test]
fn t1_has_an_unparser_since_w215() {
    let found = unparser_candidates();
    assert!(
        found.iter().any(|c| c.contains("स्रोतलेखनम्")),
        "the T1 unparser `स्रोतलेखनम्` is not found by the census: {found:?}"
    );
    assert!(
        found.iter().any(|c| c.contains("fn unparse")),
        "the Rust unparser `unparse` is not found by the census: {found:?}"
    );
}
