//! `W-336` — THE UCD VERSION IS PINNED, AND THIS REFUSES WHEN THE CORPUS ON DISK
//! DISAGREES WITH THE PIN.
//!
//! # Why this exists
//!
//! `research/specs/fetch-specs.sh` fetched the character database from
//! `$U/Public/UCD/latest/ucd` until 2026-10-01. So the corpus a conformance gate
//! graded against was **whatever Unicode had published on the day somebody ran
//! the fetcher**, and nothing recorded which version that was.
//!
//! MEASURED, not argued. The same commit `c352a1fd` held
//! `GraphemeBreakTest.txt` at 17.0.0 (796 lines, sha `e2d134d2…`) on one host
//! and 18.0.0 (883 lines, sha `b0cf047e…`) on a freshly fetched other. The
//! three-crate gate read 142 ok / 0 fail on the first; on the second,
//! `each_of_the_three_tables_is_load_bearing` went RED — *"the unmodified tables
//! must segment every case"*, left 10, right 0 — because ten cases are new in
//! UCD 18 and the segmentation tables in this tree are built for 17.
//!
//! **THE FAILURE MODE WAS SILENT AND TIME-TRIGGERED, WHICH IS WORSE THAN A RED
//! GATE.** No commit is to blame when Unicode publishes; a fresh clone and an
//! old clone disagree and neither can say why.
//!
//! # Three places already believed the pin existed
//!
//! The intent was written down, repeatedly, and correct each time — only the
//! mechanism was missing:
//!
//! - `crates/adhvan/tests/idna_tables.rs:231` — *"MEASURED off the **pinned**
//!   17.0.0 file … so a Unicode upgrade that changes which domains are rejected
//!   has to be a deliberate act."*
//! - `crates/sanskrit-text/tests/golden_regression.rs:21` — *"doc 01 D-01-A pins
//!   the UCD version"*. D-01-A defines the अक्षर as a UAX #29 extended grapheme
//!   cluster; it pins no version, so that citation overstates it.
//! - `fetch-specs.sh` itself pinned the sibling PDFs explicitly
//!   (`$U/versions/Unicode15.0.0/…`) while leaving the DATA unpinned.
//!
//! # What this checks, and why it reads the script
//!
//! The pin is read OUT OF `fetch-specs.sh` rather than written here, so there is
//! ONE source of truth. A copy in this file would be a second number to drift —
//! which is the whole defect class this row belongs to.
//!
//! # Not covered
//!
//! This does not verify the corpus CONTENT against the standard — the
//! conformance suites do that. It asserts only that the version on disk is the
//! version that was asked for. A corrupted 17.0.0 file passes here and fails
//! there, which is the correct division.

use std::path::{Path, PathBuf};

fn specs_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/specs")
}

/// The version `fetch-specs.sh` pins, read from the script itself.
///
/// **THE PATH SHAPE IS LOAD-BEARING AND THE OBVIOUS GUESS IS WRONG.** The
/// versioned convention is `Public/<version>/ucd`, and `Public/UCD/<version>/ucd`
/// — the form `latest` uses with `UCD/` retained — answers **404**. Verified
/// against the server before the pin landed, because `W-336`'s own acceptance
/// named the wrong form and would have broken every UCD fetch.
fn pinned_version() -> String {
    let script = specs_root().join("fetch-specs.sh");
    let text = std::fs::read_to_string(&script)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", script.display()));
    let line = text
        .lines()
        .find(|l| l.starts_with("UCD="))
        .unwrap_or_else(|| {
            panic!(
                "fetch-specs.sh has no `UCD=` assignment — the pin this test \
                 reads has been renamed or removed, and an unpinned fetch is \
                 exactly what `W-336` is about"
            )
        });
    assert!(
        !line.contains("/latest/"),
        "fetch-specs.sh fetches the UCD from `latest`: {line}\n  \
         An unpinned corpus makes a gate's verdict depend on WHEN it ran — the \
         same commit reads 142 ok / 0 fail on a host with 17.0.0 and RED on one \
         with 18.0.0. Pin an explicit version (`$U/Public/<version>/ucd`, NOT \
         `Public/UCD/<version>/ucd`, which 404s)."
    );
    let after = line
        .rsplit_once("/Public/")
        .map(|(_, r)| r)
        .unwrap_or_else(|| {
            panic!("`UCD=` does not name a /Public/ path, so no version can be read: {line}")
        });
    let version = after.split('/').next().unwrap_or_default().to_string();
    assert!(
        version.split('.').count() == 3 && version.split('.').all(|p| p.parse::<u32>().is_ok()),
        "the pinned segment is not a three-part version: {version:?} from {line}"
    );
    version
}

/// Every corpus file's own header names its version — `# <Name>-17.0.0.txt`.
fn header_version(rel: &str) -> Option<String> {
    let path = specs_root().join(rel);
    let text = std::fs::read_to_string(&path).ok()?;
    let first = text.lines().next()?;
    let stem = first.rsplit_once('-')?.1;
    Some(stem.trim_end_matches(".txt").trim().to_string())
}

/// ॥ THE CORPUS ON DISK IS THE VERSION THE FETCHER PINS ॥ `W-336`.
///
/// **AN ABSENT CORPUS IS SKIPPED, NOT PASSED, AND THE DISTINCTION IS PRINTED.**
/// The data files are gitignored and fetched by a script, so a fresh clone has
/// none — and a test that silently passes on an absent corpus is the false green
/// this whole row is about. The conformance suites already refuse by name when
/// the file is missing and NAME the remedy (`run research/specs/fetch-specs.sh`),
/// so that failure is covered there; this reports the skip and says so.
#[test]
fn the_ucd_corpus_on_disk_matches_the_pinned_version() {
    let pinned = pinned_version();
    println!("METRIC ucd_pinned_version {pinned}");

    let corpora = [
        "unicode/ucd/auxiliary/GraphemeBreakTest.txt",
        "unicode/ucd/NormalizationTest.txt",
        "unicode/ucd/UnicodeData.txt",
    ];

    let mut checked = 0usize;
    let mut absent = Vec::new();
    let mut wrong = Vec::new();
    for rel in corpora {
        match header_version(rel) {
            None => absent.push(rel),
            Some(found) if found == pinned => checked += 1,
            Some(found) => {
                wrong.push(format!("{rel}: on disk {found}, pinned {pinned}"));
            }
        }
    }

    println!("METRIC ucd_corpora_matching_pin {checked}");
    println!("METRIC ucd_corpora_absent {}", absent.len());

    assert!(
        wrong.is_empty(),
        "the UCD corpus on disk is NOT the version `fetch-specs.sh` pins:\n  {}\n\
         \n  This is the `W-336` skew: two hosts grade the same commit against \
         different Unicode versions and disagree with no code difference. Either \
         re-fetch (`bash research/specs/fetch-specs.sh`, and it is BASH — `sh` \
         is dash on Ubuntu and dies on line 21), or move the pin deliberately \
         and rebuild the segmentation tables, which is its own row.",
        wrong.join("\n  ")
    );

    if !absent.is_empty() {
        println!(
            "NOTE   {} of {} corpora absent (gitignored, fetched by script): {}",
            absent.len(),
            corpora.len(),
            absent.join(", ")
        );
    }
    assert!(
        checked > 0 || !absent.is_empty(),
        "neither matched nor absent — the corpus list or the header parser is broken"
    );
}
