//! **The module system, held against the corpus.** ADR-0027, task `D-002l`.
//!
//! `spec/grammar-t1.ebnf` gained `मण्डलम्` and three productions; this file is
//! the check that the productions describe the tree that already writes them,
//! and that `crates/sadhana/src/t1/mandala.rs` reads it the way the grammar says.
//!
//! # Why the acceptance was named files and not a count
//!
//! Modules reached WITHOUT being imported were found by reading, before the
//! resolver existed — `nidana.t1` → `वाक्यविभाग`, `vakyavibhaga.t1` →
//! `सङ्केतन` and → `अक्षरकोश` — and each file has since gained its import
//! (`W-190` the first two, `W-204` the last). The table is EMPTY now, so the
//! acceptance is a zero, and a zero can be vacuous: reporting none means it
//! never read a qualified name, reporting the whole corpus means it never read
//! an import. That is why a positive control sits beside the acceptance.

use sadhana::t1::mandala::{self, Spelling, Unit};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.t1` source under `crates/`, from every crate, sorted — the same set
/// `no_t1_source_binds_a_frozen_keyword_as_a_name` guards, for the same reason:
/// `crates/textapp/src/text/*.t1` are T1 sources and declare modules too.
fn every_t1_source() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "t1") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&repo_root().join("crates"), &mut v);
    v.sort();
    v
}

/// A repo-relative path with `/` on every platform, so a report reads the way a
/// person would write it.
fn rel(p: &Path) -> String {
    p.strip_prefix(repo_root())
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// **T1 sources the T1 LEXER cannot read whole**, `(path, refused lines, first)`.
///
/// FOUND BY THIS ROW AND NOT FIXED BY IT: the files belong to other lanes and the
/// defects are language defects, not module ones. They are recorded rather than
/// worked around silently, because `every_t1_source_lexes_and_parses` in
/// `t1_sources.rs` **cannot see them** — that test lexes with
/// `sadhana::lex::lex`, the **T0** lexer, where a string is a phrase of ordinary
/// words, an unclosed one is not an error, and the repertoire check runs over a
/// different set of pieces. This matters to `D-002f9`: a `.t1` source the T1
/// lexer cannot read cannot reach ELF through T1.
///
/// Two distinct defects:
///
/// - `crates/sadhana-t1/src/parse.t1:198` writes ADR-0011's escape one `इति`
///   short. A doubled close is the WORD, so the literal string `इति` is
///   `उक्तम् इति इति इति`; the line writes `उक्तम् इति इति समाप्तम्`, which opens
///   a string, spends the escape and runs off the end of the line.
/// - the four `crates/textapp/src/text/` files are machine transliterations
///   that still carry `( ) = :: ऽ` and LATIN digits (`उ32`), none of which is in
///   the doc 15 repertoire.
///
/// EXACT, not a ceiling.
const SOURCES_THE_T1_LEXER_CANNOT_READ_WHOLE: &[(&str, usize, usize)] = &[
    // `crates/sadhana-t1/src/parse.t1` (2, 198) WAS HERE AND IS NOW READABLE.
    // It is removed rather than left, exactly as the assertion below instructs:
    // "if one was FIXED, lower its row in the same commit". `D-002h`'s operator
    // reconciliation landed in `57c27a13` after this list was written, and it
    // is what made the line lex. The list is a RATCHET on the wrong side —
    // a T1 source the T1 lexer cannot read is the thing self-hosting cannot
    // survive — so every removal is progress and every addition is a defect.
    ("crates/textapp/src/text/nfc.t1", 1, 5),
    ("crates/textapp/src/text/numeral.t1", 4, 4),
    ("crates/textapp/src/text/segment.t1", 4, 5),
    ("crates/textapp/src/text/tables.t1", 5607, 32),
];

/// Every `.t1` source read into a [`Unit`], keyed by its repo-relative path,
/// with each source's refused lines beside it.
///
/// **`lex_t1`, not `lex`** — `mandala::read` says why, and
/// `the_import_keyword_inside_a_string_is_not_an_import` proves it against the
/// source that would break. **`read_lossy`, not `read`** — dropping a whole file
/// over one bad line deletes a MODULE from the set, and then every import of it
/// reads as unknown and every reach into it as a field.
type Units = Vec<(String, Unit)>;
/// `(path, how many lines were refused, the first of them)`.
type Refusals = Vec<(String, usize, usize)>;

fn read_corpus() -> (Units, Refusals) {
    let mut units = Vec::new();
    let mut refused = Vec::new();
    for p in every_t1_source() {
        let text = std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()));
        let (unit, errs) = mandala::read_lossy(&text);
        if !errs.is_empty() {
            let mut lines: Vec<usize> = errs.iter().map(|e| e.line).collect();
            lines.dedup();
            refused.push((rel(&p), lines.len(), lines[0]));
        }
        units.push((rel(&p), unit));
    }
    (units, refused)
}

fn corpus() -> Vec<(String, Unit)> {
    read_corpus().0
}

/// **Exactly the recorded sources have lines the T1 lexer refuses.**
#[test]
fn the_only_t1_lines_the_t1_lexer_refuses_are_the_ones_recorded() {
    let (units, refused) = read_corpus();
    println!(
        "METRIC sadhana_t1_sources_with_lines_lex_t1_refuses {}",
        refused.len()
    );
    for (p, count, first) in &refused {
        println!("  {p}: {count} line(s), first at :{first}");
    }

    let expected: Vec<(String, usize, usize)> = SOURCES_THE_T1_LEXER_CANNOT_READ_WHOLE
        .iter()
        .map(|(p, c, f)| ((*p).to_string(), *c, *f))
        .collect();
    assert_eq!(
        refused, expected,
        "the set of .t1 lines `lex_t1` refuses changed. If one was FIXED, lower \
         its row in SOURCES_THE_T1_LEXER_CANNOT_READ_WHOLE in the same commit; \
         if one was ADDED, fix the source — a T1 source the T1 lexer cannot read \
         is the thing self-hosting cannot survive."
    );
    assert!(
        !units.is_empty(),
        "no .t1 source was read; every check in this file is vacuous"
    );
}

/// **The line-by-line fallback loses nothing on this corpus.**
///
/// `read_lossy` is only sound because a string does not cross a newline. Every
/// source that lexes WHOLE is read both ways here and the two must agree, so a
/// module construct written across a newline — which the fallback would miss —
/// fails this by name instead of quietly shrinking the report.
#[test]
fn line_by_line_reading_agrees_with_whole_file_reading() {
    let mut compared = 0usize;
    for p in every_t1_source() {
        let text = std::fs::read_to_string(&p).expect("readable");
        let Ok(whole) = mandala::read(&text) else {
            continue;
        };
        let (lossy, errs) = mandala::read_lossy(&text);
        assert!(
            errs.is_empty(),
            "{}: whole-file lex succeeded but line-by-line did not",
            rel(&p)
        );
        assert_eq!(
            whole,
            lossy,
            "{}: reading line by line gives different module facts from reading \
             the file whole — a module construct is written across a newline",
            rel(&p)
        );
        compared += 1;
    }
    println!("METRIC sadhana_t1_sources_read_both_ways {compared}");
    assert!(
        compared >= 14,
        "only {compared} sources lex whole; this agreement check is nearly vacuous"
    );
}

// ── the grammar half ──────────────────────────────────────────────────────

/// Drop every `(* … *)` comment. ISO EBNF comments nest, so the depth counts.
fn strip_ebnf_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '(' && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            i += 2;
            continue;
        }
        if depth > 0 && chars[i] == '*' && chars.get(i + 1) == Some(&')') {
            depth -= 1;
            i += 2;
            continue;
        }
        if depth == 0 || chars[i] == '\n' {
            out.push(chars[i]);
        }
        i += 1;
    }
    out
}

/// The body of one production of `spec/grammar-t1.ebnf`, comments removed.
///
/// A near-copy of `t1_sources.rs`'s reader, and deliberately not shared: an
/// integration test is its own binary, and a helper crate to hold thirty lines
/// would be a third place for the grammar's shape to drift. Both readers parse
/// THE GRAMMAR rather than a copy of it, which is the property that matters.
fn production(name: &str) -> Option<String> {
    let path = repo_root().join("spec/grammar-t1.ebnf");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("spec/grammar-t1.ebnf must be readable: {e}"));
    let bare = strip_ebnf_comments(&src);

    let mut body = String::new();
    let mut inside = false;
    for line in bare.lines() {
        if !inside {
            if !line
                .strip_prefix(name)
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
            {
                continue;
            }
            inside = true;
        }
        match line.find(';') {
            Some(end) => {
                body.push_str(&line[..end]);
                return Some(body);
            }
            None => {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    None
}

/// The quoted terminals of one production, in order.
fn terminals(name: &str) -> Vec<String> {
    production(name)
        .unwrap_or_else(|| panic!("spec/grammar-t1.ebnf has no `{name}` production"))
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// **The grammar carries the declaring half.** ADR-0027's whole content.
///
/// Read out of `spec/grammar-t1.ebnf` rather than compared against a copy, so
/// deleting the production fails here instead of quietly making
/// `mandala::MODULE` a word this language does not have.
#[test]
fn the_grammar_declares_a_module_and_this_test_reads_it_from_the_grammar() {
    let keywords = terminals("keyword");
    assert!(
        keywords.len() >= 30,
        "only {} keywords parsed out of the `keyword` production; the parse is \
         broken and everything below is vacuous",
        keywords.len()
    );
    assert!(
        keywords.iter().any(|k| k == mandala::MODULE),
        "`{}` is not in the frozen `keyword` production of spec/grammar-t1.ebnf; \
         ADR-0027 adds it and `mandala` reads it",
        mandala::MODULE
    );
    // The import half was frozen at the freeze. If it ever leaves, the module
    // system has one half again and this says which.
    assert!(
        keywords.iter().any(|k| k == mandala::IMPORT),
        "`{}` is no longer a frozen keyword",
        mandala::IMPORT
    );

    let module_decl = production("module_decl").expect("`module_decl` production");
    assert_eq!(
        terminals("module_decl"),
        vec![mandala::MODULE.to_string()],
        "`module_decl` no longer opens with exactly `{}`",
        mandala::MODULE
    );
    assert!(
        module_decl.contains("identifier") && module_decl.contains("double_danda"),
        "`module_decl` is not `मण्डलम् , identifier , double_danda`: {module_decl}"
    );

    let import = production("import").expect("`import` production");
    assert_eq!(
        terminals("import"),
        vec![mandala::IMPORT.to_string()],
        "`import` no longer opens with exactly `{}`",
        mandala::IMPORT
    );
    assert!(
        import.contains("identifier") && import.contains("danda"),
        "`import` is not `आयातः , identifier , danda`: {import}"
    );
    assert!(
        production("qualified_name").is_some_and(|b| b.contains("member_mark")),
        "`qualified_name` does not reach through the member mark"
    );
}

// ── what the corpus actually contains ────────────────────────────────────

/// **Not vacuous, and the numbers are the report.**
///
/// Every count below is printed as a METRIC and asserted only as a floor, with
/// one exception: the split between the two SPELLINGS is asserted both ways,
/// because a reader that handles one spelling and not the other still passes a
/// floor on the total.
#[test]
fn the_corpus_declares_modules_and_reaches_across_them() {
    let units = corpus();
    let declared: Vec<&str> = units
        .iter()
        .filter_map(|(_, u)| u.module.as_ref().map(|m| m.text.as_str()))
        .collect();

    let imports: usize = units.iter().map(|(_, u)| u.imports.len()).sum();
    let qualified: Vec<&mandala::Access> = units
        .iter()
        .flat_map(|(_, u)| {
            u.accesses
                .iter()
                .filter(|a| declared.iter().any(|m| *m == a.head))
        })
        .collect();
    let unspaced = qualified
        .iter()
        .filter(|a| a.spelling == Spelling::Unspaced)
        .count();
    let spaced = qualified
        .iter()
        .filter(|a| a.spelling == Spelling::Spaced)
        .count();

    println!("METRIC sadhana_t1_module_sources {}", units.len());
    println!("METRIC sadhana_t1_modules_declared {}", declared.len());
    println!("METRIC sadhana_t1_module_imports {imports}");
    println!("METRIC sadhana_t1_qualified_names {}", qualified.len());
    println!("METRIC sadhana_t1_qualified_names_unspaced {unspaced}");
    println!("METRIC sadhana_t1_qualified_names_spaced {spaced}");

    assert!(
        units.len() >= 20,
        "only {} .t1 sources found; this file has almost nothing to read",
        units.len()
    );
    assert!(
        declared.len() >= 19,
        "only {} of {} sources declare a module: {declared:?}",
        declared.len(),
        units.len()
    );
    assert!(
        imports >= 20,
        "only {imports} imports across the corpus; the import half was already \
         frozen and in use, so this floor should not be reachable"
    );

    // THE POINT OF THE ROW. The premise this row was filed under said there was
    // no cross-module call anywhere in the corpus. There are hundreds, and the
    // spelling is why they were missed: a grep for the SPACED form finds a
    // seventh of them.
    assert!(
        qualified.len() >= 250,
        "only {} qualified names into declared modules; the corpus has hundreds",
        qualified.len()
    );
    assert!(
        unspaced >= 200,
        "only {unspaced} UNSPACED qualified names — `सङ्केतनॱस्थापनम्` is the \
         majority spelling and a reader that misses it misses the corpus"
    );
    assert!(
        spaced >= 25,
        "only {spaced} SPACED qualified names — `सङ्केतन ॱ पङ्क्तिसीमा` is the \
         minority spelling and a reader that misses it misses a seventh"
    );
}

// ── the acceptance ────────────────────────────────────────────────────────

/// The reaches into a module the file never imports.
///
/// TWO ROWS WERE REMOVED TOGETHER, both by `W-190`: `nidana.t1` → `वाक्यविभाग`
/// and `vakyavibhaga.t1` → `सङ्केतन`. One commit gave both files the import
/// they had always needed, and neither row was removed with it.
///
/// `nidana.t1` → `वाक्यविभाग` WAS HERE AND IS GONE — `W-190` gave that file the
/// `आयातः वाक्यविभाग ।` it had always needed, which is what this table's own
/// assertion instructs ("if a file gained an `आयातः`, remove its row above in
/// the same commit"). It was NOT removed in the same commit: `W-190`'s gate ran
/// `t1_execution`, `t1_sources`, `grammar_t1` and `metrics` but never
/// `t1_modules`, so that commit landed RED and stayed red until the next full
/// crate run found it. Running selected TEST TARGETS is not running the crate.
///
/// `(source, module, the lines that reach it)`. The lines are here so that
/// moving the code without fixing the import fails LOUDLY rather than silently
/// staying true.
const REACHED_WITHOUT_IMPORTING: &[(&str, &str, &[usize])] = &[
    // `vakyavibhaga.t1` → `अक्षरकोश` WAS HERE AND IS GONE — `W-204`
    // (2026-09-04) gave that file the `आयातः अक्षरकोश ।` it had always needed,
    // at the eight sites this row carried (3170, 3175, 3176, 3180, 3181, 3186,
    // 3193, 3198, inside one `अन्यथा` block opening at 3167 — which is why no
    // resolver had seen them: the resolver did not walk else bodies, W-202's
    // row). Removed IN THE SAME COMMIT as the import, as this constant's own
    // assertion instructs, and as `W-190` did not. The table is now EMPTY and
    // the test below says so by name.
];

/// **A ZERO THAT CANNOT BE VACUOUS** — the positive control for the acceptance
/// below. Added 2026-09-04 with `W-202`, when that acceptance reached zero.
///
/// This file's own header states the trap: "reporting none means it never read
/// a qualified name". While `REACHED_WITHOUT_IMPORTING` held rows, the
/// assertion below could not pass on a blind `missing_imports` — it had to name
/// the right file, module and eight line numbers. **The table is now empty, and
/// that protection is gone with it**: an implementation that returns nothing
/// for any reason at all, including being broken, satisfies it perfectly.
///
/// So the detector is shown a reach it MUST find. Two synthetic units, one
/// declaring `ख` and one reaching `खॱनाम` without importing it, and the same
/// pair again with the import present. If `missing_imports` ever goes blind,
/// the first half fails here rather than being read as progress there.
#[test]
fn missing_imports_still_finds_a_reach_that_has_no_import() {
    let provider =
        mandala::read("मण्डलम् ख ॥\nसार्वजनिक वृत्तिः नाम ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n")
            .expect("the provider unit must read");

    let without =
        mandala::read("मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् खॱनाम ।\nइति\n")
            .expect("the reaching unit must read");
    let found = mandala::missing_imports(&[
        ("provider.t1".to_string(), provider.clone()),
        ("consumer.t1".to_string(), without),
    ]);
    assert_eq!(
        found.len(),
        1,
        "a unit that reaches `ख` without importing it MUST be reported. If this \
         is 0 the detector is blind, and the empty table below is measuring \
         nothing rather than recording that nothing is wrong"
    );
    assert_eq!(found[0].module, "ख");

    // The other direction, so the control cannot pass by always reporting.
    let with = mandala::read(
        "मण्डलम् क ॥\nआयातः ख ।\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् खॱनाम ।\nइति\n",
    )
    .expect("the importing unit must read");
    let none = mandala::missing_imports(&[
        ("provider.t1".to_string(), provider),
        ("consumer.t1".to_string(), with),
    ]);
    assert!(
        none.is_empty(),
        "with the import present nothing is missing; got {none:?}"
    );
}

/// **The acceptance.** NO module is reached without being imported — the table
/// above is empty since `W-204`, and a new reach adds an import, not a row.
#[test]
fn no_module_is_reached_without_being_imported() {
    let units = corpus();
    let found = mandala::missing_imports(&units);

    let rendered: Vec<String> = found
        .iter()
        .map(|m| format!("{} reaches {} at {:?}", m.unit, m.module, m.sites))
        .collect();
    println!("METRIC sadhana_t1_missing_imports {}", found.len());
    for line in &rendered {
        println!("  {line}");
    }

    let expected: Vec<String> = REACHED_WITHOUT_IMPORTING
        .iter()
        .map(|(unit, module, sites)| format!("{unit} reaches {module} at {sites:?}"))
        .collect();

    assert_eq!(
        rendered, expected,
        "the set of modules reached without being imported changed. If a file \
         gained an `आयातः`, remove its row above in the same commit; if a file \
         gained a cross-module reference without one, add the import rather \
         than the row."
    );
}

// ── IMPORT ⇒ LOADED ────────────────────────────────────────────────────────
//
// THE SIBLING ABOVE CHECKS REACH ⇒ IMPORT AND CANNOT SEE THIS. Both guards are
// about modules and they answer different questions:
//
//     REACH  ⇒ IMPORT    does the source that USES a module say `आयातः`?
//     IMPORT ⇒ LOADED    does the RUST MANIFEST carry the module it names?
//
// `encode.t1` imported `निदान` correctly for as long as `CHAIN` did not carry
// `nidana.t1`, so the first guard passed — truthfully — over a production chain
// holding a reachable call into a module that was not there. It surfaced as
// nothing at all, because the call sits on the encoder's ERROR path.
//
// WHY IT WAS INVISIBLE, AND WHY THIS GUARD IS SHAPED THIS WAY: every other
// module guard in this tree compares `.t1` against `.t1` (this file's corpus
// walks) or Rust against Rust. THIS ONE CROSSES THE BOUNDARY — imports DERIVED
// from `.t1` sources against a manifest DECLARED in Rust — and the hole lived
// exactly there, unseeable from either side alone. It is the second boundary
// defect found in one night, after the emitter's globals marshalling, which is
// the argument for watching the boundary rather than either half of it.

/// Modules a loaded unit imports that no loaded unit declares.
///
/// **THE CLOSURE IS IN THE QUANTIFIER, NOT IN AN ITERATION.** A one-level check
/// — "does the driver reach what it calls" — goes green on a manifest that is
/// still incomplete, because the module it just admitted may import a third
/// that is absent. Quantifying over EVERY loaded unit is the fixed point: any
/// module added to the manifest is itself checked, so there is nothing left to
/// iterate toward.
///
/// Takes the units rather than reading `CHAIN` so the control below can drive
/// this exact code path with a corpus it constructs.
fn unloaded_imports(loaded: &[(String, Unit)]) -> Vec<(String, String)> {
    let declared: std::collections::BTreeSet<&str> = loaded
        .iter()
        .filter_map(|(_, u)| u.module.as_ref().map(|m| m.text.as_str()))
        .collect();
    let mut out: Vec<(String, String)> = loaded
        .iter()
        .flat_map(|(file, unit)| {
            unit.imports
                .iter()
                .filter(|i| !declared.contains(i.name.text.as_str()))
                .map(move |i| (file.clone(), i.name.text.clone()))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The control, and it shares the shape it clears — a unit that imports a
/// module no unit declares, run through the same function as the corpus.
///
/// Both directions, so the control cannot pass by always reporting.
#[test]
fn a_loaded_module_importing_an_unloaded_one_is_reported() {
    let consumer = mandala::read("मण्डलम् ग ॥\nआयातः ख ।\n").expect("the consumer reads");
    let unrelated = mandala::read("मण्डलम् क ॥\n").expect("the unrelated unit reads");

    let found = unloaded_imports(&[
        ("unrelated.t1".to_string(), unrelated),
        ("consumer.t1".to_string(), consumer),
    ]);
    assert_eq!(
        found.len(),
        1,
        "a unit importing `ख` when no unit declares it MUST be reported. If this \
         is 0 the detector is blind and the acceptance below is measuring \
         nothing rather than recording that nothing is wrong; got {found:?}"
    );
    assert_eq!(found[0].1, "ख");

    // The other direction: with a unit declaring `ख` present, nothing is missing.
    let consumer = mandala::read("मण्डलम् ग ॥\nआयातः ख ।\n").expect("the consumer reads");
    let provider = mandala::read("मण्डलम् ख ॥\n").expect("the provider reads");
    let none = unloaded_imports(&[
        ("provider.t1".to_string(), provider),
        ("consumer.t1".to_string(), consumer),
    ]);
    assert!(
        none.is_empty(),
        "with the provider loaded nothing is missing; got {none:?}"
    );
}

/// **THE ACCEPTANCE.** Every module imported by a module the shipped `CHAIN`
/// loads is itself loaded by that `CHAIN`.
///
/// Reads the SHIPPED manifest, never a copy. A second list beside the real one
/// can go green while the real one is missing a module, which is the exact
/// failure this exists to catch.
#[test]
fn every_module_imported_by_a_loaded_module_is_itself_loaded() {
    // `read_lossy`, not `read`: some corpus sources carry lines the T1 lexer
    // refuses by design (see the recorded set above), and `read` would fail the
    // guard on a source whose imports it can parse perfectly well.
    let loaded: Vec<(String, Unit)> = sadhana::t1::chain::CHAIN
        .iter()
        .map(|(name, text)| ((*name).to_string(), mandala::read_lossy(text).0))
        .collect();

    // NON-VACUITY. An empty manifest, or one whose units declare nothing, would
    // report zero missing imports and read exactly like a clean corpus.
    let declaring = loaded.iter().filter(|(_, u)| u.module.is_some()).count();
    let importing: usize = loaded.iter().map(|(_, u)| u.imports.len()).sum();
    println!("METRIC sadhana_t1_chain_modules {}", loaded.len());
    println!("METRIC sadhana_t1_chain_imports {importing}");
    assert!(
        loaded.len() >= 15 && declaring >= 15 && importing >= 15,
        "the manifest went thin — {} entries, {declaring} declaring a module, \
         {importing} imports. A guard over an empty corpus passes vacuously.",
        loaded.len()
    );

    let missing = unloaded_imports(&loaded);
    let rendered: Vec<String> = missing
        .iter()
        .map(|(file, module)| format!("{file} imports {module}, which CHAIN does not load"))
        .collect();
    for line in &rendered {
        println!("  {line}");
    }
    assert!(
        rendered.is_empty(),
        "a module the chain loads imports a module the chain does NOT load. The \
         call will resolve at CALL time and fault there, which is why this can \
         sit on an error path unnoticed — `निदान` did, for as long as \
         `encode.t1` imported it and `CHAIN` omitted it. Add the module to \
         `CHAIN` rather than removing the import: {rendered:?}"
    );
}

/// The false positive that was found before the resolver was written.
///
/// `crates/sadhana-t1/src/samyojana.t1:74` is `सारणी ॱॱ अङ्कः अन्तः कोशॱसंज्ञा`
/// — the FIELD `सारणी` followed by ADR-0003's annotation mark. `सारणी` is ALSO a
/// declared module, so a reader that takes the first half of `ॱॱ` for a member
/// mark reports a FOURTH missing import out of a colon.
///
/// This test asserts both halves of the trap, so it cannot go stale silently:
/// that `सारणी` really is a declared module, and that `samyojana.t1` really
/// writes it before the annotation mark.
#[test]
fn a_field_named_like_a_module_is_not_a_reach_into_that_module() {
    let units = corpus();

    assert!(
        units.iter().any(|(_, u)| u.declares_module("सारणी")),
        "no source declares module `सारणी` any more; this trap is gone and the \
         test that guards it is vacuous"
    );

    let samyojana = units
        .iter()
        .find(|(p, _)| p.ends_with("samyojana.t1"))
        .expect("crates/sadhana-t1/src/samyojana.t1 exists");
    let text = std::fs::read_to_string(repo_root().join(&samyojana.0)).expect("readable");
    assert!(
        text.lines().any(|l| l
            .split_whitespace()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| { w[0] == "सारणी" && w[1] == mandala::ANNOTATION_MARK })),
        "samyojana.t1 no longer writes `सारणी ॱॱ`; the trap this test guards is \
         gone and it now asserts nothing"
    );

    assert!(
        !samyojana.1.accesses.iter().any(|a| a.head == "सारणी"),
        "the annotation mark `ॱॱ` was read as the member mark `ॱ`: samyojana.t1 \
         reports a reach into module `सारणी` that is a field and a colon"
    );
}

/// `उक्तम् आयातः इति` at `crates/sadhana-t1/src/parse.t1:280` is a STRING, and
/// the word inside it is data.
///
/// This is why `mandala` reads a `lex_t1` stream and not a `lex` one: the T0
/// lexer hands the interior over as bare words, and `parse.t1` would report an
/// import of a module named `इति`.
#[test]
fn the_import_keyword_inside_a_string_is_not_an_import() {
    let text = std::fs::read_to_string(repo_root().join("crates/sadhana-t1/src/parse.t1"))
        .expect("crates/sadhana-t1/src/parse.t1 is readable");
    assert!(
        text.contains("उक्तम् आयातः इति"),
        "parse.t1 no longer writes the import keyword inside a string; this test \
         now asserts nothing"
    );

    // THE T0 STREAM, which is what `t1_sources.rs` lexes with. A string is a
    // phrase of ordinary words there, so `आयातः` arrives as a word and the
    // reader takes the next word for a module name.
    let t0 = sadhana::lex::lex(&text).expect("parse.t1 lexes as T0");
    let read_as_t0 = mandala::read_unit(&t0);
    assert!(
        read_as_t0.imports.iter().any(|i| i.name.text == "इति"),
        "the premise of this test is gone: a T0 stream no longer turns \
         `उक्तम् आयातः इति` into an import. Check whether `lex` gained the string \
         token before believing the T1 reading below proves anything."
    );

    // THE T1 STREAM, on the same words. ADR-0017 makes the literal ONE token
    // and `read_unit` never looks inside one.
    let t1 = mandala::read("मण्डलम् व्याकर ॥\nयदि मेलनम् आरभ्य उक्तम् आयातः इति समाप्तम् आदि\n")
        .expect("the line lexes as T1");
    assert_eq!(
        t1.imports,
        vec![],
        "a string literal was read as an import from a T1 stream"
    );
}

/// Every `आयातः` that names a module no source declares.
///
/// The other half of the same question, reported rather than fixed: these are
/// spelling drifts in files this row does not own. `कोश` is imported twice while
/// `crates/sadhana-t1/src/kosha.t1:1` declares `कओश`; `सअरणई` is imported three
/// times while `crates/textapp/src/text/tables.t1:1` declares `सारणी`; and
/// `पद` is imported by `lib.t1`, which declares no module at all.
///
/// EXACT, not a ceiling — a sixth would slip past a `<=`.
const IMPORTS_NAMING_NO_DECLARED_MODULE: &[(&str, &str, usize)] = &[
    // `lib.t1` → `पद` WAS HERE and is dropped in the commit that removed it
    // (`W-225`, 2026-09-04), as the assertion below instructs. `lib.t1` was
    // three `आयातः` lines and nothing else — no module, no declaration — and
    // none of the three was reached; `पद` named no module any source declares.
    // `no_import_in_this_crate_is_dead_or_names_an_undeclared_module` below
    // holds both counts at zero for `crates/sadhana-t1/src/`.
    // `कोश` WAS HERE TWICE — samyojana.t1:28 and encode.t1:4 — and both rows are
    // dropped in the commit that fixed them, as the assertion below instructs.
    // Neither import was wrong. `kosha.t1` declared `मण्डलम् कओश`: the module
    // name had been transcribed out of English one letter at a time, so the
    // module every caller correctly asked for did not exist under that spelling.
    // Rewritten 2026-08-30 as `मण्डलम् कोश`, and the two imports now resolve.
    ("crates/textapp/src/text/ident.t1", "सअरणई", 2),
    ("crates/textapp/src/text/nfc.t1", "सअरणई", 2),
    ("crates/textapp/src/text/segment.t1", "सअरणई", 2),
];

#[test]
fn every_import_that_names_no_declared_module_is_recorded() {
    let units = corpus();
    let found = mandala::unknown_imports(&units);
    println!("METRIC sadhana_t1_unknown_imports {}", found.len());

    let mut rendered: Vec<(String, String, usize)> = found
        .iter()
        .map(|u| (u.unit.clone(), u.module.clone(), u.line))
        .collect();
    rendered.sort();
    let mut expected: Vec<(String, String, usize)> = IMPORTS_NAMING_NO_DECLARED_MODULE
        .iter()
        .map(|(a, b, c)| ((*a).to_string(), (*b).to_string(), *c))
        .collect();
    expected.sort();

    assert_eq!(
        rendered, expected,
        "the set of imports naming no declared module changed. If one was FIXED, \
         drop its row above in the same commit."
    );
}

/// Three imports are written `॥ आयातः … ॥` rather than `आयातः … ।`.
///
/// `import` in `spec/grammar-t1.ebnf` ends in the daṇḍa. `crates/textapp/src/
/// text/*.t1` wrap the whole statement in the DOUBLE daṇḍa instead — ADR-0012's
/// T0 directive shape, in a T1 source. Recorded, not fixed: those files are not
/// this row's, and `mandala::Import::danda_terminated` is the field that makes
/// the divergence countable rather than a silence.
#[test]
fn the_imports_that_do_not_close_with_the_danda_are_named() {
    let units = corpus();
    let mut odd: Vec<String> = units
        .iter()
        .flat_map(|(p, u)| {
            u.imports
                .iter()
                .filter(|i| !i.danda_terminated)
                .map(move |i| format!("{p}:{}", i.name.line))
        })
        .collect();
    odd.sort();
    println!("METRIC sadhana_t1_imports_without_a_danda {}", odd.len());
    assert_eq!(
        odd,
        vec![
            "crates/textapp/src/text/ident.t1:2",
            "crates/textapp/src/text/nfc.t1:2",
            "crates/textapp/src/text/segment.t1:2",
        ],
        "the set of imports not closed by `।` changed"
    );
}

// ── W-225: dead imports, undeclared modules, one module in two files ─────

/// Every `आयातः` in a unit that no qualified name in the SAME unit reaches,
/// as `(path:line module)`.
///
/// Reach is `Unit::accesses` — every `head ॱ member` the token reader saw, in
/// ANY position. That is the instrument the census in `t1_paradigm_names.rs`
/// did not have: its "import never used as a prefix" counts only uses the
/// resolver walks, which excludes TYPE positions, and it reported
/// `samyojana.t1`'s `वास्तु` (55 reaches, all types such as
/// `अङ्कः अन्तः वास्तुॱवस्तु`) and its `कोश` (`अङ्कः अन्तः कोशॱसंज्ञा`) as
/// dead. They are not, and the refused case below holds a type-position
/// reach as live.
fn dead_imports(units: &[(String, Unit)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, u) in units {
        for i in &u.imports {
            if !u.accesses.iter().any(|a| a.head == i.name.text) {
                out.push(format!("{path}:{} {}", i.name.line, i.name.text));
            }
        }
    }
    out
}

const THIS_CRATE: &str = "crates/sadhana-t1/src/";

/// THE RATCHET, at zero for this crate's sources, with the refused case first.
///
/// `W-209`'s census named nine dead imports; reading each site found SEVEN —
/// `encode.t1`'s `पदविभाग` and `कोश`, `lex.t1`'s and `nidana.t1`'s `वास्तु`,
/// and all three of `lib.t1`'s (`पद`, which no source declares, `व्याकर`,
/// `वास्तु`) — and two live ones reached only in type positions. The seven
/// are removed with a dated note at each site; this holds the count there.
/// `crates/textapp/src/text/` is outside this lane and is reported, not
/// pinned.
#[test]
fn no_import_in_this_crate_is_dead_or_names_an_undeclared_module() {
    // REFUSED: an import nothing reaches is reported by line; an import
    // reached only in a TYPE position is live.
    let synthetic = "मण्डलम् क ॥\n\
                     आयातः ख ।\n\
                     आयातः ग ।\n\
                     आयातः घ ।\n\
                     सार्वजनिक वृत्तिः च आदाय य ॱॱ अङ्कः अन्तः गॱवस्तु ददाति अ६४ आदि\n\
                     \x20   प्रत्यागमनम् खॱगणना ० ।\n\
                     इति\n";
    let unit = mandala::read(synthetic).expect("the synthetic unit lexes");
    assert_eq!(unit.imports.len(), 3);
    assert_eq!(
        dead_imports(&[("synthetic".to_string(), unit)]),
        vec!["synthetic:4 घ".to_string()],
        "`ख` is reached in a body, `ग` only in a parameter's TYPE — both live; \
         `घ` is reached nowhere"
    );

    let units = corpus();
    let dead = dead_imports(&units);
    let (mine, others): (Vec<&String>, Vec<&String>) =
        dead.iter().partition(|d| d.starts_with(THIS_CRATE));
    println!("METRIC sadhana_t1_dead_imports {}", mine.len());
    println!(
        "METRIC sadhana_t1_dead_imports_outside_this_crate {}",
        others.len()
    );
    for d in &others {
        println!("  dead import outside this crate: {d}");
    }
    assert_eq!(
        mine,
        Vec::<&String>::new(),
        "an `आयातः` in this crate reaches nothing. Remove it with a dated note at \
         the site — an import nothing reaches is a marker pointing at nothing — \
         or reach it."
    );

    // And no import in this crate names a module no source declares. The
    // exact table above still records the three in `crates/textapp/`.
    let unknown: Vec<String> = mandala::unknown_imports(&units)
        .iter()
        .filter(|u| u.unit.starts_with(THIS_CRATE))
        .map(|u| format!("{}:{} {}", u.unit, u.line, u.module))
        .collect();
    println!(
        "METRIC sadhana_t1_unknown_imports_in_this_crate {}",
        unknown.len()
    );
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "an `आयातः` in this crate names a module no source declares"
    );
}

/// `वास्तु` is declared by `ast.t1` AND `vastu.t1`, and that is ONE module in
/// two files, not a rename: `nirvahana.rs:458` keys a routine as
/// `module ॱ name`, so both register under `वास्तु`, and the names the two
/// files declare are disjoint, so every `वास्तुॱ…` reaches exactly one
/// definition. Both headers state it (`W-225`). This pins the pair, and
/// the disjointness that makes it sound — a name added to one file that
/// the other already declares fails here by name.
#[test]
fn vastu_is_one_module_declared_by_two_files_whose_names_are_disjoint() {
    let units = corpus();
    let mut declarers: Vec<&str> = units
        .iter()
        .filter(|(_, u)| u.declares_module("वास्तु"))
        .map(|(p, _)| p.as_str())
        .collect();
    declarers.sort_unstable();
    assert_eq!(
        declarers,
        vec![
            "crates/sadhana-t1/src/ast.t1",
            "crates/sadhana-t1/src/vastu.t1"
        ],
        "the files declaring `मण्डलम् वास्तु` changed; both headers state the pair"
    );

    // Every other module is declared by exactly one file.
    let mut per_module: std::collections::BTreeMap<&str, Vec<&str>> = Default::default();
    for (p, u) in &units {
        if let Some(m) = &u.module {
            per_module.entry(m.text.as_str()).or_default().push(p);
        }
    }
    let shared: Vec<(&&str, &Vec<&str>)> = per_module.iter().filter(|(_, f)| f.len() > 1).collect();
    println!(
        "METRIC sadhana_t1_modules_declared_by_two_files {}",
        shared.len()
    );
    assert_eq!(shared.len(), 1, "only `वास्तु` is declared twice: {shared:?}");

    // The disjointness, read from the sources: every top-level declared name.
    let declared = |path: &str| -> std::collections::BTreeSet<String> {
        std::fs::read_to_string(repo_root().join(path))
            .expect("readable")
            .lines()
            .filter_map(|l| {
                let l = l.strip_prefix("सार्वजनिक ").unwrap_or(l);
                let mut w = l.split_whitespace();
                let kw = w.next()?;
                matches!(kw, "वृत्तिः" | "चरः" | "संरचना" | "गणना" | "यन्त्रम्")
                    .then(|| w.next().map(str::to_string))
                    .flatten()
            })
            .collect()
    };
    let ast = declared("crates/sadhana-t1/src/ast.t1");
    let vastu = declared("crates/sadhana-t1/src/vastu.t1");
    println!("METRIC sadhana_t1_vastu_names_in_ast {}", ast.len());
    println!("METRIC sadhana_t1_vastu_names_in_vastu {}", vastu.len());
    assert!(
        ast.len() >= 40 && vastu.len() >= 5,
        "the readers found the declarations"
    );
    let both: Vec<&String> = ast.intersection(&vastu).collect();
    assert!(
        both.is_empty(),
        "`वास्तु` declares a name in BOTH files, so `वास्तुॱ…` no longer reaches \
         one definition: {both:?}"
    );
}

// ── W-224: the junction entered from both sides ──────────────────────────
//
// research/22 §8 (2026-09-04, W-224) DECIDES: module cycles are allowed by
// rule — a junction may be entered from both sides — and the loader is
// order-independent. Research/23 §2.6 predicted the import graph acyclic and
// W-209 measured four 2-cycles, every one through `सङ्केतन`; W-214 demoted the
// prediction to a description. What follows holds the decision from both
// sides: every cycle NAMED and pinned (a fifth fails by name), and the loader
// shown to give the same program under every order the fifteen sources are
// handed in — measured, with the case that must be refused run first.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};

/// The import graph of THIS crate's sources as `(nodes, edges)`, in corpus
/// order — sorted paths, imports in written order. That is the order the
/// census in `t1_paradigm_names.rs` (statistic 20) walks, so its
/// `paradigm_name_import_cycles` — one cycle per DFS back edge — is reproduced
/// here exactly, from `mandala`'s reading rather than the T1 chain's.
fn import_graph(units: &[(String, Unit)]) -> (Vec<String>, Vec<(String, String)>) {
    fn seen(nodes: &mut Vec<String>, n: &str) {
        if !nodes.iter().any(|x| x == n) {
            nodes.push(n.to_string());
        }
    }
    let mut nodes: Vec<String> = Vec::new();
    let mut edges = Vec::new();
    for (p, u) in units.iter().filter(|(p, _)| p.starts_with(THIS_CRATE)) {
        let me = u.module.as_ref().map_or_else(
            || format!("{}(no मण्डलम्)", p.rsplit('/').next().unwrap_or(p)),
            |m| m.text.clone(),
        );
        seen(&mut nodes, &me);
        for i in &u.imports {
            seen(&mut nodes, &i.name.text);
            edges.push((me.clone(), i.name.text.clone()));
        }
    }
    (nodes, edges)
}

/// One cycle per DFS back edge, as `t1_paradigm_names.rs::cycles` counts them
/// — copied in method so the census's number is reproduced, not approximated.
fn dfs_back_edge_cycles(nodes: &[String], edges: &[(String, String)]) -> usize {
    fn visit(
        n: &str,
        edges: &[(String, String)],
        state: &mut std::collections::BTreeMap<String, u8>,
        path: &mut Vec<String>,
        out: &mut usize,
    ) {
        state.insert(n.to_string(), 1);
        path.push(n.to_string());
        for (a, b) in edges {
            if a != n {
                continue;
            }
            match state.get(b).copied().unwrap_or(0) {
                1 => *out += 1,
                0 => visit(b, edges, state, path, out),
                _ => {}
            }
        }
        path.pop();
        state.insert(n.to_string(), 2);
    }
    let mut state = std::collections::BTreeMap::new();
    let mut out = 0;
    for n in nodes {
        if state.get(n).copied().unwrap_or(0) == 0 {
            visit(n, edges, &mut state, &mut Vec::new(), &mut out);
        }
    }
    out
}

/// EVERY simple cycle of the graph, each written `a → b → c` starting from its
/// lexicographically smallest node, sorted. A DFS back-edge count is a property
/// of one walk; this is a property of the graph, and it is what a ratchet
/// that names every cycle has to pin.
fn simple_cycles(nodes: &[String], edges: &[(String, String)]) -> Vec<String> {
    fn extend(
        start: usize,
        at: usize,
        nodes: &[String],
        edges: &[(String, String)],
        path: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        for (a, b) in edges {
            if a != &nodes[at] {
                continue;
            }
            let Some(to) = nodes.iter().position(|n| n == b) else {
                continue;
            };
            if to == start {
                out.push(path.clone());
            } else if to > start && !path.contains(&to) {
                path.push(to);
                extend(start, to, nodes, edges, path, out);
                path.pop();
            }
        }
    }
    let mut found: Vec<Vec<usize>> = Vec::new();
    for start in 0..nodes.len() {
        extend(start, start, nodes, edges, &mut vec![start], &mut found);
    }
    let mut named: Vec<String> = found
        .into_iter()
        .map(|c| {
            let names: Vec<&str> = c.iter().map(|i| nodes[*i].as_str()).collect();
            let min = names
                .iter()
                .enumerate()
                .min_by_key(|(_, n)| **n)
                .map_or(0, |(i, _)| i);
            let mut rot = names[min..].to_vec();
            rot.extend_from_slice(&names[..min]);
            rot.join(" → ")
        })
        .collect();
    named.sort();
    named.dedup();
    named
}

/// The cycles the corpus HAS, pinned. Every one passes through `सङ्केतन`:
/// the four 2-cycles W-209 counted and the three longer rings the same
/// edges compose. The census's `paradigm_name_import_cycles` is one per DFS
/// back edge — a property of one walk, which read 4 at `e0374d44` and reads
/// 5 today after `parse.t1` imported `अक्षरकोश` (an edge on no cycle moved
/// it); this list is a property of the graph and did not move. Change it
/// only with the import that changes it, in the same commit, and say which
/// side of the junction was entered.
const IMPORT_CYCLES: &[&str] = &[
    "अक्षरकोश → सङ्केतन",
    "अक्षरकोश → सङ्केतन → निदान → वाक्यविभाग",
    "अक्षरकोश → सङ्केतन → वाक्यविभाग",
    "निदान → वाक्यविभाग → सङ्केतन",
    "निदान → सङ्केतन",
    "वाक्यविभाग → सङ्केतन",
    "विश्लेषण → सङ्केतन",
];

/// The module every cycle passes through. Remove it and the graph is a DAG.
const HUB: &str = "सङ्केतन";

/// Cycles the graph has that the list does not, and cycles the list names
/// that the graph has not — a pin that is loose on either side is not a pin.
fn unpinned_cycles(
    nodes: &[String],
    edges: &[(String, String)],
    pinned: &[&str],
) -> (Vec<String>, Vec<String>) {
    let have = simple_cycles(nodes, edges);
    let extra = have
        .iter()
        .filter(|c| !pinned.contains(&c.as_str()))
        .cloned()
        .collect();
    let missing = pinned
        .iter()
        .filter(|p| !have.iter().any(|c| c == *p))
        .map(ToString::to_string)
        .collect();
    (extra, missing)
}

/// **Every import cycle is named, and a fifth fails by name.** The ratchet on
/// `paradigm_name_import_cycles` (4) that W-209 filed for and W-224 decided.
#[test]
fn every_import_cycle_is_named_and_every_one_passes_through_sanketan() {
    let units = corpus();
    let (nodes, edges) = import_graph(&units);

    // ── REFUSED FIRST. A self-import is a cycle and is reported by name;
    //    a ring the list does not carry is reported by name; a cycle the list
    //    carries and the graph has lost is reported too. ──────────────────
    let mut with_self = edges.clone();
    with_self.push(("क".to_string(), "क".to_string()));
    let mut nodes_k = nodes.clone();
    nodes_k.push("क".to_string());
    let (extra, missing) = unpinned_cycles(&nodes_k, &with_self, IMPORT_CYCLES);
    assert_eq!(
        (extra, missing),
        (vec!["क".to_string()], vec![]),
        "a synthetic self-import must be the one unpinned cycle, named"
    );
    // `parse.t1` imports `पदविभाग`; make the lexer import the parser back.
    // ONE edge, THREE rings: `व्याकर` also imports `अक्षरकोश`, so the new
    // 2-cycle composes with the hub's cycles into two longer ones. The first
    // version of this expected one cycle and the instrument named all three
    // — which is the point of naming them.
    //
    // THREE RINGS → SEVEN (2026-09-14), AND THE CORPUS'S OWN CYCLES DID NOT
    // MOVE. This is the synthetic control, not the graph: the assertion above
    // still finds the real cycle set exactly as `IMPORT_CYCLES` names it. What
    // changed is what the injected edge COMPOSES with. When the compiler's spec
    // tables moved from a lex-time include to a run-time lookup in `पदविभाग`,
    // five modules — `सङ्केतन`, `निदान`, `अक्षरकोश`, `विश्लेषण`, `संयोजन` — gained a
    // real `आयातः पदविभाग`, so four more paths now reach the injected edge.
    // The control grows with the corpus BY CONSTRUCTION; a fixed list of three
    // would have had to be weakened to keep it, and the list is the assertion.
    let mut with_ring = edges.clone();
    with_ring.push(("पदविभाग".to_string(), "व्याकर".to_string()));
    let (extra, missing) = unpinned_cycles(&nodes, &with_ring, IMPORT_CYCLES);
    assert_eq!(
        (extra, missing),
        (
            vec![
                "अक्षरकोश → पदविभाग → व्याकर".to_string(),
                "अक्षरकोश → सङ्केतन → निदान → पदविभाग → व्याकर".to_string(),
                "अक्षरकोश → सङ्केतन → निदान → वाक्यविभाग → पदविभाग → व्याकर".to_string(),
                "अक्षरकोश → सङ्केतन → पदविभाग → व्याकर".to_string(),
                "अक्षरकोश → सङ्केतन → वाक्यविभाग → पदविभाग → व्याकर".to_string(),
                "अक्षरकोश → सङ्केतन → विश्लेषण → पदविभाग → व्याकर".to_string(),
                "पदविभाग → व्याकर".to_string(),
            ],
            vec![]
        ),
        "a fifth 2-cycle, not through the hub, and the six rings it composes \
         must be the unpinned cycles, each named"
    );
    let (extra, missing) = unpinned_cycles(&nodes, &edges, &IMPORT_CYCLES[1..]);
    assert_eq!(
        (extra, missing),
        (vec![IMPORT_CYCLES[0].to_string()], vec![]),
        "a cycle dropped from the list is reported as unpinned"
    );
    let mut without = IMPORT_CYCLES.to_vec();
    without.push("क → ख");
    let (extra, missing) = unpinned_cycles(&nodes, &edges, &without);
    assert_eq!(
        (extra, missing),
        (vec![], vec!["क → ख".to_string()]),
        "a cycle the list names and the graph has not is reported as missing"
    );

    // ── THE CORPUS. ──────────────────────────────────────────────────────
    let dfs = dfs_back_edge_cycles(&nodes, &edges);
    let simple = simple_cycles(&nodes, &edges);
    println!("METRIC sadhana_t1_import_modules {}", nodes.len());
    println!("METRIC sadhana_t1_import_edges {}", edges.len());
    println!("METRIC sadhana_t1_import_cycles {dfs}");
    println!("METRIC sadhana_t1_import_simple_cycles {}", simple.len());
    for c in &simple {
        println!("  CYCLE {c}");
    }
    let two: Vec<&String> = simple
        .iter()
        .filter(|c| c.matches(" → ").count() == 1)
        .collect();
    println!("METRIC sadhana_t1_import_two_cycles {}", two.len());
    let through_hub = simple
        .iter()
        .filter(|c| c.split(" → ").any(|n| n == HUB))
        .count();
    println!("METRIC sadhana_t1_import_cycles_through_hub {through_hub}");

    assert_eq!(
        dfs, 5,
        "the census's `paradigm_name_import_cycles` — one per DFS back edge in \
         corpus order — is pinned at 5 (W-224; it was 4 at e0374d44, see \
         IMPORT_CYCLES). It moved: an import was added or removed; if a cycle \
         changed, name it in IMPORT_CYCLES in the same commit, and re-pin the \
         census in t1_paradigm_names.rs beside it"
    );
    let (extra, missing) = unpinned_cycles(&nodes, &edges, IMPORT_CYCLES);
    assert!(
        extra.is_empty() && missing.is_empty(),
        "the import graph's cycles changed. Unpinned: {extra:?}. Named but \
         gone: {missing:?}. A cycle is allowed by rule (research/22 §8, W-224) \
         — a junction may be entered from both sides — but every one is \
         NAMED here, with the import that made it, in the same commit."
    );
    assert_eq!(
        through_hub,
        simple.len(),
        "a cycle avoids `{HUB}` — the junction is entered from both sides \
         at a second module, which the thesis's description does not cover"
    );
    let mut partners: Vec<&str> = two
        .iter()
        .flat_map(|c| c.split(" → "))
        .filter(|n| *n != HUB)
        .collect();
    partners.sort_unstable();
    assert_eq!(
        partners,
        vec!["अक्षरकोश", "निदान", "वाक्यविभाग", "विश्लेषण"],
        "the four modules that import `{HUB}` and are imported by it"
    );

    // Remove the hub and the graph is a DAG: the cycles are ONE junction
    // entered from both sides, not a tangle.
    let hubless: Vec<(String, String)> = edges
        .iter()
        .filter(|(a, b)| a != HUB && b != HUB)
        .cloned()
        .collect();
    let rest: Vec<String> = nodes.iter().filter(|n| *n != HUB).cloned().collect();
    assert!(
        simple_cycles(&rest, &hubless).is_empty(),
        "without `{HUB}` the import graph still has a cycle"
    );
}

// ── the loader, under every order ────────────────────────────────────────

/// A fixed permutation of `items` from `seed` — xorshift64 and Fisher–Yates,
/// so the orders are the same on every run and need no crate.
fn shuffled<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
    let mut s = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut v = items.to_vec();
    for i in (1..v.len()).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let bound = u64::try_from(i + 1).expect("a small length");
        let j = usize::try_from(s % bound).expect("below a small length");
        v.swap(i, j);
    }
    v
}

/// The fifteen sources of this crate, `(file name, text)`, sorted by name.
fn fifteen_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("src is readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| {
            let text = std::fs::read_to_string(dir.join(&n)).expect("readable");
            (n, text)
        })
        .collect()
}

fn load_in_order(sources: &[(String, String)]) -> Interpreter {
    let refs: Vec<(&str, &str)> = sources
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &repo_root().join("spec"))
        .unwrap_or_else(|e| panic!("the sources load in this order: {e:?}"))
}

/// Everything a load decides, in an order-free shape: the report, every
/// routine's parsed shape keyed by module, name and line, and every global's
/// initial value.
type Snapshot = (
    String,
    Vec<(String, String, usize, String)>,
    Vec<(String, String)>,
);

fn snapshot(it: &Interpreter) -> Snapshot {
    let mut routines: Vec<(String, String, usize, String)> = it
        .routines()
        .map(|r| (r.module.clone(), r.name.clone(), r.line, r.shape()))
        .collect();
    routines.sort();
    (
        format!("{:?}", it.report()),
        routines,
        it.globals_snapshot(),
    )
}

/// The names of the orders compared, beside the seeds that make them.
const SEEDS: &[u64] = &[0x224, 0x190, 0x193, 0x209, 0x214, 0x225];

/// The sites in the corpus where the loader's answer COULD depend on order,
/// as `Interpreter::load_order_sensitive_sites` names them — pinned. Each is
/// a global two modules both declare under the runtime's one flat key
/// (W-192's finding, measured here as the loader sees it): the values
/// coincide because the declared types do, which the snapshot proves under
/// every order below. Not one bare call or bare type is on the list, and the
/// two collisions the loader used to settle by position — a variant declared
/// twice, a module declared by two files — are settled by name since W-224
/// (`a_shared_variant_is_module_scoped_and_a_two_file_module_is_one_module`).
/// A new line here is a new way for order to matter; remove the collision
/// or explain it, in the same commit.
const LOAD_ORDER_SENSITIVE_SITES: &[&str] = &[
    // EMPTY SINCE 2026-09-11, and the emptiness is the point rather than an
    // omission. The eight entries that stood here — `आज्ञाकोश`, `आज्ञासूचकाङ्क`,
    // `वाक्यकोश`, `वाक्यसूचकाङ्क`, `दोषकोश`, `दोषसूचकाङ्क`, `पाठकोष्ठकम्`,
    // `दत्तकोष्ठकम्` — were each declared by two modules, and each said in its own
    // text "one flat key, the LAST loaded initialises it". THEY WERE SHARED
    // STORAGE, not merely shared names: `ir.t1` read the same cell `vakyavibhaga`
    // wrote, and `CHAIN` loads the second of every pair last, so the renamed side
    // initialised all eight.
    //
    // The owner's storage ruling (research/28) removed the instances by renaming
    // the `vakyavibhaga`/`samyojana` side. This list going empty is the acceptance
    // check for that, and it is STRONGER than the 592-declarations/592-distinct
    // identity because it names the eight individually — an unnamed ninth would
    // show here and could not hide in a count.
    //
    // The margin above still governs: a new line here is a new way for order to
    // matter, and it must be removed or explained IN THE SAME COMMIT. That rule
    // is what caught this edit — the removal direction needed the same treatment
    // as an addition, and neither the trunk nor I read it before gating.
];

/// **The loader gives the same program under every order of the fifteen
/// sources** — research/22 §8's decision (a), measured, with the refused
/// case first.
#[test]
fn the_loader_gives_the_same_program_under_every_order_of_the_fifteen_sources() {
    // ── REFUSED FIRST: an order dependence the instruments MUST see. `क` and
    //    `ख` export `साधारणम्` at arities two and one; `ग` calls it BARE and
    //    owns neither. The parser reads the arity from the last loaded, the
    //    runtime dispatches to the first loaded — W-193's collision, at the
    //    one place W-193 left it: a name the caller does not own. ──────────
    let ka = "मण्डलम् क ॥\n\
              सार्वजनिक वृत्तिः साधारणम् आदाय अ ॱॱ न६४ ऽ आ ॱॱ न६४ ददाति न६४ आदि\n\
              प्रत्यागमनम् अ योगः आ ।\nइति\n";
    let kha = "मण्डलम् ख ॥\n\
               सार्वजनिक वृत्तिः साधारणम् आदाय अ ॱॱ न६४ ददाति न६४ आदि\n\
               प्रत्यागमनम् अ गुणनम् १० ।\nइति\n";
    let ga = "मण्डलम् ग ॥\n\
              सार्वजनिक वृत्तिः परीक्षा ददाति न६४ आदि\n\
              प्रत्यागमनम् साधारणम् ३ ४ ।\nइति\n";
    let spec = repo_root().join("spec");
    let forward = Interpreter::load(&[("ka.t1", ka), ("kha.t1", kha), ("ga.t1", ga)], &spec)
        .expect("three synthetic modules load");
    let backward = Interpreter::load(&[("kha.t1", kha), ("ka.t1", ka), ("ga.t1", ga)], &spec)
        .expect("the same three load in the other order");
    let sites = forward.load_order_sensitive_sites();
    assert_eq!(
        sites.len(),
        1,
        "the bare call in `ग` to a name it does not own must be the one \
         order-sensitive site; got {sites:?}"
    );
    assert!(
        sites[0].starts_with("ग:2 bare call `साधारणम्`"),
        "the site names the module, the routine's line and the name: {sites:?}"
    );
    assert_eq!(
        backward.load_order_sensitive_sites(),
        sites,
        "the instrument's answer itself does not depend on the order"
    );
    let shape = |it: &Interpreter| it.routine("गॱपरीक्षा").expect("परीक्षा loaded").shape();
    assert_ne!(
        shape(&forward),
        shape(&backward),
        "with `ख` loaded last the call is read at arity one and `४` is stranded; \
         with `क` last it is read at arity two. If the two shapes agree the \
         fingerprint cannot see an order dependence and every zero below is \
         vacuous"
    );

    // ── THE CORPUS: sorted, reversed, and six seeded shuffles. ────────────
    let sorted = fifteen_sources();
    // 15 -> 17 on 2026-09-04: `W-215` added `unparse.t1` (the printer) and `W-239`
    // added `ashtaka.t1` (the octet arena). The name of this test keeps its
    // "fifteen" — the light gate cites it — and the count is the corpus's own.
    // (Found by the trunk at W-227's merge; main had been red on this pin since
    // W-215 landed without this test being in its gate's reach.)
    // 15 -> 16 on 2026-09-04, `W-236`: `yantrotsarjana.t1`, the T1 twin of the
    // RISC-V emitter, joins the corpus. The helper keeps its name — "fifteen" is
    // the count the ratchet was built at, and the assertion is what says the
    // corpus grew and by how much.
    // -> 18 on 2026-09-04 at the trunk's merge of W-236: the seventeen above plus
    // `yantrotsarjana.t1`; measured by this assertion on the merged tree.
    // -> 19 on 2026-09-04, `W-223` part 1: `sanchaya.t1`, the shared declaration
    // store (घोषणासञ्चय), joins the corpus; measured by this assertion.
    // -> 20 on 2026-09-07, `W-chain`: `shrinkhala.t1`, the DRIVER — the `.t1`
    // port of `chain.rs`'s stage sequence and the first module that sits ABOVE
    // the passes rather than beside them; measured by this assertion.
    // -> 21 on 2026-09-14: `sarani.t1`, the spec-table store — `समावेशसारणी`,
    // generated from `spec/` by tools/mkspectables.py so that a COMPILED
    // compiler can fill the store its own tables are read from. It is the first
    // member of the corpus that no one wrote by hand, and at 866 KB it is by far
    // the largest; measured by this assertion.
    assert_eq!(sorted.len(), 21, "the corpus is twenty-one sources");
    let baseline_it = load_in_order(&sorted);
    let baseline = snapshot(&baseline_it);
    // The instrument first, so a divergence below is read beside the sites
    // that could have caused it.
    let sites = baseline_it.load_order_sensitive_sites();
    println!(
        "METRIC sadhana_t1_load_order_sensitive_sites {}",
        sites.len()
    );
    for s in &sites {
        println!("  SITE {s}");
    }
    let mut orders: Vec<(String, Vec<(String, String)>)> = vec![(
        "reversed".to_string(),
        sorted.iter().rev().cloned().collect(),
    )];
    for seed in SEEDS {
        orders.push((format!("seed {seed:#x}"), shuffled(&sorted, *seed)));
    }
    let mut divergences = Vec::new();
    for (label, order) in &orders {
        assert_ne!(
            order.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            sorted.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            "{label} is the sorted order; it compares nothing"
        );
        let it = load_in_order(order);
        let got = snapshot(&it);
        if got.0 != baseline.0 {
            divergences.push(format!("{label}: report {} vs {}", got.0, baseline.0));
        }
        for (a, b) in got.1.iter().zip(&baseline.1) {
            if a != b {
                divergences.push(format!(
                    "{label}: {}ॱ{} (line {}) parsed differently",
                    a.0, a.1, a.2
                ));
            }
        }
        for (a, b) in got.2.iter().zip(&baseline.2) {
            if a != b {
                divergences.push(format!("{label}: global `{}` is {} vs {}", a.0, a.1, b.1));
            }
        }
        if got.1.len() != baseline.1.len() || got.2.len() != baseline.2.len() {
            divergences.push(format!(
                "{label}: a different number of routines or globals"
            ));
        }
    }
    println!("METRIC sadhana_t1_load_orders_compared {}", orders.len());
    println!("METRIC sadhana_t1_load_order_routines {}", baseline.1.len());
    println!("METRIC sadhana_t1_load_order_globals {}", baseline.2.len());
    println!(
        "METRIC sadhana_t1_load_order_divergences {}",
        divergences.len()
    );
    for d in &divergences {
        println!("  DIVERGES {d}");
    }
    assert!(
        divergences.is_empty(),
        "the loader's answer depends on the order the sources are given in — \
         research/22 §8 (W-224) says it must not. {divergences:#?}"
    );

    // ── THE INSTRUMENT over the corpus, pinned. ──────────────────────────
    assert_eq!(
        sites,
        LOAD_ORDER_SENSITIVE_SITES
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        "the set of places where load order could matter changed"
    );
}

/// **The two collisions the loader used to resolve by load position, refused
/// and reached by name instead** — found by the order test above before
/// either fix was written, and pinned here so neither comes back.
///
/// 1. A `गणना` variant two modules declare with DIFFERENT ordinals
///    (`अनिर्दिष्टम्`: `कोश ॱ संज्ञाखण्ड` 3, `वास्तु ॱ स्थापन` 4) was one flat
///    key, first loaded wins. Now: the declaring module's own bare use is its
///    own ordinal, a qualified use from anywhere is exact, and a bare use from
///    a module that declares neither is REFUSED — S3, a name in two places is
///    two names.
/// 2. A module declared by two sources (`वास्तु`) was two `Module`s under one
///    name: the last owned `by_module`, the first answered a qualified struct
///    lookup, so `वास्तुॱवस्तु` from `संयोजन` was a record under one order and
///    `Int(0)` under the other. Now the sources are one module.
#[test]
fn a_shared_variant_is_module_scoped_and_a_two_file_module_is_one_module() {
    let spec = repo_root().join("spec");

    // ── 1. the variant ───────────────────────────────────────────────────
    let ka = "मण्डलम् क ॥\n\
              सार्वजनिक गणना भेद आरभ्य आद्यम् ऽ अन्यत् समाप्तम् ।\n\
              सार्वजनिक वृत्तिः मम ददाति न६४ आदि\n    प्रत्यागमनम् अन्यत् ।\nइति\n";
    let kha = "मण्डलम् ख ॥\n\
               सार्वजनिक गणना भेद आरभ्य अन्यत् ऽ आद्यम् समाप्तम् ।\n\
               सार्वजनिक वृत्तिः मम ददाति न६४ आदि\n    प्रत्यागमनम् अन्यत् ।\nइति\n";
    let ga = "मण्डलम् ग ॥\nआयातः क ।\nआयातः ख ।\n\
              सार्वजनिक वृत्तिः नग्नम् ददाति न६४ आदि\n    प्रत्यागमनम् अन्यत् ।\nइति\n\
              सार्वजनिक वृत्तिः कस्य ददाति न६४ आदि\n    प्रत्यागमनम् कॱअन्यत् ।\nइति\n\
              सार्वजनिक वृत्तिः खस्य ददाति न६४ आदि\n    प्रत्यागमनम् खॱअन्यत् ।\nइति\n";
    for order in [
        [("ka.t1", ka), ("kha.t1", kha), ("ga.t1", ga)],
        [("kha.t1", kha), ("ka.t1", ka), ("ga.t1", ga)],
    ] {
        let mut it = Interpreter::load(&order, &spec).expect("three modules load");
        let answer =
            |it: &mut Interpreter, name: &str| it.call(name, vec![], 1_000).map(|v| v.as_int());
        assert_eq!(
            answer(&mut it, "कॱमम"),
            Ok(Some(1)),
            "क's own `अन्यत्` is its second variant"
        );
        assert_eq!(
            answer(&mut it, "खॱमम"),
            Ok(Some(0)),
            "ख's own `अन्यत्` is its first variant"
        );
        assert_eq!(
            answer(&mut it, "गॱकस्य"),
            Ok(Some(1)),
            "`कॱअन्यत्` from ग is exact"
        );
        assert_eq!(
            answer(&mut it, "गॱखस्य"),
            Ok(Some(0)),
            "`खॱअन्यत्` from ग is exact"
        );
        let bare = answer(&mut it, "गॱनग्नम्");
        assert!(
            bare.as_ref()
                .is_err_and(|e| e.reason.contains("not a name in scope")),
            "a BARE `अन्यत्` from a module that declares neither must be refused by \
             name, not answered by whichever loaded first: {bare:?}"
        );
        assert_eq!(
            it.global("आद्यम्"),
            None,
            "`आद्यम्` is ० in क and १ in ख — no bare key"
        );
    }
    // A variant every declarer agrees on stays a bare key.
    let agree = "मण्डलम् च ॥\nसार्वजनिक गणना भेद आरभ्य आद्यम् ऽ अन्यत् समाप्तम् ।\n";
    let it = Interpreter::load(&[("ka.t1", ka), ("cha.t1", agree)], &spec).expect("loads");
    assert!(
        matches!(it.global("आद्यम्"), Some(Value::Int(0))),
        "क and च agree on `आद्यम्` = ०"
    );

    // ── 2. the two-file module, synthetic ───────────────────────────────
    let da1 = "मण्डलम् द ॥\nसार्वजनिक संरचना र आरभ्य क्ष ॱॱ न६४ समाप्तम् ।\n";
    let da2 = "मण्डलम् द ॥\nसार्वजनिक वृत्तिः त्रयः ददाति न६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n";
    let pa = "मण्डलम् प ॥\nआयातः द ।\n\
              सार्वजनिक वृत्तिः क्षेत्रम् ददाति न६४ आदि\n    चरः व ॱॱ दॱर भवति ० ।\n    प्रत्यागमनम् व ॱ क्ष ।\nइति\n\
              सार्वजनिक वृत्तिः आह्वानम् ददाति न६४ आदि\n    प्रत्यागमनम् दॱत्रयः ।\nइति\n";
    for order in [
        [("da1.t1", da1), ("da2.t1", da2), ("pa.t1", pa)],
        [("da2.t1", da2), ("da1.t1", da1), ("pa.t1", pa)],
    ] {
        let mut it = Interpreter::load(&order, &spec).expect("a module in two files loads");
        let field = it.call("पॱक्षेत्रम्", vec![], 1_000).map(|v| v.as_int());
        assert_eq!(
            field,
            Ok(Some(0)),
            "`दॱर` must be the record whichever of द's two files loaded first; \
             `Int(0)` here means the struct was looked up in the wrong file"
        );
        let called = it.call("पॱआह्वानम्", vec![], 1_000).map(|v| v.as_int());
        assert_eq!(
            called,
            Ok(Some(3)),
            "`दॱत्रयः` must be found whichever file loaded last"
        );
    }

    // ── 2. the two-file module, in the corpus: `वास्तुॱवस्तु` read from a
    //    probe beside the fifteen sources, sorted and reversed ─────────────
    // The probe's routine name is one no corpus module uses bare: a bare
    // key in `sigs` would otherwise turn a corpus variable into a call.
    let probe = "मण्डलम् परीक्षक ॥\nआयातः वास्तु ।\n\
                 सार्वजनिक वृत्तिः वस्तुक्षेत्रपरीक्षा ददाति न६४ आदि\n    \
                 चरः व ॱॱ वास्तुॱवस्तु भवति ० ।\n    प्रत्यागमनम् व ॱ शून्यक्षेत्रम् ।\nइति\n";
    let sorted = fifteen_sources();
    for (label, order) in [
        ("sorted", sorted.clone()),
        ("reversed", sorted.iter().rev().cloned().collect::<Vec<_>>()),
    ] {
        let mut with_probe = order;
        with_probe.push(("probe.t1".to_string(), probe.to_string()));
        let mut it = load_in_order(&with_probe);
        let got = it
            .call("परीक्षकॱवस्तुक्षेत्रपरीक्षा", vec![], 10_000)
            .map(|v| v.as_int());
        assert_eq!(
            got,
            Ok(Some(0)),
            "{label}: `वास्तुॱवस्तु` from a third module must be `vastu.t1`'s record \
             whichever of `ast.t1` and `vastu.t1` loaded first"
        );
    }
}

/// The T1 chain's verdict on one file under one loaded interpreter, as the
/// resolve and typecheck censuses read it (`t1_execution.rs`
/// `measure_corpus_resolve` / `measure_corpus_typecheck`): token count,
/// declarations parsed, parse errors, resolved, the refusal's name,
/// typechecked, the checker's flag.
fn chain_verdict(it: &mut Interpreter, src: &str) -> String {
    let toks = match it.call(
        "पदविभागॱपदविभाग",
        vec![Value::Octets(Octets::new(src.as_bytes()))],
        2_000_000_000,
    ) {
        Ok(v) => v.as_int().unwrap_or(0),
        Err(e) => return format!("lex failed: {e}"),
    };
    let parsed = match it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
    {
        Ok(v) => v.as_int().unwrap_or(0),
        Err(e) => return format!("parse failed: {e}"),
    };
    let perr = it.global("दोषसूचकाङ्क").and_then(Value::as_int).unwrap_or(-1);
    let r = match it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
    {
        Ok(v) => v,
        Err(e) => return format!("resolver init failed: {e}"),
    };
    let resolved = it.call(
        "अर्थॱकार्यक्रमनिर्णयः",
        vec![r, Value::Int(parsed)],
        4_000_000_000,
    );
    let refusal = match &resolved {
        Ok(Value::Bool(true)) => String::new(),
        Ok(_) => match it.global("अनिर्णीतनाम") {
            Some(Value::Octets(o)) => format!(
                "undeclared `{}` at line {}",
                String::from_utf8_lossy(o.as_slice()),
                it.global("अनिर्णीतपङ्क्ति")
                    .and_then(Value::as_int)
                    .unwrap_or(0)
            ),
            other => format!("refused, no name ({other:?})"),
        },
        Err(e) => format!("resolve failed: {e}"),
    };
    let typed = if refusal.is_empty() {
        match it.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000,
        ) {
            Ok(Value::Bool(true)) => "TYPECHECKED".to_string(),
            Ok(v) => format!(
                "type-refused ({v:?}; flag {:?}, name {:?})",
                it.global("प्रकारदोषमस्ति"),
                it.global("प्रकारदोषनाम").map(|v| format!("{v:?}"))
            ),
            Err(e) => format!("typecheck failed: {e}"),
        }
    } else {
        "not reached".to_string()
    };
    format!("{toks} tokens, {parsed} decls, {perr} parse errors, resolve [{refusal}], {typed}")
}

/// **The resolve/typecheck census is the same under shuffled loads** — the
/// row's own acceptance for decision (a): all fifteen sources loaded in the
/// sorted order, reversed, and two seeded shuffles, and the T1 chain run over
/// every file under each. One chain pass over the corpus is minutes, so this
/// is ignored in the crate and run by `tools/check-t1-ratchets.sh` on the
/// hourly deep gate. What can move it is the loader
/// (`crates/sadhana/src/t1/nirvahana.rs`) or a `.t1` source — those two only.
#[test]
#[ignore = "ratchet: four T1-chain passes over the corpus; tools/check-t1-ratchets.sh runs it on the hourly deep gate"]
fn the_resolve_and_typecheck_census_is_the_same_under_shuffled_loads() {
    let sorted = fifteen_sources();
    let orders: Vec<(String, Vec<(String, String)>)> = vec![
        ("sorted".to_string(), sorted.clone()),
        (
            "reversed".to_string(),
            sorted.iter().rev().cloned().collect(),
        ),
        (format!("seed {:#x}", SEEDS[0]), shuffled(&sorted, SEEDS[0])),
        (format!("seed {:#x}", SEEDS[1]), shuffled(&sorted, SEEDS[1])),
    ];
    let mut per_order: Vec<Vec<(String, String)>> = Vec::new();
    for (label, order) in &orders {
        let started = std::time::Instant::now();
        let mut verdicts = Vec::new();
        for (name, src) in &sorted {
            // A fresh interpreter per file, as the censuses do: the arenas
            // are one run's, and a second file on the same run would append.
            let mut it = load_in_order(order);
            verdicts.push((name.clone(), chain_verdict(&mut it, src)));
        }
        println!(
            "  order {label}: {} files in {}s",
            verdicts.len(),
            started.elapsed().as_secs()
        );
        per_order.push(verdicts);
    }
    let baseline = &per_order[0];
    for (name, v) in baseline {
        println!("  {name:24} {v}");
    }
    let resolved = baseline
        .iter()
        .filter(|(_, v)| v.contains("resolve []"))
        .count();
    let typed = baseline
        .iter()
        .filter(|(_, v)| v.ends_with("TYPECHECKED"))
        .count();
    let mut divergences = Vec::new();
    for ((label, _), verdicts) in orders.iter().zip(&per_order).skip(1) {
        for ((name, a), (_, b)) in verdicts.iter().zip(baseline) {
            if a != b {
                divergences.push(format!("{label}: {name}: {a} vs sorted: {b}"));
            }
        }
    }
    println!("METRIC sadhana_t1_census_orders_compared {}", orders.len());
    println!("METRIC sadhana_t1_census_files {}", baseline.len());
    println!("METRIC sadhana_t1_census_resolved {resolved}");
    println!("METRIC sadhana_t1_census_typechecked {typed}");
    println!(
        "METRIC sadhana_t1_census_order_divergences {}",
        divergences.len()
    );
    for d in &divergences {
        println!("  DIVERGES {d}");
    }
    assert!(
        resolved >= 14,
        "only {resolved} of {} files resolve under the sorted load; the census \
         below compares refusals, not programs",
        baseline.len()
    );
    assert!(
        divergences.is_empty(),
        "the T1 chain's verdict on a file depends on the order the fifteen \
         sources were loaded in: {divergences:#?}"
    );
}
