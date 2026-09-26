//! The host — one call that loads an application and runs it, task `F-001f`.
//!
//! # Why this exists, and why it is here rather than in the browser crate
//!
//! [`crate::loader`] builds an address space and [`crate::supervisor`] answers the two
//! calls, but neither of them starts anything: between them sits a dozen lines of
//! arrangement — a machine of the right size, the supervisor's one word written somewhere
//! inside its own RAM, `stvec` pointed at it, a frame pool that begins above it, and a
//! grant to hand the loader. Until this module, the only place those lines existed was
//! `tests/application.rs`, which means the only thing in this tree that could host an
//! application was a test.
//!
//! `F-001`'s sentence is "apps that run in the browser **and outside it**". The browser
//! half needs exactly that arrangement, and a second copy of it written in
//! `crates/yantra-wasm` would be a second set of answers to *what a Sassembly machine
//! looks like* — one of them untested, because that crate is excluded from the workspace
//! and no gate step builds it. So the arrangement lives here, in the crate the gate
//! compiles and tests, and the browser ABI becomes what it claims to be: thin exports
//! over an interpreter, with no policy of its own. `F-001g` spent it — `yantra_host` is
//! one call into this module.
//!
//! # What this is not
//!
//! It is not an operating system and it does not pretend to schedule anything. One
//! program, one surface, one run, and a [`Hosted`] describing what happened. A second
//! program is a second call — which is a weaker statement of A5 than
//! [`Supervisor::run`]'s, where the *same* machine hosts two applications in succession,
//! and that stronger claim stays where it is measured, in `tests/supervisor.rs`.
//!
//! # The numbers, and why each one is chosen rather than typed
//!
//! [`BASE`] is where QEMU `virt` puts RAM, so a hosted machine has the same map as a
//! booted one. [`FREE`] leaves the first 64 KiB to the supervisor — its text is one word
//! today and the pool must not start on top of it. [`SCREEN`] is the handle number the
//! grant carries, and the fact that it is arbitrary is the point: the program cannot know
//! it and must read it out of the vector the loader mapped (A4).

use crate::loader::{self, Loaded};
use crate::supervisor::{self, Ended, Supervisor, Surface};
use crate::{Csrs, Machine, Privilege};

/// Where RAM begins — QEMU `virt`'s address, so that a hosted machine and a booted one
/// have the same memory map.
pub const BASE: u64 = 0x8000_0000;

/// The first physical address the loader may take a frame from. Everything below it is
/// the supervisor's: its text, and whatever a real one would keep beside it.
pub const FREE: u64 = BASE + 0x1_0000;

/// The handle the grant carries. Its value is the supervisor's to choose and the program
/// is never told it — that is what makes reading the handle vector the only way to obtain
/// a surface (A4).
pub const SCREEN: u64 = 7;

/// What happened to one hosted application.
///
/// Every field is evidence somebody can check rather than a summary somebody must trust:
/// the bytes that reached the surface, the bytes that reached the machine's own UART
/// (which an application has no way to write, so a non-empty one is a finding), the
/// `scause` of the last trap — 8 is an environment call from U-mode and S-mode cannot
/// raise it — and the entry the loader actually jumped to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hosted {
    /// Why the program stopped running. [`Ended::Stopped`] is the machine stopping, which
    /// nothing reachable through this ABI produces.
    pub ended: Ended,
    /// What the program put on the surface it was granted, in order.
    pub surface: Vec<u8>,
    /// What reached the machine's UART. An application cannot address a device, so this
    /// is empty for every program that is one.
    pub uart: Vec<u8>,
    /// `scause` after the run — the cause of the last trap the machine took.
    pub scause: u64,
    /// The entry point the loader entered, read back from the image rather than assumed.
    pub entry: u64,
}

/// Load `image` into a fresh machine with `ram` bytes and run it for at most `budget`
/// steps, granting it one writable surface known by [`SCREEN`].
///
/// The machine is entered with `sstatus.SPP` **set**, which is not decoration: a loader
/// that forgot to clear it would `sret` into S-mode, the `ecall` would raise 9 instead of
/// 8, and [`Hosted::scause`] would say so instead of the run passing.
///
/// # Errors
/// A string, whenever the load cannot honestly be completed: a file that is not a 64-bit
/// little-endian RISC-V ELF, a segment that is not page-aligned, a program linked into
/// the supervisor's own gigabyte, or a frame pool too small for the address space this
/// image needs. A refusal here is never a run that happened differently — nothing has
/// executed yet.
pub fn host(image: &[u8], ram: usize, budget: u64) -> Result<Hosted, String> {
    let mut m = Machine {
        x: [0; 32],
        pc: 0,
        base: BASE,
        mem: vec![0; ram],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    m.csr.sstatus = 1 << 8;
    supervisor::install(&mut m, BASE)?;

    let mut sup = Supervisor::new(vec![Surface::writable(SCREEN)]);
    let Loaded { entry, .. } = loader::load_application(&mut m, image, FREE, &sup.handles())?;

    let mut uart: Vec<u8> = Vec::new();
    let ended = sup.run(&mut m, budget, &mut uart);
    Ok(Hosted {
        ended,
        surface: sup
            .surface(SCREEN)
            .expect("the surface this function granted")
            .bytes
            .clone(),
        uart,
        scause: m.csr.scause,
        entry,
    })
}

/// One [`Ended`] in words, for a reader who is not holding the ABI table.
///
/// Every variant names the number it is carrying, because "it faulted" without a `scause`
/// sends the reader to the wrong instruction. The wording is the vocabulary a page or a
/// terminal shows a person; the machine-readable form is the [`Ended`] itself, and
/// nothing should parse these strings.
#[must_use]
pub fn describe(ended: &Ended) -> String {
    match ended {
        Ended::Exited { status: 0 } => {
            "the program asked to be ended and its status is 0 — सिद्धम्, call 0 with the \
             verdict call 1 returned. The machine is untouched and still runnable."
                .into()
        }
        Ended::Exited { status } => format!(
            "the program asked to be ended with status {status}. That is the program's \
             verdict on itself, not the machine's — nothing here failed."
        ),
        Ended::Faulted { cause, tval, epc } => format!(
            "the instruction at {epc:#x} trapped out of U-mode with scause {cause} \
             (13 is a load page fault, 15 a store, 12 an instruction, 2 an instruction \
             the mode may not execute), stval {tval:#x}. The supervisor has no policy for \
             it, so it ended the program rather than resuming into the same fault forever."
        ),
        Ended::Stopped(halt) => format!(
            "the MACHINE stopped, not the program: {halt:?}. Nothing an application can \
             reach through this ABI produces this — an application that gets here called \
             the firmware (A3) or touched a device (A4)."
        ),
        Ended::OutOfBudget => "the step budget ran out and the program neither exited nor \
                               faulted. Bounded on purpose: a host must not be wedged by a \
                               program that never ends."
            .into(),
        Ended::Preempted => "the quantum expired and a supervisor timer interrupt took the \
                             hart back. The program did not ask to stop, is not finished, \
                             and its context is saved: the scheduler will run it again."
            .into(),
    }
}
