//! Two processes, two address spaces, and one channel between them — task `C-014`,
//! doc 11 §7.
//!
//! [`crate::loader::load_application`] has built an address space since `F-001e2a`: a
//! fresh Sv39 root table, the program's segments under `U`, a stack, a handle vector, and
//! the supervisor's own gigapage mapped **without** `U` so a trap can fetch its handler
//! and the program cannot read a byte of it. This module holds two of those at once and
//! puts a boundary between them that something actually tries to cross.
//!
//! # What already existed, measured rather than taken from the row
//!
//! The row that asked for this said *"no address-space separation anywhere under
//! `crates/`"* and *"what does not exist is a second address space with a channel between
//! them"*. **The first is right and the second is wrong, and the difference matters.** On
//! the Sassembly side both already exist and are `done`:
//!
//! - `C-002l` — `spec/second-space.sas` + `tools/check-second-space.sh`: two root tables,
//!   and one virtual address answering differently under each `satp`.
//! - `C-002m1`/`C-002m2` — `spec/crossing.sas` and `spec/milestone-k2.sas`: a word leaves
//!   A's space, is read inside B's, and a derived reply comes back. That is M-K2's
//!   "two userspace processes exchange messages", and it is a channel.
//!
//! So this module does **not** claim either of those as new. What is new is narrower:
//!
//! - **Nothing in `crates/` ever held two spaces at once.** `load_application` installs
//!   `satp` on the machine as it builds, and the second call overwrites the first. There
//!   was no saved context, so no way to leave a space and come back to it. That is the
//!   substrate `F-007a`, `F-004g` and `F-005` are blocked on, and all three are
//!   browser-side, where `yantra` is the VM and there is no QEMU underneath.
//! - **Nothing anywhere had tried to read across the boundary and been refused.**
//!   `check-second-space.sh` says so in its own "what a green here does NOT prove"
//!   section, and `second-space.sas` repeats it: *"एक पता दो उत्तर देना यह दावा नहीं है कि
//!   कोई प्रक्रिया दूसरी के पृष्ठ तक पहुँच ही नहीं सकती"* — one address giving two answers
//!   is not the claim that one process cannot reach the other's page. **That is the whole
//!   of `C-014`**, and it is what `tests/process.rs` is for.
//!
//! The channel below is therefore not a new capability claim — `C-002m` holds that — but
//! the thing the isolation is tested *against*. It is the one component that touches both
//! address spaces, so it is the one component that could be talked into reading across on
//! a process's behalf, and
//! `tests/process.rs::the_channel_cannot_be_talked_into_reading_across_the_boundary`
//! poses that attack. Without a channel there is no deputy to confuse.
//!
//! # A process here is an address space, and deliberately not a thread
//!
//! A [`Process`] is a root page table, the physical frames that table was built out of,
//! and the register file, `sepc` and `sstatus` that were current when it last stopped.
//! Two processes are two of those. The boundary between them is `satp` and the `U` bit —
//! the same two mechanisms `tests/user.rs` already proves are load-bearing — and **not** a
//! convention, a wrapper type, or a check in Rust.
//!
//! The difference from a thread is mechanical and is what [`Kernel::switch`] does: a
//! thread would keep `satp` and swap registers, and this swaps both. `tests/process.rs`
//! pins that down the only way it can be pinned down — the isolation test is run with the
//! two processes sharing a `satp` and it fails.
//!
//! # Frames are the mechanism, and they are disjoint by construction
//!
//! `load_application` takes a frame pool and returns [`crate::loader::Loaded::free`], the
//! first frame it did **not** take. [`Kernel::spawn`] chains that: process 0 gets a pool
//! starting at [`crate::host::FREE`], process 1 gets one starting where process 0's ended.
//! So **no physical frame is ever reachable from two page tables**, and the isolation is
//! not a permission check that could be got round — the entry naming the other process's
//! memory is not in the table at all.
//!
//! The supervisor's identity gigapage *is* in both tables, and covers all of RAM including
//! both processes' frames. It carries no `U`. So the honest claim is precise: **a U-mode
//! process cannot read another U-mode process's memory**. S-mode can read everything,
//! which is true of every operating system and is how [`Kernel`] copies a message at all.
//!
//! # The channel copies, and shares nothing
//!
//! A [`Channel`] is a queue of messages held by the kernel. [`SEND`] reads the sender's
//! buffer *as the sender* and appends a copy; [`RECV`] pops one and writes it into the
//! receiver's buffer *as the receiver*. **No page is mapped into both spaces**, so the
//! channel cannot be turned into a window: what crosses is exactly the bytes the sender
//! named, and the receiver learns nothing about where they came from.
//!
//! That is a choice with a cost, and the cost is named in ADR-0021: shared memory between
//! two spaces — which is what `SharedArrayBuffer` is — is **not** what this builds, and
//! `F-005`'s workers cannot claim it from here.
//!
//! # The scheduler, and why the timer is the whole of it — `C-014a`
//!
//! `C-014` left this module with two address spaces and nothing that decided which one
//! ran: the test drove the switch by hand. [`Kernel::schedule`] decides, over a run queue
//! ([`Kernel::ready`]) that is plain round-robin — front takes the hart, and goes to the
//! back when its turn ends.
//!
//! **The turn ends by a supervisor timer interrupt, not by asking.** [`Kernel::switch`]
//! arms `Machine::timecmp` for `quantum` ticks ahead and opens `sie.STIE`; when the
//! deadline passes, the machine delivers cause 5 out of U-mode exactly as it delivers a
//! page fault, [`Kernel::run`] recognises it, saves the context and returns
//! [`Ended::Preempted`]. The process is not consulted and cannot decline — it has no
//! access to `sie`, `sstatus` or `sbi_set_timer`, all three of which are privileged and
//! trap as illegal from U-mode (`tests/user.rs`).
//!
//! **The negative half is the row again**, and it is the reason `budget` is never the
//! quantum. Every slice is handed the *whole remaining budget*, so nothing but the timer
//! can cut a slice short. A cooperative scheduler passes every test a preemptive one does
//! until something spins, so `tests/scheduler.rs` spins: a process whose only instructions
//! are a two-word loop with no `ecall` in it, run beside a process that would like a turn.
//! Under [`Kernel::preemptive`] the spinner is taken off the hart and the second process
//! runs and exits; under [`Kernel::new`] — the same programs, the same budget, one word of
//! difference — the spinner keeps the hart for the entire budget and the second process
//! never executes an instruction. That control is an assertion in the test, not a note.
//!
//! What this is *not* is doc 11 §3.4's scheduler: there are no priorities, no MLFQ, no
//! swappable policy and no blocking. `spec/schedule.sas` (`C-002i`) has all four on the
//! Sassembly side and says in its own text that it cannot preempt — *"इस नाभिक के पास
//! छीनने का कोई साधन नहीं — न घड़ी, न व्यवधान"*, this nucleus has no means of taking:
//! neither clock nor interrupt. The two halves are in different trees and neither is the
//! other; see ADR-0025.
//!
//! An endpoint is a handle, exactly as a [`Surface`] is, and it reaches the program the
//! only way anything does: through the vector the loader mapped read-only (A4). A program
//! cannot name an endpoint it was not granted, and the refusal for a number it was not
//! granted is `सीमातिक्रमः` — the same sentence, from the same table, that `F-001e2b`
//! already answers a forged surface with.

