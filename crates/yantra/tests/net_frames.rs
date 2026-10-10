//! **ADR-0047 (PROPOSED): THE NETWORK FRAME DEVICE.** A virtio-net device that moves raw
//! Ethernet frames and nothing else, driven here by small raw RISC-V guests (no `.t1`).
//!
//! - (a) the feature negotiation a guest does, register by register, and the refusal of a
//!   bit it was not offered;
//! - (b) the window refused by name with no `--net-*` flag, in the library and the binary;
//! - (c) the caps: `--net-max-frame` and the TX and RX frame counts, each refusing by name;
//! - (d) replay: a live run against a scripted backend, then its log replayed with NO
//!   backend, identical in output, instruction count, memory and transmitted frames;
//! - (e) a frame round trip between TWO `yantra-run` processes over `--net-peer=PATH` (a unix
//!   socket), then each process's log replayed alone, identical; a silent peer and
//!   `--net-timeout`;
//! - (g) a THREADED run: two threads, the frame going to the one that waits, live then replay;
//! - (h) the MAC allow-list.
//! - (f) the log format, and the other readers' refusal of an `n=` record by name.

use sadhana::kosha;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use yantra::input::{EVENT_TAG, parse_event_log, parse_thread_log};
use yantra::netdev::{
    self, FrameBackend, NET_FRAME_TOO_LARGE, NET_RX_COUNT_EXCEEDED, NET_SRC_MAC_REFUSED,
    NET_TIMEOUT, NET_TX_COUNT_EXCEEDED, NET_WINDOW_REFUSED, NetConfig, NetRecord, NetReplayed,
    NetThreads, OFFERED, VirtioNet, parse_net_log, record_live_net, replay_net,
};
use yantra::threads::{self, THREAD_ID_TAG, THREADS_TAG, Threads, ThreadsEnd};
use yantra::{FINISHER, Halt, Machine, Output, UART, WAIT};

const NET: u64 = 0x1000_3000;
const RES: u64 = 0x40000;
const RXQ: u64 = 0x10000;
const TXQ: u64 = 0x20000;
const RXBUF: u64 = 0x30000;
const TXBUF: u64 = 0x31000;
const NUM: u64 = 8;

