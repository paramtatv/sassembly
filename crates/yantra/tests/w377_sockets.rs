//! **`W-377`: SOCKETS AS A HOST-SOCKET DEVICE, IN THREE ENGINES, LIVE AND REPLAYED.**
//!
//! `docs/adr/0040-addendum-w377-sockets.md`, accepted by the owner on 2026-10-06 ("W-377
//! Six Socket Rulings: Approved as specified", all six §8 questions as recommended). A
//! REGISTER DEVICE at `SOCK = 0x1000_0110` served through `W-350`'s two existing calls:
//! NEXT (store 0) pops the queue into the RX latch, RX (load) answers an octet, EMPTY,
//! END (sticky) or UNREAD as data, TX (store) sends an octet. The queue changes only at a
//! wait. Live, `yantra-run --record-events L --listen 127.0.0.1:0` serves one client and
//! logs `s=<hex>` / `s=end` per wait; `--events L` replays it without a network.
//!
//! The addendum's §7 cases, on `w373_wait.rs`'s harness (the `.t1` chain's image and the
//! Rust twin's, beside the interpreter's option (b)):
//!
//! - (a) a LIVE localhost echo through the real binary;
//! - (b) its log replayed twice: identical stdout, `steps:` and `socket:` lines, to each
//!   other and to the live run;
//! - (c) UNREAD, EMPTY, an octet and END are four distinct answers, and END is sticky;
//! - (d) every refusal of §6, one test each;
//! - (e) six mutants, each red;
//! - (f) the three engines byte-identical on (c) and on an echo-log replay, and the
//!   interpreter's copies of the device's constants and words equal to yantra's;
//! - (g) the two source ratchets, each with its mutation control (the fixpoint's
//!   `socket:` check is in `w372_fixpoint_ratchet.rs`, beside its siblings).
//!
//! EVERY NUMERAL IS GENERATED from a Rust constant ([`dec`]), never typed; the two device
//! calls' names are copied from `ashtaka.t1`.

use sadhana::kosha;
use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{self, Interpreter, Octets, Value};
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use yantra::input::{EVENT_TAG, find_event_slot, parse_event_log, parse_thread_log};
use yantra::socket::{
    self, CLOSE, EMPTY, END, NEXT, RECORD_CAP, RX, SockRecord, SockReplayed, Socket, TX, UNREAD,
    parse_socket_log, replay_socket,
};
use yantra::threads::{THREAD_ID_TAG, THREADS_TAG};
use yantra::{FINISHER, Halt, Machine, Output, SOCK, WAIT};

const FUEL: u64 = 80_000_000_000;
const STEPS: u64 = 200_000_000;
/// Copied from `w373_wait.rs`, never constructed.
const MODULE: &str = "घटनापरीक्षण";
/// The two `W-350` device calls, copied from `ashtaka.t1:298` and `:306`.
const LOAD_CALL: &str = "उपकरणचतुरष्टकाहारः";
const STORE_CALL: &str = "उपकरणचतुरष्टकनिधानम्";

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// A decimal numeral for a `u64`, in the corpus's digits.
fn dec(n: u64) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    n.to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

/// A qualified call into `अष्टक`.
fn q(member: &str) -> String {
    format!("{}\u{971}{member}", nirvahana::WAIT_BUILTIN_MODULE)
}

fn wait_call() -> String {
    q(nirvahana::WAIT_BUILTIN_MEMBER)
}

/// Eight little-endian octets per word through the output channel (`w373_wait.rs`).
const PRINTER: &str = "वृत्तिः शब्दमुद्रणम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः अवगणना ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ८ आदि
        चरः सरणम् ॱॱ न६४ भवति क्रमः गुणनम् ८ ।
        चरः सृतम् ॱॱ न६४ भवति मूल्यम् दक्षिणसृ सरणम् ।
        चरः अष्टकम् ॱॱ न६४ भवति सृतम् युक् २५५ ।
        अवगणना भवति अष्टकॱमुद्रणम् अष्टकम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";

/// The module head: the event interface (the slot gets each record's octet count) and
/// the printer.
fn head() -> String {
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति {tag} ।
सार्वजनिक चरः घटनामूल्यक ॱॱ न६४ भवति ० ।

{PRINTER}",
        tag = dec(EVENT_TAG)
    )
}

/// `store NEXT 0`, as one statement.
fn next_stmt() -> String {
    format!("अवगणना भवति {} {} ० ।", q(STORE_CALL), dec(NEXT))
}

/// `v = load RX`, as one statement.
fn rx_stmt() -> String {
    format!("अष्टकम् भवति {} {} ।", q(LOAD_CALL), dec(RX))
}

/// THE ECHO PROGRAM (addendum §5): drain with NEXT/RX; an octet is sent back through TX
/// and counted; EMPTY waits; END stops. Then it prints the count and answers ०.
fn echo_program() -> String {
    format!(
        "{head}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः अष्टकम् ॱॱ न६४ भवति ० ।
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः समम् ० आदि
        {next}
        {rx}
        यदि अष्टकम् न्यूनम् {octets} आदि
            अवगणना भवति {store} {tx} अष्टकम् ।
            योगफलम् भवति योगफलम् योगः १ ।
        इति
        यदि अष्टकम् समम् {empty} आदि
            अवगणना भवति {wait} ० ।
        इति
        यदि अष्टकम् समम् {end} आदि
            क्रमः भवति १ ।
        इति
    इति
    अवगणना भवति शब्दमुद्रणम् योगफलम् ।
    प्रत्यागमनम् ० ।
इति
",
        head = head(),
        next = next_stmt(),
        rx = rx_stmt(),
        octets = dec(0x100),
        store = q(STORE_CALL),
        tx = dec(TX),
        empty = dec(u64::from(EMPTY)),
        end = dec(u64::from(END)),
        wait = wait_call(),
    )
}

/// THE STATUS PROGRAM, case (c): RX before any NEXT; NEXT and RX before any wait; a wait;
/// two NEXT/RX; a wait; two NEXT/RX — printing every answer and the slot after each wait.
fn status_program() -> String {
    let print = "अवगणना भवति शब्दमुद्रणम् अष्टकम् ।";
    let slot = "अवगणना भवति शब्दमुद्रणम् घटनामूल्यक ।";
    let wait = format!("अवगणना भवति {} ० ।", wait_call());
    let read = format!("{}\n    {}\n    {print}", next_stmt(), rx_stmt());
    format!(
        "{head}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः अष्टकम् ॱॱ न६४ भवति ० ।
    {rx}
    {print}
    {read}
    {wait}
    {slot}
    {read}
    {read}
    {wait}
    {slot}
    {read}
    {read}
    प्रत्यागमनम् ० ।
इति
",
        head = head(),
        rx = rx_stmt(),
    )
}

/// The octets the status program prints for `s=41`, `s=end`.
fn status_want() -> Vec<u8> {
    [
        u64::from(UNREAD),
        u64::from(EMPTY),
        1,
        0x41,
        u64::from(EMPTY),
        0,
        u64::from(END),
        u64::from(END),
    ]
    .iter()
    .flat_map(|w| w.to_le_bytes())
    .collect()
}

fn status_log() -> Vec<SockRecord> {
    parse_socket_log("s=41\ns=end\n").expect("the (c) log parses")
}

/// An echo log of four octet records — one a full 4096 — then `s=end`.
fn echo_log() -> Vec<SockRecord> {
    let big: Vec<u8> = (0..RECORD_CAP).map(|i| (i * 7 % 251) as u8).collect();
    vec![
        SockRecord::Octets(b"hello\n".to_vec()),
        SockRecord::Octets(big),
        SockRecord::Octets(vec![0, 0xff, 0x0a]),
        SockRecord::Octets(b"!".to_vec()),
        SockRecord::End,
    ]
}

fn all_octets(log: &[SockRecord]) -> Vec<u8> {
    log.iter()
        .flat_map(|r| match r {
            SockRecord::Octets(o) => o.clone(),
            SockRecord::End => Vec::new(),
        })
        .collect()
}

// ── what a run came to ───────────────────────────────────────────────────────

/// ONE VOCABULARY FOR THREE ENGINES. The UART octets and the TX octets are carried with
/// every class, so a refusal is compared on what the program had already done.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    /// Finished, every record consumed.
    Finished {
        status: u64,
        out: Vec<u8>,
        sent: Vec<u8>,
        records: usize,
        received: u64,
    },
    /// Wait `index` had no record; never resumed.
    Short {
        index: usize,
        out: Vec<u8>,
        sent: Vec<u8>,
    },
    /// Wait `index` came after END.
    AfterEnd {
        index: usize,
        out: Vec<u8>,
        sent: Vec<u8>,
    },
    /// Finished with records left over.
    Long { delivered: usize, len: usize },
    /// A device refusal, by its name.
    Device(String),
    /// Anything else, said in full.
    Other(String),
}