use crate::loader::{self, Loaded};
use crate::supervisor::{
    ADHIKARABHAVAH, ASAMARTHITAM, EXIT, Ended, RIKTAPRAVESHAH, SIDDHAM, SIMATIKRAMAH, Surface,
    WRITE, read_as_the_program, write_as_the_program,
};
use crate::{Halt, Machine, Output, Privilege};

/// `scause` 5 under [`crate::INTERRUPT`] — a supervisor timer interrupt, and the one trap
/// out of a process that is neither its request nor its fault.
///
/// The privileged spec's number, and the same one `spec/timer.sas` and
/// `tools/check-timer.sh` assert against real OpenSBI under QEMU. Named here rather than
/// written as a `5` because the `5` of an interrupt and the `5` of a
/// load-address-misaligned exception are different causes that differ only in the top bit.
const TIMER: u64 = 5;

/// Call 2 — प्रेषणम्, send. `a0` endpoint, `a1` buffer, `a2` length.
pub const SEND: u64 = 2;
/// Call 3 — ग्रहणम्, receive. `a0` endpoint, `a1` buffer, `a2` capacity.
pub const RECV: u64 = 3;

/// `रिक्तः` — the channel is empty. **Not an error and not `सिद्धम्` either**: a receiver
/// that got zero bytes has to be able to tell "nothing has been sent yet" from "a
/// zero-length message was sent", and one code for both would make an empty queue
/// indistinguishable from a message that carried nothing.
pub const RIKTAH: i64 = -5;

