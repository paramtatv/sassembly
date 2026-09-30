//! **WHERE A RUN'S STEPS WENT — the distribution behind `METRIC t1_run_steps`.**
//!
//! # The reading this exists to make possible
//!
//! `W-306` gave the census a step COUNT: `shrinkhala.t1` reaches the finisher on
//! 487,371 of [`crate::DEFAULT_STEPS`]'s million while the other eighteen sources
//! finish under 2%. A total cannot say WHY, and the two answers it hides call for
//! opposite work: 487,371 steps spread evenly over a large program is work the
//! source genuinely does, and the fix is a bigger budget; the same 487,371 steps
//! piled onto a dozen instructions is ONE HOT LOOP, and raising the budget hides
//! it. `DEFAULT_STEPS`'s own margin forbids sizing the constant off the total, so
//! the distribution is the measurement it asks for.
//!
//! # Not the other profile in this tree
//!
//! `tests/t1_profile.rs` bisects the `.t1` INTERPRETER's fuel budget to attribute
//! steps to a compiler STAGE (lex, parse, …). That measures the interpreted front
//! end. This measures the MACHINE — the compiled program's own program counter,
//! one tick per instruction the hart begins, the same clock [`Machine::time`]
//! keeps. Neither substitutes for the other.
//!
//! # Outside the machine on purpose
//!
//! [`Machine::pc`] is public and [`Machine::step`] is one instruction, so the
//! profile is a free function over the public surface rather than a field on
//! [`Machine`]. That is not tidiness: a field would be a twenty-odd-site struct
//! literal change across the test suite, and — more to the point — it would put a
//! branch in the hot path of a run that reaches twenty-three billion steps.
//! A caller that does not ask to be profiled pays nothing.

use crate::{Halt, Machine, Output};
use std::collections::BTreeMap;

/// A contiguous window of program counters, and what share of the run it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Lowest program counter in the window.
    pub lo: u64,
    /// Highest program counter in the window — INCLUSIVE, so a one-instruction
    /// window has `lo == hi` and [`Span::width`] four.
    pub hi: u64,
    /// Steps spent inside it.
    pub steps: u64,
    /// Distinct program counters inside it that the run actually reached. NOT
    /// `width / 4`: the window is an address range and a range can straddle
    /// instructions the run never executed.
    pub sites: usize,
}

impl Span {
    /// Octets from the first instruction in the window to the last, inclusive.
    #[must_use]
    pub fn width(&self) -> u64 {
        self.hi - self.lo + 4
    }
}

/// Every program counter a run began an instruction at, and how many times.
#[derive(Debug, Clone, Default)]
pub struct StepProfile {
    counts: BTreeMap<u64, u64>,
    steps: u64,
}

impl StepProfile {
    /// Steps recorded — equal to [`Machine::time`] after the run, because the
    /// same event increments both.
    #[must_use]
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// How many steps began at `pc`. Zero for an address the run never reached,
    /// and an address never reached is ABSENT rather than present-with-zero: a
    /// profile that listed every word of the text could not be asked "how much
    /// of this program ran at all".
    #[must_use]
    pub fn at(&self, pc: u64) -> u64 {
        self.counts.get(&pc).copied().unwrap_or(0)
    }

    /// Distinct program counters the run reached.
    #[must_use]
    pub fn sites(&self) -> usize {
        self.counts.len()
    }

    /// Whether the run began an instruction at `pc` AT ALL — the question
    /// [`StepProfile::at`]'s zero cannot answer, since a count of zero and an
    /// address never fetched are the same number.
    #[must_use]
    pub fn reached(&self, pc: u64) -> bool {
        self.counts.contains_key(&pc)
    }

    /// The per-site counts added up. Equal to [`StepProfile::steps`] whenever the
    /// profile is intact, which is why it is a separate reading: the two are kept
    /// by different code, so a test can ask them to agree.
    #[must_use]
    pub fn counts_sum(&self) -> u64 {
        self.counts.values().sum()
    }

    /// The `n` busiest program counters, most steps first; ties by address so the
    /// reading is stable across runs.
    #[must_use]
    pub fn hottest(&self, n: usize) -> Vec<(u64, u64)> {
        let mut by_steps: Vec<(u64, u64)> = self.counts.iter().map(|(&pc, &c)| (pc, c)).collect();
        by_steps.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        by_steps.truncate(n);
        by_steps
    }

