# Networking in Sassembly

Short version: a Sassembly program can serve **one TCP client on the loopback
interface**, but only when it runs under `yantra-run`. There is no operating-system
network stack in this release: no TCP/IP, no UDP, no TLS, no DNS, no HTTP. The
OS-level networking work is not part of this release.

**v1.1.0-alpha** adds a second, separate path: a network device that moves raw Ethernet
frames and nothing else (`--net-peer`, `--net-tap`; ADR-0047), and a network stack written
in `.t1` above it in `spec/net/`: Ethernet, ARP, IPv4, ICMP echo, UDP and a DNS client
(ADR-0048). There is no TCP, TLS or HTTP in it. Host setup, what was tested on which platform,
and the known limits are in [docs/networking-host-setup.md](docs/networking-host-setup.md).
Sections 1 to 7 below describe the v1.0.0 socket device, which is unchanged.

This file replaces an earlier text that said the language could not wait. It now can.
Every claim below names its source in this repository; paths are relative to the
repository root. The example in section 7 was run with the published v1.0.0 binaries.

## 1. Waiting

A T1 program waits by calling one member of the `अष्टक` module:

    अष्टकॱघटनाप्रतीक्षा ०

- The name is `WAIT_BUILTIN_MODULE` + U+0971 + `WAIT_BUILTIN_MEMBER`
  (`crates/sadhana/src/t1/nirvahana.rs:4556,4561`). It has no declaration in the
  corpus; it is recognised by exact qualified name (`nirvahana.rs:4577`).
- It takes one argument. The argument is reserved and ignored (`nirvahana.rs:4565-4568`);
  the example passes `०`.
- Natively it is a store to the WAIT address `0x1000_0100` (`crates/yantra/src/lib.rs:371`).
  The machine halts, keeping all state, and the host decides when to resume it. From the
  program's side the wait is one retired instruction (`crates/yantra/tests/w370_halt_wait.rs:1-11`),
  so the instruction count never depends on how long the host took.

## 2. The event and log model

A program that waits declares a tag word followed by an event slot:

    सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति <the tag, 0x544e455645534153 = "SASEVENT"> ।
    सार्वजनिक चरः घटनामूल्यक ॱॱ न६४ भवति ० ।

The host finds the slot by value (`crates/yantra/src/input.rs:422-452`; tag at
`input.rs:427`). At each wait it writes the next record into that slot and resumes.

- `yantra-run --record-events LOG ...` runs live and appends one line per wait to LOG.
- `yantra-run --events LOG ...` replays that file with no live source.
- A log shorter or longer than the program's waits is refused with exit 1.
  Without either flag, a program that waits exits 75
  (`crates/yantra/src/bin/yantra-run.rs:189-199`).
- The queue changes only at a wait, so between two waits every answer is a function
  of what was already delivered. Live and replay therefore retire the same number of
  instructions (`crates/yantra/src/socket.rs:25-28`; test
  `w377_b_the_live_log_replays_twice_identically`,
  `crates/yantra/tests/w377_sockets.rs:767`).

## 3. The socket device

A 16-octet register window at `SOCK = 0x1000_0110` (`lib.rs:397`; layout in
`crates/yantra/src/socket.rs:7-14, 34-42`). A program reaches it through the two
existing device calls `उपकरणचतुरष्टकनिधानम्` (store) and `उपकरणचतुरष्टकाहारः` (load);
there is no socket keyword or library.

| offset | access | meaning |
|---|---|---|
| +0 NEXT | store `0` | pop the head of the receive queue into the RX latch |
| +4 RX | load | latch: `0x00..0xff` an octet, `0x100` EMPTY, `0x101` END (sticky), `0x102` UNREAD (no NEXT yet) |
| +8 TX | store | send the low octet (a value above `0xff` is refused) |
| +12 | any | reserved; refused by name |

A program loops: NEXT, read RX; on an octet, TX it or use it; on EMPTY, wait; on END, stop.

What `yantra-run` serves (`yantra-run.rs:153-232, 1233-1320`):

- `--listen 127.0.0.1:PORT` (or `[::1]:PORT`) is accepted only right after
  `--record-events LOG`. Any other address is refused before any bind. Port `0` picks a
  port and prints it. Checked: `--listen 0.0.0.0:5588` exits 1 with
  "this device serves the loopback only".
- **One client.** The first wait accepts it. The listener then closes and a second
  connection is refused at connect (`yantra-run.rs:1306-1315`; test
  `w377_r3_a_second_client_is_refused_at_connect`, `w377_sockets.rs:2032`).
- Each wait logs one read of up to 4096 octets (`socket.rs:55`) as `s=<hex>`, or
  `s=end` when the peer closed. A session may deliver at most 64 MiB (`socket.rs:62`);
  the record that would pass it is refused.
- A wait after `s=end`, or idle for 30 s, exits 1 (`yantra-run.rs:1264-1269`; tests at
  `w377_sockets.rs:1292, 1311`).
- **Replay without a network:** `yantra-run --events LOG prog.elf`, where the log's
  first record starts `s=`. Same stdout, same instruction count, same `socket:` summary
  line (section 7; tests `w377_a_a_live_localhost_echo_echoes_and_exits_zero` and `w377_b_the_live_log_replays_twice_identically`, `w377_sockets.rs:722, 767`).
- Only `yantra-run` touches the network. The emulator library does not, and a source
  ratchet enforces it (`socket.rs:30-32`; test `w377_g_only_yantra_run_names_the_network`,
  `w377_sockets.rs:1802`).

