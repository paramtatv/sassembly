//! **`अक्षरकोशॱपरिधिस्थम्` MUST ANSWER EXACTLY WHAT `in_repertoire` ANSWERS.**
//!
//! `W-304` found the two engines accepting different source languages: a program
//! whose literal is `उक्तम् SAS इति` is compiled and run by the `.t1` path and
//! REFUSED by the Rust loader with *"`S` is outside the doc 15 repertoire"*. The
//! asymmetry was never a permissive arm in the `.t1` lexer — it was that
//! `repertoire.rs`'s **pass 0**, which "runs BEFORE normalization and BEFORE the
//! lexer", had no counterpart on the `.t1` side at all.
//!
//! `sanskrit_text.t1`'s `परिधिस्थम्` is that counterpart's predicate. This file is
//! the reason to believe it: the port is graded against the ORIGINAL, one code
//! point at a time, rather than against a restatement of its own ranges.
//!
//! # Why a twin and not a table of expectations
//!
//! A table written here would be a second copy of the four ranges, and two copies
//! by one author agree by construction. `repertoire_census.rs` states the rule
//! this file obeys: **"A model of the product that only ever agrees with itself is
//! not evidence."** So Rust's `in_repertoire` is the oracle, the `.t1` routine is
//! the subject, and every disagreement is named with its code point.
//!
//! # What the cases cover
//!
//! Boundaries first, because an off-by-one in a bound is the whole failure mode
//! this can have: for each of the four ranges, `lo-1`, `lo`, `lo+1`, `hi-1`, `hi`
//! and `hi+1`. `अधिकसमम्` does not occur in this corpus — `nirvahana.rs:47`
//! counts the operator set and records it — so the `.t1` side writes
//! `०x०९००..=०x०९७F` as `अधिकम् २३०३` and `न्यूनम् २४३२`, and those two constants
//! are exactly where a slip would live.
//!
//! Then the three layout characters and the two joiners with their neighbours,
//! then the ASCII letters — `S` among them, the character that opened `W-304` —
//! and then a stride across the first 0x12000 code points for breadth.

use std::path::{Path, PathBuf};

use sadhana::lex::{LexError, lex, lex_t1};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// THE WHOLE CORPUS, because `sanskrit_text.t1` imports `सङ्केतन` and
/// `पदविभाग` and a module cannot be loaded without what it names.
fn interp() -> Interpreter {
    let dir = root().join("crates/sadhana-t1/src");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("t1"))
        .collect();
    paths.sort();
    assert!(
        paths.len() >= 20,
        "expected the whole .t1 corpus, found {} source(s)",
        paths.len()
    );
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    Interpreter::load_paths(&refs, &root().join("spec"))
        .unwrap_or_else(|e| panic!("the corpus loads: {e:?}"))
}

/// What the `.t1` routine says. A non-`Bool` answer is a failure of its OWN
/// contract and is reported as one rather than coerced.
fn t1_says(it: &mut Interpreter, cp: u32) -> bool {
    match it.call(
        "अक्षरकोशॱपरिधिस्थम्",
        vec![Value::Int(i128::from(cp))],
        600_000_000,
    ) {
        Ok(Value::Bool(b)) => b,
        Ok(other) => panic!("परिधिस्थम् U+{cp:04X} answered {other:?}, not a बूल"),
        Err(e) => panic!("परिधिस्थम् U+{cp:04X} did not run: {e:?}"),
    }
}

const RANGES: [(u32, u32); 4] = [
    (0x0900, 0x097F),
    (0xA8E0, 0xA8FF),
    (0x1CD0, 0x1CFF),
    (0x11B00, 0x11B5F),
];

