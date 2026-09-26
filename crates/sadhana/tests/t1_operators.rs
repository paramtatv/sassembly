//! `D-002h` — the T1 operator census, and the reconciliation it forces.
//!
//! # The ruling this file executes
//!
//! Three incompatible answers to "what are T1's operators" were in the tree at
//! once: `spec/lexicon.tsv`'s 24 `operator` rows from doc 15, the `.t1` corpus
//! under `crates/sadhana-t1/src/`, and `crates/tree-sitter-t1/grammar.js`,
//! which delimited T1 with ASCII `{ } ( ) ;` and opened a function with
//! `कार्यम्`. The owner ruled on 2026-08-29 that **the corpus is canonical**.
//! Everything below is derived from the corpus by lexing it; the lexicon and
//! the tree-sitter grammar are checked AGAINST that derivation, never the
//! other way round.
//!
//! # Why a census and not a table
//!
//! `spec/grammar-t1.ebnf:381-392` still says operators are deferred, so there
//! was no frozen list to check against and no way to check one without
//! counting. The counts are in [`census`] and printed by
//! [`the_operator_census_is_the_corpus_and_not_doc_15`] as `METRIC` lines, so
//! the numbers move with the tree instead of being remembered.
//!
//! # The three things counting found that reading would not have
//!
//! 1. **`crates/sadhana-t1/src/parse.t1` did not lex under `lex_t1` at all.**
//!    Lines 198 and 251 wrote the string literal `"इति"` as
//!    `उक्तम् इति इति समाप्तम्` — three words where ADR-0011 needs four, since
//!    a DOUBLED `इति` is the literal word and a THIRD one closes. It went
//!    unseen because `crates/sadhana-t1/tests/t1_sources.rs:90` lexes the
//!    corpus with `sadhana::lex::lex` — the **T0** lexer, whose `Strings::Words`
//!    mode never recognises a string at all. Fifteen T1 sources, and nothing in
//!    the tree had ever run the T1 lexer over them.
//!    [`every_t1_source_lexes_with_the_lexer_of_its_own_tier`] is that check.
//!
//! 2. **`parse.t1`'s own operator table named four spellings the corpus does
//!    not write** — `वामसृतम्`, `दक्षिणसृतम्` (zero uses each; the corpus
//!    writes `वामसृ` 23 times and `दक्षिणसृ` 12), `विषमः` (zero) and
//!    `विकल्पः`, which is the T0 MNEMONIC for bitwise-or leaking into T1 where
//!    the corpus writes `विकल्प`. So the self-hosted parser could not have read
//!    a shift in the corpus it is meant to compile.
//!    [`parse_t1s_operator_table_names_the_spelling_the_corpus_writes`] derives
//!    the right spelling from `ast.t1`'s kind constants and the bodies' own
//!    counts rather than restating it.
//!
//! 3. **Four of doc 15's operator spellings are written nowhere.** `अधि`
//!    (plus), `ऊन` (minus), `गुण` (times) and `भाग` (divided by) have zero uses
//!    in the corpus, which writes `योगः`, `वियोगः`, `गुणनम्` and `विभाजनम्` —
//!    three of them words `spec/lexicon.tsv` already carried as T0 MNEMONICS.
//!    T1 did not invent an arithmetic vocabulary; it reused T0's.

