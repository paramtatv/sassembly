//! **`W-371`: AT A WAIT THE HOST WRITES ONE EVENT RECORD, AND A RECORDED LOG REPLAYS THE RUN.**
//!
//! ADR-0040 Option C, R2. `W-370` made a store to [`WAIT`] a pause; this row gives the pause
//! something to deliver. The program declares one word right after the `SASEVENT` tag
//! ([`yantra::input::EVENT_TAG`]) — the same tag-and-adjacent-global shape `SASINPUT` and
//! `SASARGV` use — and at each wait the host writes the next record of the event log into
//! that word and resumes. `yantra-run --events <log> <image>` replays a log; the format is
//! documented on [`yantra::input::parse_event_log`].
//!
//! The four falsifiers, each run against the REAL binary and each shown red before the
//! runner had the flag:
//!
//! - (a) a program that waits three times, summing what each wait delivered, answers 15 on
//!   the log `[3, 5, 7]`;
//! - (b) two replays of the same log give IDENTICAL retired counts and output digests;
//! - (c) a log SHORTER than the waits refuses naming the wait index — never a zero read —
//!   and a log LONGER than the waits refuses too, naming what was left unconsumed;
//! - (d) an image without the tag refuses at load when a log is given, before running.
//!
//! And the control the row demands: without `--events` a wait still exits 75.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sadhana::kosha;
use yantra::input::EVENT_TAG;
use yantra::{FINISHER, UART, WAIT};

// Encoders, written out for the reason `interpreter.rs` and `w370_halt_wait.rs` give: a
// test that derived its words from the encoder it checks would agree with it while
// disagreeing with the ISA.
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

/// How many times the program waits.
const WAITS: i32 = 3;
/// Word index of the wait store; the machine reports `pc = BASE + 4 * WAIT_AT` each time.
const WAIT_AT: u64 = 5;
const BASE: u64 = 0x8000_0000;
/// What the event word holds before any delivery. NOT zero, so a runner that resumed
/// without delivering would answer a sum of these rather than something that might pass
/// for an empty event.
const UNDELIVERED: u64 = 1000;

/// Waits [`WAITS`] times, adding the event word into `x6` after each wait, then prints the
/// sum as two decimal digits and a newline and writes the SUCCESS finisher. With `tagged`
/// false the tag word is zero: the same program, built without the event interface.
fn program(tagged: bool) -> Vec<u8> {
    assert_eq!(UART, 0x1000_0000, "the encoding below spells this address");
    assert_eq!(WAIT, UART + 0x100, "the encoding below spells this address");
    let words = [
        u(0x37, 5, 0x1000_0000),      // 0  x5 = UART
        addi(5, 5, 0x100),            // 1  x5 = WAIT
        u(0x17, 7, 0),                // 2  x7 = pc = BASE + 8
        addi(7, 7, 0),                // 3  x7 += (slot - 8), patched below
        addi(12, 0, WAITS),           // 4  x12 = waits left
        s(0x23, 3, 5, 0, 0),          // 5  WAIT                 <- loop top
        i(0x03, 8, 3, 7, 0),          // 6  x8 = the event word
        r(0x33, 6, 0, 6, 8, 0),       // 7  x6 += x8
        addi(12, 12, -1),             // 8
        b(1, 12, 0, -16),             // 9  bne x12, x0, loop top
        addi(13, 0, 10),              // 10 x13 = 10
        r(0x33, 14, 5, 6, 13, 1),     // 11 divu x14 = x6 / 10
        r(0x33, 15, 7, 6, 13, 1),     // 12 remu x15 = x6 % 10
        addi(14, 14, 48),             // 13 '0' +
        addi(15, 15, 48),             // 14 '0' +
        u(0x37, 16, 0x1000_0000),     // 15 x16 = UART
        s(0x23, 0, 16, 14, 0),        // 16 putc tens
        s(0x23, 0, 16, 15, 0),        // 17 putc ones
        addi(17, 0, 10),              // 18 '\n'
        s(0x23, 0, 16, 17, 0),        // 19 putc newline
        u(0x37, 10, FINISHER as u32), // 20 x10 = FINISHER
        u(0x37, 11, 0x5000),          // 21
        addi(11, 11, 0x555),          // 22 x11 = 0x5555, success
        s(0x23, 2, 10, 11, 0),        // 23 the finisher
    ];
    let mut words = words.to_vec();
    // The tag sits right after the code (8-aligned: 24 words), the slot right after it.
    let tag_at = words.len() * 4;
    let slot_at = tag_at + 8;
    words[3] = addi(7, 7, slot_at as i32 - 8);
    let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    assert_eq!(bytes.len(), tag_at);
    bytes.extend_from_slice(&(if tagged { EVENT_TAG } else { 0 }).to_le_bytes());
    bytes.extend_from_slice(&UNDELIVERED.to_le_bytes());
    bytes
}

