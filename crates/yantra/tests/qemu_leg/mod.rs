//! **`V-009` part (i-b): THE QEMU LEG FOR A COMPILED IMAGE.**
//!
//! A compiled `.t1` image — the startup object plus its modules, linked at
//! `0x8000_0000` — run on `qemu-system-riscv64`, a machine this repository did
//! not write, with its UART octets and its finisher exit code read back.
//!
//! `-bios none` puts the hart at the reset vector in **M-mode** with `mstatus.FS`
//! and `mstatus.VS` both `Off`, so the first float or vector instruction is an
//! illegal-instruction exception unless the startup has turned the units on.
//! `yantra` starts in S-mode and does not gate F or V on those fields, which is
//! why no float or vector image had run on QEMU from `V-005` to here.
//!
//! **ANY EXCEPTION IS A FAILURE, NAMED.** A compiled image takes no trap: it
//! installs no handler and enables no interrupt. So the run is logged with
//! `-d int`, and the first logged trap ends it at once with QEMU's own line
//! (cause, `epc`, description) — rather than letting a fault at `mtvec = 0` spin
//! until a timeout that says only "never finished".
//!
//! The finisher is QEMU virt's `sifive_test` at `0x10_0000`, as on `yantra`:
//! `0x5555` exits 0, `0x3333 | n << 16` exits `n`. The UART at `0x1000_0000`
//! goes to a FILE, not to stdio, so the program's octets arrive unchanged.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The vector shape `yantra` implements (`vector_oracle.rs`): VLEN 128, ELEN 64.
pub const CPU: &str = "rv64,v=true,vlen=128,elen=64";

fn scratch() -> PathBuf {
    // One directory per CALL: the tests of one file run on parallel threads.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("v009-qemu-leg-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Run `image` on QEMU. `Ok` is the UART octets of a run whose finisher wrote
/// `0x5555`; `Err` names how it failed: the first trap QEMU logged, a non-zero
/// finisher status, or no finish within 60 s.
pub fn run(image: &[u8]) -> Result<Vec<u8>, String> {
    let dir = scratch();
    let (elf, out, log) = (dir.join("q.elf"), dir.join("uart.bin"), dir.join("int.log"));
    std::fs::write(&elf, image).unwrap();
    // 1 GiB: an allocating image's startup reserves a 512 MiB record region in
    // `.bss` above its text, and QEMU's ELF loader zero-fills it into RAM.
    let mut child = Command::new("qemu-system-riscv64")
        .args(["-machine", "virt", "-cpu", CPU, "-m", "1G"])
        .args(["-display", "none", "-monitor", "none", "-bios", "none"])
        .arg("-serial")
        .arg(format!("file:{}", out.display()))
        .args(["-d", "int", "-D"])
        .arg(&log)
        .arg("-kernel")
        .arg(&elf)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-system-riscv64 — the reference machine the QEMU leg runs on");
    let first_trap = || {
        std::fs::read_to_string(&log)
            .ok()
            .and_then(|s| s.lines().find(|l| l.contains("cause:")).map(str::to_owned))
    };
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if let Some(trap) = first_trap() {
            // ONLY THIS CHILD is killed — never by name.
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("qemu trapped: {trap}"));
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("qemu never reached the finisher in 60 s".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if let Some(trap) = first_trap() {
        return Err(format!("qemu trapped: {trap}"));
    }
    let uart = std::fs::read(&out).unwrap_or_default();
    if !status.success() {
        let mut e = String::new();
        if let Some(mut s) = child.stderr.take() {
            use std::io::Read as _;
            let _ = s.read_to_string(&mut e);
        }
        return Err(format!(
            "qemu exited {status} (the finisher's status, or QEMU's own refusal): {}",
            e.trim()
        ));
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(uart)
}