use sadhana::lex::{Kind, Token, lex_t1};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The repository root, from the crate rather than the process CWD.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.t1` source of the self-hosting toolchain, sorted, read by directory
/// walk so a new one joins the census without an edit here.
fn corpus() -> Vec<(String, String)> {
    let dir = repo_root().join("crates/sadhana-t1/src");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("crates/sadhana-t1/src exists")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    paths.sort();
    assert!(
        paths.len() >= 15,
        "the corpus shrank to {} files; it had 15 when D-002h counted it",
        paths.len()
    );
    paths
        .into_iter()
        .map(|p| {
            let name = p
                .file_name()
                .expect("a file has a name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {name}: {e}"));
            (name, text)
        })
        .collect()
}

/// One written word of T1 code: `(text, file, line)`.
///
/// **Strings and comments are excluded by the lexer, not by this function.**
/// `lex_t1` takes `उक्तम् … इति` whole as one [`Kind::Str`] (ADR-0017) and
/// drops a `॰` comment, so a spelling that appears only inside a literal —
/// which is exactly how `parse.t1` writes its operator table — is not counted
/// as a use. Splitting on whitespace would have counted all 13 of them.
fn code_words(name: &str, text: &str) -> Vec<(String, String, usize)> {
    let tokens = lex_t1(text).unwrap_or_else(|errs| {
        panic!(
            "{name} does not lex with lex_t1 — {} error(s), first: {}",
            errs.len(),
            errs[0]
        )
    });
    tokens
        .into_iter()
        .filter(|t| !matches!(t.kind, Kind::Str { .. }))
        .map(|t| (t.text, name.to_string(), t.line))
        .collect()
}

/// Every code word of the whole corpus.
fn corpus_words() -> Vec<(String, String, usize)> {
    corpus()
        .iter()
        .flat_map(|(name, text)| code_words(name, text))
        .collect()
}

/// `word -> (uses, first file, first line)`.
fn census() -> BTreeMap<String, (usize, String, usize)> {
    let mut out: BTreeMap<String, (usize, String, usize)> = BTreeMap::new();
    for (w, file, line) in corpus_words() {
        out.entry(w)
            .and_modify(|e| e.0 += 1)
            .or_insert((1, file, line));
    }
    out
}

/// `word -> uses IN BINARY-OPERATOR POSITION`, which is not the same question
/// as [`census`] answers.
///
/// # Why the frequency count cannot decide a spelling
///
/// [`census`] counts a WORD; this counts a POSITION. For twelve of the fifteen
/// operators the two agree, because nothing else in the corpus is spelled like
/// `वामसृ`. `शेष` is the case that separates them: the corpus writes `शेषम्`
/// 23 times and `शेषः` 5, so the frequency rule chooses `शेषम्` — but every
/// one of those 23 is a LOCAL VARIABLE holding a remainder
/// (`artha.t1:281` declares `चरः शेषम् ॱॱ न६४`), and the two real operator
/// uses are `encode.t1:647` and `:676`, spelled `शेषः`. Following the count
/// would have made a common variable name into an operator.
///
/// The counts this function returns are 2 for `शेषः` and 0 for `शेषम्`.
///
/// This is the same error ADR-0032 made and ADR-0037 records: **a frequency
/// count over a word cannot answer a question about a grammatical position.**
///
/// # The test, and why it needs no keyword list
///
/// A binary operator has an operand on each side, so it is counted only when
/// BOTH neighbours can be one: a `Word` or `Numeral` that is not itself an
/// operator. That single rule rejects every non-operator use above without
/// knowing which words are keywords —
///
/// * `चरः शेषम् ॱॱ` — the right neighbour is a `LabelMark`, so it is a
///   declaration, not a use;
/// * `यदि शेषः समम् ०` and `यावत् शेषम् अधिकम् १` — the right neighbour is an
///   operator, and no operator follows another;
/// * `वियोगः शेषः समाप्तम्` — the LEFT neighbour is an operator, so this
///   `शेषः` is minus's operand;
/// * `शेषम् भवति …` and `इति शेषम् भवति` — the right neighbour is `भवति`, so
///   the word is an assignment TARGET. This one is checked by name: a block
///   delimiter (`इति`, `आदि`) is an ordinary `Word` to the lexer, so the
///   neighbour rule alone let two of these through.
///
/// # The sites, not just the count
///
/// The value is `(uses, the sites)`, and the sites are what make a
/// disagreement with this census checkable instead of arguable: a count alone
/// sent one reader hunting through 23 lines by hand and getting a different
/// answer from the machine's.
fn operator_position_census() -> BTreeMap<String, (usize, Vec<String>)> {
    let ops: BTreeSet<&str> = OPERATORS.iter().map(|(w, _, _)| *w).collect();
    // `Kind::Operand` BELONGS HERE and was left out of the first draft of this
    // function, which cost `encode.t1:676` — `आरभ्य मानम् शेषः १०`, as plain an
    // operator use as `:647` above it. `म्` is कर्म's sigil (`Karaka::sigil`),
    // so `मानम्` lexes as an `Operand` and `सङ्ख्या`, with no sigil, as a
    // `Word`; a rule that admits only `Word` sees one of the two.
    let operand_ish = |t: &Token| {
        matches!(t.kind, Kind::Word | Kind::Numeral | Kind::Operand { .. })
            && !ops.contains(t.text.as_str())
    };
    let mut out: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for (name, text) in corpus() {
        let tokens = lex_t1(&text).expect("the corpus lexes; code_words would have panicked");
        for i in 1..tokens.len().saturating_sub(1) {
            // `X भवति …` is an ASSIGNMENT and `X` is its target, not an
            // operator. ADR-0026 keeps `भवति` off the precedence ladder for
            // the same reason, and `bhavati_is_not_on_the_precedence_ladder`
            // holds it there.
            if tokens[i + 1].text == "भवति" {
                continue;
            }
            if operand_ish(&tokens[i - 1]) && operand_ish(&tokens[i + 1]) {
                let e = out.entry(tokens[i].text.clone()).or_default();
                e.0 += 1;
                if e.1.len() < 6 {
                    e.1.push(format!(
                        "{name}:{} `{} {} {}`",
                        tokens[i].line,
                        tokens[i - 1].text,
                        tokens[i].text,
                        tokens[i + 1].text
                    ));
                }
            }
        }
    }
    out
}

/// The reconciled operator set: `(spelling, sense, the kind constant's stem)`.
///
/// The stem is `ast.t1`'s: `योगद्विकर्मभेद` is declared there for `+`, so the
/// stem is `योग` and the written operator is that stem with a nominal ending.
/// Nothing here is free — the tests below re-derive each column.
const OPERATORS: &[(&str, &str, &str)] = &[
    ("योगः", "plus", "योग"),
    ("वियोगः", "minus", "वियोग"),
    ("गुणनम्", "times", "गुणन"),
    ("विभाजनम्", "divided by", "विभाजन"),
    ("समम्", "equal", "सम"),
    ("असमम्", "not equal", "असम"),
    ("न्यूनम्", "less than", "न्यून"),
    ("अधिकम्", "greater than", "अधिक"),
    ("युक्", "bitwise and", "युक्"),
    ("विकल्प", "bitwise or", "विकल्प"),
    ("विषम", "bitwise xor", "विषम"),
    ("वामसृ", "shift left", "वामसृ"),
    ("दक्षिणसृ", "shift right", "दक्षिणसृ"),
    // ADR-0037, 2026-09-02, by owner decision. ADR-0032 recorded both as
    // absent and each entry failed differently. MODULO: its evidence — the
    // corpus's `शेषः` tokens are all a LOCAL VARIABLE — was true on
    // 2026-08-30, and `encode.t1:647`'s operator use is dated 08-31, the day
    // after; the ADR was overtaken by `D-002a2`. `>=`: it asked whether the
    // LEXICON's `न्यूनसमम्`/`अधिकसमम्` appear, but the corpus writes
    // `बृहत्समम्`, and `vakyavibhaga.t1:3163` has carried it since 08-29 — the
    // day BEFORE — so that census, in an ADR whose principle is that the
    // CORPUS is canonical, was keyed to the wrong vocabulary.
    //
    // THE ORDER IS LOAD-BEARING: this list is compared against `ast.t1`'s kind
    // constants and `parse.t1`'s matcher arms IN ORDER, so both go last, where
    // their constants (१४, १५) are.
    ("शेषः", "remainder", "शेष"),
    ("बृहत्समम्", "greater or equal", "बृहत्सम"),
];

/// The one operator `ast.t1` declares and no body has ever written.
///
/// Kept as a NAMED constant with a length assertion rather than a silent
/// `if`, so a second unwritten operator cannot slip in behind it.
const DECLARED_BUT_UNWRITTEN: &[&str] = &["विषम"];

/// Doc 15's spellings that the corpus replaced, and what replaced each.
///
/// These are the rows `spec/lexicon.tsv` offered as T1 operators and that no
/// `.t1` source writes. They are listed so their ABSENCE can be asserted:
/// a census that only counted what is present would not notice one coming back.
/// **`अधि` is deliberately not in this list, and the reason is a measurement.**
/// The corpus writes `अधि` twice — `crates/sadhana-t1/src/utsarjana.t1:624-625`
/// — as an ordinary variable name, so its absence is not assertable and its
/// presence proves nothing. Its lexicon row survives with the one sense the
/// tree actually implements, `t1::drishya`'s text join
/// (`crates/sadhana/src/t1/drishya.rs:619`), and
/// [`no_lexicon_operator_still_claims_a_sense_the_corpus_spells_otherwise`]
/// holds the *plus* sense gone instead.
const SUPERSEDED: &[(&str, &str)] = &[
    ("ऊन", "वियोगः"),
    ("गुण", "गुणनम्"),
    ("भाग", "विभाजनम्"),
    ("विषमम्", "असमम्"),
    ("परिवृत्तअधि", "परिवृत्तयोगः"),
];

/// The senses the corpus settled, and the word it settled each on.
///
/// Checked against the lexicon's `english` column rather than its spellings,
/// because that is the column doc 01 §4 rule 5 is about: a second word for
/// *plus* is drift whatever it is spelled.
const SENSES: &[(&str, &str)] = &[
    ("add", "योगः"),
    ("subtract", "वियोगः"),
    ("multiply", "गुणनम्"),
    ("divided by", "विभाजनम्"),
    ("not equal", "असमम्"),
    // `("text join", "अधि")` STOOD HERE UNTIL 2026-08-31, AND IT WAS IN THE
    // WRONG CENSUS. `अधि` IS a real operator — `t1::drishya`'s text join,
    // `crates/sadhana/src/t1/drishya.rs:619`, `while self.eat("अधि")` — but
    // drishya is the T2 UI sub-language, and this table is the T1 operator
    // census. Claiming it here put a T2 word in a T1 count.
    //
    // It surfaced as a merge conflict rather than as a category error: this
    // branch kept an `अधि` row reglossed "text join", `origin/main` retired the
    // spelling outright, and `grammar_t1.rs`'s retirement check is written
    // against the SPELLING (`w == gone`) rather than the sense — so the two
    // suites could not both pass while a T1 row spelled `अधि` existed.
    // Removing it from THIS table is not a concession: it is where the row
    // actually belonged, and it overrides neither branch's frozen rule.
    //
    // WHAT IS STILL MISSING, filed rather than dropped: the lexicon now has no
    // record of drishya's text-join word at all, though the tree implements it.
    // That is a T2 vocabulary row, not a T1 operator row.
];

/// Senses doc 15 had that the reconciliation FOLDED AWAY.
///
/// T1 does not have a *plus* distinct from T0's *add*: the corpus writes
/// `योगः` for both, so one row carries one sense and the doc-15 duplicates are
/// gone rather than re-sensed. Asserting the emptiness is what stops `अधि
/// plus` coming back beside `योगः add`.
const FOLDED_AWAY: &[&str] = &["plus", "minus", "times", "wrapping plus"];

/// The nominal endings a T1 operator is written with, longest first.
const ENDINGS: &[&str] = &["तम्", "म्", "ः", ""];

// ── the corpus ─────────────────────────────────────────────────────────────

/// The finding that made the rest of this file possible.
///
/// `crates/sadhana-t1/tests/t1_sources.rs:90` lexes the corpus with
/// `sadhana::lex::lex`, which is T0's lexer. Under it `उक्तम्` is an ordinary
/// word and no string exists, so `parse.t1:198`'s unterminated literal read as
/// four harmless words for as long as the corpus has existed. This test uses
/// the lexer of the tier the files are written in.
#[test]
fn every_t1_source_lexes_with_the_lexer_of_its_own_tier() {
    let mut lexed = 0usize;
    for (name, text) in corpus() {
        match lex_t1(&text) {
            Ok(_) => lexed += 1,
            Err(errs) => panic!(
                "{name} does not lex with lex_t1 ({} error(s)); first at line {}: {}",
                errs.len(),
                errs[0].line,
                errs[0].reason
            ),
        }
    }
    println!("METRIC t1_sources_lexing_as_t1 {lexed}");
    assert!(lexed >= 15, "only {lexed} of the corpus lexed as T1");
}

/// The census itself, and the claim that doc 15 lost.
#[test]
fn the_operator_census_is_the_corpus_and_not_doc_15() {
    let c = census();

    println!("\noperator census — crates/sadhana-t1/src/*.t1, code words only");
    println!("{:<14} {:>6}  first written at", "spelling", "uses");
    let mut total = 0usize;
    for (spelling, sense, _) in OPERATORS {
        let (n, file, line) = c.get(*spelling).cloned().unwrap_or((0, "—".into(), 0));
        total += n;
        println!("{spelling:<14} {n:>6}  {file}:{line}  ({sense})");
    }
    println!("METRIC t1_operator_uses {total}");
    println!("METRIC t1_operators_written {}", OPERATORS.len());

    // Every operator but the one exception is written in a body.
    let unwritten: Vec<&str> = OPERATORS
        .iter()
        .filter(|(s, _, _)| c.get(*s).is_none_or(|e| e.0 == 0))
        .map(|(s, _, _)| *s)
        .collect();
    assert_eq!(
        unwritten, DECLARED_BUT_UNWRITTEN,
        "the set of declared-but-unwritten operators changed; it was `विषम` \
         alone, attested only by ast.t1's `विषमद्विकर्मभेद` and parse.t1's table"
    );

    // …and doc 15's replaced spellings are written nowhere. This is the half
    // of the census that a table of what IS present cannot state.
    for (old, new) in SUPERSEDED {
        assert!(
            !c.contains_key(*old),
            "`{old}` is written in the corpus again ({} use(s)); the corpus \
             spells this operator `{new}`, and D-002h removed `{old}` from \
             spec/lexicon.tsv on the strength of its absence",
            c.get(*old).map_or(0, |e| e.0)
        );
    }
}

// ── the corpus against itself ──────────────────────────────────────────────

/// `parse.t1`'s operator table, read out of `parse.t1`.
///
/// `द्विकर्ममेलनम्` is written as one line per operator:
///
/// ```text
/// यदि मेलनम् आरभ्य उक्तम् योगः इति समाप्तम् आदि प्रत्यागमनम् वास्तुॱयोगद्विकर्मभेद । इति
/// ```
///
/// so the pair wanted is (the [`Kind::Str`] value, the word ending in
/// `द्विकर्मभेद` that follows it). Read this way the table cannot be confused
/// with a body use: the spelling it matches is a STRING, and `lex_t1` gives it
/// a different kind from the same akṣaras written as code.
fn parse_t1_operator_table() -> Vec<(String, String)> {
    const KIND_SUFFIX: &str = "द्विकर्मभेद";
    const ROUTINE: &str = "द्विकर्ममेलनम्";
    let text = std::fs::read_to_string(repo_root().join("crates/sadhana-t1/src/parse.t1"))
        .expect("read parse.t1");
    let tokens = lex_t1(&text).expect("parse.t1 lexes");

    // ═══ THE SCRAPE IS BOUND TO ITS ROUTINE, AND WAS NOT ═══
    //
    // This walked EVERY string in `parse.t1` and paired it with the next word
    // ending in `द्विकर्मभेद`. The only bound was "a kind word must arrive
    // before the next string" — which bounds nothing for the LAST string of a
    // routine that names no kind constants at all.
    //
    // `72117493` added exactly that: the operator-spelling predicate at
    // `parse.t1:779-830` tests `पाठ समम् उक्तम् <spelling> इति` and returns
    // `सत्यम्`, naming no kinds ("`द्विकर्ममेलनम्` cannot serve here: it
    // CONSUMES"). Its last string found no following string, so the inner scan
    // ran on to `वास्तुॱगुणनद्विकर्मभेद` at `:1086` and minted one spurious
    // pair — 16 stems against `ast.t1`'s 15, the extra a second `गुणन`.
    //
    // The premise was "strings and kind constants alternate". A string-only
    // routine is legal, so the premise was never a property of the corpus. The
    // fix is the bound the scrape always needed: this routine's own token span.
    // Same repair as the duplicate-body trap in `encode.t1` — resolve a search
    // to the declaration it belongs to, never to file-wide uniqueness.
    let start = tokens
        .windows(2)
        .position(|w| w[0].text == "वृत्तिः" && w[1].text == ROUTINE)
        .unwrap_or_else(|| panic!("parse.t1 declares no `वृत्तिः {ROUTINE}`"));
    let end = tokens[start + 1..]
        .iter()
        .position(|t| t.text == "वृत्तिः")
        .map_or(tokens.len(), |o| start + 1 + o);
    let body = &tokens[start..end];

    let mut out = Vec::new();
    for (i, t) in body.iter().enumerate() {
        let Kind::Str { value } = &t.kind else {
            continue;
        };
        // The kind constant is the next word that ends in `द्विकर्मभेद`, and
        // it must arrive before the next string, or this string belonged to
        // some other test. Bounded to `body`, so it cannot leave the routine.
        for u in &body[i + 1..] {
            if matches!(u.kind, Kind::Str { .. }) {
                break;
            }
            if let Some(stem) = u.text.strip_suffix(KIND_SUFFIX) {
                // `वास्तुॱयोगद्विकर्मभेद` — the module qualifier is part of
                // the word, because `lex.rs` peels only a TRAILING mark.
                let stem = stem.rsplit('ॱ').next().unwrap_or(stem);
                out.push((value.clone(), stem.to_string()));
                break;
            }
        }
    }
    out
}

/// `ast.t1`'s binary-operator kind constants, read out of `ast.t1`.
fn ast_t1_operator_stems() -> Vec<String> {
    // RIG-ALLOW: whole-file — `ast.t1` declares its kind constants at module
    // scope, so there is no routine to bound to; the file IS the unit. Unlike
    // `parse_t1_operator_table`, which reads constants out of one routine's
    // body and must say which.
    const KIND_SUFFIX: &str = "द्विकर्मभेद";
    let text = std::fs::read_to_string(repo_root().join("crates/sadhana-t1/src/ast.t1"))
        .expect("read ast.t1");
    let tokens = lex_t1(&text).expect("ast.t1 lexes");
    let mut out = Vec::new();
    for t in &tokens {
        if let Some(stem) = t.text.strip_suffix(KIND_SUFFIX) {
            let stem = stem.rsplit('ॱ').next().unwrap_or(stem).to_string();
            if !out.contains(&stem) {
                out.push(stem);
            }
        }
    }
    out
}

/// The corpus's declaration of its own operators agrees with its use of them.
///
/// **Both sides are read from files and neither is written down here.** The
/// left side is `parse.t1`'s table; the right side is, for each of `ast.t1`'s
/// kind constants, the inflection of that constant's stem that the corpus's
/// BODIES write most often — or the bare stem when no body writes any of them.
/// That rule picked `वामसृ` over `वामसृतम्` because many lines write the first
/// and none the second, and `विकल्प` over `विकल्पः` nine uses to one. Neither
/// number is stored: both are recounted here on every run, so the rule keeps
/// working as the corpus grows instead of encoding the day it was measured.
#[test]
fn parse_t1s_operator_table_names_the_spelling_the_corpus_writes() {
    let c = census();
    let table = parse_t1_operator_table();
    let stems = ast_t1_operator_stems();

    assert_eq!(
        table.iter().map(|(_, k)| k.clone()).collect::<Vec<_>>(),
        stems,
        "parse.t1's `द्विकर्ममेलनम्` and ast.t1's kind constants no longer \
         describe the same operators, in the same order"
    );
    assert_eq!(
        stems.len(),
        OPERATORS.len(),
        "ast.t1 declares {} binary operators; the census names {}",
        stems.len(),
        OPERATORS.len()
    );

    let pos = operator_position_census();
    let mut fixed = 0usize;
    for (written, stem) in &table {
        // Candidates: the stem under each nominal ending, nothing else. This
        // is what keeps `समाप्तम्` — which also begins with `सम` — out of the
        // running for `समम्`.
        //
        // Ranked by uses in OPERATOR POSITION, not by how often the word is
        // written: see [`operator_position_census`] for the case that forced
        // the distinction.
        let best = ENDINGS
            .iter()
            .map(|e| format!("{stem}{e}"))
            .max_by_key(|cand| pos.get(cand).map_or(0, |e| e.0))
            .expect("ENDINGS is not empty");
        let want = if pos.get(&best).is_none_or(|e| e.0 == 0) {
            stem.clone() // nothing written: the bare stem, as ast.t1 spells it
        } else {
            best
        };
        assert_eq!(
            written,
            &want,
            "parse.t1 matches `{written}` for `{stem}द्विकर्मभेद`, but the \
             corpus writes `{want}` between two operands {} time(s) and \
             `{written}` {} time(s) — the table cannot recognise the operator \
             its own siblings use. (Whole-word counts, which are NOT the \
             question: `{want}` {}, `{written}` {}.)\n  `{want}` sits between \
             two operands at: {:?}\n  `{written}` at: {:?}",
            pos.get(&want).map_or(0, |e| e.0),
            pos.get(written).map_or(0, |e| e.0),
            c.get(&want).map_or(0, |e| e.0),
            c.get(written).map_or(0, |e| e.0),
            pos.get(&want).map(|e| e.1.clone()).unwrap_or_default(),
            pos.get(written).map(|e| e.1.clone()).unwrap_or_default()
        );
        if pos.get(written).is_none_or(|e| e.0 == 0) {
            fixed += 1;
        }
    }
    assert_eq!(
        fixed,
        DECLARED_BUT_UNWRITTEN.len(),
        "{fixed} operator(s) in parse.t1's table are written by no body; only \
         the xor is meant to be"
    );
}

// ── the lexicon ────────────────────────────────────────────────────────────

/// One `spec/lexicon.tsv` row.
struct Entry {
    deva: String,
    english: String,
    category: String,
}

fn lexicon() -> Vec<Entry> {
    let text =
        std::fs::read_to_string(repo_root().join("spec/lexicon.tsv")).expect("read lexicon.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .skip(1) // the header
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 5).then(|| Entry {
                deva: f[0].to_string(),
                english: f[3].to_string(),
                category: f[4].to_string(),
            })
        })
        .collect()
}

