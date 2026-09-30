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

use emitted::{ConditionalOperands, Origin, SP, conditional_operands, devanagari, devanagari_int};
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

/// **THE MUTATION THE PER-ROUTINE CUT CLAIM EXISTS TO CATCH, AND THE ONE
/// NEITHER THE RESIDUE NOR THE ORDER CENSUS CAN SEE.**
///
/// `अनामस्थानम्` (`ir.t1:1345`) guards on `अनामस्थलारम्भः समम् ०` and so cuts
/// the pool ONCE per routine, at one rank, for the whole of the lowering. Drop
/// that guard and the pool is cut AGAIN part-way through a routine: every read
/// after the second cut is eight slots further on and every read before it is
/// where it was. Same mnemonics, same instruction count, same object length,
/// same branch graph.
///
/// **AND IT IS INVISIBLE TO BOTH WEAKER CLAIMS, which is the whole reason the
/// per-routine claim exists.** Every site still satisfies
/// `slot ≡ rank (mod ८)` — eight is exactly the shift — so [`residue_shortfalls`]
/// reads it green; and both slots of a site move together, so the site keeps its
/// direction and [`directions_of`] reads it green too. Only ONE base explaining
/// EVERY site at once refuses it.
///
/// Shifts the spill loads feeding the `न्यूनलङ्घनम्` sites from the `from`th
/// (0-based, per object) onward, and answers how many load lines it moved — a
/// mutation that moved NONE would be a no-op whose quiet says nothing, so the
/// count is returned rather than assumed. `from = ०` moves every site of the
/// routine and is the CONTROL: that is one cut in a different place, and the
/// exhaustion must recover it rather than report it.
fn cut_the_pool_twice(text: &str, from: usize) -> (String, usize) {
    // The most recent spill load of each register, by line — the same
    // `आहारः <r>म् स्तूपसूचकःत् <off>न ।` shape `conditional_operands` reads a
    // slot off, and any other write to the register clears it, or a load from
    // an earlier block would be shifted for a branch it no longer feeds.
    let mut last_load: Vec<(String, usize)> = Vec::new();
    let mut shift: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut seen = 0usize;
    for (i, l) in text.lines().enumerate() {
        let f: Vec<&str> = l.split_whitespace().collect();
        if let ["॥", "वैश्विकम्", _, "॥"] = f.as_slice() {
            last_load.clear();
        }
        if let ["न्यूनलङ्घनम्", a, b, _, "।"] = f.as_slice() {
            if seen >= from {
                for (tok, suffix) in [(a, "न"), (b, "त्")] {
                    if let Some(at) = tok
                        .strip_suffix(suffix)
                        .and_then(|reg| last_load.iter().find(|(r, _)| r == reg))
                        .map(|(_, at)| *at)
                    {
                        shift.insert(at);
                    }
                }
            }
            seen += 1;
            continue;
        }
        let load = match f.as_slice() {
            ["आहारः", dst, base, off, "।"] if *base == format!("{SP}त्") => dst
                .strip_suffix("म्")
                .filter(|_| off.strip_suffix('न').and_then(devanagari_int).is_some()),
            _ => None,
        };
        if let Some(reg) = load {
            last_load.retain(|(r, _)| r != reg);
            last_load.insert(0, (reg.to_string(), i));
            continue;
        }
        if let Some(dst) = f.get(1).and_then(|d| d.strip_suffix("म्")) {
            last_load.retain(|(r, _)| r != dst);
        }
    }
    let out = text
        .lines()
        .enumerate()
        .map(|(i, l)| {
            if !shift.contains(&i) {
                return l.to_string();
            }
            let f: Vec<&str> = l.split_whitespace().collect();
            let off = f[3]
                .strip_suffix('न')
                .and_then(devanagari_int)
                .expect("the line was chosen by that very reading");
            format!(
                "{} {} {} {}न ।",
                f[0],
                f[1],
                f[2],
                devanagari(off + 8 * POOL_SLOTS as u64)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    (out, shift.len())
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

/// **WHAT A HALF-SITE STILL SAYS WHEN THE ROUTINE'S CUT IS NOT RECOVERED — THE
/// HALF RESIDUE.**
///
/// `(slot mod ८, constant, side)`. `अनामस्थानम्` (`ir.t1:1345`) interrupts the
/// declaration run ONCE and by EXACTLY `अनामस्थलसंख्या = ८`, so a name of rank
/// `r` holds slot `r` or slot `r + ८` and in both cases
///
/// > `slot ≡ rank (mod ८)`,
///
/// while the constant and the written side are the object's own and do not move
/// with the cut AT ALL. So the whole triple is predictable WITHOUT recovering
/// where the cut fell — which is what lets it reach the routines the third
/// widening leaves AMBIGUOUS, and they are most of them.
///
/// **AND WHY THERE IS NO UNINFORMATIVE CASE HERE, unlike [`residues`].** A
/// residue PAIR loses its order when its two ranks are congruent modulo eight
/// — `(२, १०)` and `(१०, २)` are one bucket either way round — which is why
/// [`residues`] drops those sites rather than count them as covered. A
/// half-site has no order to lose: what it claims BESIDES the residue is the
/// SIDE, one bit the cut cannot touch, so every half-site keeps a claim the
/// operand exchange can break and none of them is dropped.
fn half_residues(hs: &[Half]) -> Vec<Half> {
    hs.iter()
        .map(|(s, v, k)| (s % POOL_SLOTS, *v, *k))
        .collect()
}

/// Every predicted `(slot mod ८, constant, side)` triple the object does not
/// carry as often as the source demands it — over the routines `ambiguous`
/// names and no others.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
/// * **A ROUTINE WHOSE CUT *IS* RECOVERED STAYS OUT, and `ambiguous` is how.**
///   Its single base was recovered FROM these very half-sites
///   ([`cut_halves`] is what narrowed it), so asserting them again here would
///   be circular: the base was chosen BECAUSE it explains them, and a claim
///   that cannot fail is not a claim. Only a routine whose candidate set is not
///   a singleton has a residue claim nothing else in this file makes.
/// * A routine [`pairable`] accepts stays out for the stronger reason: it has
///   no anonymous pool at all, so rank IS slot and [`half_shortfalls`] already
///   asks the EXACT triple of it. The residue would be that claim weakened.
/// * Two ranks eight apart against the same constant on the same side are ONE
///   residue bucket, so this is a FLOOR per DISTINCT residue triple and never a
///   total — as everywhere in this file, `यावत्` rotation emits one source
///   comparison twice and the object may carry MORE than the source demands.
/// * A constant outside the `addi` immediate never entered `half_want` at all
///   (see [`IMMEDIATE`]): it stays named in `half_out_of_range` and cannot come
///   out residue-red here, however empty the object is.
fn half_residue_shortfalls(
    ambiguous: &std::collections::BTreeSet<String>,
    half_want: &BTreeMap<String, ByWordHalf>,
    half_have: &BTreeMap<&'static str, HalfPairs>,
) -> Vec<String> {
    const NONE: &[Half] = &[];
    let mut out = Vec::new();
    for (r, by_word) in half_want {
        if !ambiguous.contains(r) {
            continue;
        }
        for (word, hs) in by_word {
            let mut wanted: BTreeMap<Half, usize> = BTreeMap::new();
            for h in half_residues(hs) {
                *wanted.entry(h).or_default() += 1;
            }
            let carried = half_residues(
                half_have
                    .get(word)
                    .and_then(|p| p.get(r))
                    .map_or(NONE, Vec::as_slice),
            );
            for (h, n) in wanted {
                let got = carried.iter().filter(|q| **q == h).count();
                if got < n {
                    out.push(format!(
                        "{r}: the source wants {n} `{word}` comparing a local \
                         slot congruent to {} modulo eight against the constant \
                         {}, the name in the {}; the object carries {got} (it \
                         carries {carried:?})",
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

/// The ambiguous routines' residue claim in BOTH of its currencies, because
/// they are not the same number and this ledger compared them.
///
/// **THE HOLE THE TOTAL LOOKED LIKE IT WAS HIDING IS A CURRENCY ERROR,
/// MEASURED.** Cycle 1005 read `112 sites, the swap bites 105` and concluded
/// that seven sites had an exchanged form the object carries anyway. They do
/// not. [`half_residue_shortfalls`] reports ONE line per DISTINCT residue
/// bucket, never one per site, and over these 74 routines 112 sites collapse
/// into exactly 105 buckets: seven of them are a second site landing in a
/// bucket a first site already opened — two ranks EIGHT APART against the same
/// constant on the same side, which is the floor case
/// `an_ambiguous_half_routines_residue_holds_and_a_recovered_one_stays_out`
/// exercises. So the swap bites every bucket there is, the residue claim
/// reaches all 112 sites, and `105` was never a count of sites to subtract
/// from `112`. Both numbers are reported here, with the identity between them
/// asserted, so the two cannot be set against each other again.
///
/// **AND THE SHAPE THAT WOULD HAVE BEEN A REAL HOLE IS STILL LOOKED FOR.** A
/// want whose twin on the OTHER side is written at least as often under the
/// SAME word IS uninformative: exchanging the operands of that word rewrites
/// each want into the other's, the multiset does not move, and no mutation in
/// this file can bite the site — the shape `blind` and `half_exact_blind`
/// refuse one altitude up. `cancelled` names those and takes their sites OUT
/// of the covered figure rather than out of the claim. Today the corpus has
/// NONE, which is asserted rather than assumed: a bucket is one `(slot mod ८,
/// constant, side)` triple and the corpus writes no routine that compares one
/// residue bucket against one constant from both sides under one word.
///
/// **THE CASES THAT MUST STILL BE REFUSED BY `cancelled`.**
/// * A twin under a DIFFERENT word stays covered. The match is word-for-word
///   everywhere in this file, and [`swap_the_half_sides`] exchanges each word's
///   operands in place and never moves a site between words, so `क समम् ५`
///   and `५ असमम् क` do not cancel and the swap does bite them.
/// * A twin in a DIFFERENT routine stays covered, for the same reason: halves
///   are read per routine label and no exchange carries a site across a frame.
/// * A twin wanted FEWER times than this bucket stays covered. Two `करण` wants
///   against one `अपादान` want leaves the swapped object asked for two where
///   the source demands one, so the swap can still bite it and calling it
///   cancelled would take a live site out of the figure.
#[derive(Default, Debug)]
struct ResidueBuckets {
    /// Every DISTINCT residue bucket over the ambiguous routines. This is the
    /// currency [`half_residue_shortfalls`] counts in, and the only one the
    /// bite arithmetic can use.
    buckets: usize,
    /// The sites that landed in a bucket another site had already opened —
    /// `sites - buckets`, counted here rather than subtracted there.
    shared: usize,
    /// One line per shared bucket: which routine, which word, how many sites.
    shared_named: Vec<String>,
    /// The SITES of the buckets the exchange cannot move — what comes out of
    /// the covered figure.
    cancelled: usize,
    /// One line per cancelled bucket.
    cancelled_named: Vec<String>,
}

/// Read one module's ambiguous half-wants into [`ResidueBuckets`].
fn half_residue_buckets_of(
    ambiguous: &std::collections::BTreeSet<String>,
    half_want: &BTreeMap<String, ByWordHalf>,
) -> ResidueBuckets {
    let mut out = ResidueBuckets::default();
    for (r, by_word) in half_want {
        if !ambiguous.contains(r) {
            continue;
        }
        for (word, hs) in by_word {
            let mut wanted: BTreeMap<Half, usize> = BTreeMap::new();
            for h in half_residues(hs) {
                *wanted.entry(h).or_default() += 1;
            }
            out.buckets += wanted.len();
            for (h, n) in &wanted {
                let side = if h.2 {
                    "करण"
                } else {
                    "अपादान"
                };
                if *n > 1 {
                    out.shared += n - 1;
                    out.shared_named.push(format!(
                        "{r}: `{word}` writes {n} site(s) into the ONE residue \
                         bucket (slot ≡ {} mod ८, {}, {side}) — ranks eight \
                         apart are one bucket, so the claim is a FLOOR of {n} \
                         and the shortfall reader reports it ONCE; {} of these \
                         sites are carried by that one line and are not a \
                         second claim",
                        h.0,
                        h.1,
                        n - 1
                    ));
                }
                let twin = wanted.get(&(h.0, h.1, !h.2)).copied().unwrap_or(0);
                if twin >= *n {
                    out.cancelled += n;
                    out.cancelled_named.push(format!(
                        "{r}: `{word}` wants {n} comparison(s) of a local slot \
                         congruent to {} modulo eight against the constant {} \
                         with the name in the {side}, and {twin} of the SAME \
                         residue bucket on the OTHER side — exchanging this \
                         word's operands rewrites each want into the other's \
                         and the multiset does not move, so the side bit \
                         carries no claim here and these {n} site(s) are \
                         UNINFORMATIVE, not covered",
                        h.0, h.1
                    ));
                }
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

/// **THE POOL'S POSITION, IN THE THREE STATES [`consistent_bases_of`] ANSWERS
/// — AND THE ARM THAT HAD NEVER FIRED.**
///
/// The sweep read `bases.len()` and branched on `0` / `1` / `n` inline. Three
/// arms, so nothing was being hidden the way a boolean hid [`FoldVerdict`]'s
/// third state — but an inline `match` on a length cannot be handed an object,
/// so `cut_none`'s arm was a claim nobody had seen work: its corpus zero said
/// only that no fixture had ever reached it. Named here, it can be asked of a
/// compiled object, which is what
/// [`a_second_cut_part_way_through_a_routine_leaves_no_base_where_the_residue_reads_green`]
/// does.
///
/// **AND THE EMPTY SET AND THE SET OF TWO STAY APART.** An empty set explains
/// NOTHING — a second cut, a pool of another width, a declaration the harvester
/// ranks wrongly, or the dropped exchange — and a set of two explains too much;
/// collapsing them would let the first read as the second and put a RED in a
/// bucket the sweep only counts.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CutVerdict {
    /// NO candidate base satisfies every site of the routine at once. The
    /// object contradicts itself: `cut_none`, and a RED.
    Unexplained,
    /// EXACTLY ONE base does, so the pool's position is recovered FROM THE
    /// OBJECT by exhaustion and the exact `(करण, अपादान)` pair is reported.
    Recovered(usize),
    /// SEVERAL do. The existence claim stands and the position does NOT: it is
    /// named, never chosen, because naming the base that agrees is fitting the
    /// answer to the data.
    Ambiguous(Vec<usize>),
}

/// [`CutVerdict`]'s one decision. `bases` is what [`consistent_bases_of`]
/// answered — ascending and duplicate-free, a filtered `0..=names` — so the
/// slice patterns are exhaustive and `Recovered` carries the base itself rather
/// than the fact that there was one.
fn cut_verdict(bases: &[usize]) -> CutVerdict {
    match bases {
        [] => CutVerdict::Unexplained,
        [b] => CutVerdict::Recovered(*b),
        many => CutVerdict::Ambiguous(many.to_vec()),
    }
}

/// **WHAT FOLDING THE OTHER `compare_op`s DID TO THE CANDIDATE SET — IN THREE
/// STATES, BECAUSE THE TRUTH HAS THREE AND A BOOLEAN HID ONE.**
///
/// The sweep asked TWO questions of the fold a few lines apart: "did it add a
/// base" (`not_monotone`) and "did it get smaller" (`cut_narrowed`). Those two
/// booleans have four combinations and only three are legal, so a MISWIRED
/// fold that added a base while happening to leave FEWER of them was counted
/// as a narrowing and credited to the constants at the same time as it was
/// reported. One decision now, and the three arms are exclusive.
///
/// **AND THE LAW IS CONTAINMENT, NOT CARDINALITY.** A base must satisfy the
/// groups it already satisfied and one more, so the folded set is a SUBSET of
/// the `न्यूनलङ्घनम्` one. A size check is NOT the same test and would read a
/// crossed grouping green whenever it happened to leave a shorter list —
/// proved on a built object in
/// [`a_fold_that_crosses_mnemonics_adds_a_candidate_and_is_reported_not_monotone`],
/// where one crossing leaves TWO candidates against the honest reading's NINE
/// and every one of the two is a base the honest reading REFUSED.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FoldVerdict {
    /// Every base the fold kept, the `न्यूनलङ्घनम्` reading kept, and it kept
    /// FEWER: the other mnemonics' sites are evidence and they narrowed the
    /// cut. The only arm `cut_narrowed` comes off.
    Narrowed,
    /// The same set. The other mnemonics' sites are consistent with exactly
    /// the bases `न्यूनलङ्घनम्` alone left, so the fold added no evidence —
    /// QUIET and not a narrowing, or the counter would degrade to "the fold
    /// did something".
    Unchanged,
    /// At least one base the `न्यूनलङ्घनम्` reading REFUSED survived the fold,
    /// which no extra constraint can do. The grouping is wired wrongly — one
    /// mnemonic's predicted ranks matched against another mnemonic's carried
    /// pairs — and every recovery the fold reports is invented rather than
    /// witnessed. Carries the offending bases so the report names them.
    Added(Vec<usize>),
}

/// [`FoldVerdict`]'s one decision. `lt_only` and `folded` are both ascending
/// and duplicate-free (`consistent_bases_of` filters a range), so equal length
/// under containment IS set equality.
fn fold_verdict(lt_only: &[usize], folded: &[usize]) -> FoldVerdict {
    let added: Vec<usize> = folded
        .iter()
        .copied()
        .filter(|b| !lt_only.contains(b))
        .collect();
    if !added.is_empty() {
        FoldVerdict::Added(added)
    } else if folded.len() < lt_only.len() {
        FoldVerdict::Narrowed
    } else {
        FoldVerdict::Unchanged
    }
}

/// **HOW FAR A MUTATION REACHED PAST THE ONE WORD IT CLAIMS — IN FOUR STATES,
/// BECAUSE THE TWO SIDES ARE INDEPENDENT AND MUST BE TOLD APART.**
///
/// [`drop_the_exchange`] rewrites `न्यूनलङ्घनम्` lines and nothing else, so the
/// other three branch words must come out of the bitten object IDENTICAL — both
/// their two-slot pairs ([`emitted_pairs_of`]) and their NAME-AGAINST-CONSTANT
/// half-sites ([`emitted_halves_of`]). Every sensitivity figure the sweep
/// reports rests on it, and the sweep asked it as TWO booleans a line apart
/// pushing two unrelated sentences into one bucket.
///
/// **THE TWO QUESTIONS ARE GENUINELY SEPARATE, which is why one bucket cannot
/// carry both unlabelled.** The pair reader DROPS every site with a constant
/// operand and the half reader drops every two-slot site, so a mutation can move
/// either side ALONE — proved both ways round on compiled objects in
/// [`a_mutation_that_reaches_past_the_exchange_is_reported_and_the_two_sides_are_told_apart`].
/// A half-site bite reported under the pair sentence would make one zero stand
/// for two, so each arm names the WORDS that moved under it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ReachVerdict {
    /// Nothing outside `न्यूनलङ्घनम्` moved: the reach the mutation claims, and
    /// the only quiet arm.
    Contained,
    /// The other words' two-slot pairs differ, under these words.
    Pairs(Vec<&'static str>),
    /// Their half-sites differ, under these words — a bite the pair reader is
    /// blind to by construction.
    Halves(Vec<&'static str>),
    /// Both sides moved, and each names its own words.
    Both {
        pairs: Vec<&'static str>,
        halves: Vec<&'static str>,
    },
}

impl ReachVerdict {
    /// The sentence the sweep reports, or `None` for [`ReachVerdict::Contained`].
    fn complaint(&self) -> Option<String> {
        let words = |w: &[&str]| w.join(", ");
        match self {
            ReachVerdict::Contained => None,
            ReachVerdict::Pairs(p) => Some(format!(
                "changed a branch that is not `न्यूनलङ्घनम्` — the TWO-SLOT pairs \
                 of {} differ between the honest object and the bitten one",
                words(p)
            )),
            ReachVerdict::Halves(h) => Some(format!(
                "changed a NAME-AGAINST-CONSTANT branch that is not \
                 `न्यूनलङ्घनम्` — the HALF-SITES of {} differ between the honest \
                 object and the bitten one",
                words(h)
            )),
            ReachVerdict::Both { pairs, halves } => Some(format!(
                "reached past `न्यूनलङ्घनम्` on BOTH sides — the TWO-SLOT pairs of \
                 {} and the HALF-SITES of {} differ between the honest object \
                 and the bitten one",
                words(pairs),
                words(halves)
            )),
        }
    }
}

/// The words of [`EXTRA_WORDS`] one reading answers differently for the honest
/// object and the bitten one. Read off the SAME maps the sweep folds in below,
/// never a second walk of the text that could drift from them.
fn moved_words<V: PartialEq>(
    honest: &BTreeMap<&'static str, V>,
    bitten: &BTreeMap<&'static str, V>,
) -> Vec<&'static str> {
    EXTRA_WORDS
        .into_iter()
        .filter(|w| honest.get(w) != bitten.get(w))
        .collect()
}

/// [`ReachVerdict`]'s one decision.
fn reach_verdict(
    honest_pairs: &BTreeMap<&'static str, Pairs>,
    honest_halves: &BTreeMap<&'static str, HalfPairs>,
    bitten_pairs: &BTreeMap<&'static str, Pairs>,
    bitten_halves: &BTreeMap<&'static str, HalfPairs>,
) -> ReachVerdict {
    let pairs = moved_words(honest_pairs, bitten_pairs);
    let halves = moved_words(honest_halves, bitten_halves);
    match (pairs.is_empty(), halves.is_empty()) {
        (true, true) => ReachVerdict::Contained,
        (false, true) => ReachVerdict::Pairs(pairs),
        (true, false) => ReachVerdict::Halves(halves),
        (false, false) => ReachVerdict::Both { pairs, halves },
    }
}

/// **THE MISSING BASES INSIDE A CANDIDATE SET'S OWN RANGE — AND WHY A
/// HALF-SITE-ONLY ROUTINE CANNOT HAVE ONE.**
///
/// [`consistent_bases_of`] answers ascending, so `min..=max` is the run the
/// set spans and anything absent from it is a GAP: a cut position the object
/// refuses that sits BETWEEN two it accepts.
///
/// **FOR A ROUTINE WHOSE EVIDENCE IS HALF-SITES ALONE THAT IS IMPOSSIBLE, and
/// the proof is that the constraints DECOMPOSE.** [`slot_at`] sends rank `r`
/// to slot `r` when `b > r` and to `r + ८` otherwise, so two DISTINCT ranks
/// never name one slot — `r₁ = r₂ + ८` needs `r₁ > r₂` and `r₁ ≥ b > r₂`
/// at once, and `r₁ + ८ = r₂` needs `r₂ > r₁` with the sides the other way
/// round. So the wanted multiset never collides across ranks, the containment
/// check is one independent question per `(rank, constant, side)` group, and
/// each of those accepts exactly `{b : b > r}`, `{b : b ≤ r}`, all bases or
/// none — four INTERVALS. An intersection of intervals is an interval.
///
/// **AND A PAIR IS NOT, which is why the pair-folded routines are EXEMPT and
/// counted apart.** A pair `(x, y)` with `x < y` has THREE configurations over
/// `b` — both above the cut, `x` below and `y` above, both below — each an
/// interval, each accepted or refused on its own. An object carrying the first
/// and the third and not the middle one leaves a genuine gap, and demanding
/// contiguity of it would red the compiler for being right. See
/// `a_half_routines_candidate_cut_set_is_one_run_and_a_pair_fold_is_exempt`,
/// which builds exactly that set out of this very exhaustion.
///
/// **WHAT A GAP WOULD MEAN HERE.** Not a second cut and not a misranked
/// declaration — those show up EMPTY, which is `half_wide_none`'s business.
/// A gap says the half-only set was narrowed by something that is not a
/// half-site: a pair group wired into [`cut_groups`] where this sweep passes
/// `NO_LT_SITES`, or [`slot_at`] no longer monotone in the base. Both are
/// instrument faults that leave every figure above looking exactly right.
fn base_run_gaps(bases: &[usize]) -> Vec<usize> {
    let (Some(lo), Some(hi)) = (bases.first(), bases.last()) else {
        return Vec::new();
    };
    (*lo..=*hi).filter(|b| !bases.contains(b)).collect()
}

/// **THE EXACT-SLOT CLAIM OVER THE ROUTINES WHOSE CUT THE HALF-SITES
/// THEMSELVES RECOVERED — THE 22 THE THIRD WIDENING STOPPED AT.**
///
/// A half-site-only routine with exactly ONE consistent base knows where its
/// anonymous pool was cut, so [`slot_at`] turns every one of its
/// `(rank, constant, side)` wants into an exact `(slot, constant, side)` the
/// object must carry — the same claim [`half_shortfalls`] makes of a
/// `pairable` routine, but over a cut RECOVERED rather than absent. Until this
/// cycle those routines left `half_bases` with a singleton and entered NO
/// shortfall reader at all: [`half_residue_shortfalls`] excludes them by design
/// and [`half_recovered_shortfalls`] is asked of nothing else.
///
/// **AND IT IS NOT CIRCULAR, THOUGH IT LOOKS IT.** The base was chosen by
/// [`consistent_bases_of`] to satisfy these very wants, so on an HONEST object
/// this reader is green by construction — which is exactly the standing of
/// [`base_run_gaps`], and for the same reason it is worth asking. A red can
/// only come from a want the recovery never saw: a base handed in from a
/// recovery that stopped checking its half-groups, a [`slot_at`] that no longer
/// agrees with the one the exhaustion used, or a `half_have` read from a
/// DIFFERENT object than the one the base was recovered from. That last is not
/// hypothetical — it is how the claim is falsified below, by asking it of the
/// swapped object while the base stays the honest one, and it is the only
/// reading in this file under which a recovered routine can go short.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
/// * **A ROUTINE WHOSE BASE WAS NARROWED BY A PAIR GROUP MUST STAY OUT.** It is
///   not in `half_bases` at all — the sweep skips every routine `want_paired`,
///   `want_residue` or `wide_bases` claims — so it never reaches the map this
///   reader is handed, and the reader claims NOTHING it is not given. A base
///   resting on two-name evidence would make this a restatement of the pair
///   recovery rather than a claim the half-sites carry on their own.
/// * **THE CLAIM IS ASKED OF THE RECOVERED BASE ALONE.** Asking it of every
///   candidate of an AMBIGUOUS routine — "some base explains the wants" — is
///   the existential [`consistent_bases_of`] already answered, weaker than the
///   residue claim, and green on the routines that carry the most sites. So the
///   map holds one base per routine and an ambiguous routine has no entry.
/// * **A ROUTINE IN THE MAP WITH NO WANTS IS SKIPPED, NOT COUNTED.** The map is
///   built from `half_bases`, whose keys all come from `half_want`, so a
///   missing entry is an instrument fault and must not be read as a green.
fn half_recovered_shortfalls(
    recovered: &BTreeMap<String, usize>,
    half_want: &BTreeMap<String, ByWordHalf>,
    half_have: &BTreeMap<&'static str, HalfPairs>,
) -> Vec<String> {
    const NONE: &[Half] = &[];
    let mut out = Vec::new();
    for (r, base) in recovered {
        let Some(by_word) = half_want.get(r) else {
            continue;
        };
        for (word, hs) in by_word {
            let mut wanted: BTreeMap<Half, usize> = BTreeMap::new();
            for (rank, v, karana) in hs {
                *wanted
                    .entry((slot_at(*rank, *base), *v, *karana))
                    .or_default() += 1;
            }
            let carried = half_have
                .get(word)
                .and_then(|p| p.get(r))
                .map_or(NONE, Vec::as_slice);
            for (h, n) in wanted {
                let got = carried.iter().filter(|q| **q == h).count();
                if got < n {
                    out.push(format!(
                        "{r}: its cut is recovered at base {base}, so the \
                         source wants {n} `{word}` comparing local slot {} \
                         against the constant {}, the name in the {}; the \
                         object carries {got} (it carries {carried:?})",
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

/// The recovered routines' exact-slot claim counted in the currency
/// [`half_recovered_shortfalls`] reports in — one line per DISTINCT
/// `(slot, constant, side)` bucket per word — so the bite can be measured
/// against it and not against the SITES, which is the error cycle 1005 made
/// about the residue.
///
/// **AND THE SHARING HERE IS A NARROWER PHENOMENON THAN THE RESIDUE'S, which
/// is why this is not [`ResidueBuckets`] under another name.** Two ranks eight
/// apart share ONE residue bucket; they never share an exact slot, because
/// [`slot_at`] sends distinct ranks to distinct slots under every base (the
/// decomposition [`base_run_gaps`] rests on). So `shared` here can only be the
/// SAME rank against the SAME constant on the SAME side written twice —
/// `यावत्` rotation emitting one source comparison into two branches — and the
/// claim is a FLOOR of that count reported ONCE.
#[derive(Default, Debug)]
struct ExactBuckets {
    /// Every half-SITE the wants carry over the routines read — the GROSS
    /// figure `cancelled` comes out of.
    ///
    /// **COUNTED HERE, and that is the point of the field.** Both readers of
    /// this struct used to sum their sites in a second traversal beside the
    /// one that filled `cancelled`, so the covered figure and the cancellation
    /// were two readings in two currencies a hundred lines apart — the error
    /// cycle 1005 made about the residue. One traversal answers both, and the
    /// recovered reader ASSERTS its own second sum against this one rather
    /// than trusting that they agree.
    sites: usize,
    /// Every DISTINCT `(slot, constant, side)` bucket over the recovered
    /// routines.
    buckets: usize,
    /// Sites that landed in a bucket another site had already opened.
    shared: usize,
    /// One line per shared bucket.
    shared_named: Vec<String>,
    /// The SITES of the buckets [`swap_the_half_sides`] cannot move.
    cancelled: usize,
    /// One line per cancelled bucket — what comes out of the covered figure.
    cancelled_named: Vec<String>,
}

/// Read one module's RECOVERED half-wants into [`ExactBuckets`], at the exact
/// slots their recovered bases imply.
fn half_recovered_buckets_of(
    recovered: &BTreeMap<String, usize>,
    half_want: &BTreeMap<String, ByWordHalf>,
) -> ExactBuckets {
    let mut out = ExactBuckets::default();
    for (r, base) in recovered {
        let Some(by_word) = half_want.get(r) else {
            continue;
        };
        for (word, hs) in by_word {
            let mut wanted: BTreeMap<Half, usize> = BTreeMap::new();
            for (rank, v, karana) in hs {
                *wanted
                    .entry((slot_at(*rank, *base), *v, *karana))
                    .or_default() += 1;
            }
            out.sites += hs.len();
            out.buckets += wanted.len();
            for (h, n) in &wanted {
                let side = if h.2 {
                    "करण"
                } else {
                    "अपादान"
                };
                if *n > 1 {
                    out.shared += n - 1;
                    out.shared_named.push(format!(
                        "{r}: base {base}: `{word}` writes {n} site(s) into \
                         the ONE exact bucket (slot {}, {}, {side}) — one \
                         source comparison emitted twice, so the claim is a \
                         FLOOR of {n} reported ONCE and {} of these sites are \
                         carried by that one line",
                        h.0,
                        h.1,
                        n - 1
                    ));
                }
                // THE SHAPE NO MUTATION IN THIS FILE CAN BITE: the same exact
                // slot against the same constant, under the same word, wanted
                // at least as often on the OTHER side. Exchanging that word's
                // operands rewrites each want into the other's and the multiset
                // does not move. Those sites are UNINFORMATIVE, named, and
                // taken OUT of the covered figure rather than out of the claim.
                let twin = wanted.get(&(h.0, h.1, !h.2)).copied().unwrap_or(0);
                if twin >= *n {
                    out.cancelled += n;
                    out.cancelled_named.push(format!(
                        "{r}: base {base}: `{word}` wants {n} comparison(s) of \
                         local slot {} against the constant {} with the name \
                         in the {side}, and {twin} of the SAME slot and \
                         constant on the OTHER side — exchanging this word's \
                         operands rewrites each want into the other's and the \
                         multiset does not move, so these {n} site(s) are \
                         UNINFORMATIVE, not covered",
                        h.0, h.1
                    ));
                }
            }
        }
    }
    out
}

/// Read one module's `pairable` half-wants into [`ExactBuckets`], at the slots
/// their DECLARATION RANKS are.
///
/// **NO BASE, AND THAT IS THE WHOLE DIFFERENCE FROM
/// [`half_recovered_buckets_of`].** A routine [`pairable`] accepts has no
/// anonymous pool at all — its declared name count equals its emitted local
/// slot count — so `slot = rank` outright (`ir.t1:1319`) and there is no cut to
/// recover. Everything else is the same sentence: the bucket is one
/// `(slot, constant, side)` triple under one word in one routine, which is
/// exactly the currency [`half_shortfalls`] reports a line in, so the bite can
/// be compared against the buckets and never against the SITES.
///
/// **THE CASES THAT MUST STILL BE REFUSED BY `cancelled`** are
/// [`ResidueBuckets`]'s three, unchanged and for the same reasons: a twin under
/// a DIFFERENT word does not cancel, because the match is word-for-word and
/// [`swap_the_half_sides`] exchanges each word's operands in place and never
/// moves a site between words; a twin in a DIFFERENT routine does not cancel,
/// because halves are read per routine label; and a twin wanted FEWER times
/// than this bucket does not cancel, because the swapped object is still asked
/// for more of it than the source demands and the swap can still bite it.
fn half_exact_buckets_of(paired: &Pairs, half_want: &BTreeMap<String, ByWordHalf>) -> ExactBuckets {
    let mut out = ExactBuckets::default();
    for (r, by_word) in half_want {
        if !paired.contains_key(r) {
            continue;
        }
        for (word, hs) in by_word {
            let mut wanted: BTreeMap<Half, usize> = BTreeMap::new();
            for h in hs {
                *wanted.entry(*h).or_default() += 1;
            }
            out.sites += hs.len();
            out.buckets += wanted.len();
            for (h, n) in &wanted {
                let side = if h.2 {
                    "करण"
                } else {
                    "अपादान"
                };
                if *n > 1 {
                    out.shared += n - 1;
                    out.shared_named.push(format!(
                        "{r}: `{word}` writes {n} site(s) into the ONE exact \
                         bucket (slot {}, {}, {side}) — one source comparison \
                         emitted twice, so the claim is a FLOOR of {n} reported \
                         ONCE and {} of these sites are carried by that one line",
                        h.0,
                        h.1,
                        n - 1
                    ));
                }
                let twin = wanted.get(&(h.0, h.1, !h.2)).copied().unwrap_or(0);
                if twin >= *n {
                    out.cancelled += n;
                    out.cancelled_named.push(format!(
                        "{r}: `{word}` wants {n} comparison(s) of local slot {} \
                         against the constant {} with the name in the {side}, \
                         and {twin} of the SAME slot and constant on the OTHER \
                         side — exchanging this word's operands rewrites each \
                         want into the other's and the multiset does not move, \
                         so these {n} site(s) are UNINFORMATIVE, not covered",
                        h.0, h.1
                    ));
                }
            }
        }
    }
    out
}

/// **WHETHER ONE MODULE'S EXACT-SLOT HALF CLAIM IS BEING FALSIFIED — ONE
/// DECISION, AND IT HAS FOUR ANSWERS WHERE THE SWEEP ASKED FOR TWO.**
///
/// The sweep asked `if file_half_exact > 0 && swapped_short.is_empty()` and
/// pushed a blindness line. Two states, and they hid two more:
///
/// * **A module with no `pairable` half-site at all** is NOTHING TO ASK, not a
///   green. The old guard folded it into the same silent branch as a module
///   whose sides really are read, so the count of modules the control is silent
///   about was never reported. [`ExactBite::NoSites`] names it.
/// * **A module whose every bucket is UNINFORMATIVE** reads the swapped object
///   green BY ARITHMETIC. A want whose twin on the other side is written at
///   least as often under the SAME word is rewritten into that twin by the
///   exchange, the multiset does not move, and no mutation in this file can
///   bite the site. Calling that module blind would read the compiler RED for a
///   property of the SOURCE. [`ResidueBuckets`] has had this arm one altitude
///   down since the cycle that measured it; this control had none, so
///   [`ExactBite::Uninformative`] is it, and the sites come OUT of the covered
///   figure rather than out of the claim.
///
/// **AND THE ARM ORDER IS THE REFUSAL.** `Moved` is decided BEFORE
/// `Uninformative`, so a module that carries cancelled buckets AND a bucket the
/// swap does bite stays in the sensitive figure and out of both new arms. A
/// cancelled bucket excuses nothing on its own; only a module with NO
/// informative bucket left is excused.
///
/// Extracted so the claim can be handed an OBJECT — the old comparison lived
/// inside the sweep's loop, where only the corpus could reach it, and the corpus
/// has only ever produced one of the four answers. Made to fire on all four in
/// [`the_exact_slot_blindness_control_names_four_states_and_counts_the_cancelled_out`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum ExactBite {
    /// No routine [`pairable`] accepts carries a half-site, so there is no
    /// claim here and nothing for a mutation to move. Counted, never reported
    /// as a defect.
    NoSites,
    /// The swap went short — the side bit is being read. Carries the count in
    /// [`half_shortfalls`]'s own currency, one per DISTINCT bucket.
    Moved { short: usize },
    /// Every bucket has a twin on the other side wanted at least as often under
    /// the same word, so the exchange cannot move this module's multiset at all.
    /// The sites are named in the uninformative list and subtracted from the
    /// covered figure.
    Uninformative { cancelled: usize },
    /// Informative buckets, and the swapped object still reads them green. The
    /// one defect of the four.
    Blind {
        informative: usize,
        cancelled: usize,
        routines: usize,
    },
}

impl ExactBite {
    /// The bite in the currency the sweep totals — zero for the three arms that
    /// are not a bite, so the sensitive figure comes off this decision rather
    /// than off a second reading of the shortfall list.
    fn short(&self) -> usize {
        match self {
            ExactBite::Moved { short } => *short,
            ExactBite::NoSites | ExactBite::Uninformative { .. } | ExactBite::Blind { .. } => 0,
        }
    }

    /// The sentence the sweep reports, or `None` for the three arms that are not
    /// a defect.
    ///
    /// **`claim` NAMES WHICH ROUTINES CARRY THE CLAIM, and it is a parameter for
    /// the reason [`WideBite::complaint`]'s `evidence` is.** Two buckets report
    /// through this one decision — the half-sites of the routines [`pairable`]
    /// accepts, and the exact-slot claim the RECOVERED routines carry — and they
    /// are different defects with different repairs: the first says a paired
    /// routine's side is unread, the second says a routine whose cut was
    /// recovered by exhaustion carries a `(slot, constant, side)` triple nothing
    /// asks for. The two phrases are [`PAIRABLE_CLAIM`] and
    /// [`RECOVERED_CLAIM`], so neither call site spells one out.
    fn complaint(&self, claim: &str) -> Option<String> {
        match self {
            ExactBite::NoSites | ExactBite::Moved { .. } | ExactBite::Uninformative { .. } => None,
            ExactBite::Blind {
                informative,
                cancelled,
                routines,
            } => Some(format!(
                "{informative} INFORMATIVE exact-slot half-site(s) in \
                 {routines} routine(s) {claim} ({cancelled} further site(s) \
                 cancelled by a same-bucket twin on the other side), and \
                 exchanging the operands of EVERY {SWAPPED_WORDS} branch in \
                 the object still reads them green — the side is not being read"
            )),
        }
    }
}

/// The routines the FIRST exact-slot claim is made of, for
/// [`ExactBite::complaint`] — those whose frame [`pairable`] accepts.
const PAIRABLE_CLAIM: &str = "`pairable` accepts";

/// The routines the SECOND is made of — those whose cut position the half-sites
/// pinned to ONE base, so [`slot_at`] names the exact triple the object must
/// carry.
const RECOVERED_CLAIM: &str = "whose cut the half-sites RECOVERED";

/// **THE THREE WORDS [`swap_the_half_sides`] ACTUALLY REWRITES, NAMED ONCE.**
///
/// Every sentence that describes this mutation used to spell the triple out, and
/// the three that existed spelled it three ways: `ExactBite`'s said
/// `समम्`/`असमम्`/`बृहत्समम्` — the SOURCE words — while the recovered and
/// residue lines said `समम्`/`असमम्`/`अन्यूनलङ्घनम्`, two source words and one
/// emitted mnemonic. The mutation rewrites the OBJECT: it matches
/// [`EXTRA_WORDS`] against emitted lines (`:801`), so the object's vocabulary is
/// the only correct one, and a reader told to look for `बृहत्समम्` in the object
/// would not find it at all. One constant, so the next bucket to report through
/// one of these decisions cannot invent a fourth spelling.
const SWAPPED_WORDS: &str = "`समलङ्घनम्`/`विषमलङ्घनम्`/`अन्यूनलङ्घनम्`";

/// [`ExactBite`]'s one decision: ask the BITTEN object for the same exact-slot
/// triples, and read the answer against what the buckets say is informative.
///
/// `bitten` is the object and not a shortfall count, because what the sweep must
/// get right is that the claim is re-asked of a MUTATED object with the HONEST
/// wants — a caller handed a number could pass any number, and the arm that
/// matters (`Moved` before `Uninformative`) would be out of a test's reach.
fn exact_bite(
    paired: &Pairs,
    half_want: &BTreeMap<String, ByWordHalf>,
    buckets: &ExactBuckets,
    bitten: &BTreeMap<&'static str, HalfPairs>,
) -> ExactBite {
    if buckets.sites == 0 {
        return ExactBite::NoSites;
    }
    let short = half_shortfalls(paired, half_want, bitten).len();
    if short > 0 {
        return ExactBite::Moved { short };
    }
    let informative = buckets.sites - buckets.cancelled;
    if informative == 0 {
        return ExactBite::Uninformative {
            cancelled: buckets.cancelled,
        };
    }
    ExactBite::Blind {
        informative,
        cancelled: buckets.cancelled,
        routines: half_want.keys().filter(|r| paired.contains_key(*r)).count(),
    }
}

/// **THE SAME DECISION FOR THE RECOVERED ROUTINES' EXACT-SLOT CLAIM, AND THE
/// GUARD IT REPLACES WAS ONE CANCELLED BUCKET AWAY FROM SILENCE.**
///
/// The sweep asked
/// `if file_recovered_sites > 0 && file_recovered_bit.is_empty() && file_recovered.cancelled_named.is_empty()`
/// — THREE booleans, and the third is not a guard but an ESCAPE. A cancelled
/// bucket is a property of the source (a want whose twin on the other side is
/// written at least as often under the SAME word cannot be moved by the
/// exchange), so a module needs exactly ONE of them to silence the control over
/// all of its OTHER buckets, however many of those the swap reads green. The
/// ruling [`ExactBite`] was extracted under says it in one line — *"a cancelled
/// bucket excuses nothing on its own; only a module with NO informative bucket
/// left is excused"* — and the arm order is what enforces it: `Moved` before
/// `Uninformative`, and `Blind` when an informative bucket survives. The corpus
/// cannot reach the difference: its 22 recovered routines cancel NOTHING, so the
/// conjunction and this decision agree on every module in the tree today and the
/// defect is visible only to a built object. Made to fire in
/// [`a_cancelled_bucket_does_not_excuse_a_recovered_routine_whose_side_is_unread`].
///
/// `bitten` is the OBJECT for [`exact_bite`]'s reason: the claim must be re-asked
/// of a mutated object with the HONEST wants, and a caller handed a shortfall
/// count could pass any number.
fn recovered_bite(
    recovered: &BTreeMap<String, usize>,
    half_want: &BTreeMap<String, ByWordHalf>,
    buckets: &ExactBuckets,
    bitten: &BTreeMap<&'static str, HalfPairs>,
) -> ExactBite {
    if buckets.sites == 0 {
        return ExactBite::NoSites;
    }
    let short = half_recovered_shortfalls(recovered, half_want, bitten).len();
    if short > 0 {
        return ExactBite::Moved { short };
    }
    let informative = buckets.sites - buckets.cancelled;
    if informative == 0 {
        return ExactBite::Uninformative {
            cancelled: buckets.cancelled,
        };
    }
    ExactBite::Blind {
        informative,
        cancelled: buckets.cancelled,
        routines: recovered
            .keys()
            .filter(|r| half_want.contains_key(*r))
            .count(),
    }
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

/// **THE `न्यूनलङ्घनम्` GROUP, PRESENT AND EMPTY.** Every widened reading runs
/// through [`cut_groups`] with this as its `lt_want`, so the honest and the
/// widened readings share ONE builder and a routine that later gains a
/// `न्यूनम्`/`अधिकम्` head is carried by the same path rather than a second.
const NO_LT_SITES: &[(usize, usize)] = &[];

/// **WHAT THE DROPPED EXCHANGE DID TO A WIDENED ROUTINE'S CUT — ONE DECISION,
/// AND IT HAS THREE ANSWERS AND NOT TWO.**
///
/// The sweep's `wide_bit_total` bucket asserts this comes out
/// [`WideBite::Quiet`] for every widened routine, because a widened routine
/// writes no `न्यूनम्`/`अधिकम्` two-name head at all: its whole evidence is the
/// three words [`drop_the_exchange`] does not touch, so every base must survive
/// the bitten object unchanged. That zero is what licenses keeping these
/// routines OUT of `cut_sensitive`.
///
/// **AND `half_wide_bit` IS THE SAME SENTENCE ABOUT THE THIRD WIDENING**, which
/// is why there is one decision here and not two. A half-site-only routine's
/// whole evidence is `(slot, constant, side)` triples under those same three
/// words, so it too must survive the bitten object unchanged, and its no-base
/// arm is `half_wide_none`'s rather than `wide_none`'s. The two readings differ
/// only in what they are HANDED — see [`wide_bite`] — so a second copy of this
/// enum would be two chances to get the arms wrong.
///
/// **AND THE THIRD ANSWER IS WHY THIS IS NOT A BOOLEAN.** A routine the
/// exhaustion left with NO base has nothing for a mutation to move, and its
/// equality is free: folding it into the quiet arm would let one zero stand for
/// "the recovery ignores `न्यूनलङ्घनम्` evidence" and for "there was no base
/// here" at once. [`WideBite::Unrecovered`] is that case NAMED — the sweep
/// reports it through `wide_none` (or `half_wide_none`) instead, and this
/// decision declines it.
///
/// Extracted so the claim can be handed an OBJECT: an inline comparison inside
/// the sweep's loop could only ever be exercised by the corpus, and the corpus
/// has never moved it. Made to fire in
/// [`a_widened_cut_handed_a_lt_want_moves_under_the_dropped_exchange`] and, for
/// the third widening,
/// [`a_half_site_cut_handed_a_lt_half_want_moves_under_the_dropped_exchange`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum WideBite {
    /// The widened recovery left no base at all, so there is no cut for the
    /// mutation to move. Reported by `wide_none`, never by this bucket.
    Unrecovered,
    /// Every base survived the bitten object unchanged — the one quiet arm, and
    /// the one the sweep asserts.
    Quiet,
    /// The bases MOVED, and the sentence names both readings.
    Moved {
        honest: Vec<usize>,
        bitten: Vec<usize>,
    },
}

impl WideBite {
    /// The sentence the sweep reports, or `None` for the two arms that are not
    /// a defect.
    ///
    /// **`evidence` NAMES THE READING, and it is a parameter rather than a
    /// constant because two buckets report through this one decision.** A
    /// sentence that named only the second widening's evidence would put
    /// `half_wide_bit`'s complaint in `wide_bit_total`'s words, and the two are
    /// different defects with different repairs. The two phrases are
    /// [`WIDE_EVIDENCE`] and [`HALF_EVIDENCE`], so neither call site spells one
    /// out.
    fn complaint(&self, evidence: &str) -> Option<String> {
        match self {
            WideBite::Unrecovered | WideBite::Quiet => None,
            WideBite::Moved { honest, bitten } => Some(format!(
                "the dropped exchange moved a cut recovered from {evidence} \
                 alone — honest {honest:?}, bitten {bitten:?}"
            )),
        }
    }
}

/// The SECOND widening's evidence, for [`WideBite::complaint`] — two-NAME heads
/// of the three words [`drop_the_exchange`] does not touch.
const WIDE_EVIDENCE: &str = "`समम्`/`असमम्`/`बृहत्समम्`";

/// The THIRD widening's — name-against-constant heads of those same three
/// words, read as `(slot, constant, side)` triples by [`emitted_halves_of`].
const HALF_EVIDENCE: &str = "name-against-constant heads";

/// [`WideBite`]'s one decision: re-run the SAME exhaustion against the bitten
/// object's groups and compare with the honest bases.
///
/// `bitten` is passed as GROUPS rather than as an object, because what the
/// sweep must get right is precisely which `lt_want` the widened reading is
/// built with — [`NO_LT_SITES`] and not the routine's residue — and a builder
/// hidden inside this function would put that choice out of a test's reach.
///
/// **AND FOR THE THIRD WIDENING THE CHOICE IS THE `half_want`**, which is why
/// the halves are a second parameter and not derived here either. That reading's
/// pair groups are both empty, so the only thing a later cycle could leak into
/// it is a `न्यूनलङ्घनम्` entry in the HALF want — and [`cut_halves`] would then
/// match a want against `emitted_halves_of`'s `न्यूनलङ्घनम्` triples, whose SIDE
/// bit this mutation flips. Made to fire on exactly that in
/// [`a_half_site_cut_handed_a_lt_half_want_moves_under_the_dropped_exchange`].
fn wide_bite(
    honest: &[usize],
    bitten_groups: &[Group<'_>],
    bitten_halves: &[HalfGroup<'_>],
    names: usize,
) -> WideBite {
    if honest.is_empty() {
        return WideBite::Unrecovered;
    }
    let bitten = consistent_bases_of(bitten_groups, bitten_halves, names);
    if bitten == honest {
        WideBite::Quiet
    } else {
        WideBite::Moved {
            honest: honest.to_vec(),
            bitten,
        }
    }
}

/// **WHICH BUCKET ONE ROUTINE'S TWO-NAME `समम्`/`असमम्`/`बृहत्समम्` SITES ARE
/// ACCOUNTED INTO.** One variant per arm of [`route_wide_sites`], so the four
/// counters the add-up identity rests on come off a `match` the compiler checks
/// rather than two `if` chains a hundred lines apart in different orders.
///
/// **THERE IS NO CATCH-ALL HERE**, and that is the difference from
/// [`HalfRoute`]. The half-site chain's last arm is a fall-through kept as a
/// guard, so its zero is a CONSTRUCTION; this chain's last arm is the widening
/// itself — the productive one — so every one of these four is REACHABLE and
/// every one of their zeros is a MEASUREMENT. Enumerated in
/// [`the_wide_site_router_names_each_shape_and_every_bucket_is_reachable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WideRoute {
    /// The routine writes a `न्यूनम्`/`अधिकम्` two-name head too, so `cut_bases`
    /// is already narrowing its cut with these sites folded in. Not exhausted
    /// again here, or the same routine would be recovered twice.
    Folded,
    /// A routine `pairable` accepts: rank IS slot, there is no cut to recover,
    /// and the widening has nothing to add.
    AlreadyPaired,
    /// The emitter lays out NO frame, so `emitted_pairs_of` carries no pair and
    /// the HAVE side is EMPTY. Named and never exhausted — a non-empty want
    /// against an empty have leaves NO candidate, so running the exhaustion
    /// would file the census's own blindness as `wide_none`.
    NoFrame,
    /// The SECOND widening's own shape, and the only arm that exhausts: a
    /// routine whose every two-name comparison is `समम्`, `असमम्` or
    /// `बृहत्समम्`, outside every claim this file made before the widening.
    Widened,
}

/// The whole second-widening routing, in ONE pass.
struct WideRouting {
    /// Every key of `want`, routed. Total: one entry per routine handed in.
    route: BTreeMap<String, WideRoute>,
    /// The candidate cuts recovered — exactly the [`WideRoute::Widened`]
    /// routines, and the map [`route_half_sites`] reads as `wide_bases`.
    bases: BTreeMap<String, Vec<usize>>,
    /// One note per [`WideRoute::NoFrame`] routine.
    no_frame: Vec<String>,
    /// One note per exhausted routine whose candidate set came out EMPTY — no
    /// single cut position explains all of its sites at once.
    none: Vec<String>,
    /// One note per exhausted routine left with `k ≥ २` candidates. KEPT
    /// DISTINCT from `none`: an empty set explains NOTHING and a set of two
    /// explains too much, and collapsing them would let the first read as the
    /// second.
    ambiguous: Vec<String>,
    /// Exhausted routines left with EXACTLY ONE candidate, and their sites.
    recovered: usize,
    recovered_sites: usize,
    /// How many exhausted routines the name-against-constant fold narrowed, and
    /// how many it brought down to exactly one.
    narrowed_by_half: usize,
    by_half: usize,
}

/// **THE SECOND WIDENING'S ROUTING, EXTRACTED SO IT CAN BE ASKED.** The pair-side
/// twin of [`route_half_sites`], and it had the same disease in a worse form:
/// THREE traversals. One over `want_residue` counted the sites this widening
/// folds away (`cut_extra_sites`, by a `contains_key` on the other map); one
/// over `want` built `wide_bases` and counted the other three buckets; a third,
/// a hundred lines further down, read each recovered set's size into
/// `wide_none` / `wide_recovered` / `wide_ambiguous`. The add-up identity the
/// sweep asserts is over figures from all three, so it was an agreement between
/// traversals and nothing said they agreed. One pass, one chain, and every
/// figure comes off the `match`.
///
/// **THE CASE THAT MUST STILL BE REFUSED, AND IT IS THE FRAMELESS ONE.** A
/// routine the emitter lays out no frame for is routed [`WideRoute::NoFrame`]
/// BEFORE the exhaustion runs, not after. `emitted_pairs_of` reads `windows` and
/// skips, so the HAVE side is empty, and [`consistent_bases_of`] keeps a base
/// only when the object carries every pair it predicts — so a non-empty want
/// kills all `names + १` candidates and the routine would come out `none`:
/// UNEXPLAINED, which is a claim about the OBJECT, when the only thing that
/// happened is that this census cannot see the frame. Exhausting it would put
/// the census's blindness in the red bucket.
#[allow(clippy::too_many_arguments)]
fn route_wide_sites(
    file: &str,
    want: &BTreeMap<String, ByWord>,
    half_want: &BTreeMap<String, ByWordHalf>,
    want_residue: &Pairs,
    want_paired: &Pairs,
    windows: &Windows,
    have_pairs: &Pairs,
    extra_have: &BTreeMap<&'static str, Pairs>,
    half_have: &BTreeMap<&'static str, HalfPairs>,
    names_of: &dyn Fn(&str) -> usize,
) -> WideRouting {
    let mut out = WideRouting {
        route: BTreeMap::new(),
        bases: BTreeMap::new(),
        no_frame: Vec::new(),
        none: Vec::new(),
        ambiguous: Vec::new(),
        recovered: 0,
        recovered_sites: 0,
        narrowed_by_half: 0,
        by_half: 0,
    };
    for (r, by_word) in want {
        let n: usize = by_word.values().map(Vec::len).sum();
        let route = if want_residue.contains_key(r) {
            WideRoute::Folded
        } else if want_paired.contains_key(r) {
            WideRoute::AlreadyPaired
        } else if !windows.contains_key(r) {
            out.no_frame.push(format!(
                "{file}: {r}: compares two names under \
                 `समम्`/`असमम्`/`बृहत्समम्` at {n} site(s) and the emitter lays \
                 out no frame for it, so `emitted_pairs_of` carries no slot pair \
                 at all — every one of the {} candidate cut positions would die \
                 on a want the object cannot answer, and the routine would be \
                 reported UNEXPLAINED for the census's blindness and not its own",
                names_of(r) + 1
            ));
            WideRoute::NoFrame
        } else {
            let pairs_only = consistent_bases_of(
                &cut_groups(r, NO_LT_SITES, have_pairs, Some(by_word), extra_have),
                &[],
                names_of(r),
            );
            let with_halves = consistent_bases_of(
                &cut_groups(r, NO_LT_SITES, have_pairs, Some(by_word), extra_have),
                &cut_halves(r, half_want.get(r), half_have),
                names_of(r),
            );
            // **AND WHAT THE HALF-SITES ALONE ADDED**, counted against the
            // two-name fold and not against an unfolded reading, or a narrowing
            // the pairs had already made would be credited to the constants.
            if with_halves.len() < pairs_only.len() {
                out.narrowed_by_half += 1;
                if with_halves.len() == 1 && pairs_only.len() > 1 {
                    out.by_half += 1;
                }
            }
            match with_halves.len() {
                0 => {
                    let carried: BTreeMap<&'static str, usize> = EXTRA_WORDS
                        .into_iter()
                        .map(|w| {
                            (
                                w,
                                extra_have.get(w).and_then(|p| p.get(r)).map_or(0, Vec::len),
                            )
                        })
                        .collect();
                    out.none.push(format!(
                        "{file}: {r}: NO cut position explains its {n} \
                         `समम्`/`असमम्`/`बृहत्समम्` site(s) at once — {} declared \
                         name(s), predicted ranks {by_word:?}, the object carries \
                         {carried:?}; predicted half-sites {:?}, the object \
                         carries {:?}",
                        names_of(r),
                        half_want.get(r),
                        half_of(half_have, r)
                    ));
                }
                1 => {
                    out.recovered += 1;
                    out.recovered_sites += n;
                }
                k => out.ambiguous.push(format!(
                    "{file}: {r}: {k} of the {} candidate cut position(s) fit \
                     its {n} site(s) ({with_halves:?}); the position is \
                     AMBIGUOUS and the exact pair is not reported for it",
                    names_of(r) + 1
                )),
            }
            out.bases.insert(r.clone(), with_halves);
            WideRoute::Widened
        };
        out.route.insert(r.clone(), route);
    }
    out
}

/// **WHICH BUCKET ONE ROUTINE'S HALF-SITES ARE ACCOUNTED INTO.** One variant per
/// arm of [`route_half_sites`], so the accounting below is a `match` the
/// compiler checks rather than a second `if` chain that can drift from the
/// first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HalfRoute {
    /// Folded into the `न्यूनलङ्घनम्` recovery — the routine writes a
    /// `न्यूनम्`/`अधिकम्` head too, so its cut is already being narrowed.
    Residue,
    /// A routine `pairable` accepts: rank IS slot, so the half-site predicts an
    /// exact `(slot, constant, side)` — see [`half_shortfalls`].
    ExactPaired,
    /// The emitter lays out NO frame, so `emitted_halves_of` carries no entry
    /// and the HAVE side is empty. Named, never folded: every candidate cut
    /// would die on a constraint this census cannot read.
    NoFrame,
    /// Visited by the SECOND widening, from a two-NAME head of one of the three
    /// `EXTRA_WORDS`.
    Widened,
    /// Visited by the THIRD widening, on its half-sites alone.
    ThirdWidened,
    /// **THE CATCH-ALL, AND IT HAS NO ARM.** See
    /// [`the_half_site_router_names_each_shape_and_can_never_fall_through`]:
    /// the chain above is total over the four memberships, so this variant is
    /// UNREACHABLE and `half_unused_routine_unseen` is zero by construction.
    /// Kept because a bucket deleted the cycle it emptied could not say so.
    Unseen,
}

/// The whole half-site routing, in ONE pass.
struct HalfRouting {
    /// Every key of `half_want`, routed. Total: the router inserts one entry
    /// per routine it is handed.
    route: BTreeMap<String, HalfRoute>,
    /// The candidate cuts the THIRD widening recovered — exactly the
    /// [`HalfRoute::ThirdWidened`] routines.
    bases: BTreeMap<String, Vec<usize>>,
    /// One note per [`HalfRoute::NoFrame`] routine.
    no_frame: Vec<String>,
}

/// **THE HALF-SITE ROUTING, EXTRACTED SO IT CAN BE ASKED.** It used to be two
/// `if` chains a hundred lines apart — one building `half_bases` and naming the
/// frameless routines, one adding the site counts up — in DIFFERENT orders, so
/// the catch-all's emptiness rested on the two agreeing and nothing said they
/// did. One pass, one chain, and the counters come off a `match`.
///
/// The reorder is sound because `wide_bases` ⊆ `windows`: the second widening
/// `continue`s on a missing frame before it inserts. That is not argued here,
/// it is ASSERTED — a frameless routine in `wide_bases` would mean the two
/// widenings read the frame map differently.
#[allow(clippy::too_many_arguments)]
fn route_half_sites(
    file: &str,
    half_want: &BTreeMap<String, ByWordHalf>,
    want_residue: &Pairs,
    want_paired: &Pairs,
    wide_bases: &BTreeMap<String, Vec<usize>>,
    windows: &Windows,
    have_pairs: &Pairs,
    extra_have: &BTreeMap<&'static str, Pairs>,
    half_have: &BTreeMap<&'static str, HalfPairs>,
    names_of: &dyn Fn(&str) -> usize,
) -> HalfRouting {
    const NO_LT: &[(usize, usize)] = &[];
    let mut out = HalfRouting {
        route: BTreeMap::new(),
        bases: BTreeMap::new(),
        no_frame: Vec::new(),
    };
    for (r, by_word) in half_want {
        let n: usize = by_word.values().map(Vec::len).sum();
        let framed = windows.contains_key(r);
        assert!(
            framed || !wide_bases.contains_key(r),
            "{file}: {r}: the SECOND widening recovered a cut for a routine the \
             emitter lays out no frame for — the two widenings read `windows` \
             differently and the frameless arm below is no longer reachable"
        );
        let route = if want_residue.contains_key(r) {
            HalfRoute::Residue
        } else if want_paired.contains_key(r) {
            HalfRoute::ExactPaired
        } else if !framed {
            out.no_frame.push(format!(
                "{file}: {r}: compares a name against a constant at {n} \
                 site(s) and the emitter lays out no frame for it, so the \
                 object carries no half-site at all and every candidate cut \
                 dies on a constraint this census cannot read"
            ));
            HalfRoute::NoFrame
        } else if wide_bases.contains_key(r) {
            HalfRoute::Widened
        } else {
            out.bases.insert(
                r.clone(),
                consistent_bases_of(
                    &cut_groups(r, NO_LT, have_pairs, None, extra_have),
                    &cut_halves(r, Some(by_word), half_have),
                    names_of(r),
                ),
            );
            HalfRoute::ThirdWidened
        };
        out.route.insert(r.clone(), route);
    }
    out
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
    let mut half_exact_buckets = 0usize;
    let mut half_exact_shared = 0usize;
    let mut half_exact_shared_named: Vec<String> = Vec::new();
    let mut half_exact_cancelled = 0usize;
    let mut half_exact_informative = 0usize;
    let mut half_exact_uninformative: Vec<String> = Vec::new();
    let mut half_exact_no_sites = 0usize;
    let mut half_exact_all_uninformative = 0usize;
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
    let mut half_run_routines = 0usize;
    let mut half_run_broken: Vec<String> = Vec::new();
    let mut pair_run_exempt = 0usize;
    let mut pair_run_gapped: Vec<String> = Vec::new();
    let mut half_residue_routines = 0usize;
    let mut half_residue_sites = 0usize;
    let mut half_residue_total = 0usize;
    let mut half_residue_cancelled_sites = 0usize;
    let mut half_residue_buckets = 0usize;
    let mut half_residue_shared = 0usize;
    let mut half_residue_shared_named: Vec<String> = Vec::new();
    let mut half_residue_uninformative: Vec<String> = Vec::new();
    let mut half_residue_short: Vec<String> = Vec::new();
    let mut half_residue_bit = 0usize;
    let mut half_residue_blind: Vec<String> = Vec::new();
    let mut half_recovered_routines = 0usize;
    let mut half_recovered_sites = 0usize;
    let mut half_recovered_buckets = 0usize;
    let mut half_recovered_shared = 0usize;
    let mut half_recovered_shared_named: Vec<String> = Vec::new();
    let mut half_recovered_cancelled = 0usize;
    let mut half_recovered_uninformative: Vec<String> = Vec::new();
    let mut half_recovered_short: Vec<String> = Vec::new();
    let mut half_recovered_bit = 0usize;
    let mut half_recovered_blind: Vec<String> = Vec::new();

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
                // **THE LAW THE FOLD MUST OBEY, AND THE NARROWING IT PAYS
                // FOR — ONE DECISION.** More constraints, never fewer: a base
                // the folded reading keeps is one the `न्यूनलङ्घनम्` reading
                // kept. A miss is the grouping wired wrongly — a mnemonic's
                // sites matched against another mnemonic's pairs — and is a
                // recovery invented rather than witnessed, so it is NOT also
                // banked as a narrowing however short the list it left.
                match fold_verdict(&lt_only, &folded) {
                    FoldVerdict::Added(added) => not_monotone.push(format!(
                        "{file}: {r}: folding the other `compare_op`s ADDED the \
                         candidate cut(s) {added:?} — `न्यूनलङ्घनम्` alone left \
                         {lt_only:?} and the fold left {folded:?}"
                    )),
                    FoldVerdict::Narrowed => {
                        cut_narrowed += 1;
                        if folded.len() == 1 && lt_only.len() > 1 {
                            cut_by_extra += 1;
                        }
                    }
                    FoldVerdict::Unchanged => {}
                }
                // **AND THE FOLDED SITE COUNT NO LONGER COMES OFF THIS
                // LOOP.** Counting the `समम्`/`असमम्`/`बृहत्समम्` sites folded
                // in HERE and the other three buckets in a second chain over
                // `extra.want` a few lines below was the same
                // two-chains-in-two-orders shape [`route_half_sites`] was
                // extracted out of. All four come off the `match` on
                // [`route_wide_sites`]'s ONE decision now.
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
        // **A ROUTINE WITH NO FRAME WINDOW IS NAMED, AND THIS LEDGER HAD THE
        // REASON THE WRONG WAY ROUND — THE SAME WAY ROUND IT HAD THE HALF-SITE
        // ONE.** It read: `emitted_pairs_of` can carry no pair, so every
        // candidate base survives VACUOUSLY and the census reports its own
        // blindness as ambiguity. It is the other direction.
        // [`consistent_bases_of`] keeps a base only when the object carries
        // every pair that base predicts, so a NON-EMPTY want against an EMPTY
        // have kills EVERY candidate: the routine would be reported
        // UNEXPLAINED, in the RED direction and not the permissive one, and it
        // would land in `wide_none` beside the routines whose object genuinely
        // contradicts itself. Witnessed in
        // [`the_wide_site_router_names_each_shape_and_every_bucket_is_reachable`],
        // which asks the exhaustion the frameless shape directly and gets the
        // empty set. That is why the frameless arm is decided BEFORE the
        // exhaustion is run and never after it.
        //
        // **AND THE ROUTING IS ONE PASS, NOT TWO CHAINS.** See
        // [`route_wide_sites`]: this loop used to build `wide_bases` and add up
        // three of the four bucket counts, while the fourth — the sites folded
        // into the `न्यूनलङ्घनम्` recovery — was counted a hundred lines above
        // by a `contains_key` on the OTHER map. That is the shape whose
        // agreement nothing asserted, and the add-up identity below rested on
        // it. Now all four come off a `match` on one decision, and the
        // `0`/`1`/`k` reading of each recovered candidate set comes off the same
        // pass that computed it rather than a third loop over `wide_bases`.
        let wide_routing = route_wide_sites(
            &file,
            &extra.want,
            &extra.half_want,
            &want_residue,
            &want_paired,
            &windows,
            &have_pairs,
            &extra_have,
            &half_have,
            &names_of,
        );
        wide_no_frame.extend(wide_routing.no_frame);
        wide_none.extend(wide_routing.none);
        wide_ambiguous.extend(wide_routing.ambiguous);
        wide_narrowed_by_half += wide_routing.narrowed_by_half;
        wide_by_half += wide_routing.by_half;
        wide_recovered += wide_routing.recovered;
        wide_sites += wide_routing.recovered_sites;
        for (r, by_word) in &extra.want {
            let n: usize = by_word.values().map(Vec::len).sum();
            match wide_routing.route[r] {
                WideRoute::Folded => cut_extra_sites += n,
                WideRoute::AlreadyPaired => extra_unused_paired += n,
                WideRoute::NoFrame => extra_unused_unvisited += n,
                WideRoute::Widened => wide_extra_sites += n,
            }
        }
        let wide_bases = wide_routing.bases;
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
        // **AND THE ROUTING IS ONE PASS, NOT TWO CHAINS.** See
        // [`route_half_sites`]: building the third widening's candidate cuts and
        // adding the site counts up were two `if` chains in DIFFERENT orders,
        // and the catch-all's emptiness rested on them agreeing. Now the
        // counters come off a `match` on one decision.
        let routing = route_half_sites(
            &file,
            &extra.half_want,
            &want_residue,
            &want_paired,
            &wide_bases,
            &windows,
            &have_pairs,
            &extra_have,
            &half_have,
            &names_of,
        );
        half_no_frame.extend(routing.no_frame);
        let half_bases = routing.bases;
        // **AND WHERE EVERY HALF-SITE WENT.** The same four buckets the pairs
        // are accounted into, plus the two the pairs cannot have: a routine
        // whose ONLY two-operand comparison is a name against a CONSTANT, now
        // visited by the third widening above, and one that even that cannot
        // reach. The last bucket is KEPT and asserted ZERO, but its zero is a
        // CONSTRUCTION fact and not a measurement — the chain it falls off is
        // total over the four memberships, proved by enumeration in
        // [`the_half_site_router_names_each_shape_and_can_never_fall_through`].
        // Kept because a bucket deleted the cycle it emptied could not say so,
        // and because a branch added to the router later would land here.
        for (r, by_word) in &extra.half_want {
            let n: usize = by_word.values().map(Vec::len).sum();
            match routing.route[r] {
                HalfRoute::Residue => {}
                // **AND THIS BUCKET IS NO LONGER "UNUSED".** A routine
                // `pairable` accepts has no anonymous pool, so rank IS slot and
                // its half-sites predict an exact `(slot, constant, side)` the
                // object must carry — see [`half_shortfalls`], which is asked
                // of exactly these and of nothing else.
                HalfRoute::ExactPaired => {
                    half_exact_paired += n;
                    half_exact_routines += 1;
                }
                HalfRoute::NoFrame => half_unused_unvisited += n,
                HalfRoute::Widened => half_widened += n,
                HalfRoute::ThirdWidened => half_only_widened += n,
                HalfRoute::Unseen => half_unused_routine_unseen += n,
            }
        }
        // **AND THE EXACT-SLOT CLAIM, OVER EXACTLY THE ROUTINES THAT BUCKET
        // COUNTS.** Every other reader of a half-site in this file narrows a
        // candidate cut; a `pairable` routine has no cut to narrow, so its
        // half-sites are the one use that can come out RED. See
        // [`half_shortfalls`] for the three refusals it keeps.
        let file_half_short = half_shortfalls(&want_paired, &extra.half_want, &half_have);
        // **AND THE SITES, THE BUCKETS AND THE SITES THE SWAP CANNOT MOVE COME
        // OFF ONE TRAVERSAL.** The site total used to be summed here beside a
        // blindness guard that knew nothing about cancellation, so the claim's
        // covered figure and its falsifier's reach were two readings that could
        // not be compared. See [`half_exact_buckets_of`].
        let file_exact = half_exact_buckets_of(&want_paired, &extra.half_want);
        let file_half_exact = file_exact.sites;
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
        // ONE DECISION, and the four states are named: see [`ExactBite`]. The
        // guard this replaces was a per-file BOOLEAN over a site count, so a
        // module with nothing to ask and a module whose every bucket the
        // exchange cannot move both fell into the same silent branch as a module
        // whose sides are read.
        let verdict = exact_bite(&want_paired, &extra.half_want, &file_exact, &swapped_have);
        half_exact_bit += verdict.short();
        half_exact_buckets += file_exact.buckets;
        half_exact_shared += file_exact.shared;
        half_exact_shared_named.extend(
            file_exact
                .shared_named
                .iter()
                .map(|s| format!("{file}: {s}")),
        );
        half_exact_cancelled += file_exact.cancelled;
        half_exact_uninformative.extend(
            file_exact
                .cancelled_named
                .iter()
                .map(|s| format!("{file}: {s}")),
        );
        half_exact_informative += file_exact.sites - file_exact.cancelled;
        match &verdict {
            ExactBite::NoSites => half_exact_no_sites += 1,
            ExactBite::Uninformative { .. } => half_exact_all_uninformative += 1,
            ExactBite::Moved { .. } | ExactBite::Blind { .. } => {}
        }
        if let Some(c) = verdict.complaint(PAIRABLE_CLAIM) {
            half_exact_blind.push(format!("{file}: {c}"));
        }
        // **AND THE `0`/`1`/`k` READING IS NOT A THIRD LOOP.** It used to be
        // one here, over `wide_bases`, a hundred lines below the pass that
        // filled it — so `wide_none`'s emptiness and `wide_extra_sites`'s total
        // were two readings of two traversals in two orders. Both come off
        // [`route_wide_sites`] now, and `wide_routines` is the count of the arm
        // that exhausts.
        wide_routines += wide_bases.len();
        half_wide_routines += half_bases.len();
        // **THE RECOVERED BASE, KEPT RATHER THAN COUNTED AND DROPPED.** A
        // singleton candidate set fixes the cut, and a fixed cut turns every
        // one of that routine's `(rank, constant, side)` wants into an exact
        // `(slot, constant, side)` — see [`half_recovered_shortfalls`], which
        // is asked of exactly this map and of nothing else. ONE base per
        // routine: the ambiguous arm below inserts nothing.
        let mut half_recovered: BTreeMap<String, usize> = BTreeMap::new();
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
                    half_recovered.insert(r.clone(), bases[0]);
                }
                k => {
                    // **AND THE SET MUST BE ONE RUN, which nothing has asked
                    // until this cycle.** Ambiguity is where this recovery
                    // stops reporting, so the shape of what it stopped at was
                    // never read at all. A singleton is trivially contiguous
                    // and an empty set is `half_wide_none`'s; only the `k ≥ २`
                    // routines carry the claim, and they are counted so the
                    // green cannot be a green on nothing. See [`base_run_gaps`].
                    half_run_routines += 1;
                    let gaps = base_run_gaps(bases);
                    if !gaps.is_empty() {
                        half_run_broken.push(format!(
                            "{file}: {r}: the {k} candidate cut position(s) \
                             {bases:?} are not one run — {gaps:?} sits inside \
                             their own range and is refused; predicted \
                             half-sites {:?}, the object carries {:?}",
                            extra.half_want.get(r),
                            half_of(&half_have, r)
                        ));
                    }
                    half_wide_ambiguous.push(format!(
                        "{file}: {r}: {k} of the {} candidate cut position(s) \
                         fit its {n} name-against-constant site(s) \
                         ({bases:?}); a half-site names ONE slot and so fixes \
                         only which side of the cut its rank falls on, so the \
                         position is AMBIGUOUS and the exact slot is not \
                         reported for it",
                        names_of(r) + 1
                    ));
                }
            }
        }
        // **AND THE 22 ROUTINES THE THIRD WIDENING *RECOVERED* CARRY AN
        // EXACT-SLOT CLAIM THAT NOTHING HAS ASKED.** Their cut is fixed by a
        // singleton candidate set, so `slot_at(rank, base)` names the slot the
        // object must carry against each constant on each side — the same claim
        // `half_shortfalls` makes of a `pairable` routine, over a cut RECOVERED
        // rather than absent. Until this cycle these routines entered no
        // shortfall reader at all: `half_residue_shortfalls` excludes them by
        // design, as circular, and it is right to — but the exact claim is not
        // the residue one, and its FALSIFIER is not circular either, because the
        // base stays the honest object's while the object asked is the SWAPPED
        // one. See [`half_recovered_shortfalls`] for the three refusals.
        let file_recovered = half_recovered_buckets_of(&half_recovered, &extra.half_want);
        let file_recovered_sites: usize = half_recovered
            .keys()
            .map(|r| extra.half_want[r].values().map(Vec::len).sum::<usize>())
            .sum();
        // **AND THE TWO READINGS ARE MADE TO AGREE OUT LOUD.** This sum walks
        // `half_recovered` and indexes `half_want`; `ExactBuckets::sites` walks
        // `half_recovered` and SKIPS a label `half_want` does not carry. The two
        // can only differ if this index would have panicked, so the identity is
        // asserted rather than left as a premise — a covered figure and a
        // cancellation counted in two traversals is the shape cycle 1005 got
        // wrong.
        assert_eq!(
            file_recovered.sites, file_recovered_sites,
            "{file}: the recovered exact-slot sites are counted twice and the \
             two readings disagree — {} from the bucket traversal, \
             {file_recovered_sites} from the want map",
            file_recovered.sites
        );
        let file_recovered_short =
            half_recovered_shortfalls(&half_recovered, &extra.half_want, &half_have);
        // THE BITE. `drop_the_exchange` cannot reach these sites — `half_wide_bit`
        // asserts the recovery itself is unmoved by it — so the only mutation
        // that can falsify this claim is the one that exchanges the operands of
        // the three words the half-sites live under. The slot and the constant
        // are unchanged by it, every other claim in this file reads the swapped
        // object exactly as it reads the honest one, and this one must go short.
        let file_recovered_bit =
            half_recovered_shortfalls(&half_recovered, &extra.half_want, &swapped_have);
        half_recovered_routines += half_recovered.len();
        half_recovered_sites += file_recovered_sites;
        half_recovered_buckets += file_recovered.buckets;
        half_recovered_shared += file_recovered.shared;
        half_recovered_shared_named.extend(
            file_recovered
                .shared_named
                .iter()
                .map(|s| format!("{file}: {s}")),
        );
        half_recovered_cancelled += file_recovered.cancelled;
        half_recovered_uninformative.extend(
            file_recovered
                .cancelled_named
                .iter()
                .map(|s| format!("{file}: {s}")),
        );
        half_recovered_short.extend(file_recovered_short.iter().map(|s| format!("{file}: {s}")));
        // ONE DECISION, and it is [`ExactBite`] — the same four states the
        // `pairable` claim's control answers, because this is the same claim over
        // a differently chosen set of routines. The conjunction this replaces let
        // a SINGLE cancelled bucket silence the control over all of a module's
        // other buckets: see [`recovered_bite`], which is where the arm order
        // that refuses that is stated and tested.
        let recovered_verdict = recovered_bite(
            &half_recovered,
            &extra.half_want,
            &file_recovered,
            &swapped_have,
        );
        assert_eq!(
            recovered_verdict.short(),
            file_recovered_bit.len(),
            "{file}: the decision's bite and the shortfall list must be the \
             SAME reading — the sensitive figure comes off the decision now, so \
             a disagreement here is the total changing meaning without saying so"
        );
        half_recovered_bit += recovered_verdict.short();
        if let Some(c) = recovered_verdict.complaint(RECOVERED_CLAIM) {
            half_recovered_blind.push(format!("{file}: {c}"));
        }
        // **AND THE PAIR-FOLDED AMBIGUOUS ROUTINES ARE EXEMPT — COUNTED, NOT
        // ASSUMED AWAY.** A pair constrains TWO ranks at once and its
        // consistent set is a union of up to three intervals ([`base_run_gaps`]
        // says which three), so a gap there is the object being read correctly
        // and demanding one run of it would red the compiler for being right.
        // The exemption is counted because an exemption nobody measures is a
        // claim quietly dropped, and the routines that really do carry a gap
        // are NAMED so the corpus can say whether the exemption is load-bearing
        // or decoration.
        for (r, bases) in cut_bases.iter().chain(wide_bases.iter()) {
            if bases.len() < 2 {
                continue;
            }
            pair_run_exempt += 1;
            let gaps = base_run_gaps(bases);
            if !gaps.is_empty() {
                pair_run_gapped.push(format!(
                    "{file}: {r}: {bases:?} skips {gaps:?}, which one pair CAN \
                     do — both ranks above the cut and both below fit, the \
                     mixed configuration between them does not"
                ));
            }
        }
        // **AND THE RESIDUE CLAIM OVER THE ROUTINES THE THIRD WIDENING LEAVES
        // AMBIGUOUS — THE ONLY CLAIM THEY CARRY AT ALL.** A half-site names ONE
        // slot, so `slot_at(rank, b)` takes two values over the `names + १`
        // candidates and a whole run of bases gives the same one: 74 of the 96
        // routines come out ambiguous by construction, no exact slot is reported
        // for them, and until this cycle that was the end of it. But
        // `अनामस्थानम्` cuts once and by exactly eight (`ir.t1:1345`), so
        // `slot ≡ rank (mod ८)` WHEREVER the cut fell, and the constant and the
        // side do not move with the cut at all — a per-site claim that needs no
        // recovery of the position, exactly as [`residues`] is for a pair.
        //
        // **AND THE ROUTINES WHOSE CUT *IS* RECOVERED ARE EXCLUDED, because
        // their base was recovered FROM THESE VERY HALF-SITES** ([`cut_halves`]
        // is what narrowed it). Re-asserting them against a residue implied by
        // the base they chose would be circular and vacuously green — the
        // instrument agreeing with itself and reporting it as coverage.
        let half_ambiguous: std::collections::BTreeSet<String> = half_bases
            .iter()
            .filter(|(_, bases)| bases.len() != 1)
            .map(|(r, _)| r.clone())
            .collect();
        half_residue_routines += half_ambiguous.len();
        let file_residue_sites: usize = half_ambiguous
            .iter()
            .map(|r| extra.half_want[r].values().map(Vec::len).sum::<usize>())
            .sum();
        half_residue_total += file_residue_sites;
        // **AND THE SITES THE EXCHANGE CANNOT BITE COME OUT OF THE COVERED
        // FIGURE HERE.** See [`half_residue_cancelled`]: a want whose twin on
        // the other side is written at least as often under the SAME word is
        // rewritten into that twin by the exchange, so it is carried whatever
        // the object holds. It keeps its residue claim — `half_residue_short`
        // still asks the honest object for it — and it stops being counted as a
        // site the falsifier reaches.
        let buckets = half_residue_buckets_of(&half_ambiguous, &extra.half_want);
        half_residue_uninformative.extend(
            buckets
                .cancelled_named
                .iter()
                .map(|s| format!("{file}: {s}")),
        );
        half_residue_shared_named
            .extend(buckets.shared_named.iter().map(|s| format!("{file}: {s}")));
        half_residue_cancelled_sites += buckets.cancelled;
        half_residue_buckets += buckets.buckets;
        half_residue_shared += buckets.shared;
        half_residue_sites += file_residue_sites - buckets.cancelled;
        let file_residue_short =
            half_residue_shortfalls(&half_ambiguous, &extra.half_want, &half_have);
        half_residue_short.extend(file_residue_short.iter().map(|s| format!("{file}: {s}")));
        // **AND THE SAME FALSIFIER, WHICH IS WHAT SEPARATES THIS FROM THE CUT
        // RECOVERY IN ANOTHER HAT.** `swap_the_half_sides` leaves the slot and
        // the constant alone and exchanges the operands of the three words these
        // sites live under. The residue is blind to the slot moving by eight and
        // must NOT be blind to the side: `half_wide_bit` asserts the widened cut
        // recovery is unmoved by `drop_the_exchange`, so if this claim also
        // reads the swapped object green it is making no claim these routines
        // did not already carry.
        let swapped_residue =
            half_residue_shortfalls(&half_ambiguous, &extra.half_want, &swapped_have);
        half_residue_bit += swapped_residue.len();
        // **AND THE BLINDNESS IS ASKED OF THE INFORMATIVE SITES ONLY.** A
        // module whose every ambiguous site has a same-bucket twin on the other
        // side reads the swapped object green BY ARITHMETIC, and reporting that
        // as a reader that reads nothing would red the compiler for a property
        // of the source. The sites are named in `half_residue_uninformative`
        // instead; what must never happen is a module with an INFORMATIVE site
        // reading green.
        let informative = file_residue_sites - buckets.cancelled;
        if informative > 0 && swapped_residue.is_empty() {
            half_residue_blind.push(format!(
                "{file}: {informative} informative half-site(s) in \
                 {} routine(s) whose cut position is AMBIGUOUS, and exchanging \
                 the operands of EVERY `समम्`/`असमम्`/`अन्यूनलङ्घनम्` branch in \
                 the object still reads their residue green — the side is not \
                 being read, and the residue claim is the cut recovery in \
                 another hat",
                half_ambiguous.len()
            ));
        }
        for (r, bases) in &cut_bases {
            let ps = &want_residue[r];
            let carried = have_pairs.get(r).map(Vec::as_slice).unwrap_or(&[]);
            let names = routines
                .iter()
                .find(|q| format!("{module}{}", q.name) == *r)
                .map_or(0, |q| q.slots.len());
            // ONE DECISION, and the three states are named: see [`CutVerdict`].
            match cut_verdict(bases) {
                // **AND THE MESSAGE CARRIES THE FOLDED EVIDENCE, because the
                // recovery is no longer `न्यूनलङ्घनम्`'s alone.** The first red
                // this cycle produced read `predicted ranks [(२, ३)], the
                // object carries [(२, ३)]` — two sets that AGREE, reported as
                // unexplained, because the constraint that actually failed was
                // a half-site the message did not print. An instrument with
                // two states where the truth has three hides its own
                // breakage.
                CutVerdict::Unexplained => cut_none.push(format!(
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
                CutVerdict::Recovered(_) => {
                    cut_recovered += 1;
                    cut_sites += ps.len();
                }
                CutVerdict::Ambiguous(fit) => cut_ambiguous.push(format!(
                    "{file}: {r}: {} of the {} candidate cut position(s) fit \
                     its {} site(s) ({fit:?}); the position is AMBIGUOUS and \
                     the exact pair is not reported for it",
                    fit.len(),
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
        // ONE DECISION, and it names WHICH side moved: see [`ReachVerdict`].
        if let Some(how) =
            reach_verdict(&extra_have, &half_have, &extra_bitten, &half_bitten).complaint()
        {
            reached_further.push(format!("{file}: `drop_the_exchange` {how}"));
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
            // ONE DECISION, and the "no base at all" case is NAMED rather than
            // skipped: see [`WideBite`].
            if let Some(how) = wide_bite(
                bases,
                &cut_groups(
                    r,
                    NO_LT_SITES,
                    &bitten_pairs,
                    extra.want.get(r),
                    &extra_bitten,
                ),
                &cut_halves(r, extra.half_want.get(r), &half_bitten),
                names_of(r),
            )
            .complaint(WIDE_EVIDENCE)
            {
                wide_bit_total.push(format!("{file}: {r}: {how}"));
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
            // ONE DECISION, and the "no base at all" case is NAMED rather than
            // skipped by an inline `is_empty()`: see [`WideBite`]. A routine the
            // exhaustion left with nothing is `half_wide_none`'s to report, and
            // folding it into this bucket's quiet would let one zero stand for
            // "the third widening ignores `न्यूनलङ्घनम्` evidence" and for
            // "there was no base here" at once.
            if let Some(how) = wide_bite(
                bases,
                &cut_groups(r, NO_LT_SITES, &bitten_pairs, None, &extra_bitten),
                &cut_halves(r, extra.half_want.get(r), &half_bitten),
                names_of(r),
            )
            .complaint(HALF_EVIDENCE)
            {
                half_wide_bit.push(format!("{file}: {r}: {how}"));
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
            "  {file:22} half  exact-slot sites {file_half_exact:<4} buckets {:<4} short {:<3} in the routine(s) `pairable` accepts (swapped: {} short, {} site(s) uninformative, {:?})",
            file_exact.buckets,
            file_half_short.len(),
            verdict.short(),
            file_exact.cancelled,
            verdict,
        );
        println!(
            "  {file:22} half  residue sites {file_residue_sites:<4} short {:<3} in the {} routine(s) whose cut is AMBIGUOUS (swapped: {} short)",
            file_residue_short.len(),
            half_ambiguous.len(),
            swapped_residue.len(),
        );
        println!(
            "  {file:22} half  recovered-exact sites {file_recovered_sites:<4} buckets {:<4} short {:<3} in the {} routine(s) whose cut the half-sites RECOVERED (swapped: {} short)",
            file_recovered.buckets,
            file_recovered_short.len(),
            half_recovered.len(),
            file_recovered_bit.len(),
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
    println!("METRIC t1_corpus_half_exact_buckets {half_exact_buckets}");
    println!("METRIC t1_corpus_half_exact_sites_sharing_a_bucket {half_exact_shared}");
    println!("METRIC t1_corpus_half_exact_sites_informative {half_exact_informative}");
    println!("METRIC t1_corpus_half_exact_sites_uninformative {half_exact_cancelled}");
    println!(
        "METRIC t1_corpus_half_exact_buckets_uninformative {}",
        half_exact_uninformative.len()
    );
    println!("METRIC t1_corpus_half_exact_modules_with_no_site {half_exact_no_sites}");
    println!(
        "METRIC t1_corpus_half_exact_modules_all_uninformative {half_exact_all_uninformative}"
    );
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
    println!("METRIC t1_corpus_half_routines_contiguous_run {half_run_routines}");
    println!(
        "METRIC t1_corpus_half_routines_run_broken {}",
        half_run_broken.len()
    );
    println!("METRIC t1_corpus_pair_routines_run_exempt {pair_run_exempt}");
    println!(
        "METRIC t1_corpus_pair_routines_run_with_a_gap {}",
        pair_run_gapped.len()
    );
    println!("METRIC t1_corpus_half_routines_slot_residue {half_residue_routines}");
    println!("METRIC t1_corpus_half_sites_slot_residue {half_residue_sites}");
    println!("METRIC t1_corpus_half_sites_slot_residue_ambiguous_total {half_residue_total}");
    println!("METRIC t1_corpus_half_sites_residue_uninformative {half_residue_cancelled_sites}");
    println!(
        "METRIC t1_corpus_half_residue_buckets_uninformative {}",
        half_residue_uninformative.len()
    );
    println!("METRIC t1_corpus_half_residue_buckets {half_residue_buckets}");
    println!("METRIC t1_corpus_half_residue_sites_sharing_a_bucket {half_residue_shared}");
    println!("METRIC t1_corpus_half_residue_claims_sensitive {half_residue_bit}");
    println!("METRIC t1_corpus_half_recovered_routines_exact_slot {half_recovered_routines}");
    println!("METRIC t1_corpus_half_recovered_sites_exact_slot {half_recovered_sites}");
    println!("METRIC t1_corpus_half_recovered_exact_buckets {half_recovered_buckets}");
    println!("METRIC t1_corpus_half_recovered_sites_sharing_a_bucket {half_recovered_shared}");
    println!(
        "METRIC t1_corpus_half_recovered_buckets_uninformative {}",
        half_recovered_uninformative.len()
    );
    println!("METRIC t1_corpus_half_recovered_sites_uninformative {half_recovered_cancelled}");
    println!("METRIC t1_corpus_half_recovered_exact_claims_sensitive {half_recovered_bit}");
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
    for s in &half_residue_blind {
        println!("NOTE  {s}");
    }
    for s in &pair_run_gapped {
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
    for s in &half_exact_uninformative {
        println!("NOTE  {s}");
    }
    for s in &half_exact_shared_named {
        println!("NOTE  {s}");
    }
    for s in &half_residue_uninformative {
        println!("NOTE  {s}");
    }
    for s in &half_residue_shared_named {
        println!("NOTE  {s}");
    }
    for s in &half_recovered_blind {
        println!("NOTE  {s}");
    }
    for s in &half_recovered_uninformative {
        println!("NOTE  {s}");
    }
    for s in &half_recovered_shared_named {
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
         recovery it reports is invented rather than witnessed. THIS ZERO IS A \
         MEASUREMENT: the arm fires on a built object in \
         `a_fold_that_crosses_mnemonics_adds_a_candidate_and_is_reported_not_monotone`, \
         where the harvested want of one mnemonic is handed another's carried \
         pairs and bases the honest reading refused come back alive:\n  {}",
        not_monotone.join("\n  ")
    );
    assert!(
        reached_further.is_empty(),
        "`drop_the_exchange` REACHED PAST `न्यूनलङ्घनम्`. It exists to lower \
         `a अधिकम् b` as `Lt(a, b)` and nothing else — same mnemonic, same line \
         count, same labels — so the other three branch words must come out \
         IDENTICAL. If they do not, the sensitivity figures below measure a \
         mutation nobody described. THIS ZERO IS A MEASUREMENT: both arms fire \
         on COMPILED objects in \
         `a_mutation_that_reaches_past_the_exchange_is_reported_and_the_two_sides_are_told_apart`, \
         one object per side — the `समम्` head's two names exchanged moves the \
         two-slot pair alone, its constant moved moves the half-site alone — and \
         `drop_the_exchange` itself, LIVE on that object, stays quiet:\n  {}",
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
        "A HALF-SITE ENTERED NO RECOVERY AND NO REFUSAL. Before the third \
         widening 184 of them sat in this bucket — routines whose every \
         two-operand comparison is a name against a constant, entered by \
         neither recovery because both are entered from a two-NAME head. **AND \
         THIS ZERO IS NOT A MEASUREMENT.** [`route_half_sites`] is a TOTAL \
         chain over the four memberships, so the fall-through arm has no input \
         at all — enumerated over the twelve legal combinations in \
         [`the_half_site_router_names_each_shape_and_can_never_fall_through`], \
         where the compiler agrees: the variant is never constructed. So this \
         line is a guard on the router growing a branch and NOT the warrant \
         that every claimed half-site is accounted for; that warrant is the \
         identity asserted just above, where `half_claimed` is harvested \
         independently of all six buckets"
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
         none of the four is one this sweep harvested and then lost. **AND \
         WHAT THIS LINE FALSIFIES HAS CHANGED.** The four figures used to come \
         off THREE traversals in three orders, so this was an agreement between \
         them and a drift in any one showed up here; they now come off the one \
         `match` in [`route_wide_sites`], which makes the partition a \
         construction and this line blind to that drift — the enumeration in \
         [`the_wide_site_router_names_each_shape_and_every_bucket_is_reachable`] \
         is what carries it instead. What is LEFT here is the half that always \
         mattered: `extra_claimed` is counted in [`extra_pairs`], independently \
         of every bucket, so a site harvested and then dropped by the router — \
         or routed into a bucket this sum forgets — still reds: \
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
         `cut_sensitive`. **AND THIS ZERO IS A MEASUREMENT**, which the red \
         above did not say until the bucket had a falsifier: \
         [`a_widened_cut_handed_a_lt_want_moves_under_the_dropped_exchange`] \
         hands the SAME compiled object's recovery a non-empty `न्यूनलङ्घनम्` \
         want in place of [`NO_LT_SITES`] and the base MOVES, while the sweep's \
         own wiring stays [`WideBite::Quiet`] on that same object with the bite \
         proved live:\n  {}",
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
         second cut. **AND THIS ZERO IS A MEASUREMENT, UNLIKE THE HALF-SITE \
         CATCH-ALL'S.** [`route_wide_sites`] has no fall-through arm — its last \
         arm is the widening itself — so this bucket is REACHABLE, and \
         [`the_wide_site_router_names_each_shape_and_every_bucket_is_reachable`] \
         makes it fire on an object each of whose pairs is explicable and whose \
         pairs share no cut. The frameless routines are kept OUT of it for the \
         same reason: an empty have side would land here and read this census's \
         blindness as the object's contradiction:\n  {}",
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
         backwards. **AND THIS ZERO IS A MEASUREMENT.** All three arms of the \
         `match` above are reachable — \
         [`a_half_routine_no_single_cut_explains_is_reported_unexplained_not_ambiguous`] \
         makes THIS one fire, on an object whose every site is individually \
         explicable and whose ranks share no cut, and reaches `recovered` and \
         `ambiguous` from the same source — so the emptiness is the corpus \
         answering and not a bucket nothing can enter. The frameless routines \
         are kept OUT of it, and an unsatisfiable want is a SHORTFALL rather \
         than a contradiction:\n  {}",
        half_wide_none.join("\n  ")
    );
    assert!(
        half_wide_bit.is_empty(),
        "A CUT RECOVERED FROM NAME-AGAINST-CONSTANT HEADS ALONE MOVED UNDER A \
         MUTATION THAT CANNOT REACH IT. `drop_the_exchange` rewrites \
         `न्यूनलङ्घनम्` lines and nothing else, and these routines write no \
         `न्यूनम्`/`अधिकम्` two-name head and no two-name head of any word — \
         both pair groups are EMPTY. A base that moves here says the third \
         widening reads pairs it claims not to use. **AND THIS ZERO IS A \
         MEASUREMENT**, not the silence of a bucket nothing can enter: \
         [`a_half_site_cut_handed_a_lt_half_want_moves_under_the_dropped_exchange`] \
         hands the SAME compiled object's recovery a `न्यूनलङ्घनम्` HALF want in \
         place of none and the base MOVES, while the sweep's own wiring stays \
         [`WideBite::Quiet`] on that same object with the bite proved live:\n  {}",
        half_wide_bit.join("\n  ")
    );
    assert!(
        half_run_broken.is_empty(),
        "A HALF-SITE-ONLY ROUTINE'S CANDIDATE CUT SET IS NOT ONE RUN. Its \
         whole evidence is `(rank, constant, side)` triples, [`slot_at`] sends \
         two DISTINCT ranks to two DISTINCT slots under every base, so the \
         containment check decomposes into one independent question per rank \
         and each answers an INTERVAL of bases — `{{b : b > r}}` when the \
         object carries the triple low, `{{b : b ≤ r}}` when it carries it \
         high, all when it carries both, none when it carries neither. An \
         intersection of intervals cannot have a hole in it. So a gap is NOT a \
         second cut and NOT a misranked declaration — those come out EMPTY and \
         are `half_wide_none`'s. It says this set was narrowed by something \
         that is not a half-site: a pair group reaching [`cut_groups`] where \
         this sweep passes `NO_LT_SITES`, or [`slot_at`] no longer monotone in \
         the base. Either leaves every recovered and ambiguous figure above \
         looking exactly right:\n  {}",
        half_run_broken.join("\n  ")
    );
    assert!(
        half_run_routines >= 60,
        "THE CONTIGUITY CLAIM IS GREEN ON NOTHING unless it is asked of \
         routines that HAVE two or more candidates. A singleton is trivially \
         one run and an empty set is `half_wide_none`'s business, so neither is \
         counted: today 74 of the third widening's 96 routines come out \
         AMBIGUOUS by construction and every one of them is asked. This sweep \
         asked {half_run_routines}, which means the ambiguous set emptied or \
         the question went quiet rather than the corpus losing the routines"
    );
    assert!(
        pair_run_exempt >= 20,
        "THE EXEMPTION IS NOT BEING ASKED OF ANYTHING. A routine whose PAIR \
         sites were folded in constrains two ranks at once, its consistent set \
         is a union of up to three intervals, and demanding one run of it would \
         red the compiler for being right — so it is held out of \
         `half_run_broken` and counted here instead. 86 pair-folded routines \
         come out AMBIGUOUS today and every one of them is exempted; this \
         sweep exempted {pair_run_exempt}. **AND NONE OF THE 86 ACTUALLY HAS A \
         GAP, measured** — `t1_corpus_pair_routines_run_with_a_gap` is ० — so \
         the exemption is a CLAIM ABOUT WHAT A PAIR CAN DO and not an \
         observation, which is why \
         `a_half_routines_candidate_cut_set_is_one_run_and_a_pair_fold_is_exempt` \
         builds the gapped set out of this very exhaustion rather than pointing \
         at a corpus routine that has one. A corpus that grows one says so on \
         the NOTE line and nothing reds"
    );
    assert!(
        half_residue_routines >= 60 && half_residue_sites >= 90,
        "THE AMBIGUOUS HALF-ROUTINES CARRY A RESIDUE CLAIM AND NOTHING ELSE. A \
         half-site names ONE slot, so `slot_at(rank, b)` takes two values over \
         the `names + १` candidates and a run of bases gives the same one: 74 of \
         the third widening's 96 routines come out AMBIGUOUS by construction, \
         112 site(s) for which no exact slot is reported. `अनामस्थानम्` cuts \
         once and by exactly eight (`ir.t1:1345`) all the same, so \
         `slot ≡ rank (mod ८)` with the SAME constant on the SAME side, \
         wherever the cut fell. AND THE MODULO IS LOAD-BEARING, measured: \
         replacing `s % POOL_SLOTS` with `s` in [`half_residues`] reds 34 of \
         the 112 across seven modules, so those sites really are carried at \
         `rank + ८` and this is not the exact-slot claim in disguise. AND THE \
         112 ARE 105 DISTINCT BUCKETS, measured: seven sites share a bucket \
         with another, which is why the bite is counted in buckets and not \
         here. This sweep reached {half_residue_sites} covered site(s) of \
         {half_residue_total} in {half_residue_routines} routine(s); fewer \
         means `half_residues` went quiet rather than the corpus losing them"
    );
    assert!(
        half_residue_bit + half_residue_uninformative.len() == half_residue_buckets
            && half_residue_buckets >= 100,
        "THE HALF RESIDUE IS THE CUT RECOVERY IN ANOTHER HAT unless a mutation \
         moves it, and `half_wide_bit` asserts the widened recovery over these \
         very routines is UNMOVED by `drop_the_exchange`. \
         `swap_the_half_sides` leaves the slot and the constant alone and \
         exchanges the operands of the three words these sites live under; the \
         residue is blind to a slot moving by eight and must NOT be blind to \
         the side. **AND THE CHECK IS ARITHMETIC AND IN ONE CURRENCY, which is \
         what cycle 1005 got wrong.** `half_residue_shortfalls` reports one \
         line per DISTINCT residue bucket, so {half_residue_buckets} bucket(s) \
         is what the bite can be measured against — never the \
         {half_residue_sites} SITES, which are more because ranks eight apart \
         share a bucket. Every bucket is either bitten or named cancelled by \
         [`half_residue_buckets_of`]; a bucket that is neither means some \
         THIRD thing absorbed the mutation. It went short {half_residue_bit} \
         time(s), {} bucket(s) are cancelled by a same-bucket twin on the \
         other side, and the two must add to {half_residue_buckets}",
        half_residue_uninformative.len()
    );
    assert!(
        half_residue_sites == half_residue_buckets + half_residue_shared
            && half_residue_shared >= 5,
        "AND THE TWO CURRENCIES MUST RECONCILE, or the hole comes back. \
         {half_residue_sites} covered site(s) are {half_residue_buckets} \
         distinct residue bucket(s) plus {half_residue_shared} site(s) that \
         landed in a bucket another site of the SAME routine and word had \
         already opened — two ranks eight apart against one constant on one \
         side. Those are carried by their bucket's FLOOR and are not a second \
         claim, which is why the shortfall reader reports them once and why \
         subtracting the bucket count from the site count names no hole. \
         Seven were measured; fewer than five means `half_residues` stopped \
         collapsing them and the bite count silently changed currency"
    );
    assert!(
        half_residue_uninformative.is_empty(),
        "A RESIDUE BUCKET WHOSE TWIN ON THE OTHER SIDE IS WANTED AS OFTEN IS A \
         SITE NO MUTATION IN THIS FILE CAN BITE, and it must not be counted as \
         covered. The corpus has none today — no routine compares one residue \
         bucket against one constant from BOTH sides under ONE word — and the \
         detector is proved to fire on the shape by \
         `a_half_site_whose_twin_is_on_the_other_side_is_uninformative_not_covered`. \
         These are named and their sites are already out of the covered \
         figure; the line is here so a corpus that grows one says so:\n  {}",
        half_residue_uninformative.join("\n  ")
    );
    assert!(
        half_residue_blind.is_empty(),
        "A MODULE MAKES THE HALF RESIDUE CLAIM OVER ITS AMBIGUOUS ROUTINES AND \
         READS THE SWAPPED OBJECT GREEN. `क समम् ५` and `५ समम् क` are \
         DIFFERENT branches and the side does not move with the cut, so a \
         module whose residue cannot tell them apart is asserting only what \
         the candidate cut set already said:\n  {}",
        half_residue_blind.join("\n  ")
    );
    assert!(
        half_recovered_short.is_empty(),
        "A ROUTINE WHOSE CUT THE HALF-SITES THEMSELVES RECOVERED DOES NOT CARRY \
         THE EXACT SLOT THAT CUT IMPLIES. A singleton candidate set fixes where \
         `अनामस्थानम्` cut the numbering, and `slot_at(rank, base)` then names \
         the exact `(slot, constant, side)` the object must carry for every want \
         — the same claim `half_shortfalls` makes of a `pairable` routine, over \
         a cut RECOVERED rather than absent. `consistent_bases_of` chose that \
         base to satisfy these very wants, so a red here cannot be the compiler \
         disagreeing with the source: it is a base handed in from a recovery \
         that stopped checking its half-groups, a `slot_at` that no longer \
         agrees with the one the exhaustion used, or a `half_have` read from a \
         DIFFERENT object than the base was recovered from. Every one of those \
         leaves `half_wide_recovered`, `half_wide_ambiguous` and \
         `half_wide_none` reporting plausible numbers:\n  {}",
        half_recovered_short.join("\n  ")
    );
    assert!(
        half_recovered_routines >= 18 && half_recovered_sites >= 60,
        "THE RECOVERED HALF-ROUTINES' EXACT-SLOT CLAIM IS GREEN ON NOTHING. \
         `half_bases` splits the third widening's 96 routines into 74 AMBIGUOUS \
         and 22 RECOVERED, and until this cycle the 22 entered no shortfall \
         reader at all — `half_residue_shortfalls` excludes them BY DESIGN, as \
         circular, and is right to, but the exact claim over the recovered base \
         ALONE is a different and stronger one and it was simply not being \
         made. Those 22 routines carry 72 site(s). This sweep claimed \
         {half_recovered_sites} site(s) over {half_recovered_routines} \
         routine(s), which means the recovery went ambiguous or the map emptied \
         rather than the corpus losing the routines"
    );
    assert!(
        half_recovered_bit + half_recovered_uninformative.len() == half_recovered_buckets
            && half_recovered_buckets >= 55,
        "THE RECOVERED ROUTINES' EXACT-SLOT CLAIM IS THE CUT RECOVERY IN \
         ANOTHER HAT unless a mutation moves it, and this is the one place in \
         the file where that objection has teeth: the base was CHOSEN to \
         explain these wants, so on the honest object the reader is green by \
         construction. `drop_the_exchange` cannot help — `half_wide_bit` \
         asserts these very recoveries are UNMOVED by it. `swap_the_half_sides` \
         can: it exchanges the operands of the three words the half-sites live \
         under, leaves the slot and the constant alone, and is asked of the \
         object while the base stays the HONEST object's, so the claim must go \
         short on every bucket that has no same-slot twin on the other side. \
         **AND THE ARITHMETIC IS IN ONE CURRENCY.** \
         `half_recovered_shortfalls` reports one line per DISTINCT \
         `(slot, constant, side)` bucket, so {half_recovered_buckets} bucket(s) \
         is what the bite is measured against and never the \
         {half_recovered_sites} SITES — {half_recovered_shared} site(s) share a \
         bucket with another, which here can only be ONE source comparison \
         emitted twice, since `slot_at` sends distinct ranks to distinct slots. \
         Every bucket is either bitten or named cancelled by \
         `half_recovered_buckets_of`; a bucket that is neither means some THIRD \
         thing absorbed the mutation. It went short {half_recovered_bit} \
         time(s) and {} bucket(s) are cancelled:\n  {}",
        half_recovered_uninformative.len(),
        half_recovered_uninformative.join("\n  ")
    );
    assert!(
        half_recovered_blind.is_empty(),
        "A MODULE MAKES THE EXACT-SLOT CLAIM OVER ITS RECOVERED HALF-ROUTINES \
         AND READS THE SWAPPED OBJECT GREEN WITH NOTHING CANCELLED. `क समम् ५` \
         and `५ समम् क` are DIFFERENT branches, the side does not move with the \
         cut, and this is the ONLY mutation in the file that can falsify a \
         claim whose base was chosen to satisfy it — so a module that cannot \
         tell them apart is asserting exactly what the candidate cut set \
         already said:\n  {}",
        half_recovered_blind.join("\n  ")
    );
    assert!(
        half_recovered_routines + half_wide_ambiguous.len() + half_wide_none.len()
            == half_wide_routines,
        "EVERY ROUTINE THE THIRD WIDENING VISITS IS RECOVERED, AMBIGUOUS OR \
         UNEXPLAINED, AND THE EXACT CLAIM IS ASKED OF THE FIRST BUCKET AND OF \
         NOTHING ELSE. {half_recovered_routines} recovered + {} ambiguous + {} \
         unexplained must be the {half_wide_routines} visited. A recovered \
         routine also reaching `half_residue_shortfalls`, or an ambiguous one \
         reaching the exact claim, would be the existential \
         `consistent_bases_of` already answered — weaker than the residue, and \
         green on the routines that carry the most sites",
        half_wide_ambiguous.len(),
        half_wide_none.len()
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
    // **AND THE ADD-UP, BECAUSE THE CANCELLED SITES ARE SUBTRACTED AND A
    // SUBTRACTION NOBODY CHECKS IS A HOLE.** `half_exact_paired` is counted off
    // [`HalfRoute::ExactPaired`] and the informative and uninformative figures
    // off [`half_exact_buckets_of`] — two traversals in two places, which is
    // exactly the pair that must be tied. They CAN be tied: `want_residue` is
    // `want_pairs` minus `want_paired` (disjoint by construction), so the
    // router's `Residue` arm can never take a `pairable` routine and the two
    // domains are the same set.
    assert_eq!(
        half_exact_paired,
        half_exact_informative + half_exact_cancelled,
        "THE EXACT-SLOT SITES DO NOT ADD UP. {half_exact_paired} site(s) were \
         routed `ExactPaired`, and the bucket traversal reads \
         {half_exact_informative} informative plus {half_exact_cancelled} \
         cancelled — so the covered figure and the router disagree about which \
         routines are `pairable`, and one of the two is reading a domain the \
         other does not"
    );
    // **AND EVERY BUCKET IS EITHER BITTEN OR NAMED CANCELLED, IN ONE CURRENCY.**
    // Cycle 1005 set a count of SITES against a count of BUCKETS and read a
    // hole that was not there; the same subtraction is available here the moment
    // the corpus grows a cancelled bucket, so the arithmetic is asserted rather
    // than the ceiling. Measured today: 34 buckets, 34 bitten, 0 cancelled.
    assert!(
        half_exact_bit + half_exact_uninformative.len() == half_exact_buckets
            && half_exact_buckets >= 20,
        "THE EXACT-SLOT HALF CLAIM HAS A BUCKET NEITHER BITTEN NOR CANCELLED, \
         SO SOME THIRD THING ABSORBED THE MUTATION. `half_shortfalls` reports \
         one line per DISTINCT `(slot, constant, side)` bucket per word per \
         routine, which is the ONE currency the bite can be measured in — never \
         the {half_exact_paired} SITES. Of {half_exact_buckets} bucket(s) the \
         swap bit {half_exact_bit} and {} are cancelled by a same-bucket twin \
         on the other side, and the two must add up",
        half_exact_uninformative.len()
    );
    // **AND THE TWO CURRENCIES RECONCILE.** A site that landed in a bucket
    // another site of the same routine and word had already opened is carried by
    // that bucket's FLOOR and is not a second claim. The corpus has none today —
    // unlike the residue's seven, because [`slot_at`] sends distinct ranks to
    // distinct slots, so only ONE source comparison emitted twice can share an
    // exact bucket — and the line is here so a corpus that grows one says so
    // instead of the bite quietly changing currency.
    assert_eq!(
        half_exact_paired,
        half_exact_buckets + half_exact_shared,
        "THE EXACT-SLOT SITES AND BUCKETS DO NOT RECONCILE. \
         {half_exact_paired} site(s) must be {half_exact_buckets} distinct \
         bucket(s) plus {half_exact_shared} site(s) sharing one:\n  {}",
        half_exact_shared_named.join("\n  ")
    );
    assert!(
        half_exact_uninformative.is_empty(),
        "AN EXACT BUCKET WHOSE TWIN ON THE OTHER SIDE IS WANTED AS OFTEN UNDER \
         THE SAME WORD IS A SITE NO MUTATION IN THIS FILE CAN BITE, and it must \
         not be counted as covered. The corpus has none today — no `pairable` \
         routine compares one slot against one constant from BOTH sides under \
         ONE word — and the detector is proved to fire on the shape by \
         `the_exact_slot_blindness_control_names_four_states_and_counts_the_cancelled_out`, \
         which also proves the module is then `ExactBite::Uninformative` and NOT \
         reported blind for it. These sites are already out of the covered \
         figure; the line is here so a corpus that grows one says so rather than \
         the claim's reach shrinking in silence:\n  {}",
        half_exact_uninformative.join("\n  ")
    );
    assert!(
        half_exact_blind.is_empty(),
        "A MODULE MAKES THE EXACT-SLOT HALF CLAIM AND READS THE SWAPPED OBJECT \
         GREEN. `क समम् ५` and `५ समम् क` are DIFFERENT branches and this is \
         the only claim in the file that can tell them apart; a module that \
         cannot is one whose sides are not being read, and its green says \
         nothing about the emitter. The two arms that are NOT this defect are \
         out of here already — {half_exact_no_sites} module(s) carry no \
         `pairable` half-site at all and {half_exact_all_uninformative} have \
         every bucket cancelled by a same-bucket twin on the other side (see \
         [`ExactBite`]), so every line below is a module with an INFORMATIVE \
         bucket:\n  {}",
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
    assert!(
        half_residue_short.is_empty(),
        "A COMPARISON OF A NAME AGAINST A CONSTANT NAMED A SLOT THAT IS NOT ITS \
         RANK MODULO EIGHT, OR THE WRONG CONSTANT, OR THE WRONG SIDE. \
         `अनामस्थानम्` (`ir.t1:1345`) interrupts the declaration run ONCE and \
         by EXACTLY `अनामस्थलसंख्या = ८`, so the name holds slot `r` or slot \
         `r + ८` and nothing else whatever rank the cut fell at; the constant \
         and the written side do not move with the cut at all \
         (`ir.t1:2186`–`:2189`). This claim needs no recovery of the cut \
         position, so it reaches the routines whose position is AMBIGUOUS — \
         which the candidate cut set, being several bases wide, reads green. A \
         miss here is a second pool, a pool of another width, or a side read \
         backwards:\n  {}",
        half_residue_short.join("\n  ")
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

/// **THE SECOND WIDENING'S BLINDNESS CONTROL, MADE TO FIRE.**
///
/// `wide_bit_total` asserts that no widened routine's cut MOVES under
/// [`drop_the_exchange`], and nothing had ever moved it. The sibling test above
/// proves the quiet is honest — an empty `न्यूनलङ्घनम्` want takes no evidence —
/// but it proves that by handing the recovery an empty want, which is the one
/// input under which the bucket cannot possibly fire. So the zero did not yet
/// tell "the widened recovery ignores `न्यूनलङ्घनम्` evidence" apart from "this
/// reading sees nothing at all", and the assert's whole sentence was untested.
///
/// **THE DEFECT THE BUCKET EXISTS FOR IS A WIRING ONE**, and it is written out
/// here: [`cut_groups`] is handed a NON-EMPTY `न्यूनलङ्घनम्` want in place of
/// [`NO_LT_SITES`] — the shape any later cycle that folded the residue want in
/// to "narrow the widened cut further" would produce — and the recovered cut
/// then MOVES between the honest object and the bitten one. That is
/// [`WideBite::Moved`], and every routine it names would be double-counted by
/// `cut_sensitive`.
///
/// **AND IT HAD TO BE A UNIT FIXTURE, because the corpus cannot reach it.**
/// [`route_wide_sites`] decides `WideRoute::Folded` on `want_residue` FIRST, so
/// no key of `wide_bases` is ever a key of `want_residue` and swapping the
/// sweep's own `lt_want` for the residue would be a no-op over all 22 widened
/// routines. The assert's sentence is therefore only falsifiable against an
/// object built to carry both heads, which is what this fixture is.
///
/// **THE FOUR CASES THAT MUST STILL BE REFUSED.**
/// - The sweep's own wiring — [`NO_LT_SITES`] on the SAME object and the SAME
///   bite — must leave the bases IDENTICAL.
/// - And that quiet must be LIVE: [`drop_the_exchange`] is asserted to move
///   THIS object's `न्यूनलङ्घनम्` two-slot pair, or the quiet is the quiet of a
///   no-op and the refusal above is worth nothing.
/// - A routine with NO `समम्`/`असमम्`/`बृहत्समम्` evidence at all must stay out:
///   an empty want is satisfied by every candidate, so its honest and bitten
///   readings agree for free, and one zero would stand for two.
/// - A routine the exhaustion left with NO base is [`WideBite::Unrecovered`]
///   and not a bite, even when the bitten reading is non-empty — that routine
///   is `wide_none`'s to report.
///
/// Its own loader, and its own fixture — compiled by the product's own front
/// end, so what the readers are handed is an object the emitter wrote.
#[test]
fn a_widened_cut_handed_a_lt_want_moves_under_the_dropped_exchange() {
    // THE FIXTURE, parameterised at the branch list alone: `WIDE` carries the
    // `समम्` head the widening recovers from, `BARE` carries none, and both
    // carry the `न्यूनम्` head whose emitted pair the mutation bites.
    let src = |branches: &str| {
        format!(
            "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    यदि प न्यूनम् अ आदि
        प्रत्यागमनम् १ ।
    इति
{branches}    प्रत्यागमनम् ० ।
इति
"
        )
    };
    // This test's own loader. A refusal is NAMED and never folded into a green:
    // a fixture that does not compile carries no object at all, and every
    // comparison below would be an equality between two absences.
    let framed = |s: &str, what: &str| -> (String, Windows) {
        match compile_framed(s) {
            // `_frames`, three fields: `Framed::Emitted` carries the whole frame as
            // its third field and these tests read only the local windows. THIS IS
            // THE SECOND TIME this exact arity has had to be repaired at a merge --
            // the driver's lineage keeps being written against the two-field form
            // while main has carried three since SAS-016. A text merge cannot see an
            // arity, so it arrives as a clean merge that does not compile.
            Framed::Emitted(t, w, _frames) => (t, w),
            Framed::DeclaresNoModule => panic!("{what}: the fixture declares `मण्डलम् म`"),
            Framed::Refused(e) => panic!("{what}: the fixture compiles: {e}"),
        }
    };
    let wide_src = src("    यदि प समम् आ आदि\n        प्रत्यागमनम् २ ।\n    इति\n");
    let routines = source_routines(&wide_src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 3, "प, अ, आ — three names, so four candidate cuts");
    let extra = extra_pairs("म", &routines);
    assert_eq!(
        (extra.claimed, extra.half_claimed, extra.numeral),
        (1, 0, 0),
        "the harvest is REAL and it is the one two-name `समम्` head: {:?}",
        extra.want
    );
    let by_word = extra.want["मक"].clone();
    let wide_want = Some(&by_word);
    let (text, windows) = framed(&wide_src, "the honest object");
    let bitten_text = drop_the_exchange(&text);

    // The two readings, off the HONEST windows for both objects, exactly as the
    // sweep does it: a bite that moved the frames would be a different mutation.
    let read = |t: &str| -> BTreeMap<&'static str, Pairs> {
        EXTRA_WORDS
            .into_iter()
            .map(|m| (m, emitted_pairs_of(t, &windows, m, &mut Vec::new())))
            .collect()
    };
    let extra_have = read(&text);
    let extra_bitten = read(&bitten_text);
    let lt_have = emitted_pairs(&text, &windows, &mut Vec::new());
    let lt_bitten = emitted_pairs(&bitten_text, &windows, &mut Vec::new());
    let no_halves: BTreeMap<&'static str, HalfPairs> = BTreeMap::new();

    // ── ONE: THE FIXTURE CARRIES BOTH HEADS, and the object really does hold a
    // slot pair for each. Without this every comparison below could be quiet
    // for the uninteresting reason that there is nothing to compare.
    assert_eq!(
        extra_have["समलङ्घनम्"]["मक"],
        vec![(0usize, 2usize)],
        "`प समम् आ` — प at local slot ० against आ at २, so the emitted cut is          after every name"
    );
    assert_eq!(
        lt_have["मक"],
        vec![(0usize, 1usize)],
        "and `प न्यूनम् अ` is the करण/अपादान pair the mutation exists to move"
    );

    // ── TWO: AND THE BITE IS LIVE. `drop_the_exchange` exchanges THIS object's
    // `न्यूनलङ्घनम्` operands and reaches no other branch word — the two halves
    // of the refusal in part FOUR, each asserted rather than assumed.
    assert_eq!(
        lt_bitten["मक"],
        vec![(1usize, 0usize)],
        "the mutation DOES bite this object's `न्यूनलङ्घनम्` pair, or part FOUR          proves nothing"
    );
    assert_eq!(
        extra_bitten, extra_have,
        "and it reaches NO other branch word, so the widened evidence is          carried identically by both objects"
    );

    // ── THREE: THE HONEST RECOVERY, and it is a KNOWN answer rather than
    // "not empty". `(०, २)` demands `b > ०` and `b > २` at once, so of the four
    // candidates only `b = ३` fits.
    let honest_wide = consistent_bases_of(
        &cut_groups("मक", NO_LT_SITES, &lt_have, wide_want, &extra_have),
        &[],
        names,
    );
    assert_eq!(
        honest_wide,
        vec![3usize],
        "exactly one cut position explains the `समम्` site"
    );

    // ── FOUR, REFUSED: THE SWEEP'S OWN WIRING IS QUIET. `NO_LT_SITES` is an
    // empty want, so the `न्यूनलङ्घनम्` group constrains nothing and the base
    // cannot move — on an object whose `न्यूनलङ्घनम्` pair part TWO showed the
    // mutation really does rewrite.
    assert_eq!(
        wide_bite(
            &honest_wide,
            &cut_groups("मक", NO_LT_SITES, &lt_bitten, wide_want, &extra_bitten),
            &cut_halves("मक", None, &no_halves),
            names,
        ),
        WideBite::Quiet,
        "THE CASE THAT MUST STILL BE REFUSED: the widened reading takes no          evidence from `न्यूनलङ्घनम्`, so the sweep's zero is a reach and not an          absence"
    );

    // ── FIVE: AND THE DEFECT FIRES. The SAME object, the SAME bite, the SAME
    // exhaustion — the one thing changed is the want the `न्यूनलङ्घनम्` group is
    // built with, which is the wiring `wide_bit_total` exists to catch. `b = ३`
    // is the only base the `समम्` site allows and the bitten object no longer
    // carries `(०, १)` under `न्यूनलङ्घनम्`, so the recovery goes EMPTY.
    let lt_want: Vec<(usize, usize)> = vec![(0, 1)];
    let moved = wide_bite(
        &honest_wide,
        &cut_groups("मक", &lt_want, &lt_bitten, wide_want, &extra_bitten),
        &cut_halves("मक", None, &no_halves),
        names,
    );
    assert_eq!(
        moved,
        WideBite::Moved {
            honest: vec![3usize],
            bitten: Vec::new(),
        },
        "a widened recovery that READS `न्यूनलङ्घनम्` pairs moves under a          mutation confined to them"
    );
    let how = moved
        .complaint(WIDE_EVIDENCE)
        .expect("the moved arm is a complaint");
    assert!(
        how.contains("honest [3], bitten []"),
        "and the sentence the sweep pushes names BOTH readings: {how}"
    );
    // AND THE HONEST WIRING IS NOT MERELY QUIET, IT IS DIFFERENT: the two
    // readings of the same bitten object disagree, so part FOUR's zero is the
    // absence of this defect and not the absence of a question.
    assert_ne!(
        wide_bite(
            &honest_wide,
            &cut_groups("मक", &lt_want, &lt_bitten, wide_want, &extra_bitten),
            &cut_halves("मक", None, &no_halves),
            names,
        ),
        WideBite::Quiet,
        "the two wirings answer differently on one object, which is what makes          the bucket an instrument"
    );

    // ── SIX, REFUSED: A ROUTINE WITH NO `समम्`/`असमम्`/`बृहत्समम्` EVIDENCE AT
    // ALL STAYS OUT. `extra_pairs` gives it no want, so `route_wide_sites`
    // never enters it and `wide_bases` has no key for it — and it must, because
    // an empty want is satisfied by EVERY candidate: its honest and bitten
    // readings agree for free, and that zero would say nothing while looking
    // exactly like part FOUR's.
    let bare_src = src("");
    let bare_routines = source_routines(&bare_src);
    let bare_extra = extra_pairs("म", &bare_routines);
    assert!(
        bare_extra.want.is_empty() && bare_extra.claimed == 0,
        "no two-name `समम्`/`असमम्`/`बृहत्समम्` head: {:?}",
        bare_extra.want
    );
    let (bare_text, bare_windows) = framed(&bare_src, "the evidence-free object");
    let bare_lt = emitted_pairs(&bare_text, &bare_windows, &mut Vec::new());
    let bare_bitten = emitted_pairs(
        &drop_the_exchange(&bare_text),
        &bare_windows,
        &mut Vec::new(),
    );
    assert_ne!(
        bare_lt, bare_bitten,
        "the mutation bites this object too, so the free agreement below is          not an absence of a bite"
    );
    let none: BTreeMap<&'static str, Pairs> = BTreeMap::new();
    let all_bases = consistent_bases_of(
        &cut_groups("मक", NO_LT_SITES, &bare_lt, None, &none),
        &[],
        names,
    );
    assert_eq!(
        all_bases,
        vec![0usize, 1, 2, 3],
        "an empty want is satisfied by every one of the four candidates"
    );
    assert_eq!(
        wide_bite(
            &all_bases,
            &cut_groups("मक", NO_LT_SITES, &bare_bitten, None, &none),
            &cut_halves("मक", None, &no_halves),
            names,
        ),
        WideBite::Quiet,
        "so such a routine would report QUIET for free — which is why the          bucket is entered from `extra.want` and not from every routine"
    );

    // ── SEVEN, REFUSED: A ROUTINE WITH NO BASE IS NOT A BITE. The bitten
    // reading here is the WIDE object's and is non-empty, so a boolean
    // comparison would call this a move; `Unrecovered` names it instead, and
    // `wide_none` is where that routine is reported.
    assert_eq!(
        wide_bite(
            &[],
            &cut_groups("मक", NO_LT_SITES, &lt_have, wide_want, &extra_have),
            &cut_halves("मक", None, &no_halves),
            names,
        ),
        WideBite::Unrecovered,
        "an unexplained routine has no cut for a mutation to move"
    );
    assert!(
        WideBite::Unrecovered.complaint(WIDE_EVIDENCE).is_none()
            && WideBite::Quiet.complaint(WIDE_EVIDENCE).is_none(),
        "and only one of the three arms is a defect, under either reading's \
         evidence phrase"
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

/// **EVERY SHAPE THE HALF-SITE ROUTER CAN BE HANDED, AND THE ONE THE LEDGER
/// CALLED A MEASUREMENT IS A CONSTRUCTION.**
///
/// The sweep's half-site accounting names six buckets, and two of them were
/// asserted ZERO with nothing built that had ever made either fire:
/// `t1_corpus_half_routines_no_frame` and
/// `t1_corpus_cut_half_sites_unused_routine_unseen`. A bucket asserted zero
/// that no test has ever made fire is indistinguishable from a bucket nothing
/// can reach — and the two turn out to be DIFFERENT CASES:
///
/// * `half_no_frame` IS reachable, and this test makes it fire. A routine the
///   emitter lays out no frame for has an EMPTY have side, so a non-empty want
///   is satisfied by no base and the routine would be reported UNEXPLAINED —
///   the census's own blindness in the RED direction. Its corpus zero says
///   something: no such routine is in reach today.
/// * `half_unused_routine_unseen` is NOT reachable. [`route_half_sites`] is a
///   total chain over the four memberships, so the fall-through arm has no
///   input at all. Enumerated below over all twelve legal combinations. Its
///   zero is therefore NOT the warrant the ledger read it as; the warrant that
///   every claimed half-site is accounted for is the IDENTITY the sweep
///   asserts — `half_claimed` harvested independently equals the six buckets
///   summed. The bucket is kept as a guard on the router growing a branch.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
///
/// The two buckets must stay DISTINCT: a frameless routine must route
/// `NoFrame` and never the catch-all, or the catch-all's zero degrades to "no
/// frameless routine". Asserted both ways round, on the same fixture.
///
/// And the router must not reach a bucket except through the sweep's own
/// decision — so nothing here pushes a message or bumps a counter directly;
/// every assertion below reads what `route_half_sites` answered.
///
/// And `wide_bases` ⊆ `windows` is the invariant that licenses the ONE
/// reordering the extraction made (`NoFrame` now decided before `Widened`,
/// which is loop B's order, not loop A's). It is asserted in the router, not
/// argued, and the abort is witnessed here.
///
/// Its own loader, and its own fixture.
#[test]
fn the_half_site_router_names_each_shape_and_can_never_fall_through() {
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
    let extra = extra_pairs("म", &routines);
    assert_eq!(
        (extra.want.len(), extra.half_claimed),
        (0, 3),
        "the harvest is REAL and it is half-sites only: no two-NAME head, so \
         the second widening has nothing to visit and the router is asked \
         about exactly the shape the third widening exists for"
    );
    assert_eq!(
        extra.half_want.keys().collect::<Vec<_>>(),
        vec!["मक"],
        "one routine in the want"
    );

    // The object, pinning base ५ from constants alone: `(१, ५, करण)` demands
    // `b > १`, `(४, ७, करण)` demands `b > ४`, `(१३, ८, अपादान)` demands
    // `b ≤ ५`. The same three the sibling test above exhausts by hand, so a
    // route that claims to have recovered a cut can be checked against a KNOWN
    // answer rather than against "not empty".
    let half_have: BTreeMap<&'static str, HalfPairs> = [
        ("समलङ्घनम्", vec![(1usize, 5i128, true)]),
        ("विषमलङ्घनम्", vec![(4usize, 7i128, true)]),
        ("अन्यूनलङ्घनम्", vec![(13usize, 8i128, false)]),
    ]
    .into_iter()
    .map(|(w, hs)| -> (&'static str, HalfPairs) {
        (w, [("मक".to_string(), hs)].into_iter().collect())
    })
    .collect();
    let framed: Windows = [("मक".to_string(), (0u64, 80u64))].into_iter().collect();
    let one: Pairs = [("मक".to_string(), vec![(3usize, 4usize)])]
        .into_iter()
        .collect();
    let wide: BTreeMap<String, Vec<usize>> =
        [("मक".to_string(), vec![5usize])].into_iter().collect();
    let names_of = |r: &str| {
        routines
            .iter()
            .find(|q| format!("म{}", q.name) == *r)
            .map_or(0, |q| q.slots.len())
    };
    let route =
        |residue: &Pairs, paired: &Pairs, w: &Windows, wb: &BTreeMap<String, Vec<usize>>| {
            route_half_sites(
                "fixture.t1",
                &extra.half_want,
                residue,
                paired,
                wb,
                w,
                &Pairs::new(),
                &BTreeMap::new(),
                &half_have,
                &names_of,
            )
        };
    let none = Pairs::new();
    let unwide = BTreeMap::new();
    let nowin = Windows::new();

    // ── ONE: THE THIRD WIDENING'S OWN SHAPE. No `न्यूनम्` head, not
    // `pairable`, a frame laid out, no two-name pair — the routine the third
    // widening was built for. It must be routed there AND carry the recovered
    // cut, because the bucket count and the base map are now one decision.
    let third = route(&none, &none, &framed, &unwide);
    assert_eq!(third.route["मक"], HalfRoute::ThirdWidened);
    assert_eq!(
        third.bases["मक"],
        vec![5usize],
        "the route that says `ThirdWidened` is the route that carries the cut"
    );
    assert!(third.no_frame.is_empty(), "a frame was laid out");

    // ── TWO: THE NO-FRAME BUCKET FIRES, and it is the one shape that both
    // NAMES a routine and counts it. `windows` has no entry, so
    // `emitted_halves_of` would carry none either and every candidate cut dies
    // on a constraint this census cannot read.
    let frameless = route(&none, &none, &Windows::new(), &unwide);
    assert_eq!(frameless.route["मक"], HalfRoute::NoFrame);
    assert_eq!(
        frameless.no_frame.len(),
        1,
        "the bucket NAMES it: {:?}",
        frameless.no_frame
    );
    assert!(
        frameless.no_frame[0].contains("fixture.t1: मक")
            && frameless.no_frame[0].contains("3 site(s)"),
        "and the note says which routine and how many sites: {}",
        frameless.no_frame[0]
    );
    assert!(
        frameless.bases.is_empty(),
        "and NO cut is recovered for it — the whole point of the bucket is that \
         the exhaustion must not be run on an empty have side"
    );

    // ── THREE: AND THE TWO BUCKETS ARE DISTINCT. The refusal that matters: if
    // a frameless routine fell through to the catch-all instead, the
    // catch-all's zero would mean only "no frameless routine in reach" and the
    // no-frame bucket's zero would mean nothing at all.
    assert_ne!(
        frameless.route["मक"],
        HalfRoute::Unseen,
        "a frameless routine must land in `NoFrame`, never in the catch-all"
    );
    assert_ne!(
        third.route["मक"],
        HalfRoute::NoFrame,
        "and a framed one must not be named as frameless"
    );

    // ── FOUR: THE THREE BUCKETS THAT BELONG TO SOMEBODY ELSE'S RECOVERY, each
    // taken in the order the chain decides them. `Residue` wins over
    // everything, `ExactPaired` over the widenings, `NoFrame` over `Widened`.
    assert_eq!(
        route(&one, &none, &framed, &unwide).route["मक"],
        HalfRoute::Residue
    );
    assert_eq!(
        route(&one, &one, &framed, &wide).route["मक"],
        HalfRoute::Residue
    );
    assert_eq!(
        route(&none, &one, &framed, &wide).route["मक"],
        HalfRoute::ExactPaired
    );
    assert_eq!(
        route(&none, &none, &framed, &wide).route["मक"],
        HalfRoute::Widened
    );
    assert!(
        route(&none, &none, &framed, &wide).bases.is_empty(),
        "a routine the SECOND widening already recovered is not exhausted twice"
    );

    // ── FIVE: AND THE CATCH-ALL HAS NO INPUT. Enumerated, not argued: the
    // twelve legal combinations of the four memberships — `wide` without a
    // frame is the illegal one and part SIX witnesses its abort — and not one
    // of them falls through. This is what the ledger read as a measurement:
    // `half_unused_routine_unseen` is zero because the chain is TOTAL, so its
    // zero is no evidence about the corpus, and the accounting identity is the
    // only thing carrying that weight.
    let mut seen = std::collections::BTreeSet::new();
    let mut legal = 0usize;
    for res in [false, true] {
        for pai in [false, true] {
            for fr in [false, true] {
                for wi in [false, true] {
                    if wi && !fr {
                        continue;
                    }
                    legal += 1;
                    let r = route(
                        if res { &one } else { &none },
                        if pai { &one } else { &none },
                        if fr { &framed } else { &nowin },
                        if wi { &wide } else { &unwide },
                    );
                    assert_ne!(
                        r.route["मक"],
                        HalfRoute::Unseen,
                        "residue={res} paired={pai} framed={fr} wide={wi} fell \
                         through the router — the catch-all has an input after \
                         all and its asserted zero is a claim about the corpus \
                         again"
                    );
                    seen.insert(r.route["मक"]);
                }
            }
        }
    }
    assert_eq!(legal, 12, "four memberships, one combination ruled out");
    assert_eq!(
        seen.len(),
        5,
        "and all five REACHABLE buckets were exercised, so the enumeration is \
         not green on one arm twelve times: {seen:?}"
    );

    // ── SIX: THE INVARIANT THE REORDER RESTS ON, WITNESSED. The extraction
    // decides `NoFrame` before `Widened`; loop A decided `Widened` first. The
    // two agree only because the second widening `continue`s on a missing frame
    // before it inserts, so `wide_bases` ⊆ `windows`. A frameless routine in
    // `wide_bases` must ABORT rather than be routed either way — silently
    // choosing would hide the two widenings reading `windows` differently.
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        route(&none, &none, &Windows::new(), &wide).route["मक"]
    }));
    assert!(
        caught.is_err(),
        "a cut recovered for a frameless routine was routed instead of refused: \
         {caught:?}"
    );

    // ── SEVEN: AND THE ROUTING IS TOTAL THE OTHER WAY TOO — one entry per
    // routine handed in, so the `match` at the call site cannot index a key the
    // router never wrote and no half-site can be dropped by the router simply
    // omitting it.
    for r in [&third, &frameless] {
        assert_eq!(
            r.route.keys().collect::<Vec<_>>(),
            extra.half_want.keys().collect::<Vec<_>>(),
            "every routine in the want is routed"
        );
    }
}
/// **THE THIRD WIDENING'S OWN PAIR — AND THE ZERO NOTHING HAD EVER MADE FIRE.**
///
/// `half_wide_none` (`t1_corpus_half_routines_cut_unexplained`) is asserted
/// EMPTY and its twin `half_wide_ambiguous` is asserted NON-empty: one a floor
/// with a corpus witness, one a ceiling with none. A zero no test has made fire
/// is indistinguishable from a bucket nothing can reach, and last cycle's audit
/// of the half-site router found exactly that in `half_unused_routine_unseen`.
/// **THE ANSWER HERE IS THE OTHER ONE.** The `match bases.len()` the sweep runs
/// over [`route_half_sites`]'s recovered map has all THREE arms reachable, and
/// all three fire below on ONE source and three objects that differ only in
/// what they carry: `0` → `half_wide_none`, `1` → `half_wide_recovered`,
/// `k ≥ २` → `half_wide_ambiguous`. So the corpus zero says something — no
/// object in reach contradicts itself across its own name-against-constant
/// sites.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
///
/// The contradiction must be ACROSS RANKS and never WITHIN ONE. A half-site
/// names ONE slot, so [`slot_at`] sends rank `r` to `r` on `{b : b > r}` and to
/// `r + ८` on `{b : b ≤ r}` — two intervals that PARTITION the `names + १`
/// candidates. A routine whose evidence is one rank therefore cannot come out
/// empty by contradicting itself: whichever slot the object carries, a whole
/// RUN of bases gives it. Enumerated below, and the witness is checked both
/// ways round — carrying EITHER of the two contradicting ranks on BOTH sides
/// restores a non-empty set, so it is the pair that is unexplained and neither
/// site alone.
///
/// And the empty set must not be this census's blindness. Two shapes produce
/// one and only one of them is what the bucket claims: an object that carries
/// NOTHING for a rank is a SHORTFALL — the side read backwards or the constant
/// wrong — and is built below as a named control, kept DISTINCT from the
/// witness, every one of whose three sites is individually explicable.
///
/// And `none` must stay distinct from `ambiguous`: an empty set explains
/// NOTHING and a set of `k ≥ २` explains too much.
///
/// Nothing here pushes a message or bumps a counter: every candidate set read
/// below is the one [`route_half_sites`] answered with. Its own loader, its own
/// source, and its own object.
#[test]
fn a_half_routine_no_single_cut_explains_is_reported_unexplained_not_ambiguous() {
    // THIS TEST'S OWN SOURCE. Ten declared names — `प` at rank ०, then `अ` …
    // `ऌ` at १ … ९ in declaration order (`ir.t1:1320`) — so eleven candidate
    // cuts, ० through १०. Three heads, each a NAME against a CONSTANT and no
    // two-NAME head of any word: the shape the third widening exists for, and
    // the shape neither of the other two recoveries ever visits.
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
    यदि आ समम् ३ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ऊ असमम् ९ आदि
        प्रत्यागमनम् २ ।
    इति
    यदि ४ बृहत्समम् इ आदि
        प्रत्यागमनम् ३ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 10, "प, अ … ऌ — ten names, so eleven candidate cuts");
    let extra = extra_pairs("म", &routines);
    assert_eq!(
        (extra.want.len(), extra.half_claimed),
        (0, 3),
        "the harvest is REAL and it is half-sites only: no two-NAME head, so \
         the second widening has nothing to visit and the router is asked \
         about exactly the shape the third widening exists for"
    );
    // **AND THE WANT IS PINNED, because every base table below is worked out
    // by hand against these three triples and a harvest that moved would leave
    // the arithmetic reading right and meaning nothing.** `आ` is rank २, `इ`
    // rank ३, `ऊ` rank ६; a name-first head puts the name in the करण and a
    // numeral-first head in the अपादान.
    let want: ByWordHalf = [
        ("समलङ्घनम्", vec![(2usize, 3i128, true)]),
        ("विषमलङ्घनम्", vec![(6usize, 9i128, true)]),
        ("अन्यूनलङ्घनम्", vec![(3usize, 4i128, false)]),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        extra.half_want.keys().collect::<Vec<_>>(),
        vec!["मक"],
        "one routine in the want"
    );
    assert_eq!(extra.half_want["मक"], want);

    // **THE BY-HAND TABLE, one line per want, the interval it accepts.** The
    // slot is the only thing that moves with the cut — the constant is the
    // object's own and the side is the written side — so a want carried LOW
    // (`slot = rank`) accepts `{b : b > rank}`, carried HIGH
    // (`slot = rank + ८`) accepts `{b : b ≤ rank}`, carried BOTH accepts every
    // candidate, carried NEITHER accepts none.
    let lo = |r: usize, v: i128, k: bool| vec![(r, v, k)];
    let hi = |r: usize, v: i128, k: bool| vec![(r + POOL_SLOTS, v, k)];
    let both = |r: usize, v: i128, k: bool| vec![(r, v, k), (r + POOL_SLOTS, v, k)];
    let object = |s: Vec<Half>, x: Vec<Half>, a: Vec<Half>| -> BTreeMap<&'static str, HalfPairs> {
        [("समलङ्घनम्", s), ("विषमलङ्घनम्", x), ("अन्यूनलङ्घनम्", a)]
            .into_iter()
            .map(|(w, hs)| -> (&'static str, HalfPairs) {
                (w, [("मक".to_string(), hs)].into_iter().collect())
            })
            .collect()
    };
    let framed: Windows = [("मक".to_string(), (0u64, 80u64))].into_iter().collect();
    let names_of = |r: &str| {
        routines
            .iter()
            .find(|q| format!("म{}", q.name) == *r)
            .map_or(0, |q| q.slots.len())
    };
    let route = |hh: &BTreeMap<&'static str, HalfPairs>| {
        route_half_sites(
            "fixture.t1",
            &extra.half_want,
            &Pairs::new(),
            &Pairs::new(),
            &BTreeMap::new(),
            &framed,
            &Pairs::new(),
            &BTreeMap::new(),
            hh,
            &names_of,
        )
    };
    // The three objects, differing ONLY in which slot each site is carried at.
    //
    //                      rank २ (३, करण)   rank ६ (९, करण)   rank ३ (४, अपादान)
    //   unexplained        HIGH  → b ≤ २     LOW   → b ≥ ७     BOTH → every b
    //   recovered          LOW   → b ≥ ३     BOTH  → every b   HIGH → b ≤ ३
    //   ambiguous          BOTH  → every b   LOW   → b ≥ ७     BOTH → every b
    let unexplained = object(hi(2, 3, true), lo(6, 9, true), both(3, 4, false));
    let recovered = object(lo(2, 3, true), both(6, 9, true), hi(3, 4, false));
    let ambiguous = object(both(2, 3, true), lo(6, 9, true), both(3, 4, false));

    // ── ONE: `half_wide_none` FIRES, THROUGH THE ROUTER'S OWN DECISION. The
    // routine is routed to the third widening — not to the frameless bucket,
    // which is the one other way an empty set can be reached — and the map the
    // sweep reads `bases.len()` off carries the EMPTY set for it.
    let r0 = route(&unexplained);
    assert_eq!(r0.route["मक"], HalfRoute::ThirdWidened);
    assert_eq!(
        r0.bases["मक"],
        Vec::<usize>::new(),
        "`b ≤ २` and `b ≥ ७` share no candidate, so NO cut position explains \
         all three sites at once"
    );
    assert!(
        r0.no_frame.is_empty(),
        "and the empty set is NOT this census's blindness: a frame IS laid out \
         and the object DOES carry a half-site for every one of the three \
         wants: {:?}",
        r0.no_frame
    );

    // ── TWO: AND THE OTHER TWO ARMS FIRE ON THE SAME SOURCE, so `none` is
    // distinguishable from both of them rather than being the only shape the
    // fixture can make. `1` is `half_wide_recovered` and `k ≥ २` is
    // `half_wide_ambiguous`; the sweep's `match` sends these three objects to
    // three different buckets.
    let r1 = route(&recovered);
    assert_eq!(r1.route["मक"], HalfRoute::ThirdWidened);
    assert_eq!(
        r1.bases["मक"],
        vec![3usize],
        "`b ≥ ३` and `b ≤ ३` leave exactly one candidate"
    );
    let rk = route(&ambiguous);
    assert_eq!(rk.route["मक"], HalfRoute::ThirdWidened);
    assert_eq!(
        rk.bases["मक"],
        vec![7usize, 8, 9, 10],
        "only `b ≥ ७` survives, and a half-site names ONE slot so it cannot \
         narrow further"
    );
    assert_ne!(
        r0.bases["मक"].len(),
        rk.bases["मक"].len(),
        "THE REFUSAL: an EMPTY set explains nothing and a set of k ≥ २ \
         explains too much — collapsing them would let a census that explains \
         nothing read as one that explains too much"
    );

    // ── THREE: AND THE CONTRADICTION IS ACROSS RANKS, checked BOTH WAYS ROUND.
    // Carrying EITHER of the two contradicting ranks on both sides of the cut
    // widens that rank's interval to every candidate, and the set comes back
    // non-empty — so neither site is unexplained on its own and it is the PAIR
    // that no cut position reconciles.
    assert_eq!(
        route(&object(both(2, 3, true), lo(6, 9, true), both(3, 4, false))).bases["मक"],
        vec![7usize, 8, 9, 10],
        "drop rank २'s constraint and rank ६'s own interval survives whole"
    );
    assert_eq!(
        route(&object(hi(2, 3, true), both(6, 9, true), both(3, 4, false))).bases["मक"],
        vec![0usize, 1, 2],
        "drop rank ६'s constraint and rank २'s own interval survives whole"
    );

    // ── FOUR: AND WITHIN ONE RANK A CONTRADICTION IS IMPOSSIBLE — enumerated,
    // not argued. [`slot_at`] takes exactly two values over the eleven
    // candidates and the two carriages PARTITION them, so a routine whose
    // evidence is one rank always has a whole RUN of bases left however the
    // object carries it. A fixture that made `half_wide_none` fire on a single
    // half-site would be asserting something the arithmetic forbids.
    for r in [2usize, 6] {
        let low: Vec<usize> = (0..=names).filter(|b| slot_at(r, *b) == r).collect();
        let high: Vec<usize> = (0..=names)
            .filter(|b| slot_at(r, *b) == r + POOL_SLOTS)
            .collect();
        assert!(
            !low.is_empty() && !high.is_empty(),
            "rank {r}: both carriages are satisfiable: {low:?} {high:?}"
        );
        assert!(
            low.iter().all(|b| !high.contains(b)),
            "rank {r}: and they are disjoint — one slot per base"
        );
        assert_eq!(
            low.len() + high.len(),
            names + 1,
            "rank {r}: and together they are every candidate, so no base is \
             left over for a single site to fail on"
        );
    }

    // ── FIVE, REFUSED: THE OTHER EMPTY SET, AND IT IS NOT THIS BUCKET'S CLAIM.
    // An object carrying NEITHER slot for a rank also exhausts to nothing — but
    // that is a SHORTFALL, the side read backwards or the constant wrong, and
    // it would fire on a routine with ONE half-site where a cut contradiction
    // cannot. It is built here so the two are told apart by construction: the
    // witness in part ONE carries a half-site for every want, and part THREE
    // shows each of them is individually explicable.
    let missing = object(vec![], lo(6, 9, true), both(3, 4, false));
    let rm = route(&missing);
    assert_eq!(rm.route["मक"], HalfRoute::ThirdWidened);
    assert_eq!(
        rm.bases["मक"],
        Vec::<usize>::new(),
        "an unsatisfiable want kills every candidate too — which is why the \
         witness above is NOT this shape"
    );
    assert_eq!(
        route(&object(
            both(2, 3, true),
            both(6, 9, true),
            both(3, 4, false)
        ))
        .bases["मक"],
        (0..=names).collect::<Vec<usize>>(),
        "THE CONTROL: with every want carried on both sides nothing is \
         narrowed at all, so the empty sets above are the OBJECT's refusals \
         and not the exhaustion refusing by default"
    );

    // ── SIX: AND THE ROUTING IS TOTAL, so the sweep's `match` cannot index a
    // key the router never wrote and no routine can be dropped from the
    // accounting by omission.
    for r in [&r0, &r1, &rk, &rm] {
        assert_eq!(
            r.route.keys().collect::<Vec<_>>(),
            extra.half_want.keys().collect::<Vec<_>>(),
            "every routine in the want is routed"
        );
    }
}
/// **EVERY SHAPE THE SECOND WIDENING'S ROUTER CAN BE HANDED, AND THIS TIME THE
/// ZEROS ARE MEASUREMENTS.**
///
/// The pair-side twin of the audit last cycle ran on the half-sites. Three of
/// this sweep's figures are reported ZERO with nothing ever built that made any
/// of them fire — `t1_corpus_wide_routines_no_frame` and
/// `t1_corpus_cut_extra_sites_unused_routine_unvisited`, which are the routine
/// count and the site count of ONE bucket, and
/// `t1_corpus_wide_routines_cut_unexplained`. A zero no test has made fire is
/// indistinguishable from a bucket nothing can reach, and last cycle one of the
/// half-site buckets turned out to be exactly that. **THE ANSWER HERE IS THE
/// OTHER ONE, AND FOR A STRUCTURAL REASON:** [`route_wide_sites`] has NO
/// fall-through arm. Its last arm is the widening itself, so all four buckets
/// are REACHABLE, every one of them fires below, and each of those zeros is a
/// claim about the corpus — no frameless routine with a two-name
/// `समम्`/`असमम्`/`बृहत्समम्` head is in reach today, and no object in reach
/// contradicts itself across its own sites.
///
/// **AND ONE COMMENT THAT SAID THE OPPOSITE OF THE TRUTH.** The frameless
/// bucket was documented as existing because "every candidate base would survive
/// vacuously and the ambiguity reported would be the census's own blindness" —
/// the same guess, the same way round, that the half-site ledger got wrong.
/// [`consistent_bases_of`] keeps a base only when the object carries every pair
/// it predicts, so a non-empty want against an EMPTY have kills ALL `names + १`
/// candidates: the routine would be reported UNEXPLAINED, in the RED direction.
/// Part TWO asks the exhaustion the frameless shape directly and reads the empty
/// set back, which is why the frameless arm is decided BEFORE it runs.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
///
/// `none` must stay DISTINCT from `ambiguous`: an EMPTY candidate set explains
/// NOTHING and a set of `k ≥ २` explains too much, and collapsing them would let
/// a census that explains nothing read as one that explains too much. Asserted
/// both ways round, on two objects that differ only in what they carry.
///
/// A frameless routine must route `NoFrame` and must NOT be exhausted — if it
/// were, it would land in `none` beside the routines whose object genuinely
/// contradicts itself, and `wide_none`'s zero would mean "no frameless routine"
/// instead of "no contradictory object".
///
/// And a routine somebody else's recovery already visits must not be exhausted
/// either, or the same routine is recovered twice and counted twice.
///
/// Nothing here pushes a message or bumps a counter: every assertion reads what
/// [`route_wide_sites`] answered. Its own fixture, and its own object.
#[test]
fn the_wide_site_router_names_each_shape_and_every_bucket_is_reachable() {
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
    यदि अ समम् ई आदि
        प्रत्यागमनम् १ ।
    इति
    यदि आ समम् इ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 10, "प, अ … ऌ — ten names, so eleven candidate cuts");
    let extra = extra_pairs("म", &routines);
    assert_eq!(
        (extra.claimed, extra.half_claimed),
        (2, 0),
        "the harvest is REAL and it is two-NAME sites only: no numeral on either \
         side, so `cut_halves` contributes nothing and the base counts below are \
         the pairs' own"
    );
    assert_eq!(
        extra.want.keys().collect::<Vec<_>>(),
        vec!["मक"],
        "one routine in the want"
    );
    let by_word = extra.want["मक"].clone();
    assert_eq!(
        by_word.get("समलङ्घनम्").map(Vec::as_slice),
        Some([(1usize, 4usize), (2, 3)].as_slice()),
        "`अ समम् ई` is ranks (१, ४) and `आ समम् इ` is (२, ३), in WRITTEN order \
         — `समम्` does not exchange its operands (`ir.t1:2186`)"
    );

    // **THE THREE OBJECTS, AND WHAT EACH ONE PINS.** [`slot_at`] sends rank १ to
    // slot १ once `b > १` and to slot ९ otherwise, rank ४ to slot ४ once
    // `b > ४`, rank २ to slot २ once `b > २`, rank ३ to slot ३ once `b > ३`. So
    // over the eleven candidates the two wanted pairs come out:
    //   b=०,१  (९,१२) (१०,११)      b=३    (१,१२) (२,११)
    //   b=२    (१,१२) (१०,११)      b=४    (१,१२) (२,३)
    //   b≥५    (१,४)  (२,३)
    // which gives one object per bucket, from the SAME want.
    let have = |pairs: Vec<(usize, usize)>| -> BTreeMap<&'static str, Pairs> {
        [(
            "समलङ्घनम्",
            [("मक".to_string(), pairs)].into_iter().collect::<Pairs>(),
        )]
        .into_iter()
        .collect()
    };
    let one_base = have(vec![(1, 12), (2, 3)]);
    let two_bases = have(vec![(9, 12), (10, 11)]);
    // **AND THE CONTRADICTORY ONE, WHICH IS NOT MERELY AN EMPTY OBJECT.** Each
    // wanted pair IS explained here — `(१,४)` by any `b ≥ ५` and `(१०,११)` by any
    // `b ≤ २` — and no single cut explains BOTH. That is the shape `wide_none`
    // exists to name, and it is the one the residue reads green.
    let no_base = have(vec![(1, 4), (10, 11)]);

    let names_of = |r: &str| {
        routines
            .iter()
            .find(|q| format!("म{}", q.name) == *r)
            .map_or(0, |q| q.slots.len())
    };
    let route = |residue: &Pairs,
                 paired: &Pairs,
                 w: &Windows,
                 eh: &BTreeMap<&'static str, Pairs>,
                 hw: &BTreeMap<String, ByWordHalf>,
                 hh: &BTreeMap<&'static str, HalfPairs>| {
        route_wide_sites(
            "fixture.t1",
            &extra.want,
            hw,
            residue,
            paired,
            w,
            &Pairs::new(),
            eh,
            hh,
            &names_of,
        )
    };
    let none = Pairs::new();
    let nohalf = BTreeMap::new();
    let nohalfhave = BTreeMap::new();
    let framed: Windows = [("मक".to_string(), (0u64, 80u64))].into_iter().collect();
    let nowin = Windows::new();
    let one: Pairs = [("मक".to_string(), vec![(3usize, 4usize)])]
        .into_iter()
        .collect();

    // ── ONE: THE WIDENING'S OWN SHAPE, RECOVERED. No `न्यूनम्` head, not
    // `pairable`, a frame laid out — and the object pins base ४ and nothing
    // else. The route that says `Widened` is the route that carries the cut.
    let r1 = route(&none, &none, &framed, &one_base, &nohalf, &nohalfhave);
    assert_eq!(r1.route["मक"], WideRoute::Widened);
    assert_eq!(r1.bases["मक"], vec![4usize]);
    assert_eq!((r1.recovered, r1.recovered_sites), (1, 2));
    assert!(
        r1.none.is_empty() && r1.ambiguous.is_empty() && r1.no_frame.is_empty(),
        "a recovered routine is in no other bucket: {:?} {:?} {:?}",
        r1.none,
        r1.ambiguous,
        r1.no_frame
    );

    // ── TWO: THE NO-FRAME BUCKET FIRES, AND THE REASON THE LEDGER GAVE FOR IT
    // IS THE OPPOSITE OF THE TRUTH. It NAMES the routine and its site count, and
    // it recovers NOTHING — and the witness that it must not exhaust is right
    // here: handed the same want with an empty have, the exhaustion returns the
    // EMPTY set, not all eleven candidates. Vacuous survival was the guess; a
    // total kill is the fact, so exhausting a frameless routine would file this
    // census's blindness as the object's contradiction.
    let frameless = route(&none, &none, &nowin, &BTreeMap::new(), &nohalf, &nohalfhave);
    assert_eq!(frameless.route["मक"], WideRoute::NoFrame);
    assert_eq!(
        frameless.no_frame.len(),
        1,
        "the bucket NAMES it: {:?}",
        frameless.no_frame
    );
    assert!(
        frameless.no_frame[0].contains("fixture.t1: मक")
            && frameless.no_frame[0].contains("2 site(s)"),
        "and the note says which routine and how many sites: {}",
        frameless.no_frame[0]
    );
    assert!(
        frameless.bases.is_empty() && frameless.none.is_empty(),
        "and NO cut is recovered for it and NOTHING is reported unexplained — \
         the whole point of deciding this arm first: {:?} {:?}",
        frameless.bases,
        frameless.none
    );
    assert_eq!(
        consistent_bases_of(
            &cut_groups(
                "मक",
                NO_LT_SITES,
                &Pairs::new(),
                Some(&by_word),
                &BTreeMap::new()
            ),
            &[],
            names
        ),
        Vec::<usize>::new(),
        "THE WITNESS: an empty have does not let every base survive vacuously, \
         it kills all eleven — so the frameless bucket exists in the RED \
         direction and the comment that said `ambiguity` had it backwards"
    );

    // ── THREE: `none` FIRES, THROUGH THE SWEEP'S OWN DECISION. Same want, an
    // object each of whose pairs is explicable and whose pairs share no cut.
    let unexplained = route(&none, &none, &framed, &no_base, &nohalf, &nohalfhave);
    assert_eq!(unexplained.route["मक"], WideRoute::Widened);
    assert!(unexplained.bases["मक"].is_empty());
    assert_eq!(
        unexplained.none.len(),
        1,
        "the bucket NAMES it: {:?}",
        unexplained.none
    );
    assert!(
        unexplained.none[0].contains("NO cut position explains its 2")
            && unexplained.none[0].contains("10 declared name(s)"),
        "and the note carries the want and the have, or a red here says nothing \
         about which reading is wrong: {}",
        unexplained.none[0]
    );
    assert_eq!((unexplained.recovered, unexplained.recovered_sites), (0, 0));

    // ── FOUR: `ambiguous` FIRES, and THE REFUSAL THAT MATTERS — the two are
    // DISTINCT. An EMPTY candidate set is UNEXPLAINED and a set of `k ≥ २` is
    // AMBIGUOUS; if they were one bucket, a census that explains nothing would
    // read as one that explains too much, and its zero would mean neither.
    let ambiguous = route(&none, &none, &framed, &two_bases, &nohalf, &nohalfhave);
    assert_eq!(ambiguous.bases["मक"], vec![0usize, 1]);
    assert_eq!(
        ambiguous.ambiguous.len(),
        1,
        "the bucket NAMES it: {:?}",
        ambiguous.ambiguous
    );
    assert!(
        ambiguous.ambiguous[0].contains("2 of the 11 candidate cut position(s)")
            && ambiguous.ambiguous[0].contains("AMBIGUOUS"),
        "{}",
        ambiguous.ambiguous[0]
    );
    assert!(
        ambiguous.none.is_empty(),
        "A SET OF TWO IS NOT AN EMPTY SET. Reported unexplained as well as \
         ambiguous, `wide_none`'s zero would be a claim about ambiguity: {:?}",
        ambiguous.none
    );
    assert!(
        unexplained.ambiguous.is_empty(),
        "AND THE OTHER WAY ROUND — an empty set must not be reported ambiguous, \
         which would let a routine nothing explains read as one several cuts \
         fit: {:?}",
        unexplained.ambiguous
    );
    assert_eq!((ambiguous.recovered, ambiguous.recovered_sites), (0, 0));

    // ── FIVE: AND THE HALF-SITE FOLD REACHES THROUGH THE ROUTER, which now owns
    // the two counters that say so. `प` holds rank ०, and `slot_at(०, b)` is the
    // one prediction that separates `b = ०` from `b = १`: slot ८ before the cut
    // moves and slot ० after. So one name-against-constant triple takes the
    // AMBIGUOUS pair of candidates down to exactly one, which is the whole
    // warrant for folding the constants in at all.
    let half_want: BTreeMap<String, ByWordHalf> = [(
        "मक".to_string(),
        [("समलङ्घनम्", vec![(0usize, 5i128, true)])]
            .into_iter()
            .collect(),
    )]
    .into_iter()
    .collect();
    let half_have: BTreeMap<&'static str, HalfPairs> = [(
        "समलङ्घनम्",
        [("मक".to_string(), vec![(0usize, 5i128, true)])]
            .into_iter()
            .collect::<HalfPairs>(),
    )]
    .into_iter()
    .collect();
    let folded = route(&none, &none, &framed, &two_bases, &half_want, &half_have);
    assert_eq!(folded.bases["मक"], vec![1usize]);
    assert_eq!(
        (folded.narrowed_by_half, folded.by_half),
        (1, 1),
        "the constant narrowed it AND brought it to one; a zero here says the \
         fold is decoration"
    );
    assert!(
        folded.ambiguous.is_empty() && folded.recovered == 1,
        "and the routine moves OUT of the ambiguous bucket, not into a second \
         one: {:?}",
        folded.ambiguous
    );

    // ── SIX: THE TWO BUCKETS THAT BELONG TO SOMEBODY ELSE'S RECOVERY, in the
    // order the chain decides them — `Folded` wins over everything and
    // `AlreadyPaired` over the widening. Neither is EXHAUSTED: `cut_bases`
    // already folds these sites in for the first and there is no cut to recover
    // for the second, so a base inserted here would be the same routine
    // recovered twice and its sites counted twice.
    for (res, pai, want) in [
        (&one, &none, WideRoute::Folded),
        (&one, &one, WideRoute::Folded),
        (&none, &one, WideRoute::AlreadyPaired),
    ] {
        let r = route(res, pai, &framed, &one_base, &nohalf, &nohalfhave);
        assert_eq!(r.route["मक"], want);
        assert!(
            r.bases.is_empty() && r.recovered == 0 && r.recovered_sites == 0,
            "{want:?} must not be exhausted: {:?}",
            r.bases
        );
    }

    // ── SEVEN: AND EVERY BUCKET IS REACHABLE — enumerated, not argued, over all
    // eight combinations of the three memberships. Unlike
    // [`HalfRoute::Unseen`] there is no fall-through arm to prove empty: the
    // last arm is the widening, so the four are exhaustive AND all four have
    // input. That is what makes the three zeros this test came for
    // MEASUREMENTS — `t1_corpus_wide_routines_no_frame`,
    // `t1_corpus_cut_extra_sites_unused_routine_unvisited` and
    // `t1_corpus_wide_routines_cut_unexplained` say something about the corpus.
    let mut seen = std::collections::BTreeSet::new();
    for res in [false, true] {
        for pai in [false, true] {
            for fr in [false, true] {
                let r = route(
                    if res { &one } else { &none },
                    if pai { &one } else { &none },
                    if fr { &framed } else { &nowin },
                    &one_base,
                    &nohalf,
                    &nohalfhave,
                );
                // ── AND THE ROUTING IS TOTAL THE OTHER WAY TOO: one entry per
                // routine handed in, so the `match` at the call site cannot
                // index a key the router never wrote and no site can be lost by
                // the router simply omitting it.
                assert_eq!(
                    r.route.keys().collect::<Vec<_>>(),
                    extra.want.keys().collect::<Vec<_>>(),
                    "residue={res} paired={pai} framed={fr}: every routine in \
                     the want is routed"
                );
                seen.insert(r.route["मक"]);
            }
        }
    }
    assert_eq!(
        seen.len(),
        4,
        "all four buckets were exercised, so the enumeration is not green on one \
         arm eight times: {seen:?}"
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
    // **THE DEPTH, PER ROUTINE AND NOT JUST ITS MAXIMUM.** A single `deepest`
    // number has two states where the truth has three: it cannot tell "one
    // routine reached 6" from "half the corpus did", and it names no routine,
    // so a cycle that moves it has to re-run a hand probe to find out what
    // moved. Keyed `file::routine` because routine names repeat across
    // modules. The population is the routines REACHED BY AN SP ACCESS — a
    // routine that spills but never addresses `sp` is invisible to this whole
    // sweep, and that is the same population every band above is counted over.
    let mut spill_depth_by_routine: BTreeMap<String, usize> = BTreeMap::new();
    let mut deepest_routine = String::new();

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
            if frame.num_spills > deepest_spill || deepest_routine.is_empty() {
                deepest_spill = frame.num_spills;
                deepest_routine = format!("{file}: `{routine}`");
            }
            spill_depth_by_routine.insert(format!("{file}::{routine}"), frame.num_spills);
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

    // THE DEPTH DISTRIBUTION, so a later move in `deepest` is attributable
    // without a hand probe: how many routines sit at each depth, and which
    // routine holds the maximum.
    let mut depth_histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for d in spill_depth_by_routine.values() {
        *depth_histogram.entry(*d).or_default() += 1;
    }
    let routines_reached = spill_depth_by_routine.len();
    let routines_spilling = spill_depth_by_routine.values().filter(|d| **d > 0).count();
    println!(
        "deepest num_spills {deepest_spill} in {deepest_routine}; {routines_spilling} of \
         {routines_reached} routines reached by an SP access spill; depths {depth_histogram:?}"
    );
    for (d, n) in &depth_histogram {
        println!("METRIC t1_corpus_spill_depth_{d}_routines {n}");
    }
    println!("METRIC t1_corpus_routines_addressed_off_sp {routines_reached}");
    println!("METRIC t1_corpus_routines_spilling {routines_spilling}");

    // **THE NEW INSTRUMENT CHECKED AGAINST THE OLD ONE.** The histogram and
    // `deepest_spill` are two readings of the same map, so they cannot differ
    // for a legitimate reason — a difference is the histogram having lost
    // routines (or `deepest_spill` having been updated somewhere the map was
    // not), and the depth pin below would then be asserted against a number no
    // longer backed by the distribution printed beside it.
    assert_eq!(
        depth_histogram.values().sum::<usize>(),
        routines_reached,
        "the depth histogram counts {} routines but {routines_reached} were \
         reached by an SP access; the two are built from the same map",
        depth_histogram.values().sum::<usize>()
    );
    assert_eq!(
        depth_histogram.keys().last().copied().unwrap_or(0),
        deepest_spill,
        "the histogram's deepest bucket and `deepest_spill` disagree: {:?} vs \
         {deepest_spill}. Both read `frame.num_spills` at the same site",
        depth_histogram.keys().last()
    );

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
    // WHETHER IT IS PUSHED AT ALL.**
    //
    // THE PIN IS `1`, IT WAS BRIEFLY `6`, AND THE `6` BELONGED TO A CHANGE THAT
    // IS NO LONGER IN THE CORPUS.
    //
    // Measured 2026-09-29, this tree, BOTH PROFILES (debug and release give
    // byte-identical figures, so the profile is not a variable here):
    //
    //     modules 20, sp accesses 17449, deepest num_spills 1
    //     bands {incoming-argument: 1, local: 13041, return-address: 879,
    //            saved: 3494, spill: 34}
    //     depths {0: 862, 1: 17}
    //
    // THAT REPRODUCES THE 2026-09-25 PROBE EXACTLY: 17 routines spilling, every
    // one a `<module>खण्डवृद्धिः`, none deeper than a single slot. The probe was
    // right about the corpus it measured.
    //
    // **THE PARAGRAPH THAT STOOD HERE SAID THE OPPOSITE, AND IT IS REFOUNDED
    // RATHER THAN DELETED, BECAUSE ITS PREMISE LEFT THE TREE.** It read: *"THE
    // PIN WAS 1 AND THE CORPUS ANSWERED 6 … THAT CLAIM IS FALSIFIED"*, and then
    // attributed the `6` away from `e3fe5327`'s inline capacity test — *"deepest
    // 6 and spill band 144 at BOTH revisions … 36 new local accesses, ZERO new
    // spill slots"*. On this tree that attribution does not hold:
    //
    //   - `4d9ac73d`'s `ir.t1` is BYTE-IDENTICAL to the one in the tree now
    //     (`git diff --stat` empty; that commit touched only `.loop/`), and on
    //     it the corpus answers deepest **1**, spill band **34**.
    //   - the merge of `agent/tick` (`8769a3f7`), which differs from this tree in
    //     `ir.t1` and nothing else under `crates/sadhana-t1/src`, answered
    //     deepest **6** and passed this assertion.
    //   - the hunk adds 22 new `चरः` temporaries inside one block, which is what
    //     deepens a frame.
    //
    // So `6` came WITH the inline capacity test. It was restored out in
    // `24d9bb58` because it makes the 21-source self-hosted build refuse
    // `मध्यरूप` (`W-330`), and the pin follows the corpus it actually has.
    //
    // **THIS PIN READS `6` AGAIN WHEN `W-330` LANDS, and that is not a licence to
    // keep lowering it.** The number is 1 because the deeper code is not here; it
    // is not evidence that `8·(num_spills + k)` is safe past `num_spills ∈ {0, 1}`
    // — the merge run is the only thing that has ever exercised that, and it is
    // recorded above so the next attempt starts from a measurement.
    //
    // AN UPPER BOUND, NOT AN EQUALITY — owner ruling Q3, 2026-09-29.
    //
    // The equality form red on an IMPROVEMENT: restoring `ir.t1` out under
    // `W-330` dropped the depth 6 → 1 and the pin, written `== 6`, failed on a
    // corpus that had got shallower. A bound that only refuses a RISE keeps the
    // property worth having — a regression that needs a deeper spill region
    // cannot land unnoticed — while an optimisation that needs fewer slots
    // passes without a ledger entry.
    //
    // **AND A FALLING BOUND MAKES THE DISPLACEMENT LESS CHECKED, NOT MORE.** At
    // `deepest ≤ 1` nothing in the corpus exercises `8·(num_spills + k)` past
    // `num_spills ∈ {0, 1}`. The merge `8769a3f7` reached 6 and is the only
    // thing that ever has; those figures are in the margin above so the next
    // attempt starts from a measurement. `W-330` must RAISE this bound to 6 when
    // it lands, deliberately and with its own justification — which is the point
    // of a bound over an equality: coming down is free, going up is a decision.
    assert!(
        deepest_spill <= 1,
        "the deepest frame in the corpus uses {deepest_spill} spill slots, above \
         the bound of 1. Some routine now needs a deeper spill region and the \
         `8·(num_spills + k)` displacement is being exercised past where it has \
         ever been checked — read the histogram above for WHICH routines moved, \
         then raise this bound NAMING the change that raised it. It came down \
         from 6 when `W-330` restored `ir.t1`; 6 is what the merge measured"
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

/// **THE CASES THAT MUST STILL BE REFUSED, for the residue over the ambiguous
/// half-routines.**
#[test]
fn an_ambiguous_half_routines_residue_holds_and_a_recovered_one_stays_out() {
    // THIS TEST'S OWN SOURCE. The parameter takes rank ० and the nine `चरः`
    // locals ranks १ to ९ in declaration order (`ir.t1:1320`), so `क` at rank १
    // and `झ` at rank ९ are EIGHT APART — one residue bucket, and the case the
    // floor exists for. Both are compared to the SAME constant on the SAME
    // side, which is the only way two ranks can collide here.
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    चरः ग ॱॱ न६४ भवति ० ।
    चरः घ ॱॱ न६४ भवति ० ।
    चरः ङ ॱॱ न६४ भवति ० ।
    चरः च ॱॱ न६४ भवति ० ।
    चरः छ ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    चरः झ ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि झ समम् ५ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1, "{routines:?}");
    assert_eq!(
        routines[0].slots.len(),
        10,
        "the parameter and nine locals: {:?}",
        routines[0].slots
    );
    assert_eq!(
        (routines[0].slots[1].as_str(), routines[0].slots[9].as_str()),
        ("क", "झ"),
        "`क` is rank १ and `झ` is rank ९ — eight apart, so ONE residue bucket"
    );
    let extra = extra_pairs("म", &routines);
    let want = extra
        .half_want
        .get("मकार्यम्")
        .and_then(|w| w.get("समलङ्घनम्"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        want,
        vec![(1usize, 5i128, true), (9usize, 5i128, true)],
        "both heads predict the constant ५ with the NAME in the करण, at ranks १ \
         and ९: {:?}",
        extra.half_want
    );
    assert_eq!(
        half_residues(&want),
        vec![(1usize, 5i128, true), (1usize, 5i128, true)],
        "and modulo eight they are the SAME triple — which is why the claim is \
         a FLOOR of two and not two separate pins"
    );

    let ambiguous: std::collections::BTreeSet<String> =
        ["मकार्यम्".to_string()].into_iter().collect();
    let have = |w: &'static str, v: Vec<Half>| -> BTreeMap<&'static str, HalfPairs> {
        [(w, [("मकार्यम्".to_string(), v)].into_iter().collect())]
            .into_iter()
            .collect()
    };

    // ── ONE: THE OBJECT CARRIES BOTH, ON OPPOSITE SIDES OF THE CUT. A cut at
    // base ९ puts rank १ at slot १ and rank ९ at slot १७ (`slot_at`), and the
    // residue claim is exactly that it does not need to know that. Green, or
    // every red below is a reader that reads nothing.
    assert!(
        half_residue_shortfalls(
            &ambiguous,
            &extra.half_want,
            &have("समलङ्घनम्", vec![(1, 5, true), (17, 5, true)])
        )
        .is_empty(),
        "slots १ and १७ are both congruent to १ modulo eight"
    );

    // ── TWO: THE WRONG RESIDUE. Slot २ is rank २'s, not rank १'s, under any
    // cut at all — the one thing the residue can still say about a slot number.
    let bad = half_residue_shortfalls(
        &ambiguous,
        &extra.half_want,
        &have("समलङ्घनम्", vec![(2, 5, true), (17, 5, true)]),
    );
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(
        bad[0].contains("congruent to 1 modulo eight") && bad[0].contains("करण"),
        "and the refusal names the residue and the side: {}",
        bad[0]
    );

    // ── THREE: THE WRONG SIDE, WHICH IS WHAT SEPARATES THIS CLAIM FROM THE
    // CANDIDATE CUT SET. `५ समम् क` names the same slot and the same constant
    // and is a DIFFERENT branch; the side does not move with the cut, so a
    // reader that dropped it would assert only what the cut set already said —
    // and `swap_the_half_sides` is the corpus-wide form of exactly this.
    let bad = half_residue_shortfalls(
        &ambiguous,
        &extra.half_want,
        &have("समलङ्घनम्", vec![(1, 5, false), (17, 5, false)]),
    );
    assert_eq!(bad.len(), 1, "the exchanged side is caught: {bad:?}");

    // ── FOUR: THE WRONG WORD. Matched only against the half-sites the object
    // carries under the SAME branch word, as everywhere in this file.
    let bad = half_residue_shortfalls(
        &ambiguous,
        &extra.half_want,
        &have("विषमलङ्घनम्", vec![(1, 5, true), (17, 5, true)]),
    );
    assert_eq!(bad.len(), 1, "the wrong word is caught: {bad:?}");

    // ── FIVE, REFUSED: A ROUTINE WHOSE CUT *IS* RECOVERED. Its single base was
    // narrowed out of THESE VERY HALF-SITES (`cut_halves`), so asserting them
    // against the residue that base implies is the instrument agreeing with
    // itself — circular, and vacuously green. It is kept out by NOT being in
    // the ambiguous set, and an EMPTY object must not make it red.
    assert!(
        half_residue_shortfalls(
            &std::collections::BTreeSet::new(),
            &extra.half_want,
            &BTreeMap::new()
        )
        .is_empty(),
        "a routine whose cut position is recovered keeps the exact slot the \
         recovery already reported and makes NO residue claim here; reading it \
         red would read the compiler red for being right"
    );

    // ── SIX, REFUSED: THE FLOOR IS PER DISTINCT RESIDUE TRIPLE, AND TWO RANKS
    // EIGHT APART SHARE ONE. `यावत्` rotation emits one source comparison
    // twice, so the object may carry the bucket MORE often than the source
    // demands and only going SHORT is a miss.
    assert!(
        half_residue_shortfalls(
            &ambiguous,
            &extra.half_want,
            &have("समलङ्घनम्", vec![(1, 5, true), (9, 5, true), (17, 5, true)]),
        )
        .is_empty(),
        "three carried against two wanted is the rotation, not a defect"
    );
    // AND THE CONTROL, or the bucket is not being counted at all: ONE carried
    // against the two the source writes IS short, and it is short ONCE —
    // the two wants are one bucket, so a reader that reported two would be
    // pinning ranks the residue cannot tell apart.
    let bad = half_residue_shortfalls(
        &ambiguous,
        &extra.half_want,
        &have("समलङ्घनम्", vec![(1, 5, true)]),
    );
    assert_eq!(
        bad.len(),
        1,
        "one carried against two wanted is short, and short ONCE: {bad:?}"
    );
    assert!(
        bad[0].contains("wants 2 ") && bad[0].contains("carries 1 "),
        "and the refusal says the floor it missed, not merely that it missed: {}",
        bad[0]
    );

    // ── SEVEN, REFUSED: A CONSTANT OUTSIDE THE `addi` IMMEDIATE. The emitter
    // materialises it with `उपरिभारः` (`yantrotsarjana.t1:898`), so the object
    // carries no `Origin::Literal` and the site never enters `half_want` — it
    // stays NAMED in `half_out_of_range` and can never come out residue-red,
    // however empty the object is.
    let wide = src.replace("क समम् ५", "क समम् ४०९६");
    let wide_routines = source_routines(&wide);
    let wide_extra = extra_pairs("म", &wide_routines);
    assert_eq!(
        (wide_extra.half_claimed, wide_extra.half_out_of_range.len()),
        (1, 1),
        "४०९६ is outside `IMMEDIATE` and is NAMED, not claimed; `झ समम् ५` \
         still is: {:?}",
        wide_extra.half_out_of_range
    );
    let bad = half_residue_shortfalls(&ambiguous, &wide_extra.half_want, &BTreeMap::new());
    assert_eq!(
        bad.len(),
        1,
        "an EMPTY object is short for the ONE site that entered `half_want` and \
         for that site only — the refused ४०९६ adds no second red: {bad:?}"
    );
    assert!(
        bad[0].contains("the constant 5"),
        "and the red is the ५, never the ४०९६: {}",
        bad[0]
    );
}

/// **THE HOLE THE TOTAL LOOKED LIKE IT WAS HIDING, AND THE ONE IT REALLY
/// HIDES.** Cycle 1005 read `112 sites, the exchange bites 105` and named
/// seven sites the swap cannot reach. Measured, that subtraction is a currency
/// error: [`half_residue_shortfalls`] reports one line per DISTINCT residue
/// bucket, the corpus's 112 ambiguous sites are 105 buckets, and every bucket
/// is bitten. The seven are sites sharing a bucket with another site, which
/// `an_ambiguous_half_routines_residue_holds_and_a_recovered_one_stays_out`
/// already reads as one FLOOR.
///
/// The shape that WOULD be a hole is a different one, the corpus has none of
/// it, and a detector that has never fired on anything is a reader that reads
/// nothing. So here is the shape, built: one residue bucket compared against
/// one constant from BOTH sides under ONE word. The exchange rewrites each
/// want into the other's, the multiset does not move, and the site must come
/// out of the COVERED figure while keeping its claim.
#[test]
fn a_half_site_whose_twin_is_on_the_other_side_is_uninformative_not_covered() {
    // THIS TEST'S OWN SOURCE AND ITS OWN LOADER. `प` takes rank ० and the two
    // `चरः` locals ranks १ and २ (`ir.t1:1320`), so `क` is rank १ under any
    // reading and the bucket below is `(१, ५, ·)`.
    let both = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ५ समम् क आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let ambiguous: std::collections::BTreeSet<String> =
        ["मकार्यम्".to_string()].into_iter().collect();
    let wants = |src: &str| extra_pairs("म", &source_routines(src)).half_want;

    let want = wants(both);
    assert_eq!(
        want.get("मकार्यम्").and_then(|w| w.get("समलङ्घनम्")),
        Some(&vec![(1usize, 5i128, true), (1usize, 5i128, false)]),
        "`क समम् ५` and `५ समम् क` are ONE residue bucket on TWO sides — the \
         loader must read the second's name out of the अपादान: {want:?}"
    );
    let cancelled = half_residue_buckets_of(&ambiguous, &want);
    assert_eq!(
        (cancelled.buckets, cancelled.cancelled, cancelled.shared),
        (2, 2, 0),
        "two buckets, BOTH cancelled — each is the other's twin, wanted as \
         often — and no site shares a bucket: {cancelled:?}"
    );
    assert!(
        cancelled.cancelled_named[0].contains("UNINFORMATIVE")
            && cancelled.cancelled_named[0].contains("OTHER side"),
        "and the line says why, not merely that: {}",
        cancelled.cancelled_named[0]
    );

    // ── AND THE EXCHANGE AGREES, which is the only reason to believe the
    // detector. `swap_the_half_sides` flips the side of every carried half
    // under these words; here the carried multiset is its own mirror, so the
    // shortfall reader is green on the swapped object and the site really is
    // one no mutation in this file can bite.
    let have = |w: &'static str, v: Vec<Half>| -> BTreeMap<&'static str, HalfPairs> {
        [(w, [("मकार्यम्".to_string(), v)].into_iter().collect())]
            .into_iter()
            .collect()
    };
    let honest = vec![(1usize, 5i128, true), (1usize, 5i128, false)];
    let swapped: Vec<Half> = honest.iter().map(|(s, v, k)| (*s, *v, !*k)).collect();
    assert!(
        half_residue_shortfalls(&ambiguous, &want, &have("समलङ्घनम्", honest.clone())).is_empty(),
        "the honest object is green or every red below reads nothing"
    );
    assert!(
        half_residue_shortfalls(&ambiguous, &want, &have("समलङ्घनम्", swapped)).is_empty(),
        "AND SO IS THE EXCHANGED ONE — the detector and the mutation must name \
         the same sites, or one of them is wrong"
    );

    // ── ONE, REFUSED: A TWIN UNDER A DIFFERENT WORD. The match is word-for-word
    // everywhere in this file and `swap_the_half_sides` exchanges each word's
    // operands in place, never moving a site between words, so `क समम् ५` and
    // `५ असमम् क` do NOT cancel — and the swap does bite them.
    let other_word = wants(&both.replace("५ समम् क", "५ असमम् क"));
    let c = half_residue_buckets_of(&ambiguous, &other_word);
    assert_eq!(
        (c.buckets, c.cancelled),
        (2, 0),
        "one bucket under each of two words cancels nothing: {c:?}"
    );
    let bit = half_residue_shortfalls(
        &ambiguous,
        &other_word,
        &have("समलङ्घनम्", vec![(1, 5, false)]),
    );
    assert_eq!(
        bit.len(),
        2,
        "and the exchanged object is short BOTH — the समलङ्घनम् करण want and \
         the विषमलङ्घनम् अपादान want the object does not carry at all: {bit:?}"
    );

    // ── TWO, REFUSED: A TWIN IN A DIFFERENT ROUTINE. Halves are read per
    // routine label and no exchange carries a site across a frame.
    let split = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
सार्वजनिक वृत्तिः कार्यान्तरम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    यदि ५ समम् क आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let two_routines = wants(split);
    let both_labels: std::collections::BTreeSet<String> = ["मकार्यम्", "मकार्यान्तरम्"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let c = half_residue_buckets_of(&both_labels, &two_routines);
    assert_eq!(
        (c.buckets, c.cancelled),
        (2, 0),
        "the same bucket on the other side of a DIFFERENT frame cancels \
         nothing: {c:?}"
    );

    // ── THREE, REFUSED: A TWIN WANTED FEWER TIMES. Two करण wants against one
    // अपादान want leaves the swapped object asked for two where the source
    // demands one, so the swap can still bite the करण bucket; only the
    // अपादान bucket, whose twin outnumbers it, is cancelled. Calling both
    // cancelled would take a live site out of the figure.
    let lopsided = wants(&both.replace(
        "यदि ५ समम् क आदि",
        "यदि क समम् ५ आदि\n        प्रत्यागमनम् ३ ।\n    इति\n    यदि ५ समम् क आदि",
    ));
    assert_eq!(
        lopsided
            .get("मकार्यम्")
            .and_then(|w| w.get("समलङ्घनम्"))
            .map(Vec::len),
        Some(3),
        "three sites: two करण and one अपादान: {lopsided:?}"
    );
    let c = half_residue_buckets_of(&ambiguous, &lopsided);
    assert_eq!(
        (c.buckets, c.cancelled, c.cancelled_named.len()),
        (2, 1, 1),
        "ONE bucket cancelled and it is one SITE — the अपादान want, whose twin \
         is written twice: {c:?}"
    );
    assert!(
        c.cancelled_named[0].contains("अपादान"),
        "and it is the अपादान one, never the करण pair: {}",
        c.cancelled_named[0]
    );
    let bit = half_residue_shortfalls(
        &ambiguous,
        &lopsided,
        // The exchanged object: two अपादान where the source writes two करण,
        // one करण where it writes one अपादान.
        &have("समलङ्घनम्", vec![(1, 5, false), (1, 5, false), (1, 5, true)]),
    );
    assert_eq!(
        bit.len(),
        1,
        "and the swap bites exactly the bucket the detector did NOT cancel — \
         one करण carried against two wanted: {bit:?}"
    );
    assert!(
        bit[0].contains("wants 2 ") && bit[0].contains("करण"),
        "{}",
        bit[0]
    );

    // ── AND THE OTHER CURRENCY, which is what the corpus's seven really are.
    // `क` at rank १ and `झ` at rank ९ are EIGHT APART, so against the same
    // constant on the same side they are ONE bucket and TWO sites. Nothing is
    // cancelled — the swap bites the bucket — and the site count exceeds the
    // bucket count by exactly one. That difference is the 112 − 105, and it is
    // not a hole.
    let apart = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    चरः ग ॱॱ न६४ भवति ० ।
    चरः घ ॱॱ न६४ भवति ० ।
    चरः ङ ॱॱ न६४ भवति ० ।
    चरः च ॱॱ न६४ भवति ० ।
    चरः छ ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    चरः झ ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि झ समम् ५ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let eight = wants(apart);
    let c = half_residue_buckets_of(&ambiguous, &eight);
    assert_eq!(
        (c.buckets, c.shared, c.cancelled),
        (1, 1, 0),
        "two sites, ONE bucket, ONE of them sharing, and nothing cancelled: \
         {c:?}"
    );
    assert!(
        c.shared_named[0].contains("FLOOR of 2"),
        "and the line says the floor the two sites make, since that is the \
         whole of what the second one adds: {}",
        c.shared_named[0]
    );
    let bit = half_residue_shortfalls(&ambiguous, &eight, &have("समलङ्घनम्", vec![(1, 5, false)]));
    assert_eq!(
        bit.len(),
        1,
        "and the exchange bites that one bucket ONCE, which is why 105 bites \
         over 112 sites names no unbitten seven: {bit:?}"
    );
}

/// **THE EXACT-SLOT BLINDNESS CONTROL HAS FOUR STATES, AND THE SWEEP ASKED IT
/// AS A PER-FILE BOOLEAN.**
///
/// `if file_half_exact > 0 && swapped_short.is_empty()` — two states, and the
/// other two were folded into the silent branch: a module with NO `pairable`
/// half-site (nothing to ask, reported as a green) and a module whose every
/// bucket is UNINFORMATIVE (green BY ARITHMETIC, which the old guard would have
/// reported as a reader that reads nothing the moment the corpus grew one).
/// [`ResidueBuckets`] has had the cancellation arm one altitude down since the
/// cycle that measured it; this control had none.
///
/// All four arms of [`ExactBite`] are made to fire here, on wants read by this
/// test's OWN loader out of this test's own source, and the object handed to the
/// decision is always the honest one put through the SAME side flip
/// [`swap_the_half_sides`] applies — so the arm that fires is the arm the sweep
/// would reach and not one hand-written into existence.
///
/// **THE CASES THAT MUST STILL BE REFUSED.** A twin under a DIFFERENT word does
/// not cancel; a twin in a DIFFERENT routine does not cancel; a twin wanted
/// FEWER times stays covered; and a module with a bucket the swap DOES bite
/// stays out of BOTH new arms, however many of its other buckets are cancelled —
/// which is why `Moved` is decided first.
#[test]
fn the_exact_slot_blindness_control_names_four_states_and_counts_the_cancelled_out() {
    // THIS TEST'S OWN LOADER. A routine is [`pairable`] exactly when its frame
    // is as many slots wide as it declares names, so the window is DERIVED from
    // the declaration count rather than pinned — a source that grows a local
    // stays pairable and the test keeps asking what it says it asks.
    let load = |src: &str| -> (Pairs, BTreeMap<String, ByWordHalf>) {
        let routines = source_routines(src);
        let mut windows = Windows::new();
        let mut seed = Pairs::new();
        for r in &routines {
            let label = format!("म{}", r.name);
            windows.insert(label.clone(), (0u64, (r.slots.len() * 8) as u64));
            seed.insert(label, Vec::new());
        }
        let paired = pairable(&seed, &routines, "म", &windows);
        assert_eq!(
            paired.len(),
            routines.len(),
            "the loader must hand the decision a PAIRED routine, or every arm \
             below is `NoSites` for the wrong reason: {paired:?}"
        );
        (paired, extra_pairs("म", &routines).half_want)
    };
    let obj = |rows: Vec<(&'static str, &str, Vec<Half>)>| -> BTreeMap<&'static str, HalfPairs> {
        let mut out: BTreeMap<&'static str, HalfPairs> = BTreeMap::new();
        for (w, r, v) in rows {
            out.entry(w).or_default().insert(r.to_string(), v);
        }
        out
    };
    // THE MUTATION ITSELF, read off the object rather than written out.
    // `swap_the_half_sides` exchanges the operands of every one of the three
    // words, so what `emitted_halves_of` reads off the bitten object is the
    // honest halves with the SIDE bit flipped and nothing else moved.
    let mirror = |o: &BTreeMap<&'static str, HalfPairs>| -> BTreeMap<&'static str, HalfPairs> {
        o.iter()
            .map(|(w, p)| {
                (
                    *w,
                    p.iter()
                        .map(|(r, v)| {
                            (r.clone(), v.iter().map(|(s, c, k)| (*s, *c, !*k)).collect())
                        })
                        .collect(),
                )
            })
            .collect()
    };

    // ── ONE: THE BITE. `प` takes rank ० and the two `चरः` locals ranks १ and २
    // (`ir.t1:1320`), so `क` is rank १ and — the routine being pairable — slot
    // १. The honest object carries `(१, ५, करण)`; the swapped one carries the
    // अपादान and the claim must go short.
    let one = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let (paired, want) = load(one);
    assert_eq!(
        want.get("मकार्यम्").and_then(|w| w.get("समलङ्घनम्")),
        Some(&vec![(1usize, 5i128, true)]),
        "the loader reads ONE exact triple, the name in the करण: {want:?}"
    );
    let honest = obj(vec![("समलङ्घनम्", "मकार्यम्", vec![(1, 5, true)])]);
    let buckets = half_exact_buckets_of(&paired, &want);
    assert_eq!(
        (
            buckets.sites,
            buckets.buckets,
            buckets.shared,
            buckets.cancelled
        ),
        (1, 1, 0, 0),
        "one site, one bucket, nothing shared and nothing cancelled: {buckets:?}"
    );
    assert!(
        half_shortfalls(&paired, &want, &honest).is_empty(),
        "THE CONTROL: the honest object is green, or every red below is a \
         reader that reads nothing"
    );
    assert_eq!(
        exact_bite(&paired, &want, &buckets, &mirror(&honest)),
        ExactBite::Moved { short: 1 },
        "the side flip takes the one bucket short — this is the arm the corpus \
         has always answered, and the only one it has"
    );
    assert!(
        exact_bite(&paired, &want, &buckets, &mirror(&honest))
            .complaint(PAIRABLE_CLAIM)
            .is_none()
            && exact_bite(&paired, &want, &buckets, &mirror(&honest)).short() == 1,
        "and a bite is not a defect: it reports no complaint and one bite"
    );

    // ── TWO: NO SITES, WHICH IS NOTHING TO ASK AND NOT A GREEN. Two shapes
    // reach it and the old guard folded BOTH into the same silent branch as a
    // module whose sides really are read: a paired routine that writes no
    // name-against-constant head at all, and a routine that writes one but
    // which `pairable` REFUSES — the latter keeps the cut recovery, and reading
    // it blind would red the compiler for being right.
    let none: BTreeMap<String, ByWordHalf> = BTreeMap::new();
    assert_eq!(
        exact_bite(
            &paired,
            &none,
            &half_exact_buckets_of(&paired, &none),
            &honest
        ),
        ExactBite::NoSites,
        "a paired routine with no half-site is NoSites"
    );
    let unpaired = Pairs::new();
    assert_eq!(
        exact_bite(
            &unpaired,
            &want,
            &half_exact_buckets_of(&unpaired, &want),
            &obj(vec![])
        ),
        ExactBite::NoSites,
        "and so is a half-site in a routine `pairable` refuses, however empty \
         the object — the exact claim is not made of it at all"
    );
    assert!(
        ExactBite::NoSites.complaint(PAIRABLE_CLAIM).is_none() && ExactBite::NoSites.short() == 0,
        "and neither is a defect nor a bite"
    );

    // ── THREE: EVERY BUCKET UNINFORMATIVE. One slot against one constant from
    // BOTH sides under ONE word. The exchange rewrites each want into the
    // other's, the multiset does not move, and no mutation in this file can
    // bite the module. Reporting that as blindness would red the compiler for a
    // property of the SOURCE.
    let three = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ५ समम् क आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let (p3, w3) = load(three);
    let b3 = half_exact_buckets_of(&p3, &w3);
    assert_eq!(
        (b3.sites, b3.buckets, b3.shared, b3.cancelled),
        (2, 2, 0, 2),
        "two buckets, BOTH cancelled — each is the other's twin, wanted as \
         often: {b3:?}"
    );
    assert!(
        b3.cancelled_named[0].contains("UNINFORMATIVE")
            && b3.cancelled_named[0].contains("OTHER side"),
        "and the line says WHY, not merely that: {}",
        b3.cancelled_named[0]
    );
    let h3 = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (1, 5, false)],
    )]);
    assert!(
        half_shortfalls(&p3, &w3, &h3).is_empty()
            && half_shortfalls(&p3, &w3, &mirror(&h3)).is_empty(),
        "the honest object is green AND so is the mirrored one — the detector \
         and the mutation must name the same sites or one of them is wrong"
    );
    let v3 = exact_bite(&p3, &w3, &b3, &mirror(&h3));
    assert_eq!(
        v3,
        ExactBite::Uninformative { cancelled: 2 },
        "so the module is UNINFORMATIVE and its two sites come out of the \
         covered figure, not out of the claim"
    );
    assert!(
        v3.complaint(PAIRABLE_CLAIM).is_none() && v3.short() == 0,
        "and it is not a defect: {v3:?}"
    );

    // ── FOUR: BLIND, THE ONE DEFECT OF THE FOUR. An INFORMATIVE bucket — one
    // side written, the twin not — and an object that carries BOTH sides, so
    // the side flip leaves it covered. Nothing in this file can tell that
    // object from an honest one, and that is exactly what must be reported.
    let both_sides = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (1, 5, false)],
    )]);
    let v4 = exact_bite(&paired, &want, &buckets, &mirror(&both_sides));
    assert_eq!(
        v4,
        ExactBite::Blind {
            informative: 1,
            cancelled: 0,
            routines: 1
        },
        "one informative site in one routine, read green under the flip: {v4:?}"
    );
    let c = v4
        .complaint(PAIRABLE_CLAIM)
        .expect("the one arm that complains");
    assert!(
        c.contains("1 INFORMATIVE") && c.contains("the side is not being read"),
        "and the sentence names the informative figure and the defect: {c}"
    );

    // ── FIVE, REFUSED: A MODULE WITH A BUCKET THE SWAP DOES BITE STAYS OUT OF
    // BOTH NEW ARMS, and this is why `Moved` is decided BEFORE `Uninformative`.
    // Two करण wants against one अपादान want: the अपादान bucket is cancelled by
    // a twin written twice, the करण bucket is NOT — the swapped object is asked
    // for two where it carries one — so the module is sensitive and a cancelled
    // bucket excuses nothing on its own.
    let five = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि क समम् ५ आदि
        प्रत्यागमनम् ३ ।
    इति
    यदि ५ समम् क आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let (p5, w5) = load(five);
    let b5 = half_exact_buckets_of(&p5, &w5);
    assert_eq!(
        (b5.sites, b5.buckets, b5.shared, b5.cancelled),
        (3, 2, 1, 1),
        "three sites in two buckets — the करण bucket holds two of them, so it \
         is a FLOOR of two counted ONCE — and only the अपादान site, whose twin \
         outnumbers it, is cancelled: {b5:?}"
    );
    assert!(
        b5.cancelled_named[0].contains("अपादान") && b5.shared_named[0].contains("FLOOR of 2"),
        "the cancelled line is the अपादान one and the shared line says the \
         floor: {:?} {:?}",
        b5.cancelled_named,
        b5.shared_named
    );
    let h5 = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (1, 5, true), (1, 5, false)],
    )]);
    assert!(
        half_shortfalls(&p5, &w5, &h5).is_empty(),
        "THE CONTROL: the honest object carries all three"
    );
    assert_eq!(
        exact_bite(&p5, &w5, &b5, &mirror(&h5)),
        ExactBite::Moved { short: 1 },
        "and the mirrored object is short the करण bucket ALONE — one cancelled \
         bucket does not make the module uninformative"
    );

    // ── SIX, REFUSED: A TWIN UNDER A DIFFERENT WORD. The match is word-for-word
    // everywhere in this file and `swap_the_half_sides` exchanges each word's
    // operands in place, never moving a site between words, so `क समम् ५` and
    // `५ असमम् क` do NOT cancel — and the flip bites both.
    let six = three.replace("यदि ५ समम् क आदि", "यदि ५ असमम् क आदि");
    let (p6, w6) = load(&six);
    let b6 = half_exact_buckets_of(&p6, &w6);
    assert_eq!(
        (b6.sites, b6.buckets, b6.cancelled),
        (2, 2, 0),
        "one bucket under each of two words cancels nothing: {b6:?}"
    );
    let h6 = obj(vec![
        ("समलङ्घनम्", "मकार्यम्", vec![(1, 5, true)]),
        ("विषमलङ्घनम्", "मकार्यम्", vec![(1, 5, false)]),
    ]);
    assert!(
        half_shortfalls(&p6, &w6, &h6).is_empty(),
        "THE CONTROL: the honest object carries both"
    );
    assert_eq!(
        exact_bite(&p6, &w6, &b6, &mirror(&h6)),
        ExactBite::Moved { short: 2 },
        "and BOTH go short under the flip: the same bucket on the other side of \
         a different WORD is no twin"
    );

    // ── SEVEN, REFUSED: A TWIN IN A DIFFERENT ROUTINE. Halves are read per
    // routine label and no exchange carries a site across a frame.
    let seven = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
सार्वजनिक वृत्तिः कार्यान्तरम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    यदि ५ समम् क आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let (p7, w7) = load(seven);
    let b7 = half_exact_buckets_of(&p7, &w7);
    assert_eq!(
        (b7.sites, b7.buckets, b7.cancelled),
        (2, 2, 0),
        "the same bucket on the other side of a DIFFERENT frame cancels \
         nothing: {b7:?}"
    );
    let h7 = obj(vec![
        ("समलङ्घनम्", "मकार्यम्", vec![(1, 5, true)]),
        ("समलङ्घनम्", "मकार्यान्तरम्", vec![(1, 5, false)]),
    ]);
    assert!(
        half_shortfalls(&p7, &w7, &h7).is_empty(),
        "THE CONTROL: each routine's object carries its own side"
    );
    assert_eq!(
        exact_bite(&p7, &w7, &b7, &mirror(&h7)),
        ExactBite::Moved { short: 2 },
        "and the flip bites both routines: {b7:?}"
    );

    // ── AND THE CURRENCY, WHICH IS WHERE THE EXACT BUCKETS PART FROM THE
    // RESIDUE'S. Ranks १ and ९ are EIGHT APART, so they are ONE residue bucket
    // and TWO sites — and they are never one EXACT bucket, because `slot_at`
    // sends distinct ranks to distinct slots under every base. So the bite
    // ceiling here is two where the residue's is one, and `half_exact_bit` can
    // never exceed `half_exact_buckets`.
    let apart = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    चरः ग ॱॱ न६४ भवति ० ।
    चरः घ ॱॱ न६४ भवति ० ।
    चरः ङ ॱॱ न६४ भवति ० ।
    चरः च ॱॱ न६४ भवति ० ।
    चरः छ ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    चरः झ ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि झ समम् ५ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let (p8, w8) = load(apart);
    let b8 = half_exact_buckets_of(&p8, &w8);
    assert_eq!(
        (b8.sites, b8.buckets, b8.shared, b8.cancelled),
        (2, 2, 0, 0),
        "TWO exact buckets where `half_residue_buckets_of` reads ONE with a \
         shared site — the whole reason this is not `ResidueBuckets` under \
         another name: {b8:?}"
    );
    assert_eq!(
        half_residue_buckets_of(&["मकार्यम्".to_string()].into_iter().collect(), &w8).buckets,
        1,
        "and the residue reader really does collapse them, measured side by \
         side rather than argued"
    );
    let h8 = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (9, 5, true)],
    )]);
    assert!(
        half_shortfalls(&p8, &w8, &h8).is_empty(),
        "THE CONTROL: the honest object carries both slots"
    );
    assert_eq!(
        exact_bite(&p8, &w8, &b8, &mirror(&h8)),
        ExactBite::Moved { short: 2 },
        "and the flip bites TWO buckets, which is the ceiling the sweep asserts \
         the bite against"
    );
}
/// **THE RECOVERED ROUTINES' EXACT-SLOT BLINDNESS CONTROL WAS THREE BOOLEANS,
/// AND THE THIRD WAS AN ESCAPE RATHER THAN A GUARD.**
///
/// The sweep asked `file_recovered_sites > 0 && file_recovered_bit.is_empty() &&
/// file_recovered.cancelled_named.is_empty()`. The first two are the same two
/// states [`ExactBite`] was extracted out of one cycle earlier; the THIRD is not
/// a guard at all. Cancellation is a property of the SOURCE — a want whose twin
/// on the other side is written at least as often under the SAME word is
/// rewritten into that twin by the exchange — so a module needs exactly ONE
/// cancelled bucket to silence the control over every OTHER bucket it carries,
/// however many of those the exchange reads green. One cancelled bucket among
/// fifty informative ones and the module was reported as nothing at all.
///
/// [`recovered_bite`] answers [`ExactBite`]'s four arms instead, with `Moved`
/// decided BEFORE `Uninformative` so that only a module with NO informative
/// bucket left is excused. All four fire here, on wants read by this test's own
/// loader out of its own source, and the object handed to the decision is always
/// the honest one put through the SAME side flip [`swap_the_half_sides`] applies.
///
/// **THE CORPUS CANNOT REACH THE DIFFERENCE, which is why this is a built
/// object.** Its 22 recovered routines cancel NOTHING, so the conjunction and the
/// decision agree on every module in the tree today and the defect is invisible
/// to the sweep — the same reason the `pairable` control's three other arms had
/// to be built.
///
/// **THE CASES THAT MUST STILL BE REFUSED.** A module with a bucket the swap DOES
/// bite stays out of both quiet arms, however many of its others are cancelled; a
/// half-site in a routine the exhaustion did not recover is not this claim at
/// all; and the honest object is asserted green at every arm, or every red below
/// is a reader that reads nothing.
#[test]
fn a_cancelled_bucket_does_not_excuse_a_recovered_routine_whose_side_is_unread() {
    let wants = |src: &str| extra_pairs("म", &source_routines(src)).half_want;
    let obj = |rows: Vec<(&'static str, &str, Vec<Half>)>| -> BTreeMap<&'static str, HalfPairs> {
        let mut out: BTreeMap<&'static str, HalfPairs> = BTreeMap::new();
        for (w, r, v) in rows {
            out.entry(w).or_default().insert(r.to_string(), v);
        }
        out
    };
    // THE MUTATION, read off the object rather than written out:
    // `swap_the_half_sides` exchanges the operands of every one of the three
    // words in place, so what `emitted_halves_of` reads off the bitten object is
    // the honest halves with the SIDE bit flipped and nothing else moved.
    let mirror = |o: &BTreeMap<&'static str, HalfPairs>| -> BTreeMap<&'static str, HalfPairs> {
        o.iter()
            .map(|(w, p)| {
                (
                    *w,
                    p.iter()
                        .map(|(r, v)| {
                            (r.clone(), v.iter().map(|(s, c, k)| (*s, *c, !*k)).collect())
                        })
                        .collect(),
                )
            })
            .collect()
    };
    // THE RECOVERED MAP, AND ITS BASE IS NOT ARBITRARY. `प` takes rank ० and the
    // two `चरः` locals ranks १ and २ (`ir.t1:1320`), and the routine declares
    // THREE names, so the exhaustion's candidates are bases ० to ३. Base ३ cuts
    // the pool AFTER every name, so `slot_at` is the identity on all three ranks
    // and `क`'s rank १ is slot १ — the same triple the `pairable` control's arms
    // read, so the two decisions are exercised on one object shape and not two.
    let recovered: BTreeMap<String, usize> = [("मकार्यम्".to_string(), 3usize)].into_iter().collect();
    assert_eq!(
        slot_at(1, 3),
        1,
        "or the triples below are about a slot the source never named"
    );

    // ── ONE: THE BITE, which is the only arm the corpus has ever answered.
    let one = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let w1 = wants(one);
    assert_eq!(
        w1.get("मकार्यम्").and_then(|w| w.get("समलङ्घनम्")),
        Some(&vec![(1usize, 5i128, true)]),
        "the loader reads ONE exact triple, the name in the करण: {w1:?}"
    );
    let b1 = half_recovered_buckets_of(&recovered, &w1);
    assert_eq!(
        (b1.sites, b1.buckets, b1.shared, b1.cancelled),
        (1, 1, 0, 0),
        "one site, one bucket, nothing shared and nothing cancelled: {b1:?}"
    );
    let h1 = obj(vec![("समलङ्घनम्", "मकार्यम्", vec![(1, 5, true)])]);
    assert!(
        half_recovered_shortfalls(&recovered, &w1, &h1).is_empty(),
        "THE CONTROL: the honest object is green, or every red below is a reader \
         that reads nothing"
    );
    let v1 = recovered_bite(&recovered, &w1, &b1, &mirror(&h1));
    assert_eq!(
        v1,
        ExactBite::Moved { short: 1 },
        "the side flip takes the one bucket short: {v1:?}"
    );
    assert!(
        v1.complaint(RECOVERED_CLAIM).is_none() && v1.short() == 1,
        "and a bite is not a defect: it reports no complaint and one bite"
    );

    // ── TWO: NO SITES, WHICH IS NOTHING TO ASK AND NOT A GREEN. Both shapes that
    // reach it: a recovered routine that writes no name-against-constant head at
    // all, and a head in a routine the exhaustion did NOT recover — the latter
    // keeps its residue claim and its candidate SET, and reading it blind would
    // red the compiler for a routine this claim is not made of.
    let none: BTreeMap<String, ByWordHalf> = BTreeMap::new();
    assert_eq!(
        recovered_bite(
            &recovered,
            &none,
            &half_recovered_buckets_of(&recovered, &none),
            &h1
        ),
        ExactBite::NoSites,
        "a recovered routine with no half-site is NoSites"
    );
    let unrecovered: BTreeMap<String, usize> = BTreeMap::new();
    assert_eq!(
        recovered_bite(
            &unrecovered,
            &w1,
            &half_recovered_buckets_of(&unrecovered, &w1),
            &obj(vec![])
        ),
        ExactBite::NoSites,
        "and so is a half-site whose routine the exhaustion left AMBIGUOUS, \
         however empty the object"
    );

    // ── THREE: EVERY BUCKET UNINFORMATIVE. One slot against one constant from
    // BOTH sides under ONE word: the exchange rewrites each want into the
    // other's, the multiset does not move, and no mutation in this file can bite
    // the module. Reporting that as blindness would red the compiler for a
    // property of the SOURCE.
    let three = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ५ समम् क आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let w3 = wants(three);
    let b3 = half_recovered_buckets_of(&recovered, &w3);
    assert_eq!(
        (b3.sites, b3.buckets, b3.shared, b3.cancelled),
        (2, 2, 0, 2),
        "two buckets, BOTH cancelled — each is the other's twin, wanted as \
         often: {b3:?}"
    );
    let h3 = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (1, 5, false)],
    )]);
    assert!(
        half_recovered_shortfalls(&recovered, &w3, &h3).is_empty()
            && half_recovered_shortfalls(&recovered, &w3, &mirror(&h3)).is_empty(),
        "the honest object is green AND so is the mirrored one — the detector and \
         the mutation must name the same sites or one of them is wrong"
    );
    let v3 = recovered_bite(&recovered, &w3, &b3, &mirror(&h3));
    assert_eq!(
        v3,
        ExactBite::Uninformative { cancelled: 2 },
        "so the module is UNINFORMATIVE and its two sites come out of the \
         covered figure, not out of the claim: {v3:?}"
    );
    assert!(
        v3.complaint(RECOVERED_CLAIM).is_none() && v3.short() == 0,
        "and it is not a defect: {v3:?}"
    );

    // ── FOUR: BLIND, THE ONE DEFECT OF THE FOUR. An INFORMATIVE bucket — one
    // side written, the twin not — and an object that carries BOTH sides, so the
    // flip leaves it covered. Nothing in this file can tell that object from an
    // honest one, and that is exactly what must be reported.
    let both_sides = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (1, 5, false)],
    )]);
    let v4 = recovered_bite(&recovered, &w1, &b1, &mirror(&both_sides));
    assert_eq!(
        v4,
        ExactBite::Blind {
            informative: 1,
            cancelled: 0,
            routines: 1
        },
        "one informative site in one recovered routine, read green under the \
         flip: {v4:?}"
    );
    let said = v4
        .complaint(RECOVERED_CLAIM)
        .expect("the one arm that complains");
    assert!(
        said.contains("1 INFORMATIVE")
            && said.contains("RECOVERED")
            && said.contains("समलङ्घनम्")
            && said.contains("the side is not being read"),
        "and the sentence files THIS defect in ITS OWN words — the recovered \
         routines, and the words the mutation rewrites IN THE OBJECT: {said}"
    );

    // ── FIVE: THE DEFECT THE OLD CONJUNCTION COULD NOT SEE, AND IT IS THE WHOLE
    // REASON FOR THIS CYCLE. Two करण wants against one अपादान want: the अपादान
    // bucket is cancelled by a twin written twice, the करण bucket is NOT. Hand it
    // an object carrying TWO of each and the flip reads it green — so the module
    // has an informative bucket whose side is unread AND a cancelled bucket, and
    // the old third conjunct excused the first for the existence of the second.
    let five = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि क समम् ५ आदि
        प्रत्यागमनम् ३ ।
    इति
    यदि ५ समम् क आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let w5 = wants(five);
    let b5 = half_recovered_buckets_of(&recovered, &w5);
    assert_eq!(
        (b5.sites, b5.buckets, b5.shared, b5.cancelled),
        (3, 2, 1, 1),
        "three sites in two buckets — the करण bucket holds two of them, a FLOOR \
         of two counted ONCE — and only the अपादान site, whose twin outnumbers \
         it, is cancelled: {b5:?}"
    );
    let blind5 = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (1, 5, true), (1, 5, false), (1, 5, false)],
    )]);
    let bit5 = half_recovered_shortfalls(&recovered, &w5, &mirror(&blind5));
    assert!(
        half_recovered_shortfalls(&recovered, &w5, &blind5).is_empty() && bit5.is_empty(),
        "THE CONTROL AND THE BLINDNESS AT ONCE: the honest object carries all \
         three wants, and so does the mirrored one — an emitter that writes BOTH \
         sides is what no reading of the slot or the constant can refuse: {bit5:?}"
    );
    assert!(
        !b5.cancelled_named.is_empty(),
        "THE OLD ESCAPE, ASSERTED RATHER THAN DESCRIBED: `file_recovered_sites > \
         0 && file_recovered_bit.is_empty()` both hold above, so the ONLY thing \
         that kept the old conjunction silent on this module is its third \
         conjunct — and here is the cancelled bucket that closed it: {:?}",
        b5.cancelled_named
    );
    let v5 = recovered_bite(&recovered, &w5, &b5, &mirror(&blind5));
    assert_eq!(
        v5,
        ExactBite::Blind {
            informative: 2,
            cancelled: 1,
            routines: 1
        },
        "and the decision reports it: TWO informative sites blind, the cancelled \
         one counted out beside them rather than excusing them: {v5:?}"
    );

    // ── SIX, REFUSED: THE SAME MODULE WITH A BUCKET THE SWAP DOES BITE STAYS OUT
    // OF BOTH QUIET ARMS, which is what `Moved` before `Uninformative` buys. The
    // object carries exactly what the source wants, so the flip is short the करण
    // bucket ALONE — one cancelled bucket excuses nothing on its own.
    let h5 = obj(vec![(
        "समलङ्घनम्",
        "मकार्यम्",
        vec![(1, 5, true), (1, 5, true), (1, 5, false)],
    )]);
    assert!(
        half_recovered_shortfalls(&recovered, &w5, &h5).is_empty(),
        "THE CONTROL: the honest object carries all three"
    );
    assert_eq!(
        recovered_bite(&recovered, &w5, &b5, &mirror(&h5)),
        ExactBite::Moved { short: 1 },
        "the cancelled bucket is still there and the module is SENSITIVE"
    );
}

