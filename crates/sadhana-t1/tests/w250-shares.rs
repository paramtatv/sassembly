//! W-250, STEP 2: HOW MANY `भवति` SITES IN THE CORPUS SHARE?
//!
//! `w250-aliasing.rs` established, by running programs, that `भवति` between
//! two arena- or record-typed names puts both names on ONE value, and that
//! `अङ्कः अन्तः अ८` and the scalars copy. That is a fact about the language.
//! This file asks the only question that says whether it MATTERS: how often
//! does the corpus actually write one?
//!
//! # Why this is not a grep
//!
//! `भवति` occurs 4961 times. Almost all of them are `भवति ०` — a FRESH arena
//! from the declared type, not a share — or a numeral, or an operator
//! expression. A share needs the right-hand side to be a bare NAME whose type
//! is an arena or a record. So the site has to be classified by TYPE, and the
//! type has to come from the declaration, which means reading declarations.
//!
//! Two things make that tractable and honest here:
//!
//! - the corpus is lexed with `sadhana::lex::lex_t1`, the assembler's OWN
//!   lexer, so comments and `उक्तम् … इति` literals are already tokens and no
//!   hand-written stripper can get them wrong. `इतिशब्दः` — the global whose
//!   value is the word `इति` — is the case that breaks a text scanner, and it
//!   arrives here as a single `Kind::Str`.
//! - a type is always the tokens between `ॱॱ` and the next `ऽ`, `भवति`,
//!   `ददाति` or `समाप्तम्`. That one rule covers all three binding forms —
//!   `चरः`, a parameter, a `संरचना` field — so there is no separate parser per
//!   form to get separately wrong.
//!
//! # What it refuses
//!
//! A name it cannot type is NOT counted as a copy. It is counted as UNKNOWN
//! and printed, because a census that silently absorbed what it could not
//! place would report a small share count for the wrong reason. The unknown
//! count is asserted to be what it is, so it cannot drift upward unnoticed.
//!
//! THE EIGHT IT STILL REFUSES ARE NOT ITS OWN BUGS, and that had to be shown
//! rather than assumed — the first three residues WERE its bugs, and each one
//! moved the share count:
//!
//! - the `ऽ` between parameters is optional (`ashtaka.t1:103`), which hid ten
//!   parameters (`read_type`'s margin);
//! - a qualified type is ONE token (`वास्तुॱअभिव्यञ्जक`), which hid three
//!   shares in `parse.t1` and moved `paradigm_runs_assigned` from 339 to 612
//!   (`named`'s margin);
//! - a zero-argument call is spelled like a bare name (`Scope::routines`).
//!
//! What is left is eight sites whose declared type NO DECLARATION IN THE
//! CORPUS DEFINES: `इ६४` (46 uses) and `पाठ` (42 uses) are written as types
//! and never declared as one. Those cannot be a run or a record — `zero_at`
//! sends an unknown `Ty::Named` to `Value::Int(0)` — so the share count is
//! complete even though these eight are unplaced. That they exist at all is a
//! finding for another row and is reported with this one.

use sadhana::lex::{Kind, Token, lex_t1};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// THE OWNER'S RULING, 2026-09-13: token-count pins are report-only. They print a
// METRIC line and a NOTE when the count moves; they never red a landing. The
// pinned literal stays as the last recorded value, so the NOTE names the delta.
macro_rules! pin_report {
    ($left:expr, $right:expr $(, $($arg:tt)*)?) => {{
        let l = $left;
        let r = $right;
        println!("METRIC pin {} {:?}", stringify!($left), l);
        if l != r {
            eprintln!(
                "NOTE pin moved (report-only): {} measured {:?}, last recorded {:?}",
                stringify!($left),
                l,
                r
            );
        }
    }};
}

// ── the words this census reads ──────────────────────────────────────────
const W_BECOMES: &str = "भवति";
const W_VAR: &str = "चरः";
const W_FN: &str = "वृत्तिः";
const W_STRUCT: &str = "संरचना";
const W_TAKES: &str = "आदाय";
const W_GIVES: &str = "ददाति";
const W_BLOCK_OPEN: &str = "आदि";
const W_BLOCK_CLOSE: &str = "इति";
const W_GROUP_CLOSE: &str = "समाप्तम्";
const W_INDEX_OPEN: &str = "अङ्कः";
const W_INDEX_CLOSE: &str = "अन्तः";
const W_U8: &str = "अ८";
const W_POINTER: &str = "स्थानम्";
const W_OPTIONAL: &str = "सम्भाव्य";
const W_ERRUNION: &str = "दोषयुक्त";

fn source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn corpus() -> Vec<(String, String)> {
    let mut names: Vec<String> = std::fs::read_dir(source_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| {
            let p = source_dir().join(&n);
            let t = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
            (n, t)
        })
        .collect()
}

/// What `भवति` MEANS for a value of this type — the answer `w250-aliasing.rs`
/// got by running a program for each of the four shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    /// `अङ्कः अन्तः T` where `T` is not `अ८` — `Value::Arena`. SHARES.
    Arena,
    /// A `संरचना` name — `Value::Record`. SHARES.
    Record,
    /// `अङ्कः अन्तः अ८` — `Value::Octets`. Copies, and this is the asymmetry.
    Octets,
    /// `अ६४`, `बूल`, an enum tag, a pointer. Copies.
    Scalar,
    /// The census could not place it. Never counted as a copy.
    Unknown,
}

impl Shape {
    fn shares(self) -> bool {
        matches!(self, Shape::Arena | Shape::Record)
    }
}

/// The type-token slice → shape. `structs` is every `संरचना` name in the whole
/// corpus, because a type may name a record another module declares.
fn shape_of(ty: &[String], structs: &BTreeSet<String>) -> Shape {
    match ty {
        [] => Shape::Unknown,
        // `स्थानम् T` / `सम्भाव्य T` / `दोषयुक्त T` — the wrapper decides
        // nothing about sharing; look through it. A pointer is a scalar in
        // this interpreter (there is no `Value::Pointer`), so it is only the
        // optional and the error union that pass the shape through.
        [w, rest @ ..] if w == W_OPTIONAL || w == W_ERRUNION => shape_of(rest, structs),
        [w, ..] if w == W_POINTER => Shape::Scalar,
        // `अङ्कः अन्तः T` and `अङ्कः <numeral> अन्तः T`.
        [open, ..] if open == W_INDEX_OPEN => {
            let Some(close) = ty.iter().position(|t| t == W_INDEX_CLOSE) else {
                return Shape::Unknown;
            };
            match shape_of(&ty[close + 1..], structs) {
                // A run of `अ८` is the one run that is not an arena.
                Shape::Scalar if ty.get(close + 1).is_some_and(|t| t == W_U8) => Shape::Octets,
                Shape::Unknown => Shape::Unknown,
                _ => Shape::Arena,
            }
        }
        [only] => named(only, structs),
        // More than one token and none of the wrappers above: take the last.
        _ => named(ty.last().expect("non-empty"), structs),
    }
}

/// One type NAME → its shape.
///
/// A QUALIFIED TYPE IS ONE TOKEN. The corpus writes `मण्डलॱनाम` with no spaces
/// around the `ॱ` — member access is what gets the spaces — so the lexer hands
/// back `वास्तुॱअभिव्यञ्जक` whole, and a `संरचना` table keyed by the bare name
/// `अभिव्यञ्जक` does not contain it. The first version of this census failed
/// exactly there and called `parse.t1:304` and `:372` — two of the corpus's own
/// arena appenders — unplaceable rather than SHARING. Strip the qualifier.
fn named(n: &str, structs: &BTreeSet<String>) -> Shape {
    let bare = n.rsplit('\u{971}').next().unwrap_or(n);
    if structs.contains(n) || structs.contains(bare) {
        Shape::Record
    } else if is_primitive(n) || is_primitive(bare) {
        Shape::Scalar
    } else {
        Shape::Unknown
    }
}

/// The sized types and `बूल`. `अङ्कः`-free, single word.
fn is_primitive(w: &str) -> bool {
    matches!(
        w,
        "अ८" | "अ१६"
            | "अ३२"
            | "अ६४"
            | "अ१२८"
            | "न८"
            | "न१६"
            | "न३२"
            | "न६४"
            | "न१२८"
            | "बूल"
    )
}

/// A `भवति` site, classified.
#[derive(Clone, Debug)]
struct Site {
    file: String,
    line: usize,
    /// The name the value is bound INTO, as written (`क`, `क ॱ मान`, `क अङ्कः …`).
    target: String,
    /// The base name of the target — what a later write would be looked up by.
    target_base: String,
    /// `Some(name)` when the right-hand side is one bare name and nothing else.
    /// That is the only shape a share can have.
    rhs_name: Option<String>,
    /// The shape of the value the right-hand side produces.
    rhs_shape: Shape,
    /// The shape of the target's declared type, when the target is a plain name.
    target_shape: Shape,
    routine: String,
    rhs_is_global: bool,
    target_is_global: bool,
    /// The right-hand side is a zero-argument CALL, not a name. Spelled the
    /// same; means something else entirely.
    rhs_is_call: bool,
    /// Led by `चरः` — a BINDING in `spec/grammar-t1.ebnf`'s terms, rather than
    /// an assignment.
    is_binding: bool,
}

/// Everything one file declares, keyed for lookup.
#[derive(Default, Clone)]
struct Scope {
    globals: BTreeMap<String, Vec<String>>,
    /// routine name → (param or local name → type tokens)
    locals: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// struct name → field name → type tokens
    fields: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// routine name → (takes no parameters, return type tokens).
    ///
    /// A ZERO-ARGUMENT CALL IS SPELLED EXACTLY LIKE A BARE NAME. `artha.t1`'s
    /// `दोषार्थः` is `सार्वजनिक वृत्तिः दोषार्थः ददाति अर्थप्रकार`, so
    /// `चरः फलम् ॱॱ अर्थप्रकार भवति दोषार्थः ।` is a CALL returning a fresh
    /// record and `... भवति नव ।` two lines away is a SHARE of a local — and
    /// the two lines are the same shape. Without this table the census would
    /// have called the call an unplaceable name; with it, the two populations
    /// are counted apart.
    routines: BTreeMap<String, (bool, Vec<String>)>,
    /// This file's `मण्डलम्` name.
    module: String,
}

fn word(t: &Token) -> &str {
    t.text.as_str()
}

/// A type is the tokens from just after `ॱॱ` up to the next delimiter. One
/// rule for `चरः`, for a parameter and for a `संरचना` field.
///
/// THE `ऽ` IS OPTIONAL BETWEEN PARAMETERS and the first version of this
/// census did not know it. `ashtaka.t1:103` writes
/// `आदाय पाठ ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ ददाति न६४` with no
/// separator at all, so a reader that only stopped at `ऽ` swallowed the rest
/// of the parameter list into the FIRST parameter's type and registered
/// neither `आरम्भः` nor `सीमा`. Ten sites then reported an unplaceable name.
/// The stop that fixes it is structural rather than a special case: a type
/// ENDS at the token before the next `ॱॱ`, because that token is the next
/// name being bound.
fn read_type(toks: &[Token], mut i: usize) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    while i < toks.len() {
        let w = word(&toks[i]);
        if matches!(toks[i].kind, Kind::Danda | Kind::Separator)
            || w == W_BECOMES
            || w == W_GIVES
            || w == W_GROUP_CLOSE
            || w == W_BLOCK_OPEN
        {
            break;
        }
        // the next parameter's name, not part of this type
        if toks.get(i + 1).is_some_and(|t| t.kind == Kind::LabelMark) {
            break;
        }
        out.push(w.to_string());
        i += 1;
    }
    (out, i)
}