/// A queue of messages the kernel holds on behalf of two endpoints.
///
/// The bytes live **here**, in the kernel, and never in a page either process can reach.
/// That is the whole reason a channel does not weaken the boundary this module builds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Channel {
    /// Messages, oldest first. Order is the channel's one guarantee.
    pub messages: Vec<Vec<u8>>,
}

/// One process's end of a [`Channel`], and the number that process knows it by.
///
/// The two permissions are the *grant's*, not the channel's, which is what makes a
/// one-way channel expressible: the same channel granted to two processes may be
/// sendable by one and receivable by the other, and neither can do the other's half.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// The number the program passes in `a0`. The loader put it in the handle vector.
    pub handle: u64,
    /// Which channel in [`Kernel::channels`].
    pub channel: usize,
    /// Whether this grant carries प्रेषणम्.
    pub may_send: bool,
    /// Whether this grant carries ग्रहणम्.
    pub may_recv: bool,
}

/// An address space, the frames it was built from, and the context it stopped in.
///
/// **`satp` is the process.** Everything else here is bookkeeping around it: the register
/// file so the process can be left and resumed, and `frames` so a test can say which
/// physical memory belongs to whom without asking the page table twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    /// The Sv39 space. Two processes with the same value here are one process wearing two
    /// names, and `tests/process.rs` mutates exactly this to prove the point.
    pub satp: u64,
    /// `x0`..`x31` as of the last stop.
    pub x: [u64; 32],
    /// Where the program resumes.
    pub sepc: u64,
    /// `sstatus` as of the last stop — `SPP` clear, so the supervisor's `sret` returns to
    /// U-mode.
    pub sstatus: u64,
    /// The ELF's `e_entry`, read back from the image rather than assumed.
    pub entry: u64,
    /// The physical frames [`crate::loader::load_application`] took for this space:
    /// `[start, end)`. Disjoint from every other process's by construction — see the
    /// module header.
    pub frames: (u64, u64),
    /// Every surface granted to this process.
    pub surfaces: Vec<Surface>,
    /// Every channel endpoint granted to this process.
    pub endpoints: Vec<Endpoint>,
    /// Why it stopped, once it has. A process that exited is not runnable again, and
    /// [`Kernel::run`] says so rather than re-entering a program that asked to end.
    pub ended: Option<Ended>,
}

impl Process {
    /// The handle vector this process was loaded with: its surfaces, then its endpoints.
    ///
    /// One flat space of numbers, because the program sees one vector. Which kind of thing
    /// a handle names is the kernel's business and the program discovers it by calling —
    /// a `WRITE` to an endpoint is `सीमातिक्रमः`, exactly as an unknown number is.
    #[must_use]
    pub fn handles(&self) -> Vec<u64> {
        self.surfaces
            .iter()
            .map(|s| s.handle)
            .chain(self.endpoints.iter().map(|e| e.handle))
            .collect()
    }

    /// The surface known by `handle`, for a caller that wants to read what was written.
    #[must_use]
    pub fn surface(&self, handle: u64) -> Option<&Surface> {
        self.surfaces.iter().find(|s| s.handle == handle)
    }
}

