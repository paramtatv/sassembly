//! **A module nobody asks for, and the difference between a ROOT and a corpse.**
//! `W-311`'s `Next:`, answered by measurement.
//!
//! `W-311` measured `repertoire_violations` at 15,166 and found 88% of the
//! figure — 13,346 characters — in ONE file,
//! `crates/textapp/src/text/tables.t1`, a letter-by-letter transliteration of
//! English identifiers (`grapheme_break` spelled `गरअपहएमएबरएअक`). Its `Next:`
//! refused to edit that file and named the prior question instead: **is it
//! generated, hand-ported, or dead?** — warning that "rename it" would be the
//! wrong unit either way.
//!
//! ADR-0027 pins ONE direction — an import naming no declared module,
//! `IMPORTS_NAMING_NO_DECLARED_MODULE` in `t1_modules.rs`, three rows of it
//! `सअरणई`. **The opposite direction was never counted**: a module that IS
//! declared and that no source imports or reaches. Nothing in the tree could say
//! how many there are, so nothing could say `सारणी` is one.
//!
//! # The instrument has THREE states because the truth does
//!
//! The first cut of this file had two — reached, or not — and it reported SEVEN
//! unreached modules, two of them `शृङ्खला` (`shrinkhala.t1`) and `मुद्रण`
//! (`unparse.t1`). Those are not dead: `शृङ्खला` is the chain's own driver, the
//! `.t1` port of `chain.rs`'s stage sequence, and **nothing imports the top of a
//! chain**. It is entered from OUTSIDE the `.t1` corpus, by the host, through
//! `Interpreter::call` — 85 sites. A two-state instrument calls that module and
//! `tables.t1` the same thing, and the one real finding would have been filed
//! next to two false ones.
//!
//! So a declared module is in exactly one of:
//!
//! | state | a foreign `.t1` asks for it | the host calls into it |
//! | --- | --- | --- |
//! | `REACHED` | yes | — |
//! | `ENTERED` | no | yes — it is a ROOT |
//! | `DEAD` | no | no |
//!
//! # Why this is not `t1_modules.rs`'s loader
//!
//! It is a second reading of the same corpus, deliberately. `t1_modules.rs`
//! reads for `mandala::missing_imports` and `mandala::unknown_imports`, both of
//! which take the DECLARED set as their ground and ask about imports. This file
//! takes the ASKED-FOR set as its ground and asks about declarations, so a bug
//! in a shared helper would move both numbers the same way and neither would say
//! so. The two files must be able to disagree.

use sadhana::t1::mandala::{self, Spelling, Unit};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `crates/` with `ext`, sorted. `target/` is not under
/// `crates/`, so no build artefact can enter either set.
fn every_source(ext: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, ext, out);
            } else if p.extension().is_some_and(|x| x == ext) {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&repo_root().join("crates"), ext, &mut v);
    v.sort();
    v
}

fn rel(p: &Path) -> String {
    p.strip_prefix(repo_root())
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// `read_lossy`, not `read` — four of the five `crates/textapp/src/text/*.t1`
/// carry lines the T1 lexer refuses (5,607 of them in `tables.t1` alone, pinned
/// in `t1_modules.rs`). Dropping a whole file over one bad line would delete its
/// DECLARATION from the set, and the module it declares would then vanish from
/// this census instead of appearing in it — the exact misreading this file
/// exists to make impossible.
fn corpus() -> Vec<(String, Unit)> {
    every_source("t1")
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()));
            (rel(&p), mandala::read_lossy(&text).0)
        })
        .collect()
}

/// This file, excluded from its own scan. See [`host_call_targets`].
const SELF: &str = "crates/sadhana-t1/tests/t1_unreached_modules.rs";

