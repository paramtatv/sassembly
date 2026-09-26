//! **`W-279` — CAN THE OPERAND READER READ THE CORPUS, OR ONLY THE SEVEN
//! FIXTURES IT WAS BUILT ON?**
//!
//! # Why this file exists
//!
//! `t1_emitted_labels.rs` resolves a conditional's two operands back to spill
//! slots, and that column is the ONLY thing on the `स्वपरीक्षा` ladder that can
//! see a dropped `अधिकम्` exchange: dropping it keeps the mnemonic, the
//! instruction count, the block graph AND the object length, so every other
//! reading on the ladder scores `>` lowered as `<` green. That column has only
//! ever been shown SEVEN fixtures — one per rung — and the corpus writes
//! `अधिकम्` at **325 sites in 18 modules** (grep, this tree). Nothing had asked
//! whether the reader can even READ a real object's branches.
//!
//! A reader that answers `None` for every operand of every corpus conditional
//! is a reader that proves nothing about the corpus, and it looks exactly like
//! a reader that works — because `None` is also the correct answer for an
//! operand that is a materialised literal. **THAT IS THE FAILURE THIS FILE IS
//! HERE TO MAKE IMPOSSIBLE TO REPORT AS COVERAGE.**
//!
//! # THE PREDICTIONS, REGISTERED BEFORE THE RUN
//!
//! Off the corpus grep (`समम्` 1,606 · `असमम्` 321 · `न्यूनम्` 606 ·
//! `बृहत्समम्` 45 · `अधिकम्` 325 = **2,903** `compare_op` sites in 21 sources)
//! and off `ashtaka.t1`'s emitted text read by hand:
//!
//! ```text
//!   P1  conditionals emitted  > 2,903      — `विकल्पः` dispatch and the chain's
//!                                            own checks branch where no
//!                                            `compare_op` is written
//!   P2  BOTH operands a slot  < 1 in 10    — the fixture compares two
//!                                            PARAMETERS; the corpus mostly
//!                                            compares against a materialised
//!                                            literal or a field load
//!   P3  NEITHER a slot        the largest bucket, over half
//!   P4  `Undefined` operands  ZERO         — an operand register its own
//!                                            routine never writes is a
//!                                            use-before-def, and no octet
//!                                            count can see one
//!   P5  `Unreadable` operands ZERO         — every conditional's two operands
//!                                            are register tokens
//! ```
//!
//! **P1, P2 and P3 ARE METRIC** (owner ruling 2026-09-13, point 1) — they are
//! printed with their prediction beside them and asserted by nothing, because
//! a count encodes nothing about correctness and pins cost half a landing.
//! **P4 AND P5 ARE THE ASSERTIONS**, and each names its branches by routine and
//! by target rather than counting them.
//!
//! # WHAT IT MEASURED — AND THREE OF THE FIVE PREDICTIONS WERE WRONG
//!
//! ```text
//!   21 sources · 20 objects · 4,283 conditionals · 8,566 operands
//!   both a slot 526 · one 2,293 · neither 1,464
//!   operand origins: slot 3,345 · zero 343 · written:योगः 3,845 ·
//!     written:आहारः 716 · written:युक्तम् 181 · written:अचिह्नन्यूनम् 85 ·
//!     written:लङ्घनम् 0 · written:वियोगः 23 · written:गुणनम् 17 ·
//!     उपरिभारः 5 · शेषः 2 · न्यूनम् 2 · वामसरणम् 1 · सचिह्नदक्षिणसरणम् 1
//!   undefined 0 · unreadable 0 · instrument disagreements 0
//! ```
//!
//! * **P1 HELD.** 4,283 against 2,903 written `compare_op`s — 1.48 branches
//!   emitted per comparison spelled.
//! * **P2 FAILED, and narrowly.** 526 of 4,283 is **1 in 8.1**, not under 1 in
//!   10. Recorded as a miss rather than rounded into a hit.
//! * **P3 FAILED.** `neither` is 1,464 and the largest bucket is `one`, at
//!   2,293 — **more than half of all conditionals compare a slot against
//!   something else**, which is the shape "parameter tested against a literal"
//!   takes and which the two-parameter fixture never showed.
//! * **P5 HELD.** No conditional in the corpus carries an operand outside the
//!   `…न`/`…त्` roles.
//! * **P4 FAILED ON THE FIRST RUN — AND THE READER WAS WRONG, NOT THE
//!   COMPILER.** 21 branches across 8 modules, every one of them
//!   `विषमलङ्घनम्`, named an operand register their routine never writes. All
//!   21 were `शून्यः`, and `शून्यः` is **x0** (`riscv64.rs:69`): hardwired,
//!   written by nothing and writable by nothing.
//!
//! # THE INSTRUMENT DEFECT THE CORPUS FOUND, AND WHY THE LADDER COULD NOT
//!
//! `लङ्घनम् शून्यःम् <target>य् ।` — the unconditional jump — names x0 in the
//! कर्म to DISCARD the link, and the reader recorded that as a write. So one
//! register got **two different answers depending on whether its routine
//! happened to jump before it compared**: `written:लङ्घनम्` in the 322 places
//! that did, `undefined` in the 21 that did not. Neither is what x0 is, and
//! `undefined` is the reading this file asserts is a defect — so the reader was
//! manufacturing its own red.
//!
//! [`Origin`] now carries a fifth state, [`Origin::Zero`], and x0 is never a
//! destination however a line spells it. 322 + 21 = **343**, which is the
//! `zero` figure above: the two wrong answers were one register all along.
//! **NOT ONE OF THE SEVEN RUNG FIXTURES COULD HAVE SHOWN THIS** — each is a
//! single routine comparing two parameters, and none of them jumps before it
//! compares.
//!
//! # AND TWO OBJECTS ANSWER NOTHING
//!
//! `ast.t1` and `vastu.t1` emit **zero** conditionals, so the operand column
//! says nothing whatever about them. They are named here rather than left to
//! be read out of a total: a sweep reporting "20 objects" reads like 20
//! objects tested.
//!
//! # AND THEN THE EXCHANGE ITSELF — `every_adhikam_against_a_numeral…`
//!
//! The sweep above proves the reader can READ the corpus and proves NOTHING
//! about `अधिकम्`: 3,854 of its 8,586 operands answered `written:योगः`, a
//! MATERIALISED LITERAL named by the instruction that made it and not by its
//! VALUE, so no branch comparing a name against a constant could be checked.
//! [`Origin::Literal`] resolves 3,182 of those 3,854 — the remaining 672 are
//! real additions and `lui`+`addi` second halves, which must STAY `Written`.
//!
//! With the value in hand the exchange is a corpus-scale claim. `ir.rs:81`
//! lowers `अधिकम्` to `Lt` WITH ITS OPERANDS EXCHANGED, so a constant the
//! source wrote SECOND must come out FIRST:
//!
//! ```text
//!   16 modules · 229 `<name> अधिकम् <numeral>` sites in `यदि`/`यावत्` heads
//!   210 in reach of the addi immediate · 19 out of reach, NAMED not dropped
//!   0 short · 16 of 16 modules go RED when the exchange is dropped
//! ```
//!
//! The 19 out of reach are `i64::MAX` (×2), `2147483647`, `524287`, `-2049`
//! and fourteen `nidana.t1` diagnostic codes above 2047. They arrive through
//! `उपरिभारः` or the constant pool and [`Origin::Literal`] resolves neither —
//! so they are counted as OUT OF REACH, never as covered. A sweep that dropped
//! them would report 229 sites read while reading 210.
//!
//! **THE FIGURES ABOVE AND THE ONES IN THE PREVIOUS SECTION WERE TAKEN ON
//! DIFFERENT TREES** — `8814a4cd` added four `अधिकम्` sites to `artha.t1` and
//! nine conditionals to the corpus between the two runs. They are not summed,
//! and neither is pinned: every count in this file is REPORT-ONLY (owner
//! ruling 2026-09-13, point 1), and what is asserted is `short.is_empty()`,
//! per-module sensitivity, and a FLOOR on the harvest.
//!
//! **AND EVERY MODULE PROVES THE SWEEP CAN SEE THE DEFECT IN IT.** A green says
//! the exchange is there OR that the instrument is blind here, and those are
//! the same output; so each object is rewritten the way a front end that had
//! dropped the exchange would emit it — same mnemonic, same line count, same
//! labels — and the sweep must name it. Proving that on ONE object would have
//! licensed the other fifteen on nothing.
//!
//! # AND THE OTHER OPERAND SHAPE — `every_adhikam_between_two_names…`
//!
//! The numeral sweep is blind to every comparison of TWO VARIABLES, and those
//! are the larger half: both operands come out [`Origin::Slot`], and a slot
//! NUMBER is an allocation artefact no reader here tied back to a source word,
//! so `अ अधिकम् आ` and `अ न्यूनम् आ` were the same two slots in the opposite
//! order with nothing saying which order was which.
//!
//! The FRAME is what ties them back. `riscv64::Frame` puts a routine's
//! parameters and its `चरः` locals in a region of their own — slot `k` at
//! `8(num_spills + k)` — and `ir.t1:1320` (`स्थानीयघोषणम्`) hands out those `k`
//! in FIRST-DECLARATION ORDER. So `a न्यूनम् b` must put the earlier-declared
//! name in the करण exactly when `a` is declared first, and `a अधिकम् b`
//! exactly when `b` is; dropping the exchange swaps every one of them.
//!
//! ```text
//!   17 modules · 216 `<name> <op> <name>` sites in 156 routines
//!   0 short · 17 of 17 modules and 136 of 156 routines go RED when dropped
//!   38 sites OUT OF REACH, each NAMED · 0 operands in a spill slot
//! ```
//!
//! **THE ORDER AND NOT THE NUMBER**, because `ir.t1:1345` (`अनामस्थानम्`) cuts
//! a pool of EIGHT anonymous slots the first time a routine needs run growth, a
//! slice, a content equality or a length — lazily, mid-body — so a `चरः`
//! declared after that point sits eight slots above its rank. A reader that
//! predicted slot numbers would be wrong in every routine that touches a run.
//! The pool is inserted at one position and reorders nothing around it, and
//! `8·num_spills` cancels out of a comparison of two slots in one frame, so
//! the ORDER survives both and needs no offset arithmetic at all.
//!
//! **BUT THE NUMBER MODULO EIGHT SURVIVES TOO.** The pool is cut ONCE and is
//! EXACTLY `अनामस्थलसंख्या = ८` wide (`ir.t1:1344`), so a name of rank `r`
//! holds slot `r` or slot `r + ८` and nothing else — and `slot ≡ rank (mod ८)`
//! wherever the cut fell. That is a claim about the slot NUMBERS which needs
//! no recovery of the pool's position, so it reaches all 108 routines
//! [`pairable`] refuses: **153 sites, 143 routine-claims sensitive to the
//! dropped exchange, 18 routines RED on the residue that the order census
//! reads green.** Two sites are UNINFORMATIVE — ranks congruent modulo eight
//! carry the same residue pair either way round — and are named, never
//! counted as covered. Its first run found a defect in THIS FILE: a `चरः`
//! inside `उक्तम् … इति` was minting a frame slot (`shrinkhala.t1:3370`),
//! shifting every later rank by one, which an offset-blind census can never
//! see.
//!
//! **AND THE POSITION ITSELF IS RECOVERABLE — FROM THE OBJECT, NOT FROM THE
//! FRONT END.** The plan this file carried was to read `अनामस्थलारम्भः`
//! (`ir.t1:1343`) out of `Front` per routine, since it holds `१ +` the pool's
//! first slot. **THAT PREMISE IS WRONG BY CONSTRUCTION.**
//! `मध्यरूपॱकार्यक्रमरचना` lowers EVERY routine of a module inside ONE
//! interpreter call and zeroes that global at the top of each one
//! (`ir.t1:4763`, and `:4826` for the synthesised growth routine), so once
//! `build_ir` returns it holds the LAST routine's cut and nothing else —
//! and `स्थानीयचिह्नककोश` is rewritten from index १ per routine for the same
//! reason. There is no per-routine record to read; making one means a new
//! arena in `ir.t1`, the compiler's shape changed to serve a census.
//!
//! So the OBJECT is asked instead. A cut position `b` fixes the whole
//! numbering — rank `r` at slot `r` below it, at slot `r + ८` at or above it —
//! and there are only `names + १` candidates, so [`consistent_bases`] tries
//! every one and keeps those the object carries. **This is a claim PER ROUTINE
//! where the residue is one PER SITE**, and that is the difference that bites:
//! the residue lets each site of a routine fall on whichever side of the cut
//! suits it, so an object carrying ranks `(०, ९)` as `(०, ९)` and ranks
//! `(८, १)` as `(१६, ९)` satisfies the residue twice over while no single cut
//! explains both.
//!
//! **AND THE OTHER THREE `compare_op`s ARE EVIDENCE FOR THE SAME BASE.** Most
//! routines write one to four `न्यूनलङ्घनम्`s, which is why the first reading
//! left 77 of 108 AMBIGUOUS — but `समम्`, `असमम्` and `बृहत्समम्` each also pair
//! two slots of ONE frame, and `अनामस्थानम्` runs once per ROUTINE and not once
//! per mnemonic (`ir.t1:1346`), so every one of their sites constrains the SAME
//! single base. None of the three EXCHANGES — only `अधिकम्` does
//! (`ir.t1:2191`) — so each predicts `(करण, अपादान)` in written order, matched
//! against the pairs the object carries under ITS OWN branch word
//! ([`cut_groups`]): a flat list would let a `समम्` want be satisfied by a
//! `न्यूनलङ्घनम्` the object happens to carry, which is a constraint silently
//! dropped. Each extra site can only SHRINK the candidate set, and
//! [`consistent_bases_of`] is asserted monotone against the one-mnemonic
//! reading on every routine, so a recovery is witnessed and never invented.
//!
//! ```text
//!   108 routines `pairable` refuses · 0 explained by NO cut position
//!   34 routines (41 sites) where EXACTLY ONE cut position fits — position
//!     RECOVERED, and with it the exact (करण, अपादान) pair
//!   74 AMBIGUOUS: several positions fit, so none is named
//!   108 of 108 go RED when the exchange is dropped
//!   80 two-name heads under the other three operators: 11 REFUSED,
//!     28 FOLDED, 14 unused (already paired), 27 unused (routine unvisited)
//!   5 routines narrowed · 3 recovered ONLY with the extra evidence
//! ```
//!
//! **AND THE TYPE DECIDES WHETHER A BRANCH EXISTS AT ALL.** `ir.t1:2200` sends
//! `समम्`/`असमम्` between two RUNS to `खण्डसाम्यरचना` — a CALL — so the object
//! carries no `समलङ्घनम्` for it and a harvester that predicted one would empty
//! the routine's candidate set and report the compiler RED for being right. Ten
//! corpus sites are that shape and one compares against a `सम्भाव्य`; all
//! eleven are NAMED off an ALLOW-LIST of scalar type words, so a type nobody
//! has thought about lands in the named bucket and not in the claim. The four
//! buckets are asserted to ADD UP to what was claimed: a site harvested and
//! then lost is an instrument with two states where the truth has four.
//!
//! **WHAT THIS CYCLE DID NOT WIDEN.** The cut recovery runs over the routines
//! [`pairable`] refuses, which by construction write a `न्यूनम्`/`अधिकम्` head;
//! 27 claimed sites sit in routines it never visits, whose positions could be
//! recovered from the other three operators ALONE. That is a widening, it is
//! counted rather than dropped, and it is not taken here.
//!
//! **AMBIGUITY IS REPORTED, NEVER RESOLVED.** A routine of one site whose
//! predicted pair is `(०, १)` is explained by every cut at rank २ or later;
//! naming the agreeable one would be fitting the answer to the data, so the
//! existence claim stands for it and the exact pair does not.
//!
//! The 38 out of reach are a GLOBAL on one side (`parse.t1`'s `पठनस्थान`,
//! `ir.t1`'s `अनामस्थलसंख्या`, `समावेशसंख्या`, eleven qualified reads of
//! another module's global). A global has no frame slot, [`Origin::Slot`] will
//! never answer for it, and matching one against slot ० — which is a real
//! answer, the FIRST PARAMETER — would be a reading invented out of nothing.
//! The window also separates a LOCAL slot from the allocator's SPILL slots
//! below it: the corpus puts none of those in a conditional today, so the
//! guard is proved on a fixture rather than assumed off a zero.
//!
//! **AND THE HARVESTER'S OWN BLIND SPOT WAS THE FIRST THING IT FOUND.** Its
//! parameters were read as `ऽ`-separated; `ashtaka.t1:103` writes three with no
//! separator at all, so `आरम्भः` and `सीमा` had no slots and their comparison
//! was reported OUT OF REACH — the instrument's defect arriving in the bucket
//! reserved for the corpus's. It cost 24 of 216 sites.
//!
//! # TWO INSTRUMENTS, KEYED OPPOSITE WAYS
//!
//! [`crate::emitted::label_transfer`] decides a line is conditional by the
//! OPERAND ROLE — last-but-one operand a `…य्`, and not the one unconditional
//! mnemonic. This file counts them a second way, by the MNEMONIC SUFFIX: a word
//! ending `लङ्घनम्` that is neither `लङ्घनम्` nor `सापेक्षलङ्घनम्`. The two share
//! no rule, so a branch either finds both or names the disagreement.

mod emitted;

