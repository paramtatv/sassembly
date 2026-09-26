//! **DOES AN EMBED LOWER ONCE THE DRIVER RESOLVES IT?**
//!
//! `समावेशः आरभ्य <table> समाप्तम्` stubbed under cause १२ for every source the
//! chain compiles, because the chain never resolved it. The `.t1` half needed no
//! change — `ir.t1:1236` lowers a string literal and an embed IS a string — so the
//! whole of the fix is a driver step in `chain.rs`.
//!
//! # THE SECOND SIDE OF THE CONSERVATION CHECK IS THE ONE THAT MATTERS
//!
//! Cause १२ falling to zero is necessary and not sufficient. **A stub that became
//! an EMPTY string would also take the cause to zero** — the arm would be reached,
//! lower cleanly, and emit nothing. That is a re-description wearing a lowering's
//! clothes, and no by-cause column can tell them apart.
//!
//! So the test that decides it is **the emitted octets**: an embed's bytes must
//! appear in the image. `spec/registers-riscv64.tsv` is 4,122 bytes, so a source
//! carrying one embed must grow by about that much and not by zero.
//!
//! # AND ONE TABLE IS NOT THE POPULATION — 2026-09-17
//!
//! The three tests above all read `spec/registers-riscv64.tsv`, and one table is
//! one table whether it is fetched BY NAME, by index, or by a constant. The test
//! that used to stand at the foot of this file claimed to supply the population —
//! *"a source the corpus actually ships, carrying seven `समावेशः` sites"* — and
//! printed `vishlesana_embed_sites 0` while asserting nothing: `14799fad` had
//! rewritten those seven to run-time store reads three days earlier, so it was
//! measuring a retired contract. The population is `anita::table_names`, the
//! registry the host actually fills the embed store from, and the two tests at
//! the foot of this file are that set plus the name it does NOT hold.

use sadhana::t1::anita;
use sadhana::t1::chain::Front;
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// A module whose only interesting content is one embed, returned by length so
/// the value cannot be optimised away as unused.
///
/// **IT RETURNED `०` UNTIL 2026-09-17 AND SO IT DID NOT DO THAT.** The doc
/// above has always said "returned by length"; the source declared `पाठ्यम्`
/// and then answered `०`, never reading it. Measured: returning the length
/// takes the emission from 27 lines to 41, so the binding really was being
/// dropped and the fixture's stated premise — that the value cannot be
/// optimised away — was false for as long as it has existed.
const WITH_EMBED: &str = "मण्डलम् परीक्षा ॥\n\
     सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
     \u{20}   चरः पाठ्यम् ॱॱ अङ्कः अन्तः अ८ भवति समावेशः आरभ्य कोष्ठकोशः समाप्तम् ।\n\
     \u{20}   प्रत्यागमनम् पाठ्यम् ॱ दैर्घ्य ।\n\
     इति\n";

/// The same module with the embed replaced by an empty run — the control for
/// "the resolver fed the arm nothing".
const WITHOUT_EMBED: &str = "मण्डलम् परीक्षा ॥\n\
     सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
     \u{20}   प्रत्यागमनम् ० ।\n\
     इति\n";

/// The fixture above, parameterised by the table it names — the ONE thing that
/// varies across the population test below. `measure_that_every_registered_table`
/// asserts this reproduces [`WITH_EMBED`] for `कोष्ठकोशः`, so the two cannot drift.
fn with_embed_of(table: &str) -> String {
    format!(
        "मण्डलम् परीक्षा ॥\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
         \u{20}   चरः पाठ्यम् ॱॱ अङ्कः अन्तः अ८ भवति समावेशः आरभ्य {table} समाप्तम् ।\n\
         \u{20}   प्रत्यागमनम् पाठ्यम् ॱ दैर्घ्य ।\n\
         इति\n"
    )
}

/// The emitted octets, read out of the data line, returned WITH the line so a
/// caller can print what it read rather than assert against something invisible.
///
/// **ONE READING, USED BY BOTH TESTS.** This was inline in
/// `measure_that_the_emitted_octets_are_the_tables_own_bytes` and is factored
/// here unchanged: two extractions that drift are worse than one that holds, and
/// the population test below would otherwise be a SECOND opinion about what an
/// emitted data line looks like.
///
/// The data sits in ONE directive line — `emit_data` writes `॥ अष्टकाः … ॥` with
/// every octet as a numeral — so it is found by LENGTH rather than by guessing a
/// keyword, then every token that is wholly Devanagari numerals is one octet.
/// `chars().take()`, never a byte slice: Devanagari is multibyte and `&l[..n]`
/// panics on a non-char boundary.
fn recovered_octets(text: &str) -> (&str, Vec<u8>) {
    let data = text
        .lines()
        .max_by_key(|l| l.len())
        .expect("the emission has lines");
    let dev = |c: char| "०१२३४५६७८९".find(c).map(|i| i / 3);
    let bytes: Vec<u8> = data
        .split_whitespace()
        .filter(|w| !w.is_empty() && w.chars().all(|c| dev(c).is_some()))
        .filter_map(|w| {
            let mut n: u32 = 0;
            for c in w.chars() {
                n = n * 10 + dev(c)? as u32;
            }
            u8::try_from(n).ok()
        })
        .collect();
    (data, bytes)
}

