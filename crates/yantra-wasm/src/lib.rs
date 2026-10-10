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
//! wasm.yantra_rev_ptr()   -> offset    WHICH `crates/yantra/src` this module was
//! wasm.yantra_rev_len()   -> len       built from — 16 hex, or `unknown`
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
use yantra::{Halt, Machine, Output};

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
/// Retired instructions of the last [`yantra_run`], read through
/// [`yantra_steps_lo`] / [`yantra_steps_hi`].
///
/// ॥ `None` IS NOT ZERO, AND THAT IS THE WHOLE POINT ॥ `Machine::time` counts
/// RETIRED INSTRUCTIONS, which is why `T-102` holds across hosts at all and why
/// naad wanted it: the browser had the right PCM digest
/// (`29fc76d8e0cc3ed935aa1f28498406d9`, cmp-identical to native, a fourth
/// machine) and no way to read the step count, so the attestation was half a
/// measurement. Its only alternative was bisecting the budget against code 5
/// over ~28 runs.
///
/// But [`yantra_host`] CANNOT supply one — `host::Hosted` carries `ended`,
/// `surface`, `uart`, `scause` and `entry`, and no counter — so a bare
/// `lo`/`hi` pair would hand a `yantra_host` caller the previous `yantra_run`'s
/// number, or hand an unrun machine a `0` indistinguishable from a program that
/// retired no instructions. Both entry points therefore CLEAR this first, and
/// [`yantra_steps_known`] says whether there is a reading at all.
static STEPS: Mutex<Option<u64>> = Mutex::new(None);
/// The highest RAM offset any store of the last [`yantra_run`] reached, read
/// through [`yantra_water_known`] / [`yantra_water`].
///
/// ॥ WHY A SECOND INSTRUMENT FOR `touches` EXISTS AT ALL ॥ The Sassembly page
/// prints a sizing line — "demands N, touches M, Nx" — and until this export
/// the `touches` half was answerable to NOTHING. `check-sassembly-page.mjs`
/// re-derives `ram` from the embedded ELF's own PT_LOAD spans, but the high
/// water is not in any header: it is a property of a RUN. So the build's figure
/// came from the native `yantra-run`'s `YANTRA_WATERMARK` line and every arm on
/// the page compared the rendered line against that same number — prose
/// compared with itself, PROVED by doctoring the built page's figure and
/// watching every arm stay green. This is the tab's own reading of the same
/// quantity, so the page can assert two engines against one number, which is
/// what it already does for the step count.
///
/// ॥ `None` IS NOT ZERO, FOR THE SAME REASON IT IS NOT FOR [`STEPS`] ॥ A
/// program that stores nothing reports a high water of `0` legitimately — it is
/// not even bounded below by `filesz`, because the load copy is a
/// `copy_from_slice` and `Output::stored` moves on STORES ONLY. A load that
/// failed, or a [`yantra_host`] call, has NO reading. Both entry points clear
/// this first and [`yantra_water_known`] says whether there is one.
///
/// A `u32` because a store is bounds-checked against the RAM the caller asked
/// for, and that argument is a `u32`: a water past `u32::MAX` is unreachable
/// here. It is still not ASSUMED away — `u32::try_from(..).ok()` leaves the
/// reading ABSENT rather than truncating it, so the impossible case would be
/// read as "no measurement" and never as a small number.
static WATER: Mutex<Option<u32>> = Mutex::new(None);
/// The input slab, written by JS between [`yantra_input_alloc`] and [`yantra_run`].
///
/// Empty means NO INPUT, which is the behaviour every caller had before this existed:
/// a program that declares no input globals is unaffected, and one that does gets the
/// same refusal it gets natively.
static INPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// The MODULE NAME that goes with the slab, written by JS between
/// [`yantra_input_name_alloc`] and [`yantra_run`].
///
/// **This is not optional padding.** `input::inject` places TWO runs and TWO tag
/// words — `SASINPUT` for the octets and `SASINAME` for the module name
/// (`input.rs:200-202`) — and natively `--input` is refused without `--input-name`.
/// A program reads its own module name out of `निवेशमण्डलनाम`, so a slab injected
/// with an empty name is a slab the program cannot attribute.
static INPUT_NAME: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Reserve `len` bytes and return their offset in linear memory for JS to fill.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_alloc(len: u32) -> u32 {
    let mut image = IMAGE.lock().expect("single-threaded");
    *image = vec![0u8; len as usize];
    // Pointer-to-integer casts are safe; the buffer stays alive in the static, so the
    // offset JS writes to remains this Vec's storage for as long as it matters.
    image.as_ptr() as u32
}

