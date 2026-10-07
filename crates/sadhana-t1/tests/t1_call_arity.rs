//! `W-246` LANE ADDITION — EVERY `it.call` SITE IN THE RUST SUITE, CHECKED
//! AGAINST THE DECLARATION IT NAMES.
//!
//! # The gap this closes
//!
//! The Rust tests drive the T1 compiler by NAME and ARITY:
//!
//! ```ignore
//! it.call("व्याकरॱकार्यक्रमपठनम्", vec![toks], FUEL)
//! ```
//!
//! Neither half of that is checked at compile time. The name is a string, the
//! arguments are a `Vec<Value>`, and `rustc` is satisfied by any of them. The
//! trunk records that `W-223`, giving two routines one more parameter, touched
//! 35 such sites; a site left behind would have compiled, run, and failed at
//! run time with a message about a value — if that test ran at all.
//!
//! The twin pin in `t1_sources.rs` does not cover this: it compares NAMES
//! between the tables and the sources and never an arity. `W-208`'s census
//! (`t1_paradigm_calls.rs`, statistic 17) does measure arity agreement, but
//! between a T1 CALL SITE and its T1 declaration — inside the corpus. The
//! calls that come from Rust are the ones no instrument reads, and they are
//! the ones a Rust refactor moves.
//!
//! # The instrument
//!
//! Load every `.t1` source into one image and ask the interpreter for its own
//! reading of each declaration (`Interpreter::routines`, `Routine::arity`) —
//! the same reading `Interpreter::call` dispatches on, so a site that agrees
//! with it is a site that will resolve. Then read every test file's text and
//! find each call whose name is a literal, count the arguments in its `vec![…]`
//! by bracket depth, and compare.
//!
//! # What it can and cannot see
//!
//! A site whose name is a variable or a `format!` — `store_int(it, f, args)`,
//! `it.call(&format!("घोषणासञ्चयॱ{f}"), …)` — carries no literal to check, and
//! the census counts those rather than passing over them: `paradigm_arity_
//! dynamic_sites` is the size of the blind spot, and it is reported next to
//! the number checked so the coverage cannot be read as complete.

use sadhana::t1::nirvahana::Interpreter;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn repo_root() -> PathBuf {
    crate_root()
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn spec_root() -> PathBuf {
    crate_root().join("../../spec")
}

/// Every file under `dir` with `ext`, sorted, as (name, contents).
fn files_in(dir: &Path, ext: &str) -> Vec<(String, String)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("read source");
            (p.file_name().unwrap().to_string_lossy().into_owned(), text)
        })
        .collect()
}

/// Every Rust test file that could drive the interpreter.
fn test_files() -> Vec<(String, String)> {
    let mut all = Vec::new();
    for dir in [
        "crates/sadhana-t1/tests",
        "crates/sadhana/tests",
        "crates/yantra/tests",
    ] {
        for (name, text) in files_in(&repo_root().join(dir), "rs") {
            all.push((format!("{dir}/{name}"), text));
        }
    }
    all
}

/// The interpreter's own reading of every declaration: `module ॱ name` to arity.
///
/// This is the map `Interpreter::call` resolves against, so it is the right
/// authority for what a call site must pass — not the source text, which is a
/// second reader that could itself be wrong.
fn declared_arities() -> BTreeMap<String, usize> {
    let mut sources = files_in(&crate_root().join("src"), "t1");
    // The ENTRY modules under spec/entry/ (W-302's `पाठसेतुः`) are loaded
    // beside the corpus by the tests that drive them, so their routines are
    // declarations a call site may name.
    sources.extend(files_in(&repo_root().join("spec/entry"), "t1"));
    let refs: Vec<(&str, &str)> = sources
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let it = Interpreter::load(&refs, &spec_root()).expect("every T1 source loads into one image");
    it.routines()
        .map(|r| (format!("{}ॱ{}", r.module, r.name), r.arity()))
        .collect()
}

/// One call site found in a test file's text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Site {
    file: String,
    line: usize,
    /// The qualified name as the site spells it, `मण्डलॱनाम`.
    name: String,
    /// Arguments counted in the `vec![…]` that follows the name.
    args: usize,
}

