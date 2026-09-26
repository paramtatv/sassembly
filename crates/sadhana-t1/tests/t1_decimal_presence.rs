//! A decimal field's PRESENCE is asked of the text, not of a one-word optional.
//!
//! # The failure this exists for
//!
//! `ir.t1` lays `सम्भाव्य` (kind ५) out as ONE WORD with ० meaning ABSENT, and
//! its own margin names the cost: an optional whose payload can legitimately be
//! ० is indistinguishable from absent in compiled code. `सङ्केतनॱशून्यविकल्प‌परीक्षा`
//! asked the machine on 2026-09-19 and the two halves disagreed — ७००१
//! interpreted, ७००२ in the image.
//!
//! `विश्लेषणॱन्यासरचना` was a consumer of exactly that. It reads the two cells
//! of a `v>e` pair off the encoding table with `सङ्केतनॱदशाङ्कमूल्यम्`, whose
//! answer is `सम्भाव्य न६४`, and wrote the pair only when both were
//! `असमम् शून्यम्`. `मूलांश` is a BIT NUMBER and bit ० is ordinary: 484 of the
//! 2801 pairs in `spec/encodings-riscv64.tsv` carry it, and every register slot
//! opens with one (`add` carries `0>7`, `0>15`, `0>20`). Natively those pairs
//! read as ABSENT and were skipped, so index ० of `न्यास` was never written;
//! `सङ्केतनॱअवकाशविस्तारः` walks `न्यास` from ० and read `युग्मम् ॱ मूलांश`
//! off a null. That is `BadAccess { pc: 0x80009cbc, addr: 0 }`.
//!
//! # The repair, and why it is this one
//!
//! Two repairs were on the table. Changing kind ५'s REPRESENTATION — a tag word
//! or a reserved sentinel — is the one `ir.t1`'s margin reserves for "whoever
//! lowers the first optional-unwrap"; it moves every `x समम् शून्यम्` in the
//! corpus (87 sites, 63 of them with a numeric payload) and is not a repair to
//! one routine. The local repair is to stop asking the optional a question it
//! cannot answer: whether a decimal cell is THERE is a property of the TEXT.
//!
//! So `सङ्केतनॱदशाङ्कवाचकः` answers that question directly, and
//! `दशाङ्कमूल्यम्` now refuses through it rather than carrying a second copy of
//! the digit bounds. The layout is untouched and nothing here claims otherwise.
//!
//! # What must still be REFUSED
//!
//! A repair that writes every pair unconditionally would also make the fault go
//! away, and would be wrong: a genuinely ABSENT optional must not arrive as a
//! present ०. `दशाङ्कवाचकः` therefore answers असत्यम् for exactly the ranges
//! `दशाङ्कमूल्यम्` answers शून्यम् for — an empty range, and a range holding an
//! octet outside ४८…५७ — and the pair test below feeds `न्यासरचना` a map with a
//! malformed cell and requires that entry to stay unwritten.
//!
//! # Why a behavioural test is not enough on its own, and what carries the rest
//!
//! The interpreter keeps `Some(०)` and `शून्यम्` apart, so the pair tests here
//! passed BEFORE the repair as well as after: they pin the behaviour, they do
//! not witness the divergence. The divergence is a property of the SOURCE — of
//! whether the routine asks the payload word at all — so the last test is a
//! shape gate over `न्यासरचना`'s body, with the pre-repair body embedded as a
//! literal and required to be REFUSED with both of its sites named.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

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

/// **THIS TEST'S OWN LOADER.** `lex.t1` is in every set that reads a table —
/// `पदविभागॱसमावेशपाठः` is how a reader reaches a spec file — and `विश्लेषण`
/// calls into `सङ्केतन` for both routines under test, so the set is the three.
fn load() -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", &source("encode.t1")),
            ("vishlesana.t1", &source("vishlesana.t1")),
        ],
        &spec_root(),
    )
    .expect("lex.t1, encode.t1 and vishlesana.t1 load")
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

const FUEL: u64 = 20_000_000;

/// `सङ्केतनॱदशाङ्कवाचकः पाठ्यम् ० <len>` over the whole of `text`.
fn is_decimal(it: &mut Interpreter, text: &str) -> bool {
    let n = i128::try_from(text.len()).expect("a fixture is short");
    let v = it
        .call(
            "सङ्केतनॱदशाङ्कवाचकः",
            vec![octets(text), Value::Int(0), Value::Int(n)],
            FUEL,
        )
        .unwrap_or_else(|e| panic!("दशाङ्कवाचकः over {text:?}: {e:?}"));
    match v {
        Value::Bool(b) => b,
        other => panic!("दशाङ्कवाचकः must answer बूल, answered {other:?}"),
    }
}