use emitted::{ConditionalOperands, Origin, conditional_operands};
use sadhana::t1::chain::{Front, module_name};
use sadhana::t1::regalloc::allocate_registers;
use sadhana::t1::riscv64;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.t1` the corpus ships, in name order — READ OFF THE DIRECTORY, for
/// the reason `t1_corpus_globals.rs:79` gives: a sweep whose whole claim is
/// "every object" cannot be the file that decides which objects there are.
fn corpus_paths() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory is readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    assert!(
        !v.is_empty(),
        "the corpus is not empty; this sweep read no sources"
    );
    v
}

/// What one source came to. **THREE STATES, NOT TWO** — see
/// `t1_corpus_globals.rs`'s copy for why `lib.t1` is not a refusal.
#[derive(Debug)]
enum Object {
    Emitted(String),
    DeclaresNoModule,
    Refused(String),
}

/// THIS FILE'S OWN LOADER AND ITS OWN DRIVER — a fresh [`Front`] per source.
///
/// A test LOADER is part of the test. Sharing one across sources is the shape
/// `W-279` has already been burned by once: a measurement whose interval
/// carries state from its predecessor.
fn compile(src: &str) -> Object {
    let Some(module) = module_name(src) else {
        return Object::DeclaresNoModule;
    };
    let mut front = match Front::load(&spec_root()) {
        Ok(f) => f,
        Err(e) => return Object::Refused(format!("the front end does not load: {e}")),
    };
    let text = (|| -> Result<String, String> {
        front.lex(src)?;
        front.parse()?;
        front.resolve()?;
        front.typecheck()?;
        front.build_ir()?;
        let module = front.module(&module, None)?;
        riscv64::emit_module(&module).map_err(|e| format!("{e:?}"))
    })();
    match text {
        Ok(t) => Object::Emitted(t),
        Err(e) => Object::Refused(e),
    }
}

/// THE SECOND COUNT, KEYED ON THE MNEMONIC WHERE
/// [`crate::emitted::label_transfer`] is keyed on the operand role.
///
/// `…लङ्घनम्` is the ISA's branch family. `लङ्घनम्` itself is the unconditional
/// transfer and `सापेक्षलङ्घनम्` the indirect return (`ashtaka.sas:40`), whose
/// last operand is an OFFSET and not a label — so both are excluded by name and
/// everything else in the family is a conditional. A reader keyed this way
/// misses a conditional the ISA spells some other way; the role-keyed one
/// misses a conditional whose target operand is malformed. Neither can hide the
/// other's blind spot, which is the point.
fn branch_family_lines(text: &str) -> usize {
    text.lines()
        .filter(|l| {
            // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
            let Some(m) = l.split_whitespace().next() else {
                return false;
            };
            m.ends_with("लङ्घनम्") && m != "लङ्घनम्" && m != "सापेक्षलङ्घनम्"
        })
        .count()
}

/// One object's conditionals, folded into the three counts and the two defect
/// lists. NAMED, NOT COUNTED: every finding carries its routine and the label
/// the branch would have taken.
#[derive(Default)]
struct Reading {
    conditionals: usize,
    both: usize,
    one: usize,
    neither: usize,
    /// `Origin::class()` -> how many operands landed in it.
    classes: BTreeMap<String, usize>,
    undefined: Vec<String>,
    unreadable: Vec<String>,
}

fn site(c: &ConditionalOperands) -> String {
    format!("{}: {} -> {}", c.routine, c.mnemonic, c.target)
}

fn read(text: &str) -> Reading {
    let mut r = Reading::default();
    for c in conditional_operands(text) {
        r.conditionals += 1;
        match (c.karana.slot().is_some(), c.apadana.slot().is_some()) {
            (true, true) => r.both += 1,
            (false, false) => r.neither += 1,
            _ => r.one += 1,
        }
        for o in [&c.karana, &c.apadana] {
            *r.classes.entry(o.class()).or_default() += 1;
            match o {
                Origin::Undefined => r.undefined.push(site(&c)),
                Origin::Unreadable(_) => r.unreadable.push(format!("{} [{}]", site(&c), o.class())),
                _ => {}
            }
        }
    }
    r
}

/// **THE SWEEP.** Every conditional every object the corpus emits carries, with
/// both operands traced to where they came from.
#[test]
fn every_conditional_the_corpus_emits_has_both_its_operands_traced() {
    let paths = corpus_paths();
    println!("METRIC t1_corpus_conditionals_sources {}", paths.len());

    let mut no_module: Vec<String> = Vec::new();
    let mut refused: Vec<String> = Vec::new();
    let mut objects = 0usize;
    let mut total = Reading::default();
    let mut undefined: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let mut disagreed: Vec<String> = Vec::new();

    for p in &paths {
        let file = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let src = std::fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
        match compile(&src) {
            Object::DeclaresNoModule => {
                println!("  {file:22} declares no मण्डलम्");
                no_module.push(file);
            }
            // A REFUSAL IS A REFUSAL AND NEVER A MODULE WITH NO CONDITIONALS.
            // Folded together, a source the chain stopped compiling would enter
            // the census as a clean zero and every property below would hold of
            // it vacuously.
            Object::Refused(why) => {
                println!("  {file:22} REFUSED {why}");
                refused.push(format!("{file}: {why}"));
            }
            Object::Emitted(text) => {
                objects += 1;
                let r = read(&text);
                let family = branch_family_lines(&text);
                if family != r.conditionals {
                    disagreed.push(format!(
                        "{file}: the operand role says {} conditional(s), the \
                         `…लङ्घनम्` family says {family}",
                        r.conditionals
                    ));
                }
                println!(
                    "  {file:22} cond {:<5} both {:<4} one {:<4} neither {:<5} family {family}",
                    r.conditionals, r.both, r.one, r.neither
                );
                undefined.extend(r.undefined.iter().map(|s| format!("{file}: {s}")));
                unreadable.extend(r.unreadable.iter().map(|s| format!("{file}: {s}")));
                total.conditionals += r.conditionals;
                total.both += r.both;
                total.one += r.one;
                total.neither += r.neither;
                for (k, v) in r.classes {
                    *total.classes.entry(k).or_default() += v;
                }
            }
        }
    }

    println!("METRIC t1_corpus_conditionals_objects {objects}");
    println!(
        "METRIC t1_corpus_conditionals_total {} (P1 predicted > 2903)",
        total.conditionals
    );
    println!(
        "METRIC t1_corpus_conditionals_both_slots {} (P2 predicted < 1 in 10)",
        total.both
    );
    println!("METRIC t1_corpus_conditionals_one_slot {}", total.one);
    println!(
        "METRIC t1_corpus_conditionals_neither_slot {} (P3 predicted the largest bucket)",
        total.neither
    );
    // THE UNRESOLVED OPERANDS BY WHY, NOT BY HOW MANY.
    for (class, n) in &total.classes {
        println!("NOTE  operand origin {class:24} {n}");
    }
    println!(
        "NOTE  these five figures are REPORT-ONLY (owner ruling 2026-09-13, point 1). \
         The corpus writes 2,903 `compare_op` sites across 21 sources, of which 325 are \
         `अधिकम्` in 18 modules — the operator whose EXCHANGE nothing but the operand \
         column can see."
    );

    // ── THE HARNESS FIRST. Every property below is vacuously true of an empty
    // reading, so the sweep must be shown to have READ something.
    assert!(
        refused.is_empty(),
        "every source that declares a module must reach the emitter; these did \
         not, and a refusal is NOT a module with no conditionals:\n  {}",
        refused.join("\n  ")
    );
    assert_eq!(
        no_module,
        vec!["lib.t1".to_string()],
        "`lib.t1` is the ONE source that declares no `मण्डलम्`. A second such \
         source is a source this sweep silently skipped."
    );
    assert_eq!(
        objects + no_module.len(),
        paths.len(),
        "every source is accounted for in exactly one state"
    );
    assert!(
        total.conditionals > 0,
        "the corpus emits conditionals; ZERO means the READER broke, not that \
         the compiler stopped lowering `यदि`"
    );
    assert!(
        total.both > 0,
        "and at least one of them resolves BOTH operands to a slot — the column \
         the `अधिकम्` exchange is read off. All-{} unresolved is a reader that \
         proves nothing about this corpus and looks exactly like one that works",
        total.conditionals
    );
    assert!(
        disagreed.is_empty(),
        "THE TWO INSTRUMENTS DISAGREE, which means one is wrong and neither \
         says which:\n  {}",
        disagreed.join("\n  ")
    );

    // ── AND THE TWO PROPERTIES, EACH NAMED BY ROUTINE AND BY TARGET.
    assert!(
        undefined.is_empty(),
        "AN OPERAND REGISTER ITS OWN ROUTINE NEVER WRITES. The branch still \
         assembles, the object is exactly as long, and the comparison reads \
         whatever the previous routine left in that register:\n  {}",
        undefined.join("\n  ")
    );
    assert!(
        unreadable.is_empty(),
        "AN OPERAND THAT IS NOT OF THE OPERAND SHAPE. `…न` is the करण and `…त्` \
         the अपादान (`yantrotsarjana.t1:1417`); anything else is a branch this \
         reader cannot read, and a reader that DROPPED it would report a \
         shorter list as coverage:\n  {}",
        unreadable.join("\n  ")
    );
}

/// **THE CASES THAT MUST STILL BE REFUSED.**
///
/// Five of them, because the sweep makes five claims, and four are made BY HAND
/// on the text a real corpus source really emitted. `ashtaka.t1`: the smallest
/// object that carries conditionals of both resolutions.
#[test]
fn each_unreadable_branch_is_named_on_a_real_corpus_object_rather_than_counted() {
    let src = std::fs::read_to_string(corpus_dir().join("ashtaka.t1")).expect("readable");
    let Object::Emitted(text) = compile(&src) else {
        panic!("ashtaka.t1 must emit; the control has to itself be real");
    };

    // THE CONTROL IS CLEAN FIRST, or every mutation below proves nothing.
    let clean = read(&text);
    assert!(
        clean.conditionals > 0 && clean.both > 0,
        "ashtaka.t1 emits conditionals and at least one resolves both operands"
    );
    assert!(clean.undefined.is_empty() && clean.unreadable.is_empty());
    assert_eq!(branch_family_lines(&text), clean.conditionals);

    // The first conditional that resolves BOTH operands, taken from the text so
    // this control does not depend on which branch `ashtaka` emits first.
    let both = conditional_operands(&text)
        .into_iter()
        .find(|c| c.karana.slot().is_some() && c.apadana.slot().is_some())
        .expect("ashtaka.t1 emits a conditional with both operands in slots");
    let line = text
        .lines()
        .find(|l| l.starts_with(&both.mnemonic) && l.ends_with(&format!("{}य् ।", both.target)))
        .expect("the branch this reading came from is in the text")
        .to_string();
    // THE TOKEN AND THE REGISTER ARE NOT THE SAME STRING. `स्थिर०न` is the
    // operand; `स्थिर०` is what a load writes. Mutation ONE needs the register
    // and mutation TWO needs the token, and the first draft of this control
    // used the token for both — which stripped no line, bit nothing, and left
    // an EMPTY defect list reported as a pass.
    let f: Vec<&str> = line.split_whitespace().collect();
    let karana_token = f[1].to_string();
    let karana = karana_token
        .strip_suffix('न')
        .expect("the करण operand is a register in the `…न` role")
        .to_string();

    // ── ONE: THE OPERAND'S DEFINITION IS GONE. The register is never written in
    // its routine, so the compare reads whatever the caller left there — and the
    // object is exactly as long, minus one load.
    //
    // **EVERY WRITE, NOT JUST THE SPILL LOAD.** Removing only the
    // `आहारः {karana}म्` lines leaves the prologue's own
    // `योगः {karana}म् अर्थ०न ०न ।` standing, the register reads `Written`
    // rather than `Undefined`, and this control passes an EMPTY list off as a
    // finding — which it did, on the first run of this file.
    let dst = format!("{karana}म्");
    let stripped: String = text
        .lines()
        .filter(|l| l.split_whitespace().nth(1) != Some(dst.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(stripped, text, "the mutation must actually bite");
    let f1 = read(&stripped);
    assert!(
        f1.undefined.iter().any(|s| s == &site(&both)),
        "the branch whose operand is now undefined is named by its ROUTINE and \
         the label it takes, not counted:\n{:?}",
        f1.undefined
    );

    // ── TWO: THE OPERAND IS NOT OF THE OPERAND SHAPE. Strip the करण's `न`.
    let mangled = text.replace(&line, &line.replacen(&karana_token, &karana, 1));
    assert_ne!(mangled, text, "the mutation must actually bite");
    let f2 = read(&mangled);
    assert!(
        f2.unreadable.iter().any(|s| s.starts_with(&site(&both))),
        "the branch whose operand cannot be read is NAMED — never dropped from \
         the list, which is how a short list gets reported as coverage:\n{:?}",
        f2.unreadable
    );
    assert_eq!(
        f2.conditionals, clean.conditionals,
        "and it is still COUNTED as a conditional"
    );

    // ── THREE: THE TWO INSTRUMENTS DISAGREE. Take the target off the branch:
    // the mnemonic family still sees it, the operand role no longer does.
    let targetless = text.replace(&line, &line.replace(&format!("{}य् ", both.target), ""));
    assert_ne!(targetless, text, "the mutation must actually bite");
    assert_eq!(
        read(&targetless).conditionals,
        clean.conditionals - 1,
        "the role-keyed reader loses it"
    );
    assert_eq!(
        branch_family_lines(&targetless),
        clean.conditionals,
        "and the mnemonic-keyed one does not — THAT disagreement is the finding"
    );

    // ── FOUR: THE EMPTY READING IS NOT A PASS. Strip every branch of the family
    // and both defect lists are empty while the object is plainly broken.
    let none: String = text
        .lines()
        .filter(|l| branch_family_lines(l) == 0)
        .collect::<Vec<_>>()
        .join("\n");
    let f4 = read(&none);
    assert_eq!(f4.conditionals, 0, "nothing is read");
    assert!(
        f4.undefined.is_empty() && f4.unreadable.is_empty(),
        "AND EVERY DEFECT LIST IS EMPTY — which is why the sweep asserts the \
         harness before it asserts a property"
    );

    // ── FIVE: A SLOT DOES NOT CROSS A ROUTINE BOUNDARY. Built by hand rather
    // than compiled, because the emitter does not make this text: the seven rung
    // fixtures are each ONE routine, so nothing had ever shown the reset.
    let two_routines = "\
॥ वैश्विकम् क ॥
कॱॱ
आहारः स्थिर०म् स्तूपसूचकःत् ८न ।
॥ वैश्विकम् ख ॥
खॱॱ
समलङ्घनम् स्थिर०न स्थिर१त् खपर्व१य् ।";
    let c = conditional_operands(two_routines);
    assert_eq!(c.len(), 1, "one conditional, in the second routine");
    assert_eq!(
        (c[0].routine.as_str(), &c[0].karana, &c[0].apadana),
        ("ख", &Origin::Undefined, &Origin::Undefined),
        "`स्थिर०`'s slot was learned in routine `क` and MUST NOT reach routine \
         `ख` — without the reset this answers slot ८, a wrong slot reported \
         with exactly the confidence of a right one"
    );
}

// ── THE `अधिकम्` EXCHANGE, ASKED OF THE CORPUS ──────────────────────────────
//
// This is what [`Origin::Literal`] was added for. The measurement above says
// 2,293 of 4,283 conditionals compare a slot against SOMETHING ELSE and 3,845
// operands read `written:योगः` — and until that bucket resolves to a VALUE, no
// corpus branch comparing a name against a constant can be checked for the one
// defect the whole `स्वपरीक्षा` ladder is blind to.

/// The `addi` immediate. `lower_constant` (`riscv64.rs:558`) materialises a
/// value in this range as ONE `योगः <r>म् शून्यःन <k>न ।`; outside it the
/// constant arrives through `उपरिभारः`+`योगः <r>म् <r>न <lo>न ।` or through the
/// pool, and neither form is a literal this reader can resolve. The bound is
/// named rather than discovered so the out-of-range sites can be COUNTED as
/// out of reach instead of reported as matched.
const ADDI: std::ops::RangeInclusive<i128> = -2048..=2047;

/// Every `<subject> अधिकम् <numeral>` the source writes, as the numeral, in
/// source order.
///
/// **THE NUMERAL IS THE SECOND OPERAND AND THAT IS THE WHOLE POINT.** `ir.rs:81`
/// lowers `अधिकम्` to `Cmp(Lt, standard, subject)` — `Lt` with its operands
/// EXCHANGED — and `riscv64::lower_cond_branch` fuses that pair to
/// `न्यूनलङ्घनम् R(a)न R(b)त्` with करण the FIRST IR operand. So the constant the
/// source wrote SECOND must come out FIRST. A compiler that dropped the
/// exchange emits the same mnemonic, the same instruction count, the same block
/// graph and an object of exactly the same length, with `>` computed as `<`.
///
/// A comment is stripped first — `lex.t1:26` writes `अधिकम् ६` INSIDE one, and
/// a harvester that read it would demand a branch for a line the compiler never
/// saw. Only a `यदि`/`यावत्` head is taken: used as a VALUE rather than as a
/// condition, `अधिकम्` lowers to `न्यूनम्` (an `slt`) and reaches no branch.
fn source_adhikam_numerals(src: &str) -> Vec<i128> {
    let mut out = Vec::new();
    for line in src.lines() {
        let body = line.split('॰').next().unwrap_or("");
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = body.split_whitespace().collect();
        if !matches!(f.first(), Some(&"यदि") | Some(&"यावत्")) {
            continue;
        }
        for (i, t) in f.iter().enumerate() {
            if *t == "अधिकम्"
                && let Some(k) = f.get(i + 1).and_then(|n| emitted::devanagari_signed(n))
            {
                out.push(k);
            }
        }
    }
    out
}

/// How many `न्यूनलङ्घनम्` branches carry each constant in the करण — the role a
/// lowered `अधिकम्` must put its literal in. `शून्यः` counts (see
/// [`Origin::literal`]): x0 IS the constant zero.
fn karana_literal_census(text: &str) -> BTreeMap<i128, usize> {
    let mut m = BTreeMap::new();
    for c in conditional_operands(text) {
        if c.mnemonic != "न्यूनलङ्घनम्" {
            continue;
        }
        if let Some(k) = c.karana.literal() {
            *m.entry(k).or_default() += 1;
        }
    }
    m
}

/// **THE DEFECT ITSELF, WRITTEN OUT.** Exchange the two operands of every
/// `न्यूनलङ्घनम्`: `न्यूनलङ्घनम् Aन Bत् Ty् ।` becomes `न्यूनलङ्घनम् Bन Aत् Ty् ।`.
///
/// That is EXACTLY what a front end that lowered `a अधिकम् b` as `Lt(a, b)`
/// rather than `Lt(b, a)` would emit — same mnemonic, same count, same graph,
/// same length. It is the mutation the sweep below has to go red on, and no
/// octet count, no block-graph reading and no ladder rung can see it.
fn drop_the_exchange(text: &str) -> String {
    text.lines()
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.as_slice() {
                ["न्यूनलङ्घनम्", a, b, t, "।"] => {
                    match (a.strip_suffix('न'), b.strip_suffix("त्")) {
                        (Some(ka), Some(ap)) => {
                            format!("न्यूनलङ्घनम् {ap}न {ka}त् {t} ।")
                        }
                        _ => l.to_string(),
                    }
                }
                _ => l.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// **THE MUTATION THE EXACT-SLOT HALF CLAIM EXISTS TO CATCH.**
///
/// [`drop_the_exchange`] rewrites `न्यूनलङ्घनम्` and nothing else, so it cannot
/// reach a half-site: those live under `समम्`, `असमम्` and `अन्यूनलङ्घनम्`, and
/// `reached_further` asserts the bitten object carries them IDENTICALLY. A
/// claim no mutation in this file can move is a claim nobody has seen work, so
/// here is one that moves exactly it — the two operands of those three words
/// EXCHANGED, same word, same line count, same labels, same target.
///
/// `क समम् ५` and `५ समम् क` name the same slot and the same constant (see
/// [`Half`]), so the direction census, the residue and every cut recovery read
/// this object exactly as they read the honest one. Only the SIDE moves, and
/// only [`half_shortfalls`] asks about it.
fn swap_the_half_sides(text: &str) -> String {
    text.lines()
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.as_slice() {
                [w, a, b, t, "।"] if EXTRA_WORDS.contains(w) => {
                    match (a.strip_suffix('न'), b.strip_suffix("त्")) {
                        (Some(ka), Some(ap)) => format!("{w} {ap}न {ka}त् {t} ।"),
                        _ => l.to_string(),
                    }
                }
                _ => l.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One module's answer to the exchange question.
#[derive(Default, Debug)]
struct Exchange {
    /// `अधिकम् k` sites whose `k` fits the `addi` immediate.
    in_reach: usize,
    /// `अधिकम् k` sites whose `k` does not — named, never counted as matched.
    out_of_reach: Vec<i128>,
    /// An in-reach `k` the object's `न्यूनलङ्घनम्` करण census cannot cover.
    short: Vec<String>,
}

/// Read one source against its object. The claim is a MULTISET one: for every
/// constant `k`, the object carries at least as many `न्यूनलङ्घनम्` branches with
/// `k` in the करण as the source writes `अधिकम् k` sites. At least, and not
/// exactly, because `यावत्` rotation emits a compare twice from one source line
/// — measured, and the direction that matters is the one that goes short.
fn exchange_of(src: &str, text: &str) -> Exchange {
    let mut e = Exchange::default();
    let mut wanted: BTreeMap<i128, usize> = BTreeMap::new();
    for k in source_adhikam_numerals(src) {
        if ADDI.contains(&k) {
            e.in_reach += 1;
            *wanted.entry(k).or_default() += 1;
        } else {
            e.out_of_reach.push(k);
        }
    }
    let have = karana_literal_census(text);
    for (k, n) in wanted {
        let got = have.get(&k).copied().unwrap_or(0);
        if got < n {
            e.short.push(format!(
                "`अधिकम् {k}` is written {n} time(s) and only {got} `न्यूनलङ्घनम्` \
                 branch(es) carry {k} in the करण"
            ));
        }
    }
    e
}

/// **THE `अधिकम्` EXCHANGE, ASKED OF EVERY MODULE THAT WRITES ONE.**
///
/// # What this can and cannot see
///
/// It is a CENSUS BY VALUE, not a per-site pairing: a module that wrote
/// `अधिकम् ८` twice and dropped the exchange on one of them still answers two
/// branches with `८` in the करण if some other line put one there. Said here
/// rather than left to be read out of a green.
///
/// **SO EVERY MODULE IS ASKED TO PROVE THE SWEEP CAN SEE THE DEFECT IN IT.**
/// [`drop_the_exchange`] rewrites that module's own object the way a front end
/// that lowered `a अधिकम् b` as `Lt(a, b)` would emit it, and this sweep must
/// go red on the result — 16 of 16 modules do. A green from a blind instrument
/// and a green from a correct compiler are the same output, and per-module
/// sensitivity is the only thing that tells them apart: proving it on ONE
/// object would have licensed the other fifteen on nothing.
///
/// The out-of-reach sites are COUNTED AND NAMED. 19 of the corpus's `अधिकम्`
/// numerals are outside the `addi` immediate (`i64::MAX`, `2147483647`,
/// `524287`, `-2049`, and fourteen diagnostic codes in `nidana.t1` above 2047);
/// they arrive through `उपरिभारः` or the constant pool, [`Origin::Literal`]
/// cannot resolve either, and a sweep that dropped them would report 229 sites
/// covered while reading 210.
#[test]
fn every_adhikam_against_a_numeral_puts_that_numeral_in_the_karana() {
    let mut modules = 0usize;
    let mut in_reach = 0usize;
    let mut out_of_reach: Vec<String> = Vec::new();
    let mut short: Vec<String> = Vec::new();
    let mut silent: Vec<String> = Vec::new();
    let mut blind: Vec<String> = Vec::new();

    for p in corpus_paths() {
        let file = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let src = std::fs::read_to_string(&p).expect("readable");
        if source_adhikam_numerals(&src).is_empty() {
            continue;
        }
        // A SOURCE THAT WRITES `अधिकम्` AND EMITS NOTHING IS A REFUSAL, never a
        // module with no exchange to check.
        let Object::Emitted(text) = compile(&src) else {
            silent.push(format!("{file}: writes `अधिकम्` and emits no object"));
            continue;
        };
        modules += 1;
        let e = exchange_of(&src, &text);
        // **AND THIS MODULE'S OWN SENSITIVITY, MEASURED ON THIS MODULE.**
        // A green above says the exchange is there OR that the sweep cannot see
        // it here, and those are the same output. So the defect is written into
        // this object and the sweep is required to name it. Without this, a
        // module whose every `अधिकम्` constant also appears in some other
        // branch's अपादान would pass the mutation silently and be reported as
        // covered.
        let bitten = exchange_of(&src, &drop_the_exchange(&text));
        if bitten.short.is_empty() {
            blind.push(format!(
                "{file}: the exchange dropped on EVERY `न्यूनलङ्घनम्` and this                  sweep still reads it green — {} site(s) reported as covered                  that are not",
                e.in_reach
            ));
        }
        println!(
            "  {file:22} अधिकम्+numeral in reach {:<4} out of reach {:<3} short {}              (dropped: {})",
            e.in_reach,
            e.out_of_reach.len(),
            e.short.len(),
            bitten.short.len()
        );
        in_reach += e.in_reach;
        for k in e.out_of_reach {
            out_of_reach.push(format!("{file}: `अधिकम् {k}` — outside the addi immediate"));
        }
        short.extend(e.short.iter().map(|s| format!("{file}: {s}")));
    }

    println!("METRIC t1_corpus_adhikam_modules {modules}");
    println!("METRIC t1_corpus_adhikam_sites_in_reach {in_reach}");
    println!(
        "METRIC t1_corpus_adhikam_sites_out_of_reach {}",
        out_of_reach.len()
    );
    for s in &out_of_reach {
        println!("NOTE  {s}");
    }

    assert!(
        silent.is_empty(),
        "every source that writes `अधिकम्` must reach the emitter:\n  {}",
        silent.join("\n  ")
    );
    // THE HARNESS FIRST — every property below is vacuously true of a sweep
    // that read nothing.
    assert!(
        modules >= 16 && in_reach >= 200,
        "the corpus writes `अधिकम्` against a numeral in 16 modules at 210 \
         sites in reach of the addi immediate; this sweep found {modules} and \
         {in_reach}, which means the HARVESTER broke, not that the corpus \
         stopped comparing. A FLOOR and not a pin: the corpus gains `अधिकम्` \
         sites every week and a count pinned here would red on a source change \
         that is nobody's defect (owner ruling 2026-09-13, point 1)"
    );
    assert!(
        blind.is_empty(),
        "THE SWEEP CANNOT SEE THE DEFECT IT EXISTS FOR, in these modules — and          a green from a blind instrument is indistinguishable from a green from          a correct compiler:\n  {}",
        blind.join("\n  ")
    );
    assert!(
        short.is_empty(),
        "A LOWERED `अधिकम्` DID NOT PUT ITS CONSTANT IN THE करण. `ir.rs:81` \
         lowers it to `Lt` WITH ITS OPERANDS EXCHANGED, so the constant the \
         source wrote SECOND comes out FIRST. Dropping that exchange keeps the \
         mnemonic, the instruction count, the block graph and the object \
         length, and computes `>` as `<`:\n  {}",
        short.join("\n  ")
    );
}

/// **THE CASES THAT MUST STILL BE REFUSED.**
#[test]
fn every_adhikam_site_is_covered_by_a_reader_that_refuses_an_addition() {
    // ── ONE: AN ADDITION IS NOT A LITERAL. Both forms, on hand-built text,
    // because the emitter's own `योगः` lines are the ones that must keep
    // reading `Written` — `lower_constant` is the ONLY writer of the literal
    // form and every other `योगः` is arithmetic.
    let both_registers = "\
॥ वैश्विकम् क ॥
कॱॱ
योगः स्थिर०म् स्थिर१न स्थिर२न ।
समलङ्घनम् स्थिर०न शून्यःत् कपर्व१य् ।";
    assert_eq!(
        conditional_operands(both_registers)[0].karana,
        Origin::Written("योगः".to_string()),
        "`योगः कम् खन गन ।` ADDS TWO REGISTERS. Read as a literal it would \
         answer a constant this reader invented"
    );
    let register_plus_immediate = "\
॥ वैश्विकम् क ॥
कॱॱ
योगः स्थिर०म् स्थिर१न ७न ।
समलङ्घनम् स्थिर०न शून्यःत् कपर्व१य् ।";
    assert_eq!(
        conditional_operands(register_plus_immediate)[0].karana,
        Origin::Written("योगः".to_string()),
        "`योगः कम् खन ७न ।` ADDS ७ TO A REGISTER — it is `lower_constant`'s \
         `lui`+`addi` second half (`riscv64.rs:568`) and the frame adjustment \
         (`riscv64.rs:549`), and reading it as `Literal(७)` would answer a \
         WRONG CONSTANT with exactly the confidence of a right one"
    );

    // ── TWO: THE TWO FORMS THAT ARE LITERALS, INCLUDING THE NEGATIVE ONE.
    // `ऋण` and never a Latin minus (`riscv64.rs:118`); a reader built on
    // `devanagari_int` answers `None` here and the operand falls back to
    // `Written`, which is the correct-lowering bucket.
    let literals = "\
॥ वैश्विकम् क ॥
कॱॱ
योगः स्थिर०म् शून्यःन ६न ।
योगः स्थिर१म् शून्यःन ऋण२०४८न ।
न्यूनलङ्घनम् स्थिर०न स्थिर१त् कपर्व१य् ।";
    let c = &conditional_operands(literals)[0];
    assert_eq!(
        (&c.karana, &c.apadana),
        (&Origin::Literal(6), &Origin::Literal(-2048)),
        "`योगः <r>म् शून्यःन <k>न ।` is `addi rd, x0, k` and x0 is hardwired \
         zero, so the destination holds exactly `k`"
    );
    assert_eq!(c.karana.literal(), Some(6));
    assert_eq!(
        Origin::Zero.literal(),
        Some(0),
        "x0 IS the constant zero, and `<name> अधिकम् ०` is the shape the corpus \
         writes most"
    );

    // ── THREE: THE DEFECT ON A REAL OBJECT. Drop the exchange on every
    // `न्यूनलङ्घनम्` `kosha.t1` emits and the sweep must NAME the sites — this is
    // the one mutation no octet count, no block graph and no ladder rung sees.
    let src = std::fs::read_to_string(corpus_dir().join("kosha.t1")).expect("readable");
    let Object::Emitted(text) = compile(&src) else {
        panic!("kosha.t1 must emit; the control has to itself be real");
    };
    let clean = exchange_of(&src, &text);
    assert!(
        clean.in_reach >= 4 && clean.short.is_empty() && clean.out_of_reach.is_empty(),
        "THE CONTROL IS CLEAN FIRST, or the mutation below proves nothing: \
         {clean:?}",
    );
    let dropped = drop_the_exchange(&text);
    assert_ne!(dropped, text, "the mutation must actually bite");
    assert_eq!(
        dropped.lines().count(),
        text.lines().count(),
        "AND IT CHANGES NOTHING ELSE — same line count, same mnemonics, same \
         labels. That is why only the operand column can see it"
    );
    let bitten = exchange_of(&src, &dropped);
    assert_eq!(
        bitten.in_reach, clean.in_reach,
        "the same sites are still asked about"
    );
    assert!(
        !bitten.short.is_empty(),
        "THE DROPPED EXCHANGE MUST BE NAMED. An empty list here is a sweep that \
         cannot see the defect it exists for"
    );

    // ── FOUR: AND THE MUTATION IS NOT A BLANKET RED. Exchanging the operands
    // of a `समलङ्घनम्` is not a defect (equality commutes) and
    // `drop_the_exchange` must not touch one, or the control above would go red
    // on any object at all.
    let equality = "\
॥ वैश्विकम् क ॥
कॱॱ
समलङ्घनम् स्थिर०न स्थिर१त् कपर्व१य् ।";
    assert_eq!(
        drop_the_exchange(equality),
        equality,
        "only `न्यूनलङ्घनम्` is exchanged; a mutation that rewrote every branch \
         would red on modules that write no `अधिकम्` at all"
    );
}

// ── THE OTHER OPERAND SHAPE: `<name> अधिकम् <name>` ─────────────────────────
//
// The sweep above resolves `<name> अधिकम् <numeral>`, because a numeral becomes
// an `Origin::Literal` and a literal is a VALUE this reader can name. The other
// shape compares TWO NAMES, both operands come out `Origin::Slot`, and a slot
// number is an allocation artefact: `अ अधिकम् आ` and `अ न्यूनम् आ` are the same
// two slots in the opposite order, so nothing below said which order was which.
//
// What ties a slot back to a source word is the FRAME (`riscv64::Frame`): a
// routine's parameters and its `चरः` locals live in the LOCAL region, slot `k`
// at `8(num_spills + k)`, and `ir.t1:1320` (`स्थानीयघोषणम्`) hands out those
// `k` IN FIRST-DECLARATION ORDER — parameters at entry, then each `चरः` as the
// body reaches it, a repeated name reusing the slot it already has.

/// **WHY THE ORDER AND NOT THE NUMBER.** `ir.t1:1345` (`अनामस्थानम्`) cuts a
/// POOL OF EIGHT anonymous slots the first time a routine needs run growth, a
/// slice, a content equality or a length — lazily, in the middle of the body —
/// so a `चरः` declared after that point sits eight slots higher than its rank.
/// A reader that predicted slot NUMBERS would be wrong in every routine that
/// touches a run, and wrong with exactly the confidence of a right answer.
///
/// The pool is inserted at ONE position and never reorders what surrounds it,
/// so first-declaration ORDER survives it — and the order is all the exchange
/// needs: `a न्यूनम् b` emits करण `a`, `a अधिकम् b` emits करण `b`, and dropping
/// the exchange swaps them. `8·num_spills` cancels out of a comparison of two
/// slots in the same frame, which is why this needs no offset arithmetic at
/// all — only the WINDOW, to tell a local from a spill.
#[derive(Debug)]
struct SourceRoutine {
    name: String,
    /// Every name that gets a frame slot, in first-declaration order.
    slots: Vec<String>,
    /// The DECLARED TYPE TEXT of every name in `slots`, by name — the tokens
    /// after its `ॱॱ`, separators dropped. Read by [`extra_pairs`] and by
    /// nothing above it: `समम्`/`असमम्` between two RUNS is a CALL to
    /// `खण्डसाम्यरचना` and not a branch at all (`ir.t1:2200`), so the type is
    /// what tells an in-reach site from one that emits no comparison.
    types: BTreeMap<String, String>,
    /// `(left, operator, right)` for each `यदि`/`यावत्` head whose WHOLE
    /// condition is `<token> <op> <token>`. Only the five-token head is taken:
    /// a longer one may open with `आरभ्य`, a field read or an index, and a
    /// harvester that grabbed the token before `अधिकम्` there would name a
    /// fragment of an expression as if it were a variable.
    sites: Vec<(String, String, String)>,
    /// The SAME five-token heads for the THREE OTHER `compare_op`s — `समम्`,
    /// `असमम्`, `बृहत्समम्`. They are kept apart from `sites` because the order
    /// census above is built on the `अधिकम्` EXCHANGE and these three have
    /// none: each lowers to its own branch word with its operands in written
    /// order (`ir.t1:2186`–`:2189`). See [`extra_pairs`] — they are folded into
    /// the CUT RECOVERY, which needs only that a site pair two slots of one
    /// frame, and nowhere else.
    other: Vec<(String, String, String)>,
}

/// **THE SOURCE SIDE — every routine, its slot-bearing names in declaration
/// order, and its two-name comparisons.**
///
/// A routine opens with `वृत्तिः` or `सार्वजनिक वृत्तिः` at COLUMN ZERO and
/// closes with `इति` at column zero; inner `इति`s are indented, and a nested
/// block that closed the routine would attribute the rest of the file's `चरः`
/// declarations to nothing. `shrinkhala.t1:1625` writes a routine header INSIDE
/// a string literal, indented — which is why the column matters and not the
/// keyword.
///
/// **A PARAMETER IS WHAT STANDS BEFORE `ॱॱ`, NOT WHAT FOLLOWS `ऽ`.** The first
/// version of this read parameters as `ऽ`-separated, and `ashtaka.t1:103` —
/// `आदाय पाठ ॱॱ अङ्कः अन्तः अ८ आरम्भः ॱॱ न६४ सीमा ॱॱ न६४ ददाति` — writes three
/// parameters with no separator at all. That reader saw ONE, so `आरम्भः` and
/// `सीमा` had no slots, and their comparison was reported OUT OF REACH: a
/// harvester's own blind spot, arriving in the bucket reserved for the
/// corpus's. It cost 24 of 216 sites.
///
/// **AND A `चरः` INSIDE `उक्तम् … इति` IS TEXT, NOT A DECLARATION.** Found by
/// the residue claim below and by nothing before it: `shrinkhala.t1:3370`
/// builds the scale ladder's source as a STRING, and that string contains
/// `चरः स ॱॱ न६४ भवति ० ।`. This reader minted `स` a slot, which shifted
/// every rank declared after it by ONE — invisible to the direction census,
/// which is offset-blind, and invisible to [`pairable`], which refused the
/// routine for the pool. The residue named it: ranks `(१०, ८)` against an
/// object carrying `(९, ७)`. The literal runs from `उक्तम्` to the FIRST
/// `इति` after it — `:3372` writes `उक्तम् प्रत्यागमनम् स । इ इति ।`, an
/// `आदि` with no `इति` of its own inside one — so this counts nothing and
/// nests nothing; it only stops minting until the literal closes.
fn source_routines(src: &str) -> Vec<SourceRoutine> {
    let mut out: Vec<SourceRoutine> = Vec::new();
    let mut cur: Option<SourceRoutine> = None;
    let mut in_literal = false;
    for line in src.lines() {
        // A COMMENT IS STRIPPED FIRST, as `source_adhikam_numerals` does it.
        let body = line.split('॰').next().unwrap_or("");
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = body.split_whitespace().collect();
        let opens = !line.starts_with(' ')
            && (f.first() == Some(&"वृत्तिः")
                || (f.first() == Some(&"सार्वजनिक") && f.get(1) == Some(&"वृत्तिः")));
        if opens {
            if let Some(r) = cur.take() {
                out.push(r);
            }
            let at = usize::from(f[0] == "सार्वजनिक") + 1;
            let mut slots: Vec<String> = Vec::new();
            let mut types: BTreeMap<String, String> = BTreeMap::new();
            if let Some(a) = f.iter().position(|t| *t == "आदाय") {
                let end = f.iter().position(|t| *t == "ददाति").unwrap_or(f.len());
                let seg = &f[a + 1..end];
                // A PARAMETER'S TYPE RUNS TO THE NEXT NAME, not to the next
                // `ॱॱ`: `आरम्भः ॱॱ न६४ सीमा ॱॱ न६४` puts `सीमा` one before the
                // second mark, so the first type is the tokens strictly between.
                let marks: Vec<usize> = seg
                    .iter()
                    .enumerate()
                    .filter(|(j, t)| **t == "ॱॱ" && *j > 0)
                    .map(|(j, _)| j)
                    .collect();
                for (m, j) in marks.iter().enumerate() {
                    let stop = marks.get(m + 1).map_or(seg.len(), |n| n - 1);
                    types.insert(
                        seg[j - 1].to_string(),
                        seg[j + 1..stop]
                            .iter()
                            .copied()
                            .filter(|t| *t != "ऽ")
                            .collect::<Vec<&str>>()
                            .join(" "),
                    );
                    slots.push(seg[j - 1].to_string());
                }
            }
            cur = Some(SourceRoutine {
                name: f[at].to_string(),
                slots,
                types,
                sites: Vec::new(),
                other: Vec::new(),
            });
            in_literal = false;
            continue;
        }
        if line == "इति" {
            if let Some(r) = cur.take() {
                out.push(r);
            }
            in_literal = false;
            continue;
        }
        let Some(r) = cur.as_mut() else { continue };
        // EVERYTHING BETWEEN `उक्तम्` AND ITS `इति` IS A STRING, dropped here
        // before anything below reads it. A literal may run past the end of its
        // line, so the flag survives the line; a routine's own header or
        // column-zero `इति` clears it above.
        let mut f2: Vec<&str> = Vec::with_capacity(f.len());
        for t in &f {
            if in_literal {
                in_literal = *t != "इति";
            } else if *t == "उक्तम्" {
                in_literal = true;
            } else {
                f2.push(*t);
            }
        }
        let f = f2;
        for (i, t) in f.iter().enumerate() {
            // `चरः X ॱॱ <type> भवति …` — the `ॱॱ` is required, so a `चरः`
            // reached as anything but a declaration cannot mint a slot.
            if *t == "चरः"
                && f.get(i + 2) == Some(&"ॱॱ")
                && let Some(n) = f.get(i + 1)
                && !r.slots.iter().any(|x| x == n)
            {
                r.slots.push((*n).to_string());
                // `चरः X ॱॱ <type> भवति …` — the type is what stands between.
                r.types.insert(
                    (*n).to_string(),
                    f.get(i + 3..)
                        .unwrap_or_default()
                        .iter()
                        .copied()
                        .take_while(|x| *x != "भवति")
                        .collect::<Vec<&str>>()
                        .join(" "),
                );
            }
        }
        if let ["यदि" | "यावत्", a, op, b, "आदि"] = f.as_slice() {
            let site = || ((*a).to_string(), (*op).to_string(), (*b).to_string());
            if *op == "अधिकम्" || *op == "न्यूनम्" {
                r.sites.push(site());
            } else if branch_of(op).is_some() {
                r.other.push(site());
            }
        }
    }
    if let Some(r) = cur {
        out.push(r);
    }
    out
}

/// Which way round a `न्यूनलङ्घनम्`'s two LOCAL slots stand. Counted and not
/// paired, for `exchange_of`'s reason: `यावत्` rotation emits one source
/// comparison twice, so the claim is a floor.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct Directions {
    ascending: usize,
    descending: usize,
}

/// `routine label -> [first local offset, one past the last)`, in octets off
/// `स्तूपसूचकः`. See [`compile_framed`] for what lies below the window.
type Windows = BTreeMap<String, (u64, u64)>;

/// `routine label -> its whole frame`, carried ALONGSIDE [`Windows`] rather
/// than replacing it.
///
/// A window is two numbers and answers one question: is this offset in the
/// local region? That is all the pairing sweeps below need, and nine of them
/// ask it. [`emitted::slot_band`] asks a wider question — which of the frame's
/// FOUR regions an offset falls in, and whether it falls in none of them — and
/// that needs the saved list, `ra_offset` and `bytes`, which a window has
/// thrown away. Widening `Windows` itself would have rewritten nine call sites
/// that do not want the extra answer.
type Frames = BTreeMap<String, riscv64::Frame>;

/// What this sweep needs that [`compile`] does not answer: the FRAME of every
/// routine as well as its text. Its own loader, for the reason `compile`'s
/// margin gives.
enum Framed {
    /// The object, its routines' local windows, and their whole frames.
    Emitted(String, Windows, Frames),
    DeclaresNoModule,
    Refused(String),
}

/// Compile one source and lay out every routine's frame the way the emitter
/// does — `allocate_registers` then `frame_layout`, the product's own two
/// functions and not a second arithmetic that could drift from them.
///
/// The window is `[8·num_spills, 8·(num_spills + num_locals))`. **BELOW IT ARE
/// THE ALLOCATOR'S SPILL SLOTS**, which are also loaded off `स्तूपसूचकः` and
/// also read `Origin::Slot`: a spilled temporary has no source name, its number
/// says nothing about declaration order, and matching one against slot ० would
/// be an answer invented out of an allocation artefact.
fn compile_framed(src: &str) -> Framed {
    let Some(module) = module_name(src) else {
        return Framed::DeclaresNoModule;
    };
    let mut front = match Front::load(&spec_root()) {
        Ok(f) => f,
        Err(e) => return Framed::Refused(format!("the front end does not load: {e}")),
    };
    let built = (|| -> Result<(String, Windows, Frames), String> {
        front.lex(src)?;
        front.parse()?;
        front.resolve()?;
        front.typecheck()?;
        front.build_ir()?;
        let module = front.module(&module, None)?;
        let mut windows = Windows::new();
        let mut frames = Frames::new();
        for f in &module.functions {
            let alloc = allocate_registers(f, riscv64::ALLOCATABLE);
            let frame = riscv64::frame_layout(&alloc, riscv64::count_locals(f));
            let label =
                riscv64::routine_label(&module.names, f.name).map_err(|e| format!("{e:?}"))?;
            windows.insert(
                label.clone(),
                (
                    8 * frame.num_spills as u64,
                    8 * (frame.num_spills + frame.num_locals) as u64,
                ),
            );
            // The SAME `frame` the window was derived from, so the two can
            // never disagree about one routine.
            frames.insert(label, frame);
        }
        let text = riscv64::emit_module(&module).map_err(|e| format!("{e:?}"))?;
        Ok((text, windows, frames))
    })();
    match built {
        Ok((t, w, f)) => Framed::Emitted(t, w, f),
        Err(e) => Framed::Refused(e),
    }
}

/// `routine label -> the (करण, अपादान) LOCAL SLOT NUMBERS of each of its
/// slot-against-slot `न्यूनलङ्घनम्`s`. Not offsets: `Frame` puts local slot `k`
/// at `8·(num_spills + k)` (`riscv64.rs:406`), so the window's own base is
/// divided out here and what remains is the number `स्थानीयघोषणम्` handed out.
type Pairs = BTreeMap<String, Vec<(usize, usize)>>;

