//! **`W-376`: COOPERATIVE THREADS, SWITCHED BY THE HOST ONLY AT WAITS, THE SCHEDULE IN THE
//! EVENT LOG** (ADR-0040 Option C, "What follows → Threads"; the addendum
//! `docs/adr/0040-addendum-w376-threads.md`, accepted by the owner "All as recommended").
//!
//! # One machine, N saved contexts
//!
//! A thread is a saved register context over ONE [`Machine`]: `x`, `f`, `fcsr`, `pc` and
//! `vec` ([`Context`]). RAM, the devices, the CSRs and `time` are shared, because the
//! machine owns them and a second machine would mean a second RAM and a second clock. This
//! is `process.rs`'s PATTERN — the saved register file and its switch — and not its
//! `Kernel`, which gives every process disjoint memory, steps U-mode `ecall` programs and
//! cannot reach `WAIT` (addendum §0). Every field a switch touches is already `pub`, so
//! `Machine` does not change. The `lr`/`sc` reservation is dropped at every switch
//! (`process.rs`'s rule), so an `sc` can never succeed against another thread's `lr`.
//!
//! # Fixed at load
//!
//! The image declares `SASTHRDS` ([`THREADS_TAG`]) with its COUNT word after it (1 to
//! [`MAX_THREADS`]), and `SASTHRID` ([`THREAD_ID_TAG`]) with a `न६४` slot after it, which
//! the host rewrites with the running thread's number at EVERY switch-in. Both are found the
//! way [`crate::input::find_event_slot`] finds its slot, with its refusals. A threaded image
//! must also declare `SASEVENT`: a decision point is a wait, and a wait without an event
//! word would resume onto nothing.
//!
//! **ONLY THE FILE-BACKED OCTETS ARE SCANNED**, all three tags in one pass ([`scan`]): a tag
//! is an initialised global, so it is in the file, and `.bss` is zero at load. Scanning all
//! of RAM cost every image — threaded or not — two passes over it: 0.65 s to 2.25 s for a
//! trivial image at 553 MB of RAM (review of ad6f0800).
//!
//! **AN UNTHREADED PROGRAM WHOSE INITIALISED DATA HOLDS THE `SASTHRDS` WORD IS REFUSED**
//! ("the image declares SASTHRDS … without SASTHRID"): a single global equal to
//! `0x5344524854534153` is, to a scan by value, half of a thread interface, and a half-
//! declared interface must not run silently single-threaded. Such a program ran on main
//! before `W-376`; it now exits 1 at load. A word that only appears at run time, or in
//! `.bss`, is never scanned and is not a tag.
//!
//! Every thread runs the SAME startup into the SAME entry routine; the thread id chooses its
//! role. Thread 0 starts at `e_entry`, as every image does. Thread k ≥ 1 starts at
//! `e_entry + 8` with `sp` at the top of a host-made 64 KiB stack and every other register
//! zero: those eight octets are the startup's `auipc sp` / `addi sp,sp`, and [`discover`]
//! DECODES both and refuses an image whose first two words are anything else, naming them.
//!
//! **WHAT RAISING THE STORE BOUND WOULD COST, AND THE GUARD THAT PAYS IT.** Before threads,
//! a record allocator that outgrew `ram` halted `BeyondRam` at the bound. With N ≥ 2 the
//! bound sits above the host stacks, so a heap past `ram` would store into thread 1's stack
//! and read back whatever thread 1 left there — measured in review on ad6f0800: a run of
//! 1..8 summed to 58, status 0. `Machine` keeps its one bound (the ruling leaves it
//! unchanged), so each segment runs through a GUARD over the `Output` it is given: the
//! first store at or above the old top of RAM that is NOT inside the running thread's own
//! host stack ends the whole run as [`ThreadsEnd::BeyondOwnStack`], worded like `BeyondRam`
//! and naming the thread. That covers the heap past RAM (thread 0 has no host stack, so
//! any store up there is outside its own) and thread k overflowing DOWN out of its stack
//! into thread j's, which stack k − 1 sits right below. The store has landed by the time
//! the guard sees it, and the segment runs on to its wait or end; nothing after it runs.
//!
//! **NOT GUARDED, ON PURPOSE:** thread 1's stack overflowing down past its bottom into the
//! top of RAM, below the old top. That is the class thread 0's own stack overflow already is
//! — a stack running into the heap below it, which no bound in this machine has ever
//! caught — and it is left exactly as it was.
//!
//! The stacks are appended ABOVE `ram`, with `store_limit` raised over them, by
//! [`Threads::new`] — which must run BEFORE [`crate::input::inject`], so the injected slab
//! still sits above the store bound and `W-363` holds. [`Threads::new`] refuses a machine
//! whose RAM already reaches past its store bound, which is what an injection leaves.
//!
//! # The end rule
//!
//! A thread ENDS when its entry routine returns and the startup's finisher store halts
//! ([`Halt::Finisher`]); inside the thread host that means "thread k ended with status s",
//! and the others go on. The run ends when every thread has ended, and it SUCCEEDS only if
//! every status is 0 ([`first_failure`] names the first, in the order they ended, that is
//! not). ANY OTHER HALT — a fault, `BeyondRam`, the step limit — ends the whole run, because
//! RAM is shared and the other threads would run on whatever the failure left.
//!
//! # No preemption
//!
//! The host never cuts a slice: each segment is given the whole remaining budget, `timecmp`
//! and `sie` are left alone, and a switch happens only at a wait or a thread's end. That is
//! also why sharing RAM needs no lock: no inline sequence — an allocation, say — contains a
//! wait. The switch is host time and RETIRES NO INSTRUCTION, so `time`, which accumulates
//! across segments, stays a function of (program, log).
//!
//! # The schedule is a log
//!
//! At each decision point (the start, every wait, every thread's end) the host takes the
//! next record of a [`crate::input::parse_thread_log`] log: `@N`, a thread that has not
//! ended, and — when N is resuming from a wait — the value record after it, which is
//! written into `SASEVENT`'s word AT THE RESUME, never at the wait: the word is shared, and
//! a value written at the wait would be overwritten by the next thread to wait.
//! [`replay_threads`] follows a log; [`record_live_threads`] makes one, round-robin, so a
//! replay needs no policy at all.