/// **THE SHAPE OF WHAT THE RECOVERY STOPPED AT, WHICH NOBODY HAD READ.** The
/// third widening reports a half-site-only routine RECOVERED, AMBIGUOUS or
/// UNEXPLAINED, and until this cycle the middle bucket — 74 of 96 routines —
/// carried no claim about the candidate set itself beyond its size. It has
/// one: the set is a contiguous run of bases, because a half-site's constraint
/// decomposes per rank and each rank answers an interval ([`base_run_gaps`]).
///
/// **AND THE CLAIM IS NOT A TAUTOLOGY, because the SAME exhaustion breaks it
/// the moment a PAIR is folded in.** Both halves of that are built here out of
/// [`consistent_bases_of`] itself and not asserted about in the abstract: one
/// object, read as two half-sites, answers one run; read as one pair, answers
/// a set with a hole in it. That is why the pair-folded routines are exempt,
/// and it is the instrument fault the corpus assertion exists to catch — a
/// pair group reaching the half-only exhaustion would make the gap appear
/// there too, with every recovered and ambiguous figure still looking right.
#[test]
fn a_half_routines_candidate_cut_set_is_one_run_and_a_pair_fold_is_exempt() {
    // THIS TEST'S OWN SOURCE AND ITS OWN LOADER. `प` takes rank ० and the three
    // `चरः` locals ranks १ to ३ in declaration order (`ir.t1:1320`), so `क` is
    // rank १ and `ख` is rank २ — ADJACENT, which is the only way the mixed
    // configuration a pair can refuse sits strictly between two it accepts.
    // The two constants DIFFER so the two half-sites are two distinct triples
    // and the decomposition is being exercised rather than a collision.
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    चरः ग ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ख समम् ७ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1, "{routines:?}");
    assert_eq!(
        routines[0].slots.len(),
        4,
        "the parameter and three locals: {:?}",
        routines[0].slots
    );
    let extra = extra_pairs("म", &routines);
    let want = extra
        .half_want
        .get("मकार्यम्")
        .and_then(|w| w.get("समलङ्घनम्"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        want,
        vec![(1usize, 5i128, true), (2usize, 7i128, true)],
        "rank १ against ५ and rank २ against ७, the NAME in the करण \
         both times: {:?}",
        extra.half_want
    );
    const NO_PAIRS: &[Group<'static>] = &[];

    // ── ONE: THE OBJECT CARRIES EACH RANK ON BOTH SIDES OF THE CUT, AND EVERY
    // BASE SURVIVES. `slot_at(१, b)` is १ or ९ and `slot_at(२, b)` is २ or १०,
    // and the object holds all four triples — so the MIXED configuration
    // (`b = २`, rank १ below the cut and rank २ above it) is satisfied by the
    // same four with no fifth triple needed. That is the decomposition, and it
    // is what a pair cannot do.
    let both: Vec<Half> = vec![(9, 5, true), (1, 5, true), (10, 7, true), (2, 7, true)];
    let run = consistent_bases_of(NO_PAIRS, &[(&want, &both)], 4);
    assert_eq!(
        run,
        vec![0, 1, 2, 3, 4],
        "all five candidate cut positions fit, which is the widest \
         AMBIGUOUS set there is"
    );
    assert!(base_run_gaps(&run).is_empty(), "and it is one run: {run:?}");

    // ── TWO: AND A NARROWER SET IS STILL ONE RUN. Drop rank १'s LOW triple and
    // every base above १ dies at once — `{b : b ≤ १}`, an interval, and the
    // whole point is that it is an interval whatever else the object carries.
    let high: Vec<Half> = vec![(9, 5, true), (10, 7, true), (2, 7, true)];
    let narrow = consistent_bases_of(NO_PAIRS, &[(&want, &high)], 4);
    assert_eq!(
        narrow,
        vec![0, 1],
        "rank १ is carried only at slot ९, so only a cut at or below १ fits it"
    );
    assert!(base_run_gaps(&narrow).is_empty(), "{narrow:?}");

    // ── THREE, AND THIS IS THE CASE THAT MUST STILL BE REFUSED: THE SAME
    // OBJECT READ AS ONE PAIR HAS A HOLE IN IT. `(१, २)` is ONE constraint over
    // two ranks, so the object must carry the pair `(slot_at(१,b), slot_at(२,b))`
    // as a UNIT: `(९, १०)` below `b = २`, `(१, १०)` at it, `(१, २)` above. An
    // object holding the first and the third and not the middle one refuses
    // exactly `b = २` — and it is RIGHT to. Demanding one run of a pair-folded
    // routine would red the compiler for this; `pair_run_exempt` counts them
    // out instead.
    let gapped = consistent_bases(&[(1, 2)], &[(9, 10), (1, 2)], 4);
    assert_eq!(
        gapped,
        vec![0, 1, 3, 4],
        "both ranks above the cut and both below fit; the mixed configuration          does not"
    );
    assert_eq!(
        base_run_gaps(&gapped),
        vec![2],
        "and the gap is NAMED, not merely counted — a detector that has \
         never answered a non-empty list is a reader that reads nothing"
    );

    // ── FOUR: THE INSTRUMENT FAULT THE CORPUS ASSERTION EXISTS FOR, BUILT. The
    // half-only sweep passes `NO_LT_SITES` to [`cut_groups`]; wire a pair group
    // in and the SAME half-sites that answered one run in ONE now answer a set
    // with a hole — while `half_wide_recovered`, `half_wide_ambiguous` and
    // `half_wide_none` all still report plausible numbers. This is why a gap is
    // read as a leaked constraint and never as a second cut.
    let leaked = consistent_bases_of(
        &[(&[(1usize, 2usize)][..], &[(9, 10), (1, 2)][..])],
        &[(&want, &both)],
        4,
    );
    assert_eq!(
        leaked,
        vec![0, 1, 3, 4],
        "the pair narrows it and breaks it"
    );
    assert_eq!(
        base_run_gaps(&leaked),
        vec![2],
        "and THIS is what `half_run_broken` would report"
    );

    // ── FIVE, REFUSED: A SINGLETON IS TRIVIALLY ONE RUN, so counting it would
    // make the corpus figure green on nothing. The claim is asked of the `k ≥ २`
    // arm alone, and the helper is silent here whatever the arm does.
    assert!(
        base_run_gaps(&[3]).is_empty(),
        "a routine whose cut IS recovered has one candidate and no shape to ask          about"
    );

    // ── SIX, REFUSED: AN EMPTY SET IS `half_wide_none`'S, NOT THIS CLAIM'S. A
    // routine no base explains is a second cut, a misranked declaration or a
    // side read backwards, and it is already RED there; reading it red twice
    // would double-count one defect and make the contiguity figure look
    // sensitive to something it cannot see.
    assert!(
        base_run_gaps(&[]).is_empty(),
        "no candidate at all names no gap"
    );

    // ── SEVEN: AND THE DETECTOR SEES A WIDE HOLE, not only a one-base one — a
    // reader that returned `min` and `max` and stopped would pass FOUR above.
    assert_eq!(
        base_run_gaps(&[0, 4]),
        vec![1, 2, 3],
        "every missing base inside the range is named"
    );
}

/// **THE 22 ROUTINES THE THIRD WIDENING RECOVERED, AND WHAT NOBODY HAD ASKED
/// THEM FOR.** `half_bases` splits its 96 half-site-only routines into 74
/// AMBIGUOUS and 22 RECOVERED. The 74 carry the residue claim; the 22 carried
/// NOTHING — [`half_residue_shortfalls`] excludes them by design, as circular,
/// and it is right to. But a recovered routine knows where its pool was cut, so
/// its half-sites predict an EXACT `(slot, constant, side)`, and that claim is
/// [`half_recovered_shortfalls`].
///
/// **AND THE CIRCULARITY OBJECTION IS MET HERE RATHER THAN ARGUED AWAY.** The
/// base was chosen to satisfy these wants, so on the honest object the reader
/// IS green by construction — the same standing as [`base_run_gaps`]. What it
/// catches is the instrument fault: a base from a recovery that stopped reading
/// its half-groups. Built in FOUR below. What falsifies it is
/// [`swap_the_half_sides`], which moves the OBJECT while the base stays the
/// honest one. Built in THREE.
#[test]
fn a_recovered_half_routine_predicts_an_exact_slot_and_an_ambiguous_one_predicts_none() {
    // THIS TEST'S OWN SOURCE AND ITS OWN LOADER. `प` takes rank ० and the three
    // `चरः` locals ranks १ to ३ in declaration order (`ir.t1:1320`), so `क` is
    // rank १ and `ख` is rank २. The two constants DIFFER, so the two half-sites
    // are two distinct triples and each pins its own side of the cut — which is
    // what makes a SINGLETON candidate set reachable from half-sites alone.
    let src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    चरः ग ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ख समम् ७ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1, "{routines:?}");
    let extra = extra_pairs("म", &routines);
    let want = extra
        .half_want
        .get("मकार्यम्")
        .and_then(|w| w.get("समलङ्घनम्"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        want,
        vec![(1usize, 5i128, true), (2usize, 7i128, true)],
        "rank १ against ५ and rank २ against ७, the NAME in the करण both \
         times: {:?}",
        extra.half_want
    );
    assert!(
        extra.want.get("मकार्यम्").is_none_or(BTreeMap::is_empty),
        "and this routine writes NO two-name head, so no pair recovery claims \
         it and `half_bases` is the only sweep that visits it: {:?}",
        extra.want
    );
    const NO_PAIRS: &[Group<'static>] = &[];
    let have = |w: &'static str, v: Vec<Half>| -> BTreeMap<&'static str, HalfPairs> {
        [(w, [("मकार्यम्".to_string(), v)].into_iter().collect())]
            .into_iter()
            .collect()
    };
    let at = |b: usize| -> BTreeMap<String, usize> {
        [("मकार्यम्".to_string(), b)].into_iter().collect()
    };

    // ── ONE: THE CUT IS RECOVERED, AND IT IS THE SINGLETON THAT LICENSES
    // EVERYTHING BELOW. The object carries rank १ LOW (slot १) and rank २ HIGH
    // (slot १०), so `{b : b > १}` for the first meets `{b : b ≤ २}` for the
    // second and exactly ONE base survives. Each want alone answers an interval
    // of three; together they answer one.
    let object: Vec<Half> = vec![(1, 5, true), (10, 7, true)];
    let bases = consistent_bases_of(NO_PAIRS, &[(&want, &object)], 3);
    assert_eq!(
        bases,
        vec![2],
        "rank १ is carried only LOW and rank २ only HIGH, so the cut fell \
         between them and nowhere else"
    );
    assert_eq!(
        (slot_at(1, 2), slot_at(2, 2)),
        (1, 10),
        "and THAT base is what turns the two ranks into two exact slots"
    );

    // ── TWO: SO THE EXACT CLAIM IS MADE, AND IT IS GREEN — as it must be, since
    // the base was chosen to satisfy these very wants. A red here would mean the
    // reader and the exhaustion disagree about `slot_at`.
    assert!(
        half_recovered_shortfalls(&at(2), &extra.half_want, &have("समलङ्घनम्", object.clone()))
            .is_empty(),
        "the recovered base explains both sites exactly, or every red below is \
         a reader that reads nothing"
    );
    let c = half_recovered_buckets_of(&at(2), &extra.half_want);
    assert_eq!(
        (c.buckets, c.shared, c.cancelled),
        (2, 0, 0),
        "two sites, two DISTINCT exact buckets — `slot_at` sends distinct ranks \
         to distinct slots, so nothing shares here and nothing cancels: {c:?}"
    );

    // ── THREE: AND THE FALSIFIER, WHICH IS NOT CIRCULAR. The base stays the
    // HONEST object's `२`; the object asked is the swapped one, where `क समम् ५`
    // has become `५ समम् क` and the name sits in the अपादान. The slot and the
    // constant are untouched, so nothing but the side bit can fail — and both
    // buckets do.
    let swapped: Vec<Half> = object.iter().map(|(s, v, k)| (*s, *v, !*k)).collect();
    let bit = half_recovered_shortfalls(&at(2), &extra.half_want, &have("समलङ्घनम्", swapped));
    assert_eq!(
        bit.len(),
        2,
        "exchanging the operands of every `समम्` branch bites BOTH exact \
         buckets, which is what `half_recovered_bit` counts: {bit:?}"
    );
    assert!(
        bit[0].contains("recovered at base 2"),
        "and the line names the base the claim rests on, since that is the \
         whole of what distinguishes it from the residue: {}",
        bit[0]
    );

    // ── FOUR: THE INSTRUMENT FAULT THE CORPUS ASSERTION EXISTS FOR, BUILT. A
    // recovery that stopped reading its half-groups would hand back a base
    // chosen by something else — base ३, say, which `consistent_bases_of`
    // refuses above. The exact claim reds on it, and it is the ONLY reader in
    // this file that would: `half_wide_recovered` would still count one
    // recovered routine, `half_run_broken` would still see a singleton run, and
    // `half_residue_shortfalls` would never be asked.
    let fault =
        half_recovered_shortfalls(&at(3), &extra.half_want, &have("समलङ्घनम्", object.clone()));
    assert_eq!(
        fault.len(),
        1,
        "base ३ puts rank २ at slot २, which the object does not carry: {fault:?}"
    );
    assert!(
        !bases.contains(&3),
        "and the exhaustion refuses base ३, so the two instruments agree and \
         the red can only come from a base the exhaustion never produced"
    );

    // ── FIVE, REFUSED: AN AMBIGUOUS ROUTINE PREDICTS NO EXACT SLOT, AND THE
    // UNION-OVER-CANDIDATES READING CARRIES NO CLAIM AT ALL. An object holding
    // each rank on BOTH sides of the cut accepts every base — and then EVERY
    // candidate reads green, so "some candidate explains the wants" is the
    // existential `consistent_bases_of` already answered and is weaker than the
    // residue. The map holds one base per routine and this routine has no entry.
    let both: Vec<Half> = vec![(1, 5, true), (9, 5, true), (2, 7, true), (10, 7, true)];
    let wide = consistent_bases_of(NO_PAIRS, &[(&want, &both)], 3);
    assert_eq!(wide, vec![0, 1, 2, 3], "every candidate cut fits: {wide:?}");
    for b in &wide {
        assert!(
            half_recovered_shortfalls(&at(*b), &extra.half_want, &have("समलङ्घनम्", both.clone()))
                .is_empty(),
            "candidate {b} reads green, so a reader that accepted ANY candidate \
             could never red on this routine"
        );
    }
    assert!(
        half_recovered_shortfalls(
            &BTreeMap::new(),
            &extra.half_want,
            &have("समलङ्घनम्", Vec::new())
        )
        .is_empty(),
        "and the reader claims NOTHING it is not handed — an empty map over an \
         object carrying no half-site at all is silent, which is how the \
         ambiguous routines and the pair-narrowed ones stay out"
    );

    // ── SIX, REFUSED: A ROUTINE WHOSE BASE WAS NARROWED BY A PAIR GROUP IS NOT
    // IN `half_bases` AT ALL. Add ONE two-name head to the same source and
    // `extra_pairs` harvests a pair want for the routine — which is what puts it
    // in `wide_bases`, and `half_bases` skips every routine `wide_bases`,
    // `want_paired` or `want_residue` claims. Its cut rests on two-name
    // evidence, so the exact claim over it would restate the pair recovery
    // rather than be something the half-sites carry on their own.
    let paired_src = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    चरः ख ॱॱ न६४ भवति ० ।
    चरः ग ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ख समम् ग आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let paired = extra_pairs("म", &source_routines(paired_src));
    assert_eq!(
        paired
            .half_want
            .get("मकार्यम्")
            .and_then(|w| w.get("समलङ्घनम्"))
            .cloned()
            .unwrap_or_default(),
        vec![(1usize, 5i128, true)],
        "the half-site survives the addition: {:?}",
        paired.half_want
    );
    assert_eq!(
        paired
            .want
            .get("मकार्यम्")
            .and_then(|w| w.get("समलङ्घनम्"))
            .cloned()
            .unwrap_or_default(),
        vec![(2usize, 3usize)],
        "and the two-name head is harvested as a PAIR, which is exactly the \
         entry that sends this routine to `wide_bases` and out of \
         `half_bases`: {:?}",
        paired.want
    );

    // ── SEVEN: AND THE SHARED-BUCKET FLOOR, because the site count and the
    // bucket count are DIFFERENT NUMBERS and the corpus asserts an identity
    // between them. Two ranks never share an exact slot, so the only way to
    // share a bucket here is ONE source comparison written twice — and the
    // claim is then a FLOOR of two reported ONCE, never two pins.
    let twice = "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः कार्यम् आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    यदि क समम् ५ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि क समम् ५ आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let two = extra_pairs("म", &source_routines(twice));
    assert_eq!(
        two.half_want
            .get("मकार्यम्")
            .and_then(|w| w.get("समलङ्घनम्"))
            .cloned()
            .unwrap_or_default(),
        vec![(1usize, 5i128, true), (1usize, 5i128, true)],
        "the same rank against the same constant on the same side, twice: {:?}",
        two.half_want
    );
    let shared = half_recovered_buckets_of(&at(0), &two.half_want);
    assert_eq!(
        (shared.buckets, shared.shared, shared.cancelled),
        (1, 1, 0),
        "two sites, ONE bucket, ONE of them sharing: {shared:?}"
    );
    assert!(
        shared.shared_named[0].contains("FLOOR of 2"),
        "and the line says the floor the second site makes, since that is the \
         whole of what it adds: {}",
        shared.shared_named[0]
    );
    let short =
        half_recovered_shortfalls(&at(0), &two.half_want, &have("समलङ्घनम्", vec![(9, 5, true)]));
    assert_eq!(
        short.len(),
        1,
        "and an object carrying the bucket ONCE where the source wants it twice \
         goes short on ONE line, not two — the floor is reported once: {short:?}"
    );
    assert!(
        short[0].contains("wants 2 `समलङ्घनम्`") && short[0].contains("carries 1"),
        "with both numbers on it: {}",
        short[0]
    );

    // ── EIGHT, REFUSED: A BUCKET WITH A SAME-SLOT TWIN ON THE OTHER SIDE UNDER
    // ONE WORD IS UNINFORMATIVE AND COMES OUT OF THE COVERED FIGURE. Exchanging
    // that word's operands rewrites each want into the other's and the multiset
    // does not move, so no mutation in this file can bite it — and counting it
    // covered would make the bite arithmetic short by one with nothing to say
    // why. The corpus has NONE today, which is why it is built here.
    let mirrored: BTreeMap<String, ByWordHalf> = [(
        "मकार्यम्".to_string(),
        [(
            "समलङ्घनम्",
            vec![(1usize, 5i128, true), (1usize, 5i128, false)],
        )]
        .into_iter()
        .collect(),
    )]
    .into_iter()
    .collect();
    let mirror = half_recovered_buckets_of(&at(0), &mirrored);
    assert_eq!(
        (mirror.buckets, mirror.cancelled),
        (2, 2),
        "two buckets, and BOTH cancel each other: {mirror:?}"
    );
    let unmoved: Vec<Half> = vec![(9, 5, true), (9, 5, false)];
    assert!(
        half_recovered_shortfalls(&at(0), &mirrored, &have("समलङ्घनम्", unmoved.clone())).is_empty()
            && half_recovered_shortfalls(
                &at(0),
                &mirrored,
                &have(
                    "समलङ्घनम्",
                    unmoved.iter().map(|(s, v, k)| (*s, *v, !*k)).collect()
                )
            )
            .is_empty(),
        "and the swap really cannot move it — green both ways round, which is \
         why these sites are named UNINFORMATIVE rather than covered"
    );
}

/// **THE SUBSET LAW THE FOLD MUST OBEY, MADE TO FIRE — SO ITS CORPUS ZERO IS A
/// MEASUREMENT AND NOT A CONSTRUCTION.**
///
/// `not_monotone` carries the heaviest claim in this file's recovery chain: that
/// folding `समम्`/`असमम्`/`बृहत्समम्` into the `न्यूनलङ्घनम्` exhaustion can only
/// SHRINK the candidate set, so a cut it recovers is WITNESSED rather than
/// fitted. Nothing had ever put a routine in the bucket. The sibling
/// [`the_other_comparisons_narrow_the_cut_and_a_run_typed_one_is_refused_not_predicted`]
/// asserts the law over two objects, but both readings are wired CORRECTLY, so
/// it proves the law holds where it cannot be broken — green by construction.
///
/// The break is in the WIRING, not in the object: [`cut_groups`] pairs each
/// want with the pairs the object carries under the SAME mnemonic, and a
/// grouping that CROSSES them — one word's predicted ranks against another
/// word's carried pairs — is the one mistake the bucket exists to catch. Here
/// the harvested wants are handed a swapped object and bases the honest reading
/// REFUSED come back alive.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
///
/// An honest fold that merely NARROWS must be accepted (part TWO, and the same
/// object read the right way round in parts FOUR and FIVE), and EQUALITY — the
/// fold consistent with exactly the bases `न्यूनलङ्घनम्` alone left, adding no
/// evidence — must be reported neither as a miss NOR as a narrowing (part
/// THREE), or `cut_narrowed` degrades to "the fold did something".
///
/// **AND THE LAW IS CONTAINMENT, NOT CARDINALITY.** Part FOUR's crossing leaves
/// TWO candidates where the honest reading left NINE: a size check reads it
/// green while every base it kept is one the honest reading refused. Part FIVE
/// is the crossing that also GROWS the list, 10 to 11. One bucket, two shapes,
/// and only the subset test sees both.
///
/// **WHAT THIS BUCKET DOES NOT CATCH, named so the zero is not read wider than
/// it is.** A crossed grouping may also EMPTY the set — part TWO's object read
/// the wrong way round leaves NO candidate — and an empty set is a subset, so
/// `not_monotone` is quiet. That direction is `cut_none`'s, and it is the RED
/// direction: this bucket watches only the permissive one.
///
/// Its own loader, and its own fixture.
#[test]
fn a_fold_that_crosses_mnemonics_adds_a_candidate_and_is_reported_not_monotone() {
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
    यदि प न्यूनम् अ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ॠ समम् ऌ आदि
        प्रत्यागमनम् २ ।
    इति
    यदि आ असमम् इ आदि
        प्रत्यागमनम् ३ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 10, "प, अ … ऌ — ten names, so eleven candidate cuts");

    // ── ONE: THE WANTS ARE HARVESTED, NOT INVENTED. Only the OBJECT below is
    // built; the three rank pairs the crossing is fed come off this source
    // through the same two harvesters the sweep uses.
    let extra = extra_pairs("म", &routines);
    assert!(
        extra.refused.is_empty() && extra.numeral == 0,
        "three scalar two-name heads, nothing named: {:?}",
        extra.refused
    );
    assert_eq!(
        extra.want["मक"]["समलङ्घनम्"],
        vec![(8usize, 9usize)],
        "ranks ॠ ८ and ऌ ९, in WRITTEN order"
    );
    assert_eq!(
        extra.want["मक"]["विषमलङ्घनम्"],
        vec![(2usize, 3usize)],
        "ranks आ २ and इ ३"
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

    // The object, one mnemonic at a time, and the harvested want restricted to
    // the one word each reading folds — so a reading's groups are exactly two
    // and the crossing is a SWAP of their HAVE sides and nothing else.
    let carried =
        |ps: Vec<(usize, usize)>| -> Pairs { [("मक".to_string(), ps)].into_iter().collect() };
    let under = |w: &'static str, ps: Vec<(usize, usize)>| -> BTreeMap<&'static str, Pairs> {
        [(w, carried(ps))].into_iter().collect()
    };
    let want_of = |w: &'static str| -> ByWord {
        [(w, extra.want["मक"][w].clone())].into_iter().collect()
    };
    let honest = |lt_have: &Pairs, other: &BTreeMap<&'static str, Pairs>, ew: &ByWord| {
        consistent_bases_of(&cut_groups("मक", &lt, lt_have, Some(ew), other), &[], names)
    };
    // **THE CROSSING.** `lt`'s ranks against the OTHER word's carried pairs,
    // and the other word's ranks against `न्यूनलङ्घनम्`'s — the one wiring
    // mistake `not_monotone` exists to catch, expressed through the sweep's own
    // builder by exchanging what it is handed.
    let crossed = |lt_have: &Pairs, other: &BTreeMap<&'static str, Pairs>, ew: &ByWord| {
        let (&w, carried_other) = other.iter().next().expect("one word");
        consistent_bases_of(
            &cut_groups(
                "मक",
                &lt,
                carried_other,
                Some(ew),
                &under(w, lt_have["मक"].clone()),
            ),
            &[],
            names,
        )
    };
    let eq = want_of("समलङ्घनम्");
    let ne = want_of("विषमलङ्घनम्");

    // ── TWO: THE HONEST NARROWING, ACCEPTED. `(०, १)` carried as written
    // leaves every cut at rank २ or later; `(८, १७)` puts the cut strictly
    // between ranks ८ and ९, so `b = ९` and nothing else survives the fold.
    let lt_have = carried(vec![(0, 1)]);
    let nine = under("समलङ्घनम्", vec![(8, 17)]);
    let alone = consistent_bases(&lt, &lt_have["मक"], names);
    assert_eq!(
        alone,
        (2..=10).collect::<Vec<usize>>(),
        "nine candidate cuts survive the one `न्यूनलङ्घनम्`"
    );
    assert_eq!(honest(&lt_have, &nine, &eq), vec![9usize]);
    assert_eq!(
        fold_verdict(&alone, &honest(&lt_have, &nine, &eq)),
        FoldVerdict::Narrowed,
        "nine candidates to one, all nine of them already there — EVIDENCE"
    );
    // AND THE DIRECTION THIS BUCKET DOES NOT WATCH: that same object crossed
    // leaves NO candidate, which is a subset, so the law is quiet and the RED
    // is `cut_none`'s to report.
    assert!(crossed(&lt_have, &nine, &eq).is_empty());
    assert_eq!(
        fold_verdict(&alone, &crossed(&lt_have, &nine, &eq)),
        FoldVerdict::Narrowed,
        "an emptied set is still a subset — named, not silently claimed"
    );

    // ── THREE: EQUALITY, AND IT IS NEITHER A MISS NOR A NARROWING. An object
    // carrying ALL THREE forms `(२, ३)` takes under the eleven cuts — `(१०, ११)`
    // for `b ≤ २`, `(२, ११)` for `b = ३`, `(२, ३)` for `b ≥ ४` — satisfies
    // `विषमलङ्घनम्` at every candidate, so the fold leaves exactly what
    // `न्यूनलङ्घनम्` alone left.
    let mute = under("विषमलङ्घनम्", vec![(10, 11), (2, 11), (2, 3)]);
    assert_eq!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT_SITES, &Pairs::new(), Some(&ne), &mute),
            &[],
            names
        ),
        (0..=10).collect::<Vec<usize>>(),
        "the `विषमलङ्घनम्` group alone refuses nothing"
    );
    assert_eq!(honest(&lt_have, &mute, &ne), alone);
    assert_eq!(
        fold_verdict(&alone, &honest(&lt_have, &mute, &ne)),
        FoldVerdict::Unchanged,
        "THE CASE THAT MUST NOT BE REPORTED, and must not be banked either: \
         the fold added no evidence, so it is not a narrowing"
    );

    // ── FOUR: THE CROSSING FIRES, AND THE SET IT LEAVES IS SHORTER THAN THE
    // HONEST ONE. The object carries `(०, १)` and `(१६, १७)` under
    // `न्यूनलङ्घनम्` and `(८, ९)`, `(०, ९)` under `समलङ्घनम्`. Read the right way
    // round it is an honest narrowing to `b = १०`. Read crossed, `lt`'s `(०, १)`
    // is matched against `समलङ्घनम्`'s pairs — which carry `(८, ९)` and `(०, ९)`,
    // exactly what cuts ० and १ predict — while `समलङ्घनम्`'s `(८, ९)` is matched
    // against `न्यूनलङ्घनम्`'s `(१६, १७)`, what every cut up to ८ predicts. So
    // `{०, १}` survives: TWO candidates against the honest reading's NINE, and
    // both of them bases the honest reading REFUSED.
    let lt_swap = carried(vec![(0, 1), (16, 17)]);
    let eq_swap = under("समलङ्घनम्", vec![(8, 9), (0, 9)]);
    assert_eq!(
        consistent_bases(&lt, &lt_swap["मक"], names),
        alone,
        "the extra pair changes nothing the `न्यूनलङ्घनम्` reading sees"
    );
    assert_eq!(honest(&lt_swap, &eq_swap, &eq), vec![10usize]);
    assert_eq!(
        fold_verdict(&alone, &honest(&lt_swap, &eq_swap, &eq)),
        FoldVerdict::Narrowed,
        "SAME OBJECT, right way round: quiet"
    );
    let two = crossed(&lt_swap, &eq_swap, &eq);
    assert_eq!(two, vec![0usize, 1usize]);
    assert_eq!(
        fold_verdict(&alone, &two),
        FoldVerdict::Added(vec![0, 1]),
        "THE BUCKET FIRES, and a cardinality check would have read it green: \
         two candidates is FEWER than nine"
    );
    assert!(
        two.len() < alone.len(),
        "which is exactly why the law is containment and not size"
    );

    // ── FIVE: AND THE CROSSING THAT GROWS THE LIST. The same swap over an
    // object permissive BOTH ways — `न्यूनलङ्घनम्` carrying every form
    // `समलङ्घनम्`'s `(८, ९)` takes, and `समलङ्घनम्` carrying every form `lt`'s
    // `(०, १)` takes — leaves ALL ELEVEN cuts. The honest reading of the same
    // object leaves `{१०}`, and `न्यूनलङ्घनम्` alone leaves TEN: cut ० is
    // consistent with it here, because the object carries `(८, ९)`, and cut १
    // is not, because it carries no `(०, ९)`. That missing १ is the base the
    // crossing invents.
    let lt_wide = carried(vec![(0, 1), (8, 9), (8, 17), (16, 17)]);
    let eq_wide = under("समलङ्घनम्", vec![(0, 1), (0, 9), (8, 9)]);
    let wide_alone = consistent_bases(&lt, &lt_wide["मक"], names);
    assert_eq!(
        wide_alone,
        vec![0usize, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        "ten of the eleven cuts, and १ is the one missing"
    );
    assert_eq!(honest(&lt_wide, &eq_wide, &eq), vec![10usize]);
    assert_eq!(
        fold_verdict(&wide_alone, &honest(&lt_wide, &eq_wide, &eq)),
        FoldVerdict::Narrowed,
        "SAME OBJECT, right way round: quiet"
    );
    let all = crossed(&lt_wide, &eq_wide, &eq);
    assert_eq!(all, (0..=10).collect::<Vec<usize>>());
    assert_eq!(
        fold_verdict(&wide_alone, &all),
        FoldVerdict::Added(vec![1]),
        "THE SET GROWS, ten to eleven, and the added base is named"
    );
    assert!(
        all.len() > wide_alone.len(),
        "more candidates after MORE constraints"
    );
}