## 4. Cooperative threads

An image that declares `SASTHRDS` (a count, 1 to 64) and `SASTHRID` (a slot the host
sets to the running thread's number) runs as N threads over one machine
(`crates/yantra/src/threads.rs:1-60, 101-107`).

- There is no preemption. The host switches only at a wait or when a thread ends
  (`threads.rs`, "No preemption").
- The schedule is part of the log: `@N` lines say "run thread N". Live mode picks
  round-robin and records the picks. A threaded image needs `--events` or
  `--record-events`; without one it exits 1 at load (`yantra-run.rs:491`).
- RAM, devices and the clock are shared. A threaded image must also declare `SASEVENT`.
- Thread tests: `crates/yantra/tests/w376_threads.rs`.

## 5. Limits

- **Threads and sockets together are refused.** A threaded image with `--listen`, or
  replayed against a socket log, exits 1 at load with "a threaded image with a socket
  ... is refused" (`yantra-run.rs:481-487`; test
  `w377_d_threads_with_a_socket_are_refused_at_load`, `w377_sockets.rs:911`). So there
  is no multi-connection server in this release.
- Loopback only, one client, one session, 4096-octet records, 64 MiB per session.
- The socket device and the wait exist in `yantra` (the emulator, run as `yantra-run`)
  and in the T1 interpreter, which the tests compare against it
  (`w377_f_three_engines_agree_*`, `w377_sockets.rs:1527, 1539`). They are not features
  of the RISC-V ISA or of a QEMU machine. I have not run a wait or the socket window on
  QEMU, and make no claim that they work there. A bare-metal image that stores to
  `0x1000_0100` or `0x1000_0110` on any other machine is not covered by this document.
- Run without `--listen` or a socket log, a program that touches the window halts with
  "no socket device: pass `--listen` or a socket log" and exit 1 (checked).

## 6. What is not here

- No operating-system network stack: no TCP, UDP, TLS, DNS or HTTP, and no sockets
  API beyond the register window above. The OS-level networking work is not part of
  this release.
- The repository's `spec/network.sas` and `tools/check-network.sh` are a different,
  much smaller thing: a bare-metal virtio-net program that answers ARP and ICMP echo
  (ping) under QEMU. Their own headers say there is no TCP, TLS or HTTP in them. I did
  not run that check for this document, and it is not the socket device.
- No outbound connections, no listening on a non-loopback address, no DNS lookups through
  the socket device. (The v1.1.0-alpha frame device and its `.t1` stack are the separate path
  described at the top of this file.)

## 7. A verified example

`examples/networking/echo.t1` echoes every octet it receives and exits having counted them. It is
generated by `examples/networking/gen_echo.py` from the device constants and call names in the source,
not typed by hand. It uses module `शृङ्खला` and entry `स्वपरीक्षास्वप्रतिबिम्बम्`
because those are fixed by the prebuilt compiler (`stage1.elf`).

Commands, from a directory holding the release tarball and `stage1.elf`, with `echo.t1` taken from `examples/networking/`
(`examples/networking/run.sh` does all of it, and downloads them with `gh release download v1.0.0 -R
paramtatv/sassembly` if missing):

    tar xzf sassembly-v1.0.0-linux-x86_64.tar.gz
    Y=$PWD/sassembly-v1.0.0-linux-x86_64/yantra-run
    { printf 'शृङ्खला\0'; cat examples/networking/echo.t1; printf '\0'; } > p.blob
    YANTRA_INPUT=p.blob YANTRA_INPUT_NAME=x YANTRA_RAM=2684354560 \
      YANTRA_STEPS=4000000000000 $Y sassembly-v1.0.0-stage1.elf > sink   # ~12 s
    n=$(wc -c < sink); tail -c +2 sink | dd bs=1 count=$((n-2)) of=echo.elf 2>/dev/null
    $Y --record-events live.log --listen 127.0.0.1:5577 echo.elf > count.bin 2> live.err &
    sleep 1; printf 'hello\n' | nc -q1 127.0.0.1 5577
    wait
    $Y --events live.log echo.elf          # replay, no network

Real output (Linux x86_64, release binaries, `examples/networking/run.sh`):

    halt: Finisher { value: 78656307, status: Some(1200) }      <- compiler: built
    --- client received:
    hello
    --- stdout (little-endian octets-count word):
                        6
    --- yantra-run stderr (socket lines):
    socket: listening on 127.0.0.1:5577
    socket: accepted 127.0.0.1:39522
    socket: the listener is closed; a second client is refused at connect
    halt: Finisher { value: 21845, status: Some(0) }
    steps: 617 executed instructions
    socket: 6 octets received in 2 records, 6 sent, sent sha256 5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03
    --- event log:       (one comment header line omitted)
    s=68656c6c6f0a
    s=end
    --- replay stdout:
                        6
    --- replay stderr:
    halt: Finisher { value: 21845, status: Some(0) }
    steps: 617 executed instructions
    socket: 6 octets received in 2 records, 6 sent, sent sha256 5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03
    replay stdout identical to live

The replay retires the same 617 instructions and reports the same hash, with no
network. (The port number in "accepted" varies per run.) The release is checked
against `SHA256SUMS-binaries` and `SHA256SUMS` before use.

To build and run from source instead, use `cargo build --release -p sadhana -p yantra`
and the same `yantra-run` flags; the compile step still needs `stage1.elf`.
