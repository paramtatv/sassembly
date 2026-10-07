//! **THE SOCKET DEVICE (`W-377`, ADR-0040 Option C, `docs/adr/0040-addendum-w377-sockets.md`).**
//!
//! A REGISTER DEVICE served through `W-350`'s two existing calls (addendum §1, owner
//! ruling Q2): no intrinsic, no name, no compiler change. One connection's received
//! octets wait in a queue the PROGRAM drains, one octet per pop, and a read answers an
//! octet, EMPTY or END as DATA — only the wait touches control flow (ADR-0040 `:174-177`).
//!
//! ```text
//!   SOCK + 0x0  NEXT  store, value 0   pop the queue's head into the RX latch
//!   SOCK + 0x4  RX    load             the latch: 0x000..=0x0ff an octet, 0x100 EMPTY,
//!                                      0x101 END (sticky), 0x102 UNREAD (no NEXT yet)
//!   SOCK + 0x8  TX    store            send the low octet (a value above 0xff refuses)
//!   SOCK + 0xc  —     any              reserved for CLOSE (deferred); refused by name
//! ```
//!
//! **A LOAD CANNOT POP** (addendum §0): `Machine::load` takes `&self`, so every read with
//! a side effect is a STORE (NEXT) and the load (RX) only reads latched state — `patra`'s
//! rule, and `COUNTER`'s.
//!
//! **THE QUEUE CHANGES ONLY AT A WAIT** (§2): [`deliver`] appends one record there, and
//! nowhere else, so between two waits every answer is a function of the octets already
//! delivered and the retired count is a function of `(program, log)`.
//!
//! **THIS FILE DOES NO NETWORKING.** The host that holds the connection is
//! `bin/yantra-run.rs`, and nothing else under `crates/yantra/src` may name one
//! (`tests/w377_sockets.rs` holds that ratchet). TX leaves through
//! [`crate::Output::sent`], so the machine never sees a stream.

use std::collections::VecDeque;

use crate::{Halt, Machine, Output, SOCK};

/// NEXT: a store of 0 pops the queue's head into the RX latch.
pub const NEXT: u64 = SOCK;
/// RX: a load answers the latch.
pub const RX: u64 = SOCK + 4;
/// TX: a store sends its low octet.
pub const TX: u64 = SOCK + 8;
/// Reserved for CLOSE, which is deferred (§1): any access refuses by name.
pub const CLOSE: u64 = SOCK + 0xc;
/// The window's length in octets: `SOCK..SOCK + SOCK_LEN`.
pub const SOCK_LEN: u64 = 16;

/// RX after a NEXT that found the queue drained while the peer has not closed — and the
/// answer before any client exists.
pub const EMPTY: u32 = 0x100;
/// RX after a NEXT that found the queue drained and the peer closed. STICKY: every later
/// NEXT answers it again.
pub const END: u32 = 0x101;
/// RX before any NEXT: `patra`'s untouched sentinel (`patra.rs:182-185`), so a program
/// that forgets NEXT reads a value no octet can be.
pub const UNREAD: u32 = 0x102;

/// The most octets one `s=<hex>` record may carry: the live host's read size (§2).
pub const RECORD_CAP: usize = 4096;

/// THE SESSION CAP (review of `05eb3a04`): the most octets one session may DELIVER, held in
/// the queue or in the log. A client that keeps sending while the program waits without
/// popping grew the queue to 68 MB and the log to 134 MB before this; the record that
/// would pass the cap is refused by name, never applied and never logged — live, in a
/// replay and in the interpreter alike.
pub const SESSION_CAP: u64 = 64 << 20;

/// THE CAP'S REFUSAL, BY NAME (owner ruling, 2026-10-06, pasted to the coordinator):
/// `SOCKET_LIMIT_EXCEEDED`, spelled in the refusal as below — copied byte for byte from
/// the ruling by script, never retyped. A host-side refusal: it appears in no `.t1` code.
pub const SOCKET_LIMIT_EXCEEDED: &str = "सङ्केतमात्राप्रतिषेधः";

