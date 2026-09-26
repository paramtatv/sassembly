//! **No global may be initialised from a non-empty string literal.**
//!
//! The native lowering drops a global's literal initializer. `riscv64::emit_data`
//! (and its `.t1` twin) carry a global's initializer as an INTEGER; a run-typed
//! global gets zeroed storage and a length word of ०. So a global declared
//! `भवति उक्तम् … इति` is the literal to the interpreter and the EMPTY run in
//! the image — and nothing says so. The interpreter was taught to evaluate such
//! initializers on 2026-09-04 (W-242); the native side never was.
//!
//! It cost the self-hosting fixpoint a day. On 2026-09-21 the allocator's two
//! symbol names (`यन्त्ररचनासूचकनाम`, `यन्त्ररचनाक्षेत्रनाम`) were globals of this
//! shape. Natively both were empty, every inline allocation the compiled
//! compiler emitted named `ॱउपरि` / `ॱअधः` with no symbol, and a diff of the
//! text each engine generated for `artha.t1` parted at line 2155; the native
//! self-image build lost 580 instructions of that one module. `इतिशब्दः` had
//! the same shape and was empty natively too, unseen only because
//! compiler-emitted Sassembly never reaches the code that reads it.
//!
//! The honoured form is a zero-argument routine returning the literal: an
//! inline literal works in both engines, and a bare name of the routine is a
//! call, so a use site does not change.
//!
//! Until the native lowering materialises literal initializers — the real fix,
//! in both emitters — this test is what keeps the shape from coming back.

use std::path::Path;

/// Every top-level global whose initializer is a NON-EMPTY string literal.
/// An EMPTY literal (`उक्तम् इति`) is allowed: the image's empty run is its value.
fn offending(src: &str) -> Vec<String> {
    src.lines()
        .filter(|l| l.starts_with("चरः ") || l.starts_with("सार्वजनिक चरः "))
        .filter_map(|l| {
            let (_, init) = l.split_once(" भवति ")?;
            let lit = init.strip_prefix("उक्तम् ")?;
            // `उक्तम् इति …` — the literal is empty when its first word closes it.
            if lit.starts_with("इति ") && !lit.starts_with("इति इति") {
                return None;
            }
            Some(l.trim().to_string())
        })
        .collect()
}

#[test]
fn no_global_is_initialised_from_a_non_empty_literal() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    let mut scanned = 0;
    for e in std::fs::read_dir(&dir).expect("the corpus is readable") {
        let p = e.expect("an entry").path();
        if p.extension().is_some_and(|x| x == "t1") {
            scanned += 1;
            let src = std::fs::read_to_string(&p).expect("a source");
            for l in offending(&src) {
                found.push(format!("{}: {l}", p.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    assert!(
        scanned >= 20,
        "scanned only {scanned} sources — the scan has gone blind"
    );
    assert!(
        found.is_empty(),
        "a global initialised from a non-empty literal is the EMPTY run in the image \
         (the native lowering drops the initializer). Make it a zero-argument routine \
         returning the literal:\n{}",
        found.join("\n")
    );
}

/// The scan must be able to see the shape it forbids — and must let the one
/// harmless form (an EMPTY literal) through.
#[test]
fn the_scan_sees_the_shape_it_forbids() {
    let src = "मण्डलम् क ॥\n\
               सार्वजनिक चरः नाम ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् रचनासूचकः इति ।\n\
               चरः शब्दः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् इति इति इति ।\n\
               चरः रिक्तम् ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् इति ।\n\
               सार्वजनिक चरः संख्या ॱॱ न६४ भवति ७ ।\n\
               \x20   चरः स्थानीयम् ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् क इति ।\n";
    let f = offending(src);
    assert_eq!(f.len(), 2, "{f:?}");
    assert!(f[0].contains("रचनासूचकः"), "a public global with a literal");
    assert!(
        f[1].contains("इति इति इति"),
        "the self-quoting closing word is NOT empty"
    );
    // Not flagged: the empty literal, a numeric initializer, and an indented
    // LOCAL (inside a routine), which both engines honour.
}
