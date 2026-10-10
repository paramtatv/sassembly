//! THE NETWORK LIBRARY IN `.t1` (ADR-0048): `spec/net/*.t1`, the virtio-net driver and the
//! layers on it — Ethernet, ARP, IPv4, ICMP echo — built by `t1_image` and run on `yantra-run`
//! over the frame device (ADR-0047).
//!
//! The library is NOT in the compiler's corpus: the eight files and the demo program are listed
//! on the `t1_image` command line, as `spec/darshaka.t1` is. The compiler image does not change.
//!
//! WHAT IS PROVED, each on the real binaries:
//!
//! - (a) THE SELF-TEST agrees on both engines. With no argument `spec/net/net-demo.t1` never
//!   touches a device; `t1_image`'s differential gate runs it in the interpreter and natively
//!   and must say `AGREED` over the same 154 printed octets. The octets are then compared with
//!   frames this file builds by itself: the ARP request, and the echo request with its Internet
//!   checksum, from the layout in RFC 826 / 791 / 792. Thirty-eight checks, all passing, include the IPv4 header length under 5 and a total length under the header's, each of the six ARP checks alone, the capacity guards on short arrays,
//!   a bad IPv4 checksum, a bad ICMP checksum and five malformed packets refused by their word.
//! - (b) TWO `yantra-run`s over `--net-peer=PATH`: the responder answers ARP and the echo, the
//!   pinger pings, and BOTH logs replay with no flag to the same output, the same instruction
//!   count and the same frames sent.
//! - (c) THE SAME PAIR UNDER `YANTRA_VIRTIO_DEFER=1000`: a driver that read a used index once
//!   would fail; the bounded poll passes, and the logs replay (under the same deferral).
//! - (d) REFUSALS BY NAME, from replay logs this file crafts out of the recorded one: an echo
//!   reply with a bad ICMP checksum and one with a bad IPv4 checksum are `परीक्षायोगभेदः`
//!   (0x362); one whose total length passes the frame is `पिण्डावैधरूपम्` (0x363); four waits
//!   that bring nothing are `स्थाननिर्देशकालातीतम्` (0x364). The program prints the word
//!   and the name and leaves with the CODE, so yantra-run's finisher value is the word.
//! - (f) THE BOUNDED POLL: two one-read mutants of the driver pass the synchronous device and
//!   are refused by name on the deferred one.
//! - (g) THE DRIVER'S OWN REGION: a GPU draw (`spec/darshaka.t1`) and then a ping in one run
//!   succeeds; the same on the old shared region with the rings not zeroed does not. A send
//!   after a refused one is itself refused (cause 8) until the first is used. The 16-bit
//!   available and used indices wrap across 65540 sends and 65540 receives.
//! - (e) NO DEVICE WITHOUT A FLAG: the driver's first touch of slot 2 halts with `सञ्चाराभावः`.
//!
//! IGNORED by default, like `virtio_defer_darshaka.rs`: it builds ELFs with `t1_image` and runs
//! `yantra-run`, which a per-commit gate has no release binaries for. Run with
//! `cargo test --release -p yantra --test net_t1 -- --include-ignored` after
//! `cargo build --release -p sadhana --bin t1_image -p yantra --bin yantra-run`.
//! `CARGO_TARGET_DIR` is honoured.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// The driver's region is at 0xA800_0000 (`spec/net/sanchara-pankti.t1`'s margin).
const RAM: &str = "738197504";
const FILES: [&str; 9] = [
    "sanchara-pankti.t1",
    "sambandha.t1",
    "marga.t1",
    "marga-sandesha.t1",
    "sthana-nirdesha.t1",
    "nihsandhi-pinda.t1",
    "nama-nirdesha.t1",
    "sanchara.t1",
    "net-demo.t1",
];
/// The library: every file but the demo program.
const LIB: usize = 8;
/// code << 16 | 0x3333 (ADR-0048).
const fn word(code: u64) -> u64 {
    (code << 16) | 0x3333
}
const CHECKSUM: u64 = word(0x362);
const MALFORMED: u64 = word(0x363);
const ARP_TIMEOUT: u64 = word(0x364);
const NO_REPLY: u64 = word(0x36d);

fn root() -> PathBuf {
    // NET_T1_ROOT: a tree laid out like the repository whose spec/ and crates/sadhana-t1/src
    // the tests build from (the mutation runs point it at a mutated copy of spec/net)
    match std::env::var_os("NET_T1_ROOT") {
        Some(r) => PathBuf::from(r),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    }
}

fn bin_dir() -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(t) => PathBuf::from(t).join("release"),
        None => root().join("target/release"),
    }
}