fn i(op: u32, rd: u32, f3: u32, rs1: u32, imm: i32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn s(op: u32, f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    op | (imm & 0x1f) << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | (imm >> 5 & 0x7f) << 25
}
fn lui(rd: u32, imm: u32) -> u32 {
    0x37 | rd << 7 | (imm & 0xffff_f000)
}

#[derive(Default)]
struct Sink {
    out: Vec<u8>,
}
impl Output for Sink {
    fn putc(&mut self, b: u8) {
        self.out.push(b);
    }
}

fn raw_image(code: &[u32]) -> Vec<u8> {
    let mut text: Vec<u8> = code.iter().flat_map(|w| w.to_le_bytes()).collect();
    text.extend_from_slice(&EVENT_TAG.to_le_bytes());
    text.extend_from_slice(&0u64.to_le_bytes());
    kosha::write(&text)
}

fn ram_base() -> u64 {
    let img = raw_image(&[0x13]);
    Machine::load_elf(&img, 1 << 20).unwrap().base
}

/// A guest, assembled from register accesses.
struct Prog {
    c: Vec<u32>,
    base: u64,
}

impl Prog {
    fn new() -> Self {
        Self {
            c: Vec::new(),
            base: ram_base(),
        }
    }
    fn li(&mut self, rd: u32, v: u64) {
        let v32 = v as u32;
        let lo = ((v32 & 0xfff) as i32) << 20 >> 20;
        self.c.push(lui(rd, v32.wrapping_sub(lo as u32)));
        self.c.push(i(0x1b, rd, 0, rd, lo));
        if v32 >= 0x8000_0000 {
            self.c.push(i(0x13, rd, 1, rd, 32));
            self.c.push(i(0x13, rd, 5, rd, 32));
        }
    }
    fn st(&mut self, width: usize, addr: u64, v: u64) {
        self.li(5, addr);
        self.li(6, v);
        let f3 = match width {
            1 => 0,
            2 => 1,
            4 => 2,
            _ => unreachable!(),
        };
        self.c.push(s(0x23, f3, 5, 6, 0));
    }
    fn reg_w(&mut self, off: u64, v: u64) {
        self.st(4, NET + off, v);
    }
    fn reg_r(&mut self, off: u64, width: usize, dst: u64) {
        self.li(5, NET + off);
        let (ld, sf) = match width {
            1 => (4, 0),
            2 => (5, 1),
            _ => (2, 2),
        };
        self.c.push(i(0x03, 7, ld, 5, 0));
        self.li(8, self.base + dst);
        self.c.push(s(0x23, sf, 8, 7, 0));
    }
    fn wait(&mut self) {
        self.li(5, WAIT);
        self.c.push(s(0x23, 3, 5, 0, 0));
    }
    fn print(&mut self, addr: u64) {
        self.li(5, self.base + addr);
        self.c.push(i(0x03, 7, 4, 5, 0));
        self.li(8, UART);
        self.c.push(s(0x23, 0, 8, 7, 0));
    }
    fn finish(&mut self) {
        self.li(10, FINISHER);
        self.li(11, 0x5555);
        self.c.push(s(0x23, 2, 10, 11, 0));
    }
    /// The driver's negotiation and queue set-up, as virtio 1.2 §4.2.3.1 lists it for the
    /// legacy interface; results land at `RES`.
    fn negotiate(&mut self, ack_word0: u64) {
        self.reg_r(0x000, 4, RES);
        self.reg_r(0x004, 4, RES + 4);
        self.reg_r(0x008, 4, RES + 8);
        self.reg_w(0x070, 0);
        self.reg_w(0x070, 1);
        self.reg_w(0x070, 3);
        self.reg_w(0x014, 0);
        self.reg_r(0x010, 4, RES + 0xc);
        self.reg_w(0x014, 1);
        self.reg_r(0x010, 4, RES + 0x10);
        self.reg_w(0x024, 0);
        self.reg_w(0x020, ack_word0);
        self.reg_w(0x028, 4096);
        for q in 0..2 {
            self.reg_w(0x030, q);
            self.reg_r(0x034, 4, RES + 0x14 + 4 * q);
            self.reg_w(0x038, NUM);
            self.reg_w(0x03c, 4096);
            let at = if q == 0 { RXQ } else { TXQ };
            self.reg_w(0x040, (self.base + at) >> 12);
        }
        for k in 0..12 {
            self.reg_r(0x100 + k, 1, RES + 0x20 + k);
        }
        self.reg_w(0x070, 7);
    }
    /// A receive buffer posted: descriptor 0, device-writable, `len` octets.
    fn post_rx(&mut self, len: u64) {
        let b = self.base;
        self.st(4, b + RXQ, b + RXBUF);
        self.st(4, b + RXQ + 8, len);
        self.st(2, b + RXQ + 12, 2);
        self.st(2, b + RXQ + 128 + 4, 0);
        self.st(2, b + RXQ + 128 + 2, 1);
        self.reg_w(0x050, 0);
    }
    /// `frames` queued for transmit, one descriptor each, then the notify.
    fn post_tx(&mut self, frames: &[Vec<u8>]) {
        let b = self.base;
        for (k, f) in frames.iter().enumerate() {
            let k = k as u64;
            let buf = b + TXBUF + k * 0x400;
            for (j, byte) in f.iter().enumerate() {
                self.st(1, buf + 10 + j as u64, u64::from(*byte));
            }
            self.st(4, b + TXQ + 16 * k, buf);
            self.st(4, b + TXQ + 16 * k + 8, 10 + f.len() as u64);
            self.st(2, b + TXQ + 128 + 4 + 2 * k, k);
        }
        self.st(2, b + TXQ + 128 + 2, frames.len() as u64);
        self.reg_w(0x050, 1);
    }
    fn image(&self) -> Vec<u8> {
        raw_image(&self.c)
    }
}

fn frame(src: u8, payload: &[u8]) -> Vec<u8> {
    let mut f = vec![0x02, 0, 0, 0, 0, 0xff, 0x02, 0, 0, 0, 0, src, 0x88, 0xb5];
    f.extend_from_slice(payload);
    f
}

fn machine(img: &[u8], config: Option<NetConfig>) -> Machine {
    let mut m = Machine::load_elf(img, 1 << 20).unwrap();
    m.net = config.map(VirtioNet::new);
    m
}

fn word(m: &Machine, at: u64) -> u32 {
    let off = (RES + at) as usize;
    u32::from_le_bytes(m.mem[off..off + 4].try_into().unwrap())
}

fn device_halt(h: &Halt) -> (u64, &'static str) {
    match h {
        Halt::Device { addr, why, .. } => (*addr, why),
        other => panic!("expected a Device halt, got {other:?}"),
    }
}

// ── (a) negotiation ─────────────────────────────────────────────────────────

#[test]
fn a_guest_negotiates_mtu_mac_status_and_reads_the_device_it_expects() {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.finish();
    let mut m = machine(&p.image(), Some(NetConfig::default()));
    let h = m.run(100_000, &mut Sink::default());
    assert!(matches!(h, Halt::Finisher { .. }), "{h:?}");
    assert_eq!(word(&m, 0), 0x7472_6976, "MagicValue");
    assert_eq!(word(&m, 4), 1, "legacy Version");
    assert_eq!(word(&m, 8), 1, "DeviceID is virtio-net");
    assert_eq!(
        word(&m, 0xc),
        OFFERED,
        "HostFeatures word 0: MTU, MAC, STATUS"
    );
    assert_eq!(OFFERED, 1 << 3 | 1 << 5 | 1 << 16);
    assert_eq!(
        word(&m, 0x10),
        0,
        "HostFeatures word 1 is empty (no VERSION_1)"
    );
    assert_eq!(word(&m, 0x14), 256, "receiveq QueueNumMax");
    assert_eq!(word(&m, 0x18), 256, "transmitq QueueNumMax");
    let cfg: Vec<u8> = (0..12).map(|k| m.mem[(RES + 0x20 + k) as usize]).collect();
    assert_eq!(cfg[..6], netdev::DEFAULT_MAC);
    assert_eq!(u16::from_le_bytes([cfg[6], cfg[7]]), 1, "status: LINK_UP");
    assert_eq!(
        u16::from_le_bytes([cfg[10], cfg[11]]),
        1500,
        "mtu = max frame - 14"
    );
    let n = m.net.as_ref().unwrap();
    assert_eq!(n.driver_features, OFFERED);
    assert_eq!(n.status, 7);
}

#[test]
fn a_guest_that_acknowledges_a_bit_it_was_not_offered_is_refused_by_name() {
    for bit in [1u64 << 0, 1 << 1, 1 << 15, 1 << 17] {
        let mut p = Prog::new();
        p.negotiate(u64::from(netdev::F_MAC) | bit);
        p.finish();
        let mut m = machine(&p.image(), Some(NetConfig::default()));
        let (addr, why) = device_halt(&m.run(100_000, &mut Sink::default()));
        assert_eq!(addr, NET + 0x020);
        assert!(why.contains("does not offer"), "{why}");
    }
}

// ── (b) the window without a flag ───────────────────────────────────────────

#[test]
fn the_window_is_refused_by_name_without_a_net_device() {
    let mut p = Prog::new();
    p.reg_r(0x000, 4, RES);
    p.finish();
    let mut m = machine(&p.image(), None);
    let (addr, why) = device_halt(&m.run(10_000, &mut Sink::default()));
    assert_eq!((addr, why), (NET, NET_WINDOW_REFUSED));
    // And a STORE, which takes the other path.
    let mut p = Prog::new();
    p.reg_w(0x070, 1);
    p.finish();
    let mut m = machine(&p.image(), None);
    let (addr, why) = device_halt(&m.run(10_000, &mut Sink::default()));
    assert_eq!((addr, why), (NET + 0x70, NET_WINDOW_REFUSED));
    assert!(
        why.contains("no virtio device"),
        "the older empty-slot wording survives"
    );
}

// ── (c) the caps ────────────────────────────────────────────────────────────

#[test]
fn a_transmitted_frame_over_the_cap_is_refused_by_name_and_not_sent() {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_tx(&[frame(1, &[0; 10])]);
    p.finish();
    let cfg = NetConfig {
        max_frame: 20,
        ..NetConfig::default()
    };
    let mut m = machine(&p.image(), Some(cfg));
    let (_, why) = device_halt(&m.run(200_000, &mut Sink::default()));
    assert_eq!(why, NET_FRAME_TOO_LARGE);
    assert!(netdev::take_tx(&mut m).is_empty());
    // The same frame under the default cap goes out.
    let mut m = machine(&p.image(), Some(NetConfig::default()));
    assert!(matches!(
        m.run(200_000, &mut Sink::default()),
        Halt::Finisher { .. }
    ));
    assert_eq!(netdev::take_tx(&mut m), vec![frame(1, &[0; 10])]);
}

#[test]
fn the_tx_frame_count_cap_refuses_the_frame_that_passes_it() {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_tx(&[frame(1, &[1; 4]), frame(2, &[2; 4]), frame(3, &[3; 4])]);
    p.finish();
    let cfg = NetConfig {
        max_tx: 2,
        ..NetConfig::default()
    };
    let mut m = machine(&p.image(), Some(cfg));
    let (_, why) = device_halt(&m.run(300_000, &mut Sink::default()));
    assert_eq!(why, NET_TX_COUNT_EXCEEDED);
    assert_eq!(
        netdev::take_tx(&mut m).len(),
        2,
        "the two within the cap stay sent"
    );
}

#[test]
fn a_received_frame_over_either_cap_is_refused_and_not_applied() {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_rx(2048);
    p.finish();
    let cfg = NetConfig {
        max_frame: 30,
        max_rx: 1,
        ..NetConfig::default()
    };
    let mut m = machine(&p.image(), Some(cfg));
    assert!(matches!(
        m.run(300_000, &mut Sink::default()),
        Halt::Finisher { .. }
    ));
    let b = m.base;
    let net = m.net.as_mut().unwrap();
    let big = frame(9, &[0; 40]);
    let e = net.receive(&mut m.mem, b, &big).unwrap_err();
    assert!(e.contains("पिण्डातिदीर्घनिषेधः"), "{e}");
    assert_eq!(net.rx_seen, 0, "a refused frame is not counted");
    assert_eq!(net.receive(&mut m.mem, b, &frame(9, &[7; 4])).unwrap(), 18);
    let e = net.receive(&mut m.mem, b, &frame(9, &[7; 4])).unwrap_err();
    assert!(e.contains(NET_RX_COUNT_EXCEEDED), "{e}");
    let e = net.receive(&mut m.mem, b, &[0; 5]).unwrap_err();
    assert!(e.contains("पिण्डावैधरूपम्"), "{e}");
}

#[test]
fn a_frame_with_no_buffer_or_a_small_one_is_dropped_and_counted() {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_rx(16); // smaller than 10 + the frame
    p.finish();
    let mut m = machine(&p.image(), Some(NetConfig::default()));
    m.run(300_000, &mut Sink::default());
    let b = m.base;
    let net = m.net.as_mut().unwrap();
    assert_eq!(net.receive(&mut m.mem, b, &frame(1, &[0; 8])).unwrap(), 0);
    assert_eq!((net.rx_frames, net.rx_dropped), (0, 1));
    // No queue at all.
    let mut fresh = VirtioNet::new(NetConfig::default());
    assert_eq!(fresh.receive(&mut m.mem, b, &frame(1, &[0; 8])).unwrap(), 0);
    assert_eq!(fresh.rx_dropped, 1);
}

// ── (d) replay ──────────────────────────────────────────────────────────────

struct Scripted {
    frames: Vec<Vec<u8>>,
    sent: std::rc::Rc<std::cell::RefCell<Vec<Vec<u8>>>>,
    /// Whether a poll finds a frame already here (else only a blocking wait does).
    ready: bool,
}
impl FrameBackend for Scripted {
    fn send(&mut self, f: &[u8]) {
        self.sent.borrow_mut().push(f.to_vec());
    }
    fn recv(&mut self) -> Result<Vec<u8>, String> {
        if self.frames.is_empty() {
            Err(NET_TIMEOUT.to_string())
        } else {
            Ok(self.frames.remove(0))
        }
    }
    fn try_recv(&mut self) -> Result<Option<Vec<u8>>, String> {
        Ok(if self.ready && !self.frames.is_empty() {
            Some(self.frames.remove(0))
        } else {
            None
        })
    }
}

/// Post a buffer, WAIT, print the frame's payload, transmit a reply, WAIT, print again.
fn echo_guest() -> Prog {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_rx(2048);
    p.wait();
    for k in 0..4 {
        p.print(RXBUF + 10 + 14 + k);
    }
    p.post_tx(&[frame(5, b"PONG")]);
    p.wait();
    p.print(RES + 0x20);
    p.finish();
    p
}

fn tag_of(m: &Machine) -> usize {
    yantra::input::find_event_slot(&m.mem).unwrap()
}

#[test]
fn a_live_run_replays_identically_with_no_backend() {
    let img = echo_guest().image();
    let mut live = machine(&img, Some(NetConfig::default()));
    let tag = tag_of(&live);
    let sent_by_live = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut backend = Scripted {
        frames: vec![frame(7, b"PING"), frame(7, b"LATE")],
        sent: sent_by_live.clone(),
        ready: false,
    };
    let mut log = Vec::new();
    let mut out = Sink::default();
    let (halt, n) = record_live_net(
        &mut live,
        tag,
        1_000_000,
        &mut out,
        &mut log,
        &mut backend,
        &mut Vec::new(),
    )
    .unwrap();
    assert!(matches!(halt, Halt::Finisher { .. }), "{halt:?}");
    assert_eq!(n, 2);
    assert_eq!(&out.out[..4], b"PING");
    assert_eq!(*sent_by_live.borrow(), vec![frame(5, b"PONG")]);

    let text = String::from_utf8(log).unwrap();
    assert!(text.starts_with("# yantra-net v1 mac=02:53:41:4e:53:00 max-frame=1514 max-tx=65536 max-rx=65536 filter=0"), "{text}");
    assert!(
        text.contains("\nn=0:0:") && text.contains("\nn=1:0:"),
        "{text}"
    );
    let parsed = parse_net_log(&text).unwrap();
    assert_eq!(parsed.records.len(), 2);
    assert_eq!(parsed.config, NetConfig::default());

    // Replay twice, on a machine built from the log's header alone.
    let mut results = Vec::new();
    for _ in 0..2 {
        let mut m = machine(&img, Some(parsed.config));
        let mut out = Sink::default();
        let mut sent = Vec::new();
        let mut tx = |f: &[u8]| sent.push(f.to_vec());
        let r = replay_net(&mut m, tag, &parsed.records, 1_000_000, &mut out, &mut tx);
        assert!(
            matches!(
                r,
                NetReplayed::Halted {
                    delivered: 2,
                    halt: Halt::Finisher { .. }
                }
            ),
            "{r:?}"
        );
        results.push((out.out, m.time, m.mem.clone(), sent));
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[0].0, out.out, "same output as the live run");
    assert_eq!(
        results[0].1, live.time,
        "same instruction count as the live run"
    );
    assert_eq!(results[0].2, live.mem, "same memory as the live run");
    assert_eq!(results[0].3, *sent_by_live.borrow(), "same frames out");
}

#[test]
fn a_short_log_is_never_padded_and_a_capped_record_is_refused() {
    let img = echo_guest().image();
    let mut m = machine(&img, Some(NetConfig::default()));
    let tag = tag_of(&m);
    let r = replay_net(
        &mut m,
        tag,
        &[NetRecord::None],
        1_000_000,
        &mut Sink::default(),
        &mut |_| {},
    );
    assert!(matches!(r, NetReplayed::Short { index: 1, .. }), "{r:?}");
    let cfg = NetConfig {
        max_frame: 20,
        ..NetConfig::default()
    };
    let mut m = machine(&img, Some(cfg));
    let r = replay_net(
        &mut m,
        tag,
        &[NetRecord::Frame(frame(1, &[0; 30]))],
        1_000_000,
        &mut Sink::default(),
        &mut |_| {},
    );
    assert!(
        matches!(&r, NetReplayed::Refused { index: 0, why, .. } if why.contains("पिण्डातिदीर्घनिषेधः")),
        "{r:?}"
    );
}

// ── (f) the log format ──────────────────────────────────────────────────────

#[test]
fn the_log_reads_back_and_the_other_readers_refuse_it_by_name() {
    let recs = vec![NetRecord::Frame(frame(1, b"abcd")), NetRecord::None];
    let mut text = netdev::log_header(&NetConfig::default());
    for (k, r) in recs.iter().enumerate() {
        text.push('\n');
        text.push_str(&netdev::record_line(k, r));
    }
    text.push('\n');
    assert!(netdev::is_net_log(&text));
    assert_eq!(parse_net_log(&text).unwrap().records, recs);
    assert!(parse_net_log(&text).unwrap().schedule.is_none());
    for r in [
        parse_event_log("n=0:0:none\n").unwrap_err(),
        parse_thread_log("n=0:0:none\n").unwrap_err(),
    ] {
        assert!(r.contains("NET record"), "{r}");
    }
    for bad in [
        "n=0\n",
        "n=0:0:0G\n",
        "n=0:0:AB12\n",
        "n=0:0:\n",
        "7\n",
        "t=5\n",
        "s=00\n",
        "@x\n",
        "n=1:0:none\n", // wait 1 before wait 0: spliced
        "n=0:1:none\n", // a handle this device does not have
    ] {
        let e = parse_net_log(bad).unwrap_err();
        assert!(e.starts_with("line 1:"), "{bad:?}: {e}");
    }
    assert!(parse_net_log("n=0:0:none").unwrap_err().contains("torn"));
    let dup = parse_net_log("n=0:0:none\nn=0:0:none\n").unwrap_err();
    assert!(
        dup.starts_with("line 2:") && dup.contains("अभिलेखसङ्केतभेदः"),
        "{dup}"
    );
    let mismatch = parse_net_log("n=1:0:none\n").unwrap_err();
    assert!(mismatch.contains("अभिलेखसङ्केतभेदः"), "{mismatch}");
    // A threaded log keeps its schedule.
    let t = parse_net_log("@0\n@0\nn=0:0:none\n@1\n").unwrap();
    assert_eq!(t.schedule.as_ref().unwrap().len(), 4);
}

// ── (b), (e) the real binary ────────────────────────────────────────────────

fn scratch(what: &str) -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let d = std::env::temp_dir().join(format!(
        "net-frames-{what}-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct Ran {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
}
impl Ran {
    fn line(&self, p: &str) -> String {
        self.stderr
            .lines()
            .find(|l| l.starts_with(p))
            .unwrap_or_else(|| panic!("no {p:?} in {}", self.stderr))
            .to_string()
    }
}

fn run(args: &[&str]) -> Ran {
    let o = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .args(args)
        .output()
        .unwrap();
    Ran {
        code: o.status.code(),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

#[test]
fn the_binary_refuses_the_window_and_every_unlogged_or_unbacked_net_run() {
    let d = scratch("refuse");
    let mut p = Prog::new();
    p.reg_r(0x000, 4, RES);
    p.finish();
    let elf = d.join("a.elf");
    std::fs::write(&elf, p.image()).unwrap();
    let e = elf.to_str().unwrap();
    let r = run(&[e]);
    assert!(
        r.stderr.contains("सञ्चाराभावः") && r.stderr.contains("no net device"),
        "{}",
        r.stderr
    );
    let log = d.join("l").to_str().unwrap().to_string();
    let sock = d.join("p.sock").to_str().unwrap().to_string();
    // A net flag outside a logged live run.
    for args in [
        vec!["--net-peer", &sock, e],
        vec!["--events", &log, "--net-peer", &sock, e],
    ] {
        let r = run(&args);
        assert_eq!(r.code, Some(1), "{}", r.stderr);
        assert!(r.stderr.contains("net: refused"), "{}", r.stderr);
    }
    // No backend (the mac alone), two backends, a bad cap, an unknown flag, and a TAP that
    // cannot be attached: this host is unprivileged and has no such interface.
    for (args, word) in [
        (
            vec!["--record-events", &log, "--net-mac", "02:00:00:00:00:01", e],
            "no bare --net",
        ),
        (
            vec![
                "--record-events",
                &log,
                "--net-peer",
                &sock,
                "--net-tap",
                "x",
                e,
            ],
            "pick one",
        ),
        (
            vec![
                "--record-events",
                &log,
                "--net-peer",
                &sock,
                "--net-max-frame",
                "3",
                e,
            ],
            "must be 14 to 65535",
        ),
        (
            vec![
                "--record-events",
                &log,
                "--net-peer",
                &sock,
                "--net-bogus",
                "1",
                e,
            ],
            "unknown flag",
        ),
        (
            vec!["--record-events", &log, "--net-tap", "nosuchif0", e],
            "आधारानुपलब्धम्",
        ),
    ] {
        let r = run(&args);
        assert_eq!(r.code, Some(1), "{args:?}: {}", r.stderr);
        assert!(
            r.stderr.contains("net: refused") && r.stderr.contains(word),
            "{args:?}: {}",
            r.stderr
        );
    }
}

#[test]
fn a_silent_peer_times_out_by_name_under_an_explicit_net_timeout() {
    use std::os::unix::net::UnixListener;
    let d = scratch("timeout");
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_rx(2048);
    p.wait();
    p.finish();
    let elf = d.join("a.elf");
    std::fs::write(&elf, p.image()).unwrap();
    let sock = d.join("silent.sock");
    let _l = UnixListener::bind(&sock).unwrap(); // connects, never sends
    let r = run(&[
        "--record-events",
        d.join("l").to_str().unwrap(),
        &format!("--net-peer={}", sock.display()),
        "--net-timeout=300",
        elf.to_str().unwrap(),
    ]);
    assert_eq!(r.code, Some(1), "{}", r.stderr);
    assert!(r.stderr.contains("सञ्चारकालातीतम्"), "{}", r.stderr);
    let _ = std::fs::remove_dir_all(&d);
}

fn spawn(args: &[&str]) -> (std::process::Child, std::sync::mpsc::Receiver<String>) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let err = c.stderr.take().unwrap();
    std::thread::spawn(move || {
        for l in BufReader::new(err).lines().map_while(Result::ok) {
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    (c, rx)
}

#[test]
fn two_instances_exchange_frames_over_the_peer_and_each_log_replays_alone() {
    let d = scratch("peer");
    // B receives A's frame, prints its payload, replies. A sends, waits for the reply.
    let mut b = Prog::new();
    b.negotiate(u64::from(OFFERED));
    b.post_rx(2048);
    b.wait();
    for k in 0..4 {
        b.print(RXBUF + 10 + 14 + k);
    }
    b.post_tx(&[frame(0xb, b"PONG")]);
    b.finish();
    let mut a = Prog::new();
    a.negotiate(u64::from(OFFERED));
    a.post_rx(2048);
    a.post_tx(&[frame(0x0a, b"PING")]);
    a.wait();
    for k in 0..4 {
        a.print(RXBUF + 10 + 14 + k);
    }
    a.finish();
    let (ea, eb) = (d.join("a.elf"), d.join("b.elf"));
    std::fs::write(&ea, a.image()).unwrap();
    std::fs::write(&eb, b.image()).unwrap();
    let (la, lb) = (d.join("a.log"), d.join("b.log"));
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
    let (sa, sb, sla, slb) = (s(&ea), s(&eb), s(&la), s(&lb));
    let rendezvous = format!("--net-peer={}", d.join("peer.sock").display());

    let (cb, rb) = spawn(&[
        "--record-events",
        &slb,
        &rendezvous,
        "--net-max-frame",
        "1514",
        &sb,
    ]);
    loop {
        let l = rb
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("B's line");
        if l.contains("net: peer") && l.contains("listening") {
            break;
        }
    }
    let ra = run(&["--record-events", &sla, &rendezvous, &sa]);
    let ob = cb.wait_with_output().unwrap();
    let rb_err: String = rb.try_iter().collect::<Vec<_>>().join("\n");

    assert_eq!(ra.code, Some(0), "{}", ra.stderr);
    assert_eq!(ob.status.code(), Some(0), "{rb_err}");
    assert_eq!(ra.stdout, b"PONG", "A printed B's reply");
    assert_eq!(ob.stdout, b"PING", "B printed A's frame");
    assert!(
        ra.line("net: 1 frames").contains("transmitted, 1 received"),
        "{}",
        ra.stderr
    );
    assert!(
        !ra.line("net: 1 frames").contains("e3b0c442"),
        "the sent digest is over real frames"
    );

    for (elf, log, live_out, live_err) in [
        (&sa, &sla, ra.stdout.clone(), ra.stderr.clone()),
        (&sb, &slb, ob.stdout.clone(), rb_err.clone()),
    ] {
        let text = std::fs::read_to_string(log).unwrap();
        assert!(
            text.contains("\nn=") && text.starts_with("# yantra-net v1 "),
            "{text}"
        );
        let r1 = run(&["--events", log, elf]);
        let r2 = run(&["--events", log, elf]);
        assert_eq!(r1.code, Some(0), "{}", r1.stderr);
        assert_eq!(r1.stdout, live_out);
        assert_eq!(r1.stdout, r2.stdout);
        assert_eq!(r1.line("steps:"), r2.line("steps:"));
        let steps = |e: &str| {
            e.lines()
                .find(|l| l.starts_with("steps:"))
                .unwrap()
                .to_string()
        };
        assert_eq!(
            r1.line("steps:"),
            steps(&live_err),
            "same instruction count as the live run"
        );
        let netline = |e: &str| {
            e.lines()
                .find(|l| l.starts_with("net: ") && l.contains("sent sha256"))
                .unwrap()
                .to_string()
        };
        assert_eq!(netline(&r1.stderr), netline(&live_err), "same frames out");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ── (h) the MAC allow-list ──────────────────────────────────────────────────

#[test]
fn the_mac_allow_list_refuses_a_foreign_source_and_filters_foreign_destinations() {
    let mine = [0x02, 0, 0, 0, 0, 0x0a];
    let cfg = NetConfig {
        mac: mine,
        mac_filter: true,
        ..NetConfig::default()
    };
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_rx(2048);
    p.post_tx(&[frame(0x0a, b"okay")]);
    p.finish();
    let mut m = machine(&p.image(), Some(cfg));
    assert!(matches!(
        m.run(300_000, &mut Sink::default()),
        Halt::Finisher { .. }
    ));
    assert_eq!(
        netdev::take_tx(&mut m).len(),
        1,
        "a frame from the allow-listed MAC goes out"
    );
    let mut q = Prog::new();
    q.negotiate(u64::from(OFFERED));
    q.post_tx(&[frame(0x0b, b"nope")]);
    q.finish();
    let mut m2 = machine(&q.image(), Some(cfg));
    let (_, why) = device_halt(&m2.run(300_000, &mut Sink::default()));
    assert_eq!(why, NET_SRC_MAC_REFUSED);
    // RX: addressed to 02:..:ff (not mine, not a group) is filtered; to me, or broadcast, is not.
    let b = m.base;
    let net = m.net.as_mut().unwrap();
    assert_eq!(net.receive(&mut m.mem, b, &frame(1, &[0; 4])).unwrap(), 0);
    assert_eq!((net.rx_filtered, net.rx_frames), (1, 0));
    let mut to_me = frame(1, &[0; 4]);
    to_me[..6].copy_from_slice(&mine);
    assert_eq!(net.receive(&mut m.mem, b, &to_me).unwrap(), 18);
    // The filter is off without --net-mac.
    assert!(!NetConfig::default().mac_filter);
}

// ── (g) a threaded run ──────────────────────────────────────────────────────

fn beq_ne(rs1: u32, off: i32) -> u32 {
    // bne rs1, x0, off
    let o = off as u32;
    0x63 | 1 << 12
        | rs1 << 15
        | (o >> 11 & 1) << 7
        | (o >> 1 & 0xf) << 8
        | (o >> 5 & 0x3f) << 25
        | (o >> 12 & 1) << 31
}

/// Two threads. Thread 0 sets up the device, posts a buffer, WAITS, prints the payload.
/// Thread 1 prints `T`. Each ends at its own finisher.
fn threaded_image(id_addr: u64) -> Vec<u8> {
    let mut t0 = Prog::new();
    t0.negotiate(u64::from(OFFERED));
    t0.post_rx(2048);
    t0.wait();
    for k in 0..4 {
        t0.print(RXBUF + 10 + 14 + k);
    }
    t0.finish();
    let mut t1 = Prog::new();
    t1.li(6, u64::from(b'T'));
    t1.li(8, UART);
    t1.c.push(s(0x23, 0, 8, 6, 0));
    t1.finish();
    let mut head = Prog::new();
    head.c.push(0x17 | 2 << 7);
    head.c.push(i(0x13, 2, 0, 2, 0));
    head.li(5, id_addr);
    head.c.push(i(0x03, 9, 3, 5, 0));
    head.c.push(beq_ne(9, 4 * (t0.c.len() as i32 + 1)));
    head.c.extend(&t0.c);
    head.c.extend(&t1.c);
    let mut text: Vec<u8> = head.c.iter().flat_map(|w| w.to_le_bytes()).collect();
    for (tag, v) in [(THREADS_TAG, 2u64), (THREAD_ID_TAG, 99), (EVENT_TAG, 0)] {
        text.extend_from_slice(&tag.to_le_bytes());
        text.extend_from_slice(&v.to_le_bytes());
    }
    kosha::write(&text)
}

fn threaded_machine() -> (Machine, Threads, Vec<u8>) {
    // The id slot's address depends on the code's length, which does not depend on it.
    let probe = threaded_image(0x8000_0000);
    let m = Machine::load_elf(&probe, 1 << 20).unwrap();
    let d = threads::discover(&m, &probe).unwrap().unwrap();
    let img = threaded_image(m.base + d.id_tag as u64 + 8);
    let mut m = Machine::load_elf(&img, 1 << 20).unwrap();
    let d = threads::discover(&m, &img).unwrap().unwrap();
    let t = Threads::new(&mut m, d).unwrap();
    (m, t, img)
}

#[test]
fn a_threaded_run_delivers_the_frame_to_the_waiting_thread_and_replays_identically() {
    for ready in [false, true] {
        let (mut m, mut t, img) = threaded_machine();
        m.net = Some(VirtioNet::new(NetConfig::default()));
        let sent = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut nt = NetThreads::live(Box::new(Scripted {
            frames: vec![frame(7, b"PING")],
            sent,
            ready,
        }));
        let mut log = Vec::new();
        let mut out = Sink::default();
        let end = threads::record_live_threads_net(
            &mut m, &mut t, 2_000_000, &mut out, &mut log, &mut nt,
        )
        .unwrap();
        assert_eq!(end, ThreadsEnd::Ended, "{end:?}");
        // Ready: the poll wakes thread 0 at once. Not ready: thread 1 runs, then the host blocks.
        let want: &[u8] = if ready { b"PINGT" } else { b"TPING" };
        assert_eq!(out.out, want, "ready={ready}");
        let text = String::from_utf8(log).unwrap();
        let schedule: Vec<&str> = text
            .lines()
            .filter(|l| l.starts_with('@') || l.starts_with("n="))
            .collect();
        assert!(schedule.iter().any(|l| l.starts_with("n=0:0:")), "{text}");
        assert_eq!(schedule.iter().filter(|l| l.starts_with("n=")).count(), 1);

        let parsed = parse_net_log(&text).unwrap();
        let sched = parsed.schedule.clone().expect("a threaded log");
        let mut replays = Vec::new();
        for _ in 0..2 {
            let (mut m2, mut t2, img2) = threaded_machine();
            assert_eq!(img, img2);
            m2.net = Some(VirtioNet::new(parsed.config));
            let mut nt2 = NetThreads::replay(parsed.records.clone());
            let mut out2 = Sink::default();
            let end2 = threads::replay_threads_net(
                &mut m2, &mut t2, &sched, 2_000_000, &mut out2, &mut nt2,
            );
            assert_eq!(end2, ThreadsEnd::Ended, "{end2:?}");
            replays.push((out2.out, m2.time, m2.mem.clone()));
        }
        assert_eq!(replays[0], replays[1]);
        assert_eq!(replays[0].0, out.out, "same output as the live run");
        assert_eq!(
            replays[0].1, m.time,
            "same instruction count as the live run"
        );
        assert_eq!(replays[0].2, m.mem, "same memory as the live run");

        // And through the real binary, with no flag but --events.
        let d = scratch("threaded");
        std::fs::write(d.join("t.elf"), &img).unwrap();
        std::fs::write(d.join("t.log"), &text).unwrap();
        let r = run(&[
            "--events",
            d.join("t.log").to_str().unwrap(),
            d.join("t.elf").to_str().unwrap(),
        ]);
        assert_eq!(r.stdout, out.out, "{}", r.stderr);
        assert!(
            r.line("steps:").contains(&m.time.to_string()),
            "{}",
            r.stderr
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}

// ── the TAP backend (Linux; needs a pre-made interface and root for the far end) ─────────

/// `SAS_TAP_IF=sastap0 cargo test --test net_frames -- --ignored tap`, after (as root)
/// `ip tuntap add dev sastap0 mode tap user $UID; sysctl -w net.ipv6.conf.sastap0.disable_ipv6=1;
/// ip link set sastap0 up` (IPv6 off, or the kernel's router solicitation is the first frame
/// the wait hears). Run once on a Linux x86_64 host 2026-10-07: passed. A root helper
/// (`sudo -n python3`) plays the host's network: it captures yantra's PING frame off the
/// interface and answers with PONG. Ignored by default: it needs privilege on the far end.
#[test]
#[ignore = "probe: needs a pre-made TAP interface named by SAS_TAP_IF and passwordless sudo"]
fn tap_a_frame_leaves_and_a_frame_arrives_and_the_log_replays() {
    let Ok(ifname) = std::env::var("SAS_TAP_IF") else {
        panic!("set SAS_TAP_IF to the name of a TAP interface made for this user");
    };
    let d = scratch("tap");
    let mut g = Prog::new();
    g.negotiate(u64::from(OFFERED));
    g.post_rx(2048);
    g.post_tx(&[frame(0x0a, b"PING")]);
    g.wait();
    for k in 0..4 {
        g.print(RXBUF + 10 + 14 + k);
    }
    g.finish();
    std::fs::write(d.join("g.elf"), g.image()).unwrap();
    let script = format!(
        "import socket\ns=socket.socket(socket.AF_PACKET,socket.SOCK_RAW,socket.htons(0x88b5))\n\
         s.bind(('{ifname}',0))\nprint('ready',flush=True)\n\
         while True:\n  f=s.recv(2048)\n  if f[14:18]==b'PING':\n    print(f.hex(),flush=True)\n    break\n\
         s.send(bytes.fromhex('ff'*6+'020000000099')+b'\\x88\\xb5PONG')\n"
    );
    std::fs::write(d.join("h.py"), script).unwrap();
    let mut helper = Command::new("sudo")
        .args(["-n", "python3", d.join("h.py").to_str().unwrap()])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(helper.stdout.take().unwrap()).lines();
    assert_eq!(lines.next().unwrap().unwrap(), "ready");
    let log = d.join("g.log");
    let r = run(&[
        "--record-events",
        log.to_str().unwrap(),
        &format!("--net-tap={ifname}"),
        "--net-timeout=15000",
        d.join("g.elf").to_str().unwrap(),
    ]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.stdout, b"PONG");
    let seen = lines.next().unwrap().unwrap();
    assert!(
        seen.ends_with(&hex(&frame(0x0a, b"PING"))),
        "the helper captured {seen}"
    );
    let _ = helper.wait();
    let again = run(&[
        "--events",
        log.to_str().unwrap(),
        d.join("g.elf").to_str().unwrap(),
    ]);
    assert_eq!(again.stdout, b"PONG");
    assert_eq!(again.line("steps:"), r.line("steps:"));
    let _ = std::fs::remove_dir_all(&d);
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ── deferred completion (YANTRA_VIRTIO_DEFER), TX and RX ────────────────────

const TXUSED: u64 = TXQ + 4096 + 2;
const RXUSED: u64 = RXQ + 4096 + 2;
const SEEN: u64 = RES + 0x40;

impl Prog {
    /// `*(u16*)(base+src)` stored at `base+dst` (one read, as the mutant driver does).
    fn read_u16(&mut self, src: u64, dst: u64) {
        self.li(5, self.base + src);
        self.c.push(i(0x03, 7, 5, 5, 0));
        self.li(8, self.base + dst);
        self.c.push(s(0x23, 1, 8, 7, 0));
    }
    /// Spin until the u16 at `base+src` is nonzero (the polling driver).
    fn poll_nonzero(&mut self, src: u64) {
        self.li(5, self.base + src);
        self.c.push(i(0x03, 7, 5, 5, 0)); // lhu x7, 0(x5)
        // beq x7, x0, -4
        let imm: u32 = (-4i32) as u32 & 0x1fff;
        self.c.push(
            (imm >> 12 & 1) << 31
                | (imm >> 5 & 0x3f) << 25
                | 7 << 15
                | (imm >> 1 & 0xf) << 8
                | (imm >> 11 & 1) << 7
                | 0x63,
        );
    }
}

fn seen(m: &Machine) -> u16 {
    u16::from_le_bytes([m.mem[SEEN as usize], m.mem[SEEN as usize + 1]])
}

fn tx_guest(poll: bool) -> Vec<u8> {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_tx(&[frame(1, b"DEFR")]);
    if poll {
        p.poll_nonzero(TXUSED);
    }
    p.read_u16(TXUSED, SEEN);
    p.finish();
    p.image()
}

#[test]
fn a_single_read_tx_driver_passes_the_synchronous_device_and_fails_the_deferred_one() {
    let sync = NetConfig::default();
    let slow = NetConfig {
        defer: 1000,
        ..sync
    };
    for (what, poll, cfg, want) in [
        ("single read, synchronous", false, sync, 1u16),
        ("single read, deferred (the C-015 mutant)", false, slow, 0),
        ("polling, synchronous", true, sync, 1),
        ("polling, deferred", true, slow, 1),
    ] {
        let mut m = machine(&tx_guest(poll), Some(cfg));
        let h = m.run(2_000_000, &mut Sink::default());
        assert!(matches!(h, Halt::Finisher { .. }), "{what}: {h:?}");
        assert_eq!(seen(&m), want, "{what}: used.idx as the driver saw it");
        netdev::settle(&mut m);
        // The frame leaves either way: a deferred TX is settled at the halt.
        assert_eq!(netdev::take_tx(&mut m).len(), 1, "{what}");
    }
}

fn rx_guest(poll: bool) -> Vec<u8> {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_rx(2048);
    p.wait();
    if poll {
        p.poll_nonzero(RXUSED);
    }
    p.read_u16(RXUSED, SEEN);
    for k in 0..4 {
        p.print(RXBUF + 10 + 14 + k);
    }
    p.finish();
    p.image()
}

#[test]
fn a_single_read_rx_driver_passes_the_synchronous_device_and_fails_the_deferred_one() {
    let sync = NetConfig::default();
    let slow = NetConfig {
        defer: 1000,
        ..sync
    };
    for (what, poll, cfg, want_seen, want_out) in [
        ("single read, synchronous", false, sync, 1u16, &b"PING"[..]),
        ("single read, deferred", false, slow, 0, &b"\0\0\0\0"[..]),
        ("polling, deferred", true, slow, 1, &b"PING"[..]),
    ] {
        let img = rx_guest(poll);
        let mut live = machine(&img, Some(cfg));
        let tag = tag_of(&live);
        let mut backend = Scripted {
            frames: vec![frame(7, b"PING")],
            sent: Default::default(),
            ready: false,
        };
        let mut log = Vec::new();
        let mut out = Sink::default();
        let (halt, _) = record_live_net(
            &mut live,
            tag,
            2_000_000,
            &mut out,
            &mut log,
            &mut backend,
            &mut Vec::new(),
        )
        .unwrap();
        assert!(matches!(halt, Halt::Finisher { .. }), "{what}: {halt:?}");
        assert_eq!(seen(&live), want_seen, "{what}");
        assert_eq!(out.out, want_out, "{what}");
        // The header records the delay; the replay is rebuilt from it and is identical.
        let text = String::from_utf8(log).unwrap();
        assert!(
            text.contains(&format!("defer={} backend=none", cfg.defer)),
            "{text}"
        );
        let parsed = parse_net_log(&text).unwrap();
        assert_eq!(parsed.config, cfg);
        let mut m = machine(&img, Some(parsed.config));
        let mut out2 = Sink::default();
        let r = replay_net(
            &mut m,
            tag,
            &parsed.records,
            2_000_000,
            &mut out2,
            &mut |_| {},
        );
        assert!(matches!(r, NetReplayed::Halted { .. }), "{r:?}");
        assert_eq!(
            (out2.out, m.time, m.mem.clone()),
            (out.out, live.time, live.mem.clone()),
            "{what}: replay"
        );
    }
}

// ── no indefinite hang ──────────────────────────────────────────────────────

struct NeverBlocks;
impl FrameBackend for NeverBlocks {
    fn send(&mut self, _: &[u8]) {}
    fn recv(&mut self) -> Result<Vec<u8>, String> {
        panic!("a wait with no RX buffer posted must not ask the backend for a frame")
    }
    fn try_recv(&mut self) -> Result<Option<Vec<u8>>, String> {
        panic!("a wait with no RX buffer posted must not poll the backend")
    }
}

fn tx_only_guest() -> Vec<u8> {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_tx(&[frame(1, b"ONLY")]);
    p.wait();
    p.li(6, u64::from(b'K'));
    p.li(8, UART);
    p.c.push(s(0x23, 0, 8, 6, 0));
    p.finish();
    p.image()
}

#[test]
fn a_tx_only_wait_returns_at_once_with_no_frame_and_logs_none() {
    for defer in [0, 500] {
        let img = tx_only_guest();
        let mut m = machine(
            &img,
            Some(NetConfig {
                defer,
                ..NetConfig::default()
            }),
        );
        let tag = tag_of(&m);
        let mut log = Vec::new();
        let mut out = Sink::default();
        let mut sent = Vec::new();
        let (halt, n) = record_live_net(
            &mut m,
            tag,
            2_000_000,
            &mut out,
            &mut log,
            &mut NeverBlocks,
            &mut sent,
        )
        .unwrap();
        assert!(matches!(halt, Halt::Finisher { .. }), "{halt:?}");
        assert_eq!((n, out.out.as_slice()), (1, &b"K"[..]));
        assert!(String::from_utf8(log).unwrap().contains("\nn=0:0:none\n"));
        assert!(
            !sent.is_empty(),
            "the frame left at the wait, deferred or not"
        );
    }
}

/// Run `yantra-run`, failing (killing the child by its exact PID) rather than hanging.
fn run_guarded(args: &[&str], secs: u64) -> Ran {
    let mut c = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    loop {
        if c.try_wait().unwrap().is_some() {
            break;
        }
        if std::time::Instant::now() > deadline {
            c.kill().unwrap();
            let _ = c.wait();
            panic!("yantra-run {args:?} HUNG for {secs} s (killed): a TX-only wait must return");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let o = c.wait_with_output().unwrap();
    Ran {
        code: o.status.code(),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

#[test]
fn a_tx_only_guest_finishes_through_the_binary_and_a_changed_replay_config_is_refused() {
    use std::io::Read;
    use std::os::unix::net::UnixListener;
    let d = scratch("txonly");
    let elf = d.join("t.elf");
    std::fs::write(&elf, tx_only_guest()).unwrap();
    let sock = d.join("p.sock");
    let l = UnixListener::bind(&sock).unwrap();
    let got = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut len = [0u8; 2];
        s.read_exact(&mut len).unwrap();
        let mut f = vec![0u8; usize::from(u16::from_be_bytes(len))];
        s.read_exact(&mut f).unwrap();
        f // then holds the connection open and says nothing
    });
    let log = d.join("t.log");
    let r = run_guarded(
        &[
            "--record-events",
            log.to_str().unwrap(),
            &format!("--net-peer={}", sock.display()),
            elf.to_str().unwrap(),
        ],
        20,
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.stdout, b"K");
    assert_eq!(got.join().unwrap(), frame(1, b"ONLY"));
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.starts_with("# yantra-net v1 ") && text.contains(" backend=peer"),
        "{text}"
    );
    // Replay alone: identical. Replay under a different completion delay: refused by name.
    let again = run_guarded(
        &["--events", log.to_str().unwrap(), elf.to_str().unwrap()],
        20,
    );
    assert_eq!(
        (again.code, again.stdout.as_slice()),
        (Some(0), &b"K"[..]),
        "{}",
        again.stderr
    );
    let o = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .args(["--events", log.to_str().unwrap(), elf.to_str().unwrap()])
        .env("YANTRA_VIRTIO_DEFER", "7")
        .output()
        .unwrap();
    let e = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{e}");
    assert!(
        e.contains("recorded with defer=0") && e.contains("backend=peer"),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ── N7: replay must deliver log[k] at wait k, not log[0] at every wait ──────

/// Three RX buffers posted, then three waits with different amounts of work between them;
/// after each wait the guest prints the payload of that wait's buffer, and at the end it
/// transmits a frame carrying a byte from each.
fn three_frame_guest() -> Vec<u8> {
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    let b = p.base;
    for k in 0..3u64 {
        let d = b + RXQ + 16 * k;
        p.st(4, d, b + RXBUF + 0x800 * k);
        p.st(4, d + 8, 2048);
        p.st(2, d + 12, 2);
        p.st(2, b + RXQ + 128 + 4 + 2 * k, k);
    }
    p.st(2, b + RXQ + 128 + 2, 3);
    p.reg_w(0x050, 0);
    for k in 0..3u64 {
        p.wait();
        for _ in 0..=k {
            p.st(4, b + RES + 0x80, k); // filler: a different step count per round
        }
        for j in 0..4 {
            p.print(RXBUF + 0x800 * k + 10 + 14 + j);
        }
    }
    p.post_tx(&[frame(9, b"ZZZZ")]);
    p.finish();
    p.image()
}

#[test]
fn three_distinct_frames_replay_with_the_right_frame_at_each_wait() {
    let img = three_frame_guest();
    let payloads: [&[u8; 4]; 3] = [b"AAAA", b"BBBB", b"CCCC"];
    let mut live = machine(&img, Some(NetConfig::default()));
    let tag = tag_of(&live);
    let mut backend = Scripted {
        frames: payloads.iter().map(|p| frame(7, *p)).collect(),
        sent: Default::default(),
        ready: false,
    };
    let sent_live = backend.sent.clone();
    let mut log = Vec::new();
    let mut out = Sink::default();
    let (halt, n) = record_live_net(
        &mut live,
        tag,
        2_000_000,
        &mut out,
        &mut log,
        &mut backend,
        &mut Vec::new(),
    )
    .unwrap();
    assert!(matches!(halt, Halt::Finisher { .. }), "{halt:?}");
    assert_eq!(n, 3);
    assert_eq!(
        out.out, b"AAAABBBBCCCC",
        "live: each wait got its own frame"
    );
    let parsed = parse_net_log(&String::from_utf8(log).unwrap()).unwrap();
    assert_eq!(parsed.records.len(), 3);
    let mut m = machine(&img, Some(parsed.config));
    let mut out2 = Sink::default();
    let mut sent = Vec::new();
    let mut tx = |f: &[u8]| sent.push(f.to_vec());
    let r = replay_net(&mut m, tag, &parsed.records, 2_000_000, &mut out2, &mut tx);
    assert!(
        matches!(r, NetReplayed::Halted { delivered: 3, .. }),
        "{r:?}"
    );
    assert_eq!(out2.out, out.out, "output");
    assert_eq!(m.time, live.time, "instruction count");
    assert_eq!(m.mem, live.mem, "memory");
    assert_eq!(sent, *sent_live.borrow(), "TX frames");
}

#[test]
fn two_notifies_while_one_is_pending_still_send_both_frames() {
    // Defer 1000: the first notify is pending when the second arrives; the one deferred
    // serve must drain both descriptors.
    let mut p = Prog::new();
    p.negotiate(u64::from(OFFERED));
    p.post_tx(&[frame(1, b"FRST")]);
    let b = p.base;
    let f = frame(2, b"SCND");
    let buf = b + TXBUF + 0x400;
    for (j, byte) in f.iter().enumerate() {
        p.st(1, buf + 10 + j as u64, u64::from(*byte));
    }
    p.st(4, b + TXQ + 16, buf);
    p.st(4, b + TXQ + 16 + 8, 10 + f.len() as u64);
    p.st(2, b + TXQ + 128 + 4 + 2, 1);
    p.st(2, b + TXQ + 128 + 2, 2);
    p.reg_w(0x050, 1);
    p.finish();
    let mut m = machine(
        &p.image(),
        Some(NetConfig {
            defer: 1000,
            ..NetConfig::default()
        }),
    );
    assert!(matches!(
        m.run(2_000_000, &mut Sink::default()),
        Halt::Finisher { .. }
    ));
    assert!(netdev::settle(&mut m).is_none());
    assert_eq!(
        netdev::take_tx(&mut m),
        vec![frame(1, b"FRST"), frame(2, b"SCND")]
    );
}
