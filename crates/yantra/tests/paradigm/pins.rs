//! `W-237` — THE NUMBERS THE ENCODE CENSUS MEASURED, pinned once and read twice:
//! `tests/paradigm_encode.rs` asserts them (its non-ignored pin runs the chain
//! and compares), and `tests/paradigm_boundary.rs` reports them in W-212's
//! census as `paradigm_boundary_t1_runnable_on_yantra`, so the two censuses
//! cannot disagree about how many `.t1` sources reach the machine.
//!
//! Every value here is MEASURED — taken from the failing assertion on the tree
//! it describes, never written first — and the commit that moves one says so.
//! A source that assembles but halts on a fault is COUNTED at the run stage
//! (`REFUSED` of the row), never hidden: it is in `T1_ASSEMBLED` and not in
//! `T1_ON_YANTRA`, and its halt is named in the census's `RUN` line.

// Two test binaries read this file and neither reads every constant.
#![allow(dead_code)]

/// The self-hosting corpus: every `crates/sadhana-t1/src/*.t1`.
// 18 -> 19 on 2026-09-04, `W-223` part 1: `sanchaya.t1`, the shared declaration
// store, joins the corpus; measured by the pin test.
// 19 -> 20 on 2026-09-07, `W-chain`: `shrinkhala.t1`, the DRIVER — the `.t1`
// port of `chain.rs`'s stage sequence, and the first corpus module that sits
// ABOVE the passes instead of beside them; measured by the pin test.
// 20 -> 21 on 2026-09-14: `sarani.t1`, the embed store generated from `spec/`
// by `tools/mkspectables.py` — the first GENERATED source in the corpus, and
// the first whose count is not a fact about the compiler. Measured from the
// failing assertion. **EVERY OTHER NUMBER IN THIS FILE IS UNCHANGED BY IT**,
// because the census's own walk skips a generated source; read `T1_CENSUSED`,
// not this, wherever "the sources the census measured" is what is meant.
pub const T1_SOURCES: usize = 21;

/// Sources GENERATED from `spec/` rather than authored, by file name.
///
/// `crates/yantra/tests/paradigm_encode.rs`'s `corpus()` drops these from the
/// full walk and names each one on stderr: their figures move when a `spec/`
/// table changes and not when the compiler does. They are still SOURCES — the
/// product compiles them, `T1_CORPUS=sarani` censuses one deliberately, and
/// `sarani.t1` compiles with zero stubs — so they are counted in `T1_SOURCES`
/// and subtracted once, here, rather than being invisible.
///
/// A name here that is not on disk, or is on disk without the generated marker,
/// is caught by `the_t1_corpus_reaches_yantra_by_the_pinned_number`: otherwise
/// this list could inflate the directory count and hide a source that went
/// missing.
///
/// **AND THE ONE ON THIS LIST IS NOT A PASSENGER — MEASURED 2026-09-14 with
/// `T1_CORPUS=sarani`, 573 s.** `sarani.t1` goes the whole way: 23 declarations,
/// 20 routines, 3,597 instructions, 1,522 calls, `typecheck ok`, assembles,
/// links, loads and **halts at the finisher with status ०**, one module object
/// in the image — and the Rust and T1 emitters AGREE octet for octet over
/// **12,562,988 octets** of assembly text, the largest twin agreement in the
/// corpus by an order of magnitude. It is excluded from the census's full walk
/// because its figures track `spec/` rather than the compiler, NOT because it is
/// weaker than what is walked. Censusing it costs 457 s of the 573, which is
/// why the exclusion is worth having at all.
pub const T1_GENERATED: &[&str] = &["sarani.t1"];

/// The corpus the CENSUS walks: authored sources only.
///
/// Every pin below — `T1_ASSEMBLED`, `T1_ON_YANTRA`, `T1_CHAIN_ON_YANTRA`,
/// `T1_UNASSEMBLED` — was measured over this set and is compared against it.
pub const T1_CENSUSED: usize = T1_SOURCES - T1_GENERATED.len();

