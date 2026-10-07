//! `repertoire_violations` — the last `W-308` hole, measured.
//!
//! `BUILD.md` §10 has carried the target `repertoire_violations 0` since the
//! cycle-1 dashboard (`682ec312`) and **nothing has ever taken the number**.
//! `A-040` replaced the hand-typed dashboard with a harvest, this row had no
//! producer, and it silently stopped appearing; `W-308` found it by censusing
//! `TARGETS` against the tree and named it a hole. This file is its producer.
//!
//! # A count of violations is a count OVER something
//!
//! The hard part is not the predicate — [`sanskrit_text::in_repertoire`] has
//! implemented R-15-1 since the beginning and is used here rather than restated.
//! The hard part is the POPULATION, and the case that must be refused is a
//! population chosen to make the number zero. `crates/sanskrit-text/tests/
//! corpus_lexical.rs` already runs the gate, but only over the sixteen
//! retired-dialect programs in `tests/corpus/t1/` — files that pass by
//! construction. Counting those and calling it *the* violation count is the
//! shape `W-309` refused when it kept `Uncited` out of `Outside`.
//!
//! So the population is read off **doc 15 §4's own scope table**, which is the
//! repertoire rule read as data:
//!
//! | Category | Governed by R-15-1? |
//! |---|---|
//! | `.सस` source, all three tiers | **Yes** |
//! | Documentation, the research folder | No — Devanagari + IAST gloss |
//! | Foreign data the system consumes | No — "the rule governs what SANSOS
//!   writes, not what SANSOS reads" |
//!
//! and off §5, which puts "internal tooling identifiers in the **Rust bootstrap
//! tree**" outside the rule because "Rust code is not Sassembly source; it is
//! scaffolding that is deleted at S2".
//!
//! That makes the governed set the authored Sassembly source at every tier, by
//! extension and never by directory: `.सस` and `.sas` (the same T0/T1 source
//! under an ASCII file name — `spec/*.sas`, `spec/golden/*.sas`, `drafts/`)
//! and `.t1`. `.rs`, `.py`, `.sh`, `.toml`, `.md` and `.tsv` are §5 scaffolding
//! or §4 documentation and are NOT in the denominator — which is why the figure
//! is not simply "every character in the checkout".
//!
//! The file list comes from `git ls-files -z`: *tracked* is what *authored*
//! means here, it is the same set the commit hook reasons about, and `-z`
//! returns the path bytes raw, so a Devanagari file name is not octal-quoted
//! out of the population.
//!
//! # The margin is excluded, and the exclusion is counted rather than hidden
//!
//! R-15-1 as written governs "every character". Applied that way to this tree
//! the answer is 889,012, of which **873,846 sit after a `॰`** — the English
//! design margins every `.t1` in the corpus carries. This tree has already
//! ruled on that case once, in the `सारणी` generator's own R-15-1 assertion
//! (`SAS-014`, 2026-09-14): *"scoped to CODE and not margins, because that is
//! where the lexer applies it and every `.t1` in this tree carries English
//! after `॰`."* The lexer deletes the comment before any later stage sees it
//! (`spec/grammar-t1.ebnf:299`, `:398`), so a margin character reaches nothing.
//!
//! That is not only a note in the ledger — it is what the product does, and a
//! test in this tree runs it every time: `every_t1_source_lexes_and_parses`
//! (`crates/sadhana-t1/tests/t1_sources.rs:74`) refuses a `.t1` carrying a
//! character outside the repertoire (it is how `sarani.t1`'s first generated
//! form was caught, on a `#`), and all twenty-one sources pass it while every
//! one of them carries English after `॰`.
//!
//! This file follows that ruling and **prints the excluded count beside the
//! figure**, so the exclusion is a number a reader can argue with rather than a
//! silence. A string literal is NOT excluded: it is data the object carries, the
//! lexer keeps it, and R-15-1 governs it — it is reported apart so that the day
//! Latin appears inside `उक्तम् … इति` the figure says where.
//!
//! # Why the scanner knows about strings at all
//!
//! Because `॰` is legal inside one. `spec/grammar-t1.ebnf:317` states the stage
//! order — *"the lexer reads the string, and reads it BEFORE it strips the
//! comment. That is why a string may hold a space and a `॰`"* — and the corpus
//! uses it twice, for the newline escape: `tests/corpus/t1/पाठकर्म.सस:32` and
//! `tests/corpus/t1/मुख्यम्.सस:13` both write `उक्तम् नमस्ते॰न इति`. A
//! line-based classifier would call the rest of those lines a comment. On this
//! tree the two totals happen to agree — which is exactly why the agreement
//! must not be the evidence; [`the_comment_mark_inside_a_string_is_not_a_comment`]
//! hands the scanner the case instead.
//!
//! # One number was hiding a ruling and a defect
//!
//! The first cut answered 15,166 and stopped. `W-312`'s `Next:` named what that
//! total conceals: **1,724 of it sits in `spec/*.sas`, and every one of those
//! characters is the operand of a text directive** — `॥ आस्की BOOT-COUNTER-FRESH ॥`,
//! `॥ जाल VIRTIO-NET-OK ॥` — ASCII bytes a SANSOS program emits to a console or
//! a wire, written where doc 15 §4's "foreign data the system consumes" row and
//! its "`.सस` source" row both have a claim. `O-15-2` is open to the owner on
//! exactly that boundary, and NOTHING here rules on it: the headline figure is
//! unchanged and still counts all 15,166.
//!
//! What is takeable without a ruling is that the two populations stop being one
//! number, **and the line between them is not this file's invention — it is
//! already in the product**. `crates/sadhana/src/lex.rs:583` turns the
//! repertoire check OFF at the word `आस्की` and back on at the closing `॥`, and
//! at `:598` gives `जाल` a narrower exemption — `< > = " / - ! SPACE { } ; :`
//! and the ASCII letters, and nothing else. So the lexer ACCEPTS all ten
//! `spec/*.sas` files with their 1,724 Latin characters, and REFUSES
//! `crates/sadhana/src/t1/tests/repertoire.sas` and
//! `crates/textapp/src/text/*.t1`. One of those two populations is a pending
//! ruling; the other is 13,442 characters of work R-15-1 has outstanding. A
//! single figure could not tell a reader which of its numbers had moved.
//!
//! So [`Region`] gains the lexer's two text-directive regions, kept apart from
//! each other because their exemptions differ, and the verdict — refused, or
//! carried by a directive — is taken in [`census`] from the region rather than
//! baked into the scan. `crates/sadhana/tests/repertoire_text_directive_boundary.rs`
//! is the second reading: it takes the REAL lexer as ground over the same files
//! and asks whether it accepts them, so a mistake in the model below cannot
//! agree with itself.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sanskrit_text::in_repertoire;