/// Reserve `len` bytes for the INPUT SLAB and return their offset for JS to fill.
///
/// ॥ WHY THIS IS AN ALLOC AND NOT `yantra_input(ptr, len)` ॥ The slab was asked for
/// as `yantra_input(ptr, len)`, reading `len` octets from an offset JS chose. That
/// cannot be written here: reading a caller-supplied offset needs
/// `slice::from_raw_parts`, and this crate's own module note says the allowance is for
/// "`no_mangle` export attributes **and nothing else** — there is no `unsafe` block in
/// this crate." Keeping that true is worth more than matching a proposed signature, and
/// [`yantra_alloc`] already establishes the contract JS uses for the image: ask for the
/// bytes, fill them, then run. This is the same contract for a second buffer.
///
/// Call it BEFORE [`yantra_run`]. Calling it again replaces the slab; passing `0`
/// clears it, and a cleared slab means no injection at all.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_input_alloc(len: u32) -> u32 {
    let mut slab = INPUT.lock().expect("single-threaded");
    *slab = vec![0u8; len as usize];
    slab.as_ptr() as u32
}

/// Reserve `len` bytes for the input's MODULE NAME and return their offset for JS.
///
/// Required whenever a slab is set, for the reason the `INPUT_NAME` static gives: `inject`
/// places two runs and two tag words, and a program reads its own module name out of
/// `निवेशमण्डलनाम`. Natively, `--input` without `--input-name` is refused.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_input_name_alloc(len: u32) -> u32 {
    let mut name = INPUT_NAME.lock().expect("single-threaded");
    *name = vec![0u8; len as usize];
    name.as_ptr() as u32
}

/// The in-memory file root (`yantra::patra::MemFs`), or `None` when the host has not
/// enabled one — in which case every file request is refused by name, as natively
/// without `--files`. Moved INTO the machine for a run and back out after it, so the
/// files a program wrote are readable through the `yantra_memfs_*` accessors.
static MEMFS: Mutex<Option<yantra::patra::MemFs>> = Mutex::new(None);
/// The scratch buffer `yantra_memfs_alloc` hands out; `yantra_memfs_put` reads its
/// name and data from inside it (this crate has no `unsafe` block, so it cannot read
/// an arbitrary offset).
static MEMFS_SCRATCH: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Enable an EMPTY in-memory file root with the default caps (64 MiB total, 16 MiB
/// per file, 4096 files), discarding any earlier one. Call before [`yantra_run`]; a run
/// with no root enabled refuses every file request.
///
/// **THE ROOT IS CARRIED FROM RUN TO RUN**: files one run wrote are there for the next,
/// until the host enables a fresh root (or disables it). [`yantra_host`] never sees it:
/// a hosted application runs in U-mode and cannot address the MMIO file window at all.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_enable() {
    *MEMFS.lock().expect("single-threaded") = Some(yantra::patra::MemFs::default());
}

/// As [`yantra_memfs_enable`] with explicit caps in octets: `total` across all files
/// (names and per-entry overhead count), `file` for one. Returns `0` enabled, `1` refused
/// because `file > total` (the earlier root, if any, is left as it was).
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_enable_caps(total: u32, file: u32) -> u32 {
    match yantra::patra::MemFs::with_caps(total as usize, file as usize) {
        Ok(fs) => {
            *MEMFS.lock().expect("single-threaded") = Some(fs);
            0
        }
        Err(_) => 1,
    }
}

/// Drop the in-memory root: later runs refuse every file request again.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_disable() {
    *MEMFS.lock().expect("single-threaded") = None;
}

/// Reserve `len` octets of scratch and return their offset. JS writes a file's name
/// and data into it (any layout) and passes the two spans to [`yantra_memfs_put`].
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_alloc(len: u32) -> u32 {
    let mut s = MEMFS_SCRATCH.lock().expect("single-threaded");
    *s = vec![0u8; len as usize];
    s.as_ptr() as u32
}

