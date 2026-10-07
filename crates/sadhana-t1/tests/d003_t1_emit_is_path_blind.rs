//! **`D-003`, THE PATHS AXIS, MEASURED FOR THE T1 NATIVE CHAIN: THE SPEC ROOT
//! IS INERT — THE CHAIN EMITS THE SAME TEXT WITH NO SPEC ON DISK AT ALL.**
//!
//! The row's first reading of this axis was a grep: *"two
//! `env!(CARGO_MANIFEST_DIR)` uses (chain.rs:1488, nirvahana.rs:3282), both
//! locating `spec/` at dev time, neither emitted"* — a bounded search, which
//! is an ARGUMENT. This file is the measurement, and what it measured is a
//! stronger fact than the one it set out to check.
//!
//! # What was set out, and what was found
//!
//! The first draft of this probe copied `spec/` to a second absolute path and
//! compared the two emissions — the moved-checkout shape of the row's *two
//! machines*. Its disarm control (`Front::load` against a nonexistent root
//! must refuse) FIRED: **the front end loads with no spec tree on disk at
//! all.** The margin at `chain.rs:23` — *"four of the seven embed
//! `spec/*.tsv` through `समावेशः`, resolved at LOAD time"* — is STALE: all 14
//! `समावेशः` mentions across the seven [`FRONT_END`] sources are comments or
//! string literals (`lex.t1:682` and kin IMPLEMENT the construct for compiled
//! programs; none EXECUTES it), so `anita::resolve` never reaches
//! `anita.rs:419`'s `spec_root.join` on this path. The tables a compiled
//! source embeds travel as generated base-64 literals inside `sarani.t1` —
//! the spec-table store the margin at `chain.rs:167` describes — not as
//! run-time reads of `spec/`.
//!
//! So the probe pins the stronger invariant: the ONE path the chain is handed
//! reads NOTHING. Not "the path does not leak into the text" but "there is no
//! filesystem behind the emission at all", which is the whole paths axis for
//! this chain — a compile on a second machine cannot differ through a path it
//! never dereferences. `anita.rs`'s own header names the stake: *"a lexer
//! that opened files would put the filesystem inside every lexer test and
//! inside `D-003`'s determinism claim."*
//!
//! # PROVEN TO GO RED BY INJECTION
//!
//! A sweep like this is green for the wrong reason if the ghost root is
//! quietly tolerated rather than never consulted — a chain that reads the
//! spec WHEN PRESENT and falls back when absent would also pass a
//! ghost-loads check, but not this sweep, and that had to be shown rather
//! than argued. Injected into `Front::load`: a read of
//! `spec_root.join("aksara-widths.tsv")`, refusing on error — the defect
//! class itself, a chain that consults its root. Narrowed to `ashtaka.t1`,
//! the sweep went RED at the verdict comparison: *"compiled against the real
//! `spec/` and was REFUSED against the ghost."* Reverted; the sweep is
//! sensitive to the chain STARTING to touch the root, whichever round the
//! touch breaks.
//!
//! # If this file goes red on a real change
//!
//! A front-end source that starts executing a live embed, or a chain that
//! starts reading its root, makes the ghost round refuse — and that is the
//! correct alarm, not noise: the paths axis of `D-003` REOPENS at that
//! commit, and this probe must then grow the moved-copy comparison its first
//! draft had (two byte-identical spec trees at different absolute paths,
//! emissions compared to the sign).
//!
//! # The case that must still be refused
//!
//! [`the_comparator_sees_one_flipped_character`] asserts the comparison would
//! actually FAIL on a one-sign difference deep in a Devanagari text. Without
//! it a comparator that compared lengths, or nothing, would report agreement
//! over every source and read as proof of absence.
//!
//! # THREE STATES PER SOURCE, NOT TWO
//!
//! `lib.t1` declares no `मण्डलम्`, which is its own state and not a refusal.
//! Each source lands in a named variant and the run prints every name it did
//! not compare. **NO SILENT CAP.**
//!
//! # What this does NOT settle
//!
//! Same binary, same host, same byte order, same pointer width. The row's
//! *two machines* stays open for what a second host would vary that a missing
//! directory does not — endianness, libc, toolchain. What no longer stands
//! untested on one host is the axis the row's title names first: paths.

use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};