/// U+0970 DEVANAGARI ABBREVIATION SIGN — comment to end of line
/// (`spec/grammar-t1.ebnf:398`, `spec/grammar-t0.ebnf:154`).
const COMMENT_MARK: char = '\u{0970}';

/// ADR-0017's string delimiters, "spoken … thus". The close is escaped by
/// doubling (ADR-0011): `इति इति` is the word itself.
const TEXT_OPEN: &str = "उक्तम्";
const TEXT_CLOSE: &str = "इति";

/// Doc 15 §4 row 1 — "`.सस` source, all three tiers". `.sas` is the same source
/// under an ASCII file name; `.t1` is the T1 tier. Everything else in the
/// checkout is §5 scaffolding, §4 documentation, or §4 foreign data.
const GOVERNED_EXTENSIONS: &[&str] = &["सस", "sas", "t1"];

/// U+0965 DEVANAGARI DOUBLE DANDA — the directive bracket. It CLOSES a text
/// directive's operand: `lex.rs:704` and `:750` clear the exemption on this
/// token, and the Latin after it is refused again (probed on the real lexer:
/// `॥ आस्की ABC ॥ DEF` reds on the `D`).
const DOUBLE_DANDA: char = '\u{0965}';

/// D-002f6 — the operand is ASCII the program emits, and `lex.rs:584` suspends
/// R-15-1 over it ENTIRELY until the closing `॥`.
const ASCII_DIRECTIVE: &str = "आस्की";

/// D-002f7 — the same shape with a NARROWER exemption (`lex.rs:598`).
const WEB_DIRECTIVE: &str = "जाल";

/// Exactly `lex.rs:600-616`'s list. A `जाल` operand character outside it is
/// still refused — probed: `॥ जाल A&B ॥` reds on the `&`, `॥ जाल A1B ॥` on the
/// `1` — so the web directive is NOT a second blanket exemption and must not be
/// counted as one.
fn web_directive_allows(ch: char) -> bool {
    matches!(
        ch,
        '<' | '>' | '=' | '"' | '/' | '-' | '!' | ' ' | '{' | '}' | ';' | ':'
    ) || ch.is_ascii_alphabetic()
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

/// Whether a tracked path is authored Sassembly source under doc 15 §4.
///
/// By EXTENSION. Not by directory: a `.t1` written anywhere is authored source,
/// and a population defined by the directories that happen to be clean today is
/// the lie this measurement exists to avoid.
fn is_governed(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| GOVERNED_EXTENSIONS.contains(&ext))
}