/// One turn on the hart: which process took it, how long it lasted, and how it ended.
///
/// The scheduler's ledger entry, and the shape `spec/schedule.sas` settled on for the same
/// reason — *"हर चाल एक शब्द"*, one word per move. A scheduler's behaviour is not a state
/// you can look at when it has finished; it is the **sequence** of decisions it took, so
/// [`Kernel::schedule`] returns the sequence and the tests assert on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slice {
    /// Which process ran. Its index in [`Kernel::processes`].
    pub pid: usize,
    /// Ticks of [`Machine::time`] this turn consumed, from the switch to the stop.
    ///
    /// For a preempted slice this is exactly the quantum — see [`Kernel::preemptive`] for
    /// the arithmetic and which two of those ticks are the kernel's own.
    pub steps: u64,
    /// Why the turn ended. [`Ended::Preempted`] means the timer took the hart back and the
    /// process is still runnable; everything else except [`Ended::OutOfBudget`] means it is
    /// finished and off the queue.
    pub ended: Ended,
}

/// The kernel: every process, every channel, the run queue, and the watermark below which
/// frames are spoken for.
///
/// Since `C-014a` it **is** a scheduler, and a preemptive one when it is given a quantum:
/// [`Kernel::schedule`] takes processes off a run queue in turn, and a supervisor timer
/// interrupt ends a slice whether the process asked to stop or not. [`Kernel::run`] is
/// still the single-process entry point `C-014` wrote and still runs one named process
/// until it stops — a kernel built by [`Kernel::new`] arms no timer at all, which is the
/// cooperative behaviour and the control every preemption test is measured against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kernel {
    /// Every process, in the order it was spawned. The index is the pid.
    pub processes: Vec<Process>,
    /// Every channel. [`Endpoint::channel`] indexes this.
    pub channels: Vec<Channel>,
    /// The run queue: every **runnable** pid, in the order [`Kernel::schedule`] will reach
    /// them. Front is next. A preempted process goes to the back, which is the whole of the
    /// round-robin; a process that exited, faulted or stopped the machine is taken out and
    /// never put back.
    ready: Vec<usize>,
    /// Ticks a slice may last, or `None` for a kernel that does not preempt.
    ///
    /// `None` is [`Kernel::new`] and is exactly what `C-014` had: nothing arms the timer,
    /// so nothing can end a slice but the process itself or the caller's budget.
    quantum: Option<u64>,
    /// The first physical frame no process has been given. The next [`Kernel::spawn`]
    /// starts its pool here, which is what keeps the spaces disjoint.
    free: u64,
}

impl Kernel {
    /// A kernel whose frame pool begins at `free`, and which **does not preempt**.
    ///
    /// `free` must be above whatever the supervisor keeps for itself — its text, at
    /// least. [`crate::host::FREE`] is the value the rest of this crate uses.
    ///
    /// This is the cooperative kernel: no timer is armed, so a process that never yields
    /// keeps the hart until the caller's budget runs out. [`Kernel::preemptive`] is the
    /// one that takes it back.
    #[must_use]
    pub const fn new(free: u64) -> Self {
        Self {
            processes: Vec::new(),
            channels: Vec::new(),
            ready: Vec::new(),
            quantum: None,
            free,
        }
    }

    /// A kernel that takes the hart back after `quantum` ticks, whatever the process wants.
    ///
    /// **The quantum is counted in ticks of [`Machine::time`]**, which this interpreter
    /// advances once per instruction *begun* — so it is a count of instructions and
    /// emphatically not of seconds (see [`Machine::time`], and `.loop/ASSUMPTIONS.md` for
    /// why a wall clock was refused). The count starts the moment [`Kernel::switch`] puts
    /// the process on the hart, and **two of its ticks are the kernel's own**: the
    /// supervisor's `sret` on the way in, and the step on which the interrupt is taken
    /// instead of an instruction. So a quantum of `q` lets the process retire `q - 2` of
    /// its own instructions, and a preempted [`Slice::steps`] is exactly `q`.
    ///
    /// # Panics
    /// On a quantum below 3, which is a slice in which the process retires nothing. That is
    /// not a short slice, it is a scheduler that spins: the run queue would rotate forever
    /// and no program would ever advance a single instruction. Refusing it here is cheaper
    /// than finding it as a hang.
    #[must_use]
    pub fn preemptive(free: u64, quantum: u64) -> Self {
        assert!(
            quantum >= 3,
            "a quantum of {quantum} retires no instruction of the process — the two ticks \
             the kernel spends entering and interrupting are already more than that, and a \
             scheduler that hands out empty slices makes no progress at all"
        );
        Self {
            quantum: Some(quantum),
            ..Self::new(free)
        }
    }