/// Seed (create or replace) a file from two spans inside the scratch buffer.
///
/// Returns `0` stored · `1` no root enabled · `2` a span outside the scratch · `3` the
/// name is not UTF-8 · `4` the window's `Refused` (absolute path or escape) · `5` its
/// `NotWritten` (over a cap) · `9` anything else.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_put(
    name_ptr: u32,
    name_len: u32,
    data_ptr: u32,
    data_len: u32,
) -> u32 {
    let scratch = MEMFS_SCRATCH.lock().expect("single-threaded");
    let base = scratch.as_ptr() as u32;
    let span = |ptr: u32, len: u32| -> Option<&[u8]> {
        let off = ptr.checked_sub(base)? as usize;
        scratch.get(off..off.checked_add(len as usize)?)
    };
    let mut fs = MEMFS.lock().expect("single-threaded");
    let Some(fs) = fs.as_mut() else { return 1 };
    let (Some(name), Some(data)) = (span(name_ptr, name_len), span(data_ptr, data_len)) else {
        return 2;
    };
    let Ok(name) = std::str::from_utf8(name) else {
        return 3;
    };
    match fs.seed(name, data) {
        yantra::patra::Status::Wrote(_) => 0,
        yantra::patra::Status::Refused => 4,
        yantra::patra::Status::NotWritten => 5,
        _ => 9,
    }
}

/// How many files the in-memory root holds (`0` when none is enabled).
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_count() -> u32 {
    MEMFS
        .lock()
        .expect("single-threaded")
        .as_ref()
        .map_or(0, |f| f.len() as u32)
}

/// Offset of the `i`th file's name (name order), valid until the next put or run;
/// `0` when `i` is out of range.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_name(i: u32) -> u32 {
    memfs_nth(i, |n, _| (n.as_ptr() as u32, n.len() as u32)).0
}

/// Length of the `i`th file's name.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_name_len(i: u32) -> u32 {
    memfs_nth(i, |n, _| (n.as_ptr() as u32, n.len() as u32)).1
}

/// Offset of the `i`th file's data, valid until the next put or run.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_data(i: u32) -> u32 {
    memfs_nth(i, |_, d| (d.as_ptr() as u32, d.len() as u32)).0
}

/// Length of the `i`th file's data.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_memfs_data_len(i: u32) -> u32 {
    memfs_nth(i, |_, d| (d.as_ptr() as u32, d.len() as u32)).1
}

fn memfs_nth(i: u32, f: impl Fn(&str, &[u8]) -> (u32, u32)) -> (u32, u32) {
    let fs = MEMFS.lock().expect("single-threaded");
    fs.as_ref()
        .and_then(|fs| fs.nth(i as usize))
        .map_or((0, 0), |(n, d)| f(n, d))
}

