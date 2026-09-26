//! Load an ELF as an APPLICATION and print what it put on its surface — task `F-020`.
//!
//! The sibling of `yantra-run`, and the distinction between them is `ADR-0015`, not a
//! preference. `yantra-run` boots an image bare at `0x8000_0000` in machine mode and
//! offers SBI; that is right for the OS proofs and **wrong for an application**, which is
//! entered by a loader in U-mode, calls `spec/application-abi.tsv` rather than SBI, and
//! writes to a surface it was handed rather than to a device.
//!
//! Running an application the bare way faults on the entry instruction. The owner did
//! exactly that on `spec/atithi.sas` through the editor and got
//! `BadAccess { pc: 2147483648, addr: 0 }` — `pc` is `0x8000_0000`, so it died on the
//! first instruction, and the output reads like a broken program rather than a wrong
//! invocation. This binary is the other half of the pair so that the right invocation
//! exists to be chosen.
//!
//! No CLI could reach `yantra::host::host` before this: the only binary in the crate was
//! `yantra-run`. That is why the editor had one path to offer and offered it to
//! everything.
use std::process::ExitCode;

/// Matches `crates/yantra/tests/application.rs`, which is the file `spec/programs.tsv`
/// records as the application's assertion. Two runners disagreeing about how much memory
/// or how many steps an application gets would make the test and the demo different
/// claims about the same program.
const RAM: usize = 1 << 22;
const BUDGET: u64 = 200;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: yantra-host <application.elf>");
        return ExitCode::FAILURE;
    };
    let image = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match yantra::host::host(&image, RAM, BUDGET) {
        Ok(h) => {
            // The SURFACE is the application's output; the UART is the machine's own
            // device, which an application has no way to address. A non-empty `uart` here
            // is therefore a finding rather than output, and `host.rs` says so — so it is
            // printed rather than dropped.
            use std::io::Write;
            std::io::stdout().write_all(&h.surface).unwrap_or(());
            if !h.uart.is_empty() {
                eprintln!("UART: {}", String::from_utf8_lossy(&h.uart));
                eprintln!(
                    "note: an application cannot address a device — a non-empty UART is a finding"
                );
            }
            eprintln!("ended: {:?}", h.ended);
            eprintln!("scause: {}", h.scause);
            ExitCode::SUCCESS
        }
        Err(e) => {
            // A refusal from `host` is never a run that happened differently — nothing has
            // executed yet — so it is reported as a load failure and not as program output.
            eprintln!("{path}: this image could not be loaded as an application: {e}");
            ExitCode::FAILURE
        }
    }
}