/// Which of the lexer's regions a character sits in, in the lexer's own order:
/// a string is recognised first, then a comment, then a text directive's
/// operand.
///
/// This says WHERE a character is and nothing about whether it is forgiven —
/// that verdict is [`census`]'s, because the two directive regions do not
/// forgive the same set and a scan that folded them together would report a
/// refused `&` as an exemption.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Region {
    /// Everything R-15-1 governs and the lexer keeps.
    Code,
    /// After a `॰`, to end of line. Deleted before any later stage.
    Comment,
    /// Inside `उक्तम् … इति`. Governed — reported apart, not excluded.
    Text,
    /// The operand of `आस्की`, to the closing `॥`. R-15-1 suspended over it.
    Ascii,
    /// The operand of `जाल`, to the closing `॥`. R-15-1 NARROWED, not
    /// suspended — see [`web_directive_allows`].
    Web,
}

fn is_layout(ch: char) -> bool {
    ch == ' ' || ch == '\t' || ch == '\n'
}

/// Whether `word` stands at `i` as a whole space-delimited word.
///
/// Delimited by SPACE/TAB/LF and nothing else — a Devanagari word boundary is
/// not a `\b`, and `नमस्ते॰न` is one word with a `॰` in it.
fn word_at(cs: &[char], i: usize, word: &[char]) -> bool {
    if i + word.len() > cs.len() || cs[i..i + word.len()] != *word {
        return false;
    }
    if i > 0 && !is_layout(cs[i - 1]) {
        return false;
    }
    let after = i + word.len();
    after == cs.len() || is_layout(cs[after])
}

/// Hand every character of `src` to `f` with the region it belongs to.
///
/// The text-directive state is a FLAG beside the region and not a region of its
/// own, because that is how `lex.rs` carries it: `in_ascii`/`in_jal` are set at
/// the directive word, survive a line break — probed, `॥ आस्की ABC\nDEF ॥`
/// lexes clean — and are cleared only by the `॥` token, while a `॰` inside the
/// operand still opens a comment because the line is stripped before the words
/// are looked at (probed: `॥ आस्की ABC ॰ GHI\nDEF ॥` lexes clean too).
fn scan(src: &str, mut f: impl FnMut(char, Region)) {
    let cs: Vec<char> = src.chars().collect();
    let open: Vec<char> = TEXT_OPEN.chars().collect();
    let close: Vec<char> = TEXT_CLOSE.chars().collect();
    let ascii_directive: Vec<char> = ASCII_DIRECTIVE.chars().collect();
    let web_directive: Vec<char> = WEB_DIRECTIVE.chars().collect();
    let mut region = Region::Code;
    // `None` outside a text directive; the operand's region inside one.
    let mut directive: Option<Region> = None;
    let mut i = 0;
    while i < cs.len() {
        // What plain source counts as right here. Inside a directive's operand
        // that is the operand's own region, and the directive word itself is
        // part of it — `lex.rs:584` sets the flag BEFORE the check the word
        // would otherwise face.
        let plain = directive.unwrap_or(Region::Code);
        match region {
            Region::Code => {
                if word_at(&cs, i, &open) {
                    for ch in &cs[i..i + open.len()] {
                        f(*ch, plain);
                    }
                    i += open.len();
                    region = Region::Text;
                    continue;
                }
                // A WHOLE word, by `word_at`: `lex.rs` compares `word ==
                // "आस्की"`, so `कआस्की` opens nothing — probed, and the real
                // lexer reds on the Latin after it.
                if word_at(&cs, i, &ascii_directive) {
                    directive = Some(Region::Ascii);
                    for ch in &cs[i..i + ascii_directive.len()] {
                        f(*ch, Region::Ascii);
                    }
                    i += ascii_directive.len();
                    continue;
                }
                if word_at(&cs, i, &web_directive) {
                    directive = Some(Region::Web);
                    for ch in &cs[i..i + web_directive.len()] {
                        f(*ch, Region::Web);
                    }
                    i += web_directive.len();
                    continue;
                }
                if cs[i] == DOUBLE_DANDA {
                    // The bracket that closes the operand is not part of it.
                    directive = None;
                    f(cs[i], Region::Code);
                    i += 1;
                    continue;
                }
                if cs[i] == COMMENT_MARK {
                    region = Region::Comment;
                }
            }
            Region::Comment => {
                if cs[i] == '\n' {
                    region = Region::Code;
                }
            }
            Region::Ascii | Region::Web => {
                unreachable!("a directive is a flag, not a region walked")
            }
            Region::Text => {
                if word_at(&cs, i, &close) {
                    let mut k = i + close.len();
                    while k < cs.len() && is_layout(cs[k]) {
                        k += 1;
                    }
                    if word_at(&cs, k, &close) {
                        // `इति इति` — the doubled close is the word itself and
                        // the string continues (ADR-0011).
                        for ch in &cs[i..k + close.len()] {
                            f(*ch, Region::Text);
                        }
                        i = k + close.len();
                        continue;
                    }
                    for ch in &cs[i..i + close.len()] {
                        f(*ch, plain);
                    }
                    i += close.len();
                    region = Region::Code;
                    continue;
                }
            }
        }
        f(
            cs[i],
            if region == Region::Code {
                plain
            } else {
                region
            },
        );
        i += 1;
    }
}

