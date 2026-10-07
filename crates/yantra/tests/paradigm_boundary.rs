//! `W-212` — THE MACHINE BOUNDARY: statistics 22 and 23 of research/22 (§3.5 rule S5,
//! §7 ह‑14 → ल), and the ह‑junction as the machine sees it — every `आज्ञापनम्` (ecall).
//!
//! # What is measured, and what it is measured on
//!
//! **Two corpora were named.** `spec/*.sas` (48 programs, `spec/programs.tsv`) is the T0
//! corpus and every one of them is assembled here through the same four calls
//! `sadhana`'s `main` makes, at the load address the `tools/check-*.sh` that asserts about
//! it passes (`--स्थान ०षोड्८०२००००० ` for a program built above OpenSBI, the reset vector
//! otherwise), and run through **`yantra-run`'s own library path** — `Machine::load_elf`
//! with 1 MiB and a million steps, the two numbers in `src/bin/yantra-run.rs`. The
//! application (`atithi.sas`) is refused by `yantra-run` (F‑020) and is run the way
//! `yantra-host` runs it, through `yantra::host::host`. The binaries themselves are not
//! exec'd: a freshly linked binary on this machine waits minutes at `_dyld_start`, and the
//! library path is the same code with the same two numbers. What the binaries add — the
//! *composition* of the two streams in `main` — is measured by reading their source:
//! [`the_binaries_write_only_the_payload_to_stdout`].
//!
//! **The second corpus, `tests/corpus/t1/*.सस` (16 programs), is read by NO chain** — not
//! T0, not T1 (research/25 §1.4: `ॐ`, `॥ मण्डलम् ॥`, `फलम्`, 0 `आदाय`), and `W-238` retired the
//! dialect by decision. This header once said they were "executed by `sadhana::t1::nirvahana`";
//! they never were — the interpreter reads the `T1_SOURCES` (`tests/paradigm/pins.rs`)
//! `.t1` files in `crates/sadhana-t1/src/`, the self-hosting corpus, and none of the 16. The census
//! counts them and says so (`paradigm_boundary_t1_programs`,
//! `paradigm_boundary_t1_sas_readable 0` BY DESIGN).
//!
//! THIS PARAGRAPH SAID "18" UNTIL 2026-09-07 AND THE CORPUS HAD BEEN 20 SINCE THAT
//! MORNING. The pin's own history dates both steps — `18 -> 19` on 2026-09-04 when
//! `sanchaya.t1` joined, `19 -> 20` on 2026-09-07 when `shrinkhala.t1` did — so the
//! margin was wrong for three days while the assertion beside it was right the whole
//! time. It names the PIN now rather than a literal, because a figure in prose that
//! no assertion covers is a figure nothing can keep true: the count is measured by
//! `read_dir` over that directory, and any hand-copy of it goes stale on the next
//! source that lands.
//!
//! **The T1 corpus that DOES have a path onto `yantra` is the `.t1` sources**, since
//! `W-235`/`W-236` (a RISC‑V emitter from the T1 IR, in two twins) and `W-237`'s census
//! `tests/paradigm_encode.rs`, which runs each source lex → parse → resolve → typecheck → IR →
//! emit → `सङ्केतन` → link → load → run. Its measured numbers are pinned once in
//! `tests/paradigm/pins.rs` and reported here as `paradigm_boundary_t1_runnable_on_yantra`,
//! so the two censuses cannot disagree; [`the_t1_corpus_reaches_yantra_by_the_pinned_number`]
//! replaced `the_t1_corpus_has_no_path_onto_yantra_yet`, the test that flipped when the
//! emitter appeared.
//!
//! # Statistic 22 — host‑boundary purity (rule S5)
//!
//! Each run yields two byte streams, composed exactly as the binaries compose them: the
//! **payload** is what the machine put through [`yantra::Output`] (or the surface, for
//! the application), and the **diagnostics** are the lines `main` writes to stderr —
//! `halt: {halt:?}`, `{path}: {e}`, `ended:`, `scause:`, `UART:`. A run is impure when
//! (a) a payload line starts with one of the diagnostic prefixes the binaries write or
//! once wrote, (b) a diagnostic line has no such prefix — a byte that is not a diagnostic
//! reached stderr — or (c) a diagnostic line, its prefix stripped, is a payload line: the
//! `UART: <payload>` composition `issue-sassembly.md` was filed against. The refused case
//! is a program that deliberately writes `halt: Finisher { … }` to the UART:
//! [`a_program_that_writes_a_diagnostic_shaped_line_to_the_payload_is_impure`].
//!
//! # Statistic 23 — program end as re‑entry (ह‑14 → ल), in two NAMED configurations
//!
//! - **`bare`** — `yantra-run`'s machine: `Machine::run` returns a [`Halt`] and nothing
//!   is above it to re‑enter. Every end is a halt (finisher or `sbi_shutdown`) or a
//!   trap, a spin, or the step limit. Re‑entry is **0 by construction**, and that is the
//!   honest gap §7 names: on a bare machine ह at 14 really is the end.
//! - **`kernel`** — ADR‑0025's scheduler: `Kernel::preemptive(FREE, 9)`, the quantum
//!   `tests/scheduler.rs` uses, with the program spawned as a U‑mode process beside a
//!   [`lingerer`] that outlasts the budget. A program's end is a
//!   **re‑entry** when the scheduler got the hart back — the ledger shows the lingerer's
//!   slices *after* the program's last one; it is a **halt** when [`Ended::Stopped`]
//!   ended the schedule and nothing followed. Every `spec/` program is spawned this way,
//!   linked at the application address — the 47 boot proofs are not applications
//!   (ADR‑0015) and how they end under a kernel is exactly the measurement: a finisher
//!   store from U‑mode is a page fault, and the two kinds are told apart by whether
//!   anything ran afterwards. A `jal x0, .` from U‑mode — the idle idiom every boot
//!   proof parks on — is since `W‑219` an **idle**, not a halt: the process retires its
//!   jump once a tick and the clock takes every slice, so the program has no end on the
//!   ledger (kind `budget`, flagged `idle`) and the lingerer keeps running beside it.
//!   Before `W‑219` those eight programs stopped the whole machine (83% re‑entry,
//!   research/22 §8). The refused case is the bare machine, where the same word is
//!   [`Halt::SpinForever`] and 0% re‑entry is by construction —
//!   [`under_the_kernel_a_self_jump_idles_and_on_the_bare_machine_it_halts`] holds both
//!   halves, and [`the_kernel_reentry_fraction_is_a_measurement`] keeps it from being true
//!   by construction: a U‑mode word the kernel cannot resume still halts the schedule.
//!
//! # The ह‑junction at the syscall
//!
//! Every ecall the corpus executes is observed one instruction at a time: the word at
//! `pc` is fetched through the same Sv39 tables the machine uses, and when it is `ecall`
//! the call number (`a7`), the octets the call consumes and the octets it emits are
//! recorded, together with which channel the octets went to. Three interfaces meet at
//! the same instruction and are kept apart: `sbi` (S‑mode, the firmware this
//! interpreter is), `abi` (U‑mode under the kernel, `spec/application-abi.tsv`), and
//! `own` (U‑mode on the bare machine, answered by the program's own trap seam — the
//! census does not know that ABI and records no octets in). Rule S5 at the syscall: no
//! call feeds both channels (`paradigm_boundary_ecall_both_channels`, expected 0).

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{LOAD_ADDRESS, object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;
use yantra::host::{FREE, SCREEN, host};
use yantra::process::Kernel;
use yantra::supervisor::{Ended, Surface, install};
use yantra::{Csrs, FINISHER, Halt, Machine, Output, Privilege, UART};

#[path = "paradigm/pins.rs"]
mod pins;

// --- the numbers the binaries use, taken from the binaries ------------------------------

/// The limits `yantra-run` gives a program, IMPORTED rather than restated —
/// `W-262` made `yantra::DEFAULT_RAM` and `DEFAULT_STEPS` the one statement,
/// after five copies of these numbers drifted behind a comment claiming they
/// matched. A test needing different limits should say so and why.
const BARE_RAM: usize = yantra::DEFAULT_RAM;
/// The step budget, from the same one statement.
const BARE_BUDGET: u64 = yantra::DEFAULT_STEPS;
/// `yantra-host`: `RAM`.
const HOST_RAM: usize = 1 << 22;
/// `yantra-host`: `BUDGET`.
const HOST_BUDGET: u64 = 200;
/// Where OpenSBI hands an S‑mode payload the machine, and where the SBI clients are built.
const SBI_PAYLOAD: u64 = 0x8020_0000;
/// The kernel configuration's RAM: 32 MiB, so that `compositor.sas`'s address space —
/// too large for `yantra-run`'s 20 MiB and for `tests/scheduler.rs`'s 4 MiB, and at
/// 16 MiB leaving no frame for the process behind it — fits with room.
///
/// `W-262` RAISED THE RUNNER FROM 1 MiB TO 20 MiB AND THIS MARGIN QUOTED THE OLD
/// NUMBER. The reason still holds — 32 MiB is above both — but the figure it
/// argues from had changed, and nothing checks a mention: this const names the
/// runner without claiming to mirror it, which is exactly why `t1_transcriptions`
/// leaves it alone. A stale mention is the cost of that, and is paid by hand.
const KERNEL_RAM: usize = 1 << 25;
/// The quantum, `tests/scheduler.rs`'s: odd, and small enough that a schedule is short.
const QUANTUM: u64 = 9;
/// The whole schedule's budget. The [`lingerer`] outlasts it, so every schedule runs to
/// the budget and what follows a program's end is always on the ledger.
const KERNEL_BUDGET: u64 = 50_000;
/// Physical RAM's base, and the supervisor's word.
const BASE: u64 = 0x8000_0000;
/// The `ecall` word.
const ECALL: u32 = 0x0000_0073;
/// What [`lingerer`] would report if it ever got there; not 0, so "it exited" is not a
/// zeroed field.
const EXIT_STATUS: u64 = 37;

/// The prefixes `yantra-run` and `yantra-host` write — or, before `issue-sassembly.md`,
/// wrote — at the head of a diagnostic line. A payload line that starts with one of these
/// is a diagnostic in the payload stream, whoever put it there.
const DIAGNOSTIC_PREFIXES: &[&str] = &[
    "halt: ",
    "ended: ",
    "scause: ",
    "UART: ",
    "SURFACE: ",
    "note: ",
    "usage: ",
    // `yantra-run.rs`, always on: "steps: {} executed instructions" — the
    // benchmark metric row `T-102` ratified. Enrolled in the same commit that
    // added it, because the margin below already recorded `ram: high water`
    // being added WITHOUT enrolment and failing this very census. Reading that
    // note and then repeating the omission is the one outcome it was written to
    // prevent.
    "steps: ",
    // `yantra-run.rs:53`, `--version` as the sole argument: "version: {commit}",
    // the W-347 build stamp. Enrolled IN THE SAME COMMIT that added it — which is
    // what the `steps:` note above demands, and which I failed to do on the first
    // attempt: the stamp went in as a bare `{}` with no prefix at all, and this
    // census refused it at the second assertion after the first had already refused
    // it for being on stdout. Two refusals from one guard, both correct.
    //
    // It is on STDERR rather than stdout because in this binary stdout carries the
    // guest's payload and nothing else. `t1_image --version` stays on stdout: that
    // binary has 45 `println!`s of its own, reports `build:` and `write:` there, and
    // no purity contract covers it — this census names only `yantra-run.rs` and
    // `yantra-host.rs`.
    "version: ",
    // `yantra-run.rs`, `--source-stamp` (W-381): the content stamp the gate compares.
    "source: ",
    // `yantra-run.rs`, `YANTRA_VERDICT` unwritable (W-381): "verdict: {path}: {e}".
    // Enrolled in the commit that added it, as the notes above require.
    "verdict: ",
    // `yantra-run.rs:85`, behind `YANTRA_WATERMARK`: "ram: high water {} of {}
    // octets". A real diagnostic on the real stream, added without being enrolled
    // here — so this census reported an unrecognised stderr line and the step
    // FAILED. That step is one of the reds that kept `salvage()` from landing.
    //
    // This list is a spelling census, so it goes stale exactly when someone adds
    // a diagnostic correctly. That is its cost and it is worth paying: the thing
    // it catches is a payload line disguised as a diagnostic, which no dataflow
    // check in a test would see either.
    "ram: ",
    // THE INPUT CHANNEL (2026-09-21, `yantra::input`): the file the host places
    // in RAM before the first instruction. Its report and its three refusals all
    // lead with this one prefix — they first shipped with three different
    // openings ("YANTRA_INPUT …", "input refused: …", and one reflowed across
    // lines where this census cannot read it) and this step went red on main.
    // One prefix is what lets this census tell a diagnostic from a leaked
    // payload line, which is the boundary this file exists to hold: the host and
    // the program each keep their own channel, and stdout carries the payload
    // alone. A multi-line `eprintln!` still escapes the scan below — its literal
    // is on the next line — so the reflowed one was given the prefix by hand.
    "input: ",
    // `W-344` made the exit code the halt and wrote two failure diagnostics
    // under this prefix. The step-limit one reflows its literal onto the next
    // line and escaped the scan; the finisher one does not, and this census
    // reported an unrecognised stderr line — the standing red of cycle 1006.
    // Enrolled late, after the fact, which is exactly the omission the `steps: `
    // margin above was written to prevent.
    "yantra-run: ",
    // `yantra-run.rs`, always on: the argument-interface report and its
    // not-handed-over refusal. Both sit behind match arms, so the scan below —
    // which takes only lines that BEGIN with `eprintln!(` — never sees them;
    // they are enrolled because they are real diagnostics on the real stream,
    // and this list is the census of those, not of what the scan happens to reach.
    "args: ",
    // `W-371`: `--events <log>`'s report, its load-time refusals and its two replay
    // refusals (shorter, longer). Enrolled in the commit that added them.
    "events: ",
    // `W-376`: `threads: N` whenever an image declares SASTHRDS (the line
    // `tools/fixpoint.sh` refuses in a Stage 2 log), the thread host's report, each
    // thread's end, the end rule's verdict and the load-time refusals — one prefix
    // for all of them. Enrolled in the commit that added them.
    "threads: ",
    // `F-022` (b): a Saṃpuṭa archive launched directly — its verification report, its
    // S1..S8 refusals, the env-conflict and YANTRA_STEPS refusals and the step-cap clamp.
    // Enrolled in the commit that added them.
    "smp: ",
    // `W-377`: the socket device's host — `socket: listening on …`, the accepted client,
    // a send failure (said by the host, never told to the program), the address and
    // `--listen` refusals, the closing `socket: R octets received in K records, …` account
    // and the touched-with-no-device line `tools/fixpoint.sh` refuses in a Stage 2 log.
    // Enrolled in the commit that added them.
    "socket: ",
    // The deferred virtio completion (`YANTRA_VIRTIO_DEFER`): the line saying a run's
    // completions are deferred, and the refusal of a value that is not decimal digits.
    // Enrolled in the commit that added them.
    "virtio: ",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

// --- assembling the corpus the way the toolchain does -----------------------------------

/// Assemble and link `sources` at `load` — the four calls `sadhana`'s `main` makes, in its
/// order, with every refusal returned as the string `main` would print.
fn build(sources: &[(String, String)], load: u64) -> Result<Vec<u8>, String> {
    let mut objects = Vec::new();
    for (name, source) in sources {
        let program = assemble_program(source).map_err(|e| format!("{name}: {}", e.join("; ")))?;
        let (text, pending) = encode_object(&program).map_err(|e| format!("{name}: {e:?}"))?;
        let bytes = object(
            &text,
            &program,
            &pending,
            None,
            &layout_addresses(&program, Target::Uncompressed),
        );
        objects.push(read(&bytes).ok_or_else(|| format!("{name}: the object does not read back"))?);
    }
    let image = link_at(&objects, load).map_err(|e| e.join("; "))?;
    Ok(write_debuggable_at(
        &image.text,
        &image.data,
        &image.table,
        image.bss,
        &[],
        load,
    ))
}

/// One program of `spec/`, with the class `spec/programs.tsv` gives it.
#[derive(Debug, Clone)]
struct Program {
    /// `namaste.sas`.
    name: String,
    /// `app` or `boot-proof`.
    class: String,
    /// The files linked together — one, except for the pair `tools/check-link.sh` links.
    sources: Vec<(String, String)>,
    /// Where the tooling builds it.
    load: u64,
}

/// `spec/programs.tsv`, name → class.
fn classes() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join("spec/programs.tsv")).expect("programs.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && l.contains(".sas\t"))
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?.to_string(), f.next()?.to_string()))
        })
        .collect()
}