/// **THE MUTATION'S OWN REACH, MADE TO FIRE ON COMPILED OBJECTS — SO ITS CORPUS
/// ZERO IS A MEASUREMENT AND NOT A CONSTRUCTION.**
///
/// [`drop_the_exchange`] exists to lower `a अधिकम् b` as `Lt(a, b)` and nothing
/// else, so the three other branch words must come out of the bitten object
/// IDENTICAL, and `reached_further`'s assert says that if they do not "the
/// sensitivity figures below measure a mutation nobody described". Every bite
/// the sweep reports rests on that line and nothing had ever put a routine in
/// it: the arm was a claim nobody had seen work.
///
/// The fixture is not a built map — it is FOUR COMPILES of one four-line
/// fixture, bitten at the SOURCE, so what [`reach_verdict`] is handed is an object
/// the product's own emitter wrote. One routine, four names (`प`, `अ`, `आ` and
/// the `न्यूनम्` head's), a two-name `न्यूनम्` head for the mutation to bite, a
/// two-name `समम्` head and a `समम्` against a constant.
///
/// **AND THE TWO PUSH SITES ARE TOLD APART, one object each.** Exchanging the
/// `समम्` head's two NAMES moves its two-slot pair and leaves the half-site
/// alone; moving the CONSTANT moves the half-site and leaves the pair alone.
/// [`emitted_pairs_of`] drops every site with a constant operand and
/// [`emitted_halves_of`] drops every two-slot site, so neither bite can be seen
/// by the other's reader — which is why a single bucket carrying both sentences
/// would let one zero stand for two.
///
/// **THE CASE THAT MUST STILL BE REFUSED.** A mutation that reaches EXACTLY as
/// far as it claims must stay QUIET, and `drop_the_exchange` is that mutation:
/// it rewrites this object's `न्यूनलङ्घनम्` pair — asserted LIVE here, or the
/// quiet would be the quiet of a no-op — and leaves [`ReachVerdict::Contained`].
///
/// Its own loader, and its own fixture.
#[test]
fn a_mutation_that_reaches_past_the_exchange_is_reported_and_the_two_sides_are_told_apart() {
    // THE FIXTURE, parameterised at the two heads the bites move and nowhere
    // else, so every object below differs from the honest one in ONE head.
    let src = |eq: &str, half: &str| {
        format!(
            "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
    यदि प न्यूनम् अ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि {eq} आदि
        प्रत्यागमनम् २ ।
    इति
    यदि {half} आदि
        प्रत्यागमनम् ३ ।
    इति
    प्रत्यागमनम् ० ।
इति
"
        )
    };
    // This test's own loader. A refusal is NAMED, never folded into a green:
    // a fixture that does not compile carries no object and would answer
    // `Contained` for every bite.
    let framed = |s: &str, what: &str| -> (String, Windows) {
        match compile_framed(s) {
            // `_frames` and not two fields: `Framed::Emitted` carries the whole
            // frame as its third field, which this test does not read — it works
            // off the local windows, as its margin below says. The two were
            // written on lineages that had not met, and the arity is the only
            // place they disagreed.
            Framed::Emitted(t, w, _frames) => (t, w),
            Framed::DeclaresNoModule => panic!("{what}: the fixture declares `मण्डलम् म`"),
            Framed::Refused(e) => panic!("{what}: the fixture compiles: {e}"),
        }
    };
    let (text, windows) = framed(&src("अ समम् आ", "आ समम् ५"), "the honest object");
    // The two readings, off the HONEST windows for every object — exactly as
    // the sweep does it, which is the point: the frames are the honest ones and
    // a bite that moved them would be a different mutation.
    let read = |t: &str| -> (
        BTreeMap<&'static str, Pairs>,
        BTreeMap<&'static str, HalfPairs>,
    ) {
        (
            EXTRA_WORDS
                .into_iter()
                .map(|m| (m, emitted_pairs_of(t, &windows, m, &mut Vec::new())))
                .collect(),
            EXTRA_WORDS
                .into_iter()
                .map(|m| (m, emitted_halves_of(t, &windows, m, &mut Vec::new())))
                .collect(),
        )
    };
    let (have_pairs, have_halves) = read(&text);

    // ── ONE: THE FIXTURE CARRIES BOTH SHAPES, and the readers really do see one
    // each. Without this the three bites below could all be quiet for the same
    // uninteresting reason.
    let eq_pairs = have_pairs["समलङ्घनम्"].clone();
    let eq_halves = have_halves["समलङ्घनम्"].clone();
    assert_eq!(
        eq_pairs.values().map(Vec::len).sum::<usize>(),
        1,
        "one two-slot `समलङ्घनम्` — `अ समम् आ`: {eq_pairs:?}"
    );
    assert_eq!(
        eq_halves
            .values()
            .flatten()
            .map(|(_, v, _)| *v)
            .collect::<Vec<i128>>(),
        vec![5i128],
        "and one half-site against the constant ५: {eq_halves:?}"
    );

    // ── TWO, REFUSED: THE MUTATION THAT REACHES EXACTLY AS FAR AS IT CLAIMS IS
    // QUIET — and it is LIVE on this object, which is the half of the claim a
    // no-op would satisfy for free.
    let exact = drop_the_exchange(&text);
    assert_ne!(
        emitted_pairs(&exact, &windows, &mut Vec::new()),
        emitted_pairs(&text, &windows, &mut Vec::new()),
        "`drop_the_exchange` bites this object's `न्यूनलङ्घनम्` pair, so the \
         quiet below is a reach and not an absence"
    );
    let (exact_pairs, exact_halves) = read(&exact);
    assert_eq!(
        reach_verdict(&have_pairs, &have_halves, &exact_pairs, &exact_halves),
        ReachVerdict::Contained,
        "THE CASE THAT MUST STILL BE REFUSED: the described mutation rewrites \
         `न्यूनलङ्घनम्` lines only, so the three other words come out identical \
         and the bucket must stay empty"
    );

    // ── THREE: THE PAIR SIDE FIRES, AND THE HALF SIDE DOES NOT SEE IT. The
    // `समम्` head's two names exchanged: same word, same line count, same
    // target, and the half-site untouched.
    let (pair_text, pair_windows) = framed(&src("आ समम् अ", "आ समम् ५"), "the pair bite");
    assert_eq!(
        pair_windows, windows,
        "the bite moves no frame — same names, same count — so reading it off \
         the honest windows is honest"
    );
    let (bit_pairs, bit_halves) = read(&pair_text);
    assert_ne!(bit_pairs["समलङ्घनम्"], eq_pairs, "the pair really moved");
    assert_eq!(
        bit_halves["समलङ्घनम्"], eq_halves,
        "and the half-site did not, so the half reader has nothing to say"
    );
    let pair_reach = reach_verdict(&have_pairs, &have_halves, &bit_pairs, &bit_halves);
    assert_eq!(
        pair_reach,
        ReachVerdict::Pairs(vec!["समलङ्घनम्"]),
        "THE ARM FIRES, and it names the word that moved"
    );
    let said = pair_reach.complaint().expect("the pair arm reports");
    assert!(
        said.contains("TWO-SLOT") && !said.contains("HALF-SITES"),
        "and the sentence is the PAIR sentence, not the half one: {said}"
    );

    // ── FOUR: THE HALF SIDE FIRES ALONE. The constant moved ५ → ६, both inside
    // `IMMEDIATE`, so the object still carries the site and carries it with a
    // DIFFERENT value — and the pair reader drops the site entirely.
    let (half_text, half_windows) = framed(&src("अ समम् आ", "आ समम् ६"), "the half bite");
    assert_eq!(half_windows, windows, "no frame moved");
    let (bit_pairs, bit_halves) = read(&half_text);
    assert_eq!(
        bit_pairs["समलङ्घनम्"], eq_pairs,
        "the two-slot pair is untouched — the pair reader is BLIND to this bite"
    );
    assert_eq!(
        bit_halves["समलङ्घनम्"]
            .values()
            .flatten()
            .map(|(_, v, _)| *v)
            .collect::<Vec<i128>>(),
        vec![6i128],
        "and the half-site carries the moved constant"
    );
    let half_reach = reach_verdict(&have_pairs, &have_halves, &bit_pairs, &bit_halves);
    assert_eq!(
        half_reach,
        ReachVerdict::Halves(vec!["समलङ्घनम्"]),
        "THE OTHER ARM FIRES, on its own, and the two are NOT one bucket"
    );
    let said = half_reach.complaint().expect("the half arm reports");
    assert!(
        said.contains("HALF-SITES") && !said.contains("TWO-SLOT"),
        "with the half sentence and not the pair one: {said}"
    );

    // ── FIVE: BOTH AT ONCE IS ITS OWN STATE, not one of the two reported twice.
    let (both_text, both_windows) = framed(&src("आ समम् अ", "आ समम् ६"), "both bites");
    assert_eq!(both_windows, windows, "no frame moved");
    let (bit_pairs, bit_halves) = read(&both_text);
    let both_reach = reach_verdict(&have_pairs, &have_halves, &bit_pairs, &bit_halves);
    assert_eq!(
        both_reach,
        ReachVerdict::Both {
            pairs: vec!["समलङ्घनम्"],
            halves: vec!["समलङ्घनम्"],
        },
        "the fourth state, enumerated"
    );
    let said = both_reach.complaint().expect("the both arm reports");
    assert!(
        said.contains("TWO-SLOT") && said.contains("HALF-SITES"),
        "and it names BOTH sides on one line: {said}"
    );
}

