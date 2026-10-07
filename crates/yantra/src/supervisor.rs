//! The supervisor that answers the two calls — ADR-0015 clauses A4 and A5, task
//! `F-001e2b`.
//!
//! # This is not part of the machine either
//!
//! [`crate::loader`] says it: a loader is supervisor software, and it is in this crate
//! because the supervisor that will own it does not exist yet. This module is the other
//! half of that same absence — the thing on the far side of a U-mode `ecall`. Everything
//! it touches is ordinary machine state: `sepc`, `a0`, `a1`, and bytes it reads through the
//! program's own page table. A Sassembly supervisor replaces it without a line of the
//! interpreter changing, and `F-001e1` established the precondition that makes that
//! possible — **every number in `spec/application-abi.tsv`, from U-mode, is exception 8
//! delivered to `stvec`, and none of them reaches the firmware.**
//!
//! # The supervisor's entire text is one instruction
//!
//! [`install`] writes a single `sret` at `stvec` and points `pc` at it. That word is the
//! whole of this supervisor's machine code, and it does two jobs because they are the same
//! job: executed once at the start it *enters* the application (the instruction the loader
//! deliberately stopped short of), and executed after a trap it *resumes* it. Everything
//! between those two executions — decoding `a7`, reading the buffer, choosing the error —
//! happens in Rust while the hart stands on that word.
//!
//! **The mode change stays the machine's**, exactly as in the loader: nothing here assigns
//! `mode = User`. [`Supervisor::run`] services the call, sets `sepc` past the `ecall`, and
//! then steps the interpreter over the `sret`, which is what returns the privilege.
//!
//! # A5: the exit ends the program, not the machine
//!
//! Call 0 is the one call that does not return, and [`Supervisor::run`] answers it by
//! *returning* — [`Ended::Exited`] with the status the program put in `a0`. It writes no
//! finisher, raises no [`Halt`], and leaves the hart standing in S-mode on its `sret` with
//! RAM intact. The acceptance is that the caller can then do anything at all with that
//! machine, and `tests/supervisor.rs` does the sharpest available thing: it loads a
//! *second* application into the same [`Machine`] and runs that one to its own exit. "The
//! environment survives to say so" is not a sentence there; it is a second program.
//!
//! # A4: what "a surface it was handed" means here
//!
//! A [`Surface`] is a handle number, a permission and a `Vec<u8>`. That vector is not a
//! placeholder for a real surface — it is the same `Vec<u8>` that implements
//! [`crate::Output`] for the UART in the tests, which is what ADR-0015 says the browser's
//! surface is. What is missing is a compositor, not an interface.
//!
//! **The program never names a surface it was not handed.** The only handle numbers it can
//! know are the ones the loader put in its handle vector, and the supervisor answers a
//! number it did not grant with `सीमातिक्रमः` rather than with the surface that happens to
//! sit at that index of its own list.
//!
//! # The buffer is read as the program, not as the supervisor
//!
//! `रिक्तप्रवेशः` is defined as "the buffer is not readable **by this program**", and that
//! wording is load-bearing. The obvious implementation — set `sstatus.SUM`, then read
//! through `Machine::translate` — asks a different question: *may S-mode read this?* With
//! `SUM` set the answer is yes for the supervisor's own gigapage, which is mapped into
//! every application's address space (the trap has to be able to fetch its handler) without
//! `U`. A program that passed a pointer into it would have the supervisor copy the
//! supervisor's memory onto a surface for it, one `ecall` at a time.
//!
//! So `readable_by_the_program` walks the program's page table itself and requires `U`
//! at the leaf. `SUM` is never set here, and the `U` bit — which is the entire protection
//! (`tests/user.rs`) — decides the buffer exactly as it decides a load the program issues
//! for itself.
//!
//! # असमर्थितम् is on call 1's row, and this supervisor cannot produce it there
//!
//! `spec/application-abi.tsv` lists all four failures against call 1, but the table is the
//! ABI and not this file: a supervisor that implements a subset of the table answers
//! `असमर्थितम्` for a number in it, and that is the same arm as a number the table never
//! assigned. This one implements both calls, so the only way to see `असमर्थितम्` here is a
//! number outside the table — 8, say, which is `sbi_shutdown`, and which this machine
//! obeys from S-mode and refuses in a sentence from U-mode.