/// The programs a `tools/check-*.sh` builds above OpenSBI. Read from the scripts rather
/// than typed: a script whose commands pass `--स्थान ०षोड्८०२००००० ` names the programs
/// it builds there. Comment lines are skipped — `check-roundtrip.sh` *mentions* the
/// address in a comment and builds nothing at it.
fn sbi_clients() -> Vec<String> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(root().join("tools")).expect("tools/") {
        let path = entry.expect("entry").path();
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        if !file.starts_with("check-") || !file.ends_with(".sh") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read a check script");
        let code: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect();
        if !code.iter().any(|l| l.contains("८०२०००००")) {
            continue;
        }
        for line in code {
            for piece in line.split(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
                if let Some(rest) = piece.strip_prefix("spec/")
                    && rest.ends_with(".sas")
                {
                    names.push(rest.to_string());
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Every program of `spec/`, sorted by name, with its sources and its load address.
fn corpus() -> Vec<Program> {
    let classes = classes();
    let sbi = sbi_clients();
    let spec = root().join("spec");
    let mut names: Vec<String> = std::fs::read_dir(&spec)
        .expect("spec/")
        .map(|e| e.expect("entry").file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".sas"))
        .collect();
    names.sort();
    let source = |n: &str| {
        (
            n.to_string(),
            std::fs::read_to_string(spec.join(n)).unwrap_or_else(|e| panic!("{n}: {e}")),
        )
    };
    names
        .iter()
        .map(|n| {
            // `tools/check-link.sh`: `namaste-main.sas` calls a routine `lib-mudraka.sas`
            // exports, and is linked with it.
            let sources = if n == "namaste-main.sas" {
                vec![source(n), source("lib-mudraka.sas")]
            } else {
                vec![source(n)]
            };
            Program {
                name: n.clone(),
                class: classes
                    .get(n)
                    .cloned()
                    .unwrap_or_else(|| "unclassified".into()),
                sources,
                load: if sbi.contains(n) {
                    SBI_PAYLOAD
                } else {
                    LOAD_ADDRESS
                },
            }
        })
        .collect()
}

/// The two addresses `spec/application-load.tsv` links applications at.
fn application_loads() -> [u64; 2] {
    let tsv = std::fs::read_to_string(root().join("spec/application-load.tsv"))
        .expect("application-load.tsv");
    let mut addrs = vec![];
    for line in tsv.lines() {
        if line.starts_with("0x") {
            let hex = line.split('\t').next().unwrap().replace('_', "");
            addrs.push(u64::from_str_radix(&hex[2..], 16).unwrap());
        }
    }
    [addrs[0], addrs[1]]
}

// --- the two streams, and purity ---------------------------------------------------------

/// Which of the two channels a set of octets reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    /// Stdout — the program's own bytes.
    Payload,
    /// Stderr — the machine's account of the run.
    Diagnostic,
}

/// One run's two streams, composed as the binary composes them.
#[derive(Debug, Clone, Default)]
struct Streams {
    payload: Vec<u8>,
    diagnostics: Vec<u8>,
}

impl Streams {
    fn diagnose(&mut self, line: &str) {
        self.diagnostics.extend_from_slice(line.as_bytes());
        self.diagnostics.push(b'\n');
    }
}

fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect()
}

/// Does `line` begin the way a diagnostic line of the binaries begins? `program` is the
/// name whose `{path}: {e}` refusal line is also a diagnostic.
fn diagnostic_shaped(line: &[u8], program: &str) -> bool {
    DIAGNOSTIC_PREFIXES
        .iter()
        .any(|p| line.starts_with(p.as_bytes()))
        || (!program.is_empty() && line.starts_with(format!("{program}: ").as_bytes()))
}

/// Every way the two streams of one run share bytes. Empty is pure.
fn impurities(streams: &Streams, program: &str) -> Vec<String> {
    let mut found = Vec::new();
    let payload = lines(&streams.payload);
    for line in &payload {
        if diagnostic_shaped(line, program) {
            found.push(format!(
                "payload carries a diagnostic-shaped line: {:?}",
                String::from_utf8_lossy(line)
            ));
        }
    }
    for line in lines(&streams.diagnostics) {
        if !diagnostic_shaped(line, program) {
            found.push(format!(
                "diagnostics carry a line that is not a diagnostic: {:?}",
                String::from_utf8_lossy(line)
            ));
            continue;
        }
        for prefix in DIAGNOSTIC_PREFIXES {
            if let Some(rest) = line.strip_prefix(prefix.as_bytes())
                && !rest.is_empty()
                && payload.contains(&rest)
            {
                found.push(format!(
                    "diagnostics carry a payload line under {prefix:?}: {:?}",
                    String::from_utf8_lossy(rest)
                ));
            }
        }
    }
    found
}

// --- program ends --------------------------------------------------------------------------

/// How a program's run ended, in the vocabulary of §7's table.
#[derive(Debug, Clone, PartialEq, Eq)]
enum End {
    /// ह‑14 on a bare machine: the finisher, or `sbi_shutdown`. The machine stopped
    /// because the program asked it to. Under the kernel: the machine stopped, and the
    /// scheduler with it.
    Halt(String),
    /// ह‑14 → ल: the program ended and the scheduler took the hart back.
    ReEntry(String),
    /// The machine stopped on something the program did not ask for.
    Trap(String),
    /// `jal x0, .` — the program never ends and says so.
    Spin,
    /// The budget ran out with the program still running.
    Budget,
    /// The image did not load, so nothing ran.
    Unloaded,
}

impl End {
    fn kind(&self) -> &'static str {
        match self {
            End::Halt(_) => "halt",
            End::ReEntry(_) => "reentry",
            End::Trap(_) => "trap",
            End::Spin => "spin",
            End::Budget => "budget",
            End::Unloaded => "unloaded",
        }
    }
    fn detail(&self) -> String {
        match self {
            End::Halt(s) | End::ReEntry(s) | End::Trap(s) => s.clone(),
            _ => String::new(),
        }
    }
}

