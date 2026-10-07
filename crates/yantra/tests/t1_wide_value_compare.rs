//! **A VALUE AT OR ABOVE २^६३ IS NEVER COMPARED OR DIVIDED.**
//!
//! T1 integers are unsigned by contract and the reference interpreter computes
//! in `i128`, where `२^६४−१` is a large positive number. `ir.t1` lowers every
//! source comparison to the SIGNED kind and division to `भागः`, so on the
//! machine that same word is `−१`: `५ अधिकम् २^६४−१` is TRUE natively and
//! `(२^६४−१) ÷ १०` is `−१ ÷ १०` = ०. **Addition and subtraction are
//! bit-identical under both readings; only COMPARE and DIVIDE are
//! sign-sensitive**, which is why this guard names those two and lets the
//! others through.
//!
//! It has now cost the self-hosting fixpoint twice:
//!   * 2026-09-14, `अक्षरकोश`'s `अंशदोषः` — every multi-digit numeral in every
//!     source lowered to ० in the compiled compiler;
//!   * 2026-09-22, `वाक्यविभाग`'s `सङ्ख्यादोषः` (the twin guard, missed by that
//!     repair) — every numeral of two or more digits answered "overflow", so
//!     `॥ संरेखः १६ ॥` was refused in every module, the data section was never
//!     re-aligned after a section switch, and the native self-image build's
//!     image came out 1080 octets short with every module's TEXT identical.
//!
//! Both were fixed the same way: keep every compared or divided value below
//! २^६३, where the two readings agree. This guard is that rule, read from the
//! source: a name bound to a wide literal, then used as an operand of an
//! ordering comparison or a division inside the same routine.
//!
//! SINCE `W-381` STAGE 3 (owner ruling P1) THE TWO ENGINES NO LONGER PART HERE:
//! the interpreter reads every operand as its 64-bit word, signed for a
//! division and for a comparison with no unsigned NAME in it, and unsigned for
//! a comparison over a name declared unsigned, exactly as the native code
//! does (`w381_stage3_integer_semantics.rs`). The guard stays because the
//! shape it finds is still a likely mistake — a wide value meeting the SIGNED
//! reading — but a breach would now be wrong on both engines alike, not a
//! fixpoint break.

use std::path::Path;

/// `२^६३`. At or above this, a word reads negative to a signed lowering.
const WIDE: u128 = 1 << 63;

/// The operators whose lowering differs between the two readings.
const SIGN_SENSITIVE: &[&str] = &["अधिकम्", "न्यूनम्", "बृहत्समम्", "विभाजनम्", "शेषः"];

/// A Devanagari numeral's value, if the token is one.
fn numeral(tok: &str) -> Option<u128> {
    let mut v: u128 = 0;
    let mut any = false;
    for c in tok.chars() {
        let d = match c {
            '०'..='९' => c as u32 - '०' as u32,
            _ => return None,
        };
        any = true;
        v = v.checked_mul(10)?.checked_add(d as u128)?;
    }
    any.then_some(v)
}

