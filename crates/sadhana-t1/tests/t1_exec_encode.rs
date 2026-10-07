//! Executing `सङ्केतन ॱ मूल्याङ्कः` — the port of `encode.rs`'s `value_of`.
//!
//! **This file exists so that five agents editing `t1_execution.rs` do not
//! merge into each other.** Its helpers — `source`, `load`, `octets`,
//! `mutate` — are copies of that file's, deliberately, because the cost of
//! sharing them was measured and it was higher than the cost of two copies.
//!
//! # What this file is evidence for, and what it is not
//!
//! `मूल्याङ्कः` was the whole of `encode.t1`'s blocker (c). Its three clauses
//! are a register lookup, a fence-domain lookup and `numeral::bits`, and the
//! third had **no T1 port at all** until `अक्षरकोश ॱ अंशाः` and
//! `अक्षरकोश ॱ अंशदोषः` landed in `sanskrit_text.t1`. So this file is testing
//! a routine that could not have been written yesterday, and the tests are
//! shaped to say WHICH of the three clauses each one exercises.
//!
//! It is NOT evidence that the encoder encodes anything. The eight routines
//! that take an `आज्ञा` or a `कार्यक्रम` are still stubs, and
//! `t1_sources.rs`'s `programs.len()` floor is still ० — see the report in
//! that file for the gap each of them names.
//!
//! # Every expectation here is derived, never transcribed
//!
//! The register and domain expectations are read out of `spec/*.tsv` by THIS
//! file, with Rust's own `lines`/`split`, so the T1 reader is checked against
//! the file and not against a second copy of itself. The numeral expectations
//! are not read from a table at all: a number is chosen, rendered into
//! Devanagari digits by `devanagari()` below, and the routine is asked to read
//! back the number that was rendered. Nothing about the numeral reader's
//! internals is written down here for it to agree with.

use sadhana::t1::nirvahana::{Interpreter, Octets, RunError, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Building a substitute `spec/` root — shared with the other binary that does
/// it, because two copies of this routine carried the same defect.
mod spec_fixture;

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
/// A mutation applied to text that does not contain it changes nothing, and
/// the test it guards then passes for the wrong reason. A mutation applied to
/// two places is not the mutation that was described. Both are refused.
fn mutate(text: &str, from: &str, to: &str) -> String {
    let n = text.matches(from).count();
    assert_eq!(
        n, 1,
        "the mutation `{from}` -> `{to}` matches {n} places in the source; \
         a mutation test is only evidence when it changes exactly one"
    );
    text.replace(from, to)
}

/// `encode.t1` and `sanskrit_text.t1`, either of them optionally mutated.
///
/// **Both, and not `encode.t1` alone.** `मूल्याङ्कः` reaches across a module
/// boundary for the first time in this file — `अक्षरकोशॱअंशाः` — so an
/// interpreter holding only the encoder would resolve the register and the
/// domain clauses and fail on the third. That is precisely the clause the row
/// was blocked on, so loading one module would test everything except the
/// thing that changed.
/// A `RunError` with the interpreter's OWN module list attached when — and
/// only when — the refusal is a cross-module name that is not loaded.
///
/// **A TEST LOADER IS PART OF THE TEST, AND THE BARE `RunError` DOES NOT SAY
/// SO.** On 2026-09-19 `a_malformed_relation_refuses_its_form_by_name` failed
/// with `` `अष्टकॱमुद्रणम्` is not a name in scope ``, which reads as a defect
/// in `encode.t1`. It was not: `40b9c34a` had put a debug marker calling
/// `चतुरष्टकमुद्रणम्` on an unconditional path, that routine reaches into
/// `अष्टक`, and no loader in this file carries `अष्टक`. The refusal was
/// correct and the message pointed at the wrong file for a day.
///
/// **THE LIST IS READ OFF THE INTERPRETER, NEVER WRITTEN DOWN.** This file has
/// several loaders — `load` carries three modules, the malformed-relation test
/// builds its own five — so a `const` naming one of them would be false at
/// most call sites, which is worse than saying nothing. `declarations()`
/// answers what was actually loaded, so the message cannot drift from the
/// loader it is describing.
fn explain(it: &Interpreter, e: &RunError) -> String {
    let reason = e.reason.clone();
    if !reason.contains("is not a name in scope") {
        return reason;
    }
    // The missing name is the last backquoted run before the phrase.
    let Some(name) = reason
        .split("is not a name in scope")
        .next()
        .and_then(|head| head.rsplit('`').nth(1))
    else {
        return reason;
    };
    // Unqualified names are ordinary scope errors in the module being tested;
    // only a `ॱ` name is a statement about the LOADER.
    let Some((module, _)) = name.split_once('ॱ') else {
        return reason;
    };
    let modules: Vec<String> = it.declarations().into_iter().map(|d| d.name).collect();
    // JOINED BY HAND, NOT `{:?}`. Debug on a `Vec<String>` escapes every
    // combining mark — `सङ\u{94d}केतन` — so the one list the reader must read
    // arrives unreadable. Display, separated, keeps the Devanagari intact.
    let loaded = modules.join(", ");
    if modules.iter().any(|m| m == module) {
        // The module IS loaded and the routine still is not there: a real
        // missing declaration, and pointing at the loader would misdirect.
        return reason;
    }
    format!(
        "{reason}\n  THE LOADER IS PART OF THE TEST. `{name}` is a \
         cross-module name and `{module}` is NOT among the modules this \
         interpreter loaded: {loaded:?}. So the refusal is correct and the \
         defect is upstream of it — either the caller should not be reaching \
         `{module}` at all (a debug marker left on an unconditional path is \
         the usual cause; `40b9c34a` was exactly that), or this test wants a \
         loader that carries `{module}`."
    )
}

fn load(encode: &str, sanskrit_text: &str) -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", encode),
            ("sanskrit_text.t1", sanskrit_text),
        ],
        &spec_root(),
    )
    .expect("the two T1 sources load")
}

fn unmutated() -> Interpreter {
    load(&source("encode.t1"), &source("sanskrit_text.t1"))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// `मूल्याङ्कः base domains_fit`, as `Option<i128>`; `None` where the routine
/// answered `शून्यम्`.
///
/// A RUN FAILURE PANICS RATHER THAN BECOMING `None`. The two are different
/// answers — "the routine says there is no value" and "the routine did not
/// run" — and collapsing them is how a stub that returns a string reads as a
/// well-behaved refusal.
fn value_of(it: &mut Interpreter, base: &str, domains_fit: bool) -> Option<i128> {
    it.call(
        "सङ्केतनॱमूल्याङ्कः",
        vec![octets(base), Value::Bool(domains_fit)],
        20_000_000,
    )
    .unwrap_or_else(|e| panic!("मूल्याङ्कः {base:?} {domains_fit} runs: {e:?}"))
    .as_int()
}

// ─────────────────────────────────────────────────────────────────────────
// The three spec tables, read by THIS file.
// ─────────────────────────────────────────────────────────────────────────

/// Every `(name, number)` of `spec/registers-riscv64.tsv` — Rust's `register`,
/// whose row filter is `f.len() > 3` and whose number is `f[2]`.
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

/// Every `(name, bit)` of `spec/fence-domains-riscv64.tsv` — Rust's
/// `domain_set`'s per-part lookup, whose filter is `f.len() >= 3` and whose
/// bit is `f[2]`.
fn domain_table() -> Vec<(String, u64)> {
    let text = std::fs::read_to_string(spec_root().join("fence-domains-riscv64.tsv"))
        .expect("spec/fence-domains-riscv64.tsv exists");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| !l.starts_with("devanagari\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 3 {
                return None;
            }
            f[2].parse::<u64>().ok().map(|b| (f[0].to_string(), b))
        })
        .collect()
}

/// `n` written in Devanagari digits — U+0966 `०` through U+096F `९`.
///
/// This is the whole of what the numeral tests assume about the language, and
/// it is one line of the Unicode standard rather than anything read out of
/// this tree. `०` for zero, and no leading zero otherwise.
fn devanagari(mut n: u64) -> String {
    if n == 0 {
        return "\u{0966}".to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(
            char::from_u32(0x0966 + u32::try_from(n % 10).expect("a digit")).expect("a digit"),
        );
        n /= 10;
    }
    out.reverse();
    out.into_iter().collect()
}

/// `ऋण` — `sanskrit_text::numeral::NEGATIVE`, the sign that sits OUTSIDE the
/// radix prefix.
const NEGATIVE: &str = "\u{090B}\u{0923}";

// ─────────────────────────────────────────────────────────────────────────
// Clause one — a register name.
// ─────────────────────────────────────────────────────────────────────────

/// **The first clause, over the WHOLE table.** Rust returns `Some(u64::from(n))`
/// for every name `register` knows, whatever `domains_fit` says — the flag
/// guards the SECOND clause only, and a port that guarded the first with it
/// would still pass a test that only ever passed `true`. So both are asked.
#[test]
fn a_register_name_answers_its_number_from_the_spec_table() {
    let mut it = unmutated();
    let table = register_table();
    assert!(
        table.len() >= 30,
        "only {} register rows read out of spec/registers-riscv64.tsv; the \
         table reader in this test is not reading the file",
        table.len()
    );
    println!("METRIC sadhana_t1_value_of_register_rows {}", table.len());

    for (name, n) in &table {
        for fit in [false, true] {
            assert_eq!(
                value_of(&mut it, name, fit),
                Some(*n),
                "मूल्याङ्कः {name} {fit} must answer {n}, the number \
                 spec/registers-riscv64.tsv carries for it"
            );
        }
    }

    // **THE ROW THIS TEST EXISTS FOR.** `शून्यः` is register ०, and ० is also
    // what `कोष्ठपङ्क्तिः` returns for a name it does not know — which is why
    // `कोष्ठाङ्कः` answers ONE-BASED. `मूल्याङ्कः` must undo that: Rust's
    // `value_of` answers the number that goes into the field. The first draft
    // of the port did not, and every register in the ISA came out one too
    // high; this assertion names the register the defect was worst for.
    assert_eq!(
        value_of(&mut it, "शून्यः", false),
        Some(0),
        "शून्यः is the hardwired zero and मूल्याङ्कः must answer ०, not \
         कोष्ठाङ्कः's one-based १ — an instruction naming it would otherwise \
         encode x1"
    );
}

/// The off-by-one, asserted as a mutation rather than only as a value.
///
/// Without this, a port that dropped the subtraction would be caught by the
/// test above — but nothing would say that the subtraction is what caught it,
/// and a later reader could delete it as redundant with `कोष्ठाङ्कः`'s own
/// margin. Restoring the bug must move every register by exactly one.
#[test]
fn dropping_the_one_based_correction_moves_every_register_by_one() {
    let mutated = mutate(
        &source("encode.t1"),
        "        प्रत्यागमनम् कोष्ठम् वियोगः १ ।",
        "        प्रत्यागमनम् कोष्ठम् ।",
    );
    let mut it = load(&mutated, &source("sanskrit_text.t1"));

    for (name, n) in register_table() {
        assert_eq!(
            value_of(&mut it, &name, false),
            Some(n + 1),
            "with the correction removed मूल्याङ्कः must answer कोष्ठाङ्कः's \
             one-based number; if {name} still answers {n} the subtraction was \
             not what produced it"
        );
    }
}