/// `सङ्केतनॱदशाङ्कमूल्यम् पाठ्यम् ० <len>`: `None` where it answered शून्यम्.
fn decimal_value(it: &mut Interpreter, text: &str) -> Option<i128> {
    let n = i128::try_from(text.len()).expect("a fixture is short");
    let v = it
        .call(
            "सङ्केतनॱदशाङ्कमूल्यम्",
            vec![octets(text), Value::Int(0), Value::Int(n)],
            FUEL,
        )
        .unwrap_or_else(|e| panic!("दशाङ्कमूल्यम् over {text:?}: {e:?}"));
    match v {
        Value::Nil => None,
        other => Some(
            other
                .as_int()
                .unwrap_or_else(|| panic!("दशाङ्कमूल्यम् answered {other:?}")),
        ),
    }
}

/// **`0` IS A DECIMAL FIELD, AND THAT IS THE ONE THE OPTIONAL COULD NOT SAY.**
///
/// `दशाङ्कमूल्यम्` answers `Some(०)` here and `शून्यम्` for the refused rows;
/// under a one-word optional those are the SAME word, which is the whole
/// defect. `दशाङ्कवाचकः` separates them in a `बूल`, which is one word that
/// means what it says.
#[test]
fn zero_is_present_and_a_non_digit_is_not() {
    let mut it = load();

    // PRESENT — `0` first, because it is the case the fault turned on.
    for t in ["0", "00", "7", "12", "2801", "9"] {
        assert!(
            is_decimal(&mut it, t),
            "{t:?} is a run of decimal digits and दशाङ्कवाचकः must say so"
        );
    }

    // REFUSED — and these must STAY refused, or the repair has merely turned
    // every absent optional into a present ०.
    for t in ["", "-", "1a", "a1", " 1", "1 ", "+1", "1.0", "०"] {
        assert!(
            !is_decimal(&mut it, t),
            "{t:?} is not a run of decimal digits and दशाङ्कवाचकः must refuse it"
        );
    }
}

/// **THE TWO ROUTINES AGREE ON THE REFUSAL SET, WHICH IS WHY ONE MAY GUARD THE
/// OTHER.** `दशाङ्कवाचकः` is not a second opinion about what a digit is — it is
/// the only one, and `दशाङ्कमूल्यम्` refuses through it. This asserts the
/// equivalence over the same fixtures rather than assuming it from the source.
#[test]
fn the_predicate_refuses_exactly_what_the_reader_refuses() {
    let mut it = load();
    for t in [
        "0", "00", "7", "12", "2801", "9", "", "-", "1a", "a1", " 1", "1 ", "+1", "1.0", "०",
    ] {
        assert_eq!(
            is_decimal(&mut it, t),
            decimal_value(&mut it, t).is_some(),
            "दशाङ्कवाचकः and दशाङ्कमूल्यम् disagree about {t:?}"
        );
    }
    // And the value itself, so the delegation did not cost the reading.
    assert_eq!(decimal_value(&mut it, "0"), Some(0));
    assert_eq!(decimal_value(&mut it, "2801"), Some(2801));
    assert_eq!(decimal_value(&mut it, "-"), None);
}

/// One `अंशयुग्म` read out of the arena `न्यासरचना` returns, or `None` where
/// the entry was never written.
fn pair_at(v: &Value, index: usize) -> Option<(i128, i128)> {
    let Value::Arena(a) = v else {
        panic!("न्यासरचना must answer an arena, answered {v:?}");
    };
    let a = a.borrow();
    match a.get(index) {
        None | Some(Value::Nil) => None,
        Some(Value::Record(r)) => {
            let r = r.borrow();
            Some((
                r.get("मूलांश").and_then(Value::as_int).expect("मूलांश"),
                r.get("सङ्केतांश").and_then(Value::as_int).expect("सङ्केतांश"),
            ))
        }
        Some(other) => panic!("entry {index} is {other:?}"),
    }
}

fn pair_list(it: &mut Interpreter, map: &str) -> Value {
    let n = i128::try_from(map.len()).expect("a fixture is short");
    it.call(
        "विश्लेषणॱन्यासरचना",
        vec![octets(map), Value::Int(0), Value::Int(n)],
        FUEL,
    )
    .unwrap_or_else(|e| panic!("न्यासरचना over {map:?}: {e:?}"))
}