use crate::{
    Halt, Machine, Output, PPN_MASK, PTE_A, PTE_D, PTE_R, PTE_U, PTE_V, PTE_W, PTE_X, Privilege,
    SATP_SV39, loader,
};

/// `scause` 8 — an environment call from U-mode. The only cause this supervisor has a
/// policy for; every other trap from the program ends it (see [`Ended::Faulted`]).
const U_ECALL: u64 = 8;

/// `sret`, and the whole of this supervisor's machine code.
const SRET: u32 = 0x1020_0073;

/// Call 0 — समापनम्, exit. `does-not-return`.
pub const EXIT: u64 = 0;
/// Call 1 — आलेखनम्, write.
pub const WRITE: u64 = 1;

/// `सिद्धम्` — the call succeeded.
pub const SIDDHAM: i64 = 0;
/// `असमर्थितम्` — unsupported: no such call number.
pub const ASAMARTHITAM: i64 = -1;
/// `अधिकाराभावः` — capability denied: the handle does not permit this.
pub const ADHIKARABHAVAH: i64 = -2;
/// `रिक्तप्रवेशः` — null access: the buffer is not readable by this program.
pub const RIKTAPRAVESHAH: i64 = -3;
/// `सीमातिक्रमः` — index out of bounds: no such handle.
pub const SIMATIKRAMAH: i64 = -4;

/// A surface the supervisor may be asked to write to, and the number the program knows it
/// by.
///
/// `writable` is the *grant's* permission and not a property of the surface: the same
/// surface handed to two programs may be writable by one of them. A write to a handle
/// granted without it is `अधिकाराभावः`, which is a different sentence from "no such
/// handle" and has to stay one — a program that could tell them apart by trying could map
/// the supervisor's grant table by brute force.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Surface {
    /// The number the program passes in `a0`. The loader put it in the handle vector.
    pub handle: u64,
    /// Whether this grant carries आलेखनम्.
    pub writable: bool,
    /// Everything written to it, in order.
    pub bytes: Vec<u8>,
}

impl Surface {
    /// A writable surface known by `handle`.
    #[must_use]
    pub const fn writable(handle: u64) -> Self {
        Self {
            handle,
            writable: true,
            bytes: Vec::new(),
        }
    }

    /// A surface granted *without* आलेखनम् — the thing `अधिकाराभावः` is about.
    #[must_use]
    pub const fn read_only(handle: u64) -> Self {
        Self {
            handle,
            writable: false,
            bytes: Vec::new(),
        }
    }
}

/// Why the program stopped running. Never why the *machine* stopped — [`Ended::Stopped`]
/// is the one variant that says that, and it names the [`Halt`] rather than absorbing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    /// Call 0. `status` is the program's `a0`. The machine is untouched and still runnable
    /// — that is A5, and it is the whole reason this is a return value and not a `Halt`.
    Exited {
        /// What the program asked to be recorded as.
        status: u64,
    },
    /// A trap from U-mode that was not an `ecall`: a page fault, an illegal instruction, a
    /// jump to 0. The supervisor has no policy for it, so it ends the program and says
    /// which one it was rather than resuming into the same fault forever.
    Faulted {
        /// `scause`.
        cause: u64,
        /// `stval`.
        tval: u64,
        /// `sepc` — the instruction that trapped.
        epc: u64,
    },
    /// The *machine* stopped: a finisher write, a spin in S-mode, an instruction this
    /// interpreter does not implement, an SBI call from S-mode. Nothing an application
    /// can reach through this ABI produces this, which is the point of A3 — and since
    /// `W-219` a U-mode `jal x0, .` under [`crate::process::Kernel`] does not either: it
    /// is the process idling, and the clock takes the slice. [`Supervisor::run`], which
    /// has no clock and hosts one program, still reports that spin as the machine's halt.
    ///
    /// The one thing a U-mode program can still reach here is an instruction this
    /// interpreter lacks — `spec/fdt-header.sas` reaches an RV64M `divu` under the kernel
    /// (`W-212`'s census). That is the host's gap and not the program's request, and it is
    /// reported as the machine's rather than dressed as the program's fault.
    ///
    /// **A [`Halt::Wait`] comes out here too, and is NOT resumed** (`W-370`). Only S-mode
    /// can reach the `WAIT` address — an application's tables do not map a device — and
    /// neither this supervisor nor the kernel has an event source to deliver from, so
    /// resuming would invent one. The machine's `pc` is already past the store, so a host
    /// that does have a world to deliver can call [`Machine::run`] itself.
    Stopped(Halt),
    /// `budget` steps went by and the program neither exited nor faulted.
    OutOfBudget,
    /// **The quantum expired and the hart was taken back.** `C-014a`.
    ///
    /// Like [`Ended::OutOfBudget`] and unlike every other variant, this is not a program
    /// that stopped: its context is saved, it is still runnable, and
    /// [`crate::process::Kernel::schedule`] puts it back on the run queue. The difference
    /// between the two is *who* ended the slice — `OutOfBudget` is the caller's own limit
    /// running out, and this is a **supervisor timer interrupt** the process did not ask
    /// for and could not decline. That distinction is the whole of `C-014a`: a
    /// cooperative scheduler can produce `OutOfBudget` and can never produce this.
    ///
    /// [`Supervisor::run`] never returns it — it arms no timer.
    Preempted,
}