/// The lexicon names every operator the corpus writes, and names it as one.
#[test]
fn the_lexicon_names_every_operator_the_corpus_writes() {
    let lex = lexicon();
    for (spelling, sense, _) in OPERATORS {
        let row = lex
            .iter()
            .find(|e| e.deva == *spelling)
            .unwrap_or_else(|| panic!("`{spelling}` ({sense}) has no spec/lexicon.tsv row at all"));
        assert!(
            row.category.split('+').any(|c| c == "operator"),
            "`{spelling}` is in the lexicon as `{}`, not as an operator — the \
             corpus writes it as one",
            row.category
        );
    }
    let n = lex
        .iter()
        .filter(|e| e.category.split('+').any(|c| c == "operator"))
        .count();
    println!("METRIC lexicon_operator_terms {n}");
}

/// No superseded spelling is still offered as a T1 operator.
///
/// The negative half of the reconciliation, and the half that rots first: a
/// lexicon that gained `योगः` and kept `अधि` would offer two words for plus,
/// which is the drift doc 01 §4 rule 5 exists to stop.
#[test]
fn the_lexicon_offers_no_superseded_spelling_as_a_t1_operator() {
    let lex = lexicon();
    for (old, new) in SUPERSEDED {
        let still = lex
            .iter()
            .find(|e| e.deva == *old && e.category.split('+').any(|c| c == "operator"));
        assert!(
            still.is_none(),
            "spec/lexicon.tsv still offers `{old}` ({}) as an operator; the \
             corpus writes `{new}`, and no `.t1` source writes `{old}`",
            still.map_or("", |e| e.english.as_str())
        );
    }
}