/// The guard on the test above: a name the table does not carry, and which is
/// not a numeral or a domain either, must answer `शून्यम्` — Rust's fall
/// through all three clauses to `None`.
#[test]
fn a_name_no_clause_knows_answers_nothing() {
    let mut it = unmutated();
    for base in ["अविद्यमानम्", "क्षणिकः", "मुद्रकः"]
    {
        for fit in [false, true] {
            assert_eq!(
                value_of(&mut it, base, fit),
                None,
                "{base} is not a register, not a domain and not a numeral, so \
                 मूल्याङ्कः must answer शून्यम् for it"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Clause two — a fence ordering domain, and the flag that gates it.
// ─────────────────────────────────────────────────────────────────────────

/// **The second clause, and `B-107`'s flag.** Every single domain name answers
/// its bit when `क्षेत्रयोग्यम्` is true and answers `शून्यम्` when it is
/// false. The false half is the one that matters: `पठनम्` is "reading" as well
/// as the memory-read domain, and reading it as a domain everywhere is what
/// made a label of that name definable and unreferenceable.
#[test]
fn a_domain_name_answers_its_bit_only_where_domains_fit() {
    let mut it = unmutated();
    let table = domain_table();
    assert_eq!(
        table.len(),
        4,
        "spec/fence-domains-riscv64.tsv carries {} domain rows, not four; \
         if the table gained one, say which in this assertion",
        table.len()
    );
    println!("METRIC sadhana_t1_value_of_domain_rows {}", table.len());

    for (name, bit) in &table {
        assert_eq!(
            value_of(&mut it, name, true),
            Some(i128::from(*bit)),
            "मूल्याङ्कः {name} सत्यम् must answer {bit}, the bit \
             spec/fence-domains-riscv64.tsv carries for it"
        );
        assert_eq!(
            value_of(&mut it, name, false),
            None,
            "मूल्याङ्कः {name} असत्यम् must answer शून्यम्: Rust guards the \
             domain clause with `domains_fit &&`, and B-107 is the record of \
             what dropping that guard cost"
        );
    }
}

/// A SET of domains, `ऽ`-joined — Rust's `base.split('ऽ')` folded with `|`.
///
/// Built from the file's own rows rather than written out, so the expected
/// bits are the OR of what the file says and not a number recalled from the
/// ISA manual.
#[test]
fn a_joined_domain_set_answers_the_or_of_its_parts() {
    let mut it = unmutated();
    let table = domain_table();

    // Every ordered pair of distinct domains, and the four of them together.
    let mut cases: Vec<(String, u64)> = Vec::new();
    for (a, ba) in &table {
        for (b, bb) in &table {
            if a != b {
                cases.push((format!("{a}ऽ{b}"), ba | bb));
            }
        }
    }
    let all: String = table
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join("ऽ");
    cases.push((all, table.iter().map(|(_, b)| b).fold(0, |a, b| a | b)));

    for (base, bits) in &cases {
        assert_eq!(
            value_of(&mut it, base, true),
            Some(i128::from(*bits)),
            "मूल्याङ्कः {base} सत्यम् must answer {bits}, the OR of the bits \
             its parts carry in spec/fence-domains-riscv64.tsv"
        );
    }

    // `bits & b != 0` — the same domain named twice is refused outright, and
    // is NOT the same as OR-ing a bit into itself.
    for (name, _) in &table {
        assert_eq!(
            value_of(&mut it, &format!("{name}ऽ{name}"), true),
            None,
            "{name}ऽ{name} names one domain twice; Rust returns None rather \
             than the single bit"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Clause three — `numeral::bits`, the clause that was unportable.
// ─────────────────────────────────────────────────────────────────────────

/// **The third clause.** A number is rendered into Devanagari digits and the
/// routine is asked to read it back. The expectation is the number that was
/// rendered, so there is no table here to agree with — which is the point:
/// `numeral::bits` is not a spec file, and a test that copied its arithmetic
/// would be checking the port against a transcription of the port.
#[test]
fn a_decimal_numeral_answers_the_number_it_was_written_from() {
    let mut it = unmutated();

    // Small numbers, every one; then powers and neighbours out to the top of
    // the range 64 bits hold.
    let mut cases: Vec<u64> = (0..=64).collect();
    for k in 0..64 {
        let p = 1u64 << k;
        cases.push(p);
        cases.push(p - 1);
        if let Some(n) = p.checked_add(1) {
            cases.push(n);
        }
    }
    cases.push(u64::MAX);
    cases.push(u64::MAX - 1);
    cases.push(1729);
    cases.sort_unstable();
    cases.dedup();

    for n in &cases {
        let written = devanagari(*n);
        assert_eq!(
            value_of(&mut it, &written, false),
            Some(i128::from(*n)),
            "मूल्याङ्कः {written} must answer {n}, the number those digits \
             were written from"
        );
    }
    println!("METRIC sadhana_t1_value_of_numerals {}", cases.len());

    // The gate cannot touch a numeral — it guards the domain clause only, and
    // no numeral is a domain name. A handful rather than the whole list,
    // because each call walks the register table octet by octet and the whole
    // list twice is minutes rather than seconds.
    for n in [0u64, 1, 1729, u64::MAX] {
        let written = devanagari(n);
        assert_eq!(
            value_of(&mut it, &written, true),
            Some(i128::from(n)),
            "क्षेत्रयोग्यम् guards the domain clause; it must not change what \
             {written} reads as"
        );
    }
}

/// `ऋण` is two's complement and not a negation — `numeral::bits`'s whole
/// reason for existing beside `numeral::value`.
///
/// The expectation is `wrapping_neg` computed HERE, on the same `u64` the
/// literal was written from. `ऋण०` is `०` and not `2^64`, and
/// `ऋण9223372036854775808` is exactly `i64::MIN`'s bits — the magnitude that
/// negating through `i64` cannot reach.
#[test]
fn a_negative_numeral_answers_its_twos_complement() {
    let mut it = unmutated();

    let mut cases: Vec<u64> = (0..=64).collect();
    for k in 0..63 {
        cases.push(1u64 << k);
    }
    cases.push(1u64 << 63); // i64::MIN's magnitude — the last one that fits.
    cases.push((1u64 << 63) - 1); // i64::MAX.
    cases.sort_unstable();
    cases.dedup();

    for n in &cases {
        let written = format!("{NEGATIVE}{}", devanagari(*n));
        assert_eq!(
            value_of(&mut it, &written, false),
            Some(i128::from(n.wrapping_neg())),
            "मूल्याङ्कः {written} must answer the two's complement of {n}"
        );
    }

    // Past 2^63 a ऋण magnitude is TooLarge, and `.ok()` makes that शून्यम्.
    for n in [(1u64 << 63) + 1, u64::MAX] {
        let written = format!("{NEGATIVE}{}", devanagari(n));
        assert_eq!(
            value_of(&mut it, &written, false),
            None,
            "ऋण{n} is past the negative range 64 bits hold; numeral::bits \
             answers TooLarge and `.ok()` makes it शून्यम्"
        );
    }
}

/// The other radices, and the malformed literal.
///
/// `०द्वि`, `०अष्ट` and `०षोड्` are the binary, octal and hexadecimal
/// prefixes. The values are built by rendering the SAME number in decimal and
/// asking that the two agree — so the test says "these are the same number",
/// which is what a radix is, and does not write down a digit table.
#[test]
fn a_prefixed_numeral_agrees_with_the_same_number_written_in_decimal() {
    let mut it = unmutated();

    // The hexadecimal digits past nine, in the spelling `numeral.rs` gives
    // them. These are the ONE thing this test cannot derive: they are a
    // human decision recorded in the module, not arithmetic.
    const HEX: [&str; 16] = [
        "\u{0966}", "\u{0967}", "\u{0968}", "\u{0969}", "\u{096A}", "\u{096B}", "\u{096C}",
        "\u{096D}", "\u{096E}", "\u{096F}", "\u{0905}", "\u{0906}", "\u{0907}", "\u{0908}",
        "\u{0909}", "\u{090A}",
    ];

    let render = |mut n: u64, radix: u64, prefix: &str| -> String {
        let mut digits: Vec<&str> = Vec::new();
        if n == 0 {
            digits.push(HEX[0]);
        }
        while n > 0 {
            digits.push(HEX[usize::try_from(n % radix).expect("a digit")]);
            n /= radix;
        }
        digits.reverse();
        format!("{prefix}{}", digits.concat())
    };

    let radices: [(u64, &str); 3] = [
        (2, "\u{0966}\u{0926}\u{094D}\u{0935}\u{093F}"), // ०द्वि
        (8, "\u{0966}\u{0905}\u{0937}\u{094D}\u{091F}"), // ०अष्ट
        (16, "\u{0966}\u{0937}\u{094B}\u{0921}\u{094D}"), // ०षोड्
    ];

    let mut asked = 0usize;
    for (radix, prefix) in radices {
        for n in [
            0u64,
            1,
            2,
            7,
            8,
            63,
            64,
            255,
            256,
            1729,
            0xdead_beef,
            u64::MAX,
        ] {
            let written = render(n, radix, prefix);
            assert_eq!(
                value_of(&mut it, &written, false),
                Some(i128::from(n)),
                "मूल्याङ्कः {written} must answer {n} — the same number \
                 {} answers in decimal",
                devanagari(n)
            );
            asked += 1;
        }
    }
    println!("METRIC sadhana_t1_value_of_prefixed_numerals {asked}");

    // A prefix with no digits after it is `NoDigits`, and `.ok()` is शून्यम्.
    for (_, prefix) in radices {
        assert_eq!(
            value_of(&mut it, prefix, false),
            None,
            "{prefix} is a radix prefix with no digits; numeral::bits refuses it"
        );
    }

    // A decimal magnitude past u64::MAX is TooLarge and NOT a saturated
    // answer — the defect `W-075` is the record of, where a literal too large
    // by any amount assembled as `u64::MAX` and said nothing. The literal is
    // built by appending a digit to `u64::MAX`, so it is past the range by
    // construction and no constant is transcribed.
    let past_the_top = format!("{}\u{0966}", devanagari(u64::MAX));
    assert_eq!(
        value_of(&mut it, &past_the_top, false),
        None,
        "{past_the_top} is ten times u64::MAX; numeral::bits answers TooLarge \
         rather than saturating, and `.ok()` makes that शून्यम्"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Mutations.
// ─────────────────────────────────────────────────────────────────────────

/// **The order of the three clauses is load-bearing.** Moving the domain
/// clause ahead of the register clause changes nothing today — no name is
/// both — so the mutation this file makes is the one that CAN be observed:
/// dropping the `क्षेत्रयोग्यम्` gate, which is `B-107`'s bug exactly.
#[test]
fn dropping_the_domains_fit_gate_is_caught() {
    let mutated = mutate(
        &source("encode.t1"),
        "    यदि क्षेत्रयोग्यम् आदि\n        चरः समूहः",
        "    यदि सत्यम् आदि\n        चरः समूहः",
    );
    let mut it = load(&mutated, &source("sanskrit_text.t1"));

    let (name, bit) = domain_table().into_iter().next().expect("a domain row");
    assert_eq!(
        value_of(&mut it, &name, true),
        Some(i128::from(bit)),
        "the mutation must not change the सत्यम् answer, or it is not the \
         mutation described"
    );
    assert_eq!(
        value_of(&mut it, &name, false),
        Some(i128::from(bit)),
        "with the gate replaced by सत्यम् the domain must now answer even \
         when the family takes none — if it still answers शून्यम्, the gate \
         was never what decided and this test proves nothing"
    );
}

/// The third clause is REACHED, and is not shadowed by the first two.
///
/// Breaking the numeral call alone must change a numeral's answer and leave
/// every register's answer alone. Without this, a `मूल्याङ्कः` that had lost
/// its numeral clause entirely would still pass every register and domain
/// test in this file.
#[test]
fn breaking_the_numeral_clause_changes_only_the_numeral_answers() {
    let mutated = mutate(
        &source("encode.t1"),
        "    प्रत्यागमनम् अक्षरकोशॱअंशाः आरभ्य मूल ऽ ० ऽ मूल ॱ दैर्घ्य समाप्तम् ।",
        "    प्रत्यागमनम् शून्यम् ।",
    );
    let mut it = load(&mutated, &source("sanskrit_text.t1"));

    assert_eq!(
        value_of(&mut it, &devanagari(1729), false),
        None,
        "with the अंशाः call removed a numeral must answer शून्यम्; if it \
         still answers 1729 the clause was not what produced it"
    );

    let (name, n) = register_table().into_iter().next().expect("a register row");
    assert_eq!(
        value_of(&mut it, &name, false),
        Some(n),
        "the register clause returns before the numeral clause is reached, so \
         removing the numeral clause must leave it untouched"
    );
}

/// The fault reader is not decorative: `अंशाः` answers ० for a base it
/// refuses, and ० is a legitimate value, so `अंशदोषः` is the only thing that
/// tells `ऋण` past the range apart from `०`.
#[test]
fn removing_the_numeral_fault_check_makes_a_refused_literal_read_as_zero() {
    let mutated = mutate(
        &source("encode.t1"),
        // RE-POINTED 2026-09-19, SAME PROPERTY, NEW SITE. The numeral fault
        // check moved into `मूल्याङ्कवाचकः` when the caller stopped asking a
        // one-word optional WHETHER it is there, and `मूल्याङ्कः` now refuses
        // THROUGH that predicate. Removing the refusal is still exactly what
        // makes a refused literal fall through to `अंशाः`'s ०; only the three
        // lines that express it have changed. Anchoring on the old text would
        // have left this test passing over a mutation that no longer applies —
        // which is why `mutate` refuses an anchor it cannot find.
        "    यदि मूल्याङ्कवाचकः मूल क्षेत्रयोग्यम् समम् असत्यम् आदि\n        प्रत्यागमनम् शून्यम् ।\n    इति",
        "",
    );
    let mut it = load(&mutated, &source("sanskrit_text.t1"));

    let too_large = format!("{NEGATIVE}{}", devanagari(u64::MAX));
    assert_eq!(
        value_of(&mut it, &too_large, false),
        Some(0),
        "without the fault check a refused literal reads as अंशाः's ० — which \
         is exactly why the check is there and not an ornament"
    );
    assert_eq!(
        value_of(&mut it, "अविद्यमानम्", false),
        Some(0),
        "and a name that is no numeral at all reads as ० too"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `सङ्केतनदोषवचनम्` — `EncodeError::message`, the routine whose margin named
// its blocker as one row in a registry.
// ─────────────────────────────────────────────────────────────────────────

/// `encode.t1`, `nidana.t1` and `vakyavibhaga.t1`.
///
/// **Three, and every one is load-bearing.** The renderer is
/// `निदान ॱ विवरणम्`; the arena its arguments live in is
/// `वाक्यविभाग ॱ पाठांशकोश`; and `सङ्केतनदोषवचनम्` is in `encode.t1`. Drop
/// any one and the call does not resolve.
fn load_diagnostics(encode: &str) -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", encode),
            ("nidana.t1", &source("nidana.t1")),
            ("vakyavibhaga.t1", &source("vakyavibhaga.t1")),
            ("ashtaka.t1", &source("ashtaka.t1")),
        ],
        &spec_root(),
    )
    .expect("the four T1 sources load")
}

/// One row of `spec/diagnostics.tsv`: `(code, term, sanskrit, english)`.
///
/// Read by THIS test with Rust's own `lines`/`split`, so the rendering is
/// checked against the file `निदानकोशः` names and not against a second copy
/// of the registry.
fn diagnostic_rows() -> Vec<(String, String, String, String)> {
    let text = std::fs::read_to_string(spec_root().join("diagnostics.tsv"))
        .expect("spec/diagnostics.tsv exists");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter(|l| !l.starts_with("code\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 4 {
                return None;
            }
            Some((
                f[0].to_string(),
                f[1].to_string(),
                f[2].to_string(),
                f[3].to_string(),
            ))
        })
        .collect()
}

/// `{0}`…`{9}` in `template` replaced by `args`, exactly as `render` does.
///
/// An argument the template names but the caller did not supply renders as
/// nothing — `nidana.rs`'s own rule, and `पदार्थलेखनम्`'s: a compiler that
/// crashes while explaining a mistake has replaced the user's problem with
/// its own.
fn expand(template: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let b: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '{' && i + 2 < b.len() && b[i + 2] == '}' && b[i + 1].is_ascii_digit() {
            let n = (b[i + 1] as usize) - ('0' as usize);
            out.push_str(args.get(n).copied().unwrap_or(""));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// Fill `निदानपङ्क्तिकोश` from `spec/diagnostics.tsv` by calling
/// `निदान ॱ निदानपङ्क्तियोजनम्` once per row.
///
/// **THIS IS NOT `निदानपङ्क्तयः` AND DOES NOT STAND IN FOR IT.** That reader
/// — `nidana.t1:759`, the one stub in that file — is owned by another row and
/// is being written elsewhere; nothing here writes it or depends on how it
/// will. What this does is put the registry in the state that reader will
/// leave it in, from the same file, so `सङ्केतनदोषवचनम्` can be asked what it
/// renders once a registry exists. The per-row appender it calls is already
/// written and already public.
fn fill_registry(it: &mut Interpreter) -> usize {
    let rows = diagnostic_rows();
    for (code, term, sanskrit, english) in &rows {
        it.call(
            "निदानॱनिदानपङ्क्तियोजनम्",
            vec![
                octets(code),
                octets(term),
                octets(sanskrit),
                octets(english),
            ],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("निदानपङ्क्तियोजनम् {code} runs: {e:?}"));
    }
    rows.len()
}

/// Push `args` into `वाक्यविभाग ॱ पाठांशकोश` and answer the run
/// `(पदार्थारम्भ, पदार्थसंख्यान)` — Rust's `&args`, in the order the template
/// names them.
fn push_args(it: &mut Interpreter, args: &[&str]) -> (i128, i128) {
    let mut first = 0i128;
    for (n, a) in args.iter().enumerate() {
        let len = i128::try_from(a.len()).expect("fits");
        let at = it
            .call(
                "वाक्यविभागॱपाठांशयोजनम्",
                vec![octets(a), Value::Int(0), Value::Int(len)],
                20_000_000,
            )
            .unwrap_or_else(|e| panic!("पाठांशयोजनम् {a} runs: {e:?}"))
            .as_int()
            .expect("an index");
        if n == 0 {
            first = at;
        }
    }
    (first, i128::try_from(args.len()).expect("fits"))
}

/// A `सङ्केतनदोष` record, built field by field — the encoder's own struct,
/// which nothing in `encode.t1` constructs yet because the eight routines
/// that would are still stubs.
fn encode_error(code: &str, run: (i128, i128), reason: &str) -> Value {
    let mut m = std::collections::HashMap::new();
    m.insert("पङ्क्ति".to_string(), Value::Int(1));
    m.insert("सङ्केताङ्क".to_string(), octets(code));
    m.insert("पदार्थारम्भ".to_string(), Value::Int(run.0));
    m.insert("पदार्थसंख्यान".to_string(), Value::Int(run.1));
    m.insert("कारण".to_string(), octets(reason));
    Value::Record(std::rc::Rc::new(std::cell::RefCell::new(m)))
}

/// The raw octets `सङ्केतनदोषवचनम्` answers, as bytes — NOT as a `String`,
/// because what comes back is not always valid UTF-8. See `THE TWO
/// PROPERTIES OF विवरणम्` below.
fn message_bytes(it: &mut Interpreter, err: Value, lang: i128) -> Vec<u8> {
    let v = it
        .call(
            "सङ्केतनॱसङ्केतनदोषवचनम्",
            vec![err, Value::Int(lang)],
            50_000_000,
        )
        .unwrap_or_else(|e| panic!("सङ्केतनदोषवचनम् runs: {e:?}"));
    match v {
        Value::Octets(o) => o.as_slice().to_vec(),
        other => panic!("सङ्केतनदोषवचनम् must answer octets, answered {other:?}"),
    }
}

// ── THE TWO PROPERTIES OF `निदान ॱ विवरणम्` THIS FILE FOUND ──────────────
//
// Both belong to `nidana.t1` and NEITHER is `सङ्केतनदोषवचनम्`'s to fix.
// `सङ्केतनदोषवचनम्` is Rust's two-line `EncodeError::message` and returns
// whatever the renderer returns, exactly as its sibling `निदान ॱ सन्देश`
// (`nidana.t1:895`, `ParseError::message`) does — so both properties are
// `सन्देश`'s too, and were before this file existed.
//
// **(1) A LEADING NUL — FIXED 2026-08-30.** `विवरणयोजनम्` advanced
// `विवरणसूचकाङ्क` *before* it wrote, so slot ० was never written and stayed
// `०`, and `विवरणम्` returns the whole buffer — so every rendered message
// began with a NUL. The write is now AT the cursor and the advance follows.
// A byte buffer is zero-based; the one-based convention belongs to the ARENAS,
// whose slot ० is deliberately never a live entry, and conflating the two is
// the whole of this defect. The assertions below now require the NUL to be
// ABSENT, so the defect cannot return unnoticed.
//
// **(2) THE BUFFER WAS NEVER TRUNCATED — FIXED 2026-08-30.** `विवरणारम्भः`
// reset the INDEX to ० without shortening `विवरणकोश`, and `विवरणम्` returns
// the buffer rather than its first `विवरणसूचकाङ्क` octets — which T1 cannot
// express, having no subslice. So a SHORT message rendered after a LONG one
// inherited the long one's tail, and a tail can begin mid-character, so the
// result was not even valid UTF-8. `विवरणारम्भः` now EMPTIES the buffer by
// assigning an empty run, the idiom वाक्यविभाग:914 already uses, so returning
// it whole is correct rather than merely usually correct.
// `an_unregistered_code_renders_as_the_code…` and
// `a_short_message_after_a_long_one_inherits_the_tail` pin both, so that a
// repair in `nidana.t1` fails here loudly and says which lines to strike
// rather than passing unnoticed.
//
// Every content assertion in this file therefore renders in a FRESH
// interpreter, where property (2) cannot bite and property (1) is one byte.
const NUL: u8 = 0;

/// One render, in an interpreter that has rendered nothing before it.
///
/// The registry is filled from `spec/diagnostics.tsv` first, so this is what
/// `सङ्केतनदोषवचनम्` answers once `निदान ॱ निदानपङ्क्तयः` lands.
fn render_once(encode: &str, code: &str, args: &[&str], lang: i128) -> Vec<u8> {
    let mut it = load_diagnostics(encode);
    fill_registry(&mut it);
    let run = push_args(&mut it, args);
    message_bytes(&mut it, encode_error(code, run, "अप्रयुक्तम्"), lang)
}

/// `render_once`, with the leading NUL of property (1) stripped and the rest
/// read as UTF-8.
fn rendered(encode: &str, code: &str, args: &[&str], lang: i128) -> String {
    let raw = render_once(encode, code, args, lang);
    // REPAIRED 2026-08-30: this helper used to strip a leading NUL, because
    // `विवरणयोजनम्` advanced its cursor BEFORE writing and slot ० stayed ०.
    // The write is now AT the cursor, so the first octet is real content and
    // stripping one would eat it. Asserted rather than assumed — if a NUL
    // returns, this says so instead of silently losing a character.
    assert_ne!(
        raw.first().copied(),
        Some(NUL),
        "{code} rendered with a leading NUL again — the one-based write in \
         `विवरणयोजनम्` has come back"
    );
    String::from_utf8(raw).expect("a fresh render is UTF-8")
}

const SANSKRIT: i128 = 1;
const ENGLISH: i128 = 2;

/// **The acceptance.** Every row of `spec/diagnostics.tsv`, in both languages,
/// rendered through the encoder's own `message` and asserted against the
/// template the FILE carries — expanded here by `expand`, which is three lines
/// of `{n}` substitution and reads nothing out of this tree.
#[test]
fn every_diagnostic_renders_the_sentence_the_spec_file_carries() {
    let encode = source("encode.t1");
    let rows = diagnostic_rows();
    assert_eq!(
        rows.len(),
        53,
        "spec/diagnostics.tsv carries {} rows of four columns, not 53. If the \
         registry gained a diagnostic, change this number and say which — do \
         not loosen it to a floor",
        rows.len()
    );
    println!("METRIC sadhana_t1_diagnostic_rows {}", rows.len());

    // Four arguments, which is more than any template names — `expand` drops
    // the surplus and so must the routine.
    let args = ["योगः", "२", "३", "४"];

    for (code, _term, sanskrit, english) in &rows {
        for (lang, template) in [(SANSKRIT, sanskrit), (ENGLISH, english)] {
            let want = expand(template, &args);
            let got = rendered(&encode, code, &args, lang);
            assert_eq!(
                got, want,
                "सङ्केतनदोषवचनम् rendered {code} in language {lang} as {got:?}; \
                 spec/diagnostics.tsv's template is {template:?}"
            );
        }
    }
}

/// **The two properties of `विवरणम्` this file found, pinned so a repair is
/// loud.** Neither is `सङ्केतनदोषवचनम्`'s and neither is asserted anywhere
/// else; both are `निदान ॱ सन्देश`'s too.
///
/// If `nidana.t1` starts returning the first `विवरणसूचकाङ्क` octets instead of
/// the whole buffer, this test fails — and that is the good news. Strike it,
/// and strike the NUL from `rendered` in the same commit.
#[test]
fn a_short_message_after_a_long_one_inherits_the_tail() {
    let mut it = load_diagnostics(&source("encode.t1"));
    fill_registry(&mut it);
    let run = push_args(&mut it, &["योगः"]);

    // The longest and the shortest template the file carries, so the effect
    // is maximal and the two codes are chosen by the FILE rather than named.
    let rows = diagnostic_rows();
    let long = rows
        .iter()
        .max_by_key(|(_, _, s, _)| s.len())
        .expect("a longest row");
    let short = rows
        .iter()
        .min_by_key(|(_, _, s, _)| s.len())
        .expect("a shortest row");
    assert!(
        long.2.len() > short.2.len() + 8,
        "the longest and shortest Sanskrit templates are too close for this \
         test to show anything"
    );

    // A first render is the message and NOTHING ELSE — no leading NUL.
    let first = message_bytes(&mut it, encode_error(&long.0, run, "अप्रयुक्तम्"), SANSKRIT);
    assert_ne!(first.first().copied(), Some(NUL), "a leading NUL came back");
    assert_eq!(
        String::from_utf8(first.clone()).expect("utf-8"),
        expand(&long.2, &["योगः"])
    );

    // THE REPAIR. A shorter render after a longer one comes back at ITS OWN
    // length. Before `विवरणारम्भः` emptied the buffer it came back padded with
    // the longer message's tail — and a tail can begin mid-character, so the
    // result was not even valid UTF-8. That is how the defect was found:
    // `String::from_utf8` blew up on L01's English with Sanskrit welded on.
    let second = message_bytes(&mut it, encode_error(&short.0, run, "अप्रयुक्तम्"), SANSKRIT);
    assert_eq!(
        String::from_utf8(second.clone()).expect("a second render is still UTF-8"),
        expand(&short.2, &["योगः"]),
        "the short message did not come back at its own length — विवरणकोश is \
         carrying a previous render's tail again"
    );
    assert!(
        second.len() < first.len(),
        "the shorter template rendered no shorter than the longest one, so the \
         buffer is not being truncated"
    );
}

/// The two languages must actually DIFFER somewhere, or the test above is
/// satisfied by a routine that ignores `भाषा` entirely.
#[test]
fn the_language_argument_chooses_a_column_and_the_columns_differ() {
    let encode = source("encode.t1");
    let differing: Vec<String> = diagnostic_rows()
        .into_iter()
        .filter(|(_, _, s, e)| s != e)
        .map(|(c, ..)| c)
        .collect();
    assert!(
        differing.len() >= 50,
        "only {} rows of spec/diagnostics.tsv have different Sanskrit and \
         English templates; this test cannot tell the columns apart",
        differing.len()
    );

    for code in differing.iter().take(12) {
        let sa = rendered(&encode, code, &["योगः"], SANSKRIT);
        let en = rendered(&encode, code, &["योगः"], ENGLISH);
        assert_ne!(
            sa, en,
            "{code} rendered the same text in both languages; भाषा is not \
             choosing a column"
        );
    }
}

/// **`B-078c`'s fallback, and the half that was never blocked.** A diagnostic
/// with no code at all answers its own `कारण` verbatim — Rust's
/// `if self.code.is_empty() { return self.reason.clone(); }`.
#[test]
fn a_diagnostic_with_no_code_answers_its_own_reason() {
    // NOT through `rendered`: the fallback returns `कारण` itself and never
    // touches `विवरणकोश`, so there is no leading NUL to strip — and that
    // difference is worth asserting, because it is what "falls back" means.
    for lang in [SANSKRIT, ENGLISH] {
        let mut it = load_diagnostics(&source("encode.t1"));
        fill_registry(&mut it);
        let run = push_args(&mut it, &["योगः"]);
        let raw = message_bytes(&mut it, encode_error("", run, "हस्तेन लिखितम्"), lang);
        assert_eq!(
            String::from_utf8(raw).expect("utf-8"),
            "हस्तेन लिखितम्",
            "an empty कूट must answer कारण verbatim and unpadded, in either \
             language — the fallback returns the field, not the render buffer"
        );
    }
}

/// **A code the registry does not carry renders as the code itself.**
///
/// This is `nidana.rs`'s rule and it is the reason `सङ्केतनदोषवचनम्` is
/// correct while `निदानपङ्क्तयः` is still a stub: with an EMPTY arena every
/// code takes this path, which is a defined answer and not a fault. The
/// registry is deliberately NOT filled here.
#[test]
fn an_unregistered_code_renders_as_the_code_and_an_empty_registry_is_not_a_fault() {
    // A real code, with nothing in the arena to find it in — one fresh
    // interpreter each, and `fill_registry` deliberately not called.
    for (code, lang) in [("E01", SANSKRIT), ("ZZ99", ENGLISH)] {
        let mut it = load_diagnostics(&source("encode.t1"));
        let run = push_args(&mut it, &["योगः"]);
        let raw = message_bytes(&mut it, encode_error(code, run, "अप्रयुक्तम्"), lang);
        assert_ne!(raw.first().copied(), Some(NUL), "a leading NUL came back");
        assert_eq!(
            String::from_utf8(raw.clone()).expect("utf-8"),
            code,
            "with निदानपङ्क्तिकोश empty every code takes the unregistered path \
             and renders as itself. THIS IS THE ROUTINE'S CORRECTNESS PROOF \
             WHILE निदानपङ्क्तयः IS A STUB: an empty registry is a defined \
             answer, not a fault"
        );
    }
}

/// The routine reads the RUN it is given, and a wrong run renders wrong.
///
/// Without this, a `सङ्केतनदोषवचनम्` that passed `०`/`०` through to
/// `विवरणम्` would still pass every test above whose template names no
/// argument — and `E01`'s does name one.
#[test]
fn the_argument_run_is_carried_through_to_the_renderer() {
    let encode = source("encode.t1");
    let (code, _, sanskrit, _) = diagnostic_rows()
        .into_iter()
        .find(|(c, ..)| c == "E01")
        .expect("spec/diagnostics.tsv carries E01");
    assert!(
        sanskrit.contains("{0}"),
        "E01's template no longer names an argument; this test is vacuous"
    );

    // Two different arguments, each in its own interpreter, so the difference
    // can only come from the run the record carries.
    assert_eq!(
        rendered(&encode, &code, &["योगः"], SANSKRIT),
        expand(&sanskrit, &["योगः"])
    );
    assert_eq!(
        rendered(&encode, &code, &["निधानम्"], SANSKRIT),
        expand(&sanskrit, &["निधानम्"]),
        "the second run must render the second argument; if both render the \
         same word the run is not being read"
    );

    // An argument the caller did not supply renders as nothing, not a crash —
    // `पदार्थसंख्यान` ०, which is what an args-less diagnostic carries.
    let mut it = load_diagnostics(&encode);
    fill_registry(&mut it);
    let raw = message_bytes(&mut it, encode_error(&code, (0, 0), "अप्रयुक्तम्"), SANSKRIT);
    // `raw` whole, not `raw[1..]`: the leading NUL this used to skip is gone
    // since `विवरणयोजनम्` writes AT its cursor, and skipping one now eats a
    // real octet — here E01's opening backtick, which is exactly what the
    // failure showed.
    assert_eq!(
        String::from_utf8(raw.clone()).expect("utf-8"),
        expand(&sanskrit, &[]),
        "an argument the caller did not supply renders as nothing rather than \
         as a crash"
    );
}

/// Mutation: breaking the empty-code fallback must change ONLY the no-code
/// answer.
#[test]
fn breaking_the_empty_code_fallback_is_caught() {
    let mutated = mutate(
        &source("encode.t1"),
        "    यदि सङ्केतनदोषः ॱ सङ्केताङ्क ॱ दैर्घ्य समम् ० आदि\n        प्रत्यागमनम् सङ्केतनदोषः ॱ कारण ।\n    इति",
        "",
    );
    // NOT through `rendered`: with the fallback gone, an empty कूट reaches
    // विवरणम्, which walks the code's zero octets and writes NOTHING — so
    // `विवरणकोश` is never advanced and comes back EMPTY, without even the NUL
    // of its slot ०. That is the sharpest possible statement of the
    // difference, and it is why this assertion is on bytes.
    let mut it = load_diagnostics(&mutated);
    fill_registry(&mut it);
    let run = push_args(&mut it, &["योगः"]);
    assert_eq!(
        message_bytes(&mut it, encode_error("", run, "हस्तेन लिखितम्"), SANSKRIT),
        Vec::<u8>::new(),
        "without the fallback an empty कूट falls through to विवरणम्, which \
         renders it as nothing at all. If this still answers हस्तेन लिखितम् \
         the fallback was not what produced it"
    );
    // A registered code is unaffected: the fallback was never on its path.
    let (code, _, sanskrit, _) = diagnostic_rows()
        .into_iter()
        .find(|(c, ..)| c == "E01")
        .expect("E01");
    assert_eq!(
        rendered(&mutated, &code, &["योगः"], SANSKRIT),
        expand(&sanskrit, &["योगः"])
    );
}

/// Mutation: the language argument must be PASSED, not defaulted.
#[test]
fn passing_a_fixed_language_to_the_renderer_is_caught() {
    let mutated = mutate(
        &source("encode.t1"),
        "    प्रत्यागमनम् निदानॱविवरणम् दोषम् भाषा ।",
        "    प्रत्यागमनम् निदानॱविवरणम् दोषम् निदानॱसंस्कृतभाषा ।",
    );
    let (code, _, sanskrit, english) = diagnostic_rows()
        .into_iter()
        .find(|(c, _, s, e)| c == "E01" && s != e)
        .expect("E01's two templates differ");

    assert_eq!(
        rendered(&mutated, &code, &["योगः"], ENGLISH),
        expand(&sanskrit, &["योगः"]),
        "with भाषा replaced by a constant, asking for English must now answer \
         Sanskrit; if it still answers {english:?} the argument was never what \
         chose the column"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// The candidate walk — `encode_collecting`'s first filter, and the import
// that the ADR question at the head of `encode.t1` was about.
//
// # What these tests are evidence for
//
// `encode.t1`'s blocker (c) recorded that calling `विश्लेषण ॱ पङ्क्तिसङ्केतः`
// from the encoder "is an ADR and not a coding decision, which is why the
// eight are still stubs and not half-written". These tests are the evidence
// that settled it the other way: the import LOADS, the routine RUNS, and its
// answer agrees with `spec/encodings-riscv64.tsv` read by this file.
//
// Every expectation is DERIVED from the spec file by this test, with Rust's
// own `lines`/`split`, under `encodings()`'s own row filter. Nothing about
// the table is written down here for the T1 routines to agree with.
// ═════════════════════════════════════════════════════════════════════════

/// `encode.t1` and `vishlesana.t1` and `sanskrit_text.t1`, the encoder
/// optionally mutated.
///
/// **Three sources and not two.** `कुलक्षेत्रयोग्यम्` reaches
/// `विश्लेषण ॱ क्षेत्रवाचकः`, so an interpreter holding only the encoder
/// would resolve every other clause and fail on that one — which is the
/// clause the import exists for.
fn load_with_vishlesana(encode: &str) -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", encode),
            ("vishlesana.t1", &source("vishlesana.t1")),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
        ],
        &spec_root(),
    )
    .expect("encode.t1, vishlesana.t1 and sanskrit_text.t1 load together")
}

fn encodings_text() -> String {
    std::fs::read_to_string(spec_root().join("encodings-riscv64.tsv"))
        .expect("spec/encodings-riscv64.tsv exists")
}

/// One row of the encoding table, with the byte span it occupies.
struct Row {
    start: usize,
    end: usize,
    insn: String,
    family: String,
    bits: u64,
    pattern: u64,
    slots: usize,
}

/// Every row `encodings()` keeps, in file order, with its byte span.
///
/// The filter is Rust's own, at `encode.rs:257` and `:260`: not a `#`
/// comment, not the `insn\t` header, not blank, and at least seven fields.
fn encoding_rows() -> Vec<Row> {
    let text = encodings_text();
    let mut out = Vec::new();
    let mut at = 0usize;
    for line in text.split('\n') {
        let start = at;
        at += line.len() + 1;
        if line.starts_with('#') || line.starts_with("insn\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 7 {
            continue;
        }
        let (Ok(pattern), Ok(bits)) = (
            u64::from_str_radix(f[3].trim_start_matches("0x"), 16),
            f[5].parse::<u64>(),
        ) else {
            continue;
        };
        out.push(Row {
            start,
            end: start + line.len(),
            insn: f[0].to_string(),
            family: f[1].to_string(),
            bits,
            pattern,
            slots: if f[6] == "(none)" {
                0
            } else {
                f[6].split('|').count()
            },
        });
    }
    assert!(
        out.len() > 100,
        "spec/encodings-riscv64.tsv parsed to only {} rows; the filter is wrong",
        out.len()
    );
    out
}

fn record_field(v: &Value, name: &str) -> Value {
    match v {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("the record carries a field {name}")),
        other => panic!("{other:?} is not a record"),
    }
}

fn text_of(v: &Value) -> String {
    String::from_utf8(
        v.octets()
            .unwrap_or_else(|| panic!("{v:?} is octets"))
            .as_slice()
            .to_vec(),
    )
    .expect("utf-8")
}

fn arena_len(v: &Value) -> usize {
    match v {
        Value::Arena(a) => a.borrow().len(),
        other => panic!("{other:?} is not an arena"),
    }
}

// ── the import, and the routine the ADR question was about ───────────────

/// **The whole of the ADR decision, as a fact rather than an argument.**
///
/// `विश्लेषण` imports `सङ्केतन` and `सङ्केतन` now imports `विश्लेषण`. If a
/// cycle were an obstacle this would not load; it loads, and the row reader
/// answers the row `spec/encodings-riscv64.tsv` carries.
///
/// The two cycles that were already there — `सङ्केतन ↔ अक्षरकोश` and
/// `सङ्केतन ↔ निदान` — are exercised by every other test in this file, which
/// is why the decision could be taken rather than escalated.
#[test]
fn the_encoder_reaches_the_row_reader_it_imports() {
    let mut it = load_with_vishlesana(&source("encode.t1"));
    let table = encodings_text();
    let rows = encoding_rows();

    // Every twenty-third row, so the walk covers the file rather than its
    // first screen, and the whole suite still runs in seconds.
    for row in rows.iter().step_by(23) {
        let v = it
            .call(
                "विश्लेषणॱपङ्क्तिसङ्केतः",
                vec![
                    octets(&table),
                    Value::Int(i128::try_from(row.start).expect("fits")),
                    Value::Int(i128::try_from(row.end).expect("fits")),
                ],
                50_000_000,
            )
            .unwrap_or_else(|e| panic!("पङ्क्तिसङ्केतः on {:?} runs: {e:?}", row.insn));

        assert_eq!(text_of(&record_field(&v, "आज्ञा")), row.insn);
        assert_eq!(text_of(&record_field(&v, "कुल")), row.family);
        assert_eq!(
            record_field(&v, "आकृति").as_int(),
            Some(i128::from(row.pattern)),
            "the pattern of {}",
            row.insn
        );
        assert_eq!(
            record_field(&v, "अंशसंख्या").as_int(),
            Some(i128::from(row.bits)),
            "the width of {}",
            row.insn
        );
        // The slot list is ZERO-based (`पङ्क्तिसङ्केतः`'s own margin), and an
        // arena is born with one slot, so a row with no slots reports १.
        assert_eq!(
            arena_len(&record_field(&v, "अवकाशाः")),
            row.slots.max(1),
            "the slot count of {}",
            row.insn
        );
    }
}

// ── the family-keyed search ──────────────────────────────────────────────

fn family_count(it: &mut Interpreter, family: &str, bits: i128) -> i128 {
    it.call(
        "सङ्केतनॱकुलसङ्केताः",
        vec![octets(family), Value::Int(bits)],
        50_000_000,
    )
    .unwrap_or_else(|e| panic!("कुलसङ्केताः {family} {bits} runs: {e:?}"))
    .as_int()
    .expect("a count")
}

/// **The second row search, over the whole table.** For every family the
/// spec file names, at each width it is written at, the T1 walk must count
/// exactly the rows this test counts by splitting the file itself.
///
/// This is the search `encode.t1` records as keyed on FIELD १ where
/// `सङ्केताः` keys on FIELD ०. A port that reused `सङ्केताः` would answer
/// the MNEMONIC's rows, so the two are asked to disagree below.
#[test]
fn the_family_walk_counts_the_rows_the_spec_table_carries() {
    let mut it = load_with_vishlesana(&source("encode.t1"));
    let rows = encoding_rows();

    let mut families: Vec<(String, u64)> =
        rows.iter().map(|r| (r.family.clone(), r.bits)).collect();
    families.sort();
    families.dedup();

    for (family, bits) in &families {
        let want = rows
            .iter()
            .filter(|r| &r.family == family && r.bits == *bits)
            .count();
        assert_eq!(
            family_count(&mut it, family, i128::from(*bits)),
            i128::try_from(want).expect("fits"),
            "कुलसङ्केताः {family:?} {bits} must count the rows of \
             spec/encodings-riscv64.tsv whose FIELD १ is {family:?}"
        );
    }

    // A family nothing names answers ०, which is Rust's `candidates.is_empty()`
    // and the E01 branch of `encode_collecting`.
    assert_eq!(family_count(&mut it, "नास्तिकुलम्", 32), 0);
}

/// **The family key and the mnemonic are different questions, and the table
/// proves it rather than this file asserting it.**
///
/// `सङ्केताः` keys on field ० and `कुलसङ्केताः` on field १. There is at
/// least one family whose rows are NOT the rows of the mnemonic spelled the
/// same way — that is what `B-043` was raised for — and if the two searches
/// ever agreed everywhere, keying the candidate walk on the mnemonic would
/// be harmless and this routine would not need to exist.
#[test]
fn the_family_walk_and_the_mnemonic_walk_are_not_the_same_search() {
    let rows = encoding_rows();
    let differing: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            let by_family = rows
                .iter()
                .filter(|o| o.family == r.family && o.bits == r.bits)
                .count();
            let by_insn = rows
                .iter()
                .filter(|o| o.insn == r.family && o.bits == r.bits)
                .count();
            by_family != by_insn
        })
        .collect();
    assert!(
        !differing.is_empty(),
        "no family in spec/encodings-riscv64.tsv has a row set that differs \
         from the same-spelled mnemonic's; the candidate walk would then not \
         need to key on field १ at all"
    );

    let mut it = load_with_vishlesana(&source("encode.t1"));
    let r = differing[0];
    let by_family = rows
        .iter()
        .filter(|o| o.family == r.family && o.bits == r.bits)
        .count();
    assert_eq!(
        family_count(&mut it, &r.family, i128::from(r.bits)),
        i128::try_from(by_family).expect("fits"),
        "{:?} is a family whose rows differ from the mnemonic's, and \
         कुलसङ्केताः must answer the FAMILY's",
        r.family
    );
}

/// **The nth candidate, in file order.** `encode.rs:1251` records that 30
/// families are ambiguous and that FILE ORDER decides among them until
/// `B-056` names them apart, so the order is load-bearing and not incidental.
#[test]
fn the_family_walk_answers_each_candidate_row_in_file_order() {
    let mut it = load_with_vishlesana(&source("encode.t1"));
    let rows = encoding_rows();

    let mut families: Vec<(String, u64)> =
        rows.iter().map(|r| (r.family.clone(), r.bits)).collect();
    families.sort();
    families.dedup();

    for (family, bits) in families.iter().step_by(7) {
        let want: Vec<&Row> = rows
            .iter()
            .filter(|r| &r.family == family && r.bits == *bits)
            .collect();
        for (n, row) in want.iter().enumerate() {
            let got = it
                .call(
                    "सङ्केतनॱकुलसङ्केतपङ्क्तिः",
                    vec![
                        octets(family),
                        Value::Int(i128::from(*bits)),
                        Value::Int(i128::try_from(n).expect("fits")),
                    ],
                    50_000_000,
                )
                .unwrap_or_else(|e| panic!("कुलसङ्केतपङ्क्तिः {family} {n} runs: {e:?}"))
                .as_int()
                .expect("a row");
            // ONE-BASED, for `कोष्ठपङ्क्तिः`'s reason: offset ० is a real row
            // and ० is also the answer for "no such row".
            assert_eq!(
                got,
                i128::try_from(row.start + 1).expect("fits"),
                "the {n}th candidate of {family:?} at {bits} bits"
            );
        }
        // One past the last is ०.
        let past = it
            .call(
                "सङ्केतनॱकुलसङ्केतपङ्क्तिः",
                vec![
                    octets(family),
                    Value::Int(i128::from(*bits)),
                    Value::Int(i128::try_from(want.len()).expect("fits")),
                ],
                50_000_000,
            )
            .unwrap_or_else(|e| panic!("कुलसङ्केतपङ्क्तिः past the end runs: {e:?}"))
            .as_int();
        assert_eq!(past, Some(0), "one past the last candidate of {family:?}");
    }
}

// NOTE — `domains_fit` HAD A TEST HERE AND IT PASSED.
//
// `कुलक्षेत्रयोग्यम्` (Rust's `candidates.iter().any(|e| e.takes_domains)`,
// encode.rs:1161) was written, and this file asserted it against
// `spec/encodings-riscv64.tsv` for every family in the table — including the
// interesting part, that `विश्लेषण ॱ क्षेत्रवाचकः`'s "the field OPENS with
// iorw" answers the same question as Rust's `split(',').any(== "iorw")` over
// the rows that actually exist.
//
// Both were withdrawn because `the_corpus_carries_call_sites_that_cannot_be_
// executed` (t1_execution.rs:656) loads `encode.t1` and `vakyavibhaga.t1` and
// nothing else, and then requires every routine it cannot run to be named in
// `NEEDS_A_MODULE_NOT_LOADED` (:653). A routine in `encode.t1` that calls into
// `विश्लेषण` is unrunnable there by construction — exactly as `मूल्याङ्कः` and
// `सङ्केतनदोषवचनम्` are, and both of those are listed. Adding the row is one
// line in a file this row was told not to touch, so the routine waits for
// `encode_collecting`, its only caller, which needs `पङ्क्तिसङ्केतः` anyway
// and can add the row in the same commit.
//
// The import itself stays, and `the_encoder_reaches_the_row_reader_it_imports`
// above is the evidence for the decision. Nothing about it is provisional; only
// its first call site is.

// ── mutations ────────────────────────────────────────────────────────────

/// Keying the candidate walk on the MNEMONIC instead of the family — the
/// `B-043` defect, which made every `load` an `lb`.
#[test]
fn keying_the_candidate_walk_on_the_mnemonic_is_caught() {
    // THE MUTATION MOVED WITH THE WALK (2026-09-14). It used to flip field १ to
    // field ० in `क्षेत्रसाम्यम्`, and that line is still in the file — but the
    // candidate walk stopped going through it when the table index landed, so
    // the mutation applied cleanly, changed a routine nothing on this path
    // calls, and the control answered the same count either way. A negative
    // control that no longer touches the path under test is a check that cannot
    // fail. The index reads BOTH fields, into `सङ्केतसूच्याज्ञा*कोश` (field ०,
    // the mnemonic) and `सङ्केतसूचीकुल*कोश` (field १, the family), so B-043 has
    // an exact shape here: give the FAMILY bounds the mnemonic's field.
    let mutated = mutate(
        &source("encode.t1"),
        "सङ्केतसूचीकुलादिकोश अङ्कः सङ्केतसूचीसंख्या अन्तः भवति क्षेत्रारम्भः पाठ्यम् आरम्भः सीमा १ ।\n                सङ्केतसूचीकुलान्तकोश अङ्कः सङ्केतसूचीसंख्या अन्तः भवति क्षेत्रसीमा पाठ्यम् आरम्भः सीमा १ ।",
        "सङ्केतसूचीकुलादिकोश अङ्कः सङ्केतसूचीसंख्या अन्तः भवति क्षेत्रारम्भः पाठ्यम् आरम्भः सीमा ० ।\n                सङ्केतसूचीकुलान्तकोश अङ्कः सङ्केतसूचीसंख्या अन्तः भवति क्षेत्रसीमा पाठ्यम् आरम्भः सीमा ० ।",
    );
    let mut it = load_with_vishlesana(&mutated);
    let rows = encoding_rows();

    let differing = rows
        .iter()
        .find(|r| {
            rows.iter()
                .filter(|o| o.family == r.family && o.bits == r.bits)
                .count()
                != rows
                    .iter()
                    .filter(|o| o.insn == r.family && o.bits == r.bits)
                    .count()
        })
        .expect("a family whose rows differ from the mnemonic's");

    let by_family = rows
        .iter()
        .filter(|o| o.family == differing.family && o.bits == differing.bits)
        .count();
    assert_ne!(
        family_count(&mut it, &differing.family, i128::from(differing.bits)),
        i128::try_from(by_family).expect("fits"),
        "with field ० read in place of field १, the candidate walk must stop \
         answering the FAMILY's rows for {:?}",
        differing.family
    );
}

/// Dropping the width filter. `encode_collecting` is called with a `bits` of
/// 32 or of 16 and must not mix the two, or a compressed row becomes a
/// candidate for a wide instruction.
#[test]
fn dropping_the_width_filter_from_the_candidate_walk_is_caught() {
    // MOVED WITH THE WALK, as above: the width test the walk takes is now
    // `सूचितांशसाम्यम्`, which compares the width PARSED ONCE when the index was
    // built. Taking the width from the argument instead of from the row makes
    // the comparison vacuously true, which is the same defect the old mutation
    // wrote as `प्रत्यागमनम् सत्यम्` in the routine this one replaced.
    let mutated = mutate(
        &source("encode.t1"),
        "चरः मूल्यम् ॱॱ अ६४ भवति सङ्केतसूच्यंशकोश अङ्कः सूचीक्रमः अन्तः ।",
        "चरः मूल्यम् ॱॱ अ६४ भवति अंशाः ।",
    );
    let mut it = load_with_vishlesana(&mutated);
    let rows = encoding_rows();

    // A family written at BOTH widths is what can tell the difference.
    let mixed = rows
        .iter()
        .find(|r| {
            rows.iter()
                .any(|o| o.family == r.family && o.bits != r.bits)
        })
        .expect("some family is written at two widths in spec/encodings-riscv64.tsv");
    let at_width = rows
        .iter()
        .filter(|o| o.family == mixed.family && o.bits == mixed.bits)
        .count();
    assert_ne!(
        family_count(&mut it, &mixed.family, i128::from(mixed.bits)),
        i128::try_from(at_width).expect("fits"),
        "with the width test replaced by सत्यम्, {:?} must stop reporting \
         only its {}-bit rows",
        mixed.family,
        mixed.bits
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Two findings about the INTERPRETER that the eight stubs turn on.
//
// Neither is `encode.t1`'s to fix and both were recorded the other way
// round, so they are pinned here as executable facts rather than left as
// prose in a margin that has twice been wrong about this tree.
// ═════════════════════════════════════════════════════════════════════════

/// **A `कार्यक्रम` CAN be handed to the interpreter, and this is the test
/// that says so.**
///
/// `t1_sources.rs` recorded, and `encode.t1`'s blocker (c) repeated, that
/// "nothing can hand a `कार्यक्रम` to the interpreter: `वाक्यविभाग ॱ
/// सङ्कलनम्` answers a COUNT, not a Program, so an encoder written here
/// could be asserted against nothing but itself". That is true of T1 and
/// false of the TEST, which is where the assertion would live:
/// `Interpreter::invoke` (nirvahana.rs:613) checks ARITY and nothing else,
/// and `Value::Record` is a plain `HashMap` this file already builds — see
/// `encode_error` above, which has been constructing a `सङ्केतनदोष` field by
/// field since the day `सङ्केतनदोषवचनम्` landed.
///
/// So the eight are NOT blocked on being untestable. What blocks them is
/// listed at `encode.t1`'s foot, and `स्थानविन्यासः`'s share of it is the
/// `गणना` gap the next test pins.
#[test]
fn a_program_record_reaches_a_t1_routine_and_its_runs_are_readable() {
    // A module that does what `स्थानविन्यासः` would do and nothing else:
    // take a `कार्यक्रम`, walk the instruction run it names, and answer
    // something derived from the arena. Written here rather than in
    // `encode.t1` because the claim under test is about the INTERPRETER.
    let probe = concat!(
        "मण्डलम् प्रोब ॥\n",
        "आयातः वाक्यविभाग ।\n",
        "सार्वजनिक वृत्तिः पङ्क्तियोगः आदाय कार्यक्रमः ॱॱ वाक्यविभागॱकार्यक्रम ददाति अ६४ आदि\n",
        "    चरः क्रमः ॱॱ अ६४ भवति ० ।\n",
        "    चरः फलम् ॱॱ अ६४ भवति ० ।\n",
        "    यावत् क्रमः न्यूनम् कार्यक्रमः ॱ आज्ञासंख्यान आदि\n",
        "        चरः आज्ञा ॱॱ वाक्यविभागॱआज्ञा भवति वाक्यविभागॱवाक्यविभागआज्ञाकोश अङ्कः आरभ्य कार्यक्रमः ॱ आज्ञारम्भ योगः क्रमः समाप्तम् अन्तः ।\n",
        "        फलम् भवति फलम् योगः आज्ञा ॱ पङ्क्ति ।\n",
        "        क्रमः भवति क्रमः योगः १ ।\n",
        "    इति\n",
        "    प्रत्यागमनम् फलम् ।\n",
        "इति\n",
    );
    let mut it = Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("vakyavibhaga.t1", &source("vakyavibhaga.t1")),
            ("ashtaka.t1", &source("ashtaka.t1")),
            ("probe.t1", probe),
        ],
        &spec_root(),
    )
    .expect("vakyavibhaga.t1 loads");

    // Push three instructions through the appender the corpus already has,
    // exactly as a parser would.
    let family = {
        let mut m = std::collections::HashMap::new();
        m.insert("कूट".to_string(), octets("add"));
        m.insert("नाम".to_string(), octets("योगः"));
        m.insert("व्याख्या".to_string(), octets("-"));
        m.insert("विस्तारः".to_string(), octets("I"));
        m.insert("आवरणारम्भ".to_string(), Value::Int(0));
        m.insert("आवरणसंख्यान".to_string(), Value::Int(0));
        Value::Record(std::rc::Rc::new(std::cell::RefCell::new(m)))
    };
    let lines = [11i128, 22, 33];
    for line in lines {
        it.call(
            "वाक्यविभागॱआज्ञायोजनम्",
            vec![
                family.clone(),
                Value::Int(64),
                octets("अ३२"),
                octets(""),
                Value::Int(1),
                Value::Int(0),
                Value::Int(0),
                Value::Int(line),
            ],
            20_000_000,
        )
        .expect("आज्ञायोजनम् runs")
        .as_int()
        .expect("an index");
    }

    // (1) T1 ITSELF can produce a `कार्यक्रम`. `कार्यक्रमरचना`
    // (vakyavibhaga.t1:595) is `parse()`'s return — the runs the arenas now
    // hold — and it needs no argument.
    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", Vec::new(), 20_000_000)
        .expect("कार्यक्रमरचना runs");
    assert_eq!(
        record_field(&program, "आज्ञासंख्यान").as_int(),
        Some(3),
        "the कार्यक्रम must name the three instructions that were pushed"
    );

    // (2) That `कार्यक्रम` reaches a routine that TAKES one, and the run it
    // names really indexes the arena. This is the whole of what
    // `स्थानविन्यासः` would need from the interpreter.
    assert_eq!(
        it.call("प्रोबॱपङ्क्तियोगः", vec![program.clone()], 20_000_000)
            .expect("पङ्क्तियोगः runs")
            .as_int(),
        Some(lines.iter().sum::<i128>()),
        "walking the instruction run of a कार्यक्रम must reach the three \
         records that were appended"
    );

    // (3) And a `कार्यक्रम` built in RUST reaches it too — which is what
    // lets a test drive the encoder over a program the parser never saw.
    let mut m = std::collections::HashMap::new();
    m.insert("आज्ञारम्भ".to_string(), Value::Int(2));
    m.insert("आज्ञासंख्यान".to_string(), Value::Int(2));
    let hand_built = Value::Record(std::rc::Rc::new(std::cell::RefCell::new(m)));
    assert_eq!(
        it.call("प्रोबॱपङ्क्तियोगः", vec![hand_built], 20_000_000)
            .expect("पङ्क्तियोगः runs on a hand-built record")
            .as_int(),
        Some(lines[1] + lines[2]),
        "Interpreter::invoke checks arity and not type, so a Value::Record \
         with the right fields is a कार्यक्रम as far as a routine can tell"
    );
}

/// **A `गणना` variant IS a value now — this test used to pin the gap.**
///
/// It was written to fail the day the interpreter gained enums, and it did.
/// `read_head`'s `W_ENUM` arm SKIPPED the whole block to its `समाप्तम्` and
/// registered nothing, so a variant name was bound to nothing anywhere, and
/// `स्थानविन्यासः`, `लक्ष्यसङ्केतनम्` and `लक्ष्यवस्तुसङ्केतनम्` could not be
/// written whatever else was ported — each takes `लक्ष्यम् ॱॱ लक्ष्य` and
/// Rust's body branches on `Target::Compressed`.
///
/// KEPT AND INVERTED RATHER THAN DELETED. It now asserts the variant RESOLVES
/// and selects the right arm, so the defect cannot return unnoticed — a
/// deleted test would have left the gap free to reappear in silence, which is
/// how it went unrecorded in the first place.
#[test]
fn an_enum_variant_is_a_value_this_interpreter_can_read() {
    let src = concat!(
        "मण्डलम् प्रोब ॥\n",
        "सार्वजनिक गणना लक्ष्य आरभ्य\n  असङ्कुचितम् ऽ\n  सङ्कुचितम्\nसमाप्तम् ।\n",
        "सार्वजनिक वृत्तिः व्याप्तिमानम् आदाय लक्ष्यम् ॱॱ लक्ष्य ददाति अ६४ आदि\n",
        "यदि लक्ष्यम् समम् सङ्कुचितम् आदि\n    प्रत्यागमनम् २ ।\nइति\n",
        "प्रत्यागमनम् ४ ।\nइति\n",
    );
    let mut it =
        Interpreter::load(&[("probe.t1", src)], &spec_root()).expect("a गणना declaration loads");

    // ZERO-BASED BY POSITION: असङ्कुचितम् is ०, सङ्कुचितम् is १. That matches
    // the Rust discriminants these port — `Target::Uncompressed` is 0 — and is
    // deliberately NOT the one-based convention, which belongs to the ARENAS
    // whose slot ० is never a live entry.
    assert_eq!(
        it.global("असङ्कुचितम्").and_then(Value::as_int),
        Some(0),
        "the first variant must be ०"
    );
    assert_eq!(
        it.global("सङ्कुचितम्").and_then(Value::as_int),
        Some(1),
        "the second variant must be १"
    );

    // And the comparison SELECTS: the routine branches on the variant, so a
    // loader that bound both to the same number would fail here.
    let compressed = it
        .call("व्याप्तिमानम्", vec![Value::Int(1)], 1_000_000)
        .expect("सङ्कुचितम् resolves and the routine runs")
        .as_int();
    let uncompressed = it
        .call("व्याप्तिमानम्", vec![Value::Int(0)], 1_000_000)
        .expect("असङ्कुचितम् resolves and the routine runs")
        .as_int();
    assert_eq!(
        (compressed, uncompressed),
        (Some(2), Some(4)),
        "the variant must choose the arm; if both answer the same, the two \
         variants are bound to one number"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `सङ्केतन ॱ स्थानविन्यासः` — `encode.rs`'s `layout_addresses`, and the FIRST
// of `ENCODER_PROGRAM_ROUTINES` to get a real body.
//
// # Why this one moved and the other seven did not
//
// Two things had to be true at once, and the second is what the record in
// `encode.t1` (b) and in `t1_sources.rs` said was false.
//
// **A `गणना` variant became a VALUE.** `read_head`'s `W_ENUM` arm
// (nirvahana.rs:1185) used to skip the block and register nothing, so
// `असङ्कुचितम्` was "not a name in scope" and no routine could so much as
// name its own `लक्ष्य` argument.
//
// **A `कार्यक्रम` CAN be handed to the interpreter.** Both files say it
// cannot, on the grounds that `सङ्कलनम्` answers a count — which is true of
// `सङ्कलनम्` and of nothing else. `वाक्यविभाग ॱ कार्यक्रमरचना`
// (vakyavibhaga.t1:595) returns a whole `कार्यक्रम`, sealing the runs its
// appenders filled, and `build` below drives exactly that path. So this
// routine is asserted against a program built through वाक्यविभाग's own
// appenders rather than against itself, which is the thing `D-002a` reopened
// over.
//
// # What is NOT evidence here
//
// The other seven stubs all funnel into `encode_collecting`, and nothing
// below touches one. `layout_addresses` is the one of the eight that never
// calls it: it needs an instruction COUNT and the alignment requests, not any
// instruction's operands. Nothing here says the encoder encodes anything.
//
// # The expectations are computed, not transcribed
//
// `expected_layout` is Rust's own algorithm — `next_multiple_of` folded over
// the matching requests, then a four-byte width — written out independently
// in this file, exactly as `expand` above reimplements `render`. There is no
// table to read: this routine reads no `spec/*.tsv`, so the thing to check it
// against is the arithmetic and not a file. Every case also carries the
// literal addresses, so a bug shared by the port and the reimplementation
// still fails.
// ─────────────────────────────────────────────────────────────────────────

/// `encode.t1` and `vakyavibhaga.t1`.
///
/// **Both, and for a stronger reason than `load`'s.** `स्थानविन्यासः` names
/// `वाक्यविभागॱसंरेखकोश` in its body, so with `encode.t1` alone the body does
/// not PARSE and the routine reports itself unrunnable — the same shape
/// `मूल्याङ्कः` has, and the reason both are rows of
/// `NEEDS_A_MODULE_NOT_LOADED` in `t1_execution.rs`.
fn load_layout(encode: &str) -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", encode),
            ("vakyavibhaga.t1", &source("vakyavibhaga.t1")),
            ("ashtaka.t1", &source("ashtaka.t1")),
            // W-363: the whole-program layout asks `कोशॱदत्तपृष्ठाधारः` where
            // `.data` begins.
            ("kosha.t1", &source("kosha.t1")),
        ],
        &spec_root(),
    )
    .expect("the T1 sources load")
}

/// The value of a `लक्ष्य` variant, READ FROM THE INTERPRETER.
///
/// Not `Value::Int(0)`. The whole finding that unblocked this routine is that
/// a variant is a value whose number is its declaration position, and a test
/// that wrote the number down here would agree just as well with a build in
/// which the `W_ENUM` arm had gone back to registering nothing.
///
/// **THE NAME IS UNQUALIFIED, and that is a property of the fix rather than
/// of this test.** A `सार्वजनिक चरः` is registered as `मण्डलॱनाम` —
/// `सङ्केतनॱगन्तृक्षेत्र` — but `Interpreter::load` (nirvahana.rs:545) inserts
/// a variant under its BARE name, so `असङ्कुचितम्` is one name shared by
/// every loaded module and `सङ्केतनॱअसङ्कुचितम्` is not a name at all. That
/// is also why the same line uses `or_insert`: an explicit global of that
/// name wins, because two modules declaring a variant alike would otherwise
/// silently pick one.
fn target(it: &Interpreter, variant: &str) -> Value {
    it.global(variant)
        .unwrap_or_else(|| panic!("{variant} is a name in scope"))
        .clone()
}

/// A `कार्यक्रम` with `instructions` instructions and `aligns` as its
/// `text_aligns`, built through `वाक्यविभाग`'s own appenders and sealed by
/// `कार्यक्रमरचना`.
///
/// The instructions are zero records — `स्थानविन्यासः` reads the COUNT and
/// never an instruction's contents, because under `असङ्कुचितम्` every width
/// is four whatever the instruction is.
fn build(it: &mut Interpreter, instructions: usize, aligns: &[(i128, i128)]) -> Value {
    for n in 0..instructions {
        let line = i128::try_from(n).expect("fits") + 1;
        it.call(
            "वाक्यविभागॱआज्ञायोजनम्",
            vec![
                Value::Int(0),
                Value::Int(0),
                octets(""),
                octets(""),
                Value::Int(0),
                Value::Int(0),
                Value::Int(0),
                Value::Int(line),
            ],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("आज्ञायोजनम् runs: {e:?}"));
    }
    for (at, n) in aligns {
        it.call(
            "वाक्यविभागॱसंरेखयोजनम्",
            vec![Value::Int(*at), Value::Int(*n)],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("संरेखयोजनम् runs: {e:?}"));
    }
    it.call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .unwrap_or_else(|e| panic!("कार्यक्रमरचना runs: {e:?}"))
}

/// `स्थानविन्यासः program target`, as the addresses it answers.
///
/// A run failure PANICS rather than becoming an empty vector, for the reason
/// `value_of` gives: "the routine answered nothing" and "the routine did not
/// run" are different facts, and a stub that returns a string reads as the
/// first if they are collapsed.
fn layout(it: &mut Interpreter, program: Value, tgt: Value) -> Vec<i128> {
    // Fuel raised 50_000_000 -> 5_000_000_000 on 2026-09-04: under
    // `सङ्कुचितम्` every round encodes and decodes every instruction and
    // walks two spec tables per form, which the flat layout never did.
    let v = it
        .call("सङ्केतनॱस्थानविन्यासः", vec![program, tgt], 5_000_000_000)
        .unwrap_or_else(|e| panic!("स्थानविन्यासः runs: {e:?}"));
    match v {
        Value::Arena(a) => a
            .borrow()
            .iter()
            .map(|e| e.as_int().expect("every entry is an address"))
            .collect(),
        other => panic!("स्थानविन्यासः must answer an arena, answered {other:?}"),
    }
}

/// The addresses Rust's `layout_addresses` answers, computed here.
///
/// `at.next_multiple_of(n)` FOLDED over every request naming the instruction
/// — not the last one winning — then a four-byte width, then the trailing
/// address a label past the last instruction borrows.
fn expected_layout(instructions: usize, aligns: &[(i128, i128)]) -> Vec<i128> {
    let mut out = Vec::new();
    let mut pc: i128 = 0;
    for i in 0..instructions {
        let here = i128::try_from(i).expect("fits");
        let mut at = pc;
        for (idx, n) in aligns {
            if *idx == here && at % *n != 0 {
                at += *n - (at % *n);
            }
        }
        pc = at;
        out.push(pc);
        pc += 4;
    }
    out.push(pc);
    out
}

/// The uncompressed layout of a freshly built program.
fn uncompressed(instructions: usize, aligns: &[(i128, i128)]) -> Vec<i128> {
    let mut it = load_layout(&source("encode.t1"));
    let p = build(&mut it, instructions, aligns);
    let t = target(&it, "असङ्कुचितम्");
    layout(&mut it, p, t)
}

#[test]
fn every_instruction_sits_four_octets_after_the_one_before_it() {
    // The base case, and the one that says the fixpoint terminates at all.
    let got = uncompressed(5, &[]);
    assert_eq!(got, expected_layout(5, &[]));
    assert_eq!(
        got,
        vec![0, 4, 8, 12, 16, 20],
        "five 32-bit instructions lie four octets apart, and the sixth entry \
         is the address past the last of them"
    );
}

#[test]
fn the_answer_is_one_longer_than_the_instruction_count() {
    // `addresses.push(pc)` after the loop, and the comment Rust puts on it: a
    // label may sit at the very end of the program, where `at` equals the
    // instruction count and there is no instruction to borrow from. `कोश`
    // reads this entry, so its absence is a label at the wrong address rather
    // than a panic.
    for n in 0..6 {
        let got = uncompressed(n, &[]);
        assert_eq!(
            got.len(),
            n + 1,
            "{n} instructions must lay out to {} addresses",
            n + 1
        );
        assert_eq!(
            *got.last().expect("never empty"),
            i128::try_from(n).expect("fits") * 4
        );
    }
}

#[test]
fn a_program_with_no_instruction_answers_one_address_and_it_is_zero() {
    // The empty case is NOT the empty answer, and an arena could not express
    // it if it were: `भवति ०` on `अङ्कः अन्तः अ३२` yields ONE slot holding
    // `शून्यम्` (nirvahana.rs:674). Rust's empty `Vec` plus its `push(pc)` is
    // the same single entry, so the two agree — but only because the routine
    // WRITES that slot. Left unwritten it would answer `शून्यम्`, which is
    // not an address, and `as_int` in `layout` panics rather than quietly
    // reading it as ०.
    assert_eq!(uncompressed(0, &[]), vec![0]);
}

#[test]
fn an_alignment_request_pads_the_instruction_it_sits_before() {
    // `W-071`, and the whole reason `संरेख` is a RECORDED REQUEST the parser
    // does not resolve — vakyavibhaga.t1:246 names this routine as what
    // satisfies it.
    let aligns = [(1, 8)];
    let got = uncompressed(3, &aligns);
    assert_eq!(got, expected_layout(3, &aligns));
    assert_eq!(got, vec![0, 8, 12, 16]);

    // The same program WITHOUT the request, so the difference is attributable
    // to the request and not to the layout being 0 8 12 anyway.
    assert_eq!(uncompressed(3, &[]), vec![0, 4, 8, 12]);
}

#[test]
fn an_address_already_on_the_boundary_is_not_pushed_a_whole_stride() {
    // `next_multiple_of` of a number that is already a multiple is that
    // number. Counting up from ० — which is what `संरेखान्तरम्` does, T1
    // having no remainder operator — is the easy way to get this wrong by a
    // whole stride, so it is asserted rather than assumed.
    let aligns = [(1, 4)];
    assert_eq!(uncompressed(3, &aligns), expected_layout(3, &aligns));
    assert_eq!(uncompressed(3, &aligns), vec![0, 4, 8, 12]);
}

#[test]
fn two_requests_on_one_instruction_compose_rather_than_the_last_one_winning() {
    // Rust folds `at = at.next_multiple_of(n)` over every matching request,
    // so two requests before the same instruction reach 8 and not 4. A port
    // that stopped at the first match, or that recomputed the padding from
    // `pc` each time, would answer 4 — which is why the strides are ordered
    // small-then-large here.
    let aligns = [(1, 4), (1, 8)];
    let got = uncompressed(3, &aligns);
    assert_eq!(got, expected_layout(3, &aligns));
    assert_eq!(got, vec![0, 8, 12, 16]);

    // The single request the composed pair must not be confused with.
    assert_eq!(uncompressed(3, &[(1, 4)]), vec![0, 4, 8, 12]);
}

#[test]
fn a_request_naming_the_instruction_past_the_end_pads_nothing() {
    // `संरेखयोजनम् आज्ञासूचकाङ्क …` records the count of instructions pushed
    // SO FAR, so a `॥ संरेखः ॥` written after the last instruction names an
    // index no instruction has. Rust's loop never matches it, and the
    // trailing address is therefore NOT padded.
    let aligns = [(3, 16)];
    let got = uncompressed(3, &aligns);
    assert_eq!(got, expected_layout(3, &aligns));
    assert_eq!(got, vec![0, 4, 8, 12]);
}

#[test]
fn the_layout_is_a_fixpoint_and_reports_the_same_answer_twice() {
    // The relaxation loop `B-007` built runs up to `शिथिलनावृत्तिसीमा`
    // rounds and stops when a round agrees with the one before it. Under
    // `असङ्कुचितम्` nothing shrinks, so it settles on round १.
    let aligns = [(1, 8), (2, 16)];
    let first = uncompressed(4, &aligns);
    let second = uncompressed(4, &aligns);
    assert_eq!(first, second);
    assert_eq!(first, expected_layout(4, &aligns));
    assert_eq!(first, vec![0, 8, 16, 20, 24]);
}

#[test]
fn the_two_target_variants_are_numbered_by_declaration_position() {
    // The finding that unblocked this routine, asserted where it is USED and
    // not only where it was fixed: `असङ्कुचितम्` is declared first and
    // `सङ्कुचितम्` second, and the numbers are ० and १ — zero-based, matching
    // `Target::Uncompressed` and `Target::Compressed`, whose `#[default]`
    // sits on the first.
    let it = load_layout(&source("encode.t1"));
    assert_eq!(target(&it, "असङ्कुचितम्").as_int(), Some(0));
    assert_eq!(target(&it, "सङ्कुचितम्").as_int(), Some(1));
}

#[test]
fn the_compressed_target_answers_a_shorter_layout() {
    // ── THIS WAS `the_compressed_target_still_answers_the_uncompressed_
    // layout`, WHICH PINNED A GAP AND WAS WRITTEN TO FAIL THE DAY `सङ्कोचः`
    // LANDED. It landed on 2026-09-04 and this is the test it asked for.
    //
    // The program is `crates/sadhana/tests/compressed.rs`'s: six
    // instructions that all compress, with a FORWARD branch at instruction ३
    // whose target is the label after instruction ४. On round ० the label is
    // unknown, so the branch refuses and is laid out wide; on round १ it is
    // known and the branch shrinks, which moves everything after it; round २
    // agrees with round १ and the fixpoint settles. That is the first time
    // this loop has iterated since `B-007` built it, and the addresses say
    // so: `0 2 4 6 8 10 12`, not `0 2 4 8 10 12 14`.
    //
    // THE ORACLE IS THE ASSEMBLER, AND THIS TEST IS WHERE RUST'S LAYOUT WAS
    // FOUND TO DISAGREE WITH IT. `riscv64-elf-as -march=rv64gc` lays the
    // program out at `0 2 4 6 8 10` (compressed.rs:315-317), and Rust's
    // `encode_program_for(.., Compressed)` emits exactly those twelve octets.
    // When this test was written, Rust's `layout_addresses(.., Compressed)`
    // answered `0 2 4 6 10 12 14`: it ran `symbols.clear()` at the TOP of
    // every round, before the width pass, so the table built at the foot of
    // round ० was empty again when round १ asked a width, and a forward
    // branch was never seen to shrink. The two Rust loops disagreed about
    // where instruction ४ is — the E23 class of defect, in the routine
    // `kosha` hands debug addresses from — and the T1 port, which carries
    // the previous round's table forward, was the thing that said so. That
    // pin was filed as `W-241` and the Rust loop now clears AFTER its pass;
    // the assertion below is the one the pin asked for, and
    // `crates/sadhana/tests/paradigm_t0.rs` holds the same agreement over
    // every spec program.
    let src = COMPRESSED_PROGRAM;
    let rust = sadhana::parse::assemble_program(src).expect("parses");
    let want: Vec<i128> = vec![0, 2, 4, 6, 8, 10, 12];
    let emitted = sadhana::encode::encode_program_for(&rust, sadhana::encode::Target::Compressed)
        .expect("the Rust encoder encodes it");
    assert_eq!(
        emitted.len(),
        12,
        "twelve octets, so the last address is 12"
    );
    let rust_layout: Vec<i128> =
        sadhana::encode::layout_addresses(&rust, sadhana::encode::Target::Compressed)
            .into_iter()
            .map(i128::from)
            .collect();
    assert_eq!(
        rust_layout, want,
        "Rust's layout_addresses disagrees with its own encode_program_for under \
         Compressed — `0 2 4 6 10 …` here means the layout's symbol table was \
         cleared before the width pass again (W-241)"
    );

    let mut it = load_encoder(&source("encode.t1"));
    let program = mirror_program(&mut it, src);
    let compressed = target(&it, "सङ्कुचितम्");
    let got = layout(&mut it, program.clone(), compressed);
    assert_eq!(
        got, want,
        "under सङ्कुचितम् the T1 layout must be Rust's; `0 2 4 8 …` here means \
         the branch never shrank, so the fixpoint did not iterate"
    );

    // And the SAME program at the other target is twice the size, which is
    // what the choice is for — and what `लक्ष्यम्` being read at all means.
    let wide = target(&it, "असङ्कुचितम्");
    assert_eq!(
        layout(&mut it, program, wide),
        vec![0, 4, 8, 12, 16, 20, 24]
    );
}

// ── mutations ────────────────────────────────────────────────────────────

#[test]
fn shortening_the_instruction_width_moves_every_later_address() {
    // The `_ => 4` of `width_of_for`, mutated to २ — what the compressed arm
    // answers. Every instruction after the first moves, so a test that
    // checked only the FIRST address, or only the length, would pass here.
    //
    // RETARGETED 2026-09-04: the line was `क्रमसूचकः भवति क्रमसूचकः योगः ४ ।`
    // in `विन्यासावृत्तिः` until `width_of_for` became a routine of its own
    // (`आज्ञाविस्तारः`); the constant now lives at that routine's foot.
    let mutated = mutate(&source("encode.t1"), "प्रत्यागमनम् ४ ।", "प्रत्यागमनम् २ ।");
    let mut it = load_layout(&mutated);
    let p = build(&mut it, 5, &[]);
    let t = target(&it, "असङ्कुचितम्");
    assert_eq!(
        layout(&mut it, p, t),
        vec![0, 2, 4, 6, 8, 10],
        "with the width halved the layout must halve; if it still reads \
         0 4 8 … the width is coming from somewhere else"
    );
}

#[test]
fn dropping_the_alignment_fold_leaves_every_request_unsatisfied() {
    // The one line that applies the padding. Without it `स्थितम्` never
    // advances, so `संरेखान्तरम्` is computed and thrown away and the program
    // lays out as though `W-071` had never been recorded.
    let mutated = mutate(
        &source("encode.t1"),
        "स्थितम् भवति स्थितम् योगः पूरणम् ।",
        "स्थितम् भवति स्थितम् ।",
    );
    let mut it = load_layout(&mutated);
    let p = build(&mut it, 3, &[(1, 8)]);
    let t = target(&it, "असङ्कुचितम्");
    assert_eq!(
        layout(&mut it, p, t),
        vec![0, 4, 8, 12],
        "with the fold dropped the alignment request must do nothing"
    );
}

#[test]
fn matching_a_request_against_the_wrong_instruction_misplaces_the_padding() {
    // `संरेख ॱ आज्ञाक्रम` is the ZERO-BASED index of the instruction the
    // padding sits before, and क्रमः is counted the same way — the ±१ the
    // arena convention makes tempting is the defect this catches. Comparing
    // against `क्रमः योगः १` makes the request at index 1 pad index 0, which
    // sits at ० and is already aligned, so nothing moves.
    let mutated = mutate(
        &source("encode.t1"),
        "यदि संरेखः ॱ आज्ञाक्रम समम् क्रमः आदि",
        "यदि संरेखः ॱ आज्ञाक्रम समम् आरभ्य क्रमः योगः १ समाप्तम् आदि",
    );
    let mut it = load_layout(&mutated);
    let p = build(&mut it, 3, &[(1, 8)]);
    let t = target(&it, "असङ्कुचितम्");
    assert_eq!(
        layout(&mut it, p, t),
        vec![0, 4, 8, 12],
        "off by one, the request no longer reaches the instruction that asked"
    );
}

#[test]
fn the_trailing_address_is_the_program_length_and_not_a_filler() {
    // The entry a label past the last instruction borrows. The slot is easy
    // to allocate and get WRONG — writing ० there keeps the arena the right
    // length and every real instruction's address intact, so only an
    // assertion on the LAST entry catches it. `कोश` gives a trailing label
    // this number, so a ० here is a label at the start of the program.
    let mutated = mutate(
        &source("encode.t1"),
        "स्थानानि अङ्कः संख्यानम् अन्तः भवति क्रमसूचकः ।",
        "स्थानानि अङ्कः संख्यानम् अन्तः भवति ० ।",
    );
    let mut it = load_layout(&mutated);
    let p = build(&mut it, 4, &[]);
    let t = target(&it, "असङ्कुचितम्");
    assert_eq!(
        layout(&mut it, p, t),
        vec![0, 4, 8, 12, 0],
        "with the trailing write zeroed the four instructions keep their \
         addresses and only the borrowed one is wrong"
    );
    // and unmutated it is the length of the program.
    assert_eq!(uncompressed(4, &[]), vec![0, 4, 8, 12, 16]);
}

// ─────────────────────────────────────────────────────────────────────────
// `सङ्केतन ॱ स्थानसङ्केतनम्` — `encode.rs`'s `encode_collecting`, and the
// KEYSTONE the other program routines bottom out in.
//
// # What the instruction under test is, and where it comes from
//
// **Every instruction here is parsed by `sadhana::parse`, not written by
// hand into the arenas.** `mirror` below takes one line of Sassembly, hands
// it to the Rust parser, and pushes exactly the `Instruction` that came back
// through `वाक्यविभाग`'s own appenders — `कारकपदयोजनम्` for each operand,
// `आज्ञायोजनम्` for the instruction — then reads the record back OUT of
// `वाक्यविभागआज्ञाकोश`. So the T1 routine is asked about the same instruction the Rust
// encoder is asked about, and neither this file nor `encode.t1` gets to
// decide what the operands were.
//
// # The expectations are the SPEC TABLE, read here
//
// `slots_of` parses field ६ of `spec/encodings-riscv64.tsv` — the
// `kind:mask:valuebit>encodingbit;…` runs — and `place` walks that map. So
// the expected word is `pattern | place(slot, value)` computed from the file,
// with no second copy of the table and no reuse of `encode.t1`'s arithmetic.
// What the tests assert BEYOND the file is the one thing a table cannot say:
// WHICH operand goes to which slot. That is the kāraka rule, it is stated in
// each case's own comment, and it is the claim under test.
//
// A second, independent check rides along: `sadhana::encode::encode_at` is
// public, so every case also asserts the T1 answer against the Rust encoder's
// answer for the same parsed instruction. Neither check alone would be
// enough — the table check cannot see a rôle assignment shared by both
// implementations, and the differential cannot see the two agreeing on a
// misreading of the file.
// ─────────────────────────────────────────────────────────────────────────

use sadhana::lex::Karaka;
use sadhana::parse::Width;

/// `encode.t1` with everything it now reaches.
///
/// **Four modules, and each is named by a call in the body under test.**
/// `वाक्यविभाग` holds `आज्ञा`, `कारकपद` and their arenas; `विश्लेषण` holds
/// `पङ्क्तिसङ्केतः`, which `कुलसङ्केतः` spends the file's own
/// `आयातः विश्लेषण ।` on; `अक्षरकोश` is `मूल्याङ्कः`'s numeral reader.
fn load_encoder(encode: &str) -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", encode),
            ("vakyavibhaga.t1", &source("vakyavibhaga.t1")),
            ("vishlesana.t1", &source("vishlesana.t1")),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
            ("ashtaka.t1", &source("ashtaka.t1")),
            // W-363: the whole-program layout asks `कोशॱदत्तपृष्ठाधारः`.
            ("kosha.t1", &source("kosha.t1")),
        ],
        &spec_root(),
    )
    .expect("encode.t1 and the modules it reaches load together")
}

/// One operand slot of one encoding, read out of field ६ of the spec table.
struct SpecSlot {
    map: Vec<(u32, u32)>,
}

/// Field ६ of a row, parsed. `(none)` is no slots at all — `ecall`'s shape.
fn slots_of(field: &str) -> Vec<SpecSlot> {
    if field == "(none)" {
        return Vec::new();
    }
    field
        .split('|')
        .map(|s| {
            let mut parts = s.splitn(3, ':');
            let _kind = parts.next().expect("a kind");
            let _mask = parts.next().expect("a mask");
            let map = parts
                .next()
                .map(|m| {
                    m.split(';')
                        .filter(|x| !x.is_empty())
                        .map(|x| {
                            let (a, b) = x.split_once('>').expect("`valuebit>encodingbit`");
                            (a.parse().expect("a bit"), b.parse().expect("a bit"))
                        })
                        .collect()
                })
                .unwrap_or_default();
            SpecSlot { map }
        })
        .collect()
}

/// `Slot::place` — walk the derived map, bit by bit. No shift by a remembered
/// offset and no special case for the scattered B-type or J-type, which is
/// the whole point of the map being derived (`B-055`).
fn place(slot: &SpecSlot, value: u64) -> u32 {
    let mut w = 0u32;
    for (from, to) in &slot.map {
        if (value >> from) & 1 == 1 {
            w |= 1u32 << to;
        }
    }
    w
}

/// The pattern and slots of one mnemonic, from `spec/encodings-riscv64.tsv`.
fn spec_row(insn: &str) -> (u32, Vec<SpecSlot>) {
    let text = encodings_text();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("insn\t") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 7 || f[0] != insn {
            continue;
        }
        let pattern = u32::from_str_radix(f[3].trim_start_matches("0x"), 16).expect("a pattern");
        return (pattern, slots_of(f[6]));
    }
    panic!("{insn} is not a row of spec/encodings-riscv64.tsv");
}