/// The supervisor: the grants it made, and the two calls it answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Supervisor {
    /// Every surface granted to the program now running.
    pub surfaces: Vec<Surface>,
}

impl Supervisor {
    /// A supervisor that has granted `surfaces`.
    #[must_use]
    pub const fn new(surfaces: Vec<Surface>) -> Self {
        Self { surfaces }
    }

    /// The handle vector to hand [`loader::load_application`] — the numbers, in order,
    /// and nothing else. What the program receives is the grant; the surfaces stay here.
    #[must_use]
    pub fn handles(&self) -> Vec<u64> {
        self.surfaces.iter().map(|s| s.handle).collect()
    }

    /// The surface known by `handle`, for a caller that wants to read what was written.
    #[must_use]
    pub fn surface(&self, handle: u64) -> Option<&Surface> {
        self.surfaces.iter().find(|s| s.handle == handle)
    }

    /// Run the application until it exits, faults, stops the machine or exhausts `budget`
    /// steps.
    ///
    /// The machine must be standing on the `sret` [`install`] wrote, with the loader's
    /// state already in place. `out` is the machine's UART, which an application has no
    /// way to reach (A4) and which is passed only because [`Machine::step`] takes one.
    pub fn run(&mut self, m: &mut Machine, budget: u64, out: &mut impl Output) -> Ended {
        for _ in 0..budget {
            let was_user = m.mode == Privilege::User;
            if let Some(halt) = m.step(out) {
                return Ended::Stopped(halt);
            }
            // A trap out of U-mode, and the only kind of trap this supervisor answers.
            // The step that took it is the `ecall`; the step after this one is the `sret`
            // at `stvec`, which is where the hart is standing right now.
            if was_user && m.mode == Privilege::Supervisor {
                if m.csr.scause != U_ECALL {
                    return Ended::Faulted {
                        cause: m.csr.scause,
                        tval: m.csr.stval,
                        epc: m.csr.sepc,
                    };
                }
                if let Some(ended) = self.service(m) {
                    return ended;
                }
            }
        }
        Ended::OutOfBudget
    }

    /// Answer one call. `Some` ends the program; `None` resumes it.
    ///
    /// `a0` and `a1` are the only registers written, which is `spec/application-abi.tsv`'s
    /// own rule and the one a caller can rely on: a supervisor that clobbered `s2` would
    /// be a supervisor no program could call twice.
    fn service(&mut self, m: &mut Machine) -> Option<Ended> {
        match m.x[17] {
            EXIT => return Some(Ended::Exited { status: m.x[10] }),
            WRITE => {
                let (code, written) = self.write(m);
                m.x[10] = code as u64;
                m.x[11] = written;
            }
            _ => m.x[10] = ASAMARTHITAM as u64,
        }
        // Past the `ecall`. The `sret` that follows takes the program back to it otherwise,
        // and a program that called `exit` twice would be the least of it.
        m.csr.sepc = m.csr.sepc.wrapping_add(4);
        None
    }