#[test]
fn the_t1_repertoire_gate_answers_exactly_what_rust_answers() {
    let mut cases: Vec<u32> = Vec::new();

    // EVERY ADMITTED CODE POINT, NOT A SAMPLE OF THEM. The four ranges total 304
    // points — 128 + 32 + 48 + 96 — which is small enough to walk exhaustively,
    // so "the .t1 side admits exactly this set" is checked rather than sampled.
    // The first version strode the space by 97 and landed only 24 admitted points
    // out of 831; a stride cannot cover a range 32 wide, and the vacuity guard
    // below caught that before any verdict was read.
    for (lo, hi) in RANGES {
        cases.extend(lo - 1..=hi + 1);
    }
    // The layout characters and joiners, each with a neighbour on both sides.
    cases.extend([
        0x08, 0x09, 0x0A, 0x0B, 0x1F, 0x20, 0x21, 0x200B, 0x200C, 0x200D, 0x200E,
    ]);
    // ASCII letters — `S` is the character `W-304` was opened on — plus the
    // punctuation a path literal would need.
    cases.extend(u32::from(b'A')..=u32::from(b'Z'));
    cases.extend(u32::from(b'a')..=u32::from(b'z'));
    cases.extend([
        u32::from(b'0'),
        u32::from(b'9'),
        u32::from(b'/'),
        u32::from(b'.'),
        u32::from(b'-'),
        u32::from(b'_'),
    ]);
    // Breadth, so a range the ranges do not mention cannot pass unexamined.
    cases.extend((0..0x1_2000_u32).step_by(97));
    cases.sort_unstable();
    cases.dedup();

    let mut it = interp();
    let mut disagreements: Vec<String> = Vec::new();
    let mut compared = 0usize;
    let mut admitted = 0usize;

    for cp in &cases {
        let Some(ch) = char::from_u32(*cp) else {
            continue;
        };
        let rust = sanskrit_text::in_repertoire(ch);
        let t1 = t1_says(&mut it, *cp);
        compared += 1;
        if rust {
            admitted += 1;
        }
        if rust != t1 {
            disagreements.push(format!("U+{cp:04X}: rust says {rust}, .t1 says {t1}"));
        }
    }

    // NOT VACUOUS, and said out loud: a comparison that ran over nothing, or over
    // only rejections, would pass while proving neither range.
    assert!(
        compared > 300,
        "only {compared} code point(s) compared — the case set collapsed"
    );
    assert!(
        admitted > 100,
        "only {admitted} of {compared} were ADMITTED — a gate that refuses \
         everything agrees with nothing"
    );
    assert!(
        disagreements.is_empty(),
        "{} of {compared} code point(s) disagree between `in_repertoire` and \
         `अक्षरकोशॱपरिधिस्थम्`:\n  {}",
        disagreements.len(),
        disagreements.join("\n  ")
    );
    println!(
        "METRIC t1_repertoire_twin_compared {compared}\n\
         METRIC t1_repertoire_twin_admitted {admitted}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// THE WALK, not just the predicate — `W-304`'s second half.
// ─────────────────────────────────────────────────────────────────────────────

/// The real lexer's repertoire verdict, filtered exactly as
/// `repertoire_text_directive_boundary.rs` filters it: a source may fail to lex
/// for reasons that have nothing to do with R-15-1, and counting those here
/// would make this file red on an unrelated defect while saying "repertoire".
fn rust_violation(path: &str, src: &str) -> Option<LexError> {
    let result = if path.ends_with(".t1") {
        lex_t1(src)
    } else {
        lex(src)
    };
    result
        .err()
        .unwrap_or_default()
        .into_iter()
        .find(|e| e.reason.contains("outside the doc 15 repertoire"))
}

/// `परिधिदोषः` answers the offset of the first inadmissible code point, or the
/// source length when clean. `None` here means clean.
fn t1_violation(it: &mut Interpreter, src: &str) -> Option<usize> {
    let answer = it
        .call(
            "अक्षरकोशॱपरिधिदोषः",
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            600_000_000,
        )
        .unwrap_or_else(|e| panic!("परिधिदोषः did not run: {e:?}"));
    let n = match answer {
        Value::Int(n) => usize::try_from(n).expect("an offset is not negative"),
        other => panic!("परिधिदोषः answered {other:?}, not an offset"),
    };
    if n >= src.len() { None } else { Some(n) }
}

/// **THE TWO ENGINES MUST AGREE ON WHETHER A SOURCE IS ADMISSIBLE** — and the
/// exemption is the whole difficulty. Porting `repertoire::check` instead of the
/// lexer's pass would refuse the 1,724 Latin characters standing legally inside
/// `॥ आस्की … ॥` in ten `spec/*.sas` files, which `repertoire_census.rs` calls
/// "not a defect TODAY". So the corpus AND those programs are both walked.
#[test]
fn the_t1_walk_admits_and_refuses_what_the_lexer_does() {
    let root = root();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for (sub, ext) in [("crates/sadhana-t1/src", "t1"), ("spec", "sas")] {
        if let Ok(entries) = std::fs::read_dir(root.join(sub)) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some(ext) {
                    files.push(p);
                }
            }
        }
    }
    files.sort();
    assert!(
        files.len() > 25,
        "only {} source(s) found — the walk reached nothing",
        files.len()
    );

    let mut it = interp();
    let mut disagreements: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let mut with_directive = 0usize;

    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        let name = f.to_string_lossy().to_string();
        if src.contains("आस्की") || src.contains("जाल") {
            with_directive += 1;
        }
        let rust = rust_violation(&name, &src);
        let t1 = t1_violation(&mut it, &src);
        checked += 1;
        match (&rust, &t1) {
            (None, None) => {}
            (Some(e), Some(_)) => {
                // Both refuse. The offsets need not match — the lexer reports the
                // WORD's byte and this reports the CODE POINT's — so agreement is
                // on the verdict, and the character is named for the reader.
                let _ = e;
            }
            (Some(e), None) => disagreements.push(format!(
                "{}: the lexer refuses `{}` at line {} and the .t1 walk admits it",
                f.file_name().unwrap_or_default().to_string_lossy(),
                e.aksara,
                e.line
            )),
            (None, Some(at)) => disagreements.push(format!(
                "{}: the .t1 walk refuses offset {at} and the lexer admits the file",
                f.file_name().unwrap_or_default().to_string_lossy()
            )),
        }
    }

    assert!(
        with_directive > 0,
        "no source carried आस्की or जाल — the exemption went untested, which is \
         the half that cannot be got right by accident"
    );
    assert!(
        disagreements.is_empty(),
        "{} of {checked} source(s) disagree ({with_directive} carry a text \
         directive):\n  {}",
        disagreements.len(),
        disagreements.join("\n  ")
    );
    println!("METRIC t1_repertoire_walk_sources {checked}");
    println!("METRIC t1_repertoire_walk_with_directive {with_directive}");
}