/// **`cut_none` FIRES ON A COMPILED OBJECT, SO ITS CORPUS ZERO IS A MEASUREMENT
/// — AND THE THREE STATES OF [`CutVerdict`] ARE TOLD APART, ONE OBJECT EACH.**
///
/// The assert names four causes for an empty candidate set — a second cut, a
/// pool of another width, a declaration the harvester ranks wrongly, and the
/// dropped exchange — and had a fixture for NONE of them: the arm's message
/// prints four pieces of evidence no run had ever produced, and the bucket's
/// corpus zero said only that nothing had reached it. The sibling fixture
/// [`a_routine_whose_sites_need_two_different_cuts_is_reported_where_the_residue_reads_green`]
/// asks [`consistent_bases`] a HAND-BUILT pair list; this one compiles a
/// fixture three times over and hands the exhaustion an object the product's
/// own emitter wrote.
///
/// **THE CAUSE THIS FIXTURE EXHIBITS IS THE FIRST OF THE FOUR — A SECOND CUT.**
/// [`cut_the_pool_twice`] shifts the spill loads of the routine's LAST site by
/// `POOL_SLOTS` and leaves the first site's alone, which is what a lowering that
/// cut `अनामस्थलारम्भः` twice would emit.
///
/// **AND THE FIXTURE MUST BE ONE ONLY THIS CLAIM CAN SEE**, or it proves
/// nothing here: both weaker readings are asserted GREEN on the very object the
/// exhaustion refuses. The residue is green because eight is exactly the shift
/// (`slot ≡ rank (mod ८)` either side of it) and the order census is green
/// because both slots of a site move together.
///
/// **AND THE TWO CASES THAT MUST STILL BE REFUSED.**
///
/// * **ONE CUT IN A DIFFERENT PLACE IS RECOVERED, NOT REPORTED.** Shifting
///   EVERY site's loads is the pool cut at rank ० instead of rank ४ — still once
///   per routine — and the exhaustion must answer `Recovered(०)`. Without this
///   the arm would fire for any mutation at all and would be measuring the
///   mutation rather than the law.
/// * **AN AMBIGUOUS ROUTINE STAYS OUT OF THE BUCKET.** Declare the run LAST and
///   no site names the top rank, so two of the five candidates fit the honest
///   object and the verdict is `Ambiguous([३, ४])` — counted, its pair not
///   reported, and NOT in `cut_none`. One zero must not stand for two states.
///
/// Its own loader, and its own fixture.
#[test]
fn a_second_cut_part_way_through_a_routine_leaves_no_base_where_the_residue_reads_green() {
    // THE FIXTURE, parameterised at ONE point: where the run-typed `श` is
    // declared. `श ॱ दैर्घ्य` is what mints the pool at all (`ir.t1:1444`,
    // `खण्डदैर्घ्यरचना`), and it stands after every declaration in both, so the
    // cut falls at rank ४ — past all four names — either way. What moves is
    // whether a COMPARED name holds the top rank.
    let src = |first: &str, last: &str| {
        format!(
            "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
{first}
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
{last}
    अ भवति श ॱ दैर्घ्य ।
    यदि प न्यूनम् आ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि आ न्यूनम् प आदि
        प्रत्यागमनम् २ ।
    इति
    प्रत्यागमनम् ० ।
इति
"
        )
    };
    const DECL: &str = "    चरः श ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् कख इति ।";
    // This test's own loader. A refusal is NAMED, never folded into a green: a
    // fixture that does not compile carries no pair, and a non-empty want
    // against an empty have empties the candidate set — so a fixture that
    // failed to build would report `Unexplained` for the one reason this test
    // must never accept.
    let framed = |s: &str, what: &str| -> (String, Windows) {
        match compile_framed(s) {
            // `_frames`, three fields: `Framed::Emitted` carries the whole frame as
            // its third field and these tests read only the local windows. THIS IS
            // THE SECOND TIME this exact arity has had to be repaired at a merge --
            // the driver's lineage keeps being written against the two-field form
            // while main has carried three since SAS-016. A text merge cannot see an
            // arity, so it arrives as a clean merge that does not compile.
            Framed::Emitted(t, w, _frames) => (t, w),
            Framed::DeclaresNoModule => panic!("{what}: the fixture declares `मण्डलम् म`"),
            Framed::Refused(e) => panic!("{what}: the fixture compiles: {e}"),
        }
    };
    // The want side, and the exhaustion asked exactly as the sweep asks it —
    // through [`cut_groups`] and [`cut_halves`], so this fixture and the corpus
    // share one builder. The source writes no `समम्`/`असमम्`/`बृहत्समम्` head, so
    // both of those are EMPTY here and the `न्यूनलङ्घनम्` group is the whole
    // evidence.
    let asked = |s: &str, have: &Pairs, routines: &[SourceRoutine]| -> Vec<usize> {
        let (mut unreached, mut numeral) = (Vec::new(), 0usize);
        let want = predicted_pairs("म", routines, &mut unreached, &mut numeral);
        assert!(
            unreached.is_empty() && numeral == 0,
            "{s}: every site of this fixture is two names in reach: {unreached:?}"
        );
        let extra = extra_pairs("म", routines);
        assert!(
            extra.want.is_empty() && extra.half_want.is_empty(),
            "{s}: and it writes no other branch word, so the fold adds nothing"
        );
        consistent_bases_of(
            &cut_groups("मक", &want["मक"], have, None, &BTreeMap::new()),
            &cut_halves("मक", None, &BTreeMap::new()),
            routines[0].slots.len(),
        )
    };

    // ── ONE: THE HONEST OBJECT, AND ITS CUT RECOVERED AT RANK FOUR. `प` is
    // rank ०, `श` १, `अ` २, `आ` ३, so the two sites are ranks `(०, ३)` and
    // `(३, ०)` and only `b = ४` puts rank ३ at slot ३. Twelve local slots —
    // four names and the eight `अनामस्थलसंख्या` — so slot ११ is INSIDE the
    // window and a shifted load is read rather than dropped as a spill.
    let early = src(DECL, "");
    let routines = source_routines(&early);
    assert_eq!(
        routines[0].slots,
        ["प", "श", "अ", "आ"].map(str::to_string).to_vec(),
        "declaration order is the rank order (`ir.t1:1320`)"
    );
    let (text, windows) = framed(&early, "the honest object");
    assert_eq!(
        windows.get("मक").copied(),
        Some((0u64, 96u64)),
        "twelve local slots: four declared names and the anonymous pool"
    );
    let mut spilled = Vec::new();
    let have = emitted_pairs(&text, &windows, &mut spilled);
    assert!(spilled.is_empty(), "{spilled:?}");
    assert_eq!(
        have.get("मक").map(Vec::as_slice),
        Some([(0usize, 3usize), (3usize, 0usize)].as_slice()),
        "the honest object carries both sites at their declaration ranks"
    );
    assert_eq!(
        cut_verdict(&asked("the honest object", &have, &routines)),
        CutVerdict::Recovered(4),
        "one base explains both sites and no other does, so the pool's \
         position is RECOVERED from the object by exhaustion"
    );

    // ── TWO: THE ARM FIRES. The pool cut a SECOND time between the two sites:
    // the last site's two loads move by eight slots and the first site's do not.
    let (twice, moved) = cut_the_pool_twice(&text, 1);
    assert_eq!(
        moved, 2,
        "the mutation is LIVE — it moved the last site's two spill loads, and \
         a mutation that moved none would report `Unexplained` for nothing"
    );
    let mut spilled = Vec::new();
    let bit = emitted_pairs(&twice, &windows, &mut spilled);
    assert!(
        spilled.is_empty(),
        "AND THE SHIFTED SLOTS ARE INSIDE THE WINDOW. A slot past it is dropped \
         as a spill, which empties the candidate set for a reason that is not a \
         contradiction at all: {spilled:?}"
    );
    assert_eq!(
        bit.get("मक").map(Vec::as_slice),
        Some([(0usize, 3usize), (11usize, 8usize)].as_slice()),
        "one site below the second cut and one above it"
    );
    assert_eq!(
        cut_verdict(&asked("the object cut twice", &bit, &routines)),
        CutVerdict::Unexplained,
        "NO base puts rank ३ at slot ३ and rank ३ at slot ११ in one frame — \
         the arm `cut_none` counts, fired on a compiled object"
    );

    // ── THREE: THE CONTROL, AND IT IS WHY THE PER-ROUTINE CLAIM EXISTS. Both
    // weaker readings are GREEN on that same object.
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    assert!(
        residue_shortfalls(&want, &bit).is_empty(),
        "THE CONTROL: the shift is exactly `अनामस्थलसंख्या`, so every site still \
         satisfies `slot ≡ rank (mod ८)` and the residue reads this green"
    );
    assert!(
        shortfalls(&directions_of(&want), &directions_of(&bit)).is_empty(),
        "AND SO DOES THE ORDER CENSUS: both slots of a site moved together, so \
         one ascending and one descending, before and after"
    );
    assert!(
        pairable(&want, &routines, "म", &windows).is_empty(),
        "AND THE EXACT-PAIR CLAIM DOES NOT REACH THIS ROUTINE AT ALL — twelve \
         emitted slots against four declared names is `pairable`'s own refusal \
         (`ir.t1:1345`: the pool's position is unknown to a reader of the \
         declarations), so those two greens are the whole of what this object \
         would otherwise have answered for"
    );

    // ── FOUR, REFUSED: ONE CUT IN A DIFFERENT PLACE IS RECOVERED. Every site's
    // loads shifted is the pool cut at rank ० rather than rank ४ — still ONCE
    // per routine — and the exhaustion must answer the new base, not report the
    // routine. Without this the arm would fire for any mutation whatever.
    let (whole, moved) = cut_the_pool_twice(&text, 0);
    assert_eq!(moved, 4, "all four loads of both sites");
    let mut spilled = Vec::new();
    let shifted = emitted_pairs(&whole, &windows, &mut spilled);
    assert!(spilled.is_empty(), "{spilled:?}");
    assert_eq!(
        shifted.get("मक").map(Vec::as_slice),
        Some([(8usize, 11usize), (11usize, 8usize)].as_slice()),
        "both sites eight slots on"
    );
    assert_eq!(
        cut_verdict(&asked("the pool moved whole", &shifted, &routines)),
        CutVerdict::Recovered(0),
        "THE CASE THAT MUST STILL BE REFUSED: the pool in front of every name \
         is one cut at rank ०, and a claim that reported it would be measuring \
         the mutation and not the law"
    );

    // ── FIVE, REFUSED: AN AMBIGUOUS ROUTINE STAYS OUT OF THE BUCKET. `श`
    // declared LAST takes rank ३ and no site names it, so the cut may fall at
    // rank ३ or rank ४ and the object cannot say which. The existence claim
    // stands; the position does not.
    let late = src("", DECL);
    let routines = source_routines(&late);
    assert_eq!(
        routines[0].slots,
        ["प", "अ", "आ", "श"].map(str::to_string).to_vec(),
        "the same four names, the run one last"
    );
    let (text, windows) = framed(&late, "the ambiguous object");
    let mut spilled = Vec::new();
    let have = emitted_pairs(&text, &windows, &mut spilled);
    assert!(spilled.is_empty(), "{spilled:?}");
    assert_eq!(
        have.get("मक").map(Vec::as_slice),
        Some([(0usize, 2usize), (2usize, 0usize)].as_slice()),
        "ranks `(०, २)` and `(२, ०)` — the top rank is never compared"
    );
    assert_eq!(
        cut_verdict(&asked("the ambiguous object", &have, &routines)),
        CutVerdict::Ambiguous(vec![3, 4]),
        "TWO of the five candidates fit, so the position is AMBIGUOUS — a \
         verdict of its own and never `Unexplained`"
    );
}