    /// **THE LOOP QUESTION.** The NARROWEST contiguous address window holding at
    /// least `numer/denom` of the steps — `hot_span(1, 2)` for half of them.
    ///
    /// This is the reading that separates the two causes a total hides. A tight
    /// loop puts half a run's steps inside a handful of instructions, so the
    /// window comes back a few dozen octets wide; a program that is merely long
    /// needs half its own text to reach half its steps. The window is the
    /// narrowest one rather than the busiest neighbourhood because a wide window
    /// can always be found — the whole text holds all the steps — and only the
    /// narrowest one is a claim.
    ///
    /// `None` when the run took no steps. A `denom` of zero answers `None` rather
    /// than dividing.
    #[must_use]
    pub fn hot_span(&self, numer: u64, denom: u64) -> Option<Span> {
        if self.steps == 0 || denom == 0 {
            return None;
        }
        // Ceiling, so `hot_span(1, 2)` of an odd total really is at least half and
        // not one step short of it.
        let target = (self.steps * numer).div_ceil(denom);
        let sites: Vec<(u64, u64)> = self.counts.iter().map(|(&pc, &c)| (pc, c)).collect();
        // Two pointers over the sites IN ADDRESS ORDER. Every count is positive,
        // so a window's sum is monotone in its right edge: shrink from the left
        // while the sum still clears the target, and the minimal window ending at
        // each right edge is found exactly once.
        let mut best: Option<Span> = None;
        let mut sum = 0u64;
        let mut lo = 0usize;
        for hi in 0..sites.len() {
            sum += sites[hi].1;
            while sum - sites[lo].1 >= target {
                sum -= sites[lo].1;
                lo += 1;
            }
            if sum >= target {
                let span = Span {
                    lo: sites[lo].0,
                    hi: sites[hi].0,
                    steps: sum,
                    sites: hi - lo + 1,
                };
                // Narrower wins; on a tie the earlier window, so the answer does
                // not depend on which of two equal loops was written first.
                if best.is_none_or(|b| span.width() < b.width()) {
                    best = Some(span);
                }
            }
        }
        best
    }
}

/// Run `m` for at most `budget` steps, recording where each step began.
///
/// The same contract as [`Machine::run`] — including [`Halt::StepLimit`] when the
/// budget runs out — because a profile taken from a DIFFERENT run than the one
/// being explained is worse than no profile. The one thing it does not share is
/// the cost: a `BTreeMap` write per instruction is not something to put under a
/// twenty-three-billion-step self-compile.
///
/// The program counter is read BEFORE the step, which is also why an interrupt
/// taken instead of an instruction is attributed to the instruction it
/// interrupted: [`Machine::step`] ticks the clock on that step too, and the two
/// counts have to agree.
pub fn run_profiled(m: &mut Machine, budget: u64, out: &mut impl Output) -> (Halt, StepProfile) {
    let mut profile = StepProfile::default();
    for _ in 0..budget {
        let pc = m.pc;
        *profile.counts.entry(pc).or_insert(0) += 1;
        profile.steps += 1;
        if let Some(h) = m.step(out) {
            return (h, profile);
        }
    }
    (Halt::StepLimit { pc: m.pc }, profile)
}

/// A named stretch of text: one routine, and the addresses it owns.
///
/// Built from the image's symbol table, which carries a name and an ADDRESS and
/// no size — so a routine's extent is *up to the next name*. That is an
/// inference, not a record, and it is the honest one available: the linker's
/// [`Symbol`](../../sadhana/kosha/struct.Symbol.html) has no length field, and
/// padding between two routines belongs to the earlier of them either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routine {
    /// The label, as the symbol table spells it.
    pub name: String,
    /// First address the routine owns.
    pub lo: u64,
    /// Last address it owns — INCLUSIVE, as [`Span::hi`] is.
    pub hi: u64,
}

impl Routine {
    /// Octets from its first address to its last, inclusive.
    #[must_use]
    pub fn width(&self) -> u64 {
        self.hi - self.lo + 1
    }