/// One file's violations, split the way the lexer splits them.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Split {
    /// Characters the lexer would REFUSE. Work R-15-1 has outstanding.
    refused: usize,
    /// Characters a text directive's operand carries and the lexer ACCEPTS.
    /// `O-15-2` is open on whether doc 15 §4 governs them at all.
    directive: usize,
}

impl Split {
    fn total(self) -> usize {
        self.refused + self.directive
    }
}

#[derive(Default, Debug)]
struct Census {
    /// Governed files read.
    files: usize,
    /// Violations the lexer keeps, outside a string and outside a directive.
    code: usize,
    /// Violations inside `उक्तम् … इति`. Governed, so part of the figure.
    text: usize,
    /// Violations after a `॰`. EXCLUDED from the figure and printed beside it.
    margin: usize,
    /// Violations inside `॥ आस्की … ॥`. In the figure, reported apart.
    ascii_directive: usize,
    /// Violations inside `॥ जाल … ॥` AND on that directive's narrower allow
    /// list. One outside it is refused and lands in `code`, which is the
    /// distinction a single "inside a directive" bucket would have lost.
    web_directive: usize,
    /// path -> its split, for the files that have any violation.
    violating: BTreeMap<String, Split>,
    /// `॰` characters standing inside a string — the count that says whether
    /// the string-aware scan was load-bearing on this tree.
    marks_inside_text: usize,
}

impl Census {
    /// `repertoire_violations` — what R-15-1 governs and the lexer keeps.
    ///
    /// UNCHANGED by the split. Dropping the directive term here would be taking
    /// `O-15-2`, which is the owner's to take and not a measurement's.
    fn violations(&self) -> usize {
        self.code + self.text + self.ascii_directive + self.web_directive
    }

    /// The half of the figure that is a DEFECT: the lexer refuses these today.
    fn refused(&self) -> usize {
        self.code + self.text
    }

    /// The half that is a PENDING RULING: a text directive carries these and
    /// the lexer accepts them.
    fn in_directives(&self) -> usize {
        self.ascii_directive + self.web_directive
    }
}

/// Census the governed sources. `None` when the population is EMPTY.
///
/// A count over nothing is not zero violations, and `repertoire_violations 0`
/// is precisely the hand-typed figure `A-040` deleted. Emitting no row is the
/// only honest answer to an empty denominator — `W-310`'s rule one level up.
fn census(sources: &[(String, String)]) -> Option<Census> {
    if sources.is_empty() {
        return None;
    }
    let mut c = Census {
        files: sources.len(),
        ..Census::default()
    };
    for (path, src) in sources {
        let mut here = Split::default();
        scan(src, |ch, region| {
            if ch == COMMENT_MARK && region == Region::Text {
                c.marks_inside_text += 1;
            }
            if in_repertoire(ch) {
                return;
            }
            match region {
                Region::Code => {
                    c.code += 1;
                    here.refused += 1;
                }
                Region::Text => {
                    c.text += 1;
                    here.refused += 1;
                }
                Region::Comment => c.margin += 1,
                Region::Ascii => {
                    c.ascii_directive += 1;
                    here.directive += 1;
                }
                // THE VERDICT, NOT THE POSITION. `जाल` forgives a named list
                // and nothing else, so a character outside it is refused even
                // though it stands inside the directive.
                Region::Web => {
                    if web_directive_allows(ch) {
                        c.web_directive += 1;
                        here.directive += 1;
                    } else {
                        c.code += 1;
                        here.refused += 1;
                    }
                }
            }
        });
        if here.total() > 0 {
            c.violating.insert(path.clone(), here);
        }
    }
    Some(c)
}