/// UART and TX, kept apart.
#[derive(Default)]
struct TestSink {
    out: Vec<u8>,
    sent: Vec<u8>,
}

impl Output for TestSink {
    fn putc(&mut self, byte: u8) {
        self.out.push(byte);
    }
    fn sent(&mut self, byte: u8) {
        self.sent.push(byte);
    }
}

/// THE NATIVE SIDE: [`replay_socket`] as `yantra-run --events` judges it. Answers the
/// outcome and the retired count.
fn native_with(
    image: &[u8],
    log: &[SockRecord],
    out: &mut (impl Output + Taken),
) -> (Outcome, u64) {
    let mut m = Machine::load_elf(image, yantra::ram_for(image)).expect("the image loads");
    let tag = match find_event_slot(&m.mem) {
        Ok(t) => t,
        Err(e) => return (Outcome::Other(e), 0),
    };
    let r = replay_socket(&mut m, tag, log, STEPS, out);
    let (o, s) = out.taken();
    let outcome = match r {
        SockReplayed::Halted {
            halt:
                Halt::Finisher {
                    status: Some(status),
                    ..
                },
            delivered,
        } => {
            if delivered < log.len() {
                Outcome::Long {
                    delivered,
                    len: log.len(),
                }
            } else {
                Outcome::Finished {
                    status,
                    out: o,
                    sent: s,
                    records: delivered,
                    received: m.socket.as_ref().map_or(0, |s| s.received),
                }
            }
        }
        SockReplayed::Halted {
            halt: Halt::Device { why, .. },
            ..
        } => Outcome::Device(why.to_string()),
        SockReplayed::Halted { halt, delivered } => {
            Outcome::Other(format!("halted {halt:?} after {delivered} records"))
        }
        SockReplayed::Short { index, .. } => Outcome::Short {
            index,
            out: o,
            sent: s,
        },
        SockReplayed::OverCap { why, .. } => Outcome::Other(why),
        SockReplayed::AfterEnd { index, .. } => Outcome::AfterEnd {
            index,
            out: o,
            sent: s,
        },
    };
    (outcome, m.time)
}

/// What a sink collected: `(UART, TX)`.
trait Taken {
    fn taken(&self) -> (Vec<u8>, Vec<u8>);
}

impl Taken for TestSink {
    fn taken(&self) -> (Vec<u8>, Vec<u8>) {
        (self.out.clone(), self.sent.clone())
    }
}

fn native(image: &[u8], log: &[SockRecord]) -> (Outcome, u64) {
    native_with(image, log, &mut TestSink::default())
}

/// yantra's records as the interpreter's twin type.
fn twin(log: &[SockRecord]) -> Vec<nirvahana::SockRecord> {
    log.iter()
        .map(|r| match r {
            SockRecord::Octets(o) => nirvahana::SockRecord::Octets(o.clone()),
            SockRecord::End => nirvahana::SockRecord::End,
        })
        .collect()
}

/// The `wait index N` a refusal names.
fn wait_index(r: &str) -> usize {
    r.split("wait index ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(usize::MAX)
}

/// THE INTERPRETED SIDE (option (b)): the chain loaded beside the probe; `deliver` handed
/// to [`Interpreter::set_socket`] (the log itself, except in the k+1 mutant), or no socket
/// log at all.
fn interpreted_with(src: &str, len: usize, deliver: Option<&[SockRecord]>) -> Outcome {
    let file = format!("{MODULE}.t1");
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push((file.as_str(), src));
    let mut it = Interpreter::load(&srcs, &spec_root())
        .unwrap_or_else(|e| panic!("the probe must load: {}", e.reason));
    if let Some(d) = deliver
        && let Err(e) = it.set_socket(&twin(d))
    {
        return Outcome::Other(e.reason);
    }
    let r = it.call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL);
    let out = it.sink().to_vec();
    let sent = it.socket_sent().to_vec();
    let (records, received) = it.socket_delivered();
    match r {
        Ok(v) => {
            if records < len {
                Outcome::Long {
                    delivered: records,
                    len,
                }
            } else {
                Outcome::Finished {
                    status: v.as_int().map_or(u64::MAX, |s| s as u64),
                    out,
                    sent,
                    records,
                    received,
                }
            }
        }
        Err(e) => {
            let r = e.reason;
            if r.contains("SHORTER") {
                Outcome::Short {
                    index: wait_index(&r),
                    out,
                    sent,
                }
            } else if r.starts_with("a WAIT after END") {
                Outcome::AfterEnd {
                    index: wait_index(&r),
                    out,
                    sent,
                }
            } else if let Some(why) = r.lines().next().filter(|w| socket::REFUSALS.contains(w)) {
                // The interpreter appends where it was (`\n  in <routine>`); the name is
                // the first line, and it is the native device's.
                Outcome::Device(why.to_string())
            } else {
                Outcome::Other(r)
            }
        }
    }
}

fn interpreted(src: &str, log: &[SockRecord]) -> Outcome {
    interpreted_with(src, log.len(), Some(log))
}

/// THE `.t1` CHAIN'S IMAGE (`w373_wait.rs`'s `t1_image_with`).
fn t1_image(src: &str) -> Vec<u8> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(MODULE.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(src.as_bytes())]),
                arena(vec![octets(MODULE.as_bytes())]),
                Value::Int(1),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(
        !image.is_empty(),
        "the probe built no image: refusal {:?}, link {:?}",
        sadhana::t1::chain::refusal_site(&it),
        sadhana::t1::chain::link_refusals(&it)
    );
    image
}

/// THE RUST TWIN (`w373_wait.rs`'s `rust_twin_image`).
fn rust_twin_image(src: &str) -> Vec<u8> {
    use sadhana::encode::Target;
    use sadhana::nidana::Language;
    use sadhana::t1::chain::Front;
    use sadhana::t1::riscv64;
    use sadhana::{assemble_object, vastu};
    const LOAD: u64 = 0x8000_0000;

    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let module = front
        .module(MODULE, Some("मुख्यम्"))
        .expect("the module builds");
    let module_text = riscv64::emit_module(&module).expect("the Rust emitter emits");
    let startup_text = riscv64::emit_startup_object_with_records(
        Some(&format!("{MODULE}मुख्यम्")),
        riscv64::module_allocates(&module),
    );
    let to_object = |text: &str, name: Option<&str>| -> vastu::Object {
        let bytes = assemble_object(text, name, Target::Uncompressed, false, Language::English)
            .unwrap_or_else(|ds| {
                panic!("{name:?} does not assemble: {ds:?}\n--- text ---\n{text}")
            });
        vastu::read(&bytes).unwrap_or_else(|| panic!("{name:?} does not read back"))
    };
    let startup = to_object(&startup_text, Some("यन्त्रारम्भ"));
    let module_obj = to_object(&module_text, Some(MODULE));
    let linked = sadhana::samyojana::link_at(&[startup, module_obj], LOAD)
        .unwrap_or_else(|es| panic!("the Rust path does not link: {es:?}"));
    sadhana::kosha::write_debuggable_at(&linked.text, &linked.data, &[], linked.bss, &[], LOAD)
}

#[derive(Clone, Copy, Debug)]
enum Prog {
    Echo,
    Status,
}

fn source(p: Prog) -> String {
    match p {
        Prog::Echo => echo_program(),
        Prog::Status => status_program(),
    }
}

/// Each image built ONCE per test binary.
fn image(p: Prog, twin: bool) -> &'static [u8] {
    static T1: [OnceLock<Vec<u8>>; 2] = [OnceLock::new(), OnceLock::new()];
    static TWIN: [OnceLock<Vec<u8>>; 2] = [OnceLock::new(), OnceLock::new()];
    let k = p as usize;
    if twin {
        TWIN[k].get_or_init(|| rust_twin_image(&source(p)))
    } else {
        T1[k].get_or_init(|| t1_image(&source(p)))
    }
}