/// A bare machine's [`Halt`] read as an [`End`].
fn end_of_halt(halt: &Halt) -> End {
    match halt {
        Halt::Finisher { status, .. } => End::Halt(format!("finisher status={status:?}")),
        Halt::Shutdown { .. } => End::Halt("sbi_shutdown".into()),
        Halt::SpinForever { .. } => End::Spin,
        Halt::StepLimit { .. } => End::Budget,
        other => {
            let name = format!("{other:?}");
            End::Trap(name.split([' ', '{']).next().unwrap_or("").to_string())
        }
    }
}

// --- observing every ecall ---------------------------------------------------------------

/// One `आज्ञापनम्` the machine executed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Call {
    /// The program that made it.
    program: String,
    /// `sbi` from S‑mode; `abi` from U‑mode under the kernel; `own` from U‑mode on the
    /// bare machine, where the program's own seam answers. The same `a7` means three
    /// things.
    interface: &'static str,
    /// `a7`.
    number: u64,
    /// Octets of the caller's registers or memory the call consumed.
    bytes_in: u64,
    /// Octets that reached a channel.
    bytes_out: u64,
    /// The channels the call's effect reached.
    channels: Vec<Channel>,
}

/// The octets a call consumes, by the tables that define it. SBI legacy (S‑mode):
/// `set_timer` reads `a0` (8), `console_putchar` reads one octet of `a0`, `shutdown`
/// nothing. `spec/application-abi.tsv` (U‑mode under the kernel): `EXIT` reads `a0`
/// (8), `WRITE` reads `a2` octets of memory, `SEND`/`RECV` carry one word (8). A
/// program's own seam has no table here, and 0 is recorded rather than a guess.
fn bytes_in(interface: &str, number: u64, x: &[u64; 32]) -> u64 {
    match (interface, number) {
        ("sbi", 0x00) | ("abi", 0 | 2 | 3) => 8,
        ("sbi", 0x01) => 1,
        ("abi", 1) => x[12],
        _ => 0,
    }
}

/// Read `width` octets of physical memory, or `None` outside RAM.
fn physical(m: &Machine, pa: u64, width: usize) -> Option<u64> {
    let at = usize::try_from(pa.checked_sub(m.base)?).ok()?;
    if at + width > m.mem.len() {
        return None;
    }
    let mut v = 0u64;
    for (i, &b) in m.mem[at..at + width].iter().enumerate() {
        v |= u64::from(b) << (8 * i);
    }
    Some(v)
}

/// The Sv39 walk `satp` names, for a fetch. `None` when the address is not mapped — the
/// machine will fault on it and this census does not need to know why.
fn translate(m: &Machine, satp: u64, va: u64) -> Option<u64> {
    if satp >> 60 == 0 {
        return Some(va);
    }
    let mut table = (satp & ((1 << 44) - 1)) << 12;
    for level in (0..3).rev() {
        let index = (va >> (12 + 9 * level)) & 0x1ff;
        let pte = physical(m, table + index * 8, 8)?;
        if pte & 1 == 0 {
            return None;
        }
        if pte & 0xe != 0 {
            let ppn = (pte >> 10) & ((1 << 44) - 1);
            let span = 1u64 << (12 + 9 * level);
            return Some((ppn << 12 & !(span - 1)) | (va & (span - 1)));
        }
        table = (pte >> 10) << 12;
    }
    None
}

/// The instruction word at `va` under `satp`.
fn fetch_at(m: &Machine, satp: u64, va: u64) -> Option<u32> {
    let pa = translate(m, satp, va)?;
    physical(m, pa, 4).map(|w| w as u32)
}

/// The instruction word the machine will execute next.
fn fetch(m: &Machine) -> Option<u32> {
    fetch_at(m, m.csr.satp, m.pc)
}

/// What a bare run produced: the streams `yantra-run` would have written, the end, every
/// ecall seen, and the octets that crossed the junction by `sbi_console_putchar` alone.
struct Bare {
    streams: Streams,
    end: End,
    calls: Vec<Call>,
    console: Vec<u8>,
}

/// A bare run through the binary's own path, observed one instruction at a time.
fn run_bare(program: &str, elf: &[u8]) -> Bare {
    let mut streams = Streams::default();
    let mut m = match Machine::load_elf(elf, BARE_RAM) {
        Ok(m) => m,
        Err(e) => {
            streams.diagnose(&format!("{program}: {e}"));
            return Bare {
                streams,
                end: End::Unloaded,
                calls: Vec::new(),
                console: Vec::new(),
            };
        }
    };
    let mut calls = Vec::new();
    let mut console = Vec::new();
    let mut halt = None;
    for _ in 0..BARE_BUDGET {
        let is_ecall = fetch(&m) == Some(ECALL);
        let before = streams.payload.len();
        let x = m.x;
        let interface = if m.mode == Privilege::User {
            "own"
        } else {
            "sbi"
        };
        let stopped = m.step(&mut streams.payload);
        if is_ecall {
            let out = (streams.payload.len() - before) as u64;
            if interface == "sbi" && x[17] == 1 && out == 1 {
                console.push(x[10] as u8);
            }
            let mut channels = Vec::new();
            if out > 0 {
                channels.push(Channel::Payload);
            }
            if stopped.is_some() {
                channels.push(Channel::Diagnostic);
            }
            calls.push(Call {
                program: program.to_string(),
                interface,
                number: x[17],
                bytes_in: bytes_in(interface, x[17], &x),
                bytes_out: out,
                channels,
            });
        }
        if let Some(h) = stopped {
            halt = Some(h);
            break;
        }
    }
    let halt = halt.unwrap_or(Halt::StepLimit { pc: m.pc });
    streams.diagnose(&format!("halt: {halt:?}"));
    Bare {
        streams,
        end: end_of_halt(&halt),
        calls,
        console,
    }
}

/// The application run the way `yantra-host` runs it: the surface is the payload and the
/// three stderr lines are the diagnostics.
fn run_hosted(program: &str, elf: &[u8]) -> (Streams, End) {
    let mut streams = Streams::default();
    match host(elf, HOST_RAM, HOST_BUDGET) {
        Ok(h) => {
            streams.payload.clone_from(&h.surface);
            if !h.uart.is_empty() {
                streams.diagnose(&format!("UART: {}", String::from_utf8_lossy(&h.uart)));
                streams.diagnose(
                    "note: an application cannot address a device — a non-empty UART is a finding",
                );
            }
            streams.diagnose(&format!("ended: {:?}", h.ended));
            streams.diagnose(&format!("scause: {}", h.scause));
            let end = match h.ended {
                Ended::Exited { status } => End::ReEntry(format!("exited status={status}")),
                Ended::Faulted { cause, .. } => End::ReEntry(format!("faulted cause={cause}")),
                Ended::Stopped(h) => End::Trap(format!("{h:?}")),
                Ended::OutOfBudget | Ended::Preempted => End::Budget,
            };
            (streams, end)
        }
        Err(e) => {
            streams.diagnose(&format!(
                "{program}: this image could not be loaded as an application: {e}"
            ));
            (streams, End::Unloaded)
        }
    }
}

