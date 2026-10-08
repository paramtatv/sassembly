//! **`W-375`: THE WALL CLOCK IS A VALUE DELIVERED AT A WAIT, NEVER READ BETWEEN INSTRUCTIONS.**
//!
//! ADR-0040 Option C, R6. `W-371` built the event channel's REPLAY and said live mode
//! belongs to the first row with an event source; the clock is that source. So:
//!
//! - `yantra-run --record-events <log> <image>` runs LIVE: at each wait the host's current
//!   time — nanoseconds since the Unix epoch, UTC — is written into the word after the
//!   image's `SASEVENT` tag, and the record `t=<ns>` is appended to the log;
//! - `yantra-run --events <log> <image>` REPLAYS that log unchanged: a `t=` record delivers
//!   its logged value, never the clock's.
//!
//! The three falsifiers, each shown red first:
//!
//! - (a) record a run live, replay its log: the program prints the SAME clock values and
//!   the two runs retire the SAME count with the SAME output digest;
//! - (b) a ratchet: the ONLY wall-clock use under `crates/yantra/src` is the event stamper,
//!   `yantra::input::stamp_event_time` — with a mutation control that adds a second use to
//!   a scratch copy and sees the ratchet fire;
//! - (c) no clock without a wait: a program that never waits gets no time value — its event
//!   word is untouched and the log holds no record.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sadhana::kosha;
use yantra::input::EVENT_TAG;
use yantra::{FINISHER, UART, WAIT};

// Encoders written out, for the reason `w371_events.rs` gives.
fn r(op: u32, rd: u32, f3: u32, rs1: u32, rs2: u32, f7: u32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | f7 << 25
}
fn i(op: u32, rd: u32, f3: u32, rs1: u32, imm: i32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn s(op: u32, f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    op | (imm & 0x1f) << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | (imm >> 5 & 0x7f) << 25
}
fn b(f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    0x63 | (imm >> 11 & 1) << 7
        | (imm >> 1 & 0xf) << 8
        | f3 << 12
        | rs1 << 15
        | rs2 << 20
        | (imm >> 5 & 0x3f) << 25
        | (imm >> 12 & 1) << 31
}
fn u(op: u32, rd: u32, imm: u32) -> u32 {
    op | rd << 7 | (imm & 0xffff_f000)
}
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i(0x13, rd, 0, rs1, imm)
}

/// How many times the waiting program waits.
const WAITS: i32 = 3;
/// What the event word holds before any delivery — not zero, so an untouched word is
/// told apart from a delivered zero.
const UNDELIVERED: u64 = 0x0123_4567_89ab_cdef;

