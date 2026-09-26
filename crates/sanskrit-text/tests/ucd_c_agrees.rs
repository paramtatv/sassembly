//! The Rust and C Unicode tables are one table — task `W-016`, doc 01 §7.
//!
//! `ucdgen` renders the same parsed ranges twice, into
//! `crates/sanskrit-text/src/tables.rs` and `baselines/c/ucd_tables.h`. That is
//! the whole point: `W-007a5` needs Unicode tables in C for the text hot paths,
//! and a hand-written C copy would be a transcription of a table the generator
//! already derives — doc 03 §4.4's rule, one language over.
//!
//! "Generated from the same source" is a claim about a program, and a claim
//! about a program is worth what checks it. Both files are read here and held
//! to the same numbers, so a change to one emitter that does not reach the
//! other fails the build rather than shipping two tables that agree on nothing
//! in particular.
//!
//! # The invariant that matters is the VALUE, not the boundary
//!
//! Two tables can agree on every range boundary and disagree on every meaning,
//! because Rust names its values and C numbers them. The C emitter writes the
//! index of the variant in declaration order, which is exactly Rust's implicit
//! discriminant — so the same integer must mean the same property value in both
//! languages. Boundaries alone would pass with the values permuted.

use std::path::{Path, PathBuf};

/// One table as Rust writes it: a name and rows carrying a named value.
type RustTable = (String, Vec<(u32, u32, String)>);
/// One table as C writes it: a name and rows carrying a value INDEX.
type CTable = (String, Vec<(u32, u32, u8)>);

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

/// `(lo, hi, value-name)` for every table in the Rust file.
fn rust_tables() -> Vec<RustTable> {
    let text = std::fs::read_to_string(root().join("crates/sanskrit-text/src/tables.rs"))
        .expect("read tables.rs");
    let mut out: Vec<RustTable> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("pub static ") {
            let name = rest.split(':').next().unwrap_or("").trim().to_string();
            if rest.contains("&[(u32, u32,") {
                out.push((name, Vec::new()));
            }
            continue;
        }
        // `    (0x0000, 0x001F, GraphemeBreak::Control),`
        let Some(row) = line.strip_prefix('(') else {
            continue;
        };
        let Some(row) = row.strip_suffix("),") else {
            continue;
        };
        let parts: Vec<&str> = row.split(',').map(str::trim).collect();
        if parts.len() != 3 || !parts[2].contains("::") {
            continue;
        }
        let Some(last) = out.last_mut() else { continue };
        let lo = u32::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap_or(u32::MAX);
        let hi = u32::from_str_radix(parts[1].trim_start_matches("0x"), 16).unwrap_or(u32::MAX);
        let value = parts[2].split("::").nth(1).unwrap_or("").to_string();
        last.1.push((lo, hi, value));
    }
    out.retain(|(_, rows)| !rows.is_empty());
    out
}

/// `(lo, hi, value-index)` for every table in the C header, plus the `#define`
/// names so an index can be turned back into a value name.
fn c_tables() -> (Vec<CTable>, Vec<(String, u8)>) {
    let text = std::fs::read_to_string(root().join("baselines/c/ucd_tables.h"))
        .expect("read ucd_tables.h");
    let mut tables: Vec<CTable> = Vec::new();
    let mut defines: Vec<(String, u8)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#define UCD_") {
            let mut it = rest.split_whitespace();
            if let (Some(name), Some(value)) = (it.next(), it.next())
                && let Ok(v) = value.parse::<u8>()
                && !name.ends_with("_COUNT")
            {
                defines.push((name.to_string(), v));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("static const ucd_range_t UCD_") {
            let name = rest.split('[').next().unwrap_or("").to_string();
            tables.push((name, Vec::new()));
            continue;
        }
        // `    { 0x0000, 0x001F, 3 },`
        let Some(row) = line.strip_prefix("{ ") else {
            continue;
        };
        let Some(row) = row.strip_suffix(" },") else {
            continue;
        };
        let parts: Vec<&str> = row.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            continue;
        }
        let Some(last) = tables.last_mut() else {
            continue;
        };
        let lo = u32::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap_or(u32::MAX);
        let hi = u32::from_str_radix(parts[1].trim_start_matches("0x"), 16).unwrap_or(u32::MAX);
        let v = parts[2].parse::<u8>().unwrap_or(u8::MAX);
        last.1.push((lo, hi, v));
    }
    tables.retain(|(_, rows)| !rows.is_empty());
    // `CCC` reuses `ucd_range_t` because a combining class IS a small integer,
    // so it parses as a property table here while its Rust twin — typed
    // `&[(u32, u32, u8)]` with no named variants — does not. It is compared by
    // `the_three_nfc_tables_agree_too` instead, where the values are numbers on
    // both sides. Two parsers disagreeing about what counts as a table is the
    // sort of thing that reads as a missing table.
    tables.retain(|(name, _)| name != "CCC");
    (tables, defines)
}