fn scratch(what: &str) -> PathBuf {
    // pid AND a clock: a pid is REUSED and these roots are never removed (W-301).
    let d = std::env::temp_dir().join(format!(
        "net-t1-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

/// The two binaries, or `None` after saying what is missing.
fn tools() -> Option<(PathBuf, PathBuf)> {
    let d = bin_dir();
    let (t1, run) = (d.join("t1_image"), d.join("yantra-run"));
    if t1.exists() && run.exists() {
        Some((t1, run))
    } else {
        eprintln!(
            "skipped: build with `cargo build --release -p sadhana --bin t1_image -p yantra --bin yantra-run` first ({})",
            d.display()
        );
        None
    }
}

/// Build the demo program with the library; answers `(elf, t1_image's log)`.
fn build(t1_image: &Path, dir: &Path) -> (PathBuf, String) {
    build_from(t1_image, dir, "net.elf", None)
}

/// The same, with `driver` standing in for `spec/net/sanchara-pankti.t1` (a mutant).
fn build_from(t1_image: &Path, dir: &Path, name: &str, driver: Option<&Path>) -> (PathBuf, String) {
    build_files(t1_image, dir, name, driver, false)
}

/// The GPU draw, then a ping: `spec/darshaka.t1`, the six library files and `net-gpu.t1`.
fn build_gpu(t1_image: &Path, dir: &Path, name: &str, driver: Option<&Path>) -> (PathBuf, String) {
    build_files(t1_image, dir, name, driver, true)
}

fn build_files(
    t1_image: &Path,
    dir: &Path,
    name: &str,
    driver: Option<&Path>,
    gpu: bool,
) -> (PathBuf, String) {
    let root = root();
    let net = root.join("spec/net");
    let mut names: Vec<&str> = FILES[..LIB].to_vec();
    names.push(if gpu { "net-gpu.t1" } else { "net-demo.t1" });
    let mut files: Vec<PathBuf> = names
        .iter()
        .map(|f| match driver {
            Some(d) if *f == "sanchara-pankti.t1" => d.to_path_buf(),
            _ => net.join(f),
        })
        .collect();
    if gpu {
        files.insert(0, root.join("spec/darshaka.t1"));
    }
    let entry = if gpu {
        "सञ्चारचित्रपरीक्षा"
    } else {
        "सञ्चारपरीक्षा"
    };
    let out = dir.join(name);
    let mut c = Command::new(t1_image);
    c.arg("--spec-root")
        .arg(root.join("spec"))
        .arg("--compiler")
        .arg(root.join("crates/sadhana-t1/src"));
    for f in &files {
        c.arg("--load").arg(f);
    }
    c.args(["--entry", entry, "मुख्यम्", "-o"])
        .arg(&out)
        .args(&files);
    let o = c.output().expect("t1_image runs");
    let log = String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr);
    assert!(out.exists(), "t1_image built no image:\n{log}");
    (out, log)
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
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

fn command(yantra_run: &Path, defer: Option<&str>) -> Command {
    let mut c = Command::new(yantra_run);
    c.env("YANTRA_RAM", RAM)
        .env("YANTRA_STEPS", "4000000000")
        .env_remove("YANTRA_VIRTIO_DEFER")
        .env_remove("YANTRA_INPUT")
        .env_remove("YANTRA_SCANOUT");
    if let Some(d) = defer {
        c.env("YANTRA_VIRTIO_DEFER", d);
    }
    c
}

/// Run `yantra-run`, killing the child by its exact PID rather than hanging.
fn run(yantra_run: &Path, defer: Option<&str>, args: &[&str]) -> Ran {
    let mut c = command(yantra_run, defer)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("yantra-run spawns");
    wait(&mut c, 120, args);
    let o = c.wait_with_output().expect("output");
    Ran {
        code: o.status.code(),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn wait(c: &mut Child, secs: u64, what: &[&str]) {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        if c.try_wait().expect("try_wait").is_some() {
            return;
        }
        if Instant::now() > deadline {
            c.kill().expect("kill by exact pid");
            let _ = c.wait();
            panic!("yantra-run {what:?} HUNG for {secs} s (killed)");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn spawn(yantra_run: &Path, defer: Option<&str>, args: &[&str]) -> (Child, Receiver<String>) {
    let mut c = command(yantra_run, defer)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("yantra-run spawns");
    let (tx, rx) = std::sync::mpsc::channel();
    let err = c.stderr.take().expect("stderr");
    std::thread::spawn(move || {
        for l in BufReader::new(err).lines().map_while(Result::ok) {
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    (c, rx)
}

// ---- frames built here, from the RFCs, without the library ----------------------------

fn checksum(b: &[u8]) -> u16 {
    let mut s: u32 = 0;
    for ch in b.chunks(2) {
        s += (u32::from(ch[0]) << 8) | u32::from(*ch.get(1).unwrap_or(&0));
    }
    while s > 0xffff {
        s = (s & 0xffff) + (s >> 16);
    }
    !(s as u16)
}

const PINGER: [u8; 6] = [0x02, 0x53, 0x41, 0x4e, 0x53, 0x01];
const RESPONDER: [u8; 6] = [0x02, 0x53, 0x41, 0x4e, 0x53, 0x02];

fn arp_request() -> Vec<u8> {
    let mut f = vec![0xff; 6];
    f.extend_from_slice(&PINGER);
    f.extend_from_slice(&[0x08, 0x06, 0, 1, 8, 0, 6, 4, 0, 1]);
    f.extend_from_slice(&PINGER);
    f.extend_from_slice(&[10, 0, 0, 1]);
    f.extend_from_slice(&[0; 6]);
    f.extend_from_slice(&[10, 0, 0, 2]);
    f
}

/// The echo request the self-test builds: pinger to responder, identification 7, ICMP
/// identifier 0x5353, sequence 7, payload 0, 1, 2, ... 31.
fn echo_request() -> Vec<u8> {
    let mut icmp = vec![8, 0, 0, 0, 0x53, 0x53, 0, 7];
    icmp.extend((0..32u8).collect::<Vec<_>>());
    let c = checksum(&icmp);
    icmp[2..4].copy_from_slice(&c.to_be_bytes());
    let mut ip = vec![
        0x45, 0, 0, 60, 0, 7, 0x40, 0, 64, 1, 0, 0, 10, 0, 0, 1, 10, 0, 0, 2,
    ];
    let c = checksum(&ip);
    ip[10..12].copy_from_slice(&c.to_be_bytes());
    let mut f = RESPONDER.to_vec();
    f.extend_from_slice(&PINGER);
    f.extend_from_slice(&[8, 0]);
    f.extend(ip);
    f.extend(icmp);
    f
}

// ---- (a) the self-test, on both engines ------------------------------------------------

#[test]
#[ignore = "MEASUREMENT: builds an ELF with t1_image (~30 s) and runs yantra-run; run with --include-ignored after `cargo build --release -p sadhana --bin t1_image -p yantra --bin yantra-run`"]
fn the_self_test_agrees_on_both_engines_and_matches_frames_built_from_the_rfcs() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("self");
    let (elf, log) = build(&t1, &d);
    assert!(
        log.contains("gate:     AGREED"),
        "the differential gate must say AGREED:\n{log}"
    );
    assert!(
        log.contains("0 source(s) failed to compile"),
        "every source compiles:\n{log}"
    );
    assert!(
        log.contains("154 octet(s) printed"),
        "42 + 74 + 38 octets:\n{log}"
    );
    let r = run(&yr, None, &[elf.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "all 38 checks pass:\n{}", r.stderr);
    assert_eq!(r.stdout.len(), 154);
    assert_eq!(r.stdout[..42], arp_request()[..], "the ARP request frame");
    assert_eq!(
        r.stdout[42..116],
        echo_request()[..],
        "the echo request frame"
    );
    assert!(
        r.stdout[116..].iter().all(|&b| b == b'0'),
        "38 checks, each '0': {:?}",
        &r.stdout[116..]
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---- (b) and (c) two instances, logs, replay -------------------------------------------

/// The responder and the pinger over a peer socket. Answers the two runs.
fn pair(yr: &Path, elf: &Path, d: &Path, defer: Option<&str>) -> (Ran, Ran) {
    pair_of(yr, elf, elf, d, defer)
}

/// The responder runs `served`, the pinger `pinging` (role `p` of each).
fn pair_of(yr: &Path, served: &Path, pinging: &Path, d: &Path, defer: Option<&str>) -> (Ran, Ran) {
    pair_roles(yr, served, pinging, d, defer, ("s", "p"))
}

/// The same with the two roles' letters given: `(serving, calling)`.
fn pair_roles(
    yr: &Path,
    served: &Path,
    pinging: &Path,
    d: &Path,
    defer: Option<&str>,
    roles: (&str, &str),
) -> (Ran, Ran) {
    let (sl, pl) = (d.join("s.log"), d.join("p.log"));
    // NOT under `d`: a unix socket path must fit `sun_path` (104 octets on macOS, whose
    // `TMPDIR` alone is 49), and `d` carries the test's name. Measured on an Apple M1 host
    // 2026-10-09: "udp-echo-defer" made the path 106 octets, the responder was refused
    // by name and this loop waited out its 60 s. Short, and unique by pid, a per-process
    // count (tests run in parallel threads) and a clock (a pid is reused).
    static PAIRS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sock = format!(
        "--net-peer={}",
        std::env::temp_dir()
            .join(format!(
                "np-{}-{}-{}.sock",
                std::process::id(),
                PAIRS.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ))
            .display()
    );
    // A long TMPDIR can still overflow it: say so here, not as the responder's refusal.
    let path_len = sock.len() - "--net-peer=".len();
    assert!(
        path_len < 104,
        "the peer socket path is {path_len} octets; sun_path holds 103 on macOS (107 on \
         Linux). Set a shorter TMPDIR: {sock}"
    );
    let (e, pe) = (served.to_str().unwrap(), pinging.to_str().unwrap());
    let (cs, rs) = spawn(
        yr,
        defer,
        &[
            "--record-events",
            sl.to_str().unwrap(),
            "--net-mac=02:53:41:4e:53:02",
            &sock,
            e,
            roles.0,
        ],
    );
    let ready = Instant::now() + Duration::from_secs(60);
    loop {
        match rs.recv_timeout(Duration::from_millis(200)) {
            Ok(l) if l.contains("net: peer") && l.contains("listening") => break,
            // A refusal is the answer: fail on it now, not after the 60 s wait.
            Ok(l) if l.contains("refused") => panic!("the responder was refused: {l}"),
            _ if Instant::now() > ready => panic!("the responder never listened"),
            _ => {}
        }
    }
    let p = run(
        yr,
        defer,
        &[
            "--record-events",
            pl.to_str().unwrap(),
            "--net-mac=02:53:41:4e:53:01",
            &sock,
            pe,
            roles.1,
        ],
    );
    let mut cs = cs;
    wait(&mut cs, 60, &["responder"]);
    let o = cs.wait_with_output().expect("responder output");
    let err: String = rs.try_iter().collect::<Vec<_>>().join("\n");
    let s = Ran {
        code: o.status.code(),
        stdout: o.stdout,
        stderr: err,
    };
    (p, s)
}

fn steps(r: &Ran) -> String {
    r.line("steps:")
}

fn sent(r: &Ran) -> String {
    r.stderr
        .lines()
        .find(|l| l.starts_with("net: ") && l.contains("sent sha256"))
        .unwrap_or_else(|| panic!("no sent-sha line in {}", r.stderr))
        .to_string()
}

fn ping_and_answer(defer: Option<&str>, what: &str) {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch(what);
    let (elf, _) = build(&t1, &d);
    let (p, s) = pair(&yr, &elf, &d, defer);
    assert_eq!(p.code, Some(0), "pinger:\n{}", p.stderr);
    assert_eq!(p.text(), "P 0\n", "the echo reply came and checked out");
    assert_eq!(s.code, Some(0), "responder:\n{}", s.stderr);
    assert_eq!(
        s.text(),
        "S 2\n",
        "the responder answered the ARP request and the echo"
    );
    assert!(
        p.line("net: 2 frames").contains("transmitted, 2 received"),
        "{}",
        p.stderr
    );

    // each log replays ALONE, with no flag: the same output, instruction count, frames out
    for (role, live) in [("p", &p), ("s", &s)] {
        let log = d.join(format!("{role}.log"));
        let text = std::fs::read_to_string(&log).expect("the log");
        assert!(text.starts_with("# yantra-net v1 "), "{text}");
        assert_eq!(text.lines().filter(|l| l.starts_with("n=")).count(), 2);
        let args = [
            "--events",
            log.to_str().unwrap(),
            elf.to_str().unwrap(),
            role,
        ];
        let (r1, r2) = (run(&yr, defer, &args), run(&yr, defer, &args));
        assert_eq!(r1.code, Some(0), "{}", r1.stderr);
        assert_eq!(
            r1.stdout, live.stdout,
            "replay prints what the live run printed"
        );
        assert_eq!(r1.stdout, r2.stdout);
        assert_eq!(steps(&r1), steps(live), "the live run's instruction count");
        assert_eq!(steps(&r1), steps(&r2));
        assert_eq!(sent(&r1), sent(live), "the same frames out");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF and runs three yantra-run processes; --include-ignored"]
fn the_pair_pings_and_both_logs_replay_identically() {
    ping_and_answer(None, "pair");
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF and runs yantra-run under YANTRA_VIRTIO_DEFER; --include-ignored"]
fn the_pair_passes_under_deferred_completion_and_replays() {
    ping_and_answer(Some("1000"), "defer");
}

// ---- (d) refusals by name, from crafted replay logs -------------------------------------

/// Make an echo frame's ICMP checksum right again after a field was changed.
fn fix_icmp(f: &mut [u8]) {
    f[36] = 0;
    f[37] = 0;
    let c = checksum(&f[34..]);
    f[36..38].copy_from_slice(&c.to_be_bytes());
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A replay log: the recorded header, then one record per frame (`None` is `none`).
fn craft(header: &str, frames: &[Option<Vec<u8>>]) -> String {
    let mut t = format!("{header}\n");
    for (k, f) in frames.iter().enumerate() {
        match f {
            Some(f) => t.push_str(&format!("n={k}:0:{}\n", hex(f))),
            None => t.push_str(&format!("n={k}:0:none\n")),
        }
    }
    t
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF, records one pair, replays five crafted logs; --include-ignored"]
fn a_bad_checksum_a_malformed_packet_and_an_arp_timeout_are_refused_by_name() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("refuse");
    let (elf, _) = build(&t1, &d);
    let (p, _) = pair(&yr, &elf, &d, None);
    assert_eq!(p.code, Some(0), "{}", p.stderr);
    let recorded = std::fs::read_to_string(d.join("p.log")).unwrap();
    let header = recorded.lines().next().unwrap().to_string();
    let recs: Vec<Vec<u8>> = recorded
        .lines()
        .filter(|l| l.starts_with("n="))
        .map(|l| unhex(l.splitn(3, ':').nth(2).unwrap()))
        .collect();
    let (arp, echo) = (recs[0].clone(), recs[1].clone());
    assert_eq!(echo.len(), 74);

    let mut bad_icmp = echo.clone();
    bad_icmp[73] ^= 0xff; // the last payload octet
    let mut bad_ip = echo.clone();
    bad_ip[22] ^= 0x01; // the TTL
    let mut long_ip = echo.clone();
    long_ip[16..18].copy_from_slice(&200u16.to_be_bytes()); // a total length past the frame
    let mut wrong_sender = arp.clone();
    wrong_sender[28..32].copy_from_slice(&[10, 0, 0, 9]); // a reply from 10.0.0.9, not 10.0.0.2
    let mut wrong_id = echo.clone();
    wrong_id[38] ^= 0x01;
    fix_icmp(&mut wrong_id);
    let mut wrong_seq = echo.clone();
    wrong_seq[41] ^= 0x01;
    fix_icmp(&mut wrong_seq);
    let unanswered = |first: &Vec<u8>| -> Vec<Option<Vec<u8>>> {
        let mut v = vec![Some(arp.clone()), Some(first.clone())];
        v.extend(vec![None; 7]);
        v
    };
    type Case<'a> = (&'a str, Vec<Option<Vec<u8>>>, u64, &'a str);
    let cases: [Case; 9] = [
        ("good", vec![Some(arp.clone()), Some(echo.clone())], 0, ""),
        (
            "bad-icmp",
            vec![Some(arp.clone()), Some(bad_icmp)],
            CHECKSUM,
            "परीक्षायोगभेदः",
        ),
        (
            "bad-ip",
            vec![Some(arp.clone()), Some(bad_ip)],
            CHECKSUM,
            "परीक्षायोगभेदः",
        ),
        (
            "malformed-ip",
            vec![Some(arp.clone()), Some(long_ip)],
            MALFORMED,
            "पिण्डावैधरूपम्",
        ),
        (
            "arp-timeout",
            vec![None; 4],
            ARP_TIMEOUT,
            "स्थाननिर्देशकालातीतम्",
        ),
        (
            "arp-wrong-sender",
            vec![Some(wrong_sender), None, None, None],
            ARP_TIMEOUT,
            "स्थाननिर्देशकालातीतम्",
        ),
        (
            "echo-wrong-id",
            unanswered(&wrong_id),
            NO_REPLY,
            "प्रतिध्वन्यभावः",
        ),
        (
            "echo-wrong-seq",
            unanswered(&wrong_seq),
            NO_REPLY,
            "प्रतिध्वन्यभावः",
        ),
        (
            "ping-no-reply",
            std::iter::once(Some(arp.clone()))
                .chain(vec![None; 8])
                .collect(),
            NO_REPLY,
            "प्रतिध्वन्यभावः",
        ),
    ];
    for (name, frames, status, refusal) in cases {
        let log = d.join(format!("{name}.log"));
        std::fs::write(&log, craft(&header, &frames)).unwrap();
        let r = run(
            &yr,
            None,
            &[
                "--events",
                log.to_str().unwrap(),
                elf.to_str().unwrap(),
                "p",
            ],
        );
        let want = if status == 0 {
            "P 0\n".to_string()
        } else {
            format!("P {status}\n{refusal}\n")
        };
        assert_eq!(r.text(), want, "{name}:\n{}", r.stderr);
        assert_eq!(
            r.code,
            Some(i32::from(status != 0)),
            "{name}: yantra-run exits 1 on a non-zero finisher:\n{}",
            r.stderr
        );
        if name.starts_with("arp-") {
            assert!(
                r.stderr.contains("net: 4 frames transmitted"),
                "{name}: exactly four ARP requests, one per wait:\n{}",
                r.stderr
            );
        }
        if status != 0 {
            assert!(
                r.stderr.contains(&format!("value: {status},")),
                "{name}: the finisher value IS the refusal word {status}:\n{}",
                r.stderr
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- (f) the bounded poll is what passes the deferred device ----------------------------

/// The driver's two polls: the transmit completion's bound is the literal below (the receive
/// poll's bound is the variable `बन्धः`).
const TX_POLL: &str = "यावत् पठनानि न्यूनम् 10000000 आदि";
const TX_ONE_READ: &str = "यावत् पठनानि न्यूनम् 1 आदि";
const RX_POLL: &str = "यावत् पठनानि न्यूनम् बन्धः आदि";
const RX_ONE_READ: &str = "यावत् पठनानि न्यूनम् 1 आदि";

/// The Devanagari numeral spelling of an ASCII-digit string, as `spec/net` is written.
fn deva(s: &str) -> String {
    s.chars()
        .map(|c| match c.to_digit(10) {
            Some(d) => char::from_u32(0x966 + d).unwrap(),
            None => c,
        })
        .collect()
}

#[test]
#[ignore = "MEASUREMENT: builds three ELFs and replays six logs; --include-ignored"]
fn a_single_read_driver_passes_the_synchronous_device_and_fails_the_deferred_one() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("mutant");
    let (real, _) = build(&t1, &d);
    let (p, _) = pair(&yr, &real, &d, None);
    assert_eq!(p.code, Some(0), "{}", p.stderr);
    let recorded = std::fs::read_to_string(d.join("p.log")).unwrap();
    assert!(recorded.contains(" defer=0 "), "{recorded}");
    // the same two frames, delivered by a device that completes 1000 instructions late
    let sync_log = d.join("sync.log");
    let defer_log = d.join("defer.log");
    let body: String = recorded
        .lines()
        .filter(|l| !l.starts_with("n="))
        .collect::<Vec<_>>()
        .join("\n");
    let recs: String = recorded
        .lines()
        .filter(|l| l.starts_with("n="))
        .map(|l| format!("{l}\n"))
        .collect();
    let header = body.lines().next().unwrap();
    std::fs::write(&sync_log, format!("{header}\n{recs}")).unwrap();
    std::fs::write(
        &defer_log,
        format!("{}\n{recs}", header.replace(" defer=0 ", " defer=1000 ")),
    )
    .unwrap();

    let source = std::fs::read_to_string(root().join("spec/net/sanchara-pankti.t1")).unwrap();
    let (tx_poll, tx_one) = (deva(TX_POLL), deva(TX_ONE_READ));
    let (rx_poll, rx_one) = (deva(RX_POLL), deva(RX_ONE_READ));
    assert_eq!(
        source.matches(&tx_poll).count(),
        1,
        "the transmit poll's anchor must occur once"
    );
    assert_eq!(
        source.matches(&rx_poll).count(),
        1,
        "the receive poll's anchor must occur once"
    );
    let tx_mutant = d.join("tx-one-read.t1");
    let rx_mutant = d.join("rx-one-read.t1");
    std::fs::write(&tx_mutant, source.replace(&tx_poll, &tx_one)).unwrap();
    std::fs::write(&rx_mutant, source.replace(&rx_poll, &rx_one)).unwrap();
    let (tx_elf, _) = build_from(&t1, &d, "tx.elf", Some(&tx_mutant));
    let (rx_elf, _) = build_from(&t1, &d, "rx.elf", Some(&rx_mutant));

    let word = 0x35f_u64 << 16 | 0x3333;
    let absent = format!("P {word}\nसञ्चाराभावः\n");
    let play = |elf: &Path, log: &Path, defer: Option<&str>| {
        let r = run(
            &yr,
            defer,
            &[
                "--events",
                log.to_str().unwrap(),
                elf.to_str().unwrap(),
                "p",
            ],
        );
        r.text()
    };
    // [real, tx one-read, rx one-read] x [synchronous, deferred]
    for (name, elf, deferred_passes) in [
        ("real", &real, true),
        ("tx-one-read", &tx_elf, false),
        ("rx-one-read", &rx_elf, false),
    ] {
        assert_eq!(
            play(elf, &sync_log, None),
            "P 0\n",
            "{name} on the synchronous device"
        );
        let got = play(elf, &defer_log, Some("1000"));
        if deferred_passes {
            assert_eq!(got, "P 0\n", "{name} on the deferred device");
        } else {
            assert_eq!(
                got, absent,
                "{name} on the deferred device must be refused by name"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- (g) the driver's own region, outstanding sends, the 16-bit wrap -----------------------

/// A replay log of no records: the recorded header, optionally with another deferral and caps.
fn header_log(d: &Path, name: &str, defer: &str, caps: bool) -> PathBuf {
    let recorded = std::fs::read_to_string(d.join("p.log")).expect("a recorded p.log");
    let mut h = recorded.lines().next().unwrap().to_string();
    h = h.replace(" defer=0 ", &format!(" defer={defer} "));
    if caps {
        h = h
            .replace("max-tx=65536", "max-tx=70000")
            .replace("max-rx=65536", "max-rx=70000");
    }
    let p = d.join(name);
    std::fs::write(&p, format!("{h}\n")).unwrap();
    p
}

#[test]
#[ignore = "MEASUREMENT: builds four ELFs and runs two pairs; --include-ignored"]
fn a_gpu_draw_then_a_net_receive_does_not_read_a_stale_gpu_entry() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("gpu");
    let (served, _) = build(&t1, &d);
    let (gpu, log) = build_gpu(&t1, &d, "gpu.elf", None);
    assert!(log.contains("gate:     AGREED"), "{log}");
    let (p, s) = pair_of(&yr, &served, &gpu, &d, None);
    assert_eq!(p.code, Some(0), "draw then ping:\n{}", p.stderr);
    assert!(p.text().starts_with("G 0\n"), "the GPU drew: {}", p.text());
    assert_eq!(s.code, Some(0), "{}", s.stderr);

    // the OLD layout: the GPU's region and rings that are not zeroed. The ping must fail,
    // or the test above proves nothing.
    let source = std::fs::read_to_string(root().join("spec/net/sanchara-pankti.t1")).unwrap();
    let mut old = source.clone();
    for (new, was) in [
        (2852126720_u64, 2818572288_u64),
        (2852126848, 2818572416),
        (2852126852, 2818572420),
        (2852134912, 2818580480),
        (2852134916, 2818580484),
        (2852134920, 2818580488),
        (2852134924, 2818580492),
        (2852135040, 2818580608),
        (2852139008, 2818584576),
        (2852139012, 2818584580),
        (2852130816, 2818576384),
        (2852130820, 2818576388),
        (2852143104, 2818588672),
        (2852143114, 2818588682),
        (2852159488, 2818605056),
    ] {
        old = old.replace(&deva(&new.to_string()), &deva(&was.to_string()));
    }
    for zero in ["यावत् क्रमः न्यूनम् 17 आदि", "यावत् क्रमः न्यूनम् 5 आदि"]
    {
        let z = deva(zero);
        assert_eq!(old.matches(&z).count(), 1, "{zero}");
        old = old.replace(&z, &deva("यावत् क्रमः न्यूनम् 0 आदि"));
    }
    let shared = d.join("shared-region.t1");
    std::fs::write(&shared, old).unwrap();
    let (gpu_old, _) = build_gpu(&t1, &d, "gpu-old.elf", Some(&shared));
    let e = d.join("old");
    std::fs::create_dir_all(&e).unwrap();
    let (p, _) = pair_of(&yr, &served, &gpu_old, &e, None);
    assert_ne!(
        p.code,
        Some(0),
        "a driver on the GPU's rings, not zeroed, must misread them:\n{}",
        p.stderr
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore = "MEASUREMENT: builds two ELFs and replays four logs; --include-ignored"]
fn a_send_after_a_refused_one_is_refused_until_the_first_is_used() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("outstanding");
    let (real, _) = build(&t1, &d);
    let (p, _) = pair(&yr, &real, &d, None);
    assert_eq!(p.code, Some(0), "{}", p.stderr);
    let source = std::fs::read_to_string(root().join("spec/net/sanchara-pankti.t1")).unwrap();
    let (tx_poll, tx_one) = (deva(TX_POLL), deva(TX_ONE_READ));
    assert_eq!(source.matches(&tx_poll).count(), 1);
    let mutant = d.join("tx-one-read.t1");
    std::fs::write(&mutant, source.replace(&tx_poll, &tx_one)).unwrap();
    let (tx_elf, _) = build_from(&t1, &d, "tx.elf", Some(&mutant));
    let sync = header_log(&d, "sync.log", "0", false);
    let slow = header_log(&d, "slow.log", "1000", false);
    let play = |elf: &Path, log: &Path, defer: Option<&str>| {
        run(
            &yr,
            defer,
            &[
                "--events",
                log.to_str().unwrap(),
                elf.to_str().unwrap(),
                "x",
            ],
        )
        .text()
    };
    let ok = "0\n0\n0\n0\n0\n";
    assert_eq!(play(&real, &sync, None), ok);
    assert_eq!(
        play(&real, &slow, Some("1000")),
        ok,
        "the real driver waits for the device"
    );
    assert_eq!(play(&tx_elf, &sync, None), ok);
    // one read on a device 1000 instructions late: send 1 is refused (cause 6); send 2, straight
    // after, is refused for the outstanding one (cause 8) and writes nothing; after the wait
    // the first completion has arrived, send 3 is accepted, and its own single read then
    // misses its own late completion (refused, cause 6 again)
    let w = 0x35f_u64 << 16 | 0x3333;
    assert_eq!(
        play(&tx_elf, &slow, Some("1000")),
        format!("{w}\n6\n{w}\n8\n{w}\n")
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF and runs two ~10 s replays; --include-ignored"]
fn the_sixteen_bit_ring_indices_wrap_across_65540_sends_and_65540_receives() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("wrap");
    let (elf, _) = build(&t1, &d);
    let (p, _) = pair(&yr, &elf, &d, None);
    assert_eq!(p.code, Some(0), "{}", p.stderr);
    // sends: 65540 frames out, the available and used indices pass 65535 to 0
    let log = header_log(&d, "w.log", "0", true);
    let r = run(
        &yr,
        None,
        &[
            "--events",
            log.to_str().unwrap(),
            elf.to_str().unwrap(),
            "w",
        ],
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.text(), "W 65540\n", "every send passed across the wrap");
    // receives: 65540 frames in, the used index passes 65535 to 0 and the ring slot (index mod 8) goes on
    let mut frame = vec![
        0x02, 0x53, 0x41, 0x4e, 0x53, 0x01, 0x02, 0x53, 0x41, 0x4e, 0x53, 0x01, 0x88, 0xb5,
    ];
    frame.resize(60, 0);
    let mut text = std::fs::read_to_string(&log).unwrap();
    for k in 0..65540 {
        text.push_str(&format!("n={k}:0:{}\n", hex(&frame)));
    }
    let rlog = d.join("r.log");
    std::fs::write(&rlog, text).unwrap();
    let r = run(
        &yr,
        None,
        &[
            "--events",
            rlog.to_str().unwrap(),
            elf.to_str().unwrap(),
            "r",
        ],
    );
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(r.text(), "R 65540\n", "every frame arrived across the wrap");
    let _ = std::fs::remove_dir_all(&d);
}

// ---- (e) no device without a flag -------------------------------------------------------

#[test]
#[ignore = "MEASUREMENT: builds an ELF; --include-ignored"]
fn the_driver_is_refused_by_name_when_no_net_flag_gave_it_a_device() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("nodev");
    let (elf, _) = build(&t1, &d);
    let r = run(&yr, None, &[elf.to_str().unwrap(), "p"]);
    assert_ne!(r.code, Some(0), "{}", r.stderr);
    assert!(
        r.stderr.contains("सञ्चाराभावः (0x35f)"),
        "slot 2 touched with no --net-* flag is refused by its ruled name:\n{}",
        r.stderr
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ======================================================================================
// MILESTONE 3 — UDP (`निःसन्धिपिण्डः`) and a DNS client (`नामनिर्देशः`), ADR-0048
//
// `spec/net/net-udp-demo.t1` is the program: a self-test of 94 checks, a UDP echo pair
// (roles `e` and `u`) and a DNS pair (roles `n` and `d`). Every frame the tests compare is
// built HERE from RFC 768 / 791 / 1035, without the library.
// ======================================================================================

const NOT_FOUND: u64 = word(0x365);
/// `उत्तरखण्डितम्`: a truncated DNS response (ruled 2026-10-08).
const TRUNCATED: u64 = word(0x371);
const TIMEOUT: u64 = word(0x369);
/// The checks in the self-test, and the octets it prints before them (a 13-octet datagram, a
/// 30-octet query, a 46-octet response).
const UDP_CHECKS: usize = 94;
const UDP_PRINTED: usize = 13 + 30 + 46;
/// The DNS client's transaction ID (19001 = 0x4a39) and its source port, 49152 + (ID & 4095).
const DNS_ID: u16 = 19001;
const DNS_SPORT: u16 = 49152 + (DNS_ID & 4095);
const ANSWER: [u8; 4] = [192, 0, 2, 7];

const CLIENT_IP: [u8; 4] = [10, 0, 0, 1];
const SERVER_IP: [u8; 4] = [10, 0, 0, 2];

/// The library, with `swaps` applied to its sources — `(file, from, to, which)` replaces the
/// `which`th (0-based) occurrence of `from` in `file` — and the UDP/DNS demo program.
fn build_udp(
    t1_image: &Path,
    dir: &Path,
    name: &str,
    swaps: &[(&str, String, String, usize)],
) -> (PathBuf, String) {
    let (out, log) = try_build_udp(t1_image, dir, name, swaps);
    assert!(out.exists(), "t1_image built no image:\n{log}");
    (out, log)
}

/// As `build_udp`, but a refused build is an answer, not a failure: the path may not exist.
fn try_build_udp(
    t1_image: &Path,
    dir: &Path,
    name: &str,
    swaps: &[(&str, String, String, usize)],
) -> (PathBuf, String) {
    let root = root();
    let net = root.join("spec/net");
    let mut files: Vec<PathBuf> = Vec::new();
    let mut names: Vec<&str> = FILES[..LIB].to_vec();
    names.push("net-udp-demo.t1");
    for f in names {
        let mut src = std::fs::read_to_string(net.join(f)).expect("library source");
        let mut changed = false;
        for (file, from, to, which) in swaps {
            if *file == f {
                let at = src
                    .match_indices(from.as_str())
                    .nth(*which)
                    .map(|(i, _)| i)
                    .unwrap_or_else(|| panic!("{f}: no occurrence {which} of {from:?}"));
                src.replace_range(at..at + from.len(), to);
                changed = true;
            }
        }
        if changed {
            let p = dir.join(format!("{name}-{f}"));
            std::fs::write(&p, src).unwrap();
            files.push(p);
        } else {
            files.push(net.join(f));
        }
    }
    let out = dir.join(name);
    let mut c = Command::new(t1_image);
    c.arg("--spec-root")
        .arg(root.join("spec"))
        .arg("--compiler")
        .arg(root.join("crates/sadhana-t1/src"));
    for f in &files {
        c.arg("--load").arg(f);
    }
    c.args(["--entry", "सन्देशपरीक्षा", "मुख्यम्", "-o"])
        .arg(&out)
        .args(&files);
    let o = c.output().expect("t1_image runs");
    let log = String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr);
    (out, log)
}

// ---- frames built here, from the RFCs ---------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Sum {
    /// the checksum RFC 768 computes (a computed 0 goes out as 0xffff)
    Right,
    /// the field 0: "no checksum"
    None,
    /// the right checksum with the last payload octet then changed
    Wrong,
}

/// A UDP datagram with its checksum over the IPv4 pseudo-header.
fn udp_datagram(
    src: [u8; 4],
    dst: [u8; 4],
    sport: u16,
    dport: u16,
    payload: &[u8],
    sum: Sum,
) -> Vec<u8> {
    let len = (8 + payload.len()) as u16;
    let mut d = Vec::new();
    d.extend_from_slice(&sport.to_be_bytes());
    d.extend_from_slice(&dport.to_be_bytes());
    d.extend_from_slice(&len.to_be_bytes());
    d.extend_from_slice(&[0, 0]);
    d.extend_from_slice(payload);
    let mut pseudo = Vec::new();
    pseudo.extend_from_slice(&src);
    pseudo.extend_from_slice(&dst);
    pseudo.extend_from_slice(&[0, 17]);
    pseudo.extend_from_slice(&len.to_be_bytes());
    pseudo.extend_from_slice(&d);
    let mut c = checksum(&pseudo);
    if c == 0 {
        c = 0xffff;
    }
    match sum {
        Sum::None => {}
        Sum::Right => d[6..8].copy_from_slice(&c.to_be_bytes()),
        Sum::Wrong => {
            d[6..8].copy_from_slice(&c.to_be_bytes());
            let n = d.len();
            d[n - 1] ^= 0xff;
        }
    }
    d
}

/// An Ethernet + IPv4 (identification `id`, DF, TTL 64, UDP) frame around a datagram.
fn udp_frame(
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    src: [u8; 4],
    dst: [u8; 4],
    id: u16,
    udp: &[u8],
) -> Vec<u8> {
    let mut ip = vec![0x45, 0];
    ip.extend_from_slice(&((20 + udp.len()) as u16).to_be_bytes());
    ip.extend_from_slice(&id.to_be_bytes());
    ip.extend_from_slice(&[0x40, 0, 64, 17, 0, 0]);
    ip.extend_from_slice(&src);
    ip.extend_from_slice(&dst);
    let c = checksum(&ip);
    ip[10..12].copy_from_slice(&c.to_be_bytes());
    let mut f = dst_mac.to_vec();
    f.extend_from_slice(&src_mac);
    f.extend_from_slice(&[8, 0]);
    f.extend(ip);
    f.extend_from_slice(udp);
    f
}

/// The wire form of a name: labels, then the root.
fn qname(name: &str) -> Vec<u8> {
    let mut v = Vec::new();
    for l in name.split('.') {
        v.push(l.len() as u8);
        v.extend_from_slice(l.as_bytes());
    }
    v.push(0);
    v
}

/// The A-record query the client sends (RFC 1035 §4.1): recursion desired, one question.
fn dns_query(id: u16, name: &str) -> Vec<u8> {
    let mut m = Vec::new();
    m.extend_from_slice(&id.to_be_bytes());
    m.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
    m.extend(qname(name));
    m.extend_from_slice(&[0, 1, 0, 1]);
    m
}

/// A response: `flags`, the question for `name`, `an` as the count and `answers` verbatim.
fn dns_response(id: u16, flags: u16, name: &str, an: u16, answers: &[u8]) -> Vec<u8> {
    let mut m = Vec::new();
    m.extend_from_slice(&id.to_be_bytes());
    m.extend_from_slice(&flags.to_be_bytes());
    m.extend_from_slice(&[0, 1]);
    m.extend_from_slice(&an.to_be_bytes());
    m.extend_from_slice(&[0; 4]);
    m.extend(qname(name));
    m.extend_from_slice(&[0, 1, 0, 1]);
    m.extend_from_slice(answers);
    m
}

/// An A record whose owner name is `owner` (wire octets), TTL 60.
fn a_record(owner: &[u8], addr: [u8; 4]) -> Vec<u8> {
    let mut r = owner.to_vec();
    r.extend_from_slice(&[0, 1, 0, 1, 0, 0, 0, 60, 0, 4]);
    r.extend_from_slice(&addr);
    r
}

const NAME: &str = "www.sas.test";
const PTR_Q: [u8; 2] = [0xc0, 0x0c];

fn good_response() -> Vec<u8> {
    dns_response(DNS_ID, 0x8180, NAME, 1, &a_record(&PTR_Q, ANSWER))
}

// ---- (h) the self-test -------------------------------------------------------------------

#[test]
#[ignore = "MEASUREMENT: builds an ELF with t1_image (~100 s) and runs yantra-run; --include-ignored"]
fn the_udp_and_dns_self_test_agrees_on_both_engines_and_matches_messages_built_from_the_rfcs() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("udp-self");
    let (elf, log) = build_udp(&t1, &d, "udp.elf", &[]);
    assert!(
        log.contains("gate:     AGREED"),
        "the differential gate must say AGREED:\n{log}"
    );
    assert!(
        log.contains("0 source(s) failed to compile"),
        "every source compiles:\n{log}"
    );
    let printed = UDP_PRINTED + UDP_CHECKS;
    assert!(
        log.contains(&format!("{printed} octet(s) printed")),
        "13 + 30 + 46 + {UDP_CHECKS} octets:\n{log}"
    );
    let r = run(&yr, None, &[elf.to_str().unwrap()]);
    assert_eq!(
        r.code,
        Some(0),
        "all {UDP_CHECKS} checks pass:\n{}",
        r.stderr
    );
    assert_eq!(r.stdout.len(), printed);
    assert_eq!(
        r.stdout[..13],
        udp_datagram(CLIENT_IP, SERVER_IP, 7001, 7, b"Hello", Sum::Right)[..],
        "the datagram, checksum over the pseudo-header included"
    );
    assert_eq!(
        r.stdout[13..43],
        dns_query(0x5353, NAME)[..],
        "the A-record query"
    );
    assert_eq!(
        r.stdout[43..UDP_PRINTED],
        dns_response(0x5353, 0x8180, NAME, 1, &a_record(&PTR_Q, ANSWER))[..],
        "the response the library's own server builds"
    );
    assert!(
        r.stdout[UDP_PRINTED..].iter().all(|&b| b == b'0'),
        "{UDP_CHECKS} checks, each '0': {:?}",
        &r.stdout[UDP_PRINTED..]
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---- (i) the pairs -----------------------------------------------------------------------

/// Run `srole` against `crole` of one ELF over a peer socket, and replay both logs ALONE with
/// no flag. Answers `(client, server)`.
fn roles_and_replay(
    defer: Option<&str>,
    what: &str,
    roles: (&str, &str),
    outputs: (&str, &str),
    check: impl Fn(&Path, &Ran, &Ran),
) {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch(what);
    let (elf, _) = build_udp(&t1, &d, "udp.elf", &[]);
    let (c, s) = pair_roles(&yr, &elf, &elf, &d, defer, roles);
    assert_eq!(c.code, Some(0), "client:\n{}", c.stderr);
    assert_eq!(c.text(), outputs.1, "client output");
    assert_eq!(s.code, Some(0), "server:\n{}", s.stderr);
    assert_eq!(s.text(), outputs.0, "server output");
    assert!(
        c.line("net: 2 frames").contains("transmitted, 2 received"),
        "{}",
        c.stderr
    );
    check(&d, &c, &s);
    for (role, live) in [(roles.1, &c), (roles.0, &s)] {
        let log = d.join(format!("{}.log", if role == roles.1 { "p" } else { "s" }));
        let text = std::fs::read_to_string(&log).expect("the log");
        assert!(text.starts_with("# yantra-net v1 "), "{text}");
        assert_eq!(text.lines().filter(|l| l.starts_with("n=")).count(), 2);
        let args = [
            "--events",
            log.to_str().unwrap(),
            elf.to_str().unwrap(),
            role,
        ];
        let (r1, r2) = (run(&yr, defer, &args), run(&yr, defer, &args));
        assert_eq!(r1.code, Some(0), "{}", r1.stderr);
        assert_eq!(
            r1.stdout, live.stdout,
            "replay prints what the live run printed"
        );
        assert_eq!(r1.stdout, r2.stdout);
        assert_eq!(steps(&r1), steps(live), "the live run's instruction count");
        assert_eq!(steps(&r1), steps(&r2));
        assert_eq!(sent(&r1), sent(live), "the same frames out");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// A frame as the driver sends it: padded with zeros to 60 octets.
fn padded(mut f: Vec<u8>) -> Vec<u8> {
    if f.len() < 60 {
        f.resize(60, 0);
    }
    f
}

/// The frames a log recorded, in order.
fn log_frames(path: &Path) -> Vec<Vec<u8>> {
    std::fs::read_to_string(path)
        .expect("log")
        .lines()
        .filter(|l| l.starts_with("n="))
        .map(|l| unhex(l.splitn(3, ':').nth(2).unwrap()))
        .collect()
}

fn udp_echo(defer: Option<&str>, what: &str) {
    roles_and_replay(defer, what, ("e", "u"), ("E 0\n", "U 0\n"), |d, _c, _s| {
        let payload = b"Hello UDP";
        // what the echo server RECEIVED (its log): the ARP request, then the client's datagram
        let served = log_frames(&d.join("s.log"));
        assert_eq!(
            served[1],
            padded(udp_frame(
                RESPONDER,
                PINGER,
                CLIENT_IP,
                SERVER_IP,
                1,
                &udp_datagram(CLIENT_IP, SERVER_IP, 7001, 7, payload, Sum::Right)
            )),
            "the datagram the client sent"
        );
        // what the client RECEIVED: the ARP reply, then the echo
        let got = log_frames(&d.join("p.log"));
        assert_eq!(
            got[1],
            padded(udp_frame(
                PINGER,
                RESPONDER,
                SERVER_IP,
                CLIENT_IP,
                1,
                &udp_datagram(SERVER_IP, CLIENT_IP, 7, 7001, payload, Sum::Right)
            )),
            "the datagram the server sent back"
        );
    });
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF and runs three yantra-run processes per log; --include-ignored"]
fn a_udp_echo_between_two_runs_succeeds_and_both_logs_replay_identically() {
    udp_echo(None, "udp-echo");
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF and runs yantra-run under YANTRA_VIRTIO_DEFER; --include-ignored"]
fn a_udp_echo_passes_under_deferred_completion_and_replays() {
    udp_echo(Some("1000"), "udp-echo-defer");
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF and runs three yantra-run processes per log; --include-ignored"]
fn a_dns_query_is_answered_by_a_responder_program_and_both_logs_replay_identically() {
    roles_and_replay(
        None,
        "dns-pair",
        ("n", "d"),
        ("N 0\n", "D 0\nA 3221225991\n"),
        |d, _c, _s| {
            let served = log_frames(&d.join("s.log"));
            assert_eq!(
                served[1],
                udp_frame(
                    RESPONDER,
                    PINGER,
                    CLIENT_IP,
                    SERVER_IP,
                    1,
                    &udp_datagram(
                        CLIENT_IP,
                        SERVER_IP,
                        DNS_SPORT,
                        53,
                        &dns_query(DNS_ID, NAME),
                        Sum::Right
                    )
                ),
                "the query the client sent, byte for byte"
            );
            let got = log_frames(&d.join("p.log"));
            assert_eq!(
                got[1],
                udp_frame(
                    PINGER,
                    RESPONDER,
                    SERVER_IP,
                    CLIENT_IP,
                    1,
                    &udp_datagram(
                        SERVER_IP,
                        CLIENT_IP,
                        53,
                        DNS_SPORT,
                        &good_response(),
                        Sum::Right
                    )
                ),
                "the response the responder program built, byte for byte"
            );
        },
    );
}

// ---- (j) DNS refusals from crafted logs --------------------------------------------------

/// A response frame from the server to the client's source port.
fn response_frame(msg: &[u8], sum: Sum) -> Vec<u8> {
    udp_frame(
        PINGER,
        RESPONDER,
        SERVER_IP,
        CLIENT_IP,
        1,
        &udp_datagram(SERVER_IP, CLIENT_IP, 53, DNS_SPORT, msg, sum),
    )
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF, records one pair, replays crafted logs; --include-ignored"]
fn a_malformed_response_a_pointer_loop_a_wrong_id_and_nxdomain_are_refused_by_name() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("dns-refuse");
    let (elf, _) = build_udp(&t1, &d, "udp.elf", &[]);
    let (c, _) = pair_roles(&yr, &elf, &elf, &d, None, ("n", "d"));
    assert_eq!(c.code, Some(0), "{}", c.stderr);
    let recorded = std::fs::read_to_string(d.join("p.log")).unwrap();
    let header = recorded.lines().next().unwrap().to_string();
    let recs = log_frames(&d.join("p.log"));
    let arp = recs[0].clone();
    // the recorded response equals the one built here
    assert_eq!(recs[1], response_frame(&good_response(), Sum::Right));

    let good = good_response();
    let mut cut = good.clone();
    cut.truncate(good.len() - 2); // the A record's data is cut off
    let self_ptr = dns_response(DNS_ID, 0x8180, NAME, 1, &a_record(&[0xc0, 30], ANSWER));
    // a label run into a pointer back to the label's own start: 5 'a's, then (192, 30)
    let loop_back = dns_response(
        DNS_ID,
        0x8180,
        NAME,
        1,
        &a_record(&[5, b'a', b'a', b'a', b'a', b'a', 0xc0, 30], ANSWER),
    );
    let forward = dns_response(DNS_ID, 0x8180, NAME, 1, &a_record(&[0xc0, 32], ANSWER));
    let wrong_id = dns_response(DNS_ID + 1, 0x8180, NAME, 1, &a_record(&PTR_Q, ANSWER));
    let other_question = dns_response(DNS_ID, 0x8180, "www.sas.tesx", 1, &a_record(&PTR_Q, ANSWER));
    let nxdomain = dns_response(DNS_ID, 0x8183, NAME, 0, &[]);
    let no_answer = dns_response(DNS_ID, 0x8180, NAME, 0, &[]);
    let servfail = dns_response(DNS_ID, 0x8182, NAME, 0, &[]);
    let query_not_response = {
        let mut m = good.clone();
        m[2] = 0x01;
        m
    };
    // an IPv4 header whose checksum does not add up
    let mut bad_ip_header = response_frame(&good, Sum::Right);
    bad_ip_header[22] ^= 1; // the TTL
    // a response CUT short at the UDP level whose missing octets follow it in the IPv4 packet:
    // the datagram says 2 octets less than the packet holds
    let trailing = {
        let mut u = udp_datagram(SERVER_IP, CLIENT_IP, 53, DNS_SPORT, &cut, Sum::Right);
        u.extend_from_slice(&[2, 7]);
        udp_frame(PINGER, RESPONDER, SERVER_IP, CLIENT_IP, 1, &u)
    };
    let opcode1 = dns_response(DNS_ID, 0x8980, NAME, 1, &a_record(&PTR_Q, ANSWER));
    let truncated = dns_response(DNS_ID, 0x8380, NAME, 1, &a_record(&PTR_Q, ANSWER));
    let ok_out = "D 0\nA 3221225991\n".to_string();
    let refused = |w: u64, n: &str| format!("D {w}\n{n}\n");
    let none7 = |first: Vec<u8>| -> Vec<Option<Vec<u8>>> {
        let mut v = vec![Some(arp.clone()), Some(first)];
        v.extend(vec![None; 7]);
        v
    };
    let one = |f: Vec<u8>| -> Vec<Option<Vec<u8>>> { vec![Some(arp.clone()), Some(f)] };
    let spoofed = udp_frame(
        PINGER,
        RESPONDER,
        [10, 0, 0, 9],
        CLIENT_IP,
        1,
        &udp_datagram([10, 0, 0, 9], CLIENT_IP, 53, DNS_SPORT, &good, Sum::Right),
    );
    let other_port = udp_frame(
        PINGER,
        RESPONDER,
        SERVER_IP,
        CLIENT_IP,
        1,
        &udp_datagram(SERVER_IP, CLIENT_IP, 53, DNS_SPORT + 1, &good, Sum::Right),
    );
    let from_other_port = udp_frame(
        PINGER,
        RESPONDER,
        SERVER_IP,
        CLIENT_IP,
        1,
        &udp_datagram(SERVER_IP, CLIENT_IP, 5353, DNS_SPORT, &good, Sum::Right),
    );
    // an IPv4 packet whose UDP length field says more than the packet holds
    let mut long_udp = response_frame(&good, Sum::None);
    long_udp[38..40].copy_from_slice(&200u16.to_be_bytes());
    let timed_out = refused(TIMEOUT, "सञ्चारकालातीतम्");
    type DnsCase<'a> = (&'a str, Vec<Option<Vec<u8>>>, String, u64);
    let cases: Vec<DnsCase> = vec![
        (
            "good",
            one(response_frame(&good, Sum::Right)),
            ok_out.clone(),
            0,
        ),
        (
            "good-no-udp-checksum",
            one(response_frame(&good, Sum::None)),
            ok_out.clone(),
            0,
        ),
        (
            "malformed-short-is-ignored",
            none7(response_frame(&good[..11], Sum::Right)),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "malformed-cut-record-is-ignored",
            none7(response_frame(&cut, Sum::Right)),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "pointer-to-itself-is-ignored",
            none7(response_frame(&self_ptr, Sum::Right)),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "pointer-loop-is-ignored",
            none7(response_frame(&loop_back, Sum::Right)),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "pointer-forward-is-ignored",
            none7(response_frame(&forward, Sum::Right)),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "udp-length-past-packet-is-ignored",
            none7(long_udp),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "udp-bad-checksum-is-ignored",
            none7(response_frame(&good, Sum::Wrong)),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "nxdomain",
            one(response_frame(&nxdomain, Sum::Right)),
            refused(NOT_FOUND, "नामानुपलब्धिः"),
            NOT_FOUND,
        ),
        (
            "servfail",
            one(response_frame(&servfail, Sum::Right)),
            refused(NOT_FOUND, "नामानुपलब्धिः"),
            NOT_FOUND,
        ),
        (
            "no-answer",
            one(response_frame(&no_answer, Sum::Right)),
            refused(NOT_FOUND, "नामानुपलब्धिः"),
            NOT_FOUND,
        ),
        (
            "wrong-id",
            none7(response_frame(&wrong_id, Sum::Right)),
            refused(TIMEOUT, "सञ्चारकालातीतम्"),
            TIMEOUT,
        ),
        (
            "wrong-question",
            none7(response_frame(&other_question, Sum::Right)),
            refused(TIMEOUT, "सञ्चारकालातीतम्"),
            TIMEOUT,
        ),
        (
            "not-a-response",
            none7(response_frame(&query_not_response, Sum::Right)),
            refused(TIMEOUT, "सञ्चारकालातीतम्"),
            TIMEOUT,
        ),
        (
            "spoofed-source-address",
            none7(spoofed),
            refused(TIMEOUT, "सञ्चारकालातीतम्"),
            TIMEOUT,
        ),
        (
            "other-destination-port",
            none7(other_port),
            refused(TIMEOUT, "सञ्चारकालातीतम्"),
            TIMEOUT,
        ),
        (
            "not-from-port-53",
            none7(from_other_port),
            refused(TIMEOUT, "सञ्चारकालातीतम्"),
            TIMEOUT,
        ),
        (
            "bad-udp-checksum-then-the-real-answer",
            vec![
                Some(arp.clone()),
                Some(response_frame(&good, Sum::Wrong)),
                Some(response_frame(&good, Sum::Right)),
            ],
            ok_out.clone(),
            0,
        ),
        (
            "bad-ipv4-header-then-the-real-answer",
            vec![
                Some(arp.clone()),
                Some(bad_ip_header),
                Some(response_frame(&good, Sum::Right)),
            ],
            ok_out.clone(),
            0,
        ),
        (
            "malformed-body-then-the-real-answer",
            vec![
                Some(arp.clone()),
                Some(response_frame(&loop_back, Sum::Right)),
                Some(response_frame(&good, Sum::Right)),
            ],
            ok_out.clone(),
            0,
        ),
        (
            "udp-length-under-the-ip-payload-the-extra-octets-are-not-returned",
            none7(trailing),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "opcode-not-zero",
            none7(response_frame(&opcode1, Sum::Right)),
            timed_out.clone(),
            TIMEOUT,
        ),
        (
            "truncated",
            one(response_frame(&truncated, Sum::Right)),
            format!("D {TRUNCATED}\n{}\n", "उत्तरखण्डितम्"),
            TRUNCATED,
        ),
        (
            "nothing-comes",
            std::iter::once(Some(arp.clone()))
                .chain(vec![None; 8])
                .collect(),
            refused(TIMEOUT, "सञ्चारकालातीतम्"),
            TIMEOUT,
        ),
    ];
    for (name, frames, want, status) in cases {
        let log = d.join(format!("{name}.log"));
        std::fs::write(&log, craft(&header, &frames)).unwrap();
        let r = run(
            &yr,
            None,
            &[
                "--events",
                log.to_str().unwrap(),
                elf.to_str().unwrap(),
                "d",
            ],
        );
        assert_eq!(r.text(), want, "{name}:\n{}", r.stderr);
        assert_eq!(
            r.code,
            Some(i32::from(status != 0)),
            "{name}: yantra-run exits 1 on a non-zero finisher:\n{}",
            r.stderr
        );
        if status != 0 {
            assert!(
                r.stderr.contains(&format!("value: {status},")),
                "{name}: the finisher value IS the refusal word {status}:\n{}",
                r.stderr
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- (l) receive-path refusals from crafted logs: roles `a` and `b` ------------------------

/// A datagram frame from `src`:`sport` to this host (10.0.0.2) port `dport`.
fn rx_frame(src: [u8; 4], sport: u16, dport: u16, payload: &[u8]) -> Vec<u8> {
    udp_frame(
        PINGER,
        RESPONDER,
        src,
        SERVER_IP,
        1,
        &udp_datagram(src, SERVER_IP, sport, dport, payload, Sum::Right),
    )
}

#[test]
#[ignore = "MEASUREMENT: builds an ELF, records one pair, replays crafted logs; --include-ignored"]
fn the_receive_path_refuses_by_pair_by_port_and_by_buffer() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("rx-refuse");
    let (elf, _) = build_udp(&t1, &d, "udp.elf", &[]);
    let (c, _) = pair_roles(&yr, &elf, &elf, &d, None, ("n", "d"));
    assert_eq!(c.code, Some(0), "{}", c.stderr);
    let recorded = std::fs::read_to_string(d.join("p.log")).unwrap();
    let header = recorded.lines().next().unwrap().to_string();
    let arp = log_frames(&d.join("p.log"))[0].clone();
    let good = good_response();
    let timeout_b = format!("B {TIMEOUT}\nसञ्चारकालातीतम्\n");
    let too_big_b = format!("B {}\nपिण्डातिदीर्घनिषेधः\n", word(0x360));
    let none3 = |f: Vec<u8>| -> Vec<Option<Vec<u8>>> { vec![Some(f), None, None, None] };
    let from_listed_other_host = udp_frame(
        PINGER,
        RESPONDER,
        [10, 0, 0, 9],
        CLIENT_IP,
        1,
        &udp_datagram([10, 0, 0, 9], CLIENT_IP, 53, DNS_SPORT, &good, Sum::Right),
    );
    let from_listed_other_port = udp_frame(
        PINGER,
        RESPONDER,
        SERVER_IP,
        CLIENT_IP,
        1,
        &udp_datagram(SERVER_IP, CLIENT_IP, 5353, DNS_SPORT, &good, Sum::Right),
    );
    let nothing7 = |first: Vec<u8>| -> Vec<Option<Vec<u8>>> {
        let mut v = vec![Some(arp.clone()), Some(first)];
        v.extend(vec![None; 7]);
        v
    };
    let timeout_a = format!("a {TIMEOUT}\nसञ्चारकालातीतम्\n");
    // (name, role, frames, output, exit status)
    type RoleCase<'a> = (&'a str, &'a str, Vec<Option<Vec<u8>>>, String, i32);
    let cases: Vec<RoleCase> = vec![
        // role a lists (10.0.0.9, 53) and (the server, 5353) as well as (the server, 53)
        (
            "dns-answer-from-a-listed-other-host",
            "a",
            nothing7(from_listed_other_host),
            timeout_a.clone(),
            1,
        ),
        (
            "dns-answer-from-a-listed-other-port",
            "a",
            nothing7(from_listed_other_port),
            timeout_a.clone(),
            1,
        ),
        (
            "dns-answer-from-the-server",
            "a",
            vec![Some(arp.clone()), Some(response_frame(&good, Sum::Right))],
            "a 0\nA 3221225991\n".to_string(),
            0,
        ),
        // role b: a 20-octet buffer; lists (10.0.0.1, any port) and (10.0.0.3, port 99)
        (
            "oversize-payload",
            "b",
            vec![Some(rx_frame(CLIENT_IP, 7001, 7, &[7u8; 100]))],
            too_big_b.clone(),
            1,
        ),
        (
            "payload-one-over-the-buffer",
            "b",
            vec![Some(rx_frame(CLIENT_IP, 7001, 7, &[7u8; 21]))],
            too_big_b.clone(),
            1,
        ),
        (
            "payload-fills-the-buffer",
            "b",
            vec![Some(rx_frame(CLIENT_IP, 7001, 7, &[7u8; 20]))],
            "B 0\nL 20\n".to_string(),
            0,
        ),
        (
            "unlisted-host",
            "b",
            none3(rx_frame([10, 0, 0, 9], 7001, 7, &[7u8; 5])),
            timeout_b.clone(),
            1,
        ),
        (
            "listed-host-any-port",
            "b",
            vec![Some(rx_frame(CLIENT_IP, 5, 7, &[7u8; 5]))],
            "B 0\nL 5\n".to_string(),
            0,
        ),
        (
            "listed-pair",
            "b",
            vec![Some(rx_frame([10, 0, 0, 3], 99, 7, &[7u8; 6]))],
            "B 0\nL 6\n".to_string(),
            0,
        ),
        (
            "listed-host-unlisted-port",
            "b",
            none3(rx_frame([10, 0, 0, 3], 98, 7, &[7u8; 5])),
            timeout_b.clone(),
            1,
        ),
        (
            "other-destination-port",
            "b",
            none3(rx_frame(CLIENT_IP, 7001, 8, &[7u8; 5])),
            timeout_b.clone(),
            1,
        ),
    ];
    for (name, role, frames, want, code) in cases {
        let log = d.join(format!("{name}.log"));
        std::fs::write(&log, craft(&header, &frames)).unwrap();
        let r = run(
            &yr,
            None,
            &[
                "--events",
                log.to_str().unwrap(),
                elf.to_str().unwrap(),
                role,
            ],
        );
        assert_eq!(r.text(), want, "{name}:\n{}", r.stderr);
        assert_eq!(r.code, Some(code), "{name}:\n{}", r.stderr);
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---- (k) a mutant of each refusal is caught by the self-test ----------------------------

/// `(label, file, from, to, which, how)` — one edit to the library each; `how` is whether the
/// self-test must FAIL (a nonzero exit or a halt) or its printed messages must DIFFER from the
/// ones built here.
struct Mutant {
    label: &'static str,
    file: &'static str,
    from: String,
    to: String,
    which: usize,
}

fn mutant(label: &'static str, file: &'static str, from: &str, to: &str, which: usize) -> Mutant {
    Mutant {
        label,
        file,
        from: deva(from),
        to: deva(to),
        which,
    }
}

fn w(code: u64) -> String {
    word(code).to_string()
}

#[test]
#[ignore = "MEASUREMENT: builds ~24 ELFs (4 at a time) and runs the self-test of each; --include-ignored"]
fn a_mutant_of_each_udp_and_dns_refusal_fails_the_self_test() {
    let Some((t1, yr)) = tools() else { return };
    let d = scratch("udp-mutants");
    let u = "nihsandhi-pinda.t1";
    let n = "nama-nirdesha.t1";
    let s = "sanchara.t1";
    let ret = |code: u64| format!("प्रत्यागमनम् {} ।", w(code));
    let zero = "प्रत्यागमनम् 0 ।";
    // NOT mutated, because the mutant is EQUIVALENT in its answer (8f's mutation runs found these
    // and no others still alive; each repeats a check made elsewhere): `खण्डदीर्घता अधिकम् 7` in
    // the receive (and the `> 6` variant: परीक्षणम् refuses the same datagrams), `द्वितीयम् अधिकम्
    // अवसानम्` (the position is already below the end), the pointer checks `समम् सीमा` and `समम्
    // चलस्थलम्` (the other pointer checks and the jump bound refuse the same loops), the 400-step
    // and 130-step bounds (the 255-octet name limit refuses first), and `उत्तरसङ्ख्या समम् 0` (no
    // record is the same answer as no A record). The header-length checks of the response parse
    // and the datagram-length check of the UDP parse are NOT equivalent: a short message with
    // another ID, and a 4-octet datagram in an array of 4, are in the self-test for them.
    let ms: Vec<Mutant> = vec![
        // UDP
        mutant("udp-checksum", u, &ret(0x362), zero, 0),
        mutant(
            "udp-checksum-none-accepted",
            u,
            "यदि चिह्नम् समम् 0 आदि",
            "यदि चिह्नम् समम् 70000 आदि",
            0,
        ),
        mutant(
            "udp-length-under-header",
            u,
            "यदि कुलम् न्यूनम् 8 आदि",
            "यदि कुलम् न्यूनम् 0 आदि",
            0,
        ),
        mutant(
            "udp-length-past-available",
            u,
            "यदि कुलम् अधिकम् उपलब्धम् आदि",
            "यदि कुलम् अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "udp-parse-array-guard",
            u,
            "यदि धारिता न्यूनम् अवसानम् आदि",
            "यदि धारिता न्यूनम् 0 आदि",
            1,
        ),
        mutant("udp-build-array-guard", u, &ret(0x360), zero, 0),
        mutant("udp-allow-list-default-deny", u, &ret(0x367), zero, 1),
        mutant("udp-allow-list-full", u, &ret(0x367), zero, 0),
        mutant(
            "udp-allow-list-port-0-is-any",
            u,
            "यदि धृतद्वारम् समम् 0 आदि",
            "यदि धृतद्वारम् समम् 70000 आदि",
            0,
        ),
        // the family module's members
        mutant(
            "send-allow-list",
            s,
            "यदि अनुज्ञा असमम् 0 आदि",
            "यदि अनुज्ञा असमम् अनुज्ञा आदि",
            0,
        ),
        mutant(
            "send-too-big",
            s,
            "यदि भारदैर्घ्यम् अधिकम् 1472 आदि",
            "यदि भारदैर्घ्यम् अधिकम् 70000 आदि",
            0,
        ),
        // DNS
        mutant(
            "dns-rcode",
            n,
            "यदि फलसङ्केतः असमम् 0 आदि",
            "यदि फलसङ्केतः असमम् फलसङ्केतः आदि",
            0,
        ),
        mutant("dns-no-answer-count", n, &ret(0x365), zero, 1),
        mutant("dns-no-a-record", n, &ret(0x365), zero, 2),
        mutant(
            "dns-id",
            n,
            "यदि पहचानः असमम् क्रमाङ्कः आदि",
            "यदि पहचानः असमम् पहचानः आदि",
            0,
        ),
        mutant(
            "dns-qr-bit",
            n,
            "यदि उत्तरबिट् असमम् 1 आदि",
            "यदि उत्तरबिट् अधिकम् 1 आदि",
            0,
        ),
        mutant(
            "dns-question",
            n,
            "यदि प्राप्ताक्षरम् असमम् पृष्टाक्षरम् आदि",
            "यदि प्राप्ताक्षरम् असमम् प्राप्ताक्षरम् आदि",
            0,
        ),
        mutant(
            "dns-question-case",
            n,
            "यदि प्राप्ताक्षरम् न्यूनम् 91 आदि",
            "यदि प्राप्ताक्षरम् न्यूनम् 0 आदि",
            0,
        ),
        mutant(
            "dns-record-fields-cut",
            n,
            "यदि क्षेत्रान्तः अधिकम् अवसानम् आदि",
            "यदि क्षेत्रान्तः अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-truncated",
            n,
            "यदि छिन्नबिट् समम् 1 आदि",
            "यदि छिन्नबिट् समम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-opcode",
            n,
            "यदि आदेशबिट् असमम् 0 आदि",
            "यदि आदेशबिट् असमम् आदेशबिट् आदि",
            0,
        ),
        mutant(
            "dns-id-width",
            n,
            "यदि क्रमाङ्कः अधिकम् 65535 आदि",
            "यदि क्रमाङ्कः अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-record-data-cut",
            n,
            "यदि भारान्तः अधिकम् अवसानम् आदि",
            "यदि भारान्तः अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-a-record-length",
            n,
            "यदि भारदीर्घता असमम् 4 आदि",
            "यदि भारदीर्घता असमम् भारदीर्घता आदि",
            0,
        ),
        mutant(
            "dns-name-255",
            n,
            "यदि नामलम्बः अधिकम् 254 आदि",
            "यदि नामलम्बः अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-server-name-255",
            n,
            "यदि चलस्थलम् अधिकम् 266 आदि",
            "यदि चलस्थलम् अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-query-min-17",
            n,
            "यदि पृच्छादैर्घ्यम् न्यूनम् 17 आदि",
            "यदि पृच्छादैर्घ्यम् न्यूनम् 0 आदि",
            0,
        ),
        mutant(
            "dns-pointer-forward",
            n,
            "यदि लक्ष्यः अधिकम् चलस्थलम् आदि",
            "यदि लक्ष्यः अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-pointer-jump-bound",
            n,
            "यदि उडुः अधिकम् 16 आदि",
            "यदि उडुः अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-reserved-label-type",
            n,
            "यदि शीर्षद्वयम् असमम् 0 आदि",
            "यदि शीर्षद्वयम् असमम् शीर्षद्वयम् आदि",
            0,
        ),
        mutant(
            "dns-name-array-guard",
            n,
            "यदि धारणम् न्यूनम् नामदैर्घ्यम् आदि",
            "यदि धारणम् न्यूनम् 0 आदि",
            0,
        ),
        mutant(
            "dns-query-length",
            n,
            "यदि पिण्डधारणम् न्यूनम् आवश्यकम् आदि",
            "यदि पिण्डधारणम् न्यूनम् 0 आदि",
            0,
        ),
        mutant(
            "dns-label-63",
            n,
            "यदि अक्षरगणना अधिकम् 62 आदि",
            "यदि अक्षरगणना अधिकम् 70000 आदि",
            0,
        ),
        mutant(
            "dns-server-bad-query",
            n,
            "यदि अनुरोधबिट् असमम् 0 आदि",
            "यदि अनुरोधबिट् असमम् अनुरोधबिट् आदि",
            0,
        ),
        mutant("dns-server-too-small", n, &ret(0x360), zero, 0),
    ];
    // a mutant of the LAYOUT (not of a refusal): the self-test's own checks cannot see it,
    // the messages printed against the ones built here can
    let layout = vec![mutant(
        "udp-pseudo-header-protocol",
        u,
        "योगफलम् भवति योगफलम् योगः 17 ।",
        "योगफलम् भवति योगफलम् योगः 16 ।",
        0,
    )];

    let all: Vec<(Mutant, bool)> = ms
        .into_iter()
        .map(|m| (m, true))
        .chain(layout.into_iter().map(|m| (m, false)))
        .collect();
    let mut results: Vec<(String, bool, String)> = Vec::new();
    for chunk in all.chunks(4) {
        let handles: Vec<_> = chunk
            .iter()
            .map(|(m, refusal)| {
                let (t1, yr, d) = (t1.clone(), yr.clone(), d.clone());
                let swap = (m.file, m.from.clone(), m.to.clone(), m.which);
                let (label, refusal) = (m.label, *refusal);
                std::thread::spawn(move || {
                    let name = format!("{label}.elf");
                    let (elf, log) = try_build_udp(&t1, &d, &name, &[swap]);
                    if !elf.exists() {
                        // a mutant that reaches the device in a run that has none: the build's own
                        // differential gate (W-381) refuses it, which is a catch; a build that
                        // failed for any other reason is a bug in the mutant
                        let gate = log.contains("STOPPED AT the differential gate");
                        return (
                            label.to_string(),
                            gate && refusal,
                            "refused by the build's gate".to_string(),
                        );
                    }
                    let r = run(&yr, None, &[elf.to_str().unwrap()]);
                    let expect = {
                        let mut v =
                            udp_datagram(CLIENT_IP, SERVER_IP, 7001, 7, b"Hello", Sum::Right);
                        v.extend(dns_query(0x5353, NAME));
                        v.extend(dns_response(
                            0x5353,
                            0x8180,
                            NAME,
                            1,
                            &a_record(&PTR_Q, ANSWER),
                        ));
                        v
                    };
                    let fails = r.code != Some(0);
                    let differs =
                        r.stdout.len() < UDP_PRINTED || r.stdout[..UDP_PRINTED] != expect[..];
                    let caught = if refusal { fails } else { differs };
                    (
                        label.to_string(),
                        caught,
                        format!(
                            "exit {:?}; build: {}",
                            r.code,
                            log.lines()
                                .filter(|l| l.starts_with("gate:"))
                                .collect::<String>()
                        ),
                    )
                })
            })
            .collect();
        for h in handles {
            results.push(h.join().expect("mutant thread"));
        }
    }
    let missed: Vec<_> = results.iter().filter(|(_, c, _)| !*c).collect();
    for (l, c, why) in &results {
        eprintln!(
            "mutant {l}: {} ({why})",
            if *c { "caught" } else { "MISSED" }
        );
    }
    assert!(
        missed.is_empty(),
        "mutants the self-test did not catch: {missed:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