/// The number of top-level entries in a `vec![…]` whose body starts at `body`
/// (just past the `[`), or `None` when the brackets do not close.
///
/// Depth is counted over `[](){}` and both kinds of literal are skipped, so a
/// `vec![Value::Octets(Octets::new(b"a, b"))]` is one argument and not three.
///
/// COMMENTS ARE SKIPPED, and `W-306` is why. An argument list annotated one
/// line per parameter —
///
/// ```ignore
/// vec![
///     Value::Int(0),        // शर्तम् — location ०, no load
///     Value::Bool(relaxed), // विश्रम्भ
/// ],
/// ```
///
/// — read as code counts the comment's own comma as an eighth argument AND
/// lets the last comment's text re-arm `seen_value` after the trailing comma
/// cleared it, so a SEVEN-argument site measured NINE. Both halves of that
/// are this function's, not the site's: a comment separates no argument and
/// holds none.
fn vec_arity(body: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut args = 0usize;
    let mut seen_value = false;
    let mut chars = body.char_indices();
    while let Some((_, c)) = chars.next() {
        match c {
            '[' | '(' | '{' => {
                depth += 1;
                seen_value = true;
            }
            ')' | '}' => {
                depth = depth.checked_sub(1)?;
            }
            ']' if depth == 0 => {
                // A trailing comma closed the last argument already, so
                // `vec![a, b,]` is two and `vec![]` is none.
                return Some(if seen_value { args + 1 } else { args });
            }
            ']' => depth -= 1,
            ',' if depth == 0 => {
                // A trailing comma before the `]` closes no argument; the
                // `seen_value` flag below is cleared and the `]` arm adds
                // nothing when nothing follows.
                args += 1;
                seen_value = false;
            }
            '"' => {
                seen_value = true;
                // Skip the literal, honouring `\"`.
                let mut escaped = false;
                for (_, d) in chars.by_ref() {
                    if escaped {
                        escaped = false;
                    } else if d == '\\' {
                        escaped = true;
                    } else if d == '"' {
                        break;
                    }
                }
            }
            '\'' => {
                seen_value = true;
            }
            '/' => {
                // `//` to the newline, `/* … */` to its close; anything else
                // after a `/` is division or a path and is code.
                let mut peek = chars.clone();
                match peek.next() {
                    Some((_, '/')) => {
                        for (_, d) in chars.by_ref() {
                            if d == '\n' {
                                break;
                            }
                        }
                    }
                    Some((_, '*')) => {
                        chars.next();
                        let mut prev = '\0';
                        let mut closed = false;
                        for (_, d) in chars.by_ref() {
                            if prev == '*' && d == '/' {
                                closed = true;
                                break;
                            }
                            prev = d;
                        }
                        if !closed {
                            return None;
                        }
                    }
                    _ => seen_value = true,
                }
            }
            c if !c.is_whitespace() => seen_value = true,
            _ => {}
        }
    }
    None
}

/// The modules a test file DECLARES ITSELF, as `मण्डलम् नाम ॥` inside one of
/// its string literals.
///
/// Several tests build a synthetic module and run it — `प्रोब` for an appender
/// probe, `क`/`ख`/`ग` for import order, `परीक्षक` for a field check. Those
/// calls are not corpus calls and the corpus image cannot answer for them, so
/// they are counted apart rather than reported as names nobody declares. A
/// site naming a CORPUS module that does not exist still fails, which is the
/// case the ratchet is for.
fn local_modules(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("मण्डलम् ") {
        let at = from + rel + "मण्डलम् ".len();
        from = at;
        let rest = &text[at..];
        let name: String = rest
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '।' && *c != '॥')
            .collect();
        if !name.is_empty() && rest[name.len()..].trim_start().starts_with('॥') {
            out.insert(name);
        }
    }
    out
}

