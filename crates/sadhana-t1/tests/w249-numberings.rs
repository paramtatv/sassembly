//! **W-249: every per-program numbering, measured rather than argued.**
//!
//! THE CLASS: a counter restarts and the storage it indexes does not, so a
//! second program in one interpreter reads the first program's entries.
//!
//! The row's enumeration was done by reading. This file is the part that a
//! read cannot do: it RUNS two programs in ONE interpreter and reports, for
//! every module-level (counter, arena) pair the read found, the counter's
//! value and the arena's length after the SECOND program. Those two numbers
//! are the leak's precondition and are a fact rather than an inference:
//!
//!   len > cursor + 1  →  the first program's tail is still addressable.
//!
//! IT IS A PRECONDITION AND NOT THE LEAK. A retained tail is only read if
//! some site walks or guards by the arena's `दैर्घ्य` instead of by the
//! cursor — and a length guard proves an index is IN RANGE, never that the
//! slot was WRITTEN, because these arenas grow with holes. The corpus read
//! that accompanies this file found eleven `दैर्घ्य`-bounded sites on global
//! arenas and NO length-bounded walk; the walk that existed is the one the
//! extent closed. So this file measures the exposure and names it; it does
//! not claim every retained tail is a defect.
//!
//! THE SECOND PROGRAM IS DELIBERATELY THE SHORTER. A longer second program
//! overwrites every slot the first wrote and hides the retention, which is
//! exactly why the two known instances survived every census: one
//! interpreter per program, and nothing to retain.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Where a counter restarts, LOCATED rather than remembered: the unique
/// `<counter> भवति ०` in the named source. Panics unless there is exactly one —
/// a counter that restarts twice, or not at all, is a finding and not a caption.
fn restart_line(file: &str, counter: &str) -> String {
    let text = source(file);
    let needle = format!("{counter} भवति ०");
    let hits: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.trim().starts_with(&needle))
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "`{counter}` should restart at exactly one place in {file}; found {hits:?}. \
         Two restarts means the counter is cleared on two paths and this file's \
         model is wrong; none means it does not restart and it belongs in MONOTONIC."
    );
    format!("{file}:{}", hits[0])
}

fn load_all(names: &[&str]) -> Interpreter {
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// A counter's value, or `None` when the global is not an integer.
fn counter(it: &Interpreter, name: &str) -> Option<i128> {
    match it.global(name) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    }
}

/// An arena's length, or `None` when the global is not an arena. `Octets` is
/// answered too because two of the corpus's stores are byte runs.
fn arena_len(it: &Interpreter, name: &str) -> Option<usize> {
    match it.global(name) {
        Some(Value::Arena(a)) => Some(a.borrow().len()),
        Some(Value::Octets(o)) => Some(o.as_slice().len()),
        _ => None,
    }
}

/// Lex and parse one source in `it`, leaving every global where the pass put
/// it. Answers the parse's return — the program's declaration extent.
fn lex_and_parse(it: &mut Interpreter, src: &str) -> i128 {
    let toks = it
        .call("पदविभाग", vec![octets(src)], 50_000_000)
        .expect("lexes")
        .as_int()
        .unwrap_or(0);
    it.call("कार्यक्रमपठनम्", vec![Value::Int(toks)], 200_000_000)
        .expect("parses")
        .as_int()
        .unwrap_or(0)
}

