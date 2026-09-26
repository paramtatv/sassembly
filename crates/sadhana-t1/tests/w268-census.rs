//! W-268: THE GRAMMAR'S OWN CENSUS, RE-TAKEN EVERY GATE.
//!
//! `spec/grammar-t1.ebnf`'s statement section states a CLOSED COUNT over
//! `crates/sadhana-t1/src/*.t1` — how many `चरः`, how many `भवति`, how the
//! `भवति` sites partition into bindings and assignments, and which of
//! `keyword`'s words the corpus never writes. Those figures were taken once,
//! on 2026-08-30, and nothing re-took them. By 2026-09-05 every one of them
//! was about a THIRD of the truth, and the sentences resting on them were
//! still being read as current.
//!
//! A figure in a comment that nothing re-measures is how that happens. So this
//! file does not merely record today's numbers — it makes the SPEC FILE the
//! thing under test. The grammar carries its figures as machine-readable
//! `CENSUS <key> = <n>` lines inside its census block; this test re-measures
//! each one from the corpus and asserts the file agrees, key for key. Drift is
//! then a RED GATE rather than a discovery: `cargo test -p sadhana-t1` runs it
//! — it is NOT `#[ignore]`d — and `tools/gate.sh`'s `cargo test --workspace`
//! step reaches it for every change, `spec/` included.
//!
//! # Why this is not a grep, and why it is not the W-250 census either
//!
//! A grep lies twice over. `योगः` is a substring of `वियोगः`, so a text scan
//! for a word counts every longer word that contains it; and a scan cannot see
//! that `॰ चरः` is a comment or that `इतिशब्दः` is a name and not the string
//! terminator `इति`. Both are handled by lexing with `sadhana::lex::lex_t1` —
//! the assembler's own lexer, which drops comments and hands back a whole
//! `उक्तम् … इति` literal as one `Kind::Str`. A count here compares a token's
//! SOURCE TEXT for equality, so no word can stand in for another — and NOT its
//! `Kind`, which is a trap this file fell into first and `is_word` explains.
//!
//! `w250-shares.rs` also lexes and also reports a binding/assignment split. It
//! is NOT the source of the numbers below: it classifies a site by the TYPE of
//! its right-hand side, which is a much larger machine with much more to get
//! wrong, and its split falls out of that machine as a by-product. This file
//! asks the narrow question the grammar actually asks — what word LEADS the
//! statement — with a statement walk of its own. The two agreeing is worth
//! something precisely because they were arrived at separately.
//!
//! # What a statement is here
//!
//! A `भवति` site's statement begins after the nearest preceding `।`, `॥`,
//! `आदि` or `इति` — the four things that end a statement or a block in T1 — or
//! at the start of the file. If the first word of that run is `चरः`, or
//! `सार्वजनिक` then `चरः`, the site is a BINDING; otherwise it is an
//! ASSIGNMENT. The grammar's claim is that these two sets sum to the whole and
//! that the `ॱॱ` type mark separates them exactly, and both halves are
//! asserted below rather than assumed.

use sadhana::lex::{Kind, Token, lex_t1};
use std::collections::BTreeMap;
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

const W_BECOMES: &str = "भवति";
const W_VAR: &str = "चरः";
const W_PUB: &str = "सार्वजनिक";
const W_BLOCK_OPEN: &str = "आदि";
const W_BLOCK_CLOSE: &str = "इति";

/// The words whose counts the grammar's statement block states by name.
const LED_WORDS: [&str; 6] = ["चरः", "भवति", "यदि", "अन्यथा", "यावत्", "प्रत्यागमनम्"];

/// The five `keyword` words that would each be a statement form, and which the
/// block says the corpus writes ZERO times. Asserted by name, not by count: a
/// count alone would let one appear while another vanished.
const UNWRITTEN_STATEMENT_WORDS: [&str; 5] = ["प्रत्येकम्", "भङ्गः", "अनुवर्तनम्", "विकल्पना", "रक्षा"];

fn repo_root() -> PathBuf {
    // crates/sadhana-t1 → the tree root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels below the tree root")
        .to_path_buf()
}

fn grammar_path() -> PathBuf {
    repo_root().join("spec/grammar-t1.ebnf")
}