/// THIS FILE'S OWN LOADER. A new test gets its own; it does not borrow one.
fn canonical_spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// A root that names NOTHING — asserted absent before every use, because a
/// probe whose ghost directory exists is comparing two real trees.
fn ghost_spec_root() -> PathBuf {
    let ghost = std::env::temp_dir().join(format!(
        "d003-no-such-spec-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(
        !ghost.exists(),
        "{} exists; the pick of a nonexistent path failed, not the probe",
        ghost.display()
    );
    ghost
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.t1` the corpus ships, in name order, READ OFF THE DIRECTORY — a
/// hand-written list would agree with itself while a source was missing.
fn corpus_paths() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    assert!(
        v.len() >= 21,
        "the corpus is the input to this probe and it has shrunk to {} sources; \
         a probe over an empty corpus agrees with everything",
        v.len()
    );
    v
}

/// `D003_PATH_CORPUS=ashtaka.t1` narrows the sweep to the named sources — the
/// full run is two whole-chain compiles per source — and the narrowing is
/// LOUD: the run prints `NARROWED` and [`floor`] drops to the count asked
/// for, so a narrowed run can never be mistaken in a log for the full one.
fn narrowed_to() -> Option<Vec<String>> {
    let raw = std::env::var("D003_PATH_CORPUS").ok()?;
    let names: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if names.is_empty() { None } else { Some(names) }
}

/// **20 IS MEASURED AND NOT CHOSEN**: the corpus ships 21 `.t1` sources,
/// `lib.t1` declares no `मण्डलम्`, and the chain refuses none of the other
/// twenty. A source that starts being refused takes this red rather than
/// quietly leaving the sweep.
fn floor() -> usize {
    narrowed_to().map_or(20, |n| n.len())
}

/// What one round over one source came to. See the header: three states.
enum Round {
    /// The Sassembly text `riscv64::emit_module` wrote.
    Emitted(String),
    /// The source declares no module, so there is no name to compile it under.
    DeclaresNoModule,
    /// The chain refused it, with its own reason.
    Refused(String),
}

/// ONE round: a fresh [`Front`] loaded against `spec_root`, driven source to
/// Sassembly text. The root is the variable; everything else is held.
fn emit_against(spec_root: &Path, path: &Path) -> Round {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    let Some(module) = module_name(&src) else {
        return Round::DeclaresNoModule;
    };
    let mut front = match Front::load(spec_root) {
        Ok(f) => f,
        Err(e) => return Round::Refused(format!("the front end does not load: {e}")),
    };
    let text = (|| -> Result<String, String> {
        front.lex(&src)?;
        front.parse()?;
        front.resolve()?;
        front.typecheck()?;
        front.build_ir()?;
        let built = front.module(&module, None)?;
        riscv64::emit_module(&built).map_err(|e| format!("{e:?}"))
    })();
    match text {
        Ok(t) => Round::Emitted(t),
        Err(e) => Round::Refused(e),
    }
}

/// The first character at which two texts differ, by CHARACTER and not by
/// byte, because every mnemonic on this path is Devanagari and a byte offset
/// into UTF-8 names no sign a reader can find.
fn first_divergence(a: &str, b: &str) -> Option<(usize, usize, String, String)> {
    let at = a.chars().zip(b.chars()).position(|(x, y)| x != y).or({
        if a.chars().count() == b.chars().count() {
            None
        } else {
            Some(a.chars().count().min(b.chars().count()))
        }
    })?;
    let line = a.chars().take(at).filter(|c| *c == '\n').count();
    let pick = |t: &str| {
        t.lines()
            .nth(line)
            .unwrap_or("<past the last line>")
            .to_string()
    };
    Some((at, line + 1, pick(a), pick(b)))
}

/// The headline fact on its own, so a red here is one line and not a sweep:
/// the front end loads with NOTHING behind the root it is handed.
///
/// A red here is a REAL ALARM and not probe breakage — see the header: a live
/// embed has entered the front end or the chain has begun reading its root,
/// the paths axis of `D-003` reopens, and this file must grow the moved-copy
/// comparison described there.
#[test]
fn the_front_end_loads_with_no_spec_on_disk() {
    let ghost = ghost_spec_root();
    if let Err(e) = Front::load(&ghost) {
        panic!(
            "the front end REFUSED to load against a nonexistent spec root — \
             the chain has started reading through the one path it is handed, \
             and `D-003`'s paths axis is OPEN again at this commit. Extend \
             this probe to the moved-copy comparison its header describes. \
             The refusal: {e}"
        );
    }
}

/// The refused case: the comparator has to be able to see a difference, deep
/// in a many-line Devanagari text, with no length change to lean on.
#[test]
fn the_comparator_sees_one_flipped_character() {
    let good: String = (0..200)
        .map(|i| format!("पर्व{i}ॱॱ\nनिवेशनम् क०म् क१म् {i}\n"))
        .collect();
    let mut bad = good.clone();
    let at = bad.find("पर्व150ॱॱ").expect("the marker is in the text");
    bad.replace_range(at..at + "पर्व".len(), "शर्व");

    let seen = first_divergence(&good, &bad);
    assert!(
        seen.is_some(),
        "the comparator cannot distinguish two texts differing in ONE sign, so \
         every agreement it reports over the corpus is worthless"
    );
    let (_, line, _, _) = seen.unwrap();
    assert_eq!(
        good.chars().count(),
        bad.chars().count(),
        "the flip changed a sign, not a length — so this case proves the \
         comparator looks at content and not at a length"
    );
    assert!(
        line > 250,
        "the divergence is reported deep in the text, not at 1"
    );
}

/// **THE ASSERTION.** Every corpus source, compiled through the whole T1
/// chain against the real `spec/` and against a root that names NOTHING, must
/// come to the same verdict and emit the same Sassembly text to the sign.
///
/// This is deliberately stronger than a ghost-loads check alone: a chain that
/// read the spec when present and fell back when absent would pass that check
/// and fail this sweep, because whatever the read fed the emission would
/// differ between the rounds — and the injection in the header shows the
/// sweep catching exactly that class.
#[test]
fn the_t1_chain_emits_the_same_text_with_no_spec_on_disk() {
    let canonical = canonical_spec_root();
    assert!(
        canonical.join("grammar-t1.ebnf").exists(),
        "{} is not the spec tree, so the canonical round would measure nothing",
        canonical.display()
    );
    let ghost = ghost_spec_root();

    let mut compared = 0usize;
    let mut no_module: Vec<String> = Vec::new();
    let mut refused: Vec<(String, String)> = Vec::new();

    if let Some(names) = narrowed_to() {
        println!("d003 PATH NARROWED by D003_PATH_CORPUS to {names:?} — NOT the full sweep");
    }

    for path in corpus_paths() {
        let name = path
            .file_name()
            .expect("a corpus path has a file name")
            .to_string_lossy()
            .into_owned();

        if let Some(names) = narrowed_to()
            && !names.contains(&name)
        {
            continue;
        }

        let real = match emit_against(&canonical, &path) {
            Round::Emitted(t) => t,
            Round::DeclaresNoModule => {
                no_module.push(name);
                continue;
            }
            Round::Refused(why) => {
                refused.push((name, why));
                continue;
            }
        };

        let bare = match emit_against(&ghost, &path) {
            Round::Emitted(t) => t,
            Round::Refused(why) => panic!(
                "{name}: compiled against the real `spec/` and was REFUSED \
                 against the ghost — the chain reads through its spec root, \
                 and `D-003`'s paths axis is open again: {why}"
            ),
            Round::DeclaresNoModule => unreachable!(
                "{name} declared a module on the canonical round; \
                 `module_name` reads the source text and cannot change"
            ),
        };

        if let Some((at, line, mine, theirs)) = first_divergence(&real, &bare) {
            panic!(
                "{name}: the emission differs at sign {at}, line {line}, \
                 according to whether `spec/` exists on disk — the filesystem \
                 is inside the emitted text\n  with spec/: {mine}\n  without:    {theirs}"
            );
        }
        assert_eq!(
            real.chars().count(),
            bare.chars().count(),
            "{name}: {} signs without `spec/` against {} with it, agreeing on \
             every sign both have — one text is a prefix of the other",
            bare.chars().count(),
            real.chars().count()
        );

        compared += 1;
    }

    // NO SILENT CAP: say what was not covered, by name.
    println!(
        "METRIC d003_path_sources_compared {compared}\n\
         METRIC d003_path_declares_no_module {}\n\
         METRIC d003_path_refused {}",
        no_module.len(),
        refused.len()
    );
    for (n, why) in &refused {
        println!("d003 PATH NOT COMPARED (refused): {n} — {why}");
    }
    for n in &no_module {
        println!("d003 PATH NOT COMPARED (declares no module): {n}");
    }

    assert!(
        compared >= floor(),
        "only {compared} sources were compared and the floor is {}: {} \
         declared no module and {} were REFUSED ({:?}). A probe that silently \
         stops covering a source reports absence where it measured nothing.",
        floor(),
        no_module.len(),
        refused.len(),
        refused.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
    );
}
