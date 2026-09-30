//! `spec/grammar-t1.ebnf` says what it can and refuses what it cannot — `B-079a`.
//!
//! The lexical layer of T1 is frozen: keywords, type names, identifiers,
//! numerals and comments. Every one of them is spelled with signs ADR-0003
//! ratified, so none of it waits on anything.
//!
//! The phrase structure is absent, and these tests hold it absent. Doc 15
//! §3.1 assigns grouping, blocks and indexing to `꣼ ꣸ ꣺ ꣻ ᳵ ᳶ`, which task
//! A-033 measured at **0 of 27** Devanagari faces, and doc 15 §15.3.3 says the
//! replacement is decided by a readability study with human readers (`A-063`).
//! A grammar that guessed would be frozen against a measurement nobody took.

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

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

fn grammar() -> String {
    std::fs::read_to_string(root().join("spec/grammar-t1.ebnf")).expect("read spec/grammar-t1.ebnf")
}

/// Every quoted terminal, with `(* … *)` comments removed first so prose
/// examples do not masquerade as terminals.
fn terminals() -> Vec<String> {
    let text = grammar();
    let mut rest = text.as_str();
    let mut code = String::new();
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
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut lit = String::new();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                lit.push(c);
            }
            if !lit.is_empty() {
                out.push(lit);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// One production's right-hand side, comments stripped, `name = … ;`.
///
/// `terminals()` answers what the grammar CAN spell anywhere; a question about
/// where a sign is admitted needs the production and not the set. `इ` is in
/// both `hex_digit` and, if it were ever widened, `integer_type` — and only the
/// second would be a decision.
fn production(name: &str) -> String {
    let text = grammar();
    let mut rest = text.as_str();
    let mut code = String::new();
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
    for chunk in code.split(';') {
        let chunk = chunk.trim_start();
        let Some(eq) = chunk.find('=') else { continue };
        if chunk[..eq].trim() == name {
            return chunk[eq + 1..].trim().to_string();
        }
    }
    panic!("`spec/grammar-t1.ebnf` defines no production `{name}`");
}

#[test]
fn every_terminal_survives_orthographic_closure() {
    // A terminal outside doc 15's repertoire could not appear in source, so the
    // production naming it would be unreachable — the grammar would describe a
    // language nobody can write.
    for t in terminals() {
        let bad = sanskrit_text::repertoire::check(&t);
        assert!(
            bad.is_empty(),
            "terminal `{t}` leaves the repertoire: {:?}",
            bad.iter().map(|v| v.ch).collect::<Vec<_>>()
        );
    }
}

#[test]
fn no_terminal_is_a_sign_that_renders_in_no_face() {
    // The seven A-033 measured at 0 of 27. Doc 15 §3.1 assigns four of T1's
    // roles to them — grouping, blocks, indexing, strings — and ADR-0003 chose
    // words over the one T0 needed. A grammar that quietly used the other six
    // would be unwritable and unreadable at once.
    const UNRENDERED: [char; 7] = [
        '\u{A8FC}', '\u{A8F8}', '\u{A8FA}', '\u{A8FB}', '\u{A8F9}', '\u{1CF5}', '\u{1CF6}',
    ];
    for t in terminals() {
        for c in t.chars() {
            assert!(
                !UNRENDERED.contains(&c),
                "terminal `{t}` uses U+{:04X}, which A-033 measured at 0 of 27 faces",
                c as u32
            );
        }
    }
}

#[test]
fn no_latin_anywhere_in_the_terminals() {
    for t in terminals() {
        assert!(
            !t.chars()
                .any(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation()),
            "terminal `{t}` contains Latin or ASCII punctuation"
        );
    }
}

#[test]
fn the_type_names_are_the_ones_t0_already_writes() {
    // ADR-0010 made `ॱअ३२` a type name in an instruction. A declaration must
    // spell the same type the same way, or the two tiers are two languages.
    let ts = terminals();
    for part in ["अ", "न", "प", "८", "१६", "३२", "६४", "१२८"] {
        assert!(
            ts.iter().any(|t| t == part),
            "`{part}` is part of T0's type vocabulary and T1's grammar does not name it"
        );
    }
    for named in ["बूल", "अक्षरम्", "पाठ"] {
        assert!(ts.iter().any(|t| t == named), "`{named}` is missing");
    }
    // ADR-0029 replaced `पाठः` with `पाठ`. The removal is asserted as well as
    // the arrival, because a `choice` that kept both would satisfy every line
    // above and still break doc 01 §4 rule 5 in `spec/lexicon.src.tsv`.
    assert!(
        !ts.iter().any(|t| t == "पाठः"),
        "`पाठः` is back in the grammar; ADR-0029 retired it and the lexicon \
         cannot carry two words for `string`"
    );
}

#[test]
fn the_punctuation_is_exactly_what_adr_0003_ratified() {
    // Six signs, and no seventh. T1 may not quietly acquire a delimiter that
    // T0 was refused.
    let ts = terminals();
    for sign in ["।", "॥", "॰", "ॱ", "ॱॱ", "ऽ"] {
        assert!(ts.iter().any(|t| t == sign), "`{sign}` is not named");
    }
}

#[test]
fn the_phrase_structure_is_frozen_on_a_ratified_basis_and_so_are_the_operators() {
    // WAS `the_phrase_structure_is_absent_and_says_why`, which asserted `block`
    // did not exist at all. That guard was right for as long as the spelling was
    // an open question, and it CAUGHT this change — B-079b's freeze failed it
    // before landing, which is what a guard is for.
    //
    // It is replaced rather than deleted, and the replacement is STRICTLY
    // STRONGER. The old assertion would have accepted any spelling the moment
    // the ban lifted; this one requires each construct to use the exact pair
    // ADR-0003 ratified. The freeze is legitimate because ADR-0003 (accepted)
    // had already replaced these three roles' unavailable signs with words on
    // A-033's measurement — the owner's 2026-08-25 ruling removed A-063 as a
    // precondition, it did not choose the spelling.
    let text = grammar();

    // Each construct is frozen, and spelled the way the ADR says.
    for (production, open, close) in [
        ("group", "आरभ्य", "समाप्तम्"),
        ("block", "आदि", "इति"),
        ("index", "अङ्कः", "अन्तः"),
    ] {
        let line = text
            .lines()
            .find(|l| l.starts_with(production))
            .unwrap_or_else(|| panic!("`{production}` is not frozen; B-079b froze it"));
        assert!(
            line.contains(open) && line.contains(close),
            "`{production}` must use ADR-0003's pair `{open} … {close}`, got: {line}"
        );
    }

    // `statement`, `call` and `struct_body` ARE STILL DEFERRED. `expression` is
    // NOT — see `the_operator_set_is_the_corpus_set_in_both_directions` below,
    // which replaces the assertion that used to stand here. That assertion read
    // `for production in ["expression", "call", "struct_body"]` and was right
    // for exactly as long as the spelling was an open question; ADR-0032 closed
    // it by measuring the corpus, so the guard is narrowed rather than dropped.
    // NARROWED AGAIN 2026-08-30 by ADR-0027, which defined `statement` and
    // `call_expr`. What is left deferred is DECLARATION BODIES, and `enum_body`
    // joins the list because `गणना` opens with `आरभ्य` exactly as `संरचना`
    // does, so the two are one question and were never two. The guard is
    // narrowed rather than dropped for the third time, on the same reasoning
    // as the first two: a `for` list that empties is a guard that has stopped
    // guarding, and this one still has a subject.
    for production in ["struct_body", "enum_body"] {
        assert!(
            !text.contains(&format!("\n{production}")),
            "`{production}` was frozen without saying which ADR permits it"
        );
    }
    // And the ones that ARE frozen must be here, so a later edit cannot delete
    // a production and leave the narrowing above reading as if it never existed.
    for production in ["statement", "type", "call_expr", "argument_list"] {
        assert!(
            text.contains(&format!("\n{production}")),
            "`{production}` was frozen by ADR-0027 and is now missing"
        );
    }
    assert!(
        text.contains("ADR-0027"),
        "the decision that permits the statement freeze is named"
    );
    assert!(text.contains("A-063"), "the study is still named");
    assert!(text.contains("0 of 27"), "the measurement is cited");
    assert!(
        text.contains("ADR-0003"),
        "the ratification that permits the freeze is named, so the next reader \
         can check it rather than take the freeze on trust"
    );
}

/// Every whole-token word of a `.t1` source, comments and string literals
/// removed first so prose and message text cannot masquerade as code.
fn corpus_tokens() -> std::collections::HashMap<String, usize> {
    let dir = root().join("crates/sadhana-t1/src");
    let mut counts: std::collections::HashMap<String, usize> = Default::default();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read crates/sadhana-t1/src")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    files.sort();
    assert!(
        files.len() >= 15,
        "the `.t1` corpus is {} files; it was 15 when ADR-0026 measured it",
        files.len()
    );
    for f in files {
        let text = std::fs::read_to_string(&f).expect("read a `.t1` source");
        for line in text.lines() {
            // ADR-0011: `॰` opens a comment that runs to end of line.
            let mut code = line.split('॰').next().unwrap_or("").to_string();
            // ADR-0017: `उक्तम् … इति` is a string; its text is not code.
            while let Some(open) = code.find("उक्तम्") {
                match code[open..].find("इति") {
                    Some(close) => code.replace_range(open..open + close + "इति".len(), " "),
                    None => {
                        code.truncate(open);
                        break;
                    }
                }
            }
            for tok in code.split_whitespace() {
                let tok = tok.trim_matches(|c| c == '।' || c == '॥');
                if !tok.is_empty() {
                    *counts.entry(tok.to_string()).or_default() += 1;
                }
            }
        }
    }
    counts
}

/// Every whole-token word of a `.t1` source that stands in a TYPE POSITION,
/// counted separately for the two markers that introduce one.
///
/// `corpus_tokens` cannot answer ADR-0029's question. It counts a word wherever
/// it stands, and `पाठ` is a parameter name, a field name and a value far more
/// often than it is a type — 145 occurrences of which only 27 are types. A
/// count that does not know where it is looking measured the wrong thing by a
/// factor of four.
///
/// Returns `(after_annotation, after_dadati)`. The first is the position
/// `binding` FREEZES (`annotation , type`, `:603`); the second is the corpus's
/// return marker, which no production names yet — kept apart for that reason
/// rather than summed.
fn corpus_type_positions() -> (
    std::collections::HashMap<String, usize>,
    std::collections::HashMap<String, usize>,
) {
    const MEMBER: &str = "ॱ";
    let dir = root().join("crates/sadhana-t1/src");
    let mut after_annotation: std::collections::HashMap<String, usize> = Default::default();
    let mut after_dadati: std::collections::HashMap<String, usize> = Default::default();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read crates/sadhana-t1/src")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    files.sort();
    for f in files {
        let text = std::fs::read_to_string(&f).expect("read a `.t1` source");
        for line in text.lines() {
            let mut code = line.split('\u{0970}').next().unwrap_or("").to_string();
            while let Some(open) = code.find("उक्तम्") {
                match code[open..].find("इति") {
                    Some(close) => code.replace_range(open..open + close + "इति".len(), " "),
                    None => {
                        code.truncate(open);
                        break;
                    }
                }
            }
            // `ॱ` and `ॱॱ` are written with no space around them 229 times, so
            // whitespace alone does not find them. The sentinel keeps the
            // annotation whole while the member mark is split out from under it.
            let annotation = format!("{MEMBER}{MEMBER}");
            let code = code
                .replace(&annotation, "\u{1}")
                .replace(MEMBER, &format!(" {MEMBER} "))
                .replace('\u{1}', &format!(" {annotation} "));
            let toks: Vec<&str> = code
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥'))
                .filter(|t| !t.is_empty())
                .collect();
            for pair in toks.windows(2) {
                let bucket = match pair[0] {
                    a if a == annotation => &mut after_annotation,
                    "ददाति" => &mut after_dadati,
                    _ => continue,
                };
                *bucket.entry(pair[1].to_string()).or_default() += 1;
            }
        }
    }
    (after_annotation, after_dadati)
}

#[test]
fn the_string_type_is_written_patha_and_the_count_is_positional() {
    // ADR-0029, task `D-002h`. ADR-0027 recorded `पाठ` 145 against `पाठः` 39
    // "both in type position" and left `type_name` spelling the minority. Both
    // halves of that are corrected here: the position is measured, and the
    // production follows the result.
    let (annot, dadati) = corpus_type_positions();

    // Where the grammar already speaks — `binding`'s `annotation , type` — the
    // corpus is not close to divided.
    pin_report!(
        (annot.get("पाठ").copied(), annot.get("पाठः").copied()),
        // MOVED BY THE MERGE OF 2026-09-01, AND THE MOVE IS DEBT, NOT PROGRESS.
        // This read (26, 2). `land-varna` carried implementations that
        // `origin/main` did not — `artha.t1` alone contributes 500+ lines whose
        // stubs are all this ledger's corpus had — and that code was written
        // before ADR-0029 froze `पाठ`, so it spells `पाठः` in annotated
        // position 11 more times. The direction ADR-0029 chose is unchanged;
        // what changed is how much of the corpus has not yet been brought to
        // it. NOT remediated here on purpose: that sweep is `D-002h`'s, and a
        // merge commit is the wrong place to rewrite 13 type annotations.
        // (23, 13) -> (20, 13) on 2026-09-04, `W-247`: three `पाठ` annotations
        // GONE with `encode.t1`'s dead five-field `प्रतीक्षा` (नाम, भेद,
        // कोष्ठकनाम); the `पाठः` count is untouched. MEASURED here.
        // (20, 13) -> (28, 13) on 2026-09-05, `W-248`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE and not computed: the type-text reader
        // (`प्रकारपाठार्थः`, `प्रकारपाठारम्भः`, `प्रकारपाठशेषः`, `पाठप्रकारार्थः`)
        // annotates its own parameters and returns `पाठ`, the FROZEN spelling,
        // eight times. `पाठः` is unmoved at 13 — no retired spelling was added,
        // and the row's own fix was to teach the reader `पाठ`, which it had
        // never recognised though ADR-0029 froze it on 2026-08-31.
        // (28, 13) -> (38, 13) on 2026-09-05, `W-265`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE. The type-NAME resolver annotates ten more
        // bindings `पाठ`, the frozen spelling: the seven routines that take a
        // spelling (`प्रकारसंज्ञायोजनम्`, `प्रकारसंज्ञान्वेषणम्`,
        // `प्रकारसंज्ञासंख्या`, `प्रकारसंज्ञाप्रथमम्`, `प्रकारपाठनिषेधः`,
        // `नामप्रकारार्थः`, `मण्डलप्रकारपाठार्थः`) and the three locals that
        // hold one while a qualified name is split (`उपसर्गः`, `सदस्यपाठ`,
        // `प्रकारनाम`). `पाठः` is unmoved at 13: no retired spelling was added,
        // and this row wrote none.
        (Some(38), Some(13)),
        "the corpus's annotated bindings moved; ADR-0029 froze `पाठ`, and this \
         ledger records how much of the corpus still spells `पाठः`"
    );
    // And in the return position, which no production names yet, it runs the
    // other way. Pinned so that the ADR's honesty about it cannot rot: this is
    // the seven sites ADR-0029 declines to call a defect.
    pin_report!(
        (dadati.get("पाठ").copied(), dadati.get("पाठः").copied()),
        // (1, 7) BEFORE THE MERGE OF 2026-09-01. Six of those seven return
        // positions were in routines `origin/main` carried as stubs and
        // `land-varna` carried as code; the merge kept the code, and the
        // implemented versions do not return a text. So this is not the ADR
        // being obeyed — it is the sites simply not being there any more, and
        // recording it as a win would be the misreading this comment prevents.
        // (1, 1) -> (2, 1) on 2026-09-04, `W-236`: `yantrotsarjana.t1`'s
        // `यन्त्रषोडशाङ्कचिह्नम्` — hex64's sixteen digit glyphs, the twin of
        // `utsarjana.t1`'s `अङ्कचिह्नम्` — returns a text, and is written
        // `ददाति पाठ`, the spelling ADR-0029 froze, not the `पाठः` its sibling
        // still carries. One site added in the ADR's direction; none moved.
        // (2, 1) -> (4, 1) on 2026-09-04, `W-245`: `yantrotsarjana.t1`'s
        // `यन्त्रशाखापदम्` (ADR-0008's six branch words) and
        // `यन्त्रद्विपदक्रियापदम्` (the ten operator verbs) each answer a text,
        // written `ददाति पाठ` as their sibling is. Two sites added in the
        // ADR's direction; none moved.
        // (4, 1) -> (5, 1) on 2026-09-05, `W-248`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE: `प्रकारपाठशेषः` answers `ददाति पाठ` — the
        // tail of a wrapped spelling is itself a type text — and it is the one
        // new return position this row wrote. `पाठः` unmoved at 1.
        (Some(5), Some(1)),
        "the corpus's return types moved"
    );

    // `बूल`, the type name nobody disputes, is the control: it is written one
    // way in both positions, which is what a settled spelling looks like.
    //
    // The COUNTS moved with the merge of 2026-09-01 while
    // the SPELLING did not (36 -> 83 annotated, 41 -> 65 returned), and that is
    // the control doing its job: a corpus
    // that grew by ~500 implemented lines moves every census in this file, so
    // a `पाठ`/`पाठः` count that moved is not by itself evidence that anything
    // regressed. What would be evidence is `बूल` acquiring a second spelling,
    // and it has not.
    // 83 -> 88 with the parameter-loop, index and call work of 2026-09-01:
    // five more `बूल` locals, all loop guards. Corpus growth, as above.
    // 94 -> 95 on 2026-09-02: `अर्थॱप्रकारदोषमस्ति`, the type checker's
    // first-refusal guard. The control is doing exactly its job — the count
    // moved because a `बूल` was declared, and the SPELLING did not move.
    // 95 -> 98 on 2026-09-02: juxtaposed application brought three `बूल`
    // bindings — `तर्कारम्भः`'s `योग्यम्`, and the two loop guards `जुक्तचालु`
    // and `तर्कपश्चात्`. The control is doing its job: the count moved because
    // bindings were declared, and the SPELLING did not move.
    // 98 -> 101 on 2026-09-03: `गणनापठनम्`'s `सार्वजनिकत्व` parameter and its
    // two `अवगणना`/`अवगणनविभाग` consume-and-discard guards.
    // 101 -> 105 on 2026-09-04, `W-202`: FOUR `बूल` bindings, named — the
    // resolver's `शरीरफलम्` (the else branch checks the then-body's result
    // before walking `अन्यसूचकाङ्क`), the checker's two `शर्तबूल` condition
    // guards in the `यदि` and `यावत्` arms, and the समूह fold's `दुष्टम्`. The
    // control is doing its job: the count moved because bindings were declared,
    // 101 -> 105 on 2026-09-04, `W-204`, MEASURED (the test printed it) and
    // reconciled by name: `ir.t1`'s refusal record `अनिर्णीताह्वानमस्ति`, the
    // call arm's chain-walk guard `अन्वेषणचालु` and its two verdicts
    // `स्वीकृतम्` / `आह्वेयमस्ति`. (This read 106 on the branch while it also
    // carried the resolver's else-walk and its `शरीरफलम्`; that walk is
    // W-202's and went back to it, and the count followed — measured again,
    // not subtracted.) The control is doing its job: four bindings declared,
    // and the SPELLING did not move.
    // W-202 + W-204 on one tree: W-204 named four (`अनिर्णीताह्वानमस्ति`,
    // `अन्वेषणचालु`, `स्वीकृतम्`, `आह्वेयमस्ति`) and W-202 four more (`शरीरफलम्`,
    // two `शर्तबूल`, the fold's `दुष्टम्`). MEASURED here, never added.
    // 105 -> 108 on 2026-09-04 (W-228): `खण्डसीमापठनम्`'s loop guard `सीमाचालु`
    // and the two qualifier-fold flags `मण्डलमस्ति`/`तर्कमण्डलमस्ति`. Bindings
    // declared; the SPELLING did not move.
    // MEASURED after merging W-204 (105) and W-226/W-228 (+3): 108.

    // MERGE of W-202 (left) and W-215 (right) on 2026-09-04 by the trunk: the value below is
    // MEASURED on the merged tree from this assertion's own failure, never summed.

    // 108 -> MEASURED (see below) on 2026-09-04, `W-215`, MEASURED (the test printed it) and
    // reconciled by name: `parse.t1`'s `अवगणनयुग्मदण्ड` (the `॥` after a module
    // name), `अवगणनचिह्न` and the struct reader's `अवगणनविभाग` (the field loop
    // that now keeps fields), and `unparse.t1`'s `अमुद्रणीयमस्ति` (the printer's
    // first-refusal guard) and `अन्वेषणचालु` (the call arm's chain walk). The
    // control is doing its job: five bindings declared, the SPELLING unmoved.

    // 108 -> 113 on 2026-09-04, `W-215` on the merged tree, MEASURED (the test
    // printed it): the five named in the W-215 note above, and nothing else.
    // 113 -> 117 on 2026-09-04, the MERGE of W-202 (112 on its tree: the four it named) with
    // W-215 (113 on its tree: the five it named), MEASURED on the merged tree (the test
    // printed it): 108 + 4 + 5 = 117 — both notes above list their bindings, none shared.
    // MERGE of main (left: W-202 + W-215) with agent/w227 (right: W-239 part 1's arena) on 2026-09-04
    // by the trunk: the value below is MEASURED on the merged tree from this assertion's own
    // failure, never summed.
    // 108 -> 115 on 2026-09-04 (W-239): the octet arena's refusal flag
    // `अष्टकदोषमस्ति` and the six consume-and-discard guards its readers bind
    // on a refusal (`अवगणना`/`अवगणनद्वि` in three routines). Bindings declared;
    // the SPELLING did not move.
    // 117 (main: W-202 + W-215) + 7 (W-239 part 1, the octet arena, named in its note) = 124,
    // MEASURED on the merged tree 2026-09-04 (the test printed it).
    // MERGE of main (left: through D-002a2) with agent/w236 (right: the T1 emitter twin) on 2026-09-04
    // by the trunk: the value below is MEASURED on the merged tree from this assertion's own failure,
    // never summed.
    // 105 -> 113 on 2026-09-04, `W-236`, MEASURED (the test printed it) and
    // reconciled by name — eight `बूल` bindings, all in `yantrotsarjana.t1`,
    // the T1 RISC-V emitter: the refusal record's `यन्त्रनिषेधमस्ति`; hex64's
    // leading-zero guards `आरब्धम्` and `लेख्यम्`; `lower_constant`'s 12-bit
    // test `लघु`; `check_labels`'s `सङ्घट्टः`; `verify`'s Call-seen flag
    // `आहूतम्`; `emit_function`'s label guard `चिह्नितम्`; the branch-range
    // check's `बाह्यम्`. The control is doing its job: bindings declared, and
    // the SPELLING did not move.
    // MEASURED 2026-09-04 on the merged tree (W-226/W-228's 108 + W-236's 8): 116.
    // 124 (main through D-002a2) + 8 (W-236: the twin's and the bias step's, named in its note) =
    // 132, MEASURED on the merged tree 2026-09-04 (the test printed it).
    // 132 -> MEASURED below (was 117 -> 119 on the branch) on 2026-09-04, `W-240`, MEASURED (the test printed it): the two
    // `अवगणनदण्ड` guards — `संरचनापठनम्` and `गणनापठनम्` consuming their closing
    // `।`. Bindings declared, the SPELLING unmoved.
    // 132 (main through W-244's script) + W-240's two `अवगणनदण्ड` guards = 134,
    // MEASURED on the merged tree 2026-09-04 (this assertion printed it).
    // MERGE of main (left: through W-243) with agent/w231 (right: the checker's four arms) on
    // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
    // own failure, never summed.
    // 134 (main through W-243) + W-231's (named in its note; 132 -> 136 on its own tree) = 138,
    // MEASURED on the merged tree 2026-09-04 (the test printed it).
    // MERGE of main (left: through W-244) with agent/w223 (right: the declaration store) on
    // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
    // own failure, never summed.
    // 134 -> 136 on 2026-09-04, `W-223` part 1: `sanchaya.t1`'s `सञ्चयदोषमस्ति`
    // (the refusal flag) and the entry record's `सार्वजनिकत्व`; MEASURED here.
    // 138 (main through W-244) + W-223 part 1's (its own tree read 134 -> 136) = 140, MEASURED on
    // the merged tree 2026-09-04 (the test printed it).
    // MERGE of main (left) with agent/w223b (right: the extent) on 2026-09-04 by the trunk:
    // the value below is MEASURED on the merged tree from this assertion's own failure.
    // MERGE of main (left: through W-244) with agent/w223 (right: part 1, the
    // shared declaration store) on 2026-09-04 by W-223 part 2. 138 (main, with
    // W-231's arms) and 136 (part 1, with `सञ्चयदोषमस्ति` and the entry record's
    // `सार्वजनिकत्व`) are SIBLINGS off 134 — neither counted the other's two.
    // -> 140, MEASURED here from this assertion's own failure, never summed.
    // 140 -> 141 on 2026-09-04, `W-223` part 2: ONE `बूल` added, `परिधिदोषमस्ति`
    // — the flag saying `कार्यक्रमनिर्णयः`/`कार्यक्रमप्रकारपरीक्षा` were handed an
    // extent they could not walk. MEASURED from this assertion's own failure.
    // 141 -> 144 on 2026-09-04, `W-223` part 2: THREE `बूल` added, and they are
    // exactly the three flags this row introduces — `सञ्चयसिद्धमस्ति` (a collection
    // pass ran over this interpreter), `असदस्यमस्ति` (the named module declares no
    // such member) and `अप्रकार्यवाक्यमस्ति` (the first untypable statement has been
    // located). That the count moved by exactly the flags written is a CHECK on
    // the change. MEASURED from this assertion's own failure.
    // 144 -> 145 on 2026-09-04, `W-247`: `पुनर्घोषणामस्ति`, the redeclaration flag
    // beside `अनिर्णीतमस्ति` — a sibling of part 2's three, not a successor.
    // MEASURED from this assertion's own failure.
    // 145 and 152 are SIBLINGS off 144 — main's `पुनर्घोषणामस्ति` and the other
    // lane's eight — and 153 is what the MERGED tree measures, from this
    // assertion's own failure. Never summed.
    // 153 -> 154 on 2026-09-05, `W-248`, MEASURED from this assertion's own
    // failure: `अज्ञातप्रकारपाठमस्ति`, the flag the type-text reader raises when
    // it meets a spelling it does not know. One flag, one binding — a sibling
    // of `अनिर्णीतमस्ति` and `पुनर्घोषणामस्ति` and named the same way, so that
    // the count moving by exactly one is itself the check on the change.
    // 154 -> 156 on 2026-09-06, `W-254`, MEASURED from this assertion's own
    // failure, and the move is exactly the two flags the row writes — which is
    // what makes the count a CHECK rather than a tax. `ir.t1`'s `द्विगुणम्`,
    // raised when a string literal's content holds ADR-0011's doubled `इति` so
    // the pair collapses to one word, and `yantrotsarjana.t1`'s `समानम्`, which
    // says a blob already in the string pool is octet-for-octet the one in
    // hand. Two flags, two files, two bindings; a third would have meant
    // something else moved.
    // 156 -> 157 on 2026-09-06, `W-kosha` phase 4, MEASURED from this
    // assertion's own failure, and the move is exactly the one flag the fix
    // writes: `कारकपदपठनम्`'s `सङ्ख्यात्वम्`. That routine passed a HARDCODED
    // `असत्यम्` for is_numeral, so every operand it read was recorded as
    // not-a-numeral and `सङ्केतन` refused the program with E02. It asks
    // `अक्षरकोशॱअंशदोषः` now, and the answer needs a binding.
    //
    // One flag, one binding. A second would mean something else moved.
    //
    // 157 -> 158 on 2026-09-07, THE BAD-FIELD REFUSAL, MEASURED from this
    // assertion's own failure. One flag, and it is `असत्क्षेत्रमस्ति`: the क्षेत्र
    // arm of `अभिव्यञ्जकप्रकारः` had a margin claiming it refused a field no
    // struct declares and no flag behind it, so the refusal was recorded
    // nowhere and the program linked with status ०. THE OTHER FOUR MEMBERS OF
    // THAT FAMILY ARE NOT बूल — a count, two texts and a line — so a move of
    // more than one here would mean something outside this change moved.
    // 158 -> 159 on 2026-09-07, the driver merge, TAKEN FROM THIS ASSERTION'S
    // OWN FAILURE. `shrinkhala.t1` binds one flag as it walks the stage sequence.
    // 160 -> 161 on 2026-09-07: `वस्तुरचनानिषेधः`, the flag beside the object
    // builder's refusal reason. It exists because `भवति ०` on an octet-run
    // global leaves `Int(0)` and asking THAT for `ॱ दैर्घ्य` faults — so
    // "was there a refusal?" cannot be answered by the reason's length, and
    // comparing the run to ० would go through an equality that does not
    // typecheck its operands and answers false in silence.
    // 161 -> 162 on 2026-09-09, THE SYMBOL-MAPPING ROW, TAKEN FROM THIS
    // ASSERTION'S OWN FAILURE. `निर्णायकसिद्धः`, the flag saying whether the
    // held resolver is the one for the source in hand — `निर्णयः` answers `बूल`
    // and drops the resolver it built, so the driver needs a second global to
    // say the slot is filled. ONE flag, not two: the pair was declared twice on
    // this branch after a clean merge took both branches' declarations, and the
    // duplicate was removed the same day. This number counted 163 before that.
    // 162 -> 163 on 2026-09-09, THE `batch-1` PIN RE-TAKE, TAKEN FROM THIS
    // ASSERTION'S OWN FAILURE. ONE flag and it is `सफलम्`, bound at
    // `shrinkhala.t1:550` — `चरः सफलम् ॱॱ बूल भवति असत्यम् ।` — by the driver
    // fold on `agent/lexrung`. `agent/xmodule` contributes ZERO here.
    //
    // MEASURED MEMBER-WISE, NOT INFERRED FROM THE COUNT: the added-name set
    // over `4812ac9c` is exactly {`सफलम्`} and no `बूल` was removed. A
    // margin-filtered grep of `ॱॱ बूल` declarations reproduces this pin
    // ABSOLUTELY — 162 on `4812ac9c`, 163 on the batch — so it is counting this
    // bucket's population and not a quantity merely correlated with it.
    //
    // THE GUARD ABOVE IS WHY THIS IS A RE-TAKE AND NOT A DEFECT: it says a move
    // of more than one here would mean something outside the change moved. It
    // moved exactly one, and that one has a name and a line.
    //
    // THIS 163 IS NOT THE 163 THE PARAGRAPH ABOVE RECORDS. That one was a pair
    // declared twice after a clean merge took both branches' declarations, and
    // the duplicate was removed the same day. This one is a single new flag.
    // A future reader arriving at 163 cannot tell them apart by the number —
    // THE DISCRIMINATOR IS THE MEMBER SET: check that `सफलम्` is present and
    // that no name is declared twice.
    // 163 -> 170 ON 2026-09-12, THE STORAGE MODEL'S THREE LANDINGS, RE-TAKEN BY
    // THE TRUNK — and this pin had been red on main since `f444b6dd` without
    // anyone reading it, because the lane that would have seen it was gating a
    // different crate. The move is spread over three commits and NOT over one:
    // `303ff770` 163 · `f444b6dd` 166 (`W-284`) · `6784e31e` 169 (`W-285`) ·
    // `5b58052e` 170 (`W-287`). A single re-take hides that; the four readings
    // are here so the next reader can tell a drift from a step.
    //
    // MEASURED MEMBER-WISE, AS THE PARAGRAPH ABOVE DEMANDS, AND THE SET IS
    // EXACTLY SEVEN WITH NOTHING REMOVED: `ir.t1`'s `खण्डमस्ति`,
    // `स्थानमात्रम्` and `स्थानमात्रमिदम्` (the address-only flag ruled on
    // 2026-09-11, its captured local, and the slice guard), `shrinkhala.t1`'s
    // `रचनाकश्चित्`, and `yantrotsarjana.t1`'s `यन्त्ररचनामस्ति` global plus
    // `रचनामस्ति` TWICE. **The duplicate is the thing this margin says to
    // check, and it is not one**: `:1930` and `:2033` are two routines'
    // PARAMETERS, `यन्त्रारम्भोत्सर्जनम्` and `यन्त्रारम्भमण्डलोत्सर्जनम्`,
    // not a name declared twice after a merge took both sides.
    //
    // THE INSTRUMENT WAS VALIDATED BEFORE IT WAS BELIEVED. A margin-filtered
    // `ॱॱ बूल` count reproduces the ASSERTION'S OWN FAILING VALUE — 170 at
    // `e1717d28` — and the last-green 163 at `303ff770`, so it is counting this
    // bucket's population and not a quantity correlated with it. That check is
    // the only reason the seven names above can be trusted to be the whole set.
    // 170 -> 171 ON 2026-09-12, `W-293`: ONE binding, `ir.t1`'s `वैश्विकपाठ्यम्`,
    // the two-test flag the widened global-read arm needs because `वा` survives
    // UNATTESTED in the frozen grammar (`spec/grammar-t1.ebnf:658`) and reaching
    // for it would have made a lowering fix into a grammar landing.
    //
    // **THE ROW PREDICTED THIS PIN WOULD NOT MOVE AND CORRECTED ITSELF BEFORE THE
    // RUN.** The registered form was "`बूल` UNCHANGED at 170; if it moves I
    // accidentally added a boolean and the count-over-flag decision did not hold"
    // — which was testing the wrong quantity. The decision was about the ARENA's
    // representation, and the arena is `न६४` carrying octets; this `बूल` is a
    // local flag and unrelated to it. Corrected before the run rather than
    // explained after, because a prediction too blunt to distinguish the thing it
    // was written for is a DIFFERENT prediction, not a weak one.
    // 171 -> 172 ON 2026-09-13, cause 22 `assign_name` (`agent/shared-arena-witness`,
    // `808580cf`): ONE binding, `ir.t1`'s `लक्ष्यलेख्यम्`, the two-test flag the
    // global-STORE arm needed so it accepts exactly what the global-READ arm
    // accepts — `W-284`'s "a read and its write must refuse the same set", and
    // the same `बूल`-instead-of-`वा` shape as `वैश्विकपाठ्यम्` one entry above,
    // for the same reason (`वा` unattested in the frozen grammar). MEASURED
    // member-wise from this assertion's failure on the merged tree: the net
    // `ॱॱ बूल` diff over `2d41aa25..e847d227` is exactly that name. Re-taken by
    // the trunk as part of the merge, not by the lane, because this crate is
    // outside the census that certified the lowering — the same gap that let
    // `5dd5def0` push the `न६४` triple red an hour earlier.
    // 172 -> 173 ON 2026-09-13, the embed store (trunk, `lex.t1`): ONE binding,
    // `सारणीमिलितम्`, the found-flag of the table lookup in `पदविभाग`'s embed
    // collapse — a `बूल` because `वा` is unattested and a loop that must stop
    // on a hit without `वा` needs a flag. MEASURED from this assertion's failure.
    // Some(174) -> Some(175) on 2026-09-13, the self-image's entry (शृङ्खला: प्रवेशन्यासः, स्वपरीक्षा, four globals); MEASURED from this assertion's own failure.
    // Some(175) -> Some(176) on 2026-09-13, आरम्भपाठ्यरचना factored out of मण्डलानिप्रतिबिम्बम्; MEASURED from this assertion's own failure.
    pin_report!(annot.get("बूल").copied(), Some(176));
    // AND FROM THE OTHER LANE, whose bindings this tree also carries:
    // 144 -> 152 on 2026-09-04, `W-245`, MEASURED on the merged tree from
    // this assertion's own failure (144 on main and 148 on `agent/w245` are
    // SIBLINGS off 140 — main's four W-223-part-2 flags and this row's eight
    // — and the sum is a prediction the assertion then confirmed, not the
    // value). EIGHT `बूल` bindings declared and NONE respelled: `ir.t1`'s
    // `विपर्ययः` (`अधिकम्` is न्यून with its operands exchanged),
    // `yantrotsarjana.t1`'s six (the compare lowerer's four arm flags
    // `सामान्यम्`/`न्यूनवत्`/`अचिह्नवत्`/`विपर्यस्तम्`, the local-slot test
    // `स्थानवत्`, the fused-compare walk's `पठितम्`) and `utsarjana.t1`'s
    // `द्विपदम्`, the allocator's range test over the new kinds.
    // The control is doing its job: bindings declared, the SPELLING unmoved.
    // 65 -> 67 on 2026-09-02: the two NON-CONSUMING predicates juxtaposed
    // application needed — `तर्कारम्भः` (can this token begin an argument)
    // and `द्विकर्मपाठः` (is this text a binary operator). Both had to be
    // separate from `मेलनम्`/`द्विकर्ममेलनम्`, which ADVANCE on match: a stop
    // test that consumes the token it is testing cannot be asked twice.
    // 67 -> 71 on 2026-09-04, `W-202`: FOUR routines answering `बूल`, named —
    // `तुलनाद्विकर्म` and `सरणद्विकर्म` (is this binary operator a comparison, is
    // it a shift — the partition read off `compare_op`/`shift_op` in the frozen
    // grammar rather than decided in the checker), `अङ्कपदम्` (is this operand a
    // numeral literal, for the rule that a literal takes its context's type),
    // and `बूलार्थमस्ति` (is this type बूल — three field comparisons, because बूल
    // has no `Ty` of its own and is a width-१ unsigned integer). Corpus growth,
    // not a remediation: the SPELLING did not move.
    // 67 -> 68 on 2026-09-04 (W-228): `आयातितमण्डलम्`, the third non-consuming
    // predicate — is this token a module the file imports.

    // MERGE of main (left: W-202 + W-215) with agent/w227 (right: W-239 part 1's arena) on 2026-09-04
    // by the trunk: the value below is MEASURED on the merged tree from this assertion's own
    // failure, never summed.
    // 68 -> 70 on 2026-09-04 (W-239): `अष्टकनिषेधः` (records a refusal and
    // answers false) and `अष्टकस्थापनम्` (written or refused).
    // 72 (main: W-202 + W-215) + 2 (W-239 part 1) = 74, MEASURED on the merged tree 2026-09-04
    // (the test printed it).
    // MERGE of main (left: through D-002a2) with agent/w236 (right: the T1 emitter twin) on 2026-09-04
    // by the trunk: the value below is MEASURED on the merged tree from this assertion's own failure,
    // never summed.
    // 67 -> 74 on 2026-09-04, `W-236`: seven predicates of `yantrotsarjana.t1`
    // answer `ददाति बूल` — `यन्त्रसंयुक्तसाम्यम्` (two labels the same word),
    // `यन्त्रस्वचिह्नमस्ति` (an emitter-owned name), `यन्त्रचिह्नपरीक्षा`
    // (check_labels), `यन्त्रोच्चनीचविभागः` (split_hi_lo's Some/None),
    // `यन्त्रपरीक्षा` (verify), `यन्त्रपर्वबाह्यम्` (a target outside the
    // function), `यन्त्रशाखादूरपरीक्षा` (check_branch_ranges).
    // MEASURED 2026-09-04 on the merged tree (W-228's 68 + W-236's 7): 75.
    // 74 (main through D-002a2) + 7 (W-236's routines answering बूल, named in its note) = 81,
    // MEASURED on the merged tree 2026-09-04 (the test printed it).
    // 81 -> 84 on 2026-09-04, `W-223` part 1: `sanchaya.t1`'s three routines
    // answering बूल — `सञ्चयपाठसाम्यम्`, `आयातितम्`, `प्रविष्टिसार्वजनिकत्वम्`; MEASURED here.
    // 84 -> 85 on 2026-09-05, `W-259`: `ir.t1`'s `चिह्नकपाठसाम्यम्`, which answers
    // whether two tokens spell one name — how a parameter reference is matched.
    // 85 here and 86 on the other lane are siblings off 84; the merged tree
    // measures 86, from this assertion's own failure.
    // 86 -> 87 on 2026-09-05, `W-248`: `प्रकारपाठारम्भः`, which answers whether a
    // type text begins with a wrapper's spelling — the one predicate the reader
    // needs and the only routine this row added that returns बूल. MEASURED from
    // this assertion's own failure.
    // 87 -> 88 on 2026-09-07, `W-chain`: `शृङ्खलाॱनिर्णयः`, the one routine the
    // DRIVER adds that ANSWERS `बूल` — resolve-and-typecheck, which the chain
    // must be able to ask about separately because it is the stage that
    // refuses a program without producing anything to look at. MEASURED from
    // this assertion's own failure.
    //
    // NOTE THE PAIRING, WHICH IS THE CHECK DOING ITS WORK: this row moves the
    // ANNOTATION census by one (`Some(157)` -> `Some(158)`, the binding that
    // holds the answer) and this RETURNS census by one (the signature), and
    // both come from that single routine counted in the two places a `बूल`
    // can appear. One moving without the other would mean a routine that
    // answers a flag nobody binds, or binds one nobody answers.
    // 88 -> 89 on 2026-09-07: `वैश्विकत्वम्`, the one routine the object builder
    // adds that ANSWERS `बूल` — whether a name is one of the program's
    // `॥ वैश्विकम् ॥` pushes, which decides STB_GLOBAL and so whether another
    // object's link may resolve it.
    //
    // PAIRED WITH THE ANNOTATION COUNT, as this row's sibling always is: that
    // moved 160 -> 161 for `वस्तुरचनानिषेधः`, a DIFFERENT declaration. So the
    // two figures moved by one each for two unrelated reasons here, where the
    // driver moved both by one for a single routine. A pairing that holds for
    // two causes is not the same evidence as one that holds for one, and the
    // margin should not imply otherwise.
    // 89 -> 90 ON 2026-09-12, THE SAME THREE STORAGE LANDINGS, AND IT FIRED
    // SECOND — the annotation pin above had to be re-taken before this one
    // became reachable, exactly as the triple's margin at the ADR-0030 ledger
    // says to expect. **A FAILURE NAMES WHAT TRIPPED FIRST, so the first red is
    // a LOWER BOUND on how many pins a corpus change moved**, and a row that
    // moves typed bindings should expect two reds from one test and stop only
    // after the second.
    //
    // ONE ROUTINE, NAMED, AND NOTHING REMOVED: `yantrotsarjana.t1`'s
    // `यन्त्ररचनाप्रश्नः` — the emitter-side twin of `module_allocates`, the
    // predicate asking whether an image allocates. It is a `बूल` ANSWER, not a
    // `बूल` binding, which is why it lands in this bucket and not in the
    // annotation one above; the two moved for entirely different declarations.
    //
    // VALIDATED BEFORE BELIEVED, like its sibling: the twin behind this name
    // reproduces 89 at `303ff770` and 90 at `e1717d28`, the assertion's own
    // failing value.
    // 90 -> 93 on 2026-09-13, the encoder's table index: THREE routines that
    // ANSWER `बूल`, all in `encode.t1` — `परिधिसाम्यम्` (a range equals a name),
    // `सूचितकुलपङ्क्तिवत्` and `सूचितांशसाम्यम्` (the family and width tests on
    // an indexed row). MEASURED from this assertion's own failure.
    pin_report!(dadati.get("बूल").copied(), Some(93));
    // AND FROM THE OTHER LANE, whose bindings this tree also carries:
    // 84 -> 86 on 2026-09-04, `W-245`, MEASURED: two routines that ANSWER `बूल` —
    // `यन्त्रद्विपदभेदः` (is this kind one of the ten binary kinds) and
    // `यन्त्रान्यत्रपठितम्` (is the condition read anywhere but its branch).

    // `अक्षरम्` and its stem are absent from BOTH positions, so ADR-0027's
    // reason for leaving it alone still holds: an unused spelling contradicts
    // nothing.
    //
    // `न६४` LEFT THIS LIST when the ADR-0030 remediation began. It was here on
    // the strength of being frozen-and-unused; it is now written 56 times in
    // `ir.t1`, and the count that replaces this assertion is in
    // `the_integer_prefixes_are_the_ones_doc_02_derives` below, beside the two
    // it is draining.
    for unused in ["अक्षरम्", "अक्षर", "प३२", "प६४"] {
        pin_report!(
            (annot.get(unused), dadati.get(unused)),
            (None, None),
            "`{unused}` is now written in type position; ADR-0027 and ADR-0029 \
             both left it frozen-and-unused, and that is no longer true"
        );
    }

    // The one ADR-0029 names and does not settle. ADR-0030 settles it and
    // REFUSES the widening; the count is kept here because that ADR's finding
    // is a count, and `the_integer_prefixes_are_the_ones_doc_02_derives` below
    // is where the ruling itself is held.
    let i64_sites =
        annot.get("इ६४").copied().unwrap_or(0) + dadati.get("इ६४").copied().unwrap_or(0);
    // 50 BEFORE THE MERGE OF 2026-09-01; 69 after, for the reason recorded in
    // full at the ledger below — the merge kept `land-varna`'s implemented
    // routines over `origin/main`'s stubs, and the implementations carry their
    // own `इ६४` sites. Debt acknowledged, not discharged; the sweep is D-002h's.
    // 54 AFTER THE MERGE OF 2026-09-03, MEASURED. The rail's side of that merge
    // reached ZERO on its own corpus (its `encode.t1` moved the last 15); this
    // branch's newer routines carry 54 in type position. The message below is
    // the rail's and describes ITS corpus; on the merged tree the number is a
    // ratchet that may only go DOWN. See `the_signed_half_of_adr_0030_is_a_ratchet`.
    pin_report!(
        i64_sites,
        54,
        "`इ६४`'s type-position sites are the divergence ADR-0029 measured and \
         ADR-0030 ruled a port defect. It was 56, and IT IS NOW ZERO: \
         `ir.t1`'s 4, `vishlesana.t1`'s 2, `samyojana.t1`'s 30, \
         `utsarjana.t1`'s 4, `vastu.t1`'s 1 and finally `encode.t1`'s 15 are \
         remediated. THE SIGNED HALF OF ADR-0030 IS FINISHED — every `इ६४` \
         left under `crates/sadhana-t1/src/` is inside a `॰` comment about \
         the repair, and this number can now only go UP, which would mean a \
         new site spelled with a prefix ADR-0030 refused. The ledger is in \
         `the_integer_prefixes_are_the_ones_doc_02_derives` below"
    );
    println!(
        "METRIC t1_type_position_patha {}",
        annot.get("पाठ").copied().unwrap_or(0)
    );
}

/// Every `ॱ`-marked type suffix written in `spec/*.sas`, counted by prefix.
///
/// `grammar-t0.ebnf:97` is `type_suffix = member_mark , [ "अ" | "न" | "प" ] ,
/// decimal`, so the member mark is what tells a type suffix from the same two
/// signs standing for something else. Counted bare the answer is the same, and
/// counted over `drafts/` it is not — `drafts/segmenter.sas` writes `०षोड्इ६४१न`,
/// where `इ६४` is four hexadecimal digits and no kind of type.
fn t0_type_suffix_counts() -> std::collections::BTreeMap<char, usize> {
    let mut out: std::collections::BTreeMap<char, usize> = Default::default();
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join("spec"))
        .expect("read spec/")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "sas"))
        .collect();
    files.sort();
    for f in files {
        let text = std::fs::read_to_string(&f).expect("read a `.sas` source");
        for prefix in ['अ', 'न', 'प'] {
            for width in ["८", "१६", "३२", "६४", "१२८"] {
                let needle = format!("ॱ{prefix}{width}");
                *out.entry(prefix).or_default() += text.matches(&needle).count();
            }
        }
    }
    out
}

#[test]
fn the_integer_prefixes_are_the_ones_doc_02_derives() {
    // ADR-0030, task `D-002h`. `STATE.md` filed this as the largest hole left
    // in the frozen grammar, on the reading that `इ` is the real signed prefix
    // and `integer_type` is too narrow to spell it. It is not a hole. The
    // production stands and the corpus is what is wrong, in BOTH directions:
    // `इ६४` spells `i64`, and 1322 `अ६४` sites spell an unsigned quantity with
    // doc 02 §2.5's word for `i64`.
    // `इ` IS a terminal of this grammar and always was — `hex_digit` at :243
    // spells `c` with it, `अ` being `a`. So `०षोड्इ६४` already parses, as a hex
    // literal, and `drafts/segmenter.sas:9644` writes exactly that. The
    // assertion is therefore about the PRODUCTION and not about the terminal
    // set: admitting `इ` to `integer_type` would not add a sign, it would give
    // one sign two senses inside one grammar.
    let integer_type = production("integer_type");
    assert!(
        !integer_type.contains('इ'),
        "`integer_type` now names `इ`; ADR-0030 refused it, and admitting it \
         would force `अ` to be re-glossed against doc 02 §2.5, \
         `grammar-t0.ebnf:97` and `encode.rs`'s E20/E21 — and would collide \
         with `hex_digit`, where `इ` is already `c`. The production read: \
         {integer_type}"
    );
    for kept in ['अ', 'न'] {
        assert!(
            integer_type.contains(kept),
            "`integer_type` dropped `{kept}`; ADR-0030 keeps both"
        );
    }

    // The premise ADR-0030 corrects. `न` is written zero times in the T1
    // CORPUS, which is not zero times anywhere: T0 writes it 123 times, and
    // that is the count the wider claim was missing. Pinned by prefix so the
    // narrower reading cannot be re-derived from a narrower measurement.
    let t0 = t0_type_suffix_counts();
    pin_report!(
        (t0.get(&'अ').copied(), t0.get(&'न').copied()),
        (Some(1100), Some(123)),
        "T0's type-suffix counts moved; ADR-0030 rests on `न` being written and \
         not on it being rare"
    );

    // And `इ` is written at ONE width in the whole corpus, against `अ`'s four.
    // A prefix that had a sense would be spellable at every width the grammar
    // gives it; one that is a transliteration of `i64` is spellable at 64.
    let (annot, dadati) = corpus_type_positions();
    for width in ["८", "१६", "३२", "१२८"] {
        let word = format!("इ{width}");
        pin_report!(
            (annot.get(&word), dadati.get(&word)),
            (None, None),
            "`{word}` is now written in type position; ADR-0030 ruled on a leak \
             confined to one width and this is no longer that"
        );
    }
    // THE REMEDIATION LEDGER. ADR-0030 authorised the repair of 1378 sites and
    // did not perform it: 1322 `अ६४` spelling an unsigned quantity, which become
    // `न६४`, and 56 `इ६४` spelling a signed one, which become `अ६४`. It is
    // per-file work — each site's sign is read off its Rust twin, not guessed
    // from its neighbour — so the count moves one file at a time and this is
    // where a cycle records which file it took.
    //
    // Done so far, in the order taken:
    //
    // 1. `ir.t1` (56 `अ६४` -> `न६४`, 4 `इ६४` -> `अ६४`), against
    //    `crates/sadhana/src/t1/ir.rs`, where `ConstInt(i64)` is the only
    //    signed thing in the module and `ValueId`/`BlockId`/`Param`/`next_val`/
    //    `next_block` are all `usize`.
    // 2. `ast.t1` (47 `अ६४` -> `न६४`, no `इ६४` at all), against
    //    `crates/sadhana/src/t1/ast.rs`. Every site is a kind numbered from १
    //    or an index into one of that file's two arenas, so the file moves
    //    WHOLLY in one direction — the first one that does.
    //
    // 3. `vishlesana.t1` (39 `अ६४` -> `न६४`, 2 `इ६४` -> `अ६४`, and ONE `अ६४`
    //    LEFT ALONE), against `crates/sadhana/src/vishlesana.rs`, which holds
    //    exactly two signed things — `Decoded::operands: Vec<(String, i64)>`
    //    at `:50` and `fn extract(..) -> i64` at `:58` — against `width_of(..)
    //    -> usize`, `decode_at(.., offset: usize)` and `count_ones`'s `u32`.
    //    So this file is the FIRST TO MOVE IN BOTH DIRECTIONS AT ONCE with a
    //    twin that does not run out, and it is the first whose twin declares
    //    the signs rather than leaving them to be argued.
    //
    // 4. `artha.t1` (39 sites `अ६४` -> `न६४`, no `इ६४` at all — but only 34
    //    of the 39 are visible here; see the wrapper note below), against
    //    `crates/sadhana/src/t1/{types,resolve,typecheck}.rs`. THE FIRST FILE
    //    OF THE REPAIR WHOSE TWIN REACHES EVERY KIND CONSTANT IT DECLARES:
    //    `types.rs`'s `Ty` has twelve variants and the twelve `*अर्थभेद`
    //    constants are those twelve in the same order, one for one, where
    //    `ast.t1`'s twin reached only twelve of thirty-four. There is no
    //    `i64` and no `isize` anywhere in the three twin files — `width` is
    //    `u8`, `SymbolId` is `pub struct SymbolId(pub usize)` and
    //    `Resolver::next_symbol` is `usize`.
    //
    //    RE-FOUNDED 2026-09-05 (W-274, W-277) — IT LOST A WITNESS, NOT A
    //    COUNTEREXAMPLE. This cited "`SymbolId`/`NodeId` are `pub usize`
    //    (`t1/ast.rs:4`, `:7`)". `NodeId` was deleted: `pub`, constructed
    //    nowhere in the tree, and unnameable by `dead_code` for exactly that
    //    reason. The unsigned claim is UNCHANGED and now rests on one witness
    //    instead of two — recorded rather than dropped, because a citation
    //    that silently loses half its support is how a true claim becomes
    //    unfalsifiable. Both line numbers had also decayed (`ast.rs:4` -> :24,
    //    `resolve.rs:11` -> :32) under edits above them; cited by NAME now,
    //    since nothing here reads a margin's line number. — so the
    //    file moves wholly in one direction, as `ast.t1` did, but this time
    //    with a twin that testifies to every site rather than to a third
    //    of them. So the `ast.t1` gap is a property of PARTICULAR FILES —
    //    those the T1 port carried further than `crates/sadhana` did — and
    //    not of the rule, which is now shown from both sides.
    //
    // 5. `lex.t1` (74 sites `अ६४` -> `न६४`, no `इ६४` at all), against
    //    `crates/sadhana/src/lex.rs`, which declares SIX numeric fields and
    //    all six are `usize` — `Token::{byte, aksara, line}` and
    //    `LexError::{byte, index, line}` — with `Kind` and `Karaka` enums and
    //    no `i64` or `isize` anywhere in the module. The eight `*भेद` are
    //    already paired with their `lex.rs` variant by `T0_LEXER_KINDS` in
    //    `crates/sadhana-t1/tests/t1_sources.rs`, so that half of the reading
    //    is off a mapping the gate ALREADY HOLDS and is not made afresh here.
    //
    //    AND IT IS THE FIRST FILE WHOSE CASE FOR UNSIGNED IS ALSO INTERNAL.
    //    `artha.t1` argued from a twin that declared every sign and `ast.t1`
    //    from one that ran out; this file additionally lacks the two shapes a
    //    signed reading would need. EVERY SUBTRACTION IS GUARDED —
    //    `टिप्पणीसीमा`'s `सीमा वियोगः २` under a `यदि सीमा न्यूनम् ३` that
    //    returns first, `पुच्छविरामः`'s `वियोगः ६`/`वियोगः ३` under
    //    `दैर्घ्य अधिकम् ६`/`अधिकम् ३`, `विभज`'s three under
    //    `दैर्घ्य न्यूनम् ३`, and every `सीमा वियोगः आरम्भः` on the range
    //    contract आरम्भः <= सीमा that `पदविभाग` keeps at each call. AND THERE
    //    IS NO NEGATIVE SENTINEL: `विरामचिह्न` and `विभज` answer ० for "not a
    //    sign" and "no kāraka", `पदादिः`/`पदान्तिकम्`/`पुच्छविरामः` answer
    //    सीमा for "not found". A port that had reached for -१ would be the one
    //    place a signed type was earned here, and it did not.
    //
    //    IT IS ALSO THE FIRST FILE SINCE `ir.t1` WHOSE COUNT AND WHOSE LEDGER
    //    AGREE: it wraps no 64-bit type in `अङ्कः अन्तः`, `सम्भाव्य` or
    //    `दोषयुक्त`, so the triple below moves by 74 and so does the file.
    //
    // 6. `parse.t1` (75 sites `अ६४` -> `न६४`, of which 74 are visible here; no
    //    `इ६४` at all), against `crates/sadhana/src/t1/parse.rs`, which IS
    //    compiled — `t1/mod.rs` names it, the check `ast.t1` learned to make
    //    of `passes.rs` — and which declares THREE numeric fields, all three
    //    `usize`: `ParseError::line` `:6`, `ParseError::aksara` `:7` and
    //    `Parser::pos` `:13`. There is no `i64` and no `isize` in the module.
    //
    //    BUT THE TWIN RUNS OUT AGAIN, AND THIS FILE IS WHERE THE ANSWER TO
    //    THAT STOPS BEING AN ARGUMENT AND BECOMES A READING. `parse.rs` builds
    //    owned `Declaration`s and has no arena, no `घोषणा` and no error store,
    //    so `घोषणासूचकाङ्क`, `दोषसूचकाङ्क` and every `*सूचकाङ्क` parameter have
    //    no field in it whose sign could be read. What answers for them is a
    //    declaration in ANOTHER MODULE THIS FILE ALREADY IMPORTS: `आयातः
    //    वास्तु` is `ast.t1`, entry 2 above, whose `अभिव्यञ्जकसूचकाङ्क` and
    //    `वाक्यसूचकाङ्क` arena counters and whose thirteen `*द्विकर्मभेद`
    //    constants are ALREADY `न६४`. So before this step `अभिव्यञ्जकयोजनम्`
    //    incremented an `न६४` counter and returned it as `अ६४`, and
    //    `द्विकर्ममेलनम्` returned `ast.t1`'s `न६४` constants through an `अ६४`
    //    result: THE TWO SPELLINGS DISAGREED ACROSS A MODULE BOUNDARY. That is
    //    the same defect class `ir.t1`'s `अङ्कमूल्यम्` showed inside ONE
    //    routine, now shown BETWEEN two files, and it is a new thing the ledger
    //    records: as the repair advances file by file, an unremediated importer
    //    of a remediated module is a fresh disagreement the census cannot see.
    //
    //    THE INTERNAL CASE HOLDS TOO, BY `lex.t1`'s TWO TESTS. The file's ONE
    //    subtraction — `वृत्तिपठनम्`'s `पठनस्थान वियोगः १` — stands inside a
    //    branch `मेलनम्` has just taken, and `मेलनम्` answers सत्यम् only after
    //    `पठनस्थान भवति पठनस्थान योगः १`, so `पठनस्थान` is at least १ there.
    //    And there is NO NEGATIVE SENTINEL: the file's own header states the
    //    convention — `शून्यं तु अभावं सूचयति` — and `द्विकर्ममेलनम्` answers ०
    //    for *not an operator*, `दोषयोजनम्` ० for *this parse failed*, and the
    //    चर/यदि/यावत् statements ० for an absent initialiser or body.
    //
    //    ITS ONE HIDDEN SITE IS A THIRD SHAPE FOR THE SAME LESSON:
    //    `कार्यक्रमपठनम्`'s `ददाति दोषयुक्त न६४`, where the token after
    //    `ददाति` is the ErrorUnion. `artha.t1` found `सम्भाव्य`/`दोषयुक्त` and
    //    `vishlesana.t1` found `अङ्कः अन्तः`; this is `दोषयुक्त` on a `ददाति`
    //    rather than on a `ॱॱ`, so the triple moves by 74 while the file moves
    //    by 75.
    //
    //    RECORDED, NOT REPAIRED, AND NO ROW FILED — a `parse.t1` defect that
    //    is not a sign: `वृत्तिपठनम्`'s parameter loop writes
    //    `पठनस्थान भवति पठनस्थान योगः ०` in BOTH arms of one `यदि ... अन्यथा`,
    //    so the branch decides nothing. It is a no-op on either path and cannot
    //    carry a sign, which is why it is named and not touched here.
    //
    // 7. `samyojana.t1` (96 `अ६४` -> `न६४`, of which 88 are visible here; 30
    //    `इ६४` -> `अ६४`, all 30 visible; and THREE `अ६४` LEFT ALONE), against
    //    `crates/sadhana/src/samyojana.rs` and the object model it consumes,
    //    `crates/sadhana/src/vastu.rs`. IT HOLDS THE CORPUS'S LARGEST SIGNED
    //    CLUSTER — 30 of the 50 `इ६४` that were left — AND THE TWIN DECLARES
    //    EVERY ONE. `vastu.rs` has exactly one signed field,
    //    `ObjectRelocation::addend: i64` `:86`, against `ObjectSymbol::value:
    //    u64` `:59`, `section: u16` `:61`, `symbol`/`kind: u32` `:82`/`:84`,
    //    `ObjectRelocation::offset: u64` `:80` and `Object::bss: u64` `:101`;
    //    `samyojana.rs` adds the second, `patch(.., value: i64)` `:463`, and
    //    settles the three that could have been argued —
    //    `link_at(.., load: u64)` `:92`, `kind_named(..) -> Option<u32>` `:64`
    //    and `kosha.rs:45`'s `LOAD_ADDRESS: u64`.
    //
    //    EVERY SIGNED SITE IS ONE THING — RELOCATION ARITHMETIC — AND EVERY
    //    ONE IS A HOISTED CAST. `samyojana.rs` writes `let value = target as
    //    i64 + addend - pc as i64` `:281`, `hi_pc = load as i64 + text_at[n]
    //    as i64 + sym.value as i64 + r.addend` `:295` and `whole = target as
    //    i64 - hi_pc` `:296`; T1 has no cast, so each `as i64` becomes the
    //    PARAMETER's type. That is verbatim ADR-0030's own example
    //    (`encode.t1:212`'s `चिह्नितम्`), and it is why अन्तरमूल्यम्,
    //    उपरिपदस्थानम् and शोधनस्थानम् take ADDRESSES as signed and are right
    //    to: it is not the address that is signed, it is the difference.
    //
    //    AND THIS IS WHERE THE FOURTH READING RETURNS A CONTRADICTION RATHER
    //    THAN AN ANSWER — A LIMIT OF THE METHOD, FOUND AT THE SEVENTH FILE.
    //    `parse.t1` established *read the module this file imports*, and it
    //    has been answering ever since. Here it does not: पदपुनर्निर्माणम्
    //    receives विश्लेषण's `अवकाशमूल्यानि` list, whose element entry 3
    //    settled as SIGNED `अ६४` (`vishlesana.t1:218`, from
    //    `Decoded::operands: Vec<(String, i64)>`), and passes it to
    //    `सङ्केतन ॱ स्थापनम्`, whose twin is `Slot::place(v: u64)` and which
    //    the `encode.t1` cycle must therefore write `न६४`. Rust bridges the
    //    two with `as u64` at `samyojana.rs:497`. **T1 HAS NO SUCH SPELLING**,
    //    so when `encode.t1` is taken that boundary is a disagreement NO
    //    READING CAN REMOVE: it needs an explicit conversion routine, not a
    //    respelling. Recorded here rather than filed as a row, because the
    //    cycle that meets it is the one that needs the note.
    //
    //    THE THREE LEFT-ALONE SITES ARE THE SAME LESSON AS `ir.t1:299` AT
    //    LARGER SCALE: पदपुनर्निर्माणम्'s मूल्यानि element and मूल्यम्, and
    //    अंशदानम् below them, are all `patch`'s `i64` and were already right.
    //    Against them, अष्टाष्टकनिधानम्/चतुरष्टकनिधानम् take मूल्यम् as `न६४`
    //    — a BIT PATTERN, because `:219` casts to `u64` and `:371` to `u32`
    //    BEFORE the little-endian split. The `युक् २५५` mask would have hidden
    //    that question, arithmetic and logical shifts agreeing under it at
    //    every distance, so it is settled off the twin's cast and not off the
    //    body. Their only callers are this file's three stubs, so unlike the
    //    स्थापनम् boundary above it creates no live disagreement.
    //
    //    THE INTERNAL CASE HOLDS BY `lex.t1`'s TWO TESTS. Every subtraction in
    //    an unsigned routine is GUARDED — अष्टकसंरेखणम्'s `८ वियोगः शेषः`
    //    after `यदि शेषः समम् ० आदि प्रत्यागमनम्`, भेदाङ्कः's
    //    `पङ्क्तिः वियोगः १` after `यदि पङ्क्तिः समम् ० आदि प्रत्यागमनम्
    //    शून्यम्` — and the only two that are not stand inside the signed
    //    pair, where अधोभागः's result is MEANT to reach [-0x800, 0x7ff]. AND
    //    THERE IS NO NEGATIVE SENTINEL: शून्यम् for every Optional, ० for
    //    भेदपङ्क्तिः's line number, and the slot COUNT for पूरणीयावकाशः.
    //
    //    RECORDED, NOT REPAIRED, AND NO ROW FILED — THE FOURTH READING COULD
    //    NOT BE APPLIED THROUGH THIS FILE'S `आयातः` LINE, BECAUSE THE NAME IS
    //    TAKEN. `samyojana.t1` writes `आयातः वास्तु` meaning the future
    //    hand-port of `vastu.t1` (`मण्डलम् वअसतउ`), and says so in its own
    //    header; but `ast.t1` DECLARES `मण्डलम् वास्तु` today. Following the
    //    import would have read the T1 AST's arena counters for an ELF
    //    object's offsets. The signs above are read off `vastu.rs` directly
    //    for that reason. Two modules claiming one name is a `D-002b`
    //    question, not a sign, which is why it is named and not touched.
    //
    // 8. `nidana.t1` (121 `अ६४` -> `न६४`, of which 112 are visible here; no
    //    `इ६४` at all; and THREE `अ६४` LEFT ALONE), against
    //    `crates/sadhana/src/nidana.rs`, `parse.rs`'s `distance`/`nearest`
    //    and `crates/sanskrit-text/src/segment.rs`.
    //
    //    THE FIRST FILE WHOSE TWIN HOLDS NO INTEGER AT ALL, AND THE FIRST
    //    WHERE THE SIGN WAS READ OFF A NEGATIVE SENTINEL AND NOTHING ELSE.
    //    `nidana.rs` declares not one numeric type in 465 lines — its
    //    `render` substitutes with `String::replace` over an `enumerate()`
    //    index — and `segment.rs`, the twin for the two thirds of this file
    //    that is the akṣara segmenter, has `usize`, `u32` and `u8` and no
    //    signed type anywhere. So the unsigned direction is settled by the
    //    twin, and the SIGNED direction is settled by the port having
    //    invented a routine the twin does not have: `पदार्थाङ्कः` answers
    //    *which argument is the placeholder at this offset* and spells "no
    //    placeholder here" as `ऋण१`. That is `lex.t1`'s second reading — a
    //    negative sentinel earns a signed type — deciding a file ON ITS OWN
    //    for the first time, with no twin behind it.
    //
    //    THE THREE LEFT-ALONE SITES ARE THAT ONE ROUTINE AND ITS TWO
    //    CONSUMERS: `पदार्थाङ्कः`'s `ददाति अ६४`, the `चरः अङ्कम् ॱॱ अ६४`
    //    that binds it in `विवरणम्`, and `पदार्थलेखनम्`'s `अङ्कम्`
    //    PARAMETER. The parameter is the `encode.t1` contradiction one file
    //    early and one size smaller: `विवरणम्` writes `यदि अङ्कम् अधिकम्
    //    ऋण१` and only then calls, so the value is provably `०` or more at
    //    the call site — but T1 has no cast, so spelling that parameter
    //    `न६४` would pass an `अ६४` into an `न६४` three lines below its own
    //    guard. It is held at `अ६४` so the two agree, and it is a SECOND
    //    witness that ADR-0030's conversion routine is needed rather than a
    //    respelling.
    //
    //    AND READING (1) FIRED HERE WITHOUT EARNING A SIGN, WHICH IS NEW.
    //    `सङ्केताङ्कः`'s UTF-8 decode subtracts `१२८` from each
    //    continuation byte with NO range test in front of it — the shape
    //    `lex.t1` called an unguarded subtraction — and the answer is still
    //    unsigned, because the guard is on the INPUT and not in the routine:
    //    the twin takes a `&str` and reads `ch as u32` off a real `char`, so
    //    a byte below `१२८` in that position is malformed UTF-8 and not a
    //    negative number. An unguarded subtraction is a QUESTION, not a
    //    verdict; the sentinel is the half of that reading that decides.
    //
    //    RECORDED, NOT REPAIRED, AND NO ROW FILED — THE `आयातः` LIST IS
    //    NEITHER SOUND NOR COMPLETE, SO THE FOURTH READING CANNOT BE DRIVEN
    //    OFF IT. `nidana.t1:94` writes `आयातः वास्तु` and then uses NOTHING
    //    from `वास्तु`, while the module it actually reaches into —
    //    `वाक्यविभागॱपाठांशकोश` and `वाक्यविभागॱवाक्यदोष` — has no `आयातः`
    //    line at all. `samyojana.t1` found the name `वास्तु` ambiguous; this
    //    file shows the list is also unsound in both directions, so a cycle
    //    applying reading (4) must grep for `मण्डलम्ॱ`-qualified uses and
    //    not trust the header.
    //
    //    THAT LEAVES A LIVE DISAGREEMENT THIS CYCLE CREATED ON PURPOSE, IN
    //    THE DIRECTION `parse.t1` DID NOT MEET: `निदानम्`'s `पदार्थारम्भ`
    //    and `पदार्थसंख्यान` are now `न६४` and are assigned FROM
    //    `वाक्यदोष`'s fields of the same names, which `vakyavibhaga.t1:316`
    //    still spells `अ६४`. `parse.t1` was an unremediated importer of a
    //    remediated module; this is a remediated module reading an
    //    unremediated one. Both close when `vakyavibhaga.t1` is taken, and
    //    both are invisible to the triple below.
    //
    // 9. `utsarjana.t1` (122 `अ६४` -> `न६४`, of which 113 are visible here;
    //    4 `इ६४` -> `अ६४`; and ONE `अ६४` LEFT ALONE), against
    //    `crates/sadhana/src/t1/regalloc.rs`, `emit.rs` and `x86_64.rs`. THE
    //    THREE TWINS DECLARE ONE SIGNED THING BETWEEN THEM AND IT IS NOT IN
    //    THE ALLOCATOR: `emit.rs:5`'s `to_devanagari_numeral(n: i64)`.
    //    `regalloc.rs` has `Location::Register(u8)`, `Location::Spill(usize)`,
    //    `num_spills: usize`, `allocate_registers(.., num_registers: u8)`,
    //    `lifetimes: HashMap<ValueId, (usize, usize)>`, `intervals`,
    //    `active: Vec<(ValueId, usize, u8)>` and `free_registers: Vec<u8>`;
    //    `x86_64.rs` has `reg_name(&self, reg_num: usize)` and no other
    //    numeric type at all. So the whole linear-scan allocator — every
    //    index, count, position, register number and spill slot — is unsigned
    //    by the twin AND by construction, and the file's `इ६४` are the
    //    numeral formatter's and only its.
    //
    //    THE LEFT-ALONE SITE IS `अङ्कचिह्नम्`'s `अङ्कम्` PARAMETER, AND IT IS
    //    A THIRD WITNESS FOR ADR-0030's CONVERSION ROUTINE — after
    //    `samyojana.t1`'s `स्थापनम्` boundary and `nidana.t1`'s
    //    `पदार्थलेखनम्`. Its only caller passes `अवशेषः`, which this step just
    //    made `अ६४`, and the value is provably `०`..`९` there because it is a
    //    remainder modulo `१०` — but T1 has no cast, so spelling the parameter
    //    `न६४` would pass an `अ६४` into an `न६४`. AND THE TWIN CANNOT
    //    ARBITRATE EITHER: Rust writes the ten digit arms as a `match` on
    //    `char` INSIDE `to_devanagari_numeral` and declares no such routine,
    //    so this parameter's sign is its CALLER's and not a field's. Three
    //    witnesses in three consecutive files is no longer an oddity of one
    //    port: it is one boundary with one shape — a signed value reaching an
    //    unsigned parameter, which Rust crosses with `as` and T1 cannot.
    //
    //    AND THIS FILE CLOSES A READING-(4) DISAGREEMENT INSTEAD OF OPENING
    //    ONE, WHICH IS THE THIRD AND LAST ARRANGEMENT THE FOURTH READING CAN
    //    BE IN. `utsarjana.t1:128` writes `आयातः मध्यरूप` and six sites read
    //    `ir.t1`'s `पर्वारम्भ`, `पर्वसंख्यान`, `आज्ञारम्भ`, `आज्ञासंख्यान`
    //    and its two `क्रमाङ्क` — all of which `ir.t1`, entry 1 above, has
    //    spelled `न६४` since. Until this step each was bound into an `अ६४`
    //    local. `parse.t1` was an UNREMEDIATED importer of a remediated
    //    module and `nidana.t1` a REMEDIATED module read by an unremediated
    //    one; this is the case where the imported file was taken FIRST, and
    //    it is the only one of the three that settles rather than defers.
    //    It is also the first `आयातः` line of the repair that is both sound
    //    and complete — one import, used, and the file's only other
    //    `मण्डलम्ॱ`-qualified name (`वास्तुॱयोगद्विकर्मभेद`, `:96`) is inside
    //    a `॰` comment, which is the grep `nidana.t1` said this reading needs.
    //
    // 10. `sanskrit_text.t1` (17 `अ६४` -> `न६४`, of which 16 are visible here;
    //     no `इ६४` at all), `vastu.t1` (3 `अ६४` -> `न६४` and its 1
    //     `इ६४` -> `अ६४`, all four visible) and `kosha.t1` (1 `अ६४` -> `न६४`,
    //     visible) — THREE WHOLE FILES IN ONE STEP, and after them only TWO
    //     files in the corpus still hold an unremediated site.
    //
    //     `sanskrit_text.t1` IS THE FIRST FILE OF THE REPAIR THAT SETTLES
    //     ITSELF FROM ITS OWN PROSE. Its `:54` says `मानम्` answers "the
    //     MAGNITUDE, ० to १८४४६७४४०७३७०९५५१६१५" — that is `u64::MAX` written
    //     out in Devanagari digits, so the file states the width and the
    //     absence of a sign before any twin is consulted. The twin agrees and
    //     adds nothing to argue about: `crates/sanskrit-text/src/numeral.rs`
    //     declares `value(token: &str) -> Result<u64, NumeralError>` at `:232`
    //     and `bits(token: &str) -> Result<u64, NumeralError>` at `:269`, and
    //     the module declares no signed type at all. The four `अङ्क*दोषः`
    //     constants are `NumeralError`'s discriminants numbered from `०`, and
    //     a discriminant numbered from zero is unsigned BY CONSTRUCTION.
    //
    //     AND `अंशाः` IS THE ONE SITE THAT LOOKS SIGNED AND IS NOT, WHICH IS
    //     WHY THE FILE IS WORTH A PARAGRAPH RATHER THAN A LINE. `:65` says
    //     `bits` reads `ऋण` "as two's complement", so a negative literal DOES
    //     reach it — but what it returns is the sixty-four BITS and not the
    //     number, and `numeral.rs:269` returns `u64` for exactly that reason.
    //     W-075's split (`:16`-`:21`) is the whole point: a datum is a BIT
    //     PATTERN and a count is a MAGNITUDE, and neither is a signed integer.
    //     A reading that stopped at the word `ऋण` would have spelled this one
    //     `अ६४` and put the port back into the defect W-075 was filed for.
    //
    //     THE TWELVE RANGE SITES ARE SETTLED BY A CITATION AND NOT BY AN
    //     IMPORT, WHICH IS A FIFTH READING AND THE WEAKEST SO FAR. The four
    //     routines take `आरम्भः`/`सीमा` because T1 has no sub-slice
    //     expression, and `:30` names the idiom's source as
    //     `सङ्केतनॱक्षेत्रखण्डसाम्यम्` — which was UNREMEDIATED when this
    //     was written, spelling them `अ६४`, and so could not testify. IT IS
    //     REMEDIATED NOW, by the fence-table unit below, and writes
    //     `आरम्भः ॱॱ न६४ सीमा ॱॱ न६४`; the citation the reading wanted is
    //     available at last. The paragraph is corrected rather than rewritten
    //     because the reading it records was made WITHOUT it, on `lex.t1`
    //     alone, and that is the part worth keeping.
    //     `lex.t1`, entry 5 above, can: `अक्षरगणना`, `टिप्पणीसीमा`, `पदादिः`,
    //     `पदान्तिकम्`, `विरामचिह्न`, `पुच्छविरामः` and `विभज` all write
    //     `आरम्भः ॱॱ न६४ ऽ सीमा ॱॱ न६४`, seven routines of the same idiom in
    //     a file this repair has finished. Reading (4) needs an `आयातः` line
    //     and this file has none; a CITATION is weaker because nothing in the
    //     language checks it, so it is recorded as what it is — corroboration
    //     for a reading the twin had already settled, not the reason.
    //
    //     `vastu.t1` AND `kosha.t1` ARE WHERE THE METHOD IS AT ITS EASIEST,
    //     AND THAT IS ITSELF THE FINDING. Both are MECHANICAL
    //     TRANSLITERATIONS — `वअलउए` is `value`, `ओफफसएट` is `offset`,
    //     `अडडएनड` is `addend` — so every field name maps to its twin's by
    //     construction and no reading is needed at all. `vastu.rs:55`'s
    //     `ObjectSymbol { value: u64 }`, `:78`'s `ObjectRelocation { offset:
    //     u64, addend: i64 }`, `:91`'s `Object { bss: u64 }` and
    //     `kosha.rs:143`'s `Symbol { value: u64 }` answer all five sites
    //     field-for-field, and `अडडएनड` is the ONLY signed one:
    //     `Elf64_Rela`'s `r_addend`, which is signed in the ABI and not by
    //     this port's choice. IT IS THE FIRST `इ६४` OF THE REPAIR WHOSE
    //     SIGNEDNESS COMES FROM A FILE FORMAT rather than from a difference
    //     computed in the port, which is why it needed no `स्थापनम्`-shaped
    //     boundary and opened no fourth contradiction. It is annotated IN
    //     `vastu.t1` and not only here, because a lone `अ६४` among `न६४`
    //     siblings is precisely what a file-wide `sed` would silently undo.
    //
    //     WHAT IS LEFT IS TWO FILES AND NEITHER CAN BE TAKEN AS IT STANDS —
    //     `vakyavibhaga.t1`'s 487 is larger than one cycle's budget and
    //     PROTOCOL §3 forbids splitting this row to reach it, and
    //     `encode.t1`'s 171 + 15 waits on the conversion routine the
    //     `स्थापनम्` boundary asks for. Both are named in `STATE.md`'s
    //     *Next task*; this step deliberately cleared everything that could be
    //     cleared without one, so the two that remain are blocked on
    //     DECISIONS and not on effort.
    //
    // AND `artha.t1` HIDES FIVE SITES FROM THIS CENSUS BEHIND A SECOND KIND OF
    // WRAPPER. `vishlesana.t1` found `अङ्कः अन्तः`; here it is `सम्भाव्य` and
    // `दोषयुक्त`, the Optional and the ErrorUnion. `परिसरान्वेषणम्` and
    // `नामनिर्णयः` RETURN `सम्भाव्य न६४`, `पुनरुक्तघोषणम्` and `नामनिर्णयः`
    // BIND it, and `घोषणम्` returns `दोषयुक्त न६४` — five repaired sites, and
    // the triple below moves by 34 while the file moved by 39. Two different
    // wrappers now hide sites from it, which settles what the number is: the
    // census is a LOWER BOUND on the remediation and never a measure of it.
    //
    // `vishlesana.t1`'s LEFT-ALONE SITE IS THE SECOND OF ITS KIND AND THE
    // REASON A FILE-WIDE `sed` STAYS THE WRONG TOOL. `उद्धरणम्`'s local
    // `मूल्यम्` is `extract`'s `let mut value: u64` AND its `value as i64`
    // both at once, T1 having no cast; it is returned from a routine that
    // returns the signed type and the sign-extension below it can make it
    // negative, so the one binding is the SIGNED one and `अ६४` was already
    // right. `ir.t1:299` was left for the same reason.
    //
    // AND TWO SITES IN IT ARE INVISIBLE TO THIS CENSUS, WHICH COUNTS ONLY THE
    // TOKEN AFTER `ॱॱ` OR `ददाति`. Where a slice is declared that token is
    // `अङ्कः` and the element type is two words further on:
    // `अवकाशमूल्यानि`'s `मूल्यानि ॱॱ अङ्कः अन्तः इ६४` and `अवकाशभेदाः`'s
    // `भेदाः ॱॱ अङ्कः अन्तः अ६४` are both repaired and neither moves the
    // triple. A cycle that repaired only what the ledger can see would leave
    // a slice disagreeing with the element type it is filled from.
    //
    // AND `ast.t1` IS WHERE THE METHOD FIRST RUNS OUT: the twin reaches only
    // twelve of its thirty-four kind constants. `ast.rs`'s `Expression` has
    // five variants against the corpus's eight, its `Statement` two against
    // eight, and it declares no operator type at all — `ast.rs:36` and `:45`
    // defer all three out loud. For those twenty-two sites there is no field
    // whose sign could be read, and the reading is off what the constant IS.
    // So `STATE.md`'s *read it off the Rust twin* is a rule with a gap in it,
    // and the gap widens for every file the port carried further than `sadhana`
    // did. It is not a licence to guess: a discriminant numbered from १ and an
    // index into an arena are both unsigned BY CONSTRUCTION, which is a reason
    // and not a neighbour.
    //
    // `crates/sadhana/src/t1/passes.rs` IS NOT A TWIN. It is missing from
    // `crates/sadhana/src/t1/mod.rs`, so nothing compiles it, and its test
    // writes `Expression::BinaryOp`, `BinaryOp::Add` and a
    // `Declaration::Function { is_public, .. }` — three things `ast.rs` does
    // not have. A file that does not build cannot testify to a sign. Recorded
    // and NOT repaired: it is a `sadhana` defect and this row is the corpus's.
    // `encode.t1` IS TAKEN IN PARTS AND THE PART TAKEN HERE IS ITS SIGNED
    // HALF — the eleven routines that held all 18 of its `इ६४`, respelled
    // together because a signed site and its unsigned partner are declared on
    // the SAME line and a line range would cut a pair in two. With it the
    // `इ६४` column reaches ZERO and the signed direction of ADR-0030 is
    // finished; what is left of the repair is the unsigned direction alone,
    // 170 `अ६४` here and 487 in `vakyavibhaga.t1`.
    //
    // THE FILE'S OTHER `अ६४` ARE UNTOUCHED ON PURPOSE AND THE TRIPLE IS
    // THE ONLY PLACE THAT SHOWS IT. `encode.t1` contributes 134/0/52 to the
    // numbers below — the only file in the corpus with a large count in BOTH
    // the first and the third column — because a bounded part of a file is a
    // state the ledger must be able to hold. Reading it as a finished file is
    // the mistake this note exists to prevent.
    //
    // AND THE SECOND PART OF `encode.t1` IS TAKEN HERE: **THE DERIVED-TABLE
    // TEXT READER**, the nine routines under `reading a derived table's TEXT`
    // — अष्टकान्वेषणम्, पङ्क्तिसीमा, उपेक्ष्यपङ्क्तिः, क्षेत्रसंख्या,
    // क्षेत्रारम्भः, क्षेत्रसीमा, षोडशाङ्कः, षोडशाङ्कमूल्यम् and
    // दशाङ्कमूल्यम्. 38 sites, of which THIS CENSUS SEES 36: the two
    // `ददाति सम्भाव्य न६४` returns are hidden from it exactly as `artha.t1`'s
    // five were, which is the same lower-bound property recorded above and
    // not a new one. The triple moves 705/0/673 -> 669/0/709.
    //
    // IT IS THE FIRST UNIT OF THIS REPAIR THAT NEEDED NO PER-SITE READING,
    // AND THE REASON IS A PROPERTY OF THE DATA RATHER THAN OF THE CODE. Every
    // site in those nine is a byte OFFSET into a table's text, a COUNT of
    // TAB-separated fields, or a value PARSED out of one, and the twin makes
    // all three unsigned by construction: the offsets are `usize`
    // (`encode.rs:265`-`:296`, `:344`-`:362`), the counts are `f.len()`, and
    // every `parse()` in the four readers lands in an unsigned type and in no
    // other — `bias: u32` (`:67`, read at `:287`), `pattern`/`mask` through
    // `u32::from_str_radix` (`:295`, `:296`) and the fence domain bit through
    // `f[2].parse::<u64>()` (`:353`). The four `.tsv` tables carry no signed
    // column at all. Where the earlier units needed a routine-by-routine
    // reading, this one needed a reading of the TABLES, once.
    //
    // AND THE PRODUCERS MOVED WHILE THEIR CONSUMERS DID NOT, WHICH IS THE
    // BOUND AND IS DELIBERATE. Fifteen bindings in `encode.t1`'s register
    // table and fence table are filled from these nine and still declare
    // `अ६४`. Cycle 524's rule — *an index whose producer is outside the unit
    // keeps its producer's spelling until the producer moves* — is what makes
    // the producer the only end that could move first, and the disagreement
    // it leaves is the handle for the next unit rather than a defect: it is
    // exactly the set the register-table and fence-table cycles must move,
    // and it is visible at each call site instead of needing to be searched
    // for. `the_derived_table_text_reader_is_unsigned_because_its_tables_are`
    // pins both ends.
    //
    // AND THE THIRD PART OF `encode.t1` IS TAKEN HERE: **THE REGISTER
    // TABLE**, the four routines under `the register table` —
    // क्षेत्रसाम्यम्, कोष्ठपङ्क्तिः, कोष्ठाङ्कः and कोष्ठप्लवः. 19 sites, and
    // THIS CENSUS SEES ALL NINETEEN: the unit wraps no 64-bit type in
    // `अङ्कः अन्तः`, `सम्भाव्य` or `दोषयुक्त`, so for the first time since
    // `lex.t1` the file's count and the triple move by the same number. The
    // triple moves 669/0/709 -> 650/0/728, and `encode.t1`'s own contribution
    // moves 134/0/52 -> 115/0/71. It is STILL a bounded part and the file is
    // STILL not remediated; the fence table below it is untouched.
    //
    // IT IS THE FIRST UNIT OF THIS REPAIR THAT CLOSES A DISAGREEMENT RATHER
    // THAN OPENING ONE, AND THAT IS WHY IT WAS CHOSEN FIRST. The
    // derived-table text reader moved the PRODUCERS and left fifteen bindings
    // in the two table sections declaring `अ६४` and filled from an `न६४`
    // return; ten of those fifteen are in this unit and they are now spelled
    // as what fills them. The disagreement also crossed a MODULE boundary,
    // which the census cannot see and which the earlier note about `parse.t1`
    // named as a fresh defect class: `samyojana.t1`'s `भेदपङ्क्तिः` — itself
    // remediated, entry 7 — calls `सङ्केतन ॱ क्षेत्रसाम्यम्` with `न६४`
    // arguments into `अ६४` parameters, and that call now agrees at both ends.
    //
    // AND THE READING IS ONE READING AND NOT NINETEEN. The twin is
    // `encode.rs`'s `pub fn register(name: &str) -> Option<(u32, bool)>`
    // (`:314`), and it settles the only site in the unit that is not already
    // settled by the tables: the register NUMBER is `f[2].parse()` landing in
    // that `u32`. Everything else is a byte offset into the table's text, a
    // field index, or a loop counter over a name's octets — the same reading
    // the derived-table text reader was taken on, and the offsets here are
    // hand-rolled only because Rust walks the table with `.lines()` and T1 has
    // no iterator. `spec/registers-riscv64.tsv` has no signed column.
    // `the_register_table_is_unsigned_because_a_register_number_is` pins it.
    //
    // THE ONE-BASED ANSWERS ARE NOT A NEGATIVE SENTINEL AND ARE WHAT KEEPS
    // THIS UNIT UNSIGNED. `कोष्ठपङ्क्तिः` answers ० for *no such register*
    // and `कोष्ठाङ्कः` adds १ to the parsed number for the same reason: `शून्यः`
    // IS register ०, so a raw number would report the most-used register in
    // the ISA as an unknown name. A port that had reached for -१ there is the
    // one place a signed type would have been earned here, and it did not —
    // which is `a_negative_sentinel_is_what_keeps_a_site_signed`'s rule
    // applied to a file that could most easily have broken it.
    //
    // AND `.loop/STATE.md` PRE-JUDGED ONE OF THE FOUR "DIFFERENCE" ROUTINES
    // WRONG, WHICH IS WHY THE READING IS DONE AGAIN AND NOT COPIED. *Next
    // task* named स्थानाधारः, उपरिखण्डम्, स्थानमूल्यम् and विन्यासविचलनम् as
    // signed because *a signed quantity in this port is almost always a
    // DIFFERENCE*. Three are. स्थानाधारः is an ADDRESS: `encode.rs:533` has
    // `let mut pc = 0u32` and `:1490` `let base = if upper { pc } else {
    // pc.wrapping_sub(4) }`, u32 on both sides. It stays signed because T1 has
    // no `न३२` in which that wrap could be spelled and `:1491` widens `base`
    // with `i64::from` before subtracting — the right spelling, for a reason
    // the pre-judgement did not give. `the_address_half_is_signed_by_its_
    // consumer_and_not_by_being_a_difference` below pins both halves.
    //
    // AND THE FOURTH PART OF `encode.t1` IS TAKEN HERE: **THE FENCE
    // ORDERING-DOMAIN TABLE**, the four routines under `the fence
    // ordering-domain table` — अवग्रहान्वेषणम्, क्षेत्रखण्डसाम्यम्, समूहांशः
    // and क्षेत्रसमूहः. 27 sites, of which THIS CENSUS SEES 25: the two it
    // cannot see are `समूहांशः`'s `चरः मूल्यम् ॱॱ सम्भाव्य न६४` and
    // `क्षेत्रसमूहः`'s `ददाति सम्भाव्य न६४`, where the counted token is
    // `सम्भाव्य` and the type sits one word past it. The triple moves
    // 650/0/728 -> 625/0/753, and `encode.t1`'s own contribution moves
    // 115/0/71 -> 90/0/96. The file is STILL not remediated — the head
    // declarations, the slot-kind predicates, the encoding accessors,
    // bytes-and-words and diagnostics are untouched — but it no longer has a
    // TABLE section outstanding.
    //
    // THIS UNIT CLOSES THE DISAGREEMENT THE DERIVED-TABLE READER OPENED, AND
    // CLOSES IT COMPLETELY. That cycle moved the nine PRODUCERS and left
    // fifteen bindings declaring `अ६४` while filled from an `न६४` return; the
    // register table took ten and these are the other FIVE —
    // `क्षेत्रादिः`/`क्षेत्रान्तः` in `क्षेत्रखण्डसाम्यम्` and
    // `सीमा`/`अङ्कादिः`/`अङ्कान्तः` in `समूहांशः`. No binding in `encode.t1`
    // is now fed by a producer spelled differently from itself, which is why
    // item 3 of `the_derived_table_text_reader_is_unsigned_because_its_
    // tables_are` is DELETED with this unit rather than edited down.
    //
    // AND THE READING WAS INHERITED RATHER THAN MADE, WHICH IS THE POINT OF
    // HAVING TAKEN THE OTHER TWO FIRST. `spec/fence-domains-riscv64.tsv`'s
    // only numeric column is a BIT, `f[2].parse::<u64>()` at `encode.rs:353`,
    // already pinned by item 2 of the derived-table check; and
    // `क्षेत्रखण्डसाम्यम्` is `क्षेत्रसाम्यम्` with its `नाम` taken as a
    // RANGE instead of whole, so the register table's reading transfers
    // directly. The twin is `domain_set` (`encode.rs:344`), `Option<u64>`,
    // which is `ददाति सम्भाव्य न६४` here.
    //
    // THE ZERO IS NOT A NEGATIVE SENTINEL, at the two routines that could
    // most easily have made one: `समूहांशः` answers ० for *no such domain*
    // and `अवग्रहान्वेषणम्` answers `अवसानम्` — a one-past-the-end offset —
    // for *no separator found*. The first is sound only because every bit in
    // the table is a SET bit, a fact about the data that `tests/t1_embed.rs`
    // asserts rather than this file assuming.
    // `the_fence_ordering_domain_table_is_unsigned_because_its_table_is` pins
    // the four signatures, the twin's return, the stated ० and the ABSENCE of
    // a cross-module caller — the end the register table found a defect at
    // and that no census can reach.
    // AND THE FIFTH PART OF `encode.t1` IS TAKEN HERE: **THE ENCODING
    // ACCESSORS**, the five routines under `asking an encoding about its
    // slots` — आवरणावकाशः, नियततत्कालावकाशः, तत्कालावकाशः, कोष्ठसंख्यानम् and
    // अग्रिमतत्कालावकाशः — together with the ONE binding outside them that
    // reads their answer, `सङ्कोचनिषेधः`'s `चरः गन्तृक्रमः`. 19 sites and
    // this census sees all nineteen: the unit wraps no 64-bit type in
    // `अङ्कः अन्तः`, `सम्भाव्य` or `दोषयुक्त`, so the file's count and the
    // ledger move together. The triple moves 625/0/753 -> 606/0/772 and
    // `encode.t1`'s own contribution 90/0/96 -> 71/0/115. The file is STILL
    // not remediated — the head declarations, the slot-kind predicates,
    // placing a value into a field, bytes-and-words and diagnostics are
    // untouched.
    //
    // THE READING IS *THE NOT-FOUND ANSWER IS A LENGTH*. The twin returns
    // `Option<&Slot>` from `slots.iter().find(..)` (`encode.rs:230`, `:233`,
    // `:241`, `:424`) or a `usize` from `.count()` (`:244`). T1 has no
    // reference to return, so the port answers `सङ्केतः ॱ अवकाशाः ॱ दैर्घ्य`
    // — the index ONE PAST THE LAST SLOT — where Rust answers `None`. That is
    // `अवग्रहान्वेषणम्`'s `अवसानम्` shape exactly, and
    // `a_negative_sentinel_is_what_keeps_a_site_signed` decides it: a length
    // is not -१, so nothing here earns a sign. Everything else in the five is
    // an index, a running count, or `नियततत्कालावकाशः`'s `क्रमः`, which is
    // the twin's `n: usize` argument to `Iterator::nth`.
    //
    // AND THIS UNIT DISCHARGES A DEFERRAL RATHER THAN LEAVING ONE, WHICH NO
    // EARLIER PART OF THIS FILE COULD. Cycle 524 left `सङ्कोचनिषेधः`'s
    // `गन्तृक्रमः` at `अ६४` under the rule *an index whose producer is
    // outside the unit keeps its producer's spelling until the producer
    // moves*. The producer is `आवरणावकाशः` and it is in THIS unit, so the
    // binding moves in the same cycle — which is why item 4 of
    // `the_signed_half_of_encode_t1_is_what_adr_0031_names` no longer names
    // it. That check's own comment says to DELETE the part rather than edit
    // it when the unsigned direction arrives; this is that cycle.
    //
    // AND THE OTHER FOUR ROUTINES HAVE NO CALLER IN THE CORPUS AT ALL, so the
    // unit ends with NO disagreement outstanding in either direction and
    // leaves the next cycle no handle —
    // `the_encoding_accessors_are_unsigned_because_a_length_is_the_not_found_answer`
    // pins that absence, which is the end no census can reach.
    //
    // THIS CYCLE TOOK `encode.t1`'s SEVENTH BOUNDED PART, AND IT IS THE FIRST
    // BOUNDED BY A FAMILY RATHER THAN BY A DIVIDER: **THE SLOT KIND**, taken
    // wherever the file spells it — the seven constants `कोष्ठभेद` …
    // `अन्तरभेद`, the `अवकाश ॱ भेद` field that holds one, the four parameters
    // of the three predicates that read one, the two `भेदाः` slices and the
    // one binding drawn from a slice. **15 sites, of which this census sees
    // 13**, so the triple moves 599/0/779 -> 586/0/792 while `encode.t1`'s own
    // contribution moves 64/0/122 -> 51/0/135. The file is STILL not
    // remediated: 51 `अ६४` remain in eight sections, named in its own header.
    //
    // THE TWO SITES THIS CENSUS CANNOT SEE ARE THE POINT AND NOT A ROUNDING.
    // It counts the token after `ॱॱ` or `ददाति`; where a slice is declared
    // that token is `अङ्कः`, so `भेदाः ॱॱ अङ्कः अन्तः अ६४` in `पदरचना` and in
    // `अपाकर्तव्यम्` are repaired and move NOTHING here. `vishlesana.t1` met
    // this first — its `अवकाशमूल्यानि` and `अवकाशभेदाः`, named above — and it
    // is why the unit's bound is stated in SITES and pinned by name in
    // `the_slot_kind_is_unsigned_because_a_discriminant_has_no_sign_to_read`.
    //
    // THE READING IS *A DISCRIMINANT IS UNSIGNED, AND HERE THE TWIN HAS NO
    // NUMBER TO READ AT ALL* — the class cycle 529 met in `भाषा`, in its
    // strongest form. `encode.rs` spells the slot kind `pub kind: String`
    // (`:55`) and compares it with `==` (`:72`, `:81`, `:91`, `:152`); the
    // integer exists only because T1 has no string-equality operator, which
    // the constants' own block comment has said since the port was written.
    // What the port chose is therefore all there is to read: seven values
    // numbered from १, no ० — a slot always HAS a kind — and `भेद` compared
    // and never added to. AND THE SIBLINGS HAD ALREADY MOVED, INCLUDING IN
    // TWO OF THIS FILE'S OWN READERS: `vishlesana.t1:113`/`:135`/`:239` and
    // `samyojana.t1:497` spelled this same discriminant `न६४` already, so the
    // file that OWNS the constants was the one disagreeing with them. That is
    // the third cross-module disagreement the repair has found and the
    // largest, being a whole family rather than one parameter.
    //
    // *The account of BYTES-AND-WORDS WITH DIAGNOSTICS follows and is kept:*
    // CYCLE 529 TOOK the two smallest remaining sections, taken
    // as ONE unit because they share one reading — `प्रत्ययवत्`,
    // `चतुरष्टकम्`, `चतुरष्टकसंख्या` and `सङ्केतनदोषवचनम्`. 7 sites and this
    // census sees all seven: the unit wraps no 64-bit type in `अङ्कः अन्तः`,
    // `सम्भाव्य` or `दोषयुक्त`, so the file's count and the ledger move
    // together. The triple moved 606/0/772 -> 599/0/779 and `encode.t1`'s own
    // contribution 71/0/115 -> 64/0/122. The file was STILL not remediated:
    // 64 `अ६४` remained in nine sections, named in the file's own header.
    //
    // THE READING IS *THE TWIN HAS NO NUMBER HERE, AND THE PORT'S NUMBERS ARE
    // THE ONES THE STANDARD LIBRARY SPELLED FOR IT.* Every earlier part of
    // this file was read off a type the twin DECLARES; this one is the first
    // whose sites exist because of what Rust HIDES. `split_address_part`
    // (`encode.rs:411`) writes `base.strip_suffix(..)` and `words` (`:711`)
    // writes `text.chunks_exact(4)` — neither names an integer type at all —
    // and the three that stand behind them (`str::strip_suffix`'s comparison,
    // `chunks_exact`'s stride, the `Vec<u32>::len` that `चतुरष्टकसंख्या`
    // answers with) are `usize` in every signature `core` gives them. T1 has
    // none of the three, so the port writes the walk out by hand and every
    // site it writes is a length, an index or a count.
    //
    // AND THE SEVENTH SITE IS AN ENUM DISCRIMINANT WHOSE SIBLINGS ARE ALREADY
    // REMEDIATED, IN ANOTHER FILE — the first of those this file has had.
    // `सङ्केतनदोषवचनम्`'s `भाषा` is `EncodeError::message(&self, lang:
    // Language)` (`encode.rs:392`), and `nidana.t1` spells `Language` `न६४`
    // already: `संस्कृतभाषा`/`आङ्ग्लभाषा` at `:688`-`:689` and `भाषा ॱॱ न६४`
    // in both `विवरणम्` and `सन्देश`. So cycle 524's rule for `भेदाः` and
    // `सम्बन्धः` — *a kind moves with its siblings* — SETTLES this one rather
    // than defers it, and what looked like a deferral was a live cross-module
    // disagreement.
    // `bytes_and_words_and_diagnostics_borrow_their_spelling_from_a_hidden_usize`
    // pins the four signatures, the two twins that name no integer, the guard
    // that makes the one subtraction safe, and `nidana.t1`'s `न६४` `Language`.
    let a64 = annot.get("अ६४").copied().unwrap_or(0) + dadati.get("अ६४").copied().unwrap_or(0);
    let i64_sites =
        annot.get("इ६४").copied().unwrap_or(0) + dadati.get("इ६४").copied().unwrap_or(0);
    let n64 = annot.get("न६४").copied().unwrap_or(0) + dadati.get("न६४").copied().unwrap_or(0);
    pin_report!(
        (a64, i64_sites, n64),
        // 553 -> 550 ON 2026-09-03, MEASURED NOT CHOSEN, AND THIS ONE IS
        // AN UNDER-COUNT OF ITS OWN CYCLE BY DESIGN. The unit was
        // `encode.t1`'s LAST THREE PARTS together with the family they
        // share with `ir.t1` — `सम्भाव्य अ६४`, the optional that wraps a
        // sixty-four. SIX sites moved; this census can see only THREE of
        // them (`encode.t1`'s `विस्तृतिः`, `उच्चदैर्घ्य` and
        // `नीचदैर्घ्य`), because the other three sit behind `सम्भाव्य`
        // and the census counts the token immediately after `ॱॱ` or
        // `ददाति`. That is the standing caveat above made concrete: THE
        // CENSUS IS A LOWER BOUND ON THE REMEDIATION. See
        // `an_optional_that_wraps_a_sixty_four_is_unsigned_in_all_three_places_it_is_written`.
        //
        // WITH THIS, `encode.t1` IS FINISHED: its 15 remaining `अ६४` have
        // all been read and are all CORRECT, and ADR-0030 owes exactly one
        // file — `vakyavibhaga.t1`, 487 sites, untouched.
        //
        // 550 + 0 + 828 = 1378, THE SAME SUM AS 553 + 0 + 825. That is the
        // check that matters: three visible sites moved between spellings
        // and none were invented or lost. The triple alone could not have
        // told the difference, which is why the conservation assertion
        // below exists.
        //
        // 550 -> 536 ON 2026-09-03, AND THE REMEDIATION REACHES ITS
        // THIRTEENTH AND LAST FILE. `encode.t1` finished on the cycle
        // before this one; `vakyavibhaga.t1` — 487 `अ६४` and 0 `न६४`,
        // the only source this repair had never opened — gives up its
        // LINE NUMBER: six record fields, five appender parameters, a
        // module-level variable and the routine that answers one. 14
        // sites, all fourteen moving and all fourteen VISIBLE, which
        // makes this the first unit since 533 whose cycle and whose
        // census agree exactly.
        //
        // THE READING WAS MADE THREE FILES AGO. `lex.t1` spells
        // `Token::line` `न६४`, and eight call sites in `vakyavibhaga.t1`
        // were already handing that `न६४` into an `अ६४` parameter. See
        // `a_line_number_is_unsigned_because_every_producer_of_one_already_is`.
        //
        // 536 + 0 + 842 = 1378, the same sum as 550 + 0 + 828.
        // MERGED WITH origin/main ON 2026-09-03 (fifth of these merges). The rail's
        // side read (536, 0, 842) — its D-002h remediation had reached its last file,
        // `vakyavibhaga.t1` — against a corpus WITHOUT this branch's code. This side
        // read (1478, 69, 717). NEITHER TRIPLE DESCRIBES THE MERGED TREE. The rail's
        // spelling moves were carried onto this branch's text where the same site
        // exists (encode.t1 ×2, ir.t1 ×1, and vakyavibhaga.t1's 14 auto-merged); the
        // rail's per-file tests then found sites this branch's text still spelled the
        // old way and they were moved too (parse.t1's 4 declaration-kind discriminants,
        // encode.t1's 4 head-block declarations and the 2 parameters feeding them —
        // see each test's own note). The constant below is the MEASURED value on the
        // merged tree, written in after `cargo test -p sanskrit-text --test grammar_t1`
        // printed it; the SUM must equal 2264, this branch's sum before the merge,
        // because every change above is a move and none is a new binding. The rail's
        // own history for its side is kept above this note, unedited.
        // (1298, 54, 912) MEASURED 2026-09-03 on the merged tree: 1298 + 54 + 912 = 2264.
        //
        // 912 -> 932 ON 2026-09-03, `W-198`, MEASURED (the test printed it) and
        // then RECONCILED BY NAME, never by arithmetic. Corpus GROWTH, not a
        // remediation: twenty `न६४` bindings added by the conditional
        // terminator and the यदि/यावत् arms. `ir.t1`: the kind
        // `शाखावसानभेद`, the builder state `वर्तमानारम्भाज्ञा`, `पर्वयोजनम्`'s
        // `कोशाङ्क`, `पर्वसमाप्तिः`'s `आज्ञासंख्यान`, six discard bindings in
        // each of the two new arms (`अवगणना`..`अवगणनपञ्च` plus `अवगणनसङ्गम` /
        // `अवगणननिर्गम`), and `कार्यक्रमरचना`'s `अवगणनप्रवेश`, `अवगणनसमाप्ति`
        // and `पर्वसंख्यान` — nineteen — less the three that routine lost
        // (`आरम्भाज्ञा`, `आज्ञासंख्यान`, `पर्वस्थान`), plus two routines that
        // answer `ददाति न६४` (`पर्वस्थापनम्`, `पर्वसमाप्तिः`): eighteen. `ast.t1`:
        // `वास्तुॱवाक्य`'s new `अन्यसूचकाङ्क`, the `अन्यथा` slot the parser had
        // been dropping the else body for want of: nineteen. `parse.t1`:
        // `यदिस्थान`: twenty. (`अन्यशरीर` was already a `न६४` and only moved
        // out of its nested block.) 1298 + 54 + 932 = 2284.
        //
        // 912 -> 920 on 2026-09-03, W-183: the declared-type map's NEW `न६४` bindings —
        // `संज्ञासूचकाङ्क` and `प्रकारसूचकाङ्क` (two record fields), `संज्ञाप्रकारकोश`,
        // `घोषणम्`'s `प्रकार`, `प्राचलप्रकारः`, `चरप्रकारः`, `चरसूचकः`, `संज्ञासूचकः`/`प्रकारसूचकः`
        // as the census sees them. MEASURED by the gate, not counted: 1298 + 54 + 920 = 2272,
        // eight more than 2264 — bindings ADDED, and `अ६४` did not move (new sites in a
        // remediated file are written in the spelling it was remediated TO).
        //
        // ── W-202, 2026-09-04, ON THE MERGE OF W-198 AND W-183 ──────────────
        // THIS NUMBER IS THE ONE THE TEST PRINTED, NOT A SUM OF THE TWO ABOVE.
        // The two branches each added bindings to the same file and W-202 adds
        // more of its own (`तुलनाद्विकर्म`/`सरणद्विकर्म`/`अङ्कपदम्`'s `कर्म` and
        // `सूचकाङ्क` parameters, the द्विकर्म arm's `कर्म`, and the checker
        // arms' locals), so 932 and 920 cannot simply be added: they share the
        // 912 base and neither counted the other's sites. Adding them would
        // double-count the base and miss W-202 entirely — which is exactly the
        // "computed, not measured" mistake this ledger exists to catch.
        // The value below was written in from the failing assertion's own
        // output on the merged tree.
        //
        // 912 -> 951, W-202's ELEVEN, named: `तुलनाद्विकर्म`'s and
        // `सरणद्विकर्म`'s `कर्म`, `अङ्कपदम्`'s `सूचकाङ्क`, the द्विकर्म arm's
        // `कर्म` (four); the समूह fold's `प्रथमः`, `क्रमः`, `सीमा`, `अन्वेषणम्`
        // and `अग्रिमः` (nine); `अप्रकार्यशर्तसंख्या` (ten) and
        // `अप्रकार्यवाक्यसंख्या` (eleven).
        //
        // अ६४ HELD AT 1298 AND THAT IS THE POINT. The fold's five locals were
        // first written `अ६४`, mirroring the resolver's scan at `अर्थ.त१` that
        // this walk is copied from — which moved the count to 1303 and added
        // five to the ADR-0030 debt. New bindings take the spelling the file is
        // remediated TO; copying an unremediated routine copies its debt along
        // with its shape. They are `न६४`, and the count is back where it was.
        // MEASURED 2026-09-04 after merging W-183 (+8) and W-198 (+20) onto one tree.
        // (agent/w204 had measured the same (1298, 54, 940) on its own merge of the two.)
        // 940 -> 961 ON 2026-09-03, `W-204`, MEASURED (the test printed it) and
        // then RECONCILED BY NAME, never by arithmetic. Corpus GROWTH: twenty-one
        // `न६४` bindings, all in `ir.t1`, for the call arm. Three globals — the
        // argument arena's cursor `आदानसूचकाङ्क`, the scratch stack's
        // `आदानसञ्चयसूचकाङ्क`, the refusal's `अनिर्णीताह्वानचिह्नकाङ्क`; the
        // appender `आदानयोजनम्` answering `ददाति न६४` (four); the Call
        // constructor `आह्वानाज्ञायोजनम्` — three parameters `संज्ञा`,
        // `आदानारम्भ`, `आदानसंख्यान`, its `ददाति न६४`, its local `स्थानम्`
        // (nine); the आह्वान arm's nine locals `आह्वेयाङ्क`, `आह्वेयसंज्ञा`,
        // `आह्वेयचिह्नकाङ्क`, `आदानगणना`, `आदानारम्भः`, `सञ्चयाधारः`, `क्रमः`,
        // `अवगणनादान`, `अवगणनाह्वान` (eighteen); and `आह्वानादानरचना`'s
        // parameter `अभिसूचकाङ्क`, its `ददाति न६४` and its `पूर्वगणना`:
        // twenty-one. `artha.t1`'s one new binding is a `बूल` and is counted
        // by the control above. 1298 + 54 + 961 = 2313.
        // W-202 ON THE MERGE WITH W-204: the two notes above are SIBLINGS off
        // the same 940 base — W-202's eleven and W-204's twenty-one, neither
        // counting the other's. 951 and 961 may not be summed or chained. The
        // value below is what the assertion printed on THIS merged tree.
        // 961 -> 974 on 2026-09-04, W-226/W-228: THIRTEEN `न६४` bindings ADDED, none
        // moved — `ast.t1`'s `खण्डाभिव्यञ्जकभेद` (the slice kind, 1); `parse.t1`'s
        // `आयातितमण्डलम्` (a parameter and a local, 2), `खण्डसीमापठनम्` (a parameter,
        // its return, two locals, 4), the two postfix chains' member positions and
        // slice limits (4), and `वृत्तिपठनम्`'s `पूर्वस्थानम्`/`अवगणनदोष` (2). MEASURED
        // from this assertion's own failure: 1298 + 54 + 974 = 2326.
        // 974 -> 975, the same day, the W-228 AMENDMENT: `artha.t1`'s `घोष्यमाणभेदः`
        // (the kind being declared, `न६४`) — the symbol-kind arena beside it is
        // `अङ्कः अन्तः न६४` and the census reads the token after `ॱॱ`, so the
        // arena itself is not a site. ONE binding ADDED: 1298 + 54 + 975 = 2327.
        // MEASURED after merging W-204 (961) and W-226/W-228 (+14): the test printed it.
        // W-202 ON THE MERGE WITH W-226: 972 (this branch) and 975 (main) are
        // SIBLINGS — neither counted the other's bindings, so they may not be
        // summed or chained. Measured on THIS tree.
        // 986 -> 989 on 2026-09-04, `W-231`: THREE `न६४` bindings, named —
        // `घोषणम्`'s `घोषणाप्रकारः` (a routine's declared RETURN, which that
        // site passed as ० until this row needed it to type a call), and the
        // call arm's `पात्रम्` and `पात्रसंज्ञा`. `पात्रपदम्` is a
        // `वास्तुॱअभिव्यञ्जक` and the embed arm's two are `अर्थप्रकार`, so neither
        // moves this count. Corpus growth; the spelling did not move.
        // MERGE of W-202 (left) and W-215 (right) on 2026-09-04 by the trunk: the value below is
        // MEASURED on the merged tree from this assertion's own failure, never summed.
        //
        // 975 -> MEASURED (see below) ON 2026-09-04, `W-215`, MEASURED (the test printed it) and
        // RECONCILED BY NAME with a count over the two files, never by
        // arithmetic on the notes. Corpus GROWTH, 169 `न६४` bindings: `parse.t1`
        // nine — the parser state `मण्डलनामसूचकाङ्क`, the kind `गणनाघोषणाभेद`,
        // the struct reader's `क्षेत्रादि`, `क्षेत्रान्त`, `क्षेत्रनामस्थान`,
        // `क्षेत्रप्रकार`, `क्षेत्रस्थान`, and the enum reader's `भेदादि`,
        // `भेदान्त`; `unparse.t1` one hundred and sixty — a NEW FILE: 147 `ॱॱ न६४`
        // bindings (two cursors, seven reason constants, the refusal's two
        // fields, every routine's `सूचकाङ्क`/`कर्म`/`क्रमः` parameters and locals,
        // and the consume-and-discard `अवगणन…` bindings a T1 body needs for each
        // call whose answer it does not use — thirty-eight in the declaration
        // arm alone) plus 13 routines answering `ददाति न६४`. 1298 + 54 + 1130 =
        // 2482.
        // 975 -> 1148 on 2026-09-04, `W-215` merged onto W-226/W-228's 975,
        // MEASURED (the test printed it) and reconciled by name: the 169 the
        // W-215 note above lists, plus four written after the merge — the
        // printer's two arms for W-228's shapes (`अवगणनैकोनविंशति`,
        // `अवगणनविंशति` for the folded qualifier, `अवगणनैकविंशति` for the
        // slice) and `parse.t1`'s `उपेक्षितचिह्नकसंख्या`, the count of top-level
        // tokens the program reader skips, a measurement for a candidate row.
        // 1298 + 54 + 1148 = 2500.
        // 986 (W-202) and 1148 (W-215) are SIBLINGS off W-226's 975: W-202's eleven and
        // W-215's 173 — 975 + 11 + 173 = 1159, MEASURED on the merged tree 2026-09-04 (the test
        // printed it), each set named in its own note above.
        // MERGE of main (left: W-202 + W-215) with agent/w227 (right: W-239 part 1's arena) on 2026-09-04
        // by the trunk: the value below is MEASURED on the merged tree from this assertion's own
        // failure, never summed.
        // 975 -> 997 on 2026-09-04, W-239: TWENTY-TWO `न६४` bindings ADDED, none
        // moved, all in the NEW `ashtaka.t1` (the octet arena): two globals
        // (`अष्टकसूचकाङ्क`, `अष्टकदोषस्थानम्`); `अष्टकनिषेधः`'s position (1);
        // `अष्टकारम्भः`, `अष्टकदैर्घ्य`, `अष्टकयोजनम्` returning `न६४` (3);
        // `शून्याष्टकयोजनम्`'s count, return and two locals (4);
        // `अष्टकपाठयोजनम्`'s two bounds, return and three locals (6);
        // `अष्टकस्थापनम्`'s position and local (2); `अष्टकपाठः`'s position (1);
        // `अष्टकखण्डः`'s two bounds and local (3). MEASURED from this assertion's
        // own failure: 1298 + 54 + 997 = 2349.
        // 1159 (main: W-226 + W-202 + W-215) + 22 (W-239 part 1, named in its note) = 1181, MEASURED
        // on the merged tree 2026-09-04 (the test printed it); siblings, never summed by hand.
        // MERGE of main (left: through W-227 + W-239 part 1) with agent/d002a2 (right: encode.t1's
        // सङ्कोचः) on 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this
        // assertion's own failure, never summed.
        // (1298, 54, 975) -> (1299, 54, 1005) ON 2026-09-04, `D-002a2`, MEASURED
        // from this assertion's own failure and then RECONCILED BY NAME. Corpus
        // GROWTH: `encode.t1`'s last stub, `सङ्कोचः` (`compressed_at`), got its
        // body and its caller chain, and the new bindings are THIRTY `न६४` and
        // ONE `अ६४`. The `न६४`: `रूपसङ्कोचः`'s parameters `सम्बन्धः` and
        // `संख्यानम्`, its locals `शिष्टसंख्या`, `सूचकाङ्क`, `अवकाशगणना` (five);
        // `सङ्कोचः`'s locals `संख्यानम्`, `सूचकाङ्क`, `रूपसंख्या`, `रूपक्रमः`,
        // `पङ्क्तिः`, `आरम्भः`, `सीमा`, `नामादिः`, `नामान्तः`, `सम्बन्धः` (ten);
        // `आज्ञाविस्तारः`'s `ददाति न६४` (one); `अभिधानसङ्केतः`'s parameter `अंशाः`
        // and locals `पङ्क्तिः`, `आरम्भः`, `सीमा` (four); `विन्यासावृत्तिः`'s
        // `आज्ञादिः`, `आज्ञासूचकाङ्क`, `विस्तारः` (three); `सम्बन्धाङ्कः`'s
        // parameters `आरम्भः`, `सीमा`, its `ददाति न६४`, its locals `आदिः`,
        // `अवसानम्`, `दैर्घ्य`, `सङ्ख्याष्टकम्` (seven). The one `अ६४` is
        // `रूपसङ्कोचः`'s `चिह्नितम्`, a decoded operand that is signed at
        // vishlesana.rs:50 — the argument `encode.t1`'s own margin makes for
        // `मूल्यानि` at its `reduce` port. Six new sites were first spelled
        // `अ६४` by copying their neighbours and respelled before this number
        // was taken, which is why the seven this assertion first reported
        // became one. 1299 + 54 + 1005 = 2358.
        // 1181 (main through W-227 + W-239 part 1) + 30 (D-002a2's thirty न६४, named in its note) =
        // 1211, and अ६४ 1298 -> 1299 (रूपसङ्कोचः's चिह्नितम्), MEASURED on the merged tree 2026-09-04.
        // W-231 ON THE MERGE WITH W-215/W-227/W-239/D-002a2: 989 (this branch)
        // and 1211 (main) are SIBLINGS — neither counted the other's. Measured.
        // MERGE of main (left: through D-002a2) with agent/w236 (right: the T1 emitter twin) on 2026-09-04
        // by the trunk: the value below is MEASURED on the merged tree from this assertion's own failure,
        // never summed.
        // (1298, 54, 961) -> (1335, 54, 1143) ON 2026-09-04, `W-236`, MEASURED
        // (this test printed it; the sum 2532 = 2313 + 219) and then RECONCILED
        // BY NAME, never by arithmetic. Corpus GROWTH, three files, none moved:
        //   * `yantrotsarjana.t1` (NEW — the T1 twin of riscv64.rs): 32 `अ६४`
        //     and 178 `न६४`. Globals (23): यन्त्रावण्टनीयम्, यन्त्रस्तूपाष्टकाः,
        //     the ten `यन्त्र…निषेधभेद` constants, यन्त्रनिषेधभेद, यन्त्रनिषेधपर्व,
        //     यन्त्रनिषेधलक्ष्य, यन्त्रनिषेधसंख्या (अ६४), यन्त्रनामसूचकाङ्क,
        //     यन्त्रप्रवेशसंज्ञा, यन्त्राज्ञागणना (अ६४), यन्त्रवर्तमानवृत्ति,
        //     यन्त्राधिकरणसीमा, यन्त्रनिक्षेपाः, यन्त्रध्रुवसंख्यान, यन्त्रोच्चांशः (अ६४),
        //     यन्त्रनीचांशः (अ६४). Record fields (5): यन्त्रनाम's संज्ञा; यन्त्रचौकट's
        //     अष्टकाः (अ६४), रक्षितसंख्यान, पुनःस्थानस्थानम् (अ६४), निक्षेपसंख्यान.
        //     Parameters, returns and locals, by routine (182; `ददाति` counted
        //     as the census sees it, अ६४ marked, the rest न६४): यन्त्रनिषेधः भेद
        //     पर्व लक्ष्य संख्या(अ) ददाति; यन्त्रनामारम्भः ददाति; यन्त्रप्रवेशन्यासः
        //     संज्ञा ददाति; यन्त्रनामान्वेषणम् संज्ञा ददाति सूचकाङ्क; यन्त्रनामयोजनम्
        //     संज्ञा ददाति स्थलम्; यन्त्राज्ञान्तः ददाति; यन्त्रचिह्नान्तः ददाति;
        //     यन्त्राङ्कः मूल्यम्(अ) ददाति; यन्त्रकोष्ठनाम, यन्त्रक्षणिकनाम, यन्त्रार्थनाम
        //     क्रमाङ्क ददाति each; यन्त्रकोष्ठलेखनम् सङ्केतः ददाति; यन्त्रपदम् ददाति;
        //     यन्त्रकर्मपदम्, यन्त्रकरणपदम् सङ्केतः ददाति each; यन्त्रसङ्ख्यापदम्
        //     अङ्कम्(अ) ददाति; यन्त्रषोडशाङ्कचिह्नम् अङ्कम्; यन्त्रषोडशाङ्कः अंशाः(अ)
        //     ददाति आवरणम्(अ) प्रतिरूपम्(अ) शेषाः सरणम् अङ्कम्; यन्त्रवृत्तिचिह्नम्
        //     संज्ञा ददाति स्थलम्; यन्त्रवर्तमानचिह्नम् ददाति; यन्त्रवर्तमानचिह्नांशः
        //     आदिस्थलम्(अ); यन्त्रपर्वचिह्नम् पर्व ददाति; यन्त्रनिर्गमचिह्नम् ददाति;
        //     यन्त्रसंयुक्तसाम्यम् दैर्घ्यम् सूचकाङ्क वामाष्टकम् दक्षिणाष्टकम्;
        //     यन्त्रचिह्नपरीक्षा सूचकाङ्क स्थलम् धारकः पूर्वः आदिस्थलम्(अ);
        //     यन्त्रचौकटरचना कोष्ठम् मूल्यम् निक्षेपाष्टकाः(अ) रक्षिताः अपरिष्कृतम्(अ);
        //     यन्त्राधिकरणम् मूल्यम्; यन्त्रपठनम् मूल्यम् क्षणिकः अतिरिक्तम्(अ) ददाति
        //     सङ्केतः; यन्त्रलेखनम् मूल्यम् क्षणिकः ददाति; यन्त्रलेखनिधानम् मूल्यम्
        //     सङ्केतः ददाति; यन्त्रप्रस्तावना ददाति सूचकाङ्क; यन्त्रोपसंहारः ददाति
        //     सूचकाङ्क; यन्त्रोच्चनीचविभागः ध्रुवम्(अ) उच्चम्(अ) नीचम्(अ);
        //     यन्त्रध्रुवावतरणम् मूल्यम् ध्रुवम्(अ) ददाति गन्तृ स्थलम् सूचकाङ्क
        //     क्रमाङ्कः(अ); यन्त्राह्वानोत्सर्जनम् फलम् आह्वेयसंज्ञा आदानारम्भ
        //     आदानसंख्यान ददाति लक्ष्यस्थलम् स्तूपस्थाः(अ) समायोजनम्(अ) क्रमः स्रोतः
        //     कोष्ठक्रमः कोष्ठस्रोतः गन्तृ; यन्त्रशाखावतरणम् शर्तम् तदा अन्यत् अग्रिमम्
        //     ददाति शर्तकोष्ठम्; यन्त्रावसानोत्सर्जनम् पर्वाङ्कः अग्रिमम् ददाति भेदः
        //     प्रत्यागतम् स्रोतः लक्ष्यम्; यन्त्रपरीक्षा वृत्तिसूचकाङ्क आरम्भः सीमा प्रवेशः
        //     पर्वाङ्कः भेदः लक्ष्यम् अन्यलक्ष्यम् आज्ञाङ्कः शेषाज्ञा; यन्त्रपर्वबाह्यम्
        //     लक्ष्यम् आरम्भः सीमा; यन्त्रशाखादूरपरीक्षा वृत्तिसूचकाङ्क पर्वाङ्कः सीमा तदा
        //     चिह्नस्थानम्(अ) शाखास्थानम्(अ) दूरम्(अ); यन्त्रवृत्त्युत्सर्जनम्
        //     वृत्तिसूचकाङ्क ददाति नामस्थलम् आरम्भः सीमा प्रवेशः परमप्राचलः(अ) पर्वाङ्कः
        //     आज्ञाङ्कः शेषाज्ञा उच्चतमस्थानम्(अ) भेदः अग्रिमम् आज्ञाङ्कद्वि शेषाज्ञाद्वि
        //     फलाङ्कः आज्ञाभेदः गन्तृ प्राचलः; यन्त्रद्विपदोत्सर्जनम् ददाति वामकोष्ठम्
        //     दक्षिणकोष्ठम् गन्तृ; यन्त्रारम्भोत्सर्जनम् प्रवेशस्थलम् ददाति;
        //     यन्त्रदत्तोत्सर्जनम् ददाति सूचकाङ्क; यन्त्रमण्डलोत्सर्जनम् प्रवेशस्थलम्
        //     सूचकाङ्क प्राचलाः(अ) वृत्त्यङ्कः; यन्त्रप्राचलगणना वृत्तिसूचकाङ्क ददाति(अ)
        //     प्राचलसंख्या(अ) पर्वाङ्कः सीमा आज्ञाङ्कः शेषाज्ञा; यन्त्रयोगफलदृष्टान्तः
        //     ददाति प्रवेशारम्भः तदारम्भः आदानारम्भः अन्यदारम्भः मुख्यारम्भः
        //     मुख्यादानारम्भः.
        //   * `utsarjana.t1`: 3 `न६४` — `आवण्टनारम्भः`'s `सीमा` (the allocator now
        //     clears to the highest value मध्यरूप handed out, its note (f)) and
        //     `आयुर्निर्णयः`'s Call arm `आदानक्रमः`, `अवगणनपञ्च` (its note (a)).
        //   * `vishlesana.t1`: 5 `अ६४` and 1 `न६४` — `आधारयुक्तम्`, the bias step
        //     `decode16` has and `उद्धरणम्` lacked (`W-233`): `मूल्यम्`(अ),
        //     `ददाति`(अ), `विस्तारः`, `व्याप्तिः`(अ), `न्यूनता`(अ), `गुणकः`(अ) — signed
        //     because they are the field's reading and the routine answers `अ६४`.
        // 1335 + 54 + 1143 = 2532 on W-236's branch before the merge.
        // MEASURED 2026-09-04 on the merged tree (this test printed it): W-226/W-228's
        // (1298, 54, 975) and W-236's 219 bindings together — 1335 + 54 + 1157 = 2546.
        // main (1299, 54, 1211) + W-236 (+219 named on its tree: 210 new file, 3 allocator, 6 bias
        // step — its अ६४ 1298 -> 1335 and न६४ 975 -> 1157 on ITS tree) = (1336, 54, 1393) on the MERGED
        // tree, MEASURED 2026-09-04 (the test printed it); siblings, never summed by hand.
        // MERGE of main (left: through W-236) with agent/w239 (right: the octet arena wired) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // (1299, 54, 1211) -> (1309, 54, 1225) on 2026-09-04, W-239 part 2 merged onto main after D-002a2, the
        // callers in `vakyavibhaga.t1` wired to the arena: FOURTEEN `न६४` bindings
        // ADDED — `आरम्भः`'s `अष्टकशून्यम्` (1); the ASCII and web arms' arena
        // start, count, space and written locals (4 + 4, the web arm's second
        // written `लिखितद्वि` making its 5); the string case's start, count,
        // space and written (4). ELEVEN `अ६४` bindings NET (22 added, the
        // returns of the code-table builders, `कोष्ठकनाम`'s stepped start, the
        // datum indexes, the web dictionary's line and field bounds, the string
        // case's checks and pieces, the width arm's counts; 9 REMOVED with the
        // half-built stash and its two local pushes). None moved. MEASURED from
        // this assertion's own failure on the merged tree: 1309 + 54 + 1225 = 2588 (ten
        // `अ६४` net, not eleven: W-227's registry form is kept and the loader's
        // `योजितम्` binding is gone with mine).
        // main (1336, 54, 1393) + W-239's (+14 न६४, +13 अ६४ net, named in its note) = (1346, 54, 1407),
        // MEASURED on the merged tree 2026-09-04 (the test printed it); siblings, never summed by hand.
        // (1346, 54, 1407) -> (1316, 54, 1397) ON 2026-09-04, `W-237`, SAID OUT LOUD: a DELETION,
        // the first in this ledger's history that removed sites rather than respelling them.
        // `utsarjana.t1` lost the retired T0 pair — `कार्यक्रमोत्सर्जनम्`, `अधिकरणवचनम्`,
        // `त्रिपदवचनम्` and the orphaned `टिप्पनीचिह्नयोजनम्` — whose text was not T0 (research/25
        // §1.2): 30 `अ६४` (the output-buffer offsets `लेख*`, `आदिस्थलम्`, the appenders' returns
        // they held) and 12 `न६४` (their index and register locals) went with them. `ir.t1`
        // gained 2 `न६४` (`नामसंज्ञा`, `अवगणननाम`, the नाम arm that lowers a zero-argument
        // call). Read from the failing assertion, twice: (1317, 54, 1399) after the three, then
        // (1316, 54, 1397) after the fourth; the pin is the second.
        // 1397 -> 1405 ON 2026-09-04, `W-240` (lane addition), MEASURED (the test
        // printed it) and reconciled by name — eight `न६४` bindings: `ast.t1`'s
        // `स्थानसूचकाङ्क` (the statement's first token — this field WAS
        // `स्थान ॱॱ अङ्कः अन्तः अ८`, never written; retyped, so one octet-run
        // position became a `न६४` one); `parse.t1`'s `वाक्यारम्भस्थान` and the two
        // `आरम्भस्थानम्` locals (`वाक्यपठनम्`, `समूहपठनम्`); `artha.t1`'s
        // `प्रकारदोषपङ्क्ति` and `वाक्यपङ्क्तिः`'s parameter, its `ददाति न६४`
        // and its `चिह्नकाङ्क`. 1316 + 54 + 1405 = 2775.
        // MERGE of main (left: through W-240) with agent/w243 (right: the inter-module image) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // (1316, 54, 1397) -> (1316, 54, 1398) on 2026-09-04, `W-243`: ONE `न६४`
        // binding ADDED, `yantrotsarjana.t1`'s `यन्त्रारम्भमण्डलोत्सर्जनम्` (the
        // startup object, twin of `emit_startup_object`) binding its entry
        // position `प्रवेशस्थलम्`; none moved. MEASURED from this assertion's own
        // failure: 1316 + 54 + 1398 = 2768.
        // main (1316, 54, 1405) + W-243's one न६४ (प्रवेशस्थलम्, named in its note) = (1316, 54, 1406),
        // MEASURED on the merged tree 2026-09-04 (the test printed it).
        // MERGE of main (left: through W-243) with agent/w231 (right: the checker's four arms) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // W-231 on the merge with main at 927ca2b6: 1222 (this branch) and 1393
        // (main) are SIBLINGS — neither counted the other's. Measured here.
        // 1393 -> 1404, W-231's ELEVEN `न६४`, named: `घोषणम्`'s `घोषणाप्रकारः`
        // and `प्रकारयोग्यम्`'s guard; the call arm's `पात्रम्`/`पात्रसंज्ञा`; the
        // field arm's `वस्तुसंज्ञा`, `घोषणासूचकः` and `क्षेत्रक्रमः`; the arena
        // `संज्ञाघोषणाकोश` and its handshake `घोष्यमाणघोषणा`; and the two new
        // counters `अयुग्मशाखासंख्या` and `अबूलशर्तसंख्या`.
        // main (1316, 54, 1406) — after W-237's deletions and W-243 — + W-231's eleven न६४ (named in its
        // note; its own tree read 1393 -> 1404) = (1316, 54, 1417), MEASURED on the merged tree 2026-09-04.
        // (1316, 54, 1417) -> (1316, 54, 1418) on 2026-09-04 at the trunk's merge of W-231: दुष्टवाक्यस्थान respelled from an अ८ run to a
        // न६४ token index (the semantic conflict with W-240), MEASURED (the test printed it).
        // MERGE of main (left: through W-244) with agent/w223 (right: the declaration store) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // (1316, 54, 1406) -> (1316, 54, 1486) on 2026-09-04, `W-223` part 1: EIGHTY
        // `न६४` bindings ADDED, all in the NEW `sanchaya.t1` (the shared declaration
        // store): eight globals (the four arena cursors, the text store's length,
        // the refusal kind, the two kind constants); fifteen record fields (the
        // module entry's three, the entry's eight, the parameter entry's four —
        // every one an index or a range into the store); twenty-two parameters
        // (indices, ranges, kinds); seventeen locals (cursors, ranges, module
        // and entry indices); eighteen `ददाति न६४` returns (every appender,
        // collector and lookup answers an index or a count). None moved. MEASURED
        // from this assertion's own failure: 1316 + 54 + 1486 = 2856.
        // main (1316, 54, 1417) + W-223 part 1's eighty न६४ (all sanchaya.t1's, named by group in its
        // note; its own tree read 1406 -> 1486) = (1316, 54, 1498) on the MERGED tree, MEASURED.
        // MERGE of main (left) with agent/w223b (right: the extent) on 2026-09-04 by the trunk:
        // the value below is MEASURED on the merged tree from this assertion's own failure.
        // MERGE of main (left: through W-244) with agent/w223 (right: part 1) on
        // 2026-09-04 by W-223 part 2. 1418 (main, through W-231's eleven and the
        // `दुष्टवाक्यस्थान` respelling) and 1486 (part 1's EIGHTY, all in the new
        // `sanchaya.t1`: eight globals, fifteen record fields, twenty-two
        // parameters, seventeen locals, eighteen `ददाति न६४` returns) are
        // SIBLINGS off 1406 — neither counted the other's. Nothing moved; a new
        // file joined the corpus. -> (1316, 54, 1498), MEASURED here from this
        // assertion's own failure, never summed.
        // (1316, 54, 1498) -> (1316, 54, 1502) on 2026-09-04, `W-223` part 2: FOUR
        // `न६४` ADDED, all in `artha.t1` and all named — `परिधिदोषभेद` (which way the
        // extent was wrong), `परिधिअन्तः` (the extent the caller actually passed, kept
        // because "the extent was bad" does not say WHICH), and the two kind constants
        // `परिधिऋणभेद` and `परिधिबाह्यभेद`. None moved; the walk's own `सीमा` and `क्रमः`
        // were already `अ६४` and stay so. MEASURED from this assertion's own failure.
        // (1316, 54, 1502) -> (1316, 54, 1528) on 2026-09-04, `W-223` part 2:
        // TWENTY-SIX `न६४` added and NO `अ६४` — the store's four counters and its
        // two dense intern runs with their cursor, the member refusal's line, the
        // located untypable statement, `संज्ञाग्रहणम्`'s local and return,
        // `सञ्चयसंज्ञा`'s and `सदस्यनिर्णयः`'s parameters, returns and locals, the two
        // qualified branches' bound arguments, and the empty runs
        // `निर्णायकारम्भः` assigns. `संज्ञाग्रहणम्` was written `न६४` DELIBERATELY: it
        // was extracted from `घोषणम्`, whose local is `अ६४`, and copying that would
        // have moved the first number of this triple UP — against the whole
        // direction of ADR-0030. Checked: the corpus is green with the helper at
        // `न६४` and `घोषणम्`'s own binding untouched. MEASURED from this
        // assertion's own failure, never summed.
        // (1316, 54, 1528) -> (1317, 54, 1530) on 2026-09-04, `W-247`, a SIBLING of
        // part 2 off (1316, 54, 1502): ONE `न६४` REMOVED — `encode.t1`'s dead
        // five-field `संरचना प्रतीक्षा` (its `कोष्ठकम्`), the corpus's one duplicate
        // declaration, which nothing built or read — and THREE `न६४` ADDED in
        // `artha.t1` by the redeclaration refusal (`पूर्वघोषणापङ्क्ति`,
        // `पुनर्घोषणापङ्क्ति`, the local `पूर्वघोषणाङ्कः`), net +2; and ONE `अ६४`,
        // the local `परिसरदैर्घ्य`, a scope count spelled as `पुनरुक्तघोषणम्`
        // spells its own. None moved. MEASURED from this assertion's own
        // failure: 1317 + 54 + 1530 = 2901.
        // MERGE of main with agent/driver (the numeral reader) on 2026-09-05 by the trunk:
        // MEASURED on the merged tree from this assertion's own failure, never summed.
        // (1316, 54, 1528) -> (1317, 54, 1529) on 2026-09-05, `W-257`, WHICH IS
        // REVERTED AND THE TWO BINDINGS ARE GONE AGAIN: an accumulator was added
        // to `अक्षरकोश ॱ सङ्ख्या` on the reading that it should answer a numeral's
        // VALUE. It should not. `सङ्ख्या` IS THE RADIX READER — `t1_execution.rs`'s
        // corpus agreement pins it against Rust's `radix`, and the file's value
        // reader is `मानम्` beside `मानदोषः` — so the routine was correct and the
        // change made it wrong, red on main in three tests. The two bindings it
        // added (`मूल्यम्`, `अङ्कमानम्`) go with it.
        // MEASURED on the merged tree 2026-09-05 (the test printed it).
        // (1317, 54, 1530) -> (1317, 54, 1537) on 2026-09-05, `W-259`: SEVEN `न६४`
        // bindings ADDED in `ir.t1`, all by the parameter path and the census's
        // discriminator — three globals (`प्राचलचिह्नककोश`, `प्राचलमूल्यकोश`,
        // `प्राचलसंख्या`; `अङ्कस्रोतःकोश` is a slice and no site), the parameter
        // loop's `प्राचलाङ्कः` and `अवगणनप्राचल`, the name arm's `प्राचलस्थानम्`,
        // and `चिह्नकपाठसाम्यम्`'s `साम्यक्रमः`. None moved. MEASURED from this
        // assertion's own failure: 1317 + 54 + 1537 = 2908.
        // MERGED 2026-09-05: (1317, 54, 1537) here and (1322, 54, 1654) on the
        // other lane are siblings off (1316, 54, 1528) — this side's W-247,
        // W-257 and W-259 bindings, that side's eleven instruction kinds, its
        // frame slots and its stub census. The merged tree measures
        // (1323, 54, 1656), taken from this assertion's own failure and never
        // computed: 1323 + 54 + 1656 = 3033.
        // (1323, 54, 1656) -> (1324, 54, 1660) on 2026-09-05, `W-248`,
        // RE-MEASURED FROM THIS ASSERTION'S OWN FAILURE. The type-text reader
        // adds one `अ६४` (`प्रकारपाठशेषः`'s विवराष्टकम्, the space it scans past)
        // and four `न६४` (`प्रकारपाठार्थः`'s `भेदः`, `प्रकारपाठशेषः`'s `आरम्भः`,
        // and the two the refusal run needs — `अज्ञातप्रकारपाठसंख्या` and
        // `अज्ञातप्रकारपाठसूचकाङ्क`). `इ६४` is unmoved at 54, which is the half
        // that matters: this row wrote no new signed-64 site.
        // (1324, 54, 1660) -> (1326, 54, 1709) on 2026-09-05, `W-265`,
        // RE-MEASURED FROM THIS ASSERTION'S OWN FAILURE and never computed.
        // Giving a named type a symbol adds two `अ६४` — both are declaration
        // ranges walked with the extent `कार्यक्रमनिर्णयः` already uses
        // (`प्रकारक्रमः`, pass one (b)'s cursor, and `प्रपरिसरदैर्घ्य`, the scope
        // stack's own length) — and 49 `न६४`, which are what the type-name
        // table IS: four parallel dense runs and a cursor, five refusal-reason
        // constants, three counters, and the module indices, kinds, symbols and
        // slot numbers that `प्रकारसंज्ञायोजनम्`, `प्रकारसंज्ञान्वेषणम्`,
        // `प्रकारसंज्ञासंख्या`, `प्रकारसंज्ञाप्रथमम्`, `नामप्रकारार्थः`,
        // `प्रकारघोषणार्थभेदः` and `सञ्चयप्रकारबन्धः` pass between them. Every
        // one is an index, a count or a kind, which is what `न६४` is for.
        // `इ६४` IS UNMOVED AT 54, and that is the half of this triple that
        // matters here for a second reason: this row's reader now REFUSES the
        // spelling by name (`अनामप्रकारकारण`) rather than reading it as silent
        // poison, so ADR-0030's 54 sites are visible from inside the compiler
        // for the first time as well as from this ledger.
        // (1327, 54, 1750) -> (1328, 54, 1757) on 2026-09-06, `W-278`,
        // RE-MEASURED FROM THIS ASSERTION'S OWN FAILURE and never computed.
        // The files are `ir.t1` and `yantrotsarjana.t1`. The single `अ६४` is
        // the global's initial value — a WORD held in the data section, which
        // is what `अष्टाष्टकाः` lays, and the same width `यन्त्रध्रुवावतरणम्`
        // gives a pooled constant. The 7 `न६४` are the globals table and its
        // reader: the cursor, the walk index in the data block, and the counts
        // and indices the declaration branch passes between `व्याकरॱघोषणाकोश`
        // and the three parallel arrays. Every one is an index or a count.
        //
        // `इ६४` IS UNMOVED AT 54, and it is the load-bearing half again: a
        // global's ADDRESS is one word and its stored value is `अ६४`, so
        // nothing here wanted a signed sixty-four. A row that lowered a READ
        // of a signed global and still did not move 54 would be the thing to
        // look at twice — this row lowers the read, not the value's type.
        //
        // (1328, 54, 1757) -> (1328, 54, 1759) LATER THE SAME DAY, `W-278`,
        // RE-MEASURED FROM THIS ASSERTION'S OWN FAILURE and never computed.
        // The 2 `न६४` are two locals the row's SECOND commit added while
        // guarding arena reads that had been unguarded: `घोषणास्थलम्`, the
        // place a declaration sits, and `घोषितम्`, what
        // `अर्थॱसंज्ञाघोषणाकोश` answers at the name's index. Both are an
        // index or the count read at one, so the rule this ledger has applied
        // throughout holds without an exception.
        //
        // WORTH THE SENTENCE, because it is why this assertion went red at
        // all: THOSE TWO SITES LANDED IN A COMMIT WHOSE OWN MESSAGE SAID IT
        // WAS NOT LANDABLE, and the ledger was re-taken BEFORE it rather than
        // after. A pin re-measured mid-row measures the tree it was run on,
        // not the row — so the re-take has to be the LAST thing in a row that
        // keeps editing `.t1`, or it has to be done again. `इ६४` unmoved at
        // 54 for the third consecutive re-take, and the reason is unchanged.
        //
        // (1326, 54, 1709) -> (1327, 54, 1750) on 2026-09-06, `W-254`,
        // RE-MEASURED FROM THIS ASSERTION'S OWN FAILURE and never computed.
        // The files are `ir.t1` and `yantrotsarjana.t1`. The single `अ६४` is
        // `यन्त्रपाठावतरणम्`'s `क्रमाङ्कः`, the string pool index that goes into
        // the label `पाठ<k>` — the same spelling `यन्त्रध्रुवावतरणम्` already
        // gives the constant pool's, so it is a sibling and not a new kind of
        // thing. The 41 `न६४` are what a variable-length pool IS on this side:
        // where the constant pool is one array of words, a string pool is an
        // octet run plus a start and a count per entry, so `पाठारम्भ` and
        // `पाठसंख्यान` on the instruction, `पाठाक्षरसूचकाङ्क`,
        // `यन्त्रपाठाक्षरसंख्यान`, `यन्त्रपाठसंख्यान` and the three parallel
        // arrays' cursors, and the walk indices that `पाठाक्षरयोजनम्`,
        // `पाठाक्षरांशयोजनम्`, `पाठाज्ञायोजनम्` and the उक्त arm pass between
        // them. Every one is an index, a count or an offset.
        //
        // `इ६४` IS UNMOVED AT 54, and here that is the load-bearing half: a
        // string literal's octets are `अ८` and its ADDRESS is one word, so
        // nothing in this row wanted a signed sixty-four. A row that added to
        // 54 while lowering a value type would be the thing to look at twice.
        // (1327, 54, 1750) -> (1355, 54, 1784) on 2026-09-06, `W-kosha` phase 2,
        // RE-MEASURED FROM THIS ASSERTION'S OWN FAILURE. The file is `kosha.t1`,
        // which had ZERO routines and now writes an ELF image. 28 `अ६४` and 34
        // `न६४`: the writer's own offsets and sizes — `भारस्थानम्`, the header
        // and program-header sizes, `पाठस्थानम्`, `दत्तस्थानम्`, `भारमानम्`,
        // `स्मृतिमानम्` — plus the octet each of the three little-endian
        // appenders splits and the twenty-odd `अवगणना` bindings that hold the
        // arena's answer for each field written.
        //
        // `इ६४` IS UNMOVED AT 54 AGAIN, and it is load-bearing twice over here:
        // every quantity in an ELF header is a COUNT, an OFFSET or an ADDRESS,
        // and not one of them is signed. A row that added to 54 while writing a
        // file format would be describing something that can be negative.
        // (1355, 54, 1784) -> (1354, 54, 1786) on 2026-09-06, same row, second
        // reading: the padding WALK became arithmetic when
        // `no_yavat_in_the_corpus_is_without_a_step_or_an_exit` refused it. One
        // `अ६४` went (the loop's octet accumulator) and two `न६४` arrived (the
        // pad count and the call's answer). RE-MEASURED FROM THIS ASSERTION.
        //
        // A TRIPLE THAT MOVED IN BOTH DIRECTIONS AT ONCE is worth a second look
        // and this one is right: removing a loop should lose a counter, and
        // computing a length should gain one.
        // (1354, 54, 1786) -> (1355, 54, 1795) on 2026-09-06, THE MERGE OF
        // agent/kosha AND agent/nameglobal, RE-MEASURED FROM THIS ASSERTION'S
        // OWN FAILURE. Neither branch's number survived the merge and neither
        // was wrong: both lanes added corpus, so each figure was correct for
        // its own tree at its own commit and false for the tree that holds
        // both. A census pin is the one artefact GUARANTEED to conflict when
        // two lanes add corpus at once, and the resolution is never arithmetic
        // — the five conflicting hunks were resolved to one side and then every
        // figure re-taken here.
        //
        // The rows are `ir.t1` and `yantrotsarjana.t1`, 255 lines of `.t1`
        // lowering `name_global`. +1 `अ६४` and +9 `न६४`.
        //
        // `इ६४` IS UNMOVED AT 54 for the third reading running. Not asserted
        // beyond that: this triple counts positions and cannot say which site
        // is which, so what a moved row MEANS is a claim the row's own margin
        // must make, not this one.
        // (1355, 54, 1795) -> (1356, 54, 1795) on 2026-09-06, the integration
        // of agent/nameglobal AND agent/vastu-objects onto agent/kosha, TAKEN
        // FROM THIS ASSERTION'S OWN FAILURE — and this one was PREDICTED
        // BEFORE IT WAS MEASURED, which is the only reason it says anything.
        //
        // Base (1354, 54, 1786); nameglobal measured +1 `अ६४` +9 `न६४`;
        // vastu-objects measured +1 `अ६४`. If the two are independent the
        // tree holding both must read (1356, 54, 1795), and it does. A
        // predicted census that lands is evidence the changes do not
        // interact; had it read anything else, the difference would have been
        // a finding about the two changes rather than a bookkeeping problem.
        //
        // That is worth more than the number. A re-taken figure carries no
        // evidence on its own — it is whatever the tree says. A figure that
        // was written down BEFORE the run and then matched is a refutable
        // claim that survived.
        // (1356, 54, 1795) -> (1356, 54, 1797) on 2026-09-07, THE BAD-FIELD
        // REFUSAL, MEASURED FROM THIS ASSERTION'S OWN FAILURE and never
        // computed. Only `न६४` moved, by 2: the new family's count
        // `असत्क्षेत्रसंख्या` and its line `असत्क्षेत्रपङ्क्ति`. `अ६४` UNMOVED and
        // `इ६४` UNMOVED, and both readings are checks rather than decoration —
        // this change wrote no unsigned site and, more to the point, no `इ६४`,
        // which is the spelling the frozen grammar does not define and which
        // this ledger exists to stop growing. The family's two TEXTS are
        // `अङ्कः अन्तः अ८` and its flag is `बूल`, so neither reaches this triple.
        // (1356, 54, 1797) -> (1356, 54, 1813) on 2026-09-07, the driver merge,
        // TAKEN FROM THIS ASSERTION'S OWN FAILURE. `shrinkhala.t1` is the twentieth
        // source and the FIRST module that sits ABOVE the passes rather than beside
        // them. `अ६४` and `इ६४` both UNMOVED: a stage sequence is offsets and counts,
        // and not one integer of another width.
        // (1356, 54, 1813) -> (1356, 54, 1825) on 2026-09-07, the field-lowering
        // merge, TAKEN FROM THIS ASSERTION'S OWN FAILURE. `अ६४` and `इ६४` both
        // UNMOVED: a field offset is a count of words, and the lowering adds
        // twelve `न६४` and not one integer of another width.
        // (1356, 54, 1825) -> (1356, 54, 1853) on 2026-09-07, the object
        // builder. `अ६४` AND `इ६४` UNMOVED across 216 code lines, which is the
        // load-bearing half: every count, index and cursor here is `न६४`, the
        // width-specific fields are `अ३२` because the records declare them so,
        // and only octet-carriers are runs. An `अ६४` move would mean a WIDTH
        // had been named where this corpus names a ROLE.
        //
        // PREDICTED +29 AND MEASURED +28, and the missing one is a counting
        // error worth recording rather than a corpus surprise. The prediction
        // counted `ॱॱ न६४` and `ददाति न६४` in the diff's ADDED lines: 29. But
        // `प्रतीक्षायोजनम्`'s signature was WIDENED, so the diff carries it as
        // one removal and one addition, and the removed line held a
        // `ददाति न६४` of its own. Net 28.
        //
        // AN EDITED LINE IS A REMOVAL PLUS AN ADDITION, and counting only the
        // added side over-counts by whatever the old line carried. The same
        // prediction computed NET for the keyword census — where nothing was
        // removed, so it cost nothing — and additions-only here, where it did.
        // (1356, 54, 1853) -> (1356, 54, 1871) on the merge with agent/index
        // and agent/storeprobe. `अ६४` and `इ६४` STILL UNMOVED across three
        // lanes' corpus growth, which is the reading that survives being
        // unattributable: whatever the three rows added, none of them named a
        // WIDTH where this corpus names a ROLE.
        // (1356, 54, 1871) -> (1357, 54, 1876) on the driver's back half.
        // `अ६४` MOVED, +1, AND THE PREDICTION SAID IT WOULD NOT.
        //
        // The site is one line: `चरः रिक्तम् ॱॱ अ६४ भवति वाक्यविभागॱआरम्भः ।`
        // in `सङ्कलनारम्भः`. It is `अ६४` because `आरम्भः` ANSWERS `अ६४` — the
        // binding follows its callee's declaration, which is correct, and the
        // value is discarded because the call is made for its effect on the
        // table cursors.
        //
        // THE PREDICTION WAS CARRIED FORWARD FROM A DIFFERENT DIFF. "`अ६४`
        // unmoved" held for the object builder — 216 lines with every count and
        // cursor `न६४` — and was recited for this row without recounting it.
        // A claim true of one change is not a property of its author.
        //
        // `न६४` +5: four bindings and one return, counted from the diff and
        // matching. `इ६४` unmoved.
        //
        // (1357, 54, 1876) -> (1357, 54, 1883) on the corpus walk and the
        // `%pcrel_lo12` emitter. `अ६४` AND `इ६४` BOTH UNMOVED, and this time the
        // prediction was not recited — the note above records that "`अ६४`
        // unmoved" was carried forward from a different diff and was wrong, so
        // it was left unpredicted here and TAKEN FROM THE ASSERTION.
        //
        // `न६४` +7, and the sites are countable: `मण्डलानिप्रतिबिम्बम्`'s
        // `वस्तुसंख्या`, `तत्परम्`, `क्रमः` and `पुनरारम्भः`; the module global
        // `संयोजितवस्तुसंख्या`; and `पाठखण्डसंज्ञाङ्कः`, the index of the section
        // symbol a `%pcrel_lo` names. Six by inspection against seven measured —
        // THE COUNT IS THE ASSERTION'S, NOT MINE, and the one I cannot place is
        // recorded as unplaced rather than argued away.
        //
        // (1357, 54, 1883) -> (1357, 54, 1888) on 2026-09-09, THE SYMBOL-MAPPING
        // ROW, TAKEN FROM THIS ASSERTION'S OWN FAILURE. `अ६४` UNMOVED at 1357
        // and `इ६४` at 54 — this row converted none and added no signed integer.
        //
        // THE DIFF SHOWS `न६४` NET +7 AND THIS COUNTED +5. The two are
        // reconciled and the gap is not slack: **`सम्भाव्य न६४` is its own
        // annotation key** and does not reach this bucket, so of ten `न६४`
        // added, two are the optionals `निर्णीता` and `वैनिर्णीता` — both
        // answers from `अर्थॱनामनिर्णयः`, which returns "no such name" rather
        // than a symbol. 10 − 2 optional − 3 removed = 5.
        //
        // I FIRST WROTE A LIST OF FIVE SITES FROM MEMORY OF MY OWN CHANGE AND IT
        // WAS WRONG — it included `वैनिर्णीता`, which is one of the two that do
        // not count, and omitted the routine walk's. Counted from the diff
        // instead. The reading above this one placed six against seven and
        // recorded the odd site as unplaced rather than arguing it away; the
        // same discipline, one layer earlier: **count the sites, do not
        // enumerate them from what you remember writing.**
        // (1357, 54, 1888) -> (1357, 54, 1901) on 2026-09-09, THE `batch-1` PIN
        // RE-TAKE, TAKEN FROM THIS ASSERTION'S OWN FAILURE. `अ६४` UNMOVED at
        // 1357 and `इ६४` at 54 — this batch converted none and added no signed
        // integer.
        //
        // +13 RECONCILED AT BOTH ENDPOINTS, NOT ONLY IN THE DELTA. A
        // margin-filtered grep of `ॱॱ न६४` declarations plus `ददाति न६४`
        // returns gives 1667 + 221 = 1888 on `4812ac9c` and 1679 + 222 = 1901
        // on the batch. Matching the OLD value as well as the new is what says
        // the pattern's population is this bucket's; a delta can agree by
        // accident, two endpoints do not. THE OPTIONALS WARNING ABOVE IS WHY
        // THAT WORKS: `ॱॱ सम्भाव्य न६४` puts `सम्भाव्य` between the marker and
        // the type, so this pattern cannot match it — and `ir.t1` holds two.
        //
        // 12 OF THE 13 ARE `agent/xmodule`'S AND 1 IS `agent/lexrung`'S:
        //   declarations +12 = 11 + 1. The eleven are nine names new to the
        //     corpus — गणना, बाह्यक्रमः, बाह्यगणना, बाह्यगवेषणम्, बाह्यदैर्घ्यम्,
        //     बाह्यसंज्ञा, बाह्यसंज्ञारम्भः, बाह्यसंज्ञासूचकाङ्क, बाह्यादिः, the
        //     cross-module external-name arenas and the walk that reads them —
        //     PLUS TWO OCCURRENCES UNDER NAMES THE CORPUS ALREADY HAD,
        //     `क्रमः` 52 -> 53 and `स्थलम्` 13 -> 14. The one is
        //     `agent/lexrung`'s `सङ्कलनविफलसंख्या`.
        //   returns +1 = `यन्त्रबाह्यनामसञ्चयः ददाति न६४`, the arena's reader.
        //     Renamed from the bare `बाह्यनामसञ्चयः` on 2026-09-09 in the same
        //     batch, for `W-192`'s module prefix; the count is unaffected.
        //
        // THE NAME LIST IS NOT A ONE-TO-ONE ENUMERATION OF THE COUNT and is not
        // offered as one: nine names carry eleven declarations. The row above
        // records writing five sites from memory and getting them wrong; these
        // were taken by `comm` against `4812ac9c` per branch, and the two
        // reused names are exactly what an enumeration from memory would drop.
        //
        // (1357, 54, 1901) -> (1357, 54, 1909) on 2026-09-09, W-280. `न६४` +8,
        // `अ६४` and `इ६४` UNMOVED — this row adds no signed integer and converts
        // none. The eight are the exit-kind global and its seven constants in
        // `shrinkhala.t1`: `सङ्कलनविरामभेद` plus `सिद्ध`, `अघोषणा`, `अनिर्णय`,
        // `अवृत्ति`, `नामवैषम्य`, `अनाम`, `अनारब्ध`.
        //
        // NINE DECLARATIONS MATCH THE PATTERN AND THE COUNT MOVED BY EIGHT, and
        // the difference is not slack: `सङ्कलनविफलसंख्या` was added in the fold
        // and is already inside the `batch-1` re-take above. **A count and a
        // grep disagreeing by one is a question, and the answer was which tree
        // each was taken on** — the same pairing error that made a delta name
        // two trees earlier tonight.
        // (1357, 54, 1909) -> (1357, 54, 1916) on 2026-09-10, W-279. `न६४` +7,
        // `अ६४` and `इ६४` UNMOVED for the same reason as the row above: no signed
        // integer added, none converted.
        //
        // **+7 IS 8 ADDED MINUS 1 DELETED, AND AN EARLIER VERSION OF THIS MARGIN
        // NAMED THE WRONG MEMBER.** It said the −1 was `सङ्कलनगणनाभङ्गभेद`, the
        // sign-guard exit. That exit was added AND withdrawn inside this branch,
        // so against `origin/main` it is net ZERO and cannot be anyone's −1. The
        // arithmetic was right and the attribution was wrong, which is the same
        // failure this unit corrected three times over — a count right about the
        // wrong members reads exactly like a count that is right.
        //
        // The members, checked against `w268-census`'s four keys rather than
        // recalled:
        //   +8  the module-scope declarations: `सङ्कलननामप्रयासाः` and
        //       `सङ्कलननामप्रविष्टयः` (the two counts behind exit ४, because a
        //       mismatch exit is one number where the question needs three),
        //       and `वस्तुरचनाविरामभेद` with its five constants `सिद्ध`, `वाक्य`,
        //       `रिक्त`, `अष्टक`, `अनारब्ध`. `public-bindings` moved +8 with no
        //       subtraction, which is what proves these eight are the whole of
        //       the addition.
        //   −1  the LOCAL `चरः नामपूर्वम्`, deleted. It read the name cursor so
        //       the pass could subtract; the subtraction spanned
        //       `नामसञ्चयः`'s reset-on-entry and produced −३९ entries written.
        //       `bindings` moved +7 where `public-bindings` moved +8, and that
        //       difference of one IS this local.
        //
        // PREDICTED AS +7 BEFORE THE GATE RAN, from the diff and not from memory.
        // A pin that is merely re-stamped ratifies itself; this one matched an
        // enumeration of its own members, which is the check that it moved for
        // the reason claimed and not for another.
        // (1357, 54, 1916) -> (1357, 54, 1920) on 2026-09-10, W-279's cross-module
        // field lowering, TAKEN FROM THIS ASSERTION'S OWN FAILURE. `अ६४` unmoved at
        // 1357 and `इ६४` at 54; the +4 is this arm's four new `न६४` locals —
        // `सञ्चयक्रमः`, `वस्तुप्रविष्टिः`, `दूरक्षेत्रसंख्या`, `दूरक्रमः` — the interning
        // scan's cursor and found entry, and the store walk's bound and index.
        // `दूरनामपाठ` is an octet run and is not one of them.
        //
        // (1357, 54, 1920) -> (1357, 54, 1933) on 2026-09-11, `W-283`'s ruled
        // storage model, TAKEN FROM THIS ASSERTION'S OWN FAILURE. `अ६४` unmoved
        // at 1357 and `इ६४` at 54 — this row converted none and added none, so
        // the ledger's two other columns are a control on the third.
        //
        // PREDICTED AS +13 FROM THE DIFF BEFORE THE VALUE WAS READ, and split by
        // file before it was summed:
        //
        //   ir.t1            +3   the three kind constants — वैश्विकस्थान,
        //                         स्थानाहार, स्थाननिधान.
        //   yantrotsarjana   +10  यन्त्राधिकरणपदम् (its parameter and its
        //                         answer, 2); the AddrOfGlobal emitter (answer
        //                         + गन्तृ, 2); the LoadAt emitter (answer +
        //                         स्थानम् + गन्तृ, 3); the StoreAt emitter
        //                         (answer + स्थानम् + स्रोतः, 3).
        //
        // THE STORE EMITTER HAS NO `गन्तृ` AND THAT ABSENCE IS THE DESIGN, not
        // an oversight in this count: it defines no result value, which is
        // precisely why it is invisible to dead-code elimination unless
        // `is_side_effecting` names it. The missing binding here and the
        // survival test in `opt.rs` are the same fact seen from two files.
        //
        // (1357, 54, 1933) -> (1357, 54, 1946) on 2026-09-11, `W-283`'s first
        // lowering, TAKEN FROM THIS ASSERTION'S OWN FAILURE. `अ६४` unmoved at
        // 1357 and `इ६४` at 54 — this row converted none, so the ledger's two
        // other columns are a control on the third.
        //
        // +13, DERIVED FROM THE DIFF BEFORE THE VALUE WAS READ: 14 added `न६४`
        // minus ONE removed. The removal is `सार्वजनिक चरः रचितशेषसीमा ॱॱ न६४
        // भवति २८`, replaced by its २९ form — a MODIFIED line, whose `न६४` is
        // added and removed in the same edit. A naive count of the added side
        // reads 14 and the answer is 13; every figure this row moved has the
        // same one-line correction in it.
        // (1357, 54, 1946) -> (1358, 54, 1976) ON 2026-09-12, THE SAME THREE
        // STORAGE LANDINGS, RE-TAKEN BY THE TRUNK. **CORPUS GROWTH, NOTHING
        // RESPELLED**: 30 `न६४` added and 0 removed, 1 `अ६४` added and 0
        // removed, so the sum moves 3357 -> 3388 by exactly the 31 added
        // bindings. ADR-0030 is not regressing — a respelling would show as a
        // REMOVAL in one bucket against an addition in another, and there is no
        // removal in any bucket.
        //
        // THE ONE NEW `अ६४` IS NAMED, BECAUSE A NEW `अ६४` IS THE THING THIS
        // LEDGER EXISTS TO CATCH: `yantrotsarjana.t1`'s `यन्त्ररचनाष्टकाः`, the
        // record region's octet count. It is a SIZE IN OCTETS, which is the
        // category ADR-0030 leaves as `अ६४`, and it is the only one.
        //
        // THE 30 `न६४` ARE THE STORAGE KINDS AND THEIR RECEIVERS, by file:
        // `ir.t1` 14 (`क्षेत्रस्थानाज्ञाभेद`, `सूचीस्थानाज्ञाभेद`,
        // `रचनाज्ञाभेद`, `रचनाघोषणा`, `क्षेत्रसंख्या`, `खण्डसामर्थ्यम्`,
        // `चिह्नकाङ्कः`, `क्रमः` and the six `अवगणन…` receivers) ·
        // `utsarjana.t1` 6 · `yantrotsarjana.t1` 10. Named so that the next
        // reader can check the SET and not only the difference.
        //
        // THE TWIN THAT PRODUCED THESE FIGURES REPRODUCES THE OLD PIN EXACTLY —
        // (1357, 54, 1946) at `303ff770` — which is what licenses the new one.
        // A count that cannot hit the previous value has not earned the right
        // to replace it.
        // (1358, 54, 1976) -> (1358, 54, 1982) on 2026-09-12, W-293, and this
        // value is the COMPOSITION OF TWO INDEPENDENTLY MEASURED DELTAS, not a
        // third measurement. Two branches moved this triple from the SAME base:
        //
        //   agent/w293-dataloc   +4 `न६४`   four locals in `encode.t1`'s
        //                                  `वस्तुरचना` — दत्तलेखक्रमः the output
        //                                  cursor, खण्डाधारः the datum's base
        //                                  (`दत्तम् ॱ दैर्घ्य` BEFORE its bytes are
        //                                  appended, the only moment it is
        //                                  knowable), निर्देशक्रमः the walk over the
        //                                  recorded addresses, निर्देशसीमा its bound
        //   agent/w293-runfaults +2 `न६४`   two bindings both named
        //                                  `वैश्विकसामर्थ्यम्` — one in `ir.t1`'s
        //                                  global arm, one in `यन्त्रोत्सर्जन`
        //
        // 1976 + 4 + 2 = 1982. THE COMPOSITION IS ONLY VALID BECAUSE NEITHER
        // BRANCH REMOVED A BINDING: both are growth, `अ६४` and `इ६४` are UNMOVED
        // at 1358 and 54 in each, and a respelling would show as a removal in one
        // bucket against an addition in another. Three further declarations in
        // the dataloc edit are arenas, not `न६४`, and the one REMOVED there
        // (`रिक्तदत्तलेखाः`) was an arena too, so they cancel out of this count.
        //
        // **THIS IS A PREDICTION, NOT A READING.** The trunk composed it while
        // resolving the merge; no run has yet taken the triple on the combined
        // tree. The gate is its falsifier. If it answers anything but 1982 the
        // composition assumption is wrong — most likely because the two edits
        // share a binding this arithmetic counted twice.
        // 1982 -> 1983 ON 2026-09-13, `W-nil`: ONE binding, `ir.t1`'s
        // `शून्यावगणना ॱॱ न६४`, the instruction slot the `शून्यम्` arm records.
        // MEASURED FROM THIS ASSERTION'S OWN FAILURE, member-wise: the net diff
        // of `ॱॱ न६४` declarations over `1d6b10a4..2d41aa25` is exactly that
        // name — `अपूर्णहेतुसीमा` also appears in the diff and is an EDITED line
        // (a removal and an addition, net zero), which is the trap this ledger's
        // margins already record. `अ६४` 1358 and `इ६४` 54 unmoved.
        // AND THIS PIN WAS RED ON MAIN FROM `5dd5def0` TO THIS COMMIT, because
        // the landing census was `paradigm_encode` alone and this crate was not
        // run — the same shape as the 163 -> 170 paragraph above, where a lane
        // gating a different crate could not see it. A census that certifies a
        // `.t1` edit does not certify this ledger; only this crate does.
        // 1983 -> 1986 ON 2026-09-13, the gather PORTED TO THE `.t1` DRIVER
        // (`agent/shared-arena-witness`, `e58ec658`): THREE bindings in
        // `shrinkhala.t1` — `घोषणाः`, `सङ्गृहीतम्`, `सञ्चयसिद्धम्`, all `न६४` —
        // the store gather the Rust driver already did and the product's driver
        // did not, caught by `t1_driver`'s twin test minting different ids.
        // MEASURED from this assertion's failure on the trunk with the port
        // merged; the diff over `808580cf..e58ec658` shows FIVE added `चरः`
        // lines, but two are the nil arm's, carried in by that branch's merge of
        // main and already counted in 1983 — the net is three. `अ६४`, `इ६४` unmoved.
        // 1986 -> 1990 ON 2026-09-13, `agent/task08` parts 1–2 (`2a57d72f`): the
        // arena header at allocation and `artha`'s length typing. Net FOUR `न६४`
        // bindings — `अवगणनखण्डाधार`, `अवगणनशीर्षनिधान`, `अवगणनशीर्षशून्य`,
        // `अवगणनशीर्षसरण` — five added lines less `अवगणनखण्ड`, an EDITED line
        // that appears as a removal and an addition. The unit's other six
        // bindings are `मूल्याङ्क`, `चिह्नक` and `अङ्कः …` and do not count here.
        // MEASURED from this assertion's failure on the trunk with task08 merged.
        // 1990 -> 1991 ON 2026-09-13, `W-294` (2d, `agent/w293-runfaults`
        // `8dd8f6be`, narrow element loads): ONE `न६४`, `ir.t1`'s `अष्टकविस्तार`,
        // the element width in octets. 2d measured (1358, 54, 1984) on THEIR base
        // `046f3cfe` — 1982 + their binding — and the trunk's 1990 carried 6e's
        // three, the gather port's three and task08's four on top of the nil
        // arm's one, so neither branch's value was the merged value: MEASURED
        // here from this assertion's failure on the merged tree. Two branches
        // reading the same number for different bindings is why a pin that
        // agrees across a merge is not evidence the merge is a no-op.
        // 1991 -> 1995 ON 2026-09-13, the embed store (trunk, `lex.t1`): FOUR
        // `न६४` — `समावेशसंख्या` (the store's count, a global) and the three loop
        // counters of the collapse, `समावेशक्रमः`, `सारणीक्रमः`, `सरणक्रमः`. The
        // store's length arena is `अङ्कः अन्तः न६४` and is not a `न६४` binding.
        // MEASURED from this assertion's failure. `अ६४`, `इ६४` unmoved.
        // 1995 -> 2008 on 2026-09-13, agent/runhdr: +13 net bindings (न६४ / मूल्याङ्क locals of the run-header arms; two moved lines counted once), MEASURED from the failing assertion,
        // (1358, 54, 2008) -> (1358, 54, 2013) on 2026-09-13, the वास्तु lowering half, store side (two locals in ir.t1's store arm); TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // (1358, 54, 2013) -> (1358, 54, 2014) on 2026-09-13, the वास्तु lowering half (store side): MEASURED from this assertion's own failure.
        // (1358, 54, 2014) -> (1358, 54, 2018) on 2026-09-13, the self-image's entry (शृङ्खला: प्रवेशन्यासः, स्वपरीक्षा, four globals); MEASURED from this assertion's own failure.
        // (1358, 54, 2018) -> (1377, 54, 2028) on 2026-09-13, the encoder's table index (encode.t1 only): +19 `अ६४` — the encodings index's count global and answer, its builder's four locals, the two indexed tests' four parameters and three locals, and the rewritten walkers' index locals (सङ्केताः +3, कुलसङ्केतपङ्क्तिः +1, कुलक्षेत्रयोग्यम् +2); +10 `न६४` — परिधिसाम्यम्'s two bounds and counter, the register index's count global, its builder's answer and two locals, and कोष्ठपङ्क्तिः's three index locals. The seven `अङ्कः अन्तः अ६४` / four `अङ्कः अन्तः न६४` arenas are not bindings of the element type and do not count. MEASURED from this assertion's own failure.
        // (1377, 54, 2028) -> (1377, 54, 2031) on 2026-09-13, मण्डलानिप्रतिबिम्बम् collects every source before compiling any; MEASURED from this assertion's own failure.
        (1377, 54, 2031),
        "the ADR-0030 remediation ledger moved. Expected 1078 `अ६४` still to \
    // AND FROM THE OTHER LANE, whose bindings this tree also carries:
        // (1316, 54, 1528) -> (1322, 54, 1654) ON 2026-09-04, `W-245`, MEASURED on the
        // MERGED tree from this assertion's own failure: corpus GROWTH, 132 bindings net,
        // NONE respelled. (1316, 54, 1528) on main and (1322, 54, 1624) on `agent/w245`
        // are SIBLINGS off (1316, 54, 1498) — main's twenty-six W-223-part-2 न६४ plus its
        // four, and this row's — so the sum was the prediction and the assertion's failure
        // was the value. `ir.t1` +68 `न६४` (the eleven instruction kinds गुणनाज्ञाभेद …
        // निधानाज्ञाभेद, the six तुलना sub-kinds, आज्ञा's उपभेद and स्थानक्रम, the locals
        // table's स्थानीयसंख्यान, the two census bounds, the three appender wrappers' and
        // four helpers' answers, and the builder arms' locals — तुलनाभेदः, नामभेदः,
        // स्थानीयस्थलम्, चरस्थानम्, समस्थलम्, लक्ष्यभेदः, समहेतुः, प्राचलसंख्या, प्राचलस्थानम्
        // and the `अवगणन…` receivers; 2 removed with the old catch-all) and +4 `अ६४`
        // (`प्राचलक्रमः`, the declaration's own spelling; `बूलमूल्यम्`; `अङ्कारम्भः` and
        // `मानम्`, the numeral reader's — SIGNED by design, a `ऋण` numeral is negative).
        // `yantrotsarjana.t1` +52 `न६४` net and +2 `अ६४` net (the fusion's `संयोज्यम्`
        // threaded through three routines, the two fusion walks, the compare, load and
        // store lowerers' registers, the wrap-to-a-register split's `द्विषष्टिः`, the forms
        // fixture's block starts). `utsarjana.t1` +6 `न६४` (the allocator's use arm for
        // the new kinds). `इ६४` 54 — this row converted none.
         examine (1322 filed, less `ir.t1`'s 56, `ast.t1`'s 47, \
         `vishlesana.t1`'s 39, `artha.t1`'s 34, `lex.t1`'s 74, `parse.t1`'s \
         74, `samyojana.t1`'s 88, `nidana.t1`'s 112, `utsarjana.t1`'s 113, \
         `sanskrit_text.t1`'s 16, `vastu.t1`'s 3, `kosha.t1`'s 1 and \
         `encode.t1`'s 16 + 36 + 19 + 25 + 19 + 7 + 13 + 7 + 6 + 14 + 6 + 3 and \
         `vakyavibhaga.t1`'s 14, plus the 4 `इ६४` `ir.t1` \
         converted, \
         the 2 `vishlesana.t1` did, the 30 `samyojana.t1` did, the 4 \
         `utsarjana.t1` did, the 1 `vastu.t1` did and the 15 `encode.t1` \
         did, and the 4 `अ६४` W-245 added), 0 `इ६४` left and 968 `न६४` written. If a file was \
         remediated, update this triple and name the file in the comment \
         above it"
    );

    // AND THE INVARIANT THAT MAKES THE LEDGER WORTH KEEPING: the remediation
    // MOVES sites between spellings and may not lose them. 1378 is the number
    // of 64-bit integer type positions in the corpus, and it is the same number
    // before the repair (1322 + 56 + 0) and after every step of it. A row that
    // deleted a site rather than respelling it would pass the triple above only
    // by being updated to match itself; this fails instead.
    pin_report!(
        a64 + i64_sites + n64,
        // 1378 BEFORE THE MERGE OF 2026-09-01, AND THIS IS THE "SAY IT OUT
        // LOUD" THE MESSAGE BELOW ASKS FOR. The corpus GAINED typed bindings:
        // `land-varna` implemented routines `origin/main` had as
        // `उक्तम् अपूर्णम्` — ~500 lines in `artha.t1` alone, plus
        // `vishlesana.t1`'s विश्लेषणम् — and every local and signature in them
        // is a new 64-bit type position. That is corpus growth, NOT a
        // remediation moving sites between spellings, and the invariant is
        // doing exactly its job by refusing to let it pass silently.
        //
        // 321 -> 323 on 2026-09-02, W-179 / ADR-0037 (`2894899b`): two `न६४`
        // bindings ADDED — the operator kinds `शेषद्विकर्मभेद` (१४) and
        // `बृहत्समद्विकर्मभेद` (१५) in `ast.t1` — sum 2242 -> 2244, none moved.
        // ANNOTATION RETROACTIVE, written 2026-09-04 by W-218: that commit
        // changed the triple (1852, 69, 321) -> (1852, 69, 323) and the pinned
        // sum 2242 -> 2244 with neither number written in this note or in its
        // message, the ONE unannotated sum change in the ledger's history
        // (W-213's census, 45 of 46). The numbers are as that commit landed
        // them; this line only says out loud what it should have said then.
        //
        // 2244 -> 2246 ON 2026-09-02, SAID OUT LOUD AS THIS MESSAGE DEMANDS.
        // Two typed bindings were added, and NOT by a sweep: the type
        // checker's `प्रकारदोषशरीरभेद` and `प्रकारदोषप्रत्यागमनभेद`, both
        // `न६४`, so that a type-check refusal can name WHICH TWO TYPES
        // disagreed instead of answering a bare `असत्यम्`. The `measure_
        // corpus_typecheck` census printed six identical `Bool(false)`s before
        // they existed and six named causes after.
        //
        // THE MERGE EARLIER THE SAME DAY MOVED 396 SITES AND DID NOT MOVE THIS
        // NUMBER, which is the distinction worth keeping: a remediation that
        // changed the sum would be losing sites, and an addition that did not
        // change it would mean the new bindings were not being counted.
        // 2246 -> 2247, said out loud: ONE typed binding added,
        // `अनिर्णीतपङ्क्ति`. The refusal record named the WORD and not the
        // PLACE, and `sanskrit_text.t1` writes that word eighteen times — two
        // hypotheses were formed against the wrong occurrence before the line
        // number existed. A diagnostic that cannot be located is barely a
        // diagnostic.
        // 2247 -> 2249: see the note on the triple above. This is corpus
        // GROWTH — the parser gained the feature its own EBNF already had —
        // and not a remediation moving sites between spellings.
        // 2249 -> 2254: five bindings added by `गणनापठनम्`; see above. Corpus
        // growth, not a remediation moving sites between spellings.
        // 2254 -> 2255: one 64-bit binding added; see the triple above.
        // 2255 -> 2260: five न६४ bindings added; see the triple above.
        // 2260 -> 2262, said out loud: TWO `न६४` bindings added by `ir.t1`'s
        // `द्विकर्म` arm — `आज्ञाभेदः` and `अवगणनत्रि`. Corpus GROWTH: the IR
        // builder gained an arm, so `३ योगः ४` lowers to a योगाज्ञा instead of
        // to the constant-० stub every unported expression kind still gets.
        // Not a remediation moving sites between spellings.
        //
        // 2262 -> 2264: two more न६४ bindings from blocker (d)'s new
        // `अर्थप्रकारकोश` arena in `artha.t1`; see the triple above.
        // 2264 -> 2284 ON 2026-09-03, said out loud: TWENTY `न६४` bindings
        // added by `W-198` — the IR's conditional terminator, its block-in-hand
        // state, the यदि and यावत् arms, and the `अन्यथा` slot the AST gained so
        // the parser could stop dropping else bodies. Named one by one at the
        // triple above. Corpus growth, not a remediation moving sites.
        //
        // 2264 -> 2272 on 2026-09-03, W-183: EIGHT `न६४` bindings ADDED (the declared-type map —
        // two record fields, one arena, one parameter, four locals as the census sees them), none
        // moved. Conservation says a sweep may not lose a site, not that the corpus may never
        // gain one; a sum that held here would mean eight sites had gone missing.
        //
        // W-202, 2026-09-04: the two notes above are SIBLINGS, not a sequence —
        // both start from 2264 and neither saw the other's sites, so 2284 and
        // 2272 may not be added or chained. W-202 adds its own on top of both.
        // The total below is what the assertion printed on the merged tree; see
        // the triple above for why it is measured rather than summed.
        //
        // 2303 = 1298 + 54 + 951, MEASURED. Naive arithmetic on the two notes
        // above would have said 2264 + 20 + 8 = 2292 before W-202's own
        // eleven; it is 2303 because the merge is not the sum of its branches
        // and this ledger is the thing that catches that.
        // sum 2292 MEASURED 2026-09-04: 1298 + 54 + 940; W-183 (+8) and W-198 (+20) both ADDED bindings, none moved.
        // 2292 -> 2313 on 2026-09-03, said out loud: TWENTY-ONE `न६४` bindings
        // added by `W-204` — the IR's call arm, its argument arena and scratch
        // stack, its Call constructor and its refusal record. Named one by one
        // at the triple above. Corpus growth, not a remediation moving sites.
        // Measured on the merged tree, not summed — see the triple above.
        // 2324 = 1298 + 54 + 972. The arithmetic HAPPENS to close here because
        // W-202 and W-204 each only ADDED bindings off the shared 940 base and
        // neither moved a site — which is exactly the case where computing
        // looks safe and is indistinguishable from measuring. It was measured.
        // 2313 -> 2326 on 2026-09-04, W-226/W-228: thirteen `न६४` bindings ADDED (the
        // slice kind, the qualifier fold's helper, the slice-limit reader, the two
        // chains' new positions, the missing-`ददाति` refusal's two locals), none
        // moved; see the triple above. Corpus growth, said out loud.
        // 2326 -> 2327, the W-228 amendment's `घोष्यमाणभेदः`; see the triple above.
        // 2338 = 1298 + 54 + 986, MEASURED on the merge of W-202 with W-204 and
        // W-226. Three sibling rows off the shared 940 base — W-202's eleven,
        // W-204's twenty-one, W-226's fourteen — none of which counted the
        // others', so no pair of the pinned sums may be added or chained.
        // 2341 = 1298 + 54 + 989, measured. See the triple above.
        // MERGE of W-202 (left) and W-215 (right) on 2026-09-04 by the trunk: the value below is
        // MEASURED on the merged tree from this assertion's own failure, never summed.
        // 2327 -> MEASURED (see below) on 2026-09-04, said out loud: ONE HUNDRED AND SIXTY-NINE
        // `न६४` bindings added by `W-215` — a new source file, the unparser, and
        // nine in the parser it inverts. Named at the triple above by file and
        // by kind. Corpus growth, not a remediation moving sites.
        // 2327 -> 2500 on 2026-09-04, said out loud: 173 `न६४` bindings added by
        // `W-215` on the merged tree (169 + 4, named at the triple above).
        // Corpus growth, not a remediation moving sites.
        // 2500 -> 2511 on 2026-09-04, said out loud: the merge of W-202 (+11) with W-215 (+173)
        // on W-226's 2327; 1298 + 54 + 1159 = 2511, MEASURED.
        // MERGE of main (left: W-202 + W-215) with agent/w227 (right: W-239 part 1's arena) on 2026-09-04
        // by the trunk: the value below is MEASURED on the merged tree from this assertion's own
        // failure, never summed.
        // 2327 -> 2349 on 2026-09-04, W-239: twenty-two `न६४` bindings ADDED by the
        // octet arena, a new file; named one by one at the triple above. Corpus
        // growth, said out loud.
        // -> 2533 on 2026-09-04, said out loud: 1298 + 54 + 1181, the merge of main with W-239's
        // part 1 (+22), MEASURED.
        // MERGE of main (left: through W-227 + W-239 part 1) with agent/d002a2 (right: encode.t1's
        // सङ्कोचः) on 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this
        // assertion's own failure, never summed.
        // 2327 -> 2358 on 2026-09-04, `D-002a2`, said out loud: THIRTY-ONE typed
        // bindings ADDED — thirty `न६४` and one signed `अ६४` — by `encode.t1`'s
        // `सङ्कोचः` body, its three helpers and its caller chain, none moved.
        // Named one by one at the triple above. Corpus growth: the encoder
        // gained its last routine, not a remediation moving sites.
        // -> 2564 on 2026-09-04, said out loud: 1299 + 54 + 1211, the merge of main with D-002a2
        // (+30 न६४, +1 अ६४), MEASURED.
        // 2575 = 1299 + 54 + 1222, measured on this merged tree.
        // MERGE of main (left: through D-002a2) with agent/w236 (right: the T1 emitter twin) on 2026-09-04
        // by the trunk: the value below is MEASURED on the merged tree from this assertion's own failure,
        // never summed.
        // 2313 -> 2532 on 2026-09-04, said out loud: TWO HUNDRED AND NINETEEN
        // 64-bit bindings added by `W-236` — 210 in the new `yantrotsarjana.t1`
        // (the T1 twin of the RISC-V emitter), 3 in `utsarjana.t1`'s allocator,
        // 6 in `vishlesana.t1`'s bias step — every one named at the triple
        // above. Corpus growth, not a remediation moving sites; `इ६४` stays 54.
        // MERGED 2026-09-04: 2327 (W-226/W-228) + 219 (W-236) = 2546, the sum of the
        // triple the test printed on the merged tree.
        // -> 2783 on 2026-09-04, said out loud: 1336 + 54 + 1393, the merge of main with W-236 (the
        // T1 emitter twin, +219 named), MEASURED.
        // MERGE of main (left: through W-236) with agent/w239 (right: the octet arena wired) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // 2564 -> 2588 on 2026-09-04, W-239 part 2 merged onto main after D-002a2: fourteen `न६४` and ten net
        // `अ६४` bindings ADDED by wiring `vakyavibhaga.t1`'s directive arms to the
        // arena; named at the triple above. Corpus growth, said out loud.
        // -> 2807 on 2026-09-04, said out loud: 1346 + 54 + 1407, the merge of main with W-239 (the
        // octet arena wired), MEASURED.
        // 2807 -> 2767 on 2026-09-04, `W-237`, said out loud: 1316 + 54 + 1397. The one time this
        // sum FELL — 42 sites DELETED with the retired T0 emitter pair in `utsarjana.t1` (30 `अ६४`
        // + 12 `न६४`, four routines), less 2 `न६४` added in `ir.t1`; see the triple's note. A
        // deletion of routines is the one thing conservation must let through, and it is said
        // here rather than matched to itself.
        // 2767 -> 2775 on 2026-09-04, said out loud: EIGHT `न६४` bindings added by
        // `W-240`'s lane addition, named at the triple above. Corpus growth.
        // MERGE of main (left: through W-240) with agent/w243 (right: the inter-module image) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // 2767 -> 2768 on 2026-09-04, `W-243`: one `न६४` ADDED by the T1 emitter's
        // startup-object routine; see the triple's note. Corpus growth, said out loud.
        // -> 2776 on 2026-09-04, said out loud: 1316 + 54 + 1406, the merge of main with W-243 (+1),
        // MEASURED.
        // MERGE of main (left: through W-243) with agent/w231 (right: the checker's four arms) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // 2794 = 1336 + 54 + 1404, measured on the merge with main at 927ca2b6.
        // -> 2787 on 2026-09-04, said out loud: 1316 + 54 + 1417, the merge of main with W-231 (+11),
        // MEASURED.
        // -> 2788 on 2026-09-04, said out loud: the respelled instrument location, MEASURED.
        // MERGE of main (left: through W-244) with agent/w223 (right: the declaration store) on
        // 2026-09-04 by the trunk: the value below is MEASURED on the merged tree from this assertion's
        // own failure, never summed.
        // 2776 -> 2856 on 2026-09-04, `W-223` part 1: eighty `न६४` bindings ADDED by
        // the declaration store, a new file; named by group at the triple above.
        // Corpus growth, said out loud.
        // -> 2868 on 2026-09-04, said out loud: 1316 + 54 + 1498, the merge of main with W-223 part 1
        // (the declaration store, +80), MEASURED.
        // MERGE of main (left) with agent/w223b (right: the extent) on 2026-09-04 by the trunk:
        // the value below is MEASURED on the merged tree from this assertion's own failure.
        // -> 2868 on 2026-09-04, said out loud: 1316 + 54 + 1498, the merge of
        // main with W-223 part 1 (the declaration store's eighty, a new file),
        // MEASURED.
        // -> 2872 on 2026-09-04, said out loud: 1316 + 54 + 1502, W-223 part 2's four
        // extent-refusal न६४ in artha.t1, MEASURED.
        // -> 2898 on 2026-09-04, said out loud: 1316 + 54 + 1528, W-223 part 2's
        // twenty-six न६४ for the declaration store's resolver side, MEASURED.
        // 2898 -> 2901 on 2026-09-04, `W-247`: one `न६४` REMOVED with `encode.t1`'s
        // duplicate `प्रतीक्षा`, three `न६४` and one `अ६४` ADDED by the resolver's
        // redeclaration refusal; named at the triple above. Said out loud.
        // MERGE of main with agent/driver (the numeral reader) on 2026-09-05 by the trunk:
        // MEASURED on the merged tree from this assertion's own failure, never summed.
        // 2898 -> 2900 on 2026-09-05, `W-256`: the numeral reader's accumulator and
        // its digit; named at the triple above. Corpus growth, said out loud.
        // -> 2903, MEASURED.
        // 2903 -> 2901 on 2026-09-05: `W-257` reverted; see the triple's note.
        // 2901 -> 2908 on 2026-09-05, `W-259`: seven `न६४` by the parameter path;
        // named at the triple above. Corpus growth, said out loud.
        // MERGED 2026-09-05: 2908 here and 3030 on the other lane; the merged tree
        // measures 3033 = 1323 + 54 + 1656, taken from this assertion's own
        // failure and never summed. See the triple above for what each side
        // contributed.
        // 3033 -> 3038 on 2026-09-05, `W-248`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE. The invariant holds in the direction it is
        // for: this row LOST no site and MOVED none between spellings — it
        // added five 64-bit positions (one `अ६४`, four `न६४`) and respelled
        // nothing, so the sum rises by exactly what the triple rose by.
        // 3038 -> 3089 on 2026-09-05, `W-265`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE. Same shape and same direction: 51 positions
        // ADDED (two `अ६४`, 49 `न६४`), none lost and none respelled, so the sum
        // rises by exactly what the triple rose by — 1326 + 54 + 1709 = 3089.
        // That equality is the check worth making here: a row that respelled a
        // site would move the triple and leave this number standing.
        // 3139 -> 3141 LATER THE SAME DAY, `W-278`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE. 2 positions ADDED (both `न६४`), none lost
        // and none respelled: 1328 + 54 + 1759 = 3141, agreeing with the
        // triple's own failure at (1328, 54, 1759).
        //
        // THE SAME ROW RE-TAKING THE SAME PIN TWICE IS THE FINDING HERE. The
        // 3139 below was measured while the row was still editing `.t1`, so it
        // pinned a tree the row then left behind. A pin re-taken mid-row
        // measures the tree it ran on, not the row — so it belongs LAST, after
        // the last source edit, or it must be taken again. That is why this
        // entry exists, and it cost a gate cycle to learn.
        //
        // 3131 -> 3139 on 2026-09-06, `W-278`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE. 8 positions ADDED (one `अ६४`, seven `न६४`),
        // none lost and none respelled, so the sum rises by exactly what the
        // triple rose by: 1328 + 54 + 1757 = 3139. THE TWO READINGS AGREEING is
        // the check, and they do — this assertion's 3139 and the triple's own
        // failure at (1328, 54, 1757). A sum that moved by a different amount
        // would mean a site was RESPELLED rather than added, which is the one
        // thing this pair of pins exists to tell apart.
        //
        // 3089 -> 3131 on 2026-09-06, `W-254`, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE. Same shape and same direction: 42 positions
        // ADDED (one `अ६४`, 41 `न६४`), none lost and none respelled, so the sum
        // rises by exactly what the triple rose by. The two readings agreeing —
        // this assertion's 3131 and the triple's own failure at
        // (1327, 54, 1750) — is the check; a sum that moved by a different
        // amount would mean a site was RESPELLED rather than added, which is
        // the one thing this pair of pins exists to tell apart.
        // 3131 -> 3193 on 2026-09-06, `W-kosha` phase 2, RE-MEASURED FROM THIS
        // ASSERTION'S OWN FAILURE. 62 positions ADDED (28 `अ६४`, 34 `न६४`),
        // none lost and none respelled, so the sum rises by exactly what the
        // triple rose by — and the two readings agreeing is the check that a
        // site was ADDED rather than RESPELLED.
        // 3193 -> 3194 on 2026-09-06, same row: one `अ६४` lost, two `न६४`
        // gained, so the sum rises by one. RE-MEASURED FROM THIS ASSERTION'S
        // OWN FAILURE — and it agrees with the triple, which is the check that
        // a site was exchanged rather than merely respelled.
        // 3194 -> 3204 on 2026-09-06, the merge of agent/kosha and
        // agent/nameglobal, RE-MEASURED FROM THIS ASSERTION'S OWN FAILURE.
        // 1355 + 54 + 1795 = 3204: the sum rises by exactly what the triple
        // rose by, which is the check that sites were ADDED and none
        // respelled. Two instruments over one corpus, agreeing — and that
        // agreement is the only reason either figure is worth reporting,
        // because a re-taken number carries no evidence on its own.
        // 3204 -> 3205 on 2026-09-06, the three-branch integration, TAKEN
        // FROM THIS ASSERTION'S OWN FAILURE and PREDICTED before the run:
        // 1356 + 54 + 1795 = 3205. Two instruments over one corpus, agreeing,
        // and both agreeing with a figure written down in advance.
        // 3205 -> 3207 on 2026-09-07, THE BAD-FIELD REFUSAL, MEASURED FROM
        // THIS ASSERTION'S OWN FAILURE. The corpus GAINED two 64-bit type
        // positions rather than moving any between spellings, which is what
        // this invariant exists to tell apart: 1356 + 54 + 1797 = 3207 and the
        // triple above moved only its `न६४` element, by the same 2. A
        // respelling would have left this number alone; a lost site would have
        // lowered it.
        // 3207 -> 3223 on 2026-09-07, the driver merge, TAKEN FROM THIS
        // ASSERTION'S OWN FAILURE. 1356 + 54 + 1813 = 3223: the sum rises by
        // exactly what the triple rose by, so sites were ADDED and none
        // respelled. Two instruments over one corpus, agreeing.
        // 3223 -> 3235, TAKEN FROM THIS ASSERTION'S OWN FAILURE.
        // 1356 + 54 + 1825 = 3235: the sum rises by exactly what the triple
        // rose by, so sites were ADDED and none respelled.
        // 3235 -> 3263: +28 `न६४`, none lost and none respelled. Predicted
        // 3264 for the reason above — one `ददाति न६४` went out with the
        // widened signature it was on.
        // 3263 -> 3281 on the merge with agent/index and agent/storeprobe, and
        // 1356 + 54 + 1871 = 3281 — the sum follows the triple exactly, so
        // nothing was lost or respelled by three lanes' growth arriving at once.
        // 3281 -> 3287: +1 `अ६४` and +5 `न६४`, and 1357 + 54 + 1876 = 3287.
        // The sum follows the triple, so nothing was respelled — the `अ६४` is
        // a NEW site following its callee, not an `न६४` moving backwards.
        //
        // 3287 -> 3294: +7 `न६४` and nothing else, and 1357 + 54 + 1883 = 3294.
        // THE SUM FOLLOWING THE TRIPLE IS THE WHOLE VALUE OF THIS PIN — it is
        // the one check that separates corpus GROWTH from a RESPELLING, and the
        // two are indistinguishable in either number alone. Seven new bindings
        // and no width moved.
        //
        // 3294 -> 3299 on 2026-09-09, THE SYMBOL-MAPPING ROW: +5 `न६४` and
        // nothing else, and 1357 + 54 + 1888 = 3299. THE SUM FOLLOWS THE TRIPLE,
        // so this row GREW the corpus and respelled nothing.
        //
        // That is worth checking rather than assuming here, because this row DID
        // remove three `न६४` sites — the duplicate lookup `संज्ञान्वेषणम्`,
        // replaced by a call to the existing `अर्थॱनामनिर्णयः`. A removal and a
        // respelling look identical in the triple: both can leave `न६४` lower
        // than the additions alone would put it. They differ HERE, in the sum —
        // a respelling holds it fixed while a removal lowers it, and it rose by
        // exactly the net.
        // 3299 -> 3312 on 2026-09-09, THE `batch-1` PIN RE-TAKE, TAKEN FROM
        // THIS ASSERTION'S OWN FAILURE. SAYING IT OUT LOUD, WHICH IS WHAT THIS
        // ASSERTION EXISTS TO FORCE: **the corpus GAINED 13 typed bindings.
        // Nothing was respelled and nothing was removed.** This is growth, not
        // a remediation step, and the sum is the only place the difference
        // shows — a respelling holds it fixed, a removal lowers it, and it rose
        // by exactly the net of the triple's own +13.
        //
        // THE TRIPLE AND THIS SUM MOVED BY THE SAME 13, WHICH IS THE CHECK: had
        // I re-taken the triple against a tree that also dropped a site, the
        // triple would have accepted the new number and this sum would not.
        // The two agreeing is what distinguishes a re-pin from a pin updated to
        // match itself, which is the failure the paragraph above describes.
        //
        // 12 are `agent/xmodule`'s cross-module external-name arenas, their
        // walk and its reader's return; 1 is `agent/lexrung`'s
        // `सङ्कलनविफलसंख्या`. Enumerated at the triple above, member-wise by
        // `comm` against `4812ac9c`, per branch.
        // 3312 -> 3320 on 2026-09-09, W-280. **PREDICTED BEFORE THE RUN and
        // exact**: +8 and nothing else, because the triple moved +8 `न६४` with
        // `अ६४` and `इ६४` unmoved. THE SUM FOLLOWING THE TRIPLE IS THE WHOLE
        // VALUE OF THIS PIN — it separates corpus GROWTH from a RESPELLING, and
        // eight new declarations that respell nothing is growth.
        // 3320 -> 3327 on 2026-09-10, W-279. **PREDICTED AS +7 AND EXACT**, for
        // the same reason: the triple moved +7 `न६४` with `अ६४` and `इ६४`
        // unmoved, so the sum follows it. Seven net new declarations that
        // respell nothing is GROWTH, which is the whole value of this pin
        // sitting behind the triple.
        //
        // AND IT WAS INVISIBLE UNTIL THE TRIPLE WENT GREEN. Both pins were wrong
        // by the same +7 in one run, and the gate could only name the first —
        // a red is a LOWER BOUND on what is broken. The second was not a new
        // regression; it was the same one, waiting behind a failure.
        // 3327 -> 3331 on 2026-09-10, W-279, TAKEN FROM THIS ASSERTION'S OWN
        // FAILURE. SAYING IT OUT LOUD, as this assertion exists to force: the
        // corpus GAINED four typed bindings and respelled nothing. It moved by
        // the same +4 as the triple above, which is what says growth rather
        // than a respelling — a respelling holds this sum fixed and a removal
        // lowers it.
        //
        // AND IT FIRED ONLY AFTER THE TRIPLE WAS FIXED, because both live in
        // one test and the triple asserts first. The gate reported ONE failure;
        // there were two. A red is a LOWER BOUND on what is wrong.
        //
        // 3331 -> 3344 on 2026-09-11, `W-283`'s ruled storage model, TAKEN FROM
        // THIS ASSERTION'S OWN FAILURE — and it fired second again, from one
        // `cargo test` reporting one failure both times. The sequence is now a
        // property of this file and not an accident: **re-taking the triple is
        // what makes this pin reachable**, so a row that moves typed bindings
        // should expect exactly two reds and stop only after the second.
        //
        // SAYING IT OUT LOUD, as this assertion exists to force: the corpus
        // GAINED thirteen typed bindings and respelled nothing. It moved by the
        // same +13 as the triple, which is what says growth — a respelling
        // holds this sum fixed and a removal lowers it.
        //
        // THE SUM IS NOT INDEPENDENT EVIDENCE AND SHOULD NOT BE READ AS SUCH:
        // 1357 + 54 + 1933 = 3344 exactly, so this pin is derivable from the
        // triple above. What it tests is the CLASSIFICATION — that the thirteen
        // are `न६४` and not a respelling of `अ६४` or `इ६४`, which the triple's
        // two unmoved columns say and this sum confirms from the other side.
        //
        // 3344 -> 3357 on 2026-09-11, `W-283`, TAKEN FROM THIS ASSERTION'S OWN
        // FAILURE — and it fired SECOND again, from one `cargo test` reporting
        // one failure each time. That is now three rows in a row: re-taking the
        // triple is what makes this pin reachable, so a row moving typed
        // bindings should expect exactly two reds and stop only after the
        // second.
        //
        // The corpus GAINED thirteen typed bindings and respelled nothing, the
        // same +13 as the triple. And as before this is NOT independent
        // evidence: 1357 + 54 + 1946 = 3357 exactly. What it tests is the
        // CLASSIFICATION — that the thirteen are `न६४` and not a respelling of
        // `अ६४` or `इ६४` — which the triple's two unmoved columns say from the
        // other side.
        //
        // 3357 -> 3388 ON 2026-09-12, THE STORAGE MODEL'S THREE LANDINGS, AND
        // IT FIRED SECOND FOR THE FOURTH ROW RUNNING — the margin above
        // predicted exactly that, and the prediction held. **SAID OUT LOUD, AS
        // THE MESSAGE BELOW DEMANDS:** the corpus GAINED 31 typed bindings and
        // respelled NOTHING — 30 `न६४` and 1 `अ६४` added, ZERO removed in any
        // of the three buckets. Corpus growth, not a remediation moving sites
        // between spellings, and the invariant is doing its job.
        //
        // NOT INDEPENDENT EVIDENCE, as ever: 1358 + 54 + 1976 = 3388 exactly.
        // What it tests is the CLASSIFICATION — that the 31 are 30 `न६४` and
        // one `अ६४` rather than a respelling — which the triple's unmoved
        // `इ६४` column says from the other side.
        //
        // WHY THREE LANDINGS PASSED THEIR OWN GATES AND LEFT THIS RED, which is
        // the durable finding and not the number: **a `.t1` edit is a CORPUS
        // edit, and the corpus is an input to assertions in crates the edit
        // never touches.** These pins count bindings across
        // `crates/sadhana-t1/src/*.t1` from inside `sanskrit-text`, so a gate
        // scoped from CHANGED FILES cannot reach them by construction. `W-284`,
        // `W-285` and `W-287` each went green and each moved this sum; main was
        // red from `f444b6dd` until a lane gating an unrelated crate read it.
        // 3388 -> 3394 on 2026-09-12, W-293. The same composition as the triple
        // above and for the same reason: +4 from `agent/w293-dataloc` and +2 from
        // `agent/w293-runfaults`, both measured from 3388, neither removing a
        // binding. The sum is the triple's total, so it cannot move independently
        // of it — a tree that reads (1358, 54, 1982) above and anything but 3394
        // here has an inconsistency the pins are built to catch.
        //
        // AND THE PRIOR NOTE ON THIS PIN STANDS, from the dataloc branch: both
        // this and the triple were wrong by the same delta in one run and the
        // gate could name only the first. A RED IS A LOWER BOUND on what is
        // broken, never the set — the second was not a new regression, it was the
        // same one waiting behind a failure.
        // 3394 -> 3395 on 2026-09-13, `W-nil`: the triple's third column moved by
        // one (`शून्यावगणना`), so this sum moves by one IN THE SAME EDIT — the
        // lesson a previous lane paid for when moving the triple without its sum
        // cost a second red.
        // 3395 -> 3398 on 2026-09-13, the gather port: the triple's third column
        // moved by three, so this sum moves by three in the same edit.
        // 3398 -> 3402 on 2026-09-13, task08 parts 1–2: the triple's third column
        // moved by four, so this sum moves by four in the same edit.
        // 3402 -> 3403 on 2026-09-13, W-294: the triple's third column moved by
        // one, so this sum moves by one in the same edit.
        // 3403 -> 3407 on 2026-09-13, the embed store: the triple's third column
        // moved by four, so this sum moves by four in the same edit.
        // 3407 -> 3420 on 2026-09-13, agent/runhdr: the ledger's +13 carried into the sum, MEASURED from this assertion's own failure.
        // 3420 -> 3426 on 2026-09-13, the वास्तु lowering half (store side): MEASURED from this assertion's own failure.
        // 3426 -> 3430 on 2026-09-13, the self-image's entry (शृङ्खला: प्रवेशन्यासः, स्वपरीक्षा, four globals); MEASURED from this assertion's own failure.
        // 3430 -> 3459 on 2026-09-13, the encoder's table index: +29 typed 64-bit positions, the triple's own +19 `अ६४` and +10 `न६४` seen once more as a sum; nothing respelled, nothing lost. MEASURED from this assertion's own failure.
        // 3459 -> 3462 on 2026-09-13, मण्डलानिप्रतिबिम्बम् collects every source before compiling any; MEASURED from this assertion's own failure.
        3462,
        // AND FROM THE OTHER LANE, whose bindings this tree also carries:
        // 2898 -> 3030 on 2026-09-04, `W-245`, said out loud: 1322 + 54 + 1654. Corpus
        // growth — the IR forms that make a program's status more than zero (eleven
        // instruction kinds, the locals table, the fusion of the six ADR-0008 conditions
        // in both emitter twins), every binding named at the triple above. MEASURED from
        // this assertion's own failure.
        "the corpus's 64-bit integer type positions are no longer 1378. The \
         ADR-0030 remediation respells sites and does not add or remove them, \
         so this changes only when the corpus itself gains or loses a typed \
         binding — which is not a remediation and needs saying out loud"
    );
}

#[test]
fn a_negative_sentinel_is_what_keeps_a_site_signed() {
    // ADR-0030, task `D-002h`. The ledger above counts the remediation; it
    // cannot say WHY a site was left signed, and "the twin says so" does not
    // answer for `nidana.t1` — `crates/sadhana/src/nidana.rs` declares no
    // numeric type in its 465 lines and `crates/sanskrit-text/src/segment.rs`
    // declares no signed one. The whole reason those three sites are still
    // `अ६४` is that `पदार्थाङ्कः` spells "no placeholder here" as `ऋण१`, so
    // this test pins the sentinel and the sites TOGETHER: delete the `ऋण१`
    // and the sites lose their justification; respell the sites and the
    // sentinel has nowhere to go.
    let text = std::fs::read_to_string(root().join("crates/sadhana-t1/src/nidana.t1"))
        .expect("nidana.t1 exists");
    let code: String = text
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        code.contains("प्रत्यागमनम् ऋण१"),
        "`nidana.t1` no longer returns `ऋण१`. It is the only negative in the \
         file and the only reason three of its sites are still spelled `अ६४`; \
         if the sentinel went, so should they"
    );

    // Exactly three, and each one named. A count alone would pass if a fourth
    // site were left signed somewhere else and one of these were respelled.
    let signed: Vec<&str> = code
        .lines()
        .filter(|l| l.contains("अ६४"))
        .map(str::trim)
        .collect();
    // THE THREE NAMED SITES ARE STILL ASSERTED BY NAME — that claim did not
    // weaken. What changed is that the file is no longer ONLY those three.
    for named in ["वृत्तिः पदार्थाङ्कः", "वृत्तिः पदार्थलेखनम्", "चरः अङ्कम्"]
    {
        assert!(
            signed.iter().any(|l| l.contains(named)),
            "`{named}` is no longer a signed site of `nidana.t1`. It is one of \
             the three the `ऋण१` sentinel keeps signed, so this is a \
             respelling that lost the sentinel's reason. Found: {signed:?}"
        );
    }

    // MOVED BY THE MERGE OF 2026-09-02, 3 -> 13, AND THE NUMBER IS THE DEBT.
    // `origin/main` remediated `nidana.t1` to `न६४` while this branch EXTENDED
    // it. The merge carried their sweep across BY NAME — 396 sites over six
    // files, conservation sum 2244 unchanged — and this branch's own additions
    // came with no guidance from it.
    //
    // TEN OF THE EXTRA SITES ARE `अ६४` CORRECTLY, and that is why this is not
    // a loosened floor. They are the bounds `शीर्षपङ्क्तिः` walks — `आरम्भः`,
    // `कूटादिः`/`कूटसीमा` and their three siblings — every one an argument to
    // or a result of `encode.t1`'s `क्षेत्रारम्भः`/`क्षेत्रसीमा`, which
    // NEITHER side remediated and which still take and return `अ६४`.
    // Respelling them here would be a type error, not progress. They become
    // `न६४` on the day `encode.t1` is remediated and not before, which is
    // `W-180`'s row and is ORDERED there: the signatures first, then these.
    assert_eq!(
        signed.len(),
        13,
        "`nidana.t1`'s signed sites moved. Three are the `ऋण१` sentinel's and \
         ten are bounds that must stay `अ६४` until `encode.t1`'s \
         `क्षेत्रारम्भः`/`क्षेत्रसीमा` are remediated. ABOVE 13 is a new \
         un-remediated site; BELOW 13 means a bound was respelled ahead of the \
         signature it answers to. Found: {signed:?}"
    );
    // EVERY signed site belongs to one of the two groups and neither is a
    // catch-all. This replaced a `zip` against a three-name list, which read
    // the FIRST three signed lines — fine when there were only three, and
    // silently the wrong three once the bounds joined them ahead of the
    // cluster in file order.
    const SENTINEL_CLUSTER: &[&str] = &["पदार्थाङ्कः", "पदार्थलेखनम्", "अङ्कम्"];
    const ENCODE_BOUNDS: &[&str] = &[
        "आरम्भ",
        "अवसान",
        "कूटादिः",
        "कूटसीमा",
        "पदादिः",
        "पदसीमा",
        "संस्कृतादिः",
        "संस्कृतसीमा",
        "आङ्ग्लादिः",
        "आङ्ग्लसीमा",
    ];
    for line in &signed {
        assert!(
            SENTINEL_CLUSTER.iter().any(|o| line.contains(o))
                || ENCODE_BOUNDS.iter().any(|o| line.contains(o)),
            "a site in `nidana.t1` is spelled `अ६४` and belongs to NEITHER the \
             `ऋण१` sentinel cluster nor the bounds that answer to `encode.t1`'s \
             `क्षेत्रारम्भः`/`क्षेत्रसीमा`. It is a new un-remediated site and \
             wants a reason or a `न६४`: {line}"
        );
    }
}

#[test]
fn the_codegens_signed_sites_are_the_numeral_formatters_and_its_twin_says_so() {
    // ADR-0030, task `D-002h`. `nidana.t1` is held signed by a sentinel the
    // twin cannot testify to; `utsarjana.t1` is the OPPOSITE case and wants
    // the opposite guard. Its three twins — `crates/sadhana/src/t1/regalloc.rs`,
    // `emit.rs` and `x86_64.rs` — declare exactly ONE signed thing between
    // them, `to_devanagari_numeral(n: i64)`, and it is not in the register
    // allocator at all. So every `अ६४` that survives the repair in that file
    // must belong to the numeral formatter, and this checks BOTH halves: the
    // five sites by their exact text, so respelling one fails by name, and
    // the twin's `i64` and the two other twins' silence, so the reading
    // cannot quietly stop being true underneath them.
    let text = std::fs::read_to_string(root().join("crates/sadhana-t1/src/utsarjana.t1"))
        .expect("utsarjana.t1 exists");
    let code: Vec<String> = text
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or("").trim().to_string())
        .collect();

    let signed: Vec<&String> = code.iter().filter(|l| l.contains("अ६४")).collect();
    // FOUR OF THE FIVE LEFT THIS LIST, AND ONLY ONE OF THE TWO REASONS IS A
    // GOOD ONE. `अवशेषः` is not a declaration any more — the 2026-08-30
    // appender rewrite inlined it into the `अङ्कावशेषकोश` store, so it is
    // gone rather than respelled.
    //
    // THE OTHER THREE ARE `इ६४` IN THIS BRANCH, AND THAT IS DEBT, NOT A FIX.
    // (A fourth, `विस्थापनम्` at :1238, is outside the formatter and is
    // counted with them below; it is a stack displacement, genuinely signed,
    // and equally unspellable in the frozen grammar.)
    // ADR-0030 is explicit that `इ६४` is a TRANSLITERATION LEAK and not a
    // type name: `integer_type = ( "अ" | "न" ) , type_width` cannot produce
    // it. It is tempting to read `इ६४` as the honest spelling for a value the
    // twin declares `i64` — that reading is wrong, and the ledger's `इ६४`
    // count exists precisely because the frozen grammar has NO signed width
    // and these sites are waiting on one.
    //
    // `अङ्कचिह्नम्`'s `अङ्कम्` is the one that stays `अ६४`, and the reason is
    // unchanged: T1 has no cast and its one caller hands it that expression.
    let expected = ["सार्वजनिक वृत्तिः अङ्कचिह्नम् आदाय अङ्कम् ॱॱ अ६४ ददाति पाठः आदि"];

    // The formatter's `इ६४` sites are PINNED AS DEBT: exactly three, so the
    // count cannot grow quietly, and they are named so that discharging them
    // — when the grammar gains a signed width, or when the callers are
    // restructured to need none — is what makes this assertion fail.
    let leaked: Vec<&String> = code.iter().filter(|l| l.contains("इ६४")).collect();
    assert_eq!(
        leaked.len(),
        4,
        "`utsarjana.t1`'s `इ६४` sites moved. ADR-0030 calls `इ६४` a \
         transliteration leak the frozen `integer_type` cannot produce, and \
         these four — `देवनागराङ्कः`'s `मूल्यम्`, its `शेषम्`/`भागः` locals, \
         and `विस्थापनम्`, the stack displacement at :1238 — are part of the \
         ledger's acknowledged `इ६४` debt, NOT a correct spelling. More is a new leak; fewer means one was discharged \
         and this number should come down with it. Found: {leaked:?}"
    );
    // THE FIVE ARE STILL ASSERTED BY THEIR EXACT TEXT, so respelling one
    // still fails by name. What the merge of 2026-09-02 changed is that they
    // are no longer the ONLY signed sites in the file.
    for want in expected {
        assert!(
            signed.iter().any(|l| l.as_str() == want),
            "`utsarjana.t1` no longer writes `{want}`. It is one of the \
             numeral formatter's five sites, which the twin's \
             `to_devanagari_numeral(n: i64)` is the reason for. Found: \
             {signed:?}"
        );
    }

    // THE SECOND GROUP IS THE OUTPUT-BUFFER SUBSYSTEM, AND IT IS THIS
    // BRANCH'S OWN CODE. `origin/main` remediated `utsarjana.t1` to `न६४`,
    // but its version PREDATES the 2026-08-30 rewrite of the emitter from
    // `योगः` to an appender — the rewrite that made the file executable at
    // all — so its sweep had nothing to say about `निर्गमसूचकाङ्क`, the four
    // `*योजनम्` appenders, or the `लेख*` locals that hold their returns. The
    // merge ported 138 of its decisions here by name; these it could not.
    //
    // EVERY ONE IS A BYTE OFFSET INTO THE OUTPUT BUFFER and belongs in `न६४`.
    // They are NOT converted here because the cluster is only correct
    // converted WHOLE — the cursor, the four appenders' parameters and
    // returns, and every local that takes one — and doing that inside a merge
    // is how a type error gets a merge's excuse. It is `D-002h`'s next unit.
    const OUTPUT_BUFFER_CLUSTER: &[&str] = &[
        "निर्गमसूचकाङ्क",
        "निर्गमारम्भः",
        "निर्गमांशः",
        "अङ्कावशेषकोश",
        "अष्टकयोजनम्",
        "यतियोजनम्",
        "विवरयोजनम्",
        "पाठयोजनम्",
        // `टिप्पनीचिह्नयोजनम्` was here: deleted with the retired T0 pair, `W-237`.
        "विच्छेदयोजनम्",
        // `त्रिपदवचनम्` was here: deleted with the retired T0 pair, `W-237`.
        "अधिकरणग्रहणम्",
        "अधिकरणसाम्यम्",
        "यवनपर्वोत्सर्जनम्",
        "यत्यष्टकम्",
        "विवराष्टकम्",
        "आदिस्थलम्",
        "संख्यानम्",
        "लेख",
    ];
    for line in &signed {
        let known = expected.contains(&line.as_str())
            || OUTPUT_BUFFER_CLUSTER.iter().any(|o| line.contains(o));
        assert!(
            known,
            "a site in `utsarjana.t1` is spelled `अ६४` and is NEITHER one of \
             the numeral formatter's five NOR part of the output-buffer \
             cluster. Everything else in that file is an index, a count, a \
             position, a register number or a spill slot, and is `न६४`: {line}"
        );
    }

    // `इ६४` is not a spelling this grammar has, and the file was one of the
    // four that wrote it. Asserted here as well as in the ledger so that the
    // per-file claim above cannot pass on a file that still writes it.
    // THIS GUARD CAME FROM `origin/main`, WHERE IT WAS TRUE, AND IT IS NOT
    // TRUE HERE — `origin/main` swept this file's three `इ६४` sites while this
    // branch rewrote the emitter around them. It is kept, inverted into the
    // pinned count above rather than deleted, because deleting it would lose
    // the claim entirely; the count discharges to zero and then this line can
    // go back to being an absolute.
    assert!(
        leaked.len() <= 4,
        "`utsarjana.t1` writes MORE `इ६४` than the three ADR-0030 already \
         counts as debt; that spelling is refused and the file may not gain \
         another. Found: {leaked:?}"
    );

    // The other side of the reading. If `regalloc.rs` or `x86_64.rs` ever
    // grows a signed field, the claim that the allocator is unsigned BY THE
    // TWIN stops being true and this file needs re-reading — which is a thing
    // to be told, not to discover by a wrong sign.
    let twin = |name: &str| {
        std::fs::read_to_string(root().join("crates/sadhana/src/t1").join(name))
            .unwrap_or_else(|_| panic!("read crates/sadhana/src/t1/{name}"))
    };
    // `emit.rs` WAS THE TWIN READ HERE and is gone — `W-237`, 2026-09-04,
    // deleted it with `कार्यक्रमोत्सर्जनम्` (its text was not T0). The numeral
    // appender `देवनागराङ्कः` stayed, and its signed original is now
    // `riscv64.rs`'s `devanagari(n: i64)`, which `यन्त्राङ्कः` twins.
    assert!(
        twin("riscv64.rs").contains("pub fn devanagari(n: i64)"),
        "`riscv64.rs` no longer declares `devanagari(n: i64)`, which \
         is the only signed declaration behind `utsarjana.t1`'s `अ६४` numeral sites"
    );
    for name in ["regalloc.rs", "x86_64.rs"] {
        let src = twin(name);
        // A CAST IS NOT A DECLARATION, and this used to count both. On this
        // branch `regalloc.rs:210` writes `Instruction::ConstInt(i as i64)` —
        // one cast, in a builder, declaring nothing — and a whole-word search
        // for `i64` reported the allocator as signed on the strength of it.
        // The claim here is about DECLARED types: a field, a binding or a
        // signature. So the type that follows an `as` is dropped first.
        //
        // This is the same error ADR-0037 records for `शेषः`: a search whose
        // key cannot tell a position from an occurrence. Worth the few lines
        // to not make it twice in one file.
        let declared: String = src
            .split(" as ")
            .enumerate()
            .map(|(i, part)| {
                if i == 0 {
                    part
                } else {
                    part.split_once(|c: char| !c.is_alphanumeric() && c != '_')
                        .map_or("", |x| x.1)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        for signed_ty in ["i8", "i16", "i32", "i64", "i128", "isize"] {
            assert!(
                !declared
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|w| w == signed_ty),
                "`crates/sadhana/src/t1/{name}` now declares `{signed_ty}`. \
                 `utsarjana.t1`'s register allocator was spelled `न६४` \
                 throughout on the reading that these two twins declare no \
                 signed type at all"
            );
        }
    }
}

#[test]
fn a_cast_is_a_second_parameter_and_the_grammar_has_no_other_way() {
    // ADR-0031, task `D-002h` item 4. Three cycles left the `स्थापनम्`
    // boundary recorded as a contradiction "no reading can remove": a signed
    // value reaching a parameter whose twin is `Slot::place(v: u64)`, which
    // Rust crosses with `as u64` and T1 cannot. The answer is that the corpus
    // already writes the crossing — the CALLER carries both readings — and
    // that no cast is possible in this language even if one were wanted. Four
    // legs, and each is a thing that could quietly stop being true.

    // (1) NO CAST CAN BE WRITTEN. A cast would have to be an expression, and
    // no expression production in the frozen grammar mentions a type at all.
    // Checked over the whole ladder rather than at `unary_expr`, because a
    // cast could be spelled at any rung of it.
    let type_words = [
        "type",
        "type_name",
        "type_width",
        "integer_type",
        "float_type",
        "slice_type",
        "pointer_type",
        "optional_type",
        "error_union",
    ];
    for rung in [
        "expression",
        "compare_expr",
        "or_expr",
        "xor_expr",
        "and_expr",
        "shift_expr",
        "add_expr",
        "mul_expr",
        "unary_expr",
        "call_expr",
        "postfix_expr",
        "primary",
        "group",
        "index",
        "argument_list",
    ] {
        let rhs = production(rung);
        for word in type_words {
            assert!(
                !rhs.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .any(|w| w == word),
                "`{rung}` now names `{word}`, so a type can stand where a value \
                 does and T1 has grown a cast. ADR-0031 rests on the opposite: \
                 no T1 expression changes the type of what it evaluates, which \
                 is why a conversion routine cannot be written in T1 at all"
            );
        }
    }

    // (2) THE IDIOM THAT STANDS IN ITS PLACE, IN THE FILE THAT INHERITS THE
    // BOUNDARY. `encode.t1` states it in its own comment and writes it at both
    // arities. Pinned by exact text, so a respelling of either member fails by
    // name rather than by a count that two changes could cancel out.
    let encode = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("encode.t1 exists");
    let squashed: String = encode.split_whitespace().collect::<Vec<_>>().join(" ");
    // The comment marker opens every prose line, so it has to come off before
    // a sentence that wraps can be matched at all.
    let prose: String = encode
        .lines()
        .map(|l| l.trim().trim_start_matches('\u{0970}'))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        prose.contains(
            "मूल्यानि and चिह्नितानि are the SAME numbers twice, unsigned and \
             signed: अन्तर्भावः needs both, T1 has no cast"
        ),
        "`encode.t1` no longer states the two-reading idiom in its own words. \
         That comment is ADR-0031's primary witness: the port met the missing \
         cast and answered it by making the caller supply both readings"
    );
    // THE FOUR LINES BELOW ARE THE REMEDIATED SPELLING AND THEY MOVED ONCE.
    // ADR-0031 read the pair off `अ६४`/`इ६४`, the state the file was in when it
    // was written; the cycle that took `encode.t1`'s signed half respelled it
    // `न६४`/`अ६४` under that very ADR. The SHAPE is what the rule was read off
    // and the shape is unchanged — two readings of one number, declared side by
    // side, because T1 has no cast. Both members still differ, which is the
    // whole content of the check.
    for pair in [
        // the scalars — ADR-0030's own witness line, read for its SHAPE
        "मूल्यम् ॱॱ न६४ चिह्नितम् ॱॱ अ६४",
        // the same thing as two parallel slices
        "मूल्यानि ॱॱ अङ्कः अन्तः न६४ चिह्नितानि ॱॱ अङ्कः अन्तः अ६४",
        // and the loop unpacking the slices back into the scalars
        "चरः मूल्यम् ॱॱ न६४ भवति मूल्यानि अङ्कः सूचकाङ्क अन्तः ।",
        "चरः चिह्नितम् ॱॱ अ६४ भवति चिह्नितानि अङ्कः सूचकाङ्क अन्तः ।",
    ] {
        assert!(
            squashed.contains(pair),
            "`encode.t1` no longer writes `{pair}`. The pair is how this corpus \
             crosses a sign boundary; if it has gone, ADR-0031's rule has lost \
             the site it was read off"
        );
    }

    // (3) `स्थापनम्`'s TWIN IS A BIT WALK, WHICH IS WHY ITS `u64` IS A PATTERN
    // AND NOT A MAGNITUDE IN DOUBT. It reads `value` with `>>` and `& 1` and
    // with nothing else — no comparison, no addition, no subtraction — while
    // `fits` beside it re-reads the same word as `i64`. Two readings of one
    // datum, which is W-075's split and not a conversion.
    let encode_rs = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("encode.rs exists");
    let place = encode_rs
        .split_once("pub fn place(&self, value: u64) -> u32 {")
        .expect("`encode.rs` still declares `Slot::place(value: u64) -> u32`")
        .1
        .split_once("\n    }\n")
        .expect("`place` has a body")
        .0;
    assert!(
        place.contains("value >> from & 1"),
        "`Slot::place` no longer walks its parameter bit by bit. ADR-0031 reads \
         that parameter as a BIT PATTERN on exactly this evidence"
    );
    for arith in ["value <", "value >=", "value +", "value -", "value as"] {
        assert!(
            !place.contains(arith),
            "`Slot::place` now does `{arith}` on its parameter, so it reads it \
             as a NUMBER and not only as a pattern. ADR-0031's third leg — that \
             the value it is handed has no sign to convert — needs re-reading"
        );
    }
    assert!(
        encode_rs.contains("let signed = value as i64;"),
        "`encode.rs` no longer re-reads `fits`'s `u64` parameter as `i64`. That \
         line is the cast the port hoisted into `अन्तर्भावः`'s second \
         parameter, and it is where the whole idiom comes from"
    );

    // (4) ARITHMETIC DOES NOT REACH THE CASE EITHER, AND THE CORPUS SAYS SO.
    // `उद्धरणम्` converts the OTHER way in pure T1 arithmetic — two's
    // complement by subtracting `१ वामसृ विस्तारः` — but only below width 64,
    // because at 64 there is no bit above the sign to borrow from. That guard
    // is what makes a T1-arithmetic conversion routine impossible at the one
    // width every site in this decision is at.
    let vishlesana = std::fs::read_to_string(root().join("crates/sadhana-t1/src/vishlesana.t1"))
        .expect("vishlesana.t1 exists");
    let vishlesana: String = vishlesana.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        vishlesana.contains("यदि विस्तारः न्यूनम् ६४ आदि"),
        "`उद्धरणम्` no longer guards its sign extension on `विस्तारः < ६४`. \
         ADR-0031's second leg is that the arithmetic bridge stops exactly at \
         the width the `स्थापनम्` boundary is at"
    );
    assert!(
        vishlesana.contains("प्रत्यागमनम् मूल्यम् वियोगः आरभ्य १ वामसृ विस्तारः समाप्तम् ।"),
        "`उद्धरणम्` no longer performs its conversion by subtraction. That line \
         is the only conversion this corpus writes without a cast, and ADR-0031 \
         rests on where it CANNOT be written rather than on its absence"
    );
}

#[test]
fn the_operator_set_is_the_corpus_set_in_both_directions() {
    // ADR-0026, task `D-002h`. The owner ruled on 2026-08-29 that where
    // `spec/lexicon.tsv`, the `.t1` corpus and `crates/tree-sitter-t1` disagree
    // about T1's operators, THE CORPUS WINS. So the grammar may not carry an
    // operator the corpus does not have, and — the direction that is easy to
    // forget — it may not omit one the corpus DOES have. Both are checked.
    //
    // `ast.t1`'s `*द्विकर्मभेद` constants are the closed set. Pairing each
    // with its operator word here is the only hand-written thing in this test,
    // and it is what makes a drift on either side fail by name.
    //
    // FIFTEEN SINCE ADR-0037. The two additions are spelled as the corpus
    // writes them in OPERATOR POSITION, which is not the same as the spelling
    // it writes most often: `artha.t1` and `utsarjana.t1` between them write
    // `शेषम्` 23 times, but every one is a local variable holding a remainder,
    // and the operator — `encode.t1:647`, `:676` — is `शेषः`. See
    // `t1_operators.rs`'s `operator_position_census`.
    let pairs: [(&str, &str); 15] = [
        ("योगद्विकर्मभेद", "योगः"),
        ("वियोगद्विकर्मभेद", "वियोगः"),
        ("गुणनद्विकर्मभेद", "गुणनम्"),
        ("विभाजनद्विकर्मभेद", "विभाजनम्"),
        ("समद्विकर्मभेद", "समम्"),
        ("असमद्विकर्मभेद", "असमम्"),
        ("न्यूनद्विकर्मभेद", "न्यूनम्"),
        ("अधिकद्विकर्मभेद", "अधिकम्"),
        ("युक्द्विकर्मभेद", "युक्"),
        ("विकल्पद्विकर्मभेद", "विकल्प"),
        ("विषमद्विकर्मभेद", "विषम"),
        ("वामसृद्विकर्मभेद", "वामसृ"),
        ("दक्षिणसृद्विकर्मभेद", "दक्षिणसृ"),
        ("शेषद्विकर्मभेद", "शेषः"),
        ("बृहत्समद्विकर्मभेद", "बृहत्समम्"),
    ];

    let ast = std::fs::read_to_string(root().join("crates/sadhana-t1/src/ast.t1"))
        .expect("read crates/sadhana-t1/src/ast.t1");
    let declared: Vec<&str> = ast
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .filter(|w| w.ends_with("द्विकर्मभेद"))
        .collect();
    // FIFTEEN SINCE ADR-0037: `शेषद्विकर्मभेद` (१४) and `बृहत्समद्विकर्मभेद`
    // (१५), both added because the corpus writes them — which is ADR-0032's
    // own principle applied, not overruled.
    assert_eq!(
        declared.len(),
        15,
        "`ast.t1` declares {} binary-operator kinds; ADR-0026 froze 13. A kind \
         was added or removed without amending the grammar.",
        declared.len()
    );

    let ts = terminals();
    for (kind, word) in pairs {
        assert!(
            declared.contains(&kind),
            "`{kind}` is in ADR-0026's table but no longer in `ast.t1`"
        );
        assert!(
            ts.iter().any(|t| t == word),
            "`ast.t1` declares `{kind}` but `spec/grammar-t1.ebnf` spells no \
             `{word}`; the grammar has fallen behind the canonical corpus"
        );
    }
    for kind in &declared {
        assert!(
            pairs.iter().any(|(k, _)| k == kind),
            "`ast.t1` declares `{kind}`, which ADR-0026 does not name — the \
             corpus grew an operator and the grammar cannot spell it"
        );
    }

    // The one unary operator, and the one assignment verb.
    assert!(
        ts.iter().any(|t| t == "ऋण"),
        "`ऋण` is the only unary operator"
    );
}

#[test]
fn twelve_of_the_thirteen_operators_are_written_and_the_thirteenth_is_named() {
    // ADR-0026 freezes `विषम` (xor) on `ast.t1`'s authority and says out loud
    // that NO SOURCE WRITES IT. That is the kind of claim which quietly stops
    // being true, so it is asserted rather than only written down — in both
    // directions, because the interesting failure is the other one: an operator
    // this test believes is attested falling out of the corpus entirely would
    // mean the grammar is frozen on something nobody writes.
    let counts = corpus_tokens();
    let attested = [
        "योगः",
        "वियोगः",
        "गुणनम्",
        "विभाजनम्",
        "समम्",
        "असमम्",
        "न्यूनम्",
        "अधिकम्",
        "युक्",
        "विकल्प",
        "वामसृ",
        "दक्षिणसृ",
        "ऋण",
        "भवति",
    ];
    for w in attested {
        let n = counts.get(w).copied().unwrap_or(0);
        assert!(
            n > 0,
            "ADR-0026 froze `{w}` because the corpus writes it; the corpus now \
             writes it 0 times, so the evidence for the freeze is gone"
        );
    }
    assert_eq!(
        counts.get("विषम").copied().unwrap_or(0),
        0,
        "ADR-0026 says `विषम` is named by `ast.t1` and written by no source. It \
         is now written. Update the ADR — the exception it makes is no longer \
         needed, and an unattested-operator caveat that has become false is \
         worse than none."
    );

    // The four the lexicon proposes and the corpus does NOT have. ADR-0026
    // refuses to add them; this is the check that the refusal stays factual.
    for absent in ["शेष", "च", "वा", "व्यत्यय", "न्यूनसमम्", "अधिकसमम्"]
    {
        assert_eq!(
            counts.get(absent).copied().unwrap_or(0),
            0,
            "`{absent}` is in `spec/lexicon.tsv` as an operator and ADR-0026 \
             refused it as unattested. The corpus now writes it, so the grammar \
             is missing an operator the canonical source has."
        );
    }

    println!(
        "METRIC t1_operators_attested {}",
        attested.iter().filter(|w| counts.contains_key(**w)).count()
    );
}

#[test]
fn bhavati_is_not_on_the_precedence_ladder() {
    // ADR-0026's finding, and the one most likely to be "fixed" by a later
    // reader who assumes assignment must be an expression: all 1,788
    // occurrences of `भवति` are statement-initial or declarative, so it belongs
    // to `statement` and not to `expression`. If it ever appears in an
    // expression production, the grammar has started describing a language the
    // corpus does not write.
    let text = grammar();
    for line in text.lines() {
        let is_expr_production = line.starts_with("expression")
            || line.starts_with("compare_expr")
            || line.starts_with("or_expr")
            || line.starts_with("xor_expr")
            || line.starts_with("and_expr")
            || line.starts_with("shift_expr")
            || line.starts_with("add_expr")
            || line.starts_with("mul_expr")
            || line.starts_with("unary_expr")
            || line.starts_with("postfix_expr")
            || line.starts_with("call_expr")
            || line.starts_with("argument_list")
            || line.starts_with("primary")
            || line.ends_with("_op       = \"भवति\" ;");
        if is_expr_production {
            assert!(
                !line.contains("भवति"),
                "`भवति` is the assignment statement's verb, not an operator: {line}"
            );
        }
    }
    assert!(
        text.contains("ADR-0026"),
        "the decision that permits the operator freeze is named, so the next \
         reader can check it rather than take the freeze on trust"
    );
}

#[test]
fn the_statement_set_is_the_corpus_set_and_the_absences_are_real() {
    // ADR-0027, task `D-002h`. `statement` was frozen on six forms because the
    // corpus writes six, and on NO loop-exit statement because it writes none.
    // The second half is the one that rots silently: a `विरम्` appearing in the
    // corpus later would make the grammar wrong with nothing to say so. Both
    // directions are therefore re-derived here rather than trusted.
    let counts = corpus_tokens();
    let text = grammar();

    // The six leading words, with the counts ADR-0027 recorded. The assertion
    // is `>=` and not `==` on purpose: the corpus is still being written, and a
    // count that only grows is evidence the form is still the form. A count
    // that FALLS means the ADR measured something that is no longer there.
    for (word, floor) in [
        ("चरः", 1039usize),
        ("भवति", 1788),
        ("यदि", 670),
        ("अन्यथा", 29),
        ("यावत्", 120),
        ("प्रत्यागमनम्", 779),
    ] {
        let seen = counts.get(word).copied().unwrap_or(0);
        assert!(
            seen >= floor,
            "ADR-0027 froze a statement form on `{word}` at {floor} occurrences; \
             the corpus now writes it {seen} times, so the measurement the freeze \
             stands on has gone backwards"
        );
        assert!(
            text.contains(word),
            "`{word}` leads a statement the corpus writes {seen} times and the \
             grammar cannot spell it"
        );
    }

    // THE REFUSALS, and the reason they cannot be checked with `!contains`:
    // every one of these words is ALREADY in this file, in the frozen
    // `keyword` production, because doc 02 §3.2 ratified 24 keywords and the
    // corpus writes eleven of them. So the claim is not "the grammar does not
    // know the word" — it does — but "no STATEMENT FORM is given to it".
    for (word, sense) in [
        ("प्रत्येकम्", "for"),
        ("भङ्गः", "break"),
        ("अनुवर्तनम्", "continue"),
        ("विकल्पना", "switch"),
        ("रक्षा", "defer"),
    ] {
        let seen = counts.get(word).copied().unwrap_or(0);
        assert_eq!(
            seen, 0,
            "ADR-0027 gave `{word}` ({sense}) no statement form because the \
             corpus writes it zero times. It now writes it {seen} times, so the \
             production exists to be written and the ADR must be superseded"
        );
        assert!(
            text.contains(word),
            "`{word}` is one of doc 02 §3.2's 24 ratified keywords and has gone \
             missing from `keyword`; the refusal above rests on it being spelled \
             already, so a later corpus needs no invention"
        );
    }

    // The other direction, and the one a later edit is likelier to break:
    // `statement` must still list EXACTLY the six forms that were measured.
    // A seventh appearing without a corpus word behind it is the invention this
    // ADR refused, and it would slip past every assertion above.
    let stmt = text
        .split("\nstatement ")
        .nth(1)
        .expect("`statement` is frozen by ADR-0027");
    let stmt = stmt.split(';').next().expect("`statement` ends in a `;`");
    let forms: Vec<&str> = stmt
        .split('|')
        .map(|s| s.trim_start_matches(['=', ' ', '\n']).trim())
        .collect();
    assert_eq!(
        forms,
        vec![
            "binding",
            "assignment",
            "conditional",
            "loop",
            "return_statement",
            "expression_statement",
        ],
        "`statement` was measured at six forms; this is not that set"
    );

    // `अन्यथा` NEVER TAKES A CONDITION — all 29 sites are `इति अन्यथा आदि`.
    // The grammar must not have grown an `else if` chain.
    let else_line = text
        .lines()
        .find(|l| l.starts_with("conditional"))
        .expect("`conditional` is frozen by ADR-0027");
    assert!(
        !else_line.contains("अन्यथा\" , conditional") && !else_line.contains("अन्यथा\" , \"यदि\""),
        "the corpus writes no `else if`; `अन्यथा` is followed by a block and \
         nothing else: {else_line}"
    );
}

#[test]
fn a_binding_always_carries_its_type_and_an_assignment_never_does() {
    // ADR-0027's sharpest measurement, and the one a later reader is most
    // likely to "relax" into `[ ॱॱ type ]`: T1 HAS NO TYPE INFERENCE. The
    // evidence is a partition, not a sample — every `भवति` site is either a
    // `चरः` binding WITH an annotation or an assignment WITHOUT one, and the
    // two sum to the whole. If they ever stop summing, an unannotated binding
    // or an annotated assignment exists and the grammar is wrong.
    let dir = root().join("crates/sadhana-t1/src");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read crates/sadhana-t1/src")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    files.sort();

    let (mut bindings, mut assignments, mut total) = (0usize, 0usize, 0usize);
    for f in files {
        let text = std::fs::read_to_string(&f).expect("read a `.t1` source");
        for line in text.lines() {
            // The same stripper `corpus_tokens` uses: comments then strings.
            let mut code = line.split('॰').next().unwrap_or("").to_string();
            while let Some(open) = code.find("उक्तम्") {
                match code[open..].find("इति") {
                    Some(close) => code.replace_range(open..open + close + "इति".len(), " "),
                    None => {
                        code.truncate(open);
                        break;
                    }
                }
            }
            let toks: Vec<&str> = code.split_whitespace().collect();
            let Some(verb) = toks.iter().position(|t| *t == "भवति") else {
                continue;
            };
            total += 1;
            let lhs = &toks[..verb];
            if lhs.contains(&"चरः") {
                bindings += 1;
                assert!(
                    lhs.contains(&"ॱॱ"),
                    "a `चरः` binding with no `ॱॱ type`: {}",
                    code.trim()
                );
            } else {
                assignments += 1;
                assert!(
                    !lhs.contains(&"ॱॱ"),
                    "an assignment carrying a `ॱॱ type`: {}",
                    code.trim()
                );
            }
        }
    }

    assert_eq!(
        bindings + assignments,
        total,
        "every `भवति` site is a binding or an assignment, and these do not sum"
    );
    assert!(
        bindings >= 1039 && assignments >= 749 && total >= 1788,
        "ADR-0027 measured 1039 bindings + 749 assignments = 1788 `भवति` sites; \
         this tree has {bindings} + {assignments} = {total}, so a count the \
         freeze stands on has gone backwards"
    );

    // And the grammar must still say it: a REQUIRED annotation, not an optional
    // one. `[` anywhere around the annotation is the shape this guard exists to
    // refuse.
    let text = grammar();
    let binding = text
        .lines()
        .find(|l| l.starts_with("binding"))
        .expect("`binding` is frozen by ADR-0027");
    assert!(
        binding.contains("annotation , type") && !binding.contains("[ annotation"),
        "T1 has no type inference — {bindings} of {total} `भवति` sites are \
         annotated bindings and the rest are annotation-free assignments, so \
         the annotation is required: {binding}"
    );
}

#[test]
fn every_keyword_is_a_word_the_lexicon_knows_or_could() {
    // Keywords are ordinary Sanskrit words, so each is also a possible
    // identifier — which is why the grammar enumerates them. They must at
    // least be well-formed Devanagari under closure; `spec/lexicon.tsv` is
    // where their senses get ratified (`B-079b` wires that).
    let ts = terminals();
    let keywords: Vec<&String> = ts
        .iter()
        .filter(|t| t.chars().count() > 2 && !t.chars().all(|c| ('०'..='९').contains(&c)))
        .collect();
    assert!(
        keywords.len() >= 25,
        "doc 02 §3.2 lists about 35 keywords; the grammar names {}",
        keywords.len()
    );
    for k in keywords {
        assert!(
            sanskrit_text::normalize::is_nfc(k),
            "keyword `{k}` is not NFC, so two spellings of it could exist"
        );
    }
}

#[test]
fn the_extractor_actually_finds_terminals() {
    // Three cycles running I have found a check that passed by doing nothing.
    // If the comment-stripping ever eats the whole file, every test above
    // becomes an assertion about an empty list — and they would all pass.
    let ts = terminals();
    assert!(
        ts.len() >= 40,
        "only {} terminals extracted; the grammar names far more",
        ts.len()
    );
    println!("METRIC t1_grammar_terminals {}", ts.len());
}

/// The quoted terminals of the grammar's eight `*_op` productions, and nothing
/// else.
///
/// `terminals()` cannot answer this question: `न` is a terminal because it is
/// the UNSIGNED INTEGER PREFIX (`integer_type`, `:179`), and reading it as
/// evidence that the lexicon's `न` — *not*, a boolean operator the corpus never
/// writes — is frozen would be a false pass on a name collision.
fn operator_terminals() -> Vec<String> {
    let text = grammar();
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        if !name.ends_with("_op") || !line.contains('=') {
            continue;
        }
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            let mut lit = String::new();
            for c in chars.by_ref() {
                if c == '"' {
                    break;
                }
                lit.push(c);
            }
            if !lit.is_empty() {
                out.push(lit);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// `(devanagari, english, category)` for every row of a lexicon TSV.
fn lexicon_rows(file: &str) -> Vec<(String, String, String)> {
    let path = root().join("spec").join(file);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read spec/{file}: {e}"));
    // The generated file interposes `slp1` and `iast`; the source does not.
    let (english, category) = if file == "lexicon.tsv" {
        (3, 4)
    } else {
        (1, 2)
    };
    src.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() <= category {
                return None;
            }
            Some((
                f[0].to_string(),
                f[english].to_string(),
                f[category].to_string(),
            ))
        })
        .collect()
}

/// The `operator` rows the grammar has NO word for, kept because nothing
/// contradicts them.
///
/// This is the whole of ADR-0028's rule, written as a list: a lexicon proposal
/// is retired when the grammar froze a DIFFERENT word for the SAME sense, and
/// kept when the grammar has no production for that sense at all. `शेष`
/// (modulo) is the clearest case — T1 has no modulo operator, so `शेष` is
/// unattested but uncontradicted; `अधि` (plus) was contradicted by `योगः` and
/// is gone.
const OPERATORS_THE_GRAMMAR_HAS_NO_PRODUCTION_FOR: &[&str] = &[
    // `अधिकसमम्` (greater or equal) and `शेष` (modulo) STOOD HERE UNTIL
    // 2026-09-02. ADR-0037 gave both senses a production, so neither is a
    // sense the grammar lacks any more — but it froze the spellings the
    // CORPUS writes, `बृहत्समम्` and `शेषः`, not doc 15's. That is ADR-0028's
    // case exactly, so both moved to the retirement list below rather than
    // staying here.
    "च",      // and (boolean) — no production
    "न",      // not (boolean) — no production
    "न्यूनसमम्", // less or equal — no production
    // `परिवृत्तअधि` (wrapping plus) STOOD HERE UNTIL 2026-08-31. It is gone
    // because it is no longer an `operator` row at all — see the test below.
    "पूर्णक्रमेण", // by total order — no production
    "वा",      // or (boolean) — no production
    "व्यत्यय",   // bitwise not — no production; ADR-0026 refused to invent one
];

/// The words retired because the grammar froze a DIFFERENT spelling for the
/// same sense: five by ADR-0028, two more by ADR-0037 under the same rule.
///
/// The rule is doc 01 §4 rule 5 — one sense, one word. A lexicon that keeps
/// doc 15's proposal beside the frozen spelling offers two.
const RETIRED_BY_ADR_0028: &[(&str, &str)] = &[
    ("अधि", "योगः"),
    ("ऊन", "वियोगः"),
    ("गुण", "गुणनम्"),
    ("भाग", "विभाजनम्"),
    ("विषमम्", "असमम्"),
    // ADR-0037, 2026-09-02.
    ("शेष", "शेषः"),
    ("अधिकसमम्", "बृहत्समम्"),
];

#[test]
fn the_lexicon_spells_the_operators_the_grammar_froze() {
    // ADR-0028, task `D-002h`. ADR-0026 froze T1's operators from the corpus
    // and left `spec/lexicon.src.tsv` carrying doc 15's PROPOSALS for five of
    // the same senses — `अधि`/`ऊन`/`गुण`/`भाग`/`विषमम्`. Before the freeze that
    // was two unfrozen descriptions disagreeing; after it, one side is frozen,
    // so the other is simply wrong. This test is the reason a sixth cannot
    // appear quietly.
    let ops = operator_terminals();
    // SIXTEEN SINCE ADR-0037 (2026-09-02): fifteen binary and one unary.
    // ADR-0032 — renumbered from the 0026 this message still names — recorded
    // `शेषः` and `बृहत्समम्` as absent. Each entry failed differently and both
    // are corrected there: the modulo evidence was TRUE on 2026-08-30 and
    // `encode.t1:647`'s operator use is dated 08-31, the day after; the `>=`
    // entry asked whether the LEXICON's `अधिकसमम्` appears, while the corpus
    // writes `बृहत्समम्` and has since 08-29, the day before.
    assert_eq!(
        ops.len(),
        16,
        "ADR-0032 froze thirteen binary operators and one unary and ADR-0037 \
         added two binary; the grammar's \
         `*_op` productions now name {}: {ops:?}",
        ops.len()
    );

    for file in ["lexicon.src.tsv", "lexicon.tsv"] {
        let rows = lexicon_rows(file);

        // (1) Every frozen operator has a lexicon row. Three of them —
        //     `योगः`, `वियोगः`, `गुणनम्` — are `mnemonic` rows, because the word
        //     serves T0 and T1 alike; that is the shared-word pattern
        //     `a_word_that_serves_both_tiers_is_the_rule_and_not_the_exception`
        //     records for `समम्`, not an exception to it.
        for op in &ops {
            assert!(
                rows.iter().any(|(w, _, _)| w == op),
                "`{op}` is a frozen operator of spec/grammar-t1.ebnf and \
                 spec/{file} does not carry it, so the language spells a word \
                 the lexicon cannot gloss"
            );
        }

        // (2) Every `operator` row is either frozen or uncontradicted.
        for (word, english, category) in &rows {
            if category != "operator" {
                continue;
            }
            // `भवति` is category `operator` and IS a grammar terminal, but of
            // the ASSIGNMENT statement — ADR-0026 kept it off the ladder and
            // `bhavati_is_not_on_the_precedence_ladder` holds it there.
            if word == "भवति" {
                assert!(
                    !ops.iter().any(|o| o == word),
                    "`भवति` has appeared in an `*_op` production; ADR-0026 \
                     measured all 1,788 of its occurrences as statement-initial"
                );
                continue;
            }
            assert!(
                ops.iter().any(|o| o == word)
                    || OPERATORS_THE_GRAMMAR_HAS_NO_PRODUCTION_FOR.contains(&word.as_str()),
                "spec/{file} carries `{word}` ({english}) as an operator, but it \
                 is neither a frozen operator of the grammar nor listed as a \
                 sense the grammar has no production for. If the grammar froze \
                 a different word for this sense, ADR-0028 says retire this row; \
                 if it froze none, add it to the list and say so"
            );
        }

        // (3) The five retirements stay retired, in either spelling.
        for (gone, kept) in RETIRED_BY_ADR_0028 {
            assert!(
                !rows.iter().any(|(w, _, _)| w == gone),
                "`{gone}` is back in spec/{file}. ADR-0028 retired it because \
                 the grammar froze `{kept}` for the same sense; two words for \
                 one sense is the drift doc 01 §4 rule 5 forbids"
            );
            assert!(
                rows.iter().any(|(w, _, _)| w == kept),
                "ADR-0028 retired `{gone}` in favour of `{kept}`, and `{kept}` \
                 is not in spec/{file} — the reconciliation deleted a word and \
                 replaced it with nothing"
            );
        }
    }

    println!("METRIC t1_operators_in_lexicon {}", ops.len());
}

#[test]
fn no_lexicon_operator_is_built_on_a_retired_word() {
    // THE LOOSE END IS TIED, BY OWNER RULING 2026-08-31, AND THE SET IS NOW
    // EMPTY. This test used to assert `["परिवृत्तअधि"]` and the comment here
    // argued for leaving it: "the honest move is not to respell it
    // `परिवृत्तयोगः`, because no source writes either and ADR-0026's rule is
    // that the corpus decides."
    //
    // THE MERGE MADE THAT POSITION UNTENABLE, because the other branch had
    // frozen the OPPOSITE rule in `sadhana/tests/t1_operators.rs`: its
    // `SUPERSEDED` table required `परिवृत्तअधि` NOT be offered as an operator,
    // respelled to `परिवृत्तयोगः`. Two suites, two frozen rules, no lexicon
    // state satisfying both — so it went to the owner rather than to whichever
    // test file got run last.
    //
    // THE RULING TOOK THE THIRD OPTION AND IT IS THE ONE THIS COMMENT'S OWN
    // REASONING POINTS AT: neither spelling is an operator. Measured that day,
    // `परिवृत्तअधि` and `परिवृत्तयोगः` each appear ZERO times in the `.t1`
    // corpus. ADR-0026's rule is that the corpus decides; the corpus declines
    // both. So `परिवृत्तअधि` is a `concept` row now, not an `operator` one, and
    // nothing is stranded. The test still fails the moment a compound built on
    // a retired root is offered as an operator, which is the drift it was
    // written to catch.
    //
    // The match is `ends_with` and not `contains`: a retired root is the HEAD
    // of a compound and heads come last here — `परिवृत्तअधि` is *wrapping* +
    // *plus*. A leading match is a false positive and there are two of them,
    // `अधिकम्` and `अधिकसमम्`, which are `अधिक` ("greater") and not `अधि` +
    // anything.
    //
    // THE CATEGORY TEST IS `split('+')` AND NOT `==`. It was `== "operator"`
    // until 2026-08-31, which was exact enough while every operator row said
    // just `operator`. The same ruling made `योगः`, `वियोगः` and `गुणनम्` into
    // `mnemonic+operator`, and an equality check would have silently stopped
    // seeing every dual-role row — so a compound built on a retired root could
    // have returned through the one category the fix introduced.
    let rows = lexicon_rows("lexicon.src.tsv");
    let stranded: Vec<&str> = rows
        .iter()
        .filter(|(w, _, c)| {
            c.split('+').any(|x| x == "operator")
                && RETIRED_BY_ADR_0028
                    .iter()
                    .any(|(gone, _)| w.ends_with(gone) && w != gone)
        })
        .map(|(w, _, _)| w.as_str())
        .collect();
    let empty: [&str; 0] = [];
    assert_eq!(
        stranded, empty,
        "an operator word is built on a root ADR-0028 retired. The owner ruled \
         on 2026-08-31 that no such word is an operator — the corpus writes \
         none of them — so this is new drift and should be spelled from the \
         frozen set"
    );
}

#[test]
fn the_t0_mnemonics_that_share_a_word_with_t1_say_so() {
    // `योगः`, `वियोगः` and `गुणनम्` are `mnemonic` rows that ADR-0028 made carry
    // a T1 sense too. A shared word is only safe while it is WRITTEN DOWN as
    // shared: the failure mode is a later reader seeing `mnemonic` and adding a
    // second, `operator`-category row for the T1 sense, which is exactly the
    // synonym drift `no_concept_has_two_words` catches one step too late.
    let rows = lexicon_rows("lexicon.src.tsv");
    for word in ["योगः", "वियोगः", "गुणनम्"] {
        let (_, _, category) = rows
            .iter()
            .find(|(w, _, _)| w == word)
            .unwrap_or_else(|| panic!("`{word}` is no longer in spec/lexicon.src.tsv"));
        // AMENDED 2026-08-31: this asserted `category == "mnemonic"` exactly.
        // The comment above states the intent — a shared word is only safe
        // while it is WRITTEN DOWN as shared, and the failure mode named is a
        // SECOND row appearing for the T1 sense. `mnemonic+operator` is still
        // ONE row and says the sharing out loud, so it serves that intent
        // better than bare `mnemonic`, which left the T1 role implicit in a
        // note column. The `count == 1` assertion below is the substance and is
        // untouched.
        //
        // It changed because the other branch's `t1_operators.rs` requires
        // every operator the corpus writes to carry `operator` in its category,
        // and `योगः`/`वियोगः`/`गुणनम्` ARE written as T1 operators. A bare
        // `mnemonic` satisfied neither suite.
        assert!(
            category.split('+').any(|c| c == "mnemonic"),
            "`{word}` is a T0 mnemonic and must still say so; it is now `{category}`"
        );
        assert!(
            category == "mnemonic" || category == "mnemonic+operator",
            "`{word}` serves T0 and T1 from ONE row, so its category is \
             `mnemonic` or `mnemonic+operator`; it is now `{category}`"
        );
        assert_eq!(
            rows.iter().filter(|(w, _, _)| w == word).count(),
            1,
            "`{word}` has a second row — the T1 sense was given its own entry \
             instead of sharing the T0 one, which is two words for one sense"
        );
    }
}

// ── `crates/tree-sitter-t1/grammar.js` — `D-002h`'s reconciliation ──────────
//
// The tree-sitter grammar is the definition of T1 that a TOOL ACTUALLY RUNS —
// an editor, an indexer, `tree-sitter parse`. Until 2026-08-31 it carried a
// THIRD definition of the language, built out of ASCII `{ } ( ) ;` and two
// words the corpus does not have (`कार्यम्` for fn, 0 outside `कार्यक्रमः`;
// `समर्पय` for return, 0), against `वृत्तिः` 325 and `प्रत्यागमनम्` 783. That
// was tolerable while the grammar was unfrozen. ADR-0026 and ADR-0027 froze
// the expression ladder, `statement`, `type` and `call`, which makes any other
// definition of T1 a bug rather than an opinion, and the owner's 2026-08-29
// ruling — the `.t1` corpus is canonical — is what the frozen grammar was
// measured from. So the reconciliation invents nothing.
//
// These three tests hold the two files together in BOTH directions, which is
// the only shape that stops one of them drifting alone. They read `grammar.js`
// as TEXT and never invoke `tree-sitter`: the CLI is optional (`B-102`), the
// crate is excluded from the workspace (`Cargo.toml:15`), and a guard that
// silently skips when a tool is missing is the guard that was not there for
// the last two years of this file's life.

/// `crates/tree-sitter-t1/grammar.js` with its comments removed, so the prose —
/// which quotes `'ऋण'` and `'उक्तम्'` while explaining them — cannot masquerade
/// as grammar. Same reason `terminals()` strips `(* … *)` from the EBNF.
fn tree_sitter_grammar() -> String {
    let path = root().join("crates/tree-sitter-t1/grammar.js");
    let src = std::fs::read_to_string(&path).expect("read crates/tree-sitter-t1/grammar.js");

    let mut out = String::new();
    let mut rest = src.as_str();
    loop {
        let block = rest.find("/*");
        let line = rest.find("//");
        let (cut, close) = match (block, line) {
            (Some(b), Some(l)) if b < l => (b, "*/"),
            (Some(_), Some(l)) => (l, "\n"),
            (Some(b), None) => (b, "*/"),
            (None, Some(l)) => (l, "\n"),
            (None, None) => break,
        };
        out.push_str(&rest[..cut]);
        match rest[cut..].find(close) {
            Some(end) => rest = &rest[cut + end + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Every single-quoted literal in the tree-sitter grammar's rules.
fn tree_sitter_literals() -> Vec<String> {
    let code = tree_sitter_grammar();
    let mut out = Vec::new();
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\'' {
            continue;
        }
        let mut lit = String::new();
        for c in chars.by_ref() {
            if c == '\'' {
                break;
            }
            lit.push(c);
        }
        if !lit.is_empty() {
            out.push(lit);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Every rule name of the tree-sitter grammar, with tree-sitter's hidden-rule
/// underscore stripped. The ladder levels are hidden so a lone operand does not
/// come back wrapped in nine single-child nodes; the NAMES still have to be the
/// EBNF's, which is what this strip is for.
fn tree_sitter_rules() -> Vec<String> {
    let code = tree_sitter_grammar();
    let mut out = Vec::new();
    for line in code.lines() {
        let t = line.trim();
        let Some(colon) = t.find(':') else { continue };
        if !t[colon..].starts_with(": $ =>") {
            continue;
        }
        let name = t[..colon].trim_start_matches('_');
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            out.push(name.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn the_tree_sitter_grammar_spells_only_words_the_grammar_froze() {
    // The direction that catches an invented word. `समर्पय` was one for as
    // long as this file has existed, and nothing could see it — mutation-check
    // this test by putting it back in `return_statement` and it fails BY NAME.
    let frozen = terminals();
    let mut strayed = Vec::new();
    for lit in tree_sitter_literals() {
        if !lit.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c)) {
            continue; // an ASCII literal is the next test's business
        }
        if !frozen.contains(&lit) {
            strayed.push(lit);
        }
    }
    assert!(
        strayed.is_empty(),
        "crates/tree-sitter-t1/grammar.js spells {strayed:?}, which spec/grammar-t1.ebnf \
         does not. The EBNF is frozen (ADR-0026, ADR-0027); the tree-sitter grammar is a \
         TRANSCRIPTION of it, so a word here that is not there is this file's bug."
    );
}

#[test]
fn no_ascii_delimiter_survives_in_the_tree_sitter_grammar() {
    // Doc 02 §3.2's `{ } ( ) , = + * →` are a SKETCH and say so — "for
    // legibility while the semantics are under discussion". The tree-sitter
    // grammar had transcribed the sketch. Every one of these roles has a
    // ratified Devanagari word (ADR-0003's replaced-pairs table), so an ASCII
    // delimiter here is not a shortcut, it is a different language.
    let sketch = ['{', '}', '(', ')', ';', ',', '[', ']', '=', '"', '<', '>'];
    let mut found = Vec::new();
    for lit in tree_sitter_literals() {
        if lit.chars().all(|c| sketch.contains(&c)) {
            found.push(lit);
        }
    }
    assert!(
        found.is_empty(),
        "crates/tree-sitter-t1/grammar.js uses the ASCII delimiters {found:?}. Doc 02 §3.2 \
         writes those \"for legibility while the semantics are under discussion\"; ADR-0003 \
         ratified words for every one of the roles."
    );
}

/// Every production name of `spec/grammar-t1.ebnf`, in file order.
fn ebnf_productions() -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = grammar();
    // Strip comments first: `(* … *)` prose names productions too.
    let mut code = String::new();
    while let Some(open) = rest.find("(*") {
        code.push_str(&rest[..open]);
        match rest[open..].find("*)") {
            Some(close) => rest = rest[open + close + 2..].to_string(),
            None => {
                rest = String::new();
                break;
            }
        }
    }
    code.push_str(&rest);
    for line in code.lines() {
        let Some(eq) = line.find('=') else { continue };
        let name = line[..eq].trim();
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            out.push(name.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn the_tree_sitter_grammar_carries_every_production_the_ebnf_defines() {
    // The other direction, and the one that catches a SHRINKING transcription:
    // a rule quietly dropped from `grammar.js` leaves a construct unparsed, and
    // an unparsed construct is not an ERROR node — it is a clean parse of
    // something else. Cycle 128 learned that on the T0 grammar, where 23
    // instructions were swallowed into one directive with no ERROR anywhere.
    //
    // ABSENT ON PURPOSE, each for a stated reason. Nothing else may join this
    // list without an edit that says why.
    const ABSENT: &[(&str, &str)] = &[
        // Enumerations of alternatives the parser spells inline. `keyword`'s
        // words appear as string literals at the sites that use them, which is
        // what tree-sitter's `word:` directive needs to extract them.
        ("token", "an enumeration of the other token productions"),
        (
            "keyword",
            "spelled inline at each use; see the `word:` directive",
        ),
        // Folded into a single token, because they are pieces of ONE lexeme and
        // tree-sitter skips `extras` between tokens but never inside one.
        ("aksara_word", "the body of `identifier`'s regex"),
        ("type_width", "inside the `integer_type` token"),
        ("binary", "inside the `numeral` token"),
        ("octal", "inside the `numeral` token"),
        ("hexadecimal", "inside the `numeral` token"),
        ("decimal", "inside the `numeral` token"),
        ("bin_digit", "a character class inside the `numeral` token"),
        ("oct_digit", "a character class inside the `numeral` token"),
        ("dec_digit", "a character class inside the `numeral` token"),
        ("hex_digit", "a character class inside the `numeral` token"),
        ("string_open", "inside the `string` token"),
        ("string_close", "inside the `string` token"),
        // Names for a single ratified character. Tree-sitter spells those at
        // the site; giving each its own rule would add a node per punctuation
        // mark and change no language.
        ("danda", "the character `।`, spelled at each site"),
        (
            "double_danda",
            "the character `॥`, and no frozen production uses it",
        ),
        ("comment_mark", "inside the `comment` regex"),
        ("member_mark", "the character `ॱ`, spelled at each site"),
        (
            "annotation",
            "the character pair `ॱॱ`, spelled at each site",
        ),
        ("separator", "the character `ऽ`, spelled at each site"),
        // W-226 and W-228 (2026-09-04) froze three productions the EBNF measured
        // from the corpus; grammar.js is "the third description ... WRONG against
        // a frozen grammar" (the EBNF's own unfrozen-note), and wiring a `slice`
        // into its `postfix_expr` or a `routine` into its `statement` choice is
        // that reconciliation row's work, not a text edit nothing generates.
        (
            "slice",
            "W-228: `index , postfix_expr`; grammar.js's postfix_expr predates it and \
             is not regenerated here",
        ),
        (
            "routine",
            "W-226: grammar.js reads a file as `repeat($.statement)` and has no \
             declaration level at all; the reconciliation row adds it",
        ),
        (
            "parameter",
            "W-226: half of `routine`, absent for the same reason",
        ),
        // The one real hole, and it is the EBNF's rather than this file's.
        (
            "embed",
            "frozen by ADR-0019 and wired into NO other production of the EBNF, \
             so there is no position in which a parser could accept one without \
             deciding where embeds may appear — which is not grammar.js's call",
        ),
    ];

    let rules = tree_sitter_rules();
    let productions = ebnf_productions();

    let mut missing = Vec::new();
    for p in &productions {
        if rules.contains(p) || ABSENT.iter().any(|(n, _)| n == p) {
            continue;
        }
        missing.push(p.clone());
    }
    assert!(
        missing.is_empty(),
        "spec/grammar-t1.ebnf defines {missing:?} and crates/tree-sitter-t1/grammar.js has no \
         rule for them. Add the rule, or add the name to ABSENT with the reason."
    );

    // `source_file` is tree-sitter's mandatory start symbol and the EBNF has no
    // start symbol at all — declarations are not frozen (see that file's "what
    // is STILL not frozen"), so `source_file` is `repeat(statement)`. The other
    // three are tree-sitter's own grammar-level directives, which are written
    // with the same `name: $ => …` shape as a rule and so arrive here; they
    // describe how the LEXER is driven, not what the language is.
    const NOT_A_PRODUCTION: &[&str] = &["source_file", "extras", "word", "conflicts"];
    let mut invented = Vec::new();
    for r in &rules {
        if !NOT_A_PRODUCTION.contains(&r.as_str()) && !productions.contains(r) {
            invented.push(r.clone());
        }
    }
    assert!(
        invented.is_empty(),
        "crates/tree-sitter-t1/grammar.js defines {invented:?}, which spec/grammar-t1.ebnf does \
         not. `function_declaration`, `number_literal` and `string_literal` were three such \
         rules until 2026-08-31; that is how a transcription becomes a third opinion."
    );

    // A stale entry is as bad as a missing one: it would let a production be
    // deleted from the EBNF and never noticed here.
    let mut stale = Vec::new();
    for (name, _) in ABSENT {
        if !productions.contains(&name.to_string()) {
            stale.push(*name);
        }
    }
    assert!(
        stale.is_empty(),
        "ABSENT names {stale:?}, which spec/grammar-t1.ebnf no longer defines."
    );

    println!("METRIC t1_tree_sitter_rules {}", rules.len());
}

#[test]
fn the_derived_table_text_reader_is_unsigned_because_its_tables_are() {
    // ADR-0030, task `D-002h`. `encode.t1`'s SECOND part is the nine routines
    // under `reading a derived table's TEXT`, and this pins what made them one
    // unit. The ledger in `the_integer_prefixes_are_the_ones_doc_02_derives`
    // counts the 36 sites the census can see; it cannot say why they moved
    // together, and "the twin says so" is nine separate readings unless the
    // reason is the same one nine times. It is: the TABLES have no signed
    // column, so nothing read out of them can be signed.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");

    // 1. THE NINE SIGNATURES, WHOLE. A signature is where an offset, a count
    //    and a parsed value all three appear, so pinning the line pins every
    //    kind of site in the unit at once.
    for sig in [
        "वृत्तिः अष्टकान्वेषणम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ अवसानम् ॱॱ न६४ अष्टकम् ॱॱ अ८ ददाति न६४",
        "वृत्तिः पङ्क्तिसीमा आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ ददाति न६४",
        "वृत्तिः उपेक्ष्यपङ्क्तिः आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ ददाति बूल",
        "वृत्तिः क्षेत्रसंख्या आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ ददाति न६४",
        "वृत्तिः क्षेत्रारम्भः आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ क्रमः ॱॱ न६४ ददाति न६४",
        "वृत्तिः क्षेत्रसीमा आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ क्रमः ॱॱ न६४ ददाति न६४",
        "वृत्तिः षोडशाङ्कः आदाय अष्टकम् ॱॱ अ८ ददाति न६४",
        "वृत्तिः षोडशाङ्कमूल्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ ददाति सम्भाव्य न६४",
        "वृत्तिः दशाङ्कमूल्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ ददाति सम्भाव्य न६४",
    ] {
        assert!(
            src.contains(sig),
            "`encode.t1` no longer writes `{sig}`. The derived-table text \
             reader is unsigned throughout: every site in it is a byte offset \
             into a table's text, a count of TAB-separated fields, or a value \
             parsed out of one"
        );
    }

    // 2. THE READING IS OFF THE TABLES AND THIS IS THE TWIN THAT CARRIES IT.
    //    Every `parse()` and `from_str_radix` the four readers perform lands
    //    in an unsigned type. If one of these becomes signed, the unit's whole
    //    justification goes with it and the nine must be read again one at a
    //    time.
    for fragment in [
        "pub bias: u32,",
        "u32::from_str_radix(p.get(1)?.trim_start_matches(\"0x\"), 16).ok()?;",
        "pattern: u32::from_str_radix(f[3].trim_start_matches(\"0x\"), 16).ok()?,",
        "mask: u32::from_str_radix(f[4].trim_start_matches(\"0x\"), 16).ok()?,",
        ".then(|| f[2].parse::<u64>().ok())",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. The derived-table \
             readers were respelled as one unit BECAUSE the four `.tsv` tables \
             carry no signed column; a signed parse target here is the fact \
             that would undo it"
        );
    }

    // 3. THE BOUND THIS CHECK CARRIED IS GONE, AND ITS THIRD ITEM WITH IT.
    //    It pinned the consumers the nine producers had left behind: first
    //    `क्षेत्रसाम्यम्` and `क्षेत्रखण्डसाम्यम्` both at `अ६४`, then — once
    //    the REGISTER-TABLE cycle took the first — `क्षेत्रखण्डसाम्यम्`
    //    alone, as the handle for the fence-table cycle. That cycle has run.
    //    The item is DELETED rather than edited, exactly as its own comment
    //    instructed, because there is no consumer of these nine left
    //    disagreeing with them: the reading that replaces it lives in
    //    `the_register_table_is_unsigned_because_a_register_number_is` and
    //    `the_fence_ordering_domain_table_is_unsigned_because_its_table_is`,
    //    and the `न६४` spelling of both readers is pinned there. A file-wide
    //    `sed` is still refused at those two signatures; it is just no longer
    //    refused from here.
}

#[test]
fn the_fence_ordering_domain_table_is_unsigned_because_its_table_is() {
    // ADR-0030, task `D-002h`. `encode.t1`'s FOURTH part is the four routines
    // under `the fence ordering-domain table`, and this pins what made them
    // one unit. The ledger in `the_integer_prefixes_are_the_ones_doc_02_derives`
    // counts 25 of the 27 sites; it cannot say why they moved together.
    //
    // THEY MOVED TOGETHER BECAUSE THE READING WAS ALREADY MADE, TWICE, AND
    // THIS UNIT ONLY HAD TO INHERIT IT. `spec/fence-domains-riscv64.tsv` has
    // ONE numeric column and it is a BIT, parsed by `f[2].parse::<u64>()`,
    // which item 2 of `the_derived_table_text_reader_is_unsigned_because_its_
    // tables_are` already pins as an unsigned target. And
    // `क्षेत्रखण्डसाम्यम्` is `क्षेत्रसाम्यम्` with its `नाम` taken as a
    // RANGE instead of whole — the same comparator, the same table idiom —
    // so `the_register_table_is_unsigned_because_a_register_number_is`'s
    // reading transfers rather than being made again. Everything else here is
    // a byte offset into a table's text or into the caller's own `मूल`.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");

    // 1. THE FOUR SIGNATURES, WHOLE. `क्षेत्रखण्डसाम्यम्` is the one the
    //    derived-table check previously pinned at `अ६४` as the handle for this
    //    cycle; it is pinned here at `न६४` instead, so the same line still
    //    fails a file-wide `sed` — in the other direction now.
    for sig in [
        "वृत्तिः अवग्रहान्वेषणम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ अवसानम् ॱॱ न६४ ददाति न६४",
        "वृत्तिः क्षेत्रखण्डसाम्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ क्रमः ॱॱ न६४ नाम ॱॱ अङ्कः अन्तः अ८ नामादिः ॱॱ न६४ नामान्तः ॱॱ न६४ ददाति बूल",
        "वृत्तिः समूहांशः आदाय नाम ॱॱ अङ्कः अन्तः अ८ नामादिः ॱॱ न६४ नामान्तः ॱॱ न६४ ददाति न६४",
        "वृत्तिः क्षेत्रसमूहः आदाय मूल ॱॱ अङ्कः अन्तः अ८ ददाति सम्भाव्य न६४",
    ] {
        assert!(
            src.contains(sig),
            "`encode.t1` no longer writes `{sig}`. The fence ordering-domain \
             table is unsigned throughout: every site in it is a byte offset \
             into `spec/fence-domains-riscv64.tsv`'s text or into the \
             caller's own base, a TAB-separated field index, or the domain \
             BIT itself"
        );
    }

    // 2. THE TWIN THAT CARRIES THE READING. `domain_set` returns
    //    `Option<u64>`, which is what makes `क्षेत्रसमूहः`'s
    //    `ददाति सम्भाव्य न६४` the port of it and not an approximation of it.
    //    If this return becomes signed the unit's justification goes with it.
    for fragment in [
        "fn domain_set(base: &str) -> Option<u64> {",
        "let mut bits = 0u64;",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. The fence table's \
             four routines were respelled as one unit BECAUSE the domain set \
             is a `u64` of set bits end to end"
        );
    }

    // 3. THE ZERO IS NOT A NEGATIVE SENTINEL, AND THAT IS WHAT KEEPS THIS
    //    UNIT UNSIGNED WHERE A DIFFERENT PORT WOULD HAVE EARNED A SIGN.
    //    `समूहांशः` answers ० for *no such domain*, sound only because every
    //    bit in the table is a SET bit; `अवग्रहान्वेषणम्` answers `अवसानम्`
    //    for *no separator*, a one-past-the-end offset. Neither is -१, which
    //    is `a_negative_sentinel_is_what_keeps_a_site_signed`'s rule applied
    //    to the two routines here that could have broken it. The `०` return
    //    is pinned by its own line so a rewrite to a sentinel cannot pass.
    assert!(
        src.contains("॰ the bit of ONE named ordering domain, or ० when the table carries no"),
        "`encode.t1`'s `समूहांशः` no longer states that ० is its \
         not-found answer. The routine is unsigned because it does not \
         reach for -१, and the claim has to be readable next to the code"
    );

    // 4. THE ० IS SOUND ONLY IF THE TABLE SAYS SO, AND UNTIL THIS CYCLE
    //    NOTHING CHECKED THAT IT DID. `encode.t1` states, above `समूहांशः`,
    //    that "every bit in spec/fence-domains-riscv64.tsv is a set bit, so
    //    no domain answers ०" and cites `tests/t1_embed.rs` as where the
    //    invariant is asserted "where a test can see it". THAT TEST DOES NOT
    //    ASSERT IT — `crates/sadhana/tests/t1_embed.rs` names neither the
    //    table nor `क्षेत्रसमूहः` — so the one fact holding up a not-found
    //    sentinel was an unchecked claim in a comment. It is checked here,
    //    and the comment is corrected to point at this test instead. A row
    //    with bit ० would make both the port and its twin report a real
    //    ordering domain as unknown, silently.
    let table = std::fs::read_to_string(root().join("spec/fence-domains-riscv64.tsv"))
        .expect("read spec/fence-domains-riscv64.tsv");
    let mut domains = 0usize;
    for line in table.lines() {
        if line.starts_with('#') || line.starts_with("devanagari\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        assert!(
            f.len() >= 3,
            "`fence-domains-riscv64.tsv` row has no bit: {line}"
        );
        let bit: u64 = f[2]
            .parse()
            .unwrap_or_else(|_| panic!("`fence-domains-riscv64.tsv` bit is not a u64: {line}"));
        assert_ne!(
            bit, 0,
            "`{}` is assigned bit 0 in `spec/fence-domains-riscv64.tsv`. \
             `समूहांशः` and `domain_set` both answer 0 for *no such domain*, \
             so this row would be reported as unknown by a lookup that found \
             it — silently, producing a valid fence that orders the wrong \
             operations. Either the bit is wrong or both readers need a real \
             not-found value",
            f[0]
        );
        domains += 1;
    }
    assert!(
        domains >= 4,
        "`spec/fence-domains-riscv64.tsv` now carries {domains} domains; the \
         check above is vacuous if the table empties"
    );

    // 5. AND THE CROSS-MODULE END THE LEDGER CANNOT SEE. The register-table
    //    cycle found `samyojana.t1` calling `सङ्केतन ॱ क्षेत्रसाम्यम्` with
    //    `न६४` arguments into `अ६४` parameters — a call site is not a type
    //    position, so no census counts either end. These four have NO caller
    //    outside this file: `क्षेत्रसमूहः` is reached from Rust through
    //    `t1/anita.rs`'s `domain_set` mapping and the other three only from
    //    within `encode.t1`. Pinned as an absence, because that is the shape
    //    the defect took last time.
    for module in ["samyojana", "nidana", "utsarjana", "vishlesana", "artha"] {
        let other =
            std::fs::read_to_string(root().join(format!("crates/sadhana-t1/src/{module}.t1")))
                .unwrap_or_default();
        for name in ["अवग्रहान्वेषणम्", "क्षेत्रखण्डसाम्यम्", "समूहांशः", "क्षेत्रसमूहः"]
        {
            let call = format!("सङ्केतन ॱ {name}");
            assert!(
                !other.contains(&call),
                "`{module}.t1` now calls `{call}`. The fence table was \
                 respelled `न६४` with no cross-module caller to move with \
                 it; a new one has to be read at BOTH ends, because the \
                 ledger counts neither"
            );
        }
    }
}

#[test]
fn the_register_table_is_unsigned_because_a_register_number_is() {
    // ADR-0030, task `D-002h`. `encode.t1`'s THIRD part is the four routines
    // under `the register table`, and this pins what made them one unit. The
    // ledger in `the_integer_prefixes_are_the_ones_doc_02_derives` counts the
    // 19 sites; it cannot say why they moved together. They moved together
    // because ONE declaration in the twin settles all four: a register number
    // is a `u32`, and everything else in the unit is an offset into the table
    // whose number it is.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");

    // 1. THE FOUR SIGNATURES, WHOLE. `क्षेत्रसाम्यम्` is the one this file
    //    previously pinned at `अ६४` as the handle for this cycle; it is
    //    pinned here at `न६४` instead, so the same line still fails a
    //    file-wide `sed` — in the other direction now.
    for sig in [
        "वृत्तिः क्षेत्रसाम्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ क्रमः ॱॱ न६४ नाम ॱॱ अङ्कः अन्तः अ८ ददाति बूल",
        "वृत्तिः कोष्ठपङ्क्तिः आदाय नाम ॱॱ अङ्कः अन्तः अ८ ददाति न६४",
        "वृत्तिः कोष्ठाङ्कः आदाय नाम ॱॱ अङ्कः अन्तः अ८ ददाति सम्भाव्य अ३२",
        "वृत्तिः कोष्ठप्लवः आदाय नाम ॱॱ अङ्कः अन्तः अ८ ददाति बूल",
    ] {
        assert!(
            src.contains(sig),
            "`encode.t1` no longer writes `{sig}`. The register table is \
             unsigned throughout: every site in it is a byte offset into \
             `spec/registers-riscv64.tsv`'s text, a TAB-separated field index, \
             a loop counter over a name's octets, or the register number \
             itself"
        );
    }

    // 2. AND THE TWIN THAT CARRIES THE ONE SITE THE TABLES DO NOT SETTLE.
    //    `f[2].parse()` in `register` lands in the `u32` of this return type
    //    and in no other. If that becomes signed, the unit's justification
    //    goes with it and the four must be read again one at a time.
    assert!(
        twin.contains("pub fn register(name: &str) -> Option<(u32, bool)>"),
        "`encode.rs`'s `register` no longer returns `Option<(u32, bool)>`, so \
         the register NUMBER — the only site in this unit not already settled \
         by the table having no signed column — has lost the twin it was read \
         off"
    );

    // 3. THE ONE-BASED ANSWER IS THE ALTERNATIVE TO A NEGATIVE SENTINEL AND
    //    IS WHY THIS FILE IS THE EASIEST ONE IN THE CORPUS TO HAVE GOT WRONG.
    //    `शून्यः` IS register ०, so a raw number reports the most-used
    //    register in the ISA as an unknown name; the port answers ० for
    //    ABSENT and shifts the number up by one instead of answering -१.
    //    `a_negative_sentinel_is_what_keeps_a_site_signed` states the rule;
    //    this is the site where obeying it was a decision.
    assert!(
        src.contains("प्रत्यागमनम् आरभ्य दशाङ्कमूल्यम् पाठ्यम् क्षेत्रादिः क्षेत्रान्तः समाप्तम् योगः १ ।"),
        "`कोष्ठाङ्कः` no longer answers the parsed register number PLUS ONE. \
         Register ० is `शून्यः` and is real, so the ०-means-absent convention \
         this port uses everywhere costs one addition here — and dropping it \
         is how the answer would come to need a sign"
    );

    // 4. AND THE BOUND: THE CALLER ACROSS THE MODULE BOUNDARY NOW AGREES.
    //    `samyojana.t1`'s `भेदपङ्क्तिः` walks its own table with the same nine
    //    producers and hands `क्षेत्रसाम्यम्` `न६४` arguments. Before this
    //    unit those met `अ६४` parameters — a disagreement the census cannot
    //    see, because both spellings are counted and neither is a call site.
    let caller = std::fs::read_to_string(root().join("crates/sadhana-t1/src/samyojana.t1"))
        .expect("read crates/sadhana-t1/src/samyojana.t1");
    assert!(
        caller.contains("चरः आरम्भः ॱॱ न६४ भवति ०")
            && caller.contains("सङ्केतन ॱ क्षेत्रसाम्यम् पाठ्यम् आरम्भः सीमा ० नाम"),
        "`samyojana.t1`'s `भेदपङ्क्तिः` no longer hands `क्षेत्रसाम्यम्` an \
         `न६४` आरम्भः. This unit was taken partly to close that cross-module \
         disagreement; the call is what shows it closed"
    );
}

#[test]
fn the_address_half_is_signed_by_its_consumer_and_not_by_being_a_difference() {
    // ADR-0030 and ADR-0031, task `D-002h`. `encode.t1`'s SIGNED HALF is the
    // eleven routines that held all 18 of its `इ६४`, and this pins the four
    // decisions in it that a file-wide `sed` would silently undo. The ledger in
    // `the_integer_prefixes_are_the_ones_doc_02_derives` counts the sites; it
    // cannot say WHY any one of them is spelled as it is.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");

    // 1. THE PAIRS KEEP BOTH MEMBERS AND THE MEMBERS DISAGREE. ADR-0031's rule
    //    is that a routine needing the other signedness of a quantity takes
    //    BOTH readings as parameters, because the caller is the only place both
    //    exist. Respelling either member to match the other is what makes a
    //    pair read as a duplicated argument.
    for pair in [
        "मूल्यम् ॱॱ न६४ चिह्नितम् ॱॱ अ६४",
        "मूल्यानि ॱॱ अङ्कः अन्तः न६४ चिह्नितानि ॱॱ अङ्कः अन्तः अ६४",
    ] {
        assert!(
            src.contains(pair),
            "`encode.t1` no longer writes the ADR-0031 pair `{pair}`. \
             मूल्यानि and चिह्नितानि are the SAME numbers twice, unsigned and \
             signed: अन्तर्भावः needs both and T1 has no cast. Two members \
             spelled alike are not a pair"
        );
    }

    // 2. `स्थापनम्`'s VALUE IS A BIT PATTERN AND A PATTERN HAS NO SIGN. The
    //    twin reads the word with `>>` and `& 1` and with nothing else.
    assert!(
        src.contains("वृत्तिः स्थापनम् आदाय अवकाशः ॱॱ अवकाश मूल्यम् ॱॱ न६४ ददाति अ३२"),
        "`स्थापनम्`'s मूल्यम् is `Slot::place(value: u64)` and is `न६४`. It is \
         also the boundary `samyojana.t1`'s पदपुनर्निर्माणम् meets with a \
         signed argument — annotated at both ends, and respelling this to \
         `अ६४` would hide it again"
    );
    assert!(
        twin.contains("pub fn place(&self, value: u64) -> u32"),
        "`encode.rs`'s `place` no longer takes `value: u64`, so the reading \
         above has lost the twin it was read off"
    );

    // 3. THE ADDRESS HALF IS THE ONE `.loop/STATE.md` PRE-JUDGED WRONG.
    //    स्थानाधारः is Rust's `pc`/`base`, u32 on both sides — not a
    //    difference. It stays signed because T1 has no `न३२` in which
    //    `wrapping_sub` at 2^32 could be spelled, and because its only
    //    consumer widens with `i64::from` before subtracting.
    assert!(
        src.contains("वृत्तिः स्थानाधारः आदाय क्रमसूचकः ॱॱ अ६४ उपरि ॱॱ बूल ददाति अ६४"),
        "`स्थानाधारः` is signed, and the reason is at the routine: it is an \
         ADDRESS whose consumer subtracts, not a difference"
    );
    for fragment in [
        "let mut pc = 0u32;",
        "let base = if upper { pc } else { pc.wrapping_sub(4) };",
        "let offset = i64::from(*target) - i64::from(base);",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. The unsigned `pc` and \
             the `i64::from` that widens it are the whole of why स्थानाधारः is \
             signed by its consumer rather than by its own shape"
        );
    }

    // 4. THE BOUND IS PART OF THE CLAIM. This cycle took the signed half and
    //    NOT the file: the enum discriminants inside those same eleven
    //    routines stay `अ६४` because their siblings sit outside the unit, and
    //    reading `encode.t1` as remediated is the mistake that would follow
    //    from not saying so.
    //    THE THIRD ENTRY OF THIS LIST IS GONE, DELETED AND NOT EDITED, AS
    //    ITS OWN MESSAGE INSTRUCTED. It was `चरः गन्तृक्रमः ॱॱ अ६४`, held
    //    because its producer `आवरणावकाशः` sat outside the signed half. The
    //    encoding-accessor unit took that producer and took this binding with
    //    it in the same cycle, so the deferral is discharged rather than
    //    still standing.
    //    AND SO IS THE FIRST ENTRY, DELETED THE SAME WAY. It was
    //    `भेदाः ॱॱ अङ्कः अन्तः अ६४`, the slot KIND, held because its siblings
    //    sat outside the unit; the slot-kind unit took the whole family —
    //    the seven constants, `अवकाश ॱ भेद`, the three predicates and both
    //    `भेदाः` slices — and this binding with it.
    //    AND THE LAST ENTRY IS GONE THE SAME WAY, SO THIS LIST IS NOW EMPTY.
    //    It was `सम्बन्धः ॱॱ अ६४`, the compression relation, held since cycle
    //    524 because its six siblings sat inside `choosing a compressed form`
    //    and no unit had crossed that divider. The compression-relation unit
    //    crossed it and took the family whole — the six constants and this
    //    parameter together — so the entry is DELETED and not edited, exactly
    //    as its own message instructed, and with it the single-element `let`
    //    that clippy's `single_element_loop` had already forced out of a
    //    `for`. WHY THAT ONE LINE IS WORTH A PARAGRAPH IN ITS OBITUARY: it
    //    held the whole headless loop for six hours on 2026-09-02. The cycle
    //    that wrote it verified its TESTS (grammar_t1 31 passed) but the
    //    pre-commit hook runs `clippy -D warnings`, so the commit was refused,
    //    so the worktree stayed dirty, so every later cycle refused on the
    //    dirty tree and could not run the salvage that would have committed
    //    it. Ten fires against the same state, over a style lint that nothing
    //    inside the loop could break.
    //    WHAT REPLACES IT IS NOT ANOTHER DEFERRAL BUT A GUARD IN ITS OWN
    //    RIGHT: `the_compression_relation_is_unsigned_and_its_zero_is_a_\
    //    reserved_value` states the reading the respelling rests on. IF A
    //    FUTURE UNIT DEFERS A SITE AGAIN, restore the `for` here rather than
    //    adding a second binding.

    // AND THE COROLLARY THAT OUTLIVES THIS FILE: no `.t1` source spells a type
    // `इ६४` any more, anywhere, in code. ADR-0030 refused the prefix — `इ` is
    // already frozen as hexadecimal `c` at `grammar-t1.ebnf:243`, so `०षोड्इ६४`
    // is a legal literal and one sign cannot have two senses.
    let dir = root().join("crates/sadhana-t1/src");
    let mut offenders: Vec<String> = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read crates/sadhana-t1/src")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    files.sort();
    for f in &files {
        let text = std::fs::read_to_string(f).expect("read a `.t1` source");
        for (n, line) in text.lines().enumerate() {
            // `॰` opens a comment to end of line; the repair is DESCRIBED in
            // several of them and those mentions are the record, not a site.
            if line.split('\u{0970}').next().unwrap_or("").contains("इ६४") {
                offenders.push(format!(
                    "{}:{}",
                    f.file_name().unwrap_or_default().to_string_lossy(),
                    n + 1
                ));
            }
        }
    }
    // RATCHET, NOT ZERO, SINCE THE MERGE OF 2026-09-03. On the rail's corpus
    // this was empty; the merged tree carries this branch's newer code, which
    // still spells `इ६४` on 39 code lines — `encode.t1` 8, `samyojana.t1` 26,
    // `utsarjana.t1` 4, `vastu.t1` 1 — measured by this test's own walk. Each
    // is ADR-0030 debt to be read site by site, not renamed. The number may
    // only go DOWN; going up means a new site spelled with the refused prefix.
    assert_eq!(
        offenders.len(),
        39,
        "`इ६४` is written in CODE at {offenders:?} — {} lines, not 39. The \
         signed half of ADR-0030 closed at 0 on the rail's corpus; a merge on \
         2026-09-03 brought 39 back in newer code, and this ratchet may only fall",
        offenders.len()
    );
}

#[test]
fn the_encoding_accessors_are_unsigned_because_a_length_is_the_not_found_answer() {
    // ADR-0030, task `D-002h`. `encode.t1`'s FIFTH part is the five routines
    // under `asking an encoding about its slots`, and this pins what made them
    // one unit. The ledger in
    // `the_integer_prefixes_are_the_ones_doc_02_derives` counts all 19 of the
    // sites; it cannot say why they moved together.
    //
    // THEY MOVED TOGETHER BECAUSE THEY ALL ANSWER *NOT FOUND* WITH A LENGTH.
    // The twin returns `Option<&Slot>` or a `usize` count; T1 has no
    // reference to return, so each port answers with the index ONE PAST THE
    // LAST SLOT where Rust answers `None`. That is `अवग्रहान्वेषणम्`'s
    // `अवसानम्` shape, and `a_negative_sentinel_is_what_keeps_a_site_signed`
    // decides it: a length is not -१, so no site here earns a sign.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");

    // 1. THE FIVE SIGNATURES, WHOLE. `नियततत्कालावकाशः`'s `क्रमः` is an
    //    ARGUMENT and moves with the rest: it is the twin's `n: usize` handed
    //    to `Iterator::nth`, which cannot be negative and is not an index into
    //    anything the caller owns.
    for sig in [
        "वृत्तिः आवरणावकाशः आदाय सङ्केतः ॱॱ सङ्केत आवरणम् ॱॱ अ३२ ददाति न६४",
        "वृत्तिः नियततत्कालावकाशः आदाय सङ्केतः ॱॱ सङ्केत क्रमः ॱॱ न६४ ददाति न६४",
        "वृत्तिः तत्कालावकाशः आदाय सङ्केतः ॱॱ सङ्केत ददाति न६४",
        "वृत्तिः कोष्ठसंख्यानम् आदाय सङ्केतः ॱॱ सङ्केत ददाति न६४",
        "वृत्तिः अग्रिमतत्कालावकाशः आदाय सङ्केतः ॱॱ सङ्केत पूरितानि ॱॱ अङ्कः अन्तः अ३२ ददाति न६४",
    ] {
        assert!(
            src.contains(sig),
            "`encode.t1` no longer writes `{sig}`. The encoding accessors are \
             unsigned throughout: every site in them is an index into \
             `अवकाशाः`, an index into `पूरितानि`, a running count, a length, \
             or the `n` of `Iterator::nth`"
        );
    }

    // 2. THE TWIN THAT CARRIES THE READING. Four `Option<&Slot>` returns and
    //    one `usize`. If any of these became a signed return the unit's
    //    justification would go with it, because the port's `दैर्घ्य` answer
    //    is a STAND-IN for `None` and not a computed quantity.
    for fragment in [
        "fn slot_with(&self, mask: u32) -> Option<&Slot> {",
        "fn immediate(&self) -> Option<&Slot> {",
        "fn nth_immediate(&self, n: usize) -> Option<&Slot> {",
        "fn register_count(&self) -> usize {",
        "fn next_immediate<'e>(enc: &'e Encoding, filled: &[(u32, &str)]) -> Option<&'e Slot> {",
        ".count()",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. The five accessors \
             were respelled as one unit BECAUSE the twin answers `None` or a \
             `usize` count at every one of them, never a negative number"
        );
    }

    // 3. THE LENGTH IS THE NOT-FOUND ANSWER, AND IT IS RETURNED AND NOT
    //    MERELY BOUNDED. Three of the five end on `प्रत्यागमनम् दैर्घ्य`
    //    after the walk falls through — the same statement that is the loop
    //    bound is also the failure answer. A rewrite to `-१` would have to
    //    delete this line, and then it fails here rather than only in the
    //    ledger, where it would look like a count that drifted.
    // **COUNTED PER ROUTINE, NOT PER FILE — AND THE FILE-WIDE COUNT WAS WRONG
    // ABOUT ITS OWN POPULATION.** This read
    // `src.matches("\n    प्रत्यागमनम् दैर्घ्य ।").count()` and pinned it at 4.
    // It went to 6 when `पाठमुद्रणम्` (encode.t1:2168) and `विन्यासमुद्रणम्`
    // (:2180) were added, and NEITHER is an accessor: in both, `दैर्घ्य` is a
    // local holding the INPUT's length and the routine answers how many items
    // it WROTE. Same eight characters, unrelated meaning.
    //
    // Raising the pin to 6 would have been the obvious repair and it would have
    // broken the guard. The unit's claim is that the NOT-FOUND ANSWER of these
    // specific accessors is a length rather than a negative sentinel; a total
    // that mixes in emitters passes just as happily when one accessor switches
    // to `-१` and some unrelated routine gains the line. The number would stay
    // right while the property it stands for was gone.
    //
    // So each accessor is asserted BY NAME. A rewrite to a sentinel now fails
    // here saying WHICH routine changed, which is what the paragraph above
    // always said it wanted — "it fails here rather than only in the ledger,
    // where it would look like a count that drifted".
    let accessor_body = |name: &str| -> String {
        let head = format!("वृत्तिः {name} आदाय");
        let start = src
            .find(&head)
            .unwrap_or_else(|| panic!("`encode.t1` no longer declares `{name}`"));
        let rest = &src[start..];
        let end = rest
            .find("\nइति")
            .unwrap_or_else(|| panic!("`{name}` has no closing `इति`"));
        rest[..end].to_string()
    };
    // The four whose fall-through answer IS the length. Three are the walk
    // accessors; `अवकाशसंख्या` joined them in the merge of 2026-09-03,
    // answering the slot count in the same shape.
    for name in [
        "आवरणावकाशः",
        "नियततत्कालावकाशः",
        "अग्रिमतत्कालावकाशः",
        "अवकाशसंख्या",
    ] {
        assert!(
            accessor_body(name).contains("\n    प्रत्यागमनम् दैर्घ्य ।"),
            "`{name}` no longer returns `दैर्घ्य` — the index one past the \
             last slot — as its fall-through answer. That return IS the port \
             of Rust's `None` and is the whole reason these sites are \
             unsigned; if it now answers with a sentinel instead, the reading \
             behind this unit has changed"
        );
    }

    // 4. THE CONSUMER WAS TAKEN WITH THE PRODUCER, WHICH IS WHAT DISCHARGES
    //    CYCLE 524's DEFERRAL RATHER THAN MOVING IT ALONG. `सङ्कोचनिषेधः`'s
    //    `गन्तृक्रमः` is filled from `आवरणावकाशः` and was held at `अ६४` under
    //    the rule *an index whose producer is outside the unit keeps its
    //    producer's spelling until the producer moves*. Item 4 of
    //    `the_signed_half_of_encode_t1_is_what_adr_0031_names` no longer
    //    names it; this is where it is named instead, at `न६४`.
    assert!(
        src.contains("चरः गन्तृक्रमः ॱॱ न६४ भवति आवरणावकाशः सङ्केतः गन्तृक्षेत्र ।"),
        "`encode.t1`'s `सङ्कोचनिषेधः` no longer binds `गन्तृक्रमः` at `न६४` \
         from `आवरणावकाशः`. Producer and consumer moved in the SAME cycle, \
         which is the one arrangement that leaves no disagreement behind"
    );

    // 5. AND THE BOUND IS PART OF THE CLAIM. This unit is not the file: the
    //    head declarations, the slot-kind predicates, `placing a value into a
    //    field`, bytes-and-words and diagnostics still held `अ६४` when this
    //    was written, and reading `encode.t1` as remediated is the mistake
    //    that follows from not saying so. The last two of those five have
    //    since been taken, by
    //    `bytes_and_words_and_diagnostics_borrow_their_spelling_from_a_hidden_usize`;
    //    the first three still stand and are what the two names below pin.
    //    `पदार्थभेदः` is a head declaration and `बहुशब्दः` is a slot-kind
    //    predicate's answer; both are outside this cycle.
    for outside in ["तत्कालवाचकः", "कोष्ठवाचकः"] {
        assert!(
            src.contains(outside),
            "`encode.t1` no longer declares `{outside}`. The slot-kind \
             predicates are the section this unit borders and does not take; \
             if they have been renamed the border stated above is stale"
        );
    }
    assert!(
        src.contains("॰ ══════════════════ slot kind predicates ══════════════════"),
        "`encode.t1` lost the `slot kind predicates` divider. The parts of \
         this file are bounded BY those dividers, and each cycle's account \
         names the ones it did not take"
    );
}

#[test]
fn bytes_and_words_and_diagnostics_borrow_their_spelling_from_a_hidden_usize() {
    // ADR-0030, task `D-002h`. `encode.t1`'s SIXTH bounded part: the two
    // smallest sections left, `bytes and words` and `diagnostics`, taken as
    // ONE unit because they share one reading. The ledger in
    // `the_integer_prefixes_are_the_ones_doc_02_derives` counts the seven
    // sites; it cannot say WHY any of them is spelled `न६४`.
    //
    // THE READING IS *THE TWIN HAS NO NUMBER HERE.* Every earlier part of this
    // file was read off a type `encode.rs` DECLARES — `value: u64`, `pc: u32`,
    // `Option<&Slot>`. These four routines have no such type to read, because
    // the twin reaches for `str::strip_suffix`, `slice::chunks_exact` and
    // `Vec::len` and those three spell their own arithmetic. T1 has none of
    // them, so the port writes the walk out by hand — and what it writes is a
    // length, an index or a count, which is `usize` wherever `core` names it.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");
    let nidana = std::fs::read_to_string(root().join("crates/sadhana-t1/src/nidana.t1"))
        .expect("read crates/sadhana-t1/src/nidana.t1");

    // 1. THE FOUR SIGNATURES. Spelled out rather than counted, so that moving
    //    a site back leaves a named failure and not only a drifted total.
    for signature in [
        "वृत्तिः चतुरष्टकम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ स्थानम् ॱॱ न६४ ददाति अ३२",
        "वृत्तिः चतुरष्टकसंख्या आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ ददाति न६४",
        "वृत्तिः सङ्केतनदोषवचनम् आदाय सङ्केतनदोषः ॱॱ सङ्केतनदोष भाषा ॱॱ न६४ ददाति पाठ",
        "वृत्तिः प्रत्ययवत् आदाय मूल ॱॱ अङ्कः अन्तः अ८ प्रत्ययः ॱॱ अङ्कः अन्तः अ८ ददाति बूल",
    ] {
        assert!(
            src.contains(signature),
            "`encode.t1` no longer writes `{signature}`. The bytes-and-words \
             and diagnostics unit is these four routines; a site put back to \
             `अ६४` fails here as well as in the ledger"
        );
    }

    // 2. AND THE TWO TWINS NAME NO INTEGER AT ALL, which is the whole of the
    //    reading and the one thing a reader would otherwise have to take on
    //    trust. If either grows an explicit type the reading must be redone.
    for fragment in [
        "fn split_address_part(base: &str) -> Option<(&str, bool)> {",
        "if let Some(l) = base.strip_suffix(\"ॱउपरि\") {",
        "pub fn words(text: &[u8]) -> Vec<u32> {",
        "text.chunks_exact(4)",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. The port's numbers \
             here are the ones `strip_suffix` and `chunks_exact` spell for it; \
             if the twin now names a type of its own, read the sites off THAT \
             instead of off this reasoning"
        );
    }

    // 3. THE ONE SUBTRACTION IS GUARDED, WHICH IS WHY IT IS NOT A DIFFERENCE.
    //    `a_negative_sentinel_is_what_keeps_a_site_signed` asks for a negative
    //    the file could produce. `आरम्भः` is `मूलदैर्घ्य − प्रत्ययदैर्घ्य` and
    //    the comparison two lines above has already returned असत्यम् on the
    //    only case that could go below ०. The guard and the subtraction are
    //    pinned TOGETHER: delete the guard and the spelling loses its reason.
    let walk = src
        .split_once("वृत्तिः प्रत्ययवत् आदाय")
        .expect("`प्रत्ययवत्` is declared")
        .1;
    let body = walk.split_once("\nइति").expect("`प्रत्ययवत्` is closed").0;
    assert!(
        body.contains("यदि मूलदैर्घ्य न्यूनम् प्रत्ययदैर्घ्य आदि")
            && body.contains("चरः आरम्भः ॱॱ न६४ भवति मूलदैर्घ्य वियोगः प्रत्ययदैर्घ्य ।"),
        "`प्रत्ययवत्` no longer guards its subtraction with \
         `यदि मूलदैर्घ्य न्यूनम् प्रत्ययदैर्घ्य`. An unguarded \
         `मूलदैर्घ्य वियोगः प्रत्ययदैर्घ्य` CAN go negative, and then the \
         `न६४` spelling of `आरम्भः` is wrong rather than merely unexplained"
    );

    // 4. THE SEVENTH SITE IS AN ENUM DISCRIMINANT WHOSE SIBLINGS HAD ALREADY
    //    MOVED, IN ANOTHER FILE. This is what distinguishes it from `भेदाः`
    //    and `सम्बन्धः`, which item 4 of
    //    `the_address_half_is_signed_by_its_consumer_and_not_by_being_a_difference`
    //    still holds at `अ६४`: those two have siblings inside `encode.t1` that
    //    no unit has taken, and `भाषा`'s siblings are in `nidana.t1` and are
    //    `न६४`. So this was a live cross-module disagreement, not a deferral.
    for spelling in [
        "सार्वजनिक चरः संस्कृतभाषा ॱॱ न६४ भवति १ ।",
        "सार्वजनिक चरः आङ्ग्लभाषा ॱॱ न६४ भवति २ ।",
        "भाषा ॱॱ न६४ ददाति अङ्कः अन्तः अ८",
    ] {
        assert!(
            nidana.contains(spelling),
            "`nidana.t1` no longer writes `{spelling}`. `encode.t1`'s \
             `सङ्केतनदोषवचनम्` is spelled `न६४` BECAUSE `Language`'s own port \
             already is; if that changed, the two modules disagree again"
        );
    }
    assert!(
        twin.contains("pub fn message(&self, lang: Language) -> String {"),
        "`encode.rs`'s `message` no longer takes `lang: Language`, so the \
         discriminant this cycle moved has lost the twin it was read off"
    );

    // 5. AND THE BOUND IS PART OF THE CLAIM, MEASURED PER DIVIDER RATHER THAN
    //    LISTED. 64 `अ६४` remain in `encode.t1` across nine sections, and the
    //    largest of them is `laying out and emitting` with seventeen — NOT the
    //    head declarations, which the five-section candidate list in
    //    `.loop/STATE.md` had called the largest single piece left. That list
    //    accounts for 30 of the 64 and never named four of the nine sections.
    //    The two dividers below are the ones this unit is bounded BY.
    for divider in [
        "॰ ══════════════════ bytes and words ══════════════════",
        "॰ ══════════════════ diagnostics ══════════════════",
        "॰ ══════════════════ laying out and emitting ══════════════════",
    ] {
        assert!(
            src.contains(divider),
            "`encode.t1` lost the `{divider}` divider. The parts of this file \
             are bounded BY those dividers, and each cycle's account names \
             the ones it did not take"
        );
    }

    // 6. THE STALE COMMENT THIS UNIT CORRECTED, PINNED SO IT CANNOT COME BACK.
    //    `प्रत्ययवत्`'s header claimed `स्थानभागः` rests on it. It does not —
    //    it rests on `स्थानप्रत्ययवत्`, which walks the octets
    //    `स्थानप्रत्ययाष्टकम्` supplies — and `प्रत्ययवत्` has no caller in
    //    the corpus at all. A reader who believed the old comment would infer
    //    a consumer, and then a spelling from the consumer.
    assert!(
        src.contains("प्रत्यागमनम् स्थानप्रत्ययवत् मूल सत्यम् ।")
            && src.contains("यदि स्थानप्रत्ययवत् मूल सत्यम् आदि"),
        "`स्थानभागः`/`स्थानभागोपरिः` no longer call `स्थानप्रत्ययवत्`. If the \
         address half has gone back to the general `प्रत्ययवत्`, the comment \
         this cycle corrected was right after all and should be restored"
    );
    // `॰` opens a comment to end of line, and the account at the head of the
    // file names this routine several times; only CODE occurrences count.
    let code: String = src
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let callers = code.matches("प्रत्ययवत्").count() - code.matches("स्थानप्रत्ययवत्").count();
    assert_eq!(
        callers, 1,
        "`प्रत्ययवत्` is written {callers} times outside `स्थानप्रत्ययवत्`, \
         not once. One occurrence is its own declaration and no other: it is \
         the general form kept beside the specialised one, with NO caller. A \
         second occurrence means it has acquired one, and a consumer is the \
         one thing that could make its spelling something other than this \
         cycle's reading of `strip_suffix`"
    );
}

#[test]
fn the_slot_kind_is_unsigned_because_a_discriminant_has_no_sign_to_read() {
    // ADR-0030, task `D-002h`. `encode.t1`'s SEVENTH bounded part, and the
    // first bounded by a FAMILY rather than by a divider: the slot kind,
    // taken wherever this file spells it. The ledger in
    // `the_integer_prefixes_are_the_ones_doc_02_derives` counts thirteen of
    // the fifteen sites and cannot count the other two at all; it cannot say
    // WHY any of them is spelled `न६४`.
    //
    // THE READING IS *A DISCRIMINANT IS UNSIGNED, AND HERE THE TWIN HAS NO
    // NUMBER TO READ AT ALL.* Cycle 529 met a kind whose siblings had already
    // moved in another file; this is the same class in its strongest form.
    // `encode.rs` spells the slot kind as a `String` (`:55`) and compares it
    // with `==` (`:72`, `:81`, `:91`, `:152`), so there is no field, no
    // parameter and no return whose sign could be read. The integer exists
    // ONLY because T1 has no string-equality operator, which the constants'
    // own block comment has said since the port was written — so what the
    // port chose is all there is to read, and it reads: seven values numbered
    // from १, no ०, compared and never added to.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");
    let vishlesana = std::fs::read_to_string(root().join("crates/sadhana-t1/src/vishlesana.t1"))
        .expect("read crates/sadhana-t1/src/vishlesana.t1");
    let samyojana = std::fs::read_to_string(root().join("crates/sadhana-t1/src/samyojana.t1"))
        .expect("read crates/sadhana-t1/src/samyojana.t1");

    // 1. THE SEVEN CONSTANTS AND THEIR VALUES TOGETHER. The value is part of
    //    the reading and not decoration: numbering from १ with no ० is what
    //    says a slot always HAS a kind, and a family with no ० and no
    //    arithmetic has nowhere to put a sign.
    for (name, value) in [
        ("कोष्ठभेद", "१"),
        ("प्लवकोष्ठभेद", "२"),
        ("तत्कालभेद", "३"),
        ("सचिह्नतत्कालभेद", "४"),
        ("नामाङ्कभेद", "५"),
        ("स्थिरभेद", "६"),
        ("अन्तरभेद", "७"),
    ] {
        let decl = format!("सार्वजनिक चरः {name} ॱॱ न६४ भवति {value} ।");
        assert!(
            src.contains(&decl),
            "`encode.t1` no longer declares `{decl}`. The slot kinds are \
             `न६४` because they are a discriminant with no ० and no \
             arithmetic; a respelling, a renumbering from ० or a gap in the \
             run all break that reading and each has to be re-argued rather \
             than inherited"
        );
    }

    // 2. THE FIELD THAT HOLDS ONE AND THE FOUR PARAMETERS THAT READ ONE.
    //    Spelled out rather than counted, so moving a site back leaves a
    //    named failure and not only a drifted total.
    for signature in [
        "  भेद ॱॱ न६४ ऽ",
        "वृत्तिः कोष्ठवाचकः आदाय भेद ॱॱ न६४ ददाति बूल",
        "वृत्तिः तत्कालवाचकः आदाय भेद ॱॱ न६४ ददाति बूल",
        "वृत्तिः समानभेदः आदाय प्रथम ॱॱ न६४ द्वितीय ॱॱ न६४ ददाति बूल",
    ] {
        assert!(
            src.contains(signature),
            "`encode.t1` no longer writes `{signature}`. `अवकाश ॱ भेद` and \
             the three predicates that read it move with the constants; a \
             predicate taking `अ६४` against `न६४` constants is the \
             disagreement this unit closed"
        );
    }

    // 3. THE TWO SITES THIS TEST EXISTS FOR, BECAUSE THE LEDGER CANNOT SEE
    //    THEM. `corpus_type_positions` counts the token after `ॱॱ`; where a
    //    slice is declared that token is `अङ्कः` and the element type is two
    //    words further on. `vishlesana.t1` hit this first — its
    //    `अवकाशमूल्यानि` and `अवकाशभेदाः` are named in the ledger's own
    //    comment — and a cycle that repaired only what the ledger can see
    //    would leave a slice disagreeing with the element type it is filled
    //    from.
    for slice_site in [
        "वृत्तिः पदरचना आदाय सङ्केतः ॱॱ सङ्केत भेदाः ॱॱ अङ्कः अन्तः न६४",
        "वृत्तिः अपाकर्तव्यम् आदाय भेदाः ॱॱ अङ्कः अन्तः न६४",
        "चरः लिखितभेदः ॱॱ न६४ भवति भेदाः अङ्कः सूचकाङ्क अन्तः ।",
    ] {
        assert!(
            src.contains(slice_site),
            "`encode.t1` no longer writes `{slice_site}`. Two of these three \
             move NO number in the remediation ledger, which is exactly why \
             they are pinned by name here"
        );
    }

    // 4. THE TWIN, WHICH HAS NO NUMBER TO READ. This is the whole reading and
    //    the one thing a later cycle could get wrong by assuming `encode.rs`
    //    declares a kind type it does not.
    for fragment in [
        "pub kind: String,",
        "fn is_register(kind: &str) -> bool {",
        "kind == \"reg\" || kind == \"freg\"",
        "fn is_immediate(kind: &str) -> bool {",
        "kind == \"imm\" || kind == \"simm\"",
        "is_register(a) == is_register(b) && (a == \"disp\") == (b == \"disp\")",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. The slot kind being a \
             STRING in the twin is why this family has no declared sign to \
             read — if the twin has grown a numeric kind type, the reading \
             must be made again off that type and not inherited from here"
        );
    }

    // 5. THE MIRRORS THAT WERE ALREADY `न६४`, IN TWO REMEDIATED MODULES.
    //    Until this cycle the file that OWNS the constants was the one
    //    disagreeing with its own readers — the third cross-module
    //    disagreement the repair has found, after `पदरचना`/
    //    `पदपुनर्निर्माणम्` and `भाषा`/`Language`, and the largest, being a
    //    whole family rather than one parameter.
    for mirror in [
        "वृत्तिः चिह्नितावकाशः आदाय भेद ॱॱ न६४ ददाति बूल",
        "वृत्तिः अवकाशभेदाः आदाय सङ्केतः ॱॱ सङ्केतनॱसङ्केत भेदाः ॱॱ अङ्कः अन्तः न६४",
    ] {
        assert!(
            vishlesana.contains(mirror),
            "`vishlesana.t1` no longer writes `{mirror}`. It spelled the slot \
             kind `न६४` before `encode.t1` did, and the disagreement between \
             them is what this unit closed; if the mirror moved instead, the \
             two are apart again in the other direction"
        );
    }
    assert!(
        samyojana.contains("भेदः ॱॱ न६४ ददाति न६४") && samyojana.contains("सङ्केतनॱतत्कालभेद"),
        "`samyojana.t1`'s `पूरणीयावकाशः` no longer takes an `न६४` slot kind \
         and compares it to `सङ्केतन`'s constants. It is the second of the two \
         remediated readers this file was disagreeing with"
    );

    // 6. THE BORDER THIS UNIT DID NOT CROSS IS GONE, DELETED AND NOT EDITED
    //    AS ITS OWN MESSAGE INSTRUCTED. It asserted that `सम्बन्धः` — the
    //    OTHER discriminant family in this file — was still `अ६४`, because
    //    its six siblings sat inside `choosing a compressed form` and this
    //    unit did not enter that divider. Cycle 531 entered it and took the
    //    family, and the reading that settled `असम्बन्धः`'s ० — which the
    //    deleted message demanded be said out loud — is that ० is `reduce`'s
    //    `_ => None` arm, a value the enumeration RESERVES for a table column
    //    it does not model, where a slot always HAS a kind. A reserved value
    //    is not a negative one. It is stated and pinned in
    //    `the_compression_relation_is_unsigned_and_its_zero_is_a_reserved_value`.
    //    NO DISCRIMINANT FAMILY IS LEFT AT `अ६४` IN THIS FILE.

    // 7. AND THE FAMILIES THAT HAD ALREADY MOVED, WHICH IS WHY THIS ONE WAS
    //    AN OUTLIER RATHER THAN A DECISION. Every other `*भेद` discriminant
    //    family in the corpus is `न६४`; the only one left at `अ६४` is
    //    `vakyavibhaga.t1`'s, and that file is unremediated entire.
    for (file, decl) in [
        ("artha.t1", "सार्वजनिक चरः पूर्णाङ्कार्थभेद ॱॱ न६४ भवति १ ।"),
        ("ast.t1", "सार्वजनिक चरः मूलप्रकारभेद ॱॱ न६४ भवति १ ।"),
        ("ir.t1", "सार्वजनिक चरः ध्रुवाज्ञाभेद ॱॱ न६४ भवति १ ।"),
        ("lex.t1", "सार्वजनिक चरः पदभेद ॱॱ न६४ भवति १ ।"),
        ("parse.t1", "सार्वजनिक चरः वृत्तिघोषणाभेद ॱॱ न६४ भवति १ ।"),
        ("utsarjana.t1", "सार्वजनिक चरः कोष्ठाधिकरणभेद ॱॱ न६४ भवति १ ।"),
    ] {
        let text = std::fs::read_to_string(root().join("crates/sadhana-t1/src").join(file))
            .unwrap_or_else(|_| panic!("read crates/sadhana-t1/src/{file}"));
        assert!(
            text.contains(decl),
            "`{file}` no longer writes `{decl}`. The slot kind was respelled \
             because it was the LAST discriminant family in a remediated file \
             still spelled `अ६४`; if the others have moved back, that premise \
             is gone and this unit's reading needs remaking"
        );
    }
}

#[test]
fn the_compression_relation_is_unsigned_and_its_zero_is_a_reserved_value() {
    // ADR-0030, task `D-002h`. `encode.t1`'s EIGHTH bounded part — the second
    // taken as a FAMILY rather than as a divider, and the family the slot-kind
    // unit named as the border it would not cross: the six constants
    // `असम्बन्धः` … `प्रथमशून्यसम्बन्धः` and the one parameter that reads them,
    // `अपाकर्तव्यम्`'s `सम्बन्धः`. Seven sites, and unlike the slot kind's
    // fifteen the census sees ALL of them — none is wrapped in `अङ्कः अन्तः` —
    // so the file count and the ledger moved together: `encode.t1` 51/135 ->
    // 44/142, corpus ledger 586/0/792 -> 579/0/799, sum still 1378.
    //
    // THIS GUARD IS WRITTEN A CYCLE LATE, AND THAT IS THE SECOND THING IT
    // RECORDS. The unit's CODE landed at `869ce9a0`, finished by an outside
    // hand after the cycle that wrote it died between measuring the ledger and
    // writing the constant; its RECORD never landed at all. Three places in
    // the tree — `encode.t1`'s header, item 4 of
    // `the_signed_half_of_encode_t1_is_what_adr_0031_names` and item 6 of
    // `the_slot_kind_is_unsigned_because_a_discriminant_has_no_sign_to_read` —
    // already said the reading was *stated and pinned in* this test, and this
    // test did not exist. A record that names a check which is not there is
    // worse than no record, because the next reader stops looking.
    //
    // THE READING IS THE SLOT KIND'S, AND THIS IS ITS SECOND WITNESS: *A
    // DISCRIMINANT IS UNSIGNED, AND HERE THE TWIN HAS NO NUMBER TO READ AT
    // ALL.* `reduce` (`encode.rs:858`) takes `relation: &str` and matches it
    // against the five string literals `spec/compression-choices.tsv` writes in
    // its fourth column, with a `_ => None` arm under them. There is no field,
    // no parameter and no return in the twin whose sign could be read. The
    // integer exists ONLY because T1 has no string-equality operator, which
    // this section's own block comment has said since the port was written.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");
    let vishlesana = std::fs::read_to_string(root().join("crates/sadhana/src/vishlesana.rs"))
        .expect("read crates/sadhana/src/vishlesana.rs");

    // 1. THE SIX CONSTANTS WITH THEIR VALUES, BECAUSE THE VALUE IS PART OF THE
    //    READING. `असम्बन्धः भवति ०` is what distinguishes this family from the
    //    slot kinds, which number from १ and have no ० at all: a slot always
    //    HAS a kind, where the relation table is DATA ON DISK and may name a
    //    fourth-column value this reader was not written for. A renumbering
    //    breaks the reading and not only the count.
    for (name, value, gloss) in [
        ("असम्बन्धः", "०", "a relation this reader does not model"),
        ("अप्लवसम्बन्धः", "१", "none"),
        ("प्रथमानुवृत्तिसम्बन्धः", "२", "arg1=arg0"),
        ("द्वितीयशून्यसम्बन्धः", "३", "arg1=zero"),
        ("तृतीयशून्यसम्बन्धः", "४", "arg2=zero"),
        ("प्रथमशून्यसम्बन्धः", "५", "arg0=zero"),
    ] {
        let decl = format!("सार्वजनिक चरः {name} ॱॱ न६४ भवति {value} ।");
        assert!(
            src.contains(&decl),
            "`encode.t1` no longer declares `{decl}` (the `{gloss}` column). \
             The compression relations are `न६४` because they are a \
             discriminant whose twin is a `&str`; a respelling, a renumbering \
             or a gap in the run each break that reading and have to be \
             re-argued rather than inherited"
        );
    }

    // 2. THE ONE PARAMETER THAT READS THEM, AND THE TWO `अ६४` ON THE SAME LINE
    //    THAT ARE CORRECT. `भेदाः` is a slot-kind slice and moved with its own
    //    family last cycle; `मूल्यानि` is `values: &[(&str, i64)]` and STAYS
    //    signed. Pinning the whole signature is what keeps a later cycle from
    //    "finishing" the line.
    let apakartavyam = "सार्वजनिक वृत्तिः अपाकर्तव्यम् आदाय भेदाः ॱॱ अङ्कः अन्तः न६४ \
         मूल्यानि ॱॱ अङ्कः अन्तः अ६४ सम्बन्धः ॱॱ न६४ ददाति सम्भाव्य न६४ आदि";
    assert!(
        src.contains(apakartavyam),
        "`अपाकर्तव्यम्`'s signature changed. It is the ONE consumer of the \
         relation family, and it carries all three spellings the unit had to \
         separate: an `न६४` slot-kind slice, an `अ६४` operand-value slice and \
         the `न६४` relation itself"
    );
    assert!(
        src.contains(
            "सार्वजनिक वृत्तिः सङ्कोचनिषेधः आदाय सङ्केतः ॱॱ सङ्केत मूल्यानि ॱॱ अङ्कः अन्तः अ६४ ददाति बूल आदि"
        ),
        "`सङ्कोचनिषेधः`'s `मूल्यानि` is no longer `अ६४`. It is the second of \
         the two operand-value slices this divider holds, and the census can \
         see NEITHER — a slice declares `अङ्कः` after `ॱॱ` and hides its \
         element type two words on — so a cycle reading this divider as \
         `0 left` from the ledger alone would find them and could only \
         conclude they were missed. They were not"
    );
    assert!(
        vishlesana.contains("pub operands: Vec<(String, i64)>,"),
        "`Decoded::operands` is no longer signed, so the reason both \
         `मूल्यानि` slices stay `अ६४` has lost the twin it was read off"
    );

    // 3. THE TWIN, WHICH IS THE WHOLE ARGUMENT. Five string literals and a
    //    `_ => None`: no integer anywhere for a sign to live in.
    assert!(
        twin.contains(
            "fn reduce<'a>(values: &[(&'a str, i64)], relation: &str) -> Option<Vec<(&'a str, i64)>> {"
        ),
        "`encode.rs`'s `reduce` no longer takes `relation: &str`. If the twin \
         has grown a numeric relation then the port's spelling must be read \
         off THAT instead of off this port's own choice, and this unit's \
         reading is void"
    );
    for arm in [
        "\"arg1=arg0\" => (at(1)? == at(0)? && values[1].0 == values[0].0).then_some(1),",
        "\"arg1=zero\" => (at(1)? == 0 && is_register(values[1].0)).then_some(1),",
        "\"arg2=zero\" => (at(2)? == 0 && is_register(values[2].0)).then_some(2),",
        "\"arg0=zero\" => (at(0)? == 0 && is_register(values[0].0)).then_some(0),",
    ] {
        assert!(
            twin.contains(arm),
            "`encode.rs`'s `reduce` no longer matches `{arm}`. The four named \
             relations are the four constants १-५ minus `none`, and the \
             correspondence is the only thing that fixes their numbering"
        );
    }
    assert!(
        twin.contains(
            "            \"none\" => return Some(values.to_vec()),\n            _ => None,"
        ) || twin.contains("        \"none\" => return Some(values.to_vec()),\n        _ => None,"),
        "`encode.rs`'s `reduce` no longer ends with `\"none\"` above a \
         `_ => None` arm. THAT ARM IS `असम्बन्धः`'s ०: the value the \
         enumeration RESERVES for a fourth column it does not model. A \
         reserved value is not a negative one, which is the sentence this \
         whole test exists to hold"
    );

    // 4. ० IS COMPARED AND NEVER SUBTRACTED FROM, MEASURED RATHER THAN
    //    ASSERTED. Every mention of the parameter in the divider is either its
    //    declaration or a `यदि सम्बन्धः समम् …`; nothing adds to it, negates it
    //    or subtracts from it, which is the other half of *this ० is not a
    //    sign*.
    let divider: Vec<&str> = src
        .lines()
        .skip_while(|l| !l.contains("══ choosing a compressed form ══"))
        .take_while(|l| !l.contains("══ the register table ══"))
        .collect();
    assert!(
        divider.len() > 100,
        "the `choosing a compressed form` divider is {} lines and was 131. \
         Its boundaries are how every count below is scoped",
        divider.len()
    );
    let mut compared = 0;
    for line in &divider {
        let code = line.split('\u{0970}').next().unwrap_or("");
        if !code
            .split_whitespace()
            .any(|t| t.trim_matches('।') == "सम्बन्धः")
        {
            continue;
        }
        if code.contains("वृत्तिः अपाकर्तव्यम्") {
            continue;
        }
        assert!(
            code.trim_start().starts_with("यदि सम्बन्धः समम् "),
            "`{}` reads the relation some way other than comparing it. It is \
             only ever `समम्` against a constant; an addition, a negation or a \
             subtraction appearing here would mean the family has become \
             arithmetic and its spelling has to be re-derived",
            code.trim()
        );
        compared += 1;
    }
    assert_eq!(
        compared, 5,
        "the relation is compared {compared} times, not five — once for each \
         constant except `असम्बन्धः`, whose ० is reached by falling off the end \
         exactly as `reduce`'s `_ => None` is"
    );

    // 5. AND THE DIVIDER IS AT ZERO IN THE CENSUS'S OWN TERMS, WHICH IS WHAT
    //    THE HEADER'S PER-DIVIDER TABLE CLAIMS AND NOTHING HAD CHECKED. The
    //    two `अ६४` above are slice ELEMENT types and invisible to the census by
    //    construction; a visible one appearing here is a regression or an
    //    unremediated addition, and either way is a thing to look at.
    let visible = divider
        .iter()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .flat_map(|c| {
            c.replace("ॱॱ", "\u{1}")
                .replace('ॱ', " ॱ ")
                .replace('\u{1}', " ॱॱ ")
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥').to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<String>>()
        .windows(2)
        .filter(|p| (p[0] == "ॱॱ" || p[0] == "ददाति") && p[1] == "अ६४")
        .count();
    assert_eq!(
        visible, 0,
        "`choosing a compressed form` holds {visible} `अ६४` the ledger can \
         see, not 0. `encode.t1`'s header states this divider as done and the \
         remaining 44 as living in seven OTHER sections; that table is what \
         the next cycle picks its unit from"
    );

    // 6. THE DEFERRAL LIST ITEM 4 CARRIED SINCE CYCLE 524 IS EMPTY, AND THIS
    //    IS THE SUBSTANTIVE FORM OF THAT CLAIM. `सम्बन्धः ॱॱ अ६४` was its last
    //    entry, held under *a kind moves with its siblings* because the six sat
    //    inside a divider no unit had crossed. NO DISCRIMINANT FAMILY IS LEFT
    //    AT `अ६४` IN THIS FILE.
    assert!(
        !src.lines().any(|l| l
            .split('\u{0970}')
            .next()
            .unwrap_or("")
            .contains("सम्बन्धः ॱॱ अ६४")),
        "`encode.t1` spells a `सम्बन्धः` `अ६४` again. That was item 4's last \
         deferral and it was DELETED rather than edited, as its own message \
         instructed; a new one means the entry has to come back rather than \
         this assertion being relaxed"
    );

    // 7. THE FAMILY HAS NO CONSUMER OUTSIDE THIS FILE, AND THAT IS WHY THE
    //    READING COULD BE MADE FROM THE PORT ALONE. Cycle 529's finding was a
    //    CROSS-MODULE disagreement invisible to a per-file reading; the check
    //    that there is no other reader is what makes its absence a measurement
    //    rather than an assumption. The table reader that will map
    //    `spec/compression-choices.tsv`'s fourth column onto these constants is
    //    under `what is NOT ported yet`, and when it arrives it is a consumer
    //    and this assertion is the thing it has to update.
    let dir = root().join("crates/sadhana-t1/src");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read crates/sadhana-t1/src")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .filter(|p| p.file_name().is_some_and(|n| n != "encode.t1"))
        .collect();
    files.sort();
    for f in &files {
        let text = std::fs::read_to_string(f).expect("read a `.t1` source");
        for (n, line) in text.lines().enumerate() {
            let code = line.split('\u{0970}').next().unwrap_or("");
            assert!(
                !code.contains("सम्बन्धः"),
                "{}:{} names a compression relation. The family was read off \
                 `encode.rs`'s `reduce` alone because `encode.t1` is its only \
                 reader; a second one is a spelling to check against, not a \
                 thing to leave unread",
                f.display(),
                n + 1
            );
        }
    }
}

#[test]
fn the_address_half_walks_octets_because_its_twin_walks_a_string() {
    // ADR-0030, task `D-002h`. `encode.t1`'s NINTH bounded part, and the first
    // taken by a divider since cycle 529: `the two halves of an address`, whole.
    // Six sites and the census sees all six — `स्थानप्रत्ययदैर्घ्यम्`'s return,
    // `स्थानप्रत्ययाष्टकम्`'s `क्रमः`, and `स्थानप्रत्ययवत्`'s four bindings
    // `प्रत्ययदैर्घ्य`, `मूलदैर्घ्य`, `आरम्भः` and `सूचकाङ्क`. `encode.t1`
    // carried 44 `अ६४` and 142 `न६४`; it carries 38 and 148 now. Corpus ledger
    // 579/0/799 -> 573/0/805, sum still 1378. THE DIVIDER IS NOW AT 0 and drops
    // off the header's table.
    //
    // THE NEXT-TASK NOTE SAID THIS CYCLE WOULD INHERIT NO HANDLE, AND THAT WAS
    // WRONG: CYCLE 529 LEFT THE LARGEST HANDLE YET AND DID NOT SEE IT. The
    // handle is not a call site and not a family — it is a DUPLICATED ROUTINE.
    // `प्रत्ययवत्` (`bytes and words`, remediated by cycle 529) and
    // `स्थानप्रत्ययवत्` (this divider) are the same walk written twice, with
    // the same four binding names in the same order and the same early return
    // between them; the general one came out `न६४` and the specialised one was
    // left `अ६४`, in the same file, thirty lines apart. Neither reading had to
    // be made afresh — one of them was already made and only half applied.
    //
    // THE READING IS CYCLE 529'S OWN, AND THIS IS ITS SECOND AND LARGER
    // WITNESS: *THE TWIN HAS NO NUMBER HERE, AND THE PORT'S NUMBERS ARE THE
    // ONES THE STANDARD LIBRARY SPELLED FOR IT.* `split_address_part`
    // (`encode.rs:411`) is two `str::strip_suffix` calls and nothing else — no
    // integer is named anywhere in it — so every site in this divider exists
    // because T1 has no string type and had to write the octet walk out by
    // hand. A `str`'s length and an index into it are `usize` in every
    // signature `core` gives them.
    //
    // AND THE ONE SUBTRACTION IN THE DIVIDER IS THE COUNTER-CASE TO THIS
    // FILE'S STANDING RULE, WHICH IS WHY IT IS PINNED BELOW RATHER THAN
    // MENTIONED. *A signed quantity in this port is almost always a
    // DIFFERENCE* has carried four routines to `अ६४`; `आरम्भः भवति मूलदैर्घ्य
    // वियोगः प्रत्ययदैर्घ्य` is a difference too, and it is unsigned, because
    // the routine returns `असत्यम्` two lines earlier when the base is shorter
    // than the suffix. The rule holds where nothing has excluded the negative
    // case; a guarded difference is not a signed one.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");
    let code: String = src
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let devanagari = |n: usize| -> String {
        if n == 0 {
            return "०".to_string();
        }
        let digits: Vec<char> = "०१२३४५६७८९".chars().collect();
        let mut out = Vec::new();
        let mut n = n;
        while n > 0 {
            out.push(digits[n % 10]);
            n /= 10;
        }
        out.iter().rev().collect()
    };

    // 1. THE TWIN NAMES NO INTEGER, WHICH IS THE WHOLE ARGUMENT. If it ever
    //    grows one, the port's spelling has to be read off THAT instead of off
    //    `usize`, and this unit's reading is void.
    assert!(
        twin.contains("fn split_address_part(base: &str) -> Option<(&str, bool)> {")
            && twin.contains("if let Some(l) = base.strip_suffix(\"ॱउपरि\") {")
            && twin.contains("base.strip_suffix(\"ॱअधः\").map(|l| (l, false))"),
        "`encode.rs`'s `split_address_part` is no longer two `strip_suffix` \
         calls over `&str`. Every site in this divider is an octet length or \
         an octet index that exists ONLY because T1 cannot write that call; a \
         twin that names an integer type would settle their spelling instead"
    );

    // 2. THE HANDLE, PINNED AT BOTH ENDS. The general walk and the specialised
    //    one are the same four bindings in the same order, and they now agree.
    //    Pinning both is what stops them drifting apart a second time — the
    //    first time cost a cycle, because a reader of this divider alone could
    //    not see that the reading had already been made next door.
    let routine_body = |name: &str| -> String {
        let head = format!("वृत्तिः {name} आदाय");
        let start = code
            .find(&head)
            .unwrap_or_else(|| panic!("`encode.t1` no longer declares `{name}`"));
        let rest = &code[start..];
        let end = rest
            .find("\nइति")
            .unwrap_or_else(|| panic!("`{name}` has no closing `इति`"));
        rest[..end].to_string()
    };
    for name in ["प्रत्ययवत्", "स्थानप्रत्ययवत्"]
    {
        let body = routine_body(name);
        for binding in ["प्रत्ययदैर्घ्य", "मूलदैर्घ्य", "आरम्भः", "सूचकाङ्क"]
        {
            assert!(
                body.contains(&format!("चरः {binding} ॱॱ न६४ भवति ")),
                "`{name}` no longer binds `{binding}` as `न६४`. The general \
                 walk and the specialised one are the same four bindings in \
                 the same order; cycle 529 spelled only the general one and \
                 this unit spelled the other, so a disagreement between them \
                 is the exact defect that cost a cycle here"
            );
            assert!(
                !body.contains(&format!("चरः {binding} ॱॱ अ६४")),
                "`{name}` binds `{binding}` as `अ६४` again"
            );
        }
    }

    // 3. THE GUARD THAT MAKES THE ONE SUBTRACTION SAFE, IN BOTH COPIES. It is
    //    the reason a DIFFERENCE is spelled unsigned here while four
    //    differences elsewhere in this file are not.
    assert_eq!(
        code.matches("यदि मूलदैर्घ्य न्यूनम् प्रत्ययदैर्घ्य आदि").count(),
        2,
        "one of the two suffix walks no longer refuses a base shorter than \
         its suffix. `आरम्भः भवति मूलदैर्घ्य वियोगः प्रत्ययदैर्घ्य` is \
         unsigned BECAUSE that early return has already excluded the negative \
         case; without it the difference underflows and the spelling is wrong"
    );
    assert_eq!(
        code.matches("चरः आरम्भः ॱॱ न६४ भवति मूलदैर्घ्य वियोगः प्रत्ययदैर्घ्य ।")
            .count(),
        2,
        "the guarded difference is written some other way now. It is this \
         file's one counter-case to *a signed quantity in this port is almost \
         always a DIFFERENCE*, and the counter-case is the guard"
    );

    // 4. THE TWO OCTET COUNTS ARE MEASURED, NOT ASSERTED. `स्थानप्रत्ययदैर्घ्यम्`
    //    returns १५ and १२, and the file's own comment says the numbers are
    //    written "beside the akṣara so the two can be checked against each
    //    other BY EYE". They are checked against the real UTF-8 here instead.
    let upper = "ॱउपरि".as_bytes();
    let lower = "ॱअधः".as_bytes();
    for (suffix, bytes) in [("ॱउपरि", upper), ("ॱअधः", lower)] {
        let numeral = devanagari(bytes.len());
        assert!(
            code.contains(&format!("प्रत्यागमनम् {numeral} ।")),
            "`स्थानप्रत्ययदैर्घ्यम्` does not return {numeral} — the {} UTF-8 \
             octets of `{suffix}`, which is the string `split_address_part` \
             strips. A length is the first of this divider's six unsigned \
             sites and the one every other reads",
            bytes.len()
        );
    }

    // 5. AND EVERY OCTET OF THE TABLE, THE SAME WAY. `स्थानप्रत्ययाष्टकम्` is
    //    fifteen numerals standing in for a string literal the frozen `token`
    //    production cannot lex (`F-004f2`); nothing had ever compared them with
    //    the string they spell. An arm that disagrees encodes `%hi`/`%lo`
    //    against a suffix no operand carries, and `स्थानप्रत्ययवत्` would then
    //    refuse every address modifier silently.
    for (i, byte) in upper.iter().enumerate() {
        let arm = format!(
            "यदि क्रमः समम् {} आदि प्रत्यागमनम् {} ।",
            devanagari(i),
            devanagari(*byte as usize)
        );
        assert!(
            code.contains(&arm) || i == upper.len() - 1,
            "`स्थानप्रत्ययाष्टकम्` does not answer {} for octet {i} of \
             `ॱउपरि`. The table is the port's replacement for a string \
             literal and every entry has to spell the akṣara beside it",
            devanagari(*byte as usize)
        );
    }
    assert!(
        code.contains(&format!(
            "प्रत्यागमनम् {} ।",
            devanagari(upper[upper.len() - 1] as usize)
        )),
        "`ॱउपरि`'s last octet is no longer the fall-through of the upper arm"
    );
    for (i, byte) in lower.iter().enumerate() {
        if upper.get(i) == Some(byte) {
            continue; // a shared arm, already checked against `ॱउपरि`
        }
        let arm = format!(
            "यदि क्रमः समम् {} आदि प्रत्यागमनम् {} ।",
            devanagari(i),
            devanagari(*byte as usize)
        );
        assert!(
            code.contains(&arm) || i == lower.len() - 1,
            "`स्थानप्रत्ययाष्टकम्` does not answer {} for octet {i} of \
             `ॱअधः`. Only the third octet of each akṣara tells the two \
             modifiers apart, so a wrong one here is invisible to a reader \
             comparing the two columns",
            devanagari(*byte as usize)
        );
    }
    assert!(
        code.contains(&format!(
            "प्रत्यागमनम् {} ।",
            devanagari(lower[lower.len() - 1] as usize)
        )),
        "`ॱअधः`'s last octet is no longer the fall-through of the lower arm"
    );

    // 6. THE DIVIDER IS AT ZERO IN THE CENSUS'S OWN TERMS. `encode.t1`'s header
    //    states the remaining 38 as living in seven OTHER sections and this one
    //    as done; that table is what the next cycle picks its unit from, so the
    //    claim is measured rather than written down.
    let divider: Vec<&str> = src
        .lines()
        .skip_while(|l| !l.contains("══ the two halves of an address ══"))
        .take_while(|l| !l.contains("══ the value that half carries ══"))
        .collect();
    assert!(
        divider.len() > 90,
        "the `the two halves of an address` divider is {} lines and was 101. \
         Its boundaries are how the count below is scoped",
        divider.len()
    );
    let visible = divider
        .iter()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .flat_map(|c| {
            c.replace("ॱॱ", "\u{1}")
                .replace('ॱ', " ॱ ")
                .replace('\u{1}', " ॱॱ ")
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥').to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<String>>()
        .windows(2)
        .filter(|p| (p[0] == "ॱॱ" || p[0] == "ददाति") && p[1] == "अ६४")
        .count();
    assert_eq!(
        visible, 0,
        "`the two halves of an address` holds {visible} `अ६४` the ledger can \
         see, not 0. Unlike `choosing a compressed form` this divider hides \
         NO slice element types either, so 0 here is the whole claim"
    );
}

#[test]
fn the_alignment_arithmetic_is_unsigned_because_its_differences_are_guarded() {
    // ADR-0030, task `D-002h`. `encode.t1`'s TENTH bounded part, and the
    // LARGEST SINGLE PIECE THE FILE HAD LEFT: `laying out and emitting`, the
    // divider entire bar the one routine whose twin casts to `i64`. Fourteen
    // sites and the census sees all fourteen — `संरेखान्तरम्`'s two parameters,
    // its return and its `संरेखितम्`; `संरेखपूरणाष्टकम्`'s three parameters and
    // its four bindings `अग्रिमम्`, `शिष्टम्`, `स्थितिः` and `चतुष्कम्`;
    // `विन्याससाम्यम्`'s `दैर्घ्य` and `सूचकाङ्क`; and `पदाष्टकम्`'s `क्रमः`.
    // `encode.t1` carried 38 `अ६४` and 148 `न६४`; it carries 24 and 162 now.
    // Corpus ledger 573/0/805 -> 559/0/819, sum still 1378.
    //
    // THE DIVIDER DOES NOT REACH 0 AND THAT IS THE RESULT, NOT A SHORTFALL.
    // Three `अ६४` remain and all three are `विन्यासविचलनम्`'s — its two
    // parameters and its return — and they are CORRECT. A later cycle reading
    // this divider as unfinished from the count alone would respell them and
    // break the E23 argument; item 6 below is what stops that being a silent
    // change.
    //
    // THE READING IS NEW AND IT IS THE FIRST ONE THAT ADJUDICATES A DIVIDER IN
    // BOTH DIRECTIONS AT ONCE: *THE TWIN SPELLS THE SIGN OF EVERY DIFFERENCE
    // HERE, AND IT SPELLS FOUR OF THEM UNSIGNED AND ONE SIGNED — AND WHAT
    // SEPARATES THEM IS THE GUARD.* This divider is five subtractions in 110
    // lines, more than any other section of the file:
    //
    //   `संरेखितम् वियोगः क्रमसूचकः`        guarded by the `यावत्` above it
    //   `पूरणम् वियोगः अग्रिमम्`            guarded by `यदि पूरणम् अधिकम् १`
    //   `क्रमः वियोगः अग्रिमम्`             guarded by the early return above it
    //   `शिष्टम् वियोगः (शिष्टम् युक् ३)`   guarded by the remainder itself
    //   `उत्सृष्टम् वियोगः विन्यस्तम्`      NOT GUARDED — and it is the one left
    //
    // Cycle 532 found the first counter-case to this file's standing rule *a
    // signed quantity in this port is almost always a DIFFERENCE* and stated
    // the repair as *a guarded difference is not a signed one*. There it
    // decided ONE site. Here it decides a whole unit, and the twin testifies
    // to both halves of it rather than to one: `align_pad_at` and `align_fill`
    // are `u32` from parameter to return (`encode.rs:1747`, `:1759`), while
    // the drift `विन्यासविचलनम्` computes is written `(text.len() as i64 -
    // i64::from(addresses[i]))` at `:637` and `:1067` — an explicit `i64` cast
    // in a twin that names no other signed type in this whole divider. So
    // `विन्यासविचलनम्` is signed BY ITS TWIN and not merely by the rule, which
    // is a stronger claim than the one ADR-0031 filed it under.
    //
    // AND THE UNGUARDED ONE IS UNGUARDED ON PURPOSE. Layout and emission are
    // two passes and nothing requires them to agree; the encoder checks that
    // they do, and a check for a disagreement that could go either way is
    // exactly where a sign is earned. The other four are arithmetic the code
    // has already excluded the negative case for.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");
    let code: String = src
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    // 1. THE TWIN'S UNSIGNED HALF, PINNED AT ITS SIGNATURES. Both alignment
    //    routines are `u32` end to end and neither has a `usize` anywhere a
    //    sign could be read from; the port widens to 64 bits, which is a WIDTH
    //    decision and not a sign one.
    for sig in [
        "pub(crate) fn align_pad_at(program: &crate::parse::Program, i: usize, pc: u32) -> u32 {",
        "pub(crate) fn align_fill(pc: u32, pad: u32) -> Vec<u8> {",
    ] {
        assert!(
            twin.contains(sig),
            "`encode.rs` no longer declares `{sig}`. The four unsigned \
             differences in `laying out and emitting` are spelled off these \
             two signatures; if the twin grows a signed type here the \
             reading is void and every site in this unit needs re-reading"
        );
    }
    assert!(
        twin.contains("let mut addresses: Vec<u32> = Vec::new();")
            && twin.contains("let mut previous: Vec<u32> = Vec::new();"),
        "`layout_addresses`'s `addresses`/`previous` are no longer `Vec<u32>`. \
         `विन्याससाम्यम्` is `addresses == previous`, and its `दैर्घ्य` and \
         `सूचकाङ्क` are that vector's length and an index into it"
    );

    // 2. THE TWIN'S SIGNED HALF, AND IT IS ONE EXPRESSION. This is the only
    //    place in the whole divider's twin that names a signed type, and it is
    //    the routine this unit LEFT. The assertion is what makes "the twin
    //    spells four unsigned and one signed" a measurement.
    assert_eq!(
        twin.matches("(text.len() as i64 - i64::from(addresses[i])).to_string(),")
            .count(),
        2,
        "the E23 drift argument no longer casts to `i64`. `विन्यासविचलनम्` is \
         the one routine in `laying out and emitting` left at `अ६४`, and this \
         cast is WHY — layout and emission are two passes that may disagree in \
         either direction, so the difference between them is the one here that \
         nothing has excluded the negative case for"
    );
    assert!(
        !twin[twin.find("pub(crate) fn align_pad_at").unwrap()
            ..twin.find("pub(crate) fn align_fill").unwrap()]
            .contains("i64")
            && !twin[twin.find("pub(crate) fn align_fill").unwrap()..]
                .lines()
                .take(20)
                .any(|l| l.contains("i64") || l.contains("isize")),
        "one of the two alignment routines now names a signed type. They are \
         the twin half of eleven of this unit's fourteen sites"
    );

    // 3. THE FOURTEEN SITES, BY NAME. A count that moved is not the same claim
    //    as the right sites having moved, and only this half survives a
    //    re-measurement of the ledger.
    let routine_body = |name: &str| -> String {
        let head = format!("वृत्तिः {name} आदाय");
        let start = code
            .find(&head)
            .unwrap_or_else(|| panic!("`encode.t1` no longer declares `{name}`"));
        let rest = &code[start..];
        let end = rest
            .find("\nइति")
            .unwrap_or_else(|| panic!("`{name}` has no closing `इति`"));
        rest[..end].to_string()
    };
    for sig in [
        "वृत्तिः संरेखान्तरम् आदाय क्रमसूचकः ॱॱ न६४ सीमा ॱॱ न६४ ददाति न६४ आदि",
        "वृत्तिः संरेखपूरणाष्टकम् आदाय क्रमसूचकः ॱॱ न६४ पूरणम् ॱॱ न६४ क्रमः ॱॱ न६४ ददाति अ८ आदि",
        "वृत्तिः पदाष्टकम् आदाय पदम् ॱॱ अ३२ क्रमः ॱॱ न६४ ददाति अ८ आदि",
    ] {
        assert!(
            code.contains(sig),
            "`encode.t1` no longer declares `{sig}`. An octet index, an \
             alignment boundary and a program counter are `u32` in the twin \
             and unsigned in every signature `core` gives them"
        );
    }
    for (routine, binding) in [
        ("संरेखान्तरम्", "संरेखितम्"),
        ("संरेखपूरणाष्टकम्", "अग्रिमम्"),
        ("संरेखपूरणाष्टकम्", "शिष्टम्"),
        ("संरेखपूरणाष्टकम्", "स्थितिः"),
        ("संरेखपूरणाष्टकम्", "चतुष्कम्"),
        ("विन्याससाम्यम्", "दैर्घ्य"),
        ("विन्याससाम्यम्", "सूचकाङ्क"),
    ] {
        let body = routine_body(routine);
        assert!(
            body.contains(&format!("चरः {binding} ॱॱ न६४ भवति ")),
            "`{routine}` no longer binds `{binding}` as `न६४`"
        );
        assert!(
            !body.contains(&format!("चरः {binding} ॱॱ अ६४")),
            "`{routine}` binds `{binding}` as `अ६४` again"
        );
    }

    // 4. THE FOUR GUARDS. Each is the code that has already excluded the
    //    negative case from the subtraction under it, and the subtraction is
    //    pinned beside its guard rather than separately — a guard that stops
    //    guarding some OTHER expression is not what this unit read.
    for (guard, difference, why) in [
        (
            "यावत् संरेखितम् न्यूनम् क्रमसूचकः आदि",
            "प्रत्यागमनम् संरेखितम् वियोगः क्रमसूचकः ।",
            "the loop runs until `संरेखितम्` has reached `क्रमसूचकः`, so the \
             return is a difference the loop condition has already made \
             non-negative — `at.next_multiple_of(n) - pc` in the twin",
        ),
        (
            "यदि पूरणम् अधिकम् १ आदि",
            "चरः शिष्टम् ॱॱ न६४ भवति पूरणम् वियोगः अग्रिमम् ।",
            "`अग्रिमम्` is २ only inside this test and ० otherwise, so \
             `पूरणम् वियोगः अग्रिमम्` cannot underflow — it is the twin's \
             `end - at >= 2` before `at += 2`",
        ),
        (
            "यदि क्रमः न्यूनम् अग्रिमम् आदि",
            "चरः स्थितिः ॱॱ न६४ भवति क्रमः वियोगः अग्रिमम् ।",
            "the routine RETURNS two lines above this subtraction when \
             `क्रमः` is inside the leading `c.nop`, which is the whole of \
             the negative case",
        ),
        (
            "चरः चतुष्कम् ॱॱ न६४ भवति शिष्टम् वियोगः आरभ्य शिष्टम् युक् ३ समाप्तम् ।",
            "चरः चतुष्कम् ॱॱ न६४ भवति शिष्टम् वियोगः आरभ्य शिष्टम् युक् ३ समाप्तम् ।",
            "the subtrahend is a remainder OF THE MINUEND, so this one is \
             guarded by its own arithmetic and needs no test above it",
        ),
    ] {
        assert!(
            code.contains(guard),
            "the guard `{guard}` is gone from `encode.t1`. {why}. Without it \
             the difference below is signed and this unit's spelling is wrong"
        );
        assert!(
            code.contains(difference),
            "the guarded difference `{difference}` is written some other way \
             now. {why}"
        );
    }

    // 5. AND THE ROUTINE WITH NO GUARD, WHICH IS THE COUNTER-CASE AND THE
    //    REASON THE OTHER FOUR ARE A READING RATHER THAN A HABIT.
    //    `विन्यासविचलनम्` is one line: a subtraction with nothing above it.
    let drift = routine_body("विन्यासविचलनम्");
    assert!(
        drift.contains("आदाय विन्यस्तम् ॱॱ अ६४ उत्सृष्टम् ॱॱ अ६४ ददाति अ६४ आदि")
            && drift.contains("प्रत्यागमनम् उत्सृष्टम् वियोगः विन्यस्तम् ।"),
        "`विन्यासविचलनम्` is no longer `अ६४` throughout. It is the one site in \
         `laying out and emitting` this unit LEFT, its twin casts to `i64` \
         explicitly, and respelling it would make the E23 drift report a \
         backwards drift as an enormous forwards one"
    );
    assert!(
        !drift.contains("यदि") && !drift.contains("यावत्"),
        "`विन्यासविचलनम्` has grown a test. Its being unguarded is the \
         DISTINGUISHING fact of this unit — four differences beside it are \
         unsigned because something above them excluded the negative case, \
         and if this one gains such a test its spelling has to be re-read"
    );

    // 6. THE PADDING OCTETS, MEASURED AGAINST THE TWIN'S OWN LITERALS RATHER
    //    THAN TRANSCRIBED. Nothing had ever checked these. `संरेखपूरणाष्टकम्`
    //    answers १ and १९ and the file's comment explains them in prose as
    //    "0x0001 little-endian is १ then ०" and "0x00000013 little-endian is
    //    १९ ० ० ०"; the numerals are computed here from the `u16` and `u32`
    //    the twin writes. A wrong one pads `.text` with something that is not
    //    `nop`, which on RISC-V means a trap, and the differential oracle
    //    against GNU `as` would be the only thing to notice.
    assert!(
        twin.contains("out.extend_from_slice(&0x0001u16.to_le_bytes());")
            && twin.contains("out.extend_from_slice(&0x0000_0013u32.to_le_bytes());"),
        "`align_fill` no longer writes `c.nop` as `0x0001u16` and `nop` as \
         `0x0000_0013u32`. Those two literals are what the numerals below are \
         computed from"
    );
    let devanagari = |n: usize| -> String {
        if n == 0 {
            return "०".to_string();
        }
        let digits: Vec<char> = "०१२३४५६७८९".chars().collect();
        let (mut out, mut n) = (Vec::new(), n);
        while n > 0 {
            out.push(digits[n % 10]);
            n /= 10;
        }
        out.iter().rev().collect()
    };
    let fill = routine_body("संरेखपूरणाष्टकम्");
    let c_nop = 0x0001u16.to_le_bytes();
    let nop = 0x0000_0013u32.to_le_bytes();
    assert!(
        fill.contains(&format!("प्रत्यागमनम् {} ।", devanagari(c_nop[0] as usize))),
        "`संरेखपूरणाष्टकम्` does not answer {} for the first octet of \
         `c.nop`. A run of the wrong halfword is not an instruction on \
         RISC-V, it traps",
        devanagari(c_nop[0] as usize)
    );
    assert!(
        fill.contains(&format!("प्रत्यागमनम् {} ।", devanagari(nop[0] as usize))),
        "`संरेखपूरणाष्टकम्` does not answer {} for the first octet of `nop`",
        devanagari(nop[0] as usize)
    );
    assert!(
        fill.contains(&format!("अग्रिमम् भवति {} ।", devanagari(c_nop.len()))),
        "the leading `c.nop` is no longer {} octets wide. `अग्रिमम्` is the \
         width of the halfword the twin writes, and every one of `शिष्टम्`, \
         `स्थितिः` and `चतुष्कम्` is measured from it",
        devanagari(c_nop.len())
    );
    assert!(
        fill.contains(&format!("युक् {} समाप्तम्", devanagari(nop.len() - 1))),
        "the four-octet `nop` boundary is no longer {}. `चतुष्कम्` is \
         `शिष्टम्` less `शिष्टम् % {}`, and that mask is the twin's `nop` \
         width less one",
        devanagari(nop.len() - 1),
        devanagari(nop.len())
    );

    // 7. THE DIVIDER IN THE CENSUS'S OWN TERMS: 3, NOT 0, AND THE THREE ARE
    //    NAMED. `encode.t1`'s header states the remaining 24 as living in
    //    eight sections and this one as holding only the drift routine; that
    //    table is what the next cycle picks its unit from, so the claim is
    //    measured rather than written down. Every other section this repair
    //    finished reached 0; this one CANNOT, and saying which three are left
    //    is what keeps that from reading as unfinished work.
    let divider: Vec<&str> = src
        .lines()
        .skip_while(|l| !l.contains("══ laying out and emitting ══"))
        .take_while(|l| !l.contains("══ ASSEMBLING A WORD ══"))
        .collect();
    assert!(
        divider.len() > 100,
        "the `laying out and emitting` divider is {} lines and was 109. Its \
         boundaries are how the count below is scoped",
        divider.len()
    );
    let visible = divider
        .iter()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .flat_map(|c| {
            c.replace("ॱॱ", "\u{1}")
                .replace('ॱ', " ॱ ")
                .replace('\u{1}', " ॱॱ ")
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥').to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<String>>()
        .windows(2)
        .filter(|p| (p[0] == "ॱॱ" || p[0] == "ददाति") && p[1] == "अ६४")
        .count();
    assert_eq!(
        visible, 3,
        "`laying out and emitting` holds {visible} `अ६४` the ledger can see, \
         not 3. The three are `विन्यासविचलनम्`'s two parameters and its \
         return and they are CORRECT — this is the one divider of the repair \
         that does not end at 0, and 3 is its finished state"
    );
}

#[test]
fn the_pc_relative_halves_are_signed_because_the_offset_they_split_is() {
    // ADR-0030, task `D-002h`. `encode.t1`'s ELEVENTH bounded part:
    // `the value that half carries`, the divider entire. Seven sites, and
    // **NOT ONE OF THEM MOVES** — this is the first divider of the whole
    // repair that is finished at its FULL count rather than at 0 or at a
    // remainder. `laying out and emitting` ended at 3 of 17; this one ends at
    // 7 of 7. The ledger does not move and that is the result: 559/0/819,
    // sum 1378, unchanged.
    //
    // THE READING IS NEW AND IT IS THE FIRST THAT IS MADE OFF A SINGLE
    // DECLARATION RATHER THAN OFF THREE SIGNATURES: *ONE `i64` AT THE TOP OF
    // THE TWIN COLOURS EVERY SITE BELOW IT, BECAUSE THIS DIVIDER IS NOT THREE
    // ROUTINES THAT HAPPEN TO AGREE — IT IS ONE EXPRESSION SPLIT INTO THREE.*
    // `encode.rs:1491` writes `let offset = i64::from(*target) - i64::from(base)`
    // and nothing in the divider's twin ever narrows it: `:1492`'s `hi` is
    // that `offset` shifted, `:1493`'s `value` is `hi` or `offset` less `hi`
    // shifted back, and both are `i64` by inference from the one declaration.
    // Every other unit of this repair had to read each routine against its own
    // twin signature; here there ARE no signatures, only a chain of `let`s, so
    // the sign is decided once and inherited five times.
    //
    // AND THE COUNTER-CASE IS INSIDE THE SAME DIVIDER, WHICH IS WHY THE
    // COINCIDENCE IS NOT ONE. `स्थानाधारः` is the third routine here and it is
    // signed for a DIFFERENT reason — its twin (`pc`, `base`) is `u32` on both
    // sides and it is signed only because T1 has no `न३२` in which
    // `wrapping_sub` at 2^32 could be spelled. So this divider holds two
    // routines signed by the offset they split and one signed by a width the
    // port does not have, and the count 7 is a sum of two readings and not one.
    // `the_address_half_is_signed_by_its_consumer_and_not_by_being_a_difference`
    // holds the second; this test holds the first and the arithmetic.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");
    let code: String = src
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    // 1. THE ONE DECLARATION, AND THE TWO LINES THAT INHERIT FROM IT. These
    //    three fragments ARE the reading; if the twin ever narrows `offset`
    //    to an unsigned type every site in this unit needs re-reading, and a
    //    `>>` that changed from arithmetic to logical would not otherwise say
    //    so anywhere.
    for fragment in [
        "let offset = i64::from(*target) - i64::from(base);",
        "let hi = (offset + 0x800) >> 12;",
        "let value = if upper { hi } else { offset - (hi << 12) };",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. This divider's five \
             moving sites have no signatures of their own — they inherit `i64` \
             from this one chain of `let`s, and the chain is the whole reading"
        );
    }

    // 2. THE FIVE SITES, BY NAME. `उपरिखण्डम्` is `hi` and `स्थानमूल्यम्` is
    //    `value`; between them they are two parameters, two returns and one
    //    binding, and all five stay `अ६४`.
    for sig in [
        "वृत्तिः उपरिखण्डम् आदाय अन्तरम् ॱॱ अ६४ ददाति अ६४ आदि",
        "वृत्तिः स्थानमूल्यम् आदाय अन्तरम् ॱॱ अ६४ उपरि ॱॱ बूल ददाति अ६४ आदि",
    ] {
        assert!(
            code.contains(sig),
            "`encode.t1` no longer declares `{sig}`. `अन्तरम्` is the twin's \
             `offset`, which `encode.rs:1491` declares `i64` — respelling it \
             `न६४` makes a backwards reference an address near 2^64"
        );
    }
    assert!(
        code.contains("चरः उच्चम् ॱॱ अ६४ भवति उपरिखण्डम् अन्तरम् ।"),
        "`स्थानमूल्यम्`'s `उच्चम्` is no longer `अ६४` filled from \
         `उपरिखण्डम्`. It is the twin's `hi`, and it is shifted LEFT twelve \
         places and subtracted on the next line — the one site in this \
         divider that is both a result and an operand"
    );

    // 3. THE ARITHMETIC SHIFT, DEMONSTRATED RATHER THAN ASSERTED. This is what
    //    makes `अ६४` here a correctness claim and not a style one: the same
    //    two lines over the same bits give three different answers at the
    //    three spellings available, and only the signed one is the twin's.
    let offset: i64 = -0x1000;
    let signed_hi = (offset + 0x800) >> 12;
    let unsigned_hi = (offset as u64).wrapping_add(0x800) >> 12;
    assert_eq!(
        signed_hi, -1,
        "a reference one instruction-page backwards no longer has a high half \
         of -1"
    );
    assert_ne!(
        signed_hi as u64, unsigned_hi,
        "the arithmetic and the logical shift agree on a negative offset, so \
         this test has stopped being able to tell `अ६४` from `न६४` here"
    );

    // 4. AND WHAT THE ०षोड्८०० IS FOR, WHICH NOTHING HAD EVER CHECKED. The
    //    file's comment explains it in prose — *"the low half is
    //    sign-extended by the hardware, so a low half at or past 0x800 reads
    //    as negative and the high half has to have counted one more"* — and
    //    nowhere measures it. The property is that the two halves RECONSTRUCT
    //    the offset and that the low one always lands inside the signed
    //    12-bit range the `I`-format field can hold. A wrong rounding constant
    //    keeps the first and loses the second, which is exactly the failure
    //    `fits` then reports as E06 at a line the user did not write wrong.
    let low_width = 12u32;
    let round = 1i64 << (low_width - 1);
    assert_eq!(
        round, 0x800,
        "the rounding constant is not the sign bit of a {low_width}-bit low \
         half any more"
    );
    for offset in [
        -0x8000_0000i64,
        -0x1001,
        -0x1000,
        -0x801,
        -0x800,
        -0x7ff,
        -1,
        0,
        1,
        0x7ff,
        0x800,
        0x1234,
        0x7fff_f7ff,
    ] {
        let hi = (offset + round) >> low_width;
        let lo = offset - (hi << low_width);
        assert_eq!(
            (hi << low_width) + lo,
            offset,
            "`auipc`+`addi` no longer reconstruct {offset:#x}"
        );
        assert!(
            (-round..round).contains(&lo),
            "the low half of {offset:#x} is {lo:#x}, outside the signed \
             {low_width}-bit range — the ०षोड्८०० rounding is what keeps it \
             inside, and outside it the field encodes a different number"
        );
    }
    assert!(
        code.contains("प्रत्यागमनम् आरभ्य अन्तरम् योगः ०षोड्८०० समाप्तम् दक्षिणसृ १२ ।")
            && code.contains("प्रत्यागमनम् अन्तरम् वियोगः आरभ्य उच्चम् वामसृ १२ समाप्तम् ।"),
        "`उपरिखण्डम्` and `स्थानमूल्यम्` no longer write the two lines the \
         property above was checked on"
    );

    // 5. THE PAIR THIS DIVIDER LEAVES OWING, AND IT IS OWED BY A CALLER THAT
    //    DOES NOT EXIST YET. `स्थानमूल्यम्`'s answer is PLACED: the twin
    //    writes `imm.fits(value as u64)` and `imm.place(value as u64)`, and
    //    both callees take `u64` and `fits` reinterprets with `value as i64`
    //    inside itself. ADR-0031's rule is that the pair is made at the CALLER
    //    because that is the only place both readings exist — so when T1 grows
    //    this call site it hands `अन्तर्भावः` a `न६४` member that
    //    `स्थानमूल्यम्` does not return. That obligation is recorded here
    //    rather than discharged, because the caller is `D-002h`'s later work.
    for fragment in [
        "if !imm.fits(value as u64) {",
        "word |= imm.place(value as u64);",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. The cast back to `u64` \
             at the call is what makes `स्थानमूल्यम्`'s signed answer the \
             `चिह्नितम्` member of an ADR-0031 pair rather than a lone value"
        );
    }
    assert!(
        twin.contains("pub fn fits(&self, value: u64) -> bool {")
            && twin.contains("let signed = value as i64;"),
        "`fits` no longer takes `u64` and reinterprets it as `i64`. That round \
         trip is why T1 needs BOTH members: it has no cast, so the two \
         readings have to arrive as two parameters"
    );
    assert!(
        code.contains(
            "वृत्तिः अन्तर्भावः आदाय अवकाशः ॱॱ अवकाश मूल्यम् ॱॱ न६४ चिह्नितम् ॱॱ अ६४ ददाति बूल आदि"
        ),
        "`अन्तर्भावः` is no longer the `न६४`/`अ६४` pair. It is the consumer \
         `स्थानमूल्यम्`'s return is owed to, and the shape of that pair is the \
         obligation this divider leaves standing"
    );

    // 6. THE DIVIDER IN THE CENSUS'S OWN TERMS: 7, NOT 0, AND 7 IS FINISHED.
    //    `encode.t1`'s header states the remaining sites per divider and that
    //    table is what the next cycle picks its unit from. `laying out and
    //    emitting` needed this guard for 3 of 17; this divider needs it for
    //    ALL of its count, which is a stronger trap — a cycle reading the
    //    table alone would take the largest number left and respell every one
    //    of them.
    let divider: Vec<&str> = src
        .lines()
        .skip_while(|l| !l.contains("══ the value that half carries ══"))
        .take_while(|l| !l.contains("══ laying out and emitting ══"))
        .collect();
    assert!(
        divider.len() > 55,
        "the `the value that half carries` divider is {} lines and was 68. \
         Its boundaries are how the count below is scoped",
        divider.len()
    );
    let tokens: Vec<String> = divider
        .iter()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .flat_map(|c| {
            c.replace("ॱॱ", "\u{1}")
                .replace('ॱ', " ॱ ")
                .replace('\u{1}', " ॱॱ ")
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥').to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    let visible = |ty: &str| {
        tokens
            .windows(2)
            .filter(|p| (p[0] == "ॱॱ" || p[0] == "ददाति") && p[1] == ty)
            .count()
    };
    assert_eq!(
        (visible("अ६४"), visible("न६४")),
        (7, 0),
        "`the value that half carries` holds ({}, {}) `अ६४`/`न६४` the ledger \
         can see, not (7, 0). ALL SEVEN ARE CORRECT and none of them moves: \
         five are the twin's `offset`, `hi` and `value`, which descend from \
         one `i64::from(*target) - i64::from(base)`, and two are \
         `स्थानाधारः`'s, signed for the separate reason that T1 has no `न३२`. \
         This is the one divider of the repair that is finished at its full \
         count",
        visible("अ६४"),
        visible("न६४")
    );
}

#[test]
fn the_head_declarations_are_unsigned_because_a_limit_a_section_and_a_line_are() {
    // ADR-0030, task `D-002h`. `encode.t1`'s TWELFTH bounded part and the last
    // unit of any SIZE the file had with sites that actually move: the head
    // declarations, everything above `══ slot kind predicates ══`. Six sites,
    // and this census sees all six — the ledger moves 559/0/819 -> 553/0/825
    // and `encode.t1`'s own count 24/162 -> 18/168.
    //
    // THE READING IS THAT THESE SIX ARE THREE DIFFERENT KINDS OF THING AND
    // EVERY ONE OF THEM HAS ALREADY BEEN DECIDED SOMEWHERE ELSE IN THIS
    // REPAIR. A loop bound (`शिथिलनावृत्तिसीमा`), a discriminant family
    // (`पाठकोष्ठकम्`/`दत्तकोष्ठकम्`/`शोधनकोष्ठकम्`) and two record FIELDS —
    // a section and a line. Nothing new is argued here; what is new is that
    // the three readings meet in one block, which is why the unit is the
    // block and not a family.
    //
    // AND THE HEADER'S OWN TABLE NAMED THESE SIX WRONG, TWICE, WHICH IS WHY
    // ITEM 4 BELOW EXISTS. `encode.t1:347` called one of them *one register
    // field* and `:125` replaced that with *two under `which instruction set
    // the image is for`*. Neither is a site: the three register fields are
    // `अ३२`, and `लक्ष्य`'s two constants are `गणना` members that carry no
    // type annotation at all and so were never counted by anything. The six
    // are the ones enumerated in item 1. A wrong table is what a cycle picks
    // its unit from, so it is corrected in the same commit and pinned here.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");
    let kosha = std::fs::read_to_string(root().join("crates/sadhana/src/kosha.rs"))
        .expect("read crates/sadhana/src/kosha.rs");

    // 1. THE SIX SITES, EACH SPELLED OUT. The value is part of the reading for
    //    the four that have one: `८` is a round count, and the sections number
    //    from १ with no ०, so neither family has anywhere to put a sign.
    for site in [
        "सार्वजनिक चरः शिथिलनावृत्तिसीमा ॱॱ न६४ भवति ८ ।",
        "सार्वजनिक चरः पाठकोष्ठकम् ॱॱ न६४ भवति १ ।",
        "सार्वजनिक चरः दत्तकोष्ठकम् ॱॱ न६४ भवति २ ।",
        "सार्वजनिक चरः शोधनकोष्ठकम् ॱॱ न६४ भवति ३ ।",
        // `  कोष्ठकम् ॱॱ न६४ ऽ` stood here: the section field of the FIVE-field
        // `संरचना प्रतीक्षा`, which `W-247` removed on 2026-09-04 — the file
        // declared `प्रतीक्षा` twice and the code built the two-field one, which
        // has no section field. Five sites now; the reading is unchanged.
        "  पङ्क्ति ॱॱ न६४ ऽ",
    ] {
        assert!(
            src.contains(site),
            "`encode.t1` no longer writes `{site}`. These six are the file's \
             head declarations: a loop bound, a three-value section \
             discriminant and two record fields. A respelling, a renumbering \
             from ० or a gap in the run each break the reading and has to be \
             re-argued rather than inherited"
        );
    }

    // 2. THE TWIN, WHICH SPELLS TWO OF THE THREE READINGS OUTRIGHT AND HAS NO
    //    NUMBER AT ALL FOR THE THIRD. `MAX_RELAXATION_ROUNDS` is a `usize`
    //    used only as the top of a range, and `EncodeError::line` is a
    //    `usize`; `RelSection` is an enum with no `repr`, no explicit
    //    discriminant and no numeric field, so its integer exists only
    //    because T1 has no enum payload — the same shape as the slot kinds in
    //    `the_slot_kind_is_unsigned_because_a_discriminant_has_no_sign_to_read`.
    for fragment in [
        "pub(crate) const MAX_RELAXATION_ROUNDS: usize = 8;",
        "for _ in 0..MAX_RELAXATION_ROUNDS {",
        "pub section: crate::kosha::RelSection,",
        "pub line: usize,",
    ] {
        assert!(
            twin.contains(fragment),
            "`encode.rs` no longer writes `{fragment}`. Two of this unit's \
             three readings are read straight off these declarations; if the \
             twin has grown a signed one, the reading must be made again off \
             that type and not inherited from here"
        );
    }
    let rel = kosha
        .split_once("pub enum RelSection {")
        .expect("`kosha.rs` still declares `RelSection`")
        .1
        .split_once('}')
        .expect("`RelSection` closes")
        .0;
    assert!(
        rel.contains("Text,") && rel.contains("Data,") && rel.contains("Debug(&'static str),"),
        "`kosha.rs`'s `RelSection` is no longer Text/Data/Debug(name). The T1 \
         port flattens it into three numbered constants plus `प्रतीक्षा ॱ \
         कोष्ठकनाम` for the payload, and that flattening is what the \
         constants are FOR"
    );
    assert!(
        !rel.contains('=') && !kosha.contains("#[repr(u"),
        "`RelSection` has grown an explicit discriminant or a `repr`. Its \
         having NO number is the whole reason the port's numbers are the \
         port's own, and a numbered twin would have to be read instead of \
         this reading"
    );
    // `W-247`, 2026-09-04: this asserted `  कोष्ठकनाम ॱॱ पाठ` — the payload's
    // field beside the discriminant. That field stood on the FIVE-field
    // `संरचना प्रतीक्षा`, which nothing in `encode.t1` built or read; the file
    // declared `प्रतीक्षा` twice and the code built the two-field one, so the
    // duplicate went. The flattening's reading is unchanged: when the linker
    // side gives `प्रतीक्षा` its section, the discriminant and the name go in
    // side by side, as this test says — and this assertion returns with them.
    // Until then the survivor carries the name and the offset, and says so.
    assert!(
        src.contains("  नाम ॱॱ अङ्कः अन्तः अ८ ऽ") && src.contains("  स्थानम् ॱॱ अ३२"),
        "`प्रतीक्षा` no longer carries its name and offset — the two fields the \
         assembler's E05 sites record (W-247)"
    );
    assert!(
        !src.contains("  कोष्ठकनाम ॱॱ पाठ"),
        "`प्रतीक्षा` carries `कोष्ठकनाम` again: then it carries the section too, \
         and this assertion goes back to asserting the flattening (W-247)"
    );

    // 3. THE TWO MIRRORS ALREADY `न६४`, IN THREE REMEDIATED FILES. A section
    //    and a line are both spellings this repair has settled before, and
    //    until this cycle `encode.t1` disagreed with all three at once.
    //
    //    AND THE SECTION MIRROR IS A NAME COLLISION, NOT MERELY AN AGREEMENT:
    //    `samyojana.t1` declares PUBLIC constants `पाठकोष्ठकम्` and
    //    `दत्तकोष्ठकम्` for `SymSection`, which is a different enum from
    //    `RelSection`. They agree on the two values they share and the third
    //    differs by name (`शोधनकोष्ठकम्` = Debug = ३ here, `बीजकोष्ठकम्` =
    //    Bss = ३ there), so nothing is wrong today — but two modules export
    //    one name for two enums, and T1 has no module system to keep them
    //    apart (`D-002l`). Recorded where a reader of either will meet it.
    //
    //    **RE-FOUNDED 2026-09-11: THE COLLISION THIS PARAGRAPH RECORDS IS
    //    FIXED, AND THE PARAGRAPH'S PREMISE IS GONE.** `W-249` renamed
    //    `samyojana.t1`'s two to `संयोजनपाठकोष्ठकम्`/`संयोजनदत्तकोष्ठकम्`, so the
    //    two modules no longer export one name — `encode.t1` keeps the bare
    //    spelling and the assertions above it, at the `encode.t1` list, are
    //    CORRECT and must not be renamed with these. The text above is kept
    //    because it names WHY the rename was needed, not because it still
    //    describes the tree. **The bare spelling is what `nirvahana.rs`'s
    //    `global_key` would strip a `ॱ`-qualified name back to, so the fix had
    //    to concatenate without a separator; a later tidy into `संयोजनॱपाठकोष्ठकम्`
    //    reintroduces the collision and this assertion would still pass.**
    for (file, mirror) in [
        (
            "samyojana.t1",
            "सार्वजनिक चरः संयोजनपाठकोष्ठकम् ॱॱ न६४ भवति १ ।",
        ),
        (
            "samyojana.t1",
            "सार्वजनिक चरः संयोजनदत्तकोष्ठकम् ॱॱ न६४ भवति २ ।",
        ),
        ("lex.t1", "    पङ्क्ति ॱॱ न६४   ॰ Token::line: usize, 1-based"),
        ("lex.t1", "    पङ्क्ति ॱॱ न६४ ऽ   ॰ LexError::line: usize"),
        ("parse.t1", "    पङ्क्ति ॱॱ न६४ ऽ"),
    ] {
        let text = std::fs::read_to_string(root().join("crates/sadhana-t1/src").join(file))
            .unwrap_or_else(|_| panic!("read crates/sadhana-t1/src/{file}"));
        assert!(
            text.contains(mirror),
            "`{file}` no longer writes `{mirror}`. `encode.t1`'s head \
             declarations were respelled because they were the last sites in \
             a remediated file disagreeing with these; if the mirrors moved \
             instead, the two are apart again in the other direction"
        );
    }

    // 4. THE TWO THINGS THE HEADER'S TABLE MISCOUNTED AS SITES. The register
    //    fields are `अ३२` because they are RISC-V field masks and the twin
    //    writes them `u32`; `लक्ष्य` is a `गणना` whose members carry no type.
    //    Neither is a 64-bit type position and neither ever was.
    for register_field in [
        "सार्वजनिक चरः गन्तृक्षेत्र ॱॱ अ३२ भवति ३९६८ ।",
        "सार्वजनिक चरः प्रथमस्रोतःक्षेत्र ॱॱ अ३२ भवति १०१५८०८ ।",
        "सार्वजनिक चरः द्वितीयस्रोतःक्षेत्र ॱॱ अ३२ भवति ३२५०५८५६ ।",
        "सार्वजनिक चरः तृतीयस्रोतःक्षेत्र ॱॱ अ३२ भवति ४१६०७४९५६८ ।",
    ] {
        assert!(
            src.contains(register_field),
            "`encode.t1` no longer writes `{register_field}`. The header's \
             table called one of these a site of this unit for four cycles; \
             they are `अ३२` field masks, they are not 64-bit type positions, \
             and the correction is what item 4 holds"
        );
    }
    assert!(
        src.contains("सार्वजनिक गणना लक्ष्य आरभ्य")
            && src.contains("  असङ्कुचितम् ऽ")
            && !src.contains("असङ्कुचितम् ॱॱ"),
        "`लक्ष्य` is no longer a `गणना` with unannotated members. The \
         header's newest table called its two constants sites of this unit; a \
         `गणना` member carries no type, so they are counted by nothing"
    );

    // 5. THE BLOCK IN THE CENSUS'S OWN TERMS: 0 LEFT, AND 14 WRITTEN. The
    //    seven slot kinds and their field moved in the seventh part; these six
    //    are the rest of the block, so the head declarations are now finished
    //    entire. This is the trap `laying out and emitting` and `the value
    //    that half carries` each needed: a later cycle reading only the
    //    header's per-divider table must find the block already at 0 here.
    let head: Vec<&str> = src
        .lines()
        .skip_while(|l| !l.contains("══ the three register fields"))
        .take_while(|l| !l.contains("slot kind predicates"))
        .collect();
    assert!(
        head.len() > 80,
        "the head-declaration block is {} lines and was 93. Its boundaries are \
         how the count below is scoped",
        head.len()
    );
    let tokens: Vec<String> = head
        .iter()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .flat_map(|c| {
            c.replace("ॱॱ", "\u{1}")
                .replace('ॱ', " ॱ ")
                .replace('\u{1}', " ॱॱ ")
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥').to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    let visible = |ty: &str| {
        tokens
            .windows(2)
            .filter(|p| (p[0] == "ॱॱ" || p[0] == "ददाति") && p[1] == ty)
            .count()
    };
    // (0, 18) SINCE THE MERGE OF 2026-09-03: this branch had added four
    // declarations to the block after the rail closed it (`प्रतीक्षासूचकाङ्क`,
    // `प्रतीक्षायोजनम्`'s answer, `सङ्केतनदोष`'s two argument fields), all
    // spelled `अ६४`; they are moved to `न६४` in the merge, so the block is
    // finished again at 18 visible unsigned sites and 0 signed.
    // (0, 18) -> (0, 17) on 2026-09-04, `W-247`: the dead five-field `प्रतीक्षा`'s
    // `कोष्ठकम् ॱॱ न६४` went with it (see item 1); still 0 signed.
    // (0, 17) -> (0, 19) on 2026-09-07, W-247's deferral collected: `प्रतीक्षा`
    // gains `कोष्ठकम् ॱॱ न६४` and `प्रतीक्षायोजनम्` the matching parameter. The
    // row also adds `भेदः ॱॱ अ३२` twice, which this ledger does not see and
    // should not — the width is the record's, taken from `वास्तुॱपुनःस्थापनम् ॱ
    // भेदः`, and a relocation type is an ELF field of a fixed width rather
    // than a count.
    //
    // STILL 0 SIGNED, which is what this pin is for. The margin above warns
    // that "an `अ६४` back in it is a site moving the wrong way", and the same
    // claim is made independently by the ADR-0030 triple, which reads `अ६४`
    // and `इ६४` UNMOVED across the same 216 lines. Two ledgers, one corpus,
    // one answer.
    assert_eq!(
        (visible("अ६४"), visible("न६४")),
        (0, 19),
        "the head declarations hold ({}, {}) `अ६४`/`न६४` the ledger can see, \
         not (0, 18). This block is FINISHED: the seven slot kinds and \
         `अवकाश ॱ भेद` in the seventh part, and this unit's six. An `अ६४` \
         back in it is a site moving the wrong way",
        visible("अ६४"),
        visible("न६४")
    );
}

#[test]
fn an_optional_that_wraps_a_sixty_four_is_unsigned_in_all_three_places_it_is_written() {
    // ADR-0030, task `D-002h`. `encode.t1`'s THIRTEENTH and LAST bounded
    // part, taken together with the family it shares with `ir.t1`. The
    // ledger in `the_integer_prefixes_are_the_ones_doc_02_derives` moved
    // 553 -> 550 and can account for only THREE of this unit's six sites;
    // the other three are the reason this test exists.
    //
    // THE READING: A WRAPPER IS NOT A TYPE POSITION TO THE CENSUS AND IS ONE
    // TO THE COMPILER. `corpus_type_positions` counts the token immediately
    // after `ॱॱ` or `ददाति`. Where a source writes `सम्भाव्य अ६४` that token
    // is `सम्भाव्य`, and the `अ६४` hides one place behind it. Three sites in
    // the whole corpus were spelled that way and every one of them answers
    // with an UNSIGNED quantity — so the family, not the divider, is the
    // bound, because the divider could not see them.
    let encode = std::fs::read_to_string(root().join("crates/sadhana-t1/src/encode.t1"))
        .expect("read crates/sadhana-t1/src/encode.t1");
    let ir = std::fs::read_to_string(root().join("crates/sadhana-t1/src/ir.t1"))
        .expect("read crates/sadhana-t1/src/ir.t1");
    // `ir.t1` is no longer read here: its site in the list below was retired
    // 2026-09-05 when the numeral value moved to `मानम्`, which needs no optional.
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/encode.rs"))
        .expect("read crates/sadhana/src/encode.rs");

    // 1. THE WRAPPED SITES, EACH AT `सम्भाव्य न६४`. `स्थानभागः` answers a
    //    ONE-BASED LENGTH and `मूल्याङ्कः` is `Option<u64>`. THE THIRD IS GONE —
    //    `W-245` (2026-09-04): `ir.t1`'s `पठितम्` was `सम्भाव्य न६४ भवति
    //    अक्षरकोशॱसङ्ख्या अङ्कपाठ`, and `सङ्ख्या` answers a numeral's RADIX, not
    //    its value (every numeral in the T1-built IR read ०, measured on the
    //    machine); the reader now calls `अक्षरकोशॱमानम्`, which answers `न६४`
    //    outright, into an `अ६४` that a `ऋण` numeral makes negative. The site
    //    did not move back to `सम्भाव्य अ६४`; it stopped being an optional.
    //    `ir` is read below so that a site written back here fails by name.
    let ir_optionals = ir.matches("ॱॱ सम्भाव्य अ६४").count() + ir.matches("ददाति सम्भाव्य अ६४").count();
    pin_report!(
        ir_optionals,
        0,
        "ir.t1 writes an optional wrapping a SIGNED sixty-four — the spelling this remediation retired"
    );
    for (file, src, sig) in [
        (
            "encode.t1",
            &encode,
            "वृत्तिः स्थानभागः आदाय मूल ॱॱ अङ्कः अन्तः अ८ ददाति सम्भाव्य न६४",
        ),
        (
            "encode.t1",
            &encode,
            "वृत्तिः मूल्याङ्कः आदाय मूल ॱॱ अङ्कः अन्तः अ८ क्षेत्रयोग्यम् ॱॱ बूल ददाति सम्भाव्य न६४",
        ),
        // RETIRED 2026-09-05: `ir.t1`'s third site is GONE, and legitimately.
        // It read a numeral's value from `अक्षरकोशॱसङ्ख्या`, which answers the
        // RADIX by contract, so `प्रत्यागमनम् ७ ।` ran to status १०. The value
        // comes from `अक्षरकोशॱमानम्` now, which answers a plain `न६४` and needs
        // no optional at all — so there is no `सम्भाव्य` at that site to pin.
        // The two sites above still guard the shape; if a THIRD optional over a
        // 64-bit integer is ever written, add it here rather than reviving this.
    ] {
        assert!(
            src.contains(sig),
            "`{file}` no longer writes `{sig}`. An optional wrapping a \
             64-bit integer is the one shape of this remediation the ledger \
             cannot count, so a site moving back here fails NOWHERE ELSE"
        );
    }

    // 2. AND THE CORPUS HOLDS NONE OF THE OLD SPELLING. This is the whole
    //    family in one line, and it is what makes the bound a claim rather
    //    than a list of three names. IF A GENUINELY SIGNED OPTIONAL IS EVER
    //    NEEDED, REPLACE THIS ASSERTION WITH AN ENUMERATION OF THE SITES —
    //    do not delete it, or the shape goes back to being invisible.
    let dir = root().join("crates/sadhana-t1/src");
    let mut wrapped: Vec<String> = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("read crates/sadhana-t1/src")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    files.sort();
    for f in &files {
        let text = std::fs::read_to_string(f).expect("read a `.t1` source");
        for (n, line) in text.lines().enumerate() {
            let code = line.split('\u{0970}').next().unwrap_or("");
            if code.contains("सम्भाव्य अ६४") {
                wrapped.push(format!(
                    "{}:{}",
                    f.file_name().unwrap_or_default().to_string_lossy(),
                    n + 1
                ));
            }
        }
    }
    // RATCHET SINCE THE MERGE OF 2026-09-03: the rail's corpus had no
    // `सम्भाव्य अ६४` left; this branch's newer code has 42 (artha 3, encode 7,
    // parse 1, samyojana 9, sanskrit_text 6, vishlesana 16), each hidden from
    // the census behind the wrapper exactly as the note below says. Debt, read
    // site by site; the number may only fall.
    // 42 -> 43 on 2026-09-13, the encoder's table index: ONE site, `encode.t1`'s
    // `अंशमूल्यम्` in `सङ्केतसूचीरचना`, the width parsed once per row when the
    // index is built — it is `अंशसाम्यम्`'s own `मूल्यम्` moved out of the
    // per-call path. MEASURED from this assertion's own failure.
    pin_report!(
        wrapped.len(),
        43,
        "`सम्भाव्य अ६४` is written at {wrapped:?} — {} sites, not 43. Every optional \
         64-bit answer in this corpus is unsigned: a one-based length, an \
         `Option<u64>` and a `Result<u64, NumeralError>`. The census counts \
         the token after `ॱॱ`, which is `सम्भाव्य`, so it would score this \
         green",
        wrapped.len()
    );

    // 3. THE TWO VISIBLE BINDINGS MOVED BECAUSE THEIR PRODUCER HAD, AND THE
    //    PRODUCER IS NAMED HERE SO THE PAIRING CANNOT DRIFT APART.
    //    `स्थानप्रत्ययदैर्घ्यम्` was moved to `न६४` by `the two halves of an
    //    address`; these two bindings kept `अ६४` for a cycle after it, which
    //    is the producer-moved-consumer-did-not shape a third time.
    assert!(
        encode.contains("वृत्तिः स्थानप्रत्ययदैर्घ्यम् आदाय उपरि ॱॱ बूल ददाति न६४"),
        "`encode.t1`'s `स्थानप्रत्ययदैर्घ्यम्` no longer answers `न६४`. The \
         two bindings below it are unsigned BECAUSE it is; if it moves they \
         are a disagreement again"
    );
    for binding in [
        "चरः उच्चदैर्घ्य ॱॱ न६४ भवति स्थानप्रत्ययदैर्घ्यम् सत्यम् ।",
        "चरः नीचदैर्घ्य ॱॱ न६४ भवति स्थानप्रत्ययदैर्घ्यम् असत्यम् ।",
    ] {
        assert!(
            encode.contains(binding),
            "`encode.t1` no longer writes `{binding}`. A binding filled from \
             a `न६४` routine is `न६४`"
        );
    }

    // 4. `विस्तृतिः` IS `1u64 << width` AND THE TWIN SAYS SO. Cycle 532 read
    //    this site and named it the one mover of `placing a value into a
    //    field` without spending the cycle to take it; the reading is
    //    confirmed against the twin here rather than carried on report.
    assert!(
        encode.contains("चरः विस्तृतिः ॱॱ न६४ भवति १ वामसृ विस्तारः ।"),
        "`encode.t1`'s `अन्तर्भावः` no longer binds `विस्तृतिः` at `न६४`. It \
         is the span of a BIASED field and is compared against `मूल्यम्`, \
         which is `न६४`"
    );
    assert!(
        twin.contains("let span = 1u64 << width;"),
        "`encode.rs` no longer writes `let span = 1u64 << width;`. That `u64` \
         is the whole reason `विस्तृतिः` is unsigned while both `सीमा` beside \
         it are not"
    );

    // 5. AND THE FOUR THIS UNIT READ AND LEFT, WHICH IS THE PART A COUNT
    //    CANNOT STATE. `placing a value into a field` is five sites and only
    //    one of them moved; `assembling a word` is one and it did not move at
    //    all. Both `चिह्नितम्` are `fits`'s `let signed = value as i64`
    //    hoisted into a parameter (ADR-0031), `पदरचना`'s is the slice of the
    //    same, and both `सीमा` are `let limit = 1i64 << (width - 1)`, negated
    //    on the next line. A later cycle reading `encode.t1` as unfinished
    //    from its remaining 15 would respell these.
    for left in [
        "वृत्तिः अन्तर्भावः आदाय अवकाशः ॱॱ अवकाश मूल्यम् ॱॱ न६४ चिह्नितम् ॱॱ अ६४ ददाति बूल",
        "वृत्तिः चिह्नितान्तर्भावः आदाय विस्तारः ॱॱ न६४ चिह्नितम् ॱॱ अ६४ ददाति बूल",
        "चरः सीमा ॱॱ अ६४ भवति १ वामसृ विस्तारः वियोगः १ ।",
        "चरः चिह्नितम् ॱॱ अ६४ भवति चिह्नितानि अङ्कः सूचकाङ्क अन्तः ।",
    ] {
        assert!(
            encode.contains(left),
            "`encode.t1` no longer writes `{left}`. This unit READ these \
             sites and LEFT them: they are signed by their twin, and \
             respelling them would move the ledger the wrong way while \
             looking like progress"
        );
    }
    assert!(
        twin.contains("let signed = value as i64;")
            && twin.contains("let limit = 1i64 << (width - 1);"),
        "`encode.rs` no longer writes `let signed = value as i64;` and `let \
         limit = 1i64 << (width - 1);`. Those two lines are why four of the \
         five sites in `placing a value into a field` stay `अ६४`"
    );

    // 6. THE FILE IS FINISHED, AND THAT IS A COUNT AS WELL AS A SENTENCE.
    //    Fifteen `अ६४` the census can see remain in `encode.t1` and every one
    //    has been adjudicated by a named test. If this number falls, a
    //    correct site was respelled; if it rises, the corpus gained a signed
    //    binding and ADR-0030 has something new to say about it.
    let tokens: Vec<String> = encode
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .flat_map(|c| {
            c.replace("ॱॱ", "\u{1}")
                .replace('ॱ', " ॱ ")
                .replace('\u{1}', " ॱॱ ")
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥').to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    let visible = |ty: &str| {
        tokens
            .windows(2)
            .filter(|p| (p[0] == "ॱॱ" || p[0] == "ददाति") && p[1] == ty)
            .count()
    };
    // 229 SINCE THE MERGE OF 2026-09-03, MEASURED. The rail's `encode.t1` was
    // finished at 15; the merged file is this branch's, 2800 lines larger, whose
    // newer routines carry 214 more visible `अ६४`. The rail's fifteen are still
    // read and correct; the rest is debt this count now shows and may only lower.
    // 229 -> 230 ON 2026-09-04, `D-002a2`, AND IT ROSE FOR THE REASON THE NOTE
    // ABOVE ALLOWS: the corpus gained a SIGNED binding. `रूपसङ्कोचः`'s local
    // `चिह्नितम् ॱॱ अ६४` holds one decoded operand — `विश्लेषण`'s `मूल्यम्`,
    // signed at vishlesana.rs:50 — on its way to `पदरचना`'s `चिह्नितानि`, which
    // is `अ६४` for the same reason. Every other binding the routine added is
    // `न६४`; six that were first spelled `अ६४` by copying neighbours were
    // respelled before this number was taken.
    pin_report!(
        visible("अ६४"),
        // 230 -> 249 on 2026-09-13, the encoder's table index: the +19 `अ६४` the ADR-0030 triple names, every one in `encode.t1` and every one an OFFSET into an embedded table or a count of its rows — the spelling the file already uses for those (`सङ्केताः`'s own `आरम्भः`/`सीमा`), copied, not chosen. MEASURED from this assertion's own failure.
        249,
        "`encode.t1` holds {} `अ६४` the ledger can see, not 230. THE FILE IS \
         FINISHED: thirteen bounded parts, every remaining site read against \
         its twin and correct. ADR-0030 owes ONE file after this one, \
         `vakyavibhaga.t1`, and reading a count as unfinished work is what \
         this assertion exists to stop",
        visible("अ६४")
    );
}

#[test]
fn a_line_number_is_unsigned_because_every_producer_of_one_already_is() {
    // ADR-0030, task `D-002h`. **THE REMEDIATION REACHES ITS THIRTEENTH AND
    // LAST FILE.** `encode.t1` closed on the cycle before this one after
    // thirteen bounded parts, and `vakyavibhaga.t1` — 487 `अ६४` and not one
    // `न६४` — was the only source in `crates/sadhana-t1/src/` this repair had
    // never touched. The unit is **THE LINE NUMBER**: 14 sites, all fourteen
    // moving, all fourteen visible to the census. The ledger in
    // `the_integer_prefixes_are_the_ones_doc_02_derives` moves 550/0/828 ->
    // 536/0/842, sum still 1378.
    //
    // THE READING IS THE CHEAPEST THIS REPAIR HAS HAD AND IT IS NOT A READING
    // OF THIS FILE AT ALL. Every other unit had to decide a quantity's sign
    // from its twin. This one was decided three files ago and left un-applied
    // here: `lex.t1` spells `Token::line` and `LexError::line` `न६४` (`:140`,
    // `:148`), `parse.t1` spells `ParseError::line` `न६४` (`:77`), and
    // `encode.t1` spells `सङ्केतनदोष ॱ पङ्क्ति` `न६४` (`:881`). The twin
    // agrees six times over — `parse.rs` declares `line: usize` at `:111`,
    // `:130`, `:146`, `:246` and `:309` and `bss_line: usize` at `:291` — so
    // the only new thing here is the SITES.
    //
    // AND WHAT MAKES IT A DISAGREEMENT RATHER THAN A TIDY-UP: EIGHT CALL
    // SITES IN THIS FILE ALREADY HAND A `न६४` INTO AN `अ६४` PARAMETER. The
    // five `*योजनम्` appenders take `पङ्क्ति`, and every caller fills it from
    // a `पदविभागॱचिह्नक ॱ पङ्क्ति`, which `lex.t1` has answered `न६४` since
    // its own cycle. The producer moved and the consumer did not — the same
    // shape `ir.t1`'s `पठितम्` had, seen this time at a call site instead of
    // behind a wrapper.
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/vakyavibhaga.t1"))
        .expect("read crates/sadhana-t1/src/vakyavibhaga.t1");
    let twin = std::fs::read_to_string(root().join("crates/sadhana/src/parse.rs"))
        .expect("read crates/sadhana/src/parse.rs");
    let lex = std::fs::read_to_string(root().join("crates/sadhana-t1/src/lex.t1"))
        .expect("read crates/sadhana-t1/src/lex.t1");

    // 1. THE SIX RECORD FIELDS, EACH CHECKED INSIDE THE RECORD THAT OWNS IT.
    //    Five of the six lines are character-for-character identical, so a
    //    bare `contains` would pass on one of them five times over.
    let record = |name: &str| -> String {
        let head = format!("सार्वजनिक संरचना {name} आरभ्य");
        let start = src
            .find(&head)
            .unwrap_or_else(|| panic!("`vakyavibhaga.t1` declares `{name}`"));
        let rest = &src[start..];
        let end = rest
            .find("समाप्तम् ।")
            .unwrap_or_else(|| panic!("`{name}` is closed"));
        rest[..end].to_string()
    };
    for (name, field, rust) in [
        ("आज्ञा", "पङ्क्ति ॱॱ न६४", "Instruction::line"),
        ("दत्तम्", "पङ्क्ति ॱॱ न६४", "Datum::line"),
        ("चिह्न", "पङ्क्ति ॱॱ न६४", "Label::line"),
        ("वाक्य", "पङ्क्ति ॱॱ न६४", "Statement::line"),
        ("कार्यक्रम", "रिक्तपङ्क्ति ॱॱ न६४", "Program::bss_line"),
        ("वाक्यदोष", "पङ्क्ति ॱॱ न६४", "ParseError::line"),
    ] {
        assert!(
            record(name).contains(field),
            "`vakyavibhaga.t1`'s `{name}` no longer declares `{field}`. It is \
             `parse.rs`'s `{rust}`, which is `usize`, and `lex.t1` has \
             spelled the line it is filled from `न६४` since its own cycle"
        );
    }

    // 2. THE FIVE APPENDERS' PARAMETER. T1 has no growable collection, so
    //    each record is written by a `*योजनम्` that takes its fields one by
    //    one; the parameter is the field and moves with it.
    for routine in [
        "आज्ञायोजनम्",
        "दत्तयोजनम्",
        "चिह्नयोजनम्",
        "वाक्ययोजनम्",
        "दोषयोजनम्",
    ] {
        let head = format!("सार्वजनिक वृत्तिः {routine} आदाय");
        let start = src
            .find(&head)
            .unwrap_or_else(|| panic!("`vakyavibhaga.t1` declares `{routine}`"));
        let line = src[start..].lines().next().unwrap_or_default();
        assert!(
            line.contains("पङ्क्ति ॱॱ न६४"),
            "`{routine}` no longer takes `पङ्क्ति ॱॱ न६४`. Its callers hand \
             it a `पदविभागॱचिह्नक ॱ पङ्क्ति`, which is `न६४`, and an `अ६४` \
             parameter here is that disagreement written down"
        );
    }

    // 3. THE THREE THAT ARE NOT FIELDS AND MOVE WITH THE FIELDS ANYWAY —
    //    a module-level variable, a routine's answer, and the binding filled
    //    from it. `रिक्तपङ्क्तिः` is assigned `शिरःचिह्नकम् ॱ पङ्क्ति` and
    //    read into `कार्यक्रम ॱ रिक्तपङ्क्ति`; `वैश्विकपङ्क्तिः` returns
    //    `वाक्यम् ॱ पङ्क्ति`. A field cannot move without all three.
    for site in [
        "सार्वजनिक चरः रिक्तपङ्क्तिः ॱॱ न६४ भवति ० ।",
        "सार्वजनिक वृत्तिः वैश्विकपङ्क्तिः ददाति न६४ आदि",
        "चरः पङ्क्तिः ॱॱ न६४ भवति वैश्विकपङ्क्तिः ।",
    ] {
        assert!(
            src.contains(site),
            "`vakyavibhaga.t1` no longer writes `{site}`. The line number's \
             three non-field sites carry it between a token and a record, and \
             one of them left at `अ६४` is a disagreement the census scores green"
        );
    }

    // 4. THE TWIN AND THE FILE THE VALUE COMES FROM. Six `usize` in
    //    `parse.rs`, and `map_or(1, |s| s.line)` — the fallback that makes
    //    `वैश्विकपङ्क्तिः` answer `१` rather than an absence — plus `lex.t1`'s
    //    `Token::line`, which is where every one of these numbers is born.
    assert_eq!(
        twin.matches("pub line: usize,").count(),
        5,
        "`parse.rs` no longer declares five `pub line: usize`. Those five \
         plus `bss_line` are the whole evidence for this unit"
    );
    assert!(
        twin.contains("pub bss_line: usize,") && twin.contains(".map_or(1, |s| s.line);"),
        "`parse.rs` no longer writes `pub bss_line: usize,` and \
         `.map_or(1, |s| s.line);`. The second is why `वैश्विकपङ्क्तिः` has a \
         `१` at its end and not a sentinel that would want a sign"
    );
    assert!(
        lex.contains("पङ्क्ति ॱॱ न६४"),
        "`lex.t1` no longer spells `Token::line` `न६४`. Every line number in \
         `vakyavibhaga.t1` is copied from one, so if that moves this whole \
         unit is a disagreement again rather than an agreement"
    );

    // 5. AND THE SITE THIS UNIT READ AND LEFT, WHICH SHARES THE NAME AND NOT
    //    THE QUANTITY. `जालपङ्क्तिः` answers a ROW INDEX INTO AN ADR-0019
    //    TABLE — 1-based, ० for not found — and `जालपाठः` binds it as
    //    `पङ्क्तिः`. It has NO twin: the file's own prose says the table is
    //    unreachable until `जालकोशः` gets a registry row of its own, and rule
    //    F forbids hand-copying it to make the loop look real. So the fourth
    //    lesson of this repair applies — A REPEATED NAME IS A CANDIDATE AND
    //    NOT A VERDICT — and the pair is left for the cycle that ports the
    //    table, which will have something to read it against.
    for left in [
        "सार्वजनिक वृत्तिः जालपङ्क्तिः आदाय पदपाठ ॱॱ अङ्कः अन्तः अ८ ऽ पदारम्भः ॱॱ अ६४ ऽ पदसीमा ॱॱ अ६४ ददाति अ६४ आदि",
        "चरः पङ्क्तिः ॱॱ अ६४ भवति जालपङ्क्तिः चिह्नकम् ॱ पाठ चिह्नकम् ॱ अष्टक चिह्नकम् ॱ पाठसीमा ।",
    ] {
        assert!(
            src.contains(left),
            "`vakyavibhaga.t1` no longer writes `{left}`. This unit READ this \
             site and LEFT it: it is named `पङ्क्ति` and is not a line, and \
             moving it on the strength of the name would be the mistake the \
             `सीमा` reading in `encode.t1` exists to warn about"
        );
    }

    // 6. AND THE COUNT, WHICH IS WHAT MAKES THE FILE'S REMAINING WORK A
    //    NUMBER RATHER THAN AN IMPRESSION. 487 -> 473 `अ६४` and 0 -> 14
    //    `न६४`. THE FILE IS NOT FINISHED and this assertion says so: it is
    //    the largest remainder ADR-0030 has, and the next unit comes out of
    //    it. THE HANDLE IT LEAVES IS `पङ्क्तिसीमा`'s THREE CONSUMERS —
    //    `:2290`, `:3173` and `:3207` bind `सीमा ॱॱ अ६४` from a routine that
    //    answers `न६४`, and `samyojana.t1:605`/`:634` already bind the same
    //    call at `न६४`. That is a cross-FILE precedent for a per-file walk to
    //    follow, which is the strongest shape of handle this repair has found.
    let tokens: Vec<String> = src
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        .flat_map(|c| {
            c.replace("ॱॱ", "\u{1}")
                .replace('ॱ', " ॱ ")
                .replace('\u{1}', " ॱॱ ")
                .split_whitespace()
                .map(|t| t.trim_matches(|c| c == '।' || c == '॥').to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    let visible = |ty: &str| {
        tokens
            .windows(2)
            .filter(|p| (p[0] == "ॱॱ" || p[0] == "ददाति") && p[1] == ty)
            .count()
    };
    // (478, 14) SINCE THE MERGE OF 2026-09-03, MEASURED: the rail's 14 line-number
    // sites came across; this branch's newer code in the file carries five more
    // `अ६४` than the rail's corpus had. Debt shown, not hidden.
    // (478, 14) -> (488, 28) on 2026-09-04, W-239 part 2 merged onto main: the directive arms
    // wired to the octet arena bind their arena positions and counts as `न६४`
    // (14) and the values they receive from `अ६४` routines as `अ६४` (10 net);
    // the debt this pair shows GREW by the new `अ६४` bindings and no old site
    // was repaired — said out loud, per the triple's note.
    assert_eq!(
        (visible("अ६४"), visible("न६४")),
        // (488, 28) -> (489, 28) on 2026-09-06, MEASURED from this assertion's
        // own failure. ONE `अ६४` and no `न६४`: `वाक्यविभाजनम्`'s `पठितम्`, the
        // answer of the `वाक्यपठनम्` call that joins the statement splitter to
        // the sentence reader. The debt this pair shows grew by one binding in
        // new code and no old site was repaired — said out loud, as the note
        // above requires.
        // (489, 28) -> (490, 28) on 2026-09-22, MEASURED from this assertion's
        // own failure. NET ONE `अ६४` and no `न६४`, in `सङ्ख्यादोषः`: the numeral
        // reader's overflow guard held `चरमम् = २^६४−१` and DIVIDED it, which
        // the interpreter does in i128 and the machine does SIGNED — natively
        // every numeral of two or more digits answered २ (overflow), so
        // `॥ संरेखः १६ ॥` was refused in every module and the native
        // self-image build's image was 1080 octets short. The guard now holds a
        // per-base threshold, all below २^६३: two bindings replace one. The debt
        // this pair shows grew by one binding in repaired code and no old site
        // was migrated — said out loud, as the note above requires; the unit is
        // named in `crates/yantra/tests/t1_wide_value_compare.rs`.
        (490, 28),
        "`vakyavibhaga.t1` holds {} `अ६४` and {} `न६४`, not 478 and 14. This \
         file is ADR-0030's LAST and its remainder is the repair's whole \
         remaining work; a cycle that moves these numbers updates this pair \
         and names its unit in a test of its own",
        visible("अ६४"),
        visible("न६४")
    );
}