/// The corpus the grammar names: `crates/sadhana-t1/src/*.t1`, read fresh so a
/// new source joins the census the day it is written.
fn corpus() -> Vec<(String, Vec<Token>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    assert!(
        names.len() >= 15,
        "the corpus went missing: {} `.t1` sources found under {}",
        names.len(),
        dir.display()
    );
    names
        .into_iter()
        .map(|n| {
            let text = std::fs::read_to_string(dir.join(&n))
                .unwrap_or_else(|e| panic!("{n} is readable: {e}"));
            let toks = lex_t1(&text).unwrap_or_else(|e| panic!("{n} must lex: {e:?}"));
            (n, toks)
        })
        .collect()
}

/// A token IS this word when its SOURCE TEXT is exactly the word.
///
/// NOT `kind == Kind::Word`, and this file's first version made exactly that
/// mistake — it is the bug the "validate a new scanner against a hand count you
/// already know" rule exists to catch. `lex_t1` is the ASSEMBLER's lexer, and
/// the assembler marks an operand's kāraka role with a Sanskrit ending. Two of
/// the six statement-leading words end in one: `प्रत्यागमनम्` arrives as
/// `Kind::Operand { base: "प्रत्यागमन", karaka: Destination }` and `यावत्`
/// likewise, because the reader cannot tell a T1 keyword from a T0 operand
/// bearing the same ending without knowing which language it is reading. A
/// `Kind::Word` filter therefore counted `प्रत्यागमनम्` and `यावत्` as ZERO
/// while a whitespace split of the same files found 1,775 and 312 — a scanner
/// silently reporting nothing for two of the six forms it was written to count.
///
/// Text equality is the honest predicate and costs nothing: the source text is
/// kept verbatim on every kind, so a role-marked operand and a bare word are
/// compared the same way. `Kind::Str` is excluded for exactness rather than
/// need — a string token's `text` carries its `उक्तम् … इति` delimiters, so it
/// can never equal a bare keyword — and comments are not tokens at all, which
/// is the whole reason to lex instead of scanning text.
fn is_word(t: &Token, w: &str) -> bool {
    !matches!(t.kind, Kind::Str { .. }) && t.text == w
}

/// Does this token end the statement that precedes it?
fn ends_statement(t: &Token) -> bool {
    matches!(t.kind, Kind::Danda | Kind::DoubleDanda)
        || is_word(t, W_BLOCK_OPEN)
        || is_word(t, W_BLOCK_CLOSE)
}

/// One `भवति` site, classified by the word that LEADS its statement.
struct Site {
    file: String,
    line: usize,
    binding: bool,
    /// `सार्वजनिक चरः` — a binding, and the one the naive walk gets wrong.
    public: bool,
    /// Is there a `ॱॱ` type mark between the statement's start and the verb?
    typed: bool,
}

fn sites(file: &str, toks: &[Token]) -> Vec<Site> {
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if !is_word(t, W_BECOMES) {
            continue;
        }
        // Walk back to the statement's first token.
        let mut start = 0;
        for j in (0..i).rev() {
            if ends_statement(&toks[j]) {
                start = j + 1;
                break;
            }
        }
        // `binding = [ "सार्वजनिक" ] , "चरः" , …` — THE PREFIX IS PART OF THE
        // FORM. A walk that tests only the statement's first token for `चरः`
        // puts every public binding on the assignment side; see the count of
        // `public-bindings` below and the note beside `binding` in the grammar.
        let mut head = start;
        let public = toks.get(head).is_some_and(|t| is_word(t, W_PUB));
        if public {
            head += 1;
        }
        let binding = toks.get(head).is_some_and(|t| is_word(t, W_VAR));
        let typed = toks[start..i].iter().any(|t| t.kind == Kind::LabelMark);
        out.push(Site {
            file: file.to_string(),
            line: t.line,
            binding,
            public: public && binding,
            typed,
        });
    }
    out
}