/// Every (counter, arena) pair the corpus read found where the counter is a
/// module-level global that RESTARTS and the arena is a module-level global
/// it indexes. The name on the left is the T1 global as the interpreter
/// spells it — module-qualified, because two modules declare `दोषकोश`,
/// `आज्ञाकोश` and `वाक्यकोश` under the same short name.
const PAIRS: &[(&str, &str, &str)] = &[
    // (counter, arena, THE FILE the counter restarts in — NOT the line)
    //
    // **THE LINE NUMBERS WERE REMOVED BECAUSE ONE OF THEM ROTTED AND NOTHING
    // NOTICED.** `सञ्चयसंज्ञासूचकाङ्क` cited `artha.t1:1111`; that line is a bare
    // `इति` and the reset is at `:1612`. The other six were correct — this was
    // audited, not assumed — but the third field is interpolated into a PRINTED
    // REPORT and read by nothing, so a wrong value produced a false line and no
    // failure. A citation with no mechanism behind it is exactly the defect this
    // file exists to find, one level up.
    //
    // The line is now COMPUTED by `restart_line`, which locates `<counter> भवति ०`
    // in the named file and refuses unless it finds EXACTLY ONE. That turns the
    // decoration into an assertion: a counter restarting in two places, or in
    // none, now fails here instead of being described wrongly.
    ("घोषणासूचकाङ्क", "घोषणाकोश", "parse.t1"),
    ("दोषसूचकाङ्क", "दोषकोश", "parse.t1"),
    ("चिह्नकसूचकाङ्क", "चिह्नककोश", "lex.t1"),
    ("मण्डलसूचकाङ्क", "मण्डलकोश", "sanchaya.t1"),
    ("प्रविष्टिसूचकाङ्क", "प्रविष्टिकोश", "sanchaya.t1"),
    ("प्राचलप्रविष्टिसूचकाङ्क", "प्राचलप्रविष्टिकोश", "sanchaya.t1"),
    ("पाठसूचकाङ्क", "पाठकोश", "sanchaya.t1"),
    ("सञ्चयसंज्ञासूचकाङ्क", "सञ्चयसंज्ञाप्रविष्टयः", "artha.t1"),
];

/// Counters that do NOT restart, paired with the arena they key. These are the
/// control: a monotonic counter needs no clearing, and its arena's length
/// SHOULD track its cursor. If one of these ever shows len > cursor + 1 the
/// counter has begun restarting somewhere and this file should say so.
const MONOTONIC: &[(&str, &str)] = &[
    ("अभिव्यञ्जकसूचकाङ्क", "अभिव्यञ्जककोश"),
    ("वाक्यसूचकाङ्क", "वाक्यकोश"),
    ("प्राचलसूचकाङ्क", "प्राचलकोश"),
    ("अर्थप्रकारसूचकाङ्क", "अर्थप्रकारकोश"),
];

/// A LONG first program and a SHORT second one, in one interpreter.
///
/// The first declares six top-level names with parameters and bodies; the
/// second declares one. Every arena the first filled is therefore longer than
/// the second's cursor unless something truncates or clears it.
const LONG: &str = "मण्डलम् प्रथम ॥
सार्वजनिक वृत्तिः क आदाय अ ॱॱ न६४ ऽ आ ॱॱ न६४ ददाति न६४ आदि
    चरः ख ॱॱ न६४ भवति अ योगः आ ।
    ख ।
इति
सार्वजनिक वृत्तिः ग आदाय इ ॱॱ न६४ ददाति न६४ आदि
    चरः घ ॱॱ न६४ भवति इ योगः १ ।
    घ ।
इति
सार्वजनिक वृत्तिः ङ ददाति न६४ आदि
    चरः च ॱॱ न६४ भवति २ ।
    च ।
इति
सार्वजनिक वृत्तिः छ ददाति न६४ आदि
    चरः ज ॱॱ न६४ भवति ३ ।
    ज ।
इति
सार्वजनिक वृत्तिः झ ददाति न६४ आदि
    चरः ञ ॱॱ न६४ भवति ४ ।
    ञ ।
इति
सार्वजनिक वृत्तिः ट ददाति न६४ आदि
    चरः ठ ॱॱ न६४ भवति ५ ।
    ठ ।
इति
";

const SHORT: &str = "मण्डलम् द्वितीय ॥
सार्वजनिक वृत्तिः ड ददाति न६४ आदि
    चरः ढ ॱॱ न६४ भवति ६ ।
    ढ ।
इति
";