/// Sources whose emitted T0 text `सङ्केतन` assembles into an object — the
/// emitter's number (research/25 §4, `paradigm_encode_t1_assembled`), taken
/// on the IR the chain built, whatever the type checker said of the source.
// 12 -> 13 on 2026-09-04, `W-223` part 1: `sanchaya.t1` (the declaration store)
// builds, emits and assembles; MEASURED from the failing assertion.
// THIS WAS 13 WHILE THE IR CENSUS READ 15, AND THAT WAS NOT A CONTRADICTION.
// W-223 part 2, 2026-09-04. `sadhana-t1`'s IR census COLLECTS: it parses all
// nineteen sources into one interpreter, fills `घोषणासञ्चय` and raises
// `अर्थॱसञ्चयसिद्धिः`, so the resolver writes a symbol for a cross-module callee
// and the two calls `ir.t1` refused (`samyojana.t1:870`, `vishlesana.t1:615`)
// build — 15 built, 0 run-error there. THIS census does not collect: `chain()`
// builds its own interpreter per source, so the store is in the image and never
// filled, every qualified use is taken on trust exactly as before, and the two
// sources still stop at IR. Both numbers are honest about the build they
// describe — one that has seen the whole corpus, and one that has seen a file.
//
// Converting this census is `W-253`, which is where the real question lives: a
// chain that compiles one source at a time cannot see another module's
// declarations at all, and this corpus makes 1,105 cross-module references, so
// the front half has no counterpart to `W-243`'s image closure. Until that
// lands, this number bounds self-hosting for a reason that is not the emitter's.
//
// 13 -> 15 on 2026-09-04, `W-245`, MEASURED from the failing assertion on the
// merged tree: `samyojana.t1` and `vishlesana.t1` BUILD now, AND THE GAP ABOVE
// CLOSED WITHOUT `W-253`. The refused callee at both sites was W-228 (b)'s
// folded spaced qualifier — `सङ्केतन ॱ क्षेत्रारम्भः …`, one नाम node with the
// member token beside it — which `ir.t1`'s call arm now takes on the resolver's
// trust as it takes the one-token form, WITHOUT a store and without collection.
// So the 13-against-15 was two causes, not one: a per-source build that cannot
// collect (still true, still `W-253`'s) and a call arm that refused a shape the
// resolver had already accepted (this row's, and the one that was moving the
// number). No source's IR is refused any more; the four below have nothing to
// build or nothing to parse.
//
// AND THE SAME NUMBER FROM THE OTHER LANE, whose reading is also true and
// was reached independently:
// 13 -> 15 on 2026-09-04, `W-253`: THE CHAIN CENSUS COLLECTS NOW. It built a
// fresh interpreter per source, so `घोषणासञ्चय` sat in the image and was never
// filled; every qualified use was taken on trust and `ir.t1` refused the two
// calls whose callee therefore carried no symbol. With one interpreter holding
// the store, the resolver writes those symbols and `samyojana.t1` (line 870,
// `सङ्केतनॱदशाङ्कमूल्यम्`) and `vishlesana.t1` (line 615, `सङ्केतनॱअष्टकान्वेषणम्`)
// build, emit and assemble. They are the ONLY two that moved.
//
// The note this replaces said 13 here was not a contradiction of the IR census's
// 15 because that census collected and this one did not. That is no longer true
// of either half, so it is gone rather than left to mislead.
// MEASURED from the failing assertion.
// 15 -> 16 on 2026-09-06 (`W-kosha` phase 2), MEASURED from the failing
// assertion. ONE SOURCE MOVED AND IT IS `kosha.t1`: it declared no routine —
// the symbol model and nothing else — and now carries the ELF image writer, so
// it builds, emits, assembles, links, loads and RUNS (finisher status ०), with
// the Rust and T1 emitters agreeing octet for octet over 38,375 octets.
//
// THIS IS THE NUMBER THIS ROW MOVES, and it is not a stub count. Writing an
// image is not a lowering, so `paradigm_ir_stubs` does not move at all; what
// moves is how much of the corpus the chain can carry to the machine.
// 16 -> 17 on 2026-09-07, `W-chain`: `shrinkhala.t1`, THE DRIVER, PASSES THE
// WHOLE CHAIN — lex, parse, resolve, typecheck, encode, link, load, run. It is
// NOT in `T1_UNASSEMBLED`, whose three entries are unchanged (`ast.t1`/IR,
// `lib.t1`/parse, `vastu.t1`/IR). MEASURED from the failing assertion.
//
// Worth stating plainly because it was the open question of the row: the `.t1`
// compiler driver is itself compiled by the chain it drives. Whether it would
// assemble was not known when it was written — it could as easily have joined
// the three that stop — and the pin is where that was settled rather than
// argued.
// 17 -> 19 on 2026-09-13, `W-vastu`: `ast.t1` and `vastu.t1` yield DATA-ONLY
// objects — globals, no routines, `.text` of zero octets. Not a new column: this
// pin's own definition is *"sources whose emitted T0 text assembles into an
// object"*, and a data-only object is an object.
//
// **WHICH HALF MOVED: THE RUST HALF, ALONE.** The `.t1` emitter always could —
// `यन्त्रमण्डलोत्सर्जनम्` calls `यन्त्रदत्तोत्सर्जनम्` after its routine loop and
// unconditionally — and had never been asked. Two scaffolding refusals were the
// whole gate: `chain.rs:430` and the census's `routines == 0` early return.
pub const T1_ASSEMBLED: usize = 19;