use crate::input::ThreadRecord;
use crate::vector::VectorUnit;
use crate::{Halt, Machine, Output};

/// The tag in front of the thread COUNT word: `"SASTHRDS"` read as a little-endian word
/// (generated from the octets by a tool, never typed; `the_tags_spell_their_names` below).
/// Below 2⁶³ like every host tag: the `.t1` literal that declares it goes through a signed
/// lowering.
pub const THREADS_TAG: u64 = 0x5344_5248_5453_4153;
/// The tag in front of the THREAD-ID slot: `"SASTHRID"`.
pub const THREAD_ID_TAG: u64 = 0x4449_5248_5453_4153;
const _: () = assert!(THREADS_TAG < 1 << 63 && THREAD_ID_TAG < 1 << 63);

/// The most threads an image may declare (addendum §1).
pub const MAX_THREADS: u64 = 64;
/// Each host-made stack, for threads 1 to N−1: the image's own stack size (thread 0 keeps
/// that one).
pub const STACK_OCTETS: usize = 64 * 1024;

/// `auipc sp, …`: opcode `0x17`, `rd = x2` — the low twelve bits of the startup's first word.
const AUIPC_SP: u32 = 0x17 | 2 << 7;
/// `addi sp, sp, …`: opcode `0x13`, `rd = x2`, `funct3 = 0`, `rs1 = x2` — the low twenty
/// bits of its second.
const ADDI_SP_SP: u32 = 0x13 | 2 << 7 | 2 << 15;

fn word_at(mem: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(mem[at..at + 8].try_into().expect("eight octets"))
}

fn put_word(mem: &mut [u8], at: usize, w: u64) {
    mem[at..at + 8].copy_from_slice(&w.to_le_bytes());
}

/// Where the three tags a threaded image declares sit in RAM, from [`scan`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags {
    /// Offsets of `SASTHRDS`.
    pub threads: Vec<usize>,
    /// Offsets of `SASTHRID`.
    pub ids: Vec<usize>,
    /// Offsets of `SASEVENT`.
    pub events: Vec<usize>,
}