/// A fresh directory per call: pid AND a clock, because a pid is reused (W-301).
fn scratch(what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w371-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Write the image and the log into `dir`, run `yantra-run [--events log] image`.
fn run(dir: &Path, tagged: bool, log: Option<&str>) -> Output {
    let image = dir.join("events.elf");
    std::fs::write(&image, kosha::write(&program(tagged))).expect("write the image");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_yantra-run"));
    cmd.current_dir(dir);
    if let Some(log) = log {
        let path = dir.join("events.log");
        std::fs::write(&path, log).expect("write the log");
        cmd.arg("--events").arg(&path);
    }
    cmd.arg(&image).output().expect("yantra-run runs")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The `steps:` line's count — the ratified metric (`T-102`).
fn retired(out: &Output) -> u64 {
    let err = stderr(out);
    err.lines()
        .find_map(|l| l.strip_prefix("steps: "))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no steps line:\n{err}"))
}

/// FNV-1a over the payload: a digest, not a security claim.
fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

const LOG: &str = "# W-371 falsifier: one record per wait\n3\n5\n7\n";

/// (a) Three waits, three records, the sum printed: `15`.
#[test]
fn three_waits_on_the_log_3_5_7_answer_15() {
    let dir = scratch("a");
    let out = run(&dir, true, Some(LOG));
    let _ = std::fs::remove_dir_all(&dir);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "a finished replay:\n{err}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "15\n",
        "3 + 5 + 7:\n{err}"
    );
    assert!(
        err.lines()
            .any(|l| l.starts_with("events: 3 of 3 delivered")),
        "the runner says what it delivered:\n{err}"
    );
}

/// (b) The retired count is a function of (program, log): two replays agree to the
/// instruction and to the octet. Also pinned against the program's own arithmetic, so two
/// runs that agreed on a wrong count would not pass: 5 setup words, 5 per wait, 14 after.
#[test]
fn two_replays_agree_on_the_count_and_the_output_digest() {
    let (d1, d2) = (scratch("b1"), scratch("b2"));
    let (one, two) = (run(&d1, true, Some(LOG)), run(&d2, true, Some(LOG)));
    let _ = (std::fs::remove_dir_all(&d1), std::fs::remove_dir_all(&d2));
    assert_eq!(one.status.code(), Some(0), "{}", stderr(&one));
    assert_eq!(two.status.code(), Some(0), "{}", stderr(&two));
    assert_eq!(retired(&one), retired(&two), "retired counts");
    assert_eq!(digest(&one.stdout), digest(&two.stdout), "output digests");
    assert_eq!(
        retired(&one),
        5 + 5 * WAITS as u64 + 14,
        "the program's own count"
    );
}

/// (c, short) Two records for three waits: the third wait REFUSES, naming its index, and
/// the program is never resumed onto a word nobody wrote.
#[test]
fn a_log_shorter_than_the_waits_refuses_naming_the_wait_index() {
    let dir = scratch("c1");
    let out = run(&dir, true, Some("3\n5\n"));
    let _ = std::fs::remove_dir_all(&dir);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "a refusal:\n{err}");
    let pc = format!("{:#x}", BASE + WAIT_AT * 4);
    assert!(
        err.lines().any(|l| l.starts_with("events: REFUSED")
            && l.contains("wait index 2")
            && l.contains(&pc)),
        "names wait index 2 and its pc:\n{err}"
    );
    assert!(
        out.stdout.is_empty(),
        "the program never resumed to print:\n{err}"
    );
    assert!(!err.contains("halt: Finisher"), "never a finish:\n{err}");
}

/// (c, long) Four records for three waits: REFUSED, not warned. A replay whose log was not
/// consumed exactly is not a replay of that log — the run it reproduces is some other run.
#[test]
fn a_log_longer_than_the_waits_refuses_naming_the_unconsumed_records() {
    let dir = scratch("c2");
    let out = run(&dir, true, Some("3\n5\n7\n9\n"));
    let _ = std::fs::remove_dir_all(&dir);
    let err = stderr(&out);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a refusal, not a warning:\n{err}"
    );
    assert!(
        err.lines().any(|l| l.starts_with("events: REFUSED")
            && l.contains("3 of 4")
            && l.contains("1 unconsumed")),
        "names what was left:\n{err}"
    );
}

/// (d) The image has no `SASEVENT` tag and a log was given: refused at load, before one
/// instruction runs.
#[test]
fn an_image_without_the_tag_refuses_at_load_when_a_log_is_given() {
    let dir = scratch("d");
    let out = run(&dir, false, Some(LOG));
    let _ = std::fs::remove_dir_all(&dir);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "a refusal:\n{err}");
    assert!(
        err.lines()
            .any(|l| l.starts_with("events: refused") && l.contains("SASEVENT")),
        "names the missing tag:\n{err}"
    );
    assert!(!err.contains("halt: "), "nothing ran:\n{err}");
    assert!(!err.contains("steps: "), "nothing ran:\n{err}");
}

/// The control: WITHOUT `--events` a wait still exits 75, tag or no tag — `W-370`'s
/// contract is unchanged by the image declaring the interface.
#[test]
fn without_a_log_a_wait_still_exits_75() {
    for tagged in [true, false] {
        let dir = scratch("ctl");
        let out = run(&dir, tagged, None);
        let _ = std::fs::remove_dir_all(&dir);
        let err = stderr(&out);
        assert_eq!(out.status.code(), Some(75), "tagged={tagged}:\n{err}");
        assert!(
            err.lines().any(|l| l.starts_with("yantra-run: WAIT")),
            "{err}"
        );
    }
}