/// Of those, the sources whose standalone image links, loads, and RUNS on
/// `yantra` to a finisher halt (`paradigm_boundary_t1_runnable_on_yantra`).
// 8 -> 11 on 2026-09-04, `W-243`: every routine label is exported and an image
// links the modules its labels reach behind one startup object, so the three
// sources that stopped on another module's routine (unparse, vakyavibhaga,
// yantrotsarjana) link and run; MEASURED from the failing assertion.
// 11 -> 12 on 2026-09-04, `W-223` part 1: `sanchaya.t1` links alone (it calls no
// other module's routine — it reads their arenas) and halts at the finisher,
// status 0; MEASURED from the failing assertion. The checker refuses it
// (`प्रकारलेखनम्`: body kind 12 vs return kind 1 — the verdict it gives
// `lex.t1`, `nidana.t1` and four others) — that was TRUE on W-223's branch, which predates
// `W-231`; on the merged tree the checker accepts 18 of 18 and the chain count is re-measured
// below by the pin test itself (trunk, 2026-09-04).
// 12 -> 15 on 2026-09-04, `W-245`, MEASURED: the two new objects, and the
// census's image closure corrected to resolve a name the ROOT module itself
// exports — W-243's walk seeded its table from the OTHER objects only, so an
// object it pulled in that referred back to the root read as unresolved. No
// source showed it until this row put the corpus's calls inside `चरः`
// initialisers into the IR; then four sources reported their own routines
// missing (`अक्षरकोशमानम्` in `sanskrit_text.t1`, whose object defines and
// exports it — probed before the fix). EVERY source that assembles now runs.
//
// AND THE SAME NUMBER FROM THE OTHER LANE, whose reading is also true and
// was reached independently:
// 12 -> 15 on 2026-09-04, `W-253`: EVERY ASSEMBLED SOURCE NOW RUNS. Two arrived
// (samyojana, vishlesana) because the chain census collects and their callees
// carry symbols; the third is `encode.t1`, whose link stop had stood since
// `W-237` — see T1_ASSEMBLED_NOT_ON_YANTRA below for the one line that closed it.
// MEASURED from the failing assertion.
// 15 -> 16 on 2026-09-06 (`W-kosha` phase 2), MEASURED from the failing
// assertion. `kosha.t1` gained the ELF image writer, assembles, and RUNS —
// finisher status ०. EVERY SOURCE THAT ASSEMBLES STILL RUNS, so this stays
// equal to `T1_ASSEMBLED`; the day one assembles and faults, they part.
// 16 -> 17 on 2026-09-07, `W-chain`: `shrinkhala.t1`, THE DRIVER, PASSES THE
// WHOLE CHAIN — lex, parse, resolve, typecheck, encode, link, load, run. It is
// NOT in `T1_UNASSEMBLED`, whose three entries are unchanged (`ast.t1`/IR,
// `lib.t1`/parse, `vastu.t1`/IR). MEASURED from the failing assertion.
//
// Worth stating plainly because it was the open question of the row: the `.t1`
// compiler driver is itself compiled by the chain it drives. Whether it would
// assemble was not known when it was written — it could as easily have joined
// the three that stop — and the pin is where that was settled rather than
// argued.
// ══ 17 -> 11 ON 2026-09-12, AND THIS ONE IS A RULING, NOT A RE-MEASUREMENT ══
//
// **THE PIN WAS ASPIRATIONAL AND NOTHING SAID SO.** 17 with an EMPTY
// `T1_ASSEMBLED_NOT_ON_YANTRA` says *every source that assembles runs* — true
// when it was written, and false since six sources began faulting at run. The
// measured split is 11 running and 6 stopped, summing to the same 17 assembled,
// taken twice at `b28b1142` and consistent with `paradigm_ir_stubs` 2309 sites.
//
// **WHY IT IS RE-FOUNDED RATHER THAN LEFT RED.** A permanently-red assertion is
// worse than no assertion: it trains every reader to walk past a red line, and
// this one blocked EVERY lane whose gate scope touches `yantra` — a lane with
// two green changes in its own file could not land through it. A pin records a
// MEASUREMENT; a target belongs in a margin, said out loud as a target. So the
// 17 stays here as the goal and the assertion carries the 11.
//
// **WHAT IS NOT DECIDED HERE:** whether the ladder should target 17, or 20, or
// something else, is the owner's open question and this edit does not touch it.
// The six are not accepted — they are a fault with a diagnosis in flight, and
// the day one runs this number rises and one name leaves the list below.
//
// **TAKEN BY THE TRUNK, DELIBERATELY.** The lane that measured the 11 and the 6
// declined to re-found it themselves: it is a file they do not own, the value
// came from their own measurement, and re-pinning to clear their own path is
// the self-ratifying re-pin. That refusal was right.
// ══ 11 -> 13 ON 2026-09-12, `W-293`: A LANDING MOVED IT, NOT A RE-MEASUREMENT ══
//
// `artha.t1` and `ir.t1` left the stopped list because global arrays gained
// storage. A global declared `भवति ०` had a NULL BASE, so an element access
// computed `० + index × width` — `addr 8` at index १, which is what both faulted
// on. The `.t1` emitter now lays a relocated pointer beside reserved `ॱरिक्त`
// space and the pointer is filled by the LINKER, so the base exists with no code
// having run (`tools/global-base-check.py`).
//
// **THE MOVEMENT WAS REGISTERED BEFORE THE RUN**, which is the only thing that
// separates this from a re-pin that ratifies itself: the prediction named these
// two pins as the movers and named 19 other equality pins that had to HOLD.
// `T1_ASSEMBLED` held at 17 with its member list unchanged, so no module left the
// corpus — read FIRST, deliberately, because a stub fall and a module dropping
// out of the measured set are the same reading on a total.
// ══ 13 -> 17 ON 2026-09-13, `W-294`: EVERY SOURCE THAT ASSEMBLES NOW RUNS ══
//
// `BadAccess` appears ZERO times in the census output. The four that remained —
// `encode`, `samyojana`, `shrinkhala`, `vakyavibhaga` — all reach a finisher, so
// `T1_ASSEMBLED_NOT_ON_YANTRA` is EMPTY and this pin equals `T1_ASSEMBLED`.
// Their members were read, not assumed to have emptied because a total moved.
//
// WHAT MOVED THEM: `ir.t1`'s index arm refused any element narrower than a word
// (`विस्तार असमम् ६४`), so every access to an `अ८` run lowered to a constant-zero
// stub and the store landed at address 0. It now reads the width and the emitters
// pick `आहारःॱअ८` / `ॱअ१६` / `ॱअ३२`; ८ still emits the bare `आहारः`, character for
// character, so no existing image moved.
//
// **THE PREDICTION REGISTERED BEFORE THIS RUN WAS WRONG, AND THE ALARM IT NAMED
// WAS CHECKED RATHER THAN WAVED THROUGH.** It said `index_declined` would fall by
// a SMALL amount, and that a large collapse would mean guards had been deleted.
// It fell 538 -> 75. Counted: cause 27 has **17** raise sites now against 16
// before — the width test was replaced by an unknown-width sentinel, net +1, and
// **no guard was deleted**. The model was wrong because the guards are
// SEQUENTIAL: the width test is guard 15 of 16, so everything surviving the first
// fourteen reaches it, and what survives is dominated by the COMMON case — `अ८`
// is 609 of the corpus's slice declarations — not by the rare malformed input the
// defensive guards reject.
//
// AND THE CONSERVATION CHECK IS WHAT MAKES THIS READABLE AS A LOWERING AT ALL,
// because a fall with no matching rise looks identical on a stub count:
//
//     index_declined        538 -> 75     FALL  463
//     lowered_index_load    371 -> 580    rise  209
//     lowered_assign_index  170 -> 424    rise  254   = 463, residue ZERO
//
// **`RUNNABLE` IS NOT `CORRECT` AND THIS PIN HAS NEVER MEANT THAT.** Fourteen of
// the seventeen halt `Some(0)`, which a correct trivial program and a silently
// wrong one both produce; only `artha` 35176, `encode` 856 and `samyojana` 18560
// carry a discriminating answer. The cross-module gather is not in this tree, and
// a source can halt SUCCESS while a cross-module read answers 0.
// ══ 17 -> 16 ON 2026-09-13: `shrinkhala` NOW ASSEMBLES AND STOPS AT LINK ══
//
// It is not a regression and nothing stopped running. `shrinkhala` was never
// measurable here: it built NO IR at all (`insts 0`, `chain-stop IR`), so it was
// outside this pin's population. The empty-literal fix in `ir.t1` gives it 425
// instructions and 94,871 assembled octets, twin AGREE — it now
// assembles, and therefore becomes the first member of
// `T1_ASSEMBLED_NOT_ON_YANTRA`, which is where a source that assembles and does
// not run is counted. **A source moved INTO the measured set and stopped one
// stage short; it did not fall out of one.**
//
// So `T1_ASSEMBLED` stays 17 and this parts from it for the first time since
// `W-294` emptied the stopped list — the day the margin above predicted, in its
// own words: "they stay equal only while the checker …".
// 16 -> 18 on 2026-09-13, the वास्तु lowering (read side, walk of 6e3e03cc, fresh per
// source, 4 threads, 3061 s): every object is present now, so `shrinkhala.t1` LINKS and
// runs, and the two data-only objects run at the stub. `sanskrit_text.t1` moved the other
// way — see `T1_ASSEMBLED_NOT_ON_YANTRA`. MEASURED from the failing assertion.
// 18 -> 19 on 2026-09-28, `W-306`, MEASURED from the failing assertion on the full
// 19-source walk: `sanskrit_text.t1` runs, and the stopped list below empties with it.
// THE COMMIT THAT DID IT IS `14799fad` (2026-09-14), "the compiler reads its own spec
// tables at run time, not through the include" — it replaced four LEX-TIME
// `समावेशः आरभ्य <table> समाप्तम्` substitutions in `sanskrit_text.t1` with run-time
// lookups in `समावेशपाठकोश`, so the module object no longer walks a table literal
// baked into itself at load. Its own message says "NOT COVERED: the full census and
// the pins (report-only)", so the re-take was owed from that day and this is it.
// NOT RE-TAKEN ON THE STRENGTH OF ONE GREEN RUN — that is the failure this file
// records. `W-306` gave the census `Row::step_margin` first, and the margin is what
// justifies the move: `sanskrit_text.t1` halts at the finisher in **80 steps of
// 1,000,000 (0%)**, four orders of magnitude clear of the limit it used to hit. A
// source that had merely squeaked under would read `tight` and this pin would have
// stayed where it was.
pub const T1_ON_YANTRA: usize = 19;