impl Tags {
    /// The COUNT an image declares, for `yantra-run`'s `threads: N` line: the word after
    /// the FIRST `SASTHRDS` tag, whenever there is one — said even when [`discover_tags`]
    /// then refuses the image, so a log shows that the image was threaded.
    #[must_use]
    pub fn declared_count(&self, mem: &[u8]) -> Option<u64> {
        let first = *self.threads.first()?;
        (first + 16 <= mem.len()).then(|| word_at(mem, first + 8))
    }
}

/// ONE PASS over the FILE-BACKED octets of `image` as loaded in `m` (each `PT_LOAD`'s
/// `filesz`, never its zeroed `.bss` tail), at every offset — not every eighth, for the
/// reason at `input::inject`'s scan — for all three tags.
#[must_use]
pub fn scan(m: &Machine, image: &[u8]) -> Tags {
    let mut tags = Tags::default();
    let Ok(program) = crate::loader::Program::parse(image) else {
        return tags;
    };
    for s in &program.segments {
        let Some(start) = s
            .vaddr
            .checked_sub(m.base)
            .and_then(|o| usize::try_from(o).ok())
        else {
            continue;
        };
        let end = start.saturating_add(s.filesz).min(m.mem.len());
        for o in start..end.saturating_sub(7) {
            match word_at(&m.mem, o) {
                THREADS_TAG => tags.threads.push(o),
                THREAD_ID_TAG => tags.ids.push(o),
                crate::input::EVENT_TAG => tags.events.push(o),
                _ => {}
            }
        }
    }
    for v in [&mut tags.threads, &mut tags.ids, &mut tags.events] {
        v.sort_unstable();
        v.dedup();
    }
    tags
}

/// One tag's single hit, or `find_event_slot`'s refusals for a tag found more than once or
/// with no word after it.
fn the_one(mem: &[u8], found: &[usize], what: &str) -> Result<usize, String> {
    match found {
        [one] if one + 16 <= mem.len() => Ok(*one),
        [one] => Err(format!("the {what} tag at {one:#x} has no word after it")),
        many => Err(format!(
            "the {what} tag appears at {} words ({:x?}); the tag must be unique or the scan \
             cannot tell which word the program reads",
            many.len(),
            many.iter().take(6).collect::<Vec<_>>()
        )),
    }
}

/// What a threaded image declared, as [`discover`] found and checked it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// How many threads, 1 to [`MAX_THREADS`].
    pub count: usize,
    /// Offset of the `SASTHRDS` tag in RAM (the count is at `+ 8`).
    pub threads_tag: usize,
    /// Offset of the `SASTHRID` tag (the thread-id slot is at `+ 8`).
    pub id_tag: usize,
    /// Offset of the `SASEVENT` tag (the event word is at `+ 8`).
    pub event_tag: usize,
    /// `e_entry`: thread 0 starts here, thread k ≥ 1 at `entry + 8`.
    pub entry: u64,
    /// The `sp` the startup's two words compute — thread 0's stack top.
    pub startup_sp: u64,
}

/// Find and check the thread interface in a machine fresh from `load_elf` of `image`
/// (`m.pc` is still `e_entry`): [`scan`], then [`discover_tags`]. `Ok(None)` is an image
/// that declares neither tag: an ordinary, single-thread image, which nothing here touches.
///
/// # Errors
/// [`discover_tags`]'s.
pub fn discover(m: &Machine, image: &[u8]) -> Result<Option<Declared>, String> {
    discover_tags(m, &scan(m, image))
}