    /// Whether `pc` falls inside it.
    #[must_use]
    pub fn contains(&self, pc: u64) -> bool {
        self.lo <= pc && pc <= self.hi
    }
}

/// Lay a text symbol table out as routines, address order, extents inferred.
///
/// `symbols` is `(name, address)` for the TEXT symbols only — a data or `.bss`
/// name given here would claim a stretch of code it does not own. Anything
/// outside `[text_lo, text_hi]` is dropped rather than clamped: a symbol that
/// is not in this text cannot be given a piece of it.
///
/// Two names at one address are ALIASES, not two routines, and collapse to one
/// entry under the alphabetically first name — the alternative is two entries
/// with identical ranges, which would count the same steps twice in
/// [`StepProfile::owners`].
#[must_use]
pub fn routines(symbols: &[(String, u64)], text_lo: u64, text_hi: u64) -> Vec<Routine> {
    let mut by_addr: BTreeMap<u64, &str> = BTreeMap::new();
    for (name, addr) in symbols {
        if *addr < text_lo || *addr > text_hi || name.is_empty() {
            continue;
        }
        by_addr
            .entry(*addr)
            .and_modify(|held| {
                if name.as_str() < *held {
                    *held = name.as_str();
                }
            })
            .or_insert(name.as_str());
    }
    let starts: Vec<(u64, &str)> = by_addr.into_iter().collect();
    starts
        .iter()
        .enumerate()
        .map(|(i, &(lo, name))| Routine {
            name: name.to_string(),
            lo,
            // Up to the next name, or to the end of the text for the last one.
            hi: starts.get(i + 1).map_or(text_hi, |&(next, _)| next - 1),
        })
        .collect()
}

/// One routine's share of a [`Span`], or a stretch of the span no name claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    /// The routine's name, or `None` for text the symbol table does not cover.
    ///
    /// An `Option` and not a placeholder string: "this span runs off the end of
    /// every routine I know" is a different answer from "this span is in
    /// `अज्ञातम्`", and an instrument that spelled them the same could not be
    /// caught getting the symbol table wrong.
    pub name: Option<String>,
    /// First address of the OVERLAP — the span's, not the routine's.
    pub lo: u64,
    /// Last address of the overlap, inclusive.
    pub hi: u64,
    /// Steps the run spent in that overlap.
    pub steps: u64,
    /// Distinct program counters inside it the run reached.
    pub sites: usize,
}

impl StepProfile {
    /// Steps and reached sites in the inclusive address range `lo..=hi`.
    ///
    /// Zero and zero for a range the run never entered, which is the same answer
    /// as for a range that is not text at all — [`StepProfile::reached`] is the
    /// reading that separates those, and this one is a sum.
    #[must_use]
    pub fn steps_in(&self, lo: u64, hi: u64) -> (u64, usize) {
        if lo > hi {
            return (0, 0);
        }
        let mut steps = 0;
        let mut sites = 0;
        for (_, &c) in self.counts.range(lo..=hi) {
            steps += c;
            sites += 1;
        }
        (steps, sites)
    }