/// Load and run the ELF that JS wrote. Returns a small code for the halt reason.
///
/// `0` success · `1` failed finisher · `2` spun · `3` unimplemented · `4` bad access
/// `5` out of budget · `6` the image would not load · `7` an SBI call this machine does
/// not implement · `8` a breakpoint · `9` a CSR access, which is refused · `10` a page
/// fault · `11` an undelivered trap · `12` beyond RAM · `13` the input was refused ·
/// `14` an unanswered device register · `15` the program asked to WAIT (`W-370`). The
/// human-readable form is in
/// [`yantra_halt_ptr`]; the number exists so JS can colour the result without parsing text.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_run(ram: u32, budget: u32) -> u32 {
    // No reading until there is one: a load that fails must not report the
    // PREVIOUS run's count as this one's.
    *STEPS.lock().expect("single-threaded") = None;
    *WATER.lock().expect("single-threaded") = None;
    let image = IMAGE.lock().expect("single-threaded").clone();
    let mut out = OUTPUT.lock().expect("single-threaded");
    let mut halt = HALT.lock().expect("single-threaded");
    out.clear();
    // A boot proof is handed no surface, so the previous run's must not linger under it.
    SURFACE.lock().expect("single-threaded").clear();

    // ॥ THE BUDGET IS A BUDGET, SO THE DECLARED `.bss` TAIL IS NOT A FLOOR ॥ `W-363`.
    //
    // The caller here is a BROWSER, where linear memory is the hard constraint — a
    // phone will not hand out 540 MB where it would hand out 32. The `.t1` compiler
    // declares its heap as 512.0 MiB of `.bss` tail (`yantrotsarjana.t1:2070`), which
    // costs no file octets and, read as a requirement, demanded the RAM anyway. A
    // walker's high water over a whole 154-second recording is 6.3 MiB, so the ask is
    // 82x what the program touches, and `Span::Declared` refused every budget a phone
    // could give. Measured by a peer session: ten frames decode identically at
    // 553,753,920, 33,554,432, 4,194,304 and 3,145,728 octets — same output, same
    // 235,791,469 retired instructions, same digest as native.
    //
    // THE NATIVE RUNNER KEEPS `Declared` AND SHOULD. There, RAM is not scarce and a
    // load-time refusal naming the shortfall is the better failure; here the whole
    // point is to run inside a budget the host imposes. `store_limit` is what makes
    // the small reading safe rather than merely smaller — see its margin in `yantra`.
    let mut m = match Machine::load_elf_spanning(&image, ram as usize, yantra::Span::FileBacked) {
        Ok(m) => m,
        Err(e) => {
            *halt = e;
            return 6;
        }
    };
    // ॥ THE INPUT SLAB, PLACED BEFORE `pc` FIRST MOVES ॥ `F-004`, 2026-10-02,
    // owner-approved. This is the SAME call the native runner makes
    // (`bin/yantra-run.rs:110`) with the same arguments in the same order, so a
    // program fed in the browser is fed the way it is fed natively — and
    // `input::inject` locates the slots by SCANNING RAM for a magic word
    // (`input.rs:155`), not by symbol lookup, so nothing here needs to know the
    // program's layout.
    //
    // It costs no determinism, which is why it was the one capability approved
    // while a wall clock and threads were refused: the octets are in RAM before
    // the first instruction retires, so the instruction count is unchanged for a
    // given input and `T-102` still holds.
    //
    // A REFUSAL MUST NOT BE SILENT. `inject` returns `Err` when the tags are not
    // in the image — the usual cause is a program that declares no input globals,
    // which is a mistake worth seeing rather than a run that quietly reads
    // nothing. Code 13 is its own, outside every existing code's meaning — and
    // stays so: `Halt::Device` (W-351) took 14 below, because a caller reading 13
    // could not otherwise tell a refused input from an unanswered device register.
    {
        let slab = INPUT.lock().expect("single-threaded");
        let name = INPUT_NAME.lock().expect("single-threaded");
        match plan_input(&slab, &name) {
            InputPlan::Nothing => {}
            InputPlan::Refuse(why) => {
                *halt = format!("input: refused — {why}");
                return 13;
            }
            InputPlan::Inject { slab, name } => {
                let base = m.base;
                if let Err(e) = yantra::input::inject(&mut m.mem, base, slab, name, 0) {
                    *halt = format!("input: refused — {e}");
                    return 13;
                }
            }
        }
    }
    // ॥ THE UART IS NO LONGER THE WHOLE SINK — THE WATER IS MEASURED WITH IT ॥
    // `Vec<u8>` is an `Output` whose `stored` is the trait's no-op default, so a
    // run through it could say what it printed and not how much RAM it touched.
    // This is `bin/yantra-run.rs:11`'s `Sink` with the same arithmetic, which is
    // the point: the page compares the two engines' readings of one quantity, and
    // a different rule on each side would make a divergence meaningless.
    // The in-memory file root goes INTO the machine for the run and comes back out
    // after it, so its files can be read through `yantra_memfs_*`.
    m.patra_mem = MEMFS.lock().expect("single-threaded").take();
    let mut sink = Watermarked {
        out: &mut out,
        high_water: 0,
    };
    // A panic inside the run must not lose the host's files: restore, then re-raise.
    // (On wasm32-unknown-unknown a panic aborts and the instance is dead either way.)
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        m.run(u64::from(budget), &mut sink)
    }));
    *MEMFS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = m.patra_mem.take();
    let reason = match ran {
        Ok(r) => r,
        Err(e) => std::panic::resume_unwind(e),
    };
    let high_water = sink.high_water;
    // AFTER the run and before `m` drops: `m` is local to this function.
    *STEPS.lock().expect("single-threaded") = Some(m.time);
    *WATER.lock().expect("single-threaded") = water_reading(high_water);
    let code = halt_code(&reason);
    *halt = describe(&reason);
    code
}

/// What [`yantra_run`] does with the two input buffers, decided before anything is
/// written into RAM — split out of it so the decision can be tested natively, the way
/// [`halt_code`] is.
enum InputPlan<'a> {
    /// No slab was set, so RAM is left exactly as the ELF loaded it. This is every
    /// caller's behaviour from before the channel existed.
    Nothing,
    /// Place these octets under this module name.
    Inject { slab: &'a [u8], name: &'a [u8] },
    /// Stop before the first instruction retires, and say why.
    Refuse(&'static str),
}