fn emit(src: &str) -> Result<String, String> {
    let mut front = Front::load(&spec_root()).map_err(|e| format!("front: {e}"))?;
    front.lex(src).map_err(|e| format!("lex: {e}"))?;
    front.parse().map_err(|e| format!("parse: {e}"))?;
    front.resolve().map_err(|e| format!("resolve: {e}"))?;
    front.typecheck().map_err(|e| format!("typecheck: {e}"))?;
    front.build_ir().map_err(|e| format!("ir: {e}"))?;
    let m = front
        .module("परीक्षा", Some("मुख्यम्"))
        .map_err(|e| format!("module: {e}"))?;
    riscv64::emit_module(&m).map_err(|r| format!("emit: {r:?}"))
}

/// **THE WHOLE UNIT, IN ONE READING.** A source carrying an embed must reach an
/// emitted image, and that image must be materially larger than the same source
/// without the embed. Equal sizes mean the embed contributed nothing.
#[test]
fn measure_that_an_embed_reaches_the_image_and_carries_its_bytes() {
    let table = std::fs::read(spec_root().join("registers-riscv64.tsv"))
        .expect("spec/registers-riscv64.tsv is readable");
    println!("METRIC embed_table_bytes {}", table.len());

    let with = emit(WITH_EMBED);
    let without = emit(WITHOUT_EMBED);
    match (&with, &without) {
        (Ok(a), Ok(b)) => {
            println!("METRIC embed_emitted_with {}", a.len());
            println!("METRIC embed_emitted_without {}", b.len());
            println!(
                "METRIC embed_emitted_delta {}",
                a.len() as i64 - b.len() as i64
            );
        }
        _ => println!("  with={with:?}\n  without={without:?}"),
    }
    let a = with.expect("a source carrying an embed must reach an emitted image");
    let b = without.expect("the control must reach an emitted image");

    // **NOT A RATIO AND NOT A MAGNITUDE.** The emitted text is assembly, so the
    // table's 4,122 bytes do not appear verbatim and any predicted size would be
    // a guess. What is NOT a guess is the direction and that it is not zero: an
    // embed that lowered to an empty string would leave these equal.
    assert!(
        a.len() > b.len(),
        "the source WITH an embed emitted {} bytes and the one WITHOUT emitted {} — \
         equal or smaller means the embed contributed NOTHING to the image, which is \
         a stub replaced by an empty string rather than a lowering",
        a.len(),
        b.len()
    );
}

/// **THE FIDELITY CHECK — GROWTH IS NOT ENOUGH.**
///
/// A lowering that emitted 4,122 bytes of the WRONG data would grow the image
/// exactly as much as one that emitted the right data. So the emitted octets are
/// extracted and compared to the file, byte for byte.
#[test]
fn measure_that_the_emitted_octets_are_the_tables_own_bytes() {
    let table = std::fs::read(spec_root().join("registers-riscv64.tsv"))
        .expect("spec/registers-riscv64.tsv is readable");
    let text = emit(WITH_EMBED).expect("the embed source emits");
    // Print the shape once so the extraction below is checkable rather than magic.
    // `chars().take()`, never a byte slice — Devanagari is multibyte and `&l[..n]`
    // panics on a non-char boundary.
    for l in text.lines().skip_while(|l| !l.contains('०')).take(3) {
        let s: String = l.chars().take(60).collect();
        println!("  SAMPLE {s}");
    }
    println!("METRIC embed_emitted_lines {}", text.lines().count());
    println!("METRIC embed_table_bytes {}", table.len());

    // The data sits in ONE directive line - the reading is `recovered_octets`,
    // shared with the population test below so the two cannot drift.
    let (data, bytes) = recovered_octets(&text);
    let s: String = data.chars().take(70).collect();
    println!("  DATA LINE starts: {s}");
    println!("METRIC embed_data_line_bytes {}", data.len());
    println!("METRIC embed_data_octets_recovered {}", bytes.len());

    assert_eq!(
        bytes.len(),
        table.len(),
        "recovered {} octets from the emitted data line against a table of {} bytes — \
         a count that differs means the emission is not the table",
        bytes.len(),
        table.len()
    );
    assert_eq!(
        bytes, table,
        "the emitted octets are the right LENGTH and the wrong BYTES, which growth \
         alone could never have caught"
    );
    println!(
        "  EMITTED OCTETS EQUAL THE FILE, all {} of them",
        table.len()
    );
}