/// Collect declarations. One pass, no parser: every binding form in this
/// language puts `ॱॱ` between the name and the type.
fn collect(toks: &[Token]) -> Scope {
    let mut s = Scope::default();
    let mut routine = String::new();
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < toks.len() {
        let w = word(&toks[i]);
        match w {
            W_BLOCK_OPEN => depth += 1,
            W_BLOCK_CLOSE if matches!(toks[i].kind, Kind::Word) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    routine.clear();
                }
            }
            W_STRUCT => {
                let name = word(&toks[i + 1]).to_string();
                let mut j = i + 2;
                let mut fields = BTreeMap::new();
                while j < toks.len() && word(&toks[j]) != W_GROUP_CLOSE {
                    if toks[j].kind == Kind::LabelMark {
                        let f = word(&toks[j - 1]).to_string();
                        let (ty, next) = read_type(toks, j + 1);
                        fields.insert(f, ty);
                        j = next;
                    } else {
                        j += 1;
                    }
                }
                s.fields.insert(name, fields);
                i = j;
            }
            "मण्डलम्" => {
                if s.module.is_empty() {
                    s.module = word(&toks[i + 1]).to_string();
                }
            }
            W_FN => {
                routine = word(&toks[i + 1]).to_string();
                s.locals.entry(routine.clone()).or_default();
                {
                    // the signature: does it take parameters, and what does it give?
                    let zero_arg = word(&toks[i + 2]) != W_TAKES;
                    let ret = toks[i + 2..]
                        .iter()
                        .position(|t| word(t) == W_GIVES)
                        .map(|p| {
                            let g = i + 2 + p + 1;
                            toks[g..]
                                .iter()
                                .take_while(|t| word(t) != W_BLOCK_OPEN)
                                .map(|t| t.text.clone())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    s.routines.insert(routine.clone(), (zero_arg, ret));
                }
                // parameters, if any, up to `ददाति` or the body's `आदि`
                let mut j = i + 2;
                if word(&toks[j]) == W_TAKES {
                    while j < toks.len()
                        && word(&toks[j]) != W_GIVES
                        && word(&toks[j]) != W_BLOCK_OPEN
                    {
                        if toks[j].kind == Kind::LabelMark {
                            let p = word(&toks[j - 1]).to_string();
                            let (ty, next) = read_type(toks, j + 1);
                            s.locals
                                .get_mut(&routine)
                                .expect("just inserted")
                                .insert(p, ty);
                            j = next;
                        } else {
                            j += 1;
                        }
                    }
                }
                i = j.saturating_sub(1);
            }
            W_VAR => {
                let name = word(&toks[i + 1]).to_string();
                // `चरः NAME ॱॱ TYPE`
                let mut j = i + 2;
                while j < toks.len()
                    && toks[j].kind != Kind::LabelMark
                    && word(&toks[j]) != W_BECOMES
                {
                    j += 1;
                }
                let ty = if toks.get(j).is_some_and(|t| t.kind == Kind::LabelMark) {
                    read_type(toks, j + 1).0
                } else {
                    Vec::new()
                };
                if depth == 0 || routine.is_empty() {
                    s.globals.insert(name, ty);
                } else {
                    s.locals
                        .entry(routine.clone())
                        .or_default()
                        .insert(name, ty);
                }
            }
            _ => {}
        }
        i += 1;
    }
    s
}

/// Split into statements on `।`, tracking the enclosing routine.
fn sites(
    file: &str,
    toks: &[Token],
    s: &Scope,
    structs: &BTreeSet<String>,
    by_module: &BTreeMap<String, Scope>,
) -> Vec<Site> {
    let mut out = Vec::new();
    let mut routine = String::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for i in 0..toks.len() {
        let w = word(&toks[i]);
        if w == W_FN {
            routine = word(&toks[i + 1]).to_string();
        } else if w == W_BLOCK_OPEN {
            depth += 1;
        } else if w == W_BLOCK_CLOSE && toks[i].kind == Kind::Word {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                routine.clear();
            }
        }
        // A statement ends at `।`, and `आदि`/`इति` also end one.
        let ends = toks[i].kind == Kind::Danda || w == W_BLOCK_OPEN || w == W_BLOCK_CLOSE;
        if !ends {
            continue;
        }
        let stmt = &toks[start..i];
        start = i + 1;
        let Some(b) = stmt.iter().position(|t| word(t) == W_BECOMES) else {
            continue;
        };
        let lhs = &stmt[..b];
        let rhs = &stmt[b + 1..];
        out.push(classify(file, &routine, lhs, rhs, s, structs, by_module));
    }
    out
}

/// The type tokens a name resolves to, in this routine then at module level.
fn lookup<'a>(s: &'a Scope, routine: &str, name: &str) -> Option<(&'a Vec<String>, bool)> {
    if let Some(t) = s.locals.get(routine).and_then(|m| m.get(name)) {
        return Some((t, false));
    }
    s.globals.get(name).map(|t| (t, true))
}

fn classify(
    file: &str,
    routine: &str,
    lhs: &[Token],
    rhs: &[Token],
    s: &Scope,
    structs: &BTreeSet<String>,
    by_module: &BTreeMap<String, Scope>,
) -> Site {
    // The target: strip a leading `चरः`, then the base name is the first word.
    let is_binding = lhs.first().is_some_and(|t| word(t) == W_VAR);
    let lhs_body: &[Token] = if lhs.first().is_some_and(|t| word(t) == W_VAR) {
        &lhs[1..]
    } else {
        lhs
    };
    let target_base = lhs_body
        .first()
        .map(|t| word(t).to_string())
        .unwrap_or_default();
    let target: String = lhs_body
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    // The target's shape. A `चरः` says its type inline; a bare name is looked
    // up; `a ॱ f` takes the field's type; `a अङ्कः i अन्तः` takes the element.
    let (target_shape, target_is_global) = target_shape_of(lhs, lhs_body, routine, s, structs);

    // The right-hand side SHARES only if it is exactly one bare name. A
    // numeral, a `०`, an operator expression, a call, a group, a slice, an
    // index and a field read all build or copy a value.
    let mut rhs_name = None;
    let mut rhs_shape = Shape::Scalar;
    let mut rhs_is_global = false;
    let mut rhs_is_call = false;
    if rhs.len() == 1 && rhs[0].kind == Kind::Word {
        let n = word(&rhs[0]).to_string();
        rhs_name = Some(n.clone());
        if let Some((ty, is_global)) = lookup(s, routine, &n) {
            rhs_shape = shape_of(ty, structs);
            rhs_is_global = is_global;
        } else if let Some((zero_arg, ret)) = s.routines.get(&n) {
            // A ZERO-ARGUMENT CALL. `भवति दोषार्थः` and `भवति नव` are the
            // same shape at the site and are not the same thing.
            rhs_is_call = *zero_arg;
            rhs_shape = shape_of(ret, structs);
        } else if let Some((ty, is_call)) = qualified(&n, structs, by_module) {
            rhs_shape = ty;
            rhs_is_call = is_call;
            rhs_is_global = !is_call;
        } else {
            // A bare word that is neither a declared name nor a routine:
            // `सत्यम्`/`असत्यम्`, or a name this census cannot reach. The
            // census does not get to assume it copies, so it says UNKNOWN.
            rhs_shape = Shape::Unknown;
        }
    } else if rhs.len() == 1 && rhs[0].kind == Kind::Numeral {
        rhs_shape = Shape::Scalar;
    } else if rhs.is_empty() {
        rhs_shape = Shape::Unknown;
    }

    Site {
        file: file.to_string(),
        line: lhs.first().map_or(0, |t| t.line),
        target,
        target_base,
        rhs_name,
        rhs_shape,
        target_shape,
        routine: routine.to_string(),
        rhs_is_global,
        target_is_global,
        rhs_is_call,
        is_binding,
    }
}

/// `मण्डलॱनाम` — one token, because the corpus writes a qualified name with no
/// spaces around the `ॱ` while member access is written with them. Resolve it
/// against the module that declares it: a cross-module global that is an arena
/// SHARES exactly as a local one does, and a per-file table would have called
/// every one of them unplaceable.
fn qualified(
    n: &str,
    structs: &BTreeSet<String>,
    by_module: &BTreeMap<String, Scope>,
) -> Option<(Shape, bool)> {
    let (m, rest) = n.split_once('\u{971}')?;
    let scope = by_module.get(m)?;
    if let Some(ty) = scope.globals.get(rest) {
        return Some((shape_of(ty, structs), false));
    }
    let (zero_arg, ret) = scope.routines.get(rest)?;
    Some((shape_of(ret, structs), *zero_arg))
}

fn target_shape_of(
    lhs: &[Token],
    body: &[Token],
    routine: &str,
    s: &Scope,
    structs: &BTreeSet<String>,
) -> (Shape, bool) {
    // `चरः NAME ॱॱ TYPE भवति` — the type is right there.
    if lhs.first().is_some_and(|t| word(t) == W_VAR) {
        if let Some(m) = lhs.iter().position(|t| t.kind == Kind::LabelMark) {
            let ty: Vec<String> = lhs[m + 1..].iter().map(|t| t.text.clone()).collect();
            return (shape_of(&ty, structs), false);
        }
        return (Shape::Unknown, false);
    }
    // `a ॱ f भवति` — the FIELD's type.
    if let Some(m) = body.iter().position(|t| t.kind == Kind::MemberMark) {
        let base = word(&body[0]);
        let field = body.get(m + 1).map(word).unwrap_or("");
        let (Some((bty, is_global)), _) = (lookup(s, routine, base), ()) else {
            return (Shape::Unknown, false);
        };
        // the record the base names, through any wrapper
        let sname = bty.iter().rev().find(|t| structs.contains(*t));
        let shape = sname
            .and_then(|sn| s.fields.get(sn))
            .and_then(|f| f.get(field))
            .map_or(Shape::Unknown, |ty| shape_of(ty, structs));
        return (shape, is_global);
    }
    // `a अङ्कः i अन्तः भवति` — the ELEMENT's type.
    if body.iter().any(|t| word(t) == W_INDEX_OPEN) {
        let base = word(&body[0]);
        let Some((bty, is_global)) = lookup(s, routine, base) else {
            return (Shape::Unknown, false);
        };
        let elem: Vec<String> = match bty.iter().position(|t| t == W_INDEX_CLOSE) {
            Some(p) => bty[p + 1..].to_vec(),
            None => return (Shape::Unknown, is_global),
        };
        return (shape_of(&elem, structs), is_global);
    }
    // a plain name
    match lookup(
        s,
        routine,
        &body.first().map_or(String::new(), |t| t.text.clone()),
    ) {
        Some((ty, is_global)) => (shape_of(ty, structs), is_global),
        None => (Shape::Unknown, false),
    }
}

/// Every name written in this file, with the routine it is written in — used
/// to answer "is either name written AFTERWARDS". A write is `X भवति`,
/// `X ॱ f भवति` or `X अङ्कः … अन्तः भवति`, so the base of any assignment
/// target counts.
fn writes(sites: &[Site]) -> BTreeMap<String, usize> {
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    for s in sites {
        if s.target_base.is_empty() {
            continue;
        }
        *m.entry(s.target_base.clone()).or_insert(0) += 1;
        let bare = s
            .target_base
            .rsplit('\u{971}')
            .next()
            .unwrap_or(&s.target_base)
            .to_string();
        if bare != s.target_base {
            *m.entry(bare).or_insert(0) += 1;
        }
    }
    m
}

struct Census {
    /// Every `भवति` the LEXER calls a word. The census's own site count is
    /// asserted equal to it, so a statement the splitter mishandled cannot
    /// quietly shrink the population. `grep -o भवति` answers 4961 and is
    /// wrong three ways: it counts the word inside `॰` comments, inside
    /// `उक्तम् भवति इति` string literals (the T1 parser's own keyword table),
    /// and it would count it inside a longer word.
    tokens: usize,
    total: usize,
    rhs_bare_name: usize,
    shares: Vec<Site>,
    calls: Vec<Site>,
    octets_copies: usize,
    scalar_copies: usize,
    unknown: Vec<Site>,
    target_run_typed: usize,
    both_written: usize,
    either_written: usize,
    bindings: usize,
}

