//! **`D-003`, THE T1 NATIVE HALF: DOES THE EMITTED TEXT DEPEND ON HASH ORDER?**
//!
//! `crates/sadhana/tests/d003_emit_is_hash_order_blind.rs` settled the T0 `.sas`
//! path — source to object to linked image — and closed with a number and a
//! gap. The number is ONE: of the 103 `HashMap`/`HashSet` mentions in
//! `crates/sadhana/src`, exactly one sits on the path `assemble` and
//! `assemble_object` actually call (`parse.rs:829`'s `जाल` dict, `insert` then
//! `get`, never iterated). **The gap is the other 102, which live under
//! `src/t1/` — the self-hosting chain's Rust side, which that file's probes
//! never enter.** This file is the probe for that path.
//!
//! # What the path is, and what it has that the `.sas` path does not
//!
//! `Front` drives `lex → parse → resolve → typecheck → build_ir` through the
//! `.t1` interpreter (`nirvahana.rs`, 23 mentions), then `riscv64::emit_module`
//! lowers the IR to Sassembly text (`riscv64.rs`, 31 mentions) by way of
//! `allocate_registers` (`regalloc.rs`, 9). So this path carries 63 of the 102
//! on its own, and the remaining 39 sit in `ir.rs`, `opt.rs`, `build.rs`,
//! `resolve.rs`, `typecheck.rs`, `comptime.rs`, `types.rs` and `chain.rs`.
//!
//! And it carries ONE container the `.sas` path has no analogue for:
//!
//! ```text
//!   ir.rs:542   pub blocks: HashMap<BlockId, Block>
//! ```
//!
//! A routine's basic blocks, keyed by id, **in a hash map** — and a block's
//! position in the emitted text IS its address. An emitter that wrote the
//! blocks in map order would emit a different routine on every run of the same
//! binary over the same source, which is the loudest form of the defect
//! `D-003` exists to exclude.
//!
//! # What the audit found BY CONTENT at that site, before the probe ran
//!
//! Every read of `func.blocks` in `riscv64.rs` is either SORTED FIRST or
//! order-blind by aggregation, and the three sorted ones say so in their own
//! lines:
//!
//! ```text
//!   :1034  emit_function      keys().collect() then sort_by_key(|b| b.0)   SORTED
//!   :961   verify             keys().collect() then sort                    SORTED
//!   :1531  check_branch_ranges keys().collect() then sort_by_key           SORTED
//!   :446   count_locals       .values() … (an aggregate)                   BLIND
//!   :825   read_elsewhere     .values().any(…)                             BLIND
//!   :1037  max_param          .values().flat_map(…).max()                  BLIND
//!   :1056  targeted           collected INTO a HashSet, membership only    BLIND
//!   :1681  module_allocates   .iter().any(…)                               BLIND
//!   :1887  entry param count  .values().flat_map(…).count()                BLIND
//! ```
//!
//! `.max()`, `.any()` and `.count()` are permutation-invariant, and a `HashSet`
//! asked only `contains` has no order to leak. That reading is an ARGUMENT,
//! which is the thing this project does not accept — and it is also bounded to
//! one file of eleven. So the test below does not rely on it. It makes the hash
//! order MOVE across the whole chain and asks whether the text moved with it.
//!
//! # Why repeating the chain inside one process is a hash-order probe
//!
//! `RandomState::new` draws from a thread-local pair and INCREMENTS one of them
//! per instance, so two maps built in one process from identical insertions
//! generally iterate differently. A fresh [`Front`] per round therefore reseeds
//! every map on the path, and comparing round 1 against round 0 is not a
//! tautology.
//!
//! That premise carries the whole file, so
//! [`the_hasher_reseeds_per_instance_in_this_crate`] MEASURES it here rather
//! than citing the sibling file's measurement — a workspace-wide fixed-seed
//! hasher, or a toolchain that stops reseeding, would make these assertions
//! pass for the wrong reason. **An instrument with two states where the truth
//! has three hides its own breakage**; the third state is *probe disarmed*.
//!
//! # BOTH LEAKS WERE PROVEN TO GO RED BY INJECTION, AT TWO DIFFERENT SITES
//!
//! A probe whose subject contains no hash iteration at all is green for the
//! wrong reason, and the reseeding control above cannot tell that apart from a
//! real absence. So the sweep was driven RED twice, each time by a leak
//! injected into a DIFFERENT module of this path, and each time reverted:
//!
//! 1. **`riscv64.rs:1035`, `order.sort_by_key(|b| b.0)` deleted** — the block
//!    emission order taken straight from `func.blocks.keys()`. RED on
//!    `ashtaka.t1` at sign 198, line 7: round 0 wrote the label line
//!    `अष्टकअष्टकनिषेधःपर्व२ॱॱ` where round 1 wrote `योगः स्थिर०म् अर्थ०न ०न ।`.
//!    A whole block moved.
//! 2. **`regalloc.rs:68`, `intervals.sort_by_key(|&(id, (start, _))| (start, id))`
//!    deleted** — the linear-scan order taken from `lifetimes`'s hash order,
//!    so hash order reaches REGISTER ASSIGNMENT. RED on the same source at
//!    sign 348, line 11, and the difference is a register and nothing else:
//!    `योगः स्थिर२म्` against `योगः स्थिर५म्`.
//!
//! The two fire at different signs on different lines, so this is not one
//! assertion catching one thing twice: the probe is sensitive to the EMITTER's
//! ordering and to the ALLOCATOR's independently. That matters because the
//! `D-003` row's own note says *"`t1/regalloc.rs` was read and is hash-order
//! blind by construction"* — a READ, which is an argument. Injection 2 turns it
//! into a measurement: if that construction were ever lost, this sweep says so.
//!
//! # The case that must still be refused
//!
//! [`the_comparator_sees_one_flipped_character`] asserts the comparison this
//! file performs would actually FAIL on a difference. Without it a comparator
//! that compared lengths, or nothing, would report agreement over every source
//! and read as proof of absence.
//!
//! # THREE STATES PER SOURCE, NOT TWO
//!
//! `lib.t1` declares no `मण्डलम्` — Rust's crate root lists modules and T1 has
//! no crate root — so *emitted* against *did not* would file it beside a source
//! the chain REFUSED. Those are different facts and only one is about the
//! compiler, so each source lands in a named variant and the run prints every
//! name it did not compare. **NO SILENT CAP.**
//!
//! # What this does NOT settle
//!
//! The row's title says *two machines* and this session has one. Same binary,
//! same host, varying hash seeds. Cross-host — endianness, pointer width,
//! locale, directory order — is untouched and `D-003` stays open for it.