/// **THE POPULATION — EVERY TABLE THE BUILD REGISTERS, ONE SOURCE EACH.**
///
/// # WHAT THIS REPLACES, AND WHY THE SUBJECT MOVES RATHER THAN THE NUMBER
///
/// `measure_vishlesana_which_carries_seven_embeds` stood here and called itself
/// *"the population: a source the corpus actually ships, carrying seven `समावेशः`
/// sites"*. **Measured 2026-09-17 it printed `vishlesana_embed_sites 0`**, and it
/// asserted NOTHING — so it had been green on that zero for as long as the zero
/// had been true, while compiling 375 KB of emission to report it.
///
/// The zero is correct and the premise is what is dead. **The corpus has no live
/// embed site at all**: every `समावेशः आरभ्य` in the twenty-one `.t1` sources
/// today sits inside a `॰` comment. `14799fad` (2026-09-14, *"the compiler reads
/// its own spec tables at run time, not through the include"*) took the last four
/// out of `vishlesana.t1` itself, rewriting each to `पदविभागॱसमावेशपाठः उक्तम्
/// सङ्केतकोशः इति` — a RUN-TIME store read, which is a different mechanism from
/// the compile-time embed this file exists to measure. Re-pinning 7 → 0 would have
/// enshrined a count of nothing; the population has to be found somewhere real.
///
/// # AND IT IS REAL: THE THIRTEEN NAMES A PROGRAM MAY ACTUALLY SPELL
///
/// [`anita::table_names`] is the registry the host fills the embed store from
/// (`nirvahana.rs:1000`), so it is exactly the set of embeds that can resolve.
/// Thirteen tables from 632 bytes to 74,688 — and **all thirteen differ in
/// LENGTH**, so a lookup that ignored the name and answered a fixed table, or
/// indexed the store positionally, reds on twelve of them at the count alone and
/// never reaches the byte comparison.
///
/// The synthetic above proves the mechanism on ONE table and cannot see any of
/// that: one table is one table whether it is fetched by name, by index or by
/// constant.
#[test]
fn measure_that_every_registered_table_embeds_as_its_own_bytes() {
    // The parameterised fixture must BE the one the two tests above pin, or this
    // measures a different program from the one they measured.
    assert_eq!(
        with_embed_of("कोष्ठकोशः"),
        WITH_EMBED,
        "the parameterised fixture has drifted from WITH_EMBED, so the population \
         below is not the same program the fidelity test pins"
    );

    let names = anita::table_names();
    println!("METRIC embed_registered_tables {}", names.len());

    // Every row MEASURED before anything is asserted, so a failure names every
    // table that is wrong rather than only the first.
    let mut rows: Vec<(&str, &str, usize, usize, bool)> = Vec::new();
    for name in &names {
        let rel = anita::table_path(name).expect("a registered name has a path");
        let file = std::fs::read(spec_root().join(rel))
            .unwrap_or_else(|e| panic!("spec/{rel} is readable: {e}"));
        let text = match emit(&with_embed_of(name)) {
            Ok(t) => t,
            Err(e) => {
                println!("METRIC embed_table_STOPPED_AT {rel} {e}");
                rows.push((name, rel, file.len(), 0, false));
                continue;
            }
        };
        let (_, bytes) = recovered_octets(&text);
        let same = bytes == file;
        println!(
            "METRIC embed_table {rel} file {} recovered {} same {}",
            file.len(),
            bytes.len(),
            u8::from(same)
        );
        rows.push((name, rel, file.len(), bytes.len(), same));
    }

    // THE LENGTHS ARE PAIRWISE DISTINCT, and that is an assertion and not an
    // aside: it is what makes the count alone separate the thirteen, so the
    // argument above about a positional lookup is a property of this tree rather
    // than a thing that happened to be true when it was written.
    let mut lens: Vec<usize> = rows.iter().map(|r| r.2).collect();
    lens.sort_unstable();
    let before = lens.len();
    lens.dedup();
    assert_eq!(
        before,
        lens.len(),
        "two registered tables are the same LENGTH, so a wrong-table lookup could \
         pass the count check — the byte comparison still holds but this margin no \
         longer does, and it should be rewritten rather than deleted"
    );

    let wrong: Vec<String> = rows
        .iter()
        .filter(|r| !r.4)
        .map(|(_, rel, f, g, _)| format!("{rel}: file {f}, recovered {g}"))
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} registered tables did not reach the image as their own bytes — \
         a recovered count of 0 is a stub or an empty string, a count that is some \
         OTHER table's length is a lookup that ignored the name:\n  {}",
        wrong.len(),
        rows.len(),
        wrong.join("\n  ")
    );
    println!("  ALL {} REGISTERED TABLES EQUAL THEIR FILES", rows.len());
}