/// The three engines over one program and one log.
fn three(p: Prog, log: &[SockRecord]) -> [(&'static str, Outcome); 3] {
    [
        ("interpreter", interpreted(&source(p), log)),
        (".t1 image", native(image(p, false), log).0),
        ("rust twin", native(image(p, true), log).0),
    ]
}

fn assert_all(what: &str, got: &[(&str, Outcome)], want: &Outcome) {
    for (engine, o) in got {
        println!("{what}, {engine}: {}", brief(o));
    }
    for (engine, o) in got {
        assert_eq!(o, want, "{what}: the {engine} differs");
    }
}

/// An outcome with its octet runs cut short, for the log.
fn brief(o: &Outcome) -> String {
    let s = format!("{o:?}");
    if s.len() > 400 {
        format!("{}… ({} chars)", &s[..400], s.len())
    } else {
        s
    }
}

// ── the real binary, live and replayed ──────────────────────────────────────

fn scratch(what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w377-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// What one `yantra-run` did.
#[derive(Debug, Clone)]
struct Ran {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
}

impl Ran {
    fn line(&self, prefix: &str) -> Option<String> {
        self.stderr
            .lines()
            .find(|l| l.starts_with(prefix))
            .map(str::to_string)
    }
    fn socket_line(&self) -> Option<String> {
        self.stderr
            .lines()
            .find(|l| l.starts_with("socket: ") && l.contains(" octets received in "))
            .map(str::to_string)
    }
}

fn os(s: &str) -> &std::ffi::OsStr {
    std::ffi::OsStr::new(s)
}

fn yantra_run(args: &[&std::ffi::OsStr]) -> Ran {
    let o = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .args(args)
        .output()
        .expect("yantra-run runs");
    Ran {
        code: o.status.code(),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// A LIVE RUN: `yantra-run --record-events <log> --listen 127.0.0.1:0 <elf>`, the port
/// read from stderr, then `client` given the connected stream. Answers the run, the log
/// and what `client` answered.
fn live<T: Send + 'static>(
    dir: &Path,
    elf: &[u8],
    client: impl FnOnce(std::net::TcpStream) -> T + Send + 'static,
) -> (Ran, String, T) {
    let prog = dir.join("live.elf");
    std::fs::write(&prog, elf).expect("write the image");
    let log = dir.join("live.log");
    let mut child = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .arg("--record-events")
        .arg(&log)
        .arg("--listen")
        .arg("127.0.0.1:0")
        .arg(&prog)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("yantra-run starts");
    let stderr = child.stderr.take().expect("piped");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stderr).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let mut lines = Vec::new();
    let port = loop {
        let line = rx
            .recv_timeout(core::time::Duration::from_secs(120))
            .unwrap_or_else(|e| {
                panic!(
                    "no listening line ({e}); stderr so far:\n{}",
                    lines.join("\n")
                )
            });
        let port = line
            .strip_prefix("socket: listening on 127.0.0.1:")
            .map(|p| p.parse::<u16>().expect("a port"));
        lines.push(line);
        if let Some(p) = port {
            break p;
        }
    };
    assert_ne!(port, 0, "port 0 is resolved and said");
    let stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let answer = client(stream);
    let out = child.wait_with_output().expect("yantra-run ends");
    reader.join().expect("the stderr reader");
    lines.extend(rx.try_iter());
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    (
        Ran {
            code: out.status.code(),
            stdout: out.stdout,
            stderr: lines.join("\n"),
        },
        text,
        answer,
    )
}

/// What the live echo client sends: `hello\n` and 10 KiB (more than one record).
fn live_payload() -> Vec<u8> {
    let mut p = b"hello\n".to_vec();
    p.extend((0..10 * 1024).map(|i| (i * 13 % 256) as u8));
    p
}

/// THE LIVE ECHO, run once per test binary: the run, its log and the octets echoed.
fn live_echo() -> &'static (Ran, String, Vec<u8>) {
    static LIVE: OnceLock<(Ran, String, Vec<u8>)> = OnceLock::new();
    LIVE.get_or_init(|| {
        let dir = scratch("live");
        let (ran, log, echoed) = live(&dir, image(Prog::Echo, false), |mut s| {
            let mut w = s.try_clone().expect("clone");
            let sender = std::thread::spawn(move || {
                w.write_all(&live_payload()).expect("send");
                w.shutdown(std::net::Shutdown::Write)
                    .expect("shutdown write");
            });
            let mut got = Vec::new();
            s.read_to_end(&mut got).expect("read to EOF");
            sender.join().expect("the sender");
            got
        });
        // The log is kept as text; (b) writes it back into a directory of its own.
        let _ = std::fs::remove_dir_all(&dir);
        (ran, log, echoed)
    })
}

// ── (a) the live echo ────────────────────────────────────────────────────────

#[test]
fn w377_a_a_live_localhost_echo_echoes_and_exits_zero() {
    let (ran, log, echoed) = live_echo();
    println!("{}\n--- log: {} lines", ran.stderr, log.lines().count());
    assert_eq!(ran.code, Some(0), "exit 0:\n{}", ran.stderr);
    assert_eq!(echoed.len(), live_payload().len(), "every octet echoed");
    assert!(*echoed == live_payload(), "the octets echo back unchanged");
    assert_eq!(
        ran.stdout,
        (live_payload().len() as u64).to_le_bytes().to_vec(),
        "the program counted every octet"
    );
    let records = parse_socket_log(log).expect("the live log is a socket log");
    let octet_records = records
        .iter()
        .filter(|r| matches!(r, SockRecord::Octets(_)))
        .count();
    assert!(
        octet_records >= 2,
        "10 KiB is more than one record: {records:?}"
    );
    assert_eq!(
        records.last(),
        Some(&SockRecord::End),
        "the peer's close is logged"
    );
    assert_eq!(
        all_octets(&records),
        live_payload(),
        "the log holds what arrived"
    );
    assert!(socket::is_socket_log(log), "its first record starts s=");
    let line = ran.socket_line().expect("a socket: line");
    assert!(
        line.starts_with(&format!(
            "socket: {n} octets received in {} records, {n} sent, sent sha256 ",
            records.len(),
            n = live_payload().len()
        )),
        "{line}"
    );
}

// ── (b) the replay ───────────────────────────────────────────────────────────

#[test]
fn w377_b_the_live_log_replays_twice_identically() {
    let (live, log, _) = live_echo();
    let dir = scratch("replay");
    std::fs::write(dir.join("live.log"), log).expect("write the live log");
    std::fs::write(dir.join("live.elf"), image(Prog::Echo, false)).expect("write the image");
    let replay = || {
        yantra_run(&[
            os("--events"),
            dir.join("live.log").as_os_str(),
            dir.join("live.elf").as_os_str(),
        ])
    };
    let (one, two) = (replay(), replay());
    let _ = std::fs::remove_dir_all(&dir);
    let key = |r: &Ran| (r.code, r.stdout.clone(), r.line("steps: "), r.socket_line());
    println!(
        "live {:?}\none {:?}\ntwo {:?}",
        key(live),
        key(&one),
        key(&two)
    );
    assert_eq!(one.code, Some(0), "{}", one.stderr);
    assert!(one.line("steps: ").is_some() && one.socket_line().is_some());
    assert_eq!(key(&one), key(&two), "two replays");
    assert_eq!(key(&one), key(live), "a replay and the live run");
}

// ── (c) UNREAD, EMPTY, an octet and END ─────────────────────────────────────

#[test]
fn w377_c_unread_empty_octet_and_end_are_distinct_and_end_is_sticky() {
    let words = [u64::from(UNREAD), u64::from(EMPTY), u64::from(END)];
    for w in words {
        assert!(w > 0xff, "a status word is no octet: {w:#x}");
    }
    assert!(
        words[0] != words[1] && words[1] != words[2] && words[0] != words[2],
        "three distinct status words"
    );
    let (got, _) = native(image(Prog::Status, false), &status_log());
    let want = Outcome::Finished {
        status: 0,
        out: status_want(),
        sent: Vec::new(),
        records: 2,
        received: 1,
    };
    assert_eq!(got, want);
}

// ── (d) the refusals, one test each ─────────────────────────────────────────

/// `--listen` without `--record-events` before it — as the first argument, and after
/// `--events` — is refused, before anything is read (the image does not exist).
#[test]
fn w377_d_listen_without_record_events_is_refused() {
    let missing = std::ffi::OsStr::new("/nonexistent/w377.elf");
    for args in [
        vec![os("--listen"), os("127.0.0.1:0"), missing],
        vec![
            os("--events"),
            os("/nonexistent/w377.log"),
            os("--listen"),
            os("127.0.0.1:0"),
            missing,
        ],
    ] {
        let r = yantra_run(&args);
        assert_eq!(r.code, Some(1), "{args:?}: {}", r.stderr);
        assert!(
            r.stderr
                .contains("--listen is accepted only right after --record-events"),
            "{args:?}: {}",
            r.stderr
        );
        assert!(!r.stderr.contains("listening on"), "{}", r.stderr);
    }
}

/// Every address but `127.0.0.1` and `::1` is refused BEFORE ANY BIND: the image does
/// not exist and the log is never created, so nothing past the flags ran.
#[test]
fn w377_d_every_address_but_the_loopback_is_refused_before_any_bind() {
    let dir = scratch("addr");
    let log = dir.join("never.log");
    for addr in [
        "0.0.0.0:0",
        "[::]:0",
        "192.0.2.10:0",
        "10.0.0.1:0",
        "127.0.0.2:0",
        "[::ffff:127.0.0.1]:0",
        "localhost:0",
        "127.0.0.1",
        "",
    ] {
        let r = yantra_run(&[
            os("--record-events"),
            log.as_os_str(),
            os("--listen"),
            os(addr),
            os("/nonexistent/w377.elf"),
        ]);
        assert_eq!(r.code, Some(1), "{addr:?}: {}", r.stderr);
        assert!(
            r.stderr.contains("socket: refused") && r.stderr.contains("before any bind"),
            "{addr:?}: {}",
            r.stderr
        );
        assert!(!r.stderr.contains("listening on"), "{addr:?}: {}", r.stderr);
        assert!(!log.exists(), "{addr:?}: the log was created");
    }
    // THE CONTROL: the two loopback addresses get past the address rule, on an image
    // that finishes at once (so no client is needed).
    let elf = dir.join("finish.elf");
    std::fs::write(&elf, raw_image(&finish(), &[(EVENT_TAG, 0)])).unwrap();
    for addr in ["127.0.0.1:0", "[::1]:0"] {
        let r = yantra_run(&[
            os("--record-events"),
            log.as_os_str(),
            os("--listen"),
            os(addr),
            elf.as_os_str(),
        ]);
        assert!(
            !r.stderr.contains("socket: refused — --listen"),
            "{addr}: {}",
            r.stderr
        );
        if addr.starts_with("127") {
            assert_eq!(r.code, Some(0), "{addr}: {}", r.stderr);
            assert!(
                r.stderr.contains("socket: listening on 127.0.0.1:"),
                "{}",
                r.stderr
            );
            assert!(r.socket_line().is_some(), "{}", r.stderr);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A threaded image with `--listen`, or with a socket log, is refused at load.
#[test]
fn w377_d_threads_with_a_socket_are_refused_at_load() {
    let dir = scratch("threads");
    let elf = dir.join("threaded.elf");
    std::fs::write(&elf, threaded_image()).unwrap();
    let live_log = dir.join("live.log");
    let r = yantra_run(&[
        os("--record-events"),
        live_log.as_os_str(),
        os("--listen"),
        os("127.0.0.1:0"),
        elf.as_os_str(),
    ]);
    assert_eq!(r.code, Some(1), "{}", r.stderr);
    assert!(
        r.stderr.contains("a threaded image with a socket"),
        "{}",
        r.stderr
    );
    assert!(!r.stderr.contains("listening on"), "{}", r.stderr);
    let log = dir.join("socket.log");
    std::fs::write(&log, "s=41\ns=end\n").unwrap();
    let r = yantra_run(&[os("--events"), log.as_os_str(), elf.as_os_str()]);
    assert_eq!(r.code, Some(1), "{}", r.stderr);
    assert!(
        r.stderr.contains("a threaded image with a socket"),
        "{}",
        r.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A socket log for an image with no `SASEVENT` is refused at load.
#[test]
fn w377_d_a_socket_log_without_sasevent_is_refused() {
    let dir = scratch("noevent");
    let elf = dir.join("bare.elf");
    std::fs::write(&elf, raw_image(&finish(), &[])).unwrap();
    let log = dir.join("socket.log");
    std::fs::write(&log, "s=41\ns=end\n").unwrap();
    let r = yantra_run(&[os("--events"), log.as_os_str(), elf.as_os_str()]);
    assert_eq!(r.code, Some(1), "{}", r.stderr);
    assert!(
        r.stderr
            .contains("a socket run needs the event slot: no event interface"),
        "{}",
        r.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// The device's refusals, on raw images (`lui`/`addi`/`lw`/`sw` at the window).

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
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i(0x13, rd, 0, rs1, imm)
}
fn jal(rd: u32, off: i32) -> u32 {
    let o = off as u32 & 0x1f_ffff;
    (((o >> 20) & 1) << 31)
        | (((o >> 1) & 0x3ff) << 21)
        | (((o >> 11) & 1) << 20)
        | (((o >> 12) & 0xff) << 12)
        | (rd << 7)
        | 0x6f
}

/// `x5 = SOCK`, asserted so a move of the constant reds here.
fn sock_base() -> [u32; 2] {
    assert_eq!(SOCK, 0x1000_0110, "the encoding below spells this address");
    [lui(5, 0x1000_0000), addi(5, 5, 0x110)]
}

/// The SUCCESS finisher, four words.
fn finish() -> [u32; 4] {
    [
        lui(10, FINISHER as u32),
        lui(11, 0x5000),
        addi(11, 11, 0x555),
        s(0x23, 2, 10, 11, 0),
    ]
}

/// `code` then `(tag, slot)` pairs, as one text segment.
fn raw_image(code: &[u32], tags: &[(u64, u64)]) -> Vec<u8> {
    let mut text: Vec<u8> = code.iter().flat_map(|w| w.to_le_bytes()).collect();
    for (tag, slot) in tags {
        text.extend_from_slice(&tag.to_le_bytes());
        text.extend_from_slice(&slot.to_le_bytes());
    }
    kosha::write(&text)
}

/// A threaded stand-in (`w372_fixpoint_ratchet.rs`'s): the startup's two sp words, the
/// finisher, and the three tags.
fn threaded_image() -> Vec<u8> {
    let mut code = vec![0x17 | 2 << 7, addi(2, 2, 0)];
    code.extend(finish());
    raw_image(
        &code,
        &[(THREADS_TAG, 1), (THREAD_ID_TAG, 0), (EVENT_TAG, 0)],
    )
}

/// A program that WAITS FOREVER: `sd x0, 0(WAIT)` in a loop.
fn waiting_image() -> Vec<u8> {
    assert_eq!(WAIT, 0x1000_0100, "the encoding below spells this address");
    raw_image(
        &[
            lui(5, 0x1000_0000),
            addi(5, 5, 0x100),
            s(0x23, 3, 5, 0, 0),
            jal(0, -4),
        ],
        &[(EVENT_TAG, 0)],
    )
}

/// One access at `SOCK + off` with `x6 = value`, then the finisher; the halt, run on a
/// machine with a device (or none).
fn device_halt(access: u32, value: i32, device: bool) -> Halt {
    let mut code = sock_base().to_vec();
    code.push(addi(6, 0, value));
    code.push(access);
    code.extend(finish());
    let img = raw_image(&code, &[]);
    let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).unwrap();
    if device {
        m.socket = Some(Socket::new());
    }
    m.run(10_000, &mut TestSink::default())
}

fn assert_device(what: &str, h: &Halt, addr: u64, why: &str) {
    match h {
        Halt::Device {
            addr: a, why: w, ..
        } => {
            assert_eq!((*a, *w), (addr, why), "{what}");
        }
        other => panic!("{what}: halted {other:?}, not Device"),
    }
}

const SW: u32 = 2;
const SB: u32 = 0;
const LW: u32 = 2;
const LD: u32 = 3;

#[test]
fn w377_d_no_socket_device_refuses_naming_w377() {
    let h = device_halt(s(0x23, SW, 5, 6, 0), 0, false);
    assert_device("no device", &h, NEXT, socket::WHY_NO_DEVICE);
    let h = device_halt(i(0x03, 7, LW, 5, 4), 0, false);
    assert_device("no device, a read", &h, RX, socket::WHY_NO_DEVICE);
    assert!(socket::WHY_NO_DEVICE.contains("W-377"));
}

#[test]
fn w377_d_a_width_other_than_four_or_a_misaligned_access_refuses() {
    let h = device_halt(s(0x23, SB, 5, 6, 0), 0, true);
    assert_device("sb at NEXT", &h, NEXT, socket::WHY_WIDTH);
    let h = device_halt(i(0x03, 7, LD, 5, 4), 0, true);
    assert_device("ld at RX", &h, RX, socket::WHY_WIDTH);
    let h = device_halt(i(0x03, 7, LW, 5, 6), 0, true);
    assert_device("lw at RX + 2", &h, RX + 2, socket::WHY_WIDTH);
}

#[test]
fn w377_d_a_load_at_next_or_tx_and_a_store_at_rx_refuse() {
    let h = device_halt(i(0x03, 7, LW, 5, 0), 0, true);
    assert_device("lw at NEXT", &h, NEXT, socket::WHY_LOAD_STORE_ONLY);
    let h = device_halt(i(0x03, 7, LW, 5, 8), 0, true);
    assert_device("lw at TX", &h, TX, socket::WHY_LOAD_STORE_ONLY);
    let h = device_halt(s(0x23, SW, 5, 6, 4), 0, true);
    assert_device("sw at RX", &h, RX, socket::WHY_STORE_LOAD_ONLY);
}

#[test]
fn w377_d_next_with_a_value_other_than_zero_refuses() {
    let h = device_halt(s(0x23, SW, 5, 6, 0), 1, true);
    assert_device("NEXT 1", &h, NEXT, socket::WHY_NEXT_VALUE);
}

#[test]
fn w377_d_tx_with_a_value_above_0xff_refuses() {
    let h = device_halt(s(0x23, SW, 5, 6, 8), 0x100, true);
    assert_device("TX 0x100", &h, TX, socket::WHY_TX_VALUE);
    // THE CONTROL: 0xff is sent, and the run finishes.
    let h = device_halt(s(0x23, SW, 5, 6, 8), 0xff, true);
    assert!(
        matches!(
            h,
            Halt::Finisher {
                status: Some(0),
                ..
            }
        ),
        "{h:?}"
    );
}

#[test]
fn w377_d_any_access_at_0x11c_refuses() {
    let h = device_halt(i(0x03, 7, LW, 5, 0xc), 0, true);
    assert_device("lw at CLOSE", &h, CLOSE, socket::WHY_CLOSE);
    let h = device_halt(s(0x23, SW, 5, 6, 0xc), 0, true);
    assert_device("sw at CLOSE", &h, CLOSE, socket::WHY_CLOSE);
}

/// THE INTERPRETER REFUSES IN THE NATIVE DEVICE'S WORDS at the four addresses with a
/// socket log set; every other address — and every address with no socket log — keeps
/// `W-350`'s refusal.
#[test]
fn w377_d_the_interpreter_refuses_by_the_devices_words_and_keeps_w350_elsewhere() {
    let one = |body: &str| {
        format!(
            "{head}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः अष्टकम् ॱॱ न६४ भवति ० ।
    {body}
    प्रत्यागमनम् ० ।
इति
",
            head = head()
        )
    };
    let store = |at: u64, v: u64| {
        one(&format!(
            "अवगणना भवति {} {} {} ।",
            q(STORE_CALL),
            dec(at),
            dec(v)
        ))
    };
    let load = |at: u64| one(&format!("अष्टकम् भवति {} {} ।", q(LOAD_CALL), dec(at)));
    let log = status_log();
    for (what, src, why) in [
        ("NEXT 1", store(NEXT, 1), socket::WHY_NEXT_VALUE),
        ("TX 0x100", store(TX, 0x100), socket::WHY_TX_VALUE),
        ("store RX", store(RX, 0), socket::WHY_STORE_LOAD_ONLY),
        ("load NEXT", load(NEXT), socket::WHY_LOAD_STORE_ONLY),
        ("load TX", load(TX), socket::WHY_LOAD_STORE_ONLY),
        ("load CLOSE", load(CLOSE), socket::WHY_CLOSE),
        ("store CLOSE", store(CLOSE, 0), socket::WHY_CLOSE),
    ] {
        assert_eq!(
            interpreted(&src, &log),
            Outcome::Device(why.to_string()),
            "{what}"
        );
    }
    let w350 = |o: Outcome| matches!(o, Outcome::Other(r) if r.contains("(`W-350`)"));
    assert!(
        w350(interpreted(&load(SOCK + 0x10), &log)),
        "past the window"
    );
    assert!(w350(interpreted(&load(RX + 2), &log)), "inside, misaligned");
    assert!(w350(interpreted_with(&load(RX), 0, None)), "no socket log");
}

#[test]
fn w377_d_the_socket_log_refuses_what_it_cannot_read() {
    let ok = |t: &str| parse_socket_log(t).unwrap_or_else(|e| panic!("{t:?}: {e}"));
    assert_eq!(ok("# h\n\ns=41\ns=end\n"), status_log());
    let cap = format!("s={}\n", "ab".repeat(RECORD_CAP));
    assert_eq!(ok(&cap).len(), 1, "4096 octets is one record");
    for (bad, words) in [
        ("s=\n", "an empty s="),
        ("s=414\n", "two per octet"),
        ("s=4g\n", "not a lowercase hex digit"),
        ("s=4A\n", "not a lowercase hex digit"),
        (
            &*format!("s={}\n", "ab".repeat(RECORD_CAP + 1)),
            "at most 4096",
        ),
        ("s=41\ns=end\ns=42\n", "after the s=end"),
        ("s=end\ns=end\n", "after the s=end"),
        ("s=41\n3\n", "not a socket record"),
        ("s=41\nt=5\n", "a CLOCK record"),
        ("s=41\n@0\n", "a THREAD record"),
        ("s=41\ns=42", "torn"),
    ] {
        let e = parse_socket_log(bad).unwrap_err();
        assert!(e.contains(words), "{bad:?}: {e}");
    }
}

#[test]
fn w377_d_the_event_and_thread_readers_refuse_a_socket_record_by_name() {
    for log in ["s=41\n", "3\ns=end\n"] {
        let e = parse_event_log(log).unwrap_err();
        assert!(e.contains("a SOCKET record (W-377"), "{e}");
        let e = parse_thread_log(log).unwrap_err();
        assert!(e.contains("a SOCKET record (W-377"), "{e}");
    }
    // A log whose FIRST record is plain is an event log, and its s= line is refused.
    assert!(!socket::is_socket_log("# x\n3\ns=41\n"));
}

/// A SHORT log, a LONG log and a wait AFTER END refuse alike on all three engines, and
/// the real binary refuses each with exit 1 by name.
#[test]
fn w377_d_short_long_and_after_end_refuse_alike_and_in_the_binary() {
    let p = Prog::Status;
    let short = parse_socket_log("s=41\n").unwrap();
    assert_all(
        "short",
        &three(p, &short),
        &Outcome::Short {
            index: 1,
            out: status_want()[..40].to_vec(),
            sent: Vec::new(),
        },
    );
    let long = parse_socket_log("s=41\ns=42\ns=end\n").unwrap();
    assert_all(
        "long",
        &three(p, &long),
        &Outcome::Long {
            delivered: 2,
            len: 3,
        },
    );
    let after = parse_socket_log("s=end\n").unwrap();
    let words = |ws: &[u64]| -> Vec<u8> { ws.iter().flat_map(|w| w.to_le_bytes()).collect() };
    let e = u64::from(END);
    assert_all(
        "after END",
        &three(p, &after),
        &Outcome::AfterEnd {
            index: 1,
            out: words(&[u64::from(UNREAD), u64::from(EMPTY), 0, e, e]),
            sent: Vec::new(),
        },
    );
    let dir = scratch("refusals");
    let elf = dir.join("status.elf");
    std::fs::write(&elf, image(p, false)).unwrap();
    for (log, words) in [
        (
            "s=41\n",
            "the socket log is SHORTER than the run: wait index 1",
        ),
        (
            "s=41\ns=42\ns=end\n",
            "the socket log is LONGER than the run",
        ),
        ("s=end\n", "a WAIT after END: wait index 1"),
    ] {
        let path = dir.join("socket.log");
        std::fs::write(&path, log).unwrap();
        let r = yantra_run(&[os("--events"), path.as_os_str(), elf.as_os_str()]);
        assert_eq!(r.code, Some(1), "{log:?}: {}", r.stderr);
        assert!(
            r.stderr
                .lines()
                .any(|l| l.starts_with("events: REFUSED") && l.contains(words)),
            "{log:?}: {}",
            r.stderr
        );
        assert!(
            r.socket_line().is_some(),
            "the socket line is said: {}",
            r.stderr
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// LIVE: a wait after the peer closed is refused by name, exit 1.
#[test]
fn w377_d_a_live_wait_after_end_is_refused() {
    let dir = scratch("after-end");
    let (ran, log, ()) = live(&dir, &waiting_image(), drop);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(
        ran.stderr.contains("a WAIT after END: wait index 1"),
        "{}",
        ran.stderr
    );
    assert_eq!(
        parse_socket_log(&log).unwrap(),
        vec![SockRecord::End],
        "{log}"
    );
}

/// LIVE: a wait idle for more than 30 s is refused by name, exit 1 (this test takes 30 s).
#[test]
fn w377_d_a_live_wait_idle_for_30_s_is_refused() {
    let dir = scratch("idle");
    let (ran, log, held) = live(&dir, &waiting_image(), |s| {
        // Hold the connection open, send nothing, until the host gives up.
        let mut s = s;
        let mut sink = Vec::new();
        let _ = s.read_to_end(&mut sink);
        sink
    });
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(
        ran.stderr
            .contains("IDLE for more than 30 s at wait index 0"),
        "{}",
        ran.stderr
    );
    assert!(held.is_empty());
    assert!(parse_socket_log(&log).unwrap().is_empty(), "{log}");
}

// ── (e) the mutants ──────────────────────────────────────────────────────────

/// How the test-side driver below bends the device.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mutant {
    /// None: the driver is `replay_socket`, step by step (the control).
    Faithful,
    /// A latch that does not pop: each popped octet is pushed back.
    NoPop,
    /// END answered as EMPTY: `ended` cleared again after `s=end` is applied.
    EndAsEmpty,
    /// The first record delivered before the program starts, not at its wait.
    Early,
}

/// [`replay_socket`]'s loop, one [`Machine::step`] at a time, with `mutant` applied.
fn driven(img: &[u8], log: &[SockRecord], mutant: Mutant, budget: u64) -> Outcome {
    let mut m = Machine::load_elf(img, yantra::ram_for(img)).unwrap();
    m.socket = Some(Socket::new());
    let tag = find_event_slot(&m.mem).unwrap();
    let mut out = TestSink::default();
    let mut delivered = 0;
    if mutant == Mutant::Early {
        socket::deliver(&mut m, tag, &log[0]);
        delivered = 1;
    }
    for _ in 0..budget {
        let sock = m.socket.as_ref().unwrap();
        let (head, len) = (sock.queue.front().copied(), sock.queue.len());
        let h = m.step(&mut out);
        if mutant == Mutant::NoPop {
            let sock = m.socket.as_mut().unwrap();
            if sock.queue.len() < len {
                sock.queue.push_front(head.unwrap());
            }
        }
        match h {
            None => {}
            Some(Halt::Wait { .. }) => {
                if socket::ended(&m) {
                    return Outcome::AfterEnd {
                        index: delivered,
                        out: out.out,
                        sent: out.sent,
                    };
                }
                let Some(r) = log.get(delivered) else {
                    return Outcome::Short {
                        index: delivered,
                        out: out.out,
                        sent: out.sent,
                    };
                };
                socket::deliver(&mut m, tag, r);
                if mutant == Mutant::EndAsEmpty && *r == SockRecord::End {
                    m.socket.as_mut().unwrap().ended = false;
                }
                delivered += 1;
            }
            Some(Halt::Finisher {
                status: Some(status),
                ..
            }) if delivered == log.len() => {
                return Outcome::Finished {
                    status,
                    out: out.out,
                    sent: out.sent,
                    records: delivered,
                    received: m.socket.as_ref().unwrap().received,
                };
            }
            Some(h) => return Outcome::Other(format!("halted {h:?} after {delivered}")),
        }
    }
    Outcome::Other(format!("the step budget of {budget} ran out"))
}

fn echo_want(log: &[SockRecord]) -> Outcome {
    let all = all_octets(log);
    Outcome::Finished {
        status: 0,
        out: (all.len() as u64).to_le_bytes().to_vec(),
        sent: all.clone(),
        records: log.len(),
        received: all.len() as u64,
    }
}

/// The control for every driven mutant: the faithful driver equals `replay_socket`.
#[test]
fn w377_e_control_the_faithful_driver_is_the_replay() {
    let img = image(Prog::Echo, false);
    let log = echo_log();
    assert_eq!(
        driven(img, &log, Mutant::Faithful, STEPS),
        native(img, &log).0
    );
    assert_eq!(native(img, &log).0, echo_want(&log));
    let img = image(Prog::Status, false);
    assert_eq!(
        driven(img, &status_log(), Mutant::Faithful, STEPS),
        native(img, &status_log()).0
    );
}

#[test]
fn w377_e_mutant_a_latch_that_does_not_pop_is_red() {
    let log = echo_log();
    let got = driven(image(Prog::Echo, false), &log, Mutant::NoPop, 2_000_000);
    println!("{}", brief(&got));
    assert_ne!(got, echo_want(&log));
}

#[test]
fn w377_e_mutant_end_answered_as_empty_is_red() {
    let log = echo_log();
    let got = driven(image(Prog::Echo, false), &log, Mutant::EndAsEmpty, STEPS);
    println!("{}", brief(&got));
    assert_ne!(got, echo_want(&log));
    let got = driven(
        image(Prog::Status, false),
        &status_log(),
        Mutant::EndAsEmpty,
        STEPS,
    );
    assert_ne!(got, native(image(Prog::Status, false), &status_log()).0);
}

/// Records MERGED at replay: the same octets in fewer records — the count goes red (one
/// wait fewer retires fewer instructions), and so does the record count.
#[test]
fn w377_e_mutant_records_merged_at_replay_is_red() {
    let log = echo_log();
    let mut merged = log.clone();
    let SockRecord::Octets(second) = merged.remove(1) else {
        unreachable!()
    };
    if let SockRecord::Octets(first) = &mut merged[0] {
        first.extend(second);
    }
    let img = image(Prog::Echo, false);
    let (a, steps_a) = native(img, &log);
    let (b, steps_b) = native(img, &merged);
    println!("steps {steps_a} vs merged {steps_b}");
    assert_ne!(steps_a, steps_b, "the retired count");
    assert_ne!(a, b, "the record count");
}

/// TX DROPPED: a host whose `Output::sent` is the default no-op.
#[test]
fn w377_e_mutant_tx_dropped_is_red() {
    #[derive(Default)]
    struct Dropping(Vec<u8>);
    impl Output for Dropping {
        fn putc(&mut self, b: u8) {
            self.0.push(b);
        }
    }
    impl Taken for Dropping {
        fn taken(&self) -> (Vec<u8>, Vec<u8>) {
            (self.0.clone(), Vec::new())
        }
    }
    let log = echo_log();
    let (got, _) = native_with(image(Prog::Echo, false), &log, &mut Dropping::default());
    assert_ne!(got, echo_want(&log));
}

#[test]
fn w377_e_mutant_delivery_before_the_wait_is_red() {
    let img = image(Prog::Status, false);
    let got = driven(img, &status_log(), Mutant::Early, STEPS);
    println!("{}", brief(&got));
    assert_ne!(got, native(img, &status_log()).0);
}

/// AN INTERPRETER SERVING RECORD k+1 AT WAIT k, observed from outside (`w373_wait.rs`'s
/// (e)): handed the log less its first record.
#[test]
fn w377_e_mutant_an_interpreter_serving_the_next_record_is_red() {
    let log = status_log();
    let native_o = native(image(Prog::Status, false), &log).0;
    let mutant = interpreted_with(&status_program(), log.len(), Some(&log[1..]));
    println!("native {native_o:?}\nmutant {mutant:?}");
    assert_eq!(
        interpreted(&status_program(), &log),
        native_o,
        "the control"
    );
    assert_ne!(mutant, native_o);
}

// ── (f) three engines ────────────────────────────────────────────────────────

#[test]
fn w377_f_three_engines_agree_on_unread_empty_octet_end() {
    let want = Outcome::Finished {
        status: 0,
        out: status_want(),
        sent: Vec::new(),
        records: 2,
        received: 1,
    };
    assert_all("(c)", &three(Prog::Status, &status_log()), &want);
}

#[test]
fn w377_f_three_engines_agree_on_an_echo_log_replay() {
    let log = echo_log();
    assert_all("echo", &three(Prog::Echo, &log), &echo_want(&log));
}

/// The `.t1` image and the Rust twin also retire the same count on the echo log? NOT
/// claimed: two compilers emit different code. Each native engine retires the same count
/// on every replay of the same log, which is what (b) needs.
#[test]
fn w377_f_a_replay_retires_the_same_count_every_time() {
    for twin in [false, true] {
        let img = image(Prog::Echo, twin);
        assert_eq!(native(img, &echo_log()), native(img, &echo_log()));
    }
}

#[test]
fn w377_f_the_interpreters_copies_are_yantras() {
    assert_eq!(
        (
            nirvahana::SOCK_NEXT,
            nirvahana::SOCK_RX,
            nirvahana::SOCK_TX,
            nirvahana::SOCK_CLOSE
        ),
        (NEXT, RX, TX, CLOSE)
    );
    assert_eq!(
        (
            nirvahana::SOCK_EMPTY,
            nirvahana::SOCK_END,
            nirvahana::SOCK_UNREAD
        ),
        (EMPTY, END, UNREAD)
    );
    assert_eq!(
        [
            nirvahana::SOCK_WHY_LOAD_STORE_ONLY,
            nirvahana::SOCK_WHY_STORE_LOAD_ONLY,
            nirvahana::SOCK_WHY_NEXT_VALUE,
            nirvahana::SOCK_WHY_TX_VALUE,
            nirvahana::SOCK_WHY_CLOSE,
        ],
        [
            socket::WHY_LOAD_STORE_ONLY,
            socket::WHY_STORE_LOAD_ONLY,
            socket::WHY_NEXT_VALUE,
            socket::WHY_TX_VALUE,
            socket::WHY_CLOSE,
        ]
    );
    assert_eq!(nirvahana::EVENT_TAG, EVENT_TAG);
}

// ── (g) the ratchets ─────────────────────────────────────────────────────────

/// A corpus file's CODE tokens, `(line, token)`: `॰` comments cut, `उक्तम् … इति` strings
/// dropped (`w373_wait.rs`'s `code_tokens`).
fn code_tokens(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_string = false;
    for (n, line) in text.lines().enumerate() {
        let code = line.split('॰').next().unwrap_or("");
        for tok in code.split_whitespace() {
            if in_string {
                if tok == "इति" {
                    in_string = false;
                }
                continue;
            }
            if tok == "उक्तम्" {
                in_string = true;
                continue;
            }
            out.push((n + 1, tok.to_string()));
        }
    }
    out
}

/// Every spelling of `v` the ratchet refuses, GENERATED: the Devanagari decimal, the T1
/// hexadecimal (`०षोड्…`), and the `0x` forms — plain and grouped by four, in ASCII
/// digits and in Devanagari digits.
fn spellings(v: u64) -> Vec<String> {
    const DIGITS: [&str; 16] = [
        "०", "१", "२", "३", "४", "५", "६", "७", "८", "९", "अ", "आ", "इ", "ई", "उ", "ऊ",
    ];
    let hexd = format!("{v:x}");
    let t1hex: String = hexd
        .chars()
        .map(|c| DIGITS[c.to_digit(16).unwrap() as usize])
        .collect();
    let grouped = {
        let cs: Vec<char> = hexd.chars().collect();
        let mut g = String::new();
        for (k, c) in cs.iter().enumerate() {
            if k > 0 && (cs.len() - k).is_multiple_of(4) {
                g.push('_');
            }
            g.push(*c);
        }
        g
    };
    let deva = |s: &str| -> String {
        s.chars()
            .map(|c| match c.to_digit(10) {
                Some(d) => DIGITS[d as usize].to_string(),
                None => c.to_string(),
            })
            .collect()
    };
    let x = format!("0x{hexd}");
    let xg = format!("0x{grouped}");
    vec![dec(v), format!("०षोड्{t1hex}"), deva(&x), deva(&xg), x, xg]
}

fn sock_numerals() -> Vec<(u64, String)> {
    [NEXT, RX, TX, CLOSE]
        .iter()
        .flat_map(|&v| spellings(v).into_iter().map(move |s| (v, s)))
        .collect()
}

/// Every whole-token use of a SOCK numeral in corpus CODE, as `file:line: token`.
fn sock_numerals_in_code(files: &[(String, String)]) -> Vec<String> {
    let numerals = sock_numerals();
    let mut hits = Vec::new();
    for (name, text) in files {
        for (line, tok) in code_tokens(text) {
            if let Some((v, _)) = numerals.iter().find(|(_, n)| *n == tok) {
                hits.push(format!("{name}:{line}: {tok} ({v:#x})"));
            }
        }
    }
    hits
}

fn corpus() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(corpus_root())
        .expect("crates/sadhana-t1/src")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).expect("a corpus file"),
            )
        })
        .collect();
    files.sort();
    assert!(files.len() >= 20, "the corpus is {} files", files.len());
    files
}

/// (g) 2: the four SOCK numerals appear 0 times as whole tokens in corpus code.
#[test]
fn w377_g_the_sock_numerals_appear_in_no_corpus_code() {
    let n = sock_numerals();
    assert_eq!(n.len(), 24, "four addresses, six spellings each: {n:?}");
    assert!(n.iter().any(|(_, s)| s == "0x1000_0114"), "{n:?}");
    let hits = sock_numerals_in_code(&corpus());
    assert!(hits.is_empty(), "SOCK in corpus code:\n{}", hits.join("\n"));
}

/// (g) 2's MUTATION CONTROL: an RX read inserted into a COPY of `lex.t1` goes red naming
/// file:line, in each spelling; the same text in a comment or a string does not.
#[test]
fn w377_g_the_sock_numeral_ratchet_mutation_control() {
    let files = corpus();
    let (name, text) = files
        .iter()
        .find(|(n, _)| n == "lex.t1")
        .expect("lex.t1")
        .clone();
    let insert_at = 10;
    let mutate = |inserted: &str| -> Vec<(String, String)> {
        let mut lines: Vec<&str> = text.lines().collect();
        lines.insert(insert_at, inserted);
        let mut copy = files.clone();
        for f in &mut copy {
            if f.0 == name {
                f.1 = lines.join("\n");
            }
        }
        copy
    };
    for spelled in spellings(RX) {
        let call = format!("    अवगणना भवति {} {spelled} ।", q(LOAD_CALL));
        let hits = sock_numerals_in_code(&mutate(&call));
        assert_eq!(hits.len(), 1, "{spelled}: {hits:?}");
        assert!(
            hits[0].starts_with(&format!("{name}:{}: ", insert_at + 1)),
            "names file:line: {hits:?}"
        );
    }
    let comment = format!("॰ {}", dec(RX));
    assert!(
        sock_numerals_in_code(&mutate(&comment)).is_empty(),
        "a comment"
    );
    let string = format!("    चरः क ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् {} इति ।", dec(RX));
    assert!(
        sock_numerals_in_code(&mutate(&string)).is_empty(),
        "a string"
    );
}

/// The network names, matched as whole identifiers, and the two `net` paths.
const NET_TOKENS: &[&str] = &["TcpListener", "TcpStream", "UdpSocket"];
const NET_PATHS: &[&str] = &["std::net", "core::net"];

fn line_names_the_network(code: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    NET_PATHS.iter().any(|p| code.contains(p))
        || NET_TOKENS.iter().any(|tok| {
            code.match_indices(tok).any(|(at, _)| {
                !code[..at].chars().next_back().is_some_and(ident)
                    && !code[at + tok.len()..].chars().next().is_some_and(ident)
            })
        })
}

/// Every network use under `root`, as `relative/path:line: text`. Comment lines are
/// skipped: a margin may name the network.
fn net_uses(root: &Path) -> Vec<(String, String)> {
    let mut uses = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
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
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            for (n, line) in std::fs::read_to_string(&p).unwrap().lines().enumerate() {
                let code = line.trim_start();
                if !code.starts_with("//") && line_names_the_network(code) {
                    uses.push((rel.clone(), format!("{rel}:{}: {code}", n + 1)));
                }
            }
        }
    }
    uses
}

fn yantra_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

const HOST: &str = "bin/yantra-run.rs";

/// (g) 3: `TcpListener`, `TcpStream` and `std::net` appear under `crates/yantra/src`
/// only in `bin/yantra-run.rs` — and there, at least once (the scan is not vacuous).
#[test]
fn w377_g_only_yantra_run_names_the_network() {
    let uses = net_uses(&yantra_src());
    assert!(
        uses.iter().any(|(f, _)| f == HOST),
        "the host itself names no network — the scan is wrong"
    );
    let outside: Vec<&String> = uses
        .iter()
        .filter(|(f, _)| f != HOST)
        .map(|(_, l)| l)
        .collect();
    assert!(
        outside.is_empty(),
        "the network outside the host:\n{outside:#?}"
    );
}

/// (g) 3's MUTATION CONTROL: a scratch copy of `crates/yantra/src` with ONE network use
/// added to `socket.rs` — the scan reports exactly that line.
#[test]
fn w377_g_the_network_ratchet_mutation_control() {
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
    let dir = scratch("net");
    let src = dir.join("src");
    copy(&yantra_src(), &src);
    let clean = net_uses(&src);
    let f = src.join("socket.rs");
    let mut text = std::fs::read_to_string(&f).unwrap();
    text.push_str("\nfn smuggled() { let _ = TcpListener::bind(\"127.0.0.1:0\"); }\n");
    std::fs::write(&f, text).unwrap();
    let mutated = net_uses(&src);
    let _ = std::fs::remove_dir_all(&dir);
    let outside = |u: &[(String, String)]| -> Vec<String> {
        u.iter()
            .filter(|(f, _)| f != HOST)
            .map(|(_, l)| l.clone())
            .collect()
    };
    assert!(outside(&clean).is_empty(), "the copy starts clean");
    let m = outside(&mutated);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(
        m[0].starts_with("socket.rs:") && m[0].contains("smuggled"),
        "{m:?}"
    );
}

// ── review fixes at 05eb3a04 (coordinator a peer session) ────────────────────────

/// A record of `n` octets, all `0x5a`.
fn octets_of(n: usize) -> SockRecord {
    SockRecord::Octets(vec![0x5a; n])
}

/// FIX 1, REPLAY: the SESSION CAP. Records up to exactly `SESSION_CAP` delivered octets are
/// delivered; the record that would pass it is refused BY NAME at its wait, never applied.
#[test]
fn w377_r1_replay_refuses_past_the_session_cap() {
    assert_eq!(socket::SESSION_CAP, 64 << 20, "64 MiB");
    let full = (socket::SESSION_CAP as usize) / RECORD_CAP;
    let mut log: Vec<SockRecord> = (0..full).map(|_| octets_of(RECORD_CAP)).collect();
    log.push(octets_of(1));
    let img = waiting_image();
    let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).unwrap();
    let tag = find_event_slot(&m.mem).unwrap();
    let r = replay_socket(&mut m, tag, &log, STEPS, &mut TestSink::default());
    let SockReplayed::OverCap { index, why, .. } = r else {
        panic!("not refused at the cap: {r:?}")
    };
    assert_eq!(index, full, "the record past the cap");
    assert!(
        why.starts_with(socket::SOCKET_LIMIT_EXCEEDED),
        "named: {why}"
    );
    assert_eq!(
        m.socket.as_ref().unwrap().received,
        socket::SESSION_CAP,
        "exactly the cap was delivered, and the refused record was not applied"
    );
}