/// The senses, not just the spellings.
///
/// `अधि` cannot be checked by absence — the corpus writes it as a variable
/// name — so it is checked by SENSE instead: whatever row carries *plus* must
/// be the word the corpus writes, and `अधि` is not that word.
#[test]
fn no_lexicon_operator_still_claims_a_sense_the_corpus_spells_otherwise() {
    let lex = lexicon();
    for (sense, spelling) in SENSES {
        let carriers: Vec<&str> = lex
            .iter()
            .filter(|e| e.english == *sense)
            .map(|e| e.deva.as_str())
            .collect();
        assert_eq!(
            carriers,
            vec![*spelling],
            "*{sense}* is carried by {carriers:?} in spec/lexicon.tsv; the \
             corpus spells it `{spelling}` and doc 01 §4 rule 5 allows one word"
        );
    }
    for sense in FOLDED_AWAY {
        // SCOPED TO `operator` ROWS 2026-08-31. This filtered on the sense
        // alone, which was right while a folded-away sense had no row at all.
        // The owner ruled that `परिवृत्तअधि` is not an operator but KEEPS its
        // row — `origin/main`'s suite names it, and deleting it would lose the
        // record that doc 15 ever proposed it. The row is `concept` now and
        // still glossed *wrapping plus*.
        //
        // The intent above is unchanged and is what this still checks: it
        // "stops `अधि plus` coming back BESIDE `योगः add`" — a SECOND OPERATOR
        // for one sense. A `concept` row cannot be that. An unscoped filter
        // would have made the owner's ruling unrepresentable, since it forbade
        // the very row the other suite requires to exist.
        let carriers: Vec<&str> = lex
            .iter()
            .filter(|e| e.english == *sense && e.category.split('+').any(|c| c == "operator"))
            .map(|e| e.deva.as_str())
            .collect();
        assert!(
            carriers.is_empty(),
            "*{sense}* is back in spec/lexicon.tsv as an OPERATOR, spelled \
             {carriers:?}; T1 has no such operator distinct from T0's, and the \
             corpus proves it by writing T0's word"
        );
    }
}

