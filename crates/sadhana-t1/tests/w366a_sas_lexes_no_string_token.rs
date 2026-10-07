//! **W-366 (a) — A RUN-TIME WITNESS THAT NO `.sas` TEXT LEXES TO `शब्दभेद`.**
//!
//! # The divergence this file watches
//!
//! ADR-0017 (`docs/adr/0017-a-string-literal-is-a-token.md`, "T0 is not
//! changed"): Rust's `lex` keeps its token stream to the byte and never makes a
//! `Kind::Str`; only `lex_t1` takes the string. The `.t1` lexer
//! `पदविभागॱपदविभाग` is ONE routine and it DOES make `शब्दभेद` (Kind::Str) — at
//! the ADR-0018 layout-word arm (a bare `विवरम्` or `यतिः`) and at the embed
//! collapse (`समावेशः आरभ्य <name> समाप्तम्`). And the T0 reader calls that same
//! routine on T0 text: `वाक्यविभागॱसङ्कलनम्` lexes the EMITTED `.sas` through
//! `पदविभागॱपदविभाग` (`w304-pass-zero.rs` pins that premise).
//!
//! So the two T0 lexers disagree on any `.sas` that holds one of those words,
//! and nothing is red today only because no `.sas` in the tree happens to hold
//! one. `t1_sources.rs`'s `t0_lexer_kind_sites` is a TEXT count — it proves the
//! assignment is written, not whether it is reached on T0 input. This file is
//! the run-time half the row asks for: the `.sas` corpus and the compiler's own
//! emitted `.sas`, lexed through the `.t1` lexer, with the kinds OBSERVED.
//!
//! It does not decide the language question (part (b) of W-366 does: a T0
//! entry that never makes kind ३). It turns the latent divergence into a red
//! the day a `.sas` reaches it.
//!
//! # Since part (b), 2026-10-04
//!
//! `lex.t1` now has Rust's two entries over one walk: `पदविभागॱपदविभाग` is
//! `lex_t1` and `पदविभागॱवाक्यपदविभाग` is `lex`, and the T0 reader calls the
//! latter. So the two sweeps lex through the T0 ENTRY (the text the T0 reader
//! really lexes, through the routine it really calls), the T1 controls lex
//! through the T1 entry, and `the_t0_entry_makes_no_string_token_where_the_t1_
//! entry_does` holds the rule on the very inputs that make kind ३ in T1.
//!
//! # Why the controls are not optional
//!
//! "Zero tokens of kind K" is satisfied by a harness that returns no tokens at
//! all, by a K read wrongly, and by a corpus walk that found no files. So:
//! the corpus token total must be non-zero and every file must lex to tokens;
//! K is READ (the interpreter's global and the source declaration, which must
//! agree), never typed; and the SAME counting function the witness asserts
//! with must report kind-K tokens on T1 text that legitimately makes them, and
//! on a scratch `.sas` built to reach the arm (the red-first self-check).
//!
//! The harness is `sas011-lex-word-class-order.rs`'s: `lex.t1` loaded alone
//! (it imports nothing), `पदविभागॱपदविभाग` called, the `चिह्नककोश` arena read.
//! One difference, on purpose: this reads the arena only up to the count the
//! call returned, because the arena keeps stale tokens past the cursor from the
//! previous text and the embed collapse lowers the cursor without shrinking it.
//! The emitted text comes from `t1_corpus_globals.rs`'s driver — a fresh
//! `Front` per source, `riscv64::emit_module` at the back.

use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use sadhana::t1::riscv64;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The `.t1` name of Kind::Str, as `lex.t1` declares it. The NUMBER is read.
const STR_KIND_NAME: &str = "शब्दभेद";

/// A cap, not a measurement: `spec/schedule.sas` is the largest text lexed here.
const FUEL_LEX: u64 = 1_000_000_000_000;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn spec_root() -> PathBuf {
    crate_dir().join("../../spec")
}