#[test]
fn the_two_files_carry_the_same_tables() {
    let rust = rust_tables();
    let (c, defines) = c_tables();

    assert!(rust.len() >= 6, "only {} Rust tables parsed", rust.len());
    assert_eq!(
        rust.len(),
        c.len(),
        "Rust has {:?}, C has {:?}",
        rust.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        c.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );

    for ((rname, rrows), (cname, crows)) in rust.iter().zip(&c) {
        assert_eq!(rname, cname, "the tables are in a different order");
        assert_eq!(
            rrows.len(),
            crows.len(),
            "{rname}: {} rows in Rust, {} in C",
            rrows.len(),
            crows.len()
        );
        for (i, ((rlo, rhi, rv), (clo, chi, cv))) in rrows.iter().zip(crows).enumerate() {
            assert_eq!((rlo, rhi), (clo, chi), "{rname} row {i}: boundaries differ");
            // The value, through the `#define` that names it. This is the check
            // boundaries alone would pass with the meanings permuted.
            let wanted = rv.to_uppercase();
            let named = defines
                .iter()
                .filter(|(n, v)| v == cv && n.ends_with(&wanted))
                .count();
            assert!(
                named > 0,
                "{rname} row {i}: Rust says {rv}, C says {cv}, and no #define ending {wanted} has that value"
            );
        }
    }
}

#[test]
fn regenerating_changes_nothing() {
    // `A-011`'s determinism argument, extended to the second emitter: two
    // renderings from one parse are only one table if running the generator
    // again produces the same bytes. Checked by content rather than by running
    // `ucdgen` — a test that shells out to a sibling crate is a test of the
    // build, not of the tables.
    let header = std::fs::read_to_string(root().join("baselines/c/ucd_tables.h")).expect("read");
    assert!(
        header.contains("DO NOT EDIT"),
        "the header lost its warning"
    );
    assert!(
        header
            .trim_end()
            .ends_with("#endif /* SANSOS_UCD_TABLES_H */"),
        "the include guard is not closed; the file was truncated"
    );
    // No timestamps, no paths: doc 03 §6. A DATE shape rather than the digits
    // `20`, which the first version forbade and which then appeared inside
    // `2081 entries` — a check that fires on the data it is meant to ignore.
    for line in header.lines() {
        let dated = line.as_bytes().windows(10).any(|w| {
            w[0] == b'2'
                && w[1] == b'0'
                && w[2..4].iter().all(u8::is_ascii_digit)
                && w[4] == b'-'
                && w[5..7].iter().all(u8::is_ascii_digit)
                && w[7] == b'-'
                && w[8..10].iter().all(u8::is_ascii_digit)
        });
        assert!(!dated, "a date appears in the generated header: {line}");
        for forbidden in ["/Users/", "/home/", "GMT"] {
            assert!(
                !line.contains(forbidden),
                "`{forbidden}` appears in the generated header: {line}"
            );
        }
    }
}

#[test]
fn the_c_text_workload_measures_the_same_corpus() {
    // `baselines/c/text.c` counts akṣaras over a sentence written as escaped
    // UTF-8; `crates/s1/src/tranche1.rs` counts them over the same sentence
    // written as Devanagari. A benchmark comparing two languages over different
    // data compares the data — so the two byte strings are compared here.
    //
    // The cluster COUNT agreeing (both say 38 for this sentence) is a property
    // of two conforming UAX #29 implementations, which the 766 grapheme-break
    // cases already hold the Rust one to. What no other test can see is the two
    // drifting to different sentences, which is one careless edit away and
    // would leave both suites green while measuring different work.
    let c = std::fs::read_to_string(root().join("baselines/c/text.c")).expect("read text.c");
    let rust =
        std::fs::read_to_string(root().join("crates/s1/src/tranche1.rs")).expect("read tranche1");

    let seed = rust
        .lines()
        .find_map(|l| l.trim().strip_prefix("const SEED: &str = \""))
        .and_then(|l| l.strip_suffix("\";"))
        .expect("the Rust workload names its SEED");

    // The C literal is `"\xe0\xa4\x85…"` split across lines; take every `\xNN`
    // between the declaration and its semicolon.
    let at = c.find("static const char TEXT_SEED[]").expect("the C seed");
    // Up to the semicolon that ends the declaration — so a COMMENT inside the
    // literal must not contain one. It did, once: a `;` in a note about
    // composition exclusions truncated the parse and the test reported the two
    // sentences as different when only the comment had changed. Cheap to work
    // around and worth stating, because the failure looked exactly like the
    // drift this test exists to catch.
    let body = &c[at..at + c[at..].find(';').expect("the literal ends")];
    let mut bytes = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find("\\x") {
        let hex = &rest[i + 2..i + 4];
        bytes.push(u8::from_str_radix(hex, 16).expect("two hex digits"));
        // A space between the escaped runs is a literal space in the sentence.
        rest = &rest[i + 4..];
        if rest.starts_with(' ') && !rest.trim_start().starts_with('"') {
            bytes.push(b' ');
        }
    }
    // The trailing `" "` runs the sentence out with a space; count the spaces
    // that sit inside quotes rather than between string fragments.
    let c_text = String::from_utf8(bytes).expect("the C seed is UTF-8");
    let c_stripped: String = c_text.chars().filter(|c| !c.is_whitespace()).collect();
    let rust_stripped: String = seed.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(
        c_stripped, rust_stripped,
        "the C and Rust workloads read different sentences"
    );
}

