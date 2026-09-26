//! ॥ A BARE CROSS-MODULE RECORD DECLARATION AND THE TIE THAT STILL HAS NO
//! STORAGE ॥ `W-279`.
//!
//! UNTIL 2026-09-19 `ir.t1` allocated a record for `चरः x ॱॱ T भवति ०` in
//! exactly two cases: T declared in the SAME module, or T carrying a module
//! prefix (`सङ्केतनॱअवकाश`). The remote path was gated on `यदि रचनोपसर्गः ॱ
//! दैर्घ्य अधिकम् ० आदि` — a MODULE PREFIX — so a BARE name that resolved to
//! another module fell through to `चरमूल्यम् भवति ध्रुवरचना ०`, the slot held ०,
//! and the first field access faulted at address ०.
//!
//! ## WHAT CHANGED, AND WHY THIS FILE STILL HAS A RATCHET
//!
//! `ir.t1`'s `आयातितसंरचनासंख्या` now resolves a bare type through the current
//! module's IMPORTS and allocates for it — pinned by
//! `tests/w279_bare_type_lowering.rs`, which asks what the compiler LOWERS
//! where this file asks what the corpus is SPELLED LIKE. So the first count
//! below is no longer a correctness claim.
//!
//! **THE TIE IS.** `आज्ञा` is declared public by both `मध्यरूप` and `वाक्यविभाग`,
//! and `वाक्य` by both `वास्तु` and `वाक्यविभाग`. Two candidates give two field
//! counts; guessing would hand back a wrong SIZE, which corrupts silently where
//! the null at least faults. So a tie is REFUSED, lowers to ० exactly as
//! before, and faults natively at its first field access — and it is the
//! population this file must now ratchet.
//!
//! The census reported the two on ONE line for weeks (`of which ambiguous`)
//! and that was false — an ambiguous row goes to the other list and was never
//! counted in the first number, so a corpus with five ties printed `bare ...:
//! 0`. They are two lines now, and two tests.
//!
//! BOTH SHAPES HAVE BEEN SEEN NATIVELY, and they do not look alike in a
//! disassembly, which cost a wrong verdict once:
//!
//! ```text
//!   a field WRITE   ld base, N(sp) ; addi base, base, off ; sd val, 0(base)
//!   a field READ    ld ref,  N(sp) ; ld ref, 0(ref)
//! ```
//!
//! ## WHY THE UNAMBIGUOUS RATCHET IS KEPT ANYWAY
//!
//! Eight sites were qualified by hand on 2026-09-18 and that emptied the
//! population; the compiler was repaired the day after. Keeping the ratchet at
//! ० is therefore a LEGIBILITY rule and is labelled as one rather than left to
//! read as a correctness rule it has stopped being: every cross-module record
//! in this corpus is spelled with its module, a bare one costs a walk of the
//! declaration store at every lowering, and a reader cannot tell from the line
//! which module the type came from. Relaxing it is a decision, not a fix —
//! make it deliberately and say so here.
//!
//! ## WHY THE GUARD CONTAINS ITS OWN POSITIVE CONTROL
//!
//! A census that answers ० because its matcher is broken is indistinguishable
//! from one that answers ० because the corpus is clean, and this particular
//! matcher was WRONG TWICE before it was right:
//!
//!   ॱ keyed `type -> file`, so a name declared in two modules overwrote and six
//!     `चरः नव ॱॱ आज्ञा` sites INSIDE ir.t1 — which declare ir.t1's own आज्ञा and
//!     are correct — were reported as cross-module, because वाक्यविभाग also
//!     declares आज्ञा and sorted later. Ten reported, six of them phantom.
//!   ॱ ignored the initialiser, and `चरः अवकाशः ॱॱ अवकाश भवति पङ्क्त्यवकाशः ...`
//!     is a record obtained from a CALL, which lowers correctly as the
//!     expression it is. Only the numeral `०` means allocate — ir.t1's own
//!     margin says so, and says why: allocating on an expression would discard
//!     the value and hand back fresh zeroed storage, a silent wrong answer
//!     rather than a fault.
//!
//! So the guard breaks an isolated COPY of the corpus and requires the census to
//! find the break. A green here means the matcher demonstrably still matches.