/// Keep only what doc 15 §4 governs. The handed-tree tests go through this too,
/// so the filter under test is the filter that runs.
fn population(files: &[(&str, &str)]) -> Vec<(String, String)> {
    files
        .iter()
        .filter(|(p, _)| is_governed(p))
        .map(|(p, s)| ((*p).to_string(), (*s).to_string()))
        .collect()
}

/// The authored Sassembly source this checkout tracks.
///
/// Panics rather than returning an empty list when `git` cannot answer: the
/// commit hook learned this one the expensive way — reporting "could not run"
/// as a clean result names a cause that does not exist.
fn tracked_sources(root: &Path) -> Vec<(String, String)> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files must run — without it NOTHING was measured");
    assert!(
        out.status.success(),
        "git ls-files failed ({}) — NOTHING was measured, and that is not a \
         count of zero violations",
        out.status
    );
    let listing = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut sources = Vec::new();
    for path in listing.split('\0').filter(|p| !p.is_empty()) {
        if !is_governed(path) {
            continue;
        }
        let text = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|e| panic!("{path} is tracked Sassembly source and unreadable: {e}"));
        sources.push((path.to_string(), text));
    }
    sources
}

#[test]
fn a_population_with_no_governed_source_names_no_figure() {
    // 0 of 0 renders as "no violations", which is the hand-typed zero A-040
    // deleted, and it would read as the goal being met.
    assert!(census(&[]).is_none());

    // The same thing said the way it actually arrives: a tree of nothing but
    // scaffolding and documentation. Every one of these carries Latin.
    let handed = population(&[
        ("docs/DEVELOPER-GUIDE.md", "every word of this is Latin"),
        ("crates/metrics/src/main.rs", "fn main() { let x = 1; }"),
        ("tools/gate.sh", "#!/bin/sh\nexit 0\n"),
        ("Cargo.toml", "[package]\nname = \"x\"\n"),
        ("spec/mnemonics.tsv", "add\tsub\n"),
    ]);
    assert!(handed.is_empty(), "none of those is governed source");
    assert!(
        census(&handed).is_none(),
        "a checkout with no Sassembly source has no violation count to report"
    );
}

#[test]
fn the_population_is_by_extension_and_never_by_directory() {
    // Three governed files in three unrelated places, four ungoverned ones
    // sitting exactly where the governed ones usually live.
    let latin = "ABC";
    let handed = population(&[
        ("some/unexpected/place/x.t1", latin),
        ("spec/y.sas", latin),
        ("tests/corpus/t1/\u{0915}.\u{0938}\u{0938}", latin),
        ("spec/notes.md", latin),
        ("crates/sadhana-t1/src/nirvahana.rs", latin),
        ("tools/gen.py", latin),
        ("spec/golden/15-word-copy.words", latin),
    ]);
    let c = census(&handed).expect("three governed files are a population");
    assert_eq!(c.files, 3, "population: {:?}", c.violating.keys());
    assert_eq!(
        c.violations(),
        9,
        "three Latin letters in each of three governed files"
    );

    // And the negative that makes the rule a rule rather than a list: a
    // directory name is never what decides.
    assert!(is_governed("anything/at/all.t1"));
    assert!(!is_governed("spec/golden/15-word-copy.words"));
    assert!(!is_governed("noextension"));
    assert!(!is_governed("x.t1.rs"));
}

#[test]
fn a_margin_is_excluded_and_the_code_beside_it_is_not() {
    let handed = population(&[(
        "a.t1",
        "\u{0915}\u{0916} ABC \u{0970} DEF GHI\n\u{0917}\u{0918} J\n",
    )]);
    let c = census(&handed).expect("one governed file");
    assert_eq!(c.code, 4, "ABC on the code side, and J on the next line");
    assert_eq!(c.margin, 6, "DEFGHI is margin");
    assert_eq!(c.text, 0);

    // The case that must be REFUSED: dropping the comment rule would answer 10
    // and call the tree's English design margins a violation of a rule the
    // lexer never applies to them. Dropping the EXCLUSION's visibility is the
    // other half — the margin count is printed, so 873,846 is arguable.
    assert_eq!(
        c.violations() + c.margin,
        10,
        "conservation: every violating character is in exactly one region"
    );

    // A comment runs to end of line and no further.
    let c = census(&population(&[("a.t1", "\u{0970} X\n Y\n")])).expect("one file");
    assert_eq!((c.code, c.margin), (1, 1));
}

