//! # `W-315` — WHAT `yantra` DOES NOT RUN, AND WHY THE THREE NUMBERS NEVER AGREED
//!
//! The ledger carried an open contradiction for eleven cycles: **21 reds in five
//! targets** against **0 reds in 38 targets**, with a third number beside it —
//! *"the ignored set is 16, not 21 ... `#[ignore]` attributes across the crate
//! number 25, in nine files, which matches neither"*. Three readings, one crate.
//!
//! **There was never a disagreement between two instruments.** Reproduced on this
//! tree, `cargo test -p yantra --release --no-fail-fast` answers **38 targets, 253
//! passed, 0 failed, 16 ignored, exit 0** — the same command, the same population,
//! the same answer the 0-red reading gave. The **21** is a DATED CLAIM measured at
//! `0213de98` (2026-09-17 02:43) and superseded FOUR TIMES the same day by commits
//! this ledger already records: `6935fd6d` (21 in 5 → 4 in 3), `70d78509`
//! (4 in 3 → 2 in 2), `1249b485` (2 in 2 → 1 in 1), `7b6a7b29` (1 in 1 → 0). The
//! stale instrument was a NUMBER COPIED FORWARD IN PROSE, and its own repairs were
//! four paragraphs further down the same file.
//!
//! **The 25 was a grep counting nine PROSE MENTIONS as attributes.** `#[ignore` also
//! occurs inside `//`, `///` and `//!` comments — this crate's test files argue
//! about ignoring at length — and a `grep -c '#\[ignore'` cannot tell an attribute
//! from a sentence about one. The real count is **16 attributes in EIGHT files**,
//! and `t1_storage_witness.rs` is the ninth file: its only occurrence is the
//! comment *"NOT `#[ignore]`d, DELIBERATELY"*, which says the exact opposite of
//! what the count credited it with.
//!
//! So this file is the instrument that makes the reconciliation re-derivable:
//! it classifies **every** occurrence, refuses to guess about one it cannot place,
//! and asserts the structural rules that a count alone cannot state — that an
//! attribute sits on a `#[test]` (an `#[ignore]` on anything else is invisible to
//! the harness AND to the census), that it carries a reason, and that a reason
//! parks a blocker only when it says WHERE.
//!
//! **This file is inside its own population, deliberately.** `W-314`'s loader
//! excluded itself and its self-exclusion test passed vacuously; there is nothing
//! to exclude here — every `#[ignore` below sits in a comment or a string literal
//! and is counted as PROSE by the same rule that counts the other nine.
//!
//! What the sixteen guard is reported and not argued: ten are measurements, two
//! censuses, one probe, one diagnostic, one an owner's ruling — and **exactly one
//! is a parked BLOCKER whose own reason says `UNVERIFIED`**
//! (`kosha_end_to_end.rs:798`), while its sibling at `:674` records that the site
//! it names now handles the case and that the sibling itself PASSES. That is a
//! refuted margin sitting behind an attribute no gate reaches. It is NAMED here,
//! not repaired: settling it costs the 1237 s run its sibling's reason quotes.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Where an occurrence of the token sits. The truth has three states and a
/// three-state instrument is the point: a two-state one — attribute or
/// not-attribute — is exactly the reading that produced 25.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placement {
    /// The line, trimmed, BEGINS with the attribute. This is what `rustc` sees.
    Attribute,
    /// The token is inside a `//` comment or after a `"` on the same line. This is
    /// what a human wrote ABOUT ignoring, and the harness never sees it.
    Prose,
    /// Neither. The census refuses to place it rather than defaulting it to either
    /// side — a default here is how a miscount becomes a margin.
    Unplaceable,
}

/// The token as `rustc` spells it at the start of an attribute. `#[ignore]` and
/// `#[ignore = "..."]` are the only two forms; `#[ignored]` and `#[ignore_me]` are
/// NOT this attribute and must not be counted as one — the substring trap `W-312`
/// caught in `calls_into` and `W-313` caught in `कआस्की`, in a third place.
fn attribute_head(rest: &str) -> bool {
    let after = &rest["#[ignore".len()..];
    after.starts_with(']') || after.trim_start().starts_with('=')
}