/// ॥ A SLAB WITHOUT A NAME IS REFUSED, BECAUSE IT IS REFUSED NATIVELY ॥ `F-004`.
///
/// The `INPUT_NAME` static already said the name is "not optional padding" and
/// [`yantra_input_name_alloc`] already said it is "required whenever a slab is set" —
/// but NOTHING ENFORCED EITHER. `yantra::input::inject` does not check: it calls
/// `append_run(mem, base, name)` on the empty slice and writes a ZERO-LENGTH run at
/// `SASINAME`, returning `Ok`. So the browser quietly produced a slab the program
/// cannot attribute, where `yantra-run.rs:228-232` exits FAILURE with "the module name
/// is passed, never guessed". That divergence is the defect; this closes it, and the
/// code is `13` because it is the same event — the input was refused.
///
/// A NAME WITHOUT A SLAB IS NOT A REFUSAL, and that is native parity too:
/// `YANTRA_INPUT_NAME` set without `YANTRA_INPUT` is read by nothing there, because the
/// whole block is inside `if let Some(input)`. A caller who sets only the name gets no
/// injection — the same as setting neither.
fn plan_input<'a>(slab: &'a [u8], name: &'a [u8]) -> InputPlan<'a> {
    if slab.is_empty() {
        return InputPlan::Nothing;
    }
    if name.is_empty() {
        return InputPlan::Refuse(
            "an input slab was set with no module name. Call yantra_input_name_alloc and \
             write the name before yantra_run — the module name is passed, never guessed",
        );
    }
    InputPlan::Inject { slab, name }
}

/// The UART, plus the highest RAM offset any store reached — the browser's copy of
/// `crates/yantra/src/bin/yantra-run.rs:11`'s `Sink`, with the same arithmetic.
///
/// It keeps a `usize` and not the static's `u32` so the measurement is taken in the
/// interpreter's own width and narrowed once, at [`water_reading`], where the narrowing
/// is a rule with a test rather than a cast in the middle of a run.
struct Watermarked<'a> {
    out: &'a mut Vec<u8>,
    high_water: usize,
}

impl Output for Watermarked<'_> {
    fn putc(&mut self, byte: u8) {
        self.out.push(byte);
    }
    fn stored(&mut self, at: usize, width: usize) {
        if at + width > self.high_water {
            self.high_water = at + width;
        }
    }
}

/// What goes into [`WATER`] for a run whose sink ended at `high_water` — split out of
/// [`yantra_run`] so the rule can be tested natively, the way [`halt_code`] and
/// [`plan_input`] are.
///
/// ॥ A WATER THAT DOES NOT FIT IS ABSENT, NOT TRUNCATED ॥ `as u32` on a value past
/// `u32::MAX` would report a 4 GiB high water as a small number, and the page's
/// comparison against the native figure would then read as "the two engines disagree
/// about the program" when the truth is "this ABI cannot carry the answer". Unreachable
/// on this entry point — `yantra_run` takes `ram: u32` and a store is bounds-checked
/// against it — and still not assumed away.
fn water_reading(high_water: usize) -> Option<u32> {
    u32::try_from(high_water).ok()
}

/// The number [`yantra_run`] returns for a halt — split out of it so the mapping can be
/// tested natively (`W-370`). **Every variant has its own code, with one declared
/// exception**: `Shutdown` shares `0` with a successful finisher, on purpose (its arm
/// says why). The test below fails if any other two variants meet, and its exhaustive
/// match fails to compile when a variant is added and not given a code.
fn halt_code(reason: &Halt) -> u32 {
    match reason {
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
        // A virtio-mmio register the device does not answer (`W-351`) — not a wild
        // address, so not `BadAccess`'s code either.
        Halt::Device { .. } => 14, // NOT 13: the input refusal above owns it
        Halt::StepLimit { .. } => 5,
        // A program that asked the firmware to shut down got what it asked for, and the
        // page must not colour that red — `boot-sbi` ends this way on purpose.
        Halt::Shutdown { .. } => 0,
        Halt::Sbi { .. } => 7,
        Halt::Breakpoint { .. } => 8,
        Halt::Csr { .. } => 9,
        Halt::PageFault { .. } => 10,
        Halt::Undelivered { .. } => 11,
        // A PAUSE, NOT AN END (`W-370`): the program asked the host to wait for the
        // world, and this page has no world to give it. Its own code, never 0 (it did
        // not finish) and never 5 (it did not run out of budget).
        Halt::Wait { .. } => 15,
    }
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
    // `host::Hosted` carries no counter, so this entry point has NO step and NO
    // high-water reading to give. Clearing it is what stops `yantra_steps_*` and `yantra_water` handing a
    // hosted run the last `yantra_run`'s number.
    *STEPS.lock().expect("single-threaded") = None;
    *WATER.lock().expect("single-threaded") = None;
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
/// Whether there is a step reading at all — 1 after a [`yantra_run`] that
/// executed, 0 otherwise.
///
/// JS must consult this BEFORE the two halves, because `lo == 0 && hi == 0` is
/// otherwise ambiguous between "no run" and "a program that retired nothing".
/// An export that cannot express its own negative is not a measurement.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_steps_known() -> u32 {
    u32::from(STEPS.lock().expect("single-threaded").is_some())
}