/// **ONE SIDE OF A COMPARISON AGAINST A CONSTANT — THE HALF-SITE.**
///
/// `(slot, value, name_is_karana)`. On the WANT side the first field is the
/// name's declaration RANK and on the HAVE side it is the slot number the
/// object carries, and [`slot_at`] is what maps one to the other under a
/// candidate cut — exactly as for a two-name pair, with the difference that
/// only ONE of the two operands moves with the cut.
///
/// **WHY THE THIRD FIELD.** `क न्यूनम् ५` and `५ न्यूनम् क` name the same slot
/// and the same constant and are DIFFERENT branches, so a reader that dropped
/// the side would accept an emitter that swapped them — and swapping them is
/// the defect this whole file exists to catch, in its one-name form.
type Half = (usize, i128, bool);

/// `routine label -> the half-sites of one mnemonic`, the object's side.
type HalfPairs = BTreeMap<String, Vec<Half>>;

/// **EVERY BRANCH OF `mnemonic` WITH ONE LOCAL SLOT AND ONE CONSTANT.**
///
/// The two-slot reader above ([`emitted_pairs_of`]) drops these, and the
/// corpus writes far more of them than it writes two-name heads. A constant
/// pins no rank of its own, but the OTHER operand is a declared name, and
/// `slot_at(rank, b)` is a claim about it under every candidate cut — so each
/// half-site is one more constraint on the SAME base, of the same kind as a
/// pair and weaker only in that it names one slot instead of two.
///
/// **AND THE SPILL IS REFUSED HERE TOO.** A slot below the window is the
/// allocator's and not a declared name (`compile_framed`'s margin), so it is
/// named and dropped rather than folded in with a rank invented for it.
fn emitted_halves_of(
    text: &str,
    windows: &Windows,
    mnemonic: &str,
    spilled: &mut Vec<String>,
) -> HalfPairs {
    let mut out: HalfPairs = BTreeMap::new();
    for c in conditional_operands(text) {
        if c.mnemonic != mnemonic {
            continue;
        }
        // EXACTLY ONE SLOT AND EXACTLY ONE CONSTANT. Two slots belong to
        // `emitted_pairs_of`; two constants name no frame slot at all; a
        // `Written`/`Undefined` operand is one this reader cannot resolve and
        // must not guess at.
        let half = match (c.karana.slot(), c.apadana.slot()) {
            (Some(k), None) => c.apadana.literal().map(|v| (k, v, true)),
            (None, Some(a)) => c.karana.literal().map(|v| (a, v, false)),
            _ => None,
        };
        let Some((slot, value, karana)) = half else {
            continue;
        };
        let Some((lo, hi)) = windows.get(&c.routine) else {
            continue;
        };
        if !(*lo..*hi).contains(&slot) {
            spilled.push(site(&c));
            continue;
        }
        out.entry(c.routine.clone())
            .or_default()
            .push((((slot - lo) / 8) as usize, value, karana));
    }
    out
}

/// Every `न्यूनलङ्घनम्` whose BOTH operands are local slots of the routine it
/// stands in, by routine, as the exact pair of slot NUMBERS. An operand outside
/// the local window is a spill and is dropped here — it is counted and named by
/// the sweep, never folded in as if it were a declared name.
fn emitted_pairs(text: &str, windows: &Windows, spilled: &mut Vec<String>) -> Pairs {
    emitted_pairs_of(text, windows, "न्यूनलङ्घनम्", spilled)
}

/// **THE BRANCH WORD EACH OTHER `compare_op` LOWERS TO, AND THE THREE THAT ARE
/// NOT `न्यूनलङ्घनम्`.**
///
/// `ir.t1:2186`–`:2189` reads the sub-kind off the expression and
/// `riscv64::branch_word` names it: `समम्` → `समलङ्घनम्`, `असमम्` →
/// `विषमलङ्घनम्`, `बृहत्समम्` → `अन्यूनलङ्घनम्`. **NONE OF THE THREE EXCHANGES
/// ITS OPERANDS** — only `अधिकम्` does (`ir.t1:2191`, `विपर्ययः भवति सत्यम्`,
/// which is why `दक्षिण` goes first for it and for nothing else) — so each of
/// these predicts `(करण, अपादान)` in WRITTEN order. `न्यूनम्` and `अधिकम्` are
/// deliberately absent: they are the order census's own two and are harvested
/// into [`SourceRoutine::sites`].
fn branch_of(op: &str) -> Option<&'static str> {
    match op {
        "समम्" => Some("समलङ्घनम्"),
        "असमम्" => Some("विषमलङ्घनम्"),
        "बृहत्समम्" => Some("अन्यूनलङ्घनम्"),
        _ => None,
    }
}

/// The types a `compare_op` between two names lowers to a BRANCH for — the
/// scalar words, `ADR-0032`'s integer widths and `बूल`.
///
/// **THE CASE THAT MUST BE REFUSED, AND IT IS IN THE CORPUS TEN TIMES.**
/// `ir.t1:2200` sends `समम्`/`असमम्` between two RUNS to `खण्डसाम्यरचना` — a
/// CALL — so the object carries no `समलङ्घनम्` for it at all, and a harvester
/// that predicted one would empty the routine's candidate set and report the
/// compiler red for doing the right thing. An allow-list and not a deny-list,
/// so a type nobody has thought about lands in the NAMED bucket rather than in
/// the claim.
const SCALAR_TYPES: [&str; 9] = ["अ८", "अ१६", "अ३२", "अ६४", "न८", "न१६", "न३२", "न६४", "बूल"];

/// **THE CONSTANTS THE OBJECT MATERIALISES AS ONE `addi`, AND THE ONES IT DOES
/// NOT — FOUND BY THIS CYCLE'S OWN RED AND NOT BY READING THE EMITTER.**
///
/// `conditional_operands` answers `Origin::Literal` for exactly one shape:
/// `योगः <r>म् शून्यःन <k>न ।`, an `addi` off the hardwired zero. The emitter
/// writes that shape only for a value inside the `I`-immediate window —
/// `yantrotsarjana.t1:898`–`:902` guards on `ध्रुवम् अधिकम् ऋण२०४९` and
/// `ध्रुवम् न्यूनम् २०४८` — and for anything else it writes `उपरिभारः` and
/// then a `योगः` off THAT register, which this reader answers `Written` for
/// and must, since the pair carries a value it cannot know.
///
/// **SO A SITE AGAINST A LARGER CONSTANT IS ONE THE OBJECT CANNOT CARRY**, and
/// predicting it empties the routine's candidate set and reads the compiler
/// RED for emitting the right object. `यन्त्रोत्सर्जनयन्त्रशाखादूरपरीक्षा` is
/// that routine: `दूरम् बृहत्समम् ४०९६`, one site, and the whole recovery of a
/// ten-name frame went to NO cut position until this refusal was written.
const IMMEDIATE: std::ops::RangeInclusive<i128> = -2048..=2047;

/// `routine label -> the (करण, अपादान) LOCAL SLOT NUMBERS of each of its
/// slot-against-slot branches carrying `mnemonic`.
fn emitted_pairs_of(
    text: &str,
    windows: &Windows,
    mnemonic: &str,
    spilled: &mut Vec<String>,
) -> Pairs {
    let mut out: Pairs = BTreeMap::new();
    for c in conditional_operands(text) {
        if c.mnemonic != mnemonic {
            continue;
        }
        let (Origin::Slot(k), Origin::Slot(a)) = (&c.karana, &c.apadana) else {
            continue;
        };
        let Some((lo, hi)) = windows.get(&c.routine) else {
            continue;
        };
        if !(*lo..*hi).contains(k) || !(*lo..*hi).contains(a) {
            spilled.push(site(&c));
            continue;
        }
        out.entry(c.routine.clone())
            .or_default()
            .push((((k - lo) / 8) as usize, ((a - lo) / 8) as usize));
    }
    out
}

/// The direction census, derived from the pairs rather than counted a second
/// way. `k < a` is preserved by subtracting the window base and dividing by
/// eight, so the two readings cannot drift apart.
fn directions_of(pairs: &Pairs) -> BTreeMap<String, Directions> {
    let mut out: BTreeMap<String, Directions> = BTreeMap::new();
    for (r, ps) in pairs {
        let d = out.entry(r.clone()).or_default();
        for (k, a) in ps {
            if k < a {
                d.ascending += 1;
            } else if k > a {
                d.descending += 1;
            }
        }
    }
    out
}

/// One source's `<name> <op> <name>` sites, folded into the direction census
/// the object must carry, by routine label.
///
/// `a न्यूनम् b` is `Cmp(Lt, a, b)` — करण `a`. `a अधिकम् b` is the SAME `Lt`
/// with its operands exchanged (`ir.rs:81`) — करण `b`. So both operators
/// predict, and dropping the exchange swaps every prediction at once.
fn predicted_pairs(
    module: &str,
    routines: &[SourceRoutine],
    out_of_reach: &mut Vec<String>,
    numeral: &mut usize,
) -> Pairs {
    let mut want: Pairs = BTreeMap::new();
    for r in routines {
        for (a, op, b) in &r.sites {
            // THE NUMERAL SHAPE IS THE OTHER SWEEP'S. Counted here so the two
            // buckets add up to the sites harvested, never named as a defect.
            if emitted::devanagari_signed(a).is_some() || emitted::devanagari_signed(b).is_some() {
                *numeral += 1;
                continue;
            }
            let (rank_a, rank_b) = (
                r.slots.iter().position(|t| t == a),
                r.slots.iter().position(|t| t == b),
            );
            let (Some(x), Some(y)) = (rank_a, rank_b) else {
                // **THE CASE THAT MUST STILL BE REFUSED.** A name that is
                // neither a parameter nor a `चरः` of this routine is a global
                // read or an import: it has NO slot, `Origin::Slot` will never
                // answer for it, and matching it against slot ० would be a
                // reading invented out of nothing.
                out_of_reach.push(format!(
                    "{}: `{a} {op} {b}` — {} names no frame slot of this routine",
                    r.name,
                    if rank_a.is_none() { a } else { b }
                ));
                continue;
            };
            if x == y {
                out_of_reach.push(format!(
                    "{}: `{a} {op} {b}` — one name on both sides, so no order",
                    r.name
                ));
                continue;
            }
            let pair = if op == "अधिकम्" {
                (y, x)
            } else {
                (x, y)
            };
            want.entry(format!("{module}{}", r.name))
                .or_default()
                .push(pair);
        }
    }
    want
}

/// What [`extra_pairs`] harvested — the evidence, and every bucket it is not in.
#[derive(Default)]
struct Extra {
    /// `routine label -> branch word -> predicted (करण, अपादान) rank pairs`.
    want: BTreeMap<String, ByWord>,
    /// `routine label -> branch word -> predicted HALF-sites`: one declaration
    /// rank, the constant it stands against, and which side the name is on.
    /// See [`Half`] — a numeral pins no rank of its own, but the name beside it
    /// does, and that is a constraint on the same base.
    half_want: BTreeMap<String, ByWordHalf>,
    /// Sites NAMED and not claimed, because the declared type is not a scalar
    /// one. Named rather than counted: this is the bucket a `समम्` between two
    /// runs belongs in, and a silent one would hide a mnemonic that emits no
    /// branch behind a figure.
    refused: Vec<String>,
    /// Every five-token head of the three other `compare_op`s this module
    /// writes — the TOTAL every bucket below must add back up to. Without it
    /// the buckets are five figures that agree with nothing.
    seen: usize,
    /// Sites with at least one NUMERAL operand. Split four ways below, because
    /// they are not one shape: `numeral` is their sum and never a bucket.
    numeral: usize,
    /// Of `numeral`: folded into `half_want`.
    half_claimed: usize,
    /// Of `numeral`: the name beside the constant holds no frame slot — a
    /// module global or an import. **THE CASE THAT MUST STILL BE REFUSED.**
    half_unslotted: usize,
    /// Of `numeral`: BOTH operands are constants, so there is no name and no
    /// rank to claim anything about.
    half_both: usize,
    /// Of `numeral`: NAMED and not claimed, the declared type not being scalar.
    half_refused: Vec<String>,
    /// Of `numeral`: NAMED and not claimed, the constant being too wide for the
    /// one `addi` the object would have to carry. See [`IMMEDIATE`].
    half_out_of_range: Vec<String>,
    /// **THE TWO SHAPES THE ONE `unslotted` FIGURE USED TO HIDE.** A name that
    /// is a global read or an import holds NO frame slot, and a site with the
    /// same name on both sides holds ONE — they are dropped for opposite
    /// reasons, and one figure over both cannot say which the corpus grew.
    unslotted: usize,
    one_name: usize,
    /// Sites folded into `want`, summed over routines and mnemonics.
    claimed: usize,
}

/// **THE OTHER THREE `compare_op`s, HARVESTED AS EXTRA EVIDENCE FOR ONE BASE.**
///
/// The cut recovery above reads only `न्यूनलङ्घनम्`, and most routines carry
/// one to four of those — too few to leave a single candidate standing, which
/// is why 77 of 108 came out AMBIGUOUS. But `समम्`, `असमम्` and `बृहत्समम्` each
/// also pair TWO SLOTS OF ONE FRAME, and the cut those slots imply is the SAME
/// cut: `अनामस्थानम्` runs once per routine (`ir.t1:1346`), not once per
/// mnemonic. So every one of their sites is another constraint on the same
/// single base.
///
/// **NONE OF THE THREE EXCHANGES** (see [`branch_of`]), so each predicts its
/// operands in written order — and that is a claim this harvester makes and the
/// object can refuse.
///
/// **AND WHAT IS REFUSED.** A site whose declared types are not both scalar:
/// `ir.t1:2200` lowers `समम्`/`असमम्` between two RUNS to a CALL, so no branch
/// exists to match. Nine `समम्` sites and one `असमम्` site in the corpus are
/// that shape, they are NAMED, and predicting a branch for them would empty
/// their routines' candidate sets and read the compiler red for being right.
fn extra_pairs(module: &str, routines: &[SourceRoutine]) -> Extra {
    let mut e = Extra::default();
    for r in routines {
        let label = format!("{module}{}", r.name);
        let scalar = |n: &String| {
            r.types
                .get(n)
                .is_some_and(|t| SCALAR_TYPES.contains(&t.as_str()))
        };
        for (a, op, b) in &r.other {
            let Some(word) = branch_of(op) else { continue };
            e.seen += 1;
            // **THE NUMERAL SHAPES, SPLIT FOUR WAYS.** One constant and one
            // name is a HALF-SITE and is folded; two constants name no frame
            // slot; a name with no slot is the refusal below.
            let (na, nb) = (emitted::devanagari_signed(a), emitted::devanagari_signed(b));
            if na.is_some() || nb.is_some() {
                e.numeral += 1;
                // `(name, constant, the name is the करण)`. NONE of these three
                // words exchanges its operands (`ir.t1:2186`–`:2189`), so the
                // side is the written side.
                let half = match (na, nb) {
                    (None, Some(v)) => Some((a, v, true)),
                    (Some(v), None) => Some((b, v, false)),
                    _ => None,
                };
                let Some((name, value, karana)) = half else {
                    e.half_both += 1;
                    continue;
                };
                let Some(rank) = r.slots.iter().position(|t| t == name) else {
                    // **THE CASE THAT MUST STILL BE REFUSED.** A global read or
                    // an import holds no frame slot, `Origin::Slot` will never
                    // answer for it, and predicting one would empty this
                    // routine's candidate set and read the compiler RED for
                    // being right.
                    e.half_unslotted += 1;
                    continue;
                };
                if !scalar(name) {
                    e.half_refused.push(format!(
                        "{}: `{a} {op} {b}` — `{name}` is declared `{}`, and \
                         only a comparison of SCALARS becomes a branch",
                        r.name,
                        r.types.get(name).map_or("?", String::as_str),
                    ));
                    continue;
                }
                if !IMMEDIATE.contains(&value) {
                    // **THE CASE THAT MUST STILL BE REFUSED, AND THE CORPUS HAS
                    // IT.** See [`IMMEDIATE`]: outside the `addi` window the
                    // emitter writes `उपरिभारः` and a `योगः` off it, which is
                    // no `Origin::Literal`, so the object carries NO half-site
                    // for this head and every candidate cut would die on a
                    // constraint the compiler was right to emit differently.
                    e.half_out_of_range.push(format!(
                        "{}: `{a} {op} {b}` — {value} is outside the `addi` \
                         immediate, so the object materialises it with \
                         `उपरिभारः` (`yantrotsarjana.t1:898`) and carries no \
                         `योगः <r>म् शून्यःन <k>न ।` for this reader to match",
                        r.name
                    ));
                    continue;
                }
                e.half_want
                    .entry(label.clone())
                    .or_default()
                    .entry(word)
                    .or_default()
                    .push((rank, value, karana));
                e.half_claimed += 1;
                continue;
            }
            let (Some(x), Some(y)) = (
                r.slots.iter().position(|t| t == a),
                r.slots.iter().position(|t| t == b),
            ) else {
                e.unslotted += 1;
                continue;
            };
            if x == y {
                // ONE NAME ON BOTH SIDES holds a slot and carries no ORDER —
                // the opposite reason from the bucket above, and folded into it
                // this figure could not say which of the two the corpus grew.
                e.one_name += 1;
                continue;
            }
            if !scalar(a) || !scalar(b) {
                e.refused.push(format!(
                    "{}: `{a} {op} {b}` — declared `{}` and `{}`, and only a \
                     comparison of two SCALARS becomes a branch (`ir.t1:2200` \
                     sends two runs under `समम्`/`असमम्` to `खण्डसाम्यरचना`, a \
                     CALL, and a `सम्भाव्य` is no comparable word at all)",
                    r.name,
                    r.types.get(a).map_or("?", String::as_str),
                    r.types.get(b).map_or("?", String::as_str),
                ));
                continue;
            }
            e.want
                .entry(label.clone())
                .or_default()
                .entry(word)
                .or_default()
                .push((x, y));
            e.claimed += 1;
        }
    }
    e
}

/// **WHICH ROUTINES' SLOT NUMBERS ARE RECOVERED, AND WHICH ARE NOT.**
///
/// `स्थानीयघोषणम्` (`ir.t1:1319`) hands a name the next slot number, so a
/// name's number IS its first-declaration rank — *unless* `अनामस्थानम्`
/// (`ir.t1:1336`) has run first. That cuts a POOL OF EIGHT anonymous slots at
/// the first run growth, slice, content equality or length in the routine, and
/// every name declared after the cut is shifted by eight. The pool's position
/// is not a property of the routine's declarations at all — it is where in the
/// LOWERING the first such operation fell — so a source-side reader cannot
/// know it.
///
/// It can know whether there is one. `count_locals` is one past the highest
/// slot the routine loads or stores, so a routine whose emitted local count
/// equals its declared-name count HAS NO ANONYMOUS SLOT: there is nothing for a
/// shift to come from, and slot `k` is declaration rank `k` as a consequence
/// and not as a guess.
///
/// **THE CASE THAT MUST BE REFUSED.** Any other count — the pool cut (`+8`), or
/// a declared name never loaded (`<`) — leaves the mapping unknown, and
/// choosing the offset that makes the object agree would be fitting the answer
/// to the data. Those routines are reported UNPAIRED and fall back to the
/// direction census, which needs no numbering.
fn pairable(want: &Pairs, routines: &[SourceRoutine], module: &str, windows: &Windows) -> Pairs {
    let mut out = Pairs::new();
    for r in routines {
        let label = format!("{module}{}", r.name);
        let Some(ps) = want.get(&label) else { continue };
        let Some((lo, hi)) = windows.get(&label) else {
            continue;
        };
        if ((hi - lo) / 8) as usize == r.slots.len() {
            out.insert(label, ps.clone());
        }
    }
    out
}

/// Every predicted `(करण, अपादान)` slot pair the object does not carry as often
/// as the source demands it. A FLOOR per DISTINCT PAIR, not a total: `यावत्`
/// rotation emits one source comparison twice, so the object may carry more.
fn pair_shortfalls(want: &Pairs, have: &Pairs) -> Vec<String> {
    let mut out = Vec::new();
    for (r, ps) in want {
        let mut wanted: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for p in ps {
            *wanted.entry(*p).or_default() += 1;
        }
        let carried = have.get(r).map(Vec::as_slice).unwrap_or(&[]);
        for (p, n) in wanted {
            let got = carried.iter().filter(|q| **q == p).count();
            if got < n {
                out.push(format!(
                    "{r}: the source wants {n} `न्यूनलङ्घनम्` comparing local \
                     slot {} against local slot {}; the object carries {got} \
                     (it carries {:?})",
                    p.0, p.1, carried
                ));
            }
        }
    }
    out
}

/// **THE EXACT-SLOT CLAIM FOR A NAME AGAINST A CONSTANT — THE ONE HALF-SITE
/// USE THAT CAN GO RED.**
///
/// Every other reader of a half-site in this file only ever NARROWS a candidate
/// cut, and a routine [`pairable`] accepts has no cut to narrow: its declared
/// name count equals its emitted local slot count, so there is no anonymous
/// pool, and slot `k` IS declaration rank `k` (`ir.t1:1319`). For those
/// routines a predicted `(rank, constant, side)` is a prediction of the exact
/// `(slot, constant, side)` triple the object must carry under the SAME branch
/// word — the [`pair_shortfalls`] claim in its one-name form, and the only
/// half-site use whose miss is a RED rather than an AMBIGUOUS.
///
/// **WHY THE SIDE IS HALF THE CLAIM.** `क समम् ५` and `५ समम् क` name the same
/// slot and the same constant and are DIFFERENT branches (see [`Half`]); none
/// of the three words exchanges its operands (`ir.t1:2186`–`:2189`), so the
/// object must carry the WRITTEN side and a reader that dropped it would accept
/// an emitter that swapped them.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
/// * A routine [`pairable`] refuses has NO exact slot — its rank is its slot or
///   its rank plus eight and the object does not say which — so it is skipped
///   here and keeps the cut recovery, which needs no numbering. Matching it
///   against its rank would read the compiler RED for being right.
/// * A constant outside the `addi` immediate carries no `Origin::Literal` at
///   all (see [`IMMEDIATE`]): [`extra_pairs`] never folds it into `half_want`,
///   so it stays in `half_out_of_range` and can never become a shortfall here.
/// * The floor is per DISTINCT triple and not a total, for the same reason
///   [`pair_shortfalls`] is one: `यावत्` rotation emits one source comparison
///   twice, so the object may carry MORE than the source demands.
fn half_shortfalls(
    paired: &Pairs,
    half_want: &BTreeMap<String, ByWordHalf>,
    half_have: &BTreeMap<&'static str, HalfPairs>,
) -> Vec<String> {
    const NONE: &[Half] = &[];
    let mut out = Vec::new();
    for (r, by_word) in half_want {
        if !paired.contains_key(r) {
            continue;
        }
        for (word, hs) in by_word {
            let mut wanted: BTreeMap<Half, usize> = BTreeMap::new();
            for h in hs {
                *wanted.entry(*h).or_default() += 1;
            }
            let carried = half_have
                .get(word)
                .and_then(|p| p.get(r))
                .map_or(NONE, Vec::as_slice);
            for (h, n) in wanted {
                let got = carried.iter().filter(|q| **q == h).count();
                if got < n {
                    out.push(format!(
                        "{r}: the source wants {n} `{word}` comparing local \
                         slot {} against the constant {}, the name in the {}; \
                         the object carries {got} (it carries {carried:?})",
                        h.0,
                        h.1,
                        if h.2 {
                            "करण"
                        } else {
                            "अपादान"
                        }
                    ));
                }
            }
        }
    }
    out
}

/// The anonymous pool is EIGHT slots wide — `अनामस्थलसंख्या` (`ir.t1:1344`),
/// and the one number in this whole arrangement that is a CONSTANT of the
/// lowering rather than an accident of it.
const POOL_SLOTS: usize = 8;

/// **WHAT A ROUTINE'S NUMBERING STILL SAYS WHEN ITS POOL POSITION IS NOT
/// RECOVERED — THE RESIDUE.**
///
/// `स्थानीयघोषणम्` (`ir.t1:1319`) hands out slots in declaration order and
/// `अनामस्थानम्` (`ir.t1:1345`) interrupts that run EXACTLY ONCE, by EXACTLY
/// `अनामस्थलसंख्या = ८`. So whatever rank the cut fell at, a name of rank `r`
/// holds slot `r` (declared before the cut) or slot `r + ८` (declared after
/// it) — never anything else, and in both cases
///
/// > `slot ≡ rank (mod ८)`.
///
/// That is a per-site claim about the exact `(करण, अपादान)` NUMBERS which
/// needs no knowledge of where the cut fell, so it reaches the routines
/// [`pairable`] refuses. It is not the direction claim in another hat: `(१, ४)`
/// and `(४, १)` have different residues, so the dropped exchange inverts it
/// even in a routine whose ascending and descending counts balance.
///
/// **THE CASE THAT MUST STILL BE REFUSED.** Two ranks CONGRUENT mod eight —
/// ranks `२` and `१०`, say — carry the same residue pair whichever way round
/// they stand, so the residue says nothing about their order and the swap is
/// invisible to it. Such a site is UNINFORMATIVE: it is counted and named, and
/// it is never folded into the covered figure as if the claim had been made of
/// it. The floor only ever claims what the residue can actually distinguish.
fn residues(ps: &[(usize, usize)]) -> Vec<(usize, usize)> {
    ps.iter()
        .map(|(k, a)| (k % POOL_SLOTS, a % POOL_SLOTS))
        .filter(|(k, a)| k != a)
        .collect()
}

/// Every predicted residue pair the object does not carry as often as the
/// source demands it. A FLOOR per DISTINCT residue pair, for the same reason
/// [`pair_shortfalls`] is one: `यावत्` rotation emits a source comparison
/// twice, and two ranks eight apart land in the same bucket.
fn residue_shortfalls(want: &Pairs, have: &Pairs) -> Vec<String> {
    let mut out = Vec::new();
    for (r, ps) in want {
        let mut wanted: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for p in residues(ps) {
            *wanted.entry(p).or_default() += 1;
        }
        let carried = have.get(r).map(Vec::as_slice).unwrap_or(&[]);
        let carried = residues(carried);
        for (p, n) in wanted {
            let got = carried.iter().filter(|q| **q == p).count();
            if got < n {
                out.push(format!(
                    "{r}: the source wants {n} `न्यूनलङ्घनम्` whose करण slot is \
                     {} and whose अपादान slot is {} modulo eight; the object \
                     carries {got} (it carries {carried:?})",
                    p.0, p.1
                ));
            }
        }
    }
    out
}