fn karaka_number(k: Karaka) -> i128 {
    match k {
        Karaka::Destination => 1,
        Karaka::Source => 2,
        Karaka::SourceAddress => 3,
        Karaka::DestAddress => 4,
        Karaka::Locus => 5,
    }
}

fn width_number(w: Width) -> i128 {
    match w {
        Width::W8 => 1,
        Width::W16 => 2,
        Width::W32 => 3,
        Width::W64 => 4,
    }
}

fn record(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = std::collections::HashMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    Value::Record(std::rc::Rc::new(std::cell::RefCell::new(m)))
}

fn arena_at(it: &Interpreter, name: &str, i: usize) -> Value {
    match it.global(name).expect("the arena is a name in scope") {
        Value::Arena(a) => a.borrow()[i].clone(),
        other => panic!("{other:?} is not an arena"),
    }
}

/// Parse one line of Sassembly with the Rust parser and push the resulting
/// `Instruction` into `वाक्यविभाग`'s arenas through its own appenders.
fn mirror(it: &mut Interpreter, src: &str) -> (Value, sadhana::parse::Instruction) {
    let program =
        sadhana::parse::assemble_program(src).unwrap_or_else(|e| panic!("{src:?} parses: {e:?}"));
    assert_eq!(
        program.instructions.len(),
        1,
        "{src:?} must be exactly one instruction"
    );
    let inst = program.instructions[0].clone();

    let mut first = 0i128;
    for (n, op) in inst.operands.iter().enumerate() {
        let at = it
            .call(
                "वाक्यविभागॱकारकपदयोजनम्",
                vec![
                    Value::Int(karaka_number(op.karaka)),
                    octets(&op.base),
                    Value::Bool(op.is_numeral),
                ],
                20_000_000,
            )
            .expect("कारकपदयोजनम् runs")
            .as_int()
            .expect("an index");
        if n == 0 {
            first = at;
        }
    }

    let family = record(vec![
        ("कूट", octets(&inst.family.key)),
        ("नाम", octets(&inst.family.name)),
        ("व्याख्या", octets(&inst.family.gloss)),
        ("विस्तारः", octets(&inst.family.ext)),
        ("आवरणारम्भ", Value::Int(0)),
        ("आवरणसंख्यान", Value::Int(0)),
    ]);
    let at = it
        .call(
            "वाक्यविभागॱआज्ञायोजनम्",
            vec![
                family,
                Value::Int(width_number(inst.width)),
                octets(inst.types.first().map_or("", String::as_str)),
                octets(inst.types.get(1).map_or("", String::as_str)),
                Value::Int(i128::try_from(inst.types.len()).expect("fits")),
                Value::Int(first),
                Value::Int(i128::try_from(inst.operands.len()).expect("fits")),
                Value::Int(i128::try_from(inst.line).expect("fits")),
            ],
            20_000_000,
        )
        .expect("आज्ञायोजनम् runs")
        .as_int()
        .expect("an index");

    // `Interpreter::global` is keyed by the BARE name — `global_key`
    // (nirvahana.rs:835) strips a module prefix it RECOGNISES, i.e. one joined by
    // `ॱ`. Since the 2026-09-11 rename the global is `वाक्यविभागआज्ञाकोश` with the
    // module name as part of the IDENTIFIER and no separator, so there is nothing
    // to strip and the whole name is the key. It is not `आज्ञाकोश` any more:
    // that name now belongs to `मध्यरूप` alone, and this load set has no `ir.t1`.
    let value = arena_at(it, "वाक्यविभागआज्ञाकोश", usize::try_from(at).expect("fits"));
    (value, inst)
}