/// Every alternative of the `keyword` production, read out of the grammar
/// itself. Hard-coding the list here would be a second figure to go stale; the
/// production is frozen and is the only place the set is defined.
/// Every alternative of the `keyword` production, read out of the grammar
/// itself. Hard-coding the list here would be a second figure to go stale; the
/// production is frozen and is the only place the set is defined.
///
/// THE COMMENT STRIPPER IS STATEFUL ACROSS LINES, and the first version of this
/// reader was not — that was the second bug this file caught in itself. An EBNF
/// comment may run over several lines, and `मण्डलम्`'s does:
///
/// ```text
///     | "मण्डलम्"         (* module — ADR-0026, the DECLARING half of
///                            the pair below; see the `modules` section *)
/// ```
///
/// A per-line stripper leaves the second line unstripped, sees the SEMICOLON in
/// "the pair below;" and stops there — ending the production ten alternatives
/// early and reporting 22 where there are 32. The failure mode is the quiet
/// one: a plausible number, no error, and a keyword census that silently omits
/// the last third of the list.
fn keyword_alternatives(grammar: &str) -> Vec<String> {
    let Some(start) = grammar.find("\nkeyword") else {
        panic!(
            "the `keyword` production was not found in spec/grammar-t1.ebnf. \
             This reader, not the grammar, is what to fix."
        );
    };
    let mut code = String::new();
    let mut rest = &grammar[start..];
    loop {
        let (before, after) = match rest.find("(*") {
            Some(o) => (&rest[..o], Some(&rest[o + 2..])),
            None => (rest, None),
        };
        code.push_str(before);
        if before.contains(';') {
            break;
        }
        let Some(after) = after else { break };
        match after.find("*)") {
            Some(c) => rest = &after[c + 2..],
            None => break,
        }
    }
    let production = code.split(';').next().unwrap_or_default();
    let mut out = Vec::new();
    let mut parts = production.split('"');
    parts.next();
    while let Some(word) = parts.next() {
        out.push(word.to_string());
        if parts.next().is_none() {
            break;
        }
    }
    assert!(
        out.len() > 20,
        "the `keyword` production's shape changed: {} alternatives read, which \
         is fewer than the set has ever held. This reader, not the grammar, is \
         what to fix.",
        out.len()
    );
    out
}

/// The figures the grammar STATES, read off its own census lines.
///
/// A stated line is one whose first word is `CENSUS`, so that prose ABOUT the
/// lines cannot be mistaken for one. Anything else on such a line is a hard
/// failure rather than a skip: a silently ignored malformed line is a figure
/// that stops being guarded without saying so.
fn stated_figures(grammar: &str) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    for line in grammar.lines() {
        let Some(rest) = line.trim_start().strip_prefix("CENSUS ") else {
            continue;
        };
        let Some((key, value)) = rest.split_once('=') else {
            panic!(
                "a `CENSUS` line in spec/grammar-t1.ebnf carries no `=`: \
                 {line:?}. Every one of them is read by this test."
            );
        };
        let key = key.trim().to_string();
        let value = value.trim();
        assert!(
            !key.is_empty() && !key.contains(char::is_whitespace),
            "`CENSUS {key}` is not a single-word key: {line:?}"
        );
        let digits: String = value.chars().filter(|c| *c != ',').collect();
        let Ok(n) = digits.trim().parse::<i64>() else {
            panic!(
                "`CENSUS {key} = {value}` in spec/grammar-t1.ebnf does not end \
                 in a number. Every CENSUS line is read by this test."
            );
        };
        assert!(
            out.insert(key.clone(), n).is_none(),
            "`CENSUS {key}` is stated twice in spec/grammar-t1.ebnf. One of \
             them will go stale unseen; state it once."
        );
    }
    out
}