/// Of those, the sources the whole chain accepts — lex, parse, resolve AND
/// typecheck — before the emitter's stages: the owner's definition of done
/// counts these (`paradigm_encode_t1_chain_on_yantra`).
// 5 -> 6 on 2026-09-04, `W-243`: `unparse.t1` (the checker accepts it) reached
// the machine through `lex.t1`'s export; the other two new runners the checker
// refuses. MEASURED from the failing assertion.
// 6 -> 11 on 2026-09-04 at the trunk's merge of W-231: the checker accepts 18 of 18 sources now
// (the join applies only where a value is read; a Boolean variable as a test is accepted), so the
// three runners it had refused (artha, lex, nidana) and the two W-243 added (vakyavibhaga,
// yantrotsarjana) pass the whole chain — MEASURED (the pin test printed it).
// 11 -> 12 on 2026-09-04, `W-223` part 2's merge of part 1 with main: `sanchaya.t1`
// (the declaration store) joins the corpus as a nineteenth source and passes the WHOLE
// chain. Part 1 could not have pinned this one: on its own branch, cut before W-231, the
// checker refused sources this number counts, so 11 was measured against a checker that
// no longer exists. Its two siblings (T1_ASSEMBLED 12 -> 13, T1_ON_YANTRA 11 -> 12) part
// 1 did move, and they were right on both trees. MEASURED here from the failing
// assertion on the merged tree, never summed.
// The trunk measured the same 12 independently at part 1's merge and kept this note, which says
// more: the two readings agreeing is itself the check.
// 12 -> 15 on 2026-09-04, `W-245`, MEASURED from the failing assertion: the
// checker has accepted every source since W-231, so every source that reaches
// the machine reaches it through the WHOLE chain — lex, parse, resolve,
// typecheck, IR, emit, assemble, link, load, run. This number and
// `T1_ON_YANTRA` are now equal, and they stay equal only while the checker
// refuses nothing; the day it refuses a source that assembles, they part and
// this one falls.
//
// AND THE SAME NUMBER FROM THE OTHER LANE, whose reading is also true and
// was reached independently:
// 12 -> 15 on 2026-09-04, `W-253`: the chain census COLLECTS, so a qualified
// callee carries the symbol `W-223`'s store supplies; the two sources the IR
// builder refused for a callee with no symbol now build, emit and assemble; and
// the image closure counts `own`'s exports, which is what let a cyclic pair link.
// Every source that assembles passes the WHOLE chain — lex, parse, resolve,
// typecheck, encode, link, load, run — so this equals T1_ASSEMBLED and
// T1_ON_YANTRA at 15 of 19. The four that do not: three declare no routine and
// `lib.t1` parses to no declarations. MEASURED from the failing assertion.
// 15 -> 16 on 2026-09-06 (`W-kosha` phase 2), MEASURED from the failing
// assertion. `kosha.t1` passes the WHOLE chain — the census prints `typecheck
// ok` for it — so all three of these move together, as their margins say they
// must while the checker refuses nothing.
//
// AND THE SENTENCE ABOVE IS RE-FOUNDED: "The four that do not: three declare no
// routine and `lib.t1` parses to no declarations" is now THREE, and the reason
// matters more than the count. It was true of `ast.t1` and `vastu.t1`, which are
// record declarations with nothing to build BY DESIGN, and it covered
// `kosha.t1`, which had nothing to build only because THE ELF WRITER HAD NOT
// BEEN WRITTEN — the first of the two things standing between Sassembly and
// running without Rust. A true sentence doing too much work, which is why it
// went unexamined for two days.
//
// `vastu.t1` IS NOW THE ONE TO WATCH for the same reason: it declares no
// routine and its Rust twin `vastu::read` parses an object. Whether it needs
// porting is OPEN and probably NO — the owner ruled 2026-09-06 that the object
// round trip is skipped entirely, so the linker takes `वास्तुॱवस्तु` records
// built directly and no `.o` is ever serialised to be parsed back.
// 16 -> 17 on 2026-09-07, `W-chain`: `shrinkhala.t1`, THE DRIVER, PASSES THE
// WHOLE CHAIN — lex, parse, resolve, typecheck, encode, link, load, run. It is
// NOT in `T1_UNASSEMBLED`, whose three entries are unchanged (`ast.t1`/IR,
// `lib.t1`/parse, `vastu.t1`/IR). MEASURED from the failing assertion.
//
// Worth stating plainly because it was the open question of the row: the `.t1`
// compiler driver is itself compiled by the chain it drives. Whether it would
// assemble was not known when it was written — it could as easily have joined
// the three that stop — and the pin is where that was settled rather than
// argued.
// ══ 17 -> 11 ON 2026-09-12, THE THIRD RED BEHIND THE FIRST, SAME RULING ══
//
// This counts sources the WHOLE chain carries to the machine, and every such
// source must also RUN — so it can never exceed `T1_ON_YANTRA` above. When that
// became 11 this stayed 17 and went red on the next line of the same test, a
// 1,988-second run to learn a number that was implied by the previous one.
//
// **THAT IS THE THIRD TIME TODAY A RED PROVED TO BE A LOWER BOUND**: four pins
// in `sanskrit-text`, then three more in `sadhana-t1` and this test, then this
// line behind the assertion two above it. A failure names what tripped FIRST.
// Where one figure bounds another, moving the first and re-running to discover
// the second costs a full census each time — so move the DEPENDENT pins in the
// same edit and let the run confirm both.
//
// The 11 is the same measured set as above and the six stopped sources are the
// same six; nothing about the chain's acceptance changed. The checker still
// accepts what it accepted — these sources fault on the MACHINE, after the
// whole front half succeeded.
// 11 -> 13 on 2026-09-12, `W-293`, and it moves in the SAME EDIT as
// `T1_ON_YANTRA` because one BOUNDS the other — every source the chain carries to
// the machine must also run. Splitting them cost 1,988 seconds the last time:
// the census was paid twice to learn a number the first edit had already
// determined.
// 13 -> 17 on 2026-09-13, `W-294`, and it MUST move with `T1_ON_YANTRA` — the
// checker already accepted all four; what stopped them was the run, not the
// chain. A tree where these two disagree has a source the checker rejects but
// the machine runs, which is a different defect from anything this row covers.
// 17 -> 16 on 2026-09-13, in the SAME EDIT as `T1_ON_YANTRA` and for its reason:
// `shrinkhala` assembles and stops at link, so it is no longer carried to the
// machine. The checker's verdict on it did not change — it typechecks in both
// trees. What changed is that it now HAS an IR for the later stages to refuse.
// 16 -> 18 on 2026-09-13 with `T1_ON_YANTRA` above.
// 18 -> 19 on 2026-09-28, `W-306`, MEASURED (`paradigm_encode_t1_chain_on_yantra 19`):
// it moves WITH `T1_ON_YANTRA` above and for that pin's reason — the checker accepts
// `sanskrit_text.t1`, so the source it gained is one the WHOLE chain carries.
pub const T1_CHAIN_ON_YANTRA: usize = 19;