    /// The run queue, front first: every process that is runnable and the order they are
    /// next reached in.
    ///
    /// A caller cannot push to it — membership is [`Kernel::spawn`]'s and
    /// [`Kernel::schedule`]'s to decide, because a queue holding a process that has exited
    /// is a scheduler that runs a dead program.
    #[must_use]
    pub fn ready(&self) -> &[usize] {
        &self.ready
    }

    /// Add a channel and return its index.
    pub fn channel(&mut self) -> usize {
        self.channels.push(Channel::default());
        self.channels.len() - 1
    }

    /// Build an address space for `image`, grant it `surfaces` and `endpoints`, and return
    /// its pid.
    ///
    /// The pool this space is built from begins where the last one ended, so no frame is
    /// ever in two page tables. The machine is left holding this process's state; the
    /// caller reaches it through [`Kernel::run`], which restores whichever process it is
    /// asked for and does not depend on which one was spawned last.
    ///
    /// # Errors
    /// The loader's, unchanged: a bad ELF, a segment that is not page-aligned, a program
    /// linked into the supervisor's gigabyte, or a pool that ran out. A refusal here is
    /// never a process that exists in part — nothing is pushed until the load succeeds.
    pub fn spawn(
        &mut self,
        m: &mut Machine,
        image: &[u8],
        surfaces: Vec<Surface>,
        endpoints: Vec<Endpoint>,
    ) -> Result<usize, String> {
        let start = self.free;
        let handles: Vec<u64> = surfaces
            .iter()
            .map(|s| s.handle)
            .chain(endpoints.iter().map(|e| e.handle))
            .collect();
        let Loaded {
            entry, satp, free, ..
        } = loader::load_application(m, image, start, &handles)?;
        self.free = free;
        self.processes.push(Process {
            satp,
            x: m.x,
            sepc: m.csr.sepc,
            sstatus: m.csr.sstatus,
            entry,
            frames: (start, free),
            surfaces,
            endpoints,
            ended: None,
        });
        // Runnable from the moment it exists, and at the back: a process that spawned last
        // does not get the hart before one that has been waiting.
        self.ready.push(self.processes.len() - 1);
        Ok(self.processes.len() - 1)
    }

    /// Make `pid` the resident process: its space, its registers, its `sepc`.
    ///
    /// **`satp` is written here and that is the whole switch.** A version of this function
    /// that restored the registers and left `satp` alone would be a thread switch, and the
    /// mutation in `tests/process.rs` is exactly that edit.
    ///
    /// The reservation is dropped. `lr`/`sc` is a property of one hart's view of one
    /// physical address, and carrying a reservation across a change of address space would
    /// let an `sc` in one process pair with an `lr` in another.
    ///
    /// **The timer is armed here, and it is the whole of the preemption** (`C-014a`). The
    /// deadline is set from the clock *now* rather than carried, so every process gets a
    /// full quantum and none inherits the remains of the last one's. A kernel with no
    /// quantum **disarms** instead of leaving the previous schedule's deadline standing:
    /// this function decides the timer completely, so a [`Kernel::run`] after a
    /// [`Kernel::schedule`] cannot be preempted by a deadline it never asked for.
    fn switch(&self, m: &mut Machine, pid: usize) {
        let p = &self.processes[pid];
        m.csr.satp = p.satp;
        m.x = p.x;
        m.csr.sepc = p.sepc;
        m.csr.sstatus = p.sstatus;
        m.reservation = None;
        match self.quantum {
            Some(q) => {
                m.csr.sie |= crate::SI_STI;
                m.timecmp = Some(m.time.saturating_add(q));
            }
            None => {
                m.csr.sie &= !crate::SI_STI;
                m.timecmp = None;
            }
        }
        // Standing on the supervisor's one `sret`, in supervisor mode — the invariant
        // `supervisor::install` establishes and every trap restores.
        m.mode = Privilege::Supervisor;
        m.pc = m.csr.stvec;
    }