// ── the tree-sitter grammar ────────────────────────────────────────────────

/// Every single-quoted literal in `grammar.js`, in source order.
fn grammar_js_literals() -> Vec<String> {
    let text = std::fs::read_to_string(repo_root().join("crates/tree-sitter-t1/grammar.js"))
        .expect("read grammar.js");
    let mut out = Vec::new();
    for line in text.lines() {
        // Comments in that file quote the old ASCII delimiters while
        // explaining them, so a line-comment is not scanned for literals.
        let code = line.split("//").next().unwrap_or("");
        let mut rest = code;
        while let Some(open) = rest.find('\'') {
            let after = &rest[open + 1..];
            match after.find('\'') {
                Some(close) => {
                    out.push(after[..close].to_string());
                    rest = &after[close + 1..];
                }
                None => break,
            }
        }
    }
    out
}

/// The quoted terminals of `spec/grammar-t1.ebnf`, comments removed.
fn ebnf_terminals() -> Vec<String> {
    let text = std::fs::read_to_string(repo_root().join("spec/grammar-t1.ebnf"))
        .expect("read grammar-t1.ebnf");
    let mut code = String::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find("(*") {
        code.push_str(&rest[..open]);
        match rest[open..].find("*)") {
            Some(close) => rest = &rest[open + close + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    code.push_str(rest);

    let mut out = Vec::new();
    let mut chars = code.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            let mut lit = String::new();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                lit.push(c);
            }
            out.push(lit);
        }
    }
    out
}