/// Waits `waits` times; after each wait prints the event word as sixteen lowercase hex
/// digits and a newline; then writes the SUCCESS finisher. `waits == 0` builds the SAME
/// loop run once with the wait store replaced by a no-op: a program that never waits and
/// still reads and prints its event word.
fn program(waits: i32) -> Vec<u8> {
    assert_eq!(UART, 0x1000_0000, "the encoding below spells this address");
    assert_eq!(WAIT, UART + 0x100, "the encoding below spells this address");
    let wait = if waits > 0 {
        s(0x23, 3, 5, 0, 0) // sd x0, 0(x5): the WAIT
    } else {
        addi(0, 0, 0) // nop
    };
    let words = [
        u(0x37, 5, 0x1000_0000),      // 0  x5 = UART
        addi(5, 5, 0x100),            // 1  x5 = WAIT
        u(0x17, 7, 0),                // 2  x7 = pc = BASE + 8
        addi(7, 7, 0),                // 3  x7 += (slot - 8), patched below
        addi(12, 0, waits.max(1)),    // 4  x12 = iterations left
        u(0x37, 16, 0x1000_0000),     // 5  x16 = UART
        wait,                         // 6  WAIT (or nop)         <- loop top
        i(0x03, 8, 3, 7, 0),          // 7  x8 = the event word
        addi(9, 0, 60),               // 8  x9 = shift
        r(0x33, 10, 5, 8, 9, 0),      // 9  srl x10 = x8 >> x9    <- nibble top
        i(0x13, 10, 7, 10, 15),       // 10 andi x10 &= 15
        addi(11, 0, 10),              // 11
        b(4, 10, 11, 8),              // 12 blt x10, 10 -> 14
        addi(10, 10, 39),             // 13 'a' - '0' - 10
        addi(10, 10, 48),             // 14 '0' +
        s(0x23, 0, 16, 10, 0),        // 15 putc
        addi(9, 9, -4),               // 16
        b(5, 9, 0, -32),              // 17 bge x9, x0 -> 9
        addi(10, 0, 10),              // 18 '\n'
        s(0x23, 0, 16, 10, 0),        // 19 putc
        addi(12, 12, -1),             // 20
        b(1, 12, 0, -60),             // 21 bne x12, x0 -> 6
        u(0x37, 10, FINISHER as u32), // 22 x10 = FINISHER
        u(0x37, 11, 0x5000),          // 23
        addi(11, 11, 0x555),          // 24 x11 = 0x5555, success
        s(0x23, 2, 10, 11, 0),        // 25 the finisher
    ];
    let mut words = words.to_vec();
    let tag_at = words.len() * 4;
    assert_eq!(tag_at % 8, 0, "the tag word is 8-aligned");
    words[3] = addi(7, 7, (tag_at + 8) as i32 - 8);
    let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    bytes.extend_from_slice(&EVENT_TAG.to_le_bytes());
    bytes.extend_from_slice(&UNDELIVERED.to_le_bytes());
    bytes
}

/// A fresh directory per call: pid AND a clock, because a pid is reused (W-301). A
/// TEST may read the clock; the ratchet below is over `crates/yantra/src`.
fn scratch(what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w375-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Run `yantra-run <flag> <log> <image>` in `dir` with the image built from `waits`.
fn run(dir: &Path, waits: i32, flag: &str, log: &Path) -> Output {
    let image = dir.join("clock.elf");
    std::fs::write(&image, kosha::write(&program(waits))).expect("write the image");
    Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .current_dir(dir)
        .arg(flag)
        .arg(log)
        .arg(&image)
        .output()
        .expect("yantra-run runs")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn retired(out: &Output) -> u64 {
    let err = stderr(out);
    err.lines()
        .find_map(|l| l.strip_prefix("steps: "))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no steps line:\n{err}"))
}

fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

/// The values the program printed, one per line, as words.
fn printed(out: &Output) -> Vec<u64> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| u64::from_str_radix(l, 16).unwrap_or_else(|e| panic!("{l:?}: {e}")))
        .collect()
}

/// The `t=` values in a recorded log, read by hand rather than by the parser under test.
fn logged_times(log: &str) -> Vec<u64> {
    log.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            l.strip_prefix("t=")
                .unwrap_or_else(|| panic!("a live record is a clock record: {l:?}"))
                .parse()
                .unwrap_or_else(|e| panic!("{l:?}: {e}"))
        })
        .collect()
}

/// (a) LIVE, THEN REPLAY: the same clock values, the same retired count, the same digest.
#[test]
fn a_recorded_run_replays_with_the_same_clock_values_count_and_digest() {
    let dir = scratch("a");
    let log = dir.join("clock.log");
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let live = run(&dir, WAITS, "--record-events", &log);
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    let replayed = run(&dir, WAITS, "--events", &log);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        live.status.code(),
        Some(0),
        "the live run:\n{}",
        stderr(&live)
    );
    assert_eq!(
        replayed.status.code(),
        Some(0),
        "the replay:\n{}",
        stderr(&replayed)
    );
    let times = logged_times(&text);
    assert_eq!(times.len(), WAITS as usize, "one record per wait:\n{text}");
    // The unit and epoch: each stamp lies between two host readings taken around the run.
    for t in &times {
        assert!(
            before <= *t && *t <= after,
            "{t} is not ns since the Unix epoch within [{before}, {after}]"
        );
    }
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{times:?}");
    assert_eq!(printed(&live), times, "the program saw what was logged");
    assert_eq!(
        printed(&replayed),
        times,
        "the replay saw the LOGGED values"
    );
    assert_eq!(retired(&live), retired(&replayed), "retired counts");
    assert_eq!(digest(&live.stdout), digest(&replayed.stdout), "digests");
}