/// The low 32 bits of the last [`yantra_run`]'s retired-instruction count.
///
/// Split in two rather than returned as one `u64` so JS reads it without
/// BigInt and without the wasm `i64` ABI, which is what naad asked for.
/// Recombine as `hi * 2**32 + lo` — the counts in play are around 1.1e10 for
/// ten frames, well inside a double's exact integer range.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_steps_lo() -> u32 {
    STEPS.lock().expect("single-threaded").unwrap_or(0) as u32
}

/// The high 32 bits of the last [`yantra_run`]'s retired-instruction count.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_steps_hi() -> u32 {
    (STEPS.lock().expect("single-threaded").unwrap_or(0) >> 32) as u32
}

/// Whether there is a high-water reading at all — 1 after a [`yantra_run`] that
/// executed, 0 otherwise.
///
/// JS must consult this BEFORE [`yantra_water`], because `0` is otherwise
/// ambiguous between "no run" and "a run that stored nothing" — and the second is
/// a real outcome here, not a pathology. See [`WATER`].
#[unsafe(no_mangle)]
pub extern "C" fn yantra_water_known() -> u32 {
    u32::from(WATER.lock().expect("single-threaded").is_some())
}

/// The highest RAM offset any store of the last [`yantra_run`] reached, in octets.
///
/// One word and not a `lo`/`hi` pair like [`yantra_steps_lo`]: a step count is
/// unbounded and this is bounded by the `ram` argument, which is itself a `u32`.
/// `0` when there is no reading, which is why [`yantra_water_known`] exists.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_water() -> u32 {
    WATER.lock().expect("single-threaded").unwrap_or(0)
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

/// ॥ THE STAMP OF THE INTERPRETER THIS MODULE WAS BUILT FROM ॥
///
/// `crates/yantra-wasm/build.rs` seals `tools/src-rev.sh`'s 16-hex stamp over
/// `crates/yantra/src` in here at compile time, or `unknown` when no stamp could be
/// taken. It is read back by `web/yantra.mjs` and compared against the stamp
/// `tools/build-sassembly-web.sh` recorded for the NATIVE `yantra` it measured the
/// page's sizing figures with.
///
/// THE ASSEMBLER'S EQUIVALENT COMPARES TWO THINGS THAT AGREE; THIS ONE COMPARES A
/// RESULT AGAINST A MEASUREMENT NOBODY CAN RE-TAKE IN THE TAB. The halt status, the
/// step count and the UART text on the page are this module's own output, so there is
/// no second opinion to differ from — a stale interpreter that happens to be right
/// about three small programs looks exactly like a fresh one, while the `ram` and
/// `touches` figures beside them were taken by a `yantra-run` it is not.
static REV: &str = env!("YANTRA_SRC_REV");

/// Where [`REV`] sits in this module's linear memory.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_rev_ptr() -> u32 {
    REV.as_ptr() as u32
}