/// FIX 1, THE INTERPRETER: the same refusal, in the same words, at the same wait.
#[test]
fn w377_r1_the_interpreter_refuses_past_the_session_cap_alike() {
    assert_eq!(nirvahana::SOCK_SESSION_CAP, socket::SESSION_CAP);
    assert_eq!(
        nirvahana::SOCKET_LIMIT_EXCEEDED,
        socket::SOCKET_LIMIT_EXCEEDED
    );
    let src = format!(
        "{head}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः समम् ० आदि
        अवगणना भवति {wait} ० ।
    इति
    प्रत्यागमनम् ० ।
इति
",
        head = head(),
        wait = wait_call()
    );
    let full = (socket::SESSION_CAP as usize) / RECORD_CAP;
    let mut log: Vec<SockRecord> = (0..full).map(|_| octets_of(RECORD_CAP)).collect();
    log.push(octets_of(1));
    let got = interpreted(&src, &log);
    let Outcome::Other(why) = got else {
        panic!("{}", brief(&got))
    };
    assert!(
        why.starts_with(&socket::over_cap_refusal(full, 1, socket::SESSION_CAP + 1)),
        "{why}"
    );
}

/// FIX 1, LIVE: a client that keeps sending to a program that waits and never pops is
/// refused BY NAME at the cap, exit 1, and the log holds no more than the cap.
#[test]
fn w377_r1_a_live_client_sending_past_the_cap_is_refused_and_the_log_is_bounded() {
    let dir = scratch("cap");
    let (ran, log, ()) = live(&dir, &waiting_image(), |s| {
        let mut w = s.try_clone().expect("clone");
        let sender = std::thread::spawn(move || {
            let chunk = vec![0x5au8; 1 << 16];
            // 64 MiB and 1 MiB more; the host refuses partway, so a write error is expected.
            for _ in 0..(socket::SESSION_CAP as usize / chunk.len() + 16) {
                if w.write_all(&chunk).is_err() {
                    break;
                }
            }
        });
        let mut s = s;
        let _ = s.read_to_end(&mut Vec::new());
        sender.join().expect("the sender");
    });
    let _ = std::fs::remove_dir_all(&dir);
    println!("{}", ran.stderr);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(
        ran.stderr
            .lines()
            .any(|l| l.starts_with("events: REFUSED") && l.contains(socket::SOCKET_LIMIT_EXCEEDED)),
        "{}",
        ran.stderr
    );
    let records = parse_socket_log(&log).expect("the log parses");
    let total = all_octets(&records).len() as u64;
    assert!(total <= socket::SESSION_CAP, "the log holds {total} octets");
    assert!(
        log.len() as u64 <= 2 * socket::SESSION_CAP + 4 * records.len() as u64 + 4096,
        "the log is bounded: {} octets",
        log.len()
    );
    let line = ran.socket_line().expect("the account is said");
    assert!(
        line.starts_with(&format!("socket: {total} octets received")),
        "{line}"
    );
}

