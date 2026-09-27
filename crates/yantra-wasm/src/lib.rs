//! The browser ABI — a handful of functions, no imports, and no `unsafe`.
//!
//! # Why there are no imports
//!
//! The obvious design gives the module an imported `uart_out(byte)` and streams
//! characters to the page as they are written. Declaring and calling an imported function
//! is `unsafe`, and this crate is `forbid(unsafe_code)`.
//!
//! So the UART bytes are collected into a buffer instead, and JS reads it after the run.
//! The programs finish in microseconds, so nothing is lost by not streaming — and what is
//! gained is that **the entire browser path is safe Rust**, which is a better answer to
//! doc 03 §1 than "no third-party crates, but a pile of `unsafe` glue". The rule pushed
//! the design somewhere better rather than merely costing effort.
//!
//! # The whole protocol
//!
//! ```text
//! wasm.yantra_alloc(n)  -> offset      JS writes the ELF there
//! wasm.yantra_run(n, budget) -> code   boots it: reset vector, S-mode, no loader
//! wasm.yantra_host(n, budget) -> code  hosts it as an APPLICATION: loader, supervisor
//! wasm.yantra_out_ptr()  -> offset     the MACHINE'S UART. JS reads len bytes and
//! wasm.yantra_out_len()  -> len        decodes UTF-8
//! wasm.yantra_surface_ptr() -> offset  the SURFACE the application was granted —
//! wasm.yantra_surface_len() -> len     a DIFFERENT buffer, see below
//! wasm.yantra_halt_ptr()  -> offset    why it stopped, in words
//! wasm.yantra_halt_len()  -> len
//! ```
//!
//! Offsets are indices into the module's exported linear memory, which is the only thing
//! JS and Rust share. No `wasm-bindgen`, no generated bindings, no build script.
//!
//! # Why the surface is a second buffer and not more bytes in the first
//!
//! A boot proof writes to `0x10000000`, which is a UART: the machine's own device, and on
//! this page the only thing there is to see. An application cannot address a device at all
//! (ADR-0015 A4) — it writes to a surface it was *handed*, through a call the supervisor
//! answers, and the machine's UART stays empty for the whole run.
//!
//! Merging the two into one `out` buffer would cost nothing to implement and would tell
//! the page's reader the opposite of what A4 guarantees: that a program's output is a
//! program's output, whichever way it got there. They are two buffers here, and
//! [`yantra_out_len`] being **zero** after [`yantra_host`] is evidence rather than an
//! empty pane — a non-empty one is a finding.
//!
//! # Why the arrangement is not here
//!
//! [`yantra_host`] is one call into [`yantra::host::host`], which is in the workspace and
//! which the gate compiles and tests. This crate is in the root manifest's `exclude` list
//! and **no gate step builds it**, so a second copy of "what a Sassembly machine looks
//! like" written here would be the only description of one that nothing checks.
//!
//! # Why this is a separate crate
//!
//! **It contains no `unsafe` blocks — not one.** The only thing here the compiler objects
//! to is the `#[unsafe(no_mangle)]` attribute, which edition 2024 classes as unsafe
//! because two libraries exporting the same symbol is undefined at link time. That is a
//! real hazard and a completely different one from memory unsafety.
//!
//! `yantra` itself is `forbid(unsafe_code)` and stays that way: the interpreter, the ELF
//! loader and the device bridge are all safe Rust. Putting a handful of attributes in
//! their own crate keeps the strong guarantee where it is load-bearing instead of
//! downgrading the whole VM to `allow` for the sake of an export table.

#![allow(
    unsafe_code,
    reason = "`no_mangle` export attributes and nothing else — see the note above. There \
              is no `unsafe` block in this crate."
)]

use std::sync::Mutex;

use yantra::host::{self, Hosted};
use yantra::supervisor::Ended;
use yantra::{Halt, Machine};

/// The ELF, written by JS between `yantra_alloc` and `yantra_run`.
static IMAGE: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// Whatever reached the machine's own UART. A boot proof fills this; an application
/// cannot write it at all, and after [`yantra_host`] a non-empty one is a finding.
static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// Whatever an application put on the surface it was granted. Deliberately not the same
/// buffer as [`OUTPUT`] — see the module note.
static SURFACE: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// The halt reason, rendered for a human. Read through [`yantra_halt_ptr`].
static HALT: Mutex<String> = Mutex::new(String::new());