/// The clock patterns, matched as whole identifiers (or the `std::time` path).
const CLOCK_TOKENS: &[&str] = &[
    "SystemTime",
    "Instant",
    "UNIX_EPOCH",
    "clock_gettime",
    "gettimeofday",
    "chrono",
    "rdtsc",
    "_rdtsc",
];

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn line_reads_a_clock(code: &str) -> bool {
    if code.contains("std::time") {
        return true;
    }
    CLOCK_TOKENS.iter().any(|tok| {
        code.match_indices(tok).any(|(at, _)| {
            let pre = code[..at].chars().next_back();
            let post = code[at + tok.len()..].chars().next();
            !pre.is_some_and(is_ident) && !post.is_some_and(is_ident)
        })
    })
}

/// The stamper's opening line; its body runs to the next line that is exactly `}`.
const STAMPER: &str = "pub fn stamp_event_time() -> u64 {";

/// Every clock use under `root`, OUTSIDE the stamper's body, as `path:line: text`; and the
/// number of clock uses INSIDE it. Comment lines are skipped: a margin may name the clock.
fn clock_uses(root: &Path) -> (Vec<String>, usize) {
    let mut outside = Vec::new();
    let mut inside = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("{}: {e}", d.display()))
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            let mut in_stamper = false;
            for (n, line) in text.lines().enumerate() {
                if line.trim_start().starts_with(STAMPER) {
                    in_stamper = true;
                    continue;
                }
                if in_stamper && line == "}" {
                    in_stamper = false;
                    continue;
                }
                let code = line.trim_start();
                if code.starts_with("//") || !line_reads_a_clock(code) {
                    continue;
                }
                if in_stamper {
                    inside += 1;
                } else {
                    outside.push(format!("{}:{}: {}", p.display(), n + 1, code));
                }
            }
        }
    }
    (outside, inside)
}

fn yantra_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// (b) THE RATCHET: the stamper reads the clock, and nothing else under `crates/yantra/src`
/// does — not the machine, not the loader, not `yantra-run`.
#[test]
fn the_only_wall_clock_use_in_yantra_is_the_event_stamper() {
    let (outside, inside) = clock_uses(&yantra_src());
    assert!(
        inside >= 1,
        "no event stamper found: `{STAMPER}` with a clock read in its body, in crates/yantra/src"
    );
    assert!(
        outside.is_empty(),
        "a wall-clock use outside the event stamper — a time readable between instructions:\n{}",
        outside.join("\n")
    );
}

/// (b) THE MUTATION CONTROL: copy `crates/yantra/src` to a scratch tree, add ONE second
/// clock use to `lib.rs`, and the same scan must report exactly that line.
#[test]
fn the_ratchet_fires_on_a_second_clock_use_in_a_scratch_copy() {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let p = e.unwrap().path();
            let q = to.join(p.file_name().unwrap());
            if p.is_dir() {
                copy(&p, &q);
            } else {
                std::fs::copy(&p, &q).unwrap();
            }
        }
    }
    let dir = scratch("b");
    let src = dir.join("src");
    copy(&yantra_src(), &src);
    let (clean, _) = clock_uses(&src);
    let lib = src.join("lib.rs");
    let mut text = std::fs::read_to_string(&lib).unwrap();
    text.push_str("\nfn smuggled() -> u128 { std::time::Instant::now().elapsed().as_nanos() }\n");
    std::fs::write(&lib, text).unwrap();
    let (mutated, _) = clock_uses(&src);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(clean.is_empty(), "the copy starts clean: {clean:?}");
    assert_eq!(mutated.len(), 1, "{mutated:?}");
    assert!(mutated[0].contains("smuggled"), "{mutated:?}");
}