/// Place one occurrence, given the whole line and the byte offset the token starts
/// at. A WINDOW and not a parse, deliberately: it does not need to know Rust, only
/// whether a compiler or a reader is the audience.
fn place(line: &str, at: usize) -> Placement {
    let before = &line[..at];
    let rest = &line[at..];
    if before.trim().is_empty() && attribute_head(rest) {
        return Placement::Attribute;
    }
    if before.contains("//") || before.contains('"') {
        return Placement::Prose;
    }
    Placement::Unplaceable
}

#[derive(Debug, Clone)]
struct Site {
    file: String,
    line: usize,
    placement: Placement,
    text: String,
}

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// Every `.rs` directly under `crates/yantra/tests/`, sorted. The harness compiles
/// exactly these as integration targets, so this is the population the `ignored`
/// count is taken over — no walk into `hopladder/` or `paradigm/`, which are
/// modules of a target and not targets.
fn target_files() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(tests_dir())
        .expect("crates/yantra/tests must be readable")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "rs"))
        .collect();
    v.sort();
    v
}

fn scan() -> Vec<Site> {
    let mut sites = Vec::new();
    for path in target_files() {
        let name = path
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();
        let body = std::fs::read_to_string(&path).expect("test file must be readable");
        for (i, line) in body.lines().enumerate() {
            let mut from = 0usize;
            while let Some(rel) = line[from..].find("#[ignore") {
                let at = from + rel;
                sites.push(Site {
                    file: name.clone(),
                    line: i + 1,
                    placement: place(line, at),
                    text: line.trim().to_string(),
                });
                from = at + "#[ignore".len();
            }
        }
    }
    sites
}

/// The `fn` an attribute governs, and whether a `#[test]` governs it too. Scans
/// down past further attributes and comments for the item, and up past the same
/// for the `#[test]`. An `#[ignore]` with no `#[test]` above it is ignored by
/// nothing — it does not appear in the harness's count and it is not a hole the
/// count can show.
fn governed_item(body: &[&str], idx: usize) -> (Option<String>, bool) {
    let skip = |l: &str| {
        let t = l.trim();
        t.is_empty() || t.starts_with('#') || t.starts_with("//")
    };
    // THE ATTRIBUTE MAY NOT END ON ITS OWN LINE. Three of the sixteen carry a
    // `\`-continued reason spanning up to seven lines, and a scan that started at
    // `idx + 1` read the second line of a STRING as the governed item and reported
    // "governs no `fn` at all" for all three. Found by running this test, not by
    // reading it: the first version red on `kosha_end_to_end.rs:674`, `:798` and
    // `t1_hopcorpus.rs:61` — every multi-line reason in the crate and nothing else.
    let mut end = idx;
    while end < body.len() && !body[end].trim_end().ends_with(']') {
        end += 1;
    }
    let mut fname = None;
    for l in body.iter().skip(end + 1) {
        if skip(l) {
            continue;
        }
        let t = l.trim_start();
        let t = t.strip_prefix("pub ").unwrap_or(t);
        let t = t.strip_prefix("async ").unwrap_or(t);
        if let Some(r) = t.strip_prefix("fn ") {
            let n: String = r
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            fname = Some(n);
        }
        break;
    }
    let mut has_test = false;
    for j in (0..idx).rev() {
        let l = body[j];
        if l.trim() == "#[test]" {
            has_test = true;
            break;
        }
        if !skip(l) {
            break;
        }
    }
    (fname, has_test)
}

/// The reason string of an attribute, if it carries one. Multi-line reasons
/// (`\` continuations) are common here, so the opening line is enough to establish
/// that a reason EXISTS; the kind classification reads the joined text.
fn reason_of(body: &[&str], idx: usize) -> Option<String> {
    let first = body[idx].trim();
    let open = first.find('"')?;
    let mut acc = String::new();
    let mut k = idx;
    loop {
        let seg = if k == idx {
            &first[open + 1..]
        } else {
            body[k]
        };
        let mut ended = false;
        let mut prev_backslash = false;
        for c in seg.chars() {
            if c == '"' && !prev_backslash {
                ended = true;
                break;
            }
            prev_backslash = c == '\\' && !prev_backslash;
            acc.push(c);
        }
        if ended || k + 1 >= body.len() {
            break;
        }
        acc.push(' ');
        k += 1;
    }
    Some(acc)
}