/// **THE PAIR AT INDEX ० IS WRITTEN, AND IT IS THE `0>7` EVERY REGISTER SLOT
/// OPENS WITH.** `अवकाशविस्तारः` walks `न्यास` from ०, so this entry existing
/// is the precondition the fault violated.
#[test]
fn a_zero_value_bit_is_a_pair_and_lands_at_index_zero() {
    let mut it = load();
    let v = pair_list(&mut it, "0>7;1>8;2>9");
    assert_eq!(pair_at(&v, 0), Some((0, 7)), "the `0>7` pair");
    assert_eq!(pair_at(&v, 1), Some((1, 8)));
    assert_eq!(pair_at(&v, 2), Some((2, 9)));

    // `add`'s three register slots, as the file's own margin cites them.
    for map in ["0>7", "0>15", "0>20"] {
        let v = pair_list(&mut it, map);
        assert!(
            pair_at(&v, 0).is_some(),
            "{map} is one pair and index ० must carry it"
        );
    }
}

/// **AND A MALFORMED CELL IS STILL DROPPED.** This is the control that
/// separates the repair taken from one that writes every pair regardless: a
/// genuinely absent cell must not arrive as a present ०.
#[test]
fn a_cell_that_is_not_a_number_is_refused_and_does_not_become_zero() {
    let mut it = load();

    // Middle pair malformed on the left, then on the right, then both.
    for (map, bad) in [("0>7;-;2>9", 1), ("0>7;1>x;2>9", 1), ("0>7;;2>9", 1)] {
        let v = pair_list(&mut it, map);
        assert_eq!(
            pair_at(&v, bad),
            None,
            "{map}: entry {bad} is not a pair and must stay unwritten"
        );
        assert_eq!(pair_at(&v, 0), Some((0, 7)), "{map}: the good pair before");
        assert_eq!(pair_at(&v, 2), Some((2, 9)), "{map}: the good pair after");
    }
}

// ── the shape gate ───────────────────────────────────────────────────────

/// `॰`, ADR-0017's comment mark: the rest of the line, outside a literal.
const COMMENT_MARK: &str = "॰";

/// The numeric payload types. A RECORD- or RUN-typed optional is a reference
/// and ० genuinely IS how absence is spelled for one, so those are not
/// findings — that is the distinction `tools/optional-whether-census.py`
/// draws and this gate draws the same one.
const NUMERIC: &[&str] = &["न६४", "अ६४", "अ३२", "अ१६", "अ८", "इ६४", "इ३२"];

/// Every `यदि <v> समम्|असमम् शून्यम्` in `body` where `<v>` is a local declared
/// `सम्भाव्य <numeric>` in the same body, as `(line, name, payload)`.
///
/// SCOPED TO ONE ROUTINE BODY, and that is not a detail: a first pass at the
/// census keyed declarations by name across a whole FILE and matched a test of
/// some other `मूल्यम्` against a `सम्भाव्य मूल्यम्` declared 400 lines later in
/// a different routine. A name is not unique in a file.
fn numeric_whethers(body: &str) -> Vec<(usize, String, String)> {
    let mut declared: Vec<(String, String)> = Vec::new();
    let mut found = Vec::new();
    for (i, raw) in body.lines().enumerate() {
        let code = raw.split(COMMENT_MARK).next().unwrap_or("");
        // `\b` is useless here — Devanagari is not a word character to Rust's
        // or Python's matchers — so this splits on spaces, as the corpus is
        // written with every token spaced.
        let w: Vec<&str> = code.split_whitespace().collect();
        for k in 0..w.len() {
            if w[k] == "चरः"
                && k + 3 < w.len()
                && w[k + 2] == "ॱॱ"
                && w[k + 3] == "सम्भाव्य"
                && let Some(ty) = w.get(k + 4)
            {
                declared.push((w[k + 1].to_string(), (*ty).to_string()));
            }
            if w[k] == "यदि" && k + 3 < w.len() && w[k + 3] == "शून्यम्" {
                let (name, op) = (w[k + 1], w[k + 2]);
                if op != "समम्" && op != "असमम्" {
                    continue;
                }
                if let Some((_, ty)) = declared.iter().find(|(n, _)| n == name)
                    && NUMERIC.contains(&ty.as_str())
                {
                    found.push((i + 1, name.to_string(), ty.clone()));
                }
            }
        }
    }
    found
}