/// **THE CASES THAT MUST STILL BE REFUSED**, because a walk that admits
/// everything agrees with the lexer on every clean file and is still wrong.
#[test]
fn the_walk_refuses_outside_a_directive_and_narrows_inside_jal() {
    let mut it = interp();

    // `W-304`'s own case: a Latin literal outside any directive.
    let bad = "मण्डलम् ठ ॥\nसार्वजनिक वृत्तिः कृ ददाति न६४ आदि\n    प्रत्यागमनम् उक्तम् SAS इति ।\nइति\n";
    assert!(
        t1_violation(&mut it, bad).is_some(),
        "`उक्तम् SAS इति` outside a directive must be refused — it is the case \
         W-304 was opened on"
    );

    // Inside `आस्की`, anything goes until the `॥`.
    let ascii_ok = "॥ आस्की BOOT-COUNTER-FRESH ॥\n";
    assert_eq!(
        t1_violation(&mut it, ascii_ok),
        None,
        "an आस्की directive admits ASCII — ten spec/*.sas files depend on it"
    );

    // AND THE DIRECTIVE CLOSES. The same word after the `॥` must be refused
    // again, or the flag leaks and one directive disarms the rest of the file.
    let leaks = "॥ आस्की BOOT ॥\nSAS\n";
    assert!(
        t1_violation(&mut it, leaks).is_some(),
        "the आस्की flag must CLEAR at ॥ — otherwise one directive disarms the \
         whole file below it"
    );

    // `जाल` NARROWS: its twelve marks and the letters pass.
    let jal_ok = "॥ जाल VIRTIO-NET-OK ॥\n";
    assert_eq!(
        t1_violation(&mut it, jal_ok),
        None,
        "a जाल directive admits its named marks and the ASCII letters"
    );

    // But `जाल` is NOT `आस्की`: a character outside its named list is still
    // refused, which is what makes it an exemption and not a hole. `+` (U+002B)
    // is in neither the repertoire nor जाल's twelve.
    let jal_no = "॥ जाल a+b ॥\n";
    assert!(
        t1_violation(&mut it, jal_no).is_some(),
        "जाल narrows the repertoire rather than disabling it — `+` is in neither \
         the repertoire nor its twelve marks"
    );
}