/// A `चिह्नस्थान` arena — zero-based, as that record's own margin says.
fn symbols(pairs: &[(&str, i128)]) -> Value {
    let rows: Vec<Value> = if pairs.is_empty() {
        // An EMPTY map — Rust's `&BTreeMap::new()` — is an empty arena, which
        // is what a fresh `भवति ०` run is since `W-355`. Until then an arena
        // was born holding one शून्यम् and that was this language's empty map.
        vec![]
    } else {
        pairs
            .iter()
            .map(|(n, at)| record(vec![("नाम", octets(n)), ("स्थान", Value::Int(*at))]))
            .collect()
    };
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(rows)))
}

/// `स्थानसङ्केतनम् inst pc symbols`, as `Option<u32>`; `None` where it refused.
///
/// A RUN FAILURE PANICS rather than becoming `None`, for the reason
/// `value_of` gives at the head of this file: "the routine says no" and "the
/// routine did not run" are different facts.
fn encode_at_t1(it: &mut Interpreter, inst: Value, pc: i128, syms: Value) -> Option<u32> {
    let v = it
        .call(
            "सङ्केतनॱस्थानसङ्केतनम्",
            vec![inst, Value::Int(pc), syms],
            600_000_000,
        )
        .unwrap_or_else(|e| panic!("स्थानसङ्केतनम् runs: {e:?}"));
    v.as_int()
        .map(|n| u32::try_from(n).expect("an instruction word"))
}