fn source(name: &str) -> String {
    let p = crate_dir().join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// `lex.t1` alone, loaded against the real `spec/` so the host fills the embed
/// store exactly as it does on every driver.
fn load_lexer() -> Interpreter {
    let text = source("lex.t1");
    Interpreter::load(&[("lex.t1", text.as_str())], &spec_root())
        .unwrap_or_else(|e| panic!("lex.t1 load: {e:?}"))
}

/// A Devanagari numeral as a number; `None` for anything else.
fn devanagari_int(s: &str) -> Option<i128> {
    if s.is_empty() {
        return None;
    }
    s.chars().try_fold(0i128, |n, c| {
        let d = (c as u32).checked_sub(0x0966).filter(|d| *d <= 9)?;
        Some(n * 10 + i128::from(d as u8))
    })
}

/// Kind::Str's number, READ TWICE and required to agree: the global the loaded
/// interpreter holds, and the declaration `सार्वजनिक चरः शब्दभेद ॱॱ न६४ भवति N ।`
/// in `lex.t1`'s text.
fn str_kind(it: &Interpreter) -> i128 {
    let loaded = it
        .global(STR_KIND_NAME)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("lex.t1 no longer declares the global {STR_KIND_NAME}"));
    let prefix = format!("सार्वजनिक चरः {STR_KIND_NAME} ॱॱ न६४ भवति ");
    let text = source("lex.t1");
    let declared: Vec<i128> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix(prefix.as_str()))
        .filter_map(|rest| rest.split_whitespace().next().and_then(devanagari_int))
        .collect();
    assert_eq!(
        declared,
        vec![loaded],
        "lex.t1 declares {STR_KIND_NAME} once, as the number the interpreter holds"
    );
    loaded
}

/// One token: its kind, its line, and its text (sliced from `पाठ` by its own
/// bounds — an embed token's `पाठ` is the table it carries).
struct Token {
    kind: i128,
    line: i128,
    text: String,
}

/// The T1 entry, Rust's `lex_t1`.
const T1_ENTRY: &str = "पदविभागॱपदविभाग";
/// The T0 entry, Rust's `lex` — what `वाक्यविभागॱसङ्कलनम्` calls (W-366 b).
const T0_ENTRY: &str = "पदविभागॱवाक्यपदविभाग";

/// Lex `src` through `entry` and answer the tokens it WROTE — arena
/// slots १..=count, where count is the call's own answer and must equal the
/// cursor. Slot ० is never written.
fn lex(it: &mut Interpreter, entry: &str, src: &str) -> Vec<Token> {
    let count = it
        .call(
            entry,
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            FUEL_LEX,
        )
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("पदविभाग answers a count");
    let cursor = it
        .global("चिह्नकसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("lex.t1 declares its cursor");
    assert_eq!(count, cursor, "the count पदविभाग answers is its cursor");
    let count = usize::try_from(count).expect("a count is not negative");
    let arena = match it.global("चिह्नककोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("global `चिह्नककोश` is {other:?}, not an arena — THIS HARNESS is broken"),
    };
    assert!(
        arena.len() > count,
        "the arena holds slots ०..={count}; it has {}",
        arena.len()
    );
    arena[1..=count]
        .iter()
        .map(|v| {
            let Value::Record(r) = v else {
                panic!("a written token slot holds {v:?}, not a record");
            };
            let r: HashMap<String, Value> = r.borrow().clone();
            let int = |f: &str| r.get(f).and_then(Value::as_int).unwrap_or(-1);
            let text = match r.get("पाठ") {
                Some(Value::Octets(o)) => {
                    let s = o.as_slice();
                    let (lo, hi) = (int("अष्टक"), int("पाठसीमा"));
                    match (usize::try_from(lo), usize::try_from(hi)) {
                        (Ok(lo), Ok(hi)) if lo <= hi && hi <= s.len() => {
                            String::from_utf8_lossy(&s[lo..hi]).into_owned()
                        }
                        _ => String::new(),
                    }
                }
                _ => String::new(),
            };
            Token {
                kind: int("भेद"),
                line: int("पङ्क्ति"),
                text,
            }
        })
        .collect()
}

/// THE WITNESS. `(tokens lexed, the kind-K tokens as "line N: text")`. Every
/// test below asserts through this one function, the controls included, so a
/// control that passes is evidence about the witness and not about a copy.
fn witness(it: &mut Interpreter, entry: &str, k: i128, src: &str) -> (usize, Vec<String>) {
    let toks = lex(it, entry, src);
    let hits = toks
        .iter()
        .filter(|t| t.kind == k)
        .map(|t| {
            let shown: String = t.text.chars().take(40).collect();
            format!("line {}: `{shown}`", t.line)
        })
        .collect();
    (toks.len(), hits)
}

/// Every `.sas` under `spec/`, recursively (`spec/*.sas` and `spec/golden/`),
/// in path order — READ OFF THE DIRECTORY, never listed by hand.
fn sas_paths() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).expect("spec/ is readable").flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "sas") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&spec_root(), &mut v);
    v.sort();
    v
}