use sadhana::encode::Target;
use sadhana::nidana::Language;
use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::riscv64;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// THIS FILE'S OWN LOADER. A new test gets its own; it does not borrow one.
fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.t1` the corpus ships, in name order, READ OFF THE DIRECTORY — a
/// hand-written list here would agree with itself while a source was missing
/// from it, and the claim this file makes is about the corpus and not about a
/// list.
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

/// How many sources this run is ALLOWED to skip because the operator narrowed
/// it, and the names it was narrowed to.
///
/// `D003_T1_CORPUS=ashtaka.t1,vastu.t1` runs those two and nothing else. The
/// whole sweep is 174 seconds — a fresh `Front` per round is 20 sources times
/// three loads of the front end — and an injection run that has to wait three
/// minutes to learn whether a probe is sensitive gets done once and then
/// stopped being done. **The narrowing is LOUD**: the run prints `NARROWED`
/// with the names, and [`floor`] drops to exactly the count asked for, so a
/// narrowed run can never be mistaken in a log for the full one.
fn narrowed_to() -> Option<Vec<String>> {
    let raw = std::env::var("D003_T1_CORPUS").ok()?;
    let names: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if names.is_empty() { None } else { Some(names) }
}

/// The number of sources that must compare, or the probe's green is a smaller
/// claim than its name makes.
///
/// **20 IS MEASURED AND NOT CHOSEN.** The corpus ships 21 `.t1` sources and
/// `lib.t1` declares no `मण्डलम्`; the chain refused NONE of the other twenty on
/// `d1493590`. So the floor is the whole compilable corpus and a source that
/// starts being refused takes this test red rather than quietly leaving the
/// sweep.
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

/// ONE round: a FRESH [`Front`] — which is what reseeds every map on the path —
/// driven source to Sassembly text.
///
/// Reusing one `Front` across rounds would save most of the wall clock and
/// destroy the probe: the maps would be the same instances, so the second
/// reading would be the first one's and every comparison would hold by
/// construction.
fn emit_once(path: &Path) -> Round {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    let Some(module) = module_name(&src) else {
        return Round::DeclaresNoModule;
    };
    let mut front = match Front::load(&spec_root()) {
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

/// The first character at which two texts differ, by CHARACTER and not by byte,
/// because every mnemonic on this path is Devanagari and a byte offset into
/// UTF-8 names no sign a reader can find.
///
/// Returns the offset, the line it falls on, and both lines — the one octet
/// that moved, not a dump of two 40,000-line texts.
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

/// THE CONTROL the assertions below rest on: are two fresh `HashMap`s in THIS
/// crate's test process actually seeded differently?
///
/// Measured here and not cited from the sibling file, because a fixed-seed
/// hasher would be a property of the build and this is a different build.
#[test]
fn the_hasher_reseeds_per_instance_in_this_crate() {
    fn order() -> Vec<u32> {
        let mut m: HashMap<u32, u32> = HashMap::new();
        for k in 0..64u32 {
            m.insert(k, k);
        }
        m.keys().copied().collect()
    }

    let first = order();
    let differs = (0..5).any(|_| order() != first);

    assert!(
        differs,
        "PROBE DISARMED: five freshly built HashMaps with identical insertions \
         all iterated in the SAME order, so this process does not reseed per \
         instance. A fresh `Front` per round therefore does NOT vary hash \
         order, and `the_t1_chain_emits_the_same_text_under_a_moved_hash_order` \
         proves nothing about `D-003`'s third absence on the T1 path. Fix the \
         probe — fork a child process per round — before trusting its green."
    );
}

/// The refused case: the comparator has to be able to see a difference.
///
/// Driven over a text with the shape the real one has — many lines, Devanagari
/// — so that a comparator working only on short ASCII would not pass here.
#[test]
fn the_comparator_sees_one_flipped_character() {
    let good: String = (0..200)
        .map(|i| format!("पर्व{i}ॱॱ\nनिवेशनम् क०म् क१म् {i}\n"))
        .collect();
    let mut bad = good.clone();
    // One sign, deep in the text, replaced by another of the same byte width.
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
    // And it names a plausible site rather than reporting the difference at 0.
    assert!(
        line > 250,
        "the divergence is reported deep in the text, not at 1"
    );
}

/// **THE ASSERTION.** Every corpus source, compiled through the whole T1 chain
/// three times in one process — three independently seeded sets of maps — must
/// give the same Sassembly text, and that text must assemble to the same
/// octets.
///
/// The second half is what makes this END TO END rather than front-half-only:
/// `.t1` source → IR → Sassembly text → object octets, which is the chain
/// `SASSEMBLY IS DONE WHEN` names, measured for one absence.
#[test]
fn the_t1_chain_emits_the_same_text_under_a_moved_hash_order() {
    const ROUNDS: usize = 3;

    let mut compared = 0usize;
    let mut octets_compared = 0usize;
    let mut no_module: Vec<String> = Vec::new();
    let mut refused: Vec<(String, String)> = Vec::new();

    if let Some(names) = narrowed_to() {
        println!("d003 T1 NARROWED by D003_T1_CORPUS to {names:?} — NOT the full sweep");
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

        let first = match emit_once(&path) {
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

        for round in 1..ROUNDS {
            let again = match emit_once(&path) {
                Round::Emitted(t) => t,
                // A source that compiled on round 0 and did not on round 1 is a
                // determinism finding of its own, and a louder one than a
                // differing octet.
                Round::Refused(why) => panic!(
                    "{name}: compiled on round 0 and was REFUSED on round \
                     {round} — the chain's VERDICT depends on hash order: {why}"
                ),
                Round::DeclaresNoModule => unreachable!(
                    "{name} declared a module on round 0; `module_name` reads \
                     the source text and cannot change"
                ),
            };

            if let Some((at, line, mine, theirs)) = first_divergence(&first, &again) {
                panic!(
                    "{name}: round {round} differs from round 0 at sign {at}, \
                     line {line} — the emitted text depends on hash iteration \
                     order\n  round 0: {mine}\n  round {round}: {theirs}"
                );
            }
            assert_eq!(
                first.chars().count(),
                again.chars().count(),
                "{name}: round {round} is {} signs against {} on round 0, and \
                 they agree on every sign both have — one text is a prefix of \
                 the other",
                again.chars().count(),
                first.chars().count()
            );
        }

        // END TO END: the same text through `assemble_object`, so the claim is
        // about OCTETS and not only about the text that precedes them.
        //
        // `Language::English` because the emitter writes the Sanskrit mnemonic
        // set; `Uncompressed` because compression is a second variable and this
        // probe holds one.
        let to_octets = |t: &str| {
            sadhana::assemble_object(
                t,
                Some("d003t1"),
                Target::Uncompressed,
                false,
                Language::English,
            )
        };
        if let Ok(a) = to_octets(&first) {
            let b = to_octets(&first).unwrap_or_else(|d| {
                panic!(
                    "{name}: assembled once and refused on the second: {} diagnostics",
                    d.len()
                )
            });
            assert_eq!(
                a.len(),
                b.len(),
                "{name}: the object is {} octets on the second assembly against {}",
                b.len(),
                a.len()
            );
            let at = a.iter().zip(&b).position(|(x, y)| x != y);
            assert!(
                at.is_none(),
                "{name}: the assembled object differs at octet {} within one \
                 process",
                at.unwrap()
            );
            octets_compared += 1;
        }

        compared += 1;
    }

    // NO SILENT CAP: say what was not covered, by name.
    println!(
        "METRIC d003_t1_sources_compared {compared}\n\
         METRIC d003_t1_rounds_each {ROUNDS}\n\
         METRIC d003_t1_objects_compared {octets_compared}\n\
         METRIC d003_t1_declares_no_module {}\n\
         METRIC d003_t1_refused {}",
        no_module.len(),
        refused.len()
    );
    for (n, why) in &refused {
        println!("d003 T1 NOT COMPARED (refused): {n} — {why}");
    }
    for n in &no_module {
        println!("d003 T1 NOT COMPARED (declares no module): {n}");
    }

    assert!(
        compared >= floor(),
        "only {compared} sources were compared and the floor is {}: {} declared \
         no module and {} were REFUSED ({:?}). A probe that silently stops \
         covering a source reports absence where it measured nothing.",
        floor(),
        no_module.len(),
        refused.len(),
        refused.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(
        octets_compared, compared,
        "{compared} sources emitted text but only {octets_compared} of those \
         texts assembled; the end-to-end half of this probe silently covered \
         less than the front half"
    );
}