/// **THE CASE THAT MUST STILL BE REFUSED — A NAME THE STORE DOES NOT HOLD.**
///
/// The population above is thirteen greens and every one of them would stay green
/// under a lookup that answered *something* for *any* name. What separates a
/// lookup BY NAME from a lookup that cannot fail is the name that has no answer.
///
/// `शब्दकोशः` is spelled exactly like the thirteen — `<concept>कोशः`, ordinary
/// Devanagari, no path and nothing to escape — and is not among them. The host's
/// own margin (`nirvahana.rs:995-999`) states the designed outcome: *"A table that
/// does not read is simply absent from the store, and the lexer leaves its four
/// tokens alone — the parser's embed node, `ir.t1`'s cause १२ stub, the marker
/// that already exists."*
///
/// **THREE OUTCOMES, NOT TWO, AND THE ARM IS MEASURED RATHER THAN PREDICTED.**
/// A pass/fail here would report the middle state as the good one:
///
///     a phase stops, naming the name       REFUSED early
///     it emits and carries NO octets       the cause १२ stub — the designed arm
///     it emits and carries SOME table      a lookup that ignored the name
///
/// The third is the one this test exists for and the only one that is a defect:
/// an unknown name must never come back holding a REGISTERED table's bytes. The
/// first two are both honest refusals and the trace says which happened.
#[test]
fn measure_that_a_table_the_store_does_not_hold_reaches_no_octets() {
    const ABSENT: &str = "शब्दकोशः";
    assert!(
        anita::table_path(ABSENT).is_none(),
        "`{ABSENT}` has been ADDED to the registry, so it is no longer the \
         unknown-name case — pick another unregistered `<concept>कोशः` rather \
         than deleting this test"
    );

    let files: Vec<(String, Vec<u8>)> = anita::table_names()
        .iter()
        .map(|n| {
            let rel = anita::table_path(n).expect("a registered name has a path");
            let b = std::fs::read(spec_root().join(rel)).unwrap_or_default();
            (rel.to_string(), b)
        })
        .collect();

    match emit(&with_embed_of(ABSENT)) {
        Err(e) => {
            println!("METRIC embed_absent_table_STOPPED_AT {e}");
            println!("  REFUSED EARLY, which is the first of the three arms");
        }
        Ok(text) => {
            println!("METRIC embed_absent_emitted_bytes {}", text.len());
            println!("METRIC embed_absent_emitted_lines {}", text.lines().count());
            let (data, bytes) = recovered_octets(&text);
            let s: String = data.chars().take(70).collect();
            println!("  LONGEST LINE starts: {s}");
            println!("METRIC embed_absent_octets_recovered {}", bytes.len());

            // **THE ONE THAT IS A DEFECT.** Named per table, because "it matched
            // something" and "it matched `registers-riscv64.tsv`" are different
            // reports and the second one says where to look.
            for (rel, file) in &files {
                assert_ne!(
                    &bytes,
                    file,
                    "an embed of `{ABSENT}` — a name NO table answers to — came back \
                     carrying all {} bytes of spec/{rel}, so the store is not being \
                     read BY NAME",
                    file.len()
                );
            }

            // A stray numeral in an instruction line is not a table. The smallest
            // registered table is 632 octets, so anything at or above that is a
            // payload and not noise; below it, the emission carries no table.
            let floor = files.iter().map(|(_, b)| b.len()).min().unwrap_or(0);
            assert!(
                bytes.len() < floor,
                "an embed of `{ABSENT}` recovered {} octets, at or past the {floor} of \
                 the smallest registered table — it is carrying a payload from \
                 somewhere, and an unknown name has nothing to carry",
                bytes.len()
            );
            println!("  EMITTED, CARRYING NO TABLE — the cause १२ stub arm");
        }
    }
}