use std::path::{Path, PathBuf};
use std::process::Command;

const CENSUS: &str = "tools/bare-type-census.py";
const SRC: &str = "crates/sadhana-t1/src";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("crates/sadhana-t1 has a grandparent")
}

/// One of the census's two summary counts, by its exact label.
///
/// TAKEN BY LABEL AND NOT BY POSITION, because the two lines are adjacent and
/// mean different things — one is a legibility report and the other is the
/// population that still faults. A reader that took "the second number" would
/// swap them silently the next time a line is added.
fn count_labelled(report: &str, label: &str) -> usize {
    report
        .lines()
        .find_map(|l| l.trim().strip_prefix(label))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or_else(|| panic!("the census prints `{label}`; it printed:\n{report}"))
}

const BARE: &str = "bare cross-module declarations:";
const AMBIGUOUS: &str = "ambiguous bare cross-module declarations:";

/// Run the census over `srcdir`; answer `(bare count, full report)`.
fn census(srcdir: &Path) -> (usize, String) {
    let out = Command::new("python3")
        .arg(root().join(CENSUS))
        .arg(srcdir)
        .current_dir(root())
        .output()
        .expect("python3 runs — the project's own generators need it (tools/gen-*.py)");
    let report = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "the census exits 0 (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );
    let n = count_labelled(&report, BARE);
    (n, report)
}