/// Every `Interpreter::call` target named anywhere in the Rust tree, as written.
///
/// The host enters a `.t1` module by calling a QUALIFIED routine name — the
/// member-marked form passed to `Interpreter::call`. Nothing else in a `.rs`
/// file is an entry: a Devanagari name inside a synthetic source string is a
/// fixture, not a call, and five of the six `सारणीॱ` occurrences in the tree are
/// exactly that (`mandala.rs:506`'s `सारणीॱअङ्कः`, `relocatable.rs:490`'s
/// `सारणीॱउपरिन`). Matching on `.call(` is what separates them.
///
/// **[`SELF`] IS EXCLUDED, AND THAT IS NOT HOUSEKEEPING.** The first cut of this
/// file wrote the chain driver's entry point into a doc comment as an example,
/// and the scan read its own prose: `शृङ्खला` came back with 86 host calls where
/// the tree has 85, and the pinned row moved because a COMMENT was edited. A
/// source-scanning test is inside its own population unless it says otherwise.
/// The exclusion is asserted to be load-bearing in
/// `the_scan_excludes_itself_and_that_exclusion_is_load_bearing`, so it cannot
/// rot into a no-op after a rename.
fn host_call_targets() -> Vec<String> {
    let mut out = Vec::new();
    for p in every_source("rs") {
        if rel(&p) == SELF {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let mut rest = text.as_str();
        while let Some(at) = rest.find(".call(") {
            rest = &rest[at + ".call(".len()..];
            let head = rest.trim_start();
            let Some(body) = head.strip_prefix('"') else {
                continue;
            };
            if let Some(end) = body.find('"') {
                out.push(body[..end].to_string());
            }
        }
    }
    out.sort();
    out
}

/// Whether `target` is a call into `module` — **the member mark is the whole of
/// the test**.
///
/// `समावेशसारणीॱसारणीपूरणम्` ENDS in `सारणीॱसारणीपूरणम्`, and a substring match
/// would read the generated spec-table store's entry point as an entry into
/// `tables.t1`'s module and report the corpse alive. The qualified name is
/// `module` then the mark, from the START.
fn calls_into(target: &str, module: &str) -> bool {
    target
        .strip_prefix(module)
        .is_some_and(|rest| rest.starts_with('ॱ'))
}

/// What a declared module's callers say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    /// A foreign `.t1` imports it or reaches into it.
    Reached,
    /// No `.t1` asks for it, but the host calls a routine of it — a ROOT.
    Entered,
    /// Neither.
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Module {
    name: String,
    /// Every file that declares it, sorted. More than one is legal — `वास्तु`
    /// is one module in two files (`W-225`).
    declared_in: Vec<String>,
    state: State,
    /// Host `.call` sites naming a routine of it.
    host_calls: usize,
}

/// Classify every declared module.
///
/// **"Foreign" is the whole of the first judgement.** A module reaching its own
/// members is not an import and must not rescue it; a two-file module (`वास्तु`)
/// naming itself across the pair must not either, or a module could keep itself
/// alive. So a declarer's own imports and accesses are excluded and only another
/// file's count.
///
/// An access counts only where `mandala` recorded one, and it records a `ॱॱ`
/// label mark as no access at all — which is what keeps `samyojana.t1:74`'s
/// FIELD `सारणी ॱॱ अङ्कः` from reading as a reach into the module of the same
/// spelling. That case is asserted below rather than assumed.
fn classify(units: &[(String, Unit)], calls: &[String]) -> Vec<Module> {
    let mut modules: Vec<(String, Vec<String>)> = Vec::new();
    for (path, u) in units {
        if let Some(m) = &u.module {
            match modules.iter_mut().find(|(name, _)| *name == m.text) {
                Some((_, files)) => files.push(path.clone()),
                None => modules.push((m.text.clone(), vec![path.clone()])),
            }
        }
    }

    let mut out: Vec<Module> = Vec::new();
    for (name, mut declared_in) in modules {
        declared_in.sort();
        let asked = units.iter().any(|(path, u)| {
            !declared_in.contains(path)
                && (u.imports.iter().any(|i| i.name.text == name)
                    || u.accesses.iter().any(|a| a.head == name))
        });
        let host_calls = calls.iter().filter(|t| calls_into(t, &name)).count();
        let state = if asked {
            State::Reached
        } else if host_calls > 0 {
            State::Entered
        } else {
            State::Dead
        };
        out.push(Module {
            name,
            declared_in,
            state,
            host_calls,
        });
    }
    out.sort();
    out
}

/// **Every declared module no `.t1` asks for**, with the state that tells a ROOT
/// from a corpse: `(module, declaring file, state, host call sites)`.
///
/// The first THREE are asserted EXACT — not a ceiling, because a ceiling would
/// let a module fall out of use without anything saying so, which is the
/// failure this row exists to name.
///
/// **THE FOURTH IS REPORT-ONLY SINCE 2026-09-19** (owner ruling of 2026-09-13,
/// item 1). It is the site count WHEN LAST TAKEN, and a move prints a `NOTE`.
/// It reds nothing because it encoded nothing: a Rust test gaining or losing a
/// `.call("शृङ्खलाॱ…")` says nothing about whether any module is reached.
///
/// **AND IT WAS NOT MERELY STALE — IT WAS WRONG WHEN IT WAS WRITTEN.** `85`
/// arrived in `921d1a8c` (2026-09-18) into a tree that this file's own scan
/// reads as `90`; replaying the scan over `921d1a8c` AND over its parent
/// answers 90 both times, and `host_call_targets`, `calls_into` and `SELF` are
/// byte-identical between that commit and today, so the replay is the test. The
/// assertion has therefore NEVER passed since that commit — which is the
/// argument for the shape: a count nobody can keep true is a count that stops
/// being read, and it took `cargo test -p sadhana-t1` red with it.
///
/// **WHAT STILL REDS, so this is not a deletion.** `State` is DERIVED from this
/// number — `host_calls > 0` is the whole of `ENTERED` against `DEAD` — so a
/// root losing its last host call still fails the exact assertion above, at the
/// column that carries the meaning. Zero is the refusal; 85 against 90 is not.
///
/// The two `ENTERED` rows are the chain's roots and are the reason the enum has
/// three variants: `शृङ्खला` is the `.t1` port of `chain.rs`'s stage sequence
/// and nothing imports the top of a chain.
///
/// The five `DEAD` rows are one fact about one lane rather than five. Those
/// files were restored whole in the fork merge `e332932d` and have no other
/// commit; they import each other under transliterated spellings (`सअरणई`,
/// pinned three times in `t1_modules.rs`) and so reach nothing; the host has
/// never called into one; and nothing in `crates/sadhana-t1/src/` has ever named
/// one. **That is `W-311`'s answer for `tables.t1`: not generated (no banner,
/// no generator names it — asserted below), hand-ported, and DEAD.**
const UNASKED_MODULES: &[(&str, &str, State, usize)] = &[
    ("इडएनट", "crates/textapp/src/text/ident.t1", State::Dead, 0),
    ("एनएफसइ", "crates/textapp/src/text/nfc.t1", State::Dead, 0),
    (
        "नउमएरअल",
        "crates/textapp/src/text/numeral.t1",
        State::Dead,
        0,
    ),
    (
        "मुद्रण",
        "crates/sadhana-t1/src/unparse.t1",
        State::Entered,
        1,
    ),
    (
        "शृङ्खला",
        "crates/sadhana-t1/src/shrinkhala.t1",
        State::Entered,
        90,
    ),
    (
        "सएगमएनट",
        "crates/textapp/src/text/segment.t1",
        State::Dead,
        0,
    ),
    (
        "सारणी",
        "crates/textapp/src/text/tables.t1",
        State::Dead,
        0,
    ),
];

#[test]
#[ignore = "census: needs the full development repository's .t1 and tools/ census not in the public repository"]
fn every_declared_module_no_source_asks_for_is_named_and_says_whether_it_is_a_root() {
    let units = corpus();
    let calls = host_call_targets();
    let all = classify(&units, &calls);

    let unasked: Vec<&Module> = all.iter().filter(|m| m.state != State::Reached).collect();
    println!("METRIC t1_modules_declared {}", all.len());
    println!(
        "METRIC t1_modules_reached {}",
        all.iter().filter(|m| m.state == State::Reached).count()
    );
    println!(
        "METRIC t1_modules_entered_by_the_host {}",
        all.iter().filter(|m| m.state == State::Entered).count()
    );
    println!(
        "METRIC t1_modules_dead {}",
        all.iter().filter(|m| m.state == State::Dead).count()
    );
    println!("NOTE  host_call_targets {}", calls.len());
    for m in &unasked {
        println!(
            "NOTE  unasked {} {:?} host_calls {} in {}",
            m.name,
            m.state,
            m.host_calls,
            m.declared_in.join(" ")
        );
    }

    let found: Vec<(String, String, State)> = unasked
        .iter()
        .map(|m| (m.name.clone(), m.declared_in.join(" "), m.state))
        .collect();
    let expected: Vec<(String, String, State)> = UNASKED_MODULES
        .iter()
        .map(|(n, f, s, _)| ((*n).to_string(), (*f).to_string(), *s))
        .collect();

    assert_eq!(
        found, expected,
        "the set of declared-but-unasked-for modules changed. If one gained a \
         caller, drop its row above in the same commit; if one appeared, a \
         module fell out of use and that is the finding. The HOST-CALL COUNT is \
         not in this comparison and cannot be what moved it — see the margin on \
         `UNASKED_MODULES`; `ENTERED` against `DEAD` is still exact and it is \
         derived from that count being non-zero."
    );

    // The fourth column, report-only: a `NOTE` when it moves and no red. Read
    // off `UNASKED_MODULES` by NAME rather than by index, so it keeps saying the
    // truth if a row is added or the list is reordered.
    for m in &unasked {
        let Some((_, _, _, when_taken)) = UNASKED_MODULES.iter().find(|(n, _, _, _)| *n == m.name)
        else {
            continue;
        };
        if m.host_calls != *when_taken {
            println!(
                "NOTE  host_calls {} moved {} -> {} since 2026-09-19 \
                 (report-only; the state above is what is asserted)",
                m.name, when_taken, m.host_calls
            );
        }
    }
}

/// The census must be able to say NO, and on the corpus rather than on a fixture.
///
/// A predicate that answered "unasked" for everything would satisfy the row
/// above just as well, so the complement is asserted: the three states partition
/// the declared set, most of it is `REACHED`, and the host-call scan found the
/// tree's real call surface rather than nothing.
#[test]
fn the_three_states_partition_the_declared_set_and_none_of_them_is_vacuous() {
    let units = corpus();
    let calls = host_call_targets();
    let all = classify(&units, &calls);

    let reached = all.iter().filter(|m| m.state == State::Reached).count();
    let entered = all.iter().filter(|m| m.state == State::Entered).count();
    let dead = all.iter().filter(|m| m.state == State::Dead).count();
    assert_eq!(
        reached + entered + dead,
        all.len(),
        "a module fell out of the partition"
    );

    assert!(
        reached >= 10,
        "only {reached} of {} declared modules are reached — the census has \
         stopped seeing imports, which reports live modules as dead",
        all.len()
    );
    assert!(
        entered >= 1,
        "no module is entered by the host at all, so the ENTERED state is \
         vacuous and every root would be reported dead"
    );
    assert!(
        calls.len() >= 50,
        "the host-call scan found only {} targets; it has stopped reading \
         .call(\"…\") and every root would be reported dead",
        calls.len()
    );
    assert!(
        calls.iter().any(|t| calls_into(t, "शृङ्खला")),
        "the chain driver's own entry point is not among the host call targets"
    );
}

/// **The two cases that must be REFUSED.**
///
/// *A longer module name that ENDS in a shorter one.* The generated spec-table
/// store's entry point is `समावेशसारणीॱसारणीपूरणम्`, and it is run for real by
/// `t1_sarani.rs` and `t1_chain_tables.rs`. A substring match would read it as
/// an entry into `सारणी` and report `tables.t1`'s module alive — `29b1169a`
/// renamed that module to `समावेशसारणी` precisely BECAUSE `tables.t1` had taken
/// `सारणी`, so the collision is real and on disk.
///
/// *A field spelled like a module.* `crates/sadhana-t1/src/samyojana.t1:74` is
/// `सारणी ॱॱ अङ्कः अन्तः कोशॱसंज्ञा` — the FIELD `सारणी` before ADR-0027's label
/// mark, in a file that does not import the module of that spelling. A census
/// that split `ॱॱ` would read the line as a reach and drop the row above.
/// Asserted on the real line, not a fixture: a fixture would prove the lexer
/// and not the corpus.
#[test]
#[ignore = "census: needs the full development repository's .t1 and tools/ census not in the public repository"]
fn neither_a_longer_name_nor_a_field_of_the_same_spelling_rescues_a_dead_module() {
    assert!(
        calls_into("सारणीॱसारणीपूरणम्", "सारणी"),
        "the positive case is broken, so the refusals below prove nothing"
    );
    assert!(
        !calls_into("समावेशसारणीॱसारणीपूरणम्", "सारणी"),
        "a module name that ENDS in सारणी was read as a call into सारणी"
    );
    assert!(
        !calls_into("सारणीपूरणम्", "सारणी"),
        "a longer name with no member mark was read as a call into सारणी"
    );

    let units = corpus();
    let (_, samyojana) = units
        .iter()
        .find(|(p, _)| p == "crates/sadhana-t1/src/samyojana.t1")
        .expect("samyojana.t1 is in the corpus");
    assert!(
        !samyojana.imports_module("सारणी"),
        "samyojana.t1 now imports सारणी — this test's premise is gone"
    );
    let reaches: Vec<usize> = samyojana
        .accesses
        .iter()
        .filter(|a| a.head == "सारणी")
        .map(|a| a.line)
        .collect();
    assert_eq!(
        reaches,
        Vec::<usize>::new(),
        "the label mark ॱॱ was read as a member mark ॱ, so a field named सारणी \
         became a reach into the module सारणी"
    );
    // The positive control on the same file: the member marks that ARE there
    // were read. A reader that saw no marks at all would also report none above.
    assert!(
        samyojana
            .accesses
            .iter()
            .any(|a| a.spelling == Spelling::Unspaced),
        "samyojana.t1 records no unspaced access at all, so the run above \
         proves nothing"
    );

    let all = classify(&units, &host_call_targets());
    assert_eq!(
        all.iter().find(|m| m.name == "सारणी").map(|m| m.state),
        Some(State::Dead),
        "सारणी left the DEAD state without its row above being changed"
    );
}

/// **`tables.t1` is not GENERATED by this tree**, and the control is the file
/// that is.
///
/// `W-311`'s `Next:` asked whether `tables.t1` is generated, hand-ported or
/// dead. The rows above answer DEAD. This one answers NOT GENERATED, by the two
/// signals a generated `.t1` in this tree carries and these carry neither of: a
/// banner in the file, and a generator under `tools/` that names the path.
///
/// `crates/sadhana-t1/src/sarani.t1` is the control — the other very large
/// machine-produced `.t1`. It carries `॰ GENERATED by tools/mkspectables.py from
/// spec/ — DO NOT EDIT BY HAND.` and `tools/mkspectables.py` names its path, so
/// a check that could not find a banner or a generator anywhere fails here
/// first.
#[test]
#[ignore = "census: needs the full development repository's .t1 and tools/ census not in the public repository"]
fn the_textapp_text_sources_are_neither_bannered_nor_named_by_any_generator() {
    let banner = |p: &str| -> bool {
        let text = std::fs::read_to_string(repo_root().join(p))
            .unwrap_or_else(|e| panic!("{p} must be readable: {e}"));
        text.lines()
            .take(20)
            .any(|l| l.contains("GENERATED") || l.contains("DO NOT EDIT"))
    };

    let mut tools = Vec::new();
    let dir = repo_root().join("tools");
    for e in std::fs::read_dir(&dir)
        .expect("tools/ must be readable")
        .flatten()
    {
        if e.path().is_file() {
            tools.push(std::fs::read_to_string(e.path()).unwrap_or_default());
        }
    }
    assert!(tools.len() >= 20, "tools/ read as {} files", tools.len());
    let named_by_a_generator = |p: &str| -> bool { tools.iter().any(|t| t.contains(p)) };

    // The control, both halves.
    assert!(
        banner("crates/sadhana-t1/src/sarani.t1"),
        "sarani.t1 lost its generator banner, so the absences below prove nothing"
    );
    assert!(
        named_by_a_generator("sarani.t1"),
        "no file under tools/ names sarani.t1, so the absences below prove nothing"
    );

    let mut findings: Vec<(String, bool, bool)> = Vec::new();
    for p in every_source("t1") {
        let r = rel(&p);
        if r.starts_with("crates/textapp/src/text/") {
            findings.push((r.clone(), banner(&r), named_by_a_generator(&r)));
        }
    }
    findings.sort();
    println!("METRIC textapp_text_sources {}", findings.len());
    for (p, b, g) in &findings {
        println!("NOTE  provenance {p} banner {b} generator {g}");
    }

    assert_eq!(
        findings.len(),
        5,
        "crates/textapp/src/text/ is no longer five .t1 sources"
    );
    let claimed: Vec<&String> = findings
        .iter()
        .filter(|(_, b, g)| *b || *g)
        .map(|(p, _, _)| p)
        .collect();
    assert!(
        claimed.is_empty(),
        "a crates/textapp/src/text/ source now claims a generator: {claimed:?} — \
         it is no longer hand-ported and the margin above must be re-taken"
    );
}

/// **The scan excludes itself, and the exclusion is load-bearing.**
///
/// A test that greps the tree is in its own population. This one names a
/// qualified entry point in its own text — the line below is the proof, and it
/// is written here on PURPOSE so the exclusion has something to exclude. Without
/// the skip, `शृङ्खला`'s host-call count reads 91 against a tree that has 90
/// (re-taken 2026-09-19; it read 86 against 85 when this margin was written),
/// and editing a comment in this file moves a measurement about other files.
/// The assertion below is RELATIVE — `leaked == guarded + own_sites` — so it
/// does not move when the corpus does, and it did not go red when the pinned 85
/// did.
#[test]
fn the_scan_excludes_itself_and_that_exclusion_is_load_bearing() {
    // A RAW string, so this file really holds the bytes the scan looks for.
    // Written with `\"` it would hold a backslash instead and the guard would be
    // testing a needle that is not in any haystack.
    const NEEDLE: &str = r#".call("शृङ्खलाॱपठनम्""#;

    let own = std::fs::read_to_string(repo_root().join(SELF))
        .unwrap_or_else(|e| panic!("{SELF} must be readable: {e}"));
    assert!(
        own.contains(NEEDLE),
        "this file no longer names a qualified entry point, so its own exclusion \
         excludes nothing and the guard has rotted into a no-op"
    );

    let scanned = host_call_targets();
    let own_sites = own.matches(NEEDLE).count();
    assert!(own_sites >= 1);

    let mut unguarded = scanned.clone();
    for _ in 0..own_sites {
        unguarded.push("शृङ्खलाॱपठनम्".to_string());
    }
    let guarded = scanned.iter().filter(|t| calls_into(t, "शृङ्खला")).count();
    let leaked = unguarded.iter().filter(|t| calls_into(t, "शृङ्खला")).count();
    println!("NOTE  host_calls_guarded {guarded} unguarded {leaked}");
    assert_eq!(
        leaked,
        guarded + own_sites,
        "this file's own mentions did not change the count, so the exclusion is \
         untested rather than proven"
    );

    // And the file is really in the walk — a skip over a path that is not there
    // would pass the same way.
    assert!(
        every_source("rs").iter().any(|p| rel(p) == SELF),
        "{SELF} is not in the .rs walk, so skipping it proves nothing"
    );
}

/// **A module may not keep itself alive**, and neither may its other half.
///
/// `classify` excludes a declarer's own files before asking whether anything
/// asks for the module. **Nothing on the corpus exercises that**: the mutation
/// that deletes the guard — `!declared_in.contains(path)` — leaves all five
/// rows above green, because no `.t1` in this tree imports or reaches its own
/// module today. A guard no run can falsify is a comment.
///
/// So the case is handed to the classifier directly, built from real sources
/// through the real reader rather than from hand-filled [`Unit`] values, and it
/// is the two shapes that would matter:
///
/// - a module whose own file names it — a self-import must not rescue it;
/// - a module declared by TWO files where the second names it, which is
///   `वास्तु`'s shape (`W-225`, one module in two files). Either half naming the
///   pair's own name is still the module talking to itself.
///
/// The positive control is the same set with one foreign importer added: that
/// one, and only that one, must come back `REACHED`.
#[test]
fn a_module_that_only_its_own_files_name_is_still_unasked() {
    let unit = |src: &str| mandala::read_lossy(src).0;
    let m = |name: &str| format!("मण्डलम् {name} ॥\n");

    let selfish = "स्वाश्रयपरीक्षा";
    let paired = "युग्मपरीक्षा";

    let units: Vec<(String, Unit)> = vec![
        // One file that declares a module and imports its own name.
        (
            "a.t1".to_string(),
            unit(&format!("{}आयातः {selfish} ।\n", m(selfish))),
        ),
        // Two files that declare ONE module; the second names the pair's name.
        ("b1.t1".to_string(), unit(&m(paired))),
        (
            "b2.t1".to_string(),
            unit(&format!("{}आयातः {paired} ।\n", m(paired))),
        ),
    ];

    let all = classify(&units, &[]);
    let state = |name: &str| all.iter().find(|x| x.name == name).map(|x| x.state);
    assert_eq!(all.len(), 2, "b1.t1 and b2.t1 are ONE module: {all:?}");
    assert_eq!(
        state(selfish),
        Some(State::Dead),
        "a module's own file imported it and that counted as a caller"
    );
    assert_eq!(
        state(paired),
        Some(State::Dead),
        "one half of a two-file module named the pair and that counted as a caller"
    );

    // The control: the same set, plus ONE foreign file that imports each. If the
    // reader were not seeing imports at all, this would answer Dead too and the
    // refusals above would prove nothing.
    let mut with_callers = units.clone();
    with_callers.push((
        "c.t1".to_string(),
        unit(&format!(
            "{}आयातः {selfish} ।\nआयातः {paired} ।\n",
            m("आह्वायकपरीक्षा")
        )),
    ));
    let all = classify(&with_callers, &[]);
    let state = |name: &str| all.iter().find(|x| x.name == name).map(|x| x.state);
    assert_eq!(state(selfish), Some(State::Reached), "{all:?}");
    assert_eq!(state(paired), Some(State::Reached), "{all:?}");
    assert_eq!(
        state("आह्वायकपरीक्षा"),
        Some(State::Dead),
        "the caller itself is asked for by nobody and must stay Dead"
    );
}

/// **HAND-PORTED FROM WHERE**, measured rather than inferred.
///
/// The row above says `crates/textapp/src/text/*.t1` carry no generator. This
/// one says what they are a port OF, and it is not a guess: four of the five
/// stems — `ident`, `numeral`, `segment`, `tables` — are ALSO the stems of
/// `crates/sanskrit-text/src/*.rs`, and the fifth, `nfc.t1`, is that crate's
/// `normalize.rs` under the name of the function it wraps (`normalize::nfc`).
///
/// The names inside agree with the names outside. `numeral.t1` declares
/// `रअडइक्ष` with members `बइनअरय ओकटअल डएकइमअल हएक्षअडएकइमअल` and a second
/// enum `नउमएरअलएररओर` — `Radix` with `Binary Octal Decimal Hexadecimal`, and
/// `NumeralError`, both of which `crates/sanskrit-text/src/numeral.rs` defines.
/// That is a transliteration of an EXISTING Rust module, letter by letter, which
/// is why `W-311` measured 13,346 out-of-repertoire characters in one file.
///
/// **And the port is not even complete**, which is the part that settles what
/// deleting it would cost: `nfc.t1`'s `इसऽएनएफसइ` — `is_nfc` — is
/// `॰ TODO full NFC check` over `प्रत्यागमनम् सत्यम्`, a stub that answers TRUE
/// for every input, where `sanskrit_text::normalize::is_nfc` implements the
/// check. A dead module is one thing; a dead module whose routines are stubs of
/// live ones is not a port anybody is midway through.
#[test]
#[ignore = "census: needs the full development repository's .t1 and tools/ census not in the public repository"]
fn each_dead_textapp_source_is_a_transliteration_of_a_live_sanskrit_text_module() {
    let rust_stem = |stem: &str| -> bool {
        repo_root()
            .join("crates/sanskrit-text/src")
            .join(format!("{stem}.rs"))
            .is_file()
    };

    // The control: a stem that is NOT there must answer false, or the check
    // above is an `is_file` that always says yes.
    assert!(
        rust_stem("numeral"),
        "crates/sanskrit-text/src/numeral.rs is gone"
    );
    assert!(
        !rust_stem("nfc"),
        "crates/sanskrit-text/src/nfc.rs now exists, so the normalize.rs          exception below is stale"
    );

    let mut paired: Vec<(String, bool)> = Vec::new();
    for p in every_source("t1") {
        let r = rel(&p);
        if !r.starts_with("crates/textapp/src/text/") {
            continue;
        }
        let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
        paired.push((stem.clone(), rust_stem(&stem)));
    }
    paired.sort();
    for (stem, ok) in &paired {
        println!("NOTE  ported_from {stem}.t1 -> crates/sanskrit-text/src/{stem}.rs {ok}");
    }
    assert_eq!(
        paired
            .iter()
            .map(|(s, ok)| (s.as_str(), *ok))
            .collect::<Vec<_>>(),
        vec![
            ("ident", true),
            ("nfc", false),
            ("numeral", true),
            ("segment", true),
            ("tables", true),
        ],
        "the pairing with crates/sanskrit-text/src/ changed"
    );

    // `numeral.t1`'s transliterated type names are `numeral.rs`'s own.
    let t1 = std::fs::read_to_string(repo_root().join("crates/textapp/src/text/numeral.t1"))
        .expect("numeral.t1 must be readable");
    let rs = std::fs::read_to_string(repo_root().join("crates/sanskrit-text/src/numeral.rs"))
        .expect("numeral.rs must be readable");
    for (devanagari, latin) in [
        ("रअडइक्ष", "Radix"),
        ("बइनअरय", "Binary"),
        ("ओकटअल", "Octal"),
        ("डएकइमअल", "Decimal"),
        ("हएक्षअडएकइमअल", "Hexadecimal"),
        ("नउमएरअलएररओर", "NumeralError"),
    ] {
        assert!(
            t1.contains(devanagari),
            "numeral.t1 no longer writes {devanagari}"
        );
        assert!(rs.contains(latin), "numeral.rs no longer defines {latin}");
    }

    // The port is incomplete: `is_nfc` is a stub that answers true for anything.
    let nfc = std::fs::read_to_string(repo_root().join("crates/textapp/src/text/nfc.t1"))
        .expect("nfc.t1 must be readable");
    assert!(
        nfc.contains("इसऽएनएफसइ") && nfc.contains("TODO") && nfc.contains("सत्यम्"),
        "nfc.t1's is_nfc is no longer a TODO stub returning true — the margin          above must be re-taken"
    );
    let normalize =
        std::fs::read_to_string(repo_root().join("crates/sanskrit-text/src/normalize.rs"))
            .expect("normalize.rs must be readable");
    assert!(
        normalize.contains("pub fn is_nfc"),
        "sanskrit_text::normalize::is_nfc is gone, so the stub above is not a          stub OF anything"
    );
}