/// Check the thread interface [`scan`] found.
///
/// # Errors
/// Refused at load, before one instruction runs, naming the reason: one of `SASTHRDS` /
/// `SASTHRID` without the other; either tag duplicated or with no word after it; a count of
/// 0 or above [`MAX_THREADS`]; no `SASEVENT` interface; a startup whose first two words are
/// not `auipc sp` / `addi sp,sp` (both words named).
pub fn discover_tags(m: &Machine, tags: &Tags) -> Result<Option<Declared>, String> {
    let mem = &m.mem;
    let (threads, ids) = (&tags.threads, &tags.ids);
    match (threads.is_empty(), ids.is_empty()) {
        (true, true) => return Ok(None),
        (false, true) => {
            return Err(format!(
                "the image declares SASTHRDS ({THREADS_TAG:#x}) without SASTHRID \
                 ({THREAD_ID_TAG:#x}): a threaded image must declare both — the thread count \
                 and the slot the host writes each thread's number into"
            ));
        }
        (true, false) => {
            return Err(format!(
                "the image declares SASTHRID ({THREAD_ID_TAG:#x}) without SASTHRDS \
                 ({THREADS_TAG:#x}): a threaded image must declare both — the thread count \
                 and the slot the host writes each thread's number into"
            ));
        }
        (false, false) => {}
    }
    let threads_tag = the_one(mem, threads, "SASTHRDS")?;
    let id_tag = the_one(mem, ids, "SASTHRID")?;
    let count = word_at(mem, threads_tag + 8);
    if !(1..=MAX_THREADS).contains(&count) {
        return Err(format!(
            "the SASTHRDS count is {count}: an image declares 1 to {MAX_THREADS} threads"
        ));
    }
    // `find_event_slot`'s three refusals, over the scanned octets.
    let event_tag = match tags.events.as_slice() {
        [] => Err(format!(
            "no event interface in this image: the SASEVENT tag ({:#x}) appears at no \
             word. It was built without the event global — declare it, or do not pass an \
             event log",
            crate::input::EVENT_TAG
        )),
        found => the_one(mem, found, "SASEVENT"),
    }
    .map_err(|e| format!("a threaded image needs the event interface — {e}"))?;
    let entry = m.pc;
    let at = entry
        .checked_sub(m.base)
        .and_then(|o| usize::try_from(o).ok())
        .filter(|o| o + 8 <= mem.len())
        .ok_or_else(|| format!("e_entry {entry:#x} is not eight octets inside RAM"))?;
    let w0 = u32::from_le_bytes(mem[at..at + 4].try_into().expect("four octets"));
    let w1 = u32::from_le_bytes(mem[at + 4..at + 8].try_into().expect("four octets"));
    if w0 & 0xfff != AUIPC_SP || w1 & 0xf_ffff != ADDI_SP_SP {
        return Err(format!(
            "the startup at e_entry {entry:#x} begins {w0:#010x} {w1:#010x}, not the two sp \
             words the host recognises (`auipc sp, hi` then `addi sp, sp, lo`): thread k >= 1 \
             enters at e_entry + 8, past exactly those two words, and the host will not \
             guess what it would be skipping"
        ));
    }
    let hi = i64::from((w0 & 0xffff_f000) as i32);
    let lo = i64::from((w1 as i32) >> 20);
    let startup_sp = entry.wrapping_add(hi as u64).wrapping_add(lo as u64);
    Ok(Some(Declared {
        count: count as usize,
        threads_tag,
        id_tag,
        event_tag,
        entry,
        startup_sp,
    }))
}

/// One thread's saved registers: what a switch saves and restores. RAM, devices, CSRs and
/// `time` are the machine's and are shared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    /// `x0`..`x31`.
    pub x: [u64; 32],
    /// `f0`..`f31`, as raw bits.
    pub f: [u64; 32],
    /// `fcsr`.
    pub fcsr: u64,
    /// Where the thread resumes.
    pub pc: u64,
    /// The vector unit.
    pub vec: VectorUnit,
}

impl Context {
    fn of(m: &Machine) -> Self {
        Context {
            x: m.x,
            f: m.f,
            fcsr: m.fcsr,
            pc: m.pc,
            vec: m.vec.clone(),
        }
    }

    fn restore(&self, m: &mut Machine) {
        m.x = self.x;
        m.f = self.f;
        m.fcsr = self.fcsr;
        m.pc = self.pc;
        m.vec = self.vec.clone();
        // `process.rs`'s rule: a reservation is one hart's, and it does not survive a switch.
        m.reservation = None;
    }
}

/// Where a thread stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadState {
    /// Never switched in.
    Fresh,
    /// Switched in now.
    Running,
    /// Paused at the wait store at `pc`; resumes with a value record.
    Waiting {
        /// The wait store.
        pc: u64,
    },
    /// Cut off mid-segment. Reachable ONLY under [`Mutant::SwitchEvery`]: the real host
    /// never preempts.
    Preempted,
    /// Its finisher store halted.
    Ended {
        /// The raw finisher value.
        value: u64,
        /// The decoded status (`None`: a value the finisher does not define).
        status: Option<u64>,
    },
}