/// The slot a name of rank `rank` holds when the anonymous pool was cut at
/// `base` — `rank` before the cut, `rank + ८` after it. The whole of the
/// numbering follows from the one number, because the cut happens ONCE
/// (`ir.t1:1346` guards on `अनामस्थलारम्भः समम् ०`) and is EXACTLY
/// `अनामस्थलसंख्या` wide.
fn slot_at(rank: usize, base: usize) -> usize {
    if rank < base { rank } else { rank + POOL_SLOTS }
}

/// **THE POOL'S POSITION, RECOVERED FROM THE OBJECT BY EXHAUSTION — AND WHY IT
/// CANNOT BE READ OFF THE FRONT END.**
///
/// **THE PREMISE THIS LEDGER CARRIED IN WAS WRONG, AND IT IS WRONG BY
/// CONSTRUCTION.** The plan was to read `अनामस्थलारम्भः` (`ir.t1:1343`) out of
/// `Front` per routine, since it holds `१ +` the pool's first slot. It cannot
/// be read per routine: `मध्यरूपॱकार्यक्रमरचना` lowers EVERY routine of the
/// module inside ONE interpreter call and zeroes that global at the top of each
/// one (`ir.t1:4763`, and again at `:4826` for the synthesised growth
/// routine), so by the time `build_ir` returns it holds the LAST routine's cut
/// and nothing else. Nor is there a side table to read instead —
/// `स्थानीयचिह्नककोश` is rewritten from index १ for every routine for exactly
/// the same reason, and `वृत्तिनामचिह्नककोश` (`ir.t1:4818`) is the only
/// per-routine record the builder keeps. A per-routine reading would need a new
/// arena in `ir.t1`, which is the compiler's own shape changed to serve a
/// census.
///
/// What the OBJECT can be asked instead is which cut positions are consistent
/// with it. A base `b` fixes the entire numbering through [`slot_at`], and
/// there are only `names + १` candidates, so every one is tried and the ones
/// the object carries are kept. Nothing is fitted: the candidates are generated
/// from the DECLARATION COUNT alone, and the object accepts or rejects each.
///
/// **WHAT IS CLAIMED — AND WHY IT IS NOT THE RESIDUE AGAIN.** That at least one
/// base explains EVERY site of the routine AT ONCE. The residue
/// (`slot ≡ rank (mod ८)`) lets each site fall on whichever side of the cut
/// suits it: ranks `(१, ९)` carried as `(१, १७)` and ranks `(९, १)` carried as
/// `(१७, १)` satisfy the residue twice over, and NO single base puts rank १ at
/// slot १ and rank १ at slot १७ in the same frame. So this claim refuses
/// objects the residue reads green.
///
/// **AND WHAT IS REFUSED.** When several bases fit, the position is AMBIGUOUS:
/// the existence claim stands, and the exact `(करण, अपादान)` pair is NOT
/// reported for it, because naming the base that agrees is fitting the answer
/// to the data. Only a routine with EXACTLY ONE consistent base has its
/// position recovered. A FLOOR per distinct pair, as everywhere in this file:
/// `यावत्` rotation emits one source comparison twice, so the object may carry
/// a pair more often than the source demands it and never less.
fn consistent_bases(want: &[(usize, usize)], have: &[(usize, usize)], names: usize) -> Vec<usize> {
    consistent_bases_of(&[(want, have)], &[], names)
}

/// **THE SAME EXHAUSTION, ASKED OF SEVERAL MNEMONICS AT ONCE.**
///
/// One group per branch word: the ranks that mnemonic's sites predict, and the
/// slot pairs the object carries UNDER THAT MNEMONIC. A base survives only if
/// EVERY group's multiset is contained in its own — which is why the grouping
/// matters and a flat list would be wrong in the permissive direction: a
/// `समम्` site satisfied by a `न्यूनलङ्घनम्` the object happens to carry is a
/// constraint silently dropped.
///
/// **EACH EXTRA SITE CAN ONLY SHRINK THE SET**, because a base must satisfy the
/// groups it already satisfied and one more. So folding the other three
/// `compare_op`s in cannot invent a recovery, only witness one — and a routine
/// whose set goes EMPTY is a RED and never a reason to drop the evidence: it
/// says the harvester ranks a declaration wrongly, or that this mnemonic does
/// not put its operands in the order claimed above.
fn consistent_bases_of(groups: &[Group<'_>], halves: &[HalfGroup<'_>], names: usize) -> Vec<usize> {
    let carried: Vec<BTreeMap<(usize, usize), usize>> = groups
        .iter()
        .map(|(_, have)| {
            let mut m: BTreeMap<(usize, usize), usize> = BTreeMap::new();
            for p in *have {
                *m.entry(*p).or_default() += 1;
            }
            m
        })
        .collect();
    let carried_halves: Vec<BTreeMap<Half, usize>> = halves
        .iter()
        .map(|(_, have)| {
            let mut m: BTreeMap<Half, usize> = BTreeMap::new();
            for h in *have {
                *m.entry(*h).or_default() += 1;
            }
            m
        })
        .collect();
    (0..=names)
        .filter(|b| {
            groups.iter().zip(&carried).all(|((want, _), have)| {
                let mut wanted: BTreeMap<(usize, usize), usize> = BTreeMap::new();
                for (x, y) in *want {
                    *wanted
                        .entry((slot_at(*x, *b), slot_at(*y, *b)))
                        .or_default() += 1;
                }
                wanted
                    .iter()
                    .all(|(p, n)| have.get(p).copied().unwrap_or(0) >= *n)
            }) && halves.iter().zip(&carried_halves).all(|((want, _), have)| {
                // ONLY THE SLOT MOVES WITH THE CUT. The constant is the
                // object's own and the side is the written side, so a base
                // that shifted either would be matching a branch against
                // one the source never wrote.
                let mut wanted: BTreeMap<Half, usize> = BTreeMap::new();
                for (rank, v, karana) in *want {
                    *wanted.entry((slot_at(*rank, *b), *v, *karana)).or_default() += 1;
                }
                wanted
                    .iter()
                    .all(|(h, n)| have.get(h).copied().unwrap_or(0) >= *n)
            })
        })
        .collect()
}

/// ONE GROUP for [`consistent_bases_of`] — the ranks a mnemonic's sites
/// predict, and the slot pairs the object carries UNDER THAT MNEMONIC.
type Group<'a> = (&'a [(usize, usize)], &'a [(usize, usize)]);

/// ONE HALF-GROUP for [`consistent_bases_of`] — the `(rank, constant, side)`
/// triples a mnemonic's name-against-numeral sites predict, and the
/// `(slot, constant, side)` triples the object carries under THAT mnemonic.
type HalfGroup<'a> = (&'a [Half], &'a [Half]);

/// `branch word -> the (करण, अपादान) rank pairs its sites predict`, for ONE
/// routine. Keyed by word because a want may only ever be matched against the
/// pairs the object carries under the same one.
type ByWord = BTreeMap<&'static str, Vec<(usize, usize)>>;

/// The same, for half-sites. See [`Half`].
type ByWordHalf = BTreeMap<&'static str, Vec<Half>>;

/// The three branch words [`extra_pairs`] harvests, in the order `branch_of`
/// names them.
const EXTRA_WORDS: [&str; 3] = ["समलङ्घनम्", "विषमलङ्घनम्", "अन्यूनलङ्घनम्"];

/// One routine's groups for [`consistent_bases_of`] — `न्यूनलङ्घनम्` first, then
/// one per OTHER branch word its source writes, each want matched only against
/// the pairs the object carries under the SAME word.
///
/// `lt_have` and `extra_have` are the object being asked, so the honest and the
/// bitten reading share this builder and differ only in what they are handed.
fn cut_groups<'a>(
    routine: &str,
    lt_want: &'a [(usize, usize)],
    lt_have: &'a Pairs,
    extra_want: Option<&'a ByWord>,
    extra_have: &'a BTreeMap<&'static str, Pairs>,
) -> Vec<Group<'a>> {
    const NONE: &[(usize, usize)] = &[];
    let mut g = vec![(lt_want, lt_have.get(routine).map_or(NONE, Vec::as_slice))];
    for (word, want) in extra_want.into_iter().flatten() {
        g.push((
            want.as_slice(),
            extra_have
                .get(word)
                .and_then(|p| p.get(routine))
                .map_or(NONE, Vec::as_slice),
        ));
    }
    g
}

/// One routine's HALF-groups for [`consistent_bases_of`] — one per branch word
/// whose source writes a `<name> <op> <numeral>` head, each want matched only
/// against the half-sites the object carries under the SAME word.
///
/// Built beside [`cut_groups`] and never merged with it: a half-site satisfied
/// by a two-slot pair would be a constraint silently dropped, exactly as a
/// `समम्` satisfied by a `न्यूनलङ्घनम्` would be.
fn cut_halves<'a>(
    routine: &str,
    half_want: Option<&'a ByWordHalf>,
    half_have: &'a BTreeMap<&'static str, HalfPairs>,
) -> Vec<HalfGroup<'a>> {
    const NONE: &[Half] = &[];
    let mut g = Vec::new();
    for (word, want) in half_want.into_iter().flatten() {
        g.push((
            want.as_slice(),
            half_have
                .get(word)
                .and_then(|p| p.get(routine))
                .map_or(NONE, Vec::as_slice),
        ));
    }
    g
}

/// The half-sites one routine's object carries, by branch word — the HAVE side
/// of a refusal message, so a red that turns on a constant says which constant.
fn half_of(
    half_have: &BTreeMap<&'static str, HalfPairs>,
    routine: &str,
) -> Vec<(&'static str, Vec<Half>)> {
    EXTRA_WORDS
        .into_iter()
        .filter_map(|w| {
            half_have
                .get(w)
                .and_then(|q| q.get(routine))
                .map(|v| (w, v.clone()))
        })
        .collect()
}

/// Every routine whose object carries fewer slot-against-slot branches in a
/// direction than its source demands.
fn shortfalls(
    want: &BTreeMap<String, Directions>,
    have: &BTreeMap<String, Directions>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (r, w) in want {
        let h = have.get(r).copied().unwrap_or_default();
        if h.ascending < w.ascending || h.descending < w.descending {
            out.push(format!(
                "{r}: the source wants {} ascending and {} descending \
                 slot-against-slot `न्यूनलङ्घनम्`; the object carries {} and {}",
                w.ascending, w.descending, h.ascending, h.descending
            ));
        }
    }
    out
}