/// The code of the diagnostic the last refusal left behind.
fn last_code(it: &Interpreter) -> String {
    let err = it
        .global("अन्तिमसङ्केतनदोषः")
        .expect("अन्तिमसङ्केतनदोषः is a name in scope");
    text_of(&record_field(err, "सङ्केताङ्क"))
}

/// One source line, encoded twice: by the T1 routine and by the Rust encoder.
fn encoded(src: &str) -> (u32, u32) {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, inst) = mirror(&mut it, src);
    let t1 = encode_at_t1(&mut it, value, 0, symbols(&[]))
        .unwrap_or_else(|| panic!("{src:?} encodes; it refused with {}", last_code(&it)));
    let rust = sadhana::encode::encode(&inst).unwrap_or_else(|e| panic!("{src:?} encodes: {e:?}"));
    (t1, rust)
}

/// **The fixed bits, and nothing else.** Every operand is `शून्यः` — x0, the
/// hardwired zero, whose number is ० — so every register field is zero and
/// the whole word is the encoding's own pattern, read out of field ३ of the
/// spec table by this file. A port that started the word from ० rather than
/// from the pattern answers ० here; one that mislaid the one-based correction
/// on the register table answers the pattern with ones OR-ed into it.
#[test]
fn an_instruction_of_hardwired_zeros_is_the_pattern_the_spec_table_carries() {
    for (src, insn) in [
        ("योगः शून्यःम् शून्यःन शून्यःन ।", "add"),
        ("वियोगः शून्यःम् शून्यःन शून्यःन ।", "sub"),
        ("वैषम्यम् शून्यःम् शून्यःन शून्यःन ।", "xor"),
        ("योगः शून्यःम् शून्यःन ०न ।", "addi"),
        ("निधानम् शून्यःन शून्यःय् ०न ।", "sd"),
    ] {
        let (t1, rust) = encoded(src);
        let (pattern, _) = spec_row(insn);
        assert_eq!(
            t1, pattern,
            "{src:?} must be `{insn}`'s pattern {pattern:#010x} from the spec \
             table, with every field zero"
        );
        assert_eq!(t1, rust, "{src:?}: the T1 port and the Rust encoder differ");
    }
}

/// **Which slot each kāraka claimed, field by field against the map the spec
/// table carries.**
///
/// The expected word is `pattern | place(slot, n)` with `place` walking field
/// ६'s own `valuebit>encodingbit` runs. What is asserted beyond the file is
/// the kāraka rule, written out per case: for `add` the कर्म operand takes
/// the first slot and the two करण operands the second and third in written
/// order; for `sd` — which has no destination — the करण VALUE takes the first
/// slot and the सम्प्रदान ADDRESS the third, which is the separation `B-044`
/// exists for.
#[test]
fn each_karaka_lands_in_the_field_the_spec_table_maps_it_to() {
    // x5, x2, x3 — three numbers sharing no bits, so a field taking another's
    // value shows up rather than agreeing by accident.
    let (t1, rust) = encoded("योगः क्षणिक०म् स्तूपसूचकःन विश्वसूचकःन ।");
    let (pattern, slots) = spec_row("add");
    assert_eq!(
        t1,
        pattern | place(&slots[0], 5) | place(&slots[1], 2) | place(&slots[2], 3),
        "add: कर्म to slot ०, then the करण operands in written order"
    );
    assert_eq!(t1, rust);

    // A store has no rd. Its slots are (value, offset, address) in written
    // order, so the करण value takes slot ० and the सम्प्रदान base slot २ —
    // and a port that let both fall through to one field would OR them
    // together into a single register, which is exactly `B-044`.
    let (t1, rust) = encoded("निधानम् क्षणिक०न स्तूपसूचकःय् ०न ।");
    let (pattern, slots) = spec_row("sd");
    assert_eq!(
        t1,
        pattern | place(&slots[0], 5) | place(&slots[2], 2),
        "sd: करण is the value, सम्प्रदान is the address, and the offset is ०"
    );
    assert_eq!(t1, rust);

    // An immediate, in a field the map scatters over twelve bits.
    let (t1, rust) = encoded("योगः क्षणिक०म् स्तूपसूचकःन ११११न ।");
    let (pattern, slots) = spec_row("addi");
    assert_eq!(
        t1,
        pattern | place(&slots[0], 5) | place(&slots[1], 2) | place(&slots[2], 1111)
    );
    assert_eq!(t1, rust);
}

/// **A branch target is a DISTANCE, and the field that holds it is scattered.**
///
/// `beq`'s displacement map is the one field in the table no reader could
/// guess: bit १ of the value goes to bit ८ of the word and bit ११ goes to bit
/// ७. Placing it through the map is the whole of `B-055`, and a port that
/// shifted by a remembered offset would answer something plausible and wrong.
#[test]
fn a_branch_target_is_the_distance_to_the_label_placed_through_the_derived_map() {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, inst) = mirror(&mut it, "समलङ्घनम् क्षणिक०न स्तूपसूचकःत् लक्ष्यय् ।");
    let t1 = encode_at_t1(&mut it, value, 0x10, symbols(&[("लक्ष्य", 0x40)])).expect("it encodes");

    let (pattern, slots) = spec_row("beq");
    // 0x40 - 0x10 = 0x30 forward. The subject compared takes rs1 and the
    // standard of comparison — अपादान, the ablative — takes rs2 (ADR-0008).
    assert_eq!(
        t1,
        pattern | place(&slots[0], 5) | place(&slots[1], 2) | place(&slots[2], 0x30),
        "the displacement is the distance from the instruction to the label"
    );

    let mut map = std::collections::BTreeMap::new();
    map.insert("लक्ष्य".to_string(), 0x40u32);
    assert_eq!(
        t1,
        sadhana::encode::encode_at(&inst, 0x10, &map).expect("the Rust encoder encodes"),
        "the T1 port and the Rust encoder differ on a branch"
    );
}

/// **`ॱउपरि` and `ॱअधः` split one address across two instructions**, and the
/// halves are measured from DIFFERENT addresses: the high half from this
/// instruction, the low half from the one before it — the `auipc` whose
/// register it is completing. GNU spells the same thing `%pcrel_lo(1b)`.
#[test]
fn the_two_halves_of_an_address_are_measured_from_different_instructions() {
    let mut it = load_encoder(&source("encode.t1"));
    let (upper, upper_inst) = mirror(&mut it, "स्थानसापेक्षयोगः क्षणिक०म् दत्तम्ॱउपरिन ।");
    let (lower, lower_inst) = mirror(&mut it, "योगः क्षणिक०म् क्षणिक०न दत्तम्ॱअधःन ।");
    let syms = symbols(&[("दत्तम्", 0x9004)]);

    let hi = encode_at_t1(&mut it, upper, 0x1000, syms.clone()).expect("the upper half encodes");
    let lo = encode_at_t1(&mut it, lower, 0x1004, syms).expect("the lower half encodes");

    // 0x9004 - 0x1000 = 0x8004 for the upper half, measured from the auipc
    // itself; hi = (0x8004 + 0x800) >> 12 = 8. The lower half measures from
    // the PRECEDING instruction — 0x1004 - 4 — so it sees the same 0x8004 and
    // takes 0x8004 - (8 << 12) = 4.
    let (auipc_pattern, auipc_slots) = spec_row("auipc");
    assert_eq!(
        hi,
        auipc_pattern | place(&auipc_slots[0], 5) | place(&auipc_slots[1], 8)
    );
    let (addi_pattern, addi_slots) = spec_row("addi");
    assert_eq!(
        lo,
        addi_pattern
            | place(&addi_slots[0], 5)
            | place(&addi_slots[1], 5)
            | place(&addi_slots[2], 4)
    );

    let mut map = std::collections::BTreeMap::new();
    map.insert("दत्तम्".to_string(), 0x9004u32);
    assert_eq!(
        hi,
        sadhana::encode::encode_at(&upper_inst, 0x1000, &map).expect("encodes")
    );
    assert_eq!(
        lo,
        sadhana::encode::encode_at(&lower_inst, 0x1004, &map).expect("encodes")
    );
}

/// **A conversion is chosen by the PAIR of types and by nothing else.** These
/// two write the same register from the same register: same shape, same
/// destination class, same result width. Only what they READ differs
/// (`B-075`), which is the whole reason the second suffix exists.
#[test]
fn the_source_type_is_what_tells_two_conversions_apart_in_the_port() {
    let (single, rust_single) = encoded("प्लवरूपान्तरम्ॱअ३२ॱप३२ अर्थ०म् प्लव१न ।");
    let (double, rust_double) = encoded("प्लवरूपान्तरम्ॱअ३२ॱप६४ अर्थ०म् प्लव१न ।");
    assert_ne!(
        single, double,
        "reading a float32 and a float64 are different instructions"
    );
    assert_eq!(single, rust_single);
    assert_eq!(double, rust_double);

    // `अर्थ०` is a0 — x10 — and `प्लव१` is f1. The destination is an INTEGER
    // register even though the encoding reads a float, which is why a
    // conversion is exempt from the class test that would otherwise refuse
    // `ॱअ३२` on a row with an `freg` slot.
    let (pattern, slots) = spec_row("fcvt.w.s");
    assert_eq!(single, pattern | place(&slots[0], 10) | place(&slots[1], 1));
}

