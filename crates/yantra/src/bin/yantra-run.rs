//! Run an ELF natively and print what it wrote to the UART — the control for the browser.
//!
//! The browser arm is only evidence if the same interpreter, on the same artefact, is
//! known to produce the right bytes somewhere it can be checked easily.
use std::process::ExitCode;
use yantra::Halt;
use yantra::socket::{self, SockRecord};
use yantra::{DEFAULT_STEPS, Output};

/// The UART bytes, plus the highest RAM offset any store reached — printed
/// with `YANTRA_WATERMARK` set, so a run says how much of RAM it touched.
struct Sink {
    out: Vec<u8>,
    high_water: usize,
    /// Reads of the retired-instruction counter (`W-374`), reported after the run.
    counter_reads: u64,
    /// Every octet the program sent through the socket's TX (`W-377`), live or replayed:
    /// what the closing `socket:` line's `sent sha256` is taken over.
    sent: Vec<u8>,
    /// LIVE ONLY: the accepted connection's write half. Each TX octet is written AT ONCE,
    /// fire-and-forget; a failure is said by the host and never reaches the program
    /// (addendum §3 — anything it could observe would be an input the log does not hold).
    stream: Option<std::net::TcpStream>,
    /// `--listen` is in force, so TX octets go to `stream`.
    live_tx: bool,
    /// A write to the connection failed; nothing more is written to it.
    send_failed: bool,
    /// An octet was sent before any client connected (said once).
    sent_without_client: bool,
}

impl Output for Sink {
    fn putc(&mut self, byte: u8) {
        self.out.push(byte);
    }
    fn counter_read(&mut self) {
        self.counter_reads += 1;
    }
    fn sent(&mut self, byte: u8) {
        use std::io::Write;
        self.sent.push(byte);
        if !self.live_tx || self.send_failed {
            return;
        }
        let before = self.sent.len() - 1;
        match self.stream.as_mut() {
            Some(s) => {
                if let Err(e) = s.write_all(&[byte]) {
                    eprintln!(
                        "socket: send failed after {before} octets — {e}; nothing more is \
                         written, and the program is not told (W-377)"
                    );
                    self.send_failed = true;
                }
            }
            None if !self.sent_without_client => {
                eprintln!(
                    "socket: send failed after {before} octets — no client has connected yet \
                     (one is accepted at the first wait); the program is not told (W-377)"
                );
                self.sent_without_client = true;
            }
            None => {}
        }
    }
    fn stored(&mut self, at: usize, width: usize) {
        if at + width > self.high_water {
            self.high_water = at + width;
        }
    }
}
/// A verified Saṃpuṭa archive's three parts that reach the run besides its decoder
/// (`F-022`): the input channel's octets, its module name, and the header's step cap.
struct Unpacked {
    payload: Vec<u8>,
    name: String,
    step_cap: u64,
}

/// The input channel as the run receives it: the octets, the module name, and how the two
/// are named in the `input:` report line.
type Input<'a> = (std::borrow::Cow<'a, [u8]>, Vec<u8>, String, String);

/// The header line a log RECORDED under `--files` carries (v1.0.1). A `#` line, so every
/// reader skips it; the replay path looks for it and refuses a run without `--files`.
/// The directory is deliberately not recorded: the fact of the grant is what a replay needs.
const FILES_GRANT_LINE: &str =
    "# yantra-run --files: this run was granted a file root; replay it with --files DIR";