// --- the kernel configuration ----------------------------------------------------------

fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    0x13 | rd << 7 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn lui(rd: u32, imm20: u32) -> u32 {
    0x37 | rd << 7 | imm20 << 12
}
fn jal(rd: u32, imm: i32) -> u32 {
    let i = imm as u32;
    0x6f | rd << 7
        | ((i >> 12) & 0xff) << 12
        | ((i >> 11) & 1) << 20
        | ((i >> 1) & 0x3ff) << 21
        | ((i >> 20) & 1) << 31
}
/// `bne rs1, rs2, imm` — B‑type, its immediate scattered like `jal`'s.
fn bne(rs1: u32, rs2: u32, imm: i32) -> u32 {
    let i = imm as u32;
    0x63 | ((i >> 11) & 1) << 7
        | ((i >> 1) & 0xf) << 8
        | 1 << 12
        | rs1 << 15
        | rs2 << 20
        | ((i >> 5) & 0x3f) << 25
        | ((i >> 12) & 1) << 31
}
/// `s{b,w} rs2, imm(rs1)` — S‑type, `funct3` 0 for a byte and 2 for a word.
fn store(funct3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let i = imm as u32;
    0x23 | (i & 0x1f) << 7 | funct3 << 12 | rs1 << 15 | rs2 << 20 | ((i >> 5) & 0x7f) << 25
}

/// An ELF from the project's own writer holding `words`, linked at `load`.
fn image(words: &[u32], load: u64) -> Vec<u8> {
    let text: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    write_debuggable_at(&text, &[], &[], 0, &[], load)
}

/// The process queued behind every program: counts down from 65536 and then exits — but
/// 131072 instructions is more than [`KERNEL_BUDGET`], so within one schedule it never
/// gets there. It takes every slice the scheduler offers it, and a slice of its own on
/// the ledger *after* the program's last is the evidence that the scheduler got the hart
/// back. `t0` is its odometer.
fn lingerer(load: u64) -> Vec<u8> {
    image(
        &[
            lui(5, 0x10),                    // t0 = 65536
            addi(5, 5, -1),                  // t0 -= 1
            bne(5, 0, -4),                   // back to it while t0 != 0
            addi(17, 0, 0),                  // a7 = EXIT
            addi(10, 0, EXIT_STATUS as i32), // a0 = status
            ECALL,
        ],
        load,
    )
}

/// A machine with the supervisor's one `sret` installed at [`BASE`].
fn kernel_machine() -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: 0,
        base: BASE,
        mem: vec![0; KERNEL_RAM],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    m.csr.sstatus = 1 << 8;
    install(&mut m, BASE).expect("the supervisor's word is inside RAM");
    m
}

/// What one program did under the kernel configuration.
#[derive(Debug, Clone)]
struct Scheduled {
    end: End,
    /// Slices of the program that ended on the clock (ADR‑0025).
    on_clock: u64,
    /// Slices of the program that ended because it asked (call 0).
    on_request: u64,
    /// Slices of the program that ended on a fault.
    on_fault: u64,
    /// Slices of the lingerer on the ledger after the program's last slice.
    after: u64,
    /// The program had no end on the ledger and was parked on a `jal x0, .` when the
    /// budget ran out — the idle idiom under a scheduler (`W‑219`). Only meaningful for
    /// [`End::Budget`]; a program that exited or faulted is not parked anywhere.
    idle: bool,
}

/// Spawn `elf` as a U‑mode process under ADR‑0025's preemptive kernel with a
/// [`lingerer`] beside it, run the schedule, and read the program's end off the ledger.
///
/// The lingerer is spawned first — it is six words and always fits, so a refusal here is
/// always the program's — and takes the first quantum; the order of spawning changes
/// nothing measured, since every count below is by pid and by position on the ledger.
fn run_kernel(elf: &[u8], load: u64) -> Result<Scheduled, String> {
    let mut m = kernel_machine();
    let mut k = Kernel::preemptive(FREE, QUANTUM);
    let other = k
        .spawn(&mut m, &lingerer(load), Vec::new(), Vec::new())
        .map_err(|e| format!("the lingerer did not spawn: {e}"))?;
    let pid = k.spawn(&mut m, elf, vec![Surface::writable(SCREEN)], Vec::new())?;
    let mut uart = Vec::new();
    let ledger = k.schedule(&mut m, KERNEL_BUDGET, &mut uart);
    let mut on_clock = 0;
    let mut on_request = 0;
    let mut on_fault = 0;
    let mut last = None;
    let mut after = 0;
    for slice in &ledger {
        if slice.pid == pid {
            match &slice.ended {
                Ended::Preempted => on_clock += 1,
                Ended::Exited { .. } => on_request += 1,
                Ended::Faulted { .. } => on_fault += 1,
                _ => {}
            }
            if !matches!(slice.ended, Ended::Preempted) {
                last = Some(slice.ended.clone());
            }
        } else if slice.pid == other && last.is_some() {
            after += 1;
        }
    }
    let end = match last {
        Some(Ended::Exited { status }) => End::ReEntry(format!("exited status={status}")),
        Some(Ended::Faulted { cause, .. }) => End::ReEntry(format!("faulted cause={cause}")),
        Some(Ended::Stopped(h)) => End::Halt(format!("stopped {h:?}")),
        Some(Ended::OutOfBudget) | None => End::Budget,
        Some(Ended::Preempted) => unreachable!("a preempted slice is never the last"),
    };
    // Parked: the word under the saved program counter is the self‑jump. Read through
    // the process's own tables, the way every fetch in this census is.
    let p = &k.processes[pid];
    let idle = end == End::Budget && fetch_at(&m, p.satp, p.sepc) == Some(jal(0, 0));
    Ok(Scheduled {
        end,
        on_clock,
        on_request,
        on_fault,
        after,
        idle,
    })
}

/// The application under the cooperative kernel, two steps per `Kernel::run` — the
/// supervisor's `sret` and one instruction of the process — so that every ecall it makes
/// is observed. The surface is the payload channel and the kernel's `Ended` the diagnostic
/// one.
///
/// `Kernel::run` re‑enters through `stvec` on every call and resumes at the saved program
/// counter, which since `W‑220` is where the hart stood when the budget ran out — so a
/// two‑step run followed by another is the program executed once, in order. (`W‑212`
/// found the saved counter stale here and set it by hand between calls; the fix removed
/// the hand.)
fn observe_hosted_calls(program: &str, elf: &[u8]) -> Vec<Call> {
    let mut m = kernel_machine();
    let mut k = Kernel::new(FREE);
    let pid = k
        .spawn(&mut m, elf, vec![Surface::writable(SCREEN)], Vec::new())
        .expect("the application spawns");
    let mut uart: Vec<u8> = Vec::new();
    let mut calls = Vec::new();
    for _ in 0..HOST_BUDGET {
        let p = &k.processes[pid];
        let is_ecall = fetch_at(&m, p.satp, p.sepc) == Some(ECALL);
        let x = p.x;
        let before = p.surface(SCREEN).map_or(0, |s| s.bytes.len());
        let ended = k.run(&mut m, pid, 2, &mut uart);
        if is_ecall {
            let after = k.processes[pid]
                .surface(SCREEN)
                .map_or(0, |s| s.bytes.len());
            let out = (after - before) as u64;
            let mut channels = Vec::new();
            if out > 0 {
                channels.push(Channel::Payload);
            }
            if !matches!(ended, Ended::OutOfBudget) {
                channels.push(Channel::Diagnostic);
            }
            calls.push(Call {
                program: program.to_string(),
                interface: "abi",
                number: x[17],
                bytes_in: bytes_in("abi", x[17], &x),
                bytes_out: out,
                channels,
            });
        }
        if !matches!(ended, Ended::OutOfBudget) {
            break;
        }
    }
    calls
}

// --- the census ----------------------------------------------------------------------------