    /// Save the resident process's context back into `pid`.
    ///
    /// **Where the process resumes is where the hart stood, and the CSR only knows that
    /// after a trap** (`W-220`). `sepc` is written by the machine when a trap is taken and
    /// by [`Kernel::service`] when a call is answered; a slice that ends in U-mode without
    /// a trap — the caller's budget running out between two instructions, or a process
    /// idling on its jump — has not moved it since the *last* trap, and saving the CSR then
    /// would resume the process at the last trap's successor and retire every instruction
    /// since it a second time. `tests/scheduler.rs` shows that as a counter one too high.
    /// In S-mode the hart stands on the supervisor's `sret` at `stvec`, and the CSR is the
    /// one thing that knows where the `sret` goes.
    fn save(&mut self, m: &Machine, pid: usize) {
        let p = &mut self.processes[pid];
        p.x = m.x;
        p.sepc = if m.mode == Privilege::User {
            m.pc
        } else {
            m.csr.sepc
        };
        p.sstatus = m.csr.sstatus;
    }

    /// Run `pid` until it exits, faults, stops the machine, or exhausts `budget` steps.
    ///
    /// The process's context is restored before the first step and saved after the last,
    /// so a caller may run one process, then the other, then the first again, and each
    /// resumes where it left off — on the instruction it stopped at, whether the stop was
    /// a trap or the budget (see [`Kernel::save`]).
    ///
    /// # A process cannot stop the machine, and its idle idiom is an idle — `W-219`
    ///
    /// `jal x0, .` is how every boot proof and both demo programs park when they are done,
    /// and on the bare machine the interpreter reports it as [`Halt::SpinForever`] rather
    /// than burning the budget to reach the same conclusion. **Under a kernel that word is
    /// a process's, and a process has no instruction that stops the machine** (ADR-0015
    /// A3; [`Ended::Stopped`] says so). So a self-jump executed in U-mode is not a halt
    /// here: the jump retired to itself, nothing changed but the clock, and the process
    /// goes on idling until the clock takes the slice (ADR-0025 — the slice ends on the
    /// clock, and a spin is not a request) or the caller's budget does. Under
    /// [`Kernel::preemptive`] a parked process is preempted every quantum and requeued like
    /// any spinner; under [`Kernel::new`] it keeps the hart to the budget, exactly as a
    /// two-word spinner does there. That is research/22 §7's ह-14 → ल: a program's end
    /// re-enters the scheduler and does not stop the world.
    ///
    /// **The refused case is the privilege, not the kernel.** The same word executed in
    /// S-mode — the supervisor's own text, were it ever a spin — is still the machine's to
    /// report and still [`Ended::Stopped`]; and [`Machine::run`], with no kernel above it,
    /// still halts on the first execution. `tests/scheduler.rs` holds both.
    ///
    /// `out` is the machine's UART, which an application has no way to reach (A4) and
    /// which is passed only because [`Machine::step`] takes one.
    pub fn run(
        &mut self,
        m: &mut Machine,
        pid: usize,
        budget: u64,
        out: &mut impl Output,
    ) -> Ended {
        if let Some(ended) = self.processes[pid].ended.clone() {
            return ended;
        }
        self.switch(m, pid);
        let mut result = Ended::OutOfBudget;
        for _ in 0..budget {
            let was_user = m.mode == Privilege::User;
            if let Some(halt) = m.step(out) {
                // The idle idiom, from U-mode: the process retired a jump to itself. The
                // machine left `pc` on the word and advanced the clock, so the next step
                // is the same jump — or the timer, taken before it, when the deadline
                // has come. Nothing about the machine has stopped.
                if was_user && matches!(halt, Halt::SpinForever { .. }) {
                    continue;
                }
                // Every other halt ends the run — including `Halt::Wait` (`W-370`),
                // which this kernel has no event source to answer: it propagates by
                // name in `Ended::Stopped`, never resumed here. See that variant.
                result = Ended::Stopped(halt);
                break;
            }
            if was_user && m.mode == Privilege::Supervisor {
                // The timer, and the one trap out of U-mode that is neither the program's
                // doing nor its fault. It must be tested before the `scause != 8` arm
                // below or preemption would be reported as a fault and kill the process
                // it was supposed to merely interrupt.
                if m.csr.scause == crate::INTERRUPT | TIMER {
                    result = Ended::Preempted;
                    break;
                }
                if m.csr.scause != 8 {
                    result = Ended::Faulted {
                        cause: m.csr.scause,
                        tval: m.csr.stval,
                        epc: m.csr.sepc,
                    };
                    break;
                }
                if let Some(ended) = self.service(m, pid) {
                    result = ended;
                    break;
                }
            }
        }
        self.save(m, pid);
        // A process that exited or faulted is finished. `OutOfBudget` is not: the caller
        // may hand it more steps, and the saved context is what makes that a resume.
        // `Preempted` is not either, and for a stronger reason — the process did not stop,
        // it was stopped, and a scheduler that marked it ended would have turned a time
        // slice into a death sentence.
        if !matches!(result, Ended::OutOfBudget | Ended::Preempted) {
            self.processes[pid].ended = Some(result.clone());
        }
        result
    }