/// The refusal of record `index` (`octets` long) that would take the session to `total`
/// delivered octets, past [`SESSION_CAP`] — one statement of the words for every engine.
#[must_use]
pub fn over_cap_refusal(index: usize, octets: usize, total: u64) -> String {
    format!(
        "{SOCKET_LIMIT_EXCEEDED} (SOCKET_LIMIT_EXCEEDED): record {index} ({octets} octets) \
         would take the session's delivered records past the cap of {SESSION_CAP} octets \
         (64 MiB), to {total} — refused, not applied and not logged (W-377)"
    )
}

/// Every refusal of the window, by name (`Halt::Device { why }`). Public so a second
/// engine (`nirvahana.rs`'s option (b)) can be checked to say the same words.
pub const WHY_NO_DEVICE: &str = "no socket device: pass `--listen` or a socket log (W-377)";
/// A width other than four, or an access not aligned to four.
pub const WHY_WIDTH: &str = "the socket device's registers are 32 bits wide and aligned: a four-octet access at \
     NEXT, RX or TX (W-377)";
/// A load at NEXT or TX.
pub const WHY_LOAD_STORE_ONLY: &str =
    "NEXT and TX are store-only registers: read the socket at RX (W-377)";
/// A store at RX.
pub const WHY_STORE_LOAD_ONLY: &str =
    "RX is a load-only register: store 0 to NEXT to pop the queue into it (W-377)";
/// NEXT with a value other than 0.
pub const WHY_NEXT_VALUE: &str =
    "NEXT takes the value 0 only: other values are reserved for a connection id (W-377)";
/// TX with a value above `0xff`.
pub const WHY_TX_VALUE: &str =
    "TX sends one octet: a value above 0xff is refused, never truncated (W-377)";
/// Any access at [`CLOSE`].
pub const WHY_CLOSE: &str = "0x1000_011c is reserved for CLOSE, which is deferred: the program cannot close the \
     connection in this row (W-377)";

/// Every name above, for a test that classifies a refusal.
pub const REFUSALS: [&str; 7] = [
    WHY_NO_DEVICE,
    WHY_WIDTH,
    WHY_LOAD_STORE_ONLY,
    WHY_STORE_LOAD_ONLY,
    WHY_NEXT_VALUE,
    WHY_TX_VALUE,
    WHY_CLOSE,
];

/// One record of a SOCKET log (§2): what the host delivered at one wait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SockRecord {
    /// `s=<hex>`: 1 to [`RECORD_CAP`] octets that arrived.
    Octets(Vec<u8>),
    /// `s=end`: the peer closed.
    End,
}

/// The device's state (§1). `Machine::socket` holds one, or `None` — no device, and
/// every access to the window halts naming `W-377`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socket {
    /// Octets delivered and not yet popped.
    pub queue: VecDeque<u8>,
    /// The peer closed (`s=end` was delivered). Never cleared.
    pub ended: bool,
    /// What RX answers: an octet, [`EMPTY`], [`END`] or [`UNREAD`].
    pub latch: u32,
    /// Octets delivered, over every record.
    pub received: u64,
    /// Octets the program sent through TX.
    pub sent: u64,
}

impl Default for Socket {
    fn default() -> Self {
        Self::new()
    }
}

impl Socket {
    /// A device with no client yet: an empty queue, not ended, the latch [`UNREAD`].
    #[must_use]
    pub fn new() -> Self {
        Socket {
            queue: VecDeque::new(),
            ended: false,
            latch: UNREAD,
            received: 0,
            sent: 0,
        }
    }

    /// One record, at a wait: octets appended to the queue, or `ended` set. Answers the
    /// number of octets appended — what the `SASEVENT` slot is given (0 for `s=end`).
    pub fn apply(&mut self, record: &SockRecord) -> u64 {
        match record {
            SockRecord::Octets(o) => {
                self.queue.extend(o.iter().copied());
                self.received += o.len() as u64;
                o.len() as u64
            }
            SockRecord::End => {
                self.ended = true;
                0
            }
        }
    }