/// **The census.** `cargo test -p yantra --test paradigm_boundary -- --ignored --nocapture`
/// prints one `METRIC paradigm_boundary_<stat> <value>` line per statistic, and a `RUN`,
/// `END`, `KERNEL`, `CALLS`, `TIME` or `OFFENDER` line per program where a number needs
/// its list.
#[test]
#[ignore = "a census, not a check: run with --ignored --nocapture and read the METRIC lines"]
#[allow(clippy::too_many_lines)]
fn census_of_the_machine_boundary() {
    let mut report = String::new();
    let clock = Instant::now();
    let corpus = corpus();
    let t1: Vec<PathBuf> = std::fs::read_dir(root().join("tests/corpus/t1"))
        .expect("tests/corpus/t1")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "सस"))
        .collect();

    // -- build --
    let mut built: Vec<(Program, Vec<u8>)> = Vec::new();
    let mut unbuilt = Vec::new();
    for p in &corpus {
        match build(&p.sources, p.load) {
            Ok(elf) => built.push((p.clone(), elf)),
            Err(e) => unbuilt.push((p.name.clone(), e)),
        }
    }
    let _ = writeln!(report, "TIME build {:.1}s", clock.elapsed().as_secs_f64());

    // -- 22: purity, and the ह‑14 ends on the bare machine, and every ecall --
    let mut runs: Vec<(String, &'static str, Streams, End)> = Vec::new();
    let mut calls: Vec<Call> = Vec::new();
    let mut console: Vec<u8> = Vec::new();
    let mut refused_applications = 0;
    let app_loads = application_loads();
    for (p, elf) in &built {
        if p.class == "app" {
            // F‑020: `yantra-run` refuses the image; `yantra-host` is the runner.
            refused_applications += 1;
            for at in app_loads {
                let elf = build(&p.sources, at).expect("the application builds");
                let (streams, end) = run_hosted(&p.name, &elf);
                runs.push((format!("{}@{at:#x}", p.name), "hosted", streams, end));
                calls.extend(observe_hosted_calls(&p.name, &elf));
            }
            continue;
        }
        let bare = run_bare(&p.name, elf);
        runs.push((p.name.clone(), "bare", bare.streams, bare.end));
        calls.extend(bare.calls);
        console.extend(bare.console);
    }
    let _ = writeln!(
        report,
        "TIME bare+hosted {:.1}s",
        clock.elapsed().as_secs_f64()
    );

    let mut impure = Vec::new();
    for (name, config, streams, end) in &runs {
        let program = name.split('@').next().unwrap();
        let found = impurities(streams, program);
        let _ = writeln!(
            report,
            "RUN {config} {name} payload={} diagnostics={} end={} {}",
            streams.payload.len(),
            streams.diagnostics.len(),
            end.kind(),
            end.detail()
        );
        if !found.is_empty() {
            for f in &found {
                let _ = writeln!(report, "OFFENDER purity {name} {f}");
            }
            impure.push(name.clone());
        }
    }
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_t0_programs {}",
        corpus.len()
    );
    let _ = writeln!(report, "METRIC paradigm_boundary_t0_built {}", built.len());
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_t0_unbuilt {}",
        unbuilt.len()
    );
    for (name, e) in &unbuilt {
        let _ = writeln!(report, "OFFENDER unbuilt {name} {e}");
    }
    let _ = writeln!(report, "METRIC paradigm_boundary_t1_programs {}", t1.len());
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_t1_sas_readable 0 # BY DESIGN: a retired dialect read by no \
         chain (research/25 §1.4a, W-238)"
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_t1_sources {} # crates/sadhana-t1/src/*.t1, the self-hosting corpus",
        pins::T1_SOURCES
    );
    // REPORTED BESIDE IT, NEVER INSTEAD OF IT. The line above counts the
    // directory; every pin under it was measured over the authored subset. A
    // reader who sees only the first number reads the assembled count against
    // the wrong denominator, which is the failure the census's own stderr
    // notice exists to prevent.
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_t1_censused {} # authored only: {} generated from spec/ ({}) are \
         skipped by the census's full walk",
        pins::T1_CENSUSED,
        pins::T1_GENERATED.len(),
        pins::T1_GENERATED.join(", ")
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_t1_runnable_on_yantra {} # measured by tests/paradigm_encode.rs \
         (W-237), pinned in tests/paradigm/pins.rs: {} assemble, {} run to a finisher halt",
        pins::T1_ON_YANTRA,
        pins::T1_ASSEMBLED,
        pins::T1_ON_YANTRA
    );
    let _ = writeln!(report, "METRIC paradigm_boundary_runs {}", runs.len());
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_runs_bare {}",
        runs.iter().filter(|r| r.1 == "bare").count()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_runs_hosted {}",
        runs.iter().filter(|r| r.1 == "hosted").count()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_refused_applications {refused_applications}"
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_shared_stream_runs {}",
        impure.len()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_shared_stream_fraction {:.4}",
        impure.len() as f64 / runs.len().max(1) as f64
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_payload_bytes {}",
        runs.iter().map(|r| r.2.payload.len()).sum::<usize>()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_diagnostic_bytes {}",
        runs.iter().map(|r| r.2.diagnostics.len()).sum::<usize>()
    );

    // -- 23, bare: every end, by kind --
    let bare: Vec<&End> = runs
        .iter()
        .filter(|r| r.1 == "bare")
        .map(|r| &r.3)
        .collect();
    let count = |kind: &str| bare.iter().filter(|e| e.kind() == kind).count();
    for kind in ["halt", "trap", "spin", "budget", "unloaded", "reentry"] {
        let _ = writeln!(
            report,
            "METRIC paradigm_boundary_end_bare_{kind} {}",
            count(kind)
        );
    }
    let mut by_detail: BTreeMap<String, usize> = BTreeMap::new();
    for e in &bare {
        if matches!(e, End::Halt(_) | End::Trap(_)) {
            *by_detail
                .entry(format!("{}:{}", e.kind(), e.detail()))
                .or_default() += 1;
        }
    }
    for (detail, n) in &by_detail {
        let _ = writeln!(report, "END bare {detail} {n}");
    }
    for (name, _, _, end) in runs.iter().filter(|r| r.1 == "bare") {
        if !matches!(end, End::Halt(_)) {
            let _ = writeln!(
                report,
                "OFFENDER bare-not-a-halt {name} {} {}",
                end.kind(),
                end.detail()
            );
        }
    }
    let bare_ends = count("halt") + count("reentry");
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_bare_ends {bare_ends} # halt + reentry; trap/spin/budget/unloaded are not ends"
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_bare_reentry_fraction {:.4}",
        count("reentry") as f64 / bare_ends.max(1) as f64
    );

    // -- 23, kernel: every program spawned as a process under ADR‑0025's scheduler --
    let mut kernel: Vec<(String, Scheduled)> = Vec::new();
    let mut unspawned = Vec::new();
    for p in &corpus {
        let at = app_loads[0];
        match build(&p.sources, at).and_then(|elf| run_kernel(&elf, at)) {
            Ok(s) => kernel.push((p.name.clone(), s)),
            Err(e) => unspawned.push((p.name.clone(), e)),
        }
    }
    let _ = writeln!(report, "TIME kernel {:.1}s", clock.elapsed().as_secs_f64());
    for (name, s) in &kernel {
        let _ = writeln!(
            report,
            "KERNEL {name} end={} {} on_clock={} on_request={} on_fault={} after={} idle={}",
            s.end.kind(),
            s.end.detail(),
            s.on_clock,
            s.on_request,
            s.on_fault,
            s.after,
            s.idle
        );
    }
    for (name, e) in &unspawned {
        let _ = writeln!(report, "OFFENDER unspawned {name} {e}");
    }
    let kcount = |kind: &str| kernel.iter().filter(|(_, s)| s.end.kind() == kind).count();
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_programs {}",
        kernel.len()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_unspawned {}",
        unspawned.len()
    );
    for kind in ["reentry", "halt", "budget"] {
        let _ = writeln!(
            report,
            "METRIC paradigm_boundary_end_kernel_{kind} {}",
            kcount(kind)
        );
    }
    // Of the programs with no end on the ledger, how many were parked on the idle idiom
    // when the budget ran out (`W‑219`): the boot proofs that finished their work and
    // idled, told apart from a program still computing.
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_budget_idle {}",
        kernel.iter().filter(|(_, s)| s.idle).count()
    );
    let kernel_ends = kcount("reentry") + kcount("halt");
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_ends {kernel_ends}"
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_reentry_fraction {:.4}",
        kcount("reentry") as f64 / kernel_ends.max(1) as f64
    );
    // The two kinds must agree with what followed them, or the classification is a
    // reading of an enum and not a measurement.
    let mut disagree = 0;
    for (name, s) in &kernel {
        if s.end.kind() == "halt" {
            let _ = writeln!(report, "OFFENDER kernel-halt {name} {}", s.end.detail());
        }
        if s.end.kind() == "budget" {
            continue; // no end, so nothing is "after" it
        }
        let followed = s.after > 0;
        let claims = s.end.kind() == "reentry";
        if followed != claims {
            disagree += 1;
            let _ = writeln!(
                report,
                "OFFENDER kernel-ledger {name} end={} but slices after it: {}",
                s.end.kind(),
                s.after
            );
        }
    }
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_ledger_disagreements {disagree}"
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_reentry_on_request {}",
        kernel
            .iter()
            .filter(|(_, s)| s.end.detail().starts_with("exited"))
            .count()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_end_kernel_reentry_on_fault {}",
        kernel
            .iter()
            .filter(|(_, s)| s.end.detail().starts_with("faulted"))
            .count()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_slice_ends_on_clock {}",
        kernel.iter().map(|(_, s)| s.on_clock).sum::<u64>()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_slice_ends_on_request {}",
        kernel.iter().map(|(_, s)| s.on_request).sum::<u64>()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_slice_ends_on_fault {}",
        kernel.iter().map(|(_, s)| s.on_fault).sum::<u64>()
    );

    // -- the ह‑junction at the syscall --
    let mut by_number: BTreeMap<(&str, u64), (u64, u64, u64)> = BTreeMap::new();
    let mut both = Vec::new();
    let mut to_payload = 0;
    let mut to_diagnostic = 0;
    let mut silent = 0;
    for c in &calls {
        let e = by_number.entry((c.interface, c.number)).or_default();
        e.0 += 1;
        e.1 += c.bytes_in;
        e.2 += c.bytes_out;
        match c.channels.as_slice() {
            [Channel::Payload] => to_payload += 1,
            [Channel::Diagnostic] => to_diagnostic += 1,
            [] => silent += 1,
            _ => both.push(c.clone()),
        }
    }
    let mut programs_with_calls: Vec<&str> = calls.iter().map(|c| c.program.as_str()).collect();
    programs_with_calls.sort_unstable();
    programs_with_calls.dedup();
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_total {}",
        calls.len()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_programs {}",
        programs_with_calls.len()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_numbers {}",
        by_number.len()
    );
    for ((interface, number), (n, bin, bout)) in &by_number {
        let _ = writeln!(
            report,
            "METRIC paradigm_boundary_ecall_calls_{interface}_{number} {n}"
        );
        let _ = writeln!(
            report,
            "METRIC paradigm_boundary_ecall_bytes_in_{interface}_{number} {bin}"
        );
        let _ = writeln!(
            report,
            "METRIC paradigm_boundary_ecall_bytes_out_{interface}_{number} {bout}"
        );
    }
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_bytes_in {}",
        calls.iter().map(|c| c.bytes_in).sum::<u64>()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_bytes_out {}",
        calls.iter().map(|c| c.bytes_out).sum::<u64>()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_payload_channel {to_payload}"
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_diagnostic_channel {to_diagnostic}"
    );
    let _ = writeln!(report, "METRIC paradigm_boundary_ecall_silent {silent}");
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_both_channels {}",
        both.len()
    );
    for c in &both {
        let _ = writeln!(
            report,
            "OFFENDER ecall {} {}_{} fed both channels: in={} out={}",
            c.program, c.interface, c.number, c.bytes_in, c.bytes_out
        );
    }
    // The octets that crossed the junction by a call rather than by a store, kept apart
    // from the UART stores' so a diagnostic-shaped line can be blamed on the right one.
    let shaped: Vec<String> = lines(&console)
        .into_iter()
        .filter(|l| diagnostic_shaped(l, ""))
        .map(|l| String::from_utf8_lossy(l).into_owned())
        .collect();
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_console_bytes {}",
        console.len()
    );
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_ecall_console_diagnostic_shaped_lines {}",
        shaped.len()
    );
    for l in &shaped {
        let _ = writeln!(report, "OFFENDER ecall-console {l:?}");
    }
    for name in &programs_with_calls {
        let n = calls.iter().filter(|c| &c.program == name).count();
        let _ = writeln!(report, "CALLS {name} {n}");
    }
    let _ = writeln!(
        report,
        "METRIC paradigm_boundary_census_seconds {:.0}",
        clock.elapsed().as_secs_f64()
    );

    print!("{report}");
}