/// One thread's end, in the order the threads ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct End {
    /// Which thread.
    pub thread: u32,
    /// The raw finisher value.
    pub value: u64,
    /// The decoded status.
    pub status: Option<u64>,
    /// `Machine::time` when it ended.
    pub at: u64,
}

/// A HOST MUTANT, for the tests' red controls only (`tests/w376_threads.rs`): each is one
/// deviation from the ruled host, and each must turn its test red. `yantra-run` never sets
/// one; [`Threads::new`] makes `None`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutant {
    /// Preempt: cut every segment after this many instructions and take the next record.
    SwitchEvery(u64),
    /// Deliver a thread's value at its WAIT (looked ahead in the log), not at its resume.
    DeliverAtWait,
    /// Give threads k ≥ 1 the startup's own `sp`, the stack thread 0 is using.
    SharedSp,
    /// Write the thread-id slot only at a thread's FIRST switch-in, not at every one.
    NoIdRewrite,
}

/// N saved thread contexts over one machine, and the record of what they did.
#[derive(Debug, Clone)]
pub struct Threads {
    /// What the image declared.
    pub declared: Declared,
    /// Each thread's saved registers (the running thread's are in the machine).
    pub contexts: Vec<Context>,
    /// Each thread's state.
    pub states: Vec<ThreadState>,
    /// Instructions each thread has retired.
    pub retired: Vec<u64>,
    /// Every switch-in, as `(thread, Machine::time at it)`.
    pub switches: Vec<(u32, u64)>,
    /// Every thread's end, in the order they ended.
    pub ends: Vec<End>,
    /// RAM offset where the host-made stacks begin (thread 1's is the lowest).
    pub stacks_at: usize,
    /// The RAM the image was given, before the stacks: the guard's line. A store at or
    /// above it that is not in the running thread's own stack ends the run.
    pub ram_top: usize,
    /// The thread switched in now, or last.
    pub current: Option<u32>,
    #[doc(hidden)]
    pub mutant: Option<Mutant>,
}

impl Threads {
    /// Make the contexts and the stacks of threads 1 to N−1: (N−1) × [`STACK_OCTETS`]
    /// appended above `ram`, `store_limit` raised over them, each thread's `sp` the top of
    /// its own. Thread 0's context is the machine as loaded.
    ///
    /// # Errors
    /// A machine whose RAM already reaches past its store bound — an [`crate::input::inject`]
    /// has run, and the stacks would sit above the slab with the program free to store
    /// through it. The stacks are made FIRST.
    pub fn new(m: &mut Machine, declared: Declared) -> Result<Self, String> {
        let top = m.mem.len();
        if m.store_limit != usize::MAX && m.store_limit < top {
            return Err(format!(
                "the thread stacks must be made BEFORE the input is injected: RAM is {top} \
                 octets and stores stop at {}, so something already sits above the store \
                 bound, and stacks above it would leave it writable",
                m.store_limit
            ));
        }
        let n = declared.count;
        let stacks_at = top.div_ceil(16) * 16;
        m.mem.resize(stacks_at + (n - 1) * STACK_OCTETS, 0);
        if m.store_limit != usize::MAX {
            m.store_limit = m.mem.len();
        }
        let first = Context::of(m);
        let mut contexts = vec![first];
        for k in 1..n {
            let mut c = Context {
                x: [0; 32],
                f: [0; 32],
                fcsr: 0,
                pc: declared.entry.wrapping_add(8),
                vec: VectorUnit::default(),
            };
            c.x[2] = m.base + (stacks_at + k * STACK_OCTETS) as u64;
            contexts.push(c);
        }
        Ok(Threads {
            contexts,
            states: vec![ThreadState::Fresh; n],
            retired: vec![0; n],
            switches: Vec::new(),
            ends: Vec::new(),
            stacks_at,
            ram_top: top,
            current: None,
            mutant: None,
            declared,
        })
    }

    /// How many threads.
    #[must_use]
    pub fn count(&self) -> usize {
        self.declared.count
    }

    fn all_ended(&self) -> bool {
        self.states
            .iter()
            .all(|s| matches!(s, ThreadState::Ended { .. }))
    }