/// **THE THIRD WIDENING'S BLINDNESS CONTROL, MADE TO FIRE — AND THE LAST OF THE
/// THREE.**
///
/// `half_wide_bit` asserts that no cut recovered from name-against-constant
/// heads alone MOVES under [`drop_the_exchange`], and nothing had ever moved it.
/// It was an inline `bases.is_empty()` skip plus a `!=`, asserted empty with
/// nothing in it — the same shape `wide_bit_total` carried before
/// [`WideBite`] was extracted, and the same question asked the same way.
///
/// **BUT THE WIRING IT MUST CATCH IS A DIFFERENT ONE.** The second widening
/// reads its evidence through [`cut_groups`], so its leak is an `lt_want` one.
/// The third's pair groups are BOTH empty — `extra_want` is `None` and the
/// `न्यूनलङ्घनम्` want is [`NO_LT_SITES`] — so no `lt_want` a later cycle folded
/// in could reach it at all. Its whole evidence goes through [`cut_halves`],
/// and the only thing that can leak into it is a `न्यूनलङ्घनम्` entry in the HALF
/// want: the shape a cycle that "narrowed the third widening further with the
/// routine's own `न्यूनम्`/`अधिकम्` constants" would produce. That is what is
/// written out here.
///
/// **AND IT IS THE SIDE BIT THAT MOVES, WHICH IS WHY THIS FIXTURE IS NOT THE
/// SECOND WIDENING'S.** `आ न्यूनम् ७` emits `न्यूनलङ्घनम् <slot>न ७त्` — ONE slot
/// and ONE constant, so [`emitted_pairs`] is blind to it and this object carries
/// no two-slot pair at all. [`drop_the_exchange`] exchanges those two operands,
/// and [`emitted_halves_of`] reads the result as the SAME slot and the SAME
/// constant with `karana` FALSE (see [`Half`] — `क न्यूनम् ५` and `५ न्यूनम् क`
/// are different branches). So a half want that reads `न्यूनलङ्घनम्` moves and a
/// pair want could not, on one object.
///
/// **AND IT HAD TO BE A UNIT FIXTURE.** [`route_half_sites`] hands
/// [`cut_halves`] the routine's own `half_want`, whose keys all come from
/// [`branch_of`] and can never be `न्यूनलङ्घनम्`, and the sweep's `half_have` map
/// is built over [`EXTRA_WORDS`] alone — so the leak is unreachable from the
/// corpus by construction, and the assert's sentence is only falsifiable against
/// a want built to carry the fourth word.
///
/// **THE THREE CASES THAT MUST STILL BE REFUSED.**
/// - The sweep's own wiring — the routine's own `half_want` over
///   [`EXTRA_WORDS`], on the SAME object and the SAME bite — leaves the bases
///   IDENTICAL, and that quiet is LIVE: the mutation is asserted to flip THIS
///   object's `न्यूनलङ्घनम्` half-site's side bit, or the quiet is a no-op's.
/// - A routine with NO half-site evidence stays out. An empty half want adds no
///   group at all, so every candidate survives vacuously and its two readings
///   agree for free — one zero would stand for two.
/// - A routine with NO frame lands in `half_no_frame` and never in
///   `half_bases`: [`emitted_halves_of`] carries no triple for it, every
///   candidate dies on a want the object cannot answer, and [`wide_bite`]
///   answers [`WideBite::Unrecovered`] rather than [`WideBite::Moved`] even
///   against a NON-EMPTY bitten reading.
///
/// Its own loader, and its own fixture — compiled by the product's own front
/// end, so what the readers are handed is an object the emitter wrote.
#[test]
fn a_half_site_cut_handed_a_lt_half_want_moves_under_the_dropped_exchange() {
    // THE FIXTURE, parameterised at the branch list alone: the `समम्` head is a
    // name against a CONSTANT — the third widening's whole evidence, and absent
    // from `BARE` — while `आ न्यूनम् ७` is in BOTH and is the half-site the
    // mutation bites. NEITHER is a two-name head, so both pair groups stay
    // empty and this routine is exactly the shape `HalfRoute::ThirdWidened`
    // names.
    let src = |branches: &str| {
        format!(
            "\
मण्डलम् म ॥
सार्वजनिक वृत्तिः क आदाय प ॱॱ न६४ ददाति न६४ आदि
    चरः अ ॱॱ न६४ भवति ० ।
    चरः आ ॱॱ न६४ भवति ० ।
{branches}    यदि आ न्यूनम् ७ आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् ० ।
इति
"
        )
    };
    // This test's own loader. A refusal is NAMED and never folded into a green:
    // a fixture that does not compile carries no object at all, and every
    // comparison below would be an equality between two absences.
    let framed = |s: &str, what: &str| -> (String, Windows) {
        match compile_framed(s) {
            // `_frames`, three fields: `Framed::Emitted` carries the whole frame as
            // its third field and these tests read only the local windows. THIS IS
            // THE SECOND TIME this exact arity has had to be repaired at a merge --
            // the driver's lineage keeps being written against the two-field form
            // while main has carried three since SAS-016. A text merge cannot see an
            // arity, so it arrives as a clean merge that does not compile.
            Framed::Emitted(t, w, _frames) => (t, w),
            Framed::DeclaresNoModule => panic!("{what}: the fixture declares `मण्डलम् म`"),
            Framed::Refused(e) => panic!("{what}: the fixture compiles: {e}"),
        }
    };
    let half_src = src("    यदि प समम् ५ आदि\n        प्रत्यागमनम् २ ।\n    इति\n");
    let routines = source_routines(&half_src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 3, "प, अ, आ — three names, so four candidate cuts");
    let extra = extra_pairs("म", &routines);
    assert_eq!(
        (extra.claimed, extra.half_claimed, extra.numeral),
        (0, 1, 1),
        "the harvest is REAL, it is ONE half-site, and it claims no pair: {:?}",
        extra.half_want
    );
    assert!(
        extra.want.is_empty(),
        "and NO two-name head of any word, which is what makes this routine the \
         THIRD widening's and not the second's: {:?}",
        extra.want
    );
    let by_word = extra.half_want["मक"].clone();
    let half_want = Some(&by_word);
    let (text, windows) = framed(&half_src, "the honest object");
    let bitten_text = drop_the_exchange(&text);

    // The readers, off the HONEST windows for both objects, exactly as the sweep
    // does it: a bite that moved the frames would be a different mutation.
    let read_halves = |t: &str| -> BTreeMap<&'static str, HalfPairs> {
        EXTRA_WORDS
            .into_iter()
            .map(|m| (m, emitted_halves_of(t, &windows, m, &mut Vec::new())))
            .collect()
    };
    let read_pairs = |t: &str| -> BTreeMap<&'static str, Pairs> {
        EXTRA_WORDS
            .into_iter()
            .map(|m| (m, emitted_pairs_of(t, &windows, m, &mut Vec::new())))
            .collect()
    };
    let half_have = read_halves(&text);
    let half_bitten = read_halves(&bitten_text);
    let extra_have = read_pairs(&text);
    let extra_bitten = read_pairs(&bitten_text);
    let lt_have = emitted_pairs(&text, &windows, &mut Vec::new());
    let lt_bitten = emitted_pairs(&bitten_text, &windows, &mut Vec::new());
    let lt_half_have = emitted_halves_of(&text, &windows, "न्यूनलङ्घनम्", &mut Vec::new());
    let lt_half_bitten = emitted_halves_of(&bitten_text, &windows, "न्यूनलङ्घनम्", &mut Vec::new());

    // ── ONE: THE FIXTURE CARRIES THE EVIDENCE AND CARRIES NO PAIR. Without the
    // first, every comparison below could be quiet because there is nothing to
    // compare; without the second this would be the second widening's fixture
    // with an extra head.
    assert_eq!(
        half_have["समलङ्घनम्"]["मक"],
        vec![(0usize, 5i128, true)],
        "`प समम् ५` — प at local slot ०, the constant ५, and प is the करण"
    );
    assert!(
        lt_have.is_empty() && extra_have.values().all(BTreeMap::is_empty),
        "and the object carries NO two-slot pair under any of the four words, \
         so both of `cut_groups`' groups are empty: {lt_have:?} {extra_have:?}"
    );

    // ── TWO: AND THE BITE IS LIVE, ON THE `न्यूनलङ्घनम्` HALF-SITE. The
    // mutation flips the side bit of a head the pair readers cannot see, and it
    // reaches no other branch word — the two halves of the refusal in part
    // FOUR, each asserted rather than assumed.
    assert_eq!(
        lt_half_have["मक"],
        vec![(2usize, 7i128, true)],
        "`आ न्यूनम् ७` — आ at local slot २ against ७, आ the करण"
    );
    assert_eq!(
        lt_half_bitten["मक"],
        vec![(2usize, 7i128, false)],
        "and `drop_the_exchange` DOES bite it: the same slot, the same constant, \
         the side FLIPPED — or part FOUR proves nothing"
    );
    assert_eq!(
        (&half_bitten, &extra_bitten),
        (&half_have, &extra_have),
        "and it reaches NO other branch word, so the third widening's own \
         evidence is carried identically by both objects"
    );

    // ── THREE: THE HONEST RECOVERY, and it is a KNOWN answer rather than "not
    // empty". `slot_at(०, b)` is ० exactly when `० < b`, so the one half-site
    // keeps three of the four candidates — the AMBIGUOUS shape this recovery
    // usually lands in, and what the bucket compares is the SET.
    let honest = consistent_bases_of(
        &cut_groups("मक", NO_LT_SITES, &lt_have, None, &extra_have),
        &cut_halves("मक", half_want, &half_have),
        names,
    );
    assert_eq!(
        honest,
        vec![1usize, 2, 3],
        "the `समम्` half-site puts rank ० below the cut, which the three \
         non-zero bases do and `b = ०` does not"
    );

    // ── FOUR, REFUSED: THE SWEEP'S OWN WIRING IS QUIET. `half_want`'s keys all
    // come from `branch_of` and the have map is built over `EXTRA_WORDS`, so no
    // group of this reading names `न्यूनलङ्घनम्` and the base cannot move — on an
    // object whose `न्यूनलङ्घनम्` half-site part TWO showed the mutation really
    // does rewrite.
    assert_eq!(
        wide_bite(
            &honest,
            &cut_groups("मक", NO_LT_SITES, &lt_bitten, None, &extra_bitten),
            &cut_halves("मक", half_want, &half_bitten),
            names,
        ),
        WideBite::Quiet,
        "THE CASE THAT MUST STILL BE REFUSED: the third widening takes no \
         evidence from `न्यूनलङ्घनम्`, so the sweep's zero is a reach and not an \
         absence"
    );

    // ── FIVE: AND THE DEFECT FIRES. The SAME object, the SAME bite, the SAME
    // exhaustion — the one thing changed is that the HALF want gains a
    // `न्यूनलङ्घनम्` entry and the have map gains the same word, which is the
    // wiring `half_wide_bit` exists to catch. `slot_at(२, b)` is २ only at
    // `b = ३`, so the honest reading narrows to one; the bitten object carries
    // that triple with the side flipped, so the recovery goes EMPTY.
    let mut leaked = by_word.clone();
    leaked.insert("न्यूनलङ्घनम्", vec![(2usize, 7i128, true)]);
    let with_lt = |base: &BTreeMap<&'static str, HalfPairs>, lt: &HalfPairs| {
        let mut m = base.clone();
        m.insert("न्यूनलङ्घनम्", lt.clone());
        m
    };
    let honest_leaked = consistent_bases_of(
        &cut_groups("मक", NO_LT_SITES, &lt_have, None, &extra_have),
        &cut_halves("मक", Some(&leaked), &with_lt(&half_have, &lt_half_have)),
        names,
    );
    assert_eq!(
        honest_leaked,
        vec![3usize],
        "folding the `न्यूनलङ्घनम्` constant in narrows the honest reading to one \
         base — which is exactly why a later cycle would be tempted to do it"
    );
    let moved = wide_bite(
        &honest_leaked,
        &cut_groups("मक", NO_LT_SITES, &lt_bitten, None, &extra_bitten),
        &cut_halves("मक", Some(&leaked), &with_lt(&half_bitten, &lt_half_bitten)),
        names,
    );
    assert_eq!(
        moved,
        WideBite::Moved {
            honest: vec![3usize],
            bitten: Vec::new(),
        },
        "a half-site recovery that READS `न्यूनलङ्घनम्` triples moves under a \
         mutation confined to them"
    );
    // AND THE HONEST WIRING IS NOT MERELY QUIET, IT IS DIFFERENT: the two
    // readings of the same bitten object disagree, so part FOUR's zero is the
    // absence of this defect and not the absence of a question.
    assert_ne!(
        moved,
        WideBite::Quiet,
        "the two wirings answer differently on one object, which is what makes \
         the bucket an instrument"
    );
    let how = moved
        .complaint(HALF_EVIDENCE)
        .expect("the moved arm is a complaint");
    assert!(
        how.contains("name-against-constant heads alone — honest [3], bitten []"),
        "and the sentence the sweep pushes names the READING and BOTH bases: {how}"
    );
    assert_ne!(
        moved.complaint(HALF_EVIDENCE),
        moved.complaint(WIDE_EVIDENCE),
        "and the two buckets that share this decision report DIFFERENT \
         sentences, or `half_wide_bit`'s defect would be filed in \
         `wide_bit_total`'s words"
    );

    // ── SIX, REFUSED: A ROUTINE WITH NO HALF-SITE EVIDENCE STAYS OUT.
    // `extra_pairs` gives it no want, so `route_half_sites` never enters it and
    // `half_bases` has no key for it — and it must, because an empty half want
    // adds NO GROUP at all: every candidate survives vacuously, its two
    // readings agree for free, and that zero would look exactly like part
    // FOUR's while saying nothing.
    let bare_src = src("");
    let bare_routines = source_routines(&bare_src);
    let bare_extra = extra_pairs("म", &bare_routines);
    assert!(
        bare_extra.half_want.is_empty() && bare_extra.half_claimed == 0,
        "no name-against-constant head under any of the three words: {:?}",
        bare_extra.half_want
    );
    let (bare_text, bare_windows) = framed(&bare_src, "the evidence-free object");
    let bare_lt_half = emitted_halves_of(&bare_text, &bare_windows, "न्यूनलङ्घनम्", &mut Vec::new());
    let bare_lt_bitten = emitted_halves_of(
        &drop_the_exchange(&bare_text),
        &bare_windows,
        "न्यूनलङ्घनम्",
        &mut Vec::new(),
    );
    assert_ne!(
        bare_lt_half, bare_lt_bitten,
        "the mutation bites this object too, so the free agreement below is not \
         the absence of a bite"
    );
    let no_pairs = Pairs::new();
    let no_extra: BTreeMap<&'static str, Pairs> = BTreeMap::new();
    let no_halves: BTreeMap<&'static str, HalfPairs> = BTreeMap::new();
    let all_bases = consistent_bases_of(
        &cut_groups("मक", NO_LT_SITES, &no_pairs, None, &no_extra),
        &cut_halves("मक", None, &no_halves),
        names,
    );
    assert_eq!(
        all_bases,
        vec![0usize, 1, 2, 3],
        "an empty half want adds no group, so every one of the four candidates \
         survives"
    );
    assert_eq!(
        wide_bite(
            &all_bases,
            &cut_groups("मक", NO_LT_SITES, &no_pairs, None, &no_extra),
            &cut_halves("मक", None, &no_halves),
            names,
        ),
        WideBite::Quiet,
        "so such a routine would report QUIET for free — which is why the \
         bucket is entered from `half_want` and not from every routine"
    );

    // ── SEVEN, REFUSED: A ROUTINE WITH NO FRAME IS `half_no_frame`'s AND NEVER
    // A BITE. `route_half_sites` decides the frameless arm BEFORE the
    // exhaustion runs, so such a routine never reaches `half_bases` and this
    // bucket cannot see it at all.
    let mut want_map: BTreeMap<String, ByWordHalf> = BTreeMap::new();
    want_map.insert("मक".to_string(), by_word.clone());
    let frameless = route_half_sites(
        "fixture",
        &want_map,
        &Pairs::new(),
        &Pairs::new(),
        &BTreeMap::new(),
        &Windows::new(),
        &no_pairs,
        &no_extra,
        &no_halves,
        &|_| names,
    );
    assert_eq!(
        frameless.route["मक"],
        HalfRoute::NoFrame,
        "the frameless routine is ROUTED, not exhausted"
    );
    assert!(
        frameless.bases.is_empty() && frameless.no_frame.len() == 1,
        "so it enters `half_no_frame` and NOT `half_bases`: {:?} {:?}",
        frameless.bases,
        frameless.no_frame
    );
    // AND WHAT WOULD HAVE HAPPENED HAD IT BEEN EXHAUSTED, which is why the arm
    // is decided first: `emitted_halves_of` reads `windows` and skips, so the
    // HAVE side is empty and every candidate dies on a want the object cannot
    // answer. `wide_bite` names that `Unrecovered` even against the NON-EMPTY
    // bitten reading of the framed object above — a boolean `!=` would have
    // called it a move.
    let frameless_bases = consistent_bases_of(
        &cut_groups("मक", NO_LT_SITES, &no_pairs, None, &no_extra),
        &cut_halves("मक", half_want, &no_halves),
        names,
    );
    assert!(
        frameless_bases.is_empty(),
        "no base explains a want the object carries nothing for: \
         {frameless_bases:?}"
    );
    assert_eq!(
        wide_bite(
            &frameless_bases,
            &cut_groups("मक", NO_LT_SITES, &lt_have, None, &extra_have),
            &cut_halves("मक", half_want, &half_have),
            names,
        ),
        WideBite::Unrecovered,
        "THE CASE THAT MUST STILL BE REFUSED: a routine with no base is \
         `half_wide_none`'s to report and not this bucket's, however non-empty \
         the bitten reading is"
    );
}