    /// **WHAT OWNS THE WINDOW.** Every routine `span` touches, in address order,
    /// each with the part of the span it owns and the steps spent there.
    ///
    /// [`StepProfile::hot_span`] answers *where* a run's steps piled up and gives
    /// an offset; an offset is not an address anyone can go read. This maps the
    /// window back through the image's symbol table so the finding can name the
    /// routine — `W-306` measured nine tenths of `shrinkhala.t1` inside 6,064
    /// octets, and until that window has a name it is a shape without a site.
    ///
    /// **IT MUST BE ABLE TO ANSWER MORE THAN ONE.** A span that straddles two
    /// routines gets two entries, not the nearer symbol: an instrument that
    /// always named exactly one routine would name one whether or not that were
    /// true, and could never be caught. A stretch of the span no symbol claims is
    /// its own entry with `name: None`, reported only when the run actually
    /// reached something in it — an unnamed gap the run never entered is padding,
    /// and padding is not a finding.
    ///
    /// **AND ONLY WHAT RAN.** That last rule holds for NAMED routines too: a
    /// routine the window spans but the run never entered is not an owner. The
    /// length of this list is read as the spread — "half the steps in ONE
    /// routine" against "nine tenths across TEN" — so counting dead text the
    /// window merely reaches over would report a tight loop as a loose one. Every
    /// entry returned has at least one reached site, and the first and last
    /// always do, since a [`Span`]'s ends are sites the run reached.
    ///
    /// `routines` may be in any order; it is read in address order regardless.
    #[must_use]
    pub fn owners(&self, span: &Span, routines: &[Routine]) -> Vec<Owner> {
        let mut sorted: Vec<&Routine> = routines.iter().collect();
        sorted.sort_by_key(|r| (r.lo, r.hi));
        let mut owned: Vec<Owner> = Vec::new();
        // Where the last claimed octet ended, so a gap can be spotted before the
        // next routine as well as after the last one.
        let mut covered = span.lo;
        for r in sorted {
            if r.hi < span.lo || r.lo > span.hi {
                continue;
            }
            let lo = r.lo.max(span.lo);
            let hi = r.hi.min(span.hi);
            if lo > covered {
                push_gap(&mut owned, self, covered, lo - 1);
            }
            let (steps, sites) = self.steps_in(lo, hi);
            // A NAMED routine the run never entered is dropped on the same
            // terms as an unnamed one, and for the same reason. `covered` still
            // advances past it, so dropping it leaves no unnamed hole where it
            // stood — the window is not reported as having a gap it does not
            // have. The count of owners is read as "one hot loop, or spread
            // over how many?", and dead text the window merely straddles is not
            // part of the spread.
            if sites > 0 {
                owned.push(Owner {
                    name: Some(r.name.clone()),
                    lo,
                    hi,
                    steps,
                    sites,
                });
            }
            covered = covered.max(hi.saturating_add(1));
        }
        if covered <= span.hi {
            push_gap(&mut owned, self, covered, span.hi);
        }
        owned
    }
}

/// Record an unnamed stretch, but only if the run reached anything in it.
fn push_gap(into: &mut Vec<Owner>, p: &StepProfile, lo: u64, hi: u64) {
    let (steps, sites) = p.steps_in(lo, hi);
    if sites > 0 {
        into.push(Owner {
            name: None,
            lo,
            hi,
            steps,
            sites,
        });
    }
}

/// What a run did to ONE routine: how many times it was ENTERED, and what it
/// spent once inside.
///
/// `W-306`: [`StepProfile::owners`] answers where the steps are and
/// [`Routine`] answers whose code that is, and TOGETHER THEY STILL CANNOT TELL
/// A HOT LOOP FROM A HOT CALLEE. 282,302 steps in `खण्डवृद्धिः` reads the same
/// whether the routine ran once and spun, or ran eight thousand times and did
/// thirty-four steps each. Those are opposite findings — the first is a loop to
/// go fix, the second is the work the source asked for and a question about
/// `DEFAULT_STEPS` instead — and the ratio is what separates them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    /// The routine's name, as the symbol table spells it.
    pub name: String,
    /// How many times the run began an instruction at the routine's FIRST
    /// address. Entries, not steps: a call lands on `lo` once per call.
    pub entries: u64,
    /// Steps anywhere inside the routine's extent.
    pub steps: u64,
    /// Distinct program counters inside it the run reached.
    pub sites: usize,
}

impl Visit {
    /// Mean steps per entry, or `None` for a routine the run never entered
    /// through its front door.
    ///
    /// `None` IS A READING AND NOT A MISSING VALUE. `entries == 0` with
    /// `steps > 0` is the third state this instrument exists to expose: code
    /// that ran without the run ever arriving at `lo`. That is either a jump
    /// into a routine's middle or — far likelier — an extent inferred wrong,
    /// because [`routines`] reads a routine as reaching up to the next NAME and
    /// an unnamed neighbour is silently annexed. A division that answered `0`
    /// there, or that skipped the row, would spell breakage as quiet.
    #[must_use]
    pub fn steps_per_entry(&self) -> Option<u64> {
        (self.entries > 0).then(|| self.steps / self.entries)
    }
}