/// How many octets of [`REV`] there are — 16 for a stamp, 7 for `unknown`.
#[unsafe(no_mangle)]
pub extern "C" fn yantra_rev_len() -> u32 {
    REV.len() as u32
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
        Halt::Device { pc, addr, why } => format!(
            "the instruction at {pc:#x} touched the device register {addr:#x}, which the \
             machine's virtio-mmio device does not answer: {why}"
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
        Halt::Wait { pc } => format!(
            "the store at {pc:#x} asked the host to WAIT for the world (`W-370`). This page \
             has no event source, so nothing will arrive: the machine is paused, not \
             finished, and it is not resumed."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{Halt, InputPlan, Output, Watermarked, halt_code, plan_input, water_reading};

    /// `F-004` falsifier. FOUR states, not two: the refusal is only worth having if the
    /// two shapes that must still go through still do, and a reader with one `is_err()`
    /// assertion cannot tell "refuses the nameless slab" from "refuses everything".
    #[test]
    fn a_slab_without_a_module_name_is_refused_and_the_other_three_shapes_are_not() {
        // MUST REFUSE: octets with no name. This is the whole point — before this,
        // `inject` wrote a zero-length run at SASINAME and answered Ok.
        let InputPlan::Refuse(why) = plan_input(b"fLaC\0\0\0\x22", b"") else {
            panic!("a slab with an empty name must be refused");
        };
        assert!(
            why.contains("yantra_input_name_alloc"),
            "the refusal must name the call that fixes it: {why}"
        );

        // MUST INJECT: the shape a peer session actually uses.
        let InputPlan::Inject { slab, name } = plan_input(b"fLaC\0\0\0\x22", "ध्वनिः".as_bytes())
        else {
            panic!("a slab with a name is the supported call and must inject");
        };
        assert_eq!(
            slab, b"fLaC\0\0\0\x22",
            "the octets must pass through unaltered"
        );
        assert_eq!(
            name,
            "ध्वनिः".as_bytes(),
            "a non-ASCII module name is a name"
        );

        // MUST DO NOTHING: no slab at all — every caller from before the channel existed.
        assert!(
            matches!(plan_input(b"", b""), InputPlan::Nothing),
            "no slab means RAM is left as the ELF loaded it"
        );

        // MUST DO NOTHING, NOT REFUSE: a name with no slab. Native parity — the name is
        // read only inside `if let Some(input)`, so setting it alone injects nothing.
        assert!(
            matches!(plan_input(b"", "ध्वनिः".as_bytes()), InputPlan::Nothing),
            "a name without a slab injects nothing, as it does natively"
        );
    }

    /// A one-octet slab is a slab, and a one-octet name is a name. `is_empty` is the
    /// boundary the plan turns on, so the first value ON the accepting side is pinned
    /// rather than assumed — a `len() < 2` typo would pass the test above.
    #[test]
    fn the_boundary_is_emptiness_and_not_a_length() {
        assert!(
            matches!(plan_input(b"x", b"n"), InputPlan::Inject { .. }),
            "one octet under a one-octet name must inject"
        );
        assert!(
            matches!(plan_input(b"x", b""), InputPlan::Refuse(_)),
            "one octet with no name must still be refused"
        );
    }

    /// One witness of each variant, and its position — a `match` with no wildcard, so a
    /// variant added to `yantra::Halt` stops this compiling until it is listed here.
    /// THESE NUMBERS ARE POSITIONS, NOT ABI CODES: `halt_code` above is the ABI (StepLimit
    /// is 5 there and 12 here). The test only uses them to index the witnesses.
    fn ordinal(h: &Halt) -> usize {
        match h {
            Halt::Finisher { .. } => 0,
            Halt::SpinForever { .. } => 1,
            Halt::Unimplemented { .. } => 2,
            Halt::BadAccess { .. } => 3,
            Halt::BeyondRam { .. } => 4,
            Halt::Device { .. } => 5,
            Halt::Shutdown { .. } => 6,
            Halt::Sbi { .. } => 7,
            Halt::Breakpoint { .. } => 8,
            Halt::Csr { .. } => 9,
            Halt::PageFault { .. } => 10,
            Halt::Undelivered { .. } => 11,
            Halt::StepLimit { .. } => 12,
            Halt::Wait { .. } => 13,
        }
    }
    const VARIANTS: usize = 14;

    /// Every way a halt can come out, with the finisher split by the status that
    /// changes its code.
    fn every_halt() -> Vec<(&'static str, Halt)> {
        vec![
            (
                "Finisher(success)",
                Halt::Finisher {
                    value: 0x5555,
                    status: Some(0),
                },
            ),
            (
                "Finisher(failure)",
                Halt::Finisher {
                    value: 0x1_3333,
                    status: Some(1),
                },
            ),
            ("SpinForever", Halt::SpinForever { pc: 0 }),
            (
                "Unimplemented",
                Halt::Unimplemented {
                    pc: 0,
                    word: 0,
                    opcode: 0,
                },
            ),
            ("BadAccess", Halt::BadAccess { pc: 0, addr: 0 }),
            (
                "BeyondRam",
                Halt::BeyondRam {
                    pc: 0,
                    addr: 0,
                    ram: 0,
                },
            ),
            (
                "Device",
                Halt::Device {
                    pc: 0,
                    addr: 0,
                    why: "",
                },
            ),
            ("Shutdown", Halt::Shutdown { pc: 0 }),
            (
                "Sbi",
                Halt::Sbi {
                    pc: 0,
                    eid: 0,
                    fid: 0,
                },
            ),
            ("Breakpoint", Halt::Breakpoint { pc: 0 }),
            (
                "Csr",
                Halt::Csr {
                    pc: 0,
                    csr: 0,
                    write: false,
                },
            ),
            (
                "PageFault",
                Halt::PageFault {
                    pc: 0,
                    addr: 0,
                    cause: 0,
                },
            ),
            ("Undelivered", Halt::Undelivered { pc: 0, cause: 0 }),
            ("StepLimit", Halt::StepLimit { pc: 0 }),
            ("Wait", Halt::Wait { pc: 0 }),
        ]
    }

    /// `W-370` falsifier (c): no two halts share a `yantra_run` code, except the ONE
    /// sharing the ABI declares — `Shutdown` and a successful finisher both answer `0`,
    /// because a program that asked to be shut down got what it asked for. Codes `6`
    /// (the image would not load) and `13` (the input was refused) are not halts and are
    /// reserved here too, so no halt can be mistaken for either.
    #[test]
    fn every_halt_has_its_own_yantra_run_code() {
        let halts = every_halt();
        let mut seen: Vec<usize> = halts.iter().map(|(_, h)| ordinal(h)).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen,
            (0..VARIANTS).collect::<Vec<_>>(),
            "every variant needs a witness in every_halt()"
        );
        let declared_share = |a: &str, b: &str| {
            matches!(
                (a, b),
                ("Finisher(success)", "Shutdown") | ("Shutdown", "Finisher(success)")
            )
        };
        for (i, (na, a)) in halts.iter().enumerate() {
            let ca = halt_code(a);
            assert!(
                ca != 6 && ca != 13,
                "{na} answers {ca}, a code reserved for a run that never started"
            );
            for (nb, b) in &halts[i + 1..] {
                if declared_share(na, nb) {
                    continue;
                }
                assert_ne!(ca, halt_code(b), "{na} and {nb} share yantra_run code {ca}");
            }
        }
    }

    /// `F-004` falsifier for the WATERMARK. The export exists so the page can compare
    /// the tab's reading of the high water against the native runner's, so the thing
    /// under test is the ARITHMETIC — and FOUR states, because an instrument with two
    /// cannot tell "measures the water" from "reports the last store".
    #[test]
    fn the_water_is_the_highest_store_and_neither_a_lower_one_nor_the_uart_pulls_it_back() {
        let mut uart = Vec::new();
        let mut sink = Watermarked {
            out: &mut uart,
            high_water: 0,
        };

        // MUST BE ZERO BEFORE ANY STORE. Not "unset" — the sink's own zero is the
        // honest reading for a run that has stored nothing; `WATER`'s `None` is what
        // carries "no run", and the two must not be confused for one another.
        assert_eq!(sink.high_water, 0, "a sink that saw nothing is at zero");

        // MUST MOVE TO PAST-THE-END, not to the address. `at + width` is the count of
        // octets touched; a high water of `at` would under-report every store by its
        // own width and the native figure would be larger for the same program.
        sink.stored(0x1000, 8);
        assert_eq!(
            sink.high_water, 0x1008,
            "the water is the octet past the store, as in `yantra-run.rs:26`"
        );

        // MUST NOT RETRACT. This is the one that a `self.high_water = at + width`
        // would pass the arm above and fail here, and it is how the real reading goes
        // wrong: stores are not monotone in address, so the LAST store is almost never
        // the highest.
        sink.stored(0x20, 4);
        assert_eq!(
            sink.high_water, 0x1008,
            "a lower store must not pull the water back down"
        );

        // MUST NOT MOVE ON OUTPUT. The UART is the same sink and a `putc` is not a
        // store into the guest's RAM; counting one would make a chatty program look
        // like a hungry one.
        sink.putc(b'o');
        sink.putc(b'k');
        assert_eq!(
            sink.high_water, 0x1008,
            "writing the UART is not touching RAM"
        );
        assert_eq!(uart, b"ok", "and the octets still reach the UART buffer");
    }

    /// The narrowing is a rule, so it has its own arm: a water that does not fit a
    /// `u32` must read as ABSENT and never as a truncated small number, which is the
    /// one way this export could report a disagreement that is not in any program.
    #[test]
    fn a_water_past_u32_is_absent_rather_than_truncated() {
        assert_eq!(water_reading(0), Some(0), "zero is a reading");
        assert_eq!(
            water_reading(u32::MAX as usize),
            Some(u32::MAX),
            "the largest value the ABI can carry is still carried"
        );
        // `as u32` would answer `Some(0)` here — indistinguishable from a run that
        // stored nothing, and the page would read it as a measurement.
        assert_eq!(
            water_reading(u32::MAX as usize + 1),
            None,
            "one octet past the ABI's width is no reading at all"
        );
    }
}
