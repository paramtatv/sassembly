//! **A `.t1` BODY IS EXECUTED HERE** — task `D-002j`.
//!
//! # What every other test in this crate cannot do
//!
//! `tests/t1_sources.rs` is 3831 lines and every assertion in it is about TEXT:
//! a symbol is *declared*, a routine is *not a stub*, a kind is *constructed
//! somewhere*. That is not an oversight — until this file there was no other
//! kind of assertion available, because **no routine in `crates/sadhana-t1/src`
//! had ever been run.** `sadhana::t1::parse::parse_program` descends into a body
//! only when the routine's name is followed by `आदि`, and every routine writes
//! `आदाय` or `ददाति` there, so every one of them parsed to `body: None`.
//!
//! (`D-002j`'s note puts that at 256 routines, 235 `आदाय` and 21 `ददाति`.
//! Counted again on 2026-08-30 over the same 15 files it is **317, 292 and 25**,
//! with `आदि` still at ZERO. The row's premise holds; its arithmetic has moved,
//! and `the_corpus_has_no_routine_the_frozen_parser_can_descend_into` below is
//! what keeps the count honest from here on.)
//!
//! The cost was measured on 2026-08-29: inverting one comparison inside
//! `संज्ञाकुलपठनम्` — `असमम् ४०` to `समम् ४०`, the same tokens with the prune
//! reversed so the alias reader keeps exactly what it must drop — left **all 34
//! tests of this crate green**.
//!
//! Every test below is written so that it cannot. Each one CALLS a routine of
//! the corpus through [`sadhana::t1::nirvahana`] and asserts the value it
//! returns against `spec/`, and each is followed by a mutation test that breaks
//! one comparison in the routine it exercises and requires the answer to change.
//! The mutations are applied with [`mutate`], which fails if the text it is
//! given is not in the source **exactly once** — a mutation test that silently
//! matched nothing would be the same lie one layer up.
//!
//! # What the acceptance asked for, and what it turned out to need
//!
//! `D-002j` asks for one routine called with arguments and its RETURN VALUE
//! asserted, and for the `असमम् ४०` mutation to fail that test. Running it shows
//! those are two different requirements: the mutation **does not change
//! `संज्ञाकुलपठनम्`'s return value at all.** That routine returns
//! `संज्ञाकुलसूचकाङ्क`, the number of registry rows, which is ७४ either way; the
//! comparison it inverts governs `पाठांशकोश`, the alias arena, which goes from
//! १९७ pieces to १. So `the_mnemonic_reader_reads_the_registry` asserts the
//! arena as well as the return, and says why in its own body.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Replace `from` with `to` in `text`, and **fail if `from` is not there
/// exactly once.**
///
/// This is the whole guard on the mutation tests. A mutation applied to text
/// that does not contain it changes nothing and the test it guards passes for
/// the wrong reason — which is the same defect, one level up, as a ratchet that
/// counts declarations.
fn mutate(text: &str, from: &str, to: &str) -> String {
    let n = text.matches(from).count();
    assert_eq!(
        n, 1,
        "the mutation `{from}` -> `{to}` matches {n} places in the source; \
         a mutation test is only evidence when it changes exactly one"
    );
    text.replace(from, to)
}

/// Load `encode.t1` and `vakyavibhaga.t1`, optionally with one of them mutated
/// — and `ashtaka.t1`, the octet arena `vakyavibhaga.t1` imports and calls
/// from `आरम्भः` since `W-239`.
fn load(encode: &str, vakyavibhaga: &str) -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", encode),
            ("vakyavibhaga.t1", vakyavibhaga),
            ("ashtaka.t1", &source("ashtaka.t1")),
            ("vishlesana.t1", &source("vishlesana.t1")),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
        ],
        &spec_root(),
    )
    .expect("the three T1 sources load")
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

// ─────────────────────────────────────────────────────────────────────────
// The register reader — `सङ्केतन ॱ कोष्ठाङ्कः`, `encode.rs`'s `register`.
// ─────────────────────────────────────────────────────────────────────────

/// Every `(name, number)` of `spec/registers-riscv64.tsv`, read by THIS test
/// with Rust's own `lines`/`split`, so that the T1 reader is checked against
/// the file and not against another copy of itself.
fn register_table() -> Vec<(String, i128)> {
    let text = std::fs::read_to_string(spec_root().join("registers-riscv64.tsv"))
        .expect("spec/registers-riscv64.tsv exists");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| !l.starts_with("devanagari\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() <= 3 {
                return None;
            }
            f[2].parse::<i128>().ok().map(|n| (f[0].to_string(), n))
        })
        .collect()
}

/// The names the mutation test asks about: every third row of the table, plus
/// a name no row carries.
///
/// A sample and not the whole table, because a mutation run reloads and re-runs
/// the chain once per mutation and the `dev` profile is what tests are built
/// under. Every mutation below is killed by this sample; the FULL table is what
/// `the_register_reader_answers_the_number_the_table_carries` asserts.
fn sample_names() -> Vec<String> {
    let mut v: Vec<String> = register_table()
        .into_iter()
        .step_by(3)
        .map(|(n, _)| n)
        .collect();
    v.push("अविद्यमानम्".to_string());
    v
}

/// `कोष्ठाङ्कः` for each sampled name, or `None` where the routine refused.
fn read_sample(it: &mut Interpreter) -> Vec<(String, Option<Option<i128>>)> {
    sample_names()
        .into_iter()
        .map(|name| {
            let v = it
                .call("सङ्केतनॱकोष्ठाङ्कः", vec![octets(&name)], 20_000_000)
                .ok()
                .map(|v| v.as_int());
            (name, v)
        })
        .collect()
}

#[test]
fn the_corpus_has_no_routine_the_frozen_parser_can_descend_into() {
    // **The premise of `D-002j`, measured rather than quoted.** Every `.t1`
    // source is run through `sadhana::t1::parse` — the parser every other test
    // in this crate uses — and the number of routines it produced a BODY for is
    // counted. It is ०, and it is ० for a structural reason: `parse_program`
    // reads a body only when `आदि` follows the routine's name, and the corpus
    // always writes `आदाय` or `ददाति` there.
    //
    // This is the test that would have caught the row's stale arithmetic, and
    // it is the one that will notice if `t1::parse` ever learns to descend —
    // at which point the interpreter and the parser must be reconciled rather
    // than left as two readers of one language.
    // **AND IT LEXES THEM WITH `lex_t1`, WHICH IS A SECOND FINDING.**
    // `t1_sources.rs:90` — the floor test of this whole crate — lexes the T1
    // corpus with `sadhana::lex::lex`, the **T0** lexer. The two differ in
    // exactly one thing and it is what a string is: `lex` is `Strings::Words`,
    // where `उक्तम्` is an ordinary word, and `lex_t1` is `Strings::Token`,
    // where ADR-0017 has the lexer take the whole literal. Under the T1 lexer
    // `parse.t1` DOES NOT LEX, at lines 198 and 251, both
    // `उक्तम् इति इति समाप्तम्`: ADR-0011 makes `इति इति` an escaped literal
    // `इति`, so nothing closes the string and the line is refused. The intended
    // spelling is `उक्तम् इति इति इति`. Two tokens, in a file this row does not
    // own — reported, not fixed.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = 0usize;
    let mut declared = 0usize;
    let mut with_body = 0usize;
    let mut unlexable: Vec<String> = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("crates/sadhana-t1/src exists")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    entries.sort();
    for p in &entries {
        let text = std::fs::read_to_string(p).expect("a .t1 source is text");
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        sources += 1;
        let Ok(tokens) = sadhana::lex::lex_t1(&text) else {
            unlexable.push(name);
            continue;
        };
        let tokens = sadhana::t1::anita::resolve(tokens, &spec_root()).expect("embeds resolve");
        let program = sadhana::t1::parse::Parser::new(&tokens)
            .parse_program()
            .expect("parses");
        for d in &program.declarations {
            if let sadhana::t1::ast::Declaration::Function { body, .. } = d {
                declared += 1;
                if body.is_some() {
                    with_body += 1;
                }
            }
        }
    }
    println!("METRIC sadhana_t1_sources_measured {sources}");
    println!(
        "METRIC sadhana_t1_sources_not_lexable_as_t1 {}",
        unlexable.len()
    );
    println!("METRIC sadhana_t1_routines_declared {declared}");
    println!("METRIC sadhana_t1_routines_with_a_parsed_body {with_body}");
    assert!(sources >= 15, "only {sources} .t1 sources found");
    // A SUBSET check and not an equality, deliberately: `parse.t1` is the one
    // source that does not lex as T1 today, and **fixing it must make this test
    // pass, not fail.** An equality here would be a ratchet that punishes the
    // repair — the shape a ratchet must never have. A source that JOINS the
    // list still fails, which is what it is for.
    let unexpected: Vec<&String> = unlexable.iter().filter(|n| *n != "parse.t1").collect();
    assert!(
        unexpected.is_empty(),
        "{unexpected:?} do not lex as T1. Only `parse.t1` is accounted for \
         (`उक्तम् इति इति`, lines 198 and 251)"
    );
    assert!(
        declared >= 290,
        "only {declared} routines seen across the {} lexable sources; \
         the corpus is the evidence this rests on",
        sources - unlexable.len()
    );
    assert_eq!(
        with_body, 0,
        "`t1::parse` now produces {with_body} routine bodies. That is not a \
         failure, it is news: this file and `t1::nirvahana` both say the frozen \
         parser descends into none, and one of them is now wrong."
    );
}

#[test]
fn a_t1_body_is_executed_and_this_is_the_first_test_that_can_say_so() {
    // The floor. Every assertion below is meaningless if the loader silently
    // found no routines, so the counts are asserted before anything is called.
    let it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
    let r = it.report();
    println!("METRIC sadhana_t1_routines_loaded {}", r.routines);
    println!("METRIC sadhana_t1_routines_runnable {}", r.runnable);
    assert!(
        r.routines >= 150,
        "only {} routines loaded from encode.t1 and vakyavibhaga.t1",
        r.routines
    );
    // Ratchet. Raise it as the gaps below close; it may never fall.
    //
    // 143 -> 185 as the corpus grew; 185 -> 191 on 2026-09-04 (W-227), when
    // the seven ARITY FAULTS IN THE CORPUS that
    // `the_corpus_carries_call_sites_that_cannot_be_executed` used to list
    // were repaired at their call sites. The five that still do not parse
    // here are NOT five gaps in the interpreter: each calls into a module
    // this test does not load, and that test names every one.
    assert!(
        r.runnable >= 191,
        "only {} of {} routines have a body this interpreter can run (was 191)",
        r.runnable,
        r.routines
    );
}

#[test]
fn the_register_reader_answers_the_number_the_table_carries() {
    // **THIS IS `D-002j`'s ACCEPTANCE.** `कोष्ठाङ्कः` is called with the octets
    // of a register name and the number it returns is asserted against
    // `spec/registers-riscv64.tsv` — the same file the routine embeds with
    // `समावेशः आरभ्य कोष्ठकोशः समाप्तम्`, read here independently.
    //
    // The answer is ONE-BASED and the routine says why in its own margin:
    // `शून्यः` IS register ०, so a raw number would report the most-used
    // register in the ISA as an unknown name.
    let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
    let table = register_table();
    assert!(
        table.len() >= 60,
        "only {} register rows read; the table is the evidence",
        table.len()
    );

    let mut checked = 0usize;
    for (name, number) in &table {
        let v = it
            .call("सङ्केतनॱकोष्ठाङ्कः", vec![octets(name)], 20_000_000)
            .unwrap_or_else(|e| panic!("`कोष्ठाङ्कः {name}` runs: {e}"));
        assert_eq!(
            v.as_int(),
            Some(number + 1),
            "`कोष्ठाङ्कः {name}` answered {v:?}; the table says register {number}, \
             and the routine answers one-based"
        );
        checked += 1;
    }
    println!("METRIC sadhana_t1_registers_executed {checked}");

    // The other half of `Option<u32>`: a name no row carries is `शून्यम्`.
    // Without this the routine could answer `पङ्क्तिः योगः १` for anything.
    let missing = it
        .call("सङ्केतनॱकोष्ठाङ्कः", vec![octets("अविद्यमानम्")], 20_000_000)
        .expect("`कोष्ठाङ्कः` runs on an unknown name");
    assert!(
        missing.is_nil(),
        "a name the table does not carry must answer शून्यम्, not {missing:?}"
    );

    // And a name that is a PREFIX of a real one must not match it: `स्थिर१` is
    // a row, `स्थिर` is not. This is what `क्षेत्रसाम्यम्`'s length test buys.
    let prefix = it
        .call("सङ्केतनॱकोष्ठाङ्कः", vec![octets("स्थिर")], 20_000_000)
        .expect("`कोष्ठाङ्कः` runs on a prefix");
    assert!(
        prefix.is_nil(),
        "`स्थिर` is a prefix of `स्थिर१` and is not itself a row; got {prefix:?}"
    );
}

/// One mutation of `encode.t1`: what to break, and where it lives.
///
/// Every one of these is an INVERTED COMPARISON — the same tokens, the sense
/// reversed — because that is the mutation class `D-002j` was filed over. A
/// mutation that deleted a routine would be caught by the text ratchets that
/// already exist; one that reverses a test is caught by nothing but execution.
///
/// **`क्षेत्रसंख्या` IS DELIBERATELY NOT IN THIS TABLE, AND THAT IS A FINDING.**
/// Inverting its TAB test (`समम् ९` to `असमम् ९`) leaves **all 64 answers of the
/// full register table** unchanged — measured on 2026-08-30 by running exactly
/// that mutation through this test, not assumed. Its only consumer in this
/// chain is `कोष्ठपङ्क्तिः`'s `अधिकम् ३` guard, and a count that is wrong
/// UPWARDS still clears a floor of three, so no argument to `कोष्ठाङ्कः` can
/// see the fault. The routine needs a caller that compares its answer for
/// equality — `सङ्केताः`, when blocker (a) lifts — before a mutation of it is
/// observable at all. Listing it here would have made this test lie.
///
/// **THE SENTENCE BEFORE LAST WAS A PREDICTION AND IT IS STRUCK 2026-08-30.**
/// Blocker (a) lifted, `सङ्केताः` was written, the mutation was put into
/// `TABLE_READER_MUTATIONS` below, and it SURVIVED there too. `सङ्केताः`
/// tests the count at `अधिकम् ६` and `रूपधारकः` at `न्यूनम् ४` — floors
/// again, because every one of Rust's four readers tests `f.len()` against a
/// floor and a faithful port has nothing else to mirror. There is no caller
/// coming. See the note on that constant for the measurement.
const REGISTER_CHAIN_MUTATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "अष्टकान्वेषणम्",
        "the byte scan stops on the byte it should skip",
        "यदि पाठ्यम् अङ्कः सूचकाङ्क अन्तः समम् अष्टकम् आदि",
        "यदि पाठ्यम् अङ्कः सूचकाङ्क अन्तः असमम् अष्टकम् आदि",
    ),
    (
        "उपेक्ष्यपङ्क्तिः",
        "a `#` comment line stops being refused",
        "यदि पाठ्यम् अङ्कः आरम्भः अन्तः समम् ३५ आदि",
        "यदि पाठ्यम् अङ्कः आरम्भः अन्तः असमम् ३५ आदि",
    ),
    (
        "क्षेत्रारम्भः",
        "the past-the-last-field guard is reversed, so field ० answers सीमा",
        "यदि विरामः समम् सीमा आदि",
        "यदि विरामः असमम् सीमा आदि",
    ),
    (
        // क्षेत्रसाम्यम् → परिधिसाम्यम् ON 2026-09-14. The table index landed a
        // second comparator that takes the field bounds directly instead of
        // re-finding the tabs per row, with a body copied from this one — so
        // both anchors below matched twice, and the register chain now goes
        // through the NEW routine. Mutating the old one applied cleanly and
        // changed nothing the reader reads: a dead control. Both rows are
        // anchored to `परिधिसाम्यम्`'s own declaration, which supersedes the
        // note in the next row arguing against a multi-line anchor — that
        // argument was about a collision the new code could rename its way out
        // of, and this one is two routines that share a body on purpose.
        "परिधिसाम्यम्",
        "the length test is reversed, so only a name of the WRONG length can match",
        "सार्वजनिक वृत्तिः परिधिसाम्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ क्षेत्रादिः ॱॱ न६४ क्षेत्रान्तः ॱॱ न६४ नाम ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि\n    यदि आरभ्य क्षेत्रान्तः वियोगः क्षेत्रादिः समाप्तम् असमम् नाम ॱ दैर्घ्य आदि",
        "सार्वजनिक वृत्तिः परिधिसाम्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ क्षेत्रादिः ॱॱ न६४ क्षेत्रान्तः ॱॱ न६४ नाम ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि\n    यदि आरभ्य क्षेत्रान्तः वियोगः क्षेत्रादिः समाप्तम् समम् नाम ॱ दैर्घ्य आदि",
    ),
    (
        "परिधिसाम्यम्",
        "the octet-by-octet comparison is reversed",
        // This anchor stayed a single line, and `प्रथमपदसाम्यम्` — the
        // first-word matcher blocker (a) needed for `रूपाणि` — is why it is
        // worth a note. That routine walks octets the same way, and written
        // with this file's usual `सूचकाङ्क` its comparison line was spelled
        // IDENTICALLY, so `mutate` refused this entry as matching twice —
        // exactly as it is meant to. The new routine names its index
        // `पदसूचकाङ्क` instead, which is what it is. Widening this anchor to a
        // multi-line block was the alternative and it is the worse one: the
        // collision was created by the new code, so the new code carries it,
        // and a mutation that was already proven keeps the text it was proven
        // against.
        "सार्वजनिक वृत्तिः परिधिसाम्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ क्षेत्रादिः ॱॱ न६४ क्षेत्रान्तः ॱॱ न६४ नाम ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि\n    यदि आरभ्य क्षेत्रान्तः वियोगः क्षेत्रादिः समाप्तम् असमम् नाम ॱ दैर्घ्य आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    चरः सूचकाङ्क ॱॱ न६४ भवति ० ।\n\n    यावत् सूचकाङ्क न्यूनम् नाम ॱ दैर्घ्य आदि\n        यदि पाठ्यम् अङ्कः क्षेत्रादिः योगः सूचकाङ्क अन्तः असमम् नाम अङ्कः सूचकाङ्क अन्तः आदि",
        "सार्वजनिक वृत्तिः परिधिसाम्यम् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ क्षेत्रादिः ॱॱ न६४ क्षेत्रान्तः ॱॱ न६४ नाम ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि\n    यदि आरभ्य क्षेत्रान्तः वियोगः क्षेत्रादिः समाप्तम् असमम् नाम ॱ दैर्घ्य आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    चरः सूचकाङ्क ॱॱ न६४ भवति ० ।\n\n    यावत् सूचकाङ्क न्यूनम् नाम ॱ दैर्घ्य आदि\n        यदि पाठ्यम् अङ्कः क्षेत्रादिः योगः सूचकाङ्क अन्तः समम् नाम अङ्कः सूचकाङ्क अन्तः आदि",
    ),
    (
        "कोष्ठपङ्क्तिः",
        "the field-count guard is reversed, so only short rows are considered",
        "यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा अधिकम् ३ आदि",
        "यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा न्यूनम् ३ आदि",
    ),
    (
        "दशाङ्कमूल्यम्",
        "the upper digit bound is reversed, so no decimal digit is accepted",
        "यदि अष्टकम् अधिकम् ५७ आदि",
        "यदि अष्टकम् न्यूनम् ५७ आदि",
    ),
];

#[test]
fn breaking_one_comparison_in_the_register_chain_changes_the_answer() {
    // **The proof that the test above is not text-shaped.** For each mutation
    // the source is patched, the chain is re-run over the SAME table, and the
    // answers are required to differ from the true ones. `mutate` fails if the
    // text it is given is not in the source exactly once, so a mutation that
    // matched nothing cannot pass here by doing nothing.
    let truth = {
        let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
        read_sample(&mut it)
    };
    assert!(
        truth.iter().any(|(_, n)| matches!(n, Some(Some(_)))),
        "the unmutated chain answered nothing; there is no baseline to differ from"
    );

    let vak = source("vakyavibhaga.t1");
    let mut proved = 0usize;
    for (routine, what, from, to) in REGISTER_CHAIN_MUTATIONS {
        let broken = mutate(&source("encode.t1"), from, to);
        let mut it = load(&broken, &vak);
        // A mutation may make the routine REFUSE rather than answer wrongly —
        // an index outside the run, say. `read_sample` records that as `None`,
        // which differs from a number, so both are killed the same way.
        let answers = read_sample(&mut it);
        assert_ne!(
            answers, truth,
            "MUTATION SURVIVED: `{routine}` — {what}. The register reader \
             answered exactly the same numbers with the comparison reversed, \
             so `the_register_reader_answers_the_number_the_table_carries` \
             is measuring text and not behaviour."
        );
        proved += 1;
    }
    println!("METRIC sadhana_t1_register_mutations_killed {proved}");
    assert_eq!(
        proved,
        REGISTER_CHAIN_MUTATIONS.len(),
        "every mutation must be killed"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The mnemonic registry reader — `संज्ञाकुलपठनम्`, `parse.rs`'s `families()`.
// This is the routine `D-002j`'s named mutation lives in.
// ─────────────────────────────────────────────────────────────────────────

/// `(rows, alias pieces)` of `spec/mnemonics-riscv64.src.tsv` under the rule
/// `संज्ञाकुलपठनम्` states: a line that `उपेक्ष्यपङ्क्तिः` does not drop, with
/// at least five TAB-separated fields, whose field १ begins with octet २२४ —
/// the first octet of every Devanagari akṣara, which is how the header row
/// (`devanagari`) refuses itself. Field ४ is the alias column, comma-separated,
/// each piece trimmed of spaces, and a piece beginning `(` — octet ४० — is
/// dropped.
///
/// Computed here with Rust's own `split`, so the T1 reader is checked against
/// the file rather than against a second copy of its own logic.
fn registry_truth() -> (i128, i128) {
    let raw = std::fs::read(spec_root().join("mnemonics-riscv64.src.tsv"))
        .expect("spec/mnemonics-riscv64.src.tsv exists");
    let mut rows = 0i128;
    let mut pieces = 0i128;
    for line in raw.split(|b| *b == b'\n') {
        if line.first() == Some(&b'#')
            || line
                .iter()
                .all(|b| *b == b' ' || *b == b'\t' || *b == b'\r')
        {
            continue;
        }
        let f: Vec<&[u8]> = line.split(|b| *b == b'\t').collect();
        if f.len() < 5 || f[1].first() != Some(&224) {
            continue;
        }
        rows += 1;
        for piece in f[4].split(|b| *b == b',') {
            let piece = piece
                .iter()
                .position(|b| *b != b' ')
                .map_or(&piece[..0], |s| &piece[s..]);
            let piece = match piece.iter().rposition(|b| *b != b' ') {
                Some(e) => &piece[..=e],
                None => &piece[..0],
            };
            if !piece.is_empty() && piece[0] != b'(' {
                pieces += 1;
            }
        }
    }
    (rows, pieces)
}

#[test]
fn the_mnemonic_reader_reads_the_registry() {
    // `संज्ञाकुलपठनम्` takes no arguments: it embeds `नामकोशः` and walks it.
    // Both of its outputs are asserted, and the SECOND one is the reason this
    // test exists in this shape — see below.
    let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
    let (rows, pieces) = registry_truth();
    assert!(
        rows > 50,
        "only {rows} registry rows; the table is the evidence"
    );

    let returned = it
        .call("संज्ञाकुलपठनम्", Vec::new(), 4_000_000_000)
        .expect("`संज्ञाकुलपठनम्` runs");
    assert_eq!(
        returned.as_int(),
        Some(rows),
        "`संज्ञाकुलपठनम्` returned {returned:?}; \
         spec/mnemonics-riscv64.src.tsv carries {rows} registry rows"
    );

    // **THE RETURN VALUE ALONE IS NOT ENOUGH, AND MEASURING IT IS WHY.**
    // `D-002j`'s acceptance asks for the return value plus the `असमम् ४०`
    // mutation failing the test. Those are two different requirements: that
    // comparison governs which ALIAS pieces are appended to `पाठांशकोश`, and
    // the row count is ७४ with it either way. Asserting the arena is what makes
    // the mutation visible; `the_named_mutation_of_d_002j_is_killed` proves it.
    let aliases = it
        .global("पाठांशसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("पाठांशसूचकाङ्क is a global of vakyavibhaga.t1");
    assert_eq!(
        aliases, pieces,
        "`संज्ञाकुलपठनम्` appended {aliases} alias pieces; the table carries {pieces}"
    );
    println!("METRIC sadhana_t1_registry_rows_executed {rows}");
    println!("METRIC sadhana_t1_registry_aliases_executed {pieces}");
}

#[test]
fn the_named_mutation_of_d_002j_is_killed() {
    // **THE MUTATION `D-002j` WAS FILED OVER.** On 2026-08-29 this exact edit —
    // `असमम् ४०` to `समम् ४०` inside `संज्ञाकुलपठनम्`, so the alias reader keeps
    // exactly the pieces it must drop — left all 34 tests of this crate green.
    let (rows, pieces) = registry_truth();
    let broken = mutate(
        &source("vakyavibhaga.t1"),
        "यदि पाठ्यम् अङ्कः आवरणप्रारम्भः अन्तः असमम् ४० आदि",
        "यदि पाठ्यम् अङ्कः आवरणप्रारम्भः अन्तः समम् ४० आदि",
    );
    let mut it = load(&source("encode.t1"), &broken);
    let returned = it
        .call("संज्ञाकुलपठनम्", Vec::new(), 4_000_000_000)
        .expect("the mutated `संज्ञाकुलपठनम्` still runs");
    let aliases = it
        .global("पाठांशसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("पाठांशसूचकाङ्क is a global");

    // Recorded rather than asserted away: the row count does NOT move. A test
    // that asked only for the return value would still be green here, which is
    // the finding this row produced and the reason the test above asserts two
    // numbers instead of one.
    println!(
        "METRIC sadhana_t1_registry_rows_under_mutation {:?}",
        returned.as_int()
    );
    assert_eq!(
        returned.as_int(),
        Some(rows),
        "the row count is expected to be unchanged by this mutation; if it moved, \
         the finding recorded in this file is wrong and the note must be corrected"
    );
    assert_ne!(
        aliases, pieces,
        "MUTATION SURVIVED: with `असमम् ४०` read as `समम् ४०` the alias reader \
         still produced {pieces} pieces, so `the_mnemonic_reader_reads_the_registry` \
         is measuring text and not behaviour"
    );
    println!("METRIC sadhana_t1_registry_aliases_under_mutation {aliases}");
}

#[test]
fn breaking_the_registry_row_filter_changes_the_row_count() {
    // The alias mutation above leaves the row count alone, so on its own it
    // says nothing about the RETURN value. This one breaks the guard that
    // decides which lines are rows at all, and the return value must move.
    let (rows, _) = registry_truth();
    // Anchored on `देवनागरीस्थानम्`, which only `संज्ञाकुलपठनम्` writes: the
    // `यदि प्रथममष्टकम् समम् २२४` line ALONE appears twice — `निर्देशकोशपठनम्`
    // has the identical guard — and `mutate` refuses a mutation that would
    // change two places, which is exactly the protection it exists for.
    let broken = mutate(
        &source("vakyavibhaga.t1"),
        "देवनागरीस्थानम् अन्तः ।\n\n                यदि प्रथममष्टकम् समम् २२४ आदि",
        "देवनागरीस्थानम् अन्तः ।\n\n                यदि प्रथममष्टकम् असमम् २२४ आदि",
    );
    let mut it = load(&source("encode.t1"), &broken);
    let returned = it
        .call("संज्ञाकुलपठनम्", Vec::new(), 4_000_000_000)
        .map(|v| v.as_int());
    assert_ne!(
        returned,
        Ok(Some(rows)),
        "MUTATION SURVIVED: the registry row filter was inverted and \
         `संज्ञाकुलपठनम्` still returned {rows}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// What executing the corpus found in it.
// ─────────────────────────────────────────────────────────────────────────

/// **W-193's `ARITY_FAULTS` TABLE STOOD HERE AND IS RETIRED — 2026-09-04, W-227.**
///
/// It listed seven call sites in `vakyavibhaga.t1` that passed a number of
/// arguments the callee did not declare, each invisible to every other test in
/// this crate, and it was asserted BY NAME so that a repair would fail here and
/// say which row to strike. W-208's census (`tests/t1_paradigm_calls.rs`) then
/// measured the same class with an instrument that sees every call site of all
/// fifteen sources and found EIGHTEEN, the seven among them: sixteen were a
/// token's text passed as its `(पाठ ऽ अष्टक ऽ पाठसीमा)` triple to a routine
/// declaring ONE `अङ्कः अन्तः अ८` — the slice its Rust twin takes as `&str`
/// (`split_types(text: &str)`, `operands.push(Operand { base, .. })`); one was
/// the inverse, three slices handed to `निर्देशयोजनम्`'s nine parameters; and
/// one was a nested call written unbracketed (`utsarjana.t1`, `पाठयोजनम्
/// कोष्ठनाम अधिकरणम् ॱ कोष्ठ`), which the grammar's flat list reads as two
/// arguments. The corpus declares 152 slice parameters and 32 triples, so the
/// slice is its own convention and all eighteen were CORPUS ERRORS; every one
/// is rewritten at the call site to pass what the callee declares, and where
/// the callee answers an offset INTO the slice (`संज्ञासीमा`, `प्रकारारम्भः`)
/// the caller rebases it by `अष्टक` onto the whole text, which is what its
/// consumers already assumed.
///
/// So the pin moved to where the instrument lives:
/// `arity_disagreements_are_pinned_at_zero_and_a_new_one_fails_by_name` in
/// `t1_paradigm_calls.rs`, run by `tools/check-t1-ratchets.sh` on the hourly
/// deep gate. What stays HERE is the interpreter's own reading of the
/// same fact, in three parts below: the two files loaded alone leave exactly
/// the module-bound routines unrunnable and nothing else; with every module
/// loaded, every routine of `वाक्यविभाग` parses; and one repaired site put
/// back the old way is refused by name. The metric names are kept at zero
/// rather than dropped, so a dashboard row that read 7 reads 0 and not blank.
///
/// The routines that do not parse for a reason that is NOT the corpus's
/// fault: each calls into a module these tests do not load, because they load
/// only `encode.t1` and `vakyavibhaga.t1`. The interpreter reports such a
/// call as a PARSE error (`expected समाप्तम्, found ऽ`), because a name it has
/// no arity for is read as a variable and the bracket after it as a group —
/// so this list is checked in BOTH directions, and
/// `with_every_module_loaded_every_routine_of_vakyavibhaga_parses` is what
/// tells a module-bound row from a fault in the corpus.
///
///   * `अज्ञातसंज्ञादोषः` and `कारकपदपठनम्` call `पदविभागॱविभज`, declared in
///     `lex.t1`. Each ALSO carried one of the eighteen (`:2671`, `:2724`)
///     behind that reason, since `why_not` names the first fault it meets;
///     both are repaired.
///   * `अष्टकनिर्देशकार्यम्` — MOVED HERE from `ARITY_FAULTS` 2026-09-04. Its
///     arity fault (`स्थाननिर्देशयोजनम्`, 4 for 2) is repaired; what is left is
///     `अक्षरकोशॱमानदोषः`/`अक्षरकोशॱमानम्`/`अक्षरकोशॱअंशदोषः`/`अक्षरकोशॱअंशाः`,
///     declared in `sanskrit_text.t1`.
///   * `मूल्याङ्कः` is `encode.rs`'s `value_of`, whose third clause is
///     `अक्षरकोशॱअंशाः` — `numeral::bits`, declared in `sanskrit_text.t1`.
///   * `कुलक्षेत्रयोग्यम्` is `domains_fit`; it reads `विश्लेषण ॱ क्षेत्रवाचकः`
///     for field ९ of `spec/encodings-riscv64.tsv`, and `tests/t1_exec_encode.rs`
///     loads `vishlesana.t1` alongside and exercises it through `स्थानसङ्केतनम्`.
///
/// TWO ROWS STRUCK 2026-09-04, both stale before W-227 touched a line:
/// `सङ्केतनदोषवचनम्` (`निदानॱविवरणम्`) and `स्थानविन्यासः`
/// (`वाक्यविभागॱसंरेखकोश`) both PARSE in the two-file load today — measured
/// 185 runnable of 196 on the unedited tree, with these two among the 185.
/// The old check was a subset check ("so that repairing one of these makes
/// the test pass"), and a subset check is exactly what let two rows sleep; it
/// is the same shape W-193 recorded for `t1_modules`' `nidana` row. The check
/// is now equality. Whether either RUNS is `tests/t1_exec_encode.rs`'s
/// question, not this list's.
/// **EMPTY SINCE 2026-09-14, AND THE PREVIOUS ENTRY IN THIS MARGIN WAS WRONG.**
///
/// Two rows went when `lex.t1` joined this load, and I then wrote that the
/// three that remained were "`.t1` parse gaps and not module faults", renaming
/// the constant on that reasoning. That was FALSE, and sansos-c1 refuted it
/// with a measurement and with a discriminator already sitting in this file.
///
/// THE READING ERROR, because it is the transferable part: the reason string
/// carries TWO line numbers and I took the first.
///
///   `अष्टकनिर्देशकार्यम्` — vakyavibhaga.t1:3297: … found `ऽ` on line 3392
///
/// 3297 is where the ROUTINE is declared; the fault is the `on line N`, ninety-
/// five lines later. Reading the first made these look like refusals of a
/// routine's signature — a parser gap — when every one of them is a CALL:
/// `अक्षरकोशॱमानदोषः` at 3392, `अक्षरकोशॱअंशदोषः` at 2904 and at encode.t1:3662.
/// All three are declared in `sanskrit_text.t1`, which this loader did not
/// list. A name with no arity is read as a variable and the `आरभ्य … समाप्तम्`
/// after it as a group, which fails at the `ऽ` with several arguments and at
/// the `आरभ्य` with one — exactly the mechanism `load_sema`'s margin describes.
///
/// So the module was added and the list is EMPTY: with `sanskrit_text.t1`
/// present, every routine this load carries runs. MEASURED, not argued — and
/// the equality below is what makes an empty list a real assertion rather than
/// an absent one: a routine that stops being runnable fails here by name.
const CANNOT_RUN_IN_THIS_LOAD: &[&str] = &[];

#[test]
fn the_corpus_carries_call_sites_that_cannot_be_executed() {
    // **AN EQUALITY CHECK, in both directions.** A routine the two files
    // cannot run that is not listed is a new fault (or a new interpreter gap);
    // a listed routine that now runs is a stale row, and this test says so
    // rather than letting it sleep.
    let it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
    let mut unrunnable: Vec<(String, String)> = it
        .routines()
        .filter(|r| !r.is_runnable())
        .map(|r| (r.name.clone(), r.why_not().unwrap_or("?").to_string()))
        .collect();
    unrunnable.sort();
    // W-193's table, retired by W-227 — the class is pinned at zero by the
    // census ratchet in `t1_paradigm_calls.rs`; see the note above.
    println!("METRIC sadhana_t1_arity_faults 0");
    println!("METRIC sadhana_t1_arity_faults_still_open 0");
    println!("METRIC sadhana_t1_routines_unrunnable {}", unrunnable.len());
    for (name, why) in &unrunnable {
        println!("  unrunnable: {name} — {why}");
    }
    let names: Vec<&str> = unrunnable.iter().map(|(n, _)| n.as_str()).collect();
    let mut expected: Vec<&str> = CANNOT_RUN_IN_THIS_LOAD.to_vec();
    expected.sort_unstable();
    assert_eq!(
        names, expected,
        "the routines this load cannot run are not the ones \
         CANNOT_RUN_IN_THIS_LOAD accounts for. A name on the left only: a \
         new fault in the corpus or a gap in the interpreter — its `why_not` is \
         printed above. A name on the right only: a stale row; strike it."
    );
}

/// Every `.t1` source in the corpus directory, by name, sorted.
fn corpus_sources() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    names
}

/// Load every source, with `vakyavibhaga.t1`'s text replaced.
fn load_every_source_with_vakyavibhaga(vakyavibhaga: &str) -> Interpreter {
    let names = corpus_sources();
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| {
            let text = if n == "vakyavibhaga.t1" {
                vakyavibhaga.to_string()
            } else {
                source(n)
            };
            (n.clone(), text)
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

/// With every module loaded there is no module-bound excuse left, and every
/// routine of `वाक्यविभाग` parses — the statement W-193's table could make
/// only site by site, made once for the whole file. `D-002j`'s note that
/// `निर्देशकोशपठनम्` "CANNOT BE CALLED" is closed here: it is runnable.
#[test]
fn with_every_module_loaded_every_routine_of_vakyavibhaga_parses() {
    let it = load_every_source_with_vakyavibhaga(&source("vakyavibhaga.t1"));
    // **EVERY MODULE, NOT ONE (2026-09-14).** This was the only check of its
    // kind in the tree and it looked at 92 routines of 801 — so a routine that
    // stopped parsing anywhere else in the corpus was reported by nothing. The
    // loader already carries every source; only the two filters were narrow.
    // Widened by sansos-c1's measurement, which found exactly one in the other
    // 709 and would have named it the day it landed.
    let total = it.routines().count();
    let stuck: Vec<String> = it
        .routines()
        .filter(|r| !r.is_runnable())
        .map(|r| format!("{} — {}", r.name, r.why_not().unwrap_or("?")))
        .collect();
    println!("METRIC sadhana_t1_corpus_routines {total}");
    println!(
        "METRIC sadhana_t1_corpus_routines_unrunnable_with_every_module {}",
        stuck.len()
    );
    // 94 on 2026-09-04; the floor guards against a module rename making the
    // filter above match nothing and the assertion below pass on an empty set.
    assert!(total >= 780, "only {total} routines of the corpus loaded");
    assert!(
        stuck.is_empty(),
        "with all {} sources loaded these routines still do not \
         parse — a fault in the corpus, not a module boundary:\n  {}",
        corpus_sources().len(),
        stuck.join("\n  ")
    );
    assert!(
        it.routines()
            .any(|r| r.name == "निर्देशकोशपठनम्" && r.is_runnable()),
        "`निर्देशकोशपठनम्` is the routine W-193 recorded as uncallable; it is \
         expected to be runnable now"
    );
}

/// THE REFUSED CASE, LIVE. One of the eighteen is put back the way it was
/// written — `प्रकारारम्भः` handed the `(पाठ ऽ अष्टक ऽ पाठसीमा)` triple where it
/// declares one slice — and the interpreter refuses the routine BY NAME, with
/// the two counts and the line. The control is that the same routine parses on
/// the unedited text; without it the mutation would prove nothing.
#[test]
fn a_triple_passed_to_a_slice_parameter_is_refused_by_name() {
    let clean = source("vakyavibhaga.t1");
    let it = load(&source("encode.t1"), &clean);
    let control = it
        .routines()
        .find(|r| r.name == "शिरोव्याप्तिः")
        .expect("`शिरोव्याप्तिः` is declared in vakyavibhaga.t1");
    assert!(
        control.is_runnable(),
        "the control: `शिरोव्याप्तिः` must parse on the unedited text, but: {}",
        control.why_not().unwrap_or("?")
    );
    let broken = mutate(
        &clean,
        "    चरः अंशारम्भः ॱॱ अ६४ भवति शिरःचिह्नकम् ॱ अष्टक योगः प्रकारारम्भः आरभ्य \
         शिरःचिह्नकम् ॱ पाठ अङ्कः शिरःचिह्नकम् ॱ अष्टक अन्तः शिरःचिह्नकम् ॱ पाठसीमा ऽ १ समाप्तम् ।",
        "    चरः अंशारम्भः ॱॱ अ६४ भवति प्रकारारम्भः आरभ्य \
         शिरःचिह्नकम् ॱ पाठ ऽ शिरःचिह्नकम् ॱ अष्टक ऽ शिरःचिह्नकम् ॱ पाठसीमा ऽ १ समाप्तम् ।",
    );
    let it = load(&source("encode.t1"), &broken);
    let mutant = it
        .routines()
        .find(|r| r.name == "शिरोव्याप्तिः")
        .expect("`शिरोव्याप्तिः` is still declared");
    let why = mutant.why_not().unwrap_or("");
    assert!(
        why.contains("`प्रकारारम्भः` declares 2 parameters and is called with 4"),
        "MUTATION SURVIVED or was misreported: the triple put back at the \
         `प्रकारारम्भः` site of `शिरोव्याप्तिः` should be refused with both counts \
         named; the interpreter said: {why:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The AMD64 register names — `उत्सर्जन ॱ कोष्ठनाम`, `x86_64.rs`'s `reg_name`.
// ─────────────────────────────────────────────────────────────────────────

/// Every `(devanagari, number)` of `spec/registers-amd64.tsv`, read by THIS
/// test with Rust's own `lines`/`split`.
///
/// The same discipline `register_table` above is written under, and it matters
/// more here: `कोष्ठनाम` is a DISPATCH, fourteen `यदि` arms holding the names
/// as literals, so the table and the routine are two copies of one fact and
/// nothing but this test stops them drifting. Checking the routine against the
/// file is the whole point; checking it against a second list written here
/// would prove only that two things I wrote agree.
fn amd64_register_table() -> Vec<(String, i128)> {
    let text = std::fs::read_to_string(spec_root().join("registers-amd64.tsv"))
        .expect("spec/registers-amd64.tsv exists");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| !l.starts_with("devanagari\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() <= 3 {
                return None;
            }
            f[2].parse::<i128>().ok().map(|n| (f[0].to_string(), n))
        })
        .collect()
}

fn load_one(name: &str) -> Interpreter {
    let text = source(name);
    Interpreter::load(&[(name, text.as_str())], &spec_root())
        .unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

/// The name `कोष्ठनाम` returns for `n`, or `None` when the call failed.
fn amd64_name(it: &mut Interpreter, n: i128) -> Option<String> {
    let v = it
        .call("उत्सर्जनॱकोष्ठनाम", vec![Value::Int(n)], 20_000_000)
        .ok()?;
    match v {
        Value::Octets(o) => Some(String::from_utf8_lossy(o.as_slice()).into_owned()),
        other => panic!("कोष्ठनाम {n} returned {other:?}, not octets"),
    }
}

/// **The acceptance for `D-002e`'s `कोष्ठनाम`.** Executed, not counted.
#[test]
fn the_amd64_register_names_are_the_spec_table_and_not_a_second_copy_of_it() {
    let table = amd64_register_table();
    println!("METRIC sadhana_t1_amd64_registers {}", table.len());
    assert_eq!(
        table.len(),
        14,
        "spec/registers-amd64.tsv should carry the fourteen registers the \
         allocator hands out (x86_64.rs:102 matches 0..=13); it has {}",
        table.len()
    );

    let mut it = load_one("utsarjana.t1");
    for (want, number) in &table {
        let got =
            amd64_name(&mut it, *number).unwrap_or_else(|| panic!("कोष्ठनाम {number} did not run"));
        assert_eq!(
            &got, want,
            "कोष्ठनाम {number} returned `{got}`, but spec/registers-amd64.tsv \
             says `{want}`. The routine holds the names as literals; if the \
             table changed, change the arms in the same commit."
        );
    }
}

/// Rust panics past 13. This returns the empty slice, and the divergence is
/// asserted rather than described — see the routine's own margin.
#[test]
fn an_index_the_allocator_could_not_have_produced_names_no_register() {
    let mut it = load_one("utsarjana.t1");
    assert_eq!(
        amd64_name(&mut it, 14).as_deref(),
        Some(""),
        "index 14 is past the fourteen registers x86_64.rs:118 panics beyond, \
         so कोष्ठनाम must name nothing rather than name something"
    );
}

/// **The mutation.** The test above is only evidence if it FAILS when the
/// routine stops agreeing with the table — so break one arm and watch it.
///
/// The inversion is chosen to keep the SHAPE of the source intact: two names
/// swapped between two arms, both still well-formed literals, both still
/// present in the file. A test that counts arms, or greps for the names, would
/// stay green through it.
#[test]
fn swapping_two_register_names_is_caught() {
    let broken = mutate(
        &source("utsarjana.t1"),
        "यदि कोष्ठाङ्कः समम् ० आदि प्रत्यागमनम् उक्तम् सञ्चयः इति । इति",
        "यदि कोष्ठाङ्कः समम् ० आदि प्रत्यागमनम् उक्तम् आधारः इति । इति",
    );
    let mut it = Interpreter::load(&[("utsarjana.t1", broken.as_str())], &spec_root())
        .expect("the mutated source still loads");
    assert_eq!(
        amd64_name(&mut it, 0).as_deref(),
        Some("आधारः"),
        "the mutation did not take, so the test above proves nothing"
    );
    let table = amd64_register_table();
    assert_eq!(
        table[0].0, "सञ्चयः",
        "the table still says सञ्चयः for 0, so the assertion above would have \
         caught this mutation"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// GROWTH. `वाक्यविभाग ॱ पाठांशयोजनम्` — Rust's `out.spans.push(..)`.
// ─────────────────────────────────────────────────────────────────────────

/// **Does T1 have `push`?** Three blocker notes say it does not.
///
/// `artha.t1:444`, `samyojana.t1:466` (c) and `encode.t1` all stall on
/// "growable collections", and ADR-0026 repeated it: *"It is not `push` and it
/// does not unblock `artha.t1:444`."* ADR-0028 was then filed to take the
/// allocator decision those notes were waiting on.
///
/// But `vakyavibhaga.t1` contains SIX appenders already, and this is what one
/// of them does: make a record, fill it, advance a global cursor, write at the
/// cursor, return the index. That is `push`, spelled out rather than sugared,
/// and the only question left is whether it RUNS — which is a question no
/// count of routines can answer and only a call can.
///
/// If this passes, the three notes are stale in the same way `सङ्केताः` was,
/// and the eleven stubs in `artha.t1` are not waiting on a language feature.
#[test]
fn the_corpus_can_already_grow_a_collection_and_this_call_proves_it() {
    let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));

    let first = it
        .call(
            "वाक्यविभागॱपाठांशयोजनम्",
            vec![octets("क"), Value::Int(0), Value::Int(1)],
            20_000_000,
        )
        .expect("पाठांशयोजनम् runs")
        .as_int()
        .expect("it returns a number");

    let second = it
        .call(
            "वाक्यविभागॱपाठांशयोजनम्",
            vec![octets("ख"), Value::Int(1), Value::Int(2)],
            20_000_000,
        )
        .expect("पाठांशयोजनम् runs a second time")
        .as_int()
        .expect("it returns a number");

    println!("METRIC sadhana_t1_append_first {first}");
    println!("METRIC sadhana_t1_append_second {second}");

    // ONE-BASED, and ० is never a live entry — the convention the appenders
    // state in their own margins and `nirvahana.rs:188` records.
    assert_eq!(
        first, 1,
        "the first append should return १; the arena is one-based and ० is \
         never a live entry"
    );
    // THE POINT. The second append must land somewhere the first did not, or
    // the arena is not growing — it is being overwritten.
    assert_eq!(
        second, 2,
        "the second append returned {second}, not २. The cursor did not \
         advance, so this is a write over the first entry and not growth"
    );
}

/// The mutation for the test above: stop the cursor advancing, and the second
/// append must collide with the first.
///
/// Chosen because it leaves the routine's SHAPE untouched — same statements,
/// same arity, same arena write — so a test that counted appenders, or grepped
/// for `योगः`, would stay green straight through it.
#[test]
fn an_appender_that_does_not_advance_its_cursor_is_caught() {
    let broken = mutate(
        &source("vakyavibhaga.t1"),
        "पाठांशसूचकाङ्क भवति आरभ्य पाठांशसूचकाङ्क योगः १ समाप्तम् ।",
        "पाठांशसूचकाङ्क भवति आरभ्य पाठांशसूचकाङ्क योगः ० समाप्तम् ।",
    );
    let mut it = load(&source("encode.t1"), &broken);

    let a = it
        .call(
            "वाक्यविभागॱपाठांशयोजनम्",
            vec![octets("क"), Value::Int(0), Value::Int(1)],
            20_000_000,
        )
        .expect("runs")
        .as_int();
    let b = it
        .call(
            "वाक्यविभागॱपाठांशयोजनम्",
            vec![octets("ख"), Value::Int(1), Value::Int(2)],
            20_000_000,
        )
        .expect("runs")
        .as_int();

    assert_eq!(
        (a, b),
        (Some(0), Some(0)),
        "with the cursor pinned, both appends must return the same index — \
         if they still differ, the test above is not measuring the cursor"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SELF-HOSTING. `वाक्यविभाग ॱ सङ्कलनम्` over a real `.sas`.
// ─────────────────────────────────────────────────────────────────────────

/// Load any set of `.t1` sources by name.
fn load_all(names: &[&str]) -> Interpreter {
    // THE LEXER IS PART OF ANY SET THAT READS A TABLE (2026-09-14).
    // `पदविभागॱसमावेशपाठः` is declared in lex.t1 and is how every reader now
    // reaches a spec table, so a list naming `सङ्केतन`, `निदान`, `अक्षरकोश`,
    // `विश्लेषण` or `संयोजन` without the lexer resolves nothing. Added here
    // rather than at each call site because the dependency is the lookup's,
    // not any one test's.
    let mut names: Vec<&str> = names.to_vec();
    if !names.contains(&"lex.t1") {
        names.insert(0, "lex.t1");
    }
    let names: &[&str] = &names;
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

/// **`D-002` and `D-003` were reopened with one sentence:** *"ITS ONLY
/// EVIDENCE WAS A `.sas` FILE THAT DOES NOT ASSEMBLE."*
///
/// This is that evidence, or it is not, and either way it is now a test rather
/// than a claim. `सङ्कलनम्` is `D-002f9`'s port of Rust's `assemble_program`
/// (`parse.rs:1169`) — lex, then parse — and the whole T1 chain existed to
/// reach it. It is run here over `spec/lib-mudraka.sas`, a real library source from
/// the spec directory and not a fixture written to pass.
///
/// It returns a COUNT rather than a `Program`, because T1 has no `Result` and
/// the diagnostics already have somewhere to live — the row says so in its own
/// margin. ० means "nothing parsed", which is sound only because a source with
/// no tokens has no statements either.
///
/// NOTE ON WHAT THIS DOES AND DOES NOT SHOW. It shows the T1 assembler reading
/// a real program through its own lexer and statement splitter, executed. It
/// does NOT show an ELF being written; `उत्सर्जन`'s two emitter entry points
/// are still stubs. The row's claim is the first half and this tests the first
/// half.
#[test]
fn a_real_sas_program_assembles_through_the_t1_chain() {
    let mut it = load_all(&["lex.t1", "vakyavibhaga.t1", "ashtaka.t1"]);
    let src = std::fs::read_to_string(repo_root().join("spec/lib-mudraka.sas"))
        .expect("spec/lib-mudraka.sas exists");

    let statements = it
        .call("वाक्यविभागॱसङ्कलनम्", vec![octets(&src)], 200_000_000)
        .expect("सङ्कलनम् runs over a real .sas")
        .as_int()
        .expect("it returns a count");

    println!("METRIC sadhana_t1_selfhost_statements {statements}");

    // ELEVEN, and the number is checked rather than merely non-zero — "it ran"
    // and "it got the right answer" are different claims and only the second
    // is worth a row. spec/lib-mudraka.sas holds:
    //
    //   1 directive     ॥ वैश्विकम् मुद्रकः ॥
    //   3 labels        मुद्रकःॱॱ, चक्रम्ॱॱ, अन्तःॱॱ
    //   7 instructions  उपरिभारः, आहारः, समलङ्घनम्, निधानम्, योगः, लङ्घनम्,
    //                   सापेक्षलङ्घनम् — each closed by its own daṇḍa
    //
    // A statement is daṇḍa-terminated and not line-terminated, so this counts
    // sentences and not lines; the file is 16 lines and two of them are
    // comments. If the spec file gains a statement, change this number and say
    // which one — do not loosen it back to `> 0`.
    assert_eq!(
        statements, 11,
        "सङ्कलनम् read spec/lib-mudraka.sas and found {statements} statements, \
         not ११. ० would be the row's own encoding for `nothing parsed` — the \
         `.sas` that does not assemble, still not assembling — and any other \
         number means the statement splitter disagrees with the file"
    );
}

/// The guard on the test above: an EMPTY source must give ०.
///
/// Without this, `statements > 0` is satisfied by any routine that returns a
/// constant, and the acceptance would be met by a body that never read its
/// argument at all.
#[test]
fn an_empty_source_assembles_to_nothing() {
    let mut it = load_all(&["lex.t1", "vakyavibhaga.t1", "ashtaka.t1"]);
    let n = it
        .call("वाक्यविभागॱसङ्कलनम्", vec![octets("")], 20_000_000)
        .expect("सङ्कलनम् runs on the empty source")
        .as_int();
    assert_eq!(
        n,
        Some(0),
        "an empty source must assemble to ० statements; if it does not, \
         सङ्कलनम् is not reading what it was handed"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// THE DISASSEMBLER — `विश्लेषण`, the port of `crates/sadhana/src/vishlesana.rs`.
//
// **This module's whole reason to exist is to be a SECOND, INDEPENDENT reader
// of `spec/encodings-riscv64.tsv`**, because the encoder only ever uses that
// table in one direction and a direction is where this project keeps finding
// defects. A test that checked the T1 decoder against a list of expected
// answers written out here would be a THIRD transcription and would check
// nothing; so everything below is checked against the FILE, read by this test
// with Rust's own `lines`/`split` — the discipline `register_table` above is
// written under, and the reason it matters more here is that a decoder and an
// encoder disagreeing is exactly the defect class `B-058a` found.
// ═════════════════════════════════════════════════════════════════════════

/// One operand slot of one row, as the file writes it.
struct RefSlot {
    kind: String,
    map: Vec<(u32, u32)>,
    /// The fourth part of a compressed slot spec — the lowest value the field
    /// accepts (`encode.rs` `Slot::bias`); 0 for every 32-bit field.
    bias: u32,
}

/// One row of `spec/encodings-riscv64.tsv`, read by THIS test.
struct RefRow {
    insn: String,
    family: String,
    pattern: u32,
    mask: u32,
    bits: u32,
    slots: Vec<RefSlot>,
}

/// Every row of the encoding table, parsed here.
///
/// `encode.rs`'s own `encodings()` is deliberately NOT called: it is the reader
/// whose second opinion `विश्लेषण` exists to be, and asking it would make the
/// two agree by construction.
fn encoding_rows() -> Vec<RefRow> {
    let text = std::fs::read_to_string(spec_root().join("encodings-riscv64.tsv"))
        .expect("spec/encodings-riscv64.tsv exists");
    let mut out = Vec::new();
    for l in text.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 7 {
            continue;
        }
        // The header row refuses itself here exactly as it does in T1: its
        // `bits` cell is not a run of decimal digits.
        let (Ok(bits), Ok(pattern), Ok(mask)) = (
            f[5].parse::<u32>(),
            u32::from_str_radix(f[3].trim_start_matches("0x"), 16),
            u32::from_str_radix(f[4].trim_start_matches("0x"), 16),
        ) else {
            continue;
        };
        let slots = if f[6] == "(none)" {
            Vec::new()
        } else {
            f[6].split('|')
                .map(|s| {
                    let p: Vec<&str> = s.split(':').collect();
                    // `p.get(2)` and not `p[2]`: the eight compressed rows that
                    // fold an implicit `sp` into the encoding write their slot
                    // as `fixed:0x00000000`, with NO map — which is exactly
                    // what `encode.rs` reads as `unwrap_or_default()`. Eight
                    // more carry a FOURTH part, the field's bias. Neither
                    // shape appears among the 32-bit rows, and both are in the
                    // file.
                    let map = p
                        .get(2)
                        .map(|m| {
                            m.split(';')
                                .filter(|x| !x.is_empty())
                                .map(|x| {
                                    let (a, b) =
                                        x.split_once('>').expect("a map pair is `from>to`");
                                    (
                                        a.parse().expect("a value bit"),
                                        b.parse().expect("an encoding bit"),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let bias = p.get(3).map_or(0, |b| b.parse().expect("a bias"));
                    RefSlot {
                        kind: p[0].to_string(),
                        map,
                        bias,
                    }
                })
                .collect()
        };
        out.push(RefRow {
            insn: f[0].to_string(),
            family: f[1].to_string(),
            pattern,
            mask,
            bits,
            slots,
        });
    }
    out
}

/// The number `सङ्केतन` gives a slot kind, **read out of the loaded corpus**
/// rather than written down here.
///
/// `encode.t1` declares `सार्वजनिक चरः कोष्ठभेद ॱॱ अ६४ भवति १` and six more,
/// and those globals are what `विश्लेषण ॱ अवकाशभेदाङ्कः` answers with. Asking
/// the interpreter for them keeps this test from becoming a second copy of the
/// numbering. The MAPPING from the table's spelling to the corpus's name is
/// the one thing written here, and a kind the table grows that no name covers
/// panics rather than being silently scored ०.
fn kind_number(it: &Interpreter, kind: &str) -> i128 {
    let name = match kind {
        "reg" => "कोष्ठभेद",
        "freg" => "प्लवकोष्ठभेद",
        "imm" => "तत्कालभेद",
        "simm" => "सचिह्नतत्कालभेद",
        "label" => "नामाङ्कभेद",
        "fixed" => "स्थिरभेद",
        "disp" => "अन्तरभेद",
        other => panic!(
            "spec/encodings-riscv64.tsv carries a slot kind `{other}` that सङ्केतन does not number"
        ),
    };
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("सङ्केतन declares `{name}`"))
}

/// Pull a slot's value back out of a word — `vishlesana.rs`'s `extract`.
fn ref_extract(slot: &RefSlot, word: u32) -> i128 {
    let mut value: u64 = 0;
    for (from, to) in &slot.map {
        if word >> to & 1 == 1 {
            value |= 1 << from;
        }
    }
    if slot.kind == "disp" || slot.kind == "simm" {
        let width = slot.map.iter().map(|(f, _)| *f).max().map_or(0, |m| m + 1);
        if width > 0 && width < 64 && value >> (width - 1) & 1 == 1 {
            return i128::from(value) - (1i128 << width);
        }
    }
    // THE BIAS STEP — `W-233`'s finding, `vishlesana.rs` `extract`'s `slot.bias`
    // block, and since `W-236` the T1 `उद्धरणम्`'s `आधारयुक्तम्` too. This
    // reference lacked it, so it agreed with the T1 decoder's WRONG reading of
    // `x8..x15` as `0..7` for as long as both lacked it; it moves with them.
    if slot.bias != 0 {
        let width = slot.map.iter().map(|(f, _)| *f).max().map_or(0, |m| m + 1);
        let bias = u64::from(slot.bias);
        if width > 0 && width < 64 && value < bias {
            let span = 1u64 << width;
            value += span * (bias - value).div_ceil(span);
        }
    }
    i128::from(value)
}

/// Put a value into a slot's bits — `encode.rs`'s `Slot::place`, used here only
/// to BUILD the words the sample decodes.
fn ref_place(slot: &RefSlot, value: u64) -> u32 {
    let mut w = 0u32;
    for (from, to) in &slot.map {
        if value >> from & 1 == 1 {
            w |= 1 << to;
        }
    }
    w
}

/// The row of `bits` width whose fixed bits claim `word`, most constrained
/// first and the EARLIER of two equal masks kept — `decode`'s selection.
fn ref_decode(rows: &[RefRow], word: u32, bits: u32) -> Option<&RefRow> {
    let mut best: Option<&RefRow> = None;
    for r in rows {
        if r.bits != bits || word & r.mask != r.pattern {
            continue;
        }
        match best {
            None => best = Some(r),
            Some(b) if b.mask.count_ones() < r.mask.count_ones() => best = Some(r),
            _ => {}
        }
    }
    best
}

/// How many rows of `bits` width claim `word` — `candidates().len()`.
fn ref_candidates(rows: &[RefRow], word: u32, bits: u32) -> usize {
    rows.iter()
        .filter(|r| r.bits == bits && word & r.mask == r.pattern)
        .count()
}

/// One member of a `संरचना` value.
fn member(v: &Value, name: &str) -> Value {
    match v {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no member `{name}`")),
        other => panic!("{other:?} is not a record, so it has no `{name}`"),
    }
}

fn as_text(v: &Value) -> String {
    match v {
        Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("{other:?} is not a run of octets"),
    }
}

fn arena(v: &Value) -> Vec<Value> {
    match v {
        Value::Arena(a) => a.borrow().clone(),
        other => panic!("{other:?} is not an arena"),
    }
}

/// The `(kind, value)` pairs of a `विश्लिष्टम्`.
///
/// **An arena in this language cannot be empty** — `भवति ०` gives it one slot
/// and nothing shrinks it — so an encoding with no operands reports
/// `ॱ दैर्घ्य` १ with `शून्यम्` at entry ०. That reading is the routine's own,
/// stated at `मूल्यसंख्या`, and it is asserted below for `ecall`.
fn decoded_operands(d: &Value) -> Vec<(i128, i128)> {
    let cells = arena(&member(d, "मूल्यानि"));
    if cells.first().is_none_or(Value::is_nil) {
        return Vec::new();
    }
    cells
        .iter()
        .map(|c| {
            (
                member(c, "भेद").as_int().expect("a slot kind is a number"),
                member(c, "मूल्यम्").as_int().expect("an operand is a number"),
            )
        })
        .collect()
}

/// A word built from a row by filling every slot with a DIFFERENT value, so
/// that a decoder returning the right instruction and the wrong operands — or
/// the right operands in the wrong order — still fails.
fn sample_word(row: &RefRow) -> u32 {
    let mut word = row.pattern;
    for (i, s) in row.slots.iter().enumerate() {
        word |= ref_place(s, (i as u64) * 2 + 5);
    }
    word
}

/// One row in every `STEP`, so that the sample walks the whole table's shape
/// without running the decoder 195 times.
///
/// A SAMPLE and not the whole table, and the reason is the one `sample_names`
/// gives above: a single `विश्लेषणम्` walks every row of the file, and the
/// mutation tests reload and re-run the chain once per mutation. Every
/// mutation below is killed by this sample.
const STEP: usize = 13;

/// The same, for the 37 compressed rows — a smaller step because there are
/// fewer of them and because every contested word in the table is one of them.
const NARROW_STEP: usize = 4;

fn disassembler() -> Interpreter {
    load_all(&["encode.t1", "vishlesana.t1"])
}

/// Call `विश्लेषणम्` (or `सङ्कुचितविश्लेषणम्`) and hand back what it returned.
fn decode_word(it: &mut Interpreter, word: u32, bits: u32) -> Value {
    let name = if bits == 16 {
        "विश्लेषणॱसङ्कुचितविश्लेषणम्"
    } else {
        "विश्लेषणॱविश्लेषणम्"
    };
    it.call(name, vec![Value::Int(i128::from(word))], 600_000_000)
        .unwrap_or_else(|e| panic!("{name} 0x{word:08x} runs: {e}"))
}

/// **The floor.** The two sources load and the disassembler's routines have
/// bodies this interpreter can run — every assertion below is meaningless
/// otherwise, and a file of stubs would satisfy a test that only called one.
#[test]
fn the_disassembler_loads_with_every_routine_runnable() {
    let it = disassembler();
    let r = it.report();
    println!(
        "METRIC sadhana_t1_disassembler_routines_loaded {}",
        r.routines
    );
    println!(
        "METRIC sadhana_t1_disassembler_routines_runnable {}",
        r.runnable
    );
    let unrunnable: Vec<String> = it
        .routines()
        .filter(|x| x.module == "विश्लेषण" && !x.is_runnable())
        .map(|x| format!("{}:{} {}", x.module, x.line, x.why_not().unwrap_or("")))
        .collect();
    assert!(
        unrunnable.is_empty(),
        "विश्लेषण carries routines this interpreter cannot run: {}",
        unrunnable.join("; ")
    );
    let declared = it.routines().filter(|x| x.module == "विश्लेषण").count();
    assert!(
        declared >= 30,
        "only {declared} routines loaded from vishlesana.t1"
    );
}

/// **THE ACCEPTANCE.** `विश्लेषणम्` is called with a machine word and the
/// instruction, family and operands it answers are compared against
/// `spec/encodings-riscv64.tsv`, read independently above.
///
/// The words are built from the table's own patterns and field maps, one slot
/// filled per operand with a different number, so that an answer with the
/// right mnemonic and the wrong operands — or the right operands transposed —
/// fails here.
#[test]
fn the_disassembler_answers_the_row_the_table_carries() {
    let rows = encoding_rows();
    assert!(
        rows.len() >= 190,
        "only {} rows read out of spec/encodings-riscv64.tsv; the evidence \
         below rests on the file",
        rows.len()
    );
    let mut it = disassembler();

    let wide: Vec<&RefRow> = rows.iter().filter(|r| r.bits == 32).collect();
    let mut checked = 0usize;
    for row in wide.iter().step_by(STEP) {
        let word = sample_word(row);
        let want = ref_decode(&rows, word, 32).expect("the row claims its own word");
        let got = decode_word(&mut it, word, 32);
        assert!(
            !got.is_nil(),
            "विश्लेषणम् 0x{word:08x} answered शून्यम्; \
             spec/encodings-riscv64.tsv says `{}`",
            want.insn
        );
        assert_eq!(
            as_text(&member(&got, "आज्ञा")),
            want.insn,
            "विश्लेषणम् 0x{word:08x} (built from the row for `{}`)",
            row.insn
        );
        assert_eq!(
            as_text(&member(&got, "कुल")),
            want.family,
            "the family of 0x{word:08x}"
        );
        let expected: Vec<(i128, i128)> = want
            .slots
            .iter()
            .map(|s| (kind_number(&it, &s.kind), ref_extract(s, word)))
            .collect();
        assert_eq!(
            decoded_operands(&got),
            expected,
            "the operands of `{}` at 0x{word:08x}, kinds in सङ्केतन's numbering",
            want.insn
        );
        checked += 1;
    }
    println!("METRIC sadhana_t1_disassembler_words_decoded {checked}");
    assert!(checked >= 10, "only {checked} words decoded");
}

/// The three words `vishlesana.rs`'s own unit tests name, and the one it says
/// is not an instruction.
///
/// These are the same claims that file makes, asked of the T1 port — and each
/// expected answer is ALSO derived from the file, so this cannot pass by the
/// two of us agreeing on a wrong number.
#[test]
fn the_named_words_of_the_rust_original_come_apart_the_same_way() {
    let rows = encoding_rows();
    let mut it = disassembler();

    // add x1, x2, x3 — the value tools/check-toolchain.sh verifies the whole
    // oracle against.
    let d = decode_word(&mut it, 0x0031_00b3, 32);
    assert_eq!(as_text(&member(&d, "आज्ञा")), "add");
    let reg = kind_number(&it, "reg");
    assert_eq!(
        decoded_operands(&d),
        vec![(reg, 1), (reg, 2), (reg, 3)],
        "rd, rs1, rs2"
    );

    // addi a0, a1, -1. Read unsigned this is 4095, which reassembles to the
    // same word and is still the wrong number — the defect a round trip alone
    // cannot see.
    let d = decode_word(&mut it, 0xfff5_8513, 32);
    assert_eq!(as_text(&member(&d, "आज्ञा")), "addi");
    assert_eq!(
        decoded_operands(&d)[2].1,
        -1,
        "the immediate is signed, and सचिह्नतत्कालभेद is what says so"
    );

    // From spec/bare-metal.sas, the loop branch: blt t1, t2, -8.
    let d = decode_word(&mut it, 0xfe73_4ce3, 32);
    assert_eq!(as_text(&member(&d, "आज्ञा")), "blt");
    assert_eq!(
        decoded_operands(&d)[2].1,
        -8,
        "the displacement is signed, and its sign is the top MAPPED bit"
    );

    // All ones is not an instruction, and the file is what says so.
    assert!(
        ref_decode(&rows, 0xffff_ffff, 32).is_none(),
        "spec/encodings-riscv64.tsv would have to claim 0xffffffff for this \
         test to be asking the right question"
    );
    assert!(
        decode_word(&mut it, 0xffff_ffff, 32).is_nil(),
        "विश्लेषणम् invented an instruction for 0xffffffff"
    );
}

/// `ecall` fixes every bit and has NO operands, and this is the row that
/// proves the empty-arena reading is real rather than described.
#[test]
fn an_encoding_with_no_operands_reports_none_rather_than_one() {
    let rows = encoding_rows();
    let row = rows
        .iter()
        .find(|r| r.insn == "ecall")
        .expect("spec/encodings-riscv64.tsv carries ecall");
    assert!(
        row.slots.is_empty(),
        "the file gives ecall {} slots; this test is about the row that has \
         none",
        row.slots.len()
    );
    let mut it = disassembler();
    let d = decode_word(&mut it, row.pattern, 32);
    assert_eq!(as_text(&member(&d, "आज्ञा")), "ecall");
    assert!(
        decoded_operands(&d).is_empty(),
        "ecall came back with operands; an arena that cannot be empty was read \
         as a one-entry list"
    );
    // And the routine that says so, asked directly.
    let n = it
        .call("विश्लेषणॱमूल्यसंख्या", vec![d.clone()], 1_000_000)
        .expect("मूल्यसंख्या runs")
        .as_int();
    assert_eq!(n, Some(0), "मूल्यसंख्या must call ecall's operand list empty");
}

/// The 16-bit half of the table, which `विश्लेषणम्` must NOT answer for.
///
/// A 16-bit pattern compared against a 32-bit word matches on the high bits
/// being zero, which is every `c.nop`-shaped encoding claiming every word that
/// happens to start with one. That is why the width is a parameter of
/// `सङ्केतपङ्क्तिः`, and this is the test that would notice it being dropped.
#[test]
fn the_compressed_half_is_decoded_by_its_own_entry_point() {
    let rows = encoding_rows();
    let narrow: Vec<&RefRow> = rows.iter().filter(|r| r.bits == 16).collect();
    assert!(
        narrow.len() >= 30,
        "only {} compressed rows in the file",
        narrow.len()
    );
    let mut it = disassembler();
    let mut checked = 0usize;
    for row in narrow.iter().step_by(NARROW_STEP) {
        let word = sample_word(row);
        let want = ref_decode(&rows, word, 16).expect("the row claims its own word");
        let got = decode_word(&mut it, word, 16);
        assert!(
            !got.is_nil(),
            "सङ्कुचितविश्लेषणम् 0x{word:04x} answered शून्यम्; the file says `{}`",
            want.insn
        );
        assert_eq!(as_text(&member(&got, "आज्ञा")), want.insn);
        let expected: Vec<(i128, i128)> = want
            .slots
            .iter()
            .map(|s| (kind_number(&it, &s.kind), ref_extract(s, word)))
            .collect();
        assert_eq!(
            decoded_operands(&got),
            expected,
            "the operands of 0x{word:04x}"
        );
        checked += 1;
    }
    println!("METRIC sadhana_t1_disassembler_compressed_decoded {checked}");
    assert!(checked >= 8, "only {checked} compressed words decoded");

    // The separation itself: a compressed word handed to the WIDE entry point
    // must not be claimed by a 16-bit row.
    let cnop = rows
        .iter()
        .find(|r| r.insn == "c.nop")
        .expect("the file carries c.nop");
    let wide = decode_word(&mut it, cnop.pattern, 32);
    let by_file = ref_decode(&rows, cnop.pattern, 32);
    assert_eq!(
        wide.is_nil(),
        by_file.is_none(),
        "विश्लेषणम् and the file disagree about whether 0x{:08x} is a 32-bit \
         instruction",
        cnop.pattern
    );
}

/// `सम्भाविनः` and `सम्भाविसंख्या` — every row that claims a word, and how
/// many. **More than one is a fault in the table**, and this asserts the file
/// still has none while checking that the T1 walk finds what Rust's does.
#[test]
fn the_candidate_walk_finds_exactly_what_the_file_claims() {
    let rows = encoding_rows();
    let mut it = disassembler();
    let wide: Vec<&RefRow> = rows.iter().filter(|r| r.bits == 32).collect();
    let mut checked = 0usize;
    for row in wide.iter().step_by(STEP) {
        let word = sample_word(row);
        let want = ref_candidates(&rows, word, 32);
        assert_eq!(
            want, 1,
            "spec/encodings-riscv64.tsv gives 0x{word:08x} {want} candidates; \
             two rows constraining the same bits to the same values are the \
             same instruction and this is a fault in the table"
        );
        let got = it
            .call(
                "विश्लेषणॱसम्भाविसंख्या",
                vec![Value::Int(i128::from(word)), Value::Int(32)],
                600_000_000,
            )
            .expect("सम्भाविसंख्या runs")
            .as_int();
        assert_eq!(
            got,
            Some(want as i128),
            "सम्भाविसंख्या 0x{word:08x} disagrees with the file"
        );

        // And the built candidates themselves, for the same word.
        let built = arena(
            &it.call(
                "विश्लेषणॱसम्भाविनः",
                vec![Value::Int(i128::from(word))],
                600_000_000,
            )
            .expect("सम्भाविनः runs"),
        );
        assert_eq!(
            built.len(),
            want,
            "सम्भाविनः 0x{word:08x} built the wrong number of rows"
        );
        let e = &built[0];
        let by_file = ref_decode(&rows, word, 32).expect("one candidate");
        assert_eq!(as_text(&member(e, "आज्ञा")), by_file.insn);
        assert_eq!(as_text(&member(e, "कुल")), by_file.family);
        assert_eq!(
            member(e, "आकृति").as_int(),
            Some(i128::from(by_file.pattern))
        );
        assert_eq!(member(e, "आवरण").as_int(), Some(i128::from(by_file.mask)));
        assert_eq!(member(e, "अंशसंख्या").as_int(), Some(32));
        checked += 1;
    }
    println!("METRIC sadhana_t1_disassembler_candidate_words {checked}");
    assert!(checked >= 10, "only {checked} words walked");

    // An undefined word has NO candidates, and the store that cannot be empty
    // still says so — `सम्भाविसंख्या` is what a caller must ask.
    let none = it
        .call(
            "विश्लेषणॱसम्भाविसंख्या",
            vec![Value::Int(0xffff_ffff), Value::Int(32)],
            600_000_000,
        )
        .expect("सम्भाविसंख्या runs")
        .as_int();
    assert_eq!(none, Some(0), "0xffffffff is claimed by no row of the file");
}

/// `स्थानविश्लेषणम्` — the instruction at an offset in a byte stream, mixed
/// widths and all.
///
/// RISC-V puts the length in the low bits of the first halfword, which is why
/// a decoder can walk a mixed-width stream without being told where the
/// boundaries are (`B-058b1`). The stream below is built to be mixed.
#[test]
fn an_instruction_is_found_at_an_offset_in_a_stream() {
    let rows = encoding_rows();
    let wide = rows
        .iter()
        .find(|r| r.insn == "add")
        .expect("the file carries add");
    let narrow = rows
        .iter()
        .find(|r| r.bits == 16 && !r.slots.is_empty())
        .expect("the file carries a compressed row with operands");

    let w32 = sample_word(wide);
    let w16 = sample_word(narrow) as u16;

    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&w16.to_le_bytes());
    bytes.extend_from_slice(&w32.to_le_bytes());

    let mut it = disassembler();
    let call = |it: &mut Interpreter, at: i128| {
        it.call(
            "विश्लेषणॱस्थानविश्लेषणम्",
            vec![Value::Octets(Octets::new(&bytes)), Value::Int(at)],
            600_000_000,
        )
        .expect("स्थानविश्लेषणम् runs")
    };

    let at0 = call(&mut it, 0);
    assert_eq!(
        as_text(&member(&at0, "आज्ञा")),
        narrow.insn,
        "offset ० is the compressed form; its low two bits are not ०ब११"
    );
    let at2 = call(&mut it, 2);
    assert_eq!(
        as_text(&member(&at2, "आज्ञा")),
        wide.insn,
        "offset २ is the wide form, and only the halfword at ० could have said \
         where it began"
    );
    // A read that runs off the end must be शून्यम् and not a halfword of
    // zeroes, which is a legal compressed instruction.
    assert!(
        call(&mut it, (bytes.len() - 1) as i128).is_nil(),
        "a short read became an instruction"
    );
}

/// **THE FIXPOINT.** Decode a word and put it back together: if
/// `पुनःसंयोजनम्` does not reproduce the word `विश्लेषणम्` was handed, the
/// table is inconsistent with itself in a way the encoder alone can never
/// notice.
#[test]
fn what_comes_apart_goes_back_together_as_the_same_word() {
    let rows = encoding_rows();
    let mut it = disassembler();
    let wide: Vec<&RefRow> = rows.iter().filter(|r| r.bits == 32).collect();
    let mut checked = 0usize;
    for row in wide.iter().step_by(STEP) {
        let word = sample_word(row);
        let d = decode_word(&mut it, word, 32);
        let back = it
            .call("विश्लेषणॱपुनःसंयोजनम्", vec![d.clone()], 600_000_000)
            .expect("पुनःसंयोजनम् runs")
            .as_int();
        assert_eq!(
            back,
            Some(i128::from(word)),
            "0x{word:08x} decoded as `{}` and reassembled to {back:?}",
            as_text(&member(&d, "आज्ञा"))
        );
        checked += 1;
    }
    println!("METRIC sadhana_t1_disassembler_roundtrips {checked}");
    assert!(checked >= 10, "only {checked} round trips");
}

/// A word that MORE THAN ONE row of the file claims, and the row the selection
/// rule must pick for it.
///
/// **Asked of the file rather than named.** The obvious candidate was `ecall`
/// — `vishlesana.rs`'s own comment says it "fixes every bit and shares its
/// opcode with the CSR instructions" — and in THIS table it does not: nothing
/// else matches `0x00000073`, so it would have made a mutation test that could
/// never fail. Every genuinely contested word in the table is in the
/// compressed half (`c.nop` inside `c.addi`, `c.jr` inside `c.mv`, `c.ebreak`
/// inside `c.add`), which is also why the two halves are decoded separately.
fn a_contested_word(rows: &[RefRow]) -> (u32, &RefRow, &RefRow) {
    for r in rows {
        if r.bits != 16 {
            continue;
        }
        let word = r.pattern;
        let claimants: Vec<&RefRow> = rows
            .iter()
            .filter(|x| x.bits == 16 && word & x.mask == x.pattern)
            .collect();
        if claimants.len() < 2 {
            continue;
        }
        let most = ref_decode(rows, word, 16).expect("a claimant");
        let least = claimants
            .iter()
            .min_by_key(|x| x.mask.count_ones())
            .expect("a claimant");
        if most.insn != least.insn {
            return (word, most, least);
        }
    }
    panic!(
        "no word in spec/encodings-riscv64.tsv is claimed by two rows fixing \
         different numbers of bits, so the selection rule has nothing to \
         choose between and no mutation of it can be evidence"
    );
}

/// **The mutation `D-002j`'s discipline asks for, on the selection rule.**
///
/// `निर्णायकसङ्केतः` and `सङ्केतपङ्क्तिः` both say in their margins that the
/// most constrained match is the more specific claim, and that the comparison
/// is a STRICT `न्यूनम्` so a tie keeps the earlier row. Reverse it and the
/// LEAST constrained row wins.
///
/// A test that counted routines, or grepped for `अंशगणना`, would stay green
/// through this edit — the source still has both, and both still run.
#[test]
fn reversing_the_most_constrained_rule_changes_which_row_claims_a_word() {
    let rows = encoding_rows();
    let (word, most, least) = a_contested_word(&rows);

    // The unmutated answer first, so that this test asserts the rule as well
    // as its inversion.
    let mut real = disassembler();
    assert_eq!(
        as_text(&member(&decode_word(&mut real, word, 16), "आज्ञा")),
        most.insn,
        "0x{word:04x} is claimed by more than one row of the file and the most \
         constrained is `{}`",
        most.insn
    );

    let broken = mutate(
        &source("vishlesana.t1"),
        "यदि वरितबन्धः न्यूनम् बद्धांशाः आदि",
        "यदि वरितबन्धः अधिकम् बद्धांशाः आदि",
    );
    let mut it = Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", source("encode.t1").as_str()),
            ("vishlesana.t1", broken.as_str()),
        ],
        &spec_root(),
    )
    .expect("the mutated source still loads");

    let got = as_text(&member(&decode_word(&mut it, word, 16), "आज्ञा"));
    assert_eq!(
        got, least.insn,
        "with the comparison reversed the LEAST constrained row must win at \
         0x{word:04x} — `{}` rather than `{}` — and it answered `{got}`",
        least.insn, most.insn
    );
}

/// **The mutation on the sign.** `उद्धरणम्` asks `चिह्नितावकाशः` whether a
/// slot is two's complement, and `अवकाशभेदाङ्कः` is what tells `simm` from
/// `imm`. Make `simm` read as `imm` and `addi a0, a1, -1` comes back as 4095 —
/// which REASSEMBLES TO THE SAME WORD and is still the wrong number.
///
/// That is the whole reason this mutation is here rather than a round-trip
/// one: the fixpoint test above passes either way.
#[test]
fn reading_a_signed_field_as_unsigned_is_caught() {
    let broken = mutate(
        &source("vishlesana.t1"),
        "प्रत्यागमनम् सङ्केतनॱसचिह्नतत्कालभेद ।",
        "प्रत्यागमनम् सङ्केतनॱतत्कालभेद ।",
    );
    let mut it = Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", source("encode.t1").as_str()),
            ("vishlesana.t1", broken.as_str()),
        ],
        &spec_root(),
    )
    .expect("the mutated source still loads");

    let d = it
        .call("विश्लेषणॱविश्लेषणम्", vec![Value::Int(0xfff5_8513)], 600_000_000)
        .expect("विश्लेषणम् still runs");
    assert_eq!(as_text(&member(&d, "आज्ञा")), "addi");
    assert_eq!(
        decoded_operands(&d)[2].1,
        4095,
        "with simm numbered as imm the immediate must come back unsigned; if \
         it is still ऋण१ the sign is not coming from the slot kind at all"
    );

    // And the fixpoint is blind to it, which is the point.
    let back = it
        .call("विश्लेषणॱपुनःसंयोजनम्", vec![d], 600_000_000)
        .expect("पुनःसंयोजनम् runs")
        .as_int();
    assert_eq!(
        back,
        Some(0xfff5_8513),
        "4095 reassembles to the same word — this is why the operand value is \
         asserted and not only the round trip"
    );
}

/// **The mutation on the `(none)` cell.** Six rows say in words that they have
/// no operands. Invert the test and every one of them grows one while every
/// row that HAS operands loses them all.
#[test]
fn misreading_the_no_operands_cell_is_caught() {
    let broken = mutate(
        &source("vishlesana.t1"),
        "यदि पाठ्यम् अङ्कः क्षेत्रादिः अन्तः समम् ४० आदि",
        "यदि पाठ्यम् अङ्कः क्षेत्रादिः अन्तः असमम् ४० आदि",
    );
    let mut it = Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", source("encode.t1").as_str()),
            ("vishlesana.t1", broken.as_str()),
        ],
        &spec_root(),
    )
    .expect("the mutated source still loads");

    let d = it
        .call("विश्लेषणॱविश्लेषणम्", vec![Value::Int(0x0031_00b3)], 600_000_000)
        .expect("विश्लेषणम् still runs");
    assert_eq!(as_text(&member(&d, "आज्ञा")), "add");
    assert!(
        decoded_operands(&d).is_empty(),
        "with the `(none)` test inverted, `add` must lose its three registers"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// THE CODE EMITTERS — `उत्सर्जन ॱ यवनाधिकरणवचनम्` and `यवनवृत्त्युत्सर्जनम्`:
// `x86_64.rs`'s `format_location` and `emit_function`, **executed, with the
// octets they produce asserted byte for byte.**
//
// `W-237`, 2026-09-04: `कार्यक्रमोत्सर्जनम्` and `emit.rs`'s `emit_program` —
// the pair that called itself "T0 emission for riscv64" and wrote a text
// `सङ्केतन` refused on every line (research/25 §1.2) — were DELETED, and the
// three tests that asserted that text's octets went with them
// (`the_t0_emitter_writes_the_program_the_rust_original_writes`,
// `…renders_a_spill_as_a_spill`, `…over_no_function_emits_only_the_directive`).
// The T0 emitter's octets are asserted in `tests/t1_exec_riscv.rs` against
// `riscv64.rs`, and run on the machine in `crates/yantra/tests`. The OUTPUT
// BUFFER the retired routine drove — `यत्यष्टकम्`, `अष्टकयोजनम्`,
// `देवनागराङ्कः` — is what both the AMD64 emitter and the T1 RISC-V twin
// still write through, so its mutations below are now killed through the
// AMD64 emitter.
//
// `a_real_sas_program_assembles_through_the_t1_chain` above closes with the
// sentence these tests exist to strike: *"It does NOT show an ELF being
// written; `उत्सर्जन`'s two emitter entry points are still stubs."* They are
// not stubs now, and the only way to say so is to run them and read the bytes.
//
// **THE NEWLINE IS THE WHOLE POINT AND IT IS ASSERTED AS A BYTE.** ADR-0017
// gives a string literal no newline and is NOT amended; the owner's decision of
// 2026-08-30 is that the emitter appends U+000A as an octet. So every assertion
// below checks WHERE `०x0A` falls and not merely that some text came out — a
// `contains` would pass over an emitter that ran all its lines together, which
// is exactly the emitter this row was filed to avoid.
// ─────────────────────────────────────────────────────────────────────────

/// LINE FEED, named once. ADR-0018 calls it `यतिः`; `उत्सर्जन` holds it in
/// `यत्यष्टकम्`, read out of that word rather than typed as `१०`.
const YATI: u8 = 0x0A;

fn text_of(v: &Value) -> String {
    match v.octets() {
        Some(o) => String::from_utf8(o.as_slice().to_vec()).expect("the emitted run is UTF-8"),
        None => panic!("{v:?} is not a run of octets"),
    }
}

fn call_ok(it: &mut Interpreter, name: &str, args: Vec<Value>) -> Value {
    it.call(name, args, 60_000_000)
        .unwrap_or_else(|e| panic!("`{name}` runs: {e}"))
}

fn kind(it: &Interpreter, name: &str) -> Value {
    Value::Int(
        it.global(name)
            .and_then(Value::as_int)
            .unwrap_or_else(|| panic!("`{name}` is a global of the loaded corpus")),
    )
}

/// **The function BOTH Rust tests build** — `emit.rs:107` and `x86_64.rs:135`
/// are the same three instructions and the same terminator:
/// `v1 = ConstInt(10)`, `v2 = ConstInt(20)`, `v3 = Add(v1, v2)`,
/// `Return(Some(v3))`.
///
/// Built through `मध्यरूप`'s own builder — `नवमूल्यम्`, `आज्ञायोजनम्`,
/// `अवसानरचना`, `पर्वयोजनम्`, `वृत्तियोजनम्` — and not by writing its arenas
/// from Rust. Two ports are exercised here rather than one, and an emitter fed
/// a hand-written arena would be checked against a fixture instead of against
/// the IR the compiler actually produces.
///
/// Answers the function's index in `वृत्तिकोश`.
fn build_the_rust_tests_function(it: &mut Interpreter) -> i128 {
    let const_kind = kind(it, "ध्रुवाज्ञाभेद");
    let add_kind = kind(it, "योगाज्ञाभेद");
    let return_kind = kind(it, "प्रत्यागमनावसानभेद");

    call_ok(it, "मध्यरूपॱआरम्भः", vec![]);
    // ० is मध्यरूप's "no such value" — see `ON INDEX ZERO` in `ir.t1`.
    let nil_val = call_ok(it, "मध्यरूपॱमूल्याङ्कनम्", vec![Value::Int(0)]);
    let v1 = call_ok(it, "मध्यरूपॱनवमूल्यम्", vec![Value::Int(0)]);
    let v2 = call_ok(it, "मध्यरूपॱनवमूल्यम्", vec![Value::Int(0)]);
    let v3 = call_ok(it, "मध्यरूपॱनवमूल्यम्", vec![Value::Int(0)]);
    let b0 = call_ok(it, "मध्यरूपॱनवपर्व", vec![Value::Int(0)]);

    let first = call_ok(
        it,
        "मध्यरूपॱआज्ञायोजनम्",
        vec![
            const_kind.clone(),
            v1.clone(),
            Value::Int(10),
            Value::Int(0),
            nil_val.clone(),
            nil_val.clone(),
            Value::Int(0),
        ],
    )
    .as_int()
    .expect("आज्ञायोजनम् answers an index");
    call_ok(
        it,
        "मध्यरूपॱआज्ञायोजनम्",
        vec![
            const_kind,
            v2.clone(),
            Value::Int(20),
            Value::Int(0),
            nil_val.clone(),
            nil_val,
            Value::Int(0),
        ],
    );
    call_ok(
        it,
        "मध्यरूपॱआज्ञायोजनम्",
        vec![
            add_kind,
            v3.clone(),
            Value::Int(0),
            Value::Int(0),
            v1,
            v2,
            Value::Int(0),
        ],
    );

    let terminator = call_ok(it, "मध्यरूपॱअवसानरचना", vec![return_kind, v3, b0.clone()]);
    let block = call_ok(
        it,
        "मध्यरूपॱपर्वयोजनम्",
        vec![b0.clone(), Value::Int(first), Value::Int(3), terminator],
    )
    .as_int()
    .expect("पर्वयोजनम् answers an index");

    call_ok(
        it,
        "मध्यरूपॱवृत्तियोजनम्",
        vec![Value::Int(0), Value::Int(block), Value::Int(1), b0],
    )
    .as_int()
    .expect("वृत्तियोजनम् answers an index")
}

/// `AllocationMap { locations, num_spills }`, built here rather than by running
/// the allocator — **which is what `emit.rs:126` and `x86_64.rs:154` do too**,
/// and for the same reason: an emitter's test is about the TEXT a given
/// assignment produces, and running the allocator would make it a test of the
/// allocator's answer instead.
///
/// It is also the only thing that works today. See
/// `the_allocator_cannot_be_run_over_a_real_function_and_this_is_the_reason`.
fn allocation_map(highest_value: i128, spills: i128) -> Value {
    let mut m = HashMap::new();
    m.insert("अधिकरणसंख्यान".to_string(), Value::Int(highest_value));
    m.insert("निक्षेपसंख्यान".to_string(), Value::Int(spills));
    Value::Record(Rc::new(RefCell::new(m)))
}

/// Write `locations` into `उत्सर्जन ॱ अधिकरणकोश`, the array
/// `HashMap<ValueId, Location>` becomes. Each entry is `(value number, kind,
/// register, spill slot)` and is built by CALLING `अधिकरणरचना`, so the record
/// is the port's own and not a shape this test invented.
fn set_locations(it: &mut Interpreter, locations: &[(usize, i128, i128, i128)]) {
    let mut built = Vec::new();
    for (v, k, reg, slot) in locations {
        built.push((
            *v,
            call_ok(
                it,
                "उत्सर्जनॱअधिकरणरचना",
                vec![Value::Int(*k), Value::Int(*reg), Value::Int(*slot)],
            ),
        ));
    }
    let arena = match it.global("अधिकरणकोश") {
        Some(Value::Arena(a)) => Rc::clone(a),
        other => panic!("अधिकरणकोश is an arena, not {other:?}"),
    };
    let mut a = arena.borrow_mut();
    let hi = built.iter().map(|(v, _)| *v).max().unwrap_or(0);
    if a.len() <= hi {
        a.resize(hi + 1, Value::Nil);
    }
    for (v, loc) in built {
        a[v] = loc;
    }
}

/// The `Location` tag numbers, read from the module rather than written twice.
fn location_kinds(it: &Interpreter) -> (i128, i128) {
    (
        it.global("कोष्ठाधिकरणभेद")
            .and_then(Value::as_int)
            .expect("कोष्ठाधिकरणभेद is a global"),
        it.global("निक्षेपाधिकरणभेद")
            .and_then(Value::as_int)
            .expect("निक्षेपाधिकरणभेद is a global"),
    )
}

/// Load `ir.t1` plus a (possibly mutated) `utsarjana.t1`, build the function,
/// install `locations`, and run `routine` over it.
fn emit(utsarjana: &str, routine: &str, locations: &[(usize, i128, i128, i128)]) -> String {
    let ir = source("ir.t1");
    let mut it = Interpreter::load(
        &[("ir.t1", ir.as_str()), ("utsarjana.t1", utsarjana)],
        &spec_root(),
    )
    .expect("ir.t1 and utsarjana.t1 load");
    let func = build_the_rust_tests_function(&mut it);
    set_locations(&mut it, locations);
    let highest = locations
        .iter()
        .map(|(v, ..)| *v as i128)
        .max()
        .unwrap_or(0);
    let spills = locations.iter().filter(|(_, k, ..)| *k == 2).count() as i128;
    let alloc = allocation_map(highest, spills);
    text_of(&call_ok(&mut it, routine, vec![Value::Int(func), alloc]))
}

/// `x86_64.rs`'s own fixture: `Register(0)`, `Register(1)`, `Register(0)` —
/// the destination reusing the LEFT operand's register, which is the case its
/// `if rn_out != rn1` exists for.
fn amd64_fixture() -> Vec<(usize, i128, i128, i128)> {
    vec![(1, 1, 0, 0), (2, 1, 1, 0), (3, 1, 0, 0)]
}

/// **`D-002e`'s acceptance for `X86_64Emitter::format_location`**, and the
/// place this row's punctuation decision is checked rather than described.
///
/// `x86_64.rs` writes `%{name}` and `-{(slot+1)*8}(%rbp)`. `%` is not rendered
/// (the operand's own shape carries the role — see the routine's margin), `(`
/// and `)` are doc 15 §6.2's C-15-2 keywords `आरभ्य`/`समाप्तम्`, and `-` is the
/// frozen numeral production's `ऋण`, which `देवनागराङ्कः` already owned.
#[test]
fn the_amd64_location_text_is_the_punctuation_decision_this_row_took() {
    let mut it = load_one("utsarjana.t1");
    let (register, spill) = location_kinds(&it);

    let loc = |it: &mut Interpreter, k: i128, reg: i128, slot: i128| {
        let l = call_ok(
            it,
            "उत्सर्जनॱअधिकरणरचना",
            vec![Value::Int(k), Value::Int(reg), Value::Int(slot)],
        );
        text_of(&call_ok(it, "उत्सर्जनॱयवनाधिकरणवचनम्", vec![l]))
    };

    // The names come from spec/registers-amd64.tsv through `कोष्ठनाम`, which
    // `the_amd64_register_names_are_the_spec_table_and_not_a_second_copy_of_it`
    // already checks against that file. What is new here is the FRAME.
    assert_eq!(loc(&mut it, register, 0, 0), "सञ्चयः");
    assert_eq!(loc(&mut it, register, 2, 0), "गणकः");
    assert_eq!(loc(&mut it, register, 6, 0), "कोष्ठ८");

    // `(slot + 1) * 8` — `-8(%rbp)` and `-24(%rbp)`.
    assert_eq!(loc(&mut it, spill, 0, 0), "ऋण८ आरभ्य आधारसूचकः समाप्तम्");
    assert_eq!(loc(&mut it, spill, 0, 2), "ऋण२४ आरभ्य आधारसूचकः समाप्तम्");

    // भेद ० is `locations.get(v) == None`, which x86_64.rs never reaches.
    assert_eq!(loc(&mut it, 0, 0, 0), "");

    // NOT ONE OCTET OF LATIN, AND THAT IS THE DECISION UNDER TEST. ADR-0029
    // admits no escape hatch, so an emitter that reached for `%rbp` or `rax`
    // must fail here rather than merely read oddly.
    for k in [register, spill] {
        let t = loc(&mut it, k, 5, 3);
        assert!(
            !t.bytes()
                .any(|b| b.is_ascii_alphanumeric() || b == b'%' || b == b'(' || b == b')'),
            "`{t}` carries Latin or ASCII punctuation; ADR-0029 opens no escape"
        );
    }
}

/// **`D-002e`'s acceptance for `X86_64Emitter::emit_function`.**
///
/// Twelve `push_str` calls in the Rust, every one ending in a newline and most
/// beginning with two spaces — so this is the routine ADR-0018 and the owner's
/// octet decision were both about, and the assertion is again about where
/// `०x0A` falls. The three lines `x86_64.rs:167-169` asserts by `contains` are
/// all here, and so is everything between them.
#[test]
fn the_amd64_emitter_writes_the_function_the_rust_original_writes() {
    let text = emit(
        &source("utsarjana.t1"),
        "उत्सर्जनॱयवनवृत्त्युत्सर्जनम्",
        &amd64_fixture(),
    );
    println!("METRIC sadhana_t1_amd64_emitted_octets {}", text.len());

    let expected = [
        "  वैश्विकम् मुख्यम्",
        "मुख्यम्ॱॱ",
        "  स्तूपारोपः आधारसूचकः",
        "  निधेहि स्तूपसूचकःऽ आधारसूचकः",
        "  निधेहि १०ऽ सञ्चयः",
        "  निधेहि २०ऽ आधारः",
        "  योगः आधारःऽ सञ्चयः",
        "  स्तूपापहारः आधारसूचकः",
        "  प्रत्यागमनम्",
        "",
    ]
    .join("\n");
    assert_eq!(
        text, expected,
        "यवनवृत्त्युत्सर्जनम् emitted text the port does not account for"
    );

    let bytes = text.as_bytes();
    assert_eq!(
        bytes.iter().filter(|b| **b == YATI).count(),
        9,
        "nine emitted lines, nine line feeds"
    );
    assert_eq!(bytes.last(), Some(&YATI), "the last line is terminated too");

    // `main:` is the ONE line with no indent — x86_64.rs:22 writes it flush —
    // and every other line begins with exactly two spaces.
    let lines: Vec<&str> = text.split(char::from(YATI)).collect();
    assert_eq!(lines[1], "मुख्यम्ॱॱ", "the label is not indented");
    for (i, line) in lines[..lines.len() - 1].iter().enumerate() {
        if i == 1 {
            continue;
        }
        assert!(
            line.starts_with("  ") && !line.starts_with("   "),
            "`{line}` is not indented by exactly two spaces"
        );
    }

    // NO `movq %rax, %rax`. `x86_64.rs:65` skips the move when the destination
    // already holds the left operand, and this fixture is that case.
    assert!(
        !text.contains("  निधेहि सञ्चयःऽ सञ्चयः"),
        "the destination already holds the left operand; the move is dead"
    );
    assert!(
        !text.bytes().any(|b| b.is_ascii_alphanumeric()),
        "the AMD64 emitter produced Latin or ASCII digits"
    );
}

/// **The spill path through the whole AMD64 emitter.** `emit_function` reaches
/// `-{n}(%rbp)` only through a spilled operand, and this is that operand.
#[test]
fn a_spilled_value_reaches_the_amd64_frame_syntax() {
    let text = emit(
        &source("utsarjana.t1"),
        "उत्सर्जनॱयवनवृत्त्युत्सर्जनम्",
        &[(1, 1, 0, 0), (2, 2, 0, 0), (3, 1, 0, 0)],
    );
    println!("METRIC sadhana_t1_amd64_spilled_octets {}", text.len());
    assert!(
        text.contains("  निधेहि २०ऽ ऋण८ आरभ्य आधारसूचकः समाप्तम्"),
        "the spilled value's own definition must store to the frame: {text}"
    );
    assert!(
        text.contains("  योगः ऋण८ आरभ्य आधारसूचकः समाप्तम्ऽ सञ्चयः"),
        "and the add must read it back from the frame: {text}"
    );
    assert_eq!(
        text.as_bytes().last(),
        Some(&YATI),
        "the epilogue still ends in a newline when a value spills"
    );
}

/// One mutation of `utsarjana.t1`'s OUTPUT BUFFER: what to break, which
/// routine it lives in, and why the break is invisible to a text ratchet.
/// Killed through the AMD64 emitter since `W-237` retired the T0 pair that
/// first drove them; the buffer is the same one the RISC-V twin writes through.
///
/// `(routine, what, from, to)`. `mutate` refuses anything that does not match
/// exactly once, so a mutation that quietly matched nothing cannot pass here by
/// doing nothing.
const EMITTER_MUTATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "यत्यष्टकम्",
        "the line end stops being a LINE FEED and becomes a space — the exact \
         defect a `contains` assertion cannot see",
        "सार्वजनिक चरः यत्यष्टकम् ॱॱ अ६४ भवति यतिः अङ्कः ० अन्तः ।",
        "सार्वजनिक चरः यत्यष्टकम् ॱॱ अ६४ भवति विवरम् अङ्कः ० अन्तः ।",
    ),
    (
        "अष्टकयोजनम्",
        "the output cursor stops advancing, so every octet overwrites the first",
        "निर्गमसूचकाङ्क भवति आरभ्य निर्गमसूचकाङ्क योगः १ समाप्तम् ।",
        "निर्गमसूचकाङ्क भवति आरभ्य निर्गमसूचकाङ्क योगः ० समाप्तम् ।",
    ),
    (
        "देवनागराङ्कः",
        "the digit walk never terminates, so a numeral costs the whole run",
        "यावत् संख्यानम् अधिकम् ० आदि",
        "यावत् संख्यानम् बृहत्समम् ० आदि",
    ),
    // `अधिकरणवचनम्`'s and `कार्यक्रमोत्सर्जनम्`'s rows left with the routines
    // (`W-237`); `यवनाधिकरणवचनम्` is the AMD64 emitter's own location text and
    // `reversing_the_register_equality_…` below is its mutation.
];

#[test]
fn breaking_one_line_of_the_output_buffer_changes_the_octets_the_amd64_emitter_writes() {
    let truth = emit(
        &source("utsarjana.t1"),
        "उत्सर्जनॱयवनवृत्त्युत्सर्जनम्",
        &amd64_fixture(),
    );
    assert!(
        truth.contains(char::from(YATI)),
        "the unmutated emitter wrote no newline; there is no baseline to differ from"
    );
    let mut proved = 0usize;
    for (routine, what, from, to) in EMITTER_MUTATIONS {
        let broken = mutate(&source("utsarjana.t1"), from, to);
        // A mutation may make the routine REFUSE rather than emit differently —
        // `देवनागराङ्कः`'s loop guard stops terminating and exhausts its fuel —
        // and a refusal differs from the true text the same way a wrong answer
        // does, so both are killed the same way.
        let answer =
            std::panic::catch_unwind(|| emit(&broken, "उत्सर्जनॱयवनवृत्त्युत्सर्जनम्", &amd64_fixture()));
        assert_ne!(
            answer.as_ref().ok().map(String::as_str),
            Some(truth.as_str()),
            "MUTATION SURVIVED: `{routine}` — {what}. \
             `the_amd64_emitter_writes_the_function_the_rust_original_writes` is \
             measuring text and not behaviour."
        );
        proved += 1;
    }
    println!("METRIC sadhana_t1_emitter_mutations_killed {proved}");
    assert_eq!(
        proved,
        EMITTER_MUTATIONS.len(),
        "every mutation must be killed"
    );
}

/// The AMD64 emitter's own mutation, in a routine none of those above touch:
/// `अधिकरणसाम्यम्` is the structural stand-in for `x86_64.rs`'s
/// `rn_out != rn1`, the one comparison that decides whether the move into the
/// destination register is emitted at all.
///
/// Chosen because it leaves the source's shape untouched — same routine, same
/// arity, same arms, one comparison reversed — so every ratchet in
/// `t1_sources.rs` stays green straight through it.
#[test]
fn reversing_the_register_equality_changes_what_the_amd64_emitter_writes() {
    let truth = emit(
        &source("utsarjana.t1"),
        "उत्सर्जनॱयवनवृत्त्युत्सर्जनम्",
        &amd64_fixture(),
    );
    assert!(
        !truth.contains("  निधेहि सञ्चयःऽ सञ्चयः"),
        "with rn_out == rn1 the move must be absent, or reversing the test \
         that guards it proves nothing: {truth}"
    );
    let broken = mutate(
        &source("utsarjana.t1"),
        "यदि एकम् ॱ कोष्ठ असमम् द्वितीयम् ॱ कोष्ठ आदि",
        "यदि एकम् ॱ कोष्ठ समम् द्वितीयम् ॱ कोष्ठ आदि",
    );
    let answer = emit(&broken, "उत्सर्जनॱयवनवृत्त्युत्सर्जनम्", &amd64_fixture());
    assert_ne!(
        answer, truth,
        "MUTATION SURVIVED: two registers that ARE the same now compare \
         different, so the `movq {{rn1}}, {{rn_out}}` line should have appeared"
    );
    assert!(
        answer.contains("  निधेहि सञ्चयःऽ सञ्चयः"),
        "the mutation was expected to add exactly that dead move; it wrote {answer}"
    );
}

/// **`कोष्ठावण्टनम्` RUNS OVER A REAL FUNCTION, and its answer is `regalloc.rs`'s.**
///
/// Until `W-236` this test stood here as A FINDING: the allocator — the whole of
/// `regalloc.rs`, the one part of `utsarjana.t1` that was never a stub — could
/// not be run over a real function, because `आयुर्निर्णयः` asks
/// `आयुरारम्भकोश अङ्कः मूल्यम् अन्तः समम् ०` (Rust's `lifetimes.get(v) == None`)
/// and an arena here is grown only by WRITES, so the first read of value १ was
/// *"entry 1 is outside an arena of 1"*; `आवण्टनारम्भः` cleared `1..=मूल्यसीमा`
/// and `मूल्यसीमा` was ० on the first call. The finding named two repairs and
/// asked to be STRUCK when one landed. `W-236` — whose T1 RISC-V emitter must
/// allocate through this routine to agree with `riscv64.rs` — made the second:
/// `आवण्टनारम्भः` is told how many values `मध्यरूप` handed out
/// (`मध्यरूपॱअग्रिममूल्याङ्क`) and clears that far.
///
/// So this is now the allocator's TWIN-AGREEMENT test: the same three-instruction
/// function on both sides, fourteen registers, and the location of every value
/// compared — `v1` and `v2` in registers ० and १, `v3` reusing १ once both have
/// expired — plus the spill count.
#[test]
fn the_allocator_runs_over_a_real_function_and_agrees_with_regalloc() {
    use sadhana::t1::ast::SymbolId;
    use sadhana::t1::ir::{Block, BlockId, Function, Instruction, Terminator, ValueId};
    use sadhana::t1::regalloc::{Location, allocate_registers};

    let mut it = load_all(&["ir.t1", "utsarjana.t1"]);
    let func = build_the_rust_tests_function(&mut it);
    let alloc = call_ok(
        &mut it,
        "उत्सर्जनॱकोष्ठावण्टनम्",
        vec![Value::Int(func), Value::Int(14)],
    );

    // The Rust side: `emit.rs:107`'s function — the one `build_the_rust_tests_function` builds.
    let (v1, v2, v3) = (ValueId(1), ValueId(2), ValueId(3));
    let mut blocks = HashMap::new();
    blocks.insert(
        BlockId(0),
        Block {
            id: BlockId(0),
            insts: vec![
                (v1, Instruction::ConstInt(10)),
                (v2, Instruction::ConstInt(20)),
                (v3, Instruction::Add(v1, v2)),
            ],
            terminator: Some(Terminator::Return(Some(v3))),
        },
    );
    let rust = allocate_registers(
        &Function {
            name: SymbolId(0),
            blocks,
            entry_block: BlockId(0),
        },
        14,
    );

    let (register, spill) = location_kinds(&it);
    let arena = match it.global("अधिकरणकोश") {
        Some(Value::Arena(a)) => Rc::clone(a),
        other => panic!("अधिकरणकोश is an arena, not {other:?}"),
    };
    let mut agreed = 0;
    for v in 1..=3usize {
        let loc = arena.borrow()[v].clone();
        let t1 = match member(&loc, "भेद").as_int() {
            Some(k) if k == register => Location::Register(
                u8::try_from(member(&loc, "कोष्ठ").as_int().expect("a register number")).unwrap(),
            ),
            Some(k) if k == spill => Location::Spill(
                usize::try_from(member(&loc, "निक्षेप").as_int().expect("a slot")).unwrap(),
            ),
            other => panic!("value {v} has no location: भेद {other:?}"),
        };
        let expected = rust.locations[&ValueId(v)];
        assert_eq!(
            t1, expected,
            "value {v}: the T1 allocator and regalloc.rs disagree"
        );
        agreed += 1;
    }
    assert_eq!(
        member(&alloc, "निक्षेपसंख्यान").as_int(),
        Some(rust.num_spills as i128)
    );
    assert_eq!(rust.locations[&v1], Location::Register(0));
    assert_eq!(rust.locations[&v2], Location::Register(1));
    assert_eq!(
        rust.locations[&v3],
        Location::Register(1),
        "v3 reuses v2's register"
    );
    println!("METRIC sadhana_t1_allocator_twin_agreement {agreed}/3");
}

// ═════════════════════════════════════════════════════════════════════════
// `D-002i` — `अक्षरकोश`, THE MODULE THAT OWNS `sanskrit_text::numeral`.
//
// `sanskrit_text.t1` declared seven routines and all seven were stubs. Five now
// have bodies and are RUN here; the other two are blocked and this file names
// what by.
//
// # Why five routines and not two
//
// Rust's `numeral::value` and `numeral::bits` each return
// `Result<u64, NumeralError>`, and T1 has no tuple. So the fault and the answer
// are separate questions — `मानदोषः`/`मानम्` and `अंशदोषः`/`अंशाः` — and the
// file's own header gives that reason at length. `सङ्ख्या` is `is_numeral`, and
// it is a THIRD question rather than either of the other two: `validate` splits
// the sign and never looks at it again, so `ऋण१` IS a numeral while the
// magnitude reader refuses it, and `validate` does no arithmetic at all, so a
// literal too large is well-formed WRITING that both value readers refuse.
// `the_three_readers_answer_differently_and_that_is_the_point` is that claim as
// an assertion rather than a comment.
//
// # Nothing below is a second copy
//
// `B-058a`'s defect class is a typed second copy that agrees with the first by
// construction. **No expected value here is written down.** Every one comes from
// `rust_reading`, which classifies the token and then hands the digits to
// `u64::from_str_radix` — so the digit weights, the accumulation and the
// overflow decision are all the standard library's. The one thing this file
// authors is the Devanagari-to-ASCII mapping, and that is DERIVED from the code
// points `digit_value` matches on (`०` is `U+0966 + d`, `अ` is `U+0905 + v-10`)
// rather than listed, so a sixteenth hex letter cannot arrive here by a typo.
//
// The corpus of tokens is built the same way: `token_for` RENDERS a `u64` in a
// radix and prepends this language's own prefix, so the reader and the writer
// are graded against each other over 241 tokens rather than over a list someone
// kept in step by hand.
// ═════════════════════════════════════════════════════════════════════════

/// The three radix prefixes and the sign, by code point — `numeral.rs:26-28`
/// and `:144`, which take them from doc 15 §3.3's own table.
const BINARY: &str = "\u{0966}\u{0926}\u{094D}\u{0935}\u{093F}"; // ०द्वि
const OCTAL: &str = "\u{0966}\u{0905}\u{0937}\u{094D}\u{091F}"; // ०अष्ट
const HEX: &str = "\u{0966}\u{0937}\u{094B}\u{0921}\u{094D}"; // ०षोड्
const NEGATIVE: &str = "\u{090B}\u{0923}"; // ऋण

/// `०`–`९` are `U+0966 + d`; `अ आ इ ई उ ऊ` are ten to fifteen at `U+0905 + i`.
fn ascii_of(ch: char) -> Option<char> {
    let cp = ch as u32;
    let v = match cp {
        0x0966..=0x096F => cp - 0x0966,
        0x0905..=0x090A => cp - 0x0905 + 10,
        _ => return None,
    };
    char::from_digit(v, 16)
}

/// The inverse, for BUILDING a token to hand the reader.
fn deva_of(ascii: char) -> char {
    let v = ascii.to_digit(16).expect("a hex digit");
    let cp = if v < 10 { 0x0966 + v } else { 0x0905 + v - 10 };
    char::from_u32(cp).expect("a Devanagari code point")
}

fn deva(ascii: &str) -> String {
    ascii.chars().map(deva_of).collect()
}

/// Write `n` in `radix`, with this language's prefix and digits.
fn token_for(n: u64, radix: u32, negative: bool) -> String {
    let body = match radix {
        2 => format!("{BINARY}{}", deva(&format!("{n:b}"))),
        8 => format!("{OCTAL}{}", deva(&format!("{n:o}"))),
        10 => deva(&format!("{n}")),
        16 => format!("{HEX}{}", deva(&format!("{n:x}"))),
        r => panic!("no such radix: {r}"),
    };
    if negative {
        format!("{NEGATIVE}{body}")
    } else {
        body
    }
}

/// The four codes `sanskrit_text.t1` declares, mirroring `NumeralError`.
const OK: i128 = 0; // अङ्कनिर्दोषः
const MALFORMED: i128 = 1; // अङ्करूपदोषः — NoDigits, BadDigit (P19)
const TOO_LARGE: i128 = 2; // अङ्कातिमानदोषः — TooLarge (P29)
const SIGNED: i128 = 3; // अङ्कचिह्नदोषः — Signed (P30)

/// What Rust makes of the same token — **the whole oracle**.
struct Reading {
    value_code: i128,
    magnitude: i128,
    bits_code: i128,
    bits: i128,
    radix: Option<i128>,
}

fn rust_reading(token: &str) -> Reading {
    let (negative, bare) = match token.strip_prefix(NEGATIVE) {
        Some(rest) => (true, rest),
        None => (false, token),
    };

    // classify: the prefixes BEFORE the bare-digit case, because every one of
    // them begins with ० and ० is itself a digit.
    let mut radix = 0u32;
    let mut digits = bare;
    for (prefix, r) in [(HEX, 16u32), (BINARY, 2), (OCTAL, 8)] {
        if let Some(rest) = bare.strip_prefix(prefix) {
            radix = r;
            digits = rest;
            break;
        }
    }
    let refused = Reading {
        value_code: if negative { SIGNED } else { MALFORMED },
        magnitude: 0,
        bits_code: MALFORMED,
        bits: 0,
        radix: None,
    };
    if radix == 0 {
        match bare.chars().next() {
            Some(c) if (0x0966..=0x096F).contains(&(c as u32)) => {
                radix = 10;
                digits = bare;
            }
            _ => return refused,
        }
    }

    // validate: every character a digit OF THIS RADIX. `from_str_radix` would
    // refuse a bad digit too, but it cannot tell that from an overflow and the
    // two are different codes — so the split is here and the ARITHMETIC is
    // still there.
    let mut ascii = String::new();
    let mut well_formed = !digits.is_empty();
    for ch in digits.chars() {
        match ascii_of(ch).and_then(|a| a.to_digit(16)) {
            Some(v) if v < radix => ascii.push(char::from_digit(v, 16).expect("a digit")),
            _ => {
                well_formed = false;
                break;
            }
        }
    }
    if !well_formed {
        return refused;
    }

    // RUST PARSES IT. Everything above is classification.
    let radix_answer = Some(i128::from(radix));
    match u64::from_str_radix(&ascii, radix) {
        Err(_) => Reading {
            value_code: if negative { SIGNED } else { TOO_LARGE },
            magnitude: 0,
            bits_code: TOO_LARGE,
            bits: 0,
            radix: radix_answer,
        },
        Ok(m) if !negative => Reading {
            value_code: OK,
            magnitude: i128::from(m),
            bits_code: OK,
            bits: i128::from(m),
            radix: radix_answer,
        },
        // 2^63 is representable and 2^63 + 1 is not: the negative range reaches
        // one further than the positive one.
        Ok(m) if m > 1u64 << 63 => Reading {
            value_code: SIGNED,
            magnitude: 0,
            bits_code: TOO_LARGE,
            bits: 0,
            radix: radix_answer,
        },
        Ok(m) => Reading {
            value_code: SIGNED,
            magnitude: 0,
            bits_code: OK,
            bits: i128::from(m.wrapping_neg()),
            radix: radix_answer,
        },
    }
}

/// Magnitudes chosen for the boundaries they sit on, then rendered in all four
/// radices with and without `ऋण` — so no radix is covered by fewer cases than
/// another and the sign is exercised on every one.
fn magnitudes() -> Vec<u64> {
    vec![
        0,
        1,
        2,
        7,
        8,
        9,
        10,
        15,
        16,
        42,
        255,
        256,
        1729,
        4095,
        65535,
        1_000_000,
        u32::MAX as u64,
        u32::MAX as u64 + 1,
        (1u64 << 62) - 1,
        1u64 << 62,
        (1u64 << 63) - 1,
        1u64 << 63,       // the last magnitude `ऋण` can carry
        (1u64 << 63) + 1, // one past it — `bits` must refuse this with ऋण
        u64::MAX - 1,
        u64::MAX,
    ]
}

fn numeral_corpus() -> Vec<String> {
    let mut out = Vec::new();
    for n in magnitudes() {
        for radix in [2u32, 8, 10, 16] {
            for negative in [false, true] {
                out.push(token_for(n, radix, negative));
            }
        }
    }

    // The malformed ones, each BUILT from a well-formed token so that what
    // makes it malformed is one named edit and not a typed constant.
    let ten = token_for(10, 10, false);
    out.push(String::new()); // empty
    out.push(BINARY.to_string()); // a prefix with no digits after it
    out.push(OCTAL.to_string());
    out.push(HEX.to_string());
    out.push(format!("{NEGATIVE}{HEX}")); // signed, and no digits either
    out.push(NEGATIVE.to_string()); // the sign alone
    out.push("\u{0915}".to_string()); // क — an identifier, not a numeral
    out.push(format!("{ten}\u{0915}")); // digits, then a letter
    out.push("1729".to_string()); // ASCII digits are NOT numerals — R-15-1
    out.push(format!("{BINARY}\u{096E}")); // ०द्वि८ — ८ is not a binary digit
    out.push(format!("{OCTAL}\u{096E}")); // ०अष्ट८ — nor an octal one
    out.push(format!("{}\u{0905}", token_for(1, 10, false))); // १अ — अ is not decimal
    out.push(format!("{HEX}\u{0910}")); // ०षोड्ऐ — ऐ is not a hex digit

    // Overflow, SPELLED CORRECTLY: 2^64 and a good deal more, in every radix.
    for radix in [2u32, 8, 10, 16] {
        for over in [1u128 << 64, (1u128 << 64) + 1, (1u128 << 80) + 7] {
            let ascii = match radix {
                2 => format!("{over:b}"),
                8 => format!("{over:o}"),
                10 => format!("{over}"),
                _ => format!("{over:x}"),
            };
            let prefix = match radix {
                2 => BINARY,
                8 => OCTAL,
                10 => "",
                _ => HEX,
            };
            for sign in ["", NEGATIVE] {
                out.push(format!("{sign}{prefix}{}", deva(&ascii)));
            }
        }
    }

    // Leading zeroes: long, well-formed, and NOT too large. A reader that
    // counted digits instead of accumulating would refuse every one of these.
    for radix in [2u32, 8, 10, 16] {
        let t = token_for(1, radix, false);
        let (prefix, digits) = match radix {
            10 => ("", t.as_str()),
            _ => t.split_at(HEX.len()),
        };
        out.push(format!("{prefix}{}{digits}", deva(&"0".repeat(40))));
    }

    out
}

fn load_sanskrit_text(text: &str) -> Interpreter {
    Interpreter::load(
        &[("lex.t1", &source("lex.t1")), ("sanskrit_text.t1", text)],
        &spec_root(),
    )
    .expect("sanskrit_text.t1 loads")
}

/// One reader, over the whole token: `आरम्भः` is ० and `सीमा` is the length, so
/// the range IS the token. That is the no-sub-slice idiom the file's header
/// names — T1 indexes one octet and has no way to cut a slice.
fn read(it: &mut Interpreter, routine: &str, token: &str) -> Result<Value, String> {
    // `len()` on a `&str` is the byte length already — this counts OCTETS,
    // which is what the T1 routines index by, and clippy is right that
    // `.as_bytes()` adds nothing.
    let n = token.len() as i128;
    it.call(
        &format!("अक्षरकोशॱ{routine}"),
        vec![octets(token), Value::Int(0), Value::Int(n)],
        20_000_000,
    )
    .map_err(|e| e.reason)
}

/// Every disagreement between the readers and Rust, as text.
///
/// Returned rather than asserted so that the mutation test can require it to be
/// NON-empty, which is the only way a mutation can be seen to have bitten.
fn numeral_disagreements(text: &str) -> Vec<String> {
    let mut it = load_sanskrit_text(text);
    let mut bad = Vec::new();

    for token in numeral_corpus() {
        let want = rust_reading(&token);
        let shown = token.escape_unicode().to_string();

        let mut check = |routine: &str, got: Result<Value, String>, expect: i128| match got {
            Err(e) => bad.push(format!("{routine}({shown}) failed to run: {e}")),
            Ok(v) => match v.as_int() {
                Some(n) if n == expect => {}
                Some(n) => bad.push(format!("{routine}({shown}) = {n}, Rust reads {expect}")),
                None => bad.push(format!("{routine}({shown}) answered {v:?}, not a number")),
            },
        };

        check("मानदोषः", read(&mut it, "मानदोषः", &token), want.value_code);
        if want.value_code == OK {
            check("मानम्", read(&mut it, "मानम्", &token), want.magnitude);
        }
        check("अंशदोषः", read(&mut it, "अंशदोषः", &token), want.bits_code);
        if want.bits_code == OK {
            check("अंशाः", read(&mut it, "अंशाः", &token), want.bits);
        }

        // `सङ्ख्या` takes a WHOLE slice — the header notes it has no caller in
        // this tree that can cut one — and answers the radix or `शून्यम्`.
        match it.call("अक्षरकोशॱसङ्ख्या", vec![octets(&token)], 20_000_000)
        {
            Err(e) => bad.push(format!("सङ्ख्या({shown}) failed to run: {}", e.reason)),
            Ok(v) => {
                let got = if v.is_nil() { None } else { v.as_int() };
                if got != want.radix {
                    bad.push(format!(
                        "सङ्ख्या({shown}) = {got:?}, Rust reads {:?}",
                        want.radix
                    ));
                }
            }
        }
    }

    bad
}

#[test]
fn the_numeral_corpus_covers_what_it_claims_to_and_this_test_is_not_vacuous() {
    let tokens = numeral_corpus();
    println!(
        "METRIC sadhana_t1_numeral_tokens_exercised {}",
        tokens.len()
    );
    assert!(
        tokens.len() >= 200,
        "only {} tokens; the differential rests on this corpus",
        tokens.len()
    );

    // Each of the four codes must be REACHABLE, or "the reader agrees with
    // Rust" is satisfied by a reader that only ever answers ०.
    let mut seen_value = [false; 4];
    let mut seen_bits = [false; 4];
    let mut radices = std::collections::BTreeSet::new();
    for t in &tokens {
        let r = rust_reading(t);
        seen_value[r.value_code as usize] = true;
        seen_bits[r.bits_code as usize] = true;
        if let Some(x) = r.radix {
            radices.insert(x);
        }
    }
    for (code, name) in [
        (OK, "अङ्कनिर्दोषः"),
        (MALFORMED, "अङ्करूपदोषः"),
        (TOO_LARGE, "अङ्कातिमानदोषः"),
        (SIGNED, "अङ्कचिह्नदोषः"),
    ] {
        assert!(
            seen_value[code as usize],
            "no token in the corpus makes value() answer {name}"
        );
    }
    // `अङ्कचिह्नदोषः` must NEVER come back from the bit reader — that IS the
    // W-075 split — and the corpus has to be able to notice if it did.
    assert!(
        !seen_bits[SIGNED as usize],
        "bits() cannot answer अङ्कचिह्नदोषः; `ऋण` is what it exists to honour"
    );
    for code in [OK, MALFORMED, TOO_LARGE] {
        assert!(seen_bits[code as usize], "bits() never answers {code}");
    }
    assert_eq!(
        radices,
        [2i128, 8, 10, 16].into_iter().collect(),
        "every radix doc 15 §3.3 names must appear in the corpus"
    );
}

#[test]
fn the_numeral_readers_agree_with_rust_on_every_token() {
    let bad = numeral_disagreements(&source("sanskrit_text.t1"));
    assert!(
        bad.is_empty(),
        "{} disagreement(s) between अक्षरकोश and Rust:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
}

/// The negative range reaches one further than the positive one, and that is
/// the case a reader typed `i64` cannot express — `W-075`'s own example, and
/// the reason `bits` replaced `signed_value`.
#[test]
fn two_s_complement_reaches_i64_min_and_stops_one_past_it() {
    let mut it = load_sanskrit_text(&source("sanskrit_text.t1"));

    let min = token_for(1u64 << 63, 10, true); // ऋण९२२३३७२०३६८५४७७५८०८
    assert_eq!(
        read(&mut it, "अंशदोषः", &min).unwrap().as_int(),
        Some(OK),
        "i64::MIN must be readable"
    );
    assert_eq!(
        read(&mut it, "अंशाः", &min).unwrap().as_int(),
        Some(i128::from(i64::MIN as u64)),
        "the last magnitude 64 bits hold came out wrong — negating through i64 \
         is how `ऋण` followed by २^६३ used to come out one short"
    );

    let past = token_for((1u64 << 63) + 1, 10, true);
    assert_eq!(
        read(&mut it, "अंशदोषः", &past).unwrap().as_int(),
        Some(TOO_LARGE),
        "one past i64::MIN must be अङ्कातिमानदोषः and not a wrapped value"
    );

    // And the SAME magnitude without the sign is a perfectly good bit pattern.
    let positive = token_for((1u64 << 63) + 1, 10, false);
    assert_eq!(
        read(&mut it, "अंशदोषः", &positive).unwrap().as_int(),
        Some(OK)
    );
    assert_eq!(
        read(&mut it, "अंशाः", &positive).unwrap().as_int(),
        Some(i128::from((1u64 << 63) + 1))
    );
}

/// The three questions ARE three, shown on tokens where they disagree.
#[test]
fn the_three_readers_answer_differently_and_that_is_the_point() {
    let mut it = load_sanskrit_text(&source("sanskrit_text.t1"));

    // `ऋण१`: a numeral (radix १०), refused by the magnitude reader because a
    // magnitude has nowhere to put a sign, and ALL ONES to the bit reader.
    let neg_one = token_for(1, 10, true);
    assert_eq!(
        it.call("अक्षरकोशॱसङ्ख्या", vec![octets(&neg_one)], 20_000_000)
            .unwrap()
            .as_int(),
        Some(10)
    );
    assert_eq!(
        read(&mut it, "मानदोषः", &neg_one).unwrap().as_int(),
        Some(SIGNED)
    );
    assert_eq!(
        read(&mut it, "अंशदोषः", &neg_one).unwrap().as_int(),
        Some(OK)
    );
    assert_eq!(
        read(&mut it, "अंशाः", &neg_one).unwrap().as_int(),
        Some(i128::from(u64::MAX))
    );

    // Too large: well-formed WRITING, so `सङ्ख्या` still answers a radix while
    // both value readers refuse it. Reporting this as "malformed" is what sent
    // a reader looking for a typo that is not there.
    let huge = deva(&format!("{}", 1u128 << 64));
    assert_eq!(
        it.call("अक्षरकोशॱसङ्ख्या", vec![octets(&huge)], 20_000_000)
            .unwrap()
            .as_int(),
        Some(10)
    );
    assert_eq!(
        read(&mut it, "मानदोषः", &huge).unwrap().as_int(),
        Some(TOO_LARGE)
    );
    assert_eq!(
        read(&mut it, "अंशदोषः", &huge).unwrap().as_int(),
        Some(TOO_LARGE)
    );

    // `०षोड्इआऊ२९इउ४८४२२२३२५` — the module header's own example of a datum
    // that is a BIT PATTERN and not an `i64` at all.
    let pattern = token_for(0x0EAF29E484222325, 16, false);
    assert_eq!(
        read(&mut it, "अंशदोषः", &pattern).unwrap().as_int(),
        Some(OK)
    );
    assert_eq!(
        read(&mut it, "अंशाः", &pattern).unwrap().as_int(),
        Some(0x0EAF29E484222325)
    );
}

/// `अक्षराणि` is still a stub, BY NAME and with the blocker named.
///
/// A count alone is satisfied by any stub, so this asserts WHICH — exactly as
/// `T0_READER_BLOCKED` and `CODEGEN_EMITTERS_BLOCKED` do in `t1_sources.rs`.
/// And it asserts it by RUNNING it, which no text search can be fooled into
/// passing: a body that returned `अपूर्णम्` some other way would still be a
/// stub, and one that returned anything else would fail here.
///
/// **`व्यञ्जन` WAS THE OTHER AND IS WRITTEN, so it is gone from this list.**
/// Its blocker was exactly one missing row in `anita.rs`'s `TABLES`;
/// `("शिवसूत्रकोशः", "shiva-sutras.tsv")` is now `anita.rs:197` and the routine
/// derives हल् by walking the sūtras through `समावेशः`. It is exercised in
/// `tests/t1_exec_aksara.rs` against `spec/shiva-sutras.tsv` read independently
/// in Rust — every consonant true, every vowel false, and the four letters
/// inside U+0915..U+0939 that हल् does not list false.
///
/// It could not stay here in any case: `व्यञ्जन` now reaches `सङ्केतन`'s five
/// TSV helpers, so it needs `encode.t1` loaded beside it and
/// `load_sanskrit_text` deliberately loads `sanskrit_text.t1` alone.
/// The `sanskrit_text.t1` routines that are still stubs, and what each waits on.
///
/// A SLICE and not an inline array, so that the loop below stays a loop as the
/// list shrinks — it held two until `D-002i2` wrote `व्यञ्जन`, and one element
/// in an inline array is a `clippy::single_element_loop`.
/// EMPTY SINCE 2026-08-31 — `अक्षरकोश` has no stub left.
///
/// `अक्षराणि` was the last, and BOTH halves of its blocker are answered:
///
/// - the UAX #29 tables now exist in `spec/` and are named for the embed —
///   `अक्षरभेदकोशः`, `संयोगभेदकोशः`, `चित्राक्षरकोशः`, generated by
///   `tools/gen-grapheme-break.py` and `tools/gen-incb.py` from the PINNED UCD
///   and asserted against `tables.rs` code point by code point. THREE tables,
///   not one: GB9c is undecidable without InCB and GB11 without
///   Extended_Pictographic, which is why the first table cleared only a third;
/// - the slice-of-slices return was a SIGNATURE ARTEFACT, and the enumeration
///   ADR-0031 requires was done. It answers a count over an arena instead,
///   reusing `निदान`'s own `अक्षरविभागः`/`अक्षरसंख्या`/`अक्षरतुल्यम्` precedent.
///
/// It segments **766 of 766 GraphemeBreakTest.txt cases with zero failures**,
/// and no range is written into the `.t1` at all — all three tables are read
/// through `समावेशः` at run time.
///
/// Kept as an empty slice rather than deleted, so the next blocked routine has
/// somewhere to be recorded with its reason.
const SANSKRIT_TEXT_STILL_BLOCKED: &[(&str, &str)] = &[];

#[test]
fn the_two_sanskrit_text_routines_that_could_not_be_written_are_still_stubs() {
    let mut it = load_sanskrit_text(&source("sanskrit_text.t1"));
    for (name, blocker) in SANSKRIT_TEXT_STILL_BLOCKED {
        let answer = it
            .call(
                &format!("अक्षरकोशॱ{name}"),
                vec![octets("\u{0915}")],
                100_000,
            )
            .unwrap_or_else(|e| panic!("{name} did not run: {}", e.reason));
        let stub = answer
            .octets()
            .is_some_and(|o| o.as_slice() == "अपूर्णम्".as_bytes());
        assert!(
            stub,
            "{name} answered {answer:?} and is no longer a stub; name the row \
             that cleared its blocker — {blocker}"
        );
    }
}

// ── MUTATION: the differential graded against itself ─────────────────────

/// `(routine, what the mutation does, from, to)`.
///
/// The differential MUST fail under every one. A survivor is a mutation the
/// corpus cannot see, which means the corpus does not check that behaviour.
const NUMERAL_MUTANTS: &[(&str, &str, &str, &str)] = &[
    (
        "मानदोषः",
        "a ऋण stops being अङ्कचिह्नदोषः and reads as well-formed",
        "प्रत्यागमनम् अङ्कचिह्नदोषः ।",
        "प्रत्यागमनम् अङ्कनिर्दोषः ।",
    ),
    (
        "अंशाः",
        "two's complement comes out one short — the exact defect W-075 records",
        "प्रत्यागमनम् आरभ्य आरभ्य चरमम् वियोगः पदम् समाप्तम् योगः १ समाप्तम् ।",
        "प्रत्यागमनम् आरभ्य आरभ्य चरमम् वियोगः पदम् समाप्तम् योगः ० समाप्तम् ।",
    ),
    (
        "अङ्कमूल्यम्",
        "every Devanagari digit is worth one more than it is",
        "मूल्यम् भवति आरभ्य तृतीयम् वियोगः १६६ समाप्तम् ।",
        "मूल्यम् भवति आरभ्य तृतीयम् वियोगः १६५ समाप्तम् ।",
    ),
    (
        "सङ्ख्याङ्कारम्भः",
        "the radix prefix is measured at four akṣaras instead of five",
        "प्रत्यागमनम् आरभ्य स्थानम् योगः १५ समाप्तम् ।",
        "प्रत्यागमनम् आरभ्य स्थानम् योगः १२ समाप्तम् ।",
    ),
    (
        "मानदोषः",
        "the overflow guard is dropped, so a literal too large saturates \
         silently — the W-075 defect itself",
        "                प्रत्यागमनम् अङ्कातिमानदोषः ।\n            इति\n        इति\n        पदम् भवति आरभ्य आरभ्य पदम् गुणनम् मूलम् समाप्तम् योगः अङ्कमानम् समाप्तम् ।",
        "                प्रत्यागमनम् अङ्कनिर्दोषः ।\n            इति\n        इति\n        पदम् भवति आरभ्य आरभ्य पदम् गुणनम् मूलम् समाप्तम् योगः अङ्कमानम् समाप्तम् ।",
    ),
];

#[test]
fn every_mutation_of_the_numeral_readers_is_killed_by_the_differential() {
    let src = source("sanskrit_text.t1");
    let mut by_disagreement = 0usize;
    for (routine, what, from, to) in NUMERAL_MUTANTS {
        let mutated = mutate(&src, from, to);
        // A mutant may also TRAP — the interpreter refusing a body, an index
        // past the end — and that is a kill too. The two are counted apart on
        // purpose: a trap is also what a mutant that broke the LOAD would do,
        // and a suite where every mutant trapped would be saying something
        // about the parser rather than about the readers.
        let killed = match std::panic::catch_unwind(|| numeral_disagreements(&mutated)) {
            Err(_) => "trapped",
            Ok(bad) if !bad.is_empty() => {
                by_disagreement += 1;
                "disagreed"
            }
            Ok(_) => "SURVIVED",
        };
        println!("MUTANT {routine}: {killed} — {what}");
        assert_ne!(
            killed, "SURVIVED",
            "MUTATION SURVIVED in {routine} — {what}. The differential passed \
             with the source altered, so it does not actually check this."
        );
    }
    println!(
        "METRIC sadhana_t1_numeral_mutants_killed {}",
        NUMERAL_MUTANTS.len()
    );
    println!("METRIC sadhana_t1_numeral_mutants_killed_by_disagreement {by_disagreement}");
    assert!(
        by_disagreement >= 1,
        "every mutant trapped and none was killed by an actual disagreement, so \
         nothing here shows the differential can see a WRONG ANSWER"
    );
}

/// The falsification control for the mutation test above.
///
/// If `numeral_disagreements` reported "none" for a source it never really ran,
/// every mutation above would be killed by the same accident and the suite
/// would be green over nothing. **This was not hypothetical**: an earlier
/// arrangement of this section had all five mutants "killed" by a parse error
/// that the unmutated source hit as well. So the UNMUTATED source goes through
/// the identical path, `catch_unwind` and all.
#[test]
fn the_unmutated_numeral_source_survives_the_path_the_mutants_take() {
    let src = source("sanskrit_text.t1");
    match std::panic::catch_unwind(|| numeral_disagreements(&src)) {
        Err(_) => panic!("the unmutated source trapped on the mutation path"),
        Ok(bad) => assert!(bad.is_empty(), "{}", bad.join("\n  ")),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The sema — `अर्थ`, the port of `t1/{resolve,typecheck,types}.rs`.
//
// `artha.t1` carried ELEVEN stubs behind one note that said T1 has no `push`.
// It has: `वाक्यविभाग` writes six appenders and the tests above run them. Six
// of the eleven are now written and the tests below EXECUTE them; the other
// five have blockers that are not `push` and are named in the file.
//
// **THE SCOPE STACK'S READ SIDE HAD NEVER BEEN RUN AND WAS OFF BY ONE.**
// `परिसरान्वेषणम्`, `नामनिर्णयः` and `पुनरुक्तघोषणम्` were described in the
// file as its working read side. All three walked from index ० — the slot
// every arena in this corpus reserves and never fills — so each asked a
// `शून्यम्` for a member on its FIRST iteration and raised instead of
// answering. Nothing caught it because until `घोषणम्` existed there was no
// way to build a scope to walk, and every assertion this crate had was about
// TEXT. That is the whole argument for this file, measured once more.
// ─────────────────────────────────────────────────────────────────────────

/// `अर्थ` needs `वास्तु`, which is `ast.t1`.
///
/// **`sanchaya.t1` JOINED ON 2026-09-14, and the reason is a parse one rather
/// than a run one.** `artha.t1:1302` calls `घोषणासञ्चयॱप्राचलप्रविष्टिनाम आरभ्य
/// दूरप्रविष्टिः ऽ दूरक्रमः समाप्तम्` — the BRACKETED two-argument form. This
/// interpreter tells that form from a juxtaposed call by the callee's ARITY
/// (`nirvahana.rs:2865`), so with `घोषणासञ्चय` absent the arity is unknown, the
/// call reads as one argument, and the parse stops at the `ऽ`: "expected
/// `समाप्तम्`, found `ऽ`". The routine then counts as unrunnable and the
/// equality below fails at 65 of 66 — which reads like a regression in `अर्थ`
/// and is really a module missing from THIS list.
///
/// The margin on `load_sema_with_parser` warns that widening a shared helper
/// has blast radius, and that warning was taken seriously: this widening was
/// measured, not assumed. It moves three tests from red to green and none the
/// other way, and the routine floor below is unchanged.
fn load_sema() -> Interpreter {
    load_all(&["ast.t1", "artha.t1", "sanchaya.t1"])
}

/// `load_sema` PLUS the modules an executed resolver actually needs.
///
/// SEPARATE FROM `load_sema` ON PURPOSE. Widening that one took it from 24
/// routines to 51 and broke `the_sema_loads_and_every_routine_in_it_is_runnable`,
/// which counts what it loads — a shared helper has blast radius, and this test
/// does not get to change what every other sema test measures.
///
/// `पदविभाग` because `artha.t1` imports it since blocker (c) was discharged,
/// and a qualified name whose module is absent does not resolve as a call.
/// `व्याकर` because `ast.t1` declares the arenas but NO appender: the only way
/// to build an अभिव्यञ्जक is `व्याकरॱअभिव्यञ्जकयोजनम्` (parse.t1:99).
fn load_sema_with_parser() -> Interpreter {
    // `sanskrit_text.t1` is here because `व्याकर` now asks `अक्षरकोशॱसङ्ख्या`
    // whether a word is a numeral. THIS LOADER IS NOT SHARED — `load_sema`
    // stays at its own module list, whose count a census test is keyed to.
    load_all(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "artha.t1",
        "sanchaya.t1",
        "sanskrit_text.t1",
    ])
}

/// One slice-typed global as text, or an empty string.
fn text_global(it: &Interpreter, name: &str) -> String {
    match it.global(name) {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        _ => String::new(),
    }
}

/// `load_sema_with_parser` PLUS the declaration store FILLED from every source
/// named, and `अर्थॱसञ्चयसिद्धिः` called so the resolver may consult it.
///
/// W-223 part 2. THE STORE IS WHY THE CENSUSES STOPPED BUILDING ONE INTERPRETER
/// PER FILE. `घोषणासञ्चय` holds what every module declares, and it can only be
/// filled by lexing and parsing every source into ONE interpreter — `सङ्ग्रहः`
/// copies each program out of `व्याकर`'s arena after its parse. A resolver that
/// consults it must therefore live in that same interpreter.
///
/// TWO LEAKS HAD TO CLOSE BEFORE THIS WAS SAFE, and both were invisible while
/// every census built a fresh interpreter per source: `कार्यक्रमनिर्णयः` walked
/// `घोषणाकोश`'s LENGTH rather than the program's extent, so a shorter program
/// read the previous one's declarations; and the three symbol-keyed arenas were
/// never cleared while `निर्णायकारम्भः` restarts the numbering that indexes them,
/// so a second program's symbol १ read the first's type. Per-source attribution
/// survives because `कार्यक्रमपठनम्` resets `घोषणासूचकाङ्क` on entry — every
/// program still begins at declaration १ — and the extent bounds the walk.
fn load_sema_collected(dir: &std::path::Path, names: &[String]) -> Interpreter {
    let mut it = load_sema_with_parser();
    for n in names {
        let Ok(src) = std::fs::read_to_string(dir.join(n)) else {
            continue;
        };
        let Ok(toks) = it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        else {
            continue;
        };
        let toks = toks.as_int().unwrap_or(0);
        if it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .is_err()
        {
            continue;
        }
        // A source that fails to collect is not fatal: the store simply does not
        // hold it, and `सदस्यनिर्णयः` counts every use of it as trusted-because-
        // uncollected. The census asserts that count is ० precisely so a silent
        // collection failure here cannot pass as a clean measurement.
        let _ = it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 2_000_000_000);
    }
    it.call("अर्थॱसञ्चयसिद्धिः", vec![], 5_000_000)
        .expect("सञ्चयसिद्धिः runs");
    it
}

/// Push one token and answer its index — `पदविभागॱचिह्नकयोजनम्`.
///
/// `पाठसीमा` IS THE TEXT'S LENGTH, NOT ०. This helper used to pass ० for both
/// `अष्टक` and `पाठसीमा`, which builds a token the LEXER NEVER PRODUCES: the
/// real one carries the WHOLE SOURCE in `पाठ` and delimits its own text with
/// `अष्टक`..`पाठसीमा`. On the ० ० shape a consumer that read `पाठ` directly
/// looked correct, so the suite agreed with a parser that could not match a
/// single keyword against real lexer output. Passing the length here makes the
/// helper produce the lexer's own shape for a token starting at offset ०.
fn tok(it: &mut Interpreter, text: &str) -> i128 {
    let end = i128::try_from(text.len()).expect("a token length fits");
    it.call(
        "पदविभागॱचिह्नकयोजनम्",
        vec![
            Value::Int(1),
            octets(text),
            Value::Int(0),
            Value::Int(end),
            Value::Int(0),
            Value::Int(1),
        ],
        20_000_000,
    )
    .unwrap_or_else(|e| panic!("चिह्नकयोजनम् runs: {e:?}"))
    .as_int()
    .expect("an index")
}

/// Push one expression node and answer its index — `व्याकरॱअभिव्यञ्जकयोजनम्`.
fn push_expr(it: &mut Interpreter, kind: i128, value: i128, left: i128, right: i128) -> i128 {
    it.call(
        "व्याकरॱअभिव्यञ्जकयोजनम्",
        vec![
            Value::Int(kind),
            Value::Int(value),
            Value::Int(left),
            Value::Int(right),
            Value::Int(0),
        ],
        20_000_000,
    )
    .unwrap_or_else(|e| panic!("अभिव्यञ्जकयोजनम् runs: {e:?}"))
    .as_int()
    .expect("an index")
}

/// `अभिव्यञ्जकनिर्णयः`, as `Option<bool>`; a run failure PANICS rather than
/// becoming `None`, because "the routine says no" and "the routine did not
/// run" are different facts.
fn resolve_expr(it: &mut Interpreter, r: &Value, idx: i128) -> Option<bool> {
    let v = it
        .call(
            "अर्थॱअभिव्यञ्जकनिर्णयः",
            vec![r.clone(), Value::Int(idx)],
            50_000_000,
        )
        .unwrap_or_else(|e| panic!("अभिव्यञ्जकनिर्णयः runs: {e:?}"));
    match v {
        Value::Bool(b) => Some(b),
        Value::Nil => None,
        other => panic!("अभिव्यञ्जकनिर्णयः must answer a बूल, answered {other:?}"),
    }
}

fn load_sema_mutated(artha: &str) -> Interpreter {
    Interpreter::load(
        &[("ast.t1", &source("ast.t1")), ("artha.t1", artha)],
        &spec_root(),
    )
    .expect("the mutated sema loads")
}

/// The `भेद` of an `अर्थप्रकार` a routine returned — the enum discriminant.
fn kind_of(v: &Value) -> Option<i128> {
    match v {
        Value::Record(r) => r.borrow().get("भेद").and_then(Value::as_int),
        _ => None,
    }
}

/// `(भेद, विस्तार, चिह्नितम्)` — enough of an `अर्थप्रकार` to tell
/// `Ty::Int { width: 32, signed: true }` from `Ty::Void` and from `Ty::Error`.
fn ty_triple(v: &Value) -> (Option<i128>, Option<i128>, Option<bool>) {
    match v {
        Value::Record(r) => {
            let r = r.borrow();
            (
                r.get("भेद").and_then(Value::as_int),
                r.get("विस्तार").and_then(Value::as_int),
                match r.get("चिह्नितम्") {
                    Some(Value::Bool(b)) => Some(*b),
                    _ => None,
                },
            )
        }
        _ => (None, None, None),
    }
}

/// One answer, flattened so that every shape this walk can produce is
/// DISTINGUISHABLE.
///
/// `Value::as_int` is `None` for a `बूल` as well as for `शून्यम्`, and
/// `पुनरुक्तघोषणम्` answers a `बूल` — so without the two negative codes below
/// a mutation that turned `असत्यम्` into `शून्यम्` would compare equal to the
/// truth and survive. Which is a mistake this file has already made once, one
/// layer up, in the ratchets it exists to replace.
fn code(v: &Value) -> Option<i128> {
    match v {
        Value::Int(n) => Some(*n),
        Value::Bool(true) => Some(-1),
        Value::Bool(false) => Some(-2),
        _ => None,
    }
}

/// Walk one resolver through a scripted sequence and record every answer.
///
/// **This is the whole acceptance of the row, as a value.** It enters a scope,
/// declares into it, looks the name up from inside, leaves the scope and looks
/// it up again — so a resolver whose `परिसरनिर्गमः` did nothing, or whose
/// `घोषणम्` bound into the wrong scope, produces a DIFFERENT vector here. Every
/// mutation below is killed by comparing against it.
///
/// The outer `Option` is REFUSAL and the inner one is `शून्यम्`. A mutation may
/// break a routine into raising rather than into answering wrongly — an index
/// outside a run, say — and both must be killed the same way.
fn walk_the_scope_stack(it: &mut Interpreter) -> Vec<(&'static str, Option<Option<i128>>)> {
    let mut out: Vec<(&'static str, Option<Option<i128>>)> = Vec::new();
    let Ok(r) = it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
    else {
        out.push(("निर्णायकारम्भः refused", None));
        return out;
    };
    let steps: &[(&'static str, &str, &str)] = &[
        // The global scope निर्णायकारम्भः pushed.
        ("declare क (global)", "अर्थॱघोषणम्", "क"),
        ("declare ख (global)", "अर्थॱघोषणम्", "ख"),
        ("declare क again", "अर्थॱघोषणम्", "क"),
        ("resolve क", "अर्थॱनामनिर्णयः", "क"),
        ("resolve ग (absent)", "अर्थॱनामनिर्णयः", "ग"),
        // A nested scope.
        ("enter", "अर्थॱपरिसरप्रवेशः", ""),
        ("declare ग (inner)", "अर्थॱघोषणम्", "ग"),
        ("declare क (shadow)", "अर्थॱघोषणम्", "क"),
        ("resolve ग inside", "अर्थॱनामनिर्णयः", "ग"),
        ("resolve क inside", "अर्थॱनामनिर्णयः", "क"),
        // And out again.
        ("exit", "अर्थॱपरिसरनिर्गमः", ""),
        ("resolve ग outside", "अर्थॱनामनिर्णयः", "ग"),
        ("resolve क outside", "अर्थॱनामनिर्णयः", "क"),
        // **AND THEN OUT OF THE GLOBAL SCOPE TOO**, which is `Vec::pop` on a
        // one-element Vec: it leaves the stack EMPTY. Rust's `declare` then
        // panics on `self.scopes.last().unwrap()`; every routine here answers
        // instead, and the three steps below are the only ones in this walk
        // that reach the empty-stack guards at all. Without them
        // `पुनरुक्तघोषणम्`'s guard is unreachable and a mutation of it
        // survives — measured, not assumed: it did.
        ("exit the global scope", "अर्थॱपरिसरनिर्गमः", ""),
        ("duplicate? on empty", "अर्थॱपुनरुक्तघोषणम्", "क"),
        ("declare on empty", "अर्थॱघोषणम्", "क"),
        ("resolve on empty", "अर्थॱनामनिर्णयः", "क"),
    ];
    for (tag, routine, name) in steps {
        let mut args = vec![r.clone()];
        if !name.is_empty() {
            args.push(octets(name));
        }
        // W-183: `घोषणम्` takes the declared type as a third argument (an
        // अर्थप्रकारकोश index, ० = none); this walk declares untyped names.
        if *routine == "अर्थॱघोषणम्" {
            args.push(Value::Int(0));
        }
        let v = it.call(routine, args, 5_000_000).ok().map(|v| code(&v));
        out.push((tag, v));
    }
    out
}

#[test]
fn the_sema_loads_and_every_routine_in_it_is_runnable() {
    // The floor, as `a_t1_body_is_executed_…` is for the other two modules:
    // every assertion below is meaningless if the loader found no routines.
    let it = load_sema();
    let r = it.report();
    println!("METRIC sadhana_t1_sema_routines_loaded {}", r.routines);
    println!("METRIC sadhana_t1_sema_routines_runnable {}", r.runnable);
    assert!(
        r.routines >= 24,
        "only {} routines loaded from ast.t1, artha.t1 and sanchaya.t1",
        r.routines
    );
    // Ratchet, and an EQUALITY rather than a floor: `अर्थ` has no arity fault
    // and calls nothing outside the modules this loader carries, so every
    // routine it declares must be runnable. A routine that stops being runnable is a
    // regression this crate has no other way to see.
    assert_eq!(
        r.runnable,
        r.routines,
        "{} of {} sema routines have a body this interpreter cannot run",
        r.routines - r.runnable,
        r.routines
    );
}

#[test]
fn a_scope_that_is_entered_declared_into_and_left_behaves_like_a_stack() {
    // **THE ACCEPTANCE OF THIS ROW.** Not "push exists" — that was proved for
    // `वाक्यविभाग` above — but that `निर्णायकारम्भः`, `परिसरप्रवेशः`,
    // `घोषणम्` and `परिसरनिर्गमः` compose into the thing `resolve.rs` has: a
    // stack where a name is visible inside its scope and GONE outside it.
    let mut it = load_sema();
    let answers = walk_the_scope_stack(&mut it);
    for (tag, v) in &answers {
        println!("  {tag} -> {v:?}");
    }

    let by = |tag: &str| -> Option<Option<i128>> {
        answers
            .iter()
            .find(|(t, _)| *t == tag)
            .map(|(_, v)| *v)
            .expect("every step was recorded")
    };

    // Symbols are minted from १ and each call advances the counter exactly
    // once — `नवसंज्ञा` reads it and `घोषणम्` is the only site that
    // increments. Two sites minting numbers is how two names come to share one.
    //
    // ० → १ ON 2026-09-13 (c107c725), and the reason is native, not stylistic:
    // a `सम्भाव्य न६४` is ONE word natively, so `Some(०)` and the nil word are
    // the same bits and the first symbol minted was indistinguishable from "no
    // symbol". The minter starts at १ so that zero can mean absent. Every
    // expectation in this block moved by exactly one; the RELATIONS — first,
    // second, and the refused duplicate consuming none — are unchanged, which
    // is what says this was a renumbering and not a behaviour change.
    assert_eq!(by("declare क (global)"), Some(Some(1)), "first symbol is १");
    assert_eq!(
        by("declare ख (global)"),
        Some(Some(2)),
        "second symbol is २"
    );

    // The duplicate is REFUSED — `शून्यम्`, which is what a `दोषयुक्त` carries
    // here — and it does NOT consume a symbol number: the shadow below is ३.
    assert_eq!(
        by("declare क again"),
        Some(None),
        "a second binding of `क` in the SAME scope must be refused"
    );

    assert_eq!(by("resolve क"), Some(Some(1)), "`क` resolves to its symbol");
    assert_eq!(
        by("resolve ग (absent)"),
        Some(None),
        "a name no scope carries must answer शून्यम् — and this is the case \
         that RAISED before the read side was corrected, because the walk ran \
         past the last live scope into the reserved slot ०"
    );

    // Inside a nested scope: a new name binds, and an OUTER name may be
    // shadowed. Shadowing is legal and is what makes a local name local;
    // `पुनरुक्तघोषणम्` tests the innermost scope and only that one.
    assert_eq!(by("declare ग (inner)"), Some(Some(3)));
    assert_eq!(
        by("declare क (shadow)"),
        Some(Some(4)),
        "`क` is bound in an OUTER scope, so binding it in an inner one is a \
         shadow and must be allowed; refusing it would be a different language"
    );
    assert_eq!(by("resolve ग inside"), Some(Some(3)));
    assert_eq!(
        by("resolve क inside"),
        Some(Some(4)),
        "INNERMOST FIRST: `क` must resolve to the shadow (४) and not to the \
         global (०). An outermost-first walk answers ० here, and a function \
         would silently read the wrong variable while resolving cleanly"
    );

    // **And out again — the half that only a real pop can pass.**
    assert_eq!(
        by("resolve ग outside"),
        Some(None),
        "`ग` was declared in the scope that has been left; it must be GONE. \
         If `परिसरनिर्गमः` did nothing this answers २, which is the whole \
         difference between a stack and a pile"
    );
    assert_eq!(
        by("resolve क outside"),
        Some(Some(1)),
        "the shadow left with its scope, so `क` is the global १ again — not \
         ४, and not शून्यम्"
    );

    // The empty-stack tail. Rust would have panicked here; every routine
    // answers. `-2` is `असत्यम्` under `code` above.
    assert_eq!(
        by("duplicate? on empty"),
        Some(Some(-2)),
        "with no scope at all there is no innermost scope to hold a duplicate, \
         so पुनरुक्तघोषणम् must answer असत्यम् rather than read the reserved slot"
    );
    assert_eq!(
        by("declare on empty"),
        Some(None),
        "with nowhere to bind, घोषणम् refuses — the graceful form of Rust's \
         `self.scopes.last().unwrap()`"
    );
    assert_eq!(
        by("resolve on empty"),
        Some(None),
        "an empty stack carries no names"
    );
}

#[test]
fn a_resolver_starts_with_exactly_one_scope_and_it_is_empty() {
    // `Resolver::new` is `scopes: vec![HashMap::new()]`. The global scope is
    // not optional: `घोषणम्` binds into the innermost scope and a resolver
    // with none refuses the first top-level name it is given. This asserts
    // the shape of what `निर्णायकारम्भः` returned rather than trusting it.
    let mut it = load_sema();
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    let Value::Record(rec) = &r else {
        panic!("निर्णायकारम्भः returned {r:?}, which is not a निर्णायक")
    };
    let (next, scopes) = {
        let rec = rec.borrow();
        (
            rec.get("अग्रिमसंज्ञा").and_then(Value::as_int),
            rec.get("परिसराः").cloned(),
        )
    };
    assert_eq!(next, Some(1), "next_symbol starts at १ — zero means absent");
    let Some(Value::Arena(scopes)) = scopes else {
        panic!("परिसराः is not an arena")
    };
    // TWO, not one: slot ० is the reserved entry every arena in this corpus
    // keeps and never fills, so ONE live scope is a length of २. This is the
    // exact convention the read side got wrong.
    let inner = {
        let s = scopes.borrow();
        assert_eq!(
            s.len(),
            2,
            "one live scope over a reserved slot ० is a दैर्घ्य of २"
        );
        s[1].clone()
    };
    // And that one scope is empty — likewise a length of १.
    let Value::Record(inner) = inner else {
        panic!("the global scope is not a परिसर")
    };
    let entries = inner.borrow().get("प्रविष्टयः").cloned();
    let Some(Value::Arena(entries)) = entries else {
        panic!("प्रविष्टयः is not an arena")
    };
    assert_eq!(
        entries.borrow().len(),
        1,
        "the global scope must start with no bindings"
    );
}

#[test]
fn a_resolver_with_no_scope_left_refuses_instead_of_panicking() {
    // **THIS TEST ASSERTED THE OPPOSITE FIRST AND THE CODE WAS RIGHT.** It was
    // written to say that popping the last scope is a no-op, on the reasoning
    // that a resolver must always have somewhere to bind. That is not what
    // Rust does: `exit_scope` is a bare `self.scopes.pop()`, and `Vec::pop` on
    // a one-element Vec LEAVES IT EMPTY. `Resolver::new` pushes the global
    // scope and `resolve_program` never unbalances, so Rust simply never
    // reaches the state — and when it does, `declare`'s
    // `self.scopes.last().unwrap()` panics.
    //
    // So the faithful port pops unconditionally, and the question is only what
    // the routines do in a state Rust would panic in. They ANSWER: `घोषणम्`
    // refuses with `शून्यम्` and `नामनिर्णयः` finds nothing. A panic is not
    // available in this language and a wrong number would be worse than both.
    let mut it = load_sema();
    let r = it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000).expect("new");

    // One pop empties it; the two after it are `Vec::pop` on an empty Vec,
    // which is `None` — a no-op, and it must not underflow into the reserved
    // slot ०.
    for _ in 0..3 {
        it.call("अर्थॱपरिसरनिर्गमः", vec![r.clone()], 5_000_000)
            .expect("परिसरनिर्गमः runs even with nothing left to pop");
    }

    let sym = it
        .call(
            "अर्थॱघोषणम्",
            vec![r.clone(), octets("क"), Value::Int(0)],
            5_000_000,
        )
        .expect("घोषणम् runs rather than raising");
    assert!(
        sym.is_nil(),
        "with no scope left there is nowhere to bind, so घोषणम् must refuse; \
         got {sym:?}"
    );
    let found = it
        .call("अर्थॱनामनिर्णयः", vec![r.clone(), octets("क")], 5_000_000)
        .expect("नामनिर्णयः runs rather than raising");
    assert!(
        found.is_nil(),
        "an empty stack carries no names; got {found:?}"
    );

    // And it recovers: pushing a scope makes it bindable again, which is what
    // says the pops shortened the arena rather than corrupting it.
    it.call("अर्थॱपरिसरप्रवेशः", vec![r.clone()], 5_000_000)
        .expect("परिसरप्रवेशः runs");
    let sym = it
        .call(
            "अर्थॱघोषणम्",
            vec![r.clone(), octets("क"), Value::Int(0)],
            5_000_000,
        )
        .expect("घोषणम् runs")
        .as_int();
    assert_eq!(
        sym,
        Some(1),
        "a fresh scope over an emptied stack must bind, and at symbol १ \
         because no घोषणम् before it ever succeeded"
    );
}

/// One mutation of `artha.t1`, all of them in the scope stack.
///
/// The first three are the ONE-BASED corrections this row made, re-broken:
/// each restores the zero-based walk the file shipped with, and each must be
/// caught. That they are caught is the measure of what execution bought —
/// every one of them was in the tree, described as ported, for as long as the
/// file existed.
const SEMA_MUTATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "परिसरान्वेषणम्",
        "the scope walk starts at the reserved slot ० again",
        "चरः सूचकाङ्क ॱॱ अ६४ भवति १ ।\n    चरः दैर्घ्य ॱॱ अ६४ भवति परिसरः ॱ प्रविष्टयः ॱ दैर्घ्य ।",
        "चरः सूचकाङ्क ॱॱ अ६४ भवति ० ।\n    चरः दैर्घ्य ॱॱ अ६४ भवति परिसरः ॱ प्रविष्टयः ॱ दैर्घ्य ।",
    ),
    (
        "नामनिर्णयः",
        "the stack walk runs one scope too far, into the reserved slot",
        "यावत् शेषम् अधिकम् १ आदि",
        "यावत् शेषम् अधिकम् ० आदि",
    ),
    (
        "पुनरुक्तघोषणम्",
        "the empty-stack guard tests ० again, so it never fires",
        "यदि दैर्घ्य न्यूनम् २ आदि\n        प्रत्यागमनम् असत्यम् ।",
        "यदि दैर्घ्य समम् ० आदि\n        प्रत्यागमनम् असत्यम् ।",
    ),
    (
        "नामनिर्णयः",
        "the stack is walked OUTERMOST first, so a shadow resolves to the \
         global it shadows",
        "चरः परिसरः ॱॱ परिसर भवति निर्णायकः ॱ परिसराः अङ्कः शेषम् वियोगः १ अन्तः ।",
        "चरः परिसरः ॱॱ परिसर भवति निर्णायकः ॱ परिसराः अङ्कः आरभ्य दैर्घ्य वियोगः शेषम् समाप्तम् अन्तः ।",
    ),
    (
        "घोषणम्",
        "the duplicate check is reversed, so a SECOND binding is accepted and \
         a first is refused",
        "यदि पुनरुक्तघोषणम् निर्णायकः नाम आदि",
        "यदि पुनरुक्तघोषणम् निर्णायकः नाम असमम् सत्यम् आदि",
    ),
    (
        "घोषणम्",
        "the symbol counter is not advanced, so every name gets symbol ०",
        "निर्णायकः ॱ अग्रिमसंज्ञा भवति आरभ्य संज्ञा योगः १ समाप्तम् ।",
        "निर्णायकः ॱ अग्रिमसंज्ञा भवति संज्ञा ।",
    ),
    (
        "घोषणम्",
        "the binding lands in the OUTERMOST scope instead of the innermost",
        "चरः अन्तिमः ॱॱ परिसर भवति निर्णायकः ॱ परिसराः अङ्कः आरभ्य दैर्घ्य वियोगः १ समाप्तम् अन्तः ।",
        "चरः अन्तिमः ॱॱ परिसर भवति निर्णायकः ॱ परिसराः अङ्कः १ अन्तः ।",
    ),
    (
        "परिसरप्रवेशः",
        "the new scope is written over the innermost one instead of past it",
        "चरः स्थानम् ॱॱ अ६४ भवति निर्णायकः ॱ परिसराः ॱ दैर्घ्य ।",
        "चरः स्थानम् ॱॱ अ६४ भवति आरभ्य निर्णायकः ॱ परिसराः ॱ दैर्घ्य वियोगः १ समाप्तम् ।",
    ),
    (
        "परिसरनिर्गमः",
        "the rebuild copies the scope it was asked to drop, so pop is a no-op",
        "यावत् सूचकाङ्क न्यूनम् आरभ्य दैर्घ्य वियोगः १ समाप्तम् आदि",
        "यावत् सूचकाङ्क न्यूनम् दैर्घ्य आदि",
    ),
];

#[test]
fn breaking_one_comparison_in_the_scope_stack_changes_what_a_name_resolves_to() {
    // **The proof that the test above is not text-shaped.** `mutate` fails if
    // its pattern is not in the source exactly once, so a mutation that
    // matched nothing cannot pass here by doing nothing.
    let truth = walk_the_scope_stack(&mut load_sema());
    assert!(
        truth.iter().any(|(_, v)| matches!(v, Some(Some(_)))),
        "the unmutated sema answered nothing; there is no baseline to differ from"
    );
    assert!(
        truth.iter().all(|(_, v)| v.is_some()),
        "the unmutated sema REFUSED a step: {truth:?}. Every step must answer, \
         or a mutation could survive by refusing the same one"
    );

    let mut proved = 0usize;
    for (routine, what, from, to) in SEMA_MUTATIONS {
        let broken = mutate(&source("artha.t1"), from, to);
        let answers = walk_the_scope_stack(&mut load_sema_mutated(&broken));
        assert_ne!(
            answers, truth,
            "MUTATION SURVIVED: `{routine}` — {what}. The scope stack answered \
             exactly the same symbols with that broken, so \
             `a_scope_that_is_entered_declared_into_and_left_behaves_like_a_stack` \
             is measuring text and not behaviour."
        );
        proved += 1;
    }
    println!("METRIC sadhana_t1_sema_mutations_killed {proved}");
    assert_eq!(
        proved,
        SEMA_MUTATIONS.len(),
        "every mutation must be killed"
    );
}

/// Build a `वास्तुॱवाक्य` arena by hand — one-based, slot ० reserved.
///
/// `अस्त.त१` declares `वाक्यकोश` and `अभिव्यञ्जककोश` but has NO appender for
/// either: `वाक्ययोजनम्` and `अभिव्यञ्जकयोजनम्` live in `parse.t1`, which
/// does not lex as T1 (`the_corpus_has_no_routine_the_frozen_parser_can_…`
/// reports it). So the arenas are built here, which is also what lets this
/// test choose the shapes rather than take whatever a parser happened to emit.
fn statement_arena(rows: &[(i128, i128, i128)]) -> Value {
    let mut v = vec![Value::Nil];
    for (i, (kind, left, right)) in rows.iter().enumerate() {
        let mut m = HashMap::new();
        m.insert("भेद".to_string(), Value::Int(*kind));
        // `W-240`: the location is a TOKEN INDEX now, ० for a synthetic statement.
        m.insert("स्थानसूचकाङ्क".to_string(), Value::Int(0));
        m.insert("वामसूचकाङ्क".to_string(), Value::Int(*left));
        m.insert("दक्षिणसूचकाङ्क".to_string(), Value::Int(*right));
        // THE TWO FIELDS THIS HELPER USED TO OMIT, added 2026-09-04 by `W-202`.
        // A record built here is read by the same routines that read a PARSED
        // one, so a field the real `वास्तुॱवाक्य` has and this does not is a
        // synthetic tree that cannot be walked — and the reader fails inside
        // the call rather than on an assertion, which says nothing about what
        // is wrong. That is how it presented: `वाक्यप्रकारः 2 runs: <error>`,
        // once the समूह arm began reading `आदिसूचकाङ्क` to walk a block's
        // children.
        //
        // `आदिसूचकाङ्क` is where a statement's own subtree STARTS. Rows here are
        // leaves or blocks over rows already pushed, so its own 1-based index
        // is correct for a leaf and safe for a block: the walk steps to
        // `आदिसूचकाङ्क - 1`, which must DECREASE or the scan cannot terminate.
        // `अन्यसूचकाङ्क` is ० — no `अन्यथा` — exactly as `वाक्ययोजनम्` writes it.
        m.insert("आदिसूचकाङ्क".to_string(), Value::Int(i as i128 + 1));
        m.insert("अन्यसूचकाङ्क".to_string(), Value::Int(0));
        v.push(Value::Record(Rc::new(RefCell::new(m))));
    }
    Value::Arena(Rc::new(RefCell::new(v)))
}

/// Push one `वास्तुॱअभिव्यञ्जक` into the module-level `अभिव्यञ्जककोश`.
///
/// `Interpreter::global` hands back the `Value`, and an arena is an
/// `Rc<RefCell<Vec<..>>>`, so the store a routine reads can be filled from
/// here. Returns the one-based index it landed at.
fn push_expression(it: &Interpreter, kind: i128, left: i128) -> i128 {
    let Some(Value::Arena(a)) = it.global("अभिव्यञ्जककोश") else {
        panic!("वास्तु declares अभिव्यञ्जककोश and it must be an arena")
    };
    let mut m = HashMap::new();
    m.insert("भेद".to_string(), Value::Int(kind));
    m.insert("मूल्यसूचकाङ्क".to_string(), Value::Int(0));
    m.insert("वामसूचकाङ्क".to_string(), Value::Int(left));
    m.insert("दक्षिणसूचकाङ्क".to_string(), Value::Int(0));
    m.insert("द्विकर्म".to_string(), Value::Int(0));
    let mut a = a.borrow_mut();
    a.push(Value::Record(Rc::new(RefCell::new(m))));
    i128::try_from(a.len() - 1).expect("an arena index fits")
}

#[test]
fn a_statement_takes_the_type_of_what_it_states() {
    // `वाक्यप्रकारः` is `typecheck_statement`, both of Rust's arms. It is the
    // only one of the five type-directed stubs that could be written: its
    // `कोशः` is a PARAMETER, so it does not need the declaration arena the
    // two entry points wait on.
    let mut it = load_sema();

    // `अङ्काभिव्यञ्जकभेद` (२) — a Numeral, whose type is `Ty::Int{32,signed}`.
    let numeral = push_expression(&it, 2, 0);
    assert_eq!(numeral, 1, "the first pushed expression is index १");

    // १ = Expression(numeral).  २ = Block whose अन्तिम is statement १.
    let kosha = statement_arena(&[(1, numeral, 0), (2, 1, 1)]);

    let at = |it: &mut Interpreter, i: i128| {
        it.call(
            "अर्थॱवाक्यप्रकारः",
            vec![kosha.clone(), Value::Int(i)],
            5_000_000,
        )
        .unwrap_or_else(|e| panic!("वाक्यप्रकारः {i} runs: {e}"))
    };

    // Statement::Expression => typecheck_expression => Ty::Int{32,signed}.
    // `पूर्णाङ्कार्थभेद` is १ and the file's own table says so.
    assert_eq!(
        ty_triple(&at(&mut it, 1)),
        (Some(1), Some(32), Some(true)),
        "an expression statement over a numeral is अ३२"
    );

    // Statement::Block => the fold's value, which is the LAST statement's
    // type. Rust folds `last_ty` and returns it; वास्तु records the block's
    // last statement in दक्षिणसूचकाङ्क.
    assert_eq!(
        ty_triple(&at(&mut it, 2)),
        (Some(1), Some(32), Some(true)),
        "a block's type is its last statement's type"
    );

    // ० is व्याकर's "no statement", which is Rust's `last_ty = Ty::Void`
    // before the fold runs — NOT poison. शून्यार्थभेद is १०.
    assert_eq!(kind_of(&at(&mut it, 0)), Some(10), "an empty body is Void");

    // Past the end is दोषार्थभेद (१२), the poison that propagates rather than
    // stopping the pass — the same answer अभिव्यञ्जकप्रकारः gives.
    assert_eq!(
        kind_of(&at(&mut it, 9)),
        Some(12),
        "an index outside the arena is the poison type"
    );

    // प्रत्यागमनवाक्यभेद (४) IS अभावार्थभेद (११), NOT POISON — CHANGED
    // 2026-09-02 AND THIS TEST ASSERTED THE OLD RULE.
    //
    // It read `Some(12)` and said "there is no rule to port". That was the
    // considered position and it was wrong for THIS kind: the rule for a
    // return is not missing from Rust, it is inside `प्रत्यागमनसाम्यम्`
    // already, written as `body_ty != Ty::Never`. Nothing could produce
    // `Ty::Never`, so the escape had never once fired, and
    // `measure_corpus_typecheck` found SIX of fifteen sources refused because
    // a body ending in a return fell to the poison at the bottom of
    // `वाक्यप्रकारः`. A return DIVERGES; there is no value to disagree about.
    let ret = statement_arena(&[(4, numeral, 0)]);
    let v = it
        .call("अर्थॱवाक्यप्रकारः", vec![ret, Value::Int(1)], 5_000_000)
        .expect("runs");
    assert_eq!(
        kind_of(&v),
        Some(11),
        "a `प्रत्यागमनम्` statement types as अभावार्थः (Never): control leaves,          so the block it ends produces no value to compare against a declared          return type"
    );

    // `चर` (३) IS शून्यार्थः NOW, NOT POISON — changed 2026-09-04 by `W-202`.
    //
    // This assertion read `Some(12)` and its note said the other five "still
    // have no rule and are still poison". Four of the five have one now: यदि
    // and यावत् got the branch rule this row was opened for, and चर, आयात and
    // सम got the Void the old margin beside them PROMISED — "चर, आयात and सम
    // produce no value (Void, once something needs it)".
    //
    // THE FOLD IS WHAT NEEDED IT. Once the समूह arm walked every statement
    // rather than only a block's last, leaving these three as poison refused
    // any body containing a `चरः` — which is nearly every routine in the
    // corpus. Poison says "this did not typecheck"; Void says "this has no
    // value", and for a declaration the second is simply true.
    let decl = statement_arena(&[(3, numeral, 0)]);
    let v = it
        .call("अर्थॱवाक्यप्रकारः", vec![decl, Value::Int(1)], 5_000_000)
        .expect("runs");
    assert_eq!(
        kind_of(&v),
        Some(10),
        "a `चरः` declaration produces NO VALUE, which is शून्यार्थः and not \
         poison. A body ENDING in one still refuses against a declared non-Void \
         return, because Void is not न६४ — the same refusal as before, for the \
         right reason"
    );

    // AND THE POINT THE OLD ASSERTION WAS MAKING IS KEPT, because it is the
    // right point: this must not become a blanket "anything unknown is fine".
    // All eight kinds `वास्तु` declares now have a rule, so the discriminating
    // case is a kind that does not exist — poison is for what this pass has
    // never heard of, and it must still propagate rather than be waved through.
    let alien = statement_arena(&[(99, numeral, 0)]);
    let v = it
        .call("अर्थॱवाक्यप्रकारः", vec![alien, Value::Int(1)], 5_000_000)
        .expect("runs");
    assert_eq!(
        kind_of(&v),
        Some(12),
        "a statement kind with no arm is still दोषार्थः. If this ever answers \
         Void, the checker has started accepting what it cannot read"
    );
}

#[test]
fn breaking_the_statement_typechecker_changes_the_type_it_answers() {
    // Two mutations, and both are answered by a VALUE rather than a refusal:
    // the block arm reading वाम instead of दक्षिण would return the first
    // statement rather than the last, and the empty-body guard reversed turns
    // Void into poison. Neither is visible to any text ratchet.
    let truth = {
        let mut it = load_sema();
        let n = push_expression(&it, 2, 0);
        let s = push_expression(&it, 3, 0);
        let k = statement_arena(&[(1, n, 0), (1, s, 0), (2, 2, 1)]);
        (
            it.call("अर्थॱवाक्यप्रकारः", vec![k.clone(), Value::Int(0)], 5_000_000)
                .ok()
                .map(|v| kind_of(&v)),
            it.call("अर्थॱवाक्यप्रकारः", vec![k, Value::Int(3)], 5_000_000)
                .ok()
                .map(|v| ty_triple(&v)),
        )
    };
    assert_eq!(truth.0, Some(Some(10)), "baseline: an empty body is Void");
    assert_eq!(
        truth.1,
        Some((Some(1), Some(32), Some(true))),
        "baseline: the block's अन्तिम is statement १, the numeral, so अ३२ — \
         and its प्रथम is statement २, a string literal, so वाम and दक्षिण \
         differ in the ANSWER and not only in the index"
    );

    let mutations: &[(&str, &str, &str)] = &[
        (
            "the empty-body guard is reversed, so a real statement is treated \
             as no statement and Void is answered for everything",
            "यदि सूचकाङ्क न्यूनम् १ आदि\n        प्रत्यागमनम् शून्यार्थः ।",
            "यदि सूचकाङ्क अधिकम् ० आदि\n        प्रत्यागमनम् शून्यार्थः ।",
        ),
        (
            "the block arm reads वाम instead of दक्षिण, so a block takes its \
             FIRST statement's type and not its last",
            // W-231: the arm now BINDS the last child's type before answering
            // with it (so the poison instrument can name the statement), so the
            // line that does the reading is a `चरः` and not the `प्रत्यागमनम्`.
            // A mutation keyed to a literal line has to move when the line
            // does — matching zero places proves nothing, which is what this
            // test caught when W-202 rephrased the same return.
            "चरः अन्तिमम् ॱॱ अर्थप्रकार भवति वाक्यप्रकारः कोशः वाक्यम् ॱ दक्षिणसूचकाङ्क ।",
            "चरः अन्तिमम् ॱॱ अर्थप्रकार भवति वाक्यप्रकारः कोशः वाक्यम् ॱ वामसूचकाङ्क ।",
        ),
    ];

    let mut proved = 0usize;
    for (what, from, to) in mutations {
        let broken = mutate(&source("artha.t1"), from, to);
        let mut it = load_sema_mutated(&broken);
        // The block's अन्तिम is statement १, a NUMERAL, and its प्रथम is
        // statement २, a STRING LITERAL — so वाम and दक्षिण differ in the
        // answer and not only in the index. Without that the second mutation
        // is silent.
        let n = push_expression(&it, 2, 0);
        let s = push_expression(&it, 3, 0);
        let k = statement_arena(&[(1, n, 0), (1, s, 0), (2, 2, 1)]);
        // `.ok()` and not `.unwrap()`: a mutation may make the routine RAISE
        // rather than answer wrongly, and it does — reversing the empty-body
        // guard sends index ० past it and into the reserved slot, where
        // `वाक्यम् ॱ भेद` asks a शून्यम् for a member. A refusal differs from
        // an answer, so both are killed the same way, and a test that
        // unwrapped would panic on the mutation instead of recording it.
        let got = (
            it.call("अर्थॱवाक्यप्रकारः", vec![k.clone(), Value::Int(0)], 5_000_000)
                .ok()
                .map(|v| kind_of(&v)),
            it.call("अर्थॱवाक्यप्रकारः", vec![k, Value::Int(3)], 5_000_000)
                .ok()
                .map(|v| ty_triple(&v)),
        );
        assert_ne!(got, truth, "MUTATION SURVIVED: {what}");
        proved += 1;
    }
    println!("METRIC sadhana_t1_statement_type_mutations_killed {proved}");
    assert_eq!(proved, mutations.len());
}

#[test]
fn the_type_checker_constructor_hands_back_the_resolver_it_borrows() {
    // `TypeChecker::new` HAS ONE FIELD, and until `W-272` it had three: the two
    // `HashMap::new()`s nothing in `typecheck.rs` ever inserted into are gone,
    // and the margin on the struct gives the argument — a map keyed by
    // `SymbolId` cannot go stale under a `&'a Resolver`, because that shared
    // borrow stops the numbering advancing for the checker's whole life. What
    // remains is the borrow itself, so the constructor IS the borrow, and this
    // asserts that the resolver comes back with its bindings intact rather than
    // as a fresh one. (`resolved_symbols` WAS in the state the two maps were —
    // `pub`, never written, keyed by a `NodeId` nothing constructs — and was
    // DELETED for `W-274`, 2026-09-05. Being `pub` is what hid it: `dead_code`
    // does not fire on a public field, so no lint could have named it and the
    // T1 directory's blanket allow was never the cause.)
    let mut it = load_sema();
    let r = it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000).expect("new");
    it.call(
        "अर्थॱघोषणम्",
        vec![r.clone(), octets("क"), Value::Int(0)],
        5_000_000,
    )
    .expect("declare");
    let tc = it
        .call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r.clone()], 5_000_000)
        .expect("प्रकारपरीक्षकारम्भः runs");
    let found = it
        .call("अर्थॱनामनिर्णयः", vec![tc, octets("क")], 5_000_000)
        .expect("नामनिर्णयः runs on what the constructor returned")
        .as_int();
    assert_eq!(
        found,
        Some(1),
        "the type checker must see the resolver's bindings; a fresh निर्णायक \
         answers शून्यम् here"
    );
}

#[test]
fn the_sema_still_says_which_of_its_stubs_are_stubs() {
    // **The guard on this row's own honesty.** Five routines are still stubs
    // and each returns `अपूर्णम्`. If one of them starts answering, this test
    // fails and the file's blocker notes must be re-read rather than the
    // number quietly raised — the same shape as the ratchet in
    // `t1_sources.rs`, but measured by RUNNING them.
    let mut it = load_sema();
    // `अर्थॱअभिव्यञ्जकनिर्णयः` LEFT THIS LIST 2026-09-01, and the blocker it
    // was waiting on is (c): the AST carries TOKEN INDICES, not name text, so
    // the Identifier arm could not recover the `नाम ॱॱ पाठ` that नामनिर्णयः
    // takes. Discharged by `आयातः पदविभाग` — a decision, not a gap, and taken
    // on its own line with the 4-0 precedent recorded. Four remain.
    let stubs = [
        // `अर्थॱकार्यक्रमनिर्णयः` LEFT THIS LIST 2026-09-01. Blocker (b) had
        // already reduced itself to (c) — declarations ARE reachable, the
        // obstacle was that `घोषणा ॱ नामसूचकाङ्क` is a token index while
        // `घोषणम्` takes text. (c) is discharged, so what remained of (b) was
        // one import: `आयातः व्याकर`, precedent ir.t1 and lib.t1, non-circular.
        // `अर्थॱवाक्यनिर्णयः` LEFT THIS LIST 2026-09-01. It was not waiting on
        // a scope stack — that worked — but on वास्तु being unable to say
        // which statements belong to a block. `वास्तुॱवाक्य ॱ आदिसूचकाङ्क`
        // says it now, so the block arm visits each DIRECT child once.
        // `अर्थॱकार्यक्रमप्रकारपरीक्षा` LEFT THIS LIST 2026-09-01, AND IT WAS
        // THE LAST ONE. All three of its parts existed — प्रत्यागमनसाम्यम्
        // is the rule, वाक्यप्रकारः types the body, प्रकारार्थः gives the
        // declared return type — and what it could not do was FIND the
        // declarations, which was (b) and reduced to `आयातः व्याकर`.
        // `अर्थॱप्रकारार्थः` LEFT THIS LIST 2026-09-01. Its PRIMITIVE arm
        // was blocker (c) — a token index where मूलप्रकारार्थः wants text —
        // and (c) is discharged. Its four WRAPPER arms are blocker (d), and
        // they follow this file's OWN precedent rather than deciding it:
        // `अभिव्यञ्जकप्रकारः` meets the same hole at its string-literal arm
        // and writes `अन्तःसूचकाङ्क भवति ०`. So a wrapper answers the right
        // SHAPE with an unrepresented inner type — weaker than Rust, and the
        // same weakness already carried, liftable the day (d) is answered.
    ];
    for (name, arity) in stubs {
        let args: Vec<Value> = (0..arity).map(|_| Value::Int(0)).collect();
        let v = it
            .call(name, args, 5_000_000)
            .unwrap_or_else(|e| panic!("{name} runs: {e}"));
        assert_eq!(
            v.octets().map(|o| o.as_slice().to_vec()),
            Some("अपूर्णम्".as_bytes().to_vec()),
            "{name} answered {v:?}. If it is now written, move it out of this \
             list and say which blocker — (b), (c) or (d) — was discharged"
        );
    }

    // THE LIST IS EMPTY, AND AN EMPTY `for` IS A GUARD THAT HAS STOPPED
    // GUARDING — this file's own words about the same shape in
    // `the_phrase_structure_is_frozen…`. So the guard is INVERTED rather than
    // left to pass vacuously: every routine the sema declares must now answer
    // SOMETHING, and a new `अपूर्णम्` anywhere in `अर्थ` fails here.
    //
    // That is the assertion the empty list can no longer make, and it is
    // stronger: the old one watched five named routines, this one watches all
    // of them.
    let artha = source("artha.t1");
    let stubbed: Vec<&str> = artha
        .lines()
        .filter(|l| l.contains("प्रत्यागमनम् उक्तम् अपूर्णम्"))
        .collect();
    assert!(
        stubbed.is_empty(),
        "`artha.t1` has {} stub(s) again. It reached ZERO on 2026-09-01, so a \
         routine answering अपूर्णम् is either new work or a regression — and \
         either way it belongs in the list above with its blocker named. \
         Lines: {stubbed:?}",
        stubbed.len()
    );
    println!("METRIC sadhana_t1_sema_stubs_remaining {}", stubs.len());
}

// ─────────────────────────────────────────────────────────────────────────
// The encoding table — `सङ्केतन ॱ सङ्केताः`, `encode.rs`'s `encodings`.
//
// **The port's signature diverges from the Rust original's, by ADR-0026.**
// `encodings()` answers `Vec<Encoding>`; `सङ्केताः` answers a ROW NUMBER for a
// `(name, width)`, because not one of the four Rust call sites keeps the
// vector — every one is a `.find` over rows. So these tests assert a row
// OFFSET, and then read a field at it, which is what a caller does.
//
// Everything below is computed from `spec/encodings-riscv64.tsv` by THIS test
// with Rust's own `lines`/`split`, so the T1 reader is checked against the
// FILE and not against a second copy of itself.
// ─────────────────────────────────────────────────────────────────────────

/// `(insn, bits, byte offset of the row, pattern)` for every data row of
/// `spec/encodings-riscv64.tsv`, under the rule `सङ्केताः` states.
///
/// The offset is a BYTE offset into the file's raw text, because that is what
/// the embed hands the routine and what `पङ्क्तिसीमा` walks.
fn encoding_table() -> Vec<(String, i128, usize, i128)> {
    let text = std::fs::read_to_string(spec_root().join("encodings-riscv64.tsv"))
        .expect("spec/encodings-riscv64.tsv exists");
    let mut out = Vec::new();
    let mut at = 0usize;
    for line in text.split('\n') {
        let start = at;
        at += line.len() + 1;
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        // Rust's `f.len() < 7` guard and its `bits`/`pattern` parses. The
        // header row falls out here: its field ५ is the word `bits`.
        if f.len() < 7 {
            continue;
        }
        let Ok(bits) = f[5].parse::<i128>() else {
            continue;
        };
        let Ok(pattern) = i128::from_str_radix(f[3].trim_start_matches("0x"), 16) else {
            continue;
        };
        out.push((f[0].to_string(), bits, start, pattern));
    }
    out
}

#[test]
fn the_encoding_reader_answers_the_row_the_table_carries() {
    let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
    let table = encoding_table();
    assert!(
        table.len() >= 150,
        "only {} encoding rows read; the table is the evidence",
        table.len()
    );

    let mut checked = 0usize;
    for (insn, bits, start, _) in &table {
        // `सङ्केताः` is ONE-BASED, for `कोष्ठपङ्क्तिः`'s reason: row ० begins
        // at offset ० and ० is also "no such row".
        //
        // The table may carry the same `(insn, bits)` twice; Rust's `.find`
        // takes the FIRST, so the assertion is "the first such row" and not
        // "the row this loop is standing on".
        let (first, pattern) = table
            .iter()
            .find(|(n, b, _, _)| n == insn && b == bits)
            .map(|(_, _, s, p)| (*s, *p))
            .expect("the row being iterated is itself a match");

        let v = it
            .call(
                "सङ्केतनॱसङ्केताः",
                vec![octets(insn), Value::Int(*bits)],
                60_000_000,
            )
            .unwrap_or_else(|e| panic!("`सङ्केताः {insn} {bits}` runs: {e}"));
        assert_eq!(
            v.as_int(),
            Some(first as i128 + 1),
            "`सङ्केताः {insn} {bits}` answered {v:?}; the file puts the first \
             such row at byte {first} and the routine answers one-based \
             (this row is at {start})"
        );

        // And a field AT that row: `सङ्केताकृतिः` is `f[3]`, the pattern. A
        // row number nothing reads through is not evidence the row is right.
        let p = it
            .call(
                "सङ्केतनॱसङ्केताकृतिः",
                vec![octets(insn), Value::Int(*bits)],
                60_000_000,
            )
            .unwrap_or_else(|e| panic!("`सङ्केताकृतिः {insn} {bits}` runs: {e}"));
        assert_eq!(
            p.as_int(),
            Some(pattern),
            "`सङ्केताकृतिः {insn} {bits}` answered {p:?}; the file says {pattern:#x}"
        );
        checked += 1;
    }
    println!("METRIC sadhana_t1_encodings_executed {checked}");

    // The other half of Rust's `Option`: a name no row carries is ०.
    let missing = it
        .call(
            "सङ्केतनॱसङ्केताः",
            vec![octets("अविद्यमानम्"), Value::Int(32)],
            60_000_000,
        )
        .expect("`सङ्केताः` runs on an unknown name");
    assert_eq!(
        missing.as_int(),
        Some(0),
        "a name the table does not carry must answer ०, not {missing:?}"
    );
    let missing_pattern = it
        .call(
            "सङ्केतनॱसङ्केताकृतिः",
            vec![octets("अविद्यमानम्"), Value::Int(32)],
            60_000_000,
        )
        .expect("`सङ्केताकृतिः` runs on an unknown name");
    assert!(
        missing_pattern.is_nil(),
        "an unknown name has no pattern; got {missing_pattern:?}"
    );

    // **The width is load-bearing, and this is what proves it.** `add` is a
    // 32-bit row and `c.add` is a 16-bit one; asking for `add` at 16 bits must
    // find nothing, or the `bits` half of three of the four Rust `.find`s is
    // decoration the port dropped without saying so.
    let wrong_width = it
        .call(
            "सङ्केतनॱसङ्केताः",
            vec![octets("add"), Value::Int(16)],
            60_000_000,
        )
        .expect("`सङ्केताः` runs with a width no row of that name carries");
    assert_eq!(
        wrong_width.as_int(),
        Some(0),
        "`add` has no 16-bit encoding; got {wrong_width:?}"
    );

    // Width ० means ANY width — encode.rs:794's unconstrained
    // `.find(|e| e.insn == decoded.insn)`. It must answer the FIRST row of
    // that name whatever its width.
    let any = it
        .call(
            "सङ्केतनॱसङ्केताः",
            vec![octets("add"), Value::Int(0)],
            60_000_000,
        )
        .expect("`सङ्केताः` runs with the any-width wildcard");
    let first_add = table
        .iter()
        .find(|(n, _, _, _)| n == "add")
        .map(|(_, _, s, _)| *s)
        .expect("the table carries `add`");
    assert_eq!(
        any.as_int(),
        Some(first_add as i128 + 1),
        "width ० must match a row of any width; got {any:?}"
    );

    // A name that is a PREFIX of a real one must not match it: `addi` is a
    // row, `add` is a different row, and `ad` is neither.
    let prefix = it
        .call(
            "सङ्केतनॱसङ्केताः",
            vec![octets("ad"), Value::Int(32)],
            60_000_000,
        )
        .expect("`सङ्केताः` runs on a prefix");
    assert_eq!(
        prefix.as_int(),
        Some(0),
        "`ad` is a prefix of `add` and is not itself a row; got {prefix:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The compression table — `सङ्केतन ॱ रूपाणि`, `encode.rs`'s `forms_of`.
//
// **Signature diverges by ADR-0026 too.** `forms_of` answers
// `Vec<(&str, &str)>` and its ONE caller, encode.rs:801, walks it in table
// order and returns on the first success; `रूपाणि` answers the COUNT and
// `रूपपङ्क्तिः` the row of the nth, which is that walk exactly.
// ─────────────────────────────────────────────────────────────────────────

/// For every wide mnemonic in `spec/compression-choices.tsv`, its DISTINCT
/// `(compressed, relation)` forms in table order, each with the byte offset of
/// the row that first carried it.
///
/// The dedup is Rust's `!out.contains(&(f[2], f[3]))`, and it is not a
/// formality: `addi` has five matching rows and four distinct forms.
/// One mnemonic's distinct compressed forms: `(form, operands, row)` per entry.
///
/// Named because the tuple is nested three deep and clippy refuses it inline;
/// naming it also says what the shape MEANS, which the tuple did not.
type CompressionForms = Vec<(String, String, usize)>;
/// Every mnemonic of `spec/compression-choices.tsv` with its distinct forms.
type CompressionTable = Vec<(String, CompressionForms)>;

fn compression_table() -> CompressionTable {
    let text = std::fs::read_to_string(spec_root().join("compression-choices.tsv"))
        .expect("spec/compression-choices.tsv exists");
    let mut out: CompressionTable = Vec::new();
    let mut at = 0usize;
    for line in text.split('\n') {
        let start = at;
        at += line.len() + 1;
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 4 || f[1] == "-" {
            continue;
        }
        let Some(wide) = f[0].split_whitespace().next() else {
            continue;
        };
        let slot = match out.iter().position(|(w, _)| w == wide) {
            Some(i) => i,
            None => {
                out.push((wide.to_string(), Vec::new()));
                out.len() - 1
            }
        };
        if !out[slot].1.iter().any(|(a, b, _)| a == f[2] && b == f[3]) {
            out[slot]
                .1
                .push((f[2].to_string(), f[3].to_string(), start));
        }
    }
    out
}

#[test]
fn the_compression_reader_counts_the_distinct_forms_the_table_carries() {
    let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
    let raw = std::fs::read_to_string(spec_root().join("compression-choices.tsv"))
        .expect("spec/compression-choices.tsv exists");
    let table = compression_table();
    assert!(
        table.len() >= 15,
        "only {} wide mnemonics read; the table is the evidence",
        table.len()
    );

    // **The dedup must actually be exercised by this data.** If no row of the
    // file repeats an earlier form, the clause that implements Rust's
    // `contains` is untested and every assertion below would pass without it.
    let matching = raw
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            f.len() >= 4 && f[1] != "-"
        })
        .count();
    let distinct: usize = table.iter().map(|(_, v)| v.len()).sum();
    assert!(
        matching > distinct,
        "no row of spec/compression-choices.tsv repeats an earlier form, so \
         this test cannot tell a reader that dedups from one that does not"
    );

    let mut checked = 0usize;
    for (wide, forms) in &table {
        let n = it
            .call("सङ्केतनॱरूपाणि", vec![octets(wide)], 60_000_000)
            .unwrap_or_else(|e| panic!("`रूपाणि {wide}` runs: {e}"));
        assert_eq!(
            n.as_int(),
            Some(forms.len() as i128),
            "`रूपाणि {wide}` answered {n:?}; the file carries {} distinct \
             form(s) for it",
            forms.len()
        );

        for (i, (insn, relation, start)) in forms.iter().enumerate() {
            let row = it
                .call(
                    "सङ्केतनॱरूपपङ्क्तिः",
                    vec![octets(wide), Value::Int(i as i128)],
                    60_000_000,
                )
                .unwrap_or_else(|e| panic!("`रूपपङ्क्तिः {wide} {i}` runs: {e}"));
            assert_eq!(
                row.as_int(),
                Some(*start as i128 + 1),
                "`रूपपङ्क्तिः {wide} {i}` answered {row:?}; the file puts \
                 `{insn}`/`{relation}` at byte {start}, and the routine \
                 answers one-based"
            );
            checked += 1;
        }

        // One past the last distinct form is ०, not a row.
        let past = it
            .call(
                "सङ्केतनॱरूपपङ्क्तिः",
                vec![octets(wide), Value::Int(forms.len() as i128)],
                60_000_000,
            )
            .unwrap_or_else(|e| panic!("`रूपपङ्क्तिः {wide} past-end` runs: {e}"));
        assert_eq!(
            past.as_int(),
            Some(0),
            "`{wide}` has {} distinct forms, so index {} is not one; got {past:?}",
            forms.len(),
            forms.len()
        );
    }
    println!("METRIC sadhana_t1_compression_forms_executed {checked}");

    // A mnemonic the table does not carry has ० forms — Rust's empty vector,
    // and the `for` loop over it that then does nothing.
    let none = it
        .call("सङ्केतनॱरूपाणि", vec![octets("अविद्यमानम्")], 60_000_000)
        .expect("`रूपाणि` runs on an unknown mnemonic");
    assert_eq!(
        none.as_int(),
        Some(0),
        "a mnemonic the table does not carry has no forms; got {none:?}"
    );

    // **A mnemonic the assembler left WIDE in every row that names it has ०
    // forms.** This is the `f[1] == "-"` clause, and nothing above can see it:
    // every wide in `table` has at least one non-`-` row by construction.
    let mut all_dash: Vec<(&str, bool)> = Vec::new();
    for l in raw.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 4 {
            continue;
        }
        let Some(w) = f[0].split_whitespace().next() else {
            continue;
        };
        match all_dash.iter_mut().find(|(x, _)| *x == w) {
            Some(e) => e.1 &= f[1] == "-",
            None => all_dash.push((w, f[1] == "-")),
        }
    }
    let mut refused = 0usize;
    for (w, _) in all_dash.iter().filter(|(_, d)| *d) {
        let v = it
            .call("सङ्केतनॱरूपाणि", vec![octets(w)], 60_000_000)
            .unwrap_or_else(|e| panic!("`रूपाणि {w}` runs: {e}"));
        assert_eq!(
            v.as_int(),
            Some(0),
            "`{w}` is left wide in every row that names it; got {v:?}"
        );
        refused += 1;
    }
    println!("METRIC sadhana_t1_compression_wide_only_refused {refused}");
}

// ─────────────────────────────────────────────────────────────────────────
// The mutation proof for the two table readers blocker (a) closed.
// ─────────────────────────────────────────────────────────────────────────

/// The answers the mutation test compares: a sample of `सङ्केताः` row numbers
/// and EVERY `रूपाणि` count.
///
/// The encoding side is sampled — every twenty-fifth row, plus `add`/`addi`
/// by name — because a mutation run reloads and re-runs the whole chain once
/// per mutation under the `dev` profile. The compression side is not sampled:
/// there are only eighteen wide mnemonics, and the dedup mutations below are
/// visible in exactly one of them.
fn read_table_sample(it: &mut Interpreter) -> Vec<(String, Option<Option<i128>>)> {
    let mut out = Vec::new();
    let call = |it: &mut Interpreter, r: &str, args: Vec<Value>| {
        it.call(r, args, 60_000_000).ok().map(|v| v.as_int())
    };

    let encodings = encoding_table();
    let mut probes: Vec<(String, i128)> = encodings
        .iter()
        .step_by(25)
        .map(|(n, b, _, _)| (n.clone(), *b))
        .collect();
    probes.push(("add".to_string(), 32));
    probes.push(("addi".to_string(), 32));
    probes.push(("add".to_string(), 16));
    probes.push(("अविद्यमानम्".to_string(), 32));
    for (name, bits) in probes {
        let v = call(it, "सङ्केतनॱसङ्केताः", vec![octets(&name), Value::Int(bits)]);
        out.push((format!("सङ्केताः {name} {bits}"), v));
        let p = call(it, "सङ्केतनॱसङ्केताकृतिः", vec![octets(&name), Value::Int(bits)]);
        out.push((format!("सङ्केताकृतिः {name} {bits}"), p));
    }

    for (wide, forms) in compression_table() {
        let v = call(it, "सङ्केतनॱरूपाणि", vec![octets(&wide)]);
        out.push((format!("रूपाणि {wide}"), v));
        for i in 0..forms.len() {
            let r = call(
                it,
                "सङ्केतनॱरूपपङ्क्तिः",
                vec![octets(&wide), Value::Int(i as i128)],
            );
            out.push((format!("रूपपङ्क्तिः {wide} {i}"), r));
        }
    }
    out
}

/// One mutation of `encode.t1`, in the chain the two new table readers walk.
///
/// **`क्षेत्रसंख्या` IS NOT IN THIS TABLE EITHER, AND THE REASON IS A
/// REFUTATION.** `REGISTER_CHAIN_MUTATIONS` above records that inverting its
/// TAB test leaves all 64 register answers unchanged, and predicts that "the
/// routine needs a caller that compares its answer for equality —
/// `सङ्केताः`, when blocker (a) lifts — before a mutation of it is observable
/// at all." Blocker (a) lifted on 2026-08-30, `सङ्केताः` and `रूपधारकः` are
/// those callers, and **the prediction is false — measured, by putting the
/// mutation in this list and running it.** The inverted count answers a
/// number that is far too LARGE (every non-TAB octet, plus one), and both new
/// callers test it against a FLOOR: `अधिकम् ६` in `सङ्केताः` and `न्यूनम् ४`
/// in `रूपधारकः`. A count wrong upwards clears a floor, so all 60 sampled
/// answers were identical and the mutation survived.
///
/// **No faithful port can fix this, and that is the finding.** Rust's four
/// readers test `f.len()` at `< 7`, `<= 3`, `< 4` and `>= 4` — every one a
/// floor, not an equality. So there is no call site in `encode.rs` for a port
/// to mirror that would make the TAB count observable, and the register
/// chain's note should be read as naming a caller that does not exist rather
/// than one that had not been written yet. Making it observable needs a test
/// that calls `क्षेत्रसंख्या` directly and asserts the number, which is a
/// different test from this one and belongs to whoever wants it.
const TABLE_READER_MUTATIONS: &[(&str, &str, &str, &str)] = &[
    (
        // अंशसाम्यम् → सूचितांशसाम्यम् ON 2026-09-14, for the reason recorded at
        // `REGISTER_CHAIN_MUTATIONS`: the table index landed a second width
        // test that reads the width PARSED ONCE when the index was built, and
        // the readers go through it now. The old anchor still matched — in a
        // routine off the path — so the control applied cleanly and proved
        // nothing. Anchored to the new routine's declaration, it bites again.
        "सूचितांशसाम्यम्",
        "the refusal of a non-numeric width column is reversed, so the header \
         row's `bits` becomes a width and every real width stops being one",
        "सार्वजनिक वृत्तिः सूचितांशसाम्यम् आदाय सूचीक्रमः ॱॱ अ६४ अंशाः ॱॱ अ६४ ददाति बूल आदि\n    चरः मूल्यम् ॱॱ अ६४ भवति सङ्केतसूच्यंशकोश अङ्कः सूचीक्रमः अन्तः ।\n\n    यदि मूल्यम् समम् ० आदि",
        "सार्वजनिक वृत्तिः सूचितांशसाम्यम् आदाय सूचीक्रमः ॱॱ अ६४ अंशाः ॱॱ अ६४ ददाति बूल आदि\n    चरः मूल्यम् ॱॱ अ६४ भवति सङ्केतसूच्यंशकोश अङ्कः सूचीक्रमः अन्तः ।\n\n    यदि मूल्यम् असमम् ० आदि",
    ),
    (
        "सङ्केताः",
        "the seven-field guard is reversed, so only short rows are considered",
        "यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा अधिकम् ६ आदि",
        "यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा न्यूनम् ६ आदि",
    ),
    // **ROW STRUCK 2026-09-14 — the defect it describes is no longer on this
    // chain.** It mutated `प्रथमपदसाम्यम्`, which ends a key at a space by
    // scanning for one; the table index now stores each row's field bounds when
    // it is built, so `सङ्केतसूचीक्रमः` compares against those bounds through
    // `परिधिसाम्यम्` and never scans for a space at all. The mutation applied
    // cleanly and the readers answered identically — a dead control, the third
    // from that one landing, after `अंशसाम्यम्` above and `क्षेत्रसाम्यम्` in
    // `REGISTER_CHAIN_MUTATIONS`.
    //
    // `प्रथमपदसाम्यम्` IS STILL LIVE, at `encode.t1:4953`, so this is not a dead
    // routine and must not be read as one — it is a live routine that this
    // test's sample no longer reaches. Struck rather than re-anchored because
    // re-anchoring it here would only prove it again on a path this test does
    // not read; the row belongs to whatever test exercises 4953.
    (
        "सङ्कोचपङ्क्तिवत्",
        "the `-` test is reversed, so the rows the assembler left WIDE become \
         the only ones counted",
        "सार्वजनिक वृत्तिः सङ्कोचपङ्क्तिवत् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ अ६४ सीमा ॱॱ अ६४ ददाति बूल आदि\n    यदि उपेक्ष्यपङ्क्तिः पाठ्यम् आरम्भः सीमा समम् सत्यम् आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा न्यूनम् ४ आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    चरः अर्धादिः ॱॱ अ६४ भवति क्षेत्रारम्भः पाठ्यम् आरम्भः सीमा १ ।\n    चरः अर्धान्तः ॱॱ अ६४ भवति क्षेत्रसीमा पाठ्यम् आरम्भः सीमा १ ।\n\n    यदि आरभ्य अर्धान्तः वियोगः अर्धादिः समाप्तम् समम् १ आदि\n        यदि पाठ्यम् अङ्कः अर्धादिः अन्तः समम् ४५ आदि",
        "सार्वजनिक वृत्तिः सङ्कोचपङ्क्तिवत् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ अ६४ सीमा ॱॱ अ६४ ददाति बूल आदि\n    यदि उपेक्ष्यपङ्क्तिः पाठ्यम् आरम्भः सीमा समम् सत्यम् आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा न्यूनम् ४ आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    चरः अर्धादिः ॱॱ अ६४ भवति क्षेत्रारम्भः पाठ्यम् आरम्भः सीमा १ ।\n    चरः अर्धान्तः ॱॱ अ६४ भवति क्षेत्रसीमा पाठ्यम् आरम्भः सीमा १ ।\n\n    यदि आरभ्य अर्धान्तः वियोगः अर्धादिः समाप्तम् समम् १ आदि\n        यदि पाठ्यम् अङ्कः अर्धादिः अन्तः असमम् ४५ आदि",
    ),
    (
        "सङ्कोचपङ्क्तिवत्",
        "the four-field guard is reversed, so every row of a four-field table \
         is refused",
        "सार्वजनिक वृत्तिः सङ्कोचपङ्क्तिवत् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ अ६४ सीमा ॱॱ अ६४ ददाति बूल आदि\n    यदि उपेक्ष्यपङ्क्तिः पाठ्यम् आरम्भः सीमा समम् सत्यम् आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा न्यूनम् ४ आदि",
        "सार्वजनिक वृत्तिः सङ्कोचपङ्क्तिवत् आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ अ६४ सीमा ॱॱ अ६४ ददाति बूल आदि\n    यदि उपेक्ष्यपङ्क्तिः पाठ्यम् आरम्भः सीमा समम् सत्यम् आदि\n        प्रत्यागमनम् असत्यम् ।\n    इति\n\n    यदि क्षेत्रसंख्या पाठ्यम् आरम्भः सीमा अधिकम् ३ आदि",
    ),
    (
        "सङ्कोचसूचीरचना",
        "the backward scan never starts, so Rust's `contains` dedup is gone \
         and `addi` reports its five rows instead of its four forms",
        "सार्वजनिक वृत्तिः सङ्कोचसूचीरचना ददाति अ६४ आदि\n    यदि सङ्कोचसूचीसंख्या अधिकम् ० आदि\n        प्रत्यागमनम् सङ्कोचसूचीसंख्या ।\n    इति\n\n    चरः पाठ्यम् ॱॱ अङ्कः अन्तः अ८ भवति पदविभागॱसमावेशपाठः उक्तम् सङ्कोचकोशः इति ।\n    चरः आरम्भः ॱॱ अ६४ भवति ० ।\n\n    यावत् आरम्भः न्यूनम् पाठ्यम् ॱ दैर्घ्य आदि\n        चरः सीमा ॱॱ अ६४ भवति पङ्क्तिसीमा पाठ्यम् आरम्भः ।\n\n        यदि सङ्कोचपङ्क्तिवत् पाठ्यम् आरम्भः सीमा समम् सत्यम् आदि\n            सङ्कोचसूचीसंख्या भवति आरभ्य सङ्कोचसूचीसंख्या योगः १ समाप्तम् ।\n            सङ्कोचसूच्यादिकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति आरम्भः ।\n            सङ्कोचसूचीसीमाकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति सीमा ।\n            चरः क्षेत्रादिः ॱॱ अ६४ भवति क्षेत्रारम्भः पाठ्यम् आरम्भः सीमा ० ।\n            चरः क्षेत्रान्तः ॱॱ अ६४ भवति क्षेत्रसीमा पाठ्यम् आरम्भः सीमा ० ।\n            चरः पदान्तः ॱॱ अ६४ भवति अष्टकान्वेषणम् पाठ्यम् क्षेत्रादिः क्षेत्रान्तः ३२ ।\n            सङ्कोचसूचीपदादिकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति क्षेत्रादिः ।\n            सङ्कोचसूचीपदान्तकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति पदान्तः ।\n            सङ्कोचसूचीपूर्वदृष्टकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति ० ।\n            चरः पदम् ॱॱ अङ्कः अन्तः अ८ भवति पाठ्यम् अङ्कः क्षेत्रादिः अन्तः पदान्तः ।\n            चरः पूर्वक्रमः ॱॱ अ६४ भवति १ ।\n            यावत् पूर्वक्रमः न्यूनम् सङ्कोचसूचीसंख्या आदि",
        "सार्वजनिक वृत्तिः सङ्कोचसूचीरचना ददाति अ६४ आदि\n    यदि सङ्कोचसूचीसंख्या अधिकम् ० आदि\n        प्रत्यागमनम् सङ्कोचसूचीसंख्या ।\n    इति\n\n    चरः पाठ्यम् ॱॱ अङ्कः अन्तः अ८ भवति पदविभागॱसमावेशपाठः उक्तम् सङ्कोचकोशः इति ।\n    चरः आरम्भः ॱॱ अ६४ भवति ० ।\n\n    यावत् आरम्भः न्यूनम् पाठ्यम् ॱ दैर्घ्य आदि\n        चरः सीमा ॱॱ अ६४ भवति पङ्क्तिसीमा पाठ्यम् आरम्भः ।\n\n        यदि सङ्कोचपङ्क्तिवत् पाठ्यम् आरम्भः सीमा समम् सत्यम् आदि\n            सङ्कोचसूचीसंख्या भवति आरभ्य सङ्कोचसूचीसंख्या योगः १ समाप्तम् ।\n            सङ्कोचसूच्यादिकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति आरम्भः ।\n            सङ्कोचसूचीसीमाकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति सीमा ।\n            चरः क्षेत्रादिः ॱॱ अ६४ भवति क्षेत्रारम्भः पाठ्यम् आरम्भः सीमा ० ।\n            चरः क्षेत्रान्तः ॱॱ अ६४ भवति क्षेत्रसीमा पाठ्यम् आरम्भः सीमा ० ।\n            चरः पदान्तः ॱॱ अ६४ भवति अष्टकान्वेषणम् पाठ्यम् क्षेत्रादिः क्षेत्रान्तः ३२ ।\n            सङ्कोचसूचीपदादिकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति क्षेत्रादिः ।\n            सङ्कोचसूचीपदान्तकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति पदान्तः ।\n            सङ्कोचसूचीपूर्वदृष्टकोश अङ्कः सङ्कोचसूचीसंख्या अन्तः भवति ० ।\n            चरः पदम् ॱॱ अङ्कः अन्तः अ८ भवति पाठ्यम् अङ्कः क्षेत्रादिः अन्तः पदान्तः ।\n            चरः पूर्वक्रमः ॱॱ अ६४ भवति १ ।\n            यावत् पूर्वक्रमः न्यूनम् ० आदि",
    ),
    (
        "रूपयुग्मसाम्यम्",
        "the pair comparison starts at field ३, so two forms with the same \
         relation and different mnemonics collapse into one",
        "चरः क्रमः ॱॱ अ६४ भवति २ ।",
        "चरः क्रमः ॱॱ अ६४ भवति ३ ।",
    ),
];

#[test]
fn breaking_one_comparison_in_the_table_reader_chain_changes_the_answer() {
    // The same proof `breaking_one_comparison_in_the_register_chain_changes_
    // the_answer` gives for `कोष्ठाङ्कः`, for the two readers blocker (a)
    // closed on 2026-08-30. `mutate` fails if its pattern is not in the source
    // exactly once, so a mutation that matched nothing cannot pass by doing
    // nothing.
    let truth = {
        let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
        read_table_sample(&mut it)
    };
    assert!(
        truth
            .iter()
            .any(|(_, n)| matches!(n, Some(Some(v)) if *v != 0)),
        "the unmutated chain answered nothing but ०; there is no baseline to \
         differ from"
    );

    let vak = source("vakyavibhaga.t1");
    let mut proved = 0usize;
    // EVERY SURVIVOR, NOT THE FIRST (2026-09-14). An `assert_ne!` inside the
    // loop stops at row one, so a battery that has drifted off the path reports
    // one dead control per run and hides the rest — and this battery HAD
    // drifted: the table index landed a second set of comparators and the
    // readers go through those now. Collecting them names the whole population
    // in one run, which is the difference between "a row is stale" and "this
    // battery no longer measures the chain it is named for".
    let mut survived: Vec<&str> = Vec::new();
    for (routine, _what, from, to) in TABLE_READER_MUTATIONS {
        let broken = mutate(&source("encode.t1"), from, to);
        let mut it = load(&broken, &vak);
        let answers = read_table_sample(&mut it);
        if answers == truth {
            survived.push(routine);
        } else {
            proved += 1;
        }
    }
    assert!(
        survived.is_empty(),
        "MUTATIONS SURVIVED: {survived:?}. The table readers answered exactly \
         the same numbers with each of these comparisons changed, so what the \
         acceptance tests above measure is text and not behaviour — or the \
         reader no longer calls the routine the row names."
    );
    println!("METRIC sadhana_t1_table_reader_mutations_killed {proved}");
    assert_eq!(
        proved,
        TABLE_READER_MUTATIONS.len(),
        "every mutation must be killed"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `वाक्यविभाग ॱ वाक्यभेदनाम` — the T0 statement kinds, named.
// ─────────────────────────────────────────────────────────────────────────

/// **The blocker asked for a column the tree did not need.**
///
/// `वाक्यभेदनाम` was stubbed on "spec/parse-shape-t0.tsv gains a Devanagari
/// column, or the shape oracle compares `भेद` numbers instead of words". Both
/// halves are about the ORACLE'S comparison. `parse-shape-t0.tsv` is a
/// GENERATED golden of 9502 rows — one parse per line — so a column there
/// would be a per-row copy of a three-valued fact.
///
/// The three names were already `vakyavibhaga.t1`'s own, at `:116`–`:118`, and
/// `चिह्न` is attested as `label` in `spec/lexicon.tsv`. This asserts the
/// routine against THOSE CONSTANTS read out of the interpreter's globals, not
/// against a list written here — so if a constant is renumbered, this moves
/// with it instead of quietly disagreeing.
#[test]
fn the_statement_kinds_are_named_by_the_constants_this_file_declares() {
    let mut it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));

    // The numbers come from the corpus, never from this test.
    let kinds: Vec<(&str, &str)> = vec![
        ("आज्ञावाक्यभेद", "आज्ञा"),
        ("चिह्नवाक्यभेद", "चिह्न"),
        ("निर्देशवाक्यभेद", "निर्देश"),
    ];
    let mut seen = Vec::new();
    for (constant, want) in &kinds {
        let n = it
            .global(constant)
            .unwrap_or_else(|| panic!("{constant} is declared"))
            .as_int()
            .expect("a भेद constant is a number");
        let got = it
            .call("वाक्यविभागॱवाक्यभेदनाम", vec![Value::Int(n)], 5_000_000)
            .expect("वाक्यभेदनाम runs");
        let text = match got {
            Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
            other => panic!("{constant} named {other:?}, not octets"),
        };
        assert_eq!(&text, want, "{constant} (= {n}) named `{text}`");
        seen.push(n);
    }

    // The three must be DISTINCT numbers, or the test above is satisfied by a
    // routine that answers the same arm three times.
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 3, "the three भेद constants are not distinct");

    // A kind the file does not define names nothing — कारकनाम's convention.
    let unknown = it
        .call("वाक्यविभागॱवाक्यभेदनाम", vec![Value::Int(99)], 5_000_000)
        .expect("runs");
    match unknown {
        Value::Octets(o) => assert!(
            o.as_slice().is_empty(),
            "an undefined भेद named something instead of nothing"
        ),
        other => panic!("undefined भेद returned {other:?}"),
    }
}

/// The mutation: swap two arms and the constants stop agreeing with the names.
#[test]
fn swapping_two_statement_kind_names_is_caught() {
    let broken = mutate(
        &source("vakyavibhaga.t1"),
        "यदि भेद समम् चिह्नवाक्यभेद आदि प्रत्यागमनम् उक्तम् चिह्न इति । इति",
        "यदि भेद समम् चिह्नवाक्यभेद आदि प्रत्यागमनम् उक्तम् आज्ञा इति । इति",
    );
    let mut it = load(&source("encode.t1"), &broken);
    let n = it
        .global("चिह्नवाक्यभेद")
        .expect("declared")
        .as_int()
        .expect("a number");
    let got = it
        .call("वाक्यविभागॱवाक्यभेदनाम", vec![Value::Int(n)], 5_000_000)
        .expect("runs");
    match got {
        Value::Octets(o) => assert_eq!(
            String::from_utf8_lossy(o.as_slice()),
            "आज्ञा",
            "the mutation did not take, so the test above proves nothing"
        ),
        other => panic!("returned {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// `गणना` variants are values — the gap that blocked five encoder stubs.
// ─────────────────────────────────────────────────────────────────────────

/// **A `गणना` variant used to be unreachable.**
///
/// `read_head`'s `W_ENUM` arm skipped the whole block and registered nothing,
/// so `सङ्कुचितम्` answered *"is not a name in scope"* for a name the corpus
/// declares. Every `गणना` in the tree was equally unusable — `कोश ॱ संज्ञाखण्ड`,
/// `वास्तु ॱ स्थापन` — and five stubs in `encode.t1` branch on or pass one.
///
/// Found by an agent that tried to USE one, which is the only way it could be
/// found: nothing counts an enum, and a declaration nobody reads looks fine.
///
/// The values are asserted POSITIONALLY and read out of the interpreter, so a
/// variant inserted in the middle moves this test rather than silently
/// renumbering the corpus underneath it.
#[test]
fn a_gana_variant_is_a_value_and_its_number_is_its_position() {
    let it = load(&source("encode.t1"), &source("vakyavibhaga.t1"));
    // `कोश ॱ संज्ञाखण्ड` is declared in kosha.t1, which this pair does not load;
    // `वाक्यविभाग` declares none either. So use a module that does — the point
    // is the MECHANISM, and `t1_exec_link.rs` exercises the linker's own.
    let _ = it;

    let text = source("kosha.t1");
    let it2 =
        Interpreter::load(&[("kosha.t1", text.as_str())], &spec_root()).expect("kosha.t1 loads");

    // The six of `संज्ञाखण्ड`, in the order kosha.t1 writes them.
    let want: [(&str, i128); 6] = [
        ("पाठ्यम्", 0),
        ("दत्तम्", 1),
        ("शून्यक्षेत्रम्", 2),
        ("अनिर्दिष्टम्", 3),
        ("शोधनपङ्क्तिखण्डः", 4),
        ("पाठ्यखण्डः", 5),
    ];
    for (name, ordinal) in want {
        let v = it2
            .global(name)
            .unwrap_or_else(|| {
                panic!("`{name}` is not a name in scope — the W_ENUM arm registered nothing")
            })
            .as_int()
            .unwrap_or_else(|| panic!("`{name}` is not a number"));
        assert_eq!(v, ordinal, "`{name}` should be variant {ordinal}");
    }

    // ZERO-BASED, and that is the assertion that matters: the one-based
    // convention in this corpus belongs to the ARENAS, whose slot ० is never a
    // live entry. `Placement::Text` is 0 in Rust and must be 0 here.
    assert_eq!(
        it2.global("पाठ्यम्").and_then(Value::as_int),
        Some(0),
        "the first variant must be ०; a one-based enum would disagree with \
         every Rust discriminant it is ported from"
    );
}

/// The guard: variants must be DISTINCT, or the test above passes on a loader
/// that registers every one of them as the same number.
#[test]
fn the_variants_of_one_gana_are_distinct() {
    let text = source("kosha.t1");
    let it =
        Interpreter::load(&[("kosha.t1", text.as_str())], &spec_root()).expect("kosha.t1 loads");
    let names = [
        "पाठ्यम्",
        "दत्तम्",
        "शून्यक्षेत्रम्",
        "अनिर्दिष्टम्",
        "शोधनपङ्क्तिखण्डः",
        "पाठ्यखण्डः",
    ];
    let mut seen: Vec<i128> = names
        .iter()
        .filter_map(|n| it.global(n).and_then(Value::as_int))
        .collect();
    assert_eq!(seen.len(), names.len(), "a variant did not register");
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), names.len(), "two variants share a number");
}

/// `अभिव्यञ्जकनिर्णयः` refuses an undefined name — INCLUDING one nested inside
/// a binary operation.
///
/// The nesting is the point. That routine's own margin said "Numeral,
/// StringLiteral, Group and Index arms are pass-throughs; Identifier is the
/// only arm that acts" — five of the EIGHT kinds `ast.t1:29-36` declares. It
/// omits क्षेत्र, आह्वान and द्विकर्म, all of which carry child expressions.
/// A resolver written to that note answers TRUE for `क योगः अपरिचित`, because
/// it never descends into the operands — "a pass that validates nothing while
/// reporting success", which is exactly what the note was trying to prevent.
/// So the second case below is the one that would pass over the bug.
#[test]
fn the_resolver_refuses_an_undefined_name_even_inside_a_binary_operation() {
    let mut it = load_sema_with_parser();
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    it.call(
        "अर्थॱघोषणम्",
        vec![r.clone(), octets("क"), Value::Int(0)],
        5_000_000,
    )
    .expect("घोषणम् runs");

    // Build: identifier `क` (declared), identifier `अपरिचित` (not), and a
    // binary node whose right operand is the undeclared one.
    let t_known = tok(&mut it, "क");
    let t_unknown = tok(&mut it, "अपरिचित");
    let known = push_expr(&mut it, 1, t_known, 0, 0);
    let unknown = push_expr(&mut it, 1, t_unknown, 0, 0);
    let binary = push_expr(&mut it, 8, 0, known, unknown);

    let ok = resolve_expr(&mut it, &r, known);
    assert_eq!(
        ok,
        Some(true),
        "`क` is declared in the open scope and must resolve"
    );

    let bare = resolve_expr(&mut it, &r, unknown);
    assert_eq!(
        bare,
        Some(false),
        "`अपरिचित` is declared nowhere; a resolver that accepts it validates \
         nothing"
    );

    let nested = resolve_expr(&mut it, &r, binary);
    assert_eq!(
        nested,
        Some(false),
        "THE CASE THE FIVE-ARM VERSION WOULD PASS: the undefined name is an \
         OPERAND of a द्विकर्म node, so only a resolver that descends into \
         वामसूचकाङ्क and दक्षिणसूचकाङ्क can see it"
    );
}

/// `वाक्ययोजनम्` records each statement's subtree start, so a block's DIRECT
/// children can be walked without visiting a nested block's statements twice.
///
/// THE DOUBLE VISIT IS THE BUG THIS EXISTS TO PREVENT. Statements are numbered
/// post-order, so a nested block's statements lie inside its parent's range;
/// iterating the range resolves them twice, the second time in the enclosing
/// scope, and `घोषणम्` refuses a duplicate — reporting a duplicate name in a
/// program that has none. `अर्थ ॱ वाक्यनिर्णयः` is stubbed on exactly that.
///
/// Shape built here — `outer` is a block whose children are `a` and `inner`,
/// and `inner` is a block over `b` and `c`:
///
///     a  b  c  inner  outer          (post-order indices, 1-based)
///
/// So `outer`'s subtree starts at `a`, and walking from the top must yield
/// `inner` then `a` — never `b` or `c`, which belong to `inner`'s scope.
#[test]
fn a_statement_knows_where_its_own_subtree_begins() {
    let mut it = load_sema_with_parser();
    let a = push_stmt(&mut it, 1, 0, 0);
    let b = push_stmt(&mut it, 1, 0, 0);
    let c = push_stmt(&mut it, 1, 0, 0);
    let inner = push_stmt(&mut it, 2, b, c);
    let outer = push_stmt(&mut it, 2, a, inner);

    assert_eq!(subtree_start(&mut it, a), a, "a leaf begins at itself");
    assert_eq!(
        subtree_start(&mut it, inner),
        b,
        "`inner` spans b..inner, so it begins at b"
    );
    assert_eq!(
        subtree_start(&mut it, outer),
        a,
        "`outer` spans a..outer, so it begins at the FIRST statement of its \
         leftmost descendant — not at its own index"
    );

    // Walk outer's direct children from the top, skipping whole subtrees.
    let first = subtree_start(&mut it, outer);
    let mut seen = Vec::new();
    let mut k = outer - 1;
    while k >= first {
        seen.push(k);
        let s = subtree_start(&mut it, k);
        if s == 0 {
            break;
        }
        k = s - 1;
    }
    assert_eq!(
        seen,
        vec![inner, a],
        "the direct children of `outer`, top-down, are `inner` then `a`. If \
         `b` or `c` appear here the walk descended into a nested block and \
         would resolve those names twice"
    );
}

/// The `आदिसूचकाङ्क` of one statement.
fn subtree_start(it: &mut Interpreter, idx: i128) -> i128 {
    match arena_at_global(it, "वाक्यकोश", idx) {
        Value::Record(r) => r
            .borrow()
            .get("आदिसूचकाङ्क")
            .and_then(Value::as_int)
            .expect("a वाक्य carries आदिसूचकाङ्क"),
        other => panic!("वाक्यकोश[{idx}] is {other:?}, not a record"),
    }
}

fn arena_at_global(it: &Interpreter, name: &str, i: i128) -> Value {
    match it.global(name).expect("the arena is a name in scope") {
        Value::Arena(a) => a.borrow()[usize::try_from(i).expect("fits")].clone(),
        other => panic!("{other:?} is not an arena"),
    }
}

/// Push one statement — `व्याकरॱवाक्ययोजनम्`, whose arity is UNCHANGED by the
/// new field because the subtree start is computed, not passed.
fn push_stmt(it: &mut Interpreter, kind: i128, left: i128, right: i128) -> i128 {
    it.call(
        "व्याकरॱवाक्ययोजनम्",
        vec![Value::Int(kind), Value::Int(left), Value::Int(right)],
        20_000_000,
    )
    .unwrap_or_else(|e| panic!("वाक्ययोजनम् runs: {e:?}"))
    .as_int()
    .expect("an index")
}

/// `वाक्यनिर्णयः` over the three cases that distinguish a real pass.
///
/// WRITTEN AFTER A BUG THIS SUITE DID NOT CATCH. The first draft descended into
/// `वाम` and `दक्षिण` as STATEMENTS for every kind. That is right only for
/// `समूह`: `व्याकर`'s own `वाक्ययोजनम्` calls (parse.t1:291-372) put an
/// EXPRESSION in `वाम` for `अभिव्यञ्जक`, `प्रत्यागमन`, `यदि`, `यावत्` and
/// `सम`, and a TOKEN index there for `चर` and `आयात`. On a `चर` the draft would
/// have indexed `वाक्यकोश` with a token index — resolving an unrelated
/// statement. The whole suite was green over it, because nothing called it.
#[test]
fn the_statement_resolver_scopes_blocks_and_declares_bindings() {
    let mut it = load_sema_with_parser();
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");

    // (1) A `चर` DECLARES: `चरः क भवति <expr>` then `क` resolves.
    let t_k = tok(&mut it, "क");
    let lit = push_expr(&mut it, 2, 0, 0, 0); // a numeral — resolves vacuously
    let let_k = push_stmt(&mut it, 3, t_k, lit);
    assert_eq!(
        resolve_stmt(&mut it, &r, let_k),
        Some(true),
        "a `चर` whose value is a literal must resolve"
    );
    let use_k = push_expr(&mut it, 1, t_k, 0, 0);
    assert_eq!(
        resolve_expr(&mut it, &r, use_k),
        Some(true),
        "`चर` must have BOUND `क`; if this is false the declaration arm never \
         called घोषणम्"
    );

    // (2) An undefined name inside a `यदि` CONDITION is refused — the arm
    // where a resolver that stops at the statement would report success.
    let t_ghost = tok(&mut it, "अपरिचित");
    let ghost = push_expr(&mut it, 1, t_ghost, 0, 0);
    let empty_body = push_stmt(&mut it, 1, lit, 0);
    let if_stmt = push_stmt(&mut it, 6, ghost, empty_body);
    assert_eq!(
        resolve_stmt(&mut it, &r, if_stmt),
        Some(false),
        "`यदि अपरिचित …` names nothing declared and must be refused"
    );

    // (3) THE DOUBLE-VISIT CASE. A block containing a nested block, where the
    // SAME name is declared in both. Walking the parent's raw range would
    // reach the inner declaration twice and घोषणम् would refuse the duplicate
    // — reporting an error in a program that is perfectly legal, since the
    // inner `क` merely shadows.
    let t_k2 = tok(&mut it, "क");
    let inner_let = push_stmt(&mut it, 3, t_k2, lit);
    let inner_block = push_stmt(&mut it, 2, inner_let, inner_let);
    let outer_block = push_stmt(&mut it, 2, inner_let, inner_block);
    assert_eq!(
        resolve_stmt(&mut it, &r, outer_block),
        Some(true),
        "a nested block that shadows a name is LEGAL. A false here means the \
         walk visited the inner statement twice and घोषणम् refused it — the \
         invented duplicate this row was stubbed to avoid"
    );
}

/// `वाक्यनिर्णयः`, as `Option<bool>`; a run failure PANICS.
fn resolve_stmt(it: &mut Interpreter, r: &Value, idx: i128) -> Option<bool> {
    let v = it
        .call(
            "अर्थॱवाक्यनिर्णयः",
            vec![r.clone(), Value::Int(idx)],
            50_000_000,
        )
        .unwrap_or_else(|e| panic!("वाक्यनिर्णयः runs: {e:?}"));
    match v {
        Value::Bool(b) => Some(b),
        Value::Nil => None,
        other => panic!("वाक्यनिर्णयः must answer a बूल, answered {other:?}"),
    }
}

/// `कार्यक्रमनिर्णयः` over a REAL program, lexed and parsed by the T1 chain.
///
/// THIS ROUTINE WAS COMMITTED UNPROVEN and this is the test that was owed. It
/// is end-to-end on purpose: there is no public declaration appender, so the
/// only way to build `घोषणाकोश` is to lex and parse actual source.
///
/// THE FORWARD REFERENCE IS THE POINT. `क` calls `ख`, which is declared AFTER
/// it. That resolves only because the routine declares every top-level name in
/// a FIRST pass before resolving any body — Rust's shape. A resolver that
/// walked declarations once, resolving each body as it met it, would refuse
/// this program, and the language permits it.
///
/// It should also cross W-163: `व्याकर`'s declaration pushers put the FIRST
/// declaration in slot ०, while `ir.t1` reads one-based from १. If the first
/// declaration were skipped, `क` would be undeclared and its own body's call
/// would fail — so a pass here is evidence the walk sees BOTH.
#[test]
fn the_program_resolver_accepts_a_forward_reference_between_declarations() {
    let src = "वृत्तिः ख ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
               वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ख ।\nइति\n";
    let mut it = load_sema_with_parser();

    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    assert!(tokens > 0, "the source must lex to some tokens");

    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    // NOT `matches!(parsed, Value::Nil)` — that assertion was BLIND.
    // `कार्यक्रमपठनम्` returns `घोषणासूचकाङ्क`, a COUNT, and returns ० when
    // `दोषसूचकाङ्क` is non-zero (parse.t1:515-517). It never returns Nil, so
    // the old check passed on a source that failed to parse, and everything
    // below it was asserting against an EMPTY program.
    assert_eq!(
        parsed.as_int(),
        Some(2),
        "the source must parse to BOTH declarations; ० is कार्यक्रमपठनम्'s \
         parse-failure answer and would make every assertion below vacuous"
    );

    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    let resolved = it
        .call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed.clone()], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"));

    assert_eq!(
        resolved,
        Value::Bool(true),
        "`क` calls `ख`, declared after it. A false here means the two passes \
         collapsed into one — or that the first declaration was skipped, which \
         is W-163"
    );
}

/// A SHORTER PROGRAM AFTER A LONGER ONE MUST NOT SEE THE LONGER ONE'S NAMES.
///
/// W-223 part 2, 2026-09-04. THIS IS THE FAIL-FIRST TEST FOR THE EXTENT, and
/// it fails on the walk this row replaced.
///
/// THE LEAK. `व्याकरॱकार्यक्रमपठनम्` resets `घोषणासूचकाङ्क` to ० on entry
/// (parse.t1:1433), but NOTHING TRUNCATES `घोषणाकोश`: `घोषणायोजनम्` advances
/// the cursor and writes at it (parse.t1:1288), so the array only ever grows.
/// Parse a THREE-declaration program and then a ONE-declaration program into
/// the same interpreter and the array is still four long while the cursor says
/// one. `कार्यक्रमनिर्णयः` used to walk `० .. घोषणाकोश ॱ दैर्घ्य` — the ARRAY —
/// so it read the two stale entries belonging to the previous program and
/// DECLARED THEIR NAMES into this program's scope. It now walks
/// `१ .. कार्यक्रमः`, the extent its caller passes, which is the cursor.
///
/// WHY NOBODY HAD SEEN IT: every census and every test built a FRESH
/// interpreter per source, and one program per interpreter cannot have a stale
/// tail. `W-223`'s store is what stops us affording that — it is filled by
/// parsing all nineteen sources into ONE interpreter — so the leak becomes
/// reachable exactly when the store arrives.
///
/// `ख` IS DECLARED LAST IN THE FIRST PROGRAM ON PURPOSE. The second program's
/// own declaration overwrites slot १, so a name in slot १ would be gone; only
/// a name BEYOND the second program's cursor is still there to leak. Move `ख`
/// to the top of the first program and this test passes for the wrong reason.
#[test]
fn a_shorter_program_after_a_longer_one_does_not_see_the_longer_ones_names() {
    let mut it = load_sema_with_parser();

    // THREE declarations, `ख` LAST — so it lands in slot ३.
    let first = "वृत्तिः ग ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
                 वृत्तिः घ ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n\
                 वृत्तिः ख ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(first)], 50_000_000)
        .expect("पदविभाग runs")
        .as_int()
        .expect("a token count");
    let parsed_first = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 200_000_000)
        .expect("कार्यक्रमपठनम् runs")
        .as_int()
        .expect("a declaration count");
    assert_eq!(
        parsed_first, 3,
        "the first program must parse to THREE declarations, or there is no \
         stale tail for the second to leak from and this test proves nothing"
    );

    // ONE declaration, whose body names `ख` — declared only by the FIRST.
    let second = "वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ख ।\nइति\n";
    let toks2 = it
        .call("पदविभागॱपदविभाग", vec![octets(second)], 50_000_000)
        .expect("पदविभाग runs")
        .as_int()
        .expect("a token count");
    let parsed_second = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks2)], 200_000_000)
        .expect("कार्यक्रमपठनम् runs")
        .as_int()
        .expect("a declaration count");
    assert_eq!(
        parsed_second, 1,
        "the second program must parse to ONE declaration, so the cursor is १ \
         while the arena is still four long"
    );

    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    let resolved = it
        .call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r, Value::Int(parsed_second)],
            200_000_000,
        )
        .expect("कार्यक्रमनिर्णयः runs");

    assert_eq!(
        resolved,
        Value::Bool(false),
        "`ख` belongs to the PREVIOUS program and this one never declared it, \
         so resolving must REFUSE. A true here means the walk read past the \
         extent into the stale tail of `घोषणाकोश` and declared a name this \
         program never wrote — the cross-program leak this row closes"
    );
    assert_eq!(
        it.global("अनिर्णीतनाम").and_then(|v| match v {
            Value::Octets(o) => Some(String::from_utf8_lossy(o.as_slice()).into_owned()),
            _ => None,
        }),
        Some("ख".to_string()),
        "the refusal must NAME `ख`. A false answer that names something else \
         would mean this program was refused for an unrelated reason and the \
         leak is still open"
    );
}

/// AN EXTENT THAT CANNOT BE WALKED IS REFUSED BY NAME, NEVER CLAMPED.
///
/// W-223 part 2. Two kinds, and ० is deliberately not one of them: it is what
/// `कार्यक्रमपठनम्` answers for a program with no declarations, and walking
/// none of them is the right answer to it rather than an error.
#[test]
fn an_extent_outside_the_arena_is_refused_and_names_which_way_it_was_wrong() {
    let mut it = load_sema_with_parser();
    let src = "वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .expect("पदविभाग runs")
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 200_000_000)
        .expect("कार्यक्रमपठनम् runs")
        .as_int()
        .expect("a declaration count");

    for (extent, kind, what) in [
        (-1i128, 1i128, "a negative extent"),
        (parsed + 500, 2, "an extent past the arena's own दैर्घ्य"),
    ] {
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        let answer = it
            .call(
                "अर्थॱकार्यक्रमनिर्णयः",
                vec![r, Value::Int(extent)],
                200_000_000,
            )
            .expect("कार्यक्रमनिर्णयः runs");
        assert_eq!(
            answer,
            Value::Bool(false),
            "{what} must be REFUSED, not clamped to one that can be walked — \
             a clamped extent resolves some other program than the one asked \
             for and then reports success for it"
        );
        assert_eq!(
            it.global("परिधिदोषमस्ति"),
            Some(&Value::Bool(true)),
            "{what} must raise the extent flag"
        );
        assert_eq!(
            it.global("परिधिदोषभेद").and_then(Value::as_int),
            Some(kind),
            "{what} must say WHICH way it was wrong — a caller that cannot \
             tell a negative extent from one past the end cannot fix either"
        );
    }

    // ० IS NOT A REFUSAL. Pinned so nobody later "fixes" it into one.
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    let empty = it
        .call("अर्थॱकार्यक्रमनिर्णयः", vec![r, Value::Int(0)], 200_000_000)
        .expect("कार्यक्रमनिर्णयः runs");
    assert_eq!(
        empty,
        Value::Bool(true),
        "an extent of ० is what कार्यक्रमपठनम् answers for a program with no \
         declarations; walking none of them succeeds vacuously and is not an \
         error"
    );
    assert_eq!(
        it.global("परिधिदोषमस्ति"),
        Some(&Value::Bool(false)),
        "and ० must not raise the extent flag"
    );
}

/// Collect the named sources into one interpreter and raise the flag.
///
/// W-223 part 2. Synthetic twin of `load_sema_collected`, for tests that need a
/// store holding EXACTLY what they say it holds — the corpus loader collects
/// nineteen real modules and cannot express "this module was never collected".
fn collect_sources(srcs: &[(&str, &str)], raise: bool) -> Interpreter {
    let mut it = load_sema_with_parser();
    for (_, src) in srcs {
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 200_000_000)
            .expect("parses");
        it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 200_000_000)
            .expect("collects");
    }
    if raise {
        it.call("अर्थॱसञ्चयसिद्धिः", vec![], 5_000_000)
            .expect("सञ्चयसिद्धिः runs");
    }
    it
}

/// Resolve one source in an interpreter, answering the verdict.
fn resolve_in(it: &mut Interpreter, src: &str) -> Result<Value, sadhana::t1::nirvahana::RunError> {
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 200_000_000)
        .expect("parses");
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver starts");
    it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
}

/// A MEMBER THE NAMED MODULE DOES NOT DECLARE IS REFUSED, BY NAME.
///
/// W-223 part 2's first fail-first: the refusal this row was written to buy.
/// Before it, `अर्थ` verified the IMPORT and took the member on trust — its own
/// margin said "the member is taken on trust" — so a use naming a member the
/// module does not have resolved happily and `ir.t1` met it as a callee with no
/// symbol. The store makes the member checkable; this asserts it is checked.
#[test]
fn a_member_the_module_does_not_declare_is_refused_and_names_it() {
    let provider = "मण्डलम् दाता ॥\nसार्वजनिक वृत्तिः अस्ति ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    let mut it = collect_sources(&[("दाता", provider)], true);

    // `नास्ति` is NOT declared by `दाता`, which the store HOLDS — so this is
    // refusal kind २ and not "nobody collected that module".
    let user = "मण्डलम् ग्राहक ॥\nआयातः दाता ।\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    प्रत्यागमनम् दाता ॱ नास्ति ।\nइति\n";
    let verdict = resolve_in(&mut it, user).expect("कार्यक्रमनिर्णयः runs");

    assert_eq!(
        verdict,
        Value::Bool(false),
        "`दाता` is collected and declares no `नास्ति`, so the use must be REFUSED. \
         A true here means the member is still taken on trust and the store is \
         being consulted for nothing"
    );
    assert_eq!(
        it.global("असदस्यमस्ति"),
        Some(&Value::Bool(true)),
        "the refusal must be the MEMBER refusal and not some other one"
    );
    assert_eq!(
        text_global(&it, "असदस्यनाम"),
        "नास्ति",
        "and it must NAME the member; a refusal that does not say which member \
         cannot be acted on"
    );
    assert_eq!(
        text_global(&it, "असदस्यमण्डल"),
        "दाता",
        "and the module it looked in"
    );
}

/// A USE OF A MODULE NOBODY COLLECTED IS TRUSTED, AND COUNTED.
///
/// W-223 part 2's second fail-first, and the one that keeps the first honest.
/// Authority is PER MODULE: the store answers ० with two meanings, and merging
/// them would refuse every use naming a module the collection happened not to
/// reach — a test that collects three modules and names a fourth is RIGHT. The
/// count is what stops "not collected" from passing as "collected and clean".
#[test]
fn a_use_of_an_uncollected_module_is_trusted_and_counted_not_refused() {
    let provider = "मण्डलम् दाता ॥\nसार्वजनिक वृत्तिः अस्ति ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    let mut it = collect_sources(&[("दाता", provider)], true);

    // `अन्य` was never collected. Its member cannot be checked, so it is taken
    // on trust exactly as every qualified use was before this row.
    let user = "मण्डलम् ग्राहक ॥\nआयातः अन्य ।\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    प्रत्यागमनम् अन्य ॱ किञ्चित् ।\nइति\n";
    let verdict = resolve_in(&mut it, user).expect("कार्यक्रमनिर्णयः runs");

    assert_eq!(
        verdict,
        Value::Bool(true),
        "`अन्य` is not in the store at all, so its member is TRUSTED — refusing \
         it would make the store's authority per-STORE instead of per-module"
    );
    assert_eq!(
        it.global("असदस्यमस्ति"),
        Some(&Value::Bool(false)),
        "and it must not be recorded as a member refusal"
    );
    assert_eq!(
        it.global("असङ्गृहीतमण्डलसंख्या").and_then(Value::as_int),
        Some(1),
        "but it MUST be counted: a trust nobody counts is indistinguishable \
         from a check that passed"
    );
}

/// WITH NO COLLECTION AT ALL, EVERY QUALIFIED USE IS TRUSTED — AND COUNTED
/// SEPARATELY.
///
/// W-223 part 2's third fail-first, for the flag itself. An image that never
/// collected must not refuse, or every test in this crate that resolves a
/// qualified name without filling the store would go red; and it must not be
/// silent, or a loader nobody gave a collection to reads as a clean measurement.
/// The two trust paths are counted apart because "nobody ran the instrument" and
/// "the instrument ran and did not reach this module" are different facts.
#[test]
fn without_a_collection_pass_a_qualified_use_is_trusted_on_its_own_counter() {
    let provider = "मण्डलम् दाता ॥\nसार्वजनिक वृत्तिः अस्ति ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    // Collected, but the flag is NOT raised: the store is full and inert.
    let mut it = collect_sources(&[("दाता", provider)], false);

    let user = "मण्डलम् ग्राहक ॥\nआयातः दाता ।\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    प्रत्यागमनम् दाता ॱ नास्ति ।\nइति\n";
    let verdict = resolve_in(&mut it, user).expect("कार्यक्रमनिर्णयः runs");

    assert_eq!(
        verdict,
        Value::Bool(true),
        "with the flag down the store is never consulted, so even a member the \
         collected module does NOT declare is taken on trust — the flag is what \
         makes collection an asserted fact rather than an inferred one"
    );
    assert_eq!(
        it.global("असञ्चितप्रयोगसंख्या").and_then(Value::as_int),
        Some(1),
        "and it is counted on the no-collection counter, NOT the uncollected-\
         module one — merging them would hide which of the two happened"
    );
    assert_eq!(
        it.global("असङ्गृहीतमण्डलसंख्या").and_then(Value::as_int),
        Some(0),
        "the other trust counter must stay ०"
    );
}

/// A SECOND PROGRAM MUST NOT READ THE FIRST PROGRAM'S SYMBOL TYPES.
///
/// THIS PINS THE INVARIANT AND DOES NOT PROVE A FIX. It passes with the
/// clearing and WITHOUT it, so it is not evidence for the clearing — an
/// invariant restored is not a bug fixed. It is kept because the invariant is
/// worth pinning and because the next person to attempt the demonstration
/// deserves the two failures below rather than a clean slate.
///
/// TWO DESIGNS FAILED TO DEMONSTRATE THE LEAK. The first used `न६४` in both
/// programs, so the stale slot happened to hold the type the second program
/// wanted and the test could not tell the cases apart — vacuous for the reason
/// it was written. The second, below, makes the programs DISAGREE at the same
/// symbol index (`बूल` written where `न६४` is wanted) and still passes, because
/// `घोषणम्` writes `संज्ञाप्रकारकोश[संज्ञा+१]` for EVERY declaration: a second
/// program overwrites each slot as it declares its own names, and a name that
/// RESOLVES was declared, so no read-before-write path exists to expose the
/// stale value. If you find one, this test is where it belongs.
///
/// W-223 part 2's fourth test, and the one I would not have thought to write
/// before this row. `संज्ञाप्रकारकोश`, `संज्ञाभेदकोश` and `संज्ञाघोषणाकोश` are
/// GLOBAL and keyed by SymbolId+१, while `निर्णायकारम्भः` resets `अग्रिमसंज्ञा` to
/// ० — so a second program's symbol १ indexes the slot the FIRST program's
/// symbol १ wrote. Nothing cleared them because nothing needed to: one
/// interpreter per program cannot collide, and that is what every census did
/// until the store made one interpreter hold nineteen programs.
///
/// THE SHAPE IS THE EXTENT LEAK'S: a per-program numbering restarts while the
/// storage it indexes does not, invisible until one interpreter must hold two
/// programs. Filed as the signature of `W-249`.
#[test]
fn a_second_program_does_not_read_the_first_programs_symbol_types() {
    let mut it = load_sema_with_parser();

    // THE TWO PROGRAMS MUST DISAGREE AT THE SAME SYMBOL INDEX, or the test is
    // vacuous — and the first draft of it WAS. Both programs used `न६४`, so the
    // stale slot held the type the second program wanted anyway and the test
    // passed with the clearing REMOVED. It proved nothing until the types
    // differed. Here symbol १ is the routine and symbol २ the local in each, so
    // the second program's `घ` (न६४) indexes the slot the first program's `ख`
    // (बूल) wrote.
    let first = "मण्डलम् प्रथम ॥\nसार्वजनिक वृत्तिः क ददाति बूल आदि\n    चरः ख ॱॱ बूल भवति सत्यम् ।\n    प्रत्यागमनम् ख ।\nइति\n";
    assert_eq!(
        resolve_in(&mut it, first).expect("runs"),
        Value::Bool(true),
        "the first program must resolve or this test proves nothing"
    );

    // SECOND, in the SAME interpreter: a routine whose body ends in a declared
    // name. Its symbols restart at १, so without the clear it reads the first
    // program's recorded types.
    let second =
        "मण्डलम् द्वितीय ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः घ ॱॱ न६४ भवति ७ ।\n    घ ।\nइति\n";
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(second)], 50_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 200_000_000)
        .expect("parses");
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver starts");
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r.clone(), parsed.clone()],
            200_000_000
        )
        .expect("runs"),
        Value::Bool(true),
        "the second program must resolve"
    );
    it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000)
        .expect("checker starts");
    let verdict = it
        .call("अर्थॱकार्यक्रमप्रकारपरीक्षा", vec![parsed], 200_000_000)
        .expect("typecheck runs");

    assert_eq!(
        verdict,
        Value::Bool(true),
        "`घ` is declared `न६४` and `ग` returns `न६४`, so this must typecheck. A \
         false here means the checker read `घ`'s type out of the slot the FIRST \
         program's `ख` wrote, which is `बूल` — the cross-program leak the three \
         symbol-keyed arenas had until `निर्णायकारम्भः` began clearing them"
    );
}

/// A CHECKER STATE THAT PERSISTS BETWEEN PROGRAMS DOES NOT MOVE A VERDICT.
///
/// W-253, 2026-09-04. THIS PINS AN INVARIANT AND DOES NOT PROVE A FIX — it
/// passes with the per-source `प्रकारपरीक्षकारम्भः` and without it.
///
/// I PREDICTED THE OPPOSITE AND WAS WRONG, and the prediction is recorded
/// because it is the kind that sounds right. The chain census resets the
/// resolver and the IR builder per source but never the checker, which is
/// harmless only while a fresh interpreter per source does the zeroing — so
/// sharing one interpreter looked as though it would carry `पुच्छे`, the
/// TAIL-POSITION flag the join READS, into the next program and turn an accept
/// into a refusal with no number moving anywhere.
///
/// IT DOES NOT — but THE REASON GIVEN HERE WAS WRONG, and is re-founded below.
///
/// SUPERSEDED 2026-09-05 (`W-270`, `W-275`), KEPT BECAUSE THIS MARGIN ASKED TO BE
/// CONTRADICTED AND EARNED IT: "the block fold sets `पुच्छे भवति सत्यम् ।`
/// immediately BEFORE typing its last child, so a completed typecheck ENDS true —
/// the value `प्रकारपरीक्षकारम्भः` would have written". MEASURED FALSE. This test
/// drives only sources the checker ACCEPTS; three sources measured:
///     accepted (several statements)      verdict true    पुच्छे TRUE
///     refused, body/return disagree      verdict false   पुच्छे TRUE
///     refused, non-बूल `यदि` condition   verdict false   पुच्छे FALSE
/// So it is false for SOME refusals, not all: the refusal has to land INSIDE the
/// block fold, after `पुच्छे भवति असत्यम् ।` and before the `भवति सत्यम् ।` that
/// precedes the last child. The assertion below is still true OF ITS OWN
/// FIXTURES, which is exactly why it never caught this.
///
/// WHAT SURVIVES, and it is narrower: NO RULE READS THE INCOMING VALUE. `पुच्छे`
/// has one reader, `वाक्यप्रकारः`'s `यदि` arm, and every block arm writes the flag
/// immediately before typing a child — so an inherited value is overwritten before
/// anything consults it. That is why no verdict moves, and it holds for refused
/// sources too, where the old reason does not.
///
/// The other persisting pieces (`दुष्टवाक्यमस्ति`, `दुष्टकारण`) are
/// first-only DIAGNOSTICS: carrying them corrupts the recorded SITE of a later
/// poison, not the verdict. Measured here, and measured on the corpus too — the
/// typecheck census read 19 of 19 with 0 refused both before and after its own
/// reset was added; only its COUNTS were wrong (`unjoined_branches` 337 against
/// a true 53).
///
/// SO THE RESET IS DEMONSTRATED WHERE COUNTS ARE READ AND DEFENSIVE WHERE THEY
/// ARE NOT. In `t1_execution.rs`'s typecheck census it fixes a real, measured
/// over-count. In `paradigm_encode.rs`, which reads no summed counter, it
/// prevents nothing anyone can exhibit today and is kept because a flag a rule
/// reads must not outlive the program it describes.
#[test]
fn a_checker_state_that_persists_does_not_move_a_later_programs_verdict() {
    // A: a body with several statements, so the tail flag is driven both ways.
    let a = "मण्डलम् अ ॥\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    चरः ख ॱॱ न६४ भवति १ ।\n    चरः ग ॱॱ न६४ भवति २ ।\n    ग ।\nइति\n";
    // B: a one-statement body whose value IS the tail.
    let b = "मण्डलम् आ ॥\nसार्वजनिक वृत्तिः घ ददाति न६४ आदि\n    चरः ङ ॱॱ न६४ भवति ३ ।\n    ङ ।\nइति\n";

    let check = |it: &mut Interpreter, src: &str, reset: bool| -> Value {
        let t = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap()
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(t)], 200_000_000)
            .unwrap();
        let r = it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000).unwrap();
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r.clone(), parsed.clone()],
            200_000_000,
        )
        .unwrap();
        if reset {
            it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000).unwrap();
        }
        it.call("अर्थॱकार्यक्रमप्रकारपरीक्षा", vec![parsed], 200_000_000)
            .unwrap()
    };

    // B alone, with a fresh interpreter and a reset — the reference verdict.
    let mut fresh = load_sema_with_parser();
    let alone = check(&mut fresh, b, true);

    // A then B in ONE interpreter, with NO reset between them.
    let mut shared = load_sema_with_parser();
    let _ = check(&mut shared, a, true);
    let after_a = check(&mut shared, b, false);

    assert_eq!(
        shared.global("पुच्छे"),
        Some(&Value::Bool(true)),
        "a completed typecheck must leave the tail flag TRUE FOR THE ACCEPTING \
         SOURCES THIS TEST DRIVES. Narrowed 2026-09-05: it does NOT hold in \
         general — a refusal inside the block fold ends FALSE (W-275) — so this \
         is a fixture-level fact and NOT the reason carrying the flag is \
         harmless. That reason is that no rule reads the incoming value"
    );
    assert_eq!(
        alone, after_a,
        "the verdict must not depend on what was typechecked before it in the \
         same interpreter. A failure here means persisting checker state DOES \
         decide a verdict, which would make the per-source reset a fix rather \
         than a guard — and would need saying on W-253 in those words"
    );
}

/// A REFUSAL INSIDE THE BLOCK FOLD LEAVES `पुच्छे` FALSE — AND THE NEXT
/// PROGRAM'S VERDICT STILL DOES NOT MOVE.
///
/// `W-275`, 2026-09-05. THE TEST ABOVE ASKED TO BE CONTRADICTED AND THIS IS THE
/// CONTRADICTION, pinned rather than described. Its superseded sentence said a
/// completed typecheck ENDS true, and named what would follow if the flag ever
/// read false: the reasoning is void and the reset becomes load-bearing. THE
/// FLAG DOES READ FALSE. The reset is still not load-bearing — but for a
/// different reason than the one recorded, and a reason that is measured here
/// rather than reasoned, because reasoning is what put the false sentence on
/// main in the first place.
///
/// AND IT IS NOT "FALSE FOR A REFUSED SOURCE", which is the same
/// over-generalisation one qualifier short. Measured, three sources:
///     accepted, several statements        verdict true    `पुच्छे` TRUE
///     refused, body/return disagree       verdict false   `पुच्छे` TRUE
///     refused, non-`बूल` `यदि` condition  verdict false   `पुच्छे` FALSE
/// The refusal has to land INSIDE THE BLOCK FOLD — after `पुच्छे भवति असत्यम् ।`
/// and before the `भवति सत्यम् ।` that precedes the last child. A refusal
/// elsewhere still ends true, which is why the test above never caught this
/// with the accepting sources it drives.
///
/// WHAT THIS PINS is the surviving claim: no rule reads the incoming value.
/// The refusal runs FIRST so the flag arrives false, and the second source is
/// checked with NO reset against a fresh-and-reset reference — for a plain body
/// AND for one containing `यदि`/`अन्यथा`. The second is not redundant: `पुच्छे`
/// has exactly one reader, `वाक्यप्रकारः`'s `यदि` arm, so a reference using only
/// a plain body would never exercise the reader at all.
#[test]
fn a_refusal_inside_the_fold_leaves_the_tail_flag_false_and_moves_no_later_verdict() {
    // Refused, and refused INSIDE the fold: `झ` is `न६४`, so the `यदि`
    // condition is not `बूल`. The statement before it has already driven
    // `पुच्छे` false, and the refusal returns before the tail is reached.
    let refused = "मण्डलम् इ ॥\nसार्वजनिक वृत्तिः ज ददाति न६४ आदि\n    चरः झ ॱॱ न६४ भवति १ ।\n    यदि झ आदि\n        प्रत्यागमनम् १ ।\n    इति\n    झ ।\nइति\n";
    // Two accepted seconds: a plain body, and one whose body contains the
    // `यदि`/`अन्यथा` that READS the flag.
    let plain = "मण्डलम् अ ॥\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    चरः ख ॱॱ न६४ भवति १ ।\n    चरः ग ॱॱ न६४ भवति २ ।\n    ग ।\nइति\n";
    let with_if = "मण्डलम् ई ॥\nसार्वजनिक वृत्तिः ट ददाति न६४ आदि\n    चरः ठ ॱॱ बूल भवति सत्यम् ।\n    यदि ठ आदि\n        प्रत्यागमनम् १ ।\n    इति अन्यथा आदि\n        प्रत्यागमनम् २ ।\n    इति\nइति\n";

    let run = |it: &mut Interpreter, src: &str, reset: bool| -> Value {
        let t = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap()
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(t)], 200_000_000)
            .unwrap();
        let r = it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000).unwrap();
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r.clone(), parsed.clone()],
            200_000_000,
        )
        .unwrap();
        if reset {
            it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000).unwrap();
        }
        it.call("अर्थॱकार्यक्रमप्रकारपरीक्षा", vec![parsed], 200_000_000)
            .unwrap()
    };

    // THE FIXTURE'S OWN TWO FACTS, ASSERTED BEFORE ANYTHING IS CONCLUDED FROM
    // THEM. If the fixture stops refusing, or stops leaving the flag false,
    // everything below would still pass while measuring the case the test above
    // already covers — a green test quietly changing what it is about.
    let mut it = load_sema_with_parser();
    let verdict = run(&mut it, refused, true);
    assert_eq!(
        verdict,
        Value::Bool(false),
        "the fixture must be REFUSED, or this is not the case this test is about"
    );
    assert_eq!(
        it.global("पुच्छे"),
        Some(&Value::Bool(false)),
        "a refusal inside the block fold must leave the tail flag FALSE — this is \
         what contradicts the superseded `a completed typecheck ends true`. If it \
         reads true, either the fold changed or the fixture stopped refusing where \
         it used to, and this test is no longer exercising the gap"
    );

    for (label, second) in [("plain body", plain), ("body with यदि/अन्यथा", with_if)]
    {
        let mut fresh = load_sema_with_parser();
        let reference = run(&mut fresh, second, true);

        let mut shared = load_sema_with_parser();
        let _ = run(&mut shared, refused, true);
        let after = run(&mut shared, second, false); // NO reset — the case at issue
        assert_eq!(
            reference, after,
            "{label}: a FALSE tail flag carried in from a refused program moved the \
             next program's verdict. That would make the per-source reset a FIX and \
             not a guard, and `paradigm_encode.rs`'s margin — which says the reset is \
             defensive because no rule reads the incoming value — would be false and \
             would need saying there in those words"
        );
    }
}

/// THE TWO MINTERS MUST LEAVE THE THREE SYMBOL ARENAS DENSE.
///
/// `संज्ञाग्रहणम्` mints a SymbolId and has two callers. `घोषणम्` writes all three
/// arenas keyed by that id — type, kind, declaration.
///
/// SUPERSEDED 2026-09-05, AND THIS TEST WENT ON PASSING THROUGHOUT — which is the
/// reason to date it rather than delete it. The margin read, in the present tense:
/// "`सञ्चयसंज्ञा`, which interns a cross-module member from the store, writes only
/// `संज्ञाप्रकारकोश`. So every interned member advances the counter and fills one
/// arena of three, the next `घोषणम्` writes at a higher index, and
/// `nirvahana.rs:1364` pads the skipped slots with `Value::Nil` — holes INSIDE the
/// length, which a length guard passes." THAT DEFECT WAS FIXED: `सञ्चयसंज्ञा` now
/// writes all three at `artha.t1:2147-2149`, taking the kind from
/// `घोषणासञ्चयॱप्रविष्टिभेदः` and the declaration honestly `०` — a member of another
/// module HAS no declaration in this program's arena, and `०` is that arena's own
/// sentinel for "none", written deliberately so the slot is filled rather than
/// left for the padder.
///
/// SO THE TEST NOW PASSES BECAUSE THE PROPERTY HOLDS, not because it once failed
/// to catch a hole. A margin describing a live defect, above a green test, is
/// invisible to every gate here — it reads as MORE authoritative because the pass
/// appears to vouch for it. Found 2026-09-05 while reading for `W-275`, one file
/// over from the same failure. What the four unsentinelled reads in `artha.t1`
/// (807, 909, 1692, 2223) depend on is exactly this density, which is why
/// `t1_arena_density.rs` (`W-271`) now pins it as a SET EQUALITY between minters.
#[test]
fn both_minters_leave_every_symbol_arena_dense() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    let mut it = load_sema_collected(&dir, &names);
    let src = std::fs::read_to_string(dir.join("encode.t1")).unwrap();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        .unwrap()
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .unwrap();
    let r = it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000).unwrap();
    it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 4_000_000_000)
        .unwrap();

    let holes = |it: &Interpreter, name: &str| -> (usize, usize) {
        match it.global(name) {
            Some(Value::Arena(a)) => {
                let a = a.borrow();
                (
                    a.len(),
                    a.iter().filter(|v| matches!(v, Value::Nil)).count(),
                )
            }
            _ => (0, 0),
        }
    };
    for n in ["संज्ञाप्रकारकोश", "संज्ञाभेदकोश", "संज्ञाघोषणाकोश"]
    {
        let (len, nil) = holes(&it, n);
        let idx: Vec<usize> = match it.global(n) {
            Some(Value::Arena(a)) => a
                .borrow()
                .iter()
                .enumerate()
                .filter(|(_, v)| matches!(v, Value::Nil))
                .map(|(i, _)| i)
                .take(8)
                .collect(),
            _ => Vec::new(),
        };
        eprintln!("ARENA {n:<20} len {len:>5}  Nil holes {nil:>5}  at {idx:?}");
    }
    // SLOTS ० AND १ ARE RESERVED AND ARE NOT HOLES. Every arena in this corpus
    // keys from १ and never writes ०, so `संज्ञाप्रकारकोश` — which BOTH minters
    // write — reads one `Nil` at index ०. The invariant is therefore "no hole
    // ABOVE the reserved slots", and asserting zero holes outright would demand
    // something no arena here satisfies.
    //
    // ONE RESERVED SLOT BECAME TWO ON 2026-09-13, and `c107c725` says so in its
    // own message rather than leaving it to be rediscovered: "The tree and every
    // table index by id + १ as before; slot १ is now unused." Ids mint from १
    // since that landing — natively an optional is one word and `Some(०)` was
    // indistinguishable from the nil word, so zero had to stop being a valid id
    // — while the arenas still key at `संज्ञा योगः १`, which is what keeps ० free
    // for "absent". The two offsets now compose and index १ is written by
    // nothing. That is a WASTED SLOT, not a missing write, and the difference is
    // the whole point of this assertion: read it as a hole and the conclusion is
    // that a minter skipped an arena, which is what the message below says and
    // what was measured and repaired under W-245. It is not what is happening
    // here. The landing left this pin un-re-taken; this is the re-take. Measured before this was written: the
    // first draft asserted ० and would have failed forever.
    for n in ["संज्ञाप्रकारकोश", "संज्ञाभेदकोश", "संज्ञाघोषणाकोश"]
    {
        let above: Vec<usize> = match it.global(n) {
            Some(Value::Arena(a)) => a
                .borrow()
                .iter()
                .enumerate()
                .skip(2)
                .filter(|(_, v)| matches!(v, Value::Nil))
                .map(|(i, _)| i)
                .collect(),
            _ => Vec::new(),
        };
        assert!(
            above.is_empty(),
            "`{n}` holds {} holes above the two reserved slots, at {:?}. A symbol \
             was minted without writing this arena: `संज्ञाग्रहणम्` has two callers \
             and only `घोषणम्` writes all three.",
            above.len(),
            &above[..above.len().min(8)]
        );
    }
}

/// **W-265.** A NAMED TYPE THE PROGRAM DECLARES CARRIES ITS SYMBOL, and one it
/// does not is refused BY NAME WITH A REASON.
///
/// # The audit that moved this repair
///
/// The row this test comes from said the AST path "has one primitive arm and
/// four wrapper arms and then returns `दोषार्थः`", which reads as a named type
/// falling off the bottom of `प्रकारार्थः`. IT NEVER REACHES THE BOTTOM.
/// `parse.t1:378` says `अन्यत् सर्वं मूलप्रकारः` and `वास्तु` has no named-type
/// node kind at all (`ast.t1:39-43` lists five and none of them is one), so a
/// named type IS a `मूलप्रकारभेद` node carrying a name — Rust's own
/// `Type::Primitive(String)`. It entered the PRIMITIVE arm, `मूलप्रकारार्थः`
/// had no spelling for it, and that poison was returned unexamined. The site to
/// repair was the arm, not the fall-through, and this test is written on the
/// arm.
///
/// # The struct is declared AFTER the routine that returns it, deliberately
///
/// A named type's symbol is minted by pass one of `कार्यक्रमनिर्णयः`, so
/// reading a return type in the same turn that binds the names would make the
/// answer depend on DECLARATION ORDER. Writing the struct first would pass
/// against a one-pass resolver and prove nothing; writing it last fails unless
/// the pass really was split.
///
/// # Both halves, because a resolver that resolves everything is not one
///
/// The second half asks for a name NOTHING declares and requires poison AND a
/// recorded reason. Without it this test would pass just as happily against a
/// `नामप्रकारार्थः` that answered `संरचनार्थभेद` for every spelling it was
/// handed, which is the shape of vacuous guard this suite has been bitten by.
#[test]
fn a_named_type_resolves_to_its_declaration_symbol_and_an_unknown_one_is_refused() {
    let src = "वृत्तिः ख ददाति पेटिका आदि\n    प्रत्यागमनम् ० ।\nइति\n\n\
               सार्वजनिक संरचना पेटिका आरभ्य\n  क ॱॱ अ३२\nसमाप्तम् ।\n";
    let mut it = load_sema_with_parser();

    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .expect("पदविभाग runs")
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .expect("कार्यक्रमपठनम् runs")
        .as_int()
        .expect("a declaration count");
    assert_eq!(
        parsed, 2,
        "the source must parse to the routine and the struct; ० is \
         कार्यक्रमपठनम्'s parse-failure answer and would make the rest vacuous"
    );

    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 50_000_000)
        .expect("निर्णायकारम्भः runs");
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r, Value::Int(parsed)],
            500_000_000
        )
        .expect("कार्यक्रमनिर्णयः runs"),
        Value::Bool(true),
        "the program must resolve, or `पेटिका` has no symbol for reasons that \
         have nothing to do with this row"
    );

    // The ROUTINE is declaration १ and its `प्रकारसूचकाङ्क` is the AST node for
    // `पेटिका` — the same index `कार्यक्रमप्रकारपरीक्षा` reads for a declared
    // return.
    let decl = arena_at_global(&it, "घोषणाकोश", 1);
    let ty_idx = match &decl {
        Value::Record(d) => d.borrow().get("प्रकारसूचकाङ्क").and_then(Value::as_int),
        other => panic!("{other:?} is not a घोषणा"),
    }
    .expect("the routine records a return type index");
    assert!(
        ty_idx > 0,
        "the parser must have recorded `ख`'s return type"
    );

    let nodes = it
        .global("अभिव्यञ्जककोश")
        .expect("the expression arena is a name in scope")
        .clone();
    let ty = it
        .call("अर्थॱप्रकारार्थः", vec![nodes, Value::Int(ty_idx)], 50_000_000)
        .expect("प्रकारार्थः runs");
    let field = |v: &Value, k: &str| match v {
        Value::Record(rec) => rec.borrow().get(k).and_then(Value::as_int),
        _ => None,
    };
    assert_eq!(
        field(&ty, "भेद"),
        Some(7),
        "`पेटिका` must type as संरचनार्थभेद. A १२ here is दोषार्थः — the AST \
         path still answering poison for a name it can see declared"
    );
    let sym = field(&ty, "संज्ञा").expect("the type carries a संज्ञा field");
    assert!(
        sym > 0,
        "`अर्थप्रकार ॱ संज्ञा` is still ०; the kind was set and the SYMBOL was \
         not, which is the half of this row that matters"
    );

    // AND THE SYMBOL IS THE STRUCT'S, not a number that merely is not zero. A
    // non-zero assertion passes against a counter; this follows the symbol back
    // through `संज्ञाघोषणाकोश` to a declaration and reads its name.
    let decl_of_sym = arena_at_global(&it, "संज्ञाघोषणाकोश", sym)
        .as_int()
        .expect("the symbol leads to a declaration index");
    assert!(
        decl_of_sym > 0,
        "symbol {sym} came from no declaration — the table holds a number that \
         names nothing"
    );
    let struct_decl = arena_at_global(&it, "घोषणाकोश", decl_of_sym);
    let name_tok = match &struct_decl {
        Value::Record(d) => d.borrow().get("नामसूचकाङ्क").and_then(Value::as_int),
        other => panic!("{other:?} is not a घोषणा"),
    }
    .expect("the declaration names a token");
    let name = match it.call(
        "पदविभागॱचिह्नकपाठः",
        vec![arena_at_global(&it, "चिह्नककोश", name_tok)],
        20_000_000,
    ) {
        Ok(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("the name is a run of octets, not {other:?}"),
    };
    assert_eq!(
        name, "पेटिका",
        "the symbol must lead back to the STRUCT's own declaration"
    );

    // THE OTHER HALF. A spelling nothing declares stays poison, and the refusal
    // says WHICH name and WHY — reason १, `अनामप्रकारकारण`.
    let before = it
        .global("अज्ञातप्रकारपाठसंख्या")
        .and_then(Value::as_int)
        .expect("the refusal counter");
    let unknown = it
        .call(
            "अर्थॱनामप्रकारार्थः",
            vec![octets("मञ्जूषा"), Value::Int(0)],
            20_000_000,
        )
        .expect("नामप्रकारार्थः runs");
    assert_eq!(
        field(&unknown, "भेद"),
        Some(12),
        "a name nothing declares must be दोषार्थः, not a guessed struct"
    );
    assert_eq!(
        it.global("अज्ञातप्रकारपाठसंख्या").and_then(Value::as_int),
        Some(before + 1),
        "the refusal was counted"
    );
    assert_eq!(
        it.global("अज्ञातप्रकारकारण").and_then(Value::as_int),
        Some(1),
        "and it was refused for अनामप्रकारकारण — bare, and no module binds it — \
         not for an unnamed 'unknown kind'"
    );
    assert_eq!(
        text_global(&it, "अज्ञातप्रकारपाठ"),
        "मञ्जूषा",
        "and the TEXT is kept beside the reason"
    );
}

/// `प्रकारार्थः` types a primitive by NAME and a wrapper by SHAPE.
///
/// Written with the routine. Its two halves were blocked separately and only
/// one still is: the PRIMITIVE arm was blocker (c) — `वास्तुॱप्रकार ॱ
/// नामसूचकाङ्क` is a token index while `मूलप्रकारार्थः` wants text — and (c)
/// is discharged. The four WRAPPER arms are blocker (d), and they follow this
/// file's own precedent: `अभिव्यञ्जकप्रकारः` meets the same hole at its
/// string-literal arm and writes `अन्तःसूचकाङ्क भवति ०`.
///
/// SO THE SECOND ASSERTION PINS A KNOWN WEAKNESS, not a success. A slice
/// answers `खण्डार्थभेद` with `अन्तःसूचकाङ्क` ० — the right shape with an
/// unrepresented element type. `प्रकारसाम्यम्` can tell a slice from a
/// pointer but NOT a slice-of-अ८ from a slice-of-अ३२. It is asserted so the
/// day (d) is answered, this test fails and sends someone to strengthen it.
#[test]
fn the_type_evaluator_names_a_primitive_and_shapes_a_wrapper() {
    let mut it = load_sema_with_parser();

    // BUILT THROUGH `व्याकरॱअभिव्यञ्जकयोजनम्`, THE REAL CONSTRUCTOR, and not
    // by hand. The first version of this test hand-built records shaped like
    // `वास्तुॱप्रकार` — `भेद`/`नामसूचकाङ्क`/`अन्तःप्रकारसूचकाङ्क` — because
    // that is `प्रकारार्थः`'s declared parameter type. IT PASSED, and it was
    // measuring nothing: `वास्तुॱप्रकार` IS A DEAD RECORD, declared in ast.t1
    // and never constructed. `व्याकरॱप्रकारपठनम्` stores every type node with
    // `अभिव्यञ्जकयोजनम्`, so a type is an EXPRESSION — the name's token index
    // lands in `मूल्यसूचकाङ्क` and a wrapper's inner type in `वामसूचकाङ्क`.
    //
    // A test can only be as true as the shape it constructs. Using the
    // appender the parser itself uses is what makes this one true.
    let t_u8 = tok(&mut it, "अ३२");
    let prim_idx = push_expr(&mut it, 1, t_u8, 0, 0); // मूलप्रकारभेद
    let slice_idx = push_expr(&mut it, 2, 0, prim_idx, 0); // खण्डप्रकारभेद
    let nodes = it
        .global("अभिव्यञ्जककोश")
        .expect("the expression arena is a name in scope")
        .clone();

    let prim = it
        .call(
            "अर्थॱप्रकारार्थः",
            vec![nodes.clone(), Value::Int(prim_idx)],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("प्रकारार्थः runs: {e:?}"));
    let kind = |v: &Value| match v {
        Value::Record(r) => r.borrow().get("भेद").and_then(Value::as_int),
        _ => None,
    };
    assert_eq!(
        kind(&prim),
        Some(1),
        "`अ८` must type as पूर्णाङ्कार्थभेद; a दोषार्थः here means the \
         primitive arm never reached मूलप्रकारार्थः, which is blocker (c) \
         returning"
    );

    let slice = it
        .call(
            "अर्थॱप्रकारार्थः",
            vec![nodes, Value::Int(slice_idx)],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("प्रकारार्थः runs: {e:?}"));
    assert_eq!(kind(&slice), Some(4), "a slice must answer खण्डार्थभेद");

    // BLOCKER (d) IS ANSWERED, AND THIS ASSERTION IS WHAT CHANGED. It used to
    // pin अन्तःसूचकाङ्क at ० — this file's placeholder for "no अर्थप्रकार arena
    // exists" — and said in words that reaching a real index should make it
    // fail. It now DOES fail unless the index is real: a NON-ZERO slot into
    // `अर्थप्रकारकोश`, holding the element's own kind, width and signedness.
    let inner_idx = match &slice {
        Value::Record(r) => r.borrow().get("अन्तःसूचकाङ्क").and_then(Value::as_int),
        _ => None,
    };
    assert_ne!(
        inner_idx,
        Some(0),
        "a slice's element type is still the ० placeholder — blocker (d) \
         regressed, or this expression path stopped pushing into \
         `अर्थप्रकारकोश`"
    );
    let elem = it
        .global("अर्थप्रकारकोश")
        .map(arena)
        .and_then(|a| a.get(inner_idx.unwrap_or(0) as usize).cloned())
        .unwrap_or_else(|| panic!("`अर्थप्रकारकोश` at {inner_idx:?} must exist"));
    let get = |v: &Value, f: &str| match v {
        Value::Record(r) => r.borrow().get(f).and_then(Value::as_int),
        _ => None,
    };
    assert_eq!(
        get(&elem, "भेद"),
        Some(1),
        "the slice's element type must be पूर्णाङ्कार्थभेद, not {:?}",
        get(&elem, "भेद")
    );
    // The fixture wraps `अ३२` (see `t_u8` above — misleadingly named; it is
    // NOT अ८), so ३२ is what a correct recursive resolution must answer. A
    // hardcoded ८ here would pass by coincidence on the WRONG element type,
    // exactly the shape of test this file's own history warns against.
    assert_eq!(
        get(&elem, "विस्तार"),
        Some(32),
        "the element width must match what was wrapped, अ३२"
    );
}

/// `मूलप्रकारार्थः` knows the frozen type vocabulary — W-165.
///
/// IT KNEW TWO NAMES: `अ३२` and `बूल`. Everything else answered `दोषार्थः`,
/// which is POISON and exists to SUPPRESS cascading complaints — so a
/// declaration typed `अ८` or `न६४` did not fail typechecking, it silently
/// switched checking off for that declaration while the pass reported success.
///
/// THE VOCABULARY IS TAKEN FROM THE SPEC, not from the routine. `grammar-t1.ebnf`
/// freezes `integer_type = ( "अ" | "न" ) , type_width` with the margin
/// "अंश signed, निर्ऋण unsigned", `type_width = ८ | १६ | ३२ | ६४ | १२८`, and
/// `float_type = "भ" , ( "३२" | "६४" )`. A name added to the grammar and not
/// here fails this test, which is the point.
#[test]
fn every_frozen_primitive_type_name_has_a_semantic_type() {
    let mut it = load_sema_with_parser();
    // (name, kind, width, signed) — kind 1 = पूर्णाङ्कार्थभेद, 2 = भिन्नार्थभेद.
    let cases: &[(&str, i128, i128, bool)] = &[
        ("अ८", 1, 8, true),
        ("अ१६", 1, 16, true),
        ("अ३२", 1, 32, true),
        ("अ६४", 1, 64, true),
        ("अ१२८", 1, 128, true),
        ("न८", 1, 8, false),
        ("न१६", 1, 16, false),
        ("न३२", 1, 32, false),
        ("न६४", 1, 64, false),
        ("न१२८", 1, 128, false),
        ("भ३२", 2, 32, false),
        ("भ६४", 2, 64, false),
        ("बूल", 1, 1, false),
        // `अक्षरम्` AND `पाठः` — blocker (d) decided: both reuse पूर्णाङ्कार्थभेद
        // rather than getting a new kind, the same reuse बूल already makes.
        // `पाठः`'s width/signedness here describe the SLICE's ELEMENT, since
        // this table only carries four columns; the slice wrapper itself and
        // its real (non-placeholder) अर्थप्रकारकोश index are checked in
        // `the_type_evaluator_names_a_primitive_and_shapes_a_wrapper`.
        ("अक्षरम्", 1, 8, false),
    ];
    for (name, kind, width, signed) in cases {
        let v = it
            .call("अर्थॱमूलप्रकारार्थः", vec![octets(name)], 20_000_000)
            .unwrap_or_else(|e| panic!("मूलप्रकारार्थः runs on {name}: {e:?}"));
        let Value::Record(r) = &v else {
            panic!("{name} must answer an अर्थप्रकार, answered {v:?}")
        };
        let got = |f: &str| r.borrow().get(f).and_then(Value::as_int);
        assert_eq!(
            got("भेद"),
            Some(*kind),
            "`{name}` typed as kind {:?}; 12 is दोषार्थभेद, which is POISON and \
             means this name is not recognised — the whole of W-165",
            got("भेद")
        );
        assert_eq!(got("विस्तार"), Some(*width), "`{name}`'s width");
        if *kind == 1 {
            let s = matches!(r.borrow().get("चिह्नितम्"), Some(Value::Bool(true)));
            assert_eq!(
                s, *signed,
                "`{name}`'s signedness. The grammar's own margin at :206 says \
                 अंश is SIGNED and निर्ऋण UNSIGNED"
            );
        }
    }
}

/// A primitive type THE PARSER PRODUCED must resolve to that type.
///
/// THE TEST ABOVE BUILDS ITS OWN TYPE NODE AND PASSES. This one builds
/// nothing: it lexes a real source, parses it, and reads the type index off
/// the declaration `व्याकर` recorded. That difference is the entire point.
///
/// `प्रकारपठनम्`'s primitive arm stores `पठनस्थान` AFTER `अग्रिमम्` has
/// advanced it — `parse.t1:56-57` reads at the cursor and THEN increments — so
/// the index it records names the token one PAST the type name.
/// `प्रकारार्थः` reads that index straight, with no compensation, and types
/// the wrong token.
///
/// FOR THIS SOURCE the token after `अ८` is `आदि`, which matches no arm of
/// `मूलप्रकारार्थः` and comes back `दोषार्थः` — poison, whose whole job is to
/// suppress complaints, so the failure would present as a pass everywhere
/// downstream. That is why this is asserted on WIDTH and not merely on kind.
#[test]
fn a_primitive_type_the_parser_produced_resolves_to_that_type() {
    let src = "वृत्तिः क ददाति अ८ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    let mut it = load_sema_with_parser();

    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    assert!(tokens > 0, "the source must lex to some tokens");

    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    // A COUNT, not a Nil — see the note in the forward-reference test above.
    assert_eq!(
        parsed.as_int(),
        Some(1),
        "the source must parse to exactly one declaration; ० is \
         कार्यक्रमपठनम्'s parse-failure answer"
    );

    // ONE-BASED, slot ० reserved — the declaration pushers advance the cursor
    // and then write at it, so the first declaration is slot १.
    let decl = arena_at_global(&it, "घोषणाकोश", 1);
    let ty_idx = match &decl {
        Value::Record(r) => r.borrow().get("प्रकारसूचकाङ्क").and_then(Value::as_int),
        other => panic!("{other:?} is not a घोषणा"),
    }
    .expect("the declaration records a type index");
    assert!(
        ty_idx > 0,
        "the parser must have recorded a return type for `क`; ० means NO TYPE \
         and the rest of this test would be vacuous"
    );

    let nodes = it
        .global("अभिव्यञ्जककोश")
        .expect("the expression arena is a name in scope")
        .clone();
    let ty = it
        .call("अर्थॱप्रकारार्थः", vec![nodes, Value::Int(ty_idx)], 20_000_000)
        .unwrap_or_else(|e| panic!("प्रकारार्थः runs: {e:?}"));

    let field = |v: &Value, k: &str| match v {
        Value::Record(r) => r.borrow().get(k).and_then(Value::as_int),
        _ => None,
    };
    assert_eq!(
        field(&ty, "भेद"),
        Some(1),
        "`अ८` must type as पूर्णाङ्कार्थभेद. A १२ here is दोषार्थः — the \
         primitive arm resolved a token that is not the type name"
    );
    assert_eq!(
        field(&ty, "विस्तार"),
        Some(8),
        "and it must be EIGHT bits wide. The kind alone would not catch a \
         neighbouring token that happened to name another integer type"
    );
}

/// THE SELF-HOSTED PARSER READS ITS OWN CORPUS. A RATCHET, NOT A REPORT.
///
/// This began as an `#[ignore]`d measurement because it answered `4 of 15`.
/// It is a real test now that it answers all of them, and it is the ACCEPTANCE
/// CRITERION for Sassembly's front end: `व्याकर` parsing every `.t1` source in
/// this crate, 17,551 lines, with `दोषसूचकाङ्क` ०.
///
/// THE FLOOR MAY RISE AND MUST NOT FALL. It is written as a floor rather than
/// an equality so that adding a source cannot silently drop coverage; a file
/// that stops parsing fails here by name.
///
/// A FILE WITH NO DECLARATIONS PASSES. `lib.t1` is three `आयातः` lines, so
/// `कार्यक्रमपठनम्` answers ० declarations and ० errors — correct, not a
/// failure. An earlier form of this check demanded `decls > 0` and reported
/// 14 of 15 for that reason; the criterion was wrong, not the parser.
#[test]
fn the_parser_parses_every_source_in_its_own_corpus() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    assert!(
        names.len() >= 15,
        "only {} .t1 sources found; this guard is keyed to the corpus and an \
         empty list would assert nothing",
        names.len()
    );

    let mut refused: Vec<String> = vec![];
    for n in &names {
        let src = std::fs::read_to_string(dir.join(n)).unwrap();
        let mut it = load_sema_with_parser();
        let toks = match it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(e) => {
                refused.push(format!("{n}: lexer refused it — {e:?}"));
                continue;
            }
        };
        match it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        {
            Ok(_) => {
                let errs = it
                    .global("दोषसूचकाङ्क")
                    .and_then(|x| x.as_int())
                    .unwrap_or(-1);
                if errs != 0 {
                    // Name the FIRST error's line: the rest are cascades.
                    let mut where_ = String::new();
                    // Slot १, not ०: दोषकोश is one-based with slot ० reserved.
                    if let Some(Value::Arena(a)) = it.global("दोषकोश")
                        && let Some(Value::Record(r)) = a.borrow().get(1)
                    {
                        let ln = r.borrow().get("पङ्क्ति").and_then(Value::as_int).unwrap_or(0);
                        let line = src.lines().nth(ln as usize - 1).unwrap_or("?").trim();
                        where_ = format!(" first at line {ln}: {line}");
                    }
                    refused.push(format!("{n}: {errs} error(s){where_}"));
                }
            }
            Err(e) => refused.push(format!("{n}: कार्यक्रमपठनम् failed — {e:?}")),
        }
    }
    assert!(
        refused.is_empty(),
        "the self-hosted parser must read every source in its own corpus, and \
         {} of {} were refused:\n  {}",
        refused.len(),
        names.len(),
        refused.join("\n  ")
    );
}

/// `अङ्कः … अन्तः` — the index the whole corpus is written in.
///
/// MEASURED BEFORE IT WAS WRITTEN. Running the self-hosted parser over its own
/// 15 sources produced 213 parse errors, and **213 of the 213 sit on a line
/// containing `अङ्कः`** — every one, including all 102 in `vakyavibhaga.t1`.
/// It is ONE missing production with 213 symptoms, not 213 problems: the four
/// files that did parse are the four that barely index.
///
/// IT IS A TRANSCRIPTION, NOT A DESIGN. `spec/grammar-t1.ebnf` froze it —
/// `index = "अङ्कः" , expression , "अन्तः"` (:503), reached from
/// `postfix_expr` (:665) and from `place` (:776) — and `ast.t1` has carried
/// `सूचकाङ्काभिव्यञ्जकभेद` (५, `Expression::Index`) unbuilt all along.
///
/// BOTH SHAPES IN ONE TEST because one change serves both: an assignment
/// parses its target through `अभिव्यञ्जकपठनम्` like any other expression, so a
/// postfix index makes `कोश अङ्कः १ अन्तः भवति २` — THE APPENDER IDIOM, which
/// this corpus is built out of — parse as a consequence, not as a second case.
#[test]
fn the_parser_reads_an_index_and_an_index_assignment() {
    let src = "वृत्तिः क ददाति अ६४ आदि\n    \
               कोश अङ्कः १ अन्तः भवति २ ।\n    \
               प्रत्यागमनम् कोश अङ्कः १ अन्तः ।\n\
               इति\n";
    let mut it = load_sema_with_parser();
    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    let errs = it
        .global("दोषसूचकाङ्क")
        .and_then(|v| v.as_int())
        .unwrap_or(-1);
    assert_eq!(
        (parsed.as_int(), errs),
        (Some(1), 0),
        "the source must parse to one declaration with NO errors; ० is \
         कार्यक्रमपठनम्'s parse-failure answer and a non-zero दोषसूचकाङ्क is \
         the parser refusing a line the whole corpus is written in"
    );

    // AND THE NODE MUST BE AN INDEX, not a name that swallowed its brackets.
    // Counting is what separates those: a parser that skipped `अङ्कः … अन्तः`
    // would still parse and would build ZERO index nodes.
    let n = match it.global("अभिव्यञ्जककोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter(|e| match e {
                Value::Record(r) => r.borrow().get("भेद").and_then(Value::as_int) == Some(5),
                _ => false,
            })
            .count(),
        _ => 0,
    };
    assert_eq!(
        n, 2,
        "both `कोश अङ्कः १ अन्तः` occurrences must be सूचकाङ्काभिव्यञ्जकभेद (५) \
         — the assignment TARGET and the returned expression. Zero here means \
         the index was skipped rather than read"
    );

    // AND EACH INDEX MUST HAVE A BASE. Counting alone passed on a WRONG parse:
    // `मूलाभिव्यञ्जकपठनम्` reads `अङ्कः … अन्तः` as a PRIMARY, a bracketed
    // expression standing on its own, so `कोश अङ्कः १ अन्तः` came out as two
    // unrelated things — the name `कोश`, then an index with nothing indexed.
    // The count was satisfied and the parse was wrong. `spec/grammar-t1.ebnf`
    // :665 says `postfix_expr = primary , { member_mark , identifier | index }`
    // — the index is a POSTFIX ON A BASE, and that base is what this asserts.
    let based = match it.global("अभिव्यञ्जककोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter(|e| match e {
                Value::Record(r) => {
                    let b = r.borrow();
                    b.get("भेद").and_then(Value::as_int) == Some(5)
                        && b.get("वामसूचकाङ्क").and_then(Value::as_int).unwrap_or(0) > 0
                        && b.get("दक्षिणसूचकाङ्क").and_then(Value::as_int).unwrap_or(0) > 0
                }
                _ => false,
            })
            .count(),
        _ => 0,
    };
    assert_eq!(
        based, 2,
        "each index must carry BOTH a base (वामसूचकाङ्क) and a subscript \
         (दक्षिणसूचकाङ्क). An index with one operand is `अङ्कः … अन्तः` read as \
         a bracketed primary, which parses `कोश अङ्कः १ अन्तः` as two \
         statements and indexes nothing"
    );
}

/// A ROUTINE WITH PARAMETERS MUST NOT EAT ITS OWN BODY.
///
/// `वृत्तिपठनम्`'s parameter loop had NO EXIT. It bounded itself only on
/// `पठनस्थान न्यूनम् चिह्नकदैर्घ्य` — the end of the token stream — and on
/// seeing `ददाति` it stepped back one and CARRIED ON, so the next turn read
/// `ददाति` as another parameter NAME, `प्रकारपठनम्` ate the return type, and
/// the loop went on through `आदि` and the whole body.
///
/// IT WAS SILENT, WHICH IS WHY IT SURVIVED. A runaway that swallows the body
/// still answers `decls=1` with `दोषसूचकाङ्क=०` — nothing is left to complain
/// about — so a check on those two numbers passes on a parse that read no
/// statements at all. A test I wrote an hour before this one did exactly that
/// and told me the signature was fine. THIS ONE COUNTS THE STATEMENTS.
///
/// Found by bisecting `lex.t1`: of five variants of one real routine, the ONLY
/// one that parsed was the one with its parameter removed.
#[test]
fn a_routine_with_parameters_still_parses_its_body() {
    let src = "वृत्तिः क आदाय अ ॱॱ चिह्नक ददाति अ६४ आदि\n                   चरः ब ॱॱ अ६४ भवति ० ।\n                   ब भवति १ ।\n                   प्रत्यागमनम् ब ।\nइति\n";
    let mut it = load_sema_with_parser();
    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    assert_eq!(
        (
            parsed.as_int(),
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int())
        ),
        (Some(1), Some(0)),
        "one declaration, no errors"
    );

    // THE ASSERTION THAT IS NOT BLIND. Three body statements plus the block
    // that holds them: a parameter loop that ran away would leave FEWER,
    // because the tokens became parameters instead of statements.
    let n = match it.global("वाक्यकोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter(|e| matches!(e, Value::Record(_)))
            .count(),
        _ => 0,
    };
    assert!(
        n >= 4,
        "the body's three statements and their block must be in वाक्यकोश; \
         found {n}. Fewer means the parameter loop consumed the body — and it \
         does so SILENTLY, still answering one declaration and zero errors"
    );
}

/// A CALL — `f आरभ्य a ऽ b समाप्तम्` — which nothing in the corpus can do without.
///
/// `spec/grammar-t1.ebnf:703` freezes `call_expr = postfix_expr ,
/// [ argument_list ]`, and `:705` gives the bracketed form to `n >= 2`
/// deliberately: its own margin says `घ आरभ्य क समाप्तम्` is ALREADY a
/// juxtaposed call over a `group` and "the two readings AGREE", so the bracket
/// is only needed where a `ऽ` appears — "200 sites write it; all 200 carry at
/// least one `ऽ`". `व्याकर` built `आह्वानाभिव्यञ्जकभेद` ZERO times.
///
/// ARGUMENTS CURRY, because an `अभिव्यञ्जक` has exactly two child slots and an
/// argument list has n. `f आरभ्य a ऽ b समाप्तम्` is `(f a) b`: each आह्वान node
/// holds the callee-so-far in वाम and one argument in दक्षिण. That is the same
/// reading the EBNF's juxtaposed alternative already has, so the two forms
/// produce the same tree rather than two shapes meaning one thing.
#[test]
fn the_parser_reads_a_call_with_arguments() {
    let src = "वृत्तिः क ददाति अ६४ आदि\n    \
               चरः ब ॱॱ अ६४ भवति ग आरभ्य घ ऽ ० ऽ ङ समाप्तम् ।\n    \
               प्रत्यागमनम् ब ।\nइति\n";
    let mut it = load_sema_with_parser();
    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    assert_eq!(
        (
            parsed.as_int(),
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int())
        ),
        (Some(1), Some(0)),
        "a three-argument call must parse with no errors"
    );

    // THREE ARGUMENTS MEANS THREE आह्वान NODES, because they curry. Asserting
    // the COUNT and not merely "at least one" is what separates a call that
    // read all its arguments from one that read the first and stopped.
    let calls = match it.global("अभिव्यञ्जककोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter(|e| match e {
                Value::Record(r) => {
                    let b = r.borrow();
                    b.get("भेद").and_then(Value::as_int) == Some(7)
                        && b.get("वामसूचकाङ्क").and_then(Value::as_int).unwrap_or(0) > 0
                        && b.get("दक्षिणसूचकाङ्क").and_then(Value::as_int).unwrap_or(0) > 0
                }
                _ => false,
            })
            .count(),
        _ => 0,
    };
    assert_eq!(
        calls, 3,
        "`ग आरभ्य घ ऽ ० ऽ ङ समाप्तम्` is three curried आह्वान nodes, each with a \
         callee and an argument. Zero means the argument list was never read; \
         one means it stopped after the first `ऽ`"
    );
}

/// A JUXTAPOSED CALL — `आरभ्य f a b c समाप्तम्` — the corpus's ordinary call.
///
/// `spec/grammar-t1.ebnf:705` gives `argument_list` two alternatives, and the
/// bracketed `ऽ`-separated one was written first. This is the OTHER:
/// `postfix_expr , { postfix_expr }` — a callee followed by its arguments with
/// nothing between them. Every remaining first-error in the corpus is this one
/// shape: `आरभ्य अक्षरगणना मूल पूर्वाष्टक पदारम्भः समाप्तम्`,
/// `आरभ्य सङ्ख्या शेषः १० समाप्तम्`.
///
/// IT IS READ ONLY INSIDE A GROUP, and that is a deliberate narrowing rather
/// than the general rule. Juxtaposition has no terminator of its own — a
/// parser that took it anywhere would have to decide by keyword-blacklist
/// where an argument list stops, and getting that wrong breaks the eight files
/// that now parse. Inside `आरभ्य … समाप्तम्` the terminator is explicit, so the
/// rule needs no lookahead table. The corpus writes it inside a group in every
/// case measured.
///
/// ARGUMENTS CURRY, as in the bracketed form, so both spellings build one tree.
#[test]
fn the_parser_reads_a_juxtaposed_call() {
    let src = "वृत्तिः क ददाति अ६४ आदि\n    \
               चरः ब ॱॱ अ६४ भवति आरभ्य ग घ ङ च समाप्तम् ।\n    \
               प्रत्यागमनम् ब ।\nइति\n";
    let mut it = load_sema_with_parser();
    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    assert_eq!(
        (
            parsed.as_int(),
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int())
        ),
        (Some(1), Some(0)),
        "`आरभ्य ग घ ङ च समाप्तम्` — a callee and three arguments — must parse"
    );
    let calls = match it.global("अभिव्यञ्जककोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter(|e| match e {
                Value::Record(r) => {
                    let b = r.borrow();
                    b.get("भेद").and_then(Value::as_int) == Some(7)
                        && b.get("वामसूचकाङ्क").and_then(Value::as_int).unwrap_or(0) > 0
                        && b.get("दक्षिणसूचकाङ्क").and_then(Value::as_int).unwrap_or(0) > 0
                }
                _ => false,
            })
            .count(),
        _ => 0,
    };
    assert_eq!(
        calls, 3,
        "three arguments curry into three आह्वान nodes. One means it read `घ` \
         and stopped; zero means the juxtaposition was never read at all"
    );
}

/// STAGE TWO: how much of its own corpus can the RESOLVER get through?
///
/// The parser now reads all 15 sources. This asks the next question with the
/// same instrument, because "self-parsing is not self-compiling" is a claim
/// that needs a number, not a caveat: for each file, parse it and then run
/// `अर्थॱकार्यक्रमनिर्णयः` over the declarations `व्याकर` produced.
///
/// A `false` is a REAL ANSWER, not a crash — the resolver refusing a program
/// is what it is for — so this reports rather than asserts. It becomes a
/// ratchet the day the number is worth defending, exactly as the parser's did.
#[test]
#[ignore = "measurement"]
fn measure_corpus_resolve() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    let (mut ok, mut refused, mut broke) = (0, 0, 0);
    // ONE interpreter with the store filled (W-223 part 2), as the typecheck
    // census does. Everything this census reads is reset by `निर्णायकारम्भः`,
    // which runs per source below — CHECKED, not assumed, after the checker's
    // four counters were found to be reset by a routine nobody called.
    let mut it = load_sema_collected(&dir, &names);
    for n in &names {
        let src = std::fs::read_to_string(dir.join(n)).unwrap();
        let toks = match it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(_) => continue,
        };
        let parsed = match it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(_) => continue,
        };
        // WHAT THE PARSER ITSELF RECORDED, WHICH THIS CENSUS NEVER READ.
        // `व्याकरदोष` carries a LINE, an aksara and a reason (parse.t1:109),
        // and `दोषकोश` has been collecting them all along — the same shape as
        // the resolver's `अनिर्णीतनाम`: a diagnostic that existed with nothing
        // looking at it. A parse error is UPSTREAM of every resolve refusal,
        // so a file with one is not a resolver problem at all, and four
        // hypotheses about `sanskrit_text.t1`'s `अन्यथा` were argued without
        // ever asking the parser whether it had already complained.
        //
        // Printed for EVERY file and not only the refused ones: a source that
        // RESOLVES while the parser was recording errors is its own finding,
        // and a census that speaks only on failure cannot show it. The arena
        // is 1-based with slot ० reserved, so the errors run 1..=count.
        // THREE STATES, NOT TWO. `unwrap_or(0)` would collapse "the parser
        // recorded no errors" with "there is no such global", and the second
        // means THIS CENSUS is broken. That collapse has already cost this
        // session once, on the type checker's refusal record.
        let perr = match it.global("दोषसूचकाङ्क") {
            None => {
                eprintln!(
                    "  {n:24} the global दोषसूचकाङ्क does not exist — THIS \
                     CENSUS is broken, not the parser"
                );
                -1
            }
            Some(v) => Value::as_int(v).unwrap_or(-1),
        };
        if perr > 0 {
            let slots = it.global("दोषकोश").map(arena).unwrap_or_default();
            let mut shown = Vec::new();
            for i in 1..=perr.min(3) {
                if let Some(Value::Record(rec)) = slots.get(i as usize) {
                    let b = rec.borrow();
                    let line = b.get("पङ्क्ति").and_then(Value::as_int).unwrap_or(0);
                    let why = match b.get("कारण") {
                        Some(Value::Octets(o)) => {
                            String::from_utf8_lossy(o.as_slice()).into_owned()
                        }
                        _ => "<no reason>".to_string(),
                    };
                    shown.push(format!("line {line}: {why}"));
                }
            }
            eprintln!("  {n:24} PARSE ERRORS: {perr} — {shown:?}");
        }
        let r = match it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        {
            Ok(v) => v,
            Err(e) => {
                eprintln!("  {n:24} निर्णायकारम्भः failed: {e:?}");
                broke += 1;
                continue;
            }
        };
        match it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r, Value::Int(parsed)],
            4_000_000_000,
        ) {
            Ok(Value::Bool(true)) => {
                ok += 1;
                eprintln!("  {n:24} {parsed:>4} decls  RESOLVED");
            }
            Ok(other) => {
                refused += 1;
                // ASK THE RESOLVER WHICH NAME. `अनिर्णीतनाम` has recorded the
                // FIRST refusal since W-163 and this census was still printing
                // a bare `Bool(false)` — the record existed and nothing read
                // it. Three states, not two: a missing global means THIS TEST
                // is broken, which is not the same as the resolver having
                // refused somewhere that does not report. The stage-three
                // census learned that the hard way on 2026-09-02.
                //
                // The key is BARE. `Interpreter::global` looks up the name as
                // declared (nirvahana.rs:538), so `अर्थॱअनिर्णीतनाम` finds
                // nothing at all and finds it silently.
                let why = match it.global("अनिर्णीतमस्ति") {
                    None => "the record global अनिर्णीतमस्ति does not exist —                              THIS TEST is broken, not the resolver"
                        .to_string(),
                    Some(Value::Bool(true)) => match it.global("अनिर्णीतनाम") {
                        Some(Value::Octets(o)) => {
                            let line = it
                                .global("अनिर्णीतपङ्क्ति")
                                .and_then(sadhana::t1::nirvahana::Value::as_int)
                                .unwrap_or(0);
                            format!(
                                "undeclared `{}` at line {line}",
                                String::from_utf8_lossy(o.as_slice())
                            )
                        }
                        other => format!("<record is not a run: {other:?}>"),
                    },
                    Some(_) => format!("refused with no name recorded ({other:?})"),
                };
                eprintln!("  {n:24} {parsed:>4} decls  refused — {why}");
            }
            Err(e) => {
                broke += 1;
                eprintln!(
                    "  {n:24} {parsed:>4} decls  RUN ERROR {:.100}",
                    format!("{e:?}")
                );
            }
        }
    }
    eprintln!(
        "\n  RESOLVE: {ok} resolved, {refused} refused, {broke} run-error, of {} files",
        names.len()
    );
}

/// A ROUTINE'S PARAMETERS ARE IN SCOPE IN ITS BODY.
///
/// MEASURED FIRST. `measure_corpus_resolve` answered 4 resolved, 11 refused, 0
/// run-errors of 15 — and the four that resolved were the four with almost
/// nothing in them (`ast.t1` 3 declarations, `vastu.t1` 5, `kosha.t1` 1,
/// `lib.t1` 0). Bisecting `lex.t1` put the first refusal on `त्र्यष्टकसाम्यम्`,
/// whose body uses its parameters.
///
/// THE CAUSE WAS NOT IMPORTS, which is what the refusals looked like.
/// `संरचना घोषणा` had five fields and none was a parameter list, and
/// `वृत्तिपठनम्` read `प्राचलनाम`/`प्राचलप्रकार` into LOCALS and never stored
/// them — the parser parsed parameters and threw them away. So
/// `कार्यक्रमनिर्णयः` opened a body scope, declared NOTHING in it, and refused
/// every parameter use. The refusal was CORRECT on the information it had.
///
/// THE SECOND HALF OF THIS TEST IS THE POINT. A fix that declared everything —
/// or stopped resolving bodies — would pass the first assertion and switch the
/// pass off. So an undeclared name must STILL be refused, in a routine of the
/// same shape, and both are asserted together.
#[test]
fn a_routines_parameters_are_in_scope_in_its_body() {
    let uses_param = "वृत्तिः क आदाय अ ॱॱ अ६४ ऽ ब ॱॱ अ६४ ददाति अ६४ आदि\n    \
                      प्रत्यागमनम् अ ।\nइति\n";
    let uses_ghost = "वृत्तिः क आदाय अ ॱॱ अ६४ ऽ ब ॱॱ अ६४ ददाति अ६४ आदि\n    \
                      प्रत्यागमनम् अपरिचित ।\nइति\n";

    let resolve = |src: &str| -> Value {
        let mut it = load_sema_with_parser();
        let tokens = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
            .as_int()
            .expect("a token count");
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
        assert_eq!(
            (
                parsed.as_int(),
                it.global("दोषसूचकाङ्क").and_then(|v| v.as_int())
            ),
            (Some(1), Some(0)),
            "the source must parse before its resolution means anything"
        );
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"))
    };

    assert_eq!(
        resolve(uses_param),
        Value::Bool(true),
        "`अ` is a PARAMETER of `क` and its body returns it. A false here means \
         the parameters never reached the body's scope — which is every \
         substantial file in the corpus being refused"
    );
    assert_eq!(
        resolve(uses_ghost),
        Value::Bool(false),
        "AND THE PASS MUST STILL REFUSE. `अपरिचित` is declared nowhere, in a \
         routine of the same shape. A true here means the fix declared \
         everything, or stopped walking bodies — passing by switching the \
         pass off rather than by resolving anything"
    );
}

/// `सत्यम्` AND `असत्यम्` ARE LITERALS, NOT NAMES.
///
/// `spec/grammar-t1.ebnf:138` lists them among the KEYWORDS — "true / false" —
/// so they are no more identifiers than `यदि` is. `व्याकर` built a
/// `नामाभिव्यञ्जक` for each, and `अर्थ` then refused them as undeclared, which
/// was the correct answer to the wrong question.
///
/// MEASURED: of four routines differing only in their body, the one returning
/// a PARAMETER resolved and the three ending `प्रत्यागमनम् सत्यम् ।` did not.
/// The corpus writes these two words 552 times across the 15 sources, so this
/// alone refuses most files that resolve everything else.
///
/// A NINTH EXPRESSION KIND, and the divergence from `ast.rs` is deliberate:
/// that twin's `Expression` already has five variants against this corpus's
/// eight and defers the rest out loud (`ast.rs:36`). Spelling a boolean as a
/// NUMERAL would have avoided the new kind and lied about the type — the
/// typechecker would then have to unpick it, which is a worse debt than a
/// constant.
#[test]
fn a_boolean_literal_is_not_an_undeclared_name() {
    let resolve = |src: &str| -> Value {
        let mut it = load_sema_with_parser();
        let tokens = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
            .as_int()
            .expect("a token count");
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
        assert_eq!(
            (
                parsed.as_int(),
                it.global("दोषसूचकाङ्क").and_then(|v| v.as_int())
            ),
            (Some(1), Some(0)),
            "the source must parse before its resolution means anything"
        );
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"))
    };

    assert_eq!(
        resolve("वृत्तिः क ददाति बूल आदि\n    प्रत्यागमनम् सत्यम् ।\nइति\n"),
        Value::Bool(true),
        "`सत्यम्` is a keyword literal — a false here means व्याकर built a name \
         from it and अर्थ refused it as undeclared"
    );
    assert_eq!(
        resolve("वृत्तिः क ददाति बूल आदि\n    प्रत्यागमनम् असत्यम् ।\nइति\n"),
        Value::Bool(true),
        "and so is `असत्यम्`"
    );
    assert_eq!(
        resolve("वृत्तिः क ददाति बूल आदि\n    प्रत्यागमनम् अपरिचित ।\nइति\n"),
        Value::Bool(false),
        "AND A REAL NAME MUST STILL BE REFUSED. A true here would mean the fix \
         stopped resolving names rather than recognising two keywords"
    );
}

/// A MODULE-LEVEL `चरः` IS A DECLARATION, AND THE PARSER SKIPPED IT.
///
/// `spec/grammar-t1.ebnf:768` spells it out — `binding = [ "सार्वजनिक" ] ,
/// "चरः" , identifier , annotation , type , "भवति" , expression , danda` —
/// with `सार्वजनिक` optional precisely so a binding may stand at module level.
/// `कार्यक्रमपठनम्` recognised only `वृत्तिः` and `संरचना` and walked over
/// everything else ONE TOKEN AT A TIME, so a module variable never reached
/// `घोषणाकोश` and `अर्थ` could not know it existed.
///
/// MEASURED: an instrument collecting every name a body mentions less every
/// name its file declares put `पठनस्थान` at 40 uses, `चिह्नकदैर्घ्य` at 13 and
/// `घोषणासूचकाङ्क` at 10 — all of them module variables of the very file that
/// declares them.
///
/// THE REFUSAL IS KEPT, as in `W-169` and `W-170`: a routine reading an
/// undeclared name must still be refused, or "declare everything" passes.
#[test]
fn a_module_level_binding_is_in_scope_in_every_routine() {
    let resolve = |src: &str| -> (Option<i128>, Value) {
        let mut it = load_sema_with_parser();
        let tokens = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
            .as_int()
            .expect("a token count");
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
        assert_eq!(
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int()),
            Some(0),
            "the source must parse without error before its resolution means \
             anything"
        );
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        let out = it
            .call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed.clone()], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"));
        (parsed.as_int(), out)
    };

    let (decls, ok) =
        resolve("सार्वजनिक चरः ग ॱॱ अ६४ भवति ० ।\nवृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ग ।\nइति\n");
    assert_eq!(
        decls,
        Some(2),
        "the module binding AND the routine must both be declarations; १ means \
         the binding was skipped rather than parsed"
    );
    assert_eq!(
        ok,
        Value::Bool(true),
        "`ग` is declared at module level and the routine returns it. A false \
         means module bindings never reach घोषणाकोश"
    );

    let (_, refused) = resolve(
        "सार्वजनिक चरः ग ॱॱ अ६४ भवति ० ।\nवृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् अपरिचित ।\nइति\n",
    );
    assert_eq!(
        refused,
        Value::Bool(false),
        "AND A REAL UNDECLARED NAME MUST STILL BE REFUSED, in a file that does \
         have a module binding — otherwise the fix declared everything"
    );
}

/// A BLOCK RESOLVES ITS STATEMENTS IN THE ORDER THEY ARE WRITTEN.
///
/// `वाक्यनिर्णयः`'s block arm started at `दक्षिणसूचकाङ्क` — the LAST statement —
/// and stepped backwards by `आदिसूचकाङ्क वियोगः १`. That walk is correct for
/// VISITING EACH DIRECT CHILD ONCE, which is what the subtree index was added
/// for, and WRONG FOR DECLARATION ORDER: a local declared by `चरः` at the top
/// of a body was declared AFTER every use of it had already been resolved, so
/// every local was undeclared at the moment it was read.
///
/// MEASURED, over the names a statement actually points at: `फलम्` 71 uses,
/// `नव` 50, `सूचकाङ्क` 26 — the corpus's commonest LOCALS, undeclared in the
/// very bodies that declare them.
///
/// THE SECOND CASE IS THE ONE THAT KEEPS THE FIX HONEST. A body that USES a
/// name BEFORE declaring it must still be refused; a "declare every `चरः` in
/// the block first" "fix" would pass the first case and accept this one, which
/// is not order-sensitivity but its absence.
#[test]
fn a_block_resolves_its_statements_in_written_order() {
    let resolve = |src: &str| -> Value {
        let mut it = load_sema_with_parser();
        let tokens = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
            .as_int()
            .expect("a token count");
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
        assert_eq!(
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int()),
            Some(0),
            "the source must parse without error first"
        );
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"))
    };

    assert_eq!(
        resolve("वृत्तिः क ददाति अ६४ आदि\n    चरः ग ॱॱ अ६४ भवति ० ।\n    प्रत्यागमनम् ग ।\nइति\n"),
        Value::Bool(true),
        "`ग` is declared on the line before it is used. A false means the block \
         was walked backwards and the use was resolved before the declaration"
    );
    assert_eq!(
        resolve("वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् ग ।\n    चरः ग ॱॱ अ६४ भवति ० ।\nइति\n"),
        Value::Bool(false),
        "AND USE BEFORE DECLARATION MUST STILL BE REFUSED. A true here means \
         the block declares its `चरः` statements up front instead of in order, \
         which is not order-sensitivity but its absence"
    );
}

/// A QUALIFIED NAME RESOLVES AGAINST ITS IMPORT, NOT AGAINST NOTHING.
///
/// `पदविभागॱचिह्नककोश` is ONE token — the lexer peels only a leading or
/// trailing sign, so an interior `ॱ` stays inside the word. Measured: the
/// census of names a statement points at reported such names WHOLE, and 65
/// uses of them were undeclared, because they name something in ANOTHER
/// module and this file declares nothing of the sort.
///
/// THE CHECK IS THE IMPORT, NOT THE MARK. Accepting any name containing `ॱ`
/// would resolve `कल्पितॱकिञ्चित्` from a module never imported, which is
/// switching the check off for every qualified name in the corpus. So
/// `आयातः` becomes a declaration, and a qualified name resolves only when its
/// PREFIX — the part before the first `ॱ` — was imported.
///
/// The two halves are asserted together for that reason: the same source with
/// and without its `आयातः` line must resolve and refuse.
#[test]
fn a_qualified_name_resolves_only_against_an_import() {
    let resolve = |src: &str| -> Value {
        let mut it = load_sema_with_parser();
        let tokens = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
            .as_int()
            .expect("a token count");
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
        assert_eq!(
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int()),
            Some(0),
            "the source must parse without error first"
        );
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"))
    };

    let body = "वृत्तिः क ददाति अ६४ आदि\n    प्रत्यागमनम् पदविभागॱकिञ्चित् ।\nइति\n";
    assert_eq!(
        resolve(&format!("आयातः पदविभाग ।\n{body}")),
        Value::Bool(true),
        "`पदविभाग` is imported and `पदविभागॱकिञ्चित्` names something in it. A \
         false means `आयातः` is still skipped and qualified names resolve \
         against nothing"
    );
    assert_eq!(
        resolve(body),
        Value::Bool(false),
        "AND WITHOUT THE IMPORT IT MUST BE REFUSED. A true here means the fix \
         accepts any name containing `ॱ`, which switches the check off for \
         every qualified name in the corpus rather than performing it"
    );
}

/// `शून्यम्` IS A LITERAL, NOT A NAME — and the resolver said so itself.
///
/// `spec/grammar-t1.ebnf:137` lists it among the KEYWORDS, "void". The corpus
/// writes it 268 times, almost always as the value a `सम्भाव्य` is compared
/// against, and `व्याकर` built a `नामाभिव्यञ्जक` for each — so `अर्थ` refused
/// it as undeclared. Same shape as `सत्यम्`/`असत्यम्` in W-170.
///
/// HOW IT WAS FOUND IS THE POINT. Three successive censuses over the arenas
/// named the wrong culprits, because reading `वाम`/`दक्षिण` from OUTSIDE means
/// modelling what they mean for each statement kind, and a `चरवाक्य` holds a
/// TOKEN index where an `अभिव्यञ्जकवाक्य` holds an EXPRESSION index. So the
/// resolver now records the name it refused (`अर्थॱअनिर्णीतनाम`) and named
/// this in one run. That record is not scaffolding: a pass that answers only
/// `असत्यम्` cannot produce a diagnostic either.
#[test]
fn the_void_keyword_is_a_literal_not_a_name() {
    let resolve = |src: &str| -> Value {
        let mut it = load_sema_with_parser();
        let tokens = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
            .as_int()
            .expect("a token count");
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
        assert_eq!(
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int()),
            Some(0),
            "the source must parse without error first"
        );
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"))
    };

    assert_eq!(
        resolve(
            "वृत्तिः क आदाय अ ॱॱ सम्भाव्य अ६४ ददाति बूल आदि\n                 यदि अ समम् शून्यम् आदि\n        प्रत्यागमनम् सत्यम् ।\n    इति\n                 प्रत्यागमनम् असत्यम् ।\nइति\n"
        ),
        Value::Bool(true),
        "`शून्यम्` is a keyword literal — a false means व्याकर built a name from \
         it and अर्थ refused it as undeclared"
    );
    assert_eq!(
        resolve(
            "वृत्तिः क आदाय अ ॱॱ सम्भाव्य अ६४ ददाति बूल आदि\n                 यदि अ समम् अपरिचित आदि\n        प्रत्यागमनम् सत्यम् ।\n    इति\n                 प्रत्यागमनम् असत्यम् ।\nइति\n"
        ),
        Value::Bool(false),
        "AND A REAL NAME IN THE SAME POSITION MUST STILL BE REFUSED"
    );
}

/// A DOUBLED `इति` IS THE WORD ITSELF — ADR-0011, and the parser did not know.
///
/// The ADR is explicit: "`इति इति` is a literal `इति`. A single `इति` closes
/// the string", and "scanning left to right makes it unambiguous: a run of
/// `इति` tokens pairs off". `मूलाभिव्यञ्जकपठनम्`'s string arm closed at the
/// FIRST `इति` regardless, so `उक्तम् इति इति इति` — the corpus's own way of
/// writing the string "इति" — closed on the opener's heels and left TWO `इति`
/// tokens loose, which the parser then read as NAMES and `अर्थ` refused.
///
/// FOUND BY THE RESOLVER NAMING ITS OWN REFUSAL: `parse.t1`'s was `इति`. A
/// keyword arriving at the name arm is not a missing literal — it is the
/// parser having mis-shaped something earlier, which is what this was.
///
/// THE COUNT IS THE ASSERTION. A string that closed too early still produces a
/// `उक्ताभिव्यञ्जक`, so "a string node exists" would pass on the broken parse;
/// what separates them is that the tokens after it are CONSUMED rather than
/// left to be read as names.
#[test]
fn a_doubled_iti_is_the_word_itself_not_the_end_of_the_string() {
    let src = "वृत्तिः क आदाय अ ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि\n    \
               यदि अ समम् उक्तम् इति इति इति आदि\n        \
               प्रत्यागमनम् सत्यम् ।\n    इति\n    \
               प्रत्यागमनम् असत्यम् ।\nइति\n";
    let mut it = load_sema_with_parser();
    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    assert_eq!(
        (
            parsed.as_int(),
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int())
        ),
        (Some(1), Some(0)),
        "the source must parse to one declaration with no errors"
    );

    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    assert_eq!(
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}")),
        Value::Bool(true),
        "a string holding `इति` must resolve. A false means the string closed \
         at its first `इति` and the rest became NAMES — which is what \
         `parse.t1`'s own refusal reported"
    );
}

/// `समावेशः` AND `ऋण` — two frozen productions the parser did not have.
///
/// Both were named by the resolver's own FIRST-refusal record (W-176), and
/// both are transcriptions rather than designs:
///
///   `embed      = "समावेशः" , "आरभ्य" , identifier , "समाप्तम्"`  (:341)
///   `unary_expr = { unary_op } , call_expr`, `unary_op = "ऋण"`   (:659, :628)
///
/// `समावेशः` accounted for TWO of the seven remaining refusals — it is how
/// `samyojana.t1` and `sanskrit_text.t1` embed a generated table. `ऋण` is
/// SPACED negation: the lexer's own note distinguishes `ऋण१`, one token and a
/// negative numeral, from `ऋण सीमा`, which is two.
///
/// EACH CASE KEEPS ITS REFUSAL. An embed of an undeclared table and a negation
/// of an undeclared name must both still be refused, or these arms would be
/// accepting whatever follows the keyword.
#[test]
fn the_embed_and_unary_minus_productions_are_read() {
    let resolve = |src: &str| -> Value {
        let mut it = load_sema_with_parser();
        let tokens = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
            .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
            .as_int()
            .expect("a token count");
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
        assert_eq!(
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int()),
            Some(0),
            "the source must parse without error first: {src}"
        );
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("निर्णायकारम्भः runs");
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}"))
    };

    // EMBED — the table name is declared, so it resolves.
    assert_eq!(
        resolve(
            "सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः अ८ भवति ० ।\nवृत्तिः क ददाति अङ्कः अन्तः अ८ आदि\n    प्रत्यागमनम् समावेशः आरभ्य कोशः समाप्तम् ।\nइति\n"
        ),
        Value::Bool(true),
        "`समावेशः आरभ्य कोशः समाप्तम्` is the embed production — a false means \
         `समावेशः` was read as a NAME with the table as its argument"
    );
    // AND AN EMBED OF A TABLE THE PROGRAM DOES NOT DECLARE ALSO RESOLVES —
    // which is the opposite of what this test first asserted, and the corpus
    // corrected it. A table is a REGISTRY entry, not a scope name: no program
    // declares `संस्कारकोशः`, and `anita::resolve` refuses an unknown table at
    // LOAD time ("is not a table this program may bring in") before this pass
    // runs. Re-checking it here against the wrong namespace made every valid
    // embed an error — measured, as `samyojana.t1` began refusing
    // `संस्कारकोशः` the moment the arm landed.
    assert_eq!(
        resolve(
            "वृत्तिः क ददाति अङ्कः अन्तः अ८ आदि\n    प्रत्यागमनम् समावेशः आरभ्य कश्चित्कोशः समाप्तम् ।\nइति\n"
        ),
        Value::Bool(true),
        "an embed`s table is not resolved against program scope; the registry \
         check is anita`s and happens at load"
    );

    // UNARY MINUS — spaced, so two tokens.
    assert_eq!(
        resolve(
            "वृत्तिः क आदाय सीमा ॱॱ अ६४ ददाति बूल आदि\n    \
             यदि सीमा न्यूनम् ऋण सीमा आदि\n        प्रत्यागमनम् सत्यम् ।\n    इति\n    \
             प्रत्यागमनम् असत्यम् ।\nइति\n"
        ),
        Value::Bool(true),
        "`ऋण सीमा` is unary negation of a PARAMETER — a false means `ऋण` was \\
         read as a name of its own, which is encode.t1's first refusal"
    );
    assert_eq!(
        resolve(
            "वृत्तिः क आदाय सीमा ॱॱ अ६४ ददाति बूल आदि\n    \
             यदि सीमा न्यूनम् ऋण अपरिचित आदि\n        प्रत्यागमनम् सत्यम् ।\n    इति\n    \
             प्रत्यागमनम् असत्यम् ।\nइति\n"
        ),
        Value::Bool(false),
        "AND NEGATING AN UNDECLARED NAME MUST STILL BE REFUSED — otherwise the \\
         unary arm accepts whatever follows `ऋण`"
    );
}

/// `शेषः` AND `बृहत्समम्` ARE OPERATORS — ADR-0037, by owner decision.
///
/// ADR-0032 ruled both absent, "recorded so nobody re-derives it", and each
/// entry failed differently. MODULO: its evidence — the `शेषः` tokens are all
/// a local variable — was TRUE on 2026-08-30 and `encode.t1:647`'s operator
/// use is dated 08-31, the day after. `>=`: it asked whether `न्यूनसमम्` and
/// `अधिकसमम्` appear; the corpus writes `बृहत्समम्`, and `vakyavibhaga.t1` has
/// carried it since 08-29, the day BEFORE. That search was conducted in the
/// LEXICON's vocabulary inside an ADR whose principle is that the CORPUS is
/// canonical — a census keyed to a candidate list can only find candidates.
///
/// THE ASSERTION IS ON THE OPERATOR KIND, not merely on resolution. A parser
/// that read `शेषः` as a NAME would still resolve this program if `शेषः` were
/// declared, so the test declares nothing of the sort and reads the द्विकर्म
/// node's own `द्विकर्म` field.
#[test]
fn modulo_and_greater_or_equal_are_operators() {
    let mut it = load_sema_with_parser();
    let src = "वृत्तिः क आदाय अ ॱॱ अ६४ ऽ ब ॱॱ अ६४ ददाति बूल आदि\n    \
               यदि आरभ्य अ शेषः १० समाप्तम् बृहत्समम् ब आदि\n        \
               प्रत्यागमनम् सत्यम् ।\n    इति\n    \
               प्रत्यागमनम् असत्यम् ।\nइति\n";
    let tokens = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 50_000_000)
        .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"))
        .as_int()
        .expect("a token count");
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokens)], 200_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमपठनम् runs: {e:?}"));
    assert_eq!(
        (
            parsed.as_int(),
            it.global("दोषसूचकाङ्क").and_then(|v| v.as_int())
        ),
        (Some(1), Some(0)),
        "the source must parse to one declaration with no errors"
    );

    // The two new द्विकर्म kinds must BOTH appear, and as operators — १४ is
    // शेषद्विकर्मभेद and १५ is बृहत्समद्विकर्मभेद.
    let kinds: Vec<i128> = match it.global("अभिव्यञ्जककोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter_map(|e| match e {
                Value::Record(r) => r.borrow().get("द्विकर्म").and_then(Value::as_int),
                _ => None,
            })
            .filter(|k| *k > 0)
            .collect(),
        _ => vec![],
    };
    assert!(
        kinds.contains(&14),
        "`अ शेषः १०` must build a शेषद्विकर्मभेद (१४); operator kinds seen: \
         {kinds:?}. An empty or ०-only list means `शेषः` was read as a NAME"
    );
    assert!(
        kinds.contains(&15),
        "`… बृहत्समम् ब` must build a बृहत्समद्विकर्मभेद (१५); operator kinds \
         seen: {kinds:?}"
    );

    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("निर्णायकारम्भः runs");
    assert_eq!(
        it.call("अर्थॱकार्यक्रमनिर्णयः", vec![r, parsed], 200_000_000)
            .unwrap_or_else(|e| panic!("कार्यक्रमनिर्णयः runs: {e:?}")),
        Value::Bool(true),
        "and the program resolves — neither operator is a name to look up"
    );
}
/// A statement kind as its name, for the poison report.
fn return_kind(k: i128) -> &'static str {
    match k {
        1 => "अभिव्यञ्जक",
        2 => "समूह",
        3 => "चर",
        4 => "प्रत्यागमन",
        5 => "आयात",
        8 => "सम",
        _ => "unknown",
    }
}

/// STAGE THREE: how much of its own corpus can the TYPE CHECKER get through?
///
/// # Why this exists at all
///
/// "Sassembly is done when the T1 chain self-hosts" — lex, parse, resolve,
/// typecheck, encode. Stage one is 15/15 and ratcheted; stage two is measured
/// each cycle by [`measure_corpus_resolve`]. STAGE THREE HAD NO NUMBER, and an
/// unmeasured stage is not a small remainder — it is an unknown one. Every ETA
/// this task has produced was wrong in the same direction, and always because
/// the remaining causes could not be guessed from outside. The resolve census
/// is what turned "the resolver does not work" into ten named causes; this is
/// the same instrument pointed one stage further on.
///
/// # What was found before a line of it ran
///
/// `artha.t1:604` heads its lower half with *"Every वृत्तिः below is a STUB and
/// says so by returning अपूर्णम्"*. THAT IS STALE: `कार्यक्रमप्रकारपरीक्षा`
/// (:1280) is fully written, and the corpus's stub census is ONE — `सङ्कोचः`,
/// in `encode.t1`. The type checker has been implemented for some time and had
/// simply never been run over the corpus. That is the fourth stale blocker note
/// found by auditing rather than believing one.
///
/// # Reading the output
///
/// A file must RESOLVE before it can be typechecked, so a file that stage two
/// refuses is reported `not reached` and is NOT counted as a typecheck failure —
/// conflating the two would credit stage three with stage two's debt. `बूल` is
/// the answer; the return is `दोषयुक्त बूल`, so an error variant is a REFUSAL
/// the checker meant, and only an `Err` from the interpreter is a run error.
///
/// # WHAT IT FOUND ON ITS FIRST INSTRUMENTED RUN, AND IT IS ONE CAUSE
///
/// All six refusals were the SAME mismatch — `body type 12 vs declared 1`,
/// `Ty::Error` (poison) against `Ty::Int` — in six different routines in six
/// different files. Not six bugs: one.
///
/// **THAT CAUSE IS FIXED AND THE NUMBER IS NOW 5 TYPECHECKED, 5 REFUSED, 5 NOT
/// REACHED.** `वाक्यप्रकारः` gained its `प्रत्यागमनवाक्यभेद` arm — a return
/// diverges, so it types as `अभावार्थः` and `प्रत्यागमनसाम्यम्`'s Never escape
/// finally has something that can make it fire. `lex.t1` typechecks.
///
/// THE FIVE THAT REMAIN ARE TWO NEW CAUSES, both named by this census rather
/// than guessed:
///
/// * **The DECLARED side is poison** — `artha.t1`'s `निर्णायकारम्भः` and
///   `ir.t1`'s `नवमूल्यम्` read `body 1 vs declared 12`. `प्रकारार्थः` cannot
///   type a return whose type is a named record, so the declaration, not the
///   body, is what has no type.
/// * **Both sides are `Ty::Int` and it still refuses** — `samyojana.t1`'s
///   `दत्ताधारः`, `utsarjana.t1`'s `यतियोजनम्` and `vishlesana.t1`'s
///   `खण्डसीमा` all read `body 1 vs declared 1`, so `प्रकारसाम्यम्` is
///   comparing width and signedness and they differ. The reason is in
///   `अभिव्यञ्जकप्रकारः`'s own margin: an Identifier types as `अ३२`-signed
///   because "the resolver checked it exists, but we haven't stored var types
///   yet". EVERY name and numeral in the corpus types as one fixed integer, so
///   any routine returning `न६४` or `अ६४` from an expression cannot match.
///   That is not an arm to add; it is a variable type ENVIRONMENT the pass
///   does not have.
///
/// `वाक्यप्रकारः` HAS EXACTLY TWO ARMS. It types an expression-statement and a
/// block (as its last statement), and **everything else falls to `दोषार्थः`**.
/// The corpus's routines end in `प्रत्यागमनम्`, `यदि`, `यावत्` or an
/// assignment — none of which has an arm — so a body types as poison and
/// cannot match any declared return. `अभिव्यञ्जकप्रकारः` is the same shape one
/// level down: Identifier, Numeral, StringLiteral, Group and a poisoned Index,
/// with NO arm for call, binary or field, which its own margin says is because
/// "the Rust sema has no rule for them, so there is no rule to port".
///
/// SO STAGE THREE IS NOT A STAGE WITH A FEW BUGS IN IT. It is a checker that
/// implements three of roughly six statement forms and five of roughly nine
/// expression forms, and has no variable type environment at all, and the missing ones have no Rust original to port — they
/// have to be DESIGNED. That is a different kind of work from stages one and
/// two, and it is the honest basis for any estimate of what remains.
///
/// A guess is on the record as wrong, so the method is worth keeping: the six
/// were first read as "routines ending in a call or a binary", which the
/// margin of `अभिव्यञ्जकप्रकारः` made plausible. Opening the three named
/// routines showed every one ends in `प्रत्यागमनम्` with a LITERAL —
/// `असत्यम्`, `०` — so the poison comes from the statement typer, not the
/// expression typer, and the fix belongs one level up from where it looked.
///
/// Reports rather than asserts, exactly as the resolve census did until its
/// number was worth defending.
#[test]
#[ignore = "measurement"]
fn measure_corpus_typecheck() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();

    let (mut typed, mut refused, mut broke, mut unreached) = (0, 0, 0, 0);
    // W-202's two blind-spot counters, summed across the corpus. They are read
    // per file because the census builds a fresh interpreter for each one, and
    // `प्रकारपरीक्षकारम्भः` zeroes them at the start of every typecheck.
    let (mut untypable_conds, mut untypable_stmts) = (0i128, 0i128);
    // W-231: branch disagreements mid-body (counted, not refused) and conditions
    // that type to a real non-बूल type (refused everywhere; the count says
    // whether that is one defective site or a pattern the grammar must rule on).
    let (mut unjoined, mut nonbool_conds) = (0i128, 0i128);
    // W-223 part 2: qualified uses found in the store, the two trust paths, and
    // the found-but-return-type-unrecorded remainder that is W-248's number.
    let (mut store_found, mut trusted_no_collection) = (0i128, 0i128);
    let (mut trusted_uncollected_module, mut type_unrecorded) = (0i128, 0i128);
    // W-265. `no_type_text` is the half of the OLD `type_unrecorded` that was
    // never a failed read at all — a `संरचना`, a `गणना` or a `वृत्ति` with no
    // `ददाति` has no return to record — and the two now sum to what the one
    // used to hold. `weak_rule` counts the times a BARE spelling was resolved by
    // "exactly one collected module declares it" rather than by a module the
    // caller named: the weaker reading, never silent.
    let (mut no_type_text, mut weak_rule) = (0i128, 0i128);
    // WHERE each untypable statement is, not just how many. A count cannot be
    // acted on; a file and a line can be read.
    let mut untypable_sites: Vec<(String, i128)> = Vec::new();
    // THE BAD-FIELD SWEEP. A field a struct does not declare, per source: the COUNT
    // for every source and the FIRST site named. The count is carried beside
    // the site because the checker records one site per source — a count of २
    // with one line printed says the other is still unnamed, which a list of
    // sites alone would not.
    //
    // THIS SEES ONLY FIELD READS THE CHECKER ACTUALLY TYPES, and that is a
    // narrow slice: `वाक्यप्रकारः`'s return, चरः and असाइन arms answer a type
    // WITHOUT typing their operand, so a field read in a return, an
    // initialiser or an assignment's right side never reaches the क्षेत्र arm
    // at all. An empty sweep here is evidence about expression statements and
    // conditions, and about nothing else.
    let mut bad_field_sites: Vec<(String, i128, String, String, i128)> = Vec::new();
    // ONE interpreter, with the store filled from every source and the
    // collection flag raised (W-223 part 2). It was one PER SOURCE until the
    // store arrived; see `load_sema_collected` for why that had to change and
    // which two leaks had to close before it was safe to.
    let mut it = load_sema_collected(&dir, &names);
    for n in &names {
        let src = std::fs::read_to_string(dir.join(n)).unwrap();
        let toks = match it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(_) => {
                eprintln!("  {n:24} not reached (lex)");
                unreached += 1;
                continue;
            }
        };
        let parsed = match it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(_) => {
                eprintln!("  {n:24} not reached (parse)");
                unreached += 1;
                continue;
            }
        };
        let r = match it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        {
            Ok(v) => v,
            Err(_) => {
                eprintln!("  {n:24} not reached (resolver init)");
                unreached += 1;
                continue;
            }
        };
        // Stage two must pass before stage three has anything to check.
        match it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r.clone(), Value::Int(parsed)],
            4_000_000_000,
        ) {
            Ok(Value::Bool(true)) => {}
            other => {
                // AND AN `Err` IS NOT A REFUSAL. This arm used to be `_`, which
                // swallowed a RunError and a `false` into one line reading "not
                // reached (resolve)" — so a pass that CRASHED and a pass that
                // REFUSED were indistinguishable, and neither flag being set
                // looked like a third mystery rather than the obvious signal
                // that no refusal had happened because the call never finished.
                if let Err(e) = &other {
                    eprintln!("  {n:24} {parsed:>4} decls  resolve ERRORED: {}", e.reason);
                    unreached += 1;
                    continue;
                }
                // NAME THE CAUSE. "not reached (resolve)" says a file did not
                // arrive and not why, and the two causes need different work:
                // an undeclared name is the program's, a refused cross-module
                // member is the STORE's. W-223 part 2 added the second, so the
                // instrument has to tell them apart or a store bug reads as a
                // corpus bug.
                let why = if it.global("असदस्यमस्ति") == Some(&Value::Bool(true))
                {
                    format!(
                        "member `{}` not declared by module `{}` (line {})",
                        text_global(&it, "असदस्यनाम"),
                        text_global(&it, "असदस्यमण्डल"),
                        it.global("असदस्यपङ्क्ति").and_then(Value::as_int).unwrap_or(0)
                    )
                } else if it.global("अनिर्णीतमस्ति") == Some(&Value::Bool(true))
                {
                    format!(
                        "undeclared `{}` (line {})",
                        text_global(&it, "अनिर्णीतनाम"),
                        it.global("अनिर्णीतपङ्क्ति")
                            .and_then(Value::as_int)
                            .unwrap_or(0)
                    )
                } else {
                    "no refusal recorded".to_string()
                };
                eprintln!("  {n:24} {parsed:>4} decls  not reached (resolve): {why}");
                unreached += 1;
                continue;
            }
        }
        // ZERO THE CHECKER'S COUNTERS FOR THIS SOURCE. W-223 part 2.
        //
        // The comment above used to say `प्रकारपरीक्षकारम्भः` "zeroes them at the
        // start of every typecheck". NOTHING CALLED IT. They read zero per
        // source because the INTERPRETER was fresh — the right fact by the wrong
        // mechanism, and the wrong mechanism is what broke when this census
        // began sharing one interpreter: the counters are globals, nobody reset
        // them between sources, so each source's read was CUMULATIVE and the
        // census summed cumulative reads — a triangular over-count. It measured
        // `unjoined_branches` 337 where the true corpus figure is 53.
        //
        // This is `W-249`'s family from the other side: a counter that does not
        // restart while the thing it counts does.
        it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r.clone()], 5_000_000)
            .expect("प्रकारपरीक्षकारम्भः runs");
        let verdict = it.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000,
        );
        // READ THE BLIND-SPOT COUNTERS WHATEVER THE VERDICT WAS. They record
        // what this pass could not type, which is as real for a file that
        // typechecked as for one that refused — reading them only on failure
        // would report the gaps of the worst files and hide the rest.
        if it.global("अप्रकार्यवाक्यमस्ति") == Some(&Value::Bool(true))
        {
            let line = it
                .global("अप्रकार्यवाक्यस्थान")
                .and_then(Value::as_int)
                .unwrap_or(0);
            untypable_sites.push((n.clone(), line));
        }
        // READ WHATEVER THE VERDICT WAS, for the reason the block above
        // gives: a source that typechecked can still carry one of these, and
        // reading only on refusal would report the worst files and hide the
        // rest.
        let bad_fields = it
            .global("असत्क्षेत्रसंख्या")
            .and_then(Value::as_int)
            .unwrap_or(0);
        if bad_fields > 0 {
            bad_field_sites.push((
                n.clone(),
                bad_fields,
                text_global(&it, "असत्क्षेत्रनाम"),
                text_global(&it, "असत्क्षेत्रवस्तु"),
                it.global("असत्क्षेत्रपङ्क्ति")
                    .and_then(Value::as_int)
                    .unwrap_or(0),
            ));
        }
        for (g, acc) in [
            ("अप्रकार्यशर्तसंख्या", &mut untypable_conds),
            ("अप्रकार्यवाक्यसंख्या", &mut untypable_stmts),
            ("अयुग्मशाखासंख्या", &mut unjoined),
            ("अबूलशर्तसंख्या", &mut nonbool_conds),
            // W-223 part 2's store counters. THE TWO TRUST PATHS ARE READ HERE
            // OR THE REFUSAL MEANS NOTHING: a member is refused only as strongly
            // as the collection that preceded it, and these say how strong it
            // was. Both must be ० in THIS census, which collects every source —
            // a non-zero means a module went uncollected and the census measures
            // less than it claims.
            ("सञ्चितप्रयोगसंख्या", &mut store_found),
            ("असञ्चितप्रयोगसंख्या", &mut trusted_no_collection),
            ("असङ्गृहीतमण्डलसंख्या", &mut trusted_uncollected_module),
            ("अलिखितप्रकारसंख्या", &mut type_unrecorded),
            // W-265's two, read the same way and for the same reason.
            ("रिक्तसञ्चितप्रकारसंख्या", &mut no_type_text),
            ("अन्यमण्डलप्रकारसंख्या", &mut weak_rule),
        ] {
            match it.global(g) {
                None => eprintln!("  {n:24} the global {g} does not exist — THIS CENSUS is broken"),
                Some(v) => *acc += sadhana::t1::nirvahana::Value::as_int(v).unwrap_or(0),
            }
        }
        match verdict {
            Ok(Value::Bool(true)) => {
                typed += 1;
                eprintln!("  {n:24} {parsed:>4} decls  TYPECHECKED");
            }
            Ok(other) => {
                refused += 1;
                // ASK THE CHECKER WHY. A bare `Bool(false)` is what this
                // census printed on its first run and it is not actionable —
                // the same dead end the resolver's `अनिर्णीतनाम` was built to
                // end. `प्रकारदोषमस्ति` gates the record: false means the
                // refusal came from somewhere that does not yet report.
                // THE GLOBAL IS KEYED BARE, NOT MODULE-QUALIFIED. The T1
                // source writes `अर्थॱप्रकारदोषमस्ति` when another module
                // reads it, but `Interpreter::global` looks up the name as
                // declared — `nirvahana.rs:538` inserts the bare name — so a
                // qualified key silently finds nothing. Asking for the
                // qualified name is what the first version of this did.
                //
                // THREE STATES, NOT TWO. `matches!(.., Some(Bool(true)))`
                // collapses "the flag is false" and "there is no such global"
                // into one answer, and the second means the instrument is
                // broken while the first means the refusal came from an
                // uninstrumented path. Telling them apart is the difference
                // between debugging the checker and debugging this test.
                let flag = match it.global("प्रकारदोषमस्ति") {
                    None => "MISSING",
                    Some(Value::Bool(true)) => "set",
                    Some(_) => "clear",
                };
                // W-231's proof: which STATEMENT poisoned the block, by kind
                // and by the location the parser kept on it. A refused routine
                // without a named statement is where this census's trail used
                // to end.
                if matches!(it.global("दुष्टवाक्यमस्ति"), Some(Value::Bool(true)))
                {
                    let k = it
                        .global("दुष्टवाक्यभेद")
                        .and_then(sadhana::t1::nirvahana::Value::as_int)
                        .unwrap_or(-1);
                    // W-240 made the statement's location its first TOKEN (a न६४), so the
                    // instrument's global is an index now; the old octet run is kept readable
                    // for the record (merge 2026-09-04).
                    let at = match it.global("दुष्टवाक्यस्थान") {
                        Some(Value::Int(n)) => format!("token {n}"),
                        Some(Value::Octets(o)) => {
                            String::from_utf8_lossy(o.as_slice()).into_owned()
                        }
                        _ => "<no location>".into(),
                    };
                    let why = it
                        .global("दुष्टकारण")
                        .and_then(sadhana::t1::nirvahana::Value::as_int)
                        .unwrap_or(0);
                    let why = match why {
                        1 => "a condition that types to a real non-बूल type",
                        2 => "two branches that do not meet",
                        3 => "a यावत् body that poisoned",
                        _ => "a path this instrument does not yet tag",
                    };
                    let kind = match k {
                        6 => "यदि",
                        7 => "यावत्",
                        other => return_kind(other),
                    };
                    eprintln!("      poisoned by a {kind} statement: {why} (at {at:?})");
                }
                let why = if flag == "set" {
                    let name = match it.global("प्रकारदोषनाम") {
                        Some(Value::Octets(o)) => {
                            String::from_utf8_lossy(o.as_slice()).into_owned()
                        }
                        other => format!("<not a run: {other:?}>"),
                    };
                    let int = |g: &str| {
                        it.global(g)
                            .and_then(sadhana::t1::nirvahana::Value::as_int)
                            .unwrap_or(-1)
                    };
                    format!(
                        "`{name}` body type {} vs declared {}",
                        int("प्रकारदोषशरीरभेद"),
                        int("प्रकारदोषप्रत्यागमनभेद")
                    )
                } else if flag == "MISSING" {
                    "the record global अर्थॱप्रकारदोषमस्ति does not exist — \
                     THIS TEST is broken, not the checker"
                        .to_string()
                } else {
                    format!("refused by an UNINSTRUMENTED path ({other:?})")
                };
                eprintln!("  {n:24} {parsed:>4} decls  refused — {why}");
            }
            Err(e) => {
                broke += 1;
                eprintln!(
                    "  {n:24} {parsed:>4} decls  RUN ERROR {:.140}",
                    format!("{e:?}")
                );
            }
        }
    }
    // WHAT THE CHECKER COULD NOT SEE, as numbers rather than as refusals.
    //
    // `W-202` requires a बूल condition and folds every statement in a block,
    // and both would report the TYPER'S OWN GAPS as corpus defects if they
    // refused what they cannot type: `अभिव्यञ्जकप्रकारः` has no arm for a call,
    // for क्षेत्राभिव्यञ्जक, for `शून्यम्` or for समावेश. Refusing those took this
    // census from 15 of 15 to 6 and not one was a defect. So a condition that
    // types to दोषार्थः is counted here instead of refused, and a NON-TAIL
    // expression statement that does is counted rather than poisoning its
    // block — its value is discarded, so it judged nothing.
    //
    // BOTH ARE `W-231`'s ACCEPTANCE NUMBERS and both go to 0 when those kinds
    // get arms. A rise in either is the checker going blind in a new place.
    eprintln!("METRIC typecheck_untypable_conditions {untypable_conds}");
    eprintln!("METRIC typecheck_untypable_statements {untypable_stmts}");
    eprintln!("METRIC typecheck_unjoined_branches {unjoined}");
    eprintln!("METRIC typecheck_non_bool_conditions {nonbool_conds}");
    eprintln!("METRIC typecheck_store_members_found {store_found}");
    eprintln!("METRIC typecheck_trusted_no_collection {trusted_no_collection}");
    eprintln!("METRIC typecheck_trusted_uncollected_module {trusted_uncollected_module}");
    eprintln!("METRIC typecheck_store_return_type_unrecorded {type_unrecorded}");
    eprintln!("METRIC typecheck_store_entries_with_no_type_text {no_type_text}");
    eprintln!("METRIC typecheck_type_name_resolved_by_weak_rule {weak_rule}");
    // W-265: WHICH spellings, not only how many. Both runs are image-wide —
    // `निर्णायकारम्भः` restarts the COUNTS because this census sums them per
    // source, and leaves the RUNS standing because a list of names is only
    // useful whole. So these are the distinct spellings over all 19 sources.
    let run = |g: &str| -> Vec<String> {
        match it.global(g) {
            Some(Value::Arena(rows)) => rows
                .borrow()
                .iter()
                .skip(1)
                .map(|v| match v {
                    Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
                    _ => String::new(),
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    let mut weak: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for s in run("अन्यमण्डलप्रकारकोश") {
        if !s.is_empty() {
            weak.insert(s);
        }
    }
    for s in &weak {
        eprintln!("  RESOLVED BY THE WEAK RULE (bare, one collected module declares it): {s}");
    }
    // THE REASON BESIDE THE SPELLING, which is the whole of W-265's record:
    // `अज्ञातप्रकारकारणकोश` is written at the SAME index as the text, so the two
    // runs are read together. A spelling refused for two DIFFERENT reasons is
    // not a duplicate — it is the same name failing in two positions, and
    // collapsing them would hide the second.
    let reasons: Vec<i128> = match it.global("अज्ञातप्रकारकारणकोश")
    {
        Some(Value::Arena(rows)) => rows
            .borrow()
            .iter()
            .skip(1)
            .map(|v| Value::as_int(v).unwrap_or(-1))
            .collect(),
        _ => Vec::new(),
    };
    let mut refusals: std::collections::BTreeSet<(String, i128)> =
        std::collections::BTreeSet::new();
    for (i, s) in run("अज्ञातप्रकारपाठकोश").into_iter().enumerate()
    {
        refusals.insert((s, reasons.get(i).copied().unwrap_or(-1)));
    }
    for (s, code) in &refusals {
        let why = match code {
            1 => "bare; no module binds this spelling",
            2 => "the module is known and declares no such type",
            3 => "an empty spelling",
            4 => "bare; MORE THAN ONE module declares it",
            5 => "a qualified name whose module the store does not hold",
            _ => "a reason this census does not name",
        };
        eprintln!("  REFUSED BY NAME ({why}): {s:?}");
    }
    for (f, line) in &untypable_sites {
        eprintln!("  UNTYPABLE STATEMENT  {f:24} line {line}");
    }
    // PRINTED EVEN WHEN EMPTY, with the scope of the sweep beside it —
    // "no site" and "the instrument saw nothing it could have found a site in"
    // read identically otherwise, and this sweep's reach is genuinely small.
    if bad_field_sites.is_empty() {
        eprintln!(
            "\n  BAD FIELD READS: none in any of {} sources, in the positions \
             this checker types (expression statements and conditions). Field \
             reads inside a return, a चरः initialiser or an assignment's right \
             side are NOT typed and are not covered by this line.",
            names.len()
        );
    } else {
        for (f, count, field, owner, line) in &bad_field_sites {
            eprintln!(
                "  BAD FIELD READ  {f:24} {count} site(s), first: `{field}` \
                 not declared by `{owner}` at line {line}"
            );
        }
    }
    eprintln!(
        "METRIC typecheck_bad_field_sources {}",
        bad_field_sites.len()
    );
    eprintln!(
        "\n  TYPECHECK: {typed} typechecked, {refused} refused, {broke} run-error, \
         {unreached} not reached (earlier stage), of {} files",
        names.len()
    );
}

/// A ROUTINE ENDING IN A RETURN TYPECHECKS, AND ONE WITH A REAL MISMATCH STILL
/// DOES NOT.
///
/// `वाक्यप्रकारः` gained its `प्रत्यागमनवाक्यभेद` arm because
/// [`measure_corpus_typecheck`] found SIX of fifteen sources refused with the
/// same `Ty::Error` against their declared return: a body ending in a return
/// had no arm and fell to the poison at the bottom of that routine.
///
/// # Why the second half of this test is the important half
///
/// An arm that answers `अभावार्थः` makes `प्रत्यागमनसाम्यम्`'s Never escape
/// fire, and an escape that fires for EVERYTHING is not a type checker — it is
/// a pass that reports success. So the mismatch case is asserted too: a body
/// whose last statement is an EXPRESSION of the wrong type must still be
/// refused. Without it this test would pass just as happily against
/// `प्रत्यागमनसाम्यम्` returning `सत्यम्` unconditionally, which is the shape
/// of vacuous guard this suite has been bitten by before.
#[test]
fn a_body_ending_in_a_return_typechecks_and_a_real_mismatch_still_refuses() {
    // Its own loader: `load_sema_with_parser`'s module list is what other
    // census tests are keyed to, and this needs exactly the same set, so it
    // reuses rather than widens.
    let mut it = load_sema_with_parser();

    // `ददाति न६४` with `प्रत्यागमनम् ० ।` — THE `मध्यरूप ॱ आरम्भः` SHAPE, and
    // the one that shows why a return may not be typed as its value: the
    // numeral types as अ३२-signed, which is not न६४.
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert!(
        parsed > 0,
        "the program declared nothing; the test is empty"
    );
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver starts");
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r, Value::Int(parsed)],
            4_000_000_000
        )
        .expect("resolve runs"),
        Value::Bool(true),
        "the program must RESOLVE before its typecheck result means anything"
    );
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000
        )
        .expect("typecheck runs"),
        Value::Bool(true),
        "a routine ending in `प्रत्यागमनम् ० ।` must typecheck: the return \
         DIVERGES, so there is no value to disagree with `न६४` about. Before \
         the `प्रत्यागमनवाक्यभेद` arm this refused, and so did six of the \
         fifteen corpus sources"
    );

    // THE OTHER HALF. Body's last statement is an EXPRESSION — a string
    // literal, which types as a slice — against a declared `न६४`. No return,
    // so no Never, so the escape must not save it.
    let mut it2 = load_sema_with_parser();
    let bad = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    उक्तम् क इति ।\nइति\n";
    let toks2 = it2
        .call("पदविभागॱपदविभाग", vec![octets(bad)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed2 = it2
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks2)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert!(parsed2 > 0, "the mismatch program declared nothing");
    let r2 = it2
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver starts");
    let resolved2 = it2.call(
        "अर्थॱकार्यक्रमनिर्णयः",
        vec![r2, Value::Int(parsed2)],
        4_000_000_000,
    );
    assert_eq!(
        resolved2.expect("resolve runs"),
        Value::Bool(true),
        "the mismatch program must resolve, or its typecheck answer is about \
         the wrong thing"
    );
    assert_eq!(
        it2.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed2)],
            4_000_000_000
        )
        .expect("typecheck runs"),
        Value::Bool(false),
        "a body whose last statement is a STRING against a declared `न६४` must \
         still be REFUSED. If this passes, the Never escape is firing for \
         everything and the checker reports success rather than checking"
    );
}

/// FOUR `अन्यथा` SHAPES PARSE AND RESOLVE, WHICH IS HOW WE KNOW THE ELSE
/// CONSTRUCT IS NOT `sanskrit_text.t1`'s DEFECT.
///
/// The resolve census reports that file refusing an undeclared `अन्यथा` — a
/// FROZEN KEYWORD in expression position, which can only mean the parser left
/// it there. This test was written to reproduce that and DOES NOT: every shape
/// below is accepted. That is the finding, not a failure to find one.
///
/// # What it cost to learn, and why the line number was the fix
///
/// The file writes `अन्यथा` eighteen times and the refusal record named only
/// the WORD. Two hypotheses were formed and both were wrong: first the one
/// occurrence whose `इति` sits on the previous line (`:219` — parses fine),
/// then "the body must end in a call". Adding `अनिर्णीतपङ्क्ति` to the
/// resolver pointed at `:813`, whose `यदि` body is an ASSIGNMENT — so the
/// third and fourth shapes below were written for that, and they pass too.
///
/// SO THE CAUSE IS EARLIER IN THE FILE AND `:813` IS WHERE THE DAMAGE
/// SURFACES. A parser that has lost alignment reports the first token it
/// cannot place as a name, and a keyword is simply the first such token that
/// is obviously wrong. Chasing the symptom's shape was the wrong search; the
/// next move is to find where the statement stream diverges, not what
/// `अन्यथा` looks like.
///
/// Kept as a regression guard: these four shapes must keep working, and if one
/// ever breaks, the else construct really is at fault and this says so
/// immediately.
#[test]
fn the_else_construct_parses_and_resolves_in_all_four_shapes_the_corpus_writes() {
    let flat = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
                    यदि य समम् सत्यम् आदि\n        प्रत्यागमनम् १ ।\n    इति अन्यथा आदि\n\
                        प्रत्यागमनम् २ ।\n    इति\nइति\n";
    let nested = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
                    यदि य समम् सत्यम् आदि\n        यदि य समम् असत्यम् आदि\n\
                            प्रत्यागमनम् १ ।\n        इति\n    इति\n    अन्यथा आदि\n\
                        प्रत्यागमनम् २ ।\n    इति\nइति\n";

    // The shape `sanskrit_text.t1:811-813` actually uses: the `यदि` body is an
    // ASSIGNMENT, not a return. Two wrong guesses cost time here — first the
    // one `अन्यथा` whose `इति` sits on the previous line (line 219, which
    // parses fine), then "it must end in a call". The census's LINE number is
    // what finally pointed at 813, and these shapes are what separate the
    // statement forms rather than reasoning about them.
    let assign = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
                      चरः स ॱॱ न६४ भवति ० ।\n    यदि य समम् सत्यम् आदि\n        स भवति १ ।\n\
                      इति अन्यथा आदि\n        स भवति २ ।\n    इति\n    प्रत्यागमनम् स ।\nइति\n";
    let assign_nested = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
                      चरः स ॱॱ न६४ भवति ० ।\n    यदि य समम् सत्यम् आदि\n        स भवति १ ।\n\
                      इति अन्यथा आदि\n        यदि य समम् असत्यम् आदि\n            स भवति २ ।\n\
                      इति अन्यथा आदि\n            स भवति ३ ।\n        इति\n    इति\n\
                      प्रत्यागमनम् स ।\nइति\n";

    // THE SHAPE `sanskrit_text.t1:811` WRITES, and the one thing in
    // `अक्षरदर्शकवर्धनम्` that no other condition in it does: a JUXTAPOSED CALL
    // followed by a BINARY OPERATOR — `यदि चित्राक्षरम् सङ्केताङ्क समम् सत्यम् आदि`.
    // Every other `यदि` in that routine compares a plain variable. If the
    // argument loop does not stop at `समम्`, the condition swallows the
    // comparison and the `आदि` with it, and the block structure comes apart —
    // which would leave the `अन्यथा` on the next line in expression position
    // with NO parse error recorded, exactly what W-186 measured.
    let call_then_operator = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ह आदाय अ ॱॱ न६४ ददाति बूल आदि\n\
             प्रत्यागमनम् सत्यम् ।\nइति\n\
             सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
             यदि ह ० समम् सत्यम् आदि\n        प्रत्यागमनम् १ ।\n\
             इति अन्यथा आदि\n        प्रत्यागमनम् २ ।\n    इति\nइति\n";

    for (label, src) in [
        (
            "CALL then binary operator in a condition",
            call_then_operator,
        ),
        ("flat if/else, body is a return", flat),
        ("nested then, else", nested),
        ("body is an ASSIGNMENT", assign),
        ("assignment, else holds a nested if/else", assign_nested),
    ] {
        let mut it = load_sema_with_parser();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .expect("parses")
            .as_int()
            .unwrap_or(0);
        assert!(parsed > 0, "{label}: parsed nothing");
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("resolver starts");
        let got = it
            .call(
                "अर्थॱकार्यक्रमनिर्णयः",
                vec![r, Value::Int(parsed)],
                4_000_000_000,
            )
            .expect("resolve runs");
        let refused = match it.global("अनिर्णीतमस्ति") {
            Some(Value::Bool(true)) => match it.global("अनिर्णीतनाम") {
                Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
                _ => "<no name>".into(),
            },
            _ => "<none>".into(),
        };
        assert_eq!(
            got,
            Value::Bool(true),
            "{label}: refused `{refused}`. A frozen keyword in expression \
             position means the parser left it there — `अन्यथा` is an else, \
             not a name"
        );
    }
}

/// A STRING LITERAL CONTAINING A DOUBLED `इति` PARSES AS ONE STRING — ADR-0011
/// in the SELF-HOSTED parser, which is where it was never implemented.
///
/// # Why this is the `parse.t1` refusal
///
/// The resolve census refuses `parse.t1` with an undeclared `इति` at `:512`,
/// and a frozen keyword in expression position can only mean the parser left
/// it there. Rust's `lex_t1` reads that line as ONE `Kind::Str`, so the Rust
/// side is not at fault — but the census runs the corpus's OWN lexer, and
/// `lex.t1` DECLARES `शब्दभेद` (Kind::Str) and never once assigns it. The
/// self-hosted lexer emits no string tokens at all.
///
/// That is not itself a bug: ADR-0017 records that T0 spells a string as a
/// PHRASE and its parser reassembles the literal from words, and `parse.t1`
/// does the same at `:508-516` — match the word `उक्तम्`, then scan forward for
/// the closing `इति`.
///
/// THE SCANNER CLOSES ON THE FIRST `इति` AND HAS NO DOUBLING RULE. ADR-0011
/// says a DOUBLED `इति` is the literal word and a THIRD one closes, which is
/// exactly what `parse.t1:512` writes — `उक्तम् इति इति इति` is the literal
/// `"इति"`. A scanner without the rule ends the string at the first `इति`,
/// leaving `इति इति` behind as bare words, and the resolver then reports the
/// first of them as an undeclared name. The corpus's own operator table is
/// written this way, so this is not an exotic case.
///
/// # The two halves
///
/// A doubled `इति` must parse AND an ordinary single-`इति` string must still
/// close where it always did — a doubling rule that never terminates is the
/// opposite failure and would hang or swallow the rest of the file.
#[test]
fn a_string_literal_containing_a_doubled_iti_is_one_string_per_adr_0011() {
    for (label, src, decls) in [
        (
            "ordinary string, single इति closes",
            "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः स ॱॱ न६४ भवति ० ।\n\
             प्रत्यागमनम् स ।\nइति\n",
            1,
        ),
        (
            "ADR-0011 in a DECLARATION initialiser",
            "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
             चरः स ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् इति इति इति ।\n    प्रत्यागमनम् ० ।\nइति\n",
            1,
        ),
        // THE SHAPE `parse.t1:512` ACTUALLY WRITES: the literal is an ARGUMENT
        // inside a call group `आरभ्य … समाप्तम्`, not a declaration's value.
        //
        // THE DECLARATION FORM ABOVE PASSES FOR THE WRONG REASON, AND READING
        // THAT AS "the scanner is fine, the group path is at fault" WAS WRONG.
        // `parse.t1:508-516` is the whole scanner: it remembers the start, then
        // returns at the FIRST token whose text is `इति`. There is no doubling
        // rule anywhere in it — only a margin at :511 that STATES ADR-0011
        // ("इति इति इति is the string's end; the pair is itself a word") above
        // code that never implemented it.
        //
        // So `उक्तम् इति इति इति` ends at the first `इति` in BOTH forms, and
        // both leave `इति इति` behind as loose tokens. In a declaration those
        // leftovers happen to act as harmless block-closers and the program
        // still resolves; inside `आरभ्य … समाप्तम्` they land in expression
        // position and the resolver refuses the first as an undeclared name.
        // The declaration case is a FALSE NEGATIVE — it asserts a count and a
        // resolve verdict, neither of which can see a swallowed literal — and
        // it should be strengthened to assert the string's own extent once the
        // scanner is fixed.
        (
            "ADR-0011 as an ARGUMENT inside आरभ्य … समाप्तम्",
            "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ह आदाय अ ॱॱ अङ्कः अन्तः अ८ ददाति बूल आदि\n\
             प्रत्यागमनम् सत्यम् ।\nइति\n\
             सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
             यदि ह आरभ्य उक्तम् इति इति इति समाप्तम् समम् सत्यम् आदि\n\
             प्रत्यागमनम् १ ।\n    इति\n    प्रत्यागमनम् ० ।\nइति\n",
            2,
        ),
    ] {
        let mut it = load_sema_with_parser();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .expect("parses")
            .as_int()
            .unwrap_or(0);
        assert_eq!(parsed, decls, "{label}: wrong declaration count");
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("resolver starts");
        let got = it
            .call(
                "अर्थॱकार्यक्रमनिर्णयः",
                vec![r, Value::Int(parsed)],
                4_000_000_000,
            )
            .expect("resolve runs");
        let refused = match it.global("अनिर्णीतमस्ति") {
            Some(Value::Bool(true)) => match it.global("अनिर्णीतनाम") {
                Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
                _ => "<no name>".into(),
            },
            _ => "<none>".into(),
        };
        assert_eq!(
            got,
            Value::Bool(true),
            "{label}: refused `{refused}`. A `इति` left in expression position \
             means the string scanner closed early — ADR-0011's doubled `इति` \
             is the literal word, not the terminator"
        );
    }
}

/// THE PARSER'S ERROR ARENA CAN ACTUALLY MOVE — which is what makes the zero
/// in [`measure_corpus_resolve`] a finding rather than a blind instrument.
///
/// That census now reads `दोषसूचकाङ्क` and reports every parse error the
/// parser recorded, and the answer for ALL FIFTEEN corpus sources is ZERO —
/// including the four that go on to leave a FROZEN KEYWORD in expression
/// position (`अन्यथा`, `इतिशब्दः`, and before W-185 `इति`). A count of zero is
/// only evidence if the counter can be made non-zero, and a counter nothing
/// increments reads zero forever.
///
/// So this feeds `व्याकर` a program that IS malformed — a group opened with
/// `आरभ्य` and never closed — and asserts the arena moved and carries a
/// reason. `दोषयोजनम्` (parse.t1:239) advances `दोषसूचकाङ्क` and writes at the
/// new index, so a recorded error lands at 1 and the count is 1.
///
/// # What the zero then means, and it is not good news
///
/// The parser does not fail on those four files. It SUCCEEDS WRONGLY: it
/// builds a tree in which a keyword sits where an expression should be, and
/// reports no error at all. `अर्थ` is the first pass that notices, which is
/// why every one of these arrived as "undeclared name" rather than as a syntax
/// error, and why chasing the keyword's SHAPE was the wrong search each time.
#[test]
fn the_parsers_error_arena_moves_when_the_program_is_actually_malformed() {
    let mut it = load_sema_with_parser();

    // `आरभ्य` opens a group; nothing closes it. parse.t1 answers this with
    // `दोषयोजनम् … समाप्तम् अपेक्षितम्`.
    let bad = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
               प्रत्यागमनम् आरभ्य १ ।\nइति\n";
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(bad)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let _ = it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000);

    let n = match it.global("दोषसूचकाङ्क") {
        None => panic!("`दोषसूचकाङ्क` is not a global — the census that reads it is broken"),
        Some(v) => Value::as_int(v).expect("the error cursor is an integer"),
    };
    assert!(
        n > 0,
        "`व्याकर` recorded NO error for a program with an unclosed `आरभ्य`. \
         Then the zero this census reports for all fifteen corpus sources says \
         nothing at all, and the instrument is blind"
    );

    // And it carries a reason, so a future reader gets more than a count.
    let slots = it.global("दोषकोश").map(arena).unwrap_or_default();
    let Some(Value::Record(rec)) = slots.get(1) else {
        panic!("the error arena is 1-based with slot ० reserved; slot १ is empty")
    };
    let why = match rec.borrow().get("कारण") {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("`कारण` is not a run of octets: {other:?}"),
    };
    assert!(
        !why.is_empty(),
        "the recorded error has an EMPTY reason, so the arena moved but says \
         nothing — a count without a cause is the shape this suite keeps \
         having to repair"
    );
}

/// BISECT A SOURCE BY PREFIX — which routine builds the bad tree?
///
/// `measure_corpus_resolve` names a file, a refused name and a line.
/// `W-186` established that the parser records NO error while producing that
/// tree, so there is nothing to grep for: the only way to localise it is to
/// feed `व्याकर` less of the file and see when the refusal appears.
///
/// Cutting at a top-level `इति` keeps the prefix syntactically whole and keeps
/// every earlier declaration, so a name defined above the cut still resolves.
/// A cut that lands mid-routine shows up as a PARSE ERROR rather than as a
/// silent refusal, which the report distinguishes.
///
/// `SANSOS_BISECT=<file>:<line>,<file>:<line>` selects the cuts.
#[test]
#[ignore = "measurement"]
fn bisect_a_source_by_prefix() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let spec = std::env::var("SANSOS_BISECT").unwrap_or_else(|_| {
        // `अक्षरदर्शकवर्धनम्` spans 778..839 and holds the refused `अन्यथा` at
        // 813; 777 is the line before it opens.
        "sanskrit_text.t1:777,sanskrit_text.t1:839,sanskrit_text.t1:2000".to_string()
    });
    for one in spec.split(',') {
        let (file, cut_spec) = one.split_once(':').expect("<file>:<line>");
        let cut: usize = cut_spec.parse().unwrap_or(0);
        let whole = std::fs::read_to_string(dir.join(file)).expect("read the source");
        let lines: Vec<&str> = whole.lines().collect();
        // A PREFIX CANNOT ISOLATE A LATE REFUSAL, because the record keeps only
        // the FIRST one: cutting `sanskrit_text.t1` at 839 reports
        // `सङ्ख्यामूलम्` at line 23 — a forward reference the cut removed — and
        // the `अन्यथा` at 813 never gets a chance to be named.
        //
        // So a cut may also be a SLICE: `<file>:<start>-<end>` keeps the module
        // header (the first `head` lines) and then only that range, which drops
        // the earlier refusals along with everything else that is not needed.
        let (take, src) = if let Some((a, b)) = cut_spec.split_once('-') {
            let (a, b): (usize, usize) = (a.parse().unwrap(), b.parse().unwrap());
            let head = 3.min(lines.len());
            let body: Vec<&str> = lines[..head]
                .iter()
                .chain(lines[a.saturating_sub(1)..b.min(lines.len())].iter())
                .copied()
                .collect();
            (b, body.join("\n") + "\n")
        } else {
            let take = cut.min(lines.len());
            (take, lines[..take].join("\n") + "\n")
        };

        let mut it = load_sema_with_parser();
        let toks = match it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(e) => {
                eprintln!("  {file} 1..{take}: DOES NOT LEX {:.90}", format!("{e:?}"));
                continue;
            }
        };
        let parsed = match it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(e) => {
                eprintln!("  {file} 1..{take}: PARSE RAN OUT {:.90}", format!("{e:?}"));
                continue;
            }
        };
        let perr = it.global("दोषसूचकाङ्क").and_then(Value::as_int).unwrap_or(-1);
        let r = match it.call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        {
            Ok(v) => v,
            Err(_) => {
                eprintln!("  {file} 1..{take}: resolver would not start");
                continue;
            }
        };
        let verdict = it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r, Value::Int(parsed)],
            4_000_000_000,
        );
        let named = match it.global("अनिर्णीतमस्ति") {
            Some(Value::Bool(true)) => {
                let nm = match it.global("अनिर्णीतनाम") {
                    Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
                    _ => "<no name>".into(),
                };
                let ln = it
                    .global("अनिर्णीतपङ्क्ति")
                    .and_then(Value::as_int)
                    .unwrap_or(0);
                format!("`{nm}` at line {ln}")
            }
            _ => "-".to_string(),
        };
        eprintln!(
            "  {file} 1..{take:<5} {parsed:>4} decls  parse-errs {perr:>2}  {:?}  refused {named}",
            verdict
                .map(|v| format!("{v:?}"))
                .unwrap_or_else(|e| format!("{e:?}"))
        );
    }
}

/// ITS OWN LOADER, because a loader is part of the test.
///
/// `मध्यरूप` imports `वास्तु`, `व्याकर`, `पदविभाग` and `अक्षरकोश` (`ir.t1:61`),
/// and the chain census below also resolves and typechecks, so `अर्थ` comes
/// too. Widening `load_sema_with_parser` instead would have moved a module
/// count that another census is keyed to.
fn load_ir_chain() -> Interpreter {
    load_all(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "artha.t1",
        "sanchaya.t1",
        "sanskrit_text.t1",
        "ir.t1",
    ])
}

/// THE STAGE AFTER TYPECHECK — does `मध्यरूप` build IR for the whole corpus?
///
/// lex → parse → resolve → typecheck all read 15/15 (`W-191`). The owner's
/// chain names ENCODE as the last stage, but `सङ्केतन` takes a
/// `वाक्यविभागॱकार्यक्रम` — a T0 assembly program — so it is not adjacent to
/// typecheck at all: IR construction and codegen sit between them. This
/// measures the FIRST of those two, which is one call
/// (`कार्यक्रमरचना(घोषणासंख्यान)`) and therefore the same shape of increment
/// that took typecheck from unmeasured to 15/15.
///
/// # Three states, not two
///
/// A file that never reached this stage is NOT a file that failed it, and a
/// build that runs but produces NO INSTRUCTIONS is not a build that worked.
/// All three are reported separately, and the instruction count is printed for
/// every file so a zero cannot hide inside a success.
#[test]
#[ignore = "measurement"]
fn measure_corpus_ir() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();

    let (mut built, mut empty, mut broke, mut unreached, mut nofns) = (0, 0, 0, 0, 0);
    // `W-289` — THE BY-CAUSE STUB TABLE, READ HERE BECAUSE IT IS COMPLETE HERE.
    //
    // `अपूर्णगणनाकोश` is indexed BY CAUSE and holds the count; `मध्यरूपॱआरम्भः`
    // clears it per source, so the sums are taken per source and accumulated on
    // this side. `रचितगणनाकोश` is the same shape for lowered shapes.
    //
    // WHY THIS IS THE RIGHT PLACE AND NOT A NEW INSTRUMENT: the counters are
    // COMPLETE once the IR is built. Emit, assemble, link and run contribute
    // nothing to them — `read_module`'s own margin says so — and the encode
    // census pays for all four. Measured: this pass is **313 s** against the
    // census's **~1,892 s**, a 6x cut, and it was already building the IR for
    // all twenty sources and throwing the table away.
    //
    // AND IT IS UNBLOCKABLE, WHICH IS THE POINT RATHER THAN THE SPEED. The
    // encode census reaches this table only after a twin comparison and four
    // pins; when any of those trips, the figure is lost and the run says
    // nothing about the corpus. This test has no such guard ahead of it, so the
    // by-cause number survives a red anywhere else in the tree.
    let mut causes: BTreeMap<i128, i128> = BTreeMap::new();
    let mut shapes: BTreeMap<i128, i128> = BTreeMap::new();
    // ONE interpreter with the store filled (W-223 part 2). `load_ir_chain`
    // already carries `sanchaya.t1`; the collection pass and the flag are what
    // let the resolver write a symbol for a cross-module callee, and the two
    // refusals this census names — `samyojana.t1:870`, `vishlesana.t1:615` —
    // are calls `ir.t1` refused because that symbol was ०.
    let mut it = load_ir_chain();
    for n in &names {
        let Ok(src) = std::fs::read_to_string(dir.join(n)) else {
            continue;
        };
        let Ok(t) = it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        else {
            continue;
        };
        let t = t.as_int().unwrap_or(0);
        if it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(t)], 4_000_000_000)
            .is_ok()
        {
            let _ = it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 2_000_000_000);
        }
    }
    it.call("अर्थॱसञ्चयसिद्धिः", vec![], 5_000_000)
        .expect("सञ्चयसिद्धिः runs");
    for n in &names {
        let src = std::fs::read_to_string(dir.join(n)).unwrap();
        let toks = match it.call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(_) => {
                eprintln!("  {n:24} not reached (lex)");
                unreached += 1;
                continue;
            }
        };
        let parsed = match it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        {
            Ok(v) => v.as_int().unwrap_or(0),
            Err(e) => {
                // SAY WHY. `measure_corpus_resolve` parses all fifteen of these
                // and this census parses none of them, and the ONLY difference
                // between the two is the loader — so the error text is the
                // evidence, not the fact of failing.
                eprintln!("  {n:24} not reached (parse): {:.100}", format!("{e:?}"));
                unreached += 1;
                continue;
            }
        };
        if parsed == 0 {
            eprintln!("  {n:24} not reached (parse produced no declarations)");
            unreached += 1;
            continue;
        }

        // RESOLVE BEFORE LOWERING — `W-204`. The chain is lex → parse →
        // resolve → typecheck → IR (research/22 §4.4), and since `W-183` the
        // IR reads the symbol the resolver wrote onto each name node: a call
        // whose callee carries none is REFUSED. Lowering an unresolved
        // program would therefore refuse every file at its first call and
        // measure the census, not the builder.
        let resolved = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .and_then(|r| {
                it.call(
                    "अर्थॱकार्यक्रमनिर्णयः",
                    vec![r, Value::Int(parsed)],
                    4_000_000_000,
                )
            });
        match resolved {
            Ok(Value::Bool(true)) => {}
            Ok(other) => {
                eprintln!("  {n:24} not reached (resolve answered {other:?})");
                unreached += 1;
                continue;
            }
            Err(e) => {
                eprintln!("  {n:24} not reached (resolve): {:.100}", format!("{e:?}"));
                unreached += 1;
                continue;
            }
        }

        if it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000).is_err() {
            eprintln!("  {n:24} मध्यरूपॱआरम्भः would not run");
            broke += 1;
            continue;
        }
        match it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
        {
            Err(e) => {
                eprintln!(
                    "  {n:24} {parsed:>4} decls  IR RUN ERROR {:.80}",
                    format!("{e:?}")
                );
                broke += 1;
            }
            Ok(_) => {
                // THE COUNT IS PRINTED EVEN WHEN IT IS ZERO. A builder that
                // runs cleanly and appends nothing looks identical to one that
                // works, and that is exactly the shape this suite keeps having
                // to repair.
                let ins = match it.global("आज्ञासूचकाङ्क") {
                    None => {
                        eprintln!(
                            "  {n:24} the global आज्ञासूचकाङ्क does not exist — THIS \
                             CENSUS is broken, not मध्यरूप"
                        );
                        broke += 1;
                        continue;
                    }
                    Some(v) => Value::as_int(v).unwrap_or(-1),
                };
                // A FOURTH STATE — `W-204`: the builder ran and REFUSED a call
                // whose callee carries no symbol. Reported with the callee's
                // line, because a refusal that cannot be located is barely a
                // diagnostic; counted as broke, since the build is unusable.
                let refused = matches!(it.global("अनिर्णीताह्वानमस्ति"), Some(Value::Bool(true)));
                if refused {
                    let line = it
                        .global("अनिर्णीताह्वानचिह्नकाङ्क")
                        .and_then(Value::as_int)
                        .filter(|idx| *idx > 0)
                        .and_then(|idx| {
                            let tokens = arena(it.global("चिह्नककोश")?);
                            let tok = tokens.get(usize::try_from(idx).ok()?)?;
                            Value::as_int(&member(tok, "पङ्क्ति"))
                        })
                        .unwrap_or(0);
                    eprintln!(
                        "  {n:24} {parsed:>4} decls  IR REFUSED — a call's callee has no \
                         symbol, line {line}; {ins} instructions before it"
                    );
                    broke += 1;
                    continue;
                }
                // HOW MANY OF THEM ARE CALLS, so the row that lowered calls
                // can show its counts moved only where calls exist.
                let calls = arena(it.global("आज्ञाकोश").expect("आज्ञाकोश is a global"))
                    .iter()
                    .skip(1)
                    .filter(|e| matches!(e, Value::Record(_)))
                    .filter(|e| Value::as_int(&member(e, "भेद")) == Some(2))
                    .count();
                if ins > 0 {
                    eprintln!(
                        "  {n:24} {parsed:>4} decls  IR BUILT   {ins:>6} instructions, \
                         {calls:>4} calls"
                    );
                    built += 1;
                    // `W-289`: accumulate this source's stub causes and lowered
                    // shapes. Read only for a source that BUILT — a source that
                    // did not reach the builder has an arena cleared by its own
                    // `आरम्भः` and contributes zeros that would read as absence.
                    for (arena_name, into) in
                        [("अपूर्णगणनाकोश", &mut causes), ("रचितगणनाकोश", &mut shapes)]
                    {
                        if let Some(v) = it.global(arena_name) {
                            for (k, e) in arena(v).iter().enumerate() {
                                if let Some(c) = Value::as_int(e)
                                    && c > 0
                                {
                                    *into.entry(k as i128).or_insert(0) += c;
                                }
                            }
                        }
                    }
                } else if !src.contains("वृत्तिः ") {
                    // A THIRD STATE. `ast.t1`, `kosha.t1`, `lib.t1` and
                    // `vastu.t1` declare records, enums and constants and NO
                    // routines at all, so building nothing is CORRECT for them.
                    // Counting that as `empty` alongside a file whose bodies
                    // failed to build reports four healthy files as a shortfall
                    // — the same two-states-for-three-truths shape this suite
                    // keeps having to repair.
                    eprintln!("  {n:24} {parsed:>4} decls  no routines — nothing to build");
                    nofns += 1;
                } else {
                    eprintln!("  {n:24} {parsed:>4} decls  IR EMPTY — ran, appended nothing");
                    empty += 1;
                }
            }
        }
    }
    eprintln!(
        "  IR: {built} built, {empty} EMPTY WITH ROUTINES, {nofns} no routines, \
         {broke} run-error, {unreached} not reached, of {} files",
        names.len()
    );

    // `W-289` — THE TABLE, PRINTED BEFORE THE ASSERTIONS BELOW AND NOT AFTER.
    // The same ordering rule the encode census had to be taught: a diagnostic
    // that prints after a guard is a diagnostic the guard can delete.
    let total: i128 = causes.values().sum();
    println!("METRIC paradigm_ir_stubs_cheap {total}");
    for (cause, n) in &causes {
        println!("METRIC paradigm_ir_stub_cause_{cause} {n}");
    }
    for (shape, n) in &shapes {
        println!("METRIC paradigm_ir_lowered_shape_{shape} {n}");
    }
    println!("METRIC paradigm_ir_cheap_sources_built {built}");

    // ══ THE NON-VACUITY GUARD, AND IT IS OWED ON DAY ONE ══
    //
    // **A RUNG WHOSE ABSENCE IS INDISTINGUISHABLE FROM ITS SUCCESS IS NOT A
    // RUNG.** This test is `#[ignore]`d, so a whole-crate run never reaches it
    // and reports green; and `cargo test -- --ignored <name>` that matches
    // NOTHING also exits 0 — the same mechanism that let a census match no test
    // and report success twice in one evening. Both failures are silent and
    // both look exactly like a pass.
    //
    // So this test asserts what it MEASURED, not merely that it ran: sources
    // actually built, and the arena actually held something. A zero here is a
    // broken census, not a clean corpus — the corpus has never had zero stubs
    // and the day it does, this assertion is the one to revisit deliberately.
    assert!(
        built >= 10,
        "only {built} of {} sources built IR — this census is broken, not the builder",
        names.len()
    );
    assert!(
        total > 0,
        "the stub arena summed to ZERO over {built} built sources; \
         `अपूर्णगणनाकोश` was not read, not that the corpus has no stubs"
    );
}

/// A BARE CALL MUST MEAN THE CALLER'S OWN MODULE — parse-time arity and
/// run-time dispatch have to agree, and they did not.
///
/// `resolve_call` (`nirvahana.rs:1096`) looks in `self.modules[from]` FIRST,
/// so at run time a bare name means the caller's own routine. But the BODY
/// PARSER decides how many juxtaposed arguments a call takes by looking the
/// bare name up in a FLAT `sigs` map (`:1754`) that has no idea which module it
/// is parsing — `sigs.insert(f.name.clone(), …)` at `:457` lets whichever
/// module loaded last own the bare key.
///
/// So when two modules both export a name with DIFFERENT arities, the parser
/// consumes the wrong number of arguments and everything after it misaligns.
/// That is what `measure_corpus_ir` hit: loading `मध्यरूप` beside `अक्षरकोश`
/// made twelve of fifteen corpus files fail to parse with
/// `sanskrit_text.t1:21: सङ्ख्या: अङ्कमूल्यम् …` — `अक्षरकोश` calling its OWN
/// two-argument `अङ्कमूल्यम्` and being handed `मध्यरूप`'s arity of one.
///
/// # The case that must still be refused
///
/// Preferring the caller's module must NOT break reaching across on purpose.
/// The second half asserts that a QUALIFIED call still lands in the other
/// module — otherwise this fix would trade a silent collision for a silent
/// isolation, which is not better.
#[test]
fn a_bare_call_resolves_to_the_callers_own_module_and_qualified_still_crosses() {
    // `क` and `ख` both export `साधारणम्`, with DIFFERENT arities: two and one.
    let ka = "मण्डलम् क ॥\n\
              सार्वजनिक वृत्तिः साधारणम् आदाय अ ॱॱ न६४ ऽ आ ॱॱ न६४ ददाति न६४ आदि\n\
              प्रत्यागमनम् अ योगः आ ।\nइति\n\
              ॰ a BARE call to a name both modules export. Two arguments, which\n\
              ॰ is this module's own arity — under a flat map it would be read\n\
              ॰ with ख's arity of one and the second argument left stranded.\n\
              सार्वजनिक वृत्तिः परीक्षा ददाति न६४ आदि\n\
              प्रत्यागमनम् साधारणम् ३ ४ ।\nइति\n";
    let kha = "मण्डलम् ख ॥\n\
               सार्वजनिक वृत्तिः साधारणम् आदाय अ ॱॱ न६४ ददाति न६४ आदि\n\
               प्रत्यागमनम् अ गुणनम् १० ।\nइति\n";

    let it = Interpreter::load(&[("ka.t1", ka), ("kha.t1", kha)], &spec_root())
        .expect("two modules that share a routine name still load");
    let mut it = it;

    let got = it
        .call("कॱपरीक्षा", vec![], 5_000_000)
        .expect("परीक्षा runs")
        .as_int()
        .expect("it answers an integer");
    assert_eq!(
        got, 7,
        "`साधारणम् ३ ४` inside module क must call क's OWN two-argument routine \
         and answer ३+४. Answering {got} means the parser took ख's arity of \
         one, consumed a single argument, and the call misaligned"
    );

    // AND REACHING ACROSS ON PURPOSE STILL WORKS. If this fails, the fix has
    // isolated the modules instead of disambiguating them.
    let crossed = it
        .call("खॱसाधारणम्", vec![Value::Int(5)], 5_000_000)
        .expect("a qualified call still reaches the other module")
        .as_int()
        .expect("it answers an integer");
    assert_eq!(
        crossed, 50,
        "`खॱसाधारणम् ५` must reach ख's one-argument routine and answer ५×१०"
    );
}

/// A BLOCK MUST EMIT EVERY DIRECT CHILD, IN WRITTEN ORDER, AND EACH ONE ONCE.
///
/// `ir.t1:386`'s `समूह` arm returned `वाक्यरचना वाक्यम् ॱ दक्षिणसूचकाङ्क` — the
/// LAST statement alone. `वास्तु` records a block as `वाम`(प्रथम)..`दक्षिण`
/// (अन्तिम), so every earlier statement was dropped, and `measure_corpus_ir`
/// reported all fifteen corpus files as `IR EMPTY — ran, appended nothing`.
///
/// The walk is `अर्थ`'s, not a new one: `artha.t1:990` already solved this and
/// the corpus resolves 15/15 on it. `आदिसूचकाङ्क` permits only BACKWARD skips —
/// a child knows where its own subtree starts, never where the next one begins,
/// and a next-sibling index cannot be computed at push time. So written order
/// needs a scan from `अन्तिम` down keeping the smallest root still above the
/// last visited. Quadratic in a block's direct children, and blocks have few.
///
/// # The case that must still be refused
///
/// A NESTED block's statements lie INSIDE its parent's range, because
/// `वाक्ययोजनम्` numbers a statement AFTER its children. Iterating the range
/// naively emits them TWICE — the defect `ast.t1:148` warns about by name. The
/// third body below wraps its statement in a nested block and must emit exactly
/// what the flat one-statement body emits, not double.
#[test]
fn a_block_emits_each_direct_child_once_and_a_nested_block_does_not_double() {
    fn instructions(src: &str) -> i128 {
        let mut it = load_ir_chain();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .expect("parses")
            .as_int()
            .unwrap_or(0);
        assert!(parsed > 0, "the fixture must produce declarations");
        it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
            .expect("आरम्भः runs");
        it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
            .expect("कार्यक्रमरचना runs");
        match it.global("आज्ञासूचकाङ्क") {
            None => panic!("`आज्ञासूचकाङ्क` is not a global — THIS TEST is broken, not मध्यरूप"),
            Some(v) => Value::as_int(v).expect("the instruction cursor is an integer"),
        }
    }

    let one = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    ३ योगः ४ ।\nइति\n";
    let two = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    ३ योगः ४ ।\n    ५ योगः ६ ।\nइति\n";
    let nested = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    यदि सत्यम् आदि\n        ३ योगः ४ ।\n    इति\nइति\n";

    let (n1, n2, nn) = (instructions(one), instructions(two), instructions(nested));

    assert!(
        n1 > 0,
        "a one-statement body emitted {n1} instructions. The `समूह` arm is \
         still returning only `दक्षिणसूचकाङ्क` and the body never got walked"
    );
    assert!(
        n2 > n1,
        "a TWO-statement body emitted {n2} and a one-statement body {n1}. The \
         earlier statement is being dropped — a block must emit every direct \
         child, not just the last"
    );
    // THE REFUSAL. A nested block must not re-emit through its parent's range.
    assert!(
        nn <= n1 + 2,
        "a body whose single statement sits in a NESTED block emitted {nn} \
         against {n1} for the same statement at top level. A nested block's \
         statements lie INSIDE its parent's range, so a naive walk emits them \
         twice — exactly what `ast.t1:148` warns about"
    );
}

/// A `प्रत्यागमनम्` STATEMENT MUST BUILD ITS EXPRESSION.
///
/// `वास्तु` defines eight statement kinds and `वाक्यरचना` handled TWO —
/// expression and block (`W-194`). Everything else fell through to
/// `मूल्याङ्कनम् ०` and emitted nothing, which is why `measure_corpus_ir` found
/// one instruction in the whole corpus: real bodies end in `प्रत्यागमनम्`, and
/// a return built nothing at all.
///
/// `parse.t1:914` writes `वाक्ययोजनम्(प्रत्यागमनवाक्यभेद, मूल्य, ०)` — the
/// returned expression is in `वाम`, the same slot the expression arm already
/// reads, so this arm is complete rather than partial. `कार्यक्रमरचना` then
/// hands the body's value to `अवसानरचना`, so building it here is what makes a
/// function's return value exist at all.
///
/// # What this does NOT prove, measured rather than assumed
///
/// A return of `३ योगः ४` and a return of `३` both emit exactly ONE
/// instruction, and that is not the arm's doing. `अभिव्यञ्जकरचना` (`ir.t1:331`)
/// handles TWO expression kinds — numeral and group — and everything else,
/// including EVERY BINARY OPERATOR, falls through to `:354` and emits a
/// `ध्रुवाज्ञाभेद` holding ०. So an unported expression does not fail; it
/// silently builds a CONSTANT ZERO, which is wrong code rather than missing
/// code, and is the more dangerous of the two.
///
/// That is why this test asserts one instruction and not more. A draft asserted
/// `३ योगः ४` must emit MORE than `३` — a guess about IR shape, and it was
/// wrong: both hit the placeholder. When the binary arms are ported this test
/// SHOULD start failing, and its message says so.
///
/// An earlier draft also guarded a bare `प्रत्यागमनम् ।`. `parse.t1:912` calls
/// `अभिव्यञ्जकपठनम्` unconditionally, so that does not parse and the corpus has
/// no such site — the fixture failed to parse, which is how the invented case
/// was caught before it landed.
#[test]
fn a_return_statement_builds_its_expression() {
    fn instructions(src: &str) -> i128 {
        let mut it = load_ir_chain();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .expect("parses")
            .as_int()
            .unwrap_or(0);
        assert!(parsed > 0, "the fixture must produce declarations");
        it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
            .expect("आरम्भः runs");
        it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
            .expect("कार्यक्रमरचना runs");
        match it.global("आज्ञासूचकाङ्क") {
            None => panic!("`आज्ञासूचकाङ्क` is not a global — THIS TEST is broken"),
            Some(v) => Value::as_int(v).expect("the cursor is an integer"),
        }
    }

    let with_value = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ योगः ४ ।\nइति\n";
    let plain = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n";

    let (v, p) = (instructions(with_value), instructions(plain));
    assert!(
        v > 0,
        "`प्रत्यागमनम् ३ योगः ४ ।` emitted {v} instructions. The return arm is \
         still falling through to `मूल्याङ्कनम् ०` and the function has no \
         return value to hand `अवसानरचना`"
    );
    // THE PLACEHOLDER RATCHET IS GONE AND ITS FAILURE IS WHY. This read
    // `(1, 1)` and said "when the binary arms land, `v` rises and this fails ON
    // PURPOSE"; `W-196` landed them and it did. `प्रत्यागमनम् ३ योगः ४` is now
    // THREE instructions — ConstInt ३, ConstInt ४, Add — and the plain return
    // is still the one ConstInt it always was.
    assert_eq!(
        (v, p),
        (3, 1),
        "expected (3, 1): `३ योगः ४` lowers to two ध्रुवाज्ञा and one \
         योगाज्ञा, and `३` to one ध्रुवाज्ञा. Got ({v}, {p})"
    );
}

/// A BINARY OPERATOR MUST LOWER TO ITS OWN INSTRUCTION, AND AN OPERATOR WITH
/// NO INSTRUCTION MUST NOT BORROW A NEIGHBOUR'S.
///
/// `अभिव्यञ्जकरचना` handled TWO expression kinds — numeral and group — and
/// every other kind, INCLUDING EVERY BINARY OPERATOR, fell through to a
/// `ध्रुवाज्ञाभेद` holding ०. So an unported expression did not fail: it
/// silently built a CONSTANT ZERO. `measure_corpus_ir` read eleven files as
/// `IR BUILT` while every `अ योगः आ` in all eleven had been lowered to nothing.
///
/// `parse.t1:877` writes `अभिव्यञ्जकयोजनम्(द्विकर्माभिव्यञ्जकभेद, ०, वाम,
/// दक्षिण, कर्म)`, so both operands are arena indices and `कर्म` is one of
/// `ast.t1`'s fifteen operator kinds. `ir.rs:23-25` DECLARES `Add` and `Sub`
/// and constructs neither, so lowering to them joins two halves the original
/// already has rather than inventing an instruction.
///
/// # The case that must still be refused
///
/// `आज्ञा` has exactly TWO binary kinds. गुणनम्, विभाजनम्, the six
/// comparisons, the three bit operators, the two shifts and `शेषः` have no
/// instruction to be lowered TO, and must therefore keep falling through to the
/// documented stub — NOT be rounded to the nearest arm. A `गुणनम्` emitted as
/// an `Add` is the same defect one step worse: wrong code that also looks
/// right. The third fixture pins that, and it pins it BY COUNT — a multiply
/// emits the one placeholder instruction a stub emits, not the three a lowered
/// operator emits.
#[test]
fn a_binary_operator_lowers_to_its_own_instruction_and_an_unported_one_does_not() {
    fn instructions(src: &str) -> i128 {
        let mut it = load_ir_chain();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .expect("parses")
            .as_int()
            .unwrap_or(0);
        assert!(parsed > 0, "the fixture must produce declarations");
        it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
            .expect("आरम्भः runs");
        it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
            .expect("कार्यक्रमरचना runs");
        match it.global("आज्ञासूचकाङ्क") {
            None => panic!("`आज्ञासूचकाङ्क` is not a global — THIS TEST is broken"),
            Some(v) => Value::as_int(v).expect("the cursor is an integer"),
        }
    }

    fn body(stmt: &str) -> String {
        format!("मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    {stmt}\nइति\n")
    }

    // THE KIND OF THE LAST INSTRUCTION, so the count is not the only witness.
    // A test that only counts cannot tell an Add from a third ConstInt.
    //
    // Read off the ARENA rather than through a reader routine: `मध्यरूप` has
    // no accessor and adding one for a test would put a routine in the module
    // that nothing in the pipeline calls.
    fn last_kind(src: &str) -> i128 {
        let mut it = load_ir_chain();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .expect("parses")
            .as_int()
            .unwrap_or(0);
        assert!(parsed > 0, "the fixture must produce declarations");
        it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
            .expect("आरम्भः runs");
        it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
            .expect("कार्यक्रमरचना runs");
        let cursor = match it.global("आज्ञासूचकाङ्क") {
            None => panic!("`आज्ञासूचकाङ्क` is not a global — THIS TEST is broken"),
            Some(v) => Value::as_int(v).expect("the cursor is an integer"),
        };
        assert!(cursor > 0, "nothing was appended, so there is no last kind");
        // `आज्ञाकोश` is 1-based and the appender advances-then-writes, so the
        // cursor IS the index of the last live entry.
        let insts = arena(
            it.global("आज्ञाकोश")
                .expect("`आज्ञाकोश` is a global — THIS TEST is broken if not"),
        );
        let last = insts
            .get(usize::try_from(cursor).expect("the cursor fits a usize"))
            .unwrap_or_else(|| panic!("the arena is shorter than its own cursor {cursor}"));
        Value::as_int(&member(last, "भेद")).expect("a kind is an integer")
    }

    let add = body("प्रत्यागमनम् ३ योगः ४ ।");
    let sub = body("प्रत्यागमनम् ३ वियोगः ४ ।");
    let mul = body("प्रत्यागमनम् ३ गुणनम् ४ ।");
    let one = body("प्रत्यागमनम् ३ ।");

    let (a, s, m, o) = (
        instructions(&add),
        instructions(&sub),
        instructions(&mul),
        instructions(&one),
    );

    assert_eq!(
        (a, s),
        (3, 3),
        "`३ योगः ४` and `३ वियोगः ४` must each lower to two ध्रुवाज्ञा and one \
         binary instruction. Got ({a}, {s}); a value of 1 means the operand is \
         still falling through to the constant-० stub"
    );
    assert_eq!(o, 1, "`३` alone is one ध्रुवाज्ञा, got {o}");

    // `W-245`: `गुणनम्` HAS a kind now — गुणनाज्ञाभेद (६) — so it lowers to three
    // instructions like the other two. Until this row the assertion here was
    // `m == 1`: the ONE placeholder, because rounding a multiply to an Add
    // would have been wrong code that looked right. The refusal is kept in a
    // different form below: every operator has its OWN kind, and the kind
    // read back must be the operator's, never a neighbour's.
    assert_eq!(
        m, 3,
        "`३ गुणनम् ४` emitted {m} instructions; since W-245 a multiply is \
         गुणनाज्ञाभेद, two ध्रुवाज्ञा and one गुणनाज्ञा"
    );

    // AND THE KINDS, because a count alone cannot tell an Add from a ConstInt —
    // or, now, a Mul from an Add.
    let (ka, ks, km) = (last_kind(&add), last_kind(&sub), last_kind(&mul));
    assert_eq!(
        (ka, ks, km),
        (3, 4, 6),
        "expected the last instruction to be योगाज्ञाभेद(३) for `योगः`, \
         वियोगाज्ञाभेद(४) for `वियोगः` and गुणनाज्ञाभेद(६) for `गुणनम्`. Got \
         ({ka}, {ks}, {km})"
    );

    // EVERY OPERATOR THE GRAMMAR FROZE HAS ITS OWN KIND, and none is rounded:
    // the eight non-additive operators (ADR-0032) and the five comparisons,
    // which are ONE kind (तुलनाज्ञाभेद, १४) with a sub-kind.
    for (op, kind) in [
        ("विभाजनम्", 7),
        ("शेषः", 8),
        ("वामसृ", 9),
        ("दक्षिणसृ", 10),
        ("युक्", 11),
        ("विकल्प", 12),
        ("विषम", 13),
        ("समम्", 14),
        ("असमम्", 14),
        ("न्यूनम्", 14),
        ("अधिकम्", 14),
        ("बृहत्समम्", 14),
    ] {
        let src = body(&format!("प्रत्यागमनम् ३ {op} ४ ।"));
        assert_eq!(
            (instructions(&src), last_kind(&src)),
            (3, kind),
            "`३ {op} ४` lowers to two constants and one instruction of kind {kind}"
        );
    }
    println!("METRIC sadhana_t1_operators_lowered 15");
}

// ── W-183: a name's declared type reaches the checker ──────────────────────
// Shared driver: lex → parse → (resolve) → typecheck, on `load_sema_with_parser`'s
// module set. Returns the checker's verdict.
fn w183_typecheck(src: &str, resolve_first: bool) -> bool {
    let mut it = load_sema_with_parser();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert!(
        parsed > 0,
        "the program declared nothing; the test is empty"
    );
    if resolve_first {
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("resolver starts");
        assert_eq!(
            it.call(
                "अर्थॱकार्यक्रमनिर्णयः",
                vec![r, Value::Int(parsed)],
                4_000_000_000
            )
            .expect("resolve runs"),
            Value::Bool(true),
            "the program must RESOLVE before its typecheck result means anything"
        );
    }
    it.call(
        "अर्थॱकार्यक्रमप्रकारपरीक्षा",
        vec![Value::Int(parsed)],
        4_000_000_000,
    )
    .expect("typecheck runs")
        == Value::Bool(true)
}

/// THE DISTINGUISHING CASE — written to FAIL FIRST. A body whose last statement is
/// the bare name `क`, declared `न६४`, in a `ददाति न६४` routine. Before W-183 every
/// Identifier typed as अ३२-signed (the checker's admitted stub) and this was
/// REFUSED; a return statement could not distinguish (it types as Never).
#[test]
fn a_declared_name_is_typed_as_declared_when_it_ends_a_body() {
    let src =
        "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः क ॱॱ न६४ भवति ३ ।\n    क ।\nइति\n";
    assert!(
        w183_typecheck(src, true),
        "`क` is declared `न६४` and the routine returns `न६४`; the checker still \
         types the name as the fixed अ३२ — the declared-type map is not reaching it"
    );
}

/// A PARAMETER knows its type at declaration; same shape, no `चरः`.
#[test]
fn a_parameter_is_typed_as_declared_when_it_ends_a_body() {
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय क ॱॱ न६४ ददाति न६४ आदि\n    क ।\nइति\n";
    assert!(
        w183_typecheck(src, true),
        "the parameter's declared `न६४` must reach the checker"
    );
}

// ── a field the struct does not declare ────────────────────────────────────

/// Same chain as [`w183_typecheck`], plus the checker's reset — which this one
/// needs and that one does not, because the bad-field counter is a GLOBAL and a
/// second call in the same process would read the first call's total.
fn typecheck_reading_bad_field(src: &str) -> (bool, i128, String, i128) {
    let mut it = load_sema_with_parser();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert!(
        parsed > 0,
        "the program declared nothing; the test is empty"
    );
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver starts");
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r.clone(), Value::Int(parsed)],
            4_000_000_000
        )
        .expect("resolve runs"),
        Value::Bool(true),
        "the program must RESOLVE before its typecheck result means anything"
    );
    it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![r], 5_000_000)
        .expect("checker starts");
    let verdict = it
        .call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000,
        )
        .expect("typecheck runs")
        == Value::Bool(true);
    (
        verdict,
        it.global("असत्क्षेत्रसंख्या")
            .and_then(Value::as_int)
            .unwrap_or(-1),
        text_global(&it, "असत्क्षेत्रनाम"),
        it.global("असत्क्षेत्रपङ्क्ति")
            .and_then(Value::as_int)
            .unwrap_or(-1),
    )
}

/// ONE PROGRAM, ONE WORD APART, AND BOTH DIRECTIONS IN ONE TEST — because the
/// defect being fixed is precisely a refusal that never fired, and a test that
/// only checks the good half reproduces the original condition exactly.
///
/// WRITTEN TO FAIL FIRST, and it did. At `5afe68b1` with the fix absent, BOTH
/// halves typechecked and both linked with status ०: `t1_build --emit` returned
/// 0 for a good field, for the struct's first field and for a field no struct
/// declares. The क्षेत्र arm read the token ONE PAST the member — `parse.t1:883`
/// stores `पठनस्थान` after the read and `t1_paradigm_calls.rs:289` compensates
/// with `x.value - 1` where this arm did not — so it compared the wrong token
/// against every declared field, matched none, and answered poison. Its margin
/// claimed a refusal; poison SUPPRESSES a complaint rather than making one.
///
/// THE FIELD READ IS AN EXPRESSION STATEMENT ON PURPOSE. The same read spelled
/// `प्रत्यागमनम् स्थानम् ॱ नास्तिक्षेत्रम् ।`, or as a `चरः` initialiser, or on an
/// assignment's right side is STILL ACCEPTED at the commit this test lands on —
/// those three arms of `वाक्यप्रकारः` (`artha.t1:3331`, `:3478`, `:3484`) answer
/// a type without typing their operand, so the expression never reaches any
/// arm at all. That is a hole in the typecheck stage far wider than this fix
/// and it is recorded rather than repaired here.
#[test]
fn a_field_the_struct_does_not_declare_refuses_the_program() {
    let program = |field: &str| {
        format!(
            "मण्डलम् परीक्षा ॥\n\
             सार्वजनिक संरचना बिन्दुः आरभ्य\n  प्रथमम् ॱॱ न६४ ऽ\n  द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\n\
             सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
             \x20   चरः स्थानम् ॱॱ बिन्दुः भवति ० ।\n\
             \x20   स्थानम् ॱ {field} ।\n\
             \x20   प्रत्यागमनम् ० ।\nइति\n"
        )
    };

    let (ok, count, _, _) = typecheck_reading_bad_field(&program("द्वितीयम्"));
    assert!(
        ok && count == 0,
        "`द्वितीयम्` IS declared by `बिन्दुः`; the checker must still accept it \
         (verdict {ok}, {count} bad-field site(s)) — a refusal that fires on \
         every field read is not a fix, it is a wider outage"
    );

    let (ok, count, name, line) = typecheck_reading_bad_field(&program("नास्तिक्षेत्रम्"));
    assert!(
        !ok,
        "`नास्तिक्षेत्रम्` is declared by no field of `बिन्दुः` and the program must \
         be REFUSED; it typechecked, emitted, assembled and linked with status ० \
         before this fix"
    );
    assert_eq!(
        (count, name.as_str(), line),
        // 8 MEASURED FROM THIS ASSERTION'S OWN FAILURE, 2026-09-07, never
        // counted by hand: the read is the eighth line of the program above.
        (1, "नास्तिक्षेत्रम्", 8),
        "the refusal must NAME what it refused and where — a bare `false` is \
         what `chain.rs` reports as \"refused the program and recorded no cause\""
    );
}

/// THE REFUSAL THAT MUST SURVIVE: the same body against `ददाति बूल`. Without this
/// the map could be "fixed" by typing every name as the declared return.
#[test]
fn a_declared_name_against_the_wrong_return_is_still_refused() {
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति बूल आदि\n    चरः क ॱॱ न६४ भवति ३ ।\n    क ।\nइति\n";
    assert!(
        !w183_typecheck(src, true),
        "`न६४` against `बूल` must still be REFUSED"
    );
}

/// UNRESOLVED IS NOT TYPED: the map comes from the RESOLVER's write-back. Run the
/// checker without resolving first and the distinguishing program is refused as
/// before — the ० fallback, by construction.
#[test]
fn an_unresolved_program_keeps_the_old_answer() {
    let src =
        "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः क ॱॱ न६४ भवति ३ ।\n    क ।\nइति\n";
    assert!(
        !w183_typecheck(src, false),
        "with no resolver run, no symbol is on the node and the old अ३२ answer stands — refused"
    );
}

/// One block of a lowered function, read off `पर्वकोश`.
///
/// `other` is the `अवसान`'s `अन्यलक्ष्यम्` — the else-target a conditional
/// branch carries — and is read as ० when the field does not exist, so that a
/// tree without the field fails these tests on the ASSERTION that names what is
/// missing and not on a member lookup.
#[derive(Debug, Clone, Copy)]
struct LoweredBlock {
    id: i128,
    insts: i128,
    kind: i128,
    value: i128,
    target: i128,
    other: i128,
}

/// Lex, parse and lower `src` through `मध्यरूप` and answer the FIRST function's
/// entry block id and its blocks in `पर्वकोश` order.
fn lower_first_function(src: &str) -> (i128, Vec<LoweredBlock>) {
    let mut it = load_ir_chain();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert!(parsed > 0, "the fixture must produce declarations");
    it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
        .expect("आरम्भः runs");
    it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
        .expect("कार्यक्रमरचना runs");

    let int = |v: &Value| Value::as_int(v).expect("an integer");
    // The else-target, or ० when `अवसान` has no such field yet.
    let other_target = |term: &Value| -> i128 {
        match term {
            Value::Record(r) => r
                .borrow()
                .get("अन्यलक्ष्यम्")
                .map(|t| int(&member(t, "क्रमाङ्क")))
                .unwrap_or(0),
            other => panic!("{other:?} is not a record, so it is not an अवसान"),
        }
    };
    let functions = arena(it.global("वृत्तिकोश").expect("`वृत्तिकोश` is a global"));
    let func = functions.get(1).expect("the first function is at index १");
    let entry = int(&member(&member(func, "प्रवेशपर्व"), "क्रमाङ्क"));
    let first = int(&member(func, "पर्वारम्भ"));
    let count = int(&member(func, "पर्वसंख्यान"));
    let blocks = arena(it.global("पर्वकोश").expect("`पर्वकोश` is a global"));
    let mut out = Vec::new();
    for i in first..first + count {
        let b = blocks
            .get(usize::try_from(i).expect("an index fits"))
            .unwrap_or_else(|| panic!("पर्वकोश has no entry {i}; the function's run is wrong"));
        let term = member(b, "अवसानम्");
        out.push(LoweredBlock {
            id: int(&member(&member(b, "अङ्कन"), "क्रमाङ्क")),
            insts: int(&member(b, "आज्ञासंख्यान")),
            kind: int(&member(&term, "भेद")),
            value: int(&member(&member(&term, "मूल्यम्"), "क्रमाङ्क")),
            target: int(&member(&member(&term, "लक्ष्यम्"), "क्रमाङ्क")),
            other: other_target(&term),
        });
    }
    (entry, out)
}

/// `अवसान` kinds as `ir.t1` numbers them. `CONDBRANCH` is ४ because BOTH twins
/// already had THREE terminators — `Return`, `Branch`, `Unreachable` — and not
/// the two the `W-198` row counted; audited at `ir.rs:31-35` and `ir.t1:87-89`.
const RETURN: i128 = 1;
const BRANCH: i128 = 2;
const CONDBRANCH: i128 = 4;

/// EVERY TARGET OF EVERY TERMINATOR IN A FUNCTION NAMES A BLOCK OF THAT
/// FUNCTION, AND NEVER ० — research/22 Rule X3: no jump-to-anywhere.
fn assert_every_target_is_inside(blocks: &[LoweredBlock]) {
    let ids: Vec<i128> = blocks.iter().map(|b| b.id).collect();
    for b in blocks {
        for (what, t) in [("लक्ष्यम्", b.target), ("अन्यलक्ष्यम्", b.other)]
        {
            if b.kind == BRANCH || b.kind == CONDBRANCH {
                if b.kind == BRANCH && what == "अन्यलक्ष्यम्" {
                    continue;
                }
                assert!(
                    t != 0 && ids.contains(&t),
                    "block {} ends in a {what} to block {t}, which is not one of this \
                     function's blocks {ids:?}. Rule X3: a branch targets a point inside \
                     its own span",
                    b.id
                );
            }
        }
    }
}

fn body_with(stmts: &str) -> String {
    format!("मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n{stmts}\nइति\n")
}

/// A `यदि` MUST LOWER TO A CONDITIONAL BRANCH WHOSE TWO TARGETS LIE INSIDE ITS
/// OWN FUNCTION, AND A `यदि` WITH NO `अन्यथा` MUST BRANCH TO THE JOIN.
///
/// Before `W-198` both twins had `Return`, `Branch` and `Unreachable` and no
/// conditional terminator at all, so `वाक्यरचना`'s `यदि` arm did not exist: the
/// statement fell through to `मूल्याङ्कनम् ०` and its body was never walked —
/// a `यदि` with a body of `३ योगः ४` built ONE block and nothing for the body.
///
/// # What the else fixture proves that the first one cannot
///
/// `parse.t1:958-963` parsed the `अन्यथा` body into `अन्यशरीर` and then DROPPED
/// IT — `वाक्ययोजनम्(यदिवाक्यभेद, शर्त, शरीर)` had only two child slots, so no
/// later stage could see an else body. The else block below must hold the three
/// instructions of `५ योगः ६`; a count of ० there means the AST is still not
/// carrying the branch.
///
/// # The case that must still be refused
///
/// A `यदि` with no `अन्यथा` is a conditional branch whose else-target IS the
/// join — not a second empty block, not ०, and not a target outside the
/// function. `assert_every_target_is_inside` is Rule X3 for every block.
#[test]
fn an_if_lowers_to_a_conditional_branch_whose_targets_lie_inside_its_function() {
    let no_else = body_with("    यदि सत्यम् आदि\n        ३ योगः ४ ।\n    इति\n    प्रत्यागमनम् ५ ।");
    let (entry, blocks) = lower_first_function(&no_else);
    // `W-245`: a `प्रत्यागमनम्` is a TERMINATOR now — it closes the join with the
    // Return and opens an empty block after itself, which the body's end closes
    // with a Return of nothing. Three blocks of the `यदि`, plus that one.
    assert_eq!(
        blocks.len(),
        4,
        "a `यदि` with no `अन्यथा` must lower to THREE blocks — entry, then, join — \
         and the empty block after the return; got {} : {blocks:?}",
        blocks.len()
    );
    let trailing = blocks.last().expect("the trailing block");
    assert_eq!(
        (trailing.kind, trailing.insts, trailing.value),
        (RETURN, 0, 0),
        "the block after the return is empty and returns nothing: {trailing:?}"
    );
    let head = blocks
        .iter()
        .find(|b| b.id == entry)
        .expect("the entry block is in the run");
    assert_eq!(
        head.kind, CONDBRANCH,
        "the entry block must end in a शाखावसान (४); it ends in kind {} — the \
         `यदि` arm is still falling through to the placeholder",
        head.kind
    );
    assert_ne!(head.target, head.other, "then and else targets must differ");
    let then = blocks
        .iter()
        .find(|b| b.id == head.target)
        .expect("then block");
    let join = blocks
        .iter()
        .find(|b| b.id == head.other)
        .expect("else target");
    assert_eq!(
        (then.kind, then.target),
        (BRANCH, join.id),
        "the then block must end in a लङ्घन to the join; got kind {} -> {}",
        then.kind,
        then.target
    );
    assert_eq!(
        then.insts, 3,
        "`३ योगः ४` in the then block is three instructions"
    );
    assert_eq!(
        join.kind, RETURN,
        "with no `अन्यथा` the else-target IS the join, which carries the function's return"
    );
    assert!(join.value != 0, "the join's Return carries `५`, not none");
    assert_every_target_is_inside(&blocks);

    let with_else = body_with(
        "    यदि सत्यम् आदि\n        ३ योगः ४ ।\n    इति अन्यथा आदि\n        ५ योगः ६ ।\n    इति\n    प्रत्यागमनम् ७ ।",
    );
    let (entry, blocks) = lower_first_function(&with_else);
    assert_eq!(
        blocks.len(),
        5,
        "a `यदि … अन्यथा` must lower to FOUR blocks — entry, then, else, join — and \
         the empty block after the return (W-245); got {} : {blocks:?}",
        blocks.len()
    );
    let head = blocks.iter().find(|b| b.id == entry).expect("entry");
    assert_eq!(head.kind, CONDBRANCH);
    let then = blocks
        .iter()
        .find(|b| b.id == head.target)
        .expect("then block");
    let other = blocks
        .iter()
        .find(|b| b.id == head.other)
        .expect("else block");
    assert_eq!(
        (then.kind, other.kind),
        (BRANCH, BRANCH),
        "both arms jump to the join"
    );
    assert_eq!(then.target, other.target, "both arms jump to the SAME join");
    assert!(
        then.target != then.id && then.target != other.id,
        "the join is a fourth block"
    );
    assert_eq!(
        other.insts, 3,
        "the else block must hold the three instructions of `५ योगः ६`; {} means the \
         parser is still dropping `अन्यशरीर` at `parse.t1:960`",
        other.insts
    );
    let join = blocks.iter().find(|b| b.id == then.target).expect("join");
    assert_eq!(join.kind, RETURN);
    assert_every_target_is_inside(&blocks);
}

/// A `यावत्` MUST LOWER TO A BACK-EDGE THAT TARGETS ITS OWN CONDITION BLOCK.
///
/// research/22 Rule X3: *a back-edge targets a point inside its own span*. The
/// loop's own lowering creates the condition block, and the body's `लङ्घन`
/// must go THERE — to a block numbered after the function's entry and before
/// the body — not to the entry and not to anything the loop did not create.
///
/// # The case that must still be refused
///
/// The back-edge may not target the function's entry block. A loop that
/// re-entered the function is `goto`, which the doc names as the one thing the
/// IR must not gain. And every target, as always, must be one of the
/// function's own blocks.
#[test]
fn a_while_lowers_to_a_back_edge_that_targets_its_own_condition() {
    let src = body_with("    यावत् सत्यम् आदि\n        ३ योगः ४ ।\n    इति\n    प्रत्यागमनम् ५ ।");
    let (entry, blocks) = lower_first_function(&src);
    assert_eq!(
        blocks.len(),
        5,
        "a `यावत्` must lower to FOUR blocks — entry, condition, body, exit — and the \
         empty block after the return (W-245); got {} : {blocks:?}",
        blocks.len()
    );
    let head = blocks.iter().find(|b| b.id == entry).expect("entry");
    assert_eq!(
        (head.kind, head.insts),
        (BRANCH, 0),
        "the entry must fall into the condition with an empty लङ्घन; got kind {} with \
         {} instructions — the `यावत्` arm is still falling through",
        head.kind,
        head.insts
    );
    let cond = blocks
        .iter()
        .find(|b| b.id == head.target)
        .expect("condition block");
    assert_eq!(
        cond.kind, CONDBRANCH,
        "the condition block ends in a शाखावसान"
    );
    assert!(
        cond.value != 0,
        "the शाखावसान carries the condition's value"
    );
    let body = blocks
        .iter()
        .find(|b| b.id == cond.target)
        .expect("body block");
    let exit = blocks
        .iter()
        .find(|b| b.id == cond.other)
        .expect("exit block");
    assert_eq!(
        body.insts, 3,
        "`३ योगः ४` in the body is three instructions"
    );
    // THE BACK-EDGE, and Rule X3 for it.
    assert_eq!(
        (body.kind, body.target),
        (BRANCH, cond.id),
        "the body must end in a लङ्घन back to ITS OWN condition block {}; got kind {} -> {}",
        cond.id,
        body.kind,
        body.target
    );
    assert!(
        body.target > entry && body.target < body.id,
        "the back-edge's target {} must lie inside the loop's own span — after the \
         entry {entry} and before the body {} — not at the function's entry (that \
         would be goto)",
        body.target,
        body.id
    );
    assert_ne!(
        body.target, entry,
        "REFUSED: a back-edge to the function's entry"
    );
    assert_eq!(
        exit.kind, RETURN,
        "the exit block carries the function's return"
    );
    assert_every_target_is_inside(&blocks);
}

/// A STATEMENT KIND THAT IS STILL UNPORTED MUST STILL YIELD THE PLACEHOLDER —
/// no value, no instruction, no block — and not be rounded to a neighbour.
///
/// `चर` (`चरः क ॱॱ न६४ भवति ३ ।`) waits on the local-variable map `ir.rs`
/// does not have either. Adding `यदि` and `यावत्` must not make it emit
/// anything: the body below must build exactly ONE block ending in `Return`
/// and exactly ONE instruction, the return's `५`. The refusal is by count,
/// which is the only witness that can tell "nothing" from "something wrong".
#[test]
fn an_unported_statement_kind_still_yields_the_placeholder_and_opens_no_block() {
    // `W-245` ported `चर` (a Store into the routine's locals table), so the one
    // statement kind still unported is `आयात` — a no-op at IR level.
    let src = body_with("    आयातः घ ।\n    प्रत्यागमनम् ५ ।");
    let (entry, blocks) = lower_first_function(&src);
    // Two blocks since `W-245`: the entry, and the empty block the return opens
    // after itself (closed with a Return of nothing) — not one for the `आयात`.
    assert_eq!(
        blocks.len(),
        2,
        "a body of one `आयात` and one return is the entry and the block after the \
         return; got {} : {blocks:?}",
        blocks.len()
    );
    assert_eq!((blocks[1].kind, blocks[1].insts), (RETURN, 0));
    let head = &blocks[0];
    assert_eq!(head.id, entry);
    assert_eq!(
        (head.kind, head.insts),
        (RETURN, 1),
        "the `आयात` statement is unported and must emit NOTHING — one instruction for \
         the return's `५`, a Return terminator. Got kind {} with {} instructions",
        head.kind,
        head.insts
    );
    assert_every_target_is_inside(&blocks);

    // And the `चर` this test was written on: since `W-245` it is a constant and a
    // STORE (two instructions), still one block, still no value of its own.
    let src = body_with("    चरः क ॱॱ न६४ भवति ३ ।\n    प्रत्यागमनम् ५ ।");
    let (entry, blocks) = lower_first_function(&src);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].id, entry);
    assert_eq!(
        (blocks[0].kind, blocks[0].insts),
        (RETURN, 3),
        "`चरः क भवति ३` is a ध्रुवाज्ञा and a निधानाज्ञा, then the return's `५`"
    );
}

// ── W-204: the ट loop in the IR — a call lowers to a Call ────────────────────

/// One instruction of a lowered program, read off `आज्ञाकोश`.
#[derive(Debug, Clone, Copy)]
struct LoweredInst {
    kind: i128,
    result: i128,
    symbol: i128,
    arg_first: i128,
    arg_count: i128,
    /// `ध्रुवमूल्यम्` — a ConstInt's value (`W-245`: numerals read their value).
    konst: i128,
}

/// What lowering `src` left behind: `कार्यक्रमरचना`'s answer, whether the
/// builder refused a call (and the line of the callee it refused), every
/// instruction in `आज्ञाकोश` order, the argument arena `आदानकोश` as value ids,
/// and the value the first function's entry block returns.
///
/// The refusal record and the argument arena are read as ABSENT-IS-FALSE /
/// ABSENT-IS-EMPTY rather than panicking on a missing global, so that a tree
/// without `W-204` fails these tests on the assertion that names what is
/// missing and not on a member lookup.
struct Lowering {
    answer: i128,
    refused: bool,
    refused_line: i128,
    insts: Vec<LoweredInst>,
    args: Vec<i128>,
    entry_return: i128,
}

/// Lex, parse, RESOLVE when asked, and lower `src` through `मध्यरूप`.
///
/// Resolution is optional on purpose: `W-183`'s write-back is what puts a
/// symbol on a name node, so "without the resolver" is how a test spells "an
/// unresolved program" — the same move `an_unresolved_program_keeps_the_old_answer`
/// makes for the checker.
fn lower_program(src: &str, resolve_first: bool) -> Lowering {
    let mut it = load_ir_chain();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert!(parsed > 0, "the fixture must produce declarations");
    if resolve_first {
        let r = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .expect("resolver starts");
        assert_eq!(
            it.call(
                "अर्थॱकार्यक्रमनिर्णयः",
                vec![r, Value::Int(parsed)],
                4_000_000_000
            )
            .expect("resolve runs"),
            Value::Bool(true),
            "the fixture must RESOLVE before its lowering means anything"
        );
    }
    it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
        .expect("आरम्भः runs");
    let answer = it
        .call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
        .expect("कार्यक्रमरचना runs")
        .as_int()
        .expect("कार्यक्रमरचना answers an integer");

    let int = |v: &Value| Value::as_int(v).expect("an integer");
    let refused = match it.global("अनिर्णीताह्वानमस्ति") {
        Some(Value::Bool(b)) => *b,
        Some(other) => panic!("`अनिर्णीताह्वानमस्ति` is {other:?}, not a बूल"),
        None => false,
    };
    let refused_line = match it.global("अनिर्णीताह्वानचिह्नकाङ्क").map(int)
    {
        Some(idx) if idx > 0 => {
            let tokens = arena(it.global("चिह्नककोश").expect("the token arena"));
            let tok = tokens
                .get(usize::try_from(idx).expect("fits"))
                .unwrap_or_else(|| panic!("no token {idx}"));
            int(&member(tok, "पङ्क्ति"))
        }
        _ => 0,
    };

    let cursor = int(it.global("आज्ञासूचकाङ्क").expect("`आज्ञासूचकाङ्क` is a global"));
    let insts_arena = arena(it.global("आज्ञाकोश").expect("`आज्ञाकोश` is a global"));
    let mut insts = Vec::new();
    for i in 1..=cursor {
        let e = insts_arena
            .get(usize::try_from(i).expect("fits"))
            .unwrap_or_else(|| panic!("आज्ञाकोश is shorter than its cursor {cursor}"));
        insts.push(LoweredInst {
            kind: int(&member(e, "भेद")),
            result: int(&member(&member(e, "फलम्"), "क्रमाङ्क")),
            symbol: int(&member(e, "संज्ञा")),
            arg_first: int(&member(e, "आदानारम्भ")),
            arg_count: int(&member(e, "आदानसंख्यान")),
            konst: int(&member(e, "ध्रुवमूल्यम्")),
        });
    }

    let mut args = vec![0];
    if let (Some(cur), Some(a)) = (it.global("आदानसूचकाङ्क"), it.global("आदानकोश"))
    {
        let a = arena(a);
        for i in 1..=int(cur) {
            let e = a
                .get(usize::try_from(i).expect("fits"))
                .unwrap_or_else(|| panic!("आदानकोश is shorter than its cursor"));
            args.push(int(&member(e, "क्रमाङ्क")));
        }
    }

    let functions = arena(it.global("वृत्तिकोश").expect("`वृत्तिकोश` is a global"));
    let entry_return = match functions.get(1) {
        Some(Value::Record(_)) => {
            let func = &functions[1];
            let entry = int(&member(&member(func, "प्रवेशपर्व"), "क्रमाङ्क"));
            let blocks = arena(it.global("पर्वकोश").expect("`पर्वकोश` is a global"));
            let b = blocks
                .get(usize::try_from(entry).expect("fits"))
                .unwrap_or_else(|| panic!("पर्वकोश has no entry block {entry}"));
            int(&member(&member(&member(b, "अवसानम्"), "मूल्यम्"), "क्रमाङ्क"))
        }
        _ => 0,
    };

    Lowering {
        answer,
        refused,
        refused_line,
        insts,
        args,
        entry_return,
    }
}

/// `आज्ञा` kinds as `ir.t1` numbers them.
const CONSTINT: i128 = 1;
const CALL: i128 = 2;

/// A CALL MUST LOWER TO A CALL INSTRUCTION THAT NAMES ITS CALLEE, CARRIES ITS
/// ARGUMENTS AS ONE CONTIGUOUS RUN, AND DEFINES A VALUE AT THE SITE.
///
/// research/22 §7, the large loop: व (the site control returns to) → र (the
/// arguments, out) → ट् ⇢ ट (the callee's entry) → त (the body) → व् ⇢ व
/// (return). So a call is ONE instruction inside a block, not a terminator,
/// and it defines a value — the one the site continues with.
///
/// WRITTEN TO FAIL FIRST. Before `W-204` `अभिव्यञ्जकरचना`'s call arm did not
/// exist: an आह्वान node fell into the catch-all and emitted a `ध्रुवाज्ञा`
/// holding ०, so `ग आरभ्य ३ समाप्तम्` lowered to ONE instruction of kind १
/// and the census read eleven files as BUILT with every routine call in them
/// lowered to nothing. The first assertion below is that count.
///
/// # The case that must still be refused
///
/// An INTERLEAVED RUN. `parse.t1` curries — `f a b` is `(f a) b` — and an
/// argument may itself be a call, whose own run is appended while the outer
/// call is still collecting. The second fixture puts a call in the second
/// argument position and asserts the outer run is still two ADJACENT entries
/// in `आदानकोश`, in written order. Appending each argument as it is built
/// passes the first fixture and fails this one.
#[test]
fn a_call_lowers_to_a_call_naming_its_callee_with_its_arguments_in_one_run() {
    let one = body_with("    ग आरभ्य ३ समाप्तम् ।");
    let l = lower_program(&one, true);
    assert!(
        !l.refused,
        "`ग` is declared and resolved, so the builder must not refuse it"
    );
    assert_eq!(
        l.insts.len(),
        2,
        "`ग आरभ्य ३ समाप्तम्` is ONE ध्रुवाज्ञा for ३ and ONE आह्वानाज्ञा; got {} : \
         {:?}. One instruction of kind १ means the call is still falling through \
         to the constant-० stub",
        l.insts.len(),
        l.insts
    );
    let (three, call) = (l.insts[0], l.insts[1]);
    // The KIND, and since `W-245` the VALUE: `अङ्कमूल्यम्` read ० for every
    // numeral until that row (measured then: `konst: 0` — a kind guard for a
    // token kind the lexer never emits, and a call to the reader that answers
    // the radix); it reads ३ now, and the constant that reaches the machine is
    // the one the source wrote.
    assert_eq!(
        three.kind, CONSTINT,
        "the argument is built first — र before ट; got kind {}",
        three.kind
    );
    assert_eq!(
        three.konst, 3,
        "the numeral `३` is the constant ३, not the ० every numeral read before W-245"
    );
    assert_eq!(
        call.kind, CALL,
        "the second instruction is the आह्वानाज्ञा (२); got kind {}",
        call.kind
    );
    assert!(
        call.symbol != 0,
        "the Call names ग by the symbol the resolver wrote onto the node; ० is no callee"
    );
    assert_eq!(call.arg_count, 1, "one argument");
    assert_eq!(
        l.args.get(usize::try_from(call.arg_first).expect("fits")),
        Some(&three.result),
        "the run's one entry is ३'s value; run starts at {} in {:?}",
        call.arg_first,
        l.args
    );
    assert!(
        call.result != 0 && call.result != three.result,
        "the call defines a value of its own at the site — control comes BACK (व)"
    );
    assert_eq!(
        l.entry_return, call.result,
        "the body's value is the call's result, and the function returns it"
    );

    // THE REFUSAL: an argument that is itself a call must not interleave.
    let nested = body_with("    ग आरभ्य ३ ऽ ग आरभ्य ४ समाप्तम् समाप्तम् ।");
    let l = lower_program(&nested, true);
    assert!(!l.refused);
    let kinds: Vec<i128> = l.insts.iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        vec![CONSTINT, CONSTINT, CALL, CALL],
        "written order: ३, then ४, then the inner call, then the outer; got {:?}",
        l.insts
    );
    let (v3, v4, inner, outer) = (l.insts[0], l.insts[1], l.insts[2], l.insts[3]);
    assert_eq!(inner.arg_count, 1);
    assert_eq!(
        l.args[usize::try_from(inner.arg_first).expect("fits")],
        v4.result,
        "the inner call's one argument is ४"
    );
    assert_eq!(outer.arg_count, 2, "the outer call has two arguments");
    let first = usize::try_from(outer.arg_first).expect("fits");
    assert_eq!(
        &l.args[first..first + 2],
        &[v3.result, inner.result],
        "REFUSED: an interleaved run. The outer call's two arguments must be \
         ADJACENT in आदानकोश — ३'s value then the inner call's result — even \
         though the inner call's own run was appended between them; \
         आदानकोश is {:?}",
        l.args
    );
    assert_eq!(l.entry_return, outer.result);
}

/// A CALL WHOSE CALLEE CARRIES NO SYMBOL IS REFUSED BEFORE THE IR, AND
/// NOTHING IS EMITTED FOR IT.
///
/// `W-183` writes the SymbolId+१ the resolver found onto the name node, and ०
/// means UNRESOLVED. A program the resolver never ran on has ० on every name;
/// lowering it must not produce a Call to nowhere, and must not fall through
/// to the constant-० stub either — that is the wrong code this row exists to
/// stop. The builder records the refusal (with the callee's line) and
/// `कार्यक्रमरचना` answers ०, the way `कार्यक्रमनिर्णयः` answers असत्यम्.
///
/// WRITTEN TO FAIL FIRST: before `W-204` there was no refusal record at all,
/// the answer was १ (one function built), and the arena held a constant ०.
///
/// # The case that must still be accepted
///
/// The SAME source, resolved, builds — pinned here beside the refusal so that
/// the two answers cannot drift apart.
#[test]
fn a_call_whose_callee_has_no_symbol_is_refused_before_the_ir_and_emits_nothing() {
    let src = body_with("    ग आरभ्य ३ समाप्तम् ।");

    let l = lower_program(&src, false);
    assert!(
        l.refused,
        "with no resolver run the callee carries ० and the builder must REFUSE. It \
         did not: the arena holds {:?} — a Call to nowhere, or the constant-० stub",
        l.insts
    );
    assert_eq!(
        l.refused_line, 3,
        "the refusal names the callee's line — the call is on line 3 of the fixture"
    );
    assert_eq!(
        l.answer, 0,
        "कार्यक्रमरचना must answer ० for a refused program; got {}",
        l.answer
    );
    assert!(
        l.insts.iter().all(|i| i.kind != CALL),
        "no आह्वानाज्ञा may exist for a refused call; the arena holds {:?}",
        l.insts
    );

    let ok = lower_program(&src, true);
    assert!(
        !ok.refused && ok.answer == 1,
        "the same source RESOLVED must build one function and refuse nothing; \
         got refused={} answer={}",
        ok.refused,
        ok.answer
    );
}

/// A ROUTINE'S BARE NAME IN VALUE POSITION IS A CALL WITH NO ARGUMENTS, AND A
/// VARIABLE'S IS NOT — `W-237`, closing `ir.t1`'s (d).
///
/// `parse.t1` pushes one आह्वान node per ARGUMENT, so `प्रत्यागमनम् आरम्भः ।`
/// reaches the builder as a नामाभिव्यञ्जक and, until this row, took the
/// Identifier stub: every zero-argument call in the corpus lowered to a
/// constant ० that ran. What tells the two apart is the SYMBOL'S KIND —
/// `W-228`'s `अर्थ ॱ संज्ञाभेदकोश`, kind १ a routine — which the builder now
/// reads at the node's symbol. The callee is declared AFTER its caller on
/// purpose: the resolver's pass one must have seen it.
///
/// WRITTEN TO FAIL FIRST: on the tree before this row the first instruction
/// of `ग` was a `ध्रुवाज्ञा` (kind १), and the assertion below names that.
///
/// # The case that must still be refused
///
/// `प्रत्यागमनम् ख ।` with `ख` a `चरः` — a local, kind ७ — must NOT become a
/// call. The second fixture lowers to no आह्वानाज्ञा at all.
#[test]
fn a_routine_name_in_value_position_is_a_call_with_no_arguments_and_a_variable_is_not() {
    let src = "मण्डलम् क ॥\n\
               सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् आरम्भः ।\nइति\n\
               सार्वजनिक वृत्तिः आरम्भः ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";
    let l = lower_program(src, true);
    assert!(!l.refused, "`आरम्भः` is declared and resolved");
    assert_eq!(l.answer, 2, "two routines built");
    let first = l
        .insts
        .first()
        .expect("`ग` lowers to at least one instruction");
    assert_eq!(
        first.kind, CALL,
        "`प्रत्यागमनम् आरम्भः ।` must lower to an आह्वानाज्ञा (kind २), not the \
         Identifier stub's ध्रुवाज्ञा (kind १); got {:?}",
        l.insts
    );
    assert_eq!(
        first.arg_count, 0,
        "no argument was written, so none is passed"
    );
    assert!(first.symbol > 0, "the call names आरम्भः's symbol: {first:?}");
    // And the value the site continues with is the call's — `ग` returns it.
    assert_eq!(
        l.entry_return, first.result,
        "`ग` returns what the call defined"
    );
    println!("METRIC sadhana_t1_zero_argument_call_lowered 1");

    // REFUSED: a variable's name is not a call. Since `W-245` it is a LOAD from
    // the local's slot (kind १५), where until that row it was the Identifier
    // stub's constant ०.
    let variable = body_with("    चरः ख ॱॱ न६४ भवति ० ।\n    प्रत्यागमनम् ख ।");
    let v = lower_program(&variable, true);
    assert!(!v.refused);
    assert!(
        v.insts.iter().all(|i| i.kind != CALL),
        "`ख` is a चरः (kind ७), not a routine; it must not lower to a call: {:?}",
        v.insts
    );
    let kinds: Vec<i128> = v.insts.iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        vec![CONSTINT, 16, 15],
        "`चरः ख भवति ०` is a constant and a Store; `प्रत्यागमनम् ख` a Load: {:?}",
        v.insts
    );
    assert_eq!(
        v.entry_return, v.insts[2].result,
        "the routine returns what the Load defined"
    );
}

/// `W-245`: A NUMERAL READS ITS VALUE, AND A `ऋण` NUMERAL READS NEGATIVE. Until
/// this row `अङ्कमूल्यम्` answered ० for every numeral (a kind guard for a token
/// kind the lexer never emits, then a call to the reader that answers the
/// RADIX), and the encode census counted the zeros among the stubs. The signed
/// reading is what keeps the twins together: both emitters see one small
/// negative and write `योगः … ऋण१न`, where a bit pattern (2⁶⁴−1) would send the
/// T1 twin to the constant pool and the Rust twin, reading it back as an `i64`,
/// to an `addi` — found by the twin census on `sanskrit_text.t1:3528`.
#[test]
fn a_numeral_reads_its_value_and_a_negative_one_reads_negative() {
    for (text, value) in [
        ("३", 3),
        ("१२३४", 1234),
        ("ऋण१", -1),
        ("ऋण२०४९", -2049),
        ("०", 0),
    ] {
        let l = lower_program(&body_with(&format!("    प्रत्यागमनम् {text} ।")), true);
        let konst = l.insts.first().expect("one constant").konst;
        assert_eq!(l.insts[0].kind, CONSTINT);
        assert_eq!(konst, value, "`{text}` reads {value}, got {konst}");
    }
    println!("METRIC sadhana_t1_numeral_reads_its_value 1");
}

/// WHAT SITS AT THE BOTTOM OF EVERY CALL CHAIN IN THE CORPUS — a measurement,
/// `W-204`. `parse.t1` curries, so a call's callee is the node the वाम chain
/// ends at. The IR's call arm needs a NAME there (the symbol is on a name
/// node); this tallies the callee kinds per file, prints the line of the first
/// non-name callee, and says whether the file RESOLVES first — the two facts
/// the IR census's refusals reduce to.
#[test]
#[ignore = "measurement"]
fn measure_corpus_call_callees() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    let int = |v: &Value| Value::as_int(v).unwrap_or(0);

    for n in &names {
        let src = std::fs::read_to_string(dir.join(n)).unwrap();
        let mut it = load_ir_chain();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(&src)], 2_000_000_000)
            .map(|v| v.as_int().unwrap_or(0))
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .map(|v| v.as_int().unwrap_or(0))
            .unwrap_or(0);
        if parsed == 0 {
            eprintln!("  {n:24} did not parse");
            continue;
        }
        let resolved = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .and_then(|r| {
                it.call(
                    "अर्थॱकार्यक्रमनिर्णयः",
                    vec![r, Value::Int(parsed)],
                    4_000_000_000,
                )
            });
        let resolve_note = match resolved {
            Ok(Value::Bool(true)) => "resolves".to_string(),
            Ok(_) => {
                let name = it.global("अनिर्णीतनाम").map(as_text).unwrap_or_default();
                let line = it.global("अनिर्णीतपङ्क्ति").map(int).unwrap_or(0);
                format!("REFUSED by the resolver: `{name}` line {line}")
            }
            Err(e) => format!("resolve error {:.60}", format!("{e:?}")),
        };

        let exprs = arena(it.global("अभिव्यञ्जककोश").expect("the expression arena"));
        let tokens = arena(it.global("चिह्नककोश").expect("the token arena"));
        let node = |i: i128| exprs.get(usize::try_from(i).unwrap_or(usize::MAX)).cloned();
        let mut by_kind: std::collections::BTreeMap<i128, usize> = Default::default();
        let mut first_odd: Option<(i128, i128)> = None;
        for (i, e) in exprs.iter().enumerate().skip(1) {
            let Value::Record(_) = e else { continue };
            if int(&member(e, "भेद")) != 7 {
                continue;
            }
            // only OUTERMOST call nodes: skip a node that is some other
            // call's वाम.
            let is_inner = exprs.iter().skip(1).any(|o| {
                matches!(o, Value::Record(_))
                    && int(&member(o, "भेद")) == 7
                    && int(&member(o, "वामसूचकाङ्क")) == i as i128
            });
            if is_inner {
                continue;
            }
            let mut at = int(&member(e, "वामसूचकाङ्क"));
            let mut bottom = None;
            while let Some(b) = node(at) {
                if int(&member(&b, "भेद")) == 7 {
                    at = int(&member(&b, "वामसूचकाङ्क"));
                } else {
                    bottom = Some(b);
                    break;
                }
            }
            let kind = bottom.as_ref().map(|b| int(&member(b, "भेद"))).unwrap_or(0);
            *by_kind.entry(kind).or_default() += 1;
            if kind != 1 && first_odd.is_none() {
                // find a token under it for a line: descend वाम until a node
                // carries a token index
                let mut probe = bottom.clone();
                let mut line = 0;
                while let Some(b) = probe {
                    let t = int(&member(&b, "मूल्यसूचकाङ्क"));
                    if t > 0 {
                        line = tokens
                            .get(usize::try_from(t).unwrap_or(usize::MAX))
                            .map(|tok| int(&member(tok, "पङ्क्ति")))
                            .unwrap_or(0);
                        break;
                    }
                    probe = node(int(&member(&b, "वामसूचकाङ्क")));
                }
                first_odd = Some((kind, line));
            }
        }
        eprintln!(
            "  {n:24} {resolve_note}; callee kinds {by_kind:?}; first non-name callee {first_odd:?}"
        );
    }
}

// ───────────────────────────────────────────────────────────────────────────
// W-202 — यदि and यावत् in the resolver and the checker.
// ───────────────────────────────────────────────────────────────────────────

/// Lex, parse and resolve one source: `(interpreter, program, resolved, refused name)`.
///
/// The five tests below all need the same four calls before they can assert
/// anything, and inlining them five times buries the one line that is the
/// test. `अनिर्णीतनाम` is read here rather than by each caller because it is
/// only meaningful while `अनिर्णीतमस्ति`, and that pairing is easy to get
/// wrong at a distance.
fn w202_resolve(src: &str) -> (Interpreter, i128, bool, String) {
    let mut it = load_sema_with_parser();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert!(
        parsed > 0,
        "the program declared nothing; the test is empty"
    );
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver starts");
    let ok = it
        .call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r, Value::Int(parsed)],
            4_000_000_000,
        )
        .expect("resolve runs")
        == Value::Bool(true);
    let refused = match it.global("अनिर्णीतमस्ति") {
        Some(Value::Bool(true)) => match it.global("अनिर्णीतनाम") {
            Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
            _ => "<no name>".into(),
        },
        _ => "<none>".into(),
    };
    (it, parsed, ok, refused)
}

fn w202_typechecks(it: &mut Interpreter, parsed: i128) -> bool {
    it.call(
        "अर्थॱकार्यक्रमप्रकारपरीक्षा",
        vec![Value::Int(parsed)],
        4_000_000_000,
    )
    .expect("typecheck runs")
        == Value::Bool(true)
}

/// **THE TEST THAT WOULD HAVE CAUGHT THE GAP W-202 CLOSED.**
///
/// `W-198` gave `वास्तुॱवाक्य` its `अन्यसूचकाङ्क` and stopped the parser
/// dropping the `अन्यथा` body — but nothing downstream READ the field, so the
/// resolver walked the condition and the then-body and stopped. An undeclared
/// name inside an else body was accepted by the one pass whose entire purpose
/// is to refuse it.
///
/// `the_else_construct_parses_and_resolves_in_all_four_shapes_the_corpus_writes`
/// did not catch this and could not have: every name in its five fixtures is
/// declared, so it asserts the program RESOLVES and passes whether the else
/// body is walked or skipped. It is green either way. The difference only
/// becomes visible when the else body contains something that must be refused,
/// which is what this test puts there.
///
/// The three arms are one program shape with the bad name moved: then-body
/// (refused before W-202 and still), else-body (the new coverage), and neither
/// (must still resolve, so the test cannot pass by refusing everything).
#[test]
fn an_undeclared_name_inside_an_else_body_is_refused() {
    let program = |body: &str, other: &str| {
        format!(
            "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
             यदि य समम् सत्यम् आदि\n        प्रत्यागमनम् {body} ।\n\
             इति अन्यथा आदि\n        प्रत्यागमनम् {other} ।\n    इति\nइति\n"
        )
    };

    let (_it, _p, ok, name) = w202_resolve(&program("१", "अपरिचितनाम"));
    assert!(
        !ok,
        "an undeclared name in an `अन्यथा` body must be REFUSED. Accepting it \
         is what this row found: the else half was parsed, stored in \
         `अन्यसूचकाङ्क`, and then never walked"
    );
    assert_eq!(
        name, "अपरिचितनाम",
        "the refusal must NAME the else body's undeclared name. A refusal that \
         reports some other name means the walk reached the else half by \
         accident rather than by the `अन्यसूचकाङ्क` branch"
    );

    // The control that keeps the assertion above honest — the SAME name in the
    // then-body, which was already refused before this row.
    let (_it, _p, ok_then, name_then) = w202_resolve(&program("अपरिचितनाम", "२"));
    assert!(!ok_then, "the then-body case must still refuse");
    assert_eq!(name_then, "अपरिचितनाम");

    // And the one that stops this passing by refusing everything.
    let (_it, _p, ok_clean, clean) = w202_resolve(&program("१", "२"));
    assert!(
        ok_clean,
        "a well-formed if/else must still RESOLVE; refused `{clean}`"
    );
}

/// A `यावत्` body is walked by the resolver and the loop types as Void.
///
/// Both halves matter. The resolve half is the same blindness as the else
/// body: a `यावत्` whose body names something undeclared must be refused. The
/// type half is the rule — a loop RETURNS TO ITS HEAD and may run zero times,
/// so it yields no value even when its body ends in one.
#[test]
fn a_while_body_is_resolved_and_the_loop_itself_is_void() {
    let well_formed = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
         चरः स ॱॱ न६४ भवति ० ।\n    यावत् य समम् सत्यम् आदि\n        स भवति १ ।\n    इति\n\
         प्रत्यागमनम् स ।\nइति\n";
    let (mut it, parsed, ok, refused) = w202_resolve(well_formed);
    assert!(ok, "a well-formed `यावत्` must resolve; refused `{refused}`");
    assert!(
        w202_typechecks(&mut it, parsed),
        "a `यावत्` whose body assigns and whose routine then returns `स` must \
         typecheck: the loop is Void and the RETURN diverges, so neither \
         disagrees with the declared `न६४`"
    );

    let undeclared = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
         चरः स ॱॱ न६४ भवति ० ।\n    यावत् य समम् सत्यम् आदि\n        स भवति अपरिचितनाम ।\n    इति\n\
         प्रत्यागमनम् स ।\nइति\n";
    let (_it, _p, ok2, name2) = w202_resolve(undeclared);
    assert!(
        !ok2,
        "an undeclared name inside a `यावत्` body must be REFUSED"
    );
    assert_eq!(name2, "अपरिचितनाम");
}

/// A condition that is not `बूल` is refused, and a comparison is `बूल`.
///
/// The second half is the one that had to be built before the first could
/// mean anything: a comparison is `द्विकर्माभिव्यञ्जक` and had NO arm in
/// `अभिव्यञ्जकप्रकारः`, so every condition in the corpus typed as poison and
/// a "must be बूल" rule would have refused all of them — measuring the missing
/// arm rather than the program.
#[test]
fn a_condition_must_be_bool_and_a_comparison_is() {
    let comparison = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
         यदि य समम् सत्यम् आदि\n        प्रत्यागमनम् १ ।\n    इति अन्यथा आदि\n\
         प्रत्यागमनम् २ ।\n    इति\nइति\n";
    let (mut it, parsed, ok, refused) = w202_resolve(comparison);
    assert!(ok, "must resolve; refused `{refused}`");
    assert!(
        w202_typechecks(&mut it, parsed),
        "a comparison condition types as बूल, so this must typecheck. If it \
         refuses, the द्विकर्म arm is gone and every conditional in the corpus \
         is being refused for the parser's reasons rather than its own"
    );

    // An ARITHMETIC condition is not a judgement and must be refused — with
    // the `यदि` in TAIL position, which is the only place the checker reaches.
    // See `a_conditional_that_is_not_the_last_statement_is_not_typechecked_yet`
    // immediately below for why that qualification is here and is not padding.
    let arithmetic = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
         चरः स ॱॱ न६४ भवति ० ।\n    यदि स योगः १ आदि\n        प्रत्यागमनम् १ ।\n    इति\nइति\n";
    let (mut it2, parsed2, ok2, refused2) = w202_resolve(arithmetic);
    assert!(
        ok2,
        "the arithmetic-condition program must RESOLVE (its names all exist); refused `{refused2}`"
    );
    assert!(
        !w202_typechecks(&mut it2, parsed2),
        "`स योगः १` is a VALUE, not a judgement — a condition that is not बूल \
         must be refused"
    );
}

/// **POSITION MUST NOT DECIDE WHETHER A STATEMENT IS CHECKED.**
///
/// `वाक्यप्रकारः`'s समूह arm used to return the LAST statement's type and walk
/// nothing else. Its note defended that with an argument which was correct
/// until this row: "the earlier ones are typechecked for their effect, which in
/// this pass is nothing — typecheck_expression has no side effect and RAISES NO
/// ERROR."
///
/// **W-202 FALSIFIED THAT PREMISE**, because this pass can now refuse. While
/// the arm still read only the last statement, the identical malformed
/// conditional was REFUSED in tail position and ACCEPTED with a `प्रत्यागमनम्`
/// after it — measured, and the reason the arm now folds over every direct
/// child.
///
/// Both halves below must refuse. The two programs differ ONLY in whether a
/// statement follows the `यदि`, so a pass that still keys on position fails
/// exactly one of them and names which.
#[test]
fn a_conditional_is_typechecked_wherever_it_sits_in_the_body() {
    let bad_condition = "यदि स योगः १ आदि\n        प्रत्यागमनम् १ ।\n    इति\n";

    let tail = format!(
        "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः स ॱॱ न६४ भवति ० ।\n    {bad_condition}इति\n"
    );
    let (mut it, parsed, ok, refused) = w202_resolve(&tail);
    assert!(ok, "must resolve; refused `{refused}`");
    assert!(
        !w202_typechecks(&mut it, parsed),
        "in TAIL position the non-बूल condition must be refused"
    );

    let mid = format!(
        "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः स ॱॱ न६४ भवति ० ।\n    \
         {bad_condition}    प्रत्यागमनम् ० ।\nइति\n"
    );
    let (mut it2, parsed2, ok2, refused2) = w202_resolve(&mid);
    assert!(ok2, "must resolve; refused `{refused2}`");
    assert!(
        !w202_typechecks(&mut it2, parsed2),
        "MID-BODY the SAME condition must be refused too. Accepting it is the \
         old समूह arm reading only the block's last statement — a refusal \
         swallowed because something well-typed came after it"
    );
}

/// A block's type is still its LAST statement's, and the fold did not change
/// that — only whether the earlier ones are looked at.
///
/// Without this, the fold could return `शून्यार्थः` or the first child's type
/// and every test above would still pass: they all assert REFUSAL, and a wrong
/// answer that is not poison refuses nothing. This is the arm's positive half.
#[test]
fn a_block_still_answers_with_its_last_statements_type() {
    // Last statement is a RETURN, which diverges — so the body does not
    // disagree with the declared `न६४` and this must typecheck. If the fold
    // answered with the `चरः` or with Void instead, this refuses.
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः स ॱॱ न६४ भवति ० ।\n\
               चरः त ॱॱ न६४ भवति १ ।\n    प्रत्यागमनम् स ।\nइति\n";
    let (mut it, parsed, ok, refused) = w202_resolve(src);
    assert!(ok, "must resolve; refused `{refused}`");
    assert!(
        w202_typechecks(&mut it, parsed),
        "the block's type is its LAST statement's — a diverging return — so \
         this typechecks. A fold that answered with an earlier statement's \
         type, or with Void, would refuse it"
    );
}

/// Two branches that yield different types do not yield one type.
///
/// This is the join's last clause, and the one with nothing to fall back on:
/// picking either side would invent a coercion the language does not have.
#[test]
fn a_conditional_whose_branches_disagree_is_refused() {
    let disagree = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
         यदि य समम् सत्यम् आदि\n        ० ।\n    इति अन्यथा आदि\n\
         उक्तम् क इति ।\n    इति\nइति\n";
    let (mut it, parsed, ok, refused) = w202_resolve(disagree);
    assert!(ok, "must resolve; refused `{refused}`");
    assert!(
        !w202_typechecks(&mut it, parsed),
        "a numeral branch and a string-literal branch are not one type — the \
         join must refuse rather than pick a side"
    );

    // THE HALF THAT MAKES THE HALF ABOVE MEAN ANYTHING. Before this row every
    // `यदि` fell through to `दोषार्थः`, so "disagreeing branches are refused"
    // was ALSO true of a parser that refused every conditional ever written —
    // the assertion above passes on the unmodified tree, for the wrong reason.
    // Two AGREEING branches must be ACCEPTED, and that is false before the join
    // exists. Verified by reverting `artha.t1`: this assertion fails there.
    let agree = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय य ॱॱ बूल ददाति न६४ आदि\n\
         यदि य समम् सत्यम् आदि\n        प्रत्यागमनम् १ ।\n    इति अन्यथा आदि\n\
         प्रत्यागमनम् २ ।\n    इति\nइति\n";
    let (mut it2, parsed2, ok2, refused2) = w202_resolve(agree);
    assert!(ok2, "must resolve; refused `{refused2}`");
    assert!(
        w202_typechecks(&mut it2, parsed2),
        "two branches that BOTH diverge join to Never and must be ACCEPTED. If \
         this refuses, the join is absorbing nothing and the arm is poisoning \
         every conditional — which is what it did before this row"
    );
}

// ── W-215: the unparser — parse → print → parse, measured ───────────────────

/// The chain the printer needs: lexer, AST, parser, numerals, the printer.
fn load_print_chain() -> Interpreter {
    load_all(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "unparse.t1",
    ])
}

/// Lex and parse `src` in `it`; answer the declaration count (० = did not parse).
fn parse_in(it: &mut Interpreter, src: &str) -> i128 {
    let toks = match it.call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
    {
        Ok(v) => v.as_int().unwrap_or(0),
        Err(_) => return 0,
    };
    it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .map(|v| v.as_int().unwrap_or(0))
        .unwrap_or(0)
}

/// `व्याकरॱउपेक्षितचिह्नकसंख्या` after a parse: top-level tokens the program
/// reader stepped over. ० when the parser predates the counter.
fn it_skipped(it: &Interpreter) -> i128 {
    it.global("उपेक्षितचिह्नकसंख्या")
        .and_then(Value::as_int)
        .unwrap_or(0)
}

/// The parser's first recorded refusal as "line N: reason", or "none".
fn first_parse_error(it: &Interpreter) -> String {
    let n = it.global("दोषसूचकाङ्क").and_then(Value::as_int).unwrap_or(0);
    if n == 0 {
        return "none".into();
    }
    let err = arena(it.global("दोषकोश").expect("the error arena"))
        .get(1)
        .cloned()
        .expect("the first error is at index १");
    format!(
        "line {}: {}",
        Value::as_int(&member(&err, "पङ्क्ति")).unwrap_or(0),
        as_text(&member(&err, "कारण"))
    )
}

/// What one print left behind.
struct Printed {
    text: String,
    refused: bool,
    reason: i128,
    node: i128,
}

/// Print the program `it` holds (`decls` declarations) through `मुद्रण`.
fn print_in(it: &mut Interpreter, decls: i128) -> Printed {
    let out = it
        .call("मुद्रणॱस्रोतलेखनम्", vec![Value::Int(decls)], 4_000_000_000)
        .expect("स्रोतलेखनम् runs");
    let int = |v: &Value| Value::as_int(v).unwrap_or(0);
    let refused = matches!(it.global("अमुद्रणीयमस्ति"), Some(Value::Bool(true)));
    Printed {
        text: as_text(&out),
        refused,
        reason: it.global("अमुद्रणीयकारणम्").map(int).unwrap_or(0),
        node: it.global("अमुद्रणीयसूचकाङ्क").map(int).unwrap_or(0),
    }
}

/// A structural rendering of the tree `it` holds, with every token index
/// replaced by the token's TEXT, so two parses of two different sources
/// can be compared for equality of shape and words. Layout, comments and
/// the spelling of a call are not in it, because they are not in the AST.
fn tree_shape(it: &Interpreter) -> String {
    let int = |v: &Value| Value::as_int(v).unwrap_or(0);
    let tokens = arena(it.global("चिह्नककोश").expect("the token arena"));
    let tok = |i: i128| -> String {
        tokens
            .get(usize::try_from(i).unwrap_or(usize::MAX))
            .map(|t| {
                let text = as_text(&member(t, "पाठ"));
                let from = usize::try_from(int(&member(t, "अष्टक"))).unwrap_or(0);
                let to = usize::try_from(int(&member(t, "पाठसीमा"))).unwrap_or(0);
                String::from_utf8_lossy(&text.as_bytes()[from.min(to)..to.min(text.len())])
                    .into_owned()
            })
            .unwrap_or_else(|| format!("<no token {i}>"))
    };
    let exprs = arena(it.global("अभिव्यञ्जककोश").expect("the expression arena"));
    let stmts = arena(it.global("वाक्यकोश").expect("the statement arena"));
    let decls = arena(it.global("घोषणाकोश").expect("the declaration arena"));
    let params = arena(it.global("प्राचलकोश").expect("the parameter arena"));
    let ndecl = int(it.global("घोषणासूचकाङ्क").expect("the declaration cursor"));
    let module = it.global("मण्डलनामसूचकाङ्क").map(int).unwrap_or(0);

    fn expr(
        i: i128,
        as_type: bool,
        exprs: &[Value],
        tok: &dyn Fn(i128) -> String,
        int: &dyn Fn(&Value) -> i128,
    ) -> String {
        if i == 0 {
            return "-".into();
        }
        let Some(e) = exprs.get(usize::try_from(i).unwrap_or(usize::MAX)) else {
            return format!("<no expr {i}>");
        };
        let kind = int(&member(e, "भेद"));
        let v = int(&member(e, "मूल्यसूचकाङ्क"));
        let l = int(&member(e, "वामसूचकाङ्क"));
        let r = int(&member(e, "दक्षिणसूचकाङ्क"));
        let op = int(&member(e, "द्विकर्म"));
        if as_type {
            return match kind {
                1 => format!("T:{}", tok(v)),
                2 => format!("T:slice({})", expr(l, true, exprs, tok, int)),
                3 => format!("T:ptr({})", expr(l, true, exprs, tok, int)),
                4 => format!("T:opt({})", expr(l, true, exprs, tok, int)),
                5 => format!("T:err({})", expr(l, true, exprs, tok, int)),
                k => format!("T:kind{k}"),
            };
        }
        match kind {
            // a name: the folded qualifier (W-226) carries its member in दक्षिण
            1 => {
                if r > 0 {
                    format!("name({} ॱ {})", tok(v), tok(r))
                } else {
                    format!("name({})", tok(v))
                }
            }
            2 => format!("num({})", tok(v)),
            3 => {
                // a string: its content tokens, start..end exclusive
                let mut words = Vec::new();
                let mut k = v;
                while r > 0 && k < r {
                    words.push(tok(k));
                    k += 1;
                }
                if r == 0 {
                    format!("str(from {})", tok(v))
                } else {
                    format!("str({})", words.join(" "))
                }
            }
            4 => format!("group({})", expr(l, false, exprs, tok, int)),
            5 => format!(
                "index({}, {})",
                expr(l, false, exprs, tok, int),
                expr(r, false, exprs, tok, int)
            ),
            6 => format!("field({}, {})", expr(l, false, exprs, tok, int), tok(v - 1)),
            7 => format!(
                "call({}, {})",
                expr(l, false, exprs, tok, int),
                expr(r, false, exprs, tok, int)
            ),
            8 => format!(
                "bin{}({}, {})",
                op,
                expr(l, false, exprs, tok, int),
                expr(r, false, exprs, tok, int)
            ),
            9 | 10 => format!("lit({})", tok(v)),
            11 => format!("embed({})", tok(v)),
            12 => format!("neg({})", expr(l, false, exprs, tok, int)),
            13 => format!(
                "slice({}, {})",
                expr(l, false, exprs, tok, int),
                expr(r, false, exprs, tok, int)
            ),
            k => format!(
                "kind{k}({}, {}, {}, {})",
                tok(v),
                expr(l, false, exprs, tok, int),
                expr(r, false, exprs, tok, int),
                op
            ),
        }
    }

    fn stmt(
        i: i128,
        exprs: &[Value],
        stmts: &[Value],
        tok: &dyn Fn(i128) -> String,
        int: &dyn Fn(&Value) -> i128,
    ) -> String {
        if i == 0 {
            return "-".into();
        }
        let Some(s) = stmts.get(usize::try_from(i).unwrap_or(usize::MAX)) else {
            return format!("<no stmt {i}>");
        };
        let kind = int(&member(s, "भेद"));
        let l = int(&member(s, "वामसूचकाङ्क"));
        let r = int(&member(s, "दक्षिणसूचकाङ्क"));
        let other = int(&member(s, "अन्यसूचकाङ्क"));
        let ty = int(&member(s, "प्रकारसूचकाङ्क"));
        match kind {
            1 => format!("expr({})", expr(l, false, exprs, tok, int)),
            2 => {
                // direct children in written order — the same walk the passes use
                let mut parts = Vec::new();
                if l > 0 {
                    let mut limit = l;
                    while limit < r + 1 {
                        let mut probe = r;
                        let mut next = 0;
                        while probe >= l {
                            if probe >= limit {
                                next = probe;
                            }
                            let node = &stmts[usize::try_from(probe).unwrap()];
                            probe = int(&member(node, "आदिसूचकाङ्क")) - 1;
                        }
                        if next < 1 {
                            limit = r + 1;
                        } else {
                            parts.push(stmt(next, exprs, stmts, tok, int));
                            limit = next + 1;
                        }
                    }
                }
                format!("block[{}]", parts.join("; "))
            }
            3 => format!(
                "var({}: {} = {})",
                tok(l),
                expr(ty, true, exprs, tok, int),
                expr(r, false, exprs, tok, int)
            ),
            4 => format!("return({})", expr(l, false, exprs, tok, int)),
            5 => format!("import({})", tok(l)),
            6 => format!(
                "if({}, {}, {})",
                expr(l, false, exprs, tok, int),
                stmt(r, exprs, stmts, tok, int),
                stmt(other, exprs, stmts, tok, int)
            ),
            7 => format!(
                "while({}, {})",
                expr(l, false, exprs, tok, int),
                stmt(r, exprs, stmts, tok, int)
            ),
            8 => format!(
                "assign({}, {})",
                expr(l, false, exprs, tok, int),
                expr(r, false, exprs, tok, int)
            ),
            k => format!("stmt-kind{k}"),
        }
    }

    let mut out = Vec::new();
    out.push(format!(
        "module({})",
        if module > 0 { tok(module) } else { "-".into() }
    ));
    for i in 1..=ndecl {
        let Some(d) = decls.get(usize::try_from(i).unwrap_or(usize::MAX)) else {
            out.push(format!("<no decl {i}>"));
            continue;
        };
        let kind = int(&member(d, "भेद"));
        let name = tok(int(&member(d, "नामसूचकाङ्क")));
        let ty = int(&member(d, "प्रकारसूचकाङ्क"));
        let body = int(&member(d, "शरीरसूचकाङ्क"));
        let pa = int(&member(d, "प्राचलादि"));
        let pz = int(&member(d, "प्राचलान्त"));
        let public = matches!(member(d, "सार्वजनिकत्व"), Value::Bool(true));
        let mut ps = Vec::new();
        if (kind == 1 || kind == 2) && pa > 0 {
            for p in pa..=pz {
                let pr = &params[usize::try_from(p).unwrap()];
                ps.push(format!(
                    "{}: {}",
                    tok(int(&member(pr, "नामसूचकाङ्क"))),
                    expr(int(&member(pr, "प्रकारसूचकाङ्क")), true, &exprs, &tok, &int)
                ));
            }
        }
        out.push(format!(
            "{}{}({} [{}] -> {} {} vr={}..{})",
            if public { "pub " } else { "" },
            match kind {
                1 => "fn",
                2 => "type",
                3 => "device",
                4 => "var",
                5 => "import",
                6 => "enum",
                _ => "decl?",
            },
            name,
            ps.join(", "),
            expr(ty, true, &exprs, &tok, &int),
            if kind == 4 {
                expr(body, false, &exprs, &tok, &int)
            } else {
                stmt(body, &exprs, &stmts, &tok, &int)
            },
            if kind == 6 { pa } else { 0 },
            if kind == 6 { pz } else { 0 },
        ));
    }
    out.join("\n")
}

/// One program in which every construct `व्याकर` reads appears at least once.
///
/// THE STRING'S SPELLING WAS WRONG UNTIL `W-240` AND NOTHING SAID SO: it read
/// `नमः इति इति इति लोकाः इति` — a doubled `इति` (the literal word) and then a
/// LONE `इति`, which closed the string; `लोकाः इति ।` then closed the ROUTINE,
/// and every statement after it sat at top level, where the program reader
/// stepped over each one in silence. The round trip still compared EQUAL,
/// because both sides lost the same statements. W-240's refusal of a skipped
/// token was the first thing to say "line 11: घोषणा अपेक्षितम्". The corpus
/// census had covered those constructs all along; this fixture had not.
const EVERYTHING: &str = "मण्डलम् परीक्षा ॥\n\
आयातः पदविभाग ।\n\
सार्वजनिक चरः सीमा ॱॱ न६४ भवति ३ ।\n\
सार्वजनिक संरचना युग्मम् आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ अङ्कः अन्तः अ८\nसमाप्तम् ।\n\
गणना वर्णः आरभ्य रक्तः ऽ नीलः समाप्तम् ।\n\
सार्वजनिक वृत्तिः ग आदाय क ॱॱ न६४ ऽ ख ॱॱ सम्भाव्य न६४ ददाति दोषयुक्त न६४ आदि\n\
    चरः फलम् ॱॱ न६४ भवति क योगः १ ।\n\
    चरः नाम ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् नमः इति इति लोकाः इति ।\n\
    यदि फलम् अधिकम् सीमा आदि\n        फलम् भवति आरभ्य फलम् वियोगः १ समाप्तम् गुणनम् २ ।\n    इति अन्यथा आदि\n        फलम् भवति ऋण फलम् ।\n    इति\n\
    यावत् फलम् न्यूनम् १० आदि\n        फलम् भवति ग आरभ्य फलम् ऽ शून्यम् समाप्तम् ।\n    इति\n\
    चरः युग्मः ॱॱ युग्मम् भवति ० ।\n\
    युग्मः ॱ प्रथमम् भवति नाम अङ्कः ० अन्तः ।\n\
    यदि सत्यम् आदि\n        प्रत्यागमनम् पदविभागॱचिह्नकपाठः युग्मः ।\n    इति\n\
    चरः खण्डः ॱॱ अङ्कः अन्तः अ८ भवति नाम अङ्कः १ अन्तः २ ।\n\
    चरः सीमान्तः ॱॱ न६४ भवति पदविभाग ॱ चिह्नकसूचकाङ्क ।\n\
    प्रत्यागमनम् फलम् ।\n\
इति\n";

/// PARSE → PRINT → PARSE MUST BE THE IDENTITY ON THE TREE — research/22
/// Rule X4, statistic 29. `EVERYTHING` holds each construct the parser
/// reads: module, import, a public global, a struct with two fields, an
/// enum with two variants, a routine with two parameters and an
/// error-union return, a typed local, a string with a doubled `इति`, an
/// if/else, a while, an assignment to a field, an index, a group, a
/// negation, a keyword literal, a bracketed and a juxtaposed qualified
/// call, a slice (`नाम अङ्कः १ अन्तः २`, W-228's kind १३) and a spaced
/// qualifier (`पदविभाग ॱ चिह्नकसूचकाङ्क`, folded into one name node), and two
/// returns.
///
/// WRITTEN TO FAIL FIRST against the unchanged parser, and the failure
/// names what the AST did not keep: the module name (`module(-)`), the
/// struct's fields (`type(युग्मम् [] …)`) and the string's end (`str(from
/// नमः)`), which the printer REFUSES rather than guesses.
#[test]
fn a_printed_program_re_parses_to_an_equal_tree() {
    let mut first = load_print_chain();
    let decls = parse_in(&mut first, EVERYTHING);
    assert!(
        decls > 0,
        "the fixture must parse; the parser's first refusal: {}",
        first_parse_error(&first)
    );
    let before = tree_shape(&first);
    let printed = print_in(&mut first, decls);
    assert!(
        !printed.refused,
        "the printer refused reason {} at node {}; the AST does not keep what it needs \
         — tree:\n{before}",
        printed.reason, printed.node
    );

    // WHAT THE ROUND TRIP CANNOT SEE: a struct whose fields were dropped by
    // the parser prints as an empty struct and re-parses "equal" — both
    // sides lost the same thing. So the TEXT is checked for the fixture's
    // field and variant names, and the doubled `इति` inside the string.
    for word in ["प्रथमम्", "द्वितीयम्", "रक्तः", "नीलः", "इति इति"]
    {
        assert!(
            printed.text.contains(word),
            "the printed source does not contain `{word}` — the AST dropped it; printed:\n{}",
            printed.text
        );
    }

    let mut second = load_print_chain();
    let again = parse_in(&mut second, &printed.text);
    assert!(
        again > 0,
        "the reprint must parse; printed:\n{}",
        printed.text
    );
    let after = tree_shape(&second);
    assert_eq!(
        before, after,
        "the reprint re-parsed to a DIFFERENT tree; printed:\n{}",
        printed.text
    );

    // and the print is a fixed point: printing the reprint gives the same text.
    let twice = print_in(&mut second, again);
    assert_eq!(
        twice.text, printed.text,
        "print ∘ parse ∘ print is not a fixed point"
    );
}

/// THE REFUSED CASE: a node the printer cannot spell is refused BY NAME —
/// which reason, which node — and the answer is EMPTY, never a program
/// with the node silently printed as something else. An expression node
/// of a kind `ast.t1` does not declare is pushed by hand through the
/// parser's own appender, the way a future kind would arrive.
#[test]
fn an_unspellable_node_is_refused_by_name_and_nothing_is_printed_as_something_else() {
    let mut it = load_print_chain();
    let decls = parse_in(
        &mut it,
        "मण्डलम् क ॥\nवृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n",
    );
    assert!(decls > 0);
    // the return's expression: find the statement of kind ४ and repoint it
    // at a fresh node of kind ९९.
    let ghost = it
        .call(
            "व्याकरॱअभिव्यञ्जकयोजनम्",
            vec![
                Value::Int(99),
                Value::Int(0),
                Value::Int(0),
                Value::Int(0),
                Value::Int(0),
            ],
            5_000_000,
        )
        .expect("the appender runs")
        .as_int()
        .expect("an index");
    let stmts = arena(it.global("वाक्यकोश").expect("the statement arena"));
    let ret = stmts
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, s)| matches!(s, Value::Record(_)) && Value::as_int(&member(s, "भेद")) == Some(4))
        .map(|(i, _)| i)
        .expect("the fixture has a return");
    // copy-modify-write, the arena idiom
    if let Some(Value::Arena(a)) = it.global("वाक्यकोश") {
        let mut a = a.borrow_mut();
        if let Value::Record(r) = &a[ret] {
            r.borrow_mut().insert("वामसूचकाङ्क".into(), Value::Int(ghost));
        }
        let _ = &mut a;
    }
    let printed = print_in(&mut it, decls);
    assert!(
        printed.refused,
        "a node of kind ९९ must be REFUSED; the printer wrote:\n{}",
        printed.text
    );
    assert_eq!(
        (printed.reason, printed.node),
        (3, ghost),
        "the refusal names the reason (३ = an expression kind with no arm) and the node"
    );
    assert!(
        printed.text.is_empty(),
        "a refused program prints NOTHING; got:\n{}",
        printed.text
    );
}

/// STATISTIC 29 — the parse/print round trip over the corpus. Per file:
/// did it parse, did it print, did the reprint parse, and is the tree the
/// same. The 16 `tests/corpus/t1/*.सस` are measured too, and are expected
/// not to parse: they are the doc-02 dialect (`॥ ध्रुवः … ॥`, `फलम्`), not
/// the frozen grammar `व्याकर` reads — research/25 §1.4 (W-221) found the
/// same; a reader for that dialect is W-238's decision, not this row's.
/// Prints `METRIC paradigm_halves_parse_print_rate` over the files that
/// PARSE, and the list of files whose reprint does not re-parse equal.
#[test]
#[ignore = "measurement"]
fn measure_corpus_parse_print() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for (dir, ext) in [("crates/sadhana-t1/src", "t1"), ("tests/corpus/t1", "सस")] {
        let mut here: Vec<_> = std::fs::read_dir(root.join(dir))
            .expect("the directory is readable")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some(ext))
            .collect();
        here.sort();
        files.extend(here);
    }
    let (mut parsed, mut equal, mut refused, mut unequal_list) =
        (0usize, 0usize, 0usize, Vec::new());
    let mut not_parsed = 0usize;
    // per set, because the two sets are not the same thing: `व्याकर` SKIPS a
    // token it has no arm for, so a doc-02 `.सस` file "parses" into the few
    // declarations its keywords happen to share with the frozen grammar (a
    // 200-line program into 1–28 declarations) and its round trip is equal
    // over that remnant — vacuously. The `.t1` number is the statistic.
    let (mut t1_parsed, mut t1_equal, mut sas_parsed, mut sas_equal) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut t1_skipped, mut sas_skipped) = (0i128, 0i128);
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let is_t1 = name.ends_with(".t1");
        let src = std::fs::read_to_string(path).unwrap();
        let mut first = load_print_chain();
        let decls = parse_in(&mut first, &src);
        if decls == 0 {
            eprintln!("  {name:28} did not parse");
            not_parsed += 1;
            continue;
        }
        parsed += 1;
        // top-level tokens the program reader stepped over without an arm —
        // the measurement behind the skip-a-token finding (a candidate row).
        let skipped = it_skipped(&first);
        if is_t1 {
            t1_skipped += skipped;
        } else {
            sas_skipped += skipped;
        }
        if is_t1 {
            t1_parsed += 1;
        } else {
            sas_parsed += 1;
        }
        let before = tree_shape(&first);
        let printed = print_in(&mut first, decls);
        if printed.refused {
            eprintln!(
                "  {name:28} {decls:>4} decls  REFUSED reason {} at node {}",
                printed.reason, printed.node
            );
            refused += 1;
            unequal_list.push(name);
            continue;
        }
        let mut second = load_print_chain();
        let again = parse_in(&mut second, &printed.text);
        if again == 0 {
            eprintln!(
                "  {name:28} {decls:>4} decls  printed {} bytes, REPRINT DID NOT PARSE",
                printed.text.len()
            );
            unequal_list.push(name);
            continue;
        }
        let after = tree_shape(&second);
        if before == after {
            eprintln!(
                "  {name:28} {decls:>4} decls  printed {} bytes, tree EQUAL, {skipped} top-level \
                 tokens skipped{}",
                printed.text.len(),
                if is_t1 {
                    ""
                } else {
                    "  (a remnant: the parser skipped what it does not read)"
                }
            );
            equal += 1;
            if is_t1 {
                t1_equal += 1;
            } else {
                sas_equal += 1;
            }
        } else {
            let line = before
                .lines()
                .zip(after.lines())
                .position(|(a, b)| a != b)
                .map(|i| i + 1)
                .unwrap_or(0);
            eprintln!(
                "  {name:28} {decls:>4} decls  printed {} bytes, tree DIFFERS at shape line {line}",
                printed.text.len()
            );
            unequal_list.push(name);
        }
    }
    let rate = if parsed == 0 {
        0.0
    } else {
        equal as f64 * 100.0 / parsed as f64
    };
    println!("METRIC paradigm_halves_parse_print_parsed {parsed}");
    println!("METRIC paradigm_halves_parse_print_equal {equal}");
    println!("METRIC paradigm_halves_parse_print_refused {refused}");
    println!("METRIC paradigm_halves_parse_print_not_parsed {not_parsed}");
    println!("METRIC paradigm_halves_parse_print_rate {rate:.2}");
    let t1_rate = if t1_parsed == 0 {
        0.0
    } else {
        t1_equal as f64 * 100.0 / t1_parsed as f64
    };
    println!("METRIC paradigm_halves_parse_print_rate_t1 {t1_rate:.2}");
    println!("METRIC paradigm_halves_parse_print_t1_equal {t1_equal}");
    println!("METRIC paradigm_halves_parse_print_t1_parsed {t1_parsed}");
    println!("METRIC paradigm_halves_parse_print_sas_equal {sas_equal}");
    println!("METRIC paradigm_halves_parse_print_sas_parsed {sas_parsed}");
    println!("METRIC paradigm_parser_skipped_top_level_tokens_t1 {t1_skipped}");
    println!("METRIC paradigm_parser_skipped_top_level_tokens_sas {sas_skipped}");
    eprintln!(
        "  PARSE/PRINT: {equal} equal of {parsed} parsed ({rate:.2}%); {refused} refused; \
         {not_parsed} did not parse; of {} files. Not equal: {unequal_list:?}",
        files.len()
    );
}

// ── W-240: the program reader refuses what it has no arm for ─────────────────

/// A STRAY TOP-LEVEL WORD IS REFUSED AT ITS LINE, NOT STEPPED OVER.
///
/// `कार्यक्रमपठनम्`'s dispatch ended in an else that consumed the token and
/// said nothing — research/22 §3.1's marker that is neither resolved nor
/// refused. W-215 counted what that arm swallowed: 58 tokens over the
/// corpus, every one a संरचना/गणना declaration's closing `।` that the two
/// type readers returned without consuming, and 62 in the one doc-02 file
/// the frozen grammar still let through as a 28-declaration remnant.
///
/// WRITTEN TO FAIL FIRST: on the W-215 tree the fixture below PARSES (one
/// routine), the counter reads १ for the stray word, and no error is
/// recorded. After W-240 the reader answers ० and `दोषकोश` names the line.
///
/// # The case that must still be accepted
///
/// The same program without the stray word parses as before — one routine,
/// counter ०, no error — so the refusal is of the word and not of the file.
#[test]
fn a_stray_top_level_word_is_refused_at_its_line() {
    let stray = "मण्डलम् क ॥\nअपशब्दः\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n";
    let mut it = load_print_chain();
    let decls = parse_in(&mut it, stray);
    let errors = it.global("दोषसूचकाङ्क").and_then(Value::as_int).unwrap_or(0);
    assert_eq!(
        (decls, errors),
        (0, 1),
        "a stray top-level word must be REFUSED: कार्यक्रमपठनम् answers ० and records \
         one error. Got {decls} declarations and {errors} errors — the reader is still \
         stepping over the word (counter {})",
        it_skipped(&it)
    );
    let err = arena(it.global("दोषकोश").expect("the error arena"))
        .get(1)
        .cloned()
        .expect("the first error is at index १");
    assert_eq!(
        Value::as_int(&member(&err, "पङ्क्ति")),
        Some(2),
        "the refusal names the stray word's line (2)"
    );
    assert!(
        as_text(&member(&err, "कारण")).contains("घोषणा"),
        "the reason says a declaration was expected; got {:?}",
        as_text(&member(&err, "कारण"))
    );

    let clean = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n";
    let mut it = load_print_chain();
    let decls = parse_in(&mut it, clean);
    let errors = it.global("दोषसूचकाङ्क").and_then(Value::as_int).unwrap_or(0);
    assert_eq!(
        (decls, errors, it_skipped(&it)),
        (1, 0, 0),
        "without the word: one routine, no error, nothing skipped"
    );
}

/// THE TWO TYPE READERS CONSUME THEIR CLOSING DANDA, so a struct or an enum
/// followed by anything still parses — the 58 dandas W-215 counted were the
/// whole of what the program reader skipped over the corpus.
#[test]
fn a_struct_and_an_enum_consume_their_closing_danda() {
    let src = "मण्डलम् क ॥\nसंरचना युग्मम् आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\nगणना वर्णः आरभ्य रक्तः ऽ नीलः समाप्तम् ।\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n";
    let mut it = load_print_chain();
    let decls = parse_in(&mut it, src);
    let errors = it.global("दोषसूचकाङ्क").and_then(Value::as_int).unwrap_or(0);
    assert_eq!(
        (decls, errors, it_skipped(&it)),
        (5, 0, 0),
        "a struct, an enum with two variants (three declarations), a routine: five \
         declarations, no error, NOTHING skipped — the readers ate their dandas"
    );
}

/// THE CORPUS PIN: over every `.t1` source the program reader skips NOTHING.
/// A ratchet at ०: any skipped token that appears is a defect in that source,
/// to be fixed in the same commit.
#[test]
fn the_program_reader_skips_nothing_over_the_corpus() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    let mut skipped: Vec<(String, i128)> = Vec::new();
    for n in &names {
        let src = std::fs::read_to_string(dir.join(n)).unwrap();
        let mut it = load_print_chain();
        let _ = parse_in(&mut it, &src);
        let k = it_skipped(&it);
        if k > 0 {
            skipped.push((n.clone(), k));
        }
    }
    assert!(
        skipped.is_empty(),
        "the program reader skipped top-level tokens in: {skipped:?}. W-215 measured 58 \
         before W-240, all closing dandas of type declarations; a new one is a source \
         defect — fix the source in the same commit"
    );
}

// ── W-240 (lane addition): a statement knows where it begins ─────────────────

/// EVERY STATEMENT CARRIES ITS FIRST TOKEN, AND THE CHECKER'S REFUSAL NAMES
/// THE LINE. `वास्तुॱवाक्य ॱ स्थान` was a run of octets the parser never wrote
/// and no pass read; it is a token index now (`स्थानसूचकाङ्क`), prepared by
/// `वाक्यपठनम्`/`समूहपठनम्` and written by `वाक्ययोजनम्`, and the type
/// checker's first-refusal record gains `प्रकारदोषपङ्क्ति`, the line of the
/// body it refused.
///
/// WRITTEN TO FAIL FIRST: the field did not exist under this name and the
/// checker had no line to give.
///
/// # The case that must still be refused
///
/// A statement pushed by hand through `वाक्ययोजनम्` — no reader prepared a
/// location for it — stores ० and the checker blames no line: the global the
/// readers set is cleared by every push, so nothing leaks from the previous
/// statement.
#[test]
fn a_statement_carries_its_first_token_and_a_refusal_names_the_line() {
    // a routine whose body's value is a बूल against a declared न६४: the one
    // refusal the checker records.
    let src =
        "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः ख ॱॱ न६४ भवति ३ ।\n    सत्यम् ।\nइति\n";
    let mut it = load_sema_with_parser();
    let toks = it
        .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    let parsed = it
        .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0);
    assert_eq!(parsed, 1, "the fixture is one routine");

    // every statement the parser pushed has a first token, and its line is
    // the line the statement is written on: the block at 2, the चरः at 3,
    // the bare सत्यम् at 4.
    let int = |v: &Value| Value::as_int(v).unwrap_or(0);
    let tokens = arena(it.global("चिह्नककोश").expect("the token arena"));
    let line_of = |tok: i128| -> i128 {
        tokens
            .get(usize::try_from(tok).unwrap_or(usize::MAX))
            .map(|t| int(&member(t, "पङ्क्ति")))
            .unwrap_or(0)
    };
    let stmts = arena(it.global("वाक्यकोश").expect("the statement arena"));
    let mut lines: Vec<(i128, i128)> = stmts
        .iter()
        .skip(1)
        .filter(|s| matches!(s, Value::Record(_)))
        .map(|s| {
            (
                int(&member(s, "भेद")),
                line_of(int(&member(s, "स्थानसूचकाङ्क"))),
            )
        })
        .collect();
    lines.sort();
    assert_eq!(
        lines,
        vec![(1, 4), (2, 2), (3, 3)],
        "(kind, line) of every statement: the expression `सत्यम्` at 4, the block at \
         2 (its `आदि`), the `चरः` at 3; got {lines:?} — a ० line means the parser \
         did not prepare the location"
    );

    // resolve, then typecheck: refused, and the refusal names the body's line.
    let r = it
        .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
        .expect("resolver starts");
    assert_eq!(
        it.call(
            "अर्थॱकार्यक्रमनिर्णयः",
            vec![r, Value::Int(parsed)],
            4_000_000_000
        )
        .expect("resolve runs"),
        Value::Bool(true)
    );
    let ok = it
        .call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(parsed)],
            4_000_000_000,
        )
        .expect("typecheck runs");
    assert_eq!(ok, Value::Bool(false), "बूल against न६४ is refused");
    let line = it
        .global("प्रकारदोषपङ्क्ति")
        .and_then(Value::as_int)
        .unwrap_or_else(|| {
            panic!("`प्रकारदोषपङ्क्ति` is not a global — the checker's refusal has no line")
        });
    assert_eq!(
        line, 2,
        "the refusal names the line of the body it refused (the routine's `आदि`, line 2)"
    );

    // THE REFUSED CASE: a hand-pushed statement has no location and is
    // blamed for no line — the readers' global was cleared by the last push.
    let pushed = it
        .call(
            "व्याकरॱवाक्ययोजनम्",
            vec![Value::Int(1), Value::Int(0), Value::Int(0)],
            5_000_000,
        )
        .expect("the appender runs")
        .as_int()
        .expect("an index");
    let stmts = arena(it.global("वाक्यकोश").expect("the statement arena"));
    let at = int(&member(
        &stmts[usize::try_from(pushed).unwrap()],
        "स्थानसूचकाङ्क",
    ));
    assert_eq!(
        at, 0,
        "a statement nobody read from source has no first token"
    );
    let named = it
        .call("अर्थॱवाक्यपङ्क्तिः", vec![Value::Int(pushed)], 5_000_000)
        .expect("वाक्यपङ्क्तिः runs")
        .as_int()
        .unwrap_or(-1);
    assert_eq!(
        named, 0,
        "and the checker's line for it is ०, not a neighbour's"
    );
}

/// **`W-254`: A STRING LITERAL'S OCTETS ARE BUILT, AND ADR-0011's DOUBLING IS
/// COLLAPSED WHILE BUILDING THEM.**
///
/// The arm this exercises had never run when it was written — it lexed, it
/// parsed, and `t1_sources` was green, none of which executes anything. This is
/// the test that makes "builds a string literal's octets" a measurement rather
/// than a description of the code.
///
/// THE DOUBLED CASE IS THE WHOLE POINT. `उक्तम् इति इति इति` is the single word
/// `इति` — nine octets — because `lex.rs`'s `string_value` copies through the
/// FIRST `इति` and resumes after the SECOND. A builder that simply joined the
/// content tokens would give `इति इति`, NINETEEN octets, and would be wrong on
/// nineteen sites across five files of the corpus. Nine against nineteen is the
/// assertion that tells those two implementations apart, so this test fails
/// loudly for the one mistake the design exists to avoid.
#[test]
fn a_string_literal_builds_its_octets_and_a_doubled_iti_is_one_word() {
    fn octet_count(src: &str) -> i128 {
        let mut it = load_ir_chain();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .expect("lexes")
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .expect("parses")
            .as_int()
            .unwrap_or(0);
        assert!(parsed > 0, "the fixture must produce declarations: {src}");
        it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
            .expect("आरम्भः runs");
        it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
            .expect("कार्यक्रमरचना runs");
        match it.global("पाठाक्षरसूचकाङ्क") {
            None => panic!("`पाठाक्षरसूचकाङ्क` is not a global — THIS TEST is broken"),
            Some(v) => Value::as_int(v).expect("the cursor is an integer"),
        }
    }

    let head = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति अङ्कः अन्तः अ८ आदि\n";
    // मन — two akṣaras of three octets each in UTF-8.
    let plain = format!("{head}    प्रत्यागमनम् उक्तम् मन इति ।\nइति\n");
    // The doubled pair: the value is the one word इति, three akṣaras, nine octets.
    let doubled = format!("{head}    प्रत्यागमनम् उक्तम् इति इति इति ।\nइति\n");

    let p = octet_count(&plain);
    assert_eq!(
        p, 6,
        "`उक्तम् मन इति` is मन: 6 octets. Got {p} — if this is 0 the arm never \
         ran, and if it is larger the delimiters or the trailing space are in \
         the value"
    );

    let d = octet_count(&doubled);
    assert_eq!(
        d, 9,
        "`उक्तम् इति इति इति` is the ONE word इति: 9 octets. Got {d}. NINETEEN \
         means the doubled pair was copied verbatim — the collapse ADR-0011 \
         requires did not happen, and the corpus uses doubling at nineteen sites"
    );
    println!("METRIC t1_string_octets_plain {p}");
    println!("METRIC t1_string_octets_doubled {d}");
}

/// **A STRUCT OPENED WITH THE WRONG KEYWORD IS ACCEPTED SILENTLY, AND FOUR
/// FIXTURES PASSED ON ONE FOR WEEKS.** `W-292`.
///
/// A `संरचना` body opens with **`आरभ्य`**. The corpus has **59 of them and 59
/// use it**; `आदि` appears zero times. `संरचनापठनम्` matches the literal and
/// consumes it — and when the match fails it consumes nothing, the field loop
/// begins on the unconsumed token, and `आदि` and `ॱॱ` are read as field names.
///
/// **NOTHING REFUSES.** Lex, parse, resolve and typecheck all report success on
/// a declaration whose field table names the block keyword and the type
/// separator as members. The program compiles, links and runs.
///
/// # What that cost, measured rather than supposed
///
/// ```text
///                       opener `आदि` (malformed)     opener `आरभ्य` (correct)
/// 2-field struct        प्राचलादि=1 … प्राचलान्त=3     प्राचलादि=1 … प्राचलान्त=2
///                       THREE entries for two         two entries
///                       naming आदि, ॱॱ, द्वितीयम्       naming both fields
/// first field           cause ३९, never lowers        offset 0
/// second field          offset 16                     offset 8
/// ```
///
/// **The wrong offsets are self-consistent**, which is the whole reason this
/// survived: `क्षेत्रस्थानाज्ञा` and `क्षेत्राज्ञा` carry the SAME wrong number, so
/// a fixture that writes a field and reads it back **agrees with itself at the
/// wrong address and passes**. Every fixture in `t1_storage_witness` had that
/// shape and every one of them read `द्वितीयम्` — the field the malformed parse
/// happens to place. `प्रथमम्` was unreachable in all of them and no test asked
/// for it.
///
/// # The retraction this test carries
///
/// Three claims were landed on main from the malformed fixture and **all three
/// are false**: that the first field of any struct is unreachable, that a
/// two-field struct gets three entries, and that every field offset is eight
/// octets too high. **The parser was correct throughout.** What was wrong was an
/// input nobody read against a corpus struct — and a three-field variant built
/// to test the model *confirmed* it, because a second shape tests the model you
/// have while only an instance from the population tests whether the model is
/// about that population at all.
///
/// # What this asserts, and why the correct openers carry the assertions
///
/// The `आरभ्य` variants pin **correct** behaviour: no field cause fires and the
/// offsets are exactly `0, 8, 16`. The `आदि` variants **print and assert
/// nothing** — pinning their numbers would enshrine a defect, and the defect is
/// that they are accepted at all, which belongs to whichever row makes
/// `संरचनापठनम्` refuse an opener it does not recognise.
///
/// WHAT IT DOES NOT COVER: it stops at IR. And it says nothing about how many
/// other declaration forms accept a wrong keyword the same way — `वृत्तिः`
/// bodies do open with `आदि`, which is exactly why the wrong word looked right.
#[test]
#[ignore = "measurement: six tiny sources through the .t1 IR chain"]
fn a_struct_opened_with_the_wrong_keyword_is_accepted_silently() {
    let src_of = |opener: &str, decl: &str, field: &str| {
        format!(
            "{}{}{}{}{}{}{}",
            "मण्डलम् परीक्षा ॥\n",
            format_args!("संरचना धारकः {opener}\n{decl}\nसमाप्तम् ।\n"),
            "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
            "    चरः धा ॱॱ धारकः भवति ० ।\n",
            format_args!("    धा ॱ {field} भवति २१ ।\n"),
            format_args!("    प्रत्यागमनम् धा ॱ {field} ।\n"),
            "इति\n",
        )
    };
    let two = "    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४";
    let three = "    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४ ऽ\n    तृतीयम् ॱॱ न६४";
    let sources = [
        ("ok-first", src_of("आरभ्य", two, "प्रथमम्"), true),
        ("ok-second", src_of("आरभ्य", two, "द्वितीयम्"), true),
        ("ok-third-of-three", src_of("आरभ्य", three, "तृतीयम्"), true),
        ("bad-opener-first", src_of("आदि", two, "प्रथमम्"), false),
        ("bad-opener-second", src_of("आदि", two, "द्वितीयम्"), false),
    ];

    // The field arm raises SEVENTEEN DISTINCT causes — it is the INDEX arm that
    // funnels its guards into a single २७, and conflating the two is what made
    // this look unmeasurable before it was read.
    const FIELD_CAUSES: [i128; 17] = [
        40, 41, 42, 26, 43, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
    ];

    let mut wellformed: Vec<(&str, Vec<i128>, Vec<i128>)> = Vec::new();
    for (label, src, correct) in &sources {
        // A FRESH INTERPRETER PER SOURCE, so no source's arenas reach the next,
        // and TYPECHECK IN THE CHAIN, because the field walk reads arenas the
        // checker writes.
        let mut it = load_ir_chain();
        let toks = it
            .call("पदविभागॱपदविभाग", vec![octets(src)], 2_000_000_000)
            .unwrap_or_else(|e| panic!("{label}: lex: {e:?}"))
            .as_int()
            .unwrap_or(0);
        let parsed = it
            .call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(toks)], 4_000_000_000)
            .unwrap_or_else(|e| panic!("{label}: parse: {e:?}"))
            .as_int()
            .unwrap_or(0);
        assert!(parsed > 0, "{label}: the parser produced no declarations");
        let resolver = it
            .call("अर्थॱनिर्णायकारम्भः", vec![], 5_000_000)
            .unwrap_or_else(|e| panic!("{label}: निर्णायकारम्भः: {e:?}"));
        let resolved = it
            .call(
                "अर्थॱकार्यक्रमनिर्णयः",
                vec![resolver.clone(), Value::Int(parsed)],
                4_000_000_000,
            )
            .unwrap_or_else(|e| panic!("{label}: resolve: {e:?}"));
        it.call("अर्थॱप्रकारपरीक्षकारम्भः", vec![resolver], 5_000_000)
            .unwrap_or_else(|e| panic!("{label}: प्रकारपरीक्षकारम्भः: {e:?}"));
        let checked = it
            .call(
                "अर्थॱकार्यक्रमप्रकारपरीक्षा",
                vec![Value::Int(parsed)],
                4_000_000_000,
            )
            .unwrap_or_else(|e| panic!("{label}: typecheck: {e:?}"));
        it.call("मध्यरूपॱआरम्भः", vec![], 5_000_000)
            .unwrap_or_else(|e| panic!("{label}: आरम्भः: {e:?}"));
        it.call("मध्यरूपॱकार्यक्रमरचना", vec![Value::Int(parsed)], 4_000_000_000)
            .unwrap_or_else(|e| panic!("{label}: build: {e:?}"));

        let mut causes: Vec<i128> = Vec::new();
        if let Some(v) = it.global("अपूर्णगणनाकोश") {
            for (k, e) in arena(v).iter().enumerate() {
                let k = i128::try_from(k).unwrap();
                if Value::as_int(e).unwrap_or(0) > 0 && FIELD_CAUSES.contains(&k) {
                    causes.push(k);
                }
            }
        }
        // `क्षेत्राज्ञाभेद` १९ is the field read and `क्षेत्रस्थानाज्ञाभेद` २५ the
        // address; both carry the offset in `ध्रुवमूल्यम्`.
        let offsets: Vec<i128> = it
            .global("आज्ञाकोश")
            .map(arena)
            .unwrap_or_default()
            .iter()
            .filter(|e| matches!(e, Value::Record(_)))
            .filter(|e| matches!(Value::as_int(&member(e, "भेद")), Some(19) | Some(25)))
            .filter_map(|e| Value::as_int(&member(e, "ध्रुवमूल्यम्")))
            .collect();

        // PRINTED BEFORE EVERY GUARD. A diagnostic that does not survive a red
        // was not measured.
        println!(
            "METRIC t1_struct_opener_{label}_accepted resolve={resolved:?} typecheck={checked:?}"
        );
        println!("METRIC t1_struct_opener_{label}_field_causes {causes:?}");
        println!("METRIC t1_struct_opener_{label}_field_offsets {offsets:?}");
        if *correct {
            wellformed.push((label, causes, offsets));
        }
    }

    // THE DEFECT, RECORDED WHERE A READER WILL SEE IT AND ASSERTED NOWHERE: the
    // malformed sources above reached `build_ir`, which means lex, parse,
    // resolve and typecheck each answered success on a struct whose fields are
    // the block keyword and the type separator. The `assert!(parsed > 0)` in the
    // loop is the evidence — a refusal would have stopped the run there.

    for (label, causes, offsets) in &wellformed {
        assert!(
            causes.is_empty(),
            "{label}: a well-formed struct refused a field access with cause(s) \
             {causes:?}. The opener is `आरभ्य`, which is what all 59 corpus \
             structs use, so a refusal here is a real defect and not a fixture \
             that was never parsed."
        );
        assert!(
            !offsets.is_empty(),
            "{label}: no field instruction was emitted at all, so the offsets \
             below are untested rather than correct"
        );
    }
    // THE OFFSETS ARE PINNED AT THEIR CORRECT VALUES, which is possible only
    // because the openers above are right. `0` for a first field, `8` for a
    // second, `16` for a third of three — `क्षेत्रस्थानम् गुणनम् ८` over a 0-based
    // ordinal. These are the numbers the retracted finding claimed were `8`,
    // `16` and `24`.
    assert_eq!(
        wellformed[0].2,
        vec![0, 0],
        "a first field sits at offset 0"
    );
    assert_eq!(
        wellformed[1].2,
        vec![8, 8],
        "a second field sits at offset 8"
    );
    assert_eq!(
        wellformed[2].2,
        vec![16, 16],
        "the third field of three sits at offset 16"
    );
}