    /// Apply a mutant's change to the contexts it governs (the tests set the field, then
    /// call this; [`replay_threads`] reads the rest of it as it runs).
    #[doc(hidden)]
    pub fn set_mutant(&mut self, mutant: Mutant) {
        if mutant == Mutant::SharedSp {
            for c in self.contexts.iter_mut().skip(1) {
                c.x[2] = self.declared.startup_sp;
            }
        }
        self.mutant = Some(mutant);
    }

    /// SWITCH IN thread `k`: restore its registers, drop the reservation, write `k` into
    /// the thread-id slot. Host time: no instruction retires.
    fn switch_in(&mut self, m: &mut Machine, k: u32) {
        let i = k as usize;
        let fresh = self.states[i] == ThreadState::Fresh;
        self.contexts[i].restore(m);
        if fresh || self.mutant != Some(Mutant::NoIdRewrite) {
            put_word(&mut m.mem, self.declared.id_tag + 8, u64::from(k));
        }
        self.states[i] = ThreadState::Running;
        self.current = Some(k);
        self.switches.push((k, m.time));
    }

    /// Thread `k`'s own host stack, as a RAM offset range; `None` for thread 0, whose
    /// stack is the image's own, below [`Threads::ram_top`].
    #[must_use]
    pub fn own_stack(&self, k: u32) -> Option<std::ops::Range<usize>> {
        let k = k as usize;
        (k >= 1).then(|| self.stacks_at + (k - 1) * STACK_OCTETS..self.stacks_at + k * STACK_OCTETS)
    }

    /// Run the switched-in thread `k` for up to `budget` instructions through the store
    /// GUARD, and park it by its halt. `Err` is an end of the whole run.
    fn segment(
        &mut self,
        m: &mut Machine,
        k: u32,
        budget: u64,
        out: &mut impl Output,
    ) -> Result<(), ThreadsEnd> {
        let i = k as usize;
        let preempt = match self.mutant {
            Some(Mutant::SwitchEvery(q)) if q < budget => Some(q),
            _ => None,
        };
        let before = m.time;
        let mut guard = Guard {
            out,
            top: self.ram_top,
            own: self.own_stack(k).unwrap_or(0..0),
            first: None,
        };
        let halt = m.run(preempt.unwrap_or(budget), &mut guard);
        let outside = guard.first;
        self.retired[i] += m.time - before;
        if let Some(at) = outside {
            return Err(ThreadsEnd::BeyondOwnStack {
                thread: k,
                addr: m.base + at as u64,
                ram: self.ram_top,
                stack: self
                    .own_stack(k)
                    .map(|r| (m.base + r.start as u64, m.base + r.end as u64)),
                halt,
            });
        }
        match halt {
            Halt::Wait { pc } => {
                self.contexts[i] = Context::of(m);
                self.states[i] = ThreadState::Waiting { pc };
                Ok(())
            }
            Halt::Finisher { value, status } => {
                self.states[i] = ThreadState::Ended { value, status };
                self.ends.push(End {
                    thread: k,
                    value,
                    status,
                    at: m.time,
                });
                Ok(())
            }
            Halt::StepLimit { .. } if preempt.is_some() => {
                self.contexts[i] = Context::of(m);
                self.states[i] = ThreadState::Preempted;
                Ok(())
            }
            other => Err(ThreadsEnd::Halted {
                thread: k,
                halt: other,
            }),
        }
    }
}

/// THE STORE GUARD: an [`Output`] over the caller's that records the first store at or
/// above `top` not wholly inside `own`, and passes everything through unchanged.
struct Guard<'a, O: Output> {
    out: &'a mut O,
    top: usize,
    own: std::ops::Range<usize>,
    first: Option<usize>,
}

impl<O: Output> Output for Guard<'_, O> {
    fn putc(&mut self, byte: u8) {
        self.out.putc(byte);
    }
    fn stored(&mut self, at: usize, width: usize) {
        if self.first.is_none()
            && at + width > self.top
            && !(self.own.start <= at && at + width <= self.own.end)
        {
            self.first = Some(at);
        }
        self.out.stored(at, width);
    }
    fn counter_read(&mut self) {
        self.out.counter_read();
    }
}