/// `text` with the body of every RAW STRING blanked out, newlines kept so
/// line numbers still hold.
///
/// Two files in the suite hold Rust code inside `r#"…"#` as a FIXTURE for
/// their own scanners — `paradigm_t0.rs`'s `SYNTHETIC` and this file's own
/// controls — and those fixtures contain call sites written wrong ON PURPOSE.
/// Reading them as real sites made the ratchet report four defects that were
/// four test fixtures. A site inside a raw string is not a call anyone makes.
fn strip_raw_strings(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'r' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'#' {
                j += 1;
            }
            let hashes = j - i - 1;
            if hashes > 0 && j < bytes.len() && bytes[j] == b'"' {
                let close = format!("\"{}", "#".repeat(hashes));
                if let Some(rel) = text[j + 1..].find(&close) {
                    out.push_str(&text[i..=j]);
                    for c in text[j + 1..j + 1 + rel].chars() {
                        out.push(if c == '\n' { '\n' } else { ' ' });
                    }
                    out.push_str(&close);
                    i = j + 1 + rel + close.len();
                    continue;
                }
            }
        }
        let c = text[i..].chars().next().expect("a char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Every call site in `text` whose name is a literal, and the number of sites
/// whose name is NOT — a variable, a `format!`, a helper's parameter.
fn sites_in(file: &str, text: &str) -> (Vec<Site>, usize) {
    let text = &strip_raw_strings(text);
    let mut out = Vec::new();
    let mut dynamic = 0usize;
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("call(") {
        let at = from + rel;
        from = at + "call(".len();
        // `call(` must start a token: `.call(`, `call(`, but not `recall(`.
        let before = text[..at].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let rest = text[from..].trim_start();
        // `call(it, "नाम", …)` — the helper form — puts the interpreter first.
        let rest = match rest.strip_prefix("it,") {
            Some(r) => r.trim_start(),
            None => match rest.strip_prefix("&mut it,") {
                Some(r) => r.trim_start(),
                None => rest,
            },
        };
        let Some(lit) = rest.strip_prefix('"') else {
            dynamic += 1;
            continue;
        };
        let Some(end) = lit.find('"') else {
            dynamic += 1;
            continue;
        };
        let name = &lit[..end];
        // A qualified T1 name and nothing built at run time.
        if !name.contains('ॱ') || name.contains('{') {
            dynamic += 1;
            continue;
        }
        let after = lit[end + 1..].trim_start();
        let Some(args) = after
            .strip_prefix(',')
            .map(str::trim_start)
            .and_then(|a| a.strip_prefix("vec!["))
            .and_then(vec_arity)
        else {
            dynamic += 1;
            continue;
        };
        out.push(Site {
            file: file.to_string(),
            line: text[..at].lines().count(),
            name: name.to_string(),
            args,
        });
    }
    (out, dynamic)
}

/// Every literal site of the suite, the sites whose name is built at run time,
/// and the sites naming a module the file declares itself.
fn all_sites() -> (Vec<Site>, usize, Vec<Site>) {
    let mut sites = Vec::new();
    let mut synthetic = Vec::new();
    let mut dynamic = 0usize;
    for (file, text) in test_files() {
        let local = local_modules(&text);
        let (s, d) = sites_in(&file, &text);
        for site in s {
            let module = site.name.split('ॱ').next().unwrap_or("").to_string();
            if local.contains(&module) {
                synthetic.push(site);
            } else {
                sites.push(site);
            }
        }
        dynamic += d;
    }
    (sites, dynamic, synthetic)
}

// ─────────────────────────────────────────────────────────────────────────
// The ratchet.
// ─────────────────────────────────────────────────────────────────────────

/// THE INSTRUMENT. Every literal call site names a declared routine and hands
/// it the number of arguments that routine declares.
///
/// A failure here is one of two things and the message says which: a site
/// naming a routine no module declares (a rename that missed a test), or a
/// site passing the wrong number (a signature change that missed a test).
#[test]
fn every_call_site_passes_the_arity_its_declaration_names() {
    let declared = declared_arities();
    assert!(
        declared.len() > 300,
        "the image holds the corpus's routines, not {}",
        declared.len()
    );
    let (sites, _, _) = all_sites();
    assert!(
        sites.len() > 100,
        "the scanner finds the suite's call sites, not {}",
        sites.len()
    );
    let mut unknown = Vec::new();
    let mut wrong = Vec::new();
    for s in &sites {
        match declared.get(&s.name) {
            None => unknown.push(format!("{}:{} calls `{}`", s.file, s.line, s.name)),
            Some(&n) if n != s.args => wrong.push(format!(
                "{}:{} calls `{}` with {} argument(s); it declares {n}",
                s.file, s.line, s.name, s.args
            )),
            Some(_) => {}
        }
    }
    assert!(
        unknown.is_empty(),
        "{} call site(s) name a routine no module declares:\n  {}",
        unknown.len(),
        unknown.join("\n  ")
    );
    assert!(
        wrong.is_empty(),
        "{} call site(s) disagree with the declaration's arity:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The tests that let the instrument see red.
// ─────────────────────────────────────────────────────────────────────────

/// THE FAIL-FIRST CONTROL, written before the ratchet above and kept: a site
/// with one argument too few is FOUND, and named with its line.
///
/// The synthetic text is scanned by the same `sites_in` the ratchet uses, so
/// this is the instrument seeing red and not a second instrument agreeing.
#[test]
fn a_site_that_passes_too_few_arguments_is_named() {
    let text = r#"
fn a() {
    it.call("व्याकरॱकार्यक्रमपठनम्", vec![toks], FUEL);
}
fn b() {
    it.call("व्याकरॱकार्यक्रमपठनम्", vec![], FUEL);
}
"#;
    let (sites, _) = sites_in("synthetic.rs", text);
    assert_eq!(sites.len(), 2, "both sites are found: {sites:?}");
    assert_eq!(sites[0].args, 1);
    assert_eq!(sites[1].args, 0);
    let declared: BTreeMap<String, usize> =
        [("व्याकरॱकार्यक्रमपठनम्".to_string(), 1)].into_iter().collect();
    let wrong: Vec<&Site> = sites
        .iter()
        .filter(|s| declared.get(&s.name) != Some(&s.args))
        .collect();
    assert_eq!(wrong.len(), 1, "exactly the second site is wrong");
    assert_eq!(wrong[0].line, 6, "and it is named by its line");
}

/// The argument counter is not a comma count: a literal, a nested call and a
/// nested `vec!` each hold commas that separate nothing.
#[test]
fn the_argument_counter_counts_arguments_and_not_commas() {
    for (body, want) in [
        ("]", 0),
        (" ]", 0),
        ("a]", 1),
        ("a, b]", 2),
        ("a, b,]", 2),
        ("Value::Octets(Octets::new(b\"a, b, c\"))]", 1),
        ("f(x, y), g(z)]", 2),
        ("vec![a, b], c]", 2),
        ("Value::Int(1), Value::Int(2), Value::Int(3)]", 3),
        // A comment's comma separates nothing, and a comment after the
        // trailing comma closes no argument. `W-306`'s two sites, minimised.
        ("a, // one, two\n b, // three\n]", 2),
        ("a, b, /* one, two */]", 2),
        // A comment is not the whole of a line: the code before it still counts.
        ("a, b // trailing\n]", 2),
        // And a lone `/` is division, not a comment.
        ("a / b, c]", 2),
    ] {
        assert_eq!(vec_arity(body), Some(want), "`vec![{body}` holds {want}");
    }
    assert_eq!(vec_arity("a, b"), None, "an unclosed vec is refused");
    assert_eq!(
        vec_arity("a, /* never closed ]"),
        None,
        "an unterminated block comment is refused, not read as a close"
    );
}

/// `W-306` — AN ARGUMENT LIST ANNOTATED ONE COMMENT PER PARAMETER IS READ AS
/// THE ARGUMENTS IT PASSES, AND A WRONG ONE IS STILL REFUSED.
///
/// This is the instrument's own red of 2026-09-30: `t1_relaxed_jump_measured`
/// and `t1_relaxed_jump_recorded` each pass SEVEN `Value`s to
/// `यन्त्रोत्सर्जनॱयन्त्रशाखावतरणम्` on seven commented lines, and both were
/// measured at NINE. The fixture below is that shape, and the second site in
/// it is the case that must STILL be named: same annotation, one argument
/// short. An instrument that bought silence by ignoring commented lists would
/// pass the first half of this test and fail the second.
#[test]
fn a_commented_argument_list_is_counted_by_its_arguments() {
    let text = r#"
fn right() {
    it.call(
        "यन्त्रोत्सर्जनॱयन्त्रशाखावतरणम्",
        vec![
            Value::Int(1),          // शाखापर्वम् — the BRANCHING block
            Value::Int(0),          // शर्तम् — location ०, no load
            Value::Int(2),          // तदा
            Value::Int(7),          // अन्यत्
            Value::Int(next_block), // अग्रिमम्
            Value::Int(0),          // संयोज्यम् — no folded comparison
            Value::Bool(relaxed),   // विश्रम्भ
        ],
        5_000_000,
    );
}
fn wrong() {
    it.call(
        "यन्त्रोत्सर्जनॱयन्त्रशाखावतरणम्",
        vec![
            Value::Int(1), // शाखापर्वम्
            Value::Int(0), // शर्तम् — location ०, no load
            Value::Int(2), // तदा
            Value::Int(7), // अन्यत्
            Value::Int(0), // संयोज्यम् — no folded comparison
            /* विश्रम्भ dropped, and this is the defect */
        ],
        5_000_000,
    );
}
"#;
    let (sites, _) = sites_in("synthetic.rs", text);
    assert_eq!(sites.len(), 2, "both sites are found: {sites:?}");
    assert_eq!(sites[0].args, 7, "seven `Value`s on seven commented lines");
    assert_eq!(sites[1].args, 5, "and five is five, not seven");

    let declared: BTreeMap<String, usize> = [("यन्त्रोत्सर्जनॱयन्त्रशाखावतरणम्".to_string(), 7)]
        .into_iter()
        .collect();
    let wrong: Vec<&Site> = sites
        .iter()
        .filter(|s| declared.get(&s.name) != Some(&s.args))
        .collect();
    assert_eq!(
        wrong.len(),
        1,
        "exactly the short site is refused: {wrong:?}"
    );
    assert_eq!(wrong[0].args, 5);
}

/// A call site written inside a raw-string FIXTURE is not a call site.
///
/// This is the rule that keeps the ratchet honest in both directions: the
/// fixtures in `paradigm_t0.rs` and in this file spell deliberately wrong
/// calls, and a ratchet that read them would be permanently red for reasons
/// that are not defects.
#[test]
fn a_call_inside_a_raw_string_fixture_is_not_a_call_site() {
    let text = "fn a() {\n    it.call(\"कॱख\", vec![x], 1);\n}\nconst F: &str = r#\"\n    it.call(\"कॱख\", vec![], 1);\n\"#;\n";
    let (sites, _) = sites_in("synthetic.rs", text);
    assert_eq!(sites.len(), 1, "only the real site is read: {sites:?}");
    assert_eq!(sites[0].args, 1);
    assert_eq!(sites[0].line, 2, "and its line survives the blanking");
}

/// A site whose name is built at run time is COUNTED, not silently skipped.
///
/// The blind spot has to be visible: if `format!` sites were passed over in
/// silence, a suite that moved every call behind a helper would report full
/// coverage of nothing.
#[test]
fn a_name_built_at_run_time_is_counted_as_unchecked() {
    let text = r#"
    it.call(&format!("घोषणासञ्चयॱ{f}"), args, 10);
    it.call(name, args, 10);
    it.call("घोषणासञ्चयॱसङ्ग्रहः", vec![], 10);
"#;
    let (sites, dynamic) = sites_in("synthetic.rs", text);
    assert_eq!(sites.len(), 1, "one literal site: {sites:?}");
    assert_eq!(dynamic, 2, "two sites carry no literal to check");
}

// ─────────────────────────────────────────────────────────────────────────
// The census.
// ─────────────────────────────────────────────────────────────────────────

/// How much of the suite's driving of the interpreter this instrument reads.
#[test]
#[ignore = "measurement"]
fn measure_corpus_call_arity() {
    let declared = declared_arities();
    let (sites, dynamic, synthetic) = all_sites();
    let named: BTreeSet<&str> = sites.iter().map(|s| s.name.as_str()).collect();
    let mut by_file: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &sites {
        *by_file.entry(s.file.as_str()).or_default() += 1;
    }
    println!("METRIC paradigm_arity_declared_routines {}", declared.len());
    println!("METRIC paradigm_arity_literal_sites {}", sites.len());
    println!("METRIC paradigm_arity_dynamic_sites {dynamic}");
    println!(
        "METRIC paradigm_arity_synthetic_module_sites {}",
        synthetic.len()
    );
    println!("METRIC paradigm_arity_routines_named {}", named.len());
    println!(
        "METRIC paradigm_arity_share_of_routines_driven {:.2}",
        named.len() as f64 * 100.0 / declared.len() as f64
    );
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for s in &sites {
        *histogram.entry(s.args).or_default() += 1;
    }
    for (args, n) in &histogram {
        println!("METRIC paradigm_arity_sites_with_{args}_arguments {n}");
    }
    for (file, n) in &by_file {
        println!("  {file}: {n} literal site(s)");
    }
    for name in &named {
        if !declared.contains_key(*name) {
            println!("  UNDECLARED {name}");
        }
    }
}