/// **THE MEASUREMENT.** Two programs, one interpreter, every pair reported.
///
/// This test does not fail on a retained tail — a retained tail is the
/// corpus's normal condition for a monotonic arena and is only a defect where
/// something reads past the cursor. It fails only if the measurement itself
/// is vacuous: if the first program did not actually fill the arenas, every
/// number below would be zero and the report would prove nothing.
#[test]
fn a_second_program_leaves_the_first_programs_tail_addressable() {
    let mut it = load_all(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "artha.t1",
        "sanchaya.t1",
        "sanskrit_text.t1",
    ]);

    let long_extent = lex_and_parse(&mut it, LONG);
    assert!(
        long_extent >= 6,
        "the first program must declare at least six names or this measurement is \
         vacuous — the extent came back {long_extent}"
    );

    // Every arena's length AFTER the long program, so the second program's
    // numbers can be read against something.
    let after_long: Vec<Option<usize>> = PAIRS
        .iter()
        .map(|(_, arena, _)| arena_len(&it, arena))
        .collect();

    let short_extent = lex_and_parse(&mut it, SHORT);
    assert_eq!(
        short_extent, 1,
        "the second program declares exactly one name, so its extent must be १ — \
         a different number means `घोषणासूचकाङ्क` did not restart and the whole \
         class this row searches for is not what it is described to be"
    );

    let mut retained = Vec::new();
    let mut report = String::new();
    report.push_str(
        "\nW-249 — two programs, one interpreter. `cursor` is the counter after \
         the SHORT program; `len` is the arena's length.\n\n",
    );
    for (i, (ctr, arena, site)) in PAIRS.iter().enumerate() {
        let c = counter(&it, ctr);
        let l = arena_len(&it, arena);
        let was = after_long[i];
        let verdict = match (c, l) {
            (Some(c), Some(l)) => {
                // One-based arenas: a cursor of n means slots १..=n are this
                // program's, and length n+1 is exactly that and no more.
                let over = l as i128 - (c + 1);
                if over > 0 {
                    retained.push((*ctr, *arena, over));
                    format!("RETAINED {over} slot(s) beyond this program's cursor")
                } else {
                    "no tail — length tracks the cursor".to_string()
                }
            }
            (None, _) => "counter is not an Int global here".to_string(),
            (_, None) => "arena is not an Arena/Octets global here".to_string(),
        };
        report.push_str(&format!(
            "  {ctr}\n    keys   {arena}   (restarts at {})\n    \
             cursor {c:?}  len {l:?}  (len after the long program: {was:?})\n    {verdict}\n\n",
            restart_line(site, ctr)
        ));
    }

    report.push_str("  ── monotonic controls (counters that never restart) ──\n");
    for (ctr, arena) in MONOTONIC {
        let c = counter(&it, ctr);
        let l = arena_len(&it, arena);
        report.push_str(&format!("  {ctr} = {c:?}   {arena} len {l:?}\n"));
    }

    report.push_str(&format!(
        "\nMETRIC w249_pairs_measured {}\nMETRIC w249_pairs_retaining_a_tail {}\n",
        PAIRS.len(),
        retained.len()
    ));
    for (ctr, arena, over) in &retained {
        report.push_str(&format!("METRIC w249_retained {ctr} {arena} {over}\n"));
    }
    println!("{report}");

    // THE ONE ASSERTION, and it is about the INSTRUMENT rather than the corpus:
    // if nothing at all retained a tail the two programs did not differ in size
    // and every line above is a tautology.
    assert!(
        !retained.is_empty(),
        "no arena retained anything, so the long and short programs did not \
         actually differ in what they filled — the measurement is vacuous.{report}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// A FIFTH FORM, FOUND BY THIS ROW: TWO MODULES' NUMBERINGS ON ONE SLOT.
// ─────────────────────────────────────────────────────────────────────────

/// Every `.t1` of the corpus, so the collision below is measured in the image
/// that actually has it — `load_every_source_with_vakyavibhaga`'s image.
fn corpus_sources() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the corpus directory is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".t1"))
        .collect();
    names.sort();
    names
}

fn load_corpus() -> Interpreter {
    let names = corpus_sources();
    let texts: Vec<(String, String)> = names.iter().map(|n| (n.clone(), source(n))).collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("corpus loads: {e:?}"))
}