fn main() -> ExitCode {
    // ॥ `--version` IS RECOGNISED ONLY AS THE SOLE ARGUMENT — W-347 ॥
    //
    // THE MARGIN TEN LINES BELOW IS WHY, and it was written before this flag
    // existed: "EVERYTHING AFTER THE IMAGE IS THE PROGRAM'S … A flag added later
    // would have to be rejected before this line or it would silently stop
    // reaching the program." This is that rejection, and it is deliberately the
    // narrowest possible: `argc == 2` and the one argument spelled exactly. A
    // program invoked as `yantra-run prog.elf --version` still receives
    // `--version` in its own argv, because the image is argument one and this
    // test never fires. There is no path by which this binary can eat an
    // argument the program was meant to see.
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() == 2 && argv[1] == "--version" {
        // ॥ STDERR, NOT STDOUT, AND THIS BINARY'S CONTRACT IS WHY ॥
        //
        // `paradigm_boundary.rs:1620` is a SOURCE-LEVEL ratchet: in `yantra-run`
        // stdout carries the guest program's PAYLOAD BYTES and nothing else, so a
        // program's output can be piped without contamination. It caught this on
        // the gate, correctly — a `println!` here is a byte that is not the
        // payload, even on a path where no program runs.
        //
        // SO THE TWO BINARIES DIFFER DELIBERATELY: `t1_image --version` goes to
        // stdout, because that binary already reports `build:` and `write:` there
        // and owns its stdout; `yantra-run --version` goes to stderr, because it
        // does not. A script reading this one wants `2>&1`, and the usage text
        // says so.
        eprintln!("version: {}", env!("SASSEMBLY_BUILD_COMMIT"));
        return ExitCode::SUCCESS;
    }
    // `--source-stamp`, the same sole-argument rule and the same stream: the
    // content stamp `t1_image`'s gate compares with its own (`W-381`).
    if argv.len() == 2 && argv[1] == "--source-stamp" {
        eprintln!("source: {}", env!("SASSEMBLY_SOURCE_STAMP"));
        return ExitCode::SUCCESS;
    }
    // ॥ `--events <log>` IS RECOGNISED ONLY AS THE FIRST ARGUMENT — W-371 ॥
    //
    // BEFORE THE IMAGE, never after it, for the reason the margin below gives and
    // `--version` above obeys: everything after the image is the program's. So
    // `yantra-run prog.elf --events x` hands `--events` and `x` to the program, and
    // only `yantra-run --events x prog.elf …` replays. The log's format is
    // `yantra::input::parse_event_log`'s: one word per line, one per wait.
    //
    // ॥ `--record-events <log>` IS ITS LIVE TWIN, IN THE SAME FIRST PLACE — W-375 ॥
    //
    // At each wait the host's time (ns since the Unix epoch, UTC — read by
    // `yantra::input::stamp_event_time`, the ONE clock read in this crate) is
    // delivered into the SASEVENT word and appended to the log as `t=<ns>`;
    // `--events` on that log replays the run without asking the clock. The two
    // flags are one slot: only one of them can be the first argument.
    //
    // ॥ `--smp <archive>` TAKES THE SAME FIRST PLACE — F-022 ॥
    //
    // With it the file is ALWAYS parsed as a Saṃpuṭa archive, so a wrong magic is S1; without
    // it a file is an archive only when its first four octets are "SMPT". An archive takes no
    // event log (refused below), so `--smp` sharing the one slot loses nothing.
    // ॥ `--files DIR` GRANTS THE PROGRAM A FILE ROOT — v1.0.1 ॥
    //
    // Recognised ONLY as the very first argument (before `--events`, `--record-events`,
    // `--smp` or the image), for the reason every flag here is: everything after the
    // image is the program's. Without it `patra_root` stays `None` and every file request
    // is REFUSED BY NAME (`Status::Refused`), as before. With it the program's file
    // window reads and writes inside DIR only (`patra::resolve`: `..`, absolute paths and
    // symlinks that leave DIR are refused). DIR is canonicalised HERE, and a missing or
    // non-directory DIR refuses at load (exit 1), before anything runs. File requests are
    // not events: they are not logged, so a replay (`--events`) re-reads DIR and needs
    // `--files DIR` again. An archive (`--smp` or "SMPT") is refused with `--files` (64).
    let (files_root, argv): (Option<std::path::PathBuf>, Vec<String>) =
        if argv.get(1).map(String::as_str) == Some("--files") {
            let Some(dir) = argv.get(2) else {
                eprintln!("usage: yantra-run --files DIR [other flags] <program.elf> [args...]");
                return ExitCode::FAILURE;
            };
            match std::fs::canonicalize(dir) {
                Ok(c) if c.is_dir() => {
                    let mut rest = vec![argv[0].clone()];
                    rest.extend_from_slice(&argv[3..]);
                    (Some(c), rest)
                }
                Ok(_) => {
                    eprintln!("files: {dir}: not a directory (refused at load; nothing ran)");
                    return ExitCode::FAILURE;
                }
                Err(e) => {
                    eprintln!("files: {dir}: {e} (refused at load; nothing ran)");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            (None, argv)
        };
    let (events_path, live, forced_smp, rest): (Option<&String>, bool, bool, &[String]) =
        match argv.get(1).map(String::as_str) {
            Some(flag @ ("--events" | "--record-events")) => match argv.get(2) {
                Some(log) => (Some(log), flag == "--record-events", false, &argv[3..]),
                None => {
                    eprintln!("usage: yantra-run {flag} <log> <program.elf> [args...]");
                    return ExitCode::FAILURE;
                }
            },
            Some("--smp") => (None, false, true, &argv[2..]),
            _ => (None, false, false, &argv[1..]),
        };
    // ॥ `--listen 127.0.0.1:PORT` IS ACCEPTED ONLY RIGHT AFTER `--record-events <log>` — W-377 ॥
    //
    // A live socket run that left no log could not be replayed (W-375's rule), so the
    // flag has no other place; anywhere else it is refused, never handed to the program
    // as its image. The address must PARSE as the IP 127.0.0.1 or ::1 exactly
    // (`[::1]:PORT`): 0.0.0.0, ::, a LAN address and `localhost` (no DNS lookup) are all
    // refused HERE, before anything is read or bound. Port 0 asks the system for one;
    // `socket: listening on …` says which.
    let (listen, rest): (Option<std::net::SocketAddr>, &[String]) =
        match (live, rest.first().map(String::as_str)) {
            (true, Some("--listen")) => match rest.get(1).map(|a| listen_address(a)) {
                Some(Ok(addr)) => (Some(addr), &rest[2..]),
                Some(Err(why)) => {
                    eprintln!("socket: refused — {why} (refused before any bind; nothing ran)");
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!(
                        "usage: yantra-run --record-events <log> --listen 127.0.0.1:PORT \
                         <program.elf> [args...]"
                    );
                    return ExitCode::FAILURE;
                }
            },
            (_, Some("--listen")) => {
                eprintln!(
                    "socket: refused — --listen is accepted only right after --record-events \
                     <log>: a live socket run that leaves no log cannot be replayed (W-377; \
                     refused before any bind, nothing ran)"
                );
                return ExitCode::FAILURE;
            }
            _ => (None, rest),
        };
    // ॥ `--net-*` (ADR-0047, PROPOSED) — THE FRAME DEVICE, EVERY FLAG EXPLICIT ॥
    //
    // Accepted only in the same first place, right after the log flag, as `--flag value` or
    // `--flag=value`. A live net run needs `--record-events` (an unlogged run cannot be
    // replayed) and a backend, `--net-peer` or `--net-tap`; there is no bare `--net`. A replay
    // (`--events` on a net log) takes none: the log's header carries the device and no
    // backend is consulted.
    let mut net_flags = NetFlags::default();
    let mut rest = rest;
    while let Some(first) = rest.first().map(String::as_str)
        && first.starts_with("--net-")
    {
        let (flag, inline) = match first.split_once('=') {
            Some((f, v)) => (f, Some(v.to_string())),
            None => (first, None),
        };
        let used = if inline.is_some() { 1 } else { 2 };
        if !live {
            eprintln!(
                "net: refused — {flag} is accepted only right after --record-events <log>: a \
                 replay (--events) rebuilds the device from the log's header and needs no \
                 backend, and an unlogged live run cannot be replayed (ADR-0047; nothing ran)"
            );
            return ExitCode::FAILURE;
        }
        if listen.is_some() {
            eprintln!("net: refused — --listen and --net-* in one run (ADR-0047; nothing ran)");
            return ExitCode::FAILURE;
        }
        let Some(value) = inline.or_else(|| rest.get(1).cloned()) else {
            eprintln!("net: refused — {flag} needs a value (nothing ran)");
            return ExitCode::FAILURE;
        };
        if let Err(why) = net_flags.take(flag, &value) {
            eprintln!("net: refused — {why} (nothing ran)");
            return ExitCode::FAILURE;
        }
        rest = &rest[used..];
    }
    if net_flags.peer.is_some() && net_flags.tap.is_some() {
        eprintln!(
            "net: refused — --net-peer and --net-tap are two backends; pick one (nothing ran)"
        );
        return ExitCode::FAILURE;
    }
    if !net_flags.any() && net_flags.touched() {
        eprintln!(
            "net: refused — --net-mac, --net-max-* and --net-timeout configure a device that \
             only --net-peer or --net-tap creates; there is no bare --net (ADR-0047; nothing ran)"
        );
        return ExitCode::FAILURE;
    }
    let Some(path) = rest.first().cloned() else {
        eprintln!(
            "usage: yantra-run [--events <log> | --record-events <log> [--listen 127.0.0.1:PORT]] \
             <program.elf> [args...]\n\
             \x20      yantra-run --files DIR [the forms above]   (grants a file root)\n\
             \x20      yantra-run [--smp] <archive.smp>\n\
             \x20      yantra-run --version   (only as the SOLE argument; prints on STDERR)\n\
             \x20      yantra-run --source-stamp   (the same; the content stamp, W-381)\n\
             \n\
             \x20--files DIR (v1.0.1), only as the FIRST argument, grants the program a file\n\
             \x20root: its file window (patra) reads and writes inside DIR and nowhere else\n\
             \x20(`..`, absolute paths and symlinks leaving DIR are refused). DIR must be an\n\
             \x20existing directory or the run refuses at load (exit 1). Without the flag\n\
             \x20every file request is refused by name. File requests are not logged as\n\
             \x20events: a replay needs --files DIR again, and a log RECORDED under --files\n\
             \x20says so in a `# yantra-run --files` header line, so replaying it without the\n\
             \x20flag refuses at load. Not allowed with an archive (64).\n\
             \x20THREAT MODEL: DIR must not be concurrently writable by an untrusted party\n\
             \x20(path swaps and hard links between check and open are not detected; on unix\n\
             \x20a write also refuses to follow a symlink at the file itself).\n\
             \n\
             \x20--events <log> REPLAYS a recorded event log (W-371): at each WAIT the\n\
             \x20next record is written into the word after the image's SASEVENT tag\n\
             \x20and the run resumes. One record per line, one per wait, decimal or\n\
             \x200x-hex; '#' lines and blank lines are skipped. A log shorter or longer\n\
             \x20than the waits REFUSES (exit 1). Without it a WAIT exits 75.\n\
             \n\
             \x20--record-events <log> runs LIVE (W-375): at each WAIT the host's time,\n\
             \x20in nanoseconds since the Unix epoch (UTC), is written into that word and\n\
             \x20appended to <log> (created or truncated) as a line t=<ns>. --events on\n\
             \x20that log replays the same values, count and output.\n\
             \n\
             \x20--listen 127.0.0.1:PORT (or [::1]:PORT), only right after\n\
             \x20--record-events <log>, serves ONE TCP client through the socket device\n\
             \x20(W-377): at the first WAIT it is accepted, and at every WAIT one read of\n\
             \x20up to 4096 octets is logged as s=<hex> (s=end when the peer closed) and\n\
             \x20delivered. Any other address is refused before any bind; port 0 picks\n\
             \x20one and says so. A wait after s=end, or idle for 30 s, exits 1. --events\n\
             \x20on that log (its first record is s=) replays the run without a network.\n\
             \n\
             \x20A THREADED image (W-376: it declares SASTHRDS) prints `threads: N` and\n\
             \x20needs one of the two: its log also holds @N lines, \"run thread N\", one\n\
             \x20at the start, at every wait and at every thread's end, a resuming\n\
             \x20thread's value after its @N. Live mode picks round-robin and records\n\
             \x20the picks. Without a log it exits 1 at load: which thread starts is a\n\
             \x20decision, and there is no default.\n\
             \n\
             \x20A SAMPUTA ARCHIVE (F-022, the archive project's research/sampputa-v0-spec.md) runs\n\
             \x20DIRECTLY: a file starting \"SMPT\", or ANY file after --smp (so a wrong\n\
             \x20magic is S1). S1..S7 are checked before any decoder instruction runs (a\n\
             \x20refusal exits 80 + k), then the embedded ELF runs with the payload as its\n\
             \x20input under the archive's input name. Refused with exit 64: arguments\n\
             \x20after the archive, --events/--record-events, and YANTRA_INPUT or\n\
             \x20YANTRA_INPUT_NAME set as well. The step ceiling is the header's cap,\n\
             \x20which YANTRA_STEPS may only lower. --smp on an unreadable file exits 66.\n\
             \n\
             \x20YANTRA_INPUT_ENTRY=\"<module> <routine>\" (with YANTRA_INPUT, instead of\n\
             \x20YANTRA_INPUT_NAME) sets the input name to module NUL routine, which the\n\
             \x20self-image rung takes as the image's entry. Exactly one ASCII space, both\n\
             \x20halves non-empty; malformed, not UTF-8, over 4096 octets, or set with\n\
             \x20YANTRA_INPUT_NAME, it is refused by name (exit 1). Without a NUL the name keeps today's entry.\n\
             \n\
             \x20YANTRA_VERDICT=<file> writes the halt as a machine-readable verdict\n\
             \x20(W-381): `finisher <value> <status|->`, `wait`, `steplimit`, `spin` or\n\
             \x20`fault ...`, then `steps <n>`. t1_image's differential gate reads it.\n\
             \n\
             \x20--version prints the commit this build STARTED FROM. It is HEAD,\n\
             \x20not the tree: a binary built from a dirty worktree prints a\n\
             \x20commit it is not, and an uncommitted edit does not even rerun\n\
             \x20the stamp. A figure from a dirty tree must say so in words."
        );
        return ExitCode::FAILURE;
    };
    // EVERYTHING AFTER THE IMAGE IS THE PROGRAM'S, not this binary's — the
    // execve convention, and the reason there are no flags here. A flag added
    // later would have to be rejected before this line or it would silently
    // stop reaching the program.
    let program_args: Vec<String> = rest.to_vec();
    // An unreadable file under `--smp` exits 66 (EX_NOINPUT, the reference reader's status);
    // the plain ELF path keeps its panic, unchanged.
    let file = if forced_smp {
        match std::fs::read(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("smp: {path}: cannot read the archive — {e} (nothing ran)");
                return ExitCode::from(66);
            }
        }
    } else {
        std::fs::read(&path).expect("read the ELF")
    };

    // ── A SAṂPUṬA ARCHIVE IS LAUNCHED DIRECTLY — `F-022`, reading (b) ────────────
    //
    // A file whose first four octets are "SMPT" is a self-decoding archive (the archive project
    // `research/sampputa-v0-spec.md`, normative there): it is VERIFIED here, S1..S7 in the
    // spec's order, before any decoder instruction runs, and a refusal exits `80 + k` as the
    // reference reader does. Then the embedded ELF is the image and the payload is the input
    // channel, exactly as `YANTRA_INPUT` would place it, under the archive's own input name.
    // NOTHING ELSE FROM THE FILE REACHES THE DECODER.
    //
    // The archive names its input, so `YANTRA_INPUT` or `YANTRA_INPUT_NAME` set as well is a
    // CONFLICT, refused (64, the reference's usage status) rather than overridden either way.
    // The step ceiling is the header's cap; `YANTRA_STEPS` may LOWER it, never raise it, and
    // here a value that is not decimal digits is refused rather than read as the default.
    //
    // WHAT MAKES A FILE AN ARCHIVE (the coordinator's ruling, 2026-10-05): `--smp`, which
    // ALWAYS parses, so a wrong magic is S1 (the archive project's `s1_magic.smp`, 8 flips of the sweep);
    // or, without it, the "SMPT" magic alone. A corrupted magic without the flag is then
    // refused by the ELF loader ("not an ELF", exit 1) — another cause, still before anything
    // runs — and a real ELF named `*.smp` runs as an ELF.
    //
    // WHAT REACHES THE DECODER: exactly what both reference readers give it — the payload as
    // YANTRA_INPUT, the name as YANTRA_INPUT_NAME, the step ceiling as YANTRA_STEPS, and
    // YANTRA_RAM if set. So arguments after the archive are REFUSED (64; neither reference
    // passes the decoder an argument, and the argument interface is handed only argv[0], this
    // archive's path, as for any image; Saṃpuṭa v0 §4, the archive project a23fc47, makes argv[0] the
    // reader's choice and forbids a decoder to depend on it), and so are `--events`/`--record-events` (64), which
    // feed data into the program. YANTRA_RAM, YANTRA_INPUT_TRACE, YANTRA_SCANOUT and
    // YANTRA_WATERMARK stay: they are HOST diagnostics and settings (RAM size, the encoder's
    // trace level, where the screen is dumped, the store high-water report), not input.
    let claims = forced_smp || yantra::smp::is_archive(&file);
    let (image, archive): (Vec<u8>, Option<Unpacked>) = if claims {
        if let Some(flag) = events_path.map(|_| if live { "--record-events" } else { "--events" }) {
            eprintln!(
                "smp: {path}: REFUSED — {flag} feeds data into the program, and nothing but the \
                 payload and the input name reaches an archive's decoder (nothing ran)"
            );
            return ExitCode::from(64);
        }
        if files_root.is_some() {
            eprintln!(
                "smp: {path}: REFUSED — --files grants a file root, and nothing but the payload \
                 and the input name reaches an archive's decoder (nothing ran)"
            );
            return ExitCode::from(64);
        }
        if rest.len() > 1 {
            eprintln!(
                "smp: {path}: REFUSED — {} argument(s) after the archive; nothing but the payload \
                 and the input name reaches an archive's decoder (nothing ran)",
                rest.len() - 1
            );
            return ExitCode::from(64);
        }
        if let Some(v) = ["YANTRA_INPUT", "YANTRA_INPUT_NAME", "YANTRA_INPUT_ENTRY"]
            .into_iter()
            .find(|v| std::env::var_os(v).is_some())
        {
            eprintln!(
                "smp: {path}: REFUSED — {v} is set, and a Saṃpuṭa archive carries its own input \
                 and input name; unset it (nothing ran)"
            );
            return ExitCode::from(64);
        }
        match yantra::smp::parse(&file, yantra::smp::DECODER_CAP, None) {
            Ok(a) => {
                eprintln!(
                    "smp: {path}: verified S1..S7 — decoder {} octets sha256 {}, payload kind {}, \
                     {} octets, input name {:?}, step cap {}",
                    a.decoder.len(),
                    a.decoder_sha
                        .iter()
                        .map(|x| format!("{x:02x}"))
                        .collect::<String>(),
                    // Any four octets (the spec, amended at the archive project `ea43cb5`): shown as text
                    // only when all four are printable ASCII, otherwise as hex.
                    if a.kind.iter().all(|c| (0x20..0x7f).contains(c)) {
                        String::from_utf8_lossy(&a.kind).into_owned()
                    } else {
                        format!("0x{:08x}", u32::from_be_bytes(a.kind))
                    },
                    a.payload.len(),
                    a.name,
                    a.step_cap
                );
                (
                    a.decoder.to_vec(),
                    Some(Unpacked {
                        payload: a.payload.to_vec(),
                        name: a.name.to_string(),
                        step_cap: a.step_cap,
                    }),
                )
            }
            Err(r) => {
                eprintln!(
                    "smp: {path}: REFUSED {r} — before any decoder instruction ran (exit {})",
                    r.exit_code()
                );
                return ExitCode::from(r.exit_code());
            }
        }
    } else {
        (file, None)
    };
    let archive_steps = match (&archive, std::env::var_os("YANTRA_STEPS")) {
        (None, _) => None,
        (Some(a), None) => Some(a.step_cap),
        (Some(a), Some(v)) => {
            let v = v.to_string_lossy();
            match v.parse::<u64>() {
                Ok(s) if !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()) => {
                    if s > a.step_cap {
                        eprintln!(
                            "smp: YANTRA_STEPS={s} is above the archive's step cap and is clamped \
                             to {} (a reader may lower the cap, never raise it)",
                            a.step_cap
                        );
                    }
                    Some(s.min(a.step_cap))
                }
                _ => {
                    eprintln!(
                        "smp: {path}: REFUSED — YANTRA_STEPS={v:?} is not a decimal number of \
                         at most 2^64 - 1 (nothing ran)"
                    );
                    return ExitCode::from(64);
                }
            }
        }
    };

    // F-020 (W-119): Refuse application images instead of booting and faulting.
    if let (Ok(tsv), Some(stem)) = (
        std::fs::read_to_string("spec/programs.tsv"),
        std::path::Path::new(&path).file_stem(),
    ) {
        let sas_name = format!("{}.sas", stem.to_string_lossy());
        let is_app = tsv.lines().any(|line| {
            let mut parts = line.split('\t');
            parts.next() == Some(&sas_name) && parts.next() == Some("app")
        });
        if is_app {
            eprintln!(
                "{path}: this image is an application and must be run by yantra-host, not yantra-run"
            );
            return ExitCode::FAILURE;
        }
    }

    // RAM IS SIZED FROM THE IMAGE, with the product's default as the floor: the
    // image's own segments say how far it reaches (its record region is a
    // PT_LOAD with a memsz), and a compiler's image reaches further than 20 MiB
    // once its runs grow. HEADROOM above the last segment is for the stack and
    // whatever the startup places past the loaded extent. `YANTRA_RAM=<octets>`
    // overrides both for a diagnostic run; `YANTRA_WATERMARK=1` prints how much
    // of RAM the run actually wrote, which is how the region constant is sized.
    // The sizing itself is `yantra::ram_for`, the one statement the census's
    // runner shares.
    let ram = std::env::var("YANTRA_RAM")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| yantra::ram_for(&image));
    let mut m = match yantra::Machine::load_elf(&image, ram) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    m.patra_root = files_root.clone();
    // THE SCREEN (`W-378`): `YANTRA_SCANOUT=<path>` is where the virtio-gpu device writes
    // scanout 0 as a PPM on each RESOURCE_FLUSH of the resource it shows — overwritten
    // each time, so the file is the last frame. Unset, nothing is written. An environment
    // variable and not a flag for the reason given below: every argument after the image
    // is the program's.
    m.virtio.gpu.dump = std::env::var_os("YANTRA_SCANOUT").map(std::path::PathBuf::from);
    // THE DEFERRED COMPLETION: `YANTRA_VIRTIO_DEFER=<n>` serves a notified virtio queue n
    // instructions after the QueueNotify store instead of inside it, as QEMU's device does
    // (see `virtio_mmio::VirtioMmio::defer`). Unset or 0 is the synchronous device every
    // count was taken with. Anything but decimal digits is refused, not read as 0: a run
    // asked to be asynchronous that silently was not would pass the driver it exists to fail.
    if let Some(v) = std::env::var_os("YANTRA_VIRTIO_DEFER") {
        let v = v.to_string_lossy();
        match v.parse::<u64>() {
            Ok(n) if !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()) => {
                m.virtio.defer = n;
                if n > 0 {
                    eprintln!(
                        "virtio: completions deferred by {n} instruction(s) (YANTRA_VIRTIO_DEFER)"
                    );
                }
            }
            _ => {
                eprintln!(
                    "virtio: YANTRA_VIRTIO_DEFER={v:?} is not a decimal number of at most 2^64 - 1 (nothing ran)"
                );
                return ExitCode::FAILURE;
            }
        }
    }
    // ── THE THREADS (`W-376`, ADR-0040 addendum) — FOUND AND MADE AT LOAD ────────
    //
    // `threads: N` is said WHENEVER the image carries `SASTHRDS`, before any check that
    // might refuse it: `tools/fixpoint.sh` refuses a Stage 2 whose log carries the line,
    // because a compiler image whose run depended on a schedule would make the fixpoint a
    // statement about the schedule. A threaded image given NO log is refused here, exit 1
    // and never the wait's 75: which thread starts is a decision, and there is no default.
    // The stacks are made HERE, BEFORE the input is injected below, so the injected slab
    // still sits above the store bound (`W-363`).
    // ── THE SOCKET (`W-377`) — CHOSEN AT LOAD ──────────────────────────────────
    //
    // A socket run is `--listen`, or a replay whose log's FIRST RECORD starts `s=`
    // (addendum §2). Decided here, before the threads, because a threaded image with a
    // socket is refused at load in this row (§3, owner ruling Q6).
    // A LOG RECORDED UNDER `--files` IS REPLAYED UNDER `--files` (v1.0.1): the program's
    // file reads are not in the log, so a replay with no root would run a different
    // program. Refused at load, by name, before anything runs.
    if !live
        && files_root.is_none()
        && let Some(log) = events_path
        && std::fs::read_to_string(log).is_ok_and(|t| t.lines().any(|l| l == FILES_GRANT_LINE))
    {
        eprintln!(
            "events: refused — the log {log:?} was recorded with --files (a file root was \
             granted) and this replay has none; pass --files DIR (nothing ran)"
        );
        return ExitCode::FAILURE;
    }
    let socket_mode = listen.is_some()
        || (!live
            && events_path
                .and_then(|p| std::fs::read_to_string(p).ok())
                .is_some_and(|t| socket::is_socket_log(&t)));
    let net_mode = net_flags.any()
        || (!live
            && events_path
                .and_then(|p| std::fs::read_to_string(p).ok())
                .is_some_and(|t| yantra::netdev::is_net_log(&t)));
    let tags = yantra::threads::scan(&m, &image);
    if let Some(n) = tags.declared_count(&m.mem) {
        eprintln!("threads: {n}");
    }
    let mut threads = match yantra::threads::discover_tags(&m, &tags) {
        Ok(None) => None,
        Err(e) => {
            eprintln!("threads: refused — {e}");
            return ExitCode::FAILURE;
        }
        Ok(Some(_)) if socket_mode => {
            eprintln!(
                "threads: refused — a threaded image with a socket (--listen or a socket log) \
                 is refused in this row: the readiness wake waits for a multi-connection row \
                 (W-377, addendum §3; refused at load, exit 1; nothing ran)"
            );
            return ExitCode::FAILURE;
        }
        Ok(Some(_)) if events_path.is_none() => {
            eprintln!(
                "threads: refused — a threaded image needs --events or --record-events: \
                 which thread runs first is a decision the log makes, and there is no default \
                 (refused at load, exit 1; nothing ran)"
            );
            return ExitCode::FAILURE;
        }
        Ok(Some(d)) => match yantra::threads::Threads::new(&mut m, d) {
            Ok(t) => {
                eprintln!(
                    "threads: {} contexts; {} host stacks of {} octets from RAM offset {:#x}, \
                     stores now stop at {}",
                    t.count(),
                    t.count() - 1,
                    yantra::threads::STACK_OCTETS,
                    t.stacks_at,
                    m.store_limit
                );
                Some(t)
            }
            Err(e) => {
                eprintln!("threads: refused — {e}");
                return ExitCode::FAILURE;
            }
        },
    };
    // A threaded log: `@N` records beside the values (`input::parse_thread_log`).
    let mut thread_log: Vec<yantra::input::ThreadRecord> = Vec::new();
    // THE EVENT CHANNEL (`W-371`, ADR-0040 Option C R2) — checked AT LOAD, before one
    // instruction runs: a log the runner cannot read, or an image with no `SASEVENT`
    // word to deliver into, is refused here rather than discovered at the first wait.
    // A socket run's SASEVENT word and, in replay, its records (`W-377`).
    let mut socket_tag: Option<usize> = None;
    let mut net_tag: Option<usize> = None;
    let mut net_log: Vec<yantra::netdev::NetRecord> = Vec::new();
    let mut net_sent: Vec<u8> = Vec::new();
    let mut net_delivered = 0usize;
    let mut socket_log: Vec<SockRecord> = Vec::new();
    let events: Option<(usize, Vec<u64>)> = match events_path {
        None => None,
        // A SOCKET RUN: the log is a socket log (`socket::parse_socket_log`), and the image
        // must declare SASEVENT — the slot is given each record's octet count.
        Some(log) if socket_mode => {
            if !live {
                match std::fs::read_to_string(log)
                    .map_err(|e| e.to_string())
                    .and_then(|t| socket::parse_socket_log(&t))
                {
                    Ok(r) => socket_log = r,
                    Err(e) => {
                        eprintln!("events: refused — the socket log {log:?}: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            match yantra::input::find_event_slot(&m.mem) {
                Ok(tag) => {
                    if live {
                        eprintln!(
                            "events: LIVE, recording a socket log to {log:?}, SASEVENT tag at \
                             {tag:#x}"
                        );
                    } else {
                        eprintln!(
                            "events: {} socket records from {log:?}, SASEVENT tag at {tag:#x}",
                            socket_log.len()
                        );
                    }
                    socket_tag = Some(tag);
                }
                Err(e) => {
                    eprintln!("events: refused — a socket run needs the event slot: {e}");
                    return ExitCode::FAILURE;
                }
            }
            None
        }
        // A NET RUN (ADR-0047): the log is a net log. A threaded image delivers into the
        // word `discover` found; a single-thread image must declare SASEVENT.
        Some(log) if net_mode => {
            let mut config = net_flags.config();
            let mut scheduled = None;
            if !live {
                match std::fs::read_to_string(log)
                    .map_err(|e| e.to_string())
                    .and_then(|t| yantra::netdev::parse_net_log(&t))
                {
                    Ok(l) => {
                        config = l.config;
                        net_log = l.records;
                        if let Some(e) = net_defer_env()
                            && e != config.defer
                        {
                            eprintln!(
                                "events: refused — YANTRA_VIRTIO_DEFER={e} but the net log {log:?} \
                                 was recorded with defer={} (backend={}): a replay under a \
                                 different completion delay is a different run (ADR-0047; nothing ran)",
                                config.defer,
                                config.backend.word()
                            );
                            return ExitCode::FAILURE;
                        }
                        scheduled = Some(l.schedule);
                    }
                    Err(e) => {
                        eprintln!("events: refused — the net log {log:?}: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            if let Some(sched) = scheduled {
                if sched.is_some() != threads.is_some() {
                    eprintln!(
                        "events: refused — the net log {log:?} is {} but the image is {} \
                         (ADR-0047; nothing ran)",
                        if sched.is_some() {
                            "a THREADED log (it has @N lines)"
                        } else {
                            "a single-thread log"
                        },
                        if threads.is_some() {
                            "threaded"
                        } else {
                            "not threaded"
                        }
                    );
                    return ExitCode::FAILURE;
                }
                if let Some(sc) = sched {
                    thread_log = sc;
                }
            }
            let found = match &threads {
                Some(t) => Ok(t.declared.event_tag),
                None => yantra::input::find_event_slot(&m.mem),
            };
            match found {
                Ok(tag) => {
                    if live {
                        eprintln!(
                            "events: LIVE, recording a net log to {log:?}, SASEVENT tag at {tag:#x}"
                        );
                    } else {
                        eprintln!(
                            "events: {} net records from {log:?}, SASEVENT tag at {tag:#x}",
                            net_log.len()
                        );
                    }
                    net_tag = Some(tag);
                    m.net = Some(yantra::netdev::VirtioNet::new(config));
                }
                Err(e) => {
                    eprintln!("events: refused — a net run needs the event slot: {e}");
                    return ExitCode::FAILURE;
                }
            }
            None
        }
        // THREADED: `discover` already found the SASEVENT word; the log is a thread log.
        Some(log) if threads.is_some() => {
            let tag = threads.as_ref().map_or(0, |t| t.declared.event_tag);
            if live {
                eprintln!(
                    "events: LIVE, recording a thread log to {log:?}, SASEVENT tag at {tag:#x}"
                );
            } else {
                match std::fs::read_to_string(log)
                    .map_err(|e| e.to_string())
                    .and_then(|t| yantra::input::parse_thread_log(&t))
                {
                    Ok(r) => thread_log = r,
                    Err(e) => {
                        eprintln!("events: refused — the log {log:?}: {e}");
                        return ExitCode::FAILURE;
                    }
                }
                eprintln!(
                    "events: {} thread-log records from {log:?}, SASEVENT tag at {tag:#x}",
                    thread_log.len()
                );
            }
            None
        }
        Some(log) => {
            // LIVE MODE READS NO LOG — it writes one, below, once the run starts.
            let records = if live {
                Vec::new()
            } else {
                match std::fs::read_to_string(log)
                    .map_err(|e| e.to_string())
                    .and_then(|t| yantra::input::parse_event_log(&t))
                {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("events: refused — the log {log:?}: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            };
            match yantra::input::find_event_slot(&m.mem) {
                Ok(tag) if live => {
                    eprintln!("events: LIVE, recording to {log:?}, SASEVENT tag at {tag:#x}");
                    Some((tag, records))
                }
                Ok(tag) => {
                    eprintln!(
                        "events: {} records from {log:?}, SASEVENT tag at {tag:#x}",
                        records.len()
                    );
                    Some((tag, records))
                }
                Err(e) => {
                    eprintln!("events: refused — {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    };
    // THE LISTENER IS BOUND AT LOAD, before the first instruction and after every load
    // refusal above (`W-377`, addendum §3). The device is made with it — or, in replay,
    // here as well — so the program's first read is UNREAD, not "no device".
    let mut listener = match (listen, socket_tag) {
        (Some(addr), Some(_)) => match std::net::TcpListener::bind(addr) {
            Ok(l) => {
                match l.local_addr() {
                    Ok(a) => eprintln!("socket: listening on {a}"),
                    Err(e) => eprintln!("socket: listening on {addr} (local address: {e})"),
                }
                Some(l)
            }
            Err(e) => {
                eprintln!("socket: refused — cannot listen on {addr}: {e} (nothing ran)");
                return ExitCode::FAILURE;
            }
        },
        _ => None,
    };
    if socket_tag.is_some() {
        m.socket = Some(socket::Socket::new());
    }
    // THE NET BACKEND is opened at load, so a refusal (a platform without it, a missing
    // privilege) stops the run before the first instruction.
    let mut net_backend: Option<Box<dyn yantra::netdev::FrameBackend>> = None;
    if net_tag.is_some() && live {
        match net_flags.open() {
            Ok(b) => net_backend = Some(b),
            Err(why) => {
                eprintln!("net: refused — {why} (nothing ran)");
                return ExitCode::FAILURE;
            }
        }
    }
    let mut net_threads: Option<yantra::netdev::NetThreads> = match (net_tag, threads.is_some()) {
        (Some(_), true) if live => net_backend.take().map(yantra::netdev::NetThreads::live),
        (Some(_), true) => Some(yantra::netdev::NetThreads::replay(std::mem::take(
            &mut net_log,
        ))),
        _ => None,
    };
    // The live log is CREATED before the first instruction, so a run with no wait still
    // leaves a log — a header and no record — saying the program was given no time.
    let mut live_log = match (live, events_path) {
        (true, Some(log)) => match std::fs::File::create(log) {
            Ok(f) => Some(std::io::BufWriter::new(f)),
            Err(e) => {
                eprintln!("events: refused — cannot create the live log {log:?}: {e}");
                return ExitCode::FAILURE;
            }
        },
        _ => None,
    };
    // The grant is recorded in the header of a non-socket log; a socket log writes it
    // itself after its own header (`serve_live`), because that header must stay first.
    if let Some(log) = live_log.as_mut()
        && files_root.is_some()
        && !socket_mode
    {
        use std::io::Write;
        if let Err(e) = writeln!(log, "{FILES_GRANT_LINE}").and_then(|()| log.flush()) {
            eprintln!("events: refused — cannot write the live log header: {e}");
            return ExitCode::FAILURE;
        }
    }
    // THE INPUT CHANNEL (`yantra::input`) — a file placed in RAM before the
    // first instruction, for a program that declares the interface. No new
    // `ecall`: the machine still answers exactly two. `YANTRA_INPUT` is the file,
    // `YANTRA_INPUT_NAME` its module name (passed, not parsed, so the host and
    // the interpreter's `--input-name` cannot derive it two different ways),
    // `YANTRA_INPUT_TRACE=<level>` sets the program's trace level (1 markers, 2 text).
    //
    // A REFUSAL STOPS THE RUN. A program run without its input would compile
    // an empty source and halt looking like a verdict about that source.
    // Where the injected slab begins, so the run can be checked afterwards for
    // an allocator that grew into it — see the collision check below.
    let mut slab_at: Option<usize> = None;
    // (text, module name, how the two are named in the report line). An archive's are its
    // payload and its name (`F-022`); otherwise the environment's, as before.
    let input: Option<Input> = match &archive {
        Some(a) => Some((
            std::borrow::Cow::Borrowed(a.payload.as_slice()),
            a.name.as_bytes().to_vec(),
            format!("the payload of {path}"),
            format!("{:?}", a.name),
        )),
        None => match std::env::var_os("YANTRA_INPUT") {
            None => None,
            Some(input) => {
                let text = match std::fs::read(&input) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("input: YANTRA_INPUT {input:?}: {e}");
                        return ExitCode::FAILURE;
                    }
                };
                // YANTRA_INPUT_ENTRY="<module> <routine>" is the host's way to give the
                // self-image rung (`स्वपरीक्षास्वप्रतिबिम्बम्`) its entry: the input name
                // becomes `module NUL routine`, which an environment variable cannot hold.
                // Exactly one ASCII space, both halves non-empty and free of whitespace and
                // NUL; refused by name when malformed or when YANTRA_INPUT_NAME is also set.
                let (name_bytes, name_label) = match std::env::var_os("YANTRA_INPUT_ENTRY") {
                    Some(entry) => {
                        if std::env::var_os("YANTRA_INPUT_NAME").is_some() {
                            eprintln!(
                                "input: YANTRA_INPUT_ENTRY and YANTRA_INPUT_NAME are both set — the entry \
                                 form builds the name itself (module NUL routine); set one"
                            );
                            return ExitCode::FAILURE;
                        }
                        if entry.len() > 4096 {
                            eprintln!(
                                "input: YANTRA_INPUT_ENTRY is {} octets, over the 4096 limit",
                                entry.len()
                            );
                            return ExitCode::FAILURE;
                        }
                        let Ok(e) = entry.clone().into_string() else {
                            eprintln!(
                                "input: YANTRA_INPUT_ENTRY {entry:?} is not UTF-8 — module and routine names are UTF-8"
                            );
                            return ExitCode::FAILURE;
                        };
                        let ok = match e.split_once(' ') {
                            Some((m, r)) => [m, r].iter().all(|h| {
                                !h.is_empty()
                                    && !h.contains(char::is_whitespace)
                                    && !h.contains('\0')
                            }),
                            None => false,
                        };
                        if !ok {
                            eprintln!(
                                "input: YANTRA_INPUT_ENTRY {e:?} is malformed — it must be \
                                 \"<module> <routine>\": exactly one ASCII space, both halves non-empty \
                                 and without whitespace"
                            );
                            return ExitCode::FAILURE;
                        }
                        let (m, r) = e.split_once(' ').unwrap_or_default();
                        let mut n = m.as_bytes().to_vec();
                        n.push(0);
                        n.extend_from_slice(r.as_bytes());
                        (n, format!("{m:?} NUL {r:?}"))
                    }
                    None => {
                        let Some(name) = std::env::var_os("YANTRA_INPUT_NAME") else {
                            eprintln!(
                                "input: YANTRA_INPUT needs YANTRA_INPUT_NAME (or YANTRA_INPUT_ENTRY) — the module name is passed, never guessed"
                            );
                            return ExitCode::FAILURE;
                        };
                        (
                            name.to_string_lossy().as_bytes().to_vec(),
                            format!("{name:?}"),
                        )
                    }
                };
                Some((
                    std::borrow::Cow::Owned(text),
                    name_bytes,
                    format!("{input:?}"),
                    name_label,
                ))
            }
        },
    };
    if let Some((text, name, input, name_label)) = input {
        // A LEVEL, not a switch: 1 = the encoder's marker trace, 2 = each
        // module's generated Sassembly text, for diffing the two engines line
        // by line. Anything unparsable is 0 — no trace — rather than a guess.
        let trace: u64 = std::env::var("YANTRA_INPUT_TRACE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let base = m.base;
        match yantra::input::inject(&mut m.mem, base, &text, &name, trace) {
            Ok(r) => {
                eprintln!(
                    "input: {} octets of {} as module {}, tags at {:x?}, RAM {} -> {}",
                    text.len(),
                    input,
                    name_label,
                    r.tags_at,
                    r.old_top,
                    r.new_top
                );
                slab_at = Some(r.old_top);
            }
            Err(e) => {
                eprintln!("input: refused — {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    let mut sink = Sink {
        out: Vec::new(),
        high_water: 0,
        counter_reads: 0,
        sent: Vec::new(),
        stream: None,
        live_tx: listener.is_some(),
        send_failed: false,
        sent_without_client: false,
    };
    // YANTRA_STEPS=<n>: the step ceiling, for a compile that legitimately outruns
    // the default (a native compile of a module with routines does). An archive's ceiling
    // was settled above (its header's cap, or a lower YANTRA_STEPS) and replaces this one;
    // the first binding stays one statement because `t1_transcriptions.rs` reads its fallback.
    let steps = std::env::var("YANTRA_STEPS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_STEPS);
    let steps = archive_steps.unwrap_or(steps);

    // ── THE PROGRAM'S ARGUMENTS ──────────────────────────────────────────────
    //
    // **A REFUSAL HERE IS NOT AN ERROR.** Most images do not declare the
    // argument globals and never will; the whole corpus is one of them. So a
    // missing interface is reported at one line and the run continues, where a
    // missing INPUT interface above is fatal — the difference being that
    // `--input` was ASKED FOR and arguments are always present.
    //
    // The refusal is still PRINTED rather than swallowed: a program that
    // declared the globals and got nothing would otherwise see an empty run and
    // conclude it was invoked with no arguments, which is a different fact.
    let base = m.base;
    let argv: Vec<&[u8]> = program_args.iter().map(|a| a.as_bytes()).collect();
    match yantra::input::inject_arguments(&mut m.mem, base, &argv) {
        Ok(r) => eprintln!(
            "args: {} handed over, tags at {:x?}, run at {:#x}",
            r.argc, r.tags_at, r.argv_ptr
        ),
        Err(e) => eprintln!("args: not handed over — {e}"),
    }

    // WITH A LOG, A WAIT IS ANSWERED; WITHOUT ONE, IT ENDS THE RUN (exit 75 below). The
    // step ceiling covers the WHOLE replay, every segment between waits together.
    //
    // A LOG THAT DOES NOT MATCH THE WAITS IS REFUSED, BOTH WAYS. Short: the program is
    // NOT resumed — it would read whatever its event word last held — and the refusal
    // names the wait index (from 0) and its store. Long: REFUSED, not warned, after the
    // run — the program's output is still written, since it is what the program did, but
    // the exit is 1, because a replay that left records unconsumed reproduced some run
    // other than the one the log recorded, and a warning a script never reads would let
    // that pass as a replay.
    let mut refusal: Option<String> = None;
    // A threaded run that ended at a halt the end rule counts as failure even where the
    // halt alone would not (a thread spinning on `jal x0, .` is not a thread that ended).
    let mut thread_failure: Option<String> = None;
    // Socket records delivered, one per wait (`W-377`).
    let mut socket_delivered = 0usize;
    let halt = match (&events, threads.as_mut()) {
        // ── A SINGLE-THREAD NET RUN (ADR-0047) ──
        (_, None) if net_tag.is_some() => {
            let tag = net_tag.expect("guarded");
            match (net_backend.as_mut(), live_log.as_mut()) {
                (Some(backend), Some(log)) => {
                    let r = yantra::netdev::record_live_net(
                        &mut m,
                        tag,
                        steps,
                        &mut sink,
                        log,
                        backend.as_mut(),
                        &mut net_sent,
                    );
                    match r {
                        Ok((halt, delivered)) => {
                            net_delivered = delivered;
                            eprintln!(
                                "events: {delivered} net records logged and delivered, one per wait"
                            );
                            halt
                        }
                        Err(why) => {
                            refusal = Some(why);
                            Halt::Wait { pc: m.pc }
                        }
                    }
                }
                _ => {
                    let mut tx = |f: &[u8]| yantra::netdev::note_sent(&mut net_sent, f);
                    match yantra::netdev::replay_net(
                        &mut m, tag, &net_log, steps, &mut sink, &mut tx,
                    ) {
                        yantra::netdev::NetReplayed::Halted { halt, delivered } => {
                            net_delivered = delivered;
                            let ended =
                                matches!(halt, Halt::Finisher { .. } | Halt::Shutdown { .. });
                            if delivered < net_log.len() && ended {
                                refusal = Some(format!(
                                    "the net log is LONGER than the run: {delivered} of {} records \
                                     delivered; the program halted {halt:?} without waiting again, \
                                     so this log does not describe this run",
                                    net_log.len()
                                ));
                            } else if delivered < net_log.len() {
                                refusal = Some(format!(
                                    "the run stopped at {halt:?} with {} of {} net records unconsumed",
                                    net_log.len() - delivered,
                                    net_log.len()
                                ));
                            } else {
                                eprintln!(
                                    "events: {delivered} of {} net records delivered, one per wait",
                                    net_log.len()
                                );
                            }
                            halt
                        }
                        yantra::netdev::NetReplayed::Short { index, pc } => {
                            net_delivered = index;
                            refusal = Some(format!(
                                "the net log is SHORTER than the run: wait index {index} (the \
                                 store at pc {pc:#x}) has no record; the log holds {} — the \
                                 program was NOT resumed",
                                net_log.len()
                            ));
                            Halt::Wait { pc }
                        }
                        yantra::netdev::NetReplayed::Refused { index, pc, why } => {
                            net_delivered = index;
                            refusal = Some(format!("net record at wait index {index}: {why}"));
                            Halt::Wait { pc }
                        }
                    }
                }
            }
        }
        // ── A SOCKET RUN (`W-377`) — threads were refused at load, so there are none ──
        (_, None) if socket_tag.is_some() => {
            let tag = socket_tag.expect("guarded");
            match (listener.take(), live_log.as_mut()) {
                (Some(l), Some(log)) => {
                    let (end, delivered) =
                        serve_live(&mut m, tag, l, steps, &mut sink, log, files_root.is_some());
                    socket_delivered = delivered;
                    match end {
                        Ok(halt) => {
                            eprintln!(
                                "events: {delivered} socket records read, logged and delivered, \
                                 one per wait"
                            );
                            halt
                        }
                        Err((why, halt)) => {
                            refusal = Some(why);
                            halt
                        }
                    }
                }
                _ => match socket::replay_socket(&mut m, tag, &socket_log, steps, &mut sink) {
                    socket::SockReplayed::Halted { halt, delivered } => {
                        socket_delivered = delivered;
                        let ended = matches!(halt, Halt::Finisher { .. } | Halt::Shutdown { .. });
                        if delivered < socket_log.len() && ended {
                            refusal = Some(format!(
                                "the socket log is LONGER than the run: {delivered} of {} records \
                                 delivered, {} unconsumed; the program halted {halt:?} without \
                                 waiting again, so this log does not describe this run",
                                socket_log.len(),
                                socket_log.len() - delivered
                            ));
                        } else if delivered < socket_log.len() {
                            refusal = Some(format!(
                                "the run stopped at {halt:?} with {} of {} socket records \
                                 unconsumed; the log was not exhausted because the program did \
                                 not finish",
                                socket_log.len() - delivered,
                                socket_log.len()
                            ));
                        } else {
                            eprintln!(
                                "events: {delivered} of {} socket records delivered, one per wait",
                                socket_log.len()
                            );
                        }
                        halt
                    }
                    socket::SockReplayed::Short { index, pc } => {
                        socket_delivered = index;
                        refusal = Some(format!(
                            "the socket log is SHORTER than the run: wait index {index} (the \
                             store at pc {pc:#x}) has no record; the log holds {} — the program \
                             was NOT resumed, rather than let it read octets nobody delivered",
                            socket_log.len()
                        ));
                        Halt::Wait { pc }
                    }
                    socket::SockReplayed::OverCap { index, pc, why } => {
                        socket_delivered = index;
                        refusal = Some(why);
                        Halt::Wait { pc }
                    }
                    socket::SockReplayed::AfterEnd { index, pc } => {
                        socket_delivered = index;
                        refusal = Some(after_end(index, pc));
                        Halt::Wait { pc }
                    }
                },
            }
        }
        (_, Some(t)) => {
            let end = match live_log.as_mut() {
                Some(log) => {
                    let r = match net_threads.as_mut() {
                        Some(nt) => yantra::threads::record_live_threads_net(
                            &mut m, t, steps, &mut sink, log, nt,
                        ),
                        None => {
                            yantra::threads::record_live_threads(&mut m, t, steps, &mut sink, log)
                        }
                    };
                    match r {
                        Ok(end) => end,
                        Err(e) => {
                            refusal = Some(e);
                            yantra::threads::ThreadsEnd::Halted {
                                thread: t.current.unwrap_or(0),
                                halt: yantra::Halt::Wait { pc: m.pc },
                            }
                        }
                    }
                }
                None => match net_threads.as_mut() {
                    Some(nt) => yantra::threads::replay_threads_net(
                        &mut m,
                        t,
                        &thread_log,
                        steps,
                        &mut sink,
                        nt,
                    ),
                    None => {
                        yantra::threads::replay_threads(&mut m, t, &thread_log, steps, &mut sink)
                    }
                },
            };
            if let Some(nt) = net_threads.as_ref() {
                net_sent.clone_from(&nt.sent);
                net_delivered = nt.next;
            }
            for e in &t.ends {
                eprintln!(
                    "threads: thread {}: ended Finisher {{ value: {:#x}, status: {:?} }} at {} executed \
                     instructions, {} of them its own",
                    e.thread, e.value, e.status, e.at, t.retired[e.thread as usize]
                );
            }
            let finisher = |e: &yantra::threads::End| yantra::Halt::Finisher {
                value: e.value,
                status: e.status,
            };
            let last = || {
                t.ends
                    .last()
                    .map_or(yantra::Halt::Wait { pc: m.pc }, finisher)
            };
            match end {
                yantra::threads::ThreadsEnd::Ended => {
                    if !live {
                        eprintln!(
                            "events: {} of {} thread-log records consumed",
                            thread_log.len(),
                            thread_log.len()
                        );
                    }
                    match yantra::threads::first_failure(t) {
                        // THE END RULE: every thread ended, and the first non-zero status,
                        // in the order they ended, is the run's.
                        Some(f) => {
                            eprintln!(
                                "threads: thread {} ended with status {} — the first non-zero; \
                                 every thread ran to its end and the run FAILS",
                                f.thread,
                                f.status.map_or_else(
                                    || format!("undefined (finisher value {:#x})", f.value),
                                    |s| s.to_string()
                                )
                            );
                            finisher(f)
                        }
                        None => last(),
                    }
                }
                yantra::threads::ThreadsEnd::Halted { thread, halt } => {
                    let why = format!(
                        "thread {thread} halted {halt:?}, not at its finisher — that ends the \
                         WHOLE run, because RAM is shared"
                    );
                    eprintln!("threads: {why}");
                    if matches!(halt, yantra::Halt::SpinForever { .. }) {
                        thread_failure = Some(why);
                    }
                    halt
                }
                // `BeyondRam`, FOR A THREAD (W-376 review fix 1): a store above the RAM the
                // image was given and outside the thread's own stack. Always a failure,
                // whatever the segment's own halt was.
                yantra::threads::ThreadsEnd::BeyondOwnStack {
                    thread,
                    addr,
                    ram,
                    stack,
                    halt,
                } => {
                    let own = stack.map_or_else(
                        || "it has no host stack (thread 0's is the image's own)".to_string(),
                        |(lo, hi)| format!("its own stack is {lo:#x}..{hi:#x}"),
                    );
                    let why = format!(
                        "thread {thread} stored at {addr:#x}, at or above the {ram} octets of \
                         RAM the image was given and outside its own stack — {own}; a heap past \
                         RAM or a stack overflowing into another thread's. The WHOLE run ends \
                         (the segment ended {halt:?})"
                    );
                    eprintln!("threads: BeyondOwnStack — {why}");
                    thread_failure = Some(why);
                    halt
                }
                yantra::threads::ThreadsEnd::Short { index, thread } => {
                    refusal = Some(format!(
                        "the log is SHORTER than the run: record {index} is due at the decision \
                         after {} and the log holds {} — nothing was resumed, rather than run a \
                         thread nobody scheduled or hand it a value nobody delivered",
                        thread.map_or_else(|| "the start".to_string(), |k| format!("thread {k}")),
                        thread_log.len()
                    ));
                    yantra::Halt::Wait { pc: m.pc }
                }
                yantra::threads::ThreadsEnd::Long { consumed, len } => {
                    refusal = Some(format!(
                        "the log is LONGER than the run: every thread ended after {consumed} of \
                         {len} records, {} unconsumed, so this log does not describe this run",
                        len - consumed
                    ));
                    last()
                }
                yantra::threads::ThreadsEnd::Refused { index, why } => {
                    refusal = Some(format!("record {index} does not fit the run — {why}"));
                    yantra::Halt::Wait { pc: m.pc }
                }
            }
        }
        (None, None) => m.run(steps, &mut sink),
        // LIVE: every wait is answered with the stamped time, so there is no short or long
        // log to refuse; an unwritable log is refused like any other event failure.
        (Some((tag, _)), None) if live => {
            let log = live_log.as_mut().expect("created above with the live flag");
            match yantra::input::record_live(&mut m, *tag, steps, &mut sink, log) {
                Ok((halt, delivered)) => {
                    eprintln!("events: {delivered} clock records stamped and logged, one per wait");
                    halt
                }
                Err(e) => {
                    refusal = Some(e);
                    yantra::Halt::Wait { pc: m.pc }
                }
            }
        }
        (Some((tag, log)), None) => {
            match yantra::input::replay(&mut m, *tag, log, steps, &mut sink) {
                yantra::input::Replayed::Halted { halt, delivered } => {
                    // "LONGER" only when the program really ENDED (a finisher or a
                    // shutdown): a step limit or a fault partway through a correct log
                    // is a stopped run, not a long log (found in review, Naad lane).
                    let ended = matches!(
                        halt,
                        yantra::Halt::Finisher { .. } | yantra::Halt::Shutdown { .. }
                    );
                    if delivered < log.len() && ended {
                        refusal = Some(format!(
                            "the log is LONGER than the run: {delivered} of {} \
                         records delivered, {} unconsumed; the program halted {halt:?} without \
                         waiting again, so this log does not describe this run",
                            log.len(),
                            log.len() - delivered
                        ));
                    } else if delivered < log.len() {
                        refusal = Some(format!(
                            "the run stopped at {halt:?} with {} of {} records unconsumed; \
                         the log was not exhausted because the program did not finish",
                            log.len() - delivered,
                            log.len()
                        ));
                    } else {
                        eprintln!(
                            "events: {delivered} of {} delivered, one per wait",
                            log.len()
                        );
                    }
                    halt
                }
                yantra::input::Replayed::Short { index, pc } => {
                    refusal = Some(format!(
                        "the log is SHORTER than the run: wait index {index} (the \
                     store at pc {pc:#x}) has no record; the log holds {} — the program was NOT \
                     resumed, rather than let it read a word nobody delivered",
                        log.len()
                    ));
                    yantra::Halt::Wait { pc }
                }
            }
        }
    };
    use std::io::Write;
    std::io::stdout().write_all(&sink.out).unwrap_or(());
    eprintln!("halt: {halt:?}");
    // ॥ `YANTRA_VERDICT=<file>` — W-381 ॥ THE HALT, FOR A PROGRAM TO READ.
    // `t1_image`'s differential gate runs this binary as a child and compares
    // its result with the interpreted predict. stdout is the payload and stays
    // so; the halt line above is prose for a person. So the verdict goes to a
    // FILE the caller names, in a form with no Debug text to parse:
    //   `finisher <value> <status>|-` · `wait` · `steplimit` · `spin` ·
    //   `fault <halt, Debug>`, then `steps <n>`, then `refused <why>` when an
    //   event log or a threaded run was refused (it outranks the halt, below).
    // Written only on this path, where a program RAN; a run refused before the
    // first instruction leaves no file, and the caller reads its absence as
    // "no verdict" — never as a pass.
    if let Some(path) = std::env::var_os("YANTRA_VERDICT") {
        let first = match &halt {
            Halt::Finisher { value, status } => format!(
                "finisher {value} {}",
                status.map_or_else(|| "-".to_string(), |s| s.to_string())
            ),
            Halt::Wait { .. } => "wait".to_string(),
            Halt::StepLimit { .. } => "steplimit".to_string(),
            Halt::SpinForever { .. } => "spin".to_string(),
            other => format!("fault {other:?}"),
        };
        let mut text = format!("{first}\nsteps {}\n", m.time);
        if let Some(why) = refusal.as_ref().or(thread_failure.as_ref()) {
            text.push_str(&format!("refused {}\n", why.replace('\n', " ")));
        }
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("verdict: {}: {e}", std::path::Path::new(&path).display());
            return ExitCode::FAILURE;
        }
    }
    // **EXECUTED INSTRUCTIONS — the ratified benchmark metric (`T-102`).**
    //
    // `Machine::time` counts one tick per instruction the hart BEGINS, and it
    // has always been kept; nothing printed it, so the one number a benchmark
    // needs was unreachable from outside the process.
    //
    // IT IS NOT A HARDWARE CYCLE COUNT AND THE LABEL SAYS SO. This machine has
    // no `rdcycle`/`rdtime` — the honoured CSRs are the supervisor set — and a
    // step-counting interpreter's ticks are not a CPU's cycles. What they ARE
    // is exactly reproducible across hosts, schedulers and clock speeds, which
    // is the property Sassembly claims and a wall clock cannot give. Quoting
    // this as "cycles" would be quoting a different measurement.
    eprintln!("steps: {} executed instructions", m.time);
    // `W-374`/`W-372`: the counter is DIAGNOSTIC, and a read is not a halt, so nothing
    // above would show one. Said only when it happened — `tools/fixpoint.sh` refuses a
    // Stage 2 whose log carries this line, because a compiler image that read its own
    // count could make Stage 2 differ from Stage 1 by self-reference (ADR-0040).
    if sink.counter_reads > 0 {
        eprintln!(
            "counter: {} reads of the retired-instruction counter (W-374, diagnostic only)",
            sink.counter_reads
        );
    }
    // `W-377`: THE SOCKET'S ACCOUNT, in both modes — the falsifier compares it, stdout and
    // `steps:` between a live run and its replays. A `socket:` line is ALSO said when a
    // run with no device touched the window: `tools/fixpoint.sh` refuses a Stage 2 whose
    // log carries one, naming the cause the missing finisher alone would not.
    if socket_tag.is_some() {
        eprintln!(
            "{}",
            socket::report_line(m.socket.as_ref(), socket_delivered, &sink.sent)
        );
    } else if let Halt::Device { addr, why, .. } = &halt
        && (yantra::SOCK..yantra::SOCK + socket::SOCK_LEN).contains(addr)
    {
        eprintln!("socket: touched at {addr:#x} with no socket device — {why}");
    }
    if net_tag.is_some() {
        eprintln!(
            "{}",
            yantra::netdev::report_line(m.net.as_ref(), net_delivered, &net_sent)
        );
    } else if let Halt::Device { addr, why, .. } = &halt
        && yantra::virtio_mmio::slot_of(*addr)
            .is_some_and(|(slot, _)| slot == yantra::netdev::NET_SLOT)
    {
        eprintln!("net: touched at {addr:#x} with no net device — {why}");
    }
    if std::env::var_os("YANTRA_WATERMARK").is_some() {
        eprintln!("ram: high water {} of {} octets", sink.high_water, ram);
    }
    // THE COLLISION CHECK, ALWAYS ON WHEN THERE IS AN INPUT. The record
    // allocator is a bump cursor with no upper bound, so it can grow past the
    // image, through the headroom, and INTO the injected slab — measured
    // 2026-09-21 on the first native self-image build, which reached 559,504,480
    // octets over a 555,254,376-octet old top and then ran off RAM. An
    // allocator that overwrites its own input mid-read compiles corrupted
    // source and halts looking like an ordinary result, so the one number that
    // can tell is said out loud: the store high-water mark against the slab.
    if let Some(at) = slab_at
        && sink.high_water > at
    {
        eprintln!(
            "input: COLLISION — the program stored up to octet {} and the injected input \
                 begins at {at}; it may have overwritten its own source. Raise YANTRA_RAM.",
            sink.high_water
        );
    }
    // ── THE EXIT CODE IS THE HALT — `W-344` ─────────────────────────────────
    //
    // **THIS RETURNED `SUCCESS` UNCONDITIONALLY AND IT HAS ALREADY COST ONE
    // FALSE GREEN.** `tools/fixpoint.sh:78` records it: a Stage 2 attempt
    // halted `StepLimit` after two seconds and exited 0, so a check that
    // trusted the exit code "would have compared against a run that executed
    // almost nothing". `tools/check-float-oracle.sh:71` carries the same
    // margin from its own incident. Two scripts had to work around a one-line
    // omission here, and each wrote down that they were doing so.
    //
    // **A RUN THAT ENDED BADLY NOW SAYS SO IN THE ONE PLACE A SHELL READS
    // WITHOUT BEING TOLD TO.** The halt line on stderr stays exactly as it
    // was — nothing that parses it changes — and the status becomes available
    // to `&&`, `set -e` and `assert!(status.success())` as well.
    //
    // CHECKED AGAINST EVERY CONSUMER BEFORE CHANGING IT, because an exit code
    // is an interface: 18 tools and several tests invoke this binary.
    // `fixpoint.sh` has no `set -e` and greps the log for `halt: Finisher`, so
    // a non-zero exit cannot abort it; `check-float-oracle.sh` has `set -e`
    // and already ends its invocation with `|| true`; `rung-answer.sh:88` also
    // uses `|| true`; `arena-benchmark.sh:81` tests for a step count rather
    // than the code. None of them breaks, and the two margins above are
    // refounded in the same commit rather than left describing a behaviour
    // that is gone.
    //
    // WHY `SpinForever` IS A SUCCESS and not a hang: `jal x0, .` to itself is
    // the idiom both demo programs END with, documented on the variant. It is
    // how a bare-metal program says "done" without a finisher, so treating it
    // as a failure would red every demo.
    //
    // WHY `Finisher { status: None }` IS A FAILURE: the program wrote the
    // finisher with a value the finisher does not define. That is not a
    // success it declared; it is a value nobody can read, and answering
    // `SUCCESS` would be inventing an interpretation.
    //
    // AN EVENT-LOG REFUSAL (`W-371`) OUTRANKS EVERY HALT, a finisher's success included:
    // the halt is what the program did, but the run was not a replay of the log given.
    if let Some(refusal) = refusal {
        eprintln!("events: REFUSED — {refusal}");
        return ExitCode::FAILURE;
    }
    if let Some(why) = thread_failure {
        eprintln!("yantra-run: the threaded run failed — {why}");
        return ExitCode::FAILURE;
    }
    match halt {
        Halt::Finisher {
            status: Some(0), ..
        } => ExitCode::SUCCESS,
        Halt::SpinForever { .. } => ExitCode::SUCCESS,
        // `W-344`'s second half: the step-limit stop never used the word
        // `limit`, so a reader who did not know the halt names could not tell
        // a budget exhaustion from a crash. It says which ceiling and how to
        // raise it, because the fix is almost always the environment variable.
        Halt::StepLimit { pc } => {
            eprintln!(
                "yantra-run: STOPPED AT THE STEP LIMIT of {steps} instructions at pc {pc:#x} — \
                 the program did not finish. Raise YANTRA_STEPS. This is a budget \
                 exhaustion, NOT a program fault, and the {} instructions above are a \
                 LOWER BOUND on what it needed.",
                m.time
            );
            ExitCode::FAILURE
        }
        // `W-370`: THE PROGRAM ASKED TO WAIT AND THIS RUNNER HAS NOTHING TO DELIVER.
        // Not a success — it did not finish, and resuming it with nothing arrived
        // would invent an event log — and not code 1 either, which every failure
        // above shares: a script must be able to tell "paused, needs a host with an
        // event source" from "broke". 75 is sysexits' EX_TEMPFAIL. Without this arm
        // the wait would fall into `other` below: non-zero, but code 1 and unnamed.
        Halt::Wait { pc } => {
            eprintln!(
                "yantra-run: WAIT — the store at pc {pc:#x} asked the host to wait for the \
                 world (ADR-0040), and this runner has no event source (no --events log). The machine \
                 is paused, not finished, after {} instructions.",
                m.time
            );
            ExitCode::from(75)
        }
        // `V-007`: a vector word outside the executed subset halts `Unimplemented`, and
        // the decoder knows WHICH edge it hit (masking, a width, a segment, …). Say so,
        // rather than leaving the reader to decode the word by hand.
        Halt::Unimplemented { pc, word, opcode }
            if opcode == 0x57
                || (matches!(opcode, 0x07 | 0x27)
                    && yantra::vector::is_vector_width((word >> 12) & 7)) =>
        {
            // A word that decodes halts only under a vtype the engine does not compute at;
            // a non-RNE `frm` was the other cause until V-009 (i-f) honoured every mode.
            let why = yantra::vector::decode(word)
                .err()
                .unwrap_or(yantra::vector::REFUSE_VTYPE);
            eprintln!("yantra-run: the vector word {word:#010x} at pc {pc:#x} is refused — {why}");
            ExitCode::FAILURE
        }
        other => {
            eprintln!("yantra-run: the program did not reach a successful finisher — {other:?}");
            ExitCode::FAILURE
        }
    }
}

/// `--listen`'s address (`W-377`, addendum §3, owner ruling Q4): it must PARSE as an IP
/// socket address — so `localhost` is refused without a DNS lookup — and the IP must be
/// exactly `127.0.0.1` or `::1`. Every other address is refused, before any bind.
fn listen_address(text: &str) -> Result<std::net::SocketAddr, String> {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    let addr: std::net::SocketAddr = text.parse().map_err(|e| {
        format!(
            "--listen {text:?} is not an IP address and port ({e}); only 127.0.0.1:PORT and \
             [::1]:PORT are accepted, and a name such as localhost is never looked up"
        )
    })?;
    if addr.ip() == IpAddr::V4(Ipv4Addr::LOCALHOST) || addr.ip() == IpAddr::V6(Ipv6Addr::LOCALHOST)
    {
        Ok(addr)
    } else {
        Err(format!(
            "--listen {text:?}: {} is not 127.0.0.1 or ::1 — this device serves the loopback \
             only (W-377)",
            addr.ip()
        ))
    }
}

/// The refusal of a wait after END, live and in replay alike (`W-377`).
fn after_end(index: usize, pc: u64) -> String {
    format!(
        "a WAIT after END: wait index {index} (the store at pc {pc:#x}) waited after the peer \
         closed (s=end), and there is no other source to wait for (W-377)"
    )
}

/// THE IDLE LIMIT PER WAIT (`W-377`, owner ruling Q4): 30 s, so a broken client fails a
/// test instead of hanging it. A LENGTH handed to the operating system as a socket
/// timeout, never a clock read: `core::time::Duration` is spelled so on purpose, because
/// `tests/w375_clock.rs` reads any `std::time` path in this crate as a clock, and none is
/// read here.
const IDLE_SECONDS: u64 = 30;

/// LIVE MODE FOR THE SOCKET (`W-377`, addendum §3): run `m`; at the FIRST wait accept one
/// client; at EVERY wait do one blocking read of up to `RECORD_CAP` octets (0 = the peer
/// closed), APPEND the record to `log` (flushed, as `input::record_live` does), deliver it
/// and resume. Answers the end — the halt, or a refusal with the halt it left — and the
/// records delivered.
fn serve_live(
    m: &mut yantra::Machine,
    tag: usize,
    listener: std::net::TcpListener,
    steps: u64,
    sink: &mut Sink,
    log: &mut impl std::io::Write,
    granted_files: bool,
) -> (Result<Halt, (String, Halt)>, usize) {
    use std::io::Read;
    let idle = core::time::Duration::from_secs(IDLE_SECONDS);
    let start = m.time;
    let mut delivered = 0;
    let io = |e: std::io::Error| format!("writing the socket log: {e}");
    if let Err(e) = writeln!(log, "{}", socket::SOCKET_LOG_HEADER)
        .and_then(|()| {
            if granted_files {
                writeln!(log, "{FILES_GRANT_LINE}")
            } else {
                Ok(())
            }
        })
        .and_then(|()| log.flush())
    {
        return (Err((io(e), Halt::Wait { pc: m.pc })), 0);
    }
    let mut reader: Option<std::net::TcpStream> = None;
    let mut listener = Some(listener);
    loop {
        let left = steps.saturating_sub(m.time - start);
        let pc = match m.run(left, sink) {
            Halt::Wait { pc } => pc,
            halt => return (Ok(halt), delivered),
        };
        let paused = Halt::Wait { pc };
        if socket::ended(m) {
            return (Err((after_end(delivered, pc), paused)), delivered);
        }
        if reader.is_none() {
            // THE LISTENER CLOSES AT THE FIRST ACCEPT (review of `05eb3a04`): one connection
            // is all this row serves, so a second client is refused AT CONNECT, rather than
            // left in the backlog and reset when the run ends.
            let accepted = match listener.take() {
                Some(l) => accept_within(&l, idle),
                None => Err("the listener is closed and no client was accepted (W-377)".into()),
            };
            match accepted {
                Ok(s) => {
                    eprintln!(
                        "socket: the listener is closed; a second client is refused at connect"
                    );
                    sink.stream = s.try_clone().ok();
                    if sink.stream.is_none() {
                        eprintln!("socket: the connection's write half could not be cloned");
                    }
                    reader = Some(s);
                }
                Err(why) => return (Err((why, paused)), delivered),
            }
        }
        let r = reader.as_mut().expect("accepted above");
        let mut buf = vec![0u8; socket::RECORD_CAP];
        let n = loop {
            match r.read(&mut buf) {
                Ok(n) => break n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return (
                        Err((
                            format!(
                                "the socket was IDLE for more than {IDLE_SECONDS} s at wait index \
                                 {delivered} (the store at pc {pc:#x}): nothing arrived and the \
                                 peer did not close (W-377)"
                            ),
                            paused,
                        )),
                        delivered,
                    );
                }
                Err(e) => {
                    return (
                        Err((
                            format!(
                                "the read at wait index {delivered} (the store at pc {pc:#x}) \
                                 failed: {e} (W-377)"
                            ),
                            paused,
                        )),
                        delivered,
                    );
                }
            }
        };
        let record = if n == 0 {
            SockRecord::End
        } else {
            SockRecord::Octets(buf[..n].to_vec())
        };
        // THE SESSION CAP, before the record is logged or applied: the log and the queue
        // both stop at `socket::SESSION_CAP` delivered octets.
        if let Err(why) = socket::check_cap(m, &record, delivered) {
            return (Err((why, paused)), delivered);
        }
        if let Err(e) = writeln!(log, "{}", socket::record_line(&record)).and_then(|()| log.flush())
        {
            return (Err((io(e), paused)), delivered);
        }
        socket::deliver(m, tag, &record);
        delivered += 1;
    }
}

/// Accept ONE client within `idle`, polling a non-blocking listener (std has no accept
/// timeout); the accepted stream is made blocking, with `idle` as its read timeout.
fn accept_within(
    listener: &std::net::TcpListener,
    idle: core::time::Duration,
) -> Result<std::net::TcpStream, String> {
    const POLL_MS: u64 = 100;
    let polls = idle.as_millis() as u64 / POLL_MS;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("the listener cannot poll: {e}"))?;
    for _ in 0..polls {
        match listener.accept() {
            Ok((s, peer)) => {
                let set = s
                    .set_nonblocking(false)
                    .and_then(|()| s.set_read_timeout(Some(idle)))
                    .and_then(|()| s.set_nodelay(true));
                if let Err(e) = set {
                    return Err(format!("the accepted connection cannot be configured: {e}"));
                }
                eprintln!("socket: accepted {peer}");
                return Ok(s);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(core::time::Duration::from_millis(POLL_MS));
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(format!("accepting the client failed: {e} (W-377)")),
        }
    }
    Err(format!(
        "the socket was IDLE for more than {IDLE_SECONDS} s at the first wait: no client \
         connected (W-377)"
    ))
}

// ── THE FRAME BACKENDS (ADR-0047): host side only; the device (`netdev.rs`) knows none ──

use yantra::netdev::{FrameBackend, NET_BACKEND_UNAVAILABLE, NET_PEER_CLOSED, NET_TIMEOUT};

/// The `--net-*` flags, gathered. Nothing is bound or opened until the run is loaded.
#[derive(Default)]
struct NetFlags {
    peer: Option<String>,
    tap: Option<String>,
    mac: Option<[u8; 6]>,
    max_frame: Option<usize>,
    max_tx: Option<u64>,
    max_rx: Option<u64>,
    timeout_ms: Option<u64>,
}

impl NetFlags {
    /// The live wait's limit: `--net-timeout`, else the 30 s default.
    fn timeout(&self) -> core::time::Duration {
        core::time::Duration::from_millis(
            self.timeout_ms
                .unwrap_or(yantra::netdev::DEFAULT_TIMEOUT_MS),
        )
    }

    fn any(&self) -> bool {
        self.peer.is_some() || self.tap.is_some()
    }

    fn touched(&self) -> bool {
        self.mac.is_some()
            || self.max_frame.is_some()
            || self.max_tx.is_some()
            || self.max_rx.is_some()
            || self.timeout_ms.is_some()
    }

    fn take(&mut self, flag: &str, v: &str) -> Result<(), String> {
        let num = |what: &str| v.parse::<u64>().map_err(|e| format!("{what} {v:?}: {e}"));
        match flag {
            "--net-peer" => self.peer = Some(v.to_string()),
            "--net-tap" => self.tap = Some(v.to_string()),
            "--net-mac" => self.mac = Some(yantra::netdev::parse_mac(v)?),
            "--net-max-frame" => {
                let n = num("--net-max-frame")? as usize;
                if !(yantra::netdev::MIN_FRAME..=yantra::netdev::HARD_MAX_FRAME).contains(&n) {
                    return Err(format!("--net-max-frame {n}: must be 14 to 65535"));
                }
                self.max_frame = Some(n);
            }
            "--net-max-tx-frames" => self.max_tx = Some(num("--net-max-tx-frames")?),
            "--net-max-rx-frames" => self.max_rx = Some(num("--net-max-rx-frames")?),
            "--net-timeout" => {
                let n = num("--net-timeout")?;
                if n == 0 {
                    return Err("--net-timeout 0: a wait that never waits is not a timeout; omit it for the 30 s default".to_string());
                }
                self.timeout_ms = Some(n);
            }
            other => {
                return Err(format!(
                    "unknown flag {other} (the net flags are --net-peer, --net-tap, --net-mac, \
                     --net-max-frame, --net-max-tx-frames, --net-max-rx-frames, --net-timeout)"
                ));
            }
        }
        Ok(())
    }

    fn config(&self) -> yantra::netdev::NetConfig {
        let d = yantra::netdev::NetConfig::default();
        yantra::netdev::NetConfig {
            defer: net_defer_env().unwrap_or(0),
            backend: if self.tap.is_some() {
                yantra::netdev::BackendKind::Tap
            } else if self.peer.is_some() {
                yantra::netdev::BackendKind::Peer
            } else {
                yantra::netdev::BackendKind::None
            },
            mac: self.mac.unwrap_or(d.mac),
            max_frame: self.max_frame.unwrap_or(d.max_frame),
            max_tx: self.max_tx.unwrap_or(d.max_tx),
            max_rx: self.max_rx.unwrap_or(d.max_rx),
            // `--net-mac` is also the optional MAC allow-list (ADR-0047).
            mac_filter: self.mac.is_some(),
        }
    }

    /// Open the backend asked for.
    fn open(&self) -> Result<Box<dyn FrameBackend>, String> {
        // A FINITE DEFAULT: a live wait that hears nothing for 30 s halts with NET_TIMEOUT
        // instead of hanging. `--net-timeout 0` is refused; pass a large number to wait longer.
        let timeout = Some(self.timeout());
        match (&self.peer, &self.tap) {
            (Some(p), _) => open_peer(p, timeout).map(|b| Box::new(b) as Box<dyn FrameBackend>),
            (_, Some(t)) => open_tap(t, timeout).map(|b| Box::new(b) as Box<dyn FrameBackend>),
            _ => Err("no backend".to_string()),
        }
    }
}

/// `YANTRA_VIRTIO_DEFER`, if set to decimal digits (the GPU's own parse refuses the rest).
fn net_defer_env() -> Option<u64> {
    let v = std::env::var("YANTRA_VIRTIO_DEFER").ok()?;
    (!v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()))
        .then(|| v.parse().ok())
        .flatten()
}

type Incoming = Result<Vec<u8>, String>;
type Sender = Box<dyn FnMut(&[u8]) -> std::io::Result<()>>;

/// A backend made of a writer and a reader thread: the thread hands each frame it reads to
/// a channel, so a wait can block, poll, or time out without a half-read frame.
struct ChannelBackend {
    send: Sender,
    rx: std::sync::mpsc::Receiver<Incoming>,
    timeout: Option<core::time::Duration>,
    failed: bool,
    /// Removes the rendezvous file when the listening side exits.
    _guard: Option<PeerGuard>,
}

struct PeerGuard(std::path::PathBuf);

impl Drop for PeerGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

impl FrameBackend for ChannelBackend {
    fn send(&mut self, frame: &[u8]) {
        if self.failed {
            return;
        }
        if let Err(e) = (self.send)(frame) {
            eprintln!(
                "net: send failed — {e}; nothing more is sent, and the program is not told (ADR-0047)"
            );
            self.failed = true;
        }
    }

    fn recv(&mut self) -> Result<Vec<u8>, String> {
        use std::sync::mpsc::RecvTimeoutError;
        match self.timeout {
            Some(d) => match self.rx.recv_timeout(d) {
                Ok(r) => r,
                Err(RecvTimeoutError::Timeout) => Err(NET_TIMEOUT.to_string()),
                Err(RecvTimeoutError::Disconnected) => Err(NET_PEER_CLOSED.to_string()),
            },
            None => self
                .rx
                .recv()
                .unwrap_or_else(|_| Err(NET_PEER_CLOSED.to_string())),
        }
    }

    fn try_recv(&mut self) -> Result<Option<Vec<u8>>, String> {
        use std::sync::mpsc::TryRecvError;
        match self.rx.try_recv() {
            Ok(r) => r.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(NET_PEER_CLOSED.to_string()),
        }
    }
}

/// `--net-peer=PATH`: ONE flag, a rendezvous path. The first instance to start binds a
/// unix-domain stream socket there and waits for the other; the second connects. Frames
/// cross as a two-octet big-endian length and the frame (a stream needs framing, and a
/// frame is at most 65,535 octets). Unix-domain sockets need no privilege and no port, and
/// the path is the only thing shared. WINDOWS would use a named pipe (`\\.\pipe\NAME`);
/// std has no server side for one, so this build refuses by name there.
#[cfg(unix)]
fn open_peer(path: &str, timeout: Option<core::time::Duration>) -> Result<ChannelBackend, String> {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    let p = std::path::Path::new(path);
    let mut guard = None;
    let mut stream = None;
    for _ in 0..100 {
        let refused = match UnixStream::connect(p) {
            Ok(s) => {
                eprintln!("net: peer {path} connected");
                stream = Some(s);
                break;
            }
            Err(e) => e.kind() == std::io::ErrorKind::ConnectionRefused,
        };
        match UnixListener::bind(p) {
            Ok(l) => {
                guard = Some(PeerGuard(p.to_path_buf()));
                eprintln!("net: peer {path} listening");
                let s = match timeout {
                    None => l.accept().map(|(s, _)| s).map_err(|e| e.to_string()),
                    Some(d) => {
                        l.set_nonblocking(true).map_err(|e| e.to_string())?;
                        let mut waited = core::time::Duration::ZERO;
                        loop {
                            match l.accept() {
                                Ok((s, _)) => break Ok(s),
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    if waited >= d {
                                        break Err(NET_TIMEOUT.to_string());
                                    }
                                    std::thread::sleep(core::time::Duration::from_millis(10));
                                    waited += core::time::Duration::from_millis(10);
                                }
                                Err(e) => break Err(e.to_string()),
                            }
                        }
                    }
                }?;
                s.set_nonblocking(false).map_err(|e| e.to_string())?;
                eprintln!("net: peer {path} connected");
                stream = Some(s);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse && refused => {
                // A socket file nobody listens on: left by a run that died.
                let _ = std::fs::remove_file(p);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                std::thread::sleep(core::time::Duration::from_millis(50));
            }
            Err(e) => return Err(format!("cannot use the peer path {path:?}: {e}")),
        }
    }
    let stream =
        stream.ok_or_else(|| format!("cannot rendezvous at {path:?}: no peer after 100 tries"))?;
    let mut reader = stream.try_clone().map_err(|e| e.to_string())?;
    let (tx, rx) = std::sync::mpsc::channel::<Incoming>();
    std::thread::spawn(move || {
        loop {
            let mut len = [0u8; 2];
            let mut buf;
            if reader.read_exact(&mut len).is_err() {
                let _ = tx.send(Err(NET_PEER_CLOSED.to_string()));
                return;
            }
            buf = vec![0u8; usize::from(u16::from_be_bytes(len))];
            if reader.read_exact(&mut buf).is_err() {
                let _ = tx.send(Err(NET_PEER_CLOSED.to_string()));
                return;
            }
            if tx.send(Ok(buf)).is_err() {
                return;
            }
        }
    });
    let mut writer = stream;
    Ok(ChannelBackend {
        send: Box::new(move |f| {
            let mut m = (f.len() as u16).to_be_bytes().to_vec();
            m.extend_from_slice(f);
            writer.write_all(&m)
        }),
        rx,
        timeout,
        failed: false,
        _guard: guard,
    })
}

#[cfg(not(unix))]
fn open_peer(path: &str, _: Option<core::time::Duration>) -> Result<ChannelBackend, String> {
    Err(format!(
        "{NET_BACKEND_UNAVAILABLE}: --net-peer {path:?} on this platform is a Windows named \
         pipe, which this build does not implement (ADR-0047; tested on Linux only)"
    ))
}

/// A TAP interface on Linux: `--net-tap=IFNAME` attaches to `/dev/net/tun`. CREATING an
/// interface needs `CAP_NET_ADMIN`; attaching to one made beforehand for this user
/// (`ip tuntap add dev IFNAME mode tap user UID`, as root) needs none. Frames cross whole
/// (`IFF_NO_PI`). This is the one `unsafe` in the crate, a single `ioctl`.
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )
))]
fn open_tap(name: &str, timeout: Option<core::time::Duration>) -> Result<ChannelBackend, String> {
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    #[allow(unsafe_code)]
    mod sys {
        unsafe extern "C" {
            pub fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
        }
    }
    const TUNSETIFF: std::ffi::c_ulong = 0x4004_54ca;
    const IFF_TAP: u16 = 0x0002;
    const IFF_NO_PI: u16 = 0x1000;
    if name.is_empty() || name.len() >= 16 {
        return Err(format!(
            "--net-tap {name:?}: an interface name is 1 to 15 octets"
        ));
    }
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/net/tun")
        .map_err(|e| {
            format!("{NET_BACKEND_UNAVAILABLE}: cannot open /dev/net/tun: {e} (the tun module and access to the device are needed)")
        })?;
    let mut ifr = [0u8; 40];
    ifr[..name.len()].copy_from_slice(name.as_bytes());
    ifr[16..18].copy_from_slice(&(IFF_TAP | IFF_NO_PI).to_ne_bytes());
    #[allow(unsafe_code)]
    // SAFETY: `ifr` is the 40-octet `struct ifreq` TUNSETIFF reads, alive for the call.
    let rc = unsafe { sys::ioctl(f.as_raw_fd(), TUNSETIFF, ifr.as_mut_ptr()) };
    if rc < 0 {
        let e = std::io::Error::last_os_error();
        return Err(format!(
            "{NET_BACKEND_UNAVAILABLE}: cannot attach to the TAP interface {name:?}: {e}. \
             Creating one needs CAP_NET_ADMIN; an interface made beforehand for this user \
             (ip tuntap add dev {name} mode tap user UID) needs none"
        ));
    }
    eprintln!("net: tap {name} attached");
    let mut reader = f.try_clone().map_err(|e| e.to_string())?;
    let (tx, rx) = std::sync::mpsc::channel::<Incoming>();
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 65_536];
        loop {
            match reader.read(&mut buf) {
                Ok(n) if n > 0 => {
                    if tx.send(Ok(buf[..n].to_vec())).is_err() {
                        return;
                    }
                }
                _ => {
                    let _ = tx.send(Err(NET_PEER_CLOSED.to_string()));
                    return;
                }
            }
        }
    });
    let mut writer = f;
    Ok(ChannelBackend {
        send: Box::new(move |fr| writer.write_all(fr)),
        rx,
        timeout,
        failed: false,
        _guard: None,
    })
}

/// macOS (vmnet), Windows (Wintun) and the rest: refused by name, with what each needs.
/// Linux on an architecture other than x86_64, aarch64 and riscv64 is here too: TUNSETIFF's
/// request number is `_IOW('T', 202, int)` and its encoding differs on mips, powerpc and
/// sparc, which this build has not checked.
#[cfg(not(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )
)))]
fn open_tap(name: &str, _: Option<core::time::Duration>) -> Result<ChannelBackend, String> {
    let needs = if cfg!(target_os = "macos") {
        "macOS needs vmnet: the com.apple.vm.networking entitlement or root"
    } else if cfg!(target_os = "linux") {
        "this Linux architecture is not one of x86_64, aarch64 and riscv64, the ones whose TUNSETIFF number is verified"
    } else if cfg!(windows) {
        "Windows needs the Wintun driver (wintun.dll) and administrator rights"
    } else {
        "this platform has no TAP backend"
    };
    Err(format!(
        "{NET_BACKEND_UNAVAILABLE}: --net-tap {name:?} is not implemented on this platform: \
         {needs} (ADR-0047; only the Linux backend is built and tested)"
    ))
}

#[cfg(test)]
mod net_flag_tests {
    use super::NetFlags;

    #[test]
    fn the_default_timeout_is_30_s_and_the_flag_overrides_it() {
        let mut f = NetFlags::default();
        assert_eq!(f.timeout(), core::time::Duration::from_secs(30));
        f.take("--net-timeout", "250").unwrap();
        assert_eq!(f.timeout(), core::time::Duration::from_millis(250));
        assert!(f.take("--net-timeout", "0").is_err());
    }
}