fn census() -> Census {
    let files = corpus();
    let lexed: Vec<(String, Vec<Token>)> = files
        .iter()
        .map(|(n, t)| {
            let toks = lex_t1(t).unwrap_or_else(|e| panic!("{n} must lex: {e:?}"));
            (n.clone(), toks)
        })
        .collect();

    // Every `संरचना` name in the WHOLE corpus: a type may name a record another
    // module declares, and a per-file table would call those Unknown.
    let mut structs = BTreeSet::new();
    for (_, toks) in &lexed {
        for i in 0..toks.len() {
            if word(&toks[i]) == W_STRUCT
                && let Some(n) = toks.get(i + 1)
            {
                structs.insert(n.text.clone());
            }
        }
    }

    // Every file's declarations FIRST, keyed by its `मण्डलम्` name, so a
    // `मण्डलॱनाम` on a right-hand side can be resolved where it is declared.
    let scopes: Vec<(String, Scope)> = lexed
        .iter()
        .map(|(n, toks)| (n.clone(), collect(toks)))
        .collect();
    let mut by_module: BTreeMap<String, Scope> = BTreeMap::new();
    for (_, sc) in &scopes {
        by_module.insert(
            sc.module.clone(),
            Scope {
                globals: sc.globals.clone(),
                locals: BTreeMap::new(),
                fields: sc.fields.clone(),
                routines: sc.routines.clone(),
                module: sc.module.clone(),
            },
        );
    }

    let mut tokens = 0usize;
    let mut all: Vec<Site> = Vec::new();
    for ((name, toks), (_, scope)) in lexed.iter().zip(scopes.iter()) {
        tokens += toks
            .iter()
            .filter(|t| t.kind == Kind::Word && t.text == W_BECOMES)
            .count();
        all.extend(sites(name, toks, scope, &structs, &by_module));
    }

    let total = all.len();
    let bindings = all.iter().filter(|s| s.is_binding).count();
    let rhs_bare_name = all
        .iter()
        .filter(|s| s.rhs_name.is_some() && !s.rhs_is_call)
        .count();
    let target_run_typed = all.iter().filter(|s| s.target_shape.shares()).count();
    let shares: Vec<Site> = all
        .iter()
        .filter(|s| s.rhs_name.is_some() && !s.rhs_is_call && s.rhs_shape.shares())
        .cloned()
        .collect();
    // A zero-argument call whose return type is an arena or a record. Written
    // exactly like a share and NOT one — unless the routine hands back
    // something it also keeps, which no site can show and no test asserts.
    let calls: Vec<Site> = all
        .iter()
        .filter(|s| s.rhs_is_call && s.rhs_shape.shares())
        .cloned()
        .collect();
    let octets_copies = all
        .iter()
        .filter(|s| s.rhs_name.is_some() && !s.rhs_is_call && s.rhs_shape == Shape::Octets)
        .count();
    let scalar_copies = all
        .iter()
        .filter(|s| s.rhs_name.is_some() && !s.rhs_is_call && s.rhs_shape == Shape::Scalar)
        .count();
    let unknown: Vec<Site> = all
        .iter()
        .filter(|s| s.rhs_name.is_some() && !s.rhs_is_call && s.rhs_shape == Shape::Unknown)
        .cloned()
        .collect();
    // A SHARE ONLY BITES WHERE ONE SIDE IS LATER WRITTEN. Counted corpus-wide
    // and by name, because several of these shares put a GLOBAL on a local and
    // the global is written by a different routine — often in a different file
    // (`parse.t1` appends into `वास्तुॱअभिव्यञ्जककोश`, which `ast.t1` declares).
    // Counting per routine would have answered zero for exactly the sites that
    // matter. The share statement itself is one of the writes, so a name needs
    // MORE THAN ONE to count as written again.
    let w = writes(&all);
    let both_written = shares
        .iter()
        .filter(|s| {
            let t = w.get(&s.target_base).copied().unwrap_or(0);
            let r = s
                .rhs_name
                .as_deref()
                .and_then(|n| w.get(n).copied())
                .unwrap_or(0);
            t > 1 && r > 0
        })
        .count();
    let either_written = shares
        .iter()
        .filter(|s| {
            let t = w.get(&s.target_base).copied().unwrap_or(0);
            let r = s
                .rhs_name
                .as_deref()
                .and_then(|n| w.get(n).copied())
                .unwrap_or(0);
            t > 1 || r > 0
        })
        .count();
    Census {
        tokens,
        total,
        rhs_bare_name,
        shares,
        calls,
        octets_copies,
        scalar_copies,
        unknown,
        target_run_typed,
        both_written,
        either_written,
        bindings,
    }
}

#[test]
#[ignore = "census: a measurement, not a guard — run with --ignored"]
fn measure_corpus_shares() {
    let c = census();
    // THE SCANNER'S OWN AUDIT, before any of its findings are read. If the
    // statement splitter dropped or doubled a site this is where it shows.
    assert_eq!(
        c.total, c.tokens,
        "the census must classify EVERY `भवति` the lexer sees, not a subset: \
         {} statements against {} word tokens",
        c.total, c.tokens
    );
    println!("METRIC paradigm_becomes_sites {}", c.total);
    println!("METRIC paradigm_becomes_bindings {}", c.bindings);
    println!(
        "METRIC paradigm_becomes_assignments {}",
        c.total - c.bindings
    );
    println!("METRIC paradigm_becomes_rhs_bare_name {}", c.rhs_bare_name);
    println!("METRIC paradigm_runs_assigned {}", c.target_run_typed);
    println!("METRIC paradigm_runs_shared {}", c.shares.len());
    println!("METRIC paradigm_runs_from_zero_arg_call {}", c.calls.len());
    println!("METRIC paradigm_becomes_copies_octets {}", c.octets_copies);
    println!("METRIC paradigm_becomes_copies_scalar {}", c.scalar_copies);
    println!(
        "METRIC paradigm_runs_shared_and_both_written {}",
        c.both_written
    );
    println!(
        "METRIC paradigm_runs_shared_and_either_written {}",
        c.either_written
    );
    println!("METRIC paradigm_becomes_unplaced {}", c.unknown.len());

    let mut per_file: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &c.shares {
        *per_file.entry(s.file.as_str()).or_insert(0) += 1;
    }
    println!("\n── the {} sharing sites ──", c.shares.len());
    for s in &c.shares {
        println!(
            "{}:{}  [{}]  {} भवति {}   ({:?} ← {:?}){}",
            s.file,
            s.line,
            s.routine,
            s.target,
            s.rhs_name.as_deref().unwrap_or("?"),
            s.target_shape,
            s.rhs_shape,
            if s.target_is_global || s.rhs_is_global {
                "  GLOBAL"
            } else {
                ""
            }
        );
    }
    println!("\n── per file ──");
    for (f, n) in &per_file {
        println!("{f}: {n}");
    }

    let mut unplaced: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &c.unknown {
        *unplaced
            .entry(s.rhs_name.as_deref().unwrap_or("?"))
            .or_insert(0) += 1;
    }
    println!(
        "\n── the {} zero-argument CALLS returning a run or record ──",
        c.calls.len()
    );
    for s in &c.calls {
        println!(
            "{}:{}  [{}]  {} भवति {}()   ({:?})",
            s.file,
            s.line,
            s.routine,
            s.target,
            s.rhs_name.as_deref().unwrap_or("?"),
            s.rhs_shape
        );
    }

    let mut v: Vec<_> = unplaced.into_iter().collect();
    v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!(
        "\n── the {} names the census could not place, by name ──",
        c.unknown.len()
    );
    for (n, k) in v.iter().take(40) {
        println!("{n}: {k}");
    }
    for s in &c.unknown {
        println!(
            "  {}:{}  [{}]  {} भवति {}",
            s.file,
            s.line,
            s.routine,
            s.target,
            s.rhs_name.as_deref().unwrap_or("?")
        );
    }
}

