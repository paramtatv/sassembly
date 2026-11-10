# The socket device in the native runtime (W-377; NATIVE-GAPS C3, LIMITS rows 3, 39, 40)

The reference is yantra's, read only: `crates/yantra/src/socket.rs` (the device),
`crates/yantra/src/bin/yantra-run.rs` (`--listen`, `serve_live`, `accept_within`, the report line),
`docs/adr/0040-addendum-w377-sockets.md` and `crates/yantra/tests/w377_sockets.rs`. The native runtime
must print what yantra-run prints, octet for octet, wherever it runs the device. This note says
what it does, and where it does not.

Scope of this phase: **x86_64-linux, aarch64-linux, aarch64-macos and x86_64-macos, live only.**
Windows routes every socket run to yantra-run (t1-run.sh), and so does every replay (`--events` on a
socket log). On the four live targets a socket-log replay is native (phase 2; replay runtime emitted by anuvada.py, oracle==native): t1-run.sh parses
and refuses the log in yantra-run's words, and the runtime maps it (`YANTRA_SOCK_REPLAY=<log>`, read-only),
delivers one record per wait, and refuses SHORTER, SESSION_CAP and a WAIT after END itself; t1-run.sh
derives LONGER from the report line's record count.

**Darwin** (agent/rtc-sock-mac) runs the same code with BSD sockets: socket 97, setsockopt 105, bind 104,
listen 106, getsockname 32, accept 30, poll 230, sendto 133, read 3, write 4, open 5, close 6, mmap 197,
lseek 199, ftruncate 201 (+0x2000000 on x86-64; x16 and svc #0x80 on arm64). The carry is the error flag
(the runtime makes the answer -errno, as Linux answers). sockaddr_in leads with sin_len (16).
SOL_SOCKET is 0xffff (SO_REUSEADDR 4). There is no MSG_NOSIGNAL: the client gets SO_NOSIGPIPE
(0x1022) at accept. ECONNRESET is 54. On arm64 the RX latch's address is answered as a guest address
(minus delta), and the guest's word is read at hbase.

## 1. The register surface (SOCK = 0x1000_0110, socket.rs)

| Address | Name | Access | Meaning |
|---|---|---|---|
| 0x1000_0110 | NEXT | store, value 0 | pop the queue's head into the RX latch; an empty queue latches EMPTY (0x100), or END (0x101) once the peer has closed (sticky) |
| 0x1000_0114 | RX | load | the latch: an octet 0x00..0xff, EMPTY, END, or UNREAD (0x102, before any NEXT) |
| 0x1000_0118 | TX | store | send the low octet; a value above 0xff is refused, never truncated |
| 0x1000_011c | CLOSE | any | reserved (deferred); every access is refused |

- **Order of the checks** (socket.rs `load` / `store`): no device; then the width and alignment
  (exactly four octets, offset a multiple of four); then the register. Only the low 32 bits of a
  store are judged.
- **A load cannot pop.** The pop is NEXT's store; RX only reads the latch. So the queue changes
  only at NEXT and at a WAIT.
- **Native mechanics.** A guest access to the window misses RAM and reaches the runtime's slow path.
  - The STORE slow path is already called from the cold stub and returns to it.
  - The LOAD cold stub now CALLs `rt_load_slow` too (it used to jump) and continues at the load's
    own B label. Every other device load still never returns.
  - For RX, `rt_load_slow` answers the latch's ADDRESS (x86-64 rax, A64 T0). The guest's own load
    instruction then reads it, so `lw` and `lwu` (and a mapped or spilled rd) need no per-register
    code in the runtime. The values are below 0x200, so `lw` and `lwu` read the same word.
  - The width of a store (and of an A64 load) is taken from the guest's own instruction word,
    funct3 & 3, because those slow paths are not handed it.

## 2. Selection: the native contract (t1-run.sh)

yantra-run accepts `--listen 127.0.0.1:PORT` only right after `--record-events <log>`.
t1-run.sh maps that pair, and nothing else, to three environment variables. They are set only by
t1-run.sh, and it unsets them on entry:

| Variable | Meaning |
|---|---|
| `YANTRA_LISTEN=127.0.0.1:PORT` | yantra-run's `--listen`: the runtime binds at load. PORT is 1 to 5 decimal digits, at most 65535; 0 asks the system for one. |
| `YANTRA_RECORD_EVENTS=<log>` | the log. t1-run.sh points it at a temporary file and publishes the file to the user's LOG once the listener has been announced. |
| `YANTRA_SOCK_SENT=<file>` | every octet TX sent, appended. t1-run.sh puts its sha256 into the report line (§4). |

- t1-run.sh sends the run to yantra-run, which judges it, when:
  - the host is not Linux or macOS (Windows);
  - the address is not canonical `127.0.0.1:PORT` (`[::1]`, `localhost`, 0.0.0.0, leading zeros,
    a port past 65535);
  - any other flag order is given.
- **Without YANTRA_LISTEN the device does not exist**, as in yantra-run with no `--listen`. Every
  access then halts with the no-device words (§5), and the run says
  `socket: touched at 0x… with no socket device — …`. That is natively correct with no flag at all
  (LIMITS rows 39 and 40).

## 3. Load time (yantra-run's order)

1. The usual loader steps run: RAM, the segments, YANTRA_RECORD_EVENTS is opened and the input
   is injected.
2. The event slot is looked up: the SASEVENT tag must occur exactly once, and the image must not
   be threaded.
3. `rt_sk_init` runs:
   - It parses YANTRA_LISTEN.
   - It opens the listener: socket, SO_REUSEADDR (as Rust's `TcpListener::bind`), bind to
     127.0.0.1:PORT, listen with a backlog of 128, then getsockname.
   - It reserves the queue: one mmap of 64 MiB + 4 KiB, MAP_NORESERVE.
   - It rewrites the log to the socket log's header: ftruncate the clock header away, then write
     `socket::SOCKET_LOG_HEADER`, then the `--files` grant line if YANTRA_FILES is set.
   - It opens YANTRA_SOCK_SENT.
   - It sets the latch to UNREAD and prints `socket: listening on 127.0.0.1:<the bound port>`.
4. If anything fails BEFORE that line, it is a **native-only refusal**,
   `native: not in milestone 1 (…; run it under yantra-run)`, exit 2. t1-run.sh then reruns the
   run whole under yantra-run, which prints its own words. Nothing has crossed a connection yet.
   The cases are:
   - no YANTRA_RECORD_EVENTS;
   - no SASEVENT tag, or a threaded image;
   - an address it cannot parse;
   - a socket, bind, listen or mmap that fails.

## 4. The WAIT (`rt_sk_wait`, from `rt_wait` when the device exists; yantra-run's `serve_live`)

At every store to WAIT (0x1000_0100):

1. **After END**, the wait is refused (§5).
2. **At the first wait**, the runtime accepts ONE client:
   - It polls the listener with a 30 s limit, then accepts.
   - It prints `socket: accepted <ip>:<port>`, then
     `socket: the listener is closed; a second client is refused at connect`.
   - It closes the listener, so a second client meets ECONNREFUSED, as under yantra.
   - It sets TCP_NODELAY.
3. **One read per wait**: poll with a 30 s limit, then `read(client, queue tail, 4096)`. EINTR
   retries. 0 is END.
4. **The session cap** is checked before anything is logged or applied: the octets delivered so
   far plus this read must stay at or under 64 MiB.
5. **The log line**:
   - `s=<two lowercase hex digits per octet>` or `s=end`, newline-ended, in one write.
   - It is written as the record is delivered. The log is unbuffered, so a crash leaves every
     record the program saw.
6. **Deliver**:
   - The octets are appended to the queue: the tail advances, and the tail is the received
     count.
   - The count (0 for END) is written as a u64 at the SASEVENT tag + 8.
   - The record count is incremented, and the guest resumes after its store.

**What native buffers, and when.**
- **RX octets:** only in the queue, and only at a wait (one read each), exactly as yantra. The
  queue is a single run that only grows (head ≤ tail ≤ 64 MiB + 4 KiB), so no octet is ever
  moved.
- **TX octets:** not buffered. Each TX store is one `sendto(client, 1 octet, MSG_NOSIGNAL)` at
  once, as yantra's `write_all` per octet, plus one append to YANTRA_SOCK_SENT.
  - Before any client exists, TX is counted and appended but not sent. The run says once:
    `socket: send failed after N octets — no client has connected yet (one is accepted at the first wait); the program is not told (W-377)`.
  - A failed send says once:
    `socket: send failed after N octets — <error>; nothing more is written, and the program is not told (W-377)`.
    Nothing more is sent after it. MSG_NOSIGNAL (Darwin: SO_NOSIGPIPE) makes a closed peer EPIPE,
    not SIGPIPE, as Rust, which ignores SIGPIPE.
- **The UART:** as before, 64 KiB buffered, flushed at every halt.

**The report line**, printed at every exit once the listener is bound (after the halt line, as
yantra-run's comes after `steps:`):

`socket: <received> octets received in <records> records, <sent> sent, sent sha256 <digest>`

The runtime prints the marker `@YANTRA_SOCK_SENT@` in place of the digest. t1-run.sh replaces it
with the sha256 of the YANTRA_SOCK_SENT file (sha256sum, or shasum -a 256). This is the
side-file ruling sansos-99 gave for the net device's `sent sha256`, applied here: there is no
SHA-256 in the runtime.

## 5. Every refusal, by name

**Device refusals** halt `Device { pc, addr, why }`. The halt line is printed as yantra's Debug
form: `halt: Device { pc: <dec>, addr: <dec>, why: "<why>" }`. The exit is 1. The words are
copied from socket.rs by script into `SOCK_WHY` and checked by `tools/check-anuvada-words.sh`:

| Name | When |
|---|---|
| WHY_NO_DEVICE | any access with no YANTRA_LISTEN; then also `socket: touched at <addr hex> with no socket device — <why>` |
| WHY_WIDTH | a width other than 4, or an offset that is not a multiple of 4 (checked before the register) |
| WHY_LOAD_STORE_ONLY | a load at NEXT or TX |
| WHY_STORE_LOAD_ONLY | a store at RX |
| WHY_NEXT_VALUE | NEXT with low 32 bits ≠ 0 |
| WHY_TX_VALUE | TX with low 32 bits > 0xff |
| WHY_CLOSE | any access at 0x1000_011c |

**Wait refusals**, in yantra-run's words. Each prints `halt: Wait { pc: <dec> }`, then the report
line, then `events: REFUSED — <why>`, and exits 1:

| Name (this note's) | Words |
|---|---|
| AFTER_END | `a WAIT after END: wait index K (the store at pc 0x…) waited after the peer closed (s=end), and there is no other source to wait for (W-377)` |
| IDLE_FIRST | `the socket was IDLE for more than 30 s at the first wait: no client connected (W-377)` |
| IDLE | `the socket was IDLE for more than 30 s at wait index K (the store at pc 0x…): nothing arrived and the peer did not close (W-377)` |
| READ_FAILED | `the read at wait index K (the store at pc 0x…) failed: <error> (W-377)` |
| ACCEPT_FAILED | `accepting the client failed: <error> (W-377)` |
| SOCKET_LIMIT_EXCEEDED | `सङ्केतमात्राप्रतिषेधः (SOCKET_LIMIT_EXCEEDED): record K (N octets) would take the session's delivered records past the cap of 67108864 octets (64 MiB), to T — refused, not applied and not logged (W-377)` |

`<error>` is Rust's io::Error text for EPIPE (`Broken pipe (os error 32)`) and ECONNRESET
(`Connection reset by peer (os error 104)`, on Darwin `os error 54`). Any other errno prints `os error N`, which is a
named difference from yantra's strerror text.

**Native-only refusals** (§3) all happen before the listener exists, so they fall back.

**THE TERMINAL RULE** (native-frames-device.md §3).
- Once `socket: listening on` has been printed, a client may have connected and octets may have
  crossed. A rerun would bind again (on another port, if PORT is 0) and serve the connection a
  second time.
- So t1-run.sh never reruns such a run. If a native-only halt follows, for example a store into the
  code span, or a jump to a word yantra would execute but the translation does not hold, t1-run.sh prints:

  `t1-run: socket: REFUSED — the native run reached what the translator refuses at run time (<the native line>) after the listener was bound; it is NOT rerun under yantra-run, which would serve the connection a second time (W-377, SOCKET.md)`

  and exits 1. The log written so far is still published.
- A halt that is yantra's own (since the native-halts change: Unimplemented, Breakpoint, BeyondRam,
  `BadAccess { pc, addr }`, `Device { … }`) is native's answer and final anyway: it is printed as is,
  exit 1, and nothing is rerun. The rule above is only for the native-only halts.
- **Streaming.** For a socket run, t1-run.sh streams stderr:
  - Lines are held only until the `listening` line, so a pre-bind fallback stays silent and whole.
  - After that line, every line is printed as it is written, so a harness can read the port live.
  - stdout is not held either.

## 6. Differences that remain (named)

- The native runtime prints neither `events: LIVE, recording a socket log to …` nor
  `events: N socket records read, logged and delivered, one per wait`. It also prints no `steps:`
  and no `yantra-run: …` lines. The clock branch already lists these.
- An errno other than EPIPE or ECONNRESET is printed as `os error N` (§5).
- The device is not native on Windows. There, t1-run.sh routes the run to yantra-run, and the
  runtimes keep the old halt for a window access (`not in milestone 1`, which reruns).
- Replay (`--events` on a socket log) stays on yantra-run.

## 7. The .t1 port

- The runtimes are generated:
  - `runtime_sock` is RECORDED by rt2t1.py (added to its RECORDED list).
  - `runtime_sock_a64` is recorded by rta2t1.py with the rest of `runtime_a64`.
- The lowering is hand-written, and it changes in two places:
  - `main.t1a`: the load cold stub calls `rt_load_slow` and continues at the B label. The x86-64
    load lowering already defines that label.
  - `arm.t1a`: the same stub, saving x30 around the bl, plus a B label before the guarded A64
    load.
- With these, the .t1 translator built by the compiler at `trees/27` (Stage 1, native) emits the
  oracle's octets for every socket probe and every e2e image, on x86_64-linux and aarch64-linux.
- `anuvada.t1` with ANU_WIN=0 grows from 2,580,973 to 2,708,953 octets (+127,980).

## 8. Probes (rtprobe/)

- `mksock.sh DIR` builds the images:
  - `p.echo`: echo with UNREAD, n=, END sticky;
  - `p.txfirst`: TX before any client;
  - `p.after`: a wait after END;
  - `p.hoard`: the 64 MiB cap;
  - `r.*`: every device refusal, plus two accesses that succeed.
- `sockclient.sh` is the client.
- `sockpair.sh DIR RUNNER [SLOW]` writes a transcript: stdout, stderr with the ports masked, the
  log, the exit status and the client's lines. RUNNER is either yantra-run or tools/t1-run.sh.
- Two transcripts compare with `diff`. SLOW adds the two 30 s IDLE refusals and the cap.