/// **`V-003` — every float case the ORACLE carries, through the `.t1` encoder.**
///
/// # Why this is the row's acceptance and not another spot check
///
/// `V-003` asks for R-type, I-type `FLW`/`FLD`, S-type `FSW`/`FSD` and R4-type
/// `FMADD` *"byte-identical to the oracle PER INSTRUCTION FORM rather than in
/// aggregate"*. Two spot checks above this one already drive floats through the
/// port — `प्लवरूपान्तरम्` and `प्लवयोगः` — and neither says anything about a
/// float LOAD, a float STORE, or a four-register fused multiply.
///
/// The oracle already covers all of them and the corpus was already on disk.
/// `spec/conformance-t0.tsv` is generated by `tools/gen-conformance.py`, whose
/// authority is `riscv64-elf-as` and which returns 1 WITHOUT WRITING when the
/// assembler refuses a case — so every row in it is one GNU binutils agreed to.
/// **432 of those rows are float**, covering all 62 rows of
/// `spec/encodings-riscv64.tsv`: `flw`/`fld`/`fsw`/`fsd` at 28 cases each and
/// the whole `fmadd`/`fmsub`/`fnmadd`/`fnmsub` group at 6 per width. Writing
/// Sassembly source for those forms by hand would have been guessing at karaka
/// placement; the table states it — `प्लवगुणयोगःॱप३२ प्लव०म् प्लव०न प्लव०न
/// प्लव०न ।` is the R4 shape, read off the corpus and not invented here.
///
/// `tests/conformance.rs` drives the same corpus through the RUST encoder. This
/// drives the float part through the `.t1` one, which is the half `V-003` is
/// about: a port agreeing with its own twin proves nothing an oracle would.
///
/// # Per form, and the failure names the form
///
/// Disagreements are grouped by the oracle's own mnemonic, and the pass prints a
/// per-family census. A family that stops being exercised is a floor failure
/// here rather than a silent narrowing, because the count comes from the data:
/// if a future table drops `fsd`, this test says so instead of passing smaller.
#[test]
fn every_float_case_the_oracle_carries_encodes_to_the_word_the_oracle_gives() {
    let path = repo_root().join("spec/conformance-t0.tsv");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));

    // `(sassembly, word, riscv)` for the float rows. The mnemonic is the first
    // word of the riscv column; `fence`/`fence.i` begin with `f` and are
    // MISC-MEM, so they are excluded by name the way `fp_oracle.rs` does it.
    let mut cases: Vec<(String, u32, String)> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("sassembly\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let (Some(src), Some(hex), Some(riscv)) = (f.first(), f.get(1), f.get(2)) else {
            continue;
        };
        let mnemonic = riscv.split_whitespace().next().unwrap_or("");
        if !mnemonic.starts_with('f') || mnemonic.starts_with("fence") {
            continue;
        }
        let Ok(word) = u32::from_str_radix(hex.trim().trim_start_matches("0x"), 16) else {
            continue;
        };
        cases.push((src.to_string(), word, riscv.to_string()));
    }

    // ONE interpreter for all of them. `encoded()` above builds a fresh one per
    // call, which is right for a handful of spot checks and is 432 loads of
    // `encode.t1` here.
    let mut it = load_encoder(&source("encode.t1"));
    let syms = symbols(&[]);

    let mut disagreed: Vec<String> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    let mut per_family: BTreeMap<String, usize> = BTreeMap::new();
    for (src, want, riscv) in &cases {
        let mnemonic = riscv.split_whitespace().next().unwrap_or("").to_string();
        *per_family.entry(mnemonic).or_default() += 1;
        let (value, _inst) = mirror(&mut it, src);
        match encode_at_t1(&mut it, value, 0, syms.clone()) {
            None => refused.push(format!("{riscv}: {src:?} — {}", last_code(&it))),
            Some(got) if got != *want => disagreed.push(format!(
                "{riscv}: {src:?} — port {got:#010x}, oracle {want:#010x}"
            )),
            Some(_) => {}
        }
    }

    println!("METRIC t1_float_oracle_cases {}", cases.len());
    println!("METRIC t1_float_oracle_families {}", per_family.len());
    println!("METRIC t1_float_oracle_refused {}", refused.len());
    println!("METRIC t1_float_oracle_disagreements {}", disagreed.len());
    for (m, n) in &per_family {
        println!("  {m:<12} {n}");
    }

    // VACUITY FLOORS, under the measured values on purpose. A floor set AT the
    // measurement reds when a case is legitimately removed, which teaches a
    // reader to lower floors reflexively. Measured 2026-09-29: 432 cases over 62
    // mnemonics.
    assert!(
        cases.len() >= 400 && per_family.len() >= 60,
        "the float corpus shrank to {} case(s) over {} mnemonic(s) — this test is \
         about what the oracle carries, so a smaller corpus is the finding",
        cases.len(),
        per_family.len()
    );
    // THE FORMS THE ROW NAMES, each asserted present by NAME rather than left to
    // the totals. R-type and the conversions were already exercised above; these
    // four are what `V-003` says is owed, and an aggregate count cannot tell a
    // reader that `fsd` is in the run.
    for m in [
        "flw", "fld", "fsw", "fsd", "fmadd.d", "fmsub.d", "fnmadd.d", "fnmsub.d",
    ] {
        assert!(
            per_family.contains_key(m),
            "`{m}` is not in the corpus this test read, so the form `V-003` names \
             is NOT covered however green the totals look"
        );
    }

    assert!(
        refused.is_empty(),
        "the `.t1` encoder REFUSED {} float case(s) GNU binutils accepted:\n  {}",
        refused.len(),
        refused.join("\n  ")
    );
    assert!(
        disagreed.is_empty(),
        "the `.t1` encoder disagrees with the oracle on {} of {} float case(s). \
         `spec/conformance-t0.tsv` is what `riscv64-elf-as` emitted, so the port \
         is what moves:\n  {}",
        disagreed.len(),
        cases.len(),
        disagreed.join("\n  ")
    );
}

/// **`V-008` part 2 — every VECTOR case the oracle carries, through the `.t1`
/// encoder, per form.** The seven forms the vector lowering emits — `vsetvli`,
/// `vle64.v`, `vse64.v` and `vfadd`/`vfsub`/`vfmul`/`vfdiv.vv` — are rows of
/// `spec/conformance-t0.tsv` that `tools/gen-conformance.py` asked
/// `riscv64-elf-as -march=rv64gcv` for; `tests/conformance.rs` drives the same
/// rows through the Rust encoder. The `.vv` rows pin the operand ORDER as well
/// as the opcode: the first source written is GNU's `vs2`.
#[test]
fn every_vector_case_the_oracle_carries_encodes_to_the_word_the_oracle_gives() {
    let path = repo_root().join("spec/conformance-t0.tsv");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    let mut cases: Vec<(String, u32, String)> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("sassembly\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let (Some(src), Some(hex), Some(riscv)) = (f.first(), f.get(1), f.get(2)) else {
            continue;
        };
        if !riscv.starts_with('v') {
            continue;
        }
        let Ok(word) = u32::from_str_radix(hex.trim().trim_start_matches("0x"), 16) else {
            continue;
        };
        cases.push((src.to_string(), word, riscv.to_string()));
    }

    let mut it = load_encoder(&source("encode.t1"));
    let syms = symbols(&[]);
    let mut disagreed: Vec<String> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    let mut per_family: BTreeMap<String, usize> = BTreeMap::new();
    for (src, want, riscv) in &cases {
        let mnemonic = riscv.split_whitespace().next().unwrap_or("").to_string();
        *per_family.entry(mnemonic).or_default() += 1;
        let (value, _inst) = mirror(&mut it, src);
        match encode_at_t1(&mut it, value, 0, syms.clone()) {
            None => refused.push(format!("{riscv}: {src:?} — {}", last_code(&it))),
            Some(got) if got != *want => disagreed.push(format!(
                "{riscv}: {src:?} — port {got:#010x}, oracle {want:#010x}"
            )),
            Some(_) => {}
        }
    }
    println!("METRIC t1_vector_oracle_cases {}", cases.len());
    for (m, n) in &per_family {
        println!("  {m:<12} {n}");
    }
    // `vlse64.v` (`V-009` (ii)'s strided load: the matrix kernel's stride-0
    // broadcast and the transpose's column read) has the same floor as the
    // seven — the review's follow-up 2.
    for m in [
        "vsetvli", "vle64.v", "vlse64.v", "vse64.v", "vfadd.vv", "vfsub.vv", "vfmul.vv", "vfdiv.vv",
    ] {
        assert!(
            per_family.get(m).is_some_and(|n| *n >= 6),
            "`{m}` has fewer than six oracle cases in the corpus this test read"
        );
    }
    assert!(
        refused.is_empty(),
        "the `.t1` encoder REFUSED {} vector case(s) GNU binutils accepted:\n  {}",
        refused.len(),
        refused.join("\n  ")
    );
    assert!(
        disagreed.is_empty(),
        "the `.t1` encoder disagrees with the oracle on {} of {} vector case(s):\n  {}",
        disagreed.len(),
        cases.len(),
        disagreed.join("\n  ")
    );
}

/// **`V-008` part 2 — a register never fills another file's field, in the
/// port.** The twin of `encode.rs`'s `a_vector_register_never_fills_a_scalar_field`:
/// the files swapped on the vector load/store's own shape match `vle64.v` and
/// `vse64.v` BY COUNT, so only a per-field check refuses them. Each line must be
/// REFUSED; the control (the same names in the right roles) must encode.
#[test]
fn the_port_refuses_a_register_in_another_files_field() {
    let rows = encoding_rows_text();
    let mn = |insn: &str| -> String {
        rows.lines()
            .map(|l| l.split('\t').collect::<Vec<_>>())
            .find(|f| f.len() >= 3 && f[0] == insn)
            .map(|f| f[2].to_string())
            .unwrap_or_else(|| panic!("{insn} is in the encoding table"))
    };
    let regs =
        std::fs::read_to_string(spec_root().join("registers-riscv64.tsv")).expect("registers");
    let reg = |abi: &str| -> String {
        regs.lines()
            .map(|l| l.split('\t').collect::<Vec<_>>())
            .find(|f| f.len() >= 4 && f[1] == abi)
            .map(|f| f[0].to_string())
            .unwrap_or_else(|| panic!("{abi} is in the register table"))
    };
    let (ld, sd, fadd) = (mn("ld"), mn("sd"), mn("fadd.d"));
    let (v1, v2, v3, t1, t2) = (reg("v1"), reg("v2"), reg("v3"), reg("t1"), reg("t2"));
    let mut it = load_encoder(&source("encode.t1"));
    let syms = symbols(&[]);
    for src in [
        format!("{ld} {t1}म् {v1}त् ।"),
        format!("{sd} {v1}य् {t1}न ।"),
        format!("{ld} {v1}म् {t1}त् ०न ।"),
        format!("{fadd} {v1}म् {t1}न {t2}न ।"),
    ] {
        let (value, _inst) = mirror(&mut it, &src);
        let got = encode_at_t1(&mut it, value, 0, syms.clone());
        assert!(
            got.is_none(),
            "{src} encoded as {got:x?}: a register reached another file's field"
        );
    }
    for (src, want) in [
        (format!("{ld} {v1}म् {t1}त् ।"), 0x0203_7087_u32),
        (format!("{fadd} {v1}म् {v2}न {v3}न ।"), 0x0221_90d7),
    ] {
        let (value, _inst) = mirror(&mut it, &src);
        assert_eq!(
            encode_at_t1(&mut it, value, 0, syms.clone()),
            Some(want),
            "{src}"
        );
    }
}

fn encoding_rows_text() -> String {
    std::fs::read_to_string(spec_root().join("encodings-riscv64.tsv")).expect("encodings")
}

/// **`fence`'s operands are DOMAIN SETS**, and a domain is neither a register
/// nor a numeral: it is a set of names landing in an immediate field.
/// Counting one as a register is what made `स्मृतिबन्धः` report "no encoding
/// taking 2 register(s)". अपादान is what the fence orders FROM and सम्प्रदान
/// what it orders TO — predecessor and successor.
#[test]
fn a_fence_puts_its_two_domain_sets_in_the_two_immediates_the_table_gives_it() {
    let (t1, rust) = encoded("स्मृतिबन्धः पठनम्त् लेखनम्य् ।");
    let (pattern, slots) = spec_row("fence");
    // `पठनम्` is bit २ and `लेखनम्` bit १ of `spec/fence-domains-riscv64.tsv`;
    // the predecessor set is the FIRST immediate in written order.
    assert_eq!(t1, pattern | place(&slots[0], 2) | place(&slots[1], 1));
    assert_eq!(t1, rust);
}

// ── the refusals ─────────────────────────────────────────────────────────

/// The diagnostic code one refusal leaves behind, or the word it encoded to.
fn refusal(src: &str) -> Result<u32, String> {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, _) = mirror(&mut it, src);
    match encode_at_t1(&mut it, value, 0, symbols(&[])) {
        Some(w) => Ok(w),
        None => Err(last_code(&it)),
    }
}

/// **Every refusal names WHICH diagnostic, and the code is what a caller
/// reads.** Asserting the PROSE would pin whichever language the compiler
/// happens to speak, which is what `B-015` moved diagnostics into a table to
/// stop. The codes are `spec/diagnostics.tsv`'s, and this file spells them as
/// the ASCII they are — which is also the whole of what `सङ्केतनकूटः` builds.
#[test]
fn each_way_of_writing_an_instruction_wrong_names_its_own_diagnostic() {
    // E04 — the shape. `योगः` has no encoding taking four registers.
    assert_eq!(
        refusal("योगः शून्यःम् शून्यःन शून्यःन शून्यःन ।"),
        Err("E04".to_string())
    );
    // E02 — a name that is neither a register nor a numeral, on a family no
    // encoding of which takes a target. The reader is told the NAME is
    // unknown rather than that the shape is wrong (`B-055`'s reader guard).
    assert_eq!(
        refusal("योगः शून्यःम् शून्यःन अपरिचितम्न ।"),
        Err("E02".to_string())
    );
    // E05 — a branch to a label this program does not define. With `pending`
    // as `None` that is an error, which is what an executable needs.
    assert_eq!(
        refusal("समलङ्घनम् शून्यःन शून्यःत् अज्ञातम्य् ।"),
        Err("E05".to_string())
    );
    // E12 — a branch target marked with any kāraka but सम्प्रदान. The sigil
    // is the whole of D-02-C: a target that could be marked करण would make
    // "jump to" and "jump from" spellable the same way.
    assert_eq!(
        refusal("समलङ्घनम् शून्यःन शून्यःत् लक्ष्यन ।"),
        Err("E12".to_string())
    );
    // E17 — an immediate too wide for the field it was given. Silently
    // truncating it would encode a different, valid number.
    assert_eq!(refusal("योगः शून्यःम् शून्यःन ९९९९९न ।"), Err("E17".to_string()));
    // E20 — `ॱन` on a family whose own name says it is signed (ADR-0006).
    assert_eq!(
        refusal("सचिह्नदक्षिणसरणम्ॱन३२ शून्यःम् शून्यःन शून्यःन ।"),
        Err("E20".to_string())
    );
    // E21 — and `ॱअ` on one whose name says it is unsigned. Two codes and not
    // one: what differs between the two messages is the message.
    assert_eq!(
        refusal("अचिह्नन्यूनम्ॱअ३२ शून्यःम् शून्यःन शून्यःन ।"),
        Err("E21".to_string())
    );
    // E03 — a float type on a family that operates on integers. Which class
    // an encoding wants is derivable and needs no column: one with a `freg`
    // slot operates on floats (`B-077`).
    assert_eq!(
        refusal("योगःॱप३२ शून्यःम् शून्यःन शून्यःन ।"),
        Err("E03".to_string())
    );
    // E09 — and the other way round. Blaming the operand count here would
    // send the reader to check registers that were correct.
    assert_eq!(
        refusal("प्लवयोगःॱअ३२ प्लव०म् प्लव१न प्लव२न ।"),
        Err("E09".to_string())
    );
}

/// **The Rust encoder refuses the same six**, so what is refused is a
/// property of the language and not of this port.
///
/// The CODE is not compared across the two: `EncodeError` carries one, but
/// nothing public exposes it, and `B-078b` is the row for that. What IS
/// compared is that both refuse, which is the half a port gets wrong by
/// accepting.
#[test]
fn the_rust_encoder_refuses_everything_this_port_refuses() {
    for src in [
        "योगः शून्यःम् शून्यःन शून्यःन शून्यःन ।",
        "योगः शून्यःम् शून्यःन अपरिचितम्न ।",
        "समलङ्घनम् शून्यःन शून्यःत् अज्ञातम्य् ।",
        "समलङ्घनम् शून्यःन शून्यःत् लक्ष्यन ।",
        "योगः शून्यःम् शून्यःन ९९९९९न ।",
        "सचिह्नदक्षिणसरणम्ॱन३२ शून्यःम् शून्यःन शून्यःन ।",
        "अचिह्नन्यूनम्ॱअ३२ शून्यःम् शून्यःन शून्यःन ।",
        "योगःॱप३२ शून्यःम् शून्यःन शून्यःन ।",
        "प्लवयोगःॱअ३२ प्लव०म् प्लव१न प्लव२न ।",
    ] {
        let inst = sadhana::parse::assemble_program(src)
            .unwrap_or_else(|e| panic!("{src:?} parses: {e:?}"))
            .instructions[0]
            .clone();
        assert!(
            sadhana::encode::encode(&inst).is_err(),
            "{src:?} must be refused by the Rust encoder too"
        );
        assert!(refusal(src).is_err(), "{src:?} must be refused");
    }
}

/// **A displacement is judged by its SIGNED range.** Judging a positive one
/// by the unsigned range accepts values that then encode as negative: a
/// branch 4096 bytes forward became `0x800000e3`, a branch 4096 bytes
/// BACKWARD, and the corpus could not catch it because a generator emitting
/// an out-of-range branch measures GNU `as`'s long-branch rewrite instead
/// (`B-012`).
#[test]
fn a_branch_out_of_reach_is_refused_and_the_signed_range_is_why() {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, _) = mirror(&mut it, "समलङ्घनम् शून्यःन शून्यःत् लक्ष्यय् ।");
    // `beq`'s displacement is thirteen bits, two's complement, so 4096 bytes
    // forward is out of reach and 4094 is not.
    assert_eq!(
        encode_at_t1(&mut it, value.clone(), 0, symbols(&[("लक्ष्य", 4096)])),
        None,
        "4096 bytes forward does not fit a signed 13-bit displacement"
    );
    assert_eq!(last_code(&it), "E07");
    assert!(
        encode_at_t1(&mut it, value, 0, symbols(&[("लक्ष्य", 4094)])).is_some(),
        "4094 bytes forward does"
    );
}

/// **A family with no encoding at all is the FIRST thing the body asks**, and
/// it is the one refusal `mirror` cannot reach: every family the parser knows
/// has rows in `spec/encodings-riscv64.tsv`. So the registry row is built by
/// hand here — through `आज्ञायोजनम्` still, only with a key the table does
/// not carry.
#[test]
fn a_family_the_encoding_table_does_not_carry_is_refused_before_anything_else() {
    let mut it = load_encoder(&source("encode.t1"));
    let family = record(vec![
        // A Latin key, because field १ of the table is Latin and this is the
        // column `कुलपङ्क्तिवत्` compares against.
        ("कूट", octets("nosuchfamily")),
        ("नाम", octets("अविद्यमानम्")),
        ("व्याख्या", octets("-")),
        ("विस्तारः", octets("I")),
        ("आवरणारम्भ", Value::Int(0)),
        ("आवरणसंख्यान", Value::Int(0)),
    ]);
    let at = it
        .call(
            "वाक्यविभागॱआज्ञायोजनम्",
            vec![
                family,
                Value::Int(4),
                octets(""),
                octets(""),
                Value::Int(0),
                Value::Int(0),
                Value::Int(0),
                Value::Int(7),
            ],
            20_000_000,
        )
        .expect("आज्ञायोजनम् runs")
        .as_int()
        .expect("an index");
    let inst = arena_at(&it, "वाक्यविभागआज्ञाकोश", usize::try_from(at).expect("fits"));

    assert_eq!(encode_at_t1(&mut it, inst, 0, symbols(&[])), None);
    assert_eq!(last_code(&it), "E01");
    assert_eq!(
        last_args(&it),
        vec!["अविद्यमानम्".to_string()],
        "the message names the FAMILY, which is what the writer wrote, and \
         not the Latin key, which is the table's"
    );
}

/// The arguments the last refusal left in `वाक्यविभाग ॱ पाठांशकोश`, as text.
///
/// That arena is the ONLY place `निदान ॱ पदार्थलेखनम्` can read an argument
/// from, so what is asserted here is what a renderer would interpolate.
fn last_args(it: &Interpreter) -> Vec<String> {
    let err = it
        .global("अन्तिमसङ्केतनदोषः")
        .expect("अन्तिमसङ्केतनदोषः is a name in scope");
    let first = record_field(err, "पदार्थारम्भ")
        .as_int()
        .expect("a first index");
    let count = record_field(err, "पदार्थसंख्यान").as_int().expect("a count");
    (0..count)
        .map(|n| {
            let piece = arena_at(
                it,
                "पाठांशकोश",
                usize::try_from(first + n).expect("an index"),
            );
            let text = record_field(&piece, "पाठ");
            let bytes = text.octets().expect("octets").as_slice().to_vec();
            let from = usize::try_from(record_field(&piece, "आरम्भ").as_int().expect("a start"))
                .expect("fits");
            let to = usize::try_from(record_field(&piece, "सीमा").as_int().expect("an end"))
                .expect("fits");
            String::from_utf8(bytes[from..to].to_vec()).expect("utf-8")
        })
        .collect()
}

/// Every distinct `ॱ{written}ॱ{read}` the `fcvt` family names, sorted and
/// deduped — Rust's `sort`/`dedup`/`join(" ")` over the candidates' `converts`
/// column, computed HERE from `spec/encodings-riscv64.tsv`.
fn conversion_pairs(family: &str) -> String {
    let text = encodings_text();
    let mut out: Vec<String> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("insn\t"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 9 || f[1] != family || f[5] != "32" || f[8] == "-" {
                return None;
            }
            let (a, b) = f[8].split_once(',')?;
            Some(format!("\u{971}{a}\u{971}{b}"))
        })
        .collect();
    out.sort();
    out.dedup();
    out.join(" ")
}