/// An image that reads RX BEFORE ANY WAIT, prints its low two octets on the UART, and
/// finishes without waiting — so its live log is the header and no record.
fn reads_before_waiting_image() -> Vec<u8> {
    assert_eq!(
        yantra::UART,
        0x1000_0000,
        "the encoding below spells this address"
    );
    let mut code = sock_base().to_vec();
    code.extend([
        i(0x03, 6, LW, 5, 4), // lw x6, RX
        lui(7, 0x1000_0000),  // x7 = UART
        s(0x23, SB, 7, 6, 0), // sb x6, 0(x7)
        i(0x13, 6, 5, 6, 8),  // srli x6, x6, 8
        s(0x23, SB, 7, 6, 0), // sb x6, 0(x7)
    ]);
    code.extend(finish());
    raw_image(&code, &[(EVENT_TAG, 0)])
}

/// FIX 2: A HEADER-ONLY LOG IS A SOCKET LOG. Live, a client connects and closes before any
/// data; the image reads RX (UNREAD) before it would wait and finishes. The replay of that
/// log — the header and no record — runs with the device, and is identical to live.
#[test]
fn w377_r2_a_header_only_log_replays_as_a_socket_log() {
    let dir = scratch("header-only");
    let img = reads_before_waiting_image();
    let (live_run, log, ()) = live(&dir, &img, drop);
    assert_eq!(live_run.code, Some(0), "{}", live_run.stderr);
    assert_eq!(
        live_run.stdout,
        UNREAD.to_le_bytes()[..2].to_vec(),
        "live read UNREAD"
    );
    assert_eq!(
        log,
        format!("{}\n", socket::SOCKET_LOG_HEADER),
        "header and no record"
    );
    assert!(socket::is_socket_log(&log));
    let path = dir.join("header.log");
    std::fs::write(&path, &log).unwrap();
    let elf = dir.join("live.elf");
    let replay = yantra_run(&[os("--events"), path.as_os_str(), elf.as_os_str()]);
    let _ = std::fs::remove_dir_all(&dir);
    let key = |r: &Ran| (r.code, r.stdout.clone(), r.line("steps: "), r.socket_line());
    println!(
        "live {:?}\nreplay {:?}\n{}",
        key(&live_run),
        key(&replay),
        replay.stderr
    );
    assert_eq!(key(&replay), key(&live_run));
}

/// FIX 3: THE LISTENER CLOSES AT THE FIRST ACCEPT. Once the first client's octet has echoed
/// back (so it was accepted), a second client is refused AT CONNECT; the first finishes.
#[test]
fn w377_r3_a_second_client_is_refused_at_connect() {
    let dir = scratch("second");
    let (ran, _, (echo, second)) = live(&dir, image(Prog::Echo, false), |mut s| {
        let peer = s.peer_addr().expect("peer");
        s.write_all(b"a").expect("send");
        let mut one = [0u8; 1];
        s.read_exact(&mut one).expect("the echo");
        let second = std::net::TcpStream::connect(peer).map_err(|e| e.kind());
        s.shutdown(std::net::Shutdown::Write).expect("shutdown");
        let mut rest = Vec::new();
        s.read_to_end(&mut rest).expect("read to EOF");
        (one[0], second.map(|_| ()))
    });
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(echo, b'a');
    assert_eq!(
        second,
        Err(std::io::ErrorKind::ConnectionRefused),
        "a second client must be refused at connect:\n{}",
        ran.stderr
    );
}