/// **`वाक्यविभागॱआरम्भः` RESTARTS `वास्तु`'s, `मध्यरूप`'s AND `व्याकर`'s
/// COUNTERS, BECAUSE ALL FOUR MODULES SHARE ONE SLOT.**
///
/// W-249. THE FORWARD SHAPE FROM A DIRECTION THE ROW DID NOT NAME. Its three
/// recorded forms are all WITHIN one module: a numbering restarts and its own
/// arena does not; a counter fails to restart while what it counts does; a
/// per-program number escapes the program that minted it. This is a fourth:
/// TWO MODULES DECLARE A GLOBAL OF THE SAME NAME, the interpreter keys globals
/// by the BARE name, and one module's reset therefore restarts the other's
/// numbering over the other's still-full arena.
///
/// IT IS NOT A SURPRISE TO THE INTERPRETER — `nirvahana.rs:1292` states the
/// limit in its own margin ("two modules declaring a global of the same name
/// share one slot") and calls qualifying the key a corpus-wide change. What
/// was not recorded is that the corpus HAS such collisions, that four of them
/// are counter/arena PAIRS, and that the colliding slots hold DIFFERENT RECORD
/// TYPES:
///
/// | name              | declared in                    | element type          |
/// |-------------------|--------------------------------|-----------------------|
/// | `वाक्यसूचकाङ्क`/`वाक्यकोश`   | `ast.t1:225,228` + `vakyavibhaga.t1:402,403` | `वास्तुॱवाक्य` vs `वाक्यविभागॱवाक्य` |
/// | `आज्ञासूचकाङ्क`/`आज्ञाकोश`   | `ir.t1:236,237` + `vakyavibhaga.t1:394,395`  | `मध्यरूपॱआज्ञा` vs `वाक्यविभागॱआज्ञा` |
/// | `दोषसूचकाङ्क`/`दोषकोश`      | `parse.t1:165,166` + `vakyavibhaga.t1:406,407` | `व्याकरदोष` vs `वाक्यदोष` |
/// | `दत्तकोष्ठकम्`, `पाठकोष्ठकम्` | `encode.t1` + `samyojana.t1`   | (constants) |
///
/// `वाक्यसूचकाङ्क` is the sharpest of the four. `ast.t1` NEVER resets it — the
/// parser's statement arena is monotonic, which is the OTHER sound answer to
/// this whole class and the reason the file needs no clearing. Loading
/// `vakyavibhaga.t1` beside it takes that away: `आरम्भः` restarts the counter
/// while `वाक्यकोश` keeps every statement, and a monotonic arena becomes a
/// leaking one without a line of `ast.t1` changing.
///
/// THE CONTROL IS WHAT MAKES THIS A PROOF. `अभिव्यञ्जकसूचकाङ्क` is declared by
/// `ast.t1` ALONE and is not in `आरम्भः`'s reset list, so it must come through
/// unchanged. Without it this test would equally pass if `आरम्भः` zeroed every
/// global in the image, which would be a different defect and not this one.
#[test]
fn one_modules_reset_restarts_another_modules_numbering() {
    let mut it = load_corpus();

    // Fill the parser's arenas: statements into `वाक्यकोश`, expressions into
    // `अभिव्यञ्जककोश`, declarations into `घोषणाकोश`.
    let extent = lex_and_parse(&mut it, LONG);
    assert!(extent >= 6, "the program must parse or nothing is filled");

    let stmts_before = counter(&it, "वाक्यसूचकाङ्क").expect("ast's statement cursor");
    let exprs_before = counter(&it, "अभिव्यञ्जकसूचकाङ्क").expect("ast's expression cursor");
    let arena_before = arena_len(&it, "वाक्यकोश").expect("ast's statement arena");
    assert!(
        stmts_before > 0 && exprs_before > 0,
        "the parse must have numbered statements ({stmts_before}) and expressions \
         ({exprs_before}), or the collision below has nothing to destroy"
    );

    // `वाक्यविभाग`'s per-program reset — nothing to do with `वास्तु`.
    it.call("वाक्यविभागॱआरम्भः", vec![], 50_000_000)
        .expect("वाक्यविभागॱआरम्भः runs");

    let stmts_after = counter(&it, "वाक्यसूचकाङ्क").expect("still an Int");
    let own_after = counter(&it, "वाक्यविभागवाक्यसूचकाङ्क").expect("still an Int");
    let exprs_after = counter(&it, "अभिव्यञ्जकसूचकाङ्क").expect("still an Int");
    let arena_after = arena_len(&it, "वाक्यकोश").expect("still an arena");

    // THE CONTROL FIRST, so a failure below cannot be read as "आरम्भः zeroes
    // everything".
    assert_eq!(
        exprs_after, exprs_before,
        "`अभिव्यञ्जकसूचकाङ्क` is declared by `ast.t1` alone and `वाक्यविभागॱआरम्भः` \
         does not name it, so it must be untouched — it moved {exprs_before} → \
         {exprs_after}, which means this test is measuring a blanket reset and \
         not a name collision"
    );

    println!(
        // `w249_colliding_global_names` WAS EMITTED HERE AS THE LITERAL `8`,
        // while the test above emits the same metric name COMPUTED from the
        // corpus. Two emissions of one metric, one of them a constant, is a
        // number with no mechanism — it would have kept printing 8 after the
        // collisions were gone. Dropped; the computed one is the metric.
        // `w249_colliding_counter_arena_pairs` WAS THE LITERAL `3` here and is
        // dropped for the same reason as its neighbour: the three pairs were
        // (counter, arena) collisions drawn from the eight colliding names, and
        // the collision count is now 0 — so a hardcoded 3 is a number that
        // outlived its subject. IT HAS NO COMPUTED EMISSION ANYWHERE, unlike
        // `w249_colliding_global_names`, so it is removed rather than corrected:
        // a metric worth having is worth measuring, and this one was never
        // measured.
        "\n\
         वाक्यसूचकाङ्क  {stmts_before} → {stmts_after}   (ast.t1:225 + vakyavibhaga.t1:403)\n\
         वाक्यकोश len  {arena_before} → {arena_after}\n\
         अभिव्यञ्जकसूचकाङ्क {exprs_before} → {exprs_after}   (ast.t1 alone — the control)\n"
    );

    // ── **INVERTED 2026-09-11. THE TEST'S OWN INSTRUCTION SAID "delete this
    // test and keep the ratchet"; IT IS KEPT AND INVERTED INSTEAD.** ──
    //
    // The author anticipated this exact moment and prescribed deletion. The
    // ratchet IS kept — `collisions.len()` is now asserted at 0 with its reading
    // written out. But deleting removes the only executable record that the
    // defect was real, and an inverted test proves the fix where a deleted one
    // proves nothing. If the owner prefers the author's instruction, this whole
    // test goes and the ratchet alone stands; that is a live choice, not an
    // oversight.
    //
    // BOTH HALVES ON PURPOSE. Asserting only that `वास्तु`'s cursor survives
    // would pass if `आरम्भः` stopped resetting anything at all.
    assert_eq!(
        stmts_after, stmts_before,
        "`वाक्यविभागॱआरम्भः` moved `वास्तु`'s `वाक्यसूचकाङ्क`, {stmts_before} → \
         {stmts_after}. Since the 2026-09-11 rename these are separate cells and \
         one module's reset must not reach another's counter"
    );
    assert_eq!(
        own_after, 0,
        "`वाक्यविभागॱआरम्भः` did NOT reset its own `वाक्यविभागवाक्यसूचकाङ्क` — it \
         reads {own_after}. The reset still has to happen; without this the \
         assertion above passes when `आरम्भः` resets nothing"
    );
    assert_eq!(
        arena_after, arena_before,
        "and NOTHING truncated `वाक्यकोश`: {arena_before} statements are still \
         addressable. The arena was never the defect — the shared CURSOR was"
    );
}