/// The body of `वृत्तिः <name>` in `text`, from its declaration line to the
/// next routine declaration.
fn routine_body(text: &str, name: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let heads: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            let w: Vec<&str> = l.split_whitespace().collect();
            w.first() == Some(&"वृत्तिः")
                || (w.first() == Some(&"सार्वजनिक") && w.get(1) == Some(&"वृत्तिः"))
        })
        .map(|(i, _)| i)
        .collect();
    let start = *heads
        .iter()
        .find(|&&i| lines[i].split_whitespace().any(|w| w == name))
        .unwrap_or_else(|| panic!("वृत्तिः {name} is declared"));
    let end = heads
        .iter()
        .copied()
        .find(|&i| i > start)
        .unwrap_or(lines.len());
    lines[start..end].join("\n")
}

/// The body `न्यासरचना` had before this repair, verbatim from `0e53512f`. The
/// gate must find BOTH of its sites, or the gate is not reading what it claims
/// to read and its green over the repaired body means nothing.
const THE_BODY_THE_FAULT_WAS_IN: &str = "\
सार्वजनिक वृत्तिः न्यासरचना आदाय पाठ्यम् ॱॱ अङ्कः अन्तः अ८ आदिः ॱॱ अ६४ अवसानम् ॱॱ अ६४ ददाति अङ्कः अन्तः अंशयुग्म आदि
    चरः न्यासः ॱॱ अङ्कः अन्तः अंशयुग्म भवति ० ।
    चरः संख्यानम् ॱॱ अ६४ भवति खण्डसंख्या पाठ्यम् आदिः अवसानम् ५९ ।
    चरः सूचकाङ्क ॱॱ अ६४ भवति ० ।

    यावत् सूचकाङ्क न्यूनम् संख्यानम् आदि
        चरः युग्मादिः ॱॱ अ६४ भवति खण्डारम्भः पाठ्यम् आदिः अवसानम् ५९ सूचकाङ्क ।
        चरः युग्मान्तः ॱॱ अ६४ भवति खण्डसीमा पाठ्यम् आदिः अवसानम् ५९ सूचकाङ्क ।
        चरः विरामः ॱॱ अ६४ भवति सङ्केतन ॱ अष्टकान्वेषणम् पाठ्यम् युग्मादिः युग्मान्तः ६२ ।
        चरः मूलम् ॱॱ सम्भाव्य अ६४ भवति सङ्केतन ॱ दशाङ्कमूल्यम् पाठ्यम् युग्मादिः विरामः ।
        चरः गन्तृ ॱॱ सम्भाव्य अ६४ भवति सङ्केतन ॱ दशाङ्कमूल्यम् पाठ्यम् आरभ्य विरामः योगः १ समाप्तम् युग्मान्तः ।

        यदि मूलम् असमम् शून्यम् आदि
            यदि गन्तृ असमम् शून्यम् आदि
                चरः नव ॱॱ सङ्केतनॱअंशयुग्म भवति ० ।
                नव ॱ मूलांश भवति मूलम् ।
                नव ॱ सङ्केतांश भवति गन्तृ ।
                न्यासः अङ्कः सूचकाङ्क अन्तः भवति नव ।
            इति
        इति

        सूचकाङ्क भवति सूचकाङ्क योगः १ ।
    इति

    प्रत्यागमनम् न्यासः ।
इति";

/// A reference-payload optional tested the same way. It must NOT be a finding:
/// ० genuinely is how absence is spelled for a record, and a gate that reds on
/// every `समम् शून्यम्` is a gate that gets deleted the first time someone
/// tests a nullable record — and then nothing refuses the next `मूलांश`.
const A_REFERENCE_OPTIONAL_IS_NOT_A_FINDING: &str = "\
सार्वजनिक वृत्तिः काचित् ददाति बूल आदि
    चरः वस्तु ॱॱ सम्भाव्य वास्तुॱवस्तु भवति शून्यम् ।
    यदि वस्तु समम् शून्यम् आदि
        प्रत्यागमनम् असत्यम् ।
    इति
    प्रत्यागमनम् सत्यम् ।
इति";