/// **THE `अधिकम्` EXCHANGE WHERE BOTH OPERANDS ARE NAMES, ASKED OF THE CORPUS.**
///
/// The numeral sweep above reads 210 sites and is blind to every comparison of
/// two variables, because both come out `Origin::Slot` and the slot NUMBER is
/// an allocation artefact. The frame ties it back: parameters and `चरः` locals
/// take slots in first-declaration order (`ir.t1:1320`), so `a न्यूनम् b` must
/// put the EARLIER-declared name in the करण when `a` is declared first, and
/// `a अधिकम् b` must put it there when `b` is. Dropping the exchange swaps
/// every one of them.
///
/// **WHAT IS CLAIMED AND WHAT IS NOT.** A FLOOR per routine, in each direction,
/// for `exchange_of`'s reason: `यावत्` rotation emits one source comparison
/// twice, and a routine's own `न्यूनम्` sites land in the same census. So a
/// routine whose predicted directions happen to balance can absorb the
/// mutation — which is why the per-module sensitivity control below is what
/// licenses the green, and the routines that are NOT individually sensitive are
/// counted rather than assumed away.
///
/// **AND THE SITES OUT OF REACH ARE NAMED.** A global read has no slot at all;
/// so does an import. Those sites are listed, never matched against slot ०.
#[test]
fn every_adhikam_between_two_names_puts_the_second_name_in_the_karana() {
    let mut modules = 0usize;
    let mut in_reach = 0usize;
    let mut routines_predicted = 0usize;
    let mut routines_sensitive = 0usize;
    let mut numeral = 0usize;
    let mut out_of_reach: Vec<String> = Vec::new();
    let mut spilled: Vec<String> = Vec::new();
    let mut short: Vec<String> = Vec::new();
    let mut silent: Vec<String> = Vec::new();
    let mut blind: Vec<String> = Vec::new();
    let mut paired_routines = 0usize;
    let mut paired_sites = 0usize;
    let mut unpaired: Vec<String> = Vec::new();
    let mut pair_short: Vec<String> = Vec::new();
    let mut pair_sensitive = 0usize;
    let mut only_pair: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut residue_routines = 0usize;
    let mut residue_sites = 0usize;
    let mut residue_uninformative: Vec<String> = Vec::new();
    let mut residue_short: Vec<String> = Vec::new();
    let mut residue_sensitive = 0usize;
    let mut only_residue: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut cut_recovered = 0usize;
    let mut cut_sites = 0usize;
    let mut cut_ambiguous: Vec<String> = Vec::new();
    let mut cut_none: Vec<String> = Vec::new();
    let mut cut_sensitive = 0usize;
    let mut only_cut: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut cut_extra_sites = 0usize;
    let mut cut_narrowed = 0usize;
    let mut cut_by_extra = 0usize;
    let mut extra_refused: Vec<String> = Vec::new();
    let mut extra_numeral = 0usize;
    let mut extra_unslotted = 0usize;
    let mut extra_one_name = 0usize;
    let mut extra_seen = 0usize;
    let mut extra_claimed = 0usize;
    let mut half_claimed = 0usize;
    let mut half_unslotted = 0usize;
    let mut half_both = 0usize;
    let mut half_refused: Vec<String> = Vec::new();
    let mut half_out_of_range: Vec<String> = Vec::new();
    let mut half_folded = 0usize;
    let mut half_widened = 0usize;
    let mut half_exact_paired = 0usize;
    let mut half_exact_routines = 0usize;
    let mut half_pair_short: Vec<String> = Vec::new();
    let mut half_exact_bit = 0usize;
    let mut half_exact_blind: Vec<String> = Vec::new();
    let mut half_unused_unvisited = 0usize;
    let mut half_unused_routine_unseen = 0usize;
    let mut cut_narrowed_by_half = 0usize;
    let mut cut_by_half = 0usize;
    let mut wide_narrowed_by_half = 0usize;
    let mut wide_by_half = 0usize;
    let mut extra_unused_paired = 0usize;
    let mut extra_unused_unvisited = 0usize;
    let mut wide_routines = 0usize;
    let mut wide_extra_sites = 0usize;
    let mut wide_recovered = 0usize;
    let mut wide_sites = 0usize;
    let mut wide_ambiguous: Vec<String> = Vec::new();
    let mut wide_none: Vec<String> = Vec::new();
    let mut wide_no_frame: Vec<String> = Vec::new();
    let mut wide_bit_total: Vec<String> = Vec::new();
    let mut not_monotone: Vec<String> = Vec::new();
    let mut reached_further: Vec<String> = Vec::new();
    let mut half_wide_routines = 0usize;
    let mut half_only_widened = 0usize;
    let mut half_wide_recovered = 0usize;
    let mut half_wide_sites = 0usize;
    let mut half_wide_ambiguous: Vec<String> = Vec::new();
    let mut half_wide_none: Vec<String> = Vec::new();
    let mut half_no_frame: Vec<String> = Vec::new();
    let mut half_wide_bit: Vec<String> = Vec::new();

    for p in corpus_paths() {
        let file = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let src = std::fs::read_to_string(&p).expect("readable");
        let Some(module) = module_name(&src) else {
            continue;
        };
        let routines = source_routines(&src);
        if routines.iter().all(|r| r.sites.is_empty()) {
            continue;
        }
        let mut unreached: Vec<String> = Vec::new();
        let want_pairs = predicted_pairs(&module, &routines, &mut unreached, &mut numeral);
        let want = directions_of(&want_pairs);
        out_of_reach.extend(unreached.iter().map(|s| format!("{file}: {s}")));
        if want.is_empty() {
            continue;
        }
        // A SOURCE THAT COMPARES TWO NAMES AND EMITS NOTHING IS A REFUSAL —
        // AND IT CARRIES THE REASON. A list of file names would say that the
        // corpus stopped compiling and nothing about what stopped it.
        let (text, windows) = match compile_framed(&src) {
            Framed::Emitted(text, windows, _frames) => (text, windows),
            Framed::DeclaresNoModule => {
                silent.push(format!(
                    "{file}: `module_name` read a module above and                      `compile_framed` reads none — the two loaders disagree"
                ));
                continue;
            }
            Framed::Refused(why) => {
                silent.push(format!(
                    "{file}: compares two names in a condition and emits no                      object — {why}"
                ));
                continue;
            }
        };
        modules += 1;
        let sites: usize = want.values().map(|d| d.ascending + d.descending).sum();
        in_reach += sites;
        routines_predicted += want.len();
        let have_pairs = emitted_pairs(&text, &windows, &mut spilled);
        let have = directions_of(&have_pairs);
        short.extend(
            shortfalls(&want, &have)
                .iter()
                .map(|s| format!("{file}: {s}")),
        );

        // **THE PER-SITE CLAIM, FOR THE ROUTINES WHOSE NUMBERING IS RECOVERED.**
        // See [`pairable`]: a routine with no anonymous slot has slot `k` at
        // declaration rank `k` as a consequence, so the exact `(करण, अपादान)`
        // pair is predictable and a routine whose directions happen to balance
        // can no longer absorb the mutation. The rest are NAMED, not assumed.
        let want_paired = pairable(&want_pairs, &routines, &module, &windows);
        paired_routines += want_paired.len();
        paired_sites += want_paired.values().map(Vec::len).sum::<usize>();
        for (r, ps) in &want_pairs {
            if !want_paired.contains_key(r) {
                let declared = routines
                    .iter()
                    .find(|q| format!("{module}{}", q.name) == *r)
                    .map_or(0, |q| q.slots.len());
                let locals = windows
                    .get(r)
                    .map_or(0, |(lo, hi)| ((hi - lo) / 8) as usize);
                unpaired.push(format!(
                    "{file}: {r}: UNPAIRED — {} declared name(s) and {locals} \
                     emitted local slot(s), so the anonymous pool's position is \
                     not recovered; its {} site(s) fall back to the order claim",
                    declared,
                    ps.len()
                ));
            }
        }
        pair_short.extend(
            pair_shortfalls(&want_paired, &have_pairs)
                .iter()
                .map(|s| format!("{file}: {s}")),
        );

        // **AND THE RESIDUE, FOR EXACTLY THE ROUTINES `pairable` REFUSED.**
        // See [`residues`]: the pool is cut once and is eight slots wide, so a
        // name's slot is its rank or its rank plus eight and therefore
        // `slot ≡ rank (mod ८)` WHEREVER the cut fell. That is a claim about
        // the slot NUMBERS, not their order, and it needs no recovery of the
        // position at all. The sites it cannot distinguish — two ranks
        // congruent mod eight — are named, never counted as covered.
        let want_residue: Pairs = want_pairs
            .iter()
            .filter(|(r, _)| !want_paired.contains_key(*r))
            .map(|(r, ps)| (r.clone(), ps.clone()))
            .collect();
        for (r, ps) in &want_residue {
            for (x, y) in ps {
                if x % POOL_SLOTS == y % POOL_SLOTS {
                    residue_uninformative.push(format!(
                        "{file}: {r}: ranks {x} and {y} are congruent modulo \
                         eight, so the residue carries no order and this site \
                         keeps only the direction claim"
                    ));
                }
            }
        }
        residue_routines += want_residue
            .values()
            .filter(|ps| !residues(ps).is_empty())
            .count();
        residue_sites += want_residue
            .values()
            .map(|ps| residues(ps).len())
            .sum::<usize>();
        residue_short.extend(
            residue_shortfalls(&want_residue, &have_pairs)
                .iter()
                .map(|s| format!("{file}: {s}")),
        );

        // **AND THE POOL'S POSITION ITSELF, RECOVERED BY EXHAUSTION.**
        // See [`consistent_bases`] for why it cannot be read off `Front`: the
        // builder zeroes `अनामस्थलारम्भः` at every routine inside ONE call, so
        // after `build_ir` it holds the last routine's cut only. The object is
        // asked instead — which of the `names + १` cut positions explain ALL of
        // this routine's sites at once. NONE is a red; SEVERAL is ambiguity,
        // reported and not resolved by choosing the agreeable one.
        // **AND THE OTHER THREE `compare_op`s ARE FOLDED IN HERE.** See
        // [`extra_pairs`]: `समम्`, `असमम्` and `बृहत्समम्` each pair two slots of
        // ONE frame, and `अनामस्थानम्` runs once per ROUTINE and not once per
        // mnemonic, so their sites constrain the SAME base. Each can only
        // shrink the candidate set, which is what turns AMBIGUOUS into
        // RECOVERED without fitting anything.
        let extra = extra_pairs(&module, &routines);
        extra_refused.extend(extra.refused.iter().map(|s| format!("{file}: {s}")));
        extra_numeral += extra.numeral;
        extra_claimed += extra.claimed;
        extra_unslotted += extra.unslotted;
        extra_one_name += extra.one_name;
        extra_seen += extra.seen;
        half_claimed += extra.half_claimed;
        half_unslotted += extra.half_unslotted;
        half_both += extra.half_both;
        half_refused.extend(extra.half_refused.iter().map(|s| format!("{file}: {s}")));
        half_out_of_range.extend(
            extra
                .half_out_of_range
                .iter()
                .map(|s| format!("{file}: {s}")),
        );
        let extra_have: BTreeMap<&'static str, Pairs> = EXTRA_WORDS
            .into_iter()
            .map(|m| (m, emitted_pairs_of(&text, &windows, m, &mut spilled)))
            .collect();
        // **AND THE HALF-SITES THE OBJECT CARRIES**, under the same three
        // words, read by [`emitted_halves_of`] and matched word for word.
        let half_have: BTreeMap<&'static str, HalfPairs> = EXTRA_WORDS
            .into_iter()
            .map(|m| (m, emitted_halves_of(&text, &windows, m, &mut spilled)))
            .collect();
        let names_of = |r: &str| {
            routines
                .iter()
                .find(|q| format!("{module}{}", q.name) == *r)
                .map_or(0, |q| q.slots.len())
        };
        let cut_bases: BTreeMap<String, Vec<usize>> = want_residue
            .iter()
            .map(|(r, ps)| {
                let names = names_of(r);
                let lt_only = consistent_bases(
                    ps,
                    have_pairs.get(r).map(Vec::as_slice).unwrap_or(&[]),
                    names,
                );
                let paired_only = consistent_bases_of(
                    &cut_groups(r, ps, &have_pairs, extra.want.get(r), &extra_have),
                    &[],
                    names,
                );
                let folded = consistent_bases_of(
                    &cut_groups(r, ps, &have_pairs, extra.want.get(r), &extra_have),
                    &cut_halves(r, extra.half_want.get(r), &half_have),
                    names,
                );
                // **AND WHAT THE HALF-SITES ALONE ADDED.** Counted against the
                // two-name fold and not against `न्यूनलङ्घनम्` alone, or a
                // narrowing the other three mnemonics' PAIRS had already made
                // would be credited to the constants.
                if folded.len() < paired_only.len() {
                    cut_narrowed_by_half += 1;
                    if folded.len() == 1 && paired_only.len() > 1 {
                        cut_by_half += 1;
                    }
                }
                // **THE LAW THE FOLD MUST OBEY.** More constraints, never
                // fewer: a base the folded reading keeps is one the
                // `न्यूनलङ्घनम्` reading kept. A miss here is the grouping wired
                // wrongly — a mnemonic's sites matched against another
                // mnemonic's pairs — and would show up as a recovery invented
                // rather than witnessed.
                if !folded.iter().all(|b| lt_only.contains(b)) {
                    not_monotone.push(format!(
                        "{file}: {r}: folding the other `compare_op`s ADDED a \
                         candidate cut — `न्यूनलङ्घनम्` alone left {lt_only:?} \
                         and the fold left {folded:?}"
                    ));
                }
                if folded.len() < lt_only.len() {
                    cut_narrowed += 1;
                    if folded.len() == 1 && lt_only.len() > 1 {
                        cut_by_extra += 1;
                    }
                }
                if extra.want.contains_key(r) {
                    cut_extra_sites += extra.want[r].values().map(Vec::len).sum::<usize>();
                }
                if let Some(h) = extra.half_want.get(r) {
                    half_folded += h.values().map(Vec::len).sum::<usize>();
                }
                (r.clone(), folded)
            })
            .collect();
        // **AND THE WIDENING: THE CUT OF A ROUTINE THAT WRITES NO
        // `न्यूनम्`/`अधिकम्` HEAD AT ALL.** The recovery above runs over
        // `want_residue`, which is derived from `sites` — so a routine whose
        // only two-name comparisons are `समम्`, `असमम्` or `बृहत्समम्`
        // is outside every claim this file made before this cycle, however
        // many of them it writes. `अनामस्थानम्` cuts its pool once all the
        // same (`ir.t1:1346`), the cut is the SAME cut for every mnemonic, and
        // those three operators pin it by EXACTLY the exhaustion above — with
        // the `न्यूनलङ्घनम्` group present and EMPTY, so the honest and the
        // widened readings run through one builder and a routine that later
        // gains such a site is carried by the same path rather than a second.
        //
        // **THE CASE THAT MUST STILL BE REFUSED.** These routines are NOT
        // sensitive to the dropped exchange: `drop_the_exchange` rewrites
        // `न्यूनलङ्घनम्` lines and nothing else, and their evidence is the
        // other three words, so their bases must come out IDENTICAL under the
        // bitten object. They are counted APART from `cut_sensitive` and
        // asserted ZERO below — folding them in would let the sensitivity
        // control report routines the mutation cannot bite, and a bite here
        // would say the widened recovery reads `न्यूनलङ्घनम्` evidence it
        // claims not to use.
        //
        // A routine with NO FRAME WINDOW is named and left in the unused
        // bucket: `emitted_pairs_of` can carry no pair for it, so every
        // candidate base would survive vacuously and the ambiguity reported
        // would be the census's own blindness and not the object's.
        const NO_LT_SITES: &[(usize, usize)] = &[];
        let mut wide_bases: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (r, by_word) in &extra.want {
            let n: usize = by_word.values().map(Vec::len).sum();
            if want_residue.contains_key(r) {
                continue;
            }
            if want_paired.contains_key(r) {
                extra_unused_paired += n;
                continue;
            }
            if !windows.contains_key(r) {
                wide_no_frame.push(format!(
                    "{file}: {r}: compares two names under \
                     `समम्`/`असमम्`/`बृहत्समम्` at {n} site(s) and the emitter \
                     lays out no frame for it, so the object carries no slot \
                     pair to refuse a candidate cut with"
                ));
                extra_unused_unvisited += n;
                continue;
            }
            wide_extra_sites += n;
            let pairs_only = consistent_bases_of(
                &cut_groups(r, NO_LT_SITES, &have_pairs, Some(by_word), &extra_have),
                &[],
                names_of(r),
            );
            let with_halves = consistent_bases_of(
                &cut_groups(r, NO_LT_SITES, &have_pairs, Some(by_word), &extra_have),
                &cut_halves(r, extra.half_want.get(r), &half_have),
                names_of(r),
            );
            if with_halves.len() < pairs_only.len() {
                wide_narrowed_by_half += 1;
                if with_halves.len() == 1 && pairs_only.len() > 1 {
                    wide_by_half += 1;
                }
            }
            wide_bases.insert(r.clone(), with_halves);
        }
        // **AND THE THIRD WIDENING: THE CUT OF A ROUTINE WHOSE EVERY
        // TWO-OPERAND COMPARISON IS A NAME AGAINST A CONSTANT.** Both
        // recoveries above are entered from a two-NAME head — the first from
        // `sites`, the second from `extra.want` — so a routine that writes
        // neither was visited by nothing in this file at all, however many
        // half-sites it carries: 184 of them last cycle, the largest bucket
        // left. The SAME exhaustion runs with BOTH pair groups empty, because
        // `अनामस्थानम्` cuts the pool once per ROUTINE (`ir.t1:1346`) and a
        // half-site is a claim about the SAME base.
        //
        // **THE CASE THAT MUST STILL BE REFUSED, AND HERE IT IS THE COMMON
        // ONE.** A half-site names ONE slot, so it fixes only which SIDE of the
        // cut its rank falls on: `slot_at(rank, b)` takes just two values over
        // the `names + १` candidates, and one of them is shared by a whole run
        // of bases. So a routine carrying a single half-site can never come out
        // with fewer than two, and MOST of these routines are AMBIGUOUS. They
        // are NAMED and never counted as recovered.
        //
        // **AND THE SECOND, WHICH THIS LEDGER GUESSED THE WRONG WAY ROUND.** A
        // routine with no frame window needs its own bucket — but not because
        // every candidate survives vacuously. `emitted_halves_of` carries NO
        // entry for such a routine (it reads `windows` and skips), so the HAVE
        // side is EMPTY, no base satisfies a non-empty want, and the routine
        // would be reported UNEXPLAINED: the census's own blindness read as the
        // object's, in the RED direction rather than the permissive one. Named
        // here, never folded.
        let mut half_bases: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (r, by_word) in &extra.half_want {
            let n: usize = by_word.values().map(Vec::len).sum();
            if want_residue.contains_key(r)
                || want_paired.contains_key(r)
                || wide_bases.contains_key(r)
            {
                continue;
            }
            if !windows.contains_key(r) {
                half_no_frame.push(format!(
                    "{file}: {r}: compares a name against a constant at {n} \
                     site(s) and the emitter lays out no frame for it, so the \
                     object carries no half-site at all and every candidate cut \
                     dies on a constraint this census cannot read"
                ));
                continue;
            }
            half_bases.insert(
                r.clone(),
                consistent_bases_of(
                    &cut_groups(r, NO_LT_SITES, &have_pairs, None, &extra_have),
                    &cut_halves(r, Some(by_word), &half_have),
                    names_of(r),
                ),
            );
        }
        // **AND WHERE EVERY HALF-SITE WENT.** The same four buckets the pairs
        // are accounted into, plus the two the pairs cannot have: a routine
        // whose ONLY two-operand comparison is a name against a CONSTANT, now
        // visited by the third widening above, and one that even that cannot
        // reach. The last bucket is KEPT and asserted ZERO: a half-site that
        // entered no recovery and no refusal is one this sweep harvested and
        // then lost, and a bucket deleted the cycle it emptied could not say so.
        for (r, by_word) in &extra.half_want {
            let n: usize = by_word.values().map(Vec::len).sum();
            if want_residue.contains_key(r) {
                continue;
            }
            if want_paired.contains_key(r) {
                // **AND THIS BUCKET IS NO LONGER "UNUSED".** A routine
                // `pairable` accepts has no anonymous pool, so rank IS slot and
                // its half-sites predict an exact `(slot, constant, side)` the
                // object must carry — see [`half_shortfalls`], which is asked
                // of exactly these and of nothing else.
                half_exact_paired += n;
                half_exact_routines += 1;
            } else if !windows.contains_key(r) {
                half_unused_unvisited += n;
            } else if wide_bases.contains_key(r) {
                half_widened += n;
            } else if half_bases.contains_key(r) {
                half_only_widened += n;
            } else {
                half_unused_routine_unseen += n;
            }
        }
        // **AND THE EXACT-SLOT CLAIM, OVER EXACTLY THE ROUTINES THAT BUCKET
        // COUNTS.** Every other reader of a half-site in this file narrows a
        // candidate cut; a `pairable` routine has no cut to narrow, so its
        // half-sites are the one use that can come out RED. See
        // [`half_shortfalls`] for the three refusals it keeps.
        let file_half_short = half_shortfalls(&want_paired, &extra.half_want, &half_have);
        let file_half_exact: usize = extra
            .half_want
            .iter()
            .filter(|(r, _)| want_paired.contains_key(*r))
            .map(|(_, w)| w.values().map(Vec::len).sum::<usize>())
            .sum();
        half_pair_short.extend(file_half_short.iter().map(|s| format!("{file}: {s}")));
        // **AND ITS OWN FALSIFIER, BECAUSE NOTHING ELSE IN THIS FILE CAN MOVE
        // IT.** `drop_the_exchange` rewrites `न्यूनलङ्घनम्` only, and
        // `reached_further` asserts the half-sites come out identical under it
        // — so a green from this claim is, so far, indistinguishable from a
        // green from a reader that read nothing. [`swap_the_half_sides`]
        // exchanges the operands of the three words the half-sites DO live
        // under: the slot and the constant are unchanged, every other claim in
        // this file reads the swapped object exactly as it reads the honest
        // one, and this one must go short.
        let swapped_have: BTreeMap<&'static str, HalfPairs> = EXTRA_WORDS
            .into_iter()
            .map(|m| {
                (
                    m,
                    emitted_halves_of(&swap_the_half_sides(&text), &windows, m, &mut Vec::new()),
                )
            })
            .collect();
        let swapped_short = half_shortfalls(&want_paired, &extra.half_want, &swapped_have);
        half_exact_bit += swapped_short.len();
        if file_half_exact > 0 && swapped_short.is_empty() {
            half_exact_blind.push(format!(
                "{file}: {file_half_exact} exact-slot half-site(s) in \
                 {} routine(s) `pairable` accepts, and exchanging the operands \
                 of EVERY `समम्`/`असमम्`/`अन्यूनलङ्घनम्` branch in the object \
                 still reads them green — the side is not being read",
                extra
                    .half_want
                    .keys()
                    .filter(|r| want_paired.contains_key(*r))
                    .count()
            ));
        }
        wide_routines += wide_bases.len();
        for (r, bases) in &wide_bases {
            let n: usize = extra.want[r].values().map(Vec::len).sum();
            let carried: BTreeMap<&'static str, usize> = EXTRA_WORDS
                .into_iter()
                .map(|w| {
                    (
                        w,
                        extra_have.get(w).and_then(|p| p.get(r)).map_or(0, Vec::len),
                    )
                })
                .collect();
            match bases.len() {
                0 => wide_none.push(format!(
                    "{file}: {r}: NO cut position explains its {n} \
                     `समम्`/`असमम्`/`बृहत्समम्` site(s) at once — {} declared \
                     name(s), predicted ranks {:?}, the object carries \
                     {carried:?}; predicted half-sites {:?}, the object carries \
                     {:?}",
                    names_of(r),
                    extra.want[r],
                    extra.half_want.get(r),
                    half_of(&half_have, r)
                )),
                1 => {
                    wide_recovered += 1;
                    wide_sites += n;
                }
                k => wide_ambiguous.push(format!(
                    "{file}: {r}: {k} of the {} candidate cut position(s) fit \
                     its {n} site(s) ({bases:?}); the position is AMBIGUOUS \
                     and the exact pair is not reported for it",
                    names_of(r) + 1
                )),
            }
        }
        half_wide_routines += half_bases.len();
        for (r, bases) in &half_bases {
            let n: usize = extra.half_want[r].values().map(Vec::len).sum();
            match bases.len() {
                0 => half_wide_none.push(format!(
                    "{file}: {r}: NO cut position explains its {n} \
                     name-against-constant site(s) at once — {} declared \
                     name(s), predicted half-sites {:?}, the object carries {:?}",
                    names_of(r),
                    extra.half_want.get(r),
                    half_of(&half_have, r)
                )),
                1 => {
                    half_wide_recovered += 1;
                    half_wide_sites += n;
                }
                k => half_wide_ambiguous.push(format!(
                    "{file}: {r}: {k} of the {} candidate cut position(s) fit \
                     its {n} name-against-constant site(s) ({bases:?}); a \
                     half-site names ONE slot and so fixes only which side of \
                     the cut its rank falls on, so the position is AMBIGUOUS \
                     and the exact slot is not reported for it",
                    names_of(r) + 1
                )),
            }
        }
        for (r, bases) in &cut_bases {
            let ps = &want_residue[r];
            let carried = have_pairs.get(r).map(Vec::as_slice).unwrap_or(&[]);
            let names = routines
                .iter()
                .find(|q| format!("{module}{}", q.name) == *r)
                .map_or(0, |q| q.slots.len());
            match bases.len() {
                // **AND THE MESSAGE CARRIES THE FOLDED EVIDENCE, because the
                // recovery is no longer `न्यूनलङ्घनम्`'s alone.** The first red
                // this cycle produced read `predicted ranks [(२, ३)], the
                // object carries [(२, ३)]` — two sets that AGREE, reported as
                // unexplained, because the constraint that actually failed was
                // a half-site the message did not print. An instrument with
                // two states where the truth has three hides its own
                // breakage.
                0 => cut_none.push(format!(
                    "{file}: {r}: NO cut position explains its {} site(s) at \
                     once — {names} declared name(s), predicted ranks {ps:?}, \
                     the object carries {carried:?}; predicted other-mnemonic \
                     ranks {:?}, the object carries {:?}; predicted half-sites \
                     {:?}, the object carries {:?}",
                    ps.len(),
                    extra.want.get(r),
                    EXTRA_WORDS
                        .into_iter()
                        .filter_map(|w| extra_have
                            .get(w)
                            .and_then(|q| q.get(r))
                            .map(|v| (w, v.clone())))
                        .collect::<Vec<_>>(),
                    extra.half_want.get(r),
                    half_of(&half_have, r)
                )),
                1 => {
                    cut_recovered += 1;
                    cut_sites += ps.len();
                }
                n => cut_ambiguous.push(format!(
                    "{file}: {r}: {n} of the {} candidate cut position(s) fit \
                     its {} site(s) ({bases:?}); the position is AMBIGUOUS and \
                     the exact pair is not reported for it",
                    names + 1,
                    ps.len()
                )),
            }
        }

        // **AND THIS MODULE'S OWN SENSITIVITY, MEASURED ON THIS MODULE.**
        // `drop_the_exchange` is the front end that lowered `a अधिकम् b` as
        // `Lt(a, b)`: same mnemonic, same line count, same labels. Every
        // prediction above must invert, and this sweep must name it.
        let bitten_text = drop_the_exchange(&text);
        let bitten_pairs = emitted_pairs(&bitten_text, &windows, &mut Vec::new());
        let bit = shortfalls(&want, &directions_of(&bitten_pairs));
        let pair_bit = pair_shortfalls(&want_paired, &bitten_pairs);
        let residue_bit = residue_shortfalls(&want_residue, &bitten_pairs);
        // AND THE CUT CLAIM'S OWN BITE — a routine the object explains with
        // SOME cut position and the bitten object explains with NONE. A routine
        // that had no consistent base honestly is excluded: it is a red above,
        // and counting it here would read its breakage as sensitivity.
        // AND THE OTHER THREE MNEMONICS ARE ASKED OF THE BITTEN OBJECT TOO,
        // rather than carried over: `drop_the_exchange` rewrites only
        // `न्यूनलङ्घनम्` lines, so these must come out IDENTICAL, and an
        // instrument that assumed that could not notice a mutation that reached
        // further than its margin says.
        let extra_bitten: BTreeMap<&'static str, Pairs> = EXTRA_WORDS
            .into_iter()
            .map(|m| {
                (
                    m,
                    emitted_pairs_of(&bitten_text, &windows, m, &mut Vec::new()),
                )
            })
            .collect();
        let half_bitten: BTreeMap<&'static str, HalfPairs> = EXTRA_WORDS
            .into_iter()
            .map(|m| {
                (
                    m,
                    emitted_halves_of(&bitten_text, &windows, m, &mut Vec::new()),
                )
            })
            .collect();
        if extra_bitten != extra_have {
            reached_further.push(format!(
                "{file}: `drop_the_exchange` changed a branch that is not \
                 `न्यूनलङ्घनम्` — the other three mnemonics' slot pairs differ \
                 between the honest object and the bitten one"
            ));
        }
        if half_bitten != half_have {
            reached_further.push(format!(
                "{file}: `drop_the_exchange` changed a NAME-AGAINST-CONSTANT \
                 branch that is not `न्यूनलङ्घनम्` — the other three mnemonics' \
                 half-sites differ between the honest object and the bitten one"
            ));
        }
        let cut_bit: Vec<&str> = cut_bases
            .iter()
            .filter(|(r, bases)| {
                !bases.is_empty()
                    && consistent_bases_of(
                        &cut_groups(
                            r,
                            &want_residue[*r],
                            &bitten_pairs,
                            extra.want.get(*r),
                            &extra_bitten,
                        ),
                        &cut_halves(r, extra.half_want.get(*r), &half_bitten),
                        names_of(r),
                    )
                    .is_empty()
            })
            .map(|(r, _)| r.as_str())
            .collect();
        // **AND THE WIDENED RECOVERY'S OWN BITE, WHICH MUST BE NONE.** Its
        // evidence is the three words `drop_the_exchange` does not touch, so
        // every widened base must survive the bitten object unchanged. This is
        // counted APART and never added to `cut_sensitive`: that figure is
        // what licenses the order census's green, and a routine the mutation
        // cannot reach must not be able to raise it.
        for (r, bases) in &wide_bases {
            if bases.is_empty() {
                continue;
            }
            let bitten = consistent_bases_of(
                &cut_groups(
                    r,
                    NO_LT_SITES,
                    &bitten_pairs,
                    extra.want.get(r),
                    &extra_bitten,
                ),
                &cut_halves(r, extra.half_want.get(r), &half_bitten),
                names_of(r),
            );
            if &bitten != bases {
                wide_bit_total.push(format!(
                    "{file}: {r}: the dropped exchange moved a cut recovered \
                     from `समम्`/`असमम्`/`बृहत्समम्` alone — honest {bases:?}, \
                     bitten {bitten:?}"
                ));
            }
        }
        // **AND THE THIRD WIDENING'S BITE, WHICH MUST ALSO BE NONE.** Its
        // whole evidence is name-against-constant heads under the three words
        // `drop_the_exchange` does not touch, and `half_bitten` is ASKED of the
        // bitten object rather than carried over, so every base must survive it
        // unchanged. Counted apart from `cut_sensitive` for the same reason the
        // second widening is: that figure licenses the order census's green and
        // a routine the mutation cannot reach must not be able to raise it.
        for (r, bases) in &half_bases {
            if bases.is_empty() {
                continue;
            }
            let bitten = consistent_bases_of(
                &cut_groups(r, NO_LT_SITES, &bitten_pairs, None, &extra_bitten),
                &cut_halves(r, extra.half_want.get(r), &half_bitten),
                names_of(r),
            );
            if &bitten != bases {
                half_wide_bit.push(format!(
                    "{file}: {r}: the dropped exchange moved a cut recovered \
                     from name-against-constant heads alone — honest {bases:?}, \
                     bitten {bitten:?}"
                ));
            }
        }
        routines_sensitive += bit.len();
        pair_sensitive += pair_bit.len();
        residue_sensitive += residue_bit.len();
        cut_sensitive += cut_bit.len();
        // **AND THE FIGURE THE PER-SITE CLAIM EXISTS FOR** — routines whose
        // predicted directions BALANCE, so the order census absorbs the
        // dropped exchange and reports them green, and whose exact slot pairs
        // do not. Zero here would mean the pair claim is decoration.
        let order_bit: std::collections::BTreeSet<&str> =
            bit.iter().filter_map(|s| s.split(':').next()).collect();
        for r in pair_bit.iter().filter_map(|s| s.split(':').next()) {
            if !order_bit.contains(r) {
                only_pair.insert(format!("{file}: {r}"));
            }
        }
        // AND THE SAME FIGURE FOR THE RESIDUE — routines the order census
        // reads green under the dropped exchange and the residue does not.
        // Zero here would mean the residue claim is the direction claim again.
        for r in residue_bit.iter().filter_map(|s| s.split(':').next()) {
            if !order_bit.contains(r) {
                only_residue.insert(format!("{file}: {r}"));
            }
        }
        // AND THE SAME FIGURE FOR THE CUT — routines the order census reads
        // green under the dropped exchange and no single cut position explains.
        for r in &cut_bit {
            if !order_bit.contains(r) {
                only_cut.insert(format!("{file}: {r}"));
            }
        }
        if bit.is_empty() && pair_bit.is_empty() && residue_bit.is_empty() && cut_bit.is_empty() {
            blind.push(format!(
                "{file}: the exchange dropped on EVERY `न्यूनलङ्घनम्` and this \
                 sweep still reads it green — {sites} site(s) reported as \
                 covered that are not"
            ));
        }
        println!(
            "  {file:22} two-name sites {sites:<4} routines {:<4} short {:<3} paired {:<4} pair-short {:<3} (dropped: {} order, {} pair)",
            want.len(),
            shortfalls(&want, &have).len(),
            want_paired.values().map(Vec::len).sum::<usize>(),
            pair_shortfalls(&want_paired, &have_pairs).len(),
            bit.len(),
            pair_bit.len(),
        );
        println!(
            "  {file:22} residue sites {:<4} routines {:<4} uninformative {:<3} (dropped: {} residue)",
            want_residue
                .values()
                .map(|ps| residues(ps).len())
                .sum::<usize>(),
            want_residue
                .values()
                .filter(|ps| !residues(ps).is_empty())
                .count(),
            want_residue
                .values()
                .flatten()
                .filter(|(x, y)| x % POOL_SLOTS == y % POOL_SLOTS)
                .count(),
            residue_bit.len(),
        );
        println!(
            "  {file:22} cut recovered {:<4} ambiguous {:<4} unexplained {:<3} (dropped: {} cut)",
            cut_bases.values().filter(|b| b.len() == 1).count(),
            cut_bases.values().filter(|b| b.len() > 1).count(),
            cut_bases.values().filter(|b| b.is_empty()).count(),
            cut_bit.len(),
        );
        println!(
            "  {file:22} wide  recovered {:<4} ambiguous {:<4} unexplained {:<3} over {} routine(s) the `न्यूनलङ्घनम्` recovery never visits",
            wide_bases.values().filter(|b| b.len() == 1).count(),
            wide_bases.values().filter(|b| b.len() > 1).count(),
            wide_bases.values().filter(|b| b.is_empty()).count(),
            wide_bases.len(),
        );
        println!(
            "  {file:22} half  recovered {:<4} ambiguous {:<4} unexplained {:<3} over {} routine(s) NO two-name recovery visits",
            half_bases.values().filter(|b| b.len() == 1).count(),
            half_bases.values().filter(|b| b.len() > 1).count(),
            half_bases.values().filter(|b| b.is_empty()).count(),
            half_bases.len(),
        );
        println!(
            "  {file:22} half  exact-slot sites {file_half_exact:<4} short {:<3} in the routine(s) `pairable` accepts (swapped: {} short)",
            file_half_short.len(),
            swapped_short.len(),
        );
    }

    println!("METRIC t1_corpus_two_name_modules {modules}");
    println!("METRIC t1_corpus_two_name_sites_in_reach {in_reach}");
    println!("METRIC t1_corpus_two_name_routines {routines_predicted}");
    println!("METRIC t1_corpus_two_name_routines_sensitive {routines_sensitive}");
    println!(
        "METRIC t1_corpus_two_name_sites_out_of_reach {}",
        out_of_reach.len()
    );
    println!("METRIC t1_corpus_two_name_sites_against_a_numeral {numeral}");
    println!(
        "METRIC t1_corpus_two_name_operands_in_a_spill_slot {}",
        spilled.len()
    );
    println!("METRIC t1_corpus_two_name_routines_slot_paired {paired_routines}");
    println!("METRIC t1_corpus_two_name_sites_slot_paired {paired_sites}");
    println!(
        "METRIC t1_corpus_two_name_routines_unpaired {}",
        unpaired.len()
    );
    println!("METRIC t1_corpus_two_name_pair_claims_sensitive {pair_sensitive}");
    println!(
        "METRIC t1_corpus_two_name_routines_only_the_pair_claim_catches {}",
        only_pair.len()
    );
    println!("METRIC t1_corpus_two_name_routines_slot_residue {residue_routines}");
    println!("METRIC t1_corpus_two_name_sites_slot_residue {residue_sites}");
    println!(
        "METRIC t1_corpus_two_name_sites_residue_uninformative {}",
        residue_uninformative.len()
    );
    println!("METRIC t1_corpus_two_name_residue_claims_sensitive {residue_sensitive}");
    println!(
        "METRIC t1_corpus_two_name_routines_only_the_residue_catches {}",
        only_residue.len()
    );
    println!("METRIC t1_corpus_two_name_routines_cut_recovered {cut_recovered}");
    println!("METRIC t1_corpus_two_name_sites_cut_recovered {cut_sites}");
    println!(
        "METRIC t1_corpus_two_name_routines_cut_ambiguous {}",
        cut_ambiguous.len()
    );
    println!(
        "METRIC t1_corpus_two_name_routines_cut_unexplained {}",
        cut_none.len()
    );
    println!("METRIC t1_corpus_two_name_cut_claims_sensitive {cut_sensitive}");
    println!("METRIC t1_corpus_cut_extra_sites_folded {cut_extra_sites}");
    println!(
        "METRIC t1_corpus_cut_extra_sites_refused_by_type {}",
        extra_refused.len()
    );
    println!("METRIC t1_corpus_cut_extra_sites_seen {extra_seen}");
    println!("METRIC t1_corpus_cut_extra_sites_against_a_numeral {extra_numeral}");
    println!("METRIC t1_corpus_cut_extra_sites_unslotted {extra_unslotted}");
    println!("METRIC t1_corpus_cut_extra_sites_one_name_both_sides {extra_one_name}");
    println!("METRIC t1_corpus_cut_half_sites_claimed {half_claimed}");
    println!("METRIC t1_corpus_cut_half_sites_unslotted {half_unslotted}");
    println!("METRIC t1_corpus_cut_half_sites_two_constants {half_both}");
    println!(
        "METRIC t1_corpus_cut_half_sites_refused_by_type {}",
        half_refused.len()
    );
    println!(
        "METRIC t1_corpus_cut_half_sites_refused_by_immediate {}",
        half_out_of_range.len()
    );
    println!("METRIC t1_corpus_cut_half_sites_folded {half_folded}");
    println!("METRIC t1_corpus_cut_half_sites_widened {half_widened}");
    println!("METRIC t1_corpus_cut_half_sites_exact_in_a_paired_routine {half_exact_paired}");
    println!("METRIC t1_corpus_half_routines_exact_slot {half_exact_routines}");
    println!("METRIC t1_corpus_half_exact_claims_sensitive {half_exact_bit}");
    println!("METRIC t1_corpus_cut_half_sites_unused_routine_unvisited {half_unused_unvisited}");
    println!("METRIC t1_corpus_cut_half_sites_unused_routine_unseen {half_unused_routine_unseen}");
    println!("METRIC t1_corpus_cut_half_sites_third_widened {half_only_widened}");
    println!("METRIC t1_corpus_half_routines_visited {half_wide_routines}");
    println!("METRIC t1_corpus_half_routines_cut_recovered {half_wide_recovered}");
    println!("METRIC t1_corpus_half_sites_cut_recovered {half_wide_sites}");
    println!(
        "METRIC t1_corpus_half_routines_cut_ambiguous {}",
        half_wide_ambiguous.len()
    );
    println!(
        "METRIC t1_corpus_half_routines_cut_unexplained {}",
        half_wide_none.len()
    );
    println!(
        "METRIC t1_corpus_half_routines_no_frame {}",
        half_no_frame.len()
    );
    println!(
        "METRIC t1_corpus_half_cut_claims_sensitive {}",
        half_wide_bit.len()
    );
    println!("METRIC t1_corpus_cut_routines_narrowed_by_half {cut_narrowed_by_half}");
    println!("METRIC t1_corpus_cut_routines_recovered_only_with_half {cut_by_half}");
    println!("METRIC t1_corpus_wide_routines_narrowed_by_half {wide_narrowed_by_half}");
    println!("METRIC t1_corpus_wide_routines_recovered_only_with_half {wide_by_half}");
    println!("METRIC t1_corpus_cut_extra_sites_unused_already_paired {extra_unused_paired}");
    println!("METRIC t1_corpus_cut_extra_sites_unused_routine_unvisited {extra_unused_unvisited}");
    println!("METRIC t1_corpus_cut_extra_sites_widened {wide_extra_sites}");
    println!("METRIC t1_corpus_wide_routines_visited {wide_routines}");
    println!("METRIC t1_corpus_wide_routines_cut_recovered {wide_recovered}");
    println!("METRIC t1_corpus_wide_sites_cut_recovered {wide_sites}");
    println!(
        "METRIC t1_corpus_wide_routines_cut_ambiguous {}",
        wide_ambiguous.len()
    );
    println!(
        "METRIC t1_corpus_wide_routines_cut_unexplained {}",
        wide_none.len()
    );
    println!(
        "METRIC t1_corpus_wide_routines_no_frame {}",
        wide_no_frame.len()
    );
    println!(
        "METRIC t1_corpus_wide_cut_claims_sensitive {}",
        wide_bit_total.len()
    );
    println!("METRIC t1_corpus_cut_extra_sites_claimed {extra_claimed}");
    println!("METRIC t1_corpus_cut_routines_narrowed_by_extra {cut_narrowed}");
    println!("METRIC t1_corpus_cut_routines_recovered_only_with_extra {cut_by_extra}");
    println!(
        "METRIC t1_corpus_two_name_routines_only_the_cut_catches {}",
        only_cut.len()
    );
    for s in &only_cut {
        println!(
            "NOTE  the order census reads this routine green under the dropped exchange and no single cut position explains it: {s}"
        );
    }
    for s in &extra_refused {
        println!("NOTE  {s}");
    }
    for s in &half_refused {
        println!("NOTE  {s}");
    }
    for s in &half_out_of_range {
        println!("NOTE  {s}");
    }
    for s in &wide_no_frame {
        println!("NOTE  {s}");
    }
    for s in &half_no_frame {
        println!("NOTE  {s}");
    }
    for s in &wide_ambiguous {
        println!("NOTE  {s}");
    }
    for s in &half_wide_ambiguous {
        println!("NOTE  {s}");
    }
    for s in &cut_ambiguous {
        println!("NOTE  {s}");
    }
    for s in &only_residue {
        println!(
            "NOTE  the order census reads this routine green under the dropped exchange and the slot residue does not: {s}"
        );
    }
    for s in &residue_uninformative {
        println!("NOTE  {s}");
    }
    for s in &only_pair {
        println!(
            "NOTE  the order census reads this routine green under the dropped exchange and the slot pair does not: {s}"
        );
    }
    for s in &unpaired {
        println!("NOTE  {s}");
    }
    for s in &out_of_reach {
        println!("NOTE  {s}");
    }
    for s in &spilled {
        println!("NOTE  a spilled operand, outside the local window: {s}");
    }

    assert!(
        silent.is_empty(),
        "every source that compares two names must reach the emitter:\n  {}",
        silent.join("\n  ")
    );
    // THE HARNESS FIRST — every property below is vacuously true of a sweep
    // that read nothing. A FLOOR and not a pin (owner ruling 2026-09-13,
    // point 1): the corpus gains conditions every week.
    assert!(
        modules >= 17 && in_reach >= 200 && routines_predicted >= 140,
        "the corpus compares two names in a `यदि`/`यावत्` head in 17 modules at \
         216 sites across 156 routines; this sweep found {modules}, {in_reach} \
         and {routines_predicted}, which means the HARVESTER broke, not that \
         the corpus stopped comparing"
    );
    assert!(
        paired_routines >= 40 && paired_sites >= 55,
        "61 of those sites, in 48 routines, have NO anonymous slot and so a \
         recovered slot numbering; this sweep paired {paired_sites} in \
         {paired_routines}, which means `pairable` broke and the per-site \
         claim went quiet rather than the corpus losing the routines"
    );
    assert!(
        residue_routines >= 100 && residue_sites >= 140,
        "the residue claim reaches 153 sites in 108 routines — every routine \
         `pairable` refuses, less the sites whose two ranks are congruent \
         modulo eight; this sweep reached {residue_sites} in \
         {residue_routines}, which means `residues` went quiet rather than the \
         corpus losing the routines"
    );
    assert!(
        only_residue.len() >= 15,
        "THE RESIDUE IS THE DIRECTION CLAIM AGAIN unless it catches routines \
         the order census does not. Today it catches 18 against the exact \
         pair's 2, because it needs no recovery of the pool's position and so \
         reaches the 108 routines that have one; it caught {} here",
        only_residue.len()
    );
    assert!(
        cut_recovered >= 25 && cut_sites >= 30,
        "THE POOL'S POSITION IS RECOVERED WHERE THE OBJECT DETERMINES IT. Today \
         exactly one of the `names + १` candidate cut positions fits in 31 of \
         the 108 routines `pairable` refuses, carrying 36 sites — so those \
         routines DO have an exact `(करण, अपादान)` pair after all, taken from \
         the object by exhaustion and not from a global the builder clobbers. \
         This sweep recovered {cut_recovered} in {cut_sites} site(s), which \
         means `consistent_bases` went quiet rather than the corpus losing them"
    );
    assert!(
        not_monotone.is_empty(),
        "FOLDING THE OTHER `compare_op`s ADDED A CANDIDATE CUT, which no extra \
         constraint can do. A base must satisfy the groups it already satisfied \
         and one more, so the folded set is a SUBSET of the `न्यूनलङ्घनम्` one; a \
         miss here is the grouping wired wrongly — one mnemonic's predicted \
         ranks matched against another mnemonic's carried pairs — and every \
         recovery it reports is invented rather than witnessed:\n  {}",
        not_monotone.join("\n  ")
    );
    assert!(
        reached_further.is_empty(),
        "`drop_the_exchange` REACHED PAST `न्यूनलङ्घनम्`. It exists to lower \
         `a अधिकम् b` as `Lt(a, b)` and nothing else — same mnemonic, same line \
         count, same labels — so the other three branch words must come out \
         IDENTICAL. If they do not, the sensitivity figures below measure a \
         mutation nobody described:\n  {}",
        reached_further.join("\n  ")
    );
    assert!(
        cut_extra_sites >= 25 && extra_refused.len() >= 8,
        "THE OTHER THREE `compare_op`s ARE THE EVIDENCE THE CUT RECOVERY WAS \
         SHORT OF. The corpus writes 80 five-token heads comparing two names \
         under `समम्`/`असमम्`/`बृहत्समम्`; eleven are REFUSED — ten between two \
         RUNS, which become a CALL (`ir.t1:2200`), and one against a \
         `सम्भाव्य` — and 28 fall in a routine the cut recovery actually \
         visits. This sweep folded {cut_extra_sites} and refused {}, which \
         means `extra_pairs` went quiet rather than the corpus losing them",
        extra_refused.len()
    );
    assert_eq!(
        extra_seen,
        extra_claimed + extra_numeral + extra_unslotted + extra_one_name + extra_refused.len(),
        "EVERY FIVE-TOKEN `समम्`/`असमम्`/`बृहत्समम्` HEAD IS IN EXACTLY ONE \
         BUCKET. The harvester saw {extra_seen}; it claimed {extra_claimed} as \
         two-name pairs, {extra_numeral} carry a numeral, {extra_unslotted} \
         name something that holds no frame slot, {extra_one_name} write ONE \
         name on both sides and {} were refused by type. A site in none of \
         them is one this sweep harvested and then lost",
        extra_refused.len()
    );
    assert_eq!(
        extra_numeral,
        half_claimed + half_unslotted + half_both + half_refused.len() + half_out_of_range.len(),
        "**AND THE 370 NUMERAL SITES ARE NOT ONE SHAPE EITHER.** One figure \
         over all of them could not say whether the corpus grew a comparison \
         against a constant or an unresolvable name: {extra_numeral} carry a \
         numeral, of which {half_claimed} put ONE declared name beside it and \
         are folded, {half_unslotted} put a global read or an import there and \
         are REFUSED, {half_both} are two constants and name no slot at all, \
         {} were refused by type and {} by an immediate the object cannot \
         materialise in one `addi`",
        half_refused.len(),
        half_out_of_range.len()
    );
    assert!(
        !half_out_of_range.is_empty(),
        "**THE IMMEDIATE REFUSAL HAS NO CASE, so nobody has seen it work.** \
         `conditional_operands` answers `Origin::Literal` only for \
         `योगः <r>म् शून्यःन <k>न ।`, and the emitter writes that shape only \
         inside the `addi` window (`yantrotsarjana.t1:898`); a wider constant \
         comes through `उपरिभारः` and is `Written`. The corpus writes one — \
         `दूरम् बृहत्समम् ४०९६` — and it is what turned a ten-name frame's \
         whole recovery RED before this bucket existed. A guard whose case \
         never arises is a guard nobody has seen work"
    );
    assert_eq!(
        half_claimed,
        half_folded
            + half_widened
            + half_only_widened
            + half_exact_paired
            + half_unused_unvisited
            + half_unused_routine_unseen,
        "AND THE HALF-SITE BUCKETS MUST ADD UP THE SAME WAY. claimed \
         {half_claimed}, folded {half_folded}, widened {half_widened}, widened \
         ON THEIR OWN {half_only_widened}, an EXACT SLOT in a routine \
         `pairable` accepts {half_exact_paired}, no frame \
         {half_unused_unvisited}, in a routine no recovery in this file visits \
         at all {half_unused_routine_unseen}"
    );
    assert_eq!(
        half_unused_routine_unseen, 0,
        "A HALF-SITE ENTERED NO RECOVERY AND NO REFUSAL. Before this cycle 184 \
         of them sat in this bucket — routines whose every two-operand \
         comparison is a name against a constant, entered by neither recovery \
         because both are entered from a two-NAME head. The third widening \
         takes exactly those, and a routine with no frame window is named in \
         `half_no_frame` instead, so the only way to land here now is a routine \
         this sweep classified into nothing at all"
    );
    assert!(
        half_claimed >= 250 && half_folded >= 15 && half_unslotted >= 1,
        "**THE NAME BESIDE A CONSTANT IS THE EVIDENCE THE CUT RECOVERY WAS \
         STILL SHORT OF.** The corpus writes 370 five-token \
         `समम्`/`असमम्`/`बृहत्समम्` heads with a NUMERAL on one side; a numeral \
         pins no rank, but the name beside it holds a frame slot and \
         `slot_at(rank, b)` is a claim about it under every candidate cut. This \
         sweep claimed {half_claimed} of them, folded {half_folded} into the \
         `न्यूनलङ्घनम्` recovery and refused {half_unslotted} whose name holds no \
         slot — which means the half-site harvester went quiet rather than the \
         corpus losing them"
    );
    assert!(
        cut_by_half + wide_by_half >= 1,
        "FOLDING THE NAME-AGAINST-CONSTANT SITES IS DECORATION unless it \
         recovers a position the two-name fold leaves AMBIGUOUS. It narrowed \
         {cut_narrowed_by_half} of the `न्यूनलङ्घनम्` routines and \
         {wide_narrowed_by_half} of the widened ones, and brought \
         {cut_by_half} and {wide_by_half} down to exactly one"
    );
    assert_eq!(
        extra_claimed,
        cut_extra_sites + extra_unused_paired + wide_extra_sites + extra_unused_unvisited,
        "THE BUCKETS MUST ADD UP. Every site `extra_pairs` claimed is either \
         FOLDED into a routine the `न्यूनलङ्घनम्` cut recovery visits, or \
         unused because that routine's numbering is already recovered, or \
         WIDENED — folded into the recovery over a routine that writes no \
         `न्यूनम्`/`अधिकम्` head at all — or unused because that routine has no \
         frame window for the object to refuse a candidate with. A site in \
         none of the four is one this sweep harvested and then lost, and a \
         figure that does not add up is an instrument with two states where \
         the truth has three: \
         claimed {extra_claimed}, folded {cut_extra_sites}, already paired \
         {extra_unused_paired}, widened {wide_extra_sites}, unvisited \
         {extra_unused_unvisited}"
    );
    assert!(
        wide_routines >= 18 && wide_extra_sites >= 20 && wide_recovered >= 5,
        "THE WIDENING IS WHERE THE 27 HARVESTED SITES WENT. `want_residue` is \
         derived from `sites`, so a routine whose only two-name comparisons \
         are `समम्`, `असमम्` or `बृहत्समम्` was outside every claim in this \
         file: today there are 22 of them carrying 27 sites, and asking the \
         object which of the `names + १` cut positions explains ALL of a \
         routine's sites at once leaves EXACTLY ONE standing in 7 — positions \
         recovered from operators that never exchange their operands, with no \
         `न्यूनलङ्घनम्` evidence at all. This sweep visited {wide_routines} \
         routine(s) over {wide_extra_sites} site(s) and recovered \
         {wide_recovered}, which means the widening went quiet rather than \
         the corpus losing them"
    );
    assert!(
        wide_bit_total.is_empty(),
        "THE WIDENED CUT MOVED UNDER A MUTATION THAT CANNOT REACH IT. \
         `drop_the_exchange` rewrites `न्यूनलङ्घनम्` lines and nothing else, \
         and a widened routine writes no `न्यूनम्`/`अधिकम्` two-name head at \
         all — its whole evidence is `समम्`/`असमम्`/`बृहत्समम्`, which the \
         bitten object carries identically. A base that moves here says the \
         widened recovery is reading `न्यूनलङ्घनम्` pairs it claims not to use, \
         and every routine it reports would be double-counted by \
         `cut_sensitive`:\n  {}",
        wide_bit_total.join("\n  ")
    );
    assert!(
        wide_none.is_empty(),
        "A ROUTINE THE `न्यूनलङ्घनम्` RECOVERY NEVER VISITS NAMES SLOTS NO \
         SINGLE CUT POSITION EXPLAINS. Its `समम्`/`असमम्`/`बृहत्समम्` sites \
         constrain the SAME base as any other mnemonic's would — \
         `अनामस्थानम्` runs once per ROUTINE (`ir.t1:1346`) — and none of the \
         three exchanges its operands (`ir.t1:2186`–`:2191`), so each predicts \
         `(करण, अपादान)` in WRITTEN order. A miss here is that written-order \
         claim being wrong, a declaration the harvester ranks wrongly, or a \
         second cut:\n  {}",
        wide_none.join("\n  ")
    );
    assert!(
        half_wide_routines >= 40 && half_only_widened >= 120,
        "THE THIRD WIDENING IS WHERE THE 184 UNVISITED HALF-SITES WENT. Both \
         recoveries above are entered from a two-NAME head, so a routine whose \
         every two-operand comparison is a name against a CONSTANT was outside \
         every claim in this file however many it writes. This sweep visited \
         {half_wide_routines} such routine(s) carrying {half_only_widened} \
         site(s); fewer means the widening went quiet rather than the corpus \
         losing them"
    );
    assert!(
        !half_wide_ambiguous.is_empty(),
        "THE THIRD WIDENING REPORTS EVERY ROUTINE RECOVERED, WHICH IT CANNOT \
         HONESTLY DO. A half-site names ONE slot, so `slot_at(rank, b)` takes \
         only two values over the `names + १` candidates and a whole run of \
         bases gives the same one — a routine carrying a single half-site is \
         AMBIGUOUS by construction. If nothing came out ambiguous, the \
         exhaustion is not being asked, or a candidate set is being narrowed by \
         something other than the object"
    );
    assert!(
        half_wide_none.is_empty(),
        "A ROUTINE NO TWO-NAME RECOVERY VISITS NAMES A SLOT NO SINGLE CUT \
         POSITION EXPLAINS. Its name-against-constant sites constrain the SAME \
         base any pair would — `अनामस्थानम्` runs once per ROUTINE \
         (`ir.t1:1346`) — none of the three words exchanges its operands \
         (`ir.t1:2186`–`:2191`), and the constant and the written side are the \
         object's own and do not move with the cut. A miss here is a \
         declaration the harvester ranks wrongly, a second cut, or a side read \
         backwards:\n  {}",
        half_wide_none.join("\n  ")
    );
    assert!(
        half_wide_bit.is_empty(),
        "A CUT RECOVERED FROM NAME-AGAINST-CONSTANT HEADS ALONE MOVED UNDER A \
         MUTATION THAT CANNOT REACH IT. `drop_the_exchange` rewrites \
         `न्यूनलङ्घनम्` lines and nothing else, and these routines write no \
         `न्यूनम्`/`अधिकम्` two-name head and no two-name head of any word — \
         both pair groups are EMPTY. A base that moves here says the third \
         widening reads pairs it claims not to use:\n  {}",
        half_wide_bit.join("\n  ")
    );
    assert!(
        cut_by_extra >= 1,
        "FOLDING THE OTHER `compare_op`s IS DECORATION unless it recovers a \
         position `न्यूनलङ्घनम्` alone leaves AMBIGUOUS. Most routines carry one \
         to four `न्यूनलङ्घनम्` sites, which is why 77 of 108 came out \
         ambiguous; each extra site can only shrink the candidate set, and this \
         sweep narrowed {cut_narrowed} routine(s) and brought {cut_by_extra} \
         down to exactly one"
    );
    assert!(
        cut_sensitive >= 100,
        "THE CUT CLAIM IS THE RESIDUE AGAIN unless it bites where the residue \
         cannot. The residue is a claim PER SITE and lets every site fall on \
         whichever side of the cut suits it; this one asks ONE base to explain \
         a routine's sites AT ONCE, and under the dropped exchange it leaves \
         all 108 unpaired routines with NO base at all. It bit {cut_sensitive} \
         here"
    );
    assert!(
        !only_pair.is_empty(),
        "THE PER-SITE CLAIM IS DECORATION unless it catches a routine the \
         order census does not. Today it catches 2 — `वाक्यविभागआस्कीपाठः` \
         compares slot ४ against slot २ and slot १ against slot ४, one \
         descending and one ascending, so the dropped exchange swaps both and \
         the census still adds to 1 and 1. It caught none here"
    );
    assert!(
        blind.is_empty(),
        "THE SWEEP CANNOT SEE THE DEFECT IT EXISTS FOR, in these modules — and \
         a green from a blind instrument is indistinguishable from a green from \
         a correct compiler:\n  {}",
        blind.join("\n  ")
    );
    assert!(
        short.is_empty(),
        "A COMPARISON OF TWO NAMES CAME OUT IN THE WRONG ORDER. Parameters and \
         `चरः` locals take frame slots in first-declaration order \
         (`ir.t1:1320`), and `ir.rs:81` lowers `अधिकम्` to `Lt` WITH ITS \
         OPERANDS EXCHANGED — so `a अधिकम् b` must put `b` in the करण. \
         Dropping that exchange keeps the mnemonic, the instruction count and \
         the object length, and computes `>` as `<`:\n  {}",
        short.join("\n  ")
    );
    assert!(
        residue_short.is_empty(),
        "A COMPARISON OF TWO NAMES NAMED SLOTS THAT ARE NOT ITS RANKS MODULO \
         EIGHT. `अनामस्थानम्` (`ir.t1:1345`) interrupts the declaration run \
         ONCE and by EXACTLY `अनामस्थलसंख्या = ८`, so a name of rank `r` holds \
         slot `r` or slot `r + ८` and nothing else — whatever rank the cut fell \
         at. A miss here is a second pool, a pool of another width, or the \
         dropped exchange:\n  {}",
        residue_short.join("\n  ")
    );
    assert!(
        cut_none.is_empty(),
        "A ROUTINE'S COMPARISONS NAME SLOTS NO SINGLE CUT POSITION EXPLAINS. \
         `अनामस्थानम्` (`ir.t1:1345`) cuts the pool ONCE per routine and by \
         EXACTLY `अनामस्थलसंख्या = ८`, so ONE base `b` fixes the whole \
         numbering — rank `r` at slot `r` below it and at slot `r + ८` at or \
         above it — and every site of the routine must agree with the SAME `b`. \
         The residue lets each site pick its own side of the cut and so reads \
         this green; a miss here is a second cut, a cut of another width, a \
         declaration the harvester ranks wrongly, or the dropped exchange:\n  {}",
        cut_none.join("\n  ")
    );
    assert!(
        pair_short.is_empty(),
        "A COMPARISON OF TWO NAMES NAMED THE WRONG SLOTS. In a routine with no \
         anonymous slot, `स्थानीयघोषणम्` (`ir.t1:1319`) gives a name the slot \
         NUMBER of its first-declaration rank, so the exact `(करण, अपादान)` \
         pair is predicted and not merely its direction:\n  {}",
        pair_short.join("\n  ")
    );
    assert!(
        half_exact_routines >= 12 && half_exact_paired >= 25,
        "THE HALF-SITES IN A `pairable` ROUTINE ARE AN EXACT-SLOT CLAIM, and \
         until this cycle nobody made it: the bucket was called `unused`. A \
         routine with no anonymous pool has rank = slot (`ir.t1:1319`), so its \
         name-against-constant heads predict an exact `(slot, constant, side)` \
         and not merely a candidate cut to narrow. Today that is 34 site(s) in \
         17 routine(s); this sweep found {half_exact_paired} in \
         {half_exact_routines}, which means `half_shortfalls` went quiet \
         rather than the corpus losing them"
    );
    assert!(
        half_exact_bit >= 20,
        "THE EXACT-SLOT HALF CLAIM IS DECORATION unless a mutation moves it, \
         and no other mutation in this file CAN: `drop_the_exchange` rewrites \
         `न्यूनलङ्घनम्` alone and `reached_further` asserts the half-sites come \
         out identical under it. `swap_the_half_sides` exchanges the operands \
         of the three words they live under — same slot, same constant, only \
         the SIDE — and every one of the 34 claims must go short. It went \
         short {half_exact_bit} time(s)"
    );
    assert!(
        half_exact_blind.is_empty(),
        "A MODULE MAKES THE EXACT-SLOT HALF CLAIM AND READS THE SWAPPED OBJECT \
         GREEN. `क समम् ५` and `५ समम् क` are DIFFERENT branches and this is \
         the only claim in the file that can tell them apart; a module that \
         cannot is one whose sides are not being read, and its green says \
         nothing about the emitter:\n  {}",
        half_exact_blind.join("\n  ")
    );
    assert!(
        half_pair_short.is_empty(),
        "A COMPARISON OF A NAME AGAINST A CONSTANT NAMED THE WRONG SLOT, THE \
         WRONG CONSTANT OR THE WRONG SIDE. In a routine with no anonymous slot \
         the name's declaration rank IS its slot number (`ir.t1:1319`), none \
         of `समम्`/`असमम्`/`बृहत्समम्` exchanges its operands \
         (`ir.t1:2186`–`:2189`), and the constant fits one `addi` or the site \
         was refused outright (see [`IMMEDIATE`]) — so the exact \
         `(slot, constant, side)` triple is predicted and not merely a \
         candidate cut it could narrow:\n  {}",
        half_pair_short.join("\n  ")
    );
}