/// THE POSITIVE CONTROL, and it runs FIRST — an assertion about the corpus made
/// by an instrument that has not been shown to work is not evidence.
///
/// `vishlesana.t1` declares `चरः नव ॱॱ सङ्केतनॱअंशयुग्म भवति ० ।`: a record from
/// another module, QUALIFIED, and so correct. Strip the prefix on a copy and the
/// census must name that exact site. If it does not, the matcher has stopped
/// matching and the ० in the test below means nothing.
#[test]
fn the_census_finds_a_bare_declaration_when_one_is_planted() {
    let scratch = std::env::temp_dir().join(format!("w279-control-{}", std::process::id()));
    let copy = scratch.join("src");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&copy).expect("a temp dir");

    let from = root().join(SRC);
    for entry in std::fs::read_dir(&from).expect("the corpus is readable") {
        let p = entry.expect("a dir entry").path();
        if p.extension().is_some_and(|e| e == "t1") {
            std::fs::copy(&p, copy.join(p.file_name().expect("a file name"))).expect("copied");
        }
    }

    let (clean, _) = census(&copy);
    assert_eq!(clean, 0, "the untouched copy matches the corpus");

    let victim = copy.join("vishlesana.t1");
    let text = std::fs::read_to_string(&victim).expect("readable");
    let planted = text.replace("ॱॱ सङ्केतनॱअंशयुग्म भवति ०", "ॱॱ अंशयुग्म भवति ०");
    assert_ne!(
        planted, text,
        "the site the control plants on still exists in vishlesana.t1; if this \
         declaration was renamed or removed, point the control at another \
         QUALIFIED cross-module `भवति ०` declaration rather than deleting it"
    );
    std::fs::write(&victim, planted).expect("written");

    let (found, report) = census(&copy);
    assert!(
        found >= 1 && report.contains("अंशयुग्म"),
        "the census must find a planted bare declaration — it reported {found}:\n{report}"
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

/// THE LEGIBILITY RATCHET. ० on 2026-09-19.
///
/// SINCE `आयातितसंरचनासंख्या` LANDED THIS IS NOT A CORRECTNESS CLAIM. An
/// UNAMBIGUOUS bare cross-module declaration now gets storage, and
/// `w279_bare_type_lowering.rs` is what proves it. What this keeps is the
/// corpus's uniform spelling — see the header. The correctness ratchet is
/// `no_ambiguous_bare_cross_module_record_declaration` below.
#[test]
fn no_bare_cross_module_record_declaration() {
    let (n, report) = census(&root().join(SRC));
    assert_eq!(
        n, 0,
        "a bare cross-module record declaration gets NO storage natively and \
         faults at address ० on its first field access. Write the module prefix \
         (`सङ्केतनॱअवकाश`, not `अवकाश`).\n{report}"
    );
}

/// THE CORRECTNESS RATCHET, and the only one of the two that is still one.
///
/// A bare name TWO imported modules declare has no single answer.
/// `आयातितसंरचनासंख्या` counts its candidates and REFUSES rather than guess —
/// picking either would hand back a field count from the wrong record, and a
/// short allocation corrupts whatever was laid after it where a null at least
/// faults at the first access. So a tie lowers to `ध्रुवरचना ०` exactly as
/// every bare declaration did before 2026-09-19, and faults natively at its
/// first field access, at an address with no relation to the source line.
///
/// `आज्ञा` (मध्यरूप, वाक्यविभाग) and `वाक्य` (वास्तु, वाक्यविभाग) are the two
/// names in this corpus that can produce one. Qualify with the module.
#[test]
fn no_ambiguous_bare_cross_module_record_declaration() {
    let (_, report) = census(&root().join(SRC));
    assert_eq!(
        count_labelled(&report, AMBIGUOUS),
        0,
        "a bare name that TWO imported modules declare is refused by \
         `आयातितसंरचनासंख्या` — deliberately, since guessing gives a wrong SIZE \
         — so the slot holds ० and the first field access faults at address ० \
         in the native image. Write the module prefix.\n{report}"
    );
}

/// THE POSITIVE CONTROL FOR THE RATCHET ABOVE, and it is a SEPARATE plant from
/// the unambiguous one because it exercises a different branch of the matcher:
/// the row goes to the other list, and a census that lost the `len(owners) > 1`
/// split would report the planted site as UNAMBIGUOUS and answer ० here while
/// looking entirely healthy.
///
/// `वाक्य` is declared by both `वास्तु` and `वाक्यविभाग`, and `vishlesana.t1` is
/// module `विश्लेषण` — neither — so a bare `वाक्य` there is a genuine tie.
#[test]
fn the_census_finds_an_ambiguous_declaration_when_one_is_planted() {
    let scratch = std::env::temp_dir().join(format!("w279-tie-control-{}", std::process::id()));
    let copy = scratch.join("src");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&copy).expect("a temp dir");

    let from = root().join(SRC);
    for entry in std::fs::read_dir(&from).expect("the corpus is readable") {
        let p = entry.expect("a dir entry").path();
        if p.extension().is_some_and(|e| e == "t1") {
            std::fs::copy(&p, copy.join(p.file_name().expect("a file name"))).expect("copied");
        }
    }

    let (_, clean) = census(&copy);
    assert_eq!(
        count_labelled(&clean, AMBIGUOUS),
        0,
        "the untouched copy matches the corpus"
    );

    let victim = copy.join("vishlesana.t1");
    let text = std::fs::read_to_string(&victim).expect("readable");
    let planted = text.replace("ॱॱ सङ्केतनॱअंशयुग्म भवति ०", "ॱॱ वाक्य भवति ०");
    assert_ne!(
        planted, text,
        "the site the control plants on still exists in vishlesana.t1; if this \
         declaration was renamed or removed, point the control at another \
         QUALIFIED cross-module `भवति ०` declaration rather than deleting it"
    );
    std::fs::write(&victim, planted).expect("written");

    let (bare, report) = census(&copy);
    assert_eq!(
        count_labelled(&report, AMBIGUOUS),
        1,
        "the census must find a planted TIE and put it in the ambiguous \
         list:\n{report}"
    );
    assert_eq!(
        bare, 0,
        "and must NOT also count it as unambiguous — the two lists are \
         disjoint, which is the whole reason they are two lines:\n{report}"
    );
    assert!(
        report.contains("AMBIGUOUS across"),
        "and must name it as a tie:\n{report}"
    );

    let _ = std::fs::remove_dir_all(&scratch);
}