/// THE PIN, and it is NOT `#[ignore]`d.
///
/// The census above prints; this one guards. A census whose numbers nobody
/// re-reads is a number, and the whole point of the count is that a new
/// sharing site should announce itself. It asserts the COUNTS rather than
/// "the census still runs", because several of these move together — the
/// `parse.t1` fix moved shares and unplaced in opposite directions at once
/// and a "something changed" guard would have said nothing useful about it.
///
/// It costs about 1.6s: lexing twenty `.t1` files, no interpreter.
///
/// **68 → 67 on 2026-09-05 (`W-266`), and the move is a share REMOVED.**
/// `सङ्कोचः`'s save of the error slot — `चरः पूर्वदोषः ॱॱ सङ्केतनदोष भवति
/// अन्तिमसङ्केतनदोषः` — was one of the 68, and it was the first of them shown
/// to be load-bearing: run with `सङ्केतनदोषरचना` rewritten as in-place field
/// writes, the save/restore around it answered ९ instead of १. It is now five
/// field copies onto a fresh record, so the census no longer sees it. The
/// RESTORE (`encode.t1:4870`) is still a share and stays counted; its margin
/// says why it is intended. `w266-error-slot.rs` runs both halves.
#[test]
fn the_corpus_share_count_is_pinned() {
    let c = census();
    pin_report!(
        c.total,
        c.tokens,
        "every `भवति` the lexer sees must be classified: {} against {}",
        c.total,
        c.tokens
    );
    pin_report!(
        (c.shares.len(), c.calls.len(), c.unknown.len()),
        // 2026-09-05, W-266: (68, 16, 10) -> (67, 16, 10), MEASURED on the
        // merged tree and not computed from the pre-merge reading. THE MOVE IS
        // A SHARE REMOVED, and it is the first of these sites ever shown to be
        // load-bearing by running it: `सङ्कोचः`'s save of the one-slot error
        // state, `चरः पूर्वदोषः ॱॱ सङ्केतनदोष भवति अन्तिमसङ्केतनदोषः`. With
        // `सङ्केतनदोषरचना` rewritten as in-place field writes — a change 3,300
        // lines away — the save/restore around it answered ९ instead of १, the
        // refusal it exists to discard. It is now five field copies onto a
        // fresh record, so this census no longer sees it. The RESTORE
        // (`encode.t1:4870`) is still a share, stays counted, and its margin
        // says why it is intended. `w266-error-slot.rs` runs both halves.
        // 2026-09-06, W-254: (70, 17, 13) -> (71, 17, 13), TAKEN FROM THIS
        // ASSERTION'S OWN FAILURE on this tree — and NOT from the trunk's
        // gate, which read the same triple on a tree carrying the merge with
        // main. The two agreed here, which is itself the finding: the new
        // share is this row's and main's other `.t1` movement contributed
        // none, so the corpus census separates the two cleanly.
        //
        // THE MOVE IS ONE SHARE ADDED AND IT IS NAMED: `ir.t1:521`,
        // `पाठाज्ञायोजनम्`'s `आज्ञाकोश अङ्कः स्थानम् अन्तः भवति नव` — the
        // write-back half of copy-modify-write, the FIFTH instance of an
        // idiom this census already counts four times (`तुलना`, `आहार`,
        // `निधान`, `आह्वान` at :460, :469, :479 and :537). Its margin at the
        // site says the share is intended and why it is safe: `नव` is read
        // out of the arena two lines above, nothing else holds it, and it does
        // not outlive the routine. `calls` and `unknown` are UNMOVED at 17 and
        // 13, which is the check that this row added a share and reclassified
        // nothing.
        // (71, 17, 13) -> (71, 18, 13) on 2026-09-07, `W-chain`. THE NEW SITE
        // IS A CALL, NOT A SHARE, and the census named it rather than my
        // reading it off a diff: `shrinkhala.t1:55 [निर्णयः] निर्णायकः ॱॱ
        // अर्थॱनिर्णायक भवति अर्थॱनिर्णायकारम्भः() (Record)` — the driver
        // opens a resolver so it can hand the SAME resolver to the typechecker
        // on the next line, which is what makes resolve-and-typecheck one
        // stage rather than two.
        //
        // The driver has three zero-argument calls and only this one counts:
        // the other two answer `न६४`, and this category is calls returning a
        // RUN OR RECORD. I predicted the wrong one of the three from the
        // source, and the census's own `--ignored` run named the right one in
        // a single line. Ask the census; do not reason about it.
        //
        // Shares UNMOVED at 71 and the undefined-type list unmoved at 13,
        // which is the reading that matters: a driver that had bound one arena
        // to two names would have moved the first figure.
        // (71, 18, 13) -> (79, 18, 13) on 2026-09-07, the object builder.
        // Zero-argument calls and unplaced names both UNMOVED: this row adds no
        // call returning a run or record, and names no type the census cannot
        // place.
        //
        // THE EIGHT, NAMED BY THE CENSUS AND NOT BY ME — four arena APPENDS
        // (`फलम् अङ्कः लेखः अन्तः भवति <record>` at :5626, :5691, :5721, :5791)
        // and four fields bound to an arena (`वस्तु ॱ संज्ञाः`,
        // `ॱ पुनःस्थापनानि`, `ॱ शोधनपुनःस्थापनानि`, `ॱ दत्तपुनःस्थापनानि` at
        // :5829, :5830, :5844, :5845). Every one is intended: an object OWNS
        // the arenas it is built from, and the builder holds no second writer
        // to any of them after `वस्तुरचना` returns.
        //
        // I DECLINED TO PREDICT THIS FIGURE AND THE REASON TURNED OUT TO BE
        // STRONGER THAN THE ONE I GAVE. I said I could see seven candidate
        // sites but would be guessing at the census's rule. The census counts
        // only FOUR of those seven — the arena bindings, not the octet-run ones
        // — and four sites I had not considered at all. **The model was wrong
        // in COMPOSITION, not merely in count**, so a predicted 7 against a
        // measured 8 would have looked like one stray site and hidden three
        // wrong ones and four missed ones. Ask the census; do not reason about
        // it.
        // (79, 18, 13) -> (80, 19, 13) on 2026-09-08, THE DRIVER'S BACK HALF
        // ALONE. Taken from this assertion's own failure, and both new sites
        // NAMED by the census's `--ignored` run and checked against
        // `origin/main` rather than read off a diff:
        //
        //   share  `shrinkhala.t1:227` `[वस्तुप्रतिबिम्बम्]`
        //          `वस्तूनि अङ्कः ० अन्तः भवति वस्तु`        main 0 · here 1
        //   call   `shrinkhala.t1:212` `[पाठवस्तुरचना]`
        //          `... भवति वाक्यविभागॱकार्यक्रमरचना()`     main 0 · here 1
        //
        // BOTH ARE THIS ROW'S AND THE ATTRIBUTION IS NOW CLEAN. The same pair
        // was measured on a tree that also carried the record row, where the
        // increments were identical — so that reading was RIGHT, but it could
        // not have proved the record row contributed nothing. This one can:
        // there is no other lane in this tree.
        //
        // The share is an arena APPEND, the idiom this census already counts at
        // `encode.t1:5626` and its three siblings, and it is structural to the
        // pattern rather than incidental: a routine that answers an INDEX
        // rather than a constructed slice has to append. `unknown` UNMOVED at
        // 13, so nothing was reclassified and no type went unplaced.
        //
        // (80, 19, 13) -> (83, 19, 13) on 2026-09-08, THE CORPUS WALK AND THE
        // `%pcrel_lo12` EMITTER. Taken from this assertion's own failure, and
        // ALL THREE NAMED by the `--ignored` run rather than counted:
        //
        //   share  `shrinkhala.t1:345` `[मण्डलानिप्रतिबिम्बम्]`
        //          `वस्तूनि अङ्कः वस्तुसंख्या अन्तः भवति आरम्भवस्तु`   the startup object
        //   share  `shrinkhala.t1:358` `[मण्डलानिप्रतिबिम्बम्]`
        //          `वस्तूनि अङ्कः वस्तुसंख्या अन्तः भवति वस्तु`        each source's object
        //   share  `encode.t1:5732`    `[वस्तुसंज्ञासारणी]`
        //          `फलम् अङ्कः १ अन्तः भवति पाठखण्डा`               the section symbol
        //
        // THE THIRD IS IN A DIFFERENT FILE FROM THE OTHER TWO AND A BARE +3
        // WOULD NEVER HAVE SHOWN IT. Two units landed together here — the
        // corpus walk in `शृङ्खला` and the relocation emitter in `सङ्केतन` —
        // and the count alone reads as "the driver added three". Naming them
        // splits it 2 and 1 across two files and two rows. **The census was
        // asked for its members precisely because the assertion says to, and
        // the members disagreed with the obvious reading of the number.**
        //
        // The two `शृङ्खला` sites are the same arena-APPEND idiom as `:227`
        // above and structural for the same reason. The `सङ्केतन` one is the
        // `ॱपाठ` section symbol a `%pcrel_lo` names — minted at index १ before
        // every label, which is also what shifted `t1_vastu_build`'s fixed
        // indices by one. `calls` and `unknown` BOTH UNMOVED at 19 and 13, so
        // nothing was reclassified and no type went unplaced.
        // (83, 19, 13) -> (84, 19, 13) on 2026-09-09, THE SYMBOL-MAPPING ROW.
        // Taken from this assertion's own failure, and the site NAMED by the
        // census's `--ignored` run then checked against `origin/main` rather
        // than read off a diff:
        //
        //   share  `shrinkhala.t1:133` `[निर्णयः]`
        //          `निर्णायकधारः भवति निर्णायकः`   (Record ← Record) GLOBAL
        //          main 0 · here 1
        //
        // `calls` and `unknown` are UNMOVED at 19 and 13, which is the check
        // that this row added a share and reclassified nothing.
        //
        // THE SHARE IS INTENDED AND IT IS THE POINT OF THE BINDING. `निर्णयः`
        // built a resolver and discarded it; `नामसञ्चयः` needs that same
        // resolver to turn a routine's name into its symbol, and the global is
        // how it reaches one stage from the next. A write through either name
        // being visible through the other is the mechanism, not a hazard.
        //
        // ITS VALIDITY WINDOW IS ONE MODULE'S COMPILE, and that is an ORDERING
        // property rather than a structural one. `मण्डलसङ्कलनम्` runs
        // पठनम् → निर्णयः → रचना → नामसञ्चयः → उत्सर्जनम् inside one call, so the
        // holder is written and read before the next source overwrites it. Safe
        // today BY ORDERING, NOT BY CONSTRUCTION: a driver that resolved all
        // twenty sources and then named them would read the twentieth resolver
        // for every one of them, and every name would still be a plausible
        // name. THE FIX IF THAT DAY COMES IS TO PASS THE RESOLVER RATHER THAN
        // HOLD IT — recorded here because the defect would be silent.
        //
        // I PREDICTED THE PARTITION BELOW AND NOT THIS, and the miss is worth
        // more than the hit: I reasoned about which pins my change could move
        // by counting KEYWORDS in my diff, and a share is not a keyword — it is
        // a RELATION between two names that the census derives. A diff-shaped
        // model of a corpus census sees only what the diff spells.
        // 2026-09-12, W-293: (84, 19, 13) -> (85, 19, 13), MEASURED on the merged
        // tree at c4e77929, not computed from either branch's reading. THE MOVE IS
        // ONE SHARE ADDED and it is INTENDED.
        //
        // `encode.t1:5973` — `दत्तलेखाः अङ्कः दत्तलेखक्रमः अन्तः भवति दत्तलेखा` — stores
        // the freshly built data-relocation record into the arena that will become
        // the object's `दत्तपुनःस्थापनानि`. It is the exact shape of the TEXT
        // relocation share twenty lines above it, `encode.t1:5902`
        // (`फलम् अङ्कः लेखः अन्तः भवति लेखा`), which has been counted here since the
        // text path was wired. **The data path was never wired until now**, which
        // is why its twin existed and it did not.
        //
        // THE SHARE IS SAFE FOR THE REASON ITS TWIN IS: the record is constructed
        // immediately before the store and never read back through its local name
        // afterwards, so there is no second writer for the aliasing to surprise.
        // A write through `दत्तलेखा` after the store would be visible through the
        // arena — that is what this census exists to notice — and there is none.
        //
        // `calls` and `unknown` are UNMOVED at 19 and 13, which is the control:
        // this landing added a share and reclassified nothing.
        // 2026-09-13, task08 parts 1–2 (`2a57d72f`): (85, 19, 13) -> (86, 19, 13),
        // MEASURED on the trunk with task08 merged — ONE share added, calls and
        // unknown unmoved. The member was NOT isolated: a grep for a binding
        // initialised from a bare name over the unit's diff found none, so the
        // census is classifying something that grep does not — read the
        // assertion, not the pattern. Edited by anchored line: the old tuple is
        // also the W-293 HISTORY entry above.
        (86, 19, 13),
        // 2026-09-05, W-265: (67, 16, 10) -> (70, 17, 13), TAKEN FROM THIS
        // ASSERTION'S OWN FAILURE and never computed. All three move for
        // reasons this census already carries an instance of, which is the
        // reading that matters — not one of them is a new KIND of site.
        //
        // THE THREE NEW SHARES ARE `निर्णायकारम्भः`'s RESET, whose margin
        // already states the intent for the five above them: an arena keyed by
        // a numbering is cleared where that numbering restarts, and each gets
        // its OWN empty run because one run behind several globals makes them
        // the same run and the last write wins — a lesson that routine's own
        // note records costing three tests. `प्रकारसंज्ञामण्डलानि`,
        // `प्रकारसंज्ञाभेदाः` and `प्रकारसंज्ञामूल्यानि` are the type-name table's
        // integer runs; the intent is stated at the site.
        //
        // THE NEW CALL is `प्रकारसंज्ञार्थः`'s `भवति दोषार्थः()` — the same benign
        // fresh-poison-record shape as the other sixteen.
        //
        // THE THREE NEW UNPLACED ARE THE SAME STALE TYPE LIST W-248 recorded
        // two lines of below, not a new defect: `artha.t1:733` and `:879` bind
        // a `नाम` typed `पाठ`, and `:1540` resets the table's `पाठ` run. This
        // census still carries the RETIRED `पाठः` in its own type list, so
        // every `पाठ` binding lands here. Recorded rather than fixed, for the
        // reason W-248 gives: it is `W-267`'s subject, and widening the list
        // quietly would hide the second instance of a bug whose first instance
        // cost five silent days.
        // 2026-09-05, W-248: (68, 14, 8) -> (68, 16, 10), taken from this
        // assertion when it failed on the merged tree, never computed.
        // SHARES ARE UNMOVED AT 68 — the load-bearing number — so W-248
        // introduced no new aliasing. The two new CALLS are `भवति दोषार्थः()`
        // inside its new `पाठप्रकारार्थः` and `प्रकारपाठार्थः`: the same benign
        // fresh-poison-record shape as the other fourteen.
        //
        // THE TWO NEW UNPLACED SITES ARE THIS CENSUS BEING BEHIND THE GRAMMAR,
        // NOT A NEW DEFECT — `artha.t1:657` and `:660`, both `भवति नाम` where
        // `नाम` is typed `पाठ`. ADR-0029 froze `पाठ` on 2026-08-31 and W-248
        // found the TYPE READER still matching only the retired `पाठः`, so
        // every `ददाति पाठ` read as poison for five days in silence. This
        // census carries its own type list and has the SAME stale spelling.
        // Recorded rather than fixed here: it is W-267's subject and belongs
        // with it, and widening this list quietly would hide the second
        // instance of a bug whose first instance cost five silent days.
        "the corpus's share census moved. 70 `भवति` sites bind one arena or \
         record to TWO names — a write through either is visible through the \
         other; 17 more are zero-argument calls spelled exactly like one; 13 \
         name a type this census's own list does not define. A change here is a \
         new share, a new call, or a bug in this census — say which, and if it \
         is a new share say in its margin that the share is intended.\n\
         Run `cargo test -p sadhana-t1 --test w250-shares -- --ignored \
         --nocapture` to see every site by name."
    );
    // THE `भवति` TOTAL IS THIS FILE'S TO PIN. THE SPLIT BELOW IS NOT THE
    // GRAMMAR'S SPLIT — corrected 2026-09-05 by `W-268`, and the sentence that
    // stood here until then said the opposite.
    //
    // It said `spec/grammar-t1.ebnf`'s block was stale "but every number in it
    // is roughly a third of today's", with today's given as 2489 / 2416. The
    // block WAS stale and has since been re-taken; the second half was wrong.
    // `classify` sets `is_binding` from `lhs.first() == चरः`, and the grammar's
    // form is `binding = [ "सार्वजनिक" ] , "चरः" , …` — so every one of the
    // corpus's `सार्वजनिक चरः` bindings lands on the assignment side of the
    // numbers below. The two splits are one subtraction apart, and the size of
    // that subtraction is the grammar census's `public-bindings` key rather
    // than a digit kept here to go stale. `c.total` agrees to the site either
    // way, which is why the total above is this file's to pin and the split
    // is not.
    //
    // The numbers are LEFT AS THEY ARE rather than corrected, because they are
    // a true measurement of what this census measures and the share counts
    // above rest on the same `lhs.first()` reading. What is fixed is the LABEL:
    // this is the split of `भवति` sites by their statement's first token, and
    // `crates/sadhana-t1/tests/w268-census.rs` holds the grammar's, re-taken
    // against `spec/grammar-t1.ebnf` on every gate.
    pin_report!(
        (c.bindings, c.total - c.bindings, c.total),
        // 2026-09-06, W-278: (2572, 2526, 5098) -> (2582, 2540, 5122), TAKEN
        // FROM THIS ASSERTION'S OWN FAILURE. 10 bindings and 14 assignments,
        // and 10 + 14 = 24 is the whole of the total's move — the same check as
        // below: a census whose total moved by a different amount than its two
        // halves would mean a site was RECLASSIFIED rather than added, which is
        // a different event and would want a different explanation. The sites
        // are this row's: `ir.t1`'s global arm, its globals arenas and the
        // declaration branch that fills them, and `yantrotsarjana.t1`'s
        // `यन्त्रवैश्विकोत्सर्जनम्` with the globals section in
        // `यन्त्रदत्तोत्सर्जनम्`.
        // (2586, 2543, 5129) WAS agent/nameglobal's reading, correct for its own
        // tree. It survived my conflict resolution because it sat OUTSIDE the
        // markers as context while main's tuple sat inside them, so the file
        // briefly carried TWO expected tuples and rustc caught it as a missing
        // format string, not as a wrong number. A conflict's markers bound the
        // text git could not choose between, NOT the text the change is about.
        // 2026-09-06, W-278 AGAIN, LATER THE SAME DAY:
        // (2582, 2540, 5122) -> (2586, 2543, 5129), TAKEN FROM THIS ASSERTION'S
        // OWN FAILURE. 4 bindings and 3 assignments, and 4 + 3 = 7 is the whole
        // of the total's move, so nothing was reclassified — the same check the
        // entry above makes. The sites are the guards and separator fixes the
        // row's later commits added: the two locals `घोषणास्थलम्` and
        // `घोषितम्` that make two arena reads bounded, and the explicit
        // `उत्सर्जनॱविवरयोजनम् १` separators that a literal's edge blanks do
        // not survive.
        //
        // THIS PIN AND THE ADR-0030 LEDGER BOTH WENT RED FOR ONE REASON, AND IT
        // IS WORTH THE SENTENCE: BOTH WERE RE-TAKEN MID-ROW AND THEN THE ROW
        // KEPT EDITING `.t1`. A census pin measures the tree the run saw, not
        // the row's intent, so re-taking one before the last source edit pins a
        // tree that is about to stop existing. Two pins, two crates, one cause.
        // Re-take censuses LAST, after the final `.t1` edit — or expect to take
        // them again.
        //
        // 2026-09-06, W-254: (2538, 2486, 5024) -> (2572, 2526, 5098), TAKEN
        // FROM THIS ASSERTION'S OWN FAILURE. 34 bindings and 40 assignments,
        // and 34 + 40 = 74 is the whole of the total's move — which is the
        // check, since a corpus census that moved its total by a different
        // amount than its two halves would mean a site was reclassified rather
        // than added. The sites are this row's: `ir.t1`'s उक्त arm and its
        // three new routines, and `yantrotsarjana.t1`'s `यन्त्रपाठावतरणम्`
        // with the string pool's section in `यन्त्रदत्तोत्सर्जनम्`.
        // 2026-09-06, `W-kosha` phase 2: (2572, 2526, 5098) -> (2622, 2536, 5158),
        // TAKEN FROM THIS ASSERTION'S OWN FAILURE. 50 bindings and 10
        // assignments, and 50 + 10 = 60 is the total's whole move — sites were
        // ADDED and none reclassified. `kosha.t1` is the file: an ELF writer is
        // mostly bindings, because every header field is computed into a name
        // before it is written.
        //
        // `भवति` here is 5158 and `spec/grammar-t1.ebnf`'s CENSUS says 5158 too.
        // Two instruments over the same corpus, agreeing.
        // 2026-09-06, same row, second reading: (2622, 2536, 5158) ->
        // (2623, 2537, 5160), TAKEN FROM THIS ASSERTION'S OWN FAILURE. One
        // binding and one assignment, and 1 + 1 = 2 is the total's whole move.
        // The padding walk became arithmetic.
        // 2026-09-06, THE MERGE OF agent/kosha AND agent/nameglobal:
        // (2623, 2537, 5160) -> (2637, 2554, 5191), TAKEN FROM THIS
        // ASSERTION'S OWN FAILURE. 14 bindings and 17 assignments, and
        // 14 + 17 = 31 is the total's whole move — sites were ADDED and none
        // reclassified. The rows are `ir.t1` and `yantrotsarjana.t1` lowering
        // `name_global`, 255 lines of `.t1`.
        //
        // NEITHER LANE'S NUMBER SURVIVED THE MERGE AND NEITHER WAS WRONG.
        // Both branches added corpus, so each figure was true for its own tree
        // at its own commit and false for the tree holding both. This is not
        // staleness — there was no moment when either was true for everybody.
        // A census pin is the artefact guaranteed to conflict when two lanes
        // add corpus at once, and the resolution is never arithmetic: resolve
        // the hunks to one side, then re-take every figure here.
        // 2026-09-06, THE THREE-BRANCH INTEGRATION (kosha + nameglobal +
        // vastu-objects): (2637, 2554, 5191) -> (2639, 2555, 5194), TAKEN FROM
        // THIS ASSERTION'S OWN FAILURE. +2 bindings, +1 assignment, and
        // 2 + 1 = 3 is the total's whole move: added, none reclassified.
        //
        // AND IT MATCHES agent/vastu-objects' OWN MOVE OVER ITS OWN BASE.
        // That lane measured (2623, 2537, 5160) -> (2624, 2538, 5162) -> then
        // (2625, 2538, 5163) across its two commits: +2 bindings, +1
        // assignment, +3 total. The same delta arriving on a different base
        // is what INDEPENDENCE looks like from a census — the change carries
        // its own contribution regardless of what else the tree holds.
        //
        // Three pins re-taken here and the two in `grammar_t1.rs` were both
        // PREDICTED before the run and both landed exactly. A re-taken figure
        // is only what the tree says; a figure written down first and then
        // matched is a claim that could have been refuted and was not.
        // 2026-09-07, THE BAD-FIELD REFUSAL: (2639, 2555, 5194) ->
        // (2640, 2569, 5209), TAKEN FROM THIS ASSERTION'S OWN FAILURE and not
        // computed. +1 binding, +14 assignments, and 1 + 14 = 15 is the total's
        // whole move — added, none reclassified, which is the check every entry
        // above makes.
        //
        // The sites are all in `artha.t1` and all one change: the `असत्क्षेत्र…`
        // family's five declarations, its per-source zeroing in
        // `प्रकारपरीक्षकारम्भः`, and the क्षेत्र arm recording the refusal it had
        // been claiming in a margin and not making. `सार्वजनिक चरः` lands on the
        // ASSIGNMENT side under this file's `lhs.first()` reading, which is why
        // a change that is mostly new globals moves assignments and barely
        // moves bindings.
        // 2026-09-07, THE DRIVER MERGE: (2640, 2569, 5209) -> (2659, 2571, 5230),
        // TAKEN FROM THIS ASSERTION'S OWN FAILURE. +19 bindings, +2 assignments,
        // and 19 + 2 = 21 is the total's whole move — added, none reclassified.
        // `भवति` reads 5230 here AND 5230 in the grammar census: two instruments
        // over one corpus agreeing, which is the only reason either is quotable.
        // 2026-09-07, THE FIELD-LOWERING MERGE: (2659, 2571, 5230) ->
        // (2680, 2577, 5257), TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // +21 bindings, +6 assignments, and 21 + 6 = 27 is the total's whole
        // move — added, none reclassified. `भवति` reads 5257 here AND 5257 in
        // the grammar census: two instruments over one corpus agreeing.
        //
        // THE ARM THAT LOWERS `क्षेत्र` IS ITSELF MADE OF `क्षेत्र` — it reads
        // `वस्तुघोषणा ॱ प्राचलादि` and its kin — so writing it enlarged the very
        // population it exists to reduce. That is why this lane's branch-internal
        // delta (-203) and its main-side delta (-167) differ by 36.
        // (2680, 2577, 5257) -> (2724, 2643, 5367) on 2026-09-07, the object
        // builder. THE TOTAL WAS PREDICTED EXACTLY AND THE SPLIT WAS NOT, by
        // exactly ten — and the ten are named by this assertion's own label.
        //
        // The prediction took bindings as the `चरः` count, 54, giving
        // (2734, 2633). Measured (2724, 2643): 44 and 66. THE DIFFERENCE IS
        // THE TEN `सार्वजनिक चरः` THIS ROW ADDS. This census splits on a
        // statement's FIRST TOKEN, and those statements begin `सार्वजनिक` —
        // so they land in the other bucket. The label below says precisely
        // that, and `w268-census.rs` counts them as bindings BY THE GRAMMAR,
        // which is why its figure moved by the full 54.
        //
        // THE TWO CENSUSES DISAGREE ON PURPOSE AND BOTH ARE RIGHT. That is
        // what the label is for, and I had read it in this session before
        // predicting against it. The total, which does not depend on the
        // split, was predicted exactly by both instruments: 5367 here and
        // `भवति` 5367 there.
        // (2724, 2643, 5367) -> (2748, 2646, 5394) on the merge with
        // agent/index and agent/storeprobe. THE TOTAL AGREES WITH `w268`'s
        // `भवति` AT 5394 — two censuses, different rules, different files, one
        // number — which is the check that still works when no single delta is
        // attributable to one row.
        // (2748, 2646, 5394) -> (2760, 2647, 5407) on 2026-09-08, THE DRIVER'S
        // BACK HALF ALONE, taken from this assertion's own failure. Bindings
        // +12, assignments +1, and 12 + 1 = 13 IS THE WHOLE OF THE TOTAL'S
        // MOVE, so nothing was reclassified.
        //
        // AND `w268-census.rs` MEASURED THE SAME MOVE INDEPENDENTLY, BY
        // DIFFERENT RULES IN A DIFFERENT FILE: bindings +12, assignments +1,
        // `भवति` +13, and its `भवति` reads 5407 where this total reads 5407.
        // The two censuses AGREE ON THE SPLIT here, where they normally differ
        // by the public bindings — and `public-bindings` is UNMOVED at 567,
        // which is exactly why: this row declares no new name at module scope,
        // so there is nothing for the two rules to disagree about. **The size
        // of their disagreement is itself derivable, and this time it derives
        // to zero.**
        //
        // ONE LANE IN THE TREE, so every figure here is attributable to one
        // author for the first time in several readings.
        //
        // (2760, 2647, 5407) -> (2773, 2661, 5434) on 2026-09-08, THE CORPUS
        // WALK AND THE `%pcrel_lo12` EMITTER, taken from this assertion's own
        // failure. Bindings +13, assignments +14, and **13 + 14 = 27 IS THE
        // WHOLE OF THE TOTAL'S MOVE**, so nothing was reclassified.
        //
        // AND THIS TIME THE TWO CENSUSES SHOULD DISAGREE ON THE SPLIT — BY
        // EXACTLY TWO — WHICH IS A PREDICTION AND NOT AN OBSERVATION. The note
        // above records them agreeing because `public-bindings` was UNMOVED at
        // 567: nothing was declared at module scope, so the two rules had
        // nothing to differ over. **This branch declares two:**
        // `संयोजितवस्तुसंख्या` in `शृङ्खला` and `पाठखण्डसंज्ञाङ्कः` in `सङ्केतन`.
        // So `public-bindings` should read 569, `w268`'s `भवति` should still
        // read 5434 — the TOTAL never depends on the split — and the split
        // itself should differ by those two.
        //
        // IF `w268` READS 5434 AND ITS SPLIT DIFFERS BY ANYTHING BUT TWO, the
        // disagreement is not the public bindings and one of the two censuses
        // has moved for a reason nobody has named. That is the falsifier, and
        // it is worth more than the agreement: two instruments agreeing proves
        // less than two instruments differing by a predicted amount.
        // (2773, 2661, 5434) -> (2781, 2671, 5452) on 2026-09-09, THE
        // SYMBOL-MAPPING ROW. Taken from this assertion's own failure, TWICE,
        // and the second reading is the one pinned.
        //
        // FIRST READING (2781, 2673, 5454), AND IT WAS A CORRECT COUNT OF A
        // BROKEN TREE. `शृङ्खला` declared `निर्णायकधारः` and `निर्णायकसिद्धः`
        // TWICE each — two branches each declared the pair and the merge was
        // CLEAN, the additions sitting in different hunks. Removing the second
        // pair took two `सार्वजनिक चरः ... भवति` lines out, and on THIS census's
        // rule those two sit on the ASSIGNMENT side: 2673 -> 2671, total
        // 5454 -> 5452, `bindings` UNMOVED at 2781. A census cannot tell a
        // corpus from a corpus with a duplicate in it; the load-order guard in
        // `t1_modules.rs` is what separated them.
        //
        // I PREDICTED (2785, 2669, 5454) BEFORE THE FIRST RUN. The TOTAL was
        // exact; the SPLIT was wrong by four in each direction, and the four are
        // named: the `सार्वजनिक चरः` bindings this row adds. **This census splits
        // on the statement's FIRST TOKEN**, as the assertion message above says,
        // so `सार्वजनिक चरः x ॱॱ T भवति v` begins with `सार्वजनिक` and does not
        // reach the `चरः` bucket. I had apportioned by the GRAMMAR's binding
        // rule, under which all twelve added `चरः` are bindings — which is what
        // `w268-census.rs` measures, and there the same model was exact.
        //
        // 8 + 12 = 20 WAS THE WHOLE OF THE FIRST MOVE and -2 the whole of the
        // second, so nothing was reclassified in either — the check this pin's
        // own margin asks for.
        //
        // WHAT THE MISS IS WORTH: the total and the split fail on disjoint
        // inputs, so WHICH of them disagrees names the defect. A wrong total
        // would have meant my keyword count was wrong. A right total with a
        // wrong split means the CLASSIFICATION was wrong, which is exactly what
        // it was — and `grammar_t1.rs:3212`'s `bindings + assignments == total`
        // stays green throughout, because the error moved four sites BETWEEN the
        // buckets and the identity cannot see that. Two instruments, one blind
        // where the other sees.
        // (2781, 2671, 5452) -> (2794, 2688, 5482) on 2026-09-09, THE `batch-1`
        // SEVEN-UNIT TREE at `02f1e346`. TAKEN FROM THIS ASSERTION'S OWN
        // FAILURE, never computed. bindings +13, assignments +17, total +30.
        //
        // THE TOTAL WAS PREDICTED BEFORE THE RUN AND THE SPLIT WAS NOT. A lane
        // filed `भवति +30` — its own +26, a peer's +4 — against `4be03027`,
        // hours before this tree existed, and the total moved by exactly
        // thirty. The SPLIT it predicted was 18 + 12, and this pin reads
        // 13 + 17.
        //
        // BOTH ARE RIGHT AND THEY ARE NOT THE SAME PARTITION. The prediction
        // split grammatically — a `चरः` site is a binding — and this file
        // splits by the statement's FIRST token, so the `सार्वजनिक चरः` sites
        // land as assignments here and as bindings there. One population, two
        // rules, one total. `w268-census.rs` holds the grammatical split and is
        // re-taken against `spec/grammar-t1.ebnf` on every gate; the difference
        // between the two is the `public-bindings` key, not a digit kept here.
        //
        // A TEXT TWIN OF THIS PIN, AS A RECIPE AND NOT AS A NUMBER, because a
        // future reader with a `grep` will land on 5516, 5512 or 5456 and think
        // this drifted. Three clauses reproduce it EXACTLY at both endpoints —
        // 5452 at `4812ac9c` and 5482 here:
        //
        //     drop lines matching ^[[:space:]]*॰      (margins)
        //     erase spans matching उक्तम्[^।]*इति     (string literals)
        //     count LINES, not occurrences
        //
        // The three conventions that miss are occurrences-not-lines (+60),
        // margins-left-in (+56) and literals-left-in (+4). THE LAST IS THE
        // INTERESTING ONE AND THIS FILE'S HEADER PREDICTED IT: the census lexes
        // with `sadhana::lex::lex_t1`, so it counts TOKENS, and the header names
        // `इतिशब्दः` — the global whose value is the word `इति` — as the case
        // that breaks a text scanner. The same failure with a different word is
        // `उक्तम् भवति इति`, seven times across `parse.t1` and `unparse.t1`.
        // Only FOUR sit on lines carrying no real `भवति`, which is why the
        // residue is four and not seven — `unparse.t1:442` holds a real one and
        // a literal on the same line. A LINE COUNT IS NOT AN APPROXIMATION OF A
        // LEXER; it is a different instrument that agrees except exactly where
        // text and tokens disagree.
        // (2794, 2688, 5482) -> (2794, 2703, 5497) on 2026-09-09, W-280. Taken
        // from this assertion's own failure in the `--full` at `146ccf2e`.
        //
        // **+15 ASSIGNMENT, +0 BINDING, AND THE FLAT COLUMN IS THE
        // CONFIRMATION.** The fifteen are this row's: 8 declarations
        // (`सङ्कलनविरामभेद` and its seven constants) plus 7 assignments (the
        // reset on entry and one per exit). **This census splits on the
        // statement's FIRST token**, and all fifteen begin with `सार्वजनिक` or
        // the global's name — never `चरः`. A GRAMMATICAL split would have put
        // the eight declarations in the binding column and 2794 would have
        // moved; it did not, which is what says the account is complete.
        //
        // THIS IS THE THIRD PIN ONE UNIT MOVED, and they fired in sequence
        // because each was only visible after the previous was re-taken: the
        // integer-prefix triple (1901 -> 1909), its conservation sum
        // (3312 -> 3320, predicted and exact), and this. **Three instruments
        // over one population; a red names what tripped FIRST, not what is
        // wrong.**
        // (2794, 2703, 5497) -> (2793, 2717, 5510) on 2026-09-10, W-279.
        // **BINDINGS FELL BY ONE WHILE THE TOTAL ROSE**, and that is the check
        // this pin exists for — the direction is not slack, it is the whole
        // signal. Decomposed:
        //
        //   bindings    -1   the local `चरः नामपूर्वम्`, DELETED. It read the
        //                    name cursor before `नामसञ्चयः` so the pass could
        //                    subtract; the subtraction spanned that routine's
        //                    reset-on-entry and produced a count of entries
        //                    written of −३९, so both it and the local went.
        //   assignments +14  8 module-scope `सार्वजनिक चरः` declarations, plus
        //                    the 7 assignments that record the exits and the two
        //                    name counts, MINUS the removed assignment that set
        //                    `सङ्कलनविरामभेद` to the routine-less exit, which
        //                    went with the routine guard. 8 + 7 − 1 = 14.
        //
        //                    THAT CONSTANT IS NOT NAMED IN FULL HERE, AND THE
        //                    RULE IS NARROWER THAN IT LOOKS. `public_names_
        //                    referenced_nowhere` STRIPS `॰` margins from the .t1
        //                    corpus — its own unit test pins that, "a margin
        //                    naming क does not count" — but the Rust-test text it
        //                    checks second is NOT stripped, and it excludes only
        //                    `t1_paradigm_names.rs`. So a .t1 margin may name it
        //                    freely and a comment in THIS file counts as a
        //                    reference, which would drop it from the adjudicated
        //                    list where it now belongs at in-degree 0.
        //
        //                    (Not the
        //                    withdrawn `सङ्कलनगणनाभङ्गभेद`: that was added and
        //                    removed inside this branch, so against main it is
        //                    net zero and is nobody's −1.)
        //                    This census splits on the statement's FIRST
        //                    token, so a `सार्वजनिक चरः` lands here and not in
        //                    bindings — which is why 8 declarations show up as
        //                    assignments and nothing was reclassified.
        //   total       +13  = -1 + 14, so the split accounts for the move
        //                    entirely and no site changed category.
        //
        // (2793, 2717, 5510) -> (2798, 2722, 5520) on 2026-09-11, W-279, TAKEN
        // FROM THIS ASSERTION'S OWN FAILURE. The entry above is exactly the
        // hazard the paragraph at the top of this block names: that pin was
        // re-taken mid-row on 2026-09-10 and then the row kept editing `ir.t1`.
        // This is the second taking, after the last source edit.
        //
        // +5 bindings, +5 assignments, +10 total, and the decomposition was
        // derived from `git diff 8c4748a4 HEAD -- '*.t1'` BEFORE the measured
        // tuple was read, not fitted to it. 14 added lines carry `भवति` and 3
        // removed ones do; 14 - 3 = 11, which is NOT the move. The line that
        // does not count is a `॰` margin quoting `संज्ञाघोषणाकोश[संज्ञा+१]
        // भवति ०` — the census strips margins, and `public_names_referenced_
        // nowhere`'s own unit test pins that it does. 13 - 3 = 10.
        //
        // Three of those pairs are the SAME LINE on both sides and are nobody's
        // delta: `अपूर्णहेतुसीमा २७ -> ४३` is one modification, and `वस्तुघोषणा`
        // and `क्षेत्रक्रमः` were re-indented into the new `अन्यथा` branch
        // without changing. So the true additions are 5 and 5:
        //
        //   bindings    +5   the field-resolution walk's locals — `सञ्चयक्रमः`,
        //                    `वस्तुप्रविष्टिः`, `दूरक्षेत्रसंख्या`, `दूरक्रमः`,
        //                    `दूरनामपाठ`. All five begin `चरः`.
        //   assignments +5   the walk's writes — the two cursor bumps, the entry
        //                    capture, and `क्षेत्रस्थानम्`/`क्षेत्रमिलितम्`.
        //                    All five begin with the name.
        //
        // AND IT RECONCILES ACROSS THE TWO CENSUSES, which is the check worth
        // more than either pin alone. `w268-census.rs` measures the GRAMMAR's
        // split and reads bindings 3390, assignments 2130, `भवति` 5520. The
        // totals agree at 5520, and 2798 + 592 = 3390 with 2722 - 592 = 2130,
        // where 592 is that census's `public-bindings` key. Two instruments
        // that split the same population differently, bridged by a third
        // number neither of them is free to choose.
        //
        // (2798, 2722, 5520) -> (2803, 2725, 5528) on 2026-09-11, `W-283`'s
        // ruled storage model, TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // +5 bindings, +3 assignments, +8 total, and the members are the whole
        // account — every `भवति` this row added:
        //
        //   assignments +3   THE THREE KIND CONSTANTS, `ir.t1`: `सार्वजनिक चरः
        //                    वैश्विकस्थानाज्ञाभेद`, `स्थानाहाराज्ञाभेद`,
        //                    `स्थाननिधानाज्ञाभेद`. They begin `सार्वजनिक`, so
        //                    THIS census does not reach the binding bucket.
        //   bindings    +5   the emitter's locals, `yantrotsarjana.t1`: `गन्तृ`
        //                    twice (AddrOfGlobal and LoadAt), `स्थानम्` twice
        //                    (LoadAt and StoreAt), `स्रोतः` once (StoreAt).
        //                    All five begin `चरः`.
        //
        // **AND THIS ROW DISCRIMINATES THE TWO CLASSIFICATIONS, WHERE THE ENTRY
        // ABOVE COULD NOT.** W-279 added no public binding, so its +5/+5 was
        // identical under both models and its margin says so. This row adds
        // THREE, and they land in opposite columns: assignments here, bindings
        // in `w268-census.rs`, whose `assignments` key does not move AT ALL
        // (2130 both sides) while its `bindings` takes all eight. A row that
        // moves one census's assignment column by +3 and the other's by 0 is
        // the discriminating case the `grammar-t1.ebnf` margin asks for.
        //
        // The bridge holds on the NEW value and that is the check: 2803 + 595 =
        // 3398 and 2725 - 595 = 2130, where `public-bindings` itself moved
        // 592 -> 595 — the same three constants, counted a third way.
        //
        // (2803, 2725, 5528) -> (2819, 2731, 5550) on 2026-09-11, `W-283`'s
        // FIRST LOWERING — a store to a module-level global — TAKEN FROM THIS
        // ASSERTION'S OWN FAILURE. +16 bindings, +6 assignments, +22 total,
        // derived from `git diff c90d4161 -- '*.t1'` before the values were read:
        //
        //   23 added `भवति` lines, 1 removed. The removal is `रचितशेषसीमा`
        //   २८ -> २९, a MODIFICATION, and it leads with `सार्वजनिक` so it sits
        //   in the ASSIGNMENT column of this census, not the binding one.
        //   17 of the 23 carry `चरः`, but ONE of those is the `सार्वजनिक चरः`
        //   replacement — which this census splits by FIRST token, so it is an
        //   assignment here. 17 - 1 = 16 bindings, 23 - 16 = 7 assignments
        //   added, minus the 1 removed = 6.
        //
        // THE `सार्वजनिक चरः` LINE IS COUNTED TWICE AND CANCELS TWICE, which is
        // why the naive `grep -c चरः` reads 17 and the answer is 16. A modified
        // line is an addition and a removal of the same kind; a `सार्वजनिक`
        // prefix moves it out of the bucket its `चरः` suggests.
        //
        // `public-bindings` is UNMOVED at 595 — +1 added, −1 removed, the same
        // line — which is what says this row added no module-level storage.
        //
        // (2819, 2731, 5550) -> (2855, 2753, 5608) ON 2026-09-12, TAKEN FROM
        // THIS ASSERTION'S OWN FAILURE, RE-TAKEN BY THE TRUNK. +36 bindings-first
        // and +22 assignments-first, +58 total, and the split still sums:
        // 2855 + 2753 = 5608 exactly, as 2819 + 2731 = 5550 did. The cause is
        // the storage model's three landings — `W-284`, `W-285`, `W-287` — the
        // same corpus growth that moved four pins in `sanskrit-text` and the
        // grammar's stated census in `w268-census`.
        //
        // **THIS WAS THE SECOND CRATE, AND THAT IS THE LESSON THE TRUNK PAID
        // FOR TWICE TODAY.** Four pins in `sanskrit-text` were re-taken at
        // `1ae64d15` and main was reported green on the strength of one test
        // binary in one crate. **A RED IS A LOWER BOUND ACROSS CRATES, NOT ONLY
        // WITHIN ONE** — the same corpus edit moved three more pins here and one
        // in `yantra`, and only a lane gating an unrelated crate found them.
        // (2855, 2753, 5608) -> (2859, 2758, 5617) ON 2026-09-12, `W-293`, and the
        // SPLIT STILL SUMS: 2859 + 2758 = 5617 exactly, as 2855 + 2753 = 5608 did.
        // That identity is the reason this pin is worth re-taking rather than
        // merely updating — a row that lost a site instead of adding one would
        // break the sum, and only the pair can show it.
        //
        // The cause is a `.t1` edit: global arrays gained storage, which added a
        // fourth arena, its computation, and a widened read arm to `ir.t1` plus an
        // emission branch to `यन्त्रोत्सर्जन`. **`ir.t1` IS IN THE CORPUS THIS
        // COUNTS**, so a lowering fix moves this pin by construction.
        // 2026-09-12, W-293: (2859, 2758, 5617) -> (2865, 2765, 5630) on the
        // merged tree at c4e77929. THE SUM IS THE CHECK: +6 bindings and +7
        // assignments is +13 `भवति`, and 2865 + 2765 = 5630 exactly. A move that
        // did not close here would mean a site changed its statement's FIRST
        // token — a reclassification — rather than the corpus growing.
        //
        // AND THIS PIN WAS INVISIBLE UNTIL THE ONE ABOVE WENT GREEN. The share
        // tuple failed first and the gate could name only that; re-taking it
        // exposed this. A RED IS A LOWER BOUND on what is broken, never the set —
        // which is what the margin on the pin above already said, from the last
        // time this pair did exactly this.
        //
        // The +6 bindings are the same +6 the ADR-0030 triple moved (`न६४`
        // 1976 -> 1982) and the same +6 `spec/grammar-t1.ebnf` now states for
        // `चरः`. Three pins, one delta, composed from +4 in `encode.t1` and +2
        // across `ir.t1` and `यन्त्रोत्सर्जन`.
        // 2026-09-13: (2865, 2765, 5630) -> (2868, 2767, 5635), TAKEN FROM THIS
        // ASSERTION'S OWN FAILURE on the trunk carrying TWO landings, and the
        // move decomposes to named members with nothing reclassified:
        //   `W-nil` (trunk, 5dd5def0)  +2 bindings  +0 assignments  +2 भवति
        //       `शून्यफलम्`, `शून्यावगणना` — the `शून्यम्` arm's two locals
        //   cause 22 (6e, 808580cf)     +1 binding   +2 assignments  +3 भवति
        //       `लक्ष्यलेख्यम्` and the two tests that set it
        //   3 + 2 = 5 is the whole of the total's move.
        // BOTH LANDINGS WERE CENSUSED WITH `paradigm_encode` AND NEITHER CENSUS
        // RUNS THIS CRATE, so the nil half sat red on origin/main until this
        // re-take — the second pin tonight to go red that way (the ADR-0030
        // ledger was the first). A `.t1` landing's gate is not its census.
        // Edited by anchored line, not by replacing the tuple: the same tuple
        // appears above as the `W-293` HISTORY entry and a blind replace hits it.
        // 2026-09-13: (2868, 2767, 5635) -> (2871, 2767, 5638), the gather PORTED
        // TO THE `.t1` DRIVER (`e58ec658`): +3 bindings, +0 assignments, +3 भवति —
        // `shrinkhala.t1`'s `घोषणाः`, `सङ्गृहीतम्`, `सञ्चयसिद्धम्`. 3 + 0 = 3 is the
        // whole of the total's move. TAKEN FROM THIS ASSERTION'S OWN FAILURE on
        // the trunk with the port merged. The port's diff shows five added `चरः`
        // lines; two are the nil arm's carried in by a merge of main and already
        // counted above — an edited or merged-in line is not a new binding.
        // 2026-09-13: (2871, 2767, 5638) -> (2881, 2768, 5649), `agent/task08`
        // parts 1–2 (`2a57d72f`): +10 bindings, +1 assignment, +11 भवति, and
        // 10 + 1 = 11 is the whole of the total's move. Eleven `चरः` lines added
        // less `अवगणनखण्ड`, an EDITED line — the same net the `.ebnf` census and
        // the ADR-0030 ledger read for the same unit. TAKEN FROM THIS ASSERTION'S
        // OWN FAILURE on the trunk with task08 merged; anchored on the assertion
        // line, not the value.
        // 2026-09-13: (2881, 2768, 5649) -> (2882, 2773, 5655), `W-294` (2d,
        // `8dd8f6be`, narrow element loads): +1 binding, +5 assignments, +6 भवति —
        // `चरः अष्टकविस्तार ॱॱ न६४` and its five `अष्टकविस्तार भवति ०/१/२/४/८`,
        // the width table. 1 + 5 = 6 is the whole of the total's move. 2d
        // measured (2868, 2770, 5638) on THEIR base `046f3cfe`; the trunk carried
        // (2881, 2768, 5649) through task08; neither is the merged value —
        // TAKEN FROM THIS ASSERTION'S OWN FAILURE on the merged tree. Anchored on
        // the assertion line; the old tuple is also history above.
        // 2026-09-13: (2882, 2773, 5655) -> (2891, 2788, 5679), the embed store
        // (trunk, `lex.t1`): +9 bindings, +15 assignments, +24 भवति, 9 + 15 = 24.
        // The `.ebnf` census reads the same edit as +13 `चरः` and +11 assignments
        // — also 24 — because this census classifies a `चरः … भवति ०` whose ० is
        // rewritten differently from the grammar's count; two instruments, two
        // partitions of one 24. TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // 2026-09-13, agent/runhdr: (2891, 2788, 5679) -> (2920, 2789, 5709): +29 bindings,
        // +1 assignment, +30 भवति — the run-header arms' own locals (न६४ / मूल्याङ्क) and
        // one moved line; TAKEN FROM THIS ASSERTION'S OWN FAILURE on the merged tree.
        // (2920, 2789, 5709) -> (2920, 2798, 5718) on 2026-09-13, agent/lexspeed: the parser's remembered (position, text) — two public globals and their stores; TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // (2920, 2798, 5718) -> (2926, 2800, 5726) on 2026-09-13, the वास्तु lowering half (the driver's registry-name loop); TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // (2926, 2800, 5726) -> (2928, 2804, 5732) on 2026-09-13, the वास्तु lowering half, store side (two locals in ir.t1's store arm); TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // (2928, 2804, 5732) -> (2931, 2811, 5742) on 2026-09-13, the self-image's entry (शृङ्खला: प्रवेशन्यासः, स्वपरीक्षा, four globals); MEASURED from this assertion's own failure.
        // (2931, 2811, 5742) -> (2953, 2841, 5794) on 2026-09-13, the encoder's table index (encode.t1 only): +52 `भवति` sites (67 added, 15 removed with the five rewritten walkers), of which +22 begin with `चरः` and +30 do not — the 13 `सार्वजनिक चरः` index globals land on the assignment side by this file's first-token rule, and the index builders' 17 arena stores are the rest. MEASURED from this assertion's own failure.
        // (2953, 2841, 5794) -> (2957, 2842, 5799) on 2026-09-13, मण्डलानिप्रतिबिम्बम् collects every source before compiling any; MEASURED from this assertion's own failure.
        (2957, 2842, 5799),
        // 2026-09-05, W-265: (2497, 2442, 4939) -> (2538, 2486, 5024), TAKEN
        // FROM THIS ASSERTION'S OWN FAILURE. 85 `भवति` sites added and none
        // removed, split 41 binding / 44 assignment by the `lhs.first()`
        // reading this file's label above describes — which is what a row that
        // ADDS a table and its readers looks like from here. `c.total` agrees
        // with `c.tokens` at the assertion above, so every one of the 85 was
        // classified rather than absorbed.
        // 2026-09-05, TWO CHANGES MET HERE AND BOTH NOTES ARE KEPT, because each
        // explains a different part of the movement and neither alone explains it.
        //
        // The trunk's red-main fix hoisted `प्रकारपाठविवराष्टकम्` out of a routine
        // body to module scope, turning a bare `चरः` into `सार्वजनिक चरः`; THIS
        // census reads `lhs.first()`, so that declaration crossed from the binding
        // side to the assignment side. `w268-census.rs` sees the same line as
        // `public-bindings` 516 -> 517 — one edit, two readings.
        //
        // W-266 made `सङ्कोचः`'s save five field copies where it had been one
        // binding, so the assignment side gained five and the binding side did not
        // move at all.
        //
        // The figures are TAKEN FROM THIS ASSERTION ON THE MERGED TREE, never by
        // adding the two lanes' numbers together.

        // 2026-09-05, W-248: (2489, 2416, 4905) -> (2498, 2436, 4934), taken
        // from this assertion when it failed on the merged tree, never
        // computed. W-248 added a text-to-type reader to `artha.t1`, so the
        // corpus grew and the partition grew with it. BOTH SIDES OF THIS
        // CONFLICT WERE RIGHT ABOUT DIFFERENT THINGS and both are kept: the
        // numbers are W-248's re-measurement, the LABEL below is W-268's
        // correction. W-268's figures (2489/2416) were measured before W-248
        // landed and are the same reading, one merge earlier.
        "`भवति` sites split by their statement's FIRST token — not the \
         grammar's binding/assignment split, which counts the \
         `सार्वजनिक चरः` sites as bindings and is guarded by \
         `w268-census.rs` against `spec/grammar-t1.ebnf`"
    );
    pin_report!(
        (c.both_written, c.either_written),
        // 48 -> 49 on 2026-09-07, `W-chain`, and `either_written` UNMOVED at
        // 70 — which is the whole story, because it says NO NEW SHARE was
        // added. This assertion's own instruction is "a new share, a new call,
        // or a bug in this census — say which". IT IS A FOURTH THING, and
        // naming it is more useful than picking one of the three:
        //
        // **THIS COUNT IS BY NAME, CORPUS-WIDE — as the margin at the
        // computation says it must be, since several shares put a GLOBAL on a
        // local and the global is written in another file. `shrinkhala.t1`
        // binds a LOCAL called `निर्णायकः` (the resolver it opens and hands to
        // the typechecker). `artha.t1:1598` and `:1634` share
        // `निर्णायकः ॱ परिसराः`, and the threshold is `t > 1`: the share
        // statement is itself one write, so one more anywhere in the corpus
        // tips it. My local supplied that write.**
        //
        // So the driver did not create a share and did not make an existing
        // one more dangerous — a local in a module that never touches
        // `परिसराः` cannot. The by-name counting is a deliberate
        // over-approximation, and this is the first recorded instance of it
        // firing ACROSS modules on an unrelated name. Left as measured rather
        // than renamed around: renaming a correct local to satisfy a counter
        // would hide the over-approximation instead of recording it.
        //
        // Only one of `artha.t1`'s two sites moved; the other already had a
        // second write and was in `both_written` before. I did not determine
        // which, and it does not change the reading.
        // 67 → 66 on 2026-09-05 (`W-266`), and `both_written` did NOT move —
        // which is worth reading twice. The share removed there was the FIRST
        // of these sites shown to be load-bearing by running it, and it was
        // one where only ONE of the two names is ever written (`पूर्वदोषः` was
        // read and never written; the global is written all over). So it sat
        // in the weaker of these two categories the whole time. `both_written`
        // is a hint about which sites to read first, not a filter: it would
        // have ranked the one live instance below forty-four others.
        // 2026-09-05, W-265: (44, 66) -> (47, 69), TAKEN FROM THIS ASSERTION'S
        // OWN FAILURE. The three new shares are `निर्णायकारम्भः`'s reset of the
        // type-name table's three integer runs, and each is written at BOTH
        // ends — the global by `प्रकारसंज्ञायोजनम्`, the local right here — so
        // all three land in the both-written half. That is the intended shape
        // and not an incidental alias: the reset EXISTS to put a fresh run
        // behind the global, and its own margin says why each needs its own.
        // 2026-09-06, W-254: (47, 69) -> (48, 70), TAKEN FROM THIS ASSERTION'S
        // OWN FAILURE. BOTH halves moved by one, which is the reading that
        // matters: the new share at `ir.t1:521` writes `आज्ञाकोश` and `नव`, and
        // BOTH names are written elsewhere in the corpus — `आज्ञाकोश` by every
        // other constructor in the file and `नव` by the four siblings doing the
        // same copy-modify-write. So it lands in `both_written` and not merely
        // in `either_written`, which is what says the share is load-bearing
        // rather than incidental. A new share that moved only `either_written`
        // would be one whose second name nothing else ever touches, and that is
        // the shape worth looking at twice.
        // (49, 70) -> (57, 78) on 2026-09-07, the object builder. BOTH MOVED
        // BY EIGHT, which is exactly this row's eight new shares — so every
        // one of them has both names written, the stronger of the two
        // categories.
        //
        // THAT IS INTENDED AND SAYING SO IS THE POINT OF THIS PIN. Each is an
        // arena the builder fills and then binds to a field of the object it
        // is building: `फलम्` is appended to and `वस्तु ॱ संज्ञाः` is set from
        // it, so both names are written by construction. AN OBJECT OWNS THE
        // ARENAS IT IS BUILT FROM — `वस्तुरचना` holds no second writer to any
        // of them after it returns, and nothing else has a reference to hand.
        //
        // Contrast the driver's entry in this figure, which moved
        // `both_written` WITHOUT moving shares: that was the by-name,
        // corpus-wide over-approximation firing across modules on an unrelated
        // local. Here the two move together, which is what a real share looks
        // like.
        // (57, 78) -> (58, 79) on 2026-09-08, THE DRIVER'S BACK HALF ALONE.
        // THE THIRD ASSERTION IN THIS TEST TO NEED RE-TAKING, each hidden
        // behind the one above it — a test stops at its first failing
        // assertion, so re-taking a pin can only ever expose the NEXT one, and
        // "the pin is re-taken" after one green would have been wrong twice.
        //
        // BOTH MOVED BY ONE. The single new share is `shrinkhala.t1:227`
        // `[वस्तुप्रतिबिम्बम्]` `वस्तूनि अङ्कः ० अन्तः भवति वस्तु`, and both of
        // its names are written elsewhere in the corpus, so it lands in
        // `both_written` and not merely in `either_written` — an arena filled
        // and then bound to the record being built, both names written BY
        // CONSTRUCTION, with no second writer to `वस्तूनि` after the routine
        // returns.
        //
        // A NEW SHARE THAT MOVED ONLY `either_written` WOULD BE THE ONE TO LOOK
        // AT TWICE: its second name would be one nothing else ever touches.
        // This is not that.
        //
        // (58, 79) -> (61, 82) on 2026-09-08, THE CORPUS WALK AND THE
        // `%pcrel_lo12` EMITTER. THE FIFTH ASSERTION IN THIS FILE TO NEED
        // RE-TAKING, and the note above predicted the shape without knowing the
        // number: each is hidden behind the one before it, and this branch
        // needed the share triple, the statement partition, and now this pair.
        //
        // BOTH MOVED BY THREE, MATCHING THE THREE NEW SHARES EXACTLY, which is
        // the check this pin exists for: `shrinkhala.t1:345` and `:358` are the
        // corpus walk's two object appends, and `encode.t1:5732` is the `ॱपाठ`
        // section symbol. All three are an arena filled and then bound — both
        // names written BY CONSTRUCTION — so all three land in `both_written`
        // and not merely in `either_written`.
        //
        // THE TWO MOVING TOGETHER BY THE SAME AMOUNT AS `shares` IS THE WHOLE
        // READING. Had `either_written` moved by three and `both_written` by
        // fewer, one of the new shares would have a second name nothing else in
        // the corpus writes — which is the case the note above says to look at
        // twice. It did not happen, and saying so is worth more than the number.
        // (61, 82) -> (61, 83) on 2026-09-09, THE SYMBOL-MAPPING ROW. Taken
        // from this assertion's own failure. ONE NEW SHARE, and the pair says
        // WHICH KIND it is: `either_written` +1, **`both_written` UNMOVED at
        // 61**.
        //
        // That is the census answering a question I had argued from the code:
        // `निर्णायकधारः भवति निर्णायकः` aliases the resolver record, and only
        // ONE of the two names is ever written elsewhere in the corpus. The
        // holder is written here and READ in `नामसञ्चयः`; nothing writes
        // through it. A share whose two names are both written is a two-way
        // alias where either side can surprise the other — that is what
        // `both_written` counts, and this is not one of them.
        //
        // I CLAIMED THE HOLDER WAS READ-ONLY FROM READING THE CODE. An
        // instrument that knows nothing about that argument agreed, by a route
        // I did not choose and could not have steered. That is worth more than
        // the re-take: the margin's safety claim now has a measurement behind
        // it rather than only a reading. If a later row ever writes through
        // `निर्णायकधारः`, THIS number moves to 62 and the claim fails loudly
        // instead of rotting quietly.
        // **61 -> 57 on 2026-09-11, AND `either_written` UNMOVED AT 83 — which
        // is again the whole story: no share was added or removed, four shares
        // changed which side of `both_written` they sit on.** The cause is the
        // eight-global rename, and it is the aliasing showing up in a census.
        //
        // `both_written` requires the target base to have MORE THAN ONE write.
        // Before the rename, six of the eight colliding names accumulated writes
        // from TWO MODULES against ONE flat key; splitting them split the counts:
        //
        //     आज्ञाकोश      2 -> 1      वाक्यकोश   2 -> 1     दोषकोश        2 -> 1
        //     वाक्यसूचकाङ्क  4 -> 1      पाठकोष्ठकम् 2 -> 1     दत्तकोष्ठकम्   2 -> 1
        //     (आज्ञासूचकाङ्क 8 -> 5 and दोषसूचकाङ्क 6 -> 3 fell but stayed above 1)
        //
        // Six names crossed the threshold; FOUR shares have one of them as their
        // target base, so the count falls by four. **The census was counting one
        // module's writes as another's — the same defect the rename removed, seen
        // from a third instrument.** Measured member-wise against the pre-rename
        // tree, not taken from this assertion's failure.
        // 2026-09-12, W-293: (57, 83) -> (58, 84) at c4e77929. The one added
        // share (`encode.t1:5973`) has BOTH its names written elsewhere in the
        // corpus, so it moves both counters together — which is what says it is
        // load-bearing rather than incidental, and is the same answer its text
        // twin at `encode.t1:5902` gives.
        //
        // THIRD TUPLE IN THIS FILE TO MOVE FOR ONE SHARE, and each was invisible
        // until the one before it went green. Re-taking to a FIXPOINT is the only
        // safe procedure here; stopping at the first clean run would have left
        // this one red.
        // 2026-09-13, task08 parts 1–2 (`2a57d72f`): (58, 84) -> (59, 85). ONE new
        // share, and it is BOTH-WRITTEN — the class this pin exists to watch,
        // because a write through either alias is visible through the other.
        // NAMED BY THE UNIT'S OWNER: `ir.t1:2594`, `चरमूल्यम् भवति खण्डाधारः ।` — the
        // last line of part 1, the variable taking the post-header base. An
        // ASSIGNMENT, not a binding, which is why a `चरः`-shaped grep could not
        // find it: `:443-448` strips a leading `चरः` OPTIONALLY, so plain
        // assignments are sites too. Every clause checked: rhs a bare name,
        // rhs shape `मूल्याङ्क` (a record, `ir.t1:303`), target written 5 times,
        // rhs once.
        //
        // BOTH-WRITTEN BY NAME, WRITE-ONCE BY PATH. This pin counts writes to the
        // NAME corpus-wide — deliberately, so a global put on a local and written
        // in another file stays visible. Here the five writes to `चरमूल्यम्` sit
        // in mutually exclusive declaration arms below block (2497, 2622); on the
        // run arm (2557, 2601) where the alias exists, `खण्डाधारः` is written once
        // and `चरमूल्यम्` never after `:2594`. No write passes through this alias.
        // The pin is right to count it and the reading is right to say why it
        // is harmless — a classification can be right and the population behind
        // it be something else. MEASURED from this assertion's failure on the
        // trunk with task08 merged; `either_written` +1 with it, nothing reclassified.
        // 2026-09-13, the embed store (trunk, `lex.t1`): (59, 85) -> (60, 85).
        // `either_written` UNMOVED, so NO new share — an EXISTING share whose rhs
        // is `चिह्नककोश` became both-written by name, because `पदविभाग`'s embed
        // collapse now rewrites `चिह्नककोश` elements in place (the collapsed token
        // and the shifted tail). Both-written by name, in-place by design: the
        // arena's own lexer writing its own arena in the routine that fills it.
        // MEASURED from this assertion's failure.
        (60, 85),
        "of the 70 shares, 47 have BOTH names written elsewhere in the corpus \
         and 69 have at least one — which is what says the share is load-bearing \
         and not incidental"
    );
}