    /// NEXT: the head into the latch, or EMPTY / END when the queue is drained.
    fn pop(&mut self) {
        self.latch = match self.queue.pop_front() {
            Some(o) => u32::from(o),
            None if self.ended => END,
            None => EMPTY,
        };
    }
}

/// A load at `SOCK + off`, `width` octets, against `socket` — the RX arm of
/// `Machine::load_walk`. Answers the latch, or the refusal by name.
///
/// # Errors
/// One of the `WHY_*` names: no device, a width or alignment other than four, a load
/// anywhere but RX.
pub fn load(socket: Option<&Socket>, off: u64, width: usize) -> Result<u32, &'static str> {
    let s = socket.ok_or(WHY_NO_DEVICE)?;
    if width != 4 || !off.is_multiple_of(4) {
        return Err(WHY_WIDTH);
    }
    match SOCK + off {
        RX => Ok(s.latch),
        CLOSE => Err(WHY_CLOSE),
        _ => Err(WHY_LOAD_STORE_ONLY),
    }
}

/// A store at `SOCK + off` — the NEXT and TX arms of `Machine::store_walk`. `value` is
/// the register; a four-octet store writes its low 32 bits, and those are what is judged.
///
/// # Errors
/// One of the `WHY_*` names: no device, a width or alignment other than four, a store at
/// RX or CLOSE, NEXT with a value other than 0, TX with a value above `0xff`.
pub fn store(
    socket: Option<&mut Socket>,
    off: u64,
    width: usize,
    value: u64,
    out: &mut impl Output,
) -> Result<(), &'static str> {
    let s = socket.ok_or(WHY_NO_DEVICE)?;
    if width != 4 || !off.is_multiple_of(4) {
        return Err(WHY_WIDTH);
    }
    let word = value & 0xffff_ffff;
    match SOCK + off {
        NEXT if word != 0 => Err(WHY_NEXT_VALUE),
        NEXT => {
            s.pop();
            Ok(())
        }
        TX if word > 0xff => Err(WHY_TX_VALUE),
        TX => {
            s.sent += 1;
            out.sent(word as u8);
            Ok(())
        }
        CLOSE => Err(WHY_CLOSE),
        _ => Err(WHY_STORE_LOAD_ONLY),
    }
}

/// DELIVER ONE RECORD AT A WAIT — the one place the queue grows, shared by the live host
/// (`yantra-run --listen`) and [`replay_socket`]: the record applied to `m.socket` (made
/// if the machine has none), and the octet count written into the `SASEVENT` slot at
/// `tag + 8` (advisory; the device is the authority on EMPTY versus END). Answers the
/// count.
pub fn deliver(m: &mut Machine, tag: usize, record: &SockRecord) -> u64 {
    let n = m.socket.get_or_insert_with(Socket::new).apply(record);
    m.mem[tag + 8..tag + 16].copy_from_slice(&n.to_le_bytes());
    n
}

/// The cap (review of `05eb3a04`): `Err` with [`over_cap_refusal`]'s words when delivering
/// `record` as record `index` would take `m`'s session past [`SESSION_CAP`] delivered
/// octets. Checked BEFORE the record is applied — and, live, before it is logged — so the
/// queue and the log both stop at the cap.
///
/// # Errors
/// The refusal, by name.
pub fn check_cap(m: &Machine, record: &SockRecord, index: usize) -> Result<(), String> {
    let SockRecord::Octets(o) = record else {
        return Ok(());
    };
    let total = m.socket.as_ref().map_or(0, |s| s.received) + o.len() as u64;
    if total > SESSION_CAP {
        return Err(over_cap_refusal(index, o.len(), total));
    }
    Ok(())
}

/// Whether `m`'s device has seen `s=end`.
#[must_use]
pub fn ended(m: &Machine) -> bool {
    m.socket.as_ref().is_some_and(|s| s.ended)
}