/// (c) NO CLOCK WITHOUT A WAIT: live mode on a program that never waits writes nothing into
/// its event word — it prints the sentinel it was built with — and logs no record.
#[test]
fn a_program_that_never_waits_gets_no_time_value() {
    let dir = scratch("c");
    let log = dir.join("clock.log");
    let out = run(&dir, 0, "--record-events", &log);
    let text = std::fs::read_to_string(&log).ok();
    let _ = std::fs::remove_dir_all(&dir);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert_eq!(
        printed(&out),
        vec![UNDELIVERED],
        "the event word untouched:\n{err}"
    );
    let text =
        text.unwrap_or_else(|| panic!("live mode writes its log even with no waits:\n{err}"));
    assert!(logged_times(&text).is_empty(), "no record:\n{text}");
}

/// v1.0.1: the FACT of a `--files` grant is recorded in the log header, and a replay
/// without `--files` refuses at load, by name. The program here has the event interface.
#[test]
fn a_log_recorded_with_files_refuses_a_replay_without_the_flag() {
    let dir = scratch("files");
    let root = dir.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let image = dir.join("clock.elf");
    std::fs::write(&image, kosha::write(&program(2))).unwrap();
    let log = dir.join("granted.log");
    let go = |args: &[&std::ffi::OsStr]| {
        Command::new(env!("CARGO_BIN_EXE_yantra-run"))
            .args(args)
            .output()
            .expect("yantra-run runs")
    };
    let os = std::ffi::OsStr::new;
    let rec = go(&[
        os("--files"),
        root.as_os_str(),
        os("--record-events"),
        log.as_os_str(),
        image.as_os_str(),
    ]);
    assert_eq!(rec.status.code(), Some(0), "{}", stderr(&rec));
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.lines().any(|l| l.starts_with("# yantra-run --files")),
        "the header does not say a root was granted:\n{text}"
    );
    assert_eq!(
        logged_times(&text).len(),
        2,
        "the grant line disturbed the records"
    );
    // REPLAY WITHOUT THE FLAG: refused loudly, nothing ran.
    let bad = go(&[os("--events"), log.as_os_str(), image.as_os_str()]);
    let err = stderr(&bad);
    assert_eq!(bad.status.code(), Some(1), "{err}");
    assert!(err.contains("recorded with --files"), "{err}");
    assert!(!err.contains("halt:"), "it ran:\n{err}");
    // REPLAY WITH IT: the same values, count and digest as the recording.
    let ok = go(&[
        os("--files"),
        root.as_os_str(),
        os("--events"),
        log.as_os_str(),
        image.as_os_str(),
    ]);
    assert_eq!(ok.status.code(), Some(0), "{}", stderr(&ok));
    assert_eq!(printed(&ok), printed(&rec));
    assert_eq!(retired(&ok), retired(&rec));
    // CONTROL: a log recorded WITHOUT the flag has no grant line and replays without it.
    let plain = dir.join("plain.log");
    let rec2 = go(&[os("--record-events"), plain.as_os_str(), image.as_os_str()]);
    assert_eq!(rec2.status.code(), Some(0));
    assert!(!std::fs::read_to_string(&plain).unwrap().contains("--files"));
    let ok2 = go(&[os("--events"), plain.as_os_str(), image.as_os_str()]);
    assert_eq!(ok2.status.code(), Some(0), "{}", stderr(&ok2));
    let _ = std::fs::remove_dir_all(&dir);
}