/// Reserve `len` bytes and return their offset in linear memory for JS to fill.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_alloc(len: u32) -> u32 {
    let mut image = IMAGE.lock().expect("single-threaded");
    *image = vec![0u8; len as usize];
    // Pointer-to-integer casts are safe; the buffer stays alive in the static, so the
    // offset JS writes to remains this Vec's storage for as long as it matters.
    image.as_ptr() as u32
}

/// Load and run the ELF that JS wrote. Returns a small code for the halt reason.
///
/// `0` success · `1` failed finisher · `2` spun · `3` unimplemented · `4` bad access
/// `5` out of budget · `6` the image would not load · `7` an SBI call this machine does
/// not implement · `8` a breakpoint · `9` a CSR access, which is refused. The
/// human-readable form is in
/// [`yantra_halt_ptr`]; the number exists so JS can colour the result without parsing text.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_run(ram: u32, budget: u32) -> u32 {
    let image = IMAGE.lock().expect("single-threaded").clone();
    let mut out = OUTPUT.lock().expect("single-threaded");
    let mut halt = HALT.lock().expect("single-threaded");
    out.clear();
    // A boot proof is handed no surface, so the previous run's must not linger under it.
    SURFACE.lock().expect("single-threaded").clear();

    let mut m = match Machine::load_elf(&image, ram as usize) {
        Ok(m) => m,
        Err(e) => {
            *halt = e;
            return 6;
        }
    };
    let reason = m.run(u64::from(budget), &mut *out);
    let code = match &reason {
        Halt::Finisher {
            status: Some(0), ..
        } => 0,
        Halt::Finisher { .. } => 1,
        Halt::SpinForever { .. } => 2,
        Halt::Unimplemented { .. } => 3,
        Halt::BadAccess { .. } => 4,
        // NOT folded into `BadAccess`. Unmapped is the PROGRAM's wrong
        // address; past-the-end is a RAM the HOST sized too small, and only
        // the second is something the page's reader can act on. `yantra`
        // gained this variant and these two matches did not — the page build
        // stopped compiling and `check-sassembly-identity.sh` reported it as
        // "the wasm32-unknown-unknown target is not available", which is false
        // on a machine that has the target: the real error was discarded by
        // `2>/dev/null` in `build-sassembly-web.sh:48`. Measured 2026-09-24.
        Halt::BeyondRam { .. } => 12,
        Halt::StepLimit { .. } => 5,
        // A program that asked the firmware to shut down got what it asked for, and the
        // page must not colour that red — `boot-sbi` ends this way on purpose.
        Halt::Shutdown { .. } => 0,
        Halt::Sbi { .. } => 7,
        Halt::Breakpoint { .. } => 8,
        Halt::Csr { .. } => 9,
        Halt::PageFault { .. } => 10,
        Halt::Undelivered { .. } => 11,
    };
    *halt = describe(&reason);
    code
}

/// Host the ELF that JS wrote as an **application**: a loader places it, a supervisor
/// answers its calls, and it never leaves U-mode. Returns a small code for the outcome.
///
/// This is [`yantra::host::host`] and nothing else — the arrangement it needs lives in
/// the crate the gate tests, not here.
///
/// The codes are a **different space** from [`yantra_run`]'s, because the outcomes are:
/// a program that exits is not a machine that halted.
///
/// `0` exited with status 0 · `1` exited with a non-zero status, which is the program's
/// verdict on itself and not a failure of the machine · `2` trapped out of U-mode and
/// the supervisor ended it · `3` the MACHINE stopped, which nothing reachable through
/// this ABI produces and is therefore a finding · `4` the step budget ran out ·
/// `5` the quantum expired, which **this entry point never returns**: it runs one program
/// through [`yantra::supervisor::Supervisor`], which arms no timer, and the code is
/// reserved so the space stays the enum's ·
/// `6` the image would not load, and nothing executed. The words are in
/// [`yantra_halt_ptr`]; the number exists so JS can colour the result without parsing them.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_host(ram: u32, budget: u32) -> u32 {
    let image = IMAGE.lock().expect("single-threaded").clone();
    let mut out = OUTPUT.lock().expect("single-threaded");
    let mut surface = SURFACE.lock().expect("single-threaded");
    let mut halt = HALT.lock().expect("single-threaded");
    out.clear();
    surface.clear();

    let hosted = match host::host(&image, ram as usize, u64::from(budget)) {
        Ok(h) => h,
        Err(e) => {
            *halt = e;
            return 6;
        }
    };
    let Hosted {
        ended,
        surface: bytes,
        uart,
        ..
    } = hosted;
    *surface = bytes;
    *out = uart;
    *halt = host::describe(&ended);
    match ended {
        Ended::Exited { status: 0 } => 0,
        Ended::Exited { .. } => 1,
        Ended::Faulted { .. } => 2,
        Ended::Stopped(_) => 3,
        Ended::OutOfBudget => 4,
        Ended::Preempted => 5,
    }
}

