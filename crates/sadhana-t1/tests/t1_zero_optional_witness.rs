//! **No routine may read presence off a value that can legitimately be ०.**
//!
//! `ir.t1` lowers `सम्भाव्य` as ONE WORD with `०` meaning absent, so in the
//! image `Some(०)` and absence are the same bits. The interpreter keeps
//! `Value::Int(0)` and `Value::Nil` apart. Every site that compares one
//! against the other is therefore a place where the two engines MUST answer
//! differently — not a risk, a certainty, and neither engine is wrong about
//! its own representation.
//!
//! # Why this is a test and not a census
//!
//! `tools/optional-whether-census.py` can say which of the 60 numeric sites
//! COULD receive a ०. It cannot say which DO: that depends on the data the
//! corpus is run over. The first site found this way was found by running —
//! `nirvahana.rs` records every `Int(0)`-against-`Nil` comparison, and a
//! four-construct compile (`शृङ्खला ॱ स्वपरीक्षाचतुष्टयम्`) reported exactly one, at
//! `सङ्केतसूचीरचना`'s `आकृतिः`. Exactly one row of `spec/encodings-riscv64.tsv`
//! has an all-zero pattern — line 60, `c.addi4spn` — so the count the
//! interpreter reported and the count the table holds agree. That agreement is
//! what makes it a measurement rather than a warning.
//!
//! Left unrepaired, the compiled compiler leaves that row's
//! `सङ्केतसूचीरूपसिद्धकोश` at ० and cannot encode `c.addi4spn` at all — a
//! silently missing instruction, not a crash.
//!
//! # What this test does NOT cover
//!
//! It witnesses only the sites the run it performs REACHES. A site guarded by
//! data this test never supplies stays invisible here, exactly as it stayed
//! invisible before. Widening the run widens the guard; the census is the
//! upper bound and this is the lower one.

use sadhana::t1::nirvahana::{Interpreter, Value, zero_optional_witnesses};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Enough of the corpus to build the encoding table: the builder is in
/// `encode.t1`, the field readers it now asks are in `vishlesana.t1`, and the
/// numeral clauses reach `sanskrit_text.t1`.
fn load() -> Interpreter {
    let names = [
        "lex.t1",
        "encode.t1",
        "sanskrit_text.t1",
        "vishlesana.t1",
        "nidana.t1",
        "vakyavibhaga.t1",
        "ashtaka.t1",
        "kosha.t1",
    ];
    let sources: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = sources
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    Interpreter::load(&refs, &repo_root().join("spec")).expect("the corpus loads")
}

/// Building the encoding table asks no value whether it is there.
///
/// The assertion is on the WITNESSES, not on a named routine, so a new site
/// introduced anywhere the table build reaches fails this test without anyone
/// having to add a case for it.
#[test]
fn building_the_encoding_table_reads_no_presence_off_a_value() {
    let mut it = load();
    let n = it
        .call("सङ्केतनॱसङ्केतसूचीरचना", vec![], 200_000_000)
        .expect("the table builds");

    // The run has to have HAPPENED. A refusal that returned ० rows would
    // witness nothing and pass, which is the shape of a guard that measures
    // its own absence.
    let rows = n.as_int().expect("the builder answers a count");
    assert!(
        rows > 100,
        "the table build answered {rows} rows; with fewer than the spec's own \
         row count the witness below is about a run that did not happen"
    );

    let w = zero_optional_witnesses();
    assert!(
        w.is_empty(),
        "{} site(s) compared Some(०) against absent while building the \
         encoding table. At each of these the IMAGE answers the opposite of \
         what the interpreter just answered, because `सम्भाव्य` is one word \
         natively. Ask the TEXT whether the field is there — see \
         `सङ्केतन ॱ षोडशाङ्कवाचकः` — instead of reading it off the value:\n{}",
        w.len(),
        w.iter()
            .map(|(name, count)| format!("    {count:>6}  {name}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // The row the first witness named. `c.addi4spn` is the one row whose
    // pattern is 0x00000000, read out of the spec file HERE rather than
    // transcribed, so this cannot agree with a stale copy of itself.
    let tsv = std::fs::read_to_string(repo_root().join("spec/encodings-riscv64.tsv"))
        .expect("the encoding table is readable");
    let zero_pattern: Vec<&str> = tsv
        .lines()
        .filter(|l| l.split('\t').nth(3) == Some("0x00000000"))
        .filter_map(|l| l.split('\t').next())
        .collect();
    assert_eq!(
        zero_pattern,
        vec!["c.addi4spn"],
        "the spec table's all-zero-pattern rows changed; the witness above is \
         calibrated against there being exactly one, so this test's evidence \
         needs re-reading rather than its expectation updating"
    );
}

/// The predicate answers the question the value cannot.
///
/// `0x00000000` is a hexadecimal numeral and an empty field is not, and the
/// two are the same word once lowered — which is the whole reason
/// `षोडशाङ्कवाचकः` exists.
#[test]
fn the_hex_predicate_separates_a_zero_from_an_absence() {
    let mut it = load();
    let ask = |it: &mut Interpreter, text: &str| -> bool {
        let v = it
            .call(
                "सङ्केतनॱषोडशाङ्कवाचकः",
                vec![
                    Value::Octets(sadhana::t1::nirvahana::Octets::new(text.as_bytes())),
                    Value::Int(0),
                    Value::Int(text.len() as i128),
                ],
                20_000_000,
            )
            .unwrap_or_else(|e| panic!("`षोडशाङ्कवाचकः` on {text:?}: {e:?}"));
        matches!(v, Value::Bool(true))
    };

    assert!(
        ask(&mut it, "0x00000000"),
        "an all-zero pattern IS a hexadecimal numeral — this is the row \
         `c.addi4spn`, and calling it absent is the defect this predicate fixes"
    );
    assert!(ask(&mut it, "0x0000e003"), "an ordinary pattern is one too");
    assert!(!ask(&mut it, ""), "an empty field is not the number zero");
    assert!(!ask(&mut it, "-"), "a `-` cell refuses itself");
    assert!(!ask(&mut it, "pattern"), "the header refuses itself");
}