    /// Call 1 — आलेखनम्. Returns `a0` and `a1`.
    ///
    /// All of the bytes or none of them: the span is translated in full before the first
    /// one is copied, so a buffer that runs off the end of its last mapped page is
    /// `रिक्तप्रवेशः` with nothing written, rather than a partial surface and an error the
    /// program cannot act on.
    fn write(&mut self, m: &Machine) -> (i64, u64) {
        let (handle, buffer, length) = (m.x[10], m.x[11], m.x[12]);
        let Some(index) = self.surfaces.iter().position(|s| s.handle == handle) else {
            return (SIMATIKRAMAH, 0);
        };
        if !self.surfaces[index].writable {
            return (ADHIKARABHAVAH, 0);
        }
        let Some(bytes) = read_as_the_program(m, buffer, length) else {
            return (RIKTAPRAVESHAH, 0);
        };
        let written = bytes.len() as u64;
        self.surfaces[index].bytes.extend_from_slice(&bytes);
        (SIDDHAM, written)
    }
}

/// Write the supervisor's one instruction at `at`, install it as `stvec`, and stand on it.
///
/// `at` is a physical address in the supervisor's own RAM, which the loader identity-maps
/// without `U` into every application's space — so this word is fetchable from S-mode after
/// a trap and invisible to the program on either side of it.
///
/// # Errors
/// A string, when `at` is not inside this machine's RAM.
pub fn install(m: &mut Machine, at: u64) -> Result<(), String> {
    loader::put32(m, at, SRET)?;
    m.csr.stvec = at;
    m.pc = at;
    Ok(())
}

/// Read `length` bytes at `buffer` **as the program would**, or `None`.
///
/// One walk per page, because the pages of a buffer need not be physically adjacent and
/// reading past the end of the first frame is a wrong answer in somebody else's memory
/// rather than a refusal.
///
/// `pub(crate)` since `C-014`: [`crate::process`]'s channel reads a sender's buffer with
/// exactly this function and for exactly this reason. A second copy of it there would be a
/// second answer to *may this program read this pointer*, and the two could drift apart on
/// the one case that matters — a pointer into the supervisor's own gigapage.
pub(crate) fn read_as_the_program(m: &Machine, buffer: u64, length: u64) -> Option<Vec<u8>> {
    // A zero-length write touches no memory, so it cannot fail on a pointer. It is a
    // success that wrote nothing, which is what the program asked for.
    if length == 0 {
        return Some(Vec::new());
    }
    buffer.checked_add(length - 1)?;
    let mut bytes = Vec::with_capacity(length as usize);
    let mut va = buffer;
    while bytes.len() < length as usize {
        let pa = readable_by_the_program(m, va)?;
        let rest = (length as usize) - bytes.len();
        let take = rest.min((0x1000 - (va & 0xfff)) as usize);
        let at = loader::phys(m, pa, take).ok()?;
        bytes.extend_from_slice(&m.mem[at..at + take]);
        va += take as u64;
    }
    Some(bytes)
}

/// Write `bytes` at `buffer` **as the program would**, or `false` if it may not.
///
/// The mirror of [`read_as_the_program`] and it refuses on the same principle: the leaf
/// must carry `U`, and `sstatus.SUM` is never consulted, because SUM answers whether the
/// *supervisor* may write a user page and the supervisor is not who is writing. A channel
/// that wrote with SUM set would let a receiver name a pointer into the supervisor's
/// gigapage and have the kernel scribble a sender's bytes into its own text.
///
/// **All of the bytes or none of them.** Every page of the span is translated before the
/// first octet is copied, so a buffer that runs off the end of its last writable page
/// leaves the target untouched — which is what lets [`crate::process::Kernel`] put a
/// message back on the queue rather than lose half of it.
pub(crate) fn write_as_the_program(m: &mut Machine, buffer: u64, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    if buffer.checked_add(bytes.len() as u64 - 1).is_none() {
        return false;
    }
    // First pass: every page, translated and permitted, and the offsets recorded. Nothing
    // is written here.
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut va = buffer;
    let mut done = 0usize;
    while done < bytes.len() {
        let Some(pa) = writable_by_the_program(m, va) else {
            return false;
        };
        let take = (bytes.len() - done).min((0x1000 - (va & 0xfff)) as usize);
        let Ok(at) = loader::phys(m, pa, take) else {
            return false;
        };
        spans.push((at, take));
        va += take as u64;
        done += take;
    }
    // Second pass: the copy, which can no longer fail.
    let mut done = 0usize;
    for (at, take) in spans {
        m.mem[at..at + take].copy_from_slice(&bytes[done..done + take]);
        done += take;
    }
    true
}