/// **A conversion that found nothing failed on its TYPES, not on its shape**,
/// and the message says which pairs exist — so E10 carries a LIST, and this
/// test is the only thing that reads `परिवर्तनसूची`'s sort, its dedup and its
/// join.
///
/// The dedup is a real filter and not a formality: two rows of one family can
/// name the same pair, and a list that repeated it would tell the reader to
/// write something twice. The expected list is built here out of column ९ of
/// the spec table, with Rust's own `sort` and `dedup`.
#[test]
fn a_conversion_without_its_pair_is_told_which_pairs_exist() {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, inst) = mirror(&mut it, "प्लवरूपान्तरम्ॱअ३२ अर्थ०म् प्लव१न ।");
    assert_eq!(
        encode_at_t1(&mut it, value, 0, symbols(&[])),
        None,
        "one type on a conversion names no pair"
    );
    assert_eq!(last_code(&it), "E10");

    let args = last_args(&it);
    assert_eq!(
        args.len(),
        3,
        "E10 takes the family, the types and the list"
    );
    assert_eq!(args[0], inst.family.name, "the family at fault");
    assert_eq!(args[1], inst.types.join("\u{971}"), "the types as written");
    assert_eq!(
        args[2],
        conversion_pairs(&inst.family.key),
        "the pairs, sorted and deduped, out of column ९ of the spec table"
    );
    // Not vacuous: the family really does name more than one pair, and they
    // really do repeat before the dedup.
    assert!(
        args[2].split(' ').count() > 4,
        "the fcvt family names several pairs: {}",
        args[2]
    );

    // And the Rust encoder refuses it for the same reason, naming the same
    // family and the same list inside its rendered sentence.
    let e = sadhana::encode::encode(&inst).expect_err("must not encode");
    assert!(e.reason.contains(&inst.family.name), "{}", e.reason);
    assert!(e.reason.contains(&args[2]), "{}", e.reason);
}

/// **अधिकरण has no field in T0 and is refused rather than guessed at.**
/// It used to be treated as करण, which happened to give the right answer for a
/// load — rd and rs1 are both free there — and silently corrupted a store,
/// where it fell through to rs2 and was OR-ed on top of the value operand.
#[test]
fn the_locus_karaka_is_refused_and_not_read_as_an_instrument() {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, inst) = mirror(&mut it, "निधानम् क्षणिक०न स्तूपसूचकःए ०न ।");
    assert_eq!(encode_at_t1(&mut it, value, 0, symbols(&[])), None);
    assert_eq!(last_code(&it), "E14");
    assert_eq!(
        last_args(&it),
        vec!["स्तूपसूचकः".to_string()],
        "the operand at fault is named"
    );
    assert!(
        sadhana::encode::encode(&inst).is_err(),
        "the Rust encoder refuses अधिकरण too"
    );
}

// ── the mutations ────────────────────────────────────────────────────────

/// What `योगः क्षणिक०म् स्तूपसूचकःन विश्वसूचकःन ।` encodes to under a mutated
/// `encode.t1`, or the code it refused with.
fn mutated_add(from: &str, to: &str) -> Result<u32, String> {
    let mut it = load_encoder(&mutate(&source("encode.t1"), from, to));
    let (value, _) = mirror(&mut it, "योगः क्षणिक०म् स्तूपसूचकःन विश्वसूचकःन ।");
    match encode_at_t1(&mut it, value, 0, symbols(&[])) {
        Some(w) => Ok(w),
        None => Err(last_code(&it)),
    }
}

/// **The word starts from the PATTERN.** Starting it from ० leaves every
/// register field right and produces a word that is not an instruction —
/// which is why `ENCODER_WORD_BUILDERS` in `t1_sources.rs` measures a body by
/// whether it reads `ॱ आकृति` at all rather than by whether it has a body.
#[test]
fn starting_the_word_from_zero_instead_of_the_pattern_is_caught() {
    let unmutated = encoded("योगः क्षणिक०म् स्तूपसूचकःन विश्वसूचकःन ।").0;
    let mutated = mutated_add(
        "चरः आज्ञापदम् ॱॱ अ३२ भवति वृतसङ्केतः ॱ आकृति ।",
        "चरः आज्ञापदम् ॱॱ अ३२ भवति ० ।",
    )
    .expect("it still encodes, which is exactly the danger");
    assert_eq!(
        mutated,
        unmutated & !spec_row("add").0,
        "with the pattern dropped only the operand fields are left"
    );
    assert_ne!(mutated, unmutated);
}

/// **`करण` takes rs1 only ONCE.** Without the flag both source operands take
/// rs1, and the second collides with the first — which `B-044`'s guard turns
/// into E16 rather than into a silent bitwise OR of two registers.
#[test]
fn letting_the_first_source_field_be_claimed_twice_is_caught() {
    assert_eq!(
        mutated_add(
            "                        प्रथमगृहीतम् भवति सत्यम् ।\n                        अवकाशक्रमः भवति प्रथमक्रमः ।",
            "                        अवकाशक्रमः भवति प्रथमक्रमः ।",
        ),
        Err("E16".to_string()),
        "with used_rs1 never set the second करण claims rs1 again"
    );
}

/// **The register table's one-based row number is undone in `मूल्याङ्कः` and
/// nowhere else**, which is what that routine's own margin says. Dropping the
/// correction moves every register up by one and the instruction still
/// assembles — a valid instruction on the wrong registers.
#[test]
fn dropping_the_one_based_correction_moves_every_field_by_one() {
    let mutated = mutate(
        &source("encode.t1"),
        "        प्रत्यागमनम् कोष्ठम् वियोगः १ ।",
        "        प्रत्यागमनम् कोष्ठम् ।",
    );
    let mut it = load_encoder(&mutated);
    let (value, _) = mirror(&mut it, "योगः क्षणिक०म् स्तूपसूचकःन विश्वसूचकःन ।");
    let word = encode_at_t1(&mut it, value, 0, symbols(&[])).expect("it still encodes");
    let (pattern, slots) = spec_row("add");
    assert_eq!(
        word,
        pattern | place(&slots[0], 6) | place(&slots[1], 3) | place(&slots[2], 4),
        "every register is one too high"
    );
}

/// **The width filter chooses among encodings of IDENTICAL shape.**
///
/// The `add` family covers `add`, `addi`, `addiw` and `addw`, and `add` and
/// `addw` take exactly the same three registers — nothing but the width tells
/// them apart. `addw` STATES a width, so it also ends the walk; with the
/// width test dropped it therefore wins outright and a 64-bit addition
/// assembles as a 32-bit one. That is a valid instruction computing the wrong
/// thing, which is the failure this test exists for.
#[test]
fn dropping_the_width_test_from_the_selection_loop_is_caught() {
    let (add, add_slots) = spec_row("add");
    let (addw, addw_slots) = spec_row("addw");
    let fields = |p: u32, s: &[SpecSlot]| p | place(&s[0], 5) | place(&s[1], 2) | place(&s[2], 3);

    assert_eq!(
        encoded("योगः क्षणिक०म् स्तूपसूचकःन विश्वसूचकःन ।").0,
        fields(add, &add_slots),
        "a bare mnemonic is 64-bit, so `add` and not `addw`"
    );

    let mutated = mutated_add(
        "                    यदि विस्तारः असमम् अपेक्षितांशाः आदि\n                        स्वीकार्यम् भवति असत्यम् ।\n                    इति",
        "                    यदि विस्तारः असमम् अपेक्षितांशाः आदि\n                    इति",
    )
    .expect("it still encodes, which is exactly the danger");
    assert_eq!(
        mutated,
        fields(addw, &addw_slots),
        "with the width test gone the first row STATING a width wins the walk"
    );
    assert_ne!(mutated, fields(add, &add_slots));
}

/// **A refusal must not answer a word.** Every diagnostic site returns
/// `शून्यम्`, and `शून्यम्` is a different value from `०` — a port that
/// answered ० would report the instruction `0x00000000`, which on RISC-V is
/// not an instruction at all: it traps.
#[test]
fn a_refusal_is_nothing_and_not_the_number_zero() {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, _) = mirror(&mut it, "योगः शून्यःम् शून्यःन शून्यःन शून्यःन ।");
    let v = it
        .call(
            "सङ्केतनॱस्थानसङ्केतनम्",
            vec![value, Value::Int(0), symbols(&[])],
            600_000_000,
        )
        .expect("it runs");
    assert!(
        v.is_nil(),
        "a refusal answers शून्यम्, answered {v:?}; ० is a word and not a refusal"
    );
}

/// **`सङ्केतनम्` is `encode_at inst ० {}` and nothing else** — the first of
/// the five delegations, and the empty symbol table is an arena holding one
/// शून्यम्, which is how this language spells `&BTreeMap::new()`.
#[test]
fn the_bare_encoder_is_the_addressed_one_at_zero_with_no_symbols() {
    let mut it = load_encoder(&source("encode.t1"));
    let (value, inst) = mirror(&mut it, "योगः क्षणिक०म् स्तूपसूचकःन विश्वसूचकःन ।");
    let bare = it
        .call("सङ्केतनॱसङ्केतनम्", vec![value.clone()], 600_000_000)
        .expect("सङ्केतनम् runs")
        .as_int()
        .expect("a word");
    assert_eq!(
        bare,
        i128::from(encode_at_t1(&mut it, value, 0, symbols(&[])).expect("encodes")),
        "सङ्केतनम् must be स्थानसङ्केतनम् at ० with an empty table"
    );
    assert_eq!(
        bare,
        i128::from(sadhana::encode::encode(&inst).expect("encodes")),
        "and it must agree with the Rust encoder"
    );
}

// ── the symbol table between the two passes ────────────────────────────────

/// `चिह्नसङ्ग्रहः` places a label in each of the three sections.
///
/// WRITTEN WITH THE ROUTINE, 2026-08-31, and not after it. `चिह्ननाम` and
/// `चिह्नसङ्ग्रहः` were added so `लक्ष्यसङ्केतनम्` has a symbol table to
/// build; the whole T1 suite passed the moment they were written, because
/// NOTHING CALLED THEM. A green suite over unexercised code is the blind
/// instrument this file exists to refuse, so the addresses below are computed
/// by hand from `encode.rs:542-566` rather than read back from the routine.
///
/// Three 32-bit instructions put `pc` at १२. `दत्ताधारः` is १२ aligned up to
/// the page = ४०९६ (W-363, `कोशॱदत्तपृष्ठाधारः`; it was ८ = १६ before); there
/// is no `ॱदत्त` entry, so `रिक्ताधारः` is ४०९६ too. A text label
/// takes the address of the instruction it marks; a data or bss label takes
/// its base plus `दत्तसरण`. Every address is RELATIVE to the image base —
/// that is the whole point of the table and the thing encode.rs:544 records
/// getting wrong once, when `0x80000000` reached an `auipc`.
#[test]
fn the_symbol_table_places_a_label_in_each_section() {
    let mut it = load_layout(&source("encode.t1"));

    for n in 0..3 {
        it.call(
            "वाक्यविभागॱआज्ञायोजनम्",
            vec![
                Value::Int(0),
                Value::Int(0),
                octets(""),
                octets(""),
                Value::Int(0),
                Value::Int(0),
                Value::Int(0),
                Value::Int(i128::from(n) + 1),
            ],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("आज्ञायोजनम् runs: {e:?}"));
    }

    // (name, section, दत्तसरण, आज्ञाक्रम) — one label per section.
    // The name is sliced out of a longer source, so `चिह्ननाम` is exercised
    // on a genuine offset and not on a whole run that a copy would satisfy.
    let src = "आदिःदत्तम्रिक्तम्";
    let names: [(&str, i128, i128, i128); 3] = [
        ("आदिः", 1, 0, 1),  // text, marks instruction १
        ("दत्तम्", 2, 4, 0),  // data, four octets in
        ("रिक्तम्", 3, 8, 0), // bss, eight octets in
    ];
    for (name, section, offset, at) in names {
        let start = src.find(name).expect("the name lies in the source");
        let start = i128::try_from(src[..start].len()).expect("fits");
        let end = start + i128::try_from(name.len()).expect("fits");
        it.call(
            "वाक्यविभागॱचिह्नयोजनम्",
            vec![
                octets(src),
                Value::Int(start),
                Value::Int(end),
                Value::Int(section),
                Value::Int(offset),
                Value::Int(at),
                Value::Int(1),
            ],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("चिह्नयोजनम् runs: {e:?}"));
    }

    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना runs");
    let tgt = target(&it, "असङ्कुचितम्");
    let addresses = it
        .call("सङ्केतनॱस्थानविन्यासः", vec![program.clone(), tgt], 50_000_000)
        .expect("स्थानविन्यासः runs");

    let table = it
        // असत्यम् = all three sections. The object encoder passes सत्यम् for
        // text-only, because an object's data and bss addresses are the
        // linker's to fill — see `चिह्नसङ्ग्रहः`'s own head.
        .call(
            "सङ्केतनॱचिह्नसङ्ग्रहः",
            vec![program, addresses, Value::Bool(false)],
            50_000_000,
        )
        .expect("चिह्नसङ्ग्रहः runs");

    // Read each name back through `चिह्नस्थानम्`, which is `symbols.get`. That
    // exercises `चिह्ननाम` too: a wrong slice answers शून्यम् here, because
    // `समानपाठः` compares the length first.
    for (name, _, _, _) in names {
        let got = it
            .call(
                "सङ्केतनॱचिह्नस्थानम्",
                vec![table.clone(), octets(name)],
                20_000_000,
            )
            .unwrap_or_else(|e| panic!("चिह्नस्थानम् runs: {e:?}"));
        let expected = match name {
            "आदिः" => 4,    // स्थानानि[१]
            "दत्तम्" => 4100, // दत्ताधारः ४०९६ + ४
            _ => 4104,      // रिक्ताधारः ४०९६ + ८
        };
        assert_eq!(
            got.as_int(),
            Some(expected),
            "`{name}` must resolve to {expected}; a शून्यम् here means \
             `चिह्ननाम` sliced the wrong octets, since `चिह्नस्थानम्` \
             compares the length before the bytes"
        );
    }
}

/// `लक्ष्यसङ्केतनम्` emits a text section, and its octets are the Rust ones.
///
/// WRITTEN WITH THE BODY, for the reason the symbol-table test above records:
/// the whole T1 suite went green the moment `लक्ष्यसङ्केतनम्` was written,
/// because nothing called it. A routine that parses is not a routine that
/// runs — and this file's own history is that the `समाप्तम्` trap produces
/// code which parses while silently dropping terms.
///
/// The expected octets are NOT read back from T1. They come from Rust's
/// `encode_at` over the same instruction, little-endian, which is the only
/// comparison that can fail for the right reason.
#[test]
fn the_program_encoder_emits_the_octets_rust_emits() {
    // `load_encoder` AND NOT `load_layout`. The layout loader brings only
    // `encode.t1` and `वाक्यविभाग`, which is enough for `स्थानविन्यासः` — but
    // `कुलक्षेत्रयोग्यम्` calls `विश्लेषणॱक्षेत्रवाचकः`, and with `विश्लेषण`
    // absent that QUALIFIED NAME does not resolve as a call. The parser then
    // reads it as a bare identifier, the `यदि` condition ends early, and the
    // failure surfaces as "expected `आदि`, found `पाठ्यम्`" — a PARSE error
    // reported at run time, from a routine nobody edited, three hundred lines
    // from anything this test wrote.
    let mut it = load_encoder(&source("encode.t1"));

    // REAL INSTRUCTIONS, THROUGH `mirror`, AND NOT HAND-BUILT ONES. The first
    // draft of this test used `build` above, which fills an आज्ञा's fields with
    // `Int(0)`. That is enough for `स्थानविन्यासः`, which never looks inside an
    // instruction — but the full encoder does, and it failed with
    // "`ॱ कूट` read from Int(0), which has no such member". A synthetic program
    // tests the layout and nothing else.
    //
    // Symbol-free on purpose: this asserts the emit path, not resolution.
    let src = "योगः क्षणिक६म् अर्थ१न ०न ।";
    let mut expected: Vec<i128> = Vec::new();
    for _ in 0..3 {
        let (_, inst) = mirror(&mut it, src);
        let word = sadhana::encode::encode(&inst).expect("the Rust encoder encodes it");
        for k in 0..4 {
            expected.push(i128::from((word >> (k * 8)) as u8));
        }
    }

    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना runs");
    let tgt = target(&it, "असङ्कुचितम्");

    let emitted = it
        .call("सङ्केतनॱलक्ष्यसङ्केतनम्", vec![program, tgt], 200_000_000)
        .expect("लक्ष्यसङ्केतनम् runs");

    // A refusal answers शून्यम्; anything else must be the octet run.
    let bytes: Vec<i128> = match emitted {
        Value::Octets(o) => o.as_slice().iter().map(|b| i128::from(*b)).collect(),
        Value::Arena(a) => a
            .borrow()
            .iter()
            .map(|e| e.as_int().expect("an octet"))
            .collect(),
        other => panic!(
            "लक्ष्यसङ्केतनम् must answer a text section; it answered {other:?}. \
             शून्यम् here means it refused — read the recorded सङ्केतनदोष"
        ),
    };

    assert_eq!(
        bytes.len(),
        12,
        "three 32-bit instructions are twelve octets; got {bytes:?}"
    );
    assert_eq!(
        bytes, expected,
        "the T1 encoder must emit the octets the Rust encoder emits, in the \
         same little-endian order. A length that matches with different bytes \
         is the E23 class of defect: every instruction individually well \
         formed and the program still wrong"
    );
}