/// **THE CASES THAT MUST STILL BE REFUSED, for the two-name sweep.**
#[test]
fn a_name_with_no_frame_slot_is_reported_out_of_reach_rather_than_matched() {
    // ── ONE: A GLOBAL READ HAS NO SLOT. `पठनस्थान` is `parse.t1:129`'s module
    // global; `क्रमः` is a local. The site must be NAMED, not matched against
    // slot ०, which is the FIRST PARAMETER and a real answer for something else.
    let src = "\
मण्डलम् म ॥
सार्वजनिक चरः वैश्विकम् ॱॱ न६४ भवति ० ।
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यदि क्रमः अधिकम् वैश्विकम् आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(
        routines.len(),
        1,
        "one routine, and the module global is not one"
    );
    assert_eq!(
        routines[0].slots,
        vec!["प".to_string(), "क्रमः".to_string()],
        "the parameter takes slot ० and the `चरः` slot १ — declaration order \
         (`ir.t1:1320`), and the module global takes none"
    );
    let mut out_of_reach = Vec::new();
    let mut numeral = 0;
    let want = directions_of(&predicted_pairs(
        "म",
        &routines,
        &mut out_of_reach,
        &mut numeral,
    ));
    assert!(
        want.is_empty() && numeral == 0,
        "a global read is not a prediction: {want:?}"
    );
    assert_eq!(out_of_reach.len(), 1, "and it is NAMED: {out_of_reach:?}");
    assert!(
        out_of_reach[0].contains("वैश्विकम्"),
        "the refusal names the operand that has no slot: {}",
        out_of_reach[0]
    );

    // ── TWO: AND THE SAME SITE WITH TWO LOCALS IS IN REACH, or the refusal
    // above would be indistinguishable from a harvester that reads nothing.
    let both = src.replace("क्रमः अधिकम् वैश्विकम्", "क्रमः अधिकम् प");
    let routines = source_routines(&both);
    let mut out_of_reach = Vec::new();
    let want = directions_of(&predicted_pairs(
        "म",
        &routines,
        &mut out_of_reach,
        &mut numeral,
    ));
    assert!(out_of_reach.is_empty(), "{out_of_reach:?}");
    assert_eq!(
        want.get("मक").copied(),
        Some(Directions {
            ascending: 1,
            descending: 0
        }),
        "`क्रमः अधिकम् प` is `Lt(प, क्रमः)` — करण `प`, slot ०, against अपादान \
         `क्रमः`, slot १: ASCENDING. Read without the exchange it would be \
         descending, which is the whole defect"
    );

    // ── THREE: AND `न्यूनम्` PREDICTS THE OTHER WAY, on the same two names.
    // Without this the sweep would be satisfied by an emitter that ignored the
    // operator and always sorted its operands.
    let lt = src.replace("क्रमः अधिकम् वैश्विकम्", "क्रमः न्यूनम् प");
    let mut out_of_reach = Vec::new();
    let want = directions_of(&predicted_pairs(
        "म",
        &source_routines(&lt),
        &mut out_of_reach,
        &mut numeral,
    ));
    assert_eq!(
        want.get("मक").copied(),
        Some(Directions {
            ascending: 0,
            descending: 1
        }),
        "`क्रमः न्यूनम् प` is `Lt(क्रमः, प)` — करण slot १ against अपादान slot ०: \
         DESCENDING, the exact opposite of `अधिकम्` on the same pair"
    );

    // ── FOUR: A SPILL SLOT IS NOT A LOCAL. The corpus emits none today
    // (`t1_corpus_two_name_operands_in_a_spill_slot ०`), and a guard whose case
    // never arises is a guard nobody has seen work. Here the window opens at
    // १६, so slot ० and slot ८ are the ALLOCATOR's and the branch must be
    // dropped from the census and NAMED.
    let text = "\
॥ वैश्विकम् मक ॥
मकॱॱ
आहारः स्थिर०म् स्तूपसूचकःत् ०न ।
आहारः स्थिर१म् स्तूपसूचकःत् ८न ।
न्यूनलङ्घनम् स्थिर०न स्थिर१त् मकपर्व१य् ।";
    let mut windows = Windows::new();
    windows.insert("मक".to_string(), (16u64, 32u64));
    let mut spilled = Vec::new();
    let have = directions_of(&emitted_pairs(text, &windows, &mut spilled));
    assert!(
        have.is_empty() && spilled.len() == 1,
        "two spill slots are not two declared names: {have:?} {spilled:?}"
    );
    let mut windows = Windows::new();
    windows.insert("मक".to_string(), (0u64, 32u64));
    let mut spilled = Vec::new();
    assert_eq!(
        directions_of(&emitted_pairs(text, &windows, &mut spilled))
            .get("मक")
            .copied(),
        Some(Directions {
            ascending: 1,
            descending: 0
        }),
        "AND THE SAME TEXT INSIDE THE WINDOW IS READ — or the refusal above \
         would be a reader that sees nothing at all"
    );
    assert!(spilled.is_empty());

    // ── FIVE: A ROUTINE HEADER INSIDE A STRING LITERAL IS INDENTED, AND THE
    // COLUMN IS WHAT SAYS SO. `shrinkhala.t1:1625` writes one; read as a
    // routine it would take the rest of the file's `चरः` with it.
    let quoted = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    प भवति उक्तम् सार्वजनिक वृत्तिः ख आदाय य ॱॱ न६४ ददाति न६४ आदि इति ।
    चरः आ ॱॱ न६४ भवति ० ।
इति
";
    let routines = source_routines(quoted);
    assert_eq!(routines.len(), 1, "the quoted header opens no routine");
    assert_eq!(
        routines[0].slots,
        vec!["अ".to_string(), "आ".to_string()],
        "and `य` — a parameter of the routine in the STRING — takes no slot"
    );
}

/// **THE CASE THAT MUST STILL BE REFUSED, for the per-site SLOT NUMBER.**
///
/// A name's slot number is its declaration rank only while no anonymous slot
/// has been cut. `अनामस्थानम्` (`ir.t1:1336`) cuts EIGHT at the first run
/// growth, slice, content equality or length, and every name declared after
/// that is shifted by eight — by an event in the LOWERING that no reader of the
/// declarations can see. A routine whose emitted local count does not equal its
/// declared-name count must therefore be reported UNPAIRED and fall back to the
/// order claim, never matched against a guessed offset.
#[test]
fn a_routine_whose_anonymous_pool_position_is_unknown_is_unpaired_rather_than_offset() {
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यदि क्रमः अधिकम् प आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    assert!(unreached.is_empty() && numeral == 0, "{unreached:?}");
    assert_eq!(
        want.get("मक").map(Vec::as_slice),
        Some([(0usize, 1usize)].as_slice()),
        "`क्रमः अधिकम् प` is `Lt(प, क्रमः)` — करण slot ० against अपादान slot १, \
         an EXACT PAIR and not merely a direction"
    );

    // ── ONE: TWO DECLARED NAMES AND TWO EMITTED LOCAL SLOTS. Nothing an
    // anonymous pool could hide in, so the rank IS the number and the site is
    // paired.
    let mut windows = Windows::new();
    windows.insert("मक".to_string(), (0u64, 16u64));
    assert_eq!(
        pairable(&want, &routines, "म", &windows)
            .get("मक")
            .map(Vec::as_slice),
        Some([(0usize, 1usize)].as_slice()),
        "a routine with no anonymous slot is paired"
    );

    // ── TWO: THE SAME ROUTINE WITH THE POOL CUT — two names, TEN slots. The
    // pool sits at rank ० or rank १ and the object cannot say which, so `प` is
    // slot ० or slot ८ and `क्रमः` is slot १ or slot ९. REFUSED: reported
    // UNPAIRED, never fitted to whichever offset would agree.
    let mut windows = Windows::new();
    windows.insert("मक".to_string(), (0u64, 80u64));
    assert!(
        pairable(&want, &routines, "म", &windows).is_empty(),
        "eight slots appeared that no declaration accounts for; the numbering \
         is not recovered and must not be guessed"
    );

    // ── THREE: AND A SHORTFALL IS READ OFF THE EXACT PAIR, not off its
    // direction. `(०, १)` demanded and `(०, २)` carried is ascending both ways
    // — the order claim reads it green and this one must not.
    let have: Pairs = [("मक".to_string(), vec![(0usize, 2usize)])]
        .into_iter()
        .collect();
    let bad = pair_shortfalls(&want, &have);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(
        bad[0].contains("slot 0") && bad[0].contains("slot 1"),
        "and the refusal names the pair: {}",
        bad[0]
    );
    assert!(
        shortfalls(&directions_of(&want), &directions_of(&have)).is_empty(),
        "THE CONTROL: the order claim is satisfied by that same object, which \
         is exactly what the per-site claim exists to tighten"
    );
}

/// **A `चरः` INSIDE `उक्तम् … इति` IS TEXT AND MINTS NO SLOT.**
///
/// `shrinkhala.t1:3370` builds the scale ladder's source as a STRING, and that
/// string contains `चरः स ॱॱ न६४ भवति ० ।`. A reader that minted `स` a slot
/// shifted every rank declared after it by ONE — and nothing in this file could
/// see that until the residue claim, because the direction census compares two
/// ranks in one frame and a common shift cancels out of the comparison.
///
/// **THE CASE THAT MUST STILL BE REFUSED.** A `चरः` in real code on the SAME
/// line, after the literal has closed, is a declaration and must still mint.
/// Stopping at `उक्तम्` and never restarting would trade one blind spot for
/// another.
#[test]
fn a_char_inside_a_string_literal_mints_no_frame_slot() {
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः शिरः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् सार्वजनिक वृत्तिः घ ददाति न६४ आदि चरः स ॱॱ न६४ भवति ० । इति ।
    चरः गणकः ॱॱ न६४ भवति ० ।
    यदि गणकः अधिकम् प आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1, "{routines:?}");
    assert_eq!(
        routines[0].slots,
        vec!["प".to_string(), "शिरः".to_string(), "गणकः".to_string()],
        "`स` is four words inside a string and takes no frame slot; `शिरः`, \
         whose declaration OPENS that string, takes one"
    );

    // ── AND THE REFUSAL: code after the literal closes, on the same line.
    let after = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः शिरः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् चरः स ॱॱ न६४ इति । चरः पुच्छम् ॱॱ न६४ भवति ० ।
    प्रत्यागमनम् ० ।
इति
";
    assert_eq!(
        source_routines(after)[0].slots,
        vec!["प".to_string(), "शिरः".to_string(), "पुच्छम्".to_string()],
        "the literal ends at its `इति` and the declaration after it still mints"
    );
}

/// **THE RESIDUE REACHES THE ROUTINES `pairable` REFUSES — AND REFUSES THE
/// SITES IT CANNOT DISTINGUISH.**
///
/// `अनामस्थानम्` (`ir.t1:1345`) interrupts the declaration run ONCE and by
/// EXACTLY `अनामस्थलसंख्या = ८`, so a name of rank `r` holds slot `r` or slot
/// `r + ८` — whatever rank the cut fell at — and therefore
/// `slot ≡ rank (mod ८)`. That is a claim about the slot NUMBERS which needs no
/// recovery of the pool's position at all.
///
/// **THE CASE THAT MUST STILL BE REFUSED.** Two ranks CONGRUENT modulo eight
/// carry the same residue pair whichever way round they stand. The residue
/// says nothing about their order, so such a site is UNINFORMATIVE: named and
/// counted, never folded into the covered figure.
#[test]
fn two_ranks_eight_apart_carry_no_residue_and_are_reported_uninformative() {
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    चरः इ ॱॱ न६४ भवति ० ।
    चरः ई ॱॱ न६४ भवति ० ।
    चरः उ ॱॱ न६४ भवति ० ।
    चरः ऊ ॱॱ न६४ भवति ० ।
    चरः ऋ ॱॱ न६४ भवति ० ।
    चरः ॠ ॱॱ न६४ भवति ० ।
    यदि प न्यूनम् ॠ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि आ न्यूनम् प आदि
        प्रत्यागमनम् २ ।
    इति
    यदि अ न्यूनम् आ आदि
        प्रत्यागमनम् ३ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    assert!(unreached.is_empty() && numeral == 0, "{unreached:?}");
    assert_eq!(
        want.get("मक").map(Vec::as_slice),
        Some([(0usize, 8usize), (2usize, 0usize), (1usize, 2usize)].as_slice()),
        "ranks: प ०, अ १, आ २ … ॠ ८"
    );

    // ── ONE: THE POOL IS CUT AND THE POSITION IS UNKNOWN, so `pairable`
    // refuses the routine — nine names, seventeen emitted slots.
    let mut windows = Windows::new();
    windows.insert("मक".to_string(), (0u64, 8 * 17));
    assert!(
        pairable(&want, &routines, "म", &windows).is_empty(),
        "eight slots no declaration accounts for; the numbering is not recovered"
    );

    // ── TWO: AND THE RESIDUE STILL SPEAKS FOR TWO OF THE THREE SITES.
    // `(०, ८)` is REFUSED — ranks eight apart are congruent, so the residue
    // pair is `(०, ०)` whichever way round the operands stand.
    assert_eq!(
        residues(want["मक"].as_slice()),
        vec![(2usize, 0usize), (1usize, 2usize)],
        "the congruent site is dropped, not fitted to a residue it cannot carry"
    );

    // ── THREE: THE OBJECT WITH THE EXCHANGE DROPPED — every pair swapped, and
    // the pool cut at rank ० so every name sits eight above its rank.
    let bitten: Pairs = [(
        "मक".to_string(),
        vec![(8usize, 16usize), (8usize, 10usize), (10usize, 9usize)],
    )]
    .into_iter()
    .collect();
    let bad = residue_shortfalls(&want, &bitten);
    assert_eq!(bad.len(), 2, "{bad:?}");
    assert!(
        bad.iter().any(|s| s.contains("करण slot is 2")),
        "and the refusal names the residue pair: {bad:?}"
    );

    // ── FOUR: THE CONTROL. Those two sites are one ascending and one
    // descending both before and after the swap, so the direction census reads
    // this same object GREEN — which is exactly what the residue tightens.
    let honest: Pairs = [(
        "मक".to_string(),
        vec![(8usize, 16usize), (10usize, 8usize), (9usize, 10usize)],
    )]
    .into_iter()
    .collect();
    assert!(
        residue_shortfalls(&want, &honest).is_empty(),
        "the honest object carries every residue"
    );
    assert!(
        shortfalls(&directions_of(&want), &directions_of(&bitten)).is_empty(),
        "THE CONTROL: the order claim is satisfied by the BITTEN object — its \
         ascending and descending counts balance — and the residue is not"
    );
}