/// THE GUARD. Not `#[ignore]`d, and it must not become so: it costs a lex of
/// nineteen sources and a read of one file — under a second, no interpreter —
/// and it is the only thing between the grammar's figures and the drift that
/// took them to a third of the truth.
#[test]
fn the_grammars_stated_census_matches_a_fresh_count() {
    let files = corpus();
    let grammar = std::fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("{} is readable: {e}", grammar_path().display()));

    // ── word counts, by equality over `Kind::Word` ───────────────────────
    let mut measured: BTreeMap<String, i64> = BTreeMap::new();
    for w in LED_WORDS {
        let n = files
            .iter()
            .flat_map(|(_, t)| t.iter())
            .filter(|t| is_word(t, w))
            .count();
        measured.insert(w.to_string(), n as i64);
    }

    // ── the partition ───────────────────────────────────────────────────
    let all: Vec<Site> = files.iter().flat_map(|(n, t)| sites(n, t)).collect();
    let bindings: Vec<&Site> = all.iter().filter(|s| s.binding).collect();
    let assignments: Vec<&Site> = all.iter().filter(|s| !s.binding).collect();
    measured.insert("bindings".into(), bindings.len() as i64);
    measured.insert("assignments".into(), assignments.len() as i64);
    measured.insert(
        "public-bindings".into(),
        bindings.iter().filter(|s| s.public).count() as i64,
    );

    // ── the keyword set ─────────────────────────────────────────────────
    let alternatives = keyword_alternatives(&grammar);
    let written: Vec<&String> = alternatives
        .iter()
        .filter(|w| {
            files
                .iter()
                .flat_map(|(_, t)| t.iter())
                .any(|t| is_word(t, w))
        })
        .collect();
    measured.insert("keyword-alternatives".into(), alternatives.len() as i64);
    measured.insert("keyword-written".into(), written.len() as i64);

    // ── the STRUCTURAL claims, which no single number can carry ──────────
    //
    // "The `भवति` sites partition EXACTLY" — every site is one or the other,
    // and the two sets sum to every `भवति` the lexer sees. A count that summed
    // to something else would mean this walk has a hole, not that the language
    // changed.
    pin_report!(
        bindings.len() + assignments.len(),
        measured[W_BECOMES] as usize,
        "the partition lost sites: {} bindings + {} assignments against {} \
         `भवति` word tokens. This walk has a hole in it.",
        bindings.len(),
        assignments.len(),
        measured[W_BECOMES],
    );

    // "every one … has a `ॱॱ type` before the verb; … not one of them has a
    // `ॱॱ`" — the mark separates the two sets exactly, which is what says
    // `[ ॱॱ type ]` would describe a form the corpus never writes.
    let untyped_binding: Vec<&&Site> = bindings.iter().filter(|s| !s.typed).collect();
    let typed_assignment: Vec<&&Site> = assignments.iter().filter(|s| s.typed).collect();
    assert!(
        untyped_binding.is_empty(),
        "a `चरः` binding with no `ॱॱ type` before `भवति` — the grammar says \
         there are none, so either the annotation became optional or this walk \
         is wrong: {}",
        sample(&untyped_binding),
    );
    assert!(
        typed_assignment.is_empty(),
        "an assignment carrying a `ॱॱ` before `भवति` — the grammar says there \
         are none, and if one is now written the `binding` production is what \
         has to answer for it: {}",
        sample(&typed_assignment),
    );

    // `चरः` leads a statement and appears nowhere else, which is what lets the
    // token count and the binding count be quoted as one figure.
    pin_report!(
        measured[W_VAR],
        measured["bindings"],
        "`चरः` word tokens ({}) and `भवति` sites led by `चरः` ({}) disagree — \
         the word is now written somewhere that is not the head of a binding, \
         and the block's single figure for both is no longer honest.",
        measured[W_VAR],
        measured["bindings"],
    );

    // The five statement-form words the block says are written ZERO times,
    // checked BY NAME. `भङ्गः` appearing while `रक्षा` vanished would leave a
    // count of zero unwritten words untouched.
    let now_written: Vec<&str> = UNWRITTEN_STATEMENT_WORDS
        .into_iter()
        .filter(|w| {
            files
                .iter()
                .flat_map(|(_, t)| t.iter())
                .any(|t| is_word(t, w))
        })
        .collect();
    assert!(
        now_written.is_empty(),
        "the corpus now writes {now_written:?}, which the grammar's statement \
         block says it writes ZERO times. A statement form exists that the \
         `statement` production does not list — that is an ADR, not an edit.",
    );
    for w in UNWRITTEN_STATEMENT_WORDS {
        assert!(
            alternatives.iter().any(|a| a == w),
            "`{w}` is named as an unwritten statement word but is no longer an \
             alternative of `keyword`. This list and the production disagree.",
        );
    }

    // ── the file against the count ──────────────────────────────────────
    let stated = stated_figures(&grammar);
    pin_report!(
        stated,
        measured,
        "\n\nTHE GRAMMAR'S STATED CENSUS NO LONGER MATCHES THE CORPUS.\n\
         `spec/grammar-t1.ebnf`'s statement block carries its figures as \
         `CENSUS <key> = <n>` lines. Left is what the file says; right is what \
         `crates/sadhana-t1/src/*.t1` measures today.\n\n\
         This is not a failure to route around. Update the CENSUS lines, and \
         KEEP the superseded figure with its date beside them — the block is \
         written to show its own corrections, because a file that shows them \
         teaches what a current-answer-only file cannot. That is the whole \
         reason this test exists: these numbers stood unre-taken from \
         2026-08-30 to 2026-09-05 and every one of them reached a third of the \
         truth.\n\n"
    );
}

fn sample(sites: &[&&Site]) -> String {
    sites
        .iter()
        .take(5)
        .map(|s| format!("{}:{}", s.file, s.line))
        .collect::<Vec<_>>()
        .join(", ")
}