impl StepProfile {
    /// One routine's entries, steps and reached sites.
    #[must_use]
    pub fn visit(&self, r: &Routine) -> Visit {
        let (steps, sites) = self.steps_in(r.lo, r.hi);
        Visit {
            name: r.name.clone(),
            // THE ENTRY PC ONLY. Not the max count in the range and not their
            // sum: a routine entered once around a hundred-trip loop would read
            // 100 under the first and 103 under the second, and the whole point
            // of this number is to be the one the loop does NOT inflate.
            entries: self.at(r.lo),
            steps,
            sites,
        }
    }

    /// The `n` busiest routines by steps, most first; ties by name so the
    /// reading is stable across runs.
    ///
    /// Routines the run reached NOTHING in are dropped — as in
    /// [`StepProfile::owners`], dead text is not part of the spread. A routine
    /// with steps but no entries is KEPT, because that is the state worth
    /// seeing.
    #[must_use]
    pub fn busiest(&self, routines: &[Routine], n: usize) -> Vec<Visit> {
        let mut visits: Vec<Visit> = routines
            .iter()
            .map(|r| self.visit(r))
            .filter(|v| v.sites > 0)
            .collect();
        visits.sort_by(|a, b| b.steps.cmp(&a.steps).then(a.name.cmp(&b.name)));
        visits.truncate(n);
        visits
    }
}

/// Does a `T1_STEP_PROFILE` request name this source?
///
/// `all` names every source; otherwise the request matches the file name
/// (`shrinkhala.t1`) OR its bare stem (`shrinkhala`). Both spellings, because
/// `T1_CORPUS` takes the stem and a reader who narrowed with one spelling
/// reaches for the same one here — measured 2026-09-28, the stem named nothing
/// and the census read exactly like a source with no hot span.
///
/// It lives here rather than in the census so that [`armed`] and the census
/// cannot drift apart on what "matched" means: a switch whose two readers
/// disagree is the same silence by another route.
#[must_use]
pub fn names_source(request: &str, source: &str) -> bool {
    request == "all" || request == source || source.strip_suffix(".t1") == Some(request)
}

/// One census row, as much of it as [`armed`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reached<'a> {
    /// The source's file name, as the census spells it.
    pub source: &'a str,
    /// The furthest stage the chain reached for it — `"run"`, `"link"`, ….
    pub stage: &'a str,
    /// How many labels the image's link could not resolve for it. A source that
    /// stopped at `link` with a non-zero count stopped because THIS RUN'S
    /// corpus holds no object defining them — the remedy is a wider
    /// `T1_CORPUS`, not a different test. Zero with the same stage is the
    /// opposite finding: everything resolved and the stop is the test's own.
    pub unresolved: usize,
    /// Whether the profiler actually ran for it.
    pub profiled: bool,
}

/// WHAT A `T1_STEP_PROFILE` REQUEST REACHED — the third state the switch had no
/// way to say.
///
/// A profiler that prints nothing has THREE causes and the census could spell
/// only one of them, so all three read as "this source has no hot span":
///
/// 1. it fired — profile lines are in the output;
/// 2. it matched a source the chain never ran — measured 2026-09-29,
///    `T1_FULL_CENSUS=1 T1_STEP_PROFILE=shrinkhala measure_corpus_encode` ran
///    209 s, passed, and printed no profile line at all, because `shrinkhala.t1`
///    stops at LINK there and the profiler is inside the run stage;
/// 3. it matched nothing at all — the spelling trap [`names_source`] describes.
///
/// (2) and (3) call for opposite next work — a different test versus a
/// different spelling — and an instrument with two states where the truth has
/// three hides its own breakage. So this is a struct and not a bool: the counts
/// are kept apart and [`Armed::report`] cannot spell one as the other.
///
/// # AND (2) IS ITSELF TWO THINGS, measured 2026-09-29
///
/// `@link` alone was read as "this test cannot reach that source" and sent the
/// last cycle looking for a different test. It is not what the census meant.
/// `T1_CORPUS=shrinkhala T1_STEP_PROFILE=shrinkhala` leaves **33** cross-module
/// labels unresolved, because a narrowed census links each row only against the
/// OTHER ROWS OF THE SAME RUN and a one-source run has none. So the remedy is a
/// wider `T1_CORPUS`, and it is a different remedy from the one `@link` with
/// ZERO unresolved asks for. [`Reached::unresolved`] carries the count and
/// [`Armed::report`] prints it, so the reader is not left to infer which of the
/// two they have.
///
/// ## AND "WIDER IS BETTER" IS NOT A LAW — measured the same day, 375 s
///
/// The first draft of this margin said the count falls MONOTONICALLY as the
/// corpus widens, off three points: 33 at 1 source, 25 at 11, 9 at 14. The
/// fourth point refutes it. At 20 sources (every `.t1` but `lib.t1`)
/// `shrinkhala.t1` has **75** unresolved, and the census row says why in its own
/// stage column: `ir.t1` stops at **emit** there, where at 14 it reached `link`,
/// so no `मध्यरूप` object is in the image at all and every `मध्यरूपॱ*` label
/// goes unresolved. `yantrotsarjana.t1` inherits the same 71.
///
/// That is the reason the line prints a COUNT and not a verdict: a widened
/// corpus can lose a module, and the number moving the wrong way is the only
/// thing that says so. FIFTEEN of those twenty sources DO reach `run`, and the
/// five that stop are `ir`@emit and `sarani`/`shrinkhala`/`utsarjana`/
/// `yantrotsarjana`@link — the profiler has plenty to fire on; `shrinkhala` is
/// simply not one of them until `ir.t1` emits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Armed {
    /// Sources the profiler ran for.
    pub fired: Vec<String>,
    /// Sources the request named that the profiler did NOT run for, each with
    /// the stage the chain did reach and the labels its link left unresolved.
    pub stopped: Vec<Stop>,
}