/// Every source that does NOT assemble, with the stage the emitter's side of
/// the chain stopped at — `parse`, `resolve`, `IR`, `emit`, or `assemble` —
/// so a regression in `T1_ASSEMBLED` fails by NAME and a new stop is named
/// with its stage. Sorted by file name.
pub const T1_UNASSEMBLED: &[(&str, &str)] = &[
    // MEASURED 2026-09-04 (W-237), from the failing assertion on the sentinel ०:
    // three files declare no routine (nothing to build); `lib.t1` parses to no
    // declarations. RE-MEASURED 2026-09-04 (`W-245`): the two IR refusals are
    // gone — see `T1_ASSEMBLED` — and what is left is what has nothing to build.
    // **`ast.t1` AND `vastu.t1` LEFT ON 2026-09-13, `W-vastu`: THEY YIELD
    // DATA-ONLY OBJECTS.** Both declare module `वास्तु` and not one routine, and
    // both were here because two Rust refusals said a routine-less source has
    // nothing to build. `ast.t1` carries 45 globals and `vastu.t1` six — the
    // variants of `सार्वजनिक गणना स्थापन`, which are module-scope globals in a
    // different declaration form and are why my own prediction that `vastu.t1`
    // would yield nothing was wrong.
    //
    // MEASURED FROM THE ASSEMBLED OBJECTS, never the assembly text:
    //   ast.t1    .text 0  .data 368  .bss 2048  symbols 50, 45 exported
    //   vastu.t1  .text 0  .data  48  .bss    0  symbols  9,  6 exported
    ("lib.t1", "parse"),
];
// `kosha.t1` LEFT this list on 2026-09-06 (`W-kosha` phase 2) — it gained the
// ELF image writer, so there is something in it to build.
//
// AND THE SENTENCE ABOVE NEEDED RE-FOUNDING, NOT JUST A ROW REMOVED. "Three
// files declare no routine (nothing to build)" was TRUE and it was doing too
// much work: it was true of `ast.t1` and `vastu.t1`, which are record
// declarations and have nothing to build by design, and it covered `kosha.t1`,
// which had nothing to build only because THE ELF WRITER WAS NOT WRITTEN — the
// one thing standing between the chain and an image. A true sentence covering a
// real gap, which is exactly why nobody re-examined it.
//
// `vastu.t1` is now the one to watch for the same reason: it declares no routine
// AND its Rust twin `vastu::read` parses an object, so "nothing to build" is
// again true-and-covering. Whether it needs porting at all is open — the linker
// consumes `वास्तुॱवस्तु` RECORDS, so an in-memory chain may never serialise an
// object to parse back.
// `samyojana.t1` and `vishlesana.t1` LEFT this list on 2026-09-04 (`W-253`): both
// stopped at "a call whose callee carries no symbol", which is what the shared
// declaration store answers. The four that remain are structural and not
// refusals — three declare no routine, and `lib.t1` parses to no declarations.