/// `(routine, line, what)` for every wide value that reaches a sign-sensitive
/// operator: the literal on the spot, or a name bound to one in the same routine.
fn offending(src: &str) -> Vec<(String, usize, String)> {
    let mut found = Vec::new();
    let mut routine = String::from("<module scope>");
    let mut wide_names: Vec<String> = Vec::new();
    for (n, raw) in src.lines().enumerate() {
        let line = raw.split('॰').next().unwrap_or("");
        let head = line.trim_start();
        if let Some(rest) = head
            .strip_prefix("सार्वजनिक वृत्तिः ")
            .or_else(|| head.strip_prefix("वृत्तिः "))
        {
            routine = rest.split_whitespace().next().unwrap_or("").to_string();
            wide_names.clear();
        }
        let toks: Vec<&str> = line.split_whitespace().collect();
        // A binding of a wide literal: `चरः NAME ॱॱ TYPE भवति <numeral> ।`
        if let (Some(i), Some(j)) = (
            toks.iter().position(|t| *t == "भवति"),
            toks.iter().position(|t| *t == "चरः"),
        ) {
            let wide = toks
                .get(i + 1)
                .and_then(|t| numeral(t))
                .is_some_and(|v| v >= WIDE);
            if let (true, Some(name)) = (wide, toks.get(j + 1)) {
                wide_names.push((*name).to_string());
            }
        }
        // A sign-sensitive operator ANYWHERE on a line that carries a wide
        // value. NOT an adjacency test: the corpus nests its operands
        // (`पदम् अधिकम् आरभ्य आरभ्य चरमम् वियोगः अङ्कमानम् समाप्तम् विभाजनम् मूलम् समाप्तम्`),
        // so the wide name and the operator are ten tokens apart — an
        // adjacency scan passed that line, which is the very site this guard
        // exists for.
        if toks.iter().any(|t| SIGN_SENSITIVE.contains(t)) {
            for t in &toks {
                let wide_literal = numeral(t).is_some_and(|v| v >= WIDE);
                let wide_name = wide_names.iter().any(|w| w == t);
                if wide_literal || wide_name {
                    found.push((routine.clone(), n + 1, (*t).to_string()));
                }
            }
        }
    }
    found
}

#[test]
fn no_source_compares_or_divides_a_value_at_or_above_two_to_the_sixty_third() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src");
    let mut bad = Vec::new();
    let mut scanned = 0;
    for e in std::fs::read_dir(&dir).expect("the corpus is readable") {
        let p = e.expect("an entry").path();
        if p.extension().is_some_and(|x| x == "t1") {
            scanned += 1;
            let file = p.file_name().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&p).expect("a source");
            for (routine, line, what) in offending(&src) {
                bad.push(format!(
                    "{file}:{line}: `{routine}` compares or divides `{what}`"
                ));
            }
        }
    }
    assert!(
        scanned >= 20,
        "scanned only {scanned} sources — the scan has gone blind"
    );
    assert!(
        bad.is_empty(),
        "a value at or above २^६३ reads NEGATIVE to the signed lowering, so the \
         compiled compiler answers differently from the interpreter. Keep the \
         compared or divided value below २^६३ (a per-base threshold, as \
         `सङ्ख्यादोषः` and `अंशदोषः` do):\n{}",
        bad.join("\n")
    );
}

/// The scan must see the shape it forbids, and must let the safe ones through.
#[test]
fn the_scan_sees_the_shape_it_forbids() {
    let src = "मण्डलम् क ॥\n\
               वृत्तिः दोषः आदाय क ॱॱ अ६४ ददाति अ६४ आदि\n\
               \x20   चरः चरमम् ॱॱ अ६४ भवति १८४४६७४४०७३७०९५५१६१५ ।\n\
               \x20   यदि क अधिकम् आरभ्य आरभ्य चरमम् वियोगः क समाप्तम् विभाजनम् १० समाप्तम् आदि\n\
               \x20       प्रत्यागमनम् २ ।\n\
               \x20   इति\n\
               \x20   प्रत्यागमनम् चरमम् वियोगः क ।\n\
               इति\n\
               वृत्तिः सुरक्षितः आदाय क ॱॱ अ६४ ददाति अ६४ आदि\n\
               \x20   चरः सीमा ॱॱ अ६४ भवति १८४४६७४४०७३७०९५५१६१ ।\n\
               \x20   यदि क अधिकम् सीमा आदि\n\
               \x20       प्रत्यागमनम् २ ।\n\
               \x20   इति\n\
               \x20   प्रत्यागमनम् क विभाजनम् १० ।\n\
               इति\n";
    let f = offending(src);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].0, "दोषः", "the wide comparison is the one flagged");
    assert_eq!(f[0].2, "चरमम्");
    // Not flagged: the SUBTRACTION of the same wide name (bit-identical under
    // both readings), a threshold below २^६३, and a division by a small number.
}