/// The first thread, in the order the threads ended, whose status is not 0 — the one a
/// failed threaded run reports. `None`: every thread ended with status 0.
#[must_use]
pub fn first_failure(t: &Threads) -> Option<&End> {
    t.ends.iter().find(|e| e.status != Some(0))
}

/// How a threaded run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadsEnd {
    /// Every thread ended, and the log was consumed exactly; the statuses are in
    /// [`Threads::ends`] and [`first_failure`] judges them.
    Ended,
    /// A halt that is not a thread's finisher ended the whole run.
    Halted {
        /// The thread that halted.
        thread: u32,
        /// The halt.
        halt: Halt,
    },
    /// The log ran out at a decision: `index` is the record that was needed (it equals the
    /// number of records), `thread` the one at the decision — the thread that just waited
    /// or ended, or, for a missing value record, the thread that was resuming. NEVER
    /// PADDED: nothing was resumed.
    Short {
        /// The record needed.
        index: usize,
        /// The thread at the decision.
        thread: Option<u32>,
    },
    /// Every thread ended with records left over: a log of some other run.
    Long {
        /// Records consumed.
        consumed: usize,
        /// Records the log held.
        len: usize,
    },
    /// `BeyondRam`, FOR A THREAD: thread `thread` stored at `addr`, at or above the `ram`
    /// octets the image was given and outside its own host stack `stack` (`None`: thread
    /// 0, which has none) — its heap grew past RAM, or its stack overflowed into another
    /// thread's. The store landed; the run ends at the end of that segment, whose halt is
    /// `halt`, and no other thread runs on what it overwrote.
    BeyondOwnStack {
        /// The thread that stored.
        thread: u32,
        /// The first such store's address.
        addr: u64,
        /// The RAM the image was given, in octets (the stacks sit above it).
        ram: usize,
        /// The thread's own stack, `[start, end)`, as addresses.
        stack: Option<(u64, u64)>,
        /// How the segment ended after it.
        halt: Halt,
    },
    /// A record that does not fit the run, at `index`: `@N` naming an ended or
    /// out-of-range thread, a value where `@N` is due, or `@N` where a value is due.
    Refused {
        /// The record.
        index: usize,
        /// Why, in words.
        why: String,
    },
}

/// One decision: the record at `*cursor` must be `@N` for a thread that has not ended, and
/// a value record must follow it when N is resuming from a wait. `Ok((N, value))`, the
/// cursor moved past what was read.
fn decide(
    t: &Threads,
    log: &[ThreadRecord],
    cursor: &mut usize,
) -> Result<(u32, Option<u64>), ThreadsEnd> {
    let index = *cursor;
    let k = match log.get(index) {
        None => {
            return Err(ThreadsEnd::Short {
                index,
                thread: t.current,
            });
        }
        Some(ThreadRecord::Value(v)) => {
            return Err(ThreadsEnd::Refused {
                index,
                why: format!(
                    "record {index} is the value {v}, and a decision point is due: the next \
                     record must be `@N`, the thread to run"
                ),
            });
        }
        Some(ThreadRecord::Run(k)) => *k,
    };
    let Some(state) = t.states.get(k as usize) else {
        return Err(ThreadsEnd::Refused {
            index,
            why: format!(
                "record {index} is `@{k}`, and the image declares {} threads (0 to {})",
                t.count(),
                t.count() - 1
            ),
        });
    };
    if matches!(state, ThreadState::Ended { .. }) {
        return Err(ThreadsEnd::Refused {
            index,
            why: format!("record {index} is `@{k}`, and thread {k} has ended"),
        });
    }
    *cursor += 1;
    if !matches!(state, ThreadState::Waiting { .. }) {
        return Ok((k, None));
    }
    match log.get(*cursor) {
        None => Err(ThreadsEnd::Short {
            index: *cursor,
            thread: Some(k),
        }),
        Some(ThreadRecord::Run(j)) => Err(ThreadsEnd::Refused {
            index: *cursor,
            why: format!(
                "record {} is `@{j}`, and thread {k} is resuming from a wait: its value \
                 record is due",
                *cursor
            ),
        }),
        Some(ThreadRecord::Value(v)) => {
            *cursor += 1;
            Ok((k, Some(*v)))
        }
    }
}