#[test]
fn the_comment_mark_inside_a_string_is_not_a_comment() {
    // `उक्तम् नमस्ते॰न इति` — the corpus's newline escape, at
    // tests/corpus/t1/पाठकर्म.सस:32 and tests/corpus/t1/मुख्यम्.सस:13.
    // A line-based classifier calls the `X` a comment and answers 0.
    let handed = population(&[(
        "a.\u{0938}\u{0938}",
        "\u{0909}\u{0915}\u{094D}\u{0924}\u{092E}\u{094D} \
         \u{0928}\u{092E}\u{0938}\u{094D}\u{0924}\u{0947}\u{0970}\u{0928} \
         \u{0907}\u{0924}\u{093F} X\n",
    )]);
    let c = census(&handed).expect("one governed file");
    assert_eq!(c.code, 1, "the X stands after the string closed");
    assert_eq!(c.margin, 0, "there is no comment on that line");
    assert_eq!(c.marks_inside_text, 1);

    // The doubled close is the word itself (ADR-0011), so the string does not
    // end at the first `इति`: `उक्तम् W इति इति X इति Y`. W and X are both
    // inside it and only Y is code. Reading the doubled close as a close puts
    // X on the code side — and it answers `1` for the `Y` either way, which is
    // why the assertion has to name the SPLIT and not just the total.
    let c = census(&population(&[(
        "a.t1",
        "\u{0909}\u{0915}\u{094D}\u{0924}\u{092E}\u{094D} W \
         \u{0907}\u{0924}\u{093F} \u{0907}\u{0924}\u{093F} X \
         \u{0907}\u{0924}\u{093F} Y\n",
    )]))
    .expect("one governed file");
    assert_eq!(
        (c.code, c.text, c.margin),
        (1, 2, 0),
        "W and X are inside the string; only Y is code"
    );

    // And a string is GOVERNED: Latin inside `उक्तम् … इति` is in the figure,
    // counted apart so the report says where it is. Excluding it would be the
    // margin exemption smuggled one delimiter further.
    let c = census(&population(&[(
        "a.t1",
        "\u{0909}\u{0915}\u{094D}\u{0924}\u{092E}\u{094D} Z \u{0907}\u{0924}\u{093F}\n",
    )]))
    .expect("one governed file");
    assert_eq!((c.code, c.text), (0, 1));
    assert_eq!(c.violations(), 1);
    // NAMED and not merely counted (`W-306`) — a file whose only violations are
    // inside a string must still appear in the per-file report, or the figure
    // moves and nothing says which file moved it. A mutation that left the
    // total right and dropped the file from this map passed every other
    // assertion in this file.
    assert_eq!(
        c.violating.get("a.t1").copied(),
        Some(Split {
            refused: 1,
            directive: 0
        }),
        "the file is in the figure and not in the report: {:?}",
        c.violating
    );
}

/// A delimiter is a WORD, and the boundary is what makes it one.
///
/// Both halves of `word_at` are load-bearing and neither was pinned until a
/// mutation that deleted them stayed green. `उक्तम्` and `इति` are ordinary
/// Sanskrit words; a compound ending in one is not the delimiter, and the
/// scanner reading it as one would open or close a string in the middle of a
/// name — silently moving characters between `code` and `text` while every
/// total stayed right.
#[test]
fn a_delimiter_is_a_whole_word_on_both_sides() {
    // LEADING boundary: `कउक्तम्` is one word that ENDS in the open delimiter.
    // No string opens, so the Latin after it is code.
    let c = census(&population(&[(
        "a.t1",
        "\u{0915}\u{0909}\u{0915}\u{094D}\u{0924}\u{092E}\u{094D} Q\n",
    )]))
    .expect("one governed file");
    assert_eq!(
        (c.code, c.text),
        (1, 0),
        "a word ending in उक्तम् is not the open delimiter"
    );

    // TRAILING boundary: inside a string, `इतिक` BEGINS with the close
    // delimiter and is not it. The `Z` is still inside; the `Y` after the
    // standalone `इति` is not.
    let c = census(&population(&[(
        "a.t1",
        "\u{0909}\u{0915}\u{094D}\u{0924}\u{092E}\u{094D}          \u{0907}\u{0924}\u{093F}\u{0915} Z \u{0907}\u{0924}\u{093F} Y\n",
    )]))
    .expect("one governed file");
    assert_eq!(
        (c.code, c.text),
        (1, 1),
        "a word beginning with इति is not the close delimiter"
    );
}