/// THE FIRST HALF: every hand-written `.sas` in `spec/`.
#[test]
fn no_spec_sas_lexes_to_a_string_token_through_the_t1_lexer() {
    let mut it = load_lexer();
    let k = str_kind(&it);
    let paths = sas_paths();
    let top_level = paths
        .iter()
        .filter(|p| p.parent() == Some(spec_root().as_path()))
        .count();
    assert!(
        top_level > 0 && paths.len() > top_level,
        "the walk found {} .sas files, {top_level} directly in spec/ — it must find \
         both spec/*.sas and the nested ones",
        paths.len()
    );

    let mut total = 0usize;
    let mut offenders = Vec::new();
    for p in &paths {
        let src = std::fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
        let (n, hits) = witness(&mut it, T0_ENTRY, k, &src);
        assert!(
            n > 0,
            "{} lexed to no tokens; the harness is blind",
            p.display()
        );
        total += n;
        offenders.extend(hits.into_iter().map(|h| format!("{} {h}", p.display())));
    }
    println!(
        "METRIC w366a_spec_sas_files {} tokens {total} kind{k}_tokens {}",
        paths.len(),
        offenders.len()
    );
    assert!(total > 0, "the .sas corpus lexed to no tokens at all");
    assert!(
        offenders.is_empty(),
        "{} token(s) of kind {k} ({STR_KIND_NAME}, Kind::Str) in T0 text lexed by the .t1 \
         lexer. Rust's `lex` never makes Kind::Str (ADR-0017: T0 is not changed), so the two \
         T0 lexers now DISAGREE on this text — W-366 part (b), the T0 entry, is due:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// THE SECOND HALF: the `.sas` the compiler itself emits for every corpus
/// source — the text `वाक्यविभागॱसङ्कलनम्` actually lexes.
///
/// The driver is `t1_corpus_globals.rs`'s (a fresh `Front` per source, the
/// lenient Rust back end `riscv64::emit_module`). The `.t1` twin emitter makes
/// these same lines from the same arena; running this sweep through it is the
/// gap this file leaves, as that one does.
#[test]
fn no_emitted_sas_lexes_to_a_string_token_through_the_t1_lexer() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(crate_dir().join("src"))
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "the corpus is not empty");

    let mut it = load_lexer();
    let k = str_kind(&it);
    let (mut objects, mut total) = (0usize, 0usize);
    let mut no_module: Vec<String> = Vec::new();
    let mut refused = Vec::new();
    let mut offenders = Vec::new();
    for p in &paths {
        let src = std::fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
        let Some(module) = module_name(&src) else {
            no_module.push(
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            continue;
        };
        let mut front = Front::load(&spec_root()).expect("the front end loads");
        let text = (|| -> Result<String, String> {
            front.lex(&src)?;
            front.parse()?;
            front.resolve()?;
            front.typecheck()?;
            front.build_ir()?;
            let module = front.module(&module, None)?;
            riscv64::emit_module(&module).map_err(|e| format!("{e:?}"))
        })();
        let text = match text {
            Ok(t) => t,
            Err(e) => {
                refused.push(format!("{}: {e}", p.display()));
                continue;
            }
        };
        let (n, hits) = witness(&mut it, T0_ENTRY, k, &text);
        assert!(
            n > 0,
            "the emitted text of {} lexed to no tokens",
            p.display()
        );
        objects += 1;
        total += n;
        offenders.extend(
            hits.into_iter()
                .map(|h| format!("{} (emitted) {h}", p.display())),
        );
    }
    println!(
        "METRIC w366a_emitted_sas_objects {objects} tokens {total} kind{k}_tokens {} \
         no_module {} refused {}",
        offenders.len(),
        no_module.len(),
        refused.len()
    );
    for r in &refused {
        println!("REFUSED {r}");
    }
    // FULL ACCOUNTING, copied from the driver this sweep reuses
    // (t1_corpus_globals.rs): a refused or skipped source would otherwise leave
    // this green on a subset of the corpus (found in review).
    assert!(
        refused.is_empty(),
        "every source that declares a module must reach the emitter; these did not, so \
         the witness is silent about them:\n  {}",
        refused.join("\n  ")
    );
    assert_eq!(
        no_module,
        vec!["lib.t1".to_string()],
        "`lib.t1` is the ONE source that declares no module; a second is a source this \
         sweep silently skipped"
    );
    assert_eq!(
        objects + no_module.len(),
        paths.len(),
        "every source is accounted for in exactly one state"
    );
    assert!(
        objects > 0 && total > 0,
        "no corpus source emitted any lexed .sas ({} refused); the sweep is vacuous",
        refused.len()
    );
    assert!(
        offenders.is_empty(),
        "{} token(s) of kind {k} ({STR_KIND_NAME}, Kind::Str) in EMITTED T0 text lexed by \
         the .t1 lexer — the text the T0 reader really lexes. Rust's `lex` never makes \
         Kind::Str (ADR-0017), so the two T0 lexers now DISAGREE — W-366 part (b) is due:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// THE POSITIVE CONTROL, AND THE RED-FIRST SELF-CHECK — through `witness`.
///
/// 1. T1 text with the bare ADR-0018 layout words makes kind K, and the quoted
///    one does not (`sas011`'s case that must stay refused).
/// 2. The embed collapse makes kind K, with a table name READ from the store
///    the host filled — the second production site (`lex.t1`'s literal `३`).
/// 3. A scratch `.sas` holding a bare layout word as a data operand — exactly
///    the shape that would put T0 text on that arm — is reported by the witness,
///    so the corpus assertions above WOULD go red on it.
#[test]
fn the_witness_sees_kind_str_where_the_lexer_makes_it() {
    let mut it = load_lexer();
    let k = str_kind(&it);

    // (1) the layout words, bare and quoted.
    let t1 = "मण्डलम् क ॥\n\
              सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
              चरः अ ॱॱ अङ्कः अन्तः अ८ भवति विवरम् ।\n\
              चरः ब ॱॱ अङ्कः अन्तः अ८ भवति यतिः ।\n\
              चरः स ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् विवरम् इति ।\n\
              प्रत्यागमनम् ० ।\n\
              इति\n";
    let (n, hits) = witness(&mut it, T1_ENTRY, k, t1);
    println!("CONTROL layout words: {n} tokens, kind {k}: {hits:?}");
    assert!(n > 20, "the T1 control lexed to {n} tokens");
    assert_eq!(
        hits,
        vec!["line 3: `विवरम्`".to_string(), "line 4: `यतिः`".to_string()],
        "the two BARE layout words are kind {k}, and the quoted `विवरम्` is not"
    );

    // (2) the embed collapse, against a name the host actually stored.
    let stored = it
        .global("समावेशसंख्या")
        .and_then(Value::as_int)
        .expect("lex.t1 declares the embed count");
    assert!(stored > 0, "the host filled no embed table from spec/");
    let name = match it.global("समावेशनामकोश") {
        Some(Value::Arena(a)) => match a.borrow().first() {
            Some(Value::Octets(o)) => String::from_utf8(o.as_slice().to_vec()).expect("UTF-8"),
            other => panic!("the first stored name is {other:?}"),
        },
        other => panic!("`समावेशनामकोश` is {other:?}"),
    };
    let embed = format!(
        "मण्डलम् क ॥\n\
         सार्वजनिक चरः त ॱॱ अङ्कः अन्तः अ८ भवति समावेशः आरभ्य {name} समाप्तम् ।\n"
    );
    let (n, hits) = witness(&mut it, T1_ENTRY, k, &embed);
    println!(
        "CONTROL embed `{name}`: {n} tokens, kind {k}: {}",
        hits.len()
    );
    assert_eq!(hits.len(), 1, "one embed collapses to ONE kind-{k} token");

    // (3) RED FIRST: a .sas the corpus does not hold, built to reach the arm.
    let scratch = "॥ कोष्ठकम् ॱदत्त ॥\n\
                   अन्तरम्ॱॱ\n\
                   ॥ अष्टकाः विवरम् ॥\n";
    let (n, hits) = witness(&mut it, T1_ENTRY, k, scratch);
    println!("RED-FIRST scratch .sas: {n} tokens, kind {k}: {hits:?}");
    assert!(n > 0, "the scratch .sas lexed to tokens");
    assert_eq!(
        hits,
        vec!["line 3: `विवरम्`".to_string()],
        "a .sas holding a bare layout word must be REPORTED by the witness — this is the \
         input on which the corpus assertions above go red"
    );
}

/// W-366 (b): THE T0 ENTRY ON THE INPUTS THAT MAKE KIND ३ IN T1.
///
/// The same three inputs as the control above — the bare layout words, an
/// embed of a table the host stored, the scratch `.sas` — through the T0
/// entry: no kind-३ token, and the stream otherwise intact. Each input is
/// lexed through BOTH entries in this one test, so the zero cannot come from a
/// harness that sees nothing: the T1 entry's hits on the same text are counted
/// first and must be non-zero. Rust's `lex` is the reference for the shapes —
/// the layout word stays a word, the embed stays four words.
#[test]
fn the_t0_entry_makes_no_string_token_where_the_t1_entry_does() {
    let mut it = load_lexer();
    let k = str_kind(&it);
    let name = match it.global("समावेशनामकोश") {
        Some(Value::Arena(a)) => match a.borrow().first() {
            Some(Value::Octets(o)) => String::from_utf8(o.as_slice().to_vec()).expect("UTF-8"),
            other => panic!("the first stored name is {other:?}"),
        },
        other => panic!("`समावेशनामकोश` is {other:?}"),
    };
    let layout = "मण्डलम् क ॥\n\
                  सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
                  चरः अ ॱॱ अङ्कः अन्तः अ८ भवति विवरम् ।\n\
                  चरः ब ॱॱ अङ्कः अन्तः अ८ भवति यतिः ।\n\
                  प्रत्यागमनम् ० ।\n\
                  इति\n";
    let embed = format!(
        "मण्डलम् क ॥\n\
         सार्वजनिक चरः त ॱॱ अङ्कः अन्तः अ८ भवति समावेशः आरभ्य {name} समाप्तम् ।\n"
    );
    let scratch = "॥ कोष्ठकम् ॱदत्त ॥\n\
                   अन्तरम्ॱॱ\n\
                   ॥ अष्टकाः विवरम् ॥\n";
    // (input, T1 hits, how many MORE tokens T0 writes than T1)
    for (label, src, t1_hits, extra) in [
        ("layout words", layout, 2usize, 0usize),
        ("embed", embed.as_str(), 1, 3),
        ("scratch .sas", scratch, 1, 0),
    ] {
        let (n1, h1) = witness(&mut it, T1_ENTRY, k, src);
        let (n0, h0) = witness(&mut it, T0_ENTRY, k, src);
        println!(
            "W366B {label}: T1 {n1} tokens {} kind{k}; T0 {n0} tokens {} kind{k}",
            h1.len(),
            h0.len()
        );
        assert_eq!(
            h1.len(),
            t1_hits,
            "{label}: the T1 entry makes kind {k} here (the control)"
        );
        assert!(
            h0.is_empty(),
            "{label}: the T0 entry made kind {k} ({STR_KIND_NAME}) — Rust's `lex` never \
             does (ADR-0017 :206):\n{}",
            h0.join("\n")
        );
        assert_eq!(
            n0,
            n1 + extra,
            "{label}: the T0 stream is the T1 stream, un-collapsed"
        );
    }

    // And the T0 entry's word for the layout word is the WORD, kind पदभेद.
    let word_kind = it
        .global("पदभेद")
        .and_then(Value::as_int)
        .expect("lex.t1 declares पदभेद");
    let toks = lex(&mut it, T0_ENTRY, scratch);
    let w = toks
        .iter()
        .find(|t| t.text == "विवरम्")
        .expect("the T0 entry keeps the layout word as a token");
    assert_eq!(
        w.kind, word_kind,
        "on T0 text `विवरम्` is a word, as in Rust's `lex`"
    );
}