/// REPLAY a threaded log against `m` and `t`: at each decision point take [`decide`]'s
/// thread, deliver its value (if it is resuming) into the `SASEVENT` word, switch it in and
/// run it to its next wait or its end. `budget` covers the whole run, as in
/// [`crate::input::replay`].
pub fn replay_threads(
    m: &mut Machine,
    t: &mut Threads,
    log: &[ThreadRecord],
    budget: u64,
    out: &mut impl Output,
) -> ThreadsEnd {
    let start = m.time;
    let event = t.declared.event_tag + 8;
    let mut cursor = 0;
    loop {
        if t.all_ended() {
            return if cursor < log.len() {
                ThreadsEnd::Long {
                    consumed: cursor,
                    len: log.len(),
                }
            } else {
                ThreadsEnd::Ended
            };
        }
        let (k, value) = match decide(t, log, &mut cursor) {
            Ok(d) => d,
            Err(end) => return end,
        };
        if let Some(v) = value
            && t.mutant != Some(Mutant::DeliverAtWait)
        {
            put_word(&mut m.mem, event, v);
        }
        t.switch_in(m, k);
        let left = budget.saturating_sub(m.time - start);
        if let Err(end) = t.segment(m, k, left, out) {
            return end;
        }
        // THE MUTANT: the value this thread will be given, written NOW, at its wait.
        if t.mutant == Some(Mutant::DeliverAtWait)
            && matches!(t.states[k as usize], ThreadState::Waiting { .. })
            && let Some(v) = log[cursor..].windows(2).find_map(|w| match w {
                [ThreadRecord::Run(j), ThreadRecord::Value(v)] if *j == k => Some(*v),
                _ => None,
            })
        {
            put_word(&mut m.mem, event, v);
        }
    }
}

/// The first line of a log [`record_live_threads`] writes: a comment, so it carries no
/// record.
pub const RECORDED_THREAD_LOG_HEADER: &str = "# yantra-run --record-events (W-376): a threaded \
     run; @N ran thread N at each decision point, round-robin, and t=<host time in ns since the \
     Unix epoch, UTC> is the value a resuming thread was given; replay with --events";

/// LIVE MODE for a threaded image: at each decision point pick the next thread that has
/// not ended, ROUND-ROBIN from the one after the current (thread 0 first), write `@N`; when
/// N is resuming, stamp the host's time ([`crate::input::stamp_event_time`], still the one
/// clock read), deliver it and write `t=<ns>`. Every line is flushed as it is written. The
/// log replays through [`replay_threads`] to the same count, because every pick is in it —
/// a replay does not care which policy made them (`W-377` may swap in readiness).
///
/// # Errors
/// A log that cannot be written.
pub fn record_live_threads(
    m: &mut Machine,
    t: &mut Threads,
    budget: u64,
    out: &mut impl Output,
    log: &mut impl std::io::Write,
) -> Result<ThreadsEnd, String> {
    let io = |e: std::io::Error| format!("writing the event log: {e}");
    writeln!(log, "{RECORDED_THREAD_LOG_HEADER}").map_err(io)?;
    log.flush().map_err(io)?;
    let start = m.time;
    let event = t.declared.event_tag + 8;
    let n = t.count() as u32;
    loop {
        if t.all_ended() {
            return Ok(ThreadsEnd::Ended);
        }
        let from = t.current.map_or(0, |c| (c + 1) % n);
        let k = (0..n)
            .map(|d| (from + d) % n)
            .find(|&j| !matches!(t.states[j as usize], ThreadState::Ended { .. }))
            .expect("a thread that has not ended, since not all have");
        writeln!(log, "@{k}").map_err(io)?;
        if matches!(t.states[k as usize], ThreadState::Waiting { .. }) {
            let now = crate::input::stamp_event_time();
            writeln!(log, "t={now}").map_err(io)?;
            put_word(&mut m.mem, event, now);
        }
        log.flush().map_err(io)?;
        t.switch_in(m, k);
        let left = budget.saturating_sub(m.time - start);
        if let Err(end) = t.segment(m, k, left, out) {
            return Ok(end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tags_spell_their_names() {
        assert_eq!(&THREADS_TAG.to_le_bytes(), b"SASTHRDS");
        assert_eq!(&THREAD_ID_TAG.to_le_bytes(), b"SASTHRID");
    }
}