// --- tests with the routine ----------------------------------------------------------------

/// A program that writes `text` to the UART one octet at a time and then the finisher.
fn uart_writer(text: &str, load: u64) -> Vec<u8> {
    let mut words = vec![lui(5, (UART >> 12) as u32)]; // t0 = UART
    for &b in text.as_bytes() {
        words.push(addi(6, 0, i32::from(b))); // t1 = byte
        words.push(store(0, 5, 6, 0)); // sb t1, 0(t0)
    }
    words.push(lui(7, (FINISHER >> 12) as u32)); // t2 = FINISHER
    words.push(lui(28, 0x5)); // t3 = 0x5000
    words.push(addi(28, 28, 0x555)); // t3 = 0x5555
    words.push(store(2, 7, 28, 0)); // sw t3, 0(t2)
    image(&words, load)
}

/// **The refused case.** A program that writes a diagnostic‑shaped line to the payload
/// UART is reported impure by the census's own check, naming the line.
#[test]
fn a_program_that_writes_a_diagnostic_shaped_line_to_the_payload_is_impure() {
    let line = "halt: Finisher { value: 21845, status: Some(0) }\n";
    let bare = run_bare("impostor", &uart_writer(line, LOAD_ADDRESS));
    assert_eq!(
        String::from_utf8_lossy(&bare.streams.payload),
        line,
        "the program's bytes reached the payload stream, prefix and all"
    );
    assert_eq!(
        bare.end,
        End::Halt("finisher status=Some(0)".into()),
        "and it halted on the finisher afterwards"
    );
    assert!(
        bare.calls.is_empty(),
        "no ecall in it: it stores to the device"
    );
    let found = impurities(&bare.streams, "impostor");
    assert_eq!(
        found.len(),
        1,
        "exactly one impurity, the line the program forged: {found:?}"
    );
    assert!(
        found[0].starts_with("payload carries a diagnostic-shaped line"),
        "and it is named as the payload's fault, not the diagnostics': {}",
        found[0]
    );
    assert!(found[0].contains("halt: Finisher"), "{}", found[0]);
}

/// The composition `issue-sassembly.md` was filed against — `UART: ` in front of every
/// payload line and `halt:` after it, all on one stream — is impure in **both** directions.
#[test]
fn the_pre_fix_composition_is_impure_in_both_directions() {
    let clean = run_bare("namaste", &uart_writer("namaste saMsAra\n", LOAD_ADDRESS)).streams;
    assert!(
        impurities(&clean, "namaste").is_empty(),
        "the separated streams are pure: {:?}",
        impurities(&clean, "namaste")
    );
    // The old `yantra-run`: everything on stdout, the payload behind `UART: `.
    let mut old = Streams::default();
    for line in lines(&clean.payload) {
        old.payload.extend_from_slice(b"UART: ");
        old.payload.extend_from_slice(line);
        old.payload.push(b'\n');
    }
    old.payload.extend_from_slice(&clean.diagnostics);
    let found = impurities(&old, "namaste");
    assert!(
        found
            .iter()
            .any(|f| f.contains("payload carries a diagnostic-shaped line: \"UART: namaste")),
        "the `UART: ` prefix is a diagnostic in the payload: {found:?}"
    );
    assert!(
        found
            .iter()
            .any(|f| f.contains("payload carries a diagnostic-shaped line: \"halt: ")),
        "and so is the halt line: {found:?}"
    );
    // The other direction: a diagnostic stream that echoes the payload.
    let mut echo = clean.clone();
    echo.diagnose("UART: namaste saMsAra");
    let found = impurities(&echo, "namaste");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("diagnostics carry a payload line under \"UART: \""),
        "{}",
        found[0]
    );
    // And a bare byte on stderr that is no diagnostic at all.
    let mut leak = clean.clone();
    leak.diagnose("namaste saMsAra");
    let found = impurities(&leak, "namaste");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("diagnostics carry a line that is not a diagnostic"),
        "{}",
        found[0]
    );
}

/// `spec/namaste.sas`, built by the toolchain and run by the binary's path: pure, and its
/// end is a halt. On the bare machine there is nothing to re‑enter.
#[test]
fn namaste_runs_pure_and_its_end_is_a_halt_on_the_bare_machine() {
    let p = corpus()
        .into_iter()
        .find(|p| p.name == "namaste.sas")
        .expect("spec/namaste.sas");
    let elf = build(&p.sources, p.load).expect("namaste assembles");
    let bare = run_bare(&p.name, &elf);
    // `॥ अष्टकाः उक्तम् नमस्ते संसार इति ॥` and a newline, in the source.
    assert_eq!(
        String::from_utf8_lossy(&bare.streams.payload),
        "नमस्ते संसार\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&bare.streams.diagnostics),
        "halt: Finisher { value: 21845, status: Some(0) }\n",
        "the diagnostic stream is the one line `yantra-run` writes to stderr"
    );
    assert!(impurities(&bare.streams, &p.name).is_empty());
    assert_eq!(bare.end, End::Halt("finisher status=Some(0)".into()));
    assert!(
        bare.calls.is_empty(),
        "namaste stores to the device; no ecall"
    );
    assert!(bare.console.is_empty());
}

/// `spec/boot-sbi.sas` says its one line through `sbi_console_putchar` and ends with
/// `sbi_shutdown`: every call is seen, every console call's octet went to the payload and
/// nowhere else, and the shutdown's effect went to the diagnostics and nowhere else.
#[test]
fn every_ecall_of_boot_sbi_is_observed_on_exactly_one_channel() {
    let p = corpus()
        .into_iter()
        .find(|p| p.name == "boot-sbi.sas")
        .expect("spec/boot-sbi.sas");
    assert_eq!(
        p.load, SBI_PAYLOAD,
        "check-sbi-boot.sh builds it above OpenSBI"
    );
    let elf = build(&p.sources, p.load).expect("boot-sbi assembles");
    let bare = run_bare(&p.name, &elf);
    assert_eq!(
        String::from_utf8_lossy(&bare.streams.payload),
        "namaste saMsAra\n"
    );
    assert_eq!(bare.end, End::Halt("sbi_shutdown".into()));
    let console: Vec<&Call> = bare.calls.iter().filter(|c| c.number == 1).collect();
    assert_eq!(
        console.len(),
        "namaste saMsAra\n".len(),
        "one call per octet"
    );
    assert!(
        console.iter().all(|c| c.interface == "sbi"
            && c.bytes_in == 1
            && c.bytes_out == 1
            && c.channels == [Channel::Payload]),
        "{console:?}"
    );
    let last = bare.calls.last().expect("the shutdown");
    assert_eq!((last.interface, last.number), ("sbi", 8));
    assert_eq!(last.channels, [Channel::Diagnostic]);
    assert_eq!(last.bytes_out, 0);
    assert!(
        bare.calls.iter().all(|c| c.channels.len() <= 1),
        "no call fed both channels: {:?}",
        bare.calls
    );
    assert_eq!(
        bare.console, b"namaste saMsAra\n",
        "the octets that crossed by a call are the payload, and all of it"
    );
}

/// Under the kernel the application's end is a re‑entry: it exits by call 0, the
/// scheduler takes the hart back, and the process queued behind it goes on running.
#[test]
fn under_the_kernel_the_applications_end_is_a_re_entry() {
    let p = corpus()
        .into_iter()
        .find(|p| p.class == "app")
        .expect("spec/programs.tsv names one application");
    let at = application_loads()[0];
    let elf = build(&p.sources, at).expect("the application assembles");
    let s = run_kernel(&elf, at).expect("it spawns");
    assert_eq!(s.end, End::ReEntry("exited status=0".into()), "{s:?}");
    assert!(s.after > 0, "the lingerer's slices came after: {s:?}");
    assert_eq!((s.on_request, s.on_fault), (1, 0));

    // And its two calls were seen, each on one channel: WRITE fed the surface and
    // nothing else, EXIT ended the process and produced no payload.
    let calls = observe_hosted_calls(&p.name, &elf);
    let numbers: Vec<u64> = calls.iter().map(|c| c.number).collect();
    assert_eq!(numbers, [1, 0], "WRITE then EXIT: {calls:?}");
    assert_eq!(calls[0].channels, [Channel::Payload]);
    assert_eq!(
        calls[0].bytes_in, calls[0].bytes_out,
        "WRITE moved every octet it was given"
    );
    assert!(calls[0].bytes_out > 0);
    assert_eq!(calls[1].channels, [Channel::Diagnostic]);
    assert_eq!(calls[1].bytes_out, 0);
}