/// One source the request named and the profiler did not run for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    /// The source's file name, as the census spells it.
    pub source: String,
    /// The furthest stage its chain reached.
    pub stage: String,
    /// The labels its link could not resolve, from [`Reached::unresolved`].
    pub unresolved: usize,
}

impl Armed {
    /// How many sources the request named, fired or not.
    #[must_use]
    pub fn matched(&self) -> usize {
        self.fired.len() + self.stopped.len()
    }

    /// The one line the census prints — a distinct sentence per state, because
    /// the whole point is that a reader can tell them apart at a glance.
    ///
    /// It never answers "nothing to report": a request that was set and did not
    /// fire is a finding, and `all` firing on every source is the only case
    /// where the line is merely a receipt.
    #[must_use]
    pub fn report(&self, request: &str) -> String {
        let stopped = || {
            self.stopped
                .iter()
                .map(|s| {
                    if s.unresolved == 0 {
                        format!("{}@{}", s.source, s.stage)
                    } else {
                        format!("{}@{} ({} unresolved)", s.source, s.stage, s.unresolved)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        if self.matched() == 0 {
            format!(
                "unmatched {request} 0 source(s): the request names nothing in this corpus \
                 — check the spelling against the census's own row names"
            )
        } else if self.fired.is_empty() {
            format!(
                "never-fired {request} {} matched, 0 ran: {} — the profiler is inside the \
                 run stage and no matched source reached it{}",
                self.stopped.len(),
                stopped(),
                if self.stopped.iter().any(|s| s.unresolved > 0) {
                    "; the unresolved labels have no object in THIS run's corpus \
                     — widen T1_CORPUS rather than changing the test"
                } else {
                    ""
                }
            )
        } else {
            format!(
                "fired {request} {} of {}: {}{}",
                self.fired.len(),
                self.matched(),
                self.fired.join(" "),
                if self.stopped.is_empty() {
                    String::new()
                } else {
                    format!("; {} stopped earlier: {}", self.stopped.len(), stopped())
                }
            )
        }
    }
}

/// Split a census's rows by whether the `T1_STEP_PROFILE` request reached them.
///
/// Order is the rows' own, so the reading is stable across runs.
#[must_use]
pub fn armed(request: &str, rows: &[Reached<'_>]) -> Armed {
    let mut a = Armed::default();
    for r in rows.iter().filter(|r| names_source(request, r.source)) {
        if r.profiled {
            a.fired.push(r.source.to_string());
        } else {
            a.stopped.push(Stop {
                source: r.source.to_string(),
                stage: r.stage.to_string(),
                unresolved: r.unresolved,
            });
        }
    }
    a
}
