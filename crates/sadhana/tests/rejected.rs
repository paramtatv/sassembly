//! Programs that must not assemble — task B-044.
//!
//! Every other corpus checks that right programs produce right bytes. This
//! checks that wrong programs produce an error, and that the error names the
//! problem.
//!
//! It exists because the defects this assembler has actually shipped were not
//! crashes. `slli` encoded to a valid instruction with the wrong register and
//! the wrong shift. Every load encoded to a valid `lb`. A store written with
//! the locus sigil OR-ed two register numbers together and emitted a valid
//! store of register 7. In each case every individual step succeeded — which
//! is why a corpus of *rejections* is a different instrument from a corpus of
//! *comparisons*, not a weaker one.
//!
//! The expectations here are ours, unavoidably: GNU `as` has no opinion about
//! a kāraka sigil, so there is no external oracle for a diagnostic. What the
//! file can still enforce is that the message names the cause rather than
//! merely existing.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

struct Case {
    source: String,
    expect: String,
    why: String,
}

fn cases() -> Vec<Case> {
    std::fs::read_to_string(root().join("spec/rejected-t0.tsv"))
        .expect("read spec/rejected-t0.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("sassembly\t") && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 3, "malformed rejection case: {l:?}");
            Case {
                source: f[0].into(),
                expect: f[1].into(),
                why: f[2].into(),
            }
        })
        .collect()
}

/// Every message produced, and every diagnostic code, from parsing or encoding.
///
/// The codes matter as much as the text. An expectation written as a code —
/// `E17` — is checked against the code, and one written as words is checked
/// against the message. `B-078` moved these into `spec/diagnostics.tsv` so the
/// wording could change, and this corpus has needed rewording three times
/// since; a code does not move when a sentence is improved or translated.
fn errors(source: &str) -> (Vec<String>, Vec<String>) {
    // Parse errors are read structurally so their codes count too. Going
    // through `assemble_source`, which renders to strings, threw the codes away
    // and left a parse expectation with nothing to match but prose.
    let tokens = match sadhana::lex::lex(source) {
        Ok(t) => t,
        Err(e) => return (e.iter().map(ToString::to_string).collect(), Vec::new()),
    };
    match sadhana::parse::parse(&tokens) {
        Err(e) => (
            e.iter().map(ToString::to_string).collect(),
            e.iter()
                .filter(|x| !x.code.is_empty())
                .map(|x| x.code.to_string())
                .collect(),
        ),
        Ok(program) => {
            let mut msgs = Vec::new();
            let mut codes = Vec::new();
            for i in &program.instructions {
                if let Err(e) = sadhana::encode::encode(i) {
                    if !e.code.is_empty() {
                        codes.push(e.code.to_string());
                    }
                    msgs.push(e.reason);
                }
            }
            (msgs, codes)
        }
    }
}

/// Whether an expectation names a diagnostic rather than a phrase.
fn is_code(expect: &str) -> bool {
    let mut c = expect.chars();
    c.next().is_some_and(|f| f.is_ascii_uppercase())
        && expect.len() >= 2
        && expect[1..].chars().all(|d| d.is_ascii_digit())
}

#[test]
fn every_rejected_program_is_rejected() {
    let mut wrong = Vec::new();
    for c in cases() {
        let (msgs, codes) = errors(&c.source);
        let matched = if is_code(&c.expect) {
            codes.contains(&c.expect)
        } else {
            msgs.iter().any(|m| m.contains(&c.expect))
        };
        if msgs.is_empty() {
            wrong.push(format!("{} — ASSEMBLED, but {}", c.source, c.why));
        } else if !matched {
            wrong.push(format!(
                "{} — rejected, but no message contains {:?}:\n      {}",
                c.source,
                c.expect,
                msgs.join("\n      ")
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} rejection case(s) wrong:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
    println!("METRIC t0_rejection_cases {}", cases().len());
}

#[test]
fn the_store_that_started_this_no_longer_encodes() {
    // Kept apart from the table because the exact number matters. This wrote
    // 0x00703423 — `sd t2, 8(zero)` — from a program naming t0 and sp. Neither
    // register in the output was in the source.
    let src = "निधानम् क्षणिक०न स्तूपसूचकःए ८न ।";
    let parsed = sadhana::parse::assemble_source(src).expect("it parses; the grammar allows ए");
    let err = sadhana::encode::encode(&parsed[0]).expect_err("must not encode");
    assert!(
        err.reason.contains("अधिकरण"),
        "the message should name the sigil, got: {}",
        err.reason
    );
}