/// How a socket replay ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SockReplayed {
    /// The machine halted for a reason that is not a wait, after `delivered` records.
    /// A caller holding a longer log judges it, as for [`crate::input::replay`].
    Halted {
        /// The halt.
        halt: Halt,
        /// Records delivered, one per wait.
        delivered: usize,
    },
    /// The program waited and the log had no record left — never padded.
    Short {
        /// Which wait, from 0.
        index: usize,
        /// The wait store.
        pc: u64,
    },
    /// The record due at wait `index` would pass [`SESSION_CAP`]; it was not applied.
    OverCap {
        /// Which wait, from 0.
        index: usize,
        /// The wait store.
        pc: u64,
        /// [`over_cap_refusal`]'s words.
        why: String,
    },
    /// The program waited after END: there is no other source to wait for (§3), so the
    /// wait is refused, live and in replay alike.
    AfterEnd {
        /// Which wait, from 0.
        index: usize,
        /// The wait store.
        pc: u64,
    },
}

/// Replay a SOCKET log against `m`: run; at each [`Halt::Wait`] [`deliver`] the next
/// record and run again; stop at any other halt. [`crate::input::replay`]'s contract
/// (§2): `budget` covers the whole replay, a SHORT log refuses naming the wait index and
/// pc and is never padded, and a LONG log is the caller's to refuse once the program has
/// ended. A wait after END is [`SockReplayed::AfterEnd`]. `m.socket` is made if absent.
pub fn replay_socket(
    m: &mut Machine,
    tag: usize,
    log: &[SockRecord],
    budget: u64,
    out: &mut impl Output,
) -> SockReplayed {
    m.socket.get_or_insert_with(Socket::new);
    let start = m.time;
    let mut delivered = 0;
    loop {
        let left = budget.saturating_sub(m.time - start);
        match m.run(left, out) {
            Halt::Wait { pc } => {
                if ended(m) {
                    return SockReplayed::AfterEnd {
                        index: delivered,
                        pc,
                    };
                }
                let Some(record) = log.get(delivered) else {
                    return SockReplayed::Short {
                        index: delivered,
                        pc,
                    };
                };
                if let Err(why) = check_cap(m, record, delivered) {
                    return SockReplayed::OverCap {
                        index: delivered,
                        pc,
                        why,
                    };
                }
                deliver(m, tag, record);
                delivered += 1;
            }
            halt => return SockReplayed::Halted { halt, delivered },
        }
    }
}

/// The first line of a socket log `yantra-run --listen` writes: a comment.
pub const SOCKET_LOG_HEADER: &str = "# yantra-run --record-events --listen (W-377): one socket \
     record per wait, s=<hex octets> or s=end; replay with --events";

/// One record as its log line, without the newline: `s=<lowercase hex>` or `s=end`.
#[must_use]
pub fn record_line(record: &SockRecord) -> String {
    match record {
        SockRecord::End => "s=end".to_string(),
        SockRecord::Octets(o) => {
            let mut line = String::with_capacity(2 + 2 * o.len());
            line.push_str("s=");
            for b in o {
                line.push_str(&format!("{b:02x}"));
            }
            line
        }
    }
}

/// Whether `text` is a SOCKET log: it starts with [`SOCKET_LOG_HEADER`] — so a live run
/// that never reached a wait, and logged no record, still replays WITH its device (review
/// of `05eb3a04`, approved) — or its first record line (not blank, not `#`) starts with
/// `s=`. `yantra-run` chooses the parser from this (§2).
#[must_use]
pub fn is_socket_log(text: &str) -> bool {
    text.starts_with(SOCKET_LOG_HEADER)
        || text
            .lines()
            .map(str::trim)
            .find(|t| !t.is_empty() && !t.starts_with('#'))
            .is_some_and(|t| t.starts_with("s="))
}