/// Every source that assembles and does NOT run to a finisher halt, with the
/// stage that stopped it — `link` (an unresolved label), `load`, or `run:<halt>`.
/// Sorted by file name.
// EMPTY since 2026-09-04 (`W-253`): no source assembles and then fails to reach
// the machine. `encode.t1`'s "link" entry closed with a one-line fix to the image
// closure, and the fix is the row's finding rather than its incident.
//
// `link_and_run` built its export map from the OTHER objects and never from
// `own`. Choosing which others to pull in, that is right; but the walk also
// extends its worklist with a pulled-in object's undefined names, and those were
// looked up in a map that could not see `own` — so the moment the closure came
// back around to the object it started from, it called that object's own export
// unresolved. IT NEEDED A CYCLE: `encode.t1` and `vishlesana.t1` reference each
// other, and until this row `vishlesana.t1` produced no object at all (the IR
// builder refused it at line 615 for a callee with no symbol), so no closure had
// ever walked back. The failure was exactly symmetric when it did — encode's
// image reported the three `सङ्केतन*` labels ENCODE exports, vishlesana's reported
// the one `विश्लेषण*` label VISHLESANA exports, each object's own name called
// missing from its own image. Latent in `W-243`'s closure since it was written;
// reachable only once both halves of a cyclic pair had objects.
// ══ EMPTY -> SIX ON 2026-09-12 ══ The counterpart of the ruling above. These
// six assemble and then fault on the machine, all with the same halt, and they
// are NAMED rather than counted so that one recovering is visible as a name
// leaving this list rather than as a number moving.
//
// **ALL SIX SHARE ONE HALT: `BadAccess`, and `artha.t1` is reproduced to the
// digit** — `pc 2147483800, addr 8`, matched by a one-module fixture built to
// that entry condition.
//
// **THE MECHANISM — AND THIS PARAGRAPH HAS BEEN WRONG ONCE ALREADY, SO THE GRID
// IS HERE INSTEAD OF A SENTENCE.** Measured cell by cell, element type against
// scope, `भवति ०` throughout:
//
//                    LOCAL                     GLOBAL
//     न६४            works (`Some(9)`)         BadAccess addr 8
//     अ८             BadAccess addr 0          BadAccess addr 0
//     record         works                     BadAccess addr 8
//
// **THERE ARE TWO DEFECTS, NOT ONE, AND THEY SHARE A SYMPTOM.**
//
//   1. A **GLOBAL** array declared `भवति ०` has no storage **whatever its
//      element type**, and `addr = index × 8` for word-sized elements — which
//      is `artha`'s and `ir`'s 8 at index १.
//   2. SEPARATELY, a **LOCAL slice of `अ८`** declared `भवति ०` also has no
//      storage and faults at `addr 0` **regardless of index** — the index never
//      enters the address, which is why element ० and element १ both give 0.
//      This is `encode`'s `प्रकारविभाजकः` and `samyojana`'s `संस्कारोपपदम्`.
//   3. Locally, `न६४` and record element types DO allocate and round-trip.
//
// So the four that looked like one mechanism are **two mechanisms, two sources
// each** — the two `addr 8` cases are globals, the two `addr 0` cases are local
// `अ८`.
//
// **HOW THIS MARGIN WENT WRONG, BECAUSE THE ROUTE MATTERS MORE THAN THE FIX.**
// It first said GLOBAL ARRAYS, which was RIGHT. A control — a local slice of
// `अ८` faulting — was read as refuting it, and the claim was broadened to "any
// slice declared `भवति ०`". **That control shared the SYMPTOM and not the
// CAUSE.** A `BadAccess` at a small address is what BOTH defects produce, so an
// instance of the second was used as evidence against the first.
//
// > **A CONTROL MUST SHARE THE CAUSE, NOT MERELY THE SYMPTOM.** Where two
// > defects produce one symptom, a control drawn from the wrong one does not
// > narrow a claim — it BROADENS a correct claim into a false one, and it reads
// > exactly like rigour while doing it.
//
// **AND THE COUNTEREXAMPLE WAS TWELVE LINES AWAY, PASSING.**
// `an_array_local_takes_an_element_write_and_reads_it_back` is a LOCAL slice of
// `न६४` declared `भवति ०` storing element १, and it has answered `Some(9)`
// throughout. The generalisation was contradicted by a green test in the same
// file before it was ever written. **Before widening a claim, look for the
// passing test that the wider claim forbids.**
//
// `ir.t1:2449`'s array arm allocates from `खण्डसामर्थ्यम्`, a fixed run's
// CAPACITY. The one array fixture that passes as a struct FIELD gets its
// storage from the record's `AllocRecord`.
//
// **FOUR OF THE SIX FIT, ONE DOES NOT, AND ONE CANNOT BE BUILT — recorded that
// way rather than rounded into a uniform set.** `vakyavibhaga.t1`'s `कारकनाम`
// is seven lines of `प्रत्यागमनम् उक्तम् … इति`, string-literal returns with no
// slice declaration and no element store; its `addr 0` is consistent with a
// null base but the shape is different. `shrinkhala.t1` stops before an image
// exists — a front-end refusal, `octets 1..1 and पाठाक्षरकोश holds 0` — so its
// `run:bad-access` here and that stop are two unreconciled facts about it.
//
// So this list is a diagnosis in flight, not an accepted state.
// `artha.t1` and `ir.t1` LEFT on 2026-09-12 (`W-293`) — global arrays gained
// storage. The four that remain are TWO different defects and naming them apart
// is the point of keeping this as MEMBERS rather than a count:
//
//   encode · samyojana   local `अ८` slices — DEFECT 2, the index is never applied
//                        at either scope (0/0/0/0 across indexes at pc …728,
//                        against 0/8/16/24 for `न६४` at pc …752: two arms)
//   shrinkhala           `पाठाक्षरकोश` at `ir.t1:464` is a GLOBAL `अ८` array, so
//                        defect 1 gave it a base and defect 2 still discards the
//                        index — every octet lands at `base+0`. PREDICTED to stay
//                        before the run that confirmed it.
//   vakyavibhaga         returns string literals, fits neither — still unexplained
//
// **EMPTIED 2026-09-13 BY `W-294`, AND EVERY LINE ABOVE IS NOW HISTORY — kept
// dated rather than deleted, because three of the four entries carried a
// DIAGNOSIS and only one of them was right.**
//
//   encode · samyojana   "local `अ८` slices — DEFECT 2, the index is never
//                        applied at either scope". CORRECT. The width refusal
//                        planted a constant-zero stub; both now reach a finisher.
//   shrinkhala           "`पाठाक्षरकोश` is a GLOBAL `अ८` array … every octet lands
//                        at `base+0`. PREDICTED to stay." The prediction held for
//                        the tree it was made on and the DIAGNOSIS was incidental:
//                        what freed it was the same width fix, not anything about
//                        that arena. I spent an hour on that arena tonight and it
//                        was never the discriminator.
//   vakyavibhaga         "returns string literals, fits neither — still
//                        unexplained". It fitted defect 2 exactly. A string
//                        literal IS an `अ८` run, so "returns string literals" was
//                        a restatement of the cause, not an alternative to it.
//                        **An unexplained row is a row nobody has connected yet,
//                        not a row that needs its own mechanism.**
//
// The list is EMPTY because `BadAccess` appears zero times in the census, read
// per-source rather than inferred from `T1_ON_YANTRA == T1_ASSEMBLED`.
//
// ══ REFILLED 2026-09-13 WITH ONE MEMBER, AND IT ARRIVED BY ASSEMBLING ══
//
// `shrinkhala.t1` built NO IR before the empty-literal fix — `insts 0`,
// `chain-stop IR`, "instruction 309 of block 74 names octets 1..1 and
// `पाठाक्षरकोश` holds 0". It was absent from every pin on this page, so its
// arrival RAISES what is measured. Measured on `T1_CORPUS=shrinkhala`:
//
//     BEFORE `41d18dd5`   insts   0   chain-stop IR
//     AFTER  this tree    insts 425   chain-stop link   twin AGREE 94,871 octets
//
// **THE 16 UNRESOLVED ARE THE LINKER UNIT'S POPULATION, NAMED IN FULL** — every
// one a call into ANOTHER module's object, none a symbol `shrinkhala` should
// define itself:
//
//   अर्थकार्यक्रमनिर्णयः · अर्थकार्यक्रमप्रकारपरीक्षा · अर्थनामनिर्णयः
//   अर्थप्रकारपरीक्षकारम्भः · कोशप्रतिबिम्बलेखनम् · पदविभागचिह्नकपाठः
//   पदविभागपदविभाग · मध्यरूपकार्यक्रमरचना · यन्त्रोत्सर्जनयन्त्रनामयोजनम्
//   यन्त्रोत्सर्जनयन्त्रारम्भमण्डलोत्सर्जनम् · वाक्यविभागसङ्कलनम् · व्याकरकार्यक्रमपठनम्
//   संयोजनसंयोजनम् · सङ्केतनवस्तुरचना · सङ्केतनवस्तुसङ्केतनम् · सङ्केतनस्थानविन्यासः
//
// So this row does not say `shrinkhala` is broken; it says the OBJECTS of the
// nine modules it calls are not present at link. That is the linker unit, and
// this is the first source to state its cost as a number.
//
// **THE COUNT IS A PROPERTY OF THE SET IT WAS MEASURED ON.** On
// `T1_CORPUS=shrinkhala,ir` the same tree reads `insts 434`, 98,221 octets and
// **24** unresolved — `ir` in the closure lets nine more calls lower and adds its
// own unresolved names. Neither figure is the corpus's; quote each with its set.
// **RE-POPULATED 2026-09-13, `W-vastu`, AND THE SECOND FIELD IS NOW LOAD-BEARING.**
// `W-294` emptied this list; every member it has ever held meant *assembled but
// FAULTS at run*. These two mean *assembled but has NO ENTRY POINT* — a data-only
// object has nothing to run. **Different facts, one list, and the list's name
// distinguishes neither**, so the reason travels in the tuple and the assertion
// that consumes this must WALK it, never count it.
//
// AND THE IDENTITY `assembled == runnable` IS OVER. It held for exactly one
// landing and was never a law — only a coincidence of every assembled source also
// having an entry point. `T1_ASSEMBLED` is 19 here; `T1_ON_YANTRA` is 17 pending
// 6e's correction to 16 (shrinkhala, measured on the tree that fixes it, which is
// why that number is not re-taken here). **The inequality holds either way**:
// 19 ≠ 17 and 19 ≠ 16, so this claim does not wait on that landing.
//
// **AND THE ASSERTION THIS LIST SITS IN GOES GREEN BY THE UNION OF TWO LANDINGS,
// NOT BY EITHER ALONE.** `paradigm_encode.rs` asserts a TUPLE — the runnable
// COUNT beside this LIST — so on a full walk:
//
//   this commit alone   list right, count still 17 against a measured 16 -> RED
//   6e's alone          count right, list missing these two              -> RED
//   both                                                                 -> green
//
// A reader meeting that red must know which element is whose. The count and
// `("shrinkhala.t1", "link")` are 6e's, measured on the tree that fixes
// shrinkhala; the two `"no entry point"` rows are this one. **Neither lane can
// demonstrate the green, and neither is wrong** — which is the dependent-pin
// shape, two facts under one assertion moved by two authors.
//
// After both land this list has THREE members and TWO KINDS:
//
//   ("shrinkhala.t1", "link")        assembled, does not LINK
//   ("ast.t1",   "no entry point")   assembled, nothing to RUN
//   ("vastu.t1", "no entry point")   assembled, nothing to RUN
//
// **Read it by member, never by its length.** A count of three says nothing
// about whether a fault came back, and the second field is the only thing that
// does.
// ══ THE UNION, 2026-09-13: THREE MEMBERS OF TWO KINDS, FROM TWO LANDINGS ══
// `shrinkhala.t1` assembles and does not LINK (6e, agent/emptylit); `ast.t1` and
// `vastu.t1` assemble and have NO ENTRY POINT (2d, agent/vastu-object). Neither
// landing could make the tuple at `paradigm_encode.rs` green alone; the entry
// carries its reason, and a reader walks the list — never counts it.
// ══ 2026-09-13, THE WALK OF 6e3e03cc: ONE MEMBER, AND IT IS A RUN-TIME REGRESSION ══
// The lowering of cross-module reads retired cause 44 (667 -> 0) and put the two
// data-only objects and `shrinkhala.t1` on yantra. `sanskrit_text.t1` ran to a
// finisher before it and now spins to the step limit: a read that was a stub's
// ० is a real load of another module's global, whose initialiser has not run in
// this image. Named here, fixed on top; the census's stage says `run:step-limit`.
// EMPTIED 2026-09-28, `W-306`: `sanskrit_text.t1` was its one entry and it runs now
// (see `T1_ON_YANTRA`). An empty list here is the state this file warned about — it is
// the only place a source that STOPPED running would show, so emptying it by hand
// removes the instrument rather than the fault. IT IS NOT EMPTIED BY HAND AND IT IS NOT
// LEFT WITHOUT AN INSTRUMENT: the same cycle added the census's per-source
// `METRIC t1_run_steps <name> <spare|tight|exhausted> <n> of <BUDGET>` line, which
// names every source's step count whether or not it ran, so a source that falls back to
// the step limit now reads `exhausted` with its count beside it instead of showing up
// only as a new tuple in this list. The tuple equality below still reds on the move;
// what changed is that the reader is told HOW CLOSE the survivors are.
pub const T1_ASSEMBLED_NOT_ON_YANTRA: &[(&str, &str)] = &[];

/// `tests/corpus/t1/*.सस` — counted, 0 readable BY DESIGN (research/25 §1.4a,
/// `W-238`: a retired dialect, never a readable count to raise).
pub const SAS_PROGRAMS: usize = 16;