/// What an ignored test IS. Taken from the reason's own words rather than assigned
/// by this file, so a reason that names no kind at all reds instead of being
/// silently bucketed.
const KINDS: &[(&str, &[&str])] = &[
    ("blocked", &["BLOCKED"]),
    ("legacy", &["legacy"]),
    ("probe", &["probe"]),
    ("diagnostic", &["diagnostic"]),
    ("census", &["census"]),
    ("measurement", &["measurement", "MEASUREMENT"]),
];

/// THE LEAD WORD NAMES THE KIND, and that rule is not cosmetic — it was forced by
/// a real row. `kosha_end_to_end.rs:674` opens `"MEASUREMENT, 1237s — and it
/// PASSES. The old reason said BLOCKED at encode.t1:5820 ... and the claim is
/// stale"`. A table-order match would have read it as a PARKED BLOCKER and put the
/// blocked count at two, when what that reason does is RETIRE a blocker. So the
/// kind is the keyword that occurs EARLIEST in the reason, and a reason that only
/// discusses another kind later is not classified by it.
fn kind_of(reason: &str) -> Option<&'static str> {
    KINDS
        .iter()
        .filter_map(|(k, words)| {
            words
                .iter()
                .filter_map(|w| reason.find(w))
                .min()
                .map(|at| (at, *k))
        })
        .min()
        .map(|(_, k)| k)
}

/// A parked blocker names a site when its reason carries a `:` followed by a
/// digit — `encode.t1:5820`. Prose colons do not count, which is the difference
/// between a claim somebody can check and a claim nobody can.
fn names_a_site(reason: &str) -> bool {
    reason
        .as_bytes()
        .windows(2)
        .any(|w| w[0] == b':' && w[1].is_ascii_digit())
}

/// One ignored test, fully read: where it is, what it governs, why, and what kind
/// of thing it is.
#[derive(Debug, Clone)]
struct Ignored {
    site: Site,
    fname: String,
    has_test: bool,
    reason: String,
    kind: &'static str,
}