    /// Run every runnable process in turn until the queue empties or `budget` runs out —
    /// task `C-014a`, doc 11 §11.2.9.
    ///
    /// Round-robin over [`Kernel::ready`]: the front process gets the hart, and when its
    /// slice ends it goes to the **back** if it is still runnable and off the queue if it
    /// is not. Returns the ledger — one [`Slice`] per turn, in order — because what a
    /// scheduler does is a sequence and not a state, and the only way to say "this one was
    /// taken off the hart and that one got it" is to show the order.
    ///
    /// **`budget` is the whole schedule's, and is never the slice's.** Each turn is handed
    /// every step that is left, so the only thing that can end a slice early is the
    /// process itself or the timer [`Kernel::switch`] armed. That is deliberate and it is
    /// what makes the preemption claim falsifiable: were the arming removed, a process
    /// that never yields would consume the entire budget in one slice and the next process
    /// would never run at all — which is exactly what a kernel from [`Kernel::new`] does,
    /// and exactly what `tests/scheduler.rs` asserts as its control.
    ///
    /// The schedule stops on [`Ended::Stopped`]: that variant is the *machine* halting, and
    /// there is no next instruction for anybody once the hart is done. Every other ending
    /// concerns one process and the rest keep their turns.
    pub fn schedule(&mut self, m: &mut Machine, budget: u64, out: &mut impl Output) -> Vec<Slice> {
        let mut ledger = Vec::new();
        let mut left = budget;
        while left > 0 {
            let Some(&pid) = self.ready.first() else {
                break;
            };
            let before = m.time;
            let ended = self.run(m, pid, left, out);
            let steps = m.time.wrapping_sub(before);
            left = left.saturating_sub(steps);
            match ended {
                // Still runnable, and to the back of the queue. Front would be a scheduler
                // that always returns to the process it just interrupted, which is a
                // preemption that changes nothing.
                Ended::Preempted => self.ready.rotate_left(1),
                // The caller's own limit, not this process's. It keeps its place at the
                // front so a later `schedule` with more budget resumes the turn.
                Ended::OutOfBudget => {}
                // Exited, faulted, or stopped the machine: off the queue and never back.
                _ => {
                    self.ready.remove(0);
                }
            }
            let last = matches!(ended, Ended::OutOfBudget | Ended::Stopped(_));
            ledger.push(Slice { pid, steps, ended });
            if last {
                break;
            }
        }
        ledger
    }