/// **`W‑219`, both halves.** Under the kernel a process that is `jal x0, .` idles: every
/// one of its slices ends on the clock, the lingerer runs beside it to the budget, and
/// the census reads it as a program with no end, parked. On the bare machine the same
/// word is [`Halt::SpinForever`] and the end kind is `spin` — 0% re‑entry by construction,
/// which is the refused case and stays.
#[test]
fn under_the_kernel_a_self_jump_idles_and_on_the_bare_machine_it_halts() {
    let at = application_loads()[0];
    let s = run_kernel(&image(&[jal(0, 0)], at), at).expect("it spawns");
    assert_eq!(s.end, End::Budget, "no end on the ledger: {s:?}");
    assert!(
        s.idle,
        "and it was parked on the jump when the budget ran out: {s:?}"
    );
    assert!(
        s.on_clock > 10,
        "every slice it took ended on the clock: {s:?}"
    );
    assert_eq!((s.on_request, s.on_fault), (0, 0));
    assert_eq!(
        s.after, 0,
        "nothing is 'after' a program that has no end — the lingerer's turns interleave"
    );

    let bare = run_bare("parked", &image(&[jal(0, 0)], LOAD_ADDRESS));
    assert_eq!(
        bare.end,
        End::Spin,
        "on the bare machine the spin is the end, and 0% re-entry there is by construction"
    );
}

/// **The control for the kernel's re‑entry fraction.** A U‑mode word the kernel cannot
/// carry on from — here an instruction this interpreter does not implement — is
/// [`Ended::Stopped`], the schedule ends on it, and the lingerer never runs again. The
/// census calls that a halt, so "every end re‑enters" is a measurement and not a
/// consequence of `W‑219` having removed the only way to fail it.
#[test]
fn the_kernel_reentry_fraction_is_a_measurement() {
    let at = application_loads()[0];
    // Major opcode 0x7f is unassigned in every RISC‑V extension.
    let s = run_kernel(&image(&[0x0000_007f], at), at).expect("it spawns");
    assert_eq!(s.end.kind(), "halt", "{s:?}");
    assert!(s.end.detail().contains("Unimplemented"), "{s:?}");
    assert!(!s.idle);
    assert_eq!(s.after, 0, "the scheduler never got the hart back: {s:?}");
}

/// A boot proof spawned as a process cannot reach the finisher: the store is a page fault
/// from U‑mode, and a fault is a re‑entry — the scheduler runs the next process.
#[test]
fn under_the_kernel_a_finisher_store_is_a_fault_and_the_scheduler_continues() {
    let at = application_loads()[0];
    let s = run_kernel(&uart_writer("x", at), at).expect("it spawns");
    assert_eq!(
        s.end,
        End::ReEntry("faulted cause=15".into()),
        "a store page fault: {s:?}"
    );
    assert!(s.after > 0, "{s:?}");
}

/// Under ADR‑0025 a slice ends on the clock: a program that never asks is preempted, and
/// every one of its slice ends is the timer's; the lingerer takes every other turn.
#[test]
fn under_the_kernel_a_program_that_never_asks_ends_every_slice_on_the_clock() {
    let at = application_loads()[0];
    let spinner = image(&[addi(5, 5, 1), jal(0, -4)], at);
    let s = run_kernel(&spinner, at).expect("it spawns");
    assert_eq!(s.end, End::Budget, "it never ended: {s:?}");
    assert!(s.on_clock > 10, "{s:?}");
    assert_eq!((s.on_request, s.on_fault), (0, 0));
    assert_eq!(
        s.after, 0,
        "nothing is 'after' a program that has no end: {s:?}"
    );
}

/// The binaries compose the streams the way `issue-sassembly.md`'s fix says: stdout
/// receives the payload bytes and nothing else; every stderr line begins with a known
/// prefix or the path. Read from the source, since the binaries are not exec'd here.
#[test]
fn the_binaries_write_only_the_payload_to_stdout() {
    for bin in ["yantra-run.rs", "yantra-host.rs"] {
        let src = std::fs::read_to_string(root().join("crates/yantra/src/bin").join(bin))
            .expect("read the binary's source");
        // `eprintln!` contains `println!`; take the stderr macros out before looking.
        let without_stderr = src.replace("eprintln!(", "").replace("eprint!(", "");
        assert!(
            !without_stderr.contains("println!(") && !without_stderr.contains("print!("),
            "{bin}: a print! to stdout is a byte that is not the payload"
        );
        let stdout_writes: Vec<&str> = src
            .lines()
            .filter(|l| l.contains("stdout()"))
            .map(str::trim)
            .collect();
        assert!(
            !stdout_writes.is_empty(),
            "{bin} writes the payload somewhere"
        );
        // THE PAYLOAD IS NAMED THREE WAYS AND THIS LISTED TWO. `yantra-run.rs:82`
        // writes `&sink.out` — the UART sink's own buffer, filled by `m.run(…,
        // &mut sink)` at `:80` and written unchanged — which is the payload under
        // a different spelling. The guard matched a VARIABLE NAME, so renaming a
        // local turned a correct binary into a FAILED gate step, and that step is
        // one of the reds that has kept `salvage()` from landing anything.
        //
        // A guard that names a spelling rather than a condition reds on a rename
        // and waves through a real leak that happens to use an accepted name. The
        // condition it wants is "stdout receives the payload and nothing else";
        // this stays a spelling list because the alternative is dataflow analysis
        // in a test, but the list is now COMPLETE and says why each entry is on it.
        for w in &stdout_writes {
            assert!(
                w.contains("write_all(&out)")          // the plain buffer
                    || w.contains("write_all(&h.surface)")  // the hosted surface
                    || w.contains("write_all(&sink.out)"), // the UART sink's buffer
                "{bin}: stdout receives something that is not the payload: {w}"
            );
        }
        let mut stderr_lines = 0;
        for line in src
            .lines()
            .filter(|l| l.trim_start().starts_with("eprintln!("))
        {
            stderr_lines += 1;
            let literal = line.split('"').nth(1).unwrap_or("");
            let known = DIAGNOSTIC_PREFIXES
                .iter()
                .any(|p| literal.starts_with(p.trim_end()))
                || literal.starts_with("{path}: ");
            assert!(
                known || literal.is_empty(),
                "{bin}: a stderr line the census does not recognise as a diagnostic: {line}"
            );
        }
        assert!(stderr_lines >= 2, "{bin}: the diagnostics go to stderr");
    }
}