/// The tree-sitter grammar speaks the corpus's dialect and no other.
///
/// **The check is derived, not a word list.** Every Devanagari literal in
/// `grammar.js` must be a word the corpus writes, a prefix of one (`०षोड्`
/// opens `०षोड्८००`), or a quoted terminal of the frozen `spec/grammar-t1.ebnf`.
/// The version this replaces would have failed on `समर्पय`, which is in none of
/// the three and occurs nowhere else in the repository.
#[test]
fn the_tree_sitter_grammar_speaks_the_corpus_dialect() {
    let c = census();
    let terminals = ebnf_terminals();
    let literals = grammar_js_literals();
    assert!(
        literals.len() > 30,
        "only {} literal(s) found in grammar.js; the scanner is broken or the \
         grammar was gutted",
        literals.len()
    );

    let mut checked = 0usize;
    for lit in &literals {
        if !lit
            .chars()
            .any(|ch| ('\u{0900}'..='\u{097F}').contains(&ch))
        {
            continue; // an ASCII fragment of a regex or a field name
        }
        checked += 1;
        let written = c.contains_key(lit);
        let prefix = c.keys().any(|w| w.starts_with(lit.as_str()));
        let frozen = terminals.iter().any(|t| t == lit);
        assert!(
            written || prefix || frozen,
            "grammar.js writes `{lit}`, which no .t1 source writes and \
             spec/grammar-t1.ebnf does not name — the corpus is canonical, so \
             a terminal it does not contain is invented"
        );
    }
    println!("METRIC tree_sitter_t1_devanagari_terminals {checked}");
    assert!(
        checked >= 25,
        "only {checked} Devanagari terminal(s) in grammar.js"
    );

    // The specific defects the first version had. `कार्यम्` would survive the
    // derived check above — the corpus writes it, but as an ordinary variable
    // at `utsarjana.t1:289`, never as the `fn` keyword — so it is named here.
    let text = std::fs::read_to_string(repo_root().join("crates/tree-sitter-t1/grammar.js"))
        .expect("read grammar.js");
    for (bad, real) in [("कार्यम्", "वृत्तिः"), ("समर्पय", "प्रत्यागमनम्")]
    {
        assert!(
            !literals.iter().any(|l| l == bad),
            "grammar.js names `{bad}` as a keyword; T1 writes `{real}`"
        );
        assert!(
            text.contains(bad) || text.contains(real),
            "the note explaining `{bad}` was deleted along with it"
        );
    }
    for ascii in ["{", "}", "(", ")", ";"] {
        assert!(
            !literals.iter().any(|l| l == ascii),
            "grammar.js delimits T1 with `{ascii}`; ADR-0003 replaced the \
             bracket roles with paired words on A-033's font measurement, and \
             spec/grammar-t1.ebnf:334-336 froze them"
        );
    }
    // Not vacuous: the pairs that must be there instead.
    for word in ["आरभ्य", "समाप्तम्", "आदि", "इति", "अङ्कः", "अन्तः", "।", "॥"]
    {
        assert!(
            literals.iter().any(|l| l == word),
            "grammar.js no longer uses `{word}`"
        );
    }
    // Every operator the census found is a terminal of the grammar.
    for (spelling, sense, _) in OPERATORS {
        assert!(
            literals.iter().any(|l| l == spelling),
            "grammar.js has no terminal for `{spelling}` ({sense})"
        );
    }
}