    /// Answer one call from `pid`. `Some` ends the process; `None` resumes it.
    ///
    /// `a0` and `a1` are the only registers written — `spec/application-abi.tsv`'s rule,
    /// and the one a caller can rely on.
    fn service(&mut self, m: &mut Machine, pid: usize) -> Option<Ended> {
        match m.x[17] {
            EXIT => return Some(Ended::Exited { status: m.x[10] }),
            WRITE => {
                let (code, written) = self.write(m, pid);
                m.x[10] = code as u64;
                m.x[11] = written;
            }
            SEND => {
                let (code, sent) = self.send(m, pid);
                m.x[10] = code as u64;
                m.x[11] = sent;
            }
            RECV => {
                let (code, got) = self.recv(m, pid);
                m.x[10] = code as u64;
                m.x[11] = got;
            }
            _ => m.x[10] = ASAMARTHITAM as u64,
        }
        // Past the `ecall`, or the `sret` below returns to it and the program calls twice.
        m.csr.sepc = m.csr.sepc.wrapping_add(4);
        None
    }

    /// Call 1 — आलेखनम्, unchanged from [`crate::supervisor`] except that the surfaces are
    /// the calling process's rather than a single global list.
    fn write(&mut self, m: &Machine, pid: usize) -> (i64, u64) {
        let (handle, buffer, length) = (m.x[10], m.x[11], m.x[12]);
        let p = &mut self.processes[pid];
        let Some(index) = p.surfaces.iter().position(|s| s.handle == handle) else {
            return (SIMATIKRAMAH, 0);
        };
        if !p.surfaces[index].writable {
            return (ADHIKARABHAVAH, 0);
        }
        let Some(bytes) = read_as_the_program(m, buffer, length) else {
            return (RIKTAPRAVESHAH, 0);
        };
        let written = bytes.len() as u64;
        p.surfaces[index].bytes.extend_from_slice(&bytes);
        (SIDDHAM, written)
    }

    /// Call 2 — प्रेषणम्. Returns `a0` and `a1`.
    ///
    /// The buffer is read **as the sending program**, which is what stops a process from
    /// having the kernel copy the kernel: `read_as_the_program` requires `U` at the leaf
    /// and never consults `sstatus.SUM`. A pointer into the supervisor's gigapage is
    /// `रिक्तप्रवेशः`, exactly as it is for a write to a surface.
    fn send(&mut self, m: &Machine, pid: usize) -> (i64, u64) {
        let (handle, buffer, length) = (m.x[10], m.x[11], m.x[12]);
        let Some(e) = self.processes[pid]
            .endpoints
            .iter()
            .find(|e| e.handle == handle)
        else {
            return (SIMATIKRAMAH, 0);
        };
        if !e.may_send {
            return (ADHIKARABHAVAH, 0);
        }
        let channel = e.channel;
        let Some(bytes) = read_as_the_program(m, buffer, length) else {
            return (RIKTAPRAVESHAH, 0);
        };
        let sent = bytes.len() as u64;
        self.channels[channel].messages.push(bytes);
        (SIDDHAM, sent)
    }

    /// Call 3 — ग्रहणम्. Returns `a0` and `a1`.
    ///
    /// **All of the message or none of it.** A message longer than the capacity offered is
    /// `सीमातिक्रमः` with the message still queued, and a buffer the program may not write
    /// is `रिक्तप्रवेशः` with the message still queued — because a receive that consumed a
    /// message it could not deliver would lose it, and the program has no way to ask for
    /// it again.
    fn recv(&mut self, m: &mut Machine, pid: usize) -> (i64, u64) {
        let (handle, buffer, capacity) = (m.x[10], m.x[11], m.x[12]);
        let Some(e) = self.processes[pid]
            .endpoints
            .iter()
            .find(|e| e.handle == handle)
        else {
            return (SIMATIKRAMAH, 0);
        };
        if !e.may_recv {
            return (ADHIKARABHAVAH, 0);
        }
        let channel = e.channel;
        if self.channels[channel].messages.is_empty() {
            return (RIKTAH, 0);
        }
        if self.channels[channel].messages[0].len() as u64 > capacity {
            return (SIMATIKRAMAH, 0);
        }
        let message = self.channels[channel].messages[0].clone();
        if !write_as_the_program(m, buffer, &message) {
            return (RIKTAPRAVESHAH, 0);
        }
        self.channels[channel].messages.remove(0);
        (SIDDHAM, message.len() as u64)
    }
}