/// Read a SOCKET log (§2): `#` comments and blank lines skipped, every other line
/// `s=<hex>` (1 to [`RECORD_CAP`] octets, two LOWERCASE hex digits each) or `s=end`.
///
/// # Errors
/// Naming the line: a torn final record, an empty `s=`, an odd length or a non-hex (or
/// uppercase) digit, more than [`RECORD_CAP`] octets, any record after `s=end`, and a
/// plain, `t=` or `@` line — no mixed logs in this row.
pub fn parse_socket_log(text: &str) -> Result<Vec<SockRecord>, String> {
    crate::input::refuse_torn_final_record(text)?;
    let mut records = Vec::new();
    let mut ended_at: Option<usize> = None;
    for (n, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let line_no = n + 1;
        let Some(body) = t.strip_prefix("s=") else {
            let kind = if t.starts_with('@') {
                "a THREAD record (W-376)"
            } else if t.starts_with("t=") {
                "a CLOCK record (W-375)"
            } else {
                "not a socket record"
            };
            return Err(format!(
                "line {line_no}: {t:?} is {kind} — a socket log holds only s=<hex> and s=end \
                 lines (W-377: no mixed logs)"
            ));
        };
        if let Some(at) = ended_at {
            return Err(format!(
                "line {line_no}: {t:?} comes after the s=end at line {at} — the peer had closed, \
                 so no record can follow it"
            ));
        }
        if body == "end" {
            ended_at = Some(line_no);
            records.push(SockRecord::End);
            continue;
        }
        if body.is_empty() {
            return Err(format!(
                "line {line_no}: an empty s= — a record carries 1 to {RECORD_CAP} octets, and \
                 the peer's close is s=end"
            ));
        }
        if !body.len().is_multiple_of(2) {
            return Err(format!(
                "line {line_no}: s= with {} hex digits — two per octet, so the length must be even",
                body.len()
            ));
        }
        if let Some(c) = body.chars().find(|c| !matches!(c, '0'..='9' | 'a'..='f')) {
            return Err(format!(
                "line {line_no}: {c:?} in s= is not a lowercase hex digit (0-9, a-f)"
            ));
        }
        let octets: Vec<u8> = (0..body.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&body[i..i + 2], 16).expect("checked hex above"))
            .collect();
        if octets.len() > RECORD_CAP {
            return Err(format!(
                "line {line_no}: s= carries {} octets — a record holds at most {RECORD_CAP}, the \
                 live host's read size",
                octets.len()
            ));
        }
        records.push(SockRecord::Octets(octets));
    }
    Ok(records)
}

/// The `socket:` report line both modes end with (§3), from the device's counts, the
/// records delivered and the octets sent — `sent sha256` of exactly those octets.
#[must_use]
pub fn report_line(socket: Option<&Socket>, records: usize, sent: &[u8]) -> String {
    let (received, count) = socket.map_or((0, 0), |s| (s.received, s.sent));
    let digest: String = crate::smp::sha256(sent)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!(
        "socket: {received} octets received in {records} records, {count} sent, sent sha256 \
         {digest}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_is_the_rulings() {
        assert_eq!(
            (NEXT, RX, TX, CLOSE),
            (0x1000_0110, 0x1000_0114, 0x1000_0118, 0x1000_011c)
        );
        assert_eq!((EMPTY, END, UNREAD), (0x100, 0x101, 0x102));
    }

    #[test]
    fn a_record_line_reads_back() {
        let r = vec![
            SockRecord::Octets(vec![0, 0x41, 0xff]),
            SockRecord::Octets(vec![7]),
            SockRecord::End,
        ];
        let text: String = r.iter().map(|x| record_line(x) + "\n").collect();
        assert_eq!(text, "s=0041ff\ns=07\ns=end\n");
        assert_eq!(parse_socket_log(&text).unwrap(), r);
        assert!(is_socket_log(&format!("{SOCKET_LOG_HEADER}\n{text}")));
        assert!(!is_socket_log("# x\n3\ns=41\n"));
    }
}