/// **THE RED DIRECTION OF THE SUBSET LAW, MADE TO FIRE — SO `cut_none`'s CORPUS
/// ZERO IS A MEASUREMENT AND NOT A CONSTRUCTION.**
///
/// `not_monotone`'s own margin names the direction it cannot watch: a crossed
/// grouping may EMPTY the candidate set, an empty set IS a subset, so the law
/// stays quiet and `cut_none` (`t1_corpus_two_name_routines_cut_unexplained`) is
/// the arm that must speak. Nothing had ever put a routine in it. The pair-only
/// shape of the emptying is witnessed in
/// [`a_routine_whose_sites_need_two_different_cuts_is_reported_where_the_residue_reads_green`]
/// through [`consistent_bases`] — ONE group, no fold — and the half-only shape
/// in [`a_half_routine_no_single_cut_explains_is_reported_unexplained_not_ambiguous`]
/// through [`route_half_sites`]'s own map. **NEITHER IS THE BUCKET THIS
/// TEST IS ABOUT.** `cut_none` comes off `cut_bases`, which is the FOLDED
/// reading — [`cut_groups`] and [`cut_halves`] together — and the sweep's own
/// note records what that cost the first time the arm ever printed: a red
/// reading `predicted ranks [(२, ३)], the object carries [(२, ३)]`, two sets
/// that AGREE, because the constraint that actually failed was a HALF-SITE the
/// message did not print. An arm that has never fired cannot be known to name
/// its own evidence.
///
/// **SO THE CONTRADICTION HERE LIVES IN THE HALF-SITE AND NOWHERE ELSE**, which
/// is the one shape that tells the folded bucket apart from both siblings: the
/// `न्यूनलङ्घनम्` group and the `समलङ्घनम्` PAIR group leave `b = १०` standing
/// between them — part TWO is that same object read with an honest half-site and
/// it is RECOVERED — and folding the half-sites in is what empties the set.
///
/// **WHICH OF THE FOUR CAUSES THE FIXTURE EXHIBITS, said rather than left to the
/// reader.** The `cut_none` assert names four — a second cut, a cut of another
/// width, a declaration the harvester ranks wrongly, or the dropped exchange —
/// and this object is the FIRST: the eight slots appear before rank २ (`आ` at
/// slot १०) and NOT before ranks ८ and ९ (`ॠ` and `ऌ` at slots ८ and ९). A
/// single cut at base `b` shifts exactly the ranks `{r : r ≥ b}`, an UP-SET, and
/// `{२}` is not one. The width is exactly eight, so it is not the second cause;
/// both operands of every site move together, so it is not the fourth. **AND
/// THE THIRD CAUSE PRODUCES THE SAME OBJECT** — a harvester that ranked `आ` at
/// ८ or later would predict slot १० for it and agree — which is why the bucket
/// names four causes and its message PRINTS THE EVIDENCE rather than diagnosing.
///
/// **THE CASES THAT MUST STILL BE REFUSED.**
///
/// The residue reads an emptied routine GREEN by letting each site pick its own
/// side of the cut, so a fixture the residue can also see proves nothing here:
/// part FOUR asserts both residues — [`residue_shortfalls`] on the pair and
/// [`half_residue_shortfalls`] on the half — are SILENT on the very object no
/// single cut explains, since `१० ≡ २ (mod ८)`.
///
/// An AMBIGUOUS routine must stay OUT of the bucket, or one zero stands for two:
/// part FIVE is the same source and an object carrying each want on both sides
/// of the cut, and it leaves EIGHT candidates, so it is `cut_ambiguous`'s.
///
/// And an empty set has TWO causes and only one of them is this bucket's claim.
/// An object that carries NOTHING for a want empties the set as well, and that
/// is a SHORTFALL — part SIX builds it as a named control and keeps it distinct
/// by the test the witness passes and it fails: in part THREE every group ALONE
/// leaves a base standing, and only their intersection is empty.
///
/// **AND THAT DISTINCTION WAS PROVED LIVE RATHER THAN ONLY ASSERTED.** Moving
/// the witness's half-site one slot further — `११` instead of `१०`, off the
/// residue — still empties the folded set, so a test that only checked for the
/// empty set would have read it as the same finding. Part THREE refuses it at
/// the group-alone assert instead, because `११` is no cut's slot for rank `२`:
/// an off-residue slot is a SHORTFALL and this fixture is not allowed to be one.
/// Dropping the half fold entirely — `cut_halves`'s groups replaced by `&[]` —
/// reds part THREE and nothing before it, which is the other direction: the
/// `न्यूनलङ्घनम्` and pair groups leave `b = १०` standing on their own.
///
/// So all three arms of the `match bases.len()` the sweep runs over `cut_bases`
/// fire on ONE source and three objects that differ only in what they carry —
/// `0` → `cut_none`, `1` → `cut_recovered`, `k ≥ २` → `cut_ambiguous` — and the
/// corpus zero therefore SAYS something: no object in reach contradicts itself
/// across its own folded sites.
///
/// Nothing here pushes a message or bumps a counter: every candidate set read
/// below is [`consistent_bases_of`]'s answer through the sweep's own two
/// builders. Its own loader, its own source, and its own objects.
#[test]
fn a_routine_the_half_fold_leaves_unexplained_is_reported_none_and_not_recovered() {
    // THIS TEST'S OWN SOURCE. Ten declared names — `प` at rank ०, then `अ` …
    // `ऌ` at १ … ९ in declaration order (`ir.t1:1320`) — so eleven candidate
    // cuts. Three heads and three kinds: one `न्यूनम्` two-name, one `समम्`
    // two-name, one `समम्` name-against-numeral. The folded reading is the only
    // one that sees all three.
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
    यदि प न्यूनम् अ आदि
        प्रत्यागमनम् १ ।
    इति
    यदि ॠ समम् ऌ आदि
        प्रत्यागमनम् २ ।
    इति
    यदि आ समम् ३ आदि
        प्रत्यागमनम् ३ ।
    इति
    प्रत्यागमनम् ० ।
इति
";
    let routines = source_routines(src);
    assert_eq!(routines.len(), 1);
    let names = routines[0].slots.len();
    assert_eq!(names, 10, "प, अ … ऌ — ten names, so eleven candidate cuts");

    // ── ONE: THE WANTS ARE HARVESTED, NOT INVENTED. Only the OBJECTS below are
    // built; every rank the exhaustion is fed comes off this source through the
    // same two harvesters the sweep uses.
    let (mut unreached, mut numeral) = (Vec::new(), 0usize);
    let want = predicted_pairs("म", &routines, &mut unreached, &mut numeral);
    assert!(unreached.is_empty() && numeral == 0, "{unreached:?}");
    let lt = want["मक"].clone();
    assert_eq!(
        lt.as_slice(),
        [(0usize, 1usize)].as_slice(),
        "प ० against अ १"
    );
    let extra = extra_pairs("म", &routines);
    assert!(
        extra.refused.is_empty() && extra.half_out_of_range.is_empty(),
        "two scalar `समम्` heads, nothing refused: {:?} {:?}",
        extra.refused,
        extra.half_out_of_range
    );
    assert_eq!(
        (extra.seen, extra.claimed, extra.numeral, extra.half_claimed),
        (2, 1, 1, 1),
        "two heads seen, one folded as a pair and one as a half-site"
    );
    assert_eq!(
        extra.want["मक"]["समलङ्घनम्"],
        vec![(8usize, 9usize)],
        "ranks ॠ ८ and ऌ ९, in WRITTEN order"
    );
    assert_eq!(
        extra.half_want["मक"]["समलङ्घनम्"],
        vec![(2usize, 3i128, true)],
        "आ at rank २ against the constant ३, the name in the करण"
    );

    // The objects, built one reading at a time and handed to the sweep's OWN
    // two builders — so the honest and the crooked readings differ in what they
    // carry and in nothing else.
    let carried =
        |ps: Vec<(usize, usize)>| -> Pairs { [("मक".to_string(), ps)].into_iter().collect() };
    let under = |ps: Vec<(usize, usize)>| -> BTreeMap<&'static str, Pairs> {
        [("समलङ्घनम्", carried(ps))].into_iter().collect()
    };
    let halves_under = |hs: Vec<Half>| -> BTreeMap<&'static str, HalfPairs> {
        [(
            "समलङ्घनम्",
            [("मक".to_string(), hs)].into_iter().collect::<HalfPairs>(),
        )]
        .into_iter()
        .collect()
    };
    let bases = |lt_have: &Pairs,
                 other: &BTreeMap<&'static str, Pairs>,
                 half_have: &BTreeMap<&'static str, HalfPairs>| {
        consistent_bases_of(
            &cut_groups("मक", &lt, lt_have, extra.want.get("मक"), other),
            &cut_halves("मक", extra.half_want.get("मक"), half_have),
            names,
        )
    };
    let lt_have = carried(vec![(0, 1)]);
    let pair_have = under(vec![(8, 9)]);

    // ── TWO: THE HONEST OBJECT, RECOVERED. `(०, १)` carried as written leaves
    // every cut at rank २ or later; `(८, ९)` carried as written puts the cut
    // strictly after rank ९, so `b = १०`; and `आ` at slot २ — below the cut —
    // agrees. ONE candidate, so this routine's cut is RECOVERED and the arm
    // taken is `1`.
    let honest_half = halves_under(vec![(2, 3, true)]);
    assert_eq!(
        bases(&lt_have, &pair_have, &honest_half),
        vec![10usize],
        "one cut position explains all three sites at once, so it is the position"
    );

    // ── THREE: THE WITNESS. The same object with `आ` at slot १० = २ + ८ —
    // ABOVE a cut — while `ॠ` and `ऌ` stay at ८ and ९, BELOW one. No single `b`
    // does both: the half demands `b ≤ २` and the `समम्` pair demands `b = १०`.
    // The arm taken is `0`, and it is the RED direction the subset law is blind
    // to.
    let crooked_half = halves_under(vec![(10, 3, true)]);
    assert!(
        bases(&lt_have, &pair_have, &crooked_half).is_empty(),
        "the eight slots appear before rank २ and not before ranks ८ and ९ — a \
         single cut shifts an UP-SET of ranks and {{२}} is not one"
    );
    // AND EVERY GROUP ALONE LEAVES A BASE, which is what makes this a
    // CONTRADICTION rather than the shortfall of part SIX: each site is
    // individually explicable and only the three together are not.
    let no_pairs: BTreeMap<&'static str, Pairs> = BTreeMap::new();
    assert_eq!(
        consistent_bases(&lt, &lt_have["मक"], names),
        (2..=10).collect::<Vec<usize>>(),
        "nine cut positions fit the one `न्यूनलङ्घनम्` alone"
    );
    let pairs_only = consistent_bases_of(
        &cut_groups("मक", &lt, &lt_have, extra.want.get("मक"), &pair_have),
        &[],
        names,
    );
    assert_eq!(
        pairs_only,
        vec![10usize],
        "AND THE PAIR FOLD ALONE RECOVERS IT: without the half-sites this very \
         routine is reported RECOVERED at `b = १०`, which is why the emptying \
         belongs to the folded bucket and to neither sibling"
    );
    assert_eq!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT_SITES, &Pairs::new(), None, &no_pairs),
            &cut_halves("मक", extra.half_want.get("मक"), &crooked_half),
            names,
        ),
        vec![0usize, 1, 2],
        "and the half-site alone is satisfied by every cut at rank २ or earlier"
    );

    // ── FOUR: THE CONTROL, AND THE REASON THE PER-ROUTINE CLAIM EXISTS. Both
    // residues read the object of part THREE GREEN — `१० ≡ २ (mod ८)`, so the
    // half-site's slot is exactly what its rank demands, and the pair side was
    // never touched. A fixture the residue can also see would prove nothing
    // here.
    assert!(
        residue_shortfalls(&want, &lt_have).is_empty(),
        "the `न्यूनलङ्घनम्` residue is satisfied"
    );
    let ambiguous: std::collections::BTreeSet<String> = ["मक".to_string()].into_iter().collect();
    assert!(
        half_residue_shortfalls(&ambiguous, &extra.half_want, &crooked_half).is_empty(),
        "THE CONTROL: the HALF residue is satisfied by the object no single cut \
         explains — the per-site claim lets each site pick its own side of the \
         cut, and this is the per-routine claim catching what it cannot"
    );

    // ── FIVE: AMBIGUITY IS NOT THIS BUCKET'S. An object carrying each want on
    // BOTH sides of the cut satisfies the FLOOR every group is checked against
    // — `यावत्` rotation is why it is a floor and not an equality — and leaves
    // eight candidates: `(८, ९)` for `b = १०` and `(१६, १७)` for every `b ≤ ८`,
    // with `b = ९` alone refused because it would split the pair. The arm taken
    // is `k ≥ २`.
    let doubled_pair = under(vec![(8, 9), (16, 17)]);
    let doubled_half = halves_under(vec![(2, 3, true), (10, 3, true)]);
    assert_eq!(
        bases(&lt_have, &doubled_pair, &doubled_half),
        vec![2usize, 3, 4, 5, 6, 7, 8, 10],
        "eight cut positions fit, so the object does not determine one and the \
         exact pair is not reported for it"
    );

    // ── SIX: THE SHORTFALL, NAMED AND KEPT DISTINCT. An object that carries
    // NOTHING under `समलङ्घनम्` empties the set too — a non-empty want against
    // an empty have kills every candidate — but that is the half-site reader
    // carrying nothing, not the routine contradicting itself. The test that
    // tells them apart is the one part THREE passes: there, every group alone
    // leaves a base.
    let nothing: BTreeMap<&'static str, HalfPairs> = BTreeMap::new();
    assert!(
        bases(&lt_have, &pair_have, &nothing).is_empty(),
        "an empty have empties the set as well"
    );
    assert!(
        consistent_bases_of(
            &cut_groups("मक", NO_LT_SITES, &Pairs::new(), None, &no_pairs),
            &cut_halves("मक", extra.half_want.get("मक"), &nothing),
            names,
        )
        .is_empty(),
        "AND HERE THE HALF GROUP ALONE IS UNSATISFIABLE, which is the shape of \
         a shortfall — in part THREE the same group alone left three bases"
    );
}