/// **ONE CUT EXPLAINS A WHOLE ROUTINE, OR THE ROUTINE IS REPORTED — AND THE
/// RESIDUE CANNOT SEE THE DIFFERENCE.**
///
/// `अनामस्थानम्` (`ir.t1:1345`) cuts the pool ONCE per routine, so a single
/// base `b` fixes every name's slot at once. The residue
/// (`slot ≡ rank (mod ८)`) is that law's per-site SHADOW: it holds whichever
/// side of the cut a name fell on, so it lets each site of a routine pick a
/// different side and still reads green. This test builds the object where
/// exactly that happens.
///
/// **AND THE TWO CASES THAT MUST STILL BE REFUSED.** A base is only REPORTED
/// when the object leaves exactly one candidate standing; when several fit, the
/// position is AMBIGUOUS and naming the agreeable one would be fitting the
/// answer to the data.
#[test]
fn a_routine_whose_sites_need_two_different_cuts_is_reported_where_the_residue_reads_green() {
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    चरः इ ॱॱ न६४ भवति ० ।
    चरः ई ॱॱ न६४ भवति ० ।
    चरः उ ॱॱ न६४ भवति ० ।
    चरः ऊ ॱॱ न६४ भवति ० ।
    चरः ऋ ॱॱ न६४ भवति ० ।
    चरः ॠ ॱॱ न६४ भवति ० ।
    चरः ऌ ॱॱ न६४ भवति ० ।
    यदि प न्यूनम् ऌ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ॠ न्यूनम् अ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    assert!(unreached.is_empty() && numeral == 0, "{unreached:?}");
    let sites = want["मक"].clone();
    assert_eq!(
        sites.as_slice(),
        [(0usize, 9usize), (8usize, 1usize)].as_slice(),
        "ranks: प ०, अ १ … ॠ ८, ऌ ९ — ten names, so eleven candidate cuts"
    );
    let names = routines[0].slots.len();
    assert_eq!(names, 10);

    // ── ONE: THE HONEST OBJECT, CUT AFTER EVERY NAME. `(०, ९)` and `(८, १)`
    // are carried as declared, and no OTHER base reproduces both — `b = ९`
    // would put `ऌ` at slot १७ and `b ≤ ८` would put `ॠ` at slot १६. So the
    // position is RECOVERED, from the object, by exhaustion.
    let honest = vec![(0usize, 9usize), (8usize, 1usize)];
    assert_eq!(
        consistent_bases(&sites, &honest, names),
        vec![10usize],
        "exactly one cut position explains both sites, so it is the position"
    );

    // ── TWO: THE OBJECT THAT NEEDS TWO CUTS. `प न्यूनम् ऌ` is carried as
    // `(०, ९)` — both names BELOW the cut — and `ॠ न्यूनम् अ` as `(१६, ९)`,
    // both ABOVE it. No single `b` does both: the first demands `b > ९` and
    // the second demands `b ≤ १`. REFUSED, and the refusal is the point.
    let crooked = vec![(0usize, 9usize), (16usize, 9usize)];
    assert!(
        consistent_bases(&sites, &crooked, names).is_empty(),
        "two sites of one routine fell on opposite sides of one cut"
    );

    // ── THREE: THE CONTROL. That same object satisfies the RESIDUE twice
    // over — `(०, ९)` and `(१६, ९)` are both `(०, १)` modulo eight, which is
    // what both sites demand — so the per-site claim reads it GREEN. This is
    // the whole reason the per-routine claim exists.
    let crooked_pairs: Pairs = [("मक".to_string(), crooked)].into_iter().collect();
    assert!(
        residue_shortfalls(&want, &crooked_pairs).is_empty(),
        "THE CONTROL: the residue is satisfied by the object no single cut \
         explains"
    );
    assert!(
        shortfalls(
            &directions_of(&want),
            &directions_of(&crooked_pairs.clone())
        )
        .is_empty(),
        "AND SO IS THE ORDER CENSUS — one ascending and one descending, both \
         before and after"
    );

    // ── FOUR: AND AMBIGUITY IS NOT AN ANSWER. One site alone, `(०, १)`
    // carried as `(०, १)`, is explained by every cut at rank २ or later —
    // nine of the eleven candidates. The existence claim stands; the position
    // does NOT, and is reported rather than chosen.
    let one = vec![(0usize, 1usize)];
    let bases = consistent_bases(&one, &[(0usize, 1usize)], names);
    assert_eq!(
        bases,
        (2..=10).collect::<Vec<usize>>(),
        "nine cut positions fit one site, so the object does not determine one"
    );
}

/// **A ROUTINE THAT WRITES NO `न्यूनम्`/`अधिकम्` HEAD AT ALL STILL HAS A CUT,
/// AND THE OTHER THREE `compare_op`s RECOVER IT ALONE — AND CANNOT BE BITTEN.**
///
/// `want_residue` is derived from [`SourceRoutine::sites`], which holds only
/// `न्यूनम्` and `अधिकम्`, so a routine whose every two-name comparison is
/// `समम्`, `असमम्` or `बृहत्समम्` was outside every claim this file made — its
/// sites were harvested by [`extra_pairs`] and then had nowhere to go. They
/// constrain the SAME base: `अनामस्थानम्` cuts the pool once per ROUTINE
/// (`ir.t1:1346`), not once per mnemonic, and none of the three exchanges its
/// operands (`ir.t1:2186`–`:2191`), so each predicts `(करण, अपादान)` in written
/// order and the exhaustion of [`consistent_bases_of`] applies unchanged.
///
/// **AND THE TWO CASES THAT MUST STILL BE REFUSED, both of them here.** An
/// object whose sites need TWO cuts leaves no base and is reported unexplained
/// — while the residue reads it green, which is why the per-routine claim
/// exists. And such a routine is NOT SENSITIVE to the dropped exchange:
/// `drop_the_exchange` rewrites `न्यूनलङ्घनम्` lines and nothing else, the
/// widened reading carries an EMPTY `न्यूनलङ्घनम्` want and so reads no
/// `न्यूनलङ्घनम्` pair at all, and folding these routines into `cut_sensitive`
/// would have that control report routines the mutation cannot reach.
///
/// Its own loader, and its own fixture.
#[test]
fn a_routine_with_no_lt_head_has_its_cut_recovered_and_cannot_be_bitten() {
    const NO_LT: &[(usize, usize)] = &[];
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    चरः इ ॱॱ न६४ भवति ० ।
    चरः ई ॱॱ न६४ भवति ० ।
    चरः उ ॱॱ न६४ भवति ० ।
    चरः ऊ ॱॱ न६४ भवति ० ।
    चरः ऋ ॱॱ न६४ भवति ० ।
    चरः ॠ ॱॱ न६४ भवति ० ।
    चरः ऌ ॱॱ न६४ भवति ० ।
    यदि प समम् ऌ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ॠ बृहत्समम् अ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 10, "प, अ … ऌ — ten names, so eleven candidate cuts");

    // ── ONE: THE `न्यूनलङ्घनम्` RECOVERY NEVER VISITS THIS ROUTINE. It writes
    // no `न्यूनम्`/`अधिकम्` head, so `sites` is empty and `predicted_pairs`
    // gives it no entry — which puts it in neither `want_paired` nor
    // `want_residue`, and every claim above this cycle passed it by.
    assert!(
        routines[0].sites.is_empty(),
        "no `न्यूनम्`/`अधिकम्` head: {:?}",
        routines[0].sites
    );
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    assert!(
        want.is_empty() && unreached.is_empty() && numeral == 0,
        "the order census predicts nothing here: {want:?} {unreached:?}"
    );

    // ── TWO: AND `extra_pairs` DOES HARVEST IT, in WRITTEN order, one group
    // per branch word.
    let extra = extra_pairs("म", &routines);
    assert_eq!(extra.claimed, 2);
    assert!(extra.refused.is_empty() && extra.numeral == 0 && extra.unslotted == 0);
    let by_word = extra.want["मक"].clone();
    assert_eq!(
        by_word["समलङ्घनम्"],
        vec![(0usize, 9usize)],
        "प ० against ऌ ९"
    );
    assert_eq!(
        by_word["अन्यूनलङ्घनम्"],
        vec![(8usize, 1usize)],
        "ॠ ८ against अ १ — `बृहत्समम्` does not exchange either"
    );

    let object = |sam: Vec<(usize, usize)>, anyuna: Vec<(usize, usize)>| {
        let pairs = |ps: Vec<(usize, usize)>| -> Pairs {
            [("मक".to_string(), ps)].into_iter().collect()
        };
        BTreeMap::from([
            ("समलङ्घनम्", pairs(sam)),
            ("विषमलङ्घनम्", Pairs::new()),
            ("अन्यूनलङ्घनम्", pairs(anyuna)),
        ])
    };

    // ── THREE: THE HONEST OBJECT, CUT AFTER EVERY NAME — RECOVERED. `(०, ९)`
    // demands `b > ९` and `(८, १)` demands `b > ८`, so `b = १०` alone fits,
    // and it is taken from operators the order census never reads.
    let honest = object(vec![(0, 9)], vec![(8, 1)]);
    assert_eq!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT, &Pairs::new(), Some(&by_word), &honest),
            &[],
            names
        ),
        vec![10usize],
        "exactly one cut position explains both `समम्`/`बृहत्समम्` sites"
    );

    // ── FOUR: AND THE `न्यूनलङ्घनम्` PAIRS ARE NOT READ. The same object,
    // handed a `न्यूनलङ्घनम्` reading that no base could explain, answers
    // IDENTICALLY — because the widened want for that word is EMPTY and an
    // empty want constrains nothing. This is what licenses part SIX: a
    // mutation confined to `न्यूनलङ्घनम्` cannot move this base.
    let poison: Pairs = [("मक".to_string(), vec![(3usize, 4usize), (4usize, 3usize)])]
        .into_iter()
        .collect();
    assert_eq!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT, &poison, Some(&by_word), &honest),
            &[],
            names
        ),
        vec![10usize],
        "the widened reading takes no evidence from `न्यूनलङ्घनम्`"
    );

    // ── FIVE: THE CASE THAT MUST BE REFUSED — TWO CUTS. `(०, ९)` demands
    // `b > ९`; `(१६, ९)` puts both of ranks ८ and १ ABOVE the cut and demands
    // `b ≤ १`. No single base, so the routine is UNEXPLAINED and never
    // recovered.
    let crooked = object(vec![(0, 9)], vec![(16, 9)]);
    assert!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT, &Pairs::new(), Some(&by_word), &crooked),
            &[],
            names
        )
        .is_empty(),
        "two sites of one routine fell on opposite sides of one cut"
    );
    // AND THE CONTROL: the residue reads that same object GREEN — ranks
    // `(०, ९)` and `(८, १)` are `(०, १)` modulo eight, and so are the slots
    // `(०, ९)` and `(१६, ९)` the object carries. The per-routine claim is the
    // only one that bites.
    assert_eq!(
        residues(&[(0, 9), (8, 1)]),
        residues(&[(0, 9), (16, 9)]),
        "THE CONTROL: the slot residue cannot tell the two objects apart"
    );

    // ── SIX: AND THE SENSITIVITY THAT MUST BE REFUSED. `drop_the_exchange`
    // exists to lower `a अधिकम् b` as `Lt(a, b)`; it rewrites `न्यूनलङ्घनम्`
    // lines and leaves every other branch word byte-identical. So this
    // routine's whole evidence survives the bitten object unchanged, its base
    // cannot move, and counting it in `cut_sensitive` would credit the
    // mutation with a routine it cannot reach.
    let text = "\
॥ वैश्विकम् मक ॥
न्यूनलङ्घनम् एकन द्वित् लक्ष्य ।
समलङ्घनम् एकन द्वित् लक्ष्य ।
विषमलङ्घनम् एकन द्वित् लक्ष्य ।
अन्यूनलङ्घनम् एकन द्वित् लक्ष्य ।";
    let bitten = drop_the_exchange(text);
    let (t, b): (Vec<&str>, Vec<&str>) = (text.lines().collect(), bitten.lines().collect());
    assert_eq!(
        t[1], "न्यूनलङ्घनम् एकन द्वित् लक्ष्य ।",
        "the fixture is in the shape the mutation rewrites"
    );
    assert_eq!(
        b[1], "न्यूनलङ्घनम् द्विन एकत् लक्ष्य ।",
        "and the mutation DOES bite `न्यूनलङ्घनम्`, or part SIX proves nothing"
    );
    assert_eq!(
        &t[2..],
        &b[2..],
        "and it reaches NO other branch word, so a routine whose only evidence \
         is `समम्`/`असमम्`/`बृहत्समम्` is insensitive to it BY CONSTRUCTION"
    );
    assert_eq!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT, &poison, Some(&by_word), &honest),
            &[],
            names
        ),
        consistent_bases_of(
            &cut_groups("मक", NO_LT, &Pairs::new(), Some(&by_word), &honest),
            &[],
            names
        ),
        "so the honest and the bitten readings of a widened routine agree, and \
         `wide_bit_total` is asserted EMPTY rather than added to \
         `cut_sensitive`"
    );
}

/// **A NAME AGAINST A CONSTANT NARROWS THE CUT, AND THE FIVE SHAPES THAT ARE
/// NOT ONE BUCKET.**
///
/// The corpus writes 370 five-token `समम्`/`असमम्`/`बृहत्समम्` heads with a
/// NUMERAL on one side against 80 that compare two names, and every one of
/// them used to be counted into a single `numeral` figure and dropped. A
/// numeral pins no rank of its own — but the name BESIDE it holds a frame
/// slot, `slot_at(rank, b)` is a claim about that slot under every candidate
/// cut, and `अनामस्थानम्` cuts the pool once per ROUTINE (`ir.t1:1346`), so it
/// is the SAME base a two-name pair constrains. See [`Half`].
///
/// **THE CASE THAT MUST STILL BE REFUSED, and it is the reason the buckets are
/// split before anything is folded.** A name that is a module global or an
/// import holds NO frame slot at all; `Origin::Slot` never answers for it, and
/// predicting rank ० — which is the FIRST PARAMETER and a real answer for
/// something else — would empty the routine's candidate set and read the
/// compiler RED for being right. Part FOUR asserts that emptying rather than
/// assuming it.
///
/// Its own loader, its own fixture, and all six buckets in one source.
#[test]
fn a_name_against_a_constant_narrows_the_cut_and_a_global_beside_one_is_refused() {
    const NO_LT: &[(usize, usize)] = &[];
    let src = "\
मण्डलम् म ॥
सार्वजनिक चरः वैश्विकम् ॱॱ न६४ भवति ० ।
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    चरः पठ ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    यदि आ समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि अ असमम् ७ आदि
        प्रत्यागमनम् २ ।
    इति
    यदि ७ असमम् वैश्विकम् आदि
        प्रत्यागमनम् ३ ।
    इति
    यदि ३ बृहत्समम् ४ आदि
        प्रत्यागमनम् ४ ।
    इति
    यदि पठ समम् ९ आदि
        प्रत्यागमनम् ५ ।
    इति
    यदि अ समम् अ आदि
        प्रत्यागमनम् ६ ।
    इति
    यदि आ बृहत्समम् ४०९६ आदि
        प्रत्यागमनम् ७ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1, "the module global is not a routine");
    let names = routines[0].slots.len();
    assert_eq!(
        routines[0].slots,
        vec![
            "प".to_string(),
            "अ".to_string(),
            "आ".to_string(),
            "पठ".to_string()
        ],
        "four names, so five candidate cuts — and `वैश्विकम्` is not one of them"
    );

    // ── ONE: SIX HEADS, SIX SHAPES, AND EVERY ONE IN EXACTLY ONE BUCKET. One
    // figure over all of them could not say whether the corpus grew a
    // comparison against a constant or a name this reader cannot resolve.
    let extra = extra_pairs("म", &routines);
    assert_eq!(
        extra.seen, 7,
        "seven five-token heads of the other three words"
    );
    assert_eq!(
        extra.claimed, 0,
        "none of the seven compares two SLOTTED names"
    );
    assert_eq!(
        (extra.numeral, extra.unslotted, extra.one_name),
        (6, 0, 1),
        "five carry a numeral; `अ समम् अ` writes ONE name on both sides and so \
         holds a slot and NO order — the opposite reason from a name that \
         holds no slot, and folded together the figure says neither"
    );
    assert_eq!(
        (
            extra.half_claimed,
            extra.half_unslotted,
            extra.half_both,
            extra.half_refused.len(),
            extra.half_out_of_range.len()
        ),
        (2, 1, 1, 1, 1),
        "of the six: two put a SCALAR name beside the constant, one puts the \
         module global `वैश्विकम्` there, `३ बृहत्समम् ४` is two constants and \
         names no slot at all, `पठ` is declared `अङ्कः अन्तः अ८` — a run, which \
         `ir.t1:2200` sends to a CALL and not a branch — and `४०९६` is outside \
         the `addi` immediate, so the object reaches it through `उपरिभारः` and \
         carries no literal for it at all"
    );
    assert!(
        extra.half_out_of_range[0].contains("उपरिभारः"),
        "and THAT refusal carries its reason, which is the emitter's and not \
         this reader's: {:?}",
        extra.half_out_of_range
    );
    assert_eq!(
        extra.seen,
        extra.claimed + extra.numeral + extra.unslotted + extra.one_name + extra.refused.len(),
        "and the six buckets add back up to the six heads"
    );
    assert_eq!(
        extra.numeral,
        extra.half_claimed
            + extra.half_unslotted
            + extra.half_both
            + extra.half_refused.len()
            + extra.half_out_of_range.len(),
        "and the five numeral buckets add back up to the six numeral heads"
    );
    assert!(
        !extra.half_want["मक"].contains_key("अन्यूनलङ्घनम्"),
        "AND THE WIDE CONSTANT IS PREDICTED NOWHERE. `आ बृहत्समम् ४०९६` names \
         rank २ every bit as well as `आ समम् ५` does, and folding it in would \
         demand a branch the object was RIGHT not to carry — which is exactly \
         how `यन्त्रोत्सर्जनयन्त्रशाखादूरपरीक्षा` went red: {:?}",
        extra.half_want["मक"]
    );
    assert_eq!(
        extra.half_want["मक"]["समलङ्घनम्"],
        vec![(2usize, 5i128, true)],
        "`आ समम् ५` — rank २, the constant ५, and `आ` is the करण because `समम्` \
         does not exchange its operands (`ir.t1:2186`)"
    );
    assert_eq!(
        extra.half_want["मक"]["विषमलङ्घनम्"],
        vec![(1usize, 7i128, true)],
        "`अ असमम् ७` — rank १, the constant ७, करण side"
    );

    // ── TWO: THE OBJECT'S SIDE, AND WHAT IT IS NOT. A branch of two slots is
    // `emitted_pairs_of`'s and must not arrive here as a half-site; a slot
    // below the window is the allocator's.
    let text = "\
॥ वैश्विकम् मक ॥
आहारः एकम् स्तूपसूचकःत् ८०न ।
योगः द्विम् शून्यःन ५न ।
समलङ्घनम् एकन द्वित् मकपर्व१य् ।
आहारः त्रिम् स्तूपसूचकःत् ८न ।
योगः चतुर्म् शून्यःन ७न ।
विषमलङ्घनम् त्रिन चतुर्त् मकपर्व२य् ।
आहारः पञ्चम् स्तूपसूचकःत् ०न ।
समलङ्घनम् पञ्चन एकत् मकपर्व३य् ।";
    let mut windows = Windows::new();
    windows.insert("मक".to_string(), (0u64, 104u64));
    let mut spilled = Vec::new();
    let honest: BTreeMap<&'static str, HalfPairs> = EXTRA_WORDS
        .into_iter()
        .map(|m| (m, emitted_halves_of(text, &windows, m, &mut spilled)))
        .collect();
    assert!(spilled.is_empty(), "{spilled:?}");
    assert_eq!(
        honest["समलङ्घनम्"]["मक"],
        vec![(10usize, 5i128, true)],
        "one `समलङ्घनम्` has one slot and one constant; the other has TWO \
         slots and belongs to `emitted_pairs_of`, which is why it is not here"
    );
    assert_eq!(honest["विषमलङ्घनम्"]["मक"], vec![(1usize, 7i128, true)]);
    let mut narrow = Windows::new();
    narrow.insert("मक".to_string(), (16u64, 104u64));
    let mut spilled = Vec::new();
    assert_eq!(
        emitted_halves_of(text, &narrow, "विषमलङ्घनम्", &mut spilled)
            .get("मक")
            .map(Vec::len),
        None,
        "AND A SLOT BELOW THE WINDOW IS A SPILL, not a declared name"
    );
    assert_eq!(spilled.len(), 1, "and it is NAMED: {spilled:?}");

    // ── THREE: AND THE TWO HALF-SITES LEAVE EXACTLY ONE CUT. Rank २ carried at
    // slot १० puts the cut at or below २; rank १ carried at slot १ puts it
    // strictly above १. Only `b = २` is both, and this routine writes no
    // `न्यूनलङ्घनम्` and no two-name pair at all — the whole recovery is
    // constants.
    let half_want = extra.half_want.get("मक");
    assert_eq!(
        consistent_bases_of(
            &[(NO_LT, &[])],
            &cut_halves("मक", half_want, &honest),
            names
        ),
        vec![2usize],
        "two names against two constants recover the position"
    );
    assert_eq!(
        consistent_bases_of(&[(NO_LT, &[])], &[], names),
        (0..=4).collect::<Vec<usize>>(),
        "THE CONTROL: without them all five candidate cuts stand, so the \
         recovery above is the constants' and not the exhaustion's own"
    );

    // ── FOUR: **THE CASE THAT MUST STILL BE REFUSED.** `वैश्विकम्` is a module
    // global with no frame slot. Had the harvester ranked it — rank ० is the
    // first parameter and there is always one — this routine's candidate set
    // would go EMPTY and the sweep would report the compiler red for emitting
    // the right object. It is counted in `half_unslotted` and predicted
    // nowhere, and this is that emptying, asserted rather than assumed.
    let mut invented: ByWordHalf = half_want.cloned().unwrap_or_default();
    invented
        .entry("विषमलङ्घनम्")
        .or_default()
        .push((0usize, 7i128, false));
    assert!(
        consistent_bases_of(
            &[(NO_LT, &[])],
            &cut_halves("मक", Some(&invented), &honest),
            names
        )
        .is_empty(),
        "predicting a slot for the global empties the set — which is why the \
         two shapes are split and counted apart BEFORE either is folded"
    );

    // ── FIVE: AND THE SIDE IS PART OF THE CLAIM. `क समम् ५` and `५ समम् क`
    // name the same slot and the same constant and are DIFFERENT branches; a
    // reader that dropped the side would accept an emitter that swapped them,
    // which is this file's own defect in its one-name form.
    let crooked_text = text.replace("समलङ्घनम् एकन द्वित् मकपर्व१य् ।", "समलङ्घनम् द्विन एकत् मकपर्व१य् ।");
    let crooked: BTreeMap<&'static str, HalfPairs> = EXTRA_WORDS
        .into_iter()
        .map(|m| {
            (
                m,
                emitted_halves_of(&crooked_text, &windows, m, &mut Vec::new()),
            )
        })
        .collect();
    assert_eq!(
        crooked["समलङ्घनम्"]["मक"],
        vec![(10usize, 5i128, false)],
        "the same slot and the same constant, on the other side"
    );
    assert!(
        consistent_bases_of(
            &[(NO_LT, &[])],
            &cut_halves("मक", half_want, &crooked),
            names
        )
        .is_empty(),
        "and NO cut position explains it, because none ever moves the side"
    );

    // ── SIX: AND THE FOLD IS MONOTONE, as it is for the pairs. A base the
    // half-sites leave standing is one the reading without them left standing.
    let alone = consistent_bases_of(&[(NO_LT, &[])], &[], names);
    for object in [&honest, &crooked] {
        assert!(
            consistent_bases_of(&[(NO_LT, &[])], &cut_halves("मक", half_want, object), names)
                .iter()
                .all(|b| alone.contains(b)),
            "folding a half-site can only shrink the candidate set"
        );
    }
}

/// **THE OTHER THREE `compare_op`s NARROW THE CUT TO ONE, REFUSE AN OBJECT THE
/// `न्यूनलङ्घनम्` READING ALONE ACCEPTS, AND ARE THEMSELVES REFUSED WHEN THE
/// TYPE SAYS NO BRANCH EXISTS.**
///
/// The cut recovery reads one mnemonic, and most routines write it once or
/// twice — too little to leave a single candidate standing. `समम्`, `असमम्` and
/// `बृहत्समम्` pair two slots of the SAME frame under the SAME cut
/// (`अनामस्थानम्` runs once per routine, `ir.t1:1346`), so each of their sites
/// is another constraint on the one base. This fixture is its own loader, and
/// it carries all four of the things that can go wrong.
#[test]
fn the_other_comparisons_narrow_the_cut_and_a_run_typed_one_is_refused_not_predicted() {
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    चरः इ ॱॱ न६४ भवति ० ।
    चरः ई ॱॱ न६४ भवति ० ।
    चरः उ ॱॱ न६४ भवति ० ।
    चरः ऊ ॱॱ न६४ भवति ० ।
    चरः ऋ ॱॱ न६४ भवति ० ।
    चरः ॠ ॱॱ न६४ भवति ० ।
    चरः ऌ ॱॱ न६४ भवति ० ।
    चरः पठ ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    चरः पाठ ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    यदि प न्यूनम् अ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ॠ समम् ऌ आदि
        प्रत्यागमनम् २ ।
    इति
    यदि पठ समम् पाठ आदि
        प्रत्यागमनम् ३ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 12, "प, अ … ऌ, पठ, पाठ");
    // THE TYPES ARE READ — a parameter's off its `ॱॱ` and a `चरः`'s off the
    // tokens before `भवति`. Without them the run comparison below cannot be
    // told from the scalar one.
    assert_eq!(routines[0].types["प"], "न६४");
    assert_eq!(routines[0].types["पठ"], "अङ्कः अन्तः अ८");

    // ── ONE: THE HARVEST SPLITS BY OPERATOR, AND THE RUN SITE IS NAMED.
    assert_eq!(
        routines[0].sites.len(),
        1,
        "`न्यूनम्`/`अधिकम्` stay with the order census"
    );
    assert_eq!(routines[0].other.len(), 2, "the two `समम्` heads");
    let extra = extra_pairs("म", &routines);
    assert_eq!(
        extra.refused.len(),
        1,
        "the run comparison is NAMED, not predicted: {:?}",
        extra.refused
    );
    assert!(
        extra.refused[0].contains("खण्डसाम्यरचना"),
        "and the refusal carries the reason: {:?}",
        extra.refused
    );
    assert_eq!(
        extra.want["मक"]["समलङ्घनम्"],
        vec![(8usize, 9usize)],
        "ranks ॠ ८ and ऌ ९, in WRITTEN order — `समम्` does not exchange"
    );

    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    assert!(unreached.is_empty() && numeral == 0, "{unreached:?}");
    let lt = want["मक"].clone();
    assert_eq!(
        lt.as_slice(),
        [(0usize, 1usize)].as_slice(),
        "प ० against अ १"
    );

    // ── TWO: `न्यूनलङ्घनम्` ALONE IS AMBIGUOUS. One site, `(०, १)` carried as
    // written, is explained by every cut at rank २ or later.
    let lt_have: Pairs = [("मक".to_string(), vec![(0usize, 1usize)])]
        .into_iter()
        .collect();
    assert_eq!(
        consistent_bases(&lt, &[(0usize, 1usize)], names),
        (2..=12).collect::<Vec<usize>>(),
        "eleven candidate cuts survive the one `न्यूनलङ्घनम्`"
    );

    // ── THREE: AND ONE `समलङ्घनम्` LEAVES EXACTLY ONE. Ranks ८ and ९ carried as
    // `(८, १७)` put the cut strictly between them: `b = ९`, and nothing else.
    let narrowed: BTreeMap<&'static str, Pairs> = [(
        "समलङ्घनम्",
        [("मक".to_string(), vec![(8usize, 17usize)])]
            .into_iter()
            .collect::<Pairs>(),
    )]
    .into_iter()
    .collect();
    assert_eq!(
        consistent_bases_of(
            &cut_groups("मक", &lt, &lt_have, extra.want.get("मक"), &narrowed),
            &[],
            names
        ),
        vec![9usize],
        "the extra site narrows eleven candidates to ONE — the position is \
         RECOVERED where `न्यूनलङ्घनम्` alone leaves it ambiguous"
    );

    // ── FOUR: THE CASE THAT MUST STILL BE REFUSED. `(१७, ९)` — the two names
    // the other way round — is carried by NO cut: `b = ९` predicts `(८, १७)`
    // and nothing predicts the swap. EMPTY is a RED, and it is the whole point
    // of folding the mnemonic in.
    let crooked: BTreeMap<&'static str, Pairs> = [(
        "समलङ्घनम्",
        [("मक".to_string(), vec![(17usize, 9usize)])]
            .into_iter()
            .collect::<Pairs>(),
    )]
    .into_iter()
    .collect();
    assert!(
        consistent_bases_of(
            &cut_groups("मक", &lt, &lt_have, extra.want.get("मक"), &crooked),
            &[],
            names
        )
        .is_empty(),
        "no cut position puts rank ८ at slot १७ and rank ९ at slot ९"
    );

    // ── FIVE: THE CONTROL, AND THE REASON THE GROUPING IS PER MNEMONIC. That
    // same crooked object read WITHOUT its `समलङ्घनम्` group — the fold wired
    // as one flat list, so a `समम्` want could be satisfied by a
    // `न्यूनलङ्घनम्` the object happens to carry — reads GREEN, with eleven
    // candidates still standing. The constraint would be silently dropped.
    assert_eq!(
        consistent_bases_of(&cut_groups("मक", &lt, &lt_have, None, &crooked), &[], names),
        (2..=12).collect::<Vec<usize>>(),
        "THE CONTROL: drop the group and the refusal above disappears"
    );

    // ── SIX: AND THE FOLD IS MONOTONE. Whatever the object carries, a base the
    // folded reading keeps is one `न्यूनलङ्घनम्` alone kept — more constraints,
    // never fewer.
    let alone = consistent_bases(&lt, &[(0usize, 1usize)], names);
    for object in [&narrowed, &crooked] {
        assert!(
            consistent_bases_of(
                &cut_groups("मक", &lt, &lt_have, extra.want.get("मक"), object),
                &[],
                names
            )
            .iter()
            .all(|b| alone.contains(b)),
            "folding can only shrink the candidate set"
        );
    }
}