fn attributes() -> Vec<Ignored> {
    let mut out = Vec::new();
    for path in target_files() {
        let name = path
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();
        let body = std::fs::read_to_string(&path).expect("readable");
        let lines: Vec<&str> = body.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if let Some(at) = line.find("#[ignore") {
                if place(line, at) != Placement::Attribute {
                    continue;
                }
                let (fname, has_test) = governed_item(&lines, i);
                let reason = reason_of(&lines, i).unwrap_or_default();
                let kind = kind_of(&reason).unwrap_or("UNCLASSIFIED");
                out.push(Ignored {
                    site: Site {
                        file: name.clone(),
                        line: i + 1,
                        placement: Placement::Attribute,
                        text: line.trim().to_string(),
                    },
                    fname: fname.unwrap_or_default(),
                    has_test,
                    reason,
                    kind,
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The classifier, against cases it must ACCEPT and cases it must REFUSE.
// ---------------------------------------------------------------------------

#[test]
fn the_classifier_refuses_a_sentence_about_ignoring_and_accepts_only_an_attribute() {
    // ACCEPTED — the two forms `rustc` reads, at column 0 and indented.
    for l in [
        "#[ignore]",
        "    #[ignore]",
        "#[ignore = \"measurement: minutes\"]",
        "\t#[ignore = \"census\"]",
        "#[ignore = \"a reason that wraps \\",
    ] {
        let at = l.find("#[ignore").expect("token present");
        assert_eq!(
            place(l, at),
            Placement::Attribute,
            "must read as an attribute: {l:?}"
        );
    }

    // REFUSED — every one of these is a sentence, and counting them is the defect
    // this file exists to close. The last two are the substring trap: a longer
    // attribute name is NOT this attribute.
    for l in [
        "//! It is `#[ignore]`d, so a whole-crate run reports green",
        "/// This was `#[ignore]`d for one commit",
        "    // NOT `#[ignore]`d, DELIBERATELY. This asserts",
        "        \"#[ignore = \\\"x\\\"]\",",
        "#[ignored]",
        "#[ignore_me]",
    ] {
        let at = l.find("#[ignore").expect("token present");
        assert_ne!(
            place(l, at),
            Placement::Attribute,
            "must NOT read as an attribute: {l:?}"
        );
    }

    // And the third state is REACHABLE, so `Unplaceable` is not decoration: a bare
    // token in code position, in no comment and no string, is placed by neither
    // rule and the census says so rather than picking a side.
    let odd = "let x = 1; #[ignore] fn f() {}";
    let at = odd.find("#[ignore").expect("token present");
    assert_eq!(place(odd, at), Placement::Unplaceable);
}

// ---------------------------------------------------------------------------
// The census, and the rules a count alone cannot state.
// ---------------------------------------------------------------------------

#[test]
fn every_ignore_token_in_the_crate_is_placed_and_the_prose_half_is_load_bearing() {
    let sites = scan();
    let attrs: Vec<&Site> = sites
        .iter()
        .filter(|s| s.placement == Placement::Attribute)
        .collect();
    let prose: Vec<&Site> = sites
        .iter()
        .filter(|s| s.placement == Placement::Prose)
        .collect();
    let odd: Vec<&Site> = sites
        .iter()
        .filter(|s| s.placement == Placement::Unplaceable)
        .collect();

    assert!(
        odd.is_empty(),
        "the census cannot place {} occurrence(s), and placing them by default is \
         how a miscount becomes a margin — place them by hand or teach `place`:\n{}",
        odd.len(),
        odd.iter()
            .map(|s| format!("  {}:{}  {}", s.file, s.line, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // NON-VACUOUS, both halves. A classifier that answered `Attribute` for
    // everything, or `Prose` for everything, would satisfy every other assertion
    // in this file; `W-314`'s lesson — a guard no run can falsify is a comment —
    // one layer down again.
    assert!(!attrs.is_empty(), "no `#[ignore]` attribute found at all");
    assert!(
        !prose.is_empty(),
        "no PROSE occurrence found — the discriminator is doing no work, and a \
         count that does no work is the 25"
    );

    // THE ROW THAT MADE NINE FILES OUT OF EIGHT, asserted by name. Its only
    // occurrence says the OPPOSITE of what a grep credited it with.
    let witness_attrs = attrs
        .iter()
        .filter(|s| s.file == "t1_storage_witness.rs")
        .count();
    let witness_prose = prose
        .iter()
        .filter(|s| s.file == "t1_storage_witness.rs")
        .count();
    assert_eq!(
        witness_attrs, 0,
        "t1_storage_witness.rs declares itself NOT ignored; an attribute there \
         means the margin at :404 is now false"
    );
    assert!(
        witness_prose >= 1,
        "t1_storage_witness.rs's `NOT `#[ignore]`d, DELIBERATELY` margin is gone — \
         the file that proves the prose half exists no longer does"
    );

    let mut files: Vec<String> = attrs.iter().map(|s| s.file.clone()).collect();
    files.sort();
    files.dedup();

    let mut report = String::new();
    let _ = writeln!(report, "METRIC yantra_ignore_tokens {}", sites.len());
    let _ = writeln!(report, "METRIC yantra_ignored_tests {}", attrs.len());
    let _ = writeln!(report, "NOTE   yantra_ignore_prose {}", prose.len());
    // THE FINDING'S OWN NUMBER, kept legible. This file is in its own population
    // on purpose, and its string-literal cases are the bulk of the prose total —
    // so the count that says what a `grep -c` over the OTHER targets would have
    // over-reported is printed apart from it.
    let elsewhere = prose
        .iter()
        .filter(|s| s.file != "ignored_census.rs")
        .count();
    let _ = writeln!(report, "NOTE   yantra_ignore_prose_elsewhere {elsewhere}");
    let _ = writeln!(
        report,
        "NOTE   yantra_grep_would_report {}",
        attrs.len() + elsewhere
    );
    let _ = writeln!(report, "NOTE   yantra_ignored_files {}", files.len());
    for f in &files {
        let n = attrs.iter().filter(|s| &s.file == f).count();
        let _ = writeln!(report, "NOTE   yantra_ignored_in {f} {n}");
    }
    println!("{report}");
}

#[test]
fn an_ignore_attribute_governs_a_test_and_carries_a_reason_that_names_a_kind() {
    let attrs = attributes();
    assert!(!attrs.is_empty(), "no attributes to check");

    let mut faults = Vec::new();
    for Ignored {
        site,
        fname,
        has_test,
        reason,
        kind,
    } in &attrs
    {
        // INVISIBLE TO THE HARNESS AND TO THIS FILE ALIKE. An `#[ignore]` with no
        // `#[test]` above it is not in the `ignored` count and is not a hole the
        // count can show — so it has to red here or nowhere.
        if !has_test {
            faults.push(format!(
                "  {}:{}  governs `{fname}` with NO `#[test]` above it — the \
                 harness never counts it",
                site.file, site.line
            ));
        }
        if fname.is_empty() {
            faults.push(format!(
                "  {}:{}  governs no `fn` at all",
                site.file, site.line
            ));
        }
        // A REASON IS THE WHOLE VALUE OF AN IGNORED TEST. Without one, the row
        // says a test does not run and nothing says why.
        if reason.trim().is_empty() {
            faults.push(format!(
                "  {}:{}  `{fname}` is ignored with no reason",
                site.file, site.line
            ));
        } else if *kind == "UNCLASSIFIED" {
            faults.push(format!(
                "  {}:{}  `{fname}`'s reason names no kind of {:?}: {reason:?}",
                site.file,
                site.line,
                KINDS.iter().map(|(k, _)| *k).collect::<Vec<_>>()
            ));
        }
        // A PARKED BLOCKER MUST SAY WHERE. `BLOCKED` with no site is a claim
        // nobody can check, and an unverifiable claim behind an attribute no gate
        // reaches is how a refuted margin survives — which is exactly what the one
        // `blocked` row below turns out to be.
        if *kind == "blocked" && !names_a_site(reason) {
            faults.push(format!(
                "  {}:{}  `{fname}` parks a BLOCKER that names no site: {reason:?}",
                site.file, site.line
            ));
        }
    }
    assert!(
        faults.is_empty(),
        "{} ignored test(s) are not well-formed:\n{}",
        faults.len(),
        faults.join("\n")
    );

    let mut report = String::new();
    for (kind, _) in KINDS {
        let n = attrs.iter().filter(|i| &i.kind == kind).count();
        let _ = writeln!(report, "NOTE   yantra_ignored_kind_{kind} {n}");
    }
    let blocked: Vec<String> = attrs
        .iter()
        .filter(|i| i.kind == "blocked")
        .map(|i| format!("{}:{} {}", i.site.file, i.site.line, i.fname))
        .collect();
    let _ = writeln!(report, "METRIC yantra_ignored_blocked {}", blocked.len());
    for b in &blocked {
        let _ = writeln!(report, "NOTE   yantra_ignored_blocked_at {b}");
    }
    println!("{report}");
}

#[test]
fn the_kind_is_the_reason_s_lead_word_and_a_retired_blocker_is_not_a_parked_one() {
    // THE CASE THAT MUST STILL BE REFUSED, and it is a real row rather than an
    // invented one: `kosha_end_to_end.rs:674`'s reason opens as a MEASUREMENT and
    // mentions a blocker only to say the claim is STALE. Classified by table
    // order it reads `blocked`; classified by lead word it reads `measurement`,
    // which is what it is. Getting this wrong doubles the one number this file
    // reports as a finding.
    let retired = "MEASUREMENT, 1237s — and it PASSES. The old reason said BLOCKED \
                   at encode.t1:5820, refusing R_RISCV_PCREL_LO12_I unconditionally; \
                   that site now handles it and the claim is stale.";
    assert_eq!(kind_of(retired), Some("measurement"));

    // ACCEPTED as parked: the blocker leads, and nothing retires it.
    let parked = "BLOCKED at encode.t1:5820 — claim UNVERIFIED as of 2026-09-16.";
    assert_eq!(kind_of(parked), Some("blocked"));
    assert!(names_a_site(parked));

    // An owner's ruling that also mentions a census is a ruling, not a census.
    let ruling = "legacy (owner's ruling 2026-09-13: the narrowed census is the gate)";
    assert_eq!(kind_of(ruling), Some("legacy"));

    // REFUSED: a reason naming no kind at all is a hole, not a bucket.
    assert_eq!(kind_of("too slow"), None);

    // REFUSED: a prose colon is not a site. Without this the `blocked` rule passes
    // on any reason containing an em-dash clause, which is most of them.
    assert!(!names_a_site("BLOCKED: nobody has looked at this"));
    assert!(names_a_site("BLOCKED at ir.t1:2255"));
}