/// **THE CASES THAT MUST STILL BE REFUSED.**
///
/// Built by hand rather than compiled, because the corpus does not make them:
/// the whole finding above is that the corpus spills in 17 of 870 routines and
/// compares a spill in none of them, so a reader that answered `Local` for
/// EVERY offset would be green on every object this tree can build. Six bands,
/// six frames.
#[test]
fn each_frame_region_is_refused_on_a_frame_that_really_has_one() {
    use emitted::{SlotBand, slot_band};

    // Two spill slots, three locals, one saved `स्थिर`, `पुनःस्थानम्` at the top.
    //   ०,८         spill ०,१
    //   १६,२४,३२    local ०,१,२
    //   ४०          the saved स्थिर
    //   ५६          पुनःस्थानम्   (bytes = ६४, so ४८ is the rounding pad)
    let frame = riscv64::Frame {
        bytes: 64,
        saved: vec![(0, 40)],
        ra_offset: 56,
        num_spills: 2,
        num_locals: 3,
    };

    // ── ONE: THE SPILL REGION IS NOT THE LOCAL REGION. Offset ० is spill slot
    // ० and NOT local slot ०, and the whole point of this reader is that those
    // two are different things wearing the same number.
    assert_eq!(slot_band(0, &frame), SlotBand::Spill(0));
    assert_eq!(slot_band(8, &frame), SlotBand::Spill(1));
    assert_eq!(
        slot_band(0, &frame).local(),
        None,
        "SPILL SLOT ० IS NOT LOCAL SLOT ०. `ir.t1:1320` hands local slots out in \
         first-declaration order, so local ० is a source word; spill ० is whatever \
         the interference graph happened to colour, and matching a name against it \
         would answer a source variable this reader invented"
    );

    // ── TWO: THE LOCAL REGION IS OFFSET BY THE SPILL REGION. Local ० is at
    // ८·num_spills and not at ०.
    assert_eq!(slot_band(16, &frame), SlotBand::Local(0));
    assert_eq!(slot_band(24, &frame), SlotBand::Local(1));
    assert_eq!(slot_band(32, &frame), SlotBand::Local(2));
    assert_eq!(slot_band(32, &frame).local(), Some(2));

    // ── THREE: ABOVE THE SLOTS IS NOT A SLOT — the case the `Next:` line names.
    // A saved `स्थिर` and `पुनःस्थानम्` are frame words no source name reaches,
    // and a reader that folded them into the local region would report local
    // slot ३ and ५ for them.
    assert_eq!(slot_band(40, &frame), SlotBand::Saved);
    assert_eq!(slot_band(56, &frame), SlotBand::ReturnAddress);
    assert_eq!(slot_band(40, &frame).local(), None);
    assert_eq!(slot_band(56, &frame).local(), None);

    // ── FOUR: AT AND ABOVE `bytes` IS THE CALLER'S FRAME. The ९th incoming
    // argument (`riscv64.rs:917`) is addressed off the same register.
    assert_eq!(slot_band(64, &frame), SlotBand::IncomingArgument);
    assert_eq!(slot_band(72, &frame), SlotBand::IncomingArgument);

    // ── FIVE: THE ROUNDING PAD IS `Unaccounted` AND NOT A NEIGHBOUR. ४८ is
    // inside the frame, above the saved `स्थिर` and below `पुनःस्थानम्`;
    // `frame_layout` rounds ५६ up to ६४ and nothing addresses it. A reader that
    // answered `Saved` here would be hiding its own breakage in a band that
    // already has members.
    assert_eq!(slot_band(48, &frame), SlotBand::Unaccounted);
    // And an offset that is not a whole word is no region at all.
    assert_eq!(slot_band(20, &frame), SlotBand::Unaccounted);

    // ── SIX: THE FRAME IS THE ROUTINE'S. The SAME offset reads differently in a
    // routine that spills nothing — which is 853 of the corpus's 870 — and a
    // sweep handed one module-wide frame would answer a wrong band with exactly
    // the confidence of a right one.
    let unspilled = riscv64::Frame {
        bytes: 48,
        saved: vec![(0, 24)],
        ra_offset: 40,
        num_spills: 0,
        num_locals: 3,
    };
    assert_eq!(slot_band(0, &unspilled), SlotBand::Local(0));
    assert_eq!(slot_band(16, &unspilled), SlotBand::Local(2));
    assert_ne!(slot_band(16, &unspilled), slot_band(16, &frame));

    // ── AND SEVEN: A FRAME WITH NO LOCALS HAS NO LOCAL SLOT ०. `count_locals`
    // answers ० for a routine that addresses none, and `Local(०)` must not be
    // reachable — this is the state an `unwrap_or(0)` would have invented.
    let no_locals = riscv64::Frame {
        bytes: 16,
        saved: vec![],
        ra_offset: 8,
        num_spills: 0,
        num_locals: 0,
    };
    assert_eq!(slot_band(0, &no_locals), SlotBand::Unaccounted);
    assert_eq!(slot_band(0, &no_locals).local(), None);
    assert_eq!(slot_band(8, &no_locals), SlotBand::ReturnAddress);
}