/// Translate `va` for a store **by the U-mode program**, or `None` if it may not write it.
///
/// [`readable_by_the_program`] with `W` and `D` in place of `R`. `D` is required for the
/// same reason [`crate::Machine::translate`] requires it of a store: this machine faults
/// rather than filling the dirty bit, so a page without it is one the program itself could
/// not store to, and the kernel must not do on its behalf what it could not do for itself.
fn writable_by_the_program(m: &Machine, va: u64) -> Option<u64> {
    if m.csr.satp & SATP_SV39 != SATP_SV39 {
        return None;
    }
    if ((va << 25) as i64 >> 25) as u64 != va {
        return None;
    }
    let mut table = (m.csr.satp & PPN_MASK) << 12;
    for level in (0..3).rev() {
        let pte = loader::get(m, table + ((va >> (12 + 9 * level)) & 0x1ff) * 8).ok()?;
        if pte & PTE_V == 0 {
            return None;
        }
        let ppn = (pte >> 10) & PPN_MASK;
        if pte & (PTE_R | PTE_X) == 0 {
            if level == 0 {
                return None;
            }
            table = ppn << 12;
            continue;
        }
        if pte & PTE_W == 0 || pte & PTE_U == 0 || pte & PTE_A == 0 || pte & PTE_D == 0 {
            return None;
        }
        let within = (1u64 << (9 * level)) - 1;
        if ppn & within != 0 {
            return None;
        }
        return Some(((ppn & !within) | ((va >> 12) & within)) << 12 | (va & 0xfff));
    }
    unreachable!("the level-0 arm returns or refuses, so the loop always leaves early")
}

/// Translate `va` for a load **by the U-mode program**, or `None` if it may not read it.
///
/// This is `Machine::translate` with one question changed and no `sstatus` involved: the
/// leaf must carry `U`, `R`, `V` and `A`, and `sstatus.SUM` is not consulted because SUM
/// answers whether the *supervisor* may read a user page, and the supervisor is not who is
/// reading. See this module's header for what SUM would have let through.
///
/// A machine translating in Bare mode gets `None` rather than the identity: with no page
/// table there is no such thing as memory that is the program's and not the supervisor's,
/// so there is no honest way to answer, and inventing one hands out the supervisor.
fn readable_by_the_program(m: &Machine, va: u64) -> Option<u64> {
    if m.csr.satp & SATP_SV39 != SATP_SV39 {
        return None;
    }
    // Sv39 is 39 bits sign-extended. Truncating instead would map the hole in the middle
    // of the address space onto real memory — the same check `translate` makes, for the
    // same reason.
    if ((va << 25) as i64 >> 25) as u64 != va {
        return None;
    }
    let mut table = (m.csr.satp & PPN_MASK) << 12;
    for level in (0..3).rev() {
        let pte = loader::get(m, table + ((va >> (12 + 9 * level)) & 0x1ff) * 8).ok()?;
        if pte & PTE_V == 0 {
            return None;
        }
        let ppn = (pte >> 10) & PPN_MASK;
        if pte & (PTE_R | PTE_X) == 0 {
            if level == 0 {
                return None;
            }
            table = ppn << 12;
            continue;
        }
        if pte & PTE_R == 0 || pte & PTE_U == 0 || pte & PTE_A == 0 {
            return None;
        }
        let within = (1u64 << (9 * level)) - 1;
        if ppn & within != 0 {
            return None;
        }
        return Some(((ppn & !within) | ((va >> 12) & within)) << 12 | (va & 0xfff));
    }
    unreachable!("the level-0 arm returns or refuses, so the loop always leaves early")
}