/// **THE RATCHET.** No new global name may be declared by two modules.
///
/// Pinned at the eight that exist today rather than at ०, because closing them
/// is a corpus-wide rename (`nirvahana.rs:1292` says so) and this row is a
/// SEARCH. A ninth fails by name.
#[test]
fn no_new_global_name_is_declared_by_two_modules() {
    use std::collections::BTreeMap;

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut where_declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in corpus_sources() {
        let text = std::fs::read_to_string(dir.join(&name)).expect("readable");
        for line in text.lines() {
            let rest = line
                .strip_prefix("सार्वजनिक चरः ")
                .or_else(|| line.strip_prefix("चरः "));
            // Module level only: a global is declared at column ०, a local is
            // indented. `strip_prefix` on the raw line is that test.
            if let Some(g) = rest.and_then(|r| r.split_whitespace().next()) {
                let e = where_declared.entry(g.to_string()).or_default();
                if !e.contains(&name) {
                    e.push(name.clone());
                }
            }
        }
    }

    let collisions: Vec<(&String, &Vec<String>)> = where_declared
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();

    let mut report = String::from("\nglobal names declared by more than one module:\n");
    for (g, files) in &collisions {
        report.push_str(&format!("  {g}  ←  {}\n", files.join(" + ")));
    }
    report.push_str(&format!(
        "METRIC w249_colliding_global_names {}\n",
        collisions.len()
    ));
    println!("{report}");

    // 8 -> 0 on 2026-09-11. **READ, NOT RE-BASELINED** — the instruction in the
    // message below is why this needed reading rather than stamping, and the
    // reading is: the owner's storage ruling (research/28) removed all eight
    // collisions by renaming the `वाक्यविभाग`/`संयोजन` side. Zero is the count
    // going to its floor, not the instrument going quiet.
    //
    // The eight were `आज्ञाकोश`, `आज्ञासूचकाङ्क`, `वाक्यकोश`, `वाक्यसूचकाङ्क`,
    // `दोषकोश`, `दोषसूचकाङ्क`, `पाठकोष्ठकम्`, `दत्तकोष्ठकम्`. The sibling test in
    // this file demonstrated what they did by running; it is now inverted and
    // asserts the separation, with its original margin kept as the witness.
    //
    // A NON-ZERO NUMBER HERE IS NOW A NEW COLLISION and the same instruction
    // applies in the same words: read it, do not re-baseline it.
    assert_eq!(
        collisions.len(),
        0,
        "the corpus had 8 global names declared by two modules when W-249 \
         measured it and 0 after the 2026-09-11 rename. A different number is a \
         real change and must be read, not re-baselined: globals are keyed by the \
         BARE name (`nirvahana.rs:1292`), so a collision makes one module's reset \
         restart another's numbering over another's arena.{report}"
    );
}