/// `॥ आस्की … ॥` — the operand is the directive's, and the `॥` gives it back.
///
/// Every boundary below was probed against the REAL lexer before it was
/// modelled here, not read off `lex.rs` and assumed.
#[test]
fn a_text_directive_carries_its_operand_and_the_double_danda_ends_it() {
    let c = census(&population(&[("a.sas", "॥ आस्की ABC ॥ DEF\n")])).expect("one governed file");
    assert_eq!(
        (c.ascii_directive, c.code, c.margin),
        (3, 3, 0),
        "ABC is the operand; DEF stands after the directive closed"
    );
    // The figure is UNCHANGED by the split — six either way. What the split
    // adds is that three of them are a pending ruling and three are a defect.
    assert_eq!(c.violations(), 6);
    assert_eq!(
        c.violating.get("a.sas").copied(),
        Some(Split {
            refused: 3,
            directive: 3
        }),
        "the per-file report carries the split: {:?}",
        c.violating
    );

    // THE CASE THAT MUST BE REFUSED, and it is the whole reason this is a
    // region and not a per-line exemption: forgetting the `॥` would forgive
    // every Latin character to the end of the FILE. The real lexer reds on the
    // `D` here — probed — and so does this.
    assert!(c.refused() > 0, "DEF is not inside the directive");

    // A directive is opened by a WHOLE word. `lex.rs:583` compares
    // `word == "आस्की"`, so a compound ending in the name opens nothing —
    // probed: the real lexer reds on the `A` below.
    let c = census(&population(&[("a.sas", "कआस्की ABC ॥\n")])).expect("one governed file");
    assert_eq!(
        (c.ascii_directive, c.code),
        (0, 3),
        "a word ending in आस्की is not the directive"
    );
}

/// `जाल` forgives a NAMED LIST and not a character outside it.
///
/// A split that called everything inside a directive "exempt" would be the
/// margin exemption smuggled one bracket further, and it would report a
/// character the lexer really refuses as a pending ruling. Both cases below
/// were probed on the real lexer first: `॥ जाल A&B ॥` reds on the `&`,
/// `॥ जाल A1B ॥` on the `1`.
#[test]
fn the_web_directive_forgives_a_named_list_and_not_a_character_outside_it() {
    let c = census(&population(&[("a.sas", "॥ जाल A&B ॥\n")])).expect("one governed file");
    assert_eq!(
        (c.web_directive, c.code),
        (2, 1),
        "the letters are forgiven and the `&` is not"
    );
    assert_eq!(c.violations(), 3, "all three are still in the figure");
    assert_eq!(
        c.violating.get("a.sas").copied(),
        Some(Split {
            refused: 1,
            directive: 2
        })
    );

    // An ASCII DIGIT is not on the list either, and it is the likelier mistake:
    // `<`, `/` and `=` are there for markup and a digit looks like it belongs.
    let c = census(&population(&[("a.sas", "॥ जाल A1B ॥\n")])).expect("one governed file");
    assert_eq!((c.web_directive, c.code), (2, 1));

    // And the two directives are NOT one exemption. The same operand under
    // `आस्की` is forgiven whole, which is why they are counted apart.
    let c = census(&population(&[("a.sas", "॥ आस्की A&B ॥\n")])).expect("one governed file");
    assert_eq!((c.ascii_directive, c.web_directive, c.code), (3, 0, 0));

    // The list itself, at its edges — a `~` is ASCII punctuation and is NOT on
    // it, so "printable ASCII" is not the rule and must not be mistaken for it.
    assert!(web_directive_allows('<') && web_directive_allows('z') && web_directive_allows('-'));
    assert!(!web_directive_allows('~') && !web_directive_allows('7') && !web_directive_allows('&'));
}