/// `W-237` — THE FLIP, COMPLETED. This test was `the_t1_corpus_has_no_path_onto_yantra_yet`
/// (`W-212`): 16 `.सस` counted, no RISC‑V backend, `t1_runnable_on_yantra 0`. `W-235` made it
/// red on purpose (a second backend, `riscv64`) and amended it; this row REPLACES it with
/// the measured path: the T1 corpus the chain reads is the 18 `.t1` sources, `W-237`'s
/// census in `tests/paradigm_encode.rs` runs each one to the machine, and the number it
/// pins is the number this census reports. The 16 `.सस` stay counted and 0 readable BY
/// DESIGN (`W-238`), which is a decision and not a gap.
#[test]
fn the_t1_corpus_reaches_yantra_by_the_pinned_number() {
    let sas = std::fs::read_dir(root().join("tests/corpus/t1"))
        .expect("tests/corpus/t1")
        .filter(|e| {
            e.as_ref()
                .expect("entry")
                .path()
                .extension()
                .is_some_and(|x| x == "सस")
        })
        .count();
    assert_eq!(
        sas,
        pins::SAS_PROGRAMS,
        "the retired dialect's programs, counted"
    );
    let sources = std::fs::read_dir(root().join("crates/sadhana-t1/src"))
        .expect("crates/sadhana-t1/src")
        .filter(|e| {
            e.as_ref()
                .expect("entry")
                .path()
                .extension()
                .is_some_and(|x| x == "t1")
        })
        .count();
    assert_eq!(
        sources,
        pins::T1_SOURCES,
        "the self-hosting corpus, counted"
    );
    // A GENERATED SOURCE IS SUBTRACTED ONCE, SO THE LIST THAT SUBTRACTS IT MUST
    // NAME FILES THAT ARE THERE. `T1_GENERATED` reduces `T1_CENSUSED`, which is
    // what every measured pin below is compared against; a name that has been
    // deleted or renamed would shrink that expectation silently and a source
    // that went missing would read as "generated, so not censused".
    for name in pins::T1_GENERATED {
        let p = root().join("crates/sadhana-t1/src").join(name);
        let text = std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("T1_GENERATED names {name}, which is not readable: {e}"));
        assert!(
            text.lines()
                .take(8)
                .any(|l| l.contains("DO NOT EDIT BY HAND")),
            "{name} is in T1_GENERATED but carries no generated marker in its \
             first eight lines — the census's `corpus()` would WALK it while \
             this file subtracts it"
        );
    }
    let modules =
        std::fs::read_to_string(root().join("crates/sadhana/src/t1/mod.rs")).expect("t1/mod.rs");
    let backends: Vec<&str> = modules
        .lines()
        .filter_map(|l| l.strip_prefix("pub mod "))
        .map(|m| m.trim_end_matches(';'))
        .filter(|m| m.contains("x86") || m.contains("riscv") || m.contains("rv64"))
        .collect();
    assert_eq!(
        backends,
        ["riscv64", "x86_64"],
        "the backends under crates/sadhana/src/t1"
    );
    // The path exists and is measured: the pin says how many sources walk it to
    // the end, and the three pins account for every source between them. Zero
    // here would mean the emitter lane regressed to W-212's state.
    // ── ALL THREE MOVED INTO `const` BLOCKS, 2026-09-12, FOR THE REASON THE
    // ── FOURTH ONE BELOW WAS BORN IN ONE ──
    //
    // These escaped `clippy::assertions_on_constants` only because `.len()` on a
    // slice is not folded by the lint — not because they are less constant than
    // the bound below, which clippy DID refuse. **A LINT'S BLIND SPOT IS NOT A
    // DESIGN DECISION**, and leaving them as runtime asserts meant three pin
    // identities that a filtered run, an `#[ignore]`, or a lane that never opens
    // this binary could walk straight past. In a `const` block an impossible pin
    // set stops the crate BUILDING.
    //
    // `<[T]>::len` is const-callable, so the arithmetic is unchanged; what
    // changes is WHEN it is checked. The cost is the same one the fourth pays:
    // a `const` panic takes no format arguments, so each message names its pins
    // instead of printing their values, and `pins.rs` is two lines away.
    //
    // Measured before and after on the exact defect this exists for: with
    // `T1_ON_YANTRA` left at 17 against a six-entry stopped list, the third
    // identity fails at COMPILE time — `error[E0080]`, evaluation panicked —
    // where before it needed this test to be selected and run.
    const {
        assert!(
            pins::T1_ON_YANTRA != 0,
            "T1_ON_YANTRA is zero: no source reaches yantra, which would be the \
             emitter lane back at W-212's state"
        );
        assert!(
            pins::T1_CENSUSED - pins::T1_UNASSEMBLED.len() == pins::T1_ASSEMBLED,
            "T1_CENSUSED minus T1_UNASSEMBLED's length is not T1_ASSEMBLED: a \
             source is neither assembled nor named with its stop"
        );
        assert!(
            pins::T1_ASSEMBLED - pins::T1_ASSEMBLED_NOT_ON_YANTRA.len() == pins::T1_ON_YANTRA,
            "T1_ASSEMBLED minus T1_ASSEMBLED_NOT_ON_YANTRA's length is not \
             T1_ON_YANTRA: an assembled source neither runs nor is named with \
             its stop"
        );
    }
    // ── ADDED 2026-09-12, AND IT IS A RULE TURNED INTO A MECHANISM ──
    //
    // `T1_CHAIN_ON_YANTRA` counts sources the WHOLE chain carries to the
    // machine, and such a source must also RUN — so it can never exceed
    // `T1_ON_YANTRA`. That bound was true all along and lived nowhere: this
    // ledger held the other three identities and not this one.
    //
    // **THE COST OF ITS ABSENCE, MEASURED:** moving `T1_ON_YANTRA` 17 -> 11 left
    // `T1_CHAIN_ON_YANTRA` at 17, which is arithmetically impossible, and the
    // only thing that said so was the census — **1,988 seconds to be told a
    // number the previous edit had already determined.** This assertion is free
    // and answers in milliseconds.
    //
    // A `<=` and not an `==`: the two are equal today because every running
    // source also passes the checker, and the day one runs that the checker
    // refuses they part. Pinning equality here would forbid that legitimately.
    //
    // **IT IS A `const` BLOCK, AND CLIPPY IS THE REASON IT BECAME A BETTER
    // GUARD.** Written as a runtime `assert!` it was refused —
    // `clippy::assertions_on_constants`, "this assertion has a constant value",
    // which is correct: both sides are `const`. Moved into a `const` block it
    // fails at COMPILE time instead, so an impossible pin pair cannot be reached
    // by a filtered run, an `#[ignore]`, or a lane that never runs this binary —
    // it stops the crate from building at all. The three assertions above escape
    // the lint only because `.len()` on a slice is not folded; they would be
    // better here too, and that is a separate edit.
    //
    // The cost is the message: a `const` panic takes no format arguments, so it
    // names the two pins instead of printing their values. The reader is two
    // lines away from both in `pins.rs`.
    const {
        assert!(
            pins::T1_CHAIN_ON_YANTRA <= pins::T1_ON_YANTRA,
            "T1_CHAIN_ON_YANTRA exceeds T1_ON_YANTRA: the whole chain cannot \
             carry more sources to the machine than RUN there. A dependent pin \
             was left behind when the one that bounds it moved."
        );
    }
    println!(
        "METRIC paradigm_boundary_t1_runnable_on_yantra {}",
        pins::T1_ON_YANTRA
    );
}

/// The corpus is what `spec/programs.tsv` says it is, and the load addresses come from
/// the scripts: 48 programs, one application, and the SBI clients above OpenSBI.
#[test]
fn the_corpus_is_read_from_the_tables_and_the_scripts() {
    let corpus = corpus();
    assert_eq!(corpus.len(), 48);
    assert_eq!(corpus.iter().filter(|p| p.class == "app").count(), 1);
    assert!(corpus.iter().all(|p| p.class != "unclassified"));
    let above: Vec<&str> = corpus
        .iter()
        .filter(|p| p.load == SBI_PAYLOAD)
        .map(|p| p.name.as_str())
        .collect();
    assert!(above.contains(&"boot-sbi.sas"));
    assert!(above.contains(&"timer.sas"));
    assert!(
        !above.contains(&"namaste.sas"),
        "check-roundtrip.sh mentions the address in a comment only: {above:?}"
    );
    assert!(!above.contains(&"bare-metal.sas"));
    assert!(!above.contains(&"atithi.sas"));
    let pair = corpus
        .iter()
        .find(|p| p.name == "namaste-main.sas")
        .expect("namaste-main");
    assert_eq!(pair.sources.len(), 2, "linked with lib-mudraka.sas");
}

/// Sv39 as the census walks it agrees with the machine: a process's instructions fetched
/// through its own tables are the words the ELF holds, in the order the hart retires them.
#[test]
fn the_census_fetches_through_the_same_tables_the_machine_does() {
    let at = application_loads()[0];
    let words = [addi(5, 5, 1), addi(5, 5, 2), jal(0, -8)];
    let mut m = kernel_machine();
    let mut k = Kernel::new(FREE);
    let pid = k
        .spawn(&mut m, &image(&words, at), Vec::new(), Vec::new())
        .expect("spawns");
    let p = &k.processes[pid];
    assert_ne!(p.satp >> 60, 0, "the process runs under Sv39");
    assert_eq!(fetch_at(&m, p.satp, p.sepc), Some(words[0]));
    let mut uart = Vec::new();
    // Two steps: the supervisor's `sret` into U-mode, and the first `addi`.
    let _ = k.run(&mut m, pid, 2, &mut uart);
    assert_eq!(m.mode, Privilege::User);
    assert_eq!(
        fetch(&m),
        Some(words[1]),
        "the hart stands on the second word"
    );
    assert_eq!(m.x[5], 1, "and the first retired");
    // The kernel saved where the hart stood, not the CSR no trap had moved (`W‑220`).
    assert_eq!(
        k.processes[pid].sepc, m.pc,
        "the saved program counter is the second word"
    );
    assert_eq!(k.processes[pid].sepc, at + 4);
    let _ = k.run(&mut m, pid, 2, &mut uart);
    assert_eq!(fetch(&m), Some(words[2]));
    assert_eq!(m.x[5], 3, "the second retired too");
    assert_eq!(
        translate(&m, m.csr.satp, 0xdead_0000),
        None,
        "an unmapped page is None, not a guess"
    );
}

/// A sink that drops every octet, for a run whose payload is not the question.
struct Sink;
impl Output for Sink {
    fn putc(&mut self, _: u8) {}
}

/// `Machine::run` and the census's step loop end at the same place for the same reason.
#[test]
fn the_census_step_loop_ends_where_machine_run_ends() {
    let elf = uart_writer("abc\n", LOAD_ADDRESS);
    let mut m = Machine::load_elf(&elf, BARE_RAM).expect("loads");
    let halt = m.run(BARE_BUDGET, &mut Sink);
    let bare = run_bare("w", &elf);
    assert_eq!(bare.end, end_of_halt(&halt));
    assert_eq!(end_of_halt(&Halt::StepLimit { pc: 0 }), End::Budget);
    assert_eq!(end_of_halt(&Halt::SpinForever { pc: 0 }), End::Spin);
    assert_eq!(
        end_of_halt(&Halt::BadAccess { pc: 0, addr: 0 }),
        End::Trap("BadAccess".into())
    );
}

/// The lingerer is what its name says: under the budget it never reaches its exit, and
/// its odometer shows it ran.
#[test]
fn the_lingerer_outlasts_the_budget() {
    let at = application_loads()[0];
    let mut m = kernel_machine();
    let mut k = Kernel::preemptive(FREE, QUANTUM);
    let pid = k
        .spawn(&mut m, &lingerer(at), Vec::new(), Vec::new())
        .expect("spawns");
    let mut uart = Vec::new();
    let ledger = k.schedule(&mut m, KERNEL_BUDGET, &mut uart);
    assert!(
        ledger.iter().all(|s| s.pid == pid),
        "one process, every slice its own"
    );
    assert!(
        k.processes[pid].ended.is_none(),
        "it did not get to its exit: {:?}",
        ledger.last()
    );
    assert!(
        k.processes[pid].x[5] < 65536 && k.processes[pid].x[5] > 0,
        "and it was counting: t0 = {}",
        k.processes[pid].x[5]
    );
}