/// DUMP THE TOKENS AROUND A LINE — the instrument for "a frozen keyword ended
/// up in expression position", which is what two of the corpus's five resolve
/// refusals are.
///
/// `parse.t1:512` and `sanskrit_text.t1:813` are refused by `अर्थ` as
/// undeclared `इति` and `अन्यथा`. Both are FROZEN KEYWORDS, so the question is
/// never "is the source wrong" but "what did the lexer and parser actually
/// see" — and reasoning about the source answered that wrongly twice on
/// 2026-09-02. This prints the answer instead.
///
/// `SANSOS_DUMP=<file>:<line>` selects; the default is the pair above.
#[test]
#[ignore = "measurement"]
fn dump_tokens_around_a_line() {
    let want = std::env::var("SANSOS_DUMP")
        .unwrap_or_else(|_| "parse.t1:512,sanskrit_text.t1:813,vakyavibhaga.t1:2061".to_string());
    for spec in want.split(',') {
        let (file, line) = spec.split_once(':').expect("<file>:<line>");
        let line: usize = line.parse().expect("a line number");
        let path = repo_root().join("crates/sadhana-t1/src").join(file);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {file}: {e}"));
        let toks = match lex_t1(&text) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{file} DOES NOT LEX: {:?}", &e[..e.len().min(2)]);
                continue;
            }
        };
        eprintln!("\n=== {file}:{line} ===");
        for (i, t) in toks.iter().enumerate() {
            if t.line + 1 >= line && t.line <= line + 1 {
                eprintln!(
                    "  [{i:5}] line {:>5}  {:?}  {:?}",
                    t.line,
                    // the kind without its payload, which is the whole point
                    match &t.kind {
                        Kind::Str { .. } => "Str".to_string(),
                        other => format!("{other:?}"),
                    },
                    t.text
                );
            }
        }
    }
}