/// The exemption is the LEXER'S FLAG, so it outlives the line — and a `॰`
/// inside it is still a margin, because the line is stripped before the words
/// are looked at. Both probed: each source below lexes clean on the real lexer.
#[test]
fn a_directive_outlives_the_line_and_a_margin_inside_it_is_still_a_margin() {
    let c = census(&population(&[("a.sas", "॥ आस्की ABC\nDEF ॥\n")])).expect("one governed file");
    assert_eq!(
        (c.ascii_directive, c.code, c.margin),
        (6, 0, 0),
        "the operand crosses the line break"
    );

    let c =
        census(&population(&[("a.sas", "॥ आस्की ABC ॰ GHI\nDEF ॥\n")])).expect("one governed file");
    assert_eq!(
        (c.ascii_directive, c.margin),
        (6, 3),
        "GHI is a comment even inside a directive; DEF is back in the operand"
    );

    // Conservation across all four regions, which is the property the whole
    // file rests on: no character is counted twice and none is dropped.
    assert_eq!(c.violations() + c.margin, 9);
}

#[test]
fn every_authored_sassembly_source_is_measured_against_r_15_1() {
    let root = root();
    let sources = tracked_sources(&root);
    let c = census(&sources).expect("this checkout tracks Sassembly source");

    // A floor, not a pin (the owner's 2026-09-13 ruling): a scan that reached
    // a handful of files would otherwise report a small honest-looking figure.
    assert!(
        c.files >= 150,
        "only {} governed sources found — the population walk is broken, and a \
         figure taken over a short population is the defect this file exists \
         to refuse",
        c.files
    );

    // Conservation, checked against a second reading of the same bytes: every
    // non-repertoire character is in exactly one region, so the regions cannot
    // quietly lose one. Counted independently of `scan`.
    let whole: usize = sources
        .iter()
        .map(|(_, s)| s.chars().filter(|ch| !in_repertoire(*ch)).count())
        .sum();
    assert_eq!(
        c.violations() + c.margin,
        whole,
        "the regions do not partition the file: code {} + text {} + ascii {} + \
         web {} + margin {} != {}",
        c.code,
        c.text,
        c.ascii_directive,
        c.web_directive,
        c.margin,
        whole
    );

    // NAMED, not counted (`W-306`'s rule) — a bare figure would leave "which
    // files" a question, and every one of these is a row of the work R-15-1
    // still has outstanding.
    // The two halves are conserved into the figure, so neither can be quietly
    // dropped: a split that does not add back up is two numbers about nothing.
    assert_eq!(
        c.refused() + c.in_directives(),
        c.violations(),
        "the split does not conserve the figure"
    );

    // BOTH ARMS OF THE SPLIT ARE ENTERED ON THIS TREE. A region no source
    // reaches is a comment, not a measurement — the `W-312` lesson about a
    // guard no run can falsify. These are floors and not pins (the owner's
    // 2026-09-13 ruling): the figures themselves are printed below.
    assert!(
        c.refused() > 0,
        "nothing is refused — either R-15-1 is met or the scan stopped"
    );
    assert!(
        c.ascii_directive > 0 && c.web_directive > 0,
        "one of the two directive arms was never entered: ascii {} web {}",
        c.ascii_directive,
        c.web_directive
    );

    // NAMED, not counted (`W-306`'s rule) — a bare figure would leave "which
    // files" a question, and every one of these is a row of the work R-15-1
    // still has outstanding. SPLIT per file too: the whole point is that a
    // reader can see which of the two numbers moved.
    for (path, n) in &c.violating {
        println!(
            "NOTE repertoire violations outside the margin: {} in {path} \
             ({} refused, {} carried by a text directive)",
            n.total(),
            n.refused,
            n.directive
        );
    }

    println!("METRIC repertoire_violations {}", c.violations());
    println!("NOTE repertoire_population {}", c.files);
    println!("NOTE repertoire_violating_files {}", c.violating.len());
    println!("NOTE repertoire_margin_excluded {}", c.margin);
    println!("NOTE repertoire_string_violations {}", c.text);
    println!(
        "NOTE repertoire_comment_marks_in_strings {}",
        c.marks_inside_text
    );
    // THE SPLIT `W-312` ASKED FOR. `repertoire_violations` above is unchanged
    // and rules on nothing; these say which half is a defect and which half is
    // `O-15-2`, still the owner's.
    println!("NOTE repertoire_violations_refused {}", c.refused());
    println!(
        "NOTE repertoire_violations_in_text_directives {}",
        c.in_directives()
    );
    println!("NOTE repertoire_directive_ascii {}", c.ascii_directive);
    println!("NOTE repertoire_directive_web {}", c.web_directive);
    println!(
        "NOTE repertoire_files_refusing {}",
        c.violating.values().filter(|n| n.refused > 0).count()
    );
}