/// **THE GATE, AND ITS TWO CONTROLS.**
///
/// The repaired `न्यासरचना` asks no numeric `सम्भाव्य` whether it is present.
/// The pre-repair body must be refused with both sites named, and a
/// reference-payload optional must not be a finding at all.
#[test]
fn the_pair_reader_asks_no_one_word_optional_whether_it_is_there() {
    // CONTROL 1 — the body the fault was in is REFUSED, by content.
    let was = numeric_whethers(THE_BODY_THE_FAULT_WAS_IN);
    assert_eq!(
        was.len(),
        2,
        "the pre-repair न्यासरचना asks TWO numeric optionals whether they are \
         present; this gate found {was:?}, so it is not reading what it claims"
    );
    assert!(
        was.iter().any(|(_, n, _)| n == "मूलम्") && was.iter().any(|(_, n, _)| n == "गन्तृ"),
        "both sites are named: {was:?}"
    );

    // CONTROL 2 — a reference payload is not a finding.
    assert!(
        numeric_whethers(A_REFERENCE_OPTIONAL_IS_NOT_A_FINDING).is_empty(),
        "० IS how absence is spelled for a record; that must not red"
    );

    // THE GATE.
    let body = routine_body(&source("vishlesana.t1"), "न्यासरचना");
    let now = numeric_whethers(&body);
    assert!(
        now.is_empty(),
        "विश्लेषणॱन्यासरचना asks a numeric सम्भाव्य whether it is present. \
         Under `ir.t1`'s one-word layout that reads Some(०) as ABSENT in the \
         image, drops the pair, and leaves index ० of न्यास null for \
         अवकाशविस्तारः to dereference. Ask सङ्केतनॱदशाङ्कवाचकः of the TEXT \
         instead. Sites: {now:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `सङ्केतनॱस्थानसङ्केतनम्` — THE OPERAND'S VALUE, and the E02 refusal that
// used to ask it whether it is there.
// ─────────────────────────────────────────────────────────────────────────

/// **An operand whose value is ० is an ordinary operand.**
///
/// `मूल्याङ्कः` answers a `सम्भाव्य न६४` carrying an operand's VALUE, and it
/// answers correctly for `०` — it parses the numeral. The caller used to ask
/// that answer `समम् शून्यम्` and raise E02, and `ir.t1` lays kind ५ out as ONE
/// WORD with ० meaning ABSENT (`शून्यविकल्पपरीक्षा`: ७००१ interpreted, ७००२ in
/// the image). So a well-formed instruction was refused for carrying a zero.
///
/// IT WAS NOT A CORNER. The self-test's own ten-instruction program refused at
/// instruction २ — `sd s0, 0(sp)`, the first with a zero immediate — and FOUR of
/// its ten carry one. Nothing was emitted from there on and the object came back
/// EMPTY: status २००० against the interpreter's २०४०, where the status is
/// `2000 + len(object)`. After the repair, rung105 halts २०४० and the two
/// engines' traces are IDENTICAL over 586 records.
///
/// THE GATE IS NARROW ON PURPOSE. `स्थानसङ्केतनम्` legitimately asks other
/// one-word optionals WHETHER, and those are separate sites with their own
/// callers — `tools/optional-whether-census.py` still counts 61 numeric ones
/// corpus-wide. This asserts only that `मूल्याङ्कम्`, the operand VALUE, is not
/// among them, because that one is measured to receive a ०.
#[test]
fn the_operand_reader_does_not_ask_its_value_whether_it_is_there() {
    let body = routine_body(&source("encode.t1"), "स्थानसङ्केतनम्");
    assert!(
        !body.is_empty(),
        "`वृत्तिः स्थानसङ्केतनम्` is in encode.t1; if it was renamed, re-point \
         this gate rather than deleting it"
    );

    // THE POSITIVE CONTROL RUNS FIRST. A matcher that has not been shown to
    // find the defect cannot be evidence of its absence, and this exact
    // shape is what the source carried until 407d3b62.
    const PRE_REPAIR: &str = "\
            चरः मूल्याङ्कम् ॱॱ सम्भाव्य न६४ भवति मूल्याङ्कः मूलम् क्षेत्रयोग्यम् ।
            यदि मूल्याङ्कम् समम् शून्यम् आदि
                प्रत्यागमनम् शून्यम् ।
            इति
";
    let before = numeric_whethers(PRE_REPAIR);
    assert!(
        before.iter().any(|(_, n, _)| n == "मूल्याङ्कम्"),
        "the pre-repair body must be a finding — it is what this gate exists \
         to refuse; instead the matcher reported {before:?}"
    );

    let found = numeric_whethers(&body);
    assert!(
        !found.iter().any(|(_, n, _)| n == "मूल्याङ्कम्"),
        "`मूल्याङ्कम्` carries an operand's VALUE and ० is an ordinary value, so \
         asking it `शून्यम्` refuses a well-formed instruction natively. Ask \
         `मूल्याङ्कवाचकः` — whether the operand IS a value is a property of the \
         TEXT. Findings in स्थानसङ्केतनम्: {found:?}"
    );
}