/// **WRITE THROUGH ONE MODULE, READ THROUGH THE OTHER** — the control the row
/// asks for by name, applied to the collision rather than to a clearing.
///
/// `वाक्यविभागॱवाक्ययोजनम्` (`vakyavibhaga.t1:617`) advances `वाक्यसूचकाङ्क` and
/// writes a `वाक्यविभागॱवाक्य` at it. If `वास्तु`'s statement cursor moves and
/// its arena grows, the two are ONE object and `वास्तु`'s statement arena now
/// holds a record of a type `वास्तु` cannot read.
///
/// THIS NEEDS NO SECOND PROGRAM AND NO RESET. It is the collision on its own.
///
/// ── **CORRECTION 2026-09-11 — THE PARAGRAPH ABOVE IS KEPT AS THE WITNESS** ──
///
/// Everything above was TRUE and is now HISTORY. It described a real defect and
/// this test demonstrated it by running: a write through `वाक्यविभाग`'s appender
/// moved `वास्तु`'s cursor and grew `वास्तु`'s arena, because the two names were
/// ONE FLAT KEY and `CHAIN` loads `वाक्यविभाग` second, so it initialised the cell.
///
/// The owner's storage ruling (research/28) removed the collision by renaming the
/// `वाक्यविभाग` side. **The defect the paragraph above describes no longer exists,
/// so this test can no longer demonstrate it** — and the text is left standing
/// rather than deleted because it is the record that the defect was real.
///
/// **THE ASSERTION BELOW IS NOW THE FIX'S ACCEPTANCE TEST, AND IT HAS TWO HALVES
/// ON PURPOSE.** Asserting only that `वास्तु`'s arena does NOT move would pass if
/// the write landed correctly, AND if the write went nowhere, AND if the routine
/// stopped being called at all — a check that cannot fail is the defect this file
/// catalogues elsewhere, wearing the opposite sign. So it asserts BOTH: nothing
/// lands in `वास्तु`'s, and the record DOES land in `वाक्यविभाग`'s own.
#[test]
fn a_write_through_vakyavibhaga_does_not_land_in_vastus_statement_arena() {
    let mut it = load_corpus();

    let extent = lex_and_parse(&mut it, LONG);
    assert!(extent >= 6, "the program must parse");

    let cursor_before = counter(&it, "वाक्यसूचकाङ्क").expect("a cursor");
    let len_before = arena_len(&it, "वाक्यकोश").expect("an arena");
    // AND THE POSITIVE HALF'S BASELINE — `वाक्यविभाग`'s OWN arena, which is a
    // separate cell since the rename and must be where the record actually goes.
    let own_cursor_before = counter(&it, "वाक्यविभागवाक्यसूचकाङ्क").expect("a cursor");
    let own_len_before = arena_len(&it, "वाक्यविभागवाक्यकोश").expect("an arena");
    assert!(
        cursor_before > 0,
        "`वास्तु` must have numbered statements first, or the write below lands \
         in an empty arena and proves nothing about sharing"
    );

    // `वाक्यविभाग`'s appender, called directly. Nothing of `वास्तु` is named.
    it.call(
        "वाक्यविभागॱवाक्ययोजनम्",
        vec![
            Value::Int(0),
            Value::Int(1),
            octets("परीक्षा"),
            Value::Int(0),
            Value::Int(0),
        ],
        50_000_000,
    )
    .expect("वाक्यविभागॱवाक्ययोजनम् runs");

    let cursor_after = counter(&it, "वाक्यसूचकाङ्क").expect("a cursor");
    let len_after = arena_len(&it, "वाक्यकोश").expect("an arena");
    // **`w249_cross_module_write_observed` WAS THE LITERAL `1` AND IS NOW
    // COMPUTED.** It is the only one of W-249's five metrics that measures
    // BEHAVIOUR rather than declarations — it says the cross-module write was
    // caught in the act — and it was a constant, so it would have kept reporting
    // `1` after the rename separated the cells. A metric that cannot report the
    // absence of what it measures is the defect this file exists to find.
    //
    // Third hardcoded metric in this file; the other two are dropped above. This
    // one is kept because it is the ONE endpoint that makes a before/after delta
    // possible: `1` at b6367fb0 pre-rename, `0` here. The collision count and
    // `t1_modules`'s empty set can only say "nothing collides NOW".
    let observed = i32::from(cursor_after != cursor_before || len_after != len_before);
    println!(
        "\nवाक्यसूचकाङ्क {cursor_before} → {cursor_after}   वाक्यकोश len {len_before} → {len_after}\n\
         METRIC w249_cross_module_write_observed {observed}\n"
    );

    let own_cursor_after = counter(&it, "वाक्यविभागवाक्यसूचकाङ्क").expect("a cursor");
    let own_len_after = arena_len(&it, "वाक्यविभागवाक्यकोश").expect("an arena");

    // NEGATIVE HALF — the collision is closed.
    assert_eq!(
        cursor_after, cursor_before,
        "`वाक्यविभागॱवाक्ययोजनम्` moved the cursor `वास्तु` reads, {cursor_before} \
         -> {cursor_after}. The globals were renamed to separate these cells; if \
         this moves again they have been merged back"
    );
    assert_eq!(
        len_after, len_before,
        "and a record landed in the arena `वास्तु` reads its statements out of, \
         {len_before} -> {len_after} — `वास्तु` would read a `वाक्यविभागॱवाक्य` \
         through fields only `वास्तुॱवाक्य` declares"
    );

    // POSITIVE HALF — AND THE WRITE STILL HAPPENED. Without this the test passes
    // when the appender stops writing at all, which is a silence, not a fix.
    assert_eq!(
        own_cursor_after,
        own_cursor_before + 1,
        "`वाक्यविभाग`'s OWN cursor did not advance, {own_cursor_before} -> \
         {own_cursor_after}. The write went nowhere and the negative half above \
         passed vacuously"
    );
    assert!(
        own_len_after > own_len_before,
        "`वाक्यविभाग`'s own arena did not grow, {own_len_before} -> {own_len_after}"
    );
}

/// **THE CHECKER, TESTED AT ITS BOUNDARY.** `restart_line` asserts it finds
/// exactly one restart; an assertion nobody has seen fail is one nobody should
/// trust. A `MONOTONIC` counter is by definition never cleared, so asking for its
/// restart MUST panic. If this ever passes, `restart_line` has stopped
/// discriminating and every line it prints is decoration again — which is the
/// state this file was in when `artha.t1:1111` pointed at a bare `इति`.
#[test]
fn restart_line_refuses_a_counter_that_never_restarts() {
    let (ctr, _) = MONOTONIC[0];
    let hit = std::panic::catch_unwind(|| restart_line("artha.t1", ctr));
    assert!(
        hit.is_err(),
        "`restart_line` found a restart for `{ctr}`, which MONOTONIC says never \
         restarts. Either the counter now restarts — a real finding, move it out \
         of MONOTONIC — or the checker matches anything, in which case the eight \
         lines it prints mean nothing."
    );
}