/// Offset of the UART output buffer.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_out_ptr() -> u32 {
    OUTPUT.lock().expect("single-threaded").as_ptr() as u32
}

/// Length of the UART output buffer.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_out_len() -> u32 {
    OUTPUT.lock().expect("single-threaded").len() as u32
}

/// Offset of the granted surface's bytes. Empty unless [`yantra_host`] ran.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_surface_ptr() -> u32 {
    SURFACE.lock().expect("single-threaded").as_ptr() as u32
}

/// Length of the granted surface's bytes.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_surface_len() -> u32 {
    SURFACE.lock().expect("single-threaded").len() as u32
}

/// Offset of the halt description.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_halt_ptr() -> u32 {
    HALT.lock().expect("single-threaded").as_ptr() as u32
}

/// Length of the halt description.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_halt_len() -> u32 {
    HALT.lock().expect("single-threaded").len() as u32
}

/// A halt reason in words. Every variant names an address, because "it stopped" without
/// one sends the reader to the wrong place.
fn describe(h: &Halt) -> String {
    match h {
        Halt::Finisher {
            status: Some(0), ..
        } => "the program wrote 0x5555 to the finisher — success, exit 0".into(),
        Halt::Finisher { value, status } => format!(
            "the program wrote {value:#x} to the finisher — exit status {}",
            status.map_or_else(|| "unrecognised".to_string(), |s| s.to_string())
        ),
        Halt::SpinForever { pc } => {
            format!("reached the parking loop at {pc:#x} — the program is done")
        }
        Halt::Unimplemented { pc, word, opcode } => format!(
            "instruction {word:#010x} at {pc:#x} has opcode {opcode:#04x}, which this \
             interpreter does not implement. It stops rather than skipping: a VM that \
             ignores what it does not know produces a plausible wrong answer."
        ),
        Halt::BadAccess { pc, addr } => {
            format!("the instruction at {pc:#x} touched {addr:#x}, which is not mapped")
        }
        Halt::BeyondRam { pc, addr, ram } => format!(
            "the instruction at {pc:#x} touched {addr:#x}, past the {ram} octets of RAM \
             this machine was given. The address is mapped in principle — the RAM is \
             the limit that was crossed, so this is a size the host chose and not a \
             wrong address the program computed."
        ),
        Halt::StepLimit { pc } => format!(
            "ran out of instruction budget at {pc:#x}. Bounded on purpose — a browser tab \
             must not be wedged by a program that never finishes."
        ),
        Halt::Shutdown { pc } => format!(
            "the program asked the firmware to shut the machine down at {pc:#x} — SBI \
             sbi_shutdown. On this page the firmware is the interpreter."
        ),
        Halt::Sbi { pc, eid, fid } => format!(
            "the ecall at {pc:#x} asked for SBI extension {eid:#x}, function {fid:#x}, \
             which this machine does not implement. It stops rather than returning 0: a \
             call that quietly succeeds leaves the program running on a promise nothing \
             kept."
        ),
        Halt::Breakpoint { pc } => {
            format!("ebreak at {pc:#x} — there is no debugger behind it here")
        }
        Halt::PageFault { pc, addr, cause } => format!(
            "the instruction at {pc:#x} could not translate {addr:#x} — page fault, \
             scause {cause}, and `stvec` is zero so there is no handler to deliver it to. \
             On metal it would trap to address 0 and execute the rubble there."
        ),
        Halt::Undelivered { pc, cause } if cause & yantra::INTERRUPT != 0 => format!(
            "interrupt {} came due before the instruction at {pc:#x} — 5 is the timer — \
             and `stvec` is zero, so there is no handler to take it.",
            cause & !yantra::INTERRUPT
        ),
        Halt::Undelivered { pc, cause } => format!(
            "the instruction at {pc:#x} raised exception {cause} — 8 is an environment \
             call from user mode, 2 an instruction the mode it ran in may not execute — \
             and `stvec` is zero, so the kernel it is addressed to has installed no \
             handler."
        ),
        Halt::Csr { pc, csr, write } => format!(
            "the instruction at {pc:#x} would {} CSR {csr:#05x}. This machine refuses CSR \
             access rather than faking it: with no trap delivery, no MMU and no privilege \
             levels behind them, accepting the write and running on would produce a \
             program that works and is wrong.",
            if *write { "modify" } else { "read" }
        ),
    }
}