/// Every `0x…` or bare integer on a line, in order.
fn numbers(line: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let bytes: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '0' && i + 1 < bytes.len() && (bytes[i + 1] == 'x' || bytes[i + 1] == 'X') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            let hex: String = bytes[start..j].iter().collect();
            if let Ok(v) = u32::from_str_radix(&hex, 16) {
                out.push(v);
            }
            i = j;
        } else if bytes[i].is_ascii_digit() {
            let start = i;
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let dec: String = bytes[start..j].iter().collect();
            if let Ok(v) = dec.parse() {
                out.push(v);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// The rows of a table, as number tuples, from the line after its declaration
/// until the closing brace or bracket.
fn rows(text: &str, opener: &str) -> Vec<Vec<u32>> {
    let Some(at) = text.find(opener) else {
        return Vec::new();
    };
    let body = &text[at + opener.len()..];
    let end = body
        .find("];")
        .unwrap_or_else(|| body.find("};").unwrap_or(body.len()));
    body[..end]
        .lines()
        .skip(1)
        .filter(|l| l.contains("0x") || l.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .map(numbers)
        .filter(|v| !v.is_empty())
        .collect()
}

#[test]
fn the_three_nfc_tables_agree_too() {
    // `W-007a5b1`. CCC, the canonical decompositions and the primary composites
    // are what `nfc` runs on, and they were the three `W-016` left behind:
    // `emit` renders both languages, but these three had emitters of their own
    // that wrote Rust only. Row counts alone would pass on two tables of the
    // right length and the wrong contents, so every number is compared.
    let rust = std::fs::read_to_string(root().join("crates/sanskrit-text/src/tables.rs"))
        .expect("read tables.rs");
    let c = std::fs::read_to_string(root().join("baselines/c/ucd_tables.h")).expect("read header");

    let r_ccc = rows(&rust, "pub static CCC: &[(u32, u32, u8)] = &[");
    let c_ccc = rows(&c, "static const ucd_range_t UCD_CCC[] = {");
    assert_eq!(r_ccc.len(), 403, "the CCC table changed size");
    assert_eq!(r_ccc, c_ccc, "CCC differs between the two files");

    let r_comp = rows(
        &rust,
        "pub static CANONICAL_COMPOSE: &[(u32, u32, u32)] = &[",
    );
    let c_comp = rows(&c, "static const ucd_compose_t UCD_COMPOSE[] = {");
    assert_eq!(r_comp.len(), 961, "the composition table changed size");
    assert_eq!(r_comp, c_comp, "the primary composites differ");

    // The decompositions are stored differently on purpose — Rust has a slice
    // per row, C a flat array and an index — so they are compared by what they
    // MEAN rather than by how they are written.
    let r_decomp = rows(&rust, "pub static CANONICAL_DECOMP: &[(u32, &[u32])] = &[");
    let flat: Vec<u32> = rows(&c, "static const uint32_t UCD_DECOMP_DATA[] = {")
        .into_iter()
        .flatten()
        .collect();
    let index = rows(&c, "static const ucd_decomp_t UCD_DECOMP[] = {");
    assert_eq!(r_decomp.len(), 2081, "the decomposition table changed size");
    assert_eq!(r_decomp.len(), index.len(), "a decomposition was dropped");
    for (i, (r, idx)) in r_decomp.iter().zip(&index).enumerate() {
        assert_eq!(r[0], idx[0], "row {i}: different codepoint");
        let (at, len) = (idx[1] as usize, idx[2] as usize);
        assert_eq!(
            &r[1..],
            &flat[at..at + len],
            "row {i}: U+{:04X} decomposes differently",
            r[0]
        );
    }
}