/// **A ROUTINE WHOSE EVERY COMPARISON IS A NAME AGAINST A CONSTANT HAS ITS CUT
/// RECOVERED OR IS NAMED — AND ONE HALF-SITE IS NOT ENOUGH.**
///
/// Both recoveries above are entered from a two-NAME head: the `न्यूनलङ्घनम्`
/// one from [`SourceRoutine::sites`], the widening from `extra.want`. A routine
/// that writes neither is visited by nothing, however many `<name> <op>
/// <numeral>` heads it carries — 184 such sites last cycle, the largest bucket
/// this file had left. The third widening enters the SAME exhaustion with BOTH
/// pair groups empty, because `अनामस्थानम्` cuts the pool once per ROUTINE
/// (`ir.t1:1346`) and a half-site is a claim about the SAME base.
///
/// **THE CASES THAT MUST STILL BE REFUSED, and here the first one is the
/// COMMON case rather than the exception.**
///
/// ONE half-site names ONE slot, so `slot_at(rank, b)` takes only two values
/// over the `names + १` candidates and a whole RUN of bases gives the same one.
/// Such a routine cannot come out with fewer than two bases and must be
/// reported AMBIGUOUS — part TWO. Then the side and the constant are the
/// object's own and do NOT move with the cut, so an object that swapped either
/// must empty the set rather than be fitted — parts FOUR and FIVE. And a
/// routine with NO FRAME WINDOW is the one this ledger guessed backwards: its
/// HAVE side is empty, so every candidate DIES and the routine would read
/// UNEXPLAINED — the census's own blindness in the RED direction, not the
/// permissive one — which is what `half_no_frame` exists to name (part SIX).
///
/// Its own loader, and its own fixture.
#[test]
fn a_routine_whose_only_comparison_is_against_a_constant_needs_more_than_one_half_site() {
    const NO_LT: &[(usize, usize)] = &[];
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    चरः इ ॱॱ न६४ भवति ० ।
    चरः ई ॱॱ न६४ भवति ० ।
    चरः उ ॱॱ न६४ भवति ० ।
    चरः ऊ ॱॱ न६४ भवति ० ।
    चरः ऋ ॱॱ न६४ भवति ० ।
    चरः ॠ ॱॱ न६४ भवति ० ।
    चरः ऌ ॱॱ न६४ भवति ० ।
    यदि अ समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ई असमम् ७ आदि
        प्रत्यागमनम् २ ।
    इति
    यदि ८ बृहत्समम् उ आदि
        प्रत्यागमनम् ३ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 10, "प, अ … ऌ — ten names, so eleven candidate cuts");

    // ── ONE: NEITHER RECOVERY ABOVE VISITS THIS ROUTINE. It writes no
    // `न्यूनम्`/`अधिकम्` head, so `sites` is empty and it is in neither
    // `want_paired` nor `want_residue`; and it writes no two-NAME head of any
    // other word either, so `extra.want` has no entry and the widening passes
    // it by as well. Before the third widening its half-sites landed in
    // `half_unused_routine_unseen` and no claim was made about them at all.
    assert!(
        routines[0].sites.is_empty(),
        "no `न्यूनम्`/`अधिकम्` head: {:?}",
        routines[0].sites
    );
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    assert!(
        predicted_pairs("म", &routines, &mut unreached, &mut numeral).is_empty()
            && unreached.is_empty()
            && numeral == 0,
        "the order census predicts nothing here"
    );
    let extra = extra_pairs("म", &routines);
    assert!(
        extra.want.is_empty() && extra.claimed == 0,
        "and no two-name pair is harvested, so the SECOND widening never \
         visits it either: {:?}",
        extra.want
    );
    assert_eq!(
        (extra.seen, extra.numeral, extra.half_claimed),
        (3, 3, 3),
        "all three heads are name-against-constant and all three are claimed"
    );
    let by_word = extra.half_want["मक"].clone();
    assert_eq!(
        by_word["समलङ्घनम्"],
        vec![(1usize, 5i128, true)],
        "अ is rank १ and is written in the करण"
    );
    assert_eq!(
        by_word["विषमलङ्घनम्"],
        vec![(4usize, 7i128, true)],
        "ई is rank ४, करण"
    );
    assert_eq!(
        by_word["अन्यूनलङ्घनम्"],
        vec![(5usize, 8i128, false)],
        "`८ बृहत्समम् उ` puts उ — rank ५ — in the अपादान, and \
         `बृहत्समम्` exchanges nothing"
    );

    let object = |halves: Vec<(&'static str, Vec<Half>)>| {
        let mut m: BTreeMap<&'static str, HalfPairs> = EXTRA_WORDS
            .into_iter()
            .map(|w| (w, HalfPairs::new()))
            .collect();
        for (w, hs) in halves {
            m.insert(w, [("मक".to_string(), hs)].into_iter().collect());
        }
        m
    };
    let one_word =
        |w: &'static str| -> ByWordHalf { [(w, by_word[w].clone())].into_iter().collect() };
    let bases = |want: &ByWordHalf, have: &BTreeMap<&'static str, HalfPairs>| {
        consistent_bases_of(
            &cut_groups("मक", NO_LT, &Pairs::new(), None, &BTreeMap::new()),
            &cut_halves("मक", Some(want), have),
            names,
        )
    };

    // ── TWO: THE CASE THAT MUST BE REFUSED, AND IT IS THE COMMON ONE. ONE
    // half-site, honestly carried: `slot_at(१, b)` is १ for every `b > १` and ९
    // otherwise, so slot १ leaves NINE candidates standing. The position is
    // AMBIGUOUS and must be NAMED — reporting it recovered would be picking one
    // of nine because it agrees.
    let lone = one_word("समलङ्घनम्");
    let lone_have = object(vec![("समलङ्घनम्", vec![(1usize, 5i128, true)])]);
    assert_eq!(
        bases(&lone, &lone_have),
        (2..=10).collect::<Vec<usize>>(),
        "one half-site fixes only which SIDE of the cut rank १ falls on"
    );

    // ── THREE: AND THREE OF THEM DO PIN IT. Cut at `b = ५`: ranks १ and ४ fall
    // below it and keep their numbers, rank ५ is at or above it and is shifted
    // by the eight-slot pool. `(१, ५, करण)` demands `b > १`, `(४, ७, करण)`
    // demands `b > ४` and `(१३, ८, अपादान)` demands `b ≤ ५` — one base, taken
    // from constants alone with no two-name evidence anywhere.
    let honest = object(vec![
        ("समलङ्घनम्", vec![(1usize, 5i128, true)]),
        ("विषमलङ्घनम्", vec![(4usize, 7i128, true)]),
        ("अन्यूनलङ्घनम्", vec![(13usize, 8i128, false)]),
    ]);
    assert_eq!(
        bases(&by_word, &honest),
        vec![5usize],
        "three half-sites of one routine leave exactly one cut position"
    );

    // ── FOUR: THE SIDE IS REFUSED — this is the defect the whole file exists
    // for, in its one-name form. `८ बृहत्समम् उ` and `उ बृहत्समम् ८` name the
    // same slot and the same constant and are DIFFERENT branches, and none of
    // these three words exchanges its operands (`ir.t1:2186`–`:2191`). An
    // object that put उ in the करण must empty the set, not shift the cut to
    // suit it.
    let swapped = object(vec![
        ("समलङ्घनम्", vec![(1usize, 5i128, true)]),
        ("विषमलङ्घनम्", vec![(4usize, 7i128, true)]),
        ("अन्यूनलङ्घनम्", vec![(13usize, 8i128, true)]),
    ]);
    assert!(
        bases(&by_word, &swapped).is_empty(),
        "the करण and the अपादान of a half-site were exchanged and no cut \
         position can absorb it"
    );

    // ── FIVE: AND SO IS THE CONSTANT. It is the object's own and does not move
    // with the cut, so a branch against ९ where the source wrote ८ is a
    // different branch and no base explains it.
    let other_constant = object(vec![
        ("समलङ्घनम्", vec![(1usize, 5i128, true)]),
        ("विषमलङ्घनम्", vec![(4usize, 7i128, true)]),
        ("अन्यूनलङ्घनम्", vec![(13usize, 9i128, false)]),
    ]);
    assert!(
        bases(&by_word, &other_constant).is_empty(),
        "only the SLOT moves with the cut; the constant is matched as written"
    );

    // ── SIX: AND THE NO-FRAME BUCKET, WHOSE DIRECTION THIS LEDGER HAD
    // BACKWARDS. `emitted_halves_of` reads `windows` and skips a routine it has
    // no entry for, so a routine the emitter lays out no frame for has an EMPTY
    // have side — and a non-empty want is then satisfied by NO base at all, so
    // it would be reported UNEXPLAINED rather than ambiguous. The vacuous
    // survival is what happens when the WANT is empty, which is a different
    // routine entirely. Both directions are asserted so the bucket cannot be
    // read the wrong way again.
    let no_frame = object(vec![]);
    assert!(
        bases(&by_word, &no_frame).is_empty(),
        "no frame window means no half-site carried, and every candidate cut \
         dies on a constraint this census cannot read — which is why \
         `half_no_frame` names such a routine instead of letting the recovery \
         report it"
    );
    assert_eq!(
        bases(&ByWordHalf::new(), &no_frame),
        (0..=10).collect::<Vec<usize>>(),
        "THE CONTROL: it is an EMPTY WANT that survives vacuously, not an \
         empty have"
    );

    // ── SEVEN: AND THE PAIR GROUPS ARE NOT READ, which is what licenses
    // `half_wide_bit` being asserted EMPTY rather than added to
    // `cut_sensitive`. `drop_the_exchange` rewrites `न्यूनलङ्घनम्` lines and
    // nothing else; this routine's want carries no pair of any word, so an
    // object handed a `न्यूनलङ्घनम्` reading no base could explain answers
    // IDENTICALLY.
    let poison: Pairs = [("मक".to_string(), vec![(3usize, 4usize), (4usize, 3usize)])]
        .into_iter()
        .collect();
    assert_eq!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT, &poison, None, &BTreeMap::new()),
            &cut_halves("मक", Some(&by_word), &honest),
            names
        ),
        vec![5usize],
        "the third widening takes no evidence from any pair, so a mutation \
         confined to `न्यूनलङ्घनम्` cannot move its base"
    );
}

/// **A HALF-SITE IN A ROUTINE WITH NO ANONYMOUS POOL PREDICTS AN EXACT SLOT —
/// AND THE THREE CASES IT MUST STILL REFUSE.**
///
/// `pairable` accepts a routine whose declared-name count equals its emitted
/// local slot count: there is no pool to shift anything, so slot `k` IS
/// declaration rank `k` (`ir.t1:1319`). Every other reader of a half-site in
/// this file only ever NARROWS a candidate cut; in such a routine there is no
/// cut to narrow, so its `<name> <op> <numeral>` heads predict the exact
/// `(slot, constant, side)` triple the object must carry and are the one
/// half-site use whose miss is RED rather than AMBIGUOUS.
///
/// **THE CASES THAT MUST STILL BE REFUSED.** A routine `pairable` refuses (its
/// rank is its slot or its rank plus eight and the object does not say which);
/// a constant outside the `addi` immediate, which carries no `Origin::Literal`
/// for the reader to match at all; and a want the object OVER-carries, because
/// `यावत्` rotation emits one source comparison twice and the floor is per
/// distinct triple.
#[test]
fn a_half_site_in_a_routine_with_no_anonymous_pool_predicts_an_exact_slot() {
    // THIS TEST'S OWN SOURCE. `प` takes slot ० and `क्रमः` slot १ — declaration
    // order (`ir.t1:1320`) — and the `समम्` head puts the NAME in the करण.
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यदि क्रमः समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1, "{routines:?}");
    assert_eq!(
        routines[0].slots,
        vec!["प".to_string(), "क्रमः".to_string()],
        "the parameter takes slot ० and the `चरः` slot १"
    );
    let extra = extra_pairs("म", &routines);
    assert_eq!(extra.half_claimed, 1, "one half-site is claimed");
    assert_eq!(
        extra.half_want.get("मक").and_then(|w| w.get("समलङ्घनम्")),
        Some(&vec![(1usize, 5i128, true)]),
        "`क्रमः समम् ५` predicts rank १ against the constant ५, the NAME in \
         the करण — an exact triple and not a candidate cut: {:?}",
        extra.half_want
    );

    // THE ROUTINE IS `pairable`: two declared names, two emitted local slots,
    // so there is nothing an anonymous pool could hide in.
    let mut windows = Windows::new();
    windows.insert("मक".to_string(), (0u64, 16u64));
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want_pairs = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    // No two-NAME `न्यूनम्`/`अधिकम्` head here, so `pairable` is asked directly
    // about the same routine the sweep would hand it.
    let seed: Pairs = [("मक".to_string(), Vec::new())].into_iter().collect();
    assert!(
        want_pairs.is_empty() && unreached.is_empty(),
        "the only comparison is a name against a CONSTANT: {want_pairs:?}"
    );
    let paired = pairable(&seed, &routines, "म", &windows);
    assert_eq!(
        paired.keys().collect::<Vec<_>>(),
        vec!["मक"],
        "two names and two slots — the numbering is recovered, and THIS is the \
         map the sweep hands `half_shortfalls`"
    );

    let have = |w: &'static str, v: Vec<Half>| -> BTreeMap<&'static str, HalfPairs> {
        [(w, [("मक".to_string(), v)].into_iter().collect())]
            .into_iter()
            .collect()
    };

    // ── ONE: THE OBJECT CARRIES THE TRIPLE. Green, or every red below is a
    // reader that reads nothing rather than a claim that holds.
    assert!(
        half_shortfalls(
            &paired,
            &extra.half_want,
            &have("समलङ्घनम्", vec![(1, 5, true)])
        )
        .is_empty(),
        "the exact triple satisfies the claim"
    );

    // ── TWO: THE WRONG SLOT. `(०, ५, करण)` is the OTHER declared name against
    // the same constant on the same side.
    let bad = half_shortfalls(
        &paired,
        &extra.half_want,
        &have("समलङ्घनम्", vec![(0, 5, true)]),
    );
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(
        bad[0].contains("slot 1") && bad[0].contains("करण"),
        "and the refusal names the slot and the side: {}",
        bad[0]
    );

    // ── THREE: THE WRONG SIDE, WHICH IS THE WHOLE REASON [`Half`] CARRIES A
    // THIRD FIELD. `५ समम् क्रमः` names the same slot and the same constant
    // and is a DIFFERENT branch; a reader that dropped the side reads it green.
    let bad = half_shortfalls(
        &paired,
        &extra.half_want,
        &have("समलङ्घनम्", vec![(1, 5, false)]),
    );
    assert_eq!(bad.len(), 1, "the exchanged side is caught: {bad:?}");

    // ── FOUR: THE WRONG WORD. The want is matched only against the half-sites
    // the object carries under the SAME branch word — a `समम्` satisfied by a
    // `विषमलङ्घनम्` would be a constraint silently dropped.
    let bad = half_shortfalls(
        &paired,
        &extra.half_want,
        &have("विषमलङ्घनम्", vec![(1, 5, true)]),
    );
    assert_eq!(bad.len(), 1, "the wrong word is caught: {bad:?}");

    // ── FIVE, REFUSED: A ROUTINE `pairable` REFUSES. The same source with the
    // pool cut — two names, TEN slots — has `क्रमः` at slot १ or slot ९ and
    // the object does not say which, so it keeps the cut recovery and must not
    // be matched against its rank, however empty the object is.
    let mut cut = Windows::new();
    cut.insert("मक".to_string(), (0u64, 80u64));
    assert!(
        pairable(&seed, &routines, "म", &cut).is_empty(),
        "eight slots no declaration accounts for; the numbering is not recovered"
    );
    assert!(
        half_shortfalls(&Pairs::new(), &extra.half_want, &BTreeMap::new()).is_empty(),
        "a routine the sweep did not pair is NOT a shortfall — it falls back to \
         the cut recovery, and reading it red here would read the compiler red \
         for being right"
    );

    // ── SIX, REFUSED: A CONSTANT OUTSIDE THE `addi` IMMEDIATE. The emitter
    // materialises it with `उपरिभारः` (`yantrotsarjana.t1:898`), so the object
    // carries no `Origin::Literal` and hence NO half-site — the site never
    // enters `half_want` and can never become a shortfall, however empty the
    // object is.
    let wide = src.replace("क्रमः समम् ५", "क्रमः समम् ४०९६");
    let wide_routines = source_routines(&wide);
    let wide_extra = extra_pairs("म", &wide_routines);
    assert_eq!(
        (wide_extra.half_claimed, wide_extra.half_out_of_range.len()),
        (0, 1),
        "४०९६ is outside `IMMEDIATE` and is NAMED, not claimed: {:?}",
        wide_extra.half_out_of_range
    );
    assert!(
        half_shortfalls(&paired, &wide_extra.half_want, &BTreeMap::new()).is_empty(),
        "and so an EMPTY object is green for it — the refusal, not a red"
    );

    // ── SEVEN, REFUSED: THE FLOOR IS PER DISTINCT TRIPLE. `यावत्` rotation
    // emits one source comparison twice, so the object may carry MORE than the
    // source demands and only going SHORT is a miss.
    assert!(
        half_shortfalls(
            &paired,
            &extra.half_want,
            &have("समलङ्घनम्", vec![(1, 5, true), (1, 5, true)]),
        )
        .is_empty(),
        "carrying the triple twice for one source head is the rotation, not a \
         defect"
    );
    let twice: BTreeMap<String, ByWordHalf> = [(
        "मक".to_string(),
        [("समलङ्घनम्", vec![(1usize, 5i128, true), (1, 5, true)])]
            .into_iter()
            .collect(),
    )]
    .into_iter()
    .collect();
    assert_eq!(
        half_shortfalls(&paired, &twice, &have("समलङ्घनम्", vec![(1, 5, true)])).len(),
        1,
        "and THE CONTROL: two wanted against one carried IS short, or the \
         multiset is not being counted at all"
    );
}

/// **EVERY OFFSET THE CORPUS ADDRESSES OFF `स्तूपसूचकः`, CLASSIFIED BY WHICH
/// REGION OF ITS ROUTINE'S FRAME IT FALLS IN — AND NOTHING FALLS OUTSIDE.**
///
/// The sweeps above ask ONE question of an offset: is it inside the local
/// window? That is the right question for pairing a slot against a declaration,
/// and it is the only question a `(lo, hi)` window can answer. It is also a
/// question with two outcomes, while the frame has FOUR regions and a gap — so
/// "not in the window" has been standing in for spill, saved, return address,
/// an incoming argument in the caller's frame, and *an offset that belongs to
/// none of them*. The last is a reader defect wearing the same answer as four
/// correct ones.
///
/// **THIS SWEEP IS OVER EVERY SP-RELATIVE ACCESS, NOT OVER CONDITIONAL
/// OPERANDS, AND THAT DISTINCTION IS THE WHOLE TEST.** The first version of it
/// walked `conditional_operands` and answered `{"local": 3363}` — 3,363 sites,
/// one band, and the other five never exercised. It would have passed
/// unchanged with five of `SlotBand`'s six arms deleted. The cause is not that
/// the corpus has no spills: it is that a saved `स्थिर` and `पुनःस्थानम्` are
/// written by the prologue and read back by the epilogue, and NEVER reach a
/// comparison. A census that can only see comparisons cannot see them, and a
/// green from it is a statement about the reader's reach, not about the
/// emitter.
///
/// **WHY THIS IS NOT A TAUTOLOGY.** The frames are not re-derived here; they
/// come from `riscv64::frame_layout` — the product's own function, the same
/// call `compile_framed` already made for the window, so the two can never
/// disagree about a routine. The offsets are read out of the EMITTED TEXT. The
/// two sides are opposite ends of the emitter: what it planned, and what it
/// wrote. An off-by-one between `frame_layout`'s regions and `emit_module`'s
/// accesses lands in `Unaccounted` and nowhere else.
///
/// **THE BANDS ARE ASSERTED PRESENT, NOT JUST THE FAILURE ASSERTED ABSENT.** A
/// band that silently falls to zero means the sweep stopped seeing the thing it
/// classifies, and `unaccounted == 0` alone goes green on an empty corpus —
/// which is exactly how the first version passed. So the populated bands are
/// named below and required to stay populated.
#[test]
fn every_offset_the_corpus_addresses_off_sp_falls_in_a_region_its_frame_declares() {
    use emitted::{SP, SlotBand, slot_band};

    let mut census: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut unaccounted: Vec<String> = Vec::new();
    let mut routines_without_a_frame: Vec<String> = Vec::new();
    let mut modules = 0usize;
    let mut accesses = 0usize;
    let mut deepest_spill = 0usize;
    let mut window_disagreements: Vec<String> = Vec::new();

    for p in corpus_paths() {
        let file = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let src = std::fs::read_to_string(&p).expect("readable");
        if module_name(&src).is_none() {
            continue;
        }
        let Framed::Emitted(text, windows, frames) = compile_framed(&src) else {
            // A source that does not emit is not this test's subject; the
            // sweeps above already report refusals with their reasons.
            continue;
        };
        modules += 1;

        // The routine a line sits in is the last `॥ वैश्विकम् X ॥` above it —
        // the same boundary `conditional_operands` uses, and for the same
        // reason: without the reset an offset from routine A is classified
        // against routine Z's frame, which is a WRONG BAND reported with the
        // confidence of a right one.
        let mut routine = String::new();
        for l in text.lines() {
            let f: Vec<&str> = l.split_whitespace().collect();
            if let ["॥", "वैश्विकम्", name, "॥"] = f.as_slice() {
                routine = (*name).to_string();
                continue;
            }
            // Load and store share this shape; the base is the third field for
            // both, so neither mnemonic is named here and a third that
            // addresses `स्तूपसूचकः` the same way is picked up without a change.
            let [_mnemonic, _reg, base, off, "।"] = f.as_slice() else {
                continue;
            };
            if *base != format!("{SP}त्") {
                continue;
            }
            let Some(off) = off.strip_suffix('न').and_then(emitted::devanagari_int) else {
                continue;
            };
            accesses += 1;
            let Some(frame) = frames.get(&routine) else {
                // NAMED, not skipped: an access attributed to a routine the
                // frame map has never heard of means the two readers disagree
                // about where routines begin, and skipping it would hide that
                // behind a smaller census.
                routines_without_a_frame.push(format!("{file}: offset {off} in `{routine}`"));
                continue;
            };
            deepest_spill = deepest_spill.max(frame.num_spills);
            let band = slot_band(off, frame);
            *census.entry(band.class()).or_default() += 1;
            if band == SlotBand::Unaccounted {
                unaccounted.push(format!(
                    "{file}: offset {off} in `{routine}` — frame is {} octets, \
                     {} spill + {} local slots, पुनःस्थानम् at {}, saved {:?}",
                    frame.bytes, frame.num_spills, frame.num_locals, frame.ra_offset, frame.saved
                ));
            }

            // **THE TWO READERS MUST PARTITION THE SAME WAY, AND THIS IS WHERE
            // THEY ARE MADE TO SAY SO.** `slot_band` answering `Local` and the
            // window `(lo, hi)` containing the offset are two independent
            // statements about one thing: the first is derived from
            // `num_spills`/`num_locals`, the second from `8·num_spills` and
            // `8·(num_spills + num_locals)`. They agree today because they are
            // computed from the same `Frame` — and that is exactly why a
            // divergence would be a real defect rather than a rounding
            // difference. Nine call sites above trust the window; if the
            // classifier ever disagrees with it, every one of them is reading a
            // different frame than this census reports and NEITHER side would
            // red on its own.
            if let Some((lo, hi)) = windows.get(&routine) {
                let in_window = (*lo..*hi).contains(&off);
                let is_local = band.local().is_some();
                if in_window != is_local {
                    window_disagreements.push(format!(
                        "{file}: offset {off} in `{routine}` — window [{lo}, {hi}) says \
                         {}, slot_band says {}",
                        if in_window { "local" } else { "NOT local" },
                        band.class()
                    ));
                }
            }
        }
    }

    println!(
        "modules {modules}, sp accesses {accesses}, deepest num_spills \
         {deepest_spill}, bands {census:?}"
    );

    // THE CENSUS AS METRICS, so a later cycle can read the bands without
    // re-running a nine-minute sweep, and so a band moving shows up in the
    // ledger beside every other corpus figure rather than only in this file.
    for (k, v) in &census {
        println!("METRIC t1_corpus_sp_band_{k} {v}");
    }
    println!("METRIC t1_corpus_sp_accesses {accesses}");
    println!("METRIC t1_corpus_sp_deepest_num_spills {deepest_spill}");

    assert!(
        modules > 0 && accesses > 0,
        "the sweep classified {accesses} accesses over {modules} modules. A \
         census of nothing satisfies every claim below, so this is asserted \
         first: `corpus_paths` or `compile_framed` stopped reaching the corpus"
    );
    assert!(
        routines_without_a_frame.is_empty(),
        "{} SP accesses sit in a routine `compile_framed` laid out no frame \
         for. The routine is taken from the last `॥ वैश्विकम् X ॥` and the frame \
         map's keys come from `riscv64::routine_label`; if those two disagree \
         every band in this test is assigned against the wrong frame:\n  {}",
        routines_without_a_frame.len(),
        routines_without_a_frame.join("\n  ")
    );
    assert!(
        window_disagreements.is_empty(),
        "{} offsets where the local WINDOW and `slot_band` disagree about \
         whether the offset is a local. Both are computed from the same \
         `Frame`, so they cannot differ by rounding — a difference means one of \
         the two readings of that frame is wrong, and the nine call sites above \
         that trust the window would all be affected without any of them going \
         red:\n  {}",
        window_disagreements.len(),
        window_disagreements.join("\n  ")
    );
    assert!(
        unaccounted.is_empty(),
        "{} offsets fall in NO region their routine's frame declares. The frame \
         came from `riscv64::frame_layout` and the offset from the emitted \
         text, so this is the emitter's plan disagreeing with the emitter's \
         output — not a reader preference:\n  {}",
        unaccounted.len(),
        unaccounted.join("\n  ")
    );

    // THE BANDS THIS SWEEP MUST KEEP SEEING. A band dropping to zero is the
    // sweep losing its reach, which is the failure that made the first version
    // of this test vacuous.
    // **HOW DEEP THE SPILL REGION IS EVER PUSHED, PINNED SEPARATELY FROM
    // WHETHER IT IS PUSHED AT ALL.** A probe on 2026-09-25 found 17 of 871
    // routines spilling, every one of them a `<module>खण्डवृद्धिः` — the run
    // growth helper, once per module — and NONE with more than a single spill
    // slot. So `8·(num_spills + k)` is exercised, and exercised only at
    // `num_spills` ∈ {0, 1}: a displacement defect that first appears at TWO
    // spill slots is invisible to this whole corpus, and the `spill` band being
    // populated does not say otherwise.
    //
    // This is asserted as an EQUALITY, not a floor. Rising is not a
    // regression — it is the corpus reaching a depth it has never reached, and
    // the fixture that does it on purpose is the thing to write next. Reds
    // either way so that neither direction passes unnoticed.
    assert_eq!(
        deepest_spill, 1,
        "the deepest frame in the corpus uses {deepest_spill} spill slots, not \
         1. If this ROSE, some routine now needs a deeper spill region and the \
         `8·(num_spills + k)` displacement is being exercised past where it has \
         ever been checked — read the new depth as new coverage and raise this \
         pin. If it FELL to 0, the allocator stopped spilling anywhere and the \
         `spill` band below is about to empty"
    );

    for band in POPULATED_BANDS {
        assert!(
            census.get(band).copied().unwrap_or(0) > 0,
            "band `{band}` is EMPTY. It was populated when this test was \
             written, so the sweep has stopped reaching it — that is a loss of \
             coverage, and `unaccounted == 0` above is correspondingly weaker. \
             Census: {census:?}"
        );
    }
}

/// The bands the corpus actually exercises, MEASURED 2026-09-25 over 20
/// modules and 17,249 SP accesses:
///
/// ```text
///   local              12887
///   saved               3456
///   return-address       871
///   spill                 34
///   incoming-argument      1
///   unaccounted            0
/// ```
///
/// **`unaccounted` is absent from this list on purpose** — it is the band the
/// test asserts EMPTY, and listing it here would demand the opposite. Every
/// other band is present, so `slot_band`'s six arms are exercised by five and
/// refuted by one.
///
/// **THE THIN ONES ARE THE POINT OF PINNING THEM.** `spill` is 34 of 17,249
/// (0.2%) and `incoming-argument` is ONE — a single routine in the whole corpus
/// reads its ९th argument out of the caller's frame. Either could vanish
/// through an ordinary change to the allocator or to one source, and a census
/// that did not pin them would lose the only coverage those two arms have
/// without anything going red. That is precisely how the first version of this
/// test — over `conditional_operands`, which answered `{"local": 3363}` —
/// managed to pass while exercising one arm in six.
const POPULATED_BANDS: &[&str] = &[
    "local",
    "saved",
    "return-address",
    "spill",
    "incoming-argument",
];
