//! `.सस` is a different language from `.sas`, and T0 cannot read it — task `B-036c`.
//!
//! # Why this test is in SANSOS and not in the tool that does the excluding
//!
//! FastBuild's atlas maps a file extension to a grammar. It maps `.sas` to the vendored
//! T0 grammar and maps `.सस` to **nothing**, and `fb-atlas`'s own
//! `unsupported_extensions_are_none` asserts that. So the exclusion is already tested —
//! **in another repository, on an unpushed branch.** `B-036c` records exactly that: the
//! property holds, SANSOS cannot mark it done, and one edit closes it once pushed.
//!
//! What SANSOS *can* own is the fact underneath the exclusion. FastBuild excludes `.सस`
//! because T0 cannot parse it; if that ever stopped being true, the exclusion would be
//! wrong and nobody would notice. This asserts the fact, in the repository that owns the
//! language, where it survives a rebuild of the tool.
//!
//! # The failure this exists to prevent
//!
//! Pointing T0 at T1 **does not fail loudly** in a tolerant parser. Measured on this tree:
//! `tree-sitter parse` gives a four-line directive node swallowing a whole function body,
//! and `fb-atlas`, being deliberately error-tolerant, would store **60 symbols across the
//! 40 files, every one wrong** — `फलम्` from `चरः फलम्ॱॱ न६४` filed as a Function, because
//! T1 reuses `ॱॱ` for variable and field names and the T0 label rule matches them.
//!
//! So the one-line extension mapping is the obvious reach for the next reader, and it is
//! the wrong reach: sixty plausible answers beat zero honest ones only if nobody reads
//! them. This test is the thing that argues back.
//!
//! The assembler is a stricter oracle than the grammar and disagrees with it usefully:
//! where tree-sitter yields a confident wrong tree, `sadhana` refuses outright. That is
//! why the assertion is written against the assembler.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// Every `.सस` in the tree, recursively, skipping build output.
fn t1_sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target" || n == ".git") {
                    continue;
                }
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "सस") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&root(), &mut v);
    v.sort();
    v
}

#[test]
#[ignore = "needs tests/corpus/t1 (the development corpus) not in the public repository"]
fn the_t1_corpus_exists_and_this_test_is_not_vacuous() {
    // Without this, deleting or renaming the corpus would make every assertion below pass
    // by having nothing to assert over — and a green suite would then be evidence FOR a
    // mapping that the corpus exists to argue against.
    let files = t1_sources();
    println!("METRIC sadhana_t1_files {}", files.len());
    assert!(
        files.len() >= 30,
        "only {} .सस files found; the T1 corpus is the evidence this test rests on",
        files.len()
    );
}

#[test]
fn t0_refuses_every_t1_source_and_none_slips_through() {
    // The load-bearing assertion. If ANY `.सस` file assembled as T0, the extension would
    // be partially readable, and "maps to no language" would stop being the honest answer
    // for that file — which is precisely the case a per-extension mapping would seize on.
    let files = t1_sources();
    // Non-emptiness is asserted HERE and not left to the sibling test. A mutation that
    // emptied the corpus killed that sibling and left THIS test green — it iterated
    // nothing and concluded nothing was accepted, which is true and worthless. A test
    // that survives the disappearance of its own evidence is the defect this row is
    // about, one level up.
    assert!(
        !files.is_empty(),
        "no .सस sources found — this test proves nothing without them"
    );
    let mut accepted = Vec::new();
    for f in &files {
        let text =
            std::fs::read_to_string(f).unwrap_or_else(|e| panic!("read {}: {e}", f.display()));
        if sadhana::parse::assemble_program(&text).is_ok() {
            accepted.push(f.display().to_string());
        }
    }
    println!(
        "METRIC sadhana_t1_rejected {}",
        files.len() - accepted.len()
    );
    println!("METRIC sadhana_t1_accepted {}", accepted.len());
    assert!(
        accepted.is_empty(),
        "T0 accepted {} of {} T1 sources — the `.सस` exclusion in fb-atlas rests on this \
         never happening:\n  {}",
        accepted.len(),
        files.len(),
        accepted.join("\n  ")
    );
}

#[test]
fn t0_still_reads_its_own_language_so_the_refusal_means_something() {
    // The control. A parser that refused EVERYTHING would satisfy the test above while
    // being broken, and the exclusion would then be resting on a bug rather than on a
    // language boundary. `spec/namaste.sas` is T0 and must still assemble.
    let p = root().join("spec/namaste.sas");
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
    assert!(
        sadhana::parse::assemble_program(&text).is_ok(),
        "T0 cannot read its own language, so its refusal of T1 proves nothing"
    );
}

#[test]
fn the_refusal_names_a_t1_construct_rather_than_failing_generically() {
    // *Why* it refuses matters. A generic "parse error" would leave the next reader
    // guessing whether the file is T1 or simply malformed. The diagnostic should name a
    // word that is T1 vocabulary — `ॐ`, `मण्डलम्`, `इति`, `न६४` — which is what tells a
    // reader these are two languages rather than one language and a typo.
    let files = t1_sources();
    let f = files
        .first()
        .expect("the corpus is non-empty, per the test above");
    let text = std::fs::read_to_string(f).expect("read");
    let errs = sadhana::parse::assemble_program(&text).expect_err("T1 must not assemble");
    let joined = errs.join("\n");
    assert!(
        ["ॐ", "मण्डलम्", "इति", "न६४"]
            .iter()
            .any(|w| joined.contains(w)),
        "the refusal should name a T1 construct so the reason is legible, got:\n{joined}"
    );
}