/// `लक्ष्यवस्तुसङ्केतनम्` records an undefined name instead of refusing it.
///
/// WRITTEN WITH THE ROUTINE. This is the whole difference between the object
/// and program encoders: the same source that assembles as an object must be
/// REFUSED as an executable, because an executable may not name what nothing
/// defines. Asserting only that the object path succeeds would pass over a
/// `प्रतीक्षाग्रहणम्` that was never read — so both directions are checked.
#[test]
fn an_object_records_an_undefined_name_and_a_program_refuses_it() {
    let src = "समलङ्घनम् शून्यःन शून्यःत् अपरिचितलक्ष्यय् ।";

    // The object path: assembles, leaving the field for the linker.
    let mut it = load_encoder(&source("encode.t1"));
    mirror(&mut it, src);
    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना runs");
    let obj = it
        .call("सङ्केतनॱवस्तुसङ्केतनम्", vec![program], 200_000_000)
        .expect("वस्तुसङ्केतनम् runs");
    assert!(
        !matches!(obj, Value::Nil),
        "an object may name what it does not define; `वस्तुसङ्केतनम्` refused \
         it instead of recording a प्रतीक्षा"
    );

    // And the arena holds it, so the linker has something to patch.
    // THE CURSOR, not the arena's length. `प्रतीक्षासूचकाङ्क` is what says how
    // many entries are live; slots past it are stale from an earlier call and
    // counting them would make this assertion pass on someone else's work.
    let pendings = it
        .global("प्रतीक्षासूचकाङ्क")
        .and_then(|v| v.as_int())
        .expect("प्रतीक्षासूचकाङ्क is a name in scope");
    assert!(
        pendings >= 1,
        "the undefined name must be RECORDED, not silently zeroed; \
         प्रतीक्षासूचकाङ्क is {pendings}"
    );

    // The program path: the same source must be refused.
    let mut it2 = load_encoder(&source("encode.t1"));
    mirror(&mut it2, src);
    let program2 = it2
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना runs");
    let exe = it2
        .call("सङ्केतनॱकार्यक्रमसङ्केतनम्", vec![program2], 200_000_000)
        .expect("कार्यक्रमसङ्केतनम् runs");
    assert!(
        matches!(exe, Value::Nil),
        "an EXECUTABLE may not name what nothing defines; it must be E05, and \
         a non-nil answer here means प्रतीक्षाग्रहणम् leaked across calls"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `सङ्केतन ॱ सङ्कोचः` — `encode.rs`'s `compressed_at`, and the LAST of the
// eight `ENCODER_PROGRAM_ROUTINES` to get a body (`D-002a2`, 2026-09-04).
//
// # What the row said and what the tree said
//
// The row sent its worker after "twelve stubs" behind "a bitwise operator T1
// lacks". The file had ONE stub, this one, and `पदरचना` had been writing
// `विकल्प` — Rust's `|=` — since 2026-08-30. What `सङ्कोचः` waited on was
// its caller chain, and that is why the tests below reach past the routine:
// `स्थानविन्यासः` must SHRINK under `सङ्कुचितम्`, and `लक्ष्यसङ्केतनम्` must
// EMIT the halfword the layout counted, or the routine is right and the
// assembler is still wrong.
//
// # Oracles
//
// Three, and none of them is `encode.t1`:
//   * Rust's `compressed_at`, `layout_addresses` and `encode_program_for`
//     over the SAME parsed instructions — `mirror` hands each line to the
//     Rust parser and pushes exactly what came back;
//   * `spec/compression-choices.tsv`, read HERE with `lines`/`split`: the
//     `halfword` column is what GNU produced for that row, and `-` is what
//     GNU left wide;
//   * the six halfwords `riscv64-elf-as -march=rv64gc` produced for
//     `crates/sadhana/tests/compressed.rs`'s program, copied from that test.
// ─────────────────────────────────────────────────────────────────────────

/// `crates/sadhana/tests/compressed.rs:318-326`, verbatim. Every instruction
/// compresses, and the branch at instruction ३ reaches the label after
/// instruction ४ only because the two between them are two octets each.
const COMPRESSED_PROGRAM: &str = "\
योगः स्थिर०म् शून्यःन ५न ।
योगः स्थिर०म् स्थिर०न १न ।
योगः स्थिर०म् स्थिर०न स्थिर१न ।
समलङ्घनम् स्थिर०न शून्यःत् समाप्तम्य् ।
योगः स्थिर१म् स्थिर१न १न ।
समाप्तम्ॱॱ
योगः अर्थ०म् शून्यःन ०न ।
";

/// What `riscv64-elf-as` emitted for `COMPRESSED_PROGRAM`, per
/// compressed.rs:315-317 — `li s0,5; addi s0,s0,1; add s0,s0,s1; beqz s0,.+4;
/// addi s1,s1,1; li a0,0`.
const COMPRESSED_HALFWORDS: [u16; 6] = [0x4415, 0x0405, 0x9426, 0xc011, 0x0485, 0x4501];

/// A whole Sassembly program mirrored into `वाक्यविभाग` — every instruction
/// through `mirror`, every label through `चिह्नयोजनम्` with the `at` and
/// section the RUST parser gave it — and sealed by `कार्यक्रमरचना`.
///
/// The label's `at` is read off Rust's `Label` rather than counted by hand,
/// so the T1 program and the Rust oracle agree about where the label sits by
/// construction and disagree only if the layout does.
fn mirror_program(it: &mut Interpreter, src: &str) -> Value {
    let rust = sadhana::parse::assemble_program(src).expect("parses");
    for line in src.lines() {
        // A label ends at its mark and is not an instruction.
        if line.trim().is_empty() || line.ends_with("ॱॱ") {
            continue;
        }
        mirror(it, line);
    }
    for l in &rust.labels {
        let section: i128 = match l.section {
            sadhana::parse::Section::Text => 1,
            sadhana::parse::Section::Data => 2,
            sadhana::parse::Section::Bss => 3,
        };
        it.call(
            "वाक्यविभागॱचिह्नयोजनम्",
            vec![
                octets(&l.name),
                Value::Int(0),
                Value::Int(i128::try_from(l.name.len()).expect("fits")),
                Value::Int(section),
                Value::Int(i128::try_from(l.data_offset).expect("fits")),
                Value::Int(i128::try_from(l.at).expect("fits")),
                Value::Int(i128::try_from(l.line).expect("fits")),
            ],
            20_000_000,
        )
        .unwrap_or_else(|e| panic!("चिह्नयोजनम् runs: {e:?}"));
    }
    it.call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना runs")
}

/// `सङ्कोचः inst pc symbols`, as `Option<u16>`; `None` where it refused.
///
/// A run failure PANICS rather than becoming `None`, for the reason every
/// helper in this file gives: "no 16-bit form" and "the routine did not run"
/// are different facts.
fn compressed_t1(it: &mut Interpreter, inst: Value, pc: i128, syms: Value) -> Option<u16> {
    let run = it.call(
        "सङ्केतनॱसङ्कोचः",
        vec![inst, Value::Int(pc), syms],
        3_000_000_000,
    );
    let v = match run {
        Ok(v) => v,
        Err(e) => panic!("सङ्कोचः runs: {}", explain(it, &e)),
    };
    v.as_int().map(|n| u16::try_from(n).expect("a halfword"))
}

/// One symbol-free line, compressed twice: by the T1 routine and by Rust.
fn compressed_both(it: &mut Interpreter, src: &str) -> (Option<u16>, Option<u16>) {
    let (inst_v, inst) = mirror(it, src);
    let rust = sadhana::encode::compressed_at(&inst, 0, &std::collections::BTreeMap::new());
    let t1 = compressed_t1(it, inst_v, 0, symbols(&[]));
    (t1, rust)
}

/// The `halfword` column of one row of `spec/compression-choices.tsv`, keyed
/// by its `assembly` column; `None` where the row says `-`.
///
/// Read by THIS test with `lines`/`split`, exactly as `forms_of` reads it —
/// and not through `encode.t1`, so the comparison is with the table and not
/// with a second port of the reader.
fn table_halfword(assembly: &str) -> Option<u16> {
    let text = std::fs::read_to_string(spec_root().join("compression-choices.tsv"))
        .expect("spec/compression-choices.tsv exists");
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("assembly\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() >= 4 && f[0] == assembly {
            return (f[1] != "-")
                .then(|| u16::from_str_radix(f[1].trim_start_matches("0x"), 16).expect("hex"));
        }
    }
    panic!("{assembly:?} is not a row of spec/compression-choices.tsv");
}

/// A GNU row of the compression table and the Sassembly line that says the
/// same thing. The pairing is the one fact this file adds: the table's
/// `assembly` column is GNU syntax and `mirror` speaks Sassembly.
const COMPRESSION_ROWS: &[(&str, &str)] = &[
    ("add s0, s0, s1", "योगः स्थिर०म् स्थिर०न स्थिर१न ।"),
    ("addi s0, s0, 5", "योगः स्थिर०म् स्थिर०न ५न ।"),
    ("addi s0, zero, 5", "योगः स्थिर०म् शून्यःन ५न ।"),
    ("sub s0, s0, s1", "वियोगः स्थिर०म् स्थिर०न स्थिर१न ।"),
    ("slli s0, s0, 1", "वामसरणम् स्थिर०म् स्थिर०न १न ।"),
    // The rows GNU left WIDE — `-` in the table — and Rust refuses too.
    ("add s0, s1, a0", "योगः स्थिर०म् स्थिर१न अर्थ०न ।"),
    ("addi s0, s1, 5", "योगः स्थिर०म् स्थिर१न ५न ।"),
    ("addi s0, s0, 2047", "योगः स्थिर०म् स्थिर०न २०४७न ।"),
];

#[test]
fn a_compressible_instruction_answers_the_halfword_the_table_carries() {
    // Row by row, against the file `forms_of` reads: where GNU wrote a
    // halfword the T1 routine must answer exactly it, and where GNU left the
    // instruction wide the routine must answer शून्यम्. The Rust twin rides
    // along as the second oracle, and the two must agree with each other as
    // well as with the table.
    let mut it = load_encoder(&source("encode.t1"));
    for (gnu, sas) in COMPRESSION_ROWS {
        let want = table_halfword(gnu);
        let (t1, rust) = compressed_both(&mut it, sas);
        assert_eq!(
            t1, want,
            "{gnu}: the table says {want:04x?} and सङ्कोचः answered {t1:04x?}"
        );
        assert_eq!(
            t1, rust,
            "{gnu}: Rust's compressed_at answered {rust:04x?}, T1 {t1:04x?}"
        );
    }

    // THE ROW THE TABLE IS NOT AN ORACLE FOR. `addi sp, sp, 32` is `c.addi16sp`
    // 0x6105 in the table, and compressed.rs:291-299 records that the encoder
    // LEAVES IT WIDE on purpose: its immediate must be a non-zero multiple of
    // sixteen and the derived map for that field is partial, so `fits`
    // refuses what the assembler accepts. The port must refuse where its twin
    // refuses — agreeing with the table here would mean it had grown a map
    // Rust does not have.
    let (t1, rust) = compressed_both(&mut it, "योगः स्तूपसूचकःम् स्तूपसूचकःन ३२न ।");
    assert_eq!(
        t1, rust,
        "c.addi16sp: Rust answered {rust:04x?}, T1 {t1:04x?}; the table says \
         0x6105 and compressed.rs says the encoder is conservative here"
    );
}

#[test]
fn a_form_is_refused_where_the_assembler_would_produce_a_different_instruction() {
    // REFUSED, by the guard Rust puts BEFORE any form is tried (encode.rs:806):
    // a result written to `x0` is discarded, so `c.add x0, a0` is a hint and
    // not the operation. GNU ACCEPTS it, which is why the table cannot say so
    // and the guard has to. Three of 5949 rvc conformance cases turned on it.
    let mut it = load_encoder(&source("encode.t1"));
    let (t1, rust) = compressed_both(&mut it, "योगः शून्यःम् शून्यःन अर्थ०न ।");
    assert_eq!(rust, None, "the Rust oracle refuses `add zero, zero, a0`");
    assert_eq!(t1, None, "सङ्कोचः must refuse a form that writes x0");

    // REFUSED: a three-bit field will not take a register outside x8..x15.
    // `c.sub` is x8..x15 only; `sub t0, t0, t1` has the relation and not the
    // registers, so `reduce` passes and `fits` refuses.
    let (t1, rust) = compressed_both(&mut it, "वियोगः क्षणिक०म् क्षणिक०न क्षणिक१न ।");
    assert_eq!(rust, None, "the Rust oracle refuses `sub t0, t0, t1`");
    assert_eq!(
        t1, None,
        "सङ्कोचः must refuse a register no 3-bit field holds"
    );

    // And the same shape with registers the field DOES hold, so the refusal
    // above is attributable to the register and not to `sub`.
    let (t1, rust) = compressed_both(&mut it, "वियोगः स्थिर०म् स्थिर०न स्थिर१न ।");
    assert_eq!(rust, Some(0x8c05));
    assert_eq!(t1, Some(0x8c05), "`sub s0, s0, s1` is c.sub 0x8c05");
}

/// A spec root whose `compression-choices.tsv` is `content` and whose every
/// other table is the repository's.
///
/// **THIS IS A TWIN OF `spec_root_with` IN `t1_exec_aksara.rs`, AND BOTH CARRIED
/// THE SAME DEFECT.** The margin here used to end "the substituted table is
/// WRITTEN, never linked, so nothing here can edit `spec/` through a shared
/// inode". That reasoning holds only *within one process*, and the name below
/// was `encode-spec-<pid>-<counter>` — unique inside a process and NOT ON DISK,
/// with the roots never removed.
///
/// Measured 2026-09-26: **616** `encode-spec-*` directories left in `$TMPDIR`
/// from **616 distinct pids** — one per pid, so a pid repeat alone is enough —
/// and `spec/compression-choices.tsv` carried **1,133 hard links**. A reused pid
/// opens a directory that already holds a link to the table about to be
/// substituted, and `fs::write` (open with `O_TRUNC`) then writes through it
/// into the REPOSITORY. The aksara twin did exactly that in the wild the same
/// evening: the rail's `spec/shiva-sutras.tsv` gained a fixture row and blocked
/// the rail for six fires.
///
/// So: a nanosecond stamp makes the name unique across processes, and the
/// substituted path is unlinked before it is written so the damage is impossible
/// even if a name did repeat. Fixing one twin and not the other is how this
/// project has been bitten before — if a third copy of this routine is ever
/// written, give all three one implementation instead.
fn spec_root_with_compression(content: &str) -> PathBuf {
    let dir = spec_fixture::unique_root("encode-spec");
    std::fs::create_dir_all(&dir).expect("temp spec root");
    for entry in std::fs::read_dir(spec_root()).expect("spec/ is readable") {
        let p = entry.expect("entry").path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().expect("file name").to_owned();
        if name == std::ffi::OsStr::new("compression-choices.tsv") {
            continue;
        }
        let to = dir.join(name);
        if std::fs::hard_link(&p, &to).is_err() {
            std::fs::copy(&p, &to).expect("copy spec file");
        }
    }
    spec_fixture::write_substituted(&dir, "compression-choices.tsv", content);
    dir
}

#[test]
fn a_malformed_relation_refuses_its_form_by_name() {
    // REFUSED: the table reader `सम्बन्धाङ्कः` maps the relation column onto
    // six constants and answers असम्बन्धः — Rust's `_ => None` — for any
    // spelling it does not model. So a row whose relation is `arg1=arg9`
    // refuses the form it names and the instruction stays wide, exactly as a
    // Rust build against that table would (`reduce` falls through). The
    // routine is not asked to guess.
    let sas = "योगः स्थिर०म् स्थिर०न स्थिर१न ।";
    let good = std::fs::read_to_string(spec_root().join("compression-choices.tsv"))
        .expect("spec/compression-choices.tsv exists");
    let row = "add s0, s0, s1\t0x9426\tc.add\targ1=arg0";
    assert!(
        good.contains(row),
        "the row this test rewrites is in the table"
    );
    let bad = good.replace(row, "add s0, s0, s1\t0x9426\tc.add\targ1=arg9");

    let mut it = Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("encode.t1", &source("encode.t1")),
            ("vakyavibhaga.t1", &source("vakyavibhaga.t1")),
            ("vishlesana.t1", &source("vishlesana.t1")),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
        ],
        &spec_root_with_compression(&bad),
    )
    .expect("the modules load against the substituted spec root");
    let (inst_v, _) = mirror(&mut it, sas);
    assert_eq!(
        compressed_t1(&mut it, inst_v, 0, symbols(&[])),
        None,
        "with its relation misspelled the c.add row must refuse `add s0, s0, s1`"
    );

    // And against the real table the same line compresses, so the refusal
    // above is attributable to the row and not to the instruction.
    let mut it = load_encoder(&source("encode.t1"));
    let (t1, _) = compressed_both(&mut it, sas);
    assert_eq!(t1, Some(0x9426));
}

#[test]
fn the_compressed_program_emits_the_octets_the_assembler_emits() {
    // The whole chain, not the routine: `लक्ष्यसङ्केतनम्` under `सङ्कुचितम्`
    // lays out with `सङ्कोचः`, checks E22, and `उत्सर्जनक्रमः` writes the LOW
    // HALFWORD where the layout counted two octets — or E23 fires on the
    // second instruction, because the layout said २ and the emitter wrote ४.
    //
    // The expected octets are `riscv64-elf-as`'s, AND Rust's
    // `encode_program_for(.., Compressed)` over the same parsed program.
    let src = COMPRESSED_PROGRAM;
    let rust = sadhana::parse::assemble_program(src).expect("parses");
    let want_rust = sadhana::encode::encode_program_for(&rust, sadhana::encode::Target::Compressed)
        .unwrap_or_else(|e| panic!("the Rust encoder encodes it: {e:?}"));
    let want_gnu: Vec<u8> = COMPRESSED_HALFWORDS
        .iter()
        .flat_map(|h| h.to_le_bytes())
        .collect();
    assert_eq!(want_rust, want_gnu, "the two oracles agree with each other");

    let mut it = load_encoder(&source("encode.t1"));
    let program = mirror_program(&mut it, src);
    let tgt = target(&it, "सङ्कुचितम्");
    let emitted = it
        .call("सङ्केतनॱलक्ष्यसङ्केतनम्", vec![program, tgt], 10_000_000_000)
        .expect("लक्ष्यसङ्केतनम् runs");
    let bytes: Vec<u8> = match emitted {
        Value::Octets(o) => o.as_slice().to_vec(),
        Value::Arena(a) => a
            .borrow()
            .iter()
            .map(|e| u8::try_from(e.as_int().expect("an octet")).expect("fits"))
            .collect(),
        other => panic!(
            "लक्ष्यसङ्केतनम् must answer a text section; it answered {other:?}. \
             शून्यम् here means it refused — E22 if the fixpoint did not settle, \
             E23 if layout and emission disagreed about an address; read \
             `अन्तिमसङ्केतनदोषः`"
        ),
    };
    assert_eq!(
        bytes.len(),
        12,
        "six compressed instructions are twelve octets; got {bytes:02x?}"
    );
    assert_eq!(
        bytes, want_gnu,
        "the T1 encoder must emit the octets riscv64-elf-as emits, low halfword \
         only for each compressed instruction"
    );
}

#[test]
fn a_wide_refusal_inside_the_chooser_leaves_no_diagnostic_behind() {
    // Rust's `encode_at(..).ok()?` (encode.rs:778) DISCARDS the refusal. In
    // this port the refusal is module state — `अन्तिमसङ्केतनदोषः` — and the
    // forward branch in `COMPRESSED_PROGRAM` refuses on round ० every time
    // (its label is not placed yet). If `सङ्कोचः` left that E05 in the slot,
    // a program that encoded clean would report a refusal.
    let mut it = load_encoder(&source("encode.t1"));
    let program = mirror_program(&mut it, COMPRESSED_PROGRAM);
    let tgt = target(&it, "सङ्कुचितम्");
    layout(&mut it, program, tgt);
    let err = it
        .global("अन्तिमसङ्केतनदोषः")
        .expect("अन्तिमसङ्केतनदोषः is a name in scope");
    let code = match err {
        Value::Record(r) => r
            .borrow()
            .get("सङ्केताङ्क")
            .map(|v| {
                v.octets()
                    .map(|o| o.as_slice().to_vec())
                    .unwrap_or_default()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    assert!(
        code.is_empty(),
        "the layout encoded clean, yet अन्तिमसङ्केतनदोषः carries {:?}: the wide \
         refusal inside सङ्कोचः was not discarded",
        String::from_utf8_lossy(&code)
    );
}

// ── mutations ────────────────────────────────────────────────────────────

#[test]
fn dropping_the_x0_guard_compresses_a_hint_into_a_different_instruction() {
    // The guard at encode.rs:806, inverted. `add zero, zero, a0` then finds
    // `c.add` with `arg1=arg0` holding (both are x0, both registers), and
    // encodes `c.add x0, a0` — which GNU accepts as a HINT and which is not
    // the operation written.
    let mutated = mutate(
        &source("encode.t1"),
        "यदि सङ्कोचनिषेधः विस्तृतसङ्केतः मूल्यानि समम् सत्यम् आदि",
        "यदि सङ्कोचनिषेधः विस्तृतसङ्केतः मूल्यानि समम् असत्यम् आदि",
    );
    let mut it = load_encoder(&mutated);
    let (t1, rust) = compressed_both(&mut it, "योगः शून्यःम् शून्यःन अर्थ०न ।");
    assert_eq!(rust, None);
    assert!(
        t1.is_some(),
        "with the guard inverted the hint must compress; if it still refuses \
         the guard is being applied somewhere else"
    );
}

#[test]
fn dropping_the_error_slot_restore_is_caught() {
    // The one line that makes `.ok()` true of this port. Without it the
    // forward branch's round-० refusal stays in `अन्तिमसङ्केतनदोषः`, and the
    // property `a_wide_refusal_inside_the_chooser_leaves_no_diagnostic_behind`
    // asserts fails here by name.
    let mutated = mutate(
        &source("encode.t1"),
        "अन्तिमसङ्केतनदोषः भवति पूर्वदोषः ।",
        "पूर्वदोषः भवति पूर्वदोषः ।",
    );
    let mut it = load_encoder(&mutated);
    let program = mirror_program(&mut it, COMPRESSED_PROGRAM);
    let tgt = target(&it, "सङ्कुचितम्");
    layout(&mut it, program, tgt);
    assert_eq!(
        last_code(&it),
        "E05",
        "with the restore dropped the branch's round-० refusal must be left \
         behind; anything else means the slot is restored somewhere else"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// N-001 — the signature table guard.
// ─────────────────────────────────────────────────────────────────────────

/// The class letters `प्रकारवर्गः` answers 1, 2 and 3 for.
const N001_CLASSES: [&str; 3] = ["\u{0905}", "\u{0928}", "\u{092A}"];

/// The text a type code stands for: class × 1024 + width, rendered back.
fn n001_type_text(code: i128) -> Option<String> {
    let class = usize::try_from(code / 1024).ok()?;
    let width = u64::try_from(code % 1024).ok()?;
    let letter = N001_CLASSES.get(class.checked_sub(1)?)?;
    Some(format!("{letter}{}", devanagari(width)))
}

/// What the pratyāhāra signatures assume of `spec/encodings-riscv64.tsv`, and
/// why the row builder can pack a count without saturating it: no row has
/// more than 63 slots of any kind a signature field counts, and every
/// conversion text round-trips through `प्रकारकूटः` (a non-zero code whose
/// rendering is the text). Answers one line per refusal.
fn n001_table_refusals(it: &mut Interpreter, table: &str) -> Vec<String> {
    let mut refused = Vec::new();
    for line in table.lines() {
        if line.starts_with('#') || line.starts_with("insn\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 9 {
            continue;
        }
        let kinds: Vec<&str> = if f[6] == "(none)" {
            Vec::new()
        } else {
            f[6].split('|')
                .map(|s| s.split(':').next().unwrap_or(""))
                .collect()
        };
        let count = |want: &[&str]| kinds.iter().filter(|k| want.contains(k)).count();
        for (what, n) in [
            ("register", count(&["reg", "freg", "vreg"])),
            ("vector register", count(&["vreg"])),
            ("displacement", count(&["disp"])),
            ("immediate", count(&["imm", "simm"])),
        ] {
            if n > 63 {
                refused.push(format!(
                    "{}: {n} {what} slots; a signature field holds 63",
                    f[0]
                ));
            }
        }
        if f[8] == "-" || f[8].is_empty() {
            continue;
        }
        for text in f[8].split(',') {
            let code = it
                .call("सङ्केतनॱप्रकारकूटः", vec![octets(text)], 1_000_000)
                .unwrap_or_else(|e| panic!("प्रकारकूटः runs on {text:?}: {e:?}"))
                .as_int()
                .expect("a code");
            if code == 0 || n001_type_text(code).as_deref() != Some(text) {
                refused.push(format!(
                    "{}: the conversion text {text} codes to {code}, which is not it",
                    f[0]
                ));
            }
        }
    }
    refused
}

/// The table as it stands passes the guard.
#[test]
fn n001_every_encoding_row_fits_the_signature_fields() {
    let mut it = load_encoder(&source("encode.t1"));
    let refused = n001_table_refusals(&mut it, &encodings_text());
    assert!(
        refused.is_empty(),
        "the table breaks the signature fields:\n  {}",
        refused.join("\n  ")
    );
}

/// CONTROL: a row with 64 register slots is refused.
#[test]
fn n001_the_guard_refuses_a_row_with_64_slots_of_one_kind() {
    let table = encodings_text();
    let row = table
        .lines()
        .find(|l| l.starts_with("add\t"))
        .expect("`add` is a row");
    let f: Vec<&str> = row.split('\t').collect();
    let many = vec!["reg:0x00000f80:0>7;1>8;2>9;3>10;4>11"; 64].join("|");
    let mutated_row = row.replacen(f[6], &many, 1);
    let mutated = table.replacen(row, &mutated_row, 1);
    let mut it = load_encoder(&source("encode.t1"));
    let refused = n001_table_refusals(&mut it, &mutated);
    assert!(
        refused.iter().any(|r| r.starts_with("add:")),
        "64 register slots on `add` were not refused: {refused:?}"
    );
}

/// CONTROL: conversion texts that do not round-trip are refused — a leading
/// `०`, four digits, and a letter that is no class.
#[test]
fn n001_the_guard_refuses_a_conversion_text_that_does_not_round_trip() {
    let table = encodings_text();
    for bad in ["प०६४", "प६४६४", "क६४"] {
        let mutated = table.replacen("\tप६४,अ६४\t", &format!("\t{bad},अ६४\t"), 1);
        assert_ne!(mutated, table, "the table has a `प६४,अ६४` pair to mutate");
        let mut it = load_encoder(&source("encode.t1"));
        let refused = n001_table_refusals(&mut it, &mutated);
        assert!(
            refused.iter().any(|r| r.contains(bad)),
            "`{bad}` was not refused: {refused:?}"
        );
    }
}
